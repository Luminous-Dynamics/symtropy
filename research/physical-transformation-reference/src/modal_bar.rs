// SPDX-License-Identifier: AGPL-3.0-or-later
//! Small, dependency-free axial-bar modal benchmark.
//!
//! This module is a deliberately bounded 1D finite-element reference for the
//! fixed-free, uniform axial bar used in NAFEMS Benchmark Challenge 5. It
//! exists to expose the effect of consistent versus row-sum-lumped mass
//! matrices and to check mesh convergence against a closed-form solution.
//!
//! It is not a 2D truss modal solver, a beam solver, or a validated structural
//! dynamics package.

use crate::Check;

const MAX_ELEMENTS: usize = 32;
const JACOBI_MAX_SWEEPS_FACTOR: usize = 50;
const JACOBI_TOLERANCE_FACTOR: f64 = 1.0e-13;

/// Mass formulation used for the linear axial-bar elements.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MassFormulation {
    /// Consistent two-node axial mass matrix.
    Consistent,
    /// Row-sum-lumped nodal mass matrix.
    RowSumLumped,
}

/// Uniform one-dimensional axial bar, fixed at x=0 and free at x=L.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FixedFreeBar {
    pub length_m: f64,
    pub area_m2: f64,
    pub youngs_modulus_pa: f64,
    pub density_kg_m3: f64,
    pub elements: usize,
}

/// Modal results for the supported one-dimensional bar model.
#[derive(Clone, Debug, PartialEq)]
pub struct ModalFrequencies {
    pub formulation: MassFormulation,
    /// Eigenfrequencies in ascending order, measured in hertz.
    pub frequencies_hz: Vec<f64>,
    /// Closed-form continuum frequencies for matching mode numbers, in hertz.
    pub exact_frequencies_hz: Vec<f64>,
    /// Numerical frequency divided by the closed-form frequency, by mode.
    pub normalized_frequencies: Vec<f64>,
}

