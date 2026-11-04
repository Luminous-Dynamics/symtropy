// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Evidence-bounded measurement and verification for adjusted energy differences.
//!
//! This module verifies a frozen M&V calculation. It does not fit a baseline
//! model, establish causal identification, authenticate meters, or certify a
//! physical energy system. Positive savings are supported only when the
//! declared holdout-fit gates pass and the conservative lower bound is > 0.

use core::fmt;
use std::collections::BTreeSet;

use serde::Serialize;

/// M&V approach declared by the frozen project plan.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EnergyMvMethod {
    /// Isolated equipment or system measurement boundary.
    RetrofitIsolation,
    /// Whole-facility measurement boundary.
    WholeFacility,
    /// Calibrated simulation used where a measured baseline is unavailable.
    CalibratedSimulation,
}

/// Declares which operating-condition basis the baseline-model output represents.
///
/// For `BaselinePeriodConditions`, a separately evidenced routine adjustment
/// may restate the prediction to reporting-period conditions. For
/// `ReportingPeriodConditions`, routine adjustment must be zero because the
/// baseline prediction already contains that normalization. This guard prevents
/// applying the same routine adjustment twice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum EnergyMvBaselinePredictionBasis {
    /// Frozen baseline output before routine adjustment to reporting conditions.
    BaselinePeriodConditions,
    /// Frozen baseline output already evaluated at reporting-period conditions.
    ReportingPeriodConditions,
}

/// Frozen identity and caller-declared acceptance limits for an M&V plan.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyMvPlan {
    plan_id: String,
    project_id: String,
    site_id: String,
    boundary_id: String,
    method: EnergyMvMethod,
    prediction_basis: EnergyMvBaselinePredictionBasis,
    baseline_digest: String,
    model_digest: String,
    minimum_validation_points: usize,
    maximum_absolute_nmbe_percent: f64,
    maximum_cvrmse_percent: f64,
}

impl EnergyMvPlan {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        plan_id: impl Into<String>,
        project_id: impl Into<String>,
        site_id: impl Into<String>,
        boundary_id: impl Into<String>,
        method: EnergyMvMethod,
        prediction_basis: EnergyMvBaselinePredictionBasis,
        baseline_digest: impl Into<String>,
        model_digest: impl Into<String>,
        minimum_validation_points: usize,
        maximum_absolute_nmbe_percent: f64,
        maximum_cvrmse_percent: f64,
    ) -> Result<Self, EnergyMvError> {
        let plan = Self {
            plan_id: plan_id.into(),
            project_id: project_id.into(),
            site_id: site_id.into(),
            boundary_id: boundary_id.into(),
            method,
            prediction_basis,
            baseline_digest: baseline_digest.into(),
            model_digest: model_digest.into(),
            minimum_validation_points,
            maximum_absolute_nmbe_percent,
            maximum_cvrmse_percent,
        };
        for (name, value) in [
            ("plan ID", &plan.plan_id),
            ("project ID", &plan.project_id),
            ("site ID", &plan.site_id),
            ("boundary ID", &plan.boundary_id),
            ("baseline digest", &plan.baseline_digest),
            ("model digest", &plan.model_digest),
        ] {
            require_id(name, value)?;
        }
        if plan.minimum_validation_points < 2 {
            return Err(EnergyMvError::InvalidPlan(
                "minimum validation points must be at least two",
            ));
        }
        for (name, value) in [
            (
                "maximum absolute NMBE percent",
                plan.maximum_absolute_nmbe_percent,
            ),
            ("maximum CV(RMSE) percent", plan.maximum_cvrmse_percent),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(EnergyMvError::InvalidQuantity { name, value });
            }
        }
        Ok(plan)
    }

    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub fn project_id(&self) -> &str {
        &self.project_id
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn boundary_id(&self) -> &str {
        &self.boundary_id
    }

    pub const fn method(&self) -> EnergyMvMethod {
        self.method
    }

    pub const fn prediction_basis(&self) -> EnergyMvBaselinePredictionBasis {
        self.prediction_basis
    }

    pub fn baseline_digest(&self) -> &str {
        &self.baseline_digest
    }

    pub fn model_digest(&self) -> &str {
        &self.model_digest
    }
}

/// One held-out baseline observation and its frozen-model prediction.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyMvValidationPoint {
    sample_id: String,
    site_id: String,
    boundary_id: String,
    start_unix_seconds: i64,
    end_unix_seconds: i64,
    observed_energy_kwh: f64,
    predicted_baseline_energy_kwh: f64,
    baseline_digest: String,
    model_digest: String,
    conditions_digest: String,
    measurement_evidence_id: String,
    prediction_evidence_id: String,
}

impl EnergyMvValidationPoint {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sample_id: impl Into<String>,
        site_id: impl Into<String>,
        boundary_id: impl Into<String>,
        start_unix_seconds: i64,
        end_unix_seconds: i64,
        observed_energy_kwh: f64,
        predicted_baseline_energy_kwh: f64,
        baseline_digest: impl Into<String>,
        model_digest: impl Into<String>,
        conditions_digest: impl Into<String>,
        measurement_evidence_id: impl Into<String>,
        prediction_evidence_id: impl Into<String>,
    ) -> Result<Self, EnergyMvError> {
        let point = Self {
            sample_id: sample_id.into(),
            site_id: site_id.into(),
            boundary_id: boundary_id.into(),
            start_unix_seconds,
            end_unix_seconds,
            observed_energy_kwh,
            predicted_baseline_energy_kwh,
            baseline_digest: baseline_digest.into(),
            model_digest: model_digest.into(),
            conditions_digest: conditions_digest.into(),
            measurement_evidence_id: measurement_evidence_id.into(),
            prediction_evidence_id: prediction_evidence_id.into(),
        };
        point.validate()?;
        Ok(point)
    }

    fn validate(&self) -> Result<(), EnergyMvError> {
        validate_window(&self.sample_id, self.start_unix_seconds, self.end_unix_seconds)?;
        for (name, value) in [
            ("sample ID", &self.sample_id),
            ("site ID", &self.site_id),
            ("boundary ID", &self.boundary_id),
            ("baseline digest", &self.baseline_digest),
            ("model digest", &self.model_digest),
            ("conditions digest", &self.conditions_digest),
            ("measurement evidence ID", &self.measurement_evidence_id),
            ("prediction evidence ID", &self.prediction_evidence_id),
        ] {
            require_id(name, value)?;
        }
        require_non_negative("observed energy", self.observed_energy_kwh)?;
        require_non_negative("predicted baseline energy", self.predicted_baseline_energy_kwh)?;
        if self.observed_energy_kwh == 0.0 {
            return Err(EnergyMvError::InvalidQuantity {
                name: "observed validation energy must be positive",
                value: self.observed_energy_kwh,
            });
        }
        Ok(())
    }

    pub fn sample_id(&self) -> &str {
        &self.sample_id
    }

    pub const fn start_unix_seconds(&self) -> i64 {
        self.start_unix_seconds
    }

    pub const fn end_unix_seconds(&self) -> i64 {
        self.end_unix_seconds
    }
}

