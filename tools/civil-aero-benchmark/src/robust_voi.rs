//! Finite-ambiguity-set maximin EVSI for robust experiment selection.
//!
//! The ambiguity set is explicit and finite. The selector does not claim coverage of models
//! outside that set, calibrate priors, or turn the ensemble mean into a posterior probability.
//! It selects an affordable experiment by worst-case net EVSI and abstains when none is robustly
//! decision-positive.

use std::collections::{BTreeMap, HashSet};

use crate::voi::{DecisionProblem, ExperimentValue, evaluate_evsi};

const VALUE_TOLERANCE: f64 = 1.0e-12;

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionModelEnsemble {
    pub ensemble_id: String,
    pub remaining_budget_units: f64,
    /// Each member is a separately specified prior/likelihood model over a common decision task.
    pub members: Vec<DecisionProblem>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustExperimentValue {
    pub experiment_id: String,
    pub cost_units: f64,
    pub affordable: bool,
    pub member_count: usize,
    pub worst_case_net_evsi: f64,
    pub mean_net_evsi: f64,
    pub best_case_net_evsi: f64,
    pub selected: bool,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustEvsiReport {
    pub ensemble_id: String,
    pub member_count: usize,
    pub remaining_budget_units: f64,
    pub selected_experiment_id: Option<String>,
    pub evidence_level: &'static str,
    pub experiments: Vec<RobustExperimentValue>,
}

pub fn evaluate_robust_evsi(ensemble: &DecisionModelEnsemble) -> Result<RobustEvsiReport, String> {
    if ensemble.ensemble_id.trim().is_empty() {
        return Err("ensemble_id must not be empty".to_owned());
    }
    if ensemble.members.is_empty() {
        return Err("finite ambiguity set must contain at least one member".to_owned());
    }
    if !ensemble.remaining_budget_units.is_finite() || ensemble.remaining_budget_units < 0.0 {
        return Err("remaining budget must be finite and non-negative".to_owned());
    }

    let reference = &ensemble.members[0];
    let mut member_ids = HashSet::new();
    let mut reports = Vec::with_capacity(ensemble.members.len());
    for member in &ensemble.members {
        if member.problem_id.trim().is_empty() || !member_ids.insert(member.problem_id.clone()) {
            return Err("ambiguity-set member problem IDs must be non-empty and unique".to_owned());
        }
        ensure_compatible_task(reference, member)?;
        reports.push(evaluate_evsi(member, ensemble.remaining_budget_units)?);
    }

    let mut values_by_experiment: BTreeMap<String, Vec<ExperimentValue>> = BTreeMap::new();
    for report in &reports {
        for experiment in &report.experiments {
            values_by_experiment
                .entry(experiment.experiment_id.clone())
                .or_default()
                .push(experiment.clone());
        }
    }

    let mut robust_values = Vec::with_capacity(values_by_experiment.len());
    for (experiment_id, values) in values_by_experiment {
        if values.len() != reports.len() {
            return Err(format!(
                "experiment {experiment_id} is absent from one or more ambiguity-set members"
            ));
        }
        let first = &values[0];
        if values.iter().any(|value| {
            value.cost_units != first.cost_units || value.affordable != first.affordable
        }) {
            return Err(format!(
                "experiment {experiment_id} cost/affordability differs across members"
            ));
        }

        let mut worst_case_net_evsi = f64::INFINITY;
        let mut best_case_net_evsi = f64::NEG_INFINITY;
        let mut sum_net_evsi = 0.0;
        for value in &values {
            if !value.net_evsi.is_finite() {
                return Err(format!(
                    "experiment {experiment_id} has a non-finite member net EVSI"
                ));
            }
            worst_case_net_evsi = worst_case_net_evsi.min(value.net_evsi);
            best_case_net_evsi = best_case_net_evsi.max(value.net_evsi);
            sum_net_evsi += value.net_evsi;
        }
        let mean_net_evsi = sum_net_evsi / values.len() as f64;
        if !mean_net_evsi.is_finite() {
            return Err(format!(
                "experiment {experiment_id} mean net EVSI overflowed"
            ));
        }

        robust_values.push(RobustExperimentValue {
            experiment_id,
            cost_units: first.cost_units,
            affordable: first.affordable,
            member_count: values.len(),
            worst_case_net_evsi,
            mean_net_evsi,
            best_case_net_evsi,
            selected: false,
        });
    }

    robust_values.sort_by(|left, right| {
        right
            .worst_case_net_evsi
            .total_cmp(&left.worst_case_net_evsi)
            .then_with(|| left.experiment_id.cmp(&right.experiment_id))
    });
    let selected_experiment_id = robust_values
        .iter()
        .find(|value| value.affordable && value.worst_case_net_evsi > VALUE_TOLERANCE)
        .map(|value| value.experiment_id.clone());
    if let Some(selected_id) = &selected_experiment_id
        && let Some(value) = robust_values
            .iter_mut()
            .find(|value| &value.experiment_id == selected_id)
    {
        value.selected = true;
    }

    Ok(RobustEvsiReport {
        ensemble_id: ensemble.ensemble_id.clone(),
        member_count: ensemble.members.len(),
        remaining_budget_units: ensemble.remaining_budget_units,
        selected_experiment_id,
        evidence_level: "finite_ambiguity_set_maximin_not_probability_calibrated",
        experiments: robust_values,
    })
}

fn ensure_compatible_task(
    reference: &DecisionProblem,
    candidate: &DecisionProblem,
) -> Result<(), String> {
    if reference.state_ids != candidate.state_ids {
        return Err(format!(
            "member {} uses a different state space or state ordering",
            candidate.problem_id
        ));
    }
    if reference.decisions != candidate.decisions {
        return Err(format!(
            "member {} changes decision IDs or state utilities",
            candidate.problem_id
        ));
    }
    if reference.experiments.len() != candidate.experiments.len() {
        return Err(format!(
            "member {} changes the candidate experiment set",
            candidate.problem_id
        ));
    }

    let candidate_experiments: BTreeMap<&str, _> = candidate
        .experiments
        .iter()
        .map(|experiment| (experiment.experiment_id.as_str(), experiment))
        .collect();
    if candidate_experiments.len() != candidate.experiments.len() {
        return Err(format!(
            "member {} contains duplicate experiment IDs",
            candidate.problem_id
        ));
    }
    for reference_experiment in &reference.experiments {
        let candidate_experiment = candidate_experiments
            .get(reference_experiment.experiment_id.as_str())
            .ok_or_else(|| {
                format!(
                    "member {} is missing experiment {}",
                    candidate.problem_id, reference_experiment.experiment_id
                )
            })?;
        if reference_experiment.outcome_ids != candidate_experiment.outcome_ids
            || reference_experiment.cost_units != candidate_experiment.cost_units
        {
            return Err(format!(
                "member {} changes outcomes or cost for experiment {}",
                candidate.problem_id, reference_experiment.experiment_id
            ));
        }
    }
    Ok(())
}

pub const ROBUST_EVSI_CSV_HEADER: &str = "ensemble_id,member_count,remaining_budget_units,experiment_id,cost_units,affordable,worst_case_net_evsi,mean_net_evsi,best_case_net_evsi,selected,evidence_level";

pub fn report_to_csv(report: &RobustEvsiReport) -> String {
    let mut output = format!("{ROBUST_EVSI_CSV_HEADER}\n");
    for experiment in &report.experiments {
        output.push_str(&format!(
            "{},{},{:.8},{},{:.8},{},{:.12},{:.12},{:.12},{},{}\n",
            csv_escape(&report.ensemble_id),
            report.member_count,
            report.remaining_budget_units,
            csv_escape(&experiment.experiment_id),
            experiment.cost_units,
            experiment.affordable,
            experiment.worst_case_net_evsi,
            experiment.mean_net_evsi,
            experiment.best_case_net_evsi,
            experiment.selected,
            report.evidence_level
        ));
    }
    output
}

fn csv_escape(value: &str) -> String {
    format!("\"{}\"", value.replace('"', "\"\""))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::voi::{DecisionOption, Experiment};

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

    fn ensemble() -> DecisionModelEnsemble {
        DecisionModelEnsemble {
            ensemble_id: "two-model-ambiguity-set".to_owned(),
            remaining_budget_units: 2.0,
            members: vec![
                member("model_perfect", vec![vec![1.0, 0.0], vec![0.0, 1.0]]),
                member("model_blind", vec![vec![0.5, 0.5], vec![0.5, 0.5]]),
            ],
        }
    }

    #[test]
    fn maximin_selector_avoids_experiment_that_fails_under_one_model() {
        let robust = evaluate_robust_evsi(&ensemble()).expect("valid finite ambiguity set");
        assert_eq!(
            robust.selected_experiment_id.as_deref(),
            Some("robust_noisy")
        );
        let fragile = robust
            .experiments
            .iter()
            .find(|value| value.experiment_id == "fragile_perfect_or_blind")
            .expect("fragile experiment");
        let selected = robust
            .experiments
            .iter()
            .find(|value| value.experiment_id == "robust_noisy")
            .expect("robust experiment");
        assert!((fragile.worst_case_net_evsi + 1.0).abs() < 1.0e-10);
        assert!((selected.worst_case_net_evsi - 2.1).abs() < 1.0e-10);
        assert!(selected.selected);
        assert!(!fragile.selected);
    }

    #[test]
    fn nominal_models_disagree_but_maximin_decision_is_explicit() {
        let problems = ensemble();
        assert_eq!(
            evaluate_evsi(&problems.members[0], 2.0)
                .expect("model perfect")
                .selected_experiment_id
                .as_deref(),
            Some("fragile_perfect_or_blind")
        );
        assert_eq!(
            evaluate_evsi(&problems.members[1], 2.0)
                .expect("model blind")
                .selected_experiment_id
                .as_deref(),
            Some("robust_noisy")
        );
        assert_eq!(
            evaluate_robust_evsi(&problems)
                .expect("maximin across both models")
                .selected_experiment_id
                .as_deref(),
            Some("robust_noisy")
        );
    }

    #[test]
    fn abstains_when_every_experiment_has_nonpositive_worst_case_value() {
        let mut problem = ensemble();
        for experiment in &mut problem.members[0].experiments {
            experiment.likelihood_by_state = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        }
        for experiment in &mut problem.members[1].experiments {
            experiment.likelihood_by_state = vec![vec![0.5, 0.5], vec![0.5, 0.5]];
        }
        assert!(
            evaluate_robust_evsi(&problem)
                .expect("valid but uninformative models")
                .selected_experiment_id
                .is_none()
        );
    }

    #[test]
    fn rejects_changes_to_task_or_experiment_schema() {
        let mut changed = ensemble();
        changed.members[1].decisions[0].utility_by_state[0] = 11.0;
        assert!(
            evaluate_robust_evsi(&changed)
                .expect_err("utility mismatch rejected")
                .contains("changes decision IDs or state utilities")
        );

        let mut changed = ensemble();
        changed.members[1].experiments[0].cost_units = 0.9;
        assert!(
            evaluate_robust_evsi(&changed)
                .expect_err("cost mismatch rejected")
                .contains("changes outcomes or cost")
        );
    }

    #[test]
    fn rejects_duplicate_member_ids_and_empty_ambiguity_sets() {
        let mut changed = ensemble();
        changed.members[1].problem_id = changed.members[0].problem_id.clone();
        assert!(
            evaluate_robust_evsi(&changed)
                .expect_err("duplicate ID rejected")
                .contains("unique")
        );

        let mut changed = ensemble();
        changed.members.clear();
        assert!(evaluate_robust_evsi(&changed).is_err());
    }

    #[test]
    fn csv_report_is_deterministic_and_labels_finite_ambiguity_scope() {
        let report = evaluate_robust_evsi(&ensemble()).expect("valid finite ambiguity set");
        let first = report_to_csv(&report);
        assert_eq!(first, report_to_csv(&report));
        assert_eq!(first.lines().count(), 3);
        assert!(first.contains("finite_ambiguity_set_maximin_not_probability_calibrated"));
    }
}