/// Analyze a uniform axial bar with a fixed left end and free right end.
///
/// The stiffness and mass matrices are assembled independently, constrained
/// degrees of freedom are eliminated, and the generalized eigenproblem
/// K phi = omega^2 M phi is reduced using a Cholesky factorization of M.
/// A symmetric Jacobi eigensolver is then used for the small dense problem.
///
/// The returned values qualify only this idealized 1D axial model. For each
/// mode n>=1, the continuum reference is f_n=(2n-1)*sqrt(E/rho)/(4*L).
pub fn analyze_fixed_free_bar(
    bar: FixedFreeBar,
    formulation: MassFormulation,
) -> Check<ModalFrequencies> {
    validate_bar(bar)?;
    let n = bar.elements;
    let element_length = bar.length_m / n as f64;
    let element_stiffness = bar.youngs_modulus_pa * bar.area_m2 / element_length;
    let element_mass = bar.density_kg_m3 * bar.area_m2 * element_length;
    if !element_stiffness.is_finite()
        || element_stiffness <= 0.0
        || !element_mass.is_finite()
        || element_mass <= 0.0
    {
        return Err("element stiffness or mass is outside the supported numeric range".to_owned());
    }

    // The first node is fixed. The reduced matrices contain the n free nodes.
    let mut stiffness = vec![vec![0.0; n]; n];
    let mut mass = vec![vec![0.0; n]; n];
    for element in 0..n {
        let left = element;
        let right = element + 1;
        let k = element_stiffness;
        stiffness[left][left] += k;
        if right < n {
            stiffness[right][right] += k;
            stiffness[left][right] -= k;
            stiffness[right][left] -= k;
        }

        match formulation {
            MassFormulation::Consistent => {
                let m = element_mass / 6.0;
                mass[left][left] += 2.0 * m;
                if right < n {
                    mass[right][right] += 2.0 * m;
                    mass[left][right] += m;
                    mass[right][left] += m;
                } else {
                    // The fixed endpoint's row/column is eliminated. Its
                    // consistent-mass coupling to the last free node vanishes.
                    mass[left][left] += 0.0;
                }
            }
            MassFormulation::RowSumLumped => {
                let half_mass = element_mass / 2.0;
                mass[left][left] += half_mass;
                if right < n {
                    mass[right][right] += half_mass;
                }
            }
        }
    }

    // Consistent element assembly contributes 2m/6 to each node before
    // constraints. The free endpoint's diagonal term was already added above.
    // Assemble mass once more with the exact local matrix form to avoid any
    // ambiguity from the constrained endpoint bookkeeping.
    mass = assemble_mass(n, element_mass, formulation);

    let lower = cholesky_lower(&mass)?;
    let inverse_lower = invert_lower(&lower)?;
    let transformed_left = multiply(&inverse_lower, &stiffness);
    let mut symmetric = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..n {
            // A = L^-1 K L^-T, where M = L L^T.
            symmetric[i][j] = (0..n)
                .map(|k| transformed_left[i][k] * inverse_lower[j][k])
                .sum();
        }
    }
    for i in 0..n {
        for j in 0..i {
            let average = 0.5 * (symmetric[i][j] + symmetric[j][i]);
            symmetric[i][j] = average;
            symmetric[j][i] = average;
        }
    }

    let eigenvalues = symmetric_eigenvalues(symmetric)?;
    let numerical_frequencies_hz: Vec<f64> = eigenvalues
        .into_iter()
        .map(|value| value.sqrt() / (2.0 * std::f64::consts::PI))
        .collect();
    if numerical_frequencies_hz
        .iter()
        .any(|frequency| !frequency.is_finite() || *frequency <= 0.0)
    {
        return Err("modal solve produced a non-positive or non-finite frequency".to_owned());
    }

    let wave_speed = (bar.youngs_modulus_pa / bar.density_kg_m3).sqrt();
    let exact_frequencies_hz: Vec<f64> = (1..=n)
        .map(|mode| (2 * mode - 1) as f64 * wave_speed / (4.0 * bar.length_m))
        .collect();
    let normalized_frequencies = numerical_frequencies_hz
        .iter()
        .zip(exact_frequencies_hz.iter())
        .map(|(numerical, exact)| numerical / exact)
        .collect();

    Ok(ModalFrequencies {
        formulation,
        frequencies_hz: numerical_frequencies_hz,
        exact_frequencies_hz,
        normalized_frequencies,
    })
}

fn validate_bar(bar: FixedFreeBar) -> Check<()> {
    if !bar.length_m.is_finite() || bar.length_m <= 0.0 {
        return Err("length must be finite and positive".to_owned());
    }
    if !bar.area_m2.is_finite() || bar.area_m2 <= 0.0 {
        return Err("area must be finite and positive".to_owned());
    }
    if !bar.youngs_modulus_pa.is_finite() || bar.youngs_modulus_pa <= 0.0 {
        return Err("Young's modulus must be finite and positive".to_owned());
    }
    if !bar.density_kg_m3.is_finite() || bar.density_kg_m3 <= 0.0 {
        return Err("density must be finite and positive".to_owned());
    }
    if !(1..=MAX_ELEMENTS).contains(&bar.elements) {
        return Err(format!("element count must be between 1 and {MAX_ELEMENTS}"));
    }
    Ok(())
}

fn assemble_mass(
    elements: usize,
    element_mass: f64,
    formulation: MassFormulation,
) -> Vec<Vec<f64>> {
    let mut mass = vec![vec![0.0; elements]; elements];
    for element in 0..elements {
        let left = element;
        let right = element + 1;
        match formulation {
            MassFormulation::Consistent => {
                mass[left][left] += element_mass / 3.0;
                if right < elements {
                    mass[right][right] += element_mass / 3.0;
                    mass[left][right] += element_mass / 6.0;
                    mass[right][left] += element_mass / 6.0;
                }
            }
            MassFormulation::RowSumLumped => {
                mass[left][left] += element_mass / 2.0;
                if right < elements {
                    mass[right][right] += element_mass / 2.0;
                }
            }
        }
    }
    mass
}

