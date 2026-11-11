// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic closed-loop policy evaluation for a finite socio-ecological
//! service-reserve reference. Candidate policies observe public state, select
//! an action, execute the true transition, observe the new state, and replan.
//! The evaluator-only oracle exhaustively searches the declared finite horizon.
//!
//! This is an analytic benchmark adapter, not a calibrated physical water or
//! energy model. Reserve values are abstract service-capacity units.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::fmt;

const MAX_HORIZON: usize = 10;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivicState {
    pub water_reserve: u32,
    pub energy_reserve: u32,
    pub maintenance_debt: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceDemand {
    pub water: u32,
    pub energy: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ServiceFloor {
    pub water: u32,
    pub energy: u32,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct InfrastructureDynamics {
    pub water_decay_per_tick: u32,
    pub energy_decay_per_tick: u32,
    pub water_repair_gain: u32,
    pub energy_repair_gain: u32,
    pub maintenance_debt_accrual: u32,
    pub water_repair_debt_relief: u32,
    pub energy_repair_debt_relief: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivicScenario {
    pub scenario_id: String,
    pub scenario_digest: String,
    pub true_world_id: String,
    pub true_world_digest: String,
    pub initial_state: CivicState,
    pub true_dynamics: InfrastructureDynamics,
    pub demand: ServiceDemand,
    pub service_floor: ServiceFloor,
    pub service_capacity_limit: u32,
    pub maximum_maintenance_debt: u32,
    pub horizon: usize,
    pub intervention_budget: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivicCandidateModel {
    pub model_id: String,
    pub model_digest: String,
    pub dynamics: InfrastructureDynamics,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivicAction {
    Hold,
    RepairWater,
    RepairEnergy,
}

impl CivicAction {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Hold => "hold",
            Self::RepairWater => "repair_water",
            Self::RepairEnergy => "repair_energy",
        }
    }

    const fn intervention_cost(self) -> usize {
        match self {
            Self::Hold => 0,
            Self::RepairWater | Self::RepairEnergy => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CivicPolicyKind {
    ReactiveReserveBaseline,
    ActionConditionedModel,
}

impl CivicPolicyKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ReactiveReserveBaseline => "reactive_reserve_baseline",
            Self::ActionConditionedModel => "action_conditioned_model",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CivicTransition {
    pub next_state: CivicState,
    pub water_service_delivered: u32,
    pub energy_service_delivered: u32,
    pub water_service_shortfall: u32,
    pub energy_service_shortfall: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CivicStepResult {
    pub tick: usize,
    pub action: CivicAction,
    pub state_before: CivicState,
    pub predicted_water_service: Option<u32>,
    pub predicted_energy_service: Option<u32>,
    pub actual_water_service: u32,
    pub actual_energy_service: u32,
    pub state_after: CivicState,
    pub water_service_shortfall: u32,
    pub energy_service_shortfall: u32,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CivicTrajectoryMetrics {
    pub total_water_service: u64,
    pub total_energy_service: u64,
    pub water_coverage: f64,
    pub energy_coverage: f64,
    pub minimum_service_coverage: f64,
    pub total_interventions: usize,
    pub total_intervention_cost: usize,
    pub service_floor_violation_ticks: usize,
    pub water_shortfall_ticks: usize,
    pub energy_shortfall_ticks: usize,
    pub maximum_water_shortfall: u32,
    pub maximum_energy_shortfall: u32,
    pub terminal_state: CivicState,
    pub terminal_debt_within_limit: bool,
    pub feasible: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CivicOracleResult {
    pub actions: Vec<CivicAction>,
    pub metrics: CivicTrajectoryMetrics,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CivicPolicyEvaluation {
    pub scenario_id: String,
    pub scenario_digest: String,
    pub true_world_id: String,
    pub true_world_digest: String,
    pub policy_kind: CivicPolicyKind,
    pub model_id: Option<String>,
    pub model_digest: Option<String>,
    pub actions: Vec<CivicAction>,
    pub steps: Vec<CivicStepResult>,
    pub metrics: CivicTrajectoryMetrics,
    /// Signed oracle-minus-policy coverage differences, kept per service.
    pub water_service_regret: Option<f64>,
    pub energy_service_regret: Option<f64>,
    pub minimum_service_coverage_regret: Option<f64>,
    /// The oracle is evaluator-only and is never passed to the candidate policy.
    pub oracle: Option<CivicOracleResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ClosedLoopPolicyError {
    MissingIdentity { field: &'static str },
    InvalidHorizon { horizon: usize },
    ZeroServiceDemand,
    InvalidServiceFloor,
    ZeroCapacityLimit,
    InitialReserveExceedsCapacity,
    InterventionBudgetExceedsHorizon,
    ModelRequired,
    ModelUnexpected,
    ArithmeticOverflow,
    OracleSearchSpaceOverflow,
}

impl fmt::Display for ClosedLoopPolicyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ClosedLoopPolicyError {}

fn require_identity(value: &str, field: &'static str) -> Result<(), ClosedLoopPolicyError> {
    if value.trim().is_empty() {
        return Err(ClosedLoopPolicyError::MissingIdentity { field });
    }
    Ok(())
}

impl CivicScenario {
    pub fn validate(&self) -> Result<(), ClosedLoopPolicyError> {
        require_identity(&self.scenario_id, "scenario_id")?;
        require_identity(&self.scenario_digest, "scenario_digest")?;
        require_identity(&self.true_world_id, "true_world_id")?;
        require_identity(&self.true_world_digest, "true_world_digest")?;
        if self.horizon == 0 || self.horizon > MAX_HORIZON {
            return Err(ClosedLoopPolicyError::InvalidHorizon {
                horizon: self.horizon,
            });
        }
        if self.demand.water == 0 || self.demand.energy == 0 {
            return Err(ClosedLoopPolicyError::ZeroServiceDemand);
        }
        if self.service_floor.water == 0
            || self.service_floor.energy == 0
            || self.service_floor.water > self.demand.water
            || self.service_floor.energy > self.demand.energy
        {
            return Err(ClosedLoopPolicyError::InvalidServiceFloor);
        }
        if self.service_capacity_limit == 0 {
            return Err(ClosedLoopPolicyError::ZeroCapacityLimit);
        }
        if self.initial_state.water_reserve > self.service_capacity_limit
            || self.initial_state.energy_reserve > self.service_capacity_limit
        {
            return Err(ClosedLoopPolicyError::InitialReserveExceedsCapacity);
        }
        if self.intervention_budget > self.horizon {
            return Err(ClosedLoopPolicyError::InterventionBudgetExceedsHorizon);
        }
        Ok(())
    }
}

fn transition(
    state: CivicState,
    action: CivicAction,
    dynamics: InfrastructureDynamics,
    demand: ServiceDemand,
    capacity_limit: u32,
) -> Result<CivicTransition, ClosedLoopPolicyError> {
    let water_gain = if action == CivicAction::RepairWater {
        dynamics.water_repair_gain
    } else {
        0
    };
    let energy_gain = if action == CivicAction::RepairEnergy {
        dynamics.energy_repair_gain
    } else {
        0
    };
    let debt_relief = match action {
        CivicAction::RepairWater => dynamics.water_repair_debt_relief,
        CivicAction::RepairEnergy => dynamics.energy_repair_debt_relief,
        CivicAction::Hold => 0,
    };

    let water_before_decay = state
        .water_reserve
        .checked_add(water_gain)
        .ok_or(ClosedLoopPolicyError::ArithmeticOverflow)?
        .min(capacity_limit);
    let energy_before_decay = state
        .energy_reserve
        .checked_add(energy_gain)
        .ok_or(ClosedLoopPolicyError::ArithmeticOverflow)?
        .min(capacity_limit);

    let water_after_decay = water_before_decay.saturating_sub(dynamics.water_decay_per_tick);
    let energy_after_decay = energy_before_decay.saturating_sub(dynamics.energy_decay_per_tick);

    let water_service_delivered = water_after_decay.min(demand.water);
    let energy_service_delivered = energy_after_decay.min(demand.energy);
    let maintenance_debt = state
        .maintenance_debt
        .saturating_sub(debt_relief)
        .checked_add(dynamics.maintenance_debt_accrual)
        .ok_or(ClosedLoopPolicyError::ArithmeticOverflow)?;

    Ok(CivicTransition {
        next_state: CivicState {
            water_reserve: water_after_decay - water_service_delivered,
            energy_reserve: energy_after_decay - energy_service_delivered,
            maintenance_debt,
        },
        water_service_delivered,
        energy_service_delivered,
        water_service_shortfall: demand.water - water_service_delivered,
        energy_service_shortfall: demand.energy - energy_service_delivered,
    })
}

fn metrics_from_trajectory(
    scenario: &CivicScenario,
    steps: &[CivicStepResult],
    terminal_state: CivicState,
    interventions: usize,
) -> CivicTrajectoryMetrics {
    let total_water_service: u64 = steps
        .iter()
        .map(|step| u64::from(step.actual_water_service))
        .sum();
    let total_energy_service: u64 = steps
        .iter()
        .map(|step| u64::from(step.actual_energy_service))
        .sum();
    let total_water_demand = u64::from(scenario.demand.water) * scenario.horizon as u64;
    let total_energy_demand = u64::from(scenario.demand.energy) * scenario.horizon as u64;
    let water_coverage = total_water_service as f64 / total_water_demand as f64;
    let energy_coverage = total_energy_service as f64 / total_energy_demand as f64;
    let minimum_service_coverage = water_coverage.min(energy_coverage);
    let service_floor_violation_ticks = steps
        .iter()
        .filter(|step| {
            step.actual_water_service < scenario.service_floor.water
                || step.actual_energy_service < scenario.service_floor.energy
        })
        .count();
    let water_shortfall_ticks = steps
        .iter()
        .filter(|step| step.actual_water_service < scenario.service_floor.water)
        .count();
    let energy_shortfall_ticks = steps
        .iter()
        .filter(|step| step.actual_energy_service < scenario.service_floor.energy)
        .count();
    let maximum_water_shortfall = steps
        .iter()
        .map(|step| scenario.demand.water - step.actual_water_service)
        .max()
        .unwrap_or(0);
    let maximum_energy_shortfall = steps
        .iter()
        .map(|step| scenario.demand.energy - step.actual_energy_service)
        .max()
        .unwrap_or(0);
    let total_intervention_cost = interventions;
    let terminal_debt_within_limit =
        terminal_state.maintenance_debt <= scenario.maximum_maintenance_debt;
    let feasible = service_floor_violation_ticks == 0
        && terminal_debt_within_limit
        && interventions <= scenario.intervention_budget;

    CivicTrajectoryMetrics {
        total_water_service,
        total_energy_service,
        water_coverage,
        energy_coverage,
        minimum_service_coverage,
        total_interventions: interventions,
        total_intervention_cost,
        service_floor_violation_ticks,
        water_shortfall_ticks,
        energy_shortfall_ticks,
        maximum_water_shortfall,
        maximum_energy_shortfall,
        terminal_state,
        terminal_debt_within_limit,
        feasible,
    }
}

#[derive(Debug, Clone, Copy)]
struct ForecastScore {
    violations: u8,
    total_shortfall: u64,
    minimum_service_coverage: f64,
    total_service_coverage: f64,
    resulting_debt: u32,
    action_cost: usize,
}

fn compare_forecasts(left: ForecastScore, right: ForecastScore) -> Ordering {
    left.violations
        .cmp(&right.violations)
        .then_with(|| left.total_shortfall.cmp(&right.total_shortfall))
        .then_with(|| {
            right
                .minimum_service_coverage
                .total_cmp(&left.minimum_service_coverage)
        })
        .then_with(|| {
            right
                .total_service_coverage
                .total_cmp(&left.total_service_coverage)
        })
        .then_with(|| left.resulting_debt.cmp(&right.resulting_debt))
        .then_with(|| left.action_cost.cmp(&right.action_cost))
}

fn choose_action_conditioned(
    state: CivicState,
    scenario: &CivicScenario,
    model: &CivicCandidateModel,
    remaining_budget: usize,
) -> Result<(CivicAction, CivicTransition), ClosedLoopPolicyError> {
    let actions = [
        CivicAction::Hold,
        CivicAction::RepairWater,
        CivicAction::RepairEnergy,
    ];
    let mut best: Option<(CivicAction, CivicTransition, ForecastScore)> = None;

    for action in actions {
        if action.intervention_cost() > remaining_budget {
            continue;
        }
        let forecast = transition(
            state,
            action,
            model.dynamics,
            scenario.demand,
            scenario.service_capacity_limit,
        )?;
        let water_coverage =
            f64::from(forecast.water_service_delivered) / f64::from(scenario.demand.water);
        let energy_coverage =
            f64::from(forecast.energy_service_delivered) / f64::from(scenario.demand.energy);
        let score = ForecastScore {
            violations: u8::from(
                forecast.water_service_delivered < scenario.service_floor.water,
            ) + u8::from(
                forecast.energy_service_delivered < scenario.service_floor.energy,
            ),
            total_shortfall: u64::from(forecast.water_service_shortfall)
                + u64::from(forecast.energy_service_shortfall),
            minimum_service_coverage: water_coverage.min(energy_coverage),
            total_service_coverage: water_coverage + energy_coverage,
            resulting_debt: forecast.next_state.maintenance_debt,
            action_cost: action.intervention_cost(),
        };
        let replace = best
            .as_ref()
            .is_none_or(|(_, _, existing)| compare_forecasts(score, *existing) == Ordering::Greater);
        if replace {
            best = Some((action, forecast, score));
        }
    }

    let (action, forecast, _) = best.ok_or(ClosedLoopPolicyError::ArithmeticOverflow)?;
    Ok((action, forecast))
}

fn choose_reactive_action(state: CivicState, remaining_budget: usize) -> CivicAction {
    if remaining_budget == 0 {
        CivicAction::Hold
    } else if state.water_reserve <= state.energy_reserve {
        CivicAction::RepairWater
    } else {
        CivicAction::RepairEnergy
    }
}

fn execute_sequence(
    scenario: &CivicScenario,
    actions: &[CivicAction],
) -> Result<(Vec<CivicStepResult>, CivicTrajectoryMetrics), ClosedLoopPolicyError> {
    let mut state = scenario.initial_state;
    let mut steps = Vec::with_capacity(scenario.horizon);
    let mut interventions = 0_usize;

    for (tick, action) in actions.iter().copied().enumerate() {
        interventions = interventions
            .checked_add(action.intervention_cost())
            .ok_or(ClosedLoopPolicyError::ArithmeticOverflow)?;
        if interventions > scenario.intervention_budget {
            return Err(ClosedLoopPolicyError::ArithmeticOverflow);
        }
        let transition = transition(
            state,
            action,
            scenario.true_dynamics,
            scenario.demand,
            scenario.service_capacity_limit,
        )?;
        steps.push(CivicStepResult {
            tick,
            action,
            state_before: state,
            predicted_water_service: None,
            predicted_energy_service: None,
            actual_water_service: transition.water_service_delivered,
            actual_energy_service: transition.energy_service_delivered,
            state_after: transition.next_state,
            water_service_shortfall: transition.water_service_shortfall,
            energy_service_shortfall: transition.energy_service_shortfall,
        });
        state = transition.next_state;
    }

    let metrics = metrics_from_trajectory(scenario, &steps, state, interventions);
    Ok((steps, metrics))
}

fn oracle_better(candidate: &CivicOracleResult, incumbent: &CivicOracleResult) -> bool {
    candidate
        .metrics
        .minimum_service_coverage
        .total_cmp(&incumbent.metrics.minimum_service_coverage)
        .then_with(|| {
            candidate
                .metrics
                .water_coverage
                .total_cmp(&incumbent.metrics.water_coverage)
        })
        .then_with(|| {
            candidate
                .metrics
                .energy_coverage
                .total_cmp(&incumbent.metrics.energy_coverage)
        })
        .then_with(|| {
            incumbent
                .metrics
                .terminal_state
                .maintenance_debt
                .cmp(&candidate.metrics.terminal_state.maintenance_debt)
        })
        .then_with(|| {
            incumbent
                .metrics
                .total_interventions
                .cmp(&candidate.metrics.total_interventions)
        })
        == Ordering::Greater
}

fn evaluate_oracle(
    scenario: &CivicScenario,
) -> Result<Option<CivicOracleResult>, ClosedLoopPolicyError> {
    let sequence_count = 3_usize
        .checked_pow(scenario.horizon as u32)
        .ok_or(ClosedLoopPolicyError::OracleSearchSpaceOverflow)?;
    let mut best: Option<CivicOracleResult> = None;

    for encoded in 0..sequence_count {
        let mut remainder = encoded;
        let mut actions = Vec::with_capacity(scenario.horizon);
        let mut intervention_count = 0_usize;
        for _ in 0..scenario.horizon {
            let action = match remainder % 3 {
                0 => CivicAction::Hold,
                1 => CivicAction::RepairWater,
                _ => CivicAction::RepairEnergy,
            };
            remainder /= 3;
            intervention_count += action.intervention_cost();
            actions.push(action);
        }
        if intervention_count > scenario.intervention_budget {
            continue;
        }

        let (steps, metrics) = execute_sequence(scenario, &actions)?;
        if !metrics.feasible {
            continue;
        }
        let candidate = CivicOracleResult { actions, metrics };
        if best
            .as_ref()
            .is_none_or(|incumbent| oracle_better(&candidate, incumbent))
        {
            best = Some(candidate);
        }
    }

    Ok(best)
}

/// Evaluate a closed-loop policy. The candidate sees only the current public
/// state; the oracle is calculated afterward in this evaluator and is not
/// exposed to the policy. No probabilities are assigned to structural worlds.
pub fn evaluate_closed_loop_policy(
    scenario: &CivicScenario,
    policy_kind: CivicPolicyKind,
    model: Option<&CivicCandidateModel>,
) -> Result<CivicPolicyEvaluation, ClosedLoopPolicyError> {
    scenario.validate()?;
    match (policy_kind, model) {
        (CivicPolicyKind::ActionConditionedModel, None) => {
            return Err(ClosedLoopPolicyError::ModelRequired);
        }
        (CivicPolicyKind::ReactiveReserveBaseline, Some(_)) => {
            return Err(ClosedLoopPolicyError::ModelUnexpected);
        }
        _ => {}
    }
    if let Some(model) = model {
        require_identity(&model.model_id, "model_id")?;
        require_identity(&model.model_digest, "model_digest")?;
    }

    let mut state = scenario.initial_state;
    let mut remaining_budget = scenario.intervention_budget;
    let mut actions = Vec::with_capacity(scenario.horizon);
    let mut steps = Vec::with_capacity(scenario.horizon);
    let mut interventions = 0_usize;

    for tick in 0..scenario.horizon {
        let (action, predicted_transition) = match policy_kind {
            CivicPolicyKind::ReactiveReserveBaseline => {
                (choose_reactive_action(state, remaining_budget), None)
            }
            CivicPolicyKind::ActionConditionedModel => {
                let candidate_model = model.expect("model presence validated above");
                let (action, forecast) =
                    choose_action_conditioned(state, scenario, candidate_model, remaining_budget)?;
                (action, Some(forecast))
            }
        };

        interventions = interventions
            .checked_add(action.intervention_cost())
            .ok_or(ClosedLoopPolicyError::ArithmeticOverflow)?;
        remaining_budget = remaining_budget.saturating_sub(action.intervention_cost());

        let actual = transition(
            state,
            action,
            scenario.true_dynamics,
            scenario.demand,
            scenario.service_capacity_limit,
        )?;
        let (predicted_water_service, predicted_energy_service) = predicted_transition
            .map(|forecast| {
                (
                    Some(forecast.water_service_delivered),
                    Some(forecast.energy_service_delivered),
                )
            })
            .unwrap_or((None, None));

        steps.push(CivicStepResult {
            tick,
            action,
            state_before: state,
            predicted_water_service,
            predicted_energy_service,
            actual_water_service: actual.water_service_delivered,
            actual_energy_service: actual.energy_service_delivered,
            state_after: actual.next_state,
            water_service_shortfall: actual.water_service_shortfall,
            energy_service_shortfall: actual.energy_service_shortfall,
        });
        actions.push(action);
        state = actual.next_state;
    }

    let metrics = metrics_from_trajectory(scenario, &steps, state, interventions);
    let oracle = evaluate_oracle(scenario)?;
    let (water_service_regret, energy_service_regret, minimum_service_coverage_regret) =
        if let Some(reference) = &oracle {
            (
                Some(reference.metrics.water_coverage - metrics.water_coverage),
                Some(reference.metrics.energy_coverage - metrics.energy_coverage),
                Some(
                    reference.metrics.minimum_service_coverage
                        - metrics.minimum_service_coverage,
                ),
            )
        } else {
            (None, None, None)
        };

    Ok(CivicPolicyEvaluation {
        scenario_id: scenario.scenario_id.clone(),
        scenario_digest: scenario.scenario_digest.clone(),
        true_world_id: scenario.true_world_id.clone(),
        true_world_digest: scenario.true_world_digest.clone(),
        policy_kind,
        model_id: model.map(|value| value.model_id.clone()),
        model_digest: model.map(|value| value.model_digest.clone()),
        actions,
        steps,
        metrics,
        water_service_regret,
        energy_service_regret,
        minimum_service_coverage_regret,
        oracle,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy_stress_scenario() -> CivicScenario {
        CivicScenario {
            scenario_id: "energy-stress".to_string(),
            scenario_digest: "sha256-scenario-energy-stress".to_string(),
            true_world_id: "world-energy-degradation".to_string(),
            true_world_digest: "sha256-world-energy-degradation".to_string(),
            initial_state: CivicState {
                water_reserve: 6,
                energy_reserve: 6,
                maintenance_debt: 0,
            },
            true_dynamics: InfrastructureDynamics {
                water_decay_per_tick: 0,
                energy_decay_per_tick: 4,
                water_repair_gain: 8,
                energy_repair_gain: 8,
                maintenance_debt_accrual: 1,
                water_repair_debt_relief: 1,
                energy_repair_debt_relief: 1,
            },
            demand: ServiceDemand {
                water: 3,
                energy: 3,
            },
            service_floor: ServiceFloor {
                water: 3,
                energy: 3,
            },
            service_capacity_limit: 20,
            maximum_maintenance_debt: 2,
            horizon: 2,
            intervention_budget: 2,
        }
    }

    fn correct_energy_model() -> CivicCandidateModel {
        CivicCandidateModel {
            model_id: "energy-dynamics-model".to_string(),
            model_digest: "sha256-model-energy-dynamics".to_string(),
            dynamics: energy_stress_scenario().true_dynamics,
        }
    }

    fn wrong_water_model() -> CivicCandidateModel {
        CivicCandidateModel {
            model_id: "water-dynamics-model".to_string(),
            model_digest: "sha256-model-water-dynamics".to_string(),
            dynamics: InfrastructureDynamics {
                water_decay_per_tick: 4,
                energy_decay_per_tick: 0,
                water_repair_gain: 8,
                energy_repair_gain: 8,
                maintenance_debt_accrual: 1,
                water_repair_debt_relief: 1,
                energy_repair_debt_relief: 1,
            },
        }
    }

    #[test]
    fn action_conditioned_model_selects_intervention_that_preserves_both_service_floors() {
        let scenario = energy_stress_scenario();
        let baseline = evaluate_closed_loop_policy(
            &scenario,
            CivicPolicyKind::ReactiveReserveBaseline,
            None,
        )
        .expect("baseline evaluation");
        let model = evaluate_closed_loop_policy(
            &scenario,
            CivicPolicyKind::ActionConditionedModel,
            Some(&correct_energy_model()),
        )
        .expect("model evaluation");

        assert_eq!(baseline.actions[0], CivicAction::RepairWater);
        assert_eq!(model.actions[0], CivicAction::RepairEnergy);
        assert_eq!(baseline.metrics.service_floor_violation_ticks, 1);
        assert_eq!(model.metrics.service_floor_violation_ticks, 0);
        assert!(model.metrics.feasible);
        assert!(model.minimum_service_coverage_regret.unwrap().abs() < 1e-12);
        assert_eq!(model.oracle.as_ref().unwrap().actions[0], CivicAction::RepairEnergy);
    }

    #[test]
    fn misspecified_action_response_increases_regret_and_service_shortfall() {
        let scenario = energy_stress_scenario();
        let result = evaluate_closed_loop_policy(
            &scenario,
            CivicPolicyKind::ActionConditionedModel,
            Some(&wrong_water_model()),
        )
        .expect("misspecified-model evaluation");
        assert_eq!(result.actions[0], CivicAction::RepairWater);
        assert!(result.metrics.service_floor_violation_ticks > 0);
        assert!(result.minimum_service_coverage_regret.unwrap() > 0.0);
    }

    #[test]
    fn oracle_is_evaluator_only_and_enumerates_feasible_sequences() {
        let scenario = energy_stress_scenario();
        let result = evaluate_closed_loop_policy(
            &scenario,
            CivicPolicyKind::ReactiveReserveBaseline,
            None,
        )
        .expect("baseline evaluation");
        let oracle = result.oracle.expect("feasible oracle policy");
        assert!(oracle.metrics.feasible);
        assert_eq!(oracle.actions.len(), scenario.horizon);
        assert_eq!(oracle.actions[0], CivicAction::RepairEnergy);
        assert_eq!(oracle.metrics.service_floor_violation_ticks, 0);
    }

    #[test]
    fn budget_exhaustion_forces_hold_without_silent_reuse_of_repairs() {
        let mut scenario = energy_stress_scenario();
        scenario.intervention_budget = 1;
        let result = evaluate_closed_loop_policy(
            &scenario,
            CivicPolicyKind::ActionConditionedModel,
            Some(&correct_energy_model()),
        )
        .expect("one-intervention evaluation");
        assert_eq!(result.metrics.total_interventions, 1);
        assert_eq!(result.actions[1], CivicAction::Hold);
    }

    #[test]
    fn rejects_invalid_scenario_and_missing_or_unexpected_model() {
        let mut scenario = energy_stress_scenario();
        scenario.horizon = 0;
        assert!(matches!(
            evaluate_closed_loop_policy(
                &scenario,
                CivicPolicyKind::ReactiveReserveBaseline,
                None
            ),
            Err(ClosedLoopPolicyError::InvalidHorizon { .. })
        ));

        let scenario = energy_stress_scenario();
        assert_eq!(
            evaluate_closed_loop_policy(
                &scenario,
                CivicPolicyKind::ActionConditionedModel,
                None
            )
            .unwrap_err(),
            ClosedLoopPolicyError::ModelRequired
        );
        assert_eq!(
            evaluate_closed_loop_policy(
                &scenario,
                CivicPolicyKind::ReactiveReserveBaseline,
                Some(&correct_energy_model())
            )
            .unwrap_err(),
            ClosedLoopPolicyError::ModelUnexpected
        );
    }

    #[test]
    fn common_policy_interface_preserves_separate_water_and_energy_metrics() {
        let scenario = energy_stress_scenario();
        let result = evaluate_closed_loop_policy(
            &scenario,
            CivicPolicyKind::ActionConditionedModel,
            Some(&correct_energy_model()),
        )
        .expect("valid evaluation");
        assert_eq!(result.metrics.total_water_service, 6);
        assert_eq!(result.metrics.total_energy_service, 6);
        assert_eq!(result.metrics.water_coverage, 1.0);
        assert_eq!(result.metrics.energy_coverage, 1.0);
        assert_eq!(result.metrics.minimum_service_coverage, 1.0);
    }
}
