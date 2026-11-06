// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Qualification of counterfactual no-dispatch load baselines.
//!
//! A baseline is an unobserved counterfactual during dispatch. This crate
//! qualifies its one-sided lower prediction bound using disjoint no-dispatch
//! calibration and evaluation groups. It reports out-of-sample bias, normalized
//! error, lower-bound coverage and false-positive reduction rate by operating
//! regime. Only a qualified report can be used to evaluate an event.
//!
//! Event response is bounded as:
//!   lower baseline power − upper measured dispatched power
//! where the lower baseline is predicted power plus a one-sided residual
//! quantile calibrated on no-dispatch data, and the upper measured power adds
//! the PCC metrology expanded uncertainty. Signed rebound is preserved.
//!
//! This is a model-qualification and impact-estimation layer, not a causal
//! identification engine. Split-conformal-style residual quantiles require
//! exchangeability within the declared regime; the evaluation holdout tests
//! coverage but cannot prove future exchangeability. Randomized/crossover or
//! credible matched-control designs provide stronger causal evidence.

use core::fmt;
use std::collections::{BTreeMap, BTreeSet};

use serde::Serialize;

const NANOSECONDS_PER_HOUR: f64 = 3_600_000_000_000.0;
const BASIS_POINTS: f64 = 10_000.0;
const EPSILON: f64 = 1.0e-12;

/// Provenance for the baseline model and its disjoint data partitions.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaselineIdentity {
    site_id: String,
    pcc_id: String,
    baseline_model_id: String,
    feature_set_digest: String,
    training_data_digest: String,
    calibration_data_digest: String,
    evaluation_data_digest: String,
}

impl BaselineIdentity {
    pub fn new(
        site_id: impl Into<String>,
        pcc_id: impl Into<String>,
        baseline_model_id: impl Into<String>,
        feature_set_digest: impl Into<String>,
        training_data_digest: impl Into<String>,
        calibration_data_digest: impl Into<String>,
        evaluation_data_digest: impl Into<String>,
    ) -> Result<Self, BaselineQualificationError> {
        let identity = Self {
            site_id: site_id.into(),
            pcc_id: pcc_id.into(),
            baseline_model_id: baseline_model_id.into(),
            feature_set_digest: feature_set_digest.into(),
            training_data_digest: training_data_digest.into(),
            calibration_data_digest: calibration_data_digest.into(),
            evaluation_data_digest: evaluation_data_digest.into(),
        };
        for (field, value) in [
            ("site ID", identity.site_id.as_str()),
            ("PCC ID", identity.pcc_id.as_str()),
            ("baseline model ID", identity.baseline_model_id.as_str()),
            ("feature-set digest", identity.feature_set_digest.as_str()),
            (
                "training-data digest",
                identity.training_data_digest.as_str(),
            ),
            (
                "calibration-data digest",
                identity.calibration_data_digest.as_str(),
            ),
            (
                "evaluation-data digest",
                identity.evaluation_data_digest.as_str(),
            ),
        ] {
            require_id(field, value)?;
        }
        let partition_digests = [
            identity.training_data_digest.as_str(),
            identity.calibration_data_digest.as_str(),
            identity.evaluation_data_digest.as_str(),
        ];
        let unique_partition_digests: BTreeSet<&str> = partition_digests.into_iter().collect();
        if unique_partition_digests.len() != partition_digests.len() {
            return Err(BaselineQualificationError::PartitionDigestCollision);
        }
        Ok(identity)
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }

    pub fn pcc_id(&self) -> &str {
        &self.pcc_id
    }

    pub fn baseline_model_id(&self) -> &str {
        &self.baseline_model_id
    }
}

/// Disjoint no-dispatch data partitions used for residual calibration and audit.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum BaselineFold {
    Calibration,
    Evaluation,
}

/// One timestamped forecast and its observed no-dispatch outcome.
///
/// `forecast_issued_at_ns` must be no later than the target interval start.
/// The group ID is a complete independence unit (for example, site-day,
/// workload-session or experiment block), not an individual row.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct NoDispatchObservation {
    sample_id: String,
    group_id: String,
    regime_id: String,
    source_digest: String,
    forecast_issued_at_ns: u64,
    interval_start_ns: u64,
    interval_duration_ns: u64,
    predicted_no_dispatch_power_kw: f64,
    observed_no_dispatch_power_kw: f64,
    observed_expanded_uncertainty_kw: f64,
    fold: BaselineFold,
    dispatch_active: bool,
}

impl NoDispatchObservation {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        sample_id: impl Into<String>,
        group_id: impl Into<String>,
        regime_id: impl Into<String>,
        source_digest: impl Into<String>,
        forecast_issued_at_ns: u64,
        interval_start_ns: u64,
        interval_duration_ns: u64,
        predicted_no_dispatch_power_kw: f64,
        observed_no_dispatch_power_kw: f64,
        observed_expanded_uncertainty_kw: f64,
        fold: BaselineFold,
        dispatch_active: bool,
    ) -> Result<Self, BaselineQualificationError> {
        let observation = Self {
            sample_id: sample_id.into(),
            group_id: group_id.into(),
            regime_id: regime_id.into(),
            source_digest: source_digest.into(),
            forecast_issued_at_ns,
            interval_start_ns,
            interval_duration_ns,
            predicted_no_dispatch_power_kw,
            observed_no_dispatch_power_kw,
            observed_expanded_uncertainty_kw,
            fold,
            dispatch_active,
        };
        for (field, value) in [
            ("sample ID", observation.sample_id.as_str()),
            ("independence group ID", observation.group_id.as_str()),
            ("operating regime ID", observation.regime_id.as_str()),
            ("source digest", observation.source_digest.as_str()),
        ] {
            require_id(field, value)?;
        }
        if interval_duration_ns == 0 {
            return Err(BaselineQualificationError::InvalidIntervalDuration);
        }
        if forecast_issued_at_ns > interval_start_ns {
            return Err(BaselineQualificationError::FutureInformationLeakage {
                sample_id: observation.sample_id.clone(),
                forecast_issued_at_ns,
                interval_start_ns,
            });
        }
        if dispatch_active {
            return Err(BaselineQualificationError::DispatchedDataInBaselineFold {
                sample_id: observation.sample_id.clone(),
            });
        }
        for (field, value) in [
            (
                "predicted no-dispatch power",
                predicted_no_dispatch_power_kw,
            ),
            ("observed no-dispatch power", observed_no_dispatch_power_kw),
            (
                "observed no-dispatch expanded uncertainty",
                observed_expanded_uncertainty_kw,
            ),
        ] {
            if !value.is_finite() {
                return Err(BaselineQualificationError::InvalidQuantity { field, value });
            }
        }
        if observed_expanded_uncertainty_kw < 0.0 {
            return Err(BaselineQualificationError::InvalidQuantity {
                field: "observed no-dispatch expanded uncertainty",
                value: observed_expanded_uncertainty_kw,
            });
        }
        interval_start_ns
            .checked_add(interval_duration_ns)
            .ok_or(BaselineQualificationError::TimestampOverflow)?;
        Ok(observation)
    }
}

