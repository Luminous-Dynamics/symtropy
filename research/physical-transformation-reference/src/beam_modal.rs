// SPDX-License-Identifier: AGPL-3.0-or-later
//! Modal diagnostics over assembled released-joint Euler-Bernoulli beam DOFs.
//!
//! This module reuses the generalized symmetric eigensolver verified by the
//! axial-bar tests. It preserves zero/mechanism modes rather than dropping them.

#![allow(clippy::needless_range_loop)] // Modal residuals and mass products couple matrix row/column indices.

use crate::Check;
use crate::beam_pin_assembly::BeamPinAssembly;
use crate::modal_bar::{
    Matrix, generalized_symmetric_eigenpairs, normalized_eigen_residual, quadratic_form,
};

const ZERO_MODE_RELATIVE_TOLERANCE: f64 = 1.0e-10;
const MAX_DENSE_MODAL_DOFS: usize = 128;

/// Explicit modal classification. Near-zero values remain visible as
/// mechanisms/rigid modes; negative values are flagged as unstable or invalid.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeamModeClass {
    Elastic,
    NearZeroOrMechanism,
    NegativeOrUnstable,
}

/// A generalized eigenmode over the entire global DOF space. Fixed DOFs have
/// zero components; the shape is mass-normalized over the free subspace.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalMode {
    pub eigenvalue_rad_s_squared: f64,
    /// None for non-positive or threshold-level eigenvalues; these remain present.
    pub frequency_hz: Option<f64>,
    pub class: BeamModeClass,
    pub shape_global_dofs: Vec<f64>,
    pub normalized_eigen_residual: f64,
    pub mass_norm_residual: f64,
}

/// Modal solution plus explicit model diagnostics. This does not qualify a
/// structure as physically complete or safe.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalResult {
    pub modes: Vec<BeamModalMode>,
    pub zero_mode_relative_tolerance: f64,
    pub maximum_normalized_eigen_residual: f64,
    pub maximum_mass_norm_residual: f64,
    pub maximum_off_diagonal_mass_orthogonality_residual: f64,
}

