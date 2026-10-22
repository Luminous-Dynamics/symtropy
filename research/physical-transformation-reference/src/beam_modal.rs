// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dense generalized modal solve for the deterministic released-joint beam
//! assembly. This reuses the reference crate's Cholesky-reduced symmetric
//! eigenproblem and rejects zero/near-zero modes rather than silently dropping
//! them. This is an early small-mesh verification layer, not a full NBC05
//! convergence or physical-validation result.

use crate::beam_pin_assembly::BeamPinAssembly;
use crate::modal_bar::{
    Matrix, generalized_symmetric_eigenpairs, matrix_infinity_norm,
    normalized_eigen_residual, quadratic_form,
};
use crate::Check;
use std::f64::consts::PI;

const DEFAULT_MECHANISM_RELATIVE_TOLERANCE: f64 = 1.0e-12;
const MAX_MECHANISM_RELATIVE_TOLERANCE: f64 = 1.0e-3;

/// Explicit modal request options. The mechanism threshold is relative to
/// ||K||_inf / ||M||_inf and is part of the result, not a hidden solver default.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamModalOptions {
    pub requested_modes: usize,
    pub mechanism_eigenvalue_relative_tolerance: f64,
}

impl BeamModalOptions {
    pub fn first_modes(requested_modes: usize) -> Self {
        Self {
            requested_modes,
            mechanism_eigenvalue_relative_tolerance: DEFAULT_MECHANISM_RELATIVE_TOLERANCE,
        }
    }
}

/// One mass-normalized generalized eigenmode of the assembled beam structure.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalMode {
    pub ordinal: usize,
    pub eigenvalue_rad_s_squared: f64,
    pub frequency_hz: f64,
    /// Shape restricted to the ordered free-DOF map.
    pub shape_free_dofs: Vec<f64>,
    /// Full assembly-sized shape with restrained coordinates set to zero.
    pub shape_global_dofs: Vec<f64>,
    pub normalized_eigen_residual: f64,
    pub mass_norm_residual: f64,
}

/// Diagnostics accompany the frequencies, so a list of plausible numbers
/// cannot stand in for a well-conditioned generalized eigenproblem.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalSpectrum {
    pub modes: Vec<BeamModalMode>,
    pub total_free_dof_count: usize,
    pub requested_mode_count: usize,
    pub returned_mode_count: usize,
    pub stiffness_infinity_norm: f64,
    pub mass_infinity_norm: f64,
    pub reference_eigenvalue_scale_rad_s_squared: f64,
    pub mechanism_eigenvalue_relative_tolerance: f64,
    pub maximum_mass_orthogonality_residual: f64,
    pub maximum_normalized_eigen_residual: f64,
    pub maximum_mass_norm_residual: f64,
    pub claim_ceiling: &'static str,
}

