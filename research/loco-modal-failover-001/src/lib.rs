//! Recoverability-gated multimodal locomotion mode/fallback composition.
//!
//! This reference consumes upstream channel-health interpretations and
//! independent robust mode-transition receipts. It does not implement an OOD
//! detector, sensor fusion algorithm, controller, or viability solver.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelHealth {
    Valid,
    Degraded,
    Ood,
    Unavailable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthEvidence {
    pub evidence_id: String,
    pub channel_id: String,
    pub calibration_digest: String,
    pub clock_mapping_digest: String,
    pub common_cause_group: String,
    pub health: ChannelHealth,
    pub acquired_at_ns: u64,
    pub arrived_at_ns: u64,
    pub valid_until_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HealthState {
    pub channel_id: String,
    pub health: ChannelHealth,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossibleHealthSignature {
    pub signature_id: String,
    pub health_evidence_digest: String,
    pub actuator_digest: String,
    pub states: Vec<HealthState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeDefinition {
    pub mode_id: String,
    /// Higher rank means a richer / less degraded operating mode.
    pub rank: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum TransitionGateStatus {
    Admitted,
    Rejected { witness_id: String },
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeTransitionReceipt {
    pub receipt_id: String,
    pub signature_id: String,
    pub signature_digest: String,
    pub current_mode_id: String,
    pub target_mode_id: String,
    pub transition_action_id: String,
    pub initial_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub result_digest: String,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub transition_duration_ns: u64,
    pub horizon_steps: u32,
    pub status: TransitionGateStatus,
    /// Actions independently admitted for this exact health signature and
    /// complete transition successor set.
    pub admitted_action_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailoverProblem {
    pub now_ns: u64,
    pub initial_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub actuator_digest: String,
    pub horizon_steps: u32,
    pub current_mode: ModeDefinition,
    pub target_mode: ModeDefinition,
    pub last_mode_change_ns: u64,
    pub minimum_upgrade_dwell_ns: u64,
    pub transition_action_id: String,
    pub transition_duration_ns: u64,
    pub maximum_transition_duration_ns: u64,
    pub preferred_fallback_action_id: String,
    pub health_evidence: Vec<HealthEvidence>,
    /// Complete set of health signatures still physically possible according
    /// to the upstream evidence/uncertainty interpreter. Unknown must expand,
    /// not narrow, this set.
    pub possible_signatures: Vec<PossibleHealthSignature>,
    pub transition_receipts: Vec<ModeTransitionReceipt>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FailoverDisposition {
    EnterTargetMode {
        mode_id: String,
        selected_action_id: String,
        admitted_signature_count: usize,
        common_admitted_action_ids: Vec<String>,
    },
    HoldCurrentMode {
        mode_id: String,
        reason: String,
    },
    PhysicalRejected {
        signature_id: String,
        witness_id: String,
    },
    PhysicalUnknown {
        signature_id: String,
        reason: String,
    },
    NoCommonAdmittedAction,
    PreferredActionNotAdmitted {
        preferred_action_id: String,
        common_admitted_action_ids: Vec<String>,
    },
    TransitionDeadlineExceeded {
        duration_ns: u64,
        maximum_duration_ns: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FailoverReport {
    pub disposition: FailoverDisposition,
    pub canonical_signature_ids: Vec<String>,
    pub channel_health_count: usize,
    pub downshift: bool,
    pub upgrade: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailoverError {
    EmptyIdentifier,
    InvalidHorizon,
    InvalidModeTransition,
    FutureLastModeChange,
    DuplicateHealthEvidence,
    DuplicateChannelHealth,
    InvalidHealthEvidenceInterval,
    StaleHealthEvidence,
    FutureHealthEvidence,
    NoPossibleSignatures,
    DuplicateSignature,
    EmptySignature,
    DuplicateChannelInSignature,
    UnknownHealthReceipt,
    DuplicateTransitionReceipt,
    MissingTransitionReceipt,
    ReceiptIdentityMismatch,
    InvalidReceiptInterval,
    StaleTransitionReceipt,
    TransitionDurationMismatch,
    HorizonMismatch,
    DuplicateAdmittedAction,
    EmptyAdmittedActionSet,
    EmptyTransitionWitness,
    EmptyUnknownReason,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_unique_ids<'a, I>(ids: I) -> Result<(), FailoverError>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut seen = BTreeSet::new();
    for id in ids {
        if !nonempty(id) {
            return Err(FailoverError::EmptyIdentifier);
        }
        if !seen.insert(id) {
            return Err(FailoverError::DuplicateHealthEvidence);
        }
    }
    Ok(())
}

fn signature_digest(signature: &PossibleHealthSignature) -> Result<String, FailoverError> {
    if !nonempty(&signature.signature_id)
        || !nonempty(&signature.health_evidence_digest)
        || !nonempty(&signature.actuator_digest)
    {
        return Err(FailoverError::EmptyIdentifier);
    }
    let mut states = BTreeMap::new();
    for state in &signature.states {
        if !nonempty(&state.channel_id) {
            return Err(FailoverError::EmptyIdentifier);
        }
        if states.insert(state.channel_id.as_str(), state.health).is_some() {
            return Err(FailoverError::DuplicateChannelInSignature);
        }
    }
    if states.is_empty() {
        return Err(FailoverError::EmptySignature);
    }
    let mut canonical = String::new();
    canonical.push_str(&signature.health_evidence_digest);
    canonical.push('|');
    canonical.push_str(&signature.actuator_digest);
    for (channel_id, health) in states {
        canonical.push('|');
        canonical.push_str(channel_id);
        canonical.push('=');
        canonical.push_str(match health {
            ChannelHealth::Valid => "valid",
            ChannelHealth::Degraded => "degraded",
            ChannelHealth::Ood => "ood",
            ChannelHealth::Unavailable => "unavailable",
            ChannelHealth::Unknown => "unknown",
        });
    }
    Ok(canonical)
}

fn validate_health_evidence(problem: &FailoverProblem) -> Result<(), FailoverError> {
    let mut evidence_ids = BTreeSet::new();
    let mut channel_ids = BTreeSet::new();
    for evidence in &problem.health_evidence {
        if !nonempty(&evidence.evidence_id)
            || !nonempty(&evidence.channel_id)
            || !nonempty(&evidence.calibration_digest)
            || !nonempty(&evidence.clock_mapping_digest)
            || !nonempty(&evidence.common_cause_group)
        {
            return Err(FailoverError::EmptyIdentifier);
        }
        if !evidence_ids.insert(evidence.evidence_id.as_str()) {
            return Err(FailoverError::DuplicateHealthEvidence);
        }
        if !channel_ids.insert(evidence.channel_id.as_str()) {
            return Err(FailoverError::DuplicateChannelHealth);
        }
        if evidence.acquired_at_ns > evidence.arrived_at_ns
            || evidence.arrived_at_ns > problem.now_ns
            || evidence.valid_until_ns < evidence.arrived_at_ns
        {
            return Err(FailoverError::InvalidHealthEvidenceInterval);
        }
        if problem.now_ns > evidence.valid_until_ns {
            return Err(FailoverError::StaleHealthEvidence);
        }
    }
    Ok(())
}

fn validate_receipt(
    problem: &FailoverProblem,
    signature: &PossibleHealthSignature,
    receipt: &ModeTransitionReceipt,
) -> Result<(), FailoverError> {
    if !nonempty(&receipt.receipt_id)
        || !nonempty(&receipt.signature_id)
        || !nonempty(&receipt.signature_digest)
        || !nonempty(&receipt.current_mode_id)
        || !nonempty(&receipt.target_mode_id)
        || !nonempty(&receipt.transition_action_id)
        || !nonempty(&receipt.initial_state_digest)
        || !nonempty(&receipt.model_digest)
        || !nonempty(&receipt.abstraction_digest)
        || !nonempty(&receipt.domain_digest)
        || !nonempty(&receipt.actuator_digest)
        || !nonempty(&receipt.result_digest)
    {
        return Err(FailoverError::EmptyIdentifier);
    }
    if receipt.valid_from_ns > receipt.valid_until_ns {
        return Err(FailoverError::InvalidReceiptInterval);
    }
    if problem.now_ns < receipt.valid_from_ns || problem.now_ns > receipt.valid_until_ns {
        return Err(FailoverError::StaleTransitionReceipt);
    }
    if receipt.signature_id != signature.signature_id
        || receipt.signature_digest != signature_digest(signature)?
        || receipt.current_mode_id != problem.current_mode.mode_id
        || receipt.target_mode_id != problem.target_mode.mode_id
        || receipt.transition_action_id != problem.transition_action_id
        || receipt.initial_state_digest != problem.initial_state_digest
        || receipt.model_digest != problem.model_digest
        || receipt.abstraction_digest != problem.abstraction_digest
        || receipt.domain_digest != problem.domain_digest
        || receipt.actuator_digest != problem.actuator_digest
    {
        return Err(FailoverError::ReceiptIdentityMismatch);
    }
    if receipt.transition_duration_ns != problem.transition_duration_ns {
        return Err(FailoverError::TransitionDurationMismatch);
    }
    if receipt.horizon_steps != problem.horizon_steps {
        return Err(FailoverError::HorizonMismatch);
    }
    validate_unique_ids(receipt.admitted_action_ids.iter().map(String::as_str))?;
    if receipt.admitted_action_ids.is_empty() {
        return Err(FailoverError::EmptyAdmittedActionSet);
    }
    Ok(())
}

pub fn evaluate_failover(
    problem: &FailoverProblem,
) -> Result<FailoverReport, FailoverError> {
    if !nonempty(&problem.initial_state_digest)
        || !nonempty(&problem.model_digest)
        || !nonempty(&problem.abstraction_digest)
        || !nonempty(&problem.domain_digest)
        || !nonempty(&problem.actuator_digest)
        || !nonempty(&problem.current_mode.mode_id)
        || !nonempty(&problem.target_mode.mode_id)
        || !nonempty(&problem.transition_action_id)
        || !nonempty(&problem.preferred_fallback_action_id)
    {
        return Err(FailoverError::EmptyIdentifier);
    }
    if problem.horizon_steps == 0 {
        return Err(FailoverError::InvalidHorizon);
    }
    if problem.current_mode.mode_id == problem.target_mode.mode_id
        || problem.transition_duration_ns == 0
        || problem.maximum_transition_duration_ns == 0
    {
        return Err(FailoverError::InvalidModeTransition);
    }
    if problem.last_mode_change_ns > problem.now_ns {
        return Err(FailoverError::FutureLastModeChange);
    }

    validate_health_evidence(problem)?;

    let downshift = problem.target_mode.rank < problem.current_mode.rank;
    let upgrade = problem.target_mode.rank > problem.current_mode.rank;
    if upgrade
        && problem.now_ns - problem.last_mode_change_ns < problem.minimum_upgrade_dwell_ns
    {
        return Ok(FailoverReport {
            disposition: FailoverDisposition::HoldCurrentMode {
                mode_id: problem.current_mode.mode_id.clone(),
                reason: "minimum upgrade dwell has not elapsed".to_owned(),
            },
            canonical_signature_ids: Vec::new(),
            channel_health_count: problem.health_evidence.len(),
            downshift,
            upgrade,
        });
    }

    if problem.transition_duration_ns > problem.maximum_transition_duration_ns {
        return Ok(FailoverReport {
            disposition: FailoverDisposition::TransitionDeadlineExceeded {
                duration_ns: problem.transition_duration_ns,
                maximum_duration_ns: problem.maximum_transition_duration_ns,
            },
            canonical_signature_ids: Vec::new(),
            channel_health_count: problem.health_evidence.len(),
            downshift,
            upgrade,
        });
    }

    if problem.possible_signatures.is_empty() {
        return Err(FailoverError::NoPossibleSignatures);
    }
    validate_unique_ids(
        problem
            .possible_signatures
            .iter()
            .map(|signature| signature.signature_id.as_str()),
    )?;

    let mut signatures = problem.possible_signatures.clone();
    for signature in &signatures {
        let _ = signature_digest(signature)?;
    }
    signatures.sort_by(|left, right| left.signature_id.cmp(&right.signature_id));
    let canonical_signature_ids = signatures
        .iter()
        .map(|signature| signature.signature_id.clone())
        .collect::<Vec<_>>();

    let mut receipts = BTreeMap::new();
    for receipt in &problem.transition_receipts {
        if !nonempty(&receipt.signature_id) {
            return Err(FailoverError::EmptyIdentifier);
        }
        if receipts.insert(receipt.signature_id.as_str(), receipt).is_some() {
            return Err(FailoverError::DuplicateTransitionReceipt);
        }
    }

    let mut common_actions: Option<BTreeSet<String>> = None;
    for signature in &signatures {
        let receipt = receipts
            .get(signature.signature_id.as_str())
            .copied()
            .ok_or(FailoverError::MissingTransitionReceipt)?;
        validate_receipt(problem, signature, receipt)?;

        match &receipt.status {
            TransitionGateStatus::Rejected { witness_id } => {
                if !nonempty(witness_id) {
                    return Err(FailoverError::EmptyTransitionWitness);
                }
                return Ok(FailoverReport {
                    disposition: FailoverDisposition::PhysicalRejected {
                        signature_id: signature.signature_id.clone(),
                        witness_id: witness_id.clone(),
                    },
                    canonical_signature_ids,
                    channel_health_count: problem.health_evidence.len(),
                    downshift,
                    upgrade,
                });
            }
            TransitionGateStatus::Unknown { reason } => {
                if !nonempty(reason) {
                    return Err(FailoverError::EmptyUnknownReason);
                }
                return Ok(FailoverReport {
                    disposition: FailoverDisposition::PhysicalUnknown {
                        signature_id: signature.signature_id.clone(),
                        reason: reason.clone(),
                    },
                    canonical_signature_ids,
                    channel_health_count: problem.health_evidence.len(),
                    downshift,
                    upgrade,
                });
            }
            TransitionGateStatus::Admitted => {}
        }

        let actions = receipt
            .admitted_action_ids
            .iter()
            .cloned()
            .collect::<BTreeSet<_>>();
        common_actions = Some(match common_actions {
            None => actions,
            Some(current) => current.intersection(&actions).cloned().collect(),
        });
    }

    let common = common_actions.unwrap_or_default();
    let common_admitted_action_ids = common.iter().cloned().collect::<Vec<_>>();
    if common.is_empty() {
        return Ok(FailoverReport {
            disposition: FailoverDisposition::NoCommonAdmittedAction,
            canonical_signature_ids,
            channel_health_count: problem.health_evidence.len(),
            downshift,
            upgrade,
        });
    }

    if !common.contains(&problem.preferred_fallback_action_id) {
        return Ok(FailoverReport {
            disposition: FailoverDisposition::PreferredActionNotAdmitted {
                preferred_action_id: problem.preferred_fallback_action_id.clone(),
                common_admitted_action_ids,
            },
            canonical_signature_ids,
            channel_health_count: problem.health_evidence.len(),
            downshift,
            upgrade,
        });
    }

    Ok(FailoverReport {
        disposition: FailoverDisposition::EnterTargetMode {
            mode_id: problem.target_mode.mode_id.clone(),
            selected_action_id: problem.preferred_fallback_action_id.clone(),
            admitted_signature_count: signatures.len(),
            common_admitted_action_ids,
        },
        canonical_signature_ids,
        channel_health_count: problem.health_evidence.len(),
        downshift,
        upgrade,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn signature(id: &str, states: Vec<(&str, ChannelHealth)>) -> PossibleHealthSignature {
        PossibleHealthSignature {
            signature_id: id.to_owned(),
            health_evidence_digest: format!("health-{id}"),
            actuator_digest: "actuator-v1".to_owned(),
            states: states
                .into_iter()
                .map(|(channel_id, health)| HealthState {
                    channel_id: channel_id.to_owned(),
                    health,
                })
                .collect(),
        }
    }

    fn receipt(
        problem: &FailoverProblem,
        signature: &PossibleHealthSignature,
        actions: &[&str],
        status: TransitionGateStatus,
    ) -> ModeTransitionReceipt {
        ModeTransitionReceipt {
            receipt_id: format!("receipt-{}", signature.signature_id),
            signature_id: signature.signature_id.clone(),
            signature_digest: signature_digest(signature).expect("signature digest"),
            current_mode_id: problem.current_mode.mode_id.clone(),
            target_mode_id: problem.target_mode.mode_id.clone(),
            transition_action_id: problem.transition_action_id.clone(),
            initial_state_digest: problem.initial_state_digest.clone(),
            model_digest: problem.model_digest.clone(),
            abstraction_digest: problem.abstraction_digest.clone(),
            domain_digest: problem.domain_digest.clone(),
            actuator_digest: problem.actuator_digest.clone(),
            result_digest: format!("result-{}", signature.signature_id),
            valid_from_ns: 100,
            valid_until_ns: 1_000,
            transition_duration_ns: problem.transition_duration_ns,
            horizon_steps: problem.horizon_steps,
            status,
            admitted_action_ids: actions.iter().map(|item| (*item).to_owned()).collect(),
        }
    }

    fn base_problem() -> FailoverProblem {
        let mut problem = FailoverProblem {
            now_ns: 500,
            initial_state_digest: "state-001".to_owned(),
            model_digest: "model-001".to_owned(),
            abstraction_digest: "abstraction-001".to_owned(),
            domain_digest: "domain-001".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            horizon_steps: 20,
            current_mode: ModeDefinition {
                mode_id: "full-perception".to_owned(),
                rank: 3,
            },
            target_mode: ModeDefinition {
                mode_id: "proprioceptive-haptic".to_owned(),
                rank: 1,
            },
            last_mode_change_ns: 450,
            minimum_upgrade_dwell_ns: 100,
            transition_action_id: "transition-to-hold".to_owned(),
            transition_duration_ns: 50,
            maximum_transition_duration_ns: 100,
            preferred_fallback_action_id: "hold".to_owned(),
            health_evidence: vec![
                HealthEvidence {
                    evidence_id: "health-depth".to_owned(),
                    channel_id: "depth-camera".to_owned(),
                    calibration_digest: "depth-cal-v1".to_owned(),
                    clock_mapping_digest: "clock-v1".to_owned(),
                    common_cause_group: "camera-unit-1".to_owned(),
                    health: ChannelHealth::Ood,
                    acquired_at_ns: 490,
                    arrived_at_ns: 495,
                    valid_until_ns: 550,
                },
                HealthEvidence {
                    evidence_id: "health-joints".to_owned(),
                    channel_id: "joint-encoders".to_owned(),
                    calibration_digest: "joint-cal-v1".to_owned(),
                    clock_mapping_digest: "clock-v1".to_owned(),
                    common_cause_group: "encoder-bus-1".to_owned(),
                    health: ChannelHealth::Valid,
                    acquired_at_ns: 490,
                    arrived_at_ns: 495,
                    valid_until_ns: 550,
                },
            ],
            possible_signatures: Vec::new(),
            transition_receipts: Vec::new(),
        };
        let sig = signature(
            "vision-ood",
            vec![
                ("depth-camera", ChannelHealth::Ood),
                ("joint-encoders", ChannelHealth::Valid),
            ],
        );
        problem.transition_receipts = vec![receipt(
            &problem,
            &sig,
            &["hold", "retreat"],
            TransitionGateStatus::Admitted,
        )];
        problem.possible_signatures = vec![sig];
        problem
    }

    fn as_enter(report: &FailoverReport) -> (&str, &str) {
        match &report.disposition {
            FailoverDisposition::EnterTargetMode {
                mode_id,
                selected_action_id,
                ..
            } => (mode_id, selected_action_id),
            other => panic!("expected mode entry, got {other:?}"),
        }
    }

    #[test]
    fn admitted_vision_fallback_enters_only_with_common_action() {
        let report = evaluate_failover(&base_problem()).expect("valid failover");
        assert_eq!(
            as_enter(&report),
            ("proprioceptive-haptic", "hold")
        );
        assert_eq!(report.canonical_signature_ids, vec!["vision-ood".to_owned()]);
    }

    #[test]
    fn multiple_health_signatures_intersect_admitted_actions() {
        let mut problem = base_problem();
        let second = signature(
            "vision-ood-d3-degraded",
            vec![
                ("depth-camera", ChannelHealth::Ood),
                ("joint-encoders", ChannelHealth::Degraded),
            ],
        );
        problem.transition_receipts.push(receipt(
            &problem,
            &second,
            &["hold"],
            TransitionGateStatus::Admitted,
        ));
        problem.possible_signatures.push(second);
        let report = evaluate_failover(&problem).expect("intersection");
        assert_eq!(as_enter(&report).1, "hold");
        if let FailoverDisposition::EnterTargetMode {
            common_admitted_action_ids,
            ..
        } = report.disposition
        {
            assert_eq!(common_admitted_action_ids, vec!["hold".to_owned()]);
        } else {
            panic!("expected enter mode");
        }
    }

    #[test]
    fn preferred_action_outside_intersection_is_not_selected() {
        let mut problem = base_problem();
        problem.preferred_fallback_action_id = "retreat".to_owned();
        let second = signature(
            "d3-degraded",
            vec![
                ("depth-camera", ChannelHealth::Ood),
                ("joint-encoders", ChannelHealth::Degraded),
            ],
        );
        problem.transition_receipts.push(receipt(
            &problem,
            &second,
            &["hold"],
            TransitionGateStatus::Admitted,
        ));
        problem.possible_signatures.push(second);
        let report = evaluate_failover(&problem).expect("preferred rejection");
        assert!(matches!(
            report.disposition,
            FailoverDisposition::PreferredActionNotAdmitted { .. }
        ));
    }

    #[test]
    fn no_common_admitted_action_returns_unknown() {
        let mut problem = base_problem();
        let first = &mut problem.transition_receipts[0];
        first.admitted_action_ids = vec!["advance".to_owned()];
        let second = signature(
            "d2-degraded",
            vec![
                ("depth-camera", ChannelHealth::Ood),
                ("joint-encoders", ChannelHealth::Valid),
                ("foot-wrench", ChannelHealth::Degraded),
            ],
        );
        problem.transition_receipts.push(receipt(
            &problem,
            &second,
            &["retreat"],
            TransitionGateStatus::Admitted,
        ));
        problem.possible_signatures.push(second);
        let report = evaluate_failover(&problem).expect("empty intersection");
        assert_eq!(report.disposition, FailoverDisposition::NoCommonAdmittedAction);
    }

    #[test]
    fn rejected_physical_signature_blocks_fallback() {
        let mut problem = base_problem();
        let rejected = signature(
            "d3-unavailable",
            vec![
                ("depth-camera", ChannelHealth::Ood),
                ("joint-encoders", ChannelHealth::Unavailable),
            ],
        );
        problem.transition_receipts.push(receipt(
            &problem,
            &rejected,
            &[],
            TransitionGateStatus::Rejected {
                witness_id: "no-safe-stance".to_owned(),
            },
        ));
        problem.possible_signatures.push(rejected);
        let report = evaluate_failover(&problem).expect("rejected signature");
        assert!(matches!(
            report.disposition,
            FailoverDisposition::PhysicalRejected { .. }
        ));
    }

    #[test]
    fn unknown_signature_cannot_be_deleted_by_probability() {
        let mut problem = base_problem();
        let unknown = signature(
            "d3-unknown",
            vec![
                ("depth-camera", ChannelHealth::Ood),
                ("joint-encoders", ChannelHealth::Unknown),
            ],
        );
        problem.transition_receipts.push(receipt(
            &problem,
            &unknown,
            &[],
            TransitionGateStatus::Unknown {
                reason: "unbounded encoder health".to_owned(),
            },
        ));
        problem.possible_signatures.push(unknown);
        let report = evaluate_failover(&problem).expect("unknown signature");
        assert!(matches!(
            report.disposition,
            FailoverDisposition::PhysicalUnknown { .. }
        ));
    }

    #[test]
    fn upgrade_dwell_holds_current_mode_without_evaluating_target_receipts() {
        let mut problem = base_problem();
        problem.current_mode = ModeDefinition {
            mode_id: "proprioceptive-haptic".to_owned(),
            rank: 1,
        };
        problem.target_mode = ModeDefinition {
            mode_id: "full-perception".to_owned(),
            rank: 3,
        };
        problem.last_mode_change_ns = 490;
        problem.minimum_upgrade_dwell_ns = 100;
        problem.possible_signatures.clear();
        problem.transition_receipts.clear();
        let report = evaluate_failover(&problem).expect("dwell hold");
        assert!(matches!(
            report.disposition,
            FailoverDisposition::HoldCurrentMode { .. }
        ));
    }

    #[test]
    fn safety_downshift_is_not_delayed_by_upgrade_dwell() {
        let mut problem = base_problem();
        problem.last_mode_change_ns = 499;
        problem.minimum_upgrade_dwell_ns = 10_000;
        let report = evaluate_failover(&problem).expect("downshift");
        assert!(matches!(
            report.disposition,
            FailoverDisposition::EnterTargetMode { .. }
        ));
    }

    #[test]
    fn stale_health_evidence_is_rejected() {
        let mut problem = base_problem();
        problem.health_evidence[0].valid_until_ns = 499;
        assert_eq!(
            evaluate_failover(&problem),
            Err(FailoverError::StaleHealthEvidence)
        );
    }

    #[test]
    fn duplicate_health_evidence_is_rejected() {
        let mut problem = base_problem();
        problem.health_evidence.push(problem.health_evidence[0].clone());
        assert_eq!(
            evaluate_failover(&problem),
            Err(FailoverError::DuplicateHealthEvidence)
        );
    }

    #[test]
    fn actuator_receipt_mismatch_is_rejected() {
        let mut problem = base_problem();
        problem.actuator_digest = "actuator-derated-v2".to_owned();
        assert_eq!(
            evaluate_failover(&problem),
            Err(FailoverError::ReceiptIdentityMismatch)
        );
    }

    #[test]
    fn stale_transition_receipt_is_rejected() {
        let mut problem = base_problem();
        problem.transition_receipts[0].valid_until_ns = 499;
        assert_eq!(
            evaluate_failover(&problem),
            Err(FailoverError::StaleTransitionReceipt)
        );
    }

    #[test]
    fn transition_deadline_is_hard_gate() {
        let mut problem = base_problem();
        problem.transition_duration_ns = 101;
        let report = evaluate_failover(&problem).expect("deadline report");
        assert!(matches!(
            report.disposition,
            FailoverDisposition::TransitionDeadlineExceeded { .. }
        ));
    }

    #[test]
    fn canonical_signature_order_is_input_order_invariant() {
        let mut problem = base_problem();
        let second = signature(
            "d2-degraded",
            vec![
                ("depth-camera", ChannelHealth::Ood),
                ("joint-encoders", ChannelHealth::Valid),
                ("foot-wrench", ChannelHealth::Degraded),
            ],
        );
        problem.transition_receipts.push(receipt(
            &problem,
            &second,
            &["hold", "retreat"],
            TransitionGateStatus::Admitted,
        ));
        problem.possible_signatures.push(second);
        let original = evaluate_failover(&problem).expect("original");
        problem.possible_signatures.reverse();
        problem.transition_receipts.reverse();
        let permuted = evaluate_failover(&problem).expect("permuted");
        assert_eq!(original, permuted);
    }

    #[test]
    fn duplicate_channel_in_signature_is_rejected() {
        let mut problem = base_problem();
        problem.possible_signatures[0].states.push(HealthState {
            channel_id: "depth-camera".to_owned(),
            health: ChannelHealth::Valid,
        });
        assert_eq!(
            evaluate_failover(&problem),
            Err(FailoverError::DuplicateChannelInSignature)
        );
    }
}
