// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Paired, service-equivalent strategy comparison for data-centre energy policies.
//!
//! Every candidate is evaluated on the exact same offered workload and frozen
//! joint scenario cases. Ratios are computed from weighted totals rather than
//! averaging per-case efficiency percentages. Hard service, deadline, thermal,
//! reserve and grid constraints remain non-compensable. The qualified frontier
//! is Pareto-based; this module does not create a weighted winner score.
//!
//! This evaluator compares supplied outcome receipts. It does not schedule,
//! forecast, simulate, establish counterfactual causal truth, authenticate raw
//! evidence bytes, or authorize equipment dispatch.

use core::fmt;
use std::collections::BTreeMap;

use serde::Serialize;

/// Frozen service/resource contract and non-compensable policy gates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyBenchmarkContract {
    benchmark_id: String,
    benchmark_configuration_digest: String,
    service_contract_digest: String,
    scenario_set_digest: String,
    operating_cost_currency: String,
    strategy_ids: Vec<String>,
    baseline_strategy_id: String,
    probability_sum_tolerance: f64,
    minimum_expected_on_time_work_fraction: f64,
    minimum_worst_case_on_time_work_fraction: f64,
    maximum_deadline_violation_probability: f64,
    maximum_thermal_violation_probability: f64,
    maximum_reserve_breach_probability: f64,
    maximum_grid_shortfall_probability: f64,
    maximum_p95_latency_ms: f64,
}

impl StrategyBenchmarkContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        benchmark_id: impl Into<String>,
        benchmark_configuration_digest: impl Into<String>,
        service_contract_digest: impl Into<String>,
        scenario_set_digest: impl Into<String>,
        operating_cost_currency: impl Into<String>,
        mut strategy_ids: Vec<String>,
        baseline_strategy_id: impl Into<String>,
        probability_sum_tolerance: f64,
        minimum_expected_on_time_work_fraction: f64,
        minimum_worst_case_on_time_work_fraction: f64,
        maximum_deadline_violation_probability: f64,
        maximum_thermal_violation_probability: f64,
        maximum_reserve_breach_probability: f64,
        maximum_grid_shortfall_probability: f64,
        maximum_p95_latency_ms: f64,
    ) -> Result<Self, StrategyBenchmarkError> {
        let contract = Self {
            benchmark_id: benchmark_id.into(),
            benchmark_configuration_digest: benchmark_configuration_digest.into(),
            service_contract_digest: service_contract_digest.into(),
            scenario_set_digest: scenario_set_digest.into(),
            operating_cost_currency: operating_cost_currency.into(),
            strategy_ids: {
                strategy_ids.sort();
                strategy_ids
            },
            baseline_strategy_id: baseline_strategy_id.into(),
            probability_sum_tolerance,
            minimum_expected_on_time_work_fraction,
            minimum_worst_case_on_time_work_fraction,
            maximum_deadline_violation_probability,
            maximum_thermal_violation_probability,
            maximum_reserve_breach_probability,
            maximum_grid_shortfall_probability,
            maximum_p95_latency_ms,
        };

        for (field, value) in [
            ("benchmark ID", contract.benchmark_id.as_str()),
            (
                "benchmark configuration digest",
                contract.benchmark_configuration_digest.as_str(),
            ),
            ("service contract digest", contract.service_contract_digest.as_str()),
            ("scenario-set digest", contract.scenario_set_digest.as_str()),
            ("operating-cost currency", contract.operating_cost_currency.as_str()),
            ("baseline strategy ID", contract.baseline_strategy_id.as_str()),
        ] {
            require_id(field, value)?;
        }
        if contract.operating_cost_currency.len() != 3
            || !contract
                .operating_cost_currency
                .bytes()
                .all(|byte| byte.is_ascii_uppercase())
        {
            return Err(StrategyBenchmarkError::InvalidCurrencyCode(
                contract.operating_cost_currency.clone(),
            ));
        }

        if contract.strategy_ids.len() < 2 {
            return Err(StrategyBenchmarkError::InsufficientStrategies(
                contract.strategy_ids.len(),
            ));
        }
        for strategy_id in &contract.strategy_ids {
            require_id("strategy ID", strategy_id)?;
        }
        if contract.strategy_ids.windows(2).any(|pair| pair[0] == pair[1]) {
            return Err(StrategyBenchmarkError::DuplicateStrategyId);
        }
        if contract
            .strategy_ids
            .binary_search(&contract.baseline_strategy_id)
            .is_err()
        {
            return Err(StrategyBenchmarkError::UnknownBaselineStrategy(
                contract.baseline_strategy_id.clone(),
            ));
        }

        if !contract.probability_sum_tolerance.is_finite()
            || contract.probability_sum_tolerance <= 0.0
            || contract.probability_sum_tolerance > 1e-3
        {
            return Err(StrategyBenchmarkError::InvalidProbabilityTolerance(
                contract.probability_sum_tolerance,
            ));
        }

        for (field, value) in [
            (
                "minimum expected on-time work fraction",
                contract.minimum_expected_on_time_work_fraction,
            ),
            (
                "minimum worst-case on-time work fraction",
                contract.minimum_worst_case_on_time_work_fraction,
            ),
            (
                "maximum deadline-violation probability",
                contract.maximum_deadline_violation_probability,
            ),
            (
                "maximum thermal-violation probability",
                contract.maximum_thermal_violation_probability,
            ),
            (
                "maximum protected-reserve breach probability",
                contract.maximum_reserve_breach_probability,
            ),
            (
                "maximum grid-shortfall probability",
                contract.maximum_grid_shortfall_probability,
            ),
        ] {
            require_probability(field, value)?;
        }
        if !contract.maximum_p95_latency_ms.is_finite()
            || contract.maximum_p95_latency_ms < 0.0
        {
            return Err(StrategyBenchmarkError::InvalidQuantity {
                field: "maximum P95 latency",
                value: contract.maximum_p95_latency_ms,
            });
        }

        Ok(contract)
    }

    pub fn benchmark_id(&self) -> &str {
        &self.benchmark_id
    }

    pub fn service_contract_digest(&self) -> &str {
        &self.service_contract_digest
    }

    pub fn baseline_strategy_id(&self) -> &str {
        &self.baseline_strategy_id
    }

    pub fn strategy_ids(&self) -> &[String] {
        &self.strategy_ids
    }

    pub fn operating_cost_currency(&self) -> &str {
        &self.operating_cost_currency
    }
}

/// One strategy's measured/simulated outcome under one frozen joint scenario.
///
/// Accepted useful work must pass the caller's pinned correctness/quality
/// oracle. On-time useful work is the subset that also met the service
/// deadline. The evaluator does not invent that oracle.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyBenchmarkOutcome {
    strategy_id: String,
    scenario_id: String,
    scenario_digest: String,
    probability_weight: f64,
    offered_work_units: f64,
    accepted_useful_work_units: f64,
    on_time_useful_work_units: f64,
    facility_energy_kwh: f64,
    it_energy_kwh: f64,
    cooling_energy_kwh: f64,
    grid_import_energy_kwh: f64,
    water_litres: f64,
    carbon_kg_co2e: f64,
    operating_cost: f64,
    p95_latency_ms: f64,
    peak_temperature_c: f64,
    deadline_violated: bool,
    thermal_limit_violated: bool,
    protected_reserve_breached: bool,
    grid_shortfall: bool,
    service_evidence_id: String,
    facility_energy_evidence_id: String,
    it_energy_evidence_id: String,
    cooling_energy_evidence_id: String,
    grid_energy_evidence_id: String,
    water_evidence_id: String,
    carbon_evidence_id: String,
    cost_evidence_id: String,
    thermal_evidence_id: String,
    storage_evidence_id: String,
}

