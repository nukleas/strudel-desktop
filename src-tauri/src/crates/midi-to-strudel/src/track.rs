use std::collections::HashMap;

use crate::ast::Bar;
use crate::drums::{gm_drum_to_sample, note_name_to_midi_num};
use crate::midi::{NoteEvent, TrackInfo};

#[derive(Debug, Clone)]
pub struct ProcessedTrack {
    pub bars: Vec<Bar>,
    pub gains: Vec<f32>,  // Gain value for each bar (0.0 to 1.0)
    pub sustains: Vec<f32>,  // Sustain value for each bar (relative to cycle_len)
    pub pan: Option<f32>,  // Pan value (0.0=left, 0.5=center, 1.0=right)
    #[allow(dead_code)]
    pub channel: Option<u8>,
    pub program: Option<u8>,
    pub name: Option<String>,
    pub is_drum: bool,
}

pub struct TrackBuilder {
    cycle_len: f64,
    bar_limit: usize,
    flat_sequences: bool,
    notes_per_bar: usize,
    detect_drum_names: bool,
    forced_drum_channels: Vec<u8>,
}

impl TrackBuilder {
    pub fn new(
        cycle_len: f64,
        bar_limit: usize,
        flat_sequences: bool,
        notes_per_bar: usize,
        detect_drum_names: bool,
        forced_drum_channels: Vec<u8>,
    ) -> Self {
        Self {
            cycle_len,
            bar_limit,
            flat_sequences,
            notes_per_bar,
            detect_drum_names,
            forced_drum_channels,
        }
    }

    pub fn build_tracks(&self, track_info: &HashMap<usize, TrackInfo>) -> Vec<ProcessedTrack> {
        let mut tracks = Vec::new();

        // Process tracks in sorted order for consistent output
        let mut track_indices: Vec<_> = track_info.keys().collect();
        track_indices.sort();

        for &track_idx in track_indices {
            let info = &track_info[&track_idx];
            let adjusted = self.adjust_near_cycle_end(&info.events);

            if adjusted.is_empty() {
                continue;
            }

            // Group events by channel (since a single MIDI track can have multiple channels)
            let mut events_by_channel: HashMap<u8, Vec<NoteEvent>> = HashMap::new();
            for event in adjusted {
                events_by_channel.entry(event.channel).or_default().push(event);
            }

            // Create a ProcessedTrack for each channel
            for (channel, channel_events) in events_by_channel {
                // Check if this is a drum track using multiple methods:
                // 1. Standard MIDI channel 10 (index 9)
                // 2. Forced drum channels from --force-drums flag
                // 3. Track name detection if --detect-drum-names flag is set
                let is_drum = channel == 9
                    || self.forced_drum_channels.contains(&channel)
                    || (self.detect_drum_names
                        && info.name.as_ref().map_or(false, |name| {
                            crate::drums::is_drum_track_name(name)
                        }));

                let max_time = channel_events.iter().map(|e| e.time_sec).fold(0.0, f64::max);
                let num_cycles = ((max_time / self.cycle_len) as usize + 1).min(
                    if self.bar_limit > 0 {
                        self.bar_limit
                    } else {
                        usize::MAX
                    },
                );

                let mut bars = Vec::new();
                let mut gains = Vec::new();
                let mut sustains = Vec::new();

                for cycle in 0..num_cycles {
                    let start = cycle as f64 * self.cycle_len;
                    let end = start + self.cycle_len;

                    let notes_in_cycle: Vec<_> = channel_events
                        .iter()
                        .filter(|e| e.time_sec >= start && e.time_sec < end)
                        .cloned()
                        .collect();

                if notes_in_cycle.is_empty() {
                    bars.push(Bar::Rest); // Use - for rests (same as Python version)
                    gains.push(0.0); // No gain for empty bars
                    sustains.push(0.0); // No sustain for empty bars
                    continue;
                }

                // Calculate average gain from velocities with musical scaling
                // Use logarithmic scaling for more perceptual accuracy
                let avg_velocity: f32 = notes_in_cycle.iter().map(|e| e.velocity as f32).sum::<f32>()
                    / notes_in_cycle.len() as f32;

                // Logarithmic velocity curve (more perceptually accurate):
                // velocity 32 (pp) → 0.25 gain
                // velocity 64 (mf) → 0.50 gain
                // velocity 96 (f)  → 0.75 gain
                // velocity 127(ff) → 1.0 gain
                // Allow values > 1.0 for very loud notes (boosting)
                let base_gain = (avg_velocity / 127.0).powf(0.5).max(0.15);

                // Reduce drum gain by 40% since drum samples are naturally louder
                let gain = if is_drum { base_gain * 0.6 } else { base_gain };

                // Calculate sustain from note durations
                // Use maximum duration to preserve sustained notes (not average, which gets
                // pulled down by short articulation notes like grace notes or ornaments)
                // Normalize to cycle_len (1.0 = full cycle duration)
                let max_duration: f32 = notes_in_cycle
                    .iter()
                    .filter_map(|e| e.duration_sec.map(|d| d as f32))
                    .fold(0.0, |a, b| a.max(b));

                // Normalize to cycle length and clamp to reasonable range
                let sustain = (max_duration / self.cycle_len as f32).clamp(0.01, 2.0);

                let bar = if is_drum {
                    // Convert drum notes to samples with proper timing
                    self.get_drum_bar(&notes_in_cycle, start)
                } else if self.flat_sequences {
                    self.get_flat_mode_bar(&notes_in_cycle)
                } else {
                    self.get_poly_mode_bar(&notes_in_cycle, start)
                };

                    bars.push(bar);
                    gains.push(gain);
                    sustains.push(sustain);
                }

                if !bars.is_empty() {
                    // Convert MIDI pan (0-127) to Strudel pan (0.0-1.0)
                    // MIDI: 0=left, 64=center, 127=right
                    // Strudel: 0.0=left, 0.5=center, 1.0=right
                    let pan = info.pan.map(|midi_pan| midi_pan as f32 / 127.0);

                    // Use channel-specific name if available
                    let track_name = info.name.clone().map(|name| {
                        // If this is a drum track, add "(Drums)" suffix
                        if is_drum && !name.to_lowercase().contains("drum") {
                            format!("{} (Drums)", name)
                        } else {
                            name
                        }
                    });

                    tracks.push(ProcessedTrack {
                        bars,
                        gains,
                        sustains,
                        pan,
                        channel: Some(channel),
                        program: info.program,
                        name: track_name,
                        is_drum,
                    });
                }
            }  // End of channel loop
        }

        tracks
    }

