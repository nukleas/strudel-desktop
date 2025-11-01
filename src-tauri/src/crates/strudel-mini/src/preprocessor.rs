//! Preprocessor for extracting mini notation from Strudel JavaScript files
//!
//! This module handles `.strudel` files which contain JavaScript/Strudel syntax:
//! ```javascript
//! setcpm(120/4)
//! $: note(`bd sd hh cp`).sound("piano")
//! ```
//!
//! It extracts just the mini notation patterns from backticks.

/// Extract mini notation patterns from a Strudel JavaScript file
///
/// Returns a list of extracted patterns with metadata
pub fn extract_patterns(source: &str) -> Vec<ExtractedPattern> {
    let mut patterns = Vec::new();
    let mut chars = source.chars().peekable();
    let mut pos = 0;

    while let Some(&ch) = chars.peek() {
        if ch == '`' {
            // Found a backtick - extract the pattern
            chars.next(); // consume opening `
            pos += 1;

            let start = pos;
            let mut pattern = String::new();
            let mut escaped = false;

            // Read until closing backtick
            #[allow(clippy::while_let_on_iterator)]
            while let Some(ch) = chars.next() {
                pos += 1;
                if escaped {
                    pattern.push(ch);
                    escaped = false;
                } else if ch == '\\' {
                    escaped = true;
                } else if ch == '`' {
                    // Found closing backtick
                    break;
                } else {
                    pattern.push(ch);
                }
            }

            if !pattern.trim().is_empty() {
                patterns.push(ExtractedPattern {
                    pattern: pattern.trim().to_string(),
                    start_pos: start,
                    end_pos: pos - 1,
                    context: extract_context(source, start, pos),
                });
            }
        } else {
            chars.next();
            pos += 1;
        }
    }

    patterns
}

/// Represents an extracted pattern with context
#[derive(Debug, Clone)]
pub struct ExtractedPattern {
    /// The extracted mini notation pattern
    pub pattern: String,
    /// Starting position in source
    pub start_pos: usize,
    /// Ending position in source
    pub end_pos: usize,
    /// Context information (function calls, etc.)
    pub context: PatternContext,
}

/// Context extracted from the surrounding JavaScript
#[derive(Debug, Clone, Default)]
pub struct PatternContext {
    /// Function that contains this pattern (e.g., "note", "s", "sound")
    pub function: Option<String>,
    /// Method calls on this pattern (e.g., ".sound(\"piano\")")
    pub methods: Vec<MethodCall>,
    /// Track identifier (e.g., "$:")
    pub track: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MethodCall {
    pub name: String,
    pub args: Vec<String>,
}

/// Extract context from surrounding source code
fn extract_context(source: &str, start: usize, end: usize) -> PatternContext {
    let mut context = PatternContext::default();

    // Look backwards for function name (note, s, sound, etc.)
    let before_start = start.saturating_sub(50);
    if start > 0 {
        let before = &source[before_start..start.saturating_sub(1)];

        // Check for common functions - look for identifier before opening paren
        if let Some(paren_pos) = before.rfind('(') {
            // Find the start of the identifier before the paren
            let before_paren = &before[..paren_pos];
            if let Some(func_start) = before_paren.rfind(|c: char| !c.is_alphanumeric() && c != '_') {
                let func_name = before_paren[func_start + 1..].trim();
                if matches!(func_name, "note" | "s" | "sound" | "n") {
                    context.function = Some(func_name.to_string());
                }
            } else {
                // No non-alphanumeric found, the whole thing is the function name
                let func_name = before_paren.trim();
                if matches!(func_name, "note" | "s" | "sound" | "n") {
                    context.function = Some(func_name.to_string());
                }
            }
        }

        // Check for track identifier ($:)
        if before.contains("$:") {
            context.track = Some("$".to_string());
        }
    }

    // Look ahead for method calls (.sound(), .gain(), etc.)
    if end < source.len() {
        let after = &source[end + 1..(end + 100).min(source.len())];

        // Simple method extraction (doesn't handle nested calls)
        let mut remaining = after;
        while let Some(dot_pos) = remaining.find('.') {
            remaining = &remaining[dot_pos + 1..];

            if let Some(paren_pos) = remaining.find('(') {
                let method_name = &remaining[..paren_pos];

                // Find closing paren AFTER the opening paren
                let after_open = &remaining[paren_pos + 1..];
                if let Some(close_paren_offset) = after_open.find(')') {
                    let args_str = &after_open[..close_paren_offset];

                    context.methods.push(MethodCall {
                        name: method_name.trim().to_string(),
                        args: parse_simple_args(args_str),
                    });

                    // Advance past the closing paren
                    let consumed = paren_pos + 1 + close_paren_offset + 1;
                    if consumed < remaining.len() {
                        remaining = &remaining[consumed..];
                    } else {
                        break;
                    }
                } else {
                    break;
                }
            } else {
                break;
            }
        }
    }

    context
}

/// Parse simple function arguments (strings and numbers)
fn parse_simple_args(args_str: &str) -> Vec<String> {
    args_str
        .split(',')
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| {
            // Remove quotes from strings
            if (s.starts_with('"') && s.ends_with('"')) || (s.starts_with('\'') && s.ends_with('\'')) {
                s[1..s.len() - 1].to_string()
            } else {
                s.to_string()
            }
        })
        .collect()
}