impl StrategyBenchmarkOutcome {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        strategy_id: impl Into<String>,
        scenario_id: impl Into<String>,
        scenario_digest: impl Into<String>,
        probability_weight: f64,
        offered_work_units: f64,
        accepted_useful_work_units: f64,
        on_time_useful_work_units: f64,
        facility_energy_kwh: f64,
        it_energy_kwh: f64,
        cooling_energy_kwh: f64,
        grid_import_energy_kwh: f64,
        water_litres: f64,
        carbon_kg_co2e: f64,
        operating_cost: f64,
        p95_latency_ms: f64,
        peak_temperature_c: f64,
        deadline_violated: bool,
        thermal_limit_violated: bool,
        protected_reserve_breached: bool,
        grid_shortfall: bool,
        service_evidence_id: impl Into<String>,
        facility_energy_evidence_id: impl Into<String>,
        it_energy_evidence_id: impl Into<String>,
        cooling_energy_evidence_id: impl Into<String>,
        grid_energy_evidence_id: impl Into<String>,
        water_evidence_id: impl Into<String>,
        carbon_evidence_id: impl Into<String>,
        cost_evidence_id: impl Into<String>,
        thermal_evidence_id: impl Into<String>,
        storage_evidence_id: impl Into<String>,
    ) -> Result<Self, StrategyBenchmarkError> {
        let outcome = Self {
            strategy_id: strategy_id.into(),
            scenario_id: scenario_id.into(),
            scenario_digest: scenario_digest.into(),
            probability_weight,
            offered_work_units,
            accepted_useful_work_units,
            on_time_useful_work_units,
            facility_energy_kwh,
            it_energy_kwh,
            cooling_energy_kwh,
            grid_import_energy_kwh,
            water_litres,
            carbon_kg_co2e,
            operating_cost,
            p95_latency_ms,
            peak_temperature_c,
            deadline_violated,
            thermal_limit_violated,
            protected_reserve_breached,
            grid_shortfall,
            service_evidence_id: service_evidence_id.into(),
            facility_energy_evidence_id: facility_energy_evidence_id.into(),
            it_energy_evidence_id: it_energy_evidence_id.into(),
            cooling_energy_evidence_id: cooling_energy_evidence_id.into(),
            grid_energy_evidence_id: grid_energy_evidence_id.into(),
            water_evidence_id: water_evidence_id.into(),
            carbon_evidence_id: carbon_evidence_id.into(),
            cost_evidence_id: cost_evidence_id.into(),
            thermal_evidence_id: thermal_evidence_id.into(),
            storage_evidence_id: storage_evidence_id.into(),
        };

        for (field, value) in [
            ("strategy ID", outcome.strategy_id.as_str()),
            ("scenario ID", outcome.scenario_id.as_str()),
            ("scenario digest", outcome.scenario_digest.as_str()),
            ("service evidence ID", outcome.service_evidence_id.as_str()),
            (
                "facility-energy evidence ID",
                outcome.facility_energy_evidence_id.as_str(),
            ),
            ("IT-energy evidence ID", outcome.it_energy_evidence_id.as_str()),
            (
                "cooling-energy evidence ID",
                outcome.cooling_energy_evidence_id.as_str(),
            ),
            ("grid-energy evidence ID", outcome.grid_energy_evidence_id.as_str()),
            ("water evidence ID", outcome.water_evidence_id.as_str()),
            ("carbon evidence ID", outcome.carbon_evidence_id.as_str()),
            ("cost evidence ID", outcome.cost_evidence_id.as_str()),
            ("thermal evidence ID", outcome.thermal_evidence_id.as_str()),
            ("storage evidence ID", outcome.storage_evidence_id.as_str()),
        ] {
            require_id(field, value)?;
        }

        if !outcome.probability_weight.is_finite()
            || outcome.probability_weight <= 0.0
            || outcome.probability_weight > 1.0
        {
            return Err(StrategyBenchmarkError::InvalidProbabilityWeight(
                outcome.probability_weight,
            ));
        }

        for (field, value) in [
            ("offered work units", outcome.offered_work_units),
            ("accepted useful work units", outcome.accepted_useful_work_units),
            ("on-time useful work units", outcome.on_time_useful_work_units),
            ("facility energy", outcome.facility_energy_kwh),
            ("IT energy", outcome.it_energy_kwh),
            ("cooling energy", outcome.cooling_energy_kwh),
            ("grid import energy", outcome.grid_import_energy_kwh),
            ("water use", outcome.water_litres),
            ("carbon emissions", outcome.carbon_kg_co2e),
            ("operating cost", outcome.operating_cost),
            ("P95 latency", outcome.p95_latency_ms),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(StrategyBenchmarkError::InvalidQuantity { field, value });
            }
        }
        if !outcome.peak_temperature_c.is_finite() {
            return Err(StrategyBenchmarkError::InvalidQuantity {
                field: "peak temperature",
                value: outcome.peak_temperature_c,
            });
        }
        if outcome.offered_work_units <= 0.0 {
            return Err(StrategyBenchmarkError::InvalidQuantity {
                field: "offered work units",
                value: outcome.offered_work_units,
            });
        }
        if outcome.accepted_useful_work_units > outcome.offered_work_units
            || outcome.on_time_useful_work_units > outcome.accepted_useful_work_units
        {
            return Err(StrategyBenchmarkError::WorkQuantityOrdering {
                offered: outcome.offered_work_units,
                accepted: outcome.accepted_useful_work_units,
                on_time: outcome.on_time_useful_work_units,
            });
        }

        Ok(outcome)
    }
}

/// Complete matched strategy/scenario matrix.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyBenchmarkEvaluation {
    contract: StrategyBenchmarkContract,
    outcomes: Vec<StrategyBenchmarkOutcome>,
}

impl StrategyBenchmarkEvaluation {
    pub fn new(
        contract: StrategyBenchmarkContract,
        mut outcomes: Vec<StrategyBenchmarkOutcome>,
    ) -> Result<Self, StrategyBenchmarkError> {
        if outcomes.is_empty() {
            return Err(StrategyBenchmarkError::EmptyOutcomeSet);
        }

        outcomes.sort_by(|left, right| {
            left.strategy_id
                .cmp(&right.strategy_id)
                .then_with(|| left.scenario_id.cmp(&right.scenario_id))
        });

        let mut matrix = BTreeMap::new();
        for outcome in &outcomes {
            if contract.strategy_ids.binary_search(&outcome.strategy_id).is_err() {
                return Err(StrategyBenchmarkError::UnknownStrategy(
                    outcome.strategy_id.clone(),
                ));
            }
            let key = (outcome.strategy_id.clone(), outcome.scenario_id.clone());
            if matrix.insert(key, outcome).is_some() {
                return Err(StrategyBenchmarkError::DuplicateStrategyScenarioOutcome {
                    strategy_id: outcome.strategy_id.clone(),
                    scenario_id: outcome.scenario_id.clone(),
                });
            }
        }

        let baseline_outcomes = outcomes
            .iter()
            .filter(|outcome| outcome.strategy_id == contract.baseline_strategy_id)
            .collect::<Vec<_>>();
        if baseline_outcomes.is_empty() {
            return Err(StrategyBenchmarkError::MissingBaselineOutcomes);
        }

        let mut raw_weight_sum = 0.0_f64;
        let mut baseline_by_scenario = BTreeMap::new();
        for outcome in &baseline_outcomes {
            raw_weight_sum += outcome.probability_weight;
            baseline_by_scenario.insert(outcome.scenario_id.as_str(), *outcome);
        }
        if !raw_weight_sum.is_finite() || raw_weight_sum <= 0.0 {
            return Err(StrategyBenchmarkError::NumericalOverflow(
                "baseline scenario probability mass",
            ));
        }
        if (raw_weight_sum - 1.0).abs() > contract.probability_sum_tolerance {
            return Err(StrategyBenchmarkError::ProbabilityMassMismatch {
                strategy_id: contract.baseline_strategy_id.clone(),
                observed: raw_weight_sum,
                tolerance: contract.probability_sum_tolerance,
            });
        }

        let baseline_scenario_count = baseline_outcomes.len();
        let expected_matrix_size = contract
            .strategy_ids
            .len()
            .checked_mul(baseline_scenario_count)
            .ok_or(StrategyBenchmarkError::NumericalOverflow(
                "matched strategy/scenario matrix size",
            ))?;
        if outcomes.len() != expected_matrix_size {
            return Err(StrategyBenchmarkError::IncompleteStrategyScenarioMatrix {
                observed: outcomes.len(),
                expected: expected_matrix_size,
            });
        }

        for strategy_id in &contract.strategy_ids {
            let strategy_outcomes = outcomes
                .iter()
                .filter(|outcome| &outcome.strategy_id == strategy_id)
                .collect::<Vec<_>>();
            if strategy_outcomes.len() != baseline_scenario_count {
                return Err(StrategyBenchmarkError::IncompleteStrategyScenarioMatrix {
                    observed: strategy_outcomes.len(),
                    expected: baseline_scenario_count,
                });
            }

            let mut strategy_weight_sum = 0.0_f64;
            for outcome in strategy_outcomes {
                strategy_weight_sum += outcome.probability_weight;
                let baseline = baseline_by_scenario
                    .get(outcome.scenario_id.as_str())
                    .ok_or_else(|| StrategyBenchmarkError::MismatchedScenarioSet {
                        strategy_id: strategy_id.clone(),
                        scenario_id: outcome.scenario_id.clone(),
                    })?;
                if outcome.scenario_digest != baseline.scenario_digest {
                    return Err(StrategyBenchmarkError::ScenarioDigestMismatch {
                        strategy_id: strategy_id.clone(),
                        scenario_id: outcome.scenario_id.clone(),
                    });
                }
                if (outcome.probability_weight - baseline.probability_weight).abs()
                    > contract.probability_sum_tolerance
                {
                    return Err(StrategyBenchmarkError::ScenarioWeightMismatch {
                        strategy_id: strategy_id.clone(),
                        scenario_id: outcome.scenario_id.clone(),
                    });
                }
                if outcome.offered_work_units != baseline.offered_work_units {
                    return Err(StrategyBenchmarkError::OfferedWorkMismatch {
                        strategy_id: strategy_id.clone(),
                        scenario_id: outcome.scenario_id.clone(),
                    });
                }
            }
            if !strategy_weight_sum.is_finite()
                || (strategy_weight_sum - raw_weight_sum).abs()
                    > contract.probability_sum_tolerance
            {
                return Err(StrategyBenchmarkError::StrategyProbabilityMassMismatch {
                    strategy_id: strategy_id.clone(),
                    observed: strategy_weight_sum,
                    baseline_observed: raw_weight_sum,
                    tolerance: contract.probability_sum_tolerance,
                });
            }
        }

        Ok(Self { contract, outcomes })
    }

