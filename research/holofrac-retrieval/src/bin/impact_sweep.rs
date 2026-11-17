use holofrac_spatial_retrieval::impact::{run_impact_benchmark, run_shared_cause_impact_benchmark};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let source_sha = std::env::var("HOLOFRAC_SOURCE_SHA").unwrap_or_else(|_| "UNSET".to_owned());
    println!("# HoloFrac exact evidence-impact benchmark v0");
    println!("# source_sha={source_sha}");
    println!("# target_arch={}", std::env::consts::ARCH);
    println!("# target_os={}", std::env::consts::OS);
    println!(
        "topology,artifacts,chain_length,changed_sources,dependencies,evidence,affected_artifacts,stale_evidence,retained_evidence,graph_build_us,closure_us,evidence_revalidation_avoided"
    );

    for artifact_count in [100_usize, 1_000, 10_000, 100_000] {
        for changed_chains in [1_usize, 10, 100] {
            let report = run_impact_benchmark(artifact_count, 20, changed_chains)?;
            println!(
                "{},{},{},{},{},{},{},{},{},{:.3},{:.3},{}",
                "independent_chains",
                report.artifact_count,
                20,
                report.changed_source_count,
                report.dependency_count,
                report.evidence_count,
                report.affected_artifact_count,
                report.stale_evidence_count,
                report.retained_evidence_count,
                report.graph_build_micros,
                report.closure_micros,
                report.evidence_revalidation_avoided
            );
        }
    }

    for project_count in [20_usize, 200, 2_000, 20_000] {
        for shared_change in [true, false] {
            let report = run_shared_cause_impact_benchmark(project_count, shared_change)?;
            println!(
                "{},{},{},{},{},{},{},{},{},{:.3},{:.3},{}",
                if shared_change {
                    "shared_cause"
                } else {
                    "local_project_change"
                },
                report.artifact_count,
                0,
                report.changed_source_count,
                report.dependency_count,
                report.evidence_count,
                report.affected_artifact_count,
                report.stale_evidence_count,
                report.retained_evidence_count,
                report.graph_build_micros,
                report.closure_micros,
                report.evidence_revalidation_avoided
            );
        }
    }
    Ok(())
}
