//! Stable machine-readable causal reason codes.
//!
//! Human-readable Observatory/UI text should be rendered outside this crate.

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

impl ReasonCode {
    #[must_use]
    pub const fn code(self) -> u16 {
        self as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reason_codes_have_explicit_stable_values() {
        assert_eq!(ReasonCode::NoCompatibleReceptor.code(), 1);
        assert_eq!(ReasonCode::ControllerVeto.code(), 9);
    }
}
