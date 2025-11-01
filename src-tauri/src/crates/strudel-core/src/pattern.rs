use crate::{Fraction, Hap, State, TimeSpan, Value};
use std::sync::Arc;

/// A Pattern represents a time-varying sequence of values
///
/// Patterns are queried with a State (containing a timespan) and return
/// a list of Haps (events) that occur within that timespan.
pub struct Pattern {
    /// The query function that produces events for a given state
    query_func: Arc<dyn Fn(State) -> Vec<Hap> + Send + Sync>,

    /// Optional step count (number of steps per cycle)
    steps: Option<Fraction>,
}

impl Pattern {
    /// Create a new Pattern with a query function
    pub fn new<F>(query_func: F) -> Self
    where
        F: Fn(State) -> Vec<Hap> + Send + Sync + 'static,
    {
        Pattern {
            query_func: Arc::new(query_func),
            steps: None,
        }
    }

    /// Create a new Pattern with a query function and steps
    pub fn with_steps<F>(query_func: F, steps: Option<Fraction>) -> Self
    where
        F: Fn(State) -> Vec<Hap> + Send + Sync + 'static,
    {
        Pattern {
            query_func: Arc::new(query_func),
            steps,
        }
    }

    /// Query this pattern with the given state
    pub fn query(&self, state: State) -> Vec<Hap> {
        (self.query_func)(state)
    }

    /// Get the steps for this pattern
    pub fn get_steps(&self) -> Option<Fraction> {
        self.steps
    }

    /// Set the steps for this pattern
    pub fn set_steps(mut self, steps: Option<Fraction>) -> Self {
        self.steps = steps;
        self
    }

    /// Apply a function to each value in the pattern
    ///
    /// This is the functor map operation (fmap)
    pub fn with_value<F>(self, func: F) -> Pattern
    where
        F: Fn(&crate::Value) -> crate::Value + Send + Sync + 'static,
    {
        let query_func = self.query_func.clone();
        let steps = self.steps;

        Pattern {
            query_func: Arc::new(move |state| {
                query_func(state)
                    .into_iter()
                    .map(|hap| hap.with_value(&func))
                    .collect()
            }),
            steps,
        }
    }

    /// Apply a function to query time (before querying)
    pub fn with_query_time<F>(self, func: F) -> Pattern
    where
        F: Fn(Fraction) -> Fraction + Send + Sync + 'static + Copy,
    {
        let query_func = self.query_func.clone();
        let steps = self.steps;

        Pattern {
            query_func: Arc::new(move |state| {
                let new_span = TimeSpan::new(func(state.span.begin), func(state.span.end));
                query_func(state.set_span(new_span))
            }),
            steps,
        }
    }

    /// Apply a function to hap time (after querying)
    pub fn with_hap_time<F>(self, func: F) -> Pattern
    where
        F: Fn(Fraction) -> Fraction + Send + Sync + 'static + Copy,
    {
        let query_func = self.query_func.clone();
        let steps = self.steps;

        Pattern {
            query_func: Arc::new(move |state| {
                query_func(state)
                    .into_iter()
                    .map(|hap| {
                        hap.with_span(|ts| TimeSpan::new(func(ts.begin), func(ts.end)))
                    })
                    .collect()
            }),
            steps,
        }
    }

    /// Apply a function to each hap
    pub fn with_hap<F>(self, func: F) -> Pattern
    where
        F: Fn(&Hap) -> Hap + Send + Sync + 'static,
    {
        let query_func = self.query_func.clone();
        let steps = self.steps;

        Pattern {
            query_func: Arc::new(move |state| {
                query_func(state).into_iter().map(|hap| func(&hap)).collect()
            }),
            steps,
        }
    }

    /// Apply a function to all haps at once
    pub fn with_haps<F>(self, func: F) -> Pattern
    where
        F: Fn(Vec<Hap>) -> Vec<Hap> + Send + Sync + 'static,
    {
        let query_func = self.query_func.clone();
        let steps = self.steps;

        Pattern {
            query_func: Arc::new(move |state| func(query_func(state))),
            steps,
        }
    }

