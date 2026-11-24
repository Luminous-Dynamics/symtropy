// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

//! Measurement-qualified PCC power and energy evidence.
//!
//! This crate is deliberately a metrology gate, not a dispatch controller,
//! grid-adequacy engine, meter driver, calibration laboratory, or standards
//! certification tool. It applies a pinned linear gain/offset correction,
//! propagates a declared covariance-aware standard-uncertainty budget, checks
//! calibration validity and operating bounds, aligns paired intervals, and
//! emits signed response/energy bounds.
//!
//! The caller must supply traceable calibration evidence, the joint covariance
//! assumptions, and a coverage factor appropriate to the validated distribution.
//! This v0.1 evaluator uses first-order (linearized) uncertainty propagation.
//! Nonlinear or non-Gaussian chains require a separately qualified distribution
//! propagation method and are not silently approximated here.
//!
//! Clock alignment is fail-closed: paired intervals must have identical
//! corrected physical boundaries, and the sum of the two declared clock
//! uncertainties must fit the contract. V0.1 does not interpolate, resample,
//! or attempt to reconstruct missing data.

use core::fmt;

use serde::Serialize;

const WATT_NANOSECONDS_PER_WATT_HOUR: f64 = 3_600_000_000_000.0;
const COVARIANCE_TOLERANCE: f64 = 1.0e-12;

/// Stable identities binding a reading to one installed PCC measurement chain.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct MeasurementChainIdentity {
    site_id: String,
    pcc_id: String,
    meter_id: String,
    configuration_digest: String,
    calibration_id: String,
    calibration_certificate_digest: String,
}

impl MeasurementChainIdentity {
    pub fn new(
        site_id: impl Into<String>,
        pcc_id: impl Into<String>,
        meter_id: impl Into<String>,
        configuration_digest: impl Into<String>,
        calibration_id: impl Into<String>,
        calibration_certificate_digest: impl Into<String>,
    ) -> Result<Self, PccMetrologyError> {
        let identity = Self {
            site_id: site_id.into(),
            pcc_id: pcc_id.into(),
            meter_id: meter_id.into(),
            configuration_digest: configuration_digest.into(),
            calibration_id: calibration_id.into(),
            calibration_certificate_digest: calibration_certificate_digest.into(),
        };
        for (field, value) in [
            ("site ID", identity.site_id.as_str()),
            ("PCC ID", identity.pcc_id.as_str()),
            ("meter ID", identity.meter_id.as_str()),
            (
                "configuration digest",
                identity.configuration_digest.as_str(),
            ),
            ("calibration ID", identity.calibration_id.as_str()),
            (
                "calibration certificate digest",
                identity.calibration_certificate_digest.as_str(),
            ),
        ] {
            require_id(field, value)?;
        }
        Ok(identity)
    }

    pub fn site_id(&self) -> &str {
        &self.site_id
    }
    pub fn pcc_id(&self) -> &str {
        &self.pcc_id
    }
    pub fn meter_id(&self) -> &str {
        &self.meter_id
    }
    pub fn configuration_digest(&self) -> &str {
        &self.configuration_digest
    }
    pub fn calibration_id(&self) -> &str {
        &self.calibration_id
    }
    pub fn calibration_certificate_digest(&self) -> &str {
        &self.calibration_certificate_digest
    }
}

/// Correlations between raw power, calibration offset and calibration gain.
///
/// The three correlations define a 3×3 correlation matrix. The constructor
/// rejects matrices that are not positive semidefinite; this prevents a
/// mathematically impossible covariance budget from manufacturing certainty.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CalibrationCorrelations {
    raw_offset: f64,
    raw_gain: f64,
    offset_gain: f64,
}

impl CalibrationCorrelations {
    pub fn new(
        raw_offset: f64,
        raw_gain: f64,
        offset_gain: f64,
    ) -> Result<Self, PccMetrologyError> {
        for (field, value) in [
            ("raw/offset correlation", raw_offset),
            ("raw/gain correlation", raw_gain),
            ("offset/gain correlation", offset_gain),
        ] {
            if !value.is_finite() || !(-1.0..=1.0).contains(&value) {
                return Err(PccMetrologyError::InvalidCorrelation { field, value });
            }
        }
        let determinant = 1.0 + 2.0 * raw_offset * raw_gain * offset_gain
            - raw_offset.powi(2)
            - raw_gain.powi(2)
            - offset_gain.powi(2);
        if determinant < -COVARIANCE_TOLERANCE {
            return Err(
                PccMetrologyError::NonPositiveSemidefiniteCalibrationCovariance { determinant },
            );
        }
        Ok(Self {
            raw_offset,
            raw_gain,
            offset_gain,
        })
    }

    pub const fn independent() -> Self {
        Self {
            raw_offset: 0.0,
            raw_gain: 0.0,
            offset_gain: 0.0,
        }
    }
}

