// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact prior-free contingent active-sensing policy trees.
//!
//! Unlike Bayesian expected-utility experiment planners, this module treats
//! the hidden environment as a finite set with no probability weights. Every
//! reachable observation branch must have a feasible continuation, and
//! sensing costs are charged only on the root-to-leaf path where the sensor
//! is actually requested.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    ActiveSensingConfig, DecisionWorld, MissingCause, SensorAction, SensorPrediction, SensorStatus,
    TOLERANCE, WorldOracle, baseline_no_sensing, find_policy, oracle_for, validate_config,
    validate_sensor, validate_worlds,
};

const MAX_SENSORS: usize = 8;
const MAX_SENSOR_DEPTH: usize = 3;
const MAX_SEARCH_NODES: usize = 10_000;

#[derive(Debug, Clone, PartialEq)]
pub struct ContingentSensor {
    pub sensor: SensorAction,
    /// None means available at the root. Otherwise this sensor may only be
    /// requested immediately after this exact public observation token.
    pub required_previous_observation: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContingentSearchLimits {
    pub maximum_sensor_depth: usize,
    pub maximum_search_nodes: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum CandidatePolicyNode {
    StopAndAct {
        action_id: String,
    },
    RequestSensor {
        sensor_id: String,
        branches: Vec<CandidatePolicyBranch>,
    },
}

#[derive(Debug, Clone, PartialEq)]
pub struct CandidatePolicyBranch {
    pub observation_id: String,
    pub next: Box<CandidatePolicyNode>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContingentNodeReceipt {
    pub observation_history: Vec<String>,
    pub node_kind: String,
    pub active_world_ids: Vec<String>,
    pub sensor_id: Option<String>,
    pub action_id: Option<String>,
    pub protocol_digest: Option<String>,
    pub path_energy_kwh: f64,
    pub path_latency_ticks: u32,
    pub maximum_path_penalty_service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContingentPredictionReceipt {
    pub observation_history: Vec<String>,
    pub sensor_id: String,
    pub world_id: String,
    pub observation_id: String,
    pub prediction_receipt_digest: String,
    pub missing_cause: Option<MissingCause>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContingentLeafReceipt {
    pub observation_history: Vec<String>,
    pub world_id: String,
    pub selected_action_id: String,
    pub oracle_service_units: f64,
    pub gross_policy_service_units: f64,
    pub path_energy_kwh: f64,
    pub path_latency_ticks: u32,
    pub sensing_penalty_service_units: f64,
    pub net_policy_service_units: f64,
    pub regret_service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ContingentSensingReport {
    pub scenario_id: String,
    pub source_digest: String,
    pub status: SensorStatus,
    pub recommended: bool,
    pub baseline: super::NoSensingAssessment,
    pub worst_case_post_sensing_regret: Option<f64>,
    pub worst_case_regret_reduction: Option<f64>,
    pub resolves_no_sensing_infeasibility: bool,
    pub maximum_sensor_depth: usize,
    pub search_nodes_evaluated: usize,
    pub selected_tree_depth: usize,
    pub maximum_path_energy_kwh: f64,
    pub maximum_path_latency_ticks: u32,
    pub maximum_path_penalty_service_units: f64,
    /// Candidate-facing projection. Contains observation tokens and actions
    /// only; no world IDs, true outcomes, missingness causes or regret.
    pub candidate_policy_tree: Option<CandidatePolicyNode>,
    /// Evaluator-only receipts. Never pass these records to a candidate.
    pub evaluator_node_receipts: Vec<ContingentNodeReceipt>,
    pub evaluator_prediction_receipts: Vec<ContingentPredictionReceipt>,
    pub evaluator_leaf_receipts: Vec<ContingentLeafReceipt>,
}

#[derive(Clone)]
struct NodeSolution {
    candidate: CandidatePolicyNode,
    node_receipts: Vec<ContingentNodeReceipt>,
    prediction_receipts: Vec<ContingentPredictionReceipt>,
    leaf_receipts: Vec<ContingentLeafReceipt>,
    worst_case_regret: f64,
    maximum_sensor_depth: usize,
    maximum_path_energy_kwh: f64,
    maximum_path_latency_ticks: u32,
    maximum_path_penalty_service_units: f64,
    first_sensor_id: Option<String>,
}

fn sorted_world_ids(worlds: &[&DecisionWorld]) -> Vec<String> {
    let mut ids: Vec<String> = worlds.iter().map(|world| world.world_id.clone()).collect();
    ids.sort();
    ids
}

fn max_penalty(penalties: &BTreeMap<String, f64>) -> f64 {
    penalties.values().copied().fold(0.0_f64, f64::max)
}

fn better_solution(candidate: &NodeSolution, current: &NodeSolution, tolerance: f64) -> bool {
    if candidate.worst_case_regret + tolerance < current.worst_case_regret {
        return true;
    }
    if current.worst_case_regret + tolerance < candidate.worst_case_regret {
        return false;
    }

    // On a regret tie, prefer the shorter policy (STOP wins unnecessary
    // sensing ties), then lower worst-path penalty, then stable sensor ID.
    if candidate.maximum_sensor_depth != current.maximum_sensor_depth {
        return candidate.maximum_sensor_depth < current.maximum_sensor_depth;
    }
    if candidate.maximum_path_penalty_service_units + tolerance
        < current.maximum_path_penalty_service_units
    {
        return true;
    }
    if current.maximum_path_penalty_service_units + tolerance
        < candidate.maximum_path_penalty_service_units
    {
        return false;
    }
    if candidate.maximum_path_energy_kwh + tolerance < current.maximum_path_energy_kwh {
        return true;
    }
    if current.maximum_path_energy_kwh + tolerance < candidate.maximum_path_energy_kwh {
        return false;
    }
    if candidate.maximum_path_latency_ticks < current.maximum_path_latency_ticks {
        return true;
    }
    if current.maximum_path_latency_ticks < candidate.maximum_path_latency_ticks {
        return false;
    }
    candidate.first_sensor_id.as_deref().unwrap_or("")
        < current.first_sensor_id.as_deref().unwrap_or("")
}

fn stop_solution(
    worlds: &[&DecisionWorld],
    oracle: &[WorldOracle],
    history: &[String],
    path_energy_kwh: f64,
    path_latency_ticks: u32,
    penalties: &BTreeMap<String, f64>,
    tolerance: f64,
) -> Result<Option<NodeSolution>, String> {
    if worlds.is_empty() {
        return Err("contingent policy node cannot contain an empty world set".to_owned());
    }

    let mut policy_ids: Vec<String> = worlds[0]
        .policies
        .iter()
        .map(|policy| policy.policy_id.clone())
        .collect();
    policy_ids.sort();

    let mut best: Option<NodeSolution> = None;
    for policy_id in policy_ids {
        let mut feasible = true;
        let mut worst_regret = 0.0_f64;
        let mut leaf_receipts = Vec::with_capacity(worlds.len());

        for world in worlds {
            let policy = find_policy(world, &policy_id)?;
            let penalty = penalties
                .get(&world.world_id)
                .copied()
                .ok_or_else(|| format!("missing cumulative penalty for {}", world.world_id))?;
            let net_service = policy.service_units - penalty;
            if !net_service.is_finite() {
                return Err(format!(
                    "non-finite post-sensing service in {}",
                    world.world_id
                ));
            }
            if net_service + TOLERANCE < world.service_floor {
                feasible = false;
                break;
            }

            let truth = oracle_for(oracle, &world.world_id)?.oracle_service_units;
            let regret = (truth - net_service).max(0.0);
            if !regret.is_finite() {
                return Err(format!(
                    "non-finite contingent regret in {}",
                    world.world_id
                ));
            }
            worst_regret = worst_regret.max(regret);
            leaf_receipts.push(ContingentLeafReceipt {
                observation_history: history.to_vec(),
                world_id: world.world_id.clone(),
                selected_action_id: policy_id.clone(),
                oracle_service_units: truth,
                gross_policy_service_units: policy.service_units,
                path_energy_kwh,
                path_latency_ticks,
                sensing_penalty_service_units: penalty,
                net_policy_service_units: net_service,
                regret_service_units: regret,
            });
        }

        if !feasible {
            continue;
        }

        let solution = NodeSolution {
            candidate: CandidatePolicyNode::StopAndAct {
                action_id: policy_id.clone(),
            },
            node_receipts: vec![ContingentNodeReceipt {
                observation_history: history.to_vec(),
                node_kind: "stop_and_act".to_owned(),
                active_world_ids: sorted_world_ids(worlds),
                sensor_id: None,
                action_id: Some(policy_id),
                protocol_digest: None,
                path_energy_kwh,
                path_latency_ticks,
                maximum_path_penalty_service_units: max_penalty(penalties),
            }],
            prediction_receipts: Vec::new(),
            leaf_receipts,
            worst_case_regret: worst_regret,
            maximum_sensor_depth: 0,
            maximum_path_energy_kwh: path_energy_kwh,
            maximum_path_latency_ticks: path_latency_ticks,
            maximum_path_penalty_service_units: max_penalty(penalties),
            first_sensor_id: None,
        };

        if best
            .as_ref()
            .is_none_or(|current| better_solution(&solution, current, tolerance))
        {
            best = Some(solution.clone());
        }
    }
    Ok(best)
}

#[allow(clippy::too_many_arguments)]
fn solve_node(
    config: &ActiveSensingConfig,
    worlds: &[&DecisionWorld],
    oracle: &[WorldOracle],
    sensors: &[ContingentSensor],
    prediction_maps: &[BTreeMap<String, &SensorPrediction>],
    history: &[String],
    used_sensors: &BTreeSet<usize>,
    depth_remaining: usize,
    path_energy_kwh: f64,
    path_latency_ticks: u32,
    penalties: &BTreeMap<String, f64>,
    search_nodes: &mut usize,
    maximum_search_nodes: usize,
) -> Result<Option<NodeSolution>, String> {
    *search_nodes += 1;
    if *search_nodes > maximum_search_nodes {
        return Err(
            "SEARCH_SPACE_LIMIT: contingent sensor policy search exceeded its declared node cap"
                .to_owned(),
        );
    }

    let mut best = stop_solution(
        worlds,
        oracle,
        history,
        path_energy_kwh,
        path_latency_ticks,
        penalties,
        config.tie_tolerance,
    )?;

    if depth_remaining == 0 {
        return Ok(best);
    }

    for (sensor_index, sensor_spec) in sensors.iter().enumerate() {
        if used_sensors.contains(&sensor_index) {
            continue;
        }
        match sensor_spec.required_previous_observation.as_deref() {
            None if !history.is_empty() => continue,
            Some(required) if history.last().map(String::as_str) != Some(required) => continue,
            _ => {}
        }

        let sensor = &sensor_spec.sensor;
        let next_energy = path_energy_kwh + sensor.energy_cost_kwh;
        let next_latency = path_latency_ticks
            .checked_add(sensor.latency_ticks)
            .ok_or_else(|| "SEARCH_SPACE_LIMIT: path latency overflow".to_owned())?;
        if next_energy > config.maximum_energy_kwh + config.tie_tolerance
            || next_latency > config.maximum_latency_ticks
        {
            continue;
        }

        let energy_penalty = sensor.energy_cost_kwh * sensor.service_units_per_kwh;
        if !energy_penalty.is_finite() {
            return Err(format!(
                "non-finite energy penalty for {}",
                sensor.sensor_id
            ));
        }

        let mut grouped_worlds: BTreeMap<String, Vec<&DecisionWorld>> = BTreeMap::new();
        let mut grouped_penalties: BTreeMap<String, BTreeMap<String, f64>> = BTreeMap::new();
        let mut prediction_receipts = Vec::with_capacity(worlds.len());
        let mut within_penalty_budget = true;

        for world in worlds {
            let prediction = prediction_maps[sensor_index]
                .get(&world.world_id)
                .ok_or_else(|| {
                    format!(
                        "missing {} prediction for {}",
                        sensor.sensor_id, world.world_id
                    )
                })?;
            let prior_penalty = penalties
                .get(&world.world_id)
                .copied()
                .ok_or_else(|| format!("missing prior path penalty for {}", world.world_id))?;
            let latency_penalty =
                f64::from(sensor.latency_ticks) * prediction.latency_service_loss_per_tick;
            let cumulative_penalty = prior_penalty + energy_penalty + latency_penalty;
            if !cumulative_penalty.is_finite() {
                return Err(format!(
                    "non-finite cumulative sensing penalty in {}",
                    world.world_id
                ));
            }
            if cumulative_penalty > config.maximum_penalty_service_units + config.tie_tolerance {
                within_penalty_budget = false;
                break;
            }

            grouped_worlds
                .entry(prediction.observation_id.clone())
                .or_default()
                .push(world);
            grouped_penalties
                .entry(prediction.observation_id.clone())
                .or_default()
                .insert(world.world_id.clone(), cumulative_penalty);
            prediction_receipts.push(ContingentPredictionReceipt {
                observation_history: history.to_vec(),
                sensor_id: sensor.sensor_id.clone(),
                world_id: world.world_id.clone(),
                observation_id: prediction.observation_id.clone(),
                prediction_receipt_digest: prediction.prediction_receipt_digest.clone(),
                missing_cause: prediction.missing_cause.clone(),
            });
        }

        if !within_penalty_budget || grouped_worlds.is_empty() {
            continue;
        }

        let mut next_used = used_sensors.clone();
        next_used.insert(sensor_index);
        let mut branches = Vec::with_capacity(grouped_worlds.len());
        let mut child_solutions = Vec::with_capacity(grouped_worlds.len());
        let mut all_branches_feasible = true;

        for (observation_id, branch_worlds) in &grouped_worlds {
            let mut next_history = history.to_vec();
            next_history.push(observation_id.clone());
            let branch_penalties = grouped_penalties
                .get(observation_id)
                .ok_or_else(|| format!("missing grouped penalties for {}", sensor.sensor_id))?;

            let child = solve_node(
                config,
                branch_worlds,
                oracle,
                sensors,
                prediction_maps,
                &next_history,
                &next_used,
                depth_remaining - 1,
                next_energy,
                next_latency,
                branch_penalties,
                search_nodes,
                maximum_search_nodes,
            )?;
            let Some(child) = child else {
                all_branches_feasible = false;
                break;
            };

            branches.push(CandidatePolicyBranch {
                observation_id: observation_id.clone(),
                next: Box::new(child.candidate.clone()),
            });
            child_solutions.push(child);
        }

        if !all_branches_feasible || child_solutions.is_empty() {
            continue;
        }

        let worst_case_regret = child_solutions
            .iter()
            .map(|child| child.worst_case_regret)
            .fold(0.0_f64, f64::max);
        let maximum_sensor_depth = 1 + child_solutions
            .iter()
            .map(|child| child.maximum_sensor_depth)
            .max()
            .unwrap_or(0);
        let maximum_path_penalty_service_units = child_solutions
            .iter()
            .map(|child| child.maximum_path_penalty_service_units)
            .fold(0.0_f64, f64::max);
        let maximum_path_energy_kwh = child_solutions
            .iter()
            .map(|child| child.maximum_path_energy_kwh)
            .fold(0.0_f64, f64::max);
        let maximum_path_latency_ticks = child_solutions
            .iter()
            .map(|child| child.maximum_path_latency_ticks)
            .max()
            .unwrap_or(next_latency);

        let mut node_receipts = vec![ContingentNodeReceipt {
            observation_history: history.to_vec(),
            node_kind: "request_sensor".to_owned(),
            active_world_ids: sorted_world_ids(worlds),
            sensor_id: Some(sensor.sensor_id.clone()),
            action_id: None,
            protocol_digest: Some(sensor.protocol_digest.clone()),
            path_energy_kwh: next_energy,
            path_latency_ticks: next_latency,
            maximum_path_penalty_service_units: grouped_penalties
                .values()
                .flat_map(|map| map.values().copied())
                .fold(0.0_f64, f64::max),
        }];
        let mut all_prediction_receipts = prediction_receipts;
        let mut leaf_receipts = Vec::new();
        for child in child_solutions {
            node_receipts.extend(child.node_receipts);
            all_prediction_receipts.extend(child.prediction_receipts);
            leaf_receipts.extend(child.leaf_receipts);
        }

        let solution = NodeSolution {
            candidate: CandidatePolicyNode::RequestSensor {
                sensor_id: sensor.sensor_id.clone(),
                branches,
            },
            node_receipts,
            prediction_receipts: all_prediction_receipts,
            leaf_receipts,
            worst_case_regret,
            maximum_sensor_depth,
            maximum_path_energy_kwh,
            maximum_path_latency_ticks,
            maximum_path_penalty_service_units,
            first_sensor_id: Some(sensor.sensor_id.clone()),
        };

        if best
            .as_ref()
            .is_none_or(|current| better_solution(&solution, current, config.tie_tolerance))
        {
            best = Some(solution.clone());
        }
    }

    Ok(best)
}

/// Evaluate an exact prior-free observation-conditioned sensor policy tree.
///
/// A sensor may be restricted to a prior observation token. Its cost is
/// charged only on the branch where it is requested. Every reachable outcome
/// branch must have a robust feasible continuation.
pub fn assess_contingent_sensing(
    config: &ActiveSensingConfig,
    limits: &ContingentSearchLimits,
    worlds: &[DecisionWorld],
    sensors: &[ContingentSensor],
) -> Result<ContingentSensingReport, String> {
    validate_config(config)?;
    if limits.maximum_sensor_depth > MAX_SENSOR_DEPTH
        || limits.maximum_search_nodes == 0
        || limits.maximum_search_nodes > MAX_SEARCH_NODES
    {
        return Err(
            "SEARCH_SPACE_LIMIT: contingent depth must be <= 3 and node cap must be in 1..=10000"
                .to_owned(),
        );
    }
    if sensors.len() > MAX_SENSORS {
        return Err("SEARCH_SPACE_LIMIT: contingent sensor catalogue exceeds 8 sensors".to_owned());
    }

    let oracle = validate_worlds(worlds)?;
    let baseline = baseline_no_sensing(worlds, &oracle);
    let mut sensor_ids = BTreeSet::new();
    let mut prediction_maps = Vec::with_capacity(sensors.len());

    for sensor in sensors {
        if !sensor_ids.insert(sensor.sensor.sensor_id.as_str()) {
            return Err(format!(
                "duplicate contingent sensor {}",
                sensor.sensor.sensor_id
            ));
        }
        if let Some(required) = &sensor.required_previous_observation
            && required.trim().is_empty()
        {
            return Err(format!(
                "empty sensor prerequisite for {}",
                sensor.sensor.sensor_id
            ));
        }
        prediction_maps.push(validate_sensor(&sensor.sensor, worlds)?);
    }

    let world_refs: Vec<&DecisionWorld> = worlds.iter().collect();
    let initial_penalties: BTreeMap<String, f64> = worlds
        .iter()
        .map(|world| (world.world_id.clone(), 0.0))
        .collect();
    let mut search_nodes = 0;
    let solution = solve_node(
        config,
        &world_refs,
        &oracle,
        sensors,
        &prediction_maps,
        &[],
        &BTreeSet::new(),
        limits.maximum_sensor_depth,
        0.0,
        0,
        &initial_penalties,
        &mut search_nodes,
        limits.maximum_search_nodes,
    )?;

    let Some(solution) = solution else {
        return Ok(ContingentSensingReport {
            scenario_id: config.scenario_id.clone(),
            source_digest: config.source_digest.clone(),
            status: SensorStatus::NoFeasiblePolicy,
            recommended: false,
            baseline: baseline.clone(),
            worst_case_post_sensing_regret: None,
            worst_case_regret_reduction: None,
            resolves_no_sensing_infeasibility: false,
            maximum_sensor_depth: limits.maximum_sensor_depth,
            search_nodes_evaluated: search_nodes,
            selected_tree_depth: 0,
            maximum_path_energy_kwh: 0.0,
            maximum_path_latency_ticks: 0,
            maximum_path_penalty_service_units: 0.0,
            candidate_policy_tree: None,
            evaluator_node_receipts: Vec::new(),
            evaluator_prediction_receipts: Vec::new(),
            evaluator_leaf_receipts: Vec::new(),
        });
    };

    let root_is_sensor = matches!(
        &solution.candidate,
        CandidatePolicyNode::RequestSensor { .. }
    );
    let resolves_no_sensing_infeasibility =
        baseline.common_feasible_policy_count == 0 && root_is_sensor;
    let worst_case_regret_reduction = baseline
        .worst_case_regret
        .map(|before| before - solution.worst_case_regret);
    let recommended = root_is_sensor
        && (resolves_no_sensing_infeasibility
            || worst_case_regret_reduction
                .is_some_and(|reduction| reduction > config.tie_tolerance));

    // Keep the selected policy tree even when its root is STOP.
    // `recommended` describes whether sensing improves on the no-sensing
    // benchmark, not whether the controller should receive its current action.
    let candidate_policy_tree = Some(solution.candidate);
    let mut evaluator_node_receipts = solution.node_receipts;
    evaluator_node_receipts.sort_by(|left, right| {
        left.observation_history
            .cmp(&right.observation_history)
            .then_with(|| left.node_kind.cmp(&right.node_kind))
            .then_with(|| left.sensor_id.cmp(&right.sensor_id))
            .then_with(|| left.action_id.cmp(&right.action_id))
    });
    let mut evaluator_prediction_receipts = solution.prediction_receipts;
    evaluator_prediction_receipts.sort_by(|left, right| {
        left.observation_history
            .cmp(&right.observation_history)
            .then_with(|| left.sensor_id.cmp(&right.sensor_id))
            .then_with(|| left.world_id.cmp(&right.world_id))
    });
    let mut evaluator_leaf_receipts = solution.leaf_receipts;
    evaluator_leaf_receipts.sort_by(|left, right| {
        left.observation_history
            .cmp(&right.observation_history)
            .then_with(|| left.world_id.cmp(&right.world_id))
    });

    Ok(ContingentSensingReport {
        scenario_id: config.scenario_id.clone(),
        source_digest: config.source_digest.clone(),
        status: SensorStatus::Evaluated,
        recommended,
        baseline,
        worst_case_post_sensing_regret: Some(solution.worst_case_regret),
        worst_case_regret_reduction,
        resolves_no_sensing_infeasibility,
        maximum_sensor_depth: limits.maximum_sensor_depth,
        search_nodes_evaluated: search_nodes,
        selected_tree_depth: solution.maximum_sensor_depth,
        maximum_path_energy_kwh: solution.maximum_path_energy_kwh,
        maximum_path_latency_ticks: solution.maximum_path_latency_ticks,
        maximum_path_penalty_service_units: solution.maximum_path_penalty_service_units,
        candidate_policy_tree,
        evaluator_node_receipts,
        evaluator_prediction_receipts,
        evaluator_leaf_receipts,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-8,
            "floating-point mismatch: actual={actual:.12}, expected={expected:.12}"
        );
    }

    fn config() -> ActiveSensingConfig {
        ActiveSensingConfig {
            scenario_id: "contingent-three-world-screen-and-inspect-v1".to_owned(),
            source_digest: "sha256-contingent-three-world-source-v1".to_owned(),
            service_unit: "service_units".to_owned(),
            energy_unit: "kWh".to_owned(),
            maximum_energy_kwh: 0.5,
            maximum_latency_ticks: 2,
            maximum_penalty_service_units: 2.0,
            tie_tolerance: 1.0e-9,
        }
    }

    fn limits(depth: usize) -> ContingentSearchLimits {
        ContingentSearchLimits {
            maximum_sensor_depth: depth,
            maximum_search_nodes: 1_000,
        }
    }

    fn world(id: &str, defer: f64, repair: f64) -> DecisionWorld {
        DecisionWorld {
            world_id: id.to_owned(),
            world_digest: format!("sha256-{id}"),
            service_floor: 70.0,
            policies: vec![
                super::super::PolicyOutcome {
                    policy_id: "defer".to_owned(),
                    service_units: defer,
                },
                super::super::PolicyOutcome {
                    policy_id: "repair".to_owned(),
                    service_units: repair,
                },
            ],
        }
    }

    fn worlds() -> Vec<DecisionWorld> {
        vec![
            world("healthy", 100.0, 85.0),
            world("degraded", 40.0, 80.0),
            world("healthy_sensor_fault", 100.0, 85.0),
        ]
    }

    fn sensor(
        id: &str,
        energy: f64,
        conversion: f64,
        latency: u32,
        prerequisite: Option<&str>,
        outcomes: Vec<(&str, &str, Option<MissingCause>, f64)>,
    ) -> ContingentSensor {
        let predictions = outcomes
            .into_iter()
            .map(
                |(world_id, observation_id, missing_cause, latency_loss)| SensorPrediction {
                    world_id: world_id.to_owned(),
                    world_digest: format!("sha256-{world_id}"),
                    observation_id: observation_id.to_owned(),
                    missing_cause,
                    latency_service_loss_per_tick: latency_loss,
                    prediction_receipt_digest: format!("sha256-{id}-{world_id}-receipt"),
                },
            )
            .collect();
        ContingentSensor {
            sensor: SensorAction {
                sensor_id: id.to_owned(),
                protocol_digest: format!("sha256-{id}-protocol"),
                energy_cost_kwh: energy,
                service_units_per_kwh: conversion,
                latency_ticks: latency,
                predictions,
            },
            required_previous_observation: prerequisite.map(str::to_owned),
        }
    }

    fn sensors() -> Vec<ContingentSensor> {
        vec![
            sensor(
                "screen",
                0.2,
                2.0,
                1,
                None,
                vec![
                    ("healthy", "healthy", None, 0.2),
                    (
                        "degraded",
                        "missing",
                        Some(MissingCause::CommonCausePowerLoss),
                        0.5,
                    ),
                    (
                        "healthy_sensor_fault",
                        "missing",
                        Some(MissingCause::TelemetryNetworkFault),
                        0.1,
                    ),
                ],
            ),
            sensor(
                "independent_inspection",
                0.15,
                2.0,
                1,
                Some("missing"),
                vec![
                    ("healthy", "healthy", None, 0.2),
                    ("degraded", "degraded", None, 0.2),
                    ("healthy_sensor_fault", "healthy", None, 0.1),
                ],
            ),
            sensor(
                "uninformative_followup",
                0.1,
                2.0,
                1,
                Some("missing"),
                vec![
                    ("healthy", "healthy", None, 0.1),
                    (
                        "degraded",
                        "missing",
                        Some(MissingCause::SensorDeviceFailure),
                        0.1,
                    ),
                    (
                        "healthy_sensor_fault",
                        "missing",
                        Some(MissingCause::InspectionNonresponse),
                        0.1,
                    ),
                ],
            ),
            sensor(
                "over_budget_followup",
                1.0,
                2.0,
                1,
                Some("missing"),
                vec![
                    ("healthy", "healthy", None, 0.1),
                    ("degraded", "degraded", None, 0.1),
                    ("healthy_sensor_fault", "healthy", None, 0.1),
                ],
            ),
        ]
    }

    #[test]
    fn contingent_tree_stops_on_easy_branch_and_inspects_only_ambiguous_branch() {
        let report =
            assess_contingent_sensing(&config(), &limits(2), &worlds(), &sensors()).unwrap();
        assert_eq!(report.status, SensorStatus::Evaluated);
        assert!(report.recommended);
        assert_close(report.worst_case_regret_reduction.unwrap(), 13.6);
        assert_close(report.worst_case_post_sensing_regret.unwrap(), 1.4);
        assert_eq!(report.selected_tree_depth, 2);
        assert_eq!(report.maximum_path_energy_kwh, 0.35);
        assert_eq!(report.maximum_path_latency_ticks, 2);
        assert_eq!(report.maximum_path_penalty_service_units, 1.4);

        let root = report.candidate_policy_tree.unwrap();
        let CandidatePolicyNode::RequestSensor {
            sensor_id,
            branches,
        } = root
        else {
            panic!("root must request the low-cost screen");
        };
        assert_eq!(sensor_id, "screen");
        assert_eq!(branches.len(), 2);
        let healthy = branches
            .iter()
            .find(|branch| branch.observation_id == "healthy")
            .unwrap();
        assert_eq!(
            *healthy.next,
            CandidatePolicyNode::StopAndAct {
                action_id: "defer".to_owned()
            }
        );
        let ambiguous = branches
            .iter()
            .find(|branch| branch.observation_id == "missing")
            .unwrap();
        let CandidatePolicyNode::RequestSensor {
            sensor_id,
            branches,
        } = ambiguous.next.as_ref()
        else {
            panic!("only the ambiguous branch should request a follow-up");
        };
        assert_eq!(sensor_id, "independent_inspection");
        assert_eq!(branches.len(), 2);
        assert_eq!(
            *branches
                .iter()
                .find(|branch| branch.observation_id == "degraded")
                .unwrap()
                .next,
            CandidatePolicyNode::StopAndAct {
                action_id: "repair".to_owned()
            }
        );
        assert_eq!(
            *branches
                .iter()
                .find(|branch| branch.observation_id == "healthy")
                .unwrap()
                .next,
            CandidatePolicyNode::StopAndAct {
                action_id: "defer".to_owned()
            }
        );
    }

    #[test]
    fn one_step_greedy_screen_is_not_selected_over_no_sensing() {
        let report =
            assess_contingent_sensing(&config(), &limits(1), &worlds(), &sensors()).unwrap();
        assert!(!report.recommended);
        assert_eq!(
            report.candidate_policy_tree,
            Some(CandidatePolicyNode::StopAndAct {
                action_id: "repair".to_owned()
            })
        );
        assert_eq!(report.worst_case_post_sensing_regret, Some(15.0));
        assert_eq!(report.baseline.worst_case_regret, Some(15.0));
    }

    #[test]
    fn distinct_hidden_missingness_causes_remain_one_public_observation_branch() {
        let report =
            assess_contingent_sensing(&config(), &limits(2), &worlds(), &sensors()).unwrap();
        let screen_receipts: Vec<&ContingentPredictionReceipt> = report
            .evaluator_prediction_receipts
            .iter()
            .filter(|receipt| receipt.sensor_id == "screen" && receipt.observation_id == "missing")
            .collect();
        assert_eq!(screen_receipts.len(), 2);
        assert_eq!(
            screen_receipts
                .iter()
                .map(|receipt| receipt.missing_cause.as_ref().unwrap())
                .collect::<BTreeSet<_>>()
                .len(),
            2
        );
        let root = report.candidate_policy_tree.unwrap();
        let CandidatePolicyNode::RequestSensor { branches, .. } = root else {
            unreachable!()
        };
        assert_eq!(
            branches
                .iter()
                .filter(|branch| branch.observation_id == "missing")
                .count(),
            1
        );
    }

    #[test]
    fn search_prefers_stop_when_followup_does_not_reduce_robust_regret() {
        let mut items = sensors();
        items.retain(|item| item.sensor.sensor_id != "independent_inspection");
        let report = assess_contingent_sensing(&config(), &limits(2), &worlds(), &items).unwrap();
        assert!(!report.recommended);
        assert_eq!(
            report.candidate_policy_tree,
            Some(CandidatePolicyNode::StopAndAct {
                action_id: "repair".to_owned()
            })
        );
        assert_eq!(report.worst_case_post_sensing_regret, Some(15.0));
    }

    #[test]
    fn over_budget_followup_cannot_be_selected() {
        let mut items = sensors();
        items.retain(|item| item.sensor.sensor_id != "independent_inspection");
        let report = assess_contingent_sensing(&config(), &limits(2), &worlds(), &items).unwrap();
        assert!(!report.recommended);
        assert!(
            !report
                .evaluator_node_receipts
                .iter()
                .any(|receipt| receipt.sensor_id.as_deref() == Some("over_budget_followup"))
        );
    }

    #[test]
    fn contingent_tree_uses_different_corroborating_sensors_for_false_healthy_and_missing() {
        let mut cfg = config();
        cfg.scenario_id = "contingent-false-reassurance-corroboration-v1".to_owned();
        cfg.source_digest = "sha256-false-reassurance-corroboration-v1".to_owned();
        cfg.maximum_energy_kwh = 0.25;
        cfg.maximum_penalty_service_units = 1.0;

        let worlds = vec![
            world("healthy", 100.0, 85.0),
            world("degraded", 40.0, 80.0),
            world("healthy_sensor_fault", 100.0, 85.0),
            world("degraded_stuck_high", 40.0, 80.0),
        ];

        let sensors = vec![
            sensor(
                "screen",
                0.1,
                2.0,
                1,
                None,
                vec![
                    ("healthy", "healthy", None, 0.1),
                    (
                        "degraded",
                        "missing",
                        Some(MissingCause::CommonCausePowerLoss),
                        0.1,
                    ),
                    (
                        "healthy_sensor_fault",
                        "missing",
                        Some(MissingCause::TelemetryNetworkFault),
                        0.1,
                    ),
                    ("degraded_stuck_high", "healthy", None, 0.1),
                ],
            ),
            sensor(
                "stuck_high_confirmation",
                0.1,
                2.0,
                1,
                Some("healthy"),
                vec![
                    ("healthy", "verified_healthy", None, 0.1),
                    ("degraded", "not_requested", None, 0.1),
                    ("healthy_sensor_fault", "not_requested", None, 0.1),
                    ("degraded_stuck_high", "verified_degraded", None, 0.1),
                ],
            ),
            sensor(
                "independent_missing_branch_inspection",
                0.1,
                2.0,
                1,
                Some("missing"),
                vec![
                    ("healthy", "not_requested", None, 0.1),
                    ("degraded", "verified_degraded", None, 0.1),
                    ("healthy_sensor_fault", "verified_healthy", None, 0.1),
                    ("degraded_stuck_high", "not_requested", None, 0.1),
                ],
            ),
        ];

        let report = assess_contingent_sensing(&cfg, &limits(2), &worlds, &sensors).unwrap();
        assert_eq!(report.baseline.worst_case_regret, Some(15.0));
        assert!(report.recommended);
        assert_close(report.worst_case_post_sensing_regret.unwrap(), 0.6);
        assert_close(report.worst_case_regret_reduction.unwrap(), 14.4);
        assert_eq!(report.maximum_path_energy_kwh, 0.2);
        assert_eq!(report.maximum_path_latency_ticks, 2);
        assert_eq!(report.maximum_path_penalty_service_units, 0.6);

        let CandidatePolicyNode::RequestSensor {
            sensor_id,
            branches,
        } = report.candidate_policy_tree.unwrap()
        else {
            panic!("root must request the screening sensor");
        };
        assert_eq!(sensor_id, "screen");
        let healthy = branches
            .iter()
            .find(|branch| branch.observation_id == "healthy")
            .unwrap();
        let missing = branches
            .iter()
            .find(|branch| branch.observation_id == "missing")
            .unwrap();

        let CandidatePolicyNode::RequestSensor {
            sensor_id,
            branches: healthy_branches,
        } = healthy.next.as_ref()
        else {
            panic!("false-healthy branch must request independent corroboration");
        };
        assert_eq!(sensor_id, "stuck_high_confirmation");
        let CandidatePolicyNode::RequestSensor {
            sensor_id,
            branches: missing_branches,
        } = missing.next.as_ref()
        else {
            panic!("missing branch must request its own independent inspection");
        };
        assert_eq!(sensor_id, "independent_missing_branch_inspection");

        assert_eq!(
            healthy_branches
                .iter()
                .map(|branch| branch.observation_id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["verified_degraded", "verified_healthy"])
        );
        assert_eq!(
            missing_branches
                .iter()
                .map(|branch| branch.observation_id.as_str())
                .collect::<BTreeSet<_>>(),
            BTreeSet::from(["verified_degraded", "verified_healthy"])
        );
    }

    #[test]
    fn mutually_exclusive_branch_costs_are_not_summed_across_the_whole_tree() {
        let report =
            assess_contingent_sensing(&config(), &limits(2), &worlds(), &sensors()).unwrap();
        let healthy_leaf = report
            .evaluator_leaf_receipts
            .iter()
            .find(|receipt| receipt.world_id == "healthy")
            .unwrap();
        let degraded_leaf = report
            .evaluator_leaf_receipts
            .iter()
            .find(|receipt| receipt.world_id == "degraded")
            .unwrap();
        assert_eq!(healthy_leaf.path_energy_kwh, 0.2);
        assert_eq!(healthy_leaf.path_latency_ticks, 1);
        assert_close(healthy_leaf.sensing_penalty_service_units, 0.6);
        assert_eq!(degraded_leaf.path_energy_kwh, 0.35);
        assert_eq!(degraded_leaf.path_latency_ticks, 2);
        assert_close(degraded_leaf.sensing_penalty_service_units, 1.4);
        assert_eq!(report.maximum_path_energy_kwh, 0.35);
        assert_eq!(report.maximum_path_latency_ticks, 2);
    }

    #[test]
    fn input_order_does_not_change_the_selected_contingent_policy_or_receipts() {
        let cfg = config();
        let sensors_forward = sensors();
        let mut sensors_reverse = sensors_forward.clone();
        sensors_reverse.reverse();
        let worlds_forward = worlds();
        let mut worlds_reverse = worlds_forward.clone();
        worlds_reverse.reverse();

        let forward =
            assess_contingent_sensing(&cfg, &limits(2), &worlds_forward, &sensors_forward).unwrap();
        let reverse =
            assess_contingent_sensing(&cfg, &limits(2), &worlds_reverse, &sensors_reverse).unwrap();
        assert_eq!(forward.candidate_policy_tree, reverse.candidate_policy_tree);
        assert_eq!(
            forward.worst_case_post_sensing_regret,
            reverse.worst_case_post_sensing_regret
        );
        assert_eq!(
            forward.evaluator_node_receipts,
            reverse.evaluator_node_receipts
        );
        assert_eq!(
            forward.evaluator_prediction_receipts,
            reverse.evaluator_prediction_receipts
        );
        assert_eq!(
            forward.evaluator_leaf_receipts,
            reverse.evaluator_leaf_receipts
        );
    }

    #[test]
    fn node_cap_is_enforced_during_recursive_search() {
        let mut cap = limits(2);
        cap.maximum_search_nodes = 1;
        let error = assess_contingent_sensing(&config(), &cap, &worlds(), &sensors()).unwrap_err();
        assert!(error.contains("SEARCH_SPACE_LIMIT"));
    }

    #[test]
    fn invalid_budget_depth_node_cap_and_duplicate_sensor_fail_closed() {
        assert!(
            assess_contingent_sensing(
                &config(),
                &ContingentSearchLimits {
                    maximum_sensor_depth: 4,
                    maximum_search_nodes: 10
                },
                &worlds(),
                &sensors()
            )
            .is_err()
        );
        assert!(
            assess_contingent_sensing(
                &config(),
                &ContingentSearchLimits {
                    maximum_sensor_depth: 2,
                    maximum_search_nodes: 0
                },
                &worlds(),
                &sensors()
            )
            .is_err()
        );
        let mut duplicate = sensors();
        duplicate.push(duplicate[0].clone());
        assert!(assess_contingent_sensing(&config(), &limits(2), &worlds(), &duplicate).is_err());
    }
}
