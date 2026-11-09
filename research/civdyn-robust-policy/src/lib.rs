// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

//! Decision robustness over a declared CivDyn parameter design.
//!
//! This crate consumes the existing parameterized resilience kernel. It does
//! not create a physical simulator, assign probabilities to LHS rows, or
//! combine native-unit outcomes into one universal policy score.

use civdyn_cross_scale_resilience::{InterventionPolicy, ShockKind};
use civdyn_resilience_uq::{
    DesignPoint, OutcomeMetric, ParameterSet, RunMetrics, UqError, latin_hypercube_design,
    run_parameterized_scenario,
};
use core::fmt;
use std::collections::{BTreeMap, BTreeSet};
pub mod model_set_robustness;

pub const DEFAULT_LHS_POINTS: usize = 30;
pub const DEFAULT_SERVICE_FLOOR: f64 = 0.95;
pub const DEFAULT_MAX_BREACH_FRACTION: f64 = 0.05;
const TIE_TOLERANCE: f64 = 1e-12;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ServiceConstraints {
    pub minimum_water_coverage: f64,
    pub minimum_energy_coverage: f64,
    pub maximum_water_breach_fraction: f64,
    pub maximum_energy_breach_fraction: f64,
}

impl Default for ServiceConstraints {
    fn default() -> Self {
        Self {
            minimum_water_coverage: DEFAULT_SERVICE_FLOOR,
            minimum_energy_coverage: DEFAULT_SERVICE_FLOOR,
            maximum_water_breach_fraction: DEFAULT_MAX_BREACH_FRACTION,
            maximum_energy_breach_fraction: DEFAULT_MAX_BREACH_FRACTION,
        }
    }
}

