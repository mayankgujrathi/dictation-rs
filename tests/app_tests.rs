//! App module tests
//!
//! Tests for the VoiceApp UI component logic and constants.

use std::collections::VecDeque;
use std::sync::{Arc, atomic::Ordering};

/// Constants mirrored from app.rs for testing
pub const WINDOW_INNER_SIZE: [f32; 2] = [100.0, 40.0];
pub const HISTORY_LEN: usize = 8;

/// Simulates the volume level update logic from VoiceApp::update
pub struct VolumeHistory {
  history: VecDeque<f32>,
  capacity: usize,
}

impl VolumeHistory {
  pub fn new(capacity: usize) -> Self {
    Self {
      history: VecDeque::from(vec![0.0; capacity]),
      capacity,
    }
  }

  /// Updates history with new volume reading
  /// Mirrors the logic: current_vol = volume_atomic.load() as f32 / 1000.0
  pub fn update(&mut self, volume_raw: u32) {
    let current_vol = volume_raw as f32 / 1000.0;
    self.history.push_back(current_vol);
    if self.history.len() > self.capacity {
      self.history.pop_front();
    }
  }

  pub fn get_history(&self) -> &VecDeque<f32> {
    &self.history
  }

  pub fn len(&self) -> usize {
    self.history.len()
  }

  pub fn is_empty(&self) -> bool {
    self.history.is_empty()
  }
}

/// Calculates the bar height for visualization
/// Mirrors the logic in app.rs update method
pub fn calculate_bar_height(amplitude: f32, max_height: f32) -> f32 {
  (amplitude * max_height * 4.0).clamp(2.0, max_height * 0.9)
}

/// Calculates the x position for a bar in the visualization
pub fn calculate_bar_x(
  index: usize,
  num_bars: usize,
  rect_width: f32,
  rect_left: f32,
) -> f32 {
  let spacing = rect_width / num_bars as f32;
  rect_left + (index as f32 * spacing) + (spacing / 2.0)
}

#[cfg(test)]
mod constants_tests {
  use super::*;

  #[test]
  fn test_window_inner_size_dimensions() {
    assert_eq!(WINDOW_INNER_SIZE.len(), 2);
    assert_eq!(WINDOW_INNER_SIZE[0], 100.0); // Width
    assert_eq!(WINDOW_INNER_SIZE[1], 40.0); // Height
  }

  #[test]
  fn test_history_length() {
    assert_eq!(HISTORY_LEN, 8);
  }
}

#[cfg(test)]
mod volume_history_tests {
  use super::*;

  #[test]
  fn test_volume_history_initialization() {
    let history = VolumeHistory::new(HISTORY_LEN);
    assert_eq!(history.len(), HISTORY_LEN);
    assert!(!history.is_empty());

    // All initial values should be 0.0
    for val in history.get_history() {
      assert!((val - 0.0).abs() < f32::EPSILON);
    }
  }

  #[test]
  fn test_volume_history_update() {
    let mut history = VolumeHistory::new(HISTORY_LEN);

    // Update with some volume levels
    history.update(0); // 0.0
    history.update(500); // 0.5
    history.update(1000); // 1.0

    assert_eq!(history.len(), HISTORY_LEN);

    // Most recent value should be the last update
    let last = history.get_history().back();
    assert!(last.is_some());
    assert!((last.unwrap() - 1.0).abs() < f32::EPSILON);
  }

  #[test]
  fn test_volume_history_max_capacity() {
    let capacity = 4;
    let mut history = VolumeHistory::new(capacity);

    // Add more updates than capacity
    for i in 0..10 {
      history.update(i * 100);
    }

    // Should still only have 'capacity' items
    assert_eq!(history.len(), capacity);
  }

  #[test]
  fn test_volume_history_fifo_behavior() {
    let capacity = 4;
    let mut history = VolumeHistory::new(capacity);

    // Fill with distinct values
    for i in 0..capacity {
      history.update((i + 1) as u32 * 100);
    }

    // First value should have been pushed out after 5th update
    history.update(500);

    // Check that the oldest value is no longer present
    let h = history.get_history();
    assert!(!h.contains(&100.0));
  }

  #[test]
  fn test_volume_raw_value_conversion() {
    // Test the conversion: raw_value / 1000.0 = volume_level
    let mut history = VolumeHistory::new(HISTORY_LEN);

    // Test various raw values
    history.update(0); // 0.0
    history.update(250); // 0.25
    history.update(500); // 0.5
    history.update(750); // 0.75
    history.update(1000); // 1.0
    history.update(1500); // 1.5 (clamped in UI)

    let last = history.get_history().back().unwrap();
    assert!((last - 1.5).abs() < f32::EPSILON);
  }
}

#[cfg(test)]
mod visualization_tests {
  use super::*;

