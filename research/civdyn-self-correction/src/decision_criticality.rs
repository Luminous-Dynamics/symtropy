// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decision-relative consequences of model discrepancy.
//!
//! This module does not upgrade an inadequate model. It measures whether a
//! discrepancy-bearing challenge world changes hard feasibility, the optimal
//! policy set or native-unit regret relative to an evidence-qualified source
//! model. Results are finite-model-set consequences, never probabilities.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::model_adequacy::SplitRole;
use crate::policy_sensitivity::{ModelAdequacyState, ModelWorld, ObjectiveDirection};

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionCriticalityConfig {
    pub objective_id: String,
    pub measurement_unit: String,
    pub direction: ObjectiveDirection,
    /// Smallest absolute objective-unit regret considered material.
    pub materiality_threshold: f64,
    /// Absolute objective-unit tolerance used only to retain exact/near ties.
    pub tie_tolerance: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionPolicyOutcome {
    pub policy_id: String,
    pub objective_value: OrderedFloatValue,
    pub source_digest: String,
    pub outcome_receipt_digest: String,
    pub feasible: bool,
    pub violated_constraints: Vec<String>,
}

/// A finite, validated representation of an objective value. The wrapper
/// keeps construction fallible so NaN and infinities cannot enter ordering.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct OrderedFloatValue(u64);

impl OrderedFloatValue {
    pub fn new(value: f64) -> Result<Self, DecisionCriticalityError> {
        if !value.is_finite() {
            return Err(DecisionCriticalityError::NonFiniteObjectiveValue);
        }
        let canonical = if value == 0.0 { 0.0 } else { value };
        Ok(Self(canonical.to_bits()))
    }

    pub fn get(self) -> f64 {
        f64::from_bits(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionWorldEvidence {
    pub model: ModelWorld,
    pub outcomes: Vec<DecisionPolicyOutcome>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionImpact {
    StableWithinDeclaredThreshold,
    RankingChanged,
    FeasibilityChanged,
    RegretExceedsThreshold,
    NoFeasiblePolicy,
    NotAssessed,
}

impl DecisionImpact {
    pub const fn name(self) -> &'static str {
        match self {
            Self::StableWithinDeclaredThreshold => "stable_within_declared_threshold",
            Self::RankingChanged => "ranking_changed",
            Self::FeasibilityChanged => "feasibility_changed",
            Self::RegretExceedsThreshold => "regret_exceeds_threshold",
            Self::NoFeasiblePolicy => "no_feasible_policy",
            Self::NotAssessed => "not_assessed",
        }
    }

    pub const fn is_decision_critical(self) -> bool {
        matches!(
            self,
            Self::RankingChanged
                | Self::FeasibilityChanged
                | Self::RegretExceedsThreshold
                | Self::NoFeasiblePolicy
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PolicyConsequence {
    pub policy_id: String,
    pub reference_value: OrderedFloatValue,
    pub alternative_value: OrderedFloatValue,
    pub reference_source_digest: String,
    pub alternative_source_digest: String,
    pub reference_outcome_receipt_digest: String,
    pub alternative_outcome_receipt_digest: String,
    pub reference_feasible: bool,
    pub alternative_feasible: bool,
    pub reference_violations: Vec<String>,
    pub alternative_violations: Vec<String>,
    /// Regret is absent where the policy is infeasible in the alternative.
    pub regret: Option<OrderedFloatValue>,
    pub reference_optimal: bool,
    pub alternative_optimal: bool,
    pub feasibility_reversed: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionWorldResult {
    pub reference_model_id: String,
    pub reference_model_digest: String,
    pub reference_adequacy_receipt_digest: String,
    pub alternative_model_id: String,
    pub alternative_model_digest: String,
    pub alternative_adequacy_receipt_digest: String,
    pub alternative_adequacy: ModelAdequacyState,
    pub objective_id: String,
    pub measurement_unit: String,
    pub direction: ObjectiveDirection,
    pub materiality_threshold: OrderedFloatValue,
    pub reference_optimal_policy_ids: Vec<String>,
    pub alternative_optimal_policy_ids: Vec<String>,
    pub any_reference_optimal_became_infeasible: bool,
    pub maximum_reference_policy_regret: Option<OrderedFloatValue>,
    pub impact: DecisionImpact,
    pub consequences: Vec<PolicyConsequence>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionCriticalityReport {
    pub reference_model_id: String,
    pub reference_model_digest: String,
    pub objective_id: String,
    pub measurement_unit: String,
    pub direction: ObjectiveDirection,
    pub materiality_threshold: OrderedFloatValue,
    pub alternative_world_count: usize,
    pub decision_critical_world_count: usize,
    pub not_assessed_world_count: usize,
    pub worlds: Vec<DecisionWorldResult>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionCriticalityError {
    InvalidConfiguration,
    NonFiniteObjectiveValue,
    InvalidPolicyIdentity,
    DuplicateModelIdentity { model_id: String },
    DuplicatePolicyIdentity { policy_id: String },
    DuplicateModelDigest { digest: String },
    DuplicateOutcome { model_id: String, policy_id: String },
    IncompleteOutcomeMatrix { model_id: String, expected: usize, actual: usize },
    UnknownAlternativePolicy { model_id: String, policy_id: String },
    ContradictoryFeasibilityReceipt { model_id: String, policy_id: String },
    InvalidModelEvidence { model_id: String },
    ReferenceModelNotQualified,
    ReferenceMustBeValidation,
    AlternativeMustBeChallenge { model_id: String },
    NoReferenceFeasiblePolicy,
    EmptyAlternativeWorlds,
}

impl fmt::Display for DecisionCriticalityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DecisionCriticalityError {}

fn optimal_policy_ids(
    outcomes: &[DecisionPolicyOutcome],
    direction: ObjectiveDirection,
    tie_tolerance: f64,
) -> Vec<String> {
    let feasible: Vec<&DecisionPolicyOutcome> =
        outcomes.iter().filter(|outcome| outcome.feasible).collect();
    let Some(best) = feasible.iter().map(|outcome| outcome.objective_value.get()).reduce(
        |left, right| match direction {
            ObjectiveDirection::Maximize => left.max(right),
            ObjectiveDirection::Minimize => left.min(right),
        },
    ) else {
        return Vec::new();
    };

    let mut result: Vec<String> = feasible
        .into_iter()
        .filter(|outcome| (outcome.objective_value.get() - best).abs() <= tie_tolerance)
        .map(|outcome| outcome.policy_id.clone())
        .collect();
    result.sort();
    result
}

fn regret(
    alternative_best: f64,
    policy_value: f64,
    direction: ObjectiveDirection,
) -> f64 {
    match direction {
        ObjectiveDirection::Maximize => (alternative_best - policy_value).max(0.0),
        ObjectiveDirection::Minimize => (policy_value - alternative_best).max(0.0),
    }
}

fn validate_world<'a>(
    world: &'a DecisionWorldEvidence,
    expected_policy_ids: &BTreeSet<&str>,
    is_reference: bool,
) -> Result<BTreeMap<&'a str, &'a DecisionPolicyOutcome>, DecisionCriticalityError> {
    let model = &world.model;
    if model.id().trim().is_empty()
        || model.model_digest().trim().is_empty()
        || model.adequacy_receipt_digest().trim().is_empty()
        || model.adequacy_model_digest() != model.model_digest()
    {
        return Err(DecisionCriticalityError::InvalidModelEvidence {
            model_id: model.id().to_string(),
        });
    }
    if is_reference {
        if !model.adequacy_required_evidence_complete() {
            return Err(DecisionCriticalityError::InvalidModelEvidence {
                model_id: model.id().to_string(),
            });
        }
        if model.adequacy_state() != ModelAdequacyState::NoPrimaryDiscrepancyDetected {
            return Err(DecisionCriticalityError::ReferenceModelNotQualified);
        }
        if model.adequacy_role() != Some(SplitRole::Validation) {
            return Err(DecisionCriticalityError::ReferenceMustBeValidation);
        }
    } else if matches!(
        model.adequacy_state(),
        ModelAdequacyState::NotAssessed | ModelAdequacyState::OutOfDomain
    ) {
        if model.adequacy_role().is_some_and(|role| role != SplitRole::Challenge) {
            return Err(DecisionCriticalityError::AlternativeMustBeChallenge {
                model_id: model.id().to_string(),
            });
        }
    } else if model.adequacy_role() != Some(SplitRole::Challenge)
        || !model.adequacy_required_evidence_complete()
    {
        return Err(DecisionCriticalityError::AlternativeMustBeChallenge {
            model_id: model.id().to_string(),
        });
    }

    let mut by_policy = BTreeMap::new();
    for outcome in &world.outcomes {
        if outcome.policy_id.trim().is_empty() {
            return Err(DecisionCriticalityError::InvalidPolicyIdentity);
        }
        if !expected_policy_ids.contains(outcome.policy_id.as_str()) {
            return Err(DecisionCriticalityError::UnknownAlternativePolicy {
                model_id: model.id().to_string(),
                policy_id: outcome.policy_id.clone(),
            });
        }
        if outcome.source_digest.trim().is_empty()
            || outcome.outcome_receipt_digest.trim().is_empty()
        {
            return Err(DecisionCriticalityError::InvalidModelEvidence {
                model_id: model.id().to_string(),
            });
        }
        if (outcome.feasible && !outcome.violated_constraints.is_empty())
            || (!outcome.feasible && outcome.violated_constraints.is_empty())
        {
            return Err(DecisionCriticalityError::ContradictoryFeasibilityReceipt {
                model_id: model.id().to_string(),
                policy_id: outcome.policy_id.clone(),
            });
        }
        if by_policy.insert(outcome.policy_id.as_str(), outcome).is_some() {
            return Err(DecisionCriticalityError::DuplicateOutcome {
                model_id: model.id().to_string(),
                policy_id: outcome.policy_id.clone(),
            });
        }
    }
    if by_policy.len() != expected_policy_ids.len() {
        return Err(DecisionCriticalityError::IncompleteOutcomeMatrix {
            model_id: model.id().to_string(),
            expected: expected_policy_ids.len(),
            actual: by_policy.len(),
        });
    }
    Ok(by_policy)
}

/// Compare every held-out challenge model with one evidence-qualified
/// validation reference. All policy rankings, feasibility and regret remain
/// per-world; no model-count probability or pooled score is produced.
pub fn assess_decision_criticality(
    reference: &DecisionWorldEvidence,
    alternatives: &[DecisionWorldEvidence],
    config: &DecisionCriticalityConfig,
) -> Result<DecisionCriticalityReport, DecisionCriticalityError> {
    if alternatives.is_empty()
        || config.objective_id.trim().is_empty()
        || config.measurement_unit.trim().is_empty()
        || !config.materiality_threshold.is_finite()
        || config.materiality_threshold < 0.0
        || !config.tie_tolerance.is_finite()
        || config.tie_tolerance < 0.0
    {
        return Err(if alternatives.is_empty() {
            DecisionCriticalityError::EmptyAlternativeWorlds
        } else {
            DecisionCriticalityError::InvalidConfiguration
        });
    }

    let reference_model_id = reference.model.id().to_string();
    let reference_model_digest = reference.model.model_digest().to_string();
    let mut policy_ids = BTreeSet::new();
    for outcome in &reference.outcomes {
        if outcome.policy_id.trim().is_empty() {
            return Err(DecisionCriticalityError::InvalidPolicyIdentity);
        }
        if !policy_ids.insert(outcome.policy_id.as_str()) {
            return Err(DecisionCriticalityError::DuplicatePolicyIdentity {
                policy_id: outcome.policy_id.clone(),
            });
        }
    }
    if policy_ids.is_empty() {
        return Err(DecisionCriticalityError::InvalidConfiguration);
    }
    let reference_outcomes = validate_world(reference, &policy_ids, true)?;
    let reference_optimal_policy_ids = optimal_policy_ids(
        &reference.outcomes,
        config.direction,
        config.tie_tolerance,
    );
    if reference_optimal_policy_ids.is_empty() {
        return Err(DecisionCriticalityError::NoReferenceFeasiblePolicy);
    }
    let reference_optimal: BTreeSet<&str> = reference_optimal_policy_ids
        .iter()
        .map(String::as_str)
        .collect();

    let mut model_ids = BTreeSet::from([reference_model_id.as_str()]);
    let mut model_digests = BTreeSet::from([reference_model_digest.as_str()]);
    let mut worlds = Vec::with_capacity(alternatives.len());

    for alternative in alternatives {
        let alternative_id = alternative.model.id();
        let alternative_digest = alternative.model.model_digest();
        if !model_ids.insert(alternative_id) {
            return Err(DecisionCriticalityError::DuplicateModelIdentity {
                model_id: alternative_id.to_string(),
            });
        }
        if !model_digests.insert(alternative_digest) {
            return Err(DecisionCriticalityError::DuplicateModelDigest {
                digest: alternative_digest.to_string(),
            });
        }
        let alternative_outcomes = validate_world(alternative, &policy_ids, false)?;
        let alternative_optimal_policy_ids = optimal_policy_ids(
            &alternative.outcomes,
            config.direction,
            config.tie_tolerance,
        );
        let alternative_optimal: BTreeSet<&str> = alternative_optimal_policy_ids
            .iter()
            .map(String::as_str)
            .collect();
        let no_feasible_policy = alternative_optimal_policy_ids.is_empty();

        let mut any_reference_optimal_became_infeasible = false;
        let mut max_regret: Option<f64> = None;
        let mut consequences = Vec::with_capacity(policy_ids.len());
        for policy_id in &policy_ids {
            let reference_outcome = reference_outcomes[policy_id];
            let alternative_outcome = alternative_outcomes[policy_id];
            let is_reference_optimal = reference_optimal.contains(policy_id);
            let is_alternative_optimal = alternative_optimal.contains(policy_id);
            let feasibility_reversed = is_reference_optimal && !alternative_outcome.feasible;
            any_reference_optimal_became_infeasible |= feasibility_reversed;

            let policy_regret = if alternative_outcome.feasible && is_reference_optimal && !no_feasible_policy {
                let alt_best = alternative_optimal
                    .iter()
                    .map(|id| alternative_outcomes[id].objective_value.get())
                    .reduce(|left, right| match config.direction {
                        ObjectiveDirection::Maximize => left.max(right),
                        ObjectiveDirection::Minimize => left.min(right),
                    })
                    .unwrap_or(alternative_outcome.objective_value.get());
                Some(regret(
                    alt_best,
                    alternative_outcome.objective_value.get(),
                    config.direction,
                ))
            } else {
                None
            };
            if let Some(value) = policy_regret {
                max_regret = Some(max_regret.map_or(value, |current| current.max(value)));
            }
            consequences.push(PolicyConsequence {
                policy_id: (*policy_id).to_string(),
                reference_value: reference_outcome.objective_value,
                alternative_value: alternative_outcome.objective_value,
                reference_source_digest: reference_outcome.source_digest.clone(),
                alternative_source_digest: alternative_outcome.source_digest.clone(),
                reference_outcome_receipt_digest: reference_outcome.outcome_receipt_digest.clone(),
                alternative_outcome_receipt_digest: alternative_outcome.outcome_receipt_digest.clone(),
                reference_feasible: reference_outcome.feasible,
                alternative_feasible: alternative_outcome.feasible,
                reference_violations: reference_outcome.violated_constraints.clone(),
                alternative_violations: alternative_outcome.violated_constraints.clone(),
                regret: policy_regret.map(|value| OrderedFloatValue(value.to_bits())),
                reference_optimal: is_reference_optimal,
                alternative_optimal: is_alternative_optimal,
                feasibility_reversed,
            });
        }
        consequences.sort_by(|left, right| left.policy_id.cmp(&right.policy_id));
        let ranking_changed = reference_optimal_policy_ids
            .iter()
            .all(|policy_id| !alternative_optimal.contains(policy_id.as_str()));
        let inadequate = matches!(
            alternative.model.adequacy_state(),
            ModelAdequacyState::NotAssessed | ModelAdequacyState::OutOfDomain
        );
        let impact = if inadequate {
            DecisionImpact::NotAssessed
        } else if no_feasible_policy {
            DecisionImpact::NoFeasiblePolicy
        } else if any_reference_optimal_became_infeasible {
            DecisionImpact::FeasibilityChanged
        } else if ranking_changed {
            DecisionImpact::RankingChanged
        } else if max_regret.is_some_and(|value| value > config.materiality_threshold) {
            DecisionImpact::RegretExceedsThreshold
        } else {
            DecisionImpact::StableWithinDeclaredThreshold
        };

        worlds.push(DecisionWorldResult {
            reference_model_id: reference_model_id.clone(),
            reference_model_digest: reference_model_digest.clone(),
            reference_adequacy_receipt_digest: reference.model.adequacy_receipt_digest().to_string(),
            alternative_model_id: alternative_id.to_string(),
            alternative_model_digest: alternative_digest.to_string(),
            alternative_adequacy_receipt_digest: alternative.model.adequacy_receipt_digest().to_string(),
            alternative_adequacy: alternative.model.adequacy_state(),
            objective_id: config.objective_id.clone(),
            measurement_unit: config.measurement_unit.clone(),
            direction: config.direction,
            materiality_threshold: OrderedFloatValue(config.materiality_threshold.to_bits()),
            reference_optimal_policy_ids: reference_optimal_policy_ids.clone(),
            alternative_optimal_policy_ids,
            any_reference_optimal_became_infeasible,
            maximum_reference_policy_regret: max_regret.map(|value| OrderedFloatValue(value.to_bits())),
            impact,
            consequences,
        });
    }

    worlds.sort_by(|left, right| left.alternative_model_id.cmp(&right.alternative_model_id));
    let decision_critical_world_count = worlds.iter().filter(|world| world.impact.is_decision_critical()).count();
    let not_assessed_world_count = worlds.iter().filter(|world| world.impact == DecisionImpact::NotAssessed).count();

    Ok(DecisionCriticalityReport {
        reference_model_id,
        reference_model_digest,
        objective_id: config.objective_id.clone(),
        measurement_unit: config.measurement_unit.clone(),
        direction: config.direction,
        materiality_threshold: OrderedFloatValue(config.materiality_threshold.to_bits()),
        alternative_world_count: worlds.len(),
        decision_critical_world_count,
        not_assessed_world_count,
        worlds,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model_adequacy::{
        assess_predictive_partition, IndependentUnitKind, ObservedStatistic, PredictiveDraw,
        PredictiveKind, SplitManifest, StatisticRegistry, StatisticSpec,
    };
    use crate::policy_sensitivity::ModelWorld;

    const CALIBRATION: &str = "sha256-decision-calibration";
    const MODEL_BASELINE: &str = "sha256-decision-reference-model";

    fn model_world(
        id: &str,
        model_digest: &str,
        challenge: bool,
        discrepant: bool,
    ) -> ModelWorld {
        let role = if challenge { SplitRole::Challenge } else { SplitRole::Validation };
        let registry = StatisticRegistry {
            version: "decision-adequacy-v1".to_string(),
            statistics: vec![StatisticSpec {
                id: "service-predictor".to_string(),
                construct: "essential service state".to_string(),
                measurement_unit: "service_units".to_string(),
                population: "synthetic-region".to_string(),
                context: "decision-criticality-known-answer".to_string(),
                source_family: "synthetic-decision-source".to_string(),
                source_digest: "sha256-decision-source".to_string(),
                independent_unit: IndependentUnitKind::Region,
                role,
                primary: true,
                required: true,
                minimum_draws: 5,
            }],
        };
        let manifest = SplitManifest {
            id: format!("{id}-split"),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: CALIBRATION.to_string(),
            manifest_digest: format!("sha256-{id}-manifest"),
            calibration_units: vec![format!("{id}-calibration")],
            validation_units: if challenge { vec![format!("{id}-reserved-validation")] } else { vec![format!("{id}-eval")] },
            challenge_units: if challenge { vec![format!("{id}-eval")] } else { Vec::new() },
        };
        let evaluation_unit = format!("{id}-eval");
        let observed = ObservedStatistic {
            statistic_id: "service-predictor".to_string(),
            independent_unit_id: evaluation_unit.clone(),
            measurement_unit: "service_units".to_string(),
            source_digest: "sha256-decision-source".to_string(),
            sample_size: 100,
            value: 5.0,
            measurement_standard_error: Some(0.1),
            observation_digest: format!("sha256-{id}-observation"),
        };
        let base_draws = if discrepant { [0.0, 1.0, 2.0, 3.0, 4.0] } else { [3.0, 4.0, 5.0, 6.0, 7.0] };
        let draws: Vec<PredictiveDraw> = base_draws
            .iter()
            .enumerate()
            .map(|(index, value)| PredictiveDraw {
                kind: PredictiveKind::Split,
                statistic_id: "service-predictor".to_string(),
                independent_unit_id: evaluation_unit.clone(),
                sample_size: 100,
                draw_index: index as u64,
                value: *value,
                model_digest: model_digest.to_string(),
                draw_set_digest: format!("sha256-{id}-draws"),
                calibration_data_digest: Some(CALIBRATION.to_string()),
            })
            .collect();
        let assessment = assess_predictive_partition(
            &registry,
            &manifest,
            &[observed],
            &draws,
            role,
            PredictiveKind::Split,
            0.9,
        )
        .expect("known-answer adequacy assessment");
        ModelWorld::from_adequacy_assessment(
            id,
            format!("family-{id}"),
            model_digest,
            format!("sha256-{id}-adequacy-receipt"),
            &registry,
            &manifest,
            &assessment,
        )
        .expect("model world from actual adequacy receipt")
    }

    fn world(model: ModelWorld, values: [f64; 3], feasible: [bool; 3]) -> DecisionWorldEvidence {
        let ids = ["expand", "maintain", "conserve"];
        DecisionWorldEvidence {
            model,
            outcomes: ids
                .iter()
                .enumerate()
                .map(|(index, id)| DecisionPolicyOutcome {
                    policy_id: (*id).to_string(),
                    objective_value: OrderedFloatValue::new(values[index]).expect("finite objective"),
                    source_digest: format!("sha256-{}-policy-source", model.id()),
                    outcome_receipt_digest: format!("sha256-{}-{}-outcome", model.id(), id),
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

    fn config() -> DecisionCriticalityConfig {
        DecisionCriticalityConfig {
            objective_id: "essential-service-coverage".to_string(),
            measurement_unit: "service_units".to_string(),
            direction: ObjectiveDirection::Maximize,
            materiality_threshold: 10.0,
            tie_tolerance: 0.0,
        }
    }

    fn reference() -> DecisionWorldEvidence {
        world(
            model_world("reference", MODEL_BASELINE, false, false),
            [100.0, 90.0, 80.0],
            [true, true, true],
        )
    }

    #[test]
    fn stable_decision_remains_stable_within_declared_threshold() {
        let alternative = world(
            model_world("stable-challenge", "sha256-stable-challenge", true, false),
            [99.0, 91.0, 79.0],
            [true, true, true],
        );
        let report = assess_decision_criticality(&reference(), &[alternative], &config())
            .expect("stable consequence report");
        assert_eq!(report.worlds[0].impact, DecisionImpact::StableWithinDeclaredThreshold);
        assert_eq!(report.worlds[0].reference_optimal_policy_ids, vec!["expand"]);
        assert_eq!(report.worlds[0].alternative_optimal_policy_ids, vec!["expand"]);
        assert_eq!(report.decision_critical_world_count, 0);
    }

    #[test]
    fn ranking_reversal_reports_native_unit_regret() {
        let alternative = world(
            model_world("ranking-challenge", "sha256-ranking-challenge", true, true),
            [70.0, 95.0, 80.0],
            [true, true, true],
        );
        let report = assess_decision_criticality(&reference(), &[alternative], &config())
            .expect("ranking reversal report");
        let result = &report.worlds[0];
        assert_eq!(result.impact, DecisionImpact::RankingChanged);
        assert_eq!(result.reference_optimal_policy_ids, vec!["expand"]);
        assert_eq!(result.alternative_optimal_policy_ids, vec!["maintain"]);
        assert_eq!(
            result.maximum_reference_policy_regret.expect("regret").get(),
            25.0
        );
    }

    #[test]
    fn feasibility_reversal_is_more_important_than_objective_regret() {
        let alternative = world(
            model_world("feasibility-challenge", "sha256-feasibility-challenge", true, true),
            [120.0, 90.0, 80.0],
            [false, true, true],
        );
        let report = assess_decision_criticality(&reference(), &[alternative], &config())
            .expect("feasibility reversal report");
        let result = &report.worlds[0];
        assert_eq!(result.impact, DecisionImpact::FeasibilityChanged);
        assert!(result.any_reference_optimal_became_infeasible);
        assert!(result.consequences.iter().find(|row| row.policy_id == "expand").unwrap().regret.is_none());
    }

    #[test]
    fn no_feasible_policy_is_an_explicit_critical_outcome() {
        let alternative = world(
            model_world("collapse-challenge", "sha256-collapse-challenge", true, true),
            [120.0, 90.0, 80.0],
            [false, false, false],
        );
        let report = assess_decision_criticality(&reference(), &[alternative], &config())
            .expect("feasibility collapse report");
        assert_eq!(report.worlds[0].impact, DecisionImpact::NoFeasiblePolicy);
    }

    #[test]
    fn unassessed_challenge_cannot_produce_a_stability_claim() {
        let alternative = world(
            model_world("unknown-challenge", "sha256-unknown-challenge", true, false),
            [100.0, 90.0, 80.0],
            [true, true, true],
        );
        // The helper creates a fully assessed model; a separate out-of-domain
        // record is conservatively represented as an explicit non-claim.
        let mut candidate = alternative;
        candidate.model = ModelWorld::out_of_domain(
            "unknown-challenge",
            "family-unknown",
            "sha256-unknown-challenge",
            "sha256-unknown-receipt",
            "target outside registered applicability domain",
        )
        .expect("explicit out-of-domain world");
        let report = assess_decision_criticality(&reference(), &[candidate], &config())
            .expect("not-assessed consequence report");
        assert_eq!(report.worlds[0].impact, DecisionImpact::NotAssessed);
        assert_eq!(report.not_assessed_world_count, 1);
    }

    #[test]
    fn exact_ties_remain_sets_without_arbitrary_tie_breaking() {
        let reference = world(
            model_world("tie-reference", "sha256-tie-reference", false, false),
            [100.0, 100.0, 80.0],
            [true, true, true],
        );
        let alternative = world(
            model_world("tie-challenge", "sha256-tie-challenge", true, false),
            [99.0, 100.0, 80.0],
            [true, true, true],
        );
        let report = assess_decision_criticality(&reference, &[alternative], &config())
            .expect("tie report");
        assert_eq!(
            report.worlds[0].reference_optimal_policy_ids,
            vec!["expand", "maintain"]
        );
        assert_eq!(report.worlds[0].impact, DecisionImpact::RankingChanged);
        assert_eq!(
            report.worlds[0].alternative_optimal_policy_ids,
            vec!["maintain"]
        );
    }

    #[test]
    fn rejects_incomplete_duplicate_and_contradictory_outcomes() {
        let mut incomplete = world(
            model_world("incomplete-challenge", "sha256-incomplete", true, false),
            [100.0, 90.0, 80.0],
            [true, true, true],
        );
        incomplete.outcomes.pop();
        assert!(matches!(
            assess_decision_criticality(&reference(), &[incomplete], &config()),
            Err(DecisionCriticalityError::IncompleteOutcomeMatrix { .. })
        ));

        let mut duplicate = world(
            model_world("duplicate-challenge", "sha256-duplicate", true, false),
            [100.0, 90.0, 80.0],
            [true, true, true],
        );
        duplicate.outcomes[1].policy_id = duplicate.outcomes[0].policy_id.clone();
        assert!(matches!(
            assess_decision_criticality(&reference(), &[duplicate], &config()),
            Err(DecisionCriticalityError::DuplicateOutcome { .. })
        ));

        let mut contradictory = world(
            model_world("contradictory-challenge", "sha256-contradictory", true, false),
            [100.0, 90.0, 80.0],
            [true, true, true],
        );
        contradictory.outcomes[0].feasible = false;
        assert!(matches!(
            assess_decision_criticality(&reference(), &[contradictory], &config()),
            Err(DecisionCriticalityError::ContradictoryFeasibilityReceipt { .. })
        ));
    }

    #[test]
    fn rejects_unqualified_reference_model() {
        let bad_reference = world(
            model_world("bad-reference", "sha256-bad-reference", false, true),
            [100.0, 90.0, 80.0],
            [true, true, true],
        );
        let alternative = world(
            model_world("alt", "sha256-alt", true, false),
            [100.0, 90.0, 80.0],
            [true, true, true],
        );
        assert_eq!(
            assess_decision_criticality(&bad_reference, &[alternative], &config()).unwrap_err(),
            DecisionCriticalityError::ReferenceModelNotQualified
        );
    }
}
