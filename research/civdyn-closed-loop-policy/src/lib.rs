// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-horizon closed-loop policy-value benchmark.
//!
//! Candidate policies receive only the current observable state, action set,
//! and (for model-based arms) a declared candidate transition model. The true
//! world and evaluator-only oracle remain outside the policy interface.

#![forbid(unsafe_code)]

use core::fmt;

pub const HORIZON: usize = 4;
pub const SERVICE_FLOOR: i32 = 4;
pub const TERMINAL_RESERVE_FLOOR: i32 = 10;
pub const MAX_MAINTENANCE_DEBT: i32 = 15;
pub const INITIAL_STATE: State = State {
    water_service: 8,
    energy_service: 8,
    water_maintenance_debt: 0,
    energy_maintenance_debt: 0,
    water_reserve: 20,
    energy_reserve: 20,
    maintenance_credits: 2,
};

pub const WATER_DEGRADATION_WORLD: WorldSpec = WorldSpec {
    id: "water_degradation",
    digest: "sha256-civdyn-true-world-water-v1",
    params: TransitionParams {
        water_decay: 2,
        energy_decay: 1,
        water_repair: 4,
        energy_repair: 2,
    },
};

pub const ENERGY_DEGRADATION_WORLD: WorldSpec = WorldSpec {
    id: "energy_degradation",
    digest: "sha256-civdyn-true-world-energy-v1",
    params: TransitionParams {
        water_decay: 1,
        energy_decay: 2,
        water_repair: 2,
        energy_repair: 4,
    },
};

pub const WATER_CANDIDATE_MODEL: CandidateModel = CandidateModel {
    id: "water_mechanism_model",
    digest: "sha256-civdyn-candidate-model-water-v1",
    params: WATER_DEGRADATION_WORLD.params,
};

pub const ENERGY_CANDIDATE_MODEL: CandidateModel = CandidateModel {
    id: "energy_mechanism_model",
    digest: "sha256-civdyn-candidate-model-energy-v1",
    params: ENERGY_DEGRADATION_WORLD.params,
};

pub const SCENARIO_MANIFEST_DIGEST: &str = "sha256-civdyn-closed-loop-policy-scenario-v1";
pub const SPLIT_MANIFEST_DIGEST: &str = "sha256-civdyn-closed-loop-synthetic-holdout-v1";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct State {
    pub water_service: i32,
    pub energy_service: i32,
    pub water_maintenance_debt: i32,
    pub energy_maintenance_debt: i32,
    pub water_reserve: i32,
    pub energy_reserve: i32,
    pub maintenance_credits: i32,
}

