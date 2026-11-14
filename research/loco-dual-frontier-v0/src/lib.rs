//! Dual-frontier curriculum gating for adaptive locomotion research.
//!
//! The physical frontier and learning frontier are deliberately separate:
//! - physical assurance is a model-conditional, set-based result supplied by
//!   the independent viability/assurance stack;
//! - learning recoverability is an empirical finite-horizon curriculum signal.
//!
//! An empirical learning pass never creates a physical viability certificate.
//! Simulation-only exploration may proceed outside the current physical
//! frontier, but its label remains simulation-only and cannot be promoted to
//! hardware permission by this package.

/// Execution context for a proposed curriculum expansion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExecutionTarget {
    Simulation,
    Hardware,
}

/// Status from the independent set-valued physical viability analysis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PhysicalFrontierStatus {
    Certified,
    Unknown,
    OutOfEnvelope,
}

/// Status from the independent certificate lifecycle gate.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AssuranceEvidenceStatus {
    Valid,
    Unknown,
    Invalid,
}

/// HORIZON-style operational learning criteria, expressed as fixed-point
/// quantities. This is an empirical training signal, never a physical proof.
///
/// Ratios use parts per million (1_000_000 = 1.0). Tracking error uses the
/// caller's declared milli-unit; the unit must be fixed in the experiment
/// protocol before comparing policies or platforms.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearningThresholds {
    pub minimum_episode_fraction_ppm: u32,
    pub maximum_tracking_error_milli: u32,
    pub maximum_checkpoint_tracking_ratio_ppm: u32,
    pub minimum_checkpoint_reward_ratio_ppm: u32,
}

impl Default for LearningThresholds {
    fn default() -> Self {
        Self {
            minimum_episode_fraction_ppm: 850_000,
            maximum_tracking_error_milli: 500,
            maximum_checkpoint_tracking_ratio_ppm: 1_050_000,
            minimum_checkpoint_reward_ratio_ppm: 950_000,
        }
    }
}

