// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Joint probabilistic scenario-forecast evaluation.
//!
//! Evaluates complete weighted multivariate paths using:
//! - a normalized weighted Energy Score;
//! - a caller-pinned Variogram Score with p = 0.5;
//! - Brier scores for declared conjunction events, including joint scarcity.
//!
//! The ensemble is one joint distribution over complete paths. The evaluator
//! never reconstructs joint paths by multiplying marginal probabilities.
//! It normalizes supplied weights only when their total is within the frozen
//! tolerance and preserves raw and normalized weights in every receipt.
//!
//! This is an evaluator, not a scenario generator, a calibration guarantee,
//! a DRO ambiguity-set constructor, or an evidence authenticator.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

/// Comparison operator for a forecast event condition.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ScenarioConditionOperator {
    LessThan,
    LessThanOrEqual,
    GreaterThan,
    GreaterThanOrEqual,
}

/// Fixed score-normalization scale and its provenance for one forecast dimension.
///
/// The same frozen scale is used in the marginal CRPS, Energy Score and
/// Variogram Score. Event thresholds remain in the original dimension units.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioDimensionScale {
    dimension_id: String,
    normalization_scale: f64,
    evidence_id: String,
}

impl ScenarioDimensionScale {
    pub fn new(
        dimension_id: impl Into<String>,
        normalization_scale: f64,
        evidence_id: impl Into<String>,
    ) -> Result<Self, JointScenarioForecastError> {
        let scale = Self {
            dimension_id: dimension_id.into(),
            normalization_scale,
            evidence_id: evidence_id.into(),
        };
        require_id("dimension ID", &scale.dimension_id)?;
        require_id("normalization-scale evidence ID", &scale.evidence_id)?;
        if !scale.normalization_scale.is_finite() || scale.normalization_scale <= 0.0 {
            return Err(JointScenarioForecastError::InvalidNormalizationScale {
                dimension_id: scale.dimension_id,
                value: scale.normalization_scale,
            });
        }
        Ok(scale)
    }

    pub fn dimension_id(&self) -> &str {
        &self.dimension_id
    }

    pub const fn normalization_scale(&self) -> f64 {
        self.normalization_scale
    }

    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }
}

/// A dimension pair included in the dependence-sensitive variogram score.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioVariogramPair {
    left_dimension_id: String,
    right_dimension_id: String,
    weight: f64,
}

impl ScenarioVariogramPair {
    pub fn new(
        left_dimension_id: impl Into<String>,
        right_dimension_id: impl Into<String>,
        weight: f64,
    ) -> Result<Self, JointScenarioForecastError> {
        let mut pair = Self {
            left_dimension_id: left_dimension_id.into(),
            right_dimension_id: right_dimension_id.into(),
            weight,
        };
        require_id("left variogram dimension ID", &pair.left_dimension_id)?;
        require_id("right variogram dimension ID", &pair.right_dimension_id)?;
        if pair.left_dimension_id == pair.right_dimension_id {
            return Err(JointScenarioForecastError::InvalidVariogramPair {
                left: pair.left_dimension_id,
                right: pair.right_dimension_id,
            });
        }
        if !pair.weight.is_finite() || pair.weight <= 0.0 {
            return Err(JointScenarioForecastError::InvalidVariogramWeight(pair.weight));
        }
        if pair.left_dimension_id > pair.right_dimension_id {
            core::mem::swap(
                &mut pair.left_dimension_id,
                &mut pair.right_dimension_id,
            );
        }
        Ok(pair)
    }
}

/// One conjunction term for an operational event probability.
///
/// Thresholds use the original units of their declared dimension; score
/// normalization scales are never applied to event predicates.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioEventCondition {
    dimension_id: String,
    operator: ScenarioConditionOperator,
    threshold: f64,
}

impl ScenarioEventCondition {
    pub fn new(
        dimension_id: impl Into<String>,
        operator: ScenarioConditionOperator,
        threshold: f64,
    ) -> Result<Self, JointScenarioForecastError> {
        let condition = Self {
            dimension_id: dimension_id.into(),
            operator,
            threshold,
        };
        require_id("event condition dimension ID", &condition.dimension_id)?;
        if !condition.threshold.is_finite() {
            return Err(JointScenarioForecastError::NonFiniteValue {
                field: "event threshold",
                value: condition.threshold,
            });
        }
        Ok(condition)
    }
}

/// Event probability computed as the conjunction of all its conditions.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioEventDefinition {
    event_id: String,
    conditions: Vec<ScenarioEventCondition>,
}

impl ScenarioEventDefinition {
    pub fn new(
        event_id: impl Into<String>,
        mut conditions: Vec<ScenarioEventCondition>,
    ) -> Result<Self, JointScenarioForecastError> {
        let event = Self {
            event_id: event_id.into(),
            conditions: {
                conditions.sort_by(|left, right| {
                    left.dimension_id.cmp(&right.dimension_id)
                });
                conditions
            },
        };
        require_id("event ID", &event.event_id)?;
        if event.conditions.is_empty() {
            return Err(JointScenarioForecastError::EmptyEventConditions(
                event.event_id,
            ));
        }
        for pair in event.conditions.windows(2) {
            if pair[0].dimension_id == pair[1].dimension_id {
                return Err(JointScenarioForecastError::DuplicateEventDimension {
                    event_id: event.event_id.clone(),
                    dimension_id: pair[0].dimension_id.clone(),
                });
            }
        }
        Ok(event)
    }

    pub fn event_id(&self) -> &str {
        &self.event_id
    }

    pub fn conditions(&self) -> &[ScenarioEventCondition] {
        &self.conditions
    }
}

/// Frozen evaluation contract for one multivariate scenario benchmark.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JointScenarioForecastContract {
    benchmark_id: String,
    forecast_model_digest: String,
    evaluation_configuration_digest: String,
    dimensions: Vec<ScenarioDimensionScale>,
    variogram_pairs: Vec<ScenarioVariogramPair>,
    events: Vec<ScenarioEventDefinition>,
    probability_sum_tolerance: f64,
    minimum_cases: usize,
}

