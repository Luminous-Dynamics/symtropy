// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Explicit finite-model envelopes for collision-probability evidence.
//!
//! This module keeps two different uncertainty semantics separate:
//! - an unweighted finite ambiguity set, reported by its minimum lower and
//!   maximum upper conditional probability bounds;
//! - a declared discrete model distribution, reported by its weighted
//!   probability interval.
//!
//! Neither operation proves that the supplied model family contains the true
//! physical model. Completeness is a caller declaration and is recorded in
//! the report. These operations also do not turn Monte Carlo confidence into
//! model uncertainty or vice versa.

use crate::KernelError;

/// The declared coverage status of a finite model family.
///
/// DeclaredComplete means the caller asserts that the finite set contains
/// every model in the intended ambiguity set. This function does not prove
/// that assertion. KnownIncomplete means that only the supplied subset is
/// represented; its range must not be described as a complete robust envelope.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModelSetCoverage {
    /// Caller declares that the finite model set is complete for its purpose.
    DeclaredComplete,
    /// The supplied models are a known-incomplete subset.
    KnownIncomplete,
}

/// A conditional probability interval for one immutable model revision.
///
/// The bounds must already include the numerical / estimator uncertainty
/// claimed by the upstream probability calculation. This type does not
/// reinterpret or validate that upstream confidence statement.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelProbabilityInterval {
    /// Stable identifier for the model family member.
    pub model_id: String,
    /// Immutable revision identifier, parameter digest, or content hash.
    pub model_revision: String,
    /// Lower conditional event-probability bound, in [0, 1].
    pub probability_lower: f64,
    /// Upper conditional event-probability bound, in [0, 1].
    pub probability_upper: f64,
}

/// Robust probability envelope over an unweighted finite model set.
///
/// For a complete declared set this bounds every supplied conditional model
/// probability. For a known-incomplete set it bounds only the admitted subset.
#[derive(Debug, Clone, PartialEq)]
pub struct ModelFamilyEnvelopeReport {
    /// Immutable revision of the model-family declaration.
    pub family_revision: String,
    /// Whether the caller declared the family complete.
    pub coverage: ModelSetCoverage,
    /// Number of distinct model revisions included.
    pub model_count: usize,
    /// Smallest lower conditional probability bound across the supplied set.
    pub probability_lower: f64,
    /// Largest upper conditional probability bound across the supplied set.
    pub probability_upper: f64,
    /// Model attaining the reported lower endpoint.
    pub lower_attaining_model: String,
    /// Model attaining the reported upper endpoint.
    pub upper_attaining_model: String,
    /// True only when the caller declared the finite family complete.
    pub qualifies_declared_family: bool,
}

/// A conditional probability interval with an explicit prior model weight.
///
/// The weight is a probability mass in a declared discrete model distribution,
/// not a heuristic importance weight and not a sample frequency.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightedModelScenario {
    /// Conditional probability interval for this model revision.
    pub estimate: ModelProbabilityInterval,
    /// Declared prior probability mass assigned to this model.
    pub prior_weight: f64,
}

/// Probability interval under an explicitly supplied discrete model prior.
#[derive(Debug, Clone, PartialEq)]
pub struct WeightedModelAverageReport {
    /// Immutable revision of the model-family declaration.
    pub family_revision: String,
    /// Whether the caller declared the prior's model support complete.
    pub coverage: ModelSetCoverage,
    /// Number of distinct model revisions included.
    pub model_count: usize,
    /// Sum of the supplied prior weights before normalization.
    pub supplied_weight_sum: f64,
    /// Outward-expanded lower model-averaged probability bound.
    pub probability_lower: f64,
    /// Outward-expanded upper model-averaged probability bound.
    pub probability_upper: f64,
    /// True only when the caller declared the model support complete.
    pub qualifies_declared_family: bool,
}

/// Compute a min/max probability envelope over a finite unweighted model set.
///
/// For an ambiguity set, this returns [min(model lower), max(model upper)].
/// It does not average the models. The result is complete only when
/// coverage is DeclaredComplete; otherwise it is explicitly a subset
/// envelope. Every input interval is treated as a conditional probability
/// interval supplied by an upstream estimator.
///
/// This operation is deterministic and does not establish physical-model
/// completeness, calibration, or the validity of the input intervals.
pub fn finite_model_family_envelope(
    family_revision: &str,
    coverage: ModelSetCoverage,
    models: &[ModelProbabilityInterval],
) -> Result<ModelFamilyEnvelopeReport, KernelError> {
    validate_family(family_revision, models)?;

    let mut lower_model = &models[0];
    let mut upper_model = &models[0];
    for model in &models[1..] {
        if model.probability_lower < lower_model.probability_lower {
            lower_model = model;
        }
        if model.probability_upper > upper_model.probability_upper {
            upper_model = model;
        }
    }

    Ok(ModelFamilyEnvelopeReport {
        family_revision: family_revision.to_owned(),
        coverage,
        model_count: models.len(),
        probability_lower: lower_model.probability_lower,
        probability_upper: upper_model.probability_upper,
        lower_attaining_model: lower_model.model_id.clone(),
        upper_attaining_model: upper_model.model_id.clone(),
        qualifies_declared_family: coverage == ModelSetCoverage::DeclaredComplete,
    })
}

