#[cfg(target_os = "macos")]
use std::path::Path;
use std::process::Command;
use std::sync::{
  Arc, Mutex, OnceLock,
  atomic::{AtomicU32, Ordering},
};
use std::time::{Duration, Instant};
use std::{io::Read, io::Write};

use arboard::Clipboard;
use directories::ProjectDirs;
use enigo::{Direction, Enigo, Key, Keyboard, Settings};
use reqwest::blocking::Client;
use reqwest::redirect::Policy;
use tracing::{debug, error, info, warn};

use crate::audio::recording_output_path;
use crate::llm::{
  LlmAppContext, LlmPostProcessorConfig, process_transcript_with_llm,
};
use crate::settings::{self, TranscriptReformattingLevel};
use transcribe_rs::onnx::Quantization;
use transcribe_rs::onnx::parakeet::{
  ParakeetModel, ParakeetParams, TimestampGranularity,
};
use transcribe_rs::{OrtAccelerator, set_ort_accelerator};

use super::VoiceApp;

const MODEL_DIR_NAME: &str = "parakeet-tdt-0.6b-v3-int8";
const MODEL_FILES: [&str; 4] = [
  "encoder-model.int8.onnx",
  "decoder_joint-model.int8.onnx",
  "nemo128.onnx",
  "vocab.txt",
];
const MODEL_SUCCESS_FLAG: &str = "download.success.flag";

struct CachedParakeetModel {
  model: ParakeetModel,
  last_used_at: Instant,
}

#[derive(Debug, Clone, Default)]
struct ActiveApplicationInfo {
  window_title: String,
  application_name: Option<String>,
  application_description: Option<String>,
}

static MODEL_CACHE: OnceLock<Mutex<Option<CachedParakeetModel>>> =
  OnceLock::new();

fn model_cache() -> &'static Mutex<Option<CachedParakeetModel>> {
  MODEL_CACHE.get_or_init(|| Mutex::new(None))
}

fn model_cache_ttl() -> Duration {
  let settings_ttl_secs =
    settings::current().transcription.model_cache_ttl_secs;
  let ttl_secs = std::env::var("DICTATION_MODEL_CACHE_TTL_SECS")
    .ok()
    .and_then(|v| v.parse::<u64>().ok())
    .unwrap_or(settings_ttl_secs)
    .max(1);
  Duration::from_secs(ttl_secs)
}

fn is_cache_entry_expired(
  last_used_at: Instant,
  now: Instant,
  ttl: Duration,
) -> bool {
  now.saturating_duration_since(last_used_at) >= ttl
}

fn model_base_dir() -> std::path::PathBuf {
  ProjectDirs::from("com", "dictation", "dictation")
    .map(|dirs| dirs.data_dir().to_path_buf())
    .unwrap_or_else(|| {
      std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
    })
}

fn model_dir_path() -> std::path::PathBuf {
  model_base_dir().join("models").join(MODEL_DIR_NAME)
}

fn model_success_flag_path() -> std::path::PathBuf {
  model_dir_path().join(MODEL_SUCCESS_FLAG)
}

fn has_invalid_text_signature(path: &std::path::Path) -> bool {
  let mut buf = [0_u8; 64];
  let Ok(mut f) = std::fs::File::open(path) else {
    return true;
  };
  let Ok(n) = f.read(&mut buf) else {
    return true;
  };
  let head = String::from_utf8_lossy(&buf[..n]).to_ascii_lowercase();
  head.contains("<html")
    || head.contains("<!doctype")
    || head.contains("git-lfs.github.com/spec/v1")
}

fn is_model_file_sane(path: &std::path::Path) -> bool {
  let Ok(meta) = std::fs::metadata(path) else {
    return false;
  };
  if !meta.is_file() {
    return false;
  }

  let size = meta.len();
  if size == 0 {
    return false;
  }

  let ext = path
    .extension()
    .and_then(|e| e.to_str())
    .unwrap_or_default();
  if ext.eq_ignore_ascii_case("onnx") {
    // Reject tiny files and obvious text/HTML/LFS pointer responses.
    if size < 1024 {
      return false;
    }
    if has_invalid_text_signature(path) {
      return false;
    }
  }

  true
}