/// Qualification policy. Thresholds are caller-pre-registered values, not
/// universal standards or market rules.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaselineQualificationContract {
    residual_lower_tail_alpha_basis_points: u16,
    minimum_calibration_groups_per_regime: usize,
    minimum_evaluation_groups_per_regime: usize,
    maximum_absolute_normalized_bias_basis_points: u16,
    maximum_normalized_mae_basis_points: u16,
    minimum_lower_bound_coverage_basis_points: u16,
    maximum_false_positive_reduction_rate_basis_points: u16,
    minimum_response_kw: f64,
    minimum_event_qualifying_fraction_basis_points: u16,
}

impl BaselineQualificationContract {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        residual_lower_tail_alpha_basis_points: u16,
        minimum_calibration_groups_per_regime: usize,
        minimum_evaluation_groups_per_regime: usize,
        maximum_absolute_normalized_bias_basis_points: u16,
        maximum_normalized_mae_basis_points: u16,
        minimum_lower_bound_coverage_basis_points: u16,
        maximum_false_positive_reduction_rate_basis_points: u16,
        minimum_response_kw: f64,
        minimum_event_qualifying_fraction_basis_points: u16,
    ) -> Result<Self, BaselineQualificationError> {
        if !(1..=10_000).contains(&residual_lower_tail_alpha_basis_points) {
            return Err(BaselineQualificationError::InvalidBasisPoints {
                field: "residual lower-tail alpha",
                value: residual_lower_tail_alpha_basis_points,
            });
        }
        if minimum_calibration_groups_per_regime == 0 || minimum_evaluation_groups_per_regime == 0 {
            return Err(BaselineQualificationError::InvalidMinimumSampleCount);
        }
        let minimum_for_quantile = (10_000usize
            .div_ceil(residual_lower_tail_alpha_basis_points as usize))
        .saturating_sub(1);
        if minimum_calibration_groups_per_regime < minimum_for_quantile {
            return Err(BaselineQualificationError::InsufficientCalibrationForTail {
                minimum_required: minimum_for_quantile,
                configured: minimum_calibration_groups_per_regime,
            });
        }
        for (field, value) in [
            (
                "maximum absolute normalized bias",
                maximum_absolute_normalized_bias_basis_points,
            ),
            (
                "maximum normalized MAE",
                maximum_normalized_mae_basis_points,
            ),
            (
                "minimum lower-bound coverage",
                minimum_lower_bound_coverage_basis_points,
            ),
            (
                "maximum false-positive reduction rate",
                maximum_false_positive_reduction_rate_basis_points,
            ),
            (
                "minimum event qualifying fraction",
                minimum_event_qualifying_fraction_basis_points,
            ),
        ] {
            if value > 10_000 {
                return Err(BaselineQualificationError::InvalidBasisPoints { field, value });
            }
        }
        if !minimum_response_kw.is_finite() || minimum_response_kw <= 0.0 {
            return Err(BaselineQualificationError::InvalidQuantity {
                field: "minimum response",
                value: minimum_response_kw,
            });
        }
        Ok(Self {
            residual_lower_tail_alpha_basis_points,
            minimum_calibration_groups_per_regime,
            minimum_evaluation_groups_per_regime,
            maximum_absolute_normalized_bias_basis_points,
            maximum_normalized_mae_basis_points,
            minimum_lower_bound_coverage_basis_points,
            maximum_false_positive_reduction_rate_basis_points,
            minimum_response_kw,
            minimum_event_qualifying_fraction_basis_points,
        })
    }

    pub const fn minimum_response_kw(&self) -> f64 {
        self.minimum_response_kw
    }
}

/// Frozen relative interval geometry for one complete baseline trajectory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct BaselineTrajectoryIntervalShape {
    pub offset_from_group_start_ns: u64,
    pub duration_ns: u64,
}

/// Per-regime baseline error and conformal-style lower-bound validation receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RegimeBaselineReceipt {
    pub regime_id: String,
    pub calibration_sample_count: usize,
    pub evaluation_sample_count: usize,
    pub calibration_group_count: usize,
    pub evaluation_group_count: usize,
    /// Lower-tail quantile of (observed no-dispatch − predicted no-dispatch).
    pub lower_residual_quantile_kw: f64,
    pub evaluation_bias_kw: f64,
    pub normalized_bias_basis_points: f64,
    pub evaluation_mae_kw: f64,
    pub normalized_mae_basis_points: f64,
    pub evaluation_rmse_kw: f64,
    pub lower_bound_coverage_basis_points: f64,
    pub false_positive_reduction_rate_basis_points: f64,
    /// The exact path shape on which this regime's lower residual is calibrated.
    pub trajectory_shape: Vec<BaselineTrajectoryIntervalShape>,
    pub qualified: bool,
}

/// Qualification result for one baseline model and one declared operating domain.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaselineQualificationReport {
    identity: BaselineIdentity,
    residual_lower_tail_alpha_basis_points: u16,
    minimum_response_kw: f64,
    minimum_event_qualifying_fraction_basis_points: u16,
    calibration_method: String,
    calibration_evaluation_groups_disjoint: bool,
    forecast_issue_time_checked: bool,
    regimes: Vec<RegimeBaselineReceipt>,
    qualified: bool,
    claim_boundary: String,
}

impl BaselineQualificationReport {
    pub fn identity(&self) -> &BaselineIdentity {
        &self.identity
    }

    pub const fn is_qualified(&self) -> bool {
        self.qualified
    }

    pub const fn residual_lower_tail_alpha_basis_points(&self) -> u16 {
        self.residual_lower_tail_alpha_basis_points
    }

    pub const fn minimum_response_kw(&self) -> f64 {
        self.minimum_response_kw
    }

    pub const fn minimum_event_qualifying_fraction_basis_points(&self) -> u16 {
        self.minimum_event_qualifying_fraction_basis_points
    }

    pub fn calibration_method(&self) -> &str {
        &self.calibration_method
    }

    pub const fn calibration_evaluation_groups_disjoint(&self) -> bool {
        self.calibration_evaluation_groups_disjoint
    }

    pub const fn forecast_issue_time_checked(&self) -> bool {
        self.forecast_issue_time_checked
    }

    pub fn regimes(&self) -> &[RegimeBaselineReceipt] {
        &self.regimes
    }

    pub fn claim_boundary(&self) -> &str {
        &self.claim_boundary
    }

    pub fn regime(&self, regime_id: &str) -> Option<&RegimeBaselineReceipt> {
        self.regimes
            .iter()
            .find(|regime| regime.regime_id == regime_id)
    }
}

