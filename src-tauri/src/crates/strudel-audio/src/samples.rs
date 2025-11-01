//! Sample loading and management
//!
//! Handles loading audio samples from bundled assets or HTTP URLs,
//! with caching to avoid redundant decoding.

use crate::{AudioError, Result};
use parking_lot::RwLock;
use std::collections::HashMap;
use std::io::Cursor;
use std::sync::Arc;
use symphonia::core::audio::AudioBufferRef;
use symphonia::core::codecs::DecoderOptions;
use symphonia::core::formats::FormatOptions;
use symphonia::core::io::MediaSourceStream;
use symphonia::core::meta::MetadataOptions;
use symphonia::core::probe::Hint;

/// An audio sample with decoded PCM data
#[derive(Debug, Clone)]
pub struct Sample {
    /// Sample name (e.g., "bd", "sd", "hh")
    pub name: String,
    /// Sample index within the bank (e.g., "bd:0", "bd:1")
    pub index: usize,
    /// Audio data (interleaved stereo f32, normalized to [-1.0, 1.0])
    pub data: Arc<Vec<f32>>,
    /// Sample rate in Hz
    pub sample_rate: u32,
    /// Number of channels (1 = mono, 2 = stereo)
    pub channels: u16,
}

impl Sample {
    /// Get the duration of this sample in seconds
    pub fn duration(&self) -> f64 {
        self.data.len() as f64 / (self.sample_rate as f64 * self.channels as f64)
    }

    /// Get the number of frames (samples per channel)
    pub fn frames(&self) -> usize {
        self.data.len() / self.channels as usize
    }
}

/// A collection of related samples (e.g., all "bd" samples)
#[derive(Debug, Clone)]
pub struct SampleBank {
    /// Bank name (e.g., "bd", "sd")
    pub name: String,
    /// All samples in this bank
    pub samples: Vec<Sample>,
}

impl SampleBank {
    /// Create a new empty sample bank
    pub fn new(name: String) -> Self {
        SampleBank {
            name,
            samples: Vec::new(),
        }
    }

    /// Add a sample to this bank
    pub fn add_sample(&mut self, sample: Sample) {
        self.samples.push(sample);
    }

    /// Get a sample by index, wrapping if out of bounds
    pub fn get(&self, index: usize) -> Option<&Sample> {
        if self.samples.is_empty() {
            None
        } else {
            Some(&self.samples[index % self.samples.len()])
        }
    }

    /// Get the number of samples in this bank
    pub fn len(&self) -> usize {
        self.samples.len()
    }

    /// Check if this bank is empty
    pub fn is_empty(&self) -> bool {
        self.samples.is_empty()
    }
}

/// Loads and caches audio samples
pub struct SampleLoader {
    /// Cached sample banks
    banks: Arc<RwLock<HashMap<String, SampleBank>>>,
    /// Base URL for HTTP fallback
    fallback_base_url: String,
}

impl SampleLoader {
    /// Create a new sample loader
    pub fn new() -> Self {
        SampleLoader {
            banks: Arc::new(RwLock::new(HashMap::new())),
            fallback_base_url: "https://raw.githubusercontent.com/tidalcycles/Dirt-Samples/master"
                .to_string(),
        }
    }

    /// Set the fallback base URL for HTTP loading
    pub fn with_fallback_url(mut self, url: String) -> Self {
        self.fallback_base_url = url;
        self
    }

    /// Load a sample bank by name
    ///
    /// First tries to load from bundled assets, then falls back to HTTP
    pub fn load_bank(&self, bank_name: &str) -> Result<()> {
        // Check if already loaded
        {
            let banks = self.banks.read();
            if banks.contains_key(bank_name) {
                return Ok(());
            }
        }

        // Try bundled samples first
        if let Some(bank) = self.try_load_bundled(bank_name)? {
            let mut banks = self.banks.write();
            banks.insert(bank_name.to_string(), bank);
            return Ok(());
        }

        // Fall back to HTTP
        if let Some(bank) = self.try_load_http(bank_name)? {
            let mut banks = self.banks.write();
            banks.insert(bank_name.to_string(), bank);
            return Ok(());
        }

        Err(AudioError::SampleNotFound(bank_name.to_string()))
    }

    /// Get a sample from a loaded bank
    pub fn get_sample(&self, bank_name: &str, index: usize) -> Result<Sample> {
        let banks = self.banks.read();
        let bank = banks
            .get(bank_name)
            .ok_or_else(|| AudioError::SampleNotFound(bank_name.to_string()))?;

        bank.get(index)
            .cloned()
            .ok_or_else(|| AudioError::SampleNotFound(format!("{}:{}", bank_name, index)))
    }

