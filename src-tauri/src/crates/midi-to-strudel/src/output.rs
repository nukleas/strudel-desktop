use crate::ast::{Bar, ModifierValue, Pattern};
use crate::instruments::get_track_sound;
use crate::track::ProcessedTrack;

pub struct OutputFormatter {
    tab_size: usize,
    compact: bool,
}

impl OutputFormatter {
    pub fn new(tab_size: usize, compact: bool) -> Self {
        Self { tab_size, compact }
    }

    /// Build JSON output of the AST
    pub fn build_output_json(&self, tracks: &[ProcessedTrack], bpm: f64) -> String {
        #[derive(serde::Serialize)]
        struct JsonOutput {
            bpm: f64,
            tracks: Vec<Pattern>,
        }

        // Convert all tracks to Pattern AST
        let patterns: Vec<Pattern> = tracks
            .iter()
            .map(|track| self.track_to_pattern(track))
            .collect();

        let output = JsonOutput {
            bpm,
            tracks: patterns,
        };

        serde_json::to_string_pretty(&output).unwrap_or_else(|e| {
            eprintln!("Error serializing to JSON: {}", e);
            "{}".to_string()
        })
    }

    /// Convert ProcessedTrack to Pattern AST
    fn track_to_pattern(&self, track: &ProcessedTrack) -> Pattern {
        // Convert gain values to ModifierValue
        let gain = self.format_gain_to_modifier(&track.gains);

        // Convert sustain values to single value
        let sustain = if !track.is_drum {
            self.format_sustain_to_value(&track.sustains)
        } else {
            None
        };

        // Get sound name (only for melodic tracks - drum tracks use s() which already specifies sound)
        let sound = if !track.is_drum {
            Some(get_track_sound(track.name.as_deref(), track.program))
        } else {
            None
        };

        Pattern {
            bars: track.bars.clone(),
            is_drum: track.is_drum,
            sound,
            gain,
            pan: track.pan,
            sustain,
        }
    }

    /// Format gain values - single value if low variance, pattern if high variance
    fn format_gain_to_modifier(&self, gains: &[f32]) -> Option<ModifierValue> {
        if gains.is_empty() {
            return None;
        }

        // Calculate average gain from non-zero bars only
        let non_zero_gains: Vec<f32> = gains.iter().copied().filter(|&g| g > 0.0).collect();

        if non_zero_gains.is_empty() {
            return None;
        }

        let avg_gain: f32 = non_zero_gains.iter().sum::<f32>() / non_zero_gains.len() as f32;

        // Omit gain if it's close to default (1.0) or very low (close to 0)
        if !(0.1..=0.95).contains(&avg_gain) {
            return None;
        }

        // Calculate variance to decide between single value vs pattern
        let variance: f32 = non_zero_gains
            .iter()
            .map(|&g| (g - avg_gain).powi(2))
            .sum::<f32>()
            / non_zero_gains.len() as f32;

        // If low variance, use single average value
        if variance < 0.02 {
            return Some(ModifierValue::Single(avg_gain));
        }

        // High variance - generate pattern by grouping into ~6 sections
        let sections = 6.min(non_zero_gains.len());
        let chunk_size = (non_zero_gains.len() as f32 / sections as f32).ceil() as usize;

        let mut section_gains = Vec::new();
        for i in 0..sections {
            let start = i * chunk_size;
            let end = ((i + 1) * chunk_size).min(non_zero_gains.len());
            if start < non_zero_gains.len() {
                let chunk = &non_zero_gains[start..end];
                let chunk_avg = chunk.iter().sum::<f32>() / chunk.len() as f32;
                section_gains.push(chunk_avg);
            }
        }

        // Check if the pattern values are too similar - if so, just use average
        if section_gains.len() > 1 {
            let pattern_avg = section_gains.iter().sum::<f32>() / section_gains.len() as f32;
            let pattern_variance = section_gains
                .iter()
                .map(|&g| (g - pattern_avg).powi(2))
                .sum::<f32>()
                / section_gains.len() as f32;

            // If pattern variance is low, the pattern isn't adding value
            if pattern_variance < 0.005 {
                return Some(ModifierValue::Single(avg_gain));
            }
        }

        Some(ModifierValue::Pattern(section_gains))
    }

