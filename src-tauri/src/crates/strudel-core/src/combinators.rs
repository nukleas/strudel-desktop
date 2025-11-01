use crate::{Fraction, Hap, Pattern, TimeSpan, Value};
use std::sync::Arc;

/// Create a pattern with a single constant value
///
/// The value is active for all time
pub fn pure(value: Value) -> Pattern {
    Pattern::new(move |state| {
        vec![Hap::new(Some(state.span), state.span, value.clone())]
    })
}

/// Create an empty/silent pattern
///
/// Returns no events for any query
pub fn silence() -> Pattern {
    Pattern::new(|_state| Vec::new())
}

/// Concatenate patterns, switching between them successively per cycle
///
/// This is also known as "slowcat" - each pattern plays for one full cycle
pub fn slowcat(patterns: Vec<Pattern>) -> Pattern {
    if patterns.is_empty() {
        return silence();
    }

    if patterns.len() == 1 {
        return patterns.into_iter().next().unwrap();
    }

    let pat_count = patterns.len() as i64;
    let patterns_rc = Arc::new(patterns);

    // Calculate LCM of all pattern steps
    let steps = patterns_rc
        .iter()
        .filter_map(|p| p.get_steps())
        .reduce(|acc, s| {
            let lcm_val = Fraction::lcm(
                acc.numerator * s.denominator,
                s.numerator * acc.denominator,
            );
            Fraction::new(lcm_val / acc.denominator, s.denominator)
        });

    Pattern::with_steps(
        move |state| {
            let span = state.span;

            // Calculate which pattern to use based on the cycle
            let begin_cycle = span.begin.floor().numerator;
            let pat_n = ((begin_cycle % pat_count) + pat_count) % pat_count; // Handle negative cycles

            if let Some(pat) = patterns_rc.get(pat_n as usize) {
                // Calculate offset to make pattern cycles line up correctly
                let cycle_offset = Fraction::from_int(begin_cycle)
                    - (Fraction::from_int(begin_cycle / pat_count) * Fraction::from_int(pat_count));

                // Query the pattern with adjusted timespan
                let adjusted_span = TimeSpan::new(span.begin - cycle_offset, span.end - cycle_offset);
                let adjusted_state = state.set_span(adjusted_span);

                pat.query(adjusted_state)
                    .into_iter()
                    .map(|hap| {
                        hap.with_span(|ts| {
                            TimeSpan::new(ts.begin + cycle_offset, ts.end + cycle_offset)
                        })
                    })
                    .collect()
            } else {
                Vec::new()
            }
        },
        steps,
    )
    .split_queries()
}

/// Concatenate patterns, cramming them all into one cycle
///
/// This is also known as "fastcat" or "sequence" - plays all patterns
/// within a single cycle
pub fn fastcat(patterns: Vec<Pattern>) -> Pattern {
    if patterns.is_empty() {
        return silence();
    }

    if patterns.len() == 1 {
        return patterns.into_iter().next().unwrap();
    }

    let pat_count = patterns.len();
    let mut result = slowcat(patterns);

    // Speed up by the number of patterns
    result = result.with_query_time(move |t| t * Fraction::from_int(pat_count as i64));
    result = result.with_hap_time(move |t| t / Fraction::from_int(pat_count as i64));

    result.set_steps(Some(Fraction::from_int(pat_count as i64)))
}

/// Alias for fastcat
pub fn sequence(patterns: Vec<Pattern>) -> Pattern {
    fastcat(patterns)
}

