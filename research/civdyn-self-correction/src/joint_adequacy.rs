// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Joint and trajectory predictive diagnostics over source-pinned vector draws.
//!
//! This module preserves vector-valued constructs instead of evaluating only
//! separate scalar components. It reports energy and variogram scores
//! independently. Component order, native units, fixed normalization
//! references, split identity and predictive draw provenance are explicit.
//!
//! These are diagnostics for a declared finite predictive draw set. They do
//! not establish model truth, causal identification, or adequacy beyond the
//! declared split, independent unit and component vector.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

use crate::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};

#[derive(Debug, Clone, PartialEq)]
pub struct JointComponentSpec {
    pub id: String,
    pub measurement_unit: String,
    /// Frozen before the held-out data are inspected.
    pub reference_center: f64,
    pub reference_scale: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointStatisticSpec {
    pub id: String,
    pub source_family: String,
    pub source_digest: String,
    pub population: String,
    pub context: String,
    pub independent_unit: IndependentUnitKind,
    pub role: SplitRole,
    pub primary: bool,
    pub required: bool,
    pub minimum_draws: usize,
    /// Ordered semantic vocabulary. Reordering these components changes the
    /// statistic and must be treated as an identity mismatch.
    pub components: Vec<JointComponentSpec>,
}

impl JointStatisticSpec {
    pub fn validate(&self) -> Result<(), JointAdequacyError> {
        if self.id.trim().is_empty()
            || self.source_family.trim().is_empty()
            || self.source_digest.trim().is_empty()
            || self.population.trim().is_empty()
            || self.context.trim().is_empty()
            || self.minimum_draws < 2
            || self.components.len() < 2
            || (self.primary && !self.required)
        {
            return Err(JointAdequacyError::InvalidStatistic {
                statistic_id: self.id.clone(),
            });
        }

        let mut component_ids = BTreeSet::new();
        for component in &self.components {
            if component.id.trim().is_empty()
                || component.measurement_unit.trim().is_empty()
                || !component.reference_center.is_finite()
                || !component.reference_scale.is_finite()
                || component.reference_scale <= 0.0
                || !component_ids.insert(component.id.as_str())
            {
                return Err(JointAdequacyError::InvalidComponent {
                    statistic_id: self.id.clone(),
                    component_id: component.id.clone(),
                });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointComponentValue {
    pub component_id: String,
    pub measurement_unit: String,
    pub value: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointObservation {
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub sample_size: usize,
    pub source_digest: String,
    pub observation_digest: String,
    pub components: Vec<JointComponentValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointPredictiveDraw {
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub sample_size: usize,
    pub draw_index: u64,
    pub model_digest: String,
    pub draw_set_digest: String,
    pub calibration_data_digest: Option<String>,
    pub components: Vec<JointComponentValue>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct JointMetricConfig {
    /// Energy-score exponent, constrained to (0, 2).
    pub energy_beta: f64,
    /// Variogram exponent, constrained to (0, 2).
    pub variogram_power: f64,
    /// Pair weights in lexicographic component order:
    /// (0,1), (0,2), ..., (d-2,d-1).
    pub pair_weights: Vec<f64>,
}

impl JointMetricConfig {
    pub fn validate(&self, dimensions: usize) -> Result<(), JointAdequacyError> {
        let expected_pairs = dimensions.saturating_mul(dimensions.saturating_sub(1)) / 2;
        if !self.energy_beta.is_finite()
            || self.energy_beta <= 0.0
            || self.energy_beta >= 2.0
            || !self.variogram_power.is_finite()
            || self.variogram_power <= 0.0
            || self.variogram_power >= 2.0
            || self.pair_weights.len() != expected_pairs
            || self
                .pair_weights
                .iter()
                .any(|weight| !weight.is_finite() || *weight < 0.0)
            || !self.pair_weights.iter().any(|weight| *weight > 0.0)
        {
            return Err(JointAdequacyError::InvalidMetricConfig {
                dimensions,
                expected_pair_weights: expected_pairs,
                actual_pair_weights: self.pair_weights.len(),
            });
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct VariogramPairResult {
    pub component_a: String,
    pub component_b: String,
    pub weight: f64,
    pub observed_pair_distance: f64,
    pub predictive_pair_distance: f64,
    pub weighted_contribution: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct JointAdequacyResult {
    pub statistic_id: String,
    pub independent_unit_id: String,
    pub independent_unit_kind: IndependentUnitKind,
    pub sample_size: usize,
    pub predictive_draw_count: usize,
    pub energy_beta: f64,
    /// Lower is better. This remains separate from the variogram score.
    pub energy_score: f64,
    pub variogram_power: f64,
    /// Lower is better. This remains separate from the energy score.
    pub variogram_score: f64,
    pub component_residuals_standardized: Vec<(String, f64)>,
    pub variogram_pairs: Vec<VariogramPairResult>,
    pub model_digest: String,
    pub draw_set_digest: String,
    pub source_digest: String,
    pub observation_digest: String,
    pub calibration_data_digest: String,
    pub split_manifest_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JointAdequacyError {
    InvalidStatistic { statistic_id: String },
    InvalidComponent { statistic_id: String, component_id: String },
    InvalidMetricConfig {
        dimensions: usize,
        expected_pair_weights: usize,
        actual_pair_weights: usize,
    },
    InvalidSplitManifest,
    CalibrationRoleNotAllowed,
    StatisticRoleMismatch { statistic_id: String },
    UnitKindMismatch { statistic_id: String },
    UnknownIndependentUnit { unit_id: String },
    InvalidObservation { statistic_id: String, unit_id: String },
    InvalidComponentVector { statistic_id: String, unit_id: String },
    ComponentOrderMismatch {
        statistic_id: String,
        expected: String,
        actual: String,
    },
    ComponentUnitMismatch {
        statistic_id: String,
        component_id: String,
    },
    MissingPredictiveDraws { statistic_id: String, unit_id: String },
    InsufficientPredictiveDraws {
        statistic_id: String,
        unit_id: String,
        required: usize,
        actual: usize,
    },
    InvalidPredictiveDraw { statistic_id: String, unit_id: String },
    DuplicatePredictiveDraw { statistic_id: String, unit_id: String, draw_index: u64 },
    MixedModelIdentities { statistic_id: String, unit_id: String },
    MixedDrawSetIdentities { statistic_id: String, unit_id: String },
    CalibrationDigestMismatch { statistic_id: String, unit_id: String },
    NonFiniteScore { statistic_id: String, unit_id: String },
}

impl fmt::Display for JointAdequacyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for JointAdequacyError {}

/// Evaluate one held-out vector statistic against sample-size-matched
/// predictive draws conditioned on the exact calibration-data digest.
pub fn assess_joint_predictive_draws(
    statistic: &JointStatisticSpec,
    manifest: &SplitManifest,
    role: SplitRole,
    observation: &JointObservation,
    draws: &[JointPredictiveDraw],
    config: &JointMetricConfig,
) -> Result<JointAdequacyResult, JointAdequacyError> {
    statistic.validate()?;
    manifest
        .validate()
        .map_err(|_| JointAdequacyError::InvalidSplitManifest)?;
    config.validate(statistic.components.len())?;

    if role == SplitRole::Calibration {
        return Err(JointAdequacyError::CalibrationRoleNotAllowed);
    }
    if statistic.role != role || observation.statistic_id != statistic.id {
        return Err(JointAdequacyError::StatisticRoleMismatch {
            statistic_id: statistic.id.clone(),
        });
    }
    if statistic.independent_unit != manifest.independent_unit {
        return Err(JointAdequacyError::UnitKindMismatch {
            statistic_id: statistic.id.clone(),
        });
    }

    let expected_units = match role {
        SplitRole::Calibration => &manifest.calibration_units,
        SplitRole::Validation => &manifest.validation_units,
        SplitRole::Challenge => &manifest.challenge_units,
    };
    if !expected_units
        .iter()
        .any(|unit_id| unit_id == &observation.independent_unit_id)
    {
        return Err(JointAdequacyError::UnknownIndependentUnit {
            unit_id: observation.independent_unit_id.clone(),
        });
    }

    if observation.sample_size == 0
        || observation.source_digest != statistic.source_digest
        || observation.observation_digest.trim().is_empty()
    {
        return Err(JointAdequacyError::InvalidObservation {
            statistic_id: statistic.id.clone(),
            unit_id: observation.independent_unit_id.clone(),
        });
    }
    validate_component_vector(
        statistic,
        &observation.independent_unit_id,
        &observation.components,
    )?;

    let relevant_draws: Vec<&JointPredictiveDraw> = draws
        .iter()
        .filter(|draw| {
            draw.statistic_id == statistic.id
                && draw.independent_unit_id == observation.independent_unit_id
        })
        .collect();
    if relevant_draws.is_empty() {
        return Err(JointAdequacyError::MissingPredictiveDraws {
            statistic_id: statistic.id.clone(),
            unit_id: observation.independent_unit_id.clone(),
        });
    }

    let mut seen_draws = BTreeSet::new();
    let mut model_digest: Option<&str> = None;
    let mut draw_set_digest: Option<&str> = None;
    let mut vectors = Vec::with_capacity(relevant_draws.len());

    for draw in relevant_draws {
        if draw.sample_size != observation.sample_size
            || draw.model_digest.trim().is_empty()
            || draw.draw_set_digest.trim().is_empty()
        {
            return Err(JointAdequacyError::InvalidPredictiveDraw {
                statistic_id: statistic.id.clone(),
                unit_id: observation.independent_unit_id.clone(),
            });
        }
        if draw.calibration_data_digest.as_deref()
            != Some(manifest.calibration_data_digest.as_str())
        {
            return Err(JointAdequacyError::CalibrationDigestMismatch {
                statistic_id: statistic.id.clone(),
                unit_id: observation.independent_unit_id.clone(),
            });
        }
        if !seen_draws.insert(draw.draw_index) {
            return Err(JointAdequacyError::DuplicatePredictiveDraw {
                statistic_id: statistic.id.clone(),
                unit_id: observation.independent_unit_id.clone(),
                draw_index: draw.draw_index,
            });
        }
        if model_digest.is_some_and(|previous| previous != draw.model_digest.as_str()) {
            return Err(JointAdequacyError::MixedModelIdentities {
                statistic_id: statistic.id.clone(),
                unit_id: observation.independent_unit_id.clone(),
            });
        }
        if draw_set_digest.is_some_and(|previous| previous != draw.draw_set_digest.as_str()) {
            return Err(JointAdequacyError::MixedDrawSetIdentities {
                statistic_id: statistic.id.clone(),
                unit_id: observation.independent_unit_id.clone(),
            });
        }
        model_digest = Some(&draw.model_digest);
        draw_set_digest = Some(&draw.draw_set_digest);

        validate_component_vector(
            statistic,
            &observation.independent_unit_id,
            &draw.components,
        )?;

        vectors.push(standardize_component_values(
            statistic,
            &observation.independent_unit_id,
            &draw.components,
        )?);
    }

    if vectors.len() < statistic.minimum_draws {
        return Err(JointAdequacyError::InsufficientPredictiveDraws {
            statistic_id: statistic.id.clone(),
            unit_id: observation.independent_unit_id.clone(),
            required: statistic.minimum_draws,
            actual: vectors.len(),
        });
    }

    let observed = standardize_component_values(
        statistic,
        &observation.independent_unit_id,
        &observation.components,
    )?;

    let energy = energy_score(&vectors, &observed, config.energy_beta);
    let (variogram, pair_results) =
        variogram_score(&vectors, &observed, statistic, config);
    if !energy.is_finite() || !variogram.is_finite() {
        return Err(JointAdequacyError::NonFiniteScore {
            statistic_id: statistic.id.clone(),
            unit_id: observation.independent_unit_id.clone(),
        });
    }

    let component_residuals_standardized = statistic
        .components
        .iter()
        .enumerate()
        .map(|(index, spec)| {
            let predictive_mean = vectors
                .iter()
                .map(|vector| vector[index])
                .sum::<f64>()
                / vectors.len() as f64;
            (spec.id.clone(), observed[index] - predictive_mean)
        })
        .collect();

    if component_residuals_standardized
        .iter()
        .any(|(_, residual)| !residual.is_finite())
    {
        return Err(JointAdequacyError::NonFiniteScore {
            statistic_id: statistic.id.clone(),
            unit_id: observation.independent_unit_id.clone(),
        });
    }

    Ok(JointAdequacyResult {
        statistic_id: statistic.id.clone(),
        independent_unit_id: observation.independent_unit_id.clone(),
        independent_unit_kind: manifest.independent_unit,
        sample_size: observation.sample_size,
        predictive_draw_count: vectors.len(),
        energy_beta: config.energy_beta,
        energy_score: energy,
        variogram_power: config.variogram_power,
        variogram_score: variogram,
        component_residuals_standardized,
        variogram_pairs: pair_results,
        model_digest: model_digest.unwrap_or_default().to_string(),
        draw_set_digest: draw_set_digest.unwrap_or_default().to_string(),
        source_digest: statistic.source_digest.clone(),
        observation_digest: observation.observation_digest.clone(),
        calibration_data_digest: manifest.calibration_data_digest.clone(),
        split_manifest_digest: manifest.manifest_digest.clone(),
    })
}

fn standardize_component_values(
    statistic: &JointStatisticSpec,
    unit_id: &str,
    values: &[JointComponentValue],
) -> Result<Vec<f64>, JointAdequacyError> {
    validate_component_vector(statistic, unit_id, values)?;
    let standardized: Vec<f64> = values
        .iter()
        .zip(&statistic.components)
        .map(|(value, spec)| (value.value - spec.reference_center) / spec.reference_scale)
        .collect();
    if standardized.iter().any(|value| !value.is_finite()) {
        return Err(JointAdequacyError::InvalidComponentVector {
            statistic_id: statistic.id.clone(),
            unit_id: unit_id.to_string(),
        });
    }
    Ok(standardized)
}

fn validate_component_vector(
    statistic: &JointStatisticSpec,
    unit_id: &str,
    values: &[JointComponentValue],
) -> Result<(), JointAdequacyError> {
    if values.len() != statistic.components.len() {
        return Err(JointAdequacyError::InvalidComponentVector {
            statistic_id: statistic.id.clone(),
            unit_id: unit_id.to_string(),
        });
    }

    for (expected, actual) in statistic.components.iter().zip(values) {
        if expected.id != actual.component_id {
            return Err(JointAdequacyError::ComponentOrderMismatch {
                statistic_id: statistic.id.clone(),
                expected: expected.id.clone(),
                actual: actual.component_id.clone(),
            });
        }
        if expected.measurement_unit != actual.measurement_unit {
            return Err(JointAdequacyError::ComponentUnitMismatch {
                statistic_id: statistic.id.clone(),
                component_id: expected.id.clone(),
            });
        }
        if !actual.value.is_finite() {
            return Err(JointAdequacyError::InvalidComponentVector {
                statistic_id: statistic.id.clone(),
                unit_id: unit_id.to_string(),
            });
        }
    }
    Ok(())
}

fn energy_score(vectors: &[Vec<f64>], observation: &[f64], beta: f64) -> f64 {
    let first_term = vectors
        .iter()
        .map(|vector| euclidean_distance(vector, observation).powf(beta))
        .sum::<f64>()
        / vectors.len() as f64;

    let mut pair_sum = 0.0;
    for left in vectors {
        for right in vectors {
            pair_sum += euclidean_distance(left, right).powf(beta);
        }
    }
    let second_term = pair_sum / (2.0 * (vectors.len() * vectors.len()) as f64);
    first_term - second_term
}

fn variogram_score(
    vectors: &[Vec<f64>],
    observation: &[f64],
    statistic: &JointStatisticSpec,
    config: &JointMetricConfig,
) -> (f64, Vec<VariogramPairResult>) {
    let mut results = Vec::with_capacity(config.pair_weights.len());
    let mut pair_index = 0;
    let mut total = 0.0;

    for left in 0..statistic.components.len() {
        for right in (left + 1)..statistic.components.len() {
            let weight = config.pair_weights[pair_index];
            let observed_distance =
                (observation[left] - observation[right]).abs().powf(config.variogram_power);
            let predictive_distance = vectors
                .iter()
                .map(|vector| {
                    (vector[left] - vector[right])
                        .abs()
                        .powf(config.variogram_power)
                })
                .sum::<f64>()
                / vectors.len() as f64;
            let difference = observed_distance - predictive_distance;
            let contribution = weight * difference * difference;
            total += contribution;

            results.push(VariogramPairResult {
                component_a: statistic.components[left].id.clone(),
                component_b: statistic.components[right].id.clone(),
                weight,
                observed_pair_distance: observed_distance,
                predictive_pair_distance: predictive_distance,
                weighted_contribution: contribution,
            });
            pair_index += 1;
        }
    }
    (total, results)
}

fn euclidean_distance(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(a, b)| {
            let difference = a - b;
            difference * difference
        })
        .sum::<f64>()
        .sqrt()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn statistic() -> JointStatisticSpec {
        JointStatisticSpec {
            id: "district-resource-state".to_string(),
            source_family: "synthetic-resource-source".to_string(),
            source_digest: "sha256-source".to_string(),
            population: "synthetic-households".to_string(),
            context: "known-answer-joint-adequacy".to_string(),
            independent_unit: IndependentUnitKind::Region,
            role: SplitRole::Validation,
            primary: true,
            required: true,
            minimum_draws: 2,
            components: vec![
                JointComponentSpec {
                    id: "water".to_string(),
                    measurement_unit: "m3".to_string(),
                    reference_center: 0.0,
                    reference_scale: 1.0,
                },
                JointComponentSpec {
                    id: "energy".to_string(),
                    measurement_unit: "kWh".to_string(),
                    reference_center: 0.0,
                    reference_scale: 1.0,
                },
            ],
        }
    }

    fn manifest() -> SplitManifest {
        SplitManifest {
            id: "joint-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: "sha256-calibration".to_string(),
            manifest_digest: "sha256-manifest".to_string(),
            calibration_units: vec!["region-cal".to_string()],
            validation_units: vec!["region-a".to_string()],
            challenge_units: vec![],
        }
    }

    fn observation(values: [f64; 2]) -> JointObservation {
        JointObservation {
            statistic_id: "district-resource-state".to_string(),
            independent_unit_id: "region-a".to_string(),
            sample_size: 10,
            source_digest: "sha256-source".to_string(),
            observation_digest: "sha256-observation".to_string(),
            components: vec![
                JointComponentValue {
                    component_id: "water".to_string(),
                    measurement_unit: "m3".to_string(),
                    value: values[0],
                },
                JointComponentValue {
                    component_id: "energy".to_string(),
                    measurement_unit: "kWh".to_string(),
                    value: values[1],
                },
            ],
        }
    }

    fn draw(index: u64, values: [f64; 2], draw_set: &str) -> JointPredictiveDraw {
        JointPredictiveDraw {
            statistic_id: "district-resource-state".to_string(),
            independent_unit_id: "region-a".to_string(),
            sample_size: 10,
            draw_index: index,
            model_digest: "sha256-model".to_string(),
            draw_set_digest: draw_set.to_string(),
            calibration_data_digest: Some("sha256-calibration".to_string()),
            components: vec![
                JointComponentValue {
                    component_id: "water".to_string(),
                    measurement_unit: "m3".to_string(),
                    value: values[0],
                },
                JointComponentValue {
                    component_id: "energy".to_string(),
                    measurement_unit: "kWh".to_string(),
                    value: values[1],
                },
            ],
        }
    }

    fn config() -> JointMetricConfig {
        JointMetricConfig {
            energy_beta: 1.0,
            variogram_power: 1.0,
            pair_weights: vec![1.0],
        }
    }

    #[test]
    fn same_marginals_with_opposite_dependence_are_distinguished() {
        let spec = statistic();
        let manifest = manifest();
        let observed = observation([1.0, 1.0]);
        let positive = vec![
            draw(0, [0.0, 0.0], "draws-positive"),
            draw(1, [1.0, 1.0], "draws-positive"),
        ];
        let negative = vec![
            draw(0, [0.0, 1.0], "draws-negative"),
            draw(1, [1.0, 0.0], "draws-negative"),
        ];

        // Both draws have the same marginal values {0,1}; only coupling differs.
        let positive_result = assess_joint_predictive_draws(
            &spec, &manifest, SplitRole::Validation, &observed, &positive, &config()
        ).expect("positive dependence reference");
        let negative_result = assess_joint_predictive_draws(
            &spec, &manifest, SplitRole::Validation, &observed, &negative, &config()
        ).expect("negative dependence reference");

        assert!((positive_result.energy_score - 0.353_553_390_593_273_8).abs() < 1e-12);
        assert!((negative_result.energy_score - 0.646_446_609_406_726_3).abs() < 1e-12);
        assert_eq!(positive_result.variogram_score, 0.0);
        assert_eq!(negative_result.variogram_score, 1.0);
    }

    #[test]
    fn overflowed_standardization_fails_closed() {
        let mut spec = statistic();
        spec.components[0].reference_scale = f64::MIN_POSITIVE;
        assert!(matches!(
            assess_joint_predictive_draws(
                &spec,
                &manifest(),
                SplitRole::Validation,
                &observation([f64::MAX, 1.0]),
                &[draw(0, [0.0, 0.0], "d"), draw(1, [1.0, 1.0], "d")],
                &config()
            ),
            Err(JointAdequacyError::InvalidComponentVector { .. })
        ));
    }

    #[test]
    fn component_order_is_a_semantic_contract() {
        let mut observed = observation([1.0, 1.0]);
        observed.components.swap(0, 1);
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation, &observed,
                &[draw(0, [0.0, 0.0], "d"), draw(1, [1.0, 1.0], "d")], &config()
            ),
            Err(JointAdequacyError::ComponentOrderMismatch { .. })
        ));
    }

    #[test]
    fn unit_mismatch_is_rejected_even_when_numeric_values_match() {
        let mut observed = observation([1.0, 1.0]);
        observed.components[1].measurement_unit = "m3".to_string();
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation, &observed,
                &[draw(0, [0.0, 0.0], "d"), draw(1, [1.0, 1.0], "d")], &config()
            ),
            Err(JointAdequacyError::ComponentUnitMismatch { .. })
        ));
    }

    #[test]
    fn component_scale_and_metric_configuration_are_validated() {
        let mut spec = statistic();
        spec.components[1].reference_scale = 0.0;
        assert!(matches!(
            spec.validate(),
            Err(JointAdequacyError::InvalidComponent { .. })
        ));

        let mut metric_config = config();
        metric_config.pair_weights.clear();
        assert!(matches!(
            metric_config.validate(2),
            Err(JointAdequacyError::InvalidMetricConfig { .. })
        ));
    }

    #[test]
    fn mixed_model_and_draw_set_identities_are_rejected() {
        let mut model_draws = vec![
            draw(0, [0.0, 0.0], "same"),
            draw(1, [1.0, 1.0], "same"),
        ];
        model_draws[1].model_digest = "sha256-other-model".to_string();
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observation([1.0, 1.0]), &model_draws, &config()
            ),
            Err(JointAdequacyError::MixedModelIdentities { .. })
        ));

        let draw_sets = vec![
            draw(0, [0.0, 0.0], "first"),
            draw(1, [1.0, 1.0], "second"),
        ];
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observation([1.0, 1.0]), &draw_sets, &config()
            ),
            Err(JointAdequacyError::MixedDrawSetIdentities { .. })
        ));
    }

