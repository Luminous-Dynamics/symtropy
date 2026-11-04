// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent weighted-scenario service and tail-risk evaluation.
//!
//! This module evaluates a finite, frozen joint scenario ensemble. It does not
//! generate scenarios, fit a probability model, optimize a schedule or provide
//! a distributionally robust guarantee. Weighted empirical CVaR is not worst-
//! case DRO-CVaR. Scenario-set digests and evidence IDs are retained so the
//! ensemble and the evaluated policy can be audited separately.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

/// Frozen evaluation identity and caller-declared service/risk gates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioRiskContract {
    benchmark_id: String,
    strategy_id: String,
    scenario_set_digest: String,
    service_contract_digest: String,
    evaluation_configuration_digest: String,
    cvar_confidence_level: f64,
    maximum_power_shortfall_probability: f64,
    maximum_energy_shortfall_probability: f64,
    maximum_deadline_violation_probability: f64,
    maximum_thermal_violation_probability: f64,
    maximum_reserve_breach_probability: f64,
    maximum_peak_power_shortfall_cvar_kw: f64,
    maximum_energy_shortfall_cvar_kwh: f64,
    minimum_expected_work_completion_fraction: f64,
    minimum_worst_case_work_completion_fraction: f64,
    probability_sum_tolerance: f64,
}

impl ScenarioRiskContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        benchmark_id: impl Into<String>,
        strategy_id: impl Into<String>,
        scenario_set_digest: impl Into<String>,
        service_contract_digest: impl Into<String>,
        evaluation_configuration_digest: impl Into<String>,
        cvar_confidence_level: f64,
        maximum_power_shortfall_probability: f64,
        maximum_energy_shortfall_probability: f64,
        maximum_deadline_violation_probability: f64,
        maximum_thermal_violation_probability: f64,
        maximum_reserve_breach_probability: f64,
        maximum_peak_power_shortfall_cvar_kw: f64,
        maximum_energy_shortfall_cvar_kwh: f64,
        minimum_expected_work_completion_fraction: f64,
        minimum_worst_case_work_completion_fraction: f64,
        probability_sum_tolerance: f64,
    ) -> Result<Self, ScenarioRiskError> {
        let contract = Self {
            benchmark_id: benchmark_id.into(),
            strategy_id: strategy_id.into(),
            scenario_set_digest: scenario_set_digest.into(),
            service_contract_digest: service_contract_digest.into(),
            evaluation_configuration_digest: evaluation_configuration_digest.into(),
            cvar_confidence_level,
            maximum_power_shortfall_probability,
            maximum_energy_shortfall_probability,
            maximum_deadline_violation_probability,
            maximum_thermal_violation_probability,
            maximum_reserve_breach_probability,
            maximum_peak_power_shortfall_cvar_kw,
            maximum_energy_shortfall_cvar_kwh,
            minimum_expected_work_completion_fraction,
            minimum_worst_case_work_completion_fraction,
            probability_sum_tolerance,
        };
        for (name, value) in [
            ("benchmark ID", &contract.benchmark_id),
            ("strategy ID", &contract.strategy_id),
            ("scenario-set digest", &contract.scenario_set_digest),
            ("service-contract digest", &contract.service_contract_digest),
            ("evaluation configuration digest", &contract.evaluation_configuration_digest),
        ] {
            require_id(name, value)?;
        }
        if !contract.cvar_confidence_level.is_finite()
            || contract.cvar_confidence_level <= 0.0
            || contract.cvar_confidence_level >= 1.0
        {
            return Err(ScenarioRiskError::InvalidProbability {
                name: "CVaR confidence level",
                value: contract.cvar_confidence_level,
            });
        }
        for (name, value) in [
            ("maximum power-shortfall probability", contract.maximum_power_shortfall_probability),
            ("maximum energy-shortfall probability", contract.maximum_energy_shortfall_probability),
            ("maximum deadline-violation probability", contract.maximum_deadline_violation_probability),
            ("maximum thermal-violation probability", contract.maximum_thermal_violation_probability),
            ("maximum reserve-breach probability", contract.maximum_reserve_breach_probability),
            ("minimum expected work completion", contract.minimum_expected_work_completion_fraction),
            ("minimum worst-case work completion", contract.minimum_worst_case_work_completion_fraction),
        ] {
            require_probability(name, value, true)?;
        }
        for (name, value) in [
            ("maximum peak power-shortfall CVaR", contract.maximum_peak_power_shortfall_cvar_kw),
            ("maximum energy-shortfall CVaR", contract.maximum_energy_shortfall_cvar_kwh),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ScenarioRiskError::InvalidQuantity { name, value });
            }
        }
        if !contract.probability_sum_tolerance.is_finite()
            || contract.probability_sum_tolerance <= 0.0
            || contract.probability_sum_tolerance > 1e-3
        {
            return Err(ScenarioRiskError::InvalidProbabilityTolerance(
                contract.probability_sum_tolerance,
            ));
        }
        Ok(contract)
    }

    pub fn benchmark_id(&self) -> &str {
        &self.benchmark_id
    }

    pub fn strategy_id(&self) -> &str {
        &self.strategy_id
    }

    pub fn scenario_set_digest(&self) -> &str {
        &self.scenario_set_digest
    }

    pub const fn cvar_confidence_level(&self) -> f64 {
        self.cvar_confidence_level
    }
}

