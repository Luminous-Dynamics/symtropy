use std::env;

use holofrac_spatial_retrieval::privacy_attack::{
    run_synthetic_basis_query_attack, PRIVACY_ATTACK_GENERATOR, PRIVACY_ATTACK_SCHEMA_VERSION,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    let report = run_synthetic_basis_query_attack()?;

    println!("# HoloFrac anonymous preference chosen-query reconstruction attack");
    println!("# schema_version={PRIVACY_ATTACK_SCHEMA_VERSION}");
    println!("# generator={PRIVACY_ATTACK_GENERATOR}");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!("# cohort_size={}", report.cohort_size);
    println!("# query_count={}", report.query_count);
    println!("# reconstruction_complete={}", report.reconstruction_complete);
    println!("# max_absolute_error={:.12}", report.max_absolute_error);
    println!("# mean_absolute_error={:.12}", report.mean_absolute_error);
    println!("# recovered_weight_sum={:.12}", report.recovered_weight_sum);
    println!("criterion,expected_mean_weight,recovered_mean_weight,absolute_error");

    for row in &report.recovered {
        println!(
            "{},{:.12},{:.12},{:.12}",
            row.criterion.as_str(),
            row.expected_mean_weight,
            row.recovered_mean_weight,
            row.absolute_error
        );
    }

    Ok(())
}
