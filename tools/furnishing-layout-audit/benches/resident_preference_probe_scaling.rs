//! Scaling probe for regret-guided resident preference question selection.
//!
//! Candidate scores and explicit plausible preference scenarios are generated
//! outside the timed section. This records the complete public report path,
//! including validation, query partitions, minimax regret, and query Pareto set.
//! Synthetic results do not establish human-preference accuracy or an SLA.

use furnishing_layout_audit::alternative_frontier::VerificationStatus;
use furnishing_layout_audit::resident_preference_probe::{
    PairwisePreferenceQuestion, PreferenceAlternative, PreferenceScenario,
    ResidentPreferenceProbeRequest, compute_resident_preference_probe,
};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn candidate(index: usize, criteria: usize, count: usize) -> PreferenceAlternative {
    let revision = format!("{:064x}", index + 1);
    let criterion_scores_bps = (0..criteria)
        .map(|criterion| {
            ((index * 1_973 + criterion * 3_821 + (index + 1) * (criterion + 3) * 101)
                % 10_001) as u16
        })
        .collect();
    PreferenceAlternative {
        candidate_id: format!("candidate-{index:03}"),
        revision_sha256: revision.clone(),
        verifier_receipt_sha256: format!("{:064x}", index + count + 1),
        verified_revision_sha256: revision,
        hard_feasibility: VerificationStatus::Verified,
        criterion_scores_bps,
    }
}

fn scenario(index: usize, criteria: usize) -> PreferenceScenario {
    let raw = (0..criteria)
        .map(|criterion| ((index * 613 + criterion * 997 + 173) % 1_000 + 1) as u64)
        .collect::<Vec<_>>();
    let sum = raw.iter().sum::<u64>();
    let mut weights_bps = raw
        .iter()
        .map(|weight| ((weight * 10_000) / sum) as u16)
        .collect::<Vec<_>>();
    let assigned = weights_bps.iter().map(|weight| u32::from(*weight)).sum::<u32>();
    let residual = 10_000_u32 - assigned;
    let last = weights_bps.last_mut().expect("benchmark criteria are non-empty");
    *last = (u32::from(*last) + residual) as u16;
    PreferenceScenario {
        scenario_id: format!("scenario-{index:03}"),
        weights_bps,
    }
}

fn question(left: usize, right: usize) -> PairwisePreferenceQuestion {
    PairwisePreferenceQuestion {
        question_id: format!("pair-{left:03}-{right:03}"),
        left_candidate_id: format!("candidate-{left:03}"),
        right_candidate_id: format!("candidate-{right:03}"),
        query_cost_units: ((left.abs_diff(right) % 7) + 1) as u32,
        query_duration_units: (((left * 3 + right * 7) % 11) + 1) as u32,
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

fn run_case(alternatives: usize, scenarios: usize, criteria: usize, samples: usize) {
    let candidate_rows = (0..alternatives)
        .map(|index| candidate(index, criteria, alternatives))
        .collect::<Vec<_>>();
    let scenario_rows = (0..scenarios)
        .map(|index| scenario(index, criteria))
        .collect::<Vec<_>>();
    let mut questions = Vec::with_capacity(alternatives * (alternatives - 1) / 2);
    for left in 0..alternatives {
        for right in (left + 1)..alternatives {
            questions.push(question(left, right));
        }
    }
    let request = ResidentPreferenceProbeRequest {
        criterion_ids: (0..criteria)
            .map(|index| format!("criterion-{index:02}"))
            .collect(),
        query_cost_unit: "effort_points".to_owned(),
        query_duration_unit: "synthetic_seconds".to_owned(),
        indifference_margin_units: 250_000,
        alternatives: candidate_rows,
        scenarios: scenario_rows,
        questions,
    };

    let expected = compute_resident_preference_probe(&request)
        .expect("synthetic workload must satisfy the probe contract");
    let mut timings = Vec::with_capacity(samples);
    for _ in 0..samples {
        let start = Instant::now();
        let report = black_box(compute_resident_preference_probe(black_box(&request)))
            .expect("same frozen request must remain valid");
        timings.push(start.elapsed());
        assert_eq!(report, expected, "frozen request must replay deterministically");
    }

    let p50 = percentile(&timings, 1, 2);
    let p95 = percentile(&timings, 95, 100);
    let maximum = timings.iter().copied().max().unwrap_or_default();
    println!(
        "resident_preference_probe_bench alternatives={alternatives} scenarios={scenarios}          criteria={criteria} questions={} indifference_margin_units={} samples={samples} p50_ms={:.3} p95_ms={:.3}          max_ms={:.3} baseline_regret_units={} frontier_questions={}",
        request.questions.len(),
        expected.indifference_margin_units,
        p50.as_secs_f64() * 1_000.0,
        p95.as_secs_f64() * 1_000.0,
        maximum.as_secs_f64() * 1_000.0,
        expected.baseline_minimax_regret_units,
        expected.frontier_question_ids.len(),
    );
}

fn main() {
    println!(
        "Synthetic exact finite-model query-selection benchmark. No resident data,          scenario probabilities, speedup claim, or service-level objective is implied."
    );
    run_case(8, 8, 4, 15);
    run_case(16, 32, 8, 10);
    run_case(32, 64, 8, 5);
    run_case(48, 128, 16, 5);
}