/// One candidate strategy's outcome in one jointly sampled scenario.
///
/// The probability weight belongs to the frozen scenario ensemble. Do not
/// assemble a joint scenario by multiplying independent marginal weights;
/// encode its joint identity and dependence in the scenario-set digest.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioRiskOutcome {
    scenario_id: String,
    scenario_digest: String,
    probability_weight: f64,
    operating_cost: f64,
    facility_energy_kwh: f64,
    completed_work_fraction: f64,
    peak_power_shortfall_kw: f64,
    energy_shortfall_kwh: f64,
    deadline_violated: bool,
    thermal_limit_violated: bool,
    protected_reserve_breached: bool,
    workload_evidence_id: String,
    power_evidence_id: String,
    thermal_evidence_id: String,
    storage_evidence_id: String,
}

impl ScenarioRiskOutcome {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        scenario_id: impl Into<String>,
        scenario_digest: impl Into<String>,
        probability_weight: f64,
        operating_cost: f64,
        facility_energy_kwh: f64,
        completed_work_fraction: f64,
        peak_power_shortfall_kw: f64,
        energy_shortfall_kwh: f64,
        deadline_violated: bool,
        thermal_limit_violated: bool,
        protected_reserve_breached: bool,
        workload_evidence_id: impl Into<String>,
        power_evidence_id: impl Into<String>,
        thermal_evidence_id: impl Into<String>,
        storage_evidence_id: impl Into<String>,
    ) -> Result<Self, ScenarioRiskError> {
        let outcome = Self {
            scenario_id: scenario_id.into(),
            scenario_digest: scenario_digest.into(),
            probability_weight,
            operating_cost,
            facility_energy_kwh,
            completed_work_fraction,
            peak_power_shortfall_kw,
            energy_shortfall_kwh,
            deadline_violated,
            thermal_limit_violated,
            protected_reserve_breached,
            workload_evidence_id: workload_evidence_id.into(),
            power_evidence_id: power_evidence_id.into(),
            thermal_evidence_id: thermal_evidence_id.into(),
            storage_evidence_id: storage_evidence_id.into(),
        };
        for (name, value) in [
            ("scenario ID", &outcome.scenario_id),
            ("scenario digest", &outcome.scenario_digest),
            ("workload evidence ID", &outcome.workload_evidence_id),
            ("power evidence ID", &outcome.power_evidence_id),
            ("thermal evidence ID", &outcome.thermal_evidence_id),
            ("storage evidence ID", &outcome.storage_evidence_id),
        ] {
            require_id(name, value)?;
        }
        if !outcome.probability_weight.is_finite()
            || outcome.probability_weight <= 0.0
            || outcome.probability_weight > 1.0
        {
            return Err(ScenarioRiskError::InvalidProbabilityWeight(
                outcome.probability_weight,
            ));
        }
        if !outcome.operating_cost.is_finite() {
            return Err(ScenarioRiskError::InvalidQuantity {
                name: "operating cost",
                value: outcome.operating_cost,
            });
        }
        for (name, value) in [
            ("facility energy", outcome.facility_energy_kwh),
            ("peak power shortfall", outcome.peak_power_shortfall_kw),
            ("energy shortfall", outcome.energy_shortfall_kwh),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ScenarioRiskError::InvalidQuantity { name, value });
            }
        }
        require_probability(
            "completed work fraction",
            outcome.completed_work_fraction,
            true,
        )?;
        Ok(outcome)
    }

    pub fn scenario_id(&self) -> &str {
        &self.scenario_id
    }

    pub const fn probability_weight(&self) -> f64 {
        self.probability_weight
    }
}

