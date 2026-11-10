//! Fault diagnosis, robust action-admission and graceful-recovery episode auditor.
//!
//! This crate consumes already-qualified physical fault possibility sets and
//! per-hypothesis admission results. It does not classify raw sensors or
//! compute viability.

use std::collections::BTreeSet;

pub const PPM_SCALE: u32 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceTier {
    Simulation,
    HardwareInLoop,
    TetheredHardware,
    Hardware,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EpisodeIdentity {
    pub episode_id: String,
    pub robot_digest: String,
    pub scenario_digest: String,
    pub controller_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub actuator_digest: String,
    pub domain_digest: String,
    pub initial_state_digest: String,
    pub evidence_tier: EvidenceTier,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActualFault {
    /// Evaluator-only ground truth. This field must never be exposed to the
    /// controller or its diagnosis policy.
    pub present: bool,
    pub hypothesis_id: Option<String>,
    pub onset_ns: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiagnosisState {
    Nominal,
    Suspected,
    Isolated {
        hypothesis_id: String,
        confidence_ppm: u32,
    },
    Unknown {
        reason: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiagnosisSnapshot {
    pub observed_at_ns: u64,
    pub state: DiagnosisState,
    /// Controller-side plausible set. This is not the authoritative physical
    /// possibility set used for action admission.
    pub controller_plausible_fault_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdmissionStatus {
    Admitted,
    Rejected,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultAdmission {
    pub fault_hypothesis_id: String,
    pub status: AdmissionStatus,
    pub result_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AdmissionReceipt {
    pub receipt_id: String,
    pub action_id: String,
    pub physical_fault_set_digest: String,
    pub state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub actuator_digest: String,
    pub domain_digest: String,
    pub result_digest: String,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActionDecision {
    pub observed_at_ns: u64,
    pub current_state_digest: String,
    pub action_id: String,
    pub physical_fault_set_digest: String,
    pub selected: bool,
    pub physically_possible_fault_ids: Vec<String>,
    /// Must contain one result for each physically possible fault hypothesis.
    pub admissions: Vec<FaultAdmission>,
    pub receipt: AdmissionReceipt,
    pub deadline_ns: u64,
    /// None means application was not independently confirmed.
    pub applied_at_ns: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OperatingMode {
    Nominal,
    Suspected,
    DegradedMobile,
    SafeHold,
    Recovery,
    Retreat,
    AbortedSafe,
    Unrecoverable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ModeTransition {
    pub at_ns: u64,
    pub mode: OperatingMode,
    pub evidence_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TerminalDisposition {
    TaskCompleted,
    SafeHold,
    Recovered,
    Retreated,
    AbortedSafe,
    Unrecoverable,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultEpisode {
    pub identity: EpisodeIdentity,
    pub now_ns: u64,
    pub actual_fault: ActualFault,
    pub diagnosis: Vec<DiagnosisSnapshot>,
    pub action_decisions: Vec<ActionDecision>,
    pub mode_transitions: Vec<ModeTransition>,
    pub terminal_disposition: TerminalDisposition,
    /// Independently observed time when the robot reached a declared safe
    /// physical state. A controller mode label is not sufficient.
    pub safe_state_reached_ns: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateViolation {
    PhysicalFaultSetOmitsGroundTruth {
        at_ns: u64,
        fault_hypothesis_id: String,
    },
    MissingAdmissionForPhysicalFault {
        at_ns: u64,
        fault_hypothesis_id: String,
    },
    RejectedForPhysicalFault {
        at_ns: u64,
        fault_hypothesis_id: String,
    },
    UnknownAdmissionForPhysicalFault {
        at_ns: u64,
        fault_hypothesis_id: String,
    },
    InvalidOrStaleReceipt {
        at_ns: u64,
    },
    UnconfirmedApplication {
        at_ns: u64,
    },
    AppliedAfterDeadline {
        at_ns: u64,
        deadline_ns: u64,
        applied_at_ns: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FaultEpisodeReport {
    pub episode_id: String,
    pub evidence_tier: EvidenceTier,
    pub detection_latency_ns: Option<u64>,
    pub correct_isolation_latency_ns: Option<u64>,
    pub wrong_isolation_count: u32,
    pub false_alarm: bool,
    pub controller_excluded_ground_truth_count: u32,
    pub gate_violations: Vec<GateViolation>,
    pub safe_state_latency_ns: Option<u64>,
    pub terminal_disposition: TerminalDisposition,
    pub task_completed: bool,
    pub safe_state_reached: bool,
    /// True only when a safe physical terminal state was observed and every
    /// selected action was admitted under every physical fault possibility.
    pub operational_outcome_qualified: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FaultEpisodeError {
    EmptyIdentifier,
    InvalidActualFault,
    MissingActualFaultIdentity,
    FaultOnsetAfterEvaluation,
    DuplicateDiagnosisTime,
    DiagnosisTimeOutOfOrder,
    InvalidDiagnosisConfidence,
    IsolationBelowQualificationThreshold,
    InvalidDiagnosisState,
    DuplicateControllerHypothesis,
    DuplicatePhysicalHypothesis,
    DuplicateAdmissionHypothesis,
    AdmissionForImpossibleFault,
    MissingFaultAdmissionForSelectedAction,
    MissingActionIdentity,
    ActionDecisionTimeOutOfOrder,
    ActionApplicationBeforeDecision,
    ActionApplicationAfterEvaluation,
    ActionDeadlineBeforeDecision,
    EmptyPhysicalFaultSet,
    InvalidReceiptInterval,
    ModeTransitionOutOfOrder,
    DuplicateModeTransitionTime,
    MissingModeEvidence,
    SafeStateTimeAfterEvaluation,
    SafeStateBeforeFaultOnset,
    SafeDispositionMissingPhysicalSafeState,
    UnsafeDispositionHasSafeStateTime,
    EvaluationBeforeEvidence,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn validate_unique_ids(
    values: &[String],
    duplicate_error: FaultEpisodeError,
) -> Result<(), FaultEpisodeError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !nonempty(value) {
            return Err(FaultEpisodeError::EmptyIdentifier);
        }
        if !seen.insert(value.as_str()) {
            return Err(duplicate_error);
        }
    }
    Ok(())
}

fn is_safe_terminal(disposition: TerminalDisposition) -> bool {
    matches!(
        disposition,
        TerminalDisposition::TaskCompleted
            | TerminalDisposition::SafeHold
            | TerminalDisposition::Recovered
            | TerminalDisposition::Retreated
            | TerminalDisposition::AbortedSafe
    )
}

fn receipt_is_valid(
    receipt: &AdmissionReceipt,
    identity: &EpisodeIdentity,
    action_id: &str,
    physical_fault_set_digest: &str,
    state_digest: &str,
    at_ns: u64,
) -> bool {
    nonempty(&receipt.receipt_id)
        && nonempty(&receipt.result_digest)
        && nonempty(&receipt.action_id)
        && nonempty(&receipt.physical_fault_set_digest)
        && receipt.action_id == action_id
        && receipt.physical_fault_set_digest == physical_fault_set_digest
        && receipt.valid_from_ns <= receipt.valid_until_ns
        && at_ns >= receipt.valid_from_ns
        && at_ns <= receipt.valid_until_ns
        && receipt.state_digest == state_digest
        && receipt.model_digest == identity.model_digest
        && receipt.abstraction_digest == identity.abstraction_digest
        && receipt.actuator_digest == identity.actuator_digest
        && receipt.domain_digest == identity.domain_digest
}

fn validate_identity(identity: &EpisodeIdentity) -> Result<(), FaultEpisodeError> {
    let required = [
        identity.episode_id.as_str(),
        identity.robot_digest.as_str(),
        identity.scenario_digest.as_str(),
        identity.controller_digest.as_str(),
        identity.model_digest.as_str(),
        identity.abstraction_digest.as_str(),
        identity.actuator_digest.as_str(),
        identity.domain_digest.as_str(),
        identity.initial_state_digest.as_str(),
    ];
    if required.iter().any(|value| !nonempty(value)) {
        return Err(FaultEpisodeError::EmptyIdentifier);
    }
    Ok(())
}

fn evaluate_action_decision(
    decision: &ActionDecision,
    episode: &FaultEpisode,
) -> Result<Vec<GateViolation>, FaultEpisodeError> {
    if !nonempty(&decision.action_id) {
        return Err(FaultEpisodeError::MissingActionIdentity);
    }
    if !nonempty(&decision.current_state_digest)
        || !nonempty(&decision.physical_fault_set_digest)
    {
        return Err(FaultEpisodeError::EmptyIdentifier);
    }
    if decision.physically_possible_fault_ids.is_empty() {
        return Err(FaultEpisodeError::EmptyPhysicalFaultSet);
    }
    validate_unique_ids(
        &decision.physically_possible_fault_ids,
        FaultEpisodeError::DuplicatePhysicalHypothesis,
    )?;
    if decision.deadline_ns < decision.observed_at_ns {
        return Err(FaultEpisodeError::ActionDeadlineBeforeDecision);
    }
    if decision
        .applied_at_ns
        .is_some_and(|applied_at| applied_at < decision.observed_at_ns)
    {
        return Err(FaultEpisodeError::ActionApplicationBeforeDecision);
    }
    if decision
        .applied_at_ns
        .is_some_and(|applied_at| applied_at > episode.now_ns)
    {
        return Err(FaultEpisodeError::ActionApplicationAfterEvaluation);
    }

    let mut admission_map = std::collections::BTreeMap::new();
    for admission in &decision.admissions {
        if !nonempty(&admission.fault_hypothesis_id) || !nonempty(&admission.result_digest) {
            return Err(FaultEpisodeError::EmptyIdentifier);
        }
        if !decision
            .physically_possible_fault_ids
            .contains(&admission.fault_hypothesis_id)
        {
            return Err(FaultEpisodeError::AdmissionForImpossibleFault);
        }
        if admission_map
            .insert(admission.fault_hypothesis_id.as_str(), admission.status)
            .is_some()
        {
            return Err(FaultEpisodeError::DuplicateAdmissionHypothesis);
        }
    }

    if !decision.selected {
        return Ok(Vec::new());
    }

    let mut violations = Vec::new();
    if !receipt_is_valid(
        &decision.receipt,
        &episode.identity,
        &decision.action_id,
        &decision.physical_fault_set_digest,
        &decision.current_state_digest,
        decision.observed_at_ns,
    ) {
        violations.push(GateViolation::InvalidOrStaleReceipt {
            at_ns: decision.observed_at_ns,
        });
    }

    if episode.actual_fault.present {
        let actual_id = episode
            .actual_fault
            .hypothesis_id
            .as_deref()
            .ok_or(FaultEpisodeError::MissingActualFaultIdentity)?;
        if decision.observed_at_ns >= episode.actual_fault.onset_ns.unwrap_or(u64::MAX)
            && !decision
                .physically_possible_fault_ids
                .iter()
                .any(|fault_id| fault_id == actual_id)
        {
            violations.push(GateViolation::PhysicalFaultSetOmitsGroundTruth {
                at_ns: decision.observed_at_ns,
                fault_hypothesis_id: actual_id.to_owned(),
            });
        }
    }

    for fault_id in &decision.physically_possible_fault_ids {
        match admission_map.get(fault_id.as_str()) {
            None => violations.push(GateViolation::MissingAdmissionForPhysicalFault {
                at_ns: decision.observed_at_ns,
                fault_hypothesis_id: fault_id.clone(),
            }),
            Some(AdmissionStatus::Rejected) => {
                violations.push(GateViolation::RejectedForPhysicalFault {
                    at_ns: decision.observed_at_ns,
                    fault_hypothesis_id: fault_id.clone(),
                });
            }
            Some(AdmissionStatus::Unknown) => {
                violations.push(GateViolation::UnknownAdmissionForPhysicalFault {
                    at_ns: decision.observed_at_ns,
                    fault_hypothesis_id: fault_id.clone(),
                });
            }
            Some(AdmissionStatus::Admitted) => {}
        }
    }

    match decision.applied_at_ns {
        None => violations.push(GateViolation::UnconfirmedApplication {
            at_ns: decision.observed_at_ns,
        }),
        Some(applied_at_ns) if applied_at_ns > decision.deadline_ns => {
            violations.push(GateViolation::AppliedAfterDeadline {
                at_ns: decision.observed_at_ns,
                deadline_ns: decision.deadline_ns,
                applied_at_ns,
            });
        }
        Some(_) => {}
    }

    Ok(violations)
}

/// Evaluate a fault episode against its evaluator-only ground truth and the
/// upstream physical admission evidence supplied at every selected action.
pub fn audit_fault_episode(
    episode: &FaultEpisode,
) -> Result<FaultEpisodeReport, FaultEpisodeError> {
    validate_identity(&episode.identity)?;
    if episode.actual_fault.present {
        if episode.actual_fault.hypothesis_id.as_deref().is_none_or(|id| !nonempty(id)) {
            return Err(FaultEpisodeError::MissingActualFaultIdentity);
        }
        if episode.actual_fault.onset_ns.is_none() {
            return Err(FaultEpisodeError::InvalidActualFault);
        }
        if episode.actual_fault.onset_ns.is_some_and(|onset| onset > episode.now_ns) {
            return Err(FaultEpisodeError::FaultOnsetAfterEvaluation);
        }
    } else if episode.actual_fault.hypothesis_id.is_some() || episode.actual_fault.onset_ns.is_some() {
        return Err(FaultEpisodeError::InvalidActualFault);
    }

    let actual_id = episode.actual_fault.hypothesis_id.as_deref();
    let onset_ns = episode.actual_fault.onset_ns;
    let mut detection_latency_ns = None;
    let mut correct_isolation_latency_ns = None;
    let mut wrong_isolation_count = 0_u32;
    let mut false_alarm = false;
    let mut controller_excluded_ground_truth_count = 0_u32;
    let mut previous_diagnosis_time = None;

    for snapshot in &episode.diagnosis {
        if snapshot.observed_at_ns > episode.now_ns {
            return Err(FaultEpisodeError::EvaluationBeforeEvidence);
        }
        if previous_diagnosis_time.is_some_and(|previous| snapshot.observed_at_ns < previous) {
            return Err(FaultEpisodeError::DiagnosisTimeOutOfOrder);
        }
        if previous_diagnosis_time == Some(snapshot.observed_at_ns) {
            return Err(FaultEpisodeError::DuplicateDiagnosisTime);
        }
        previous_diagnosis_time = Some(snapshot.observed_at_ns);
        validate_unique_ids(
            &snapshot.controller_plausible_fault_ids,
            FaultEpisodeError::DuplicateControllerHypothesis,
        )?;

        match &snapshot.state {
            DiagnosisState::Nominal => {}
            DiagnosisState::Suspected => {
                if !episode.actual_fault.present {
                    false_alarm = true;
                } else if onset_ns.is_some_and(|onset| snapshot.observed_at_ns >= onset)
                    && detection_latency_ns.is_none()
                {
                    detection_latency_ns =
                        Some(snapshot.observed_at_ns.saturating_sub(onset_ns.unwrap_or(0)));
                }
            }
            DiagnosisState::Isolated {
                hypothesis_id,
                confidence_ppm,
            } => {
                if !nonempty(hypothesis_id) {
                    return Err(FaultEpisodeError::InvalidDiagnosisState);
                }
                if *confidence_ppm > PPM_SCALE {
                    return Err(FaultEpisodeError::InvalidDiagnosisConfidence);
                }
                if *confidence_ppm < 900_000 {
                    return Err(FaultEpisodeError::IsolationBelowQualificationThreshold);
                }
                if !snapshot
                    .controller_plausible_fault_ids
                    .iter()
                    .any(|id| id == hypothesis_id)
                {
                    return Err(FaultEpisodeError::InvalidDiagnosisState);
                }
                if !episode.actual_fault.present {
                    false_alarm = true;
                } else if onset_ns.is_some_and(|onset| snapshot.observed_at_ns >= onset) {
                    let onset = onset_ns.unwrap_or(0);
                    if hypothesis_id == actual_id.unwrap_or_default()
                        && correct_isolation_latency_ns.is_none()
                    {
                        correct_isolation_latency_ns =
                            Some(snapshot.observed_at_ns.saturating_sub(onset));
                    } else if hypothesis_id != actual_id.unwrap_or_default() {
                        wrong_isolation_count = wrong_isolation_count
                            .checked_add(1)
                            .ok_or(FaultEpisodeError::InvalidDiagnosisState)?;
                    }
                    if detection_latency_ns.is_none() {
                        detection_latency_ns = Some(snapshot.observed_at_ns.saturating_sub(onset));
                    }
                }
            }
            DiagnosisState::Unknown { reason } => {
                if !nonempty(reason) {
                    return Err(FaultEpisodeError::InvalidDiagnosisState);
                }
            }
        }

        if episode.actual_fault.present
            && onset_ns.is_some_and(|onset| snapshot.observed_at_ns >= onset)
            && actual_id.is_some_and(|id| {
                !snapshot
                    .controller_plausible_fault_ids
                    .iter()
                    .any(|candidate| candidate == id)
            })
        {
            controller_excluded_ground_truth_count = controller_excluded_ground_truth_count
                .checked_add(1)
                .ok_or(FaultEpisodeError::InvalidDiagnosisState)?;
        }
    }

    let mut previous_action_time = None;
    let mut gate_violations = Vec::new();
    for decision in &episode.action_decisions {
        if decision.observed_at_ns > episode.now_ns {
            return Err(FaultEpisodeError::EvaluationBeforeEvidence);
        }
        if previous_action_time.is_some_and(|previous| decision.observed_at_ns < previous) {
            return Err(FaultEpisodeError::ActionDecisionTimeOutOfOrder);
        }
        previous_action_time = Some(decision.observed_at_ns);
        gate_violations.extend(evaluate_action_decision(decision, episode)?);
    }

    let mut previous_mode_time = None;
    for transition in &episode.mode_transitions {
        if transition.at_ns > episode.now_ns {
            return Err(FaultEpisodeError::EvaluationBeforeEvidence);
        }
        if !nonempty(&transition.evidence_digest) {
            return Err(FaultEpisodeError::MissingModeEvidence);
        }
        if previous_mode_time.is_some_and(|previous| transition.at_ns < previous) {
            return Err(FaultEpisodeError::ModeTransitionOutOfOrder);
        }
        if previous_mode_time == Some(transition.at_ns) {
            return Err(FaultEpisodeError::DuplicateModeTransitionTime);
        }
        previous_mode_time = Some(transition.at_ns);
    }

    let safe_terminal = is_safe_terminal(episode.terminal_disposition);
    let safe_state_reached = episode.safe_state_reached_ns.is_some();
    if safe_terminal && !safe_state_reached {
        return Err(FaultEpisodeError::SafeDispositionMissingPhysicalSafeState);
    }
    if !safe_terminal && safe_state_reached {
        return Err(FaultEpisodeError::UnsafeDispositionHasSafeStateTime);
    }
    if let Some(safe_at) = episode.safe_state_reached_ns {
        if safe_at > episode.now_ns {
            return Err(FaultEpisodeError::EvaluationBeforeEvidence);
        }
        if onset_ns.is_some_and(|onset| safe_at < onset) {
            return Err(FaultEpisodeError::SafeStateBeforeFaultOnset);
        }
    }

    let safe_state_latency_ns = match (onset_ns, episode.safe_state_reached_ns) {
        (Some(onset), Some(safe_at)) => Some(safe_at.saturating_sub(onset)),
        _ => None,
    };
    let operational_outcome_qualified =
        safe_state_reached && gate_violations.is_empty()
            && episode.terminal_disposition != TerminalDisposition::Unknown;

    Ok(FaultEpisodeReport {
        episode_id: episode.identity.episode_id.clone(),
        evidence_tier: episode.identity.evidence_tier,
        detection_latency_ns,
        correct_isolation_latency_ns,
        wrong_isolation_count,
        false_alarm,
        controller_excluded_ground_truth_count,
        gate_violations,
        safe_state_latency_ns,
        terminal_disposition: episode.terminal_disposition,
        task_completed: episode.terminal_disposition == TerminalDisposition::TaskCompleted,
        safe_state_reached,
        operational_outcome_qualified,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> EpisodeIdentity {
        EpisodeIdentity {
            episode_id: "episode-001".to_owned(),
            robot_digest: "robot-v1".to_owned(),
            scenario_digest: "scenario-v1".to_owned(),
            controller_digest: "controller-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            abstraction_digest: "abstraction-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            domain_digest: "domain-v1".to_owned(),
            initial_state_digest: "state-v1".to_owned(),
            evidence_tier: EvidenceTier::Simulation,
        }
    }

    fn receipt() -> AdmissionReceipt {
        AdmissionReceipt {
            receipt_id: "receipt-001".to_owned(),
            action_id: "safe-hold".to_owned(),
            physical_fault_set_digest: "fault-set-v1".to_owned(),
            state_digest: "state-v1".to_owned(),
            model_digest: "model-v1".to_owned(),
            abstraction_digest: "abstraction-v1".to_owned(),
            actuator_digest: "actuator-v1".to_owned(),
            domain_digest: "domain-v1".to_owned(),
            result_digest: "result-v1".to_owned(),
            valid_from_ns: 0,
            valid_until_ns: 1_000,
        }
    }

    fn admitted(fault: &str) -> FaultAdmission {
        FaultAdmission {
            fault_hypothesis_id: fault.to_owned(),
            status: AdmissionStatus::Admitted,
            result_digest: format!("result-{fault}"),
        }
    }

    fn base_episode() -> FaultEpisode {
        FaultEpisode {
            identity: identity(),
            now_ns: 500,
            actual_fault: ActualFault {
                present: true,
                hypothesis_id: Some("joint-locked".to_owned()),
                onset_ns: Some(100),
            },
            diagnosis: vec![
                DiagnosisSnapshot {
                    observed_at_ns: 100,
                    state: DiagnosisState::Suspected,
                    controller_plausible_fault_ids: vec![
                        "joint-locked".to_owned(),
                        "torque-derated".to_owned(),
                    ],
                },
                DiagnosisSnapshot {
                    observed_at_ns: 150,
                    state: DiagnosisState::Isolated {
                        hypothesis_id: "joint-locked".to_owned(),
                        confidence_ppm: 900_000,
                    },
                    controller_plausible_fault_ids: vec!["joint-locked".to_owned()],
                },
            ],
            action_decisions: vec![ActionDecision {
                observed_at_ns: 160,
                current_state_digest: "state-v1".to_owned(),
                action_id: "safe-hold".to_owned(),
                physical_fault_set_digest: "fault-set-v1".to_owned(),
                selected: true,
                physically_possible_fault_ids: vec![
                    "joint-locked".to_owned(),
                    "torque-derated".to_owned(),
                ],
                admissions: vec![admitted("joint-locked"), admitted("torque-derated")],
                receipt: receipt(),
                deadline_ns: 180,
                applied_at_ns: Some(170),
            }],
            mode_transitions: vec![
                ModeTransition {
                    at_ns: 100,
                    mode: OperatingMode::Suspected,
                    evidence_digest: "mode-1".to_owned(),
                },
                ModeTransition {
                    at_ns: 170,
                    mode: OperatingMode::SafeHold,
                    evidence_digest: "mode-2".to_owned(),
                },
            ],
            terminal_disposition: TerminalDisposition::SafeHold,
            safe_state_reached_ns: Some(200),
        }
    }

    #[test]
    fn common_action_admitted_across_physical_fault_set_is_qualified() {
        let report = audit_fault_episode(&base_episode()).expect("qualified episode");
        assert_eq!(report.detection_latency_ns, Some(0));
        assert_eq!(report.correct_isolation_latency_ns, Some(50));
        assert_eq!(report.safe_state_latency_ns, Some(100));
        assert!(report.operational_outcome_qualified);
        assert!(report.gate_violations.is_empty());
    }

    #[test]
    fn diagnosis_subset_cannot_hide_physical_fault_with_rejected_action() {
        let mut episode = base_episode();
        episode.action_decisions[0].selected = true;
        episode.action_decisions[0].admissions[0].status = AdmissionStatus::Admitted;
        episode.action_decisions[0].admissions[1].status = AdmissionStatus::Rejected;
        assert!(!audit_fault_episode(&episode)
            .expect("report")
            .operational_outcome_qualified);
    }

    #[test]
    fn high_confidence_wrong_diagnosis_does_not_override_physical_gate() {
        let mut episode = base_episode();
        episode.diagnosis[1].state = DiagnosisState::Isolated {
            hypothesis_id: "torque-derated".to_owned(),
            confidence_ppm: 990_000,
        };
        episode.diagnosis[1].controller_plausible_fault_ids = vec!["torque-derated".to_owned()];
        let report = audit_fault_episode(&episode).expect("report");
        assert_eq!(report.wrong_isolation_count, 1);
        assert!(report.operational_outcome_qualified);
        assert_eq!(report.correct_isolation_latency_ns, None);
    }

    #[test]
    fn physically_possible_zero_mass_fault_must_still_be_admitted() {
        let mut episode = base_episode();
        episode.action_decisions[0].physically_possible_fault_ids =
            vec!["joint-locked".to_owned(), "free-swing".to_owned()];
        episode.action_decisions[0].admissions = vec![
            admitted("joint-locked"),
            FaultAdmission {
                fault_hypothesis_id: "free-swing".to_owned(),
                status: AdmissionStatus::Rejected,
                result_digest: "result-free-swing".to_owned(),
            },
        ];
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report.gate_violations.iter().any(|violation| {
            matches!(
                violation,
                GateViolation::RejectedForPhysicalFault {
                    fault_hypothesis_id,
                    ..
                } if fault_hypothesis_id == "free-swing"
            )
        }));
        assert!(!report.operational_outcome_qualified);
    }

    #[test]
    fn missing_admission_for_physical_hypothesis_fails_closed() {
        let mut episode = base_episode();
        episode.action_decisions[0].admissions.pop();
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report.gate_violations.iter().any(|violation| {
            matches!(
                violation,
                GateViolation::MissingAdmissionForPhysicalFault {
                    fault_hypothesis_id,
                    ..
                } if fault_hypothesis_id == "torque-derated"
            )
        }));
    }

    #[test]
    fn admission_receipt_must_bind_current_state_not_only_episode_initial_state() {
        let mut episode = base_episode();
        episode.action_decisions[0].current_state_digest = "state-after-step".to_owned();
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report.gate_violations.contains(
            &GateViolation::InvalidOrStaleReceipt { at_ns: 160 }
        ));
        assert!(!report.operational_outcome_qualified);
    }

    #[test]
    fn receipt_cannot_be_replayed_for_a_different_action() {
        let mut episode = base_episode();
        episode.action_decisions[0].action_id = "walk".to_owned();
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report
            .gate_violations
            .contains(&GateViolation::InvalidOrStaleReceipt { at_ns: 160 }));
    }

    #[test]
    fn receipt_cannot_be_replayed_for_a_different_physical_fault_set() {
        let mut episode = base_episode();
        episode.action_decisions[0].physical_fault_set_digest = "fault-set-v2".to_owned();
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report
            .gate_violations
            .contains(&GateViolation::InvalidOrStaleReceipt { at_ns: 160 }));
    }

    #[test]
    fn stale_receipt_blocks_operational_qualification() {
        let mut episode = base_episode();
        episode.action_decisions[0].receipt.valid_until_ns = 159;
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report
            .gate_violations
            .contains(&GateViolation::InvalidOrStaleReceipt { at_ns: 160 }));
        assert!(!report.operational_outcome_qualified);
    }

    #[test]
    fn application_after_deadline_is_recorded_as_violation() {
        let mut episode = base_episode();
        episode.action_decisions[0].applied_at_ns = Some(181);
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report.gate_violations.iter().any(|violation| {
            matches!(
                violation,
                GateViolation::AppliedAfterDeadline {
                    deadline_ns: 180,
                    applied_at_ns: 181,
                    ..
                }
            )
        }));
    }

    #[test]
    fn unconfirmed_command_application_is_not_success() {
        let mut episode = base_episode();
        episode.action_decisions[0].applied_at_ns = None;
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report
            .gate_violations
            .contains(&GateViolation::UnconfirmedApplication { at_ns: 160 }));
        assert!(!report.operational_outcome_qualified);
    }

    #[test]
    fn physically_possible_set_omitting_ground_truth_is_a_violation() {
        let mut episode = base_episode();
        episode.action_decisions[0].physically_possible_fault_ids =
            vec!["torque-derated".to_owned()];
        episode.action_decisions[0].admissions = vec![admitted("torque-derated")];
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report.gate_violations.iter().any(|violation| {
            matches!(
                violation,
                GateViolation::PhysicalFaultSetOmitsGroundTruth {
                    fault_hypothesis_id,
                    ..
                } if fault_hypothesis_id == "joint-locked"
            )
        }));
    }

    #[test]
    fn false_alarm_on_healthy_episode_is_reported() {
        let mut episode = base_episode();
        episode.actual_fault = ActualFault {
            present: false,
            hypothesis_id: None,
            onset_ns: None,
        };
        episode.diagnosis[0].state = DiagnosisState::Suspected;
        let report = audit_fault_episode(&episode).expect("report");
        assert!(report.false_alarm);
        assert_eq!(report.detection_latency_ns, None);
    }

    #[test]
    fn safe_hold_requires_independent_physical_safe_state_time() {
        let mut episode = base_episode();
        episode.safe_state_reached_ns = None;
        assert_eq!(
            audit_fault_episode(&episode),
            Err(FaultEpisodeError::SafeDispositionMissingPhysicalSafeState)
        );
    }

    #[test]
    fn unknown_terminal_never_qualifies_as_recovery() {
        let mut episode = base_episode();
        episode.terminal_disposition = TerminalDisposition::Unknown;
        episode.safe_state_reached_ns = None;
        let report = audit_fault_episode(&episode).expect("unknown report");
        assert!(!report.operational_outcome_qualified);
        assert!(!report.safe_state_reached);
    }

    #[test]
    fn diagnosis_and_mode_timelines_must_be_monotonic() {
        let mut episode = base_episode();
        episode.diagnosis.reverse();
        assert_eq!(
            audit_fault_episode(&episode),
            Err(FaultEpisodeError::DiagnosisTimeOutOfOrder)
        );
    }

    #[test]
    fn wrong_fault_label_cannot_be_reported_as_correct_isolation() {
        let mut episode = base_episode();
        episode.diagnosis[1].state = DiagnosisState::Isolated {
            hypothesis_id: "free-swing".to_owned(),
            confidence_ppm: 950_000,
        };
        episode.diagnosis[1].controller_plausible_fault_ids = vec!["free-swing".to_owned()];
        let report = audit_fault_episode(&episode).expect("report");
        assert_eq!(report.correct_isolation_latency_ns, None);
        assert_eq!(report.wrong_isolation_count, 1);
        assert_eq!(report.controller_excluded_ground_truth_count, 1);
    }
}