/// Solve the constrained generalized eigenproblem K_ff phi = lambda M_ff phi
/// for an assembled beam model. Any zero/near-zero eigenvalue is classified as
/// a mechanism or insufficiently resolved mode and causes an error. The
/// caller must correct the constraints/model or explicitly revise the named
/// tolerance; the solver never silently filters such a mode.
pub fn solve_released_joint_beam_modes(
    assembly: &BeamPinAssembly,
    options: BeamModalOptions,
) -> Check<BeamModalSpectrum> {
    if assembly.stiffness_global.is_empty()
        || assembly.stiffness_global.len() != assembly.mass_global.len()
    {
        return Err("global stiffness and mass dimensions do not agree".to_owned());
    }
    if options.requested_modes == 0 || options.requested_modes > assembly.free_dof_indices.len() {
        return Err(format!(
            "requested mode count must be between 1 and {}",
            assembly.free_dof_indices.len()
        ));
    }
    if !options.mechanism_eigenvalue_relative_tolerance.is_finite()
        || options.mechanism_eigenvalue_relative_tolerance <= 0.0
        || options.mechanism_eigenvalue_relative_tolerance > MAX_MECHANISM_RELATIVE_TOLERANCE
    {
        return Err(format!(
            "mechanism tolerance must be finite and in (0, {MAX_MECHANISM_RELATIVE_TOLERANCE}]"
        ));
    }

    let free = &assembly.free_dof_indices;
    let full_size = assembly.stiffness_global.len();
    if assembly.mass_global.len() != full_size
        || assembly.stiffness_global.iter().any(|row| row.len() != full_size)
        || assembly.mass_global.iter().any(|row| row.len() != full_size)
        || free.iter().any(|index| *index >= full_size)
    {
        return Err("assembly matrices or free-DOF map have invalid dimensions".to_owned());
    }

    let stiffness = reduce_matrix(&assembly.stiffness_global, free);
    let mass = reduce_matrix(&assembly.mass_global, free);
    if stiffness.is_empty() || mass.is_empty() {
        return Err("the constrained model has no free degrees of freedom".to_owned());
    }
    let stiffness_norm = matrix_infinity_norm(&stiffness);
    let mass_norm = matrix_infinity_norm(&mass);
    if !stiffness_norm.is_finite()
        || stiffness_norm <= 0.0
        || !mass_norm.is_finite()
        || mass_norm <= 0.0
    {
        return Err("reduced stiffness and mass norms must be finite and positive".to_owned());
    }
    let eigenvalue_scale = stiffness_norm / mass_norm;
    if !eigenvalue_scale.is_finite() || eigenvalue_scale <= 0.0 {
        return Err("reference eigenvalue scale is not finite and positive".to_owned());
    }
    let mechanism_floor = eigenvalue_scale * options.mechanism_eigenvalue_relative_tolerance;

    let eigenpairs = generalized_symmetric_eigenpairs(&stiffness, &mass)?;
    if options.requested_modes > eigenpairs.len() {
        return Err("requested mode count exceeds the available eigenpairs".to_owned());
    }

    // Examine every eigenvalue, not just the requested leading modes. A
    // mechanism above the output cutoff still makes this model unqualified.
    for (index, (eigenvalue, _)) in eigenpairs.iter().enumerate() {
        if !eigenvalue.is_finite() || *eigenvalue <= mechanism_floor {
            return Err(format!(
                "mode {} is non-positive or below the mechanism floor: lambda={:.17e}, floor={:.17e}",
                index + 1,
                eigenvalue,
                mechanism_floor
            ));
        }
    }

    let mut max_orthogonality_residual = 0.0_f64;
    for left in 0..eigenpairs.len() {
        for right in 0..eigenpairs.len() {
            let product = matrix_vector(&mass, &eigenpairs[right].1)?;
            let modal_mass: f64 = eigenpairs[left]
                .1
                .iter()
                .zip(product)
                .map(|(a, b)| a * b)
                .sum();
            if !modal_mass.is_finite() {
                return Err("modal orthogonality calculation is non-finite".to_owned());
            }
            let expected = if left == right { 1.0 } else { 0.0 };
            max_orthogonality_residual =
                max_orthogonality_residual.max((modal_mass - expected).abs());
        }
    }

    let mut modes = Vec::with_capacity(options.requested_modes);
    let mut max_eigen_residual = 0.0_f64;
    let mut max_mass_norm_residual = 0.0_f64;
    for (ordinal, (eigenvalue, shape_free)) in
        eigenpairs.iter().take(options.requested_modes).enumerate()
    {
        let mass_norm = quadratic_form(&mass, shape_free)?;
        let mass_norm_residual = (mass_norm - 1.0).abs();
        let eigen_residual =
            normalized_eigen_residual(&stiffness, &mass, *eigenvalue, shape_free)?;
        if !eigen_residual.is_finite() || !mass_norm_residual.is_finite() {
            return Err(format!("mode {} has non-finite diagnostics", ordinal + 1));
        }
        let frequency_hz = eigenvalue.sqrt() / (2.0 * PI);
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
            return Err(format!("mode {} has an invalid natural frequency", ordinal + 1));
        }
        let mut shape_global = vec![0.0; full_size];
        for (free_position, global_index) in free.iter().enumerate() {
            shape_global[*global_index] = shape_free[free_position];
        }
        max_eigen_residual = max_eigen_residual.max(eigen_residual);
        max_mass_norm_residual = max_mass_norm_residual.max(mass_norm_residual);
        modes.push(BeamModalMode {
            ordinal: ordinal + 1,
            eigenvalue_rad_s_squared: *eigenvalue,
            frequency_hz,
            shape_free_dofs: shape_free.clone(),
            shape_global_dofs: shape_global,
            normalized_eigen_residual: eigen_residual,
            mass_norm_residual,
        });
    }

    Ok(BeamModalSpectrum {
        modes,
        total_free_dof_count: free.len(),
        requested_mode_count: options.requested_modes,
        returned_mode_count: options.requested_modes,
        stiffness_infinity_norm: stiffness_norm,
        mass_infinity_norm: mass_norm,
        reference_eigenvalue_scale_rad_s_squared: eigenvalue_scale,
        mechanism_eigenvalue_relative_tolerance: options.mechanism_eigenvalue_relative_tolerance,
        maximum_mass_orthogonality_residual: max_orthogonality_residual,
        maximum_normalized_eigen_residual: max_eigen_residual,
        maximum_mass_norm_residual: max_mass_norm_residual,
        claim_ceiling: "coarse single-element-per-member released-joint Euler-Bernoulli beam model; no full NBC05 convergence or physical validation",
    })
}

