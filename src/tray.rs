use std::sync::{
  Arc,
  atomic::{AtomicBool, Ordering},
};
use std::time::Duration;

use tracing::{debug, info, warn};
use tray_icon::{
  Icon, TrayIcon, TrayIconBuilder,
  menu::{Menu, MenuEvent, MenuItem, PredefinedMenuItem},
};

use crate::app;

const TRAY_ICON_PNG: &[u8] = include_bytes!("../assets/activity.png");
const ABOUT_URL: &str = "https://github.com/mayankgujrathi/dictation-rs";

#[cfg(target_os = "windows")]
fn open_about_url() -> Result<(), String> {
  use windows_sys::Win32::Foundation::HWND;
  use windows_sys::Win32::UI::Shell::ShellExecuteW;

  fn to_wide(s: &str) -> Vec<u16> {
    s.encode_utf16().chain(std::iter::once(0)).collect()
  }

  let operation = to_wide("open");
  let url = to_wide(ABOUT_URL);

  // SAFETY: pointers are valid NUL-terminated UTF-16 buffers for duration of call.
  let result = unsafe {
    ShellExecuteW(
      std::ptr::null_mut::<core::ffi::c_void>() as HWND,
      operation.as_ptr(),
      url.as_ptr(),
      std::ptr::null(),
      std::ptr::null(),
      1,
    )
  };

  if result as usize <= 32 {
    Err(format!(
      "ShellExecuteW failed with code {}",
      result as usize
    ))
  } else {
    Ok(())
  }
}

#[cfg(target_os = "macos")]
fn open_about_url() -> Result<(), String> {
  std::process::Command::new("open")
    .arg(ABOUT_URL)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .spawn()
    .map(|_| ())
    .map_err(|e| format!("open URL failed: {e}"))
}

#[cfg(target_os = "linux")]
fn open_about_url() -> Result<(), String> {
  std::process::Command::new("xdg-open")
    .arg(ABOUT_URL)
    .stdout(std::process::Stdio::null())
    .stderr(std::process::Stdio::null())
    .spawn()
    .map(|_| ())
    .map_err(|e| format!("xdg-open URL failed: {e}"))
}

#[cfg(not(any(
  target_os = "windows",
  target_os = "macos",
  target_os = "linux"
)))]
fn open_about_url() -> Result<(), String> {
  Err("about URL open not supported on this OS".to_string())
}

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
    let menu_receiver = MenuEvent::receiver();
    loop {
      if exit_requested.load(Ordering::SeqCst) {
        info!("tray polling thread exiting due to app exit flag");
        break;
      }

      if let Ok(event) = menu_receiver.recv_timeout(Duration::from_millis(1000))
      {
        match event.id.as_ref() {
          "exit" => {
            info!("tray exit command received");
            exit_requested.store(true, Ordering::SeqCst);
            app::wake_ui();
            break;
          }
          "about" => {
            if let Err(e) = open_about_url() {
              warn!(error = %e, "failed to open about URL");
            }
          }
          _ => {}
        }
      }
    }
  });
}