    pub fn contract(&self) -> &StrategyBenchmarkContract {
        &self.contract
    }

    pub fn outcomes(&self) -> &[StrategyBenchmarkOutcome] {
        &self.outcomes
    }
}

/// Per-scenario evidence receipt after probability normalization.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyOutcomeReceipt {
    pub strategy_id: String,
    pub scenario_id: String,
    pub scenario_digest: String,
    pub raw_probability_weight: f64,
    pub normalized_probability_weight: f64,
    pub offered_work_units: f64,
    pub accepted_useful_work_units: f64,
    pub on_time_useful_work_units: f64,
    pub facility_energy_kwh: f64,
    pub it_energy_kwh: f64,
    pub cooling_energy_kwh: f64,
    pub grid_import_energy_kwh: f64,
    pub water_litres: f64,
    pub carbon_kg_co2e: f64,
    pub operating_cost: f64,
    pub p95_latency_ms: f64,
    pub peak_temperature_c: f64,
    pub deadline_violated: bool,
    pub thermal_limit_violated: bool,
    pub protected_reserve_breached: bool,
    pub grid_shortfall: bool,
    pub service_evidence_id: String,
    pub facility_energy_evidence_id: String,
    pub it_energy_evidence_id: String,
    pub cooling_energy_evidence_id: String,
    pub grid_energy_evidence_id: String,
    pub water_evidence_id: String,
    pub carbon_evidence_id: String,
    pub cost_evidence_id: String,
    pub thermal_evidence_id: String,
    pub storage_evidence_id: String,
}

/// Aggregate, ratio-of-totals service/resource metrics for one strategy.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyResourceSummary {
    pub strategy_id: String,
    pub is_baseline: bool,
    pub scenario_count: usize,
    pub expected_offered_work_units: f64,
    pub expected_accepted_useful_work_units: f64,
    pub expected_on_time_useful_work_units: f64,
    pub expected_on_time_useful_work_fraction: f64,
    pub worst_case_on_time_useful_work_fraction: f64,
    pub expected_facility_energy_kwh: f64,
    pub expected_it_energy_kwh: f64,
    /// Ratio of weighted facility and IT energy totals; not automatically ISO PUE.
    pub facility_to_it_energy_ratio: Option<f64>,
    pub expected_cooling_energy_kwh: f64,
    pub expected_grid_import_energy_kwh: f64,
    pub expected_water_litres: f64,
    pub expected_carbon_kg_co2e: f64,
    pub expected_operating_cost: f64,
    pub expected_scenario_p95_latency_ms: f64,
    pub maximum_scenario_p95_latency_ms: f64,
    pub maximum_peak_temperature_c: f64,
    pub facility_kwh_per_accepted_useful_work_unit: Option<f64>,
    pub facility_kwh_per_on_time_useful_work_unit: Option<f64>,
    pub grid_kwh_per_on_time_useful_work_unit: Option<f64>,
    pub operating_cost_per_on_time_useful_work_unit: Option<f64>,
    pub water_litres_per_on_time_useful_work_unit: Option<f64>,
    pub carbon_kg_per_on_time_useful_work_unit: Option<f64>,
    pub deadline_violation_probability: f64,
    pub thermal_violation_probability: f64,
    pub protected_reserve_breach_probability: f64,
    pub grid_shortfall_probability: f64,
    pub service_gate_passed: bool,
    pub deadline_gate_passed: bool,
    pub latency_gate_passed: bool,
    pub thermal_gate_passed: bool,
    pub reserve_gate_passed: bool,
    pub grid_gate_passed: bool,
    pub passed: bool,
    pub service_equivalent_energy_savings_vs_baseline_fraction: Option<f64>,
    pub pareto_qualified: bool,
    pub pareto_dominated_by: Vec<String>,
}

/// Paired per-scenario counterfactual delta against the pinned baseline.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyPairedDeltaReceipt {
    pub strategy_id: String,
    pub baseline_strategy_id: String,
    pub scenario_id: String,
    pub delta_facility_energy_kwh: f64,
    pub delta_it_energy_kwh: f64,
    pub delta_cooling_energy_kwh: f64,
    pub delta_grid_import_energy_kwh: f64,
    pub delta_water_litres: f64,
    pub delta_carbon_kg_co2e: f64,
    pub delta_operating_cost: f64,
    pub delta_accepted_useful_work_units: f64,
    pub delta_on_time_useful_work_units: f64,
    pub normalized_probability_weight: f64,
}

/// Aggregate paired comparison against the baseline.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyPairedComparisonSummary {
    pub strategy_id: String,
    pub baseline_strategy_id: String,
    pub expected_delta_facility_energy_kwh: f64,
    pub expected_delta_it_energy_kwh: f64,
    pub expected_delta_cooling_energy_kwh: f64,
    pub expected_delta_grid_import_energy_kwh: f64,
    pub expected_delta_water_litres: f64,
    pub expected_delta_carbon_kg_co2e: f64,
    pub expected_delta_operating_cost: f64,
    pub expected_delta_accepted_useful_work_units: f64,
    pub expected_delta_on_time_useful_work_units: f64,
    pub energy_intensity_savings_fraction: Option<f64>,
    pub scenario_deltas: Vec<StrategyPairedDeltaReceipt>,
}

/// Complete paired strategy benchmark report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct StrategyBenchmarkReport {
    pub benchmark_id: String,
    pub benchmark_configuration_digest: String,
    pub service_contract_digest: String,
    pub scenario_set_digest: String,
    pub operating_cost_currency: String,
    pub baseline_strategy_id: String,
    pub metric_convention_id: String,
    pub scenario_count: usize,
    pub probability_weight_sum: f64,
    pub probability_mass_normalized: bool,
    pub strategy_summaries: Vec<StrategyResourceSummary>,
    pub paired_comparisons: Vec<StrategyPairedComparisonSummary>,
    pub outcome_receipts: Vec<StrategyOutcomeReceipt>,
}

