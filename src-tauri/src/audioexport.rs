use hound::{SampleFormat, WavSpec, WavWriter};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::process::{Command, Stdio};
use tauri::WebviewWindow;
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AudioExportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Hound error: {0}")]
    Hound(#[from] hound::Error),

    #[error("Audio encoding error: {0}")]
    Encoding(String),

    #[error("Invalid parameters: {0}")]
    InvalidParams(String),

    #[error("Pattern rendering error: {0}")]
    RenderError(String),

    #[error("Subprocess error: {0}")]
    Subprocess(String),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
}

impl From<AudioExportError> for String {
    fn from(e: AudioExportError) -> Self {
        e.to_string()
    }
}

#[derive(Deserialize, Debug, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ExportParams {
    pub output_path: String,
    pub format: String,
    pub sample_rate: u32,
    pub channels: u16,
    pub duration_cycles: f64,
    pub bit_depth: Option<u16>,
    pub mp3_bitrate: Option<u32>,
}

#[derive(Deserialize, Debug)]
#[serde(rename_all = "camelCase")]
struct RenderedAudio {
    sample_rate: u32,
    channels: u16,
    sample_count: usize,
    left_channel: Vec<f32>,
    right_channel: Vec<f32>,
}

impl ExportParams {
    pub fn validate(&self) -> Result<(), AudioExportError> {
        // Validate channels
        if self.channels != 2 {
            return Err(AudioExportError::InvalidParams(
                "Only stereo (2 channels) is supported".to_string(),
            ));
        }

        // Validate format
        if self.format != "wav" && self.format != "mp3" {
            return Err(AudioExportError::InvalidParams(format!(
                "Unsupported format: {}. Use 'wav' or 'mp3'",
                self.format
            )));
        }

        // Validate bit depth for WAV
        if self.format == "wav" {
            let bit_depth = self.bit_depth.unwrap_or(16);
            if ![16, 24, 32].contains(&bit_depth) {
                return Err(AudioExportError::InvalidParams(format!(
                    "Bit depth must be 16, 24, or 32, got {}",
                    bit_depth
                )));
            }
        }

        // Validate MP3 bitrate
        if self.format == "mp3" {
            let bitrate = self.mp3_bitrate.unwrap_or(320);
            if ![128, 192, 256, 320].contains(&bitrate) {
                return Err(AudioExportError::InvalidParams(format!(
                    "MP3 bitrate must be 128, 192, 256, or 320 kbps, got {}",
                    bitrate
                )));
            }
        }

        // Validate sample rate
        if ![44100, 48000, 96000].contains(&self.sample_rate) {
            return Err(AudioExportError::InvalidParams(format!(
                "Sample rate must be 44100, 48000, or 96000 Hz, got {}",
                self.sample_rate
            )));
        }

        // Validate duration
        if self.duration_cycles <= 0.0 {
            return Err(AudioExportError::InvalidParams(
                "Duration must be positive".to_string(),
            ));
        }

        Ok(())
    }
}

/// Encode audio buffers to WAV file
fn encode_wav(left: &[f32], right: &[f32], params: &ExportParams) -> Result<(), AudioExportError> {
    let bit_depth = params.bit_depth.unwrap_or(16);

    let spec = WavSpec {
        channels: 2,
        sample_rate: params.sample_rate,
        bits_per_sample: bit_depth,
        sample_format: if bit_depth == 32 {
            SampleFormat::Float
        } else {
            SampleFormat::Int
        },
    };

    let mut writer = WavWriter::create(&params.output_path, spec)?;

    // Interleave stereo channels and write samples
    match bit_depth {
        16 => {
            for (l, r) in left.iter().zip(right.iter()) {
                // Clamp to prevent clipping and convert to i16
                let l_sample = (l.clamp(-1.0, 1.0) * 32767.0) as i16;
                let r_sample = (r.clamp(-1.0, 1.0) * 32767.0) as i16;
                writer.write_sample(l_sample)?;
                writer.write_sample(r_sample)?;
            }
        }
        24 => {
            for (l, r) in left.iter().zip(right.iter()) {
                // Clamp to prevent clipping and convert to i32 (24-bit)
                let l_sample = (l.clamp(-1.0, 1.0) * 8388607.0) as i32;
                let r_sample = (r.clamp(-1.0, 1.0) * 8388607.0) as i32;
                writer.write_sample(l_sample)?;
                writer.write_sample(r_sample)?;
            }
        }
        32 => {
            for (l, r) in left.iter().zip(right.iter()) {
                writer.write_sample(*l)?;
                writer.write_sample(*r)?;
            }
        }
        _ => unreachable!(), // Validation ensures only 16, 24, or 32
    }

    writer.finalize()?;

    Ok(())
}

