// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Provenance-carrying first-order propagation of correlated uncertainty.
//!
//! This module applies a declared linear transform y = A x to an input vector
//! with a full covariance matrix Sigma, producing Sigma_y = A Sigma A^T.
//! Correlations are never inferred from file/source labels. The supplied
//! covariance is authoritative; source receipts preserve where uncertainty
//! came from but are not treated as independent variance terms.
//!
//! Structural model discrepancy is deliberately not represented as covariance.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum NumericUncertaintyKind {
    SamplingDesign,
    Measurement,
    Calibration,
    SpatialCrosswalk,
    TemporalAggregation,
    UnitConversion,
    Imputation,
    ParameterEstimation,
    OtherRegisteredNumeric,
}

impl NumericUncertaintyKind {
    pub const fn name(self) -> &'static str {
        match self {
            Self::SamplingDesign => "sampling_design",
            Self::Measurement => "measurement",
            Self::Calibration => "calibration",
            Self::SpatialCrosswalk => "spatial_crosswalk",
            Self::TemporalAggregation => "temporal_aggregation",
            Self::UnitConversion => "unit_conversion",
            Self::Imputation => "imputation",
            Self::ParameterEstimation => "parameter_estimation",
            Self::OtherRegisteredNumeric => "other_registered_numeric",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertaintySourceReceipt {
    pub source_id: String,
    pub kind: NumericUncertaintyKind,
    pub source_digest: String,
    /// Variables affected by this source. The dependence group identifies a
    /// shared cause; the covariance matrix, not this label, encodes its size.
    pub affected_variable_ids: Vec<String>,
    pub dependence_group_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UncertaintyVariable {
    pub variable_id: String,
    pub measurement_unit: String,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CovarianceMatrix {
    pub values: Vec<Vec<f64>>,
}

impl CovarianceMatrix {
    pub fn dimension(&self) -> usize {
        self.values.len()
    }

    pub fn validate(&self, expected_dimension: usize) -> Result<(), UncertaintyPropagationError> {
        if expected_dimension == 0
            || self.values.len() != expected_dimension
            || self
                .values
                .iter()
                .any(|row| row.len() != expected_dimension)
        {
            return Err(UncertaintyPropagationError::CovarianceDimensionMismatch);
        }
        if self.values.iter().flatten().any(|value| !value.is_finite()) {
            return Err(UncertaintyPropagationError::NonFiniteCovariance);
        }

        let scale = self
            .values
            .iter()
            .flatten()
            .map(|value| value.abs())
            .fold(1.0_f64, f64::max);
        let tolerance = 1.0e-10 * scale;

        for row in 0..expected_dimension {
            if self.values[row][row] < -tolerance {
                return Err(UncertaintyPropagationError::NegativeVariance {
                    variable_index: row,
                });
            }
            for column in 0..row {
                if (self.values[row][column] - self.values[column][row]).abs() > tolerance {
                    return Err(UncertaintyPropagationError::CovarianceNotSymmetric {
                        row,
                        column,
                    });
                }
            }
        }

        // Semidefinite Cholesky test. Zero pivots are allowed only when the
        // corresponding residual covariance is also zero within tolerance.
        let mut lower = vec![vec![0.0_f64; expected_dimension]; expected_dimension];
        for row in 0..expected_dimension {
            for column in 0..=row {
                let mut residual = self.values[row][column];
                for inner in 0..column {
                    residual -= lower[row][inner] * lower[column][inner];
                }
                if row == column {
                    if residual < -tolerance {
                        return Err(
                            UncertaintyPropagationError::CovarianceNotPositiveSemidefinite {
                                pivot_index: row,
                            },
                        );
                    }
                    lower[row][column] = residual.max(0.0).sqrt();
                } else if lower[column][column] > tolerance.sqrt() {
                    lower[row][column] = residual / lower[column][column];
                } else if residual.abs() > tolerance {
                    return Err(
                        UncertaintyPropagationError::CovarianceNotPositiveSemidefinite {
                            pivot_index: column,
                        },
                    );
                }
            }
        }
        Ok(())
    }

    pub fn flattened(&self) -> Vec<f64> {
        self.values.iter().flatten().copied().collect()
    }

    fn correlation_matrix(&self) -> Vec<Vec<Option<f64>>> {
        let standard_uncertainties: Vec<f64> = (0..self.dimension())
            .map(|index| self.values[index][index].max(0.0).sqrt())
            .collect();
        (0..self.dimension())
            .map(|row| {
                (0..self.dimension())
                    .map(|column| {
                        let denominator =
                            standard_uncertainties[row] * standard_uncertainties[column];
                        (denominator > 0.0).then(|| self.values[row][column] / denominator)
                    })
                    .collect()
            })
            .collect()
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct UncertaintyState {
    pub state_id: String,
    /// Canonical coordinate/unit shared by every variable in this v0.1 state.
    pub measurement_unit: String,
    pub variables: Vec<UncertaintyVariable>,
    pub estimates: Vec<f64>,
    pub covariance: CovarianceMatrix,
    pub uncertainty_sources: Vec<UncertaintySourceReceipt>,
}

impl UncertaintyState {
    pub fn validate(&self) -> Result<(), UncertaintyPropagationError> {
        nonempty(
            &self.state_id,
            UncertaintyPropagationError::MissingStateIdentity,
        )?;
        nonempty(
            &self.measurement_unit,
            UncertaintyPropagationError::MissingMeasurementUnit,
        )?;
        if self.variables.is_empty() || self.variables.len() != self.estimates.len() {
            return Err(UncertaintyPropagationError::VariableDimensionMismatch);
        }
        if self.estimates.iter().any(|value| !value.is_finite()) {
            return Err(UncertaintyPropagationError::NonFiniteEstimate);
        }
        let mut variable_ids = BTreeSet::new();
        for variable in &self.variables {
            nonempty(
                &variable.variable_id,
                UncertaintyPropagationError::MissingVariableIdentity,
            )?;
            nonempty(
                &variable.measurement_unit,
                UncertaintyPropagationError::MissingMeasurementUnit,
            )?;
            if variable.measurement_unit != self.measurement_unit {
                return Err(UncertaintyPropagationError::InputUnitMismatch {
                    variable_id: variable.variable_id.clone(),
                });
            }
            if !variable_ids.insert(variable.variable_id.as_str()) {
                return Err(UncertaintyPropagationError::DuplicateVariableIdentity {
                    variable_id: variable.variable_id.clone(),
                });
            }
        }
        self.covariance.validate(self.variables.len())?;
        if self.uncertainty_sources.is_empty()
            && self
                .covariance
                .values
                .iter()
                .flatten()
                .any(|value| value.abs() > 0.0)
        {
            return Err(UncertaintyPropagationError::MissingUncertaintySource);
        }

        let mut source_ids = BTreeSet::new();
        let mut attributed_variables = BTreeSet::new();
        for source in &self.uncertainty_sources {
            nonempty(
                &source.source_id,
                UncertaintyPropagationError::MissingUncertaintySourceIdentity,
            )?;
            nonempty(
                &source.source_digest,
                UncertaintyPropagationError::MissingSourceDigest,
            )?;
            if let Some(group_id) = source.dependence_group_id.as_deref() {
                nonempty(
                    group_id,
                    UncertaintyPropagationError::MissingDependenceGroupIdentity,
                )?;
            }
            if source.affected_variable_ids.is_empty() {
                return Err(UncertaintyPropagationError::SourceHasNoAffectedVariables {
                    source_id: source.source_id.clone(),
                });
            }
            if !source_ids.insert(source.source_id.as_str()) {
                return Err(UncertaintyPropagationError::DuplicateUncertaintySource {
                    source_id: source.source_id.clone(),
                });
            }
            let mut local_variables = BTreeSet::new();
            for variable_id in &source.affected_variable_ids {
                if !variable_ids.contains(variable_id.as_str()) {
                    return Err(UncertaintyPropagationError::UnknownAffectedVariable {
                        source_id: source.source_id.clone(),
                        variable_id: variable_id.clone(),
                    });
                }
                if !local_variables.insert(variable_id.as_str()) {
                    return Err(UncertaintyPropagationError::DuplicateAffectedVariable {
                        source_id: source.source_id.clone(),
                        variable_id: variable_id.clone(),
                    });
                }
                attributed_variables.insert(variable_id.as_str());
            }
        }
        for variable in &self.variables {
            if !attributed_variables.contains(variable.variable_id.as_str()) {
                return Err(
                    UncertaintyPropagationError::VariableHasNoUncertaintyLineage {
                        variable_id: variable.variable_id.clone(),
                    },
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct LinearTransform {
    pub transform_id: String,
    pub transform_digest: String,
    pub protocol_digest: String,
    pub input_state_id: String,
    pub output_state_id: String,
    /// Jacobian / linear operator. Every input and output is in the same
    /// registered canonical unit; unit conversion is a separate prior step.
    pub coefficients: Vec<Vec<f64>>,
    pub output_variable_ids: Vec<String>,
}

impl LinearTransform {
    fn validate(&self, input: &UncertaintyState) -> Result<(), UncertaintyPropagationError> {
        for (value, error) in [
            (
                &self.transform_id,
                UncertaintyPropagationError::MissingTransformIdentity,
            ),
            (
                &self.transform_digest,
                UncertaintyPropagationError::MissingTransformDigest,
            ),
            (
                &self.protocol_digest,
                UncertaintyPropagationError::MissingProtocolDigest,
            ),
            (
                &self.input_state_id,
                UncertaintyPropagationError::MissingTransformInputIdentity,
            ),
            (
                &self.output_state_id,
                UncertaintyPropagationError::MissingTransformOutputIdentity,
            ),
        ] {
            nonempty(value, error)?;
        }
        if self.input_state_id != input.state_id {
            return Err(UncertaintyPropagationError::InputStateIdentityMismatch);
        }
        if self.coefficients.is_empty()
            || self.output_variable_ids.len() != self.coefficients.len()
            || self
                .coefficients
                .iter()
                .any(|row| row.len() != input.variables.len())
        {
            return Err(UncertaintyPropagationError::TransformDimensionMismatch);
        }
        if self
            .coefficients
            .iter()
            .flatten()
            .any(|value| !value.is_finite())
        {
            return Err(UncertaintyPropagationError::NonFiniteTransformCoefficient);
        }
        let mut output_ids = BTreeSet::new();
        for id in &self.output_variable_ids {
            nonempty(
                id,
                UncertaintyPropagationError::MissingOutputVariableIdentity,
            )?;
            if !output_ids.insert(id.as_str()) {
                return Err(
                    UncertaintyPropagationError::DuplicateOutputVariableIdentity {
                        variable_id: id.clone(),
                    },
                );
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PropagatedUncertainty {
    pub output_state: UncertaintyState,
    pub input_state_id: String,
    pub input_variables: Vec<UncertaintyVariable>,
    pub input_estimates: Vec<f64>,
    pub input_covariance: CovarianceMatrix,
    pub input_uncertainty_sources: Vec<UncertaintySourceReceipt>,
    pub transform_id: String,
    pub transform_digest: String,
    pub protocol_digest: String,
    pub jacobian: Vec<Vec<f64>>,
    pub standard_uncertainties: Vec<f64>,
    pub correlation_matrix: Vec<Vec<Option<f64>>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UncertaintyPropagationError {
    MissingStateIdentity,
    MissingMeasurementUnit,
    VariableDimensionMismatch,
    NonFiniteEstimate,
    MissingVariableIdentity,
    DuplicateVariableIdentity {
        variable_id: String,
    },
    InputUnitMismatch {
        variable_id: String,
    },
    CovarianceDimensionMismatch,
    NonFiniteCovariance,
    NegativeVariance {
        variable_index: usize,
    },
    CovarianceNotSymmetric {
        row: usize,
        column: usize,
    },
    CovarianceNotPositiveSemidefinite {
        pivot_index: usize,
    },
    MissingUncertaintySource,
    MissingUncertaintySourceIdentity,
    MissingSourceDigest,
    MissingDependenceGroupIdentity,
    SourceHasNoAffectedVariables {
        source_id: String,
    },
    DuplicateUncertaintySource {
        source_id: String,
    },
    UnknownAffectedVariable {
        source_id: String,
        variable_id: String,
    },
    DuplicateAffectedVariable {
        source_id: String,
        variable_id: String,
    },
    VariableHasNoUncertaintyLineage {
        variable_id: String,
    },
    MissingTransformIdentity,
    MissingTransformDigest,
    MissingProtocolDigest,
    MissingTransformInputIdentity,
    MissingTransformOutputIdentity,
    InputStateIdentityMismatch,
    TransformDimensionMismatch,
    NonFiniteTransformCoefficient,
    MissingOutputVariableIdentity,
    DuplicateOutputVariableIdentity {
        variable_id: String,
    },
    NonFiniteOutputEstimate,
    NonFiniteOutputCovariance,
}

impl fmt::Display for UncertaintyPropagationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for UncertaintyPropagationError {}

fn nonempty(
    value: &str,
    error: UncertaintyPropagationError,
) -> Result<(), UncertaintyPropagationError> {
    if value.trim().is_empty() {
        Err(error)
    } else {
        Ok(())
    }
}

fn multiply(
    left: &[Vec<f64>],
    right: &[Vec<f64>],
) -> Result<Vec<Vec<f64>>, UncertaintyPropagationError> {
    if left.is_empty() || right.is_empty() || left[0].is_empty() {
        return Err(UncertaintyPropagationError::TransformDimensionMismatch);
    }
    let inner = left[0].len();
    if left.iter().any(|row| row.len() != inner) || right.len() != inner {
        return Err(UncertaintyPropagationError::TransformDimensionMismatch);
    }
    let columns = right[0].len();
    if right.iter().any(|row| row.len() != columns) {
        return Err(UncertaintyPropagationError::TransformDimensionMismatch);
    }
    let mut output = vec![vec![0.0; columns]; left.len()];
    for row in 0..left.len() {
        for column in 0..columns {
            output[row][column] = (0..inner)
                .map(|index| left[row][index] * right[index][column])
                .sum();
        }
    }
    if output.iter().flatten().any(|value| !value.is_finite()) {
        return Err(UncertaintyPropagationError::NonFiniteOutputCovariance);
    }
    Ok(output)
}

fn transpose(matrix: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let rows = matrix.len();
    let columns = matrix[0].len();
    (0..columns)
        .map(|column| (0..rows).map(|row| matrix[row][column]).collect())
        .collect()
}

/// Propagate a registered joint covariance through a declared linear map:
/// output mean = A * input mean; output covariance = A * Sigma * A^T.
///
/// All input and output variables are in one canonical unit. If a spatial,
/// temporal, calibration or imputation correction has uncertainty, represent
/// it as an additional input variable and include its covariance/correlation
/// in the joint input matrix. Do not assume independence unless the supplied
/// covariance actually encodes zero covariance.
pub fn propagate_linear_uncertainty(
    input: &UncertaintyState,
    transform: &LinearTransform,
) -> Result<PropagatedUncertainty, UncertaintyPropagationError> {
    input.validate()?;
    transform.validate(input)?;

    let output_estimates: Vec<f64> = transform
        .coefficients
        .iter()
        .map(|row| {
            row.iter()
                .zip(&input.estimates)
                .map(|(coefficient, estimate)| coefficient * estimate)
                .sum()
        })
        .collect();
    if output_estimates.iter().any(|value| !value.is_finite()) {
        return Err(UncertaintyPropagationError::NonFiniteOutputEstimate);
    }

    let covariance_times_jacobian_transpose = multiply(
        &input.covariance.values,
        &transpose(&transform.coefficients),
    )?;
    let output_covariance_values = multiply(
        &transform.coefficients,
        &covariance_times_jacobian_transpose,
    )?;
    let output_covariance = CovarianceMatrix {
        values: output_covariance_values,
    };
    output_covariance.validate(transform.output_variable_ids.len())?;

    let output_variables: Vec<UncertaintyVariable> = transform
        .output_variable_ids
        .iter()
        .map(|variable_id| UncertaintyVariable {
            variable_id: variable_id.clone(),
            measurement_unit: input.measurement_unit.clone(),
        })
        .collect();
    let input_indices: std::collections::BTreeMap<&str, usize> = input
        .variables
        .iter()
        .enumerate()
        .map(|(index, variable)| (variable.variable_id.as_str(), index))
        .collect();
    let mut output_sources = Vec::new();
    for source in &input.uncertainty_sources {
        let affected_input_indices: Vec<usize> = source
            .affected_variable_ids
            .iter()
            .filter_map(|variable_id| input_indices.get(variable_id.as_str()).copied())
            .collect();
        let affected_output_ids: Vec<String> = transform
            .coefficients
            .iter()
            .enumerate()
            .filter(|(_, row)| {
                affected_input_indices
                    .iter()
                    .any(|input_index| row[*input_index] != 0.0)
            })
            .map(|(output_index, _)| transform.output_variable_ids[output_index].clone())
            .collect();
        if !affected_output_ids.is_empty() {
            let mut propagated_source = source.clone();
            propagated_source.affected_variable_ids = affected_output_ids;
            output_sources.push(propagated_source);
        }
    }
    let output_state = UncertaintyState {
        state_id: transform.output_state_id.clone(),
        measurement_unit: input.measurement_unit.clone(),
        variables: output_variables,
        estimates: output_estimates,
        covariance: output_covariance.clone(),
        uncertainty_sources: output_sources,
    };
    output_state.validate()?;
    let standard_uncertainties = (0..output_covariance.dimension())
        .map(|index| output_covariance.values[index][index].max(0.0).sqrt())
        .collect();

    Ok(PropagatedUncertainty {
        output_state,
        input_state_id: input.state_id.clone(),
        input_variables: input.variables.clone(),
        input_estimates: input.estimates.clone(),
        input_covariance: input.covariance.clone(),
        input_uncertainty_sources: input.uncertainty_sources.clone(),
        transform_id: transform.transform_id.clone(),
        transform_digest: transform.transform_digest.clone(),
        protocol_digest: transform.protocol_digest.clone(),
        jacobian: transform.coefficients.clone(),
        standard_uncertainties,
        correlation_matrix: output_covariance.correlation_matrix(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn source(
        id: &str,
        kind: NumericUncertaintyKind,
        digest: &str,
        variables: &[&str],
        dependence_group: Option<&str>,
    ) -> UncertaintySourceReceipt {
        UncertaintySourceReceipt {
            source_id: id.to_string(),
            kind,
            source_digest: digest.to_string(),
            affected_variable_ids: variables.iter().map(|value| value.to_string()).collect(),
            dependence_group_id: dependence_group.map(str::to_string),
        }
    }

    fn state(
        state_id: &str,
        ids: &[&str],
        values: &[f64],
        covariance: Vec<Vec<f64>>,
        sources: Vec<UncertaintySourceReceipt>,
    ) -> UncertaintyState {
        UncertaintyState {
            state_id: state_id.to_string(),
            measurement_unit: "canonical_service_units".to_string(),
            variables: ids
                .iter()
                .map(|id| UncertaintyVariable {
                    variable_id: (*id).to_string(),
                    measurement_unit: "canonical_service_units".to_string(),
                })
                .collect(),
            estimates: values.to_vec(),
            covariance: CovarianceMatrix { values: covariance },
            uncertainty_sources: sources,
        }
    }

    fn transform(
        input: &UncertaintyState,
        output_ids: &[&str],
        coefficients: Vec<Vec<f64>>,
        transform_id: &str,
    ) -> LinearTransform {
        LinearTransform {
            transform_id: transform_id.to_string(),
            transform_digest: format!("sha256-{transform_id}"),
            protocol_digest: "sha256-linear-propagation-protocol-v1".to_string(),
            input_state_id: input.state_id.clone(),
            output_state_id: format!("output-{transform_id}"),
            coefficients,
            output_variable_ids: output_ids.iter().map(|id| id.to_string()).collect(),
        }
    }

    #[test]
    fn correlated_sum_and_difference_retain_off_diagonal_covariance() {
        let input = state(
            "input-correlated",
            &["water-service", "energy-service"],
            &[100.0, 80.0],
            vec![vec![4.0, 3.0], vec![3.0, 9.0]],
            vec![
                source(
                    "water-survey",
                    NumericUncertaintyKind::SamplingDesign,
                    "sha256-water-survey",
                    &["water-service"],
                    Some("common-household-sample"),
                ),
                source(
                    "energy-survey",
                    NumericUncertaintyKind::SamplingDesign,
                    "sha256-energy-survey",
                    &["energy-service"],
                    Some("common-household-sample"),
                ),
            ],
        );
        let result = propagate_linear_uncertainty(
            &input,
            &transform(
                &input,
                &["combined-service", "service-difference"],
                vec![vec![1.0, 1.0], vec![1.0, -1.0]],
                "sum-difference",
            ),
        )
        .expect("correlated uncertainty propagation");

        assert_eq!(result.output_state.estimates, vec![180.0, 20.0]);
        assert_eq!(
            result.output_state.covariance.values,
            vec![vec![19.0, -5.0], vec![-5.0, 7.0]]
        );
        assert_eq!(
            result.standard_uncertainties,
            vec![19.0_f64.sqrt(), 7.0_f64.sqrt()]
        );
        assert_eq!(result.correlation_matrix.len(), 2);
        assert!(result.correlation_matrix[0][1].expect("defined correlation") < 0.0);
        assert_eq!(result.output_state.uncertainty_sources.len(), 2);
    }

    #[test]
    fn common_cause_error_reinforces_sum_and_cancels_difference() {
        let input = state(
            "input-common-cause",
            &["meter-a", "meter-b"],
            &[10.0, 12.0],
            vec![vec![4.0, 4.0], vec![4.0, 4.0]],
            vec![source(
                "shared-calibration",
                NumericUncertaintyKind::Calibration,
                "sha256-shared-calibration",
                &["meter-a", "meter-b"],
                Some("calibration-cause-1"),
            )],
        );
        let result = propagate_linear_uncertainty(
            &input,
            &transform(
                &input,
                &["sum", "difference"],
                vec![vec![1.0, 1.0], vec![1.0, -1.0]],
                "shared-cause-sum-difference",
            ),
        )
        .expect("shared cause propagation");

        assert_eq!(
            result.output_state.covariance.values,
            vec![vec![16.0, 0.0], vec![0.0, 0.0]]
        );
        assert_eq!(result.standard_uncertainties, vec![4.0, 0.0]);
        assert_eq!(result.correlation_matrix[1][1], None);
    }

    #[test]
    fn transform_uncertainty_is_explicitly_carried_as_an_input_variable() {
        let input = state(
            "input-with-crosswalk-error",
            &["observed-value", "crosswalk-adjustment"],
            &[100.0, 2.0],
            vec![vec![4.0, 0.5], vec![0.5, 1.0]],
            vec![
                source(
                    "survey-sampling",
                    NumericUncertaintyKind::SamplingDesign,
                    "sha256-survey-sampling",
                    &["observed-value"],
                    None,
                ),
                source(
                    "spatial-crosswalk",
                    NumericUncertaintyKind::SpatialCrosswalk,
                    "sha256-crosswalk",
                    &["crosswalk-adjustment"],
                    Some("boundary-allocation"),
                ),
            ],
        );
        let result = propagate_linear_uncertainty(
            &input,
            &transform(
                &input,
                &["harmonized-estimate"],
                vec![vec![1.0, 1.0]],
                "apply-crosswalk-adjustment",
            ),
        )
        .expect("crosswalk uncertainty propagation");

        assert_eq!(result.output_state.estimates, vec![102.0]);
        assert_eq!(result.output_state.covariance.values, vec![vec![6.0]]);
        assert_eq!(result.standard_uncertainties, vec![6.0_f64.sqrt()]);
        assert_eq!(result.output_state.uncertainty_sources.len(), 2);
    }

    #[test]
    fn propagated_output_state_remaps_uncertainty_sources_to_output_variables() {
        let input = state(
            "input-lineage",
            &["water", "energy"],
            &[10.0, 20.0],
            vec![vec![1.0, 0.2], vec![0.2, 4.0]],
            vec![
                source(
                    "water-source",
                    NumericUncertaintyKind::SamplingDesign,
                    "sha256-water",
                    &["water"],
                    None,
                ),
                source(
                    "energy-source",
                    NumericUncertaintyKind::Measurement,
                    "sha256-energy",
                    &["energy"],
                    None,
                ),
            ],
        );
        let result = propagate_linear_uncertainty(
            &input,
            &transform(
                &input,
                &["combined", "water-only"],
                vec![vec![1.0, 1.0], vec![1.0, 0.0]],
                "lineage-remap",
            ),
        )
        .expect("source lineage maps through transform");
        result
            .output_state
            .validate()
            .expect("output state is independently valid");
        let water = result
            .output_state
            .uncertainty_sources
            .iter()
            .find(|source| source.source_id == "water-source")
            .expect("water source retained");
        assert_eq!(
            water.affected_variable_ids,
            vec!["combined".to_string(), "water-only".to_string()]
        );
        let energy = result
            .output_state
            .uncertainty_sources
            .iter()
            .find(|source| source.source_id == "energy-source")
            .expect("energy source retained");
        assert_eq!(energy.affected_variable_ids, vec!["combined".to_string()]);
    }

    #[test]
    fn independent_inputs_are_only_independent_when_covariance_says_so() {
        let correlated = state(
            "input-correlation",
            &["x1", "x2"],
            &[3.0, 4.0],
            vec![vec![4.0, 3.0], vec![3.0, 9.0]],
            vec![
                source(
                    "source-1",
                    NumericUncertaintyKind::Measurement,
                    "sha256-s1",
                    &["x1"],
                    None,
                ),
                source(
                    "source-2",
                    NumericUncertaintyKind::Measurement,
                    "sha256-s2",
                    &["x2"],
                    None,
                ),
            ],
        );
        let result = propagate_linear_uncertainty(
            &correlated,
            &transform(&correlated, &["sum"], vec![vec![1.0, 1.0]], "sum"),
        )
        .expect("covariance matrix is explicit");
        assert_eq!(result.output_state.covariance.values, vec![vec![19.0]]);
    }

    #[test]
    fn rejects_non_square_asymmetric_and_non_psd_covariance() {
        let bad_dimension = state(
            "bad-dimension",
            &["x1", "x2"],
            &[1.0, 2.0],
            vec![vec![1.0]],
            vec![source(
                "src",
                NumericUncertaintyKind::Measurement,
                "sha256-src",
                &["x1", "x2"],
                None,
            )],
        );
        assert_eq!(
            propagate_linear_uncertainty(
                &bad_dimension,
                &transform(&bad_dimension, &["y"], vec![vec![1.0, 1.0]], "bad-dim")
            )
            .unwrap_err(),
            UncertaintyPropagationError::CovarianceDimensionMismatch
        );

        let asymmetric = state(
            "asymmetric",
            &["x1", "x2"],
            &[1.0, 2.0],
            vec![vec![1.0, 0.1], vec![0.2, 1.0]],
            vec![source(
                "src",
                NumericUncertaintyKind::Measurement,
                "sha256-src",
                &["x1", "x2"],
                None,
            )],
        );
        assert!(matches!(
            propagate_linear_uncertainty(
                &asymmetric,
                &transform(&asymmetric, &["y"], vec![vec![1.0, 1.0]], "asym")
            ),
            Err(UncertaintyPropagationError::CovarianceNotSymmetric { .. })
        ));

        let non_psd = state(
            "non-psd",
            &["x1", "x2"],
            &[1.0, 2.0],
            vec![vec![1.0, 2.0], vec![2.0, 1.0]],
            vec![source(
                "src",
                NumericUncertaintyKind::Measurement,
                "sha256-src",
                &["x1", "x2"],
                None,
            )],
        );
        assert!(matches!(
            propagate_linear_uncertainty(
                &non_psd,
                &transform(&non_psd, &["y"], vec![vec![1.0, 1.0]], "non-psd")
            ),
            Err(UncertaintyPropagationError::CovarianceNotPositiveSemidefinite { .. })
        ));
    }

    #[test]
    fn rejects_missing_duplicate_and_unknown_source_lineage() {
        let missing_source = state(
            "missing-source",
            &["x"],
            &[1.0],
            vec![vec![1.0]],
            Vec::new(),
        );
        assert_eq!(
            propagate_linear_uncertainty(
                &missing_source,
                &transform(&missing_source, &["y"], vec![vec![1.0]], "missing")
            )
            .unwrap_err(),
            UncertaintyPropagationError::MissingUncertaintySource
        );

        let duplicate_source = state(
            "duplicate-source",
            &["x"],
            &[1.0],
            vec![vec![1.0]],
            vec![
                source(
                    "duplicate",
                    NumericUncertaintyKind::Measurement,
                    "sha256-a",
                    &["x"],
                    None,
                ),
                source(
                    "duplicate",
                    NumericUncertaintyKind::Calibration,
                    "sha256-b",
                    &["x"],
                    None,
                ),
            ],
        );
        assert!(matches!(
            propagate_linear_uncertainty(
                &duplicate_source,
                &transform(&duplicate_source, &["y"], vec![vec![1.0]], "dup")
            ),
            Err(UncertaintyPropagationError::DuplicateUncertaintySource { .. })
        ));

        let unknown_variable = state(
            "unknown-variable",
            &["x"],
            &[1.0],
            vec![vec![1.0]],
            vec![source(
                "source",
                NumericUncertaintyKind::Measurement,
                "sha256-source",
                &["other"],
                None,
            )],
        );
        assert!(matches!(
            propagate_linear_uncertainty(
                &unknown_variable,
                &transform(&unknown_variable, &["y"], vec![vec![1.0]], "unknown")
            ),
            Err(UncertaintyPropagationError::UnknownAffectedVariable { .. })
        ));
    }

    #[test]
    fn rejects_wrong_input_identity_and_non_finite_jacobian() {
        let input = state(
            "identity",
            &["x"],
            &[2.0],
            vec![vec![1.0]],
            vec![source(
                "src",
                NumericUncertaintyKind::Measurement,
                "sha256-src",
                &["x"],
                None,
            )],
        );
        let mut wrong_input = transform(&input, &["y"], vec![vec![1.0]], "wrong-input");
        wrong_input.input_state_id = "some-other-state".to_string();
        assert_eq!(
            propagate_linear_uncertainty(&input, &wrong_input).unwrap_err(),
            UncertaintyPropagationError::InputStateIdentityMismatch
        );

        let mut non_finite = transform(&input, &["y"], vec![vec![f64::NAN]], "nan");
        non_finite.input_state_id = input.state_id.clone();
        assert_eq!(
            propagate_linear_uncertainty(&input, &non_finite).unwrap_err(),
            UncertaintyPropagationError::NonFiniteTransformCoefficient
        );
    }

    #[test]
    fn zero_variance_directions_are_valid_and_deterministic() {
        let input = state(
            "zero-direction",
            &["x1", "x2"],
            &[5.0, 5.0],
            vec![vec![1.0, 1.0], vec![1.0, 1.0]],
            vec![source(
                "shared",
                NumericUncertaintyKind::Calibration,
                "sha256-shared",
                &["x1", "x2"],
                Some("same-error"),
            )],
        );
        let transform = transform(
            &input,
            &["difference"],
            vec![vec![1.0, -1.0]],
            "difference-zero-variance",
        );
        let first = propagate_linear_uncertainty(&input, &transform).expect("valid");
        let second = propagate_linear_uncertainty(&input, &transform).expect("valid");
        assert_eq!(first, second);
        assert_eq!(first.output_state.covariance.values, vec![vec![0.0]]);
        assert_eq!(first.standard_uncertainties, vec![0.0]);
    }
}
