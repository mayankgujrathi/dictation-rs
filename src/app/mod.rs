use std::collections::VecDeque;
use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, AtomicU32},
};
use std::time::Instant;

mod constants;
mod positioning;
mod render;
mod state;
mod workers;

pub use constants::{HISTORY_LEN, WINDOW_INNER_SIZE};
pub use state::UIState;

#[cfg(test)]
pub(crate) static TEST_CWD_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

pub struct VoiceApp {
  pub(crate) volume_atomic: Arc<AtomicU32>,
  pub(crate) is_recording: Arc<AtomicBool>,
  pub(crate) mic_ready: Arc<AtomicBool>,
  pub(crate) should_exit: Arc<AtomicBool>,
  pub(crate) ui_state: UIState,
  pub(crate) history: VecDeque<f32>,
  pub(crate) positioned: bool,
  pub(crate) saw_recording_active: bool,
  pub(crate) download_progress_atomic: Arc<AtomicU32>,
  pub(crate) download_spawned: bool,
  pub(crate) transcription_status: Arc<Mutex<Option<bool>>>,
  pub(crate) transcription_spawned: bool,
  pub(crate) transcription_rendered_at: Option<Instant>,
}

impl VoiceApp {
  pub fn new(
    volume_atomic: Arc<AtomicU32>,
    is_recording: Arc<AtomicBool>,
    mic_ready: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
  ) -> Self {
    let initial_state = if workers::is_model_downloaded_placeholder() {
      UIState::VisualizerRecording
    } else {
      UIState::ModelDownloading
    };

    Self {
      volume_atomic,
      is_recording,
      mic_ready,
      should_exit,
      ui_state: initial_state,
      history: VecDeque::from(vec![0.0; HISTORY_LEN]),
      positioned: false,
      saw_recording_active: false,
      download_progress_atomic: Arc::new(AtomicU32::new(0)),
      download_spawned: false,
      transcription_status: Arc::new(Mutex::new(None)),
      transcription_spawned: false,
      transcription_rendered_at: None,
    }
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32},
  };
  use tempfile::tempdir;

  fn model_flag_path() -> std::path::PathBuf {
    std::env::current_dir()
      .unwrap_or_else(|_| std::env::temp_dir())
      .join("hf_model_downloaded.flag")
  }

  #[test]
  fn test_voice_app_new_initializes_default_fields() {
    let _guard = TEST_CWD_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let old_cwd = std::env::current_dir().expect("current dir should be available");
    let temp = tempdir().expect("temp dir should be created");
    std::env::set_current_dir(temp.path()).expect("should switch current dir");

    let flag = model_flag_path();
    let _ = std::fs::remove_file(&flag);

    let app = VoiceApp::new(
      Arc::new(AtomicU32::new(0)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(app.history.len(), HISTORY_LEN);
    assert!(app.history.iter().all(|v| (*v - 0.0).abs() < f32::EPSILON));
    assert!(!app.positioned);
    assert!(!app.saw_recording_active);
    assert!(!app.download_spawned);
    assert!(!app.transcription_spawned);
    assert_eq!(
      app
        .download_progress_atomic
        .load(std::sync::atomic::Ordering::Relaxed),
      0
    );
    assert_eq!(
      *app
        .transcription_status
        .lock()
        .expect("status lock poisoned"),
      None
    );
    assert_eq!(app.transcription_rendered_at, None);
    assert_eq!(app.ui_state, UIState::ModelDownloading);

    std::env::set_current_dir(old_cwd).expect("should restore current dir");
  }

  #[test]
  fn test_voice_app_new_uses_visualizer_when_model_flag_exists() {
    let _guard = TEST_CWD_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let old_cwd = std::env::current_dir().expect("current dir should be available");
    let temp = tempdir().expect("temp dir should be created");
    std::env::set_current_dir(temp.path()).expect("should switch current dir");

    let flag = model_flag_path();
    std::fs::write(&flag, b"downloaded").expect("should create model flag");

    let app = VoiceApp::new(
      Arc::new(AtomicU32::new(0)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(app.ui_state, UIState::VisualizerRecording);

    let _ = std::fs::remove_file(&flag);
    std::env::set_current_dir(old_cwd).expect("should restore current dir");
  }
}
