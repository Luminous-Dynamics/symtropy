//! Policy-level preference evaluation scaling probe.
//!
//! All reachable leaves are enumerated. No answer probabilities are assigned;
//! per-leaf medians are not expected-value estimates.

use furnishing_layout_audit::alternative_frontier::VerificationStatus;
use furnishing_layout_audit::resident_preference_probe::{
    PairwisePreferenceQuestion, PreferenceAlternative, PreferenceScenario,
    ResidentPreferenceProbeRequest,
    response_robustness::{
        BoundedPreferenceResponseChannel,
        policy_evaluation::{QuestionPolicy, evaluate_question_policy},
    },
};
use std::hint::black_box;
use std::time::{Duration, Instant};

fn alternative(id: &str, scores: &[u16], index: usize) -> PreferenceAlternative {
    let revision = format!("{:064x}", index + 1);
    PreferenceAlternative {
        candidate_id: id.to_owned(),
        revision_sha256: revision.clone(),
        verifier_receipt_sha256: format!("{:064x}", index + 101),
        verified_revision_sha256: revision,
        hard_feasibility: VerificationStatus::Verified,
        criterion_scores_bps: scores.to_vec(),
    }
}

fn question(id: &str, left: &str, right: &str) -> PairwisePreferenceQuestion {
    PairwisePreferenceQuestion {
        question_id: id.to_owned(),
        left_candidate_id: left.to_owned(),
        right_candidate_id: right.to_owned(),
        query_cost_units: 1,
        query_duration_units: 1,
    }
}

fn request() -> ResidentPreferenceProbeRequest {
    let alternatives = vec![
        alternative("low-cost", &[10_000, 0], 0),
        alternative("high-support", &[0, 10_000], 1),
        alternative("balanced", &[5_500, 5_500], 2),
        alternative("space-efficient", &[8_000, 4_000], 3),
    ];
    let mut questions = Vec::new();
    for left in 0..alternatives.len() {
        for right in (left + 1)..alternatives.len() {
            questions.push(question(
                &format!("pair-{:02}-{:02}", left, right),
                &alternatives[left].candidate_id,
                &alternatives[right].candidate_id,
            ));
        }
    }
    ResidentPreferenceProbeRequest {
        criterion_ids: vec!["affordability".to_owned(), "task_support".to_owned()],
        query_cost_unit: "effort_points".to_owned(),
        query_duration_unit: "synthetic_seconds".to_owned(),
        indifference_margin_units: 0,
        alternatives,
        scenarios: vec![
            PreferenceScenario {
                scenario_id: "train-cost".to_owned(),
                weights_bps: vec![10_000, 0],
            },
            PreferenceScenario {
                scenario_id: "train-support".to_owned(),
                weights_bps: vec![0, 10_000],
            },
            PreferenceScenario {
                scenario_id: "train-cost-heavy".to_owned(),
                weights_bps: vec![7_000, 3_000],
            },
            PreferenceScenario {
                scenario_id: "train-support-heavy".to_owned(),
                weights_bps: vec![3_000, 7_000],
            },
        ],
        questions,
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

fn run_matrix_case(
    input: &ResidentPreferenceProbeRequest,
    channel: &BoundedPreferenceResponseChannel,
    held_out: &PreferenceScenario,
    policy: QuestionPolicy,
    budget: usize,
) {
    let expected =
        evaluate_question_policy(input, channel, held_out, policy, budget, 37)
            .expect("frozen policy evaluation");
    let mut timings = Vec::with_capacity(5);
    for _ in 0..5 {
        let start = Instant::now();
        let report = black_box(evaluate_question_policy(
            black_box(input),
            black_box(channel),
            black_box(held_out),
            policy,
            budget,
            37,
        ))
        .expect("policy request remains valid");
        timings.push(start.elapsed());
        assert_eq!(report, expected, "policy trace must replay deterministically");
    }
    println!(
        "policy_eval policy={policy:?} budget={budget} held_out={} channel={:?} leaves={} resolved={} unresolved={} worst_resolved_regret={:?} lower_median_resolved_regret={:?} max_questions={} nodes={} repetitions={} p50_us={} p95_us={} max_us={}",
        held_out.scenario_id,
        channel.allowed_observed_mask_by_true_outcome,
        expected.leaves.len(),
        expected.resolved_leaf_count,
        expected.unresolved_leaf_count,
        expected.worst_resolved_leaf_regret_units,
        expected.lower_median_resolved_leaf_regret_units,
        expected.max_questions_used,
        expected.expansion_nodes_visited,
        timings.len(),
        percentile(&timings, 1, 2).as_micros(),
        percentile(&timings, 95, 100).as_micros(),
        timings.iter().max().expect("samples are non-empty").as_micros(),
    );
}

fn main() {
    let input = request();
    let held_out_models = [
        PreferenceScenario {
            scenario_id: "heldout-balanced".to_owned(),
            weights_bps: vec![5_000, 5_000],
        },
        PreferenceScenario {
            scenario_id: "heldout-support-leaning".to_owned(),
            weights_bps: vec![2_000, 8_000],
        },
    ];
    let channels = [
        BoundedPreferenceResponseChannel::exact(),
        BoundedPreferenceResponseChannel {
            allowed_observed_mask_by_true_outcome: [0b0011, 0b0110, 0b1100],
        },
    ];
    for channel in &channels {
        for held_out in &held_out_models {
            for policy in QuestionPolicy::ALL {
                for budget in [1, 2, 3, 6] {
                    run_matrix_case(&input, channel, held_out, policy, budget);
                }
            }
        }
    }
}
