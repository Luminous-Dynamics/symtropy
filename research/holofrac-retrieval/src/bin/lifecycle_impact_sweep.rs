use std::env;

use holofrac_spatial_retrieval::lifecycle_impact::{
    evaluate_lifecycle_mutation, LifecycleMutation,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac lifecycle evidence-impact benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", env::consts::ARCH);
    println!("# target_os={}", env::consts::OS);
    println!(
        "mutation,changed_artifact,stale_evidence_count,retained_evidence_count,revalidation_avoided,conservative_full_invalidation,stale_evidence_ids,retained_evidence_ids"
    );

    for mutation in LifecycleMutation::ALL {
        let report = evaluate_lifecycle_mutation(mutation, true)?;
        println!(
            "{},{},{},{},{},{},{},{}",
            mutation.as_str(),
            report.changed_artifact_id,
            report.stale_evidence_ids.len(),
            report.retained_evidence_ids.len(),
            report.evidence_revalidation_avoided,
            report.conservative_full_invalidation,
            report.stale_evidence_ids.join(";"),
            report.retained_evidence_ids.join(";")
        );
    }

    Ok(())
}