/// Invalid matched strategy results or arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum StrategyBenchmarkError {
    EmptyIdentifier(&'static str),
    InvalidQuantity {
        field: &'static str,
        value: f64,
    },
    InvalidProbability {
        field: &'static str,
        value: f64,
    },
    InvalidCurrencyCode(String),
    InvalidProbabilityWeight(f64),
    InvalidProbabilityTolerance(f64),
    InsufficientStrategies(usize),
    DuplicateStrategyId,
    UnknownBaselineStrategy(String),
    WorkQuantityOrdering {
        offered: f64,
        accepted: f64,
        on_time: f64,
    },
    EmptyOutcomeSet,
    UnknownStrategy(String),
    DuplicateStrategyScenarioOutcome {
        strategy_id: String,
        scenario_id: String,
    },
    MissingBaselineOutcomes,
    IncompleteStrategyScenarioMatrix {
        observed: usize,
        expected: usize,
    },
    MismatchedScenarioSet {
        strategy_id: String,
        scenario_id: String,
    },
    ScenarioDigestMismatch {
        strategy_id: String,
        scenario_id: String,
    },
    ScenarioWeightMismatch {
        strategy_id: String,
        scenario_id: String,
    },
    OfferedWorkMismatch {
        strategy_id: String,
        scenario_id: String,
    },
    ProbabilityMassMismatch {
        strategy_id: String,
        observed: f64,
        tolerance: f64,
    },
    StrategyProbabilityMassMismatch {
        strategy_id: String,
        observed: f64,
        baseline_observed: f64,
        tolerance: f64,
    },
    ZeroBaselineOnTimeWork,
    ZeroBaselineEnergyIntensity,
    NumericalOverflow(&'static str),
}

impl fmt::Display for StrategyBenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(field) => write!(f, "{field} must not be empty"),
            Self::InvalidQuantity { field, value } => {
                write!(f, "{field} must be finite and non-negative; got {value}")
            }
            Self::InvalidProbability { field, value } => {
                write!(f, "{field} must be in [0, 1]; got {value}")
            }
            Self::InvalidCurrencyCode(value) => {
                write!(f, "operating-cost currency must be a three-letter uppercase code; got {value}")
            }
            Self::InvalidProbabilityWeight(value) => {
                write!(f, "scenario probability weight must be in (0, 1]; got {value}")
            }
            Self::InvalidProbabilityTolerance(value) => {
                write!(f, "scenario probability tolerance is invalid; got {value}")
            }
            Self::InsufficientStrategies(count) => {
                write!(f, "at least two strategies are required; got {count}")
            }
            Self::DuplicateStrategyId => write!(f, "strategy IDs must be unique"),
            Self::UnknownBaselineStrategy(id) => {
                write!(f, "baseline strategy {id} is not declared")
            }
            Self::WorkQuantityOrdering {
                offered,
                accepted,
                on_time,
            } => write!(
                f,
                "work units must satisfy 0 <= on-time ({on_time}) <= accepted ({accepted}) <= offered ({offered})"
            ),
            Self::EmptyOutcomeSet => write!(f, "strategy outcome set is empty"),
            Self::UnknownStrategy(id) => write!(f, "unknown strategy {id}"),
            Self::DuplicateStrategyScenarioOutcome {
                strategy_id,
                scenario_id,
            } => write!(
                f,
                "strategy {strategy_id} has duplicate outcome for scenario {scenario_id}"
            ),
            Self::MissingBaselineOutcomes => write!(f, "baseline strategy has no outcomes"),
            Self::IncompleteStrategyScenarioMatrix { observed, expected } => write!(
                f,
                "strategy/scenario matrix has {observed} entries; expected {expected}"
            ),
            Self::MismatchedScenarioSet {
                strategy_id,
                scenario_id,
            } => write!(
                f,
                "strategy {strategy_id} is missing matched scenario {scenario_id}"
            ),
            Self::ScenarioDigestMismatch {
                strategy_id,
                scenario_id,
            } => write!(
                f,
                "strategy {strategy_id} has a mismatched scenario digest for {scenario_id}"
            ),
            Self::ScenarioWeightMismatch {
                strategy_id,
                scenario_id,
            } => write!(
                f,
                "strategy {strategy_id} has a mismatched probability weight for {scenario_id}"
            ),
            Self::OfferedWorkMismatch {
                strategy_id,
                scenario_id,
            } => write!(
                f,
                "strategy {strategy_id} has mismatched offered work for {scenario_id}"
            ),
            Self::ProbabilityMassMismatch {
                strategy_id,
                observed,
                tolerance,
            } => write!(
                f,
                "strategy {strategy_id} scenario weights sum to {observed}; tolerance is {tolerance}"
            ),
            Self::StrategyProbabilityMassMismatch {
                strategy_id,
                observed,
                baseline_observed,
                tolerance,
            } => write!(
                f,
                "strategy {strategy_id} probability sum {observed} differs from baseline {baseline_observed}; tolerance is {tolerance}"
            ),
            Self::ZeroBaselineOnTimeWork => {
                write!(f, "baseline has no on-time useful work; energy comparison is undefined")
            }
            Self::ZeroBaselineEnergyIntensity => {
                write!(f, "baseline facility energy intensity is zero; relative savings are undefined")
            }
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite or overflowing arithmetic during {operation}")
            }
        }
    }
}

impl std::error::Error for StrategyBenchmarkError {}

/// Evaluate matched strategies on the same normalized joint scenario distribution.
pub fn evaluate_strategy_benchmark(
    evaluation: &StrategyBenchmarkEvaluation,
) -> Result<StrategyBenchmarkReport, StrategyBenchmarkError> {
    let contract = evaluation.contract();
    let scenario_ids = evaluation
        .outcomes
        .iter()
        .filter(|outcome| outcome.strategy_id == contract.baseline_strategy_id)
        .map(|outcome| outcome.scenario_id.clone())
        .collect::<Vec<_>>();

    let mut baseline_weights = BTreeMap::new();
    let baseline_outcomes = evaluation
        .outcomes
        .iter()
        .filter(|outcome| outcome.strategy_id == contract.baseline_strategy_id)
        .collect::<Vec<_>>();
    let probability_weight_sum = baseline_outcomes
        .iter()
        .map(|outcome| outcome.probability_weight)
        .sum::<f64>();
    if !probability_weight_sum.is_finite() || probability_weight_sum <= 0.0 {
        return Err(StrategyBenchmarkError::NumericalOverflow(
            "baseline probability normalization",
        ));
    }
    for outcome in &baseline_outcomes {
        baseline_weights.insert(
            outcome.scenario_id.clone(),
            outcome.probability_weight / probability_weight_sum,
        );
    }

    let mut normalized_weights = Vec::with_capacity(scenario_ids.len());
    for scenario_id in &scenario_ids {
        normalized_weights.push(
            *baseline_weights
                .get(scenario_id)
                .ok_or_else(|| StrategyBenchmarkError::MismatchedScenarioSet {
                    strategy_id: contract.baseline_strategy_id.clone(),
                    scenario_id: scenario_id.clone(),
                })?,
        );
    }

    let strategy_count = contract.strategy_ids.len();
    let mut receipts = Vec::with_capacity(evaluation.outcomes.len());
    let mut summaries = Vec::with_capacity(strategy_count);
    let mut summaries_by_strategy = BTreeMap::new();

    for strategy_id in &contract.strategy_ids {
        let strategy_outcomes = evaluation
            .outcomes
            .iter()
            .filter(|outcome| &outcome.strategy_id == strategy_id)
            .collect::<Vec<_>>();
        let mut sums = StrategyAccumulator::default();
        let mut worst_case_on_time_fraction = 1.0_f64;
        let mut maximum_p95_latency_ms = 0.0_f64;
        let mut maximum_peak_temperature_c = f64::NEG_INFINITY;

        for (outcome, normalized_weight) in
            strategy_outcomes.iter().zip(normalized_weights.iter())
        {
            sums.add(outcome, *normalized_weight)?;
            let on_time_fraction =
                outcome.on_time_useful_work_units / outcome.offered_work_units;
            worst_case_on_time_fraction =
                worst_case_on_time_fraction.min(on_time_fraction);
            maximum_p95_latency_ms = maximum_p95_latency_ms.max(outcome.p95_latency_ms);
            maximum_peak_temperature_c =
                maximum_peak_temperature_c.max(outcome.peak_temperature_c);

            receipts.push(StrategyOutcomeReceipt::from_outcome(
                outcome,
                *normalized_weight,
            ));
        }

        let summary = sums.finish(
            strategy_id,
            strategy_outcomes.len(),
            worst_case_on_time_fraction,
            maximum_p95_latency_ms,
            maximum_peak_temperature_c,
            contract,
        )?;
        summaries_by_strategy.insert(strategy_id.clone(), summary.clone());
        summaries.push(summary);
    }

    let baseline = summaries_by_strategy
        .get(&contract.baseline_strategy_id)
        .ok_or_else(|| StrategyBenchmarkError::UnknownBaselineStrategy(
            contract.baseline_strategy_id.clone(),
        ))?;
    let baseline_energy_intensity = baseline
        .facility_kwh_per_on_time_useful_work_unit
        .ok_or(StrategyBenchmarkError::ZeroBaselineOnTimeWork)?;
    if baseline_energy_intensity <= 0.0 || !baseline_energy_intensity.is_finite() {
        return Err(StrategyBenchmarkError::ZeroBaselineEnergyIntensity);
    }

    for summary in &mut summaries {
        summary.service_equivalent_energy_savings_vs_baseline_fraction = summary
            .facility_kwh_per_on_time_useful_work_unit
            .map(|intensity| 1.0 - intensity / baseline_energy_intensity);
    }

    let mut paired_comparisons = Vec::with_capacity(strategy_count.saturating_sub(1));
    for strategy_id in &contract.strategy_ids {
        if strategy_id == &contract.baseline_strategy_id {
            continue;
        }
        let mut comparison = StrategyPairedAccumulator::default();
        let mut scenario_deltas = Vec::with_capacity(scenario_ids.len());
        for (scenario_id, normalized_weight) in
            scenario_ids.iter().zip(normalized_weights.iter())
        {
            let candidate = find_outcome(evaluation, strategy_id, scenario_id)?;
            let baseline_outcome =
                find_outcome(evaluation, &contract.baseline_strategy_id, scenario_id)?;
            let delta = StrategyPairedDeltaReceipt {
                strategy_id: strategy_id.clone(),
                baseline_strategy_id: contract.baseline_strategy_id.clone(),
                scenario_id: scenario_id.clone(),
                delta_facility_energy_kwh: candidate.facility_energy_kwh
                    - baseline_outcome.facility_energy_kwh,
                delta_it_energy_kwh: candidate.it_energy_kwh - baseline_outcome.it_energy_kwh,
                delta_cooling_energy_kwh: candidate.cooling_energy_kwh
                    - baseline_outcome.cooling_energy_kwh,
                delta_grid_import_energy_kwh: candidate.grid_import_energy_kwh
                    - baseline_outcome.grid_import_energy_kwh,
                delta_water_litres: candidate.water_litres - baseline_outcome.water_litres,
                delta_carbon_kg_co2e: candidate.carbon_kg_co2e - baseline_outcome.carbon_kg_co2e,
                delta_operating_cost: candidate.operating_cost - baseline_outcome.operating_cost,
                delta_accepted_useful_work_units: candidate.accepted_useful_work_units
                    - baseline_outcome.accepted_useful_work_units,
                delta_on_time_useful_work_units: candidate.on_time_useful_work_units
                    - baseline_outcome.on_time_useful_work_units,
                normalized_probability_weight: *normalized_weight,
            };
            comparison.add(&delta, *normalized_weight)?;
            scenario_deltas.push(delta);
        }
        let expected = comparison.finish();
        let candidate_summary = summaries_by_strategy
            .get(strategy_id)
            .ok_or_else(|| StrategyBenchmarkError::UnknownStrategy(strategy_id.clone()))?;
        paired_comparisons.push(StrategyPairedComparisonSummary {
            strategy_id: strategy_id.clone(),
            baseline_strategy_id: contract.baseline_strategy_id.clone(),
            expected_delta_facility_energy_kwh: expected.facility,
            expected_delta_it_energy_kwh: expected.it,
            expected_delta_cooling_energy_kwh: expected.cooling,
            expected_delta_grid_import_energy_kwh: expected.grid,
            expected_delta_water_litres: expected.water,
            expected_delta_carbon_kg_co2e: expected.carbon,
            expected_delta_operating_cost: expected.cost,
            expected_delta_accepted_useful_work_units: expected.accepted_work,
            expected_delta_on_time_useful_work_units: expected.on_time_work,
            energy_intensity_savings_fraction: candidate_summary
                .service_equivalent_energy_savings_vs_baseline_fraction,
            scenario_deltas,
        });
    }

    apply_qualified_pareto_frontier(&mut summaries);

    Ok(StrategyBenchmarkReport {
        benchmark_id: contract.benchmark_id.clone(),
        benchmark_configuration_digest: contract.benchmark_configuration_digest.clone(),
        service_contract_digest: contract.service_contract_digest.clone(),
        scenario_set_digest: contract.scenario_set_digest.clone(),
        operating_cost_currency: contract.operating_cost_currency.clone(),
        baseline_strategy_id: contract.baseline_strategy_id.clone(),
        metric_convention_id: "SYMTROPY-PAIRED-SERVICE-EQUIVALENT-RESOURCE-V0.1".to_string(),
        scenario_count: scenario_ids.len(),
        probability_weight_sum,
        probability_mass_normalized: true,
        strategy_summaries: summaries,
        paired_comparisons,
        outcome_receipts: receipts,
    })
}

