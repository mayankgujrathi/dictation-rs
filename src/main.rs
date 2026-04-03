mod app;
mod audio;
mod tray;

use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use eframe::egui;

fn main() -> eframe::Result<()> {
  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .thread_name("dictation-worker")
    .worker_threads(4)
    .max_blocking_threads(4)
    .build()
    .expect("Failed to build Tokio runtime");

  let _guard = runtime.enter();

  // Shared exit flag
  let should_exit = Arc::new(AtomicBool::new(false));

  // Set up tray icon on main thread
  let _tray_manager = tray::TrayManager::new(should_exit.clone());

  // Spawn background thread for tray event polling
  tray::spawn_poll_thread(should_exit.clone());

  // Recording state
  let recording_state = audio::RecordingState::new();
  let volume_level = recording_state.volume_level.clone();
  let is_recording = recording_state.is_recording.clone();

  // Set up global keyboard listener for Right Alt (AltGr) toggle
  let recording_state_clone = recording_state.clone();
  let should_exit_clone = should_exit.clone();

  let _keyboard_handle = runtime.spawn_blocking(move || {
    let mut altgr_was_pressed = false;

    if let Err(e) = rdev::listen(move |event| {
      // Check for tray exit
      if should_exit_clone.load(Ordering::SeqCst) {
        return;
      }

      // Check for Right Alt (AltGr) key
      if event.event_type == rdev::EventType::KeyPress(rdev::Key::AltGr) {
        if !altgr_was_pressed {
          altgr_was_pressed = true;

          // Toggle recording
          if recording_state_clone.is_recording() {
            // Stop recording
            recording_state_clone.set_recording(false);
          } else {
            // Start recording
            recording_state_clone.record();
          }
        }
      } else if event.event_type == rdev::EventType::KeyRelease(rdev::Key::AltGr) {
        altgr_was_pressed = false;
      }
    }) {
      eprintln!("Failed to start global keyboard listener: {:?}", e);
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
    Box::new(move |_cc| Box::new(app::VoiceApp::new(volume_level, is_recording, should_exit))),
  );

  // Shutdown runtime with a timeout to ensure clean exit
  runtime.shutdown_timeout(std::time::Duration::from_millis(500));

  result
}