    #[test]
    fn wrong_calibration_digest_is_rejected() {
        let mut draws = vec![
            draw(0, [0.0, 0.0], "d"),
            draw(1, [1.0, 1.0], "d"),
        ];
        draws[0].calibration_data_digest = Some("sha256-foreign".to_string());
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observation([1.0, 1.0]), &draws, &config()
            ),
            Err(JointAdequacyError::CalibrationDigestMismatch { .. })
        ));
    }

    #[test]
    fn wrong_source_sample_size_and_duplicate_draws_fail_closed() {
        let mut observed = observation([1.0, 1.0]);
        observed.source_digest = "sha256-foreign".to_string();
        let valid_draws = vec![
            draw(0, [0.0, 0.0], "d"),
            draw(1, [1.0, 1.0], "d"),
        ];
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observed, &valid_draws, &config()
            ),
            Err(JointAdequacyError::InvalidObservation { .. })
        ));

        let mut wrong_size = valid_draws.clone();
        wrong_size[1].sample_size = 11;
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observation([1.0, 1.0]), &wrong_size, &config()
            ),
            Err(JointAdequacyError::InvalidPredictiveDraw { .. })
        ));

        let mut duplicate = valid_draws;
        duplicate[1].draw_index = duplicate[0].draw_index;
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observation([1.0, 1.0]), &duplicate, &config()
            ),
            Err(JointAdequacyError::DuplicatePredictiveDraw { .. })
        ));
    }

    #[test]
    fn missing_or_insufficient_draws_are_not_a_pass() {
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observation([1.0, 1.0]), &[], &config()
            ),
            Err(JointAdequacyError::MissingPredictiveDraws { .. })
        ));

        let draws = vec![draw(0, [0.0, 0.0], "d")];
        assert!(matches!(
            assess_joint_predictive_draws(
                &statistic(), &manifest(), SplitRole::Validation,
                &observation([1.0, 1.0]), &draws, &config()
            ),
            Err(JointAdequacyError::InsufficientPredictiveDraws { .. })
        ));
    }

    #[test]
    fn challenge_role_is_only_allowed_when_manifest_declares_challenge_unit() {
        let mut manifest = manifest();
        manifest.challenge_units = vec!["region-challenge".to_string()];
        let mut observed = observation([1.0, 1.0]);
        observed.independent_unit_id = "region-challenge".to_string();
        let mut draws = vec![
            draw(0, [0.0, 0.0], "d"),
            draw(1, [1.0, 1.0], "d"),
        ];
        for row in &mut draws {
            row.independent_unit_id = "region-challenge".to_string();
        }
        let mut spec = statistic();
        spec.role = SplitRole::Challenge;
        assert!(assess_joint_predictive_draws(
            &spec, &manifest, SplitRole::Challenge, &observed, &draws, &config()
        ).is_ok());
    }
}
