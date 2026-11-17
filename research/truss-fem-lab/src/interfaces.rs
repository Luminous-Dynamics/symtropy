//! Axial end-spring compliance for modular member-to-hub interfaces.
//!
//! Each member has one translational spring at each end, acting only along
//! the member axis. Spring stiffness is supplied in N/m. This captures
//! first-order axial connector slip but not rotation, frictional contact,
//! bolt preload, gaps, backlash, nonlinear bearing, or multi-axis behavior.

use std::error::Error;
use std::fmt;

use symtropy_freeflow_structure_lab::Vec3;

use crate::{
    validate_inputs, solve_dense, AnalysisError, BoundaryConditions, TrussMaterial, TrussModel,
    MAX_FREE_DOFS, MAX_GLOBAL_DOFS,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct JointEndStiffness {
    pub from_axial_n_per_m: f64,
    pub to_axial_n_per_m: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CompliantMemberResult {
    pub from: usize,
    pub to: usize,
    pub length_m: f64,
    pub effective_axial_stiffness_n_per_m: f64,
    pub axial_force_n: f64,
    pub axial_stress_pa: f64,
    pub total_nodal_extension_m: f64,
    pub bar_extension_m: f64,
    pub from_joint_slip_m: f64,
    pub to_joint_slip_m: f64,
    pub bar_strain_energy_j: f64,
    pub joint_strain_energy_j: f64,
    pub total_strain_energy_j: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompliantAnalysisResult {
    pub node_displacements_m: Vec<[f64; 3]>,
    pub support_reactions_n: Vec<[f64; 3]>,
    pub members: Vec<CompliantMemberResult>,
    pub total_strain_energy_j: f64,
    pub max_free_dof_equilibrium_residual_n: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum InterfaceAnalysisError {
    JointPropertyCountMismatch { expected: usize, actual: usize },
    InvalidJointStiffness { member_index: usize, end: &'static str },
    NonFiniteResult { member_index: Option<usize> },
    Analysis(AnalysisError),
}

impl fmt::Display for InterfaceAnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::JointPropertyCountMismatch { expected, actual } => write!(
                f,
                "joint stiffness count {actual} does not match member count {expected}"
            ),
            Self::InvalidJointStiffness { member_index, end } => write!(
                f,
                "member {member_index} has invalid {end}-end axial joint stiffness"
            ),
            Self::NonFiniteResult { member_index } => match member_index {
                Some(index) => write!(f, "member {index} produced a non-finite compliant result"),
                None => write!(f, "compliant truss analysis produced a non-finite global result"),
            },
            Self::Analysis(error) => write!(f, "base truss analysis validation failed: {error}"),
        }
    }
}
impl Error for InterfaceAnalysisError {}

/// Solve a pin-jointed truss with an axial translational spring in series at
/// both ends of each member.
///
/// The element compliance is:
/// L/(E*A) + 1/k_from + 1/k_to.
/// Member stress is computed from member-bar strain only; connector slip is
/// reported separately. This prevents joint displacement from being
/// misattributed to material strain.
pub fn analyze_with_joint_end_stiffness(
    model: &TrussModel,
    material: TrussMaterial,
    boundary: &BoundaryConditions,
    joint_stiffness: &[JointEndStiffness],
) -> Result<CompliantAnalysisResult, InterfaceAnalysisError> {
    validate_inputs(model, material, boundary).map_err(InterfaceAnalysisError::Analysis)?;
    if joint_stiffness.len() != model.members.len() {
        return Err(InterfaceAnalysisError::JointPropertyCountMismatch {
            expected: model.members.len(),
            actual: joint_stiffness.len(),
        });
    }

    for (member_index, stiffness) in joint_stiffness.iter().enumerate() {
        if !stiffness.from_axial_n_per_m.is_finite()
            || stiffness.from_axial_n_per_m <= 0.0
        {
            return Err(InterfaceAnalysisError::InvalidJointStiffness {
                member_index,
                end: "from",
            });
        }
        if !stiffness.to_axial_n_per_m.is_finite()
            || stiffness.to_axial_n_per_m <= 0.0
        {
            return Err(InterfaceAnalysisError::InvalidJointStiffness {
                member_index,
                end: "to",
            });
        }
    }

    let dof_count = model
        .nodes
        .len()
        .checked_mul(3)
        .ok_or(AnalysisError::ModelTooLarge)
        .map_err(InterfaceAnalysisError::Analysis)?;
    if dof_count > MAX_GLOBAL_DOFS {
        return Err(InterfaceAnalysisError::Analysis(AnalysisError::ModelTooLarge));
    }

    let mut stiffness_matrix = vec![vec![0.0_f64; dof_count]; dof_count];
    let mut effective_stiffnesses = Vec::with_capacity(model.members.len());

    for (member_index, member) in model.members.iter().enumerate() {
        let from = model.nodes[member.from];
        let to = model.nodes[member.to];
        let length = from.distance(to);
        let bar_compliance = length
            / (material.youngs_modulus_pa * material.cross_section_area_m2);
        let end_stiffness = joint_stiffness[member_index];
        let total_compliance = bar_compliance
            + 1.0 / end_stiffness.from_axial_n_per_m
            + 1.0 / end_stiffness.to_axial_n_per_m;
        let effective_axial_stiffness = 1.0 / total_compliance;

        if !bar_compliance.is_finite()
            || bar_compliance <= 0.0
            || !total_compliance.is_finite()
            || total_compliance <= 0.0
            || !effective_axial_stiffness.is_finite()
            || effective_axial_stiffness <= 0.0
        {
            return Err(InterfaceAnalysisError::NonFiniteResult {
                member_index: Some(member_index),
            });
        }

        assemble_compliant_member(
            &mut stiffness_matrix,
            &model.nodes,
            *member,
            effective_axial_stiffness,
        )
        .map_err(InterfaceAnalysisError::Analysis)?;
        effective_stiffnesses.push(effective_axial_stiffness);
    }

    let mut free_dofs = Vec::new();
    for (node, fixed) in boundary.fixed_dofs.iter().enumerate() {
        for (axis, is_fixed) in fixed.iter().enumerate() {
            if !is_fixed {
                free_dofs.push(3 * node + axis);
            }
        }
    }
    if free_dofs.len() > MAX_FREE_DOFS {
        return Err(InterfaceAnalysisError::Analysis(AnalysisError::ModelTooLarge));
    }

    let mut reduced = vec![vec![0.0; free_dofs.len()]; free_dofs.len()];
    let mut reduced_force = vec![0.0; free_dofs.len()];
    for (row_index, &global_row) in free_dofs.iter().enumerate() {
        reduced_force[row_index] =
            boundary.nodal_forces_n[global_row / 3][global_row % 3];
        for (column_index, &global_column) in free_dofs.iter().enumerate() {
            reduced[row_index][column_index] =
                stiffness_matrix[global_row][global_column];
        }
    }

    let free_displacements = if free_dofs.is_empty() {
        Vec::new()
    } else {
        solve_dense(reduced, reduced_force).map_err(InterfaceAnalysisError::Analysis)?
    };

    let mut displacement_flat = vec![0.0; dof_count];
    for (&global_dof, value) in free_dofs.iter().zip(free_displacements.iter()) {
        displacement_flat[global_dof] = *value;
    }
    if !displacement_flat.iter().all(|value| value.is_finite()) {
        return Err(InterfaceAnalysisError::NonFiniteResult { member_index: None });
    }

    let mut reaction_flat = vec![0.0; dof_count];
    let mut max_free_dof_residual = 0.0_f64;
    for row in 0..dof_count {
        let internal_force: f64 = stiffness_matrix[row]
            .iter()
            .zip(displacement_flat.iter())
            .map(|(k, u)| k * u)
            .sum();
        let applied = boundary.nodal_forces_n[row / 3][row % 3];
        let residual = internal_force - applied;
        reaction_flat[row] = residual;
        if !boundary.fixed_dofs[row / 3][row % 3] {
            max_free_dof_residual = max_free_dof_residual.max(residual.abs());
        }
    }

    let node_displacements_m = displacement_flat
        .chunks_exact(3)
        .map(|v| [v[0], v[1], v[2]])
        .collect::<Vec<_>>();
    let support_reactions_n = reaction_flat
        .chunks_exact(3)
        .map(|v| [v[0], v[1], v[2]])
        .collect::<Vec<_>>();

    let mut members = Vec::with_capacity(model.members.len());
    let mut total_strain_energy_j = 0.0;
    for (member_index, member) in model.members.iter().enumerate() {
        let from = model.nodes[member.from];
        let to = model.nodes[member.to];
        let delta = to - from;
        let length = delta.norm();
        let direction = delta.scale(1.0 / length);
        let displacement_from = node_displacements_m[member.from];
        let displacement_to = node_displacements_m[member.to];
        let relative = [
            displacement_to[0] - displacement_from[0],
            displacement_to[1] - displacement_from[1],
            displacement_to[2] - displacement_from[2],
        ];
        let total_extension =
            relative[0] * direction.x + relative[1] * direction.y + relative[2] * direction.z;
        let axial_force = effective_stiffnesses[member_index] * total_extension;
        let bar_extension = axial_force * length
            / (material.youngs_modulus_pa * material.cross_section_area_m2);
        let from_joint_slip =
            axial_force / joint_stiffness[member_index].from_axial_n_per_m;
        let to_joint_slip = axial_force / joint_stiffness[member_index].to_axial_n_per_m;
        let axial_stress = axial_force / material.cross_section_area_m2;
        let bar_energy = 0.5 * axial_force * bar_extension;
        let joint_energy =
            0.5 * axial_force * (from_joint_slip + to_joint_slip);
        let total_energy = 0.5 * axial_force * total_extension;

        if !length.is_finite()
            || !total_extension.is_finite()
            || !axial_force.is_finite()
            || !bar_extension.is_finite()
            || !from_joint_slip.is_finite()
            || !to_joint_slip.is_finite()
            || !axial_stress.is_finite()
            || !bar_energy.is_finite()
            || !joint_energy.is_finite()
            || !total_energy.is_finite()
        {
            return Err(InterfaceAnalysisError::NonFiniteResult {
                member_index: Some(member_index),
            });
        }
        total_strain_energy_j += total_energy;
        members.push(CompliantMemberResult {
            from: member.from,
            to: member.to,
            length_m: length,
            effective_axial_stiffness_n_per_m: effective_stiffnesses[member_index],
            axial_force_n: axial_force,
            axial_stress_pa: axial_stress,
            total_nodal_extension_m: total_extension,
            bar_extension_m: bar_extension,
            from_joint_slip_m: from_joint_slip,
            to_joint_slip_m: to_joint_slip,
            bar_strain_energy_j: bar_energy,
            joint_strain_energy_j: joint_energy,
            total_strain_energy_j: total_energy,
        });
    }

    if !total_strain_energy_j.is_finite() || !max_free_dof_residual.is_finite() {
        return Err(InterfaceAnalysisError::NonFiniteResult { member_index: None });
    }

    Ok(CompliantAnalysisResult {
        node_displacements_m,
        support_reactions_n,
        members,
        total_strain_energy_j,
        max_free_dof_equilibrium_residual_n: max_free_dof_residual,
    })
}

fn assemble_compliant_member(
    global: &mut [Vec<f64>],
    nodes: &[Vec3],
    member: crate::TrussMember,
    axial_stiffness: f64,
) -> Result<(), AnalysisError> {
    let delta = nodes[member.to] - nodes[member.from];
    let length = delta.norm();
    if !length.is_finite() || length <= f64::MIN_POSITIVE {
        return Err(AnalysisError::InvalidGeometry);
    }
    let direction = delta.scale(1.0 / length);
    let n = [direction.x, direction.y, direction.z];

    for row_axis in 0..3 {
        for column_axis in 0..3 {
            let value = axial_stiffness * n[row_axis] * n[column_axis];
            let from_row = 3 * member.from + row_axis;
            let from_column = 3 * member.from + column_axis;
            let to_row = 3 * member.to + row_axis;
            let to_column = 3 * member.to + column_axis;

            global[from_row][from_column] += value;
            global[from_row][to_column] -= value;
            global[to_row][from_column] -= value;
            global[to_row][to_column] += value;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundaryConditions, TrussMember, TrussMaterial, TrussModel};
    use symtropy_freeflow_structure_lab::Vec3;

    fn model_and_boundary(force_n: f64) -> (TrussModel, BoundaryConditions, TrussMaterial) {
        let model = TrussModel {
            nodes: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)],
            members: vec![TrussMember { from: 0, to: 1 }],
        };
        let material = TrussMaterial {
            youngs_modulus_pa: 200.0e9,
            cross_section_area_m2: 0.01,
        };
        let mut boundary = BoundaryConditions::new(2);
        boundary.fix_node(0).unwrap();
        boundary.fix_dof(1, 1).unwrap();
        boundary.fix_dof(1, 2).unwrap();
        boundary.add_force(1, [force_n, 0.0, 0.0]).unwrap();
        (model, boundary, material)
    }

    #[test]
    fn very_stiff_end_springs_approach_the_rigid_joint_reference() {
        let (model, boundary, material) = model_and_boundary(1_000.0);
        let rigid = model.analyze(material, &boundary).unwrap();
        let compliant = analyze_with_joint_end_stiffness(
            &model,
            material,
            &boundary,
            &[JointEndStiffness {
                from_axial_n_per_m: 1.0e18,
                to_axial_n_per_m: 1.0e18,
            }],
        )
        .unwrap();
        let rigid_extension = rigid.node_displacements_m[1][0];
        let compliant_extension = compliant.node_displacements_m[1][0];
        assert!((compliant_extension - rigid_extension).abs() / rigid_extension < 1e-8);
    }

    #[test]
    fn finite_end_springs_separate_bar_strain_from_connector_slip() {
        let (model, boundary, material) = model_and_boundary(1_000.0);
        let result = analyze_with_joint_end_stiffness(
            &model,
            material,
            &boundary,
            &[JointEndStiffness {
                from_axial_n_per_m: 1.0e9,
                to_axial_n_per_m: 1.0e9,
            }],
        )
        .unwrap();
        let member = result.members[0];

        assert!((member.axial_force_n - 1_000.0).abs() < 1e-7);
        assert!((member.bar_extension_m - 1.0e-6).abs() < 1e-12);
        assert!((member.from_joint_slip_m - 1.0e-6).abs() < 1e-12);
        assert!((member.to_joint_slip_m - 1.0e-6).abs() < 1e-12);
        assert!((member.total_nodal_extension_m - 3.0e-6).abs() < 1e-12);
        assert!(
            (member.total_strain_energy_j
                - member.bar_strain_energy_j
                - member.joint_strain_energy_j)
                .abs()
                < 1e-12
        );
    }

    #[test]
    fn compression_joint_slip_has_the_same_sign_as_axial_force() {
        let (model, boundary, material) = model_and_boundary(-1_000.0);
        let result = analyze_with_joint_end_stiffness(
            &model,
            material,
            &boundary,
            &[JointEndStiffness {
                from_axial_n_per_m: 1.0e9,
                to_axial_n_per_m: 1.0e9,
            }],
        )
        .unwrap();
        assert!(result.members[0].axial_force_n < 0.0);
        assert!(result.members[0].from_joint_slip_m < 0.0);
        assert!(result.members[0].to_joint_slip_m < 0.0);
    }

    #[test]
    fn invalid_joint_stiffness_and_property_count_fail_closed() {
        let (model, boundary, material) = model_and_boundary(1_000.0);
        assert!(matches!(
            analyze_with_joint_end_stiffness(&model, material, &boundary, &[]),
            Err(InterfaceAnalysisError::JointPropertyCountMismatch { .. })
        ));
        assert!(matches!(
            analyze_with_joint_end_stiffness(
                &model,
                material,
                &boundary,
                &[JointEndStiffness {
                    from_axial_n_per_m: f64::NAN,
                    to_axial_n_per_m: 1.0e9,
                }]
            ),
            Err(InterfaceAnalysisError::InvalidJointStiffness { .. })
        ));
    }
}
