//! Time-bounded evidence provenance and explicit missingness.
//!
//! Stored evidence records when it was sampled. "Current", "held", or "stale"
//! are decision-time classifications derived from provenance, the decision tick,
//! and a profile/policy; they are not persisted here as eternal truth.

use crate::{AggregationMethodId, Tick, TickArithmeticError};

/// Provenance for a present sample.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum SampleProvenance {
    /// A sample observed at one exact canonical tick.
    Instant { sampled_tick: Tick },
    /// A deterministic aggregate over an inclusive canonical tick window.
    Window {
        start_tick: Tick,
        end_tick: Tick,
        method_id: AggregationMethodId,
    },
}

/// Invalid aggregate sample window.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleWindowError {
    /// The declared start lies after the declared end.
    Reversed,
}

/// Why a canonical evidence value is absent.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MissingEvidenceReason {
    /// No sample was produced for the relevant interval.
    NotSampled,
    /// The external/source system was unavailable.
    SourceUnavailable,
    /// A produced sample failed canonical validation and was rejected.
    RejectedInvalid,
    /// The active receptor/adapter does not support this evidence channel.
    Unsupported,
}

/// A value that is either present with exact sample provenance or explicitly absent.
///
/// This prevents missing/unknown evidence from silently becoming a numerical zero.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum EvidenceValue<T> {
    Present {
        value: T,
        provenance: SampleProvenance,
    },
    Missing {
        reason: MissingEvidenceReason,
    },
}

impl SampleProvenance {
    #[must_use]
    pub const fn instant(sampled_tick: Tick) -> Self {
        Self::Instant { sampled_tick }
    }

    pub const fn window(
        start_tick: Tick,
        end_tick: Tick,
        method_id: AggregationMethodId,
    ) -> Result<Self, SampleWindowError> {
        if start_tick.raw() <= end_tick.raw() {
            Ok(Self::Window {
                start_tick,
                end_tick,
                method_id,
            })
        } else {
            Err(SampleWindowError::Reversed)
        }
    }

    /// Most recent canonical tick contributing to this sample.
    #[must_use]
    pub const fn latest_tick(self) -> Tick {
        match self {
            Self::Instant { sampled_tick } => sampled_tick,
            Self::Window { end_tick, .. } => end_tick,
        }
    }

    /// Age of the most recent contributing observation at `decision_tick`.
    /// Future-dated evidence fails closed.
    pub fn age_at(self, decision_tick: Tick) -> Result<u64, TickArithmeticError> {
        decision_tick.elapsed_since(self.latest_tick())
    }
}

impl<T> EvidenceValue<T> {
    #[must_use]
    pub fn present(value: T, provenance: SampleProvenance) -> Self {
        Self::Present { value, provenance }
    }

    #[must_use]
    pub fn missing(reason: MissingEvidenceReason) -> Self {
        Self::Missing { reason }
    }

    #[must_use]
    pub const fn is_missing(&self) -> bool {
        matches!(self, Self::Missing { .. })
    }

    #[must_use]
    pub fn as_ref(&self) -> EvidenceValue<&T> {
        match self {
            Self::Present { value, provenance } => EvidenceValue::Present {
                value,
                provenance: *provenance,
            },
            Self::Missing { reason } => EvidenceValue::Missing { reason: *reason },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::UnitQ;

    #[test]
    fn aggregate_windows_fail_closed_when_reversed() {
        let method = AggregationMethodId::from_bytes([7; 32]);
        assert_eq!(
            SampleProvenance::window(Tick::new(8), Tick::new(7), method),
            Err(SampleWindowError::Reversed)
        );
    }

    #[test]
    fn age_is_relative_to_decision_tick() {
        let sample = SampleProvenance::instant(Tick::new(10));
        assert_eq!(sample.age_at(Tick::new(14)), Ok(4));
        assert_eq!(
            sample.age_at(Tick::new(9)),
            Err(TickArithmeticError::FutureReference)
        );
    }

    #[test]
    fn measured_zero_is_not_missing() {
        let measured_zero = EvidenceValue::present(
            UnitQ::ZERO,
            SampleProvenance::instant(Tick::new(3)),
        );
        let missing = EvidenceValue::<UnitQ>::missing(MissingEvidenceReason::NotSampled);

        assert!(!measured_zero.is_missing());
        assert!(missing.is_missing());
        assert_ne!(measured_zero, missing);
    }
}