  #[test]
  fn test_bar_height_calculation() {
    let max_height = 100.0;

    // Zero amplitude should give minimum height
    let h = calculate_bar_height(0.0, max_height);
    assert_eq!(h, 2.0);

    // Low amplitude
    let h = calculate_bar_height(0.1, max_height);
    assert_eq!(h, 40.0); // 0.1 * 100 * 4 = 40, clamped to max 0.9 * 100 = 90

    // Medium amplitude - 0.25 * 100 * 4 = 100, but clamped to 90
    let h = calculate_bar_height(0.25, max_height);
    assert_eq!(h, 90.0); // Clamped to max * 0.9

    // High amplitude
    let h = calculate_bar_height(0.5, max_height);
    assert_eq!(h, 90.0); // 0.5 * 100 * 4 = 200, clamped to 90 (max * 0.9)
  }

  #[test]
  fn test_bar_height_minimum() {
    let max_height = 100.0;

    // Very small amplitude should still have minimum height
    let h = calculate_bar_height(0.001, max_height);
    assert_eq!(h, 2.0); // Minimum is always 2.0
  }

  #[test]
  fn test_bar_height_maximum() {
    let max_height = 100.0;

    // Large amplitude should be clamped
    let h = calculate_bar_height(1.0, max_height);
    assert_eq!(h, 90.0); // max * 0.9 = 100 * 0.9 = 90
  }

  #[test]
  fn test_bar_x_position() {
    let rect_width = 100.0;
    let rect_left = 0.0;
    let num_bars = 8;

    let positions: Vec<f32> = (0..num_bars)
      .map(|i| calculate_bar_x(i, num_bars, rect_width, rect_left))
      .collect();

    // All positions should be within the rect
    for pos in &positions {
      assert!(*pos >= 0.0 && *pos <= rect_width);
    }

    // First bar should be centered in first slot
    assert!((positions[0] - 6.25).abs() < 0.01); // spacing/2 = 100/16 = 6.25
  }

  #[test]
  fn test_bar_x_position_with_offset() {
    let rect_width = 100.0;
    let rect_left = 50.0;
    let num_bars = 4;

    let pos = calculate_bar_x(0, num_bars, rect_width, rect_left);
    assert!((pos - 62.5).abs() < 0.01); // 50 + 100/8 = 62.5
  }
}

#[cfg(test)]
mod atomic_integration_tests {
  use super::*;
  use std::sync::atomic::{AtomicBool, AtomicU32};

  #[test]
  fn test_volume_atomic_load_store() {
    let volume = Arc::new(AtomicU32::new(0));

    // Store a value
    volume.store(500, Ordering::Relaxed);
    assert_eq!(volume.load(Ordering::Relaxed), 500);

    // Convert to volume level
    let level = volume.load(Ordering::Relaxed) as f32 / 1000.0;
    assert!((level - 0.5).abs() < f32::EPSILON);
  }

  #[test]
  fn test_running_flag() {
    let running = Arc::new(AtomicBool::new(true));

    assert!(running.load(Ordering::SeqCst));

    running.store(false, Ordering::SeqCst);
    assert!(!running.load(Ordering::SeqCst));
  }

  #[test]
  fn test_shared_atomics() {
    let volume = Arc::new(AtomicU32::new(0));
    let running = Arc::new(AtomicBool::new(true));

    let vol_clone = volume.clone();
    let run_clone = running.clone();

    // Simulate audio thread updating
    vol_clone.store(750, Ordering::Relaxed);
    run_clone.store(false, Ordering::SeqCst);

    // Main thread should see updates
    assert_eq!(volume.load(Ordering::Relaxed), 750);
    assert!(!running.load(Ordering::SeqCst));
  }

  #[test]
  fn test_volume_to_display_conversion() {
    // Test the full conversion pipeline
    let volume_raw = 427u32;
    let volume_display = volume_raw as f32 / 1000.0;

    // Display value should be used for history
    let mut history = VolumeHistory::new(8);
    history.update(volume_raw);

    let last = history.get_history().back().unwrap();
    assert!((last - volume_display).abs() < f32::EPSILON);
  }
}

#[cfg(test)]
mod frame_calculation_tests {
  #[test]
  fn test_frame_duration_16khz() {
    let sample_rate = 16000u32;
    let samples_per_frame = 512;
    let frame_duration_ms =
      (samples_per_frame as f64 / sample_rate as f64) * 1000.0;

    // 512 samples at 16kHz ≈ 32ms
    assert!((frame_duration_ms - 32.0).abs() < 0.1);
  }

  #[test]
  fn test_samples_for_duration() {
    let duration_ms = 1000u32;
    let sample_rate = 16000u32;
    let expected_samples =
      (duration_ms as f64 / 1000.0 * sample_rate as f64) as usize;

    assert_eq!(expected_samples, 16000);
  }
}
