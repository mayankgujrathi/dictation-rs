use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU32},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use hound::{WavSpec, WavWriter};

/// Calculate RMS (Root Mean Square) from audio samples for volume monitoring
pub fn calculate_rms_volume(samples: &[f32]) -> u32 {
  let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
  let rms = (sum_sq / samples.len() as f32).sqrt();
  (rms * 1000.0) as u32
}

/// Mix multiple channels into a single mono sample by averaging
fn mix_to_mono(samples: &[f32], channels: usize, frame_idx: usize) -> f32 {
  let mut mono_sample: f32 = 0.0;
  for ch in 0..channels {
    mono_sample += samples[frame_idx * channels + ch];
  }
  mono_sample / channels as f32
}

/// Resample a mono sample using accumulator-based downsampling
/// Returns the sample if it should be written (based on drop ratio)
fn resample_sample(mono_sample: f32, accumulator: &mut f64, sample_drop_ratio: f64) -> Option<i16> {
  *accumulator += 1.0;

  if *accumulator >= sample_drop_ratio {
    *accumulator -= sample_drop_ratio;
    Some((mono_sample * i16::MAX as f32) as i16)
  } else {
    None
  }
}

/// Shared state for recording that can be accessed from multiple threads
#[derive(Clone)]
pub struct RecordingState {
  /// Current volume level for visualization
  pub volume_level: Arc<AtomicU32>,
  /// Flag to signal recording is active
  pub is_recording: Arc<AtomicBool>,
  /// Flag to indicate microphone stream is successfully initialized and running
  pub mic_ready: Arc<AtomicBool>,
}

impl RecordingState {
  pub fn new() -> Self {
    Self {
      volume_level: Arc::new(AtomicU32::new(0)),
      is_recording: Arc::new(AtomicBool::new(false)),
      mic_ready: Arc::new(AtomicBool::new(false)),
    }
  }

  pub fn set_recording(&self, recording: bool) {
    self
      .is_recording
      .store(recording, std::sync::atomic::Ordering::SeqCst);
  }

  #[allow(dead_code)]
  pub fn is_recording(&self) -> bool {
    self.is_recording.load(std::sync::atomic::Ordering::SeqCst)
  }

  /// Start a new recording session
  pub fn record(&self) {
    // User intent: recording requested
    self
      .is_recording
      .store(true, std::sync::atomic::Ordering::SeqCst);
    // Runtime readiness: mic stream not ready until initialization succeeds
    self
      .mic_ready
      .store(false, std::sync::atomic::Ordering::SeqCst);

    let state = self.clone();

    // Run recording in background thread
    std::thread::spawn(move || {
      let host = cpal::default_host();
      let device = match host.default_input_device() {
        Some(d) => d,
        None => {
          eprintln!("No default input device found");
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          state
            .mic_ready
            .store(false, std::sync::atomic::Ordering::SeqCst);
          return;
        }
      };

      let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
          eprintln!("Failed to get input config: {}", e);
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          state
            .mic_ready
            .store(false, std::sync::atomic::Ordering::SeqCst);
          return;
        }
      };

      let source_sample_rate = config.sample_rate().0 as f64;
      let channels = config.channels() as usize;

      // Target: 16kHz, Mono, 16-bit PCM for Whisper compatibility
      let target_sample_rate = 16000u32;
      let spec = WavSpec {
        channels: 1,
        sample_rate: target_sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
      };

      // Create temp file path
      let temp_path = std::env::temp_dir().join("dictation_temp.wav");

      let writer = match WavWriter::create(&temp_path, spec) {
        Ok(w) => w,
        Err(e) => {
          eprintln!("Failed to create WavWriter: {}", e);
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          return;
        }
      };

      let stream_config: cpal::StreamConfig = config.into();

      // Shared state between closure and main thread
      let writer_arc = Arc::new(std::sync::Mutex::new(Some(writer)));
      let is_active = Arc::new(std::sync::Mutex::new(true));

      // Resampling state
      let sample_drop_ratio = source_sample_rate / target_sample_rate as f64;
      let accumulator_arc = Arc::new(std::sync::Mutex::new(0.0_f64));

      let is_recording_callback = state.is_recording.clone();
      let volume_level = state.volume_level.clone();
      let writer_cb = writer_arc.clone();
      let _is_active = is_active.clone();
      let acc_cb = accumulator_arc.clone();

      let stream = match device.build_input_stream(
        &stream_config,
        move |data: &[f32], _| {
          // Update volume monitor
          volume_level.store(
            calculate_rms_volume(data),
            std::sync::atomic::Ordering::Relaxed,
          );

          // Only write to WAV if still recording
          if let Ok(mut writer_opt) = writer_cb.lock() {
            if let Some(writer) = writer_opt.as_mut() {
              if let Ok(mut acc) = acc_cb.lock() {
                let num_frames = data.len() / channels;

                for frame_idx in 0..num_frames {
                  let mono_sample = mix_to_mono(data, channels, frame_idx);

                  if let Some(sample) = resample_sample(mono_sample, &mut acc, sample_drop_ratio) {
                    let _ = writer.write_sample(sample);
                  }
                }
              }
            }
          }
        },
        |err| {
          eprintln!("Stream error: {}", err);
        },
        None,
      ) {
        Ok(s) => s,
        Err(e) => {
          eprintln!("Failed to build input stream: {}", e);
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          state
            .mic_ready
            .store(false, std::sync::atomic::Ordering::SeqCst);
          return;
        }
      };

      if let Err(e) = stream.play() {
        eprintln!("Failed to play stream: {}", e);
        state
          .is_recording
          .store(false, std::sync::atomic::Ordering::SeqCst);
        state
          .mic_ready
          .store(false, std::sync::atomic::Ordering::SeqCst);
        return;
      }

      state
        .mic_ready
        .store(true, std::sync::atomic::Ordering::SeqCst);

      // Wait for recording to be stopped
      while is_recording_callback.load(std::sync::atomic::Ordering::SeqCst) {
        std::thread::sleep(std::time::Duration::from_millis(50));
      }

      // Signal callback to stop
      if let Ok(mut active) = is_active.lock() {
        *active = false;
      }

      // Stop the stream
      drop(stream);
      state
        .mic_ready
        .store(false, std::sync::atomic::Ordering::SeqCst);

      // Finalize the writer
      if let Ok(writer_opt) = Arc::try_unwrap(writer_arc) {
        if let Some(writer) = writer_opt.into_inner().ok().flatten() {
          let _ = writer.finalize();
        }
      }

      // Copy the temp file to the final location
      if std::path::Path::new(&temp_path).exists() {
        let final_path = std::env::current_dir()
          .unwrap_or_else(|_| std::env::temp_dir())
          .join("recording.wav");
        let _ = std::fs::copy(&temp_path, &final_path);
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);
      }
    });
  }
}

/// Legacy function for backward compatibility - runs volume monitor only
#[allow(dead_code)]
pub fn run_volume_monitor(
  _volume_level: Arc<AtomicU32>,
  _running: Arc<std::sync::atomic::AtomicBool>,
) {
  // This is now a no-op since recording is controlled via RecordingState
  // Keep for API compatibility
  loop {
    std::thread::sleep(std::time::Duration::from_secs(1));
  }
}
