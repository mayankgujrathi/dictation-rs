use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppSettings {
  pub logging: LoggingSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct LoggingSettings {
  pub app_log_max_lines: usize,
  pub trace_file_limit: usize,
  pub enable_debug_logs: bool,
}

impl Default for LoggingSettings {
  fn default() -> Self {
    Self {
      app_log_max_lines: 1000,
      trace_file_limit: 100,
      enable_debug_logs: false,
    }
  }
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      logging: LoggingSettings::default(),
    }
  }
}

static SETTINGS: OnceLock<RwLock<AppSettings>> = OnceLock::new();

pub fn data_dir() -> PathBuf {
  ProjectDirs::from("com", "dictation", "dictation")
    .map(|dirs| dirs.data_dir().to_path_buf())
    .unwrap_or_else(|| {
      std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
    })
}

pub fn settings_path() -> PathBuf {
  data_dir().join("settings.json")
}

fn write_default_settings(path: &std::path::Path) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create settings dir failed: {e}"))?;
  }
  let json = serde_json::to_string_pretty(&AppSettings::default())
    .map_err(|e| format!("serialize default settings failed: {e}"))?;
  fs::write(path, json).map_err(|e| format!("write default settings failed: {e}"))
}

fn load_from_disk() -> Result<AppSettings, String> {
  let path = settings_path();
  if !path.exists() {
    write_default_settings(&path)?;
    return Ok(AppSettings::default());
  }

  let raw = fs::read_to_string(&path)
    .map_err(|e| format!("read settings failed: {e}"))?;
  serde_json::from_str::<AppSettings>(&raw)
    .map_err(|e| format!("parse settings failed: {e}"))
}

pub fn initialize() {
  let initial = load_from_disk().unwrap_or_default();
  let _ = SETTINGS.get_or_init(|| RwLock::new(initial));
}

pub fn current() -> AppSettings {
  initialize();
  match SETTINGS.get() {
    Some(lock) => lock.read().map(|g| g.clone()).unwrap_or_default(),
    None => AppSettings::default(),
  }
}

pub fn refresh_from_disk() -> Result<bool, String> {
  initialize();
  let next = load_from_disk()?;

  let Some(lock) = SETTINGS.get() else {
    return Ok(false);
  };

  let mut guard = lock
    .write()
    .map_err(|_| "settings lock poisoned".to_string())?;
  if *guard == next {
    return Ok(false);
  }

  *guard = next;
  Ok(true)
}
