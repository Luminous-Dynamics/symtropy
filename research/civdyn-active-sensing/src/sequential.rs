// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Two-stage active sensing reference. The first-stage observation can be
//! decision-irrelevant by itself while still unlocking a useful second-stage
//! measurement. All sensing costs are charged against the original physical
//! world oracle; candidate policies see only the concatenated observation
//! history.

use std::collections::{BTreeMap, BTreeSet};

use super::{
    ActiveSensingConfig, ActiveSensingReport, DecisionWorld, NoSensingAssessment, SensorAction,
    SensorPrediction, SensorStatus, TOLERANCE, assess_active_sensing, baseline_no_sensing,
    find_policy, finite_nonnegative, oracle_for, validate_config, validate_sensor, validate_worlds,
};

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialSensorStage {
    pub sensor: SensorAction,
    /// The exact previous-stage observation required before this stage can be
    /// acquired. Stage zero must have no prerequisite.
    pub required_previous_observation: Option<String>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialSensorPlan {
    pub plan_id: String,
    pub protocol_digest: String,
    pub stages: Vec<SequentialSensorStage>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialCandidateDecision {
    pub scenario_id: String,
    pub plan_id: String,
    pub observation_history: String,
    pub action_id: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialWorldRegret {
    pub world_id: String,
    pub selected_policy_id: String,
    pub oracle_service_units: f64,
    pub gross_policy_service_units: f64,
    pub sensing_penalty_service_units: f64,
    pub net_policy_service_units: f64,
    pub regret_service_units: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialObservationPartition {
    /// Candidate-visible observation history, not a latent-world identifier.
    pub observation_history: String,
    pub selected_policy_id: Option<String>,
    pub worst_case_regret: Option<f64>,
    pub world_regrets: Vec<SequentialWorldRegret>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SequentialSensingAssessment {
    pub scenario_id: String,
    pub source_digest: String,
    pub plan_id: String,
    pub protocol_digest: String,
    pub status: SensorStatus,
    pub recommended: bool,
    pub myopic_stage_one_status: SensorStatus,
    pub myopic_stage_one_regret: Option<f64>,
    pub baseline: NoSensingAssessment,
    pub total_energy_kwh: f64,
    pub total_energy_cost_service_units: f64,
    pub total_latency_ticks: u32,
    pub maximum_latency_service_loss: f64,
    pub maximum_total_penalty_service_units: f64,
    pub worst_case_post_sensing_regret: Option<f64>,
    pub worst_case_regret_reduction: Option<f64>,
    pub resolves_no_sensing_infeasibility: bool,
    pub partitions: Vec<SequentialObservationPartition>,
    /// Safe controller-facing projection; evaluator-only world identities
    /// and costs are not part of this record.
    pub candidate_policy: Vec<SequentialCandidateDecision>,
}

fn validate_plan<'a>(
    plan: &'a SequentialSensorPlan,
    worlds: &[DecisionWorld],
) -> Result<Vec<BTreeMap<String, &'a SensorPrediction>>, String> {
    if plan.plan_id.trim().is_empty() || plan.protocol_digest.trim().is_empty() {
        return Err("sequential plan ID and protocol digest must be non-empty".to_owned());
    }
    if plan.stages.len() != 2 {
        return Err("v0.1 sequential active sensing requires exactly two stages".to_owned());
    }
    if plan.stages[0].required_previous_observation.is_some() {
        return Err("the first sensing stage cannot have a previous-stage prerequisite".to_owned());
    }
    let prerequisite = plan.stages[1]
        .required_previous_observation
        .as_deref()
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| {
            "the second sensing stage must declare its prerequisite observation".to_owned()
        })?;

    let mut stage_ids = BTreeSet::new();
    let mut stage_predictions = Vec::with_capacity(2);
    for stage in &plan.stages {
        if !stage_ids.insert(stage.sensor.sensor_id.as_str()) {
            return Err("sequential plan contains duplicate sensor-stage identities".to_owned());
        }
        stage_predictions.push(validate_sensor(&stage.sensor, worlds)?);
    }

    for world in worlds {
        let prior = stage_predictions[0]
            .get(&world.world_id)
            .ok_or_else(|| format!("first-stage result missing for {}", world.world_id))?;
        if prior.observation_id != prerequisite {
            return Err(format!(
                "second stage prerequisite {prerequisite} is not satisfied in {}",
                world.world_id
            ));
        }
    }
    Ok(stage_predictions)
}

fn final_observation_history(
    world_id: &str,
    stages: &[SequentialSensorStage],
    stage_predictions: &[BTreeMap<String, &SensorPrediction>],
) -> Result<String, String> {
    let mut parts = Vec::with_capacity(stages.len());
    for predictions in stage_predictions {
        let prediction = predictions
            .get(world_id)
            .ok_or_else(|| format!("missing sequential prediction for {world_id}"))?;
        parts.push(prediction.observation_id.as_str());
    }
    Ok(parts.join(">"))
}

/// Evaluate a two-stage sensing plan. The first-stage sensor is also assessed
/// alone so the report can distinguish myopic from non-myopic decision value.
pub fn assess_sequential_sensor_plan(
    config: &ActiveSensingConfig,
    worlds: &[DecisionWorld],
    plan: &SequentialSensorPlan,
) -> Result<SequentialSensingAssessment, String> {
    validate_config(config)?;
    let oracle = validate_worlds(worlds)?;
    let stage_predictions = validate_plan(plan, worlds)?;
    let baseline = baseline_no_sensing(worlds, &oracle);

    let first_stage_report: ActiveSensingReport =
        assess_active_sensing(config, worlds, &[plan.stages[0].sensor.clone()])?;
    let first_stage = first_stage_report
        .sensors
        .first()
        .ok_or_else(|| "first-stage myopic assessment missing".to_owned())?;

    let total_energy_kwh: f64 = plan
        .stages
        .iter()
        .map(|stage| stage.sensor.energy_cost_kwh)
        .sum();
    let total_energy_cost_service_units: f64 = plan
        .stages
        .iter()
        .map(|stage| stage.sensor.energy_cost_kwh * stage.sensor.service_units_per_kwh)
        .sum();
    let total_latency_ticks: u32 = plan
        .stages
        .iter()
        .map(|stage| stage.sensor.latency_ticks)
        .sum();

    if !finite_nonnegative(total_energy_kwh) || !finite_nonnegative(total_energy_cost_service_units)
    {
        return Err("non-finite aggregate sequential sensing cost".to_owned());
    }

    let mut latency_loss_by_world: BTreeMap<String, f64> = BTreeMap::new();
    let mut histories: BTreeMap<String, Vec<&DecisionWorld>> = BTreeMap::new();

    for world in worlds {
        let mut latency_loss = 0.0;
        for (index, stage) in plan.stages.iter().enumerate() {
            let prediction = stage_predictions[index]
                .get(&world.world_id)
                .ok_or_else(|| format!("missing stage {index} result for {}", world.world_id))?;
            latency_loss +=
                f64::from(stage.sensor.latency_ticks) * prediction.latency_service_loss_per_tick;
        }
        if !latency_loss.is_finite() {
            return Err(format!("non-finite latency loss in {}", world.world_id));
        }
        latency_loss_by_world.insert(world.world_id.clone(), latency_loss);

        let history = final_observation_history(&world.world_id, &plan.stages, &stage_predictions)?;
        histories.entry(history).or_default().push(world);
    }

    let maximum_latency_service_loss = latency_loss_by_world
        .values()
        .copied()
        .fold(0.0_f64, f64::max);
    let maximum_total_penalty_service_units =
        total_energy_cost_service_units + maximum_latency_service_loss;
    if !maximum_total_penalty_service_units.is_finite() {
        return Err("non-finite total sequential sensing penalty".to_owned());
    }

    let affordable = total_energy_kwh <= config.maximum_energy_kwh + config.tie_tolerance
        && total_latency_ticks <= config.maximum_latency_ticks
        && maximum_total_penalty_service_units
            <= config.maximum_penalty_service_units + config.tie_tolerance;

    let mut partitions = Vec::with_capacity(histories.len());
    for (history, members) in histories {
        let mut policy_ids: Vec<String> = members[0]
            .policies
            .iter()
            .map(|policy| policy.policy_id.clone())
            .collect();
        policy_ids.sort();

        let mut candidates: Vec<(String, f64, Vec<SequentialWorldRegret>)> = Vec::new();
        for policy_id in policy_ids {
            let mut common_feasible = true;
            let mut worst_regret = 0.0_f64;
            let mut rows = Vec::with_capacity(members.len());

            for world in &members {
                let policy = find_policy(world, &policy_id)?;
                let penalty = total_energy_cost_service_units
                    + *latency_loss_by_world
                        .get(&world.world_id)
                        .ok_or_else(|| format!("missing cost receipt for {}", world.world_id))?;
                let net_service = policy.service_units - penalty;
                if net_service + TOLERANCE < world.service_floor {
                    common_feasible = false;
                    break;
                }

                let truth = oracle_for(&oracle, &world.world_id)?.oracle_service_units;
                let regret = (truth - net_service).max(0.0);
                worst_regret = worst_regret.max(regret);
                rows.push(SequentialWorldRegret {
                    world_id: world.world_id.clone(),
                    selected_policy_id: policy_id.clone(),
                    oracle_service_units: truth,
                    gross_policy_service_units: policy.service_units,
                    sensing_penalty_service_units: penalty,
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
        partitions.push(SequentialObservationPartition {
            observation_history: history,
            selected_policy_id: selected.map(|item| item.0.clone()),
            worst_case_regret: selected.map(|item| item.1),
            world_regrets: selected.map(|item| item.2.clone()).unwrap_or_default(),
        });
    }

    partitions.sort_by(|left, right| left.observation_history.cmp(&right.observation_history));
    let has_unresolved_partition = partitions
        .iter()
        .any(|partition| partition.worst_case_regret.is_none());
    let worst_case_post_sensing_regret = if has_unresolved_partition {
        None
    } else {
        partitions
            .iter()
            .filter_map(|partition| partition.worst_case_regret)
            .reduce(f64::max)
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
    let worst_case_regret_reduction =
        match (baseline.worst_case_regret, worst_case_post_sensing_regret) {
            (Some(before), Some(after)) => Some(before - after),
            _ => None,
        };
    let recommended = status == SensorStatus::Evaluated
        && (resolves_no_sensing_infeasibility
            || worst_case_regret_reduction
                .is_some_and(|reduction| reduction > config.tie_tolerance));

    let candidate_policy = if recommended {
        partitions
            .iter()
            .filter_map(|partition| {
                partition
                    .selected_policy_id
                    .as_ref()
                    .map(|action_id| SequentialCandidateDecision {
                        scenario_id: config.scenario_id.clone(),
                        plan_id: plan.plan_id.clone(),
                        observation_history: partition.observation_history.clone(),
                        action_id: action_id.clone(),
                    })
            })
            .collect()
    } else {
        Vec::new()
    };

    Ok(SequentialSensingAssessment {
        scenario_id: config.scenario_id.clone(),
        source_digest: config.source_digest.clone(),
        plan_id: plan.plan_id.clone(),
        protocol_digest: plan.protocol_digest.clone(),
        status,
        recommended,
        myopic_stage_one_status: first_stage.status.clone(),
        myopic_stage_one_regret: first_stage.worst_case_post_sensing_regret,
        baseline,
        total_energy_kwh,
        total_energy_cost_service_units,
        total_latency_ticks,
        maximum_latency_service_loss,
        maximum_total_penalty_service_units,
        worst_case_post_sensing_regret,
        worst_case_regret_reduction,
        resolves_no_sensing_infeasibility,
        partitions,
        candidate_policy,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{MissingCause, PolicyOutcome};

    fn assert_close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() <= 1.0e-8,
            "floating-point mismatch: actual={actual:.12}, expected={expected:.12}"
        );
    }

    fn config() -> ActiveSensingConfig {
        ActiveSensingConfig {
            scenario_id: "nonmyopic-two-stage-sensing-v1".to_owned(),
            source_digest: "sha256-nonmyopic-two-stage-source-v1".to_owned(),
            service_unit: "service_units".to_owned(),
            energy_unit: "kWh".to_owned(),
            maximum_energy_kwh: 1.0,
            maximum_latency_ticks: 2,
            maximum_penalty_service_units: 3.0,
            tie_tolerance: 1.0e-9,
        }
    }

    fn worlds() -> Vec<DecisionWorld> {
        vec![
            DecisionWorld {
                world_id: "world_a".to_owned(),
                world_digest: "sha256-world_a".to_owned(),
                service_floor: 70.0,
                policies: vec![
                    PolicyOutcome {
                        policy_id: "repair_a".to_owned(),
                        service_units: 90.0,
                    },
                    PolicyOutcome {
                        policy_id: "repair_b".to_owned(),
                        service_units: 50.0,
                    },
                ],
            },
            DecisionWorld {
                world_id: "world_b".to_owned(),
                world_digest: "sha256-world_b".to_owned(),
                service_floor: 70.0,
                policies: vec![
                    PolicyOutcome {
                        policy_id: "repair_a".to_owned(),
                        service_units: 50.0,
                    },
                    PolicyOutcome {
                        policy_id: "repair_b".to_owned(),
                        service_units: 90.0,
                    },
                ],
            },
        ]
    }

    fn stage(
        id: &str,
        energy: f64,
        latency: u32,
        required_previous_observation: Option<&str>,
        outcomes: Vec<(&str, &str, f64)>,
    ) -> SequentialSensorStage {
        let worlds = worlds();
        SequentialSensorStage {
            sensor: SensorAction {
                sensor_id: id.to_owned(),
                protocol_digest: format!("sha256-{id}-protocol"),
                energy_cost_kwh: energy,
                service_units_per_kwh: 2.0,
                latency_ticks: latency,
                predictions: outcomes
                    .into_iter()
                    .map(|(world_id, observation_id, loss)| {
                        let world = worlds
                            .iter()
                            .find(|world| world.world_id == world_id)
                            .unwrap();
                        SensorPrediction {
                            world_id: world_id.to_owned(),
                            world_digest: world.world_digest.clone(),
                            observation_id: observation_id.to_owned(),
                            missing_cause: if observation_id == "missing" {
                                Some(MissingCause::SensorDeviceFailure)
                            } else {
                                None
                            },
                            latency_service_loss_per_tick: loss,
                            prediction_receipt_digest: format!("sha256-{id}-{world_id}-receipt"),
                        }
                    })
                    .collect(),
            },
            required_previous_observation: required_previous_observation.map(str::to_owned),
        }
    }

    fn build_plan(second_stage_outcomes: Vec<(&str, &str, f64)>) -> SequentialSensorPlan {
        SequentialSensorPlan {
            plan_id: "setup-then-inspect".to_owned(),
            protocol_digest: "sha256-setup-then-inspect".to_owned(),
            stages: vec![
                stage(
                    "inspection_rig_setup",
                    0.1,
                    1,
                    None,
                    vec![("world_a", "ready", 0.3), ("world_b", "ready", 0.3)],
                ),
                stage(
                    "fine_inspection",
                    0.15,
                    1,
                    Some("ready"),
                    second_stage_outcomes,
                ),
            ],
        }
    }

    #[test]
    fn myopically_useless_setup_enables_nonmyopic_safe_inspection() {
        let plan = build_plan(vec![
            ("world_a", "trace_alpha", 0.4),
            ("world_b", "trace_beta", 0.4),
        ]);
        let result = assess_sequential_sensor_plan(&config(), &worlds(), &plan).unwrap();

        assert_eq!(
            result.myopic_stage_one_status,
            SensorStatus::NoFeasiblePolicy
        );
        assert!(result.myopic_stage_one_regret.is_none());
        assert_eq!(result.baseline.common_feasible_policy_count, 0);
        assert_eq!(result.status, SensorStatus::Evaluated);
        assert!(result.resolves_no_sensing_infeasibility);
        assert!(result.recommended);
        assert_eq!(result.total_energy_kwh, 0.25);
        assert_eq!(result.total_energy_cost_service_units, 0.5);
        assert_eq!(result.total_latency_ticks, 2);
        assert_close(result.maximum_latency_service_loss, 0.7);
        assert_close(result.maximum_total_penalty_service_units, 1.2);
        assert_close(result.worst_case_post_sensing_regret.unwrap(), 1.2);
        assert!(result.worst_case_regret_reduction.is_none());
        assert_eq!(result.candidate_policy.len(), 2);
        assert_eq!(result.candidate_policy[0].action_id, "repair_a");
        assert_eq!(result.candidate_policy[1].action_id, "repair_b");
        assert_eq!(
            result.candidate_policy[0].observation_history,
            "ready>trace_alpha"
        );
    }

    #[test]
    fn an_uninformative_second_measurement_does_not_create_feasibility() {
        let plan = build_plan(vec![
            ("world_a", "same_trace", 0.4),
            ("world_b", "same_trace", 0.4),
        ]);
        let result = assess_sequential_sensor_plan(&config(), &worlds(), &plan).unwrap();
        assert_eq!(result.status, SensorStatus::NoFeasiblePolicy);
        assert!(!result.recommended);
        assert!(result.candidate_policy.is_empty());
    }

    #[test]
    fn over_budget_two_stage_plan_is_not_recommended() {
        let plan = build_plan(vec![
            ("world_a", "trace_alpha", 0.4),
            ("world_b", "trace_beta", 0.4),
        ]);
        let mut limits = config();
        limits.maximum_energy_kwh = 0.2;
        let result = assess_sequential_sensor_plan(&limits, &worlds(), &plan).unwrap();
        assert_eq!(result.status, SensorStatus::OverBudget);
        assert!(!result.recommended);
        assert!(result.candidate_policy.is_empty());
    }

    #[test]
    fn missing_prerequisite_or_duplicate_stage_identity_fails_closed() {
        let mut plan = build_plan(vec![
            ("world_a", "trace_alpha", 0.4),
            ("world_b", "trace_beta", 0.4),
        ]);
        plan.stages[1].required_previous_observation = Some("not_ready".to_owned());
        assert!(assess_sequential_sensor_plan(&config(), &worlds(), &plan).is_err());

        let mut plan = build_plan(vec![
            ("world_a", "trace_alpha", 0.4),
            ("world_b", "trace_beta", 0.4),
        ]);
        plan.stages[1].sensor.sensor_id = plan.stages[0].sensor.sensor_id.clone();
        assert!(assess_sequential_sensor_plan(&config(), &worlds(), &plan).is_err());
    }
}
