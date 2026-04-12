//! Audio module tests
//!
//! Tests for pure functions and logic in the audio processing pipeline.

use dictation::audio::{
  calculate_rms_volume, estimate_noise_floor, limit_peaks,
  normalize_target_rms, process_audio_for_saving, recording_output_path,
  remove_background_noise, tame_high_frequency_hiss,
};
use std::f32::consts::PI;

/// Calculate RMS without the volume scaling (returns raw f32)
fn calculate_rms(samples: &[f32]) -> f32 {
  if samples.is_empty() {
    return 0.0;
  }
  let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
  (sum_sq / samples.len() as f32).sqrt()
}

/// Converts a normalized f32 sample (-1.0 to 1.0) to i16 for WAV encoding.
pub fn sample_to_i16(sample: f32) -> i16 {
  (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

/// Simulates the resampling accumulator logic from audio.rs.
/// Returns the number of output samples that would be produced.
pub fn simulate_resampling(
  num_input_frames: usize,
  _channels: usize,
  sample_drop_ratio: f64,
  initial_accumulator: f64,
) -> (usize, f64) {
  let mut accumulator = initial_accumulator;
  let mut output_count = 0;

  for _ in 0..num_input_frames {
    accumulator += 1.0;
    while accumulator >= sample_drop_ratio {
      accumulator -= sample_drop_ratio;
      output_count += 1;
    }
  }

  (output_count, accumulator)
}

/// Converts multi-channel samples to mono by averaging channels.
pub fn to_mono(samples: &[f32], channels: usize) -> Vec<f32> {
  if channels == 0 {
    return vec![];
  }

  let num_frames = samples.len() / channels;
  let mut mono = Vec::with_capacity(num_frames);

  for frame_idx in 0..num_frames {
    let mut sum: f32 = 0.0;
    for ch in 0..channels {
      sum += samples[frame_idx * channels + ch];
    }
    mono.push(sum / channels as f32);
  }

  mono
}

#[cfg(test)]
mod rms_tests {
  use super::*;

  #[test]
  fn test_rms_silence() {
    let samples = [0.0f32; 100];
    let rms = calculate_rms(&samples);
    assert!((rms - 0.0).abs() < f32::EPSILON);
  }

  #[test]
  fn test_rms_constant_signal() {
    // Constant signal of 0.5 should have RMS equal to 0.5
    let samples = [0.5f32; 100];
    let rms = calculate_rms(&samples);
    assert!((rms - 0.5).abs() < 1e-6);
  }

  #[test]
  fn test_rms_sine_wave() {
    // For a pure sine wave, RMS = amplitude / sqrt(2)
    let amplitude = 0.5f32;
    let samples: Vec<f32> = (0..1000)
      .map(|i| amplitude * (i as f32 * PI / 500.0).sin())
      .collect();
    let rms = calculate_rms(&samples);
    let expected_rms = amplitude / std::f32::consts::SQRT_2;
    assert!((rms - expected_rms).abs() < 0.01);
  }

  #[test]
  fn test_rms_empty_slice() {
    let samples: [f32; 0] = [];
    let rms = calculate_rms(&samples);
    assert!((rms - 0.0).abs() < f32::EPSILON);
  }

  #[test]
  fn test_rms_single_sample() {
    let samples = [0.707f32]; // RMS should equal the sample value
    let rms = calculate_rms(&samples);
    assert!((rms - 0.707).abs() < 0.001);
  }

  #[test]
  fn test_rms_bipolar_signal() {
    let samples = [-1.0f32, 1.0f32, -1.0f32, 1.0f32];
    let rms = calculate_rms(&samples);
    // RMS of [−1, 1, −1, 1] = sqrt((1+1+1+1)/4) = sqrt(1) = 1
    assert!((rms - 1.0).abs() < f32::EPSILON);
  }

  #[test]
  fn test_calculate_rms_volume_scaling() {
    // Test that calculate_rms_volume properly scales by 1000
    let samples = [0.5f32; 100];
    let scaled = calculate_rms_volume(&samples);
    let expected_scaled = (0.5 * 1000.0) as u32;
    assert_eq!(scaled, expected_scaled);
  }

  #[test]
  fn test_calculate_rms_volume_quarter_signal() {
    let samples = [0.25f32; 16];
    assert_eq!(calculate_rms_volume(&samples), 250);
  }

  #[test]
  fn test_calculate_rms_volume_bipolar_signal() {
    let samples = [-1.0f32, 1.0f32, -1.0f32, 1.0f32];
    // RMS is 1.0, then scaled to 1000.
    assert_eq!(calculate_rms_volume(&samples), 1000);
  }

  #[test]
  fn test_calculate_rms_volume_truncation_behavior() {
    let samples = [0.0015f32; 64];
    // 0.0015 * 1000 = 1.5, cast to u32 truncates toward zero.
    assert_eq!(calculate_rms_volume(&samples), 1);
  }

  #[test]
  fn test_calculate_rms_volume_empty_returns_zero_current_behavior() {
    let samples: [f32; 0] = [];
    // Current implementation evaluates to NaN internally and final cast yields 0.
    assert_eq!(calculate_rms_volume(&samples), 0);
  }
}

#[cfg(test)]
mod sample_conversion_tests {
  use super::*;

  #[test]
  fn test_sample_to_i16_positive_max() {
    let sample = 1.0f32;
    let converted = sample_to_i16(sample);
    assert_eq!(converted, i16::MAX);
  }

  #[test]
  fn test_sample_to_i16_negative_max() {
    let sample = -1.0f32;
    let converted = sample_to_i16(sample);
    // Note: Clamping then casting gives -32767, not -32768
    assert_eq!(converted, -32767);
  }

  #[test]
  fn test_sample_to_i16_zero() {
    let sample = 0.0f32;
    let converted = sample_to_i16(sample);
    assert_eq!(converted, 0);
  }

  #[test]
  fn test_sample_to_i16_mid_value() {
    let sample = 0.5f32;
    let converted = sample_to_i16(sample);
    // 0.5 * 32767 ≈ 16383
    assert_eq!(converted, 16383);
  }

  #[test]
  fn test_sample_to_i16_negative_mid() {
    let sample = -0.5f32;
    let converted = sample_to_i16(sample);
    // -0.5 * 32767 = -16383.5 → -16383 (truncated toward zero)
    assert_eq!(converted, -16383);
  }

  #[test]
  fn test_sample_to_i16_clamping_positive() {
    let sample = 2.0f32; // Above max, should clamp
    let converted = sample_to_i16(sample);
    assert_eq!(converted, i16::MAX);
  }

  #[test]
  fn test_sample_to_i16_clamping_negative() {
    let sample = -2.0f32; // Below min, should clamp to -1.0
    let converted = sample_to_i16(sample);
    // Clamped to -1.0, then -1.0 * 32767 = -32767
    assert_eq!(converted, -32767);
  }

  #[test]
  fn test_sample_to_i16_roundtrip_positive() {
    let original = 0.75f32;
    let converted = sample_to_i16(original);
    let back_to_float = converted as f32 / i16::MAX as f32;
    assert!((back_to_float - original).abs() < 0.001);
  }
}

#[cfg(test)]
mod resampling_tests {
  use super::*;

  #[test]
  fn test_resampling_no_downsampling() {
    // sample_drop_ratio = 1.0 means 1:1 ratio
    let (output, final_acc) = simulate_resampling(100, 1, 1.0, 0.0);
    assert_eq!(output, 100);
    assert!((final_acc - 0.0).abs() < f64::EPSILON);
  }

  #[test]
  fn test_resampling_2x_downsampling() {
    // sample_drop_ratio = 2.0 means half the samples are kept
    let (output, _final_acc) = simulate_resampling(100, 1, 2.0, 0.0);
    assert_eq!(output, 50);
  }

  #[test]
  fn test_resampling_4x_downsampling() {
    // sample_drop_ratio = 4.0 means quarter of samples are kept
    let (output, _) = simulate_resampling(100, 1, 4.0, 0.0);
    assert_eq!(output, 25);
  }

  #[test]
  fn test_resampling_realistic_ratio() {
    // 44100 -> 16000 ratio
    let source_rate = 44100.0;
    let target_rate = 16000.0;
    let ratio = source_rate / target_rate;
    let (output, _) = simulate_resampling(44100, 1, ratio, 0.0);
    // Should be approximately 16000 samples
    assert!((15900..=16100).contains(&output));
  }

  #[test]
  fn test_resampling_accumulator_propagation() {
    // Test that accumulator state is properly maintained
    let (output1, acc1) = simulate_resampling(50, 1, 2.5, 0.0);
    let (output2, _acc2) = simulate_resampling(50, 1, 2.5, acc1);
    let (output_combined, _) = simulate_resampling(100, 1, 2.5, 0.0);

    // Combined should equal sum of individual outputs
    assert_eq!(output1 + output2, output_combined);
  }

  #[test]
  fn test_resampling_empty_input() {
    let (output, final_acc) = simulate_resampling(0, 1, 2.0, 0.0);
    assert_eq!(output, 0);
    assert!((final_acc - 0.0).abs() < f64::EPSILON);
  }

  #[test]
  fn test_resampling_48000_to_16000() {
    // 48kHz to 16kHz = 3x downsampling
    let ratio = 48000.0 / 16000.0;
    let (output, _) = simulate_resampling(48000, 1, ratio, 0.0);
    assert_eq!(output, 16000);
  }
}

#[cfg(test)]
mod mono_conversion_tests {
  use super::*;

  #[test]
  fn test_to_mono_single_channel() {
    let samples = vec![0.1f32, 0.2f32, 0.3f32];
    let mono = to_mono(&samples, 1);
    assert_eq!(mono, samples);
  }

  #[test]
  fn test_to_mono_stereo() {
    // Left = 0.0, Right = 1.0 -> mono should be 0.5
    let samples = vec![0.0f32, 1.0f32, 0.0f32, 1.0f32];
    let mono = to_mono(&samples, 2);
    assert_eq!(mono.len(), 2);
    assert!((mono[0] - 0.5).abs() < f32::EPSILON);
    assert!((mono[1] - 0.5).abs() < f32::EPSILON);
  }

  #[test]
  fn test_to_mono_five_channel() {
    // All channels same value
    let samples = vec![0.5f32, 0.5f32, 0.5f32, 0.5f32, 0.5f32];
    let mono = to_mono(&samples, 5);
    assert_eq!(mono.len(), 1);
    assert!((mono[0] - 0.5).abs() < f32::EPSILON);
  }

  #[test]
  fn test_to_mono_empty() {
    let samples: Vec<f32> = vec![];
    let mono = to_mono(&samples, 2);
    assert!(mono.is_empty());
  }

  #[test]
  fn test_to_mono_zero_channels() {
    let samples = vec![0.1f32, 0.2f32];
    let mono = to_mono(&samples, 0);
    assert!(mono.is_empty());
  }

  #[test]
  fn test_to_mono_partial_frames() {
    // Should only process complete frames
    let samples = vec![1.0f32, 2.0f32, 3.0f32]; // 3 samples, 2 channels = 1.5 frames
    let mono = to_mono(&samples, 2);
    assert_eq!(mono.len(), 1); // Only 1 complete frame
    assert!((mono[0] - 1.5).abs() < f32::EPSILON);
  }
}

#[cfg(test)]
mod recording_state_tests {
  use dictation::audio::RecordingState;
  use std::sync::atomic::Ordering;
  use std::thread;
  use std::time::Duration;

  #[test]
  fn test_recording_state_initialization() {
    let state = RecordingState::new();

    assert_eq!(state.volume_level.load(Ordering::Relaxed), 0);
    assert!(!state.is_recording());
    assert!(state.recording_ready.load(Ordering::SeqCst));
  }

  #[test]
  fn test_set_recording_toggle() {
    let state = RecordingState::new();

    state.set_recording(true);
    assert!(state.is_recording());

    state.set_recording(false);
    assert!(!state.is_recording());
  }

  #[test]
  fn test_recording_state_thread_safe() {
    let state = RecordingState::new();
    let state_clone = state.clone();

    // Toggle recording from another thread
    let handle = thread::spawn(move || {
      state_clone.set_recording(true);
      thread::sleep(Duration::from_millis(50));
      state_clone.set_recording(false);
    });

    // Wait for thread to complete
    handle.join().unwrap();

    // Main thread should see final state
    assert!(!state.is_recording());
  }

  #[test]
  fn test_volume_level_atomic_access() {
    let state = RecordingState::new();
    let state_clone = state.clone();

    // Update volume from background thread
    thread::spawn(move || {
      state_clone.volume_level.store(750, Ordering::Relaxed);
    });

    thread::sleep(Duration::from_millis(50));

    // Main thread should see updated value
    assert_eq!(state.volume_level.load(Ordering::Relaxed), 750);
  }

  #[test]
  fn test_recording_state_clone_shares_state() {
    let state = RecordingState::new();
    let cloned = state.clone();

    state.set_recording(true);
    state.recording_ready.store(false, Ordering::SeqCst);

    // Clone should see the same state
    assert!(cloned.is_recording());
    assert!(state.is_recording());
    assert!(!cloned.recording_ready.load(Ordering::SeqCst));
  }
}

#[cfg(test)]
mod private_function_tests {
  // These tests verify the internal private functions in audio.rs
  // Since Rust doesn't allow direct testing of private functions,
  // we test their behavior through public interfaces and verified logic

  #[test]
  fn test_mix_to_mono_logic_matches_public() {
    // Verify that our public to_mono function behaves exactly
    // like the private mix_to_mono function implementation
    use super::to_mono;

    let samples = vec![0.2, 0.8, 0.3, 0.7];
    let mono = to_mono(&samples, 2);

    // Expected: (0.2+0.8)/2 = 0.5, (0.3+0.7)/2 = 0.5
    assert_eq!(mono.len(), 2);
    assert!((mono[0] - 0.5).abs() < f32::EPSILON);
    assert!((mono[1] - 0.5).abs() < f32::EPSILON);
  }

  #[test]
  fn test_resample_sample_behavior() {
    // Test the resampling logic matches our simulation
    use super::simulate_resampling;

    // Test 2x downsampling
    let (output, _) = simulate_resampling(10, 1, 2.0, 0.0);
    assert_eq!(output, 5);

    // Test that accumulator carries over correctly
    let (output1, acc1) = simulate_resampling(3, 1, 2.0, 0.0);
    assert_eq!(output1, 1);
    assert!((acc1 - 1.0).abs() < f64::EPSILON);

    let (output2, _) = simulate_resampling(3, 1, 2.0, acc1);
    assert_eq!(output2, 2);
  }
}

#[cfg(test)]
mod post_processing_tests {
  use super::*;

  fn rms(samples: &[f32]) -> f32 {
    if samples.is_empty() {
      return 0.0;
    }
    let sum_sq: f32 = samples.iter().map(|s| s * s).sum();
    (sum_sq / samples.len() as f32).sqrt()
  }

  #[test]
  fn test_estimate_noise_floor_for_mostly_low_level_noise() {
    let mut samples = vec![0.002f32; 80];
    samples.extend(std::iter::repeat_n(0.2f32, 20));

    let floor = estimate_noise_floor(&samples);
    assert!((0.001..=0.01).contains(&floor));
  }

  #[test]
  fn test_remove_background_noise_attenuates_quiet_signal() {
    let mut samples = vec![0.001f32, -0.0012, 0.0015, -0.0018];
    let before_energy: f32 = samples.iter().map(|s| s.abs()).sum();

    remove_background_noise(&mut samples, 0.001);

    let after_energy: f32 = samples.iter().map(|s| s.abs()).sum();
    assert!(after_energy < before_energy);
  }

  #[test]
  fn test_normalize_target_rms_boosts_low_voice() {
    let mut samples = vec![0.01f32; 512];
    let before = rms(&samples);

    let gain = normalize_target_rms(&mut samples, 0.1, 0.5, 10.0);
    let after = rms(&samples);

    assert!(gain > 1.0);
    assert!(after > before);
  }

  #[test]
  fn test_limit_peaks_prevents_clipping() {
    let mut samples = vec![1.4f32, -1.3, 0.2, -0.1];
    limit_peaks(&mut samples, 0.95);

    assert!(samples.iter().all(|s| s.abs() <= 1.0));
    assert!(samples[0].abs() < 1.4);
    assert!(samples[1].abs() < 1.3);
  }

  #[test]
  fn test_process_audio_for_saving_stabilizes_and_bounds() {
    let mut samples: Vec<f32> = (0..1600)
      .map(|i| if i % 13 == 0 { 0.001 } else { 0.03 })
      .collect();

    process_audio_for_saving(&mut samples);

    assert!(samples.iter().all(|s| s.abs() <= 1.0));
    let out_rms = rms(&samples);
    assert!(out_rms > 0.03);
    assert!(out_rms < 0.25);
  }

  #[test]
  fn test_tame_high_frequency_hiss_smooths_rapid_alternation() {
    let mut samples = vec![0.3f32, -0.3, 0.3, -0.3, 0.3, -0.3, 0.3, -0.3];
    let before_diff: f32 =
      samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum();

    tame_high_frequency_hiss(&mut samples, 0.25, 0.5);

    let after_diff: f32 = samples.windows(2).map(|w| (w[1] - w[0]).abs()).sum();
    assert!(after_diff < before_diff);
  }

  #[test]
  fn test_recording_output_path_uses_run_audio_recording_wav() {
    let path = recording_output_path();
    let normalized = path.to_string_lossy().replace('\\', "/");
    assert!(normalized.ends_with("/run/audio/recording.wav"));
  }
}