pub(crate) fn is_model_downloaded() -> bool {
  let model_dir = model_dir_path();

  if !model_success_flag_path().exists() {
    return false;
  }

  MODEL_FILES
    .iter()
    .all(|name| is_model_file_sane(&model_dir.join(name)))
}

fn transcribe_call() -> Result<(), ()> {
  info!("transcription worker started");
  set_ort_accelerator(OrtAccelerator::Auto);
  let cache = model_cache();
  let mut cache_guard = match cache.lock() {
    Ok(g) => g,
    Err(e) => e.into_inner(),
  };

  let now = Instant::now();
  let ttl = model_cache_ttl();

  let should_reload = match cache_guard.as_ref() {
    Some(entry) => is_cache_entry_expired(entry.last_used_at, now, ttl),
    None => true,
  };

  if should_reload {
    debug!("loading transcription model into cache");
    // Drop expired/missing model and load a fresh one.
    *cache_guard = None;

    let load_result =
      ParakeetModel::load(&model_dir_path(), &Quantization::Int8);
    let Ok(model) = load_result else {
      error!(error = ?load_result.err(), "unable to load model");
      return Err(());
    };

    *cache_guard = Some(CachedParakeetModel {
      model,
      last_used_at: now,
    });
  }

  let Some(entry) = cache_guard.as_mut() else {
    error!("model cache unexpectedly empty after load/check");
    return Err(());
  };
  let samples =
    transcribe_rs::audio::read_wav_samples(&recording_output_path());
  let Ok(samples) = samples else {
    error!(error = ?samples.err(), "unable to load recording file");
    return Err(());
  };
  let result = entry.model.transcribe_with(
    &samples,
    &ParakeetParams {
      timestamp_granularity: Some(TimestampGranularity::Segment),
      ..Default::default()
    },
  );
  let Ok(result) = result else {
    error!(error = ?result.err(), "unable to transcribe recording");
    return Err(());
  };

  let transcript_text = result.text.clone();
  entry.last_used_at = Instant::now();
  drop(cache_guard);

  let normalized_transcript =
    process_transcript_with_custom_dictionary(transcript_text.as_str());
  let active_app_info = get_active_application_info();
  let final_transcript = match post_process_transcript(
    normalized_transcript.as_str(),
    &active_app_info,
  ) {
    Ok(text) => text,
    Err(()) => {
      error!("post processing transcript failed");
      return Err(());
    }
  };

  info!(
    final_transcript = %final_transcript,
    "final transcript after post-processing"
  );

  if let Err(e) = update_clipboard_if_changed(final_transcript.as_str()) {
    warn!(error = %e, "failed updating clipboard");
  }
  if let Err(e) = paste_from_clipboard_into_active_input_field() {
    warn!(error = %e, "failed pasting transcript into active input field");
  } else {
    info!("paste shortcut dispatched to active window");
  }

  info!(text = %result.text, "transcription completed");
  Ok(())
}

fn process_transcript_with_custom_dictionary(transcript_text: &str) -> String {
  let cfg = settings::current();
  let mut rules = build_dictionary_rules(
    &cfg.transcription.built_in_dictionary,
    &cfg.transcription.user_dictionary,
  );

  if rules.is_empty() {
    return transcript_text.to_owned();
  }

  rules.sort_by(|a, b| b.0.len().cmp(&a.0.len()));

  let mut out = transcript_text.to_owned();
  for (from, to) in rules {
    out = replace_case_insensitive(&out, &from, &to);
  }
  out
}