/// One reporting-period observation, counterfactual prediction and adjustment set.
///
/// Adjustment signs are explicit: positive values increase the counterfactual
/// baseline; negative values decrease it. Uncertainty fields are deterministic
/// absolute upper bounds in kWh, not standard deviations or confidence intervals.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyMvReportingPoint {
    sample_id: String,
    site_id: String,
    boundary_id: String,
    start_unix_seconds: i64,
    end_unix_seconds: i64,
    observed_energy_kwh: f64,
    predicted_baseline_energy_kwh: f64,
    routine_adjustment_kwh: f64,
    routine_adjustment_evidence_id: Option<String>,
    non_routine_adjustment_kwh: f64,
    non_routine_adjustment_evidence_id: Option<String>,
    baseline_uncertainty_bound_kwh: f64,
    measurement_uncertainty_bound_kwh: f64,
    routine_adjustment_uncertainty_bound_kwh: f64,
    non_routine_adjustment_uncertainty_bound_kwh: f64,
    baseline_digest: String,
    model_digest: String,
    conditions_digest: String,
    measurement_evidence_id: String,
    prediction_evidence_id: String,
}

impl EnergyMvReportingPoint {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sample_id: impl Into<String>,
        site_id: impl Into<String>,
        boundary_id: impl Into<String>,
        start_unix_seconds: i64,
        end_unix_seconds: i64,
        observed_energy_kwh: f64,
        predicted_baseline_energy_kwh: f64,
        routine_adjustment_kwh: f64,
        routine_adjustment_evidence_id: Option<String>,
        non_routine_adjustment_kwh: f64,
        non_routine_adjustment_evidence_id: Option<String>,
        baseline_uncertainty_bound_kwh: f64,
        measurement_uncertainty_bound_kwh: f64,
        routine_adjustment_uncertainty_bound_kwh: f64,
        non_routine_adjustment_uncertainty_bound_kwh: f64,
        baseline_digest: impl Into<String>,
        model_digest: impl Into<String>,
        conditions_digest: impl Into<String>,
        measurement_evidence_id: impl Into<String>,
        prediction_evidence_id: impl Into<String>,
    ) -> Result<Self, EnergyMvError> {
        let point = Self {
            sample_id: sample_id.into(),
            site_id: site_id.into(),
            boundary_id: boundary_id.into(),
            start_unix_seconds,
            end_unix_seconds,
            observed_energy_kwh,
            predicted_baseline_energy_kwh,
            routine_adjustment_kwh,
            routine_adjustment_evidence_id,
            non_routine_adjustment_kwh,
            non_routine_adjustment_evidence_id,
            baseline_uncertainty_bound_kwh,
            measurement_uncertainty_bound_kwh,
            routine_adjustment_uncertainty_bound_kwh,
            non_routine_adjustment_uncertainty_bound_kwh,
            baseline_digest: baseline_digest.into(),
            model_digest: model_digest.into(),
            conditions_digest: conditions_digest.into(),
            measurement_evidence_id: measurement_evidence_id.into(),
            prediction_evidence_id: prediction_evidence_id.into(),
        };
        point.validate()?;
        Ok(point)
    }

    fn validate(&self) -> Result<(), EnergyMvError> {
        validate_window(&self.sample_id, self.start_unix_seconds, self.end_unix_seconds)?;
        for (name, value) in [
            ("sample ID", &self.sample_id),
            ("site ID", &self.site_id),
            ("boundary ID", &self.boundary_id),
            ("baseline digest", &self.baseline_digest),
            ("model digest", &self.model_digest),
            ("conditions digest", &self.conditions_digest),
            ("measurement evidence ID", &self.measurement_evidence_id),
            ("prediction evidence ID", &self.prediction_evidence_id),
        ] {
            require_id(name, value)?;
        }
        require_non_negative("observed energy", self.observed_energy_kwh)?;
        require_non_negative("predicted baseline energy", self.predicted_baseline_energy_kwh)?;
        if self.observed_energy_kwh == 0.0 {
            return Err(EnergyMvError::InvalidQuantity {
                name: "observed reporting energy must be positive",
                value: self.observed_energy_kwh,
            });
        }
        for (name, value) in [
            ("routine adjustment", self.routine_adjustment_kwh),
            ("non-routine adjustment", self.non_routine_adjustment_kwh),
        ] {
            if !value.is_finite() {
                return Err(EnergyMvError::InvalidQuantity { name, value });
            }
        }
        for (name, value) in [
            ("baseline uncertainty bound", self.baseline_uncertainty_bound_kwh),
            ("measurement uncertainty bound", self.measurement_uncertainty_bound_kwh),
            ("routine adjustment uncertainty bound", self.routine_adjustment_uncertainty_bound_kwh),
            ("non-routine adjustment uncertainty bound", self.non_routine_adjustment_uncertainty_bound_kwh),
        ] {
            require_non_negative(name, value)?;
        }
        if self.routine_adjustment_kwh != 0.0
            && self.routine_adjustment_evidence_id.as_deref().is_none_or(str::is_empty)
        {
            return Err(EnergyMvError::MissingEvidence {
                sample_id: self.sample_id.clone(),
                evidence_kind: "routine adjustment",
            });
        }
        if self.routine_adjustment_kwh == 0.0 {
            if let Some(id) = &self.routine_adjustment_evidence_id {
                require_id("routine adjustment evidence ID", id)?;
            }
        }
        if self.non_routine_adjustment_kwh != 0.0
            && self.non_routine_adjustment_evidence_id.as_deref().is_none_or(str::is_empty)
        {
            return Err(EnergyMvError::MissingEvidence {
                sample_id: self.sample_id.clone(),
                evidence_kind: "non-routine adjustment",
            });
        }
        if self.non_routine_adjustment_kwh == 0.0 {
            if let Some(id) = &self.non_routine_adjustment_evidence_id {
                require_id("non-routine adjustment evidence ID", id)?;
            }
        }
        Ok(())
    }

    pub fn sample_id(&self) -> &str {
        &self.sample_id
    }

    pub const fn start_unix_seconds(&self) -> i64 {
        self.start_unix_seconds
    }

    pub const fn end_unix_seconds(&self) -> i64 {
        self.end_unix_seconds
    }
}