    /// Split queries at cycle boundaries
    ///
    /// This is useful for patterns that need to know about cycle boundaries
    pub fn split_queries(self) -> Pattern {
        let query_func = self.query_func.clone();
        let steps = self.steps;

        Pattern {
            query_func: Arc::new(move |state| {
                let span = state.span;
                let begin_cycle = span.begin.floor();
                let end_cycle = span.end.ceil();

                let mut all_haps = Vec::new();

                let mut cycle = begin_cycle;
                while cycle < end_cycle {
                    let cycle_begin = if cycle < span.begin {
                        span.begin
                    } else {
                        cycle
                    };

                    let cycle_end = if cycle + Fraction::from_int(1) > span.end {
                        span.end
                    } else {
                        cycle + Fraction::from_int(1)
                    };

                    let cycle_span = TimeSpan::new(cycle_begin, cycle_end);
                    let cycle_state = state.set_span(cycle_span);
                    all_haps.extend(query_func(cycle_state));

                    cycle = cycle + Fraction::from_int(1);
                }

                all_haps
            }),
            steps,
        }
    }

    /// Speed up the pattern by a constant factor
    ///
    /// Multiplies the query time by the factor and divides hap times
    pub fn fast(self, factor: f64) -> Pattern {
        let frac_factor = Fraction::from_float(factor);
        self.with_query_time(move |t| t * frac_factor)
            .with_hap_time(move |t| t / frac_factor)
    }

    /// Slow down the pattern by a constant factor
    ///
    /// Divides the query time by the factor and multiplies hap times
    pub fn slow(self, factor: f64) -> Pattern {
        let frac_factor = Fraction::from_float(factor);
        self.with_query_time(move |t| t / frac_factor)
            .with_hap_time(move |t| t * frac_factor)
    }

    /// Repeat each cycle n times
    ///
    /// Each source cycle is repeated n times before moving to the next.
    /// Matches Strudel's repeatCycles implementation.
    pub fn repeat_cycles(self, n: usize) -> Pattern {
        if n <= 1 {
            return self;
        }

        let query_func = self.query_func.clone();
        let steps = self.steps;
        let n_frac = Fraction::from_int(n as i64);

        Pattern {
            query_func: Arc::new(move |state| {
                // Get cycle number (floor of begin)
                let cycle = state.span.begin.floor();

                // Calculate which source cycle to use
                let source_cycle = cycle / n_frac;
                let source_cycle_floor = source_cycle.floor();

                // Calculate delta (offset from source cycle)
                let delta = cycle - source_cycle_floor;

                // Query pattern with adjusted span (subtract delta)
                let adjusted_state = state.with_span(|span| {
                    TimeSpan::new(span.begin - delta, span.end - delta)
                });

                // Query and shift results back (add delta)
                query_func(adjusted_state)
                    .into_iter()
                    .map(|hap| {
                        hap.with_span(|span| {
                            TimeSpan::new(span.begin + delta, span.end + delta)
                        })
                    })
                    .collect()
            }),
            steps,
        }
        .split_queries()
    }

    /// Replicate pattern n times within same timespan
    ///
    /// Implemented as repeatCycles(n).fast(n) to match Strudel.
    /// This repeats the entire pattern n times and speeds it up by n.
    pub fn replicate(self, n: usize) -> Pattern {
        if n == 0 {
            return Pattern::new(|_| Vec::new());
        }

        if n == 1 {
            return self;
        }

        self.repeat_cycles(n).fast(n as f64)
    }

    /// Apply a Euclidean rhythm pattern
    ///
    /// Filters events based on the Bjorklund algorithm distribution
    pub fn euclid(self, pulse: usize, step: usize, rotation: Option<usize>) -> Pattern {
        let rot = rotation.unwrap_or(0);
        let rhythm = crate::euclid::bjorklund(pulse, step, rot);

        if rhythm.is_empty() {
            return Pattern::new(|_| Vec::new());
        }

        self.split_queries().with_haps(move |haps| {
            haps.into_iter()
                .enumerate()
                .filter(|(i, _)| rhythm[i % rhythm.len()])
                .map(|(_, hap)| hap)
                .collect()
        })
    }