/// Render a Strudel pattern to audio buffers using the Node.js Dough engine
async fn render_pattern(
    pattern_code: &str,
    sample_rate: u32,
    duration_cycles: f64,
) -> Result<(Vec<f32>, Vec<f32>), AudioExportError> {
    // Find the render-pattern.mjs script path
    // Try multiple locations in order:
    // 1. Relative to current working directory (for development)
    // 2. Relative to project root (go up from src-tauri/)

    let possible_paths = vec![
        PathBuf::from("packages/supradough/render-pattern.mjs"),
        PathBuf::from("../packages/supradough/render-pattern.mjs"),
        PathBuf::from("../../packages/supradough/render-pattern.mjs"),
    ];

    let script_path = possible_paths
        .iter()
        .find(|p| p.exists())
        .ok_or_else(|| {
            AudioExportError::Subprocess(format!(
                "Render script not found. Tried: {:?}. Current dir: {:?}",
                possible_paths,
                std::env::current_dir().ok()
            ))
        })?;

    // Calculate cps (cycles per second) - default to 0.5 (120 BPM at 4/4)
    // In the future, this should come from the pattern's tempo
    let cps = 0.5;

    // Execute Node.js script
    let output = Command::new("node")
        .arg(script_path)
        .arg(pattern_code)
        .arg(sample_rate.to_string())
        .arg(duration_cycles.to_string())
        .arg(cps.to_string())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .map_err(|e| AudioExportError::Subprocess(format!("Failed to execute node: {}", e)))?;

    // Check if the process succeeded
    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(AudioExportError::Subprocess(
            format!("Rendering failed: {}", stderr)
        ));
    }

    // Parse JSON output
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Debug: log first 200 chars of stdout
    eprintln!("DEBUG stdout (first 200 chars): {:?}", &stdout.chars().take(200).collect::<String>());
    eprintln!("DEBUG stdout length: {}", stdout.len());

    let rendered: RenderedAudio = serde_json::from_str(&stdout)
        .map_err(|e| {
            eprintln!("DEBUG JSON parse error: {}", e);
            eprintln!("DEBUG stdout first 500 chars: {}", &stdout.chars().take(500).collect::<String>());
            AudioExportError::Json(e)
        })?;

    // Validate output
    if rendered.left_channel.len() != rendered.sample_count
        || rendered.right_channel.len() != rendered.sample_count
    {
        return Err(AudioExportError::RenderError(
            "Mismatched audio buffer sizes".to_string()
        ));
    }

    Ok((rendered.left_channel, rendered.right_channel))
}

/// Export a Strudel pattern as an audio file (WAV or MP3)
///
/// This function uses the existing Node.js-based dough-export.mjs script
/// to render the pattern to audio, then encodes it to the requested format.
#[tauri::command]
pub async fn export_pattern_audio(
    _window: WebviewWindow,
    pattern_code: String,
    params: ExportParams,
) -> Result<String, String> {
    // Validate parameters
    params.validate()?;

    // Render pattern using Node.js Dough engine
    let (left_channel, right_channel) = if pattern_code.trim().is_empty() {
        // If no pattern provided, generate test tone
        let sample_count = (params.sample_rate as f64 * params.duration_cycles) as usize;
        let mut left = Vec::with_capacity(sample_count);
        let mut right = Vec::with_capacity(sample_count);

        let frequency = 440.0;
        let two_pi_f = 2.0 * std::f32::consts::PI * frequency;

        for i in 0..sample_count {
            let t = i as f32 / params.sample_rate as f32;
            let sample = (two_pi_f * t).sin() * 0.3;
            left.push(sample);
            right.push(sample);
        }

        (left, right)
    } else {
        // Render actual Strudel pattern
        render_pattern(&pattern_code, params.sample_rate, params.duration_cycles).await?
    };

    // Encode based on format
    match params.format.as_str() {
        "wav" => {
            encode_wav(&left_channel, &right_channel, &params)?;
        }
        "mp3" => {
            // TODO: Implement MP3 encoding
            return Err("MP3 export not yet implemented".to_string());
        }
        _ => unreachable!(), // Validation ensures only wav or mp3
    }

    Ok(params.output_path.clone())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_params_validation() {
        let valid_params = ExportParams {
            output_path: "/tmp/test.wav".to_string(),
            format: "wav".to_string(),
            sample_rate: 48000,
            channels: 2,
            duration_cycles: 30.0,
            bit_depth: Some(16),
            mp3_bitrate: None,
        };
        assert!(valid_params.validate().is_ok());

        let invalid_channels = ExportParams {
            channels: 1,
            ..valid_params.clone()
        };
        assert!(invalid_channels.validate().is_err());

        let invalid_format = ExportParams {
            format: "ogg".to_string(),
            ..valid_params.clone()
        };
        assert!(invalid_format.validate().is_err());

        let invalid_bit_depth = ExportParams {
            bit_depth: Some(8),
            ..valid_params.clone()
        };
        assert!(invalid_bit_depth.validate().is_err());
    }
}
