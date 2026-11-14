// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Algebraic screening diagnostics for intervention-driven linear state-space models.
//!
//! Observability, controllability, and experiment-design rank are reported
//! independently. None of these diagnostics proves causal identification.

#![forbid(unsafe_code)]
#![allow(clippy::needless_range_loop)]

/// Maximum number of entries accepted in a matrix.
pub const MAX_MATRIX_ELEMENTS: usize = 2_000_000;
/// Maximum state / input / output dimension.
pub const MAX_DIMENSION: usize = MAX_MATRIX_ELEMENTS;
/// Maximum finite horizon for observability / controllability matrices.
pub const MAX_HORIZON: usize = 512;

/// Dense row-major matrix with validated dimensions and finite values.
#[derive(Debug, Clone, PartialEq)]
pub struct DenseMatrix {
    rows: usize,
    columns: usize,
    values: Vec<f64>,
}

impl DenseMatrix {
    /// Creates a matrix from row-major values.
    pub fn new(
        rows: usize,
        columns: usize,
        values: Vec<f64>,
    ) -> Result<Self, IdentifiabilityError> {
        if rows == 0 || columns == 0 || rows > MAX_DIMENSION || columns > MAX_DIMENSION {
            return Err(IdentifiabilityError::InvalidMatrixDimensions);
        }
        let expected = rows
            .checked_mul(columns)
            .ok_or(IdentifiabilityError::MatrixSizeOverflow)?;
        if expected > MAX_MATRIX_ELEMENTS {
            return Err(IdentifiabilityError::MatrixTooLarge);
        }
        if values.len() != expected {
            return Err(IdentifiabilityError::IncorrectValueCount {
                expected,
                actual: values.len(),
            });
        }
        if values.iter().any(|value| !value.is_finite()) {
            return Err(IdentifiabilityError::NonFiniteMatrixValue);
        }
        Ok(Self {
            rows,
            columns,
            values,
        })
    }

    /// Creates a matrix from a rectangular row collection.
    pub fn from_rows(rows: Vec<Vec<f64>>) -> Result<Self, IdentifiabilityError> {
        if rows.is_empty() || rows[0].is_empty() {
            return Err(IdentifiabilityError::InvalidMatrixDimensions);
        }
        let columns = rows[0].len();
        if rows.iter().any(|row| row.len() != columns) {
            return Err(IdentifiabilityError::RaggedRows);
        }
        let row_count = rows.len();
        let values = rows.into_iter().flatten().collect();
        Self::new(row_count, columns, values)
    }

    /// Returns the row count.
    #[must_use]
    pub fn rows(&self) -> usize {
        self.rows
    }

    /// Returns the column count.
    #[must_use]
    pub fn columns(&self) -> usize {
        self.columns
    }

    /// Returns a value at zero-based row and column.
    #[must_use]
    pub fn get(&self, row: usize, column: usize) -> f64 {
        self.values[row * self.columns + column]
    }

    /// Returns the row-major values.
    #[must_use]
    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

/// Explicit matrix and design-validation failures.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum IdentifiabilityError {
    InvalidMatrixDimensions,
    MatrixSizeOverflow,
    MatrixTooLarge,
    IncorrectValueCount { expected: usize, actual: usize },
    RaggedRows,
    NonFiniteMatrixValue,
    InvalidRankTolerance,
    InvalidHorizon,
    InvalidStateTransitionDimensions,
    InvalidInputDimensions,
    InvalidObservationDimensions,
    NumericalOverflow,
}

impl std::fmt::Display for IdentifiabilityError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidMatrixDimensions => {
                write!(
                    formatter,
                    "matrix dimensions must be positive and within configured limits"
                )
            }
            Self::MatrixSizeOverflow => write!(formatter, "matrix element count overflowed usize"),
            Self::MatrixTooLarge => {
                write!(formatter, "matrix exceeds the configured element limit")
            }
            Self::IncorrectValueCount { expected, actual } => {
                write!(
                    formatter,
                    "matrix expected {expected} values but received {actual}"
                )
            }
            Self::RaggedRows => write!(formatter, "matrix rows must have equal lengths"),
            Self::NonFiniteMatrixValue => write!(formatter, "matrix contains NaN or infinity"),
            Self::InvalidRankTolerance => {
                write!(
                    formatter,
                    "relative rank tolerance must be finite and between zero and one"
                )
            }
            Self::InvalidHorizon => {
                write!(
                    formatter,
                    "finite horizon must be positive and within configured limits"
                )
            }
            Self::InvalidStateTransitionDimensions => {
                write!(formatter, "state-transition matrix must be square")
            }
            Self::InvalidInputDimensions => {
                write!(
                    formatter,
                    "input matrix row count must equal state dimension"
                )
            }
            Self::InvalidObservationDimensions => {
                write!(
                    formatter,
                    "observation matrix column count must equal state dimension"
                )
            }
            Self::NumericalOverflow => {
                write!(formatter, "matrix arithmetic produced a non-finite value")
            }
        }
    }
}

