//! Stable machine-readable causal reason codes.
//!
//! Human-readable Observatory/UI text should be rendered outside this crate.

use core::fmt;

/// Structured reason attached to a canonical rejection, constraint, or decision trace.
///
/// Numeric discriminants are explicit so adapters can version any future wire/evidence
/// mapping without depending on Rust's source-order representation.
#[repr(u16)]
#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub enum ReasonCode {
    NoCompatibleReceptor = 1,
    EvidenceBelowThreshold = 2,
    EvidenceStale = 3,
    EvidenceMissing = 4,
    LowSupportConfidence = 5,
    MissingCapability = 6,
    FatigueConstraint = 7,
    RequestRejectedUnsafe = 8,
    ControllerVeto = 9,
    InformationGainPreferred = 10,
}

/// Unknown canonical reason discriminant.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UnknownReasonCode {
    raw: u16,
}

impl ReasonCode {
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }

    /// Decodes an explicit reason code without accepting unknown values.
    pub const fn from_code(code: u16) -> Result<Self, UnknownReasonCode> {
        match code {
            1 => Ok(Self::NoCompatibleReceptor),
            2 => Ok(Self::EvidenceBelowThreshold),
            3 => Ok(Self::EvidenceStale),
            4 => Ok(Self::EvidenceMissing),
            5 => Ok(Self::LowSupportConfidence),
            6 => Ok(Self::MissingCapability),
            7 => Ok(Self::FatigueConstraint),
            8 => Ok(Self::RequestRejectedUnsafe),
            9 => Ok(Self::ControllerVeto),
            10 => Ok(Self::InformationGainPreferred),
            raw => Err(UnknownReasonCode { raw }),
        }
    }
}

impl TryFrom<u16> for ReasonCode {
    type Error = UnknownReasonCode;

    fn try_from(value: u16) -> Result<Self, Self::Error> {
        Self::from_code(value)
    }
}

impl UnknownReasonCode {
    #[must_use]
    pub const fn raw(self) -> u16 {
        self.raw
    }
}

impl fmt::Display for UnknownReasonCode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "unknown ANIMA reason code {}", self.raw)
    }
}

impl std::error::Error for UnknownReasonCode {}

#[cfg(test)]
mod tests {
    use super::*;

    const ALL_REASONS: [ReasonCode; 10] = [
        ReasonCode::NoCompatibleReceptor,
        ReasonCode::EvidenceBelowThreshold,
        ReasonCode::EvidenceStale,
        ReasonCode::EvidenceMissing,
        ReasonCode::LowSupportConfidence,
        ReasonCode::MissingCapability,
        ReasonCode::FatigueConstraint,
        ReasonCode::RequestRejectedUnsafe,
        ReasonCode::ControllerVeto,
        ReasonCode::InformationGainPreferred,
    ];

    #[test]
    fn reason_codes_have_explicit_stable_values() {
        assert_eq!(ReasonCode::NoCompatibleReceptor.code(), 1);
        assert_eq!(ReasonCode::ControllerVeto.code(), 9);
    }

    #[test]
    fn every_known_reason_round_trips_exactly() {
        for reason in ALL_REASONS {
            assert_eq!(ReasonCode::from_code(reason.code()), Ok(reason));
            assert_eq!(ReasonCode::try_from(reason.code()), Ok(reason));
        }
    }

    #[test]
    fn unknown_reason_codes_fail_closed() {
        assert_eq!(ReasonCode::from_code(0), Err(UnknownReasonCode { raw: 0 }));
        assert_eq!(
            ReasonCode::from_code(u16::MAX),
            Err(UnknownReasonCode { raw: u16::MAX })
        );
    }
}
