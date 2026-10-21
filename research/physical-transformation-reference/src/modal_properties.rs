// SPDX-License-Identifier: AGPL-3.0-or-later
//! Effective modal mass and participation factors for global X/Y excitation.
//!
//! These quantities complement eigenfrequency and residual checks: they show
//! how strongly each mode couples to a uniform translational influence in X/Y.
//! The routine uses the full assembled mass matrix for total-mass denominators
//! and global mode vectors with restrained DOFs already set to zero.

use crate::Check;
use crate::beam_modal::{BeamModalMode, BeamModalResult};
use crate::beam_pin_assembly::{BeamPinAssembly, BeamPinDofKind};
use crate::modal_bar::{Matrix, quadratic_form};

/// Participation measures for one eigenmode. The effective modal masses are
/// non-negative; the factors themselves may have either sign depending on the
/// eigenvector's sign convention.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModeParticipation {
    pub mode_index: usize,
    pub class: crate::beam_modal::BeamModeClass,
    pub frequency_hz: Option<f64>,
    pub generalized_mass_kg: f64,
    pub participation_factor_x: f64,
    pub participation_factor_y: f64,
    pub effective_modal_mass_x_kg: f64,
    pub effective_modal_mass_y_kg: f64,
    pub effective_modal_mass_ratio_x: f64,
    pub effective_modal_mass_ratio_y: f64,
    pub cumulative_effective_modal_mass_ratio_x: f64,
    pub cumulative_effective_modal_mass_ratio_y: f64,
}

/// Physical modal participation summaries for the assembled mass matrix.
///
/// Total masses include translational mass at both restrained and free nodes.
/// Effective modal mass uses each mass-normalized global eigenvector, whose
/// restrained DOF components are zero. The sum of participating ratios may
/// therefore be below one; report that shortfall rather than renormalizing it
/// away.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamModalParticipationProperties {
    pub total_translational_mass_x_kg: f64,
    pub total_translational_mass_y_kg: f64,
    pub modes: Vec<BeamModeParticipation>,
}

