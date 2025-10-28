// Music Theory utilities for Strudel pattern generation
// Provides scales, chord progressions, and euclidean rhythms

use std::collections::HashMap;

/// Note names in chromatic order
const NOTE_NAMES: [&str; 12] = [
    "C", "C#", "D", "D#", "E", "F", "F#", "G", "G#", "A", "A#", "B",
];

/// Scale definitions as semitone intervals from root
pub struct MusicTheory;

impl MusicTheory {
    /// Get scale intervals for a given scale type
    fn get_scale_intervals(scale_type: &str) -> Option<&'static [i32]> {
        match scale_type.to_lowercase().as_str() {
            "major" => Some(&[0, 2, 4, 5, 7, 9, 11]),
            "minor" | "aeolian" => Some(&[0, 2, 3, 5, 7, 8, 10]),
            "dorian" => Some(&[0, 2, 3, 5, 7, 9, 10]),
            "phrygian" => Some(&[0, 1, 3, 5, 7, 8, 10]),
            "lydian" => Some(&[0, 2, 4, 6, 7, 9, 11]),
            "mixolydian" => Some(&[0, 2, 4, 5, 7, 9, 10]),
            "locrian" => Some(&[0, 1, 3, 5, 6, 8, 10]),
            "pentatonic" | "pentatonic_major" => Some(&[0, 2, 4, 7, 9]),
            "pentatonic_minor" => Some(&[0, 3, 5, 7, 10]),
            "blues" => Some(&[0, 3, 5, 6, 7, 10]),
            "chromatic" => Some(&[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11]),
            "wholetone" => Some(&[0, 2, 4, 6, 8, 10]),
            "harmonic_minor" => Some(&[0, 2, 3, 5, 7, 8, 11]),
            "melodic_minor" => Some(&[0, 2, 3, 5, 7, 9, 11]),
            _ => None,
        }
    }

    /// Generate scale notes from root and scale type
    /// Returns note names (e.g., ["C", "D", "E", "F", "G", "A", "B"])
    #[allow(dead_code)]
    pub fn generate_scale(root: &str, scale_type: &str) -> Result<Vec<String>, String> {
        let root_upper = root.to_uppercase();
        let root_index = NOTE_NAMES
            .iter()
            .position(|&n| n == root_upper)
            .ok_or_else(|| format!("Invalid root note: {}", root))?;

        let intervals = Self::get_scale_intervals(scale_type)
            .ok_or_else(|| format!("Invalid scale type: {}", scale_type))?;

        let scale_notes = intervals
            .iter()
            .map(|&interval| {
                let note_index = (root_index as i32 + interval) as usize % 12;
                NOTE_NAMES[note_index].to_string()
            })
            .collect();

        Ok(scale_notes)
    }

    /// Get chord progression template for a style
    fn get_chord_progression_template(style: &str) -> Option<Vec<&'static str>> {
        match style.to_lowercase().as_str() {
            "pop" => Some(vec!["I", "V", "vi", "IV"]),
            "jazz" => Some(vec!["IIM7", "V7", "IM7"]),
            "blues" => Some(vec![
                "I7", "I7", "I7", "I7", "IV7", "IV7", "I7", "I7", "V7", "IV7", "I7", "V7",
            ]),
            "folk" => Some(vec!["I", "IV", "I", "V"]),
            "rock" => Some(vec!["I", "bVII", "IV", "I"]),
            "classical" => Some(vec!["I", "IV", "V", "I"]),
            "modal" => Some(vec!["i", "bVII", "IV", "i"]),
            "edm" => Some(vec!["i", "VI", "III", "VII"]),
            _ => None,
        }
    }

    /// Transpose a note by semitones
    pub fn transpose_note(root: &str, semitones: i32) -> Result<String, String> {
        let root_upper = root.to_uppercase();
        let root_index = NOTE_NAMES
            .iter()
            .position(|&n| n == root_upper)
            .ok_or_else(|| format!("Invalid root note: {}", root))?;

        let new_index = ((root_index as i32 + semitones).rem_euclid(12)) as usize;
        Ok(NOTE_NAMES[new_index].to_string())
    }

    /// Convert roman numeral to actual chord name
    fn roman_to_chord(root: &str, numeral: &str) -> String {
        // Chord mapping based on roman numerals
        let chord_map: HashMap<&str, (i32, &str)> = [
            ("I", (0, "")),
            ("I7", (0, "7")),
            ("i", (0, "m")),
            ("IM7", (0, "maj7")),
            ("ii", (2, "m")),
            ("IIM7", (2, "m7")),
            ("iii", (4, "m")),
            ("III", (4, "")),
            ("IV", (5, "")),
            ("IV7", (5, "7")),
            ("V", (7, "")),
            ("V7", (7, "7")),
            ("vi", (9, "m")),
            ("VI", (9, "")),
            ("VII", (11, "")),
            ("bVII", (10, "")),
        ]
        .iter()
        .cloned()
        .collect();

        if let Some(&(semitones, suffix)) = chord_map.get(numeral) {
            if let Ok(note) = Self::transpose_note(root, semitones) {
                return format!("{}{}", note, suffix);
            }
        }

        // Fallback to root
        root.to_string()
    }

    /// Generate chord progression in Strudel format
    /// Returns a string like: "C G Am F" or "Dm7 G7 Cmaj7"
    pub fn generate_chord_progression(key: &str, style: &str) -> Result<String, String> {
        let progression = Self::get_chord_progression_template(style)
            .ok_or_else(|| format!("Invalid progression style: {}", style))?;

        let chords: Vec<String> = progression
            .iter()
            .map(|&numeral| Self::roman_to_chord(key, numeral))
            .collect();

        Ok(chords.join(" "))
    }

    /// Generate Euclidean rhythm pattern
    /// Distributes `hits` evenly across `steps` using Bjorklund's algorithm
    /// Returns pattern like "1 ~ 1 ~ 1 1 ~ 1" for 5 hits in 8 steps
    pub fn generate_euclidean_rhythm(hits: usize, steps: usize) -> Result<String, String> {
        if hits > steps {
            return Err("Hits cannot exceed steps".to_string());
        }

        if steps == 0 {
            return Err("Steps must be greater than 0".to_string());
        }

        if hits == 0 {
            return Ok(vec!["~"; steps].join(" "));
        }

        // Bjorklund's algorithm (simplified)
        let mut pattern = vec![false; steps];
        let interval = steps as f32 / hits as f32;

        for i in 0..hits {
            let index = (i as f32 * interval).floor() as usize;
            if index < steps {
                pattern[index] = true;
            }
        }

        Ok(pattern
            .iter()
            .map(|&hit| if hit { "1" } else { "~" })
            .collect::<Vec<_>>()
            .join(" "))
    }

    /// Generate Euclidean rhythm as Strudel pattern
    /// Returns pattern like: s("bd").struct("1 ~ 1 ~ 1 1 ~ 1")
    pub fn generate_euclidean_pattern(
        hits: usize,
        steps: usize,
        sound: &str,
    ) -> Result<String, String> {
        let rhythm = Self::generate_euclidean_rhythm(hits, steps)?;
        Ok(format!(r#"s("{}").struct("{}")"#, sound, rhythm))
    }

    /// List available scale types
    #[allow(dead_code)]
    pub fn available_scales() -> Vec<&'static str> {
        vec![
            "major",
            "minor",
            "dorian",
            "phrygian",
            "lydian",
            "mixolydian",
            "aeolian",
            "locrian",
            "pentatonic",
            "pentatonic_minor",
            "blues",
            "chromatic",
            "wholetone",
            "harmonic_minor",
            "melodic_minor",
        ]
    }

    /// List available progression styles
    #[allow(dead_code)]
    pub fn available_progressions() -> Vec<&'static str> {
        vec![
            "pop",
            "jazz",
            "blues",
            "folk",
            "rock",
            "classical",
            "modal",
            "edm",
        ]
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_c_major_scale() {
        let scale = MusicTheory::generate_scale("C", "major").unwrap();
        assert_eq!(scale, vec!["C", "D", "E", "F", "G", "A", "B"]);
    }

    #[test]
    fn test_generate_d_dorian_scale() {
        let scale = MusicTheory::generate_scale("D", "dorian").unwrap();
        assert_eq!(scale, vec!["D", "E", "F", "G", "A", "B", "C"]);
    }

    #[test]
    fn test_generate_pop_progression() {
        let progression = MusicTheory::generate_chord_progression("C", "pop").unwrap();
        assert_eq!(progression, "C G Am F");
    }

    #[test]
    fn test_generate_jazz_progression() {
        let progression = MusicTheory::generate_chord_progression("C", "jazz").unwrap();
        assert_eq!(progression, "Dm7 G7 Cmaj7");
    }

    #[test]
    fn test_euclidean_rhythm() {
        let rhythm = MusicTheory::generate_euclidean_rhythm(5, 8).unwrap();
        // Should distribute 5 hits across 8 steps
        assert_eq!(rhythm.matches('1').count(), 5);
        assert_eq!(rhythm.split_whitespace().count(), 8);
    }

    #[test]
    fn test_euclidean_pattern() {
        let pattern = MusicTheory::generate_euclidean_pattern(3, 8, "bd").unwrap();
        assert!(pattern.contains(r#"s("bd")"#));
        assert!(pattern.contains("struct("));
    }

    #[test]
    fn test_transpose_note() {
        assert_eq!(MusicTheory::transpose_note("C", 2).unwrap(), "D");
        assert_eq!(MusicTheory::transpose_note("C", 7).unwrap(), "G");
        assert_eq!(MusicTheory::transpose_note("G", 5).unwrap(), "C");
    }

    #[test]
    fn test_invalid_scale() {
        assert!(MusicTheory::generate_scale("X", "major").is_err());
        assert!(MusicTheory::generate_scale("C", "invalid_scale").is_err());
    }

    #[test]
    fn test_euclidean_edge_cases() {
        assert!(MusicTheory::generate_euclidean_rhythm(5, 3).is_err()); // hits > steps
        assert!(MusicTheory::generate_euclidean_rhythm(0, 8).is_ok()); // 0 hits
        assert!(MusicTheory::generate_euclidean_rhythm(8, 8).is_ok()); // all hits
    }
}
