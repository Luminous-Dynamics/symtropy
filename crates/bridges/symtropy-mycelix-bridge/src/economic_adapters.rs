//! Executable cross-ontology adapter boundary for EconomicEventEnvelopeV1.
//!
//! The adapter never silently upgrades a simulation fact into a physical,
//! legal, financial, or settlement fact. Every projection carries an explicit
//! semantic-loss classification and remains reversible to the source event.

use super::economic_events::{EconomicEventEnvelopeV1, EconomicEventKind, SimulationValidity};
use super::recognition::{RecognitionRecord, RecognitionTarget};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SemanticDisposition {
    Preserved,
    Derived,
    BoundedLoss,
    Unmapped,
    Conflict,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ProjectedKind {
    MycelixEconomicEvent,
    ValueflowsEconomicEvent,
    ValueflowsIntent,
    ValueflowsCommitment,
    IntegralCosObservation,
    TendTransactionCandidate,
    AccountingProjection,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SemanticLossEntry {
    pub field: &'static str,
    pub disposition: SemanticDisposition,
    pub reason: &'static str,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EconomicProjection {
    pub source_event_id: String,
    pub source_origin: String,
    pub source_replay_fingerprint: String,
    pub target: RecognitionTarget,
    pub target_id: String,
    pub projected_kind: ProjectedKind,
    pub source_claim_ceiling: &'static str,
    pub semantic_loss: Vec<SemanticLossEntry>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum AdapterError {
    InvalidSource,
    UnsupportedEvent {
        target: RecognitionTarget,
        event: EconomicEventKind,
    },
    InvalidTargetId,
}

impl EconomicProjection {
    pub fn from_event(
        event: &EconomicEventEnvelopeV1,
        target: RecognitionTarget,
        target_id: impl Into<String>,
    ) -> Result<Self, AdapterError> {
        event.validate().map_err(|_| AdapterError::InvalidSource)?;
        let target_id = target_id.into();
        if target_id.is_empty() {
            return Err(AdapterError::InvalidTargetId);
        }

        let projected_kind = match target {
            RecognitionTarget::MycelixEconomicEvent => ProjectedKind::MycelixEconomicEvent,
            RecognitionTarget::ValueflowsEconomicEvent => match &event.source_event_type {
                EconomicEventKind::OfferCreated => ProjectedKind::ValueflowsIntent,
                EconomicEventKind::OfferAccepted => ProjectedKind::ValueflowsCommitment,
                _ => ProjectedKind::ValueflowsEconomicEvent,
            },
            RecognitionTarget::IntegralCosObservation => match &event.source_event_type {
                EconomicEventKind::ResourceProduced
                | EconomicEventKind::ResourceConsumed
                | EconomicEventKind::ServicePerformed
                | EconomicEventKind::AssetTransferred
                | EconomicEventKind::DeliveryCompleted
                | EconomicEventKind::RepairPerformed
                | EconomicEventKind::OutcomeObserved => ProjectedKind::IntegralCosObservation,
                _ => return Err(AdapterError::UnsupportedEvent {
                    target,
                    event: event.source_event_type.clone(),
                }),
            },
            RecognitionTarget::TendTransactionCandidate => ProjectedKind::TendTransactionCandidate,
            RecognitionTarget::AccountingProjection => ProjectedKind::AccountingProjection,
        };

        let mut semantic_loss = vec![
            SemanticLossEntry {
                field: "source_event_id/origin/replay_fingerprint",
                disposition: SemanticDisposition::Preserved,
                reason: "projection retains the exact simulation source identity",
            },
            SemanticLossEntry {
                field: "simulation_claim_ceiling",
                disposition: SemanticDisposition::Preserved,
                reason: "projection cannot widen the source claim ceiling",
            },
        ];

        match target {
            RecognitionTarget::ValueflowsEconomicEvent => {
                semantic_loss.push(SemanticLossEntry {
                    field: "world/build/replay context",
                    disposition: SemanticDisposition::Preserved,
                    reason: "retained as provenance metadata rather than Valueflows ontology semantics",
                });
            }
            RecognitionTarget::IntegralCosObservation => {
                semantic_loss.push(SemanticLossEntry {
                    field: "ITC entitlement",
                    disposition: SemanticDisposition::Unmapped,
                    reason: "a COS observation does not itself mint contribution credit",
                });
                semantic_loss.push(SemanticLossEntry {
                    field: "physical-world qualification",
                    disposition: SemanticDisposition::Unmapped,
                    reason: "simulation evidence remains simulation-scoped",
                });
            }
            RecognitionTarget::TendTransactionCandidate => {
                semantic_loss.push(SemanticLossEntry {
                    field: "settlement",
                    disposition: SemanticDisposition::Unmapped,
                    reason: "candidate recognition does not authorize or settle an instrument",
                });
            }
            RecognitionTarget::AccountingProjection => {
                semantic_loss.push(SemanticLossEntry {
                    field: "accounting meaning",
                    disposition: SemanticDisposition::Derived,
                    reason: "journal/accounting treatment requires a separate accounting policy",
                });
                semantic_loss.push(SemanticLossEntry {
                    field: "physical occurrence",
                    disposition: SemanticDisposition::BoundedLoss,
                    reason: "accounting projection is not physical-world evidence",
                });
            }
            RecognitionTarget::MycelixEconomicEvent => {}
        }

        if matches!(event.validity, SimulationValidity::Superseded | SimulationValidity::Conflicting | SimulationValidity::Indeterminate) {
            semantic_loss.push(SemanticLossEntry {
                field: "validity",
                disposition: SemanticDisposition::Conflict,
                reason: "non-current or unresolved simulation events must not be promoted by adapters",
            });
        }

        Ok(Self {
            source_event_id: event.source_event_id.clone(),
            source_origin: event.origin.clone(),
            source_replay_fingerprint: event.replay_fingerprint.digest_hex.clone(),
            target,
            target_id,
            projected_kind,
            source_claim_ceiling: event.simulation_claim_ceiling(),
            semantic_loss,
        })
    }

    pub fn recognition(&self) -> RecognitionRecord {
        RecognitionRecord {
            source_event_id: self.source_event_id.clone(),
            source_origin: self.source_origin.clone(),
            target: self.target,
            target_id: self.target_id.clone(),
            source_replay_fingerprint: self.source_replay_fingerprint.clone(),
        }
    }

    pub fn reversible_to(&self, event: &EconomicEventEnvelopeV1) -> bool {
        self.source_event_id == event.source_event_id
            && self.source_origin == event.origin
            && self.source_replay_fingerprint == event.replay_fingerprint.digest_hex
    }

    pub fn has_conflict(&self) -> bool {
        self.semantic_loss.iter().any(|e| e.disposition == SemanticDisposition::Conflict)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ConservationDimension {
    Identity,
    Origin,
    Replay,
    Quantity,
    Unit,
    Causality,
    Validity,
    CorrectionLineage,
    ClaimCeiling,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ConservationCheck {
    pub dimension: ConservationDimension,
    pub disposition: SemanticDisposition,
    pub reason: &'static str,
}

impl EconomicProjection {
    pub fn conservation_checks(&self, event: &EconomicEventEnvelopeV1) -> Vec<ConservationCheck> {
        let base = [
            (ConservationDimension::Identity, self.source_event_id == event.source_event_id),
            (ConservationDimension::Origin, self.source_origin == event.origin),
            (ConservationDimension::Replay, self.source_replay_fingerprint == event.replay_fingerprint.digest_hex),
            (ConservationDimension::ClaimCeiling, self.source_claim_ceiling == event.simulation_claim_ceiling()),
        ];
        base.into_iter().map(|(dimension, preserved)| ConservationCheck {
            dimension,
            disposition: if preserved { SemanticDisposition::Preserved } else { SemanticDisposition::Conflict },
            reason: if preserved { "source invariant is preserved" } else { "source invariant was mutated" },
        }).chain([
            ConservationCheck {
                dimension: ConservationDimension::Quantity,
                disposition: SemanticDisposition::Preserved,
                reason: "adapter does not rewrite source quantities; target conversion must be explicit",
            },
            ConservationCheck {
                dimension: ConservationDimension::Unit,
                disposition: SemanticDisposition::Preserved,
                reason: "adapter does not perform implicit unit conversion",
            },
            ConservationCheck {
                dimension: ConservationDimension::Causality,
                disposition: SemanticDisposition::Preserved,
                reason: "causal source event remains the provenance anchor",
            },
            ConservationCheck {
                dimension: ConservationDimension::Validity,
                disposition: if self.has_conflict() { SemanticDisposition::Conflict } else { SemanticDisposition::Preserved },
                reason: if self.has_conflict() { "source validity is unresolved" } else { "source validity is current" },
            },
            ConservationCheck {
                dimension: ConservationDimension::CorrectionLineage,
                disposition: SemanticDisposition::Preserved,
                reason: "correction lineage remains owned by the source event",
            },
        ]).collect()
    }

    pub fn conservation_holds(&self, event: &EconomicEventEnvelopeV1) -> bool {
        self.conservation_checks(event).iter().all(|check| {
            check.disposition == SemanticDisposition::Preserved
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::economic_events::{EconomicEventKind, SimulationQuantity};

    fn event(kind: EconomicEventKind) -> EconomicEventEnvelopeV1 {
        let mut e = EconomicEventEnvelopeV1::new(
            "world-a", "episode-1", "build-7", "replay-7", 42,
            "evt-1", kind, "symtropy:world-a",
        );
        e.resource_refs.push("ore".into());
        e.quantities_and_units.push(SimulationQuantity { value: 5, unit: "kg".into() });
        e.refresh_fingerprint();
        e
    }

    #[test]
    fn valueflows_offer_is_not_promoted_to_actual_event() {
        let p = EconomicProjection::from_event(
            &event(EconomicEventKind::OfferCreated),
            RecognitionTarget::ValueflowsEconomicEvent,
            "vf-intent-1",
        ).unwrap();
        assert_eq!(p.projected_kind, ProjectedKind::ValueflowsIntent);
        assert!(!p.has_conflict());
    }

    #[test]
    fn integral_projection_does_not_mint_itc() {
        let p = EconomicProjection::from_event(
            &event(EconomicEventKind::ResourceProduced),
            RecognitionTarget::IntegralCosObservation,
            "cos-observation-1",
        ).unwrap();
        assert!(p.semantic_loss.iter().any(|e|
            e.field == "ITC entitlement" && e.disposition == SemanticDisposition::Unmapped
        ));
    }

    #[test]
    fn accounting_is_derived_not_physical() {
        let p = EconomicProjection::from_event(
            &event(EconomicEventKind::ResourceProduced),
            RecognitionTarget::AccountingProjection,
            "journal-1",
        ).unwrap();
        assert!(p.semantic_loss.iter().any(|e|
            e.field == "accounting meaning" && e.disposition == SemanticDisposition::Derived
        ));
        assert!(p.semantic_loss.iter().any(|e|
            e.field == "physical occurrence" && e.disposition == SemanticDisposition::BoundedLoss
        ));
    }

    #[test]
    fn tend_projection_is_candidate_not_settlement() {
        let p = EconomicProjection::from_event(
            &event(EconomicEventKind::OfferAccepted),
            RecognitionTarget::TendTransactionCandidate,
            "tend-candidate-1",
        ).unwrap();
        assert!(p.semantic_loss.iter().any(|e|
            e.field == "settlement" && e.disposition == SemanticDisposition::Unmapped
        ));
    }

    #[test]
    fn every_projection_remains_reversible_to_source_identity() {
        let e = event(EconomicEventKind::ServicePerformed);
        for target in [
            RecognitionTarget::MycelixEconomicEvent,
            RecognitionTarget::ValueflowsEconomicEvent,
            RecognitionTarget::IntegralCosObservation,
            RecognitionTarget::TendTransactionCandidate,
            RecognitionTarget::AccountingProjection,
        ] {
            let p = EconomicProjection::from_event(&e, target, "target-1").unwrap();
            assert!(p.reversible_to(&e));
        }
    }

    #[test]
    fn unresolved_source_cannot_become_clean_projection() {
        let mut e = event(EconomicEventKind::ResourceProduced);
        e.validity = SimulationValidity::Indeterminate;
        e.refresh_fingerprint();
        let p = EconomicProjection::from_event(
            &e, RecognitionTarget::AccountingProjection, "journal-1"
        ).unwrap();
        assert!(p.has_conflict());
    }
}
