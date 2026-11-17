//! Small 3D linear Euler-Bernoulli frame assembly and static solver.
//!
//! This module assembles the isolated 12-DOF beam elements into a global
//! six-DOF-per-node stiffness system. It is a bounded reference solver,
//! not a production FE implementation.
//!
//! Included: axial, torsional and two-plane bending stiffness.
//! Excluded: Timoshenko shear, geometric stiffness, eigenvalue buckling,
//! nonlinear/post-buckling, contact, joint nonlinearities, damping,
//! distributed loads, thermal strains and code design factors.

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use symtropy_freeflow_structure_lab::Vec3;

use crate::beam_element::{
    BeamElement3D, BeamElementError, BeamMaterial3D, BeamSection3D,
};
use crate::{solve_dense, MAX_FREE_DOFS, MAX_GLOBAL_DOFS};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMember3D {
    pub from: usize,
    pub to: usize,
    /// Global vector used to orient the element local y-axis.
    pub orientation_hint_global: Vec3,
    pub material: BeamMaterial3D,
    pub section: BeamSection3D,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameModel3D {
    pub nodes: Vec<Vec3>,
    pub members: Vec<FrameMember3D>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameBoundaryConditions {
    /// Per node: [ux, uy, uz, rx, ry, rz] constrained DOFs.
    pub fixed_dofs: Vec<[bool; 6]>,
    /// Per node: [Fx, Fy, Fz, Mx, My, Mz] in N and N*m.
    pub nodal_loads: Vec<[f64; 6]>,
}

impl FrameBoundaryConditions {
    pub fn new(node_count: usize) -> Self {
        Self {
            fixed_dofs: vec![[false; 6]; node_count],
            nodal_loads: vec![[0.0; 6]; node_count],
        }
    }

    pub fn fix_node(&mut self, node: usize) -> Result<(), FrameAnalysisError> {
        let fixed = self
            .fixed_dofs
            .get_mut(node)
            .ok_or(FrameAnalysisError::InvalidBoundaryConditions)?;
        *fixed = [true; 6];
        Ok(())
    }

    pub fn fix_dof(
        &mut self,
        node: usize,
        axis: usize,
    ) -> Result<(), FrameAnalysisError> {
        if axis >= 6 {
            return Err(FrameAnalysisError::InvalidBoundaryConditions);
        }
        let fixed = self
            .fixed_dofs
            .get_mut(node)
            .ok_or(FrameAnalysisError::InvalidBoundaryConditions)?;
        fixed[axis] = true;
        Ok(())
    }

    pub fn add_load(
        &mut self,
        node: usize,
        load: [f64; 6],
    ) -> Result<(), FrameAnalysisError> {
        if !load.iter().all(|value| value.is_finite()) {
            return Err(FrameAnalysisError::NonFiniteInput);
        }
        let nodal_load = self
            .nodal_loads
            .get_mut(node)
            .ok_or(FrameAnalysisError::InvalidBoundaryConditions)?;
        for (target, value) in nodal_load.iter_mut().zip(load) {
            *target += value;
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameMemberResponse {
    pub member_index: usize,
    /// Local member-end actions, [Ni, Vi_y, Vi_z, Ti, Mi_y, Mi_z,
    /// Nj, Vj_y, Vj_z, Tj, Mj_y, Mj_z].
    pub local_end_forces: [f64; 12],
    pub strain_energy_j: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FrameAnalysisResult {
    /// Per node: [ux, uy, uz, rx, ry, rz].
    pub nodal_displacements: Vec<[f64; 6]>,
    /// Reactions are returned for all DOFs; only constrained components are
    /// support reactions. Free components are numerical residuals.
    pub nodal_reactions: Vec<[f64; 6]>,
    pub members: Vec<FrameMemberResponse>,
    pub total_strain_energy_j: f64,
    pub maximum_free_dof_residual: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FrameEquilibriumAudit {
    pub applied_force_n: Vec3,
    pub support_force_n: Vec3,
    pub net_force_residual_n: Vec3,
    pub net_force_residual_norm_n: f64,
    pub applied_moment_nm: Vec3,
    pub support_moment_nm: Vec3,
    pub net_moment_residual_nm: Vec3,
    pub net_moment_residual_norm_nm: f64,
    pub strain_energy_j: f64,
    pub half_external_work_j: f64,
    pub energy_relative_error: f64,
    pub maximum_free_dof_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum FrameAnalysisError {
    InvalidBoundaryConditions,
    InvalidGeometry,
    DuplicateMember { from: usize, to: usize },
    ModelTooLarge,
    NonFiniteInput,
    NonFiniteResult,
    SingularStiffnessMatrix {
        pivot_index: usize,
        pivot_magnitude: f64,
        matrix_scale: f64,
    },
    BeamElement(BeamElementError),
}

impl fmt::Display for FrameAnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidBoundaryConditions => {
                write!(f, "frame boundary-condition dimensions or indices are invalid")
            }
            Self::InvalidGeometry => write!(f, "frame geometry or member endpoints are invalid"),
            Self::DuplicateMember { from, to } => {
                write!(f, "duplicate frame member between nodes {from} and {to}")
            }
            Self::ModelTooLarge => write!(f, "frame exceeds the reference dense-solver budget"),
            Self::NonFiniteInput => write!(f, "frame input contains a non-finite value"),
            Self::NonFiniteResult => write!(f, "frame analysis produced a non-finite value"),
            Self::SingularStiffnessMatrix {
                pivot_index,
                pivot_magnitude,
                matrix_scale,
            } => write!(
                f,
                "singular frame stiffness matrix at pivot {pivot_index}: {pivot_magnitude:e} (scale {matrix_scale:e})"
            ),
            Self::BeamElement(error) => write!(f, "beam element construction failed: {error}"),
        }
    }
}
impl Error for FrameAnalysisError {}

impl FrameModel3D {
    pub fn analyze(
        &self,
        boundary: &FrameBoundaryConditions,
    ) -> Result<FrameAnalysisResult, FrameAnalysisError> {
        validate_frame_inputs(self, boundary)?;

        let dof_count = self
            .nodes
            .len()
            .checked_mul(6)
            .ok_or(FrameAnalysisError::ModelTooLarge)?;
        if dof_count > MAX_GLOBAL_DOFS {
            return Err(FrameAnalysisError::ModelTooLarge);
        }

        let mut elements = Vec::with_capacity(self.members.len());
        for member in &self.members {
            let element = BeamElement3D::new(
                self.nodes[member.from],
                self.nodes[member.to],
                member.orientation_hint_global,
                member.material,
                member.section,
            )
            .map_err(FrameAnalysisError::BeamElement)?;
            elements.push(element);
        }

        let mut global_stiffness = vec![vec![0.0_f64; dof_count]; dof_count];
        for (member_index, member) in self.members.iter().enumerate() {
            let element_stiffness = elements[member_index].global_stiffness_matrix();
            let dof_map = member_dof_map(member.from, member.to);
            for (local_row, global_row) in dof_map.iter().copied().enumerate() {
                let global_row_values = &mut global_stiffness[global_row];
                for (local_column, global_column) in dof_map.iter().copied().enumerate() {
                    global_row_values[global_column] +=
                        element_stiffness[local_row][local_column];
                }
            }
        }

        if !global_stiffness
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        {
            return Err(FrameAnalysisError::NonFiniteResult);
        }

        let mut free_dofs = Vec::new();
        for (node, fixed) in boundary.fixed_dofs.iter().enumerate() {
            for (axis, is_fixed) in fixed.iter().copied().enumerate() {
                if !is_fixed {
                    free_dofs.push(6 * node + axis);
                }
            }
        }
        if free_dofs.len() > MAX_FREE_DOFS {
            return Err(FrameAnalysisError::ModelTooLarge);
        }

        let mut reduced_stiffness =
            vec![vec![0.0_f64; free_dofs.len()]; free_dofs.len()];
        let mut reduced_load = vec![0.0_f64; free_dofs.len()];
        for (reduced_row, global_row) in free_dofs.iter().copied().enumerate() {
            reduced_load[reduced_row] =
                boundary.nodal_loads[global_row / 6][global_row % 6];
            for (reduced_column, global_column) in free_dofs.iter().copied().enumerate() {
                reduced_stiffness[reduced_row][reduced_column] =
                    global_stiffness[global_row][global_column];
            }
        }

        let free_displacements = if free_dofs.is_empty() {
            Vec::new()
        } else {
            solve_dense_frame(reduced_stiffness, reduced_load)?
        };

        let mut global_displacements = vec![0.0_f64; dof_count];
        for (global_dof, displacement) in free_dofs
            .iter()
            .copied()
            .zip(free_displacements.iter().copied())
        {
            global_displacements[global_dof] = displacement;
        }

        if !global_displacements.iter().all(|value| value.is_finite()) {
            return Err(FrameAnalysisError::NonFiniteResult);
        }

        let mut global_reactions = vec![0.0_f64; dof_count];
        let mut maximum_free_dof_residual = 0.0_f64;
        for (row, stiffness_row) in global_stiffness.iter().enumerate() {
            let internal: f64 = stiffness_row
                .iter()
                .zip(global_displacements.iter())
                .map(|(stiffness, displacement)| stiffness * displacement)
                .sum();
            let applied = boundary.nodal_loads[row / 6][row % 6];
            let residual = internal - applied;
            global_reactions[row] = residual;
            if !boundary.fixed_dofs[row / 6][row % 6] {
                maximum_free_dof_residual =
                    maximum_free_dof_residual.max(residual.abs());
            }
        }

        let nodal_displacements = global_displacements
            .chunks_exact(6)
            .map(|values| {
                [
                    values[0], values[1], values[2], values[3], values[4], values[5],
                ]
            })
            .collect::<Vec<_>>();
        let nodal_reactions = global_reactions
            .chunks_exact(6)
            .map(|values| {
                [
                    values[0], values[1], values[2], values[3], values[4], values[5],
                ]
            })
            .collect::<Vec<_>>();

        let mut member_responses = Vec::with_capacity(self.members.len());
        let mut total_strain_energy_j = 0.0_f64;
        for (member_index, member) in self.members.iter().enumerate() {
            let element = &elements[member_index];
            let dof_map = member_dof_map(member.from, member.to);
            let element_global_displacements: [f64; 12] =
                std::array::from_fn(|index| global_displacements[dof_map[index]]);
            let transform = element.global_to_local_transform();
            let local_displacements: [f64; 12] = std::array::from_fn(|row| {
                transform[row]
                    .iter()
                    .zip(element_global_displacements.iter())
                    .map(|(coefficient, displacement)| coefficient * displacement)
                    .sum()
            });
            let local_stiffness = element.local_stiffness_matrix();
            let local_end_forces: [f64; 12] = std::array::from_fn(|row| {
                local_stiffness[row]
                    .iter()
                    .zip(local_displacements.iter())
                    .map(|(coefficient, displacement)| coefficient * displacement)
                    .sum()
            });
            let strain_energy_j = 0.5
                * local_end_forces
                    .iter()
                    .zip(local_displacements.iter())
                    .map(|(force, displacement)| force * displacement)
                    .sum::<f64>();
            if !strain_energy_j.is_finite()
                || !local_end_forces.iter().all(|value| value.is_finite())
            {
                return Err(FrameAnalysisError::NonFiniteResult);
            }
            total_strain_energy_j += strain_energy_j;
            member_responses.push(FrameMemberResponse {
                member_index,
                local_end_forces,
                strain_energy_j,
            });
        }

        if !total_strain_energy_j.is_finite()
            || !maximum_free_dof_residual.is_finite()
        {
            return Err(FrameAnalysisError::NonFiniteResult);
        }

        Ok(FrameAnalysisResult {
            nodal_displacements,
            nodal_reactions,
            members: member_responses,
            total_strain_energy_j,
            maximum_free_dof_residual,
        })
    }
}

pub fn audit_frame_equilibrium(
    model: &FrameModel3D,
    boundary: &FrameBoundaryConditions,
    result: &FrameAnalysisResult,
) -> Result<FrameEquilibriumAudit, FrameAnalysisError> {
    let node_count = model.nodes.len();
    if node_count == 0
        || boundary.fixed_dofs.len() != node_count
        || boundary.nodal_loads.len() != node_count
        || result.nodal_displacements.len() != node_count
        || result.nodal_reactions.len() != node_count
    {
        return Err(FrameAnalysisError::InvalidBoundaryConditions);
    }

    let mut applied_force = Vec3::new(0.0, 0.0, 0.0);
    let mut support_force = Vec3::new(0.0, 0.0, 0.0);
    let mut applied_moment = Vec3::new(0.0, 0.0, 0.0);
    let mut support_moment = Vec3::new(0.0, 0.0, 0.0);
    let mut external_work_j = 0.0_f64;

    for (node, (position, ((load, reaction), (fixed, displacement)))) in model
        .nodes
        .iter()
        .zip(
            boundary
                .nodal_loads
                .iter()
                .zip(result.nodal_reactions.iter())
                .zip(
                    boundary
                        .fixed_dofs
                        .iter()
                        .zip(result.nodal_displacements.iter()),
                ),
        )
        .enumerate()
    {
        let force = Vec3::new(load[0], load[1], load[2]);
        let couple = Vec3::new(load[3], load[4], load[5]);
        applied_force = applied_force + force;
        applied_moment = applied_moment + position.cross(force) + couple;

        for (load_component, displacement_component) in load.iter().zip(displacement.iter()) {
            external_work_j += load_component * displacement_component;
        }

        let reaction_force = Vec3::new(
            if fixed[0] { reaction[0] } else { 0.0 },
            if fixed[1] { reaction[1] } else { 0.0 },
            if fixed[2] { reaction[2] } else { 0.0 },
        );
        let reaction_couple = Vec3::new(
            if fixed[3] { reaction[3] } else { 0.0 },
            if fixed[4] { reaction[4] } else { 0.0 },
            if fixed[5] { reaction[5] } else { 0.0 },
        );
        support_force = support_force + reaction_force;
        support_moment = support_moment + position.cross(reaction_force) + reaction_couple;
    }

    let net_force = applied_force + support_force;
    let net_moment = applied_moment + support_moment;
    let half_external_work_j = 0.5 * external_work_j;
    let energy_scale = result
        .total_strain_energy_j
        .abs()
        .max(half_external_work_j.abs())
        .max(1.0e-15);
    let energy_relative_error = (result.total_strain_energy_j - half_external_work_j).abs()
        / energy_scale;
    let scalar_values = [
        applied_force.x,
        applied_force.y,
        applied_force.z,
        support_force.x,
        support_force.y,
        support_force.z,
        applied_moment.x,
        applied_moment.y,
        applied_moment.z,
        support_moment.x,
        support_moment.y,
        support_moment.z,
        net_force.norm(),
        net_moment.norm(),
        result.total_strain_energy_j,
        half_external_work_j,
        energy_relative_error,
    ];
    if !scalar_values.iter().all(|value| value.is_finite()) {
        return Err(FrameAnalysisError::NonFiniteResult);
    }

    Ok(FrameEquilibriumAudit {
        applied_force_n: applied_force,
        support_force_n: support_force,
        net_force_residual_n: net_force,
        net_force_residual_norm_n: net_force.norm(),
        applied_moment_nm: applied_moment,
        support_moment_nm: support_moment,
        net_moment_residual_nm: net_moment,
        net_moment_residual_norm_nm: net_moment.norm(),
        strain_energy_j: result.total_strain_energy_j,
        half_external_work_j,
        energy_relative_error,
        maximum_free_dof_residual: result.maximum_free_dof_residual,
    })
}

fn validate_frame_inputs(
    model: &FrameModel3D,
    boundary: &FrameBoundaryConditions,
) -> Result<(), FrameAnalysisError> {
    if model.nodes.is_empty()
        || model.members.is_empty()
        || model.nodes.len() != boundary.fixed_dofs.len()
        || model.nodes.len() != boundary.nodal_loads.len()
    {
        return Err(FrameAnalysisError::InvalidBoundaryConditions);
    }
    if !model.nodes.iter().all(|node| node.is_finite())
        || !boundary
            .nodal_loads
            .iter()
            .flatten()
            .all(|value| value.is_finite())
    {
        return Err(FrameAnalysisError::NonFiniteInput);
    }
    let mut seen = HashSet::with_capacity(model.members.len());
    for member in &model.members {
        if member.from >= model.nodes.len()
            || member.to >= model.nodes.len()
            || member.from == member.to
        {
            return Err(FrameAnalysisError::InvalidGeometry);
        }
        let key = if member.from < member.to {
            (member.from, member.to)
        } else {
            (member.to, member.from)
        };
        if !seen.insert(key) {
            return Err(FrameAnalysisError::DuplicateMember {
                from: key.0,
                to: key.1,
            });
        }
        if !member.orientation_hint_global.is_finite() {
            return Err(FrameAnalysisError::NonFiniteInput);
        }
        let length = model.nodes[member.from].distance(model.nodes[member.to]);
        if !length.is_finite() || length <= f64::MIN_POSITIVE {
            return Err(FrameAnalysisError::InvalidGeometry);
        }
    }
    Ok(())
}

fn member_dof_map(from: usize, to: usize) -> [usize; 12] {
    [
        6 * from,
        6 * from + 1,
        6 * from + 2,
        6 * from + 3,
        6 * from + 4,
        6 * from + 5,
        6 * to,
        6 * to + 1,
        6 * to + 2,
        6 * to + 3,
        6 * to + 4,
        6 * to + 5,
    ]
}

fn solve_dense_frame(
    matrix: Vec<Vec<f64>>,
    rhs: Vec<f64>,
) -> Result<Vec<f64>, FrameAnalysisError> {
    let size = rhs.len();
    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err(FrameAnalysisError::InvalidBoundaryConditions);
    }
    let matrix_scale = matrix
        .iter()
        .flat_map(|row| row.iter())
        .fold(0.0_f64, |largest, value| largest.max(value.abs()));
    if !matrix_scale.is_finite() || matrix_scale == 0.0 {
        return Err(FrameAnalysisError::SingularStiffnessMatrix {
            pivot_index: 0,
            pivot_magnitude: 0.0,
            matrix_scale,
        });
    }

    // The crate's truss solver uses the same pivot policy; translate its
    // typed singular outcome so the frame path remains independent at API level.
    solve_dense(matrix, rhs).map_err(|error| match error {
        crate::AnalysisError::SingularStiffnessMatrix {
            pivot_index,
            pivot_magnitude,
            matrix_scale,
        } => FrameAnalysisError::SingularStiffnessMatrix {
            pivot_index,
            pivot_magnitude,
            matrix_scale,
        },
        _ => FrameAnalysisError::NonFiniteResult,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn material() -> BeamMaterial3D {
        BeamMaterial3D {
            youngs_modulus_pa: 200.0e9,
            shear_modulus_pa: 80.0e9,
        }
    }

    fn section() -> BeamSection3D {
        BeamSection3D {
            area_m2: 0.01,
            torsional_constant_m4: 1.0e-5,
            second_moment_y_m4: 1.0e-5,
            second_moment_z_m4: 2.0e-5,
        }
    }

    fn cantilever_model() -> (FrameModel3D, FrameBoundaryConditions) {
        let model = FrameModel3D {
            nodes: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ],
            members: vec![FrameMember3D {
                from: 0,
                to: 1,
                orientation_hint_global: Vec3::new(0.0, 1.0, 0.0),
                material: material(),
                section: section(),
            }],
        };
        let mut boundary = FrameBoundaryConditions::new(2);
        boundary.fix_node(0).unwrap();
        (model, boundary)
    }

    #[test]
    fn cantilever_global_frame_solution_matches_closed_form_y_bending() {
        let (model, mut boundary) = cantilever_model();
        boundary.add_load(1, [0.0, 1_000.0, 0.0, 0.0, 0.0, 0.0]).unwrap();
        let result = model.analyze(&boundary).unwrap();
        let audit = audit_frame_equilibrium(&model, &boundary, &result).unwrap();

        let length = 2.0_f64;
        let force = 1_000.0_f64;
        let expected_deflection = force * length.powi(3)
            / (3.0 * material().youngs_modulus_pa * section().second_moment_z_m4);
        let expected_rotation = force * length.powi(2)
            / (2.0 * material().youngs_modulus_pa * section().second_moment_z_m4);

        assert!(
            (result.nodal_displacements[1][1] - expected_deflection).abs()
                / expected_deflection
                < 1.0e-10
        );
        assert!(
            (result.nodal_displacements[1][5] - expected_rotation).abs()
                / expected_rotation
                < 1.0e-10
        );
        assert!(audit.net_force_residual_norm_n < 1.0e-7);
        assert!(audit.net_moment_residual_norm_nm < 1.0e-7);
        assert!(audit.energy_relative_error < 1.0e-10);
    }

    #[test]
    fn cantilever_global_frame_solution_matches_closed_form_z_bending() {
        let (model, mut boundary) = cantilever_model();
        boundary.add_load(1, [0.0, 0.0, 1_000.0, 0.0, 0.0, 0.0]).unwrap();
        let result = model.analyze(&boundary).unwrap();

        let length = 2.0_f64;
        let force = 1_000.0_f64;
        let expected_deflection = force * length.powi(3)
            / (3.0 * material().youngs_modulus_pa * section().second_moment_y_m4);
        let expected_rotation = -force * length.powi(2)
            / (2.0 * material().youngs_modulus_pa * section().second_moment_y_m4);

        assert!(
            (result.nodal_displacements[1][2] - expected_deflection).abs()
                / expected_deflection
                < 1.0e-10
        );
        assert!(
            (result.nodal_displacements[1][4] - expected_rotation).abs()
                / expected_rotation.abs()
                < 1.0e-10
        );
    }

    #[test]
    fn free_frame_detects_six_rigid_body_mechanisms() {
        let (model, boundary) = cantilever_model();
        let free = FrameBoundaryConditions::new(2);
        assert!(matches!(
            model.analyze(&free),
            Err(FrameAnalysisError::SingularStiffnessMatrix { .. })
        ));
        assert_eq!(boundary.fixed_dofs[0], [true; 6]);
    }

    #[test]
    fn duplicate_members_and_invalid_loads_fail_closed() {
        let (mut model, mut boundary) = cantilever_model();
        model.members.push(model.members[0]);
        assert!(matches!(
            model.analyze(&boundary),
            Err(FrameAnalysisError::DuplicateMember { .. })
        ));

        let (model, mut boundary) = cantilever_model();
        assert_eq!(
            boundary.add_load(1, [f64::NAN, 0.0, 0.0, 0.0, 0.0, 0.0]),
            Err(FrameAnalysisError::NonFiniteInput)
        );
        boundary.add_load(1, [1.0, 0.0, 0.0, 0.0, 0.0, 0.0]).unwrap();
        assert!(model.analyze(&boundary).is_ok());
    }
}