    /// Randomly remove events with a given probability
    ///
    /// # Arguments
    /// * `amount` - Probability of removing each event (0.0 = keep all, 1.0 = remove all)
    /// * `seed` - Random seed for reproducibility
    pub fn degrade_by(self, amount: f64, seed: u64) -> Pattern {
        use rand::{Rng, SeedableRng};
        use rand::rngs::StdRng;

        self.with_haps(move |haps| {
            let mut rng = StdRng::seed_from_u64(seed);
            haps.into_iter()
                .filter(|_| rng.gen::<f64>() > amount)
                .collect()
        })
    }

    /// Randomly remove 50% of events
    pub fn degrade(self) -> Pattern {
        self.degrade_by(0.5, 0)
    }

    /// Map numeric values to a musical scale
    ///
    /// This is a simplified implementation that stores the scale name in context
    /// and maps numbers to notes. In the future, this could be enhanced with
    /// a proper music theory library.
    ///
    /// # Arguments
    /// * `scale_name` - Name of the scale (e.g., "C:major", "D:minor")
    ///
    /// For now, this implementation:
    /// - Stores the scale name in the event context
    /// - Converts numeric values to string notes based on a simple major scale
    pub fn scale(self, scale_name: String) -> Pattern {
        self.with_hap(move |hap| {
            // Store scale in context
            let mut new_context = hap.context.clone();
            new_context.metadata.insert("scale".to_string(), Value::String(scale_name.clone()));

            // If value is a number, map it to a note in the scale
            let new_value = match &hap.value {
                Value::Number(n) => {
                    // Simple major scale mapping: C D E F G A B
                    // This is a basic implementation - could be enhanced with proper scale theory
                    let note_names = ["C", "D", "E", "F", "G", "A", "B"];
                    let step = (*n as i32).rem_euclid(note_names.len() as i32) as usize;
                    let octave = (*n as i32).div_euclid(note_names.len() as i32) + 3; // Start at octave 3
                    Value::String(format!("{}{}", note_names[step], octave))
                }
                _ => hap.value.clone(),
            };

            Hap::with_context(hap.whole, hap.part, new_value, new_context)
        })
    }

    /// Apply a boolean/rhythm structure pattern to this pattern
    ///
    /// The structure pattern determines when events from this pattern should occur.
    /// Events are only kept where the structure pattern has truthy values.
    ///
    /// This is similar to TidalCycles/Strudel's `struct` function, which applies
    /// a rhythmic structure to a pattern. The structure pattern provides the timing,
    /// while this pattern provides the values.
    ///
    /// # Arguments
    /// * `structure` - A pattern that provides the rhythmic structure. Events from
    ///   this pattern are kept only where the structure has truthy values (non-zero
    ///   numbers, non-empty strings, true booleans, etc.)
    ///
    /// # Example
    /// ```
    /// use strudel_core::{pure, fastcat, Value};
    ///
    /// // Apply a simple on-off rhythm
    /// let values = fastcat(vec![
    ///     pure(Value::String("a".into())),
    ///     pure(Value::String("b".into())),
    ///     pure(Value::String("c".into())),
    ///     pure(Value::String("d".into())),
    /// ]);
    ///
    /// let structure = fastcat(vec![
    ///     pure(Value::Number(1.0)),  // keep
    ///     pure(Value::Number(0.0)),  // skip
    ///     pure(Value::Number(1.0)),  // keep
    ///     pure(Value::Number(0.0)),  // skip
    /// ]);
    ///
    /// let result = values.struct_(structure);
    /// // Result will only have events at positions 0 and 2
    /// ```
    pub fn struct_(self, structure: Pattern) -> Pattern {
        let value_pattern = self;
        let structure_pattern = structure;

        Pattern::new(move |state| {
            let mut result_haps = Vec::new();

            // Query the structure pattern to get the timing information
            let structure_haps = structure_pattern.query(state.clone());

            // For each structure hap, query the value pattern at that timespan
            for structure_hap in structure_haps {
                // Check if the structure value is truthy
                if is_truthy(&structure_hap.value) {
                    // Query the value pattern for this timespan
                    let value_state = state.set_span(structure_hap.whole_or_part());
                    let value_haps = value_pattern.query(value_state);

                    // For each value hap that intersects with the structure hap
                    for value_hap in value_haps {
                        if let Some(new_part) = structure_hap.part.intersection(&value_hap.part) {
                            // Create a new hap with the structure's whole timespan
                            // but the value from the value pattern
                            let new_hap = Hap::new(
                                structure_hap.whole,
                                new_part,
                                value_hap.value.clone(),
                            );
                            result_haps.push(new_hap);
                        }
                    }
                }
            }

            result_haps
        })
    }

