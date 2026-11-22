use nav_observation_toy_fim::clustered_risk::{
    cluster_bootstrap_false_safe, ClusterEvaluationCase,
};
use nav_observation_toy_fim::risk_coverage::EvaluationCase;

fn item(id: &str, cluster: &str, confidence: u32, claim: bool, truth: bool, seconds: u64) -> ClusterEvaluationCase {
    ClusterEvaluationCase {
        case: EvaluationCase {
            record_id: id.to_owned(),
            confidence_ppm: confidence,
            claim_state_one: claim,
            truth_state_one: truth,
            exposure_seconds: seconds,
        },
        cluster_id: cluster.to_owned(),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let records = vec![
        item("site-a-event-1-window-1", "site-a-event-1", 950_000, true, false, 60),
        item("site-a-event-1-window-2", "site-a-event-1", 940_000, true, false, 60),
        item("site-a-event-1-window-3", "site-a-event-1", 930_000, true, false, 60),
        item("site-b-event-2-window-1", "site-b-event-2", 950_000, true, true, 300),
        item("site-c-event-3-window-1", "site-c-event-3", 950_000, false, true, 120),
        item("site-d-event-4-window-1", "site-d-event-4", 950_000, true, true, 180),
    ];
    let result = cluster_bootstrap_false_safe(&records, 900_000, 2000, 20261122)?;
    println!("algorithm={}", result.algorithm);
    println!("clusters={} clusters_with_coverage={}", result.cluster_count, result.clusters_with_covered_records);
    println!("covered_records={} false_safe_records={} false_safe_rate_ppm={:?}", result.covered_records, result.false_safe_records, result.false_safe_rate_ppm);
    println!("valid_replicates={} zero_coverage_replicates={} seed={}", result.valid_replicates, result.zero_coverage_replicates, result.seed);
    println!("record_rate_cluster_ci={:?}", result.record_rate_interval);
    println!("exposure_rate_cluster_ci={:?}", result.exposure_rate_interval);
    println!("scope=declared_single_cluster_axis_nonparametric_bootstrap_not_integrity_certificate");
    Ok(())
}
