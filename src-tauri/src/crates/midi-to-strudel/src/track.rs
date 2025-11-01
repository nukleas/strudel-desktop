use std::collections::HashMap;

use crate::drums::{gm_drum_to_sample, note_name_to_midi_num};
use crate::midi::{NoteEvent, TrackInfo};

#[derive(Debug, Clone)]
pub struct ProcessedTrack {
    pub bars: Vec<String>,
    pub gains: Vec<f32>,  // Gain value for each bar (0.0 to 1.0)
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
}

impl TrackBuilder {
    pub fn new(
        cycle_len: f64,
        bar_limit: usize,
        flat_sequences: bool,
        notes_per_bar: usize,
    ) -> Self {
        Self {
            cycle_len,
            bar_limit,
            flat_sequences,
            notes_per_bar,
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

            // Check if this is a drum track (channel 10 = index 9)
            let is_drum = info.channel == Some(9);

            let max_time = adjusted.iter().map(|e| e.time_sec).fold(0.0, f64::max);
            let num_cycles = ((max_time / self.cycle_len) as usize + 1).min(
                if self.bar_limit > 0 {
                    self.bar_limit
                } else {
                    usize::MAX
                },
            );

            let mut bars = Vec::new();
            let mut gains = Vec::new();

            for cycle in 0..num_cycles {
                let start = cycle as f64 * self.cycle_len;
                let end = start + self.cycle_len;

                let notes_in_cycle: Vec<_> = adjusted
                    .iter()
                    .filter(|e| e.time_sec >= start && e.time_sec < end)
                    .cloned()
                    .collect();

                if notes_in_cycle.is_empty() {
                    bars.push("-".to_string()); // Use - for rests (same as Python version)
                    gains.push(0.0); // No gain for empty bars
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
            }

            if !bars.is_empty() {
                tracks.push(ProcessedTrack {
                    bars,
                    gains,
                    channel: info.channel,
                    program: info.program,
                    name: info.name.clone(),
                    is_drum,
                });
            }
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
                    }
                } else {
                    event.clone()
                }
            })
            .collect()
    }

    fn get_drum_bar(&self, events: &[NoteEvent], start: f64) -> String {
        // Use subdivision logic like melodic tracks for proper timing
        let mut subdivisions = vec!["-".to_string(); self.notes_per_bar];
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
                    samples[0].clone()
                } else {
                    // Multiple drums at same time - use commas (layered)
                    format!("[{}]", samples.join(","))
                };
            }
        }

        // Check if all are rests
        if subdivisions.iter().all(|s| s == "-") {
            return "-".to_string();
        }

        // Simplify subdivisions
        let simplified = self.simplify_subdivisions(&subdivisions);

        if simplified.len() == 1 {
            simplified[0].clone()
        } else {
            format!("[{}]", simplified.join(" "))
        }
    }

    fn get_flat_mode_bar(&self, events: &[NoteEvent]) -> String {
        let mut sorted = events.to_vec();
        sorted.sort_by(|a, b| a.time_sec.partial_cmp(&b.time_sec).unwrap());

        let notes: Vec<_> = sorted.iter().map(|e| e.note.as_str()).collect();

        if notes.len() == 1 {
            notes[0].to_string()
        } else {
            format!("[{}]", notes.join(" "))
        }
    }

    fn get_poly_mode_bar(&self, events: &[NoteEvent], cycle_start: f64) -> String {
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
            return "-".to_string();
        }

        // Build subdivisions array
        let mut subdivisions = vec!["-".to_string(); self.notes_per_bar];

        for (idx, notes) in time_groups {
            if idx < self.notes_per_bar {
                subdivisions[idx] = if notes.len() == 1 {
                    notes[0].clone()
                } else {
                    format!("[{}]", notes.join(","))
                };
            }
        }

        // Check if all are rests
        if subdivisions.iter().all(|s| s == "-") {
            return "-".to_string();
        }

        // Simplify subdivisions
        let simplified = self.simplify_subdivisions(&subdivisions);

        if simplified.len() == 1 {
            simplified[0].clone()
        } else {
            format!("[{}]", simplified.join(" "))
        }
    }

    fn quantize_time(&self, timestamp: f64, cycle_start: f64) -> f64 {
        let rel_time = (timestamp - cycle_start) / self.cycle_len;
        let quantized = (rel_time * self.notes_per_bar as f64).round() / self.notes_per_bar as f64;
        quantized.min(1.0 - 1e-9)
    }

    fn simplify_subdivisions(&self, subdivs: &[String]) -> Vec<String> {
        let mut current = subdivs.to_vec();

        while current.len().is_multiple_of(2) {
            // Check if any second element in pairs is not a rest
            let mut has_second = false;
            for i in (1..current.len()).step_by(2) {
                if current[i] != "-" {
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