/// Frozen input data for one independently reconstructed M&V calculation.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyMvDataset {
    validation_points: Vec<EnergyMvValidationPoint>,
    reporting_points: Vec<EnergyMvReportingPoint>,
}

impl EnergyMvDataset {
    pub fn new(
        validation_points: Vec<EnergyMvValidationPoint>,
        reporting_points: Vec<EnergyMvReportingPoint>,
    ) -> Result<Self, EnergyMvError> {
        if validation_points.is_empty() || reporting_points.is_empty() {
            return Err(EnergyMvError::EmptyDataset);
        }
        Ok(Self {
            validation_points,
            reporting_points,
        })
    }
}

/// Errors indicating malformed evidence, invalid windows or unsafe arithmetic.
#[derive(Debug, Clone, PartialEq)]
pub enum EnergyMvError {
    EmptyIdentifier(&'static str),
    InvalidQuantity { name: &'static str, value: f64 },
    InvalidPlan(&'static str),
    EmptyDataset,
    DuplicateSampleId(String),
    OverlappingWindows { earlier: String, later: String },
    PostTreatmentValidationLeakage { sample_id: String },
    SiteMismatch { sample_id: String },
    BoundaryMismatch { sample_id: String },
    BaselineDigestMismatch { sample_id: String },
    ModelDigestMismatch { sample_id: String },
    MissingEvidence { sample_id: String, evidence_kind: &'static str },
    RoutineAdjustmentAlreadyApplied { sample_id: String },
    NegativeAdjustedBaseline { sample_id: String, adjusted_kwh: f64 },
    NumericalOverflow(&'static str),
    TooFewValidationPoints { minimum: usize, observed: usize },
}

impl fmt::Display for EnergyMvError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentifier(name) => write!(f, "{name} must not be empty"),
            Self::InvalidQuantity { name, value } => {
                write!(f, "{name} must be finite and valid; got {value}")
            }
            Self::InvalidPlan(message) => write!(f, "invalid M&V plan: {message}"),
            Self::EmptyDataset => write!(f, "M&V dataset requires validation and reporting points"),
            Self::DuplicateSampleId(id) => write!(f, "duplicate M&V sample ID: {id}"),
            Self::OverlappingWindows { earlier, later } => {
                write!(f, "M&V windows overlap: {earlier} and {later}")
            }
            Self::PostTreatmentValidationLeakage { sample_id } => write!(
                f,
                "holdout validation sample {sample_id} is not wholly before reporting"
            ),
            Self::SiteMismatch { sample_id } => write!(f, "M&V sample {sample_id} has a different site ID"),
            Self::BoundaryMismatch { sample_id } => write!(f, "M&V sample {sample_id} has a different measurement boundary"),
            Self::BaselineDigestMismatch { sample_id } => write!(f, "M&V sample {sample_id} has a different baseline digest"),
            Self::ModelDigestMismatch { sample_id } => write!(f, "M&V sample {sample_id} has a different model digest"),
            Self::MissingEvidence { sample_id, evidence_kind } => {
                write!(f, "M&V sample {sample_id} is missing {evidence_kind} evidence")
            }
            Self::RoutineAdjustmentAlreadyApplied { sample_id } => write!(
                f,
                "M&V sample {sample_id} cannot add a routine adjustment when the prediction already uses reporting-period conditions"
            ),
            Self::NegativeAdjustedBaseline { sample_id, adjusted_kwh } => write!(
                f,
                "M&V sample {sample_id} has negative adjusted baseline energy: {adjusted_kwh} kWh"
            ),
            Self::NumericalOverflow(name) => write!(f, "non-finite M&V arithmetic: {name}"),
            Self::TooFewValidationPoints { minimum, observed } => write!(
                f,
                "M&V plan requires {minimum} validation points; observed {observed}"
            ),
        }
    }
}

impl std::error::Error for EnergyMvError {}

/// Retained evidence and exact measurement window for a held-out validation point.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyMvValidationReceipt {
    pub sample_id: String,
    pub site_id: String,
    pub boundary_id: String,
    pub start_unix_seconds: i64,
    pub end_unix_seconds: i64,
    pub observed_energy_kwh: f64,
    pub predicted_baseline_energy_kwh: f64,
    pub residual_kwh: f64,
    pub baseline_digest: String,
    pub model_digest: String,
    pub conditions_digest: String,
    pub measurement_evidence_id: String,
    pub prediction_evidence_id: String,
}

/// Retained evidence, adjustments and interval arithmetic for a reporting point.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyMvReportingReceipt {
    pub sample_id: String,
    pub site_id: String,
    pub boundary_id: String,
    pub start_unix_seconds: i64,
    pub end_unix_seconds: i64,
    pub observed_energy_kwh: f64,
    pub predicted_baseline_energy_kwh: f64,
    pub routine_adjustment_kwh: f64,
    pub routine_adjustment_evidence_id: Option<String>,
    pub non_routine_adjustment_kwh: f64,
    pub non_routine_adjustment_evidence_id: Option<String>,
    pub adjusted_baseline_energy_kwh: f64,
    pub adjusted_energy_difference_kwh: f64,
    pub baseline_uncertainty_bound_kwh: f64,
    pub measurement_uncertainty_bound_kwh: f64,
    pub routine_adjustment_uncertainty_bound_kwh: f64,
    pub non_routine_adjustment_uncertainty_bound_kwh: f64,
    pub uncertainty_bound_kwh: f64,
    pub baseline_digest: String,
    pub model_digest: String,
    pub conditions_digest: String,
    pub measurement_evidence_id: String,
    pub prediction_evidence_id: String,
}

