// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bounded cross-owner restart/reconciliation contract.
//!
//! This crate validates consistency between owner-supplied persistence,
//! effect-observation, mission-resolution, queue-fence and fresh-ticket claims.
//! It does not persist records, authenticate evidence, authorize a native sink,
//! or replace M1, M2, Mycelix or the controller queue.
//!
//! A FreshAttemptCandidate is an assessment result only. The regular native
//! application boundary must still revalidate the exact fresh ticket.

pub const CONTRACT_VERSION: &str = "0.1";

pub type Commitment = [u8; 32];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum PersistenceVerdict {
    Committed,
    DefinitelyNotCommitted,
    IndeterminateCommit,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EffectOutcome {
    Unknown,
    Applied,
    NoEffect,
    Partial,
    Disputed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MissionResolution {
    Unknown,
    EffectPredicateSatisfied,
    NoEffect,
    PartialEffect,
    Disputed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OwnerVerdict {
    Current,
    Stale,
    Rejected,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct QueueFenceReceipt {
    pub receipt_id: String,
    pub prior_boot_epoch_id: String,
    pub current_boot_epoch_id: String,
    pub from_controller_epoch: u64,
    pub to_controller_epoch: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct EffectEvidenceRef {
    pub evidence_id: String,
    pub observer_profile_id: String,
    pub observer_failure_domain_id: String,
    pub effect_failure_domain_id: String,
    pub logical_effect_id: String,
    pub attempt_id: String,
    pub outcome: EffectOutcome,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct MissionResolutionRef {
    pub resolution_id: String,
    pub logical_effect_id: String,
    pub attempt_id: String,
    pub resolution: MissionResolution,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RecoveryPermitRef {
    pub permit_id: String,
    pub logical_effect_id: String,
    pub failed_attempt_id: String,
    pub next_attempt_id: String,
    pub authority_epoch: u64,
    pub plan_binding_commitment: Commitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct FreshActionTicketRef {
    pub logical_effect_id: String,
    pub attempt_id: String,
    pub boot_epoch_id: String,
    pub controller_epoch: u64,
    pub m1_authority: OwnerVerdict,
    pub m2_applicability: OwnerVerdict,
    pub ticket_binding_commitment: Commitment,
    pub live_binding_commitment: Commitment,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RestartRecoveryEvidence {
    pub logical_effect_id: String,
    pub prior_attempt_id: String,
    pub prior_boot_epoch_id: String,
    pub current_boot_epoch_id: String,
    pub prior_controller_epoch: u64,
    pub current_controller_epoch: u64,
    pub persistence: PersistenceVerdict,
    pub queue_fence: Option<QueueFenceReceipt>,
    pub effect_evidence: Option<EffectEvidenceRef>,
    pub mission_resolution: Option<MissionResolutionRef>,
    pub recovery_permit: Option<RecoveryPermitRef>,
    pub next_ticket: Option<FreshActionTicketRef>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HoldReason {
    MissingIdentity,
    BootEpochUnchanged,
    ControllerEpochNotAdvanced,
    QueueFenceMissing,
    QueueFenceMismatch,
    IndeterminatePersistence,
    EffectEvidenceMissing,
    EffectOutcomeUnknown,
    EffectEvidenceIdentityMismatch,
    ObserverProfileMissing,
    ObserverFailureDomainNotIndependent,
    MissionResolutionMissing,
    MissionResolutionIdentityMismatch,
    MissionResolutionMismatch,
    PartialEffectQuarantine,
    ContradictoryEffectEvidence,
    UnexpectedRecoveryContext,
    RecoveryPermitMissing,
    RecoveryPermitMismatch,
    FreshTicketMissing,
    FreshTicketIdentityMismatch,
    FreshTicketBootMismatch,
    FreshTicketEpochMismatch,
    M1AuthorityNotCurrent,
    M2ApplicabilityNotCurrent,
    BindingCommitmentZero,
    BindingCommitmentMismatch,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RecoveryDecision {
    Hold(HoldReason),
    EffectPredicateObserved,
    FreshAttemptCandidate,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn nonzero(value: &Commitment) -> bool {
    value.iter().any(|byte| *byte != 0)
}

fn hold(reason: HoldReason) -> RecoveryDecision {
    RecoveryDecision::Hold(reason)
}

/// Assess a restart recovery packet without creating or authorizing an effect.
///
/// The local persistence verdict and physical-effect observation are separate
/// inputs. Committed and DefinitelyNotCommitted are deliberately not mapped to
/// Applied or NoEffect. IndeterminateCommit always keeps the gate closed.
pub fn assess_restart_recovery(evidence: &RestartRecoveryEvidence) -> RecoveryDecision {
    if !nonempty(&evidence.logical_effect_id)
        || !nonempty(&evidence.prior_attempt_id)
        || !nonempty(&evidence.prior_boot_epoch_id)
        || !nonempty(&evidence.current_boot_epoch_id)
    {
        return hold(HoldReason::MissingIdentity);
    }

    if evidence.prior_boot_epoch_id == evidence.current_boot_epoch_id {
        return hold(HoldReason::BootEpochUnchanged);
    }
    if evidence.current_controller_epoch <= evidence.prior_controller_epoch {
        return hold(HoldReason::ControllerEpochNotAdvanced);
    }

    let Some(queue_fence) = evidence.queue_fence.as_ref() else {
        return hold(HoldReason::QueueFenceMissing);
    };
    if !nonempty(&queue_fence.receipt_id)
        || queue_fence.prior_boot_epoch_id != evidence.prior_boot_epoch_id
        || queue_fence.current_boot_epoch_id != evidence.current_boot_epoch_id
        || queue_fence.from_controller_epoch != evidence.prior_controller_epoch
        || queue_fence.to_controller_epoch != evidence.current_controller_epoch
    {
        return hold(HoldReason::QueueFenceMismatch);
    }

    if evidence.persistence == PersistenceVerdict::IndeterminateCommit {
        return hold(HoldReason::IndeterminatePersistence);
    }

    let Some(effect) = evidence.effect_evidence.as_ref() else {
        return hold(HoldReason::EffectEvidenceMissing);
    };
    if effect.outcome == EffectOutcome::Unknown {
        return hold(HoldReason::EffectOutcomeUnknown);
    }
    if effect.logical_effect_id != evidence.logical_effect_id
        || effect.attempt_id != evidence.prior_attempt_id
    {
        return hold(HoldReason::EffectEvidenceIdentityMismatch);
    }
    if !nonempty(&effect.evidence_id) || !nonempty(&effect.observer_profile_id) {
        return hold(HoldReason::ObserverProfileMissing);
    }
    if !nonempty(&effect.observer_failure_domain_id)
        || !nonempty(&effect.effect_failure_domain_id)
        || effect.observer_failure_domain_id == effect.effect_failure_domain_id
    {
        return hold(HoldReason::ObserverFailureDomainNotIndependent);
    }

    let Some(mission) = evidence.mission_resolution.as_ref() else {
        return hold(HoldReason::MissionResolutionMissing);
    };
    if !nonempty(&mission.resolution_id)
        || mission.logical_effect_id != evidence.logical_effect_id
        || mission.attempt_id != evidence.prior_attempt_id
    {
        return hold(HoldReason::MissionResolutionIdentityMismatch);
    }

    let matching_resolution = match effect.outcome {
        EffectOutcome::Unknown => MissionResolution::Unknown,
        EffectOutcome::Applied => MissionResolution::EffectPredicateSatisfied,
        EffectOutcome::NoEffect => MissionResolution::NoEffect,
        EffectOutcome::Partial => MissionResolution::PartialEffect,
        EffectOutcome::Disputed => MissionResolution::Disputed,
    };
    if mission.resolution != matching_resolution {
        return hold(HoldReason::MissionResolutionMismatch);
    }

    match effect.outcome {
        EffectOutcome::Unknown => hold(HoldReason::EffectOutcomeUnknown),
        EffectOutcome::Disputed => hold(HoldReason::ContradictoryEffectEvidence),
        EffectOutcome::Partial => hold(HoldReason::PartialEffectQuarantine),
        EffectOutcome::Applied => {
            if evidence.recovery_permit.is_some() || evidence.next_ticket.is_some() {
                return hold(HoldReason::UnexpectedRecoveryContext);
            }
            RecoveryDecision::EffectPredicateObserved
        }
        EffectOutcome::NoEffect => assess_no_effect_recovery(evidence),
    }
}

fn assess_no_effect_recovery(evidence: &RestartRecoveryEvidence) -> RecoveryDecision {
    let Some(permit) = evidence.recovery_permit.as_ref() else {
        return hold(HoldReason::RecoveryPermitMissing);
    };
    let Some(ticket) = evidence.next_ticket.as_ref() else {
        return hold(HoldReason::FreshTicketMissing);
    };

    if !nonempty(&permit.permit_id)
        || permit.logical_effect_id != evidence.logical_effect_id
        || permit.failed_attempt_id != evidence.prior_attempt_id
        || permit.next_attempt_id != ticket.attempt_id
        || permit.authority_epoch != evidence.prior_controller_epoch
    {
        return hold(HoldReason::RecoveryPermitMismatch);
    }

    if !nonempty(&ticket.logical_effect_id)
        || !nonempty(&ticket.attempt_id)
        || ticket.logical_effect_id != evidence.logical_effect_id
        || ticket.attempt_id == evidence.prior_attempt_id
    {
        return hold(HoldReason::FreshTicketIdentityMismatch);
    }
    if ticket.boot_epoch_id != evidence.current_boot_epoch_id {
        return hold(HoldReason::FreshTicketBootMismatch);
    }
    if ticket.controller_epoch != evidence.current_controller_epoch {
        return hold(HoldReason::FreshTicketEpochMismatch);
    }
    if ticket.m1_authority != OwnerVerdict::Current {
        return hold(HoldReason::M1AuthorityNotCurrent);
    }
    if ticket.m2_applicability != OwnerVerdict::Current {
        return hold(HoldReason::M2ApplicabilityNotCurrent);
    }
    if !nonzero(&ticket.ticket_binding_commitment)
        || !nonzero(&ticket.live_binding_commitment)
        || !nonzero(&permit.plan_binding_commitment)
    {
        return hold(HoldReason::BindingCommitmentZero);
    }
    if ticket.ticket_binding_commitment != ticket.live_binding_commitment
        || permit.plan_binding_commitment != ticket.ticket_binding_commitment
    {
        return hold(HoldReason::BindingCommitmentMismatch);
    }

    RecoveryDecision::FreshAttemptCandidate
}

#[cfg(test)]
mod tests {
    use super::*;

    fn base_evidence() -> RestartRecoveryEvidence {
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
                evidence_id: "observer-evidence-1".to_owned(),
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

    fn set_effect(
        evidence: &mut RestartRecoveryEvidence,
        outcome: EffectOutcome,
        resolution: MissionResolution,
    ) {
        if let Some(effect) = evidence.effect_evidence.as_mut() {
            effect.outcome = outcome;
        }
        if let Some(mission) = evidence.mission_resolution.as_mut() {
            mission.resolution = resolution;
        }
    }

    fn assert_hold(evidence: &RestartRecoveryEvidence, expected: HoldReason) {
        assert_eq!(
            assess_restart_recovery(evidence),
            RecoveryDecision::Hold(expected)
        );
    }

    #[test]
    fn exact_no_effect_recovery_is_only_a_candidate() {
        assert_eq!(
            assess_restart_recovery(&base_evidence()),
            RecoveryDecision::FreshAttemptCandidate
        );
    }

    #[test]
    fn committed_local_record_does_not_prove_physical_effect() {
        let mut evidence = base_evidence();
        evidence.persistence = PersistenceVerdict::Committed;
        assert_eq!(
            assess_restart_recovery(&evidence),
            RecoveryDecision::FreshAttemptCandidate
        );
    }

    #[test]
    fn definitely_not_committed_does_not_prove_physical_no_effect() {
        for persistence in [
            PersistenceVerdict::Committed,
            PersistenceVerdict::DefinitelyNotCommitted,
        ] {
            let mut evidence = base_evidence();
            evidence.persistence = persistence;
            set_effect(
                &mut evidence,
                EffectOutcome::Unknown,
                MissionResolution::Unknown,
            );
            assert_hold(&evidence, HoldReason::EffectOutcomeUnknown);
        }
    }

    #[test]
    fn indeterminate_persistence_blocks_even_with_applied_observation() {
        let mut evidence = base_evidence();
        evidence.persistence = PersistenceVerdict::IndeterminateCommit;
        set_effect(
            &mut evidence,
            EffectOutcome::Applied,
            MissionResolution::EffectPredicateSatisfied,
        );
        evidence.recovery_permit = None;
        evidence.next_ticket = None;
        assert_hold(&evidence, HoldReason::IndeterminatePersistence);
    }

    #[test]
    fn applied_effect_is_observed_but_does_not_authorize_next_action() {
        let mut evidence = base_evidence();
        set_effect(
            &mut evidence,
            EffectOutcome::Applied,
            MissionResolution::EffectPredicateSatisfied,
        );
        evidence.recovery_permit = None;
        evidence.next_ticket = None;
        assert_eq!(
            assess_restart_recovery(&evidence),
            RecoveryDecision::EffectPredicateObserved
        );
    }

    #[test]
    fn applied_effect_cannot_smuggle_a_new_ticket_through_reconciliation() {
        let mut evidence = base_evidence();
        set_effect(
            &mut evidence,
            EffectOutcome::Applied,
            MissionResolution::EffectPredicateSatisfied,
        );
        evidence.recovery_permit = None;
        assert_hold(&evidence, HoldReason::UnexpectedRecoveryContext);
    }

    #[test]
    fn no_effect_requires_one_shot_recovery_permit() {
        let mut evidence = base_evidence();
        evidence.recovery_permit = None;
        assert_hold(&evidence, HoldReason::RecoveryPermitMissing);
    }

    #[test]
    fn no_effect_requires_fresh_ticket() {
        let mut evidence = base_evidence();
        evidence.next_ticket = None;
        assert_hold(&evidence, HoldReason::FreshTicketMissing);
    }

    #[test]
    fn partial_effect_remains_quarantined_even_with_permit_and_ticket() {
        let mut evidence = base_evidence();
        set_effect(
            &mut evidence,
            EffectOutcome::Partial,
            MissionResolution::PartialEffect,
        );
        assert_hold(&evidence, HoldReason::PartialEffectQuarantine);
    }

    #[test]
    fn disputed_effect_is_an_integrity_hold() {
        let mut evidence = base_evidence();
        set_effect(
            &mut evidence,
            EffectOutcome::Disputed,
            MissionResolution::Disputed,
        );
        assert_hold(&evidence, HoldReason::ContradictoryEffectEvidence);
    }

    #[test]
    fn fresh_ticket_must_use_a_new_attempt_id() {
        let mut evidence = base_evidence();
        let ticket = evidence.next_ticket.as_mut().expect("fixture ticket");
        ticket.attempt_id = "attempt-1".to_owned();
        assert_hold(&evidence, HoldReason::RecoveryPermitMismatch);
    }

    #[test]
    fn fresh_ticket_must_keep_logical_effect_identity() {
        let mut evidence = base_evidence();
        evidence
            .next_ticket
            .as_mut()
            .expect("fixture ticket")
            .logical_effect_id = "effect-substituted".to_owned();
        assert_hold(&evidence, HoldReason::FreshTicketIdentityMismatch);
    }

    #[test]
    fn fresh_ticket_must_bind_current_boot_and_controller_epoch() {
        let mut evidence = base_evidence();
        evidence
            .next_ticket
            .as_mut()
            .expect("fixture ticket")
            .boot_epoch_id = "boot-old".to_owned();
        assert_hold(&evidence, HoldReason::FreshTicketBootMismatch);

        let mut evidence = base_evidence();
        evidence
            .next_ticket
            .as_mut()
            .expect("fixture ticket")
            .controller_epoch = 8;
        assert_hold(&evidence, HoldReason::FreshTicketEpochMismatch);
    }

    #[test]
    fn m1_and_m2_must_both_report_current() {
        let mut evidence = base_evidence();
        evidence
            .next_ticket
            .as_mut()
            .expect("fixture ticket")
            .m1_authority = OwnerVerdict::Unknown;
        assert_hold(&evidence, HoldReason::M1AuthorityNotCurrent);

        let mut evidence = base_evidence();
        evidence
            .next_ticket
            .as_mut()
            .expect("fixture ticket")
            .m2_applicability = OwnerVerdict::Rejected;
        assert_hold(&evidence, HoldReason::M2ApplicabilityNotCurrent);
    }

    #[test]
    fn ticket_and_live_binding_must_match_and_be_nonzero() {
        let mut evidence = base_evidence();
        evidence
            .next_ticket
            .as_mut()
            .expect("fixture ticket")
            .live_binding_commitment = [8_u8; 32];
        assert_hold(&evidence, HoldReason::BindingCommitmentMismatch);

        let mut evidence = base_evidence();
        evidence
            .next_ticket
            .as_mut()
            .expect("fixture ticket")
            .ticket_binding_commitment = [0_u8; 32];
        evidence
            .recovery_permit
            .as_mut()
            .expect("fixture permit")
            .plan_binding_commitment = [0_u8; 32];
        assert_hold(&evidence, HoldReason::BindingCommitmentZero);
    }

    #[test]
    fn queue_fence_must_bind_both_boots_and_epochs() {
        let mut evidence = base_evidence();
        evidence.queue_fence.as_mut().expect("fixture fence").to_controller_epoch = 10;
        assert_hold(&evidence, HoldReason::QueueFenceMismatch);

        let mut evidence = base_evidence();
        evidence.queue_fence = None;
        assert_hold(&evidence, HoldReason::QueueFenceMissing);
    }

    #[test]
    fn old_boot_or_non_advancing_epoch_fails_closed() {
        let mut evidence = base_evidence();
        evidence.current_boot_epoch_id = "boot-old".to_owned();
        assert_hold(&evidence, HoldReason::BootEpochUnchanged);

        let mut evidence = base_evidence();
        evidence.current_controller_epoch = 8;
        assert_hold(&evidence, HoldReason::ControllerEpochNotAdvanced);
    }

    #[test]
    fn evidence_and_mission_resolution_must_name_exact_attempt() {
        let mut evidence = base_evidence();
        evidence
            .effect_evidence
            .as_mut()
            .expect("fixture observation")
            .attempt_id = "attempt-foreign".to_owned();
        assert_hold(&evidence, HoldReason::EffectEvidenceIdentityMismatch);

        let mut evidence = base_evidence();
        evidence
            .mission_resolution
            .as_mut()
            .expect("fixture resolution")
            .attempt_id = "attempt-foreign".to_owned();
        assert_hold(&evidence, HoldReason::MissionResolutionIdentityMismatch);
    }

    #[test]
    fn observer_must_have_a_distinct_declared_failure_domain() {
        let mut evidence = base_evidence();
        let effect = evidence
            .effect_evidence
            .as_mut()
            .expect("fixture observation");
        effect.observer_failure_domain_id = effect.effect_failure_domain_id.clone();
        assert_hold(
            &evidence,
            HoldReason::ObserverFailureDomainNotIndependent,
        );
    }

    #[test]
    fn mismatched_mission_resolution_never_clears_the_hold() {
        let mut evidence = base_evidence();
        if let Some(mission) = evidence.mission_resolution.as_mut() {
            mission.resolution = MissionResolution::EffectPredicateSatisfied;
        }
        assert_hold(&evidence, HoldReason::MissionResolutionMismatch);
    }

    #[test]
    fn exhaustive_decision_matrix_never_promotes_unknown_partial_or_disputed() {
        let persistence_values = [
            PersistenceVerdict::Committed,
            PersistenceVerdict::DefinitelyNotCommitted,
            PersistenceVerdict::IndeterminateCommit,
        ];
        let effect_values = [
            EffectOutcome::Unknown,
            EffectOutcome::Applied,
            EffectOutcome::NoEffect,
            EffectOutcome::Partial,
            EffectOutcome::Disputed,
        ];
        let mission_values = [
            MissionResolution::Unknown,
            MissionResolution::EffectPredicateSatisfied,
            MissionResolution::NoEffect,
            MissionResolution::PartialEffect,
            MissionResolution::Disputed,
        ];
        let owner_values = [
            OwnerVerdict::Current,
            OwnerVerdict::Stale,
            OwnerVerdict::Rejected,
            OwnerVerdict::Unknown,
        ];

        for persistence in persistence_values {
            for effect in effect_values {
                for mission in mission_values {
                    for m1 in owner_values {
                        for m2 in owner_values {
                            let mut evidence = base_evidence();
                            evidence.persistence = persistence;
                            set_effect(&mut evidence, effect, mission);
                            if let Some(ticket) = evidence.next_ticket.as_mut() {
                                ticket.m1_authority = m1;
                                ticket.m2_applicability = m2;
                            }
                            if effect != EffectOutcome::NoEffect {
                                evidence.recovery_permit = None;
                                evidence.next_ticket = None;
                            }

                            let decision = assess_restart_recovery(&evidence);
                            if persistence == PersistenceVerdict::IndeterminateCommit
                                || matches!(
                                    effect,
                                    EffectOutcome::Unknown
                                        | EffectOutcome::Partial
                                        | EffectOutcome::Disputed
                                )
                            {
                                assert_ne!(
                                    decision,
                                    RecoveryDecision::FreshAttemptCandidate
                                );
                            }
                            if decision == RecoveryDecision::FreshAttemptCandidate {
                                assert_eq!(effect, EffectOutcome::NoEffect);
                                assert_eq!(mission, MissionResolution::NoEffect);
                                assert_eq!(m1, OwnerVerdict::Current);
                                assert_eq!(m2, OwnerVerdict::Current);
                                assert_ne!(
                                    persistence,
                                    PersistenceVerdict::IndeterminateCommit
                                );
                            }
                        }
                    }
                }
            }
        }
    }
}