fn build_dictionary_rules(
  built_in: &[String],
  user_defined: &[String],
) -> Vec<(String, String)> {
  use std::collections::HashMap;

  let mut by_key: HashMap<String, String> = HashMap::new();

  for (from, to) in built_in
    .iter()
    .filter_map(|entry| parse_dictionary_entry(entry.as_str()))
  {
    by_key.insert(from, to);
  }

  for (from, to) in user_defined
    .iter()
    .filter_map(|entry| parse_dictionary_entry(entry.as_str()))
  {
    // User-defined dictionary overrides built-ins.
    by_key.insert(from, to);
  }

  by_key.into_iter().collect()
}

fn parse_dictionary_entry(entry: &str) -> Option<(String, String)> {
  let separators = ["=>", "->", "="];

  for sep in separators {
    let mut parts = entry.splitn(2, sep);
    let left = parts.next().map(str::trim).unwrap_or_default();
    let right = parts.next().map(str::trim).unwrap_or_default();
    if !left.is_empty() && !right.is_empty() {
      return Some((left.to_ascii_lowercase(), right.to_owned()));
    }
  }

  None
}

fn replace_case_insensitive(input: &str, from: &str, to: &str) -> String {
  if from.is_empty() {
    return input.to_owned();
  }

  let input_lower = input.to_ascii_lowercase();
  let mut out = String::with_capacity(input.len());
  let mut cursor = 0usize;

  while let Some(found) = input_lower[cursor..].find(from) {
    let start = cursor + found;
    let end = start + from.len();
    out.push_str(&input[cursor..start]);
    out.push_str(to);
    cursor = end;
  }

  out.push_str(&input[cursor..]);
  out
}

fn get_active_window_title() -> String {
  active_win_pos_rs::get_active_window()
    .ok()
    .map(|w| w.title)
    .unwrap_or_default()
}

fn get_active_application_info() -> ActiveApplicationInfo {
  let window_title = get_active_window_title();
  let (application_name, application_description) =
    identify_active_application(window_title.as_str());

  ActiveApplicationInfo {
    window_title,
    application_name,
    application_description,
  }
}

fn identify_active_application(
  active_window_title: &str,
) -> (Option<String>, Option<String>) {
  #[cfg(target_os = "windows")]
  {
    identify_active_application_windows(active_window_title)
  }

  #[cfg(target_os = "linux")]
  {
    identify_active_application_linux(active_window_title)
  }

  #[cfg(target_os = "macos")]
  {
    identify_active_application_macos(active_window_title)
  }

  #[cfg(not(any(
    target_os = "windows",
    target_os = "linux",
    target_os = "macos"
  )))]
  {
    let _ = active_window_title;
    (None, None)
  }
}

#[cfg(target_os = "windows")]
fn identify_active_application_windows(
  active_window_title: &str,
) -> (Option<String>, Option<String>) {
  if active_window_title.is_empty() {
    return (None, None);
  }

  let escaped_title = active_window_title.replace('\'', "''");
  let ps_script = format!(
    "$p = Get-Process | Where-Object {{$_.MainWindowTitle -eq '{}' }} | Select-Object -First 1; if ($null -ne $p) {{ $desc = $p.MainModule.FileVersionInfo.FileDescription; Write-Output ($p.ProcessName + \"`t\" + $desc) }}",
    escaped_title
  );

  let output = Command::new("powershell")
    .args(["-NoProfile", "-Command", ps_script.as_str()])
    .output();

  parse_app_metadata_from_tsv_output(output)
}

#[cfg(target_os = "linux")]
fn identify_active_application_linux(
  active_window_title: &str,
) -> (Option<String>, Option<String>) {
  let _ = active_window_title;
  let shell_cmd = "pid=$(xdotool getactivewindow getwindowpid 2>/dev/null) || exit 0; comm=$(ps -p \"$pid\" -o comm= 2>/dev/null); cmd=$(ps -p \"$pid\" -o args= 2>/dev/null); printf \"%s\\t%s\" \"$comm\" \"$cmd\"";

  let output = Command::new("sh").args(["-c", shell_cmd]).output();
  parse_app_metadata_from_tsv_output(output)
}

