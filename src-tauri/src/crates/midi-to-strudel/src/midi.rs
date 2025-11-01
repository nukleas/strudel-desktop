use anyhow::{Context, Result};
use midly::{MetaMessage, MidiMessage, Smf, Timing, TrackEventKind};
use std::collections::HashMap;
use std::path::Path;

use crate::note::note_num_to_str;

#[derive(Debug, Clone)]
pub struct NoteEvent {
    pub time_sec: f64,
    pub note: String,
    pub velocity: u8,
    pub duration_sec: Option<f64>,
    pub channel: u8,  // Store channel for each note event
}

#[derive(Debug, Clone)]
pub struct TrackInfo {
    pub events: Vec<NoteEvent>,
    pub channel: Option<u8>,
    pub program: Option<u8>,
    pub name: Option<String>,
    pub pan: Option<u8>,  // MIDI pan value (0=left, 64=center, 127=right)
}

pub struct MidiData {
    pub bpm: f64,
    pub cycle_len: f64,
    pub track_info: HashMap<usize, TrackInfo>,
}

impl MidiData {
    pub fn from_file(path: &Path) -> Result<Self> {
        let data = std::fs::read(path)
            .with_context(|| format!("Failed to read MIDI file: {}", path.display()))?;

        let smf = Smf::parse(&data)
            .context("Failed to parse MIDI file")?;

        let ticks_per_beat = match smf.header.timing {
            Timing::Metrical(tpb) => tpb.as_int() as u32,
            Timing::Timecode(fps, subframe) => {
                // Convert timecode to ticks per beat approximation
                (fps.as_f32() * subframe as f32 * 4.0) as u32
            }
        };

        // Extract tempo from first track
        let tempo = Self::extract_tempo(&smf)?;
        let bpm = 60_000_000.0 / tempo as f64;
        let cycle_len = 60.0 / bpm * 4.0;

        // Collect note events and instrument info from all tracks
        let track_info = Self::collect_track_info(&smf, ticks_per_beat, tempo);

        Ok(MidiData {
            bpm,
            cycle_len,
            track_info,
        })
    }

    fn extract_tempo(smf: &Smf) -> Result<u32> {
        for track in &smf.tracks {
            for event in track {
                if let TrackEventKind::Meta(MetaMessage::Tempo(tempo)) = event.kind {
                    return Ok(tempo.as_int());
                }
            }
        }
        // Default tempo: 120 BPM = 500000 microseconds per beat
        Ok(500000)
    }

    fn collect_track_info(
        smf: &Smf,
        ticks_per_beat: u32,
        tempo: u32,
    ) -> HashMap<usize, TrackInfo> {
        let mut track_info_map = HashMap::new();

        for (track_idx, track) in smf.tracks.iter().enumerate() {
            let mut time_sec = 0.0;
            let mut events = Vec::new();
            let mut channel: Option<u8> = None;
            let mut program: Option<u8> = None;
            let mut track_name: Option<String> = None;
            let mut pan_values: Vec<u8> = Vec::new();  // Collect all pan CC messages

            // Track active notes: (channel, note_num) -> (start_time, velocity, event_index)
            let mut active_notes: HashMap<(u8, u8), (f64, u8, usize)> = HashMap::new();

            for event in track {
                // Convert delta time to seconds
                let delta_sec = tick_to_second(event.delta.as_int(), ticks_per_beat, tempo);
                time_sec += delta_sec;

                match event.kind {
                    TrackEventKind::Midi { channel: ch, message } => {
                        // Remember the channel this track uses
                        if channel.is_none() {
                            channel = Some(ch.as_int());
                        }

                        match message {
                            // Collect note_on events with velocity > 0
                            MidiMessage::NoteOn { key, vel } => {
                                let note_key = (ch.as_int(), key.as_int());

                                if vel.as_int() > 0 {
                                    // Add event without duration initially
                                    let event_idx = events.len();
                                    events.push(NoteEvent {
                                        time_sec,
                                        note: note_num_to_str(key.as_int()),
                                        velocity: vel.as_int(),
                                        duration_sec: None,
                                        channel: ch.as_int(),  // Store the channel for this note
                                    });

                                    // Track as active note
                                    active_notes.insert(note_key, (time_sec, vel.as_int(), event_idx));
                                } else {
                                    // NoteOn with velocity 0 = NoteOff
                                    if let Some((start_time, _, event_idx)) = active_notes.remove(&note_key) {
                                        let duration = time_sec - start_time;
                                        if let Some(note_event) = events.get_mut(event_idx) {
                                            note_event.duration_sec = Some(duration);
                                        }
                                    }
                                }
                            }
                            // Handle explicit NoteOff events
                            MidiMessage::NoteOff { key, vel: _ } => {
                                let note_key = (ch.as_int(), key.as_int());
                                if let Some((start_time, _, event_idx)) = active_notes.remove(&note_key) {
                                    let duration = time_sec - start_time;
                                    if let Some(note_event) = events.get_mut(event_idx) {
                                        note_event.duration_sec = Some(duration);
                                    }
                                }
                            }
                            // Extract program change (instrument)
                            MidiMessage::ProgramChange { program: prog } => {
                                program = Some(prog.as_int());
                            }
                            // Extract pan (CC10)
                            MidiMessage::Controller { controller, value } => {
                                if controller.as_int() == 10 {
                                    pan_values.push(value.as_int());
                                }
                            }
                            _ => {}
                        }
                    }
                    // Extract track name from meta messages
                    TrackEventKind::Meta(MetaMessage::TrackName(name)) => {
                        if let Ok(name_str) = std::str::from_utf8(name) {
                            // Clean track name: trim null bytes and control characters
                            let cleaned = name_str.trim_end_matches('\0').trim();
                            if !cleaned.is_empty() {
                                track_name = Some(cleaned.to_string());
                            }
                        }
                    }
                    _ => {}
                }
            }

            // For any notes still active (missing NoteOff), set a default duration
            // Use 10% of a beat as default (short note)
            let default_duration = (tempo as f64 / 1_000_000.0) * 0.1;
            for (_, (_, _, event_idx)) in active_notes {
                if let Some(note_event) = events.get_mut(event_idx) {
                    if note_event.duration_sec.is_none() {
                        note_event.duration_sec = Some(default_duration);
                    }
                }
            }

            if !events.is_empty() {
                // Calculate average pan value if we have any pan messages
                let pan = if !pan_values.is_empty() {
                    let sum: u32 = pan_values.iter().map(|&v| v as u32).sum();
                    Some((sum / pan_values.len() as u32) as u8)
                } else {
                    None
                };

                track_info_map.insert(
                    track_idx,
                    TrackInfo {
                        events,
                        channel,
                        program,
                        name: track_name,
                        pan,
                    },
                );
            }
        }

        track_info_map
    }
}

fn tick_to_second(ticks: u32, ticks_per_beat: u32, tempo: u32) -> f64 {
    let seconds_per_tick = (tempo as f64 / 1_000_000.0) / ticks_per_beat as f64;
    ticks as f64 * seconds_per_tick
}
