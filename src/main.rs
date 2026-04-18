mod app;
mod audio;
mod llm;
mod logging;
mod settings;
mod tray;

use std::io::Write;
use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use eframe::egui;
use single_instance::SingleInstance;
use tracing::{debug, error, info, warn};

fn main() -> eframe::Result<()> {
  settings::initialize();
  if let Err(e) = logging::init_logging() {
    // Fallback path before logger is available.
    let _ = std::io::stderr()
      .write_all(format!("Failed to initialize logging: {e}\n").as_bytes());
  }
  info!("application startup initiated");

  // Prevent launching multiple app instances.
  let instance = SingleInstance::new("dictation-rs-single-instance")
    .expect("Failed to create app instance lock");
  if !instance.is_single() {
    warn!("dictation is already running; exiting duplicate instance");
    return Ok(());
  }

  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("dictation-worker")
    .worker_threads(4)
    .max_blocking_threads(4)
    .build()
    .expect("Failed to build Tokio runtime");
  info!("tokio runtime initialized");

  let _guard = runtime.enter();

  // Shared exit flag
  let should_exit = Arc::new(AtomicBool::new(false));

  // Set up tray icon on main thread
  let _tray_manager = tray::TrayManager::new(should_exit.clone());
  info!("tray initialized");

  // Spawn background thread for tray event polling
  tray::spawn_poll_thread(should_exit.clone());
  debug!("tray polling thread spawned");

  let should_exit_for_settings = should_exit.clone();
  let _settings_refresh_handle = runtime.spawn(async move {
    while !should_exit_for_settings.load(Ordering::SeqCst) {
      logging::apply_runtime_logging_settings();
      tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
  });

  // Recording state
  let recording_state = audio::RecordingState::new();
  let volume_level = recording_state.volume_level.clone();
  let is_recording = recording_state.is_recording.clone();
  let mic_ready = recording_state.mic_ready.clone();
  let recording_ready = recording_state.recording_ready.clone();

  // Set up global keyboard listener for start/stop toggle:
  // Windows/Linux: Ctrl + `
  // macOS: Command (Meta) + `
  let recording_state_clone = recording_state.clone();
  let should_exit_clone = should_exit.clone();

  let _keyboard_handle = runtime.spawn_blocking(move || {
    debug!("global keyboard listener thread started");
    let mut hotkey_was_pressed = false;
    let mut modifier_pressed = false;

    #[cfg(target_os = "macos")]
    fn is_modifier_key(key: rdev::Key) -> bool {
      matches!(key, rdev::Key::MetaLeft | rdev::Key::MetaRight)
    }

    #[cfg(not(target_os = "macos"))]
    fn is_modifier_key(key: rdev::Key) -> bool {
      matches!(key, rdev::Key::ControlLeft | rdev::Key::ControlRight)
    }

    fn is_trigger_key(key: rdev::Key) -> bool {
      matches!(key, rdev::Key::BackQuote)
    }

    if let Err(e) = rdev::listen(move |event| {
      // Check for tray exit
      if should_exit_clone.load(Ordering::SeqCst) {
        return;
      }

      // Check for configured hotkey key press/release.
      if let rdev::EventType::KeyPress(key) = event.event_type {
        if is_modifier_key(key) {
          modifier_pressed = true;
          return;
        }

        if modifier_pressed && is_trigger_key(key) && !hotkey_was_pressed {
          hotkey_was_pressed = true;
          info!("recording hotkey trigger received");

          if !app::is_model_ready() {
            warn!("hotkey ignored because speech model is not ready");
            return;
          }

          // Toggle recording
          if recording_state_clone.is_recording() {
            // Stop recording
            recording_state_clone.set_recording(false);
          } else {
            // Start recording
            recording_state_clone.record();
          }
        }
      } else if let rdev::EventType::KeyRelease(key) = event.event_type {
        if is_trigger_key(key) {
          hotkey_was_pressed = false;
        } else if is_modifier_key(key) {
          modifier_pressed = false;
          hotkey_was_pressed = false;
        }
      }
    }) {
      error!(error = ?e, "failed to start global keyboard listener");
    }
  });

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size(app::WINDOW_INNER_SIZE)
      .with_decorations(false)
      .with_transparent(true)
      .with_always_on_top()
      .with_position(egui::pos2(0.0, 0.0))
      .with_taskbar(false)
      .with_active(false)
      .with_visible(false),
    ..Default::default()
  };

  // Keep tray manager alive
  std::mem::forget(_tray_manager);

  // Create VoiceApp with new parameters
  let result = eframe::run_native(
    "Voice Widget",
    options,
    Box::new(move |_cc| {
      Box::new(app::VoiceApp::new(
        volume_level,
        is_recording,
        mic_ready,
        recording_ready,
        should_exit,
      ))
    }),
  );

  // Shutdown runtime with a timeout to ensure clean exit
  runtime.shutdown_timeout(std::time::Duration::from_millis(500));
  info!("runtime shutdown completed");
  logging::enforce_app_log_retention();

  result
}
