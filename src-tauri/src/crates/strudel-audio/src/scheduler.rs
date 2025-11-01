//! Scheduler for triggering pattern events at precise times

use crate::{Fraction, Hap, Pattern, SampleLoader, State, TimeSpan, Value, Voice};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Scheduler for querying patterns and triggering samples
pub struct Scheduler {
    /// Sample loader
    loader: Arc<SampleLoader>,
    /// Active voices
    voices: Arc<Mutex<Vec<Voice>>>,
    /// Tempo in BPM (beats per minute)
    tempo: f64,
    /// When playback started
    start_time: Instant,
    /// Current cycle position
    current_cycle: Fraction,
}

impl Scheduler {
    /// Create a new scheduler
    pub fn new(loader: Arc<SampleLoader>, tempo: f64) -> Self {
        Scheduler {
            loader,
            voices: Arc::new(Mutex::new(Vec::new())),
            tempo,
            start_time: Instant::now(),
            current_cycle: Fraction::from(0),
        }
    }

    /// Set the tempo in BPM
    pub fn set_tempo(&mut self, tempo: f64) {
        self.tempo = tempo;
    }

    /// Get the tempo in BPM
    pub fn tempo(&self) -> f64 {
        self.tempo
    }

    /// Get the current time in cycles since start
    pub fn current_time(&self) -> Fraction {
        let elapsed = self.start_time.elapsed();
        let seconds = elapsed.as_secs_f64();

        // Convert seconds to cycles based on tempo
        // tempo is in beats per minute, assuming 4 beats per cycle
        let cycles_per_second = self.tempo / 60.0 / 4.0;
        let cycles = seconds * cycles_per_second;

        Fraction::from_float(cycles)
    }

    /// Query a pattern for the current time window and trigger any new events
    pub fn update(&mut self, pattern: &Pattern, lookahead: Duration) {
        let now = self.current_time();
        let lookahead_cycles = Fraction::from_float(
            lookahead.as_secs_f64() * self.tempo / 60.0 / 4.0
        );

        // Query the pattern for events in the lookahead window
        let span = TimeSpan::new(now, now + lookahead_cycles);
        let state = State::new(span);
        let haps = pattern.query(state);

        // Trigger each event
        for hap in haps {
            if hap.part.begin >= self.current_cycle {
                self.trigger_hap(&hap);
            }
        }

        self.current_cycle = now;
    }

    /// Trigger a single hap (event)
    fn trigger_hap(&mut self, hap: &Hap) {
        // Extract sample name and index from the value
        let (sample_name, index, gain, speed) = match &hap.value {
            Value::String(s) => {
                // Parse "bd:0" or just "bd"
                if let Some((name, idx)) = s.split_once(':') {
                    let index = idx.parse::<usize>().unwrap_or(0);
                    (name.to_string(), index, 1.0, 1.0)
                } else {
                    (s.clone(), 0, 1.0, 1.0)
                }
            }
            _ => return, // Skip non-string values for now
        };

        // Try to load the sample
        if self.loader.load_bank(&sample_name).is_err() {
            // Sample not available
            return;
        }

        // Get the sample
        if let Ok(sample) = self.loader.get_sample(&sample_name, index) {
            // Create a voice
            let voice = Voice::new(Arc::new(sample))
                .set_gain(gain)
                .set_speed(speed);

            // Add to active voices
            self.voices.lock().push(voice);
        }
    }

    /// Fill an audio buffer with the output of all active voices
    pub fn fill_buffer(&mut self, buffer: &mut [f32], sample_rate: u32) {
        // Clear buffer
        buffer.fill(0.0);

        // Mix all voices
        let mut voices = self.voices.lock();
        voices.retain_mut(|voice| {
            voice.fill_buffer(buffer, sample_rate);
            voice.is_active()
        });
    }

    /// Reset the scheduler
    pub fn reset(&mut self) {
        self.start_time = Instant::now();
        self.current_cycle = Fraction::from(0);
        self.voices.lock().clear();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_scheduler_timing() {
        let loader = Arc::new(SampleLoader::new());
        let scheduler = Scheduler::new(loader, 120.0);

        // At 120 BPM (30 cycles per minute), each cycle is 2 seconds
        let time = scheduler.current_time();
        assert_eq!(time, Fraction::from(0));
    }
}