/// Stack/layer multiple patterns on top of each other
///
/// All patterns play simultaneously (polyrhythm)
pub fn stack(patterns: Vec<Pattern>) -> Pattern {
    if patterns.is_empty() {
        return silence();
    }

    if patterns.len() == 1 {
        return patterns.into_iter().next().unwrap();
    }

    let patterns_rc = Arc::new(patterns);

    // Calculate LCM of all pattern steps
    let steps = patterns_rc
        .iter()
        .filter_map(|p| p.get_steps())
        .reduce(|acc, s| {
            let lcm_val = Fraction::lcm(
                acc.numerator * s.denominator,
                s.numerator * acc.denominator,
            );
            Fraction::new(lcm_val / acc.denominator, s.denominator)
        });

    Pattern::with_steps(
        move |state| {
            patterns_rc
                .iter()
                .flat_map(|pat| pat.query(state.clone()))
                .collect()
        },
        steps,
    )
}

/// Alias for stack
pub fn polyrhythm(patterns: Vec<Pattern>) -> Pattern {
    stack(patterns)
}

/// Polymeter - play patterns with different step counts simultaneously
///
/// Each pattern is sped up proportionally so they all complete
/// their cycles at the same time based on LCM of step counts.
///
/// For example, `{bd sd, hh oh cp}`:
/// - Pattern 1 has 2 steps, Pattern 2 has 3 steps
/// - LCM(2, 3) = 6 steps
/// - Pattern 1 plays 3 times (2 * 3 = 6)
/// - Pattern 2 plays 2 times (3 * 2 = 6)
pub fn polymeter(patterns: Vec<Pattern>) -> Pattern {
    if patterns.is_empty() {
        return silence();
    }

    if patterns.len() == 1 {
        return patterns.into_iter().next().unwrap();
    }

    // Get step counts for each pattern
    let step_counts: Vec<i64> = patterns
        .iter()
        .map(|p| {
            p.get_steps()
                .map(|f| f.numerator)
                .unwrap_or(1)
        })
        .collect();

    // Calculate LCM of all step counts
    let total_steps = step_counts
        .iter()
        .fold(1, |acc, &s| Fraction::lcm(acc, s));

    // Speed up each pattern proportionally
    let adjusted_patterns: Vec<Pattern> = patterns
        .into_iter()
        .zip(step_counts.iter())
        .map(|(pat, &steps)| {
            let speed_factor = (total_steps as f64) / (steps as f64);
            pat.fast(speed_factor)
        })
        .collect();

    // Stack the adjusted patterns
    stack(adjusted_patterns)
}

/// Choose - randomly select one pattern per cycle
///
/// Uses seed for deterministic selection based on cycle number.
/// Each cycle will consistently choose the same pattern for the same seed.
pub fn choose(patterns: Vec<Pattern>, seed: u64) -> Pattern {
    if patterns.is_empty() {
        return silence();
    }

    if patterns.len() == 1 {
        return patterns.into_iter().next().unwrap();
    }

    let patterns_rc = Arc::new(patterns);
    let pat_count = patterns_rc.len();

    Pattern::new(move |state| {
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;

        // Use cycle number + seed for deterministic selection
        let cycle = state.span.begin.floor().numerator;
        let cycle_seed = seed.wrapping_add(cycle as u64);

        let mut rng = StdRng::seed_from_u64(cycle_seed);
        let choice = rng.gen_range(0..pat_count);

        patterns_rc[choice].query(state)
    })
    .split_queries()
}