impl JointScenarioForecastContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        benchmark_id: impl Into<String>,
        forecast_model_digest: impl Into<String>,
        evaluation_configuration_digest: impl Into<String>,
        dimensions: Vec<ScenarioDimensionScale>,
        mut variogram_pairs: Vec<ScenarioVariogramPair>,
        mut events: Vec<ScenarioEventDefinition>,
        probability_sum_tolerance: f64,
        minimum_cases: usize,
    ) -> Result<Self, JointScenarioForecastError> {
        let contract = Self {
            benchmark_id: benchmark_id.into(),
            forecast_model_digest: forecast_model_digest.into(),
            evaluation_configuration_digest: evaluation_configuration_digest.into(),
            dimensions,
            variogram_pairs: {
                variogram_pairs.sort_by(|left, right| {
                    left.left_dimension_id
                        .cmp(&right.left_dimension_id)
                        .then_with(|| left.right_dimension_id.cmp(&right.right_dimension_id))
                });
                variogram_pairs
            },
            events: {
                events.sort_by(|left, right| left.event_id.cmp(&right.event_id));
                events
            },
            probability_sum_tolerance,
            minimum_cases,
        };
        require_id("benchmark ID", &contract.benchmark_id)?;
        require_id("forecast-model digest", &contract.forecast_model_digest)?;
        require_id(
            "evaluation configuration digest",
            &contract.evaluation_configuration_digest,
        )?;
        if contract.dimensions.is_empty() {
            return Err(JointScenarioForecastError::EmptyDimensionSet);
        }
        let mut dimension_ids = BTreeSet::new();
        for dimension in &contract.dimensions {
            if !dimension_ids.insert(dimension.dimension_id.clone()) {
                return Err(JointScenarioForecastError::DuplicateDimension(
                    dimension.dimension_id.clone(),
                ));
            }
        }
        if contract.variogram_pairs.is_empty() {
            return Err(JointScenarioForecastError::EmptyVariogramPairs);
        }
        for pair in &contract.variogram_pairs {
            if !dimension_ids.contains(&pair.left_dimension_id)
                || !dimension_ids.contains(&pair.right_dimension_id)
            {
                return Err(JointScenarioForecastError::UnknownVariogramDimension {
                    left: pair.left_dimension_id.clone(),
                    right: pair.right_dimension_id.clone(),
                });
            }
        }
        for pair in contract.variogram_pairs.windows(2) {
            if pair[0].left_dimension_id == pair[1].left_dimension_id
                && pair[0].right_dimension_id == pair[1].right_dimension_id
            {
                return Err(JointScenarioForecastError::DuplicateVariogramPair {
                    left: pair[0].left_dimension_id.clone(),
                    right: pair[0].right_dimension_id.clone(),
                });
            }
        }
        if contract.events.is_empty() {
            return Err(JointScenarioForecastError::EmptyEventSet);
        }
        for event in &contract.events {
            for condition in &event.conditions {
                if !dimension_ids.contains(&condition.dimension_id) {
                    return Err(JointScenarioForecastError::UnknownEventDimension {
                        event_id: event.event_id.clone(),
                        dimension_id: condition.dimension_id.clone(),
                    });
                }
            }
        }
        for pair in contract.events.windows(2) {
            if pair[0].event_id == pair[1].event_id {
                return Err(JointScenarioForecastError::DuplicateEventId(
                    pair[0].event_id.clone(),
                ));
            }
        }
        if !contract.probability_sum_tolerance.is_finite()
            || contract.probability_sum_tolerance <= 0.0
            || contract.probability_sum_tolerance > 1e-3
        {
            return Err(JointScenarioForecastError::InvalidProbabilityTolerance(
                contract.probability_sum_tolerance,
            ));
        }
        if contract.minimum_cases == 0 {
            return Err(JointScenarioForecastError::InvalidMinimumCases);
        }
        Ok(contract)
    }

    pub fn benchmark_id(&self) -> &str {
        &self.benchmark_id
    }

    pub fn forecast_model_digest(&self) -> &str {
        &self.forecast_model_digest
    }

    pub fn evaluation_configuration_digest(&self) -> &str {
        &self.evaluation_configuration_digest
    }

    pub fn dimensions(&self) -> &[ScenarioDimensionScale] {
        &self.dimensions
    }

    pub fn variogram_pairs(&self) -> &[ScenarioVariogramPair] {
        &self.variogram_pairs
    }

    pub fn events(&self) -> &[ScenarioEventDefinition] {
        &self.events
    }

    pub const fn probability_sum_tolerance(&self) -> f64 {
        self.probability_sum_tolerance
    }
}

/// One weighted complete-path scenario from a joint forecast ensemble.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct WeightedScenarioPath {
    scenario_id: String,
    scenario_digest: String,
    probability_weight: f64,
    values: Vec<f64>,
    evidence_id: String,
}

impl WeightedScenarioPath {
    pub fn new(
        scenario_id: impl Into<String>,
        scenario_digest: impl Into<String>,
        probability_weight: f64,
        values: Vec<f64>,
        evidence_id: impl Into<String>,
    ) -> Result<Self, JointScenarioForecastError> {
        let scenario = Self {
            scenario_id: scenario_id.into(),
            scenario_digest: scenario_digest.into(),
            probability_weight,
            values,
            evidence_id: evidence_id.into(),
        };
        require_id("scenario ID", &scenario.scenario_id)?;
        require_id("scenario digest", &scenario.scenario_digest)?;
        require_id("scenario evidence ID", &scenario.evidence_id)?;
        if !scenario.probability_weight.is_finite()
            || scenario.probability_weight <= 0.0
            || scenario.probability_weight > 1.0
        {
            return Err(JointScenarioForecastError::InvalidProbabilityWeight(
                scenario.probability_weight,
            ));
        }
        if scenario.values.is_empty() {
            return Err(JointScenarioForecastError::EmptyScenarioVector(
                scenario.scenario_id,
            ));
        }
        for value in &scenario.values {
            if !value.is_finite() {
                return Err(JointScenarioForecastError::NonFiniteValue {
                    field: "scenario path value",
                    value: *value,
                });
            }
        }
        Ok(scenario)
    }

    pub fn scenario_id(&self) -> &str {
        &self.scenario_id
    }

    pub const fn probability_weight(&self) -> f64 {
        self.probability_weight
    }

    pub fn values(&self) -> &[f64] {
        &self.values
    }
}

/// Realized complete path and evidence for every declared dimension.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RealizedScenarioPath {
    values: Vec<f64>,
    evidence_ids: Vec<String>,
}

impl RealizedScenarioPath {
    pub fn new(
        values: Vec<f64>,
        evidence_ids: Vec<String>,
    ) -> Result<Self, JointScenarioForecastError> {
        if values.is_empty() {
            return Err(JointScenarioForecastError::EmptyRealizedVector);
        }
        if values.len() != evidence_ids.len() {
            return Err(JointScenarioForecastError::RealizedEvidenceCountMismatch {
                values: values.len(),
                evidence_ids: evidence_ids.len(),
            });
        }
        for (index, value) in values.iter().enumerate() {
            if !value.is_finite() {
                return Err(JointScenarioForecastError::NonFiniteValue {
                    field: "realized path value",
                    value: *value,
                });
            }
            require_id("realized observation evidence ID", &evidence_ids[index])?;
        }
        Ok(Self {
            values,
            evidence_ids,
        })
    }

    pub fn values(&self) -> &[f64] {
        &self.values
    }

    pub fn evidence_ids(&self) -> &[String] {
        &self.evidence_ids
    }
}

/// One forecast vintage and its jointly sampled scenarios and realized path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JointScenarioForecastCase {
    case_id: String,
    issued_at_ns: u64,
    valid_start_ns: u64,
    valid_end_ns: u64,
    scenario_set_digest: String,
    scenarios: Vec<WeightedScenarioPath>,
    realization: RealizedScenarioPath,
}

impl JointScenarioForecastCase {
    pub fn new(
        case_id: impl Into<String>,
        issued_at_ns: u64,
        valid_start_ns: u64,
        valid_end_ns: u64,
        scenario_set_digest: impl Into<String>,
        mut scenarios: Vec<WeightedScenarioPath>,
        realization: RealizedScenarioPath,
    ) -> Result<Self, JointScenarioForecastError> {
        let case = Self {
            case_id: case_id.into(),
            issued_at_ns,
            valid_start_ns,
            valid_end_ns,
            scenario_set_digest: scenario_set_digest.into(),
            scenarios: {
                scenarios.sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
                scenarios
            },
            realization,
        };
        require_id("forecast case ID", &case.case_id)?;
        require_id("scenario-set digest", &case.scenario_set_digest)?;
        if case.valid_end_ns <= case.valid_start_ns {
            return Err(JointScenarioForecastError::InvalidCaseWindow {
                case_id: case.case_id,
                valid_start_ns: case.valid_start_ns,
                valid_end_ns: case.valid_end_ns,
            });
        }
        if case.issued_at_ns >= case.valid_start_ns {
            return Err(JointScenarioForecastError::ForecastIssuedAfterValidityStarted {
                case_id: case.case_id,
                issued_at_ns: case.issued_at_ns,
                valid_start_ns: case.valid_start_ns,
            });
        }
        if case.scenarios.is_empty() {
            return Err(JointScenarioForecastError::EmptyScenarioSet {
                case_id: case.case_id,
            });
        }
        let mut scenario_ids = BTreeSet::new();
        for scenario in &case.scenarios {
            if !scenario_ids.insert(scenario.scenario_id.clone()) {
                return Err(JointScenarioForecastError::DuplicateScenarioId {
                    case_id: case.case_id,
                    scenario_id: scenario.scenario_id.clone(),
                });
            }
        }
        Ok(case)
    }

