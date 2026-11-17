use symtropy_freeflow_structure_lab::assembly::plan_tetrahedral_assembly;
use symtropy_freeflow_structure_lab::tetrahedral::{TetrahedralAssembly, TetrahedralConfig};
use symtropy_freeflow_structure_lab::{Config, StructureGraph};

fn main() {
    println!("Symtropy Free-Flow Modular Structure Lab");
    println!("geometry/topology research only — no strength or flight qualification");

    let branch_graph =
        StructureGraph::generate(Config::default()).expect("valid branching geometry fixture");
    let branch_metrics = branch_graph
        .audit()
        .expect("branching graph should pass topology audit");
    println!("branching_motif.nodes={}", branch_metrics.node_count);
    println!("branching_motif.members={}", branch_metrics.member_count);
    println!("branching_motif.cycle_rank={}", branch_metrics.cycle_rank);
    println!(
        "branching_motif.connected_components={}",
        branch_metrics.connected_components
    );
    println!(
        "branching_motif.digest_non_security={:016x}",
        branch_metrics.same_build_digest_non_security
    );

    let tetrahedral = TetrahedralAssembly::generate(TetrahedralConfig::default())
        .expect("valid face-attached tetrahedral fixture");
    let tetra_metrics = tetrahedral
        .audit()
        .expect("tetrahedral assembly should pass geometry/topology audit");

    println!("tetrahedral.cells={}", tetra_metrics.cell_count);
    println!("tetrahedral.nodes={}", tetra_metrics.node_count);
    println!("tetrahedral.members={}", tetra_metrics.member_count);
    println!("tetrahedral.cycle_rank={}", tetra_metrics.cycle_rank);
    println!(
        "tetrahedral.connected_components={}",
        tetra_metrics.connected_components
    );
    println!(
        "tetrahedral.maximum_node_degree={}",
        tetra_metrics.maximum_node_degree
    );
    println!(
        "tetrahedral.total_member_length_m={:.9}",
        tetra_metrics.total_member_length_m
    );
    println!(
        "tetrahedral.minimum_face_area_m2={:.9}",
        tetra_metrics.minimum_attachment_face_area_m2
    );
    println!(
        "tetrahedral.minimum_cell_volume_m3={:.9}",
        tetra_metrics.minimum_cell_volume_m3
    );
    println!(
        "tetrahedral.digest_non_security={:016x}",
        tetra_metrics.same_build_digest_non_security
    );
    println!(
        "tetrahedral.ideal_attachment_contract={}",
        tetra_metrics.ideal_attachment_contract
    );
    println!("claim_ceiling={}", tetra_metrics.claim_ceiling);

    let plan =
        plan_tetrahedral_assembly(&tetrahedral).expect("valid parent-before-child assembly plan");
    println!("assembly_plan.root_cell_id={}", plan.root_cell_id);
    println!("assembly_plan.steps={}", plan.steps.len());
    println!("assembly_plan.maximum_generation={}", plan.maximum_generation);
    println!(
        "assembly_plan.blocked_attachment_faces={}",
        plan.blocked_attachment_faces
    );
    println!(
        "assembly_plan.digest_non_security={:016x}",
        plan.digest_non_security
    );
    println!("assembly_plan.claim_ceiling={}", plan.claim_ceiling);
}
