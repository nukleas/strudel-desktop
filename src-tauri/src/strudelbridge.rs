//! Strudel pattern manipulation commands
//!
//! This module provides Tauri commands for parsing, validating, formatting,
//! and evaluating Strudel mini notation patterns using the Rust implementation.

use serde::{Deserialize, Serialize};
use strudel_core::{Fraction, Hap, State, TimeSpan, Value};
use strudel_mini::{evaluate, format, parse, ParseError};
use tauri::command;

/// Error type for Strudel commands
#[derive(Debug, Serialize, Deserialize)]
pub struct StrudelError {
    pub message: String,
    pub location: Option<ErrorLocation>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ErrorLocation {
    pub line: usize,
    pub column: usize,
    pub span_start: usize,
    pub span_end: usize,
}

impl From<ParseError> for StrudelError {
    fn from(err: ParseError) -> Self {
        StrudelError {
            message: err.to_string(),
            location: err.span().map(|span| ErrorLocation {
                line: 0, // TODO: Calculate line from span
                column: 0,
                span_start: span.start,
                span_end: span.end,
            }),
        }
    }
}

impl From<String> for StrudelError {
    fn from(message: String) -> Self {
        StrudelError {
            message,
            location: None,
        }
    }
}

/// Serializable version of Hap for returning to frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct SerializableHap {
    pub value: Value,
    pub part_begin: f64,
    pub part_end: f64,
    pub whole_begin: Option<f64>,
    pub whole_end: Option<f64>,
}

impl From<Hap> for SerializableHap {
    fn from(hap: Hap) -> Self {
        SerializableHap {
            value: hap.value,
            part_begin: hap.part.begin.to_float(),
            part_end: hap.part.end.to_float(),
            whole_begin: hap.whole.as_ref().map(|ts| ts.begin.to_float()),
            whole_end: hap.whole.as_ref().map(|ts| ts.end.to_float()),
        }
    }
}

/// Validate a mini notation pattern
///
/// Returns Ok(()) if the pattern is valid, or an error with location information
#[command]
pub fn validate_pattern(pattern: String) -> Result<(), StrudelError> {
    parse(&pattern)?;
    Ok(())
}

/// Format a mini notation pattern
///
/// Parses the pattern and returns a canonical formatted version
#[command]
pub fn format_pattern(pattern: String) -> Result<String, StrudelError> {
    let ast = parse(&pattern)?;
    Ok(format(&ast))
}

/// Evaluate a pattern and return its events for a given time range
///
/// Returns a list of events (Haps) that occur in the specified cycle range
#[command]
pub fn evaluate_pattern(
    pattern: String,
    from_cycle: f64,
    duration_cycles: f64,
) -> Result<Vec<SerializableHap>, StrudelError> {
    // Parse the pattern
    let ast = parse(&pattern)?;

    // Evaluate to a Pattern
    let pat = evaluate(&ast).map_err(|e| StrudelError::from(e.to_string()))?;

    // Query the pattern for the specified time range
    let begin = Fraction::from_float(from_cycle);
    let end = Fraction::from_float(from_cycle + duration_cycles);
    let span = TimeSpan::new(begin, end);
    let state = State::new(span);

    let haps = pat.query(state);

    // Convert to serializable format
    Ok(haps.into_iter().map(SerializableHap::from).collect())
}

/// Analyze a pattern and return metrics
#[command]
pub fn analyze_pattern(
    pattern: String,
    cycles: f64,
) -> Result<PatternMetrics, StrudelError> {
    // Parse and evaluate the pattern
    let ast = parse(&pattern)?;
    let pat = evaluate(&ast).map_err(|e| StrudelError::from(e.to_string()))?;

    // Query for the specified number of cycles
    let begin = Fraction::from_int(0);
    let end = Fraction::from_float(cycles);
    let span = TimeSpan::new(begin, end);
    let state = State::new(span);

    let haps = pat.query(state);

    // Calculate metrics
    let event_count = haps.len();
    let density = event_count as f64 / cycles;

    // Count unique values
    let mut unique_values: Vec<String> = haps
        .iter()
        .map(|h| h.value.to_string())
        .collect::<std::collections::HashSet<_>>()
        .into_iter()
        .collect();
    unique_values.sort();

    Ok(PatternMetrics {
        event_count,
        density,
        cycles,
        unique_values,
    })
}