    pub fn case_id(&self) -> &str {
        &self.case_id
    }

    pub const fn issued_at_ns(&self) -> u64 {
        self.issued_at_ns
    }

    pub const fn valid_start_ns(&self) -> u64 {
        self.valid_start_ns
    }

    pub const fn valid_end_ns(&self) -> u64 {
        self.valid_end_ns
    }

    pub fn scenarios(&self) -> &[WeightedScenarioPath] {
        &self.scenarios
    }
}

/// Frozen evaluation set over complete-path forecast cases.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JointScenarioForecastEvaluation {
    contract: JointScenarioForecastContract,
    cases: Vec<JointScenarioForecastCase>,
}

impl JointScenarioForecastEvaluation {
    pub fn new(
        contract: JointScenarioForecastContract,
        mut cases: Vec<JointScenarioForecastCase>,
    ) -> Result<Self, JointScenarioForecastError> {
        if cases.len() < contract.minimum_cases {
            return Err(JointScenarioForecastError::InsufficientCases {
                available: cases.len(),
                required: contract.minimum_cases,
            });
        }
        cases.sort_by(|left, right| {
            left.valid_start_ns
                .cmp(&right.valid_start_ns)
                .then_with(|| left.case_id.cmp(&right.case_id))
        });
        let mut case_ids = BTreeSet::new();
        for case in &cases {
            if !case_ids.insert(case.case_id.clone()) {
                return Err(JointScenarioForecastError::DuplicateCaseId(
                    case.case_id.clone(),
                ));
            }
            let dimension_count = contract.dimensions.len();
            if case.realization.values.len() != dimension_count {
                return Err(JointScenarioForecastError::DimensionCountMismatch {
                    case_id: case.case_id.clone(),
                    expected: dimension_count,
                    observed: case.realization.values.len(),
                });
            }
            for scenario in &case.scenarios {
                if scenario.values.len() != dimension_count {
                    return Err(JointScenarioForecastError::DimensionCountMismatch {
                        case_id: case.case_id.clone(),
                        expected: dimension_count,
                        observed: scenario.values.len(),
                    });
                }
            }
            let probability_sum = case
                .scenarios
                .iter()
                .map(|scenario| scenario.probability_weight)
                .sum::<f64>();
            if !probability_sum.is_finite() {
                return Err(JointScenarioForecastError::NumericalOverflow(
                    "scenario probability mass",
                ));
            }
            if (probability_sum - 1.0).abs() > contract.probability_sum_tolerance {
                return Err(JointScenarioForecastError::ProbabilityMassMismatch {
                    case_id: case.case_id.clone(),
                    observed: probability_sum,
                    tolerance: contract.probability_sum_tolerance,
                });
            }
        }
        for pair in cases.windows(2) {
            if pair[0].valid_end_ns > pair[1].valid_start_ns {
                return Err(JointScenarioForecastError::OverlappingCaseWindows {
                    earlier_case_id: pair[0].case_id.clone(),
                    later_case_id: pair[1].case_id.clone(),
                });
            }
        }
        Ok(Self { contract, cases })
    }

    pub fn contract(&self) -> &JointScenarioForecastContract {
        &self.contract
    }

    pub fn cases(&self) -> &[JointScenarioForecastCase] {
        &self.cases
    }
}

/// Raw and normalized probability weight preserved for one scenario path.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioPathProbabilityReceipt {
    pub scenario_id: String,
    pub scenario_digest: String,
    pub raw_probability_weight: f64,
    pub normalized_probability_weight: f64,
    pub scenario_evidence_id: String,
}

/// Marginal CRPS diagnostic for one forecast dimension.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioMarginalScoreReceipt {
    pub dimension_id: String,
    pub crps: f64,
}

/// Aggregate marginal CRPS for one forecast dimension across evaluation cases.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioDimensionScoreAggregate {
    pub dimension_id: String,
    pub mean_crps: f64,
}

/// One dimension pair's variogram forecast expectation and observed target.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioVariogramPairReceipt {
    pub left_dimension_id: String,
    pub right_dimension_id: String,
    pub pair_weight: f64,
    pub expected_absolute_difference_power_half: f64,
    pub observed_absolute_difference_power_half: f64,
    pub score_contribution: f64,
}

/// Forecast probability and realized result for one joint event.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioEventCaseReceipt {
    pub event_id: String,
    pub forecast_probability: f64,
    pub observed: bool,
    pub brier_score: f64,
}

/// Per-case scores and evidence-bearing path receipts.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JointScenarioForecastCaseReceipt {
    pub case_id: String,
    pub scenario_set_digest: String,
    pub probability_weight_sum: f64,
    pub energy_score: f64,
    pub variogram_score: f64,
    pub scenario_count: usize,
    pub scenario_probabilities: Vec<ScenarioPathProbabilityReceipt>,
    pub variogram_pairs: Vec<ScenarioVariogramPairReceipt>,
    pub marginal_crps: Vec<ScenarioMarginalScoreReceipt>,
    pub events: Vec<ScenarioEventCaseReceipt>,
    pub realization_evidence_ids: Vec<String>,
}

/// Aggregate forecast-event calibration over the held-out evaluation cases.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ScenarioEventAggregateReport {
    pub event_id: String,
    pub case_count: usize,
    pub mean_forecast_probability: f64,
    pub observed_frequency: f64,
    pub mean_brier_score: f64,
}

/// Aggregate multivariate probabilistic scenario evaluation report.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct JointScenarioForecastReport {
    pub benchmark_id: String,
    pub forecast_model_digest: String,
    pub evaluation_configuration_digest: String,
    pub dimensions: Vec<ScenarioDimensionScale>,
    pub variogram_pairs: Vec<ScenarioVariogramPair>,
    pub event_definitions: Vec<ScenarioEventDefinition>,
    pub marginal_crps_by_dimension: Vec<ScenarioDimensionScoreAggregate>,
    pub metric_convention_id: String,
    pub case_count: usize,
    pub total_scenario_count: usize,
    pub mean_energy_score: f64,
    pub mean_variogram_score: f64,
    pub mean_event_brier_score: f64,
    pub maximum_probability_mass_deviation: f64,
    pub event_aggregates: Vec<ScenarioEventAggregateReport>,
    pub case_receipts: Vec<JointScenarioForecastCaseReceipt>,
}

