// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Prior-free, finite-model-set experiment selection by worst-case regret
//! reduction. Version 0.1 uses deterministic categorical experiment outcomes.
//! Model counts are never treated as probabilities.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::decision_criticality::OrderedFloatValue;
use crate::model_adequacy::SplitRole;
use crate::policy_sensitivity::{ModelAdequacyState, ModelWorld, ObjectiveDirection};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionPolicyState {
    pub policy_id: String,
    pub objective_value: OrderedFloatValue,
    pub source_digest: String,
    pub outcome_receipt_digest: String,
    pub feasible: bool,
    pub violated_constraints: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionModelWorld {
    pub model: ModelWorld,
    pub policies: Vec<DecisionPolicyState>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentOutcomePrediction {
    pub model_id: String,
    pub model_digest: String,
    pub outcome_id: String,
    /// Digest of the model-specific predicted outcome receipt.
    pub prediction_receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentSpec {
    pub id: String,
    pub protocol_digest: String,
    pub cost: f64,
    pub predictions: Vec<ExperimentOutcomePrediction>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionVoiConfig {
    pub objective_id: String,
    pub measurement_unit: String,
    pub direction: ObjectiveDirection,
    pub cost_unit: String,
    pub maximum_experiment_cost: f64,
    pub tie_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MinimaxDecision {
    pub optimal_policy_ids: Vec<String>,
    pub worst_case_regret: Option<OrderedFloatValue>,
    pub common_feasible_policy_count: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentPredictionReceipt {
    pub model_id: String,
    pub model_digest: String,
    pub outcome_id: String,
    pub prediction_receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionVoiModelReceipt {
    pub model_id: String,
    pub model_digest: String,
    pub adequacy_state: ModelAdequacyState,
    pub adequacy_role: SplitRole,
    pub adequacy_receipt_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentOutcomePartition {
    pub outcome_id: String,
    pub model_ids: Vec<String>,
    pub minimax_optimal_policy_ids: Vec<String>,
    pub minimax_regret: Option<OrderedFloatValue>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ExperimentAssessmentStatus {
    Evaluated,
    OverBudget,
    NotAssessed,
}

impl ExperimentAssessmentStatus {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Evaluated => "evaluated",
            Self::OverBudget => "over_budget",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExperimentValueAssessment {
    pub experiment_id: String,
    pub protocol_digest: String,
    pub predictions: Vec<ExperimentPredictionReceipt>,
    pub cost: OrderedFloatValue,
    pub affordable: bool,
    pub status: ExperimentAssessmentStatus,
    pub model_pairs_separated: usize,
    pub model_pairs_total: usize,
    pub worst_case_post_experiment_regret: Option<OrderedFloatValue>,
    pub worst_case_regret_reduction: Option<OrderedFloatValue>,
    pub partitions: Vec<ExperimentOutcomePartition>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionVoiReport {
    pub objective_id: String,
    pub measurement_unit: String,
    pub direction: ObjectiveDirection,
    pub cost_unit: String,
    pub model_receipts: Vec<DecisionVoiModelReceipt>,
    pub maximum_experiment_cost: OrderedFloatValue,
    pub source_validation_world_count: usize,
    pub held_out_challenge_world_count: usize,
    pub current_minimax_decision: MinimaxDecision,
    pub recommended_experiment_id: Option<String>,
    pub experiments: Vec<ExperimentValueAssessment>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionVoiError {
    InvalidConfiguration,
    EmptyModelSet,
    EmptyExperimentSet,
    InvalidModelEvidence { model_id: String },
    DuplicateModelId { model_id: String },
    DuplicateModelDigest { digest: String },
    MissingSourceValidationWorld,
    MissingHeldOutChallengeWorld,
    InvalidPolicyIdentity,
    DuplicatePolicyIdentity { policy_id: String },
    UnknownPolicy { model_id: String, policy_id: String },
    DuplicatePolicyOutcome { model_id: String, policy_id: String },
    IncompletePolicyMatrix { model_id: String, expected: usize, actual: usize },
    ContradictoryFeasibilityReceipt { model_id: String, policy_id: String },
    InvalidExperimentIdentity,
    DuplicateExperimentIdentity { experiment_id: String },
    InvalidExperimentCost { experiment_id: String },
    DuplicatePrediction { experiment_id: String, model_id: String },
    MissingPrediction { experiment_id: String, model_id: String },
    UnknownPredictionModel { experiment_id: String, model_id: String },
    PredictionModelDigestMismatch { experiment_id: String, model_id: String },
    MissingPredictionReceipt { experiment_id: String, model_id: String },
    CountOverflow,
}

impl fmt::Display for DecisionVoiError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DecisionVoiError {}

fn validate_worlds(
    worlds: &[DecisionModelWorld],
) -> Result<(BTreeSet<&str>, usize, usize), DecisionVoiError> {
    if worlds.is_empty() {
        return Err(DecisionVoiError::EmptyModelSet);
    }

    let mut model_ids = BTreeSet::new();
    let mut model_digests = BTreeSet::new();
    let mut policy_ids = BTreeSet::new();
    let mut source_count = 0_usize;
    let mut challenge_count = 0_usize;

    for (index, world) in worlds.iter().enumerate() {
        let model = &world.model;
        if model.id().trim().is_empty()
            || model.model_digest().trim().is_empty()
            || model.adequacy_receipt_digest().trim().is_empty()
            || model.adequacy_model_digest() != model.model_digest()
            || !model.adequacy_required_evidence_complete()
            || matches!(
                model.adequacy_state(),
                ModelAdequacyState::NotAssessed | ModelAdequacyState::OutOfDomain
            )
        {
            return Err(DecisionVoiError::InvalidModelEvidence {
                model_id: model.id().to_string(),
            });
        }
        if !model_ids.insert(model.id()) {
            return Err(DecisionVoiError::DuplicateModelId {
                model_id: model.id().to_string(),
            });
        }
        if !model_digests.insert(model.model_digest()) {
            return Err(DecisionVoiError::DuplicateModelDigest {
                digest: model.model_digest().to_string(),
            });
        }
        match model.adequacy_role() {
            Some(SplitRole::Validation) => source_count += 1,
            Some(SplitRole::Challenge) => challenge_count += 1,
            Some(SplitRole::Calibration) | None => {
                return Err(DecisionVoiError::InvalidModelEvidence {
                    model_id: model.id().to_string(),
                });
            }
        }

        let mut local_policy_ids = BTreeSet::new();
        for policy in &world.policies {
            if policy.policy_id.trim().is_empty()
                || policy.source_digest.trim().is_empty()
                || policy.outcome_receipt_digest.trim().is_empty()
            {
                return Err(DecisionVoiError::InvalidPolicyIdentity);
            }
            if policy.feasible && !policy.violated_constraints.is_empty()
                || !policy.feasible && policy.violated_constraints.is_empty()
            {
                return Err(DecisionVoiError::ContradictoryFeasibilityReceipt {
                    model_id: model.id().to_string(),
                    policy_id: policy.policy_id.clone(),
                });
            }
            if !local_policy_ids.insert(policy.policy_id.as_str()) {
                return Err(DecisionVoiError::DuplicatePolicyOutcome {
                    model_id: model.id().to_string(),
                    policy_id: policy.policy_id.clone(),
                });
            }
            if index == 0 {
                policy_ids.insert(policy.policy_id.as_str());
            }
        }
        if index > 0 {
            for policy_id in &local_policy_ids {
                if !policy_ids.contains(policy_id) {
                    return Err(DecisionVoiError::UnknownPolicy {
                        model_id: model.id().to_string(),
                        policy_id: (*policy_id).to_string(),
                    });
                }
            }
            if local_policy_ids.len() != policy_ids.len() {
                return Err(DecisionVoiError::IncompletePolicyMatrix {
                    model_id: model.id().to_string(),
                    expected: policy_ids.len(),
                    actual: local_policy_ids.len(),
                });
            }
        }
    }

    if source_count == 0 {
        return Err(DecisionVoiError::MissingSourceValidationWorld);
    }
    if challenge_count == 0 {
        return Err(DecisionVoiError::MissingHeldOutChallengeWorld);
    }
    if policy_ids.is_empty() {
        return Err(DecisionVoiError::InvalidPolicyIdentity);
    }

    Ok((policy_ids, source_count, challenge_count))
}

fn minimax_decision(
    worlds: &[&DecisionModelWorld],
    policy_ids: &BTreeSet<&str>,
    direction: ObjectiveDirection,
    tie_tolerance: f64,
) -> Result<MinimaxDecision, DecisionVoiError> {
    let policy_maps: Vec<BTreeMap<&str, &DecisionPolicyState>> = worlds
        .iter()
        .map(|world| {
            world
                .policies
                .iter()
                .map(|policy| (policy.policy_id.as_str(), policy))
                .collect()
        })
        .collect();

    let mut candidate_regrets = Vec::new();
    for policy_id in policy_ids {
        if policy_maps
            .iter()
            .any(|map| !map.get(policy_id).is_some_and(|policy| policy.feasible))
        {
            continue;
        }
        let mut worst_regret = 0.0_f64;
        for map in &policy_maps {
            let feasible_values: Vec<f64> = map
                .values()
                .filter(|policy| policy.feasible)
                .map(|policy| policy.objective_value.get())
                .collect();
            let Some(best) = feasible_values.into_iter().reduce(|left, right| match direction {
                ObjectiveDirection::Maximize => left.max(right),
                ObjectiveDirection::Minimize => left.min(right),
            }) else {
                continue;
            };
            let value = map[policy_id].objective_value.get();
            let regret = match direction {
                ObjectiveDirection::Maximize => (best - value).max(0.0),
                ObjectiveDirection::Minimize => (value - best).max(0.0),
            };
            worst_regret = worst_regret.max(regret);
        }
        candidate_regrets.push(((*policy_id).to_string(), worst_regret));
    }

    if candidate_regrets.is_empty() {
        return Ok(MinimaxDecision {
            optimal_policy_ids: Vec::new(),
            worst_case_regret: None,
            common_feasible_policy_count: 0,
        });
    }
    let best_regret = candidate_regrets
        .iter()
        .map(|(_, regret)| *regret)
        .fold(f64::INFINITY, f64::min);
    let mut optimal_policy_ids: Vec<String> = candidate_regrets
        .iter()
        .filter(|(_, regret)| (*regret - best_regret).abs() <= tie_tolerance)
        .map(|(policy_id, _)| policy_id.clone())
        .collect();
    optimal_policy_ids.sort();
    Ok(MinimaxDecision {
        optimal_policy_ids,
        worst_case_regret: Some(OrderedFloatValue::new(best_regret).map_err(|_| {
            DecisionVoiError::InvalidConfiguration
        })?),
        common_feasible_policy_count: candidate_regrets.len(),
    })
}

/// Evaluate deterministic experiments using only the declared finite model
/// set. The experiment's worst-case post-test regret is the maximum of the
/// minimax regrets over all non-empty outcome-compatible world subsets.
pub fn assess_decision_value_of_information(
    worlds: &[DecisionModelWorld],
    experiments: &[ExperimentSpec],
    config: &DecisionVoiConfig,
) -> Result<DecisionVoiReport, DecisionVoiError> {
    if config.objective_id.trim().is_empty()
        || config.measurement_unit.trim().is_empty()
        || config.cost_unit.trim().is_empty()
        || !config.maximum_experiment_cost.is_finite()
        || config.maximum_experiment_cost < 0.0
        || !config.tie_tolerance.is_finite()
        || config.tie_tolerance < 0.0
    {
        return Err(DecisionVoiError::InvalidConfiguration);
    }
    if experiments.is_empty() {
        return Err(DecisionVoiError::EmptyExperimentSet);
    }

    let (policy_ids, source_count, challenge_count) = validate_worlds(worlds)?;
    let world_lookup: BTreeMap<&str, &DecisionModelWorld> = worlds
        .iter()
        .map(|world| (world.model.id(), world))
        .collect();
    let current_minimax_decision = minimax_decision(
        &worlds.iter().collect::<Vec<_>>(),
        &policy_ids,
        config.direction,
        config.tie_tolerance,
    )
    .map_err(|error| error)?;

    let mut experiment_ids = BTreeSet::new();
    let mut assessments = Vec::with_capacity(experiments.len());
    let model_pair_total = worlds
        .len()
        .checked_mul(worlds.len().saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or(DecisionVoiError::CountOverflow)?;

    for experiment in experiments {
        if experiment.id.trim().is_empty() || experiment.protocol_digest.trim().is_empty() {
            return Err(DecisionVoiError::InvalidExperimentIdentity);
        }
        if !experiment_ids.insert(experiment.id.as_str()) {
            return Err(DecisionVoiError::DuplicateExperimentIdentity {
                experiment_id: experiment.id.clone(),
            });
        }
        if !experiment.cost.is_finite() || experiment.cost < 0.0 {
            return Err(DecisionVoiError::InvalidExperimentCost {
                experiment_id: experiment.id.clone(),
            });
        }

        let mut predictions = BTreeMap::new();
        for prediction in &experiment.predictions {
            let Some(world) = world_lookup.get(prediction.model_id.as_str()) else {
                return Err(DecisionVoiError::UnknownPredictionModel {
                    experiment_id: experiment.id.clone(),
                    model_id: prediction.model_id.clone(),
                });
            };
            if prediction.model_digest != world.model.model_digest() {
                return Err(DecisionVoiError::PredictionModelDigestMismatch {
                    experiment_id: experiment.id.clone(),
                    model_id: prediction.model_id.clone(),
                });
            }
            if prediction.outcome_id.trim().is_empty()
                || prediction.prediction_receipt_digest.trim().is_empty()
            {
                return Err(DecisionVoiError::MissingPredictionReceipt {
                    experiment_id: experiment.id.clone(),
                    model_id: prediction.model_id.clone(),
                });
            }
            if predictions.insert(prediction.model_id.as_str(), prediction).is_some() {
                return Err(DecisionVoiError::DuplicatePrediction {
                    experiment_id: experiment.id.clone(),
                    model_id: prediction.model_id.clone(),
                });
            }
        }
        if predictions.len() != worlds.len() {
            let missing = worlds
                .iter()
                .find(|world| !predictions.contains_key(world.model.id()))
                .map(|world| world.model.id().to_string())
                .unwrap_or_else(|| "unknown".to_string());
            return Err(DecisionVoiError::MissingPrediction {
                experiment_id: experiment.id.clone(),
                model_id: missing,
            });
        }

        let mut partitions: BTreeMap<&str, Vec<&DecisionModelWorld>> = BTreeMap::new();
        for world in worlds {
            let prediction = predictions[world.model.id()];
            partitions
                .entry(prediction.outcome_id.as_str())
                .or_default()
                .push(world);
        }

        let mut partition_results = Vec::with_capacity(partitions.len());
        let mut worst_post_regret = Some(0.0_f64);
        let mut every_partition_has_robust_action = true;
        for (outcome_id, partition_worlds) in &partitions {
            let decision = minimax_decision(
                partition_worlds,
                &policy_ids,
                config.direction,
                config.tie_tolerance,
            )?;
            if let Some(regret) = decision.worst_case_regret {
                if every_partition_has_robust_action {
                    worst_post_regret =
                        Some(worst_post_regret.unwrap_or(0.0).max(regret.get()));
                }
            } else {
                every_partition_has_robust_action = false;
                worst_post_regret = None;
            }
            let mut model_ids: Vec<String> = partition_worlds
                .iter()
                .map(|world| world.model.id().to_string())
                .collect();
            model_ids.sort();
            partition_results.push(ExperimentOutcomePartition {
                outcome_id: (*outcome_id).to_string(),
                model_ids,
                minimax_optimal_policy_ids: decision.optimal_policy_ids,
                minimax_regret: decision.worst_case_regret,
            });
        }

        let within_partition_pairs = partitions.values().try_fold(0_usize, |sum, group| {
            let pairs = group
                .len()
                .checked_mul(group.len().saturating_sub(1))
                .and_then(|value| value.checked_div(2))
                .ok_or(DecisionVoiError::CountOverflow)?;
            sum.checked_add(pairs).ok_or(DecisionVoiError::CountOverflow)
        })?;
        let model_pairs_separated = model_pair_total
            .checked_sub(within_partition_pairs)
            .ok_or(DecisionVoiError::CountOverflow)?;

        let worst_post_value = if every_partition_has_robust_action {
            worst_post_regret
        } else {
            None
        };
        let regret_reduction = match (
            current_minimax_decision.worst_case_regret,
            worst_post_value,
        ) {
            (Some(current), Some(after)) => {
                Some(OrderedFloatValue::new((current.get() - after).max(0.0)).map_err(
                    |_| DecisionVoiError::InvalidConfiguration,
                )?)
            }
            _ => None,
        };

        let affordable = experiment.cost <= config.maximum_experiment_cost;
        assessments.push(ExperimentValueAssessment {
            experiment_id: experiment.id.clone(),
            protocol_digest: experiment.protocol_digest.clone(),
            predictions: predictions
                .iter()
                .map(|(model_id, prediction)| ExperimentPredictionReceipt {
                    model_id: (*model_id).to_string(),
                    model_digest: prediction.model_digest.clone(),
                    outcome_id: prediction.outcome_id.clone(),
                    prediction_receipt_digest: prediction.prediction_receipt_digest.clone(),
                })
                .collect(),
            cost: OrderedFloatValue::new(experiment.cost)
                .map_err(|_| DecisionVoiError::InvalidExperimentCost {
                    experiment_id: experiment.id.clone(),
                })?,
            affordable,
            status: if affordable {
                ExperimentAssessmentStatus::Evaluated
            } else {
                ExperimentAssessmentStatus::OverBudget
            },
            model_pairs_separated,
            model_pairs_total: model_pair_total,
            worst_case_post_experiment_regret: worst_post_value
                .map(|value| OrderedFloatValue::new(value).expect("finite minimax regret")),
            worst_case_regret_reduction: regret_reduction,
            partitions: partition_results,
        });
    }

    assessments.sort_by(|left, right| left.experiment_id.cmp(&right.experiment_id));
    let mut recommended: Option<&ExperimentValueAssessment> = None;
    for assessment in assessments.iter().filter(|assessment| {
        assessment.affordable
            && assessment
                .worst_case_regret_reduction
                .is_some_and(|reduction| reduction.get() > 0.0)
    }) {
        let replace = match recommended {
            None => true,
            Some(current) => {
                let candidate_reduction = assessment
                    .worst_case_regret_reduction
                    .expect("filtered positive regret reduction")
                    .get();
                let current_reduction = current
                    .worst_case_regret_reduction
                    .expect("recommended reduction")
                    .get();
                candidate_reduction > current_reduction
                    || (candidate_reduction == current_reduction
                        && (assessment.cost.get() < current.cost.get()
                            || (assessment.cost == current.cost
                                && assessment.experiment_id < current.experiment_id)))
            }
        };
        if replace {
            recommended = Some(assessment);
        }
    }

    let mut model_receipts: Vec<DecisionVoiModelReceipt> = worlds
        .iter()
        .map(|world| DecisionVoiModelReceipt {
            model_id: world.model.id().to_string(),
            model_digest: world.model.model_digest().to_string(),
            adequacy_state: world.model.adequacy_state(),
            adequacy_role: world.model.adequacy_role().expect("validated model role"),
            adequacy_receipt_digest: world.model.adequacy_receipt_digest().to_string(),
        })
        .collect();
    model_receipts.sort_by(|left, right| left.model_id.cmp(&right.model_id));

    Ok(DecisionVoiReport {
        objective_id: config.objective_id.clone(),
        measurement_unit: config.measurement_unit.clone(),
        direction: config.direction,
        cost_unit: config.cost_unit.clone(),
        model_receipts,
        maximum_experiment_cost: OrderedFloatValue::new(config.maximum_experiment_cost)
            .map_err(|_| DecisionVoiError::InvalidConfiguration)?,
        source_validation_world_count: source_count,
        held_out_challenge_world_count: challenge_count,
        current_minimax_decision,
        recommended_experiment_id: recommended.map(|experiment| experiment.experiment_id.clone()),
        experiments: assessments,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_adequacy::{
        assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
        PredictiveKind, SplitManifest, StatisticRegistry, StatisticSpec,
    };

    const CALIBRATION: &str = "sha256-voi-calibration";
    const SOURCE: &str = "sha256-voi-source";

    fn model(
        id: &str,
        digest: &str,
        role: SplitRole,
        discrepancy: bool,
    ) -> ModelWorld {
        let registry = StatisticRegistry {
            version: "voi-registry-v1".to_string(),
            statistics: vec![StatisticSpec {
                id: "service-fit".to_string(),
                construct: "essential service".to_string(),
                measurement_unit: "service_units".to_string(),
                population: "synthetic-district".to_string(),
                context: "decision-voi-fixture".to_string(),
                source_family: "synthetic-voi-source".to_string(),
                source_digest: SOURCE.to_string(),
                independent_unit: IndependentUnitKind::Region,
                role,
                primary: true,
                required: true,
                minimum_draws: 5,
            }],
        };
        let evaluation_unit = format!("{id}-unit");
        let manifest = SplitManifest {
            id: format!("{id}-split"),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: CALIBRATION.to_string(),
            manifest_digest: format!("sha256-{id}-manifest"),
            calibration_units: vec![format!("{id}-calibration")],
            validation_units: if role == SplitRole::Validation {
                vec![evaluation_unit.clone()]
            } else {
                vec![format!("{id}-reserved-validation")]
            },
            challenge_units: if role == SplitRole::Challenge {
                vec![evaluation_unit.clone()]
            } else {
                Vec::new()
            },
        };
        let observation = ObservedStatistic {
            statistic_id: "service-fit".to_string(),
            independent_unit_id: evaluation_unit.clone(),
            measurement_unit: "service_units".to_string(),
            source_digest: SOURCE.to_string(),
            sample_size: 100,
            value: 5.0,
            measurement_standard_error: Some(0.1),
            observation_digest: format!("sha256-{id}-observation"),
        };
        let values = if discrepancy {
            [0.0, 1.0, 2.0, 3.0, 4.0]
        } else {
            [3.0, 4.0, 5.0, 6.0, 7.0]
        };
        let draws: Vec<PredictiveDraw> = values
            .iter()
            .enumerate()
            .map(|(index, value)| PredictiveDraw {
                kind: PredictiveKind::Split,
                statistic_id: "service-fit".to_string(),
                independent_unit_id: evaluation_unit.clone(),
                sample_size: 100,
                draw_index: index as u64,
                value: *value,
                model_digest: digest.to_string(),
                draw_set_digest: format!("sha256-{id}-draws"),
                calibration_data_digest: Some(CALIBRATION.to_string()),
            })
            .collect();
        let assessment = assess_predictive_partition(
            &registry,
            &manifest,
            &[observation],
            &draws,
            role,
            PredictiveKind::Split,
            0.9,
        )
        .expect("known-answer model adequacy");
        ModelWorld::from_adequacy_assessment(
            id,
            format!("family-{id}"),
            digest,
            format!("sha256-{id}-adequacy"),
            &registry,
            &manifest,
            &assessment,
        )
        .expect("model from actual adequacy assessment")
    }

    fn world(
        model: ModelWorld,
        values: [f64; 3],
        feasible: [bool; 3],
    ) -> DecisionModelWorld {
        let ids = ["expand", "maintain", "conserve"];
        let model_id = model.id().to_string();
        DecisionModelWorld {
            model,
            policies: ids
                .iter()
                .enumerate()
                .map(|(index, id)| DecisionPolicyState {
                    policy_id: (*id).to_string(),
                    objective_value: OrderedFloatValue::new(values[index]).expect("finite value"),
                    source_digest: SOURCE.to_string(),
                    outcome_receipt_digest: format!("sha256-{model_id}-{id}-policy"),
                    feasible: feasible[index],
                    violated_constraints: if feasible[index] {
                        Vec::new()
                    } else {
                        vec!["water-reserve-floor".to_string()]
                    },
                })
                .collect(),
        }
    }

    fn worlds() -> Vec<DecisionModelWorld> {
        vec![
            world(
                model("source", "sha256-voi-source-model", SplitRole::Validation, false),
                [100.0, 90.0, 70.0],
                [true, true, true],
            ),
            world(
                model("alternative", "sha256-voi-alternative-model", SplitRole::Challenge, true),
                [120.0, 80.0, 100.0],
                [false, true, true],
            ),
        ]
    }

    fn prediction(model_id: &str, model_digest: &str, outcome_id: &str, experiment: &str) -> ExperimentOutcomePrediction {
        ExperimentOutcomePrediction {
            model_id: model_id.to_string(),
            model_digest: model_digest.to_string(),
            outcome_id: outcome_id.to_string(),
            prediction_receipt_digest: format!("sha256-{experiment}-{model_id}-prediction"),
        }
    }

    fn config() -> DecisionVoiConfig {
        DecisionVoiConfig {
            objective_id: "essential-service-coverage".to_string(),
            measurement_unit: "service_units".to_string(),
            direction: ObjectiveDirection::Maximize,
            cost_unit: "analyst_hours".to_string(),
            maximum_experiment_cost: 20.0,
            tie_tolerance: 0.0,
        }
    }

    fn informative_experiment(id: &str, cost: f64) -> ExperimentSpec {
        ExperimentSpec {
            id: id.to_string(),
            protocol_digest: format!("sha256-{id}-protocol"),
            cost,
            predictions: vec![
                prediction("source", "sha256-voi-source-model", "no-breach", id),
                prediction("alternative", "sha256-voi-alternative-model", "breach", id),
            ],
        }
    }

    #[test]
    fn discriminating_experiment_reduces_worst_case_regret() {
        let report = assess_decision_value_of_information(
            &worlds(),
            &[informative_experiment("discriminating", 5.0)],
            &config(),
        )
        .expect("decision value report");
        assert_eq!(report.current_minimax_decision.worst_case_regret.unwrap().get(), 20.0);
        let experiment = &report.experiments[0];
        assert_eq!(experiment.model_pairs_separated, 1);
        assert_eq!(experiment.worst_case_post_experiment_regret.unwrap().get(), 0.0);
        assert_eq!(experiment.worst_case_regret_reduction.unwrap().get(), 20.0);
        assert_eq!(report.recommended_experiment_id.as_deref(), Some("discriminating"));
    }

    #[test]
    fn experiment_without_discrimination_has_zero_decision_value() {
        let mut experiment = informative_experiment("uninformative", 5.0);
        experiment.predictions[1].outcome_id = "no-breach".to_string();
        let report = assess_decision_value_of_information(
            &worlds(),
            &[experiment],
            &config(),
        )
        .expect("un-informative experiment report");
        assert_eq!(report.experiments[0].model_pairs_separated, 0);
        assert_eq!(report.experiments[0].worst_case_regret_reduction.unwrap().get(), 0.0);
        assert_eq!(report.recommended_experiment_id, None);
    }

    #[test]
    fn information_without_policy_value_does_not_create_an_experiment_recommendation() {
        let candidate_worlds = vec![
            world(
                model("source-same-decision", "sha256-same-decision-source", SplitRole::Validation, false),
                [100.0, 80.0, 70.0],
                [true, true, true],
            ),
            world(
                model("target-same-decision", "sha256-same-decision-target", SplitRole::Challenge, false),
                [90.0, 75.0, 60.0],
                [true, true, true],
            ),
        ];
        let experiment = ExperimentSpec {
            id: "distinguish-but-no-decision-change".to_string(),
            protocol_digest: "sha256-no-decision-value-protocol".to_string(),
            cost: 1.0,
            predictions: vec![
                prediction("source-same-decision", "sha256-same-decision-source", "a", "no-decision-value"),
                prediction("target-same-decision", "sha256-same-decision-target", "b", "no-decision-value"),
            ],
        };
        let report = assess_decision_value_of_information(
            &candidate_worlds,
            &[experiment],
            &config(),
        )
        .expect("information without policy value report");
        assert_eq!(report.current_minimax_decision.worst_case_regret.unwrap().get(), 0.0);
        assert_eq!(report.experiments[0].model_pairs_separated, 1);
        assert_eq!(report.experiments[0].worst_case_regret_reduction.unwrap().get(), 0.0);
        assert_eq!(report.recommended_experiment_id, None);
    }

    #[test]
    fn over_budget_experiment_is_never_recommended() {
        let report = assess_decision_value_of_information(
            &worlds(),
            &[informative_experiment("too-expensive", 21.0)],
            &config(),
        )
        .expect("over-budget experiment report");
        assert_eq!(report.experiments[0].status, ExperimentAssessmentStatus::OverBudget);
        assert_eq!(report.recommended_experiment_id, None);
    }

    #[test]
    fn tied_regret_reduction_prefers_lower_cost_then_stable_id() {
        let report = assess_decision_value_of_information(
            &worlds(),
            &[
                informative_experiment("zeta", 5.0),
                informative_experiment("alpha", 4.0),
                informative_experiment("beta", 4.0),
            ],
            &config(),
        )
        .expect("experiment tie report");
        assert_eq!(report.recommended_experiment_id.as_deref(), Some("alpha"));
    }

    #[test]
    fn missing_or_mismatched_prediction_receipts_fail_closed() {
        let mut missing = informative_experiment("missing", 5.0);
        missing.predictions.pop();
        assert!(matches!(
            assess_decision_value_of_information(&worlds(), &[missing], &config()),
            Err(DecisionVoiError::MissingPrediction { .. })
        ));

        let mut mismatch = informative_experiment("mismatch", 5.0);
        mismatch.predictions[0].model_digest = "sha256-other-model".to_string();
        assert!(matches!(
            assess_decision_value_of_information(&worlds(), &[mismatch], &config()),
            Err(DecisionVoiError::PredictionModelDigestMismatch { .. })
        ));
    }

    #[test]
    fn duplicate_experiments_and_infeasible_policy_receipts_are_rejected() {
        let experiment = informative_experiment("duplicate", 5.0);
        assert!(matches!(
            assess_decision_value_of_information(
                &worlds(),
                &[experiment.clone(), experiment],
                &config()
            ),
            Err(DecisionVoiError::DuplicateExperimentIdentity { .. })
        ));

        let mut invalid_worlds = worlds();
        invalid_worlds[1].policies[0].feasible = false;
        invalid_worlds[1].policies[0].violated_constraints.clear();
        assert!(matches!(
            assess_decision_value_of_information(
                &invalid_worlds,
                &[informative_experiment("invalid-feasibility", 5.0)],
                &config()
            ),
            Err(DecisionVoiError::ContradictoryFeasibilityReceipt { .. })
        ));
    }
}