/// Frozen scenario ensemble and candidate outcomes.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioRiskEvaluation {
    contract: ScenarioRiskContract,
    outcomes: Vec<ScenarioRiskOutcome>,
}

impl ScenarioRiskEvaluation {
    pub fn new(
        contract: ScenarioRiskContract,
        outcomes: Vec<ScenarioRiskOutcome>,
    ) -> Result<Self, ScenarioRiskError> {
        if outcomes.is_empty() {
            return Err(ScenarioRiskError::EmptyScenarioSet);
        }
        let mut ids = BTreeSet::new();
        for outcome in &outcomes {
            if !ids.insert(outcome.scenario_id.clone()) {
                return Err(ScenarioRiskError::DuplicateScenarioId(
                    outcome.scenario_id.clone(),
                ));
            }
        }
        Ok(Self { contract, outcomes })
    }

    pub fn contract(&self) -> &ScenarioRiskContract {
        &self.contract
    }

    pub fn outcomes(&self) -> &[ScenarioRiskOutcome] {
        &self.outcomes
    }
}

/// Per-scenario retained result receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioRiskReceipt {
    pub scenario_id: String,
    pub scenario_digest: String,
    pub probability_weight: f64,
    pub operating_cost: f64,
    pub facility_energy_kwh: f64,
    pub completed_work_fraction: f64,
    pub peak_power_shortfall_kw: f64,
    pub energy_shortfall_kwh: f64,
    pub deadline_violated: bool,
    pub thermal_limit_violated: bool,
    pub protected_reserve_breached: bool,
    pub workload_evidence_id: String,
    pub power_evidence_id: String,
    pub thermal_evidence_id: String,
    pub storage_evidence_id: String,
}

/// Independent risk report for a finite frozen joint scenario ensemble.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioRiskReport {
    pub benchmark_id: String,
    pub strategy_id: String,
    pub scenario_set_digest: String,
    pub service_contract_digest: String,
    pub evaluation_configuration_digest: String,
    pub metric_convention_id: String,
    pub scenario_count: usize,
    pub probability_weight_sum: f64,
    pub cvar_confidence_level: f64,
    pub expected_operating_cost: f64,
    pub expected_facility_energy_kwh: f64,
    pub expected_work_completion_fraction: f64,
    pub minimum_work_completion_fraction: f64,
    pub power_shortfall_probability: f64,
    pub energy_shortfall_probability: f64,
    pub deadline_violation_probability: f64,
    pub thermal_violation_probability: f64,
    pub protected_reserve_breach_probability: f64,
    pub peak_power_shortfall_var_kw: f64,
    pub peak_power_shortfall_cvar_kw: f64,
    pub energy_shortfall_var_kwh: f64,
    pub energy_shortfall_cvar_kwh: f64,
    pub probability_mass_tail: f64,
    pub probability_mass_normalized: bool,
    pub power_shortfall_gate_passed: bool,
    pub energy_shortfall_gate_passed: bool,
    pub deadline_gate_passed: bool,
    pub thermal_gate_passed: bool,
    pub reserve_gate_passed: bool,
    pub power_cvar_gate_passed: bool,
    pub energy_cvar_gate_passed: bool,
    pub expected_service_gate_passed: bool,
    pub worst_case_service_gate_passed: bool,
    pub passed: bool,
    pub receipts: Vec<ScenarioRiskReceipt>,
}