/// Standard uncertainties for calibration gain and offset.
///
/// The raw-reading standard uncertainty is supplied with each meter reading.
/// Correlations are explicit; independence is never inferred from missing data.
#[derive(Debug, Clone, Copy, PartialEq, Serialize)]
pub struct CalibrationParameterUncertainty {
    gain_standard_uncertainty: f64,
    offset_standard_uncertainty_w: f64,
    correlations: CalibrationCorrelations,
}

impl CalibrationParameterUncertainty {
    pub fn new(
        gain_standard_uncertainty: f64,
        offset_standard_uncertainty_w: f64,
        correlations: CalibrationCorrelations,
    ) -> Result<Self, PccMetrologyError> {
        for (field, value) in [
            ("gain standard uncertainty", gain_standard_uncertainty),
            ("offset standard uncertainty", offset_standard_uncertainty_w),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(PccMetrologyError::InvalidQuantity { field, value });
            }
        }
        Ok(Self {
            gain_standard_uncertainty,
            offset_standard_uncertainty_w,
            correlations,
        })
    }

    pub const fn zero() -> Self {
        Self {
            gain_standard_uncertainty: 0.0,
            offset_standard_uncertainty_w: 0.0,
            correlations: CalibrationCorrelations::independent(),
        }
    }
}

/// Version-pinned linear active-power calibration model.
///
/// Corrected power is (raw power − offset) × gain. The valid interval and raw
/// operating range are explicit. A calibration receipt is not treated as
/// valid outside either bound.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct LinearPowerCalibration {
    calibration_id: String,
    gain: f64,
    offset_w: f64,
    uncertainty: CalibrationParameterUncertainty,
    valid_from_ns: u64,
    valid_until_ns: u64,
    minimum_raw_power_w: f64,
    maximum_raw_power_w: f64,
}

impl LinearPowerCalibration {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        calibration_id: impl Into<String>,
        gain: f64,
        offset_w: f64,
        uncertainty: CalibrationParameterUncertainty,
        valid_from_ns: u64,
        valid_until_ns: u64,
        minimum_raw_power_w: f64,
        maximum_raw_power_w: f64,
    ) -> Result<Self, PccMetrologyError> {
        let calibration_id = calibration_id.into();
        require_id("calibration ID", &calibration_id)?;
        if !gain.is_finite() || gain <= 0.0 {
            return Err(PccMetrologyError::InvalidQuantity {
                field: "calibration gain",
                value: gain,
            });
        }
        if !offset_w.is_finite() {
            return Err(PccMetrologyError::InvalidQuantity {
                field: "calibration offset",
                value: offset_w,
            });
        }
        if valid_from_ns >= valid_until_ns {
            return Err(PccMetrologyError::InvalidCalibrationWindow {
                valid_from_ns,
                valid_until_ns,
            });
        }
        if !minimum_raw_power_w.is_finite()
            || !maximum_raw_power_w.is_finite()
            || minimum_raw_power_w >= maximum_raw_power_w
        {
            return Err(PccMetrologyError::InvalidOperatingRange {
                minimum_raw_power_w,
                maximum_raw_power_w,
            });
        }
        Ok(Self {
            calibration_id,
            gain,
            offset_w,
            uncertainty,
            valid_from_ns,
            valid_until_ns,
            minimum_raw_power_w,
            maximum_raw_power_w,
        })
    }

    pub fn calibration_id(&self) -> &str {
        &self.calibration_id
    }
    pub const fn gain(&self) -> f64 {
        self.gain
    }
    pub const fn offset_w(&self) -> f64 {
        self.offset_w
    }
    pub const fn valid_from_ns(&self) -> u64 {
        self.valid_from_ns
    }
    pub const fn valid_until_ns(&self) -> u64 {
        self.valid_until_ns
    }

    fn correct(
        &self,
        raw_power_w: f64,
        raw_standard_uncertainty_w: f64,
    ) -> Result<CorrectedPower, PccMetrologyError> {
        if raw_power_w < self.minimum_raw_power_w || raw_power_w > self.maximum_raw_power_w {
            return Err(PccMetrologyError::OutsideCalibrationOperatingRange {
                raw_power_w,
                minimum_raw_power_w: self.minimum_raw_power_w,
                maximum_raw_power_w: self.maximum_raw_power_w,
            });
        }

        let difference = raw_power_w - self.offset_w;
        let gain = self.gain;
        let u_raw = raw_standard_uncertainty_w;
        let u_offset = self.uncertainty.offset_standard_uncertainty_w;
        let u_gain = self.uncertainty.gain_standard_uncertainty;
        let corr = self.uncertainty.correlations;

        // First-order propagation for P = (raw - offset) × gain:
        // Var(P) = J Σ Jᵀ, retaining all declared covariance terms.
        let variance = gain.powi(2) * u_raw.powi(2)
            + gain.powi(2) * u_offset.powi(2)
            + difference.powi(2) * u_gain.powi(2)
            - 2.0 * gain.powi(2) * corr.raw_offset * u_raw * u_offset
            + 2.0 * gain * difference * corr.raw_gain * u_raw * u_gain
            - 2.0 * gain * difference * corr.offset_gain * u_offset * u_gain;

        let scale = (gain.powi(2) * u_raw.powi(2)
            + gain.powi(2) * u_offset.powi(2)
            + difference.powi(2) * u_gain.powi(2))
        .max(1.0);
        if !variance.is_finite() || variance < -COVARIANCE_TOLERANCE * scale {
            return Err(PccMetrologyError::InvalidPropagatedVariance { variance });
        }
        let corrected_power_w = difference * gain;
        if !corrected_power_w.is_finite() {
            return Err(PccMetrologyError::NumericalOverflow("corrected power"));
        }
        Ok(CorrectedPower {
            power_w: corrected_power_w,
            standard_uncertainty_w: variance.max(0.0).sqrt(),
        })
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
struct CorrectedPower {
    power_w: f64,
    standard_uncertainty_w: f64,
}

/// Data-quality state for one reading. Only Qualified readings can contribute
/// to a metrology-qualified trace; other states are explicit abstentions.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum ReadingQuality {
    Qualified,
    Missing,
    Clipped,
    Saturated,
    Stale,
    MeterReset,
    UnsupportedOperatingCondition,
}

