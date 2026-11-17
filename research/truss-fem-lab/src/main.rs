use symtropy_freeflow_structure_lab::tetrahedral::{TetrahedralAssembly, TetrahedralConfig};
use symtropy_freeflow_structure_lab::Vec3;
use symtropy_truss_fem_lab::beam_element::{BeamMaterial3D, BeamSection3D};
use symtropy_truss_fem_lab::frame_analysis::{
    audit_frame_equilibrium, FrameBoundaryConditions, FrameMember3D, FrameModel3D,
};
use symtropy_truss_fem_lab::benchmark::{
    evaluate_mass_normalized_candidate, matched_tetrahedron_pair, MassNormalizedBenchmarkSpec,
};
use symtropy_truss_fem_lab::damage::{
    bounded_pair_member_removal_campaign, single_member_removal_campaign,
};
use symtropy_truss_fem_lab::interfaces::{
    analyze_with_joint_end_stiffness, JointEndStiffness,
};
use symtropy_truss_fem_lab::stability::{
    screen_member_stability, MemberStabilityProperties,
};
use symtropy_truss_fem_lab::verification::audit_equilibrium;
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
    let equilibrium = audit_equilibrium(&model, &boundary, &result)
        .expect("finite model result should pass equilibrium audit input validation");
    println!(
        "global_force_residual_n={:.12e}",
        equilibrium.net_force_residual_norm_n
    );
    println!(
        "global_moment_residual_nm={:.12e}",
        equilibrium.net_moment_residual_norm_nm
    );
    println!(
        "strain_energy_relative_error={:.12e}",
        equilibrium.energy_relative_error
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

    let joint_properties = vec![
        JointEndStiffness {
            from_axial_n_per_m: 1.0e9,
            to_axial_n_per_m: 1.0e9,
        };
        model.members.len()
    ];
    let compliant = analyze_with_joint_end_stiffness(
        &model,
        material,
        &boundary,
        &joint_properties,
    )
    .expect("finite positive connector stiffness should produce a compliant analysis");
    let compliant_max_displacement = compliant
        .node_displacements_m
        .iter()
        .map(|v| (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt())
        .fold(0.0_f64, f64::max);
    let maximum_joint_slip = compliant
        .members
        .iter()
        .map(|member| member.from_joint_slip_m.abs().max(member.to_joint_slip_m.abs()))
        .fold(0.0_f64, f64::max);
    let total_joint_energy: f64 = compliant
        .members
        .iter()
        .map(|member| member.joint_strain_energy_j)
        .sum();
    println!(
        "finite_joint_max_displacement_m={:.12e}",
        compliant_max_displacement
    );
    println!(
        "finite_joint_displacement_ratio={:.9}",
        compliant_max_displacement / max_displacement
    );
    println!("maximum_end_joint_slip_m={:.12e}", maximum_joint_slip);
    println!("total_joint_strain_energy_j={:.12e}", total_joint_energy);

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
    println!(
        "mass_matched_max_euler_utilization={:?}",
        mass_matched.maximum_elastic_euler_utilization
    );
    println!(
        "mass_matched_single_removal_solvability={:.6}",
        mass_matched.single_member_removal_solvability_fraction
    );
    println!("mass_matched_pair_removal_tested={}", mass_matched.tested_two_member_pairs);
    println!("mass_matched_pair_removal_omitted={}", mass_matched.omitted_two_member_pairs);
    println!(
        "mass_matched_input_digest_non_security={:016x}",
        mass_matched.input_digest_non_security
    );
    println!(
        "mass_matched_global_force_residual_n={:.12e}",
        mass_matched.global_force_residual_n
    );
    println!(
        "mass_matched_global_moment_residual_nm={:.12e}",
        mass_matched.global_moment_residual_nm
    );
    println!(
        "mass_matched_energy_relative_error={:.12e}",
        mass_matched.energy_relative_error
    );

    let matched = matched_tetrahedron_pair(1.0, [0.0, 0.0, -100.0])
        .expect("valid matched-envelope tetrahedron fixture");
    let matched_spec = MassNormalizedBenchmarkSpec {
        member_mass_budget_kg: 10.0,
        material_density_kg_m3: 2_700.0,
        youngs_modulus_pa: 69.0e9,
        yield_strength_pa: 250.0e6,
        effective_length_factor: 1.0,
        pair_removal_budget: 8,
    };
    let matched_baseline = evaluate_mass_normalized_candidate(
        &matched.baseline.model,
        &matched.baseline.boundary,
        matched_spec,
    )
    .expect("baseline topology should produce benchmark receipt");
    let matched_hub = evaluate_mass_normalized_candidate(
        &matched.interior_hub.model,
        &matched.interior_hub.boundary,
        matched_spec,
    )
    .expect("interior-hub topology should produce benchmark receipt");
    println!("matched_baseline_name={}", matched.baseline.name);
    println!("matched_baseline_member_count={}", matched_baseline.member_count);
    println!(
        "matched_baseline_member_mass_kg={:.9}",
        matched_baseline.computed_member_mass_kg
    );
    println!(
        "matched_baseline_area_m2={:.12e}",
        matched_baseline.uniform_cross_section_area_m2
    );
    println!(
        "matched_baseline_displacement_m={:.12e}",
        matched_baseline.maximum_node_displacement_m
    );
    println!(
        "matched_baseline_max_stress_pa={:.12e}",
        matched_baseline.maximum_absolute_axial_stress_pa
    );
    println!(
        "matched_baseline_yield_utilization={:.12e}",
        matched_baseline.maximum_yield_utilization
    );
    println!("matched_hub_name={}", matched.interior_hub.name);
    println!("matched_hub_member_count={}", matched_hub.member_count);
    println!(
        "matched_hub_member_mass_kg={:.9}",
        matched_hub.computed_member_mass_kg
    );
    println!(
        "matched_hub_area_m2={:.12e}",
        matched_hub.uniform_cross_section_area_m2
    );
    println!(
        "matched_hub_displacement_m={:.12e}",
        matched_hub.maximum_node_displacement_m
    );
    println!(
        "matched_hub_max_stress_pa={:.12e}",
        matched_hub.maximum_absolute_axial_stress_pa
    );
    println!(
        "matched_hub_yield_utilization={:.12e}",
        matched_hub.maximum_yield_utilization
    );
    println!(
        "matched_hub_to_baseline_displacement_ratio={:.9}",
        matched_hub.maximum_node_displacement_m / matched_baseline.maximum_node_displacement_m
    );
    println!("claim_ceiling=ideal pin-jointed linear-elastic reference only; member mass only");

    let frame_material = BeamMaterial3D {
        youngs_modulus_pa: 200.0e9,
        shear_modulus_pa: 80.0e9,
    };
    let frame_section = BeamSection3D {
        area_m2: 0.01,
        torsional_constant_m4: 1.0e-5,
        second_moment_y_m4: 1.0e-5,
        second_moment_z_m4: 2.0e-5,
    };
    let frame_model = FrameModel3D {
        nodes: vec![Vec3::new(0.0, 0.0, 0.0), Vec3::new(2.0, 0.0, 0.0)],
        members: vec![FrameMember3D {
            from: 0,
            to: 1,
            orientation_hint_global: Vec3::new(0.0, 1.0, 0.0),
            material: frame_material,
            section: frame_section,
        }],
    };
    let mut frame_boundary = FrameBoundaryConditions::new(frame_model.nodes.len());
    frame_boundary.fix_node(0).expect("cantilever root support");
    frame_boundary
        .add_load(1, [0.0, 1_000.0, 0.0, 0.0, 0.0, 0.0])
        .expect("finite cantilever tip force");
    let frame_result = frame_model
        .analyze(&frame_boundary)
        .expect("supported 3D beam cantilever should solve");
    let frame_audit = audit_frame_equilibrium(&frame_model, &frame_boundary, &frame_result)
        .expect("frame equilibrium audit should be finite");
    println!("frame_element_count={}", frame_result.members.len());
    println!(
        "frame_tip_y_displacement_m={:.12e}",
        frame_result.nodal_displacements[1][1]
    );
    println!(
        "frame_tip_z_rotation_rad={:.12e}",
        frame_result.nodal_displacements[1][5]
    );
    println!(
        "frame_global_force_residual_n={:.12e}",
        frame_audit.net_force_residual_norm_n
    );
    println!(
        "frame_global_moment_residual_nm={:.12e}",
        frame_audit.net_moment_residual_norm_nm
    );
    println!(
        "frame_energy_relative_error={:.12e}",
        frame_audit.energy_relative_error
    );
    println!("frame_claim_ceiling=linear Euler-Bernoulli frame reference; no global buckling qualification");
}
