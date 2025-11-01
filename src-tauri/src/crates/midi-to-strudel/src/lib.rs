//! MIDI to Strudel converter library
//!
//! This library provides functionality to convert MIDI files to Strudel pattern code.

pub mod drums;
pub mod instruments;
pub mod midi;
pub mod note;
pub mod output;
pub mod track;

// Re-export main types for convenience
pub use midi::MidiData;
pub use output::OutputFormatter;
pub use track::{ProcessedTrack, TrackBuilder};