/// Independent M&V verification report.
///
/// The uncertainty interval is a conservative sum of declared absolute kWh
/// bounds. It is not a probabilistic confidence interval. verification_passed
/// means the declared baseline-fit limits passed. A positive-savings claim is
/// separately supported only when the lower bound is strictly greater than zero.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EnergyMvVerificationReport {
    plan_id: String,
    project_id: String,
    site_id: String,
    boundary_id: String,
    method: EnergyMvMethod,
    prediction_basis: EnergyMvBaselinePredictionBasis,
    baseline_digest: String,
    model_digest: String,
    validation_sample_ids: Vec<String>,
    reporting_sample_ids: Vec<String>,
    validation_receipts: Vec<EnergyMvValidationReceipt>,
    reporting_receipts: Vec<EnergyMvReportingReceipt>,
    validation_point_count: usize,
    reporting_point_count: usize,
    normalized_mean_bias_error_percent: f64,
    coefficient_of_variation_rmse_percent: f64,
    absolute_nmbe_limit_percent: f64,
    cvrmse_limit_percent: f64,
    adjusted_baseline_energy_kwh: f64,
    measured_reporting_energy_kwh: f64,
    routine_adjustment_energy_kwh: f64,
    non_routine_adjustment_energy_kwh: f64,
    adjusted_energy_difference_kwh: f64,
    conservative_uncertainty_bound_kwh: f64,
    savings_lower_bound_kwh: f64,
    savings_upper_bound_kwh: f64,
    savings_percent_of_adjusted_baseline: Option<f64>,
    model_fit_passed: bool,
    verification_passed: bool,
    positive_savings_claim_supported: bool,
}

impl EnergyMvVerificationReport {
    pub fn plan_id(&self) -> &str {
        &self.plan_id
    }

    pub fn validation_sample_ids(&self) -> &[String] {
        &self.validation_sample_ids
    }

    pub fn reporting_sample_ids(&self) -> &[String] {
        &self.reporting_sample_ids
    }

    pub fn validation_receipts(&self) -> &[EnergyMvValidationReceipt] {
        &self.validation_receipts
    }

    pub fn reporting_receipts(&self) -> &[EnergyMvReportingReceipt] {
        &self.reporting_receipts
    }

    pub const fn validation_point_count(&self) -> usize {
        self.validation_point_count
    }

    pub const fn reporting_point_count(&self) -> usize {
        self.reporting_point_count
    }

    pub const fn adjusted_baseline_energy_kwh(&self) -> f64 {
        self.adjusted_baseline_energy_kwh
    }

    pub const fn measured_reporting_energy_kwh(&self) -> f64 {
        self.measured_reporting_energy_kwh
    }

    pub const fn routine_adjustment_energy_kwh(&self) -> f64 {
        self.routine_adjustment_energy_kwh
    }

    pub const fn non_routine_adjustment_energy_kwh(&self) -> f64 {
        self.non_routine_adjustment_energy_kwh
    }

    pub const fn savings_percent_of_adjusted_baseline(&self) -> Option<f64> {
        self.savings_percent_of_adjusted_baseline
    }

    pub const fn normalized_mean_bias_error_percent(&self) -> f64 {
        self.normalized_mean_bias_error_percent
    }

    pub const fn coefficient_of_variation_rmse_percent(&self) -> f64 {
        self.coefficient_of_variation_rmse_percent
    }

    pub const fn adjusted_energy_difference_kwh(&self) -> f64 {
        self.adjusted_energy_difference_kwh
    }

    pub const fn conservative_uncertainty_bound_kwh(&self) -> f64 {
        self.conservative_uncertainty_bound_kwh
    }

    pub const fn savings_lower_bound_kwh(&self) -> f64 {
        self.savings_lower_bound_kwh
    }

    pub const fn savings_upper_bound_kwh(&self) -> f64 {
        self.savings_upper_bound_kwh
    }

    pub const fn model_fit_passed(&self) -> bool {
        self.model_fit_passed
    }

    pub const fn verification_passed(&self) -> bool {
        self.verification_passed
    }

    pub const fn positive_savings_claim_supported(&self) -> bool {
        self.positive_savings_claim_supported
    }
}

