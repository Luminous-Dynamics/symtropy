// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Constraint-first, interval-aware Pareto evaluation.
//!
//! Safety and operating limits are hard constraints, never weighted objectives.
//! A design must be proven feasible before it may enter the Pareto front.
//! Missing or overlapping uncertainty intervals are preserved as
//! NotEvaluable or Uncertain rather than silently treated as safe.
//!
//! Robust dominance is deliberately conservative: for every maximize
//! objective, A's lower bound must be at least B's upper bound; for every
//! minimize objective, A's upper bound must be at most B's lower bound.
//! Overlapping intervals do not justify an invented ranking.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum MetricError {
    InvalidInterval,
    NonFiniteConstraintLimit,
    EmptyCandidateId,
    EmptyMetricName,
    EmptyObjectiveSet,
    EmptyConstraintName,
    DuplicateObjectiveMetric,
    DuplicateCandidateMetric,
    DuplicateConstraintName,
    DuplicateCandidateId,
    MissingMetric(String),
}

impl fmt::Display for MetricError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInterval => f.write_str("metric interval must be finite and lower <= upper"),
            Self::NonFiniteConstraintLimit => f.write_str("constraint limit must be finite"),
            Self::EmptyCandidateId => f.write_str("candidate ID must not be empty"),
            Self::EmptyMetricName => f.write_str("metric name must not be empty"),
            Self::EmptyObjectiveSet => f.write_str("at least one Pareto objective is required"),
            Self::EmptyConstraintName => f.write_str("constraint name must not be empty"),
            Self::DuplicateObjectiveMetric => f.write_str("objective metric names must be unique"),
            Self::DuplicateCandidateMetric => f.write_str("candidate metric names must be unique"),
            Self::DuplicateConstraintName => f.write_str("constraint names must be unique"),
            Self::DuplicateCandidateId => f.write_str("candidate IDs must be unique in a portfolio"),
            Self::MissingMetric(metric) => write!(f, "required metric is missing: {metric}"),
        }
    }
}

impl std::error::Error for MetricError {}

/// Closed interval containing a metric's declared uncertainty.
///
/// Exact reported values use lower == upper. Unknown uncertainty must not be
/// represented as a zero-width interval because that implies unjustified precision.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    pub lower: f64,
    pub upper: f64,
}

impl Interval {
    pub fn new(lower: f64, upper: f64) -> Result<Self, MetricError> {
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err(MetricError::InvalidInterval);
        }
        Ok(Self { lower, upper })
    }

    pub fn exact(value: f64) -> Result<Self, MetricError> {
        Self::new(value, value)
    }
}

/// A candidate design and its reported metric intervals.
#[derive(Debug, Clone, PartialEq)]
pub struct Candidate {
    pub id: String,
    pub metrics: BTreeMap<String, Interval>,
}

