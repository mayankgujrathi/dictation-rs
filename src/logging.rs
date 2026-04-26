use std::fs::{self, OpenOptions};
use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
#[cfg(not(target_os = "windows"))]
use std::process::Command;
use std::sync::Mutex;
use std::sync::OnceLock;
use std::time::{SystemTime, UNIX_EPOCH};

use tracing::level_filters::LevelFilter;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_chrome::FlushGuard;
use tracing_subscriber::Layer;
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::util::SubscriberInitExt;

#[cfg(target_os = "windows")]
use windows_sys::Win32::UI::Shell::ShellExecuteW;

use crate::settings;

static APP_LOG_GUARD: OnceLock<WorkerGuard> = OnceLock::new();
static TRACE_FLUSH_GUARD: OnceLock<Mutex<FlushGuard>> = OnceLock::new();

fn data_dir() -> PathBuf {
  settings::data_dir()
}

fn logs_dir() -> PathBuf {
  data_dir().join("logs")
}

pub fn logs_dir_path() -> PathBuf {
  logs_dir()
}

fn traces_dir() -> PathBuf {
  logs_dir().join("traces")
}

fn debug_enabled() -> bool {
  settings::current().logging.enable_debug_logs
}

#[cfg(target_os = "windows")]
fn to_wide_null(s: &str) -> Vec<u16> {
  s.encode_utf16().chain(std::iter::once(0)).collect()
}

#[cfg(target_os = "windows")]
fn shell_execute_open(target: &str) -> Result<(), String> {
  let operation = to_wide_null("open");
  let file = to_wide_null(target);

  let result = unsafe {
    ShellExecuteW(
      std::ptr::null_mut(),
      operation.as_ptr(),
      file.as_ptr(),
      std::ptr::null(),
      std::ptr::null(),
      1, // SW_SHOWNORMAL
    )
  };

  let code = result as isize;
  if code <= 32 {
    return Err(format!("ShellExecuteW failed with code {code}"));
  }

  Ok(())
}

fn trim_to_last_n_lines(path: &Path, max_lines: usize) -> Result<(), String> {
  if !path.exists() {
    return Ok(());
  }

  let file = std::fs::File::open(path)
    .map_err(|e| format!("open app log failed: {e}"))?;
  let reader = BufReader::new(file);
  let lines: Vec<String> = reader.lines().map_while(Result::ok).collect();
  if lines.len() <= max_lines {
    return Ok(());
  }

  let start = lines.len() - max_lines;
  let mut file = std::fs::File::create(path)
    .map_err(|e| format!("rewrite app log failed: {e}"))?;
  for line in &lines[start..] {
    writeln!(file, "{line}")
      .map_err(|e| format!("write trimmed app log failed: {e}"))?;
  }
  Ok(())
}

fn prune_old_trace_files(dir: &Path, keep: usize) -> Result<(), String> {
  if !dir.exists() {
    return Ok(());
  }

  let mut files = fs::read_dir(dir)
    .map_err(|e| format!("read traces dir failed: {e}"))?
    .filter_map(Result::ok)
    .filter_map(|entry| {
      let path = entry.path();
      if !path.is_file() {
        return None;
      }
      let modified = entry
        .metadata()
        .and_then(|m| m.modified())
        .unwrap_or(SystemTime::UNIX_EPOCH);
      Some((path, modified))
    })
    .collect::<Vec<_>>();

  files.sort_by_key(|(_, modified)| *modified);
  if files.len() <= keep {
    return Ok(());
  }

  let to_remove = files.len().saturating_sub(keep);
  for (path, _) in files.into_iter().take(to_remove) {
    let _ = fs::remove_file(path);
  }

  Ok(())
}