/// Reconstruct baseline fit, adjusted energy difference and conservative bound.
///
/// This v0.1 verifier pins the following explicit descriptive conventions:
/// NMBE = 100 * sum(predicted - observed) / sum(observed), and
/// CV(RMSE) = 100 * sqrt(mean(residual^2)) / mean(observed), using the
/// population mean-square residual (denominator n). These diagnostics are
/// independently recomputed from holdout observations; they are not asserted
/// to be a universal standards formula or a causal-identification test.
///
/// A positive routine adjustment restates a baseline-period prediction upward
/// to reporting conditions. If the prediction is already evaluated at reporting
/// conditions, the plan prohibits a further routine adjustment. A positive
/// non-routine adjustment increases counterfactual baseline energy. All energy
/// values are in kWh.
pub fn verify_energy_mv(
    plan: &EnergyMvPlan,
    dataset: &EnergyMvDataset,
) -> Result<EnergyMvVerificationReport, EnergyMvError> {
    if dataset.validation_points.len() < plan.minimum_validation_points {
        return Err(EnergyMvError::TooFewValidationPoints {
            minimum: plan.minimum_validation_points,
            observed: dataset.validation_points.len(),
        });
    }

    let mut ids = BTreeSet::new();
    for point in &dataset.validation_points {
        point.validate()?;
        validate_identity(
            plan, &point.sample_id, &point.site_id, &point.boundary_id,
            &point.baseline_digest, &point.model_digest, &point.conditions_digest,
            &point.measurement_evidence_id, &point.prediction_evidence_id,
        )?;
        if !ids.insert(point.sample_id.clone()) {
            return Err(EnergyMvError::DuplicateSampleId(point.sample_id.clone()));
        }
    }
    for point in &dataset.reporting_points {
        point.validate()?;
        validate_identity(
            plan, &point.sample_id, &point.site_id, &point.boundary_id,
            &point.baseline_digest, &point.model_digest, &point.conditions_digest,
            &point.measurement_evidence_id, &point.prediction_evidence_id,
        )?;
        if !ids.insert(point.sample_id.clone()) {
            return Err(EnergyMvError::DuplicateSampleId(point.sample_id.clone()));
        }
    }

    validate_non_overlapping_validation(&dataset.validation_points)?;
    validate_non_overlapping_reporting(&dataset.reporting_points)?;
    let first_reporting_start = dataset
        .reporting_points
        .iter()
        .map(|point| point.start_unix_seconds)
        .min()
        .ok_or(EnergyMvError::EmptyDataset)?;
    for point in &dataset.validation_points {
        if point.end_unix_seconds > first_reporting_start {
            return Err(EnergyMvError::PostTreatmentValidationLeakage {
                sample_id: point.sample_id.clone(),
            });
        }
    }

    let observed_total = sum_finite(
        dataset.validation_points.iter().map(|point| point.observed_energy_kwh),
        "validation observed energy",
    )?;
    let residuals: Vec<f64> = dataset
        .validation_points
        .iter()
        .map(|point| point.predicted_baseline_energy_kwh - point.observed_energy_kwh)
        .collect();
    if residuals.iter().any(|value| !value.is_finite()) {
        return Err(EnergyMvError::NumericalOverflow("validation residual"));
    }
    let residual_sum = sum_finite(residuals.iter().copied(), "validation residual sum")?;
    let squared_residual_sum = sum_finite(
        residuals.iter().map(|value| value * value),
        "squared validation residual sum",
    )?;
    let count = dataset.validation_points.len() as f64;
    let mean_observed = observed_total / count;
    let nmbe = 100.0 * residual_sum / observed_total;
    let cvrmse = 100.0 * (squared_residual_sum / count).sqrt() / mean_observed;
    if !nmbe.is_finite() || !cvrmse.is_finite() {
        return Err(EnergyMvError::NumericalOverflow("validation metrics"));
    }

    let mut adjusted_baseline_total = 0.0;
    let mut measured_reporting_total = 0.0;
    let mut routine_total = 0.0;
    let mut non_routine_total = 0.0;
    let mut uncertainty_total = 0.0;
    for point in &dataset.reporting_points {
        if plan.prediction_basis == EnergyMvBaselinePredictionBasis::ReportingPeriodConditions
            && point.routine_adjustment_kwh != 0.0
        {
            return Err(EnergyMvError::RoutineAdjustmentAlreadyApplied {
                sample_id: point.sample_id.clone(),
            });
        }
        if point.routine_adjustment_kwh != 0.0
            && point.routine_adjustment_evidence_id.as_deref().is_none_or(|id| id.trim().is_empty())
        {
            return Err(EnergyMvError::MissingEvidence {
                sample_id: point.sample_id.clone(),
                evidence_kind: "routine adjustment",
            });
        }
        if point.non_routine_adjustment_kwh != 0.0
            && point.non_routine_adjustment_evidence_id.as_deref().is_none_or(|id| id.trim().is_empty())
        {
            return Err(EnergyMvError::MissingEvidence {
                sample_id: point.sample_id.clone(),
                evidence_kind: "non-routine adjustment",
            });
        }
        let adjusted = point.predicted_baseline_energy_kwh
            + point.routine_adjustment_kwh
            + point.non_routine_adjustment_kwh;
        if !adjusted.is_finite() {
            return Err(EnergyMvError::NumericalOverflow("adjusted baseline energy"));
        }
        if adjusted < 0.0 {
            return Err(EnergyMvError::NegativeAdjustedBaseline {
                sample_id: point.sample_id.clone(),
                adjusted_kwh: adjusted,
            });
        }
        adjusted_baseline_total += adjusted;
        measured_reporting_total += point.observed_energy_kwh;
        routine_total += point.routine_adjustment_kwh;
        non_routine_total += point.non_routine_adjustment_kwh;
        uncertainty_total += point.baseline_uncertainty_bound_kwh
            + point.measurement_uncertainty_bound_kwh
            + point.routine_adjustment_uncertainty_bound_kwh
            + point.non_routine_adjustment_uncertainty_bound_kwh;
    }
    for (name, value) in [
        ("adjusted baseline total", adjusted_baseline_total),
        ("reporting energy total", measured_reporting_total),
        ("routine adjustment total", routine_total),
        ("non-routine adjustment total", non_routine_total),
        ("uncertainty bound total", uncertainty_total),
    ] {
        if !value.is_finite() {
            return Err(EnergyMvError::NumericalOverflow(name));
        }
    }

    let savings = adjusted_baseline_total - measured_reporting_total;
    let lower = savings - uncertainty_total;
    let upper = savings + uncertainty_total;
    let model_fit_passed = nmbe.abs() <= plan.maximum_absolute_nmbe_percent
        && cvrmse <= plan.maximum_cvrmse_percent;
    let verification_passed = model_fit_passed;
    let positive_savings_claim_supported = verification_passed && lower > 0.0;
    let savings_percent = if adjusted_baseline_total > 0.0 {
        Some(100.0 * savings / adjusted_baseline_total)
    } else {
        None
    };
    if !savings.is_finite() || !lower.is_finite() || !upper.is_finite()
        || savings_percent.is_some_and(|value| !value.is_finite())
    {
        return Err(EnergyMvError::NumericalOverflow("savings result"));
    }

    let validation_receipts = dataset.validation_points.iter().map(|point| {
        EnergyMvValidationReceipt {
            sample_id: point.sample_id.clone(),
            site_id: point.site_id.clone(),
            boundary_id: point.boundary_id.clone(),
            start_unix_seconds: point.start_unix_seconds,
            end_unix_seconds: point.end_unix_seconds,
            observed_energy_kwh: point.observed_energy_kwh,
            predicted_baseline_energy_kwh: point.predicted_baseline_energy_kwh,
            residual_kwh: point.predicted_baseline_energy_kwh - point.observed_energy_kwh,
            baseline_digest: point.baseline_digest.clone(),
            model_digest: point.model_digest.clone(),
            conditions_digest: point.conditions_digest.clone(),
            measurement_evidence_id: point.measurement_evidence_id.clone(),
            prediction_evidence_id: point.prediction_evidence_id.clone(),
        }
    }).collect();
    let reporting_receipts = dataset.reporting_points.iter().map(|point| {
        let adjusted_baseline = point.predicted_baseline_energy_kwh
            + point.routine_adjustment_kwh + point.non_routine_adjustment_kwh;
        let uncertainty = point.baseline_uncertainty_bound_kwh
            + point.measurement_uncertainty_bound_kwh
            + point.routine_adjustment_uncertainty_bound_kwh
            + point.non_routine_adjustment_uncertainty_bound_kwh;
        EnergyMvReportingReceipt {
            sample_id: point.sample_id.clone(),
            site_id: point.site_id.clone(),
            boundary_id: point.boundary_id.clone(),
            start_unix_seconds: point.start_unix_seconds,
            end_unix_seconds: point.end_unix_seconds,
            observed_energy_kwh: point.observed_energy_kwh,
            predicted_baseline_energy_kwh: point.predicted_baseline_energy_kwh,
            routine_adjustment_kwh: point.routine_adjustment_kwh,
            routine_adjustment_evidence_id: point.routine_adjustment_evidence_id.clone(),
            non_routine_adjustment_kwh: point.non_routine_adjustment_kwh,
            non_routine_adjustment_evidence_id: point.non_routine_adjustment_evidence_id.clone(),
            adjusted_baseline_energy_kwh: adjusted_baseline,
            adjusted_energy_difference_kwh: adjusted_baseline - point.observed_energy_kwh,
            baseline_uncertainty_bound_kwh: point.baseline_uncertainty_bound_kwh,
            measurement_uncertainty_bound_kwh: point.measurement_uncertainty_bound_kwh,
            routine_adjustment_uncertainty_bound_kwh: point.routine_adjustment_uncertainty_bound_kwh,
            non_routine_adjustment_uncertainty_bound_kwh: point.non_routine_adjustment_uncertainty_bound_kwh,
            uncertainty_bound_kwh: uncertainty,
            baseline_digest: point.baseline_digest.clone(),
            model_digest: point.model_digest.clone(),
            conditions_digest: point.conditions_digest.clone(),
            measurement_evidence_id: point.measurement_evidence_id.clone(),
            prediction_evidence_id: point.prediction_evidence_id.clone(),
        }
    }).collect();

    Ok(EnergyMvVerificationReport {
        plan_id: plan.plan_id.clone(),
        project_id: plan.project_id.clone(),
        site_id: plan.site_id.clone(),
        boundary_id: plan.boundary_id.clone(),
        method: plan.method,
        prediction_basis: plan.prediction_basis,
        baseline_digest: plan.baseline_digest.clone(),
        model_digest: plan.model_digest.clone(),
        validation_sample_ids: dataset.validation_points.iter().map(|point| point.sample_id.clone()).collect(),
        reporting_sample_ids: dataset.reporting_points.iter().map(|point| point.sample_id.clone()).collect(),
        validation_receipts,
        reporting_receipts,
        validation_point_count: dataset.validation_points.len(),
        reporting_point_count: dataset.reporting_points.len(),
        normalized_mean_bias_error_percent: nmbe,
        coefficient_of_variation_rmse_percent: cvrmse,
        absolute_nmbe_limit_percent: plan.maximum_absolute_nmbe_percent,
        cvrmse_limit_percent: plan.maximum_cvrmse_percent,
        adjusted_baseline_energy_kwh: adjusted_baseline_total,
        measured_reporting_energy_kwh: measured_reporting_total,
        routine_adjustment_energy_kwh: routine_total,
        non_routine_adjustment_energy_kwh: non_routine_total,
        adjusted_energy_difference_kwh: savings,
        conservative_uncertainty_bound_kwh: uncertainty_total,
        savings_lower_bound_kwh: lower,
        savings_upper_bound_kwh: upper,
        savings_percent_of_adjusted_baseline: savings_percent,
        model_fit_passed,
        verification_passed,
        positive_savings_claim_supported,
    })
}