/// Choose with weights - randomly select one pattern per cycle using weighted probabilities
///
/// Uses seed for deterministic selection based on cycle number.
/// Weights determine the relative probability of selecting each pattern.
///
/// # Arguments
/// * `patterns_with_weights` - Vector of (pattern, weight) tuples
/// * `seed` - Random seed for deterministic selection
///
/// # Examples
/// ```
/// use strudel_core::{pure, Value, choose_weighted};
///
/// let bd = pure(Value::String("bd".into()));
/// let sd = pure(Value::String("sd".into()));
/// let cp = pure(Value::String("cp".into()));
///
/// // bd is 2x more likely to be selected than sd or cp
/// let pattern = choose_weighted(vec![(bd, 2.0), (sd, 1.0), (cp, 1.0)], 0);
/// ```
pub fn choose_weighted(patterns_with_weights: Vec<(Pattern, f64)>, seed: u64) -> Pattern {
    if patterns_with_weights.is_empty() {
        return silence();
    }

    if patterns_with_weights.len() == 1 {
        return patterns_with_weights.into_iter().next().unwrap().0;
    }

    let patterns: Vec<Pattern> = patterns_with_weights.iter().map(|(p, _)| p.clone()).collect();
    let weights: Vec<f64> = patterns_with_weights.iter().map(|(_, w)| *w).collect();

    let patterns_rc = Arc::new(patterns);
    let weights_rc = Arc::new(weights);

    Pattern::new(move |state| {
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;

        // Use cycle number + seed for deterministic selection
        let cycle = state.span.begin.floor().numerator;
        let cycle_seed = seed.wrapping_add(cycle as u64);

        let mut rng = StdRng::seed_from_u64(cycle_seed);

        // Calculate total weight
        let total_weight: f64 = weights_rc.iter().sum();

        if total_weight <= 0.0 {
            // If all weights are zero or negative, choose uniformly
            let choice = rng.gen_range(0..patterns_rc.len());
            return patterns_rc[choice].query(state);
        }

        // Generate random value in [0, total_weight)
        let random_value = rng.gen::<f64>() * total_weight;

        // Find the pattern corresponding to this value
        let mut cumulative = 0.0;
        for (i, weight) in weights_rc.iter().enumerate() {
            cumulative += weight;
            if random_value < cumulative {
                return patterns_rc[i].query(state);
            }
        }

        // Fallback (shouldn't happen due to floating point precision)
        patterns_rc[patterns_rc.len() - 1].query(state)
    })
    .split_queries()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::State;

    #[test]
    fn test_pure() {
        let pat = pure(Value::Number(42.0));
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));

        let haps = pat.query(state);
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::Number(42.0));
    }

    #[test]
    fn test_silence() {
        let pat = silence();
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));

        let haps = pat.query(state);
        assert_eq!(haps.len(), 0);
    }

    #[test]
    fn test_fastcat() {
        let pat1 = pure(Value::String("a".into()));
        let pat2 = pure(Value::String("b".into()));
        let pat3 = pure(Value::String("c".into()));

        let combined = fastcat(vec![pat1, pat2, pat3]);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));

        let haps = combined.query(state);
        assert_eq!(haps.len(), 3);

        // Each event should occupy 1/3 of a cycle
        assert!(haps[0].whole.unwrap().begin == Fraction::new(0, 1));
        assert!(haps[0].whole.unwrap().end == Fraction::new(1, 3));
        assert_eq!(haps[0].value, Value::String("a".into()));

        assert!(haps[1].whole.unwrap().begin == Fraction::new(1, 3));
        assert!(haps[1].whole.unwrap().end == Fraction::new(2, 3));
        assert_eq!(haps[1].value, Value::String("b".into()));

        assert!(haps[2].whole.unwrap().begin == Fraction::new(2, 3));
        assert!(haps[2].whole.unwrap().end == Fraction::new(1, 1));
        assert_eq!(haps[2].value, Value::String("c".into()));
    }

    #[test]
    fn test_slowcat() {
        let pat1 = pure(Value::String("a".into()));
        let pat2 = pure(Value::String("b".into()));

        let combined = slowcat(vec![pat1, pat2]);

        // Query first cycle (should get "a")
        let state1 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps1 = combined.query(state1);
        assert_eq!(haps1.len(), 1);
        assert_eq!(haps1[0].value, Value::String("a".into()));

        // Query second cycle (should get "b")
        let state2 = State::new(TimeSpan::new(Fraction::from_int(1), Fraction::from_int(2)));
        let haps2 = combined.query(state2);
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps2[0].value, Value::String("b".into()));

        // Query third cycle (should wrap back to "a")
        let state3 = State::new(TimeSpan::new(Fraction::from_int(2), Fraction::from_int(3)));
        let haps3 = combined.query(state3);
        assert_eq!(haps3.len(), 1);
        assert_eq!(haps3[0].value, Value::String("a".into()));
    }

    #[test]
    fn test_stack() {
        let pat1 = pure(Value::String("a".into()));
        let pat2 = pure(Value::String("b".into()));

        let combined = stack(vec![pat1, pat2]);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));

        let haps = combined.query(state);
        assert_eq!(haps.len(), 2);

        // Both events should span the full cycle
        assert_eq!(haps[0].whole.unwrap().begin, Fraction::from_int(0));
        assert_eq!(haps[0].whole.unwrap().end, Fraction::from_int(1));
        assert_eq!(haps[1].whole.unwrap().begin, Fraction::from_int(0));
        assert_eq!(haps[1].whole.unwrap().end, Fraction::from_int(1));

        // Should have both values
        let values: Vec<_> = haps.iter().map(|h| &h.value).collect();
        assert!(values.contains(&&Value::String("a".into())));
        assert!(values.contains(&&Value::String("b".into())));
    }

    #[test]
    fn test_polymeter() {
        // Create two patterns: one with 2 steps, one with 3 steps
        // LCM(2, 3) = 6, so pattern1 should play 3 times and pattern2 should play 2 times
        let pat1 = fastcat(vec![
            pure(Value::String("a".into())),
            pure(Value::String("b".into())),
        ]);
        let pat2 = fastcat(vec![
            pure(Value::String("c".into())),
            pure(Value::String("d".into())),
            pure(Value::String("e".into())),
        ]);

        let combined = polymeter(vec![pat1, pat2]);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));

        let haps = combined.query(state);

        // Should have 6 events total (3 + 3 from pattern1, and 2 + 2 + 2 from pattern2)
        // But stacked, so 12 total events
        assert!(haps.len() >= 6); // At least 6 because they're stacked

        // Verify we have events from both patterns
        let values: Vec<_> = haps.iter().map(|h| &h.value).collect();
        assert!(values.contains(&&Value::String("a".into())));
        assert!(values.contains(&&Value::String("c".into())));
    }

    #[test]
    fn test_choose_deterministic() {
        let pat1 = pure(Value::String("a".into()));
        let pat2 = pure(Value::String("b".into()));
        let pat3 = pure(Value::String("c".into()));

        let combined = choose(vec![pat1, pat2, pat3], 42);

        // Query same cycle multiple times - should get same result
        let state1 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps1 = combined.query(state1);

        let state2 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps2 = combined.query(state2);

        assert_eq!(haps1.len(), 1);
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps1[0].value, haps2[0].value);
    }

    #[test]
    fn test_choose_different_cycles() {
        let pat1 = pure(Value::String("a".into()));
        let pat2 = pure(Value::String("b".into()));
        let pat3 = pure(Value::String("c".into()));

        let combined = choose(vec![pat1, pat2, pat3], 42);

        // Query different cycles
        let state1 = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps1 = combined.query(state1);

        let state2 = State::new(TimeSpan::new(Fraction::from_int(1), Fraction::from_int(2)));
        let haps2 = combined.query(state2);

        let state3 = State::new(TimeSpan::new(Fraction::from_int(2), Fraction::from_int(3)));
        let haps3 = combined.query(state3);

        // Each should have exactly one event
        assert_eq!(haps1.len(), 1);
        assert_eq!(haps2.len(), 1);
        assert_eq!(haps3.len(), 1);

        // All should be valid values (a, b, or c)
        let valid_values = [
            Value::String("a".into()),
            Value::String("b".into()),
            Value::String("c".into()),
        ];
        assert!(valid_values.contains(&haps1[0].value));
        assert!(valid_values.contains(&haps2[0].value));
        assert!(valid_values.contains(&haps3[0].value));
    }
}