impl Candidate {
    pub fn new<I, K>(id: impl Into<String>, metrics: I) -> Result<Self, MetricError>
    where
        I: IntoIterator<Item = (K, Interval)>,
        K: Into<String>,
    {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(MetricError::EmptyCandidateId);
        }
        let mut indexed = BTreeMap::new();
        for (name, interval) in metrics {
            let name = name.into();
            if name.trim().is_empty() {
                return Err(MetricError::EmptyMetricName);
            }
            if indexed.insert(name, interval).is_some() {
                return Err(MetricError::DuplicateCandidateMetric);
            }
        }
        Ok(Self { id, metrics: indexed })
    }

    pub fn metric(&self, name: &str) -> Option<Interval> {
        self.metrics.get(name).copied()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectiveDirection {
    Maximize,
    Minimize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Objective {
    pub metric: String,
    pub direction: ObjectiveDirection,
}

impl Objective {
    pub fn new(metric: impl Into<String>, direction: ObjectiveDirection) -> Result<Self, MetricError> {
        let metric = metric.into();
        if metric.trim().is_empty() {
            return Err(MetricError::EmptyMetricName);
        }
        Ok(Self { metric, direction })
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintSense {
    /// The metric must satisfy x <= limit.
    AtMost,
    /// The metric must satisfy x >= limit.
    AtLeast,
}

#[derive(Debug, Clone, PartialEq)]
pub struct HardConstraint {
    pub name: String,
    pub metric: String,
    pub sense: ConstraintSense,
    pub limit: f64,
    pub unit: String,
}

impl HardConstraint {
    pub fn new(
        name: impl Into<String>,
        metric: impl Into<String>,
        sense: ConstraintSense,
        limit: f64,
        unit: impl Into<String>,
    ) -> Result<Self, MetricError> {
        let name = name.into();
        let metric = metric.into();
        let unit = unit.into();
        if name.trim().is_empty() {
            return Err(MetricError::EmptyConstraintName);
        }
        if metric.trim().is_empty() || unit.trim().is_empty() {
            return Err(MetricError::EmptyMetricName);
        }
        if !limit.is_finite() {
            return Err(MetricError::NonFiniteConstraintLimit);
        }
        Ok(Self { name, metric, sense, limit, unit })
    }

    fn evaluate(&self, interval: Option<Interval>) -> ConstraintOutcome {
        let Some(interval) = interval else {
            return ConstraintOutcome::Missing;
        };
        match self.sense {
            ConstraintSense::AtMost => {
                if interval.upper <= self.limit {
                    ConstraintOutcome::Safe
                } else if interval.lower > self.limit {
                    ConstraintOutcome::Violated
                } else {
                    ConstraintOutcome::Uncertain
                }
            }
            ConstraintSense::AtLeast => {
                if interval.lower >= self.limit {
                    ConstraintOutcome::Safe
                } else if interval.upper < self.limit {
                    ConstraintOutcome::Violated
                } else {
                    ConstraintOutcome::Uncertain
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintOutcome {
    Safe,
    Violated,
    Uncertain,
    Missing,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConstraintFinding {
    pub name: String,
    pub outcome: ConstraintOutcome,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Disposition {
    Feasible,
    Infeasible,
    Uncertain,
    NotEvaluable,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CandidateEvaluation {
    pub candidate_id: String,
    pub disposition: Disposition,
    pub constraints: Vec<ConstraintFinding>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct EvaluationPlan {
    pub objectives: Vec<Objective>,
    pub constraints: Vec<HardConstraint>,
}

impl EvaluationPlan {
    pub fn new(
        objectives: Vec<Objective>,
        constraints: Vec<HardConstraint>,
    ) -> Result<Self, MetricError> {
        if objectives.is_empty() {
            return Err(MetricError::EmptyObjectiveSet);
        }
        let mut objective_names = BTreeSet::new();
        for objective in &objectives {
            if !objective_names.insert(objective.metric.as_str()) {
                return Err(MetricError::DuplicateObjectiveMetric);
            }
        }
        let mut constraint_names = BTreeSet::new();
        for constraint in &constraints {
            if !constraint_names.insert(constraint.name.as_str()) {
                return Err(MetricError::DuplicateConstraintName);
            }
        }
        Ok(Self { objectives, constraints })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PortfolioEvaluation {
    /// Sorted by candidate ID, independent of candidate input order.
    pub evaluations: Vec<CandidateEvaluation>,
    /// Sorted candidate IDs; only proven feasible candidates can appear.
    pub feasible_pareto_front: Vec<String>,
}

pub fn evaluate_candidate(candidate: &Candidate, plan: &EvaluationPlan) -> CandidateEvaluation {
    let constraints: Vec<_> = plan.constraints.iter().map(|constraint| {
        ConstraintFinding {
            name: constraint.name.clone(),
            outcome: constraint.evaluate(candidate.metric(&constraint.metric)),
        }
    }).collect();

    let violation = constraints.iter().any(|x| x.outcome == ConstraintOutcome::Violated);
    let missing_constraint = constraints.iter().any(|x| x.outcome == ConstraintOutcome::Missing);
    let uncertain_constraint = constraints.iter().any(|x| x.outcome == ConstraintOutcome::Uncertain);
    let missing_objective = plan.objectives.iter().any(|x| candidate.metric(&x.metric).is_none());

    // A proven violation outranks other missing evidence; never call it safe.
    let disposition = if violation {
        Disposition::Infeasible
    } else if missing_constraint || missing_objective {
        Disposition::NotEvaluable
    } else if uncertain_constraint {
        Disposition::Uncertain
    } else {
        Disposition::Feasible
    };

    CandidateEvaluation {
        candidate_id: candidate.id.clone(),
        disposition,
        constraints,
    }
}

/// A dominates B only if A is guaranteed no worse on every objective over
/// the full intervals and guaranteed strictly better on at least one.
pub fn robustly_dominates(
    a: &Candidate,
    b: &Candidate,
    objectives: &[Objective],
) -> Result<bool, MetricError> {
    let mut strictly_better = false;
    for objective in objectives {
        let av = a.metric(&objective.metric)
            .ok_or_else(|| MetricError::MissingMetric(objective.metric.clone()))?;
        let bv = b.metric(&objective.metric)
            .ok_or_else(|| MetricError::MissingMetric(objective.metric.clone()))?;
        let (no_worse, strictly) = match objective.direction {
            ObjectiveDirection::Maximize => (av.lower >= bv.upper, av.lower > bv.upper),
            ObjectiveDirection::Minimize => (av.upper <= bv.lower, av.upper < bv.lower),
        };
        if !no_worse {
            return Ok(false);
        }
        strictly_better |= strictly;
    }
    Ok(strictly_better)
}

/// Only Feasible candidates enter the robust Pareto front. Input order does
/// not affect output order or disposition.
pub fn evaluate_portfolio(
    candidates: &[Candidate],
    plan: &EvaluationPlan,
) -> Result<PortfolioEvaluation, MetricError> {
    let mut ids = BTreeSet::new();
    for candidate in candidates {
        if !ids.insert(candidate.id.as_str()) {
            return Err(MetricError::DuplicateCandidateId);
        }
    }

    let mut evaluations: Vec<_> = candidates.iter()
        .map(|candidate| evaluate_candidate(candidate, plan))
        .collect();
    evaluations.sort_by(|a, b| a.candidate_id.cmp(&b.candidate_id));

    let feasible: Vec<_> = candidates.iter().filter(|candidate| {
        evaluations.iter()
            .find(|evaluation| evaluation.candidate_id == candidate.id)
            .is_some_and(|evaluation| evaluation.disposition == Disposition::Feasible)
    }).collect();

    let mut front = Vec::new();
    for candidate in &feasible {
        let mut dominated = false;
        for other in &feasible {
            if candidate.id == other.id {
                continue;
            }
            if robustly_dominates(other, candidate, &plan.objectives)? {
                dominated = true;
                break;
            }
        }
        if !dominated {
            front.push(candidate.id.clone());
        }
    }
    front.sort();

    Ok(PortfolioEvaluation {
        evaluations,
        feasible_pareto_front: front,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn plan() -> EvaluationPlan {
        EvaluationPlan::new(
            vec![
                Objective::new("efficiency_percent", ObjectiveDirection::Maximize).unwrap(),
                Objective::new("shield_loss_w", ObjectiveDirection::Minimize).unwrap(),
                Objective::new("load_power_w", ObjectiveDirection::Maximize).unwrap(),
            ],
            vec![
                HardConstraint::new(
                    "electric_exposure_limit",
                    "e_field_v_per_m",
                    ConstraintSense::AtMost,
                    100.0,
                    "V/m",
                ).unwrap(),
                HardConstraint::new(
                    "temperature_rise_limit",
                    "temperature_rise_c",
                    ConstraintSense::AtMost,
                    40.0,
                    "degC",
                ).unwrap(),
            ],
        ).unwrap()
    }

    fn candidate(
        id: &str,
        efficiency: (f64, f64),
        shield_loss: (f64, f64),
        power: (f64, f64),
        e_field: (f64, f64),
        temperature: (f64, f64),
    ) -> Candidate {
        Candidate::new(
            id,
            [
                ("efficiency_percent", Interval::new(efficiency.0, efficiency.1).unwrap()),
                ("shield_loss_w", Interval::new(shield_loss.0, shield_loss.1).unwrap()),
                ("load_power_w", Interval::new(power.0, power.1).unwrap()),
                ("e_field_v_per_m", Interval::new(e_field.0, e_field.1).unwrap()),
                ("temperature_rise_c", Interval::new(temperature.0, temperature.1).unwrap()),
            ],
        ).unwrap()
    }

    #[test]
    fn hard_safety_violation_cannot_be_compensated_by_efficiency() {
        let item = candidate("unsafe", (99.9, 100.0), (1.0, 1.1), (100_000.0, 110_000.0), (101.0, 105.0), (20.0, 25.0));
        assert_eq!(evaluate_candidate(&item, &plan()).disposition, Disposition::Infeasible);
    }

    #[test]
    fn interval_crossing_a_hard_limit_is_uncertain() {
        let item = candidate("crossing", (95.0, 96.0), (2.0, 3.0), (60_000.0, 65_000.0), (98.0, 102.0), (20.0, 25.0));
        assert_eq!(evaluate_candidate(&item, &plan()).disposition, Disposition::Uncertain);
    }

    #[test]
    fn missing_required_safety_metric_is_not_evaluable() {
        let item = Candidate::new("missing", [
            ("efficiency_percent", Interval::exact(95.0).unwrap()),
            ("shield_loss_w", Interval::exact(5.0).unwrap()),
            ("load_power_w", Interval::exact(50_000.0).unwrap()),
            ("temperature_rise_c", Interval::exact(20.0).unwrap()),
        ]).unwrap();
        assert_eq!(evaluate_candidate(&item, &plan()).disposition, Disposition::NotEvaluable);
    }

    #[test]
    fn proven_violation_takes_priority_over_another_missing_constraint() {
        let item = Candidate::new("bad", [
            ("efficiency_percent", Interval::exact(99.0).unwrap()),
            ("shield_loss_w", Interval::exact(2.0).unwrap()),
            ("load_power_w", Interval::exact(90_000.0).unwrap()),
            ("e_field_v_per_m", Interval::exact(120.0).unwrap()),
        ]).unwrap();
        assert_eq!(evaluate_candidate(&item, &plan()).disposition, Disposition::Infeasible);
    }

    #[test]
    fn exact_inclusive_upper_and_lower_boundaries_are_safe() {
        let item = candidate("boundary", (95.0, 95.0), (4.0, 4.0), (60_000.0, 60_000.0), (100.0, 100.0), (40.0, 40.0));
        assert_eq!(evaluate_candidate(&item, &plan()).disposition, Disposition::Feasible);
        let minimum = HardConstraint::new("minimum", "power", ConstraintSense::AtLeast, 60.0, "W").unwrap();
        assert_eq!(minimum.evaluate(Some(Interval::exact(60.0).unwrap())), ConstraintOutcome::Safe);
    }

    #[test]
    fn robust_dominance_requires_interval_separation() {
        let objectives = vec![
            Objective::new("power", ObjectiveDirection::Maximize).unwrap(),
            Objective::new("loss", ObjectiveDirection::Minimize).unwrap(),
        ];
        let a = Candidate::new("a", [
            ("power", Interval::new(110.0, 120.0).unwrap()),
            ("loss", Interval::new(4.0, 5.0).unwrap()),
        ]).unwrap();
        let b = Candidate::new("b", [
            ("power", Interval::new(90.0, 100.0).unwrap()),
            ("loss", Interval::new(6.0, 7.0).unwrap()),
        ]).unwrap();
        assert!(robustly_dominates(&a, &b, &objectives).unwrap());
        assert!(!robustly_dominates(&b, &a, &objectives).unwrap());
    }

    #[test]
    fn overlapping_uncertainties_are_not_forced_into_a_ranking() {
        let objectives = vec![
            Objective::new("power", ObjectiveDirection::Maximize).unwrap(),
            Objective::new("loss", ObjectiveDirection::Minimize).unwrap(),
        ];
        let a = Candidate::new("a", [
            ("power", Interval::new(90.0, 105.0).unwrap()),
            ("loss", Interval::new(8.0, 9.0).unwrap()),
        ]).unwrap();
        let b = Candidate::new("b", [
            ("power", Interval::new(95.0, 110.0).unwrap()),
            ("loss", Interval::new(8.5, 9.5).unwrap()),
        ]).unwrap();
        assert!(!robustly_dominates(&a, &b, &objectives).unwrap());
        assert!(!robustly_dominates(&b, &a, &objectives).unwrap());
    }

    #[test]
    fn only_feasible_candidates_enter_the_pareto_front() {
        let a = candidate("a", (96.0, 96.0), (5.0, 5.0), (70_000.0, 70_000.0), (80.0, 80.0), (30.0, 30.0));
        let b = candidate("b", (97.0, 97.0), (4.0, 4.0), (75_000.0, 75_000.0), (90.0, 90.0), (35.0, 35.0));
        let c = candidate("c", (99.0, 99.0), (1.0, 1.0), (100_000.0, 100_000.0), (110.0, 110.0), (20.0, 20.0));
        let d = candidate("d", (98.0, 98.0), (2.0, 2.0), (90_000.0, 90_000.0), (99.0, 101.0), (20.0, 20.0));

        let result = evaluate_portfolio(&[a, b, c, d], &plan()).unwrap();
        assert_eq!(result.feasible_pareto_front, vec!["b".to_string()]);
        assert_eq!(result.evaluations.iter().find(|x| x.candidate_id == "c").unwrap().disposition, Disposition::Infeasible);
        assert_eq!(result.evaluations.iter().find(|x| x.candidate_id == "d").unwrap().disposition, Disposition::Uncertain);
    }

    #[test]
    fn result_is_deterministic_under_input_reordering() {
        let a = candidate("a", (96.0, 96.0), (5.0, 5.0), (70_000.0, 70_000.0), (80.0, 80.0), (30.0, 30.0));
        let b = candidate("b", (97.0, 97.0), (4.0, 4.0), (75_000.0, 75_000.0), (90.0, 90.0), (35.0, 35.0));
        let c = candidate("c", (99.0, 99.0), (1.0, 1.0), (100_000.0, 100_000.0), (110.0, 110.0), (20.0, 20.0));
        let first = evaluate_portfolio(&[a.clone(), b.clone(), c.clone()], &plan()).unwrap();
        let second = evaluate_portfolio(&[c, b, a], &plan()).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn invalid_intervals_and_duplicate_objectives_are_rejected() {
        assert_eq!(Interval::new(f64::NAN, 1.0), Err(MetricError::InvalidInterval));
        assert_eq!(Interval::new(2.0, 1.0), Err(MetricError::InvalidInterval));
        let duplicate = EvaluationPlan::new(vec![
            Objective::new("power", ObjectiveDirection::Maximize).unwrap(),
            Objective::new("power", ObjectiveDirection::Minimize).unwrap(),
        ], vec![]);
        assert_eq!(duplicate, Err(MetricError::DuplicateObjectiveMetric));
    }

    #[test]
    fn duplicate_candidate_ids_are_rejected() {
        let a = candidate("same", (95.0, 95.0), (5.0, 5.0), (60_000.0, 60_000.0), (90.0, 90.0), (20.0, 20.0));
        assert_eq!(evaluate_portfolio(&[a.clone(), a], &plan()), Err(MetricError::DuplicateCandidateId));
    }
}