impl ServiceConstraints {
    pub fn validate(self) -> Result<(), DecisionError> {
        for (name, value) in [
            ("minimum_water_coverage", self.minimum_water_coverage),
            ("minimum_energy_coverage", self.minimum_energy_coverage),
            (
                "maximum_water_breach_fraction",
                self.maximum_water_breach_fraction,
            ),
            (
                "maximum_energy_breach_fraction",
                self.maximum_energy_breach_fraction,
            ),
        ] {
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                return Err(DecisionError::InvalidConstraint { name, value });
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyRun {
    pub design_point: DesignPoint,
    pub shock_kind: ShockKind,
    pub policy: InterventionPolicy,
    /// None explicitly represents an unavailable/withheld outcome. It is never a zero.
    pub metrics: Option<RunMetrics>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConstraintStatus {
    Pass,
    Fail,
    Unknown,
}

impl ConstraintStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Pass => "pass",
            Self::Fail => "fail",
            Self::Unknown => "unknown",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct PolicyConstraintSummary {
    pub shock_kind: ShockKind,
    pub policy: InterventionPolicy,
    pub design_point_count: usize,
    pub pass_count: usize,
    pub fail_count: usize,
    pub unknown_count: usize,
    /// Some(true) only if every declared design point passes. Some(false) if
    /// at least one fails. None if no failure is observed but at least one is unknown.
    pub finite_design_robust_feasible: Option<bool>,
    /// Minimum observed margins over known runs. These are not extrapolated
    /// across the continuous parameter box.
    pub worst_water_coverage_margin: Option<f64>,
    pub worst_energy_coverage_margin: Option<f64>,
    pub worst_water_breach_margin: Option<f64>,
    pub worst_energy_breach_margin: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct MetricRegretSummary {
    pub shock_kind: ShockKind,
    pub policy: InterventionPolicy,
    pub metric: OutcomeMetric,
    pub design_point_count: usize,
    pub valid_point_count: usize,
    pub unknown_point_count: usize,
    /// Mean regret in this metric's native unit over valid matched design points.
    pub mean_regret: Option<f64>,
    pub p95_regret: Option<f64>,
    pub worst_regret: Option<f64>,
    /// Number of parameter points where this policy is tied for the metric-wise
    /// best outcome. Ties can make best_count sum to more than design_point_count.
    pub best_count: usize,
    pub best_fraction: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct RobustPolicyReport {
    pub seed: u64,
    pub lhs_point_count: usize,
    pub constraints: ServiceConstraints,
    pub constraint_summaries: Vec<PolicyConstraintSummary>,
    pub metric_regrets: Vec<MetricRegretSummary>,
    /// Distinct winning policies per shock/metric across the finite design.
    /// This is decision instability, not a ranking across unlike metrics.
    pub distinct_winner_counts: Vec<DistinctWinnerSummary>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DistinctWinnerSummary {
    pub shock_kind: ShockKind,
    pub metric: OutcomeMetric,
    pub distinct_best_policy_count: usize,
    pub design_points_with_unique_winner: usize,
    pub design_points_with_tied_winners: usize,
    pub design_points_with_unknown_comparison: usize,
}

#[derive(Debug, Clone, PartialEq)]
pub enum DecisionError {
    Uq(UqError),
    EmptyRuns,
    InvalidConstraint {
        name: &'static str,
        value: f64,
    },
    InvalidDesignId,
    InvalidDesignPoint {
        design_id: String,
    },
    InconsistentDesignPoint {
        design_id: String,
    },
    DuplicatePolicyRun {
        design_id: String,
        shock: &'static str,
        policy: &'static str,
    },
    MissingPolicyRun {
        design_id: String,
        shock: &'static str,
        policy: &'static str,
    },
    NonFiniteOutcome {
        design_id: String,
        shock: &'static str,
        policy: &'static str,
        metric: &'static str,
        value: f64,
    },
}

impl fmt::Display for DecisionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Uq(error) => write!(f, "parameterized resilience error: {error}"),
            Self::EmptyRuns => write!(f, "policy evaluation requires at least one run"),
            Self::InvalidConstraint { name, value } => {
                write!(f, "{name} must be finite and in [0, 1], got {value}")
            }
            Self::InvalidDesignId => write!(f, "design point ID must not be empty"),
            Self::InvalidDesignPoint { design_id } => {
                write!(
                    f,
                    "design point {design_id} has invalid parameter coordinates"
                )
            }
            Self::InconsistentDesignPoint { design_id } => {
                write!(
                    f,
                    "design ID {design_id} maps to inconsistent parameter coordinates"
                )
            }
            Self::DuplicatePolicyRun {
                design_id,
                shock,
                policy,
            } => write!(
                f,
                "duplicate policy run for design {design_id}, shock {shock}, policy {policy}"
            ),
            Self::MissingPolicyRun {
                design_id,
                shock,
                policy,
            } => write!(
                f,
                "missing policy run for design {design_id}, shock {shock}, policy {policy}"
            ),
            Self::NonFiniteOutcome {
                design_id,
                shock,
                policy,
                metric,
                value,
            } => write!(
                f,
                "design {design_id}, shock {shock}, policy {policy}, metric {metric} is non-finite: {value}"
            ),
        }
    }
}

impl std::error::Error for DecisionError {}

impl From<UqError> for DecisionError {
    fn from(value: UqError) -> Self {
        Self::Uq(value)
    }
}

/// Run all 3 declared shocks × 4 policies against one common Latin-hypercube
/// parameter design. LHS rows are design points, not stochastic replications,
/// empirical priors or probability-weighted future scenarios.
pub fn execute_policy_design(
    seed: u64,
    lhs_point_count: usize,
    constraints: ServiceConstraints,
) -> Result<(Vec<PolicyRun>, RobustPolicyReport), DecisionError> {
    constraints.validate()?;
    let design = latin_hypercube_design(seed, lhs_point_count)?;
    let mut runs =
        Vec::with_capacity(design.len() * ShockKind::ALL.len() * InterventionPolicy::ALL.len());

    for design_point in design {
        for shock_kind in ShockKind::ALL {
            for policy in InterventionPolicy::ALL {
                let metrics =
                    run_parameterized_scenario(shock_kind, policy, design_point.parameters)?;
                runs.push(PolicyRun {
                    design_point: design_point.clone(),
                    shock_kind,
                    policy,
                    metrics: Some(metrics),
                });
            }
        }
    }

    let report = analyze_policy_runs(seed, constraints, &runs)?;
    Ok((runs, report))
}

/// Analyze a complete matched design. Reordering rows does not change the result.
/// Every design point must contain one record for every declared shock and policy;
/// a withheld outcome is represented as None and remains UNKNOWN.
pub fn analyze_policy_runs(
    seed: u64,
    constraints: ServiceConstraints,
    runs: &[PolicyRun],
) -> Result<RobustPolicyReport, DecisionError> {
    constraints.validate()?;
    if runs.is_empty() {
        return Err(DecisionError::EmptyRuns);
    }

    let mut design_points = BTreeMap::<String, DesignPoint>::new();
    let mut records = BTreeMap::<(String, &'static str, &'static str), &PolicyRun>::new();

    for run in runs {
        let design_id = run.design_point.design_id.clone();
        if design_id.trim().is_empty() {
            return Err(DecisionError::InvalidDesignId);
        }
        run.design_point
            .parameters
            .validate()
            .map_err(|_| DecisionError::InvalidDesignPoint {
                design_id: design_id.clone(),
            })?;
        if run
            .design_point
            .unit_coordinates
            .iter()
            .any(|value| !value.is_finite() || !(0.0..=1.0).contains(value))
        {
            return Err(DecisionError::InvalidDesignPoint { design_id });
        }
        let mapped_parameters = ParameterSet::from_unit_coordinates(
            run.design_point.unit_coordinates,
        )
        .map_err(|_| DecisionError::InvalidDesignPoint {
            design_id: run.design_point.design_id.clone(),
        })?;
        if mapped_parameters != run.design_point.parameters {
            return Err(DecisionError::InvalidDesignPoint {
                design_id: run.design_point.design_id.clone(),
            });
        }

        if let Some(previous) = design_points.get(&run.design_point.design_id) {
            if previous != &run.design_point {
                return Err(DecisionError::InconsistentDesignPoint {
                    design_id: run.design_point.design_id.clone(),
                });
            }
        } else {
            design_points.insert(run.design_point.design_id.clone(), run.design_point.clone());
        }

        if let Some(metrics) = run.metrics {
            for metric in OutcomeMetric::ALL {
                let value = metrics.value(metric);
                if !value.is_finite() {
                    return Err(DecisionError::NonFiniteOutcome {
                        design_id: run.design_point.design_id.clone(),
                        shock: run.shock_kind.name(),
                        policy: run.policy.name(),
                        metric: metric.name(),
                        value,
                    });
                }
            }
        }

        let key = (
            run.design_point.design_id.clone(),
            run.shock_kind.name(),
            run.policy.name(),
        );
        if records.insert(key, run).is_some() {
            return Err(DecisionError::DuplicatePolicyRun {
                design_id: run.design_point.design_id.clone(),
                shock: run.shock_kind.name(),
                policy: run.policy.name(),
            });
        }
    }

    let design_ids = design_points.keys().cloned().collect::<Vec<_>>();
    for design_id in &design_ids {
        for shock in ShockKind::ALL {
            for policy in InterventionPolicy::ALL {
                let key = (design_id.clone(), shock.name(), policy.name());
                if !records.contains_key(&key) {
                    return Err(DecisionError::MissingPolicyRun {
                        design_id: design_id.clone(),
                        shock: shock.name(),
                        policy: policy.name(),
                    });
                }
            }
        }
    }

    let mut constraint_summaries =
        Vec::with_capacity(ShockKind::ALL.len() * InterventionPolicy::ALL.len());
    for shock in ShockKind::ALL {
        for policy in InterventionPolicy::ALL {
            constraint_summaries.push(summarize_constraints(
                shock,
                policy,
                &design_ids,
                &records,
                constraints,
            ));
        }
    }

    let mut metric_regrets = Vec::with_capacity(
        ShockKind::ALL.len() * InterventionPolicy::ALL.len() * OutcomeMetric::ALL.len(),
    );
    let mut distinct_winner_counts =
        Vec::with_capacity(ShockKind::ALL.len() * OutcomeMetric::ALL.len());

    for shock in ShockKind::ALL {
        for metric in OutcomeMetric::ALL {
            let mut regret_values: BTreeMap<&'static str, Vec<f64>> = InterventionPolicy::ALL
                .iter()
                .map(|policy| (policy.name(), Vec::with_capacity(design_ids.len())))
                .collect();
            let mut unknown_counts: BTreeMap<&'static str, usize> = InterventionPolicy::ALL
                .iter()
                .map(|policy| (policy.name(), 0))
                .collect();
            let mut best_counts: BTreeMap<&'static str, usize> = InterventionPolicy::ALL
                .iter()
                .map(|policy| (policy.name(), 0))
                .collect();
            let mut distinct_best = BTreeSet::new();
            let mut unique_winner_points = 0usize;
            let mut tied_winner_points = 0usize;
            let mut unknown_comparison_points = 0usize;

            for design_id in &design_ids {
                let point_runs = InterventionPolicy::ALL
                    .iter()
                    .map(|policy| records[&(design_id.clone(), shock.name(), policy.name())])
                    .collect::<Vec<_>>();
                let values = point_runs
                    .iter()
                    .map(|run| run.metrics.map(|metrics| metrics.value(metric)))
                    .collect::<Vec<_>>();

                if values.iter().any(Option::is_none) {
                    unknown_comparison_points += 1;
                    for policy in InterventionPolicy::ALL {
                        *unknown_counts
                            .get_mut(policy.name())
                            .expect("policy exists") += 1;
                    }
                    continue;
                }

                let numeric = values
                    .iter()
                    .map(|value| value.unwrap())
                    .collect::<Vec<_>>();
                let best = best_value(metric, &numeric);
                let winners = InterventionPolicy::ALL
                    .iter()
                    .zip(&numeric)
                    .filter(|(_, value)| (*value - best).abs() <= TIE_TOLERANCE)
                    .map(|(policy, _)| policy.name())
                    .collect::<Vec<_>>();
                distinct_best.extend(winners.iter().copied());
                if winners.len() == 1 {
                    unique_winner_points += 1;
                } else {
                    tied_winner_points += 1;
                }

                for (policy, value) in InterventionPolicy::ALL.iter().zip(numeric) {
                    let regret = metric_regret(metric, value, best);
                    regret_values
                        .get_mut(policy.name())
                        .expect("policy exists")
                        .push(regret);
                    if winners.contains(&policy.name()) {
                        *best_counts.get_mut(policy.name()).expect("policy exists") += 1;
                    }
                }
            }

            for policy in InterventionPolicy::ALL {
                let values = &regret_values[policy.name()];
                let mut sorted = values.clone();
                sorted.sort_by(f64::total_cmp);
                metric_regrets.push(MetricRegretSummary {
                    shock_kind: shock,
                    policy,
                    metric,
                    design_point_count: design_ids.len(),
                    valid_point_count: sorted.len(),
                    unknown_point_count: unknown_counts[policy.name()],
                    mean_regret: average_option(&sorted),
                    p95_regret: percentile_option(&sorted, 0.95),
                    worst_regret: sorted.last().copied(),
                    best_count: best_counts[policy.name()],
                    best_fraction: best_counts[policy.name()] as f64 / design_ids.len() as f64,
                });
            }

            distinct_winner_counts.push(DistinctWinnerSummary {
                shock_kind: shock,
                metric,
                distinct_best_policy_count: distinct_best.len(),
                design_points_with_unique_winner: unique_winner_points,
                design_points_with_tied_winners: tied_winner_points,
                design_points_with_unknown_comparison: unknown_comparison_points,
            });
        }
    }

    Ok(RobustPolicyReport {
        seed,
        lhs_point_count: design_ids.len(),
        constraints,
        constraint_summaries,
        metric_regrets,
        distinct_winner_counts,
    })
}

fn summarize_constraints(
    shock: ShockKind,
    policy: InterventionPolicy,
    design_ids: &[String],
    records: &BTreeMap<(String, &'static str, &'static str), &PolicyRun>,
    constraints: ServiceConstraints,
) -> PolicyConstraintSummary {
    let mut pass_count = 0usize;
    let mut fail_count = 0usize;
    let mut unknown_count = 0usize;
    let mut water_margin: Option<f64> = None;
    let mut energy_margin: Option<f64> = None;
    let mut water_breach_margin: Option<f64> = None;
    let mut energy_breach_margin: Option<f64> = None;

    for design_id in design_ids {
        let run = records[&(design_id.clone(), shock.name(), policy.name())];
        let Some(metrics) = run.metrics else {
            unknown_count += 1;
            continue;
        };

        let margins = [
            metrics.minimum_water_coverage - constraints.minimum_water_coverage,
            metrics.minimum_energy_coverage - constraints.minimum_energy_coverage,
            constraints.maximum_water_breach_fraction - metrics.water_floor_breach_fraction,
            constraints.maximum_energy_breach_fraction - metrics.energy_floor_breach_fraction,
        ];
        water_margin = min_option(water_margin, margins[0]);
        energy_margin = min_option(energy_margin, margins[1]);
        water_breach_margin = min_option(water_breach_margin, margins[2]);
        energy_breach_margin = min_option(energy_breach_margin, margins[3]);

        if margins.iter().any(|margin| *margin < -TIE_TOLERANCE) {
            fail_count += 1;
        } else {
            pass_count += 1;
        }
    }

    let finite_design_robust_feasible = if fail_count > 0 {
        Some(false)
    } else if unknown_count > 0 {
        None
    } else {
        Some(true)
    };

    PolicyConstraintSummary {
        shock_kind: shock,
        policy,
        design_point_count: design_ids.len(),
        pass_count,
        fail_count,
        unknown_count,
        finite_design_robust_feasible,
        worst_water_coverage_margin: water_margin,
        worst_energy_coverage_margin: energy_margin,
        worst_water_breach_margin: water_breach_margin,
        worst_energy_breach_margin: energy_breach_margin,
    }
}

fn best_value(metric: OutcomeMetric, values: &[f64]) -> f64 {
    let minimize = matches!(
        metric,
        OutcomeMetric::TotalUnmetWaterM3
            | OutcomeMetric::TotalUnmetEnergyKwh
            | OutcomeMetric::WaterDeficitAreaHouseholdSteps
            | OutcomeMetric::EnergyDeficitAreaHouseholdSteps
            | OutcomeMetric::WaterFloorBreachFraction
            | OutcomeMetric::EnergyFloorBreachFraction
    );
    if minimize {
        values.iter().copied().fold(f64::INFINITY, f64::min)
    } else {
        values.iter().copied().fold(f64::NEG_INFINITY, f64::max)
    }
}

fn metric_regret(metric: OutcomeMetric, value: f64, best: f64) -> f64 {
    let minimize = matches!(
        metric,
        OutcomeMetric::TotalUnmetWaterM3
            | OutcomeMetric::TotalUnmetEnergyKwh
            | OutcomeMetric::WaterDeficitAreaHouseholdSteps
            | OutcomeMetric::EnergyDeficitAreaHouseholdSteps
            | OutcomeMetric::WaterFloorBreachFraction
            | OutcomeMetric::EnergyFloorBreachFraction
    );
    if minimize {
        (value - best).max(0.0)
    } else {
        (best - value).max(0.0)
    }
}

fn average_option(values: &[f64]) -> Option<f64> {
    if values.is_empty() {
        None
    } else {
        Some(values.iter().sum::<f64>() / values.len() as f64)
    }
}

fn percentile_option(sorted: &[f64], probability: f64) -> Option<f64> {
    if sorted.is_empty() {
        return None;
    }
    if sorted.len() == 1 {
        return Some(sorted[0]);
    }
    let position = probability * (sorted.len() - 1) as f64;
    let lower = position.floor() as usize;
    let upper = position.ceil() as usize;
    if lower == upper {
        Some(sorted[lower])
    } else {
        let weight = position - lower as f64;
        Some(sorted[lower] * (1.0 - weight) + sorted[upper] * weight)
    }
}

fn min_option(current: Option<f64>, candidate: f64) -> Option<f64> {
    Some(current.map_or(candidate, |value| value.min(candidate)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use civdyn_resilience_uq::{ParameterSet, RunMetrics};

    fn design_point(id: &str, coordinate: f64) -> DesignPoint {
        let unit_coordinates = [coordinate; 7];
        DesignPoint {
            design_id: id.into(),
            unit_coordinates,
            parameters: ParameterSet::from_unit_coordinates(unit_coordinates).unwrap(),
        }
    }

    fn metrics(
        water_unmet: f64,
        energy_unmet: f64,
        water_coverage: f64,
        energy_coverage: f64,
    ) -> RunMetrics {
        RunMetrics {
            total_unmet_water_m3: water_unmet,
            total_unmet_energy_kwh: energy_unmet,
            water_deficit_area_household_steps: water_unmet / 10.0,
            energy_deficit_area_household_steps: energy_unmet / 10.0,
            minimum_water_coverage: water_coverage,
            minimum_energy_coverage: energy_coverage,
            water_floor_breach_fraction: 1.0 - water_coverage,
            energy_floor_breach_fraction: 1.0 - energy_coverage,
            recovery_success: if water_coverage >= 0.95 && energy_coverage >= 0.95 {
                1.0
            } else {
                0.0
            },
        }
    }

    fn complete_fixture() -> Vec<PolicyRun> {
        let mut runs = Vec::new();
        for (id, coordinate) in [("lhs_00", 0.1), ("lhs_01", 0.9)] {
            let point = design_point(id, coordinate);
            for shock in ShockKind::ALL {
                for policy in InterventionPolicy::ALL {
                    let severity = match shock {
                        ShockKind::WaterSupply => 0.1,
                        ShockKind::EnergySupply => 0.2,
                        ShockKind::CompoundWaterAndEnergy => 0.3,
                    };
                    // The synthetic policy response is deliberately directional:
                    // no-transfer has the largest unmet demand, followed by proportional
                    // solidarity, reserve protection, and deficit-first allocation.
                    let policy_effect = match policy {
                        InterventionPolicy::NoTransfer => 0.30,
                        InterventionPolicy::ProportionalSolidarity => 0.15,
                        InterventionPolicy::ReserveProtected => 0.10,
                        InterventionPolicy::DeficitFirst => 0.05,
                    };
                    let service_coverage = 0.99 - severity * 0.02 - policy_effect * 0.02;
                    runs.push(PolicyRun {
                        design_point: point.clone(),
                        shock_kind: shock,
                        policy,
                        metrics: Some(metrics(
                            severity + policy_effect,
                            severity * 10.0 + policy_effect * 10.0,
                            service_coverage,
                            service_coverage,
                        )),
                    });
                }
            }
        }
        runs
    }

    #[test]
    fn same_policy_values_have_zero_metric_regret() {
        let mut runs = complete_fixture();
        for run in &mut runs {
            run.metrics = Some(metrics(1.0, 2.0, 0.97, 0.96));
        }
        let report = analyze_policy_runs(7, ServiceConstraints::default(), &runs).unwrap();
        assert!(
            report
                .metric_regrets
                .iter()
                .all(|entry| entry.worst_regret == Some(0.0))
        );
        assert!(
            report
                .metric_regrets
                .iter()
                .all(|entry| entry.best_count == 2)
        );
    }

    #[test]
    fn metric_regret_is_directional_and_keeps_native_resource_units() {
        let report =
            analyze_policy_runs(7, ServiceConstraints::default(), &complete_fixture()).unwrap();
        let no_transfer_water = report
            .metric_regrets
            .iter()
            .find(|entry| {
                entry.shock_kind == ShockKind::WaterSupply
                    && entry.policy == InterventionPolicy::NoTransfer
                    && entry.metric == OutcomeMetric::TotalUnmetWaterM3
            })
            .unwrap();
        let no_transfer_energy = report
            .metric_regrets
            .iter()
            .find(|entry| {
                entry.shock_kind == ShockKind::EnergySupply
                    && entry.policy == InterventionPolicy::NoTransfer
                    && entry.metric == OutcomeMetric::TotalUnmetEnergyKwh
            })
            .unwrap();
        assert!(no_transfer_water.worst_regret.unwrap() > 0.0);
        assert!(no_transfer_energy.worst_regret.unwrap() > 0.0);
        assert_ne!(
            no_transfer_water.worst_regret,
            no_transfer_energy.worst_regret
        );
    }

    #[test]
    fn one_service_floor_failure_withholds_finite_design_robust_pass() {
        let mut runs = complete_fixture();
        let target = runs
            .iter_mut()
            .find(|run| {
                run.design_point.design_id == "lhs_01"
                    && run.shock_kind == ShockKind::CompoundWaterAndEnergy
                    && run.policy == InterventionPolicy::ReserveProtected
            })
            .unwrap();
        target.metrics = Some(metrics(10.0, 10.0, 0.80, 0.90));
        let report = analyze_policy_runs(8, ServiceConstraints::default(), &runs).unwrap();
        let summary = report
            .constraint_summaries
            .iter()
            .find(|entry| {
                entry.shock_kind == ShockKind::CompoundWaterAndEnergy
                    && entry.policy == InterventionPolicy::ReserveProtected
            })
            .unwrap();
        assert_eq!(summary.finite_design_robust_feasible, Some(false));
        assert_eq!(summary.fail_count, 1);
        assert_eq!(summary.pass_count, 1);
    }

    #[test]
    fn missing_outcomes_remain_unknown_not_zero_or_pass() {
        let mut runs = complete_fixture();
        let target = runs
            .iter_mut()
            .find(|run| {
                run.design_point.design_id == "lhs_01"
                    && run.shock_kind == ShockKind::WaterSupply
                    && run.policy == InterventionPolicy::NoTransfer
            })
            .unwrap();
        target.metrics = None;
        let report = analyze_policy_runs(9, ServiceConstraints::default(), &runs).unwrap();
        let constraints = report
            .constraint_summaries
            .iter()
            .find(|entry| {
                entry.shock_kind == ShockKind::WaterSupply
                    && entry.policy == InterventionPolicy::NoTransfer
            })
            .unwrap();
        assert_eq!(constraints.unknown_count, 1);
        assert_eq!(constraints.finite_design_robust_feasible, None);
        let regrets = report
            .metric_regrets
            .iter()
            .filter(|entry| {
                entry.shock_kind == ShockKind::WaterSupply
                    && entry.policy == InterventionPolicy::NoTransfer
            })
            .collect::<Vec<_>>();
        assert!(regrets.iter().all(|entry| entry.unknown_point_count == 1));
    }

    #[test]
    fn row_order_does_not_change_the_report() {
        let runs = complete_fixture();
        let first = analyze_policy_runs(10, ServiceConstraints::default(), &runs).unwrap();
        let reversed = runs.iter().cloned().rev().collect::<Vec<_>>();
        let second = analyze_policy_runs(10, ServiceConstraints::default(), &reversed).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn duplicate_missing_and_inconsistent_design_records_fail_closed() {
        let mut duplicate = complete_fixture();
        duplicate.push(duplicate[0].clone());
        assert!(matches!(
            analyze_policy_runs(1, ServiceConstraints::default(), &duplicate),
            Err(DecisionError::DuplicatePolicyRun { .. })
        ));

        let mut missing = complete_fixture();
        missing.pop();
        assert!(matches!(
            analyze_policy_runs(1, ServiceConstraints::default(), &missing),
            Err(DecisionError::MissingPolicyRun { .. })
        ));

        let mut inconsistent = complete_fixture();
        inconsistent[1].design_point.unit_coordinates = [0.5; 7];
        inconsistent[1].design_point.parameters =
            ParameterSet::from_unit_coordinates([0.5; 7]).unwrap();
        assert!(matches!(
            analyze_policy_runs(1, ServiceConstraints::default(), &inconsistent),
            Err(DecisionError::InconsistentDesignPoint { .. })
        ));
    }
}
