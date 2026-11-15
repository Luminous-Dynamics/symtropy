//! Exact finite robust-design oracle for protective-systems research.
//!
//! Synthetic-only. It compares candidates across a declared finite ambiguity
//! set, applies non-compensatory scenario safety floors, and computes exact
//! discrete upper-tail CVaR. It is not a physical hazard model or certification.

use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Distribution {
    pub distribution_id: String,
    pub probabilities: Vec<f64>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Design {
    pub design_id: String,
    pub loss_by_scenario: Vec<f64>,
    pub hard_safety_floor_satisfied: Vec<bool>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Problem {
    pub schema_version: String,
    pub evidence_class: String,
    pub task_id: String,
    pub uncertainty_set_complete: bool,
    pub cvar_alpha: f64,
    pub scenarios: Vec<String>,
    pub ambiguity_set: Vec<Distribution>,
    pub designs: Vec<Design>,
    pub expected_disposition: String,
    pub expected_best_design_id: Option<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DistributionScore {
    pub distribution_id: String,
    pub expected_loss: f64,
    pub upper_tail_cvar: f64,
}

#[derive(Clone, Debug, Serialize)]
pub struct DesignScore {
    pub design_id: String,
    pub robust_feasible: bool,
    pub hard_floor_failure_scenarios: Vec<String>,
    pub worst_case_scenario_loss: f64,
    pub worst_case_expected_loss: f64,
    pub worst_case_upper_tail_cvar: f64,
    pub by_distribution: Vec<DistributionScore>,
}

#[derive(Clone, Debug, Serialize)]
pub struct Report {
    pub task_id: String,
    pub evidence_class: &'static str,
    pub claim_ceiling: &'static str,
    pub disposition: String,
    pub cvar_alpha: f64,
    pub selected_design_id: Option<String>,
    pub expected_disposition: String,
    pub expected_best_design_id: Option<String>,
    pub oracle_matches_fixture: bool,
    pub scores: Vec<DesignScore>,
}

#[derive(Debug, PartialEq, Eq)]
pub struct ProblemError(pub String);

impl std::fmt::Display for ProblemError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl std::error::Error for ProblemError {}

const ROBUST_SELECTED: &str = "ROBUST_FEASIBLE_DESIGN_SELECTED";
const NO_FEASIBLE: &str = "NO_ROBUST_FEASIBLE_DESIGN";
const ABSTAIN_INCOMPLETE: &str = "ABSTAIN_INCOMPLETE_UNCERTAINTY_SET";

fn validate(problem: &Problem) -> Result<(), ProblemError> {
    if problem.schema_version != "protective-robust/0.1" {
        return Err(ProblemError("unsupported schema_version".into()));
    }
    if problem.evidence_class != "synthetic" {
        return Err(ProblemError("only synthetic problems are accepted".into()));
    }
    if problem.task_id.trim().is_empty() {
        return Err(ProblemError("task_id must be non-empty".into()));
    }
    if !problem.cvar_alpha.is_finite() || !(0.0..1.0).contains(&problem.cvar_alpha) {
        return Err(ProblemError("cvar_alpha must be finite and in [0,1)".into()));
    }
    if problem.scenarios.is_empty() {
        return Err(ProblemError("at least one scenario is required".into()));
    }
    let mut scenario_ids = BTreeSet::new();
    for scenario in &problem.scenarios {
        if scenario.trim().is_empty() || !scenario_ids.insert(scenario.as_str()) {
            return Err(ProblemError("scenario IDs must be non-empty and unique".into()));
        }
    }
    if problem.ambiguity_set.is_empty() {
        return Err(ProblemError("ambiguity_set must be non-empty".into()));
    }
    if problem.designs.is_empty() {
        return Err(ProblemError("designs must be non-empty".into()));
    }

    let mut distribution_ids = BTreeSet::new();
    for distribution in &problem.ambiguity_set {
        if distribution.distribution_id.trim().is_empty()
            || !distribution_ids.insert(distribution.distribution_id.as_str())
        {
            return Err(ProblemError(
                "distribution IDs must be non-empty and unique".into(),
            ));
        }
        if distribution.probabilities.len() != problem.scenarios.len() {
            return Err(ProblemError(format!(
                "{} probability count does not match scenario count",
                distribution.distribution_id
            )));
        }
        let mut sum = 0.0;
        for probability in &distribution.probabilities {
            if !probability.is_finite() || !(0.0..=1.0).contains(probability) {
                return Err(ProblemError(format!(
                    "{} contains an invalid probability",
                    distribution.distribution_id
                )));
            }
            sum += probability;
        }
        if (sum - 1.0).abs() > 1e-12 {
            return Err(ProblemError(format!(
                "{} probabilities do not sum to one",
                distribution.distribution_id
            )));
        }
    }

    let mut design_ids = BTreeSet::new();
    for design in &problem.designs {
        if design.design_id.trim().is_empty() || !design_ids.insert(design.design_id.as_str()) {
            return Err(ProblemError("design IDs must be non-empty and unique".into()));
        }
        if design.loss_by_scenario.len() != problem.scenarios.len()
            || design.hard_safety_floor_satisfied.len() != problem.scenarios.len()
        {
            return Err(ProblemError(format!(
                "{} outcome vectors do not match scenario count",
                design.design_id
            )));
        }
        if design
            .loss_by_scenario
            .iter()
            .any(|loss| !loss.is_finite() || *loss < 0.0)
        {
            return Err(ProblemError(format!(
                "{} contains a non-finite or negative loss",
                design.design_id
            )));
        }
    }
    Ok(())
}

/// Exact discrete CVaR of the upper loss tail.
///
/// The tail contains probability mass (1 - alpha). If the boundary cuts
/// through an atom, only the required fraction of that atom is included.
pub fn upper_tail_cvar(losses: &[f64], probabilities: &[f64], alpha: f64) -> f64 {
    let mut atoms: Vec<(f64, f64, usize)> = losses
        .iter()
        .copied()
        .zip(probabilities.iter().copied())
        .enumerate()
        .map(|(index, (loss, probability))| (loss, probability, index))
        .collect();
    atoms.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.2.cmp(&b.2)));

    let tail_mass = 1.0 - alpha;
    let mut remaining = tail_mass;
    let mut tail_loss = 0.0;
    for (loss, probability, _) in atoms {
        if remaining <= 0.0 {
            break;
        }
        let included_mass = probability.min(remaining);
        tail_loss += included_mass * loss;
        remaining -= included_mass;
    }
    tail_loss / tail_mass
}

fn expected_loss(losses: &[f64], probabilities: &[f64]) -> f64 {
    losses
        .iter()
        .zip(probabilities)
        .map(|(loss, probability)| loss * probability)
        .sum()
}

pub fn run(problem: &Problem) -> Result<Report, ProblemError> {
    validate(problem)?;

    let mut scores = Vec::with_capacity(problem.designs.len());
    for design in &problem.designs {
        let hard_floor_failure_scenarios: Vec<String> = design
            .hard_safety_floor_satisfied
            .iter()
            .enumerate()
            .filter(|(_, satisfied)| !**satisfied)
            .map(|(index, _)| problem.scenarios[index].clone())
            .collect();
        let robust_feasible = hard_floor_failure_scenarios.is_empty();
        let worst_case_scenario_loss = design
            .loss_by_scenario
            .iter()
            .copied()
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);

        let by_distribution: Vec<DistributionScore> = problem
            .ambiguity_set
            .iter()
            .map(|distribution| DistributionScore {
                distribution_id: distribution.distribution_id.clone(),
                expected_loss: expected_loss(
                    &design.loss_by_scenario,
                    &distribution.probabilities,
                ),
                upper_tail_cvar: upper_tail_cvar(
                    &design.loss_by_scenario,
                    &distribution.probabilities,
                    problem.cvar_alpha,
                ),
            })
            .collect();

        let worst_case_expected_loss = by_distribution
            .iter()
            .map(|score| score.expected_loss)
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);
        let worst_case_upper_tail_cvar = by_distribution
            .iter()
            .map(|score| score.upper_tail_cvar)
            .max_by(f64::total_cmp)
            .unwrap_or(0.0);

        scores.push(DesignScore {
            design_id: design.design_id.clone(),
            robust_feasible,
            hard_floor_failure_scenarios,
            worst_case_scenario_loss,
            worst_case_expected_loss,
            worst_case_upper_tail_cvar,
            by_distribution,
        });
    }

    let selected = scores
        .iter()
        .filter(|score| score.robust_feasible)
        .min_by(|a, b| {
            a.worst_case_expected_loss
                .total_cmp(&b.worst_case_expected_loss)
                .then_with(|| {
                    a.worst_case_upper_tail_cvar
                        .total_cmp(&b.worst_case_upper_tail_cvar)
                })
                .then_with(|| {
                    a.worst_case_scenario_loss
                        .total_cmp(&b.worst_case_scenario_loss)
                })
                .then_with(|| a.design_id.cmp(&b.design_id))
        });

    let disposition = if !problem.uncertainty_set_complete {
        ABSTAIN_INCOMPLETE.to_owned()
    } else if selected.is_some() {
        ROBUST_SELECTED.to_owned()
    } else {
        NO_FEASIBLE.to_owned()
    };
    let selected_design_id = if problem.uncertainty_set_complete {
        selected.map(|score| score.design_id.clone())
    } else {
        None
    };

    Ok(Report {
        task_id: problem.task_id.clone(),
        evidence_class: "synthetic",
        claim_ceiling: "finite_synthetic_robust_decision_only",
        disposition: disposition.clone(),
        cvar_alpha: problem.cvar_alpha,
        selected_design_id: selected_design_id.clone(),
        expected_disposition: problem.expected_disposition.clone(),
        expected_best_design_id: problem.expected_best_design_id.clone(),
        oracle_matches_fixture: disposition == problem.expected_disposition
            && selected_design_id == problem.expected_best_design_id,
        scores,
    })
}

