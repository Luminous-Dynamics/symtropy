//! Single-member-removal campaign for ideal pin-jointed truss models.
//!
//! This is a deterministic damage sensitivity study under one frozen load
//! case. It is not a probabilistic reliability calculation, fracture model,
//! fatigue analysis or qualification campaign.

use crate::{
    AnalysisError, AnalysisResult, BoundaryConditions, TrussMaterial, TrussModel,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ResponseEnvelope {
    pub maximum_node_displacement_m: f64,
    pub maximum_absolute_axial_stress_pa: f64,
    pub strain_energy_j: f64,
    pub equilibrium_residual_n: f64,
}

impl From<&AnalysisResult> for ResponseEnvelope {
    fn from(result: &AnalysisResult) -> Self {
        let maximum_node_displacement_m = result
            .node_displacements_m
            .iter()
            .map(|displacement| {
                (displacement[0].powi(2)
                    + displacement[1].powi(2)
                    + displacement[2].powi(2))
                .sqrt()
            })
            .fold(0.0_f64, f64::max);

        let maximum_absolute_axial_stress_pa = result
            .members
            .iter()
            .map(|member| member.axial_stress_pa.abs())
            .fold(0.0_f64, f64::max);

        Self {
            maximum_node_displacement_m,
            maximum_absolute_axial_stress_pa,
            strain_energy_j: result.total_strain_energy_j,
            equilibrium_residual_n: result.max_free_dof_equilibrium_residual_n,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum RemovalOutcome {
    Stable(ResponseEnvelope),
    KinematicMechanism(AnalysisError),
    OtherFailure(AnalysisError),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MemberRemovalScenario {
    /// Index in the original model's member array.
    pub removed_member_index: usize,
    pub removed_member: crate::TrussMember,
    pub outcome: RemovalOutcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DamageCampaign {
    pub intact_response: ResponseEnvelope,
    pub scenarios: Vec<MemberRemovalScenario>,
    pub stable_removals: usize,
    pub mechanism_removals: usize,
    pub other_failures: usize,
}

impl DamageCampaign {
    pub fn total_removals(&self) -> usize {
        self.scenarios.len()
    }

    /// Fraction of single-member removals that preserve a solvable model
    /// under the exact same support and load case.
    ///
    /// This is a deterministic scenario fraction, not a probability of
    /// survival under a random physical failure process.
    pub fn single_removal_solvability_fraction(&self) -> f64 {
        if self.scenarios.is_empty() {
            return 1.0;
        }
        self.stable_removals as f64 / self.scenarios.len() as f64
    }
}

pub fn single_member_removal_campaign(
    model: &TrussModel,
    material: TrussMaterial,
    boundary: &BoundaryConditions,
) -> Result<DamageCampaign, AnalysisError> {
    let intact = model.analyze(material, boundary)?;
    let intact_response = ResponseEnvelope::from(&intact);

    let mut scenarios = Vec::with_capacity(model.members.len());
    let mut stable_removals = 0usize;
    let mut mechanism_removals = 0usize;
    let mut other_failures = 0usize;

    for removed_member_index in 0..model.members.len() {
        let removed_member = model.members[removed_member_index];
        let mut damaged = model.clone();
        damaged.members.remove(removed_member_index);

        let outcome = match damaged.analyze(material, boundary) {
            Ok(result) => {
                stable_removals += 1;
                RemovalOutcome::Stable(ResponseEnvelope::from(&result))
            }
            Err(error @ AnalysisError::SingularStiffnessMatrix { .. }) => {
                mechanism_removals += 1;
                RemovalOutcome::KinematicMechanism(error)
            }
            Err(error) => {
                other_failures += 1;
                RemovalOutcome::OtherFailure(error)
            }
        };

        scenarios.push(MemberRemovalScenario {
            removed_member_index,
            removed_member,
            outcome,
        });
    }

    Ok(DamageCampaign {
        intact_response,
        scenarios,
        stable_removals,
        mechanism_removals,
        other_failures,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundaryConditions, TrussMaterial, TrussMember, TrussModel};
    use symtropy_freeflow_structure_lab::tetrahedral::{
        TetrahedralAssembly, TetrahedralConfig,
    };
    use symtropy_freeflow_structure_lab::Vec3;

    fn material() -> TrussMaterial {
        TrussMaterial {
            youngs_modulus_pa: 200.0e9,
            cross_section_area_m2: 0.01,
        }
    }

    fn supported_bar() -> (TrussModel, BoundaryConditions) {
        let model = TrussModel {
            nodes: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)],
            members: vec![TrussMember { from: 0, to: 1 }],
        };
        let mut boundary = BoundaryConditions::new(2);
        boundary.fix_node(0).unwrap();
        boundary.fix_dof(1, 1).unwrap();
        boundary.fix_dof(1, 2).unwrap();
        boundary.add_force(1, [1_000.0, 0.0, 0.0]).unwrap();
        (model, boundary)
    }

    #[test]
    fn removing_the_only_bar_exposes_a_kinematic_mechanism() {
        let (model, boundary) = supported_bar();
        let campaign = single_member_removal_campaign(&model, material(), &boundary).unwrap();
        assert_eq!(campaign.total_removals(), 1);
        assert_eq!(campaign.stable_removals, 0);
        assert_eq!(campaign.mechanism_removals, 1);
        assert_eq!(campaign.other_failures, 0);
        assert_eq!(campaign.single_removal_solvability_fraction(), 0.0);
        assert!(matches!(
            campaign.scenarios[0].outcome,
            RemovalOutcome::KinematicMechanism(AnalysisError::SingularStiffnessMatrix { .. })
        ));
    }

    #[test]
    fn campaign_tests_every_member_without_mutating_the_original_model() {
        let assembly = TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let model = TrussModel::from_tetrahedral_assembly(&assembly);
        let original_member_count = model.members.len();
        let mut boundary = BoundaryConditions::new(model.nodes.len());
        boundary.fix_node(0).unwrap();
        boundary.fix_dof(1, 1).unwrap();
        boundary.fix_dof(1, 2).unwrap();
        boundary.fix_dof(2, 2).unwrap();
        boundary
            .add_force(model.nodes.len() - 1, [25.0, -10.0, -100.0])
            .unwrap();

        let campaign = single_member_removal_campaign(&model, material(), &boundary).unwrap();
        assert_eq!(campaign.total_removals(), original_member_count);
        assert_eq!(model.members.len(), original_member_count);
        assert_eq!(
            campaign.stable_removals
                + campaign.mechanism_removals
                + campaign.other_failures,
            original_member_count
        );
        assert!(campaign.intact_response.strain_energy_j > 0.0);
        assert!(campaign.intact_response.equilibrium_residual_n < 1e-5);
    }
}
