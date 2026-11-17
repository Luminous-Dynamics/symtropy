//! Global force, moment and strain-energy consistency checks.
//!
//! These checks audit a solver result under the supplied boundary conditions.
//! They do not prove the model represents the real structure or that omitted
//! physics are negligible.

use std::error::Error;
use std::fmt;

use symtropy_freeflow_structure_lab::Vec3;

use crate::{AnalysisResult, BoundaryConditions, TrussModel};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct EquilibriumAudit {
    pub total_applied_force_n: Vec3,
    pub total_support_reaction_n: Vec3,
    pub net_force_residual_n: Vec3,
    pub net_force_residual_norm_n: f64,
    pub total_applied_moment_nm: Vec3,
    pub total_support_moment_nm: Vec3,
    pub net_moment_residual_nm: Vec3,
    pub net_moment_residual_norm_nm: f64,
    pub strain_energy_j: f64,
    pub half_external_work_j: f64,
    pub energy_relative_error: f64,
    pub max_free_dof_residual_n: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum VerificationError {
    DimensionMismatch,
    NonFiniteInput,
    NonFiniteResult,
}

impl fmt::Display for VerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DimensionMismatch => write!(f, "model, boundary and result dimensions differ"),
            Self::NonFiniteInput => write!(f, "verification input contains a non-finite value"),
            Self::NonFiniteResult => write!(f, "equilibrium audit produced a non-finite value"),
        }
    }
}
impl Error for VerificationError {}