    /// Append another pattern to this one (list cons operator)
    ///
    /// Creates a sequence where this pattern is followed by the given pattern.
    /// Similar to fastcat([self, other]) - concatenates patterns within a cycle.
    /// This implements the ":" operator from mini notation (e.g., "a:b" becomes [a, b]).
    pub fn tail(self, other: Pattern) -> Pattern {
        crate::fastcat(vec![self, other])
    }

    /// Shift pattern in time (nudge forward or backward)
    ///
    /// Positive values shift the pattern later in time (equivalent to `late`).
    /// Negative values shift the pattern earlier in time (equivalent to `early`).
    ///
    /// This is implemented by:
    /// - Subtracting the offset from query time (asking for earlier/later events)
    /// - Adding the offset to hap time (placing events earlier/later in output)
    ///
    /// # Arguments
    /// * `amount` - Number of cycles to shift. Positive = later, negative = earlier
    ///
    /// # Example
    /// ```
    /// use strudel_core::{pure, Value, State, TimeSpan, Fraction};
    ///
    /// // Shift pattern 0.25 cycles later
    /// let pattern = pure(Value::String("bd".into())).split_queries().shift(0.25);
    /// let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
    /// let haps = pattern.query(state);
    ///
    /// // Event that was at 0.0 is now at 0.25
    /// assert_eq!(haps[0].part.begin, Fraction::new(1, 4));
    /// ```
    pub fn shift(self, amount: f64) -> Pattern {
        let frac_amount = Fraction::from_float(amount);

        // Shift works like late: subtract from query time, add to hap time
        // This makes positive values shift the pattern later
        self.with_query_time(move |t| t - frac_amount)
            .with_hap_time(move |t| t + frac_amount)
    }

    /// Nudge pattern earlier in time (equivalent to shift with negative value)
    ///
    /// This is the same as Strudel/TidalCycles' `early` function or `<~` operator.
    ///
    /// # Arguments
    /// * `amount` - Number of cycles to shift earlier (positive value shifts earlier)
    ///
    /// # Example
    /// ```
    /// use strudel_core::{pure, Value};
    ///
    /// // These are equivalent:
    /// let pattern1 = pure(Value::String("bd".into())).early(0.25);
    /// let pattern2 = pure(Value::String("bd".into())).shift(-0.25);
    /// ```
    pub fn early(self, amount: f64) -> Pattern {
        self.shift(-amount)
    }

    /// Nudge pattern later in time (equivalent to shift with positive value)
    ///
    /// This is the same as Strudel/TidalCycles' `late` function or `~>` operator.
    ///
    /// # Arguments
    /// * `amount` - Number of cycles to shift later (positive value shifts later)
    ///
    /// # Example
    /// ```
    /// use strudel_core::{pure, Value};
    ///
    /// // These are equivalent:
    /// let pattern1 = pure(Value::String("bd".into())).late(0.25);
    /// let pattern2 = pure(Value::String("bd".into())).shift(0.25);
    /// ```
    pub fn late(self, amount: f64) -> Pattern {
        self.shift(amount)
    }

