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
        let frequency =
            (2 * mode - 1) as f64 * wave_speed / (4.0 * length_m);
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err("analytical frequency is not finite and positive".to_string());
        }
        frequencies.push(frequency);
    }
    Ok(frequencies)
}

/// Solve the longitudinal modal frequencies of a uniform fixed-free axial bar
/// using linear two-node elements. The returned values are ordered ascending
/// and are in hertz. At most one mode per element is available in this 1D
/// discretization.
pub fn fixed_free_axial_bar_frequencies(
    length_m: f64,
    area_m2: f64,
    youngs_modulus_pa: f64,
    density_kg_m3: f64,
    element_count: usize,
    requested_modes: usize,
    mass_formulation: MassMatrixFormulation,
) -> Check<Vec<f64>> {
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

    let eigenvalues = generalized_symmetric_eigenvalues(&stiffness, &mass)?;
    let mut frequencies = Vec::with_capacity(requested_modes);
    for eigenvalue in eigenvalues.into_iter().take(requested_modes) {
        if !eigenvalue.is_finite() || eigenvalue <= 0.0 {
            return Err("modal eigenvalue is not finite and positive".to_string());
        }
        let frequency = eigenvalue.sqrt() / (2.0 * PI);
        if !frequency.is_finite() || frequency <= 0.0 {
            return Err("modal frequency is not finite and positive".to_string());
        }
        frequencies.push(frequency);
    }
    Ok(frequencies)
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
fn generalized_symmetric_eigenvalues(stiffness: &Matrix, mass: &Matrix) -> Check<Vec<f64>> {
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

    let mut eigenvalues = symmetric_jacobi_eigenvalues(transformed)?;
    eigenvalues.sort_by(f64::total_cmp);
    Ok(eigenvalues)
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

fn symmetric_jacobi_eigenvalues(mut matrix: Matrix) -> Check<Vec<f64>> {
    let size = matrix.len();
    if size == 1 {
        return Ok(vec![matrix[0][0]]);
    }

    let scale = matrix
        .iter()
        .flatten()
        .map(f64::abs)
        .fold(0.0_f64, f64::max);
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
            }
        }

        let mut largest_off_diagonal = 0.0_f64;
        for row in 0..size {
            for col in (row + 1)..size {
                largest_off_diagonal =
                    largest_off_diagonal.max(matrix[row][col].abs());
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
    if eigenvalues.iter().all(|value| value.is_finite()) {
        Ok(eigenvalues)
    } else {
        Err("eigensolver returned a non-finite eigenvalue".to_string())
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

    fn solve(
        elements: usize,
        modes: usize,
        formulation: MassMatrixFormulation,
    ) -> Vec<f64> {
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
            assert!(
                (frequencies[mode] / frequencies[mode - 1] - expected_ratio).abs() < 1.0e-12
            );
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
    fn invalid_domains_and_unavailable_modes_fail_closed() {
        assert!(analytical_fixed_free_axial_bar_frequencies(
            0.0,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            1,
        )
        .is_err());
        assert!(fixed_free_axial_bar_frequencies(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            0,
            1,
            MassMatrixFormulation::Consistent,
        )
        .is_err());
        assert!(fixed_free_axial_bar_frequencies(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            2,
            3,
            MassMatrixFormulation::Consistent,
        )
        .is_err());
        assert!(fixed_free_axial_bar_frequencies(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            DENSITY_KG_M3,
            1,
            1,
            MassMatrixFormulation::Lumped,
        )
        .is_ok());
    }
}
