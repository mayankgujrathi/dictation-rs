use std::sync::{
  Arc, Mutex, OnceLock,
  atomic::{AtomicU32, Ordering},
};
use std::time::{Duration, Instant};
use std::{io::Read, io::Write};

use arboard::Clipboard;
use directories::ProjectDirs;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;

use crate::audio::recording_output_path;
use std::path::PathBuf;
use transcribe_rs::onnx::Quantization;
use transcribe_rs::onnx::parakeet::{
  ParakeetModel, ParakeetParams, TimestampGranularity,
};
use transcribe_rs::{OrtAccelerator, set_ort_accelerator};

use super::VoiceApp;

const MODEL_DIR_NAME: &str = "parakeet-tdt-0.6b-v3-int8";
const MODEL_FILES: [&str; 4] = [
  "encoder-model.int8.onnx",
  "decoder_joint-model.int8.onnx",
  "nemo128.onnx",
  "vocab.txt",
];
const MODEL_SUCCESS_FLAG: &str = "download.success.flag";
const MODEL_IDLE_TTL_SECS: u64 = 10 * 60;

struct CachedParakeetModel {
  model: ParakeetModel,
  last_used_at: Instant,
}

static MODEL_CACHE: OnceLock<Mutex<Option<CachedParakeetModel>>> =
  OnceLock::new();

fn model_cache() -> &'static Mutex<Option<CachedParakeetModel>> {
  MODEL_CACHE.get_or_init(|| Mutex::new(None))
}

fn model_cache_ttl() -> Duration {
  let ttl_secs = std::env::var("DICTATION_MODEL_CACHE_TTL_SECS")
    .ok()
    .and_then(|v| v.parse::<u64>().ok())
    .unwrap_or(MODEL_IDLE_TTL_SECS);
  Duration::from_secs(ttl_secs)
}

fn is_cache_entry_expired(
  last_used_at: Instant,
  now: Instant,
  ttl: Duration,
) -> bool {
  now.saturating_duration_since(last_used_at) >= ttl
}

fn model_base_dir() -> std::path::PathBuf {
  if let Ok(override_path) = std::env::var("DICTATION_MODEL_BASE_DIR") {
    return std::path::PathBuf::from(override_path);
  }

  ProjectDirs::from("com", "dictation", "dictation")
    .map(|dirs| dirs.data_dir().to_path_buf())
    .unwrap_or_else(|| {
      std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
    })
}

fn model_dir_path() -> std::path::PathBuf {
  model_base_dir().join("models").join(MODEL_DIR_NAME)
}

fn model_success_flag_path() -> std::path::PathBuf {
  model_dir_path().join(MODEL_SUCCESS_FLAG)
}

fn has_invalid_text_signature(path: &std::path::Path) -> bool {
  let mut buf = [0_u8; 64];
  let Ok(mut f) = std::fs::File::open(path) else {
    return true;
  };
  let Ok(n) = f.read(&mut buf) else {
    return true;
  };
  let head = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
  head.contains("<html")
    || head.contains("<!doctype")
    || head.contains("git-lfs.github.com/spec/v1")
}

fn is_model_file_sane(path: &std::path::Path) -> bool {
  let Ok(meta) = std::fs::metadata(path) else {
    return false;
  };
  if !meta.is_file() {
    return false;
  }

  let size = meta.len();
  if size == 0 {
    return false;
  }

  let ext = path
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or_default();
  if ext.eq_ignore_ascii_case("onnx") {
    // Reject tiny files and obvious text/HTML/LFS pointer responses.
    if size < 1024 {
      return false;
    }
    if has_invalid_text_signature(path) {
      return false;
    }
  }

  true
}

pub(crate) fn is_model_downloaded() -> bool {
  let model_dir = model_dir_path();

  if !model_success_flag_path().exists() {
    return false;
  }

  MODEL_FILES
    .iter()
    .all(|name| is_model_file_sane(&model_dir.join(name)))
}

fn transcribe_call() -> Result<(), ()> {
  set_ort_accelerator(OrtAccelerator::Auto);
  let cache = model_cache();
  let mut cache_guard = match cache.lock() {
    Ok(g) => g,
    Err(e) => e.into_inner(),
  };

  let now = Instant::now();
  let ttl = model_cache_ttl();

  let should_reload = match cache_guard.as_ref() {
    Some(entry) => is_cache_entry_expired(entry.last_used_at, now, ttl),
    None => true,
  };

  if should_reload {
    // Drop expired/missing model and load a fresh one.
    *cache_guard = None;

    let load_result = ParakeetModel::load(
      &PathBuf::from(model_dir_path()),
      &Quantization::Int8,
    );
    let Ok(model) = load_result else {
      eprintln!("Unable to load model: {:?}", load_result.err());
      return Err(());
    };

    *cache_guard = Some(CachedParakeetModel {
      model,
      last_used_at: now,
    });
  }

  let Some(entry) = cache_guard.as_mut() else {
    eprintln!("Model cache unexpectedly empty after load/check");
    return Err(());
  };
  let samples =
    transcribe_rs::audio::read_wav_samples(&recording_output_path());
  let Ok(samples) = samples else {
    eprintln!("Unable to load recording file: {:?}", samples.err());
    return Err(());
  };
  let result = entry.model.transcribe_with(
    &samples,
    &ParakeetParams {
      timestamp_granularity: Some(TimestampGranularity::Segment),
      ..Default::default()
    },
  );
  let Ok(result) = result else {
    eprintln!("Unable to transcribe: {:?}", result.err());
    return Err(());
  };

  let transcript_text = result.text.clone();
  entry.last_used_at = Instant::now();
  drop(cache_guard);

  let active_window_title = get_active_window_title();
  let final_transcript = post_process_transcript(
    transcript_text.as_str(),
    active_window_title.as_str(),
  );

  if let Err(e) = update_clipboard_if_changed(final_transcript.as_str()) {
    eprintln!("Failed updating clipboard: {e}");
  }
  if let Err(e) = paste_from_clipboard_into_active_input_field() {
    eprintln!("Failed pasting transcript into active input field: {e}");
  } else {
    println!("Paste shortcut dispatched to active window");
  }

  println!("Transcription: {:?}", result);
  Ok(())
}

