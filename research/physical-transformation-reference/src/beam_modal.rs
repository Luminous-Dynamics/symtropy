// SPDX-License-Identifier: AGPL-3.0-or-later
//! Small dense modal solve for the explicit planar released-joint beam model.
//!
//! This consumes the global stiffness/mass assembly, reduces to the declared
//! free DOF set and calls the shared dense generalized symmetric eigensolver.
//! It is deliberately capped at 128 free DOFs; this is not the sparse/modal
//! backend needed for the 200-elements-per-member NAFEMS convergence study.

#![allow(clippy::needless_range_loop)] // Modal matrix reduction and residuals intentionally index coupled DOFs.

use crate::beam_pin_assembly::BeamPinAssembly;
use crate::modal_bar::solve_dense_generalized_symmetric;
use crate::Check;

const MAX_DENSE_BEAM_DOFS: usize = 128;

/// One computed generalized eigenpair for the released-joint beam model.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamPinMode {
    /// One-based ascending mode number within this constrained model.
    pub mode_number: usize,
    /// Generalized eigenvalue in rad²/s².
    pub eigenvalue_rad_s_squared: f64,
    /// Natural frequency in hertz.
    pub frequency_hz: f64,
    /// Mass-normalized shape aligned with BeamPinAssembly::free_dof_indices.
    pub shape_free_dofs: Vec<f64>,
    /// Full shape aligned with global DOF indices; restrained DOFs are zero.
    pub shape_global_dofs: Vec<f64>,
    /// Scaled infinity norm of K*phi - lambda*M*phi.
    pub normalized_eigen_residual: f64,
    /// Absolute deviation of phiᵀ M phi from one.
    pub mass_norm_residual: f64,
}

/// Auditable output of a small dense released-joint beam modal solve.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamPinModalSolution {
    pub total_dof_count: usize,
    pub free_dof_indices: Vec<usize>,
    pub fixed_dof_indices: Vec<usize>,
    pub requested_mode_count: usize,
    pub solver_method: &'static str,
    pub modes: Vec<BeamPinMode>,
    pub claim_ceiling: &'static str,
}

/// Solve a small constrained generalized eigenproblem from the beam assembly.
///
/// The free/fixed partition must be complete, unique and in range. Non-square,
/// asymmetric or non-finite matrices fail closed. A non-positive eigenvalue
/// is not silently discarded as a rigid/mechanism mode; this function returns
/// an error and leaves mode classification to a dedicated follow-up contract.
pub fn solve_released_joint_beam_modes(
    assembly: &BeamPinAssembly,
    requested_modes: usize,
) -> Check<BeamPinModalSolution> {
    let total_dofs = assembly.stiffness_global.len();
    if total_dofs == 0 || total_dofs > MAX_DENSE_BEAM_DOFS {
        return Err(format!(
            "dense beam modal solve requires 1..={MAX_DENSE_BEAM_DOFS} total DOFs"
        ));
    }
    validate_square_matrix("stiffness_global", &assembly.stiffness_global, total_dofs)?;
    validate_square_matrix("mass_global", &assembly.mass_global, total_dofs)?;
    let free = validate_dof_partition(
        total_dofs,
        &assembly.fixed_dof_indices,
        &assembly.free_dof_indices,
    )?;
    if free.is_empty() {
        return Err("beam modal solve has no free DOFs".to_owned());
    }
    if requested_modes == 0 || requested_modes > free.len() {
        return Err(format!(
            "requested modes must be between 1 and {}",
            free.len()
        ));
    }

    let stiffness_reduced = reduce_matrix(&assembly.stiffness_global, &free);
    let mass_reduced = reduce_matrix(&assembly.mass_global, &free);
    let eigenpairs = solve_dense_generalized_symmetric(&stiffness_reduced, &mass_reduced)?;
    if eigenpairs.len() != free.len() {
        return Err("generalized eigensolver returned an incomplete spectrum".to_owned());
    }

    let stiffness_norm = matrix_infinity_norm(&stiffness_reduced);
    let mass_norm = matrix_infinity_norm(&mass_reduced);
    let mut modes = Vec::with_capacity(requested_modes);
    for (zero_index, (eigenvalue, shape)) in
        eigenpairs.into_iter().take(requested_modes).enumerate()
    {
        if !eigenvalue.is_finite() || eigenvalue <= 0.0 {
            return Err(format!(
                "mode {} has a non-positive/non-finite eigenvalue ({eigenvalue:.17e}); possible mechanism or instability",
                zero_index + 1
            ));
        }
        if shape.len() != free.len() || shape.iter().any(|value| !value.is_finite()) {
            return Err(format!("mode {} has an invalid eigenvector", zero_index + 1));
        }

        let frequency_hz = eigenvalue.sqrt() / (2.0 * std::f64::consts::PI);
        if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
            return Err(format!("mode {} has an invalid frequency", zero_index + 1));
        }
        let mass_norm_squared = quadratic_form(&mass_reduced, &shape);
        if !mass_norm_squared.is_finite() || mass_norm_squared <= 0.0 {
            return Err(format!("mode {} has invalid modal mass", zero_index + 1));
        }
        let mass_norm_residual = (mass_norm_squared - 1.0).abs();
        if mass_norm_residual > 1.0e-8 {
            return Err(format!(
                "mode {} is not mass normalized: residual={mass_norm_residual:.17e}",
                zero_index + 1
            ));
        }

        let normalized_eigen_residual = normalized_eigen_residual(
            &stiffness_reduced,
            &mass_reduced,
            eigenvalue,
            &shape,
            stiffness_norm,
            mass_norm,
        )?;
        if normalized_eigen_residual > 1.0e-8 {
            return Err(format!(
                "mode {} eigenpair residual exceeds tolerance: {:.17e}",
                zero_index + 1,
                normalized_eigen_residual
            ));
        }

        let mut global_shape = vec![0.0; total_dofs];
        for (free_position, global_index) in free.iter().copied().enumerate() {
            global_shape[global_index] = shape[free_position];
        }
        modes.push(BeamPinMode {
            mode_number: zero_index + 1,
            eigenvalue_rad_s_squared: eigenvalue,
            frequency_hz,
            shape_free_dofs: shape,
            shape_global_dofs: global_shape,
            normalized_eigen_residual,
            mass_norm_residual,
        });
    }

    Ok(BeamPinModalSolution {
        total_dof_count: total_dofs,
        free_dof_indices: free,
        fixed_dof_indices: assembly.fixed_dof_indices.clone(),
        requested_mode_count: requested_modes,
        solver_method: "dense-cholesky-jacobi-generalized-symmetric",
        modes,
        claim_ceiling: "small synthetic released-joint Euler-Bernoulli model; no 200-element/member convergence, full NAFEMS beam reproduction, physical validation or structural-safety claim",
    })
}

