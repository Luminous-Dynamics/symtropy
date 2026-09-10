// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Conservative game-combat assessments from observer-scoped knowledge projections.
//!
//! This module is intentionally about *what an observer has evidence for*, not hidden
//! combat truth. It cannot inspect active-field charge, health conditions, capability
//! values, biometrics, or authoritative target state. A visible interception supports
//! only protection activity; observed movement supports only movement performance.

use crate::{FULL_CONFIDENCE, ObservableCue, ObservationBand, ProjectedKnowledge};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeSet, error::Error, fmt};
use symtropy_game_state::StableId;

/// A bounded, observer-specific assessment of one subject.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CombatAssessment {
    pub observer_id: StableId,
    pub subject_id: StableId,
    /// Confidence that the observer witnessed active protection behavior.
    /// This is not field charge, condition, coverage, or readiness.
    pub protection_activity_confidence: Option<u16>,
    /// Confidence that the observer witnessed impaired/unstable mobility.
    /// This is not a diagnosis or hidden health severity.
    pub mobility_limitation_confidence: Option<u16>,
    /// Confidence that the observer detected an ambiguous equipment emission.
    pub equipment_emission_confidence: Option<u16>,
    /// Claims actually used by this assessment after confidence/staleness filtering.
    pub evidence_claim_ids: BTreeSet<StableId>,
}

impl CombatAssessment {
    /// Small simulation-only decision surface proving that distinct evidence can lead
    /// to distinct behavior without omniscient state reads.
    pub fn posture(&self) -> GameCombatPosture {
        match (
            self.protection_activity_confidence.is_some(),
            self.mobility_limitation_confidence.is_some(),
        ) {
            (false, false) => GameCombatPosture::ObserveFurther,
            (true, false) => GameCombatPosture::MaintainSeparation,
            (false, true) => GameCombatPosture::ApproachCautiously,
            (true, true) => GameCombatPosture::RepositionForInformation,
        }
    }
}

/// Deliberately high-level game intent. No weapon selection, aiming, real-world
/// engagement doctrine, or hidden-state targeting is encoded here.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum GameCombatPosture {
    ObserveFurther,
    MaintainSeparation,
    ApproachCautiously,
    RepositionForInformation,
}

/// Build an assessment from already-projected observations for exactly one observer
/// and exactly one subject.
///
/// A mixed-observer or mixed-subject frame is rejected rather than silently merging
/// perspectives. Claims that are stale or below `minimum_confidence` are ignored.
pub fn assess_combat_observations(
    observer_id: &StableId,
    subject_id: &StableId,
    current_tick: u64,
    minimum_confidence: u16,
    observations: &[ProjectedKnowledge],
) -> Result<CombatAssessment, CombatAssessmentError> {
    if minimum_confidence > FULL_CONFIDENCE {
        return Err(CombatAssessmentError::MinimumConfidenceOutOfRange(
            minimum_confidence,
        ));
    }

    let mut assessment = CombatAssessment {
        observer_id: observer_id.clone(),
        subject_id: subject_id.clone(),
        protection_activity_confidence: None,
        mobility_limitation_confidence: None,
        equipment_emission_confidence: None,
        evidence_claim_ids: BTreeSet::new(),
    };

    for projected in observations {
        if &projected.observer_id != observer_id {
            return Err(CombatAssessmentError::ObserverMismatch {
                expected: observer_id.clone(),
                actual: projected.observer_id.clone(),
            });
        }
        if &projected.claim.subject_id != subject_id {
            return Err(CombatAssessmentError::SubjectMismatch {
                expected: subject_id.clone(),
                actual: projected.claim.subject_id.clone(),
            });
        }
        if projected.cue.subject_id() != &projected.claim.subject_id {
            return Err(CombatAssessmentError::ProjectionIntegrityMismatch {
                claim_id: projected.claim.id.clone(),
            });
        }
        if projected.claim.confidence > FULL_CONFIDENCE {
            return Err(CombatAssessmentError::ClaimConfidenceOutOfRange {
                claim_id: projected.claim.id.clone(),
                confidence: projected.claim.confidence,
            });
        }
        if projected.claim.is_stale(current_tick)
            || projected.claim.confidence < minimum_confidence
        {
            continue;
        }

        let used = match &projected.cue {
            ObservableCue::FieldInterception { .. } => {
                raise_confidence(
                    &mut assessment.protection_activity_confidence,
                    projected.claim.confidence,
                );
                true
            }
            ObservableCue::EquipmentEmission { .. } => {
                // Emission remains deliberately ambiguous. It is retained as evidence
                // but never promoted into protection state.
                raise_confidence(
                    &mut assessment.equipment_emission_confidence,
                    projected.claim.confidence,
                );
                true
            }
            ObservableCue::MobilityPerformance {
                observed_performance,
                observed_instability,
                ..
            } => {
                if mobility_limitation_observed(*observed_performance, *observed_instability) {
                    raise_confidence(
                        &mut assessment.mobility_limitation_confidence,
                        projected.claim.confidence,
                    );
                    true
                } else {
                    false
                }
            }
        };

        if used {
            assessment
                .evidence_claim_ids
                .insert(projected.claim.id.clone());
        }
    }

    Ok(assessment)
}