    /// Try to load a sample bank from bundled assets
    fn try_load_bundled(&self, bank_name: &str) -> Result<Option<SampleBank>> {
        let mut bank = SampleBank::new(bank_name.to_string());

        // Load bundled samples using include_bytes!()
        match bank_name {
            "bd" => {
                // Bass drums
                let samples_data = [
                    ("bd/BT0A0A7.wav", include_bytes!("../assets/samples/bd/BT0A0A7.wav").as_slice()),
                    ("bd/BT0A0D0.wav", include_bytes!("../assets/samples/bd/BT0A0D0.wav").as_slice()),
                    ("bd/BT0A0D3.wav", include_bytes!("../assets/samples/bd/BT0A0D3.wav").as_slice()),
                    ("bd/BT0A0DA.wav", include_bytes!("../assets/samples/bd/BT0A0DA.wav").as_slice()),
                    ("bd/BT0AAD0.wav", include_bytes!("../assets/samples/bd/BT0AAD0.wav").as_slice()),
                ];
                for (i, (name, data)) in samples_data.iter().enumerate() {
                    let sample = self.decode_audio(data, name, i)?;
                    bank.add_sample(sample);
                }
            }
            "sd" => {
                // Snare drums
                let samples_data = [
                    ("sd/rytm-00-hard.wav", include_bytes!("../assets/samples/sd/rytm-00-hard.wav").as_slice()),
                    ("sd/rytm-01-classic.wav", include_bytes!("../assets/samples/sd/rytm-01-classic.wav").as_slice()),
                ];
                for (i, (name, data)) in samples_data.iter().enumerate() {
                    let sample = self.decode_audio(data, name, i)?;
                    bank.add_sample(sample);
                }
            }
            "hh" => {
                // Hi-hats
                let samples_data = [
                    ("hh/000_hh3closedhh.wav", include_bytes!("../assets/samples/hh/000_hh3closedhh.wav").as_slice()),
                    ("hh/001_hh3crash.wav", include_bytes!("../assets/samples/hh/001_hh3crash.wav").as_slice()),
                    ("hh/002_hh3hit1.wav", include_bytes!("../assets/samples/hh/002_hh3hit1.wav").as_slice()),
                ];
                for (i, (name, data)) in samples_data.iter().enumerate() {
                    let sample = self.decode_audio(data, name, i)?;
                    bank.add_sample(sample);
                }
            }
            "cp" => {
                // Claps
                let samples_data = [
                    ("cp/HANDCLP0.wav", include_bytes!("../assets/samples/cp/HANDCLP0.wav").as_slice()),
                    ("cp/HANDCLPA.wav", include_bytes!("../assets/samples/cp/HANDCLPA.wav").as_slice()),
                ];
                for (i, (name, data)) in samples_data.iter().enumerate() {
                    let sample = self.decode_audio(data, name, i)?;
                    bank.add_sample(sample);
                }
            }
            _ => return Ok(None), // Bank not bundled
        }

        Ok(Some(bank))
    }

    /// Try to load a sample bank from HTTP
    fn try_load_http(&self, bank_name: &str) -> Result<Option<SampleBank>> {
        // Fetch strudel.json from the base URL
        let json_url = format!("{}/strudel.json", self.fallback_base_url);

        let json: serde_json::Value = reqwest::blocking::get(&json_url)
            .map_err(|e| AudioError::HttpError(format!("Failed to fetch sample map: {}", e)))?
            .json()
            .map_err(|e| AudioError::HttpError(format!("Failed to parse JSON: {}", e)))?;

        // Get the bank entry
        let bank_entry = json.get(bank_name);
        if bank_entry.is_none() {
            return Ok(None);
        }

        let bank_entry = bank_entry.unwrap();

        // Get sample paths (could be array or object)
        let sample_paths: Vec<String> = if let Some(arr) = bank_entry.as_array() {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect()
        } else {
            return Ok(None);
        };

        if sample_paths.is_empty() {
            return Ok(None);
        }

        // Download and decode each sample
        let mut bank = SampleBank::new(bank_name.to_string());

        for (i, path) in sample_paths.iter().enumerate() {
            let sample_url = format!("{}/{}", self.fallback_base_url, path);

            // Download the sample
            let bytes = reqwest::blocking::get(&sample_url)
                .map_err(|e| AudioError::HttpError(format!("Failed to download {}: {}", path, e)))?
                .bytes()
                .map_err(|e| AudioError::HttpError(format!("Failed to read bytes: {}", e)))?;

            // Decode the sample
            match self.decode_audio(&bytes, path, i) {
                Ok(sample) => bank.add_sample(sample),
                Err(e) => {
                    eprintln!("Warning: Failed to decode {}: {}", path, e);
                    continue;
                }
            }
        }

        if bank.is_empty() {
            Ok(None)
        } else {
            Ok(Some(bank))
        }
    }