/// Solve the generalized eigenproblem on the assembly's unconstrained DOFs.
///
/// The returned mode count equals the free-DOF count. Small/negative modes
/// are retained and classified instead of silently filtered.
pub fn solve_beam_pin_assembly_modes(assembly: &BeamPinAssembly) -> Check<BeamModalResult> {
    if assembly.stiffness_global.len() > MAX_DENSE_MODAL_DOFS
        || assembly.mass_global.len() > MAX_DENSE_MODAL_DOFS
    {
        return Err(format!(
            "dense beam modal solve is capped at {MAX_DENSE_MODAL_DOFS} total DOFs; use a sparse eigensolver for larger meshes"
        ));
    }
    validate_modal_assembly(assembly)?;
    let stiffness_reduced = reduce_matrix(&assembly.stiffness_global, &assembly.free_dof_indices)?;
    let mass_reduced = reduce_matrix(&assembly.mass_global, &assembly.free_dof_indices)?;
    let eigenpairs = generalized_symmetric_eigenpairs(&stiffness_reduced, &mass_reduced)?;

    if eigenpairs.len() != assembly.free_dof_indices.len() {
        return Err("eigenpair count does not match the free-DOF count".to_owned());
    }
    let largest_magnitude = eigenpairs
        .iter()
        .map(|(eigenvalue, _)| eigenvalue.abs())
        .fold(0.0_f64, f64::max);
    if !largest_magnitude.is_finite() || largest_magnitude <= 0.0 {
        return Err("modal eigenproblem has no finite positive spectral scale".to_owned());
    }
    let zero_tolerance = largest_magnitude * ZERO_MODE_RELATIVE_TOLERANCE;

    let mut modes = Vec::with_capacity(eigenpairs.len());
    for (eigenvalue, shape_free) in eigenpairs {
        if shape_free.len() != assembly.free_dof_indices.len() {
            return Err("returned mode shape dimension differs from free-DOF map".to_owned());
        }
        let class = if eigenvalue < -zero_tolerance {
            BeamModeClass::NegativeOrUnstable
        } else if eigenvalue <= zero_tolerance {
            BeamModeClass::NearZeroOrMechanism
        } else {
            BeamModeClass::Elastic
        };
        let frequency_hz = if class == BeamModeClass::Elastic {
            let frequency = eigenvalue.sqrt() / (2.0 * std::f64::consts::PI);
            if !frequency.is_finite() || frequency <= 0.0 {
                return Err("elastic modal frequency is not finite and positive".to_owned());
            }
            Some(frequency)
        } else {
            None
        };

        let eigen_residual = normalized_eigen_residual(
            &stiffness_reduced,
            &mass_reduced,
            eigenvalue,
            &shape_free,
        )?;
        let mass_norm = quadratic_form(&mass_reduced, &shape_free)?;
        let mut shape_global = vec![0.0; assembly.stiffness_global.len()];
        for (free_index, global_index) in assembly.free_dof_indices.iter().enumerate() {
            shape_global[*global_index] = shape_free[free_index];
        }
        modes.push(BeamModalMode {
            eigenvalue_rad_s_squared: eigenvalue,
            frequency_hz,
            class,
            shape_global_dofs: shape_global,
            normalized_eigen_residual: eigen_residual,
            mass_norm_residual: (mass_norm - 1.0).abs(),
        });
    }

    let max_residual = modes
        .iter()
        .map(|mode| mode.normalized_eigen_residual)
        .fold(0.0_f64, f64::max);
    let max_mass_norm = modes
        .iter()
        .map(|mode| mode.mass_norm_residual)
        .fold(0.0_f64, f64::max);
    let mut max_orthogonality = 0.0_f64;
    for first in 0..modes.len() {
        for second in (first + 1)..modes.len() {
            let inner = mass_inner_product(
                &mass_reduced,
                &extract_free_shape(&modes[first], &assembly.free_dof_indices),
                &extract_free_shape(&modes[second], &assembly.free_dof_indices),
            )?;
            max_orthogonality = max_orthogonality.max(inner.abs());
        }
    }

    Ok(BeamModalResult {
        modes,
        zero_mode_relative_tolerance: ZERO_MODE_RELATIVE_TOLERANCE,
        maximum_normalized_eigen_residual: max_residual,
        maximum_mass_norm_residual: max_mass_norm,
        maximum_off_diagonal_mass_orthogonality_residual: max_orthogonality,
    })
}

