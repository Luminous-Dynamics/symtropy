//! Dependency-free scaling probe for evidence-qualified Pareto ranking.
//!
//! Candidate construction is outside the timed section. The tradeoff-heavy
//! profile stresses the worst-case non-dominated set; the dominated-chain
//! profile checks that a stable dominance witness avoids quadratic report
//! memory when most candidates are dominated.

use furnishing_layout_audit::alternative_frontier::{
    compute_pareto_frontier, AlternativeCandidate, ObjectiveDirection, ObjectiveEvidence,
    ObjectiveObservation, VerificationStatus,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn candidate(index: usize, count: usize, profile: &str) -> AlternativeCandidate {
    let (cost, coverage) = match profile {
        "tradeoff-heavy" => (index as f64 + 1.0, index as f64 / count.max(1) as f64),
        "dominated-chain" => (index as f64 + 1.0, 1.0 / (index as f64 + 1.0)),
        _ => unreachable!("only named benchmark profiles are used"),
    };
    AlternativeCandidate {
        candidate_id: format!("candidate-{index:05}"),
        revision_sha256: Some(format!("{:064x}", index + 1)),
        verifier_receipt_sha256: Some(format!("{:064x}", index + count + 1)),
        verified_revision_sha256: Some(format!("{:064x}", index + 1)),
        verification: VerificationStatus::Verified,
        objectives: vec![
            ObjectiveObservation {
                dimension: "install_cost".to_owned(),
                unit: "USD".to_owned(),
                direction: ObjectiveDirection::Minimize,
                value: cost,
                evidence: ObjectiveEvidence::AcceptedByPolicy,
            },
            ObjectiveObservation {
                dimension: "task_coverage".to_owned(),
                unit: "ratio".to_owned(),
                direction: ObjectiveDirection::Maximize,
                value: coverage,
                evidence: ObjectiveEvidence::AcceptedByPolicy,
            },
        ],
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

fn run_profile(count: usize, profile: &str, iterations: usize) {
    let candidates = (0..count)
        .map(|index| candidate(index, count, profile))
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
        "frontier_bench profile={profile} n={count} samples={iterations} \
         p50_ms={:.3} p95_ms={:.3} max_ms={:.3} frontier={expected_frontier}",
        p50.as_secs_f64() * 1000.0,
        p95.as_secs_f64() * 1000.0,
        maximum.as_secs_f64() * 1000.0,
    );
}

fn main() {
    println!("Benchmark input construction is excluded from timed measurements.");
    for count in [25_usize, 100, 1_000, 10_000] {
        let iterations = match count {
            25 => 50,
            100 => 30,
            1_000 => 10,
            _ => 5,
        };
        run_profile(count, "tradeoff-heavy", iterations);
        run_profile(count, "dominated-chain", iterations);
    }
}
