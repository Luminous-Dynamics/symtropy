//! Decision-critical value-of-information allocation under hard actuation deadlines.
//!
//! This reference allocator compares predicted decision-loss reduction and keeps
//! latency, energy, simulator and human costs as separate dimensions. It is not
//! a safety authority, calibrated probability model or query executor.

use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const PROBABILITY_SCALE_PPM: u64 = 1_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InputIntegrity {
    ValidFresh,
    Degraded,
    Rejected,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum QueryKind {
    SensorReacquisition,
    AdditionalViewpoint,
    CounterfactualSimulation,
    HumanCorrection,
    HighFidelityWorldModel,
}

impl QueryKind {
    fn permitted_when_rejected(self) -> bool {
        self == Self::SensorReacquisition
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum InformationPrivilege {
    PolicyVisible,
    EvaluatorOnly,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutcomeProvenance {
    PredictedBeforeDecision,
    RealizedAfterDecision,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AllocationPreference {
    ParetoOnly,
    HighestDecisionValue,
    LowestLatency,
    LowestEnergy,
    FewestSimulatorTransitions,
    LeastHumanTime,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ActionEstimate {
    pub action_id: String,
    /// Domain-calibrated decision loss. The unit is local to this task protocol.
    pub expected_loss_units: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceBudget {
    pub maximum_query_latency_us: u64,
    pub maximum_energy_uj: u64,
    pub maximum_simulator_transitions: u64,
    pub maximum_human_intervention_ms: u64,
    pub maximum_model_compute_units: u64,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionContext {
    pub decision_id: String,
    pub model_revision: String,
    pub decision_timestamp_us: u64,
    /// Latest timestamp at which actuation may start.
    pub actuation_deadline_timestamp_us: u64,
    /// Time that must remain available for actuation/fallback after the query.
    pub actuation_reserve_us: u64,
    pub input_integrity: InputIntegrity,
    pub current_action_id: String,
    pub current_action_estimates: Vec<ActionEstimate>,
    /// Allowed absolute difference between prior and reconstructed expected loss.
    /// This only absorbs protocol quantization; it is not a free optimism margin.
    pub outcome_loss_consistency_tolerance_units: u64,
    pub resource_budget: ResourceBudget,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryOutcome {
    /// Probability of this predicted observation/result, in parts per million.
    pub probability_ppm: u32,
    pub action_estimates: Vec<ActionEstimate>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct QueryCandidate {
    pub query_id: String,
    pub kind: QueryKind,
    /// Conservative upper bound, not an average or p50 estimate.
    pub latency_bound_us: u64,
    pub energy_uj: u64,
    pub simulator_transitions: u64,
    pub human_intervention_ms: u64,
    pub model_compute_units: u64,
    pub information_privilege: InformationPrivilege,
    pub outcome_provenance: OutcomeProvenance,
    pub fault_schedule_visible: bool,
    pub outcomes: Vec<QueryOutcome>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct DecisionRequest {
    pub schema_version: u32,
    pub context: DecisionContext,
    pub preference: AllocationPreference,
    pub candidates: Vec<QueryCandidate>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RejectionReason {
    SchemaVersionMismatch,
    EmptyIdentity,
    DuplicateActionId,
    MissingCurrentAction,
    CurrentActionNotBestEstimate,
    DuplicateQueryId,
    EmptyOutcomeDistribution,
    InvalidProbabilityMass,
    EmptyOutcomeActionSet,
    OutcomeActionSetMismatch,
    OutcomeLossDistributionInconsistent,
    DuplicateOutcomeAction,
    NonPolicyVisibleEvidence,
    RealizedOutcomeLeakage,
    FaultScheduleLeakage,
    RejectedInputRequiresReacquisition,
    QueryMissesActuationCutoff,
    QueryLatencyBudgetExceeded,
    EnergyBudgetExceeded,
    SimulatorBudgetExceeded,
    HumanTimeBudgetExceeded,
    ModelComputeBudgetExceeded,
    ZeroDecisionValue,
    InvalidDecisionWindow,
    ArithmeticOverflow,
}

impl RejectionReason {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::SchemaVersionMismatch => "schema_version_mismatch",
            Self::EmptyIdentity => "empty_identity",
            Self::DuplicateActionId => "duplicate_action_id",
            Self::MissingCurrentAction => "missing_current_action",
            Self::CurrentActionNotBestEstimate => "current_action_not_best_estimate",
            Self::DuplicateQueryId => "duplicate_query_id",
            Self::EmptyOutcomeDistribution => "empty_outcome_distribution",
            Self::InvalidProbabilityMass => "invalid_probability_mass",
            Self::EmptyOutcomeActionSet => "empty_outcome_action_set",
            Self::OutcomeActionSetMismatch => "outcome_action_set_mismatch",
            Self::OutcomeLossDistributionInconsistent => "outcome_loss_distribution_inconsistent",
            Self::DuplicateOutcomeAction => "duplicate_outcome_action",
            Self::NonPolicyVisibleEvidence => "non_policy_visible_evidence",
            Self::RealizedOutcomeLeakage => "realized_outcome_leakage",
            Self::FaultScheduleLeakage => "fault_schedule_leakage",
            Self::RejectedInputRequiresReacquisition => "rejected_input_requires_reacquisition",
            Self::QueryMissesActuationCutoff => "query_misses_actuation_cutoff",
            Self::QueryLatencyBudgetExceeded => "query_latency_budget_exceeded",
            Self::EnergyBudgetExceeded => "energy_budget_exceeded",
            Self::SimulatorBudgetExceeded => "simulator_budget_exceeded",
            Self::HumanTimeBudgetExceeded => "human_time_budget_exceeded",
            Self::ModelComputeBudgetExceeded => "model_compute_budget_exceeded",
            Self::ZeroDecisionValue => "zero_decision_value",
            Self::InvalidDecisionWindow => "invalid_decision_window",
            Self::ArithmeticOverflow => "arithmetic_overflow",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CandidateAssessment {
    pub query_id: String,
    pub kind: QueryKind,
    pub feasible: bool,
    pub pareto_frontier: bool,
    pub predicted_best_action_change_probability_ppm: u64,
    pub baseline_best_action_id: String,
    pub expected_loss_before_units: u64,
    /// Rounded upward, conservatively, from the expected integer loss.
    pub expected_loss_after_units: Option<u64>,
    /// Non-negative improvement; zero-value and harmful queries are rejected.
    pub expected_decision_loss_reduction_units: Option<u64>,
    /// Exact expected loss numerator before division by 1,000,000.
    pub expected_loss_after_numerator_ppm: Option<u128>,
    pub completion_timestamp_us: Option<u64>,
    pub latest_query_completion_timestamp_us: u64,
    pub latency_bound_us: u64,
    pub energy_uj: u64,
    pub simulator_transitions: u64,
    pub human_intervention_ms: u64,
    pub model_compute_units: u64,
    pub rejection_reasons: Vec<RejectionReason>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct DecisionReport {
    pub schema_version: u32,
    pub decision_id: String,
    pub model_revision: String,
    pub disposition: String,
    pub baseline_best_action_id: String,
    pub baseline_expected_loss_units: u64,
    pub actuation_deadline_timestamp_us: u64,
    pub actuation_reserve_us: u64,
    pub latest_query_completion_timestamp_us: u64,
    pub preference: AllocationPreference,
    /// Pareto options; no weighted sum is used across resource dimensions.
    pub pareto_frontier_query_ids: Vec<String>,
    /// Optional choice made only by the explicit request preference.
    pub recommended_query_id: Option<String>,
    pub skipped_query_is_valid: bool,
    pub candidates: Vec<CandidateAssessment>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone)]
struct ScoredCandidate {
    assessment: CandidateAssessment,
}

fn unique_action_map(
    estimates: &[ActionEstimate],
) -> Result<BTreeMap<String, u64>, Vec<RejectionReason>> {
    let mut map = BTreeMap::new();
    let mut reasons = BTreeSet::new();
    if estimates.is_empty() {
        reasons.insert(RejectionReason::EmptyOutcomeActionSet);
    }
    for estimate in estimates {
        if estimate.action_id.trim().is_empty() {
            reasons.insert(RejectionReason::EmptyIdentity);
        }
        if map
            .insert(estimate.action_id.clone(), estimate.expected_loss_units)
            .is_some()
        {
            reasons.insert(RejectionReason::DuplicateOutcomeAction);
        }
    }
    if reasons.is_empty() {
        Ok(map)
    } else {
        Err(reasons.into_iter().collect())
    }
}

fn argmin_action(estimates: &[ActionEstimate]) -> Result<(String, u64), Vec<RejectionReason>> {
    let map = unique_action_map(estimates)?;
    let mut best: Option<(String, u64)> = None;
    for (action_id, loss) in map {
        let replace = best.as_ref().is_none_or(|(best_id, best_loss)| {
            loss < *best_loss || (loss == *best_loss && action_id.as_str() < best_id.as_str())
        });
        if replace {
            best = Some((action_id, loss));
        }
    }
    best.ok_or_else(|| vec![RejectionReason::EmptyOutcomeActionSet])
}

fn checked_latest_completion(context: &DecisionContext) -> Result<u64, RejectionReason> {
    context
        .actuation_deadline_timestamp_us
        .checked_sub(context.actuation_reserve_us)
        .filter(|latest| *latest >= context.decision_timestamp_us)
        .ok_or(RejectionReason::InvalidDecisionWindow)
}

fn expected_loss_after(
    candidate: &QueryCandidate,
    current_action_losses: &BTreeMap<String, u64>,
    consistency_tolerance_units: u64,
    current_best_action_id: &str,
) -> Result<(u128, u64, u64), Vec<RejectionReason>> {
    if candidate.outcomes.is_empty() {
        return Err(vec![RejectionReason::EmptyOutcomeDistribution]);
    }
    let current_action_ids = current_action_losses.keys().cloned().collect::<BTreeSet<_>>();
    let mut probability_total = 0_u64;
    let mut loss_numerator = 0_u128;
    let mut action_change_probability = 0_u64;
    let mut prior_loss_numerators = current_action_ids
        .iter()
        .map(|action_id| (action_id.clone(), 0_u128))
        .collect::<BTreeMap<_, _>>();

    for outcome in &candidate.outcomes {
        probability_total = probability_total
            .checked_add(u64::from(outcome.probability_ppm))
            .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?;
        let estimates = unique_action_map(&outcome.action_estimates)?;
        if estimates.keys().cloned().collect::<BTreeSet<_>>() != current_action_ids {
            return Err(vec![RejectionReason::OutcomeActionSetMismatch]);
        }

        for (action_id, action_loss) in &estimates {
            let contribution = u128::from(outcome.probability_ppm)
                .checked_mul(u128::from(*action_loss))
                .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?;
            let total = prior_loss_numerators
                .get_mut(action_id)
                .ok_or_else(|| vec![RejectionReason::OutcomeActionSetMismatch])?;
            *total = total
                .checked_add(contribution)
                .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?;
        }

        let (best_action, best_loss) = argmin_action(&outcome.action_estimates)?;
        loss_numerator = loss_numerator
            .checked_add(
                u128::from(outcome.probability_ppm)
                    .checked_mul(u128::from(best_loss))
                    .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?,
            )
            .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?;
        if best_action != current_best_action_id {
            action_change_probability = action_change_probability
                .checked_add(u64::from(outcome.probability_ppm))
                .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?;
        }
    }
    if probability_total != PROBABILITY_SCALE_PPM {
        return Err(vec![RejectionReason::InvalidProbabilityMass]);
    }

    let allowed_difference = u128::from(consistency_tolerance_units)
        .checked_mul(u128::from(PROBABILITY_SCALE_PPM))
        .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?;
    for (action_id, prior_loss) in current_action_losses {
        let predicted_numerator = *prior_loss_numerators
            .get(action_id)
            .ok_or_else(|| vec![RejectionReason::OutcomeActionSetMismatch])?;
        let prior_numerator = u128::from(*prior_loss)
            .checked_mul(u128::from(PROBABILITY_SCALE_PPM))
            .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?;
        if predicted_numerator.abs_diff(prior_numerator) > allowed_difference {
            return Err(vec![RejectionReason::OutcomeLossDistributionInconsistent]);
        }
    }

    let rounded_up = loss_numerator
        .checked_add(u128::from(PROBABILITY_SCALE_PPM - 1))
        .ok_or_else(|| vec![RejectionReason::ArithmeticOverflow])?
        / u128::from(PROBABILITY_SCALE_PPM);
    let expected_loss_units =
        u64::try_from(rounded_up).map_err(|_| vec![RejectionReason::ArithmeticOverflow])?;
    Ok((loss_numerator, expected_loss_units, action_change_probability))
}

fn candidate_reasons(
    context: &DecisionContext,
    candidate: &QueryCandidate,
    latest_completion: u64,
) -> Vec<RejectionReason> {
    let mut reasons = BTreeSet::new();
    let completion = context
        .decision_timestamp_us
        .checked_add(candidate.latency_bound_us);
    match completion {
        None => {
            reasons.insert(RejectionReason::ArithmeticOverflow);
        }
        Some(timestamp) if timestamp > latest_completion => {
            reasons.insert(RejectionReason::QueryMissesActuationCutoff);
        }
        Some(_) => {}
    }
    if candidate.latency_bound_us > context.resource_budget.maximum_query_latency_us {
        reasons.insert(RejectionReason::QueryLatencyBudgetExceeded);
    }
    if candidate.energy_uj > context.resource_budget.maximum_energy_uj {
        reasons.insert(RejectionReason::EnergyBudgetExceeded);
    }
    if candidate.simulator_transitions > context.resource_budget.maximum_simulator_transitions {
        reasons.insert(RejectionReason::SimulatorBudgetExceeded);
    }
    if candidate.human_intervention_ms > context.resource_budget.maximum_human_intervention_ms {
        reasons.insert(RejectionReason::HumanTimeBudgetExceeded);
    }
    if candidate.model_compute_units > context.resource_budget.maximum_model_compute_units {
        reasons.insert(RejectionReason::ModelComputeBudgetExceeded);
    }
    if candidate.information_privilege != InformationPrivilege::PolicyVisible {
        reasons.insert(RejectionReason::NonPolicyVisibleEvidence);
    }
    if candidate.outcome_provenance != OutcomeProvenance::PredictedBeforeDecision {
        reasons.insert(RejectionReason::RealizedOutcomeLeakage);
    }
    if candidate.fault_schedule_visible {
        reasons.insert(RejectionReason::FaultScheduleLeakage);
    }
    if context.input_integrity == InputIntegrity::Rejected && !candidate.kind.permitted_when_rejected()
    {
        reasons.insert(RejectionReason::RejectedInputRequiresReacquisition);
    }
    reasons.into_iter().collect()
}

fn dominates(left: &CandidateAssessment, right: &CandidateAssessment) -> bool {
    let (Some(left_value), Some(right_value)) = (
        left.expected_decision_loss_reduction_units,
        right.expected_decision_loss_reduction_units,
    ) else {
        return false;
    };
    let no_worse = left_value >= right_value
        && left.latency_bound_us <= right.latency_bound_us
        && left.energy_uj <= right.energy_uj
        && left.simulator_transitions <= right.simulator_transitions
        && left.human_intervention_ms <= right.human_intervention_ms
        && left.model_compute_units <= right.model_compute_units;
    let strictly_better = left_value > right_value
        || left.latency_bound_us < right.latency_bound_us
        || left.energy_uj < right.energy_uj
        || left.simulator_transitions < right.simulator_transitions
        || left.human_intervention_ms < right.human_intervention_ms
        || left.model_compute_units < right.model_compute_units;
    no_worse && strictly_better
}

fn better_by_preference(
    left: &CandidateAssessment,
    right: &CandidateAssessment,
    preference: AllocationPreference,
) -> bool {
    let left_value = left.expected_decision_loss_reduction_units.unwrap_or(0);
    let right_value = right.expected_decision_loss_reduction_units.unwrap_or(0);
    let primary_order = match preference {
        AllocationPreference::ParetoOnly => return false,
        AllocationPreference::HighestDecisionValue => right_value.cmp(&left_value),
        AllocationPreference::LowestLatency => left.latency_bound_us.cmp(&right.latency_bound_us),
        AllocationPreference::LowestEnergy => left.energy_uj.cmp(&right.energy_uj),
        AllocationPreference::FewestSimulatorTransitions => {
            left.simulator_transitions.cmp(&right.simulator_transitions)
        }
        AllocationPreference::LeastHumanTime => {
            left.human_intervention_ms.cmp(&right.human_intervention_ms)
        }
    };
    if primary_order.is_lt() {
        return true;
    }
    if primary_order.is_gt() {
        return false;
    }
    // Explicit deterministic tie-breaks: greater decision value, then lower
    // latency, energy, simulator transitions, human time, compute, and ID.
    (
        std::cmp::Reverse(left_value),
        left.latency_bound_us,
        left.energy_uj,
        left.simulator_transitions,
        left.human_intervention_ms,
        left.model_compute_units,
        &left.query_id,
    ) < (
        std::cmp::Reverse(right_value),
        right.latency_bound_us,
        right.energy_uj,
        right.simulator_transitions,
        right.human_intervention_ms,
        right.model_compute_units,
        &right.query_id,
    )
}

fn validate_request(request: &DecisionRequest) -> Result<(String, u64, u64), Vec<RejectionReason>> {
    let mut reasons = BTreeSet::new();
    if request.schema_version != 1 {
        reasons.insert(RejectionReason::SchemaVersionMismatch);
    }
    for identity in [
        request.context.decision_id.as_str(),
        request.context.model_revision.as_str(),
        request.context.current_action_id.as_str(),
    ] {
        if identity.trim().is_empty() {
            reasons.insert(RejectionReason::EmptyIdentity);
        }
    }
    if request.context.current_action_estimates.is_empty() {
        reasons.insert(RejectionReason::MissingCurrentAction);
    }
    let current_map = match unique_action_map(&request.context.current_action_estimates) {
        Ok(map) => map,
        Err(errors) => {
            reasons.extend(errors);
            BTreeMap::new()
        }
    };
    if !current_map.contains_key(&request.context.current_action_id) {
        reasons.insert(RejectionReason::MissingCurrentAction);
    }
    if request.context.resource_budget.maximum_query_latency_us == 0 {
        reasons.insert(RejectionReason::InvalidDecisionWindow);
    }
    let latest = match checked_latest_completion(&request.context) {
        Ok(value) => value,
        Err(error) => {
            reasons.insert(error);
            request.context.decision_timestamp_us
        }
    };
    let best_current = match argmin_action(&request.context.current_action_estimates) {
        Ok(best) => Some(best),
        Err(errors) => {
            reasons.extend(errors);
            None
        }
    };
    if let Some((best_action_id, _)) = &best_current {
        if best_action_id != &request.context.current_action_id {
            reasons.insert(RejectionReason::CurrentActionNotBestEstimate);
        }
    }
    let (baseline_action_id, baseline_loss) = best_current.unwrap_or_default();
    if reasons.is_empty() {
        Ok((baseline_action_id, baseline_loss, latest))
    } else {
        Err(reasons.into_iter().collect())
    }
}

pub fn allocate(request: &DecisionRequest) -> Result<DecisionReport, Vec<RejectionReason>> {
    let (baseline_best_action_id, baseline_loss, latest_completion) =
        validate_request(request)?;
    let context = &request.context;
    let current_action_losses = context
        .current_action_estimates
        .iter()
        .map(|estimate| (estimate.action_id.clone(), estimate.expected_loss_units))
        .collect::<BTreeMap<_, _>>();
    let mut seen_query_ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(request.candidates.len());

    for candidate in &request.candidates {
        let mut reasons = candidate_reasons(context, candidate, latest_completion)
            .into_iter()
            .collect::<BTreeSet<_>>();
        if candidate.query_id.trim().is_empty() {
            reasons.insert(RejectionReason::EmptyIdentity);
        }
        if !seen_query_ids.insert(candidate.query_id.as_str()) {
            reasons.insert(RejectionReason::DuplicateQueryId);
        }

        let expected = expected_loss_after(
            candidate,
            &current_action_losses,
            context.outcome_loss_consistency_tolerance_units,
            &baseline_best_action_id,
        );
        let (loss_numerator, expected_after, change_probability) = match expected {
            Ok((numerator, after, change_probability)) => {
                (Some(numerator), Some(after), change_probability)
            }
            Err(errors) => {
                reasons.extend(errors);
                (None, None, 0)
            }
        };
        if expected_after.is_some_and(|after| after >= baseline_loss) {
            reasons.insert(RejectionReason::ZeroDecisionValue);
        }
        let reduction = expected_after.map(|after| baseline_loss.saturating_sub(after));
        if reduction == Some(0) {
            reasons.insert(RejectionReason::ZeroDecisionValue);
        }
        let completion = context
            .decision_timestamp_us
            .checked_add(candidate.latency_bound_us);
        let feasible = reasons.is_empty();
        assessments.push(CandidateAssessment {
            query_id: candidate.query_id.clone(),
            kind: candidate.kind,
            feasible,
            pareto_frontier: false,
            predicted_best_action_change_probability_ppm: change_probability,
            baseline_best_action_id: baseline_best_action_id.clone(),
            expected_loss_before_units: baseline_loss,
            expected_loss_after_units: expected_after,
            expected_decision_loss_reduction_units: reduction,
            expected_loss_after_numerator_ppm: loss_numerator,
            completion_timestamp_us: completion,
            latest_query_completion_timestamp_us: latest_completion,
            latency_bound_us: candidate.latency_bound_us,
            energy_uj: candidate.energy_uj,
            simulator_transitions: candidate.simulator_transitions,
            human_intervention_ms: candidate.human_intervention_ms,
            model_compute_units: candidate.model_compute_units,
            rejection_reasons: reasons.into_iter().collect(),
        });
    }

    let mut frontier_indices = Vec::new();
    for index in 0..assessments.len() {
        if !assessments[index].feasible {
            continue;
        }
        let dominated = assessments.iter().enumerate().any(|(other_index, other)| {
            other_index != index && other.feasible && dominates(other, &assessments[index])
        });
        if !dominated {
            assessments[index].pareto_frontier = true;
            frontier_indices.push(index);
        }
    }
    let mut frontier_ids = frontier_indices
        .iter()
        .map(|index| assessments[*index].query_id.clone())
        .collect::<Vec<_>>();
    frontier_ids.sort();

    let recommended_index = if request.preference == AllocationPreference::ParetoOnly {
        None
    } else {
        frontier_indices
            .iter()
            .map(|index| &assessments[*index])
            .reduce(|best, current| {
                if better_by_preference(current, best, request.preference) {
                    current
                } else {
                    best
                }
            })
            .and_then(|selected| {
                assessments
                    .iter()
                    .position(|assessment| assessment.query_id == selected.query_id)
            })
    };
    let recommended_query_id =
        recommended_index.map(|index| assessments[index].query_id.clone());
    let disposition = if assessments.is_empty() {
        "no_query_candidates"
    } else if frontier_ids.is_empty() {
        "no_feasible_positive_voi_query"
    } else if recommended_query_id.is_none() {
        "pareto_options_available"
    } else {
        "query_recommended"
    };

    Ok(DecisionReport {
        schema_version: 1,
        decision_id: context.decision_id.clone(),
        model_revision: context.model_revision.clone(),
        disposition: disposition.to_string(),
        baseline_best_action_id,
        baseline_expected_loss_units: baseline_loss,
        actuation_deadline_timestamp_us: context.actuation_deadline_timestamp_us,
        actuation_reserve_us: context.actuation_reserve_us,
        latest_query_completion_timestamp_us: latest_completion,
        preference: request.preference,
        pareto_frontier_query_ids: frontier_ids,
        recommended_query_id,
        skipped_query_is_valid: true,
        candidates: assessments,
        claim_ceiling: "Predicted decision-value arithmetic and hard resource/deadline conformance for declared action-loss distributions only. This does not establish probability calibration, correct human/safety loss weights, simulator validity, vehicle safety or authority to actuate.".to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estimate(action: &str, loss: u64) -> ActionEstimate {
        ActionEstimate {
            action_id: action.to_string(),
            expected_loss_units: loss,
        }
    }

    fn candidate(
        query_id: &str,
        kind: QueryKind,
        latency: u64,
        energy: u64,
        steps: u64,
        human_ms: u64,
        outcomes: Vec<QueryOutcome>,
    ) -> QueryCandidate {
        QueryCandidate {
            query_id: query_id.to_string(),
            kind,
            latency_bound_us: latency,
            energy_uj: energy,
            simulator_transitions: steps,
            human_intervention_ms: human_ms,
            model_compute_units: 100,
            information_privilege: InformationPrivilege::PolicyVisible,
            outcome_provenance: OutcomeProvenance::PredictedBeforeDecision,
            fault_schedule_visible: false,
            outcomes,
        }
    }

    fn outcome(probability: u32, brake: u64, proceed: u64) -> QueryOutcome {
        QueryOutcome {
            probability_ppm: probability,
            action_estimates: vec![estimate("brake", brake), estimate("proceed", proceed)],
        }
    }

    fn base_request(candidates: Vec<QueryCandidate>) -> DecisionRequest {
        DecisionRequest {
            schema_version: 1,
            context: DecisionContext {
                decision_id: "pedestrian-occlusion-decision-1".to_string(),
                model_revision: "symthaea-hdc-ltc-test-v1".to_string(),
                decision_timestamp_us: 1_000_000,
                actuation_deadline_timestamp_us: 1_100_000,
                actuation_reserve_us: 30_000,
                input_integrity: InputIntegrity::ValidFresh,
                current_action_id: "brake".to_string(),
                current_action_estimates: vec![
                    estimate("brake", 50),
                    estimate("proceed", 50),
                ],
                outcome_loss_consistency_tolerance_units: 0,
                resource_budget: ResourceBudget {
                    maximum_query_latency_us: 60_000,
                    maximum_energy_uj: 10_000,
                    maximum_simulator_transitions: 8,
                    maximum_human_intervention_ms: 30_000,
                    maximum_model_compute_units: 200,
                },
            },
            preference: AllocationPreference::HighestDecisionValue,
            candidates,
        }
    }

    fn beneficial_query(query_id: &str, kind: QueryKind, latency: u64) -> QueryCandidate {
        candidate(
            query_id,
            kind,
            latency,
            500,
            2,
            0,
            vec![outcome(500_000, 20, 80), outcome(500_000, 80, 20)],
        )
    }

    fn higher_value_query(query_id: &str, kind: QueryKind, latency: u64) -> QueryCandidate {
        candidate(
            query_id,
            kind,
            latency,
            1_500,
            4,
            0,
            vec![outcome(500_000, 5, 95), outcome(500_000, 95, 5)],
        )
    }

    fn rejection(report: &DecisionReport, query_id: &str, expected: RejectionReason) -> bool {
        report
            .candidates
            .iter()
            .find(|item| item.query_id == query_id)
            .is_some_and(|item| item.rejection_reasons.contains(&expected))
    }

    #[test]
    fn entropy_without_action_or_loss_change_has_zero_decision_value() {
        let irrelevant = candidate(
            "high-entropy-but-irrelevant",
            QueryKind::HighFidelityWorldModel,
            10_000,
            200,
            1,
            0,
            vec![
                outcome(500_000, 40, 100),
                outcome(500_000, 60, 100),
            ],
        );
        let report = allocate(&base_request(vec![irrelevant])).unwrap();
        assert_eq!(report.disposition, "no_feasible_positive_voi_query");
        assert!(rejection(&report, "high-entropy-but-irrelevant", RejectionReason::ZeroDecisionValue));
        assert_eq!(report.candidates[0].predicted_best_action_change_probability_ppm, 0);
    }

    #[test]
    fn useful_observation_with_positive_decision_value_is_pareto_eligible() {
        let report = allocate(&base_request(vec![beneficial_query(
            "reacquire-front-camera",
            QueryKind::SensorReacquisition,
            10_000,
        )]))
        .unwrap();
        assert_eq!(report.disposition, "query_recommended");
        assert_eq!(
            report.recommended_query_id.as_deref(),
            Some("reacquire-front-camera")
        );
        assert_eq!(report.candidates[0].expected_decision_loss_reduction_units, Some(30));
        assert_eq!(report.candidates[0].predicted_best_action_change_probability_ppm, 500_000);
    }

    #[test]
    fn query_that_finishes_after_actuation_reserve_is_rejected() {
        let report = allocate(&base_request(vec![beneficial_query(
            "too-late-world-rollout",
            QueryKind::CounterfactualSimulation,
            71_000,
        )]))
        .unwrap();
        assert!(rejection(
            &report,
            "too-late-world-rollout",
            RejectionReason::QueryMissesActuationCutoff
        ));
        assert_eq!(report.recommended_query_id, None);
    }

    #[test]
    fn rejected_input_only_permits_sensor_reacquisition() {
        let mut request = base_request(vec![
            beneficial_query("reacquire", QueryKind::SensorReacquisition, 10_000),
            beneficial_query("reason-on-invalid-frame", QueryKind::HighFidelityWorldModel, 10_000),
        ]);
        request.context.input_integrity = InputIntegrity::Rejected;
        let report = allocate(&request).unwrap();
        assert!(report.pareto_frontier_query_ids.contains(&"reacquire".to_string()));
        assert!(rejection(
            &report,
            "reason-on-invalid-frame",
            RejectionReason::RejectedInputRequiresReacquisition
        ));
    }

    #[test]
    fn each_resource_budget_is_enforced_separately() {
        let mut expensive = beneficial_query("expensive", QueryKind::CounterfactualSimulation, 10_000);
        expensive.energy_uj = 10_001;
        expensive.simulator_transitions = 9;
        expensive.human_intervention_ms = 30_001;
        expensive.model_compute_units = 201;
        let report = allocate(&base_request(vec![expensive])).unwrap();
        assert!(rejection(&report, "expensive", RejectionReason::EnergyBudgetExceeded));
        assert!(rejection(&report, "expensive", RejectionReason::SimulatorBudgetExceeded));
        assert!(rejection(&report, "expensive", RejectionReason::HumanTimeBudgetExceeded));
        assert!(rejection(&report, "expensive", RejectionReason::ModelComputeBudgetExceeded));
    }

    #[test]
    fn invalid_probability_mass_is_rejected() {
        let malformed = candidate(
            "bad-probabilities",
            QueryKind::AdditionalViewpoint,
            10_000,
            100,
            0,
            0,
            vec![outcome(500_000, 30, 120)],
        );
        let report = allocate(&base_request(vec![malformed])).unwrap();
        assert!(rejection(
            &report,
            "bad-probabilities",
            RejectionReason::InvalidProbabilityMass
        ));
    }

    #[test]
    fn optimistic_post_query_losses_that_break_prior_bayes_consistency_are_rejected() {
        let optimistic = candidate(
            "fabricated-optimism",
            QueryKind::HighFidelityWorldModel,
            10_000,
            100,
            1,
            0,
            vec![
                outcome(500_000, 1, 1),
                outcome(500_000, 1, 1),
            ],
        );
        let report = allocate(&base_request(vec![optimistic])).unwrap();
        assert!(rejection(
            &report,
            "fabricated-optimism",
            RejectionReason::OutcomeLossDistributionInconsistent
        ));
    }

    #[test]
    fn query_outcome_cannot_change_the_declared_action_set() {
        let mut malformed = beneficial_query("action-set-leak", QueryKind::SensorReacquisition, 10_000);
        malformed.outcomes[0].action_estimates.push(estimate("hidden-action", 0));
        let report = allocate(&base_request(vec![malformed])).unwrap();
        assert!(rejection(
            &report,
            "action-set-leak",
            RejectionReason::OutcomeActionSetMismatch
        ));
    }

    #[test]
    fn evaluator_truth_realized_outcomes_and_fault_schedule_are_rejected() {
        let mut evaluator_only = beneficial_query("evaluator-only", QueryKind::CounterfactualSimulation, 10_000);
        evaluator_only.information_privilege = InformationPrivilege::EvaluatorOnly;
        let mut realized = beneficial_query("realized-label", QueryKind::HighFidelityWorldModel, 10_000);
        realized.outcome_provenance = OutcomeProvenance::RealizedAfterDecision;
        let mut leaked_schedule = beneficial_query("fault-schedule-leak", QueryKind::SensorReacquisition, 10_000);
        leaked_schedule.fault_schedule_visible = true;
        let report = allocate(&base_request(vec![evaluator_only, realized, leaked_schedule])).unwrap();
        assert!(rejection(&report, "evaluator-only", RejectionReason::NonPolicyVisibleEvidence));
        assert!(rejection(&report, "realized-label", RejectionReason::RealizedOutcomeLeakage));
        assert!(rejection(&report, "fault-schedule-leak", RejectionReason::FaultScheduleLeakage));
    }

    #[test]
    fn pareto_frontier_preserves_resource_tradeoffs_without_weighted_sum() {
        let fast = candidate(
            "fast-lower-value",
            QueryKind::SensorReacquisition,
            5_000,
            100,
            0,
            0,
            vec![outcome(500_000, 20, 80), outcome(500_000, 80, 20)],
        );
        let valuable = higher_value_query(
            "slower-higher-value",
            QueryKind::CounterfactualSimulation,
            20_000,
        );
        let dominated = candidate(
            "dominated",
            QueryKind::HighFidelityWorldModel,
            30_000,
            900,
            3,
            0,
            vec![outcome(500_000, 40, 60), outcome(500_000, 60, 40)],
        );
        let report = allocate(&base_request(vec![fast, valuable, dominated])).unwrap();
        assert!(report.pareto_frontier_query_ids.contains(&"fast-lower-value".to_string()));
        assert!(report.pareto_frontier_query_ids.contains(&"slower-higher-value".to_string()));
        assert!(!report.pareto_frontier_query_ids.contains(&"dominated".to_string()));
    }

    #[test]
    fn explicit_preference_selects_only_among_pareto_options() {
        let mut request = base_request(vec![
            higher_value_query("slow-high-benefit", QueryKind::CounterfactualSimulation, 20_000),
            candidate(
                "fast-low-cost",
                QueryKind::SensorReacquisition,
                5_000,
                100,
                0,
                0,
                vec![outcome(500_000, 20, 80), outcome(500_000, 80, 20)],
            ),
        ]);
        request.preference = AllocationPreference::LowestLatency;
        let report = allocate(&request).unwrap();
        assert_eq!(report.recommended_query_id.as_deref(), Some("fast-low-cost"));
        request.preference = AllocationPreference::ParetoOnly;
        let report = allocate(&request).unwrap();
        assert_eq!(report.recommended_query_id, None);
        assert_eq!(report.disposition, "pareto_options_available");
    }

    #[test]
    fn no_query_is_a_valid_first_class_result() {
        let report = allocate(&base_request(Vec::new())).unwrap();
        assert_eq!(report.disposition, "no_query_candidates");
        assert!(report.skipped_query_is_valid);
        assert_eq!(report.recommended_query_id, None);
    }

    #[test]
    fn invalid_decision_window_rejects_the_campaign_not_just_one_query() {
        let mut request = base_request(vec![beneficial_query("query", QueryKind::SensorReacquisition, 1_000)]);
        request.context.actuation_reserve_us = 200_000;
        let errors = allocate(&request).unwrap_err();
        assert!(errors.contains(&RejectionReason::InvalidDecisionWindow));
    }

    #[test]
    fn current_policy_action_must_match_the_declared_current_loss_argmin() {
        let mut request = base_request(vec![]);
        request.context.current_action_id = "proceed".to_string();
        let errors = allocate(&request).unwrap_err();
        assert!(errors.contains(&RejectionReason::CurrentActionNotBestEstimate));
    }

    #[test]
    fn outcome_probability_action_change_probability_is_bounded_by_declared_mass() {
        let report = allocate(&base_request(vec![beneficial_query(
            "bounded-action-change",
            QueryKind::SensorReacquisition,
            10_000,
        )]))
        .unwrap();
        assert_eq!(
            report.candidates[0].predicted_best_action_change_probability_ppm,
            500_000
        );
        assert!(report.candidates[0].predicted_best_action_change_probability_ppm
            <= PROBABILITY_SCALE_PPM);
    }

    #[test]
    fn duplicate_query_ids_and_duplicate_action_ids_are_rejected() {
        let query = beneficial_query("duplicate", QueryKind::SensorReacquisition, 1_000);
        let report = allocate(&base_request(vec![query.clone(), query])).unwrap();
        assert!(report.candidates[1].rejection_reasons.contains(&RejectionReason::DuplicateQueryId));

        let mut request = base_request(vec![]);
        request.context.current_action_estimates.push(estimate("proceed", 1));
        assert!(allocate(&request).unwrap_err().contains(&RejectionReason::DuplicateOutcomeAction));
    }
}
