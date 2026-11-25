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
    /// Opaque caller-supplied full-revolution angular-grid identity.
    pub grid_id: String,
    pub evidence_kind: EvidenceKind,
    pub samples: Vec<FieldSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EncoderRequirements {
    pub pole_pairs: u32,
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

#[derive(Clone, Debug, PartialEq)]
pub enum EvaluationError {
    InvalidMetadata,
    InvalidRequirements,
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

        if index > 0
            && sample.mechanical_angle_rad <= trace.samples[index - 1].mechanical_angle_rad
        {
            return Err(EvaluationError::NonMonotonicAngleGrid { index });
        }

        let expected_angle = start_angle + index as f64 * angle_step;
        if (sample.mechanical_angle_rad - expected_angle).abs()
            > ANGLE_GRID_TOLERANCE_RAD
        {
            return Err(EvaluationError::NonUniformOrIncompleteRevolution { index });
        }

        let in_plane_magnitude = sample.bx_t.hypot(sample.by_t);
        if !in_plane_magnitude.is_finite() {
            return Err(EvaluationError::NumericOverflow);
        }
        if in_plane_magnitude < requirements.minimum_in_plane_field_t {
            below_threshold_sample_count += 1;
        }

        let expected_electrical_phase = requirements.pole_pairs as f64
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
    let maximum_absolute_phase_error_rad =
        absolute_phase_errors.iter().copied().fold(0.0_f64, f64::max);

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
            metrics.iter().map(|metric| metric.amplitude_ripple_fraction),
        ),
        worst_rms_cross_axis_ratio: maximum_optional(
            metrics.iter().map(|metric| metric.rms_cross_axis_ratio),
        ),
        rejected_trace_ids,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirements(pole_pairs: u32, phase_zero_offset_rad: f64) -> EncoderRequirements {
        EncoderRequirements {
            pole_pairs,
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
            grid_id: format!("full-turn-{sample_count}-uniform-v0"),
            evidence_kind,
            samples,
        }
    }

    fn constant_amplitude(_: f64) -> f64 {
        1.0
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
        assert!(metrics.violations.contains(&RequirementViolation::PeakPhaseErrorExceeded));
        assert!(metrics.violations.contains(&RequirementViolation::RmsPhaseErrorExceeded));
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
        assert!(metrics.violations.contains(&RequirementViolation::BelowMinimumInPlaneField));
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
        assert!(metrics.violations.contains(&RequirementViolation::AmplitudeRippleExceeded));
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
        assert!(metrics.violations.contains(&RequirementViolation::CrossAxisRatioExceeded));
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
