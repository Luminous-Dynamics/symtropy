// SPDX-License-Identifier: AGPL-3.0-or-later
//! Banded shift-invert Lanczos for low modes of a continuous Euler-Bernoulli
//! beam line. This keeps stiffness/mass storage O(N) for fixed bandwidth and
//! provides a path beyond the dense solver's 128-DOF limit.
//!
//! The model is a uniform 1D beam with consistent translational mass only.
//! It is not a general sparse FEA framework and does not reproduce BE25 until
//! the published mass/input contradiction has been resolved.

use crate::Check;
use crate::beam_line::BeamLineBoundaryCase;
use crate::modal_bar::generalized_symmetric_eigenpairs;

const BANDWIDTH: usize = 3;
const MATRIX_STORAGE_COUNT: usize = 3; // K, M and banded Cholesky factor.

/// Canonical 200-element candidate request. The mass value is the derived
/// gamma*A/g candidate, not an accepted resolution of the source discrepancy.
pub const fn canonical_request() -> &'static str {
    concat!(
        "schema=sofistik-be25-sparse-modal-v0.1",
        "|benchmark=PHYS-BE25-SPARSE-MODAL-001",
        "|case=SOFiSTiK-2026-BE25-simple-cantilever-candidate",
        "|elements=200|requested_modes=6|max_lanczos_iterations=120",
        "|length_m=1|youngs_modulus_pa=200000000|second_moment_m4=1e-9",
        "|mass_per_length_kg_per_m=0.25492905324448206",
        "|mass_model=consistent_translational_only|boundary=simple-cantilever",
        "|claim_ceiling=sparse-numerical-candidate-not-BE25-reproduction"
    )
}

#[derive(Clone, Debug, PartialEq)]
pub struct SparseBeamLineMode {
    pub eigenvalue_rad_s_squared: f64,
    pub frequency_hz: f64,
    pub normalized_eigen_residual: f64,
    /// Full mesh shape in [v0, theta0, v1, theta1, ...] order.
    pub shape: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SparseBeamLineSolution {
    pub boundary_case: BeamLineBoundaryCase,
    pub element_count: usize,
    pub total_dof_count: usize,
    pub free_dof_count: usize,
    pub requested_modes: usize,
    pub actual_lanczos_iterations: usize,
    pub matrix_storage_bytes: usize,
    pub lanczos_basis_storage_bytes: usize,
    pub dense_stiffness_mass_storage_bytes: usize,
    pub modes: Vec<SparseBeamLineMode>,
    pub analytical_first_frequency_hz: f64,
    pub first_frequency_relative_error: f64,
    pub maximum_normalized_eigen_residual: f64,
}

#[derive(Clone, Debug)]
struct SymmetricBandedMatrix {
    dimension: usize,
    bandwidth: usize,
    lower: Vec<f64>,
}

impl SymmetricBandedMatrix {
    fn zeros(dimension: usize, bandwidth: usize) -> Self {
        Self {
            dimension,
            bandwidth,
            lower: vec![0.0; dimension * (bandwidth + 1)],
        }
    }

    fn get(&self, row: usize, col: usize) -> f64 {
        let (lower_row, lower_col) = if row >= col { (row, col) } else { (col, row) };
        if lower_row - lower_col > self.bandwidth {
            0.0
        } else {
            self.lower[lower_row * (self.bandwidth + 1) + (lower_row - lower_col)]
        }
    }

    fn set_lower(&mut self, row: usize, col: usize, value: f64) -> Check<()> {
        if row < col || row - col > self.bandwidth {
            return Err("banded lower-triangle write lies outside storage".to_owned());
        }
        if !value.is_finite() {
            return Err("banded matrix write is non-finite".to_owned());
        }
        let index = row * (self.bandwidth + 1) + (row - col);
        self.lower[index] = value;
        Ok(())
    }

