/// Evaluator for mini notation AST
///
/// Converts parsed AST nodes into executable strudel-core patterns
use crate::ast::*;
use crate::error::{ParseError, Result};
use strudel_core::{choose, choose_weighted, fastcat, polymeter, pure, silence, slowcat, stack, Fraction, Pattern, State, TimeSpan, Value};

/// Evaluate an AST node into a Pattern
pub fn evaluate(ast: &Ast) -> Result<Pattern> {
    match ast {
        Ast::Atom(atom) => eval_atom(atom),
        Ast::Pattern(pattern) => eval_pattern(pattern),
        Ast::Element(element) => eval_element(element),
        Ast::Operator(op) => eval_operator(op),
        Ast::Command(_) => Ok(silence()), // Commands don't produce patterns
    }
}

/// Evaluate an atom into a constant pattern
fn eval_atom(atom: &AtomNode) -> Result<Pattern> {
    match &atom.value {
        AtomValue::Number(n) => Ok(pure(Value::Number(*n)).split_queries()),
        AtomValue::String(s) => Ok(pure(Value::String(s.clone())).split_queries()),
        AtomValue::Silence => Ok(silence()),
    }
}

/// Evaluate a pattern node with alignment
fn eval_pattern(pattern: &PatternNode) -> Result<Pattern> {
    // Handle empty patterns
    if pattern.children.is_empty() {
        return Ok(silence());
    }

    // Apply alignment
    let result = match pattern.alignment {
        Alignment::Fastcat => {
            let child_patterns: Result<Vec<_>> = pattern
                .children
                .iter()
                .map(evaluate)
                .collect();
            fastcat(child_patterns?)
        }
        Alignment::Stack => {
            let child_patterns: Result<Vec<_>> = pattern
                .children
                .iter()
                .map(evaluate)
                .collect();
            stack(child_patterns?)
        }
        Alignment::PolymeterSlowcat => {
            let child_patterns: Result<Vec<_>> = pattern
                .children
                .iter()
                .map(evaluate)
                .collect();
            slowcat(child_patterns?)
        }
        Alignment::Rand => {
            let seed = pattern.seed.unwrap_or(0);

            // Extract weights and evaluate patterns
            let patterns_with_weights: Result<Vec<_>> = pattern
                .children
                .iter()
                .map(|child| {
                    let weight = extract_weight(child);
                    let pat = evaluate(child)?;
                    Ok((pat, weight))
                })
                .collect();

            let patterns_with_weights = patterns_with_weights?;

            // Check if any weights are non-default (> 1.0)
            let has_weights = patterns_with_weights.iter().any(|(_, w)| *w > 1.0);

            if has_weights {
                choose_weighted(patterns_with_weights, seed)
            } else {
                // All weights are 1.0, use unweighted choose
                let child_patterns = patterns_with_weights.into_iter().map(|(p, _)| p).collect();
                choose(child_patterns, seed)
            }
        }
        Alignment::Polymeter => {
            let child_patterns: Result<Vec<_>> = pattern
                .children
                .iter()
                .map(evaluate)
                .collect();
            polymeter(child_patterns?)
        }
        Alignment::Feet => {
            // Feet/dot operator: like fastcat but children are treated as complete sub-patterns
            // "a . b c" is equivalent to "[a] [b c]"
            // Each dot-separated section gets equal time in the cycle
            let child_patterns: Result<Vec<_>> = pattern
                .children
                .iter()
                .map(evaluate)
                .collect();
            fastcat(child_patterns?)
        }
    };

    Ok(result)
}

/// Evaluate an element with operators applied
fn eval_element(element: &ElementNode) -> Result<Pattern> {
    let mut pattern = evaluate(&element.source)?;

    // Apply operators in sequence
    // Note: reps is redundant with Replicate operator, so we ignore it here
    for op in &element.ops {
        pattern = apply_slice_op(pattern, op)?;
    }

    // TODO: Apply weight
    // Weight is used for weighted random choice, not implemented yet

    Ok(pattern)
}

