//! Exact-versus-bounded-response preference analysis scaling probe.
//!
//! Requests and expected results are built outside the timed regions. Synthetic
//! timings do not establish user preference accuracy or a production SLA.

use furnishing_layout_audit::alternative_frontier::VerificationStatus;
use furnishing_layout_audit::resident_preference_probe::{
    PairwisePreferenceQuestion, PreferenceAlternative, PreferenceScenario,
    ResidentPreferenceProbeRequest, compute_resident_preference_probe,
    response_robustness::{BoundedPreferenceResponseChannel, compute_response_robustness},
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
    let input = ResidentPreferenceProbeRequest {
        criterion_ids: (0..criteria)
            .map(|index| format!("criterion-{index:02}"))
            .collect(),
        query_cost_unit: "effort_points".to_owned(),
        query_duration_unit: "synthetic_seconds".to_owned(),
        indifference_margin_units: 250_000,
        alternatives: (0..alternatives)
            .map(|index| candidate(index, criteria, alternatives))
            .collect(),
        scenarios: (0..scenarios)
            .map(|index| scenario(index, criteria))
            .collect(),
        questions: (0..alternatives)
            .flat_map(|left| {
                ((left + 1)..alternatives).map(move |right| question(left, right))
            })
            .collect(),
    };
    let exact_expected =
        compute_resident_preference_probe(&input).expect("frozen exact request");
    let channel = BoundedPreferenceResponseChannel {
        allowed_observed_mask_by_true_outcome: [0b0011, 0b0110, 0b1100],
    };
    let robust_expected =
        compute_response_robustness(&input, &channel).expect("frozen channel request");

    let mut exact_timings = Vec::with_capacity(samples);
    let mut robust_timings = Vec::with_capacity(samples);
    for sample in 0..samples {
        if sample % 2 == 0 {
            let start = Instant::now();
            let result = black_box(compute_resident_preference_probe(black_box(&input)))
                .expect("exact request remains valid");
            exact_timings.push(start.elapsed());
            assert_eq!(result, exact_expected, "exact report must replay deterministically");
            let start = Instant::now();
            let result = black_box(compute_response_robustness(
                black_box(&input),
                black_box(&channel),
            ))
            .expect("bounded response request remains valid");
            robust_timings.push(start.elapsed());
            assert_eq!(result, robust_expected, "robust report must replay deterministically");
        } else {
            let start = Instant::now();
            let result = black_box(compute_response_robustness(
                black_box(&input),
                black_box(&channel),
            ))
            .expect("bounded response request remains valid");
            robust_timings.push(start.elapsed());
            assert_eq!(result, robust_expected, "robust report must replay deterministically");
            let start = Instant::now();
            let result = black_box(compute_resident_preference_probe(black_box(&input)))
                .expect("exact request remains valid");
            exact_timings.push(start.elapsed());
            assert_eq!(result, exact_expected, "exact report must replay deterministically");
        }
    }

    println!(
        "response_robustness alternatives={alternatives} scenarios={scenarios} criteria={criteria} questions={} channel=bounded_set-valued samples={} exact_p50_us={} exact_p95_us={} exact_max_us={} robust_p50_us={} robust_p95_us={} robust_max_us={} baseline_regret_units={} worst_case_regret_reduction_sum_units={}",
        input.questions.len(),
        samples,
        percentile(&exact_timings, 1, 2).as_micros(),
        percentile(&exact_timings, 95, 100).as_micros(),
        exact_timings.iter().max().expect("samples are non-empty").as_micros(),
        percentile(&robust_timings, 1, 2).as_micros(),
        percentile(&robust_timings, 95, 100).as_micros(),
        robust_timings.iter().max().expect("samples are non-empty").as_micros(),
        robust_expected.baseline_minimax_regret_units,
        robust_expected
            .questions
            .iter()
            .map(|question| question.worst_case_regret_reduction_units)
            .sum::<u64>(),
    );
}

fn main() {
    run_case(8, 8, 4, 15);
    run_case(16, 32, 8, 10);
    run_case(32, 64, 8, 5);
    run_case(48, 128, 16, 5);
}
