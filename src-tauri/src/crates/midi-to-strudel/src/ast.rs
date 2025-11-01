//! Abstract Syntax Tree (AST) types for Strudel patterns
//!
//! This module provides type-safe representations of Strudel patterns, enabling
//! validation, optimization, and serialization before string generation.

use serde::{Deserialize, Serialize};

/// A single bar/pattern element
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum Bar {
    /// Rest/silence (-)
    Rest,

    /// Single note (e.g., "a4")
    Note(String),

    /// Simultaneous notes - chord (e.g., [a4,c5,e5])
    Chord(Vec<String>),

    /// Polyphonic sequence (e.g., [a4 b4 c5])
    Sequence(Vec<String>),

    /// Nested subdivision (for complex rhythms)
    Subdivision(Vec<Bar>),
}

impl Bar {
    /// Convert to Strudel mini notation string
    pub fn to_strudel(&self) -> String {
        match self {
            Bar::Rest => "-".to_string(),
            Bar::Note(n) => n.clone(),
            Bar::Chord(notes) => format!("[{}]", notes.join(",")),
            Bar::Sequence(notes) => format!("[{}]", notes.join(" ")),
            Bar::Subdivision(bars) => {
                let inner: Vec<String> = bars.iter().map(|b| b.to_strudel()).collect();
                format!("[{}]", inner.join(" "))
            }
        }
    }

    /// Check if bar is empty/silent
    pub fn is_silent(&self) -> bool {
        matches!(self, Bar::Rest)
    }

    /// Get all notes in this bar (for analysis)
    pub fn notes(&self) -> Vec<String> {
        match self {
            Bar::Rest => vec![],
            Bar::Note(n) => vec![n.clone()],
            Bar::Chord(notes) | Bar::Sequence(notes) => notes.clone(),
            Bar::Subdivision(bars) => bars.iter().flat_map(|b| b.notes()).collect(),
        }
    }
}

/// Gain/velocity value - single or pattern
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub enum ModifierValue {
    Single(f32),
    Pattern(Vec<f32>),
}

impl ModifierValue {
    pub fn to_strudel(&self) -> String {
        match self {
            ModifierValue::Single(v) => format!("{:.2}", v),
            ModifierValue::Pattern(vals) => {
                let formatted: Vec<String> = vals.iter().map(|v| format!("{:.2}", v)).collect();
                format!("\"<{}>\"", formatted.join(" "))
            }
        }
    }
}

/// A complete pattern with modifiers
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub bars: Vec<Bar>,
    pub is_drum: bool,

    // Modifiers
    pub sound: Option<String>,
    pub gain: Option<ModifierValue>,
    pub pan: Option<f32>,
    pub sustain: Option<f32>,
}

impl Pattern {
    /// Convert to Strudel string output
    pub fn to_strudel(&self, compact: bool) -> String {
        let mut output = String::new();

        // Build pattern body
        let bars_str: Vec<String> = if compact {
            compress_bars(&self.bars)
                .iter()
                .map(|b| b.to_strudel())
                .collect()
        } else {
            self.bars.iter().map(|b| b.to_strudel()).collect()
        };

        if self.is_drum {
            output.push_str(&format!("s(`<{}>`)", bars_str.join(" ")));
        } else {
            output.push_str(&format!("note(`<{}>`)", bars_str.join(" ")));
        }

        // Add modifiers
        if let Some(sound) = &self.sound {
            output.push_str(&format!(".sound(\"{}\")", sound));
        }

        if let Some(sustain) = self.sustain {
            if !(0.95..=1.05).contains(&sustain) && sustain >= 0.05 {
                output.push_str(&format!(".sustain({:.2})", sustain));
            }
        }

        if let Some(gain) = &self.gain {
            output.push_str(&format!(".gain({})", gain.to_strudel()));
        }

        if let Some(pan) = self.pan {
            if !(0.45..=0.55).contains(&pan) {
                output.push_str(&format!(".pan({:.2})", pan));
            }
        }

        output
    }

    /// Validate pattern structure
    pub fn validate(&self) -> Result<(), String> {
        for (i, bar) in self.bars.iter().enumerate() {
            match bar {
                Bar::Chord(notes) if notes.is_empty() => {
                    return Err(format!("Empty chord at bar {}", i));
                }
                Bar::Sequence(notes) if notes.is_empty() => {
                    return Err(format!("Empty sequence at bar {}", i));
                }
                Bar::Note(n) if n.is_empty() => {
                    return Err(format!("Empty note at bar {}", i));
                }
                _ => {}
            }
        }
        Ok(())
    }

