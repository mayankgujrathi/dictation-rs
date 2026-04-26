pub mod bridge;
mod window;

pub use window::{
  open_settings_window, run_settings_process, should_run_as_settings_process,
};

pub const SETTINGS_WINDOW_TITLE: &str = "Vocoflow Settings";
pub const SETTINGS_WINDOW_WIDTH: f64 = 960.0;
pub const SETTINGS_WINDOW_HEIGHT: f64 = 540.0;
pub const SETTINGS_WINDOW_ARG: &str = "--settings-window";