    fn add_symmetric_lower(&mut self, row: usize, col: usize, value: f64) -> Check<()> {
        if row < col {
            return Err("symmetric banded assembly requires lower-triangle entries".to_owned());
        }
        if row - col > self.bandwidth {
            return Err("beam element exceeds declared banded storage".to_owned());
        }
        if !value.is_finite() {
            return Err("element assembly value is non-finite".to_owned());
        }
        let index = row * (self.bandwidth + 1) + (row - col);
        self.lower[index] += value;
        Ok(())
    }

    fn mul(&self, vector: &[f64]) -> Check<Vec<f64>> {
        if vector.len() != self.dimension {
            return Err("banded matvec dimension mismatch".to_owned());
        }
        let mut result = vec![0.0; self.dimension];
        for row in 0..self.dimension {
            let first_col = row.saturating_sub(self.bandwidth);
            for col in first_col..=row {
                let value = self.get(row, col);
                result[row] += value * vector[col];
                if row != col {
                    result[col] += value * vector[row];
                }
            }
        }
        if result.iter().all(|value| value.is_finite()) {
            Ok(result)
        } else {
            Err("banded matvec produced a non-finite value".to_owned())
        }
    }

    fn storage_bytes(&self) -> usize {
        self.lower.len() * std::mem::size_of::<f64>()
    }
}

#[derive(Clone, Debug)]
struct BandedCholesky {
    factor: SymmetricBandedMatrix,
}

impl BandedCholesky {
    fn factorize(matrix: &SymmetricBandedMatrix) -> Check<Self> {
        let n = matrix.dimension;
        let bandwidth = matrix.bandwidth;
        let mut factor = SymmetricBandedMatrix::zeros(n, bandwidth);
        for row in 0..n {
            let first_col = row.saturating_sub(bandwidth);
            for col in first_col..=row {
                let mut value = matrix.get(row, col);
                let first_inner = row.saturating_sub(bandwidth).max(col.saturating_sub(bandwidth));
                for inner in first_inner..col {
                    value -= factor.get(row, inner) * factor.get(col, inner);
                }
                if row == col {
                    if !value.is_finite() || value <= 0.0 {
                        return Err(format!(
                            "stiffness Cholesky factorization is not positive definite at pivot {row}"
                        ));
                    }
                    factor.set_lower(row, col, value.sqrt())?;
                } else {
                    let pivot = factor.get(col, col);
                    if !pivot.is_finite() || pivot <= 0.0 {
                        return Err(format!("invalid Cholesky pivot at row {col}"));
                    }
                    factor.set_lower(row, col, value / pivot)?;
                }
            }
        }
        Ok(Self { factor })
    }

