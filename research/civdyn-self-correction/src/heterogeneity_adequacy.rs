// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Stratified heterogeneity fingerprints for CIVDYN population adequacy.
//!
//! This module distinguishes pooled fit from the allocation of variation
//! within and between declared strata. It uses the population variance of the
//! finite observed sample (denominator n), equal observation weights and the
//! Type-7 / linear-interpolation sample quantile convention. These summaries
//! are descriptive; they do not by themselves quantify sampling uncertainty.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

use crate::joint_adequacy::JointComponentValue;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum HeterogeneityError {
    EmptyStrata,
    EmptyObservationId,
    EmptyStratumId,
    DuplicateStratumId { stratum_id: String },
    DuplicateObservationId { observation_id: String },
    UndeclaredStratum { observation_id: String, stratum_id: String },
    NonFiniteOutcome { observation_id: String },
    NoObservations,
    InvalidMinimumObservationCount { stratum_id: String },
    StratumNotAssessed {
        stratum_id: String,
        observed: usize,
        required: usize,
    },
}

impl fmt::Display for HeterogeneityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for HeterogeneityError {}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StratumDefinition {
    pub id: String,
    pub minimum_observations: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeObservation {
    pub observation_id: String,
    pub stratum_id: String,
    pub value: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StratumAssessmentStatus {
    Assessed,
    NotAssessed,
}

impl StratumAssessmentStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Assessed => "assessed",
            Self::NotAssessed => "not_assessed",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct StratumHeterogeneity {
    pub stratum_id: String,
    pub required: bool,
    pub observed_count: usize,
    pub minimum_observations: usize,
    pub sample_share: f64,
    pub status: StratumAssessmentStatus,
    pub mean: Option<f64>,
    pub population_variance: Option<f64>,
    pub p10: Option<f64>,
    pub median: Option<f64>,
    pub p90: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HeterogeneityProfile {
    pub observation_count: usize,
    pub stratum_count: usize,
    pub global_mean: f64,
    pub global_population_variance: f64,
    pub global_p10: f64,
    pub global_median: f64,
    pub global_p90: f64,
    /// E[Var(Y | G)] using observed sample shares.
    pub within_stratum_variance: f64,
    /// Var(E[Y | G]) using observed sample shares.
    pub between_stratum_variance: f64,
    /// Absolute numerical residual in Var(Y) = within + between.
    pub variance_decomposition_residual: f64,
    pub strata: Vec<StratumHeterogeneity>,
}

fn population_moments(values: &[f64]) -> (f64, f64) {
    let mean = values.iter().sum::<f64>() / values.len() as f64;
    let variance = values
        .iter()
        .map(|value| {
            let difference = value - mean;
            difference * difference
        })
        .sum::<f64>()
        / values.len() as f64;
    (mean, variance)
}

/// Hyndman-Fan Type-7 quantile: h = (n - 1) * p, with linear interpolation.
fn type7_quantile(values: &[f64], probability: f64) -> f64 {
    let mut ordered = values.to_vec();
    ordered.sort_by(f64::total_cmp);
    let position = (ordered.len() - 1) as f64 * probability;
    let lower_index = position.floor() as usize;
    let upper_index = position.ceil() as usize;
    let fraction = position - lower_index as f64;
    ordered[lower_index] * (1.0 - fraction) + ordered[upper_index] * fraction
}

impl HeterogeneityProfile {
    pub fn summarize(
        definitions: &[StratumDefinition],
        observations: &[OutcomeObservation],
    ) -> Result<Self, HeterogeneityError> {
        if definitions.is_empty() {
            return Err(HeterogeneityError::EmptyStrata);
        }
        if observations.is_empty() {
            return Err(HeterogeneityError::NoObservations);
        }

        let mut declared = BTreeMap::<String, &StratumDefinition>::new();
        for definition in definitions {
            if definition.id.trim().is_empty() {
                return Err(HeterogeneityError::EmptyStratumId);
            }
            if definition.minimum_observations == 0 {
                return Err(HeterogeneityError::InvalidMinimumObservationCount {
                    stratum_id: definition.id.clone(),
                });
            }
            if declared.insert(definition.id.clone(), definition).is_some() {
                return Err(HeterogeneityError::DuplicateStratumId {
                    stratum_id: definition.id.clone(),
                });
            }
        }

        let mut observation_ids = BTreeSet::new();
        let mut values_by_stratum = BTreeMap::<String, Vec<f64>>::new();
        let mut all_values = Vec::with_capacity(observations.len());
        for observation in observations {
            if observation.observation_id.trim().is_empty() {
                return Err(HeterogeneityError::EmptyObservationId);
            }
            if !observation_ids.insert(observation.observation_id.as_str()) {
                return Err(HeterogeneityError::DuplicateObservationId {
                    observation_id: observation.observation_id.clone(),
                });
            }
            if !observation.value.is_finite() {
                return Err(HeterogeneityError::NonFiniteOutcome {
                    observation_id: observation.observation_id.clone(),
                });
            }
            if !declared.contains_key(&observation.stratum_id) {
                return Err(HeterogeneityError::UndeclaredStratum {
                    observation_id: observation.observation_id.clone(),
                    stratum_id: observation.stratum_id.clone(),
                });
            }
            values_by_stratum
                .entry(observation.stratum_id.clone())
                .or_default()
                .push(observation.value);
            all_values.push(observation.value);
        }

        let (global_mean, global_population_variance) = population_moments(&all_values);
        let global_p10 = type7_quantile(&all_values, 0.10);
        let global_median = type7_quantile(&all_values, 0.50);
        let global_p90 = type7_quantile(&all_values, 0.90);

        let mut strata = Vec::with_capacity(declared.len());
        let total_count = observations.len();
        let mut within_stratum_variance = 0.0;
        let mut between_stratum_variance = 0.0;

        for (stratum_id, definition) in declared {
            let values = values_by_stratum.get(&stratum_id).cloned().unwrap_or_default();
            let observed_count = values.len();
            let sample_share = observed_count as f64 / total_count as f64;
            let status = if observed_count >= definition.minimum_observations {
                StratumAssessmentStatus::Assessed
            } else {
                StratumAssessmentStatus::NotAssessed
            };

            let (mean, variance, p10, median, p90) = if values.is_empty() {
                (None, None, None, None, None)
            } else {
                let (mean, variance) = population_moments(&values);
                (
                    Some(mean),
                    Some(variance),
                    Some(type7_quantile(&values, 0.10)),
                    Some(type7_quantile(&values, 0.50)),
                    Some(type7_quantile(&values, 0.90)),
                )
            };

            if let (Some(mean), Some(variance)) = (mean, variance) {
                within_stratum_variance += sample_share * variance;
                let mean_difference = mean - global_mean;
                between_stratum_variance += sample_share * mean_difference * mean_difference;
            }

            strata.push(StratumHeterogeneity {
                stratum_id,
                required: definition.required,
                observed_count,
                minimum_observations: definition.minimum_observations,
                sample_share,
                status,
                mean,
                population_variance: variance,
                p10,
                median,
                p90,
            });
        }

        let variance_decomposition_residual =
            (global_population_variance - within_stratum_variance - between_stratum_variance)
                .abs();
        if [
            global_mean,
            global_population_variance,
            global_p10,
            global_median,
            global_p90,
            within_stratum_variance,
            between_stratum_variance,
            variance_decomposition_residual,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(HeterogeneityError::NoObservations);
        }

        Ok(Self {
            observation_count: total_count,
            stratum_count: strata.len(),
            global_mean,
            global_population_variance,
            global_p10,
            global_median,
            global_p90,
            within_stratum_variance,
            between_stratum_variance,
            variance_decomposition_residual,
            strata,
        })
    }

    /// Convert the full stratified fingerprint into one ordered joint vector.
    /// If any declared stratum is below its minimum observation count, no
    /// joint adequacy score is produced: missing strata cannot be averaged out.
    pub fn joint_component_values(&self) -> Result<Vec<JointComponentValue>, HeterogeneityError> {
        if let Some(stratum) = self
            .strata
            .iter()
            .find(|stratum| stratum.status == StratumAssessmentStatus::NotAssessed)
        {
            return Err(HeterogeneityError::StratumNotAssessed {
                stratum_id: stratum.stratum_id.clone(),
                observed: stratum.observed_count,
                required: stratum.minimum_observations,
            });
        }

        let mut components = vec![
            component("global-mean", "outcome_units", self.global_mean),
            component(
                "global-population-variance",
                "outcome_units_squared",
                self.global_population_variance,
            ),
            component("global-p10", "outcome_units", self.global_p10),
            component("global-median", "outcome_units", self.global_median),
            component("global-p90", "outcome_units", self.global_p90),
            component(
                "within-stratum-variance",
                "outcome_units_squared",
                self.within_stratum_variance,
            ),
            component(
                "between-stratum-variance",
                "outcome_units_squared",
                self.between_stratum_variance,
            ),
        ];

        for stratum in &self.strata {
            let mean = stratum.mean.ok_or_else(|| HeterogeneityError::StratumNotAssessed {
                stratum_id: stratum.stratum_id.clone(),
                observed: stratum.observed_count,
                required: stratum.minimum_observations,
            })?;
            let variance = stratum.population_variance.ok_or_else(|| {
                HeterogeneityError::StratumNotAssessed {
                    stratum_id: stratum.stratum_id.clone(),
                    observed: stratum.observed_count,
                    required: stratum.minimum_observations,
                }
            })?;
            let p10 = stratum.p10.ok_or_else(|| HeterogeneityError::StratumNotAssessed {
                stratum_id: stratum.stratum_id.clone(),
                observed: stratum.observed_count,
                required: stratum.minimum_observations,
            })?;
            let median = stratum.median.ok_or_else(|| HeterogeneityError::StratumNotAssessed {
                stratum_id: stratum.stratum_id.clone(),
                observed: stratum.observed_count,
                required: stratum.minimum_observations,
            })?;
            let p90 = stratum.p90.ok_or_else(|| HeterogeneityError::StratumNotAssessed {
                stratum_id: stratum.stratum_id.clone(),
                observed: stratum.observed_count,
                required: stratum.minimum_observations,
            })?;

            components.push(component(
                &format!("stratum-{}-sample-share", stratum.stratum_id),
                "ratio",
                stratum.sample_share,
            ));
            components.push(component(
                &format!("stratum-{}-mean", stratum.stratum_id),
                "outcome_units",
                mean,
            ));
            components.push(component(
                &format!("stratum-{}-population-variance", stratum.stratum_id),
                "outcome_units_squared",
                variance,
            ));
            components.push(component(
                &format!("stratum-{}-p10", stratum.stratum_id),
                "outcome_units",
                p10,
            ));
            components.push(component(
                &format!("stratum-{}-median", stratum.stratum_id),
                "outcome_units",
                median,
            ));
            components.push(component(
                &format!("stratum-{}-p90", stratum.stratum_id),
                "outcome_units",
                p90,
            ));
        }

        Ok(components)
    }
}

fn component(id: &str, measurement_unit: &str, value: f64) -> JointComponentValue {
    JointComponentValue {
        component_id: id.to_string(),
        measurement_unit: measurement_unit.to_string(),
        value,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::joint_adequacy::{
        assess_joint_predictive_draws, JointComponentSpec, JointMetricConfig, JointObservation,
        JointPredictiveDraw, JointStatisticSpec,
    };
    use crate::model_adequacy::{IndependentUnitKind, SplitManifest, SplitRole};

    fn definitions() -> Vec<StratumDefinition> {
        vec![
            StratumDefinition {
                id: "group-a".to_string(),
                minimum_observations: 2,
            },
            StratumDefinition {
                id: "group-b".to_string(),
                required: true,
                minimum_observations: 2,
            },
        ]
    }

    fn reference_observations() -> Vec<OutcomeObservation> {
        vec![
            OutcomeObservation {
                observation_id: "a-1".to_string(),
                stratum_id: "group-a".to_string(),
                value: 0.0,
            },
            OutcomeObservation {
                observation_id: "a-2".to_string(),
                stratum_id: "group-a".to_string(),
                value: 2.0,
            },
            OutcomeObservation {
                observation_id: "b-1".to_string(),
                stratum_id: "group-b".to_string(),
                value: 8.0,
            },
            OutcomeObservation {
                observation_id: "b-2".to_string(),
                stratum_id: "group-b".to_string(),
                value: 10.0,
            },
        ]
    }

    fn collapsed_observations() -> Vec<OutcomeObservation> {
        let offset = 17.0_f64.sqrt();
        vec![
            OutcomeObservation {
                observation_id: "a-1".to_string(),
                stratum_id: "group-a".to_string(),
                value: 5.0 - offset,
            },
            OutcomeObservation {
                observation_id: "a-2".to_string(),
                stratum_id: "group-a".to_string(),
                value: 5.0 + offset,
            },
            OutcomeObservation {
                observation_id: "b-1".to_string(),
                stratum_id: "group-b".to_string(),
                value: 5.0 - offset,
            },
            OutcomeObservation {
                observation_id: "b-2".to_string(),
                stratum_id: "group-b".to_string(),
                value: 5.0 + offset,
            },
        ]
    }

    #[test]
    fn recovers_population_total_variance_decomposition() {
        let profile =
            HeterogeneityProfile::summarize(&definitions(), &reference_observations())
                .expect("reference heterogeneity profile");
        assert_eq!(profile.observation_count, 4);
        assert_eq!(profile.stratum_count, 2);
        assert_eq!(profile.global_mean, 5.0);
        assert_eq!(profile.global_population_variance, 17.0);
        assert_eq!(profile.within_stratum_variance, 1.0);
        assert_eq!(profile.between_stratum_variance, 16.0);
        assert!(profile.variance_decomposition_residual < 1e-12);
    }

    #[test]
    fn pooled_mean_and_variance_do_not_reveal_variance_collapse() {
        let reference =
            HeterogeneityProfile::summarize(&definitions(), &reference_observations())
                .expect("reference profile");
        let collapsed =
            HeterogeneityProfile::summarize(&definitions(), &collapsed_observations())
                .expect("collapsed profile");
        assert_eq!(reference.global_mean, collapsed.global_mean);
        assert!((reference.global_population_variance - collapsed.global_population_variance).abs() < 1e-12);
        assert!((reference.within_stratum_variance - 1.0).abs() < 1e-12);
        assert!((collapsed.within_stratum_variance - 17.0).abs() < 1e-12);
        assert!((reference.between_stratum_variance - 16.0).abs() < 1e-12);
        assert!(collapsed.between_stratum_variance.abs() < 1e-12);
    }

    #[test]
    fn type7_quantiles_are_frozen_and_known() {
        let profile =
            HeterogeneityProfile::summarize(&definitions(), &reference_observations())
                .expect("profile");
        assert!((profile.global_p10 - 0.6).abs() < 1e-12);
        assert_eq!(profile.global_median, 5.0);
        assert!((profile.global_p90 - 9.4).abs() < 1e-12);
        let group_a = profile
            .strata
            .iter()
            .find(|stratum| stratum.stratum_id == "group-a")
            .expect("group A");
        assert_eq!(group_a.mean, Some(1.0));
        assert_eq!(group_a.population_variance, Some(1.0));
        assert!((group_a.p10.expect("p10") - 0.2).abs() < 1e-12);
        assert_eq!(group_a.median, Some(1.0));
        assert!((group_a.p90.expect("p90") - 1.8).abs() < 1e-12);
    }

    #[test]
    fn row_order_and_definition_order_preserve_canonical_fingerprint() {
        let first =
            HeterogeneityProfile::summarize(&definitions(), &reference_observations())
                .expect("profile");
        let mut reversed_definitions = definitions();
        reversed_definitions.reverse();
        let mut reversed_observations = reference_observations();
        reversed_observations.reverse();
        let second =
            HeterogeneityProfile::summarize(&reversed_definitions, &reversed_observations)
                .expect("profile");
        assert_eq!(first, second);
        assert_eq!(
            first.joint_component_values().expect("complete vector"),
            second.joint_component_values().expect("complete vector")
        );
    }

    #[test]
    fn incomplete_strata_are_explicit_and_cannot_be_jointly_scored() {
        let mut observations = reference_observations();
        observations.retain(|row| row.stratum_id != "group-b");
        let profile =
            HeterogeneityProfile::summarize(&definitions(), &observations)
                .expect("profile with incomplete target");
        assert_eq!(
            profile.strata[1].status,
            StratumAssessmentStatus::NotAssessed
        );
        assert!(matches!(
            profile.joint_component_values(),
            Err(HeterogeneityError::StratumNotAssessed { .. })
        ));
    }

    #[test]
    fn malformed_observations_fail_closed() {
        let mut duplicate = reference_observations();
        duplicate.push(duplicate[0].clone());
        assert!(matches!(
            HeterogeneityProfile::summarize(&definitions(), &duplicate),
            Err(HeterogeneityError::DuplicateObservationId { .. })
        ));

        let mut undeclared = reference_observations();
        undeclared[0].stratum_id = "undeclared-group".to_string();
        assert!(matches!(
            HeterogeneityProfile::summarize(&definitions(), &undeclared),
            Err(HeterogeneityError::UndeclaredStratum { .. })
        ));

        let mut non_finite = reference_observations();
        non_finite[0].value = f64::NAN;
        assert!(matches!(
            HeterogeneityProfile::summarize(&definitions(), &non_finite),
            Err(HeterogeneityError::NonFiniteOutcome { .. })
        ));
    }

    fn joint_schema(
        reference: &HeterogeneityProfile,
    ) -> JointStatisticSpec {
        let components = reference
            .joint_component_values()
            .expect("reference profile complete")
            .into_iter()
            .map(|value| JointComponentSpec {
                id: value.component_id,
                measurement_unit: value.measurement_unit,
                reference_center: value.value,
                reference_scale: 1.0,
            })
            .collect();
        JointStatisticSpec {
            id: "stratified-behavioral-heterogeneity".to_string(),
            source_family: "synthetic-stratified-known-answer".to_string(),
            source_digest: "sha256-heterogeneity-source".to_string(),
            population: "two-stratum-synthetic-population".to_string(),
            context: "same-pooled-moments-different-heterogeneity".to_string(),
            independent_unit: IndependentUnitKind::Region,
            role: SplitRole::Validation,
            primary: true,
            required: true,
            minimum_draws: 20,
            components,
        }
    }

    fn assess_profile(
        profile: &HeterogeneityProfile,
        schema: &JointStatisticSpec,
        model_digest: &str,
    ) -> crate::joint_adequacy::JointAdequacyResult {
        let manifest = SplitManifest {
            id: "heterogeneity-split-v1".to_string(),
            independent_unit: IndependentUnitKind::Region,
            calibration_data_digest: "sha256-heterogeneity-calibration".to_string(),
            manifest_digest: "sha256-heterogeneity-manifest".to_string(),
            calibration_units: vec!["calibration-region".to_string()],
            validation_units: vec!["held-out-region".to_string()],
            challenge_units: Vec::new(),
        };
        let observation = JointObservation {
            statistic_id: schema.id.clone(),
            independent_unit_id: "held-out-region".to_string(),
            sample_size: 4,
            source_digest: schema.source_digest.clone(),
            observation_digest: "sha256-heterogeneity-observation".to_string(),
            split_manifest_digest: manifest.manifest_digest.clone(),
            components: schema
                .components
                .iter()
                .map(|spec| JointComponentValue {
                    component_id: spec.id.clone(),
                    measurement_unit: spec.measurement_unit.clone(),
                    value: spec.reference_center,
                })
                .collect(),
        };
        let components = profile
            .joint_component_values()
            .expect("candidate profile complete");
        let draws = (0..20)
            .map(|draw_index| JointPredictiveDraw {
                statistic_id: schema.id.clone(),
                independent_unit_id: "held-out-region".to_string(),
                sample_size: 4,
                draw_index,
                model_digest: model_digest.to_string(),
                draw_set_digest: format!("sha256-heterogeneity-draws-{model_digest}"),
                calibration_data_digest: Some(manifest.calibration_data_digest.clone()),
                split_manifest_digest: manifest.manifest_digest.clone(),
                components: components.clone(),
            })
            .collect::<Vec<_>>();
        assess_joint_predictive_draws(
            schema,
            &manifest,
            SplitRole::Validation,
            &observation,
            &draws,
            &JointMetricConfig {
                energy_beta: 1.0,
                variogram_power: 1.0,
                pair_weights: vec![
                    1.0;
                    schema.components.len() * (schema.components.len() - 1) / 2
                ],
            },
        )
        .expect("heterogeneity joint assessment")
    }

    #[test]
    fn joint_score_detects_variance_collapse_with_same_pooled_moments() {
        let reference =
            HeterogeneityProfile::summarize(&definitions(), &reference_observations())
                .expect("reference");
        let collapsed =
            HeterogeneityProfile::summarize(&definitions(), &collapsed_observations())
                .expect("collapsed");
        let schema = joint_schema(&reference);
        let correct = assess_profile(&reference, &schema, "sha256-heterogeneity-model-correct");
        let wrong = assess_profile(&collapsed, &schema, "sha256-heterogeneity-model-collapsed");
        assert_eq!(correct.energy_score, 0.0);
        assert_eq!(correct.variogram_score, 0.0);
        assert!(wrong.energy_score > 0.0);
        assert!(wrong.variogram_score > 0.0);
    }
}
