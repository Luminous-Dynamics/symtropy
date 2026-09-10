// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Bounded observable cues for non-omniscient Symtropy agents.
//!
//! This bridge intentionally accepts *observations*, not authoritative shield or
//! health state. Sensor/world adapters decide what an observer could actually see or
//! measure and provide one coarse cue. The bridge turns that cue into the existing
//! observer-scoped `KnowledgeClaim` representation without inventing hidden facts.

use serde::{Deserialize, Serialize};
use std::{error::Error, fmt};
use symtropy_game_state::StableId;
use symtropy_protection_core::coverage::{ApproachSector, ProtectionRegion};
use symtropy_residents::{ClaimPrivacy, KnowledgeClaim};

pub const FULL_CONFIDENCE: u16 = 10_000;

/// Deliberately coarse observable magnitude. Exact authoritative resource/health
/// values do not cross this boundary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ObservationBand {
    Trace,
    Low,
    Medium,
    High,
}

impl ObservationBand {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Trace => "trace",
            Self::Low => "low",
            Self::Medium => "medium",
            Self::High => "high",
        }
    }
}

/// A cue that can plausibly be exposed by a sensor, animation, visual/audio effect,
/// or other observer-facing world adapter.
///
/// There is intentionally no cue for exact field charge, exact field temperature,
/// private condition severity, diagnosis, player biometrics, or hidden capability.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum ObservableCue {
    /// The observer witnessed an active protection interception/flash.
    FieldInterception {
        subject_id: StableId,
        region: ProtectionRegion,
        approach: ApproachSector,
        visible_intensity: ObservationBand,
    },
    /// The observer measured an externally detectable equipment emission/signature.
    EquipmentEmission {
        subject_id: StableId,
        observed_signature: ObservationBand,
    },
    /// The observer saw/measured movement performance, not the underlying condition.
    MobilityPerformance {
        subject_id: StableId,
        observed_performance: ObservationBand,
        observed_instability: ObservationBand,
    },
}

impl ObservableCue {
    pub fn subject_id(&self) -> &StableId {
        match self {
            Self::FieldInterception { subject_id, .. }
            | Self::EquipmentEmission { subject_id, .. }
            | Self::MobilityPerformance { subject_id, .. } => subject_id,
        }
    }