    fn adjust_near_cycle_end(&self, events: &[NoteEvent]) -> Vec<NoteEvent> {
        events
            .iter()
            .map(|event| {
                let rel = (event.time_sec % self.cycle_len) / self.cycle_len;
                if rel > 0.95 {
                    NoteEvent {
                        time_sec: (event.time_sec / self.cycle_len).ceil() * self.cycle_len,
                        note: event.note.clone(),
                        velocity: event.velocity,
                        duration_sec: event.duration_sec,
                        channel: event.channel,
                    }
                } else {
                    event.clone()
                }
            })
            .collect()
    }

    fn get_drum_bar(&self, events: &[NoteEvent], start: f64) -> Bar {
        // Use subdivision logic like melodic tracks for proper timing
        let mut subdivisions = vec![Bar::Rest; self.notes_per_bar];
        let mut time_groups: std::collections::HashMap<usize, Vec<String>> = std::collections::HashMap::new();

        for event in events {
            // Quantize the event time to a subdivision index
            let rel_time = self.quantize_time(event.time_sec, start);
            let idx = (rel_time * self.notes_per_bar as f64).round() as usize;

            if idx >= self.notes_per_bar {
                continue;
            }

            // Convert note to drum sample
            let sample = if let Some(midi_num) = note_name_to_midi_num(&event.note) {
                if let Some(s) = gm_drum_to_sample(midi_num) {
                    s.to_string()
                } else {
                    format!("perc:{}", event.note)
                }
            } else {
                continue;
            };

            // Group samples by time index
            time_groups.entry(idx).or_default().push(sample);
        }

        // Build subdivisions array with drum samples
        for (idx, samples) in time_groups {
            if idx < self.notes_per_bar {
                subdivisions[idx] = if samples.len() == 1 {
                    Bar::Note(samples[0].clone())
                } else {
                    // Multiple drums at same time - use commas (layered)
                    Bar::Chord(samples)
                };
            }
        }

        // Check if all are rests
        if subdivisions.iter().all(|s| s.is_silent()) {
            return Bar::Rest;
        }

        // Simplify subdivisions
        let simplified = self.simplify_subdivisions(&subdivisions);

        if simplified.len() == 1 {
            simplified[0].clone()
        } else {
            Bar::Subdivision(simplified)
        }
    }