/// One interval-average PCC active-power reading.
///
/// Power is signed: positive import and negative export are preserved. The
/// timestamp is the corrected physical interval start, not a transport time.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct MeterReading {
    interval_start_ns: u64,
    interval_duration_ns: u64,
    raw_power_w: f64,
    raw_standard_uncertainty_w: f64,
    quality: ReadingQuality,
    evidence_id: String,
}

impl MeterReading {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        interval_start_ns: u64,
        interval_duration_ns: u64,
        raw_power_w: f64,
        raw_standard_uncertainty_w: f64,
        quality: ReadingQuality,
        evidence_id: impl Into<String>,
    ) -> Result<Self, PccMetrologyError> {
        if interval_duration_ns == 0 {
            return Err(PccMetrologyError::InvalidIntervalDuration {
                interval_duration_ns,
            });
        }
        if !raw_power_w.is_finite() {
            return Err(PccMetrologyError::InvalidQuantity {
                field: "raw power",
                value: raw_power_w,
            });
        }
        if !raw_standard_uncertainty_w.is_finite() || raw_standard_uncertainty_w < 0.0 {
            return Err(PccMetrologyError::InvalidQuantity {
                field: "raw standard uncertainty",
                value: raw_standard_uncertainty_w,
            });
        }
        let evidence_id = evidence_id.into();
        require_id("meter reading evidence ID", &evidence_id)?;
        interval_start_ns
            .checked_add(interval_duration_ns)
            .ok_or(PccMetrologyError::TimestampOverflow)?;
        Ok(Self {
            interval_start_ns,
            interval_duration_ns,
            raw_power_w,
            raw_standard_uncertainty_w,
            quality,
            evidence_id,
        })
    }

    pub const fn interval_start_ns(&self) -> u64 {
        self.interval_start_ns
    }
    pub const fn interval_duration_ns(&self) -> u64 {
        self.interval_duration_ns
    }
    pub const fn raw_power_w(&self) -> f64 {
        self.raw_power_w
    }
    pub const fn quality(&self) -> ReadingQuality {
        self.quality
    }
    pub fn evidence_id(&self) -> &str {
        &self.evidence_id
    }
}

/// A baseline/candidate pair with explicit correlation of corrected-power
/// measurement errors. This correlation is a source-backed input, not guessed.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PairedPowerInterval {
    baseline: MeterReading,
    candidate: MeterReading,
    corrected_error_correlation: f64,
}

impl PairedPowerInterval {
    pub fn new(
        baseline: MeterReading,
        candidate: MeterReading,
        corrected_error_correlation: f64,
    ) -> Result<Self, PccMetrologyError> {
        if !corrected_error_correlation.is_finite()
            || !(-1.0..=1.0).contains(&corrected_error_correlation)
        {
            return Err(PccMetrologyError::InvalidCorrelation {
                field: "paired baseline/candidate error correlation",
                value: corrected_error_correlation,
            });
        }
        if baseline.evidence_id == candidate.evidence_id {
            return Err(PccMetrologyError::EvidenceReferenceCollision {
                evidence_id: baseline.evidence_id.clone(),
            });
        }
        Ok(Self {
            baseline,
            candidate,
            corrected_error_correlation,
        })
    }
}

/// Clock uncertainty for the two paired traces.
///
/// V0.1 requires samples to already be expressed on one corrected physical
/// time axis. The alignment-model ID and both uncertainty bounds are retained.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct ClockAlignmentEvidence {
    baseline_clock_uncertainty_ns: u64,
    candidate_clock_uncertainty_ns: u64,
    baseline_maximum_absolute_power_slew_w_per_s: Option<f64>,
    candidate_maximum_absolute_power_slew_w_per_s: Option<f64>,
    alignment_model_id: String,
}