/// One dispatch-event interval with baseline forecast and metrology-bounded
/// observed PCC load. The measurement uncertainty is an expanded upper bound
/// supplied by the independent PCC metrology verifier.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum DispatchEventPhase {
    Dispatch,
    Recovery,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct DispatchEventPoint {
    point_id: String,
    phase: DispatchEventPhase,
    regime_id: String,
    forecast_issued_at_ns: u64,
    interval_start_ns: u64,
    interval_duration_ns: u64,
    predicted_no_dispatch_power_kw: f64,
    measured_dispatch_power_kw: f64,
    metrology_expanded_uncertainty_kw: f64,
    evidence_id: String,
    source_digest: String,
}

impl DispatchEventPoint {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        point_id: impl Into<String>,
        phase: DispatchEventPhase,
        regime_id: impl Into<String>,
        forecast_issued_at_ns: u64,
        interval_start_ns: u64,
        interval_duration_ns: u64,
        predicted_no_dispatch_power_kw: f64,
        measured_dispatch_power_kw: f64,
        metrology_expanded_uncertainty_kw: f64,
        evidence_id: impl Into<String>,
        source_digest: impl Into<String>,
    ) -> Result<Self, BaselineQualificationError> {
        let point = Self {
            point_id: point_id.into(),
            phase,
            regime_id: regime_id.into(),
            forecast_issued_at_ns,
            interval_start_ns,
            interval_duration_ns,
            predicted_no_dispatch_power_kw,
            measured_dispatch_power_kw,
            metrology_expanded_uncertainty_kw,
            evidence_id: evidence_id.into(),
            source_digest: source_digest.into(),
        };
        for (field, value) in [
            ("event point ID", point.point_id.as_str()),
            ("event regime ID", point.regime_id.as_str()),
            ("event evidence ID", point.evidence_id.as_str()),
            ("event trace source digest", point.source_digest.as_str()),
        ] {
            require_id(field, value)?;
        }
        if interval_duration_ns == 0 {
            return Err(BaselineQualificationError::InvalidIntervalDuration);
        }
        if forecast_issued_at_ns > interval_start_ns {
            return Err(BaselineQualificationError::FutureInformationLeakage {
                sample_id: point.point_id.clone(),
                forecast_issued_at_ns,
                interval_start_ns,
            });
        }
        for (field, value) in [
            ("event baseline forecast", predicted_no_dispatch_power_kw),
            ("measured dispatch power", measured_dispatch_power_kw),
            (
                "PCC metrology expanded uncertainty",
                metrology_expanded_uncertainty_kw,
            ),
        ] {
            if !value.is_finite() {
                return Err(BaselineQualificationError::InvalidQuantity { field, value });
            }
        }
        if metrology_expanded_uncertainty_kw < 0.0 {
            return Err(BaselineQualificationError::InvalidQuantity {
                field: "PCC metrology expanded uncertainty",
                value: metrology_expanded_uncertainty_kw,
            });
        }
        interval_start_ns
            .checked_add(interval_duration_ns)
            .ok_or(BaselineQualificationError::TimestampOverflow)?;
        Ok(point)
    }
}

/// Explicit evidence for the dispatch-end boundary and subsequent recovery
/// observation. The caller binds useful-work and physical-state reconciliation
/// to this source digest; a bare completion boolean cannot certify recovery.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct EventRecoveryEvidence {
    event_start_ns: u64,
    dispatch_end_ns: u64,
    observation_horizon_end_ns: u64,
    evidence_id: String,
    source_digest: String,
    workload_reconciliation_receipt_id: String,
    physical_state_reconciliation_receipt_id: String,
    workload_obligations_reconciled: bool,
    physical_resource_states_reconciled: bool,
}

impl EventRecoveryEvidence {
    // The constructor pins two independent reconciliation receipts alongside
    // the source horizon and its time boundaries; keep this explicit API.
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        event_start_ns: u64,
        dispatch_end_ns: u64,
        observation_horizon_end_ns: u64,
        evidence_id: impl Into<String>,
        source_digest: impl Into<String>,
        workload_reconciliation_receipt_id: impl Into<String>,
        physical_state_reconciliation_receipt_id: impl Into<String>,
        workload_obligations_reconciled: bool,
        physical_resource_states_reconciled: bool,
    ) -> Result<Self, BaselineQualificationError> {
        let evidence = Self {
            event_start_ns,
            dispatch_end_ns,
            observation_horizon_end_ns,
            evidence_id: evidence_id.into(),
            source_digest: source_digest.into(),
            workload_reconciliation_receipt_id: workload_reconciliation_receipt_id.into(),
            physical_state_reconciliation_receipt_id: physical_state_reconciliation_receipt_id
                .into(),
            workload_obligations_reconciled,
            physical_resource_states_reconciled,
        };
        require_id("event recovery evidence ID", &evidence.evidence_id)?;
        require_id("event recovery source digest", &evidence.source_digest)?;
        require_id(
            "workload reconciliation receipt ID",
            &evidence.workload_reconciliation_receipt_id,
        )?;
        require_id(
            "physical-state reconciliation receipt ID",
            &evidence.physical_state_reconciliation_receipt_id,
        )?;
        if !(event_start_ns < dispatch_end_ns && dispatch_end_ns < observation_horizon_end_ns) {
            return Err(BaselineQualificationError::InvalidEventRecoveryHorizon {
                event_start_ns,
                dispatch_end_ns,
                observation_horizon_end_ns,
            });
        }
        Ok(evidence)
    }

    pub fn workload_reconciliation_receipt_id(&self) -> &str {
        &self.workload_reconciliation_receipt_id
    }

    pub fn physical_state_reconciliation_receipt_id(&self) -> &str {
        &self.physical_state_reconciliation_receipt_id
    }

    pub const fn is_complete(&self) -> bool {
        self.workload_obligations_reconciled && self.physical_resource_states_reconciled
    }
}

/// Per-interval conservative event-impact receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaselineEventIntervalReceipt {
    pub point_id: String,
    pub phase: DispatchEventPhase,
    pub regime_id: String,
    pub interval_start_ns: u64,
    pub interval_duration_ns: u64,
    pub predicted_no_dispatch_power_kw: f64,
    pub baseline_lower_power_bound_kw: f64,
    pub measured_dispatch_power_kw: f64,
    pub measured_dispatch_upper_power_bound_kw: f64,
    pub baseline_residual_lower_quantile_kw: f64,
    pub metrology_expanded_uncertainty_kw: f64,
    pub reduction_point_estimate_kw: f64,
    pub conservative_reduction_lower_bound_kw: f64,
    pub conservative_reduction_energy_kwh: f64,
    pub interval_qualified: bool,
    pub evidence_id: String,
}

