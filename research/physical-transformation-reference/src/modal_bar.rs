// SPDX-License-Identifier: AGPL-3.0-or-later
//! Verification model for longitudinal vibration of a uniform axial bar.
//!
//! This is intentionally a one-dimensional fixed-free bar model. It is not a
//! modal solver for the two-dimensional pin-jointed truss, and it does not
//! model transverse member bending.

#![allow(clippy::needless_range_loop)] // Matrix algorithms intentionally couple row/column indices.

use super::Check;
use std::f64::consts::PI;

const MAX_ELEMENTS: usize = 128;

type Matrix = Vec<Vec<f64>>;
type Eigenpair = (f64, Vec<f64>);

/// The element mass approximation is part of the model contract, not a solver
/// implementation detail. The two formulations can differ materially on
/// coarse meshes.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MassMatrixFormulation {
    /// Two-node consistent mass matrix for a uniform 1D axial element.
    Consistent,
    /// Diagonal row-sum lumped mass matrix for a uniform 1D axial element.
    Lumped,
}

/// A mass-normalized longitudinal eigenmode of the fixed-free axial bar.
/// Shape entries are the free nodal axial degrees of freedom; the fixed-end
/// degree of freedom is omitted and is understood to be zero.
#[derive(Clone, Debug, PartialEq)]
pub struct AxialBarMode {
    /// Generalized eigenvalue in rad²/s².
    pub eigenvalue_rad_s_squared: f64,
    /// Natural frequency in hertz.
    pub frequency_hz: f64,
    /// Mass-normalized mode shape over free axial degrees of freedom.
    pub shape_free_dofs: Vec<f64>,
    /// Infinity-norm residual scaled by ||K||∞ + |lambda| ||M||∞.
    pub normalized_eigen_residual: f64,
    /// Absolute deviation of phiᵀ M phi from one.
    pub mass_norm_residual: f64,
}

/// Analytical longitudinal natural frequencies for a uniform bar fixed at one
/// end and free at the other, in hertz.
///
/// The exact spectrum is
/// `f_n = (2n - 1) / (4 L) * sqrt(E / rho)`.
pub fn analytical_fixed_free_axial_bar_frequencies(
    length_m: f64,
    youngs_modulus_pa: f64,
    density_kg_m3: f64,
    requested_modes: usize,
) -> Check<Vec<f64>> {
    validate_positive("length_m", length_m)?;
    validate_positive("youngs_modulus_pa", youngs_modulus_pa)?;
    validate_positive("density_kg_m3", density_kg_m3)?;
    validate_mode_count(requested_modes, MAX_ELEMENTS)?;

    let wave_speed = (youngs_modulus_pa / density_kg_m3).sqrt();
    if !wave_speed.is_finite() || wave_speed <= 0.0 {
        return Err("invalid axial wave speed".to_string());
    }

    let mut frequencies = Vec::with_capacity(requested_modes);
    for mode in 1..=requested_modes {
        let frequency = (2 * mode - 1) as f64 * wave_speed / (4.0 * length_m);
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err("analytical frequency is not finite and positive".to_string());
        }
        frequencies.push(frequency);
    }
    Ok(frequencies)
}

