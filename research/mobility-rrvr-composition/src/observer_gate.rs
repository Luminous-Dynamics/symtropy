// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Claim-specific observer-quorum gate for RRVR cross-owner composition.
//!
//! The underlying restart/downstream/evidence composition remains unchanged.
//! This adapter adds two separate observer-assurance packets:
//! - a dependency/fault-model quorum for the exact old-attempt NoEffect claim;
//! - a distinct dependency/fault-model quorum for downstream graph quiescence.
//!
//! The output is a non-authorizing assessment. It does not authenticate
//! observer packets, consume permits, revalidate M1/M2, or call a native sink.

use super::{CompositionAssessment, CompositionDisposition};
use mobility_observer_assurance::{
    ObserverAssuranceDecision, ObserverAssurancePacket, assess_observer_assurance,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserverBoundHold {
    BaseCompositionNotReady,
    NoEffectObserverClaimMismatch,
    DownstreamObserverClaimMismatch,
    NoEffectObserverQuorumNotQualified,
    DownstreamObserverQuorumNotQualified,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ObserverBoundDisposition {
    PhysicalEffectAlreadyObservedNoRetry,
    PhysicalEffectObservedPersistenceUnresolvedNoRetry,
    ReadyForOwnerNativeRevalidation,
    Hold(ObserverBoundHold),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObserverBoundAssessment {
    /// The original composition result is preserved without reinterpretation.
    pub base: CompositionAssessment,
    /// Observer quorum for the exact old-attempt NoEffect claim.
    pub no_effect_observer: ObserverAssuranceDecision,
    /// Separate observer quorum for downstream graph quiescence.
    pub downstream_observer: ObserverAssuranceDecision,
    pub no_effect_observer_claim_matches: bool,
    pub downstream_observer_claim_matches: bool,
    pub disposition: ObserverBoundDisposition,
}

fn is_quorum_qualified(decision: &ObserverAssuranceDecision) -> bool {
    matches!(
        decision,
        ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel { .. }
    )
}

/// Apply two claim-specific observer-assurance checks to an already-computed
/// RRVR composition assessment.
///
/// This function never changes the base assessment. If an Applied physical
/// effect was already observed, it preserves the existing no-retry outcome;
/// it does not require a NoEffect quorum for an action that is not eligible
/// for retry. For a fresh-attempt candidate, both claim-specific observer
/// quorums must be qualified and exactly scope-bound before owner-native
/// revalidation may proceed.
pub fn assess_pre_native_with_observers(
    base: &CompositionAssessment,
    no_effect_observers: &ObserverAssurancePacket,
    downstream_observers: &ObserverAssurancePacket,
) -> ObserverBoundAssessment {
    let no_effect_observer = assess_observer_assurance(no_effect_observers);
    let downstream_observer = assess_observer_assurance(downstream_observers);

    let no_effect_observer_claim_matches = no_effect_observers.claim_id == base.no_effect_claim_id
        && no_effect_observers.claim_scope_commitment.0
            == base.no_effect_claim_scope_commitment.0;
    let downstream_observer_claim_matches = downstream_observers.claim_id
        == base.downstream_claim_id
        && downstream_observers.claim_scope_commitment.0
            == base.downstream_claim_scope_commitment.0;

    let disposition = match base.disposition {
        CompositionDisposition::PhysicalEffectAlreadyObservedNoRetry => {
            ObserverBoundDisposition::PhysicalEffectAlreadyObservedNoRetry
        }
        CompositionDisposition::PhysicalEffectObservedPersistenceUnresolvedNoRetry => {
            ObserverBoundDisposition::PhysicalEffectObservedPersistenceUnresolvedNoRetry
        }
        CompositionDisposition::Hold(_) => {
            ObserverBoundDisposition::Hold(ObserverBoundHold::BaseCompositionNotReady)
        }
        CompositionDisposition::ReadyForOwnerNativeRevalidation => {
            if !no_effect_observer_claim_matches {
                ObserverBoundDisposition::Hold(ObserverBoundHold::NoEffectObserverClaimMismatch)
            } else if !is_quorum_qualified(&no_effect_observer) {
                ObserverBoundDisposition::Hold(
                    ObserverBoundHold::NoEffectObserverQuorumNotQualified,
                )
            } else if !downstream_observer_claim_matches {
                ObserverBoundDisposition::Hold(ObserverBoundHold::DownstreamObserverClaimMismatch)
            } else if !is_quorum_qualified(&downstream_observer) {
                ObserverBoundDisposition::Hold(
                    ObserverBoundHold::DownstreamObserverQuorumNotQualified,
                )
            } else {
                ObserverBoundDisposition::ReadyForOwnerNativeRevalidation
            }
        }
    };

    ObserverBoundAssessment {
        base: base.clone(),
        no_effect_observer,
        downstream_observer,
        no_effect_observer_claim_matches,
        downstream_observer_claim_matches,
        disposition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        CompositionAssessment, CompositionDisposition, CompositionHold, EvidenceCommitment,
    };
    use mobility_downstream_fence::FenceDecision;
    use mobility_evidence_verdict::EvidenceVerdict;
    use mobility_observer_assurance::{
        Commitment, DependencyComponent, DependencyEdge, FaultScenario, ObserverChannel,
        ObserverSample,
    };
    use mobility_restart_reconciliation::RecoveryDecision;

    fn commitment(value: u8) -> Commitment {
        Commitment([value; 32])
    }

    fn evidence_commitment(value: u8) -> EvidenceCommitment {
        EvidenceCommitment([value; 32])
    }

    fn component(id: &str, domain: &str, kind: &str) -> DependencyComponent {
        DependencyComponent {
            component_id: id.to_owned(),
            failure_domain_id: domain.to_owned(),
            kind: kind.to_owned(),
        }
    }

    fn edge(from: &str, dependency: &str) -> DependencyEdge {
        DependencyEdge {
            component_id: from.to_owned(),
            depends_on_component_id: dependency.to_owned(),
        }
    }

    fn channel(id: &str, source: &str, scope: Commitment) -> ObserverChannel {
        ObserverChannel {
            channel_id: id.to_owned(),
            source_component_id: source.to_owned(),
            measurement_scope_commitment: scope,
            calibration_commitment: commitment(2),
            clock_profile_id: format!("clock-{id}"),
            maximum_time_uncertainty_ns: 20,
            maximum_sample_age_ns: 500,
        }
    }

    fn sample(id: &str, scope: Commitment) -> ObserverSample {
        ObserverSample {
            channel_id: id.to_owned(),
            measurement_scope_commitment: scope,
            calibration_commitment: commitment(2),
            clock_profile_id: format!("clock-{id}"),
            time_uncertainty_ns: 10,
            observed_at_ns: 900,
            evaluated_at_ns: 1_000,
            calibration_valid: true,
            placement_scope_matches: true,
            operating_envelope_matches: true,
            raw_evidence_commitment: commitment(3),
        }
    }

    fn observer_packet(claim_id: &str, scope_byte: u8) -> ObserverAssurancePacket {
        let scope = commitment(scope_byte);
        ObserverAssurancePacket {
            claim_id: claim_id.to_owned(),
            claim_scope_commitment: scope,
            dependency_profile_commitment: commitment(scope_byte.wrapping_add(1)),
            required_independent_observers: 2,
            components: vec![
                component("sensor-a", "sensor-domain-a", "encoder"),
                component("power-a", "power-domain-a", "power"),
                component("clock-a", "clock-domain-a", "clock"),
                component("sensor-b", "sensor-domain-b", "laser"),
                component("power-b", "power-domain-b", "power"),
                component("clock-b", "clock-domain-b", "clock"),
                component("sensor-c", "sensor-domain-c", "camera"),
                component("power-c", "power-domain-c", "power"),
                component("clock-c", "clock-domain-c", "clock"),
            ],
            dependency_edges: vec![
                edge("sensor-a", "power-a"),
                edge("sensor-a", "clock-a"),
                edge("sensor-b", "power-b"),
                edge("sensor-b", "clock-b"),
                edge("sensor-c", "power-c"),
                edge("sensor-c", "clock-c"),
            ],
            channels: vec![
                channel("observer-a", "sensor-a", scope),
                channel("observer-b", "sensor-b", scope),
                channel("observer-c", "sensor-c", scope),
            ],
            scenarios: vec![FaultScenario {
                scenario_id: "baseline".to_owned(),
                failed_component_ids: Vec::new(),
            }],
            samples: vec![
                sample("observer-a", scope),
                sample("observer-b", scope),
                sample("observer-c", scope),
            ],
        }
    }

    fn base_ready() -> CompositionAssessment {
        CompositionAssessment {
            recovery: RecoveryDecision::FreshAttemptCandidate,
            downstream: FenceDecision::AllDeclaredLayersHaveCurrentQuiescenceEvidence {
                qualified_layers: 5,
            },
            no_effect_evidence: EvidenceVerdict::Confirmed,
            no_effect_claim_id: "old-attempt-effect-1-attempt-1-no-effect".to_owned(),
            no_effect_claim_scope_commitment: evidence_commitment(55),
            downstream_evidence: EvidenceVerdict::Confirmed,
            downstream_claim_id: "downstream-graph-boot-new-epoch-9-quiescent".to_owned(),
            downstream_claim_scope_commitment: evidence_commitment(59),
            required_no_effect_claim_id_matches: true,
            no_effect_claim_scope_matches: true,
            required_downstream_claim_id_matches: true,
            downstream_claim_scope_matches: true,
            cross_owner_identity_matches: true,
            disposition: CompositionDisposition::ReadyForOwnerNativeRevalidation,
        }
    }

    fn no_effect_packet() -> ObserverAssurancePacket {
        observer_packet("old-attempt-effect-1-attempt-1-no-effect", 55)
    }

    fn downstream_packet() -> ObserverAssurancePacket {
        observer_packet("downstream-graph-boot-new-epoch-9-quiescent", 59)
    }

    #[test]
    fn both_claim_specific_observer_quorums_are_required_for_revalidation() {
        let base = base_ready();
        let result =
            assess_pre_native_with_observers(&base, &no_effect_packet(), &downstream_packet());
        assert!(matches!(
            result.no_effect_observer,
            ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel { .. }
        ));
        assert!(matches!(
            result.downstream_observer,
            ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel { .. }
        ));
        assert!(result.no_effect_observer_claim_matches);
        assert!(result.downstream_observer_claim_matches);
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::ReadyForOwnerNativeRevalidation
        );
    }

    #[test]
    fn shared_power_common_cause_blocks_only_the_no_effect_observer_claim() {
        let base = base_ready();
        let mut no_effect = no_effect_packet();
        no_effect
            .components
            .push(component("shared-power", "shared-power-domain", "power"));
        no_effect
            .dependency_edges
            .push(edge("sensor-a", "shared-power"));
        no_effect
            .dependency_edges
            .push(edge("sensor-b", "shared-power"));
        no_effect
            .dependency_edges
            .push(edge("sensor-c", "shared-power"));

        let result = assess_pre_native_with_observers(&base, &no_effect, &downstream_packet());
        assert!(matches!(
            result.no_effect_observer,
            ObserverAssuranceDecision::Hold { .. }
        ));
        assert!(matches!(
            result.downstream_observer,
            ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel { .. }
        ));
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::Hold(ObserverBoundHold::NoEffectObserverQuorumNotQualified)
        );
    }

    #[test]
    fn shared_clock_common_cause_blocks_only_downstream_graph_observer_claim() {
        let base = base_ready();
        let mut downstream = downstream_packet();
        downstream
            .components
            .push(component("shared-clock", "shared-clock-domain", "clock"));
        downstream
            .dependency_edges
            .push(edge("sensor-a", "shared-clock"));
        downstream
            .dependency_edges
            .push(edge("sensor-b", "shared-clock"));
        downstream
            .dependency_edges
            .push(edge("sensor-c", "shared-clock"));

        let result = assess_pre_native_with_observers(&base, &no_effect_packet(), &downstream);
        assert!(matches!(
            result.no_effect_observer,
            ObserverAssuranceDecision::QuorumQualifiedForDeclaredFaultModel { .. }
        ));
        assert!(matches!(
            result.downstream_observer,
            ObserverAssuranceDecision::Hold { .. }
        ));
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::Hold(ObserverBoundHold::DownstreamObserverQuorumNotQualified)
        );
    }

    #[test]
    fn observer_claim_identity_and_scope_must_match_independently() {
        let base = base_ready();
        let mut no_effect = no_effect_packet();
        no_effect.claim_scope_commitment = commitment(91);
        let result = assess_pre_native_with_observers(&base, &no_effect, &downstream_packet());
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::Hold(ObserverBoundHold::NoEffectObserverClaimMismatch)
        );

        let mut downstream = downstream_packet();
        downstream.claim_id = "wrong-downstream-claim".to_owned();
        let result = assess_pre_native_with_observers(&base, &no_effect_packet(), &downstream);
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::Hold(ObserverBoundHold::DownstreamObserverClaimMismatch)
        );
    }

    #[test]
    fn observed_applied_outcomes_remain_no_retry_even_if_observer_quorums_fail() {
        let mut base = base_ready();
        base.recovery = RecoveryDecision::EffectPredicateObserved;
        base.disposition = CompositionDisposition::PhysicalEffectAlreadyObservedNoRetry;
        let mut no_effect = no_effect_packet();
        no_effect.claim_id = "irrelevant-to-applied-outcome".to_owned();
        no_effect.required_independent_observers = 3;
        no_effect.scenarios[0].failed_component_ids = vec![
            "sensor-a".to_owned(),
            "sensor-b".to_owned(),
            "sensor-c".to_owned(),
        ];
        let mut downstream = downstream_packet();
        downstream.required_independent_observers = 3;
        downstream.scenarios[0].failed_component_ids = vec![
            "sensor-a".to_owned(),
            "sensor-b".to_owned(),
            "sensor-c".to_owned(),
        ];
        let result = assess_pre_native_with_observers(&base, &no_effect, &downstream);
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::PhysicalEffectAlreadyObservedNoRetry
        );
    }

    #[test]
    fn unresolved_persistence_with_observed_effect_remains_no_retry() {
        let mut base = base_ready();
        base.recovery = RecoveryDecision::EffectPredicateObservedPersistenceUnresolved;
        base.disposition =
            CompositionDisposition::PhysicalEffectObservedPersistenceUnresolvedNoRetry;
        let result =
            assess_pre_native_with_observers(&base, &no_effect_packet(), &downstream_packet());
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::PhysicalEffectObservedPersistenceUnresolvedNoRetry
        );
    }

    #[test]
    fn a_held_base_assessment_cannot_be_released_by_observer_quorums() {
        let mut base = base_ready();
        base.disposition = CompositionDisposition::Hold(CompositionHold::RecoveryAssessmentHeld);
        let result =
            assess_pre_native_with_observers(&base, &no_effect_packet(), &downstream_packet());
        assert_eq!(
            result.disposition,
            ObserverBoundDisposition::Hold(ObserverBoundHold::BaseCompositionNotReady)
        );
    }
}