    /// Set target destination for pattern events
    ///
    /// Adds a "target" metadata entry to each event's context, indicating where
    /// the event should be routed (e.g., to specific OSC targets, orbits, or MIDI devices).
    ///
    /// This corresponds to the `target` operator in Strudel/TidalCycles mini notation,
    /// which is used like: `target "mysynth" $ pattern`
    ///
    /// # Arguments
    /// * `target_name` - Name of the target destination
    ///
    /// # Example
    /// ```
    /// use strudel_core::{pure, Value};
    ///
    /// // Route pattern to a specific target
    /// let pattern = pure(Value::String("bd".into())).target("drums".to_string());
    ///
    /// // Multiple patterns can have different targets
    /// let synth_pattern = pure(Value::String("c4".into())).target("synth".to_string());
    /// let drums_pattern = pure(Value::String("bd".into())).target("drums".to_string());
    /// ```
    pub fn target(self, target_name: String) -> Pattern {
        self.with_hap(move |hap| {
            let mut new_context = hap.context.clone();
            new_context.metadata.insert("target".to_string(), Value::String(target_name.clone()));
            Hap::with_context(hap.whole, hap.part, hap.value.clone(), new_context)
        })
    }
}

/// Helper function to determine if a value is "truthy"
/// In the context of struct, we consider:
/// - Numbers: 0 is false, anything else is true
/// - Strings: empty string is false, anything else is true (including "~")
/// - Booleans: use their boolean value
/// - Lists: empty list is false, non-empty is true
/// - Silence: false
fn is_truthy(value: &crate::Value) -> bool {
    match value {
        crate::Value::Number(n) => *n != 0.0,
        crate::Value::String(s) => !s.is_empty() && s != "~",
        crate::Value::Bool(b) => *b,
        crate::Value::List(l) => !l.is_empty(),
        crate::Value::Silence => false,
    }
}

// Implement Clone for Pattern
impl Clone for Pattern {
    fn clone(&self) -> Self {
        Pattern {
            query_func: self.query_func.clone(),
            steps: self.steps,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Value;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new(|_state| Vec::new());
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));