#[allow(clippy::too_many_arguments)]
fn validate_identity(
    plan: &EnergyMvPlan,
    sample_id: &str,
    site_id: &str,
    boundary_id: &str,
    baseline_digest: &str,
    model_digest: &str,
    conditions_digest: &str,
    measurement_evidence_id: &str,
    prediction_evidence_id: &str,
) -> Result<(), EnergyMvError> {
    if site_id != plan.site_id {
        return Err(EnergyMvError::SiteMismatch { sample_id: sample_id.to_string() });
    }
    if boundary_id != plan.boundary_id {
        return Err(EnergyMvError::BoundaryMismatch { sample_id: sample_id.to_string() });
    }
    if baseline_digest != plan.baseline_digest {
        return Err(EnergyMvError::BaselineDigestMismatch { sample_id: sample_id.to_string() });
    }
    if model_digest != plan.model_digest {
        return Err(EnergyMvError::ModelDigestMismatch { sample_id: sample_id.to_string() });
    }
    for (kind, id) in [
        ("conditions", conditions_digest),
        ("measurement", measurement_evidence_id),
        ("prediction", prediction_evidence_id),
    ] {
        if id.trim().is_empty() {
            return Err(EnergyMvError::MissingEvidence {
                sample_id: sample_id.to_string(),
                evidence_kind: kind,
            });
        }
    }
    Ok(())
}