/// Rebound-aware event impact estimate. This is model-based M&V, not by itself
/// randomized causal evidence, market settlement or ELCC.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct BaselineEventReport {
    pub identity: BaselineIdentity,
    pub baseline_qualification_passed: bool,
    pub interval_count: usize,
    pub dispatch_interval_count: usize,
    pub recovery_interval_count: usize,
    pub qualifying_interval_count: usize,
    pub qualifying_interval_fraction_basis_points: f64,
    pub gross_reduction_energy_kwh: f64,
    pub gross_rebound_energy_kwh: f64,
    pub net_reduction_energy_kwh: f64,
    /// Sum of interval conservative lower-bound energy contributions. This is
    /// not a joint probability bound without cross-interval dependence.
    pub conservative_net_reduction_energy_kwh: f64,
    pub minimum_response_kw: f64,
    pub event_qualified: bool,
    /// Interval bounds are not automatically a joint event-horizon probability.
    pub event_lower_bound_is_joint_probability: bool,
    pub event_and_recovery_included_in_supplied_horizon: bool,
    pub recovery_evidence_id: String,
    pub recovery_source_digest: String,
    pub workload_reconciliation_receipt_id: String,
    pub physical_state_reconciliation_receipt_id: String,
    pub workload_obligations_reconciled: bool,
    pub physical_resource_states_reconciled: bool,
    pub event_start_ns: u64,
    pub dispatch_end_ns: u64,
    pub observation_horizon_end_ns: u64,
    pub intervals: Vec<BaselineEventIntervalReceipt>,
    pub claim_boundary: String,
}

/// Fail-closed baseline qualification and event-impact errors.
#[derive(Debug, Clone, PartialEq)]
pub enum BaselineQualificationError {
    MissingIdentity {
        field: &'static str,
    },
    InvalidQuantity {
        field: &'static str,
        value: f64,
    },
    InvalidBasisPoints {
        field: &'static str,
        value: u16,
    },
    InvalidMinimumSampleCount,
    SourceDigestMismatch {
        fold: BaselineFold,
        expected: String,
        observed: String,
    },
    InsufficientCalibrationForTail {
        minimum_required: usize,
        configured: usize,
    },
    InvalidIntervalDuration,
    FutureInformationLeakage {
        sample_id: String,
        forecast_issued_at_ns: u64,
        interval_start_ns: u64,
    },
    DispatchedDataInBaselineFold {
        sample_id: String,
    },
    DuplicateSampleId {
        sample_id: String,
    },
    CalibrationEvaluationGroupOverlap {
        group_id: String,
    },
    PartitionDigestCollision,
    InsufficientCalibrationGroups {
        regime_id: String,
        observed_groups: usize,
        required_groups: usize,
    },
    InsufficientEvaluationGroups {
        regime_id: String,
        observed_groups: usize,
        required_groups: usize,
    },
    NoSharedOperatingRegimes,
    EvaluationRegimeMissingCalibration {
        regime_id: String,
    },
    NonContiguousBaselineTrajectory {
        regime_id: String,
        group_id: String,
        index: usize,
        expected_start_ns: u64,
        observed_start_ns: u64,
    },
    BaselineTrajectoryShapeMismatch {
        regime_id: String,
        group_id: String,
        expected: Vec<BaselineTrajectoryIntervalShape>,
        observed: Vec<BaselineTrajectoryIntervalShape>,
    },
    EventTrajectoryShapeMismatch {
        regime_id: String,
        expected: Vec<BaselineTrajectoryIntervalShape>,
        observed: Vec<BaselineTrajectoryIntervalShape>,
    },
    InvalidNormalizedPowerScale {
        regime_id: String,
    },
    BaselineNotQualified,
    BaselineIdentityMismatch,
    UnknownOperatingRegime {
        regime_id: String,
    },
    NonContiguousEventIntervals {
        index: usize,
        expected_start_ns: u64,
        observed_start_ns: u64,
    },
    DuplicateEventPoint {
        point_id: String,
    },
    EventPointSourceDigestMismatch {
        point_id: String,
        expected: String,
        observed: String,
    },
    InvalidEventRecoveryHorizon {
        event_start_ns: u64,
        dispatch_end_ns: u64,
        observation_horizon_end_ns: u64,
    },
    EventHorizonStartMismatch {
        expected_start_ns: u64,
        observed_start_ns: u64,
    },
    EventHorizonEndMismatch {
        expected_end_ns: u64,
        observed_end_ns: u64,
    },
    EventIntervalStraddlesDispatchEnd {
        index: usize,
        dispatch_end_ns: u64,
    },
    DispatchRecoveryPhaseOrderViolation {
        index: usize,
    },
    MissingRecoveryIntervals,
    EmptyEvent,
    TimestampOverflow,
    NumericalOverflow(&'static str),
}

impl fmt::Display for BaselineQualificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::InvalidQuantity { field, value } => {
                write!(f, "{field} must be finite and in range; got {value}")
            }
            Self::InvalidBasisPoints { field, value } => {
                write!(f, "{field} must be in 0..=10000 basis points; got {value}")
            }
            Self::InvalidMinimumSampleCount => write!(
                f,
                "calibration and evaluation sample minima must be positive"
            ),
            Self::SourceDigestMismatch {
                fold,
                expected,
                observed,
            } => write!(
                f,
                "{fold:?} sample source digest {observed} does not match pinned partition digest {expected}"
            ),
            Self::InsufficientCalibrationForTail {
                minimum_required,
                configured,
            } => write!(
                f,
                "calibration sample minimum {configured} cannot support the requested lower tail; at least {minimum_required} are required"
            ),
            Self::InvalidIntervalDuration => write!(f, "interval duration must be positive"),
            Self::FutureInformationLeakage {
                sample_id,
                forecast_issued_at_ns,
                interval_start_ns,
            } => write!(
                f,
                "forecast {sample_id} was issued at {forecast_issued_at_ns} after target interval start {interval_start_ns}"
            ),
            Self::DispatchedDataInBaselineFold { sample_id } => write!(
                f,
                "sample {sample_id} is dispatch-active and cannot qualify a no-dispatch baseline"
            ),
            Self::DuplicateSampleId { sample_id } => {
                write!(f, "duplicate baseline sample ID {sample_id}")
            }
            Self::CalibrationEvaluationGroupOverlap { group_id } => write!(
                f,
                "independence group {group_id} leaks across calibration and evaluation"
            ),
            Self::PartitionDigestCollision => {
                write!(f, "calibration and evaluation data digests must differ")
            }
            Self::InsufficientCalibrationGroups {
                regime_id,
                observed_groups,
                required_groups,
            } => write!(
                f,
                "regime {regime_id} has {observed_groups} independent calibration groups; requires {required_groups}"
            ),
            Self::InsufficientEvaluationGroups {
                regime_id,
                observed_groups,
                required_groups,
            } => write!(
                f,
                "regime {regime_id} has {observed_groups} independent evaluation groups; requires {required_groups}"
            ),
            Self::NoSharedOperatingRegimes => write!(
                f,
                "calibration and evaluation sets have no shared operating regimes"
            ),
            Self::EvaluationRegimeMissingCalibration { regime_id } => write!(
                f,
                "evaluation operating regime {regime_id} has no disjoint calibration residual partition"
            ),
            Self::NonContiguousBaselineTrajectory {
                regime_id,
                group_id,
                index,
                expected_start_ns,
                observed_start_ns,
            } => write!(
                f,
                "baseline trajectory group {group_id} in regime {regime_id} has a gap/overlap at interval {index}: expected {expected_start_ns}, observed {observed_start_ns}"
            ),
            Self::BaselineTrajectoryShapeMismatch {
                regime_id,
                group_id,
                expected,
                observed,
            } => write!(
                f,
                "baseline trajectory group {group_id} in regime {regime_id} has interval geometry {observed:?}; expected {expected:?}"
            ),
            Self::EventTrajectoryShapeMismatch {
                regime_id,
                expected,
                observed,
            } => write!(
                f,
                "event/recovery trace in regime {regime_id} has interval geometry {observed:?}; qualified baseline horizon has {expected:?}"
            ),
            Self::InvalidNormalizedPowerScale { regime_id } => write!(
                f,
                "regime {regime_id} has a zero or invalid normalization scale"
            ),
            Self::BaselineNotQualified => write!(
                f,
                "baseline model did not pass its pre-registered holdout qualification"
            ),
            Self::BaselineIdentityMismatch => write!(
                f,
                "event model identity does not match qualified baseline identity"
            ),
            Self::UnknownOperatingRegime { regime_id } => {
                write!(f, "event operating regime {regime_id} was not qualified")
            }
            Self::NonContiguousEventIntervals {
                index,
                expected_start_ns,
                observed_start_ns,
            } => write!(
                f,
                "event interval {index} starts at {observed_start_ns}; expected contiguous start {expected_start_ns}"
            ),
            Self::DuplicateEventPoint { point_id } => {
                write!(f, "duplicate event point ID {point_id}")
            }
            Self::EventPointSourceDigestMismatch {
                point_id,
                expected,
                observed,
            } => write!(
                f,
                "event point {point_id} source digest {observed} differs from pinned horizon digest {expected}"
            ),
            Self::InvalidEventRecoveryHorizon {
                event_start_ns,
                dispatch_end_ns,
                observation_horizon_end_ns,
            } => write!(
                f,
                "event recovery horizon must satisfy start < dispatch end < observed-through; got {event_start_ns}, {dispatch_end_ns}, {observation_horizon_end_ns}"
            ),
            Self::EventHorizonStartMismatch {
                expected_start_ns,
                observed_start_ns,
            } => write!(
                f,
                "event trace starts at {observed_start_ns}; recovery evidence requires {expected_start_ns}"
            ),
            Self::EventHorizonEndMismatch {
                expected_end_ns,
                observed_end_ns,
            } => write!(
                f,
                "event trace ends at {observed_end_ns}; recovery evidence requires {expected_end_ns}"
            ),
            Self::EventIntervalStraddlesDispatchEnd {
                index,
                dispatch_end_ns,
            } => write!(
                f,
                "event interval {index} straddles dispatch-end boundary {dispatch_end_ns}"
            ),
            Self::DispatchRecoveryPhaseOrderViolation { index } => write!(
                f,
                "dispatch/recovery phase order is invalid at interval {index}"
            ),
            Self::MissingRecoveryIntervals => write!(
                f,
                "event evidence requires at least one recovery interval after dispatch ends"
            ),
            Self::EmptyEvent => write!(f, "event evaluation requires at least one interval"),
            Self::TimestampOverflow => write!(f, "timestamp arithmetic overflow"),
            Self::NumericalOverflow(field) => write!(f, "numerical overflow computing {field}"),
        }
    }
}