/// Audit global translational equilibrium, moment equilibrium about the
/// global origin, and the linear-elastic strain-energy identity.
///
/// Support reactions are summed only on constrained degrees of freedom.
/// Free-degree residual reactions are excluded from the support resultant.
/// All loads and reactions are assumed to act at the supplied node positions.
pub fn audit_equilibrium(
    model: &TrussModel,
    boundary: &BoundaryConditions,
    result: &AnalysisResult,
) -> Result<EquilibriumAudit, VerificationError> {
    let node_count = model.nodes.len();
    if node_count == 0
        || boundary.fixed_dofs.len() != node_count
        || boundary.nodal_forces_n.len() != node_count
        || result.node_displacements_m.len() != node_count
        || result.support_reactions_n.len() != node_count
    {
        return Err(VerificationError::DimensionMismatch);
    }

    if !model.nodes.iter().all(|node| node.is_finite())
        || !boundary
            .nodal_forces_n
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        || !result
            .node_displacements_m
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        || !result
            .support_reactions_n
            .iter()
            .flatten()
            .all(|value| value.is_finite())
        || !result.total_strain_energy_j.is_finite()
        || !result.max_free_dof_equilibrium_residual_n.is_finite()
    {
        return Err(VerificationError::NonFiniteInput);
    }

    let mut total_applied_force = Vec3::new(0.0, 0.0, 0.0);
    let mut total_support_reaction = Vec3::new(0.0, 0.0, 0.0);
    let mut applied_moment = Vec3::new(0.0, 0.0, 0.0);
    let mut support_moment = Vec3::new(0.0, 0.0, 0.0);
    let mut external_work = 0.0_f64;

    for node in 0..node_count {
        let position = model.nodes[node];
        let applied = Vec3::new(
            boundary.nodal_forces_n[node][0],
            boundary.nodal_forces_n[node][1],
            boundary.nodal_forces_n[node][2],
        );
        total_applied_force = total_applied_force + applied;
        applied_moment = applied_moment + position.cross(applied);

        let displacement = result.node_displacements_m[node];
        external_work += applied.x * displacement[0]
            + applied.y * displacement[1]
            + applied.z * displacement[2];

        let reaction = result.support_reactions_n[node];
        let mut constrained_reaction = Vec3::new(0.0, 0.0, 0.0);
        for axis in 0..3 {
            if boundary.fixed_dofs[node][axis] {
                match axis {
                    0 => constrained_reaction.x = reaction[0],
                    1 => constrained_reaction.y = reaction[1],
                    2 => constrained_reaction.z = reaction[2],
                    _ => unreachable!("axis iterates only over 0..3"),
                }
            }
        }
        total_support_reaction = total_support_reaction + constrained_reaction;
        support_moment = support_moment + position.cross(constrained_reaction);
    }

    let net_force_residual = total_applied_force + total_support_reaction;
    let net_moment_residual = applied_moment + support_moment;
    let half_external_work = 0.5 * external_work;
    let energy_scale = result
        .total_strain_energy_j
        .abs()
        .max(half_external_work.abs())
        .max(1.0e-15);
    let energy_relative_error =
        (result.total_strain_energy_j - half_external_work).abs() / energy_scale;

    let values = [
        total_applied_force.x,
        total_applied_force.y,
        total_applied_force.z,
        total_support_reaction.x,
        total_support_reaction.y,
        total_support_reaction.z,
        net_force_residual.x,
        net_force_residual.y,
        net_force_residual.z,
        applied_moment.x,
        applied_moment.y,
        applied_moment.z,
        support_moment.x,
        support_moment.y,
        support_moment.z,
        net_moment_residual.x,
        net_moment_residual.y,
        net_moment_residual.z,
        result.total_strain_energy_j,
        half_external_work,
        energy_relative_error,
        result.max_free_dof_equilibrium_residual_n,
    ];
    if !values.iter().all(|value| value.is_finite()) {
        return Err(VerificationError::NonFiniteResult);
    }

    Ok(EquilibriumAudit {
        total_applied_force_n: total_applied_force,
        total_support_reaction_n: total_support_reaction,
        net_force_residual_n: net_force_residual,
        net_force_residual_norm_n: net_force_residual.norm(),
        total_applied_moment_nm: applied_moment,
        total_support_moment_nm: support_moment,
        net_moment_residual_nm: net_moment_residual,
        net_moment_residual_norm_nm: net_moment_residual.norm(),
        strain_energy_j: result.total_strain_energy_j,
        half_external_work_j: half_external_work,
        energy_relative_error,
        max_free_dof_residual_n: result.max_free_dof_equilibrium_residual_n,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundaryConditions, TrussMaterial, TrussMember, TrussModel};
    use symtropy_freeflow_structure_lab::Vec3;

    #[test]
    fn single_bar_closes_force_moment_and_energy_balances() {
        let model = TrussModel {
            nodes: vec![
                Vec3::new(0.0, 0.0, 0.0),
                Vec3::new(2.0, 0.0, 0.0),
            ],
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
        boundary.add_force(1, [1_000.0, 0.0, 0.0]).unwrap();

        let result = model.analyze(material, &boundary).unwrap();
        let audit = audit_equilibrium(&model, &boundary, &result).unwrap();

        assert!(audit.net_force_residual_norm_n < 1.0e-8);
        assert!(audit.net_moment_residual_norm_nm < 1.0e-8);
        assert!(audit.energy_relative_error < 1.0e-10);
        assert!(audit.max_free_dof_residual_n < 1.0e-8);
        assert!((audit.total_applied_force_n.x - 1_000.0).abs() < 1.0e-12);
        assert!((audit.total_support_reaction_n.x + 1_000.0).abs() < 1.0e-8);
    }

    #[test]
    fn support_reactions_are_not_counted_on_free_dofs() {
        let model = TrussModel {
            nodes: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(1.0, 0.0, 0.0)],
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
        boundary.add_force(1, [500.0, 0.0, 0.0]).unwrap();

        let result = model.analyze(material, &boundary).unwrap();
        let audit = audit_equilibrium(&model, &boundary, &result).unwrap();
        assert!(audit.net_force_residual_norm_n < 1.0e-8);
    }

    #[test]
    fn mismatched_result_dimensions_fail_closed() {
        let model = TrussModel {
            nodes: vec![Vec3::new(0.0, 0.0, 0.0)],
            members: Vec::new(),
        };
        let boundary = BoundaryConditions::new(1);
        let result = AnalysisResult {
            node_displacements_m: Vec::new(),
            support_reactions_n: Vec::new(),
            members: Vec::new(),
            total_strain_energy_j: 0.0,
            max_free_dof_equilibrium_residual_n: 0.0,
        };
        assert_eq!(
            audit_equilibrium(&model, &boundary, &result),
            Err(VerificationError::DimensionMismatch)
        );
    }
}