fn raise_confidence(slot: &mut Option<u16>, candidate: u16) {
    *slot = Some(slot.map_or(candidate, |current| current.max(candidate)));
}

fn mobility_limitation_observed(
    performance: ObservationBand,
    instability: ObservationBand,
) -> bool {
    performance <= ObservationBand::Low || instability >= ObservationBand::Medium
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CombatAssessmentError {
    MinimumConfidenceOutOfRange(u16),
    ObserverMismatch {
        expected: StableId,
        actual: StableId,
    },
    SubjectMismatch {
        expected: StableId,
        actual: StableId,
    },
    ProjectionIntegrityMismatch {
        claim_id: StableId,
    },
    ClaimConfidenceOutOfRange {
        claim_id: StableId,
        confidence: u16,
    },
}

impl fmt::Display for CombatAssessmentError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MinimumConfidenceOutOfRange(value) => write!(
                f,
                "minimum combat-observation confidence must be in 0..=10000, got {value}"
            ),
            Self::ObserverMismatch { expected, actual } => write!(
                f,
                "combat observation frame for {expected} contains evidence routed to {actual}"
            ),
            Self::SubjectMismatch { expected, actual } => write!(
                f,
                "combat observation frame for subject {expected} contains evidence about {actual}"
            ),
            Self::ProjectionIntegrityMismatch { claim_id } => write!(
                f,
                "projected cue and canonical claim disagree on subject for {claim_id}"
            ),
            Self::ClaimConfidenceOutOfRange {
                claim_id,
                confidence,
            } => write!(
                f,
                "claim {claim_id} has out-of-range confidence {confidence}"
            ),
        }
    }
}