fn cholesky_lower(matrix: &[Vec<f64>]) -> Check<Vec<Vec<f64>>> {
    let n = matrix.len();
    let mut lower = vec![vec![0.0; n]; n];
    for i in 0..n {
        for j in 0..=i {
            let correction: f64 = (0..j).map(|k| lower[i][k] * lower[j][k]).sum();
            let value = matrix[i][j] - correction;
            if i == j {
                if !value.is_finite() || value <= 0.0 {
                    return Err("mass matrix is not numerically positive definite".to_owned());
                }
                lower[i][j] = value.sqrt();
            } else {
                lower[i][j] = value / lower[j][j];
            }
        }
    }
    Ok(lower)
}

fn invert_lower(lower: &[Vec<f64>]) -> Check<Vec<Vec<f64>>> {
    let n = lower.len();
    let mut inverse = vec![vec![0.0; n]; n];
    for column in 0..n {
        for row in 0..n {
            let correction: f64 = (0..row)
                .map(|k| lower[row][k] * inverse[k][column])
                .sum();
            let rhs = if row == column { 1.0 } else { 0.0 };
            inverse[row][column] = (rhs - correction) / lower[row][row];
            if !inverse[row][column].is_finite() {
                return Err("mass matrix inverse factor is non-finite".to_owned());
            }
        }
    }
    Ok(inverse)
}

fn multiply(left: &[Vec<f64>], right: &[Vec<f64>]) -> Vec<Vec<f64>> {
    let n = left.len();
    let mut result = vec![vec![0.0; n]; n];
    for i in 0..n {
        for k in 0..n {
            let value = left[i][k];
            for j in 0..n {
                result[i][j] += value * right[k][j];
            }
        }
    }
    result
}

fn symmetric_eigenvalues(mut matrix: Vec<Vec<f64>>) -> Check<Vec<f64>> {
    let n = matrix.len();
    let scale = matrix
        .iter()
        .enumerate()
        .map(|(index, row)| row[index].abs())
        .fold(0.0_f64, f64::max);
    if !scale.is_finite() || scale <= 0.0 {
        return Err("transformed stiffness matrix has no positive finite scale".to_owned());
    }
    let tolerance = scale * JACOBI_TOLERANCE_FACTOR;
    let max_iterations = JACOBI_MAX_SWEEPS_FACTOR * n * n;

    for _ in 0..max_iterations {
        let mut largest = 0.0_f64;
        let mut pivot = None;
        for i in 0..n {
            for j in (i + 1)..n {
                let candidate = matrix[i][j].abs();
                if candidate > largest {
                    largest = candidate;
                    pivot = Some((i, j));
                }
            }
        }
        if largest <= tolerance {
            let mut values: Vec<f64> = (0..n).map(|i| matrix[i][i]).collect();
            values.sort_by(f64::total_cmp);
            if values.iter().any(|value| !value.is_finite() || *value <= 0.0) {
                return Err("generalized eigenproblem has a non-positive eigenvalue".to_owned());
            }
            return Ok(values);
        }
        let (p, q) = pivot.ok_or_else(|| "Jacobi eigensolver has no pivot".to_owned())?;
        let app = matrix[p][p];
        let aqq = matrix[q][q];
        let apq = matrix[p][q];
        let tau = (aqq - app) / (2.0 * apq);
        let t = if tau >= 0.0 {
            1.0 / (tau + tau.hypot(1.0))
        } else {
            -1.0 / (-tau + tau.hypot(1.0))
        };
        let cosine = 1.0 / (1.0 + t * t).sqrt();
        let sine = t * cosine;

        for k in 0..n {
            if k != p && k != q {
                let akp = matrix[k][p];
                let akq = matrix[k][q];
                matrix[k][p] = cosine * akp - sine * akq;
                matrix[p][k] = matrix[k][p];
                matrix[k][q] = sine * akp + cosine * akq;
                matrix[q][k] = matrix[k][q];
            }
        }
        matrix[p][p] = cosine * cosine * app - 2.0 * sine * cosine * apq
            + sine * sine * aqq;
        matrix[q][q] = sine * sine * app
            + 2.0 * sine * cosine * apq
            + cosine * cosine * aqq;
        matrix[p][q] = 0.0;
        matrix[q][p] = 0.0;
    }
    Err("Jacobi eigensolver did not converge within its iteration budget".to_owned())
}

