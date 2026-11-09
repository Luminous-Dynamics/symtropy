//! Finite viability-conditioned value-of-information composition oracle.
//!
//! Consumes upstream physical probe admission and outcome-specific robust
//! continuation evidence. It does not compute or authenticate those upstream
//! safety/viability results.

use std::collections::BTreeSet;

pub const SCALE_PPB: u64 = 1_000_000_000;
const SCALE: u128 = SCALE_PPB as u128;
pub const MAX_JOINT_CELLS: usize = 1_000_000;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct EvidenceReceipt {
    pub receipt_id: String,
    pub subject_id: String,
    pub initial_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub result_digest: String,
    pub valid_from_ns: u64,
    pub valid_until_ns: u64,
    pub horizon_steps: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ProbeGateStatus {
    Admitted,
    Rejected { witness_id: String },
    Unknown { reason: String },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PhysicalBranchOutcome {
    pub branch_id: String,
    pub outcome_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PossibleHypothesisBranch {
    pub hypothesis_id: String,
    pub branch_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProbeAdmission {
    pub receipt: EvidenceReceipt,
    pub status: ProbeGateStatus,
    pub probe_action_id: String,
    pub probe_duration_steps: u32,
    pub branch_ids: Vec<String>,
    /// Physical possibility is independent of joint probability mass.
    pub physically_possible_branch_outcomes: Vec<PhysicalBranchOutcome>,
    pub possible_hypothesis_branches: Vec<PossibleHypothesisBranch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeKind {
    Known,
    Unknown,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Outcome {
    pub id: String,
    pub kind: OutcomeKind,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Hypothesis {
    pub id: String,
    pub passive_prior_mass_ppb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Branch {
    pub id: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JointMassCell {
    pub hypothesis_id: String,
    pub branch_id: String,
    pub outcome_id: String,
    pub mass_ppb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ContinuationEvidence {
    pub receipt: EvidenceReceipt,
    pub outcome_id: String,
    pub post_state_set_digest: String,
    /// Returned by the upstream probe-continuation/viability authorities.
    pub robust_admitted_action_ids: Vec<String>,
    pub explicit_fallback_action_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionLossCell {
    pub action_id: String,
    pub hypothesis_id: String,
    pub branch_id: String,
    pub outcome_id: String,
    pub loss_ppb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PassiveLossCell {
    pub action_id: String,
    pub hypothesis_id: String,
    pub loss_ppb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceUsage {
    pub energy_mj: u64,
    pub elapsed_ms: u64,
    pub actuator_peak_ppm: u64,
    pub risk_alpha_ppb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceLimits {
    pub energy_mj: u64,
    pub elapsed_ms: u64,
    pub actuator_peak_ppm: u64,
    pub risk_alpha_ppb: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViableVoiProblem {
    pub now_ns: u64,
    pub initial_state_digest: String,
    pub model_digest: String,
    pub abstraction_digest: String,
    pub domain_digest: String,
    pub probe: ProbeAdmission,
    pub hypotheses: Vec<Hypothesis>,
    pub branches: Vec<Branch>,
    pub outcomes: Vec<Outcome>,
    pub joint_mass_cells: Vec<JointMassCell>,
    pub continuation_evidence: Vec<ContinuationEvidence>,
    pub passive_admitted_action_ids: Vec<String>,
    pub passive_loss_cells: Vec<PassiveLossCell>,
    pub continuation_loss_cells: Vec<DecisionLossCell>,
    pub resource_usage: ResourceUsage,
    pub resource_limits: ResourceLimits,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GateDisposition {
    PhysicalRejected { witness_id: String },
    PhysicalUnknown { reason: String },
    ResourceRejected { resource_id: String },
    Evaluated,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeDecision {
    pub outcome_id: String,
    pub unknown: bool,
    pub joint_mass_ppb: u64,
    pub admitted_action_ids: Vec<String>,
    pub selected_action_id: String,
    /// Unnormalized joint mass × loss numerator; denominator is SCALE.
    pub selected_risk_numerator_ppb_squared: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ViableVoiReport {
    pub disposition: GateDisposition,
    pub passive_selected_action_id: Option<String>,
    /// Unnormalized prior mass × loss numerator; denominator is SCALE.
    pub passive_risk_numerator_ppb_squared: Option<u128>,
    /// Sum over outcomes of unnormalized joint mass × loss; denominator SCALE.
    pub post_probe_risk_numerator_ppb_squared: Option<u128>,
    /// Difference of risk numerators; divide by SCALE² for normalized VOI.
    pub viable_voi_numerator_ppb_cubed: Option<i128>,
    pub unknown_outcome_mass_ppb: u64,
    pub outcome_decisions: Vec<OutcomeDecision>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ViableVoiError {
    EmptyIdentifier,
    InvalidHorizon,
    InvalidReceiptInterval,
    StaleReceipt,
    ReceiptContextMismatch,
    ProbeSubjectMismatch,
    InvalidProbeDuration,
    NoHypotheses,
    NoBranches,
    NoOutcomes,
    DuplicateHypothesisId,
    DuplicateBranchId,
    DuplicateOutcomeId,
    DuplicatePhysicalPair,
    DuplicateHypothesisBranchPair,
    UnknownPhysicalBranch,
    UnknownPhysicalOutcome,
    UnknownHypothesisBranch,
    BranchHasNoPhysicalOutcome,
    OutcomeHasNoPhysicalBranch,
    JointCellCardinalityMismatch,
    DuplicateJointMassCell,
    MissingJointMassCell,
    InvalidJointMass,
    JointMassNotNormalized,
    PositiveMassForImpossibleCell,
    PriorMassNotNormalized,
    DuplicateContinuationOutcome,
    MissingContinuationOutcome,
    ContinuationContextMismatch,
    EmptyPostStateSetDigest,
    DuplicateContinuationAction,
    NoRobustContinuation,
    MissingUnknownFallback,
    UnknownFallbackNotAdmitted,
    NoPassiveActions,
    DuplicatePassiveAction,
    DuplicatePassiveLossCell,
    MissingPassiveLossCell,
    InvalidLoss,
    DuplicateContinuationLossCell,
    MissingContinuationLossCell,
    ArithmeticOverflow,
    ResourceLimitInvalid,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn add_u128(left: u128, right: u128) -> Result<u128, ViableVoiError> {
    left.checked_add(right)
        .ok_or(ViableVoiError::ArithmeticOverflow)
}

fn mul_u128(left: u128, right: u128) -> Result<u128, ViableVoiError> {
    left.checked_mul(right)
        .ok_or(ViableVoiError::ArithmeticOverflow)
}

fn validate_receipt(
    receipt: &EvidenceReceipt,
    problem: &ViableVoiProblem,
    expected_subject: &str,
    expected_horizon: u32,
) -> Result<(), ViableVoiError> {
    if !nonempty(&receipt.receipt_id)
        || !nonempty(&receipt.subject_id)
        || !nonempty(&receipt.initial_state_digest)
        || !nonempty(&receipt.model_digest)
        || !nonempty(&receipt.abstraction_digest)
        || !nonempty(&receipt.domain_digest)
        || !nonempty(&receipt.result_digest)
    {
        return Err(ViableVoiError::EmptyIdentifier);
    }
    if receipt.valid_from_ns > receipt.valid_until_ns {
        return Err(ViableVoiError::InvalidReceiptInterval);
    }
    if problem.now_ns < receipt.valid_from_ns || problem.now_ns > receipt.valid_until_ns {
        return Err(ViableVoiError::StaleReceipt);
    }
    if receipt.initial_state_digest != problem.initial_state_digest
        || receipt.model_digest != problem.model_digest
        || receipt.abstraction_digest != problem.abstraction_digest
        || receipt.domain_digest != problem.domain_digest
        || receipt.horizon_steps != expected_horizon
    {
        return Err(ViableVoiError::ReceiptContextMismatch);
    }
    if receipt.subject_id != expected_subject {
        return Err(ViableVoiError::ProbeSubjectMismatch);
    }
    Ok(())
}

fn validate_unique_nonempty<'a, I>(values: I) -> Result<(), ViableVoiError>
where
    I: IntoIterator<Item = &'a str>,
{
    let mut seen = BTreeSet::new();
    for value in values {
        if !nonempty(value) {
            return Err(ViableVoiError::EmptyIdentifier);
        }
        if !seen.insert(value) {
            return Err(ViableVoiError::DuplicatePassiveAction);
        }
    }
    Ok(())
}

fn resource_exceeded(
    usage: &ResourceUsage,
    limits: &ResourceLimits,
) -> Result<Option<String>, ViableVoiError> {
    if usage.actuator_peak_ppm > SCALE_PPB || limits.actuator_peak_ppm > SCALE_PPB
        || usage.risk_alpha_ppb > SCALE_PPB || limits.risk_alpha_ppb > SCALE_PPB
    {
        return Err(ViableVoiError::ResourceLimitInvalid);
    }
    if usage.energy_mj > limits.energy_mj {
        return Ok(Some("energy_mj".to_owned()));
    }
    if usage.elapsed_ms > limits.elapsed_ms {
        return Ok(Some("elapsed_ms".to_owned()));
    }
    if usage.actuator_peak_ppm > limits.actuator_peak_ppm {
        return Ok(Some("actuator_peak_ppm".to_owned()));
    }
    if usage.risk_alpha_ppb > limits.risk_alpha_ppb {
        return Ok(Some("risk_alpha_ppb".to_owned()));
    }
    Ok(None)
}

fn passive_risk(
    problem: &ViableVoiProblem,
) -> Result<(String, u128), ViableVoiError> {
    if problem.passive_admitted_action_ids.is_empty() {
        return Err(ViableVoiError::NoPassiveActions);
    }
    validate_unique_nonempty(
        problem
            .passive_admitted_action_ids
            .iter()
            .map(String::as_str),
    )?;

    let mut best: Option<(String, u128)> = None;
    for action_id in &problem.passive_admitted_action_ids {
        let mut risk = 0_u128;
        for hypothesis in &problem.hypotheses {
            let matches: Vec<&PassiveLossCell> = problem
                .passive_loss_cells
                .iter()
                .filter(|cell| {
                    cell.action_id == *action_id && cell.hypothesis_id == hypothesis.id
                })
                .collect();
            if matches.len() != 1 {
                return Err(if matches.is_empty() {
                    ViableVoiError::MissingPassiveLossCell
                } else {
                    ViableVoiError::DuplicatePassiveLossCell
                });
            }
            let loss = matches[0].loss_ppb;
            if loss > SCALE_PPB {
                return Err(ViableVoiError::InvalidLoss);
            }
            risk = add_u128(
                risk,
                mul_u128(
                    u128::from(hypothesis.passive_prior_mass_ppb),
                    u128::from(loss),
                )?,
            )?;
        }
        match &best {
            None => best = Some((action_id.clone(), risk)),
            Some((best_id, best_risk)) if risk < *best_risk
                || (risk == *best_risk && action_id < best_id) =>
            {
                best = Some((action_id.clone(), risk));
            }
            Some(_) => {}
        }
    }
    best.ok_or(ViableVoiError::NoPassiveActions)
}

fn continuation_risk_for_action(
    problem: &ViableVoiProblem,
    action_id: &str,
    outcome_id: &str,
) -> Result<u128, ViableVoiError> {
    let mut total = 0_u128;
    for hypothesis in &problem.hypotheses {
        for branch in &problem.branches {
            let masses: Vec<&JointMassCell> = problem
                .joint_mass_cells
                .iter()
                .filter(|cell| {
                    cell.hypothesis_id == hypothesis.id
                        && cell.branch_id == branch.id
                        && cell.outcome_id == outcome_id
                })
                .collect();
            if masses.len() != 1 {
                return Err(if masses.is_empty() {
                    ViableVoiError::MissingJointMassCell
                } else {
                    ViableVoiError::DuplicateJointMassCell
                });
            }

            let losses: Vec<&DecisionLossCell> = problem
                .continuation_loss_cells
                .iter()
                .filter(|cell| {
                    cell.action_id == action_id
                        && cell.hypothesis_id == hypothesis.id
                        && cell.branch_id == branch.id
                        && cell.outcome_id == outcome_id
                })
                .collect();
            if losses.len() != 1 {
                return Err(if losses.is_empty() {
                    ViableVoiError::MissingContinuationLossCell
                } else {
                    ViableVoiError::DuplicateContinuationLossCell
                });
            }
            let loss = losses[0].loss_ppb;
            if loss > SCALE_PPB {
                return Err(ViableVoiError::InvalidLoss);
            }
            total = add_u128(
                total,
                mul_u128(u128::from(masses[0].mass_ppb), u128::from(loss))?,
            )?;
        }
    }
    Ok(total)
}

fn validate_and_compute(
    problem: &ViableVoiProblem,
) -> Result<ViableVoiReport, ViableVoiError> {
    if !nonempty(&problem.initial_state_digest)
        || !nonempty(&problem.model_digest)
        || !nonempty(&problem.abstraction_digest)
        || !nonempty(&problem.domain_digest)
    {
        return Err(ViableVoiError::EmptyIdentifier);
    }
    if problem.probe.receipt.horizon_steps == 0
        || problem.probe.probe_duration_steps == 0
        || problem.probe.probe_duration_steps >= problem.probe.receipt.horizon_steps
    {
        return Err(ViableVoiError::InvalidHorizon);
    }
    validate_receipt(
        &problem.probe.receipt,
        problem,
        &problem.probe.probe_action_id,
        problem.probe.receipt.horizon_steps,
    )?;
    if problem.probe.receipt.subject_id != problem.probe.probe_action_id {
        return Err(ViableVoiError::ProbeSubjectMismatch);
    }

    match &problem.probe.status {
        ProbeGateStatus::Rejected { witness_id } => {
            if !nonempty(witness_id) {
                return Err(ViableVoiError::EmptyIdentifier);
            }
            return Ok(ViableVoiReport {
                disposition: GateDisposition::PhysicalRejected {
                    witness_id: witness_id.clone(),
                },
                passive_selected_action_id: None,
                passive_risk_numerator_ppb_squared: None,
                post_probe_risk_numerator_ppb_squared: None,
                viable_voi_numerator_ppb_cubed: None,
                unknown_outcome_mass_ppb: 0,
                outcome_decisions: Vec::new(),
            });
        }
        ProbeGateStatus::Unknown { reason } => {
            if !nonempty(reason) {
                return Err(ViableVoiError::EmptyIdentifier);
            }
            return Ok(ViableVoiReport {
                disposition: GateDisposition::PhysicalUnknown {
                    reason: reason.clone(),
                },
                passive_selected_action_id: None,
                passive_risk_numerator_ppb_squared: None,
                post_probe_risk_numerator_ppb_squared: None,
                viable_voi_numerator_ppb_cubed: None,
                unknown_outcome_mass_ppb: 0,
                outcome_decisions: Vec::new(),
            });
        }
        ProbeGateStatus::Admitted => {}
    }

    if let Some(resource_id) = resource_exceeded(&problem.resource_usage, &problem.resource_limits)? {
        return Ok(ViableVoiReport {
            disposition: GateDisposition::ResourceRejected { resource_id },
            passive_selected_action_id: None,
            passive_risk_numerator_ppb_squared: None,
            post_probe_risk_numerator_ppb_squared: None,
            viable_voi_numerator_ppb_cubed: None,
            unknown_outcome_mass_ppb: 0,
            outcome_decisions: Vec::new(),
        });
    }

    if problem.hypotheses.is_empty() {
        return Err(ViableVoiError::NoHypotheses);
    }
    if problem.branches.is_empty() {
        return Err(ViableVoiError::NoBranches);
    }
    if problem.outcomes.is_empty() {
        return Err(ViableVoiError::NoOutcomes);
    }
    validate_unique_nonempty(problem.hypotheses.iter().map(|item| item.id.as_str()))?;
    validate_unique_nonempty(problem.branches.iter().map(|item| item.id.as_str()))?;
    validate_unique_nonempty(problem.outcomes.iter().map(|item| item.id.as_str()))?;

    let mut prior_total = 0_u128;
    for hypothesis in &problem.hypotheses {
        if hypothesis.passive_prior_mass_ppb > SCALE_PPB {
            return Err(ViableVoiError::InvalidJointMass);
        }
        prior_total = add_u128(prior_total, u128::from(hypothesis.passive_prior_mass_ppb))?;
    }
    if prior_total != SCALE {
        return Err(ViableVoiError::PriorMassNotNormalized);
    }

    let remaining_horizon = problem.probe.receipt.horizon_steps
        .checked_sub(problem.probe.probe_duration_steps)
        .ok_or(ViableVoiError::InvalidHorizon)?;
    let mut continuation_ids = BTreeSet::new();
    for continuation in &problem.continuation_evidence {
        if !continuation_ids.insert(continuation.outcome_id.as_str()) {
            return Err(ViableVoiError::DuplicateContinuationOutcome);
        }
        if !problem.outcomes.iter().any(|outcome| outcome.id == continuation.outcome_id) {
            return Err(ViableVoiError::UnknownPhysicalOutcome);
        }
        validate_receipt(
            &continuation.receipt,
            problem,
            &continuation.outcome_id,
            remaining_horizon,
        )
        .map_err(|_| ViableVoiError::ContinuationContextMismatch)?;
        if !nonempty(&continuation.post_state_set_digest) {
            return Err(ViableVoiError::EmptyPostStateSetDigest);
        }
        validate_unique_nonempty(
            continuation.robust_admitted_action_ids.iter().map(String::as_str),
        )
        .map_err(|_| ViableVoiError::DuplicateContinuationAction)?;
        if continuation.robust_admitted_action_ids.is_empty() {
            return Err(ViableVoiError::NoRobustContinuation);
        }
        if matches!(
            problem.outcomes.iter().find(|outcome| outcome.id == continuation.outcome_id)
                .map(|outcome| &outcome.kind),
            Some(OutcomeKind::Unknown)
        ) {
            let fallback = continuation
                .explicit_fallback_action_id
                .as_ref()
                .ok_or(ViableVoiError::MissingUnknownFallback)?;
            if !continuation.robust_admitted_action_ids.contains(fallback) {
                return Err(ViableVoiError::UnknownFallbackNotAdmitted);
            }
            if continuation.robust_admitted_action_ids.len() != 1 {
                return Err(ViableVoiError::UnknownFallbackNotAdmitted);
            }
        }
    }
    if continuation_ids.len() != problem.outcomes.len() {
        return Err(ViableVoiError::MissingContinuationOutcome);
    }

    let mut physical_pairs = BTreeSet::new();
    for pair in &problem.probe.physically_possible_branch_outcomes {
        if !problem.branches.iter().any(|branch| branch.id == pair.branch_id) {
            return Err(ViableVoiError::UnknownPhysicalBranch);
        }
        if !problem.outcomes.iter().any(|outcome| outcome.id == pair.outcome_id) {
            return Err(ViableVoiError::UnknownPhysicalOutcome);
        }
        if !physical_pairs.insert((pair.branch_id.as_str(), pair.outcome_id.as_str())) {
            return Err(ViableVoiError::DuplicatePhysicalPair);
        }
    }
    if physical_pairs.is_empty() {
        return Err(ViableVoiError::NoBranches);
    }
    for branch in &problem.branches {
        if !physical_pairs.iter().any(|(branch_id, _)| *branch_id == branch.id) {
            return Err(ViableVoiError::BranchHasNoPhysicalOutcome);
        }
    }
    for outcome in &problem.outcomes {
        if !physical_pairs.iter().any(|(_, outcome_id)| *outcome_id == outcome.id) {
            return Err(ViableVoiError::OutcomeHasNoPhysicalBranch);
        }
    }

    let mut possible_hb = BTreeSet::new();
    for pair in &problem.probe.possible_hypothesis_branches {
        if !problem.hypotheses.iter().any(|item| item.id == pair.hypothesis_id)
            || !problem.branches.iter().any(|item| item.id == pair.branch_id)
        {
            return Err(ViableVoiError::UnknownHypothesisBranch);
        }
        if !possible_hb.insert((pair.hypothesis_id.as_str(), pair.branch_id.as_str())) {
            return Err(ViableVoiError::DuplicateHypothesisBranchPair);
        }
    }

    let expected_joint_cells = problem
        .hypotheses
        .len()
        .checked_mul(problem.branches.len())
        .and_then(|value| value.checked_mul(problem.outcomes.len()))
        .ok_or(ViableVoiError::ArithmeticOverflow)?;
    if expected_joint_cells > MAX_JOINT_CELLS {
        return Err(ViableVoiError::JointCellCardinalityMismatch);
    }
    if problem.joint_mass_cells.len() != expected_joint_cells {
        return Err(ViableVoiError::JointCellCardinalityMismatch);
    }

    let mut joint_keys = BTreeSet::new();
    let mut joint_total = 0_u128;
    for cell in &problem.joint_mass_cells {
        if cell.mass_ppb > SCALE_PPB {
            return Err(ViableVoiError::InvalidJointMass);
        }
        if !problem.hypotheses.iter().any(|item| item.id == cell.hypothesis_id)
            || !problem.branches.iter().any(|item| item.id == cell.branch_id)
            || !problem.outcomes.iter().any(|item| item.id == cell.outcome_id)
        {
            return Err(ViableVoiError::MissingJointMassCell);
        }
        if !joint_keys.insert((
            cell.hypothesis_id.as_str(),
            cell.branch_id.as_str(),
            cell.outcome_id.as_str(),
        )) {
            return Err(ViableVoiError::DuplicateJointMassCell);
        }
        if cell.mass_ppb > 0
            && (!physical_pairs.contains(&(cell.branch_id.as_str(), cell.outcome_id.as_str()))
                || !possible_hb.contains(&(cell.hypothesis_id.as_str(), cell.branch_id.as_str())))
        {
            return Err(ViableVoiError::PositiveMassForImpossibleCell);
        }
        joint_total = add_u128(joint_total, u128::from(cell.mass_ppb))?;
    }
    if joint_total != SCALE {
        return Err(ViableVoiError::JointMassNotNormalized);
    }

    let (passive_action_id, passive_risk) = passive_risk(problem)?;
    let mut outcome_decisions = Vec::with_capacity(problem.outcomes.len());
    let mut post_probe_risk = 0_u128;
    let mut unknown_mass = 0_u64;

    let mut ordered_outcomes = problem.outcomes.clone();
    ordered_outcomes.sort_by(|left, right| left.id.cmp(&right.id));

    for outcome in &ordered_outcomes {
        let continuation = problem
            .continuation_evidence
            .iter()
            .find(|item| item.outcome_id == outcome.id)
            .ok_or(ViableVoiError::MissingContinuationOutcome)?;
        let mut admitted_actions = continuation.robust_admitted_action_ids.clone();
        admitted_actions.sort();

        let mut joint_mass = 0_u128;
        for cell in &problem.joint_mass_cells {
            if cell.outcome_id == outcome.id {
                joint_mass = add_u128(joint_mass, u128::from(cell.mass_ppb))?;
            }
        }
        if outcome.kind == OutcomeKind::Unknown {
            unknown_mass = u64::try_from(add_u128(
                u128::from(unknown_mass),
                joint_mass,
            )?)
            .map_err(|_| ViableVoiError::ArithmeticOverflow)?;
        }

        let mut best: Option<(String, u128)> = None;
        for action_id in &admitted_actions {
            let risk = continuation_risk_for_action(problem, action_id, &outcome.id)?;
            match &best {
                None => best = Some((action_id.clone(), risk)),
                Some((best_id, best_risk)) if risk < *best_risk
                    || (risk == *best_risk && action_id < best_id) =>
                {
                    best = Some((action_id.clone(), risk));
                }
                Some(_) => {}
            }
        }
        let (selected_action_id, selected_risk) =
            best.ok_or(ViableVoiError::NoRobustContinuation)?;
        post_probe_risk = add_u128(post_probe_risk, selected_risk)?;

        outcome_decisions.push(OutcomeDecision {
            outcome_id: outcome.id.clone(),
            unknown: outcome.kind == OutcomeKind::Unknown,
            joint_mass_ppb: u64::try_from(joint_mass)
                .map_err(|_| ViableVoiError::ArithmeticOverflow)?,
            admitted_action_ids: admitted_actions,
            selected_action_id,
            selected_risk_numerator_ppb_squared: selected_risk,
        });
    }

    let passive_signed = i128::try_from(passive_risk)
        .map_err(|_| ViableVoiError::ArithmeticOverflow)?;
    let post_signed = i128::try_from(post_probe_risk)
        .map_err(|_| ViableVoiError::ArithmeticOverflow)?;
    let voi = passive_signed
        .checked_sub(post_signed)
        .ok_or(ViableVoiError::ArithmeticOverflow)?;

    Ok(ViableVoiReport {
        disposition: GateDisposition::Evaluated,
        passive_selected_action_id: Some(passive_action_id),
        passive_risk_numerator_ppb_squared: Some(passive_risk),
        post_probe_risk_numerator_ppb_squared: Some(post_probe_risk),
        viable_voi_numerator_ppb_cubed: Some(voi),
        unknown_outcome_mass_ppb: unknown_mass,
        outcome_decisions,
    })
}

/// Evaluate only after the upstream robust probe gate has admitted every
/// physical branch/prefix. Rejected/Unknown gates return without probability
/// ranking or decision-value arithmetic.
pub fn evaluate_viable_voi(
    problem: &ViableVoiProblem,
) -> Result<ViableVoiReport, ViableVoiError> {
    validate_and_compute(problem)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn receipt(subject: &str, result: &str, horizon: u32) -> EvidenceReceipt {
        EvidenceReceipt {
            receipt_id: format!("receipt-{subject}-{result}"),
            subject_id: subject.to_owned(),
            initial_state_digest: "state-001".to_owned(),
            model_digest: "model-001".to_owned(),
            abstraction_digest: "abstraction-001".to_owned(),
            domain_digest: "domain-001".to_owned(),
            result_digest: result.to_owned(),
            valid_from_ns: 100,
            valid_until_ns: 1_000,
            horizon_steps: horizon,
        }
    }

    fn safe_problem() -> ViableVoiProblem {
        let hypotheses = vec![
            Hypothesis { id: "h-good".to_owned(), passive_prior_mass_ppb: 500_000_000 },
            Hypothesis { id: "h-bad".to_owned(), passive_prior_mass_ppb: 500_000_000 },
        ];
        let branches = vec![
            Branch { id: "b-good".to_owned() },
            Branch { id: "b-degraded".to_owned() },
        ];
        let outcomes = vec![
            Outcome { id: "good-contact".to_owned(), kind: OutcomeKind::Known },
            Outcome { id: "degraded-contact".to_owned(), kind: OutcomeKind::Known },
        ];
        let mut joint_mass_cells = Vec::new();
        for h in &hypotheses {
            for b in &branches {
                for y in &outcomes {
                    let mass = match (h.id.as_str(), b.id.as_str(), y.id.as_str()) {
                        ("h-good", "b-good", "good-contact") => 500_000_000,
                        ("h-bad", "b-degraded", "degraded-contact") => 500_000_000,
                        _ => 0,
                    };
                    joint_mass_cells.push(JointMassCell {
                        hypothesis_id: h.id.clone(),
                        branch_id: b.id.clone(),
                        outcome_id: y.id.clone(),
                        mass_ppb: mass,
                    });
                }
            }
        }

        let continuation_evidence = vec![
            ContinuationEvidence {
                receipt: receipt("good-contact", "continuation-good", 3),
                outcome_id: "good-contact".to_owned(),
                post_state_set_digest: "post-state-good".to_owned(),
                robust_admitted_action_ids: vec!["advance".to_owned(), "retreat".to_owned()],
                explicit_fallback_action_id: None,
            },
            ContinuationEvidence {
                receipt: receipt("degraded-contact", "continuation-degraded", 3),
                outcome_id: "degraded-contact".to_owned(),
                post_state_set_digest: "post-state-degraded".to_owned(),
                robust_admitted_action_ids: vec!["retreat".to_owned()],
                explicit_fallback_action_id: None,
            },
        ];

        let mut continuation_loss_cells = Vec::new();
        for action_id in ["advance", "retreat"] {
            for h in &hypotheses {
                for b in &branches {
                    continuation_loss_cells.push(DecisionLossCell {
                        action_id: action_id.to_owned(),
                        hypothesis_id: h.id.clone(),
                        branch_id: b.id.clone(),
                        outcome_id: "good-contact".to_owned(),
                        loss_ppb: match (action_id, h.id.as_str(), b.id.as_str()) {
                            ("advance", "h-good", "b-good") => 0,
                            ("retreat", "h-good", "b-good") => 600_000_000,
                            _ => 0,
                        },
                    });
                }
            }
        }
        for h in &hypotheses {
            for b in &branches {
                continuation_loss_cells.push(DecisionLossCell {
                    action_id: "retreat".to_owned(),
                    hypothesis_id: h.id.clone(),
                    branch_id: b.id.clone(),
                    outcome_id: "degraded-contact".to_owned(),
                    loss_ppb: match (h.id.as_str(), b.id.as_str()) {
                        ("h-bad", "b-degraded") => 200_000_000,
                        _ => 0,
                    },
                });
            }
        }

        ViableVoiProblem {
            now_ns: 500,
            initial_state_digest: "state-001".to_owned(),
            model_digest: "model-001".to_owned(),
            abstraction_digest: "abstraction-001".to_owned(),
            domain_digest: "domain-001".to_owned(),
            probe: ProbeAdmission {
                receipt: receipt("probe-foot", "probe-admission", 4),
                status: ProbeGateStatus::Admitted,
                probe_action_id: "probe-foot".to_owned(),
                probe_duration_steps: 1,
                branch_ids: vec!["b-good".to_owned(), "b-degraded".to_owned()],
                physically_possible_branch_outcomes: vec![
                    PhysicalBranchOutcome { branch_id: "b-good".to_owned(), outcome_id: "good-contact".to_owned() },
                    PhysicalBranchOutcome { branch_id: "b-degraded".to_owned(), outcome_id: "degraded-contact".to_owned() },
                ],
                possible_hypothesis_branches: vec![
                    PossibleHypothesisBranch { hypothesis_id: "h-good".to_owned(), branch_id: "b-good".to_owned() },
                    PossibleHypothesisBranch { hypothesis_id: "h-bad".to_owned(), branch_id: "b-degraded".to_owned() },
                ],
            },
            hypotheses,
            branches,
            outcomes,
            joint_mass_cells,
            continuation_evidence,
            passive_admitted_action_ids: vec!["passive-A".to_owned(), "passive-B".to_owned()],
            passive_loss_cells: vec![
                PassiveLossCell { action_id: "passive-A".to_owned(), hypothesis_id: "h-good".to_owned(), loss_ppb: 0 },
                PassiveLossCell { action_id: "passive-A".to_owned(), hypothesis_id: "h-bad".to_owned(), loss_ppb: SCALE_PPB },
                PassiveLossCell { action_id: "passive-B".to_owned(), hypothesis_id: "h-good".to_owned(), loss_ppb: SCALE_PPB },
                PassiveLossCell { action_id: "passive-B".to_owned(), hypothesis_id: "h-bad".to_owned(), loss_ppb: 0 },
            ],
            continuation_loss_cells,
            resource_usage: ResourceUsage { energy_mj: 50_000, elapsed_ms: 100, actuator_peak_ppm: 700_000, risk_alpha_ppb: 500_000 },
            resource_limits: ResourceLimits { energy_mj: 100_000, elapsed_ms: 200, actuator_peak_ppm: 900_000, risk_alpha_ppb: 1_000_000 },
        }
    }

    #[test]
    fn viable_voi_uses_outcome_specific_robust_action_sets() {
        let report = evaluate_viable_voi(&safe_problem()).expect("valid finite composition");
        assert_eq!(report.disposition, GateDisposition::Evaluated);
        assert_eq!(report.passive_selected_action_id.as_deref(), Some("passive-A"));
        assert_eq!(report.passive_risk_numerator_ppb_squared, Some(500_000_000_000_000_000));
        assert_eq!(report.post_probe_risk_numerator_ppb_squared, Some(100_000_000_000_000_000));
        assert_eq!(report.viable_voi_numerator_ppb_cubed, Some(400_000_000_000_000_000));
        assert_eq!(
            report.outcome_decisions.iter().map(|item| item.selected_action_id.as_str()).collect::<Vec<_>>(),
            vec!["degraded-contact", "good-contact"].map(|id| if id == "degraded-contact" { "retreat" } else { "advance" })
        );
    }

    #[test]
    fn rejected_physical_gate_returns_without_decision_ranking() {
        let mut problem = safe_problem();
        problem.probe.status = ProbeGateStatus::Rejected {
            witness_id: "branch-loses-recovery".to_owned(),
        };
        problem.joint_mass_cells.clear();
        let report = evaluate_viable_voi(&problem).expect("rejected gate report");
        assert_eq!(
            report.disposition,
            GateDisposition::PhysicalRejected { witness_id: "branch-loses-recovery".to_owned() }
        );
        assert_eq!(report.viable_voi_numerator_ppb_cubed, None);
    }

    #[test]
    fn unknown_mass_is_retained_and_uses_declared_fallback() {
        let mut problem = safe_problem();
        problem.outcomes[1].kind = OutcomeKind::Unknown;
        problem.continuation_evidence[1].explicit_fallback_action_id = Some("retreat".to_owned());
        let report = evaluate_viable_voi(&problem).expect("unknown fallback evaluation");
        assert_eq!(report.unknown_outcome_mass_ppb, 500_000_000);
        assert_eq!(report.outcome_decisions[0].selected_action_id, "retreat");
        assert_eq!(report.outcome_decisions[0].joint_mass_ppb, 500_000_000);
    }

    #[test]
    fn resource_hard_limit_excludes_high_value_probe_before_ranking() {
        let mut problem = safe_problem();
        problem.resource_usage.energy_mj = 100_001;
        let report = evaluate_viable_voi(&problem).expect("resource rejection");
        assert_eq!(report.disposition, GateDisposition::ResourceRejected { resource_id: "energy_mj".to_owned() });
        assert_eq!(report.viable_voi_numerator_ppb_cubed, None);
    }

    #[test]
    fn positive_joint_mass_for_impossible_physical_cell_is_rejected() {
        let mut problem = safe_problem();
        let cell = problem.joint_mass_cells.iter_mut().find(|cell| {
            cell.hypothesis_id == "h-good" && cell.branch_id == "b-degraded" && cell.outcome_id == "good-contact"
        }).expect("joint cell");
        cell.mass_ppb = 1;
        assert_eq!(
            evaluate_viable_voi(&problem),
            Err(ViableVoiError::PositiveMassForImpossibleCell)
        );
    }

    #[test]
    fn missing_outcome_continuation_is_rejected() {
        let mut problem = safe_problem();
        problem.continuation_evidence.pop();
        assert_eq!(
            evaluate_viable_voi(&problem),
            Err(ViableVoiError::MissingContinuationOutcome)
        );
    }

    #[test]
    fn unknown_outcome_requires_exactly_one_declared_fallback_action() {
        let mut problem = safe_problem();
        problem.outcomes[1].kind = OutcomeKind::Unknown;
        problem.continuation_evidence[1].robust_admitted_action_ids = vec!["retreat".to_owned(), "advance".to_owned()];
        problem.continuation_evidence[1].explicit_fallback_action_id = Some("retreat".to_owned());
        assert_eq!(
            evaluate_viable_voi(&problem),
            Err(ViableVoiError::UnknownFallbackNotAdmitted)
        );
    }

    #[test]
    fn hypothesis_branch_mass_and_joint_cells_are_order_invariant() {
        let original = evaluate_viable_voi(&safe_problem()).expect("original result");
        let mut permuted = safe_problem();
        permuted.hypotheses.reverse();
        permuted.branches.reverse();
        permuted.outcomes.reverse();
        permuted.joint_mass_cells.reverse();
        permuted.continuation_loss_cells.reverse();
        let reordered = evaluate_viable_voi(&permuted).expect("permuted result");
        assert_eq!(original, reordered);
    }

    #[test]
    fn stale_continuation_receipt_is_rejected() {
        let mut problem = safe_problem();
        problem.continuation_evidence[0].receipt.valid_until_ns = 499;
        assert_eq!(
            evaluate_viable_voi(&problem),
            Err(ViableVoiError::ContinuationContextMismatch)
        );
    }
}