impl ClockAlignmentEvidence {
    pub fn new(
        baseline_clock_uncertainty_ns: u64,
        candidate_clock_uncertainty_ns: u64,
        alignment_model_id: impl Into<String>,
    ) -> Result<Self, PccMetrologyError> {
        let alignment_model_id = alignment_model_id.into();
        require_id("clock alignment model ID", &alignment_model_id)?;
        baseline_clock_uncertainty_ns
            .checked_add(candidate_clock_uncertainty_ns)
            .ok_or(PccMetrologyError::TimestampOverflow)?;
        Ok(Self {
            baseline_clock_uncertainty_ns,
            candidate_clock_uncertainty_ns,
            baseline_maximum_absolute_power_slew_w_per_s: None,
            candidate_maximum_absolute_power_slew_w_per_s: None,
            alignment_model_id,
        })
    }

    /// Attach externally validated conservative absolute power-slew bounds.
    ///
    /// The evaluator converts each clock-time uncertainty into a deterministic
    /// power-error bound and adds both sides conservatively. Bounds must apply
    /// to the complete event window and measurement boundary.
    pub fn with_maximum_power_slew_bounds(
        mut self,
        baseline_w_per_s: f64,
        candidate_w_per_s: f64,
    ) -> Result<Self, PccMetrologyError> {
        for (field, value) in [
            ("baseline maximum absolute power slew", baseline_w_per_s),
            ("candidate maximum absolute power slew", candidate_w_per_s),
        ] {
            if !value.is_finite() || value < 0.0 {
                return Err(PccMetrologyError::InvalidQuantity { field, value });
            }
        }
        self.baseline_maximum_absolute_power_slew_w_per_s = Some(baseline_w_per_s);
        self.candidate_maximum_absolute_power_slew_w_per_s = Some(candidate_w_per_s);
        Ok(self)
    }

    pub const fn combined_uncertainty_ns(&self) -> u64 {
        // The constructor already rejects overflow.
        self.baseline_clock_uncertainty_ns + self.candidate_clock_uncertainty_ns
    }

    pub fn alignment_model_id(&self) -> &str {
        &self.alignment_model_id
    }
}

/// Product/evidence-specific metrology acceptance policy.
///
/// coverage_factor is a caller-selected multiplier for the propagated
/// standard uncertainty. The report explicitly avoids asserting a universal
/// confidence probability or treating k=2 as universally valid.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PccMetrologyContract {
    coverage_factor: f64,
    minimum_response_w: f64,
    maximum_clock_uncertainty_ns: u64,
    maximum_relative_expanded_uncertainty: f64,
    minimum_qualifying_interval_fraction: f64,
}

impl PccMetrologyContract {
    pub fn new(
        coverage_factor: f64,
        minimum_response_w: f64,
        maximum_clock_uncertainty_ns: u64,
        maximum_relative_expanded_uncertainty: f64,
        minimum_qualifying_interval_fraction: f64,
    ) -> Result<Self, PccMetrologyError> {
        for (field, value) in [
            ("coverage factor", coverage_factor),
            ("minimum response", minimum_response_w),
            (
                "maximum relative expanded uncertainty",
                maximum_relative_expanded_uncertainty,
            ),
            (
                "minimum qualifying interval fraction",
                minimum_qualifying_interval_fraction,
            ),
        ] {
            if !value.is_finite() || value <= 0.0 {
                return Err(PccMetrologyError::InvalidQuantity { field, value });
            }
        }
        if !(0.0..=1.0).contains(&maximum_relative_expanded_uncertainty)
            || !(0.0..=1.0).contains(&minimum_qualifying_interval_fraction)
        {
            return Err(PccMetrologyError::InvalidFraction {
                field: "metrology contract fraction",
                value: maximum_relative_expanded_uncertainty
                    .max(minimum_qualifying_interval_fraction),
            });
        }
        Ok(Self {
            coverage_factor,
            minimum_response_w,
            maximum_clock_uncertainty_ns,
            maximum_relative_expanded_uncertainty,
            minimum_qualifying_interval_fraction,
        })
    }

    pub const fn coverage_factor(&self) -> f64 {
        self.coverage_factor
    }
    pub const fn minimum_response_w(&self) -> f64 {
        self.minimum_response_w
    }
}