        let haps = pattern.query(state);
        assert_eq!(haps.len(), 0);
    }

    #[test]
    fn test_pattern_with_value() {
        let pattern = Pattern::new(|state| {
            vec![Hap::new(
                Some(state.span),
                state.span,
                Value::Number(10.0),
            )]
        });

        let mapped = pattern.with_value(|v| match v {
            Value::Number(n) => Value::Number(n + 5.0),
            _ => v.clone(),
        });

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = mapped.query(state);

        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::Number(15.0));
    }

    #[test]
    fn test_pattern_with_steps() {
        let pattern =
            Pattern::with_steps(|_| Vec::new(), Some(Fraction::from_int(4)));

        assert_eq!(pattern.get_steps(), Some(Fraction::from_int(4)));
    }

    #[test]
    fn test_pattern_with_hap() {
        let pattern = Pattern::new(|state| {
            vec![Hap::new(
                Some(state.span),
                state.span,
                Value::String("test".into()),
            )]
        });

        let modified = pattern.with_hap(|hap| {
            hap.with_value(|_| Value::String("modified".into()))
        });

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = modified.query(state);

        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::String("modified".into()));
    }

    #[test]
    fn test_repeat_cycles() {
        // Test based on Strudel's test: slowcat(0, 1).repeatCycles(2).fast(6).firstCycleValues
        // should give [0, 0, 1, 1, 0, 0]
        use crate::slowcat;

        let pattern = slowcat(vec![
            Pattern::new(|state| {
                vec![Hap::new(Some(state.span), state.span, Value::Number(0.0))]
            }),
            Pattern::new(|state| {
                vec![Hap::new(Some(state.span), state.span, Value::Number(1.0))]
            }),
        ]);

        let repeated = pattern.repeat_cycles(2).fast(6.0);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = repeated.query(state);

        // Should have 6 events
        assert_eq!(haps.len(), 6);

        // Extract values
        let values: Vec<f64> = haps
            .iter()
            .map(|h| match &h.value {
                Value::Number(n) => *n,
                _ => panic!("Expected number"),
            })
            .collect();

        // Should be [0, 0, 1, 1, 0, 0]
        assert_eq!(values, vec![0.0, 0.0, 1.0, 1.0, 0.0, 0.0]);
    }

    #[test]
    fn test_replicate() {
        use crate::fastcat;

        // Create pattern [bd sd]
        let pattern = fastcat(vec![
            Pattern::new(|state| {
                vec![Hap::new(
                    Some(state.span),
                    state.span,
                    Value::String("bd".into()),
                )]
            }),
            Pattern::new(|state| {
                vec![Hap::new(
                    Some(state.span),
                    state.span,
                    Value::String("sd".into()),
                )]
            }),
        ]);

        // Replicate 2 times
        let replicated = pattern.replicate(2);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = replicated.query(state);

        // Should have 4 events: bd sd bd sd
        assert_eq!(haps.len(), 4);

        let values: Vec<String> = haps
            .iter()
            .map(|h| match &h.value {
                Value::String(s) => s.clone(),
                _ => panic!("Expected string"),
            })
            .collect();

        assert_eq!(values, vec!["bd", "sd", "bd", "sd"]);

        // Check timing - each should be 1/4 cycle
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
    fn test_tail() {
        use crate::pure;

        // Create two simple patterns
        let pattern_a = pure(Value::String("a".into()));
        let pattern_b = pure(Value::String("b".into()));

        // Use tail to concatenate them
        let combined = pattern_a.tail(pattern_b);
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = combined.query(state);

        // Should have 2 events: a in first half, b in second half
        assert_eq!(haps.len(), 2);
        assert_eq!(haps[0].value, Value::String("a".into()));
        assert_eq!(haps[1].value, Value::String("b".into()));

        // Check timing
        assert_eq!(haps[0].part.begin, Fraction::from_int(0));
        assert_eq!(haps[0].part.end, Fraction::new(1, 2));
        assert_eq!(haps[1].part.begin, Fraction::new(1, 2));
        assert_eq!(haps[1].part.end, Fraction::from_int(1));
    }

    #[test]
    fn test_target() {
        use crate::pure;

        // Create a simple pattern
        let pattern = pure(Value::String("bd".into()));

        // Apply target
        let targeted = pattern.target("drums".to_string());
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = targeted.query(state);

        // Should have 1 event with target metadata
        assert_eq!(haps.len(), 1);
        assert_eq!(haps[0].value, Value::String("bd".into()));
        assert_eq!(
            haps[0].context.metadata.get("target"),
            Some(&Value::String("drums".into()))
        );
    }

    #[test]
    fn test_target_with_fastcat() {
        use crate::fastcat;
        use crate::pure;

        // Create a sequence of patterns
        let pattern = fastcat(vec![
            pure(Value::String("bd".into())),
            pure(Value::String("sd".into())),
            pure(Value::String("cp".into())),
        ]);

        // Apply target to the whole sequence
        let targeted = pattern.target("percussion".to_string());
        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));
        let haps = targeted.query(state);

        // Should have 3 events, all with the same target
        assert_eq!(haps.len(), 3);

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
    fn test_target_doesnt_affect_timing() {
        use crate::fastcat;
        use crate::pure;

        // Create a pattern
        let pattern = fastcat(vec![
            pure(Value::String("a".into())),
            pure(Value::String("b".into())),
        ]);

        let state = State::new(TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1)));

        // Query without target
        let haps_no_target = pattern.clone().query(state.clone());

        // Query with target
        let haps_with_target = pattern.target("test".to_string()).query(state);

        // Should have same timing
        assert_eq!(haps_no_target.len(), haps_with_target.len());
        for i in 0..haps_no_target.len() {
            assert_eq!(haps_no_target[i].part, haps_with_target[i].part);
            assert_eq!(haps_no_target[i].whole, haps_with_target[i].whole);
            assert_eq!(haps_no_target[i].value, haps_with_target[i].value);
        }
    }
}