fn reduce_matrix(matrix: &[Vec<f64>], free_indices: &[usize]) -> Matrix {
    free_indices
        .iter()
        .map(|row| {
            free_indices
                .iter()
                .map(|col| matrix[*row][*col])
                .collect()
        })
        .collect()
}

fn matrix_vector(matrix: &Matrix, vector: &[f64]) -> Check<Vec<f64>> {
    if matrix.len() != vector.len() || matrix.iter().any(|row| row.len() != vector.len()) {
        return Err("matrix/vector dimensions do not agree".to_owned());
    }
    Ok(matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(a, b)| a * b).sum())
        .collect())
}

#[cfg(test)]
mod tests {
    use super::{BeamModalOptions, solve_released_joint_beam_modes};
    use crate::beam_pin_assembly::assemble_released_joint_pin_truss;

    fn fixture_inputs() -> (Vec<crate::beam_pin_assembly::BeamPinNode>, Vec<crate::beam_pin_assembly::BeamPinMember>) {
        crate::beam_pin_assembly::nafems_nbc05_released_joint_beam_fixture()
    }

    fn frequencies(area_scale: f64, modulus_scale: f64, density_scale: f64) -> Vec<f64> {
        let (nodes, mut members) = fixture_inputs();
        for member in &mut members {
            member.area_m2 *= area_scale;
            member.second_moment_m4 *= area_scale;
            member.youngs_modulus_pa *= modulus_scale;
            member.density_kg_m3 *= density_scale;
        }
        let assembly = assemble_released_joint_pin_truss(&nodes, &members)
            .expect("valid released-joint fixture");
        solve_released_joint_beam_modes(&assembly, BeamModalOptions::first_modes(6))
            .expect("valid beam modal spectrum")
            .modes
            .into_iter()
            .map(|mode| mode.frequency_hz)
            .collect()
    }

    #[test]
    fn six_modes_are_sorted_positive_and_have_small_eigenpair_residuals() {
        let (nodes, members) = fixture_inputs();
        let assembly = assemble_released_joint_pin_truss(&nodes, &members)
            .expect("valid assembly");
        let spectrum = solve_released_joint_beam_modes(&assembly, BeamModalOptions::first_modes(6))
            .expect("six-mode solution");

        assert_eq!(spectrum.total_free_dof_count, 6);
        assert_eq!(spectrum.returned_mode_count, 6);
        assert!(spectrum.modes.iter().all(|mode| mode.frequency_hz.is_finite() && mode.frequency_hz > 0.0));
        assert!(spectrum.modes.windows(2).all(|pair| pair[0].frequency_hz < pair[1].frequency_hz));
        assert!(spectrum.maximum_normalized_eigen_residual < 1.0e-8);
        assert!(spectrum.maximum_mass_norm_residual < 1.0e-8);
        assert!(spectrum.maximum_mass_orthogonality_residual < 1.0e-8);
        assert!(spectrum.modes.iter().all(|mode| mode.shape_global_dofs.len() == 10));
    }