pub fn parse_problem(input: &str) -> Result<Problem, serde_json::Error> {
    serde_json::from_str(input)
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../robust/fixtures/finite-ambiguity-v0.1.json");

    fn problem() -> Problem {
        parse_problem(FIXTURE).expect("fixture parses")
    }

    #[test]
    fn worst_case_ambiguity_set_selects_the_robust_feasible_design() {
        let report = run(&problem()).expect("valid finite problem");
        assert_eq!(report.disposition, ROBUST_SELECTED);
        assert_eq!(report.selected_design_id.as_deref(), Some("ROBUST_DESIGN"));
        assert!(report.oracle_matches_fixture);
    }

    #[test]
    fn scenario_hard_floor_failure_is_non_compensatory() {
        let report = run(&problem()).expect("valid finite problem");
        let fragile = report
            .scores
            .iter()
            .find(|score| score.design_id == "CHEAP_FRAGILE_DESIGN")
            .expect("fragile design exists");
        assert!(!fragile.robust_feasible);
        assert_eq!(
            fragile.hard_floor_failure_scenarios,
            vec!["DISTRIBUTION_SHIFT".to_owned()]
        );
        assert_ne!(report.selected_design_id.as_deref(), Some("CHEAP_FRAGILE_DESIGN"));
    }

    #[test]
    fn exact_discrete_cvar_handles_a_fractional_tail_atom() {
        let result = upper_tail_cvar(&[1.0, 10.0], &[0.9, 0.1], 0.8);
        // Upper 20% consists of 10% at loss 10 and 10% at loss 1.
        assert!((result - 5.5).abs() < 1e-12);
    }

    #[test]
    fn robust_design_reduces_tail_loss_under_shifted_distribution() {
        let report = run(&problem()).expect("valid finite problem");
        let nominal = report
            .scores
            .iter()
            .find(|score| score.design_id == "NOMINAL_OPTIMUM")
            .expect("nominal design");
        let robust = report
            .scores
            .iter()
            .find(|score| score.design_id == "ROBUST_DESIGN")
            .expect("robust design");
        assert!((nominal.worst_case_expected_loss - 7.5).abs() < 1e-12);
        assert!((robust.worst_case_expected_loss - 4.75).abs() < 1e-12);
        assert!((nominal.worst_case_upper_tail_cvar - 18.0).abs() < 1e-12);
        assert!((robust.worst_case_upper_tail_cvar - 8.0).abs() < 1e-12);
    }

    #[test]
    fn incomplete_uncertainty_set_abstains_instead_of_imputing_zero_risk() {
        let mut problem = problem();
        problem.uncertainty_set_complete = false;
        let report = run(&problem).expect("structurally valid incomplete problem");
        assert_eq!(report.disposition, ABSTAIN_INCOMPLETE);
        assert_eq!(report.selected_design_id, None);
        assert!(!report.oracle_matches_fixture);
    }

    #[test]
    fn no_robust_feasible_design_is_an_explicit_result() {
        let mut problem = problem();
        for design in &mut problem.designs {
            design.hard_safety_floor_satisfied.fill(false);
        }
        problem.expected_disposition = NO_FEASIBLE.to_owned();
        problem.expected_best_design_id = None;
        let report = run(&problem).expect("valid problem");
        assert_eq!(report.disposition, NO_FEASIBLE);
        assert_eq!(report.selected_design_id, None);
        assert!(report.oracle_matches_fixture);
    }

    #[test]
    fn malformed_probability_mass_is_rejected() {
        let mut problem = problem();
        problem.ambiguity_set[0].probabilities[0] = 0.8;
        assert!(run(&problem).is_err());
    }
}