/// Compute a probability interval under an explicit discrete prior over models.
///
/// This is model averaging, not a robust ambiguity-set envelope. The caller
/// must provide strictly positive prior weights whose sum is within 1e-12 of
/// one. The reported interval encloses the weighted sum of the supplied
/// conditional probability intervals, using adjacent-float expansion for
/// products, accumulation, and division.
///
/// When coverage is KnownIncomplete, the report describes only the supplied
/// prior support and does not qualify a complete model-family result.
pub fn weighted_model_probability_interval(
    family_revision: &str,
    coverage: ModelSetCoverage,
    scenarios: &[WeightedModelScenario],
) -> Result<WeightedModelAverageReport, KernelError> {
    if scenarios.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    let estimates: Vec<ModelProbabilityInterval> =
        scenarios.iter().map(|scenario| scenario.estimate.clone()).collect();
    validate_family(family_revision, &estimates)?;

    let mut supplied_weight_sum = 0.0_f64;
    for scenario in scenarios {
        if !scenario.prior_weight.is_finite() || scenario.prior_weight <= 0.0 {
            return Err(KernelError::InvalidMixtureWeight);
        }
        supplied_weight_sum += scenario.prior_weight;
        if !supplied_weight_sum.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
    }
    if (supplied_weight_sum - 1.0).abs() > 1.0e-12 {
        return Err(KernelError::MixtureWeightsNotNormalized);
    }

    // Enclose the exact sum of the supplied f64 weights.
    let mut weight_sum_lower = 0.0_f64;
    let mut weight_sum_upper = 0.0_f64;
    let mut numerator_lower = 0.0_f64;
    let mut numerator_upper = 0.0_f64;

    for scenario in scenarios {
        let weight = scenario.prior_weight;
        let estimate = &scenario.estimate;

        let product_lower = next_down(weight * estimate.probability_lower).max(0.0);
        let product_upper = next_up(weight * estimate.probability_upper);
        numerator_lower = next_down(numerator_lower + product_lower).max(0.0);
        numerator_upper = next_up(numerator_upper + product_upper);

        weight_sum_lower = next_down(weight_sum_lower + next_down(weight)).max(0.0);
        weight_sum_upper = next_up(weight_sum_upper + next_up(weight));
        if !numerator_upper.is_finite() || !weight_sum_upper.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
    }

    if weight_sum_lower <= 0.0 {
        return Err(KernelError::NumericalOverflow);
    }
    let probability_lower = next_down(numerator_lower / weight_sum_upper).max(0.0);
    let probability_upper = next_up(numerator_upper / weight_sum_lower).min(1.0);
    if !probability_lower.is_finite()
        || !probability_upper.is_finite()
        || probability_lower > probability_upper
    {
        return Err(KernelError::NumericalOverflow);
    }

    Ok(WeightedModelAverageReport {
        family_revision: family_revision.to_owned(),
        coverage,
        model_count: scenarios.len(),
        supplied_weight_sum,
        probability_lower,
        probability_upper,
        qualifies_declared_family: coverage == ModelSetCoverage::DeclaredComplete,
    })
}