/// Interval-level corrected values, uncertainty and signed energy receipt.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PccMetrologyIntervalReceipt {
    pub interval_start_ns: u64,
    pub interval_duration_ns: u64,
    pub baseline_corrected_power_w: f64,
    pub candidate_corrected_power_w: f64,
    pub baseline_standard_uncertainty_w: f64,
    pub candidate_standard_uncertainty_w: f64,
    pub corrected_error_correlation: f64,
    pub measured_reduction_w: f64,
    pub reduction_standard_uncertainty_w: f64,
    pub expanded_uncertainty_w: f64,
    /// Deterministic power-error allowance induced by clock-time uncertainty
    /// and the caller-supplied conservative absolute power-slew bounds.
    pub time_alignment_uncertainty_bound_w: f64,
    pub conservative_reduction_lower_bound_w: f64,
    pub relative_expanded_uncertainty: f64,
    pub response_threshold_pass: bool,
    pub expanded_uncertainty_pass: bool,
    pub interval_qualified: bool,
    pub measured_reduction_energy_wh: f64,
    pub lower_bound_reduction_energy_wh: f64,
    pub baseline_evidence_id: String,
    pub candidate_evidence_id: String,
}

/// Replayable result for a pinned PCC chain and paired trace.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct PccMetrologyReport {
    pub identity: MeasurementChainIdentity,
    pub calibration_id: String,
    pub calibration_certificate_digest: String,
    pub clock_alignment_model_id: String,
    pub baseline_clock_uncertainty_ns: u64,
    pub candidate_clock_uncertainty_ns: u64,
    pub combined_clock_uncertainty_ns: u64,
    pub baseline_maximum_absolute_power_slew_w_per_s: f64,
    pub candidate_maximum_absolute_power_slew_w_per_s: f64,
    pub coverage_factor: f64,
    pub coverage_assumption: String,
    pub minimum_response_w: f64,
    pub interval_count: usize,
    pub qualifying_interval_count: usize,
    pub qualifying_interval_fraction: f64,
    pub gross_reduction_energy_wh: f64,
    pub gross_rebound_energy_wh: f64,
    pub net_reduction_energy_wh: f64,
    /// Sum of interval lower-bound energy contributions. This is not a joint
    /// confidence bound unless the cross-interval dependence is also qualified.
    pub summed_interval_lower_bound_energy_wh: f64,
    pub metrology_qualified: bool,
    pub sub_interval_transients_unverified: bool,
    pub intervals: Vec<PccMetrologyIntervalReceipt>,
}

