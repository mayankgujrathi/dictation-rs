use std::sync::{
  Arc, Mutex,
  atomic::{AtomicU32, Ordering},
};
use std::time::Duration;
use std::{io::Read, io::Write};

use directories::ProjectDirs;
use reqwest::blocking::Client;
use reqwest::redirect::Policy;

use super::VoiceApp;

const MODEL_DIR_NAME: &str = "parakeet-tdt-0.6b-v3-int8";
const MODEL_FILES: [&str; 4] = [
  "encoder-model.int8.onnx",
  "decoder_joint-model.int8.onnx",
  "nemo128.onnx",
  "vocab.txt",
];
const MODEL_SUCCESS_FLAG: &str = "download.success.flag";

fn model_base_dir() -> std::path::PathBuf {
  if let Ok(override_path) = std::env::var("DICTATION_MODEL_BASE_DIR") {
    return std::path::PathBuf::from(override_path);
  }

  ProjectDirs::from("com", "dictation", "dictation")
    .map(|dirs| dirs.data_dir().to_path_buf())
    .unwrap_or_else(|| std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir()))
}

fn model_dir_path() -> std::path::PathBuf {
  model_base_dir().join("models").join(MODEL_DIR_NAME)
}

fn model_success_flag_path() -> std::path::PathBuf {
  model_dir_path().join(MODEL_SUCCESS_FLAG)
}

pub(crate) fn is_model_downloaded() -> bool {
  let model_dir = model_dir_path();

  if !model_success_flag_path().exists() {
    return false;
  }

  MODEL_FILES.iter().all(|name| model_dir.join(name).exists())
}

fn transcribe_call() -> Result<(), ()> {
  std::thread::sleep(Duration::from_millis(2500));
  Err(())
}

fn run_model_download(progress: Arc<AtomicU32>) {
  progress.store(0, Ordering::Relaxed);

  let model_dir = model_dir_path();
  if std::fs::create_dir_all(&model_dir).is_err() {
    return;
  }

  let _ = std::fs::remove_file(model_success_flag_path());

  let endpoints = MODEL_FILES.iter().map(|filename| {
    (
      *filename,
      format!(
        "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/{}?download=true",
        filename
      ),
    )
  });

  let endpoint_list: Vec<(&str, String)> = endpoints.collect();
  let client = match Client::builder()
    .connect_timeout(Duration::from_secs(15))
    .timeout(Duration::from_secs(600))
    .redirect(Policy::limited(10))
    .user_agent("dictation-rs/0.1")
    .build()
  {
    Ok(client) => client,
    Err(e) => {
      eprintln!("Failed to create HTTP client: {e}");
      return;
    }
  };

  let mut total_bytes: u64 = 0;
  for (_, url) in &endpoint_list {
    let size = client
      .head(url)
      .send()
      .ok()
      .and_then(|resp| {
        resp
          .headers()
          .get(reqwest::header::CONTENT_LENGTH)
          .and_then(|v| v.to_str().ok())
          .and_then(|s| s.parse::<u64>().ok())
      })
      .unwrap_or(0);
    total_bytes = total_bytes.saturating_add(size);
  }

  let mut downloaded_bytes: u64 = 0;

  for (filename, url) in endpoint_list {
    let mut response = match client.get(url).send() {
      Ok(resp) if resp.status().is_success() => resp,
      Ok(resp) => {
        eprintln!("Download failed for {filename}: HTTP {}", resp.status());
        return;
      }
      Err(e) => {
        eprintln!("Download request failed for {filename}: {e}");
        return;
      }
    };

    if total_bytes == 0 {
      total_bytes = total_bytes.saturating_add(response.content_length().unwrap_or(0));
    }

    let mut file = match std::fs::File::create(model_dir.join(filename)) {
      Ok(f) => f,
      Err(_) => return,
    };

    let mut buffer = [0_u8; 64 * 1024];
    loop {
      let read = match response.read(&mut buffer) {
        Ok(0) => break,
        Ok(n) => n,
        Err(e) => {
          eprintln!("Download stream read failed for {filename}: {e}");
          return;
        }
      };

      if file.write_all(&buffer[..read]).is_err() {
        eprintln!("Writing file failed for {filename}");
        return;
      }

      downloaded_bytes = downloaded_bytes.saturating_add(read as u64);
      if total_bytes > 0 {
        let pct = ((downloaded_bytes.saturating_mul(100)) / total_bytes).min(100) as u32;
        progress.store(pct, Ordering::Relaxed);
      }
    }
  }

  progress.store(100, Ordering::Relaxed);
  let _ = std::fs::write(model_success_flag_path(), b"downloaded");
}

fn run_transcription(status_slot: Arc<Mutex<Option<bool>>>) {
  let is_error = transcribe_call().is_err();
  if let Ok(mut slot) = status_slot.lock() {
    *slot = Some(is_error);
  }
}

impl VoiceApp {
  pub(crate) fn spawn_model_download_worker_if_needed(&mut self) {
    if self.download_spawned {
      return;
    }

    self.download_spawned = true;
    let progress = self.download_progress_atomic.clone();
    std::thread::spawn(move || run_model_download(progress));
  }

  pub(crate) fn spawn_transcription_worker_if_needed(&mut self) {
    if self.transcription_spawned {
      return;
    }

    self.transcription_spawned = true;
    let status_slot = self.transcription_status.clone();
    std::thread::spawn(move || run_transcription(status_slot));
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use crate::app::TEST_CWD_LOCK;
  use tempfile::tempdir;

  #[test]
  fn test_model_downloaded_false_when_success_flag_absent() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let temp = tempdir().expect("temp dir should be created");
    unsafe { std::env::set_var("DICTATION_MODEL_BASE_DIR", temp.path()) };

    let model_dir = model_dir_path();
    std::fs::create_dir_all(&model_dir).expect("should create model dir");
    for file in MODEL_FILES {
      std::fs::write(model_dir.join(file), b"x").expect("should create model file");
    }

    assert!(!is_model_downloaded());

    unsafe { std::env::remove_var("DICTATION_MODEL_BASE_DIR") };
  }

  #[test]
  fn test_model_downloaded_true_when_files_and_success_flag_present() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let temp = tempdir().expect("temp dir should be created");
    unsafe { std::env::set_var("DICTATION_MODEL_BASE_DIR", temp.path()) };

    let model_dir = model_dir_path();
    std::fs::create_dir_all(&model_dir).expect("should create model dir");
    for file in MODEL_FILES {
      std::fs::write(model_dir.join(file), b"x").expect("should create model file");
    }
    std::fs::write(model_success_flag_path(), b"downloaded").expect("should create model flag");

    assert!(is_model_downloaded());

    unsafe { std::env::remove_var("DICTATION_MODEL_BASE_DIR") };
  }
}