fn get_active_window_title() -> String {
  active_win_pos_rs::get_active_window()
    .ok()
    .map(|w| w.title)
    .unwrap_or_default()
}

fn post_process_transcript(
  transcript_text: &str,
  active_window_title: &str,
) -> String {
  println!(
    "post_process_transcript::Active window: {:?}",
    active_window_title
  );
  transcript_text.to_owned()
}

fn paste_from_clipboard_into_active_input_field() -> Result<(), String> {
  let mut enigo = Enigo::new(&Settings::default())
    .map_err(|e| format!("enigo init failed: {e}"))?;

  #[cfg(target_os = "macos")]
  {
    enigo
      .key(Key::Meta, Direction::Press)
      .map_err(|e| format!("meta press failed: {e}"))?;
    enigo
      .key(Key::Unicode('v'), Direction::Click)
      .map_err(|e| format!("v click failed: {e}"))?;
    enigo
      .key(Key::Meta, Direction::Release)
      .map_err(|e| format!("meta release failed: {e}"))?;
  }

  #[cfg(not(target_os = "macos"))]
  {
    enigo
      .key(Key::Control, Direction::Press)
      .map_err(|e| format!("control press failed: {e}"))?;
    enigo
      .key(Key::Unicode('v'), Direction::Click)
      .map_err(|e| format!("v click failed: {e}"))?;
    enigo
      .key(Key::Control, Direction::Release)
      .map_err(|e| format!("control release failed: {e}"))?;
  }

  Ok(())
}

fn should_update_clipboard(current: Option<&str>, next: &str) -> bool {
  current != Some(next)
}

fn update_clipboard_if_changed(text: &str) -> Result<(), String> {
  let mut clipboard =
    Clipboard::new().map_err(|e| format!("clipboard init failed: {e}"))?;
  let current_text = clipboard.get_text().ok();
  if should_update_clipboard(current_text.as_deref(), text) {
    clipboard
      .set_text(text.to_owned())
      .map_err(|e| format!("set clipboard failed: {e}"))?;
  }
  Ok(())
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
      total_bytes =
        total_bytes.saturating_add(response.content_length().unwrap_or(0));
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
        let pct = ((downloaded_bytes.saturating_mul(100)) / total_bytes)
          .min(100) as u32;
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
  fn test_cache_entry_expiry_logic() {
    let now = Instant::now();
    let ttl = Duration::from_secs(600);

    let fresh = now - Duration::from_secs(100);
    let expired = now - Duration::from_secs(700);

    assert!(!is_cache_entry_expired(fresh, now, ttl));
    assert!(is_cache_entry_expired(expired, now, ttl));
  }

  #[test]
  fn test_model_cache_ttl_reads_env_override() {
    unsafe { std::env::set_var("DICTATION_MODEL_CACHE_TTL_SECS", "5") };
    assert_eq!(model_cache_ttl(), Duration::from_secs(5));
    unsafe { std::env::remove_var("DICTATION_MODEL_CACHE_TTL_SECS") };
  }

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
      std::fs::write(model_dir.join(file), b"x")
        .expect("should create model file");
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
      if file.ends_with(".onnx") {
        std::fs::write(model_dir.join(file), vec![7_u8; 2048])
          .expect("should create sane onnx model file");
      } else {
        std::fs::write(model_dir.join(file), b"x")
          .expect("should create model file");
      }
    }
    std::fs::write(model_success_flag_path(), b"downloaded")
      .expect("should create model flag");

    assert!(is_model_downloaded());

    unsafe { std::env::remove_var("DICTATION_MODEL_BASE_DIR") };
  }

  #[test]
  fn test_model_downloaded_false_when_onnx_is_html() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let temp = tempdir().expect("temp dir should be created");
    unsafe { std::env::set_var("DICTATION_MODEL_BASE_DIR", temp.path()) };

    let model_dir = model_dir_path();
    std::fs::create_dir_all(&model_dir).expect("should create model dir");
    for file in MODEL_FILES {
      if file == "encoder-model.int8.onnx" {
        std::fs::write(model_dir.join(file), b"<html>bad response</html>")
          .expect("should create bad onnx file");
      } else if file.ends_with(".onnx") {
        std::fs::write(model_dir.join(file), vec![7_u8; 2048])
          .expect("should create sane onnx model file");
      } else {
        std::fs::write(model_dir.join(file), b"x")
          .expect("should create model file");
      }
    }
    std::fs::write(model_success_flag_path(), b"downloaded")
      .expect("should create model flag");

    assert!(!is_model_downloaded());

    unsafe { std::env::remove_var("DICTATION_MODEL_BASE_DIR") };
  }

  #[test]
  fn test_post_process_transcript_returns_input_text() {
    let out = post_process_transcript("hello world", "Some Window");
    assert_eq!(out, "hello world");
  }

  #[test]
  fn test_should_update_clipboard_when_different() {
    assert!(should_update_clipboard(Some("a"), "b"));
    assert!(should_update_clipboard(None, "b"));
    assert!(!should_update_clipboard(Some("b"), "b"));
  }
}