#[derive(Debug, Serialize, Deserialize)]
pub struct PatternMetrics {
    pub event_count: usize,
    pub density: f64,
    pub cycles: f64,
    pub unique_values: Vec<String>,
}

// ============================================================================
// MIDI Import Functionality
// ============================================================================

use midi_to_strudel::{MidiData, OutputFormatter, TrackBuilder};
use std::path::Path;

/// Options for MIDI to Strudel conversion
#[derive(Debug, Serialize, Deserialize)]
pub struct MidiConversionOptions {
    /// Maximum number of bars to convert (0 = unlimited)
    #[serde(default)]
    pub bar_limit: usize,

    /// Use compact notation (compress repetitions with ! operator)
    #[serde(default)]
    pub compact: bool,

    /// Tempo scaling factor (1.0 = original tempo)
    #[serde(default = "default_tempo_scale")]
    pub tempo_scale: f64,

    /// Number of notes per bar for quantization
    #[serde(default = "default_notes_per_bar")]
    pub notes_per_bar: usize,

    /// Indentation size in spaces
    #[serde(default = "default_tab_size")]
    pub tab_size: usize,
}

fn default_tempo_scale() -> f64 { 1.0 }
fn default_notes_per_bar() -> usize { 64 }
fn default_tab_size() -> usize { 2 }

impl Default for MidiConversionOptions {
    fn default() -> Self {
        Self {
            bar_limit: 0,
            compact: false,
            tempo_scale: 1.0,
            notes_per_bar: 64,
            tab_size: 2,
        }
    }
}

/// Import and convert a MIDI file to Strudel code
#[command]
pub fn import_midi_file(
    file_path: String,
    options: Option<MidiConversionOptions>,
) -> Result<String, StrudelError> {
    let opts = options.unwrap_or_default();
    let path = Path::new(&file_path);

    // Parse MIDI file using the full midi-to-strudel library
    let midi_data = MidiData::from_file(path)
        .map_err(|e| StrudelError::from(format!("Failed to parse MIDI file: {}", e)))?;

    // Build tracks using the proper TrackBuilder
    let track_builder = TrackBuilder::new(
        midi_data.cycle_len,
        opts.bar_limit,
        false, // flat_sequences = false for full polyphonic patterns
        opts.notes_per_bar,
    );
    let tracks = track_builder.build_tracks(&midi_data.track_info);

    // Format output using the proper OutputFormatter
    let formatter = OutputFormatter::new(opts.tab_size, opts.compact);
    let scaled_bpm = midi_data.bpm * opts.tempo_scale;
    let output = formatter.build_output(&tracks, scaled_bpm);

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_pattern() {
        assert!(validate_pattern("bd sd cp".to_string()).is_ok());
        assert!(validate_pattern("bd [sd cp]".to_string()).is_ok());
        assert!(validate_pattern("bd*2".to_string()).is_ok());
        assert!(validate_pattern("invalid!@#$%".to_string()).is_err());
    }

    #[test]
    fn test_format_pattern() {
        let result = format_pattern("bd   sd    cp".to_string()).unwrap();
        // Formatting should normalize whitespace
        assert!(result.contains("bd"));
        assert!(result.contains("sd"));
        assert!(result.contains("cp"));
    }

    #[test]
    fn test_evaluate_pattern() {
        let result = evaluate_pattern("bd sd cp".to_string(), 0.0, 1.0).unwrap();
        assert_eq!(result.len(), 3);
        assert_eq!(result[0].value, Value::String("bd".into()));
        assert_eq!(result[1].value, Value::String("sd".into()));
        assert_eq!(result[2].value, Value::String("cp".into()));
    }

    #[test]
    fn test_analyze_pattern() {
        let result = analyze_pattern("bd sd cp".to_string(), 2.0).unwrap();
        assert_eq!(result.event_count, 6); // 3 events per cycle * 2 cycles
        assert_eq!(result.density, 3.0); // 6 events / 2 cycles
        assert_eq!(result.unique_values.len(), 3);
    }
}