impl std::error::Error for IdentifiabilityError {}

/// Rank and pivot diagnostics from column-pivoted, reorthogonalized modified Gram-Schmidt.
#[derive(Debug, Clone, PartialEq)]
pub struct RankDiagnostics {
    pub rows: usize,
    pub columns: usize,
    pub rank: usize,
    pub full_column_rank: bool,
    /// Column indices in the order selected by pivoting.
    pub pivot_order: Vec<usize>,
    /// Diagonal magnitudes from normalized, pivoted QR.
    pub accepted_diagonal: Vec<f64>,
    /// Max/min accepted QR diagonal ratio; a screening proxy, not a spectral condition number.
    pub qr_diagonal_condition_proxy: Option<f64>,
}

/// Estimates numerical column rank after normalizing each input column.
///
/// Column normalization makes the rank decision invariant to positive changes
/// in individual feature units. The caller-selected relative tolerance is
/// compared with the largest residual column norm after pivoting.
pub fn rank_diagnostics(
    matrix: &DenseMatrix,
    relative_tolerance: f64,
) -> Result<RankDiagnostics, IdentifiabilityError> {
    validate_tolerance(relative_tolerance)?;
    let rows = matrix.rows;
    let columns = matrix.columns;

    let mut work = vec![vec![0.0; rows]; columns];
    for column in 0..columns {
        let mut norm = 0.0_f64;
        for row in 0..rows {
            norm = norm.hypot(matrix.get(row, column));
        }
        if !norm.is_finite() {
            return Err(IdentifiabilityError::NumericalOverflow);
        }
        if norm > 0.0 {
            for row in 0..rows {
                work[column][row] = matrix.get(row, column) / norm;
            }
        }
    }

    let mut pivot_order: Vec<usize> = (0..columns).collect();
    let mut accepted_diagonal = Vec::with_capacity(rows.min(columns));
    let maximum_pivots = rows.min(columns);

    for pivot in 0..maximum_pivots {
        let mut best_index = pivot;
        let mut best_norm = vector_norm(&work[pivot]);
        for candidate in (pivot + 1)..columns {
            let candidate_norm = vector_norm(&work[candidate]);
            if candidate_norm > best_norm {
                best_norm = candidate_norm;
                best_index = candidate;
            }
        }

        if best_index != pivot {
            work.swap(pivot, best_index);
            pivot_order.swap(pivot, best_index);
        }

        let pivot_norm = vector_norm(&work[pivot]);
        if !pivot_norm.is_finite() {
            return Err(IdentifiabilityError::NumericalOverflow);
        }
        if pivot_norm <= relative_tolerance {
            break;
        }

        accepted_diagonal.push(pivot_norm);
        for value in &mut work[pivot] {
            *value /= pivot_norm;
        }

        for candidate in (pivot + 1)..columns {
            let projection = dot(&work[pivot], &work[candidate]);
            for row in 0..rows {
                work[candidate][row] -= projection * work[pivot][row];
            }

            // A second pass limits loss of orthogonality near the rank threshold.
            let correction = dot(&work[pivot], &work[candidate]);
            for row in 0..rows {
                work[candidate][row] -= correction * work[pivot][row];
            }
        }
    }

    let rank = accepted_diagonal.len();
    let qr_diagonal_condition_proxy = if let (Some(maximum), Some(minimum)) = (
        accepted_diagonal.iter().copied().reduce(f64::max),
        accepted_diagonal.iter().copied().reduce(f64::min),
    ) {
        if minimum > 0.0 {
            Some(maximum / minimum)
        } else {
            None
        }
    } else {
        None
    };

    Ok(RankDiagnostics {
        rows,
        columns,
        rank,
        full_column_rank: rank == columns,
        pivot_order,
        accepted_diagonal,
        qr_diagonal_condition_proxy,
    })
}

