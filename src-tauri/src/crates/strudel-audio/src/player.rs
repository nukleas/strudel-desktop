//! High-level audio player for Strudel patterns

use crate::{AudioEngine, Pattern, Result, SampleLoader, Scheduler};
use parking_lot::Mutex;
use std::sync::Arc;
use std::time::Duration;

/// Configuration for the audio player
#[derive(Debug, Clone)]
pub struct PlayerConfig {
    /// Tempo in BPM (beats per minute)
    pub tempo: f64,
    /// Lookahead time for scheduling events
    pub lookahead: Duration,
    /// Fallback URL for sample loading
    pub fallback_url: Option<String>,
}

impl Default for PlayerConfig {
    fn default() -> Self {
        PlayerConfig {
            tempo: 120.0,
            lookahead: Duration::from_millis(100),
            fallback_url: Some(
                "https://raw.githubusercontent.com/tidalcycles/Dirt-Samples/master".to_string(),
            ),
        }
    }
}

/// High-level audio player for Strudel patterns
pub struct Player {
    /// Audio engine
    engine: Arc<AudioEngine>,
    /// Sample loader
    loader: Arc<SampleLoader>,
    /// Scheduler
    scheduler: Arc<Mutex<Scheduler>>,
    /// Current pattern being played
    pattern: Arc<Mutex<Option<Pattern>>>,
    /// Configuration
    config: PlayerConfig,
}

impl Player {
    /// Create a new player with the given configuration
    pub fn new(config: PlayerConfig) -> Result<Self> {
        #[allow(clippy::arc_with_non_send_sync)]
        let engine = Arc::new(AudioEngine::new()?);

        let mut loader = SampleLoader::new();
        if let Some(url) = &config.fallback_url {
            loader = loader.with_fallback_url(url.clone());
        }
        let loader = Arc::new(loader);

        let scheduler = Arc::new(Mutex::new(Scheduler::new(
            Arc::clone(&loader),
            config.tempo,
        )));

        Ok(Player {
            engine,
            loader,
            scheduler,
            pattern: Arc::new(Mutex::new(None)),
            config,
        })
    }

    /// Create a player with default configuration
    pub fn with_defaults() -> Result<Self> {
        Self::new(PlayerConfig::default())
    }

    /// Set the tempo in BPM
    pub fn set_tempo(&self, tempo: f64) {
        self.scheduler.lock().set_tempo(tempo);
    }

    /// Get the current tempo
    pub fn tempo(&self) -> f64 {
        self.scheduler.lock().tempo()
    }

    /// Get the sample loader (for preloading samples)
    pub fn loader(&self) -> Arc<SampleLoader> {
        Arc::clone(&self.loader)
    }

    /// Start playing a pattern
    pub fn play(&self, pattern: Pattern) -> Result<()> {
        // Store the pattern
        *self.pattern.lock() = Some(pattern);

        // Reset the scheduler
        self.scheduler.lock().reset();

        // Start the audio stream
        let scheduler = Arc::clone(&self.scheduler);
        let pattern = Arc::clone(&self.pattern);
        let lookahead = self.config.lookahead;
        let sample_rate = self.engine.sample_rate();

        self.engine.start(move |buffer| {
            // Update scheduler to trigger new events
            let mut sched = scheduler.lock();
            if let Some(pat) = pattern.lock().as_ref() {
                sched.update(pat, lookahead);
            }

            // Fill the buffer with audio from active voices
            sched.fill_buffer(buffer, sample_rate);
        })?;

        Ok(())
    }

    /// Stop playback
    pub fn stop(&self) -> Result<()> {
        self.engine.stop()?;
        *self.pattern.lock() = None;
        self.scheduler.lock().reset();
        Ok(())
    }

    /// Check if currently playing
    pub fn is_playing(&self) -> bool {
        self.engine.is_running()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_player_creation() {
        let _player = Player::with_defaults();
        // Note: This will fail if no audio device is available
    }
}
