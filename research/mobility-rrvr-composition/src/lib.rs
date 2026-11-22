// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Non-authorizing composition of the RRVR reference contracts.
//!
//! This crate preserves three independent result algebras:
//! - restart/physical-effect recovery assessment;
//! - downstream command-graph quiescence evidence assessment;
//! - three-valued scoped evidence verdict.
//!
//! It joins the evidence to the same logical effect, old attempt, boot and
//! controller epochs and requires a Confirmed verdict for the exact
//! evaluator-declared NoEffect claim/scope before returning a
//! ReadyForOwnerNativeRevalidation assessment.
//!
//! It does not authenticate receipts, consume a recovery permit, validate
//! M1/M2 at the real callback, fence hardware, or call a native sink. Its
//! output is never a bearer capability or dispatch authorization.

use mobility_downstream_fence::{
    DownstreamFencePacket, FenceDecision, assess_downstream_quiescence,
};
use mobility_evidence_verdict::{
    ClaimProfile, Commitment as EvidenceCommitment, EvidenceFact, EvidenceVerdict,
    assess_evidence_claim,
};
use mobility_restart_reconciliation::{
    RecoveryDecision, RestartRecoveryEvidence, assess_restart_recovery,
};

pub const CONTRACT_VERSION: &str = "0.1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionHold {
    RecoveryAssessmentHeld,
    RecoveryPersistenceUnresolved,
    RecoveryNotAReattemptCandidate,
    CrossOwnerLogicalEffectMismatch,
    CrossOwnerAttemptMismatch,
    CrossOwnerBootEpochMismatch,
    CrossOwnerControllerEpochMismatch,
    DownstreamGraphNotQualified,
    RequiredClaimIdMismatch,
    ClaimScopeMismatch,
    EvidenceVerdictNotConfirmed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompositionDisposition {
    /// An independent Applied outcome has already been observed. This
    /// composition layer does not assess a second attempt.
    PhysicalEffectAlreadyObservedNoRetry,
    /// The physical effect is independently observed, but local persistence
    /// remains unresolved; preserve both facts and never assess a retry.
    PhysicalEffectObservedPersistenceUnresolvedNoRetry,
    /// All three owner contracts agree for this exact old attempt. The
    /// ordinary native authority/permit/queue boundary must still revalidate
    /// and consume all required owner state atomically.
    ReadyForOwnerNativeRevalidation,
    Hold(CompositionHold),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompositionAssessment {
    /// Preserved restart-contract result; never rewritten by the other planes.
    pub recovery: RecoveryDecision,
    /// Preserved graph-evidence result; never promoted to physical NoEffect.
    pub downstream: FenceDecision,
    /// Preserved scoped evidence verdict; never converted to action authority.
    pub evidence: EvidenceVerdict,
    /// Whether the evaluator profile names the required exact old-attempt claim.
    pub required_claim_id_matches: bool,
    /// Whether the supplied claim profile is bound to the expected scope digest.
    pub claim_scope_matches: bool,
    /// Cross-owner identity join across the restart and downstream contracts.
    pub cross_owner_identity_matches: bool,
    pub disposition: CompositionDisposition,
}

pub struct CompositionInput<'a> {
    pub recovery: &'a RestartRecoveryEvidence,
    pub downstream: &'a DownstreamFencePacket,
    pub claim_profile: &'a ClaimProfile,
    pub claim_facts: &'a [EvidenceFact],
    /// Exact evaluator claim ID for the old attempt's NoEffect predicate.
    pub required_claim_id: &'a str,
    /// Externally derived commitment over the exact effect, attempt, stack,
    /// transition, observer profile, operating scope and validity interval.
    /// This crate checks equality but does not invent or authenticate a digest.
    pub expected_scope_commitment: EvidenceCommitment,
}

