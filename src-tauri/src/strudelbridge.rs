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

use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::collections::HashMap;

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

    // Read MIDI file
    let bytes = std::fs::read(&file_path)
        .map_err(|e| StrudelError::from(format!("Failed to read MIDI file: {}", e)))?;

    // Convert to Strudel code
    convert_midi_bytes(&bytes, opts)
}

/// Convert MIDI bytes to Strudel code
fn convert_midi_bytes(bytes: &[u8], options: MidiConversionOptions) -> Result<String, StrudelError> {
    // Parse MIDI file
    let smf = Smf::parse(bytes)
        .map_err(|e| StrudelError::from(format!("Failed to parse MIDI file: {}", e)))?;

    let ticks_per_beat = match smf.header.timing {
        Timing::Metrical(tpb) => tpb.as_int() as u32,
        Timing::Timecode(fps, subframe) => {
            (fps.as_f32() * subframe as f32 * 4.0) as u32
        }
    };

    // Extract tempo and calculate BPM
    let tempo = extract_tempo(&smf);
    let bpm = 60_000_000.0 / tempo as f64;
    let scaled_bpm = bpm * options.tempo_scale;
    let cycle_len = 60.0 / scaled_bpm * 4.0;

    // Collect track information
    let track_info = collect_track_info(&smf, ticks_per_beat, tempo);

    // Build Strudel code
    let mut output = Vec::new();
    output.push(format!("setcpm({}/4)\n", scaled_bpm as i32));

    for (idx, (_, track)) in track_info.iter().enumerate() {
        // Add track name comment
        if let Some(name) = &track.name {
            output.push(format!("// Track {}: {}", idx + 1, name));
        }

        // Convert track to pattern string
        let pattern = build_track_pattern(track, cycle_len, &options);
        output.push(pattern);
    }

    Ok(output.join("\n"))
}

fn extract_tempo(smf: &Smf) -> u32 {
    for track in &smf.tracks {
        for event in track {
            if let TrackEventKind::Meta(MetaMessage::Tempo(tempo)) = event.kind {
                return tempo.as_int();
            }
        }
    }
    500000 // Default: 120 BPM
}

#[derive(Debug, Clone)]
struct NoteEvent {
    _time_sec: f64,  // Will be used for advanced timing in future
    note: String,
    _velocity: u8,   // Will be used for gain/dynamics in future
}

#[derive(Debug, Clone)]
struct TrackInfo {
    events: Vec<NoteEvent>,
    channel: Option<u8>,
    program: Option<u8>,
    name: Option<String>,
}

fn collect_track_info(smf: &Smf, ticks_per_beat: u32, tempo: u32) -> HashMap<usize, TrackInfo> {
    let mut track_info_map = HashMap::new();

    for (track_idx, track) in smf.tracks.iter().enumerate() {
        let mut time_sec = 0.0;
        let mut events = Vec::new();
        let mut channel: Option<u8> = None;
        let mut program: Option<u8> = None;
        let mut track_name: Option<String> = None;

        for event in track {
            let delta_sec = tick_to_second(event.delta.as_int(), ticks_per_beat, tempo);
            time_sec += delta_sec;

            match event.kind {
                TrackEventKind::Midi { channel: ch, message } => {
                    if channel.is_none() {
                        channel = Some(ch.as_int());
                    }

                    match message {
                        MidiMessage::NoteOn { key, vel } => {
                            if vel.as_int() > 0 {
                                events.push(NoteEvent {
                                    _time_sec: time_sec,
                                    note: note_num_to_str(key.as_int()),
                                    _velocity: vel.as_int(),
                                });
                            }
                        }
                        MidiMessage::ProgramChange { program: prog } => {
                            program = Some(prog.as_int());
                        }
                        _ => {}
                    }
                }
                TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                    if let Ok(name_str) = std::str::from_utf8(name) {
                        let cleaned = name_str.trim_end_matches('\0').trim();
                        if !cleaned.is_empty() {
                            track_name = Some(cleaned.to_string());
                        }
                    }
                }
                _ => {}
            }
        }

        if !events.is_empty() {
            track_info_map.insert(track_idx, TrackInfo {
                events,
                channel,
                program,
                name: track_name,
            });
        }
    }

    track_info_map
}

fn tick_to_second(ticks: u32, ticks_per_beat: u32, tempo: u32) -> f64 {
    let seconds_per_tick = (tempo as f64 / 1_000_000.0) / ticks_per_beat as f64;
    ticks as f64 * seconds_per_tick
}

fn note_num_to_str(note_num: u8) -> String {
    let note_names = ["c", "cs", "d", "ds", "e", "f", "fs", "g", "gs", "a", "as", "b"];
    let octave = (note_num / 12) as i32 - 1;
    let note_idx = (note_num % 12) as usize;
    format!("{}{}", note_names[note_idx], octave)
}

fn build_track_pattern(track: &TrackInfo, _cycle_len: f64, _options: &MidiConversionOptions) -> String {
    // Simplified pattern builder - creates a basic note sequence
    // For a full implementation, we'd need the complete track building logic

    let is_drum = track.channel == Some(9); // MIDI channel 10 (0-indexed: 9) is drums

    if is_drum {
        // Drum pattern
        let samples: Vec<String> = track.events.iter()
            .map(|e| drum_note_to_sample(&e.note))
            .collect();

        format!("$: s(`{}`)", samples.join(" "))
    } else {
        // Melodic pattern
        let notes: Vec<String> = track.events.iter()
            .map(|e| e.note.clone())
            .collect();

        let instrument = program_to_instrument(track.program.unwrap_or(0));
        format!("$: note(`{}`).sound(\"{}\")", notes.join(" "), instrument)
    }
}

fn drum_note_to_sample(note: &str) -> String {
    // Map MIDI drum notes to Strudel sample names
    // This is a simplified mapping
    match note {
        "c1" => "bd".to_string(),
        "d1" | "e1" => "sd".to_string(),
        "fs1" | "gs1" => "hh".to_string(),
        "a1" => "oh".to_string(),
        _ => "perc".to_string(),
    }
}

fn program_to_instrument(program: u8) -> String {
    // Map General MIDI program numbers to Strudel instruments
    match program {
        0..=7 => "piano",
        8..=15 => "glockenspiel",
        16..=23 => "organ",
        24..=31 => "guitar",
        32..=39 => "bass",
        40..=47 => "strings",
        48..=55 => "ensemble",
        56..=63 => "brass",
        64..=71 => "reed",
        72..=79 => "pipe",
        80..=87 => "lead",
        88..=95 => "pad",
        _ => "piano",
    }.to_string()
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
