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

/// Hard cap prevents a caller from turning a research campaign into
/// an unbounded quadratic solve workload.
pub const MAX_PAIR_REMOVAL_SCENARIOS: usize = 1_024;

#[derive(Clone, Debug, PartialEq)]
pub struct PairRemovalScenario {
    /// The two original member-array indices, in ascending order.
    pub removed_member_indices: [usize; 2],
    /// Lexicographic pair rank in the complete pair space.
    pub pair_rank: usize,
    pub outcome: RemovalOutcome,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PairDamageCampaign {
    pub intact_response: ResponseEnvelope,
    pub total_possible_pairs: usize,
    pub requested_budget: usize,
    pub applied_budget: usize,
    pub scenarios: Vec<PairRemovalScenario>,
    pub stable_removals: usize,
    pub mechanism_removals: usize,
    pub other_failures: usize,
}

impl PairDamageCampaign {
    pub fn omitted_scenarios(&self) -> usize {
        self.total_possible_pairs.saturating_sub(self.scenarios.len())
    }
}

/// Resolve a zero-based lexicographic rank into an unordered pair (i, j),
/// with i < j, without materializing the full O(n²) pair list.
fn pair_at_rank(member_count: usize, mut rank: usize) -> Option<(usize, usize)> {
    for left in 0..member_count {
        let row_count = member_count.checked_sub(left + 1)?;
        if rank < row_count {
            return Some((left, left + 1 + rank));
        }
        rank -= row_count;
    }
    None
}

/// Run a deterministic, evenly-spaced sample of the complete two-member
/// removal space. Every omitted pair is counted. The result is a bounded
/// scenario campaign, not an estimate of physical failure probability.
pub fn bounded_pair_member_removal_campaign(
    model: &TrussModel,
    material: TrussMaterial,
    boundary: &BoundaryConditions,
    requested_budget: usize,
) -> Result<PairDamageCampaign, AnalysisError> {
    let intact = model.analyze(material, boundary)?;
    let intact_response = ResponseEnvelope::from(&intact);

    let member_count = model.members.len();
    let total_possible_pairs = member_count
        .checked_mul(member_count.saturating_sub(1))
        .map(|value| value / 2)
        .ok_or(AnalysisError::ModelTooLarge)?;
    let applied_budget = requested_budget
        .min(total_possible_pairs)
        .min(MAX_PAIR_REMOVAL_SCENARIOS);

    let mut scenarios = Vec::with_capacity(applied_budget);
    let mut stable_removals = 0usize;
    let mut mechanism_removals = 0usize;
    let mut other_failures = 0usize;

    for sample_index in 0..applied_budget {
        // Select the center of each equal-width interval in pair-rank space.
        // u128 keeps the multiplication safe for large, but bounded, models.
        let numerator = (2_u128 * sample_index as u128 + 1)
            * total_possible_pairs as u128;
        let denominator = 2_u128 * applied_budget as u128;
        let pair_rank = (numerator / denominator) as usize;
        let (left, right) = pair_at_rank(member_count, pair_rank)
            .ok_or(AnalysisError::InvalidGeometry)?;

        let mut damaged = model.clone();
        damaged.members.remove(right);
        damaged.members.remove(left);

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

        scenarios.push(PairRemovalScenario {
            removed_member_indices: [left, right],
            pair_rank,
            outcome,
        });
    }

    Ok(PairDamageCampaign {
        intact_response,
        total_possible_pairs,
        requested_budget,
        applied_budget,
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
            &campaign.scenarios[0].outcome,
            RemovalOutcome::KinematicMechanism(AnalysisError::SingularStiffnessMatrix { .. })
        ));
    }

    #[test]
    fn pair_campaign_is_bounded_and_reports_every_omitted_pair() {
        let assembly = TetrahedralAssembly::generate(TetrahedralConfig::default()).unwrap();
        let model = TrussModel::from_tetrahedral_assembly(&assembly);
        let mut boundary = BoundaryConditions::new(model.nodes.len());
        boundary.fix_node(0).unwrap();
        boundary.fix_dof(1, 1).unwrap();
        boundary.fix_dof(1, 2).unwrap();
        boundary.fix_dof(2, 2).unwrap();
        boundary
            .add_force(model.nodes.len() - 1, [25.0, -10.0, -100.0])
            .unwrap();

        let campaign =
            bounded_pair_member_removal_campaign(&model, material(), &boundary, 8).unwrap();
        assert_eq!(campaign.applied_budget, 8);
        assert_eq!(campaign.scenarios.len(), 8);
        assert_eq!(
            campaign.stable_removals
                + campaign.mechanism_removals
                + campaign.other_failures,
            8
        );
        assert_eq!(
            campaign.omitted_scenarios(),
            campaign.total_possible_pairs - 8
        );
        let ranks = campaign
            .scenarios
            .iter()
            .map(|scenario| scenario.pair_rank)
            .collect::<std::collections::HashSet<_>>();
        assert_eq!(ranks.len(), 8);
    }

    #[test]
    fn zero_pair_budget_is_an_explicit_no_sampling_result() {
        let (model, boundary) = supported_bar();
        let campaign =
            bounded_pair_member_removal_campaign(&model, material(), &boundary, 0).unwrap();
        assert_eq!(campaign.total_possible_pairs, 0);
        assert_eq!(campaign.applied_budget, 0);
        assert_eq!(campaign.omitted_scenarios(), 0);
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