/// Solve the longitudinal modes of a uniform fixed-free axial bar using
/// linear two-node elements and an explicitly selected mass formulation.
/// Frequencies are ordered ascending. Shapes use mass normalization and omit
/// the fixed-end degree of freedom. At most one mode per element is available.
pub fn fixed_free_axial_bar_modes(
    length_m: f64,
    area_m2: f64,
    youngs_modulus_pa: f64,
    density_kg_m3: f64,
    element_count: usize,
    requested_modes: usize,
    mass_formulation: MassMatrixFormulation,
) -> Check<Vec<AxialBarMode>> {
    validate_positive("length_m", length_m)?;
    validate_positive("area_m2", area_m2)?;
    validate_positive("youngs_modulus_pa", youngs_modulus_pa)?;
    validate_positive("density_kg_m3", density_kg_m3)?;

    if !(1..=MAX_ELEMENTS).contains(&element_count) {
        return Err(format!(
            "element_count must be between 1 and {MAX_ELEMENTS}"
        ));
    }
    validate_mode_count(requested_modes, element_count)?;

    let element_length = length_m / element_count as f64;
    validate_positive("element_length", element_length)?;

    let element_stiffness = youngs_modulus_pa * area_m2 / element_length;
    let element_mass = density_kg_m3 * area_m2 * element_length;
    validate_positive("element_stiffness", element_stiffness)?;
    validate_positive("element_mass", element_mass)?;

    // Node 0 is fixed and removed from the reduced system. Nodes 1..=N
    // contribute one free axial degree of freedom each.
    let mut stiffness = vec![vec![0.0; element_count]; element_count];
    let mut mass = vec![vec![0.0; element_count]; element_count];

    for element in 0..element_count {
        let nodes = [element, element + 1];

        for local_row in 0..2 {
            let global_row = nodes[local_row];
            if global_row == 0 {
                continue;
            }
            let row = global_row - 1;

            for local_col in 0..2 {
                let global_col = nodes[local_col];
                if global_col == 0 {
                    continue;
                }
                let col = global_col - 1;

                let stiffness_value = if local_row == local_col {
                    element_stiffness
                } else {
                    -element_stiffness
                };
                stiffness[row][col] += stiffness_value;

                if mass_formulation == MassMatrixFormulation::Consistent {
                    let factor = if local_row == local_col { 2.0 } else { 1.0 };
                    mass[row][col] += element_mass * factor / 6.0;
                }
            }
        }

        if mass_formulation == MassMatrixFormulation::Lumped {
            for node in nodes {
                if node != 0 {
                    mass[node - 1][node - 1] += element_mass / 2.0;
                }
            }
        }
    }

    ensure_finite_matrix("stiffness", &stiffness)?;
    ensure_finite_matrix("mass", &mass)?;

    let eigenpairs = generalized_symmetric_eigenpairs(&stiffness, &mass)?;
    let mut modes = Vec::with_capacity(requested_modes);
    for (eigenvalue, shape) in eigenpairs.into_iter().take(requested_modes) {
        if !eigenvalue.is_finite() || eigenvalue <= 0.0 {
            return Err("modal eigenvalue is not finite and positive".to_string());
        }
        let frequency = eigenvalue.sqrt() / (2.0 * PI);
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err("modal frequency is not finite and positive".to_string());
        }
        let residual = normalized_eigen_residual(&stiffness, &mass, eigenvalue, &shape)?;
        let mass_norm = quadratic_form(&mass, &shape)?;
        modes.push(AxialBarMode {
            eigenvalue_rad_s_squared: eigenvalue,
            frequency_hz: frequency,
            shape_free_dofs: shape,
            normalized_eigen_residual: residual,
            mass_norm_residual: (mass_norm - 1.0).abs(),
        });
    }
    Ok(modes)
}

/// Frequency-only convenience wrapper over the richer modal result.
pub fn fixed_free_axial_bar_frequencies(
    length_m: f64,
    area_m2: f64,
    youngs_modulus_pa: f64,
    density_kg_m3: f64,
    element_count: usize,
    requested_modes: usize,
    mass_formulation: MassMatrixFormulation,
) -> Check<Vec<f64>> {
    Ok(fixed_free_axial_bar_modes(
        length_m,
        area_m2,
        youngs_modulus_pa,
        density_kg_m3,
        element_count,
        requested_modes,
        mass_formulation,
    )?
    .into_iter()
    .map(|mode| mode.frequency_hz)
    .collect())
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if !value.is_finite() || value <= 0.0 {
        return Err(format!("{name} must be finite and positive"));
    }
    Ok(())
}

fn validate_mode_count(requested_modes: usize, available_modes: usize) -> Check<()> {
    if requested_modes == 0 || requested_modes > available_modes {
        return Err(format!(
            "requested_modes must be between 1 and {available_modes}"
        ));
    }
    Ok(())
}