    /// Optimize pattern (merge rests, simplify, etc.)
    pub fn optimize(&mut self) {
        // Remove trailing rests
        while self.bars.last().map(|b| b.is_silent()).unwrap_or(false) {
            self.bars.pop();
        }

        // Simplify single-note chords and sequences
        self.bars = self
            .bars
            .iter()
            .map(|b| match b {
                Bar::Chord(notes) if notes.len() == 1 => Bar::Note(notes[0].clone()),
                Bar::Sequence(notes) if notes.len() == 1 => Bar::Note(notes[0].clone()),
                other => other.clone(),
            })
            .collect();
    }
}

/// Compress consecutive identical bars using replication operator (!)
fn compress_bars(bars: &[Bar]) -> Vec<Bar> {
    if bars.is_empty() {
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

        // For now, just return the bars as-is
        // TODO: Implement replication operator support in Bar enum
        for _ in 0..count {
            result.push(current.clone());
        }

        i += count;
    }

    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bar_rest() {
        assert_eq!(Bar::Rest.to_strudel(), "-");
        assert!(Bar::Rest.is_silent());
        assert!(Bar::Rest.notes().is_empty());
    }

    #[test]
    fn test_bar_note() {
        let bar = Bar::Note("a4".to_string());
        assert_eq!(bar.to_strudel(), "a4");
        assert!(!bar.is_silent());
        assert_eq!(bar.notes(), vec!["a4"]);
    }

    #[test]
    fn test_bar_chord() {
        let chord = Bar::Chord(vec!["a4".into(), "c5".into(), "e5".into()]);
        assert_eq!(chord.to_strudel(), "[a4,c5,e5]");
        assert!(!chord.is_silent());
        assert_eq!(chord.notes(), vec!["a4", "c5", "e5"]);
    }

    #[test]
    fn test_bar_sequence() {
        let seq = Bar::Sequence(vec!["a4".into(), "b4".into()]);
        assert_eq!(seq.to_strudel(), "[a4 b4]");
        assert!(!seq.is_silent());
        assert_eq!(seq.notes(), vec!["a4", "b4"]);
    }

    #[test]
    fn test_bar_subdivision() {
        let subdiv = Bar::Subdivision(vec![
            Bar::Note("a4".into()),
            Bar::Rest,
            Bar::Chord(vec!["c5".into(), "e5".into()]),
        ]);
        assert_eq!(subdiv.to_strudel(), "[a4 - [c5,e5]]");
        assert!(!subdiv.is_silent());
        assert_eq!(subdiv.notes(), vec!["a4", "c5", "e5"]);
    }

    #[test]
    fn test_modifier_value_single() {
        let mv = ModifierValue::Single(0.75);
        assert_eq!(mv.to_strudel(), "0.75");
    }

    #[test]
    fn test_modifier_value_pattern() {
        let mv = ModifierValue::Pattern(vec![0.5, 0.7, 0.9]);
        assert_eq!(mv.to_strudel(), "\"<0.50 0.70 0.90>\"");
    }

    #[test]
    fn test_pattern_melodic_basic() {
        let pattern = Pattern {
            bars: vec![Bar::Note("a4".into()), Bar::Note("b4".into())],
            is_drum: false,
            sound: Some("piano".into()),
            gain: Some(ModifierValue::Single(0.8)),
            pan: Some(0.6),
            sustain: Some(0.5),
        };

        let output = pattern.to_strudel(false);
        assert!(output.contains("note(`<a4 b4>`"));
        assert!(output.contains(".sound(\"piano\")"));
        assert!(output.contains(".sustain(0.50)"));
        assert!(output.contains(".gain(0.80)"));
        assert!(output.contains(".pan(0.60)"));
    }

    #[test]
    fn test_pattern_drum_basic() {
        let pattern = Pattern {
            bars: vec![Bar::Note("bd".into()), Bar::Rest, Bar::Note("sd".into())],
            is_drum: true,
            sound: None,
            gain: Some(ModifierValue::Single(0.7)),
            pan: None,
            sustain: None,
        };

        let output = pattern.to_strudel(false);
        assert!(output.contains("s(`<bd - sd>`"));
        assert!(output.contains(".gain(0.70)"));
        assert!(!output.contains(".sound"));
        assert!(!output.contains(".sustain"));
        assert!(!output.contains(".pan"));
    }

    #[test]
    fn test_pattern_gain_pattern() {
        let pattern = Pattern {
            bars: vec![Bar::Note("a4".into())],
            is_drum: false,
            sound: Some("sine".into()),
            gain: Some(ModifierValue::Pattern(vec![0.5, 0.7, 0.9])),
            pan: None,
            sustain: None,
        };

        let output = pattern.to_strudel(false);
        assert!(output.contains(".gain(\"<0.50 0.70 0.90>\")"));
    }

    #[test]
    fn test_pattern_skip_centered_pan() {
        let pattern = Pattern {
            bars: vec![Bar::Note("a4".into())],
            is_drum: false,
            sound: Some("piano".into()),
            gain: None,
            pan: Some(0.5), // Centered - should be skipped
            sustain: None,
        };

        let output = pattern.to_strudel(false);
        assert!(!output.contains(".pan"));
    }

    #[test]
    fn test_pattern_skip_default_sustain() {
        let pattern = Pattern {
            bars: vec![Bar::Note("a4".into())],
            is_drum: false,
            sound: Some("piano".into()),
            gain: None,
            pan: None,
            sustain: Some(1.0), // Default - should be skipped
        };

        let output = pattern.to_strudel(false);
        assert!(!output.contains(".sustain"));
    }

    #[test]
    fn test_pattern_validation_empty_chord() {
        let pattern = Pattern {
            bars: vec![Bar::Chord(vec![])],
            is_drum: false,
            sound: None,
            gain: None,
            pan: None,
            sustain: None,
        };

        let result = pattern.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty chord"));
    }

    #[test]
    fn test_pattern_validation_empty_sequence() {
        let pattern = Pattern {
            bars: vec![Bar::Sequence(vec![])],
            is_drum: false,
            sound: None,
            gain: None,
            pan: None,
            sustain: None,
        };

        let result = pattern.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty sequence"));
    }

    #[test]
    fn test_pattern_validation_empty_note() {
        let pattern = Pattern {
            bars: vec![Bar::Note("".into())],
            is_drum: false,
            sound: None,
            gain: None,
            pan: None,
            sustain: None,
        };

        let result = pattern.validate();
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Empty note"));
    }

    #[test]
    fn test_pattern_optimize_trailing_rests() {
        let mut pattern = Pattern {
            bars: vec![
                Bar::Note("a4".into()),
                Bar::Note("b4".into()),
                Bar::Rest,
                Bar::Rest,
            ],
            is_drum: false,
            sound: None,
            gain: None,
            pan: None,
            sustain: None,
        };

        pattern.optimize();
        assert_eq!(pattern.bars.len(), 2);
        assert_eq!(pattern.bars[0], Bar::Note("a4".into()));
        assert_eq!(pattern.bars[1], Bar::Note("b4".into()));
    }

    #[test]
    fn test_pattern_optimize_single_note_chord() {
        let mut pattern = Pattern {
            bars: vec![Bar::Chord(vec!["a4".into()])],
            is_drum: false,
            sound: None,
            gain: None,
            pan: None,
            sustain: None,
        };

        pattern.optimize();
        assert_eq!(pattern.bars.len(), 1);
        assert_eq!(pattern.bars[0], Bar::Note("a4".into()));
    }

    #[test]
    fn test_pattern_optimize_single_note_sequence() {
        let mut pattern = Pattern {
            bars: vec![Bar::Sequence(vec!["a4".into()])],
            is_drum: false,
            sound: None,
            gain: None,
            pan: None,
            sustain: None,
        };

        pattern.optimize();
        assert_eq!(pattern.bars.len(), 1);
        assert_eq!(pattern.bars[0], Bar::Note("a4".into()));
    }

    #[test]
    fn test_compress_bars_identical() {
        let bars = vec![
            Bar::Note("a4".into()),
            Bar::Note("a4".into()),
            Bar::Note("a4".into()),
        ];

        let compressed = compress_bars(&bars);
        // For now, compress_bars doesn't actually compress, just returns a copy
        assert_eq!(compressed.len(), 3);
    }

    #[test]
    fn test_serialize_deserialize() {
        let pattern = Pattern {
            bars: vec![
                Bar::Note("a4".into()),
                Bar::Chord(vec!["c5".into(), "e5".into()]),
            ],
            is_drum: false,
            sound: Some("piano".into()),
            gain: Some(ModifierValue::Single(0.8)),
            pan: Some(0.6),
            sustain: Some(0.5),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&pattern).unwrap();
        assert!(json.contains("a4"));
        assert!(json.contains("piano"));

        // Deserialize back
        let deserialized: Pattern = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.bars.len(), 2);
        assert_eq!(deserialized.sound, Some("piano".into()));
    }
}
