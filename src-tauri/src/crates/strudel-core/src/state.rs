use crate::{TimeSpan, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// State represents the context for querying a pattern
///
/// It contains the timespan being queried and any control parameters
/// that might affect pattern generation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct State {
    /// The timespan being queried
    pub span: TimeSpan,

    /// Control parameters (e.g., tempo, effects, etc.)
    pub controls: HashMap<String, Value>,
}

impl State {
    /// Create a new State with the given timespan
    pub fn new(span: TimeSpan) -> Self {
        State {
            span,
            controls: HashMap::new(),
        }
    }

    /// Create a new State with the given timespan and controls
    pub fn with_controls(span: TimeSpan, controls: HashMap<String, Value>) -> Self {
        State { span, controls }
    }

    /// Return a new State with a different span
    pub fn set_span(&self, span: TimeSpan) -> State {
        State {
            span,
            controls: self.controls.clone(),
        }
    }

    /// Return a new State with the span modified by a function
    pub fn with_span<F>(&self, func: F) -> State
    where
        F: FnOnce(&TimeSpan) -> TimeSpan,
    {
        self.set_span(func(&self.span))
    }

    /// Return a new State with added/updated controls
    pub fn set_controls(&self, new_controls: HashMap<String, Value>) -> State {
        let mut controls = self.controls.clone();
        for (k, v) in new_controls {
            controls.insert(k, v);
        }
        State {
            span: self.span,
            controls,
        }
    }

    /// Return a new State with a single control added
    pub fn set_control(&self, key: String, value: Value) -> State {
        let mut controls = self.controls.clone();
        controls.insert(key, value);
        State {
            span: self.span,
            controls,
        }
    }

    /// Get a control value by key
    pub fn get_control(&self, key: &str) -> Option<&Value> {
        self.controls.get(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Fraction;

    #[test]
    fn test_state_creation() {
        let span = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let state = State::new(span);

        assert_eq!(state.span, span);
        assert!(state.controls.is_empty());
    }

    #[test]
    fn test_set_span() {
        let span1 = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let span2 = TimeSpan::new(Fraction::from_int(1), Fraction::from_int(2));

        let state1 = State::new(span1);
        let state2 = state1.set_span(span2);

        assert_eq!(state2.span, span2);
    }

    #[test]
    fn test_with_span() {
        let span = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let state = State::new(span);

        let new_state = state.with_span(|s| TimeSpan::new(s.begin + Fraction::from_int(1), s.end + Fraction::from_int(1)));

        assert_eq!(
            new_state.span,
            TimeSpan::new(Fraction::from_int(1), Fraction::from_int(2))
        );
    }

    #[test]
    fn test_controls() {
        let span = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let mut state = State::new(span);

        state = state.set_control("tempo".to_string(), Value::Number(120.0));

        assert_eq!(state.get_control("tempo"), Some(&Value::Number(120.0)));
    }

    #[test]
    fn test_set_controls() {
        let span = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let state = State::new(span);

        let mut controls = HashMap::new();
        controls.insert("tempo".to_string(), Value::Number(120.0));
        controls.insert("volume".to_string(), Value::Number(0.8));

        let new_state = state.set_controls(controls);

        assert_eq!(new_state.get_control("tempo"), Some(&Value::Number(120.0)));
        assert_eq!(
            new_state.get_control("volume"),
            Some(&Value::Number(0.8))
        );
    }
}
