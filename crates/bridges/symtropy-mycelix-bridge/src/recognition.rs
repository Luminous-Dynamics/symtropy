//! Recognition boundary for projections leaving the Symtropy simulation.
//!
//! A recognition record names a target representation without mutating the
//! source event or upgrading its epistemic scope.

use super::economic_events::{EconomicEventEnvelopeV1, EconomicEventValidationError};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RecognitionTarget {
    MycelixEconomicEvent,
    ValueflowsEconomicEvent,
    IntegralCosObservation,
    TendTransactionCandidate,
    AccountingProjection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecognitionRecord {
    pub source_event_id: String,
    pub source_origin: String,
    pub target: RecognitionTarget,
    pub target_id: String,
    pub source_replay_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RecognitionError {
    InvalidSource(EconomicEventValidationError),
    EmptyTargetId,
    OriginMutation,
    FingerprintMutation,
}

impl RecognitionRecord {
    pub fn recognize(
        event: &EconomicEventEnvelopeV1,
        target: RecognitionTarget,
        target_id: impl Into<String>,
    ) -> Result<Self, RecognitionError> {
        event.validate().map_err(RecognitionError::InvalidSource)?;
        let target_id = target_id.into();
        if target_id.is_empty() {
            return Err(RecognitionError::EmptyTargetId);
        }

        Ok(Self {
            source_event_id: event.source_event_id.clone(),
            source_origin: event.origin.clone(),
            target,
            target_id,
            source_replay_fingerprint: event.replay_fingerprint.digest_hex.clone(),
        })
    }

    pub fn preserves_origin(&self, event: &EconomicEventEnvelopeV1) -> bool {
        self.source_event_id == event.source_event_id
            && self.source_origin == event.origin
            && self.source_replay_fingerprint == event.replay_fingerprint.digest_hex
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economic_events::{EconomicEventKind, SimulationQuantity};

    fn event() -> EconomicEventEnvelopeV1 {
        let mut e = EconomicEventEnvelopeV1::new(
            "world-a", "episode-1", "build-7", "replay-7", 42,
            "evt-1", EconomicEventKind::OfferAccepted, "symtropy:world-a",
        );
        e.resource_refs.push("ore".into());
        e.quantities_and_units.push(SimulationQuantity { value: 5, unit: "kg".into() });
        e.refresh_fingerprint();
        e
    }

    #[test]
    fn recognition_preserves_source_identity_and_origin() {
        let e = event();
        let r = RecognitionRecord::recognize(
            &e,
            RecognitionTarget::TendTransactionCandidate,
            "tend-candidate-1",
        ).unwrap();
        assert!(r.preserves_origin(&e));
        assert_eq!(r.source_origin, "symtropy:world-a");
    }

    #[test]
    fn recognition_rejects_invalid_source() {
        let mut e = event();
        e.quantities_and_units[0].value = 6;
        assert!(matches!(
            RecognitionRecord::recognize(&e, RecognitionTarget::MycelixEconomicEvent, "m1"),
            Err(RecognitionError::InvalidSource(
                EconomicEventValidationError::FingerprintMismatch
            ))
        ));
    }

    #[test]
    fn recognition_does_not_create_authority() {
        let e = event();
        let r = RecognitionRecord::recognize(
            &e,
            RecognitionTarget::AccountingProjection,
            "journal-1",
        ).unwrap();
        // Target identity is deliberately separate from source identity.
        assert_ne!(r.target_id, r.source_event_id);
        assert!(r.preserves_origin(&e));
    }
}