/// Malformed probability sets, identity errors or non-finite arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum ScenarioRiskError {
    EmptyIdentifier(&'static str),
    InvalidQuantity { name: &'static str, value: f64 },
    InvalidProbability { name: &'static str, value: f64 },
    InvalidProbabilityWeight(f64),
    InvalidProbabilityTolerance(f64),
    EmptyScenarioSet,
    DuplicateScenarioId(String),
    ProbabilityMassMismatch { observed: f64, tolerance: f64 },
    NumericalOverflow(&'static str),
}

impl fmt::Display for ScenarioRiskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(name) => write!(f, "{name} must not be empty"),
            Self::InvalidQuantity { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::InvalidProbability { name, value } => {
                write!(f, "{name} must be within its valid probability interval; got {value}")
            }
            Self::InvalidProbabilityWeight(value) => {
                write!(f, "scenario probability weight must be in (0, 1]; got {value}")
            }
            Self::InvalidProbabilityTolerance(value) => {
                write!(f, "probability-sum tolerance must be in (0, 1e-3]; got {value}")
            }
            Self::EmptyScenarioSet => write!(f, "scenario risk evaluation requires outcomes"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate scenario ID: {id}"),
            Self::ProbabilityMassMismatch { observed, tolerance } => write!(
                f,
                "scenario probability mass sums to {observed}; required distance from 1 <= {tolerance}"
            ),
            Self::NumericalOverflow(name) => write!(f, "non-finite scenario risk arithmetic: {name}"),
        }
    }
}

impl std::error::Error for ScenarioRiskError {}

/// Reconstruct weighted expectations, violation probabilities and upper-tail risk.
///
/// VaR uses the left-continuous weighted empirical quantile: the smallest loss
/// whose cumulative probability is at least alpha. CVaR is the mean of exactly
/// the worst (1-alpha) probability mass, consuming a fractional weight from
/// the quantile-boundary scenario when necessary. This is empirical finite-set
/// CVaR, not a moment-based or worst-case distributionally robust guarantee.
pub fn evaluate_scenario_risk(
    evaluation: &ScenarioRiskEvaluation,
) -> Result<ScenarioRiskReport, ScenarioRiskError> {
    let contract = &evaluation.contract;
    let outcomes = &evaluation.outcomes;
    let probability_sum = sum_finite(
        outcomes.iter().map(|outcome| outcome.probability_weight),
        "scenario probability sum",
    )?;
    if (probability_sum - 1.0).abs() > contract.probability_sum_tolerance {
        return Err(ScenarioRiskError::ProbabilityMassMismatch {
            observed: probability_sum,
            tolerance: contract.probability_sum_tolerance,
        });
    }

    let expected_operating_cost = weighted_sum(outcomes, |outcome| outcome.operating_cost)?;
    let expected_facility_energy = weighted_sum(outcomes, |outcome| outcome.facility_energy_kwh)?;
    let expected_work_completion = weighted_sum(outcomes, |outcome| outcome.completed_work_fraction)?;
    let minimum_work_completion = outcomes
        .iter()
        .map(|outcome| outcome.completed_work_fraction)
        .fold(1.0_f64, f64::min);

    let power_shortfall_probability =
        weighted_event_probability(outcomes, |outcome| outcome.peak_power_shortfall_kw > 0.0)?;
    let energy_shortfall_probability =
        weighted_event_probability(outcomes, |outcome| outcome.energy_shortfall_kwh > 0.0)?;
    let deadline_violation_probability =
        weighted_event_probability(outcomes, |outcome| outcome.deadline_violated)?;
    let thermal_violation_probability =
        weighted_event_probability(outcomes, |outcome| outcome.thermal_limit_violated)?;
    let reserve_breach_probability =
        weighted_event_probability(outcomes, |outcome| outcome.protected_reserve_breached)?;

    let power_losses: Vec<_> = outcomes
        .iter()
        .map(|outcome| (outcome.peak_power_shortfall_kw, outcome.probability_weight))
        .collect();
    let energy_losses: Vec<_> = outcomes
        .iter()
        .map(|outcome| (outcome.energy_shortfall_kwh, outcome.probability_weight))
        .collect();
    let (power_var, power_cvar) =
        weighted_var_cvar(&power_losses, contract.cvar_confidence_level)?;
    let (energy_var, energy_cvar) =
        weighted_var_cvar(&energy_losses, contract.cvar_confidence_level)?;

    let power_shortfall_gate_passed =
        power_shortfall_probability <= contract.maximum_power_shortfall_probability;
    let energy_shortfall_gate_passed =
        energy_shortfall_probability <= contract.maximum_energy_shortfall_probability;
    let deadline_gate_passed =
        deadline_violation_probability <= contract.maximum_deadline_violation_probability;
    let thermal_gate_passed =
        thermal_violation_probability <= contract.maximum_thermal_violation_probability;
    let reserve_gate_passed =
        reserve_breach_probability <= contract.maximum_reserve_breach_probability;
    let power_cvar_gate_passed = power_cvar <= contract.maximum_peak_power_shortfall_cvar_kw;
    let energy_cvar_gate_passed = energy_cvar <= contract.maximum_energy_shortfall_cvar_kwh;
    let expected_service_gate_passed =
        expected_work_completion >= contract.minimum_expected_work_completion_fraction;
    let worst_case_service_gate_passed =
        minimum_work_completion >= contract.minimum_worst_case_work_completion_fraction;

    let passed = power_shortfall_gate_passed
        && energy_shortfall_gate_passed
        && deadline_gate_passed
        && thermal_gate_passed
        && reserve_gate_passed
        && power_cvar_gate_passed
        && energy_cvar_gate_passed
        && expected_service_gate_passed
        && worst_case_service_gate_passed;

    let receipts = outcomes
        .iter()
        .map(|outcome| ScenarioRiskReceipt {
            scenario_id: outcome.scenario_id.clone(),
            scenario_digest: outcome.scenario_digest.clone(),
            probability_weight: outcome.probability_weight,
            operating_cost: outcome.operating_cost,
            facility_energy_kwh: outcome.facility_energy_kwh,
            completed_work_fraction: outcome.completed_work_fraction,
            peak_power_shortfall_kw: outcome.peak_power_shortfall_kw,
            energy_shortfall_kwh: outcome.energy_shortfall_kwh,
            deadline_violated: outcome.deadline_violated,
            thermal_limit_violated: outcome.thermal_limit_violated,
            protected_reserve_breached: outcome.protected_reserve_breached,
            workload_evidence_id: outcome.workload_evidence_id.clone(),
            power_evidence_id: outcome.power_evidence_id.clone(),
            thermal_evidence_id: outcome.thermal_evidence_id.clone(),
            storage_evidence_id: outcome.storage_evidence_id.clone(),
        })
        .collect();

    Ok(ScenarioRiskReport {
        benchmark_id: contract.benchmark_id.clone(),
        strategy_id: contract.strategy_id.clone(),
        scenario_set_digest: contract.scenario_set_digest.clone(),
        service_contract_digest: contract.service_contract_digest.clone(),
        evaluation_configuration_digest: contract.evaluation_configuration_digest.clone(),
        metric_convention_id: "SYMTROPY-WEIGHTED-DISCRETE-CVAR-V0.1".to_string(),
        scenario_count: outcomes.len(),
        probability_weight_sum: probability_sum,
        cvar_confidence_level: contract.cvar_confidence_level,
        expected_operating_cost,
        expected_facility_energy_kwh: expected_facility_energy,
        expected_work_completion_fraction: expected_work_completion,
        minimum_work_completion_fraction: minimum_work_completion,
        power_shortfall_probability,
        energy_shortfall_probability,
        deadline_violation_probability,
        thermal_violation_probability,
        protected_reserve_breach_probability: reserve_breach_probability,
        peak_power_shortfall_var_kw: power_var,
        peak_power_shortfall_cvar_kw: power_cvar,
        energy_shortfall_var_kwh: energy_var,
        energy_shortfall_cvar_kwh: energy_cvar,
        probability_mass_tail: 1.0 - contract.cvar_confidence_level,
        probability_mass_normalized: true,
        power_shortfall_gate_passed,
        energy_shortfall_gate_passed,
        deadline_gate_passed,
        thermal_gate_passed,
        reserve_gate_passed,
        power_cvar_gate_passed,
        energy_cvar_gate_passed,
        expected_service_gate_passed,
        worst_case_service_gate_passed,
        passed,
        receipts,
    })
}

/// Calculate the weighted alpha quantile and exact upper-tail empirical CVaR.
fn weighted_var_cvar(
    weighted_losses: &[(f64, f64)],
    alpha: f64,
) -> Result<(f64, f64), ScenarioRiskError> {
    if weighted_losses.is_empty() {
        return Err(ScenarioRiskError::EmptyScenarioSet);
    }
    require_probability("CVaR confidence level", alpha, false)?;
    let mut ascending = weighted_losses.to_vec();
    ascending.sort_by(|left, right| left.0.total_cmp(&right.0));
    let mut cumulative = 0.0;
    let mut value_at_risk = None;
    for (loss, weight) in &ascending {
        cumulative += *weight;
        if !cumulative.is_finite() {
            return Err(ScenarioRiskError::NumericalOverflow("weighted quantile"));
        }
        if cumulative + 1e-15 >= alpha {
            value_at_risk = Some(*loss);
            break;
        }
    }
    let value_at_risk = value_at_risk.ok_or(ScenarioRiskError::NumericalOverflow(
        "weighted quantile did not reach confidence level",
    ))?;

    let tail_mass = 1.0 - alpha;
    let mut descending = weighted_losses.to_vec();
    descending.sort_by(|left, right| right.0.total_cmp(&left.0));
    let mut remaining = tail_mass;
    let mut tail_loss_mass = 0.0;
    for (loss, weight) in descending {
        if remaining <= 0.0 {
            break;
        }
        let selected_weight = weight.min(remaining);
        tail_loss_mass += loss * selected_weight;
        remaining -= selected_weight;
    }
    if remaining > 1e-10 || !tail_loss_mass.is_finite() {
        return Err(ScenarioRiskError::NumericalOverflow("weighted CVaR tail"));
    }
    let cvar = tail_loss_mass / tail_mass;
    if !cvar.is_finite() {
        return Err(ScenarioRiskError::NumericalOverflow("weighted CVaR result"));
    }
    Ok((value_at_risk, cvar))
}

fn weighted_sum(
    outcomes: &[ScenarioRiskOutcome],
    value: impl Fn(&ScenarioRiskOutcome) -> f64,
) -> Result<f64, ScenarioRiskError> {
    let sum = outcomes
        .iter()
        .map(|outcome| value(outcome) * outcome.probability_weight)
        .sum::<f64>();
    if !sum.is_finite() {
        return Err(ScenarioRiskError::NumericalOverflow("weighted expectation"));
    }
    Ok(sum)
}

fn weighted_event_probability(
    outcomes: &[ScenarioRiskOutcome],
    predicate: impl Fn(&ScenarioRiskOutcome) -> bool,
) -> Result<f64, ScenarioRiskError> {
    let sum = outcomes
        .iter()
        .filter(|outcome| predicate(outcome))
        .map(|outcome| outcome.probability_weight)
        .sum::<f64>();
    if !sum.is_finite() {
        return Err(ScenarioRiskError::NumericalOverflow("violation probability"));
    }
    Ok(sum)
}

fn sum_finite(
    values: impl Iterator<Item = f64>,
    name: &'static str,
) -> Result<f64, ScenarioRiskError> {
    let sum = values.sum::<f64>();
    if !sum.is_finite() {
        return Err(ScenarioRiskError::NumericalOverflow(name));
    }
    Ok(sum)
}

fn require_id(name: &'static str, value: &str) -> Result<(), ScenarioRiskError> {
    if value.trim().is_empty() {
        return Err(ScenarioRiskError::EmptyIdentifier(name));
    }
    Ok(())
}

fn require_probability(
    name: &'static str,
    value: f64,
    inclusive_one: bool,
) -> Result<(), ScenarioRiskError> {
    let upper_valid = if inclusive_one { value <= 1.0 } else { value < 1.0 };
    if !value.is_finite() || value < 0.0 || !upper_valid {
        return Err(ScenarioRiskError::InvalidProbability { name, value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract(alpha: f64) -> ScenarioRiskContract {
        ScenarioRiskContract::new(
            "benchmark-1",
            "strategy-1",
            "joint-scenario-set-digest",
            "service-contract-digest",
            "evaluation-config-digest",
            alpha,
            0.10, 0.10, 0.05, 0.01, 0.01,
            10.0, 10.0, 0.95, 0.90, 1e-9,
        ).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn outcome(
        id: &str,
        weight: f64,
        power_deficit: f64,
        energy_deficit: f64,
        work: f64,
        deadline: bool,
        thermal: bool,
        reserve: bool,
    ) -> ScenarioRiskOutcome {
        ScenarioRiskOutcome::new(
            id,
            format!("scenario-digest-{id}"),
            weight,
            100.0 + power_deficit,
            500.0 + energy_deficit,
            work,
            power_deficit,
            energy_deficit,
            deadline,
            thermal,
            reserve,
            format!("work-{id}"),
            format!("power-{id}"),
            format!("thermal-{id}"),
            format!("storage-{id}"),
        ).unwrap()
    }

    fn valid_evaluation(alpha: f64) -> ScenarioRiskEvaluation {
        ScenarioRiskEvaluation::new(
            contract(alpha),
            vec![
                outcome("s1", 0.50, 0.0, 0.0, 1.0, false, false, false),
                outcome("s2", 0.30, 0.0, 0.0, 1.0, false, false, false),
                outcome("s3", 0.15, 0.0, 0.0, 0.98, false, false, false),
                outcome("s4", 0.05, 0.5, 1.0, 0.95, false, false, false),
            ],
        ).unwrap()
    }

    #[test]
    fn accepts_normalized_joint_scenario_ensemble() {
        let report = evaluate_scenario_risk(&valid_evaluation(0.95)).unwrap();
        assert!(report.passed);
        assert!(report.probability_mass_normalized);
        assert!((report.expected_work_completion_fraction - 0.9945).abs() < 1e-12);
        assert_eq!(report.scenario_count, 4);
        assert_eq!(report.receipts.len(), 4);
    }

    #[test]
    fn computes_fractional_probability_mass_at_cvar_tail_boundary() {
        let evaluation = ScenarioRiskEvaluation::new(
            contract(0.95),
            vec![
                outcome("common", 0.90, 0.0, 0.0, 1.0, false, false, false),
                outcome("moderate", 0.08, 4.0, 8.0, 0.98, false, false, false),
                outcome("rare-severe", 0.02, 20.0, 40.0, 0.95, false, false, false),
            ],
        ).unwrap();
        let report = evaluate_scenario_risk(&evaluation).unwrap();
        assert!((report.peak_power_shortfall_var_kw - 4.0).abs() < 1e-12);
        assert!((report.peak_power_shortfall_cvar_kw - 10.4).abs() < 1e-12);
        assert!((report.energy_shortfall_cvar_kwh - 20.8).abs() < 1e-12);
    }

    #[test]
    fn rare_high_deficit_increases_tail_risk_more_than_expected_deficit() {
        let evaluation = ScenarioRiskEvaluation::new(
            contract(0.95),
            vec![
                outcome("normal", 0.98, 0.0, 0.0, 1.0, false, false, false),
                outcome("rare", 0.02, 50.0, 100.0, 0.95, false, false, false),
            ],
        ).unwrap();
        let report = evaluate_scenario_risk(&evaluation).unwrap();
        assert!((report.power_shortfall_probability - 0.02).abs() < 1e-12);
        assert!((report.peak_power_shortfall_cvar_kw - 20.0).abs() < 1e-12);
        assert!((report.energy_shortfall_cvar_kwh - 40.0).abs() < 1e-12);
    }

    #[test]
    fn rejects_risk_neutral_low_cost_candidate_on_shortfall_probability() {
        let evaluation = ScenarioRiskEvaluation::new(
            contract(0.95),
            vec![
                outcome("nominal", 0.50, 0.0, 0.0, 1.0, false, false, false),
                outcome("outage", 0.50, 5.0, 10.0, 1.0, false, false, false),
            ],
        ).unwrap();
        let report = evaluate_scenario_risk(&evaluation).unwrap();
        assert!(!report.passed);
        assert!(!report.power_shortfall_gate_passed);
        assert!(!report.energy_shortfall_gate_passed);
    }

    #[test]
    fn thermal_and_reserve_probabilities_are_independent_gates() {
        let evaluation = ScenarioRiskEvaluation::new(
            contract(0.95),
            vec![
                outcome("normal", 0.95, 0.0, 0.0, 1.0, false, false, false),
                outcome("thermal", 0.03, 0.0, 0.0, 1.0, false, true, false),
                outcome("reserve", 0.02, 0.0, 0.0, 1.0, false, false, true),
            ],
        ).unwrap();
        let report = evaluate_scenario_risk(&evaluation).unwrap();
        assert!(!report.passed);
        assert!(!report.thermal_gate_passed);
        assert!(!report.reserve_gate_passed);
        assert!(report.power_shortfall_gate_passed);
    }

    #[test]
    fn rejects_probability_mass_that_does_not_sum_to_one() {
        let evaluation = ScenarioRiskEvaluation::new(
            contract(0.95),
            vec![
                outcome("s1", 0.80, 0.0, 0.0, 1.0, false, false, false),
                outcome("s2", 0.10, 0.0, 0.0, 1.0, false, false, false),
            ],
        ).unwrap();
        assert!(matches!(
            evaluate_scenario_risk(&evaluation),
            Err(ScenarioRiskError::ProbabilityMassMismatch { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_scenario_identity() {
        let result = ScenarioRiskEvaluation::new(
            contract(0.95),
            vec![
                outcome("same", 0.50, 0.0, 0.0, 1.0, false, false, false),
                outcome("same", 0.50, 0.0, 0.0, 1.0, false, false, false),
            ],
        );
        assert!(matches!(result, Err(ScenarioRiskError::DuplicateScenarioId(_))));
    }

    #[test]
    fn rejects_missing_evidence_and_invalid_confidence_level() {
        assert!(ScenarioRiskOutcome::new(
            "s1", "digest", 1.0, 10.0, 20.0, 1.0, 0.0, 0.0,
            false, false, false, "work", "", "thermal", "storage",
        ).is_err());
        assert!(ScenarioRiskContract::new(
            "benchmark", "strategy", "scenario-set", "service", "config",
            1.0, 0.1, 0.1, 0.1, 0.1, 0.1, 1.0, 1.0, 0.9, 0.9, 1e-9,
        ).is_err());
    }

    #[test]
    fn worst_case_work_completion_is_not_hidden_by_expected_average() {
        let evaluation = ScenarioRiskEvaluation::new(
            contract(0.95),
            vec![
                outcome("most", 0.99, 0.0, 0.0, 1.0, false, false, false),
                outcome("bad", 0.01, 0.0, 0.0, 0.50, false, false, false),
            ],
        ).unwrap();
        let report = evaluate_scenario_risk(&evaluation).unwrap();
        assert!(report.expected_service_gate_passed);
        assert!(!report.worst_case_service_gate_passed);
        assert!(!report.passed);
    }

    #[test]
    fn rejects_zero_probability_weight() {
        assert!(ScenarioRiskOutcome::new(
            "s1", "digest", 0.0, 10.0, 20.0, 1.0, 0.0, 0.0,
            false, false, false, "work", "power", "thermal", "storage",
        ).is_err());
    }
}