#[cfg(target_os = "macos")]
fn identify_active_application_macos(
  active_window_title: &str,
) -> (Option<String>, Option<String>) {
  let _ = active_window_title;
  let script = r#"tell application \"System Events\"
set frontApp to first application process whose frontmost is true
set appName to name of frontApp
set appPath to \"\"
try
  set appPath to POSIX path of (file of frontApp as alias)
end try
return appName & tab & appPath
end tell"#;

  let output = Command::new("osascript").args(["-e", script]).output();
  let (name, path) = parse_app_metadata_from_tsv_output(output);
  let description = path.and_then(|p| {
    Path::new(&p)
      .file_stem()
      .and_then(|s| s.to_str())
      .map(|s| s.to_owned())
  });

  (name, description)
}

fn parse_app_metadata_from_tsv_output(
  output: std::io::Result<std::process::Output>,
) -> (Option<String>, Option<String>) {
  let Ok(output) = output else {
    return (None, None);
  };

  if !output.status.success() {
    return (None, None);
  }

  let text = String::from_utf8_lossy(&output.stdout);
  let line = text
    .lines()
    .find(|l| !l.trim().is_empty())
    .unwrap_or_default();
  if line.is_empty() {
    return (None, None);
  }

  let mut parts = line.splitn(2, '\t');
  let app_name = parts
    .next()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_owned);
  let app_description = parts
    .next()
    .map(str::trim)
    .filter(|s| !s.is_empty())
    .map(str::to_owned);

  (app_name, app_description)
}

fn post_process_transcript(
  transcript_text: &str,
  active_app_info: &ActiveApplicationInfo,
) -> Result<String, ()> {
  debug!(
    window_title = %active_app_info.window_title,
    app_name = ?active_app_info.application_name,
    app_description = ?active_app_info.application_description,
    "post processing transcript for active app context"
  );

  let cfg = settings::current().transcription;
  let reformatting_level = &cfg.transcript_reformatting_level;
  if matches!(reformatting_level, TranscriptReformattingLevel::None) {
    debug!(
      "transcript reformatting level is none; skipping llm post-processing"
    );
    return Ok(transcript_text.to_owned());
  }

  if cfg.llm_base_url.trim().is_empty()
    || cfg.llm_model_name.trim().is_empty()
    || cfg.llm_custom_prompt.trim().is_empty()
  {
    error!(
      level = ?reformatting_level,
      base_url = %cfg.llm_base_url,
      model = %cfg.llm_model_name,
      "llm post processing requires model settings when reformatting level is not none"
    );
    return Err(());
  }

  let llm_cfg = LlmPostProcessorConfig {
    api_key: cfg.llm_api_key,
    base_url: cfg.llm_base_url,
    model_name: cfg.llm_model_name,
    custom_prompt: cfg.llm_custom_prompt,
    system_prompt: cfg.llm_system_prompt,
    reformatting_level: reformatting_level_label(reformatting_level).to_owned(),
  };
  let app_context = LlmAppContext {
    window_title: active_app_info.window_title.clone(),
    application_name: active_app_info.application_name.clone(),
    application_description: active_app_info.application_description.clone(),
  };

  process_transcript_with_llm(&llm_cfg, transcript_text, &app_context)
    .map_err(|_| ())
}

fn reformatting_level_label(
  level: &TranscriptReformattingLevel,
) -> &'static str {
  match level {
    TranscriptReformattingLevel::None => "none",
    TranscriptReformattingLevel::Minimal => "minimal",
    TranscriptReformattingLevel::Normal => "normal",
    TranscriptReformattingLevel::Freeform => "freeform",
  }
}

