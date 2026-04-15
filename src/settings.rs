use std::fs;
use std::path::PathBuf;
use std::sync::{OnceLock, RwLock};

use directories::ProjectDirs;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct AppSettings {
  pub logging: LoggingSettings,
  pub transcription: TranscriptionSettings,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct LoggingSettings {
  pub app_log_max_lines: usize,
  pub trace_file_limit: usize,
  pub enable_debug_logs: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TranscriptionSettings {
  pub built_in_dictionary: Vec<String>,
  pub user_dictionary: Vec<String>,
  pub model_cache_ttl_secs: u64,
}

impl Default for TranscriptionSettings {
  fn default() -> Self {
    Self {
      built_in_dictionary: Vec::new(),
      user_dictionary: Vec::new(),
      model_cache_ttl_secs: 10 * 60,
    }
  }
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
      transcription: TranscriptionSettings::default(),
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
  write_settings(path, &AppSettings::default())
}

fn write_settings(
  path: &std::path::Path,
  settings: &AppSettings,
) -> Result<(), String> {
  if let Some(parent) = path.parent() {
    fs::create_dir_all(parent)
      .map_err(|e| format!("create settings dir failed: {e}"))?;
  }
  let json = serde_json::to_string_pretty(settings)
    .map_err(|e| format!("serialize settings failed: {e}"))?;
  fs::write(path, json).map_err(|e| format!("write settings failed: {e}"))
}

fn load_from_disk() -> Result<AppSettings, String> {
  let path = settings_path();
  if !path.exists() {
    write_default_settings(&path)?;
    return Ok(AppSettings::default());
  }

  let raw = fs::read_to_string(&path)
    .map_err(|e| format!("read settings failed: {e}"))?;
  parse_and_backfill_settings(raw.as_str(), &path)
}

fn parse_and_backfill_settings(
  raw: &str,
  path: &std::path::Path,
) -> Result<AppSettings, String> {
  let parsed = serde_json::from_str::<AppSettings>(raw)
    .map_err(|e| format!("parse settings failed: {e}"))?;

  // Backfill newly added/defaulted keys into the on-disk settings file.
  if let Ok(raw_value) = serde_json::from_str::<serde_json::Value>(raw) {
    let normalized_value = serde_json::to_value(&parsed)
      .map_err(|e| format!("serialize settings for migration failed: {e}"))?;

    if raw_value != normalized_value {
      write_settings(path, &parsed)?;
    }
  }

  Ok(parsed)
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

#[cfg(test)]
mod tests {
  use super::*;

  #[test]
  fn test_parse_and_backfill_settings_writes_missing_transcription() {
    let unique = format!(
      "dictation_settings_test_{}",
      std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0)
    );

    let dir = std::env::temp_dir().join(unique);
    let file = dir.join("settings.json");
    fs::create_dir_all(&dir).expect("should create temp test dir");

    let old = r#"{
  "logging": {
    "app_log_max_lines": 1000,
    "trace_file_limit": 100,
    "enable_debug_logs": false
  }
}"#;
    fs::write(&file, old).expect("should write old settings json");

    let parsed = parse_and_backfill_settings(old, &file)
      .expect("should parse and backfill settings");
    assert_eq!(parsed.transcription, TranscriptionSettings::default());

    let updated =
      fs::read_to_string(&file).expect("should read backfilled settings");
    assert!(updated.contains("\"transcription\""));
    assert!(updated.contains("\"built_in_dictionary\""));
    assert!(updated.contains("\"user_dictionary\""));
    assert!(updated.contains("\"model_cache_ttl_secs\""));

    let _ = fs::remove_file(&file);
    let _ = fs::remove_dir_all(&dir);
  }
}
