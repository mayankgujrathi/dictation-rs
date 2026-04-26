use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU32},
};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use directories::ProjectDirs;
use hound::{WavReader, WavSpec, WavWriter};
use tracing::{debug, error, info, warn};

use crate::app;

const TARGET_RMS: f32 = 0.12;
const MIN_GAIN: f32 = 0.8;
const MAX_GAIN: f32 = 6.0;
const PEAK_LIMIT: f32 = 0.95;
const HISS_SMOOTHING_ALPHA: f32 = 0.25;

/// Calculate RMS (Root Mean Square) from audio samples for volume monitoring
pub fn calculate_rms_volume(samples: &[f32]) -> u32 {
  if samples.is_empty() {
    return 0;
  }

  let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
  let rms = (sum_sq / samples.len() as f32).sqrt();
  (rms * 1000.0) as u32
}

fn calculate_rms(samples: &[f32]) -> f32 {
  if samples.is_empty() {
    return 0.0;
  }

  let sum_sq: f32 = samples.iter().map(|&s| s * s).sum();
  (sum_sq / samples.len() as f32).sqrt()
}

fn model_base_dir() -> std::path::PathBuf {
  ProjectDirs::from("com", "vocoflow", "vocoflow")
    .map(|dirs| dirs.data_dir().to_path_buf())
    .unwrap_or_else(|| {
      std::env::current_dir().unwrap_or_else(|_| std::env::temp_dir())
    })
}

pub fn recording_output_path() -> std::path::PathBuf {
  model_base_dir()
    .join("run")
    .join("audio")
    .join("recording.wav")
}

fn sample_to_i16(sample: f32) -> i16 {
  (sample.clamp(-1.0, 1.0) * i16::MAX as f32) as i16
}

/// Estimate noise floor using lower-energy (20th percentile) absolute sample magnitudes.
pub fn estimate_noise_floor(samples: &[f32]) -> f32 {
  if samples.is_empty() {
    return 0.0;
  }

  let mut magnitudes: Vec<f32> = samples.iter().map(|s| s.abs()).collect();
  magnitudes.sort_by(f32::total_cmp);
  let idx = ((magnitudes.len() as f32) * 0.2) as usize;
  magnitudes[idx.min(magnitudes.len() - 1)]
}

/// Reduce low-level background noise with soft attenuation below threshold.
pub fn remove_background_noise(samples: &mut [f32], noise_floor: f32) {
  if samples.is_empty() {
    return;
  }

  let threshold = (noise_floor * 2.5).max(0.003);
  let min_attenuation = 0.2;

  for sample in samples.iter_mut() {
    let amp = sample.abs();
    if amp < threshold {
      let ratio = if threshold > 0.0 {
        amp / threshold
      } else {
        0.0
      };
      let soft_scale = min_attenuation + (1.0 - min_attenuation) * ratio;
      *sample *= soft_scale;
    }
  }
}

/// Tame sharp high-frequency background hiss using a gentle low-pass blend.
/// This preserves most voice energy while smoothing harsh high-frequency noise.
pub fn tame_high_frequency_hiss(samples: &mut [f32], alpha: f32, blend: f32) {
  if samples.is_empty() {
    return;
  }

  let a = alpha.clamp(0.01, 0.99);
  let mix = blend.clamp(0.0, 1.0);
  let mut smooth = samples[0];

  for sample in samples.iter_mut() {
    smooth = a * *sample + (1.0 - a) * smooth;
    // Mix toward smoothed value; reduces harshness while retaining speech shape.
    *sample = *sample * (1.0 - mix) + smooth * mix;
  }
}

/// Normalize towards a target RMS with bounded gain.
pub fn normalize_target_rms(
  samples: &mut [f32],
  target_rms: f32,
  min_gain: f32,
  max_gain: f32,
) -> f32 {
  let rms = calculate_rms(samples);
  if rms <= f32::EPSILON {
    return 1.0;
  }

  let gain = (target_rms / rms).clamp(min_gain, max_gain);
  for sample in samples.iter_mut() {
    *sample *= gain;
  }
  gain
}