fn ensure_finite_matrix(name: &str, matrix: &Matrix) -> Check<()> {
    if matrix.iter().flatten().all(|value| value.is_finite()) {
        Ok(())
    } else {
        Err(format!("{name} matrix contains a non-finite value"))
    }
}

/// Solve K x = lambda M x by a Cholesky reduction followed by a symmetric
/// Jacobi eigensolve. The reduced M must be positive definite.
/// Shared internal entry point for small, dense generalized symmetric
/// eigenproblems used by other reference model classes. The returned shapes
/// are mass-normalized in the supplied generalized coordinates. Large sparse
/// systems need a separate solver/backend; callers must not silently send
/// unbounded meshes through this dense algorithm.
pub(crate) fn solve_dense_generalized_symmetric(
    stiffness: &[Vec<f64>],
    mass: &[Vec<f64>],
) -> Check<Vec<(f64, Vec<f64>)>> {
    let size = stiffness.len();
    if size == 0 || mass.len() != size {
        return Err("stiffness and mass matrices must be non-empty and equally sized".to_owned());
    }
    if stiffness.iter().any(|row| row.len() != size)
        || mass.iter().any(|row| row.len() != size)
    {
        return Err("generalized eigenproblem matrices must be square".to_owned());
    }
    ensure_finite_matrix("shared stiffness", &stiffness.to_vec())?;
    ensure_finite_matrix("shared mass", &mass.to_vec())?;
    for row in 0..size {
        for col in 0..size {
            let k_scale = stiffness[row][col]
                .abs()
                .max(stiffness[col][row].abs())
                .max(1.0);
            let m_scale = mass[row][col]
                .abs()
                .max(mass[col][row].abs())
                .max(1.0);
            if (stiffness[row][col] - stiffness[col][row]).abs() > 1.0e-10 * k_scale {
                return Err(format!("stiffness is not symmetric at ({row}, {col})"));
            }
            if (mass[row][col] - mass[col][row]).abs() > 1.0e-10 * m_scale {
                return Err(format!("mass is not symmetric at ({row}, {col})"));
            }
        }
    }
    let stiffness_owned = stiffness.to_vec();
    let mass_owned = mass.to_vec();
    generalized_symmetric_eigenpairs(&stiffness_owned, &mass_owned)
}

fn generalized_symmetric_eigenpairs(stiffness: &Matrix, mass: &Matrix) -> Check<Vec<Eigenpair>> {
    let lower = cholesky_lower(mass)?;
    let inverse_lower = inverse_lower_triangular(&lower)?;
    let transformed_left = matrix_multiply(&inverse_lower, stiffness);
    let inverse_lower_transpose = transpose(&inverse_lower);
    let mut transformed = matrix_multiply(&transformed_left, &inverse_lower_transpose);

    // Roundoff in the matrix products can introduce tiny asymmetry.
    for row in 0..transformed.len() {
        for col in (row + 1)..transformed.len() {
            let average = 0.5 * (transformed[row][col] + transformed[col][row]);
            transformed[row][col] = average;
            transformed[col][row] = average;
        }
    }
    ensure_finite_matrix("transformed eigenproblem", &transformed)?;

    let (eigenvalues, eigenvectors) = symmetric_jacobi_eigenpairs(transformed)?;
    let mut eigenpairs = Vec::with_capacity(eigenvalues.len());
    for (column, eigenvalue) in eigenvalues.into_iter().enumerate() {
        let transformed_shape: Vec<f64> = (0..eigenvectors.len())
            .map(|row| eigenvectors[row][column])
            .collect();
        let shape = solve_lower_transpose(&lower, &transformed_shape)?;
        eigenpairs.push((eigenvalue, shape));
    }
    eigenpairs.sort_by(|left, right| left.0.total_cmp(&right.0));
    Ok(eigenpairs)
}