fn cross_owner_identity_matches(
    recovery: &RestartRecoveryEvidence,
    downstream: &DownstreamFencePacket,
) -> bool {
    let transition = &downstream.transition;
    recovery.logical_effect_id == transition.logical_effect_id
        && recovery.prior_attempt_id == transition.old_attempt_id
        && recovery.prior_boot_epoch_id == transition.old_boot_epoch_id
        && recovery.current_boot_epoch_id == transition.new_boot_epoch_id
        && recovery.prior_controller_epoch == transition.old_controller_epoch
        && recovery.current_controller_epoch == transition.new_controller_epoch
}

/// Evaluate pre-native conditions without producing authority.
///
/// A ReadyForOwnerNativeRevalidation result means only that these three
/// supplied contracts agree on a specific old-attempt NoEffect recovery. The
/// actual M1/M2/native authority owner still has to revalidate the ticket,
/// authenticate and atomically consume the permit, and close the final
/// hardware-facing queue at the real application linearization point.
pub fn assess_pre_native(input: &CompositionInput<'_>) -> CompositionAssessment {
    let recovery = assess_restart_recovery(input.recovery);
    let downstream = assess_downstream_quiescence(input.downstream);
    let evidence = assess_evidence_claim(input.claim_profile, input.claim_facts);
    let required_claim_id_matches = input.claim_profile.claim_id == input.required_claim_id;
    let claim_scope_matches =
        input.claim_profile.scope_commitment == input.expected_scope_commitment;
    let cross_owner_identity_matches =
        cross_owner_identity_matches(input.recovery, input.downstream);

    let disposition = match recovery {
        RecoveryDecision::EffectPredicateObserved => {
            CompositionDisposition::PhysicalEffectAlreadyObservedNoRetry
        }
        RecoveryDecision::EffectPredicateObservedPersistenceUnresolved => {
            CompositionDisposition::PhysicalEffectObservedPersistenceUnresolvedNoRetry
        }
        RecoveryDecision::Hold(_) => {
            CompositionDisposition::Hold(CompositionHold::RecoveryAssessmentHeld)
        }
        RecoveryDecision::FreshAttemptCandidate => {
            if !cross_owner_identity_matches {
                CompositionDisposition::Hold(
                    match (
                        input.recovery.logical_effect_id
                            == input.downstream.transition.logical_effect_id,
                        input.recovery.prior_attempt_id
                            == input.downstream.transition.old_attempt_id,
                        input.recovery.prior_boot_epoch_id
                            == input.downstream.transition.old_boot_epoch_id
                            && input.recovery.current_boot_epoch_id
                                == input.downstream.transition.new_boot_epoch_id,
                    ) {
                        (false, _, _) => CompositionHold::CrossOwnerLogicalEffectMismatch,
                        (_, false, _) => CompositionHold::CrossOwnerAttemptMismatch,
                        (_, _, false) => CompositionHold::CrossOwnerBootEpochMismatch,
                        _ => CompositionHold::CrossOwnerControllerEpochMismatch,
                    },
                )
            } else if !matches!(
                downstream,
                FenceDecision::AllDeclaredLayersHaveCurrentQuiescenceEvidence { .. }
            ) {
                CompositionDisposition::Hold(CompositionHold::DownstreamGraphNotQualified)
            } else if !required_claim_id_matches {
                CompositionDisposition::Hold(CompositionHold::RequiredClaimIdMismatch)
            } else if !claim_scope_matches {
                CompositionDisposition::Hold(CompositionHold::ClaimScopeMismatch)
            } else if evidence != EvidenceVerdict::Confirmed {
                CompositionDisposition::Hold(CompositionHold::EvidenceVerdictNotConfirmed)
            } else {
                CompositionDisposition::ReadyForOwnerNativeRevalidation
            }
        }
    };

    CompositionAssessment {
        recovery,
        downstream,
        evidence,
        required_claim_id_matches,
        claim_scope_matches,
        cross_owner_identity_matches,
        disposition,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use mobility_downstream_fence::{
        CommandHoldingLayer, CommandPathEdge, Commitment as FenceCommitment,
        DownstreamFencePacket, LayerQuiescenceReceipt, QueueDisposition, QueuePathInventory,
        RestartTransition,
    };
    use mobility_evidence_verdict::{
        ClaimProfile, EvidenceRelation, EvidenceSourceKind,
    };
    use mobility_restart_reconciliation::{
        EffectEvidenceRef, EffectOutcome, FreshActionTicketRef, MissionResolution,
        MissionResolutionRef, OwnerVerdict, PersistenceVerdict, QueueFenceReceipt,
        RecoveryPermitRef, RestartRecoveryEvidence,
    };

    fn fence_commitment(value: u8) -> FenceCommitment {
        FenceCommitment([value; 32])
    }

    fn evidence_commitment(value: u8) -> EvidenceCommitment {
        EvidenceCommitment([value; 32])
    }

    fn layer(id: &str, path_index: u16, domain: &str) -> CommandHoldingLayer {
        CommandHoldingLayer {
            layer_id: id.to_owned(),
            path_index,
            kind: "test-driver-queue".to_owned(),
            configuration_commitment: fence_commitment(path_index as u8 + 1),
            sequence_generation_id: format!("generation-{id}"),
            owner_failure_domain_id: domain.to_owned(),
        }
    }

    fn edge(id: &str, from: &str, to: &str) -> CommandPathEdge {
        CommandPathEdge {
            edge_id: id.to_owned(),
            from_layer_id: from.to_owned(),
            to_layer_id: to.to_owned(),
            configuration_commitment: fence_commitment(10),
        }
    }

    fn layer_receipt(layer: &CommandHoldingLayer) -> LayerQuiescenceReceipt {
        LayerQuiescenceReceipt {
            receipt_id: format!("receipt-{}", layer.layer_id),
            evidence_id: format!("raw-{}", layer.layer_id),
            layer_id: layer.layer_id.clone(),
            layer_configuration_commitment: layer.configuration_commitment,
            stack_configuration_commitment: fence_commitment(9),
            inventory_commitment: fence_commitment(8),
            logical_effect_id: "effect-1".to_owned(),
            old_attempt_id: "attempt-1".to_owned(),
            old_boot_epoch_id: "boot-old".to_owned(),
            new_boot_epoch_id: "boot-new".to_owned(),
            old_controller_epoch: 8,
            new_controller_epoch: 9,
            sequence_generation_id: layer.sequence_generation_id.clone(),
            last_accepted_sequence: 42,
            last_retired_sequence: 42,
            outstanding_commands: 0,
            disposition: QueueDisposition::Quiescent,
            issuer_failure_domain_id: layer.owner_failure_domain_id.clone(),
            verifier_failure_domain_id: format!("verifier-{}", layer.layer_id),
            raw_evidence_commitment: fence_commitment(6),
        }
    }

    fn recovery_packet() -> RestartRecoveryEvidence {
        let binding = [7_u8; 32];
        RestartRecoveryEvidence {
            logical_effect_id: "effect-1".to_owned(),
            prior_attempt_id: "attempt-1".to_owned(),
            prior_boot_epoch_id: "boot-old".to_owned(),
            current_boot_epoch_id: "boot-new".to_owned(),
            prior_controller_epoch: 8,
            current_controller_epoch: 9,
            persistence: PersistenceVerdict::DefinitelyNotCommitted,
            queue_fence: Some(QueueFenceReceipt {
                receipt_id: "queue-fence-1".to_owned(),
                prior_boot_epoch_id: "boot-old".to_owned(),
                current_boot_epoch_id: "boot-new".to_owned(),
                from_controller_epoch: 8,
                to_controller_epoch: 9,
            }),
            effect_evidence: Some(EffectEvidenceRef {
                evidence_id: "effect-observation-1".to_owned(),
                observer_profile_id: "observer-profile-1".to_owned(),
                observer_failure_domain_id: "observer-domain".to_owned(),
                effect_failure_domain_id: "actuator-domain".to_owned(),
                logical_effect_id: "effect-1".to_owned(),
                attempt_id: "attempt-1".to_owned(),
                outcome: EffectOutcome::NoEffect,
            }),
            mission_resolution: Some(MissionResolutionRef {
                resolution_id: "m1-resolution-1".to_owned(),
                logical_effect_id: "effect-1".to_owned(),
                attempt_id: "attempt-1".to_owned(),
                resolution: MissionResolution::NoEffect,
            }),
            recovery_permit: Some(RecoveryPermitRef {
                permit_id: "permit-1".to_owned(),
                logical_effect_id: "effect-1".to_owned(),
                failed_attempt_id: "attempt-1".to_owned(),
                next_attempt_id: "attempt-2".to_owned(),
                authority_epoch: 8,
                plan_binding_commitment: binding,
            }),
            next_ticket: Some(FreshActionTicketRef {
                logical_effect_id: "effect-1".to_owned(),
                attempt_id: "attempt-2".to_owned(),
                boot_epoch_id: "boot-new".to_owned(),
                controller_epoch: 9,
                m1_authority: OwnerVerdict::Current,
                m2_applicability: OwnerVerdict::Current,
                ticket_binding_commitment: binding,
                live_binding_commitment: binding,
            }),
        }
    }

    fn downstream_packet() -> DownstreamFencePacket {
        let layers = vec![
            layer("controller", 0, "controller-domain"),
            layer("left-driver", 1, "left-domain"),
            layer("right-driver", 2, "right-domain"),
            layer("left-actuator", 3, "left-actuator-domain"),
            layer("right-actuator", 4, "right-actuator-domain"),
        ];
        let receipts = layers.iter().map(layer_receipt).collect();
        DownstreamFencePacket {
            inventory: QueuePathInventory {
                stack_id: "dual-arm-stack".to_owned(),
                profile_revision: "test-profile-v0.1".to_owned(),
                stack_configuration_commitment: fence_commitment(9),
                inventory_commitment: fence_commitment(8),
                entry_layer_id: "controller".to_owned(),
                terminal_layer_ids: vec![
                    "left-actuator".to_owned(),
                    "right-actuator".to_owned(),
                ],
                layers,
                edges: vec![
                    edge("controller-left", "controller", "left-driver"),
                    edge("controller-right", "controller", "right-driver"),
                    edge("left-actuator-link", "left-driver", "left-actuator"),
                    edge("right-actuator-link", "right-driver", "right-actuator"),
                ],
            },
            transition: RestartTransition {
                logical_effect_id: "effect-1".to_owned(),
                old_attempt_id: "attempt-1".to_owned(),
                old_boot_epoch_id: "boot-old".to_owned(),
                new_boot_epoch_id: "boot-new".to_owned(),
                old_controller_epoch: 8,
                new_controller_epoch: 9,
            },
            receipts,
        }
    }

    fn claim_profile() -> ClaimProfile {
        ClaimProfile {
            claim_id: "old-attempt-effect-1-attempt-1-no-effect".to_owned(),
            scope_commitment: evidence_commitment(55),
            subject_failure_domain_id: "robot-under-test".to_owned(),
            required_independent_domains: vec![
                "read-only-controller-observer".to_owned(),
                "independent-physical-observer".to_owned(),
            ],
        }
    }

    fn claim_fact(
        profile: &ClaimProfile,
        evidence_id: &str,
        domain: &str,
    ) -> EvidenceFact {
        EvidenceFact {
            claim_id: profile.claim_id.clone(),
            scope_commitment: profile.scope_commitment,
            evidence_id: evidence_id.to_owned(),
            source_id: format!("source-{evidence_id}"),
            failure_domain_id: domain.to_owned(),
            source_kind: EvidenceSourceKind::IndependentObservation,
            relation: EvidenceRelation::Supports,
            current: true,
            scope_matches: true,
            raw_evidence_commitment: evidence_commitment(56),
        }
    }

    fn facts(profile: &ClaimProfile) -> Vec<EvidenceFact> {
        vec![
            claim_fact(profile, "controller-observation", "read-only-controller-observer"),
            claim_fact(profile, "physical-observation", "independent-physical-observer"),
        ]
    }

    fn input<'a>(
        recovery: &'a RestartRecoveryEvidence,
        downstream: &'a DownstreamFencePacket,
        profile: &'a ClaimProfile,
        facts: &'a [EvidenceFact],
    ) -> CompositionInput<'a> {
        CompositionInput {
            recovery,
            downstream,
            claim_profile: profile,
            claim_facts: facts,
            required_claim_id: "old-attempt-effect-1-attempt-1-no-effect",
            expected_scope_commitment: evidence_commitment(55),
        }
    }

    #[test]
    fn all_three_owner_contracts_must_agree_before_native_owner_revalidation() {
        let recovery = recovery_packet();
        let downstream = downstream_packet();
        let profile = claim_profile();
        let facts = facts(&profile);
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));

        assert_eq!(result.recovery, RecoveryDecision::FreshAttemptCandidate);
        assert!(matches!(
            result.downstream,
            FenceDecision::AllDeclaredLayersHaveCurrentQuiescenceEvidence {
                qualified_layers: 5
            }
        ));
        assert_eq!(result.evidence, EvidenceVerdict::Confirmed);
        assert!(result.cross_owner_identity_matches);
        assert!(result.claim_scope_matches);
        assert_eq!(
            result.disposition,
            CompositionDisposition::ReadyForOwnerNativeRevalidation
        );
    }

    #[test]
    fn a_structurally_incomplete_downstream_graph_blocks_a_fresh_attempt() {
        let recovery = recovery_packet();
        let mut downstream = downstream_packet();
        downstream.receipts.pop();
        let profile = claim_profile();
        let facts = facts(&profile);
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));

        assert_eq!(result.recovery, RecoveryDecision::FreshAttemptCandidate);
        assert!(matches!(result.downstream, FenceDecision::Hold(_)));
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::DownstreamGraphNotQualified)
        );
    }

    #[test]
    fn cross_owner_logical_effect_or_attempt_substitution_blocks_composition() {
        let recovery = recovery_packet();
        let mut downstream = downstream_packet();
        downstream.transition.logical_effect_id = "other-effect".to_owned();
        let profile = claim_profile();
        let facts = facts(&profile);
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::CrossOwnerLogicalEffectMismatch)
        );

        let mut downstream = downstream_packet();
        downstream.transition.old_attempt_id = "other-attempt".to_owned();
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::CrossOwnerAttemptMismatch)
        );
    }

    #[test]
    fn cross_owner_boot_or_controller_epoch_mismatch_blocks_composition() {
        let recovery = recovery_packet();
        let mut downstream = downstream_packet();
        downstream.transition.new_boot_epoch_id = "stale-boot".to_owned();
        let profile = claim_profile();
        let facts = facts(&profile);
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::CrossOwnerBootEpochMismatch)
        );

        let mut downstream = downstream_packet();
        downstream.transition.new_controller_epoch = 10;
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::CrossOwnerControllerEpochMismatch)
        );
    }

    #[test]
    fn wrong_claim_id_or_scope_never_confirms_the_required_no_effect_claim() {
        let recovery = recovery_packet();
        let downstream = downstream_packet();
        let mut profile = claim_profile();
        let facts = facts(&profile);
        profile.claim_id = "some-other-claim".to_owned();
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::RequiredClaimIdMismatch)
        );

        let mut profile = claim_profile();
        let facts = facts(&profile);
        let mut composition_input = input(&recovery, &downstream, &profile, &facts);
        composition_input.expected_scope_commitment = evidence_commitment(99);
        let result = assess_pre_native(&composition_input);
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::ClaimScopeMismatch)
        );
    }

    #[test]
    fn refuted_or_insufficient_evidence_cannot_become_a_native_candidate() {
        let recovery = recovery_packet();
        let downstream = downstream_packet();
        let profile = claim_profile();

        let mut contradictory = facts(&profile);
        contradictory.push(EvidenceFact {
            claim_id: profile.claim_id.clone(),
            scope_commitment: profile.scope_commitment,
            evidence_id: "contradictory-observation".to_owned(),
            source_id: "source-contradictory-observation".to_owned(),
            failure_domain_id: "external-motion-observer".to_owned(),
            source_kind: EvidenceSourceKind::IndependentObservation,
            relation: EvidenceRelation::Contradicts,
            current: true,
            scope_matches: true,
            raw_evidence_commitment: evidence_commitment(57),
        });
        let result = assess_pre_native(&input(
            &recovery,
            &downstream,
            &profile,
            &contradictory,
        ));
        assert_eq!(result.evidence, EvidenceVerdict::Refuted);
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::EvidenceVerdictNotConfirmed)
        );

        let incomplete = vec![claim_fact(
            &profile,
            "controller-observation-only",
            "read-only-controller-observer",
        )];
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &incomplete));
        assert_eq!(result.evidence, EvidenceVerdict::InsufficientEvidence);
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::EvidenceVerdictNotConfirmed)
        );
    }

    #[test]
    fn subject_support_alone_does_not_satisfy_the_independent_claim() {
        let recovery = recovery_packet();
        let downstream = downstream_packet();
        let profile = claim_profile();
        let subject_only = vec![EvidenceFact {
            claim_id: profile.claim_id.clone(),
            scope_commitment: profile.scope_commitment,
            evidence_id: "subject-claim".to_owned(),
            source_id: "robot-self-report".to_owned(),
            failure_domain_id: "robot-under-test".to_owned(),
            source_kind: EvidenceSourceKind::SubjectAssertion,
            relation: EvidenceRelation::Supports,
            current: true,
            scope_matches: true,
            raw_evidence_commitment: evidence_commitment(58),
        }];
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &subject_only));
        assert_eq!(result.evidence, EvidenceVerdict::InsufficientEvidence);
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::EvidenceVerdictNotConfirmed)
        );
    }

    #[test]
    fn already_applied_physical_effect_is_never_promoted_to_recovery() {
        let mut recovery = recovery_packet();
        recovery.effect_evidence.as_mut().expect("effect evidence").outcome =
            mobility_restart_reconciliation::EffectOutcome::Applied;
        recovery.mission_resolution.as_mut().expect("M1 resolution").resolution =
            mobility_restart_reconciliation::MissionResolution::EffectPredicateSatisfied;
        recovery.recovery_permit = None;
        recovery.next_ticket = None;
        let downstream = downstream_packet();
        let profile = claim_profile();
        let facts = facts(&profile);
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.disposition,
            CompositionDisposition::PhysicalEffectAlreadyObservedNoRetry
        );
    }

    #[test]
    fn applied_effect_with_indeterminate_persistence_preserves_both_facts() {
        let mut recovery = recovery_packet();
        recovery.persistence =
            mobility_restart_reconciliation::PersistenceVerdict::IndeterminateCommit;
        recovery.effect_evidence.as_mut().expect("effect evidence").outcome =
            mobility_restart_reconciliation::EffectOutcome::Applied;
        recovery.mission_resolution.as_mut().expect("M1 resolution").resolution =
            mobility_restart_reconciliation::MissionResolution::EffectPredicateSatisfied;
        recovery.recovery_permit = None;
        recovery.next_ticket = None;
        let downstream = downstream_packet();
        let profile = claim_profile();
        let facts = facts(&profile);
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.recovery,
            RecoveryDecision::EffectPredicateObservedPersistenceUnresolved
        );
        assert_eq!(
            result.disposition,
            CompositionDisposition::PhysicalEffectObservedPersistenceUnresolvedNoRetry
        );
    }

    #[test]
    fn indeterminate_persistence_cannot_be_overridden_by_graph_and_verdict_passes() {
        let mut recovery = recovery_packet();
        recovery.persistence =
            mobility_restart_reconciliation::PersistenceVerdict::IndeterminateCommit;
        let downstream = downstream_packet();
        let profile = claim_profile();
        let facts = facts(&profile);
        let result = assess_pre_native(&input(&recovery, &downstream, &profile, &facts));
        assert_eq!(
            result.disposition,
            CompositionDisposition::Hold(CompositionHold::RecoveryAssessmentHeld)
        );
    }
}
