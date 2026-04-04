use std::sync::{
  Arc, Mutex,
  atomic::{AtomicU32, Ordering},
};
use std::time::Duration;

use super::VoiceApp;

pub(crate) fn is_model_downloaded_placeholder() -> bool {
  std::env::current_dir()
    .unwrap_or_else(|_| std::env::temp_dir())
    .join("hf_model_downloaded.flag")
    .exists()
}

fn placeholder_transcribe_call() -> Result<(), ()> {
  std::thread::sleep(Duration::from_millis(2500));
  Err(())
}

fn run_model_download(progress: Arc<AtomicU32>) {
  for step in 0..=100 {
    progress.store(step, Ordering::Relaxed);
    std::thread::sleep(Duration::from_millis(30));
  }

  let _ = std::fs::write(
    std::env::current_dir()
      .unwrap_or_else(|_| std::env::temp_dir())
      .join("hf_model_downloaded.flag"),
    b"downloaded",
  );
}

fn run_transcription(status_slot: Arc<Mutex<Option<bool>>>) {
  let is_error = placeholder_transcribe_call().is_err();
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

  fn model_flag_path() -> std::path::PathBuf {
    std::env::current_dir()
      .unwrap_or_else(|_| std::env::temp_dir())
      .join("hf_model_downloaded.flag")
  }

  #[test]
  fn test_model_downloaded_placeholder_false_when_absent() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let old_cwd = std::env::current_dir().expect("current dir should be available");
    let temp = tempdir().expect("temp dir should be created");
    std::env::set_current_dir(temp.path()).expect("should switch current dir");

    let flag = model_flag_path();
    let _ = std::fs::remove_file(&flag);

    assert!(!is_model_downloaded_placeholder());

    std::env::set_current_dir(old_cwd).expect("should restore current dir");
  }

  #[test]
  fn test_model_downloaded_placeholder_true_when_present() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let old_cwd = std::env::current_dir().expect("current dir should be available");
    let temp = tempdir().expect("temp dir should be created");
    std::env::set_current_dir(temp.path()).expect("should switch current dir");

    let flag = model_flag_path();
    let _ = std::fs::remove_file(&flag);
    std::fs::write(&flag, b"downloaded").expect("should create model flag");

    assert!(is_model_downloaded_placeholder());

    let _ = std::fs::remove_file(&flag);
    std::env::set_current_dir(old_cwd).expect("should restore current dir");
  }

  #[test]
  fn test_run_model_download_reaches_100_and_creates_flag() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let old_cwd = std::env::current_dir().expect("current dir should be available");
    let temp = tempdir().expect("temp dir should be created");
    std::env::set_current_dir(temp.path()).expect("should switch current dir");

    let flag = model_flag_path();
    let _ = std::fs::remove_file(&flag);

    let progress = Arc::new(AtomicU32::new(0));
    run_model_download(progress.clone());

    assert_eq!(progress.load(Ordering::Relaxed), 100);
    assert!(flag.exists());

    let _ = std::fs::remove_file(&flag);
    std::env::set_current_dir(old_cwd).expect("should restore current dir");
  }

  #[test]
  fn test_run_transcription_sets_error_status() {
    let status_slot = Arc::new(Mutex::new(None));
    run_transcription(status_slot.clone());

    assert_eq!(
      *status_slot.lock().expect("status lock poisoned"),
      Some(true)
    );
  }
}
