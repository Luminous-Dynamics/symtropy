// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-model active sensing with explicit energy, acquisition latency,
//! world-dependent service loss and policy-visible missingness.
//!
//! Candidate control is conditioned only on the categorical observation ID.
//! Missingness causes and model identities remain evaluator-side evidence.

#![forbid(unsafe_code)]

pub mod contingent;
pub mod sequential;

use std::collections::{BTreeMap, BTreeSet};

const TOLERANCE: f64 = 1.0e-9;
const MISSING_OUTCOME: &str = "missing";

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum MissingCause {
    CommonCausePowerLoss,
    TelemetryNetworkFault,
    SensorDeviceFailure,
    InspectionNonresponse,
}

impl MissingCause {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::CommonCausePowerLoss => "common_cause_power_loss",
            Self::TelemetryNetworkFault => "telemetry_network_fault",
            Self::SensorDeviceFailure => "sensor_device_failure",
            Self::InspectionNonresponse => "inspection_nonresponse",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyOutcome {
    pub policy_id: String,
    pub service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionWorld {
    pub world_id: String,
    pub world_digest: String,
    pub service_floor: f64,
    pub policies: Vec<PolicyOutcome>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorPrediction {
    pub world_id: String,
    pub world_digest: String,
    pub observation_id: String,
    pub missing_cause: Option<MissingCause>,
    pub latency_service_loss_per_tick: f64,
    pub prediction_receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorAction {
    pub sensor_id: String,
    pub protocol_digest: String,
    pub energy_cost_kwh: f64,
    pub service_units_per_kwh: f64,
    pub latency_ticks: u32,
    pub predictions: Vec<SensorPrediction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveSensingConfig {
    pub scenario_id: String,
    pub source_digest: String,
    pub service_unit: String,
    pub energy_unit: String,
    pub maximum_energy_kwh: f64,
    pub maximum_latency_ticks: u32,
    pub maximum_penalty_service_units: f64,
    pub tie_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct WorldOracle {
    pub world_id: String,
    pub oracle_policy_id: String,
    pub oracle_service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct NoSensingAssessment {
    pub selected_policy_id: Option<String>,
    pub worst_case_regret: Option<f64>,
    pub common_feasible_policy_count: usize,
    pub worlds: Vec<WorldOracle>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct PartitionWorldRegret {
    pub world_id: String,
    pub selected_policy_id: String,
    pub oracle_service_units: f64,
    pub gross_policy_service_units: f64,
    pub sensing_penalty_service_units: f64,
    pub net_policy_service_units: f64,
    pub regret_service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ObservationPartition {
    // This field is candidate-visible. Evaluator missingness causes are not
    // used to define these partitions.
    pub observation_id: String,
    pub world_ids: Vec<String>,
    pub selected_policy_id: Option<String>,
    pub worst_case_regret: Option<f64>,
    pub world_regrets: Vec<PartitionWorldRegret>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SensorStatus {
    Evaluated,
    OverBudget,
    NoFeasiblePolicy,
}

impl SensorStatus {
    pub const fn id(&self) -> &'static str {
        match self {
            Self::Evaluated => "evaluated",
            Self::OverBudget => "over_budget",
            Self::NoFeasiblePolicy => "no_feasible_policy",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PredictionAuditReceipt {
    pub sensor_id: String,
    pub world_id: String,
    pub world_digest: String,
    pub observation_id: String,
    pub prediction_receipt_digest: String,
    // Evaluator-side only.
    pub missing_cause: Option<MissingCause>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SensorAssessment {
    pub sensor_id: String,
    pub protocol_digest: String,
    pub status: SensorStatus,
    pub affordable: bool,
    pub energy_cost_kwh: f64,
    pub service_units_per_kwh: f64,
    pub energy_cost_service_units: f64,
    pub latency_ticks: u32,
    pub maximum_latency_service_loss: f64,
    pub maximum_total_penalty_service_units: f64,
    pub worst_case_post_sensing_regret: Option<f64>,
    pub worst_case_regret_reduction: Option<f64>,
    pub resolves_no_sensing_infeasibility: bool,
    pub partitions: Vec<ObservationPartition>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ActiveSensingReport {
    pub scenario_id: String,
    pub source_digest: String,
    pub baseline: NoSensingAssessment,
    pub sensors: Vec<SensorAssessment>,
    pub recommended_sensor_id: Option<String>,
}

fn finite_nonnegative(value: f64) -> bool {
    value.is_finite() && value >= 0.0
}

fn validate_config(config: &ActiveSensingConfig) -> Result<(), String> {
    if config.scenario_id.trim().is_empty()
        || config.source_digest.trim().is_empty()
        || config.service_unit.trim().is_empty()
        || config.energy_unit.trim().is_empty()
    {
        return Err("scenario identity, source digest and units must be non-empty".to_owned());
    }
    if !finite_nonnegative(config.maximum_energy_kwh)
        || !finite_nonnegative(config.maximum_penalty_service_units)
        || !config.tie_tolerance.is_finite()
        || config.tie_tolerance < 0.0
    {
        return Err("budgets and tie tolerance must be finite and non-negative".to_owned());
    }
    Ok(())
}

fn validate_worlds(worlds: &[DecisionWorld]) -> Result<Vec<WorldOracle>, String> {
    if worlds.is_empty() {
        return Err("at least one decision world is required".to_owned());
    }
    let mut world_ids = BTreeSet::new();
    let mut common_policy_ids: Option<BTreeSet<String>> = None;
    let mut oracle = Vec::with_capacity(worlds.len());

    for world in worlds {
        if world.world_id.trim().is_empty()
            || world.world_digest.trim().is_empty()
            || !world.service_floor.is_finite()
            || world.policies.is_empty()
        {
            return Err(format!(
                "invalid identity, service floor or empty policies in {}",
                world.world_id
            ));
        }
        if !world_ids.insert(world.world_id.as_str()) {
            return Err(format!("duplicate decision world {}", world.world_id));
        }

        let mut policy_ids = BTreeSet::new();
        for policy in &world.policies {
            if policy.policy_id.trim().is_empty() || !policy.service_units.is_finite() {
                return Err(format!("invalid policy outcome in {}", world.world_id));
            }
            if !policy_ids.insert(policy.policy_id.clone()) {
                return Err(format!(
                    "duplicate policy {} in {}",
                    policy.policy_id, world.world_id
                ));
            }
        }
        if let Some(expected) = &common_policy_ids {
            if expected != &policy_ids {
                return Err("all worlds must contain the same complete policy set".to_owned());
            }
        } else {
            common_policy_ids = Some(policy_ids);
        }

        let best = world
            .policies
            .iter()
            .filter(|policy| policy.service_units + TOLERANCE >= world.service_floor)
            .max_by(|left, right| {
                left.service_units
                    .total_cmp(&right.service_units)
                    .then_with(|| right.policy_id.cmp(&left.policy_id))
            })
            .ok_or_else(|| {
                format!(
                    "no physical policy meets service floor in {}",
                    world.world_id
                )
            })?;

        oracle.push(WorldOracle {
            world_id: world.world_id.clone(),
            oracle_policy_id: best.policy_id.clone(),
            oracle_service_units: best.service_units,
        });
    }

    oracle.sort_by(|left, right| left.world_id.cmp(&right.world_id));
    Ok(oracle)
}

fn find_policy<'a>(world: &'a DecisionWorld, policy_id: &str) -> Result<&'a PolicyOutcome, String> {
    world
        .policies
        .iter()
        .find(|policy| policy.policy_id == policy_id)
        .ok_or_else(|| format!("policy {policy_id} absent from {}", world.world_id))
}

fn oracle_for<'a>(oracle: &'a [WorldOracle], world_id: &str) -> Result<&'a WorldOracle, String> {
    oracle
        .iter()
        .find(|item| item.world_id == world_id)
        .ok_or_else(|| format!("missing oracle receipt for {world_id}"))
}

fn baseline_no_sensing(worlds: &[DecisionWorld], oracle: &[WorldOracle]) -> NoSensingAssessment {
    let mut policy_ids: Vec<String> = worlds[0]
        .policies
        .iter()
        .map(|policy| policy.policy_id.clone())
        .collect();
    policy_ids.sort();

    let mut candidates = Vec::new();
    for policy_id in policy_ids {
        let mut worst_regret = 0.0_f64;
        let mut feasible = true;
        for world in worlds {
            let policy = find_policy(world, &policy_id).expect("validated policy matrix");
            if policy.service_units + TOLERANCE < world.service_floor {
                feasible = false;
                break;
            }
            let truth = oracle_for(oracle, &world.world_id)
                .expect("complete oracle")
                .oracle_service_units;
            worst_regret = worst_regret.max((truth - policy.service_units).max(0.0));
        }
        if feasible {
            candidates.push((policy_id, worst_regret));
        }
    }
    candidates.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    NoSensingAssessment {
        selected_policy_id: candidates.first().map(|entry| entry.0.clone()),
        worst_case_regret: candidates.first().map(|entry| entry.1),
        common_feasible_policy_count: candidates.len(),
        worlds: oracle.to_vec(),
    }
}

fn validate_sensor<'a>(
    sensor: &'a SensorAction,
    worlds: &[DecisionWorld],
) -> Result<BTreeMap<String, &'a SensorPrediction>, String> {
    if sensor.sensor_id.trim().is_empty() || sensor.protocol_digest.trim().is_empty() {
        return Err("sensor ID and protocol digest must be non-empty".to_owned());
    }
    if !finite_nonnegative(sensor.energy_cost_kwh)
        || !finite_nonnegative(sensor.service_units_per_kwh)
    {
        return Err(format!(
            "invalid energy cost/conversion in {}",
            sensor.sensor_id
        ));
    }
    if sensor.predictions.len() != worlds.len() {
        return Err(format!(
            "{} contains {} predictions for {} worlds",
            sensor.sensor_id,
            sensor.predictions.len(),
            worlds.len()
        ));
    }

    let world_map: BTreeMap<&str, &DecisionWorld> = worlds
        .iter()
        .map(|world| (world.world_id.as_str(), world))
        .collect();
    let mut predictions = BTreeMap::new();

    for prediction in &sensor.predictions {
        if prediction.world_id.trim().is_empty()
            || prediction.world_digest.trim().is_empty()
            || prediction.observation_id.trim().is_empty()
            || prediction.prediction_receipt_digest.trim().is_empty()
            || !finite_nonnegative(prediction.latency_service_loss_per_tick)
        {
            return Err(format!(
                "invalid prediction metadata for {}",
                sensor.sensor_id
            ));
        }
        let world = world_map
            .get(prediction.world_id.as_str())
            .ok_or_else(|| format!("unknown world {}", prediction.world_id))?;
        if prediction.world_digest != world.world_digest {
            return Err(format!("world digest mismatch for {}", prediction.world_id));
        }
        if (prediction.observation_id == MISSING_OUTCOME) != prediction.missing_cause.is_some() {
            return Err(format!(
                "missingness cause must be present iff result is missing in {}",
                prediction.world_id
            ));
        }
        if predictions
            .insert(prediction.world_id.clone(), prediction)
            .is_some()
        {
            return Err(format!(
                "duplicate sensor prediction for {}",
                prediction.world_id
            ));
        }
    }
    Ok(predictions)
}

fn assess_partition(
    observation_id: &str,
    worlds: &[&DecisionWorld],
    predictions: &BTreeMap<String, &SensorPrediction>,
    oracle: &[WorldOracle],
    energy_cost_service_units: f64,
    latency_ticks: u32,
) -> ObservationPartition {
    let mut policy_ids: Vec<String> = worlds[0]
        .policies
        .iter()
        .map(|policy| policy.policy_id.clone())
        .collect();
    policy_ids.sort();

    let mut candidates: Vec<(String, f64, Vec<PartitionWorldRegret>)> = Vec::new();
    for policy_id in policy_ids {
        let mut common_feasible = true;
        let mut worst_regret = 0.0_f64;
        let mut rows = Vec::with_capacity(worlds.len());

        for world in worlds {
            let prediction = predictions
                .get(&world.world_id)
                .expect("complete predictions");
            let policy = find_policy(world, &policy_id).expect("common policy set");
            let energy_penalty = energy_cost_service_units;
            let latency_penalty =
                f64::from(latency_ticks) * prediction.latency_service_loss_per_tick;
            let total_penalty = energy_penalty + latency_penalty;
            let net_service = policy.service_units - total_penalty;

            if net_service + TOLERANCE < world.service_floor {
                common_feasible = false;
                break;
            }

            let oracle_value = oracle_for(oracle, &world.world_id)
                .expect("oracle covers all worlds")
                .oracle_service_units;
            let regret = (oracle_value - net_service).max(0.0);
            worst_regret = worst_regret.max(regret);
            rows.push(PartitionWorldRegret {
                world_id: world.world_id.clone(),
                selected_policy_id: policy_id.clone(),
                oracle_service_units: oracle_value,
                gross_policy_service_units: policy.service_units,
                sensing_penalty_service_units: total_penalty,
                net_policy_service_units: net_service,
                regret_service_units: regret,
            });
        }

        if common_feasible {
            candidates.push((policy_id, worst_regret, rows));
        }
    }

    candidates.sort_by(|left, right| {
        left.1
            .total_cmp(&right.1)
            .then_with(|| left.0.cmp(&right.0))
    });

    let selected = candidates.first();
    let mut world_ids: Vec<String> = worlds.iter().map(|world| world.world_id.clone()).collect();
    world_ids.sort();

    ObservationPartition {
        observation_id: observation_id.to_owned(),
        world_ids,
        selected_policy_id: selected.map(|entry| entry.0.clone()),
        worst_case_regret: selected.map(|entry| entry.1),
        world_regrets: selected.map(|entry| entry.2.clone()).unwrap_or_default(),
    }
}

/// Evaluate sensing requests by finite-set minimax regret. The candidate sees
/// outcome IDs only; hidden missingness causes are retained in audit receipts.
pub fn assess_active_sensing(
    config: &ActiveSensingConfig,
    worlds: &[DecisionWorld],
    sensors: &[SensorAction],
) -> Result<ActiveSensingReport, String> {
    validate_config(config)?;
    let oracle = validate_worlds(worlds)?;
    let baseline = baseline_no_sensing(worlds, &oracle);

    let mut sensor_ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(sensors.len());

    for sensor in sensors {
        if !sensor_ids.insert(sensor.sensor_id.as_str()) {
            return Err(format!("duplicate sensor action {}", sensor.sensor_id));
        }
        let predictions = validate_sensor(sensor, worlds)?;
        let energy_cost_service_units = sensor.energy_cost_kwh * sensor.service_units_per_kwh;
        let maximum_latency_service_loss = sensor
            .predictions
            .iter()
            .map(|prediction| prediction.latency_service_loss_per_tick)
            .fold(0.0_f64, f64::max)
            * f64::from(sensor.latency_ticks);
        let maximum_total_penalty_service_units =
            energy_cost_service_units + maximum_latency_service_loss;

        if !maximum_total_penalty_service_units.is_finite() {
            return Err(format!(
                "non-finite sensing penalty in {}",
                sensor.sensor_id
            ));
        }

        let affordable = sensor.energy_cost_kwh <= config.maximum_energy_kwh + config.tie_tolerance
            && sensor.latency_ticks <= config.maximum_latency_ticks
            && maximum_total_penalty_service_units
                <= config.maximum_penalty_service_units + config.tie_tolerance;

        let mut grouped: BTreeMap<String, Vec<&DecisionWorld>> = BTreeMap::new();

        for world in worlds {
            let prediction = predictions
                .get(&world.world_id)
                .expect("complete prediction map");
            grouped
                .entry(prediction.observation_id.clone())
                .or_default()
                .push(world);
        }

        let partitions: Vec<ObservationPartition> = grouped
            .iter()
            .map(|(outcome, members)| {
                assess_partition(
                    outcome,
                    members,
                    &predictions,
                    &oracle,
                    energy_cost_service_units,
                    sensor.latency_ticks,
                )
            })
            .collect();

        let unresolved = partitions
            .iter()
            .any(|partition| partition.worst_case_regret.is_none());
        let worst_case_post_sensing_regret = if unresolved {
            None
        } else {
            partitions
                .iter()
                .filter_map(|partition| partition.worst_case_regret)
                .reduce(f64::max)
        };
        let worst_case_regret_reduction =
            match (baseline.worst_case_regret, worst_case_post_sensing_regret) {
                (Some(before), Some(after)) => Some(before - after),
                _ => None,
            };

        let status = if !affordable {
            SensorStatus::OverBudget
        } else if worst_case_post_sensing_regret.is_none() {
            SensorStatus::NoFeasiblePolicy
        } else {
            SensorStatus::Evaluated
        };
        let resolves_no_sensing_infeasibility =
            baseline.common_feasible_policy_count == 0 && status == SensorStatus::Evaluated;

        assessments.push(SensorAssessment {
            sensor_id: sensor.sensor_id.clone(),
            protocol_digest: sensor.protocol_digest.clone(),
            status,
            affordable,
            energy_cost_kwh: sensor.energy_cost_kwh,
            service_units_per_kwh: sensor.service_units_per_kwh,
            energy_cost_service_units,
            latency_ticks: sensor.latency_ticks,
            maximum_latency_service_loss,
            maximum_total_penalty_service_units,
            worst_case_post_sensing_regret,
            worst_case_regret_reduction,
            resolves_no_sensing_infeasibility,
            partitions,
        });
    }

    assessments.sort_by(|left, right| left.sensor_id.cmp(&right.sensor_id));
    let mut eligible: Vec<&SensorAssessment> = assessments
        .iter()
        .filter(|item| {
            item.status == SensorStatus::Evaluated
                && item.affordable
                && (item.resolves_no_sensing_infeasibility
                    || item
                        .worst_case_regret_reduction
                        .is_some_and(|reduction| reduction > config.tie_tolerance))
        })
        .collect();

    eligible.sort_by(|left, right| {
        right
            .worst_case_regret_reduction
            .unwrap_or(f64::NEG_INFINITY)
            .total_cmp(
                &left
                    .worst_case_regret_reduction
                    .unwrap_or(f64::NEG_INFINITY),
            )
            .then_with(|| {
                left.maximum_total_penalty_service_units
                    .total_cmp(&right.maximum_total_penalty_service_units)
            })
            .then_with(|| left.energy_cost_kwh.total_cmp(&right.energy_cost_kwh))
            .then_with(|| left.sensor_id.cmp(&right.sensor_id))
    });

    Ok(ActiveSensingReport {
        scenario_id: config.scenario_id.clone(),
        source_digest: config.source_digest.clone(),
        baseline,
        recommended_sensor_id: eligible.first().map(|item| item.sensor_id.clone()),
        sensors: assessments,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidatePolicyDecision {
    pub scenario_id: String,
    pub sensor_id: String,
    pub observation_id: String,
    pub action_id: String,
}

/// Export the policy a controller may execute after receiving an observation.
/// This projection deliberately contains no hidden world IDs, world outcomes,
/// missingness causes, prediction receipts or evaluator regret rows.
pub fn candidate_policy_map(report: &ActiveSensingReport) -> Vec<CandidatePolicyDecision> {
    let Some(sensor_id) = report.recommended_sensor_id.as_deref() else {
        return Vec::new();
    };
    let Some(assessment) = report.sensors.iter().find(|item| {
        item.sensor_id == sensor_id && item.status == SensorStatus::Evaluated && item.affordable
    }) else {
        return Vec::new();
    };

    let mut decisions: Vec<CandidatePolicyDecision> = assessment
        .partitions
        .iter()
        .filter_map(|partition| {
            partition
                .selected_policy_id
                .as_ref()
                .map(|policy_id| CandidatePolicyDecision {
                    scenario_id: report.scenario_id.clone(),
                    sensor_id: assessment.sensor_id.clone(),
                    observation_id: partition.observation_id.clone(),
                    action_id: policy_id.clone(),
                })
        })
        .collect();
    decisions.sort_by(|left, right| left.observation_id.cmp(&right.observation_id));
    decisions
}

/// Build evaluator-only provenance for the sensor outcome process. This
/// function is separate from candidate decision assessment; its returned
/// missingness causes must never be supplied to candidate policies.
pub fn evaluator_prediction_audit(
    sensors: &[SensorAction],
) -> Result<Vec<PredictionAuditReceipt>, String> {
    let mut sensor_ids = BTreeSet::new();
    let mut receipts = Vec::new();
    for sensor in sensors {
        if sensor.sensor_id.trim().is_empty() || !sensor_ids.insert(sensor.sensor_id.as_str()) {
            return Err("empty or duplicate sensor ID in evaluator audit".to_owned());
        }
        for prediction in &sensor.predictions {
            if prediction.world_id.trim().is_empty()
                || prediction.world_digest.trim().is_empty()
                || prediction.observation_id.trim().is_empty()
                || prediction.prediction_receipt_digest.trim().is_empty()
            {
                return Err(format!(
                    "invalid evaluator prediction receipt in {}",
                    sensor.sensor_id
                ));
            }
            receipts.push(PredictionAuditReceipt {
                sensor_id: sensor.sensor_id.clone(),
                world_id: prediction.world_id.clone(),
                world_digest: prediction.world_digest.clone(),
                observation_id: prediction.observation_id.clone(),
                prediction_receipt_digest: prediction.prediction_receipt_digest.clone(),
                missing_cause: prediction.missing_cause.clone(),
            });
        }
    }
    receipts.sort_by(|left, right| {
        left.sensor_id
            .cmp(&right.sensor_id)
            .then_with(|| left.world_id.cmp(&right.world_id))
    });
    Ok(receipts)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> ActiveSensingConfig {
        ActiveSensingConfig {
            scenario_id: "three-world-sensor-dual-control-v1".to_owned(),
            source_digest: "sha256-three-world-sensor-source".to_owned(),
            service_unit: "service_units".to_owned(),
            energy_unit: "kWh".to_owned(),
            maximum_energy_kwh: 1.0,
            maximum_latency_ticks: 2,
            maximum_penalty_service_units: 8.0,
            tie_tolerance: 1.0e-9,
        }
    }

    fn world(id: &str, defer: f64, repair: f64) -> DecisionWorld {
        DecisionWorld {
            world_id: id.to_owned(),
            world_digest: format!("sha256-{id}"),
            service_floor: 70.0,
            policies: vec![
                PolicyOutcome {
                    policy_id: "defer".to_owned(),
                    service_units: defer,
                },
                PolicyOutcome {
                    policy_id: "repair".to_owned(),
                    service_units: repair,
                },
            ],
        }
    }

    fn worlds() -> Vec<DecisionWorld> {
        vec![
            world("healthy", 100.0, 85.0),
            world("degraded", 0.0, 80.0),
            world("healthy_sensor_fault", 100.0, 85.0),
        ]
    }

    fn sensor(
        id: &str,
        energy: f64,
        conversion: f64,
        latency: u32,
        predictions: Vec<(&str, &str, Option<MissingCause>, f64)>,
    ) -> SensorAction {
        SensorAction {
            sensor_id: id.to_owned(),
            protocol_digest: format!("sha256-{id}-protocol"),
            energy_cost_kwh: energy,
            service_units_per_kwh: conversion,
            latency_ticks: latency,
            predictions: predictions
                .into_iter()
                .map(
                    |(world_id, observation_id, missing_cause, loss)| SensorPrediction {
                        world_id: world_id.to_owned(),
                        world_digest: format!("sha256-{world_id}"),
                        observation_id: observation_id.to_owned(),
                        missing_cause,
                        latency_service_loss_per_tick: loss,
                        prediction_receipt_digest: format!("sha256-{id}-{world_id}-prediction"),
                    },
                )
                .collect(),
        }
    }

    fn sensors() -> Vec<SensorAction> {
        vec![
            sensor(
                "remote_telemetry",
                0.5,
                2.0,
                2,
                vec![
                    ("healthy", "healthy", None, 1.0),
                    (
                        "degraded",
                        "missing",
                        Some(MissingCause::CommonCausePowerLoss),
                        3.0,
                    ),
                    (
                        "healthy_sensor_fault",
                        "missing",
                        Some(MissingCause::TelemetryNetworkFault),
                        2.0,
                    ),
                ],
            ),
            sensor(
                "independent_field_inspection",
                0.5,
                2.0,
                1,
                vec![
                    ("healthy", "healthy", None, 1.0),
                    ("degraded", "degraded", None, 3.0),
                    ("healthy_sensor_fault", "healthy", None, 1.0),
                ],
            ),
            sensor(
                "uninformative_status_check",
                0.0,
                2.0,
                0,
                vec![
                    ("healthy", "status_unknown", None, 0.0),
                    ("degraded", "status_unknown", None, 0.0),
                    ("healthy_sensor_fault", "status_unknown", None, 0.0),
                ],
            ),
            sensor(
                "over_budget_lab_test",
                2.0,
                2.0,
                1,
                vec![
                    ("healthy", "healthy", None, 1.0),
                    ("degraded", "degraded", None, 3.0),
                    ("healthy_sensor_fault", "healthy", None, 1.0),
                ],
            ),
        ]
    }

    fn close(left: f64, right: f64) {
        assert!((left - right).abs() < 1.0e-8, "{left} != {right}");
    }

    #[test]
    fn candidate_policy_export_contains_only_observation_to_action_mapping() {
        let (config, worlds, sensors) = {
            let config = config();
            let worlds = worlds();
            let sensors = sensors();
            (config, worlds, sensors)
        };
        let report = assess_active_sensing(&config, &worlds, &sensors).unwrap();
        let decisions = candidate_policy_map(&report);
        assert_eq!(decisions.len(), 2);
        assert_eq!(
            decisions[0].scenario_id,
            "three-world-sensor-dual-control-v1"
        );
        assert_eq!(decisions[0].sensor_id, "independent_field_inspection");
        assert_eq!(decisions[0].observation_id, "degraded");
        assert_eq!(decisions[0].action_id, "repair");
        assert_eq!(decisions[1].observation_id, "healthy");
        assert_eq!(decisions[1].action_id, "defer");
    }

    #[test]
    fn candidate_policy_export_is_empty_when_no_sensor_is_recommended() {
        let mut configuration = config();
        configuration.maximum_energy_kwh = 0.0;
        configuration.maximum_penalty_service_units = 0.0;
        let report = assess_active_sensing(&configuration, &worlds(), &sensors()).unwrap();
        assert!(report.recommended_sensor_id.is_none());
        assert!(candidate_policy_map(&report).is_empty());
    }

    #[test]
    fn no_sensing_baseline_is_repair_with_fifteen_units_worst_case_regret() {
        let report = assess_active_sensing(&config(), &worlds(), &sensors()).unwrap();
        assert_eq!(
            report.baseline.selected_policy_id.as_deref(),
            Some("repair")
        );
        close(report.baseline.worst_case_regret.unwrap(), 15.0);
    }

    #[test]
    fn independent_inspection_charges_energy_and_latency_and_reduces_regret() {
        let report = assess_active_sensing(&config(), &worlds(), &sensors()).unwrap();
        let item = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == "independent_field_inspection")
            .unwrap();
        assert_eq!(item.status, SensorStatus::Evaluated);
        close(item.energy_cost_service_units, 1.0);
        close(item.maximum_latency_service_loss, 3.0);
        close(item.maximum_total_penalty_service_units, 4.0);
        close(item.worst_case_post_sensing_regret.unwrap(), 4.0);
        close(item.worst_case_regret_reduction.unwrap(), 11.0);
        assert_eq!(
            report.recommended_sensor_id.as_deref(),
            Some("independent_field_inspection")
        );
    }

    #[test]
    fn remote_missingness_merges_degraded_and_healthy_sensor_fault_worlds() {
        let report = assess_active_sensing(&config(), &worlds(), &sensors()).unwrap();
        let item = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == "remote_telemetry")
            .unwrap();
        let missing = item
            .partitions
            .iter()
            .find(|partition| partition.observation_id == MISSING_OUTCOME)
            .unwrap();
        assert_eq!(
            missing.world_ids,
            vec!["degraded".to_owned(), "healthy_sensor_fault".to_owned()]
        );
        close(item.worst_case_post_sensing_regret.unwrap(), 20.0);
        close(item.worst_case_regret_reduction.unwrap(), -5.0);
        let audit = evaluator_prediction_audit(&sensors()).unwrap();
        let causes: BTreeSet<&str> = audit
            .iter()
            .filter(|receipt| {
                receipt.sensor_id == "remote_telemetry" && receipt.observation_id == MISSING_OUTCOME
            })
            .filter_map(|receipt| receipt.missing_cause.as_ref().map(MissingCause::id))
            .collect();
        assert_eq!(causes.len(), 2);
    }

    #[test]
    fn uninformative_sensor_has_no_decision_value() {
        let report = assess_active_sensing(&config(), &worlds(), &sensors()).unwrap();
        let item = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == "uninformative_status_check")
            .unwrap();
        close(item.worst_case_post_sensing_regret.unwrap(), 15.0);
        close(item.worst_case_regret_reduction.unwrap(), 0.0);
        assert_ne!(
            report.recommended_sensor_id.as_deref(),
            Some("uninformative_status_check")
        );
    }

    #[test]
    fn over_budget_sensor_is_not_recommended_even_if_informative() {
        let report = assess_active_sensing(&config(), &worlds(), &sensors()).unwrap();
        let item = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == "over_budget_lab_test")
            .unwrap();
        assert_eq!(item.status, SensorStatus::OverBudget);
        assert!(!item.affordable);
        assert_ne!(
            report.recommended_sensor_id.as_deref(),
            Some("over_budget_lab_test")
        );
    }

    #[test]
    fn hidden_missingness_cause_never_changes_candidate_partition() {
        let report = assess_active_sensing(&config(), &worlds(), &sensors()).unwrap();
        let item = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == "remote_telemetry")
            .unwrap();
        let missing = item
            .partitions
            .iter()
            .find(|partition| partition.observation_id == "missing")
            .unwrap();
        assert_eq!(missing.world_ids.len(), 2);
        assert_eq!(missing.observation_id, "missing");

        // Hidden cause is available only from the explicitly evaluator-only audit API.
        let audit = evaluator_prediction_audit(&sensors()).unwrap();
        let causes: BTreeSet<&str> = audit
            .iter()
            .filter(|receipt| {
                receipt.sensor_id == "remote_telemetry" && receipt.observation_id == "missing"
            })
            .filter_map(|receipt| receipt.missing_cause.as_ref().map(MissingCause::id))
            .collect();
        assert_eq!(causes.len(), 2);
    }

    #[test]
    fn sensing_can_restore_feasibility_when_no_blind_action_is_common() {
        let mut first = world("world_a", 90.0, 50.0);
        first.policies[0].policy_id = "repair_a".to_owned();
        first.policies[1].policy_id = "repair_b".to_owned();
        let mut second = world("world_b", 50.0, 90.0);
        second.policies[0].policy_id = "repair_a".to_owned();
        second.policies[1].policy_id = "repair_b".to_owned();
        let worlds = vec![first, second];

        let sensor = sensor(
            "discriminating_sensor",
            0.0,
            0.0,
            0,
            vec![("world_a", "a", None, 0.0), ("world_b", "b", None, 0.0)],
        );

        let report = assess_active_sensing(&config(), &worlds, &[sensor]).unwrap();
        assert_eq!(report.baseline.common_feasible_policy_count, 0);
        assert!(report.baseline.selected_policy_id.is_none());
        assert!(report.baseline.worst_case_regret.is_none());

        let item = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == "discriminating_sensor")
            .unwrap();
        assert_eq!(item.status, SensorStatus::Evaluated);
        assert!(item.resolves_no_sensing_infeasibility);
        assert_eq!(item.worst_case_post_sensing_regret, Some(0.0));
        assert!(item.worst_case_regret_reduction.is_none());
        assert_eq!(
            report.recommended_sensor_id.as_deref(),
            Some("discriminating_sensor")
        );
    }

    #[test]
    fn uninformative_sensor_cannot_resolve_no_sensing_infeasibility() {
        let mut first = world("world_a", 90.0, 50.0);
        first.policies[0].policy_id = "repair_a".to_owned();
        first.policies[1].policy_id = "repair_b".to_owned();
        let mut second = world("world_b", 50.0, 90.0);
        second.policies[0].policy_id = "repair_a".to_owned();
        second.policies[1].policy_id = "repair_b".to_owned();
        let worlds = vec![first, second];

        let sensor = sensor(
            "blind_sensor",
            0.0,
            0.0,
            0,
            vec![
                ("world_a", "unknown", None, 0.0),
                ("world_b", "unknown", None, 0.0),
            ],
        );
        let report = assess_active_sensing(&config(), &worlds, &[sensor]).unwrap();
        let item = &report.sensors[0];
        assert_eq!(item.status, SensorStatus::NoFeasiblePolicy);
        assert!(!item.resolves_no_sensing_infeasibility);
        assert!(report.recommended_sensor_id.is_none());
    }

    #[test]
    fn incomplete_or_digest_mismatched_predictions_fail_closed() {
        let mut items = sensors();
        items[0].predictions.pop();
        assert!(assess_active_sensing(&config(), &worlds(), &items).is_err());

        let mut items = sensors();
        items[0].predictions[0].world_digest = "wrong-digest".to_owned();
        assert!(assess_active_sensing(&config(), &worlds(), &items).is_err());
    }

    #[test]
    fn malformed_missingness_state_and_duplicate_sensor_ids_fail_closed() {
        let mut items = sensors();
        items[0].predictions[0].observation_id = "missing".to_owned();
        items[0].predictions[0].missing_cause = None;
        assert!(assess_active_sensing(&config(), &worlds(), &items).is_err());

        let mut duplicate = sensors();
        duplicate.push(duplicate[0].clone());
        assert!(assess_active_sensing(&config(), &worlds(), &duplicate).is_err());
    }

    #[test]
    fn latency_that_breaks_service_floor_produces_no_feasible_policy() {
        let mut items = sensors();
        items[1].latency_ticks = 10;
        let mut limits = config();
        limits.maximum_latency_ticks = 20;
        limits.maximum_penalty_service_units = 100.0;
        let report = assess_active_sensing(&limits, &worlds(), &items).unwrap();
        let item = report
            .sensors
            .iter()
            .find(|item| item.sensor_id == "independent_field_inspection")
            .unwrap();
        assert_eq!(item.status, SensorStatus::NoFeasiblePolicy);
        assert!(item.worst_case_post_sensing_regret.is_none());
        assert_ne!(
            report.recommended_sensor_id.as_deref(),
            Some("independent_field_inspection")
        );
    }
}