#[derive(Default)]
struct StrategyAccumulator {
    offered_work: f64,
    accepted_work: f64,
    on_time_work: f64,
    facility_energy: f64,
    it_energy: f64,
    cooling_energy: f64,
    grid_energy: f64,
    water: f64,
    carbon: f64,
    cost: f64,
    scenario_p95_latency: f64,
    deadline_probability: f64,
    thermal_probability: f64,
    reserve_probability: f64,
    grid_probability: f64,
}

impl StrategyAccumulator {
    fn add(
        &mut self,
        outcome: &StrategyBenchmarkOutcome,
        weight: f64,
    ) -> Result<(), StrategyBenchmarkError> {
        self.offered_work += outcome.offered_work_units * weight;
        self.accepted_work += outcome.accepted_useful_work_units * weight;
        self.on_time_work += outcome.on_time_useful_work_units * weight;
        self.facility_energy += outcome.facility_energy_kwh * weight;
        self.it_energy += outcome.it_energy_kwh * weight;
        self.cooling_energy += outcome.cooling_energy_kwh * weight;
        self.grid_energy += outcome.grid_import_energy_kwh * weight;
        self.water += outcome.water_litres * weight;
        self.carbon += outcome.carbon_kg_co2e * weight;
        self.cost += outcome.operating_cost * weight;
        self.scenario_p95_latency += outcome.p95_latency_ms * weight;
        self.deadline_probability += if outcome.deadline_violated { weight } else { 0.0 };
        self.thermal_probability += if outcome.thermal_limit_violated { weight } else { 0.0 };
        self.reserve_probability += if outcome.protected_reserve_breached { weight } else { 0.0 };
        self.grid_probability += if outcome.grid_shortfall { weight } else { 0.0 };

        let values = [
            self.offered_work,
            self.accepted_work,
            self.on_time_work,
            self.facility_energy,
            self.it_energy,
            self.cooling_energy,
            self.grid_energy,
            self.water,
            self.carbon,
            self.cost,
            self.scenario_p95_latency,
            self.deadline_probability,
            self.thermal_probability,
            self.reserve_probability,
            self.grid_probability,
        ];
        if values.iter().any(|value| !value.is_finite()) {
            return Err(StrategyBenchmarkError::NumericalOverflow(
                "strategy weighted totals",
            ));
        }
        Ok(())
    }

    #[allow(clippy::too_many_arguments)]
    fn finish(
        &self,
        strategy_id: &str,
        scenario_count: usize,
        worst_case_on_time_fraction: f64,
        maximum_p95_latency_ms: f64,
        maximum_peak_temperature_c: f64,
        contract: &StrategyBenchmarkContract,
    ) -> Result<StrategyResourceSummary, StrategyBenchmarkError> {
        if self.offered_work <= 0.0 {
            return Err(StrategyBenchmarkError::NumericalOverflow(
                "expected offered workload denominator",
            ));
        }
        let on_time_fraction = self.on_time_work / self.offered_work;
        let accepted_energy_intensity = if self.accepted_work > 0.0 {
            Some(self.facility_energy / self.accepted_work)
        } else {
            None
        };
        let on_time_energy_intensity = if self.on_time_work > 0.0 {
            Some(self.facility_energy / self.on_time_work)
        } else {
            None
        };
        let grid_energy_intensity = if self.on_time_work > 0.0 {
            Some(self.grid_energy / self.on_time_work)
        } else {
            None
        };
        let facility_to_it_energy_ratio = if self.it_energy > 0.0 {
            Some(self.facility_energy / self.it_energy)
        } else {
            None
        };
        let operating_cost_intensity = if self.on_time_work > 0.0 {
            Some(self.cost / self.on_time_work)
        } else {
            None
        };
        let water_intensity = if self.on_time_work > 0.0 {
            Some(self.water / self.on_time_work)
        } else {
            None
        };
        let carbon_intensity = if self.on_time_work > 0.0 {
            Some(self.carbon / self.on_time_work)
        } else {
            None
        };

        let service_gate_passed = on_time_fraction
            >= contract.minimum_expected_on_time_work_fraction
            && worst_case_on_time_fraction
                >= contract.minimum_worst_case_on_time_work_fraction;
        let deadline_gate_passed =
            self.deadline_probability <= contract.maximum_deadline_violation_probability;
        let latency_gate_passed =
            maximum_p95_latency_ms <= contract.maximum_p95_latency_ms;
        let thermal_gate_passed =
            self.thermal_probability <= contract.maximum_thermal_violation_probability;
        let reserve_gate_passed =
            self.reserve_probability <= contract.maximum_reserve_breach_probability;
        let grid_gate_passed =
            self.grid_probability <= contract.maximum_grid_shortfall_probability;
        let passed = service_gate_passed
            && deadline_gate_passed
            && latency_gate_passed
            && thermal_gate_passed
            && reserve_gate_passed
            && grid_gate_passed;

        Ok(StrategyResourceSummary {
            strategy_id: strategy_id.to_string(),
            is_baseline: strategy_id == contract.baseline_strategy_id,
            scenario_count,
            expected_offered_work_units: self.offered_work,
            expected_accepted_useful_work_units: self.accepted_work,
            expected_on_time_useful_work_units: self.on_time_work,
            expected_on_time_useful_work_fraction: on_time_fraction,
            worst_case_on_time_useful_work_fraction: worst_case_on_time_fraction,
            expected_facility_energy_kwh: self.facility_energy,
            expected_it_energy_kwh: self.it_energy,
            facility_to_it_energy_ratio,
            expected_cooling_energy_kwh: self.cooling_energy,
            expected_grid_import_energy_kwh: self.grid_energy,
            expected_water_litres: self.water,
            expected_carbon_kg_co2e: self.carbon,
            expected_operating_cost: self.cost,
            expected_scenario_p95_latency_ms: self.scenario_p95_latency,
            maximum_scenario_p95_latency_ms: maximum_p95_latency_ms,
            maximum_peak_temperature_c,
            facility_kwh_per_accepted_useful_work_unit: accepted_energy_intensity,
            facility_kwh_per_on_time_useful_work_unit: on_time_energy_intensity,
            grid_kwh_per_on_time_useful_work_unit: grid_energy_intensity,
            operating_cost_per_on_time_useful_work_unit: operating_cost_intensity,
            water_litres_per_on_time_useful_work_unit: water_intensity,
            carbon_kg_per_on_time_useful_work_unit: carbon_intensity,
            deadline_violation_probability: self.deadline_probability,
            thermal_violation_probability: self.thermal_probability,
            protected_reserve_breach_probability: self.reserve_probability,
            grid_shortfall_probability: self.grid_probability,
            service_gate_passed,
            deadline_gate_passed,
            latency_gate_passed,
            thermal_gate_passed,
            reserve_gate_passed,
            grid_gate_passed,
            passed,
            service_equivalent_energy_savings_vs_baseline_fraction: None,
            pareto_qualified: false,
            pareto_dominated_by: Vec::new(),
        })
    }
}

