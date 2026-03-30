use std::fs::File;
use std::sync::{
  Arc,
  atomic::{AtomicBool, AtomicU32, Ordering},
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

pub fn run_volume_monitor(volume_level: Arc<AtomicU32>, running: Arc<AtomicBool>) {
  let host = cpal::default_host();
  let device = host.default_input_device().expect("No mic found");
  let config = device.default_input_config().unwrap();
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

  let file = File::create(&temp_path).expect("Failed to create temp file");
  let writer = WavWriter::new(file, spec).expect("Failed to create WavWriter");

  let stream_config: cpal::StreamConfig = config.into();

  let writer_arc = Arc::new(std::sync::Mutex::new(writer));
  let writer_cb = writer_arc.clone();

  // Resampling state
  let sample_drop_ratio = source_sample_rate / target_sample_rate as f64;
  let mut accumulator: f64 = 0.0;

  let stream = device
    .build_input_stream(
      &stream_config,
      move |data: &[f32], _| {
        // Update volume monitor
        volume_level.store(calculate_rms_volume(data), Ordering::Relaxed);

        // Write to WAV with resampling
        if let Ok(mut writer) = writer_cb.lock() {
          let num_frames = data.len() / channels;

          for frame_idx in 0..num_frames {
            let mono_sample = mix_to_mono(data, channels, frame_idx);

            if let Some(sample) = resample_sample(mono_sample, &mut accumulator, sample_drop_ratio)
            {
              let _ = writer.write_sample(sample);
            }
          }
        }
      },
      |_| {},
      None,
    )
    .unwrap();

  stream.play().unwrap();

  // Wait for running to be set to false
  while running.load(Ordering::SeqCst) {
    std::thread::sleep(std::time::Duration::from_millis(100));
  }

  // Drop the stream to release the closure and writer_cb
  drop(stream);

  // Finalize the writer
  if let Ok(writer_mutex) = Arc::try_unwrap(writer_arc) {
    let writer = writer_mutex.into_inner().unwrap();
    writer.finalize().expect("Failed to finalize WAV");
  }

  // Copy the temp file to the final location
  if std::path::Path::new(&temp_path).exists() {
    let final_path = std::env::current_dir().unwrap().join("recording.wav");
    let _ = std::fs::copy(&temp_path, &final_path);
    // Clean up temp file
    let _ = std::fs::remove_file(&temp_path);
  }
}