/// Soft peak limiting to avoid clipping artifacts.
pub fn limit_peaks(samples: &mut [f32], threshold: f32) {
  if threshold <= 0.0 {
    return;
  }

  for sample in samples.iter_mut() {
    let amp = sample.abs();
    if amp > threshold {
      let sign = sample.signum();
      let excess = amp - threshold;
      let compressed =
        threshold + (excess / (1.0 + excess / (1.0 - threshold)));
      *sample = sign * compressed.min(1.0);
    }
    *sample = sample.clamp(-1.0, 1.0);
  }
}

/// End-to-end post-processing pipeline to stabilize voice level and suppress noise.
pub fn process_audio_for_saving(samples: &mut [f32]) {
  if samples.is_empty() {
    return;
  }

  let noise_floor = estimate_noise_floor(samples);
  remove_background_noise(samples, noise_floor);
  // Extra pass to reduce sharp hiss-like background noise.
  tame_high_frequency_hiss(samples, HISS_SMOOTHING_ALPHA, 0.35);
  normalize_target_rms(samples, TARGET_RMS, MIN_GAIN, MAX_GAIN);
  limit_peaks(samples, PEAK_LIMIT);
}

fn post_process_and_save(temp_path: &std::path::Path) -> Result<(), String> {
  debug!(temp_path = %temp_path.display(), "starting audio post-processing");
  let mut reader = WavReader::open(temp_path)
    .map_err(|e| format!("open temp wav failed: {e}"))?;
  let input_spec = reader.spec();

  let mut samples: Vec<f32> = reader
    .samples::<i16>()
    .filter_map(Result::ok)
    .map(|s| s as f32 / i16::MAX as f32)
    .collect();

  process_audio_for_saving(&mut samples);

  let output_path = recording_output_path();
  if let Some(parent) = output_path.parent() {
    std::fs::create_dir_all(parent)
      .map_err(|e| format!("create output dir failed: {e}"))?;
  }

  let out_spec = WavSpec {
    channels: 1,
    sample_rate: input_spec.sample_rate,
    bits_per_sample: 16,
    sample_format: hound::SampleFormat::Int,
  };

  let mut writer = WavWriter::create(&output_path, out_spec)
    .map_err(|e| format!("create output wav failed: {e}"))?;
  for sample in samples {
    writer
      .write_sample(sample_to_i16(sample))
      .map_err(|e| format!("write output sample failed: {e}"))?;
  }
  writer
    .finalize()
    .map_err(|e| format!("finalize output wav failed: {e}"))?;

  info!(output_path = %output_path.display(), "recording saved after post-processing");

  Ok(())
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
fn resample_sample(
  mono_sample: f32,
  accumulator: &mut f64,
  sample_drop_ratio: f64,
) -> Option<i16> {
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
  /// Flag to indicate current recording cycle output file is finalized and ready for transcription
  pub recording_ready: Arc<AtomicBool>,
}

impl RecordingState {
  pub fn new() -> Self {
    Self {
      volume_level: Arc::new(AtomicU32::new(0)),
      is_recording: Arc::new(AtomicBool::new(false)),
      mic_ready: Arc::new(AtomicBool::new(false)),
      recording_ready: Arc::new(AtomicBool::new(true)),
    }
  }

  pub fn set_recording(&self, recording: bool) {
    debug!(recording, "recording state set");
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
    info!("recording requested");
    // User intent: recording requested
    self
      .is_recording
      .store(true, std::sync::atomic::Ordering::SeqCst);
    // Runtime readiness: mic stream not ready until initialization succeeds
    self
      .mic_ready
      .store(false, std::sync::atomic::Ordering::SeqCst);
    self
      .recording_ready
      .store(false, std::sync::atomic::Ordering::SeqCst);

    let state = self.clone();

    // Run recording in background thread
    std::thread::spawn(move || {
      debug!("recording worker started");
      let host = cpal::default_host();
      let device = match host.default_input_device() {
        Some(d) => d,
        None => {
          error!("no default input device found");
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          state
            .mic_ready
            .store(false, std::sync::atomic::Ordering::SeqCst);
          app::wake_ui();
          return;
        }
      };

      let config = match device.default_input_config() {
        Ok(c) => c,
        Err(e) => {
          error!(error = %e, "failed to get input config");
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          state
            .mic_ready
            .store(false, std::sync::atomic::Ordering::SeqCst);
          app::wake_ui();
          return;
        }
      };

      let source_sample_rate = config.sample_rate().0 as f64;
      let channels = config.channels() as usize;
      info!(source_sample_rate, channels, "input device initialized");

      // Target: 16kHz, Mono, 16-bit PCM for Whisper compatibility
      let target_sample_rate = 16000u32;
      let spec = WavSpec {
        channels: 1,
        sample_rate: target_sample_rate,
        bits_per_sample: 16,
        sample_format: hound::SampleFormat::Int,
      };

      // Create temp file path
      let temp_path = std::env::temp_dir().join("vocoflow_temp.wav");

      let writer = match WavWriter::create(&temp_path, spec) {
        Ok(w) => w,
        Err(e) => {
          error!(error = %e, temp_path = %temp_path.display(), "failed to create wav writer");
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          app::wake_ui();
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
          if let Ok(mut writer_opt) = writer_cb.lock()
            && let Some(writer) = writer_opt.as_mut()
            && let Ok(mut acc) = acc_cb.lock()
          {
            let num_frames = data.len() / channels;

            for frame_idx in 0..num_frames {
              let mono_sample = mix_to_mono(data, channels, frame_idx);

              if let Some(sample) =
                resample_sample(mono_sample, &mut acc, sample_drop_ratio)
              {
                let _ = writer.write_sample(sample);
              }
            }
          }
        },
        |err| {
          error!(error = %err, "input stream error");
        },
        None,
      ) {
        Ok(s) => s,
        Err(e) => {
          error!(error = %e, "failed to build input stream");
          state
            .is_recording
            .store(false, std::sync::atomic::Ordering::SeqCst);
          state
            .mic_ready
            .store(false, std::sync::atomic::Ordering::SeqCst);
          app::wake_ui();
          return;
        }
      };

      if let Err(e) = stream.play() {
        error!(error = %e, "failed to start input stream playback");
        state
          .is_recording
          .store(false, std::sync::atomic::Ordering::SeqCst);
        state
          .mic_ready
          .store(false, std::sync::atomic::Ordering::SeqCst);
        app::wake_ui();
        return;
      }

      state
        .mic_ready
        .store(true, std::sync::atomic::Ordering::SeqCst);
      app::wake_ui();
      info!("microphone stream is active");

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
      if let Ok(writer_opt) = Arc::try_unwrap(writer_arc)
        && let Some(writer) = writer_opt.into_inner().ok().flatten()
      {
        let _ = writer.finalize();
      }
      debug!("recording stream finalized");

      // Post-process and save output to run/audio/recording.wav under model-base path
      if std::path::Path::new(&temp_path).exists() {
        if let Err(e) = post_process_and_save(&temp_path) {
          warn!(error = %e, "audio post-processing failed");
        }
        // Clean up temp file
        let _ = std::fs::remove_file(&temp_path);
      }

      // Signal that this recording cycle is fully finalized (success or failure).
      state
        .recording_ready
        .store(true, std::sync::atomic::Ordering::SeqCst);
      app::wake_ui();
      info!("recording cycle finalized and ready for transcription");
    });
  }
}

impl Default for RecordingState {
  fn default() -> Self {
    Self::new()
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