/// Malformed joint paths, event declarations, probabilities or score arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum JointScenarioForecastError {
    EmptyIdentifier(&'static str),
    NonFiniteValue {
        field: &'static str,
        value: f64,
    },
    InvalidNormalizationScale {
        dimension_id: String,
        value: f64,
    },
    InvalidProbabilityWeight(f64),
    InvalidProbabilityTolerance(f64),
    InvalidVariogramWeight(f64),
    EmptyDimensionSet,
    DuplicateDimension(String),
    EmptyVariogramPairs,
    InvalidVariogramPair {
        left: String,
        right: String,
    },
    UnknownVariogramDimension {
        left: String,
        right: String,
    },
    DuplicateVariogramPair {
        left: String,
        right: String,
    },
    EmptyEventSet,
    EmptyEventConditions(String),
    DuplicateEventDimension {
        event_id: String,
        dimension_id: String,
    },
    UnknownEventDimension {
        event_id: String,
        dimension_id: String,
    },
    DuplicateEventId(String),
    InvalidMinimumCases,
    EmptyScenarioVector(String),
    EmptyRealizedVector,
    RealizedEvidenceCountMismatch {
        values: usize,
        evidence_ids: usize,
    },
    InvalidCaseWindow {
        case_id: String,
        valid_start_ns: u64,
        valid_end_ns: u64,
    },
    ForecastIssuedAfterValidityStarted {
        case_id: String,
        issued_at_ns: u64,
        valid_start_ns: u64,
    },
    EmptyScenarioSet {
        case_id: String,
    },
    DuplicateScenarioId {
        case_id: String,
        scenario_id: String,
    },
    InsufficientCases {
        available: usize,
        required: usize,
    },
    DuplicateCaseId(String),
    DimensionCountMismatch {
        case_id: String,
        expected: usize,
        observed: usize,
    },
    ProbabilityMassMismatch {
        case_id: String,
        observed: f64,
        tolerance: f64,
    },
    OverlappingCaseWindows {
        earlier_case_id: String,
        later_case_id: String,
    },
    NumericalOverflow(&'static str),
}

impl fmt::Display for JointScenarioForecastError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(field) => write!(f, "{field} must not be empty"),
            Self::NonFiniteValue { field, value } => {
                write!(f, "{field} must be finite; got {value}")
            }
            Self::InvalidNormalizationScale {
                dimension_id,
                value,
            } => write!(
                f,
                "dimension {dimension_id} normalization scale must be positive and finite; got {value}"
            ),
            Self::InvalidProbabilityWeight(value) => {
                write!(f, "scenario probability weight must be in (0, 1]; got {value}")
            }
            Self::InvalidProbabilityTolerance(value) => {
                write!(f, "probability-sum tolerance is invalid; got {value}")
            }
            Self::InvalidVariogramWeight(value) => {
                write!(f, "variogram pair weight must be positive and finite; got {value}")
            }
            Self::EmptyDimensionSet => write!(f, "at least one forecast dimension is required"),
            Self::DuplicateDimension(id) => write!(f, "duplicate forecast dimension {id}"),
            Self::EmptyVariogramPairs => write!(f, "at least one variogram pair is required"),
            Self::InvalidVariogramPair { left, right } => {
                write!(f, "invalid variogram pair ({left}, {right})")
            }
            Self::UnknownVariogramDimension { left, right } => {
                write!(f, "variogram pair ({left}, {right}) references an undeclared dimension")
            }
            Self::DuplicateVariogramPair { left, right } => {
                write!(f, "duplicate variogram pair ({left}, {right})")
            }
            Self::EmptyEventSet => write!(f, "at least one event definition is required"),
            Self::EmptyEventConditions(id) => {
                write!(f, "event {id} requires at least one condition")
            }
            Self::DuplicateEventDimension {
                event_id,
                dimension_id,
            } => write!(
                f,
                "event {event_id} repeats dimension condition {dimension_id}"
            ),
            Self::UnknownEventDimension {
                event_id,
                dimension_id,
            } => write!(
                f,
                "event {event_id} references undeclared dimension {dimension_id}"
            ),
            Self::DuplicateEventId(id) => write!(f, "duplicate event ID {id}"),
            Self::InvalidMinimumCases => write!(f, "minimum evaluation case count must be positive"),
            Self::EmptyScenarioVector(id) => write!(f, "scenario {id} has an empty vector"),
            Self::EmptyRealizedVector => write!(f, "realized path is empty"),
            Self::RealizedEvidenceCountMismatch {
                values,
                evidence_ids,
            } => write!(
                f,
                "realized path has {values} values and {evidence_ids} evidence IDs"
            ),
            Self::InvalidCaseWindow {
                case_id,
                valid_start_ns,
                valid_end_ns,
            } => write!(
                f,
                "case {case_id} has invalid target window [{valid_start_ns}, {valid_end_ns})"
            ),
            Self::ForecastIssuedAfterValidityStarted {
                case_id,
                issued_at_ns,
                valid_start_ns,
            } => write!(
                f,
                "case {case_id} issued at {issued_at_ns}, not before target start {valid_start_ns}"
            ),
            Self::EmptyScenarioSet { case_id } => {
                write!(f, "case {case_id} contains no scenarios")
            }
            Self::DuplicateScenarioId {
                case_id,
                scenario_id,
            } => write!(
                f,
                "case {case_id} duplicates scenario ID {scenario_id}"
            ),
            Self::InsufficientCases { available, required } => write!(
                f,
                "{available} forecast cases available; {required} required"
            ),
            Self::DuplicateCaseId(id) => write!(f, "duplicate forecast case ID {id}"),
            Self::DimensionCountMismatch {
                case_id,
                expected,
                observed,
            } => write!(
                f,
                "case {case_id} has {observed} dimensions; expected {expected}"
            ),
            Self::ProbabilityMassMismatch {
                case_id,
                observed,
                tolerance,
            } => write!(
                f,
                "case {case_id} scenario probability mass is {observed}; tolerance is {tolerance}"
            ),
            Self::OverlappingCaseWindows {
                earlier_case_id,
                later_case_id,
            } => write!(
                f,
                "forecast cases {earlier_case_id} and {later_case_id} overlap"
            ),
            Self::NumericalOverflow(operation) => {
                write!(f, "non-finite arithmetic during {operation}")
            }
        }
    }
}

impl std::error::Error for JointScenarioForecastError {}