impl std::error::Error for BaselineQualificationError {}

/// Qualify a model's no-dispatch residuals using disjoint calibration and
/// evaluation groups. The model itself is fitted elsewhere; this evaluator
/// independently audits forecast timing, residual calibration, bias and
/// negative-control apparent reductions.
pub fn qualify_counterfactual_baseline(
    identity: &BaselineIdentity,
    contract: &BaselineQualificationContract,
    observations: &[NoDispatchObservation],
) -> Result<BaselineQualificationReport, BaselineQualificationError> {
    if observations.is_empty() {
        return Err(BaselineQualificationError::NoSharedOperatingRegimes);
    }

    let mut sample_ids = BTreeSet::new();
    let mut calibration_groups = BTreeSet::new();
    let mut evaluation_groups = BTreeSet::new();
    let mut calibration: BTreeMap<&str, Vec<&NoDispatchObservation>> = BTreeMap::new();
    let mut evaluation: BTreeMap<&str, Vec<&NoDispatchObservation>> = BTreeMap::new();

    for observation in observations {
        let expected_source_digest = match observation.fold {
            BaselineFold::Calibration => &identity.calibration_data_digest,
            BaselineFold::Evaluation => &identity.evaluation_data_digest,
        };
        if &observation.source_digest != expected_source_digest {
            return Err(BaselineQualificationError::SourceDigestMismatch {
                fold: observation.fold,
                expected: expected_source_digest.clone(),
                observed: observation.source_digest.clone(),
            });
        }
        if !sample_ids.insert(observation.sample_id.as_str()) {
            return Err(BaselineQualificationError::DuplicateSampleId {
                sample_id: observation.sample_id.clone(),
            });
        }
        match observation.fold {
            BaselineFold::Calibration => {
                calibration_groups.insert(observation.group_id.as_str());
                calibration
                    .entry(&observation.regime_id)
                    .or_default()
                    .push(observation);
            }
            BaselineFold::Evaluation => {
                evaluation_groups.insert(observation.group_id.as_str());
                evaluation
                    .entry(&observation.regime_id)
                    .or_default()
                    .push(observation);
            }
        }
    }
    if let Some(group_id) = calibration_groups.intersection(&evaluation_groups).next() {
        return Err(
            BaselineQualificationError::CalibrationEvaluationGroupOverlap {
                group_id: (*group_id).to_owned(),
            },
        );
    }

    for regime_id in evaluation.keys() {
        if !calibration.contains_key(regime_id) {
            return Err(
                BaselineQualificationError::EvaluationRegimeMissingCalibration {
                    regime_id: (*regime_id).to_owned(),
                },
            );
        }
    }
    let regimes: BTreeSet<&str> = evaluation.keys().copied().collect();
    if regimes.is_empty() {
        return Err(BaselineQualificationError::NoSharedOperatingRegimes);
    }

    let mut regime_receipts = Vec::with_capacity(regimes.len());
    for regime_id in regimes {
        let calibration_samples = calibration.get(regime_id).expect("regime key exists");
        let evaluation_samples = evaluation.get(regime_id).expect("regime key exists");
        let calibration_group_ids: BTreeSet<&str> = calibration_samples
            .iter()
            .map(|sample| sample.group_id.as_str())
            .collect();
        let evaluation_group_ids: BTreeSet<&str> = evaluation_samples
            .iter()
            .map(|sample| sample.group_id.as_str())
            .collect();
        if calibration_group_ids.len() < contract.minimum_calibration_groups_per_regime {
            return Err(BaselineQualificationError::InsufficientCalibrationGroups {
                regime_id: regime_id.to_owned(),
                observed_groups: calibration_group_ids.len(),
                required_groups: contract.minimum_calibration_groups_per_regime,
            });
        }
        if evaluation_group_ids.len() < contract.minimum_evaluation_groups_per_regime {
            return Err(BaselineQualificationError::InsufficientEvaluationGroups {
                regime_id: regime_id.to_owned(),
                observed_groups: evaluation_group_ids.len(),
                required_groups: contract.minimum_evaluation_groups_per_regime,
            });
        }

        // Treat each independence group as one calibration unit. Use its
        // worst interval residual so serially correlated rows cannot inflate
        // the nominal tail sample count or hide a bad interval within a block.
        let mut group_lower_residuals: BTreeMap<&str, f64> = BTreeMap::new();
        for sample in calibration_samples {
            let conservative_residual = sample.observed_no_dispatch_power_kw
                - sample.observed_expanded_uncertainty_kw
                - sample.predicted_no_dispatch_power_kw;
            group_lower_residuals
                .entry(sample.group_id.as_str())
                .and_modify(|current| *current = current.min(conservative_residual))
                .or_insert(conservative_residual);
        }
        let calibration_shapes = trajectory_shapes(calibration_samples, regime_id)?;
        let evaluation_shapes = trajectory_shapes(evaluation_samples, regime_id)?;
        let expected_trajectory_shape = calibration_shapes
            .values()
            .next()
            .cloned()
            .ok_or(BaselineQualificationError::NoSharedOperatingRegimes)?;
        for (group_id, shape) in calibration_shapes.iter().chain(evaluation_shapes.iter()) {
            if shape != &expected_trajectory_shape {
                return Err(
                    BaselineQualificationError::BaselineTrajectoryShapeMismatch {
                        regime_id: regime_id.to_owned(),
                        group_id: (*group_id).to_owned(),
                        expected: expected_trajectory_shape.clone(),
                        observed: shape.clone(),
                    },
                );
            }
        }
        let mut calibration_residuals: Vec<f64> = group_lower_residuals.values().copied().collect();
        calibration_residuals.sort_by(f64::total_cmp);
        let alpha = contract.residual_lower_tail_alpha_basis_points as usize;
        let rank = ((calibration_residuals.len() + 1) * alpha / 10_000)
            .clamp(1, calibration_residuals.len());
        let lower_residual_quantile_kw = calibration_residuals[rank - 1];

        // Give each independent evaluation block equal weight. Long sessions
        // must not dominate bias/MAE/RMSE merely because they contain more rows.
        let mut group_errors: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        let mut group_actual_magnitudes: BTreeMap<&str, Vec<f64>> = BTreeMap::new();
        for sample in evaluation_samples {
            let error =
                sample.predicted_no_dispatch_power_kw - sample.observed_no_dispatch_power_kw;
            group_errors
                .entry(sample.group_id.as_str())
                .or_default()
                .push(error);
            group_actual_magnitudes
                .entry(sample.group_id.as_str())
                .or_default()
                .push(sample.observed_no_dispatch_power_kw.abs());
        }
        let group_biases: Vec<f64> = group_errors.values().map(|errors| mean(errors)).collect();
        let group_maes: Vec<f64> = group_errors
            .values()
            .map(|errors| errors.iter().map(|error| error.abs()).sum::<f64>() / errors.len() as f64)
            .collect();
        let group_mses: Vec<f64> = group_errors
            .values()
            .map(|errors| {
                errors.iter().map(|error| error.powi(2)).sum::<f64>() / errors.len() as f64
            })
            .collect();
        let group_actual_scales: Vec<f64> = group_actual_magnitudes
            .values()
            .map(|values| values.iter().sum::<f64>() / values.len() as f64)
            .collect();
        let evaluation_bias_kw = mean(&group_biases);
        let evaluation_mae_kw = mean(&group_maes);
        let evaluation_rmse_kw = mean(&group_mses).sqrt();
        let mean_absolute_actual_kw = mean(&group_actual_scales);
        if !mean_absolute_actual_kw.is_finite() || mean_absolute_actual_kw <= EPSILON {
            return Err(BaselineQualificationError::InvalidNormalizedPowerScale {
                regime_id: regime_id.to_owned(),
            });
        }
        let normalized_bias_basis_points =
            evaluation_bias_kw.abs() / mean_absolute_actual_kw * BASIS_POINTS;
        let normalized_mae_basis_points =
            evaluation_mae_kw / mean_absolute_actual_kw * BASIS_POINTS;

        // Evaluate at the independent-group level: one bad interval means
        // that a group fails joint within-block coverage. This avoids treating
        // autocorrelated intervals as independent Bernoulli trials.
        let mut group_audit: BTreeMap<&str, (bool, bool)> = BTreeMap::new();
        for sample in evaluation_samples {
            let lower_bound = sample.predicted_no_dispatch_power_kw + lower_residual_quantile_kw;
            let observed_lower_bound =
                sample.observed_no_dispatch_power_kw - sample.observed_expanded_uncertainty_kw;
            let covered = observed_lower_bound + 1.0e-9 >= lower_bound;
            let false_positive = lower_bound - observed_lower_bound >= contract.minimum_response_kw;
            let audit = group_audit
                .entry(sample.group_id.as_str())
                .or_insert((true, false));
            audit.0 &= covered;
            audit.1 |= false_positive;
        }
        let covered_groups = group_audit.values().filter(|(covered, _)| *covered).count();
        let false_positive_groups = group_audit
            .values()
            .filter(|(_, false_positive)| *false_positive)
            .count();
        let lower_bound_coverage_basis_points =
            covered_groups as f64 / group_audit.len() as f64 * BASIS_POINTS;
        let false_positive_reduction_rate_basis_points =
            false_positive_groups as f64 / group_audit.len() as f64 * BASIS_POINTS;

        let qualified = normalized_bias_basis_points
            <= contract.maximum_absolute_normalized_bias_basis_points as f64
            && normalized_mae_basis_points <= contract.maximum_normalized_mae_basis_points as f64
            && lower_bound_coverage_basis_points
                >= contract.minimum_lower_bound_coverage_basis_points as f64
            && false_positive_reduction_rate_basis_points
                <= contract.maximum_false_positive_reduction_rate_basis_points as f64;

        regime_receipts.push(RegimeBaselineReceipt {
            regime_id: regime_id.to_owned(),
            calibration_sample_count: calibration_samples.len(),
            evaluation_sample_count: evaluation_samples.len(),
            calibration_group_count: calibration_group_ids.len(),
            evaluation_group_count: evaluation_group_ids.len(),
            lower_residual_quantile_kw,
            evaluation_bias_kw,
            normalized_bias_basis_points,
            evaluation_mae_kw,
            normalized_mae_basis_points,
            evaluation_rmse_kw,
            lower_bound_coverage_basis_points,
            false_positive_reduction_rate_basis_points,
            trajectory_shape: expected_trajectory_shape,
            qualified,
        });
    }

    let qualified = regime_receipts.iter().all(|regime| regime.qualified);
    Ok(BaselineQualificationReport {
        identity: identity.clone(),
        residual_lower_tail_alpha_basis_points: contract.residual_lower_tail_alpha_basis_points,
        minimum_response_kw: contract.minimum_response_kw,
        minimum_event_qualifying_fraction_basis_points:
            contract.minimum_event_qualifying_fraction_basis_points,
        calibration_method: "empirical lower-tail quantile of observed-minus-predicted residuals on a disjoint no-dispatch calibration partition; audited on independent no-dispatch evaluation groups".to_owned(),
        calibration_evaluation_groups_disjoint: true,
        forecast_issue_time_checked: true,
        regimes: regime_receipts,
        qualified,
        claim_boundary: "model-based counterfactual baseline qualification on the supplied no-dispatch holdout distribution; not causal identification, market accreditation or ELCC".to_owned(),
    })
}

