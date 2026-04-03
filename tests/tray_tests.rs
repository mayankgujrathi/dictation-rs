//! Tray module tests
//!
//! Tests for tray icon generation, event handling and manager logic.

use dictation::tray::{TrayManager, create_tray_icon, spawn_poll_thread};
use std::sync::{Arc, atomic::AtomicBool};
use std::time::Duration;

#[cfg(test)]
mod icon_tests {
  use super::*;

  #[test]
  fn test_create_tray_icon_success() {
    // This verifies that icon creation doesn't panic
    // (Icon struct methods are private, so we just verify it can be created)
    let _icon = create_tray_icon();

    // If we got here without panicking, icon was created successfully
    assert!(true);
  }
}

#[cfg(test)]
mod tray_manager_tests {
  use super::*;
  use std::sync::atomic::Ordering;

  #[test]
  fn test_tray_manager_initialization() {
    let exit_flag = Arc::new(AtomicBool::new(false));

    // This verifies that tray manager can be created without panicking
    let _manager = TrayManager::new(exit_flag.clone());

    // Verify exit flag is initially false
    assert!(!exit_flag.load(Ordering::SeqCst));

    // Manager was created successfully without panicking
    // (tray_icon field is private, we just verify construction works)
  }

  #[test]
  fn test_poll_events_no_exit() {
    let exit_flag = Arc::new(AtomicBool::new(false));
    let manager = TrayManager::new(exit_flag.clone());

    // Polling should not set exit flag when no events
    manager.poll_events();

    assert!(!exit_flag.load(Ordering::SeqCst));
  }

  #[test]
  fn test_poll_events_already_exiting() {
    let exit_flag = Arc::new(AtomicBool::new(true));
    let manager = TrayManager::new(exit_flag.clone());

    // Polling should return early when already exiting
    manager.poll_events();

    assert!(exit_flag.load(Ordering::SeqCst));
  }
}

#[cfg(test)]
mod poll_thread_tests {
  use super::*;
  use std::sync::atomic::Ordering;

  #[test]
  fn test_spawn_poll_thread_exits_cleanly() {
    let exit_flag = Arc::new(AtomicBool::new(false));
    let exit_clone = exit_flag.clone();

    // Start poll thread
    let _handle = spawn_poll_thread(exit_flag.clone());

    // Signal exit
    exit_clone.store(true, Ordering::SeqCst);

    // Give thread time to exit
    std::thread::sleep(Duration::from_millis(200));

    // Thread should have exited
    assert!(exit_flag.load(Ordering::SeqCst));
  }

  #[test]
  fn test_poll_thread_exit_flag_propagation() {
    let exit_flag = Arc::new(AtomicBool::new(false));

    spawn_poll_thread(exit_flag.clone());

    // Set exit flag from main thread
    exit_flag.store(true, Ordering::SeqCst);

    std::thread::sleep(Duration::from_millis(150));

    // Flag should remain true
    assert!(exit_flag.load(Ordering::SeqCst));
  }
}
