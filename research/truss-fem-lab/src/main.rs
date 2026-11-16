use symtropy_freeflow_structure_lab::tetrahedral::{TetrahedralAssembly, TetrahedralConfig};
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
    println!("claim_ceiling=ideal pin-jointed linear-elastic truss reference only");
}
