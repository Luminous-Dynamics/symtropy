use std::env;

use holofrac_spatial_retrieval::composition::run_compositional_generalization_fixture;

fn main() {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    let report = run_compositional_generalization_fixture();

    println!("# HoloFrac held-out typed-module composition qualification v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!("metric,value");
    println!(
        "training_composition_count,{}",
        report.training_composition_count
    );
    println!("held_out_module_count,{}", report.held_out_module_count);
    println!(
        "held_out_connection_count,{}",
        report.held_out_connection_count
    );
    println!(
        "held_out_signature_unseen,{}",
        report.held_out_signature_unseen
    );
    println!(
        "all_held_out_pair_rules_seen,{}",
        report.all_held_out_pair_rules_seen
    );
    println!(
        "exact_port_contract_valid,{}",
        report.exact_port_contract_valid
    );
    println!("violation_count,{}", report.violation_count);
}
