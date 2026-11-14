#![forbid(unsafe_code)]

//! Bounded event-time intervals for evidence streams.
//!
//! A point timestamp is insufficient to prove temporal order when clocks,
//! acquisition pipelines or timestamp placement have known uncertainty.
//! This module admits an ordering only when the event-time intervals do not
//! overlap. It does not synchronize clocks or qualify a physical installation.

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundedEventTimeInterval {
    earliest_seconds: f64,
    latest_seconds: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TemporalIntervalError {
    NonFinite,
    NegativeTimestamp,
    NegativeUncertainty,
    IntervalUnderflow,
    IntervalOverflow,
}

impl BoundedEventTimeInterval {
    /// Construct [timestamp - uncertainty, timestamp + uncertainty].
    /// Invalid bounds are rejected; they are never clamped.
    pub fn new(timestamp_seconds: f64, uncertainty_seconds: f64) -> Result<Self, TemporalIntervalError> {
        if !timestamp_seconds.is_finite() || !uncertainty_seconds.is_finite() {
            return Err(TemporalIntervalError::NonFinite);
        }
        if timestamp_seconds < 0.0 {
            return Err(TemporalIntervalError::NegativeTimestamp);
        }
        if uncertainty_seconds < 0.0 {
            return Err(TemporalIntervalError::NegativeUncertainty);
        }
        let earliest_seconds = timestamp_seconds - uncertainty_seconds;
        let latest_seconds = timestamp_seconds + uncertainty_seconds;
        if !earliest_seconds.is_finite() || !latest_seconds.is_finite() {
            return Err(TemporalIntervalError::IntervalOverflow);
        }
        if earliest_seconds < 0.0 {
            return Err(TemporalIntervalError::IntervalUnderflow);
        }
        Ok(Self { earliest_seconds, latest_seconds })
    }

    pub fn earliest_seconds(self) -> f64 {
        self.earliest_seconds
    }

    pub fn latest_seconds(self) -> f64 {
        self.latest_seconds
    }

    /// True only when every possible time in this interval is strictly after
    /// every possible time in the previous interval.
    pub fn is_strictly_after(self, previous: Self) -> bool {
        self.earliest_seconds > previous.latest_seconds
    }

    /// True only when every possible time in this interval is strictly before
    /// every possible time in the other interval.
    pub fn is_strictly_before(self, other: Self) -> bool {
        self.latest_seconds < other.earliest_seconds
    }

    /// Inclusive intervals that overlap or touch do not establish ordering.
    pub fn overlaps_or_touches(self, other: Self) -> bool {
        !(self.is_strictly_before(other) || other.is_strictly_before(self))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn interval_is_conservative_and_inclusive() {
        let a = BoundedEventTimeInterval::new(1.0, 0.01).unwrap();
        let b = BoundedEventTimeInterval::new(1.03, 0.01).unwrap();
        assert!((a.earliest_seconds() - 0.99).abs() < 1e-12);
        assert!((a.latest_seconds() - 1.01).abs() < 1e-12);
        assert!(b.is_strictly_after(a));
    }

    #[test]
    fn overlapping_or_touching_intervals_do_not_establish_order() {
        let a = BoundedEventTimeInterval::new(1.0, 0.01).unwrap();
        let overlap = BoundedEventTimeInterval::new(1.015, 0.01).unwrap();
        let touching = BoundedEventTimeInterval::new(1.02, 0.01).unwrap();
        assert!(a.overlaps_or_touches(overlap));
        assert!(a.overlaps_or_touches(touching));
        assert!(!overlap.is_strictly_after(a));
        assert!(!touching.is_strictly_after(a));
    }

    #[test]
    fn timestamp_underflow_overflow_and_invalid_uncertainty_are_rejected() {
        assert_eq!(
            BoundedEventTimeInterval::new(0.0, 0.1),
            Err(TemporalIntervalError::IntervalUnderflow)
        );
        assert_eq!(
            BoundedEventTimeInterval::new(f64::MAX, f64::MAX),
            Err(TemporalIntervalError::IntervalOverflow)
        );
        assert_eq!(
            BoundedEventTimeInterval::new(1.0, -0.1),
            Err(TemporalIntervalError::NegativeUncertainty)
        );
        assert_eq!(
            BoundedEventTimeInterval::new(f64::NAN, 0.0),
            Err(TemporalIntervalError::NonFinite)
        );
    }

    #[test]
    fn wider_uncertainty_never_creates_stronger_ordering() {
        let earlier = BoundedEventTimeInterval::new(1.0, 0.01).unwrap();
        let later = BoundedEventTimeInterval::new(1.05, 0.01).unwrap();
        assert!(later.is_strictly_after(earlier));
        let widened_later = BoundedEventTimeInterval::new(1.05, 0.04).unwrap();
        assert!(!widened_later.is_strictly_after(earlier));
    }
}
