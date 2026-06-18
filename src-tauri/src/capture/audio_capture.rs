// Task 9: Audio capture via cpal 0.15, writing to WAV using hound 3.
//
// cpal 0.15 API notes:
// - build_input_stream<T, D, E>(config, data_cb, err_cb, timeout: Option<Duration>)
// - Samples captured as f32 and written as 32-bit float WAV.
// - device_id is matched against device.name() for human-readable selection.

use std::path::Path;
use std::sync::{Arc, Mutex};

use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};

pub struct AudioCapture {
    stream: cpal::Stream,
    writer: Arc<Mutex<Option<hound::WavWriter<std::io::BufWriter<std::fs::File>>>>>,
}

/// List available microphone input devices as (id, display_name) pairs.
/// The `id` is the device name, used as `device_id` in `AudioCapture::start`.
pub fn list_microphones() -> Vec<(String, String)> {
    let host = cpal::default_host();
    host.input_devices()
        .map(|devs| {
            devs.filter_map(|d| {
                d.name().ok().map(|n| (n.clone(), n))
            })
            .collect()
        })
        .unwrap_or_default()
}

impl AudioCapture {
    /// Start recording audio from `device_id` (or the default input device if `None`)
    /// and write a 32-bit float WAV to `audio_tmp`.
    pub fn start(device_id: Option<String>, audio_tmp: &Path) -> Result<Self, String> {
        let host = cpal::default_host();

        let device = match device_id {
            Some(ref name) => host
                .input_devices()
                .map_err(|e| e.to_string())?
                .find(|d| d.name().map(|n| &n == name).unwrap_or(false))
                .ok_or_else(|| format!("microphone '{name}' not found"))?,
            None => host
                .default_input_device()
                .ok_or_else(|| "no default input device".to_string())?,
        };

        let config = device
            .default_input_config()
            .map_err(|e| e.to_string())?;

        let spec = hound::WavSpec {
            channels: config.channels(),
            sample_rate: config.sample_rate().0,
            bits_per_sample: 32,
            sample_format: hound::SampleFormat::Float,
        };

        let writer = hound::WavWriter::create(audio_tmp, spec).map_err(|e| e.to_string())?;
        let writer = Arc::new(Mutex::new(Some(writer)));
        let w2 = writer.clone();

        let err_fn = |e: cpal::StreamError| eprintln!("audio stream error: {e}");

        let stream = device
            .build_input_stream(
                &config.into(),
                move |data: &[f32], _: &cpal::InputCallbackInfo| {
                    if let Ok(mut guard) = w2.lock() {
                        if let Some(ref mut w) = *guard {
                            for &sample in data {
                                let _ = w.write_sample(sample);
                            }
                        }
                    }
                },
                err_fn,
                None, // timeout: Option<Duration>
            )
            .map_err(|e| e.to_string())?;

        stream.play().map_err(|e| e.to_string())?;

        Ok(Self { stream, writer })
    }

    /// Stop the audio stream and finalize (flush + close) the WAV file.
    pub fn stop(self) -> Result<(), String> {
        // Dropping the stream stops capture.
        drop(self.stream);

        if let Ok(mut guard) = self.writer.lock() {
            if let Some(w) = guard.take() {
                w.finalize().map_err(|e| e.to_string())?;
            }
        }
        Ok(())
    }
}
