#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum UIState {
  ModelDownloading,
  VisualizerRecording,
  Transcribing,
}