    fn get_flat_mode_bar(&self, events: &[NoteEvent]) -> Bar {
        let mut sorted = events.to_vec();
        sorted.sort_by(|a, b| a.time_sec.partial_cmp(&b.time_sec).unwrap());

        let notes: Vec<String> = sorted.iter().map(|e| e.note.clone()).collect();

        if notes.len() == 1 {
            Bar::Note(notes[0].clone())
        } else {
            Bar::Sequence(notes)
        }
    }

    fn get_poly_mode_bar(&self, events: &[NoteEvent], cycle_start: f64) -> Bar {
        // Group notes by their quantized time position
        let mut time_groups: HashMap<usize, Vec<String>> = HashMap::new();

        for event in events {
            let pos = self.quantize_time(event.time_sec, cycle_start);
            let idx = (pos * self.notes_per_bar as f64).round() as usize;

            // Check if we should merge with an existing time group (within threshold)
            let mut merged = false;
            for (&existing_idx, group) in time_groups.iter_mut() {
                if existing_idx.abs_diff(idx) < 1 {
                    group.push(event.note.clone());
                    merged = true;
                    break;
                }
            }

            if !merged {
                time_groups.insert(idx, vec![event.note.clone()]);
            }
        }

        if time_groups.is_empty() {
            return Bar::Rest;
        }

        // Build subdivisions array
        let mut subdivisions = vec![Bar::Rest; self.notes_per_bar];

        for (idx, notes) in time_groups {
            if idx < self.notes_per_bar {
                subdivisions[idx] = if notes.len() == 1 {
                    Bar::Note(notes[0].clone())
                } else {
                    Bar::Chord(notes)
                };
            }
        }

        // Check if all are rests
        if subdivisions.iter().all(|s| s.is_silent()) {
            return Bar::Rest;
        }

        // Simplify subdivisions
        let simplified = self.simplify_subdivisions(&subdivisions);

        if simplified.len() == 1 {
            simplified[0].clone()
        } else {
            Bar::Subdivision(simplified)
        }
    }

    fn quantize_time(&self, timestamp: f64, cycle_start: f64) -> f64 {
        let rel_time = (timestamp - cycle_start) / self.cycle_len;
        let quantized = (rel_time * self.notes_per_bar as f64).round() / self.notes_per_bar as f64;
        quantized.min(1.0 - 1e-9)
    }

    fn simplify_subdivisions(&self, subdivs: &[Bar]) -> Vec<Bar> {
        let mut current = subdivs.to_vec();

        // Count how sparse the pattern is
        let rest_count = current.iter().filter(|s| s.is_silent()).count();
        let sparsity = rest_count as f32 / current.len() as f32;

        // For very sparse patterns (>80% rests), simplify more aggressively
        if sparsity > 0.8 && current.len() > 8 {
            // Try to simplify to 1/4 length for very sparse patterns
            while current.len() > 4 && current.len().is_multiple_of(4) {
                let mut can_simplify = true;
                for i in (0..current.len()).step_by(4) {
                    // Check if 3 out of 4 are rests
                    let chunk = &current[i..i.min(current.len()).min(i+4)];
                    let chunk_rests = chunk.iter().filter(|s| s.is_silent()).count();
                    if chunk_rests < 3 {
                        can_simplify = false;
                        break;
                    }
                }

                if can_simplify {
                    // Take every 4th element
                    current = current.iter().step_by(4).cloned().collect();
                } else {
                    break;
                }
            }
        }

        // Standard simplification - remove pairs where second is always rest
        while current.len().is_multiple_of(2) {
            let mut has_second = false;
            for i in (1..current.len()).step_by(2) {
                if !current[i].is_silent() {
                    has_second = true;
                    break;
                }
            }

            if has_second {
                break;
            }

            // Simplify by taking only first of each pair
            current = current.iter().step_by(2).cloned().collect();
        }

        current
    }
}