fn paste_from_clipboard_into_active_input_field() -> Result<(), String> {
  let mut enigo = Enigo::new(&Settings::default())
    .map_err(|e| format!("enigo init failed: {e}"))?;

  #[cfg(target_os = "macos")]
  {
    enigo
      .key(Key::Meta, Direction::Press)
      .map_err(|e| format!("meta press failed: {e}"))?;
    enigo
      .key(Key::Unicode('v'), Direction::Click)
      .map_err(|e| format!("v click failed: {e}"))?;
    enigo
      .key(Key::Meta, Direction::Release)
      .map_err(|e| format!("meta release failed: {e}"))?;
  }

  #[cfg(not(target_os = "macos"))]
  {
    enigo
      .key(Key::Control, Direction::Press)
      .map_err(|e| format!("control press failed: {e}"))?;
    enigo
      .key(Key::Unicode('v'), Direction::Click)
      .map_err(|e| format!("v click failed: {e}"))?;
    enigo
      .key(Key::Control, Direction::Release)
      .map_err(|e| format!("control release failed: {e}"))?;
  }

  Ok(())
}

fn should_update_clipboard(current: Option<&str>, next: &str) -> bool {
  current != Some(next)
}

fn update_clipboard_if_changed(text: &str) -> Result<(), String> {
  let mut clipboard =
    Clipboard::new().map_err(|e| format!("clipboard init failed: {e}"))?;
  let current_text = clipboard.get_text().ok();
  if should_update_clipboard(current_text.as_deref(), text) {
    clipboard
      .set_text(text.to_owned())
      .map_err(|e| format!("set clipboard failed: {e}"))?;
  }
  Ok(())
}

fn run_model_download(progress: Arc<AtomicU32>) {
  info!("model download started");
  progress.store(0, Ordering::Relaxed);

  let model_dir = model_dir_path();
  if std::fs::create_dir_all(&model_dir).is_err() {
    error!(model_dir = %model_dir.display(), "failed to create model dir");
    return;
  }

  let _ = std::fs::remove_file(model_success_flag_path());

  let endpoints = MODEL_FILES.iter().map(|filename| {
    (
      *filename,
      format!(
        "https://huggingface.co/istupakov/parakeet-tdt-0.6b-v3-onnx/resolve/main/{}?download=true",
        filename
      ),
    )
  });

  let endpoint_list: Vec<(&str, String)> = endpoints.collect();
  let client = match Client::builder()
    .connect_timeout(Duration::from_secs(15))
    .timeout(Duration::from_secs(600))
    .redirect(Policy::limited(10))
    .user_agent("dictation-rs/0.1")
    .build()
  {
    Ok(client) => client,
    Err(e) => {
      error!(error = %e, "failed to create http client");
      return;
    }
  };

  let mut total_bytes: u64 = 0;
  for (_, url) in &endpoint_list {
    let size = client
      .head(url)
      .send()
      .ok()
      .and_then(|resp| {
        resp
          .headers()
          .get(reqwest::header::CONTENT_LENGTH)
          .and_then(|v| v.to_str().ok())
          .and_then(|s| s.parse::<u64>().ok())
      })
      .unwrap_or(0);
    total_bytes = total_bytes.saturating_add(size);
  }

  let mut downloaded_bytes: u64 = 0;

  for (filename, url) in endpoint_list {
    let mut response = match client.get(url).send() {
      Ok(resp) if resp.status().is_success() => resp,
      Ok(resp) => {
        error!(filename, status = %resp.status(), "download failed");
        return;
      }
      Err(e) => {
        error!(filename, error = %e, "download request failed");
        return;
      }
    };

    if total_bytes == 0 {
      total_bytes =
        total_bytes.saturating_add(response.content_length().unwrap_or(0));
    }

    let mut file = match std::fs::File::create(model_dir.join(filename)) {
      Ok(f) => f,
      Err(_) => return,
    };

    let mut buffer = [0_u8; 64 * 1024];
    loop {
      let read = match response.read(&mut buffer) {
        Ok(0) => break,
        Ok(n) => n,
        Err(e) => {
          error!(filename, error = %e, "download stream read failed");
          return;
        }
      };

      if file.write_all(&buffer[..read]).is_err() {
        error!(filename, "writing downloaded file failed");
        return;
      }

      downloaded_bytes = downloaded_bytes.saturating_add(read as u64);
      if total_bytes > 0 {
        let pct = ((downloaded_bytes.saturating_mul(100)) / total_bytes)
          .min(100) as u32;
        progress.store(pct, Ordering::Relaxed);
      }
    }
  }

  progress.store(100, Ordering::Relaxed);
  let _ = std::fs::write(model_success_flag_path(), b"downloaded");
  info!("model download completed");
}

