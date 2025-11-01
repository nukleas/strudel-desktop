//! Voice represents a single sample playback instance

use crate::Sample;
use std::sync::Arc;

/// A voice for playing back a single sample
pub struct Voice {
    /// The sample being played
    sample: Arc<Sample>,
    /// Current playback position (in frames)
    position: f64,
    /// Playback speed multiplier (1.0 = normal, 2.0 = double speed)
    speed: f64,
    /// Gain/volume (0.0 to 1.0)
    gain: f32,
    /// Whether this voice is still active
    active: bool,
}

impl Voice {
    /// Create a new voice for the given sample
    pub fn new(sample: Arc<Sample>) -> Self {
        Voice {
            sample,
            position: 0.0,
            speed: 1.0,
            gain: 1.0,
            active: true,
        }
    }

    /// Set the playback speed
    pub fn set_speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    /// Set the gain
    pub fn set_gain(mut self, gain: f32) -> Self {
        self.gain = gain.clamp(0.0, 1.0);
        self
    }

    /// Check if this voice is still active
    pub fn is_active(&self) -> bool {
        self.active
    }

    /// Get the next stereo sample pair (L, R)
    ///
    /// Returns None if the voice has finished playing
    pub fn next_sample(&mut self, output_sample_rate: u32) -> Option<(f32, f32)> {
        if !self.active {
            return None;
        }

        let frames = self.sample.frames();

        if self.position >= frames as f64 {
            self.active = false;
            return None;
        }

        // Get the sample value (with linear interpolation for better quality)
        let (left, right) = if self.sample.channels == 1 {
            // Mono: duplicate to both channels
            let sample = self.interpolate_sample_at_position(self.position, 0);
            (sample, sample)
        } else {
            // Stereo: interpolate each channel separately
            // Note: for stereo, we need to maintain the frame position and offset by channel
            let frame_pos = self.position;
            let left = self.interpolate_sample_at_position(frame_pos, 0);
            let right = self.interpolate_sample_at_position(frame_pos, 1);
            (left, right)
        };

        // Apply gain
        let left = left * self.gain;
        let right = right * self.gain;

        // Advance position (accounting for sample rate differences and speed)
        let rate_ratio = self.sample.sample_rate as f64 / output_sample_rate as f64;
        self.position += self.speed * rate_ratio;

        Some((left, right))
    }

    /// Interpolate sample at the given fractional position using linear interpolation
    ///
    /// Linear interpolation provides smoother audio playback, especially when:
    /// - Sample rates differ between the audio file and output
    /// - Playback speed is adjusted (time-stretching)
    /// - Pitch shifting is applied
    ///
    /// # Arguments
    /// * `frame_position` - The fractional frame position (can have decimal component)
    /// * `channel_offset` - The channel offset (0 for left/mono, 1 for right in stereo)
    ///
    /// # Returns
    /// Interpolated sample value between -1.0 and 1.0
    fn interpolate_sample_at_position(&self, frame_position: f64, channel_offset: usize) -> f32 {
        let data = &self.sample.data;
        let channels = self.sample.channels as usize;

        // Calculate the actual data index based on frame position and channel
        // For stereo: frame 0 has indices [0, 1], frame 1 has indices [2, 3], etc.
        // For mono: frame 0 has index [0], frame 1 has index [1], etc.
        let base_index = (frame_position.floor() as usize) * channels + channel_offset;

        // Check bounds
        if base_index >= data.len() {
            return 0.0;
        }

        // Get the fractional part of the position (0.0 to 1.0)
        let fraction = (frame_position - frame_position.floor()) as f32;

        // Get the current sample
        let sample_current = data[base_index];

        // If we're at the last sample or beyond, just return the current sample
        let next_index = base_index + channels;
        if next_index >= data.len() {
            return sample_current;
        }

        // Get the next sample (same channel, next frame)
        let sample_next = data[next_index];

        // Linear interpolation formula: current + (next - current) * fraction
        // This creates a smooth transition between adjacent samples
        sample_current + (sample_next - sample_current) * fraction
    }

