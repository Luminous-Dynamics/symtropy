// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Deterministic weighted least-squares for a linearized survey adjustment step.
//!
//! Callers supply linearized observation equations and stochastic weights. The
//! implementation uses an SVD of the weighted design matrix, not the normal
//! equations, so rank deficiency is detected before a solution is returned.
//!
//! This module is deliberately not a complete geodetic network engine: it does
//! not create range/angle equations, iterate a nonlinear model, transform CRSs,
//! select geodetic datums, or certify field accuracy. Exact fixed parameters are
//! conditioned as exact; their uncertainty is not propagated by this step.

use crate::world_contract::ArtifactRef;
use nalgebra::{DMatrix, DVector};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

pub const SURVEY_ADJUSTMENT_SCHEMA_VERSION: u16 = 1;
pub const MAX_ADJUSTMENT_PARAMETERS: usize = 128;
pub const MAX_ADJUSTMENT_OBSERVATIONS: usize = 8_192;
pub const MAX_ADJUSTMENT_SOURCE_ARTIFACTS: usize = 4_096;
pub const MAX_ADJUSTMENT_PAYLOAD_BYTES: usize = 64 * 1024 * 1024;
pub const MAX_ADJUSTMENT_ID_LENGTH: usize = 256;
const SHA256_HEX_LENGTH: usize = 64;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SurveyAdjustmentErrorV1 {
    UnsupportedSchemaVersion,
    InvalidIdentity(&'static str),
    InvalidDigest(&'static str),
    InvalidArtifact(&'static str),
    TooManyItems(&'static str),
    EmptySet(&'static str),
    DuplicateIdentity(&'static str),
    DuplicateParameterIndex(usize),
    ParameterIndexOutOfRange(usize),
    InvalidDimensions(&'static str),
    InvalidNumber(&'static str),
    InvalidStandardUncertainty,
    InvalidRankTolerance,
    InsufficientObservations,
    RankDeficient { rank: usize, required_rank: usize },
    NumericalFailure(&'static str),
}

impl fmt::Display for SurveyAdjustmentErrorV1 {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SurveyAdjustmentErrorV1 {}

fn valid_identity(value: &str) -> bool {
    !value.trim().is_empty() && value.len() <= MAX_ADJUSTMENT_ID_LENGTH
}

fn valid_sha256(value: &str) -> bool {
    value.len() == SHA256_HEX_LENGTH && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn validate_artifact(
    artifact: &ArtifactRef,
    path: &'static str,
) -> Result<(), SurveyAdjustmentErrorV1> {
    if !valid_identity(&artifact.artifact_id) || !valid_identity(&artifact.uri) {
        return Err(SurveyAdjustmentErrorV1::InvalidArtifact(path));
    }
    if !valid_sha256(&artifact.sha256) {
        return Err(SurveyAdjustmentErrorV1::InvalidDigest(path));
    }
    Ok(())
}

fn ensure_unique<'a>(
    identities: impl IntoIterator<Item = &'a str>,
    field: &'static str,
) -> Result<(), SurveyAdjustmentErrorV1> {
    let mut seen = BTreeSet::new();
    for identity in identities {
        if !valid_identity(identity) {
            return Err(SurveyAdjustmentErrorV1::InvalidIdentity(field));
        }
        if !seen.insert(identity) {
            return Err(SurveyAdjustmentErrorV1::DuplicateIdentity(field));
        }
    }
    Ok(())
}

/// One row of a linearized observation equation: coefficients dot parameters
/// is approximately the observed value. The standard uncertainty is one sigma
/// in the units of the observation; its reciprocal scales the row.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearizedSurveyObservationV1 {
    pub observation_id: String,
    pub coefficients: Vec<f64>,
    pub value: f64,
    pub standard_uncertainty: f64,
}

impl LinearizedSurveyObservationV1 {
    fn validate(&self, parameter_count: usize) -> Result<(), SurveyAdjustmentErrorV1> {
        if !valid_identity(&self.observation_id) {
            return Err(SurveyAdjustmentErrorV1::InvalidIdentity("observation_id"));
        }
        if self.coefficients.len() != parameter_count {
            return Err(SurveyAdjustmentErrorV1::InvalidDimensions(
                "coefficient_count",
            ));
        }
        if !self.value.is_finite()
            || !self
                .coefficients
                .iter()
                .all(|coefficient| coefficient.is_finite())
        {
            return Err(SurveyAdjustmentErrorV1::InvalidNumber("observation"));
        }
        if !self.standard_uncertainty.is_finite() || self.standard_uncertainty <= 0.0 {
            return Err(SurveyAdjustmentErrorV1::InvalidStandardUncertainty);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyAdjustmentProvenanceV1 {
    pub algorithm_id: String,
    pub algorithm_version: String,
    /// SHA-256 of the exact software build or reproducible build manifest.
    pub algorithm_build_sha256: String,
    /// SHA-256 of the exact serialized solver configuration.
    pub configuration_sha256: String,
    /// Immutable source observations/control files used for this adjustment.
    pub source_artifacts: Vec<ArtifactRef>,
}

impl SurveyAdjustmentProvenanceV1 {
    fn validate(&self) -> Result<(), SurveyAdjustmentErrorV1> {
        if !valid_identity(&self.algorithm_id) {
            return Err(SurveyAdjustmentErrorV1::InvalidIdentity("algorithm_id"));
        }
        if !valid_identity(&self.algorithm_version) {
            return Err(SurveyAdjustmentErrorV1::InvalidIdentity(
                "algorithm_version",
            ));
        }
        for (digest, name) in [
            (
                self.algorithm_build_sha256.as_str(),
                "algorithm_build_sha256",
            ),
            (self.configuration_sha256.as_str(), "configuration_sha256"),
        ] {
            if !valid_sha256(digest) {
                return Err(SurveyAdjustmentErrorV1::InvalidDigest(name));
            }
        }
        if self.source_artifacts.is_empty() {
            return Err(SurveyAdjustmentErrorV1::EmptySet("source_artifacts"));
        }
        if self.source_artifacts.len() > MAX_ADJUSTMENT_SOURCE_ARTIFACTS {
            return Err(SurveyAdjustmentErrorV1::TooManyItems("source_artifacts"));
        }
        ensure_unique(
            self.source_artifacts
                .iter()
                .map(|artifact| artifact.artifact_id.as_str()),
            "source_artifact_id",
        )?;
        for artifact in &self.source_artifacts {
            validate_artifact(artifact, "source_artifacts")?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FixedSurveyParameterV1 {
    pub parameter_index: usize,
    pub value: f64,
}

/// Fitting, independent-checkpoint, and excluded rows are separate collections
/// so the solver cannot accidentally use held-out checkpoints to fit the solution.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct LinearizedSurveyAdjustmentV1 {
    pub schema_version: u16,
    pub adjustment_id: String,
    pub parameter_names: Vec<String>,
    pub fitting_observations: Vec<LinearizedSurveyObservationV1>,
    pub independent_checkpoints: Vec<LinearizedSurveyObservationV1>,
    /// Retained for diagnostics, but never weighted into the solution.
    pub excluded_observations: Vec<LinearizedSurveyObservationV1>,
    /// Exact constraints on parameter values; uncertainty of these controls
    /// must be modeled upstream if it is not negligible.
    pub fixed_parameters: Vec<FixedSurveyParameterV1>,
    /// Relative cutoff compared with the largest singular value of the weighted
    /// design matrix. Must be in (0, 1).
    pub relative_rank_tolerance: f64,
    pub provenance: SurveyAdjustmentProvenanceV1,
}

impl LinearizedSurveyAdjustmentV1 {
    pub fn validate(&self) -> Result<(), SurveyAdjustmentErrorV1> {
        if self.schema_version != SURVEY_ADJUSTMENT_SCHEMA_VERSION {
            return Err(SurveyAdjustmentErrorV1::UnsupportedSchemaVersion);
        }
        if !valid_identity(&self.adjustment_id) {
            return Err(SurveyAdjustmentErrorV1::InvalidIdentity("adjustment_id"));
        }
        let parameter_count = self.parameter_names.len();
        if parameter_count == 0 {
            return Err(SurveyAdjustmentErrorV1::EmptySet("parameter_names"));
        }
        if parameter_count > MAX_ADJUSTMENT_PARAMETERS {
            return Err(SurveyAdjustmentErrorV1::TooManyItems("parameter_names"));
        }
        ensure_unique(
            self.parameter_names.iter().map(String::as_str),
            "parameter_names",
        )?;
        let observation_count = self.fitting_observations.len()
            + self.independent_checkpoints.len()
            + self.excluded_observations.len();
        if observation_count > MAX_ADJUSTMENT_OBSERVATIONS {
            return Err(SurveyAdjustmentErrorV1::TooManyItems("observations"));
        }
        if self.fitting_observations.is_empty() {
            return Err(SurveyAdjustmentErrorV1::EmptySet("fitting_observations"));
        }
        if !self.relative_rank_tolerance.is_finite()
            || self.relative_rank_tolerance <= 0.0
            || self.relative_rank_tolerance >= 1.0
        {
            return Err(SurveyAdjustmentErrorV1::InvalidRankTolerance);
        }

        let mut all_ids = Vec::with_capacity(observation_count);
        for observation in self
            .fitting_observations
            .iter()
            .chain(&self.independent_checkpoints)
            .chain(&self.excluded_observations)
        {
            observation.validate(parameter_count)?;
            all_ids.push(observation.observation_id.as_str());
        }
        ensure_unique(all_ids, "observation_id")?;

        let mut fixed_indices = BTreeSet::new();
        for fixed in &self.fixed_parameters {
            if fixed.parameter_index >= parameter_count {
                return Err(SurveyAdjustmentErrorV1::ParameterIndexOutOfRange(
                    fixed.parameter_index,
                ));
            }
            if !fixed.value.is_finite() {
                return Err(SurveyAdjustmentErrorV1::InvalidNumber("fixed_parameter"));
            }
            if !fixed_indices.insert(fixed.parameter_index) {
                return Err(SurveyAdjustmentErrorV1::DuplicateParameterIndex(
                    fixed.parameter_index,
                ));
            }
        }
        if fixed_indices.len() == parameter_count {
            return Err(SurveyAdjustmentErrorV1::EmptySet("free_parameters"));
        }
        let free_count = parameter_count - fixed_indices.len();
        if self.fitting_observations.len() < free_count {
            return Err(SurveyAdjustmentErrorV1::InsufficientObservations);
        }
        self.provenance.validate()
    }

    /// Run one weighted least-squares solve for supplied linearized equations.
    /// Nonlinear range/angle networks must linearize and call this in a controlled
    /// outer iteration with convergence and model-credibility checks.
    pub fn solve(&self) -> Result<SurveyAdjustmentResultV1, SurveyAdjustmentErrorV1> {
        self.validate()?;

        let parameter_count = self.parameter_names.len();
        let fixed: BTreeMap<usize, f64> = self
            .fixed_parameters
            .iter()
            .map(|entry| (entry.parameter_index, entry.value))
            .collect();
        let free_indices: Vec<usize> = (0..parameter_count)
            .filter(|index| !fixed.contains_key(index))
            .collect();
        let free_count = free_indices.len();
        let row_count = self.fitting_observations.len();

        let mut weighted_design = DMatrix::<f64>::zeros(row_count, free_count);
        let mut weighted_values = DVector::<f64>::zeros(row_count);

        for (row_index, observation) in self.fitting_observations.iter().enumerate() {
            let fixed_contribution = self
                .fixed_parameters
                .iter()
                .map(|entry| observation.coefficients[entry.parameter_index] * entry.value)
                .sum::<f64>();
            let sigma = observation.standard_uncertainty;
            weighted_values[row_index] = (observation.value - fixed_contribution) / sigma;
            for (column_index, parameter_index) in free_indices.iter().enumerate() {
                weighted_design[(row_index, column_index)] =
                    observation.coefficients[*parameter_index] / sigma;
            }
        }

        if !weighted_design.iter().all(|value| value.is_finite())
            || !weighted_values.iter().all(|value| value.is_finite())
        {
            return Err(SurveyAdjustmentErrorV1::NumericalFailure(
                "weighted_system_not_finite",
            ));
        }

        // SVD avoids squaring the condition number by explicitly forming AᵀPA.
        let decomposition = weighted_design.svd(true, true);
        let singular_values = decomposition.singular_values.clone();
        let largest = singular_values.iter().copied().fold(0.0_f64, f64::max);
        if !largest.is_finite() || largest <= 0.0 {
            return Err(SurveyAdjustmentErrorV1::RankDeficient {
                rank: 0,
                required_rank: free_count,
            });
        }
        let cutoff = largest * self.relative_rank_tolerance;
        let rank = singular_values
            .iter()
            .filter(|value| value.is_finite() && **value > cutoff)
            .count();
        if rank < free_count {
            return Err(SurveyAdjustmentErrorV1::RankDeficient {
                rank,
                required_rank: free_count,
            });
        }

        let u = decomposition
            .u
            .ok_or(SurveyAdjustmentErrorV1::NumericalFailure("missing_u"))?;
        let v_t = decomposition
            .v_t
            .ok_or(SurveyAdjustmentErrorV1::NumericalFailure("missing_vt"))?;
        let projected = u.transpose() * weighted_values;
        let scaled = DVector::from_iterator(
            free_count,
            singular_values
                .iter()
                .enumerate()
                .map(|(index, singular)| projected[index] / singular),
        );
        let v = v_t.transpose();
        let free_solution = &v * scaled;
        if !free_solution.iter().all(|value| value.is_finite()) {
            return Err(SurveyAdjustmentErrorV1::NumericalFailure(
                "solution_not_finite",
            ));
        }

        let mut parameters = vec![0.0; parameter_count];
        for (index, value) in &fixed {
            parameters[*index] = *value;
        }
        for (free_position, parameter_index) in free_indices.iter().enumerate() {
            parameters[*parameter_index] = free_solution[free_position];
        }

        let mut residuals = Vec::with_capacity(
            self.fitting_observations.len()
                + self.independent_checkpoints.len()
                + self.excluded_observations.len(),
        );
        let mut weighted_sum_squares = 0.0_f64;
        for observation in &self.fitting_observations {
            let residual = residual_for(observation, &parameters)?;
            let normalized = residual / observation.standard_uncertainty;
            weighted_sum_squares += normalized * normalized;
            residuals.push(SurveyAdjustmentResidualV1 {
                observation_id: observation.observation_id.clone(),
                role: SurveyAdjustmentObservationRoleV1::Fit,
                residual,
                normalized_residual: normalized,
            });
        }
        for observation in &self.independent_checkpoints {
            let residual = residual_for(observation, &parameters)?;
            residuals.push(SurveyAdjustmentResidualV1 {
                observation_id: observation.observation_id.clone(),
                role: SurveyAdjustmentObservationRoleV1::IndependentCheckpoint,
                residual,
                normalized_residual: residual / observation.standard_uncertainty,
            });
        }
        for observation in &self.excluded_observations {
            let residual = residual_for(observation, &parameters)?;
            residuals.push(SurveyAdjustmentResidualV1 {
                observation_id: observation.observation_id.clone(),
                role: SurveyAdjustmentObservationRoleV1::Excluded,
                residual,
                normalized_residual: residual / observation.standard_uncertainty,
            });
        }
        if !weighted_sum_squares.is_finite()
            || residuals
                .iter()
                .any(|item| !item.residual.is_finite() || !item.normalized_residual.is_finite())
        {
            return Err(SurveyAdjustmentErrorV1::NumericalFailure(
                "residual_not_finite",
            ));
        }

        let degrees_of_freedom = row_count - free_count;
        let posterior_variance_factor =
            (degrees_of_freedom > 0).then_some(weighted_sum_squares / degrees_of_freedom as f64);

        // Formal covariance/cofactor matrix from the weighted design SVD.
        let mut covariance_row_major = vec![0.0; free_count * free_count];
        for row in 0..free_count {
            for column in 0..free_count {
                let covariance = (0..free_count)
                    .map(|component| {
                        v[(row, component)] * v[(column, component)]
                            / (singular_values[component] * singular_values[component])
                    })
                    .sum::<f64>();
                if !covariance.is_finite() {
                    return Err(SurveyAdjustmentErrorV1::NumericalFailure(
                        "covariance_not_finite",
                    ));
                }
                covariance_row_major[row * free_count + column] = covariance;
            }
        }

        Ok(SurveyAdjustmentResultV1 {
            schema_version: SURVEY_ADJUSTMENT_SCHEMA_VERSION,
            adjustment_id: self.adjustment_id.clone(),
            parameter_estimates: self
                .parameter_names
                .iter()
                .enumerate()
                .map(|(index, name)| SurveyParameterEstimateV1 {
                    parameter_name: name.clone(),
                    value: parameters[index],
                    is_fixed: fixed.contains_key(&index),
                })
                .collect(),
            free_parameter_indices: free_indices,
            formal_covariance_row_major: covariance_row_major,
            residuals,
            fitting_observation_count: row_count,
            independent_checkpoint_count: self.independent_checkpoints.len(),
            excluded_observation_count: self.excluded_observations.len(),
            free_parameter_count: free_count,
            degrees_of_freedom,
            weighted_sum_squares,
            posterior_variance_factor,
            singular_values: singular_values.iter().copied().collect(),
            relative_rank_tolerance: self.relative_rank_tolerance,
            provenance: self.provenance.clone(),
        })
    }

    /// Encode exact input bytes and bind them with an out-of-band artifact digest.
    pub fn encode_with_digest(
        &self,
        artifact_id: impl Into<String>,
        uri: impl Into<String>,
    ) -> Result<(Vec<u8>, ArtifactRef), SurveyAdjustmentErrorV1> {
        self.validate()?;
        let artifact_id = artifact_id.into();
        let uri = uri.into();
        if !valid_identity(&artifact_id) || !valid_identity(&uri) {
            return Err(SurveyAdjustmentErrorV1::InvalidArtifact("adjustment_input"));
        }
        let bytes = serde_json::to_vec(self)
            .map_err(|_| SurveyAdjustmentErrorV1::NumericalFailure("serialize_input"))?;
        if bytes.len() > MAX_ADJUSTMENT_PAYLOAD_BYTES {
            return Err(SurveyAdjustmentErrorV1::TooManyItems("payload_bytes"));
        }
        let sha256 = format!("{:x}", Sha256::digest(&bytes));
        Ok((
            bytes,
            ArtifactRef {
                artifact_id,
                uri,
                sha256,
            },
        ))
    }
}

fn residual_for(
    observation: &LinearizedSurveyObservationV1,
    parameters: &[f64],
) -> Result<f64, SurveyAdjustmentErrorV1> {
    let predicted = observation
        .coefficients
        .iter()
        .zip(parameters)
        .map(|(coefficient, parameter)| coefficient * parameter)
        .sum::<f64>();
    let residual = predicted - observation.value;
    if residual.is_finite() {
        Ok(residual)
    } else {
        Err(SurveyAdjustmentErrorV1::NumericalFailure(
            "observation_residual_not_finite",
        ))
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SurveyAdjustmentObservationRoleV1 {
    Fit,
    IndependentCheckpoint,
    Excluded,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyAdjustmentResidualV1 {
    pub observation_id: String,
    pub role: SurveyAdjustmentObservationRoleV1,
    /// Predicted minus observed; the sign convention is explicit.
    pub residual: f64,
    pub normalized_residual: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyParameterEstimateV1 {
    pub parameter_name: String,
    pub value: f64,
    pub is_fixed: bool,
}

/// The formal covariance matrix has shape free_parameter_count ×
/// free_parameter_count, ordered by free_parameter_indices. It is the covariance
/// implied by declared sigmas; posterior variance factor is reported separately.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SurveyAdjustmentResultV1 {
    pub schema_version: u16,
    pub adjustment_id: String,
    pub parameter_estimates: Vec<SurveyParameterEstimateV1>,
    pub free_parameter_indices: Vec<usize>,
    pub formal_covariance_row_major: Vec<f64>,
    pub residuals: Vec<SurveyAdjustmentResidualV1>,
    pub fitting_observation_count: usize,
    pub independent_checkpoint_count: usize,
    pub excluded_observation_count: usize,
    pub free_parameter_count: usize,
    pub degrees_of_freedom: usize,
    pub weighted_sum_squares: f64,
    pub posterior_variance_factor: Option<f64>,
    pub singular_values: Vec<f64>,
    pub relative_rank_tolerance: f64,
    pub provenance: SurveyAdjustmentProvenanceV1,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn artifact(id: &str) -> ArtifactRef {
        ArtifactRef {
            artifact_id: id.to_owned(),
            uri: format!("survey-adjustment-test://{id}"),
            sha256: format!("{:x}", Sha256::digest(id.as_bytes())),
        }
    }

    fn observation(
        id: &str,
        coefficients: &[f64],
        value: f64,
        sigma: f64,
    ) -> LinearizedSurveyObservationV1 {
        LinearizedSurveyObservationV1 {
            observation_id: id.to_owned(),
            coefficients: coefficients.to_vec(),
            value,
            standard_uncertainty: sigma,
        }
    }

    fn input() -> LinearizedSurveyAdjustmentV1 {
        LinearizedSurveyAdjustmentV1 {
            schema_version: SURVEY_ADJUSTMENT_SCHEMA_VERSION,
            adjustment_id: "control-step-001".to_owned(),
            parameter_names: vec!["east_delta_m".to_owned(), "north_delta_m".to_owned()],
            fitting_observations: vec![
                observation("fit-east-a", &[1.0, 0.0], 9.99, 0.1),
                observation("fit-east-b", &[1.0, 0.0], 10.01, 0.1),
                observation("fixed-north", &[0.0, 1.0], 5.0, 0.02),
            ],
            independent_checkpoints: vec![observation("check-east", &[1.0, 0.0], 10.03, 0.02)],
            excluded_observations: vec![observation("excluded-east", &[1.0, 0.0], 50.0, 0.01)],
            fixed_parameters: vec![FixedSurveyParameterV1 {
                parameter_index: 1,
                value: 5.0,
            }],
            relative_rank_tolerance: 1.0e-12,
            provenance: SurveyAdjustmentProvenanceV1 {
                algorithm_id: "survey-linearized-wls-svd".to_owned(),
                algorithm_version: "1.0.0".to_owned(),
                algorithm_build_sha256: artifact("build").sha256,
                configuration_sha256: artifact("configuration").sha256,
                source_artifacts: vec![artifact("raw-control-observations")],
            },
        }
    }

    #[test]
    fn weighted_solution_respects_fixed_datum_and_reports_covariance() {
        let result = input().solve().expect("full-rank survey adjustment");
        assert!((result.parameter_estimates[0].value - 10.0).abs() < 1.0e-10);
        assert_eq!(result.parameter_estimates[1].value, 5.0);
        assert!(result.parameter_estimates[1].is_fixed);
        assert_eq!(result.free_parameter_indices, vec![0]);
        assert_eq!(result.formal_covariance_row_major.len(), 1);
        assert!((result.formal_covariance_row_major[0] - 0.005).abs() < 1.0e-10);
        assert_eq!(result.degrees_of_freedom, 2);
        assert_eq!(result.independent_checkpoint_count, 1);
        assert_eq!(result.excluded_observation_count, 1);
    }

    #[test]
    fn two_parameter_unequal_weight_solution_matches_hand_computed_reference() {
        let mut adjustment = input();
        adjustment.parameter_names = vec!["east_m".to_owned(), "north_m".to_owned()];
        adjustment.fixed_parameters.clear();
        adjustment.fitting_observations = vec![
            observation("east", &[1.0, 0.0], 10.0, 1.0),
            observation("north", &[0.0, 1.0], 20.0, 2.0),
            observation("diagonal", &[1.0, 1.0], 31.0, 1.0),
        ];
        adjustment.independent_checkpoints.clear();
        adjustment.excluded_observations.clear();

        let result = adjustment.solve().expect("full-rank two-parameter network");
        assert!((result.parameter_estimates[0].value - 61.0 / 6.0).abs() < 1.0e-10);
        assert!((result.parameter_estimates[1].value - 62.0 / 3.0).abs() < 1.0e-10);
        assert_eq!(result.degrees_of_freedom, 1);
        assert_eq!(result.formal_covariance_row_major.len(), 4);
        assert!((result.formal_covariance_row_major[0] - 5.0 / 6.0).abs() < 1.0e-10);
        assert!((result.formal_covariance_row_major[1] + 2.0 / 3.0).abs() < 1.0e-10);
        assert!((result.formal_covariance_row_major[2] + 2.0 / 3.0).abs() < 1.0e-10);
        assert!((result.formal_covariance_row_major[3] - 4.0 / 3.0).abs() < 1.0e-10);
    }

    #[test]
    fn independent_checkpoints_never_change_the_fit() {
        let mut original = input();
        let original_result = original.solve().expect("solution");
        original.independent_checkpoints[0].value = -900.0;
        let changed_result = original.solve().expect("same fit solution");
        assert_eq!(
            original_result.parameter_estimates,
            changed_result.parameter_estimates
        );
        let checkpoint = changed_result
            .residuals
            .iter()
            .find(|residual| {
                residual.role == SurveyAdjustmentObservationRoleV1::IndependentCheckpoint
            })
            .expect("checkpoint residual retained");
        assert_eq!(checkpoint.residual, 910.0);
    }

    #[test]
    fn excluded_observations_never_change_the_fit() {
        let mut original = input();
        let expected = original.solve().expect("solution");
        original.excluded_observations[0].value = -1.0e9;
        let actual = original.solve().expect("solution unchanged");
        assert_eq!(expected.parameter_estimates, actual.parameter_estimates);
    }

    #[test]
    fn rank_deficiency_is_rejected_instead_of_regularized_silently() {
        let mut invalid = input();
        invalid.fixed_parameters.clear();
        invalid.parameter_names = vec!["east".to_owned(), "east_duplicate".to_owned()];
        invalid.fitting_observations = vec![
            observation("one", &[1.0, 1.0], 2.0, 1.0),
            observation("two", &[2.0, 2.0], 4.0, 1.0),
            observation("three", &[3.0, 3.0], 6.0, 1.0),
        ];
        assert_eq!(
            invalid.solve(),
            Err(SurveyAdjustmentErrorV1::RankDeficient {
                rank: 1,
                required_rank: 2,
            })
        );
    }

    #[test]
    fn duplicate_ids_across_fit_and_checkpoint_are_rejected() {
        let mut invalid = input();
        invalid.independent_checkpoints[0].observation_id =
            invalid.fitting_observations[0].observation_id.clone();
        assert_eq!(
            invalid.validate(),
            Err(SurveyAdjustmentErrorV1::DuplicateIdentity("observation_id"))
        );
    }

    #[test]
    fn non_positive_or_non_finite_uncertainty_is_rejected() {
        for sigma in [0.0, -1.0, f64::NAN, f64::INFINITY] {
            let mut invalid = input();
            invalid.fitting_observations[0].standard_uncertainty = sigma;
            assert_eq!(
                invalid.validate(),
                Err(SurveyAdjustmentErrorV1::InvalidStandardUncertainty)
            );
        }
    }

    #[test]
    fn non_finite_design_or_observation_is_rejected() {
        let mut bad_design = input();
        bad_design.fitting_observations[0].coefficients[0] = f64::NAN;
        assert_eq!(
            bad_design.validate(),
            Err(SurveyAdjustmentErrorV1::InvalidNumber("observation"))
        );
        let mut bad_value = input();
        bad_value.fitting_observations[0].value = f64::INFINITY;
        assert_eq!(
            bad_value.validate(),
            Err(SurveyAdjustmentErrorV1::InvalidNumber("observation"))
        );
    }

    #[test]
    fn duplicate_fixed_indices_and_out_of_range_indices_are_rejected() {
        let mut duplicate = input();
        duplicate
            .fixed_parameters
            .push(duplicate.fixed_parameters[0].clone());
        assert_eq!(
            duplicate.validate(),
            Err(SurveyAdjustmentErrorV1::DuplicateParameterIndex(1))
        );
        let mut out_of_range = input();
        out_of_range.fixed_parameters[0].parameter_index = 9;
        assert_eq!(
            out_of_range.validate(),
            Err(SurveyAdjustmentErrorV1::ParameterIndexOutOfRange(9))
        );
    }

    #[test]
    fn source_build_and_configuration_digests_are_required() {
        let mut invalid = input();
        invalid.provenance.configuration_sha256 = "not-a-digest".to_owned();
        assert_eq!(
            invalid.validate(),
            Err(SurveyAdjustmentErrorV1::InvalidDigest(
                "configuration_sha256"
            ))
        );
        let mut missing_source = input();
        missing_source.provenance.source_artifacts.clear();
        assert_eq!(
            missing_source.validate(),
            Err(SurveyAdjustmentErrorV1::EmptySet("source_artifacts"))
        );
    }

    #[test]
    fn covariance_is_not_scaled_by_the_posterior_factor_implicitly() {
        let result = input().solve().expect("solution");
        let covariance = result.formal_covariance_row_major[0];
        let factor = result.posterior_variance_factor.expect("positive dof");
        assert!(covariance > 0.0);
        assert!(factor.is_finite());
        assert!((covariance - 0.005).abs() < 1.0e-12);
    }

    #[test]
    fn deterministic_encoding_binds_exact_input_bytes() {
        let input = input();
        let (first_bytes, first_artifact) = input
            .encode_with_digest("control-adjustment", "survey-adjustment-test://input")
            .expect("valid input");
        let (second_bytes, second_artifact) = input
            .encode_with_digest("control-adjustment", "survey-adjustment-test://input")
            .expect("same deterministic bytes");
        assert_eq!(first_bytes, second_bytes);
        assert_eq!(first_artifact, second_artifact);
        let actual = format!("{:x}", Sha256::digest(&first_bytes));
        assert_eq!(actual, first_artifact.sha256);
    }
}