/// Compute translational participation factors and effective modal masses.
///
/// For influence vector r and eigenmode phi:
/// Gamma = (phi^T M r)/(phi^T M phi), and
/// M_eff = (phi^T M r)^2/(phi^T M phi).
///
/// One global influence vector is one for each node translation in the named
/// direction, including restrained node translations, and zero for the other
/// direction and private member-end/interior rotations.
pub fn beam_modal_participation_properties(
    assembly: &BeamPinAssembly,
    modal_result: &BeamModalResult,
) -> Check<BeamModalParticipationProperties> {
    let dof_count = assembly.stiffness_global.len();
    if dof_count == 0
        || assembly.mass_global.len() != dof_count
        || assembly.mass_global.iter().any(|row| row.len() != dof_count)
    {
        return Err("global mass matrix must be non-empty and square".to_owned());
    }
    if assembly.dof_descriptors.len() != dof_count {
        return Err("global DOF descriptors do not match the mass matrix size".to_owned());
    }
    if modal_result.modes.is_empty() {
        return Err("at least one modal solution is required".to_owned());
    }
    if modal_result.modes.iter().any(|mode| {
        mode.shape_global_dofs.len() != dof_count
            || !mode.eigenvalue_rad_s_squared.is_finite()
            || mode
                .shape_global_dofs
                .iter()
                .any(|value| !value.is_finite())
            || mode
                .frequency_hz
                .is_some_and(|frequency| !frequency.is_finite() || frequency <= 0.0)
    }) {
        return Err("modal vectors or modal values are invalid for the assembled DOFs".to_owned());
    }
    if modal_result
        .modes
        .windows(2)
        .any(|pair| pair[0].eigenvalue_rad_s_squared > pair[1].eigenvalue_rad_s_squared)
    {
        return Err("modal modes must be ordered by nondecreasing eigenvalue".to_owned());
    }
    if assembly.mass_global.iter().flatten().any(|value| !value.is_finite()) {
        return Err("global mass matrix contains a non-finite entry".to_owned());
    }

    let mut seen = vec![false; dof_count];
    let mut influence_x = vec![0.0; dof_count];
    let mut influence_y = vec![0.0; dof_count];
    for descriptor in &assembly.dof_descriptors {
        if descriptor.index >= dof_count || seen[descriptor.index] {
            return Err("global DOF indices must be unique and cover the matrix".to_owned());
        }
        seen[descriptor.index] = true;
        match descriptor.kind {
            BeamPinDofKind::NodeTranslationX { .. } => influence_x[descriptor.index] = 1.0,
            BeamPinDofKind::NodeTranslationY { .. } => influence_y[descriptor.index] = 1.0,
            BeamPinDofKind::MemberEndRotation { .. }
            | BeamPinDofKind::MemberInteriorRotation { .. } => {}
        }
    }
    if seen.iter().any(|present| !present) {
        return Err("global DOF descriptors leave an index unclassified".to_owned());
    }

    let total_mass_x = quadratic_form(&assembly.mass_global, &influence_x)?;
    let total_mass_y = quadratic_form(&assembly.mass_global, &influence_y)?;
    let mass_influence_x = matrix_vector_product(&assembly.mass_global, &influence_x)?;
    let mass_influence_y = matrix_vector_product(&assembly.mass_global, &influence_y)?;

    let mut cumulative_x = 0.0;
    let mut cumulative_y = 0.0;
    let mut modes = Vec::with_capacity(modal_result.modes.len());
    for (mode_index, mode) in modal_result.modes.iter().enumerate() {
        let generalized_mass = quadratic_form(&assembly.mass_global, &mode.shape_global_dofs)?;
        let projection_x = dot(&mode.shape_global_dofs, &mass_influence_x)?;
        let projection_y = dot(&mode.shape_global_dofs, &mass_influence_y)?;
        let participation_factor_x = projection_x / generalized_mass;
        let participation_factor_y = projection_y / generalized_mass;
        let effective_mass_x = projection_x * projection_x / generalized_mass;
        let effective_mass_y = projection_y * projection_y / generalized_mass;
        let ratio_x = effective_mass_x / total_mass_x;
        let ratio_y = effective_mass_y / total_mass_y;
        cumulative_x += ratio_x;
        cumulative_y += ratio_y;
        let values = [
            generalized_mass,
            participation_factor_x,
            participation_factor_y,
            effective_mass_x,
            effective_mass_y,
            ratio_x,
            ratio_y,
            cumulative_x,
            cumulative_y,
        ];
        if values.iter().any(|value| !value.is_finite())
            || effective_mass_x < 0.0
            || effective_mass_y < 0.0
            || ratio_x < 0.0
            || ratio_y < 0.0
        {
            return Err(format!("modal participation output is invalid for mode {mode_index}"));
        }

        modes.push(BeamModeParticipation {
            mode_index,
            class: mode.class,
            frequency_hz: mode.frequency_hz,
            generalized_mass_kg: generalized_mass,
            participation_factor_x,
            participation_factor_y,
            effective_modal_mass_x_kg: effective_mass_x,
            effective_modal_mass_y_kg: effective_mass_y,
            effective_modal_mass_ratio_x: ratio_x,
            effective_modal_mass_ratio_y: ratio_y,
            cumulative_effective_mass_ratio_x: cumulative_x,
            cumulative_effective_mass_ratio_y: cumulative_y,
        });
    }

    Ok(BeamModalParticipationProperties {
        total_translational_mass_x_kg: total_mass_x,
        total_translational_mass_y_kg: total_mass_y,
        modes,
    })
}

fn matrix_vector_product(matrix: &Matrix, vector: &[f64]) -> Check<Vec<f64>> {
    if matrix.len() != vector.len() || matrix.iter().any(|row| row.len() != vector.len()) {
        return Err("matrix/vector dimensions do not agree".to_owned());
    }
    let product = matrix
        .iter()
        .map(|row| row.iter().zip(vector).map(|(left, right)| left * right).sum::<f64>())
        .collect::<Vec<_>>();
    if product.iter().all(|value| value.is_finite()) {
        Ok(product)
    } else {
        Err("mass/influence product contains a non-finite value".to_owned())
    }
}

