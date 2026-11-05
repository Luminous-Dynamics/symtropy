// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Single-zone chronological adequacy and ELCC-like load-carrying capability.
//!
//! The reference case treats the candidate data-centre load as inflexible.
//! The augmented case applies a caller-supplied net load-reduction trajectory:
//! positive values are reductions and negative values are rebound/recovery load.
//! Every interval remains chronological within its weather/outage/workload
//! scenario, preserving caller-supplied cross-variable dependence.
//!
//! This is a copperplate prototype. Supply is assumed deliverable to one zone;
//! there is no transmission, unit commitment, reserve, voltage, frequency,
//! storage dispatch, or endogenous workload scheduler. Response trajectories
//! must already include workload deadlines, recovery, outages and failed
//! response opportunities. Scenario weights and traces are caller evidence,
//! not validated probability forecasts. The output is not market accreditation.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

/// The single adequacy criterion used to calculate the ELCC-like result.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ChronologicalAdequacyMetric {
    /// Expected number of simulation intervals containing any positive unserved load.
    LossOfLoadPeriods,
    /// Expected duration of intervals with unserved load, in hours.
    LossOfLoadHours,
    /// Expected number of calendar days with any positive unserved load.
    LossOfLoadDays,
    /// Expected unserved electrical energy over the modeled horizon.
    ExpectedUnservedEnergyMwh,
    /// Upper-tail conditional value at risk of scenario-level EUE.
    /// Confidence level is encoded in basis points (9,500 = 95%).
    UpperTailUnservedEnergyCvar { confidence_level_basis_points: u16 },
}

/// Immutable provenance identity for one frozen adequacy study.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ChronologicalAdequacyIdentity {
    study_id: String,
    portfolio_id: String,
    load_basis_id: String,
    supply_and_outage_basis_id: String,
    flexibility_trajectory_basis_id: String,
    single_zone_model_id: String,
    evidence_scope_id: String,
}

impl ChronologicalAdequacyIdentity {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        study_id: impl Into<String>,
        portfolio_id: impl Into<String>,
        load_basis_id: impl Into<String>,
        supply_and_outage_basis_id: impl Into<String>,
        flexibility_trajectory_basis_id: impl Into<String>,
        single_zone_model_id: impl Into<String>,
        evidence_scope_id: impl Into<String>,
    ) -> Result<Self, ChronologicalAdequacyError> {
        let identity = Self {
            study_id: study_id.into(),
            portfolio_id: portfolio_id.into(),
            load_basis_id: load_basis_id.into(),
            supply_and_outage_basis_id: supply_and_outage_basis_id.into(),
            flexibility_trajectory_basis_id: flexibility_trajectory_basis_id.into(),
            single_zone_model_id: single_zone_model_id.into(),
            evidence_scope_id: evidence_scope_id.into(),
        };
        for (name, value) in [
            ("study ID", identity.study_id.as_str()),
            ("portfolio ID", identity.portfolio_id.as_str()),
            ("load basis ID", identity.load_basis_id.as_str()),
            ("supply and outage basis ID", identity.supply_and_outage_basis_id.as_str()),
            ("flexibility trajectory basis ID", identity.flexibility_trajectory_basis_id.as_str()),
            ("single-zone model ID", identity.single_zone_model_id.as_str()),
            ("evidence scope ID", identity.evidence_scope_id.as_str()),
        ] {
            require_id(name, value)?;
        }
        Ok(identity)
    }

    pub fn study_id(&self) -> &str { &self.study_id }
    pub fn portfolio_id(&self) -> &str { &self.portfolio_id }
    pub fn load_basis_id(&self) -> &str { &self.load_basis_id }
    pub fn supply_and_outage_basis_id(&self) -> &str { &self.supply_and_outage_basis_id }
    pub fn flexibility_trajectory_basis_id(&self) -> &str {
        &self.flexibility_trajectory_basis_id
    }
    pub fn single_zone_model_id(&self) -> &str { &self.single_zone_model_id }
    pub fn evidence_scope_id(&self) -> &str { &self.evidence_scope_id }
}

/// Numerical and search policy for a frozen chronological adequacy evaluation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChronologicalAdequacyContract {
    identity: ChronologicalAdequacyIdentity,
    metric: ChronologicalAdequacyMetric,
    interval_duration_hours: f64,
    shortfall_epsilon_kw: f64,
    adequacy_metric_tolerance: f64,
    maximum_added_load_kw: f64,
    load_search_tolerance_kw: f64,
    maximum_search_iterations: usize,
}

impl ChronologicalAdequacyContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        identity: ChronologicalAdequacyIdentity,
        metric: ChronologicalAdequacyMetric,
        interval_duration_hours: f64,
        shortfall_epsilon_kw: f64,
        adequacy_metric_tolerance: f64,
        maximum_added_load_kw: f64,
        load_search_tolerance_kw: f64,
        maximum_search_iterations: usize,
    ) -> Result<Self, ChronologicalAdequacyError> {
        for (name, value) in [
            ("interval duration hours", interval_duration_hours),
            ("maximum added load kW", maximum_added_load_kw),
            ("load search tolerance kW", load_search_tolerance_kw),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(ChronologicalAdequacyError::InvalidQuantity { name, value });
            }
        }
        for (name, value) in [
            ("shortfall epsilon kW", shortfall_epsilon_kw),
            ("adequacy metric tolerance", adequacy_metric_tolerance),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(ChronologicalAdequacyError::InvalidNonNegativeQuantity {
                    name,
                    value,
                });
            }
        }
        if let ChronologicalAdequacyMetric::UpperTailUnservedEnergyCvar {
            confidence_level_basis_points,
        } = metric
        {
            if !(1..10_000).contains(&confidence_level_basis_points) {
                return Err(ChronologicalAdequacyError::InvalidCvarConfidenceLevel(
                    confidence_level_basis_points,
                ));
            }
        }
        if load_search_tolerance_kw > maximum_added_load_kw {
            return Err(ChronologicalAdequacyError::SearchToleranceExceedsRange {
                tolerance_kw: load_search_tolerance_kw,
                maximum_added_load_kw,
            });
        }
        if !(1..=128).contains(&maximum_search_iterations) {
            return Err(ChronologicalAdequacyError::InvalidSearchIterationCount(
                maximum_search_iterations,
            ));
        }
        Ok(Self {
            identity,
            metric,
            interval_duration_hours,
            shortfall_epsilon_kw,
            adequacy_metric_tolerance,
            maximum_added_load_kw,
            load_search_tolerance_kw,
            maximum_search_iterations,
        })
    }

    pub fn identity(&self) -> &ChronologicalAdequacyIdentity { &self.identity }
    pub const fn metric(&self) -> ChronologicalAdequacyMetric { self.metric }
    pub const fn interval_duration_hours(&self) -> f64 { self.interval_duration_hours }
    pub const fn shortfall_epsilon_kw(&self) -> f64 { self.shortfall_epsilon_kw }
    pub const fn adequacy_metric_tolerance(&self) -> f64 { self.adequacy_metric_tolerance }
    pub const fn maximum_added_load_kw(&self) -> f64 { self.maximum_added_load_kw }
    pub const fn load_search_tolerance_kw(&self) -> f64 { self.load_search_tolerance_kw }
    pub const fn maximum_search_iterations(&self) -> usize {
        self.maximum_search_iterations
    }
}