/// Combine multiple extracted patterns into a single playable pattern
pub fn combine_patterns(patterns: &[ExtractedPattern], strategy: CombineStrategy) -> String {
    match strategy {
        CombineStrategy::Stack => {
            // Stack all patterns together (play simultaneously)
            patterns
                .iter()
                .map(|p| format!("({})", p.pattern))
                .collect::<Vec<_>>()
                .join(", ")
        }
        CombineStrategy::Sequence => {
            // Play patterns in sequence
            patterns
                .iter()
                .map(|p| format!("({})", p.pattern))
                .collect::<Vec<_>>()
                .join(" ")
        }
        CombineStrategy::First => {
            // Just use the first pattern
            patterns.first().map(|p| p.pattern.clone()).unwrap_or_default()
        }
        CombineStrategy::Separate => {
            // Return patterns separated by newlines (for --extract command)
            patterns
                .iter()
                .enumerate()
                .map(|(i, p)| format!("// Pattern {}\n{}", i + 1, p.pattern))
                .collect::<Vec<_>>()
                .join("\n\n")
        }
    }
}

/// Strategy for combining multiple patterns
#[derive(Debug, Clone, Copy)]
pub enum CombineStrategy {
    /// Stack patterns (play together)
    Stack,
    /// Sequence patterns (play in order)
    Sequence,
    /// Use only the first pattern
    First,
    /// Keep separate (for extraction)
    Separate,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_extract_simple_pattern() {
        let source = r#"s(`bd sd hh cp`)"#;
        let patterns = extract_patterns(source);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern, "bd sd hh cp");
        assert_eq!(patterns[0].context.function, Some("s".to_string()));
    }

    #[test]
    fn test_extract_with_methods() {
        let source = r#"note(`c e g`).sound("piano").gain(0.8)"#;
        let patterns = extract_patterns(source);

        assert_eq!(patterns.len(), 1);
        assert_eq!(patterns[0].pattern, "c e g");
        assert_eq!(patterns[0].context.function, Some("note".to_string()));
        assert_eq!(patterns[0].context.methods.len(), 2);
        assert_eq!(patterns[0].context.methods[0].name, "sound");
        assert_eq!(patterns[0].context.methods[0].args, vec!["piano"]);
    }

    #[test]
    fn test_extract_multiple_patterns() {
        let source = r#"
            s(`bd sd`)
            s(`hh*8`)
        "#;
        let patterns = extract_patterns(source);

        assert_eq!(patterns.len(), 2);
        assert_eq!(patterns[0].pattern, "bd sd");
        assert_eq!(patterns[1].pattern, "hh*8");
    }

    #[test]
    fn test_combine_stack() {
        let patterns = vec![
            ExtractedPattern {
                pattern: "bd sd".to_string(),
                start_pos: 0,
                end_pos: 5,
                context: PatternContext::default(),
            },
            ExtractedPattern {
                pattern: "hh*8".to_string(),
                start_pos: 10,
                end_pos: 14,
                context: PatternContext::default(),
            },
        ];

        let combined = combine_patterns(&patterns, CombineStrategy::Stack);
        assert_eq!(combined, "(bd sd), (hh*8)");
    }
}
