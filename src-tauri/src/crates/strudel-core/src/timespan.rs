use crate::Fraction;
use serde::{Deserialize, Serialize};
use std::fmt;

/// Represents a span of time from begin to end
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct TimeSpan {
    pub begin: Fraction,
    pub end: Fraction,
}

impl TimeSpan {
    /// Create a new timespan
    pub fn new(begin: Fraction, end: Fraction) -> Self {
        TimeSpan { begin, end }
    }

    /// Create a timespan from two integers (whole numbers)
    pub fn from_ints(begin: i64, end: i64) -> Self {
        TimeSpan {
            begin: Fraction::from_int(begin),
            end: Fraction::from_int(end),
        }
    }

    /// Create a timespan from two floats
    pub fn from_floats(begin: f64, end: f64) -> Self {
        TimeSpan {
            begin: Fraction::from_float(begin),
            end: Fraction::from_float(end),
        }
    }

    /// Get the duration of this timespan
    pub fn duration(&self) -> Fraction {
        self.end - self.begin
    }

    /// Check if this timespan contains a point in time
    pub fn contains(&self, time: Fraction) -> bool {
        time >= self.begin && time < self.end
    }

    /// Check if two timespans overlap
    pub fn overlaps(&self, other: &TimeSpan) -> bool {
        self.begin < other.end && other.begin < self.end
    }

    /// Get the intersection of two timespans, if any
    pub fn intersection(&self, other: &TimeSpan) -> Option<TimeSpan> {
        if !self.overlaps(other) {
            return None;
        }
        Some(TimeSpan::new(
            self.begin.max(other.begin),
            self.end.min(other.end),
        ))
    }

    /// Get the midpoint of the timespan
    pub fn midpoint(&self) -> Fraction {
        (self.begin + self.end) * Fraction::new(1, 2)
    }

    /// Check if the timespan is empty (begin == end)
    pub fn is_empty(&self) -> bool {
        self.begin == self.end
    }

    /// Shift the timespan by an offset
    pub fn shift(&self, offset: Fraction) -> TimeSpan {
        TimeSpan::new(self.begin + offset, self.end + offset)
    }

    /// Scale the timespan by a factor
    pub fn scale(&self, factor: Fraction) -> TimeSpan {
        TimeSpan::new(self.begin * factor, self.end * factor)
    }
}

impl fmt::Display for TimeSpan {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} - {}", self.begin, self.end)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_timespan_creation() {
        let ts = TimeSpan::from_ints(0, 1);
        assert_eq!(ts.begin, Fraction::from_int(0));
        assert_eq!(ts.end, Fraction::from_int(1));
    }

    #[test]
    fn test_timespan_duration() {
        let ts = TimeSpan::from_ints(0, 2);
        assert_eq!(ts.duration(), Fraction::from_int(2));
    }

    #[test]
    fn test_timespan_contains() {
        let ts = TimeSpan::from_ints(0, 1);
        assert!(ts.contains(Fraction::new(1, 2)));
        assert!(!ts.contains(Fraction::from_int(2)));
    }

    #[test]
    fn test_timespan_overlap() {
        let ts1 = TimeSpan::from_ints(0, 2);
        let ts2 = TimeSpan::from_ints(1, 3);
        assert!(ts1.overlaps(&ts2));

        let ts3 = TimeSpan::from_ints(3, 4);
        assert!(!ts1.overlaps(&ts3));
    }

    #[test]
    fn test_timespan_intersection() {
        let ts1 = TimeSpan::from_ints(0, 2);
        let ts2 = TimeSpan::from_ints(1, 3);
        let intersection = ts1.intersection(&ts2).unwrap();
        assert_eq!(intersection.begin, Fraction::from_int(1));
        assert_eq!(intersection.end, Fraction::from_int(2));
    }
}