/// Fail-closed input or evaluation errors.
#[derive(Debug, Clone, PartialEq)]
pub enum PccMetrologyError {
    MissingIdentity {
        field: &'static str,
    },
    InvalidQuantity {
        field: &'static str,
        value: f64,
    },
    InvalidFraction {
        field: &'static str,
        value: f64,
    },
    InvalidCorrelation {
        field: &'static str,
        value: f64,
    },
    NonPositiveSemidefiniteCalibrationCovariance {
        determinant: f64,
    },
    InvalidCalibrationWindow {
        valid_from_ns: u64,
        valid_until_ns: u64,
    },
    InvalidOperatingRange {
        minimum_raw_power_w: f64,
        maximum_raw_power_w: f64,
    },
    InvalidIntervalDuration {
        interval_duration_ns: u64,
    },
    InvalidIntervalCount,
    EmptyTrace,
    CalibrationIdMismatch {
        chain_calibration_id: String,
        model_calibration_id: String,
    },
    CalibrationNotValid {
        interval_start_ns: u64,
        interval_end_ns: u64,
        valid_from_ns: u64,
        valid_until_ns: u64,
    },
    OutsideCalibrationOperatingRange {
        raw_power_w: f64,
        minimum_raw_power_w: f64,
        maximum_raw_power_w: f64,
    },
    ReadingIneligible {
        evidence_id: String,
        quality: ReadingQuality,
    },
    InvalidPropagatedVariance {
        variance: f64,
    },
    ClockUncertaintyExceedsContract {
        combined_uncertainty_ns: u64,
        maximum_ns: u64,
    },
    MissingClockSlewBounds {
        baseline_clock_uncertainty_ns: u64,
        candidate_clock_uncertainty_ns: u64,
    },
    PairedTimestampMismatch {
        index: usize,
        baseline_start_ns: u64,
        candidate_start_ns: u64,
        baseline_duration_ns: u64,
        candidate_duration_ns: u64,
    },
    NonContiguousIntervals {
        index: usize,
        expected_start_ns: u64,
        observed_start_ns: u64,
    },
    EvidenceReferenceCollision {
        evidence_id: String,
    },
    TimestampOverflow,
    NumericalOverflow(&'static str),
}

impl fmt::Display for PccMetrologyError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::MissingIdentity { field } => write!(f, "{field} must not be empty"),
            Self::InvalidQuantity { field, value } => write!(
                f,
                "{field} must be finite and positive/in-range; got {value}"
            ),
            Self::InvalidFraction { field, value } => {
                write!(f, "{field} must be in [0, 1]; got {value}")
            }
            Self::InvalidCorrelation { field, value } => {
                write!(f, "{field} must be in [-1, 1]; got {value}")
            }
            Self::NonPositiveSemidefiniteCalibrationCovariance { determinant } => write!(
                f,
                "calibration correlation matrix is not positive semidefinite (determinant {determinant})"
            ),
            Self::InvalidCalibrationWindow {
                valid_from_ns,
                valid_until_ns,
            } => write!(
                f,
                "invalid calibration validity window [{valid_from_ns}, {valid_until_ns})"
            ),
            Self::InvalidOperatingRange {
                minimum_raw_power_w,
                maximum_raw_power_w,
            } => write!(
                f,
                "invalid calibration operating range [{minimum_raw_power_w}, {maximum_raw_power_w}] W"
            ),
            Self::InvalidIntervalDuration {
                interval_duration_ns,
            } => write!(
                f,
                "interval duration must be nonzero; got {interval_duration_ns} ns"
            ),
            Self::InvalidIntervalCount => {
                write!(f, "minimum qualifying interval count cannot be represented")
            }
            Self::EmptyTrace => write!(
                f,
                "PCC metrology evaluation requires at least one paired interval"
            ),
            Self::CalibrationIdMismatch {
                chain_calibration_id,
                model_calibration_id,
            } => write!(
                f,
                "chain calibration ID {chain_calibration_id} does not match model calibration ID {model_calibration_id}"
            ),
            Self::CalibrationNotValid {
                interval_start_ns,
                interval_end_ns,
                valid_from_ns,
                valid_until_ns,
            } => write!(
                f,
                "calibration valid [{valid_from_ns}, {valid_until_ns}) does not cover reading interval [{interval_start_ns}, {interval_end_ns})"
            ),
            Self::OutsideCalibrationOperatingRange {
                raw_power_w,
                minimum_raw_power_w,
                maximum_raw_power_w,
            } => write!(
                f,
                "raw power {raw_power_w} W is outside calibrated range [{minimum_raw_power_w}, {maximum_raw_power_w}] W"
            ),
            Self::ReadingIneligible {
                evidence_id,
                quality,
            } => write!(
                f,
                "reading {evidence_id} is ineligible due to quality state {quality:?}"
            ),
            Self::InvalidPropagatedVariance { variance } => {
                write!(f, "propagated variance is invalid: {variance}")
            }
            Self::ClockUncertaintyExceedsContract {
                combined_uncertainty_ns,
                maximum_ns,
            } => write!(
                f,
                "combined paired clock uncertainty {combined_uncertainty_ns} ns exceeds contract maximum {maximum_ns} ns"
            ),
            Self::MissingClockSlewBounds {
                baseline_clock_uncertainty_ns,
                candidate_clock_uncertainty_ns,
            } => write!(
                f,
                "clock-time uncertainty ({baseline_clock_uncertainty_ns} ns baseline, {candidate_clock_uncertainty_ns} ns candidate) requires externally validated absolute power-slew bounds"
            ),
            Self::PairedTimestampMismatch {
                index,
                baseline_start_ns,
                candidate_start_ns,
                baseline_duration_ns,
                candidate_duration_ns,
            } => write!(
                f,
                "paired interval {index} time boundaries differ: baseline {baseline_start_ns}+{baseline_duration_ns} ns, candidate {candidate_start_ns}+{candidate_duration_ns} ns"
            ),
            Self::NonContiguousIntervals {
                index,
                expected_start_ns,
                observed_start_ns,
            } => write!(
                f,
                "interval {index} starts at {observed_start_ns} ns; contiguous chronology requires {expected_start_ns} ns"
            ),
            Self::EvidenceReferenceCollision { evidence_id } => {
                write!(f, "baseline and candidate reuse evidence ID {evidence_id}")
            }
            Self::TimestampOverflow => write!(f, "timestamp arithmetic overflow"),
            Self::NumericalOverflow(field) => write!(f, "numerical overflow computing {field}"),
        }
    }
}

impl std::error::Error for PccMetrologyError {}