/// Finite-horizon observability matrix [C; CA; ...; CA^(horizon-1)].
pub fn observability_matrix(
    transition: &DenseMatrix,
    observation: &DenseMatrix,
    horizon: usize,
) -> Result<DenseMatrix, IdentifiabilityError> {
    validate_system_horizon(horizon)?;
    let state_dim = transition.rows;
    if transition.columns != state_dim {
        return Err(IdentifiabilityError::InvalidStateTransitionDimensions);
    }
    if observation.columns != state_dim {
        return Err(IdentifiabilityError::InvalidObservationDimensions);
    }

    let output_dim = observation.rows;
    let total_rows = output_dim
        .checked_mul(horizon)
        .ok_or(IdentifiabilityError::MatrixSizeOverflow)?;
    let entries = total_rows
        .checked_mul(state_dim)
        .ok_or(IdentifiabilityError::MatrixSizeOverflow)?;
    if entries > MAX_MATRIX_ELEMENTS {
        return Err(IdentifiabilityError::MatrixTooLarge);
    }

    let mut stacked = vec![0.0; entries];
    let mut power = identity(state_dim);
    for step in 0..horizon {
        let block = multiply(observation, &power)?;
        for row in 0..output_dim {
            for column in 0..state_dim {
                stacked[(step * output_dim + row) * state_dim + column] = block.get(row, column);
            }
        }
        power = multiply(transition, &power)?;
    }

    DenseMatrix::new(total_rows, state_dim, stacked)
}

/// Finite-horizon controllability matrix [B AB A²B ... A^(horizon-1)B].
pub fn controllability_matrix(
    transition: &DenseMatrix,
    input: &DenseMatrix,
    horizon: usize,
) -> Result<DenseMatrix, IdentifiabilityError> {
    validate_system_horizon(horizon)?;
    let state_dim = transition.rows;
    if transition.columns != state_dim {
        return Err(IdentifiabilityError::InvalidStateTransitionDimensions);
    }
    if input.rows != state_dim {
        return Err(IdentifiabilityError::InvalidInputDimensions);
    }

    let input_dim = input.columns;
    let total_columns = input_dim
        .checked_mul(horizon)
        .ok_or(IdentifiabilityError::MatrixSizeOverflow)?;
    let entries = state_dim
        .checked_mul(total_columns)
        .ok_or(IdentifiabilityError::MatrixSizeOverflow)?;
    if entries > MAX_MATRIX_ELEMENTS {
        return Err(IdentifiabilityError::MatrixTooLarge);
    }

    let mut horizontal = vec![0.0; entries];
    let mut block = input.clone();
    for step in 0..horizon {
        for row in 0..state_dim {
            for column in 0..input_dim {
                horizontal[row * total_columns + step * input_dim + column] =
                    block.get(row, column);
            }
        }
        block = multiply(transition, &block)?;
    }

    DenseMatrix::new(state_dim, total_columns, horizontal)
}

/// Independent finite-horizon diagnostics for one state-space model and one realized design.
#[derive(Debug, Clone, PartialEq)]
pub struct LinearSystemDiagnostics {
    pub state_dimension: usize,
    pub input_dimension: usize,
    pub observation_dimension: usize,
    pub horizon: usize,
    pub observability: RankDiagnostics,
    pub controllability: RankDiagnostics,
    pub parameter_design: RankDiagnostics,
    pub fully_observable: bool,
    pub fully_reachable: bool,
    pub parameter_design_full_rank: bool,
}

impl LinearSystemDiagnostics {
    /// Returns true only when all three declared algebraic gates pass.
    ///
    /// This is not a causal-identification or instrument-validity certificate.
    #[must_use]
    pub fn all_algebraic_gates_pass(&self) -> bool {
        self.fully_observable && self.fully_reachable && self.parameter_design_full_rank
    }
}

/// Computes observability, reachability and caller-supplied design rank separately.
///
/// The parameter design must be built from the exact history-aligned regression
/// features that the downstream estimator will use. This function does not
/// construct lags or decide which covariates are valid instruments.
pub fn diagnose_linear_system(
    transition: &DenseMatrix,
    input: &DenseMatrix,
    observation: &DenseMatrix,
    parameter_design: &DenseMatrix,
    horizon: usize,
    relative_tolerance: f64,
) -> Result<LinearSystemDiagnostics, IdentifiabilityError> {
    validate_tolerance(relative_tolerance)?;
    let observability = observability_matrix(transition, observation, horizon)?;
    let controllability = controllability_matrix(transition, input, horizon)?;
    let observability_rank = rank_diagnostics(&observability, relative_tolerance)?;
    let controllability_rank = rank_diagnostics(&controllability, relative_tolerance)?;
    let parameter_rank = rank_diagnostics(parameter_design, relative_tolerance)?;

    let state_dimension = transition.rows;
    Ok(LinearSystemDiagnostics {
        state_dimension,
        input_dimension: input.columns,
        observation_dimension: observation.rows,
        horizon,
        fully_observable: observability_rank.rank == state_dimension,
        fully_reachable: controllability_rank.rank == state_dimension,
        parameter_design_full_rank: parameter_rank.full_column_rank,
        observability: observability_rank,
        controllability: controllability_rank,
        parameter_design: parameter_rank,
    })
}