fn solve_lower_transpose(lower: &Matrix, right_hand_side: &[f64]) -> Check<Vec<f64>> {
    let size = lower.len();
    if right_hand_side.len() != size {
        return Err("eigenvector dimension does not match Cholesky factor".to_string());
    }

    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        let mut value = right_hand_side[row];
        for col in (row + 1)..size {
            value -= lower[col][row] * solution[col];
        }
        let diagonal = lower[row][row];
        if !diagonal.is_finite() || diagonal <= 0.0 {
            return Err("invalid transposed Cholesky diagonal".to_string());
        }
        solution[row] = value / diagonal;
    }
    if solution.iter().all(|value| value.is_finite()) {
        Ok(solution)
    } else {
        Err("generalized eigenvector contains a non-finite value".to_string())
    }
}

fn matrix_infinity_norm(matrix: &Matrix) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0_f64, f64::max)
}

fn vector_infinity_norm(vector: &[f64]) -> f64 {
    vector
        .iter()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
}

fn matrix_vector_product(matrix: &Matrix, vector: &[f64]) -> Check<Vec<f64>> {
    if matrix.iter().any(|row| row.len() != vector.len()) || matrix.len() != vector.len() {
        return Err("matrix/vector dimensions do not agree".to_string());
    }
    let product: Vec<f64> = matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum())
        .collect();
    if product.iter().all(|value| value.is_finite()) {
        Ok(product)
    } else {
        Err("matrix/vector product contains a non-finite value".to_string())
    }
}

fn quadratic_form(matrix: &Matrix, vector: &[f64]) -> Check<f64> {
    let product = matrix_vector_product(matrix, vector)?;
    let value: f64 = vector.iter().zip(product).map(|(a, b)| a * b).sum();
    if value.is_finite() && value > 0.0 {
        Ok(value)
    } else {
        Err("modal mass norm is not finite and positive".to_string())
    }
}

fn normalized_eigen_residual(
    stiffness: &Matrix,
    mass: &Matrix,
    eigenvalue: f64,
    shape: &[f64],
) -> Check<f64> {
    let stiffness_shape = matrix_vector_product(stiffness, shape)?;
    let mass_shape = matrix_vector_product(mass, shape)?;
    let numerator = stiffness_shape
        .iter()
        .zip(&mass_shape)
        .map(|(k_phi, m_phi)| (k_phi - eigenvalue * m_phi).abs())
        .fold(0.0_f64, f64::max);
    let denominator = (matrix_infinity_norm(stiffness)
        + eigenvalue.abs() * matrix_infinity_norm(mass))
        * vector_infinity_norm(shape);
    if !numerator.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
        return Err("cannot normalize the generalized eigenpair residual".to_string());
    }
    let result = numerator / denominator;
    if result.is_finite() {
        Ok(result)
    } else {
        Err("normalized eigenpair residual is non-finite".to_string())
    }
}
fn cholesky_lower(matrix: &Matrix) -> Check<Matrix> {
    let size = matrix.len();
    let mut lower = vec![vec![0.0; size]; size];

    for row in 0..size {
        for col in 0..=row {
            let mut value = matrix[row][col];
            for inner in 0..col {
                value -= lower[row][inner] * lower[col][inner];
            }

            if row == col {
                if !value.is_finite() || value <= 0.0 {
                    return Err("mass matrix is not positive definite".to_string());
                }
                lower[row][col] = value.sqrt();
            } else {
                let diagonal = lower[col][col];
                if !diagonal.is_finite() || diagonal <= 0.0 {
                    return Err("invalid Cholesky diagonal".to_string());
                }
                lower[row][col] = value / diagonal;
            }
        }
    }

    ensure_finite_matrix("Cholesky factor", &lower)?;
    Ok(lower)
}

