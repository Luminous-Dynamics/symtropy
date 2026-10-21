//! Dependency-free scaling probe for evidence-qualified Pareto ranking.
//!
//! Candidate construction is outside the timed section. Tradeoff-heavy profiles
//! keep every candidate non-dominated; dominated-chain profiles keep one point
//! on the frontier. Three objectives exercise the new Fenwick-tree sweep while
//! four objectives exercise the exact pairwise fallback.

use furnishing_layout_audit::alternative_frontier::{
    AlternativeCandidate, ObjectiveDirection, ObjectiveEvidence, ObjectiveObservation,
    VerificationStatus, compute_pareto_frontier,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn observation(
    dimension: &str,
    unit: &str,
    direction: ObjectiveDirection,
    value: f64,
) -> ObjectiveObservation {
    ObjectiveObservation {
        dimension: dimension.to_owned(),
        unit: unit.to_owned(),
        direction,
        value,
        evidence: ObjectiveEvidence::AcceptedByPolicy,
    }
}

fn candidate(
    index: usize,
    count: usize,
    profile: &str,
    dimensions: usize,
) -> AlternativeCandidate {
    let cost = index as f64 + 1.0;
    let coverage = match profile {
        "tradeoff-heavy" => index as f64 / count.max(1) as f64,
        "dominated-chain" => 1.0 / (index as f64 + 1.0),
        _ => unreachable!("only named benchmark profiles are used"),
    };
    let area = match profile {
        "tradeoff-heavy" => (count - index) as f64,
        "dominated-chain" => index as f64 + 1.0,
        _ => unreachable!("only named benchmark profiles are used"),
    };
    let effort = match profile {
        "tradeoff-heavy" => (count - index) as f64,
        "dominated-chain" => index as f64 + 1.0,
        _ => unreachable!("only named benchmark profiles are used"),
    };

    let mut objectives = vec![
        observation("install_cost", "USD", ObjectiveDirection::Minimize, cost),
        observation("task_coverage", "ratio", ObjectiveDirection::Maximize, coverage),
    ];
    if dimensions >= 3 {
        objectives.push(observation(
            "occupied_area",
            "m2",
            ObjectiveDirection::Minimize,
            area,
        ));
    }
    if dimensions >= 4 {
        objectives.push(observation(
            "install_effort",
            "hours",
            ObjectiveDirection::Minimize,
            effort,
        ));
    }

    AlternativeCandidate {
        candidate_id: format!("candidate-{index:05}"),
        revision_sha256: Some(format!("{:064x}", index + 1)),
        verifier_receipt_sha256: Some(format!("{:064x}", index + count + 1)),
        verified_revision_sha256: Some(format!("{:064x}", index + 1)),
        verification: VerificationStatus::Verified,
        objectives,
    }
}

fn percentile(samples: &[Duration], numerator: usize, denominator: usize) -> Duration {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    let rank = sorted
        .len()
        .saturating_mul(numerator)
        .div_ceil(denominator)
        .max(1);
    sorted[rank - 1]
}

fn run_profile(count: usize, profile: &str, dimensions: usize, iterations: usize) {
    let candidates = (0..count)
        .map(|index| candidate(index, count, profile, dimensions))
        .collect::<Vec<_>>();

    let reference =
        compute_pareto_frontier(&candidates).expect("synthetic profile must be comparable");
    assert!(reference.ranking_complete);
    let expected_frontier = if profile == "tradeoff-heavy" {
        count
    } else {
        1
    };
    assert_eq!(reference.frontier_ids.len(), expected_frontier);

    let mut durations = Vec::with_capacity(iterations);
    for _ in 0..iterations {
        let start = Instant::now();
        let report = black_box(compute_pareto_frontier(black_box(&candidates)))
            .expect("candidate schema remains valid");
        durations.push(start.elapsed());
        assert_eq!(report.frontier_ids.len(), expected_frontier);
        assert!(report.ranking_complete);
    }

    let p50 = percentile(&durations, 1, 2);
    let p95 = percentile(&durations, 95, 100);
    let maximum = durations.iter().copied().max().unwrap_or_default();
    println!(
        "frontier_bench dimensions={dimensions} profile={profile} n={count} samples={iterations} \
         p50_ms={:.3} p95_ms={:.3} max_ms={:.3} frontier={expected_frontier}",
        p50.as_secs_f64() * 1000.0,
        p95.as_secs_f64() * 1000.0,
        maximum.as_secs_f64() * 1000.0,
    );
}

fn iterations_for(count: usize) -> usize {
    match count {
        25 => 50,
        100 => 30,
        1_000 => 10,
        _ => 5,
    }
}

fn main() {
    println!("Benchmark input construction is excluded from timed measurements.");
    for count in [25_usize, 100, 1_000, 10_000] {
        let iterations = iterations_for(count);
        for dimensions in [2_usize, 3] {
            run_profile(count, "tradeoff-heavy", dimensions, iterations);
            run_profile(count, "dominated-chain", dimensions, iterations);
        }
        // Keep the quadratic fallback benchmark tractable while still exposing
        // its dimensionality cost and checking the larger input size.
        if count <= 1_000 {
            run_profile(count, "tradeoff-heavy", 4, iterations);
            run_profile(count, "dominated-chain", 4, iterations);
        }
    }
}