fn validate_square_matrix(name: &str, matrix: &[Vec<f64>], size: usize) -> Check<()> {
    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err(format!("{name} must be {size}x{size}"));
    }
    if matrix.iter().flatten().any(|value| !value.is_finite()) {
        return Err(format!("{name} contains a non-finite value"));
    }
    for row in 0..size {
        for col in 0..size {
            let scale = matrix[row][col]
                .abs()
                .max(matrix[col][row].abs())
                .max(1.0);
            if (matrix[row][col] - matrix[col][row]).abs() > 1.0e-10 * scale {
                return Err(format!("{name} is not symmetric at ({row}, {col})"));
            }
        }
    }
    Ok(())
}

fn validate_dof_partition(
    total_dofs: usize,
    fixed_dofs: &[usize],
    free_dofs: &[usize],
) -> Check<Vec<usize>> {
    let mut seen = vec![false; total_dofs];
    for (kind, indices) in [("fixed", fixed_dofs), ("free", free_dofs)] {
        for index in indices {
            if *index >= total_dofs {
                return Err(format!("{kind} DOF index {index} is out of range"));
            }
            if seen[*index] {
                return Err(format!("DOF index {index} is duplicated or both fixed/free"));
            }
            seen[*index] = true;
        }
    }
    if seen.iter().any(|value| !value) {
        return Err("fixed/free DOF sets do not cover the whole model".to_owned());
    }
    let mut free = free_dofs.to_vec();
    free.sort_unstable();
    Ok(free)
}

fn reduce_matrix(matrix: &[Vec<f64>], free: &[usize]) -> Vec<Vec<f64>> {
    free.iter()
        .map(|row| {
            free.iter()
                .map(|col| matrix[*row][*col])
                .collect::<Vec<_>>()
        })
        .collect()
}

fn matrix_vector_product(matrix: &[Vec<f64>], vector: &[f64]) -> Vec<f64> {
    matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(left, right)| left * right).sum())
        .collect()
}

fn matrix_infinity_norm(matrix: &[Vec<f64>]) -> f64 {
    matrix
        .iter()
        .map(|row| row.iter().map(|value| value.abs()).sum::<f64>())
        .fold(0.0_f64, f64::max)
}

fn vector_infinity_norm(vector: &[f64]) -> f64 {
    vector.iter().map(|value| value.abs()).fold(0.0_f64, f64::max)
}

fn quadratic_form(matrix: &[Vec<f64>], vector: &[f64]) -> f64 {
    let product = matrix_vector_product(matrix, vector);
    vector.iter().zip(product).map(|(left, right)| left * right).sum()
}