fn run_transcription(status_slot: Arc<Mutex<Option<bool>>>) {
  debug!("run_transcription invoked");
  let is_error = transcribe_call().is_err();
  if let Ok(mut slot) = status_slot.lock() {
    *slot = Some(is_error);
  }
}

impl VoiceApp {
  pub(crate) fn spawn_model_download_worker_if_needed(&mut self) {
    if self.download_spawned {
      debug!("model download worker already spawned");
      return;
    }

    self.download_spawned = true;
    let progress = self.download_progress_atomic.clone();
    std::thread::spawn(move || run_model_download(progress));
  }

  pub(crate) fn spawn_transcription_worker_if_needed(&mut self) {
    if self.transcription_spawned {
      debug!("transcription worker already spawned");
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

  #[test]
  fn test_cache_entry_expiry_logic() {
    let now = Instant::now();
    let ttl = Duration::from_secs(600);

    let fresh = now - Duration::from_secs(100);
    let expired = now - Duration::from_secs(700);

    assert!(!is_cache_entry_expired(fresh, now, ttl));
    assert!(is_cache_entry_expired(expired, now, ttl));
  }

  #[test]
  fn test_model_cache_ttl_reads_env_override() {
    unsafe { std::env::set_var("DICTATION_MODEL_CACHE_TTL_SECS", "5") };
    assert_eq!(model_cache_ttl(), Duration::from_secs(5));
    unsafe { std::env::remove_var("DICTATION_MODEL_CACHE_TTL_SECS") };
  }

  #[test]
  fn test_model_downloaded_false_when_success_flag_absent() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let model_dir = model_dir_path();
    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );
    std::fs::create_dir_all(&model_dir).expect("should create model dir");
    for file in MODEL_FILES {
      std::fs::write(model_dir.join(file), b"x")
        .expect("should create model file");
    }

    assert!(!is_model_downloaded());

    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );
  }

  #[test]
  fn test_model_downloaded_true_when_files_and_success_flag_present() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let model_dir = model_dir_path();
    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );
    std::fs::create_dir_all(&model_dir).expect("should create model dir");
    for file in MODEL_FILES {
      if file.ends_with(".onnx") {
        std::fs::write(model_dir.join(file), vec![7_u8; 2048])
          .expect("should create sane onnx model file");
      } else {
        std::fs::write(model_dir.join(file), b"x")
          .expect("should create model file");
      }
    }
    std::fs::write(model_success_flag_path(), b"downloaded")
      .expect("should create model flag");

    assert!(is_model_downloaded());

    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );
  }

  #[test]
  fn test_model_downloaded_false_when_onnx_is_html() {
    let _guard = match TEST_CWD_LOCK.lock() {
      Ok(g) => g,
      Err(e) => e.into_inner(),
    };
    let model_dir = model_dir_path();
    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );
    std::fs::create_dir_all(&model_dir).expect("should create model dir");
    for file in MODEL_FILES {
      if file == "encoder-model.int8.onnx" {
        std::fs::write(model_dir.join(file), b"<html>bad response</html>")
          .expect("should create bad onnx file");
      } else if file.ends_with(".onnx") {
        std::fs::write(model_dir.join(file), vec![7_u8; 2048])
          .expect("should create sane onnx model file");
      } else {
        std::fs::write(model_dir.join(file), b"x")
          .expect("should create model file");
      }
    }
    std::fs::write(model_success_flag_path(), b"downloaded")
      .expect("should create model flag");

    assert!(!is_model_downloaded());

    let _ = std::fs::remove_dir_all(
      model_dir.parent().unwrap_or(model_dir.as_path()),
    );
  }

  #[test]
  fn test_post_process_transcript_returns_input_text() {
    let _guard = TEST_CWD_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());

    let mut cfg = settings::current();
    cfg.transcription.transcript_reformatting_level =
      TranscriptReformattingLevel::None;
    let json =
      serde_json::to_string_pretty(&cfg).expect("should serialize settings");
    std::fs::write(settings::settings_path(), json)
      .expect("should write settings file");
    let _ = settings::refresh_from_disk();

    let app_info = ActiveApplicationInfo {
      window_title: "Some Window".to_owned(),
      application_name: Some("Editor".to_owned()),
      application_description: Some("Code Editor".to_owned()),
    };
    let out = post_process_transcript("hello world", &app_info)
      .expect("post processing should succeed");
    assert_eq!(out, "hello world");
  }

  #[test]
  fn test_post_process_transcript_returns_err_when_enabled_with_missing_model_name()
   {
    let _guard = TEST_CWD_LOCK
      .lock()
      .unwrap_or_else(|poisoned| poisoned.into_inner());

    let app_info = ActiveApplicationInfo {
      window_title: "Some Window".to_owned(),
      application_name: Some("Editor".to_owned()),
      application_description: Some("Code Editor".to_owned()),
    };

    let mut cfg = settings::current();
    cfg.transcription.transcript_reformatting_level =
      TranscriptReformattingLevel::Minimal;
    cfg.transcription.llm_model_name = String::new();
    cfg.transcription.llm_base_url = "http://localhost:11434/v1".to_owned();
    cfg.transcription.llm_custom_prompt = "Fix transcript".to_owned();

    let json =
      serde_json::to_string_pretty(&cfg).expect("should serialize settings");
    std::fs::write(settings::settings_path(), json)
      .expect("should write settings file");
    let _ = settings::refresh_from_disk();

    let out = post_process_transcript("hello world", &app_info);
    assert!(out.is_err());
  }

  #[test]
  fn test_parse_app_metadata_from_tsv_output_handles_empty() {
    let out = parse_app_metadata_from_tsv_output(Ok(std::process::Output {
      status: std::process::ExitStatus::default(),
      stdout: Vec::new(),
      stderr: Vec::new(),
    }));
    assert_eq!(out, (None, None));
  }

  #[test]
  fn test_should_update_clipboard_when_different() {
    assert!(should_update_clipboard(Some("a"), "b"));
    assert!(should_update_clipboard(None, "b"));
    assert!(!should_update_clipboard(Some("b"), "b"));
  }

  #[test]
  fn test_parse_dictionary_entry_accepts_multiple_separators() {
    assert_eq!(
      parse_dictionary_entry("lang chain=>LangChain"),
      Some(("lang chain".to_owned(), "LangChain".to_owned()))
    );
    assert_eq!(
      parse_dictionary_entry("langchain->LangChain"),
      Some(("langchain".to_owned(), "LangChain".to_owned()))
    );
    assert_eq!(
      parse_dictionary_entry("llm=LLM"),
      Some(("llm".to_owned(), "LLM".to_owned()))
    );
  }

  #[test]
  fn test_build_dictionary_rules_user_overrides_built_in() {
    let built_in = vec!["langchain=>Lang Chain".to_owned()];
    let user = vec!["langchain=>LangChain".to_owned()];

    let rules = build_dictionary_rules(&built_in, &user);
    assert!(rules.contains(&("langchain".to_owned(), "LangChain".to_owned())));
    assert!(
      !rules.contains(&("langchain".to_owned(), "Lang Chain".to_owned()))
    );
  }

  #[test]
  fn test_replace_case_insensitive_replaces_all_matches() {
    let out = replace_case_insensitive(
      "we use Lang Chain and lang chain daily",
      "lang chain",
      "LangChain",
    );
    assert_eq!(out, "we use LangChain and LangChain daily");
  }
}
