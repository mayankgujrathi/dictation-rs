mod app;
mod audio;

use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU32},
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

  let volume_level = Arc::new(AtomicU32::new(0));
  let running = Arc::new(AtomicBool::new(true));

  // Set up global keyboard listener for ESC key using tokio spawn_blocking
  let running_clone = running.clone();
  let _keyboard_handle = runtime.spawn_blocking(move || {
    if let Err(e) = rdev::listen(move |event| {
      if event.event_type == rdev::EventType::KeyPress(rdev::Key::Escape) {
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
      }
    }) {
      eprintln!("Failed to start global keyboard listener: {:?}", e);
    }
  });

  // Spawn audio volume monitor using tokio spawn_blocking
  let _audio_handle = runtime.spawn_blocking({
    let volume_level = volume_level.clone();
    let running = running.clone();
    move || audio::run_volume_monitor(volume_level, running)
  });

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size(app::WINDOW_INNER_SIZE)
      .with_decorations(false)
      .with_transparent(true)
      .with_always_on_top()
      .with_position(egui::pos2(0.0, 0.0))
      .with_taskbar(false)
      .with_active(false),
    ..Default::default()
  };

  // Move running into the closure (VoiceApp will own it)
  let result = eframe::run_native(
    "Voice Widget",
    options,
    Box::new(move |_cc| Box::new(app::VoiceApp::new(volume_level, running))),
  );

  // Shutdown runtime with a timeout to ensure clean exit
  runtime.shutdown_timeout(std::time::Duration::from_millis(500));

  result
}
