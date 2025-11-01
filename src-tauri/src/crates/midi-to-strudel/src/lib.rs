//! MIDI to Strudel converter library
//!
//! This library provides functionality to convert MIDI files to Strudel pattern code.

pub mod ast;
pub mod drums;
pub mod instruments;
pub mod midi;
pub mod note;
pub mod output;
pub mod track;

// Re-export main types for convenience
pub use ast::{Bar, ModifierValue, Pattern};
pub use drums::is_drum_track_name;
pub use midi::MidiData;
pub use output::OutputFormatter;
pub use track::{ProcessedTrack, TrackBuilder};