    #[test]
    fn two_free_beam_rotations_match_independent_closed_form_generalized_eigenvalues() {
        use crate::beam_pin_assembly::{BeamPinMember, BeamPinNode};

        let length_m = 2.0_f64;
        let area_m2 = 0.004_f64;
        let youngs_modulus_pa = 70.0e9_f64;
        let density_kg_m3 = 2700.0_f64;
        let second_moment_m4 = 2.0e-6_f64;
        let nodes = vec![
            BeamPinNode {
                id: "A".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "B".to_owned(),
                x_m: length_m,
                y_m: 0.0,
                restrain_x: true,
                restrain_y: true,
            },
        ];
        let members = vec![BeamPinMember {
            id: "AB".to_owned(),
            start_node_id: "A".to_owned(),
            end_node_id: "B".to_owned(),
            area_m2,
            youngs_modulus_pa,
            density_kg_m3,
            second_moment_m4,
        }];
        let assembly = assemble_released_joint_pin_truss(&nodes, &members)
            .expect("simply-supported beam assembly");
        let spectrum = solve_released_joint_beam_modes(&assembly, BeamModalOptions::first_modes(2))
            .expect("two rotational modes");

        // Independent closed-form 2x2 generalized eigenproblem for the two
        // end rotations with both endpoint translations restrained. This
        // bypasses the production eigen-solver entirely.
        let k11 = 4.0 * youngs_modulus_pa * second_moment_m4 / length_m;
        let k12 = 2.0 * youngs_modulus_pa * second_moment_m4 / length_m;
        let translational_factor = density_kg_m3 * area_m2 * length_m / 420.0;
        let rotary_factor = density_kg_m3 * second_moment_m4 * length_m / 30.0;
        let m11 = translational_factor * 4.0 * length_m.powi(2) + rotary_factor * 4.0;
        let m12 = translational_factor * -3.0 * length_m.powi(2) - rotary_factor;
        let quadratic_a = m11 * m11 - m12 * m12;
        let quadratic_b = -2.0 * k11 * m11 + 2.0 * k12 * m12;
        let quadratic_c = k11 * k11 - k12 * k12;
        let discriminant = quadratic_b * quadratic_b
            - 4.0 * quadratic_a * quadratic_c;
        assert!(discriminant.is_finite() && discriminant > 0.0);
        let root = discriminant.sqrt();
        let expected = [
            (-quadratic_b - root) / (2.0 * quadratic_a),
            (-quadratic_b + root) / (2.0 * quadratic_a),
        ];

        for (mode, expected_eigenvalue) in spectrum.modes.iter().zip(expected) {
            let relative_error =
                (mode.eigenvalue_rad_s_squared - expected_eigenvalue).abs() / expected_eigenvalue;
            assert!(
                relative_error < 2.0e-9,
                "mode {} eigenvalue relative error={relative_error:.17e}",
                mode.ordinal
            );
            let expected_frequency = expected_eigenvalue.sqrt() / (2.0 * std::f64::consts::PI);
            let frequency_error =
                (mode.frequency_hz - expected_frequency).abs() / expected_frequency;
            assert!(
                frequency_error < 2.0e-9,
                "mode {} frequency relative error={frequency_error:.17e}",
                mode.ordinal
            );
        }
    }

    #[test]
    fn modal_spectrum_obeys_material_and_uniform_section_scaling() {
        let base = frequencies(1.0, 1.0, 1.0);
        let four_x_modulus = frequencies(1.0, 4.0, 1.0);
        let four_x_density = frequencies(1.0, 1.0, 4.0);
        let four_x_area_and_second_moment = frequencies(4.0, 1.0, 1.0);
        for mode in 0..6 {
            assert!(((four_x_modulus[mode] / base[mode]) - 2.0).abs() < 1.0e-8);
            assert!(((four_x_density[mode] / base[mode]) - 0.5).abs() < 1.0e-8);
            assert!((four_x_area_and_second_moment[mode] / base[mode] - 1.0).abs() < 1.0e-8);
        }
    }

    #[test]
    fn zero_modes_are_reported_as_unqualified_not_silently_filtered() {
        let (nodes, members) = fixture_inputs();
        let mut assembly = assemble_released_joint_pin_truss(&nodes, &members)
            .expect("valid assembly");
        // Preserve a nonzero stiffness scale, but create one isolated free
        // coordinate with exactly zero stiffness so the reduced K is singular.
        let mechanism_dof = assembly.free_dof_indices[0];
        for row in &mut assembly.stiffness_global {
            row[mechanism_dof] = 0.0;
        }
        assembly.stiffness_global[mechanism_dof].fill(0.0);
        let error = solve_released_joint_beam_modes(&assembly, BeamModalOptions::first_modes(2))
            .expect_err("a zero eigenvalue must fail closed");
        assert!(error.contains("mechanism floor"), "{error}");
    }

    #[test]
    fn invalid_mode_requests_and_mechanism_thresholds_fail_closed() {
        let (nodes, members) = fixture_inputs();
        let assembly = assemble_released_joint_pin_truss(&nodes, &members)
            .expect("valid assembly");
        assert!(solve_released_joint_beam_modes(&assembly, BeamModalOptions::first_modes(0)).is_err());
        assert!(solve_released_joint_beam_modes(&assembly, BeamModalOptions::first_modes(7)).is_err());

        let options = BeamModalOptions {
            requested_modes: 1,
            mechanism_eigenvalue_relative_tolerance: 0.01,
        };
        assert!(solve_released_joint_beam_modes(&assembly, options).is_err());
    }
}
