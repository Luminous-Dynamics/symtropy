//! Exact finite value-of-information microbenchmark.
//!
//! The problem is intentionally synthetic and binary. It tests whether a
//! policy uses the declared likelihood direction and fully burdened experiment
//! cost. It is not a materials model or evidence of physical performance.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Prior {
    pub state_a: f64,
    pub state_b: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Loss {
    pub correct_decision: f64,
    pub incorrect_decision: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Experiment {
    pub id: String,
    pub cost: f64,
    pub p_observation_a_given_state_a: f64,
    pub p_observation_a_given_state_b: f64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Problem {
    pub schema_version: String,
    pub evidence_class: String,
    pub intended_use: String,
    pub task_id: String,
    pub prior: Prior,
    pub loss: Loss,
    pub experiment_budget: f64,
    pub decision_cost_weight: f64,
    pub expected_best_experiment_id: String,
    pub experiments: Vec<Experiment>,
}

#[derive(Clone, Debug, Serialize)]
pub struct ExperimentScore {
    pub experiment_id: String,
    pub cost: f64,
    pub expected_bayes_loss_after_observation: f64,
    pub expected_value_of_sample_information: f64,
    pub cost_adjusted_objective: f64,
    pub affordable: bool,
    pub naive_same_direction_loss: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct BenchmarkReport {
    pub task_id: String,
    pub evidence_class: &'static str,
    pub claim_ceiling: &'static str,
    pub prior_bayes_loss: f64,
    pub best_experiment_id: Option<String>,
    pub best_cost_adjusted_objective: f64,
    pub expected_best_experiment_id: String,
    pub oracle_matches_fixture: bool,
    pub cheapest_experiment_id: String,
    pub cheapest_experiment_objective: f64,
    pub scores: Vec<ExperimentScore>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProblemError(pub String);

impl std::fmt::Display for ProblemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ProblemError {}

fn validate(problem: &Problem) -> Result<(), ProblemError> {
    if problem.schema_version != "protective-voi/0.1" {
        return Err(ProblemError("unsupported schema_version".into()));
    }
    if problem.evidence_class != "synthetic" {
        return Err(ProblemError("only synthetic tasks are accepted by this microbenchmark".into()));
    }
    if problem.task_id.trim().is_empty() || problem.intended_use.trim().is_empty() {
        return Err(ProblemError("task_id and intended_use must be non-empty".into()));
    }
    if !problem.prior.state_a.is_finite()
        || !problem.prior.state_b.is_finite()
        || problem.prior.state_a < 0.0
        || problem.prior.state_b < 0.0
        || (problem.prior.state_a + problem.prior.state_b - 1.0).abs() > 1e-12
    {
        return Err(ProblemError("prior probabilities must be finite, non-negative, and sum to one".into()));
    }
    if !problem.loss.correct_decision.is_finite()
        || !problem.loss.incorrect_decision.is_finite()
        || problem.loss.incorrect_decision <= problem.loss.correct_decision
    {
        return Err(ProblemError("losses must be finite and incorrect loss must exceed correct loss".into()));
    }
    if !problem.experiment_budget.is_finite() || problem.experiment_budget < 0.0 {
        return Err(ProblemError("experiment_budget must be finite and non-negative".into()));
    }
    if !problem.decision_cost_weight.is_finite() || problem.decision_cost_weight < 0.0 {
        return Err(ProblemError("decision_cost_weight must be finite and non-negative".into()));
    }
    if problem.experiments.is_empty() {
        return Err(ProblemError("at least one experiment is required".into()));
    }

    let mut ids = BTreeSet::new();
    for experiment in &problem.experiments {
        if experiment.id.trim().is_empty() || !ids.insert(experiment.id.as_str()) {
            return Err(ProblemError("experiment IDs must be non-empty and unique".into()));
        }
        if !experiment.cost.is_finite() || experiment.cost < 0.0 {
            return Err(ProblemError(format!("{} has an invalid cost", experiment.id)));
        }
        for p in [
            experiment.p_observation_a_given_state_a,
            experiment.p_observation_a_given_state_b,
        ] {
            if !p.is_finite() || !(0.0..=1.0).contains(&p) {
                return Err(ProblemError(format!("{} has a likelihood outside [0,1]", experiment.id)));
            }
        }
    }
    Ok(())
}

fn min_loss(a_mass: f64, b_mass: f64, loss: &Loss) -> f64 {
    let choose_a_loss = b_mass * loss.incorrect_decision + a_mass * loss.correct_decision;
    let choose_b_loss = a_mass * loss.incorrect_decision + b_mass * loss.correct_decision;
    choose_a_loss.min(choose_b_loss)
}

/// Exact expected Bayes loss after observing either A or B.
///
/// Joint state/observation masses are summed by observation. No Monte Carlo
/// sampling is involved, so this is an exact oracle for the finite task.
pub fn expected_bayes_loss(problem: &Problem, experiment: &Experiment) -> f64 {
    let pa = problem.prior.state_a;
    let pb = problem.prior.state_b;
    let p_a_a = experiment.p_observation_a_given_state_a;
    let p_a_b = experiment.p_observation_a_given_state_b;

    let observation_a = min_loss(pa * p_a_a, pb * p_a_b, &problem.loss);
    let observation_b = min_loss(pa * (1.0 - p_a_a), pb * (1.0 - p_a_b), &problem.loss);
    observation_a + observation_b
}

pub fn prior_bayes_loss(problem: &Problem) -> f64 {
    min_loss(problem.prior.state_a, problem.prior.state_b, &problem.loss)
}

/// Error from the deliberately naive policy that always maps observation A
/// to decision A and observation B to decision B, even when the proxy is
/// anti-correlated. This is a negative-control diagnostic.
pub fn naive_same_direction_loss(problem: &Problem, experiment: &Experiment) -> f64 {
    let pa = problem.prior.state_a;
    let pb = problem.prior.state_b;
    let wrong_when_a = pb * experiment.p_observation_a_given_state_b;
    let wrong_when_b = pa * (1.0 - experiment.p_observation_a_given_state_a);
    wrong_when_a * problem.loss.incorrect_decision
        + wrong_when_b * problem.loss.incorrect_decision
        + (1.0 - wrong_when_a - wrong_when_b) * problem.loss.correct_decision
}

pub fn run(problem: &Problem) -> Result<BenchmarkReport, ProblemError> {
    validate(problem)?;
    let baseline = prior_bayes_loss(problem);

    let mut scores = Vec::with_capacity(problem.experiments.len());
    for experiment in &problem.experiments {
        let after = expected_bayes_loss(problem, experiment);
        let voi = baseline - after;
        let objective = after + problem.decision_cost_weight * experiment.cost;
        scores.push(ExperimentScore {
            experiment_id: experiment.id.clone(),
            cost: experiment.cost,
            expected_bayes_loss_after_observation: after,
            expected_value_of_sample_information: voi,
            cost_adjusted_objective: objective,
            affordable: experiment.cost <= problem.experiment_budget,
            naive_same_direction_loss: naive_same_direction_loss(problem, experiment),
        });
    }

    let best_candidate = scores
        .iter()
        .filter(|s| s.affordable)
        .min_by(|a, b| {
            a.cost_adjusted_objective
                .total_cmp(&b.cost_adjusted_objective)
                .then_with(|| a.experiment_id.cmp(&b.experiment_id))
        });
    // "Do nothing" is a first-class action. Never run a test whose expected
    // cost-adjusted objective is no better than the current decision.
    let best = best_candidate.filter(|s| s.cost_adjusted_objective < baseline);
    let cheapest = scores
        .iter()
        .min_by(|a, b| a.cost.total_cmp(&b.cost).then_with(|| a.experiment_id.cmp(&b.experiment_id)))
        .expect("validated non-empty experiment set");

    let best_id = best.map(|s| s.experiment_id.clone());
    let best_objective = best
        .map(|s| s.cost_adjusted_objective)
        .unwrap_or(baseline);

    Ok(BenchmarkReport {
        task_id: problem.task_id.clone(),
        evidence_class: "synthetic",
        claim_ceiling: "finite_synthetic_decision_policy_only",
        prior_bayes_loss: baseline,
        best_experiment_id: best_id.clone(),
        best_cost_adjusted_objective: best_objective,
        expected_best_experiment_id: problem.expected_best_experiment_id.clone(),
        oracle_matches_fixture: best_id.as_deref() == Some(problem.expected_best_experiment_id.as_str()),
        cheapest_experiment_id: cheapest.experiment_id.clone(),
        cheapest_experiment_objective: cheapest.cost_adjusted_objective,
        scores,
    })
}

pub fn parse_problem(input: &str) -> Result<Problem, serde_json::Error> {
    serde_json::from_str(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../voi/fixtures/exact-oracle-v0.1.json");

    fn problem() -> Problem {
        parse_problem(FIXTURE).expect("fixture parses")
    }

    #[test]
    fn exact_oracle_selects_anti_correlated_proxy_after_inverting_it() {
        let report = run(&problem()).expect("valid problem");
        assert_eq!(report.best_experiment_id.as_deref(), Some("ANTI_CORRELATED_PROXY"));
        assert!(report.oracle_matches_fixture);
    }

    #[test]
    fn exact_expected_losses_match_hand_calculated_values() {
        let problem = problem();
        assert!((prior_bayes_loss(&problem) - 0.4).abs() < 1e-12);

        let by_id = |id: &str| problem.experiments.iter().find(|e| e.id == id).unwrap();
        assert!((expected_bayes_loss(&problem, by_id("CHEAP_WEAK_PROXY")) - 0.34).abs() < 1e-12);
        assert!((expected_bayes_loss(&problem, by_id("EXPENSIVE_HIGH_INFORMATION")) - 0.10).abs() < 1e-12);
        assert!((expected_bayes_loss(&problem, by_id("ANTI_CORRELATED_PROXY")) - 0.05).abs() < 1e-12);
        assert!((expected_bayes_loss(&problem, by_id("UNINFORMATIVE_PROXY")) - 0.40).abs() < 1e-12);
    }

    #[test]
    fn cheapest_experiment_is_worse_than_doing_nothing() {
        let report = run(&problem()).expect("valid problem");
        assert_eq!(report.cheapest_experiment_id, "UNINFORMATIVE_PROXY");
        assert!(report.cheapest_experiment_objective > report.prior_bayes_loss);
    }

    #[test]
    fn naive_signal_direction_fails_on_anti_correlated_proxy() {
        let problem = problem();
        let experiment = problem.experiments.iter()
            .find(|e| e.id == "ANTI_CORRELATED_PROXY").unwrap();
        assert!((naive_same_direction_loss(&problem, experiment) - 0.95).abs() < 1e-12);
        assert!(expected_bayes_loss(&problem, experiment) < 0.1);
    }

    #[test]
    fn over_budget_experiment_is_not_selected() {
        let mut problem = problem();
        problem.experiment_budget = 0.7;
        let report = run(&problem).expect("valid problem");
        assert_ne!(report.best_experiment_id.as_deref(), Some("ANTI_CORRELATED_PROXY"));
    }

    #[test]
    fn no_experiment_is_selected_when_every_test_costs_more_than_its_value() {
        let mut problem = problem();
        problem.decision_cost_weight = 10.0;
        let report = run(&problem).expect("valid problem");
        assert_eq!(report.best_experiment_id, None);
        assert!((report.best_cost_adjusted_objective - report.prior_bayes_loss).abs() < 1e-12);
    }

    #[test]
    fn invalid_probability_is_rejected() {
        let mut problem = problem();
        problem.experiments[0].p_observation_a_given_state_a = 1.01;
        assert!(run(&problem).is_err());
    }
}