/// Evaluate weighted multivariate scenarios against held-out realized paths.
pub fn evaluate_joint_scenario_forecasts(
    evaluation: &JointScenarioForecastEvaluation,
) -> Result<JointScenarioForecastReport, JointScenarioForecastError> {
    let contract = evaluation.contract();
    let dimension_count = contract.dimensions.len();
    let mut case_receipts = Vec::with_capacity(evaluation.cases.len());
    let mut total_scenario_count = 0_usize;
    let mut marginal_crps_sums = vec![0.0_f64; dimension_count];
    let mut energy_score_sum = 0.0_f64;
    let mut variogram_score_sum = 0.0_f64;
    let mut maximum_probability_mass_deviation = 0.0_f64;
    let mut event_probability_sums = vec![0.0_f64; contract.events.len()];
    let mut event_observed_counts = vec![0_usize; contract.events.len()];
    let mut event_brier_sums = vec![0.0_f64; contract.events.len()];
    let mut total_event_case_count = 0_usize;

    for case in &evaluation.cases {
        let probability_weight_sum = case
            .scenarios
            .iter()
            .map(|scenario| scenario.probability_weight)
            .sum::<f64>();
        if !probability_weight_sum.is_finite() || probability_weight_sum <= 0.0 {
            return Err(JointScenarioForecastError::NumericalOverflow(
                "scenario probability normalization",
            ));
        }
        let deviation = (probability_weight_sum - 1.0).abs();
        maximum_probability_mass_deviation =
            maximum_probability_mass_deviation.max(deviation);
        if deviation > contract.probability_sum_tolerance {
            return Err(JointScenarioForecastError::ProbabilityMassMismatch {
                case_id: case.case_id.clone(),
                observed: probability_weight_sum,
                tolerance: contract.probability_sum_tolerance,
            });
        }

        let normalized_weights = case
            .scenarios
            .iter()
            .map(|scenario| scenario.probability_weight / probability_weight_sum)
            .collect::<Vec<_>>();
        let mut scenario_probabilities = Vec::with_capacity(case.scenarios.len());
        for (scenario, normalized_weight) in
            case.scenarios.iter().zip(normalized_weights.iter())
        {
            scenario_probabilities.push(ScenarioPathProbabilityReceipt {
                scenario_id: scenario.scenario_id.clone(),
                scenario_digest: scenario.scenario_digest.clone(),
                raw_probability_weight: scenario.probability_weight,
                normalized_probability_weight: *normalized_weight,
                scenario_evidence_id: scenario.evidence_id.clone(),
            });
        }

        let energy_score = weighted_energy_score(
            &case.scenarios,
            &case.realization.values,
            &contract.dimensions,
            &normalized_weights,
        )?;
        let (variogram_score, variogram_pairs) = weighted_variogram_score(
            &case.scenarios,
            &case.realization.values,
            &contract.dimensions,
            &contract.variogram_pairs,
            &normalized_weights,
        )?;
        let mut marginal_crps = Vec::with_capacity(dimension_count);
        for dimension_index in 0..dimension_count {
            let crps = weighted_marginal_crps(
                &case.scenarios,
                case.realization.values[dimension_index],
                &contract.dimensions[dimension_index],
                dimension_index,
                &normalized_weights,
            )?;
            marginal_crps_sums[dimension_index] += crps;
            if !marginal_crps_sums[dimension_index].is_finite() {
                return Err(JointScenarioForecastError::NumericalOverflow(
                    "aggregate marginal CRPS",
                ));
            }
            marginal_crps.push(ScenarioMarginalScoreReceipt {
                dimension_id: contract.dimensions[dimension_index].dimension_id.clone(),
                crps,
            });
        }

        let mut event_receipts = Vec::with_capacity(contract.events.len());
        for (event_index, event) in contract.events.iter().enumerate() {
            let forecast_probability = forecast_event_probability(
                event,
                &case.scenarios,
                &contract.dimensions,
                &normalized_weights,
            )?;
            let observed = event_matches(
                event,
                &case.realization.values,
                &contract.dimensions,
            )?;
            let observed_value = if observed { 1.0 } else { 0.0 };
            let brier_score = (forecast_probability - observed_value).powi(2);
            if !brier_score.is_finite() {
                return Err(JointScenarioForecastError::NumericalOverflow(
                    "event Brier score",
                ));
            }
            event_probability_sums[event_index] += forecast_probability;
            event_brier_sums[event_index] += brier_score;
            if observed {
                event_observed_counts[event_index] += 1;
            }
            total_event_case_count += 1;
            event_receipts.push(ScenarioEventCaseReceipt {
                event_id: event.event_id.clone(),
                forecast_probability,
                observed,
                brier_score,
            });
        }

        total_scenario_count = total_scenario_count
            .checked_add(case.scenarios.len())
            .ok_or(JointScenarioForecastError::NumericalOverflow(
                "total scenario count",
            ))?;
        energy_score_sum += energy_score;
        variogram_score_sum += variogram_score;
        if !energy_score_sum.is_finite() || !variogram_score_sum.is_finite() {
            return Err(JointScenarioForecastError::NumericalOverflow(
                "aggregate multivariate proper scores",
            ));
        }
        case_receipts.push(JointScenarioForecastCaseReceipt {
            case_id: case.case_id.clone(),
            scenario_set_digest: case.scenario_set_digest.clone(),
            probability_weight_sum,
            energy_score,
            variogram_score,
            scenario_count: case.scenarios.len(),
            scenario_probabilities,
            variogram_pairs,
            marginal_crps,
            events: event_receipts,
            realization_evidence_ids: case.realization.evidence_ids.clone(),
        });
    }

    let case_count = evaluation.cases.len();
    if event_probability_sums.iter().any(|value| !value.is_finite())
        || event_brier_sums.iter().any(|value| !value.is_finite())
    {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "event aggregate sums",
        ));
    }
    let event_case_denominator = case_count as f64;
    let event_aggregates = contract
        .events
        .iter()
        .enumerate()
        .map(|(index, event)| ScenarioEventAggregateReport {
            event_id: event.event_id.clone(),
            case_count,
            mean_forecast_probability: event_probability_sums[index] / event_case_denominator,
            observed_frequency: event_observed_counts[index] as f64 / event_case_denominator,
            mean_brier_score: event_brier_sums[index] / event_case_denominator,
        })
        .collect::<Vec<_>>();
    let mean_event_brier_score = if total_event_case_count == 0 {
        0.0
    } else {
        event_brier_sums.iter().sum::<f64>() / total_event_case_count as f64
    };
    if !mean_event_brier_score.is_finite() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "mean event Brier score",
        ));
    }

    let mean_energy_score = energy_score_sum / case_count as f64;
    let mean_variogram_score = variogram_score_sum / case_count as f64;
    if !mean_energy_score.is_finite() || !mean_variogram_score.is_finite() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "mean multivariate proper scores",
        ));
    }

    Ok(JointScenarioForecastReport {
        benchmark_id: contract.benchmark_id.clone(),
        forecast_model_digest: contract.forecast_model_digest.clone(),
        evaluation_configuration_digest: contract.evaluation_configuration_digest.clone(),
        dimensions: contract.dimensions.clone(),
        variogram_pairs: contract.variogram_pairs.clone(),
        event_definitions: contract.events.clone(),
        marginal_crps_by_dimension: contract
            .dimensions
            .iter()
            .enumerate()
            .map(|(index, dimension)| ScenarioDimensionScoreAggregate {
                dimension_id: dimension.dimension_id.clone(),
                mean_crps: marginal_crps_sums[index] / case_count as f64,
            })
            .collect(),
        metric_convention_id: "SYMTROPY-JOINT-SCENARIO-ENERGY-VARIOGRAM05-BRIER-V0.1"
            .to_string(),
        case_count,
        total_scenario_count,
        mean_energy_score,
        mean_variogram_score,
        mean_event_brier_score,
        maximum_probability_mass_deviation,
        event_aggregates,
        case_receipts,
    })
}

/// Weighted multivariate Energy Score using dimension-normalized Euclidean
/// distance and the standard within-ensemble dispersion correction.
///
/// The distance is sqrt(mean_d(((x_d - y_d) / scale_d)^2)). Dividing by the
/// square root of dimension count is a fixed positive rescaling of Euclidean
/// distance; it makes scores comparable across the declared vector width.
fn weighted_energy_score(
    scenarios: &[WeightedScenarioPath],
    observed: &[f64],
    dimensions: &[ScenarioDimensionScale],
    normalized_weights: &[f64],
) -> Result<f64, JointScenarioForecastError> {
    if scenarios.len() != normalized_weights.len() || observed.len() != dimensions.len() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "energy score input dimensions",
        ));
    }
    let mut first_term = 0.0_f64;
    for (scenario, weight) in scenarios.iter().zip(normalized_weights.iter()) {
        let distance = normalized_path_distance(
            &scenario.values,
            observed,
            dimensions,
        )?;
        first_term += *weight * distance;
    }
    let mut dispersion_term = 0.0_f64;
    for (left_index, left) in scenarios.iter().enumerate() {
        for (right_index, right) in scenarios.iter().enumerate() {
            let distance = normalized_path_distance(
                &left.values,
                &right.values,
                dimensions,
            )?;
            dispersion_term +=
                normalized_weights[left_index] * normalized_weights[right_index] * distance;
        }
    }
    let score = first_term - 0.5 * dispersion_term;
    if !score.is_finite() || score < -1e-10 {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "weighted energy score",
        ));
    }
    Ok(score.max(0.0))
}