    /// Decode audio data from bytes using Symphonia
    pub fn decode_audio(&self, data: &[u8], name: &str, index: usize) -> Result<Sample> {
        // Create a media source from the byte slice (need to own the data)
        let owned_data = data.to_vec();
        let cursor = Cursor::new(owned_data);
        let mss = MediaSourceStream::new(Box::new(cursor), Default::default());

        // Create a hint to help the format registry guess the format
        let mut hint = Hint::new();
        if name.ends_with(".wav") {
            hint.with_extension("wav");
        } else if name.ends_with(".mp3") {
            hint.with_extension("mp3");
        } else if name.ends_with(".ogg") {
            hint.with_extension("ogg");
        }

        // Probe the media source
        let probed = symphonia::default::get_probe()
            .format(&hint, mss, &FormatOptions::default(), &MetadataOptions::default())
            .map_err(|e| AudioError::DecodeError(format!("Failed to probe format: {}", e)))?;

        let mut format = probed.format;
        let track = format
            .tracks()
            .iter()
            .find(|t| t.codec_params.codec != symphonia::core::codecs::CODEC_TYPE_NULL)
            .ok_or_else(|| AudioError::DecodeError("No valid audio track found".to_string()))?;

        let track_id = track.id;
        let codec_params = &track.codec_params;

        // Create decoder
        let mut decoder = symphonia::default::get_codecs()
            .make(codec_params, &DecoderOptions::default())
            .map_err(|e| AudioError::DecodeError(format!("Failed to create decoder: {}", e)))?;

        // Get sample rate and channels
        let sample_rate = codec_params.sample_rate.unwrap_or(44100);
        let channels = codec_params.channels.map(|c| c.count()).unwrap_or(2) as u16;

        // Decode all audio data
        let mut audio_data: Vec<f32> = Vec::new();

        loop {
            match format.next_packet() {
                Ok(packet) if packet.track_id() == track_id => {
                    match decoder.decode(&packet) {
                        Ok(decoded) => {
                            // Convert to f32 and append
                            self.convert_audio_buffer(&decoded, &mut audio_data)?;
                        }
                        Err(e) => {
                            return Err(AudioError::DecodeError(format!(
                                "Failed to decode packet: {}",
                                e
                            )));
                        }
                    }
                }
                Ok(_) => continue,
                Err(symphonia::core::errors::Error::IoError(e))
                    if e.kind() == std::io::ErrorKind::UnexpectedEof =>
                {
                    break;
                }
                Err(e) => {
                    return Err(AudioError::DecodeError(format!("Format error: {}", e)));
                }
            }
        }

        Ok(Sample {
            name: name.to_string(),
            index,
            data: Arc::new(audio_data),
            sample_rate,
            channels,
        })
    }

    /// Convert Symphonia audio buffer to f32 samples
    fn convert_audio_buffer(&self, buffer: &AudioBufferRef, output: &mut Vec<f32>) -> Result<()> {
        match buffer {
            AudioBufferRef::F32(buf) => {
                // Already f32, just copy
                for plane in buf.planes().planes() {
                    output.extend_from_slice(plane);
                }
            }
            AudioBufferRef::U8(buf) => {
                // Convert u8 to f32: [0, 255] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    output.extend(plane.iter().map(|&s| (s as f32 / 127.5) - 1.0));
                }
            }
            AudioBufferRef::U16(buf) => {
                // Convert u16 to f32: [0, 65535] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    output.extend(plane.iter().map(|&s| (s as f32 / 32767.5) - 1.0));
                }
            }
            AudioBufferRef::U24(buf) => {
                // Convert u24 to f32: [0, 16777215] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    #[allow(deprecated)]
                    output.extend(plane.iter().map(|&s| (s.into_u32() as f32 / 8388607.5) - 1.0));
                }
            }
            AudioBufferRef::U32(buf) => {
                // Convert u32 to f32: [0, 4294967295] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    output.extend(plane.iter().map(|&s| (s as f32 / 2147483647.5) - 1.0));
                }
            }
            AudioBufferRef::S8(buf) => {
                // Convert s8 to f32: [-128, 127] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    output.extend(plane.iter().map(|&s| s as f32 / 128.0));
                }
            }
            AudioBufferRef::S16(buf) => {
                // Convert s16 to f32: [-32768, 32767] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    output.extend(plane.iter().map(|&s| s as f32 / 32768.0));
                }
            }
            AudioBufferRef::S24(buf) => {
                // Convert s24 to f32: [-8388608, 8388607] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    #[allow(deprecated)]
                    output.extend(plane.iter().map(|&s| s.into_i32() as f32 / 8388608.0));
                }
            }
            AudioBufferRef::S32(buf) => {
                // Convert s32 to f32: [-2147483648, 2147483647] -> [-1.0, 1.0]
                for plane in buf.planes().planes() {
                    output.extend(plane.iter().map(|&s| s as f32 / 2147483648.0));
                }
            }
            AudioBufferRef::F64(buf) => {
                // Convert f64 to f32
                for plane in buf.planes().planes() {
                    output.extend(plane.iter().map(|&s| s as f32));
                }
            }
        }
        Ok(())
    }
}

impl Default for SampleLoader {
    fn default() -> Self {
        Self::new()
    }
}