    fn solve(&self, rhs: &[f64]) -> Check<Vec<f64>> {
        let n = self.factor.dimension;
        let bandwidth = self.factor.bandwidth;
        if rhs.len() != n {
            return Err("banded Cholesky solve dimension mismatch".to_owned());
        }

        let mut intermediate = vec![0.0; n];
        for row in 0..n {
            let mut value = rhs[row];
            for col in row.saturating_sub(bandwidth)..row {
                value -= self.factor.get(row, col) * intermediate[col];
            }
            let pivot = self.factor.get(row, row);
            if !pivot.is_finite() || pivot <= 0.0 {
                return Err(format!("invalid forward-solve pivot at row {row}"));
            }
            intermediate[row] = value / pivot;
        }

        let mut solution = vec![0.0; n];
        for row in (0..n).rev() {
            let mut value = intermediate[row];
            let end = (row + bandwidth + 1).min(n);
            for next_row in (row + 1)..end {
                value -= self.factor.get(next_row, row) * solution[next_row];
            }
            let pivot = self.factor.get(row, row);
            if !pivot.is_finite() || pivot <= 0.0 {
                return Err(format!("invalid backward-solve pivot at row {row}"));
            }
            solution[row] = value / pivot;
        }
        if solution.iter().all(|value| value.is_finite()) {
            Ok(solution)
        } else {
            Err("banded Cholesky solve produced a non-finite result".to_owned())
        }
    }
}

/// Low-mode solver for a uniform beam line.
///
/// The inverse operator K^-1 M is self-adjoint in the M inner product. Its
/// largest positive eigenvalues correspond to the smallest physical K/M
/// eigenvalues. Banded Cholesky solves and fully reorthogonalized Lanczos avoid
/// constructing dense global matrices.
pub fn solve_sparse_beam_line_modes(
    length_m: f64,
    youngs_modulus_pa: f64,
    second_moment_m4: f64,
    mass_per_length_kg_per_m: f64,
    element_count: usize,
    boundary_case: BeamLineBoundaryCase,
    requested_modes: usize,
    max_lanczos_iterations: usize,
) -> Check<SparseBeamLineSolution> {
    for (name, value) in [
        ("length_m", length_m),
        ("youngs_modulus_pa", youngs_modulus_pa),
        ("second_moment_m4", second_moment_m4),
        ("mass_per_length_kg_per_m", mass_per_length_kg_per_m),
    ] {
        validate_positive(name, value)?;
    }
    if element_count == 0 || element_count > 10_000 {
        return Err("element_count must be between 1 and 10000".to_owned());
    }
    let total_dof_count = 2 * (element_count + 1);
    let fixed_dofs = boundary_fixed_dofs(element_count, boundary_case);
    let fixed_set: std::collections::BTreeSet<usize> = fixed_dofs.iter().copied().collect();
    let free_dof_indices = (0..total_dof_count)
        .filter(|dof| !fixed_set.contains(dof))
        .collect::<Vec<_>>();
    let free_dof_count = free_dof_indices.len();
    if requested_modes == 0 || requested_modes > free_dof_count {
        return Err(format!("requested_modes must be between 1 and {free_dof_count}"));
    }
    if max_lanczos_iterations < requested_modes + 2 {
        return Err("max_lanczos_iterations must exceed requested_modes by at least two".to_owned());
    }
    let iterations_limit = max_lanczos_iterations.min(free_dof_count);

    let mut global_to_free = vec![None; total_dof_count];
    for (free_index, global_index) in free_dof_indices.iter().enumerate() {
        global_to_free[*global_index] = Some(free_index);
    }

    let mut stiffness = SymmetricBandedMatrix::zeros(free_dof_count, BANDWIDTH);
    let mut mass = SymmetricBandedMatrix::zeros(free_dof_count, BANDWIDTH);
    let element_length_m = length_m / element_count as f64;
    let h = element_length_m;
    let h2 = h * h;
    let h3 = h2 * h;
    let stiffness_factor = youngs_modulus_pa * second_moment_m4 / h3;
    let stiffness_shape = [
        [12.0, 6.0 * h, -12.0, 6.0 * h],
        [6.0 * h, 4.0 * h2, -6.0 * h, 2.0 * h2],
        [-12.0, -6.0 * h, 12.0, -6.0 * h],
        [6.0 * h, 2.0 * h2, -6.0 * h, 4.0 * h2],
    ];
    let mass_factor = mass_per_length_kg_per_m * h / 420.0;
    let mass_shape = [
        [156.0, 22.0 * h, 54.0, -13.0 * h],
        [22.0 * h, 4.0 * h2, 13.0 * h, -3.0 * h2],
        [54.0, 13.0 * h, 156.0, -22.0 * h],
        [-13.0 * h, -3.0 * h2, -22.0 * h, 4.0 * h2],
    ];

    for element in 0..element_count {
        let base = 2 * element;
        let global_map = [base, base + 1, base + 2, base + 3];
        for local_row in 0..4 {
            for local_col in 0..=local_row {
                let Some(row) = global_to_free[global_map[local_row]] else {
                    continue;
                };
                let Some(col) = global_to_free[global_map[local_col]] else {
                    continue;
                };
                if row < col {
                    return Err("free-DOF map does not preserve banded ordering".to_owned());
                }
                stiffness.add_symmetric_lower(
                    row,
                    col,
                    stiffness_factor * stiffness_shape[local_row][local_col],
                )?;
                mass.add_symmetric_lower(
                    row,
                    col,
                    mass_factor * mass_shape[local_row][local_col],
                )?;
            }
        }
    }

    let factor = BandedCholesky::factorize(&stiffness)?;
    let mut state = 0x5350_4152_5345_4C31_u64;
    let mut q = (0..free_dof_count)
        .map(|_| 2.0 * next_sweep_unit(&mut state) - 1.0)
        .collect::<Vec<_>>();
    let initial_norm = mass_norm(&mass, &q)?;
    if !initial_norm.is_finite() || initial_norm <= 0.0 {
        return Err("Lanczos initial vector has non-positive mass norm".to_owned());
    }
    for value in &mut q {
        *value /= initial_norm;
    }

    let mut basis: Vec<Vec<f64>> = Vec::with_capacity(iterations_limit);
    let mut diagonal = Vec::with_capacity(iterations_limit);
    let mut off_diagonal = Vec::with_capacity(iterations_limit.saturating_sub(1));
    let mut previous: Option<Vec<f64>> = None;
    let mut previous_beta = 0.0_f64;

    for iteration_index in 0..iterations_limit {
        basis.push(q.clone());
        let mq = mass.mul(&q)?;
        let mut w = factor.solve(&mq)?;
        let mw = mass.mul(&w)?;
        let alpha = dot(&q, &mw);
        diagonal.push(alpha);
        for (value, qvalue) in w.iter_mut().zip(&q) {
            *value -= alpha * qvalue;
        }
        if let Some(previous_vector) = &previous {
            for (value, previous_value) in w.iter_mut().zip(previous_vector) {
                *value -= previous_beta * previous_value;
            }
        }

        for vector in &basis {
            let mw = mass.mul(&w)?;
            let projection = dot(vector, &mw);
            for (value, vector_value) in w.iter_mut().zip(vector) {
                *value -= projection * vector_value;
            }
        }

        let beta = mass_norm(&mass, &w)?;
        let scale = alpha.abs().max(previous_beta.abs()).max(1.0);
        if !beta.is_finite() || beta < 1.0e-13 * scale {
            break;
        }
        if iteration_index + 1 == iterations_limit {
            break;
        }
        off_diagonal.push(beta);
        for value in &mut w {
            *value /= beta;
        }
        previous = Some(q);
        q = w;
        previous_beta = beta;
    }

    if diagonal.len() < requested_modes {
        return Err(format!(
            "Lanczos produced only {} basis vectors for {requested_modes} modes",
            diagonal.len()
        ));
    }
    while off_diagonal.len() + 1 < diagonal.len() {
        // Breakdown at the final step means the projected tridiagonal has
        // one fewer off-diagonal coefficient than diagonal coefficients.
        break;
    }
    let krylov_dimension = diagonal.len();
    if off_diagonal.len() != krylov_dimension.saturating_sub(1) {
        return Err("Lanczos projected tridiagonal has inconsistent dimensions".to_owned());
    }

    let mut tridiagonal = vec![vec![0.0; krylov_dimension]; krylov_dimension];
    for index in 0..krylov_dimension {
        tridiagonal[index][index] = diagonal[index];
        if index < off_diagonal.len() {
            tridiagonal[index][index + 1] = off_diagonal[index];
            tridiagonal[index + 1][index] = off_diagonal[index];
        }
    }
    let identity = (0..krylov_dimension)
        .map(|row| {
            (0..krylov_dimension)
                .map(|col| if row == col { 1.0 } else { 0.0 })
                .collect::<Vec<_>>()
        })
        .collect::<Vec<_>>();
    let ritz_pairs = generalized_symmetric_eigenpairs(&tridiagonal, &identity)?;

    let mut modes = Vec::with_capacity(requested_modes);
    for (theta, ritz_vector) in ritz_pairs.into_iter().rev().take(requested_modes) {
        if !theta.is_finite() || theta <= 0.0 {
            return Err("Lanczos inverse eigenvalue is non-positive or non-finite".to_owned());
        }
        let eigenvalue = 1.0 / theta;
        let mut reduced_shape = vec![0.0; free_dof_count];
        for basis_index in 0..krylov_dimension {
            for dof in 0..free_dof_count {
                reduced_shape[dof] += basis[basis_index][dof] * ritz_vector[basis_index];
            }
        }
        let shape_norm = mass_norm(&mass, &reduced_shape)?;
        if !shape_norm.is_finite() || shape_norm <= 0.0 {
            return Err("Lanczos Ritz vector has non-positive mass norm".to_owned());
        }
        for value in &mut reduced_shape {
            *value /= shape_norm;
        }

        let kx = stiffness.mul(&reduced_shape)?;
        let mx = mass.mul(&reduced_shape)?;
        let residual = kx
            .iter()
            .zip(&mx)
            .map(|(stiffness_value, mass_value)| stiffness_value - eigenvalue * mass_value)
            .collect::<Vec<_>>();
        let residual_norm = euclidean_norm(&residual);
        let denominator = euclidean_norm(&kx) + eigenvalue * euclidean_norm(&mx);
        let normalized_eigen_residual = residual_norm / denominator.max(f64::MIN_POSITIVE);
        let mut shape = vec![0.0; total_dof_count];
        for (reduced_index, global_index) in free_dof_indices.iter().enumerate() {
            shape[*global_index] = reduced_shape[reduced_index];
        }
        modes.push(SparseBeamLineMode {
            eigenvalue_rad_s_squared: eigenvalue,
            frequency_hz: eigenvalue.sqrt() / (2.0 * std::f64::consts::PI),
            normalized_eigen_residual,
            shape,
        });
    }
    modes.sort_by(|left, right| left.frequency_hz.total_cmp(&right.frequency_hz));

    let analytical_first_frequency_hz = analytical_frequency_hz(
        boundary_case.lambda(),
        youngs_modulus_pa,
        second_moment_m4,
        mass_per_length_kg_per_m,
        length_m,
    );
    let first_frequency = modes[0].frequency_hz;
    let first_frequency_relative_error =
        (first_frequency - analytical_first_frequency_hz).abs() / analytical_first_frequency_hz;
    let maximum_normalized_eigen_residual = modes
        .iter()
        .map(|mode| mode.normalized_eigen_residual)
        .fold(0.0_f64, f64::max);

    let matrix_storage_bytes =
        MATRIX_STORAGE_COUNT * stiffness.storage_bytes();
    let lanczos_basis_storage_bytes =
        basis.len() * free_dof_count * std::mem::size_of::<f64>();
    let dense_stiffness_mass_storage_bytes =
        2 * free_dof_count * free_dof_count * std::mem::size_of::<f64>();

    Ok(SparseBeamLineSolution {
        boundary_case,
        element_count,
        total_dof_count,
        free_dof_count,
        requested_modes,
        actual_lanczos_iterations: krylov_dimension,
        matrix_storage_bytes,
        lanczos_basis_storage_bytes,
        dense_stiffness_mass_storage_bytes,
        modes,
        analytical_first_frequency_hz,
        first_frequency_relative_error,
        maximum_normalized_eigen_residual,
    })
}

fn boundary_fixed_dofs(
    element_count: usize,
    boundary_case: BeamLineBoundaryCase,
) -> Vec<usize> {
    let last_node = element_count;
    match boundary_case {
        BeamLineBoundaryCase::SimpleCantilever => vec![0, 1],
        BeamLineBoundaryCase::CantileverWithSimplySupportedEnd => vec![0, 1, 2 * last_node],
        BeamLineBoundaryCase::SimplySupported => vec![0, 2 * last_node],
        BeamLineBoundaryCase::BothEndsFixed => vec![0, 1, 2 * last_node, 2 * last_node + 1],
    }
}

fn analytical_frequency_hz(
    lambda: f64,
    youngs_modulus_pa: f64,
    second_moment_m4: f64,
    mass_per_length_kg_per_m: f64,
    length_m: f64,
) -> f64 {
    lambda.powi(2) / (2.0 * std::f64::consts::PI)
        * (youngs_modulus_pa * second_moment_m4
            / (mass_per_length_kg_per_m * length_m.powi(4)))
            .sqrt()
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and positive"))
    }
}