fn dot(left: &[f64], right: &[f64]) -> Check<f64> {
    if left.len() != right.len() {
        return Err("dot-product vectors do not have matching dimensions".to_owned());
    }
    let result = left.iter().zip(right).map(|(a, b)| a * b).sum::<f64>();
    if result.is_finite() {
        Ok(result)
    } else {
        Err("modal participation projection is non-finite".to_owned())
    }
}

#[cfg(test)]
mod tests {
    use super::beam_modal_participation_properties;
    use crate::beam_modal::{BeamModalMode, BeamModalResult, BeamModeClass};
    use crate::beam_pin_assembly::{
        BeamPinAssembly, BeamPinDofDescriptor, BeamPinDofKind,
    };

    fn descriptor(index: usize, kind: BeamPinDofKind) -> BeamPinDofDescriptor {
        BeamPinDofDescriptor { index, kind }
    }

    fn assembly(mass: Vec<Vec<f64>>) -> BeamPinAssembly {
        BeamPinAssembly {
            ordered_node_ids: vec!["A".to_owned()],
            ordered_member_ids: Vec::new(),
            dof_descriptors: vec![
                descriptor(
                    0,
                    BeamPinDofKind::NodeTranslationX {
                        node_id: "A".to_owned(),
                    },
                ),
                descriptor(
                    1,
                    BeamPinDofKind::NodeTranslationY {
                        node_id: "A".to_owned(),
                    },
                ),
            ],
            stiffness_global: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            mass_global: mass.clone(),
            translational_mass_global: mass,
            rotary_inertia_mass_global: vec![vec![0.0, 0.0], vec![0.0, 0.0]],
            fixed_dof_indices: Vec::new(),
            free_dof_indices: vec![0, 1],
        }
    }

    fn result() -> BeamModalResult {
        BeamModalResult {
            modes: vec![
                BeamModalMode {
                    eigenvalue_rad_s_squared: 1.0,
                    frequency_hz: Some(1.0 / (2.0 * std::f64::consts::PI)),
                    class: BeamModeClass::Elastic,
                    shape_global_dofs: vec![1.0 / 2.0_f64.sqrt(), 0.0],
                    normalized_eigen_residual: 0.0,
                    mass_norm_residual: 0.0,
                },
                BeamModalMode {
                    eigenvalue_rad_s_squared: 2.0,
                    frequency_hz: Some(2.0_f64.sqrt() / (2.0 * std::f64::consts::PI)),
                    class: BeamModeClass::Elastic,
                    shape_global_dofs: vec![0.0, 1.0 / 3.0_f64.sqrt()],
                    normalized_eigen_residual: 0.0,
                    mass_norm_residual: 0.0,
                },
            ],
            zero_mode_relative_tolerance: 1.0e-10,
            maximum_normalized_eigen_residual: 0.0,
            maximum_mass_norm_residual: 0.0,
            maximum_off_diagonal_mass_orthogonality_residual: 0.0,
        }
    }