/// Weighted univariate CRPS for one dimension, retained alongside joint scores.
fn weighted_marginal_crps(
    scenarios: &[WeightedScenarioPath],
    observed: f64,
    dimension: &ScenarioDimensionScale,
    dimension_index: usize,
    normalized_weights: &[f64],
) -> Result<f64, JointScenarioForecastError> {
    if scenarios.len() != normalized_weights.len() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "marginal CRPS input dimensions",
        ));
    }
    let observed_normalized = observed / dimension.normalization_scale;
    let mut first_term = 0.0_f64;
    for (scenario, weight) in scenarios.iter().zip(normalized_weights.iter()) {
        let value = scenario.values[dimension_index] / dimension.normalization_scale;
        first_term += *weight * (value - observed_normalized).abs();
    }
    let mut dispersion_term = 0.0_f64;
    for (left_index, left) in scenarios.iter().enumerate() {
        for (right_index, right) in scenarios.iter().enumerate() {
            let left_value = left.values[dimension_index] / dimension.normalization_scale;
            let right_value = right.values[dimension_index] / dimension.normalization_scale;
            dispersion_term += normalized_weights[left_index]
                * normalized_weights[right_index]
                * (left_value - right_value).abs();
        }
    }
    let score = first_term - 0.5 * dispersion_term;
    if !score.is_finite() || score < -1e-10 {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "weighted marginal CRPS",
        ));
    }
    Ok(score.max(0.0))
}

/// Variogram Score with p = 0.5 for caller-selected dimension pairs.
///
/// Both endpoints are transformed by their own frozen normalization scales.
/// Pair weights are caller-pinned; the reported score is their weighted sum.
fn weighted_variogram_score(
    scenarios: &[WeightedScenarioPath],
    observed: &[f64],
    dimensions: &[ScenarioDimensionScale],
    pairs: &[ScenarioVariogramPair],
    normalized_weights: &[f64],
) -> Result<(f64, Vec<ScenarioVariogramPairReceipt>), JointScenarioForecastError> {
    let mut total_score = 0.0_f64;
    let mut receipts = Vec::with_capacity(pairs.len());
    for pair in pairs {
        let left_index = dimension_index(dimensions, &pair.left_dimension_id)?;
        let right_index = dimension_index(dimensions, &pair.right_dimension_id)?;
        let observed_difference = (observed[left_index]
            / dimensions[left_index].normalization_scale
            - observed[right_index] / dimensions[right_index].normalization_scale)
            .abs()
            .sqrt();
        let mut expected_difference = 0.0_f64;
        for (scenario, weight) in scenarios.iter().zip(normalized_weights.iter()) {
            let difference = (scenario.values[left_index]
                / dimensions[left_index].normalization_scale
                - scenario.values[right_index] / dimensions[right_index].normalization_scale)
                .abs()
                .sqrt();
            expected_difference += *weight * difference;
        }
        let residual = observed_difference - expected_difference;
        let contribution = pair.weight * residual * residual;
        if !observed_difference.is_finite()
            || !expected_difference.is_finite()
            || !contribution.is_finite()
        {
            return Err(JointScenarioForecastError::NumericalOverflow(
                "variogram score",
            ));
        }
        total_score += contribution;
        receipts.push(ScenarioVariogramPairReceipt {
            left_dimension_id: pair.left_dimension_id.clone(),
            right_dimension_id: pair.right_dimension_id.clone(),
            pair_weight: pair.weight,
            expected_absolute_difference_power_half: expected_difference,
            observed_absolute_difference_power_half: observed_difference,
            score_contribution: contribution,
        });
    }
    if !total_score.is_finite() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "total variogram score",
        ));
    }
    Ok((total_score, receipts))
}

fn normalized_path_distance(
    left: &[f64],
    right: &[f64],
    dimensions: &[ScenarioDimensionScale],
) -> Result<f64, JointScenarioForecastError> {
    if left.len() != dimensions.len() || right.len() != dimensions.len() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "normalized path distance dimensions",
        ));
    }
    let mut sum_squares = 0.0_f64;
    for index in 0..dimensions.len() {
        let normalized_difference =
            (left[index] - right[index]) / dimensions[index].normalization_scale;
        sum_squares += normalized_difference * normalized_difference;
        if !normalized_difference.is_finite() || !sum_squares.is_finite() {
            return Err(JointScenarioForecastError::NumericalOverflow(
                "normalized path distance",
            ));
        }
    }
    let mean_square = sum_squares / dimensions.len() as f64;
    let distance = mean_square.sqrt();
    if !distance.is_finite() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "normalized path distance norm",
        ));
    }
    Ok(distance)
}

fn forecast_event_probability(
    event: &ScenarioEventDefinition,
    scenarios: &[WeightedScenarioPath],
    dimensions: &[ScenarioDimensionScale],
    normalized_weights: &[f64],
) -> Result<f64, JointScenarioForecastError> {
    let mut probability = 0.0_f64;
    for (scenario, weight) in scenarios.iter().zip(normalized_weights.iter()) {
        if event_matches(event, &scenario.values, dimensions)? {
            probability += *weight;
        }
    }
    if !probability.is_finite() {
        return Err(JointScenarioForecastError::NumericalOverflow(
            "joint event forecast probability",
        ));
    }
    Ok(probability)
}

fn event_matches(
    event: &ScenarioEventDefinition,
    values: &[f64],
    dimensions: &[ScenarioDimensionScale],
) -> Result<bool, JointScenarioForecastError> {
    for condition in &event.conditions {
        let index = dimension_index(dimensions, &condition.dimension_id)?;
        let value = *values
            .get(index)
            .ok_or(JointScenarioForecastError::NumericalOverflow(
                "event path dimension",
            ))?;
        let matched = match condition.operator {
            ScenarioConditionOperator::LessThan => value < condition.threshold,
            ScenarioConditionOperator::LessThanOrEqual => value <= condition.threshold,
            ScenarioConditionOperator::GreaterThan => value > condition.threshold,
            ScenarioConditionOperator::GreaterThanOrEqual => value >= condition.threshold,
        };
        if !matched {
            return Ok(false);
        }
    }
    Ok(true)
}

fn dimension_index(
    dimensions: &[ScenarioDimensionScale],
    dimension_id: &str,
) -> Result<usize, JointScenarioForecastError> {
    dimensions
        .iter()
        .position(|dimension| dimension.dimension_id == dimension_id)
        .ok_or_else(|| JointScenarioForecastError::UnknownEventDimension {
            event_id: "metric".to_string(),
            dimension_id: dimension_id.to_string(),
        })
}