fn validate_family(
    family_revision: &str,
    models: &[ModelProbabilityInterval],
) -> Result<(), KernelError> {
    if family_revision.trim().is_empty() {
        return Err(KernelError::InvalidModelRevision);
    }
    if models.is_empty() {
        return Err(KernelError::EmptyDimension);
    }

    let mut identifiers: Vec<&str> = Vec::with_capacity(models.len());
    for model in models {
        if model.model_id.trim().is_empty() {
            return Err(KernelError::InvalidModelIdentifier);
        }
        if model.model_revision.trim().is_empty() {
            return Err(KernelError::InvalidModelRevision);
        }
        if identifiers.contains(&model.model_id.as_str()) {
            return Err(KernelError::DuplicateModelIdentifier);
        }
        identifiers.push(model.model_id.as_str());

        if !model.probability_lower.is_finite()
            || !model.probability_upper.is_finite()
            || model.probability_lower < 0.0
            || model.probability_upper > 1.0
            || model.probability_lower > model.probability_upper
        {
            return Err(KernelError::InvalidProbability);
        }
    }
    Ok(())
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn estimate(
        model_id: &str,
        revision: &str,
        lower: f64,
        upper: f64,
    ) -> ModelProbabilityInterval {
        ModelProbabilityInterval {
            model_id: model_id.to_owned(),
            model_revision: revision.to_owned(),
            probability_lower: lower,
            probability_upper: upper,
        }
    }

    #[test]
    fn finite_ambiguity_set_uses_minimum_lower_and_maximum_upper() {
        let report = finite_model_family_envelope(
            "family-r3",
            ModelSetCoverage::DeclaredComplete,
            &[
                estimate("nominal", "sha-a", 1.0e-6, 1.2e-6),
                estimate("high-drag", "sha-b", 2.0e-6, 2.4e-6),
                estimate("low-drag", "sha-c", 0.3e-6, 0.5e-6),
            ],
        )
        .expect("valid finite model set");
        assert!((report.probability_lower - 0.3e-6).abs() < 1.0e-20);
        assert!((report.probability_upper - 2.4e-6).abs() < 1.0e-20);
        assert_eq!(report.lower_attaining_model, "low-drag");
        assert_eq!(report.upper_attaining_model, "high-drag");
        assert!(report.qualifies_declared_family);
    }

    #[test]
    fn known_incomplete_family_is_explicitly_not_qualified() {
        let report = finite_model_family_envelope(
            "family-r3",
            ModelSetCoverage::KnownIncomplete,
            &[estimate("nominal", "sha-a", 1.0e-6, 1.2e-6)],
        )
        .expect("valid model subset");
        assert_eq!(report.coverage, ModelSetCoverage::KnownIncomplete);
        assert!(!report.qualifies_declared_family);
    }

    #[test]
    fn weighted_model_average_uses_only_explicit_prior_weights() {
        let report = weighted_model_probability_interval(
            "family-r3",
            ModelSetCoverage::DeclaredComplete,
            &[
                WeightedModelScenario {
                    estimate: estimate("a", "sha-a", 1.0e-6, 2.0e-6),
                    prior_weight: 0.25,
                },
                WeightedModelScenario {
                    estimate: estimate("b", "sha-b", 3.0e-6, 4.0e-6),
                    prior_weight: 0.75,
                },
            ],
        )
        .expect("valid explicit prior");
        assert!(report.probability_lower <= 2.5e-6);
        assert!(report.probability_upper >= 3.5e-6);
        assert!(report.qualifies_declared_family);
    }

    #[test]
    fn invalid_or_unnormalized_weights_are_rejected() {
        let invalid = [WeightedModelScenario {
            estimate: estimate("a", "sha-a", 0.1, 0.2),
            prior_weight: 0.0,
        }];
        assert_eq!(
            weighted_model_probability_interval(
                "family-r1",
                ModelSetCoverage::DeclaredComplete,
                &invalid
            ),
            Err(KernelError::InvalidMixtureWeight)
        );

        let unnormalized = [WeightedModelScenario {
            estimate: estimate("a", "sha-a", 0.1, 0.2),
            prior_weight: 0.4,
        }];
        assert_eq!(
            weighted_model_probability_interval(
                "family-r1",
                ModelSetCoverage::DeclaredComplete,
                &unnormalized
            ),
            Err(KernelError::MixtureWeightsNotNormalized)
        );
    }

    #[test]
    fn duplicate_model_ids_and_blank_revisions_are_rejected() {
        assert_eq!(
            finite_model_family_envelope(
                "family-r1",
                ModelSetCoverage::DeclaredComplete,
                &[
                    estimate("same", "sha-a", 0.1, 0.2),
                    estimate("same", "sha-b", 0.2, 0.3),
                ]
            ),
            Err(KernelError::DuplicateModelIdentifier)
        );
        assert_eq!(
            finite_model_family_envelope(
                "family-r1",
                ModelSetCoverage::DeclaredComplete,
                &[estimate("a", " ", 0.1, 0.2)]
            ),
            Err(KernelError::InvalidModelRevision)
        );
    }

    #[test]
    fn invalid_probability_intervals_and_empty_families_fail_closed() {
        assert_eq!(
            finite_model_family_envelope("family-r1", ModelSetCoverage::DeclaredComplete, &[]),
            Err(KernelError::EmptyDimension)
        );
        assert_eq!(
            finite_model_family_envelope(
                "family-r1",
                ModelSetCoverage::DeclaredComplete,
                &[estimate("a", "sha-a", 0.3, 0.2)]
            ),
            Err(KernelError::InvalidProbability)
        );
        assert_eq!(
            finite_model_family_envelope(
                " ",
                ModelSetCoverage::DeclaredComplete,
                &[estimate("a", "sha-a", 0.1, 0.2)]
            ),
            Err(KernelError::InvalidModelRevision)
        );
    }
}
