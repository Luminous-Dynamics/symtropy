//! Mass-normalized candidate evaluation for matched structural experiments.
//!
//! The mass budget covers straight prismatic members only. Connector, node,
//! fastener, service, manufacturing, inspection and deployment masses are
//! deliberately excluded and must be measured in a later system-level study.

use std::error::Error;
use std::f64::consts::PI;
use std::fmt;

use crate::damage::{
    bounded_pair_member_removal_campaign, single_member_removal_campaign,
};
use crate::stability::{screen_member_stability, MemberStabilityProperties};
use crate::{
    AnalysisError, AnalysisResult, BoundaryConditions, TrussMaterial, TrussModel,
};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MassNormalizedBenchmarkSpec {
    pub member_mass_budget_kg: f64,
    pub material_density_kg_m3: f64,
    pub youngs_modulus_pa: f64,
    pub yield_strength_pa: f64,
    /// Effective-length factor for the ideal pinned-column member screen.
    pub effective_length_factor: f64,
    /// Bounded number of two-member removal scenarios. Zero disables this screen.
    pub pair_removal_budget: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MassNormalizedBenchmarkReceipt {
    pub node_count: usize,
    pub member_count: usize,
    pub total_member_length_m: f64,
    pub target_member_mass_kg: f64,
    pub computed_member_mass_kg: f64,
    pub uniform_cross_section_area_m2: f64,
    pub solid_round_weak_axis_inertia_m4: f64,
    pub maximum_node_displacement_m: f64,
    pub maximum_absolute_axial_stress_pa: f64,
    pub total_strain_energy_j: f64,
    pub equilibrium_residual_n: f64,
    pub maximum_yield_utilization: f64,
    pub yield_exceedance_count: usize,
    pub maximum_elastic_euler_utilization: Option<f64>,
    pub elastic_euler_exceedance_count: usize,
    pub euler_outside_nominal_range_count: usize,
    pub single_member_removal_solvability_fraction: f64,
    pub total_two_member_pairs: usize,
    pub tested_two_member_pairs: usize,
    pub omitted_two_member_pairs: usize,
    pub two_member_removal_solvability_fraction: Option<f64>,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BenchmarkError {
    InvalidSpec(&'static str),
    InvalidGeometry,
    NonFiniteResult,
    Analysis(AnalysisError),
    Stability(crate::stability::StabilityError),
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSpec(field) => write!(f, "invalid benchmark specification: {field}"),
            Self::InvalidGeometry => write!(f, "candidate has invalid or zero total member length"),
            Self::NonFiniteResult => write!(f, "benchmark calculation produced a non-finite value"),
            Self::Analysis(error) => write!(f, "structural analysis failed: {error}"),
            Self::Stability(error) => write!(f, "member stability screen failed: {error}"),
        }
    }
}
impl Error for BenchmarkError {}

/// Compute the uniform member cross-section area required to hit a mass
/// budget for the supplied member lengths and material density.
pub fn area_for_member_mass_budget(
    target_mass_kg: f64,
    density_kg_m3: f64,
    total_member_length_m: f64,
) -> Result<f64, BenchmarkError> {
    if !target_mass_kg.is_finite() || target_mass_kg <= 0.0 {
        return Err(BenchmarkError::InvalidSpec("member_mass_budget_kg"));
    }
    if !density_kg_m3.is_finite() || density_kg_m3 <= 0.0 {
        return Err(BenchmarkError::InvalidSpec("material_density_kg_m3"));
    }
    if !total_member_length_m.is_finite() || total_member_length_m <= 0.0 {
        return Err(BenchmarkError::InvalidGeometry);
    }
    let area = target_mass_kg / (density_kg_m3 * total_member_length_m);
    if !area.is_finite() || area <= 0.0 {
        return Err(BenchmarkError::NonFiniteResult);
    }
    Ok(area)
}

/// Evaluate one candidate under a declared member-only mass budget.
///
/// Candidate topology, geometric envelope, boundary conditions and loads must
/// be made comparable by the calling benchmark protocol. This function does
/// not silently normalize geometry or alter supports/loads.
pub fn evaluate_mass_normalized_candidate(
    model: &TrussModel,
    boundary: &BoundaryConditions,
    spec: MassNormalizedBenchmarkSpec,
) -> Result<MassNormalizedBenchmarkReceipt, BenchmarkError> {
    if !spec.youngs_modulus_pa.is_finite() || spec.youngs_modulus_pa <= 0.0 {
        return Err(BenchmarkError::InvalidSpec("youngs_modulus_pa"));
    }
    if !spec.yield_strength_pa.is_finite() || spec.yield_strength_pa <= 0.0 {
        return Err(BenchmarkError::InvalidSpec("yield_strength_pa"));
    }
    if !spec.effective_length_factor.is_finite()
        || spec.effective_length_factor <= 0.0
    {
        return Err(BenchmarkError::InvalidSpec("effective_length_factor"));
    }

    if model.nodes.is_empty() || model.members.is_empty() {
        return Err(BenchmarkError::InvalidGeometry);
    }
    let mut total_member_length_m = 0.0_f64;
    for member in &model.members {
        if member.from >= model.nodes.len()
            || member.to >= model.nodes.len()
            || member.from == member.to
        {
            return Err(BenchmarkError::InvalidGeometry);
        }
        let length = model.nodes[member.from].distance(model.nodes[member.to]);
        if !length.is_finite() || length <= f64::MIN_POSITIVE {
            return Err(BenchmarkError::InvalidGeometry);
        }
        total_member_length_m += length;
    }
    if !total_member_length_m.is_finite() || total_member_length_m <= 0.0 {
        return Err(BenchmarkError::InvalidGeometry);
    }

    let area = area_for_member_mass_budget(
        spec.member_mass_budget_kg,
        spec.material_density_kg_m3,
        total_member_length_m,
    )?;
    // For a solid circular section: A = pi*r^2 and I = pi*r^4/4 = A^2/(4*pi).
    let weak_axis_inertia = area * area / (4.0 * PI);
    if !weak_axis_inertia.is_finite() || weak_axis_inertia <= 0.0 {
        return Err(BenchmarkError::NonFiniteResult);
    }

    let material = TrussMaterial {
        youngs_modulus_pa: spec.youngs_modulus_pa,
        cross_section_area_m2: area,
    };
    let analysis: AnalysisResult = model
        .analyze(material, boundary)
        .map_err(BenchmarkError::Analysis)?;

    let member_properties = vec![
        MemberStabilityProperties {
            youngs_modulus_pa: spec.youngs_modulus_pa,
            cross_section_area_m2: area,
            weak_axis_second_moment_m4: weak_axis_inertia,
            yield_strength_pa: spec.yield_strength_pa,
            effective_length_factor: spec.effective_length_factor,
        };
        model.members.len()
    ];
    let stability = screen_member_stability(&analysis, &member_properties)
        .map_err(BenchmarkError::Stability)?;

    let single_damage = single_member_removal_campaign(model, material, boundary)
        .map_err(BenchmarkError::Analysis)?;
    let pair_damage = bounded_pair_member_removal_campaign(
        model,
        material,
        boundary,
        spec.pair_removal_budget,
    )
    .map_err(BenchmarkError::Analysis)?;

    let maximum_node_displacement_m = analysis
        .node_displacements_m
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .fold(0.0_f64, f64::max);
    let maximum_absolute_axial_stress_pa = analysis
        .members
        .iter()
        .map(|member| member.axial_stress_pa.abs())
        .fold(0.0_f64, f64::max);
    let computed_member_mass_kg =
        spec.material_density_kg_m3 * area * total_member_length_m;
    let two_member_removal_solvability_fraction =
        if pair_damage.applied_budget == 0 {
            None
        } else {
            Some(pair_damage.stable_removals as f64 / pair_damage.applied_budget as f64)
        };

    if !maximum_node_displacement_m.is_finite()
        || !maximum_absolute_axial_stress_pa.is_finite()
        || !computed_member_mass_kg.is_finite()
        || !analysis.total_strain_energy_j.is_finite()
    {
        return Err(BenchmarkError::NonFiniteResult);
    }

    Ok(MassNormalizedBenchmarkReceipt {
        node_count: model.nodes.len(),
        member_count: model.members.len(),
        total_member_length_m,
        target_member_mass_kg: spec.member_mass_budget_kg,
        computed_member_mass_kg,
        uniform_cross_section_area_m2: area,
        solid_round_weak_axis_inertia_m4: weak_axis_inertia,
        maximum_node_displacement_m,
        maximum_absolute_axial_stress_pa,
        total_strain_energy_j: analysis.total_strain_energy_j,
        equilibrium_residual_n: analysis.max_free_dof_equilibrium_residual_n,
        maximum_yield_utilization: stability.maximum_yield_utilization,
        yield_exceedance_count: stability.yield_exceedance_count,
        maximum_elastic_euler_utilization: stability.maximum_elastic_euler_utilization,
        elastic_euler_exceedance_count: stability.elastic_euler_exceedance_count,
        euler_outside_nominal_range_count: stability.euler_outside_nominal_range_count,
        single_member_removal_solvability_fraction: single_damage
            .single_removal_solvability_fraction(),
        total_two_member_pairs: pair_damage.total_possible_pairs,
        tested_two_member_pairs: pair_damage.applied_budget,
        omitted_two_member_pairs: pair_damage.omitted_scenarios(),
        two_member_removal_solvability_fraction,
        claim_ceiling: "member-mass-normalized ideal pin-jointed linear-elastic reference only; connector, node, build, inspection, lifecycle and qualification costs excluded",
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BoundaryConditions, TrussMember, TrussModel};
    use symtropy_freeflow_structure_lab::tetrahedral::{
        TetrahedralAssembly, TetrahedralConfig,
    };

    fn spec() -> MassNormalizedBenchmarkSpec {
        MassNormalizedBenchmarkSpec {
            member_mass_budget_kg: 10.0,
            material_density_kg_m3: 2_700.0,
            youngs_modulus_pa: 69.0e9,
            yield_strength_pa: 250.0e6,
            effective_length_factor: 1.0,
            pair_removal_budget: 8,
        }
    }

    #[test]
    fn area_normalization_hits_declared_member_mass_budget() {
        let area = area_for_member_mass_budget(10.0, 2_700.0, 12.0).unwrap();
        let mass = 2_700.0 * area * 12.0;
        assert!((mass - 10.0).abs() < 1.0e-12);
    }

    #[test]
    fn solid_round_inertia_matches_area_identity() {
        let area = 1.0e-4;
        let inertia = area * area / (4.0 * PI);
        let radius = (area / PI).sqrt();
        let expected = PI * radius.powi(4) / 4.0;
        assert!((inertia - expected).abs() < 1.0e-24);
    }

    #[test]
    fn mass_normalized_tetrahedral_candidate_has_a_complete_receipt() {
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

        let receipt = evaluate_mass_normalized_candidate(&model, &boundary, spec()).unwrap();
        assert_eq!(receipt.node_count, model.nodes.len());
        assert_eq!(receipt.member_count, model.members.len());
        assert!((receipt.computed_member_mass_kg - receipt.target_member_mass_kg).abs() < 1e-10);
        assert!(receipt.maximum_node_displacement_m.is_finite());
        assert!(receipt.maximum_absolute_axial_stress_pa.is_finite());
        assert!(receipt.equilibrium_residual_n < 1e-5);
        assert_eq!(receipt.tested_two_member_pairs, 8);
        assert_eq!(receipt.omitted_two_member_pairs, receipt.total_two_member_pairs - 8);
    }

    #[test]
    fn invalid_mass_budget_is_rejected() {
        assert!(matches!(
            area_for_member_mass_budget(0.0, 2_700.0, 10.0),
            Err(BenchmarkError::InvalidSpec("member_mass_budget_kg"))
        ));
    }

    #[test]
    fn duplicate_member_geometry_is_rejected_by_the_analysis_layer() {
        let model = TrussModel {
            nodes: vec![
                symtropy_freeflow_structure_lab::Vec3::new(0.0, 0.0, 0.0),
                symtropy_freeflow_structure_lab::Vec3::new(1.0, 0.0, 0.0),
            ],
            members: vec![
                TrussMember { from: 0, to: 1 },
                TrussMember { from: 1, to: 0 },
            ],
        };
        let boundary = BoundaryConditions::new(2);
        let error = evaluate_mass_normalized_candidate(&model, &boundary, spec()).unwrap_err();
        assert!(matches!(
            error,
            BenchmarkError::Analysis(AnalysisError::DuplicateMember { .. })
        ));
    }
}
