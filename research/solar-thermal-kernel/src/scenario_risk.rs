// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Scenario-weighted useful-heat service risk metrics.
//!
//! This module evaluates service outcomes across declared scenarios. It does not
//! create weather scenarios or optimize dispatch. Scenario chronology/correlation
//! and probability calibration belong to the upstream data/benchmark contract.

use super::{EnergyJ, Fraction};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenarioProbability(f64);

impl ScenarioProbability {
    pub fn new(value: f64) -> Result<Self, ScenarioRiskError> {
        if !value.is_finite() {
            return Err(ScenarioRiskError::InvalidProbability);
        }
        if value <= 0.0 || value > 1.0 {
            return Err(ScenarioRiskError::InvalidProbability);
        }
        Ok(Self(value))
    }

    pub fn value(self) -> f64 {
        self.0
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ScenarioServiceOutcome {
    /// Stable, caller-assigned identifier; identifiers must be unique.
    pub scenario_id: u64,
    /// Probability mass for this scenario; all masses must sum to one.
    pub probability: ScenarioProbability,
    pub requested_useful_heat: EnergyJ,
    pub delivered_useful_heat: EnergyJ,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ScenarioTailRiskReport {
    pub scenario_count: usize,
    pub minimum_service_fraction: Fraction,
    /// Probability-weighted absolute unmet useful heat, in joules.
    pub expected_unmet_useful_heat: EnergyJ,
    /// Largest absolute unmet useful heat among scenarios, in joules.
    pub worst_unmet_useful_heat: EnergyJ,
    /// Largest shortfall fraction. A zero-demand scenario has zero shortfall.
    pub worst_shortfall_fraction: f64,
    /// Probability-weighted mean of scenario shortfall fractions.
    pub expected_shortfall_fraction: f64,
    /// Weighted empirical 95th percentile. First value whose cumulative mass
    /// reaches or exceeds 0.95, with equal shortfalls treated as ties.
    pub p95_shortfall_fraction: f64,
    /// Mean shortfall over the worst 5% probability mass, splitting scenario
    /// weights at the boundary when needed.
    pub cvar95_shortfall_fraction: f64,
    /// Probability mass for scenarios whose delivered/requested fraction is
    /// strictly below the declared minimum service fraction.
    pub probability_below_service_floor: f64,
    /// Deterministic tie-break: lowest scenario ID among worst-shortfall ties.
    pub worst_case_scenario_id: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ScenarioRiskError {
    EmptyScenarioSet,
    InvalidProbability,
    ProbabilityMassDoesNotSumToOne,
    DuplicateScenarioId(u64),
    DeliveredHeatExceedsDemand(u64),
    NonFiniteMetric,
    ReportMismatch(&'static str),
}

impl std::fmt::Display for ScenarioRiskError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyScenarioSet => write!(f, "scenario set must not be empty"),
            Self::InvalidProbability => write!(f, "scenario probability must be finite and in (0, 1]"),
            Self::ProbabilityMassDoesNotSumToOne => write!(f, "scenario probabilities must sum to one"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate scenario id: {id}"),
            Self::DeliveredHeatExceedsDemand(id) => write!(f, "delivered useful heat exceeds demand in scenario {id}"),
            Self::NonFiniteMetric => write!(f, "scenario-risk metric is not finite"),
            Self::ReportMismatch(name) => write!(f, "independent scenario-risk verification failed: {name}"),
        }
    }
}

impl std::error::Error for ScenarioRiskError {}

fn close(a: f64, b: f64) -> bool {
    a.is_finite() && b.is_finite()
        && (a - b).abs() <= 1.0e-9 * a.abs().max(b.abs()).max(1.0)
}

fn shortfall_fraction(outcome: ScenarioServiceOutcome) -> f64 {
    let requested = outcome.requested_useful_heat.value();
    if requested <= 0.0 {
        0.0
    } else {
        ((requested - outcome.delivered_useful_heat.value()) / requested).clamp(0.0, 1.0)
    }
}

fn validate_scenarios(outcomes: &[ScenarioServiceOutcome]) -> Result<f64, ScenarioRiskError> {
    if outcomes.is_empty() {
        return Err(ScenarioRiskError::EmptyScenarioSet);
    }

    let mut ids = outcomes.iter().map(|outcome| outcome.scenario_id).collect::<Vec<_>>();
    ids.sort_unstable();
    for pair in ids.windows(2) {
        if pair[0] == pair[1] {
            return Err(ScenarioRiskError::DuplicateScenarioId(pair[0]));
        }
    }

    let total_probability: f64 = outcomes.iter().map(|outcome| outcome.probability.value()).sum();
    if !total_probability.is_finite() || !close(total_probability, 1.0) {
        return Err(ScenarioRiskError::ProbabilityMassDoesNotSumToOne);
    }

    for outcome in outcomes {
        if outcome.delivered_useful_heat.value() > outcome.requested_useful_heat.value() {
            return Err(ScenarioRiskError::DeliveredHeatExceedsDemand(outcome.scenario_id));
        }
    }
    Ok(total_probability)
}

/// Summarize service shortfall across a probability-weighted scenario set.
///
/// The method rejects duplicate IDs, non-normalized probability masses and
/// impossible delivered-heat values. It never samples, interpolates, or repairs
/// missing scenarios; those choices belong in the scenario manifest.
pub fn summarize_scenario_service_risk(
    outcomes: &[ScenarioServiceOutcome],
    minimum_service_fraction: Fraction,
) -> Result<ScenarioTailRiskReport, ScenarioRiskError> {
    validate_scenarios(outcomes)?;

    let mut expected_unmet = 0.0;
    let mut expected_shortfall = 0.0;
    let mut probability_below_floor = 0.0;
    let mut worst_unmet: f64 = 0.0;
    let mut worst_shortfall: f64 = 0.0;
    let mut worst_scenario_id = u64::MAX;
    let mut shortfalls = Vec::with_capacity(outcomes.len());

    for outcome in outcomes {
        let weight = outcome.probability.value();
        let requested = outcome.requested_useful_heat.value();
        let delivered = outcome.delivered_useful_heat.value();
        let unmet = requested - delivered;
        let shortfall = shortfall_fraction(*outcome);
        expected_unmet += weight * unmet;
        expected_shortfall += weight * shortfall;

        let service_fraction = if requested == 0.0 { 1.0 } else { delivered / requested };
        if service_fraction < minimum_service_fraction.value() {
            probability_below_floor += weight;
        }

        if shortfall > worst_shortfall
            || (shortfall == worst_shortfall && outcome.scenario_id < worst_scenario_id)
        {
            worst_shortfall = shortfall;
            worst_scenario_id = outcome.scenario_id;
        }
        if unmet > worst_unmet {
            worst_unmet = unmet;
        }
        shortfalls.push((shortfall, weight, outcome.scenario_id));
    }

    // Ascending tail order; scenario ID makes ties deterministic.
    shortfalls.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
    let mut cumulative = 0.0;
    let mut p95 = 0.0;
    for (shortfall, weight, _) in &shortfalls {
        cumulative += *weight;
        p95 = *shortfall;
        if cumulative + 1.0e-12 >= 0.95 {
            break;
        }
    }

    // CVaR95 is the mean of exactly the upper 5% probability mass. A scenario's
    // weight is fractionally included if the 5% boundary cuts through it.
    shortfalls.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.2.cmp(&b.2)));
    let mut tail_mass = 0.05;
    let mut tail_integral = 0.0;
    for (shortfall, weight, _) in &shortfalls {
        if tail_mass <= 1.0e-12 {
            break;
        }
        let take = (*weight).min(tail_mass);
        tail_integral += take * *shortfall;
        tail_mass -= take;
    }
    if tail_mass > 1.0e-9 {
        return Err(ScenarioRiskError::ProbabilityMassDoesNotSumToOne);
    }
    let cvar95 = tail_integral / 0.05;

    if !expected_unmet.is_finite()
        || !expected_shortfall.is_finite()
        || !probability_below_floor.is_finite()
        || !worst_unmet.is_finite()
        || !worst_shortfall.is_finite()
        || !p95.is_finite()
        || !cvar95.is_finite()
    {
        return Err(ScenarioRiskError::NonFiniteMetric);
    }

    Ok(ScenarioTailRiskReport {
        scenario_count: outcomes.len(),
        minimum_service_fraction,
        expected_unmet_useful_heat: EnergyJ::new(expected_unmet)
            .map_err(|_| ScenarioRiskError::NonFiniteMetric)?,
        worst_unmet_useful_heat: EnergyJ::new(worst_unmet)
            .map_err(|_| ScenarioRiskError::NonFiniteMetric)?,
        worst_shortfall_fraction: worst_shortfall,
        expected_shortfall_fraction: expected_shortfall,
        p95_shortfall_fraction: p95,
        cvar95_shortfall_fraction: cvar95,
        probability_below_service_floor: probability_below_floor,
        worst_case_scenario_id: worst_scenario_id,
    })
}

/// Independently recompute a tail-risk report without calling the summarizer.
pub fn verify_scenario_service_risk_report(
    outcomes: &[ScenarioServiceOutcome],
    minimum_service_fraction: Fraction,
    claimed: &ScenarioTailRiskReport,
) -> Result<(), ScenarioRiskError> {
    let total_probability = validate_scenarios(outcomes)?;
    if !close(claimed.minimum_service_fraction.value(), minimum_service_fraction.value()) {
        return Err(ScenarioRiskError::ReportMismatch("minimum_service_fraction"));
    }
    if claimed.scenario_count != outcomes.len() {
        return Err(ScenarioRiskError::ReportMismatch("scenario_count"));
    }

    let floor = minimum_service_fraction.value();
    let mut weighted_unmet = 0.0;
    let mut weighted_shortfall = 0.0;
    let mut floor_violation_mass = 0.0;
    let mut maximum_unmet: f64 = 0.0;
    let mut maximum_shortfall: f64 = 0.0;
    let mut worst_id = u64::MAX;

    for outcome in outcomes {
        let demand = outcome.requested_useful_heat.value();
        let delivery = outcome.delivered_useful_heat.value();
        let unmet = demand - delivery;
        let shortfall = if demand == 0.0 { 0.0 } else { unmet / demand };
        weighted_unmet += outcome.probability.value() * unmet;
        weighted_shortfall += outcome.probability.value() * shortfall;
        let service = if demand == 0.0 { 1.0 } else { delivery / demand };
        if service < floor {
            floor_violation_mass += outcome.probability.value();
        }
        if unmet > maximum_unmet {
            maximum_unmet = unmet;
        }
        if shortfall > maximum_shortfall
            || (shortfall == maximum_shortfall && outcome.scenario_id < worst_id)
        {
            maximum_shortfall = shortfall;
            worst_id = outcome.scenario_id;
        }
    }

    // The verifier builds a fresh ordered distribution and independently
    // computes the quantile and upper-tail integral.
    let mut distribution = outcomes.iter()
        .map(|outcome| {
            let demand = outcome.requested_useful_heat.value();
            let missing = demand - outcome.delivered_useful_heat.value();
            let ratio = if demand == 0.0 { 0.0 } else { missing / demand };
            (ratio, outcome.probability.value(), outcome.scenario_id)
        })
        .collect::<Vec<_>>();
    distribution.sort_by(|a, b| a.0.total_cmp(&b.0).then_with(|| a.2.cmp(&b.2)));
    let mut quantile_mass = 0.0;
    let mut quantile = 0.0;
    for point in &distribution {
        quantile_mass += point.1;
        quantile = point.0;
        if quantile_mass >= 0.95 - 1.0e-12 {
            break;
        }
    }
    distribution.sort_by(|a, b| b.0.total_cmp(&a.0).then_with(|| a.2.cmp(&b.2)));
    let mut remaining_tail = total_probability * 0.05;
    let target_tail = remaining_tail;
    let mut tail_total = 0.0;
    for point in &distribution {
        if remaining_tail <= 1.0e-12 {
            break;
        }
        let selected_mass = point.1.min(remaining_tail);
        tail_total += selected_mass * point.0;
        remaining_tail -= selected_mass;
    }
    if remaining_tail > 1.0e-9 || target_tail <= 0.0 {
        return Err(ScenarioRiskError::ProbabilityMassDoesNotSumToOne);
    }
    let cvar = tail_total / target_tail;

    let expected_unmet = EnergyJ::new(weighted_unmet)
        .map_err(|_| ScenarioRiskError::NonFiniteMetric)?;
    let worst_unmet = EnergyJ::new(maximum_unmet)
        .map_err(|_| ScenarioRiskError::NonFiniteMetric)?;
    let values = [
        (claimed.expected_unmet_useful_heat.value(), expected_unmet.value(), "expected_unmet"),
        (claimed.worst_unmet_useful_heat.value(), worst_unmet.value(), "worst_unmet"),
        (claimed.expected_shortfall_fraction, weighted_shortfall, "expected_shortfall"),
        (claimed.worst_shortfall_fraction, maximum_shortfall, "worst_shortfall"),
        (claimed.p95_shortfall_fraction, quantile, "p95_shortfall"),
        (claimed.cvar95_shortfall_fraction, cvar, "cvar95_shortfall"),
        (claimed.probability_below_service_floor, floor_violation_mass, "service_floor_probability"),
    ];
    for (actual, expected, field) in values {
        if !close(actual, expected) {
            return Err(ScenarioRiskError::ReportMismatch(field));
        }
    }
    if claimed.worst_case_scenario_id != worst_id {
        return Err(ScenarioRiskError::ReportMismatch("worst_case_scenario_id"));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(value: f64) -> EnergyJ { EnergyJ::new(value).unwrap() }
    fn p(value: f64) -> ScenarioProbability { ScenarioProbability::new(value).unwrap() }

    fn outcomes() -> Vec<ScenarioServiceOutcome> {
        vec![
            ScenarioServiceOutcome {
                scenario_id: 10,
                probability: p(0.80),
                requested_useful_heat: e(100.0),
                delivered_useful_heat: e(100.0),
            },
            ScenarioServiceOutcome {
                scenario_id: 20,
                probability: p(0.17),
                requested_useful_heat: e(100.0),
                delivered_useful_heat: e(80.0),
            },
            ScenarioServiceOutcome {
                scenario_id: 30,
                probability: p(0.03),
                requested_useful_heat: e(100.0),
                delivered_useful_heat: e(20.0),
            },
        ]
    }

    #[test]
    fn hand_calculable_distribution_reports_mean_p95_cvar_and_floor_risk() {
        let report = summarize_scenario_service_risk(
            &outcomes(), Fraction::new(0.8).unwrap(),
        ).unwrap();
        assert_eq!(report.scenario_count, 3);
        assert!(close(report.expected_unmet_useful_heat.value(), 5.8));
        assert!(close(report.worst_unmet_useful_heat.value(), 80.0));
        assert!(close(report.expected_shortfall_fraction, 0.058));
        assert!(close(report.p95_shortfall_fraction, 0.2));
        assert!(close(report.cvar95_shortfall_fraction, 0.56));
        assert!(close(report.worst_shortfall_fraction, 0.8));
        assert!(close(report.probability_below_service_floor, 0.03));
        assert_eq!(report.worst_case_scenario_id, 30);
        verify_scenario_service_risk_report(&outcomes(), Fraction::new(0.8).unwrap(), &report).unwrap();
    }

    #[test]
    fn scenario_order_does_not_change_summary_or_tie_break() {
        let original = outcomes();
        let mut reversed = original.clone();
        reversed.reverse();
        let a = summarize_scenario_service_risk(&original, Fraction::new(0.8).unwrap()).unwrap();
        let b = summarize_scenario_service_risk(&reversed, Fraction::new(0.8).unwrap()).unwrap();
        assert_eq!(a, b);
    }

    #[test]
    fn equal_worst_shortfalls_use_lowest_scenario_id() {
        let tied = vec![
            ScenarioServiceOutcome {
                scenario_id: 10,
                probability: p(0.80),
                requested_useful_heat: e(100.0),
                delivered_useful_heat: e(100.0),
            },
            ScenarioServiceOutcome {
                scenario_id: 40,
                probability: p(0.15),
                requested_useful_heat: e(100.0),
                delivered_useful_heat: e(80.0),
            },
            ScenarioServiceOutcome {
                scenario_id: 30,
                probability: p(0.025),
                requested_useful_heat: e(100.0),
                delivered_useful_heat: e(20.0),
            },
            ScenarioServiceOutcome {
                scenario_id: 20,
                probability: p(0.025),
                requested_useful_heat: e(100.0),
                delivered_useful_heat: e(20.0),
            },
        ];
        let report = summarize_scenario_service_risk(&tied, Fraction::new(0.8).unwrap()).unwrap();
        assert_eq!(report.worst_case_scenario_id, 20);
        assert!(close(report.p95_shortfall_fraction, 0.2));
        assert!(close(report.cvar95_shortfall_fraction, 0.8));
        verify_scenario_service_risk_report(&tied, Fraction::new(0.8).unwrap(), &report).unwrap();
    }

    #[test]
    fn empty_or_non_normalized_scenario_sets_are_rejected() {
        assert_eq!(
            summarize_scenario_service_risk(&[], Fraction::new(0.8).unwrap()),
            Err(ScenarioRiskError::EmptyScenarioSet)
        );
        let mut invalid = outcomes();
        invalid[0].probability = p(0.70);
        assert_eq!(
            summarize_scenario_service_risk(&invalid, Fraction::new(0.8).unwrap()),
            Err(ScenarioRiskError::ProbabilityMassDoesNotSumToOne)
        );
    }

    #[test]
    fn invalid_probability_duplicate_id_and_over_delivery_are_rejected() {
        assert_eq!(ScenarioProbability::new(0.0), Err(ScenarioRiskError::InvalidProbability));
        assert_eq!(ScenarioProbability::new(f64::NAN), Err(ScenarioRiskError::InvalidProbability));
        let mut duplicate = outcomes();
        duplicate[2].scenario_id = 20;
        assert_eq!(
            summarize_scenario_service_risk(&duplicate, Fraction::new(0.8).unwrap()),
            Err(ScenarioRiskError::DuplicateScenarioId(20))
        );
        let mut over_delivery = outcomes();
        over_delivery[1].delivered_useful_heat = e(101.0);
        assert_eq!(
            summarize_scenario_service_risk(&over_delivery, Fraction::new(0.8).unwrap()),
            Err(ScenarioRiskError::DeliveredHeatExceedsDemand(20))
        );
    }

    #[test]
    fn verifier_rejects_tampered_cvar_and_floor_probability() {
        let report = summarize_scenario_service_risk(
            &outcomes(), Fraction::new(0.8).unwrap(),
        ).unwrap();
        let bad_cvar = ScenarioTailRiskReport {
            cvar95_shortfall_fraction: 0.1,
            ..report.clone()
        };
        assert_eq!(
            verify_scenario_service_risk_report(&outcomes(), Fraction::new(0.8).unwrap(), &bad_cvar),
            Err(ScenarioRiskError::ReportMismatch("cvar95_shortfall"))
        );
        let bad_floor = ScenarioTailRiskReport {
            probability_below_service_floor: 0.0,
            ..report
        };
        assert_eq!(
            verify_scenario_service_risk_report(&outcomes(), Fraction::new(0.8).unwrap(), &bad_floor),
            Err(ScenarioRiskError::ReportMismatch("service_floor_probability"))
        );
        let tampered_threshold = ScenarioTailRiskReport {
            minimum_service_fraction: Fraction::new(0.0).unwrap(),
            probability_below_service_floor: 0.0,
            ..summarize_scenario_service_risk(
                &outcomes(), Fraction::new(0.0).unwrap(),
            ).unwrap()
        };
        assert_eq!(
            verify_scenario_service_risk_report(
                &outcomes(), Fraction::new(0.8).unwrap(), &tampered_threshold,
            ),
            Err(ScenarioRiskError::ReportMismatch("minimum_service_fraction"))
        );
    }

    #[test]
    fn zero_demand_is_defined_as_no_service_shortfall() {
        let outcomes = [ScenarioServiceOutcome {
            scenario_id: 1,
            probability: p(1.0),
            requested_useful_heat: e(0.0),
            delivered_useful_heat: e(0.0),
        }];
        let report = summarize_scenario_service_risk(&outcomes, Fraction::new(1.0).unwrap()).unwrap();
        assert_eq!(report.expected_shortfall_fraction, 0.0);
        assert_eq!(report.p95_shortfall_fraction, 0.0);
        assert_eq!(report.cvar95_shortfall_fraction, 0.0);
        assert_eq!(report.probability_below_service_floor, 0.0);
        verify_scenario_service_risk_report(&outcomes, Fraction::new(1.0).unwrap(), &report).unwrap();
    }
}