#[cfg(test)]
mod tests {
    use super::{
        FixedFreeBar, MassFormulation, analyze_fixed_free_bar, validate_bar,
    };

    fn nafems_reference_bar(elements: usize) -> FixedFreeBar {
        FixedFreeBar {
            length_m: 1.0,
            area_m2: std::f64::consts::PI * 0.01_f64.powi(2),
            youngs_modulus_pa: 210.0e9,
            density_kg_m3: 7_800.0,
            elements,
        }
    }

    fn close(actual: f64, expected: f64, tolerance: f64, label: &str) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "{label}: actual={actual:.12e}, expected={expected:.12e}, tolerance={tolerance:.3e}"
        );
    }

    #[test]
    fn one_element_brackets_exact_fixed_free_frequency_for_both_mass_models() {
        let bar = nafems_reference_bar(1);
        let exact = (bar.youngs_modulus_pa / bar.density_kg_m3).sqrt()
            / (4.0 * bar.length_m);
        let consistent = analyze_fixed_free_bar(bar, MassFormulation::Consistent)
            .expect("consistent-mass solution");
        let lumped = analyze_fixed_free_bar(bar, MassFormulation::RowSumLumped)
            .expect("row-sum-lumped solution");

        close(
            consistent.normalized_frequencies[0],
            2.0 * 3.0_f64.sqrt() / std::f64::consts::PI,
            1.0e-12,
            "one-element consistent ratio",
        );
        close(
            lumped.normalized_frequencies[0],
            2.0 * 2.0_f64.sqrt() / std::f64::consts::PI,
            1.0e-12,
            "one-element lumped ratio",
        );
        assert!(consistent.frequencies_hz[0] > exact);
        assert!(lumped.frequencies_hz[0] < exact);
    }

    #[test]
    fn first_mode_converges_from_opposite_sides_under_mesh_refinement() {
        let mut previous_consistent_error = f64::INFINITY;
        let mut previous_lumped_error = f64::INFINITY;
        for elements in [1, 2, 4, 8, 16] {
            let bar = nafems_reference_bar(elements);
            let consistent = analyze_fixed_free_bar(bar, MassFormulation::Consistent)
                .expect("consistent-mass sweep");
            let lumped = analyze_fixed_free_bar(bar, MassFormulation::RowSumLumped)
                .expect("row-sum-lumped sweep");
            let exact = consistent.exact_frequencies_hz[0];
            let consistent_ratio = consistent.normalized_frequencies[0];
            let lumped_ratio = lumped.normalized_frequencies[0];
            let consistent_error = (consistent_ratio - 1.0).abs();
            let lumped_error = (lumped_ratio - 1.0).abs();

            assert!(consistent_ratio >= 1.0);
            assert!(lumped_ratio <= 1.0);
            assert!(consistent_error < previous_consistent_error);
            assert!(lumped_error < previous_lumped_error);
            assert!(consistent.frequencies_hz[0] > 0.0);
            assert!(lumped.frequencies_hz[0] > 0.0);
            close(exact, lumped.exact_frequencies_hz[0], 1.0e-10, "exact frequency");
            previous_consistent_error = consistent_error;
            previous_lumped_error = lumped_error;
        }
        assert!(previous_consistent_error < 0.01);
        assert!(previous_lumped_error < 0.01);
    }

    #[test]
    fn second_mode_converges_toward_independent_continuum_value() {
        let mut previous_consistent_error = f64::INFINITY;
        let mut previous_lumped_error = f64::INFINITY;
        for elements in [2, 4, 8, 16] {
            let bar = nafems_reference_bar(elements);
            let consistent = analyze_fixed_free_bar(bar, MassFormulation::Consistent)
                .expect("consistent-mass sweep");
            let lumped = analyze_fixed_free_bar(bar, MassFormulation::RowSumLumped)
                .expect("row-sum-lumped sweep");
            let consistent_error = (consistent.normalized_frequencies[1] - 1.0).abs();
            let lumped_error = (lumped.normalized_frequencies[1] - 1.0).abs();
            assert!(consistent_error < previous_consistent_error);
            assert!(lumped_error < previous_lumped_error);
            previous_consistent_error = consistent_error;
            previous_lumped_error = lumped_error;
        }
        assert!(previous_consistent_error < 0.02);
        assert!(previous_lumped_error < 0.02);
    }

    #[test]
    fn frequency_is_invariant_to_area_and_joint_scaling_of_modulus_and_density() {
        let reference = analyze_fixed_free_bar(
            nafems_reference_bar(8),
            MassFormulation::Consistent,
        )
        .expect("reference modal solution");
        let mut scaled_area = nafems_reference_bar(8);
        scaled_area.area_m2 *= 7.0;
        let area_result =
            analyze_fixed_free_bar(scaled_area, MassFormulation::Consistent)
                .expect("area-scaled modal solution");
        let mut scaled_material = nafems_reference_bar(8);
        scaled_material.youngs_modulus_pa *= 9.0;
        scaled_material.density_kg_m3 *= 9.0;
        let material_result =
            analyze_fixed_free_bar(scaled_material, MassFormulation::Consistent)
                .expect("modulus/density-scaled modal solution");
        let mut scaled_length = nafems_reference_bar(8);
        scaled_length.length_m *= 2.0;
        let length_result =
            analyze_fixed_free_bar(scaled_length, MassFormulation::Consistent)
                .expect("length-scaled modal solution");

        close(
            area_result.frequencies_hz[0],
            reference.frequencies_hz[0],
            1.0e-9,
            "area invariance",
        );
        close(
            material_result.frequencies_hz[0],
            reference.frequencies_hz[0],
            1.0e-9,
            "E/rho invariance",
        );
        close(
            length_result.frequencies_hz[0],
            reference.frequencies_hz[0] / 2.0,
            1.0e-9,
            "inverse-length scaling",
        );
    }

    #[test]
    fn rejects_unsupported_domains_before_assembly() {
        let mut bar = nafems_reference_bar(4);
        bar.length_m = 0.0;
        assert!(validate_bar(bar).is_err());
        bar = nafems_reference_bar(4);
        bar.density_kg_m3 = f64::NAN;
        assert!(validate_bar(bar).is_err());
        bar = nafems_reference_bar(0);
        assert!(validate_bar(bar).is_err());
        bar = nafems_reference_bar(33);
        assert!(validate_bar(bar).is_err());
        bar = nafems_reference_bar(1);
        bar.youngs_modulus_pa = 0.0;
        assert!(validate_bar(bar).is_err());
    }

    #[test]
    fn mode_count_equals_free_axial_degree_count_and_frequencies_are_sorted() {
        for elements in [1, 2, 4, 8, 16] {
            for formulation in [
                MassFormulation::Consistent,
                MassFormulation::RowSumLumped,
            ] {
                let result = analyze_fixed_free_bar(nafems_reference_bar(elements), formulation)
                    .expect("bounded modal solve");
                assert_eq!(result.frequencies_hz.len(), elements);
                assert!(result.frequencies_hz.windows(2).all(|pair| pair[0] < pair[1]));
                assert!(result
                    .normalized_frequencies
                    .iter()
                    .all(|ratio| ratio.is_finite() && *ratio > 0.0));
            }
        }
    }
}