fn inverse_lower_triangular(lower: &Matrix) -> Check<Matrix> {
    let size = lower.len();
    let mut inverse = vec![vec![0.0; size]; size];

    for col in 0..size {
        for row in col..size {
            let mut value = if row == col { 1.0 } else { 0.0 };
            for inner in 0..row {
                value -= lower[row][inner] * inverse[inner][col];
            }
            let diagonal = lower[row][row];
            if !diagonal.is_finite() || diagonal <= 0.0 {
                return Err("invalid lower-triangular diagonal".to_string());
            }
            inverse[row][col] = value / diagonal;
        }
    }

    ensure_finite_matrix("inverse Cholesky factor", &inverse)?;
    Ok(inverse)
}

fn matrix_multiply(left: &Matrix, right: &Matrix) -> Matrix {
    let rows = left.len();
    let inner_size = right.len();
    let cols = right[0].len();
    let mut result = vec![vec![0.0; cols]; rows];

    for row in 0..rows {
        for inner in 0..inner_size {
            let value = left[row][inner];
            for col in 0..cols {
                result[row][col] += value * right[inner][col];
            }
        }
    }
    result
}

fn transpose(matrix: &Matrix) -> Matrix {
    let rows = matrix.len();
    let cols = matrix[0].len();
    let mut result = vec![vec![0.0; rows]; cols];

    for row in 0..rows {
        for col in 0..cols {
            result[col][row] = matrix[row][col];
        }
    }
    result
}

