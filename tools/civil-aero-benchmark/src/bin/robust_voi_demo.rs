use std::process;

use civil_aero_benchmark::robust_voi::{DecisionModelEnsemble, evaluate_robust_evsi, report_to_csv};
use civil_aero_benchmark::voi::{DecisionOption, DecisionProblem, Experiment};

fn member(problem_id: &str, fragile_likelihood: Vec<Vec<f64>>) -> DecisionProblem {
    DecisionProblem {
        problem_id: problem_id.to_owned(),
        state_ids: vec!["state_a".to_owned(), "state_b".to_owned()],
        prior_probabilities: vec![0.5, 0.5],
        decisions: vec![
            DecisionOption {
                decision_id: "design_a".to_owned(),
                utility_by_state: vec![10.0, 0.0],
            },
            DecisionOption {
                decision_id: "design_b".to_owned(),
                utility_by_state: vec![0.0, 10.0],
            },
        ],
        experiments: vec![
            Experiment {
                experiment_id: "fragile_perfect_or_blind".to_owned(),
                outcome_ids: vec!["signal_a".to_owned(), "signal_b".to_owned()],
                cost_units: 1.0,
                likelihood_by_state: fragile_likelihood,
            },
            Experiment {
                experiment_id: "robust_noisy".to_owned(),
                outcome_ids: vec!["signal_a".to_owned(), "signal_b".to_owned()],
                cost_units: 0.4,
                likelihood_by_state: vec![vec![0.75, 0.25], vec![0.25, 0.75]],
            },
        ],
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic robust EVSI demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let ensemble = DecisionModelEnsemble {
        ensemble_id: "two-model-ambiguity-set".to_owned(),
        remaining_budget_units: 2.0,
        members: vec![
            member(
                "model_perfect",
                vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            ),
            member(
                "model_blind",
                vec![vec![0.5, 0.5], vec![0.5, 0.5]],
            ),
        ],
    };
    let report = evaluate_robust_evsi(&ensemble)?;
    eprintln!(
        "synthetic_robust_evsi_known_answer_only members={} selected={} criterion=maximin_worst_case_net_evsi",
        report.member_count,
        report.selected_experiment_id.as_deref().unwrap_or("abstain")
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
