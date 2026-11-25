//! Evidence-bounded functional metrics for multipole magnetic encoder field traces.
//!
//! The evaluator measures the electrical phase represented by the in-plane field
//! vector. It does not model a sensor ASIC or infer manufacturing probability.

use std::collections::HashSet;
use std::f64::consts::{PI, TAU};

const ANGLE_GRID_TOLERANCE_RAD: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    SyntheticScenario,
    PhysicalBuild,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldSample {
    /// Mechanical reference angle, monotonically increasing over one revolution.
    pub mechanical_angle_rad: f64,
    /// Sensor-plane field components in tesla.
    pub bx_t: f64,
    pub by_t: f64,
    pub bz_t: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldTrace {
    pub trace_id: String,
    /// Opaque caller-supplied coordinate-frame identity.
    pub frame_id: String,
    /// Opaque identity for the matched sensor/air-gap/environment condition.
    pub operating_condition_id: String,
    /// Opaque caller-supplied full-revolution angular-grid identity.
    pub grid_id: String,
    pub evidence_kind: EvidenceKind,
    pub samples: Vec<FieldSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderRequirements {
    pub pole_pairs: u32,
    /// Direction of electrical phase progression as mechanical angle increases: +1 or -1.
    pub phase_rotation_sense: i8,
    /// Identity of the independent calibration record supplying the phase zero.
    pub phase_zero_calibration_id: String,
    /// Zero-phase calibration from that independent record; never fit on the trace under test.
    pub phase_zero_offset_rad: f64,
    pub minimum_in_plane_field_t: f64,
    pub maximum_peak_phase_error_rad: f64,
    pub maximum_rms_phase_error_rad: f64,
    pub maximum_amplitude_ripple_fraction: f64,
    pub maximum_rms_cross_axis_ratio: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RequirementViolation {
    BelowMinimumInPlaneField,
    PeakPhaseErrorExceeded,
    RmsPhaseErrorExceeded,
    AmplitudeRippleExceeded,
    CrossAxisRatioExceeded,
    InPlaneSignalUnavailable,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldCovariance2d {
    /// Variance of Bx, in tesla squared.
    pub xx_t2: f64,
    /// Covariance of Bx and By, in tesla squared.
    pub xy_t2: f64,
    /// Variance of By, in tesla squared.
    pub yy_t2: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct FieldCovarianceSample {
    pub mechanical_angle_rad: f64,
    pub covariance: FieldCovariance2d,
}

#[derive(Clone, Debug, PartialEq)]
pub struct FieldCovarianceTrace {
    pub trace_id: String,
    pub frame_id: String,
    pub operating_condition_id: String,
    pub grid_id: String,
    pub samples: Vec<FieldCovarianceSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct AngularReferenceUncertainty {
    /// Identity of the record supplying the angular uncertainty budget.
    pub uncertainty_record_id: String,
    /// Must match EncoderRequirements.phase_zero_calibration_id.
    pub phase_zero_calibration_id: String,
    /// Standard uncertainty of the independent phase-zero calibration, radians.
    pub phase_zero_std_rad: f64,
    /// Standard uncertainty of each mechanical angle reference sample, radians.
    pub mechanical_angle_std_rad: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PhaseUncertaintyMetrics {
    pub trace_id: String,
    pub sample_count: usize,
    pub uncertainty_record_id: String,
    pub phase_zero_calibration_id: String,
    pub measurement_phase_std_mean_rad: f64,
    pub measurement_phase_std_rms_rad: f64,
    pub measurement_phase_std_p95_rad: f64,
    pub measurement_phase_std_max_rad: f64,
    pub reference_phase_zero_std_rad: f64,
    /// Pole-pair-scaled contribution p * sigma_theta_reference.
    pub reference_mechanical_phase_std_rad: f64,
    pub total_phase_std_mean_rad: f64,
    pub total_phase_std_rms_rad: f64,
    pub total_phase_std_p95_rad: f64,
    pub total_phase_std_max_rad: f64,
    /// Descriptive normalized residuals only; not a p-value or coverage test.
    pub rms_phase_error_over_total_std: Option<f64>,
    pub maximum_phase_error_over_total_std: Option<f64>,
    pub zero_uncertainty_nonzero_residual_sample_count: usize,
}


#[derive(Clone, Debug, PartialEq)]
pub struct OperatingConditionScenarioSetMetrics {
    pub set_id: String,
    pub evidence_kind: EvidenceKind,
    pub frame_id: String,
    pub grid_id: String,
    pub scenario_count: usize,
    pub accepted_scenario_count: usize,
    pub rejected_scenario_count: usize,
    pub all_scenarios_accepted: bool,
    pub worst_peak_phase_error_rad: f64,
    pub worst_rms_phase_error_rad: f64,
    pub worst_p95_phase_error_rad: f64,
    pub minimum_in_plane_field_t: f64,
    pub worst_amplitude_ripple_fraction: Option<f64>,
    pub worst_rms_cross_axis_ratio: Option<f64>,
    pub worst_below_threshold_sample_count: usize,
    pub scenarios: Vec<TraceMetrics>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum EvaluationError {
    InvalidMetadata,
    InvalidRequirements,
    InvalidReferenceUncertainty,
    CovarianceTraceIdentityMismatch,
    CovarianceTraceFrameMismatch,
    CovarianceTraceConditionMismatch,
    CovarianceTraceGridMismatch,
    NonFiniteFieldCovariance { index: usize },
    NonPositiveSemidefiniteFieldCovariance { index: usize },
    FieldBelowMinimumForPhase { index: usize },
    PhaseUncertaintyNumericOverflow { index: usize },
    ZeroInPlaneFieldForPhase { index: usize },
    TooFewSamples,
    NonFiniteSample { index: usize },
    NonMonotonicAngleGrid { index: usize },
    NonUniformOrIncompleteRevolution { index: usize },
    NumericOverflow,
    EmptyTraceSet,
    DuplicateTraceId,
    MixedEvidenceKinds,
    FrameIdentityMismatch,
    GridIdentityMismatch,
    OperatingConditionMismatch,
    CoordinateGridMismatch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraceMetrics {
    pub trace_id: String,
    pub sample_count: usize,
    pub minimum_in_plane_field_t: f64,
    pub mean_in_plane_field_t: f64,
    pub maximum_in_plane_field_t: f64,
    pub amplitude_ripple_fraction: Option<f64>,
    pub rms_phase_error_rad: f64,
    pub p95_absolute_phase_error_rad: f64,
    pub maximum_absolute_phase_error_rad: f64,
    pub rms_cross_axis_ratio: Option<f64>,
    pub below_threshold_sample_count: usize,
    pub accepted: bool,
    pub violations: Vec<RequirementViolation>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TraceSetMetrics {
    pub evidence_kind: EvidenceKind,
    pub trace_count: usize,
    pub passing_trace_count: usize,
    /// True only when every supplied trace satisfies every hard requirement.
    pub all_traces_accepted: bool,
    pub minimum_in_plane_field_t: f64,
    pub worst_peak_phase_error_rad: f64,
    pub worst_rms_phase_error_rad: f64,
    pub worst_p95_absolute_phase_error_rad: f64,
    /// None means at least one trace had no defined in-plane amplitude ripple.
    pub worst_amplitude_ripple_fraction: Option<f64>,
    /// None means at least one trace had no defined in-plane reference signal.
    pub worst_rms_cross_axis_ratio: Option<f64>,
    pub rejected_trace_ids: Vec<String>,
}

fn validate_requirements(requirements: &EncoderRequirements) -> Result<(), EvaluationError> {
    let finite_nonnegative = [
        requirements.maximum_peak_phase_error_rad,
        requirements.maximum_rms_phase_error_rad,
        requirements.maximum_amplitude_ripple_fraction,
        requirements.maximum_rms_cross_axis_ratio,
    ]
    .into_iter()
    .all(|value| value.is_finite() && value >= 0.0);

    if requirements.pole_pairs == 0
        || !matches!(requirements.phase_rotation_sense, -1 | 1)
        || requirements.phase_zero_calibration_id.trim().is_empty()
        || !requirements.phase_zero_offset_rad.is_finite()
        || !requirements.minimum_in_plane_field_t.is_finite()
        || requirements.minimum_in_plane_field_t <= 0.0
        || !finite_nonnegative
    {
        return Err(EvaluationError::InvalidRequirements);
    }
    Ok(())
}

fn wrap_to_pi(angle: f64) -> f64 {
    (angle + PI).rem_euclid(TAU) - PI
}

fn rms(values: &[f64]) -> f64 {
    values
        .iter()
        .fold(0.0_f64, |accumulator, value| accumulator.hypot(*value))
        / (values.len() as f64).sqrt()
}

fn maximum_optional(values: impl Iterator<Item = Option<f64>>) -> Option<f64> {
    let mut maximum = 0.0_f64;
    for value in values {
        match value {
            Some(value) => maximum = maximum.max(value),
            None => return None,
        }
    }
    Some(maximum)
}

/// Evaluate one full-revolution trace on its declared uniform angular grid.
///
/// No interpolation, offset fitting, probability estimate or sensor-ASIC model
/// is performed. The supplied phase-zero offset must come from independent
/// calibration, not from the trace under test.
pub fn evaluate_trace(
    trace: &FieldTrace,
    requirements: &EncoderRequirements,
) -> Result<TraceMetrics, EvaluationError> {
    validate_requirements(requirements)?;

    if trace.trace_id.trim().is_empty()
        || trace.frame_id.trim().is_empty()
        || trace.operating_condition_id.trim().is_empty()
        || trace.grid_id.trim().is_empty()
    {
        return Err(EvaluationError::InvalidMetadata);
    }
    if trace.samples.len() < 4 {
        return Err(EvaluationError::TooFewSamples);
    }

    let sample_count = trace.samples.len();
    let angle_step = TAU / sample_count as f64;
    let start_angle = trace.samples[0].mechanical_angle_rad;

    let mut magnitudes = Vec::with_capacity(sample_count);
    let mut phase_errors = Vec::with_capacity(sample_count);
    let mut absolute_phase_errors = Vec::with_capacity(sample_count);
    let mut bz_values = Vec::with_capacity(sample_count);
    let mut below_threshold_sample_count = 0_usize;

    for (index, sample) in trace.samples.iter().enumerate() {
        if !sample.mechanical_angle_rad.is_finite()
            || !sample.bx_t.is_finite()
            || !sample.by_t.is_finite()
            || !sample.bz_t.is_finite()
        {
            return Err(EvaluationError::NonFiniteSample { index });
        }

        if index > 0 && sample.mechanical_angle_rad <= trace.samples[index - 1].mechanical_angle_rad
        {
            return Err(EvaluationError::NonMonotonicAngleGrid { index });
        }

        let expected_angle = start_angle + index as f64 * angle_step;
        if (sample.mechanical_angle_rad - expected_angle).abs() > ANGLE_GRID_TOLERANCE_RAD {
            return Err(EvaluationError::NonUniformOrIncompleteRevolution { index });
        }

        let in_plane_magnitude = sample.bx_t.hypot(sample.by_t);
        if !in_plane_magnitude.is_finite() {
            return Err(EvaluationError::NumericOverflow);
        }
        if in_plane_magnitude < requirements.minimum_in_plane_field_t {
            below_threshold_sample_count += 1;
        }

        let expected_electrical_phase = requirements.phase_rotation_sense as f64
            * requirements.pole_pairs as f64
            * sample.mechanical_angle_rad
            + requirements.phase_zero_offset_rad;
        if !expected_electrical_phase.is_finite() {
            return Err(EvaluationError::NumericOverflow);
        }

        let measured_electrical_phase = sample.by_t.atan2(sample.bx_t);
        let phase_error = wrap_to_pi(measured_electrical_phase - expected_electrical_phase);

        magnitudes.push(in_plane_magnitude);
        phase_errors.push(phase_error);
        absolute_phase_errors.push(phase_error.abs());
        bz_values.push(sample.bz_t);
    }

    let minimum_in_plane_field_t = magnitudes.iter().copied().fold(f64::INFINITY, f64::min);
    let maximum_in_plane_field_t = magnitudes.iter().copied().fold(0.0_f64, f64::max);
    let mean_in_plane_field_t = magnitudes
        .iter()
        .map(|magnitude| *magnitude / sample_count as f64)
        .sum::<f64>();

    if !minimum_in_plane_field_t.is_finite()
        || !maximum_in_plane_field_t.is_finite()
        || !mean_in_plane_field_t.is_finite()
    {
        return Err(EvaluationError::NumericOverflow);
    }

    let amplitude_ripple_fraction = if mean_in_plane_field_t > 0.0 {
        Some((maximum_in_plane_field_t - minimum_in_plane_field_t) / mean_in_plane_field_t)
    } else {
        None
    };

    let rms_phase_error_rad = rms(&phase_errors);
    let rms_in_plane_field_t = rms(&magnitudes);
    let rms_out_of_plane_field_t = rms(&bz_values);
    let rms_cross_axis_ratio = if rms_in_plane_field_t > 0.0 {
        Some(rms_out_of_plane_field_t / rms_in_plane_field_t)
    } else {
        None
    };

    if !rms_phase_error_rad.is_finite()
        || !rms_in_plane_field_t.is_finite()
        || !rms_out_of_plane_field_t.is_finite()
        || amplitude_ripple_fraction.is_some_and(|value| !value.is_finite())
        || rms_cross_axis_ratio.is_some_and(|value| !value.is_finite())
    {
        return Err(EvaluationError::NumericOverflow);
    }

    absolute_phase_errors.sort_by(f64::total_cmp);
    let p95_index = (sample_count * 95).div_ceil(100).saturating_sub(1);
    let p95_absolute_phase_error_rad = absolute_phase_errors[p95_index];
    let maximum_absolute_phase_error_rad = absolute_phase_errors
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);

    let mut violations = Vec::new();
    if below_threshold_sample_count > 0 {
        violations.push(RequirementViolation::BelowMinimumInPlaneField);
    }
    if maximum_absolute_phase_error_rad > requirements.maximum_peak_phase_error_rad {
        violations.push(RequirementViolation::PeakPhaseErrorExceeded);
    }
    if rms_phase_error_rad > requirements.maximum_rms_phase_error_rad {
        violations.push(RequirementViolation::RmsPhaseErrorExceeded);
    }
    match amplitude_ripple_fraction {
        Some(value) if value <= requirements.maximum_amplitude_ripple_fraction => {}
        Some(_) => violations.push(RequirementViolation::AmplitudeRippleExceeded),
        None => violations.push(RequirementViolation::InPlaneSignalUnavailable),
    }
    match rms_cross_axis_ratio {
        Some(value) if value <= requirements.maximum_rms_cross_axis_ratio => {}
        Some(_) => violations.push(RequirementViolation::CrossAxisRatioExceeded),
        None if !violations.contains(&RequirementViolation::InPlaneSignalUnavailable) => {
            violations.push(RequirementViolation::InPlaneSignalUnavailable);
        }
        None => {}
    }

    Ok(TraceMetrics {
        trace_id: trace.trace_id.clone(),
        sample_count,
        minimum_in_plane_field_t,
        mean_in_plane_field_t,
        maximum_in_plane_field_t,
        amplitude_ripple_fraction,
        rms_phase_error_rad,
        p95_absolute_phase_error_rad,
        maximum_absolute_phase_error_rad,
        rms_cross_axis_ratio,
        below_threshold_sample_count,
        accepted: violations.is_empty(),
        violations,
    })
}

/// Evaluate a finite collection of traces without treating it as a probability
/// distribution or manufacturing-yield sample.
pub fn evaluate_trace_set(
    traces: &[FieldTrace],
    requirements: &EncoderRequirements,
) -> Result<TraceSetMetrics, EvaluationError> {
    let first = traces.first().ok_or(EvaluationError::EmptyTraceSet)?;
    let mut trace_ids = HashSet::with_capacity(traces.len());
    let mut metrics = Vec::with_capacity(traces.len());

    for trace in traces {
        if !trace_ids.insert(trace.trace_id.as_str()) {
            return Err(EvaluationError::DuplicateTraceId);
        }
        if trace.evidence_kind != first.evidence_kind {
            return Err(EvaluationError::MixedEvidenceKinds);
        }
        if trace.frame_id != first.frame_id {
            return Err(EvaluationError::FrameIdentityMismatch);
        }
        if trace.operating_condition_id != first.operating_condition_id {
            return Err(EvaluationError::OperatingConditionMismatch);
        }
        if trace.grid_id != first.grid_id {
            return Err(EvaluationError::GridIdentityMismatch);
        }
        if trace.samples.len() != first.samples.len()
            || trace
                .samples
                .iter()
                .zip(first.samples.iter())
                .any(|(left, right)| {
                    left.mechanical_angle_rad.to_bits() != right.mechanical_angle_rad.to_bits()
                })
        {
            return Err(EvaluationError::CoordinateGridMismatch);
        }
        metrics.push(evaluate_trace(trace, requirements)?);
    }

    let passing_trace_count = metrics.iter().filter(|metric| metric.accepted).count();
    let rejected_trace_ids = metrics
        .iter()
        .filter(|metric| !metric.accepted)
        .map(|metric| metric.trace_id.clone())
        .collect();

    Ok(TraceSetMetrics {
        evidence_kind: first.evidence_kind,
        trace_count: metrics.len(),
        passing_trace_count,
        all_traces_accepted: passing_trace_count == metrics.len(),
        minimum_in_plane_field_t: metrics
            .iter()
            .map(|metric| metric.minimum_in_plane_field_t)
            .fold(f64::INFINITY, f64::min),
        worst_peak_phase_error_rad: metrics
            .iter()
            .map(|metric| metric.maximum_absolute_phase_error_rad)
            .fold(0.0_f64, f64::max),
        worst_rms_phase_error_rad: metrics
            .iter()
            .map(|metric| metric.rms_phase_error_rad)
            .fold(0.0_f64, f64::max),
        worst_p95_absolute_phase_error_rad: metrics
            .iter()
            .map(|metric| metric.p95_absolute_phase_error_rad)
            .fold(0.0_f64, f64::max),
        worst_amplitude_ripple_fraction: maximum_optional(
            metrics
                .iter()
                .map(|metric| metric.amplitude_ripple_fraction),
        ),
        worst_rms_cross_axis_ratio: maximum_optional(
            metrics.iter().map(|metric| metric.rms_cross_axis_ratio),
        ),
        rejected_trace_ids,
    })
}



/// Evaluate multiple operating-condition traces under one fixed encoder
/// requirement set. Operating-condition IDs may differ, but evidence class,
/// coordinate frame, grid identity, sample count and bit-identical angle
/// coordinates must match. This finite scenario set is not an IID build sample
/// and does not imply probability, manufacturing yield or CVaR.
pub fn evaluate_operating_condition_scenarios(
    set_id: &str,
    traces: &[FieldTrace],
    requirements: &EncoderRequirements,
) -> Result<OperatingConditionScenarioSetMetrics, EvaluationError> {
    if set_id.trim().is_empty() {
        return Err(EvaluationError::InvalidScenarioSetId);
    }
    let first = traces.first().ok_or(EvaluationError::EmptyTraceSet)?;
    let mut trace_ids = HashSet::with_capacity(traces.len());
    let mut metrics = Vec::with_capacity(traces.len());

    for trace in traces {
        if !trace_ids.insert(trace.trace_id.as_str()) {
            return Err(EvaluationError::DuplicateTraceId);
        }
        if trace.evidence_kind != first.evidence_kind {
            return Err(EvaluationError::MixedEvidenceKinds);
        }
        if trace.frame_id != first.frame_id {
            return Err(EvaluationError::ScenarioFrameMismatch);
        }
        if trace.grid_id != first.grid_id {
            return Err(EvaluationError::ScenarioGridMismatch);
        }
        if trace.samples.len() != first.samples.len()
            || trace
                .samples
                .iter()
                .zip(first.samples.iter())
                .any(|(left, right)| {
                    left.mechanical_angle_rad.to_bits()
                        != right.mechanical_angle_rad.to_bits()
                })
        {
            return Err(EvaluationError::ScenarioGridCoordinatesMismatch);
        }
        metrics.push(evaluate_trace(trace, requirements)?);
    }

    let accepted_scenario_count = metrics.iter().filter(|metric| metric.accepted).count();
    let rejected_scenario_count = metrics.len() - accepted_scenario_count;

    Ok(OperatingConditionScenarioSetMetrics {
        set_id: set_id.to_owned(),
        evidence_kind: first.evidence_kind,
        frame_id: first.frame_id.clone(),
        grid_id: first.grid_id.clone(),
        scenario_count: metrics.len(),
        accepted_scenario_count,
        rejected_scenario_count,
        all_scenarios_accepted: rejected_scenario_count == 0,
        worst_peak_phase_error_rad: metrics
            .iter()
            .map(|metric| metric.maximum_absolute_phase_error_rad)
            .fold(0.0_f64, f64::max),
        worst_rms_phase_error_rad: metrics
            .iter()
            .map(|metric| metric.rms_phase_error_rad)
            .fold(0.0_f64, f64::max),
        worst_p95_phase_error_rad: metrics
            .iter()
            .map(|metric| metric.p95_absolute_phase_error_rad)
            .fold(0.0_f64, f64::max),
        minimum_in_plane_field_t: metrics
            .iter()
            .map(|metric| metric.minimum_in_plane_field_t)
            .fold(f64::INFINITY, f64::min),
        worst_amplitude_ripple_fraction: maximum_optional(
            metrics.iter().map(|metric| metric.amplitude_ripple_fraction),
        ),
        worst_rms_cross_axis_ratio: maximum_optional(
            metrics.iter().map(|metric| metric.rms_cross_axis_ratio),
        ),
        worst_below_threshold_sample_count: metrics
            .iter()
            .map(|metric| metric.below_threshold_sample_count)
            .max()
            .unwrap_or(0),
        scenarios: metrics,
    })
}

fn validate_field_covariance(
    covariance: FieldCovariance2d,
    index: usize,
) -> Result<(), EvaluationError> {
    if !covariance.xx_t2.is_finite()
        || !covariance.xy_t2.is_finite()
        || !covariance.yy_t2.is_finite()
    {
        return Err(EvaluationError::NonFiniteFieldCovariance { index });
    }

    let scale = covariance
        .xx_t2
        .abs()
        .max(covariance.xy_t2.abs())
        .max(covariance.yy_t2.abs());
    if scale == 0.0 {
        return Ok(());
    }

    let xx = covariance.xx_t2 / scale;
    let xy = covariance.xy_t2 / scale;
    let yy = covariance.yy_t2 / scale;
    let determinant = xx * yy - xy * xy;
    const PSD_TOLERANCE: f64 = 1.0e-12;
    if xx < -PSD_TOLERANCE || yy < -PSD_TOLERANCE || determinant < -PSD_TOLERANCE {
        return Err(EvaluationError::NonPositiveSemidefiniteFieldCovariance { index });
    }
    Ok(())
}

fn nearest_rank_p95(values: &[f64]) -> f64 {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let index = (sorted.len() * 95).div_ceil(100).saturating_sub(1);
    sorted[index]
}

/// Propagate per-sample in-plane field covariance through atan2(By, Bx).
///
/// The covariance trace must have exact matching IDs and bit-identical sample
/// angles. The mechanical-reference contribution is scaled by pole_pairs and
/// added separately from the phase-zero calibration contribution. Returned
/// residual/standard-uncertainty ratios are descriptive, not coverage tests.
pub fn propagate_phase_uncertainty(
    trace: &FieldTrace,
    covariance_trace: &FieldCovarianceTrace,
    requirements: &EncoderRequirements,
    reference: &AngularReferenceUncertainty,
) -> Result<PhaseUncertaintyMetrics, EvaluationError> {
    validate_requirements(requirements)?;
    if reference.uncertainty_record_id.trim().is_empty()
        || reference.phase_zero_calibration_id.trim().is_empty()
        || reference.phase_zero_calibration_id != requirements.phase_zero_calibration_id
        || !reference.phase_zero_std_rad.is_finite()
        || reference.phase_zero_std_rad < 0.0
        || !reference.mechanical_angle_std_rad.is_finite()
        || reference.mechanical_angle_std_rad < 0.0
    {
        return Err(EvaluationError::InvalidReferenceUncertainty);
    }

    // Reuse the deterministic evaluator for trace metadata, grid and field validation.
    let _deterministic_metrics = evaluate_trace(trace, requirements)?;

    if covariance_trace.trace_id != trace.trace_id {
        return Err(EvaluationError::CovarianceTraceIdentityMismatch);
    }
    if covariance_trace.frame_id != trace.frame_id {
        return Err(EvaluationError::CovarianceTraceFrameMismatch);
    }
    if covariance_trace.operating_condition_id != trace.operating_condition_id {
        return Err(EvaluationError::CovarianceTraceConditionMismatch);
    }
    if covariance_trace.grid_id != trace.grid_id {
        return Err(EvaluationError::CovarianceTraceGridMismatch);
    }
    if covariance_trace.samples.len() != trace.samples.len() {
        return Err(EvaluationError::CoordinateGridMismatch);
    }

    let reference_phase_zero_std_rad = reference.phase_zero_std_rad;
    let reference_mechanical_phase_std_rad =
        f64::from(requirements.pole_pairs) * reference.mechanical_angle_std_rad;
    if !reference_mechanical_phase_std_rad.is_finite() {
        return Err(EvaluationError::InvalidReferenceUncertainty);
    }

    let reference_variance = reference_phase_zero_std_rad * reference_phase_zero_std_rad
        + reference_mechanical_phase_std_rad * reference_mechanical_phase_std_rad;
    if !reference_variance.is_finite() {
        return Err(EvaluationError::InvalidReferenceUncertainty);
    }

    let mut measurement_stds = Vec::with_capacity(trace.samples.len());
    let mut total_stds = Vec::with_capacity(trace.samples.len());
    let mut normalized_residuals = Vec::with_capacity(trace.samples.len());
    let mut zero_uncertainty_nonzero_residual_sample_count = 0_usize;

    for (index, (sample, covariance_sample)) in trace
        .samples
        .iter()
        .zip(covariance_trace.samples.iter())
        .enumerate()
    {
        if sample.mechanical_angle_rad.to_bits()
            != covariance_sample.mechanical_angle_rad.to_bits()
        {
            return Err(EvaluationError::CoordinateGridMismatch);
        }
        validate_field_covariance(covariance_sample.covariance, index)?;

        let magnitude = sample.bx_t.hypot(sample.by_t);
        if !magnitude.is_finite() {
            return Err(EvaluationError::PhaseUncertaintyNumericOverflow { index });
        }
        if magnitude == 0.0 {
            return Err(EvaluationError::ZeroInPlaneFieldForPhase { index });
        }
        if magnitude < requirements.minimum_in_plane_field_t {
            return Err(EvaluationError::FieldBelowMinimumForPhase { index });
        }

        // Stable form of (-By / |B|^2, Bx / |B|^2).
        let gradient_x = -(sample.by_t / magnitude) / magnitude;
        let gradient_y = (sample.bx_t / magnitude) / magnitude;
        let covariance = covariance_sample.covariance;
        let term_x = gradient_x * gradient_x * covariance.xx_t2;
        let term_xy = 2.0 * gradient_x * gradient_y * covariance.xy_t2;
        let term_y = gradient_y * gradient_y * covariance.yy_t2;
        let measurement_variance = term_x + term_xy + term_y;
        let variance_scale = term_x.abs() + term_xy.abs() + term_y.abs();

        if !measurement_variance.is_finite() || !variance_scale.is_finite() {
            return Err(EvaluationError::PhaseUncertaintyNumericOverflow { index });
        }
        if measurement_variance < -1.0e-12 * variance_scale.max(f64::MIN_POSITIVE) {
            return Err(EvaluationError::NonPositiveSemidefiniteFieldCovariance { index });
        }

        let measurement_std = measurement_variance.max(0.0).sqrt();
        let total_variance = measurement_variance.max(0.0) + reference_variance;
        let total_std = total_variance.sqrt();
        if !measurement_std.is_finite() || !total_std.is_finite() {
            return Err(EvaluationError::PhaseUncertaintyNumericOverflow { index });
        }

        let expected_phase = f64::from(requirements.pole_pairs)
            * f64::from(requirements.phase_rotation_sense)
            * sample.mechanical_angle_rad
            + requirements.phase_zero_offset_rad;
        let measured_phase = sample.by_t.atan2(sample.bx_t);
        let absolute_phase_error = wrap_to_pi(measured_phase - expected_phase).abs();

        if total_std > 0.0 {
            normalized_residuals.push(absolute_phase_error / total_std);
        } else if absolute_phase_error > 0.0 {
            zero_uncertainty_nonzero_residual_sample_count += 1;
        }

        measurement_stds.push(measurement_std);
        total_stds.push(total_std);
    }

    let rms_measurement_std = rms(&measurement_stds);
    let rms_total_std = rms(&total_stds);
    let rms_normalized_residual = (!normalized_residuals.is_empty())
        .then(|| rms(&normalized_residuals));
    let maximum_normalized_residual = normalized_residuals
        .iter()
        .copied()
        .reduce(f64::max);

    Ok(PhaseUncertaintyMetrics {
        trace_id: trace.trace_id.clone(),
        sample_count: trace.samples.len(),
        uncertainty_record_id: reference.uncertainty_record_id.clone(),
        phase_zero_calibration_id: reference.phase_zero_calibration_id.clone(),
        measurement_phase_std_mean_rad: measurement_stds
            .iter()
            .map(|value| *value / measurement_stds.len() as f64)
            .sum(),
        measurement_phase_std_rms_rad: rms_measurement_std,
        measurement_phase_std_p95_rad: nearest_rank_p95(&measurement_stds),
        measurement_phase_std_max_rad: measurement_stds.iter().copied().fold(0.0, f64::max),
        reference_phase_zero_std_rad,
        reference_mechanical_phase_std_rad,
        total_phase_std_mean_rad: total_stds
            .iter()
            .map(|value| *value / total_stds.len() as f64)
            .sum(),
        total_phase_std_rms_rad: rms_total_std,
        total_phase_std_p95_rad: nearest_rank_p95(&total_stds),
        total_phase_std_max_rad: total_stds.iter().copied().fold(0.0, f64::max),
        rms_phase_error_over_total_std: rms_normalized_residual,
        maximum_phase_error_over_total_std: maximum_normalized_residual,
        zero_uncertainty_nonzero_residual_sample_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirements(pole_pairs: u32, phase_zero_offset_rad: f64) -> EncoderRequirements {
        EncoderRequirements {
            pole_pairs,
            phase_rotation_sense: 1,
            phase_zero_calibration_id: "calibration-record-v0".to_owned(),
            phase_zero_offset_rad,
            minimum_in_plane_field_t: 0.020,
            maximum_peak_phase_error_rad: 0.010,
            maximum_rms_phase_error_rad: 0.005,
            maximum_amplitude_ripple_fraction: 0.10,
            maximum_rms_cross_axis_ratio: 0.05,
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn make_trace<F>(
        trace_id: &str,
        evidence_kind: EvidenceKind,
        sample_count: usize,
        pole_pairs: u32,
        phase_zero_offset_rad: f64,
        phase_error_rad: f64,
        amplitude_scale: F,
        cross_axis_ratio: f64,
    ) -> FieldTrace
    where
        F: Fn(f64) -> f64,
    {
        let samples = (0..sample_count)
            .map(|index| {
                let angle = TAU * index as f64 / sample_count as f64;
                let amplitude = 0.040 * amplitude_scale(angle);
                let phase = pole_pairs as f64 * angle + phase_zero_offset_rad + phase_error_rad;
                FieldSample {
                    mechanical_angle_rad: angle,
                    bx_t: amplitude * phase.cos(),
                    by_t: amplitude * phase.sin(),
                    bz_t: amplitude * cross_axis_ratio,
                }
            })
            .collect();

        FieldTrace {
            trace_id: trace_id.to_owned(),
            frame_id: "sensor-plane-v0".to_owned(),
            operating_condition_id: "gap-3mm-temp-25C-v0".to_owned(),
            grid_id: format!("full-turn-{sample_count}-uniform-v0"),
            evidence_kind,
            samples,
        }
    }

    fn constant_amplitude(_: f64) -> f64 {
        1.0
    }


    fn covariance_trace(
        trace: &FieldTrace,
        covariance: FieldCovariance2d,
    ) -> FieldCovarianceTrace {
        FieldCovarianceTrace {
            trace_id: trace.trace_id.clone(),
            frame_id: trace.frame_id.clone(),
            operating_condition_id: trace.operating_condition_id.clone(),
            grid_id: trace.grid_id.clone(),
            samples: trace
                .samples
                .iter()
                .map(|sample| FieldCovarianceSample {
                    mechanical_angle_rad: sample.mechanical_angle_rad,
                    covariance,
                })
                .collect(),
        }
    }

    fn reference_uncertainty(
        phase_zero_std_rad: f64,
        mechanical_angle_std_rad: f64,
    ) -> AngularReferenceUncertainty {
        AngularReferenceUncertainty {
            uncertainty_record_id: "angle-reference-uncertainty-v0".to_owned(),
            phase_zero_calibration_id: "calibration-record-v0".to_owned(),
            phase_zero_std_rad,
            mechanical_angle_std_rad,
        }
    }


    #[test]
    fn propagates_diagonal_field_covariance_and_reference_uncertainty() {
        let trace = make_trace(
            "uncertainty-known-answer",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.2,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: 0.0,
            yy_t2: 1.0e-8,
        };
        let covariance_trace = covariance_trace(&trace, covariance);
        let metrics = propagate_phase_uncertainty(
            &trace,
            &covariance_trace,
            &requirements(1, 0.2),
            &reference_uncertainty(0.001, 0.0002),
        )
        .unwrap();

        assert!((metrics.measurement_phase_std_mean_rad - 0.0025).abs() < 1.0e-12);
        assert!((metrics.total_phase_std_mean_rad - 0.0027).abs() < 1.0e-12);
        assert!((metrics.reference_mechanical_phase_std_rad - 0.0002).abs() < 1.0e-15);
        assert_eq!(metrics.zero_uncertainty_nonzero_residual_sample_count, 0);
    }

    #[test]
    fn phase_uncertainty_scales_inversely_with_field_amplitude() {
        let lower = make_trace(
            "lower-field",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let higher = make_trace(
            "higher-field",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            |angle| 2.0,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: 0.0,
            yy_t2: 1.0e-8,
        };
        let lower_metrics = propagate_phase_uncertainty(
            &lower,
            &covariance_trace(&lower, covariance),
            &requirements(1, 0.0),
            &reference_uncertainty(0.0, 0.0),
        )
        .unwrap();
        let higher_metrics = propagate_phase_uncertainty(
            &higher,
            &covariance_trace(&higher, covariance),
            &requirements(1, 0.0),
            &reference_uncertainty(0.0, 0.0),
        )
        .unwrap();

        assert!(
            (lower_metrics.measurement_phase_std_mean_rad
                / higher_metrics.measurement_phase_std_mean_rad
                - 2.0)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn rotating_field_and_covariance_together_preserves_phase_uncertainty() {
        let trace = make_trace(
            "rotation-covariance-base",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let alpha = 0.37_f64;
        let cosine = alpha.cos();
        let sine = alpha.sin();
        let mut rotated = trace.clone();
        rotated.trace_id = "rotation-covariance-rotated".to_owned();
        for sample in &mut rotated.samples {
            let bx = sample.bx_t;
            let by = sample.by_t;
            sample.bx_t = cosine * bx - sine * by;
            sample.by_t = sine * bx + cosine * by;
        }

        let base_covariance = FieldCovariance2d {
            xx_t2: 4.0e-8,
            xy_t2: 1.0e-8,
            yy_t2: 1.0e-8,
        };
        let rotated_covariance = FieldCovariance2d {
            xx_t2: cosine * cosine * base_covariance.xx_t2
                - 2.0 * cosine * sine * base_covariance.xy_t2
                + sine * sine * base_covariance.yy_t2,
            xy_t2: cosine * sine * base_covariance.xx_t2
                + (cosine * cosine - sine * sine) * base_covariance.xy_t2
                - sine * cosine * base_covariance.yy_t2,
            yy_t2: sine * sine * base_covariance.xx_t2
                + 2.0 * sine * cosine * base_covariance.xy_t2
                + cosine * cosine * base_covariance.yy_t2,
        };

        let base_metrics = propagate_phase_uncertainty(
            &trace,
            &covariance_trace(&trace, base_covariance),
            &requirements(1, 0.0),
            &reference_uncertainty(0.0, 0.0),
        )
        .unwrap();
        let rotated_metrics = propagate_phase_uncertainty(
            &rotated,
            &covariance_trace(&rotated, rotated_covariance),
            &requirements(1, alpha),
            &reference_uncertainty(0.0, 0.0),
        )
        .unwrap();

        assert!(
            (base_metrics.measurement_phase_std_rms_rad
                - rotated_metrics.measurement_phase_std_rms_rad)
                .abs()
                < 1.0e-12
        );
    }

    #[test]
    fn rejects_indefinite_field_covariance() {
        let trace = make_trace(
            "indefinite-covariance",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: 2.0e-8,
            yy_t2: 1.0e-8,
        };
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace(&trace, covariance),
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::NonPositiveSemidefiniteFieldCovariance { index: 0 })
        );
    }

    #[test]
    fn rejects_nonfinite_field_covariance() {
        let trace = make_trace(
            "nan-covariance",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: f64::NAN,
            yy_t2: 1.0e-8,
        };
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace(&trace, covariance),
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::NonFiniteFieldCovariance { index: 0 })
        );
    }

    #[test]
    fn requires_field_covariance_identity_and_grid_to_match() {
        let trace = make_trace(
            "covariance-identity",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: 0.0,
            yy_t2: 1.0e-8,
        };
        let mut mismatched = covariance_trace(&trace, covariance);
        mismatched.frame_id = "other-frame".to_owned();
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &mismatched,
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::CovarianceTraceFrameMismatch)
        );

        let mut mismatched_grid = covariance_trace(&trace, covariance);
        mismatched_grid.samples[10].mechanical_angle_rad += 1.0e-12;
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &mismatched_grid,
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::CoordinateGridMismatch)
        );
    }

    #[test]
    fn rejects_reference_uncertainty_bound_to_another_phase_calibration() {
        let trace = make_trace(
            "wrong-calibration-id",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: 0.0,
            yy_t2: 1.0e-8,
        };
        let mut reference = reference_uncertainty(0.0, 0.0);
        reference.phase_zero_calibration_id = "unrelated-calibration".to_owned();
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace(&trace, covariance),
                &requirements(1, 0.0),
                &reference,
            ),
            Err(EvaluationError::InvalidReferenceUncertainty)
        );
    }


    #[test]
    fn refuses_zero_field_as_undefined_phase() {
        let trace = make_trace(
            "zero-field",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            |_| 0.0,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 0.0,
            xy_t2: 0.0,
            yy_t2: 0.0,
        };
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace(&trace, covariance),
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::ZeroInPlaneFieldForPhase { index: 0 })
        );
    }

    #[test]
    fn refuses_phase_uncertainty_below_minimum_field() {
        let trace = make_trace(
            "weak-field",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            |angle| 0.25,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: 0.0,
            yy_t2: 1.0e-8,
        };
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace(&trace, covariance),
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::FieldBelowMinimumForPhase { index: 0 })
        );
    }

    #[test]
    fn counts_nonzero_residual_when_declared_uncertainty_is_zero() {
        let trace = make_trace(
            "residual-zero-uncertainty",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.001,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 0.0,
            xy_t2: 0.0,
            yy_t2: 0.0,
        };
        let metrics = propagate_phase_uncertainty(
            &trace,
            &covariance_trace(&trace, covariance),
            &requirements(1, 0.0),
            &reference_uncertainty(0.0, 0.0),
        )
        .unwrap();

        assert_eq!(
            metrics.zero_uncertainty_nonzero_residual_sample_count,
            trace.samples.len()
        );
        assert_eq!(metrics.rms_phase_error_over_total_std, None);
    }


    #[test]
    fn mechanical_reference_uncertainty_scales_with_pole_pairs() {
        let trace = make_trace(
            "ten-pole-pair-reference-uncertainty",
            EvidenceKind::SyntheticScenario,
            720,
            10,
            -0.23,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 0.0,
            xy_t2: 0.0,
            yy_t2: 0.0,
        };
        let metrics = propagate_phase_uncertainty(
            &trace,
            &covariance_trace(&trace, covariance),
            &requirements(10, -0.23),
            &reference_uncertainty(0.0, 0.0002),
        )
        .unwrap();

        assert!((metrics.reference_mechanical_phase_std_rad - 0.002).abs() < 1.0e-15);
        assert!((metrics.total_phase_std_mean_rad - 0.002).abs() < 1.0e-15);
    }

    #[test]
    fn rejects_negative_reference_uncertainty() {
        let trace = make_trace(
            "negative-reference-uncertainty",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 0.0,
            xy_t2: 0.0,
            yy_t2: 0.0,
        };
        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace(&trace, covariance),
                &requirements(1, 0.0),
                &reference_uncertainty(-1.0e-3, 0.0),
            ),
            Err(EvaluationError::InvalidReferenceUncertainty)
        );
    }

    #[test]
    fn rejects_mismatched_covariance_trace_identity() {
        let trace = make_trace(
            "field-trace-id",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 0.0,
            xy_t2: 0.0,
            yy_t2: 0.0,
        };
        let mut covariance_trace = covariance_trace(&trace, covariance);
        covariance_trace.trace_id = "different-trace".to_owned();

        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace,
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::CovarianceTraceIdentityMismatch)
        );
    }

    #[test]
    fn rejects_mismatched_operating_condition() {
        let trace = make_trace(
            "condition-mismatch",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 0.0,
            xy_t2: 0.0,
            yy_t2: 0.0,
        };
        let mut covariance_trace = covariance_trace(&trace, covariance);
        covariance_trace.operating_condition_id = "other-gap".to_owned();

        assert_eq!(
            propagate_phase_uncertainty(
                &trace,
                &covariance_trace,
                &requirements(1, 0.0),
                &reference_uncertainty(0.0, 0.0),
            ),
            Err(EvaluationError::CovarianceTraceConditionMismatch)
        );
    }

    #[test]
    fn accepts_rank_one_positive_semidefinite_field_covariance() {
        let trace = make_trace(
            "rank-one-covariance",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let covariance = FieldCovariance2d {
            xx_t2: 1.0e-8,
            xy_t2: 1.0e-8,
            yy_t2: 1.0e-8,
        };
        let metrics = propagate_phase_uncertainty(
            &trace,
            &covariance_trace(&trace, covariance),
            &requirements(1, 0.0),
            &reference_uncertainty(0.0, 0.0),
        )
        .unwrap();

        assert!(metrics.measurement_phase_std_max_rad.is_finite());
    }


    #[test]
    fn vector_phase_tournament_allows_distinct_operating_conditions() {
        let nominal = make_trace(
            "nominal-gap",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let mut wider_gap = nominal.clone();
        wider_gap.trace_id = "wider-gap".to_owned();
        wider_gap.operating_condition_id = "gap-4mm-temp-25C".to_owned();
        for sample in &mut wider_gap.samples {
            sample.bx_t *= 0.75;
            sample.by_t *= 0.75;
        }

        let result = evaluate_operating_condition_scenarios(
            "encoder-air-gap-sweep-v0",
            &[nominal, wider_gap],
            &requirements(1, 0.0),
        )
        .unwrap();

        assert_eq!(result.scenario_count, 2);
        assert_eq!(result.accepted_scenario_count, 2);
        assert!(result.all_scenarios_accepted);
    }

    #[test]
    fn vector_phase_tournament_preserves_a_worst_case_failure() {
        let nominal = make_trace(
            "nominal",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let weak = make_trace(
            "weak-field",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            |angle| 0.25,
            0.0,
        );
        let result = evaluate_operating_condition_scenarios(
            "encoder-field-margin-sweep-v0",
            &[nominal, weak],
            &requirements(1, 0.0),
        )
        .unwrap();

        assert_eq!(result.accepted_scenario_count, 1);
        assert_eq!(result.rejected_scenario_count, 1);
        assert!(!result.all_scenarios_accepted);
        assert_eq!(result.scenarios[1].trace_id, "weak-field");
    }

    #[test]
    fn vector_phase_tournament_rejects_mixed_evidence() {
        let first = make_trace(
            "synthetic",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let mut second = first.clone();
        second.trace_id = "physical".to_owned();
        second.evidence_kind = EvidenceKind::PhysicalBuild;

        assert_eq!(
            evaluate_operating_condition_scenarios(
                "mixed-evidence",
                &[first, second],
                &requirements(1, 0.0),
            ),
            Err(EvaluationError::MixedEvidenceKinds)
        );
    }

    #[test]
    fn vector_phase_tournament_rejects_frame_and_grid_mismatch() {
        let first = make_trace(
            "first",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let mut frame_mismatch = first.clone();
        frame_mismatch.trace_id = "frame-mismatch".to_owned();
        frame_mismatch.frame_id = "other-frame".to_owned();
        assert_eq!(
            evaluate_operating_condition_scenarios(
                "frame-mismatch-set",
                &[first.clone(), frame_mismatch],
                &requirements(1, 0.0),
            ),
            Err(EvaluationError::ScenarioFrameMismatch)
        );

        let mut grid_mismatch = first.clone();
        grid_mismatch.trace_id = "grid-mismatch".to_owned();
        grid_mismatch.grid_id = "other-grid".to_owned();
        assert_eq!(
            evaluate_operating_condition_scenarios(
                "grid-mismatch-set",
                &[first.clone(), grid_mismatch],
                &requirements(1, 0.0),
            ),
            Err(EvaluationError::ScenarioGridMismatch)
        );

        let mut coordinate_mismatch = first.clone();
        coordinate_mismatch.trace_id = "coordinate-mismatch".to_owned();
        coordinate_mismatch.samples[4].mechanical_angle_rad += 1.0e-12;
        assert_eq!(
            evaluate_operating_condition_scenarios(
                "coordinate-mismatch-set",
                &[first, coordinate_mismatch],
                &requirements(1, 0.0),
            ),
            Err(EvaluationError::ScenarioGridCoordinatesMismatch)
        );
    }

    #[test]
    fn ideal_single_pole_pair_trace_passes() {
        let trace = make_trace(
            "build-001",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.37,
            0.0,
            constant_amplitude,
            0.0,
        );
        let metrics = evaluate_trace(&trace, &requirements(1, 0.37)).unwrap();

        assert!(metrics.accepted);
        assert_eq!(metrics.below_threshold_sample_count, 0);
        assert!(metrics.maximum_absolute_phase_error_rad < 1.0e-12);
        assert!(metrics.rms_phase_error_rad < 1.0e-12);
        assert!(metrics.amplitude_ripple_fraction.unwrap() < 1.0e-12);
    }

    #[test]
    fn multipole_trace_uses_electrical_not_mechanical_phase() {
        let trace = make_trace(
            "build-002",
            EvidenceKind::SyntheticScenario,
            720,
            10,
            -0.23,
            0.0,
            constant_amplitude,
            0.0,
        );
        let metrics = evaluate_trace(&trace, &requirements(10, -0.23)).unwrap();

        assert!(metrics.accepted);
        assert!(metrics.maximum_absolute_phase_error_rad < 1.0e-12);
    }

    #[test]
    fn circular_phase_error_wraps_across_pi() {
        let offset = PI - 0.01;
        let trace = make_trace(
            "wrap-control",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            offset,
            0.0,
            constant_amplitude,
            0.0,
        );
        let metrics = evaluate_trace(&trace, &requirements(1, offset)).unwrap();

        assert!(metrics.accepted);
        assert!(metrics.maximum_absolute_phase_error_rad < 1.0e-12);
    }

    #[test]
    fn excessive_phase_error_fails_hard_gates() {
        let trace = make_trace(
            "phase-error",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.020,
            constant_amplitude,
            0.0,
        );
        let metrics = evaluate_trace(&trace, &requirements(1, 0.0)).unwrap();

        assert!(!metrics.accepted);
        assert!(metrics.maximum_absolute_phase_error_rad > 0.019);
        assert!(
            metrics
                .violations
                .contains(&RequirementViolation::PeakPhaseErrorExceeded)
        );
        assert!(
            metrics
                .violations
                .contains(&RequirementViolation::RmsPhaseErrorExceeded)
        );
    }

    #[test]
    fn one_field_dropout_cannot_be_hidden_by_the_mean() {
        let trace = make_trace(
            "dropout",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            |angle| if angle > 1.0 && angle < 1.1 { 0.1 } else { 1.0 },
            0.0,
        );
        let metrics = evaluate_trace(&trace, &requirements(1, 0.0)).unwrap();

        assert!(!metrics.accepted);
        assert!(metrics.below_threshold_sample_count > 0);
        assert!(
            metrics
                .violations
                .contains(&RequirementViolation::BelowMinimumInPlaneField)
        );
    }

    #[test]
    fn amplitude_ripple_is_reported_separately() {
        let trace = make_trace(
            "ripple",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            |angle| 1.0 + 0.15 * angle.cos(),
            0.0,
        );
        let metrics = evaluate_trace(&trace, &requirements(1, 0.0)).unwrap();

        assert!(!metrics.accepted);
        assert!(metrics.amplitude_ripple_fraction.unwrap() > 0.10);
        assert!(
            metrics
                .violations
                .contains(&RequirementViolation::AmplitudeRippleExceeded)
        );
    }

    #[test]
    fn out_of_plane_leakage_is_reported_separately() {
        let trace = make_trace(
            "cross-axis",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.10,
        );
        let metrics = evaluate_trace(&trace, &requirements(1, 0.0)).unwrap();

        assert!(!metrics.accepted);
        assert!(metrics.rms_cross_axis_ratio.unwrap() > 0.05);
        assert!(
            metrics
                .violations
                .contains(&RequirementViolation::CrossAxisRatioExceeded)
        );
    }

    #[test]
    fn trace_set_requires_every_trace_to_pass() {
        let good = make_trace(
            "scenario-nominal",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let bad = make_trace(
            "scenario-eccentric",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.020,
            constant_amplitude,
            0.0,
        );
        let summary = evaluate_trace_set(&[good, bad], &requirements(1, 0.0)).unwrap();

        assert!(!summary.all_traces_accepted);
        assert_eq!(summary.trace_count, 2);
        assert_eq!(summary.passing_trace_count, 1);
        assert_eq!(summary.rejected_trace_ids, vec!["scenario-eccentric"]);
    }

    #[test]
    fn trace_set_rejects_mismatched_frame_or_grid_identity() {
        let first = make_trace(
            "one",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let mut second = first.clone();
        second.trace_id = "two".to_owned();
        second.grid_id = "different-grid".to_owned();

        assert_eq!(
            evaluate_trace_set(&[first.clone(), second], &requirements(1, 0.0)),
            Err(EvaluationError::GridIdentityMismatch)
        );

        let mut frame_mismatch = first.clone();
        frame_mismatch.trace_id = "three".to_owned();
        frame_mismatch.frame_id = "other-frame".to_owned();
        assert_eq!(
            evaluate_trace_set(&[first, frame_mismatch], &requirements(1, 0.0)),
            Err(EvaluationError::FrameIdentityMismatch)
        );
    }

    #[test]
    fn trace_rejects_nonuniform_and_nonfinite_samples() {
        let mut nonuniform = make_trace(
            "nonuniform",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        nonuniform.samples[20].mechanical_angle_rad += 0.001;
        assert_eq!(
            evaluate_trace(&nonuniform, &requirements(1, 0.0)),
            Err(EvaluationError::NonUniformOrIncompleteRevolution { index: 20 })
        );

        let mut nonfinite = make_trace(
            "nonfinite",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        nonfinite.samples[2].bz_t = f64::NAN;
        assert_eq!(
            evaluate_trace(&nonfinite, &requirements(1, 0.0)),
            Err(EvaluationError::NonFiniteSample { index: 2 })
        );
    }

    #[test]
    fn requirements_must_name_an_independent_phase_calibration() {
        let trace = make_trace(
            "missing-calibration-id",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let mut requirements = requirements(1, 0.0);
        requirements.phase_zero_calibration_id.clear();

        assert_eq!(
            evaluate_trace(&trace, &requirements),
            Err(EvaluationError::InvalidRequirements)
        );
    }

    #[test]
    fn reversed_electrical_phase_direction_is_supported() {
        let mut trace = make_trace(
            "reverse-direction",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.19,
            0.0,
            constant_amplitude,
            0.0,
        );
        for sample in &mut trace.samples {
            let phase = -sample.mechanical_angle_rad + 0.19;
            let magnitude = sample.bx_t.hypot(sample.by_t);
            sample.bx_t = magnitude * phase.cos();
            sample.by_t = magnitude * phase.sin();
        }
        let mut requirements = requirements(1, 0.19);
        requirements.phase_rotation_sense = -1;
        let metrics = evaluate_trace(&trace, &requirements).unwrap();

        assert!(metrics.accepted);
        assert!(metrics.maximum_absolute_phase_error_rad < 1.0e-12);
    }

    #[test]
    fn trace_set_rejects_different_operating_conditions() {
        let first = make_trace(
            "gap-3mm",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let mut second = first.clone();
        second.trace_id = "gap-4mm".to_owned();
        second.operating_condition_id = "gap-4mm-temp-25C-v0".to_owned();

        assert_eq!(
            evaluate_trace_set(&[first, second], &requirements(1, 0.0)),
            Err(EvaluationError::OperatingConditionMismatch)
        );
    }

    #[test]
    fn trace_set_rejects_mixed_evidence_and_duplicate_ids() {
        let first = make_trace(
            "same-id",
            EvidenceKind::SyntheticScenario,
            360,
            1,
            0.0,
            0.0,
            constant_amplitude,
            0.0,
        );
        let mut second = first.clone();
        second.evidence_kind = EvidenceKind::PhysicalBuild;
        assert_eq!(
            evaluate_trace_set(&[first.clone(), second], &requirements(1, 0.0)),
            Err(EvaluationError::MixedEvidenceKinds)
        );

        let duplicate = first.clone();
        assert_eq!(
            evaluate_trace_set(&[first, duplicate], &requirements(1, 0.0)),
            Err(EvaluationError::DuplicateTraceId)
        );
    }
}