    /// Format sustain values using 75th percentile
    fn format_sustain_to_value(&self, sustains: &[f32]) -> Option<f32> {
        if sustains.is_empty() {
            return None;
        }

        // Get non-zero sustains and sort for percentile calculation
        let mut non_zero_sustains: Vec<f32> =
            sustains.iter().copied().filter(|&s| s > 0.0).collect();

        if non_zero_sustains.is_empty() {
            return None;
        }

        non_zero_sustains.sort_by(|a, b| a.partial_cmp(b).unwrap());

        // Use 75th percentile
        let p75_index = (non_zero_sustains.len() * 3) / 4;
        let p75_sustain = non_zero_sustains[p75_index.min(non_zero_sustains.len() - 1)];

        // Omit sustain if it's very short (< 0.05) or close to default (0.95-1.05)
        if p75_sustain < 0.05 || (0.95..=1.05).contains(&p75_sustain) {
            return None;
        }

        Some(p75_sustain)
    }

    pub fn build_output(&self, tracks: &[ProcessedTrack], bpm: f64) -> String {
        let mut output = Vec::new();

        // Set CPM (cycles per minute)
        output.push(format!("setcpm({}/4)\n", bpm as i32));

        for (idx, track) in tracks.iter().enumerate() {
            // Add track name as comment if available
            if let Some(name) = &track.name {
                output.push(format!("// Track {}: {}", idx + 1, name));
            }

            // Convert track to Pattern AST
            let pattern = self.track_to_pattern(track);

            // Validate pattern (optional - log warnings)
            if let Err(e) = pattern.validate() {
                eprintln!("Warning: Track {} validation error: {}", idx + 1, e);
            }

            // Convert pattern to Strudel code using AST
            let pattern_str = self.format_pattern_with_indent(&pattern);

            output.push(format!("$: {}\n", pattern_str));
        }

        output.join("\n")
    }

    /// Format a pattern with proper indentation for multi-line output
    fn format_pattern_with_indent(&self, pattern: &Pattern) -> String {
        // Get bars as strings (with compression if compact mode)
        let bars_str: Vec<String> = if self.compact {
            self.compress_bars(&pattern.bars)
        } else {
            pattern.bars.iter().map(|b| b.to_strudel()).collect()
        };

        // Build the pattern start
        let mut output = Vec::new();
        if pattern.is_drum {
            output.push("s(`<".to_string());
        } else {
            output.push("note(`<".to_string());
        }

        // Group bars into chunks of 4 for readability
        for chunk_start in (0..bars_str.len()).step_by(4) {
            let chunk_end = (chunk_start + 4).min(bars_str.len());
            let chunk = &bars_str[chunk_start..chunk_end];

            let indent = self.get_indent(2);
            output.push(format!("{}{}", indent, chunk.join(" ")));
        }

        // Add closing bracket and modifiers on the last line
        let last_idx = output.len() - 1;
        output[last_idx].push_str(">`)");

        // Add modifiers using Pattern's logic
        // Sound
        if let Some(sound) = &pattern.sound {
            output[last_idx].push_str(&format!(".sound(\"{}\")", sound));
        }

        // Sustain
        if let Some(sustain) = pattern.sustain {
            if !(0.95..=1.05).contains(&sustain) && sustain >= 0.05 {
                output[last_idx].push_str(&format!(".sustain({:.2})", sustain));
            }
        }

        // Gain
        if let Some(gain) = &pattern.gain {
            output[last_idx].push_str(&format!(".gain({})", gain.to_strudel()));
        }

        // Pan
        if let Some(pan) = pattern.pan {
            if !(0.45..=0.55).contains(&pan) {
                output[last_idx].push_str(&format!(".pan({:.2})", pan));
            }
        }

        output.join("\n")
    }

    fn get_indent(&self, tabs: usize) -> String {
        " ".repeat(self.tab_size * tabs)
    }

    /// Compress consecutive identical bars using replication operator (!)
    fn compress_bars(&self, bars: &[Bar]) -> Vec<String> {
        if !self.compact || bars.is_empty() {
            return bars.iter().map(|b| b.to_strudel()).collect();
        }

        let mut result = Vec::new();
        let mut i = 0;

        while i < bars.len() {
            let current = &bars[i];
            let mut count = 1;

            // Count consecutive identical bars
            while i + count < bars.len() && &bars[i + count] == current {
                count += 1;
            }

            // Use replication operator if count > 1
            if count > 1 {
                result.push(format!("{}!{}", current.to_strudel(), count));
            } else {
                result.push(current.to_strudel());
            }

            i += count;
        }

        result
    }

}