/// Apply a slice operator to a pattern
fn apply_slice_op(pattern: Pattern, op: &SliceOp) -> Result<Pattern> {
    match op {
        SliceOp::Stretch { amount, op_type } => {
            // For now, we only handle numeric stretch amounts
            // Pattern-based stretch would require applicative pattern operations
            let amount_val = extract_number(amount)?;
            match op_type {
                StretchType::Fast => Ok(pattern.fast(amount_val)),
                StretchType::Slow => Ok(pattern.slow(amount_val)),
            }
        }
        SliceOp::Replicate { amount } => Ok(pattern.replicate(*amount)),
        SliceOp::Bjorklund {
            pulse,
            step,
            rotation,
        } => {
            let pulse_val = extract_number(pulse)? as usize;
            let step_val = extract_number(step)? as usize;
            let rotation_val = rotation
                .as_ref()
                .map(|r| extract_number(r).map(|v| v as usize))
                .transpose()?;

            Ok(pattern.euclid(pulse_val, step_val, rotation_val))
        }
        SliceOp::DegradeBy { amount, seed } => {
            let degrade_amount = amount.unwrap_or(0.5);
            Ok(pattern.degrade_by(degrade_amount, *seed))
        }
        SliceOp::Tail { element } => {
            // Tail operator (:) appends another pattern to this one
            // "a:b" means [a, b] - concatenate patterns within a cycle
            let tail_pattern = evaluate(element)?;
            Ok(pattern.tail(tail_pattern))
        }
        SliceOp::Range { element } => {
            // Range operator: start..end expands to [start, start+1, ..., end]
            // Extract start from the source pattern (must be a number)
            let start = extract_number_from_pattern(&pattern)?;
            let end = extract_number(element)?;

            // Create a sequence of numbers from start to end (inclusive)
            let mut numbers = Vec::new();
            if start <= end {
                let mut current = start;
                while current <= end {
                    numbers.push(current);
                    current += 1.0;
                }
            } else {
                // Descending range
                let mut current = start;
                while current >= end {
                    numbers.push(current);
                    current -= 1.0;
                }
            }

            // Convert to patterns and fastcat them
            let patterns: Vec<Pattern> = numbers
                .into_iter()
                .map(|n| pure(Value::Number(n)).split_queries())
                .collect();

            Ok(fastcat(patterns))
        }
    }
}

/// Evaluate top-level operators
fn eval_operator(op: &OperatorNode) -> Result<Pattern> {
    let source_pattern = evaluate(&op.source)?;

    match op.op_type {
        OperatorType::Fast => match &op.args {
            OperatorArgs::Number(n) => Ok(source_pattern.fast(*n)),
            _ => Err(ParseError::custom(
                "Fast operator requires numeric argument",
                Some(op.span),
            )),
        },
        OperatorType::Slow => match &op.args {
            OperatorArgs::Number(n) => Ok(source_pattern.slow(*n)),
            _ => Err(ParseError::custom(
                "Slow operator requires numeric argument",
                Some(op.span),
            )),
        },
        OperatorType::Bjorklund => match &op.args {
            OperatorArgs::Bjorklund {
                pulse,
                step,
                rotation,
            } => Ok(source_pattern.euclid(
                *pulse as usize,
                *step as usize,
                rotation.map(|r| r as usize),
            )),
            _ => Err(ParseError::custom(
                "Bjorklund operator requires pulse, step, and optional rotation",
                Some(op.span),
            )),
        },
        OperatorType::Scale => match &op.args {
            OperatorArgs::String(scale_name) => Ok(source_pattern.scale(scale_name.clone())),
            _ => Err(ParseError::custom(
                "Scale operator requires string argument",
                Some(op.span),
            )),
        },
        OperatorType::Struct => match &op.args {
            OperatorArgs::Pattern(structure_ast) => {
                // Evaluate the structure pattern
                let structure_pattern = evaluate(structure_ast)?;
                Ok(source_pattern.struct_(structure_pattern))
            }
            _ => Err(ParseError::custom(
                "Struct operator requires pattern argument",
                Some(op.span),
            )),
        },
        OperatorType::Shift => match &op.args {
            OperatorArgs::Number(n) => Ok(source_pattern.shift(*n)),
            _ => Err(ParseError::custom(
                "Shift operator requires numeric argument",
                Some(op.span),
            )),
        },
        OperatorType::Target => match &op.args {
            OperatorArgs::String(target_name) => Ok(source_pattern.target(target_name.clone())),
            _ => Err(ParseError::custom(
                "Target operator requires string argument",
                Some(op.span),
            )),
        },
    }
}

/// Helper function to extract a number from an AST node
fn extract_number(ast: &Ast) -> Result<f64> {
    match ast {
        Ast::Atom(atom) => match &atom.value {
            AtomValue::Number(n) => Ok(*n),
            _ => Err(ParseError::custom(
                "Expected number, got non-numeric atom",
                Some(atom.span),
            )),
        },
        // Handle Element nodes that wrap simple atoms
        Ast::Element(element) if element.ops.is_empty() => {
            extract_number(&element.source)
        }
        _ => Err(ParseError::custom(
            "Expected number, got complex expression",
            Some(ast.span()),
        )),
    }
}