fn validate_non_overlapping_validation(
    points: &[EnergyMvValidationPoint],
) -> Result<(), EnergyMvError> {
    let mut windows: Vec<_> = points
        .iter()
        .map(|point| (point.start_unix_seconds, point.end_unix_seconds, point.sample_id.as_str()))
        .collect();
    windows.sort_by_key(|window| window.0);
    for pair in windows.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(EnergyMvError::OverlappingWindows {
                earlier: pair[0].2.to_string(),
                later: pair[1].2.to_string(),
            });
        }
    }
    Ok(())
}

fn validate_non_overlapping_reporting(
    points: &[EnergyMvReportingPoint],
) -> Result<(), EnergyMvError> {
    let mut windows: Vec<_> = points
        .iter()
        .map(|point| (point.start_unix_seconds, point.end_unix_seconds, point.sample_id.as_str()))
        .collect();
    windows.sort_by_key(|window| window.0);
    for pair in windows.windows(2) {
        if pair[0].1 > pair[1].0 {
            return Err(EnergyMvError::OverlappingWindows {
                earlier: pair[0].2.to_string(),
                later: pair[1].2.to_string(),
            });
        }
    }
    Ok(())
}

fn validate_window(sample_id: &str, start: i64, end: i64) -> Result<(), EnergyMvError> {
    if sample_id.trim().is_empty() {
        return Err(EnergyMvError::EmptyIdentifier("sample ID"));
    }
    if start >= end {
        return Err(EnergyMvError::InvalidPlan(
            "sample window must be a non-empty half-open interval",
        ));
    }
    Ok(())
}

fn require_id(name: &'static str, value: &str) -> Result<(), EnergyMvError> {
    if value.trim().is_empty() {
        return Err(EnergyMvError::EmptyIdentifier(name));
    }
    Ok(())
}

fn require_non_negative(name: &'static str, value: f64) -> Result<(), EnergyMvError> {
    if !value.is_finite() || value < 0.0 {
        return Err(EnergyMvError::InvalidQuantity { name, value });
    }
    Ok(())
}

