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

pub struct VoiceApp {
  pub(crate) volume_atomic: Arc<AtomicU32>,
  pub(crate) is_recording: Arc<AtomicBool>,
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
