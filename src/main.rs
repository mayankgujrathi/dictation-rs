mod app;
mod audio;

use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU32},
};

use eframe::egui;

fn main() -> eframe::Result<()> {
  let volume_level = Arc::new(AtomicU32::new(0));
  let running = Arc::new(AtomicBool::new(true));

  // Set up global keyboard listener for ESC key
  let running_clone = running.clone();
  std::thread::spawn(move || {
    if let Err(e) = rdev::listen(move |event| {
      if event.event_type == rdev::EventType::KeyPress(rdev::Key::Escape) {
        running_clone.store(false, std::sync::atomic::Ordering::SeqCst);
      }
    }) {
      eprintln!("Failed to start global keyboard listener: {:?}", e);
    }
  });

  let runtime = tokio::runtime::Builder::new_multi_thread()
    .enable_all()
    .build()
    .expect("Failed to build Tokio runtime");

  let _enter_guard = runtime.enter();

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

  drop(_enter_guard);
  drop(runtime);

  result
}