fn next_sweep_unit(state: &mut u64) -> f64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    ((*state >> 11) as f64) / ((1_u64 << 53) as f64)
}

fn dot(left: &[f64], right: &[f64]) -> f64 {
    left.iter().zip(right).map(|(a, b)| a * b).sum()
}

fn euclidean_norm(vector: &[f64]) -> f64 {
    dot(vector, vector).sqrt()
}

fn mass_norm(mass: &SymmetricBandedMatrix, vector: &[f64]) -> Check<f64> {
    let product = mass.mul(vector)?;
    let value = dot(vector, &product);
    if value.is_finite() && value >= 0.0 {
        Ok(value.sqrt())
    } else {
        Err("mass inner product is negative or non-finite".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::solve_sparse_beam_line_modes;
    use crate::beam_line::{BeamLineBoundaryCase, BeamLineMassModel, solve_beam_line_modes};

    const E_PA: f64 = 200.0e6;
    const I_M4: f64 = 1.0e-9;
    const L_M: f64 = 1.0;
    const MU_KG_PER_M: f64 = 25_000.0 * 1.0e-4 / 9.80665;

    #[test]
    fn canonical_sparse_modal_request_matches_frozen_fixture() {
        assert_eq!(
            super::canonical_request(),
            include_str!("../fixtures/sofistik_be25_sparse_modal_request_v0_1.txt")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn banded_lanczos_matches_dense_solver_for_all_boundary_cases() {
        for boundary in BeamLineBoundaryCase::ALL {
            let dense = solve_beam_line_modes(
                L_M,
                E_PA,
                I_M4,
                MU_KG_PER_M,
                10,
                boundary,
                BeamLineMassModel::ConsistentTranslational,
            )
            .expect("dense 10-element reference");
            let sparse = solve_sparse_beam_line_modes(
                L_M,
                E_PA,
                I_M4,
                MU_KG_PER_M,
                10,
                boundary,
                4,
                32,
            )
            .expect("banded Lanczos 10-element result");
            for mode_index in 0..4 {
                let dense_frequency = dense.modes[mode_index].frequency_hz;
                let sparse_frequency = sparse.modes[mode_index].frequency_hz;
                let relative_error =
                    (dense_frequency - sparse_frequency).abs() / dense_frequency;
                assert!(
                    relative_error < 1.0e-7,
                    "{} mode {} dense/sparse relative error {relative_error:.17e}",
                    boundary.name(),
                    mode_index + 1
                );
            }
            assert!(sparse.maximum_normalized_eigen_residual < 1.0e-8);
        }
    }

    #[test]
    fn two_hundred_element_cantilever_returns_six_low_modes_and_uses_banded_storage() {
        let solution = solve_sparse_beam_line_modes(
            L_M,
            E_PA,
            I_M4,
            MU_KG_PER_M,
            200,
            BeamLineBoundaryCase::SimpleCantilever,
            6,
            120,
        )
        .expect("200-element sparse beam line");
        assert_eq!(solution.total_dof_count, 402);
        assert_eq!(solution.free_dof_count, 400);
        assert_eq!(solution.modes.len(), 6);
        assert!(solution.maximum_normalized_eigen_residual < 1.0e-8);
        assert!(solution.first_frequency_relative_error < 1.0e-2);
        assert!(solution.matrix_storage_bytes * 100 < solution.dense_stiffness_mass_storage_bytes * 5);
        assert!(solution.lanczos_basis_storage_bytes < 400 * 120 * 8 + 8);
        assert!(solution.modes.windows(2).all(|pair| pair[0].frequency_hz < pair[1].frequency_hz));
    }

    #[test]
    fn incorrect_inputs_fail_closed() {
        assert!(solve_sparse_beam_line_modes(
            0.0, E_PA, I_M4, MU_KG_PER_M, 200,
            BeamLineBoundaryCase::SimpleCantilever, 6, 120
        ).is_err());
        assert!(solve_sparse_beam_line_modes(
            L_M, E_PA, I_M4, MU_KG_PER_M, 10,
            BeamLineBoundaryCase::SimpleCantilever, 0, 20
        ).is_err());
        assert!(solve_sparse_beam_line_modes(
            L_M, E_PA, I_M4, MU_KG_PER_M, 10,
            BeamLineBoundaryCase::SimpleCantilever, 4, 4
        ).is_err());
    }
}