impl State {
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if !(0..=10).contains(&self.water_service)
            || !(0..=10).contains(&self.energy_service)
            || self.water_maintenance_debt < 0
            || self.energy_maintenance_debt < 0
            || self.maintenance_credits < 0
        {
            return Err(BenchmarkError::InvalidState);
        }
        Ok(())
    }

    pub fn is_service_feasible(&self) -> bool {
        self.water_service >= SERVICE_FLOOR && self.energy_service >= SERVICE_FLOOR
    }

    pub fn is_transition_feasible(&self) -> bool {
        self.is_service_feasible()
            && self.water_reserve >= 0
            && self.energy_reserve >= 0
            && self.water_maintenance_debt <= MAX_MAINTENANCE_DEBT
            && self.energy_maintenance_debt <= MAX_MAINTENANCE_DEBT
    }

    pub fn is_terminal_feasible(&self) -> bool {
        self.is_transition_feasible()
            && self.water_reserve >= TERMINAL_RESERVE_FLOOR
            && self.energy_reserve >= TERMINAL_RESERVE_FLOOR
    }

    pub fn service_points(&self) -> i64 {
        i64::from(self.water_service + self.energy_service)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TransitionParams {
    pub water_decay: i32,
    pub energy_decay: i32,
    pub water_repair: i32,
    pub energy_repair: i32,
}

impl TransitionParams {
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        let values = [
            self.water_decay,
            self.energy_decay,
            self.water_repair,
            self.energy_repair,
        ];
        if values.iter().any(|value| !(1..=10).contains(value)) {
            return Err(BenchmarkError::InvalidTransitionParameters);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorldSpec {
    pub id: &'static str,
    pub digest: &'static str,
    pub params: TransitionParams,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CandidateModel {
    pub id: &'static str,
    pub digest: &'static str,
    pub params: TransitionParams,
}

impl CandidateModel {
    pub fn validate(&self) -> Result<(), BenchmarkError> {
        if self.id.trim().is_empty() || self.digest.trim().is_empty() {
            return Err(BenchmarkError::MissingModelIdentity);
        }
        self.params.validate()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Action {
    NoOp,
    RepairWater,
    RepairEnergy,
}

impl Action {
    pub const ALL: [Self; 3] = [Self::NoOp, Self::RepairWater, Self::RepairEnergy];

    pub const fn id(self) -> &'static str {
        match self {
            Self::NoOp => "noop",
            Self::RepairWater => "repair_water",
            Self::RepairEnergy => "repair_energy",
        }
    }

    pub fn is_available(self, state: &State) -> bool {
        match self {
            Self::NoOp => true,
            Self::RepairWater | Self::RepairEnergy => {
                state.maintenance_credits > 0 && state.energy_reserve >= 2
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkError {
    InvalidState,
    InvalidTransitionParameters,
    MissingModelIdentity,
    InvalidAction,
    ZeroHorizon,
    NoFeasiblePlan,
    ArithmeticOverflow,
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for BenchmarkError {}

fn clamp_service(value: i32) -> i32 {
    value.clamp(0, 10)
}

/// Apply one declared action and then one physical transition.
///
/// Repairs consume two energy-reserve units and one maintenance credit.
/// Each tick consumes two water-reserve units and one energy-reserve unit.
/// Maintenance debt affects future degradation and is reduced by a repair.
pub fn advance(
    state: &State,
    action: Action,
    params: TransitionParams,
) -> Result<State, BenchmarkError> {
    state.validate()?;
    params.validate()?;
    if !action.is_available(state) {
        return Err(BenchmarkError::InvalidAction);
    }

    let mut next = *state;
    match action {
        Action::NoOp => {}
        Action::RepairWater => {
            next.water_service = clamp_service(
                next.water_service
                    .checked_add(params.water_repair)
                    .ok_or(BenchmarkError::ArithmeticOverflow)?,
            );
            next.water_maintenance_debt = (next.water_maintenance_debt - 3).max(0);
            next.maintenance_credits -= 1;
            next.energy_reserve = next.energy_reserve.checked_sub(2).ok_or(BenchmarkError::ArithmeticOverflow)?;
        }
        Action::RepairEnergy => {
            next.energy_service = clamp_service(
                next.energy_service
                    .checked_add(params.energy_repair)
                    .ok_or(BenchmarkError::ArithmeticOverflow)?,
            );
            next.energy_maintenance_debt = (next.energy_maintenance_debt - 3).max(0);
            next.maintenance_credits -= 1;
            next.energy_reserve -= 2;
        }
    }

    let water_debt_decay = next.water_maintenance_debt / 4;
    let energy_debt_decay = next.energy_maintenance_debt / 4;
    next.water_service = clamp_service(
        next.water_service
            - params.water_decay
            - water_debt_decay,
    );
    next.energy_service = clamp_service(
        next.energy_service
            - params.energy_decay
            - energy_debt_decay,
    );

    next.water_maintenance_debt = next
        .water_maintenance_debt
        .checked_add(params.water_decay)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    next.energy_maintenance_debt = next
        .energy_maintenance_debt
        .checked_add(params.energy_decay)
        .ok_or(BenchmarkError::ArithmeticOverflow)?;
    next.water_reserve = next.water_reserve.checked_sub(2).ok_or(BenchmarkError::ArithmeticOverflow)?;
    next.energy_reserve = next.energy_reserve.checked_sub(1).ok_or(BenchmarkError::ArithmeticOverflow)?;

    next.validate()?;
    Ok(next)
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Plan {
    pub actions: Vec<Action>,
    pub service_points: i64,
}

fn terminal_allowed(state: &State) -> bool {
    state.is_terminal_feasible()
}

fn search_plans(
    state: &State,
    params: TransitionParams,
    steps_remaining: usize,
    actions: &mut Vec<Action>,
    service_points: i64,
    best: &mut Option<Plan>,
) -> Result<(), BenchmarkError> {
    if steps_remaining == 0 {
        if terminal_allowed(state)
            && best
                .as_ref()
                .is_none_or(|known| service_points > known.service_points)
        {
            *best = Some(Plan {
                actions: actions.clone(),
                service_points,
            });
        }
        return Ok(());
    }

    for action in Action::ALL {
        if !action.is_available(state) {
            continue;
        }
        let next = advance(state, action, params)?;
        if !next.is_transition_feasible() {
            continue;
        }
        actions.push(action);
        search_plans(
            &next,
            params,
            steps_remaining - 1,
            actions,
            service_points
                .checked_add(next.service_points())
                .ok_or(BenchmarkError::ArithmeticOverflow)?,
            best,
        )?;
        actions.pop();
    }
    Ok(())
}

/// Exhaustive evaluator for the small, frozen finite-horizon reference.
/// It is used by candidate-model planning and separately by the evaluator
/// with the true world. The policy API never receives the evaluator result.
pub fn best_feasible_plan(
    state: &State,
    params: TransitionParams,
    steps: usize,
) -> Result<Plan, BenchmarkError> {
    state.validate()?;
    params.validate()?;
    if steps == 0 {
        return Err(BenchmarkError::ZeroHorizon);
    }
    let mut best = None;
    search_plans(
        state,
        params,
        steps,
        &mut Vec::with_capacity(steps),
        0,
        &mut best,
    )?;
    best.ok_or(BenchmarkError::NoFeasiblePlan)
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PolicyKind {
    ReactiveNoModel,
    FixedOpenLoop,
    ActionConditionedModel,
    EvaluatorOnlyOracle,
}

impl PolicyKind {
    pub const fn id(self) -> &'static str {
        match self {
            Self::ReactiveNoModel => "reactive_no_model",
            Self::FixedOpenLoop => "fixed_open_loop",
            Self::ActionConditionedModel => "action_conditioned_model",
            Self::EvaluatorOnlyOracle => "evaluator_only_oracle",
        }
    }
}

fn choose_reactive_action(state: &State) -> Action {
    if state.maintenance_credits <= 0 || state.energy_reserve < 2 {
        return Action::NoOp;
    }
    if state.water_service < state.energy_service {
        Action::RepairWater
    } else if state.energy_service < state.water_service {
        Action::RepairEnergy
    } else if state.water_maintenance_debt >= state.energy_maintenance_debt {
        Action::RepairWater
    } else {
        Action::RepairEnergy
    }
}

fn fixed_open_loop_action(tick: usize, state: &State) -> Action {
    if state.maintenance_credits <= 0 || state.energy_reserve < 2 {
        return Action::NoOp;
    }
    match tick {
        0 | 1 => Action::RepairWater,
        _ => Action::NoOp,
    }
}

fn choose_model_action(
    state: &State,
    model: CandidateModel,
    steps_remaining: usize,
) -> Option<Action> {
    best_feasible_plan(state, model.params, steps_remaining)
        .ok()
        .and_then(|plan| plan.actions.first().copied())
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct ForecastErrors {
    pub water_service_abs: f64,
    pub energy_service_abs: f64,
    pub water_debt_abs: f64,
    pub energy_debt_abs: f64,
    pub water_reserve_abs: f64,
    pub energy_reserve_abs: f64,
    pub observations: usize,
}

impl ForecastErrors {
    fn add(&mut self, predicted: &State, actual: &State) {
        self.water_service_abs +=
            f64::from((predicted.water_service - actual.water_service).abs());
        self.energy_service_abs +=
            f64::from((predicted.energy_service - actual.energy_service).abs());
        self.water_debt_abs += f64::from(
            (predicted.water_maintenance_debt - actual.water_maintenance_debt).abs(),
        );
        self.energy_debt_abs += f64::from(
            (predicted.energy_maintenance_debt - actual.energy_maintenance_debt).abs(),
        );
        self.water_reserve_abs +=
            f64::from((predicted.water_reserve - actual.water_reserve).abs());
        self.energy_reserve_abs +=
            f64::from((predicted.energy_reserve - actual.energy_reserve).abs());
        self.observations += 1;
    }

    pub fn mean_absolute_errors(self) -> Option<[f64; 6]> {
        (self.observations > 0).then(|| {
            let divisor = self.observations as f64;
            [
                self.water_service_abs / divisor,
                self.energy_service_abs / divisor,
                self.water_debt_abs / divisor,
                self.energy_debt_abs / divisor,
                self.water_reserve_abs / divisor,
                self.energy_reserve_abs / divisor,
            ]
        })
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyOutcome {
    pub world_id: &'static str,
    pub world_digest: &'static str,
    pub policy: PolicyKind,
    pub candidate_model_id: Option<&'static str>,
    pub candidate_model_digest: Option<&'static str>,
    pub evaluator_oracle_access: bool,
    pub actions: Vec<Action>,
    pub cumulative_service_points: i64,
    pub oracle_service_points: i64,
    pub raw_service_regret: i64,
    pub minimum_water_service: i32,
    pub minimum_energy_service: i32,
    pub water_floor_violation_ticks: usize,
    pub energy_floor_violation_ticks: usize,
    pub worst_water_shortfall: i32,
    pub worst_energy_shortfall: i32,
    pub ending_water_reserve: i32,
    pub ending_energy_reserve: i32,
    pub ending_water_debt: i32,
    pub ending_energy_debt: i32,
    pub ending_maintenance_credits: i32,
    pub terminal_feasible: bool,
    pub action_ranking_matches: usize,
    pub decision_count: usize,
    pub replan_count: usize,
    pub abstention_ticks: usize,
    pub forecast_errors: Option<[f64; 6]>,
    pub scenario_manifest_digest: &'static str,
    pub split_manifest_digest: &'static str,
}

fn oracle_action(state: &State, true_world: WorldSpec, steps_remaining: usize) -> Option<Action> {
    best_feasible_plan(state, true_world.params, steps_remaining)
        .ok()
        .and_then(|plan| plan.actions.first().copied())
}

pub fn run_policy(
    true_world: WorldSpec,
    policy: PolicyKind,
    candidate_model: Option<CandidateModel>,
) -> Result<PolicyOutcome, BenchmarkError> {
    true_world.params.validate()?;
    if policy == PolicyKind::ActionConditionedModel {
        candidate_model
            .ok_or(BenchmarkError::MissingModelIdentity)?
            .validate()?;
    }

    let mut state = INITIAL_STATE;
    let mut cumulative_service_points = 0_i64;
    let mut actions = Vec::with_capacity(HORIZON);
    let mut minimum_water_service = state.water_service;
    let mut minimum_energy_service = state.energy_service;
    let mut water_floor_violation_ticks = 0_usize;
    let mut energy_floor_violation_ticks = 0_usize;
    let mut worst_water_shortfall = 0_i32;
    let mut worst_energy_shortfall = 0_i32;
    let mut action_ranking_matches = 0_usize;
    let mut forecast_errors = ForecastErrors::default();
    let mut abstention_ticks = 0_usize;

    for tick in 0..HORIZON {
        let steps_remaining = HORIZON - tick;
        // Evaluator-only reference. It is never passed into a policy chooser.
        let reference_action = oracle_action(&state, true_world, steps_remaining);
        let selected_action = match policy {
            PolicyKind::ReactiveNoModel => choose_reactive_action(&state),
            PolicyKind::FixedOpenLoop => fixed_open_loop_action(tick, &state),
            PolicyKind::ActionConditionedModel => {
                let model = candidate_model.ok_or(BenchmarkError::MissingModelIdentity)?;
                match choose_model_action(&state, model, steps_remaining) {
                    Some(action) => action,
                    None => {
                        abstention_ticks += 1;
                        Action::NoOp
                    }
                }
            }
            PolicyKind::EvaluatorOnlyOracle => reference_action.unwrap_or(Action::NoOp),
        };
        let action = if selected_action.is_available(&state) {
            selected_action
        } else {
            Action::NoOp
        };

        if reference_action == Some(action) {
            action_ranking_matches += 1;
        }

        if let Some(model) = candidate_model.filter(|_| {
            policy == PolicyKind::ActionConditionedModel
        }) {
            if let Ok(predicted) = advance(&state, action, model.params) {
                let actual = advance(&state, action, true_world.params)?;
                forecast_errors.add(&predicted, &actual);
            }
        }

        state = advance(&state, action, true_world.params)?;
        cumulative_service_points = cumulative_service_points
            .checked_add(state.service_points())
            .ok_or(BenchmarkError::ArithmeticOverflow)?;
        actions.push(action);
        minimum_water_service = minimum_water_service.min(state.water_service);
        minimum_energy_service = minimum_energy_service.min(state.energy_service);
        if state.water_service < SERVICE_FLOOR {
            water_floor_violation_ticks += 1;
            worst_water_shortfall =
                worst_water_shortfall.max(SERVICE_FLOOR - state.water_service);
        }
        if state.energy_service < SERVICE_FLOOR {
            energy_floor_violation_ticks += 1;
            worst_energy_shortfall =
                worst_energy_shortfall.max(SERVICE_FLOOR - state.energy_service);
        }
    }

    let oracle = best_feasible_plan(&INITIAL_STATE, true_world.params, HORIZON)?;
    Ok(PolicyOutcome {
        world_id: true_world.id,
        world_digest: true_world.digest,
        policy,
        candidate_model_id: candidate_model.map(|model| model.id),
        candidate_model_digest: candidate_model.map(|model| model.digest),
        evaluator_oracle_access: policy == PolicyKind::EvaluatorOnlyOracle,
        actions,
        cumulative_service_points,
        oracle_service_points: oracle.service_points,
        raw_service_regret: oracle.service_points - cumulative_service_points,
        minimum_water_service,
        minimum_energy_service,
        water_floor_violation_ticks,
        energy_floor_violation_ticks,
        worst_water_shortfall,
        worst_energy_shortfall,
        ending_water_reserve: state.water_reserve,
        ending_energy_reserve: state.energy_reserve,
        ending_water_debt: state.water_maintenance_debt,
        ending_energy_debt: state.energy_maintenance_debt,
        ending_maintenance_credits: state.maintenance_credits,
        terminal_feasible: state.is_terminal_feasible()
            && water_floor_violation_ticks == 0
            && energy_floor_violation_ticks == 0,
        action_ranking_matches,
        decision_count: HORIZON,
        replan_count: if policy == PolicyKind::ActionConditionedModel {
            HORIZON
        } else {
            0
        },
        abstention_ticks,
        forecast_errors: forecast_errors.mean_absolute_errors(),
        scenario_manifest_digest: SCENARIO_MANIFEST_DIGEST,
        split_manifest_digest: SPLIT_MANIFEST_DIGEST,
    })
}

pub fn run_reference_portfolio() -> Result<Vec<PolicyOutcome>, BenchmarkError> {
    let mut outcomes = Vec::with_capacity(10);
    for world in [WATER_DEGRADATION_WORLD, ENERGY_DEGRADATION_WORLD] {
        let (correct_model, misspecified_model) = if world.id == WATER_DEGRADATION_WORLD.id {
            (WATER_CANDIDATE_MODEL, ENERGY_CANDIDATE_MODEL)
        } else {
            (ENERGY_CANDIDATE_MODEL, WATER_CANDIDATE_MODEL)
        };

        outcomes.push(run_policy(world, PolicyKind::ReactiveNoModel, None)?);
        outcomes.push(run_policy(world, PolicyKind::FixedOpenLoop, None)?);
        outcomes.push(run_policy(
            world,
            PolicyKind::ActionConditionedModel,
            Some(correct_model),
        )?);
        outcomes.push(run_policy(
            world,
            PolicyKind::ActionConditionedModel,
            Some(misspecified_model),
        )?);
        outcomes.push(run_policy(world, PolicyKind::EvaluatorOnlyOracle, None)?);
    }
    Ok(outcomes)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actions(outcome: &PolicyOutcome) -> Vec<&'static str> {
        outcome.actions.iter().map(|action| action.id()).collect()
    }

    #[test]
    fn water_world_oracle_has_exact_known_answer() {
        let plan =
            best_feasible_plan(&INITIAL_STATE, WATER_DEGRADATION_WORLD.params, HORIZON).unwrap();
        assert_eq!(plan.service_points, 50);
        assert_eq!(
            plan.actions,
            vec![
                Action::NoOp,
                Action::RepairWater,
                Action::NoOp,
                Action::RepairWater
            ]
        );
    }

    #[test]
    fn energy_world_oracle_has_exact_known_answer() {
        let plan =
            best_feasible_plan(&INITIAL_STATE, ENERGY_DEGRADATION_WORLD.params, HORIZON).unwrap();
        assert_eq!(plan.service_points, 50);
        assert_eq!(
            plan.actions,
            vec![
                Action::NoOp,
                Action::RepairEnergy,
                Action::NoOp,
                Action::RepairEnergy
            ]
        );
    }

    #[test]
    fn correct_action_conditioned_models_match_the_evaluator_oracle() {
        for (world, model) in [
            (WATER_DEGRADATION_WORLD, WATER_CANDIDATE_MODEL),
            (ENERGY_DEGRADATION_WORLD, ENERGY_CANDIDATE_MODEL),
        ] {
            let model_outcome =
                run_policy(world, PolicyKind::ActionConditionedModel, Some(model)).unwrap();
            let oracle_outcome =
                run_policy(world, PolicyKind::EvaluatorOnlyOracle, None).unwrap();
            assert!(model_outcome.terminal_feasible);
            assert_eq!(model_outcome.cumulative_service_points, 50);
            assert_eq!(model_outcome.actions, oracle_outcome.actions);
            assert_eq!(model_outcome.action_ranking_matches, HORIZON);
            assert_eq!(model_outcome.forecast_errors, Some([0.0; 6]));
            assert!(!model_outcome.evaluator_oracle_access);
        }
    }

    #[test]
    fn reactive_policy_cannot_substitute_current_weakest_service_for_lookahead() {
        let water =
            run_policy(WATER_DEGRADATION_WORLD, PolicyKind::ReactiveNoModel, None).unwrap();
        let energy =
            run_policy(ENERGY_DEGRADATION_WORLD, PolicyKind::ReactiveNoModel, None).unwrap();
        assert!(!water.terminal_feasible);
        assert!(water.water_floor_violation_ticks > 0);
        assert!(!energy.terminal_feasible);
        assert!(energy.energy_floor_violation_ticks > 0);
    }

    #[test]
    fn misspecified_action_model_is_confidently_wrong_on_the_other_world() {
        let water =
            run_policy(WATER_DEGRADATION_WORLD, PolicyKind::ActionConditionedModel, Some(ENERGY_CANDIDATE_MODEL)).unwrap();
        let energy =
            run_policy(ENERGY_DEGRADATION_WORLD, PolicyKind::ActionConditionedModel, Some(WATER_CANDIDATE_MODEL)).unwrap();
        assert!(!water.terminal_feasible);
        assert!(!energy.terminal_feasible);
        assert!(water.worst_water_shortfall > 0);
        assert!(energy.worst_energy_shortfall > 0);
        assert!(water.forecast_errors.unwrap().iter().any(|value| *value > 0.0));
        assert!(energy.forecast_errors.unwrap().iter().any(|value| *value > 0.0));
    }

    #[test]
    fn fixed_open_loop_plan_fails_under_both_structural_worlds() {
        for world in [WATER_DEGRADATION_WORLD, ENERGY_DEGRADATION_WORLD] {
            let outcome = run_policy(world, PolicyKind::FixedOpenLoop, None).unwrap();
            assert!(!outcome.terminal_feasible);
            assert!(outcome.water_floor_violation_ticks + outcome.energy_floor_violation_ticks > 0);
        }
    }

    #[test]
    fn service_floor_is_not_averaged_away_by_raw_utility() {
        let outcomes = run_reference_portfolio().unwrap();
        for world_id in [WATER_DEGRADATION_WORLD.id, ENERGY_DEGRADATION_WORLD.id] {
            let rows: Vec<&PolicyOutcome> =
                outcomes.iter().filter(|outcome| outcome.world_id == world_id).collect();
            let oracle = rows
                .iter()
                .find(|outcome| outcome.policy == PolicyKind::EvaluatorOnlyOracle)
                .unwrap();
            let reactive = rows
                .iter()
                .find(|outcome| outcome.policy == PolicyKind::ReactiveNoModel)
                .unwrap();
            assert!(oracle.terminal_feasible);
            assert!(!reactive.terminal_feasible);
            // Even if a policy has a larger raw service sum, its hard-floor
            // failure keeps it out of the admissible decision set.
        }
    }

    #[test]
    fn candidate_policy_interface_does_not_accept_the_true_world() {
        // Candidate action choice receives only the observation and candidate
        // model. The true world is confined to run_policy's evaluator harness.
        let selected = choose_model_action(
            &INITIAL_STATE,
            WATER_CANDIDATE_MODEL,
            HORIZON,
        )
        .unwrap();
        assert_eq!(selected, Action::NoOp);
    }

    #[test]
    fn invalid_parameters_and_model_identities_fail_closed() {
        assert_eq!(
            TransitionParams {
                water_decay: 0,
                energy_decay: 1,
                water_repair: 1,
                energy_repair: 1
            }
            .validate(),
            Err(BenchmarkError::InvalidTransitionParameters)
        );
        assert_eq!(
            CandidateModel {
                id: "",
                digest: "",
                params: WATER_DEGRADATION_WORLD.params
            }
            .validate(),
            Err(BenchmarkError::MissingModelIdentity)
        );
    }

    #[test]
    fn transition_costs_and_stock_accounting_are_exact() {
        let next = advance(
            &INITIAL_STATE,
            Action::RepairWater,
            WATER_DEGRADATION_WORLD.params,
        )
        .unwrap();
        assert_eq!(next.water_reserve, 18);
        assert_eq!(next.energy_reserve, 17);
        assert_eq!(next.maintenance_credits, 1);
        assert_eq!(next.water_maintenance_debt, 2);
    }
}