fn require_id(
    field: &'static str,
    value: &str,
) -> Result<(), JointScenarioForecastError> {
    if value.trim().is_empty() {
        return Err(JointScenarioForecastError::EmptyIdentifier(field));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dimensions() -> Vec<ScenarioDimensionScale> {
        vec![
            ScenarioDimensionScale::new("renewable", 1.0, "scale-renewable").unwrap(),
            ScenarioDimensionScale::new("workload", 1.0, "scale-workload").unwrap(),
        ]
    }

    fn event() -> ScenarioEventDefinition {
        ScenarioEventDefinition::new(
            "joint-scarcity",
            vec![
                ScenarioEventCondition::new(
                    "renewable",
                    ScenarioConditionOperator::LessThan,
                    1.0,
                )
                .unwrap(),
                ScenarioEventCondition::new(
                    "workload",
                    ScenarioConditionOperator::GreaterThan,
                    9.0,
                )
                .unwrap(),
            ],
        )
        .unwrap()
    }

    fn contract() -> JointScenarioForecastContract {
        JointScenarioForecastContract::new(
            "joint-benchmark",
            "model-digest",
            "evaluation-config-digest",
            dimensions(),
            vec![ScenarioVariogramPair::new("renewable", "workload", 1.0).unwrap()],
            vec![event()],
            1e-9,
            1,
        )
        .unwrap()
    }

    fn scenario(id: &str, weight: f64, values: [f64; 2]) -> WeightedScenarioPath {
        WeightedScenarioPath::new(
            id,
            format!("digest-{id}"),
            weight,
            values.to_vec(),
            format!("scenario-evidence-{id}"),
        )
        .unwrap()
    }

    fn realized(values: [f64; 2]) -> RealizedScenarioPath {
        RealizedScenarioPath::new(
            values.to_vec(),
            vec!["actual-renewable".to_string(), "actual-workload".to_string()],
        )
        .unwrap()
    }

    fn case_at(
        id: &str,
        start: u64,
        scenarios: Vec<WeightedScenarioPath>,
        actual: [f64; 2],
    ) -> JointScenarioForecastCase {
        JointScenarioForecastCase::new(
            id,
            start - 10,
            start,
            start + 10,
            format!("joint-set-{id}"),
            scenarios,
            realized(actual),
        )
        .unwrap()
    }

    fn case(
        id: &str,
        scenarios: Vec<WeightedScenarioPath>,
        actual: [f64; 2],
    ) -> JointScenarioForecastCase {
        case_at(id, 100, scenarios, actual)
    }

    #[test]
    fn energy_score_variogram_and_joint_event_penalize_broken_dependence() {
        // Both ensembles have identical marginals {0, 10}, but their joint
        // paths differ. The first preserves the observed scarcity co-occurrence.
        let dependent = case(
            "dependent",
            vec![scenario("s1", 0.5, [0.0, 10.0]), scenario("s2", 0.5, [10.0, 0.0])],
            [0.0, 10.0],
        );
        let broken = case(
            "broken",
            vec![scenario("s1", 0.5, [0.0, 0.0]), scenario("s2", 0.5, [10.0, 10.0])],
            [0.0, 10.0],
        );
        let dep_report =
            evaluate_joint_scenario_forecasts(&JointScenarioForecastEvaluation::new(
                contract(),
                vec![dependent],
            ).unwrap()).unwrap();
        let broken_report =
            evaluate_joint_scenario_forecasts(&JointScenarioForecastEvaluation::new(
                contract(),
                vec![broken],
            ).unwrap()).unwrap();

        assert!((dep_report.mean_energy_score - 2.5).abs() < 1e-12);
        assert!(broken_report.mean_energy_score > dep_report.mean_energy_score);
        assert_eq!(
            dep_report.marginal_crps_by_dimension,
            broken_report.marginal_crps_by_dimension
        );
        assert!(dep_report.mean_variogram_score < 1e-12);
        assert!((broken_report.mean_variogram_score - 10.0).abs() < 1e-12);
        assert!((dep_report.event_aggregates[0].mean_forecast_probability - 0.5).abs() < 1e-12);
        assert!((dep_report.event_aggregates[0].mean_brier_score - 0.25).abs() < 1e-12);
        assert!((broken_report.event_aggregates[0].mean_forecast_probability).abs() < 1e-12);
        assert!((broken_report.event_aggregates[0].mean_brier_score - 1.0).abs() < 1e-12);
    }

    #[test]
    fn normalizes_tolerated_probability_mass_drift_and_preserves_both_weights() {
        let input = JointScenarioForecastEvaluation::new(
            contract(),
            vec![case(
                "drift",
                vec![
                    scenario("s1", 0.5, [0.0, 10.0]),
                    scenario("s2", 0.499_999_999_5, [10.0, 0.0]),
                ],
                [0.0, 10.0],
            )],
        )
        .unwrap();
        let report = evaluate_joint_scenario_forecasts(&input).unwrap();
        let receipts = &report.case_receipts[0].scenario_probabilities;
        assert!((report.case_receipts[0].probability_weight_sum - 0.999_999_999_5).abs() < 1e-15);
        assert!((receipts[0].raw_probability_weight - 0.5).abs() < 1e-15);
        assert!((receipts[1].raw_probability_weight - 0.499_999_999_5).abs() < 1e-15);
        assert!(
            (receipts[0].normalized_probability_weight - 0.500_000_000_25).abs() < 1e-15
        );
        assert!(
            (receipts[1].normalized_probability_weight - 0.499_999_999_75).abs() < 1e-15
        );
        let normalized_sum: f64 = receipts
            .iter()
            .map(|receipt| receipt.normalized_probability_weight)
            .sum();
        assert!((normalized_sum - 1.0).abs() < 1e-15);
        assert_eq!(receipts.len(), 2);
    }

    #[test]
    fn rejects_probability_mass_outside_tolerance() {
        let result = JointScenarioForecastEvaluation::new(
            contract(),
            vec![case(
                "bad-mass",
                vec![scenario("s1", 0.8, [0.0, 10.0]), scenario("s2", 0.1, [10.0, 0.0])],
                [0.0, 10.0],
            )],
        );
        assert!(matches!(
            result,
            Err(JointScenarioForecastError::ProbabilityMassMismatch { .. })
        ));
    }

    #[test]
    fn rejects_incomplete_vectors_and_missing_evidence() {
        assert!(matches!(
            WeightedScenarioPath::new("s", "digest", 1.0, vec![0.0], ""),
            Err(JointScenarioForecastError::EmptyIdentifier(_))
        ));
        let bad_realized = RealizedScenarioPath::new(
            vec![0.0, 1.0],
            vec!["only-one-evidence".to_string()],
        );
        assert!(matches!(
            bad_realized,
            Err(JointScenarioForecastError::RealizedEvidenceCountMismatch {
                ..
            })
        ));
        let bad_scenario = WeightedScenarioPath::new(
            "bad",
            "digest",
            1.0,
            vec![0.0],
            "evidence",
        )
        .unwrap();
        let result = JointScenarioForecastEvaluation::new(
            contract(),
            vec![case(
                "bad-dimensions",
                vec![bad_scenario],
                [0.0, 10.0],
            )],
        );
        assert!(matches!(
            result,
            Err(JointScenarioForecastError::DimensionCountMismatch { .. })
        ));
    }

    #[test]
    fn rejects_duplicate_scenario_case_and_dimension_identities() {
        assert!(matches!(
            ScenarioDimensionScale::new("renewable", f64::NAN, "scale"),
            Err(JointScenarioForecastError::InvalidNormalizationScale { .. })
        ));
        assert!(matches!(
            JointScenarioForecastCase::new(
                "duplicate-scenarios",
                90,
                100,
                110,
                "digest",
                vec![
                    scenario("same", 0.5, [0.0, 10.0]),
                    scenario("same", 0.5, [10.0, 0.0])
                ],
                realized([0.0, 10.0])
            ),
            Err(JointScenarioForecastError::DuplicateScenarioId { .. })
        ));
        let duplicate_dimensions = vec![
            ScenarioDimensionScale::new("x", 1.0, "scale-x").unwrap(),
            ScenarioDimensionScale::new("x", 1.0, "scale-x2").unwrap(),
        ];
        assert!(matches!(
            JointScenarioForecastContract::new(
                "b",
                "m",
                "config",
                duplicate_dimensions,
                vec![ScenarioVariogramPair::new("x", "y", 1.0).unwrap()],
                vec![event()],
                1e-9,
                1,
            ),
            Err(JointScenarioForecastError::DuplicateDimension(_))
        ));
    }

    #[test]
    fn rejects_duplicate_case_identity_and_unknown_event_dimensions() {
        let duplicate_case = JointScenarioForecastEvaluation::new(
            contract(),
            vec![
                case("reused", vec![scenario("s1", 1.0, [0.0, 10.0])], [0.0, 10.0]),
                case_at(
                    "reused",
                    120,
                    vec![scenario("s2", 1.0, [10.0, 0.0])],
                    [10.0, 0.0],
                ),
            ],
        );
        assert!(matches!(
            duplicate_case,
            Err(JointScenarioForecastError::DuplicateCaseId(_))
        ));

        let unknown_event = ScenarioEventDefinition::new(
            "unknown-dimension-event",
            vec![ScenarioEventCondition::new(
                "unregistered-channel",
                ScenarioConditionOperator::GreaterThan,
                0.0,
            )
            .unwrap()],
        )
        .unwrap();
        let result = JointScenarioForecastContract::new(
            "benchmark",
            "model",
            "config",
            dimensions(),
            vec![ScenarioVariogramPair::new("renewable", "workload", 1.0).unwrap()],
            vec![unknown_event],
            1e-9,
            1,
        );
        assert!(matches!(
            result,
            Err(JointScenarioForecastError::UnknownEventDimension { .. })
        ));
    }

    #[test]
    fn rejects_forecast_vintage_and_overlapping_case_windows() {
        let late = JointScenarioForecastCase::new(
            "late",
            100,
            100,
            110,
            "set",
            vec![scenario("s", 1.0, [0.0, 10.0])],
            realized([0.0, 10.0]),
        );
        assert!(matches!(
            late,
            Err(JointScenarioForecastError::ForecastIssuedAfterValidityStarted { .. })
        ));
        let evaluation = JointScenarioForecastEvaluation::new(
            contract(),
            vec![
                case("case1", vec![scenario("s", 1.0, [0.0, 10.0])], [0.0, 10.0]),
                JointScenarioForecastCase::new(
                    "case2",
                    104,
                    105,
                    115,
                    "set2",
                    vec![scenario("s", 1.0, [0.0, 10.0])],
                    realized([0.0, 10.0]),
                )
                .unwrap(),
            ],
        );
        assert!(matches!(
            evaluation,
            Err(JointScenarioForecastError::OverlappingCaseWindows { .. })
        ));
    }

    #[test]
    fn permuting_scenario_and_case_input_order_is_deterministic() {
        let first = case(
            "case-a",
            vec![scenario("s2", 0.5, [10.0, 0.0]), scenario("s1", 0.5, [0.0, 10.0])],
            [0.0, 10.0],
        );
        let second = case_at(
            "case-b",
            120,
            vec![scenario("s1", 0.5, [0.0, 10.0]), scenario("s2", 0.5, [10.0, 0.0])],
            [10.0, 0.0],
        );
        let a = JointScenarioForecastEvaluation::new(contract(), vec![first.clone(), second.clone()]).unwrap();
        let b = JointScenarioForecastEvaluation::new(contract(), vec![second, first]).unwrap();
        let report_a = evaluate_joint_scenario_forecasts(&a).unwrap();
        let report_b = evaluate_joint_scenario_forecasts(&b).unwrap();
        assert_eq!(report_a, report_b);
    }

    #[test]
    fn reports_event_probabilities_and_brier_scores_over_multiple_cases() {
        let input = JointScenarioForecastEvaluation::new(
            contract(),
            vec![
                case(
                    "event-case",
                    vec![scenario("s1", 0.25, [0.0, 10.0]), scenario("s2", 0.75, [10.0, 0.0])],
                    [0.0, 10.0],
                ),
                case_at(
                    "non-event-case",
                    120,
                    vec![scenario("s1", 0.25, [0.0, 10.0]), scenario("s2", 0.75, [10.0, 0.0])],
                    [10.0, 0.0],
                ),
            ],
        )
        .unwrap();
        let report = evaluate_joint_scenario_forecasts(&input).unwrap();
        let event = &report.event_aggregates[0];
        assert!((event.mean_forecast_probability - 0.25).abs() < 1e-12);
        assert!((event.observed_frequency - 0.5).abs() < 1e-12);
        assert!((event.mean_brier_score - 0.3125).abs() < 1e-12);
    }

    #[test]
    fn rejects_invalid_event_and_variogram_definitions() {
        assert!(ScenarioVariogramPair::new("x", "x", 1.0).is_err());
        assert!(ScenarioVariogramPair::new("x", "y", 0.0).is_err());
        assert!(ScenarioEventDefinition::new("empty", vec![]).is_err());
        assert!(ScenarioEventDefinition::new(
            "duplicate-condition",
            vec![
                ScenarioEventCondition::new(
                    "renewable",
                    ScenarioConditionOperator::LessThan,
                    1.0,
                )
                .unwrap(),
                ScenarioEventCondition::new(
                    "renewable",
                    ScenarioConditionOperator::GreaterThan,
                    9.0,
                )
                .unwrap(),
            ],
        )
        .is_err());
    }

    #[test]
    fn rejects_empty_case_set_and_non_finite_forecast_values() {
        assert!(matches!(
            JointScenarioForecastEvaluation::new(contract(), vec![]),
            Err(JointScenarioForecastError::InsufficientCases { .. })
        ));
        assert!(matches!(
            WeightedScenarioPath::new(
                "nan-scenario",
                "digest",
                1.0,
                vec![f64::INFINITY, 0.0],
                "evidence",
            ),
            Err(JointScenarioForecastError::NonFiniteValue { .. })
        ));
    }

    #[test]
    fn energy_and_variogram_scores_are_nonnegative_for_a_known_fixture() {
        let input = JointScenarioForecastEvaluation::new(
            contract(),
            vec![case(
                "known",
                vec![
                    scenario("s1", 0.25, [0.0, 10.0]),
                    scenario("s2", 0.25, [10.0, 0.0]),
                    scenario("s3", 0.5, [2.0, 8.0]),
                ],
                [0.0, 10.0],
            )],
        )
        .unwrap();
        let report = evaluate_joint_scenario_forecasts(&input).unwrap();
        assert!(report.mean_energy_score >= 0.0);
        assert!(report.mean_variogram_score >= 0.0);
        assert!(report.mean_event_brier_score >= 0.0);
        assert_eq!(report.case_receipts[0].scenario_probabilities.len(), 3);
        assert_eq!(report.case_receipts[0].variogram_pairs.len(), 1);
        assert_eq!(report.case_receipts[0].marginal_crps.len(), 2);
        assert_eq!(report.marginal_crps_by_dimension.len(), 2);
        assert_eq!(
            report.case_receipts[0].realization_evidence_ids,
            vec!["actual-renewable".to_string(), "actual-workload".to_string()]
        );
        assert_eq!(report.evaluation_configuration_digest, "evaluation-config-digest");
        assert_eq!(report.dimensions.len(), 2);
    }
}