#[derive(Default)]
struct StrategyPairedAccumulator {
    facility: f64,
    it: f64,
    cooling: f64,
    grid: f64,
    water: f64,
    carbon: f64,
    cost: f64,
    accepted_work: f64,
    on_time_work: f64,
}

impl StrategyPairedAccumulator {
    fn add(
        &mut self,
        delta: &StrategyPairedDeltaReceipt,
        weight: f64,
    ) -> Result<(), StrategyBenchmarkError> {
        self.facility += delta.delta_facility_energy_kwh * weight;
        self.it += delta.delta_it_energy_kwh * weight;
        self.cooling += delta.delta_cooling_energy_kwh * weight;
        self.grid += delta.delta_grid_import_energy_kwh * weight;
        self.water += delta.delta_water_litres * weight;
        self.carbon += delta.delta_carbon_kg_co2e * weight;
        self.cost += delta.delta_operating_cost * weight;
        self.accepted_work += delta.delta_accepted_useful_work_units * weight;
        self.on_time_work += delta.delta_on_time_useful_work_units * weight;
        if [
            self.facility,
            self.it,
            self.cooling,
            self.grid,
            self.water,
            self.carbon,
            self.cost,
            self.accepted_work,
            self.on_time_work,
        ]
        .iter()
        .any(|value| !value.is_finite())
        {
            return Err(StrategyBenchmarkError::NumericalOverflow(
                "paired strategy differences",
            ));
        }
        Ok(())
    }

    fn finish(&self) -> Self {
        Self {
            facility: self.facility,
            it: self.it,
            cooling: self.cooling,
            grid: self.grid,
            water: self.water,
            carbon: self.carbon,
            cost: self.cost,
            accepted_work: self.accepted_work,
            on_time_work: self.on_time_work,
        }
    }
}

impl StrategyOutcomeReceipt {
    fn from_outcome(
        value: &StrategyBenchmarkOutcome,
        normalized_probability_weight: f64,
    ) -> Self {
        Self {
            strategy_id: value.strategy_id.clone(),
            scenario_id: value.scenario_id.clone(),
            scenario_digest: value.scenario_digest.clone(),
            raw_probability_weight: value.probability_weight,
            normalized_probability_weight,
            offered_work_units: value.offered_work_units,
            accepted_useful_work_units: value.accepted_useful_work_units,
            on_time_useful_work_units: value.on_time_useful_work_units,
            facility_energy_kwh: value.facility_energy_kwh,
            it_energy_kwh: value.it_energy_kwh,
            cooling_energy_kwh: value.cooling_energy_kwh,
            grid_import_energy_kwh: value.grid_import_energy_kwh,
            water_litres: value.water_litres,
            carbon_kg_co2e: value.carbon_kg_co2e,
            operating_cost: value.operating_cost,
            p95_latency_ms: value.p95_latency_ms,
            peak_temperature_c: value.peak_temperature_c,
            deadline_violated: value.deadline_violated,
            thermal_limit_violated: value.thermal_limit_violated,
            protected_reserve_breached: value.protected_reserve_breached,
            grid_shortfall: value.grid_shortfall,
            service_evidence_id: value.service_evidence_id.clone(),
            facility_energy_evidence_id: value.facility_energy_evidence_id.clone(),
            it_energy_evidence_id: value.it_energy_evidence_id.clone(),
            cooling_energy_evidence_id: value.cooling_energy_evidence_id.clone(),
            grid_energy_evidence_id: value.grid_energy_evidence_id.clone(),
            water_evidence_id: value.water_evidence_id.clone(),
            carbon_evidence_id: value.carbon_evidence_id.clone(),
            cost_evidence_id: value.cost_evidence_id.clone(),
            thermal_evidence_id: value.thermal_evidence_id.clone(),
            storage_evidence_id: value.storage_evidence_id.clone(),
        }
    }
}

fn find_outcome<'a>(
    evaluation: &'a StrategyBenchmarkEvaluation,
    strategy_id: &str,
    scenario_id: &str,
) -> Result<&'a StrategyBenchmarkOutcome, StrategyBenchmarkError> {
    evaluation
        .outcomes
        .iter()
        .find(|outcome| outcome.strategy_id == strategy_id && outcome.scenario_id == scenario_id)
        .ok_or_else(|| StrategyBenchmarkError::MismatchedScenarioSet {
            strategy_id: strategy_id.to_string(),
            scenario_id: scenario_id.to_string(),
        })
}

fn apply_qualified_pareto_frontier(summaries: &mut [StrategyResourceSummary]) {
    let qualified = summaries
        .iter()
        .filter(|summary| summary.passed)
        .cloned()
        .collect::<Vec<_>>();

    for summary in summaries.iter_mut() {
        if !summary.passed
            || summary.facility_kwh_per_on_time_useful_work_unit.is_none()
            || summary.grid_kwh_per_on_time_useful_work_unit.is_none()
            || summary.operating_cost_per_on_time_useful_work_unit.is_none()
            || summary.water_litres_per_on_time_useful_work_unit.is_none()
            || summary.carbon_kg_per_on_time_useful_work_unit.is_none()
        {
            summary.pareto_qualified = false;
            continue;
        }

        let candidate = summary.clone();
        let mut dominated_by = Vec::new();
        for other in &qualified {
            if other.strategy_id == candidate.strategy_id
                || other.facility_kwh_per_on_time_useful_work_unit.is_none()
                || other.grid_kwh_per_on_time_useful_work_unit.is_none()
                || other.operating_cost_per_on_time_useful_work_unit.is_none()
                || other.water_litres_per_on_time_useful_work_unit.is_none()
                || other.carbon_kg_per_on_time_useful_work_unit.is_none()
            {
                continue;
            }
            if dominates(other, &candidate) {
                dominated_by.push(other.strategy_id.clone());
            }
        }
        dominated_by.sort();
        summary.pareto_qualified = true;
        summary.pareto_dominated_by = dominated_by;
    }
}

fn dominates(left: &StrategyResourceSummary, right: &StrategyResourceSummary) -> bool {
    let left_values = [
        left.facility_kwh_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        left.grid_kwh_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        left.operating_cost_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        left.water_litres_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        left.carbon_kg_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        -left.expected_on_time_useful_work_fraction,
    ];
    let right_values = [
        right.facility_kwh_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        right.grid_kwh_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        right.operating_cost_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        right.water_litres_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        right.carbon_kg_per_on_time_useful_work_unit.unwrap_or(f64::INFINITY),
        -right.expected_on_time_useful_work_fraction,
    ];
    let no_worse = left_values
        .iter()
        .zip(right_values.iter())
        .all(|(left, right)| left <= right);
    let strictly_better = left_values
        .iter()
        .zip(right_values.iter())
        .any(|(left, right)| left < right);
    no_worse && strictly_better
}

