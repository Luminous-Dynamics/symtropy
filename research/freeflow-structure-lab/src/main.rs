use symtropy_freeflow_structure_lab::{Config, StructureGraph};

fn main() {
    let graph = StructureGraph::generate(Config::default()).expect("valid geometry fixture");
    let metrics = graph
        .audit()
        .expect("generated graph should pass topology audit");

    println!("Symtropy Free-Flow Modular Structure Lab");
    println!("nodes={}", metrics.node_count);
    println!("members={}", metrics.member_count);
    println!("connected_components={}", metrics.connected_components);
    println!("cycle_rank={}", metrics.cycle_rank);
    println!("maximum_node_degree={}", metrics.maximum_node_degree);
    println!("total_member_length={:.9}", metrics.total_member_length);
    println!("minimum_member_length={:.9}", metrics.minimum_member_length);
    println!("maximum_member_length={:.9}", metrics.maximum_member_length);
    println!(
        "bounds_min=({:.6},{:.6},{:.6})",
        metrics.bounds_min.x, metrics.bounds_min.y, metrics.bounds_min.z
    );
    println!(
        "bounds_max=({:.6},{:.6},{:.6})",
        metrics.bounds_max.x, metrics.bounds_max.y, metrics.bounds_max.z
    );
    println!(
        "digest_non_security={:016x}",
        metrics.same_build_digest_non_security
    );
    println!("claim_ceiling={}", metrics.claim_ceiling);
}