/// One chronological time interval in one joint weather/outage/response path.
///
/// net_load_reduction_kw is positive for demand reduction and negative for
/// rebound or recovery load. It is a measured/modelled net trajectory relative
/// to the inflexible reference load, not an independently dispatchable rating.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChronologicalAdequacyInterval {
    day_index: u32,
    system_load_kw: f64,
    available_supply_kw: f64,
    net_load_reduction_kw: f64,
    evidence_id: String,
}

impl ChronologicalAdequacyInterval {
    pub fn new(
        day_index: u32,
        system_load_kw: f64,
        available_supply_kw: f64,
        net_load_reduction_kw: f64,
        evidence_id: impl Into<String>,
    ) -> Result<Self, ChronologicalAdequacyError> {
        let evidence_id = evidence_id.into();
        require_id("interval evidence ID", &evidence_id)?;
        for (name, value) in [
            ("system load kW", system_load_kw),
            ("available supply kW", available_supply_kw),
            ("net load reduction kW", net_load_reduction_kw),
        ] {
            if !value.is_finite() {
                return Err(ChronologicalAdequacyError::InvalidQuantity { name, value });
            }
        }
        if system_load_kw < 0.0 {
            return Err(ChronologicalAdequacyError::InvalidNonNegativeQuantity {
                name: "system load kW",
                value: system_load_kw,
            });
        }
        if available_supply_kw < 0.0 {
            return Err(ChronologicalAdequacyError::InvalidNonNegativeQuantity {
                name: "available supply kW",
                value: available_supply_kw,
            });
        }
        if net_load_reduction_kw > system_load_kw {
            return Err(ChronologicalAdequacyError::ReductionExceedsReferenceLoad {
                reduction_kw: net_load_reduction_kw,
                system_load_kw,
            });
        }
        Ok(Self {
            day_index,
            system_load_kw,
            available_supply_kw,
            net_load_reduction_kw,
            evidence_id,
        })
    }

    pub const fn day_index(&self) -> u32 { self.day_index }
    pub const fn system_load_kw(&self) -> f64 { self.system_load_kw }
    pub const fn available_supply_kw(&self) -> f64 { self.available_supply_kw }
    pub const fn net_load_reduction_kw(&self) -> f64 { self.net_load_reduction_kw }
    pub fn evidence_id(&self) -> &str { &self.evidence_id }
}

/// One joint chronological realization with an explicit scenario weight.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChronologicalAdequacyScenario {
    scenario_id: String,
    probability_weight: f64,
    intervals: Vec<ChronologicalAdequacyInterval>,
}

impl ChronologicalAdequacyScenario {
    pub fn new(
        scenario_id: impl Into<String>,
        probability_weight: f64,
        intervals: Vec<ChronologicalAdequacyInterval>,
    ) -> Result<Self, ChronologicalAdequacyError> {
        let scenario_id = scenario_id.into();
        require_id("scenario ID", &scenario_id)?;
        if !probability_weight.is_finite() || probability_weight <= 0.0 {
            return Err(ChronologicalAdequacyError::InvalidScenarioProbability {
                scenario_id,
                probability_weight,
            });
        }
        if intervals.is_empty() {
            return Err(ChronologicalAdequacyError::EmptyScenario(scenario_id));
        }
        Ok(Self { scenario_id, probability_weight, intervals })
    }

    pub fn scenario_id(&self) -> &str { &self.scenario_id }
    pub const fn probability_weight(&self) -> f64 { self.probability_weight }
    pub fn intervals(&self) -> &[ChronologicalAdequacyInterval] { &self.intervals }
}

/// Fully validated, frozen chronological adequacy study.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChronologicalAdequacyStudy {
    contract: ChronologicalAdequacyContract,
    scenarios: Vec<ChronologicalAdequacyScenario>,
}

impl ChronologicalAdequacyStudy {
    pub fn new(
        contract: ChronologicalAdequacyContract,
        mut scenarios: Vec<ChronologicalAdequacyScenario>,
    ) -> Result<Self, ChronologicalAdequacyError> {
        if scenarios.is_empty() {
            return Err(ChronologicalAdequacyError::EmptyScenarioSet);
        }
        scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
        let mut scenario_ids = BTreeSet::new();
        let mut evidence_ids = BTreeSet::new();
        let expected_day_indices = scenarios[0]
            .intervals
            .iter()
            .map(|interval| interval.day_index)
            .collect::<Vec<_>>();
        validate_day_index_sequence(&scenarios[0].scenario_id, &expected_day_indices)?;

        let expected_interval_count = expected_day_indices.len();
        let mut probability_sum = 0.0_f64;
        for scenario in &scenarios {
            if !scenario_ids.insert(scenario.scenario_id.clone()) {
                return Err(ChronologicalAdequacyError::DuplicateScenarioId(
                    scenario.scenario_id.clone(),
                ));
            }
            probability_sum += scenario.probability_weight;
            if !probability_sum.is_finite() {
                return Err(ChronologicalAdequacyError::ProbabilityMassOverflow);
            }
            if scenario.intervals.len() != expected_interval_count {
                return Err(ChronologicalAdequacyError::IntervalCountMismatch {
                    scenario_id: scenario.scenario_id.clone(),
                    observed: scenario.intervals.len(),
                    expected: expected_interval_count,
                });
            }
            for (index, interval) in scenario.intervals.iter().enumerate() {
                if interval.day_index != expected_day_indices[index] {
                    return Err(ChronologicalAdequacyError::ChronologyAlignmentMismatch {
                        scenario_id: scenario.scenario_id.clone(),
                        interval_index: index,
                        expected_day_index: expected_day_indices[index],
                        observed_day_index: interval.day_index,
                    });
                }
                if !evidence_ids.insert((scenario.scenario_id.clone(), interval.evidence_id.clone())) {
                    return Err(ChronologicalAdequacyError::DuplicateIntervalEvidenceId {
                        scenario_id: scenario.scenario_id.clone(),
                        evidence_id: interval.evidence_id.clone(),
                    });
                }
            }
        }
        if probability_sum <= 0.0 || !probability_sum.is_finite() {
            return Err(ChronologicalAdequacyError::ProbabilityMassOverflow);
        }
        Ok(Self { contract, scenarios })
    }