fn sum_finite(
    values: impl Iterator<Item = f64>,
    name: &'static str,
) -> Result<f64, EnergyMvError> {
    let sum = values.sum::<f64>();
    if !sum.is_finite() {
        return Err(EnergyMvError::NumericalOverflow(name));
    }
    Ok(sum)
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASELINE: &str = "baseline-sha256";
    const MODEL: &str = "model-sha256";

    fn plan() -> EnergyMvPlan {
        EnergyMvPlan::new(
            "mv-plan-1", "project-1", "site-1", "facility-pcc",
            EnergyMvMethod::WholeFacility, EnergyMvBaselinePredictionBasis::BaselinePeriodConditions,
            BASELINE, MODEL, 3, 2.0, 5.0,
        ).unwrap()
    }

    fn validation_point(id: &str, start: i64, observed: f64, predicted: f64) -> EnergyMvValidationPoint {
        EnergyMvValidationPoint::new(
            id, "site-1", "facility-pcc", start, start + 3600, observed, predicted,
            BASELINE, MODEL, format!("conditions-{id}"), format!("meter-{id}"),
            format!("prediction-{id}"),
        ).unwrap()
    }

    #[allow(clippy::too_many_arguments)]
    fn reporting_point(
        id: &str,
        start: i64,
        observed: f64,
        predicted: f64,
        routine: f64,
        non_routine: f64,
        uncertainty: f64,
        routine_evidence: Option<String>,
        non_routine_evidence: Option<String>,
    ) -> EnergyMvReportingPoint {
        EnergyMvReportingPoint::new(
            id, "site-1", "facility-pcc", start, start + 3600, observed, predicted,
            routine, routine_evidence, non_routine, non_routine_evidence,
            uncertainty, uncertainty, 0.0, 0.0, BASELINE, MODEL,
            format!("conditions-{id}"), format!("meter-{id}"),
            format!("prediction-{id}"),
        ).unwrap()
    }

    fn valid_dataset() -> EnergyMvDataset {
        EnergyMvDataset::new(
            vec![
                validation_point("v1", 1_000, 100.0, 101.0),
                validation_point("v2", 5_000, 110.0, 109.0),
                validation_point("v3", 9_000, 90.0, 90.0),
            ],
            vec![
                reporting_point("r1", 20_000, 80.0, 100.0, 2.0, -1.0, 1.0,
                    Some("routine-evidence".into()), Some("nonroutine-evidence".into())),
                reporting_point("r2", 25_000, 70.0, 90.0, 0.0, 0.0, 1.0, None, None),
            ],
        ).unwrap()
    }

    #[test]
    fn reconstructs_holdout_fit_adjustments_and_conservative_savings_bound() {
        let report = verify_energy_mv(&plan(), &valid_dataset()).unwrap();
        assert!(report.verification_passed());
        assert!(report.positive_savings_claim_supported());
        assert!((report.adjusted_energy_difference_kwh() - 41.0).abs() < 1e-12);
        assert!((report.conservative_uncertainty_bound_kwh() - 4.0).abs() < 1e-12);
        assert!((report.savings_lower_bound_kwh() - 37.0).abs() < 1e-12);
        assert_eq!(report.validation_sample_ids().len(), 3);
        assert_eq!(report.reporting_sample_ids().len(), 2);
    }

    #[test]
    fn poor_holdout_fit_fails_without_hiding_reconstructed_result() {
        let mut data = valid_dataset();
        data.validation_points[0].predicted_baseline_energy_kwh = 150.0;
        let report = verify_energy_mv(&plan(), &data).unwrap();
        assert!(!report.model_fit_passed());
        assert!(!report.verification_passed());
        assert!(!report.positive_savings_claim_supported());
        assert!(report.adjusted_energy_difference_kwh() > 0.0);
    }

    #[test]
    fn uncertainty_can_suppress_savings_claim_without_erasing_arithmetic() {
        let mut data = valid_dataset();
        data.reporting_points[0].baseline_uncertainty_bound_kwh = 30.0;
        let report = verify_energy_mv(&plan(), &data).unwrap();
        assert!(report.verification_passed());
        assert!(!report.positive_savings_claim_supported());
        assert!(report.savings_lower_bound_kwh() < 0.0);
    }

    #[test]
    fn rejects_missing_evidence_for_nonzero_adjustment() {
        let mut data = valid_dataset();
        data.reporting_points[0].routine_adjustment_evidence_id = None;
        assert!(matches!(
            verify_energy_mv(&plan(), &data),
            Err(EnergyMvError::MissingEvidence { evidence_kind: "routine adjustment", .. })
        ));
    }

    #[test]
    fn rejects_cross_boundary_samples() {
        let mut data = valid_dataset();
        data.reporting_points[0].boundary_id = "ups-output".into();
        assert!(matches!(
            verify_energy_mv(&plan(), &data),
            Err(EnergyMvError::BoundaryMismatch { .. })
        ));
    }

    #[test]
    fn rejects_overlapping_reporting_windows() {
        let mut data = valid_dataset();
        data.reporting_points[1].start_unix_seconds = 21_000;
        data.reporting_points[1].end_unix_seconds = 26_000;
        assert!(matches!(
            verify_energy_mv(&plan(), &data),
            Err(EnergyMvError::OverlappingWindows { .. })
        ));
    }

    #[test]
    fn rejects_holdout_leakage_into_reporting_period() {
        let mut data = valid_dataset();
        data.validation_points[2].start_unix_seconds = 20_000;
        data.validation_points[2].end_unix_seconds = 23_000;
        assert!(matches!(
            verify_energy_mv(&plan(), &data),
            Err(EnergyMvError::PostTreatmentValidationLeakage { .. })
        ));
    }

    #[test]
    fn negative_adjustments_reduce_counterfactual_baseline() {
        let data = EnergyMvDataset::new(
            vec![
                validation_point("v1", 1_000, 100.0, 100.0),
                validation_point("v2", 5_000, 100.0, 100.0),
                validation_point("v3", 9_000, 100.0, 100.0),
            ],
            vec![reporting_point(
                "r1", 20_000, 95.0, 100.0, -3.0, -1.0, 1.0,
                Some("routine-negative".into()), Some("nonroutine-negative".into()),
            )],
        ).unwrap();
        let report = verify_energy_mv(&plan(), &data).unwrap();
        assert!((report.adjusted_energy_difference_kwh() - 1.0).abs() < 1e-12);
        assert!(!report.positive_savings_claim_supported());
    }

    #[test]
    fn no_savings_is_not_promoted_to_a_positive_claim() {
        let mut data = valid_dataset();
        data.reporting_points[0].observed_energy_kwh = 130.0;
        data.reporting_points[1].observed_energy_kwh = 130.0;
        let report = verify_energy_mv(&plan(), &data).unwrap();
        assert!(report.verification_passed());
        assert!(!report.positive_savings_claim_supported());
    }

    #[test]
    fn rejects_routine_adjustment_when_prediction_is_already_at_reporting_conditions() {
        let reporting_basis_plan = EnergyMvPlan::new(
            "mv-plan-reporting", "project-1", "site-1", "facility-pcc",
            EnergyMvMethod::WholeFacility,
            EnergyMvBaselinePredictionBasis::ReportingPeriodConditions,
            BASELINE, MODEL, 3, 2.0, 5.0,
        ).unwrap();
        assert!(matches!(
            verify_energy_mv(&reporting_basis_plan, &valid_dataset()),
            Err(EnergyMvError::RoutineAdjustmentAlreadyApplied { sample_id }) if sample_id == "r1"
        ));
    }

    #[test]
    fn rejects_whitespace_only_adjustment_evidence_id() {
        let mut data = valid_dataset();
        data.reporting_points[0].routine_adjustment_evidence_id = Some("   ".into());
        assert!(matches!(
            verify_energy_mv(&plan(), &data),
            Err(EnergyMvError::MissingEvidence {
                evidence_kind: "routine adjustment",
                ..
            })
        ));
    }

    #[test]
    fn rejects_duplicate_sample_ids_across_validation_and_reporting() {
        let data = EnergyMvDataset::new(
            vec![
                validation_point("duplicate", 1_000, 100.0, 100.0),
                validation_point("v2", 5_000, 100.0, 100.0),
                validation_point("v3", 9_000, 100.0, 100.0),
            ],
            vec![reporting_point(
                "duplicate", 20_000, 90.0, 100.0, 0.0, 0.0, 0.0, None, None,
            )],
        ).unwrap();
        assert!(matches!(
            verify_energy_mv(&plan(), &data),
            Err(EnergyMvError::DuplicateSampleId(id)) if id == "duplicate"
        ));
    }

}