/// Apply a qualified lower baseline bound and upper PCC measurement bound to a
/// complete chronological event plus recovery trace.
pub fn evaluate_baseline_bounded_event(
    qualified_baseline: &BaselineQualificationReport,
    event_identity: &BaselineIdentity,
    recovery_evidence: &EventRecoveryEvidence,
    points: &[DispatchEventPoint],
) -> Result<BaselineEventReport, BaselineQualificationError> {
    if !qualified_baseline.qualified {
        return Err(BaselineQualificationError::BaselineNotQualified);
    }
    if &qualified_baseline.identity != event_identity {
        return Err(BaselineQualificationError::BaselineIdentityMismatch);
    }
    if points.is_empty() {
        return Err(BaselineQualificationError::EmptyEvent);
    }

    if points[0].interval_start_ns != recovery_evidence.event_start_ns {
        return Err(BaselineQualificationError::EventHorizonStartMismatch {
            expected_start_ns: recovery_evidence.event_start_ns,
            observed_start_ns: points[0].interval_start_ns,
        });
    }
    let mut point_ids = BTreeSet::new();
    let mut previous_end_ns: Option<u64> = None;
    let mut seen_recovery_phase = false;
    let mut dispatch_interval_count = 0usize;
    let mut recovery_interval_count = 0usize;
    let mut receipts = Vec::with_capacity(points.len());
    let mut gross_reduction_energy_kwh = 0.0;
    let mut gross_rebound_energy_kwh = 0.0;
    let mut net_reduction_energy_kwh = 0.0;
    let mut conservative_net_reduction_energy_kwh = 0.0;
    let mut qualifying_interval_count = 0usize;

    for (index, point) in points.iter().enumerate() {
        if point.source_digest != recovery_evidence.source_digest {
            return Err(BaselineQualificationError::EventPointSourceDigestMismatch {
                point_id: point.point_id.clone(),
                expected: recovery_evidence.source_digest.clone(),
                observed: point.source_digest.clone(),
            });
        }
        if !point_ids.insert(point.point_id.as_str()) {
            return Err(BaselineQualificationError::DuplicateEventPoint {
                point_id: point.point_id.clone(),
            });
        }
        if let Some(expected_start_ns) = previous_end_ns
            && point.interval_start_ns != expected_start_ns
        {
            return Err(BaselineQualificationError::NonContiguousEventIntervals {
                index,
                expected_start_ns,
                observed_start_ns: point.interval_start_ns,
            });
        }
        let end_ns = point
            .interval_start_ns
            .checked_add(point.interval_duration_ns)
            .ok_or(BaselineQualificationError::TimestampOverflow)?;
        if point.interval_start_ns < recovery_evidence.dispatch_end_ns
            && end_ns > recovery_evidence.dispatch_end_ns
        {
            return Err(
                BaselineQualificationError::EventIntervalStraddlesDispatchEnd {
                    index,
                    dispatch_end_ns: recovery_evidence.dispatch_end_ns,
                },
            );
        }
        match point.phase {
            DispatchEventPhase::Dispatch => {
                if seen_recovery_phase || end_ns > recovery_evidence.dispatch_end_ns {
                    return Err(
                        BaselineQualificationError::DispatchRecoveryPhaseOrderViolation { index },
                    );
                }
                dispatch_interval_count += 1;
            }
            DispatchEventPhase::Recovery => {
                if point.interval_start_ns < recovery_evidence.dispatch_end_ns {
                    return Err(
                        BaselineQualificationError::DispatchRecoveryPhaseOrderViolation { index },
                    );
                }
                seen_recovery_phase = true;
                recovery_interval_count += 1;
            }
        }
        previous_end_ns = Some(end_ns);

        let regime = qualified_baseline.regime(&point.regime_id).ok_or_else(|| {
            BaselineQualificationError::UnknownOperatingRegime {
                regime_id: point.regime_id.clone(),
            }
        })?;
        if !regime.qualified {
            return Err(BaselineQualificationError::BaselineNotQualified);
        }

        let baseline_lower_power_bound_kw =
            point.predicted_no_dispatch_power_kw + regime.lower_residual_quantile_kw;
        let measured_dispatch_upper_power_bound_kw =
            point.measured_dispatch_power_kw + point.metrology_expanded_uncertainty_kw;
        let reduction_point_estimate_kw =
            point.predicted_no_dispatch_power_kw - point.measured_dispatch_power_kw;
        let conservative_reduction_lower_bound_kw =
            baseline_lower_power_bound_kw - measured_dispatch_upper_power_bound_kw;

        let duration_hours = point.interval_duration_ns as f64 / NANOSECONDS_PER_HOUR;
        let conservative_reduction_energy_kwh =
            conservative_reduction_lower_bound_kw * duration_hours;
        if !baseline_lower_power_bound_kw.is_finite()
            || !measured_dispatch_upper_power_bound_kw.is_finite()
            || !reduction_point_estimate_kw.is_finite()
            || !conservative_reduction_lower_bound_kw.is_finite()
            || !conservative_reduction_energy_kwh.is_finite()
        {
            return Err(BaselineQualificationError::NumericalOverflow(
                "event impact",
            ));
        }
        if point.phase == DispatchEventPhase::Dispatch
            && conservative_reduction_lower_bound_kw >= qualified_baseline.minimum_response_kw
        {
            qualifying_interval_count += 1;
        }
        let point_energy_kwh = reduction_point_estimate_kw * duration_hours;
        if !point_energy_kwh.is_finite() {
            return Err(BaselineQualificationError::NumericalOverflow(
                "point-estimate event energy",
            ));
        }
        if point_energy_kwh >= 0.0 {
            gross_reduction_energy_kwh += point_energy_kwh;
        } else {
            gross_rebound_energy_kwh += -point_energy_kwh;
        }
        net_reduction_energy_kwh += point_energy_kwh;
        conservative_net_reduction_energy_kwh += conservative_reduction_energy_kwh;
        if !gross_reduction_energy_kwh.is_finite()
            || !gross_rebound_energy_kwh.is_finite()
            || !net_reduction_energy_kwh.is_finite()
            || !conservative_net_reduction_energy_kwh.is_finite()
        {
            return Err(BaselineQualificationError::NumericalOverflow(
                "aggregate event energy",
            ));
        }

        receipts.push(BaselineEventIntervalReceipt {
            point_id: point.point_id.clone(),
            phase: point.phase,
            regime_id: point.regime_id.clone(),
            interval_start_ns: point.interval_start_ns,
            interval_duration_ns: point.interval_duration_ns,
            predicted_no_dispatch_power_kw: point.predicted_no_dispatch_power_kw,
            baseline_lower_power_bound_kw,
            measured_dispatch_power_kw: point.measured_dispatch_power_kw,
            measured_dispatch_upper_power_bound_kw,
            baseline_residual_lower_quantile_kw: regime.lower_residual_quantile_kw,
            metrology_expanded_uncertainty_kw: point.metrology_expanded_uncertainty_kw,
            reduction_point_estimate_kw,
            conservative_reduction_lower_bound_kw,
            conservative_reduction_energy_kwh,
            interval_qualified: conservative_reduction_lower_bound_kw
                >= qualified_baseline.minimum_response_kw,
            evidence_id: point.evidence_id.clone(),
        });
    }

    if dispatch_interval_count == 0 {
        return Err(BaselineQualificationError::EmptyEvent);
    }
    if recovery_interval_count == 0 {
        return Err(BaselineQualificationError::MissingRecoveryIntervals);
    }
    let event_origin_ns = recovery_evidence.event_start_ns;
    let event_shape: Vec<BaselineTrajectoryIntervalShape> = points
        .iter()
        .map(|point| BaselineTrajectoryIntervalShape {
            offset_from_group_start_ns: point.interval_start_ns - event_origin_ns,
            duration_ns: point.interval_duration_ns,
        })
        .collect();
    let used_regimes: BTreeSet<&str> = points
        .iter()
        .map(|point| point.regime_id.as_str())
        .collect();
    for regime_id in used_regimes {
        let expected_shape = &qualified_baseline
            .regime(regime_id)
            .ok_or_else(|| BaselineQualificationError::UnknownOperatingRegime {
                regime_id: regime_id.to_owned(),
            })?
            .trajectory_shape;
        if &event_shape != expected_shape {
            return Err(BaselineQualificationError::EventTrajectoryShapeMismatch {
                regime_id: regime_id.to_owned(),
                expected: expected_shape.clone(),
                observed: event_shape.clone(),
            });
        }
    }
    let observed_horizon_end_ns = previous_end_ns.ok_or(BaselineQualificationError::EmptyEvent)?;
    if observed_horizon_end_ns != recovery_evidence.observation_horizon_end_ns {
        return Err(BaselineQualificationError::EventHorizonEndMismatch {
            expected_end_ns: recovery_evidence.observation_horizon_end_ns,
            observed_end_ns: observed_horizon_end_ns,
        });
    }
    let qualifying_interval_fraction_basis_points =
        qualifying_interval_count as f64 / dispatch_interval_count as f64 * BASIS_POINTS;
    let event_qualified = recovery_evidence.is_complete()
        && qualifying_interval_fraction_basis_points
            >= qualified_baseline.minimum_event_qualifying_fraction_basis_points as f64;

    Ok(BaselineEventReport {
        identity: qualified_baseline.identity.clone(),
        baseline_qualification_passed: true,
        interval_count: points.len(),
        dispatch_interval_count,
        recovery_interval_count,
        qualifying_interval_count,
        qualifying_interval_fraction_basis_points,
        gross_reduction_energy_kwh,
        gross_rebound_energy_kwh,
        net_reduction_energy_kwh,
        conservative_net_reduction_energy_kwh,
        minimum_response_kw: qualified_baseline.minimum_response_kw,
        event_qualified,
        event_lower_bound_is_joint_probability: false,
        event_and_recovery_included_in_supplied_horizon: true,
        recovery_evidence_id: recovery_evidence.evidence_id.clone(),
        recovery_source_digest: recovery_evidence.source_digest.clone(),
        workload_reconciliation_receipt_id: recovery_evidence
            .workload_reconciliation_receipt_id
            .clone(),
        physical_state_reconciliation_receipt_id: recovery_evidence
            .physical_state_reconciliation_receipt_id
            .clone(),
        workload_obligations_reconciled: recovery_evidence.workload_obligations_reconciled,
        physical_resource_states_reconciled: recovery_evidence.physical_resource_states_reconciled,
        event_start_ns: recovery_evidence.event_start_ns,
        dispatch_end_ns: recovery_evidence.dispatch_end_ns,
        observation_horizon_end_ns: recovery_evidence.observation_horizon_end_ns,
        intervals: receipts,
        claim_boundary: "model-based conservative impact bound using holdout-qualified counterfactual baseline and caller-supplied expanded PCC measurement uncertainty; no causal identification is asserted".to_owned(),
    })
}