pub fn init_logging() -> Result<(), String> {
  let settings = settings::current();
  let app_log_max_lines = settings.logging.app_log_max_lines;
  let trace_file_limit = settings.logging.trace_file_limit;

  let logs_dir = logs_dir();
  let traces_dir = traces_dir();
  fs::create_dir_all(&logs_dir)
    .map_err(|e| format!("create logs dir failed: {e}"))?;
  fs::create_dir_all(&traces_dir)
    .map_err(|e| format!("create traces dir failed: {e}"))?;

  let app_log_path = logs_dir.join("application.log");
  trim_to_last_n_lines(&app_log_path, app_log_max_lines)?;

  prune_old_trace_files(&traces_dir, trace_file_limit)?;
  let trace_file_name = format!(
    "trace-{}.json",
    SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .map(|d| d.as_millis())
      .unwrap_or(0)
  );
  let trace_file_path = traces_dir.join(trace_file_name);

  let app_file = OpenOptions::new()
    .create(true)
    .append(true)
    .open(&app_log_path)
    .map_err(|e| format!("open application log failed: {e}"))?;
  let (app_non_blocking, app_guard) = tracing_appender::non_blocking(app_file);
  let (trace_layer, trace_guard) = tracing_chrome::ChromeLayerBuilder::new()
    .file(&trace_file_path)
    .include_args(true)
    .build();

  let level = if debug_enabled() {
    LevelFilter::DEBUG
  } else {
    LevelFilter::INFO
  };

  let app_layer = tracing_subscriber::fmt::layer()
    .with_writer(app_non_blocking)
    .with_ansi(false)
    .with_target(true)
    .with_filter(level);

  let trace_layer = trace_layer.with_filter(level);

  tracing_subscriber::registry()
    .with(app_layer)
    .with(trace_layer)
    .try_init()
    .map_err(|e| format!("install tracing subscriber failed: {e}"))?;

  let _ = APP_LOG_GUARD.set(app_guard);
  let _ = TRACE_FLUSH_GUARD.set(Mutex::new(trace_guard));
  tracing::info!(
    app_log = %app_log_path.display(),
    trace_file = %trace_file_path.display(),
    debug_enabled = settings.logging.enable_debug_logs,
    app_log_max_lines,
    trace_file_limit,
    "logging initialized"
  );

  // Re-apply file-count retention after creating this run's trace.
  prune_old_trace_files(&traces_dir, trace_file_limit)?;
  Ok(())
}

pub fn enforce_app_log_retention() {
  let app_log_max_lines = settings::current().logging.app_log_max_lines;
  let path = logs_dir().join("application.log");
  let _ = trim_to_last_n_lines(&path, app_log_max_lines);
}

pub fn apply_runtime_logging_settings() {
  let cfg = settings::current().logging;
  let app_log_path = logs_dir().join("application.log");
  let _ = trim_to_last_n_lines(&app_log_path, cfg.app_log_max_lines);
  let _ = prune_old_trace_files(&traces_dir(), cfg.trace_file_limit);
}

pub fn open_logs_dir_in_file_manager() -> Result<PathBuf, String> {
  let dir = logs_dir();
  fs::create_dir_all(&dir)
    .map_err(|e| format!("create logs dir failed: {e}"))?;

  #[cfg(target_os = "windows")]
  {
    shell_execute_open(&dir.display().to_string())
      .map_err(|e| format!("open logs dir in Explorer failed: {e}"))?;
  }

  #[cfg(target_os = "macos")]
  {
    Command::new("open")
      .arg(&dir)
      .spawn()
      .map_err(|e| format!("open logs dir in Finder failed: {e}"))?;
  }

  #[cfg(all(unix, not(target_os = "macos")))]
  {
    Command::new("xdg-open")
      .arg(&dir)
      .spawn()
      .map_err(|e| format!("open logs dir with xdg-open failed: {e}"))?;
  }

  Ok(dir)
}

pub fn open_url_in_default_browser(url: &str) -> Result<(), String> {
  #[cfg(target_os = "windows")]
  {
    shell_execute_open(url)
      .map_err(|e| format!("open url in browser failed (windows): {e}"))?;
  }

  #[cfg(target_os = "macos")]
  {
    Command::new("open")
      .arg(url)
      .spawn()
      .map_err(|e| format!("open url in browser failed (macos): {e}"))?;
  }

  #[cfg(all(unix, not(target_os = "macos")))]
  {
    Command::new("xdg-open")
      .arg(url)
      .spawn()
      .map_err(|e| format!("open url in browser failed (linux): {e}"))?;
  }

  Ok(())
}