fn normalized_eigen_residual(
    stiffness: &[Vec<f64>],
    mass: &[Vec<f64>],
    eigenvalue: f64,
    shape: &[f64],
    stiffness_norm: f64,
    mass_norm: f64,
) -> Check<f64> {
    let stiffness_shape = matrix_vector_product(stiffness, shape);
    let mass_shape = matrix_vector_product(mass, shape);
    let numerator = stiffness_shape
        .iter()
        .zip(&mass_shape)
        .map(|(left, right)| (left - eigenvalue * right).abs())
        .fold(0.0_f64, f64::max);
    let denominator =
        (stiffness_norm + eigenvalue.abs() * mass_norm) * vector_infinity_norm(shape);
    if !numerator.is_finite() || !denominator.is_finite() || denominator <= 0.0 {
        return Err("cannot normalize generalized eigenpair residual".to_owned());
    }
    let residual = numerator / denominator;
    if residual.is_finite() {
        Ok(residual)
    } else {
        Err("normalized eigenpair residual is non-finite".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::solve_released_joint_beam_modes;
    use crate::beam_pin_assembly::{
        assemble_released_joint_pin_truss, BeamPinMember, BeamPinNode,
    };

    fn sample_nodes() -> Vec<BeamPinNode> {
        vec![
            BeamPinNode {
                id: "A".to_owned(),
                x_m: 0.0,
                y_m: 1.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "B".to_owned(),
                x_m: 0.0,
                y_m: 0.0,
                restrain_x: true,
                restrain_y: true,
            },
            BeamPinNode {
                id: "C".to_owned(),
                x_m: 1.0,
                y_m: 1.0,
                restrain_x: false,
                restrain_y: false,
            },
        ]
    }

    fn sample_members() -> Vec<BeamPinMember> {
        vec![
            BeamPinMember {
                id: "AC".to_owned(),
                start_node_id: "A".to_owned(),
                end_node_id: "C".to_owned(),
                area_m2: 3.141592653589793e-4,
                youngs_modulus_pa: 210.0e9,
                density_kg_m3: 7800.0,
                second_moment_m4: 7.853981633974483e-9,
            },
            BeamPinMember {
                id: "BC".to_owned(),
                start_node_id: "B".to_owned(),
                end_node_id: "C".to_owned(),
                area_m2: 3.141592653589793e-4,
                youngs_modulus_pa: 210.0e9,
                density_kg_m3: 7800.0,
                second_moment_m4: 7.853981633974483e-9,
            },
        ]
    }

    #[test]
    fn supported_two_member_pin_beam_returns_six_stable_modes() {
        let assembly =
            assemble_released_joint_pin_truss(&sample_nodes(), &sample_members())
                .expect("valid NAFEMS-shaped pin-beam matrix model");
        let solution = solve_released_joint_beam_modes(&assembly, 6)
            .expect("stable synthetic released-joint beam modes");
        assert_eq!(solution.total_dof_count, 10);
        assert_eq!(solution.free_dof_indices.len(), 6);
        assert_eq!(solution.fixed_dof_indices.len(), 4);
        assert_eq!(solution.modes.len(), 6);
        for (index, mode) in solution.modes.iter().enumerate() {
            assert_eq!(mode.mode_number, index + 1);
            assert!(mode.frequency_hz.is_finite() && mode.frequency_hz > 0.0);
            assert!(mode.normalized_eigen_residual < 1.0e-8);
            assert!(mode.mass_norm_residual < 1.0e-8);
            assert_eq!(mode.shape_global_dofs.len(), solution.total_dof_count);
            for fixed in &solution.fixed_dof_indices {
                assert_eq!(mode.shape_global_dofs[*fixed], 0.0);
            }
        }
    }

    #[test]
    fn modal_frequencies_obey_modulus_and_density_scaling() {
        let baseline_assembly =
            assemble_released_joint_pin_truss(&sample_nodes(), &sample_members())
                .expect("baseline assembly");
        let baseline =
            solve_released_joint_beam_modes(&baseline_assembly, 6).expect("baseline modes");

        let mut stiff_members = sample_members();
        for member in &mut stiff_members {
            member.youngs_modulus_pa *= 4.0;
        }
        let stiff_assembly =
            assemble_released_joint_pin_truss(&sample_nodes(), &stiff_members)
                .expect("stiff assembly");
        let stiff = solve_released_joint_beam_modes(&stiff_assembly, 6).expect("stiff modes");

        let mut dense_members = sample_members();
        for member in &mut dense_members {
            member.density_kg_m3 *= 4.0;
        }
        let dense_assembly =
            assemble_released_joint_pin_truss(&sample_nodes(), &dense_members)
                .expect("dense assembly");
        let dense = solve_released_joint_beam_modes(&dense_assembly, 6).expect("dense modes");

        for mode in 0..6 {
            assert!(
                ((stiff.modes[mode].frequency_hz / baseline.modes[mode].frequency_hz) - 2.0)
                    .abs()
                    < 1.0e-8
            );
            assert!(
                ((dense.modes[mode].frequency_hz / baseline.modes[mode].frequency_hz) - 0.5)
                    .abs()
                    < 1.0e-8
            );
        }
    }

    #[test]
    fn solver_rejects_incomplete_or_overlarge_dof_maps() {
        let mut assembly =
            assemble_released_joint_pin_truss(&sample_nodes(), &sample_members())
                .expect("valid assembly");
        assembly.free_dof_indices.pop();
        assert!(solve_released_joint_beam_modes(&assembly, 1).is_err());

        let assembly =
            assemble_released_joint_pin_truss(&sample_nodes(), &sample_members())
                .expect("valid assembly");
        assert!(solve_released_joint_beam_modes(&assembly, 0).is_err());
        assert!(solve_released_joint_beam_modes(&assembly, 7).is_err());
    }
}