    pub fn contract(&self) -> &ChronologicalAdequacyContract { &self.contract }
    pub fn scenarios(&self) -> &[ChronologicalAdequacyScenario] { &self.scenarios }
}

/// Expected adequacy and resource trajectory metrics for one scenario.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChronologicalAdequacyScenarioReceipt {
    pub scenario_id: String,
    pub raw_probability_weight: f64,
    pub normalized_probability_weight: f64,
    pub reference_loss_of_load_days: u32,
    pub reference_loss_of_load_hours: f64,
    pub reference_unserved_energy_mwh: f64,
    pub augmented_loss_of_load_days: u32,
    pub augmented_loss_of_load_hours: f64,
    pub augmented_unserved_energy_mwh: f64,
    pub augmented_unserved_energy_cvar_mwh: Option<f64>,
    pub augmented_gross_reduction_energy_mwh: f64,
    pub augmented_rebound_energy_mwh: f64,
}

/// Auditable single-zone chronological adequacy result.
///
/// The elcc_like_added_load_kw is the constant added system load that
/// approximately restores the selected reference risk metric. It is not
/// resource accreditation and is only meaningful for the pinned model/traces.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ChronologicalAdequacyReport {
    pub identity: ChronologicalAdequacyIdentity,
    pub metric: ChronologicalAdequacyMetric,
    pub claim_ceiling: String,
    pub scenario_count: usize,
    pub intervals_per_scenario: usize,
    pub raw_probability_weight_sum: f64,
    pub interval_duration_hours: f64,
    pub reference_loss_of_load_days: f64,
    pub reference_loss_of_load_periods: f64,
    pub reference_loss_of_load_hours: f64,
    pub reference_unserved_energy_mwh: f64,
    pub reference_unserved_energy_cvar_mwh: Option<f64>,
    pub augmented_loss_of_load_days: f64,
    pub augmented_loss_of_load_periods: f64,
    pub augmented_loss_of_load_hours: f64,
    pub augmented_unserved_energy_mwh: f64,
    pub augmented_gross_reduction_energy_mwh: f64,
    pub augmented_rebound_energy_mwh: f64,
    pub reference_metric_value: f64,
    pub achieved_augmented_metric_value: f64,
    pub adequacy_metric_tolerance: f64,
    pub residual_metric_delta: f64,
    pub maximum_added_load_kw: f64,
    pub load_search_tolerance_kw: f64,
    pub elcc_like_added_load_kw: f64,
    pub criterion_preserved_at_zero_added_load: bool,
    pub reference_criterion_preserved: bool,
    pub upper_bound_censored: bool,
    pub search_converged: bool,
    pub search_iterations: usize,
    pub scenario_receipts: Vec<ChronologicalAdequacyScenarioReceipt>,
}

#[derive(Debug, Clone, Copy)]
struct ScenarioMetrics {
    loss_of_load_days: u32,
    loss_of_load_periods: u32,
    loss_of_load_hours: f64,
    unserved_energy_mwh: f64,
    gross_reduction_energy_mwh: f64,
    rebound_energy_mwh: f64,
}

#[derive(Debug, Clone, Copy)]
struct AggregateMetrics {
    loss_of_load_days: f64,
    loss_of_load_periods: f64,
    loss_of_load_hours: f64,
    unserved_energy_mwh: f64,
    unserved_energy_cvar_mwh: f64,
    gross_reduction_energy_mwh: f64,
    rebound_energy_mwh: f64,
}

