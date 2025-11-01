use crate::instruments::gm_program_to_sound;
use crate::track::ProcessedTrack;

pub struct OutputFormatter {
    tab_size: usize,
    compact: bool,
}

impl OutputFormatter {
    pub fn new(tab_size: usize, compact: bool) -> Self {
        Self { tab_size, compact }
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

            if track.is_drum {
                // Drum tracks use s() syntax with sample names spread across cycles
                output.push("$: s(`<".to_string());

                // Compress bars if compact mode is enabled
                let bars = self.compress_bars(&track.bars);

                // Group bars into chunks of 4
                for chunk_start in (0..bars.len()).step_by(4) {
                    let chunk_end = (chunk_start + 4).min(bars.len());
                    let chunk = &bars[chunk_start..chunk_end];

                    let indent = self.get_indent(2);
                    output.push(format!("{}{}", indent, chunk.join(" ")));
                }

                // Add closing bracket
                let last_idx = output.len() - 1;
                output[last_idx].push_str(">`)");

                // Add gain pattern if there are velocity variations
                let gain_pattern = self.format_gain_pattern(&track.gains);
                if !gain_pattern.is_empty() {
                    output[last_idx].push_str(&format!(".gain(\"{}\")", gain_pattern));
                }

                output[last_idx].push('\n');
            } else {
                // Melodic tracks use note() syntax
                output.push("$: note(`<".to_string());

                // Compress bars if compact mode is enabled
                let bars = self.compress_bars(&track.bars);

                // Group bars into chunks of 4
                for chunk_start in (0..bars.len()).step_by(4) {
                    let chunk_end = (chunk_start + 4).min(bars.len());
                    let chunk = &bars[chunk_start..chunk_end];

                    let indent = self.get_indent(2);
                    output.push(format!("{}{}", indent, chunk.join(" ")));
                }

                // Add closing bracket
                let last_idx = output.len() - 1;
                output[last_idx].push_str(">`)");

                // Add sound/instrument based on program change
                if let Some(program) = track.program {
                    let sound_name = gm_program_to_sound(program);
                    output[last_idx].push_str(&format!(".sound(\"{}\")", sound_name));
                } else {
                    // No program change found, use default
                    output[last_idx].push_str(".sound(\"piano\")");
                }

                // Add gain pattern if there are velocity variations
                let gain_pattern = self.format_gain_pattern(&track.gains);
                if !gain_pattern.is_empty() {
                    output[last_idx].push_str(&format!(".gain(\"{}\")", gain_pattern));
                }

                output[last_idx].push('\n');
            }
        }

        output.join("\n")
    }

    fn get_indent(&self, tabs: usize) -> String {
        " ".repeat(self.tab_size * tabs)
    }

    /// Compress consecutive identical bars using replication operator (!)
    fn compress_bars(&self, bars: &[String]) -> Vec<String> {
        if !self.compact || bars.is_empty() {
            return bars.to_vec();
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
                result.push(format!("{}!{}", current, count));
            } else {
                result.push(current.clone());
            }

            i += count;
        }

        result
    }

    /// Format gain values as a single averaged value
    /// Returns empty string if average is very low (< 0.1) or very high (> 0.95)
    fn format_gain_pattern(&self, gains: &[f32]) -> String {
        if gains.is_empty() {
            return String::new();
        }

        // Calculate average gain
        let avg_gain: f32 = gains.iter().sum::<f32>() / gains.len() as f32;

        // Omit gain if it's close to default (1.0) or very low (close to 0)
        if !(0.1..=0.95).contains(&avg_gain) {
            return String::new();
        }

        // Return single averaged value
        format!("{:.2}", avg_gain)
    }
}
