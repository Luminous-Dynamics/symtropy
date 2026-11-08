//! Finite probe-transition and robust-continuation composition oracle.
//!
//! This crate consumes an upstream finite physical transition abstraction and
//! viability/recovery state sets. It does not generate or prove their
//! completeness, authenticate receipts, or certify a physical robot.

use std::collections::BTreeSet;

pub const PROBABILITY_SCALE_PPB: u64 = 1_000_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViabilityReceipt {
    pub receipt_id: String,
    pub state_digest: String,
    pub abstraction_digest: String,
    pub assurance_receipt_id: String,
    /// Digests of the exact, separately qualified finite result sets.
    pub viability_result_digest: String,
    pub recovery_result_digest: String,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub horizon_steps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeExecutionReceipt {
    pub receipt_id: String,
    pub command_action_id: String,
    pub delivered_action_id: String,
    pub measured_actuation_digest: String,
    pub actuator_model_digest: String,
    /// Set by an upstream verifier; this crate checks the flag and identities.
    pub delivery_verified: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HorizonStateSet {
    pub remaining_steps: u32,
    pub state_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TimedState {
    pub state_id: String,
    /// Elapsed decision steps since probe start; must be in 1..duration.
    pub elapsed_steps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObservationKind {
    Known,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObservationOutcome {
    pub id: String,
    pub kind: ObservationKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeBranch {
    pub id: String,
    /// Calibrated branch probability. Zero is allowed for a physically
    /// possible branch; it never removes that branch from the robust gate.
    pub probability_ppb: u64,
    pub intermediate_states: Vec<TimedState>,
    pub terminal_state_id: String,
    /// Physical possibility relation, independent of probability mass.
    pub physically_possible_observation_ids: Vec<String>,
    /// Joint calibrated mass P(branch, outcome), aligned with the outcomes
    /// vector. The row must sum exactly to probability_ppb.
    pub joint_probability_ppb_by_outcome: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StateActionSuccessors {
    pub source_state_id: String,
    pub successor_state_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationAction {
    pub id: String,
    /// Unknown observations require at least one admitted fallback action.
    pub fallback_only: bool,
    pub successors_by_state: Vec<StateActionSuccessors>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeProblem {
    pub now_ns: u64,
    pub current_state_id: String,
    pub current_state_digest: String,
    pub abstraction_digest: String,
    pub actuator_model_digest: String,
    pub probe_action_id: String,
    pub horizon_steps: u32,
    pub probe_duration_steps: u32,
    pub viability_receipt: ViabilityReceipt,
    pub execution_receipt: ProbeExecutionReceipt,
    pub viable_states_by_remaining_horizon: Vec<HorizonStateSet>,
    /// Kept distinct from viability: these are qualified recovery/hold targets.
    pub recovery_states_by_remaining_horizon: Vec<HorizonStateSet>,
    pub outcomes: Vec<ObservationOutcome>,
    pub branches: Vec<ProbeBranch>,
    pub continuation_actions: Vec<ContinuationAction>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeViolation {
    CurrentStateOutsideViableOrRecoverySet { state_id: String, remaining_steps: u32 },
    IntermediateStateOutsideViableOrRecoverySet {
        branch_id: String,
        state_id: String,
        remaining_steps: u32,
    },
    TerminalStateOutsideViableOrRecoverySet {
        branch_id: String,
        state_id: String,
        remaining_steps: u32,
    },
    NoRobustContinuation { outcome_id: String },
    MissingUnknownFallback { outcome_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeContinuation {
    pub outcome_id: String,
    pub is_unknown: bool,
    /// Sum of calibrated joint masses for this observation, in ppb.
    pub probability_ppb: u64,
    /// Includes every physically possible branch, even at zero probability.
    pub possible_branch_ids: Vec<String>,
    pub possible_terminal_state_ids: Vec<String>,
    pub possible_terminal_viable_state_ids: Vec<String>,
    pub possible_terminal_recovery_state_ids: Vec<String>,
    /// Safe from every state and all declared successors.
    pub robust_admissible_action_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAssessment {
    pub probe_action_id: String,
    pub admitted: bool,
    pub branch_probability_total_ppb: u64,
    pub joint_probability_total_ppb: u64,
    pub violations: Vec<ProbeViolation>,
    pub outcome_continuations: Vec<OutcomeContinuation>,
    pub excluded_action_ids: Vec<String>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProbeContinuationError {
    EmptyIdentifier,
    InvalidHorizon,
    InvalidReceiptInterval,
    StaleReceipt,
    ReceiptContextMismatch,
    ProbeDeliveryNotVerified,
    NoViabilitySets,
    DuplicateHorizonSet,
    DuplicateViableState,
    DuplicateRecoveryState,
    StateInBothViableAndRecoverySet,
    MissingViabilityHorizon,
    NoOutcomes,
    DuplicateOutcomeId,
    NoBranches,
    DuplicateBranchId,
    InvalidBranchProbability,
    BranchProbabilityNotNormalized,
    JointProbabilityCardinalityMismatch,
    InvalidJointProbability,
    JointProbabilityNotNormalized,
    UnknownOutcomeNotPhysicallyRepresented,
    UnknownObservationId,
    EmptyPhysicalObservationSet,
    InvalidIntermediateStateTime,
    DuplicateActionId,
    DuplicateActionSourceState,
    EmptyActionSuccessors,
    DuplicateActionSuccessor,
    ArithmeticOverflow,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn add_u64(left: u64, right: u64) -> Result<u64, ProbeContinuationError> {
    left.checked_add(right)
        .ok_or(ProbeContinuationError::ArithmeticOverflow)
}

fn contains_viable_state(
    sets: &[HorizonStateSet],
    remaining_steps: u32,
    state_id: &str,
) -> Result<bool, ProbeContinuationError> {
    let set = sets
        .iter()
        .find(|set| set.remaining_steps == remaining_steps)
        .ok_or(ProbeContinuationError::MissingViabilityHorizon)?;
    Ok(set.state_ids.iter().any(|candidate| candidate.as_str() == state_id))
}

fn contains_recovery_state(
    sets: &[HorizonStateSet],
    remaining_steps: u32,
    state_id: &str,
) -> bool {
    sets.iter()
        .find(|set| set.remaining_steps == remaining_steps)
        .is_some_and(|set| set.state_ids.iter().any(|candidate| candidate == state_id))
}

fn contains_viable_or_recovery_state(
    viable_sets: &[HorizonStateSet],
    recovery_sets: &[HorizonStateSet],
    remaining_steps: u32,
    state_id: &str,
) -> Result<bool, ProbeContinuationError> {
    Ok(contains_viable_state(viable_sets, remaining_steps, state_id)?
        || contains_recovery_state(recovery_sets, remaining_steps, state_id))
}

fn has_viability_horizon(
    sets: &[HorizonStateSet],
    remaining_steps: u32,
) -> bool {
    sets.iter().any(|set| set.remaining_steps == remaining_steps)
}

fn validate_problem(problem: &ProbeProblem) -> Result<(), ProbeContinuationError> {
    if !nonempty(&problem.current_state_id)
        || !nonempty(&problem.current_state_digest)
        || !nonempty(&problem.abstraction_digest)
        || !nonempty(&problem.actuator_model_digest)
        || !nonempty(&problem.probe_action_id)
    {
        return Err(ProbeContinuationError::EmptyIdentifier);
    }
    if problem.horizon_steps == 0
        || problem.probe_duration_steps == 0
        || problem.probe_duration_steps >= problem.horizon_steps
    {
        return Err(ProbeContinuationError::InvalidHorizon);
    }

    let receipt = &problem.viability_receipt;
    let execution = &problem.execution_receipt;
    if !nonempty(&receipt.receipt_id)
        || !nonempty(&receipt.state_digest)
        || !nonempty(&receipt.abstraction_digest)
        || !nonempty(&receipt.assurance_receipt_id)
        || !nonempty(&receipt.viability_result_digest)
        || !nonempty(&receipt.recovery_result_digest)
        || !nonempty(&execution.receipt_id)
        || !nonempty(&execution.command_action_id)
        || !nonempty(&execution.delivered_action_id)
        || !nonempty(&execution.measured_actuation_digest)
        || !nonempty(&execution.actuator_model_digest)
    {
        return Err(ProbeContinuationError::EmptyIdentifier);
    }
    if receipt.valid_from_ns > receipt.valid_until_ns {
        return Err(ProbeContinuationError::InvalidReceiptInterval);
    }
    if problem.now_ns < receipt.valid_from_ns || problem.now_ns > receipt.valid_until_ns {
        return Err(ProbeContinuationError::StaleReceipt);
    }
    if receipt.state_digest != problem.current_state_digest
        || receipt.abstraction_digest != problem.abstraction_digest
        || receipt.horizon_steps != problem.horizon_steps
        || execution.command_action_id != problem.probe_action_id
        || execution.actuator_model_digest != problem.actuator_model_digest
    {
        return Err(ProbeContinuationError::ReceiptContextMismatch);
    }
    if !execution.delivery_verified {
        return Err(ProbeContinuationError::ProbeDeliveryNotVerified);
    }
    if problem.viable_states_by_remaining_horizon.is_empty() {
        return Err(ProbeContinuationError::NoViabilitySets);
    }

    let mut horizons = BTreeSet::new();
    for set in &problem.viable_states_by_remaining_horizon {
        if !horizons.insert(set.remaining_steps) {
            return Err(ProbeContinuationError::DuplicateHorizonSet);
        }
        let mut states = BTreeSet::new();
        for state in &set.state_ids {
            if !nonempty(state) {
                return Err(ProbeContinuationError::EmptyIdentifier);
            }
            if !states.insert(state) {
                return Err(ProbeContinuationError::DuplicateViableState);
            }
        }
    }

    let mut recovery_horizons = BTreeSet::new();
    for set in &problem.recovery_states_by_remaining_horizon {
        if !recovery_horizons.insert(set.remaining_steps) {
            return Err(ProbeContinuationError::DuplicateHorizonSet);
        }
        let mut recovery_states = BTreeSet::new();
        for state in &set.state_ids {
            if !nonempty(state) {
                return Err(ProbeContinuationError::EmptyIdentifier);
            }
            if !recovery_states.insert(state) {
                return Err(ProbeContinuationError::DuplicateRecoveryState);
            }
        }
        if let Some(viable) = problem.viable_states_by_remaining_horizon
            .iter()
            .find(|viable| viable.remaining_steps == set.remaining_steps)
        {
            if set.state_ids.iter().any(|state| viable.state_ids.contains(state)) {
                return Err(ProbeContinuationError::StateInBothViableAndRecoverySet);
            }
        }
    }

    if problem.outcomes.is_empty() {
        return Err(ProbeContinuationError::NoOutcomes);
    }
    let mut outcome_ids = BTreeSet::new();
    for outcome in &problem.outcomes {
        if !nonempty(&outcome.id) {
            return Err(ProbeContinuationError::EmptyIdentifier);
        }
        if !outcome_ids.insert(outcome.id.as_str()) {
            return Err(ProbeContinuationError::DuplicateOutcomeId);
        }
    }

    if problem.branches.is_empty() {
        return Err(ProbeContinuationError::NoBranches);
    }
    let mut branch_ids = BTreeSet::new();
    let mut branch_probability_total = 0_u64;
    let mut joint_probability_total = 0_u64;
    let outcome_id_set: BTreeSet<&str> =
        problem.outcomes.iter().map(|item| item.id.as_str()).collect();

    for branch in &problem.branches {
        if !nonempty(&branch.id) || !nonempty(&branch.terminal_state_id) {
            return Err(ProbeContinuationError::EmptyIdentifier);
        }
        if !branch_ids.insert(branch.id.as_str()) {
            return Err(ProbeContinuationError::DuplicateBranchId);
        }
        if branch.probability_ppb > PROBABILITY_SCALE_PPB {
            return Err(ProbeContinuationError::InvalidBranchProbability);
        }
        branch_probability_total = add_u64(branch_probability_total, branch.probability_ppb)?;

        if branch.physically_possible_observation_ids.is_empty() {
            return Err(ProbeContinuationError::EmptyPhysicalObservationSet);
        }
        let mut physical_outcomes = BTreeSet::new();
        for outcome_id in &branch.physically_possible_observation_ids {
            if !outcome_id_set.contains(outcome_id.as_str()) {
                return Err(ProbeContinuationError::UnknownObservationId);
            }
            if !physical_outcomes.insert(outcome_id.as_str()) {
                return Err(ProbeContinuationError::UnknownObservationId);
            }
        }

        if branch.joint_probability_ppb_by_outcome.len() != problem.outcomes.len() {
            return Err(ProbeContinuationError::JointProbabilityCardinalityMismatch);
        }
        let mut branch_joint_total = 0_u64;
        for (index, mass) in branch.joint_probability_ppb_by_outcome.iter().enumerate() {
            if *mass > PROBABILITY_SCALE_PPB {
                return Err(ProbeContinuationError::InvalidJointProbability);
            }
            if *mass > 0
                && !physical_outcomes.contains(problem.outcomes[index].id.as_str())
            {
                return Err(ProbeContinuationError::UnknownOutcomeNotPhysicallyRepresented);
            }
            branch_joint_total = add_u64(branch_joint_total, *mass)?;
        }
        if branch_joint_total != branch.probability_ppb {
            return Err(ProbeContinuationError::JointProbabilityNotNormalized);
        }
        joint_probability_total = add_u64(joint_probability_total, branch_joint_total)?;

        let expected_intermediate_count =
            usize::try_from(problem.probe_duration_steps - 1)
                .map_err(|_| ProbeContinuationError::InvalidIntermediateStateTime)?;
        if branch.intermediate_states.len() != expected_intermediate_count {
            return Err(ProbeContinuationError::InvalidIntermediateStateTime);
        }
        for (index, state) in branch.intermediate_states.iter().enumerate() {
            let expected_elapsed = u32::try_from(index + 1)
                .map_err(|_| ProbeContinuationError::InvalidIntermediateStateTime)?;
            if !nonempty(&state.state_id) || state.elapsed_steps != expected_elapsed {
                return Err(ProbeContinuationError::InvalidIntermediateStateTime);
            }
        }
    }

    if branch_probability_total != PROBABILITY_SCALE_PPB {
        return Err(ProbeContinuationError::BranchProbabilityNotNormalized);
    }
    if joint_probability_total != PROBABILITY_SCALE_PPB {
        return Err(ProbeContinuationError::JointProbabilityNotNormalized);
    }

    // Every declared outcome must be physically possible on at least one
    // branch, including an outcome that has zero calibrated probability.
    for outcome in &problem.outcomes {
        if !problem.branches.iter().any(|branch| {
            branch
                .physically_possible_observation_ids
                .iter()
                .any(|id| id.as_str() == outcome.id.as_str())
        }) {
            return Err(ProbeContinuationError::UnknownOutcomeNotPhysicallyRepresented);
        }
    }

    let mut action_ids = BTreeSet::new();
    for action in &problem.continuation_actions {
        if !nonempty(&action.id) {
            return Err(ProbeContinuationError::EmptyIdentifier);
        }
        if !action_ids.insert(action.id.as_str()) {
            return Err(ProbeContinuationError::DuplicateActionId);
        }
        let mut source_states = BTreeSet::new();
        for row in &action.successors_by_state {
            if !nonempty(&row.source_state_id) {
                return Err(ProbeContinuationError::EmptyIdentifier);
            }
            if !source_states.insert(row.source_state_id.as_str()) {
                return Err(ProbeContinuationError::DuplicateActionSourceState);
            }
            if row.successor_state_ids.is_empty() {
                return Err(ProbeContinuationError::EmptyActionSuccessors);
            }
            let mut successors = BTreeSet::new();
            for successor in &row.successor_state_ids {
                if !nonempty(successor) {
                    return Err(ProbeContinuationError::EmptyIdentifier);
                }
                if !successors.insert(successor.as_str()) {
                    return Err(ProbeContinuationError::DuplicateActionSuccessor);
                }
            }
        }
    }

    // Require all horizon-indexed sets inspected by this evaluator.
    let terminal_remaining = problem.horizon_steps - problem.probe_duration_steps;
    let action_remaining = terminal_remaining - 1;
    if !has_viability_horizon(&problem.viable_states_by_remaining_horizon, problem.horizon_steps)
        || !has_viability_horizon(&problem.viable_states_by_remaining_horizon, terminal_remaining)
        || !has_viability_horizon(&problem.viable_states_by_remaining_horizon, action_remaining)
    {
        return Err(ProbeContinuationError::MissingViabilityHorizon);
    }
    for branch in &problem.branches {
        for state in &branch.intermediate_states {
            let remaining = problem.horizon_steps - state.elapsed_steps;
            if !has_viability_horizon(&problem.viable_states_by_remaining_horizon, remaining) {
                return Err(ProbeContinuationError::MissingViabilityHorizon);
            }
        }
    }

    Ok(())
}

/// Evaluate a probe's complete finite branch prefixes and the robustly
/// admissible continuation set for each physically possible observation.
///
/// Probabilities are reported separately. Zero-probability physical branches
/// remain in the possibility and viability checks.
pub fn evaluate_probe_continuation(
    problem: &ProbeProblem,
) -> Result<ProbeAssessment, ProbeContinuationError> {
    validate_problem(problem)?;

    let mut violations = Vec::new();
    if !contains_viable_or_recovery_state(
        &problem.viable_states_by_remaining_horizon,
        &problem.recovery_states_by_remaining_horizon,
        problem.horizon_steps,
        &problem.current_state_id,
    )? {
        violations.push(ProbeViolation::CurrentStateOutsideViableOrRecoverySet {
            state_id: problem.current_state_id.clone(),
            remaining_steps: problem.horizon_steps,
        });
    }

    let terminal_remaining = problem.horizon_steps - problem.probe_duration_steps;
    let action_remaining = terminal_remaining - 1;
    for branch in &problem.branches {
        for state in &branch.intermediate_states {
            let remaining = problem.horizon_steps - state.elapsed_steps;
            if !contains_viable_or_recovery_state(
                &problem.viable_states_by_remaining_horizon,
                &problem.recovery_states_by_remaining_horizon,
                remaining,
                &state.state_id,
            )? {
                violations.push(ProbeViolation::IntermediateStateOutsideViableOrRecoverySet {
                    branch_id: branch.id.clone(),
                    state_id: state.state_id.clone(),
                    remaining_steps: remaining,
                });
            }
        }
        if !contains_viable_or_recovery_state(
            &problem.viable_states_by_remaining_horizon,
            &problem.recovery_states_by_remaining_horizon,
            terminal_remaining,
            &branch.terminal_state_id,
        )? {
            violations.push(ProbeViolation::TerminalStateOutsideViableOrRecoverySet {
                branch_id: branch.id.clone(),
                state_id: branch.terminal_state_id.clone(),
                remaining_steps: terminal_remaining,
            });
        }
    }

    let mut outcome_continuations = Vec::with_capacity(problem.outcomes.len());
    let mut excluded_action_ids = BTreeSet::new();
    for (outcome_index, outcome) in problem.outcomes.iter().enumerate() {
        let mut possible_branch_ids = BTreeSet::new();
        let mut possible_terminal_state_ids = BTreeSet::new();
        let mut possible_terminal_viable_state_ids = BTreeSet::new();
        let mut possible_terminal_recovery_state_ids = BTreeSet::new();
        let mut probability_ppb = 0_u64;

        for branch in &problem.branches {
            if branch
                .physically_possible_observation_ids
                .iter()
                .any(|id| id == &outcome.id)
            {
                possible_branch_ids.insert(branch.id.clone());
                possible_terminal_state_ids.insert(branch.terminal_state_id.clone());
                if contains_viable_state(
                    &problem.viable_states_by_remaining_horizon,
                    terminal_remaining,
                    &branch.terminal_state_id,
                )? {
                    possible_terminal_viable_state_ids.insert(branch.terminal_state_id.clone());
                }
                if contains_recovery_state(
                    &problem.recovery_states_by_remaining_horizon,
                    terminal_remaining,
                    &branch.terminal_state_id,
                ) {
                    possible_terminal_recovery_state_ids.insert(branch.terminal_state_id.clone());
                }
            }
            probability_ppb = add_u64(
                probability_ppb,
                branch.joint_probability_ppb_by_outcome[outcome_index],
            )?;
        }

        let mut robust_admissible_action_ids = Vec::new();
        for action in &problem.continuation_actions {
            if matches!(&outcome.kind, ObservationKind::Unknown) && !action.fallback_only {
                excluded_action_ids.insert(action.id.clone());
                continue;
            }

            let mut action_admissible = true;
            for state_id in &possible_terminal_state_ids {
                let row = action
                    .successors_by_state
                    .iter()
                    .find(|row| row.source_state_id.as_str() == state_id.as_str());
                let Some(row) = row else {
                    action_admissible = false;
                    break;
                };
                let mut all_successors_viable = true;
                for successor in &row.successor_state_ids {
                    if !contains_viable_or_recovery_state(
                        &problem.viable_states_by_remaining_horizon,
                        &problem.recovery_states_by_remaining_horizon,
                        action_remaining,
                        successor,
                    )? {
                        all_successors_viable = false;
                        break;
                    }
                }
                if !all_successors_viable {
                    action_admissible = false;
                    break;
                }
            }

            if action_admissible {
                robust_admissible_action_ids.push(action.id.clone());
            } else {
                excluded_action_ids.insert(action.id.clone());
            }
        }
        robust_admissible_action_ids.sort();

        if robust_admissible_action_ids.is_empty() {
            violations.push(ProbeViolation::NoRobustContinuation {
                outcome_id: outcome.id.clone(),
            });
        }
        if matches!(&outcome.kind, ObservationKind::Unknown)
            && !problem.continuation_actions.iter().any(|action| {
                action.fallback_only
                    && robust_admissible_action_ids
                        .iter()
                        .any(|admitted| admitted == &action.id)
            })
        {
            violations.push(ProbeViolation::MissingUnknownFallback {
                outcome_id: outcome.id.clone(),
            });
        }

        outcome_continuations.push(OutcomeContinuation {
            outcome_id: outcome.id.clone(),
            is_unknown: matches!(&outcome.kind, ObservationKind::Unknown),
            probability_ppb,
            possible_branch_ids: possible_branch_ids.into_iter().collect(),
            possible_terminal_state_ids: possible_terminal_state_ids.into_iter().collect(),
            possible_terminal_viable_state_ids: possible_terminal_viable_state_ids.into_iter().collect(),
            possible_terminal_recovery_state_ids: possible_terminal_recovery_state_ids.into_iter().collect(),
            robust_admissible_action_ids,
        });
    }

    outcome_continuations.sort_by(|left, right| left.outcome_id.cmp(&right.outcome_id));
    violations.sort_by_key(|violation| format!("{violation:?}"));

    let branch_probability_total_ppb = problem
        .branches
        .iter()
        .try_fold(0_u64, |sum, branch| add_u64(sum, branch.probability_ppb))?;
    let joint_probability_total_ppb = problem
        .branches
        .iter()
        .flat_map(|branch| branch.joint_probability_ppb_by_outcome.iter())
        .try_fold(0_u64, |sum, mass| add_u64(sum, *mass))?;

    Ok(ProbeAssessment {
        probe_action_id: problem.probe_action_id.clone(),
        admitted: violations.is_empty(),
        branch_probability_total_ppb,
        joint_probability_total_ppb,
        violations,
        outcome_continuations,
        excluded_action_ids: excluded_action_ids.into_iter().collect(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt() -> ViabilityReceipt {
        ViabilityReceipt {
            receipt_id: "viability-receipt".to_owned(),
            state_digest: "state-digest".to_owned(),
            abstraction_digest: "abstraction-digest".to_owned(),
            assurance_receipt_id: "assurance-receipt".to_owned(),
            viability_result_digest: "viability-result".to_owned(),
            recovery_result_digest: "recovery-result".to_owned(),
            valid_from_ns: 100,
            valid_until_ns: 1_000,
            horizon_steps: 3,
        }
    }

    fn execution() -> ProbeExecutionReceipt {
        ProbeExecutionReceipt {
            receipt_id: "probe-execution".to_owned(),
            command_action_id: "probe-foot".to_owned(),
            delivered_action_id: "delivered-probe-foot".to_owned(),
            measured_actuation_digest: "actuation-evidence".to_owned(),
            actuator_model_digest: "actuator-model".to_owned(),
            delivery_verified: true,
        }
    }

    fn horizon(remaining_steps: u32, states: &[&str]) -> HorizonStateSet {
        HorizonStateSet {
            remaining_steps,
            state_ids: states.iter().map(|state| (*state).to_owned()).collect(),
        }
    }

    fn row(source: &str, successors: &[&str]) -> StateActionSuccessors {
        StateActionSuccessors {
            source_state_id: source.to_owned(),
            successor_state_ids: successors.iter().map(|state| (*state).to_owned()).collect(),
        }
    }

    fn safe_problem() -> ProbeProblem {
        ProbeProblem {
            now_ns: 500,
            current_state_id: "s0".to_owned(),
            current_state_digest: "state-digest".to_owned(),
            abstraction_digest: "abstraction-digest".to_owned(),
            actuator_model_digest: "actuator-model".to_owned(),
            probe_action_id: "probe-foot".to_owned(),
            horizon_steps: 3,
            probe_duration_steps: 1,
            viability_receipt: receipt(),
            execution_receipt: execution(),
            viable_states_by_remaining_horizon: vec![
                horizon(3, &["s0"]),
                horizon(2, &["grip-good", "grip-degraded"]),
                horizon(1, &["advance-good", "retreat-good", "retreat-degraded"]),
            ],
            recovery_states_by_remaining_horizon: vec![],
            outcomes: vec![
                ObservationOutcome { id: "grip-confirmed".to_owned(), kind: ObservationKind::Known },
                ObservationOutcome { id: "grip-degraded".to_owned(), kind: ObservationKind::Known },
            ],
            branches: vec![
                ProbeBranch {
                    id: "b-good".to_owned(),
                    probability_ppb: 600_000_000,
                    intermediate_states: vec![],
                    terminal_state_id: "grip-good".to_owned(),
                    physically_possible_observation_ids: vec!["grip-confirmed".to_owned()],
                    joint_probability_ppb_by_outcome: vec![600_000_000, 0],
                },
                ProbeBranch {
                    id: "b-degraded".to_owned(),
                    probability_ppb: 400_000_000,
                    intermediate_states: vec![],
                    terminal_state_id: "grip-degraded".to_owned(),
                    physically_possible_observation_ids: vec!["grip-degraded".to_owned()],
                    joint_probability_ppb_by_outcome: vec![0, 400_000_000],
                },
            ],
            continuation_actions: vec![
                ContinuationAction {
                    id: "advance".to_owned(),
                    fallback_only: false,
                    successors_by_state: vec![
                        row("grip-good", &["advance-good"]),
                        row("grip-degraded", &["dead-end"]),
                    ],
                },
                ContinuationAction {
                    id: "retreat".to_owned(),
                    fallback_only: true,
                    successors_by_state: vec![
                        row("grip-good", &["retreat-good"]),
                        row("grip-degraded", &["retreat-degraded"]),
                    ],
                },
            ],
        }
    }

    #[test]
    fn safe_probe_reports_branch_specific_robust_continuations() {
        let report = evaluate_probe_continuation(&safe_problem()).expect("valid fixture");
        assert!(report.admitted);
        assert_eq!(report.branch_probability_total_ppb, PROBABILITY_SCALE_PPB);
        assert_eq!(report.joint_probability_total_ppb, PROBABILITY_SCALE_PPB);
        assert_eq!(
            report.outcome_continuations.iter()
                .find(|item| item.outcome_id == "grip-confirmed").unwrap()
                .robust_admissible_action_ids,
            vec!["advance".to_owned(), "retreat".to_owned()]
        );
        assert_eq!(
            report.outcome_continuations.iter()
                .find(|item| item.outcome_id == "grip-degraded").unwrap()
                .robust_admissible_action_ids,
            vec!["retreat".to_owned()]
        );
    }

    #[test]
    fn zero_probability_physical_branch_still_blocks_probe() {
        let mut problem = safe_problem();
        problem.branches.push(ProbeBranch {
            id: "b-zero-probability-trap".to_owned(),
            probability_ppb: 0,
            intermediate_states: vec![],
            terminal_state_id: "dead-end".to_owned(),
            physically_possible_observation_ids: vec!["grip-degraded".to_owned()],
            joint_probability_ppb_by_outcome: vec![0, 0],
        });
        let report = evaluate_probe_continuation(&problem).expect("valid probabilities");
        assert!(!report.admitted);
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            ProbeViolation::TerminalStateOutsideViableOrRecoverySet { branch_id, .. }
                if branch_id == "b-zero-probability-trap"
        )));
    }

    #[test]
    fn observation_alias_requires_intersection_across_all_terminal_states() {
        let mut problem = safe_problem();
        problem.outcomes = vec![ObservationOutcome {
            id: "ambiguous".to_owned(),
            kind: ObservationKind::Known,
        }];
        problem.viable_states_by_remaining_horizon[2].state_ids = vec![
            "advance-good".to_owned(),
            "retreat-good".to_owned(),
            "retreat-degraded".to_owned(),
        ];
        problem.branches[0].physically_possible_observation_ids = vec!["ambiguous".to_owned()];
        problem.branches[0].joint_probability_ppb_by_outcome = vec![600_000_000];
        problem.branches[1].physically_possible_observation_ids = vec!["ambiguous".to_owned()];
        problem.branches[1].joint_probability_ppb_by_outcome = vec![400_000_000];

        let report = evaluate_probe_continuation(&problem).expect("valid aliased observation");
        assert!(report.admitted);
        assert_eq!(
            report.outcome_continuations[0].possible_terminal_state_ids,
            vec!["grip-degraded".to_owned(), "grip-good".to_owned()]
        );
        assert_eq!(
            report.outcome_continuations[0].robust_admissible_action_ids,
            vec!["retreat".to_owned()]
        );
    }

    #[test]
    fn intermediate_latency_state_must_remain_viable() {
        let mut problem = safe_problem();
        problem.probe_duration_steps = 2;
        problem.viable_states_by_remaining_horizon = vec![
            horizon(3, &["s0"]),
            horizon(2, &[]),
            horizon(1, &[
                "grip-good", "grip-degraded", "advance-good",
                "retreat-good", "retreat-degraded"
            ]),
        ];
        for branch in &mut problem.branches {
            branch.intermediate_states = vec![TimedState {
                state_id: "latency-prefix".to_owned(),
                elapsed_steps: 1,
            }];
        }

        let report = evaluate_probe_continuation(&problem).expect("valid branch timing");
        assert!(!report.admitted);
        assert_eq!(
            report.violations.iter().filter(|violation| matches!(
                violation,
                ProbeViolation::IntermediateStateOutsideViableOrRecoverySet { .. }
            )).count(),
            2
        );
    }

    #[test]
    fn unknown_outcome_requires_an_admitted_fallback() {
        let mut problem = safe_problem();
        problem.outcomes.push(ObservationOutcome {
            id: "unknown".to_owned(),
            kind: ObservationKind::Unknown,
        });
        for branch in &mut problem.branches {
            branch.physically_possible_observation_ids.push("unknown".to_owned());
            branch.joint_probability_ppb_by_outcome.push(0);
        }

        let report = evaluate_probe_continuation(&problem).expect("unknown outcome fixture");
        assert!(report.admitted);
        let unknown = report.outcome_continuations.iter()
            .find(|item| item.outcome_id == "unknown").unwrap();
        assert!(unknown.is_unknown);
        assert_eq!(unknown.probability_ppb, 0);
        assert_eq!(unknown.robust_admissible_action_ids, vec!["retreat".to_owned()]);

        let mut no_fallback = problem;
        no_fallback.continuation_actions.retain(|action| !action.fallback_only);
        let report = evaluate_probe_continuation(&no_fallback).expect("well-formed no fallback");
        assert!(!report.admitted);
        assert!(report.violations.iter().any(|violation| matches!(
            violation,
            ProbeViolation::MissingUnknownFallback { outcome_id } if outcome_id == "unknown"
        )));
    }

    #[test]
    fn recovery_target_remains_distinct_from_walkable_viability() {
        let mut problem = safe_problem();
        problem.viable_states_by_remaining_horizon[1]
            .state_ids
            .retain(|state| state != "grip-degraded");
        problem.viable_states_by_remaining_horizon[2]
            .state_ids
            .retain(|state| state != "retreat-degraded");
        problem.recovery_states_by_remaining_horizon = vec![
            horizon(2, &["grip-degraded"]),
            horizon(1, &["retreat-degraded"]),
        ];

        let report = evaluate_probe_continuation(&problem).expect("separate recovery fixture");
        assert!(report.admitted);
        let degraded = report
            .outcome_continuations
            .iter()
            .find(|item| item.outcome_id == "grip-degraded")
            .expect("degraded observation");
        assert!(degraded.possible_terminal_viable_state_ids.is_empty());
        assert_eq!(
            degraded.possible_terminal_recovery_state_ids,
            vec!["grip-degraded".to_owned()]
        );
        assert_eq!(
            degraded.robust_admissible_action_ids,
            vec!["retreat".to_owned()]
        );
    }

    #[test]
    fn state_cannot_be_classified_as_both_viable_and_recovery() {
        let mut problem = safe_problem();
        problem.recovery_states_by_remaining_horizon =
            vec![horizon(2, &["grip-good"])];
        assert_eq!(
            evaluate_probe_continuation(&problem),
            Err(ProbeContinuationError::StateInBothViableAndRecoverySet)
        );
    }

    #[test]
    fn mismatched_or_expired_viability_receipt_is_rejected() {
        let mut mismatch = safe_problem();
        mismatch.viability_receipt.abstraction_digest = "other-abstraction".to_owned();
        assert_eq!(
            evaluate_probe_continuation(&mismatch),
            Err(ProbeContinuationError::ReceiptContextMismatch)
        );

        let mut expired = safe_problem();
        expired.viability_receipt.valid_until_ns = 499;
        assert_eq!(
            evaluate_probe_continuation(&expired),
            Err(ProbeContinuationError::StaleReceipt)
        );
    }

    #[test]
    fn adding_a_physical_branch_cannot_expand_robust_actions() {
        let base = evaluate_probe_continuation(&safe_problem()).expect("base result");
        let mut expanded = safe_problem();
        expanded.viable_states_by_remaining_horizon[1]
            .state_ids.push("retreat-weak".to_owned());
        expanded.viable_states_by_remaining_horizon[2]
            .state_ids.push("grip-weak".to_owned());
        expanded.branches.push(ProbeBranch {
            id: "b-extra-zero-mass".to_owned(),
            probability_ppb: 0,
            intermediate_states: vec![],
            terminal_state_id: "grip-weak".to_owned(),
            physically_possible_observation_ids: vec!["grip-confirmed".to_owned()],
            joint_probability_ppb_by_outcome: vec![0, 0],
        });
        expanded.continuation_actions[0].successors_by_state.push(
            row("grip-weak", &["dead-end"])
        );
        expanded.continuation_actions[1].successors_by_state.push(
            row("grip-weak", &["retreat-weak"])
        );

        let after = evaluate_probe_continuation(&expanded).expect("expanded result");
        assert!(after.admitted);
        let before_good = base.outcome_continuations.iter()
            .find(|item| item.outcome_id == "grip-confirmed").unwrap();
        let after_good = after.outcome_continuations.iter()
            .find(|item| item.outcome_id == "grip-confirmed").unwrap();
        assert_eq!(before_good.robust_admissible_action_ids, vec!["advance".to_owned(), "retreat".to_owned()]);
        assert_eq!(after_good.robust_admissible_action_ids, vec!["retreat".to_owned()]);
    }

    #[test]
    fn input_order_does_not_change_canonical_continuation_results() {
        let original = evaluate_probe_continuation(&safe_problem()).expect("original");
        let mut permuted = safe_problem();
        permuted.branches.reverse();
        // Reverse the outcomes and the aligned per-branch joint-mass columns.
        permuted.outcomes.reverse();
        for branch in &mut permuted.branches {
            branch.joint_probability_ppb_by_outcome.reverse();
        }
        permuted.continuation_actions.reverse();
        let reordered = evaluate_probe_continuation(&permuted).expect("permuted");
        assert_eq!(original, reordered);
    }

    #[test]
    fn missing_action_transition_is_not_assumed_safe() {
        let mut problem = safe_problem();
        problem.continuation_actions[0]
            .successors_by_state
            .retain(|row| row.source_state_id != "grip-degraded");
        let report = evaluate_probe_continuation(&problem).expect("partial action map");
        assert!(report.admitted);
        assert_eq!(
            report.outcome_continuations.iter()
                .find(|item| item.outcome_id == "grip-degraded").unwrap()
                .robust_admissible_action_ids,
            vec!["retreat".to_owned()]
        );
    }
}
