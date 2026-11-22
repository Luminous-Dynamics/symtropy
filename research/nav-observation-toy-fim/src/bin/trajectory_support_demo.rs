use nav_observation_toy_fim::trajectory_support_eval::{
    evaluate_trajectory_service_trace, TrajectoryServiceRecord, TrajectoryServiceState,
    TrajectorySupport,
};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut records = Vec::new();
    for index in 0..900_u64 {
        records.push(TrajectoryServiceRecord {
            record_id: format!("measured-{index}"),
            cluster_id: "construction-site-run-1".to_owned(),
            timestamp_ns: index * 10_000_000,
            support_duration_ns: 10_000_000,
            reference_valid: true,
            support: TrajectorySupport::MeasuredQualified,
            service_state: TrajectoryServiceState::Normal,
            position_error_m: Some(0.08),
        });
    }
    for index in 900..950_u64 {
        records.push(TrajectoryServiceRecord {
            record_id: format!("propagated-{index}"),
            cluster_id: "construction-site-run-1".to_owned(),
            timestamp_ns: index * 10_000_000,
            support_duration_ns: 10_000_000,
            reference_valid: true,
            support: TrajectorySupport::DeadReckonedBounded,
            service_state: TrajectoryServiceState::Normal,
            position_error_m: Some(0.18),
        });
    }
    for index in 950..975_u64 {
        records.push(TrajectoryServiceRecord {
            record_id: format!("interpolated-{index}"),
            cluster_id: "construction-site-run-1".to_owned(),
            timestamp_ns: index * 10_000_000,
            support_duration_ns: 10_000_000,
            reference_valid: true,
            support: TrajectorySupport::Interpolated,
            service_state: TrajectoryServiceState::Degraded,
            position_error_m: Some(0.30),
        });
    }
    for index in 975..990_u64 {
        records.push(TrajectoryServiceRecord {
            record_id: format!("extrapolated-{index}"),
            cluster_id: "construction-site-run-1".to_owned(),
            timestamp_ns: index * 10_000_000,
            support_duration_ns: 10_000_000,
            reference_valid: true,
            support: TrajectorySupport::Extrapolated,
            service_state: TrajectoryServiceState::IntegrityUncertain,
            position_error_m: Some(0.45),
        });
    }
    for index in 990..1000_u64 {
        records.push(TrajectoryServiceRecord {
            record_id: format!("missing-{index}"),
            cluster_id: "construction-site-run-1".to_owned(),
            timestamp_ns: index * 10_000_000,
            support_duration_ns: 10_000_000,
            reference_valid: true,
            support: TrajectorySupport::Missing,
            service_state: TrajectoryServiceState::Unavailable,
            position_error_m: None,
        });
    }
    let result = evaluate_trajectory_service_trace(&records)?;
    println!("records={} clusters={} valid_reference_samples={}", result.record_count, result.distinct_cluster_count, result.valid_reference_samples);
    println!("official_coverage_ppm={:?} evidence_backed_coverage_ppm={:?} unqualified_output_coverage_ppm={:?}", result.official_pose_coverage_ppm, result.evidence_backed_pose_coverage_ppm, result.unqualified_output_coverage_ppm);
    println!("coverage_gate_met={:?} challenge_compatible_exponential_score={:?}", result.official_coverage_gate_met, result.challenge_compatible_exponential_score);
    println!("measured={} bounded_dead_reckoned={} interpolated={} extrapolated={} missing={}", result.measured_qualified_samples, result.dead_reckoned_bounded_samples, result.interpolated_samples, result.extrapolated_samples, result.missing_estimate_samples);
    println!("normal_without_qualified_support_samples={} missing_exposure_ns={} unqualified_output_exposure_ns={}", result.normal_without_qualified_support_samples, result.missing_estimate_exposure_ns, result.unqualified_output_exposure_ns);
    println!("error_p50_um={:?} p95_um={:?} p99_um={:?} max_um={:?}", result.all_estimate_errors.p50_micrometers, result.all_estimate_errors.p95_micrometers, result.all_estimate_errors.p99_micrometers, result.all_estimate_errors.maximum_micrometers);
    println!("scope=postmatched_synthetic_trajectory_support_evaluation_not_ros_association_or_physical_integrity");
    Ok(())
}