    /// Fill a buffer with samples from this voice
    ///
    /// The buffer is interleaved stereo (L, R, L, R, ...)
    pub fn fill_buffer(&mut self, buffer: &mut [f32], output_sample_rate: u32) {
        for chunk in buffer.chunks_mut(2) {
            if let Some((left, right)) = self.next_sample(output_sample_rate) {
                if chunk.len() == 2 {
                    chunk[0] += left;
                    chunk[1] += right;
                }
            } else {
                break;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::Arc;

    #[test]
    fn test_voice_basic() {
        // Create a simple test sample (1 second of 440Hz sine wave)
        let sample_rate = 44100;
        let duration = 1.0;
        let frequency = 440.0;
        let num_samples = (sample_rate as f64 * duration) as usize;

        let mut data = Vec::with_capacity(num_samples * 2);
        for i in 0..num_samples {
            let t = i as f64 / sample_rate as f64;
            let value = (2.0 * std::f64::consts::PI * frequency * t).sin() as f32;
            data.push(value); // Left
            data.push(value); // Right
        }

        let sample = Arc::new(Sample {
            name: "test".to_string(),
            index: 0,
            data: Arc::new(data),
            sample_rate,
            channels: 2,
        });

        let mut voice = Voice::new(sample);

        // Get a few samples
        assert!(voice.next_sample(sample_rate).is_some());
        assert!(voice.is_active());
    }

    #[test]
    fn test_linear_interpolation() {
        // Create a simple test sample with known values for easy verification
        // Using a linear ramp: [0.0, 0.1, 0.2, 0.3, 0.4, 0.5]
        let data = vec![
            0.0, 0.0, // Frame 0: Left, Right
            0.2, 0.2, // Frame 1: Left, Right
            0.4, 0.4, // Frame 2: Left, Right
            0.6, 0.6, // Frame 3: Left, Right
        ];

        let sample = Arc::new(Sample {
            name: "test_interp".to_string(),
            index: 0,
            data: Arc::new(data),
            sample_rate: 44100,
            channels: 2,
        });

        let voice = Voice::new(sample);

        // Test interpolation at exact frame boundaries (should return exact values)
        assert_eq!(voice.interpolate_sample_at_position(0.0, 0), 0.0);
        assert_eq!(voice.interpolate_sample_at_position(1.0, 0), 0.2);
        assert_eq!(voice.interpolate_sample_at_position(2.0, 0), 0.4);

        // Test interpolation at midpoints (should return average of adjacent samples)
        let mid_0_1 = voice.interpolate_sample_at_position(0.5, 0);
        assert!((mid_0_1 - 0.1).abs() < 0.001, "Expected ~0.1, got {}", mid_0_1);

        let mid_1_2 = voice.interpolate_sample_at_position(1.5, 0);
        assert!((mid_1_2 - 0.3).abs() < 0.001, "Expected ~0.3, got {}", mid_1_2);

        // Test interpolation at quarter points
        let quarter = voice.interpolate_sample_at_position(0.25, 0);
        assert!((quarter - 0.05).abs() < 0.001, "Expected ~0.05, got {}", quarter);

        let three_quarters = voice.interpolate_sample_at_position(0.75, 0);
        assert!((three_quarters - 0.15).abs() < 0.001, "Expected ~0.15, got {}", three_quarters);

        // Test right channel interpolation
        let right_mid = voice.interpolate_sample_at_position(0.5, 1);
        assert!((right_mid - 0.1).abs() < 0.001, "Expected ~0.1, got {}", right_mid);
    }

    #[test]
    fn test_interpolation_edge_cases() {
        // Create a simple stereo sample
        let data = vec![
            0.0, 1.0, // Frame 0
            0.5, 0.5, // Frame 1
        ];

        let sample = Arc::new(Sample {
            name: "test_edge".to_string(),
            index: 0,
            data: Arc::new(data),
            sample_rate: 44100,
            channels: 2,
        });

        let voice = Voice::new(sample);

        // Test at the last valid frame - should not interpolate with non-existent next frame
        let last_frame = voice.interpolate_sample_at_position(1.0, 0);
        assert_eq!(last_frame, 0.5);

        // Test beyond bounds - should return 0.0
        let beyond = voice.interpolate_sample_at_position(10.0, 0);
        assert_eq!(beyond, 0.0);
    }

    #[test]
    fn test_mono_interpolation() {
        // Create a mono sample
        let data = vec![0.0, 0.4, 0.8];

        let sample = Arc::new(Sample {
            name: "test_mono".to_string(),
            index: 0,
            data: Arc::new(data),
            sample_rate: 44100,
            channels: 1,
        });

        let voice = Voice::new(sample);

        // Test exact positions
        assert_eq!(voice.interpolate_sample_at_position(0.0, 0), 0.0);
        assert_eq!(voice.interpolate_sample_at_position(1.0, 0), 0.4);
        assert_eq!(voice.interpolate_sample_at_position(2.0, 0), 0.8);

        // Test midpoint interpolation
        let mid = voice.interpolate_sample_at_position(0.5, 0);
        assert!((mid - 0.2).abs() < 0.001, "Expected ~0.2, got {}", mid);
    }

    #[test]
    fn test_playback_with_speed_changes() {
        // Create a simple sample
        let data = vec![0.0, 0.0, 0.5, 0.5, 1.0, 1.0];

        let sample = Arc::new(Sample {
            name: "test_speed".to_string(),
            index: 0,
            data: Arc::new(data),
            sample_rate: 44100,
            channels: 2,
        });

        // Test with 2x speed
        let mut voice = Voice::new(sample.clone()).set_speed(2.0);

        // First sample should work
        let first = voice.next_sample(44100);
        assert!(first.is_some());

        // With 2x speed, we should consume the sample faster
        // The position should advance by 2.0 per call
        assert!(voice.position >= 1.9 && voice.position <= 2.1);
    }
}
