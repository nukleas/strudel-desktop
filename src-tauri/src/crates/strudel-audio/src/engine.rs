//! Audio output engine using cpal
//!
//! Manages the audio device and output stream

use crate::{AudioError, Result};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use cpal::{Device, Stream, StreamConfig};
use parking_lot::Mutex;
use std::sync::Arc;

/// Audio output engine
pub struct AudioEngine {
    /// Audio output device
    device: Device,
    /// Stream configuration
    config: StreamConfig,
    /// Output stream (when active)
    stream: Arc<Mutex<Option<Stream>>>,
    /// Sample rate
    sample_rate: u32,
}

impl AudioEngine {
    /// Create a new audio engine with the default output device
    pub fn new() -> Result<Self> {
        let host = cpal::default_host();

        let device = host
            .default_output_device()
            .ok_or_else(|| AudioError::DeviceError("No output device available".to_string()))?;

        let config = device
            .default_output_config()
            .map_err(|e| AudioError::DeviceError(format!("Failed to get default config: {}", e)))?;

        let sample_rate = config.sample_rate().0;
        let config = config.into();

        Ok(AudioEngine {
            device,
            config,
            #[allow(clippy::arc_with_non_send_sync)]
            stream: Arc::new(Mutex::new(None)),
            sample_rate,
        })
    }

    /// Get the sample rate of the output device
    pub fn sample_rate(&self) -> u32 {
        self.sample_rate
    }

    /// Get the stream configuration
    pub fn config(&self) -> &StreamConfig {
        &self.config
    }

    /// Start the audio stream with a callback that fills the output buffer
    ///
    /// The callback receives: (output_buffer: &mut [f32], info: OutputCallbackInfo)
    /// The output buffer is interleaved stereo (L, R, L, R, ...)
    pub fn start<F>(&self, mut callback: F) -> Result<()>
    where
        F: FnMut(&mut [f32]) + Send + 'static,
    {
        let stream = self
            .device
            .build_output_stream(
                &self.config,
                move |data: &mut [f32], _: &cpal::OutputCallbackInfo| {
                    callback(data);
                },
                |err| {
                    eprintln!("Audio stream error: {}", err);
                },
                None,
            )
            .map_err(|e| AudioError::DeviceError(format!("Failed to build stream: {}", e)))?;

        stream
            .play()
            .map_err(|e| AudioError::DeviceError(format!("Failed to play stream: {}", e)))?;

        *self.stream.lock() = Some(stream);

        Ok(())
    }

    /// Stop the audio stream
    pub fn stop(&self) -> Result<()> {
        let mut stream = self.stream.lock();
        if let Some(s) = stream.take() {
            s.pause()
                .map_err(|e| AudioError::DeviceError(format!("Failed to stop stream: {}", e)))?;
        }
        Ok(())
    }

    /// Check if the audio stream is running
    pub fn is_running(&self) -> bool {
        self.stream.lock().is_some()
    }
}

impl Default for AudioEngine {
    fn default() -> Self {
        Self::new().expect("Failed to create default audio engine")
    }
}
