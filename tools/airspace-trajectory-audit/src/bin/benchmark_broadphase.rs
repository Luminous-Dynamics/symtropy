//! Deterministic scalability probe for the synthetic trajectory broadphase.
//!
//! Emits CSV measurements for a paired-path workload at 10, 100, 1,000 and 10,000
//! trajectories. This is a reproducible fixture, not an operational fleet or SOTA claim.

use std::collections::BTreeSet;
use std::time::Instant;

use symtropy_airspace_trajectory_audit::{
    ConflictDisposition, SeparationCriteria, Trajectory, TrajectoryPoint, audit_trajectory_pair,
    broadphase_candidate_pairs,
};

fn point(time_utc_ms: i64, x_m: f64, y_m: f64, z_m: f64) -> TrajectoryPoint {
    TrajectoryPoint {
        time_utc_ms,
        x_m,
        y_m,
        z_m,
    }
}

fn synthetic_fleet(count: usize) -> Vec<Trajectory> {
    let mut fleet = Vec::with_capacity(count);
    for index in 0..count {
        let group = index / 2;
        let side = index % 2;
        let x = group as f64 * 500.0;
        let (start_x, end_x, y) = if side == 0 {
            (x, x + 100.0, 0.0)
        } else {
            (x + 100.0, x, 10.0)
        };
        fleet.push(Trajectory {
            trajectory_id: format!("bench-{index:05}"),
            revision: "broadphase-benchmark-v1".to_owned(),
            reference_frame_id: "BENCH-LOCAL-XYZ-METRES".to_owned(),
            vertical_datum_id: "BENCH-VERTICAL-DATUM".to_owned(),
            source_reference: format!("fixture://broadphase-benchmark/{index:05}"),
            source_sha256: "a".repeat(64),
            horizontal_uncertainty_bound_m: (index % 3) as f64,
            vertical_uncertainty_bound_m: (index % 2) as f64,
            points: vec![
                point(1_000, start_x, y, 100.0),
                point(11_000, end_x, y, 100.0),
            ],
        });
    }
    fleet
}

fn criteria() -> SeparationCriteria {
    SeparationCriteria {
        criteria_id: "BENCH-CRITERIA-001".to_owned(),
        revision: "benchmark-v1".to_owned(),
        jurisdiction: "fictional-benchmark-only".to_owned(),
        source_reference: "fixture://broadphase-benchmark/criteria".to_owned(),
        source_sha256: "b".repeat(64),
        valid_from_utc_ms: 0,
        valid_until_utc_ms: 12_000,
        minimum_horizontal_separation_m: 100.0,
        minimum_vertical_separation_m: 50.0,
    }
}

fn percentile(sorted: &[u128], numerator: usize, denominator: usize) -> u128 {
    let index = (sorted.len() * numerator)
        .div_ceil(denominator)
        .saturating_sub(1);
    sorted[index.min(sorted.len() - 1)]
}

fn run_size(count: usize) -> Result<(), Box<dyn std::error::Error>> {
    assert_eq!(count % 2, 0, "fixture size must contain complete pairs");
    let fleet = synthetic_fleet(count);
    let rule = criteria();
    let all_pairs = count * (count - 1) / 2;
    let expected_conflict_pairs = count / 2;
    let mut elapsed_micros = Vec::with_capacity(11);
    let mut final_candidates = Vec::new();

    for _ in 0..11 {
        let start = Instant::now();
        let candidates = broadphase_candidate_pairs(&fleet, &rule)?;
        elapsed_micros.push(start.elapsed().as_micros());
        final_candidates = candidates;
    }

    let candidate_set: BTreeSet<(usize, usize)> = final_candidates
        .iter()
        .map(|candidate| (candidate.first_index, candidate.second_index))
        .collect();
    let expected: BTreeSet<(usize, usize)> = (0..count)
        .step_by(2)
        .map(|index| (index, index + 1))
        .collect();
    assert_eq!(
        candidate_set, expected,
        "fixed paired-path fixture must preserve every expected conflict pair and prune separated groups"
    );
    assert_eq!(final_candidates.len(), expected_conflict_pairs);
    for &(first, second) in &expected {
        let report = audit_trajectory_pair(&fleet[first], &fleet[second], &rule)?;
        assert_eq!(
            report.disposition,
            ConflictDisposition::ConflictDetected,
            "fixed paired workload must contain a known conflict for ({first}, {second})"
        );
    }
    elapsed_micros.sort_unstable();
    let p50 = percentile(&elapsed_micros, 50, 100);
    let p95 = percentile(&elapsed_micros, 95, 100);
    let reduction = 100.0 * (1.0 - final_candidates.len() as f64 / all_pairs as f64);

    println!(
        "{count},{},{},{},{reduction:.6},{p50},{p95}",
        elapsed_micros.len(),
        final_candidates.len(),
        all_pairs,
    );
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("trajectories,repetitions,candidates,all_pairs,candidate_reduction_pct,p50_us,p95_us");
    for count in [10_usize, 100, 1_000, 10_000] {
        run_size(count)?;
    }
    Ok(())
}