/// Helper function to extract a number from a Pattern by querying it
/// Returns the numeric value if the pattern produces exactly one numeric event
fn extract_number_from_pattern(pattern: &Pattern) -> Result<f64> {
    let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
    let haps = pattern.query(state);

    if haps.len() != 1 {
        return Err(ParseError::custom(
            "Range operator requires source to be a single number",
            None,
        ));
    }

    match &haps[0].value {
        Value::Number(n) => Ok(*n),
        _ => Err(ParseError::custom(
            "Range operator requires source to be a number",
            None,
        )),
    }
}

/// Helper function to extract weight from an AST node
/// Returns the weight value, defaulting to 1.0 if no weight is found
fn extract_weight(ast: &Ast) -> f64 {
    match ast {
        // Pattern nodes: check if they have a single element and extract its weight
        Ast::Pattern(pattern) => {
            if pattern.children.len() == 1 {
                extract_weight(&pattern.children[0])
            } else {
                1.0
            }
        }
        // Element nodes have weight directly
        Ast::Element(element) => element.weight,
        // Other nodes have default weight
        _ => 1.0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::parse;
    use strudel_core::{Fraction, State, TimeSpan};

    #[test]
    fn test_eval_single_atom() {
        let ast = parse("bd").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::String("bd".into()));
    }

    #[test]
    fn test_eval_number() {
        let ast = parse("42").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::Number(42.0));
    }

    #[test]
    fn test_eval_silence() {
        let ast = parse("~").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 0);
    }

    #[test]
    fn test_eval_sequence() {
        let ast = parse("bd sd cp").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 3);
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(haps[1].value, Value::String("sd".into()));
        assert_eq!(haps[2].value, Value::String("cp".into()));
    }

    #[test]
    fn test_eval_stack() {
        let ast = parse("bd,sd").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 2);
        // Both events should be simultaneous (same timespan)
    }

    #[test]
    fn test_eval_with_fast() {
        let ast = parse("bd*2").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 2); // Should have 2 events in one cycle
    }

    #[test]
    fn test_eval_with_slow() {
        let ast = parse("bd/2").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(2)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 1); // Should have 1 event over 2 cycles
    }

    #[test]
    fn test_eval_with_replicate() {
        let ast = parse("bd!3").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 3); // Should replicate into 3 events
    }

    #[test]
    fn test_eval_euclidean() {
        let ast = parse("bd(3,8)").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 3 events distributed across 8 steps
        assert!(haps.len() <= 3);
    }

    #[test]
    fn test_eval_polymeter() {
        let ast = parse("{bd sd, hh oh cp}").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have events from both patterns
        // Pattern 1: bd sd (2 steps) - plays 3 times = 6 events
        // Pattern 2: hh oh cp (3 steps) - plays 2 times = 6 events
        // Total: 12 events (stacked)
        assert!(haps.len() >= 6);

        // Check that we have events from both patterns
        let values: Vec<_> = haps.iter().map(|h| &h.value).collect();
        assert!(values.iter().any(|v| matches!(v, Value::String(s) if s == "bd")));
        assert!(values.iter().any(|v| matches!(v, Value::String(s) if s == "hh")));
    }

    #[test]
    fn test_eval_rand() {
        let ast = parse("bd | sd | cp").unwrap();
        let pattern = evaluate(&ast).unwrap();

        // Query multiple cycles to verify deterministic randomness
        let state1 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps1 = pattern.query(state1);

        let state2 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps2 = pattern.query(state2);

        // Same cycle should produce same result (deterministic)
        assert_eq!(haps1.len(), 1);
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps1[0].value, haps2[0].value);

        // Value should be one of the three options
        let valid_values = [
            Value::String("bd".into()),
            Value::String("sd".into()),
            Value::String("cp".into()),
        ];
        assert!(valid_values.contains(&haps1[0].value));
    }

    #[test]
    fn test_eval_rand_different_cycles() {
        let ast = parse("bd | sd | cp").unwrap();
        let pattern = evaluate(&ast).unwrap();

        // Query different cycles
        let state1 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps1 = pattern.query(state1);

        let state2 = State::new(TimeSpan::new(Fraction::from_int(1), Fraction::from_int(2)));
        let haps2 = pattern.query(state2);

        let state3 = State::new(TimeSpan::new(Fraction::from_int(2), Fraction::from_int(3)));
        let haps3 = pattern.query(state3);

        // Each should have exactly one event
        assert_eq!(haps1.len(), 1);
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps3.len(), 1);

        // All should be valid values
        let valid_values = [
            Value::String("bd".into()),
            Value::String("sd".into()),
            Value::String("cp".into()),
        ];
        assert!(valid_values.contains(&haps1[0].value));
        assert!(valid_values.contains(&haps2[0].value));
        assert!(valid_values.contains(&haps3[0].value));
    }

    #[test]
    fn test_eval_feet_operator() {
        // Feet operator (.) creates sub-patterns
        // "a . b c" is equivalent to "[a] [b c]"
        let ast1 = parse("a . b c").unwrap();
        let pattern1 = evaluate(&ast1).unwrap();

        let ast2 = parse("[a] [b c]").unwrap();
        let pattern2 = evaluate(&ast2).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps1 = pattern1.query(state.clone());
        let haps2 = pattern2.query(state);

        // Should have same number of events
        assert_eq!(haps1.len(), haps2.len());
        assert_eq!(haps1.len(), 3);

        // First group [a] should occupy first half
        assert_eq!(haps1[0].whole.as_ref().unwrap().begin, Fraction::from_int(0));
        assert_eq!(haps1[0].whole.as_ref().unwrap().end, Fraction::new(1, 2));

        // Second group [b c] should occupy second half
        assert_eq!(haps1[1].whole.as_ref().unwrap().begin, Fraction::new(1, 2));
        assert_eq!(haps1[2].whole.as_ref().unwrap().end, Fraction::from_int(1));

        // Both patterns should produce identical results
        assert_eq!(haps1[0].value, haps2[0].value);
        assert_eq!(haps1[1].value, haps2[1].value);
        assert_eq!(haps1[2].value, haps2[2].value);
    }

    #[test]
    fn test_eval_weighted_rand() {
        let ast = parse("bd@2 | sd | cp").unwrap();
        let pattern = evaluate(&ast).unwrap();

        // Test determinism - same seed should give same results
        let state1 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps1 = pattern.query(state1);

        let state2 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps2 = pattern.query(state2);

        assert_eq!(haps1.len(), 1);
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps1[0].value, haps2[0].value);

        // All values should be valid
        let valid_values = [
            Value::String("bd".into()),
            Value::String("sd".into()),
            Value::String("cp".into()),
        ];
        assert!(valid_values.contains(&haps1[0].value));
    }

    #[test]
    fn test_eval_weighted_rand_distribution() {
        let ast = parse("bd@3 | sd").unwrap();
        let pattern = evaluate(&ast).unwrap();

        // Sample over many cycles to check distribution
        let mut bd_count = 0;
        let mut sd_count = 0;

        for cycle in 0..100 {
            let state = State::new(TimeSpan::new(
                Fraction::from_int(cycle),
                Fraction::from_int(cycle + 1),
            ));
            let haps = pattern.query(state);

            if haps.len() == 1 {
                match &haps[0].value {
                    Value::String(s) if s == "bd" => bd_count += 1,
                    Value::String(s) if s == "sd" => sd_count += 1,
                    _ => {}
                }
            }
        }

        // bd should appear approximately 3x more than sd (75% vs 25%)
        // With 100 samples, expect ~75 bd and ~25 sd
        // Allow for randomness: bd should be > 60% and < 90%
        assert!(bd_count > 60, "bd appeared {} times, expected > 60", bd_count);
        assert!(bd_count < 90, "bd appeared {} times, expected < 90", bd_count);
        assert!(sd_count > 10, "sd appeared {} times, expected > 10", sd_count);
        assert!(sd_count < 40, "sd appeared {} times, expected < 40", sd_count);
    }

    #[test]
    fn test_extract_weight() {
        // Test weight extraction from different AST nodes
        let ast1 = parse("bd@2").unwrap();
        let weight1 = extract_weight(&ast1);
        assert_eq!(weight1, 2.0);

        let ast2 = parse("sd").unwrap();
        let weight2 = extract_weight(&ast2);
        assert_eq!(weight2, 1.0);

        let ast3 = parse("cp@5").unwrap();
        let weight3 = extract_weight(&ast3);
        assert_eq!(weight3, 5.0);
    }

    #[test]
    fn test_eval_tail_simple() {
        // Test simple tail operator: a:b should concatenate a and b
        let ast = parse("bd:sd").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 2 events: bd in first half, sd in second half
        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(haps[1].value, Value::String("sd".into()));

        // Check timing - each should occupy half the cycle
        assert_eq!(haps[0].part.begin, Fraction::from_int(0));
        assert_eq!(haps[0].part.end, Fraction::new(1, 2));
        assert_eq!(haps[1].part.begin, Fraction::new(1, 2));
        assert_eq!(haps[1].part.end, Fraction::from_int(1));
    }

    #[test]
    fn test_eval_tail_nested() {
        // Test nested tail operator: a:b:c should concatenate a, b, and c
        let ast = parse("bd:sd:cp").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 3 events: bd, sd, cp, each occupying 1/3 of the cycle
        assert_eq!(haps.len(), 3);
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(haps[1].value, Value::String("sd".into()));
        assert_eq!(haps[2].value, Value::String("cp".into()));
    }

    #[test]
    fn test_eval_tail_with_pattern() {
        // Test tail with bracketed pattern: a:[b c]
        let ast = parse("bd:[sd cp]").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 3 events:
        // - bd in first half
        // - sd and cp splitting the second half
        assert_eq!(haps.len(), 3);
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(haps[1].value, Value::String("sd".into()));
        assert_eq!(haps[2].value, Value::String("cp".into()));

        // bd should occupy first half
        assert_eq!(haps[0].part.begin, Fraction::from_int(0));
        assert_eq!(haps[0].part.end, Fraction::new(1, 2));

        // sd and cp should split the second half
        assert_eq!(haps[1].part.begin, Fraction::new(1, 2));
        assert_eq!(haps[1].part.end, Fraction::new(3, 4));
        assert_eq!(haps[2].part.begin, Fraction::new(3, 4));
        assert_eq!(haps[2].part.end, Fraction::from_int(1));
    }

    #[test]
    fn test_struct_basic() {
        // Test basic struct operation with a simple pattern
        // Create a chord pattern and apply a simple rhythm
        use strudel_core::{fastcat, pure};

        let values = stack(vec![
            pure(Value::String("c".into())),
            pure(Value::String("e".into())),
            pure(Value::String("g".into())),
        ]);

        // Structure: 1 0 1 0 (play on beats 1 and 3)
        let structure = fastcat(vec![
            pure(Value::Number(1.0)),
            pure(Value::Number(0.0)),
            pure(Value::Number(1.0)),
            pure(Value::Number(0.0)),
        ]);

        let result = values.struct_(structure);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = result.query(state);

        // Should have 6 events (3 notes × 2 times when structure is 1)
        assert_eq!(haps.len(), 6);

        // All events should be one of our chord notes
        for hap in &haps {
            match &hap.value {
                Value::String(s) => assert!(s == "c" || s == "e" || s == "g"),
                _ => panic!("Expected string value"),
            }
        }
    }

    #[test]
    fn test_struct_with_strings() {
        // Test struct with string patterns (like "x ~ x ~")
        use strudel_core::{fastcat, pure};

        let values = pure(Value::String("bd".into()));

        // Structure using strings: "x" for on, "~" for off
        let structure = fastcat(vec![
            pure(Value::String("x".into())),
            pure(Value::String("~".into())),
            pure(Value::String("x".into())),
            pure(Value::String("~".into())),
        ]);

        let result = values.struct_(structure);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = result.query(state);

        // Should have 2 events (at positions 0 and 2)
        assert_eq!(haps.len(), 2);

        // Both should be "bd"
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(haps[1].value, Value::String("bd".into()));

        // Check timing - should be at first and third quarter
        assert_eq!(haps[0].part.begin, Fraction::from_int(0));
        assert_eq!(haps[0].part.end, Fraction::new(1, 4));
        assert_eq!(haps[1].part.begin, Fraction::new(1, 2));
        assert_eq!(haps[1].part.end, Fraction::new(3, 4));
    }

    #[test]
    fn test_struct_silence() {
        // Test that zeros in structure pattern filter out events
        use strudel_core::{fastcat, pure};

        let values = fastcat(vec![
            pure(Value::String("a".into())),
            pure(Value::String("b".into())),
            pure(Value::String("c".into())),
            pure(Value::String("d".into())),
        ]);

        // All zeros - should filter everything out
        let structure = fastcat(vec![
            pure(Value::Number(0.0)),
            pure(Value::Number(0.0)),
            pure(Value::Number(0.0)),
            pure(Value::Number(0.0)),
        ]);

        let result = values.struct_(structure);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = result.query(state);

        // Should have no events
        assert_eq!(haps.len(), 0);
    }

    #[test]
    fn test_eval_range_simple() {
        // Test simple range: 0 .. 3 should expand to [0, 1, 2, 3]
        // Note: spaces are required around .. operator for proper tokenization
        let ast = parse("0 .. 3").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 4 events: 0, 1, 2, 3
        assert_eq!(haps.len(), 4);
        assert_eq!(haps[0].value, Value::Number(0.0));
        assert_eq!(haps[1].value, Value::Number(1.0));
        assert_eq!(haps[2].value, Value::Number(2.0));
        assert_eq!(haps[3].value, Value::Number(3.0));

        // Check timing - each should occupy 1/4 of the cycle
        assert_eq!(haps[0].part.begin, Fraction::from_int(0));
        assert_eq!(haps[0].part.end, Fraction::new(1, 4));
        assert_eq!(haps[1].part.begin, Fraction::new(1, 4));
        assert_eq!(haps[1].part.end, Fraction::new(1, 2));
        assert_eq!(haps[2].part.begin, Fraction::new(1, 2));
        assert_eq!(haps[2].part.end, Fraction::new(3, 4));
        assert_eq!(haps[3].part.begin, Fraction::new(3, 4));
        assert_eq!(haps[3].part.end, Fraction::from_int(1));
    }

    #[test]
    fn test_eval_range_larger() {
        // Test larger range: 0 .. 7 should expand to [0, 1, 2, 3, 4, 5, 6, 7]
        let ast = parse("0 .. 7").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 8 events
        assert_eq!(haps.len(), 8);

        // Check all values
        for (i, hap) in haps.iter().enumerate().take(8) {
            assert_eq!(hap.value, Value::Number(i as f64));
        }
    }

    #[test]
    fn test_eval_range_descending() {
        // Test descending range: 5 .. 2 should expand to [5, 4, 3, 2]
        let ast = parse("5 .. 2").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 4 events in descending order
        assert_eq!(haps.len(), 4);
        assert_eq!(haps[0].value, Value::Number(5.0));
        assert_eq!(haps[1].value, Value::Number(4.0));
        assert_eq!(haps[2].value, Value::Number(3.0));
        assert_eq!(haps[3].value, Value::Number(2.0));
    }

    #[test]
    fn test_eval_range_single() {
        // Test single value range: 3 .. 3 should expand to [3]
        let ast = parse("3 .. 3").unwrap();
        let pattern = evaluate(&ast).unwrap();

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 1 event
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::Number(3.0));
    }

    #[test]
    fn test_scale_operator_basic() {
        // Test basic scale operator with numbers
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: 0 2 4
        let pattern_ast = Ast::Pattern(PatternNode::new(
            vec![
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(0.0, Span::new(0, 1))),
                    Span::new(0, 1),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(2.0, Span::new(2, 3))),
                    Span::new(2, 3),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(4.0, Span::new(4, 5))),
                    Span::new(4, 5),
                )),
            ],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 5),
        ));

        // Wrap in Scale operator
        let scale_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Scale,
            OperatorArgs::String("C:major".to_string()),
            pattern_ast,
            Span::new(0, 20),
        ));

        let pattern = evaluate(&scale_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 3 events
        assert_eq!(haps.len(), 3);

        // Check that numbers were converted to notes
        // 0 -> C3, 2 -> E3, 4 -> G3 (in C major scale)
        assert_eq!(haps[0].value, Value::String("C3".into()));
        assert_eq!(haps[1].value, Value::String("E3".into()));
        assert_eq!(haps[2].value, Value::String("G3".into()));

        // Check that scale is stored in context
        assert_eq!(
            haps[0].context.metadata.get("scale"),
            Some(&Value::String("C:major".into()))
        );
    }

    #[test]
    fn test_scale_operator_with_negative_numbers() {
        // Test scale with negative numbers (should wrap to lower octaves)
        use crate::ast::*;
        use crate::span::Span;

        let pattern_ast = Ast::Pattern(PatternNode::new(
            vec![
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(-7.0, Span::new(0, 2))),
                    Span::new(0, 2),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(0.0, Span::new(3, 4))),
                    Span::new(3, 4),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(7.0, Span::new(5, 6))),
                    Span::new(5, 6),
                )),
            ],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 6),
        ));

        let scale_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Scale,
            OperatorArgs::String("C:major".to_string()),
            pattern_ast,
            Span::new(0, 20),
        ));

        let pattern = evaluate(&scale_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        assert_eq!(haps.len(), 3);

        // -7 wraps to C2 (one octave below C3)
        assert_eq!(haps[0].value, Value::String("C2".into()));
        // 0 is C3
        assert_eq!(haps[1].value, Value::String("C3".into()));
        // 7 wraps to C4 (one octave above C3)
        assert_eq!(haps[2].value, Value::String("C4".into()));
    }

    #[test]
    fn test_scale_operator_preserves_strings() {
        // Test that scale operator doesn't modify string values
        use crate::ast::*;
        use crate::span::Span;

        let pattern_ast = Ast::Pattern(PatternNode::new(
            vec![
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("bd", Span::new(0, 2))),
                    Span::new(0, 2),
                )),
            ],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 2),
        ));

        let scale_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Scale,
            OperatorArgs::String("C:major".to_string()),
            pattern_ast,
            Span::new(0, 20),
        ));

        let pattern = evaluate(&scale_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // String should be preserved
        assert_eq!(haps[0].value, Value::String("bd".into()));

        // But scale should still be in context
        assert_eq!(
            haps[0].context.metadata.get("scale"),
            Some(&Value::String("C:major".into()))
        );
    }

    #[test]
    fn test_shift_operator_basic() {
        // Test basic shift operator - shift pattern later (positive value)
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: bd
        let pattern_ast = Ast::Element(ElementNode::new(
            Ast::Atom(AtomNode::string("bd", Span::new(0, 2))),
            Span::new(0, 2),
        ));

        // First test without shift to understand base behavior
        let base_pattern = evaluate(&pattern_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let base_haps = base_pattern.query(state);

        println!("Base pattern (no shift):");
        for hap in &base_haps {
            println!("  part: {} to {}, whole: {:?}", hap.part.begin, hap.part.end, hap.whole);
        }

        // Wrap in Shift operator with amount 0.25 (1/4 cycle)
        let shift_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Shift,
            OperatorArgs::Number(0.25),
            pattern_ast,
            Span::new(0, 10),
        ));

        let pattern = evaluate(&shift_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        println!("Shifted pattern (shift 0.25):");
        for hap in &haps {
            println!("  part: {} to {}, whole: {:?}", hap.part.begin, hap.part.end, hap.whole);
        }

        // Should have 1 event, shifted 0.25 cycles later
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::String("bd".into()));

        // Event should start at 0.25 instead of 0.0
        assert_eq!(haps[0].part.begin, Fraction::new(1, 4));
        // The pattern repeats every cycle, so end should be at 1.0 (not 1.25)
        // because split_queries splits at cycle boundaries
        assert_eq!(haps[0].part.end, Fraction::from_int(1));
    }

    #[test]
    fn test_shift_operator_negative() {
        // Test shift operator with negative value (shift earlier)
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: sd
        let pattern_ast = Ast::Element(ElementNode::new(
            Ast::Atom(AtomNode::string("sd", Span::new(0, 2))),
            Span::new(0, 2),
        ));

        // Shift -0.5 cycles (half cycle earlier)
        let shift_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Shift,
            OperatorArgs::Number(-0.5),
            pattern_ast,
            Span::new(0, 10),
        ));

        let pattern = evaluate(&shift_ast).unwrap();
        // Query cycle 0 to 1
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        println!("Shifted pattern (shift -0.5):");
        for hap in &haps {
            println!("  part: {} to {}, whole: {:?}", hap.part.begin, hap.part.end, hap.whole);
        }

        // With split_queries and shift -0.5:
        // When we query [0, 1), the shift causes it to query [-0.5, 0.5)
        // split_queries splits this into two queries: [-0.5, 0) and [0, 0.5)
        // The first query gets the pattern from cycle -1, shifted to [0, 0.5)
        // The second query gets the pattern from cycle 0, shifted to [0.5, 1)
        // So we get the whole cycle [0, 1) covered by two shifted events
        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value, Value::String("sd".into()));
        assert_eq!(haps[0].part.begin, Fraction::from_int(0));
        assert_eq!(haps[0].part.end, Fraction::new(1, 2));
        assert_eq!(haps[1].value, Value::String("sd".into()));
        assert_eq!(haps[1].part.begin, Fraction::new(1, 2));
        assert_eq!(haps[1].part.end, Fraction::from_int(1));
    }

    #[test]
    fn test_shift_operator_with_sequence() {
        // Test shift operator on a sequence pattern
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: bd sd cp
        let pattern_ast = Ast::Pattern(PatternNode::new(
            vec![
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("bd", Span::new(0, 2))),
                    Span::new(0, 2),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("sd", Span::new(3, 5))),
                    Span::new(3, 5),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("cp", Span::new(6, 8))),
                    Span::new(6, 8),
                )),
            ],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 8),
        ));

        // Shift 0.125 cycles later (1/8 cycle)
        let shift_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Shift,
            OperatorArgs::Number(0.125),
            pattern_ast,
            Span::new(0, 20),
        ));

        let pattern = evaluate(&shift_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        println!("Shifted sequence (shift 0.125):");
        for (i, hap) in haps.iter().enumerate() {
            println!("  [{}] {}: {} to {}", i, hap.value, hap.part.begin, hap.part.end);
        }

        // Should have 3 events, all shifted
        assert_eq!(haps.len(), 3);

        // Check values
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(haps[1].value, Value::String("sd".into()));
        assert_eq!(haps[2].value, Value::String("cp".into()));

        // Check timing - each event should be shifted by 0.125
        // Original: bd at 0-1/3, sd at 1/3-2/3, cp at 2/3-1
        // Shifted: bd at 1/8-11/24, sd at 11/24-19/24, cp at 19/24-1

        assert_eq!(haps[0].part.begin, Fraction::new(1, 8)); // 0 + 1/8
        // bd end: 1/3 + 1/8 = 8/24 + 3/24 = 11/24
        assert_eq!(haps[0].part.end, Fraction::new(11, 24));
        // sd begin: 1/3 + 1/8 = 11/24
        assert_eq!(haps[1].part.begin, Fraction::new(11, 24));
        // sd end: 2/3 + 1/8 = 16/24 + 3/24 = 19/24
        assert_eq!(haps[1].part.end, Fraction::new(19, 24));
        // cp begin: 2/3 + 1/8 = 19/24
        assert_eq!(haps[2].part.begin, Fraction::new(19, 24));
        // cp end: 1 (at cycle boundary)
        assert_eq!(haps[2].part.end, Fraction::from_int(1));
    }

    #[test]
    fn test_target_operator_basic() {
        // Test basic target operator - set target metadata
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: bd
        let pattern_ast = Ast::Element(ElementNode::new(
            Ast::Atom(AtomNode::string("bd", Span::new(0, 2))),
            Span::new(0, 2),
        ));

        // Wrap in Target operator with target name "drums"
        let target_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Target,
            OperatorArgs::String("drums".to_string()),
            pattern_ast,
            Span::new(0, 20),
        ));

        let pattern = evaluate(&target_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 1 event
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::String("bd".into()));

        // Check that target is stored in context
        assert_eq!(
            haps[0].context.metadata.get("target"),
            Some(&Value::String("drums".into()))
        );
    }

    #[test]
    fn test_target_operator_with_sequence() {
        // Test target operator on a sequence pattern
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: bd sd cp
        let pattern_ast = Ast::Pattern(PatternNode::new(
            vec![
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("bd", Span::new(0, 2))),
                    Span::new(0, 2),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("sd", Span::new(3, 5))),
                    Span::new(3, 5),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("cp", Span::new(6, 8))),
                    Span::new(6, 8),
                )),
            ],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 8),
        ));

        // Apply target "percussion"
        let target_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Target,
            OperatorArgs::String("percussion".to_string()),
            pattern_ast,
            Span::new(0, 30),
        ));

        let pattern = evaluate(&target_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 3 events, all with the same target
        assert_eq!(haps.len(), 3);

        // Check that all events have the target metadata
        for hap in &haps {
            assert_eq!(
                hap.context.metadata.get("target"),
                Some(&Value::String("percussion".into()))
            );
        }

        // Verify values are preserved
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(haps[1].value, Value::String("sd".into()));
        assert_eq!(haps[2].value, Value::String("cp".into()));
    }

    #[test]
    fn test_target_operator_preserves_timing() {
        // Test that target operator doesn't affect timing
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: bd sd
        let pattern_ast = Ast::Pattern(PatternNode::new(
            vec![
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("bd", Span::new(0, 2))),
                    Span::new(0, 2),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::string("sd", Span::new(3, 5))),
                    Span::new(3, 5),
                )),
            ],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 5),
        ));

        // Query without target
        let pattern_no_target = evaluate(&pattern_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps_no_target = pattern_no_target.query(state.clone());

        // Query with target
        let target_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Target,
            OperatorArgs::String("synth".to_string()),
            pattern_ast,
            Span::new(0, 20),
        ));
        let pattern_with_target = evaluate(&target_ast).unwrap();
        let haps_with_target = pattern_with_target.query(state);

        // Should have same number of events
        assert_eq!(haps_no_target.len(), haps_with_target.len());

        // Check that timing is preserved
        for i in 0..haps_no_target.len() {
            assert_eq!(haps_no_target[i].part.begin, haps_with_target[i].part.begin);
            assert_eq!(haps_no_target[i].part.end, haps_with_target[i].part.end);
            assert_eq!(haps_no_target[i].value, haps_with_target[i].value);
        }
    }

    #[test]
    fn test_target_operator_with_numbers() {
        // Test target operator with numeric patterns
        use crate::ast::*;
        use crate::span::Span;

        // Create AST for: 0 1 2
        let pattern_ast = Ast::Pattern(PatternNode::new(
            vec![
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(0.0, Span::new(0, 1))),
                    Span::new(0, 1),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(1.0, Span::new(2, 3))),
                    Span::new(2, 3),
                )),
                Ast::Element(ElementNode::new(
                    Ast::Atom(AtomNode::number(2.0, Span::new(4, 5))),
                    Span::new(4, 5),
                )),
            ],
            Alignment::Fastcat,
            None,
            false,
            Span::new(0, 5),
        ));

        // Apply target "midi"
        let target_ast = Ast::Operator(OperatorNode::new(
            OperatorType::Target,
            OperatorArgs::String("midi".to_string()),
            pattern_ast,
            Span::new(0, 20),
        ));

        let pattern = evaluate(&target_ast).unwrap();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = pattern.query(state);

        // Should have 3 events with target metadata
        assert_eq!(haps.len(), 3);

        for (i, hap) in haps.iter().enumerate() {
            assert_eq!(hap.value, Value::Number(i as f64));
            assert_eq!(
                hap.context.metadata.get("target"),
                Some(&Value::String("midi".into()))
            );
        }
    }
}
