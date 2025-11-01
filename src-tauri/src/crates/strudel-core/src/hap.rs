use crate::{Fraction, TimeSpan, Value};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Context metadata for a Hap (event)
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    /// Source code locations causing this event
    pub locations: Vec<String>,
    /// Additional metadata
    pub metadata: HashMap<String, Value>,
}

impl Context {
    /// Create an empty context
    pub fn new() -> Self {
        Context {
            locations: Vec::new(),
            metadata: HashMap::new(),
        }
    }

    /// Combine two contexts, merging locations and metadata
    pub fn combine(&self, other: &Context) -> Context {
        let mut locations = self.locations.clone();
        locations.extend(other.locations.clone());

        let mut metadata = self.metadata.clone();
        for (k, v) in &other.metadata {
            metadata.insert(k.clone(), v.clone());
        }

        Context {
            locations,
            metadata,
        }
    }
}

impl Default for Context {
    fn default() -> Self {
        Self::new()
    }
}

/// A Hap (Happening/Event) represents a value active during a timespan
///
/// The 'part' is the timespan fragment of this event, which may be smaller
/// than the 'whole' timespan if the event is fragmented. The 'part' must
/// never extend outside of the 'whole'. If the event represents a continuously
/// changing value, then 'whole' will be None.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Hap {
    /// The full timespan of the event (may be None for continuous events)
    pub whole: Option<TimeSpan>,

    /// The active fragment timespan (always present)
    pub part: TimeSpan,

    /// The value of this event
    pub value: Value,

    /// Metadata context for this event
    pub context: Context,
}

impl Hap {
    /// Create a new Hap with the given timespans and value
    pub fn new(whole: Option<TimeSpan>, part: TimeSpan, value: Value) -> Self {
        Hap {
            whole,
            part,
            value,
            context: Context::new(),
        }
    }

    /// Create a new Hap with context
    pub fn with_context(
        whole: Option<TimeSpan>,
        part: TimeSpan,
        value: Value,
        context: Context,
    ) -> Self {
        Hap {
            whole,
            part,
            value,
            context,
        }
    }

    /// Get the whole timespan or fall back to part
    pub fn whole_or_part(&self) -> TimeSpan {
        self.whole.unwrap_or(self.part)
    }

    /// Check if this hap contains the onset (beginning of whole matches beginning of part)
    pub fn has_onset(&self) -> bool {
        match self.whole {
            Some(w) => w.begin == self.part.begin,
            None => false,
        }
    }

    /// Apply a function to the value, returning a new Hap
    pub fn with_value<F>(&self, func: F) -> Hap
    where
        F: FnOnce(&Value) -> Value,
    {
        Hap {
            whole: self.whole,
            part: self.part,
            value: func(&self.value),
            context: self.context.clone(),
        }
    }

    /// Apply a function to the timespans, returning a new Hap
    pub fn with_span<F>(&self, func: F) -> Hap
    where
        F: Fn(&TimeSpan) -> TimeSpan,
    {
        Hap {
            whole: self.whole.map(|w| func(&w)),
            part: func(&self.part),
            value: self.value.clone(),
            context: self.context.clone(),
        }
    }

    /// Set the context for this Hap
    pub fn set_context(&self, context: Context) -> Hap {
        Hap {
            whole: self.whole,
            part: self.part,
            value: self.value.clone(),
            context,
        }
    }

    /// Combine context with another Hap's context
    pub fn combine_context(&self, other: &Hap) -> Context {
        self.context.combine(&other.context)
    }

    /// Check if this hap's whole timespan equals another's
    pub fn span_equals(&self, other: &Hap) -> bool {
        match (self.whole, other.whole) {
            (None, None) => true,
            (Some(a), Some(b)) => a == b,
            _ => false,
        }
    }

    /// Check if this hap is completely equal to another
    pub fn equals(&self, other: &Hap) -> bool {
        self.span_equals(other) && self.part == other.part && self.value == other.value
    }

    /// Get the duration of this event
    pub fn duration(&self) -> Fraction {
        match self.whole {
            Some(w) => w.end - w.begin,
            None => self.part.end - self.part.begin,
        }
    }

    /// Check if this event is active at a given time
    pub fn is_active(&self, time: Fraction) -> bool {
        let w = self.whole_or_part();
        w.begin <= time && w.end >= time
    }

    /// Check if this event is in the past relative to a given time
    pub fn is_in_past(&self, time: Fraction) -> bool {
        let w = self.whole_or_part();
        time > w.end
    }

    /// Check if this event is in the future relative to a given time
    pub fn is_in_future(&self, time: Fraction) -> bool {
        let w = self.whole_or_part();
        time < w.begin
    }

    /// Check if this event is within a time range
    pub fn is_within_time(&self, min: Fraction, max: Fraction) -> bool {
        let w = self.whole_or_part();
        w.begin <= max && w.end >= min
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hap_creation() {
        let ts = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let hap = Hap::new(Some(ts), ts, Value::Number(42.0));

        assert_eq!(hap.value, Value::Number(42.0));
        assert_eq!(hap.whole, Some(ts));
        assert_eq!(hap.part, ts);
    }

    #[test]
    fn test_has_onset() {
        let whole = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let part = TimeSpan::new(Fraction::from_int(0), Fraction::new(1, 2));

        let hap = Hap::new(Some(whole), part, Value::String("test".into()));
        assert!(hap.has_onset());

        let part_no_onset = TimeSpan::new(Fraction::new(1, 4), Fraction::new(1, 2));
        let hap_no_onset = Hap::new(Some(whole), part_no_onset, Value::String("test".into()));
        assert!(!hap_no_onset.has_onset());
    }

    #[test]
    fn test_with_value() {
        let ts = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let hap = Hap::new(Some(ts), ts, Value::Number(10.0));

        let new_hap = hap.with_value(|v| match v {
            Value::Number(n) => Value::Number(n + 5.0),
            _ => v.clone(),
        });

        assert_eq!(new_hap.value, Value::Number(15.0));
    }

    #[test]
    fn test_context_combine() {
        let mut ctx1 = Context::new();
        ctx1.locations.push("loc1".to_string());

        let mut ctx2 = Context::new();
        ctx2.locations.push("loc2".to_string());

        let combined = ctx1.combine(&ctx2);
        assert_eq!(combined.locations.len(), 2);
        assert!(combined.locations.contains(&"loc1".to_string()));
        assert!(combined.locations.contains(&"loc2".to_string()));
    }

    #[test]
    fn test_is_active() {
        let ts = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(1));
        let hap = Hap::new(Some(ts), ts, Value::Number(1.0));

        assert!(hap.is_active(Fraction::new(1, 2)));
        assert!(!hap.is_active(Fraction::from_int(2)));
    }

    #[test]
    fn test_duration() {
        let ts = TimeSpan::new(Fraction::from_int(0), Fraction::from_int(2));
        let hap = Hap::new(Some(ts), ts, Value::Number(1.0));

        assert_eq!(hap.duration(), Fraction::from_int(2));
    }
}
