use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use tray_icon::{
  Icon, TrayIcon, TrayIconBuilder,
  menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

/// Create a simple colored microphone icon for the tray (32x32)
pub fn create_tray_icon() -> Icon {
  let size = 32;
  let mut buffer = vec![0u8; (size * size * 4) as usize];

  let cx = 16;
  let cy = 16;

  // Draw filled circle for mic head (bright blue)
  for y in 8..24 {
    for x in 10..23 {
      let dx = (x as f32 - cx as f32 + 0.5) / 6.5;
      let dy = (y as f32 - cy as f32 + 2.0) / 8.0;
      if dx * dx + dy * dy <= 1.0 {
        let idx = ((y * size + x) * 4) as usize;
        buffer[idx] = 30; // R
        buffer[idx + 1] = 120; // G
        buffer[idx + 2] = 220; // B
        buffer[idx + 3] = 255; // A
      }
    }
  }

  // Draw mic handle (white)
  for y in 22..29 {
    for x in 14..19 {
      let idx = ((y * size + x) * 4) as usize;
      buffer[idx] = 240;
      buffer[idx + 1] = 240;
      buffer[idx + 2] = 250;
      buffer[idx + 3] = 255;
    }
  }

  Icon::from_rgba(buffer, size, size).expect("Failed to create icon from rgba")
}

/// Tray manager that holds the tray icon and handles events
pub struct TrayManager {
  exit_requested: Arc<AtomicBool>,
  _tray_icon: Option<TrayIcon>,
}

impl TrayManager {
  pub fn new(exit_requested: Arc<AtomicBool>) -> Self {
    let icon = create_tray_icon();
    let exit_item = MenuItem::with_id("exit", "Exit", true, None);
    let about_item = MenuItem::with_id("about", "About", true, None);

    let mut menu = Menu::new();
    menu.append(&exit_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&about_item).unwrap();

    let tray_icon = TrayIconBuilder::new()
      .with_menu(Box::new(menu))
      .with_tooltip("Dictation - Recording App")
      .with_icon(icon)
      .build()
      .expect("Failed to create tray icon");

    Self {
      exit_requested,
      _tray_icon: Some(tray_icon),
    }
  }

  /// Poll for tray events. Call this periodically from the main loop.
  pub fn poll_events(&self) {
    if self.exit_requested.load(Ordering::SeqCst) {
      return;
    }

    let menu_receiver = MenuEvent::receiver();
    if let Ok(event) = menu_receiver.try_recv() {
      if event.id.as_ref() == "exit" {
        self.exit_requested.store(true, Ordering::SeqCst);
      }
    }
  }
}

/// Spawn a background thread to poll for tray events
pub fn spawn_poll_thread(exit_requested: Arc<AtomicBool>) {
  std::thread::spawn(move || {
    loop {
      if exit_requested.load(Ordering::SeqCst) {
        break;
      }

      let menu_receiver = MenuEvent::receiver();
      if let Ok(event) = menu_receiver.try_recv() {
        if event.id.as_ref() == "exit" {
          exit_requested.store(true, Ordering::SeqCst);
          break;
        }
      }

      std::thread::sleep(std::time::Duration::from_millis(100));
    }
  });
}
