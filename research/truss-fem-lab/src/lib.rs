//! Small reference solver for linear-elastic 3D pin-jointed trusses.
//!
//! The solver is intentionally narrow. It is a reference for axial bar
//! mechanics and kinematic-mechanism detection, not a production FEM code.

pub mod beam_element;
pub mod benchmark;
pub mod damage;
pub mod interfaces;
pub mod stability;
pub mod verification;

use std::collections::HashSet;
use std::error::Error;
use std::fmt;

use symtropy_freeflow_structure_lab::tetrahedral::TetrahedralAssembly;
use symtropy_freeflow_structure_lab::{StructureGraph, Vec3};

const MAX_GLOBAL_DOFS: usize = 300;
const MAX_FREE_DOFS: usize = 256;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TrussMaterial {
    pub youngs_modulus_pa: f64,
    pub cross_section_area_m2: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TrussMember {
    pub from: usize,
    pub to: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundaryConditions {
    /// True means the corresponding translational degree of freedom is fixed.
    pub fixed_dofs: Vec<[bool; 3]>,
    /// Applied nodal forces in newtons.
    pub nodal_forces_n: Vec<[f64; 3]>,
}

impl BoundaryConditions {
    pub fn new(node_count: usize) -> Self {
        Self {
            fixed_dofs: vec![[false; 3]; node_count],
            nodal_forces_n: vec![[0.0; 3]; node_count],
        }
    }

    pub fn fix_node(&mut self, node: usize) -> Result<(), AnalysisError> {
        let fixed = self
            .fixed_dofs
            .get_mut(node)
            .ok_or(AnalysisError::InvalidBoundaryConditions)?;
        *fixed = [true; 3];
        Ok(())
    }

    pub fn fix_dof(&mut self, node: usize, axis: usize) -> Result<(), AnalysisError> {
        if axis >= 3 {
            return Err(AnalysisError::InvalidBoundaryConditions);
        }
        let fixed = self
            .fixed_dofs
            .get_mut(node)
            .ok_or(AnalysisError::InvalidBoundaryConditions)?;
        fixed[axis] = true;
        Ok(())
    }

    pub fn add_force(&mut self, node: usize, force_n: [f64; 3]) -> Result<(), AnalysisError> {
        if !force_n.iter().all(|value| value.is_finite()) {
            return Err(AnalysisError::NonFiniteInput);
        }
        let force = self
            .nodal_forces_n
            .get_mut(node)
            .ok_or(AnalysisError::InvalidBoundaryConditions)?;
        for axis in 0..3 {
            force[axis] += force_n[axis];
        }
        Ok(())
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct TrussModel {
    pub nodes: Vec<Vec3>,
    pub members: Vec<TrussMember>,
}

impl TrussModel {
    pub fn from_branch_graph(graph: &StructureGraph) -> Self {
        Self {
            nodes: graph.nodes.iter().map(|node| node.position).collect(),
            members: graph
                .members
                .iter()
                .map(|member| TrussMember {
                    from: member.from as usize,
                    to: member.to as usize,
                })
                .collect(),
        }
    }

    pub fn from_tetrahedral_assembly(assembly: &TetrahedralAssembly) -> Self {
        Self {
            nodes: assembly.nodes.iter().map(|node| node.position).collect(),
            members: assembly
                .members
                .iter()
                .map(|member| TrussMember {
                    from: member.from as usize,
                    to: member.to as usize,
                })
                .collect(),
        }
    }

    pub fn analyze(
        &self,
        material: TrussMaterial,
        boundary: &BoundaryConditions,
    ) -> Result<AnalysisResult, AnalysisError> {
        validate_inputs(self, material, boundary)?;

        let node_count = self.nodes.len();
        let dof_count = node_count
            .checked_mul(3)
            .ok_or(AnalysisError::ModelTooLarge)?;
        if dof_count > MAX_GLOBAL_DOFS {
            return Err(AnalysisError::ModelTooLarge);
        }

        let mut stiffness = vec![vec![0.0_f64; dof_count]; dof_count];
        for member in &self.members {
            assemble_member(&mut stiffness, &self.nodes, *member, material)?;
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
            return Err(AnalysisError::ModelTooLarge);
        }

        let mut reduced = vec![vec![0.0; free_dofs.len()]; free_dofs.len()];
        let mut reduced_force = vec![0.0; free_dofs.len()];
        for (row_index, &global_row) in free_dofs.iter().enumerate() {
            reduced_force[row_index] = boundary.nodal_forces_n[global_row / 3][global_row % 3];
            for (column_index, &global_column) in free_dofs.iter().enumerate() {
                reduced[row_index][column_index] = stiffness[global_row][global_column];
            }
        }

        let free_displacements = if free_dofs.is_empty() {
            Vec::new()
        } else {
            solve_dense(reduced, reduced_force)?
        };

        let mut displacement_flat = vec![0.0; dof_count];
        for (&global_dof, value) in free_dofs.iter().zip(free_displacements.iter()) {
            displacement_flat[global_dof] = *value;
        }
        if !displacement_flat.iter().all(|value| value.is_finite()) {
            return Err(AnalysisError::NonFiniteResult);
        }

        let mut reaction_flat = vec![0.0; dof_count];
        let mut max_free_dof_residual = 0.0_f64;
        for row in 0..dof_count {
            let internal_force: f64 = stiffness[row]
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
            .map(|values| [values[0], values[1], values[2]])
            .collect::<Vec<_>>();
        let support_reactions_n = reaction_flat
            .chunks_exact(3)
            .map(|values| [values[0], values[1], values[2]])
            .collect::<Vec<_>>();

        let mut member_results = Vec::with_capacity(self.members.len());
        let mut total_strain_energy = 0.0;
        for member in &self.members {
            let a = self.nodes[member.from];
            let b = self.nodes[member.to];
            let delta = b - a;
            let length = delta.norm();
            let direction = delta.scale(1.0 / length);
            let displacement_a = node_displacements_m[member.from];
            let displacement_b = node_displacements_m[member.to];
            let relative = [
                displacement_b[0] - displacement_a[0],
                displacement_b[1] - displacement_a[1],
                displacement_b[2] - displacement_a[2],
            ];
            let extension =
                relative[0] * direction.x + relative[1] * direction.y + relative[2] * direction.z;
            let strain = extension / length;
            let stress = material.youngs_modulus_pa * strain;
            let axial_force = stress * material.cross_section_area_m2;
            let strain_energy = 0.5 * axial_force * extension;
            if !extension.is_finite()
                || !strain.is_finite()
                || !stress.is_finite()
                || !axial_force.is_finite()
                || !strain_energy.is_finite()
            {
                return Err(AnalysisError::NonFiniteResult);
            }
            total_strain_energy += strain_energy;
            member_results.push(MemberResult {
                from: member.from,
                to: member.to,
                length_m: length,
                extension_m: extension,
                axial_strain: strain,
                axial_force_n: axial_force,
                axial_stress_pa: stress,
                strain_energy_j: strain_energy,
            });
        }

        if !total_strain_energy.is_finite() || !max_free_dof_residual.is_finite() {
            return Err(AnalysisError::NonFiniteResult);
        }

        Ok(AnalysisResult {
            node_displacements_m,
            support_reactions_n,
            members: member_results,
            total_strain_energy_j: total_strain_energy,
            max_free_dof_equilibrium_residual_n: max_free_dof_residual,
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MemberResult {
    pub from: usize,
    pub to: usize,
    pub length_m: f64,
    pub extension_m: f64,
    pub axial_strain: f64,
    pub axial_force_n: f64,
    pub axial_stress_pa: f64,
    pub strain_energy_j: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AnalysisResult {
    pub node_displacements_m: Vec<[f64; 3]>,
    pub support_reactions_n: Vec<[f64; 3]>,
    pub members: Vec<MemberResult>,
    pub total_strain_energy_j: f64,
    pub max_free_dof_equilibrium_residual_n: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum AnalysisError {
    InvalidMaterial,
    InvalidBoundaryConditions,
    InvalidGeometry,
    DuplicateMember {
        from: usize,
        to: usize,
    },
    NonFiniteInput,
    NonFiniteResult,
    ModelTooLarge,
    SingularStiffnessMatrix {
        pivot_index: usize,
        pivot_magnitude: f64,
        matrix_scale: f64,
    },
}

impl fmt::Display for AnalysisError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidMaterial => {
                write!(f, "Young's modulus and area must be finite and positive")
            }
            Self::InvalidBoundaryConditions => {
                write!(f, "boundary condition vectors do not match model")
            }
            Self::InvalidGeometry => {
                write!(f, "model contains invalid geometry or member endpoints")
            }
            Self::DuplicateMember { from, to } => {
                write!(f, "duplicate truss member between {from} and {to}")
            }
            Self::NonFiniteInput => write!(f, "model input contains a non-finite value"),
            Self::NonFiniteResult => write!(f, "analysis produced a non-finite result"),
            Self::ModelTooLarge => write!(
                f,
                "model exceeds the reference solver's dense-matrix budget"
            ),
            Self::SingularStiffnessMatrix {
                pivot_index,
                pivot_magnitude,
                matrix_scale,
            } => write!(
                f,
                "singular or near-singular stiffness matrix at pivot {pivot_index}: {pivot_magnitude:e} (scale {matrix_scale:e})"
            ),
        }
    }
}
impl Error for AnalysisError {}

fn validate_inputs(
    model: &TrussModel,
    material: TrussMaterial,
    boundary: &BoundaryConditions,
) -> Result<(), AnalysisError> {
    if !material.youngs_modulus_pa.is_finite()
        || material.youngs_modulus_pa <= 0.0
        || !material.cross_section_area_m2.is_finite()
        || material.cross_section_area_m2 <= 0.0
    {
        return Err(AnalysisError::InvalidMaterial);
    }
    if model.nodes.is_empty()
        || model.nodes.len() != boundary.fixed_dofs.len()
        || model.nodes.len() != boundary.nodal_forces_n.len()
    {
        return Err(AnalysisError::InvalidBoundaryConditions);
    }
    if !model.nodes.iter().all(|node| node.is_finite())
        || !boundary
            .nodal_forces_n
            .iter()
            .flatten()
            .all(|force| force.is_finite())
    {
        return Err(AnalysisError::NonFiniteInput);
    }
    let mut seen = HashSet::with_capacity(model.members.len());
    for member in &model.members {
        if member.from >= model.nodes.len()
            || member.to >= model.nodes.len()
            || member.from == member.to
        {
            return Err(AnalysisError::InvalidGeometry);
        }
        let key = if member.from < member.to {
            (member.from, member.to)
        } else {
            (member.to, member.from)
        };
        if !seen.insert(key) {
            return Err(AnalysisError::DuplicateMember {
                from: key.0,
                to: key.1,
            });
        }
        let length = model.nodes[member.from].distance(model.nodes[member.to]);
        if !length.is_finite() || length <= f64::MIN_POSITIVE {
            return Err(AnalysisError::InvalidGeometry);
        }
    }
    Ok(())
}

fn assemble_member(
    global: &mut [Vec<f64>],
    nodes: &[Vec3],
    member: TrussMember,
    material: TrussMaterial,
) -> Result<(), AnalysisError> {
    let from = nodes[member.from];
    let to = nodes[member.to];
    let delta = to - from;
    let length = delta.norm();
    if !length.is_finite() || length <= f64::MIN_POSITIVE {
        return Err(AnalysisError::InvalidGeometry);
    }
    let direction = delta.scale(1.0 / length);
    let n = [direction.x, direction.y, direction.z];
    let axial_stiffness = material.youngs_modulus_pa * material.cross_section_area_m2 / length;
    if !axial_stiffness.is_finite() || axial_stiffness <= 0.0 {
        return Err(AnalysisError::InvalidMaterial);
    }

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

fn solve_dense(mut matrix: Vec<Vec<f64>>, mut rhs: Vec<f64>) -> Result<Vec<f64>, AnalysisError> {
    let size = rhs.len();
    if matrix.len() != size || matrix.iter().any(|row| row.len() != size) {
        return Err(AnalysisError::InvalidBoundaryConditions);
    }
    let matrix_scale = matrix
        .iter()
        .flat_map(|row| row.iter())
        .fold(0.0_f64, |largest, value| largest.max(value.abs()));
    if !matrix_scale.is_finite() || matrix_scale == 0.0 {
        return Err(AnalysisError::SingularStiffnessMatrix {
            pivot_index: 0,
            pivot_magnitude: 0.0,
            matrix_scale,
        });
    }

    for pivot in 0..size {
        let mut best_row = pivot;
        let mut best_value = matrix[pivot][pivot].abs();
        for row in (pivot + 1)..size {
            let candidate = matrix[row][pivot].abs();
            if candidate > best_value {
                best_value = candidate;
                best_row = row;
            }
        }
        if !best_value.is_finite() || best_value <= matrix_scale * 1e-12 {
            return Err(AnalysisError::SingularStiffnessMatrix {
                pivot_index: pivot,
                pivot_magnitude: best_value,
                matrix_scale,
            });
        }
        if best_row != pivot {
            matrix.swap(best_row, pivot);
            rhs.swap(best_row, pivot);
        }

        let diagonal = matrix[pivot][pivot];
        for row in (pivot + 1)..size {
            let factor = matrix[row][pivot] / diagonal;
            matrix[row][pivot] = 0.0;
            for column in (pivot + 1)..size {
                matrix[row][column] -= factor * matrix[pivot][column];
            }
            rhs[row] -= factor * rhs[pivot];
        }
    }

    let mut solution = vec![0.0; size];
    for row in (0..size).rev() {
        let tail: f64 = ((row + 1)..size)
            .map(|column| matrix[row][column] * solution[column])
            .sum();
        let diagonal = matrix[row][row];
        if !diagonal.is_finite() || diagonal.abs() <= matrix_scale * 1e-12 {
            return Err(AnalysisError::SingularStiffnessMatrix {
                pivot_index: row,
                pivot_magnitude: diagonal.abs(),
                matrix_scale,
            });
        }
        solution[row] = (rhs[row] - tail) / diagonal;
        if !solution[row].is_finite() {
            return Err(AnalysisError::NonFiniteResult);
        }
    }
    Ok(solution)
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_freeflow_structure_lab::tetrahedral::{TetrahedralAssembly, TetrahedralConfig};
    use symtropy_freeflow_structure_lab::{Config, StructureGraph};

    fn material() -> TrussMaterial {
        TrussMaterial {
            youngs_modulus_pa: 200.0e9,
            cross_section_area_m2: 0.01,
        }
    }

    #[test]
    fn single_bar_matches_closed_form_axial_deformation() {
        let model = TrussModel {
            nodes: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)],
            members: vec![TrussMember { from: 0, to: 1 }],
        };
        let mut boundary = BoundaryConditions::new(2);
        boundary.fix_node(0).unwrap();
        boundary.fix_dof(1, 1).unwrap();
        boundary.fix_dof(1, 2).unwrap();
        boundary.add_force(1, [1_000.0, 0.0, 0.0]).unwrap();

        let result = model.analyze(material(), &boundary).unwrap();
        let expected_displacement = 1_000.0 * 2.0 / (200.0e9 * 0.01);
        assert!((result.node_displacements_m[1][0] - expected_displacement).abs() < 1e-12);
        assert!((result.members[0].axial_force_n - 1_000.0).abs() < 1e-8);
        assert!((result.members[0].axial_stress_pa - 100_000.0).abs() < 1e-6);
        assert!(
            (result.total_strain_energy_j - 0.5 * 1_000.0 * expected_displacement).abs() < 1e-12
        );
        assert!(result.max_free_dof_equilibrium_residual_n < 1e-7);
    }

    #[test]
    fn face_attached_tetrahedral_assembly_solves_under_supported_load() {
        let assembly = TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let model = TrussModel::from_tetrahedral_assembly(&assembly);
        let mut boundary = BoundaryConditions::new(model.nodes.len());

        // Three non-collinear support nodes remove the six rigid-body DOFs.
        boundary.fix_node(0).unwrap();
        boundary.fix_dof(1, 1).unwrap();
        boundary.fix_dof(1, 2).unwrap();
        boundary.fix_dof(2, 2).unwrap();
        let last_node = model.nodes.len() - 1;
        boundary
            .add_force(last_node, [25.0, -10.0, -100.0])
            .unwrap();

        let result = model.analyze(material(), &boundary).unwrap();
        assert!(
            result
                .node_displacements_m
                .iter()
                .flatten()
                .all(|v| v.is_finite())
        );
        assert!(result.members.iter().all(|m| m.axial_stress_pa.is_finite()));
        assert!(result.total_strain_energy_j > 0.0);
        assert!(result.max_free_dof_equilibrium_residual_n < 1e-5);
    }

    #[test]
    fn one_point_attached_branching_graph_exposes_a_kinematic_mechanism() {
        let graph = StructureGraph::generate(Config::default()).unwrap();
        let model = TrussModel::from_branch_graph(&graph);
        let mut boundary = BoundaryConditions::new(model.nodes.len());
        boundary.fix_node(0).unwrap();
        let last_node = model.nodes.len() - 1;
        boundary.add_force(last_node, [0.0, 0.0, -1.0]).unwrap();

        let error = model.analyze(material(), &boundary).unwrap_err();
        assert!(matches!(
            error,
            AnalysisError::SingularStiffnessMatrix { .. }
        ));
    }

    #[test]
    fn invalid_material_and_boundary_vectors_fail_closed() {
        let model = TrussModel {
            nodes: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
            members: vec![TrussMember { from: 0, to: 1 }],
        };
        let boundary = BoundaryConditions::new(2);
        let invalid_material = TrussMaterial {
            youngs_modulus_pa: f64::NAN,
            cross_section_area_m2: 0.01,
        };
        assert_eq!(
            model.analyze(invalid_material, &boundary),
            Err(AnalysisError::InvalidMaterial)
        );
        assert_eq!(
            model.analyze(material(), &BoundaryConditions::new(1)),
            Err(AnalysisError::InvalidBoundaryConditions)
        );
    }
}
