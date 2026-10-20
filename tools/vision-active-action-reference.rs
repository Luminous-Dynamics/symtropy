// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline evaluator arithmetic for active visual-action selection.
//!
//! This is an evaluator-side reference, NOT an online policy or sensor API.
//! Counterfactual losses and candidate costs must remain evaluator-private until
//! the policy action has been committed. Realized task loss and actual resource
//! charges are post-execution outcomes.
//!
//! Standalone qualification command (no workspace membership or dependencies):
//! rustc --edition=2024 --test tools/vision-active-action-reference.rs -o /tmp/vis-act-tests
//! /tmp/vis-act-tests

#![deny(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum VisionAction {
    KeepCheap,
    EscalateCurrent,
    AcquireNext,
    ChangeView,
    WaitForEvent,
    AbstainOrHandoff,
}

impl VisionAction {
    /// These choices can acquire a new sensor observation only after execution.
    pub fn can_acquire_observation(self) -> bool {
        matches!(self, Self::AcquireNext | Self::ChangeView | Self::WaitForEvent)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResourceAxisProfileV1 {
    /// Stable identifier for the unit, e.g. latency_us, gpu_work_unit,
    /// sensor_sample, or energy_uj. Never mix unlike units.
    pub unit_id: String,
    /// Optional per-trial hard ceiling, expressed in unit_id.
    pub trial_budget: Option<u64>,
    /// Optional run-wide hard ceiling, expressed in unit_id.
    pub run_budget: Option<u128>,
}

pub type ResourceProfileV1 = BTreeMap<String, ResourceAxisProfileV1>;

/// Evaluator-private outcome of a candidate action in a paired synthetic trial.
/// Its task loss and resource quote are not producer-visible before commitment.
#[derive(Debug, Clone, PartialEq)]
pub struct CandidateOutcomeV1 {
    pub action: VisionAction,
    /// False for physically impossible, disallowed, or otherwise illegal actions.
    pub feasible: bool,
    pub counterfactual_task_loss: f64,
    /// Declared/estimated candidate resource use in each axis' canonical unit.
    /// This vector is for budget eligibility; it is not a measured energy claim.
    pub declared_resource_units: BTreeMap<String, u64>,
}

/// One evaluator-owned trial. chosen_action is the policy's committed action;
/// realized_task_loss and actual resource charges are post-action observations.
/// Candidate outcomes are evaluator-only counterfactuals.
#[derive(Debug, Clone, PartialEq)]
pub struct TrialV1 {
    pub trial_id: u64,
    pub loss_profile_id: String,
    pub resource_profile: ResourceProfileV1,
    pub candidates: Vec<CandidateOutcomeV1>,
    pub chosen_action: VisionAction,
    /// True only when the selected action really executed.
    pub action_executed: bool,
    /// True only when execution returned a fresh observation via the declared
    /// sensor channel (not a render, prediction, or unexecuted alternate view).
    pub fresh_observation_received: bool,
    /// Measured loss for what actually happened, including fallback after a
    /// failed action. It is not copied from a counterfactual outcome.
    pub realized_task_loss: f64,
    /// None means unknown/not measured, never zero. Units come from profile.
    pub actual_resource_units: BTreeMap<String, Option<u64>>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BudgetStatus {
    WithinBudget,
    Exceeded,
    Unknown,
    NotDeclared,
}

#[derive(Debug, Clone, PartialEq)]
pub struct TrialScoreV1 {
    pub trial_id: u64,
    pub loss_profile_id: String,
    pub chosen_action: VisionAction,
    pub action_executed: bool,
    pub realized_task_loss: f64,
    pub best_trial_budget_feasible_counterfactual_loss: f64,
    /// Can be negative when the realized outcome differs stochastically from
    /// the evaluator's paired counterfactuals. Do not clamp or discard negatives.
    pub regret_vs_trial_oracle: f64,
    /// Positive means the chosen action's evaluator-side counterfactual loss is
    /// lower than KeepCheap; this is not producer-visible or a causal claim by itself.
    pub counterfactual_action_value_vs_cheap: f64,
    pub actual_resource_units: BTreeMap<String, Option<u64>>,
    pub trial_budget_status: BTreeMap<String, BudgetStatus>,
    /// Retained for run-level validation; never send candidate outcomes to policy.
    pub resource_profile: ResourceProfileV1,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunSummaryV1 {
    pub scores: Vec<TrialScoreV1>,
    pub mean_realized_loss: f64,
    pub mean_trial_oracle_loss: f64,
    pub mean_regret_vs_trial_oracle: f64,
    pub mean_run_budget_feasible_counterfactual_loss: f64,
    pub mean_regret_vs_run_budget_oracle: f64,
    /// Evaluator-only choices, aligned to scores sorted by trial_id.
    pub run_budget_oracle_actions: Vec<VisionAction>,
    pub total_actual_resource_units: BTreeMap<String, Option<u128>>,
    pub run_budget_status: BTreeMap<String, BudgetStatus>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EvaluationError {
    EmptyRun,
    EmptyLossProfile,
    InvalidResourceAxis,
    InvalidResourceUnit,
    EmptyResourceProfile,
    ResourceAxesMismatch,
    DuplicateCandidateAction,
    MissingCheapBaseline,
    MissingChosenAction,
    ChosenActionInfeasible,
    ChosenActionOverTrialBudget,
    NoBudgetFeasibleCandidate,
    NoRunBudgetFeasiblePlan,
    OracleStateLimitExceeded,
    NonFiniteCandidateLoss,
    NonFiniteRealizedLoss,
    FreshObservationWithoutExecutedAcquisition,
    DuplicateTrialId,
    MixedLossProfiles,
    MixedResourceAxes,
    InconsistentRunBudget(String),
    ResourceArithmeticOverflow(String),
    NonFiniteAggregate,
}

fn same_keys<T, U>(left: &BTreeMap<String, T>, right: &BTreeMap<String, U>) -> bool {
    left.len() == right.len() && left.keys().eq(right.keys())
}

fn validate_profile(profile: &ResourceProfileV1) -> Result<(), EvaluationError> {
    if profile.is_empty() {
        return Err(EvaluationError::EmptyResourceProfile);
    }
    for (axis_id, axis) in profile {
        if axis_id.trim().is_empty() {
            return Err(EvaluationError::InvalidResourceAxis);
        }
        if axis.unit_id.trim().is_empty() {
            return Err(EvaluationError::InvalidResourceUnit);
        }
    }
    Ok(())
}

fn under_trial_budgets(
    cost: &BTreeMap<String, u64>,
    profile: &ResourceProfileV1,
) -> bool {
    profile.iter().all(|(axis_id, axis)| {
        axis.trial_budget
            .map(|limit| cost.get(axis_id).is_some_and(|used| *used <= limit))
            .unwrap_or(true)
    })
}

fn status_for_amount(amount: Option<u64>, budget: Option<u64>) -> BudgetStatus {
    match (amount, budget) {
        (_, None) => BudgetStatus::NotDeclared,
        (None, Some(_)) => BudgetStatus::Unknown,
        (Some(used), Some(limit)) if used > limit => BudgetStatus::Exceeded,
        (Some(_), Some(_)) => BudgetStatus::WithinBudget,
    }
}

/// Score one trial after the policy choice and execution result are committed.
/// Only feasible actions that satisfy declared per-trial budgets enter this per-trial
/// oracle comparison. Run-wide budgets are checked after aggregation, but this
/// reference does not solve the global run-budget allocation problem.
pub fn evaluate_trial_v1(trial: &TrialV1) -> Result<TrialScoreV1, EvaluationError> {
    if trial.loss_profile_id.trim().is_empty() {
        return Err(EvaluationError::EmptyLossProfile);
    }
    if !trial.realized_task_loss.is_finite() {
        return Err(EvaluationError::NonFiniteRealizedLoss);
    }
    validate_profile(&trial.resource_profile)?;

    if !same_keys(&trial.actual_resource_units, &trial.resource_profile) {
        return Err(EvaluationError::ResourceAxesMismatch);
    }

    if trial.fresh_observation_received
        && (!trial.action_executed || !trial.chosen_action.can_acquire_observation())
    {
        return Err(EvaluationError::FreshObservationWithoutExecutedAcquisition);
    }

    let mut by_action = BTreeMap::new();
    for candidate in &trial.candidates {
        if !candidate.counterfactual_task_loss.is_finite() {
            return Err(EvaluationError::NonFiniteCandidateLoss);
        }
        if !same_keys(&candidate.declared_resource_units, &trial.resource_profile) {
            return Err(EvaluationError::ResourceAxesMismatch);
        }
        if by_action.insert(candidate.action, candidate).is_some() {
            return Err(EvaluationError::DuplicateCandidateAction);
        }
    }

    let cheap = by_action
        .get(&VisionAction::KeepCheap)
        .ok_or(EvaluationError::MissingCheapBaseline)?;
    let chosen = by_action
        .get(&trial.chosen_action)
        .ok_or(EvaluationError::MissingChosenAction)?;

    if !chosen.feasible {
        return Err(EvaluationError::ChosenActionInfeasible);
    }
    if !under_trial_budgets(&chosen.declared_resource_units, &trial.resource_profile) {
        return Err(EvaluationError::ChosenActionOverTrialBudget);
    }

    let oracle_loss = by_action
        .values()
        .filter(|candidate| {
            candidate.feasible
                && under_trial_budgets(
                    &candidate.declared_resource_units,
                    &trial.resource_profile,
                )
        })
        .map(|candidate| candidate.counterfactual_task_loss)
        .min_by(f64::total_cmp)
        .ok_or(EvaluationError::NoBudgetFeasibleCandidate)?;

    let regret = trial.realized_task_loss - oracle_loss;
    let action_value = cheap.counterfactual_task_loss - chosen.counterfactual_task_loss;
    if !regret.is_finite() || !action_value.is_finite() {
        return Err(EvaluationError::NonFiniteAggregate);
    }

    let trial_budget_status = trial
        .resource_profile
        .iter()
        .map(|(axis_id, axis)| {
            (
                axis_id.clone(),
                status_for_amount(
                    trial.actual_resource_units.get(axis_id).copied().flatten(),
                    axis.trial_budget,
                ),
            )
        })
        .collect();

    Ok(TrialScoreV1 {
        trial_id: trial.trial_id,
        loss_profile_id: trial.loss_profile_id.clone(),
        chosen_action: trial.chosen_action,
        action_executed: trial.action_executed,
        realized_task_loss: trial.realized_task_loss,
        best_trial_budget_feasible_counterfactual_loss: oracle_loss,
        regret_vs_trial_oracle: regret,
        counterfactual_action_value_vs_cheap: action_value,
        actual_resource_units: trial.actual_resource_units.clone(),
        trial_budget_status,
        resource_profile: trial.resource_profile.clone(),
    })
}

/// Aggregate same-profile trials and enforce declared run-wide budgets.
/// If any actual charge is missing on an axis, that axis' total remains unknown.
/// Per-trial losses and costs are never silently imputed as zero.
const MAX_RUN_ORACLE_STATES: usize = 100_000;

/// Exact dynamic program over declared run-budget axes. It optimizes the sum of
/// evaluator-only counterfactual losses subject to per-trial eligibility and
/// declared run-wide budget limits. Costs are declared estimates, not actual
/// measurements. Fails closed rather than returning an approximate oracle when
/// the number of unique resource states exceeds the explicit cap.
fn run_budget_feasible_oracle_v1(
    trials: &[TrialV1],
) -> Result<(f64, Vec<VisionAction>), EvaluationError> {
    let first = trials.first().ok_or(EvaluationError::EmptyRun)?;
    let bounded_axes: Vec<(String, u128)> = first
        .resource_profile
        .iter()
        .filter_map(|(axis_id, axis)| {
            axis.run_budget.map(|budget| (axis_id.clone(), budget))
        })
        .collect();

    let initial_state = vec![0_u128; bounded_axes.len()];
    let mut states: BTreeMap<Vec<u128>, (f64, Vec<VisionAction>)> =
        BTreeMap::from([(initial_state, (0.0, Vec::new()))]);

    for trial in trials {
        let mut candidates: Vec<&CandidateOutcomeV1> = trial
            .candidates
            .iter()
            .filter(|candidate| {
                candidate.feasible
                    && under_trial_budgets(
                        &candidate.declared_resource_units,
                        &trial.resource_profile,
                    )
            })
            .collect();
        candidates.sort_by_key(|candidate| candidate.action);

        let mut next_states: BTreeMap<Vec<u128>, (f64, Vec<VisionAction>)> = BTreeMap::new();
        for (spent, (prior_loss, prior_actions)) in &states {
            for candidate in &candidates {
                let mut next_spent = spent.clone();
                let mut within_run_budget = true;
                for (axis_index, (axis_id, limit)) in bounded_axes.iter().enumerate() {
                    let units = candidate
                        .declared_resource_units
                        .get(axis_id)
                        .copied()
                        .ok_or(EvaluationError::ResourceAxesMismatch)?;
                    let total = next_spent[axis_index]
                        .checked_add(u128::from(units))
                        .ok_or_else(|| EvaluationError::ResourceArithmeticOverflow(axis_id.clone()))?;
                    if total > *limit {
                        within_run_budget = false;
                        break;
                    }
                    next_spent[axis_index] = total;
                }
                if !within_run_budget {
                    continue;
                }

                let total_loss = *prior_loss + candidate.counterfactual_task_loss;
                if !total_loss.is_finite() {
                    return Err(EvaluationError::NonFiniteAggregate);
                }
                let mut actions = prior_actions.clone();
                actions.push(candidate.action);

                match next_states.get_mut(&next_spent) {
                    Some(incumbent) => {
                        let lower_loss = total_loss.total_cmp(&incumbent.0).is_lt();
                        let deterministic_tie_break =
                            total_loss.total_cmp(&incumbent.0).is_eq() && actions < incumbent.1;
                        if lower_loss || deterministic_tie_break {
                            *incumbent = (total_loss, actions);
                        }
                    }
                    None => {
                        next_states.insert(next_spent, (total_loss, actions));
                        if next_states.len() > MAX_RUN_ORACLE_STATES {
                            return Err(EvaluationError::OracleStateLimitExceeded);
                        }
                    }
                }
            }
        }

        if next_states.is_empty() {
            return Err(EvaluationError::NoRunBudgetFeasiblePlan);
        }
        states = next_states;
    }

    states
        .into_values()
        .min_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        })
        .map(|(loss, actions)| (loss, actions))
        .ok_or(EvaluationError::NoRunBudgetFeasiblePlan)
}

/// Aggregate same-profile trials, compute a run-budget-constrained counterfactual
/// oracle, and enforce actual run-wide budgets. If any actual charge is missing
/// on an axis, that axis' measured total remains unknown. Per-trial results and
/// oracle outcomes remain evaluator-side; do not serialize them to the policy.
pub fn summarize_run_v1(trials: &[TrialV1]) -> Result<RunSummaryV1, EvaluationError> {
    if trials.is_empty() {
        return Err(EvaluationError::EmptyRun);
    }

    let mut ordered_trials = trials.to_vec();
    ordered_trials.sort_by_key(|trial| trial.trial_id);

    let mut scores = Vec::with_capacity(ordered_trials.len());
    for trial in &ordered_trials {
        scores.push(evaluate_trial_v1(trial)?);
    }

    let ids: BTreeSet<u64> = scores.iter().map(|score| score.trial_id).collect();
    if ids.len() != scores.len() {
        return Err(EvaluationError::DuplicateTrialId);
    }

    let first = scores.first().ok_or(EvaluationError::EmptyRun)?;
    if scores
        .iter()
        .any(|score| score.loss_profile_id != first.loss_profile_id)
    {
        return Err(EvaluationError::MixedLossProfiles);
    }

    for score in &scores[1..] {
        if !same_keys(&score.resource_profile, &first.resource_profile) {
            return Err(EvaluationError::MixedResourceAxes);
        }
        for (axis_id, axis) in &first.resource_profile {
            let other = score
                .resource_profile
                .get(axis_id)
                .ok_or(EvaluationError::MixedResourceAxes)?;
            if axis.unit_id != other.unit_id {
                return Err(EvaluationError::MixedResourceAxes);
            }
            if axis.run_budget != other.run_budget {
                return Err(EvaluationError::InconsistentRunBudget(axis_id.clone()));
            }
        }
    }

    let mut total_actual_resource_units = BTreeMap::new();
    let mut run_budget_status = BTreeMap::new();
    for (axis_id, axis) in &first.resource_profile {
        let mut sum = 0_u128;
        let mut complete = true;
        for score in &scores {
            match score.actual_resource_units.get(axis_id).copied().flatten() {
                Some(value) => {
                    sum = sum
                        .checked_add(u128::from(value))
                        .ok_or_else(|| EvaluationError::ResourceArithmeticOverflow(axis_id.clone()))?;
                }
                None => complete = false,
            }
        }

        let total = if complete { Some(sum) } else { None };
        let status = match (total, axis.run_budget) {
            (_, None) => BudgetStatus::NotDeclared,
            (None, Some(_)) => BudgetStatus::Unknown,
            (Some(used), Some(limit)) if used > limit => BudgetStatus::Exceeded,
            (Some(_), Some(_)) => BudgetStatus::WithinBudget,
        };
        total_actual_resource_units.insert(axis_id.clone(), total);
        run_budget_status.insert(axis_id.clone(), status);
    }

    let (run_oracle_total_loss, run_budget_oracle_actions) =
        run_budget_feasible_oracle_v1(&ordered_trials)?;
    let n = scores.len() as f64;
    let mean_realized_loss = scores.iter().map(|s| s.realized_task_loss).sum::<f64>() / n;
    let mean_trial_oracle_loss = scores
        .iter()
        .map(|s| s.best_trial_budget_feasible_counterfactual_loss)
        .sum::<f64>()
        / n;
    let mean_regret_vs_trial_oracle =
        scores.iter().map(|s| s.regret_vs_trial_oracle).sum::<f64>() / n;
    let mean_run_budget_feasible_counterfactual_loss = run_oracle_total_loss / n;
    let mean_regret_vs_run_budget_oracle =
        mean_realized_loss - mean_run_budget_feasible_counterfactual_loss;

    if !mean_realized_loss.is_finite()
        || !mean_trial_oracle_loss.is_finite()
        || !mean_regret_vs_trial_oracle.is_finite()
        || !mean_run_budget_feasible_counterfactual_loss.is_finite()
        || !mean_regret_vs_run_budget_oracle.is_finite()
    {
        return Err(EvaluationError::NonFiniteAggregate);
    }

    Ok(RunSummaryV1 {
        scores,
        mean_realized_loss,
        mean_trial_oracle_loss,
        mean_regret_vs_trial_oracle,
        mean_run_budget_feasible_counterfactual_loss,
        mean_regret_vs_run_budget_oracle,
        run_budget_oracle_actions,
        total_actual_resource_units,
        run_budget_status,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(trial_budget: u64, run_budget: u128) -> ResourceProfileV1 {
        BTreeMap::from([
            (
                "latency".to_string(),
                ResourceAxisProfileV1 {
                    unit_id: "millisecond".to_string(),
                    trial_budget: Some(trial_budget),
                    run_budget: Some(run_budget),
                },
            ),
            (
                "sensor".to_string(),
                ResourceAxisProfileV1 {
                    unit_id: "sample".to_string(),
                    trial_budget: Some(2),
                    run_budget: Some(8),
                },
            ),
        ])
    }

    fn units(latency: u64, sensor: u64) -> BTreeMap<String, u64> {
        BTreeMap::from([
            ("latency".to_string(), latency),
            ("sensor".to_string(), sensor),
        ])
    }

    fn actual(latency: Option<u64>, sensor: Option<u64>) -> BTreeMap<String, Option<u64>> {
        BTreeMap::from([
            ("latency".to_string(), latency),
            ("sensor".to_string(), sensor),
        ])
    }

    fn candidate(
        action: VisionAction,
        feasible: bool,
        loss: f64,
        latency: u64,
        sensor: u64,
    ) -> CandidateOutcomeV1 {
        CandidateOutcomeV1 {
            action,
            feasible,
            counterfactual_task_loss: loss,
            declared_resource_units: units(latency, sensor),
        }
    }

    fn trial(id: u64, chosen: VisionAction, chosen_loss: f64) -> TrialV1 {
        TrialV1 {
            trial_id: id,
            loss_profile_id: "navigation-risk-v1".to_string(),
            resource_profile: profile(20, 100),
            candidates: vec![
                candidate(VisionAction::KeepCheap, true, 10.0, 2, 0),
                candidate(VisionAction::EscalateCurrent, true, chosen_loss, 8, 0),
                candidate(VisionAction::ChangeView, true, 4.0, 5, 1),
                candidate(VisionAction::AbstainOrHandoff, true, 6.0, 1, 0),
                candidate(VisionAction::AcquireNext, false, 1.0, 7, 1),
            ],
            chosen_action: chosen,
            action_executed: true,
            fresh_observation_received: chosen.can_acquire_observation(),
            realized_task_loss: chosen_loss,
            actual_resource_units: actual(
                Some(8),
                Some(if chosen.can_acquire_observation() { 1 } else { 0 }),
            ),
        }
    }

    #[test]
    fn positive_negative_and_neutral_action_value_are_distinguished() {
        let mut helpful = trial(1, VisionAction::EscalateCurrent, 3.0);
        helpful.candidates[1].counterfactual_task_loss = 3.0;
        let helpful_score = evaluate_trial_v1(&helpful).unwrap();
        assert_eq!(helpful_score.counterfactual_action_value_vs_cheap, 7.0);

        let mut harmful = trial(2, VisionAction::EscalateCurrent, 12.0);
        harmful.candidates[1].counterfactual_task_loss = 12.0;
        let harmful_score = evaluate_trial_v1(&harmful).unwrap();
        assert_eq!(harmful_score.counterfactual_action_value_vs_cheap, -2.0);

        let mut neutral = trial(3, VisionAction::EscalateCurrent, 10.0);
        neutral.candidates[1].counterfactual_task_loss = 10.0;
        let neutral_score = evaluate_trial_v1(&neutral).unwrap();
        assert_eq!(neutral_score.counterfactual_action_value_vs_cheap, 0.0);
    }

    #[test]
    fn oracle_respects_each_declared_trial_budget_axis() {
        let mut t = trial(1, VisionAction::ChangeView, 4.0);
        t.candidates.push(candidate(VisionAction::WaitForEvent, true, 0.0, 21, 1));
        let score = evaluate_trial_v1(&t).unwrap();
        assert_eq!(score.best_trial_budget_feasible_counterfactual_loss, 4.0);
    }

    #[test]
    fn infeasible_chosen_action_fails_closed() {
        let mut t = trial(1, VisionAction::AcquireNext, 1.0);
        t.action_executed = false;
        t.fresh_observation_received = false;
        assert_eq!(
            evaluate_trial_v1(&t),
            Err(EvaluationError::ChosenActionInfeasible)
        );
    }

    #[test]
    fn chosen_action_over_budget_fails_closed() {
        let mut t = trial(1, VisionAction::ChangeView, 4.0);
        t.candidates[2].declared_resource_units = units(21, 1);
        assert_eq!(
            evaluate_trial_v1(&t),
            Err(EvaluationError::ChosenActionOverTrialBudget)
        );
    }

    #[test]
    fn unexecuted_action_cannot_claim_a_fresh_observation() {
        let mut t = trial(1, VisionAction::ChangeView, 4.0);
        t.action_executed = false;
        t.fresh_observation_received = true;
        assert_eq!(
            evaluate_trial_v1(&t),
            Err(EvaluationError::FreshObservationWithoutExecutedAcquisition)
        );
    }

    #[test]
    fn current_frame_escalation_cannot_claim_a_new_sensor_observation() {
        let mut t = trial(1, VisionAction::EscalateCurrent, 3.0);
        t.fresh_observation_received = true;
        assert_eq!(
            evaluate_trial_v1(&t),
            Err(EvaluationError::FreshObservationWithoutExecutedAcquisition)
        );
    }

    #[test]
    fn missing_actual_cost_is_unknown_not_zero() {
        let mut t = trial(1, VisionAction::EscalateCurrent, 3.0);
        t.actual_resource_units = actual(None, Some(0));
        let score = evaluate_trial_v1(&t).unwrap();
        assert_eq!(score.actual_resource_units["latency"], None);
        assert_eq!(score.trial_budget_status["latency"], BudgetStatus::Unknown);
    }

    #[test]
    fn duplicate_actions_and_missing_cheap_baseline_are_rejected() {
        let mut duplicate = trial(1, VisionAction::KeepCheap, 10.0);
        duplicate.candidates.push(candidate(VisionAction::KeepCheap, true, 9.0, 1, 0));
        assert_eq!(
            evaluate_trial_v1(&duplicate),
            Err(EvaluationError::DuplicateCandidateAction)
        );

        let mut no_cheap = trial(2, VisionAction::EscalateCurrent, 3.0);
        no_cheap.candidates.retain(|c| c.action != VisionAction::KeepCheap);
        assert_eq!(
            evaluate_trial_v1(&no_cheap),
            Err(EvaluationError::MissingCheapBaseline)
        );
    }

    #[test]
    fn candidate_resource_axes_must_match_declared_profile() {
        let mut t = trial(1, VisionAction::EscalateCurrent, 3.0);
        t.candidates[1].declared_resource_units.remove("sensor");
        assert_eq!(
            evaluate_trial_v1(&t),
            Err(EvaluationError::ResourceAxesMismatch)
        );
    }

    #[test]
    fn run_budget_overrun_is_reported_separately_per_axis() {
        let mut first = trial(1, VisionAction::EscalateCurrent, 3.0);
        let mut second = trial(2, VisionAction::ChangeView, 4.0);
        first.actual_resource_units = actual(Some(70), Some(3));
        second.actual_resource_units = actual(Some(40), Some(3));
        let summary = summarize_run_v1(&[first, second]).unwrap();
        assert_eq!(summary.run_budget_status["latency"], BudgetStatus::Exceeded);
        assert_eq!(summary.run_budget_status["sensor"], BudgetStatus::WithinBudget);
        assert_eq!(summary.total_actual_resource_units["latency"], Some(110));
    }

    #[test]
    fn missing_any_run_charge_makes_axis_total_unknown() {
        let mut first = trial(1, VisionAction::EscalateCurrent, 3.0);
        let second = trial(2, VisionAction::ChangeView, 4.0);
        first.actual_resource_units = actual(None, Some(1));
        let summary = summarize_run_v1(&[first, second]).unwrap();
        assert_eq!(summary.total_actual_resource_units["latency"], None);
        assert_eq!(summary.run_budget_status["latency"], BudgetStatus::Unknown);
    }

    #[test]
    fn run_must_not_mix_loss_profiles_or_duplicate_trial_ids() {
        let first = trial(1, VisionAction::EscalateCurrent, 3.0);
        let mut mixed = trial(2, VisionAction::ChangeView, 4.0);
        mixed.loss_profile_id = "unrelated-loss-v9".to_string();
        assert_eq!(
            summarize_run_v1(&[first.clone(), mixed]),
            Err(EvaluationError::MixedLossProfiles)
        );
        assert_eq!(
            summarize_run_v1(&[first.clone(), first]),
            Err(EvaluationError::DuplicateTrialId)
        );
    }

    #[test]
    fn counterfactual_costs_and_actual_costs_are_not_conflated() {
        let mut t = trial(1, VisionAction::ChangeView, 4.0);
        t.candidates[2].declared_resource_units = units(5, 1);
        t.actual_resource_units = actual(Some(17), Some(2));
        let score = evaluate_trial_v1(&t).unwrap();
        assert_eq!(score.best_trial_budget_feasible_counterfactual_loss, 4.0);
        assert_eq!(score.actual_resource_units["latency"], Some(17));
        assert_eq!(score.trial_budget_status["latency"], BudgetStatus::WithinBudget);
    }

    #[test]
    fn candidate_input_order_does_not_change_score() {
        let a = trial(1, VisionAction::ChangeView, 4.0);
        let mut b = a.clone();
        b.candidates.reverse();
        assert_eq!(evaluate_trial_v1(&a), evaluate_trial_v1(&b));
    }

    #[test]
    fn run_oracle_respects_shared_budget_when_individual_best_actions_do_not_fit() {
        let mut first = trial(1, VisionAction::EscalateCurrent, 1.0);
        let mut second = trial(2, VisionAction::ChangeView, 4.0);

        for t in [&mut first, &mut second] {
            t.resource_profile.get_mut("latency").unwrap().run_budget = Some(20);
            t.candidates[1].declared_resource_units = units(15, 0);
            t.candidates[1].counterfactual_task_loss = 1.0;
            t.candidates[2].declared_resource_units = units(5, 1);
        }
        first.actual_resource_units = actual(Some(15), Some(0));
        second.actual_resource_units = actual(Some(5), Some(1));

        let summary = summarize_run_v1(&[second, first]).unwrap();
        assert_eq!(summary.mean_trial_oracle_loss, 1.0);
        assert_eq!(summary.mean_run_budget_feasible_counterfactual_loss, 2.5);
        assert_eq!(
            summary.run_budget_oracle_actions,
            vec![VisionAction::EscalateCurrent, VisionAction::ChangeView]
        );
        assert_eq!(summary.total_actual_resource_units["latency"], Some(20));
        assert_eq!(summary.run_budget_status["latency"], BudgetStatus::WithinBudget);
        assert_eq!(summary.mean_regret_vs_run_budget_oracle, 0.0);
    }

    #[test]
    fn no_globally_budget_feasible_plan_is_reported_not_approximated() {
        let mut t = trial(1, VisionAction::EscalateCurrent, 3.0);
        t.resource_profile.get_mut("latency").unwrap().run_budget = Some(0);
        assert_eq!(
            summarize_run_v1(&[t]),
            Err(EvaluationError::NoRunBudgetFeasiblePlan)
        );
    }

    #[test]
    fn empty_run_and_inconsistent_run_budgets_fail_closed() {
        assert_eq!(summarize_run_v1(&[]), Err(EvaluationError::EmptyRun));

        let first = trial(1, VisionAction::EscalateCurrent, 3.0);
        let mut second = trial(2, VisionAction::ChangeView, 4.0);
        second.resource_profile.get_mut("latency").unwrap().run_budget = Some(99);
        assert_eq!(
            summarize_run_v1(&[first, second]),
            Err(EvaluationError::InconsistentRunBudget("latency".to_string()))
        );
    }
}
