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

  // Clone for the audio thread
  audio::spawn_volume_monitor(volume_level.clone(), running.clone());

  let options = eframe::NativeOptions {
    viewport: egui::ViewportBuilder::default()
      .with_inner_size(app::WINDOW_INNER_SIZE)
      .with_decorations(false)
      .with_transparent(true)
      .with_always_on_top()
      .with_position(egui::pos2(0.0, 0.0)),
    ..Default::default()
  };

  // Move running into the closure (VoiceApp will own it)
  eframe::run_native(
    "Voice Widget",
    options,
    Box::new(move |_cc| Box::new(app::VoiceApp::new(volume_level, running))),
  )
}