    #[test]
    fn orthogonal_directional_modes_report_complete_effective_mass() {
        let assembly = assembly(vec![vec![2.0, 0.0], vec![0.0, 3.0]]);
        let properties =
            beam_modal_participation_properties(&assembly, &result()).expect("participation");

        assert!((properties.total_translational_mass_x_kg - 2.0).abs() < 1.0e-12);
        assert!((properties.total_translational_mass_y_kg - 3.0).abs() < 1.0e-12);
        assert_eq!(properties.modes.len(), 2);

        let x_mode = &properties.modes[0];
        assert!((x_mode.generalized_mass_kg - 1.0).abs() < 1.0e-12);
        assert!((x_mode.participation_factor_x - 2.0_f64.sqrt()).abs() < 1.0e-12);
        assert!((x_mode.effective_modal_mass_x_kg - 2.0).abs() < 1.0e-12);
        assert!((x_mode.effective_modal_mass_ratio_x - 1.0).abs() < 1.0e-12);
        assert!(x_mode.effective_modal_mass_ratio_y.abs() < 1.0e-12);

        let y_mode = &properties.modes[1];
        assert!((y_mode.participation_factor_y - 3.0_f64.sqrt()).abs() < 1.0e-12);
        assert!((y_mode.effective_modal_mass_y_kg - 3.0).abs() < 1.0e-12);
        assert!((y_mode.effective_modal_mass_ratio_y - 1.0).abs() < 1.0e-12);
        assert!((y_mode.cumulative_effective_mass_ratio_x - 1.0).abs() < 1.0e-12);
        assert!((y_mode.cumulative_effective_mass_ratio_y - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn participation_uses_total_mass_including_restrained_nodes() {
        let assembly = BeamPinAssembly {
            ordered_node_ids: vec!["fixed".to_owned(), "free".to_owned()],
            ordered_member_ids: Vec::new(),
            dof_descriptors: vec![
                descriptor(
                    0,
                    BeamPinDofKind::NodeTranslationX {
                        node_id: "fixed".to_owned(),
                    },
                ),
                descriptor(
                    1,
                    BeamPinDofKind::NodeTranslationX {
                        node_id: "free".to_owned(),
                    },
                ),
                descriptor(
                    2,
                    BeamPinDofKind::NodeTranslationY {
                        node_id: "fixed".to_owned(),
                    },
                ),
                descriptor(
                    3,
                    BeamPinDofKind::NodeTranslationY {
                        node_id: "free".to_owned(),
                    },
                ),
            ],
            stiffness_global: vec![vec![1.0; 4]; 4],
            mass_global: vec![
                vec![2.0, 0.0, 0.0, 0.0],
                vec![0.0, 2.0, 0.0, 0.0],
                vec![0.0, 0.0, 3.0, 0.0],
                vec![0.0, 0.0, 0.0, 3.0],
            ],
            translational_mass_global: vec![
                vec![2.0, 0.0, 0.0, 0.0],
                vec![0.0, 2.0, 0.0, 0.0],
                vec![0.0, 0.0, 3.0, 0.0],
                vec![0.0, 0.0, 0.0, 3.0],
            ],
            rotary_inertia_mass_global: vec![vec![0.0; 4]; 4],
            fixed_dof_indices: vec![0, 2],
            free_dof_indices: vec![1, 3],
        };
        let mut modal_result = result();
        modal_result.modes[0].shape_global_dofs = vec![0.0, 1.0 / 2.0_f64.sqrt(), 0.0, 0.0];
        modal_result.modes[1].shape_global_dofs = vec![0.0, 0.0, 0.0, 1.0 / 3.0_f64.sqrt()];
        let properties = beam_modal_participation_properties(&assembly, &modal_result)
            .expect("participation with support mass");
        assert!((properties.total_translational_mass_x_kg - 4.0).abs() < 1.0e-12);
        assert!((properties.total_translational_mass_y_kg - 6.0).abs() < 1.0e-12);
        assert!((properties.modes[0].effective_modal_mass_ratio_x - 0.5).abs() < 1.0e-12);
        assert!((properties.modes[1].effective_modal_mass_ratio_y - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn malformed_or_inconsistent_modal_inputs_fail_closed() {
        let assembly = assembly(vec![vec![2.0, 0.0], vec![0.0, 3.0]]);
        let mut malformed = result();
        malformed.modes[0].shape_global_dofs.pop();
        assert!(beam_modal_participation_properties(&assembly, &malformed).is_err());

        let mut unsorted = result();
        unsorted.modes.reverse();
        assert!(beam_modal_participation_properties(&assembly, &unsorted).is_err());

        let mut nonfinite = assembly.clone();
        nonfinite.mass_global[0][0] = f64::NAN;
        assert!(beam_modal_participation_properties(&nonfinite, &result()).is_err());
    }
}