fn validate_modal_assembly(assembly: &BeamPinAssembly) -> Check<()> {
    let size = assembly.stiffness_global.len();
    if size == 0 || assembly.mass_global.len() != size {
        return Err("global stiffness/mass dimensions are empty or inconsistent".to_owned());
    }
    if assembly.stiffness_global.iter().any(|row| row.len() != size)
        || assembly.mass_global.iter().any(|row| row.len() != size)
    {
        return Err("global stiffness and mass must be square".to_owned());
    }
    if assembly.dof_descriptors.len() != size
        || assembly
            .dof_descriptors
            .iter()
            .enumerate()
            .any(|(expected, descriptor)| descriptor.index != expected)
    {
        return Err("global DOF descriptors are incomplete or non-canonical".to_owned());
    }

    let mut seen = vec![false; size];
    for (kind, indices) in [
        ("fixed", assembly.fixed_dof_indices.as_slice()),
        ("free", assembly.free_dof_indices.as_slice()),
    ] {
        for index in indices {
            if *index >= size {
                return Err(format!("{kind} DOF index {index} is out of range"));
            }
            if seen[*index] {
                return Err(format!("DOF index {index} is duplicated or both fixed/free"));
            }
            seen[*index] = true;
        }
    }
    if seen.iter().any(|present| !present) {
        return Err("fixed/free DOF partition does not cover all global coordinates".to_owned());
    }
    if assembly.free_dof_indices.is_empty() {
        return Err("modal assembly has no free DOFs".to_owned());
    }

    for (name, matrix) in [
        ("global stiffness", &assembly.stiffness_global),
        ("global mass", &assembly.mass_global),
    ] {
        if matrix.iter().flatten().any(|value| !value.is_finite()) {
            return Err(format!("{name} contains a non-finite entry"));
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
    }
    Ok(())
}

fn reduce_matrix(matrix: &[Vec<f64>], free_indices: &[usize]) -> Check<Matrix> {
    if matrix.iter().any(|row| row.len() != matrix.len()) {
        return Err("global matrix must be square".to_owned());
    }
    if free_indices.iter().any(|index| *index >= matrix.len()) {
        return Err("free-DOF map contains an out-of-range index".to_owned());
    }
    Ok(free_indices
        .iter()
        .map(|row| {
            free_indices
                .iter()
                .map(|col| matrix[*row][*col])
                .collect::<Vec<_>>()
        })
        .collect())
}

fn extract_free_shape(mode: &BeamModalMode, free_indices: &[usize]) -> Vec<f64> {
    free_indices
        .iter()
        .map(|index| mode.shape_global_dofs[*index])
        .collect()
}

fn mass_inner_product(mass: &Matrix, left: &[f64], right: &[f64]) -> Check<f64> {
    if left.len() != mass.len()
        || right.len() != mass.len()
        || mass.iter().any(|row| row.len() != mass.len())
    {
        return Err("mass inner-product dimensions do not agree".to_owned());
    }
    let mut result = 0.0_f64;
    for row in 0..mass.len() {
        for col in 0..mass.len() {
            result += left[row] * mass[row][col] * right[col];
        }
    }
    if result.is_finite() {
        Ok(result)
    } else {
        Err("modal mass inner product is non-finite".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use crate::beam_pin_assembly::{BeamPinMember, BeamPinNode, assemble_released_joint_pin_truss};

    use super::{BeamModeClass, solve_beam_pin_assembly_modes};

    fn fixture() -> crate::beam_pin_assembly::BeamPinAssembly {
        fixture_with_scales(1.0, 1.0)
    }

    fn fixture_with_scales(
        youngs_modulus_scale: f64,
        density_scale: f64,
    ) -> crate::beam_pin_assembly::BeamPinAssembly {
        let nodes = vec![
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
        ];
        let members = vec![
            BeamPinMember {
                id: "AC".to_owned(),
                start_node_id: "A".to_owned(),
                end_node_id: "C".to_owned(),
                area_m2: 3.141592653589793e-4,
                youngs_modulus_pa: 210.0e9 * youngs_modulus_scale,
                density_kg_m3: 7800.0 * density_scale,
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
        ];
        assemble_released_joint_pin_truss(&nodes, &members).expect("beam pin assembly fixture")
    }

    #[test]
    fn independent_diagonal_generalized_eigenproblem_matches_exact_roots() {
        // K phi = lambda M phi, with a diagonal system whose exact roots are
        // lambda=[4,9]. This isolates eigenvalue ordering and mass normalization
        // from the frame-element assembly and any external solver.
        let stiffness = vec![vec![4.0, 0.0], vec![0.0, 36.0]];
        let mass = vec![vec![1.0, 0.0], vec![0.0, 4.0]];
        let eigenpairs = crate::modal_bar::generalized_symmetric_eigenpairs(
            &stiffness,
            &mass,
        )
        .expect("diagonal generalized eigenproblem");
        assert_eq!(eigenpairs.len(), 2);
        assert!((eigenpairs[0].0 - 4.0).abs() < 1.0e-12);
        assert!((eigenpairs[1].0 - 9.0).abs() < 1.0e-12);
        assert!(eigenpairs[0].0 < eigenpairs[1].0);

        for (eigenvalue, shape) in &eigenpairs {
            assert!(crate::modal_bar::normalized_eigen_residual(
                &stiffness,
                &mass,
                *eigenvalue,
                shape
            )
            .expect("finite residual")
                < 1.0e-12);
            assert!(
                (crate::modal_bar::quadratic_form(&mass, shape).expect("modal norm") - 1.0).abs()
                    < 1.0e-12
            );
        }
    }

    #[test]
    fn modes_preserve_all_free_dofs_and_have_small_residuals() {
        let assembly = fixture();
        let result = solve_beam_pin_assembly_modes(&assembly).expect("modal result");
        assert_eq!(result.modes.len(), assembly.free_dof_indices.len());
        assert!(
            result
                .modes
                .iter()
                .all(|mode| mode.class == BeamModeClass::Elastic)
        );
        assert!(result.modes.windows(2).all(|pair| {
            pair[0].frequency_hz.expect("elastic mode")
                < pair[1].frequency_hz.expect("elastic mode")
        }));
        assert!(result.maximum_normalized_eigen_residual < 1.0e-10);
        assert!(result.maximum_mass_norm_residual < 1.0e-10);
        assert!(result.maximum_off_diagonal_mass_orthogonality_residual < 1.0e-10);
        for mode in result.modes {
            assert!(mode.shape_global_dofs.iter().all(|value| value.is_finite()));
            for fixed_dof in &assembly.fixed_dof_indices {
                assert_eq!(mode.shape_global_dofs[*fixed_dof], 0.0);
            }
        }
    }

    #[test]
    fn mode_frequencies_obey_material_and_density_scaling() {
        let baseline = solve_beam_pin_assembly_modes(&fixture()).expect("baseline modes");
        let stiff = solve_beam_pin_assembly_modes(&fixture_with_scales(4.0, 1.0))
            .expect("four-times modulus modes");
        let dense = solve_beam_pin_assembly_modes(&fixture_with_scales(1.0, 4.0))
            .expect("four-times density modes");
        assert_eq!(baseline.modes.len(), stiff.modes.len());
        assert_eq!(baseline.modes.len(), dense.modes.len());

        for index in 0..baseline.modes.len() {
            let base_frequency = baseline.modes[index].frequency_hz.expect("elastic baseline");
            let stiff_frequency = stiff.modes[index].frequency_hz.expect("elastic stiff");
            let dense_frequency = dense.modes[index].frequency_hz.expect("elastic dense");
            assert!(((stiff_frequency / base_frequency) - 2.0).abs() < 1.0e-8);
            assert!(((dense_frequency / base_frequency) - 0.5).abs() < 1.0e-8);
        }
    }

    #[test]
    fn negative_eigenvalues_are_reported_as_unstable_not_dropped() {
        let mut assembly = fixture();
        let negative_global = *assembly.free_dof_indices.last().expect("free DOF");
        for row in 0..assembly.stiffness_global.len() {
            assembly.stiffness_global[row][negative_global] = 0.0;
            assembly.stiffness_global[negative_global][row] = 0.0;
        }
        assembly.stiffness_global[negative_global][negative_global] = -1.0e30;

        let result = solve_beam_pin_assembly_modes(&assembly).expect("classified negative mode");
        assert_eq!(result.modes.len(), assembly.free_dof_indices.len());
        assert!(result.modes.iter().any(|mode| {
            mode.class == BeamModeClass::NegativeOrUnstable && mode.frequency_hz.is_none()
        }));
    }

    #[test]
    fn malformed_dof_partition_fails_closed() {
        let mut assembly = fixture();
        assembly
            .free_dof_indices
            .push(assembly.fixed_dof_indices[0]);
        assert!(solve_beam_pin_assembly_modes(&assembly).is_err());

        let mut assembly = fixture();
        assembly.dof_descriptors.pop();
        assert!(solve_beam_pin_assembly_modes(&assembly).is_err());
    }

    #[test]
    fn dense_modal_size_cap_fails_closed_before_factorization() {
        let mut assembly = fixture();
        assembly.stiffness_global = vec![vec![0.0; MAX_DENSE_MODAL_DOFS + 1]; MAX_DENSE_MODAL_DOFS + 1];
        assert!(solve_beam_pin_assembly_modes(&assembly).is_err());
    }

    #[test]
    fn zero_eigenvalues_are_reported_not_silently_dropped() {
        let mut assembly = fixture();
        let free = assembly.free_dof_indices.clone();
        let last_global = *free.last().expect("free DOF");
        // Introduce a synthetic mechanism in a cloned matrix by zeroing one
        // free row/column; the remaining DOFs retain the positive spectral scale.
        for row in 0..assembly.stiffness_global.len() {
            assembly.stiffness_global[row][last_global] = 0.0;
            assembly.stiffness_global[last_global][row] = 0.0;
        }
        let result = solve_beam_pin_assembly_modes(&assembly).expect("classified mechanism");
        assert_eq!(result.modes.len(), assembly.free_dof_indices.len());
        assert!(result.modes.iter().any(|mode| {
            mode.class == BeamModeClass::NearZeroOrMechanism && mode.frequency_hz.is_none()
        }));
    }
}
