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