impl Error for CombatAssessmentError {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ObservationEvidence, project_observable_cue,
    };
    use symtropy_protection_core::coverage::{ApproachSector, ProtectionRegion};
    use symtropy_residents::ClaimPrivacy;

    fn id(value: &str) -> StableId {
        StableId::parse(value).unwrap()
    }

    fn evidence(
        claim_id: &str,
        observer_id: &str,
        source_id: &str,
        tick: u64,
        stale_after: u64,
        confidence: u16,
    ) -> ObservationEvidence {
        ObservationEvidence {
            claim_id: id(claim_id),
            observer_id: id(observer_id),
            source_id: id(source_id),
            observed_tick: tick,
            stale_after_tick: Some(stale_after),
            confidence,
            privacy: ClaimPrivacy::Private,
        }
    }

    #[test]
    fn two_observers_can_form_different_legitimate_assessments_of_same_subject() {
        let target = id("frame:target");
        let observer_a = id("resident:observer-a");
        let observer_b = id("resident:observer-b");

        let field_flash = project_observable_cue(
            ObservableCue::FieldInterception {
                subject_id: target.clone(),
                region: ProtectionRegion::parse("body:chest").unwrap(),
                approach: ApproachSector::Front,
                visible_intensity: ObservationBand::High,
            },
            evidence(
                "claim:a-field-flash",
                "resident:observer-a",
                "sensor:a-vision",
                100,
                130,
                9_000,
            ),
        )
        .unwrap();

        let unstable_gait = project_observable_cue(
            ObservableCue::MobilityPerformance {
                subject_id: target.clone(),
                observed_performance: ObservationBand::Low,
                observed_instability: ObservationBand::High,
            },
            evidence(
                "claim:b-mobility",
                "resident:observer-b",
                "sensor:b-vision",
                104,
                140,
                8_500,
            ),
        )
        .unwrap();

        let a = assess_combat_observations(
            &observer_a,
            &target,
            110,
            5_000,
            std::slice::from_ref(&field_flash),
        )
        .unwrap();
        let b = assess_combat_observations(
            &observer_b,
            &target,
            110,
            5_000,
            std::slice::from_ref(&unstable_gait),
        )
        .unwrap();

        assert_eq!(a.protection_activity_confidence, Some(9_000));
        assert_eq!(a.mobility_limitation_confidence, None);
        assert_eq!(a.posture(), GameCombatPosture::MaintainSeparation);

        assert_eq!(b.protection_activity_confidence, None);
        assert_eq!(b.mobility_limitation_confidence, Some(8_500));
        assert_eq!(b.posture(), GameCombatPosture::ApproachCautiously);

        for projection in [&field_flash, &unstable_gait] {
            let proposition = projection.claim.proposition.as_str();
            assert!(!proposition.contains("charge"));
            assert!(!proposition.contains("injury"));
            assert!(!proposition.contains("severity"));
            assert!(!proposition.contains("diagnosis"));
        }
    }

    #[test]
    fn mixed_observer_frame_fails_closed() {
        let target = id("frame:target");
        let projection = project_observable_cue(
            ObservableCue::EquipmentEmission {
                subject_id: target.clone(),
                observed_signature: ObservationBand::Medium,
            },
            evidence(
                "claim:b-emission",
                "resident:observer-b",
                "sensor:b-em",
                100,
                120,
                8_000,
            ),
        )
        .unwrap();

        assert!(matches!(
            assess_combat_observations(
                &id("resident:observer-a"),
                &target,
                105,
                5_000,
                &[projection],
            ),
            Err(CombatAssessmentError::ObserverMismatch { .. })
        ));
    }

    #[test]
    fn stale_or_weak_evidence_does_not_drive_posture() {
        let target = id("frame:target");
        let observer = id("resident:observer");
        let stale = project_observable_cue(
            ObservableCue::FieldInterception {
                subject_id: target.clone(),
                region: ProtectionRegion::parse("body:chest").unwrap(),
                approach: ApproachSector::Front,
                visible_intensity: ObservationBand::High,
            },
            evidence(
                "claim:stale-field",
                "resident:observer",
                "sensor:vision",
                90,
                100,
                9_000,
            ),
        )
        .unwrap();
        let weak = project_observable_cue(
            ObservableCue::MobilityPerformance {
                subject_id: target.clone(),
                observed_performance: ObservationBand::Low,
                observed_instability: ObservationBand::High,
            },
            evidence(
                "claim:weak-mobility",
                "resident:observer",
                "sensor:vision",
                105,
                130,
                2_000,
            ),
        )
        .unwrap();

        let assessment = assess_combat_observations(
            &observer,
            &target,
            110,
            5_000,
            &[stale, weak],
        )
        .unwrap();
        assert_eq!(assessment.posture(), GameCombatPosture::ObserveFurther);
        assert!(assessment.evidence_claim_ids.is_empty());
    }

    #[test]
    fn equipment_emission_does_not_promote_to_protection_state() {
        let target = id("frame:target");
        let observer = id("resident:observer");
        let emission = project_observable_cue(
            ObservableCue::EquipmentEmission {
                subject_id: target.clone(),
                observed_signature: ObservationBand::High,
            },
            evidence(
                "claim:emission",
                "resident:observer",
                "sensor:em",
                100,
                130,
                9_500,
            ),
        )
        .unwrap();

        let assessment = assess_combat_observations(
            &observer,
            &target,
            105,
            5_000,
            &[emission],
        )
        .unwrap();
        assert_eq!(assessment.equipment_emission_confidence, Some(9_500));
        assert_eq!(assessment.protection_activity_confidence, None);
        assert_eq!(assessment.posture(), GameCombatPosture::ObserveFurther);
    }

    #[test]
    fn healthy_looking_mobility_is_not_misclassified_as_limitation() {
        let target = id("frame:target");
        let observer = id("resident:observer");
        let movement = project_observable_cue(
            ObservableCue::MobilityPerformance {
                subject_id: target.clone(),
                observed_performance: ObservationBand::High,
                observed_instability: ObservationBand::Trace,
            },
            evidence(
                "claim:movement-good",
                "resident:observer",
                "sensor:vision",
                100,
                130,
                9_000,
            ),
        )
        .unwrap();

        let assessment = assess_combat_observations(
            &observer,
            &target,
            105,
            5_000,
            &[movement],
        )
        .unwrap();
        assert_eq!(assessment.mobility_limitation_confidence, None);
        assert_eq!(assessment.posture(), GameCombatPosture::ObserveFurther);
    }

    #[test]
    fn deterministic_replay_produces_identical_assessment() {
        let target = id("frame:target");
        let observer = id("resident:observer");
        let projection = project_observable_cue(
            ObservableCue::FieldInterception {
                subject_id: target.clone(),
                region: ProtectionRegion::parse("frame:front-arc").unwrap(),
                approach: ApproachSector::Front,
                visible_intensity: ObservationBand::Medium,
            },
            evidence(
                "claim:deterministic-field",
                "resident:observer",
                "sensor:vision",
                100,
                130,
                7_500,
            ),
        )
        .unwrap();

        let a = assess_combat_observations(
            &observer,
            &target,
            105,
            5_000,
            std::slice::from_ref(&projection),
        );
        let b = assess_combat_observations(
            &observer,
            &target,
            105,
            5_000,
            &[projection],
        );
        assert_eq!(a, b);
    }
}
