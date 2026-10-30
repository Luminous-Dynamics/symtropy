use std::{process, env};

use civil_aero_benchmark::voi::{
    DecisionOption, DecisionProblem, Experiment, evaluate_evsi, report_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic EVSI demo rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    if env::args().len() != 1 {
        return Err("usage: voi_demo (runs a fixed synthetic decision problem)".to_owned());
    }

    let problem = DecisionProblem {
        problem_id: "two-state-design-choice".to_owned(),
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
                experiment_id: "perfect_information".to_owned(),
                outcome_ids: vec!["a".to_owned(), "b".to_owned()],
                cost_units: 1.0,
                likelihood_by_state: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            },
            Experiment {
                experiment_id: "noisy_information".to_owned(),
                outcome_ids: vec!["a_signal".to_owned(), "b_signal".to_owned()],
                cost_units: 1.0,
                likelihood_by_state: vec![vec![0.8, 0.2], vec![0.2, 0.8]],
            },
            Experiment {
                experiment_id: "uninformative".to_owned(),
                outcome_ids: vec!["coin_1".to_owned(), "coin_2".to_owned()],
                cost_units: 0.25,
                likelihood_by_state: vec![vec![0.5, 0.5], vec![0.5, 0.5]],
            },
            Experiment {
                experiment_id: "too_expensive".to_owned(),
                outcome_ids: vec!["a".to_owned(), "b".to_owned()],
                cost_units: 8.0,
                likelihood_by_state: vec![vec![1.0, 0.0], vec![0.0, 1.0]],
            },
        ],
    };

    let report = evaluate_evsi(&problem, 2.0)?;
    eprintln!(
        "synthetic finite-state EVSI demonstration only; probabilities are caller-supplied and not independently calibrated; this is not an aerodynamic result"
    );
    eprintln!(
        "prior_decision={} prior_expected_utility={:.6} selected_experiment={}",
        report.prior_best_decision_id,
        report.prior_best_expected_utility,
        report.selected_experiment_id.as_deref().unwrap_or("abstain")
    );
    print!("{}", report_to_csv(&report));
    Ok(())
}
