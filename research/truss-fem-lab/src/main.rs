use symtropy_freeflow_structure_lab::tetrahedral::{TetrahedralAssembly, TetrahedralConfig};
use symtropy_truss_fem_lab::benchmark::{
    evaluate_mass_normalized_candidate, MassNormalizedBenchmarkSpec,
};
use symtropy_truss_fem_lab::damage::{
    bounded_pair_member_removal_campaign, single_member_removal_campaign,
};
use symtropy_truss_fem_lab::stability::{
    screen_member_stability, MemberStabilityProperties,
};
use symtropy_truss_fem_lab::{BoundaryConditions, TrussMaterial, TrussModel};

fn main() {
    let assembly = TetrahedralAssembly::generate(TetrahedralConfig::default())
        .expect("valid tetrahedral geometry fixture");
    let model = TrussModel::from_tetrahedral_assembly(&assembly);
    let mut boundary = BoundaryConditions::new(model.nodes.len());

    boundary.fix_node(0).expect("support node 0");
    boundary.fix_dof(1, 1).expect("support node 1 y");
    boundary.fix_dof(1, 2).expect("support node 1 z");
    boundary.fix_dof(2, 2).expect("support node 2 z");
    boundary
        .add_force(model.nodes.len() - 1, [25.0, -10.0, -100.0])
        .expect("finite load");

    let material = TrussMaterial {
        youngs_modulus_pa: 200.0e9,
        cross_section_area_m2: 0.01,
    };
    let result = model
        .analyze(material, &boundary)
        .expect("supported ideal truss should solve");

    println!("Symtropy pin-jointed truss FEM reference lab");
    println!("nodes={}", model.nodes.len());
    println!("members={}", model.members.len());
    println!(
        "total_strain_energy_j={:.12e}",
        result.total_strain_energy_j
    );
    println!(
        "max_free_dof_equilibrium_residual_n={:.12e}",
        result.max_free_dof_equilibrium_residual_n
    );
    let max_displacement = result
        .node_displacements_m
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .fold(0.0_f64, f64::max);
    let max_stress = result
        .members
        .iter()
        .map(|m| m.axial_stress_pa.abs())
        .fold(0.0_f64, f64::max);
    println!("max_node_displacement_m={:.12e}", max_displacement);
    println!("max_abs_axial_stress_pa={:.12e}", max_stress);

    let section = MemberStabilityProperties {
        youngs_modulus_pa: material.youngs_modulus_pa,
        cross_section_area_m2: material.cross_section_area_m2,
        weak_axis_second_moment_m4: 1.0e-6,
        yield_strength_pa: 250.0e6,
        effective_length_factor: 1.0,
    };
    let member_sections = vec![section; model.members.len()];
    let stability = screen_member_stability(&result, &member_sections)
        .expect("valid declared member stability properties");
    println!(
        "maximum_yield_utilization={:.12e}",
        stability.maximum_yield_utilization
    );
    println!(
        "maximum_elastic_euler_utilization={:?}",
        stability.maximum_elastic_euler_utilization
    );
    println!("yield_exceedance_count={}", stability.yield_exceedance_count);
    println!(
        "elastic_euler_exceedance_count={}",
        stability.elastic_euler_exceedance_count
    );
    println!(
        "euler_outside_nominal_range_count={}",
        stability.euler_outside_nominal_range_count
    );

    let damage = single_member_removal_campaign(&model, material, &boundary)
        .expect("intact model must solve before damage campaign");
    println!("single_member_removals={}", damage.total_removals());
    println!("single_member_stable={}", damage.stable_removals);
    println!("single_member_mechanisms={}", damage.mechanism_removals);
    println!("single_member_other_failures={}", damage.other_failures);
    println!(
        "single_member_solvability_fraction={:.6}",
        damage.single_removal_solvability_fraction()
    );

    let pair_damage = bounded_pair_member_removal_campaign(&model, material, &boundary, 32)
        .expect("intact model must solve before pair-removal campaign");
    println!("two_member_pairs_total={}", pair_damage.total_possible_pairs);
    println!("two_member_pairs_tested={}", pair_damage.applied_budget);
    println!("two_member_pairs_omitted={}", pair_damage.omitted_scenarios());
    println!("two_member_stable={}", pair_damage.stable_removals);
    println!("two_member_mechanisms={}", pair_damage.mechanism_removals);
    println!("two_member_other_failures={}", pair_damage.other_failures);

    let mass_matched = evaluate_mass_normalized_candidate(
        &model,
        &boundary,
        MassNormalizedBenchmarkSpec {
            member_mass_budget_kg: 10.0,
            material_density_kg_m3: 2_700.0,
            youngs_modulus_pa: 69.0e9,
            yield_strength_pa: 250.0e6,
            effective_length_factor: 1.0,
            pair_removal_budget: 8,
        },
    )
    .expect("mass-normalized candidate should produce a bounded benchmark receipt");
    println!("mass_matched_target_member_mass_kg={:.9}", mass_matched.target_member_mass_kg);
    println!("mass_matched_computed_member_mass_kg={:.9}", mass_matched.computed_member_mass_kg);
    println!("mass_matched_uniform_area_m2={:.12e}", mass_matched.uniform_cross_section_area_m2);
    println!("mass_matched_max_displacement_m={:.12e}", mass_matched.maximum_node_displacement_m);
    println!("mass_matched_max_yield_utilization={:.12e}", mass_matched.maximum_yield_utilization);
    println!("mass_matched_max_euler_utilization={:?}", mass_matched.maximum_elastic_euler_utilization);
    println!("mass_matched_single_removal_solvability={:.6}", mass_matched.single_member_removal_solvability_fraction);
    println!("mass_matched_pair_removal_tested={}", mass_matched.tested_two_member_pairs);
    println!("mass_matched_pair_removal_omitted={}", mass_matched.omitted_two_member_pairs);
    println!("claim_ceiling=ideal pin-jointed linear-elastic reference only; member mass only");
}
