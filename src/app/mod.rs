use std::collections::VecDeque;
use std::sync::{
  Arc, Mutex,
  atomic::{AtomicBool, AtomicU32},
};
use std::time::Instant;

use eframe::egui;
use tracing::info;

mod constants;
mod positioning;
mod render;
mod state;
mod workers;

pub use constants::{
  DEFAULT_LLM_BASE_URL, DEFAULT_LLM_CUSTOM_PROMPT, DEFAULT_LLM_MODEL_NAME,
  DEFAULT_LLM_SYSTEM_PROMPT, HISTORY_LEN, WINDOW_INNER_SIZE,
};
pub use state::UIState;

static UI_CONTEXT: std::sync::OnceLock<egui::Context> =
  std::sync::OnceLock::new();

#[cfg(test)]
pub(crate) static TEST_CWD_LOCK: std::sync::Mutex<()> =
  std::sync::Mutex::new(());

pub struct VoiceApp {
  pub(crate) volume_atomic: Arc<AtomicU32>,
  pub(crate) is_recording: Arc<AtomicBool>,
  pub(crate) mic_ready: Arc<AtomicBool>,
  pub(crate) recording_ready: Arc<AtomicBool>,
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
  pub(crate) viewport_visible: bool,
  pub(crate) viewport_size: [f32; 2],
}

impl VoiceApp {
  pub fn new(
    volume_atomic: Arc<AtomicU32>,
    is_recording: Arc<AtomicBool>,
    mic_ready: Arc<AtomicBool>,
    recording_ready: Arc<AtomicBool>,
    should_exit: Arc<AtomicBool>,
  ) -> Self {
    let initial_state = if workers::is_model_downloaded() {
      UIState::VisualizerRecording
    } else {
      UIState::ModelDownloading
    };
    info!(state = ?initial_state, "voice app initialized");

    Self {
      volume_atomic,
      is_recording,
      mic_ready,
      recording_ready,
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
      viewport_visible: false,
      viewport_size: [0.0, 0.0],
    }
  }
}

pub fn register_ui_context(ctx: &egui::Context) {
  let _ = UI_CONTEXT.set(ctx.clone());
}

pub fn wake_ui() {
  if let Some(ctx) = UI_CONTEXT.get() {
    ctx.request_repaint();
  }
}

pub fn is_model_ready() -> bool {
  let ready = workers::is_model_downloaded();
  if !ready {
    tracing::debug!("model is not ready yet");
  }
  ready
}

#[cfg(test)]
mod tests {
  use super::*;
  use directories::ProjectDirs;
  use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU32},
  };

  fn model_dir_path() -> std::path::PathBuf {
    if let Ok(override_path) = std::env::var("DICTATION_MODEL_BASE_DIR") {
      return std::path::PathBuf::from(override_path)
        .join("models")
        .join("parakeet-tdt-0.6b-v3-int8");
    }

    ProjectDirs::from("com", "dictation", "dictation")
      .map(|dirs| {
        dirs
          .data_dir()
          .join("models")
          .join("parakeet-tdt-0.6b-v3-int8")
      })
      .unwrap_or_else(|| {
        std::env::current_dir()
          .unwrap_or_else(|_| std::env::temp_dir())
          .join("models")
          .join("parakeet-tdt-0.6b-v3-int8")
      })
  }

  #[test]
  fn test_voice_app_new_initializes_default_fields() {
    let _guard = TEST_CWD_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let model_dir = model_dir_path();
    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );

    let app = VoiceApp::new(
      Arc::new(AtomicU32::new(0)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(true)),
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
  }

  #[test]
  fn test_voice_app_new_uses_visualizer_when_model_flag_exists() {
    let _guard = TEST_CWD_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());
    let model_dir = model_dir_path();
    std::fs::create_dir_all(&model_dir).expect("should create model dir");
    for file in [
      "encoder-model.int8.onnx",
      "decoder_joint-model.int8.onnx",
      "nemo128.onnx",
      "vocab.txt",
    ] {
      if file.ends_with(".onnx") {
        std::fs::write(model_dir.join(file), vec![7_u8; 2048])
          .expect("should create sane onnx model file");
      } else {
        std::fs::write(model_dir.join(file), b"x")
          .expect("should create model file");
      }
    }
    std::fs::write(model_dir.join("download.success.flag"), b"downloaded")
      .expect("should create model success flag");

    let app = VoiceApp::new(
      Arc::new(AtomicU32::new(0)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(false)),
      Arc::new(AtomicBool::new(true)),
      Arc::new(AtomicBool::new(false)),
    );

    assert_eq!(app.ui_state, UIState::VisualizerRecording);

    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );
  }
}
