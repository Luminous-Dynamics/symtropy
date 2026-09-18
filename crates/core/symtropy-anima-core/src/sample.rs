//! Time-bounded evidence provenance and explicit missingness.
//!
//! Stored evidence records when it was sampled. "Current", "held", or "stale"
//! are decision-time classifications derived from provenance, the decision tick,
//! and a profile/policy; they are not persisted here as eternal truth.

use core::fmt;

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
///
/// Explicit discriminants are part of the stable semantic code surface. They are
/// not themselves a wire format; product adapters still own schema framing.
#[repr(u8)]
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub enum MissingEvidenceReason {
    /// No sample was produced for the relevant interval.
    NotSampled = 1,
    /// The external/source system was unavailable.
    SourceUnavailable = 2,
    /// A produced sample failed canonical validation and was rejected.
    RejectedInvalid = 3,
    /// The active receptor/adapter does not support this evidence channel.
    Unsupported = 4,
}

/// Unknown canonical missing-evidence discriminant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnknownMissingEvidenceReasonCode {
    raw: u8,
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

impl MissingEvidenceReason {
    #[must_use]
    pub const fn code(self) -> u8 {
        self as u8
    }

    /// Decodes a stable semantic missingness code and rejects unknown values.
    pub const fn from_code(code: u8) -> Result<Self, UnknownMissingEvidenceReasonCode> {
        match code {
            1 => Ok(Self::NotSampled),
            2 => Ok(Self::SourceUnavailable),
            3 => Ok(Self::RejectedInvalid),
            4 => Ok(Self::Unsupported),
            raw => Err(UnknownMissingEvidenceReasonCode { raw }),
        }
    }
}

impl TryFrom<u8> for MissingEvidenceReason {
    type Error = UnknownMissingEvidenceReasonCode;

    fn try_from(value: u8) -> Result<Self, Self::Error> {
        Self::from_code(value)
    }
}

impl UnknownMissingEvidenceReasonCode {
    #[must_use]
    pub const fn raw(self) -> u8 {
        self.raw
    }
}

impl fmt::Display for UnknownMissingEvidenceReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown ANIMA missing-evidence reason code {}", self.raw)
    }
}

impl std::error::Error for UnknownMissingEvidenceReasonCode {}

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

    const ALL_MISSING_REASONS: [MissingEvidenceReason; 4] = [
        MissingEvidenceReason::NotSampled,
        MissingEvidenceReason::SourceUnavailable,
        MissingEvidenceReason::RejectedInvalid,
        MissingEvidenceReason::Unsupported,
    ];

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
        let measured_zero =
            EvidenceValue::present(UnitQ::ZERO, SampleProvenance::instant(Tick::new(3)));
        let missing = EvidenceValue::<UnitQ>::missing(MissingEvidenceReason::NotSampled);

        assert!(!measured_zero.is_missing());
        assert!(missing.is_missing());
        assert_ne!(measured_zero, missing);
    }

    #[test]
    fn missing_reason_codes_round_trip_exactly() {
        for reason in ALL_MISSING_REASONS {
            assert_eq!(MissingEvidenceReason::from_code(reason.code()), Ok(reason));
            assert_eq!(MissingEvidenceReason::try_from(reason.code()), Ok(reason));
        }
    }

    #[test]
    fn unknown_missing_reason_codes_fail_closed() {
        assert_eq!(
            MissingEvidenceReason::from_code(0),
            Err(UnknownMissingEvidenceReasonCode { raw: 0 })
        );
        assert_eq!(
            MissingEvidenceReason::from_code(u8::MAX),
            Err(UnknownMissingEvidenceReasonCode { raw: u8::MAX })
        );
    }
}