    /// Deterministic concise proposition for the current resident knowledge store.
    /// The string is an explanation/storage compatibility surface, not world truth.
    pub fn proposition(&self) -> String {
        match self {
            Self::FieldInterception {
                region,
                approach,
                visible_intensity,
                ..
            } => format!(
                "observed field interception at {} from {:?}; visible-intensity={}",
                region.as_str(),
                approach,
                visible_intensity.label()
            ),
            Self::EquipmentEmission {
                observed_signature,
                ..
            } => format!(
                "observed equipment emission; signature-band={}",
                observed_signature.label()
            ),
            Self::MobilityPerformance {
                observed_performance,
                observed_instability,
                ..
            } => format!(
                "observed mobility performance={}; instability={}",
                observed_performance.label(),
                observed_instability.label()
            ),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ObservationEvidence {
    pub claim_id: StableId,
    /// Knowledge-base owner who receives the projected claim.
    pub observer_id: StableId,
    /// Sensor/person/device that produced the observation.
    pub source_id: StableId,
    pub observed_tick: u64,
    pub stale_after_tick: Option<u64>,
    /// Observation confidence in basis points, 0..=10_000.
    pub confidence: u16,
    pub privacy: ClaimPrivacy,
}

impl ObservationEvidence {
    pub fn validate(&self) -> Result<(), ObservationError> {
        if self.confidence > FULL_CONFIDENCE {
            return Err(ObservationError::ConfidenceOutOfRange(self.confidence));
        }
        if self
            .stale_after_tick
            .is_some_and(|expiry| expiry < self.observed_tick)
        {
            return Err(ObservationError::StaleBeforeObserved {
                observed_tick: self.observed_tick,
                stale_after_tick: self.stale_after_tick.unwrap_or_default(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProjectedKnowledge {
    /// Route this claim only into this observer's knowledge base.
    pub observer_id: StableId,
    pub claim: KnowledgeClaim,
}

pub fn project_observable_cue(
    cue: ObservableCue,
    evidence: ObservationEvidence,
) -> Result<ProjectedKnowledge, ObservationError> {
    evidence.validate()?;
    Ok(ProjectedKnowledge {
        observer_id: evidence.observer_id,
        claim: KnowledgeClaim {
            id: evidence.claim_id,
            subject_id: cue.subject_id().clone(),
            proposition: cue.proposition(),
            confidence: evidence.confidence,
            source_id: evidence.source_id,
            observed_tick: evidence.observed_tick,
            stale_after_tick: evidence.stale_after_tick,
            privacy: evidence.privacy,
        },
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationError {
    ConfidenceOutOfRange(u16),
    StaleBeforeObserved {
        observed_tick: u64,
        stale_after_tick: u64,
    },
}

impl fmt::Display for ObservationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ConfidenceOutOfRange(value) => {
                write!(f, "observation confidence must be in 0..=10000, got {value}")
            }
            Self::StaleBeforeObserved {
                observed_tick,
                stale_after_tick,
            } => write!(
                f,
                "observation cannot become stale at {stale_after_tick} before it is observed at {observed_tick}"
            ),
        }
    }
}

impl Error for ObservationError {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;
    use symtropy_residents::{DisclosureContext, KnowledgeBase};

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn evidence(claim_id: &str) -> ObservationEvidence {
        ObservationEvidence {
            claim_id: id(claim_id),
            observer_id: id("resident:observer"),
            source_id: id("sensor:observer-vision"),
            observed_tick: 100,
            stale_after_tick: Some(140),
            confidence: 7_500,
            privacy: ClaimPrivacy::Private,
        }
    }

    #[test]
    fn visible_field_cue_does_not_expose_hidden_charge_or_heat() {
        let projected = project_observable_cue(
            ObservableCue::FieldInterception {
                subject_id: id("resident:target"),
                region: ProtectionRegion::parse("body:chest").unwrap(),
                approach: ApproachSector::Front,
                visible_intensity: ObservationBand::High,
            },
            evidence("claim:field-flash"),
        )
        .unwrap();
        assert!(projected.claim.proposition.contains("field interception"));
        assert!(!projected.claim.proposition.contains("charge"));
        assert!(!projected.claim.proposition.contains("heat"));
    }

    #[test]
    fn mobility_cue_reports_performance_not_diagnosis_or_condition_severity() {
        let projected = project_observable_cue(
            ObservableCue::MobilityPerformance {
                subject_id: id("resident:target"),
                observed_performance: ObservationBand::Low,
                observed_instability: ObservationBand::High,
            },
            evidence("claim:movement"),
        )
        .unwrap();
        assert!(projected.claim.proposition.contains("mobility performance=low"));
        assert!(!projected.claim.proposition.contains("injury"));
        assert!(!projected.claim.proposition.contains("condition"));
        assert!(!projected.claim.proposition.contains("severity"));
    }

    #[test]
    fn claim_is_routed_to_explicit_observer_and_keeps_source_provenance() {
        let projected = project_observable_cue(
            ObservableCue::EquipmentEmission {
                subject_id: id("frame:target"),
                observed_signature: ObservationBand::Medium,
            },
            evidence("claim:signature"),
        )
        .unwrap();
        assert_eq!(projected.observer_id, id("resident:observer"));
        assert_eq!(projected.claim.source_id, id("sensor:observer-vision"));
        assert_eq!(projected.claim.confidence, 7_500);
        assert_eq!(projected.claim.observed_tick, 100);
        assert_eq!(projected.claim.stale_after_tick, Some(140));
    }

    #[test]
    fn invalid_confidence_fails_closed() {
        let mut bad = evidence("claim:bad");
        bad.confidence = 10_001;
        assert!(matches!(
            project_observable_cue(
                ObservableCue::EquipmentEmission {
                    subject_id: id("frame:target"),
                    observed_signature: ObservationBand::Low,
                },
                bad,
            ),
            Err(ObservationError::ConfidenceOutOfRange(10_001))
        ));
    }

    #[test]
    fn stale_deadline_cannot_precede_observation() {
        let mut bad = evidence("claim:stale");
        bad.stale_after_tick = Some(99);
        assert!(matches!(
            project_observable_cue(
                ObservableCue::EquipmentEmission {
                    subject_id: id("frame:target"),
                    observed_signature: ObservationBand::Low,
                },
                bad,
            ),
            Err(ObservationError::StaleBeforeObserved { .. })
        ));
    }

    #[test]
    fn bridge_does_not_bypass_existing_knowledge_privacy() {
        let projected = project_observable_cue(
            ObservableCue::MobilityPerformance {
                subject_id: id("resident:target"),
                observed_performance: ObservationBand::Medium,
                observed_instability: ObservationBand::Low,
            },
            evidence("claim:private-movement"),
        )
        .unwrap();
        let mut knowledge = KnowledgeBase::new(projected.observer_id.clone());
        knowledge.remember(projected.claim.clone());

        let other = DisclosureContext {
            requester_id: id("resident:other"),
            requester_household_id: None,
            consented_claim_ids: BTreeSet::new(),
            life_safety_emergency: false,
        };
        assert_eq!(knowledge.disclose(&other).count(), 0);

        let owner = DisclosureContext {
            requester_id: projected.claim.source_id.clone(),
            requester_household_id: None,
            consented_claim_ids: BTreeSet::new(),
            life_safety_emergency: false,
        };
        // Current resident privacy semantics authorize the claim source for Private.
        assert_eq!(knowledge.disclose(&owner).count(), 1);
    }

    #[test]
    fn identical_cue_and_evidence_project_identically() {
        let cue = ObservableCue::EquipmentEmission {
            subject_id: id("frame:target"),
            observed_signature: ObservationBand::High,
        };
        let evidence = evidence("claim:deterministic");
        assert_eq!(
            project_observable_cue(cue.clone(), evidence.clone()),
            project_observable_cue(cue, evidence)
        );
    }
}
