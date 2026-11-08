// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Emit per-episode, per-method receipts for the common-stream benchmark.

use civdyn_self_correction::sequential_methods::{
    MAX_PAIRS, SequentialDecision, SequentialEpisodeResult, SequentialMethod, SequentialRegime,
    run_sequential_method_portfolio, wilson_95,
};

const CSV_COLUMNS: usize = 14;

fn number(value: f64) -> String {
    format!("{value:.8}")
}

fn row(outcome: &SequentialEpisodeResult) -> Vec<String> {
    let result = outcome.receipt;
    vec![
        format!("{:016x}", outcome.seed),
        outcome.seed_index.to_string(),
        outcome.regime.name().to_string(),
        number(outcome.true_effect),
        number(outcome.observational_association),
        format!("{:016x}", outcome.stream_hash),
        result.method.name().to_string(),
        result.decision.name().to_string(),
        result.pairs_used.to_string(),
        number(result.mean_difference),
        number(result.z_harm),
        number(result.z_benefit),
        result.log_e_harm.map(number).unwrap_or_default(),
        result.log_e_benefit.map(number).unwrap_or_default(),
    ]
}

fn percentile(sorted: &[usize], fraction: f64) -> usize {
    if sorted.is_empty() {
        return 0;
    }
    let index = ((sorted.len() - 1) as f64 * fraction).ceil() as usize;
    sorted[index]
}

fn print_summary(outcomes: &[SequentialEpisodeResult]) {
    for method in SequentialMethod::ALL {
        let mut alternative_costs: Vec<usize> = outcomes
            .iter()
            .filter(|item| {
                item.receipt.method == method
                    && item.regime != SequentialRegime::ConfoundedNull
                    && item.regime != SequentialRegime::BenefitBoundary
            })
            .map(|item| item.receipt.pairs_used)
            .collect();
        alternative_costs.sort_unstable();

        let count = alternative_costs.len();
        let mean = alternative_costs.iter().sum::<usize>() as f64 / count as f64;
        let false_actions_zero = outcomes
            .iter()
            .filter(|item| {
                item.receipt.method == method
                    && item.regime == SequentialRegime::ConfoundedNull
                    && item.receipt.decision != SequentialDecision::Inconclusive
            })
            .count();
        let false_actions_boundary = outcomes
            .iter()
            .filter(|item| {
                item.receipt.method == method
                    && item.regime == SequentialRegime::BenefitBoundary
                    && item.receipt.decision != SequentialDecision::Inconclusive
            })
            .count();
        let zero_interval =
            wilson_95(false_actions_zero, 256).expect("256 zero-effect episodes are present");
        let boundary_interval = wilson_95(false_actions_boundary, 256)
            .expect("256 minimum-benefit boundary episodes are present");

        eprintln!(
            "method={} alternative_n={} mean_pairs={:.2} p50_pairs={} p95_pairs={} savings_vs_2048={:.2}% zero_effect_false_actions={}/256 wilson95=[{:.4},{:.4}] benefit_boundary_false_actions={}/256 wilson95=[{:.4},{:.4}]",
            method.name(),
            count,
            mean,
            percentile(&alternative_costs, 0.50),
            percentile(&alternative_costs, 0.95),
            100.0 * (1.0 - mean / MAX_PAIRS as f64),
            false_actions_zero,
            zero_interval.lower,
            zero_interval.upper,
            false_actions_boundary,
            boundary_interval.lower,
            boundary_interval.upper,
        );
    }
}

fn main() {
    println!(
        "seed,seed_index,regime,true_effect,observational_association,stream_hash,method,decision,pairs_used,mean_difference,z_harm,z_benefit,log_e_harm,log_e_benefit"
    );

    let outcomes = run_sequential_method_portfolio();
    assert_eq!(
        outcomes.len(),
        SequentialRegime::ALL.len() * 256 * SequentialMethod::ALL.len()
    );
    assert!(outcomes
        .iter()
        .all(|item| item.receipt.pairs_used <= MAX_PAIRS));

    for outcome in &outcomes {
        let fields = row(outcome);
        assert_eq!(fields.len(), CSV_COLUMNS);
        println!("{}", fields.join(","));
    }

    print_summary(&outcomes);
}