/// Empirical values measured for one proposed frontier phase.
///
/// Missing measurements produce Unknown unless another measured value has
/// already failed a gate. This makes a known failure dominate missing data.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LearningMetrics {
    pub episode_fraction_ppm: Option<u32>,
    pub tracking_error_milli: Option<u32>,
    pub checkpoint_tracking_ratio_ppm: Option<u32>,
    pub checkpoint_reward_ratio_ppm: Option<u32>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum LearningFinding {
    EpisodeFractionMissing,
    EpisodeFractionBelowThreshold {
        observed_ppm: u32,
        required_ppm: u32,
    },
    TrackingErrorMissing,
    TrackingErrorAboveThreshold {
        observed_milli: u32,
        maximum_milli: u32,
    },
    CheckpointTrackingRatioMissing,
    CheckpointTrackingRatioAboveThreshold {
        observed_ppm: u32,
        maximum_ppm: u32,
    },
    CheckpointRewardRatioMissing,
    CheckpointRewardRatioBelowThreshold {
        observed_ppm: u32,
        minimum_ppm: u32,
    },
    InvalidThresholds,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LearningGateStatus {
    Pass,
    Fail,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LearningAssessment {
    pub status: LearningGateStatus,
    pub findings: Vec<LearningFinding>,
}

fn assess_learning(metrics: LearningMetrics, thresholds: LearningThresholds) -> LearningAssessment {
    let mut findings = Vec::new();

    if thresholds.minimum_episode_fraction_ppm > 1_000_000 {
        findings.push(LearningFinding::InvalidThresholds);
    }

    match metrics.episode_fraction_ppm {
        Some(value) if value < thresholds.minimum_episode_fraction_ppm => {
            findings.push(LearningFinding::EpisodeFractionBelowThreshold {
                observed_ppm: value,
                required_ppm: thresholds.minimum_episode_fraction_ppm,
            });
        }
        Some(_) => {}
        None => findings.push(LearningFinding::EpisodeFractionMissing),
    }

    match metrics.tracking_error_milli {
        Some(value) if value > thresholds.maximum_tracking_error_milli => {
            findings.push(LearningFinding::TrackingErrorAboveThreshold {
                observed_milli: value,
                maximum_milli: thresholds.maximum_tracking_error_milli,
            });
        }
        Some(_) => {}
        None => findings.push(LearningFinding::TrackingErrorMissing),
    }

    match metrics.checkpoint_tracking_ratio_ppm {
        Some(value) if value > thresholds.maximum_checkpoint_tracking_ratio_ppm => {
            findings.push(LearningFinding::CheckpointTrackingRatioAboveThreshold {
                observed_ppm: value,
                maximum_ppm: thresholds.maximum_checkpoint_tracking_ratio_ppm,
            });
        }
        Some(_) => {}
        None => findings.push(LearningFinding::CheckpointTrackingRatioMissing),
    }

    match metrics.checkpoint_reward_ratio_ppm {
        Some(value) if value < thresholds.minimum_checkpoint_reward_ratio_ppm => {
            findings.push(LearningFinding::CheckpointRewardRatioBelowThreshold {
                observed_ppm: value,
                minimum_ppm: thresholds.minimum_checkpoint_reward_ratio_ppm,
            });
        }
        Some(_) => {}
        None => findings.push(LearningFinding::CheckpointRewardRatioMissing),
    }

    findings.sort();
    findings.dedup();

    let status = if findings.iter().any(|finding| {
        !matches!(
            finding,
            LearningFinding::EpisodeFractionMissing
                | LearningFinding::TrackingErrorMissing
                | LearningFinding::CheckpointTrackingRatioMissing
                | LearningFinding::CheckpointRewardRatioMissing
        )
    }) {
        LearningGateStatus::Fail
    } else if findings.is_empty() {
        LearningGateStatus::Pass
    } else {
        LearningGateStatus::Unknown
    };

    LearningAssessment { status, findings }
}

/// Candidate physical-domain expansion and the evidence references attached
/// to it. The caller must obtain these statuses from separate tools; this
/// crate does not create physical or assurance certificates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DomainProposal {
    pub domain_id: String,
    pub policy_checkpoint_digest: String,
    pub proposal_digest: String,
    pub execution_target: ExecutionTarget,
    pub physical_frontier: PhysicalFrontierStatus,
    pub physical_certificate_ref: Option<String>,
    pub assurance: AssuranceEvidenceStatus,
    pub assurance_receipt_ref: Option<String>,
    pub learning_metrics: LearningMetrics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurriculumDecision {
    CommitHardwareFrontier,
    CommitSimulationOnly,
    RejectInvalidProposal,
    RejectLearningGate,
    RejectPhysicalOutOfEnvelope,
    RejectAssuranceInvalid,
    DeferLearningEvidence,
    DeferPhysicalUnknown,
    DeferAssuranceUnknown,
    DeferMissingPhysicalCertificate,
    DeferMissingAssuranceReceipt,
}

/// Full, auditable decision record. Physical and learning status remain
/// separately visible even when the final curriculum decision is a single
/// enum.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DualFrontierReport {
    pub domain_id: String,
    pub proposal_digest: String,
    pub execution_target: ExecutionTarget,
    pub physical_frontier: PhysicalFrontierStatus,
    pub assurance: AssuranceEvidenceStatus,
    pub learning: LearningAssessment,
    pub decision: CurriculumDecision,
}

fn nonblank(value: &str) -> bool {
    !value.trim().is_empty()
}

/// Evaluate one proposed domain expansion.
///
/// Hardware permission requires all three independent conditions:
/// 1. physical frontier is Certified and carries a non-empty certificate ref;
/// 2. assurance evidence is Valid and carries a non-empty receipt ref;
/// 3. empirical learning gate passes.
///
/// Simulation-only proposals may be committed when the learning gate passes,
/// but the report preserves whether their physical status was Certified,
/// Unknown or OutOfEnvelope. Such a simulation decision never authorizes
/// hardware execution.
pub fn evaluate_domain_proposal(
    proposal: &DomainProposal,
    thresholds: LearningThresholds,
) -> DualFrontierReport {
    let learning = assess_learning(proposal.learning_metrics, thresholds);

    let invalid_shape = !nonblank(&proposal.domain_id)
        || !nonblank(&proposal.policy_checkpoint_digest)
        || !nonblank(&proposal.proposal_digest);

    let decision = if invalid_shape {
        CurriculumDecision::RejectInvalidProposal
    } else if learning.status == LearningGateStatus::Fail {
        CurriculumDecision::RejectLearningGate
    } else if learning.status == LearningGateStatus::Unknown {
        CurriculumDecision::DeferLearningEvidence
    } else {
        match proposal.execution_target {
            ExecutionTarget::Simulation => CurriculumDecision::CommitSimulationOnly,
            ExecutionTarget::Hardware => match proposal.physical_frontier {
                PhysicalFrontierStatus::OutOfEnvelope => {
                    CurriculumDecision::RejectPhysicalOutOfEnvelope
                }
                PhysicalFrontierStatus::Unknown => CurriculumDecision::DeferPhysicalUnknown,
                PhysicalFrontierStatus::Certified => {
                    if !proposal
                        .physical_certificate_ref
                        .as_deref()
                        .is_some_and(nonblank)
                    {
                        CurriculumDecision::DeferMissingPhysicalCertificate
                    } else {
                        match proposal.assurance {
                            AssuranceEvidenceStatus::Invalid => {
                                CurriculumDecision::RejectAssuranceInvalid
                            }
                            AssuranceEvidenceStatus::Unknown => {
                                CurriculumDecision::DeferAssuranceUnknown
                            }
                            AssuranceEvidenceStatus::Valid => {
                                if proposal
                                    .assurance_receipt_ref
                                    .as_deref()
                                    .is_some_and(nonblank)
                                {
                                    CurriculumDecision::CommitHardwareFrontier
                                } else {
                                    CurriculumDecision::DeferMissingAssuranceReceipt
                                }
                            }
                        }
                    }
                }
            },
        }
    };

    DualFrontierReport {
        domain_id: proposal.domain_id.clone(),
        proposal_digest: proposal.proposal_digest.clone(),
        execution_target: proposal.execution_target,
        physical_frontier: proposal.physical_frontier,
        assurance: proposal.assurance,
        learning,
        decision,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_metrics() -> LearningMetrics {
        LearningMetrics {
            episode_fraction_ppm: Some(900_000),
            tracking_error_milli: Some(400),
            checkpoint_tracking_ratio_ppm: Some(1_020_000),
            checkpoint_reward_ratio_ppm: Some(980_000),
        }
    }

    fn proposal(target: ExecutionTarget) -> DomainProposal {
        DomainProposal {
            domain_id: "terrain-compliance-plus-payload".to_owned(),
            policy_checkpoint_digest: "policy-sha-001".to_owned(),
            proposal_digest: "proposal-sha-001".to_owned(),
            execution_target: target,
            physical_frontier: PhysicalFrontierStatus::Certified,
            physical_certificate_ref: Some("physical-cert-001".to_owned()),
            assurance: AssuranceEvidenceStatus::Valid,
            assurance_receipt_ref: Some("assurance-receipt-001".to_owned()),
            learning_metrics: passing_metrics(),
        }
    }

    fn thresholds() -> LearningThresholds {
        LearningThresholds::default()
    }

    #[test]
    fn all_three_hardware_gates_must_pass_to_commit() {
        let report = evaluate_domain_proposal(&proposal(ExecutionTarget::Hardware), thresholds());
        assert_eq!(report.learning.status, LearningGateStatus::Pass);
        assert_eq!(report.decision, CurriculumDecision::CommitHardwareFrontier);
    }

    #[test]
    fn empirical_learning_pass_cannot_override_unknown_physical_frontier() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.physical_frontier = PhysicalFrontierStatus::Unknown;
        candidate.physical_certificate_ref = None;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.learning.status, LearningGateStatus::Pass);
        assert_eq!(report.decision, CurriculumDecision::DeferPhysicalUnknown);
    }

    #[test]
    fn empirical_learning_pass_cannot_override_out_of_envelope_hardware() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.physical_frontier = PhysicalFrontierStatus::OutOfEnvelope;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(
            report.decision,
            CurriculumDecision::RejectPhysicalOutOfEnvelope
        );
    }

    #[test]
    fn invalid_assurance_blocks_hardware_despite_physical_and_learning_pass() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.assurance = AssuranceEvidenceStatus::Invalid;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.decision, CurriculumDecision::RejectAssuranceInvalid);
    }

    #[test]
    fn unknown_assurance_defers_hardware_despite_physical_and_learning_pass() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.assurance = AssuranceEvidenceStatus::Unknown;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.decision, CurriculumDecision::DeferAssuranceUnknown);
    }

    #[test]
    fn missing_physical_certificate_reference_cannot_be_inferred_from_status() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.physical_certificate_ref = None;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(
            report.decision,
            CurriculumDecision::DeferMissingPhysicalCertificate
        );
    }

    #[test]
    fn missing_assurance_receipt_cannot_be_inferred_from_valid_status() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.assurance_receipt_ref = None;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(
            report.decision,
            CurriculumDecision::DeferMissingAssuranceReceipt
        );
    }

    #[test]
    fn simulation_can_explore_outside_physical_frontier_but_remains_simulation_only() {
        let mut candidate = proposal(ExecutionTarget::Simulation);
        candidate.physical_frontier = PhysicalFrontierStatus::OutOfEnvelope;
        candidate.physical_certificate_ref = None;
        candidate.assurance = AssuranceEvidenceStatus::Invalid;
        candidate.assurance_receipt_ref = None;

        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.decision, CurriculumDecision::CommitSimulationOnly);
        assert_eq!(
            report.physical_frontier,
            PhysicalFrontierStatus::OutOfEnvelope
        );
        assert_eq!(report.execution_target, ExecutionTarget::Simulation);
    }

    #[test]
    fn simulation_unknown_physics_is_not_promoted_to_certified() {
        let mut candidate = proposal(ExecutionTarget::Simulation);
        candidate.physical_frontier = PhysicalFrontierStatus::Unknown;
        candidate.physical_certificate_ref = None;

        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.decision, CurriculumDecision::CommitSimulationOnly);
        assert_eq!(report.physical_frontier, PhysicalFrontierStatus::Unknown);
    }

    #[test]
    fn simulation_still_requires_learning_gate_to_pass() {
        let mut candidate = proposal(ExecutionTarget::Simulation);
        candidate.learning_metrics.episode_fraction_ppm = Some(500_000);
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.decision, CurriculumDecision::RejectLearningGate);
    }

    #[test]
    fn episode_length_alone_is_not_enough_when_checkpoint_preservation_fails() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.learning_metrics.checkpoint_reward_ratio_ppm = Some(700_000);
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.learning.status, LearningGateStatus::Fail);
        assert_eq!(report.decision, CurriculumDecision::RejectLearningGate);
        assert!(report.learning.findings.iter().any(|finding| {
            matches!(
                finding,
                LearningFinding::CheckpointRewardRatioBelowThreshold { .. }
            )
        }));
    }

    #[test]
    fn current_episode_fraction_and_tracking_can_pass_while_checkpoint_tracking_fails() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.learning_metrics.checkpoint_tracking_ratio_ppm = Some(1_200_000);
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.learning.status, LearningGateStatus::Fail);
        assert_eq!(report.decision, CurriculumDecision::RejectLearningGate);
    }

    #[test]
    fn missing_learning_measurements_produce_unknown_not_pass() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.learning_metrics.checkpoint_reward_ratio_ppm = None;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.learning.status, LearningGateStatus::Unknown);
        assert_eq!(report.decision, CurriculumDecision::DeferLearningEvidence);
    }

    #[test]
    fn known_learning_failure_dominates_other_missing_metrics() {
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.learning_metrics.episode_fraction_ppm = Some(100_000);
        candidate.learning_metrics.tracking_error_milli = None;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.learning.status, LearningGateStatus::Fail);
        assert_eq!(report.decision, CurriculumDecision::RejectLearningGate);
    }

    #[test]
    fn threshold_boundaries_are_inclusive() {
        let boundary = LearningMetrics {
            episode_fraction_ppm: Some(850_000),
            tracking_error_milli: Some(500),
            checkpoint_tracking_ratio_ppm: Some(1_050_000),
            checkpoint_reward_ratio_ppm: Some(950_000),
        };
        let mut candidate = proposal(ExecutionTarget::Hardware);
        candidate.learning_metrics = boundary;
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.learning.status, LearningGateStatus::Pass);
    }

    #[test]
    fn episode_fraction_threshold_above_one_is_invalid() {
        let invalid = LearningThresholds {
            minimum_episode_fraction_ppm: 1_000_001,
            ..thresholds()
        };
        let report = evaluate_domain_proposal(&proposal(ExecutionTarget::Simulation), invalid);
        assert_eq!(report.learning.status, LearningGateStatus::Fail);
        assert!(
            report
                .learning
                .findings
                .contains(&LearningFinding::InvalidThresholds)
        );
        assert_eq!(report.decision, CurriculumDecision::RejectLearningGate);
    }

    #[test]
    fn proposal_identity_is_required_for_any_commit() {
        let mut candidate = proposal(ExecutionTarget::Simulation);
        candidate.proposal_digest.clear();
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.decision, CurriculumDecision::RejectInvalidProposal);
    }

    #[test]
    fn learning_findings_are_deterministically_ordered() {
        let mut candidate = proposal(ExecutionTarget::Simulation);
        candidate.learning_metrics = LearningMetrics {
            episode_fraction_ppm: None,
            tracking_error_milli: Some(900),
            checkpoint_tracking_ratio_ppm: None,
            checkpoint_reward_ratio_ppm: Some(700_000),
        };
        let first = evaluate_domain_proposal(&candidate, thresholds());
        let second = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(first, second);
        let mut sorted = first.learning.findings.clone();
        sorted.sort();
        sorted.dedup();
        assert_eq!(first.learning.findings, sorted);
    }

    #[test]
    fn physical_status_and_learning_status_remain_separate_in_report() {
        let mut candidate = proposal(ExecutionTarget::Simulation);
        candidate.physical_frontier = PhysicalFrontierStatus::Unknown;
        candidate.learning_metrics.tracking_error_milli = Some(900);
        let report = evaluate_domain_proposal(&candidate, thresholds());
        assert_eq!(report.physical_frontier, PhysicalFrontierStatus::Unknown);
        assert_eq!(report.learning.status, LearningGateStatus::Fail);
        assert_eq!(report.decision, CurriculumDecision::RejectLearningGate);
    }
}