/// Build a contiguous, relative-time trajectory shape for every independent
/// group. Absolute start times may differ; interval offsets and durations may
/// not. Groups with different numbers of samples are never pooled together.
fn trajectory_shapes<'a>(
    samples: &[&'a NoDispatchObservation],
    regime_id: &str,
) -> Result<BTreeMap<&'a str, Vec<BaselineTrajectoryIntervalShape>>, BaselineQualificationError> {
    let mut grouped: BTreeMap<&str, Vec<&NoDispatchObservation>> = BTreeMap::new();
    for sample in samples {
        grouped
            .entry(sample.group_id.as_str())
            .or_default()
            .push(*sample);
    }

    let mut shapes = BTreeMap::new();
    for (group_id, mut group_samples) in grouped {
        group_samples.sort_by_key(|sample| sample.interval_start_ns);
        let origin_ns = group_samples
            .first()
            .ok_or(BaselineQualificationError::NoSharedOperatingRegimes)?
            .interval_start_ns;
        let mut previous_end_ns: Option<u64> = None;
        let mut shape = Vec::with_capacity(group_samples.len());
        for (index, sample) in group_samples.iter().enumerate() {
            if let Some(expected_start_ns) = previous_end_ns
                && sample.interval_start_ns != expected_start_ns
            {
                return Err(
                    BaselineQualificationError::NonContiguousBaselineTrajectory {
                        regime_id: regime_id.to_owned(),
                        group_id: group_id.to_owned(),
                        index,
                        expected_start_ns,
                        observed_start_ns: sample.interval_start_ns,
                    },
                );
            }
            let end_ns = sample
                .interval_start_ns
                .checked_add(sample.interval_duration_ns)
                .ok_or(BaselineQualificationError::TimestampOverflow)?;
            previous_end_ns = Some(end_ns);
            shape.push(BaselineTrajectoryIntervalShape {
                offset_from_group_start_ns: sample.interval_start_ns - origin_ns,
                duration_ns: sample.interval_duration_ns,
            });
        }
        shapes.insert(group_id, shape);
    }
    Ok(shapes)
}

fn mean(values: &[f64]) -> f64 {
    values.iter().sum::<f64>() / values.len() as f64
}

fn require_id(field: &'static str, value: &str) -> Result<(), BaselineQualificationError> {
    if value.trim().is_empty() {
        Err(BaselineQualificationError::MissingIdentity { field })
    } else {
        Ok(())
    }
}
