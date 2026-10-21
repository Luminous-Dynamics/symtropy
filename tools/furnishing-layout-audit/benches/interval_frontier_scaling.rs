//! Scaling probe for the interval-valued guaranteed-dominance frontier.
//!
//! Both the pairwise reference and the public API produce per-candidate report
//! rows; this is a fuller comparison than timing a frontier-ID-only kernel.
//! Candidate construction is excluded. Timings are synthetic and non-SLA.

use furnishing_layout_audit::alternative_frontier::{
    ObjectiveDirection, ObjectiveEvidence, VerificationStatus,
};
use furnishing_layout_audit::interval_frontier::{
    compute_interval_frontier, IntervalAlternativeCandidate, IntervalCandidateDisposition,
    IntervalCandidateResult, IntervalFrontierReport, IntervalObjectiveObservation,
    IntervalSemantics, IntervalSemanticsKind,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn objective(
    dimension: &str,
    direction: ObjectiveDirection,
    lower: f64,
    upper: f64,
) -> IntervalObjectiveObservation {
    IntervalObjectiveObservation {
        dimension: dimension.to_owned(),
        unit: if dimension == "install_cost" {
            "USD".to_owned()
        } else {
            "synthetic_unit".to_owned()
        },
        direction,
        lower,
        upper,
        semantics: IntervalSemantics {
            kind: IntervalSemanticsKind::DeterministicEnvelope,
            basis_id: "interval-frontier-synthetic-envelope-v1".to_owned(),
        },
        evidence: ObjectiveEvidence::AcceptedByPolicy,
    }
}

fn candidate(
    index: usize,
    count: usize,
    profile: &str,
    dimensions: usize,
) -> IntervalAlternativeCandidate {
    let mut objectives = vec![
        objective(
            "install_cost",
            ObjectiveDirection::Minimize,
            (index * 10) as f64,
            (index * 10 + 2) as f64,
        ),
        objective(
            "task_coverage",
            ObjectiveDirection::Maximize,
            match profile {
                "dominated-chain" => (1_000_000 - index * 10) as f64,
                "tradeoff-heavy" => (index * 10) as f64,
                _ => unreachable!("only fixed benchmark profiles are used"),
            },
            match profile {
                "dominated-chain" => (1_000_000 - index * 10 + 2) as f64,
                "tradeoff-heavy" => (index * 10 + 2) as f64,
                _ => unreachable!("only fixed benchmark profiles are used"),
            },
        ),
    ];

    for axis in 2..dimensions {
        let direction = if axis % 2 == 0 {
            ObjectiveDirection::Minimize
        } else {
            ObjectiveDirection::Maximize
        };
        let (lower, upper) = match (profile, direction) {
            ("dominated-chain", ObjectiveDirection::Minimize) => {
                ((index * 10 + axis) as f64, (index * 10 + axis + 2) as f64)
            }
            ("dominated-chain", ObjectiveDirection::Maximize) => (
                (1_000_000 - index * 10 - axis) as f64,
                (1_000_000 - index * 10 - axis + 2) as f64,
            ),
            ("tradeoff-heavy", _) => (100.0, 102.0),
            _ => unreachable!("only fixed benchmark profiles are used"),
        };
        objectives.push(objective(&format!("objective_{axis}"), direction, lower, upper));
    }

    let revision = format!("{:064x}", index + 1);
    IntervalAlternativeCandidate {
        candidate_id: format!("candidate-{index:05}"),
        revision_sha256: Some(revision.clone()),
        verifier_receipt_sha256: Some(format!("{:064x}", index + count + 1)),
        verified_revision_sha256: Some(revision),
        verification: VerificationStatus::Verified,
        objectives,
    }
}

fn guaranteed_dominates(
    left: &IntervalAlternativeCandidate,
    right: &IntervalAlternativeCandidate,
) -> bool {
    let mut strictly_better = false;
    for (a, b) in left.objectives.iter().zip(&right.objectives) {
        match a.direction {
            ObjectiveDirection::Minimize => {
                if a.upper > b.lower {
                    return false;
                }
                strictly_better |= a.upper < b.lower;
            }
            ObjectiveDirection::Maximize => {
                if a.lower < b.upper {
                    return false;
                }
                strictly_better |= a.lower > b.upper;
            }
        }
    }
    strictly_better
}

/// Independent O(n²) reference that produces a complete report, not just IDs.
/// It may choose a different valid witness than the incremental implementation.
fn pairwise_full_report(
    candidates: &[IntervalAlternativeCandidate],
) -> IntervalFrontierReport {
    let mut ordered = candidates.iter().collect::<Vec<_>>();
    ordered.sort_by_key(|candidate| candidate.candidate_id.clone());
    let mut frontier_ids = Vec::new();
    let mut results = Vec::with_capacity(ordered.len());

    for target in &ordered {
        let witness = ordered
            .iter()
            .find(|other| {
                other.candidate_id != target.candidate_id && guaranteed_dominates(other, target)
            })
            .map(|other| other.candidate_id.clone());
        let disposition = match witness {
            Some(witness_id) => IntervalCandidateDisposition::GuaranteedDominatedBy(witness_id),
            None => {
                frontier_ids.push(target.candidate_id.clone());
                IntervalCandidateDisposition::OnFrontier
            }
        };
        results.push(IntervalCandidateResult {
            candidate_id: target.candidate_id.clone(),
            revision_sha256: target.revision_sha256.clone(),
            verifier_receipt_sha256: target.verifier_receipt_sha256.clone(),
            verified_revision_sha256: target.verified_revision_sha256.clone(),
            disposition,
        });
    }

    IntervalFrontierReport {
        frontier_ids,
        candidates: results,
        ranking_complete: !ordered.is_empty(),
        relation_name: "guaranteed_interval_dominance_v0_1",
        claim_ceiling: "Synthetic pairwise reference only; intervals and receipts are caller supplied.",
    }
}

fn validate_witnesses(
    input: &[IntervalAlternativeCandidate],
    report: &IntervalFrontierReport,
) {
    for result in &report.candidates {
        if let IntervalCandidateDisposition::GuaranteedDominatedBy(witness_id) =
            &result.disposition
        {
            let witness = input
                .iter()
                .find(|candidate| candidate.candidate_id == *witness_id)
                .expect("witness ID must reference an input candidate");
            let target = input
                .iter()
                .find(|candidate| candidate.candidate_id == result.candidate_id)
                .expect("report row must reference an input candidate");
            assert!(
                guaranteed_dominates(witness, target),
                "invalid witness {witness_id} for {}",
                result.candidate_id
            );
        }
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

fn baseline_iterations(count: usize) -> usize {
    match count {
        25 => 30,
        100 => 15,
        1_000 => 5,
        _ => 3,
    }
}

fn api_iterations(count: usize) -> usize {
    match count {
        25 => 50,
        100 => 30,
        1_000 => 10,
        _ => 5,
    }
}

fn run_profile(count: usize, dimensions: usize, profile: &str) {
    let input = (0..count)
        .map(|index| candidate(index, count, profile, dimensions))
        .collect::<Vec<_>>();
    let oracle = pairwise_full_report(&input);
    let initial = compute_interval_frontier(&input).expect("synthetic schema is valid");
    assert_eq!(initial.frontier_ids, oracle.frontier_ids);
    validate_witnesses(&input, &oracle);
    validate_witnesses(&input, &initial);
    assert_eq!(initial.ranking_complete, oracle.ranking_complete);
    for (actual, expected) in initial.candidates.iter().zip(&oracle.candidates) {
        assert_eq!(actual.candidate_id, expected.candidate_id);
        assert_eq!(
            matches!(actual.disposition, IntervalCandidateDisposition::OnFrontier),
            matches!(expected.disposition, IntervalCandidateDisposition::OnFrontier),
            "disposition mismatch for {}",
            actual.candidate_id
        );
    }

    let baseline_n = baseline_iterations(count);
    let mut baseline_samples = Vec::with_capacity(baseline_n);
    for _ in 0..baseline_n {
        let start = Instant::now();
        let report = black_box(pairwise_full_report(black_box(&input)));
        baseline_samples.push(start.elapsed());
        assert_eq!(report.frontier_ids, oracle.frontier_ids);
        validate_witnesses(&input, &report);
    }

    let api_n = api_iterations(count);
    let mut api_samples = Vec::with_capacity(api_n);
    for _ in 0..api_n {
        let start = Instant::now();
        let report = black_box(compute_interval_frontier(black_box(&input)))
            .expect("synthetic schema stays valid");
        api_samples.push(start.elapsed());
        assert_eq!(report.frontier_ids, oracle.frontier_ids);
        validate_witnesses(&input, &report);
    }

    let baseline_p50 = percentile(&baseline_samples, 1, 2);
    let baseline_p95 = percentile(&baseline_samples, 95, 100);
    let api_p50 = percentile(&api_samples, 1, 2);
    let api_p95 = percentile(&api_samples, 95, 100);
    let baseline_max = baseline_samples.iter().copied().max().unwrap_or_default();
    let api_max = api_samples.iter().copied().max().unwrap_or_default();
    println!(
        "interval_frontier_bench dimensions={dimensions} profile={profile} n={count}          baseline_samples={baseline_n} pairwise_full_report_p50_ms={:.3}          pairwise_full_report_p95_ms={:.3} pairwise_full_report_max_ms={:.3}          api_samples={api_n} adaptive_api_p50_ms={:.3} adaptive_api_p95_ms={:.3}          adaptive_api_max_ms={:.3} frontier={}",
        baseline_p50.as_secs_f64() * 1000.0,
        baseline_p95.as_secs_f64() * 1000.0,
        baseline_max.as_secs_f64() * 1000.0,
        api_p50.as_secs_f64() * 1000.0,
        api_p95.as_secs_f64() * 1000.0,
        api_max.as_secs_f64() * 1000.0,
        initial.frontier_ids.len(),
    );
}

fn main() {
    println!(
        "Interval frontier benchmark: same synthetic inputs and complete report shape;          pairwise witness identity may differ but every witness is validated. No SLA claim."
    );
    for count in [25_usize, 100, 1_000, 10_000] {
        for dimensions in [2_usize, 4, 6, 8] {
            for profile in ["dominated-chain", "tradeoff-heavy"] {
                run_profile(count, dimensions, profile);
            }
        }
    }
}
