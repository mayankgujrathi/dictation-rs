use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};

use tracing::{debug, info};
use tray_icon::{
  Icon, TrayIcon, TrayIconBuilder,
  menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

const TRAY_ICON_PNG: &[u8] = include_bytes!("../assets/activity.png");

/// Create tray icon from an embedded PNG asset.
pub fn create_tray_icon() -> Icon {
  let image = image::load_from_memory(TRAY_ICON_PNG)
    .expect("Failed to decode embedded tray icon PNG")
    .into_rgba8();
  let (width, height) = image.dimensions();
  let icon = Icon::from_rgba(image.into_raw(), width, height)
    .expect("Failed to create icon from rgba");
  debug!("tray icon created");
  icon
}

/// Tray manager that holds the tray icon and handles events
pub struct TrayManager {
  _tray_icon: Option<TrayIcon>,
}

impl TrayManager {
  pub fn new(_exit_requested: Arc<AtomicBool>) -> Self {
    info!("initializing tray manager");
    let icon = create_tray_icon();
    let exit_item = MenuItem::with_id("exit", "Exit", true, None);
    let about_item = MenuItem::with_id("about", "About", true, None);

    let menu = Menu::new();
    menu.append(&exit_item).unwrap();
    menu.append(&PredefinedMenuItem::separator()).unwrap();
    menu.append(&about_item).unwrap();

    let tray_icon = TrayIconBuilder::new()
      .with_menu(Box::new(menu))
      .with_tooltip("dictation-rs")
      .with_icon(icon)
      .build()
      .expect("Failed to create tray icon");
    info!("tray icon ready");

    Self {
      _tray_icon: Some(tray_icon),
    }
  }

  /// Test-only constructor that avoids creating real GTK-backed tray resources.
  ///
  /// This keeps unit tests deterministic in headless CI environments where
  /// GTK may be unavailable or not initialized.
  #[allow(dead_code)]
  pub fn new_for_test(_exit_requested: Arc<AtomicBool>) -> Self {
    info!("initializing tray manager in test mode (no OS tray resources)");
    Self { _tray_icon: None }
  }
}

/// Spawn a background thread to poll for tray events
pub fn spawn_poll_thread(exit_requested: Arc<AtomicBool>) {
  std::thread::spawn(move || {
    debug!("tray polling thread started");
    loop {
      if exit_requested.load(Ordering::SeqCst) {
        info!("tray polling thread exiting due to app exit flag");
        break;
      }

      let menu_receiver = MenuEvent::receiver();
      if let Ok(event) = menu_receiver.try_recv()
        && event.id.as_ref() == "exit"
      {
        info!("tray exit command received");
        exit_requested.store(true, Ordering::SeqCst);
        break;
      }

      std::thread::sleep(std::time::Duration::from_millis(100));
    }
  });
}