/// Evaluate a paired baseline/candidate PCC trace.
///
/// The result preserves negative delivery as rebound; it never clamps signed
/// response to zero. Each interval is independently checked against the
/// declared minimum response and relative expanded-uncertainty limits.
pub fn evaluate_pcc_metrology(
    identity: &MeasurementChainIdentity,
    calibration: &LinearPowerCalibration,
    contract: &PccMetrologyContract,
    clock: &ClockAlignmentEvidence,
    intervals: &[PairedPowerInterval],
) -> Result<PccMetrologyReport, PccMetrologyError> {
    if intervals.is_empty() {
        return Err(PccMetrologyError::EmptyTrace);
    }
    if identity.calibration_id != calibration.calibration_id {
        return Err(PccMetrologyError::CalibrationIdMismatch {
            chain_calibration_id: identity.calibration_id.clone(),
            model_calibration_id: calibration.calibration_id.clone(),
        });
    }
    let combined_clock_uncertainty_ns = clock.combined_uncertainty_ns();
    if combined_clock_uncertainty_ns > contract.maximum_clock_uncertainty_ns {
        return Err(PccMetrologyError::ClockUncertaintyExceedsContract {
            combined_uncertainty_ns: combined_clock_uncertainty_ns,
            maximum_ns: contract.maximum_clock_uncertainty_ns,
        });
    }
    let (baseline_slew_bound, candidate_slew_bound) = match (
        clock.baseline_maximum_absolute_power_slew_w_per_s,
        clock.candidate_maximum_absolute_power_slew_w_per_s,
    ) {
        (Some(baseline), Some(candidate)) => (baseline, candidate),
        _ if combined_clock_uncertainty_ns == 0 => (0.0, 0.0),
        _ => {
            return Err(PccMetrologyError::MissingClockSlewBounds {
                baseline_clock_uncertainty_ns: clock.baseline_clock_uncertainty_ns,
                candidate_clock_uncertainty_ns: clock.candidate_clock_uncertainty_ns,
            });
        }
    };

    let mut receipts = Vec::with_capacity(intervals.len());
    let mut gross_reduction_energy_wh = 0.0;
    let mut gross_rebound_energy_wh = 0.0;
    let mut net_reduction_energy_wh = 0.0;
    let mut summed_interval_lower_bound_energy_wh = 0.0;
    let mut qualifying_interval_count = 0usize;
    let mut previous_end_ns: Option<u64> = None;

    for (index, pair) in intervals.iter().enumerate() {
        let baseline = &pair.baseline;
        let candidate = &pair.candidate;
        if baseline.interval_start_ns != candidate.interval_start_ns
            || baseline.interval_duration_ns != candidate.interval_duration_ns
        {
            return Err(PccMetrologyError::PairedTimestampMismatch {
                index,
                baseline_start_ns: baseline.interval_start_ns,
                candidate_start_ns: candidate.interval_start_ns,
                baseline_duration_ns: baseline.interval_duration_ns,
                candidate_duration_ns: candidate.interval_duration_ns,
            });
        }
        if let Some(expected_start_ns) = previous_end_ns
            && baseline.interval_start_ns != expected_start_ns
        {
            return Err(PccMetrologyError::NonContiguousIntervals {
                index,
                expected_start_ns,
                observed_start_ns: baseline.interval_start_ns,
            });
        }
        let interval_end_ns = baseline
            .interval_start_ns
            .checked_add(baseline.interval_duration_ns)
            .ok_or(PccMetrologyError::TimestampOverflow)?;
        previous_end_ns = Some(interval_end_ns);

        for reading in [baseline, candidate] {
            if reading.quality != ReadingQuality::Qualified {
                return Err(PccMetrologyError::ReadingIneligible {
                    evidence_id: reading.evidence_id.clone(),
                    quality: reading.quality,
                });
            }
            if reading.interval_start_ns < calibration.valid_from_ns
                || interval_end_ns > calibration.valid_until_ns
            {
                return Err(PccMetrologyError::CalibrationNotValid {
                    interval_start_ns: reading.interval_start_ns,
                    interval_end_ns,
                    valid_from_ns: calibration.valid_from_ns,
                    valid_until_ns: calibration.valid_until_ns,
                });
            }
        }

        let baseline_corrected =
            calibration.correct(baseline.raw_power_w, baseline.raw_standard_uncertainty_w)?;
        let candidate_corrected =
            calibration.correct(candidate.raw_power_w, candidate.raw_standard_uncertainty_w)?;

        let measured_reduction_w = baseline_corrected.power_w - candidate_corrected.power_w;
        let response_variance = baseline_corrected.standard_uncertainty_w.powi(2)
            + candidate_corrected.standard_uncertainty_w.powi(2)
            - 2.0
                * pair.corrected_error_correlation
                * baseline_corrected.standard_uncertainty_w
                * candidate_corrected.standard_uncertainty_w;
        let variance_scale = (baseline_corrected.standard_uncertainty_w.powi(2)
            + candidate_corrected.standard_uncertainty_w.powi(2))
        .max(1.0);
        if !response_variance.is_finite()
            || response_variance < -COVARIANCE_TOLERANCE * variance_scale
        {
            return Err(PccMetrologyError::InvalidPropagatedVariance {
                variance: response_variance,
            });
        }
        let reduction_standard_uncertainty_w = response_variance.max(0.0).sqrt();
        // Timing uncertainty is not folded into a Gaussian RSS budget. It is
        // a deterministic worst-case bound: max|dP/dt| × max|clock error|,
        // summed across baseline and candidate to avoid assuming cancellation.
        let baseline_time_bound_w =
            baseline_slew_bound * clock.baseline_clock_uncertainty_ns as f64 / 1_000_000_000.0;
        let candidate_time_bound_w =
            candidate_slew_bound * clock.candidate_clock_uncertainty_ns as f64 / 1_000_000_000.0;
        let time_alignment_uncertainty_bound_w = baseline_time_bound_w + candidate_time_bound_w;
        if !time_alignment_uncertainty_bound_w.is_finite() {
            return Err(PccMetrologyError::NumericalOverflow(
                "clock-time power uncertainty",
            ));
        }
        let expanded_uncertainty_w = contract.coverage_factor * reduction_standard_uncertainty_w
            + time_alignment_uncertainty_bound_w;
        let conservative_reduction_lower_bound_w = measured_reduction_w - expanded_uncertainty_w;
        let relative_expanded_uncertainty = if measured_reduction_w.abs() > f64::EPSILON {
            expanded_uncertainty_w / measured_reduction_w.abs()
        } else {
            f64::INFINITY
        };
        let response_threshold_pass =
            conservative_reduction_lower_bound_w >= contract.minimum_response_w;
        let expanded_uncertainty_pass =
            relative_expanded_uncertainty <= contract.maximum_relative_expanded_uncertainty;
        let interval_qualified = response_threshold_pass && expanded_uncertainty_pass;
        qualifying_interval_count += usize::from(interval_qualified);

        let duration = baseline.interval_duration_ns as f64;
        let measured_reduction_energy_wh =
            measured_reduction_w * duration / WATT_NANOSECONDS_PER_WATT_HOUR;
        let lower_bound_reduction_energy_wh =
            conservative_reduction_lower_bound_w * duration / WATT_NANOSECONDS_PER_WATT_HOUR;
        if !measured_reduction_energy_wh.is_finite() || !lower_bound_reduction_energy_wh.is_finite()
        {
            return Err(PccMetrologyError::NumericalOverflow("interval energy"));
        }
        if measured_reduction_energy_wh >= 0.0 {
            gross_reduction_energy_wh += measured_reduction_energy_wh;
        } else {
            gross_rebound_energy_wh += -measured_reduction_energy_wh;
        }
        net_reduction_energy_wh += measured_reduction_energy_wh;
        summed_interval_lower_bound_energy_wh += lower_bound_reduction_energy_wh;

        receipts.push(PccMetrologyIntervalReceipt {
            interval_start_ns: baseline.interval_start_ns,
            interval_duration_ns: baseline.interval_duration_ns,
            baseline_corrected_power_w: baseline_corrected.power_w,
            candidate_corrected_power_w: candidate_corrected.power_w,
            baseline_standard_uncertainty_w: baseline_corrected.standard_uncertainty_w,
            candidate_standard_uncertainty_w: candidate_corrected.standard_uncertainty_w,
            corrected_error_correlation: pair.corrected_error_correlation,
            measured_reduction_w,
            reduction_standard_uncertainty_w,
            expanded_uncertainty_w,
            time_alignment_uncertainty_bound_w,
            conservative_reduction_lower_bound_w,
            relative_expanded_uncertainty,
            response_threshold_pass,
            expanded_uncertainty_pass,
            interval_qualified,
            measured_reduction_energy_wh,
            lower_bound_reduction_energy_wh,
            baseline_evidence_id: baseline.evidence_id.clone(),
            candidate_evidence_id: candidate.evidence_id.clone(),
        });
    }

    let qualifying_interval_fraction = qualifying_interval_count as f64 / intervals.len() as f64;
    Ok(PccMetrologyReport {
        identity: identity.clone(),
        calibration_id: calibration.calibration_id.clone(),
        calibration_certificate_digest: identity.calibration_certificate_digest.clone(),
        clock_alignment_model_id: clock.alignment_model_id.clone(),
        baseline_clock_uncertainty_ns: clock.baseline_clock_uncertainty_ns,
        candidate_clock_uncertainty_ns: clock.candidate_clock_uncertainty_ns,
        combined_clock_uncertainty_ns,
        baseline_maximum_absolute_power_slew_w_per_s: baseline_slew_bound,
        candidate_maximum_absolute_power_slew_w_per_s: candidate_slew_bound,
        coverage_factor: contract.coverage_factor,
        coverage_assumption: "caller-specified coverage factor applied to first-order covariance-aware electrical standard uncertainty, plus a deterministic max-slew × clock-error bound; no universal confidence probability asserted".to_owned(),
        minimum_response_w: contract.minimum_response_w,
        interval_count: intervals.len(),
        qualifying_interval_count,
        qualifying_interval_fraction,
        gross_reduction_energy_wh,
        gross_rebound_energy_wh,
        net_reduction_energy_wh,
        summed_interval_lower_bound_energy_wh,
        metrology_qualified: qualifying_interval_fraction
            >= contract.minimum_qualifying_interval_fraction,
        sub_interval_transients_unverified: true,
        intervals: receipts,
    })
}

fn require_id(field: &'static str, value: &str) -> Result<(), PccMetrologyError> {
    if value.trim().is_empty() {
        Err(PccMetrologyError::MissingIdentity { field })
    } else {
        Ok(())
    }
}