fn validate_tolerance(relative_tolerance: f64) -> Result<(), IdentifiabilityError> {
    if !relative_tolerance.is_finite() || relative_tolerance <= 0.0 || relative_tolerance >= 1.0 {
        return Err(IdentifiabilityError::InvalidRankTolerance);
    }
    Ok(())
}

fn validate_system_horizon(horizon: usize) -> Result<(), IdentifiabilityError> {
    if horizon == 0 || horizon > MAX_HORIZON {
        return Err(IdentifiabilityError::InvalidHorizon);
    }
    Ok(())
}

fn vector_norm(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |norm, value| norm.hypot(*value))
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter()
        .zip(right)
        .map(|(left_value, right_value)| left_value * right_value)
        .sum()
}

fn identity(dimension: usize) -> DenseMatrix {
    let mut values = vec![0.0; dimension * dimension];
    for index in 0..dimension {
        values[index * dimension + index] = 1.0;
    }
    DenseMatrix {
        rows: dimension,
        columns: dimension,
        values,
    }
}

fn multiply(left: &DenseMatrix, right: &DenseMatrix) -> Result<DenseMatrix, IdentifiabilityError> {
    if left.columns != right.rows {
        return Err(IdentifiabilityError::InvalidMatrixDimensions);
    }
    let entries = left
        .rows
        .checked_mul(right.columns)
        .ok_or(IdentifiabilityError::MatrixSizeOverflow)?;
    if entries > MAX_MATRIX_ELEMENTS {
        return Err(IdentifiabilityError::MatrixTooLarge);
    }

    let mut values = vec![0.0; entries];
    for row in 0..left.rows {
        for column in 0..right.columns {
            let mut sum = 0.0;
            for inner in 0..left.columns {
                sum += left.get(row, inner) * right.get(inner, column);
            }
            if !sum.is_finite() {
                return Err(IdentifiabilityError::NumericalOverflow);
            }
            values[row * right.columns + column] = sum;
        }
    }
    DenseMatrix::new(left.rows, right.columns, values)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matrix(rows: &[&[f64]]) -> DenseMatrix {
        DenseMatrix::from_rows(rows.iter().map(|row| row.to_vec()).collect())
            .expect("valid matrix fixture")
    }

    #[test]
    fn a_system_can_be_observable_but_not_fully_reachable() {
        let transition = matrix(&[&[1.0, 1.0], &[0.0, 1.0]]);
        let input = matrix(&[&[1.0], &[0.0]]);
        let observation = matrix(&[&[1.0, 0.0]]);
        let design = matrix(&[&[1.0, 0.0], &[1.0, 1.0], &[1.0, 2.0]]);

        let report = diagnose_linear_system(&transition, &input, &observation, &design, 2, 1.0e-10)
            .expect("valid system");

        assert_eq!(report.observability.rank, 2);
        assert_eq!(report.controllability.rank, 1);
        assert!(report.fully_observable);
        assert!(!report.fully_reachable);
        assert!(report.parameter_design_full_rank);
        assert!(!report.all_algebraic_gates_pass());
    }

    #[test]
    fn a_system_can_be_fully_reachable_but_unobservable() {
        let transition = matrix(&[&[1.0, 1.0], &[0.0, 1.0]]);
        let input = matrix(&[&[0.0], &[1.0]]);
        let observation = matrix(&[&[0.0, 1.0]]);
        let design = matrix(&[&[1.0, 0.0], &[0.0, 1.0], &[1.0, 1.0]]);

        let report = diagnose_linear_system(&transition, &input, &observation, &design, 2, 1.0e-10)
            .expect("valid system");

        assert_eq!(report.observability.rank, 1);
        assert_eq!(report.controllability.rank, 2);
        assert!(!report.fully_observable);
        assert!(report.fully_reachable);
        assert!(report.parameter_design_full_rank);
    }

    #[test]
    fn full_observability_and_reachability_are_separate_from_design_rank() {
        let transition = matrix(&[&[1.0, 1.0], &[0.0, 1.0]]);
        let input = matrix(&[&[0.0], &[1.0]]);
        let observation = matrix(&[&[1.0, 0.0]]);
        let deficient_design = matrix(&[&[1.0, 2.0], &[2.0, 4.0], &[3.0, 6.0]]);

        let report = diagnose_linear_system(
            &transition,
            &input,
            &observation,
            &deficient_design,
            2,
            1.0e-10,
        )
        .expect("valid system");

        assert!(report.fully_observable);
        assert!(report.fully_reachable);
        assert_eq!(report.parameter_design.rank, 1);
        assert!(!report.parameter_design_full_rank);
        assert!(!report.all_algebraic_gates_pass());
    }

    #[test]
    fn rank_is_invariant_to_column_units() {
        let base = matrix(&[&[1.0, 0.0], &[0.0, 1.0], &[1.0, 1.0]]);
        let rescaled = matrix(&[&[1.0e9, 0.0], &[0.0, 1.0e-9], &[1.0e9, 1.0e-9]]);
        let base_rank = rank_diagnostics(&base, 1.0e-10).expect("base rank");
        let scaled_rank = rank_diagnostics(&rescaled, 1.0e-10).expect("scaled rank");

        assert_eq!(base_rank.rank, 2);
        assert_eq!(scaled_rank.rank, 2);
    }

    #[test]
    fn pivot_order_is_reported_and_rank_is_permutation_invariant() {
        let first = matrix(&[&[0.0, 1.0, 1.0], &[0.0, 0.0, 1.0], &[1.0, 0.0, 1.0]]);
        let permuted = matrix(&[&[1.0, 0.0, 1.0], &[0.0, 1.0, 1.0], &[0.0, 0.0, 1.0]]);
        let a = rank_diagnostics(&first, 1.0e-10).expect("first rank");
        let b = rank_diagnostics(&permuted, 1.0e-10).expect("permuted rank");

        assert_eq!(a.rank, 3);
        assert_eq!(b.rank, 3);
        assert_eq!(a.pivot_order.len(), 3);
        assert_eq!(b.pivot_order.len(), 3);
    }

    #[test]
    fn rank_deficiency_is_reported_not_regularized_away() {
        let deficient = matrix(&[&[1.0, 2.0, 3.0], &[2.0, 4.0, 6.0], &[3.0, 6.0, 9.0]]);
        let report = rank_diagnostics(&deficient, 1.0e-10).expect("rank report");
        assert_eq!(report.rank, 1);
        assert!(!report.full_column_rank);
        assert_eq!(report.accepted_diagonal.len(), 1);
    }

    #[test]
    fn dimensions_and_non_finite_values_fail_closed() {
        assert_eq!(
            DenseMatrix::new(2, 2, vec![1.0, 2.0, 3.0]),
            Err(IdentifiabilityError::IncorrectValueCount {
                expected: 4,
                actual: 3
            })
        );
        assert_eq!(
            DenseMatrix::new(1, 1, vec![f64::NAN]),
            Err(IdentifiabilityError::NonFiniteMatrixValue)
        );
        assert_eq!(
            DenseMatrix::from_rows(vec![vec![1.0], vec![1.0, 2.0]]),
            Err(IdentifiabilityError::RaggedRows)
        );
    }

    #[test]
    fn invalid_tolerance_and_horizon_fail_closed() {
        let identity = matrix(&[&[1.0, 0.0], &[0.0, 1.0]]);
        let input = matrix(&[&[1.0], &[0.0]]);
        let observation = matrix(&[&[1.0, 0.0]]);

        assert_eq!(
            rank_diagnostics(&identity, 0.0),
            Err(IdentifiabilityError::InvalidRankTolerance)
        );
        assert_eq!(
            observability_matrix(&identity, &observation, 0),
            Err(IdentifiabilityError::InvalidHorizon)
        );
        assert_eq!(
            controllability_matrix(&identity, &input, MAX_HORIZON + 1),
            Err(IdentifiabilityError::InvalidHorizon)
        );
    }

    #[test]
    fn invalid_state_space_dimensions_are_not_coerced() {
        let non_square = matrix(&[&[1.0, 0.0]]);
        let input = matrix(&[&[1.0]]);
        let observation = matrix(&[&[1.0]]);
        assert_eq!(
            observability_matrix(&non_square, &observation, 2),
            Err(IdentifiabilityError::InvalidStateTransitionDimensions)
        );
        assert_eq!(
            controllability_matrix(&non_square, &input, 2),
            Err(IdentifiabilityError::InvalidStateTransitionDimensions)
        );
    }
}