fn symmetric_jacobi_eigenpairs(mut matrix: Matrix) -> Check<(Vec<f64>, Matrix)> {
    let size = matrix.len();
    if size == 0 {
        return Err("eigenproblem matrix is empty".to_string());
    }
    let mut eigenvectors = vec![vec![0.0; size]; size];
    for index in 0..size {
        eigenvectors[index][index] = 1.0;
    }
    if size == 1 {
        return Ok((vec![matrix[0][0]], eigenvectors));
    }

    let scale = matrix
        .iter()
        .flatten()
        .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
    if !scale.is_finite() || scale <= 0.0 {
        return Err("eigenproblem matrix has no finite positive scale".to_string());
    }
    let tolerance = 64.0 * f64::EPSILON * scale;
    let max_sweeps = 100;
    let mut converged = false;

    for _ in 0..max_sweeps {
        for p in 0..size {
            for q in (p + 1)..size {
                let off_diagonal = matrix[p][q];
                if off_diagonal.abs() <= tolerance {
                    continue;
                }

                let tau = (matrix[q][q] - matrix[p][p]) / (2.0 * off_diagonal);
                let tangent = if tau >= 0.0 {
                    1.0 / (tau + tau.hypot(1.0))
                } else {
                    -1.0 / (-tau + tau.hypot(1.0))
                };
                let cosine = 1.0 / (1.0 + tangent * tangent).sqrt();
                let sine = tangent * cosine;
                let pp = matrix[p][p];
                let qq = matrix[q][q];

                matrix[p][p] = pp - tangent * off_diagonal;
                matrix[q][q] = qq + tangent * off_diagonal;
                matrix[p][q] = 0.0;
                matrix[q][p] = 0.0;

                for index in 0..size {
                    if index == p || index == q {
                        continue;
                    }
                    let ip = matrix[index][p];
                    let iq = matrix[index][q];
                    let new_ip = cosine * ip - sine * iq;
                    let new_iq = sine * ip + cosine * iq;
                    matrix[index][p] = new_ip;
                    matrix[p][index] = new_ip;
                    matrix[index][q] = new_iq;
                    matrix[q][index] = new_iq;
                }
                for row in 0..size {
                    let vp = eigenvectors[row][p];
                    let vq = eigenvectors[row][q];
                    eigenvectors[row][p] = cosine * vp - sine * vq;
                    eigenvectors[row][q] = sine * vp + cosine * vq;
                }
            }
        }

        let mut largest_off_diagonal = 0.0_f64;
        for row in 0..size {
            for col in (row + 1)..size {
                largest_off_diagonal = largest_off_diagonal.max(matrix[row][col].abs());
            }
        }
        if largest_off_diagonal <= tolerance {
            converged = true;
            break;
        }
    }

    if !converged {
        return Err("symmetric Jacobi eigensolver did not converge".to_string());
    }

    let eigenvalues: Vec<f64> = (0..size).map(|index| matrix[index][index]).collect();
    if eigenvalues.iter().all(|value| value.is_finite())
        && eigenvectors.iter().flatten().all(|value| value.is_finite())
    {
        Ok((eigenvalues, eigenvectors))
    } else {
        Err("eigensolver returned a non-finite eigenpair".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::{
        MassMatrixFormulation, analytical_fixed_free_axial_bar_frequencies,
        fixed_free_axial_bar_frequencies,
    };

    const LENGTH_M: f64 = 1.0;
    const AREA_M2: f64 = 1.0;
    const YOUNGS_MODULUS_PA: f64 = 210.0e9;
    const DENSITY_KG_M3: f64 = 7800.0;

    fn exact(mode_count: usize) -> Vec<f64> {
        analytical_fixed_free_axial_bar_frequencies(
            LENGTH_M,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            mode_count,
        )
        .expect("analytical frequencies")
    }

    fn solve(elements: usize, modes: usize, formulation: MassMatrixFormulation) -> Vec<f64> {
        fixed_free_axial_bar_frequencies(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            elements,
            modes,
            formulation,
        )
        .expect("modal frequencies")
    }

    fn relative_error(actual: f64, expected: f64) -> f64 {
        (actual - expected).abs() / expected
    }

    #[test]
    fn analytical_spectrum_has_odd_integer_ratios() {
        let frequencies = exact(4);
        for mode in 1..frequencies.len() {
            let expected_ratio = (2 * mode + 1) as f64 / (2 * mode - 1) as f64;
            assert!((frequencies[mode] / frequencies[mode - 1] - expected_ratio).abs() < 1.0e-12);
        }
        assert!((frequencies[0] - 1297.0).abs() < 1.0);
    }

    #[test]
    fn single_element_mass_formulations_reproduce_closed_form_ratios() {
        let analytical_fundamental = exact(1)[0];
        let consistent = solve(1, 1, MassMatrixFormulation::Consistent)[0];
        let lumped = solve(1, 1, MassMatrixFormulation::Lumped)[0];

        let expected_consistent_ratio = 2.0 * 3.0_f64.sqrt() / std::f64::consts::PI;
        let expected_lumped_ratio = 2.0 * 2.0_f64.sqrt() / std::f64::consts::PI;
        assert!((consistent / analytical_fundamental - expected_consistent_ratio).abs() < 1.0e-12);
        assert!((lumped / analytical_fundamental - expected_lumped_ratio).abs() < 1.0e-12);
        assert!(consistent > analytical_fundamental);
        assert!(lumped < analytical_fundamental);
    }

    #[test]
    fn first_mode_error_converges_for_both_mass_formulations() {
        let target = exact(1)[0];

        for formulation in [
            MassMatrixFormulation::Consistent,
            MassMatrixFormulation::Lumped,
        ] {
            let mut errors = Vec::new();
            for elements in [1, 2, 4, 8, 16] {
                errors.push(relative_error(solve(elements, 1, formulation)[0], target));
            }
            assert!(errors.windows(2).all(|pair| pair[1] < pair[0]));
            assert!(errors[1] < 0.05);
        }
    }

    #[test]
    fn second_mode_is_within_five_percent_at_eight_elements() {
        let targets = exact(2);
        for formulation in [
            MassMatrixFormulation::Consistent,
            MassMatrixFormulation::Lumped,
        ] {
            let frequencies = solve(8, 2, formulation);
            assert!(relative_error(frequencies[1], targets[1]) < 0.05);
        }
    }

    #[test]
    fn frequency_is_invariant_to_area_and_scales_with_sqrt_of_modulus_over_density() {
        let baseline = fixed_free_axial_bar_frequencies(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            8,
            2,
            MassMatrixFormulation::Consistent,
        )
        .expect("baseline");
        let different_area = fixed_free_axial_bar_frequencies(
            LENGTH_M,
            0.003,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            8,
            2,
            MassMatrixFormulation::Consistent,
        )
        .expect("different area");
        let scaled = fixed_free_axial_bar_frequencies(
            LENGTH_M,
            AREA_M2,
            4.0 * YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            8,
            2,
            MassMatrixFormulation::Consistent,
        )
        .expect("scaled modulus");

        for mode in 0..2 {
            assert!(relative_error(different_area[mode], baseline[mode]) < 1.0e-11);
            assert!(relative_error(scaled[mode], 2.0 * baseline[mode]) < 1.0e-11);
        }
    }

    #[test]
    fn modes_report_small_eigen_residual_and_mass_normalization_error() {
        let modes = super::fixed_free_axial_bar_modes(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            8,
            3,
            MassMatrixFormulation::Consistent,
        )
        .expect("modal result");
        assert_eq!(modes.len(), 3);
        assert!(
            modes
                .windows(2)
                .all(|pair| pair[0].frequency_hz < pair[1].frequency_hz)
        );
        for mode in modes {
            assert_eq!(mode.shape_free_dofs.len(), 8);
            assert!(mode.normalized_eigen_residual < 1.0e-10);
            assert!(mode.mass_norm_residual < 1.0e-10);
            assert!(mode.shape_free_dofs.iter().all(|value| value.is_finite()));
        }
    }

    #[test]
    fn first_three_mode_shapes_match_independent_analytical_sine_samples() {
        for formulation in [
            MassMatrixFormulation::Consistent,
            MassMatrixFormulation::Lumped,
        ] {
            let modes = super::fixed_free_axial_bar_modes(
                LENGTH_M,
                AREA_M2,
                YOUNGS_MODULUS_PA,
                DENSITY_KG_M3,
                8,
                3,
                formulation,
            )
            .expect("modal shapes");

            for (mode_index, mode) in modes.iter().enumerate() {
                let analytical_shape: Vec<f64> = (1..=8)
                    .map(|node| {
                        let x = LENGTH_M * node as f64 / 8.0;
                        (((2 * mode_index + 1) as f64) * std::f64::consts::PI * x
                            / (2.0 * LENGTH_M))
                            .sin()
                    })
                    .collect();
                let dot = mode
                    .shape_free_dofs
                    .iter()
                    .zip(&analytical_shape)
                    .map(|(actual, expected)| actual * expected)
                    .sum::<f64>();
                let actual_norm = mode
                    .shape_free_dofs
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>();
                let expected_norm = analytical_shape
                    .iter()
                    .map(|value| value * value)
                    .sum::<f64>();
                let modal_assurance = dot * dot / (actual_norm * expected_norm);

                assert!(
                    modal_assurance > 1.0 - 1.0e-12,
                    "mode {} MAC too low for {:?}: {:.17e}",
                    mode_index + 1,
                    formulation,
                    modal_assurance
                );
            }
        }
    }

    #[test]
    fn invalid_domains_and_unavailable_modes_fail_closed() {
        assert!(
            analytical_fixed_free_axial_bar_frequencies(0.0, YOUNGS_MODULUS_PA, DENSITY_KG_M3, 1,)
                .is_err()
        );
        assert!(
            fixed_free_axial_bar_frequencies(
                LENGTH_M,
                AREA_M2,
                YOUNGS_MODULUS_PA,
                DENSITY_KG_M3,
                0,
                1,
                MassMatrixFormulation::Consistent,
            )
            .is_err()
        );
        assert!(
            fixed_free_axial_bar_frequencies(
                LENGTH_M,
                AREA_M2,
                YOUNGS_MODULUS_PA,
                DENSITY_KG_M3,
                2,
                3,
                MassMatrixFormulation::Consistent,
            )
            .is_err()
        );
        assert!(
            fixed_free_axial_bar_frequencies(
                LENGTH_M,
                AREA_M2,
                YOUNGS_MODULUS_PA,
                DENSITY_KG_M3,
                1,
                1,
                MassMatrixFormulation::Lumped,
            )
            .is_ok()
        );
    }
}
