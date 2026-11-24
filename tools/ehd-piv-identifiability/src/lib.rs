// SPDX-License-Identifier: AGPL-3.0-or-later

//! Local identifiability diagnostics for physical PIV parameters in the
//! presence of measurement nuisance parameters.
//!
//! Input rows are residual sensitivities already whitened by a qualified
//! observation-noise covariance. Each column is multiplied by its declared
//! characteristic parameter scale before analysis. The kernel then:
//! - performs a deterministic one-sided Jacobi SVD;
//! - estimates joint local numerical rank and weak parameter combinations;
//! - projects physical sensitivities out of the span of nuisance sensitivities;
//! - assesses physical identifiability after nuisance profiling;
//! - reports pairwise physical/nuisance sensitivity correlations.
//!
//! This is a local linear diagnostic, not a global or structural identifiability
//! proof. A full-rank Jacobian does not prove that a nonlinear likelihood is
//! globally unimodal or that the model is physically correct.

#![forbid(unsafe_code)]

use std::cmp::Ordering;
use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ParameterRole {
    Physical,
    Nuisance,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterSpec {
    pub id: String,
    pub name: String,
    pub role: ParameterRole,
    /// Converts a parameter perturbation in declared units to the
    /// characteristic perturbation represented by one scaled Jacobian unit.
    pub characteristic_scale: f64,
    pub unit: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdentifiabilityConfig {
    /// Relative singular-value threshold for numerical rank.
    pub rank_relative_tolerance: f64,
    /// A physical or joint condition number above this threshold is
    /// categorized as ill-conditioned rather than accepted.
    pub maximum_condition_number: f64,
    /// Absolute floor on a nuisance-profiled physical singular value.
    /// Sensitivities are in whitened residual units per characteristic
    /// parameter perturbation.
    pub minimum_profiled_physical_sensitivity: f64,
    /// Absolute correlation threshold for a physical/nuisance column pair.
    pub physical_nuisance_correlation_threshold: f64,
    /// Relative cross-column correlation at which a Jacobi rotation is made.
    pub jacobi_tolerance: f64,
    pub maximum_jacobi_sweeps: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdentifiabilityInput {
    pub parameters: Vec<ParameterSpec>,
    /// Row-major residual-sensitivity matrix: observations x parameters.
    ///
    /// Residual rows MUST already be whitened with the admitted observation
    /// covariance. The implementation scales each column by the matching
    /// ParameterSpec.characteristic_scale.
    pub whitened_sensitivity_matrix: Vec<Vec<f64>>,
    pub config: IdentifiabilityConfig,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IdentifiabilityStatus {
    Identifiable,
    IllConditioned,
    RankDeficient,
    NoInformativeSensitivity,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ParameterSensitivity {
    pub parameter_id: String,
    pub role: ParameterRole,
    pub scaled_sensitivity_norm: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DirectionCoefficient {
    pub parameter_id: String,
    pub coefficient: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WeakParameterDirection {
    pub singular_value: f64,
    pub coefficients: Vec<DirectionCoefficient>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhysicalNuisanceConfounding {
    pub physical_parameter_id: String,
    pub nuisance_parameter_id: String,
    /// Signed normalized sensitivity-column correlation in [-1, 1].
    pub signed_correlation: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IdentifiabilityReport {
    pub observation_count: usize,
    pub parameter_count: usize,
    pub physical_parameter_count: usize,
    pub nuisance_parameter_count: usize,
    pub joint_numerical_rank: usize,
    pub joint_singular_values: Vec<f64>,
    pub joint_condition_number: Option<f64>,
    pub joint_status: IdentifiabilityStatus,
    pub nuisance_numerical_rank: usize,
    pub unprofiled_physical_singular_values: Vec<f64>,
    pub nuisance_profiled_physical_singular_values: Vec<f64>,
    pub nuisance_profiled_physical_rank: usize,
    pub nuisance_profiled_physical_condition_number: Option<f64>,
    pub physical_status: IdentifiabilityStatus,
    pub parameter_sensitivities: Vec<ParameterSensitivity>,
    pub weak_joint_parameter_directions: Vec<WeakParameterDirection>,
    pub physical_nuisance_confounding: Vec<PhysicalNuisanceConfounding>,
    pub jacobi_sweeps: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum IdentifiabilityError {
    EmptyObservationMatrix,
    EmptyParameterSet,
    TooManyObservations {
        count: usize,
    },
    TooManyParameters {
        count: usize,
    },
    NoPhysicalParameters,
    RaggedSensitivityMatrix {
        row: usize,
        expected: usize,
        actual: usize,
    },
    EmptyParameterIdentity {
        index: usize,
        field: &'static str,
    },
    DuplicateParameterId {
        id: String,
    },
    InvalidCharacteristicScale {
        parameter_id: String,
    },
    NonFiniteSensitivity {
        row: usize,
        column: usize,
    },
    SensitivityScalingOverflow {
        row: usize,
        column: usize,
    },
    InvalidConfiguration {
        field: &'static str,
    },
    NumericalOverflow,
    JacobiDidNotConverge {
        sweeps: usize,
    },
}

impl fmt::Display for IdentifiabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            f,
            "invalid PIV identifiability input or decomposition: {self:?}"
        )
    }
}

impl Error for IdentifiabilityError {}

#[derive(Clone, Debug)]
struct SvdResult {
    singular_values: Vec<f64>,
    /// Right singular vectors sorted to match singular_values. Each vector
    /// contains one coefficient per input column.
    right_vectors: Vec<Vec<f64>>,
    sweeps: usize,
}

pub fn analyze_local_identifiability(
    input: &IdentifiabilityInput,
) -> Result<IdentifiabilityReport, IdentifiabilityError> {
    validate_input(input)?;
    let observation_count = input.whitened_sensitivity_matrix.len();
    let parameter_count = input.parameters.len();

    let mut columns = vec![vec![0.0; observation_count]; parameter_count];
    for (row_index, row) in input.whitened_sensitivity_matrix.iter().enumerate() {
        for (column_index, value) in row.iter().enumerate() {
            let scaled = value * input.parameters[column_index].characteristic_scale;
            if !scaled.is_finite() {
                return Err(IdentifiabilityError::SensitivityScalingOverflow {
                    row: row_index,
                    column: column_index,
                });
            }
            columns[column_index][row_index] = scaled;
        }
    }

    let joint_svd = jacobi_svd(&columns, &input.config)?;
    let joint_max = joint_svd.singular_values.first().copied().unwrap_or(0.0);
    let joint_threshold = joint_max * input.config.rank_relative_tolerance;
    let joint_rank = count_above(&joint_svd.singular_values, joint_threshold);
    let joint_condition_number =
        condition_number(&joint_svd.singular_values, joint_rank, parameter_count);
    let joint_status = set_status_threshold(
        classify_status(joint_rank, parameter_count, joint_condition_number),
        joint_condition_number,
        input.config.maximum_condition_number,
    );

    let physical_indices: Vec<usize> = input
        .parameters
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            (parameter.role == ParameterRole::Physical).then_some(index)
        })
        .collect();
    let nuisance_indices: Vec<usize> = input
        .parameters
        .iter()
        .enumerate()
        .filter_map(|(index, parameter)| {
            (parameter.role == ParameterRole::Nuisance).then_some(index)
        })
        .collect();

    let nuisance_columns: Vec<Vec<f64>> = nuisance_indices
        .iter()
        .map(|index| columns[*index].clone())
        .collect();
    let nuisance_svd = if nuisance_columns.is_empty() {
        None
    } else {
        Some(jacobi_svd(&nuisance_columns, &input.config)?)
    };
    let nuisance_max = nuisance_svd
        .as_ref()
        .and_then(|svd| svd.singular_values.first().copied())
        .unwrap_or(0.0);
    let nuisance_threshold = (nuisance_max * input.config.rank_relative_tolerance)
        .max(input.config.minimum_profiled_physical_sensitivity);
    let nuisance_rank = nuisance_svd.as_ref().map_or(0, |svd| {
        count_above(&svd.singular_values, nuisance_threshold)
    });

    let nuisance_basis = if let Some(svd) = &nuisance_svd {
        left_singular_basis(&nuisance_columns, svd, nuisance_threshold)?
    } else {
        Vec::new()
    };

    let physical_columns: Vec<Vec<f64>> = physical_indices
        .iter()
        .map(|index| columns[*index].clone())
        .collect();
    let unprofiled_physical_svd = jacobi_svd(&physical_columns, &input.config)?;
    let unprofiled_physical_max = unprofiled_physical_svd
        .singular_values
        .first()
        .copied()
        .unwrap_or(0.0);

    let profiled_physical_columns: Vec<Vec<f64>> = physical_columns
        .iter()
        .map(|column| project_onto_complement(column, &nuisance_basis))
        .collect::<Result<Vec<_>, _>>()?;
    let profiled_physical_svd = jacobi_svd(&profiled_physical_columns, &input.config)?;

    let physical_threshold = (unprofiled_physical_max * input.config.rank_relative_tolerance)
        .max(input.config.minimum_profiled_physical_sensitivity);
    let physical_rank = count_above(&profiled_physical_svd.singular_values, physical_threshold);
    let physical_condition_number = if physical_rank == physical_indices.len() && physical_rank > 0
    {
        let weakest = profiled_physical_svd
            .singular_values
            .get(physical_rank - 1)
            .copied()
            .unwrap_or(0.0);
        if weakest > 0.0 {
            Some(unprofiled_physical_max / weakest)
        } else {
            None
        }
    } else {
        None
    };
    let physical_base_status =
        if unprofiled_physical_max < input.config.minimum_profiled_physical_sensitivity {
            IdentifiabilityStatus::NoInformativeSensitivity
        } else if physical_rank < physical_indices.len() {
            IdentifiabilityStatus::RankDeficient
        } else {
            IdentifiabilityStatus::Identifiable
        };
    let physical_status = set_status_threshold(
        physical_base_status,
        physical_condition_number,
        input.config.maximum_condition_number,
    );

    let mut parameter_sensitivities = Vec::with_capacity(parameter_count);
    for (index, parameter) in input.parameters.iter().enumerate() {
        parameter_sensitivities.push(ParameterSensitivity {
            parameter_id: parameter.id.clone(),
            role: parameter.role,
            scaled_sensitivity_norm: vector_norm(&columns[index])?,
        });
    }

    let weak_cutoff = if joint_max == 0.0 {
        0.0
    } else {
        (joint_max * input.config.rank_relative_tolerance)
            .max(joint_max / input.config.maximum_condition_number)
    };
    let mut weak_joint_parameter_directions = Vec::new();
    for (index, singular_value) in joint_svd.singular_values.iter().enumerate() {
        if *singular_value <= weak_cutoff {
            let coefficients = joint_svd.right_vectors[index]
                .iter()
                .enumerate()
                .map(|(parameter_index, coefficient)| DirectionCoefficient {
                    parameter_id: input.parameters[parameter_index].id.clone(),
                    coefficient: *coefficient,
                })
                .collect();
            weak_joint_parameter_directions.push(WeakParameterDirection {
                singular_value: *singular_value,
                coefficients,
            });
        }
    }

    let mut physical_nuisance_confounding = Vec::new();
    for physical_index in &physical_indices {
        for nuisance_index in &nuisance_indices {
            let correlation =
                normalized_correlation(&columns[*physical_index], &columns[*nuisance_index])?;
            if let Some(signed_correlation) = correlation {
                if signed_correlation.abs() >= input.config.physical_nuisance_correlation_threshold
                {
                    physical_nuisance_confounding.push(PhysicalNuisanceConfounding {
                        physical_parameter_id: input.parameters[*physical_index].id.clone(),
                        nuisance_parameter_id: input.parameters[*nuisance_index].id.clone(),
                        signed_correlation,
                    });
                }
            }
        }
    }

    Ok(IdentifiabilityReport {
        observation_count,
        parameter_count,
        physical_parameter_count: physical_indices.len(),
        nuisance_parameter_count: nuisance_indices.len(),
        joint_numerical_rank: joint_rank,
        joint_singular_values: joint_svd.singular_values,
        joint_condition_number,
        joint_status,
        nuisance_numerical_rank: nuisance_rank,
        unprofiled_physical_singular_values: unprofiled_physical_svd.singular_values,
        nuisance_profiled_physical_singular_values: profiled_physical_svd.singular_values,
        nuisance_profiled_physical_rank: physical_rank,
        nuisance_profiled_physical_condition_number: physical_condition_number,
        physical_status,
        parameter_sensitivities,
        weak_joint_parameter_directions,
        physical_nuisance_confounding,
        jacobi_sweeps: joint_svd.sweeps,
    })
}

fn validate_input(input: &IdentifiabilityInput) -> Result<(), IdentifiabilityError> {
    if input.whitened_sensitivity_matrix.is_empty() {
        return Err(IdentifiabilityError::EmptyObservationMatrix);
    }
    if input.parameters.is_empty() {
        return Err(IdentifiabilityError::EmptyParameterSet);
    }
    let row_count = input.whitened_sensitivity_matrix.len();
    let column_count = input.parameters.len();
    if row_count > 65_536 {
        return Err(IdentifiabilityError::TooManyObservations { count: row_count });
    }
    if column_count > 64 {
        return Err(IdentifiabilityError::TooManyParameters {
            count: column_count,
        });
    }
    if !input
        .parameters
        .iter()
        .any(|parameter| parameter.role == ParameterRole::Physical)
    {
        return Err(IdentifiabilityError::NoPhysicalParameters);
    }

    let mut seen_ids = std::collections::BTreeSet::new();
    for (index, parameter) in input.parameters.iter().enumerate() {
        if parameter.id.trim().is_empty() {
            return Err(IdentifiabilityError::EmptyParameterIdentity { index, field: "id" });
        }
        if parameter.name.trim().is_empty() {
            return Err(IdentifiabilityError::EmptyParameterIdentity {
                index,
                field: "name",
            });
        }
        if parameter.unit.trim().is_empty() {
            return Err(IdentifiabilityError::EmptyParameterIdentity {
                index,
                field: "unit",
            });
        }
        if !seen_ids.insert(parameter.id.as_str()) {
            return Err(IdentifiabilityError::DuplicateParameterId {
                id: parameter.id.clone(),
            });
        }
        if !parameter.characteristic_scale.is_finite() || parameter.characteristic_scale <= 0.0 {
            return Err(IdentifiabilityError::InvalidCharacteristicScale {
                parameter_id: parameter.id.clone(),
            });
        }
    }

    for (row_index, row) in input.whitened_sensitivity_matrix.iter().enumerate() {
        if row.len() != column_count {
            return Err(IdentifiabilityError::RaggedSensitivityMatrix {
                row: row_index,
                expected: column_count,
                actual: row.len(),
            });
        }
        for (column_index, value) in row.iter().enumerate() {
            if !value.is_finite() {
                return Err(IdentifiabilityError::NonFiniteSensitivity {
                    row: row_index,
                    column: column_index,
                });
            }
        }
    }

    let config = &input.config;
    if !config.rank_relative_tolerance.is_finite()
        || config.rank_relative_tolerance <= 0.0
        || config.rank_relative_tolerance >= 1.0
    {
        return Err(IdentifiabilityError::InvalidConfiguration {
            field: "rank_relative_tolerance",
        });
    }
    if !config.maximum_condition_number.is_finite() || config.maximum_condition_number <= 1.0 {
        return Err(IdentifiabilityError::InvalidConfiguration {
            field: "maximum_condition_number",
        });
    }
    if !config.minimum_profiled_physical_sensitivity.is_finite()
        || config.minimum_profiled_physical_sensitivity <= 0.0
    {
        return Err(IdentifiabilityError::InvalidConfiguration {
            field: "minimum_profiled_physical_sensitivity",
        });
    }
    if !config.physical_nuisance_correlation_threshold.is_finite()
        || !(0.0..=1.0).contains(&config.physical_nuisance_correlation_threshold)
        || config.physical_nuisance_correlation_threshold == 0.0
    {
        return Err(IdentifiabilityError::InvalidConfiguration {
            field: "physical_nuisance_correlation_threshold",
        });
    }
    if !config.jacobi_tolerance.is_finite()
        || config.jacobi_tolerance <= 0.0
        || config.jacobi_tolerance >= 1.0
    {
        return Err(IdentifiabilityError::InvalidConfiguration {
            field: "jacobi_tolerance",
        });
    }
    if config.maximum_jacobi_sweeps == 0 || config.maximum_jacobi_sweeps > 10_000 {
        return Err(IdentifiabilityError::InvalidConfiguration {
            field: "maximum_jacobi_sweeps",
        });
    }
    Ok(())
}

fn jacobi_svd(
    input_columns: &[Vec<f64>],
    config: &IdentifiabilityConfig,
) -> Result<SvdResult, IdentifiabilityError> {
    let column_count = input_columns.len();
    if column_count == 0 {
        return Ok(SvdResult {
            singular_values: Vec::new(),
            right_vectors: Vec::new(),
            sweeps: 0,
        });
    }
    let row_count = input_columns[0].len();
    let mut columns = input_columns.to_vec();
    let mut right_columns = vec![vec![0.0; column_count]; column_count];
    for (index, column) in right_columns.iter_mut().enumerate() {
        column[index] = 1.0;
    }

    let mut converged = false;
    let mut sweeps = 0;
    for sweep in 0..config.maximum_jacobi_sweeps {
        let mut rotated = false;
        for first in 0..column_count {
            for second in (first + 1)..column_count {
                let alpha = dot(&columns[first], &columns[first])?;
                let beta = dot(&columns[second], &columns[second])?;
                let gamma = dot(&columns[first], &columns[second])?;
                if alpha == 0.0 || beta == 0.0 {
                    continue;
                }
                let denominator = alpha.sqrt() * beta.sqrt();
                let correlation = gamma.abs() / denominator;
                if correlation <= config.jacobi_tolerance {
                    continue;
                }

                let tau = (beta - alpha) / (2.0 * gamma);
                if !tau.is_finite() {
                    return Err(IdentifiabilityError::NumericalOverflow);
                }
                let tau_hypot = tau.hypot(1.0);
                let tangent = if tau >= 0.0 {
                    1.0 / (tau + tau_hypot)
                } else {
                    -1.0 / (-tau + tau_hypot)
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = cosine * tangent;

                for row in 0..row_count {
                    let left = columns[first][row];
                    let right = columns[second][row];
                    columns[first][row] = cosine * left - sine * right;
                    columns[second][row] = sine * left + cosine * right;
                }
                for row in 0..column_count {
                    let left = right_columns[first][row];
                    let right = right_columns[second][row];
                    right_columns[first][row] = cosine * left - sine * right;
                    right_columns[second][row] = sine * left + cosine * right;
                }
                rotated = true;
            }
        }
        sweeps = sweep + 1;
        if !rotated {
            converged = true;
            break;
        }
    }
    if !converged {
        return Err(IdentifiabilityError::JacobiDidNotConverge { sweeps });
    }

    let mut norms = Vec::with_capacity(column_count);
    for column in &columns {
        norms.push(vector_norm(column)?);
    }
    let mut order: Vec<usize> = (0..column_count).collect();
    order.sort_by(|left, right| {
        norms[*right]
            .partial_cmp(&norms[*left])
            .unwrap_or(Ordering::Equal)
    });

    let singular_values = order.iter().map(|index| norms[*index]).collect();
    let right_vectors = order
        .iter()
        .map(|index| right_columns[*index].clone())
        .collect();
    Ok(SvdResult {
        singular_values,
        right_vectors,
        sweeps,
    })
}

fn left_singular_basis(
    columns: &[Vec<f64>],
    svd: &SvdResult,
    singular_threshold: f64,
) -> Result<Vec<Vec<f64>>, IdentifiabilityError> {
    let row_count = columns.first().map_or(0, Vec::len);
    let mut basis = Vec::new();
    for (direction_index, singular_value) in svd.singular_values.iter().enumerate() {
        if *singular_value <= singular_threshold || *singular_value == 0.0 {
            continue;
        }
        let mut vector = vec![0.0; row_count];
        for (column_index, column) in columns.iter().enumerate() {
            let coefficient = svd.right_vectors[direction_index][column_index];
            for row in 0..row_count {
                vector[row] += column[row] * coefficient / singular_value;
            }
        }
        let norm = vector_norm(&vector)?;
        if norm > 0.0 {
            for value in &mut vector {
                *value /= norm;
            }
            basis.push(vector);
        }
    }
    Ok(basis)
}

fn project_onto_complement(
    vector: &[f64],
    basis: &[Vec<f64>],
) -> Result<Vec<f64>, IdentifiabilityError> {
    let mut residual = vector.to_vec();
    // Two modified Gram-Schmidt passes reduce loss of orthogonality.
    for _ in 0..2 {
        for basis_vector in basis {
            let coefficient = dot(&residual, basis_vector)?;
            for (value, basis_value) in residual.iter_mut().zip(basis_vector.iter()) {
                *value -= coefficient * basis_value;
                if !value.is_finite() {
                    return Err(IdentifiabilityError::NumericalOverflow);
                }
            }
        }
    }
    Ok(residual)
}

fn count_above(values: &[f64], threshold: f64) -> usize {
    values.iter().filter(|value| **value > threshold).count()
}

fn condition_number(
    singular_values: &[f64],
    numerical_rank: usize,
    parameter_count: usize,
) -> Option<f64> {
    if numerical_rank != parameter_count || parameter_count == 0 {
        return None;
    }
    let maximum = singular_values.first().copied()?;
    let minimum = singular_values.get(parameter_count - 1).copied()?;
    if minimum <= 0.0 {
        None
    } else {
        Some(maximum / minimum)
    }
}

fn classify_status(
    numerical_rank: usize,
    required_rank: usize,
    condition_number: Option<f64>,
) -> IdentifiabilityStatus {
    if numerical_rank == 0 {
        IdentifiabilityStatus::NoInformativeSensitivity
    } else if numerical_rank < required_rank {
        IdentifiabilityStatus::RankDeficient
    } else if condition_number.is_some_and(|value| !value.is_finite()) {
        IdentifiabilityStatus::IllConditioned
    } else {
        IdentifiabilityStatus::Identifiable
    }
}

fn dot(left: &[f64], right: &[f64]) -> Result<f64, IdentifiabilityError> {
    let mut sum = 0.0;
    for (a, b) in left.iter().zip(right.iter()) {
        sum += a * b;
        if !sum.is_finite() {
            return Err(IdentifiabilityError::NumericalOverflow);
        }
    }
    Ok(sum)
}

fn vector_norm(vector: &[f64]) -> Result<f64, IdentifiabilityError> {
    Ok(dot(vector, vector)?.sqrt())
}

fn normalized_correlation(
    left: &[f64],
    right: &[f64],
) -> Result<Option<f64>, IdentifiabilityError> {
    let left_norm = vector_norm(left)?;
    let right_norm = vector_norm(right)?;
    if left_norm == 0.0 || right_norm == 0.0 {
        return Ok(None);
    }
    let value = left
        .iter()
        .zip(right.iter())
        .map(|(left_value, right_value)| (left_value / left_norm) * (right_value / right_norm))
        .sum::<f64>();
    if !value.is_finite() {
        return Err(IdentifiabilityError::NumericalOverflow);
    }
    Ok(Some(value.clamp(-1.0, 1.0)))
}

fn set_status_threshold(
    status: IdentifiabilityStatus,
    condition_number: Option<f64>,
    maximum_condition_number: f64,
) -> IdentifiabilityStatus {
    if status == IdentifiabilityStatus::Identifiable
        && condition_number.is_some_and(|value| value > maximum_condition_number)
    {
        IdentifiabilityStatus::IllConditioned
    } else {
        status
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateMeasurementAction {
    pub id: String,
    /// Positive declared acquisition / diagnostic cost in one consistent unit.
    pub cost: f64,
    /// New residual-sensitivity rows after conditional whitening with the
    /// covariance of all previously observed residuals. These are innovations,
    /// not raw rows when candidate observations correlate with existing ones.
    pub whitened_conditional_innovation_rows: Vec<Vec<f64>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidateActionScore {
    pub action_id: String,
    pub cost: f64,
    pub added_observation_count: usize,
    pub physical_rank_gain: usize,
    pub smallest_profiled_physical_sensitivity_before: f64,
    pub smallest_profiled_physical_sensitivity_after: f64,
    pub smallest_profiled_physical_sensitivity_gain: f64,
    pub sensitivity_gain_per_cost: f64,
    pub physical_condition_number_before: Option<f64>,
    pub physical_condition_number_after: Option<f64>,
    pub physical_status_after: IdentifiabilityStatus,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MeasurementActionRanking {
    pub baseline_physical_status: IdentifiabilityStatus,
    pub baseline_profiled_physical_singular_values: Vec<f64>,
    /// Sorted by physical rank gain, then weakest-direction sensitivity gain
    /// per unit cost, then remaining sensitivity; ties are ordered by ID.
    pub candidates: Vec<CandidateActionScore>,
    pub recommended_action_id: Option<String>,
}

/// Rank candidate actions by how much they improve the weakest physical
/// sensitivity after nuisance directions have been profiled out.
///
/// Candidate rows must be conditionally whitened innovations: if candidate
/// observations correlate with existing observations, the caller must first
/// apply the appropriate conditional covariance whitening. Simply treating
/// correlated image pixels as independent rows invalidates the information
/// increment.
pub fn rank_candidate_measurement_actions(
    baseline: &IdentifiabilityInput,
    actions: &[CandidateMeasurementAction],
) -> Result<MeasurementActionRanking, IdentifiabilityError> {
    validate_input(baseline)?;
    if actions.is_empty() {
        return Err(IdentifiabilityError::EmptyCandidateActions);
    }
    if actions.len() > 256 {
        return Err(IdentifiabilityError::TooManyCandidateActions {
            count: actions.len(),
        });
    }
    let baseline_report = analyze_local_identifiability(baseline)?;
    let baseline_smallest = baseline_report
        .nuisance_profiled_physical_singular_values
        .last()
        .copied()
        .unwrap_or(0.0);
    let mut seen_ids = std::collections::BTreeSet::new();
    let mut scores = Vec::with_capacity(actions.len());

    for action in actions {
        if action.id.trim().is_empty() {
            return Err(IdentifiabilityError::EmptyCandidateActionId);
        }
        if !seen_ids.insert(action.id.as_str()) {
            return Err(IdentifiabilityError::DuplicateCandidateActionId {
                id: action.id.clone(),
            });
        }
        if !action.cost.is_finite() || action.cost <= 0.0 {
            return Err(IdentifiabilityError::InvalidCandidateActionCost {
                id: action.id.clone(),
            });
        }
        if action.whitened_conditional_innovation_rows.is_empty() {
            return Err(IdentifiabilityError::EmptyCandidateActionRows {
                id: action.id.clone(),
            });
        }
        let augmented_row_count = baseline
            .whitened_sensitivity_matrix
            .len()
            .saturating_add(action.whitened_conditional_innovation_rows.len());
        if augmented_row_count > 65_536 {
            return Err(IdentifiabilityError::TooManyObservations {
                count: augmented_row_count,
            });
        }
        for (row_index, row) in action
            .whitened_conditional_innovation_rows
            .iter()
            .enumerate()
        {
            if row.len() != baseline.parameters.len() {
                return Err(IdentifiabilityError::CandidateRaggedSensitivityRow {
                    action_id: action.id.clone(),
                    row: row_index,
                    expected: baseline.parameters.len(),
                    actual: row.len(),
                });
            }
            for (column_index, value) in row.iter().enumerate() {
                if !value.is_finite() {
                    return Err(IdentifiabilityError::CandidateNonFiniteSensitivity {
                        action_id: action.id.clone(),
                        row: row_index,
                        column: column_index,
                    });
                }
            }
        }

        let mut augmented = baseline.clone();
        augmented
            .whitened_sensitivity_matrix
            .extend(action.whitened_conditional_innovation_rows.iter().cloned());
        let augmented_report = analyze_local_identifiability(&augmented)?;
        let smallest_after = augmented_report
            .nuisance_profiled_physical_singular_values
            .last()
            .copied()
            .unwrap_or(0.0);
        let sensitivity_gain = (smallest_after - baseline_smallest).max(0.0);
        let physical_rank_gain = augmented_report
            .nuisance_profiled_physical_rank
            .saturating_sub(baseline_report.nuisance_profiled_physical_rank);

        let sensitivity_gain_per_cost = sensitivity_gain / action.cost;
        if !sensitivity_gain_per_cost.is_finite() {
            return Err(IdentifiabilityError::NumericalOverflow);
        }
        scores.push(CandidateActionScore {
            action_id: action.id.clone(),
            cost: action.cost,
            added_observation_count: action.whitened_conditional_innovation_rows.len(),
            physical_rank_gain,
            smallest_profiled_physical_sensitivity_before: baseline_smallest,
            smallest_profiled_physical_sensitivity_after: smallest_after,
            smallest_profiled_physical_sensitivity_gain: sensitivity_gain,
            sensitivity_gain_per_cost,
            physical_condition_number_before: baseline_report
                .nuisance_profiled_physical_condition_number,
            physical_condition_number_after: augmented_report
                .nuisance_profiled_physical_condition_number,
            physical_status_after: augmented_report.physical_status,
        });
    }

    scores.sort_by(|left, right| {
        right
            .physical_rank_gain
            .cmp(&left.physical_rank_gain)
            .then_with(|| {
                right
                    .sensitivity_gain_per_cost
                    .partial_cmp(&left.sensitivity_gain_per_cost)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| {
                right
                    .smallest_profiled_physical_sensitivity_after
                    .partial_cmp(&left.smallest_profiled_physical_sensitivity_after)
                    .unwrap_or(Ordering::Equal)
            })
            .then_with(|| left.action_id.cmp(&right.action_id))
    });

    let recommended_action_id = scores
        .iter()
        .find(|score| {
            score.physical_rank_gain > 0
                || score.smallest_profiled_physical_sensitivity_gain
                    > baseline.config.minimum_profiled_physical_sensitivity
        })
        .map(|score| score.action_id.clone());

    Ok(MeasurementActionRanking {
        baseline_physical_status: baseline_report.physical_status,
        baseline_profiled_physical_singular_values: baseline_report
            .nuisance_profiled_physical_singular_values,
        candidates: scores,
        recommended_action_id,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parameter(id: &str, role: ParameterRole) -> ParameterSpec {
        ParameterSpec {
            id: id.into(),
            name: id.into(),
            role,
            characteristic_scale: 1.0,
            unit: "normalized".into(),
        }
    }

    fn config() -> IdentifiabilityConfig {
        IdentifiabilityConfig {
            rank_relative_tolerance: 1.0e-10,
            maximum_condition_number: 1.0e6,
            minimum_profiled_physical_sensitivity: 1.0e-8,
            physical_nuisance_correlation_threshold: 0.95,
            jacobi_tolerance: 1.0e-12,
            maximum_jacobi_sweeps: 100,
        }
    }

    fn input(parameters: Vec<ParameterSpec>, matrix: Vec<Vec<f64>>) -> IdentifiabilityInput {
        IdentifiabilityInput {
            parameters,
            whitened_sensitivity_matrix: matrix,
            config: config(),
        }
    }

    #[test]
    fn orthogonal_physical_and_nuisance_directions_are_identifiable() {
        let report = analyze_local_identifiability(&input(
            vec![
                parameter("flow_velocity", ParameterRole::Physical),
                parameter("camera_gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 0.0], vec![0.0, 1.0], vec![1.0, 1.0]],
        ))
        .unwrap();
        assert_eq!(report.joint_numerical_rank, 2);
        assert_eq!(report.joint_status, IdentifiabilityStatus::Identifiable);
        assert_eq!(report.nuisance_profiled_physical_rank, 1);
        assert_eq!(report.physical_status, IdentifiabilityStatus::Identifiable);
        assert!(report.physical_nuisance_confounding.is_empty());
    }

    #[test]
    fn collinear_physical_and_gain_directions_are_not_physically_identifiable() {
        let report = analyze_local_identifiability(&input(
            vec![
                parameter("flow_brightness", ParameterRole::Physical),
                parameter("camera_gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 1.0], vec![2.0, 2.0], vec![3.0, 3.0]],
        ))
        .unwrap();
        assert_eq!(report.joint_numerical_rank, 1);
        assert_eq!(report.joint_status, IdentifiabilityStatus::RankDeficient);
        assert_eq!(report.nuisance_profiled_physical_rank, 0);
        assert_eq!(report.physical_status, IdentifiabilityStatus::RankDeficient);
        assert_eq!(report.physical_nuisance_confounding.len(), 1);
        assert!((report.physical_nuisance_confounding[0].signed_correlation - 1.0).abs() < 1.0e-10);
    }

    #[test]
    fn nuisance_only_rank_deficiency_does_not_invalidate_independent_flow_parameter() {
        let report = analyze_local_identifiability(&input(
            vec![
                parameter("flow_velocity", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
                parameter("offset", ParameterRole::Nuisance),
            ],
            vec![
                vec![1.0, 0.0, 0.0],
                vec![0.0, 1.0, 2.0],
                vec![0.0, 0.0, 0.0],
            ],
        ))
        .unwrap();
        assert_eq!(report.joint_numerical_rank, 2);
        assert_eq!(report.joint_status, IdentifiabilityStatus::RankDeficient);
        assert_eq!(report.nuisance_profiled_physical_rank, 1);
        assert_eq!(report.physical_status, IdentifiabilityStatus::Identifiable);
    }

    #[test]
    fn near_collinear_physical_and_nuisance_directions_are_ill_conditioned() {
        let mut problem = input(
            vec![
                parameter("flow_velocity", ParameterRole::Physical),
                parameter("camera_timing", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 1.0], vec![0.0, 1.0e-7], vec![0.0, 0.0]],
        );
        problem.config.maximum_condition_number = 1.0e5;
        let report = analyze_local_identifiability(&problem).unwrap();
        assert_eq!(
            report.physical_status,
            IdentifiabilityStatus::IllConditioned
        );
        assert!(report.nuisance_profiled_physical_condition_number.unwrap() > 1.0e5);
    }

    #[test]
    fn characteristic_parameter_scaling_is_applied_before_decomposition() {
        let mut first = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![2.0, 0.0], vec![0.0, 3.0]],
        );
        first.parameters[0].characteristic_scale = 0.5;
        first.parameters[1].characteristic_scale = 2.0;

        let second = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 0.0], vec![0.0, 6.0]],
        );
        let first_report = analyze_local_identifiability(&first).unwrap();
        let second_report = analyze_local_identifiability(&second).unwrap();
        for (left, right) in first_report
            .joint_singular_values
            .iter()
            .zip(second_report.joint_singular_values.iter())
        {
            assert!((left - right).abs() < 1.0e-10);
        }
    }

    #[test]
    fn decomposition_is_invariant_to_observation_row_permutation() {
        let parameters = vec![
            parameter("flow", ParameterRole::Physical),
            parameter("gain", ParameterRole::Nuisance),
            parameter("psf", ParameterRole::Nuisance),
        ];
        let rows = vec![
            vec![1.0, 0.0, 2.0],
            vec![0.0, 1.0, 1.0],
            vec![2.0, 1.0, 0.0],
            vec![1.0, 3.0, 1.0],
        ];
        let forward =
            analyze_local_identifiability(&input(parameters.clone(), rows.clone())).unwrap();
        let mut reversed_rows = rows;
        reversed_rows.reverse();
        let reverse = analyze_local_identifiability(&input(parameters, reversed_rows)).unwrap();
        assert_eq!(forward.joint_numerical_rank, reverse.joint_numerical_rank);
        for (left, right) in forward
            .joint_singular_values
            .iter()
            .zip(reverse.joint_singular_values.iter())
        {
            assert!((left - right).abs() < 1.0e-9);
        }
    }

    #[test]
    fn positive_and_negative_physical_nuisance_confounds_keep_their_sign() {
        let positive = analyze_local_identifiability(&input(
            vec![
                parameter("flow_a", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 1.0], vec![2.0, 2.0]],
        ))
        .unwrap();
        assert!(positive.physical_nuisance_confounding[0].signed_correlation > 0.99);

        let negative = analyze_local_identifiability(&input(
            vec![
                parameter("flow_b", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, -1.0], vec![2.0, -2.0]],
        ))
        .unwrap();
        assert!(negative.physical_nuisance_confounding[0].signed_correlation < -0.99);
    }

    #[test]
    fn zero_sensitivity_parameter_is_reported_as_a_weak_direction() {
        let report = analyze_local_identifiability(&input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("unobserved_nuisance", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 0.0], vec![2.0, 0.0]],
        ))
        .unwrap();
        assert_eq!(report.joint_numerical_rank, 1);
        assert_eq!(report.weak_joint_parameter_directions.len(), 1);
        assert_eq!(report.physical_status, IdentifiabilityStatus::Identifiable);
    }

    #[test]
    fn underdetermined_measurement_count_exposes_joint_null_direction() {
        let report = analyze_local_identifiability(&input(
            vec![
                parameter("flow_x", ParameterRole::Physical),
                parameter("flow_y", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 0.0, 0.0], vec![0.0, 1.0, 0.0]],
        ))
        .unwrap();
        assert_eq!(report.joint_numerical_rank, 2);
        assert_eq!(report.joint_status, IdentifiabilityStatus::RankDeficient);
        assert_eq!(report.weak_joint_parameter_directions.len(), 1);
    }

    #[test]
    fn malformed_inputs_and_nonfinite_values_fail_closed() {
        let mut malformed = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 0.0], vec![0.0]],
        );
        assert_eq!(
            analyze_local_identifiability(&malformed),
            Err(IdentifiabilityError::RaggedSensitivityMatrix {
                row: 1,
                expected: 2,
                actual: 1
            })
        );
        malformed.whitened_sensitivity_matrix = vec![vec![f64::NAN, 0.0]];
        assert_eq!(
            analyze_local_identifiability(&malformed),
            Err(IdentifiabilityError::NonFiniteSensitivity { row: 0, column: 0 })
        );
    }

    #[test]
    fn invalid_ids_scales_and_no_physical_parameter_fail_closed() {
        let mut problem = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("flow", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 0.0]],
        );
        assert_eq!(
            analyze_local_identifiability(&problem),
            Err(IdentifiabilityError::DuplicateParameterId { id: "flow".into() })
        );
        problem.parameters[1].id = "gain".into();
        problem.parameters[1].characteristic_scale = 0.0;
        assert_eq!(
            analyze_local_identifiability(&problem),
            Err(IdentifiabilityError::InvalidCharacteristicScale {
                parameter_id: "gain".into()
            })
        );
        problem.parameters[1].characteristic_scale = 1.0;
        problem.parameters[0].role = ParameterRole::Nuisance;
        assert_eq!(
            analyze_local_identifiability(&problem),
            Err(IdentifiabilityError::NoPhysicalParameters)
        );
    }

    #[test]
    fn a_single_parameter_column_is_supported_without_pair_rotations() {
        let report = analyze_local_identifiability(&input(
            vec![parameter("flow_velocity", ParameterRole::Physical)],
            vec![vec![1.0], vec![2.0], vec![3.0]],
        ))
        .unwrap();
        assert_eq!(report.joint_numerical_rank, 1);
        assert_eq!(report.physical_status, IdentifiabilityStatus::Identifiable);
        assert_eq!(report.jacobi_sweeps, 1);
    }
    #[test]
    fn jacobi_nonconvergence_fails_closed() {
        let mut problem = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
                parameter("psf", ParameterRole::Nuisance),
            ],
            vec![
                vec![1.0, 1.0, 0.0],
                vec![0.0, 1.0, 1.0],
                vec![1.0, 0.0, 1.0],
            ],
        );
        problem.config.maximum_jacobi_sweeps = 1;
        assert!(matches!(
            analyze_local_identifiability(&problem),
            Err(IdentifiabilityError::JacobiDidNotConverge { sweeps: 1 })
        ));
    }

    #[test]
    fn a_physical_parameter_with_no_sensitivity_is_no_information_not_confounding() {
        let report = analyze_local_identifiability(&input(
            vec![parameter("flow", ParameterRole::Physical)],
            vec![vec![0.0], vec![0.0], vec![0.0]],
        ))
        .unwrap();
        assert_eq!(report.nuisance_profiled_physical_rank, 0);
        assert_eq!(
            report.physical_status,
            IdentifiabilityStatus::NoInformativeSensitivity
        );
    }
    #[test]
    fn candidate_action_that_breaks_flow_gain_confounding_is_ranked_first() {
        let baseline = input(
            vec![
                parameter("flow_brightness", ParameterRole::Physical),
                parameter("camera_gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 1.0]],
        );
        let actions = vec![
            CandidateMeasurementAction {
                id: "same-ambiguous-view".into(),
                cost: 1.0,
                whitened_conditional_innovation_rows: vec![vec![1.0, 1.0]],
            },
            CandidateMeasurementAction {
                id: "gain-reference-view".into(),
                cost: 1.0,
                whitened_conditional_innovation_rows: vec![vec![0.0, 1.0]],
            },
        ];
        let ranking = rank_candidate_measurement_actions(&baseline, &actions).unwrap();
        assert_eq!(
            ranking.recommended_action_id.as_deref(),
            Some("gain-reference-view")
        );
        assert_eq!(ranking.candidates[0].physical_rank_gain, 1);
        assert_eq!(
            ranking.candidates[0].physical_status_after,
            IdentifiabilityStatus::Identifiable
        );
        assert_eq!(ranking.candidates[1].physical_rank_gain, 0);
    }

    #[test]
    fn cost_breaks_ties_after_rank_gain_and_sensitivity_gain() {
        let baseline = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 1.0]],
        );
        let actions = vec![
            CandidateMeasurementAction {
                id: "expensive".into(),
                cost: 10.0,
                whitened_conditional_innovation_rows: vec![vec![0.0, 1.0]],
            },
            CandidateMeasurementAction {
                id: "cheap".into(),
                cost: 1.0,
                whitened_conditional_innovation_rows: vec![vec![0.0, 1.0]],
            },
        ];
        let ranking = rank_candidate_measurement_actions(&baseline, &actions).unwrap();
        assert_eq!(ranking.candidates[0].action_id, "cheap");
        assert!(
            ranking.candidates[0].sensitivity_gain_per_cost
                > ranking.candidates[1].sensitivity_gain_per_cost
        );
    }

    #[test]
    fn nuisance_only_candidate_does_not_receive_physical_information_credit() {
        let baseline = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 0.0], vec![0.0, 1.0]],
        );
        let actions = vec![CandidateMeasurementAction {
            id: "more-gain-only".into(),
            cost: 1.0,
            whitened_conditional_innovation_rows: vec![vec![0.0, 1.0]],
        }];
        let ranking = rank_candidate_measurement_actions(&baseline, &actions).unwrap();
        assert_eq!(ranking.candidates[0].physical_rank_gain, 0);
        assert_eq!(
            ranking.candidates[0].smallest_profiled_physical_sensitivity_gain,
            0.0
        );
        assert_eq!(ranking.recommended_action_id, None);
    }

    #[test]
    fn candidate_action_inputs_fail_closed() {
        let baseline = input(
            vec![
                parameter("flow", ParameterRole::Physical),
                parameter("gain", ParameterRole::Nuisance),
            ],
            vec![vec![1.0, 1.0]],
        );
        let empty = rank_candidate_measurement_actions(&baseline, &[]);
        assert_eq!(empty, Err(IdentifiabilityError::EmptyCandidateActions));

        let malformed = vec![CandidateMeasurementAction {
            id: "bad-row".into(),
            cost: 1.0,
            whitened_conditional_innovation_rows: vec![vec![1.0]],
        }];
        assert!(matches!(
            rank_candidate_measurement_actions(&baseline, &malformed),
            Err(IdentifiabilityError::CandidateRaggedSensitivityRow { .. })
        ));

        let nonfinite = vec![CandidateMeasurementAction {
            id: "bad-number".into(),
            cost: 1.0,
            whitened_conditional_innovation_rows: vec![vec![f64::NAN, 1.0]],
        }];
        assert!(matches!(
            rank_candidate_measurement_actions(&baseline, &nonfinite),
            Err(IdentifiabilityError::CandidateNonFiniteSensitivity { .. })
        ));
    }
}
