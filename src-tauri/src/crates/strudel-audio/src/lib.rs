//! Native audio playback engine for Strudel patterns
//!
//! This crate provides a self-contained audio playback system that can:
//! - Play Strudel patterns using bundled samples
//! - Fall back to HTTP loading for additional samples
//! - Schedule sample triggers with precise timing
//! - Mix multiple voices with gain control

pub mod engine;
pub mod player;
pub mod samples;
pub mod scheduler;
pub mod voice;

pub use engine::AudioEngine;
pub use player::{Player, PlayerConfig};
pub use samples::{Sample, SampleBank, SampleLoader};
pub use scheduler::Scheduler;
pub use voice::Voice;

/// Re-export common types from strudel-core
pub use strudel_core::{Fraction, Hap, Pattern, State, TimeSpan, Value};

/// Audio playback errors
#[derive(Debug, thiserror::Error)]
pub enum AudioError {
    #[error("Audio device error: {0}")]
    DeviceError(String),

    #[error("Sample not found: {0}")]
    SampleNotFound(String),

    #[error("Failed to decode audio: {0}")]
    DecodeError(String),

    #[error("Failed to load sample from URL: {0}")]
    HttpError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Other error: {0}")]
    Other(String),
}

pub type Result<T> = std::result::Result<T, AudioError>;

#[cfg(test)]
mod tests {
    #[test]
    fn it_works() {
        assert_eq!(2 + 2, 4);
    }
}