fn require_id(field: &'static str, value: &str) -> Result<(), StrategyBenchmarkError> {
    if value.trim().is_empty() {
        return Err(StrategyBenchmarkError::EmptyIdentifier(field));
    }
    Ok(())
}

fn require_probability(field: &'static str, value: f64) -> Result<(), StrategyBenchmarkError> {
    if !value.is_finite() || !(0.0..=1.0).contains(&value) {
        return Err(StrategyBenchmarkError::InvalidProbability { field, value });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn contract() -> StrategyBenchmarkContract {
        StrategyBenchmarkContract::new(
            "benchmark",
            "configuration-digest",
            "service-contract-digest",
            "scenario-set-digest",
            "USD",
            vec!["baseline".to_string(), "candidate".to_string()],
            "baseline",
            1e-9,
            0.95,
            0.90,
            0.02,
            0.01,
            0.01,
            0.01,
            500.0,
        )
        .unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn outcome(
        strategy: &str,
        scenario: &str,
        weight: f64,
        accepted: f64,
        on_time: f64,
        facility: f64,
        cooling: f64,
        water: f64,
        carbon: f64,
        deadline: bool,
        thermal: bool,
        reserve: bool,
        grid: bool,
    ) -> StrategyBenchmarkOutcome {
        StrategyBenchmarkOutcome::new(
            strategy,
            scenario,
            format!("digest-{scenario}"),
            weight,
            100.0,
            accepted,
            on_time,
            facility,
            facility * 0.55,
            cooling,
            facility * 0.80,
            water,
            carbon,
            facility * 0.10,
            100.0,
            30.0,
            deadline,
            thermal,
            reserve,
            grid,
            format!("service-{strategy}-{scenario}"),
            format!("facility-{strategy}-{scenario}"),
            format!("it-{strategy}-{scenario}"),
            format!("cooling-{strategy}-{scenario}"),
            format!("grid-{strategy}-{scenario}"),
            format!("water-{strategy}-{scenario}"),
            format!("carbon-{strategy}-{scenario}"),
            format!("cost-{strategy}-{scenario}"),
            format!("thermal-{strategy}-{scenario}"),
            format!("storage-{strategy}-{scenario}"),
        )
        .unwrap()
    }

    fn evaluation(candidate: Vec<StrategyBenchmarkOutcome>) -> StrategyBenchmarkEvaluation {
        let mut outcomes = vec![
            outcome("baseline", "s1", 0.5, 100.0, 100.0, 120.0, 30.0, 12.0, 6.0, false, false, false, false),
            outcome("baseline", "s2", 0.5, 90.0, 90.0, 110.0, 28.0, 11.0, 5.0, false, false, false, false),
        ];
        outcomes.extend(candidate);
        StrategyBenchmarkEvaluation::new(contract(), outcomes).unwrap()
    }

    fn valid_candidate() -> Vec<StrategyBenchmarkOutcome> {
        vec![
            outcome("candidate", "s1", 0.5, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false),
            outcome("candidate", "s2", 0.5, 90.0, 90.0, 99.0, 22.0, 9.0, 4.0, false, false, false, false),
        ]
    }

    #[test]
    fn service_equivalent_candidate_wins_without_reducing_work() {
        let report = evaluate_strategy_benchmark(&evaluation(valid_candidate())).unwrap();
        let baseline = report.strategy_summaries.iter().find(|s| s.strategy_id == "baseline").unwrap();
        let candidate = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!(candidate.passed);
        assert!(candidate.pareto_qualified);
        assert!(candidate.service_equivalent_energy_savings_vs_baseline_fraction.unwrap() > 0.09);
        assert_eq!(candidate.expected_on_time_useful_work_units, baseline.expected_on_time_useful_work_units);
        assert!((candidate.facility_kwh_per_on_time_useful_work_unit.unwrap() - 1.089_473_684_210_526_3).abs() < 1e-12);
        assert!(candidate.facility_to_it_energy_ratio.unwrap() > 1.0);
        assert!(candidate.grid_kwh_per_on_time_useful_work_unit.unwrap() > 0.0);
    }

    #[test]
    fn candidate_cannot_claim_energy_win_by_completing_less_work() {
        let mut candidate = valid_candidate();
        candidate[0] = outcome("candidate", "s1", 0.5, 80.0, 80.0, 80.0, 18.0, 8.0, 3.0, false, false, false, false);
        candidate[1] = outcome("candidate", "s2", 0.5, 70.0, 70.0, 75.0, 17.0, 7.0, 3.0, false, false, false, false);
        let report = evaluate_strategy_benchmark(&evaluation(candidate)).unwrap();
        let result = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!(!result.service_gate_passed);
        assert!(!result.passed);
    }

    #[test]
    fn late_work_does_not_count_as_on_time_goodput() {
        let mut candidate = valid_candidate();
        candidate[0] = outcome("candidate", "s1", 0.5, 100.0, 80.0, 108.0, 24.0, 10.0, 4.0, true, false, false, false);
        let report = evaluate_strategy_benchmark(&evaluation(candidate)).unwrap();
        let result = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!(result.expected_accepted_useful_work_units > result.expected_on_time_useful_work_units);
        assert!(!result.deadline_gate_passed);
    }

    #[test]
    fn cooling_energy_shift_is_visible_at_the_facility_boundary() {
        let mut candidate = valid_candidate();
        candidate[0] = outcome("candidate", "s1", 0.5, 100.0, 100.0, 118.0, 48.0, 10.0, 4.0, false, false, false, false);
        let report = evaluate_strategy_benchmark(&evaluation(candidate)).unwrap();
        let result = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!((result.expected_cooling_energy_kwh - 35.0).abs() < 1e-12);
        assert!((result.expected_facility_energy_kwh - 108.5).abs() < 1e-12);
    }

    #[test]
    fn water_and_carbon_tradeoffs_are_reported_without_a_blended_score() {
        let mut candidate = valid_candidate();
        candidate[0] = outcome("candidate", "s1", 0.5, 100.0, 100.0, 108.0, 24.0, 20.0, 10.0, false, false, false, false);
        candidate[1] = outcome("candidate", "s2", 0.5, 90.0, 90.0, 99.0, 22.0, 18.0, 9.0, false, false, false, false);
        let report = evaluate_strategy_benchmark(&evaluation(candidate)).unwrap();
        let result = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!(result.service_equivalent_energy_savings_vs_baseline_fraction.unwrap() > 0.09);
        assert!(result.water_litres_per_on_time_useful_work_unit.unwrap() > 0.15);
        assert!(result.carbon_kg_per_on_time_useful_work_unit.unwrap() > 0.08);
        assert!(result.pareto_qualified);
    }

    #[test]
    fn rare_thermal_reserve_and_grid_breaches_are_independent_non_compensable_gates() {
        let mut candidate = valid_candidate();
        candidate[1] = outcome("candidate", "s2", 0.5, 90.0, 90.0, 99.0, 22.0, 9.0, 4.0, false, true, true, true);
        let report = evaluate_strategy_benchmark(&evaluation(candidate)).unwrap();
        let result = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!(!result.thermal_gate_passed);
        assert!(!result.reserve_gate_passed);
        assert!(!result.grid_gate_passed);
        assert!(!result.passed);
    }

    #[test]
    fn worst_case_service_gate_blocks_high_average_with_bad_tail_case() {
        let mut outcomes = vec![
            outcome("baseline", "s1", 0.98, 100.0, 100.0, 120.0, 30.0, 12.0, 6.0, false, false, false, false),
            outcome("baseline", "s2", 0.01, 100.0, 100.0, 110.0, 28.0, 11.0, 5.0, false, false, false, false),
            outcome("baseline", "s3", 0.01, 100.0, 100.0, 110.0, 28.0, 11.0, 5.0, false, false, false, false),
            outcome("candidate", "s1", 0.98, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false),
            outcome("candidate", "s2", 0.01, 100.0, 100.0, 99.0, 22.0, 9.0, 4.0, false, false, false, false),
            outcome("candidate", "s3", 0.01, 50.0, 50.0, 50.0, 10.0, 5.0, 2.0, false, false, false, false),
        ];
        let contract = contract();
        let input = StrategyBenchmarkEvaluation::new(contract, outcomes).unwrap();
        let report = evaluate_strategy_benchmark(&input).unwrap();
        let result = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!(!result.service_gate_passed);
        assert!(result.expected_on_time_useful_work_fraction > 0.98);
        assert_eq!(result.worst_case_on_time_useful_work_fraction, 0.5);
    }

    #[test]
    fn rejects_mismatched_scenario_digest_weight_and_offered_work() {
        let mut outcomes = vec![
            outcome("baseline", "s1", 0.5, 100.0, 100.0, 120.0, 30.0, 12.0, 6.0, false, false, false, false),
            outcome("baseline", "s2", 0.5, 90.0, 90.0, 110.0, 28.0, 11.0, 5.0, false, false, false, false),
            outcome("candidate", "s1", 0.5, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false),
            outcome("candidate", "s2", 0.4, 90.0, 90.0, 99.0, 22.0, 9.0, 4.0, false, false, false, false),
        ];
        assert!(matches!(
            StrategyBenchmarkEvaluation::new(contract(), outcomes.clone()),
            Err(StrategyBenchmarkError::ScenarioWeightMismatch { .. })
        ));
        outcomes[3] = outcome("candidate", "s2", 0.5, 90.0, 90.0, 99.0, 22.0, 9.0, 4.0, false, false, false, false);
        outcomes[3].scenario_digest = "tampered-digest".to_string();
        assert!(matches!(
            StrategyBenchmarkEvaluation::new(contract(), outcomes.clone()),
            Err(StrategyBenchmarkError::ScenarioDigestMismatch { .. })
        ));
        outcomes[3].scenario_digest = "digest-s2".to_string();
        outcomes[3].offered_work_units = 99.0;
        assert!(matches!(
            StrategyBenchmarkEvaluation::new(contract(), outcomes),
            Err(StrategyBenchmarkError::OfferedWorkMismatch { .. })
        ));
    }

    #[test]
    fn rejects_incomplete_or_duplicate_strategy_scenario_matrix() {
        let mut outcomes = vec![
            outcome("baseline", "s1", 0.5, 100.0, 100.0, 120.0, 30.0, 12.0, 6.0, false, false, false, false),
            outcome("baseline", "s2", 0.5, 90.0, 90.0, 110.0, 28.0, 11.0, 5.0, false, false, false, false),
            outcome("candidate", "s1", 0.5, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false),
        ];
        assert!(matches!(
            StrategyBenchmarkEvaluation::new(contract(), outcomes.clone()),
            Err(StrategyBenchmarkError::IncompleteStrategyScenarioMatrix { .. })
        ));
        outcomes.push(outcome("candidate", "s1", 0.5, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false));
        assert!(matches!(
            StrategyBenchmarkEvaluation::new(contract(), outcomes),
            Err(StrategyBenchmarkError::DuplicateStrategyScenarioOutcome { .. })
        ));
    }

    #[test]
    fn raw_weight_drift_is_normalized_once_and_preserved_for_every_strategy() {
        let outcomes = vec![
            outcome("baseline", "s1", 0.5, 100.0, 100.0, 120.0, 30.0, 12.0, 6.0, false, false, false, false),
            outcome("baseline", "s2", 0.499_999_999_5, 90.0, 90.0, 110.0, 28.0, 11.0, 5.0, false, false, false, false),
            outcome("candidate", "s1", 0.5, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false),
            outcome("candidate", "s2", 0.499_999_999_5, 90.0, 90.0, 99.0, 22.0, 9.0, 4.0, false, false, false, false),
        ];
        let input = StrategyBenchmarkEvaluation::new(contract(), outcomes).unwrap();
        let report = evaluate_strategy_benchmark(&input).unwrap();
        assert!(report.probability_mass_normalized);
        assert!((report.probability_weight_sum - 0.999_999_999_5).abs() < 1e-15);
        let receipts = report.outcome_receipts.iter()
            .filter(|receipt| receipt.strategy_id == "candidate")
            .collect::<Vec<_>>();
        assert!((receipts[0].raw_probability_weight - 0.5).abs() < 1e-15);
        assert!((receipts[1].normalized_probability_weight - 0.499_999_999_75).abs() < 1e-15);
        let normalized_sum: f64 = receipts.iter().map(|receipt| receipt.normalized_probability_weight).sum();
        assert!((normalized_sum - 1.0).abs() < 1e-15);
    }

    #[test]
    fn rejects_aggregate_strategy_weight_drift_even_when_each_cell_is_within_tolerance() {
        let outcomes = vec![
            outcome("baseline", "s1", 0.5, 100.0, 100.0, 120.0, 30.0, 12.0, 6.0, false, false, false, false),
            outcome("baseline", "s2", 0.5, 100.0, 100.0, 110.0, 28.0, 11.0, 5.0, false, false, false, false),
            outcome("candidate", "s1", 0.500_000_000_8, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false),
            outcome("candidate", "s2", 0.500_000_000_3, 100.0, 100.0, 99.0, 22.0, 9.0, 4.0, false, false, false, false),
        ];
        assert!(matches!(
            StrategyBenchmarkEvaluation::new(contract(), outcomes),
            Err(StrategyBenchmarkError::StrategyProbabilityMassMismatch { .. })
        ));
    }

    #[test]
    fn latency_gate_uses_the_worst_scenario_p95_not_only_the_average() {
        let mut candidate = valid_candidate();
        candidate[0].p95_latency_ms = 600.0;
        let report = evaluate_strategy_benchmark(&evaluation(candidate)).unwrap();
        let result = report.strategy_summaries.iter().find(|s| s.strategy_id == "candidate").unwrap();
        assert!(result.expected_scenario_p95_latency_ms < 500.0);
        assert_eq!(result.maximum_scenario_p95_latency_ms, 600.0);
        assert!(!result.latency_gate_passed);
        assert!(!result.passed);
    }

    #[test]
    fn zero_on_time_baseline_is_not_used_for_energy_savings_comparison() {
        let mut outcomes = vec![
            outcome("baseline", "s1", 0.5, 100.0, 0.0, 120.0, 30.0, 12.0, 6.0, true, false, false, false),
            outcome("baseline", "s2", 0.5, 100.0, 0.0, 110.0, 28.0, 11.0, 5.0, true, false, false, false),
            outcome("candidate", "s1", 0.5, 100.0, 100.0, 108.0, 24.0, 10.0, 4.0, false, false, false, false),
            outcome("candidate", "s2", 0.5, 100.0, 100.0, 99.0, 22.0, 9.0, 4.0, false, false, false, false),
        ];
        let input = StrategyBenchmarkEvaluation::new(contract(), outcomes).unwrap();
        assert!(matches!(
            evaluate_strategy_benchmark(&input),
            Err(StrategyBenchmarkError::ZeroBaselineOnTimeWork)
        ));
    }

    #[test]
    fn deterministic_report_and_pareto_frontier_do_not_depend_on_input_order() {
        let first = evaluation(valid_candidate());
        let mut reversed = first.outcomes.clone();
        reversed.reverse();
        let second = StrategyBenchmarkEvaluation::new(contract(), reversed).unwrap();
        assert_eq!(
            evaluate_strategy_benchmark(&first).unwrap(),
            evaluate_strategy_benchmark(&second).unwrap()
        );
    }

    #[test]
    fn zero_baseline_energy_intensity_does_not_produce_infinite_savings() {
        let outcomes = vec![
            outcome("baseline", "s1", 0.5, 100.0, 100.0, 0.0, 0.0, 0.0, 0.0, false, false, false, false),
            outcome("baseline", "s2", 0.5, 100.0, 100.0, 0.0, 0.0, 0.0, 0.0, false, false, false, false),
            outcome("candidate", "s1", 0.5, 100.0, 100.0, 10.0, 2.0, 1.0, 1.0, false, false, false, false),
            outcome("candidate", "s2", 0.5, 100.0, 100.0, 10.0, 2.0, 1.0, 1.0, false, false, false, false),
        ];
        let input = StrategyBenchmarkEvaluation::new(contract(), outcomes).unwrap();
        assert!(matches!(
            evaluate_strategy_benchmark(&input),
            Err(StrategyBenchmarkError::ZeroBaselineEnergyIntensity)
        ));
    }

    #[test]
    fn rejects_invalid_quantities_and_work_ordering() {
        assert!(matches!(
            StrategyBenchmarkOutcome::new(
                "s", "case", "digest", 1.0, 100.0, 120.0, 100.0,
                10.0, 5.0, 2.0, 10.0, 1.0, 1.0, 1.0, 100.0, 30.0,
                false, false, false, false,
                "service", "facility", "it", "cooling", "grid",
                "water", "carbon", "cost", "thermal", "storage"
            ),
            Err(StrategyBenchmarkError::WorkQuantityOrdering { .. })
        ));
        assert!(StrategyBenchmarkOutcome::new(
            "s", "case", "digest", f64::NAN, 100.0, 100.0, 100.0,
            10.0, 5.0, 2.0, 10.0, 1.0, 1.0, 1.0, 100.0, 30.0,
            false, false, false, false,
            "service", "facility", "it", "cooling", "grid",
            "water", "carbon", "cost", "thermal", "storage"
        ).is_err());
    }
}
