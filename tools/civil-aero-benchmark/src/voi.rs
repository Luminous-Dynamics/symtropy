//! Finite-state expected value of sample information (EVSI) for experiment selection.
//!
//! This module evaluates caller-supplied priors, decision utilities and outcome likelihoods.
//! It does not calibrate those probabilities, infer them from evidence, or certify a design.
//! Its purpose is to test decision arithmetic and fixed-budget experiment selection separately
//! from any learned Symthaea acquisition policy.

use std::collections::HashSet;

const PROBABILITY_TOLERANCE: f64 = 1.0e-9;
const VALUE_TOLERANCE: f64 = 1.0e-12;

pub const EVSI_OUTPUT_HEADER: &str =
    "problem_id,experiment_id,cost_units,remaining_budget_units,affordable,prior_best_decision_id,prior_best_expected_utility,expected_best_utility_after_observation,gross_evsi,net_evsi,outcome_count,decision_changing_outcomes,selected,evidence_level";

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionOption {
    pub decision_id: String,
    /// Utility for each state, in the same order as DecisionProblem.state_ids.
    pub utility_by_state: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct Experiment {
    pub experiment_id: String,
    pub outcome_ids: Vec<String>,
    pub cost_units: f64,
    /// P(outcome | state), indexed [state][outcome].
    pub likelihood_by_state: Vec<Vec<f64>>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionProblem {
    pub problem_id: String,
    pub state_ids: Vec<String>,
    pub prior_probabilities: Vec<f64>,
    pub decisions: Vec<DecisionOption>,
    pub experiments: Vec<Experiment>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeValue {
    pub outcome_id: String,
    pub probability: f64,
    pub best_decision_id: Option<String>,
    pub best_posterior_expected_utility: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct ExperimentValue {
    pub problem_id: String,
    pub experiment_id: String,
    pub cost_units: f64,
    pub remaining_budget_units: f64,
    pub affordable: bool,
    pub prior_best_decision_id: String,
    pub prior_best_expected_utility: f64,
    pub expected_best_utility_after_observation: f64,
    pub gross_evsi: f64,
    pub net_evsi: f64,
    pub outcome_count: usize,
    pub decision_changing_outcomes: usize,
    pub selected: bool,
    pub outcomes: Vec<OutcomeValue>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvsiReport {
    pub problem_id: String,
    pub remaining_budget_units: f64,
    pub prior_best_decision_id: String,
    pub prior_best_expected_utility: f64,
    pub selected_experiment_id: Option<String>,
    pub experiments: Vec<ExperimentValue>,
}

/// Validate all IDs, matrix dimensions, probability mass, utility values and costs.
/// This deliberately rejects under-specified likelihood matrices instead of imputing values.
pub fn validate_problem(problem: &DecisionProblem, remaining_budget: f64) -> Result<(), String> {
    if problem.problem_id.trim().is_empty() {
        return Err("problem_id must not be empty".to_owned());
    }
    if !remaining_budget.is_finite() || remaining_budget < 0.0 {
        return Err("remaining_budget_units must be finite and non-negative".to_owned());
    }
    let state_count = problem.state_ids.len();
    if state_count == 0 || problem.prior_probabilities.len() != state_count {
        return Err(
            "state IDs and prior probabilities must have the same non-zero length".to_owned(),
        );
    }
    let mut state_ids = HashSet::new();
    for state_id in &problem.state_ids {
        if state_id.trim().is_empty() || !state_ids.insert(state_id) {
            return Err("state IDs must be non-empty and unique".to_owned());
        }
    }
    let mut prior_sum = 0.0;
    for (index, probability) in problem.prior_probabilities.iter().enumerate() {
        if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
            return Err(format!(
                "prior probability for state {} must be finite and in [0, 1]",
                problem.state_ids[index]
            ));
        }
        prior_sum += probability;
    }
    if (prior_sum - 1.0).abs() > PROBABILITY_TOLERANCE {
        return Err(format!("prior probabilities must sum to 1; got {prior_sum}"));
    }

    if problem.decisions.is_empty() {
        return Err("at least one decision option is required".to_owned());
    }
    let mut decision_ids = HashSet::new();
    for decision in &problem.decisions {
        if decision.decision_id.trim().is_empty() || !decision_ids.insert(&decision.decision_id) {
            return Err("decision IDs must be non-empty and unique".to_owned());
        }
        if decision.utility_by_state.len() != state_count {
            return Err(format!(
                "decision {} must provide one utility per state",
                decision.decision_id
            ));
        }
        if decision.utility_by_state.iter().any(|value| !value.is_finite()) {
            return Err(format!("decision {} contains a non-finite utility", decision.decision_id));
        }
    }

    if problem.experiments.is_empty() {
        return Err("at least one candidate experiment is required".to_owned());
    }
    let mut experiment_ids = HashSet::new();
    for experiment in &problem.experiments {
        if experiment.experiment_id.trim().is_empty()
            || !experiment_ids.insert(&experiment.experiment_id)
        {
            return Err("experiment IDs must be non-empty and unique".to_owned());
        }
        if !experiment.cost_units.is_finite() || experiment.cost_units < 0.0 {
            return Err(format!(
                "experiment {} cost must be finite and non-negative",
                experiment.experiment_id
            ));
        }
        if experiment.outcome_ids.is_empty() {
            return Err(format!(
                "experiment {} must declare at least one outcome",
                experiment.experiment_id
            ));
        }
        let mut outcome_ids = HashSet::new();
        for outcome_id in &experiment.outcome_ids {
            if outcome_id.trim().is_empty() || !outcome_ids.insert(outcome_id) {
                return Err(format!(
                    "experiment {} outcome IDs must be non-empty and unique",
                    experiment.experiment_id
                ));
            }
        }
        if experiment.likelihood_by_state.len() != state_count {
            return Err(format!(
                "experiment {} must declare one likelihood row per state",
                experiment.experiment_id
            ));
        }
        for (state_index, row) in experiment.likelihood_by_state.iter().enumerate() {
            if row.len() != experiment.outcome_ids.len() {
                return Err(format!(
                    "experiment {} likelihood row for state {} has the wrong outcome count",
                    experiment.experiment_id, problem.state_ids[state_index]
                ));
            }
            let mut row_sum = 0.0;
            for probability in row {
                if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
                    return Err(format!(
                        "experiment {} likelihoods must be finite and in [0, 1]",
                        experiment.experiment_id
                    ));
                }
                row_sum += probability;
            }
            if (row_sum - 1.0).abs() > PROBABILITY_TOLERANCE {
                return Err(format!(
                    "experiment {} likelihoods for state {} must sum to 1; got {row_sum}",
                    experiment.experiment_id, problem.state_ids[state_index]
                ));
            }
        }
    }
    Ok(())
}

/// Evaluate exact one-step EVSI for each candidate experiment.
///
/// The selector maximizes net EVSI among experiments that fit the remaining budget. It selects
/// nothing when all affordable experiments have non-positive net value. This is a one-step
/// finite-state policy, not a globally optimal multi-stage experimental design policy.
pub fn evaluate_evsi(
    problem: &DecisionProblem,
    remaining_budget: f64,
) -> Result<EvsiReport, String> {
    validate_problem(problem, remaining_budget)?;
    let (prior_best_decision_id, prior_best_expected_utility) =
        best_decision(problem, &problem.prior_probabilities)?;

    let mut experiments = Vec::with_capacity(problem.experiments.len());
    for experiment in &problem.experiments {
        let mut outcomes = Vec::with_capacity(experiment.outcome_ids.len());
        let mut expected_best_after = 0.0;

        for (outcome_index, outcome_id) in experiment.outcome_ids.iter().enumerate() {
            let probability = problem
                .prior_probabilities
                .iter()
                .enumerate()
                .map(|(state_index, prior)| {
                    prior * experiment.likelihood_by_state[state_index][outcome_index]
                })
                .sum::<f64>();
            if !probability.is_finite() || probability < -PROBABILITY_TOLERANCE {
                return Err(format!(
                    "experiment {} produced an invalid outcome probability",
                    experiment.experiment_id
                ));
            }

            if probability <= 0.0 {
                outcomes.push(OutcomeValue {
                    outcome_id: outcome_id.clone(),
                    probability: 0.0,
                    best_decision_id: None,
                    best_posterior_expected_utility: None,
                });
                continue;
            }

            let posterior: Vec<f64> = problem
                .prior_probabilities
                .iter()
                .enumerate()
                .map(|(state_index, prior)| {
                    prior * experiment.likelihood_by_state[state_index][outcome_index]
                        / probability
                })
                .collect();
            let (best_decision_id, best_posterior_expected_utility) =
                best_decision(problem, &posterior)?;
            expected_best_after += probability * best_posterior_expected_utility;
            outcomes.push(OutcomeValue {
                outcome_id: outcome_id.clone(),
                probability,
                best_decision_id: Some(best_decision_id),
                best_posterior_expected_utility: Some(best_posterior_expected_utility),
            });
        }

        if !expected_best_after.is_finite() {
            return Err(format!(
                "experiment {} expected utility is non-finite",
                experiment.experiment_id
            ));
        }
        let raw_evsi = expected_best_after - prior_best_expected_utility;
        if raw_evsi < -VALUE_TOLERANCE {
            return Err(format!(
                "experiment {} produced negative gross EVSI beyond floating-point tolerance",
                experiment.experiment_id
            ));
        }
        let gross_evsi = raw_evsi.max(0.0);
        let decision_changing_outcomes = outcomes
            .iter()
            .filter(|outcome| {
                outcome.probability > 0.0
                    && outcome.best_decision_id.as_deref()
                        != Some(prior_best_decision_id.as_str())
            })
            .count();
        let affordable = experiment.cost_units <= remaining_budget + VALUE_TOLERANCE;
        experiments.push(ExperimentValue {
            problem_id: problem.problem_id.clone(),
            experiment_id: experiment.experiment_id.clone(),
            cost_units: experiment.cost_units,
            remaining_budget_units: remaining_budget,
            affordable,
            prior_best_decision_id: prior_best_decision_id.clone(),
            prior_best_expected_utility,
            expected_best_utility_after_observation: expected_best_after,
            gross_evsi,
            net_evsi: gross_evsi - experiment.cost_units,
            outcome_count: experiment.outcome_ids.len(),
            decision_changing_outcomes,
            selected: false,
            outcomes,
        });
    }

    experiments.sort_by(|left, right| {
        right
            .net_evsi
            .total_cmp(&left.net_evsi)
            .then_with(|| left.experiment_id.cmp(&right.experiment_id))
    });
    let selected_experiment_id = experiments
        .iter()
        .find(|experiment| {
            experiment.affordable && experiment.net_evsi > VALUE_TOLERANCE
        })
        .map(|experiment| experiment.experiment_id.clone());
    if let Some(selected_id) = &selected_experiment_id {
        if let Some(selected) = experiments
            .iter_mut()
            .find(|experiment| &experiment.experiment_id == selected_id)
        {
            selected.selected = true;
        }
    }

    Ok(EvsiReport {
        problem_id: problem.problem_id.clone(),
        remaining_budget_units: remaining_budget,
        prior_best_decision_id,
        prior_best_expected_utility,
        selected_experiment_id,
        experiments,
    })
}

fn best_decision(
    problem: &DecisionProblem,
    probabilities: &[f64],
) -> Result<(String, f64), String> {
    let mut best: Option<(&DecisionOption, f64)> = None;
    for decision in &problem.decisions {
        let expected_utility = probabilities
            .iter()
            .zip(&decision.utility_by_state)
            .map(|(probability, utility)| probability * utility)
            .sum::<f64>();
        if !expected_utility.is_finite() {
            return Err(format!(
                "decision {} expected utility is non-finite",
                decision.decision_id
            ));
        }
        match best {
            None => best = Some((decision, expected_utility)),
            Some((current, current_value))
                if expected_utility > current_value + VALUE_TOLERANCE
                    || ((expected_utility - current_value).abs() <= VALUE_TOLERANCE
                        && decision.decision_id < current.decision_id) =>
            {
                best = Some((decision, expected_utility));
            }
            _ => {}
        }
    }
    let (decision, value) = best.expect("decision options validated as non-empty");
    Ok((decision.decision_id.clone(), value))
}

fn csv_escape(value: &str) -> String {
    value.replace('"', "\"\"")
}

pub fn report_to_csv(report: &EvsiReport) -> String {
    let mut output = format!("{EVSI_OUTPUT_HEADER}\n");
    for experiment in &report.experiments {
        output.push_str(&format!(
            "\"{}\",\"{}\",{:.6},{:.6},{},\"{}\",{:.12},{:.12},{:.12},{:.12},{},{},{},caller_supplied_probabilities_not_independently_calibrated\n",
            csv_escape(&report.problem_id),
            csv_escape(&experiment.experiment_id),
            experiment.cost_units,
            experiment.remaining_budget_units,
            experiment.affordable,
            csv_escape(&experiment.prior_best_decision_id),
            experiment.prior_best_expected_utility,
            experiment.expected_best_utility_after_observation,
            experiment.gross_evsi,
            experiment.net_evsi,
            experiment.outcome_count,
            experiment.decision_changing_outcomes,
            experiment.selected
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn synthetic_problem() -> DecisionProblem {
        DecisionProblem {
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
        }
    }

    #[test]
    fn perfect_information_is_selected_over_noisy_and_uninformative_experiments() {
        let report = evaluate_evsi(&synthetic_problem(), 2.0).expect("valid problem");
        assert_eq!(report.prior_best_decision_id, "design_a");
        assert!((report.prior_best_expected_utility - 5.0).abs() < 1e-12);
        assert_eq!(
            report.selected_experiment_id.as_deref(),
            Some("perfect_information")
        );
        let perfect = report
            .experiments
            .iter()
            .find(|item| item.experiment_id == "perfect_information")
            .expect("perfect experiment");
        assert!((perfect.gross_evsi - 5.0).abs() < 1e-12);
        assert!((perfect.net_evsi - 4.0).abs() < 1e-12);
        assert_eq!(perfect.decision_changing_outcomes, 2);
        assert_eq!(perfect.outcomes[0].best_decision_id.as_deref(), Some("design_a"));
        assert_eq!(perfect.outcomes[1].best_decision_id.as_deref(), Some("design_b"));
    }

    #[test]
    fn uninformative_measurement_has_zero_gross_evsi() {
        let report = evaluate_evsi(&synthetic_problem(), 2.0).expect("valid problem");
        let item = report
            .experiments
            .iter()
            .find(|experiment| experiment.experiment_id == "uninformative")
            .expect("uninformative experiment");
        assert!(item.gross_evsi.abs() < 1e-12);
        assert!((item.net_evsi + 0.25).abs() < 1e-12);
        assert_eq!(item.decision_changing_outcomes, 0);
    }

    #[test]
    fn noisy_experiment_has_positive_but_lower_evsi_than_perfect_information() {
        let report = evaluate_evsi(&synthetic_problem(), 2.0).expect("valid problem");
        let noisy = report
            .experiments
            .iter()
            .find(|experiment| experiment.experiment_id == "noisy_information")
            .expect("noisy experiment");
        let perfect = report
            .experiments
            .iter()
            .find(|experiment| experiment.experiment_id == "perfect_information")
            .expect("perfect experiment");
        assert!((noisy.gross_evsi - 3.0).abs() < 1e-12);
        assert!(noisy.gross_evsi < perfect.gross_evsi);
    }

    #[test]
    fn over_budget_experiment_cannot_be_selected() {
        let report = evaluate_evsi(&synthetic_problem(), 2.0).expect("valid problem");
        let item = report
            .experiments
            .iter()
            .find(|experiment| experiment.experiment_id == "too_expensive")
            .expect("expensive experiment");
        assert!(!item.affordable);
        assert!(!item.selected);
    }

    #[test]
    fn rejects_invalid_priors_likelihoods_and_duplicate_ids() {
        let mut problem = synthetic_problem();
        problem.prior_probabilities = vec![0.4, 0.4];
        assert!(evaluate_evsi(&problem, 2.0).is_err());

        let mut problem = synthetic_problem();
        problem.experiments[0].likelihood_by_state[0] = vec![0.7, 0.7];
        assert!(evaluate_evsi(&problem, 2.0).is_err());

        let mut problem = synthetic_problem();
        problem.experiments[1].experiment_id = "perfect_information".to_owned();
        assert!(evaluate_evsi(&problem, 2.0).is_err());
    }

    #[test]
    fn selector_abstains_when_no_affordable_experiment_has_positive_net_evsi() {
        let mut problem = synthetic_problem();
        problem.experiments[0].cost_units = 5.0;
        problem.experiments[1].cost_units = 5.0;
        let report = evaluate_evsi(&problem, 6.0).expect("valid problem");
        assert!(report.selected_experiment_id.is_none());
        assert!(report.experiments.iter().all(|item| !item.selected));
    }

    #[test]
    fn zero_probability_outcome_is_not_divided_by_zero() {
        let mut problem = synthetic_problem();
        problem.experiments.truncate(1);
        problem.experiments[0].outcome_ids.push("impossible".to_owned());
        for row in &mut problem.experiments[0].likelihood_by_state {
            row.push(0.0);
        }
        let report = evaluate_evsi(&problem, 2.0).expect("zero-probability outcome is valid");
        let item = &report.experiments[0];
        assert_eq!(item.outcomes[2].probability, 0.0);
        assert!(item.outcomes[2].best_decision_id.is_none());
    }

    #[test]
    fn csv_output_is_deterministic_and_declares_probability_provenance_limit() {
        let report = evaluate_evsi(&synthetic_problem(), 2.0).expect("valid problem");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert!(first.contains("caller_supplied_probabilities_not_independently_calibrated"));
        assert!(first.contains("perfect_information"));
    }
}