impl AggregateMetrics {
    fn adequacy_value(self, metric: ChronologicalAdequacyMetric) -> f64 {
        match metric {
            ChronologicalAdequacyMetric::LossOfLoadPeriods => self.loss_of_load_periods,
            ChronologicalAdequacyMetric::LossOfLoadHours => self.loss_of_load_hours,
            ChronologicalAdequacyMetric::LossOfLoadDays => self.loss_of_load_days,
            ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh => self.unserved_energy_mwh,
            ChronologicalAdequacyMetric::UpperTailUnservedEnergyCvar { .. } => {
                self.unserved_energy_cvar_mwh
            }
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum ChronologicalAdequacyError {
    MissingIdentity { name: &'static str },
    InvalidQuantity { name: &'static str, value: f64 },
    InvalidNonNegativeQuantity { name: &'static str, value: f64 },
    InvalidScenarioProbability { scenario_id: String, probability_weight: f64 },
    InvalidCvarConfidenceLevel(u16),
    InvalidSearchIterationCount(usize),
    SearchToleranceExceedsRange { tolerance_kw: f64, maximum_added_load_kw: f64 },
    EmptyScenarioSet,
    EmptyScenario(String),
    DuplicateScenarioId(String),
    ProbabilityMassOverflow,
    InvalidDayIndexSequence { scenario_id: String, interval_index: usize },
    IntervalCountMismatch { scenario_id: String, observed: usize, expected: usize },
    ChronologyAlignmentMismatch {
        scenario_id: String,
        interval_index: usize,
        expected_day_index: u32,
        observed_day_index: u32,
    },
    DuplicateIntervalEvidenceId { scenario_id: String, evidence_id: String },
    ReductionExceedsReferenceLoad { reduction_kw: f64, system_load_kw: f64 },
    NoReferenceAdequacyRisk,
    NumericalOverflow(&'static str),
}

impl fmt::Display for ChronologicalAdequacyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { name } => write!(f, "{name} must not be empty"),
            Self::InvalidQuantity { name, value } => {
                write!(f, "{name} must be finite and positive; got {value}")
            }
            Self::InvalidNonNegativeQuantity { name, value } => {
                write!(f, "{name} must be finite and non-negative; got {value}")
            }
            Self::InvalidScenarioProbability { scenario_id, probability_weight } => write!(
                f, "scenario {scenario_id} has invalid probability weight {probability_weight}"
            ),
            Self::InvalidCvarConfidenceLevel(level) => write!(
                f, "CVaR confidence level must be between 1 and 9999 basis points; got {level}"
            ),
            Self::InvalidSearchIterationCount(count) => write!(
                f, "search iteration count must be between 1 and 128; got {count}"
            ),
            Self::SearchToleranceExceedsRange { tolerance_kw, maximum_added_load_kw } => write!(
                f, "load search tolerance {tolerance_kw} kW exceeds search range {maximum_added_load_kw} kW"
            ),
            Self::EmptyScenarioSet => write!(f, "adequacy study requires at least one scenario"),
            Self::EmptyScenario(id) => write!(f, "scenario {id} contains no intervals"),
            Self::DuplicateScenarioId(id) => write!(f, "duplicate scenario ID {id}"),
            Self::ProbabilityMassOverflow => write!(f, "scenario probability mass is non-finite or zero"),
            Self::InvalidDayIndexSequence { scenario_id, interval_index } => write!(
                f, "scenario {scenario_id} has a non-contiguous day-index sequence at interval {interval_index}"
            ),
            Self::IntervalCountMismatch { scenario_id, observed, expected } => write!(
                f, "scenario {scenario_id} has {observed} intervals; expected {expected}"
            ),
            Self::ChronologyAlignmentMismatch { scenario_id, interval_index, expected_day_index, observed_day_index } => write!(
                f, "scenario {scenario_id} day index at interval {interval_index} is {observed_day_index}; expected {expected_day_index}"
            ),
            Self::DuplicateIntervalEvidenceId { scenario_id, evidence_id } => write!(
                f, "scenario {scenario_id} repeats interval evidence ID {evidence_id}"
            ),
            Self::ReductionExceedsReferenceLoad { reduction_kw, system_load_kw } => write!(
                f, "net reduction {reduction_kw} kW exceeds reference load {system_load_kw} kW"
            ),
            Self::NoReferenceAdequacyRisk => write!(
                f, "reference case has zero selected adequacy risk; an ELCC-like increment is not identifiable from this study"
            ),
            Self::NumericalOverflow(operation) => write!(f, "non-finite arithmetic during {operation}"),
        }
    }
}

impl std::error::Error for ChronologicalAdequacyError {}

/// Evaluate chronological reference risk and search for additional load at
/// which the flexible-load augmented case restores the selected reference risk.
pub fn evaluate_chronological_adequacy(
    study: &ChronologicalAdequacyStudy,
) -> Result<ChronologicalAdequacyReport, ChronologicalAdequacyError> {
    let contract = &study.contract;
    let probability_sum = study.scenarios.iter().map(|s| s.probability_weight).sum::<f64>();
    if !probability_sum.is_finite() || probability_sum <= 0.0 {
        return Err(ChronologicalAdequacyError::ProbabilityMassOverflow);
    }

    let reference = aggregate_scenarios(study, probability_sum, 0.0, false, contract.metric)?;
    let reference_value = reference.adequacy_value(contract.metric);
    if reference_value <= 0.0 {
        return Err(ChronologicalAdequacyError::NoReferenceAdequacyRisk);
    }
    let allowed_value = reference_value + contract.adequacy_metric_tolerance;
    if !allowed_value.is_finite() {
        return Err(ChronologicalAdequacyError::NumericalOverflow(
            "reference adequacy tolerance",
        ));
    }

    let at_zero = aggregate_scenarios(study, probability_sum, 0.0, true, contract.metric)?;
    let at_zero_value = at_zero.adequacy_value(contract.metric);
    let criterion_at_zero = at_zero_value <= allowed_value;

    let mut estimate_kw = 0.0;
    let mut final_metrics = at_zero;
    let mut upper_bound_censored = false;
    let mut search_converged = false;
    let mut search_iterations = 0_usize;

    if criterion_at_zero {
        let at_max = aggregate_scenarios(
            study,
            probability_sum,
            contract.maximum_added_load_kw,
            true,
            contract.metric,
        )?;
        let at_max_value = at_max.adequacy_value(contract.metric);
        if at_max_value <= allowed_value {
            estimate_kw = contract.maximum_added_load_kw;
            final_metrics = at_max;
            upper_bound_censored = true;
        } else {
            let mut feasible_kw = 0.0_f64;
            let mut infeasible_kw = contract.maximum_added_load_kw;
            for iteration in 1..=contract.maximum_search_iterations {
                search_iterations = iteration;
                if infeasible_kw - feasible_kw <= contract.load_search_tolerance_kw {
                    search_converged = true;
                    break;
                }
                let midpoint_kw = feasible_kw + (infeasible_kw - feasible_kw) / 2.0;
                let midpoint_metrics =
                    aggregate_scenarios(study, probability_sum, midpoint_kw, true, contract.metric)?;
                if midpoint_metrics.adequacy_value(contract.metric) <= allowed_value {
                    feasible_kw = midpoint_kw;
                } else {
                    infeasible_kw = midpoint_kw;
                }
            }
            if infeasible_kw - feasible_kw <= contract.load_search_tolerance_kw {
                search_converged = true;
            }
            estimate_kw = feasible_kw;
            final_metrics =
                aggregate_scenarios(study, probability_sum, estimate_kw, true, contract.metric)?;
        }
    }

    let achieved_value = final_metrics.adequacy_value(contract.metric);
    let residual_delta = achieved_value - reference_value;
    if ![
        reference.loss_of_load_days,
        reference.loss_of_load_periods,
        reference.loss_of_load_hours,
        reference.unserved_energy_mwh,
        at_zero.loss_of_load_days,
        at_zero.loss_of_load_periods,
        at_zero.loss_of_load_hours,
        at_zero.unserved_energy_mwh,
        final_metrics.loss_of_load_days,
        final_metrics.loss_of_load_periods,
        final_metrics.loss_of_load_hours,
        final_metrics.unserved_energy_mwh,
        achieved_value,
        estimate_kw,
        residual_delta,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(ChronologicalAdequacyError::NumericalOverflow(
            "adequacy report construction",
        ));
    }

    let mut receipts = Vec::with_capacity(study.scenarios.len());
    for scenario in &study.scenarios {
        let normalized_weight = scenario.probability_weight / probability_sum;
        let reference_scenario = evaluate_scenario(
            scenario,
            contract.interval_duration_hours,
            contract.shortfall_epsilon_kw,
            0.0,
            false,
        )?;
        let augmented_scenario = evaluate_scenario(
            scenario,
            contract.interval_duration_hours,
            contract.shortfall_epsilon_kw,
            estimate_kw,
            true,
        )?;
        receipts.push(ChronologicalAdequacyScenarioReceipt {
            scenario_id: scenario.scenario_id.clone(),
            raw_probability_weight: scenario.probability_weight,
            normalized_probability_weight: normalized_weight,
            reference_loss_of_load_days: reference_scenario.loss_of_load_days,
            reference_loss_of_load_hours: reference_scenario.loss_of_load_hours,
            reference_unserved_energy_mwh: reference_scenario.unserved_energy_mwh,
            augmented_loss_of_load_days: augmented_scenario.loss_of_load_days,
            augmented_loss_of_load_hours: augmented_scenario.loss_of_load_hours,
            augmented_unserved_energy_mwh: augmented_scenario.unserved_energy_mwh,
            augmented_gross_reduction_energy_mwh: augmented_scenario.gross_reduction_energy_mwh,
            augmented_rebound_energy_mwh: augmented_scenario.rebound_energy_mwh,
        });
    }

    let claim_ceiling = if upper_bound_censored {
        "SINGLE_ZONE_CHRONOLOGICAL_ELCC_LIKE_LOWER_BOUND_CENSORED_AT_CALLER_SEARCH_CAP_NOT_SYSTEM_RESOURCE_ACCREDITATION"
    } else if !criterion_at_zero {
        "SINGLE_ZONE_CHRONOLOGICAL_STUDY_SHOWS_AUGMENTED_RESOURCE_WORSENS_SELECTED_REFERENCE_RISK_NO_POSITIVE_CAPACITY_VALUE"
    } else if !search_converged {
        "SINGLE_ZONE_CHRONOLOGICAL_ELCC_LIKE_SEARCH_NOT_CONVERGED_NO_CAPACITY_CREDIT_CLAIM"
    } else {
        "SINGLE_ZONE_CHRONOLOGICAL_ELCC_LIKE_LOAD_INCREMENT_FOR_PINNED_TRACES_AND_SELECTED_METRIC_NOT_SYSTEM_RESOURCE_ACCREDITATION"
    };

    Ok(ChronologicalAdequacyReport {
        identity: contract.identity.clone(),
        metric: contract.metric,
        claim_ceiling: claim_ceiling.to_string(),
        scenario_count: study.scenarios.len(),
        intervals_per_scenario: study.scenarios[0].intervals.len(),
        raw_probability_weight_sum: probability_sum,
        interval_duration_hours: contract.interval_duration_hours,
        reference_loss_of_load_days: reference.loss_of_load_days,
        reference_loss_of_load_periods: reference.loss_of_load_periods,
        reference_loss_of_load_hours: reference.loss_of_load_hours,
        reference_unserved_energy_mwh: reference.unserved_energy_mwh,
        reference_unserved_energy_cvar_mwh: cvar_if_selected(contract.metric, reference.unserved_energy_cvar_mwh),
        augmented_loss_of_load_days: final_metrics.loss_of_load_days,
        augmented_loss_of_load_periods: final_metrics.loss_of_load_periods,
        augmented_loss_of_load_hours: final_metrics.loss_of_load_hours,
        augmented_unserved_energy_mwh: final_metrics.unserved_energy_mwh,
        augmented_unserved_energy_cvar_mwh: cvar_if_selected(contract.metric, final_metrics.unserved_energy_cvar_mwh),
        augmented_gross_reduction_energy_mwh: final_metrics.gross_reduction_energy_mwh,
        augmented_rebound_energy_mwh: final_metrics.rebound_energy_mwh,
        reference_metric_value: reference_value,
        achieved_augmented_metric_value: achieved_value,
        adequacy_metric_tolerance: contract.adequacy_metric_tolerance,
        residual_metric_delta: residual_delta,
        maximum_added_load_kw: contract.maximum_added_load_kw,
        load_search_tolerance_kw: contract.load_search_tolerance_kw,
        elcc_like_added_load_kw: estimate_kw,
        criterion_preserved_at_zero_added_load: criterion_at_zero,
        reference_criterion_preserved: criterion_at_zero && achieved_value <= allowed_value,
        upper_bound_censored,
        search_converged,
        search_iterations,
        scenario_receipts: receipts,
    })
}

fn aggregate_scenarios(
    study: &ChronologicalAdequacyStudy,
    probability_sum: f64,
    added_load_kw: f64,
    apply_flexibility: bool,
    metric: ChronologicalAdequacyMetric,
) -> Result<AggregateMetrics, ChronologicalAdequacyError> {
    let mut aggregate = AggregateMetrics {
        loss_of_load_days: 0.0,
        loss_of_load_periods: 0.0,
        loss_of_load_hours: 0.0,
        unserved_energy_mwh: 0.0,
        unserved_energy_cvar_mwh: 0.0,
        gross_reduction_energy_mwh: 0.0,
        rebound_energy_mwh: 0.0,
    };
    let mut scenario_eue = Vec::with_capacity(study.scenarios.len());
    for scenario in &study.scenarios {
        let weight = scenario.probability_weight / probability_sum;
        let metrics = evaluate_scenario(
            scenario,
            study.contract.interval_duration_hours,
            study.contract.shortfall_epsilon_kw,
            added_load_kw,
            apply_flexibility,
        )?;
        aggregate.loss_of_load_days += weight * f64::from(metrics.loss_of_load_days);
        aggregate.loss_of_load_periods += weight * f64::from(metrics.loss_of_load_periods);
        aggregate.loss_of_load_hours += weight * metrics.loss_of_load_hours;
        aggregate.unserved_energy_mwh += weight * metrics.unserved_energy_mwh;
        aggregate.gross_reduction_energy_mwh += weight * metrics.gross_reduction_energy_mwh;
        aggregate.rebound_energy_mwh += weight * metrics.rebound_energy_mwh;
        scenario_eue.push((metrics.unserved_energy_mwh, weight));
    }
    if let ChronologicalAdequacyMetric::UpperTailUnservedEnergyCvar {
        confidence_level_basis_points,
    } = metric
    {
        aggregate.unserved_energy_cvar_mwh =
            weighted_upper_tail_cvar(&scenario_eue, confidence_level_basis_points)?;
    }
    if ![
        aggregate.loss_of_load_days,
        aggregate.loss_of_load_periods,
        aggregate.loss_of_load_hours,
        aggregate.unserved_energy_mwh,
        aggregate.unserved_energy_cvar_mwh,
        aggregate.gross_reduction_energy_mwh,
        aggregate.rebound_energy_mwh,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(ChronologicalAdequacyError::NumericalOverflow(
            "weighted scenario aggregation",
        ));
    }
    Ok(aggregate)
}

fn cvar_if_selected(
    metric: ChronologicalAdequacyMetric,
    value: f64,
) -> Option<f64> {
    matches!(
        metric,
        ChronologicalAdequacyMetric::UpperTailUnservedEnergyCvar { .. }
    )
    .then_some(value)
}

/// Weighted mean of the worst (1 - alpha) probability mass of scenario EUE.
fn weighted_upper_tail_cvar(
    scenario_eue: &[(f64, f64)],
    confidence_level_basis_points: u16,
) -> Result<f64, ChronologicalAdequacyError> {
    if !(1..10_000).contains(&confidence_level_basis_points) || scenario_eue.is_empty() {
        return Err(ChronologicalAdequacyError::InvalidCvarConfidenceLevel(
            confidence_level_basis_points,
        ));
    }
    let mut outcomes = scenario_eue.to_vec();
    let mut total_weight = 0.0_f64;
    for (eue, weight) in &outcomes {
        if !eue.is_finite() || *eue < 0.0 || !weight.is_finite() || *weight <= 0.0 {
            return Err(ChronologicalAdequacyError::NumericalOverflow(
                "CVaR input distribution",
            ));
        }
        total_weight += *weight;
    }
    if !total_weight.is_finite() || total_weight <= 0.0 {
        return Err(ChronologicalAdequacyError::NumericalOverflow(
            "CVaR probability mass",
        ));
    }
    outcomes.sort_by(|left, right| right.0.total_cmp(&left.0));
    let tail_mass = 1.0 - f64::from(confidence_level_basis_points) / 10_000.0;
    let mut remaining_mass = tail_mass;
    let mut tail_energy = 0.0_f64;
    for (eue, raw_weight) in outcomes {
        let normalized_weight = raw_weight / total_weight;
        let included_weight = normalized_weight.min(remaining_mass);
        tail_energy += eue * included_weight;
        remaining_mass -= included_weight;
        if remaining_mass <= total_weight * 1e-12 {
            break;
        }
    }
    if !tail_energy.is_finite()
        || !tail_mass.is_finite()
        || tail_mass <= 0.0
        || remaining_mass > total_weight * 1e-10
    {
        return Err(ChronologicalAdequacyError::NumericalOverflow(
            "weighted upper-tail CVaR",
        ));
    }
    let result = tail_energy / tail_mass;
    if !result.is_finite() {
        return Err(ChronologicalAdequacyError::NumericalOverflow(
            "weighted upper-tail CVaR result",
        ));
    }
    Ok(result)
}

fn evaluate_scenario(
    scenario: &ChronologicalAdequacyScenario,
    interval_duration_hours: f64,
    shortfall_epsilon_kw: f64,
    added_load_kw: f64,
    apply_flexibility: bool,
) -> Result<ScenarioMetrics, ChronologicalAdequacyError> {
    let mut loss_days = BTreeSet::new();
    let mut loss_of_load_periods = 0_u32;
    let mut loss_of_load_hours = 0.0_f64;
    let mut unserved_energy_mwh = 0.0_f64;
    let mut gross_reduction_energy_mwh = 0.0_f64;
    let mut rebound_energy_mwh = 0.0_f64;

    for interval in &scenario.intervals {
        let net_reduction_kw = if apply_flexibility {
            interval.net_load_reduction_kw
        } else {
            0.0
        };
        let net_load_kw = interval.system_load_kw + added_load_kw - net_reduction_kw;
        let unserved_kw = (net_load_kw - interval.available_supply_kw).max(0.0);
        let interval_unserved_mwh = unserved_kw * interval_duration_hours / 1_000.0;
        let reduction_mwh =
            net_reduction_kw.max(0.0) * interval_duration_hours / 1_000.0;
        let rebound_mwh =
            (-net_reduction_kw).max(0.0) * interval_duration_hours / 1_000.0;
        loss_of_load_hours += if unserved_kw > shortfall_epsilon_kw {
            interval_duration_hours
        } else {
            0.0
        };
        if unserved_kw > shortfall_epsilon_kw {
            loss_days.insert(interval.day_index);
            loss_of_load_periods = loss_of_load_periods.checked_add(1).ok_or(
                ChronologicalAdequacyError::NumericalOverflow("loss-of-load period count"),
            )?;
        }
        unserved_energy_mwh += interval_unserved_mwh;
        gross_reduction_energy_mwh += reduction_mwh;
        rebound_energy_mwh += rebound_mwh;
    }

    if ![
        loss_of_load_hours,
        unserved_energy_mwh,
        gross_reduction_energy_mwh,
        rebound_energy_mwh,
    ]
    .iter()
    .all(|value| value.is_finite())
    {
        return Err(ChronologicalAdequacyError::NumericalOverflow(
            "chronological interval accounting",
        ));
    }

    Ok(ScenarioMetrics {
        loss_of_load_days: u32::try_from(loss_days.len()).map_err(|_| {
            ChronologicalAdequacyError::NumericalOverflow("loss-of-load day count")
        })?,
        loss_of_load_periods,
        loss_of_load_hours,
        unserved_energy_mwh,
        gross_reduction_energy_mwh,
        rebound_energy_mwh,
    })
}

fn validate_day_index_sequence(
    scenario_id: &str,
    day_indices: &[u32],
) -> Result<(), ChronologicalAdequacyError> {
    if day_indices.first().copied() != Some(0) {
        return Err(ChronologicalAdequacyError::InvalidDayIndexSequence {
            scenario_id: scenario_id.to_string(),
            interval_index: 0,
        });
    }
    for (index, pair) in day_indices.windows(2).enumerate() {
        if pair[1] < pair[0] || pair[1] > pair[0].saturating_add(1) {
            return Err(ChronologicalAdequacyError::InvalidDayIndexSequence {
                scenario_id: scenario_id.to_string(),
                interval_index: index + 1,
            });
        }
    }
    Ok(())
}

fn require_id(name: &'static str, value: &str) -> Result<(), ChronologicalAdequacyError> {
    if value.trim().is_empty() {
        Err(ChronologicalAdequacyError::MissingIdentity { name })
    } else {
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn identity() -> ChronologicalAdequacyIdentity {
        ChronologicalAdequacyIdentity::new(
            "study-1",
            "portfolio-1",
            "load-basis-1",
            "supply-outage-basis-1",
            "flex-trajectory-basis-1",
            "single-zone-copperplate-v1",
            "synthetic-known-truth",
        ).expect("valid identity")
    }

    fn contract(
        metric: ChronologicalAdequacyMetric,
        maximum_added_load_kw: f64,
    ) -> ChronologicalAdequacyContract {
        ChronologicalAdequacyContract::new(
            identity(),
            metric,
            1.0,
            1e-9,
            1e-9,
            maximum_added_load_kw,
            1e-6,
            64,
        ).expect("valid contract")
    }

    fn interval(
        day_index: u32,
        load_kw: f64,
        supply_kw: f64,
        net_reduction_kw: f64,
        id: &str,
    ) -> ChronologicalAdequacyInterval {
        ChronologicalAdequacyInterval::new(
            day_index,
            load_kw,
            supply_kw,
            net_reduction_kw,
            id,
        ).expect("valid interval")
    }

    #[test]
    fn weighted_eue_search_recovers_known_17_5_kw_value() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh, 40.0),
            vec![
                ChronologicalAdequacyScenario::new(
                    "low-flex",
                    1.0,
                    vec![interval(0, 100.0, 90.0, 10.0, "low-0")],
                ).expect("scenario"),
                ChronologicalAdequacyScenario::new(
                    "high-flex",
                    3.0,
                    vec![interval(0, 100.0, 90.0, 20.0, "high-0")],
                ).expect("scenario"),
            ],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert!((report.reference_unserved_energy_mwh - 0.01).abs() < 1e-12);
        assert!((report.elcc_like_added_load_kw - 17.5).abs() < 1e-5);
        assert!((report.achieved_augmented_metric_value - 0.01).abs() < 1e-8);
        assert!((report.scenario_receipts[0].normalized_probability_weight - 0.75).abs() < 1e-12);
        assert!((report.scenario_receipts[1].normalized_probability_weight - 0.25).abs() < 1e-12);
        assert!(report.reference_criterion_preserved);
        assert!(report.search_converged);
        assert!(!report.upper_bound_censored);
    }

    #[test]
    fn loss_of_load_days_are_deduplicated_within_a_day() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::LossOfLoadDays, 20.0),
            vec![ChronologicalAdequacyScenario::new(
                "year-a",
                1.0,
                vec![
                    interval(0, 100.0, 90.0, 10.0, "a-0"),
                    interval(0, 100.0, 95.0, 0.0, "a-1"),
                ],
            ).expect("scenario")],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert_eq!(report.reference_loss_of_load_days, 1.0);
        assert_eq!(report.reference_loss_of_load_hours, 2.0);
    }

    #[test]
    fn a_capacity_value_censored_at_search_cap_is_not_presented_as_exact() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh, 5.0),
            vec![ChronologicalAdequacyScenario::new(
                "year-a",
                1.0,
                vec![interval(0, 100.0, 90.0, 20.0, "a-0")],
            ).expect("scenario")],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert_eq!(report.elcc_like_added_load_kw, 5.0);
        assert!(report.upper_bound_censored);
        assert!(!report.search_converged);
        assert!(report.claim_ceiling.contains("LOWER_BOUND_CENSORED"));
    }

    #[test]
    fn rebound_can_make_the_augmented_case_worse_than_reference() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh, 20.0),
            vec![ChronologicalAdequacyScenario::new(
                "rebound-year",
                1.0,
                vec![
                    interval(0, 100.0, 90.0, 20.0, "r-0"),
                    interval(1, 100.0, 80.0, -30.0, "r-1"),
                ],
            ).expect("scenario")],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert_eq!(report.elcc_like_added_load_kw, 0.0);
        assert!(!report.criterion_preserved_at_zero_added_load);
        assert!(!report.reference_criterion_preserved);
        assert!((report.augmented_rebound_energy_mwh - 0.03).abs() < 1e-12);
        assert!(report.claim_ceiling.contains("WORSENS_SELECTED_REFERENCE_RISK"));
    }

    #[test]
    fn zero_reference_risk_is_rejected_as_unidentifiable() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh, 20.0),
            vec![ChronologicalAdequacyScenario::new(
                "healthy-year",
                1.0,
                vec![interval(0, 80.0, 100.0, 10.0, "h-0")],
            ).expect("scenario")],
        ).expect("study");
        assert_eq!(
            evaluate_chronological_adequacy(&study),
            Err(ChronologicalAdequacyError::NoReferenceAdequacyRisk)
        );
    }

    #[test]
    fn response_reduction_cannot_exceed_reference_load() {
        assert!(matches!(
            ChronologicalAdequacyInterval::new(0, 10.0, 20.0, 10.1, "bad"),
            Err(ChronologicalAdequacyError::ReductionExceedsReferenceLoad { .. })
        ));
    }

    #[test]
    fn scenario_chronologies_must_align() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh, 20.0),
            vec![
                ChronologicalAdequacyScenario::new(
                    "a",
                    1.0,
                    vec![interval(0, 100.0, 90.0, 10.0, "a-0")],
                ).expect("scenario"),
                ChronologicalAdequacyScenario::new(
                    "b",
                    1.0,
                    vec![interval(1, 100.0, 90.0, 10.0, "b-0")],
                ).expect("scenario"),
            ],
        );
        assert!(matches!(
            study,
            Err(ChronologicalAdequacyError::ChronologyAlignmentMismatch { .. })
        ));
    }

    #[test]
    fn non_contiguous_day_indices_are_rejected() {
        let result = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh, 20.0),
            vec![ChronologicalAdequacyScenario::new(
                "gapped-year",
                1.0,
                vec![
                    interval(0, 100.0, 90.0, 10.0, "g-0"),
                    interval(2, 100.0, 90.0, 10.0, "g-1"),
                ],
            ).expect("scenario")],
        );
        assert!(matches!(
            result,
            Err(ChronologicalAdequacyError::InvalidDayIndexSequence { .. })
        ));
    }

    #[test]
    fn zero_flexibility_has_no_positive_load_carrying_value() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::ExpectedUnservedEnergyMwh, 20.0),
            vec![ChronologicalAdequacyScenario::new(
                "no-flex",
                1.0,
                vec![interval(0, 100.0, 90.0, 0.0, "n-0")],
            ).expect("scenario")],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert!(report.elcc_like_added_load_kw.abs() < 1e-6);
        assert!(report.reference_criterion_preserved);
        assert!(report.search_converged);
    }
    #[test]
    fn invalid_scenario_probability_is_rejected() {
        assert!(matches!(
            ChronologicalAdequacyScenario::new(
                "invalid-probability",
                0.0,
                vec![interval(0, 100.0, 90.0, 10.0, "bad-p-0")],
            ),
            Err(ChronologicalAdequacyError::InvalidScenarioProbability { .. })
        ));
    }

    #[test]
    fn one_day_can_have_multiple_loss_hours_but_only_one_loss_day() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::LossOfLoadDays, 20.0),
            vec![ChronologicalAdequacyScenario::new(
                "two-hours-one-day",
                1.0,
                vec![
                    interval(0, 100.0, 90.0, 0.0, "two-0"),
                    interval(0, 100.0, 95.0, 0.0, "two-1"),
                ],
            ).expect("scenario")],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert_eq!(report.reference_loss_of_load_days, 1.0);
        assert_eq!(report.reference_loss_of_load_hours, 2.0);
    }


    #[test]
    fn weighted_upper_tail_cvar_uses_fractional_probability_mass() {
        let value = weighted_upper_tail_cvar(
            &[(0.01, 0.9), (0.03, 0.1)],
            8_000,
        ).expect("weighted CVaR");
        assert!((value - 0.02).abs() < 1e-12);
    }

    #[test]
    fn cvar_search_restores_reference_upper_tail_eue() {
        let study = ChronologicalAdequacyStudy::new(
            contract(
                ChronologicalAdequacyMetric::UpperTailUnservedEnergyCvar {
                    confidence_level_basis_points: 9_000,
                },
                20.0,
            ),
            vec![
                ChronologicalAdequacyScenario::new(
                    "high-stress",
                    0.1,
                    vec![interval(0, 100.0, 80.0, 10.0, "high-0")],
                ).expect("high scenario"),
                ChronologicalAdequacyScenario::new(
                    "low-stress",
                    0.9,
                    vec![interval(0, 100.0, 90.0, 0.0, "low-0")],
                ).expect("low scenario"),
            ],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert!((report.reference_unserved_energy_cvar_mwh.unwrap() - 0.02).abs() < 1e-12);
        assert!((report.elcc_like_added_load_kw - 10.0).abs() < 1e-5);
        assert!((report.augmented_unserved_energy_cvar_mwh.unwrap() - 0.02).abs() < 1e-8);
        assert!(report.reference_criterion_preserved);
        assert!(report.search_converged);
    }

    #[test]
    fn invalid_cvar_confidence_levels_are_rejected() {
        for level in [0, 10_000, u16::MAX] {
            let result = ChronologicalAdequacyContract::new(
                identity(),
                ChronologicalAdequacyMetric::UpperTailUnservedEnergyCvar {
                    confidence_level_basis_points: level,
                },
                1.0,
                1e-9,
                1e-9,
                20.0,
                1e-6,
                64,
            );
            assert!(matches!(
                result,
                Err(ChronologicalAdequacyError::InvalidCvarConfidenceLevel(_))
            ));
        }
    }

    #[test]
    fn loss_of_load_periods_match_simulation_steps_not_elapsed_hours() {
        let c = ChronologicalAdequacyContract::new(
            identity(),
            ChronologicalAdequacyMetric::LossOfLoadPeriods,
            0.25,
            1e-9,
            1e-9,
            1.0,
            1e-6,
            64,
        ).expect("contract");
        let study = ChronologicalAdequacyStudy::new(
            c,
            vec![ChronologicalAdequacyScenario::new(
                "two-quarter-hours",
                1.0,
                vec![
                    interval(0, 100.0, 90.0, 0.0, "period-0"),
                    interval(0, 100.0, 95.0, 0.0, "period-1"),
                ],
            ).expect("scenario")],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert_eq!(report.reference_loss_of_load_periods, 2.0);
        assert_eq!(report.reference_loss_of_load_hours, 0.5);
        assert_eq!(report.reference_loss_of_load_days, 1.0);
        assert_eq!(report.reference_metric_value, 2.0);
    }

    #[test]
    fn loss_of_load_hours_and_days_are_distinct_primary_metrics() {
        let study = ChronologicalAdequacyStudy::new(
            contract(ChronologicalAdequacyMetric::LossOfLoadHours, 10.0),
            vec![ChronologicalAdequacyScenario::new(
                "clustered-shortfall",
                1.0,
                vec![
                    interval(0, 100.0, 90.0, 0.0, "lole-0"),
                    interval(0, 100.0, 95.0, 0.0, "lole-1"),
                ],
            ).expect("scenario")],
        ).expect("study");
        let report = evaluate_chronological_adequacy(&study).expect("evaluation");
        assert_eq!(report.reference_loss_of_load_hours, 2.0);
        assert_eq!(report.reference_loss_of_load_days, 1.0);
        assert_eq!(report.reference_metric_value, 2.0);
    }


}
