// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Finite-sample outcome-distribution sensitivity bounds for causal transport.
//!
//! This module implements the arm-level weighted empirical greedy
//! mass-redistribution procedure described by Asiaee et al. (CLeaR 2026).
//! Generalization weights are supplied by a separately pinned estimator.
//! For each treatment arm, normalized baseline weights p_i are tilted by
//! multipliers in [1/Lambda, Lambda] whose weighted sum is one. The lower
//! (upper) mean puts the additional mass on low (high) outcomes first.
//!
//! This is an empirical sensitivity bound, not a sampling confidence
//! interval. It does not implement the paper's full conditional-X model or
//! its sampling-inference theory.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use crate::causal_transport::{AssumptionDisposition, AssumptionEvidence};

const NORMALIZATION_TOLERANCE: f64 = 1.0e-12;
const FRONTIER_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TreatmentArm {
    Control,
    Treatment,
}

impl TreatmentArm {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Control => "control",
            Self::Treatment => "treatment",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutcomeShiftAssumptions {
    pub source_randomization: AssumptionEvidence,
    pub covariate_overlap: AssumptionEvidence,
    pub no_interference: AssumptionEvidence,
    pub treatment_version_invariance: AssumptionEvidence,
    pub outcome_measurement_invariance: AssumptionEvidence,
}

impl OutcomeShiftAssumptions {
    fn validate(&self) -> Result<(), OutcomeShiftError> {
        let evidence = [
            ("source_randomization", &self.source_randomization),
            ("covariate_overlap", &self.covariate_overlap),
            ("no_interference", &self.no_interference),
            (
                "treatment_version_invariance",
                &self.treatment_version_invariance,
            ),
            (
                "outcome_measurement_invariance",
                &self.outcome_measurement_invariance,
            ),
        ];

        for (name, item) in evidence {
            if item.evidence_digest.trim().is_empty() {
                return Err(OutcomeShiftError::MissingAssumptionEvidenceDigest {
                    assumption: name.to_string(),
                });
            }
            if !matches!(
                item.disposition,
                AssumptionDisposition::SupportedByDesign
                    | AssumptionDisposition::SupportedByEvidence
            ) {
                return Err(OutcomeShiftError::AssumptionNotSupported {
                    assumption: name.to_string(),
                    disposition: item.disposition,
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct WeightedOutcomeObservation {
    pub observation_id: String,
    pub arm: TreatmentArm,
    pub outcome: f64,
    /// Positive weight from a separately identified target-covariate
    /// generalization/participation-weight estimator.
    pub generalization_weight: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeShiftSensitivityInput {
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub protocol_digest: String,
    pub weight_estimator_digest: String,
    pub measurement_unit: String,
    pub outcome_support_lower: f64,
    pub outcome_support_upper: f64,
    pub assumptions: OutcomeShiftAssumptions,
    pub observations: Vec<WeightedOutcomeObservation>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EffectDirectionStatus {
    PositiveEffectRobust,
    NegativeEffectRobust,
    SignAmbiguous,
}

impl EffectDirectionStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::PositiveEffectRobust => "positive_effect_robust",
            Self::NegativeEffectRobust => "negative_effect_robust",
            Self::SignAmbiguous => "sign_ambiguous",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ArmOutcomeShiftBounds {
    pub observation_count: usize,
    pub baseline_weighted_mean: f64,
    pub lower_mean: f64,
    pub upper_mean: f64,
    pub effective_sample_size: f64,
    pub minimum_observed_outcome: f64,
    pub maximum_observed_outcome: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeShiftSensitivityResult {
    pub sensitivity_lambda: f64,
    pub estimand: &'static str,
    pub measurement_unit: String,
    pub baseline_weighted_ate: f64,
    pub target_ate_lower: f64,
    pub target_ate_upper: f64,
    pub effect_direction: EffectDirectionStatus,
    pub control: ArmOutcomeShiftBounds,
    pub treatment: ArmOutcomeShiftBounds,
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub protocol_digest: String,
    pub weight_estimator_digest: String,
    pub outcome_support_lower: f64,
    pub outcome_support_upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeShiftSensitivityFrontier {
    pub results: Vec<OutcomeShiftSensitivityResult>,
    /// First declared Lambda at which zero enters the target ATE identified
    /// interval. This is a sensitivity-grid threshold, not a probability.
    pub sign_tipping_point: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeShiftError {
    MissingIdentity { field: String },
    InvalidMeasurementUnit,
    InvalidOutcomeSupport,
    InvalidLambda,
    LambdaGridMustStartAtOne,
    LambdaGridNotStrictlyIncreasing,
    EmptyObservations,
    EmptyTreatmentArm,
    EmptyControlArm,
    EmptyObservationId,
    DuplicateObservationId { observation_id: String },
    InvalidOutcome { observation_id: String },
    OutcomeOutsideDeclaredSupport { observation_id: String },
    InvalidGeneralizationWeight { observation_id: String },
    MissingAssumptionEvidenceDigest { assumption: String },
    AssumptionNotSupported {
        assumption: String,
        disposition: AssumptionDisposition,
    },
    WeightNormalizationFailed { arm: TreatmentArm },
    ProbabilityNormalizationFailed { arm: TreatmentArm },
    NonFiniteCalculation,
    InvalidIdentifiedInterval,
    NonNestedSensitivityFrontier,
}

impl fmt::Display for OutcomeShiftError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for OutcomeShiftError {}

#[derive(Debug, Clone)]
struct NormalizedObservation<'a> {
    observation_id: &'a str,
    outcome: f64,
    probability: f64,
}

fn compensated_sum(values: impl IntoIterator<Item = f64>) -> f64 {
    let mut sum = 0.0;
    let mut correction = 0.0;
    for value in values {
        let next = sum + value;
        if sum.abs() >= value.abs() {
            correction += (sum - next) + value;
        } else {
            correction += (value - next) + sum;
        }
        sum = next;
    }
    sum + correction
}

fn normalize_arm<'a>(
    observations: &[&'a WeightedOutcomeObservation],
    arm: TreatmentArm,
) -> Result<Vec<NormalizedObservation<'a>>, OutcomeShiftError> {
    if observations.is_empty() {
        return Err(match arm {
            TreatmentArm::Control => OutcomeShiftError::EmptyControlArm,
            TreatmentArm::Treatment => OutcomeShiftError::EmptyTreatmentArm,
        });
    }

    // Canonicalize before summation so every receipt is invariant to caller
    // row order, including floating-point normalization order.
    let mut ordered_observations = observations.to_vec();
    ordered_observations.sort_by(|left, right| {
        left.observation_id.cmp(&right.observation_id)
    });

    let max_weight = ordered_observations
        .iter()
        .map(|observation| observation.generalization_weight)
        .fold(0.0_f64, f64::max);
    if !max_weight.is_finite() || max_weight <= 0.0 {
        return Err(OutcomeShiftError::WeightNormalizationFailed { arm });
    }

    // Scale before summing so valid large finite weights cannot overflow.
    let scaled_weights: Vec<f64> = ordered_observations
        .iter()
        .map(|observation| observation.generalization_weight / max_weight)
        .collect();
    let scaled_total = compensated_sum(scaled_weights.iter().copied());
    if !scaled_total.is_finite() || scaled_total <= 0.0 {
        return Err(OutcomeShiftError::WeightNormalizationFailed { arm });
    }

    let normalized: Vec<NormalizedObservation<'a>> = ordered_observations
        .iter()
        .zip(scaled_weights)
        .map(|(observation, scaled_weight)| NormalizedObservation {
            observation_id: observation.observation_id.as_str(),
            outcome: observation.outcome,
            probability: scaled_weight / scaled_total,
        })
        .collect();

    let probability_total = compensated_sum(
        normalized
            .iter()
            .map(|observation| observation.probability),
    );
    if !probability_total.is_finite()
        || (probability_total - 1.0).abs() > NORMALIZATION_TOLERANCE
    {
        return Err(OutcomeShiftError::WeightNormalizationFailed { arm });
    }

    Ok(normalized)
}

fn tilted_mean(
    normalized: &[NormalizedObservation<'_>],
    lambda: f64,
    maximize: bool,
    arm: TreatmentArm,
) -> Result<f64, OutcomeShiftError> {
    let reciprocal = 1.0 / lambda;
    let mut budget = 1.0 - reciprocal;
    let capacity_scale = lambda - reciprocal;
    let mut tilted: Vec<f64> = normalized
        .iter()
        .map(|observation| observation.probability * reciprocal)
        .collect();

    let mut order: Vec<usize> = (0..normalized.len()).collect();
    order.sort_by(|left, right| {
        let outcome_order = normalized[*left]
            .outcome
            .total_cmp(&normalized[*right].outcome);
        let stable_tie_break = normalized[*left]
            .observation_id
            .cmp(normalized[*right].observation_id);
        if maximize {
            outcome_order
                .reverse()
                .then_with(stable_tie_break)
        } else {
            outcome_order.then_with(stable_tie_break)
        }
    });

    for index in order {
        if budget <= 0.0 {
            break;
        }
        let capacity = normalized[index].probability * capacity_scale;
        if !capacity.is_finite() || capacity < 0.0 {
            return Err(OutcomeShiftError::NonFiniteCalculation);
        }
        let added = capacity.min(budget);
        tilted[index] += added;
        budget = (budget - added).max(0.0);
    }

    if budget > NORMALIZATION_TOLERANCE {
        return Err(OutcomeShiftError::ProbabilityNormalizationFailed { arm });
    }

    let tilted_total = compensated_sum(tilted.iter().copied());
    if !tilted_total.is_finite()
        || tilted_total <= 0.0
        || (tilted_total - 1.0).abs() > NORMALIZATION_TOLERANCE
    {
        return Err(OutcomeShiftError::ProbabilityNormalizationFailed { arm });
    }

    let mean = compensated_sum(
        normalized
            .iter()
            .zip(tilted)
            .map(|(observation, probability)| {
                (probability / tilted_total) * observation.outcome
            }),
    );
    if !mean.is_finite() {
        return Err(OutcomeShiftError::NonFiniteCalculation);
    }
    Ok(mean)
}

fn assess_arm(
    observations: &[&WeightedOutcomeObservation],
    arm: TreatmentArm,
    lambda: f64,
) -> Result<ArmOutcomeShiftBounds, OutcomeShiftError> {
    let normalized = normalize_arm(observations, arm)?;
    let baseline_weighted_mean = compensated_sum(
        normalized
            .iter()
            .map(|observation| observation.probability * observation.outcome),
    );
    let lower_mean = tilted_mean(&normalized, lambda, false, arm)?;
    let upper_mean = tilted_mean(&normalized, lambda, true, arm);
    let upper_mean = upper_mean?;

    let sum_squared_weights = compensated_sum(
        normalized
            .iter()
            .map(|observation| observation.probability * observation.probability),
    );
    let effective_sample_size = 1.0 / sum_squared_weights;
    let minimum_observed_outcome = normalized
        .iter()
        .map(|observation| observation.outcome)
        .fold(f64::INFINITY, f64::min);
    let maximum_observed_outcome = normalized
        .iter()
        .map(|observation| observation.outcome)
        .fold(f64::NEG_INFINITY, f64::max);

    if !baseline_weighted_mean.is_finite()
        || !sum_squared_weights.is_finite()
        || sum_squared_weights <= 0.0
        || !effective_sample_size.is_finite()
        || !minimum_observed_outcome.is_finite()
        || !maximum_observed_outcome.is_finite()
        || lower_mean > upper_mean + FRONTIER_TOLERANCE
    {
        return Err(OutcomeShiftError::NonFiniteCalculation);
    }

    Ok(ArmOutcomeShiftBounds {
        observation_count: normalized.len(),
        baseline_weighted_mean,
        lower_mean,
        upper_mean,
        effective_sample_size,
        minimum_observed_outcome,
        maximum_observed_outcome,
    })
}

fn validate_input(input: &OutcomeShiftSensitivityInput) -> Result<(), OutcomeShiftError> {
    for (field, value) in [
        ("source_data_digest", input.source_data_digest.as_str()),
        (
            "target_population_digest",
            input.target_population_digest.as_str(),
        ),
        ("protocol_digest", input.protocol_digest.as_str()),
        (
            "weight_estimator_digest",
            input.weight_estimator_digest.as_str(),
        ),
    ] {
        if value.trim().is_empty() {
            return Err(OutcomeShiftError::MissingIdentity {
                field: field.to_string(),
            });
        }
    }
    if input.measurement_unit.trim().is_empty() {
        return Err(OutcomeShiftError::InvalidMeasurementUnit);
    }
    if !input.outcome_support_lower.is_finite()
        || !input.outcome_support_upper.is_finite()
        || input.outcome_support_lower >= input.outcome_support_upper
    {
        return Err(OutcomeShiftError::InvalidOutcomeSupport);
    }
    if input.observations.is_empty() {
        return Err(OutcomeShiftError::EmptyObservations);
    }
    input.assumptions.validate()?;

    let mut observation_ids = BTreeSet::new();
    let mut control_count = 0_usize;
    let mut treatment_count = 0_usize;
    for observation in &input.observations {
        if observation.observation_id.trim().is_empty() {
            return Err(OutcomeShiftError::EmptyObservationId);
        }
        if !observation_ids.insert(observation.observation_id.as_str()) {
            return Err(OutcomeShiftError::DuplicateObservationId {
                observation_id: observation.observation_id.clone(),
            });
        }
        if !observation.outcome.is_finite() {
            return Err(OutcomeShiftError::InvalidOutcome {
                observation_id: observation.observation_id.clone(),
            });
        }
        if observation.outcome < input.outcome_support_lower
            || observation.outcome > input.outcome_support_upper
        {
            return Err(OutcomeShiftError::OutcomeOutsideDeclaredSupport {
                observation_id: observation.observation_id.clone(),
            });
        }
        if !observation.generalization_weight.is_finite()
            || observation.generalization_weight <= 0.0
        {
            return Err(OutcomeShiftError::InvalidGeneralizationWeight {
                observation_id: observation.observation_id.clone(),
            });
        }
        match observation.arm {
            TreatmentArm::Control => control_count += 1,
            TreatmentArm::Treatment => treatment_count += 1,
        }
    }
    if control_count == 0 {
        return Err(OutcomeShiftError::EmptyControlArm);
    }
    if treatment_count == 0 {
        return Err(OutcomeShiftError::EmptyTreatmentArm);
    }
    Ok(())
}

/// Compute empirical sharp sensitivity bounds for the target risk/mean
/// difference at one predeclared Lambda >= 1.
pub fn assess_outcome_shift_sensitivity(
    input: &OutcomeShiftSensitivityInput,
    lambda: f64,
) -> Result<OutcomeShiftSensitivityResult, OutcomeShiftError> {
    validate_input(input)?;
    if !lambda.is_finite() || lambda < 1.0 || (1.0 / lambda) == 0.0 {
        return Err(OutcomeShiftError::InvalidLambda);
    }

    let control_observations: Vec<_> = input
        .observations
        .iter()
        .filter(|observation| observation.arm == TreatmentArm::Control)
        .collect();
    let treatment_observations: Vec<_> = input
        .observations
        .iter()
        .filter(|observation| observation.arm == TreatmentArm::Treatment)
        .collect();

    let control = assess_arm(&control_observations, TreatmentArm::Control, lambda)?;
    let treatment = assess_arm(&treatment_observations, TreatmentArm::Treatment, lambda)?;

    let baseline_weighted_ate =
        treatment.baseline_weighted_mean - control.baseline_weighted_mean;
    let target_ate_lower = treatment.lower_mean - control.upper_mean;
    let target_ate_upper = treatment.upper_mean - control.lower_mean;
    if !baseline_weighted_ate.is_finite()
        || !target_ate_lower.is_finite()
        || !target_ate_upper.is_finite()
        || target_ate_lower > target_ate_upper + FRONTIER_TOLERANCE
    {
        return Err(OutcomeShiftError::InvalidIdentifiedInterval);
    }

    let effect_direction = if target_ate_lower > 0.0 {
        EffectDirectionStatus::PositiveEffectRobust
    } else if target_ate_upper < 0.0 {
        EffectDirectionStatus::NegativeEffectRobust
    } else {
        EffectDirectionStatus::SignAmbiguous
    };

    Ok(OutcomeShiftSensitivityResult {
        sensitivity_lambda: lambda,
        estimand: "target_average_treatment_effect_weighted_mean_difference",
        measurement_unit: input.measurement_unit.clone(),
        baseline_weighted_ate,
        target_ate_lower,
        target_ate_upper,
        effect_direction,
        control,
        treatment,
        source_data_digest: input.source_data_digest.clone(),
        target_population_digest: input.target_population_digest.clone(),
        protocol_digest: input.protocol_digest.clone(),
        weight_estimator_digest: input.weight_estimator_digest.clone(),
        outcome_support_lower: input.outcome_support_lower,
        outcome_support_upper: input.outcome_support_upper,
    })
}

/// Evaluate a frozen, strictly increasing sensitivity grid beginning at
/// Lambda=1. Intervals must be nested as Lambda increases.
pub fn assess_outcome_shift_frontier(
    input: &OutcomeShiftSensitivityInput,
    lambdas: &[f64],
) -> Result<OutcomeShiftSensitivityFrontier, OutcomeShiftError> {
    if lambdas.is_empty() || lambdas[0] != 1.0 {
        return Err(OutcomeShiftError::LambdaGridMustStartAtOne);
    }
    if lambdas
        .iter()
        .any(|lambda| !lambda.is_finite() || *lambda < 1.0 || (1.0 / *lambda) == 0.0)
    {
        return Err(OutcomeShiftError::InvalidLambda);
    }
    if lambdas.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(OutcomeShiftError::LambdaGridNotStrictlyIncreasing);
    }

    let mut results = Vec::with_capacity(lambdas.len());
    let mut previous: Option<&OutcomeShiftSensitivityResult> = None;
    let mut sign_tipping_point = None;

    for &lambda in lambdas {
        let result = assess_outcome_shift_sensitivity(input, lambda)?;
        if let Some(prior) = previous {
            if result.target_ate_lower > prior.target_ate_lower + FRONTIER_TOLERANCE
                || result.target_ate_upper < prior.target_ate_upper - FRONTIER_TOLERANCE
            {
                return Err(OutcomeShiftError::NonNestedSensitivityFrontier);
            }
        }
        if sign_tipping_point.is_none()
            && result.effect_direction == EffectDirectionStatus::SignAmbiguous
        {
            sign_tipping_point = Some(lambda);
        }
        results.push(result);
        previous = results.last();
    }

    Ok(OutcomeShiftSensitivityFrontier {
        results,
        sign_tipping_point,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn evidence(disposition: AssumptionDisposition, digest: &str) -> AssumptionEvidence {
        AssumptionEvidence {
            disposition,
            evidence_digest: digest.to_string(),
        }
    }

    fn supported_assumptions() -> OutcomeShiftAssumptions {
        OutcomeShiftAssumptions {
            source_randomization: evidence(
                AssumptionDisposition::SupportedByDesign,
                "sha256-randomization",
            ),
            covariate_overlap: evidence(
                AssumptionDisposition::SupportedByEvidence,
                "sha256-overlap",
            ),
            no_interference: evidence(
                AssumptionDisposition::SupportedByDesign,
                "sha256-no-interference",
            ),
            treatment_version_invariance: evidence(
                AssumptionDisposition::SupportedByEvidence,
                "sha256-treatment-version",
            ),
            outcome_measurement_invariance: evidence(
                AssumptionDisposition::SupportedByEvidence,
                "sha256-outcome-measurement",
            ),
        }
    }

    fn input() -> OutcomeShiftSensitivityInput {
        OutcomeShiftSensitivityInput {
            source_data_digest: "sha256-source-rct".to_string(),
            target_population_digest: "sha256-target-population".to_string(),
            protocol_digest: "sha256-outcome-shift-protocol".to_string(),
            weight_estimator_digest: "sha256-generalization-weights".to_string(),
            measurement_unit: "risk_difference".to_string(),
            outcome_support_lower: 0.0,
            outcome_support_upper: 1.0,
            assumptions: supported_assumptions(),
            observations: vec![
                WeightedOutcomeObservation {
                    observation_id: "control-low".to_string(),
                    arm: TreatmentArm::Control,
                    outcome: 0.1,
                    generalization_weight: 1.0,
                },
                WeightedOutcomeObservation {
                    observation_id: "control-high".to_string(),
                    arm: TreatmentArm::Control,
                    outcome: 0.4,
                    generalization_weight: 1.0,
                },
                WeightedOutcomeObservation {
                    observation_id: "treatment-low".to_string(),
                    arm: TreatmentArm::Treatment,
                    outcome: 0.2,
                    generalization_weight: 1.0,
                },
                WeightedOutcomeObservation {
                    observation_id: "treatment-high".to_string(),
                    arm: TreatmentArm::Treatment,
                    outcome: 0.8,
                    generalization_weight: 1.0,
                },
            ],
        }
    }

    fn close(actual: f64, expected: f64) {
        assert!(
            (actual - expected).abs() < 1.0e-12,
            "expected {expected}, received {actual}"
        );
    }

    #[test]
    fn lambda_one_recovers_the_weighted_transport_point_estimate() {
        let result = assess_outcome_shift_sensitivity(&input(), 1.0)
            .expect("Lambda=1 recovers point estimate");
        close(result.baseline_weighted_ate, 0.25);
        close(result.target_ate_lower, 0.25);
        close(result.target_ate_upper, 0.25);
        close(result.control.lower_mean, 0.25);
        close(result.control.upper_mean, 0.25);
        close(result.treatment.lower_mean, 0.5);
        close(result.treatment.upper_mean, 0.5);
        assert_eq!(
            result.effect_direction,
            EffectDirectionStatus::PositiveEffectRobust
        );
    }

    #[test]
    fn lambda_two_matches_the_greedy_known_answer() {
        let result = assess_outcome_shift_sensitivity(&input(), 2.0)
            .expect("Lambda=2 empirical sensitivity bounds");
        close(result.treatment.lower_mean, 0.35);
        close(result.treatment.upper_mean, 0.65);
        close(result.control.lower_mean, 0.175);
        close(result.control.upper_mean, 0.325);
        close(result.target_ate_lower, 0.025);
        close(result.target_ate_upper, 0.475);
        close(result.treatment.effective_sample_size, 2.0);
        close(result.control.effective_sample_size, 2.0);
        assert_eq!(
            result.effect_direction,
            EffectDirectionStatus::PositiveEffectRobust
        );
    }

    #[test]
    fn frontier_is_nested_and_reports_the_first_sign_tipping_lambda() {
        let frontier = assess_outcome_shift_frontier(&input(), &[1.0, 1.5, 2.0, 3.0, 5.0])
            .expect("monotone sensitivity frontier");
        assert_eq!(frontier.results.len(), 5);
        close(frontier.results[0].target_ate_lower, 0.25);
        close(frontier.results[1].target_ate_lower, 0.10);
        close(frontier.results[2].target_ate_lower, 0.025);
        close(frontier.results[3].target_ate_lower, -0.05);
        close(frontier.results[3].target_ate_upper, 0.55);
        close(frontier.results[4].target_ate_lower, -0.11);
        close(frontier.results[4].target_ate_upper, 0.61);
        assert_eq!(frontier.sign_tipping_point, Some(3.0));
        assert_eq!(
            frontier.results[2].effect_direction,
            EffectDirectionStatus::PositiveEffectRobust
        );
        assert_eq!(
            frontier.results[3].effect_direction,
            EffectDirectionStatus::SignAmbiguous
        );
    }

    #[test]
    fn row_order_does_not_change_bounds_or_tipping_point() {
        let mut shuffled = input();
        shuffled.observations.reverse();
        let first = assess_outcome_shift_frontier(&input(), &[1.0, 1.5, 2.0, 3.0, 5.0])
            .expect("original ordering");
        let second =
            assess_outcome_shift_frontier(&shuffled, &[1.0, 1.5, 2.0, 3.0, 5.0])
                .expect("shuffled ordering");
        assert_eq!(first.sign_tipping_point, second.sign_tipping_point);
        for (left, right) in first.results.iter().zip(second.results.iter()) {
            close(left.target_ate_lower, right.target_ate_lower);
            close(left.target_ate_upper, right.target_ate_upper);
            close(left.treatment.effective_sample_size, right.treatment.effective_sample_size);
        }
    }

    #[test]
    fn unequal_generalization_weights_are_normalized_before_tilting() {
        let mut candidate = input();
        candidate.observations[0].generalization_weight = 3.0;
        candidate.observations[1].generalization_weight = 1.0;
        let result = assess_outcome_shift_sensitivity(&candidate, 1.0)
            .expect("weighted baseline");
        close(result.control.baseline_weighted_mean, 0.175);
        close(result.target_ate_lower, 0.325);
        close(result.target_ate_upper, 0.325);
        close(result.control.effective_sample_size, 1.6);
    }

    #[test]
    fn rejects_nonfinite_or_subunit_lambda_and_invalid_grids() {
        let candidate = input();
        assert_eq!(
            assess_outcome_shift_sensitivity(&candidate, 0.99).unwrap_err(),
            OutcomeShiftError::InvalidLambda
        );
        assert_eq!(
            assess_outcome_shift_sensitivity(&candidate, f64::NAN).unwrap_err(),
            OutcomeShiftError::InvalidLambda
        );
        assert_eq!(
            assess_outcome_shift_frontier(&candidate, &[1.1, 2.0]).unwrap_err(),
            OutcomeShiftError::LambdaGridMustStartAtOne
        );
        assert_eq!(
            assess_outcome_shift_frontier(&candidate, &[1.0, 2.0, 1.5]).unwrap_err(),
            OutcomeShiftError::LambdaGridNotStrictlyIncreasing
        );
    }

    #[test]
    fn rejects_empty_arms_duplicate_ids_invalid_weights_and_outcomes() {
        let mut empty_arm = input();
        empty_arm
            .observations
            .retain(|observation| observation.arm == TreatmentArm::Control);
        assert_eq!(
            assess_outcome_shift_sensitivity(&empty_arm, 1.0).unwrap_err(),
            OutcomeShiftError::EmptyTreatmentArm
        );

        let mut duplicate = input();
        duplicate.observations[1].observation_id = "control-low".to_string();
        assert!(matches!(
            assess_outcome_shift_sensitivity(&duplicate, 1.0),
            Err(OutcomeShiftError::DuplicateObservationId { .. })
        ));

        let mut invalid_weight = input();
        invalid_weight.observations[0].generalization_weight = 0.0;
        assert!(matches!(
            assess_outcome_shift_sensitivity(&invalid_weight, 1.0),
            Err(OutcomeShiftError::InvalidGeneralizationWeight { .. })
        ));

        let mut invalid_outcome = input();
        invalid_outcome.observations[0].outcome = f64::NAN;
        assert!(matches!(
            assess_outcome_shift_sensitivity(&invalid_outcome, 1.0),
            Err(OutcomeShiftError::InvalidOutcome { .. })
        ));
    }

    #[test]
    fn unknown_or_violated_assumptions_block_all_sensitivity_bounds() {
        let mut unknown = input();
        unknown.assumptions.covariate_overlap.disposition = AssumptionDisposition::Unknown;
        assert!(matches!(
            assess_outcome_shift_sensitivity(&unknown, 2.0),
            Err(OutcomeShiftError::AssumptionNotSupported {
                assumption,
                disposition: AssumptionDisposition::Unknown
            }) if assumption == "covariate_overlap"
        ));

        let mut violated = input();
        violated.assumptions.no_interference.disposition = AssumptionDisposition::Violated;
        assert!(matches!(
            assess_outcome_shift_sensitivity(&violated, 2.0),
            Err(OutcomeShiftError::AssumptionNotSupported {
                disposition: AssumptionDisposition::Violated,
                ..
            })
        ));
    }

    #[test]
    fn missing_provenance_or_support_violations_fail_closed() {
        let mut missing = input();
        missing.weight_estimator_digest.clear();
        assert!(matches!(
            assess_outcome_shift_sensitivity(&missing, 1.0),
            Err(OutcomeShiftError::MissingIdentity { field }) if field == "weight_estimator_digest"
        ));

        let mut support = input();
        support.observations[0].outcome = 1.1;
        assert!(matches!(
            assess_outcome_shift_sensitivity(&support, 1.0),
            Err(OutcomeShiftError::OutcomeOutsideDeclaredSupport { .. })
        ));

        let mut missing_receipt = input();
        missing_receipt.assumptions.covariate_overlap.evidence_digest.clear();
        assert!(matches!(
            assess_outcome_shift_sensitivity(&missing_receipt, 1.0),
            Err(OutcomeShiftError::MissingAssumptionEvidenceDigest {
                assumption
            }) if assumption == "covariate_overlap"
        ));
    }
}
