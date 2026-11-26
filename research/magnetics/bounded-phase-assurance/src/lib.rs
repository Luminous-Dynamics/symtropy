//! Distribution-free, deterministic bounded-error assurance for magnetic encoder phase.
//!
//! This crate does not estimate probability or infer uncertainty bounds. It
//! consumes independently established absolute bounds and propagates them
//! conservatively into an encoder's field-to-phase function.

use std::f64::consts::{PI, TAU};

const ANGLE_GRID_TOLERANCE_RAD: f64 = 1.0e-9;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EvidenceKind {
    SyntheticScenario,
    PhysicalBuild,
}

/// One sensor-plane observation and a deterministic bound on its in-plane
/// vector error. The true (Bx, By) must lie in the closed Euclidean disk
/// centred on the supplied vector with this radius.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BoundedFieldSample {
    pub mechanical_angle_rad: f64,
    pub bx_t: f64,
    pub by_t: f64,
    pub field_error_radius_t: f64,
}

/// A full-revolution field trace. The error-bound record must describe a
/// simultaneous, absolute bound for every sample in this trace.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundedFieldTrace {
    pub trace_id: String,
    pub frame_id: String,
    pub operating_condition_id: String,
    pub grid_id: String,
    pub field_error_bound_record_id: String,
    pub evidence_kind: EvidenceKind,
    pub samples: Vec<BoundedFieldSample>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedEncoderRequirements {
    pub pole_pairs: u32,
    /// Electrical phase progression for increasing mechanical angle: +1 or -1.
    pub phase_rotation_sense: i8,
    pub phase_zero_calibration_id: String,
    pub phase_zero_offset_rad: f64,
    pub minimum_true_in_plane_field_t: f64,
    pub maximum_peak_phase_error_rad: f64,
    pub maximum_rms_phase_error_rad: f64,
}

/// Absolute angular error bounds, not standard deviations or confidence
/// intervals. The phase-zero identity must match the requirements record.
#[derive(Clone, Debug, PartialEq)]
pub struct BoundedAngularErrorRecord {
    pub uncertainty_record_id: String,
    pub trace_id: String,
    pub frame_id: String,
    pub operating_condition_id: String,
    pub grid_id: String,
    pub phase_zero_calibration_id: String,
    pub mechanical_angle_error_bound_rad: f64,
    pub phase_zero_error_bound_rad: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BoundedViolation {
    MinimumFieldNotGuaranteed,
    PeakPhaseErrorNotGuaranteed,
    RmsPhaseErrorNotGuaranteed,
    PhaseUndefinedAtOrWithinFieldErrorDisk,
}

#[derive(Clone, Debug, PartialEq)]
pub enum BoundedEvaluationError {
    InvalidMetadata,
    InvalidRequirements,
    InvalidAngularErrorRecord,
    AngularTraceIdentityMismatch,
    AngularFrameIdentityMismatch,
    AngularConditionIdentityMismatch,
    AngularGridIdentityMismatch,
    AngularCalibrationIdentityMismatch,
    TooFewSamples,
    NonFiniteSample { index: usize },
    NegativeFieldErrorRadius { index: usize },
    NonMonotonicAngleGrid { index: usize },
    NonUniformOrIncompleteRevolution { index: usize },
    NumericOverflow { index: usize },
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedSampleMetrics {
    pub sample_index: usize,
    pub mechanical_angle_rad: f64,
    pub measured_in_plane_field_t: f64,
    pub field_error_radius_t: f64,
    pub guaranteed_true_field_lower_bound_t: f64,
    /// asin(r / |B|), or pi when the error disk reaches the origin.
    pub field_phase_error_bound_rad: f64,
    pub nominal_signed_phase_residual_rad: f64,
    /// Sum of field, pole-pair-scaled reference, and phase-zero bounds.
    pub total_phase_error_bound_rad: f64,
    /// Conservative upper bound on circular absolute phase error.
    pub absolute_phase_error_upper_bound_rad: f64,
    pub phase_defined_for_all_admissible_fields: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BoundedTraceMetrics {
    pub trace_id: String,
    pub frame_id: String,
    pub operating_condition_id: String,
    pub grid_id: String,
    pub field_error_bound_record_id: String,
    pub uncertainty_record_id: String,
    pub phase_zero_calibration_id: String,
    pub evidence_kind: EvidenceKind,
    pub sample_count: usize,
    pub phase_undefined_sample_count: usize,
    pub minimum_guaranteed_true_field_t: f64,
    pub maximum_field_phase_error_bound_rad: f64,
    pub mechanical_phase_error_bound_rad: f64,
    pub phase_zero_error_bound_rad: f64,
    pub maximum_total_phase_error_bound_rad: f64,
    pub peak_absolute_phase_error_upper_bound_rad: f64,
    pub rms_absolute_phase_error_upper_bound_rad: f64,
    /// Nearest-rank p95 of the per-sample conservative absolute-error bounds.
    pub p95_absolute_phase_error_upper_bound_rad: f64,
    pub minimum_field_margin_t: f64,
    pub peak_phase_error_margin_rad: f64,
    pub rms_phase_error_margin_rad: f64,
    pub accepted: bool,
    pub violations: Vec<BoundedViolation>,
    /// Retained per-sample results make the aggregate independently auditable.
    pub samples: Vec<BoundedSampleMetrics>,
}

fn nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn wrap_to_pi(angle: f64) -> f64 {
    (angle + PI).rem_euclid(TAU) - PI
}

fn validate_requirements(
    requirements: &BoundedEncoderRequirements,
) -> Result<(), BoundedEvaluationError> {
    if requirements.pole_pairs == 0
        || !matches!(requirements.phase_rotation_sense, -1 | 1)
        || !nonempty(&requirements.phase_zero_calibration_id)
        || !requirements.phase_zero_offset_rad.is_finite()
        || !requirements.minimum_true_in_plane_field_t.is_finite()
        || requirements.minimum_true_in_plane_field_t <= 0.0
        || !requirements.maximum_peak_phase_error_rad.is_finite()
        || !(0.0..=PI).contains(&requirements.maximum_peak_phase_error_rad)
        || !requirements.maximum_rms_phase_error_rad.is_finite()
        || !(0.0..=PI).contains(&requirements.maximum_rms_phase_error_rad)
    {
        return Err(BoundedEvaluationError::InvalidRequirements);
    }
    Ok(())
}

fn validate_trace(trace: &BoundedFieldTrace) -> Result<(), BoundedEvaluationError> {
    if !nonempty(&trace.trace_id)
        || !nonempty(&trace.frame_id)
        || !nonempty(&trace.operating_condition_id)
        || !nonempty(&trace.grid_id)
        || !nonempty(&trace.field_error_bound_record_id)
    {
        return Err(BoundedEvaluationError::InvalidMetadata);
    }

    let count = trace.samples.len();
    if count < 3 {
        return Err(BoundedEvaluationError::TooFewSamples);
    }

    let first_angle = trace.samples[0].mechanical_angle_rad;
    if !first_angle.is_finite() {
        return Err(BoundedEvaluationError::NonFiniteSample { index: 0 });
    }
    let step = TAU / count as f64;

    for (index, sample) in trace.samples.iter().enumerate() {
        if !sample.mechanical_angle_rad.is_finite()
            || !sample.bx_t.is_finite()
            || !sample.by_t.is_finite()
            || !sample.field_error_radius_t.is_finite()
        {
            return Err(BoundedEvaluationError::NonFiniteSample { index });
        }
        if sample.field_error_radius_t < 0.0 {
            return Err(BoundedEvaluationError::NegativeFieldErrorRadius { index });
        }
        if index > 0
            && sample.mechanical_angle_rad <= trace.samples[index - 1].mechanical_angle_rad
        {
            return Err(BoundedEvaluationError::NonMonotonicAngleGrid { index });
        }

        let expected_angle = first_angle + index as f64 * step;
        if (sample.mechanical_angle_rad - expected_angle).abs()
            > ANGLE_GRID_TOLERANCE_RAD
        {
            return Err(BoundedEvaluationError::NonUniformOrIncompleteRevolution { index });
        }

        if !sample.bx_t.hypot(sample.by_t).is_finite() {
            return Err(BoundedEvaluationError::NonFiniteSample { index });
        }
    }

    Ok(())
}

fn validate_angular_error_record(
    record: &BoundedAngularErrorRecord,
    trace: &BoundedFieldTrace,
    requirements: &BoundedEncoderRequirements,
) -> Result<(), BoundedEvaluationError> {
    if !nonempty(&record.uncertainty_record_id)
        || !nonempty(&record.trace_id)
        || !nonempty(&record.frame_id)
        || !nonempty(&record.operating_condition_id)
        || !nonempty(&record.grid_id)
        || !nonempty(&record.phase_zero_calibration_id)
        || !record.mechanical_angle_error_bound_rad.is_finite()
        || record.mechanical_angle_error_bound_rad < 0.0
        || !record.phase_zero_error_bound_rad.is_finite()
        || record.phase_zero_error_bound_rad < 0.0
    {
        return Err(BoundedEvaluationError::InvalidAngularErrorRecord);
    }

    if record.trace_id != trace.trace_id {
        return Err(BoundedEvaluationError::AngularTraceIdentityMismatch);
    }
    if record.frame_id != trace.frame_id {
        return Err(BoundedEvaluationError::AngularFrameIdentityMismatch);
    }
    if record.operating_condition_id != trace.operating_condition_id {
        return Err(BoundedEvaluationError::AngularConditionIdentityMismatch);
    }
    if record.grid_id != trace.grid_id {
        return Err(BoundedEvaluationError::AngularGridIdentityMismatch);
    }
    if record.phase_zero_calibration_id != requirements.phase_zero_calibration_id {
        return Err(BoundedEvaluationError::AngularCalibrationIdentityMismatch);
    }

    Ok(())
}

/// Evaluate whether every field sample satisfies the encoder requirements
/// for all realizations inside the supplied deterministic error bounds.
///
/// For measured vector b and a Euclidean error disk of radius r, when
/// r < ||b||, the maximum angular deviation is asin(r / ||b||), and the true
/// field magnitude is at least ||b|| - r. If the disk reaches the origin,
/// the phase is not guaranteed to be defined and a conservative pi-radian
/// phase-error ceiling is reported.
///
/// The field, mechanical-reference and phase-zero bounds are added as
/// absolute worst-case contributions. No independence or probability
/// distribution is assumed. The caller is responsible for proving that the
/// supplied bounds are simultaneously valid for the evaluated trace.
pub fn evaluate_bounded_trace(
    trace: &BoundedFieldTrace,
    requirements: &BoundedEncoderRequirements,
    angular_error: &BoundedAngularErrorRecord,
) -> Result<BoundedTraceMetrics, BoundedEvaluationError> {
    validate_requirements(requirements)?;
    validate_trace(trace)?;
    validate_angular_error_record(angular_error, trace, requirements)?;

    let mechanical_phase_bound =
        requirements.pole_pairs as f64 * angular_error.mechanical_angle_error_bound_rad;
    if !mechanical_phase_bound.is_finite() {
        return Err(BoundedEvaluationError::InvalidAngularErrorRecord);
    }

    let mut sample_metrics = Vec::with_capacity(trace.samples.len());
    let mut phase_undefined_sample_count = 0_usize;

    for (index, sample) in trace.samples.iter().enumerate() {
        let measured_magnitude = sample.bx_t.hypot(sample.by_t);
        let radius = sample.field_error_radius_t;

        let (field_lower_bound, field_phase_bound, phase_defined) =
            if measured_magnitude > radius {
                (
                    measured_magnitude - radius,
                    (radius / measured_magnitude).asin(),
                    true,
                )
            } else {
                // The admissible disk touches or contains zero. A positive
                // field floor cannot be guaranteed, and phase may be undefined.
                (0.0, PI, false)
            };

        if !field_lower_bound.is_finite() || !field_phase_bound.is_finite() {
            return Err(BoundedEvaluationError::NumericOverflow { index });
        }

        let expected_phase = requirements.phase_rotation_sense as f64
            * requirements.pole_pairs as f64
            * sample.mechanical_angle_rad
            + requirements.phase_zero_offset_rad;
        if !expected_phase.is_finite() {
            return Err(BoundedEvaluationError::NumericOverflow { index });
        }

        let nominal_phase = sample.by_t.atan2(sample.bx_t);
        let nominal_residual = wrap_to_pi(nominal_phase - expected_phase);
        let total_phase_bound =
            field_phase_bound + mechanical_phase_bound + angular_error.phase_zero_error_bound_rad;
        if !nominal_residual.is_finite() || !total_phase_bound.is_finite() {
            return Err(BoundedEvaluationError::NumericOverflow { index });
        }

        let absolute_error_upper = (nominal_residual.abs() + total_phase_bound).min(PI);
        if !absolute_error_upper.is_finite() {
            return Err(BoundedEvaluationError::NumericOverflow { index });
        }

        if !phase_defined {
            phase_undefined_sample_count += 1;
        }

        sample_metrics.push(BoundedSampleMetrics {
            sample_index: index,
            mechanical_angle_rad: sample.mechanical_angle_rad,
            measured_in_plane_field_t: measured_magnitude,
            field_error_radius_t: radius,
            guaranteed_true_field_lower_bound_t: field_lower_bound,
            field_phase_error_bound_rad: field_phase_bound,
            nominal_signed_phase_residual_rad: nominal_residual,
            total_phase_error_bound_rad: total_phase_bound,
            absolute_phase_error_upper_bound_rad: absolute_error_upper,
            phase_defined_for_all_admissible_fields: phase_defined,
        });
    }

    let minimum_field = sample_metrics
        .iter()
        .map(|sample| sample.guaranteed_true_field_lower_bound_t)
        .fold(f64::INFINITY, f64::min);

    let maximum_field_phase_bound = sample_metrics
        .iter()
        .map(|sample| sample.field_phase_error_bound_rad)
        .fold(0.0_f64, f64::max);

    let maximum_total_phase_bound = sample_metrics
        .iter()
        .map(|sample| sample.total_phase_error_bound_rad)
        .fold(0.0_f64, f64::max);

    let mut absolute_error_upper_bounds = sample_metrics
        .iter()
        .map(|sample| sample.absolute_phase_error_upper_bound_rad)
        .collect::<Vec<_>>();

    let peak_upper = absolute_error_upper_bounds
        .iter()
        .copied()
        .fold(0.0_f64, f64::max);

    let sum_squares = absolute_error_upper_bounds
        .iter()
        .map(|value| value * value)
        .sum::<f64>();
    let rms_upper = (sum_squares / absolute_error_upper_bounds.len() as f64).sqrt();

    absolute_error_upper_bounds.sort_by(f64::total_cmp);
    let nearest_rank = ((0.95 * absolute_error_upper_bounds.len() as f64).ceil() as usize)
        .max(1);
    let p95_upper = absolute_error_upper_bounds[nearest_rank - 1];

    let minimum_field_margin =
        minimum_field - requirements.minimum_true_in_plane_field_t;
    let peak_margin = requirements.maximum_peak_phase_error_rad - peak_upper;
    let rms_margin = requirements.maximum_rms_phase_error_rad - rms_upper;

    let mut violations = Vec::new();
    if minimum_field_margin < 0.0 {
        violations.push(BoundedViolation::MinimumFieldNotGuaranteed);
    }
    if peak_margin < 0.0 {
        violations.push(BoundedViolation::PeakPhaseErrorNotGuaranteed);
    }
    if rms_margin < 0.0 {
        violations.push(BoundedViolation::RmsPhaseErrorNotGuaranteed);
    }
    if phase_undefined_sample_count > 0 {
        violations.push(BoundedViolation::PhaseUndefinedAtOrWithinFieldErrorDisk);
    }

    Ok(BoundedTraceMetrics {
        trace_id: trace.trace_id.clone(),
        frame_id: trace.frame_id.clone(),
        operating_condition_id: trace.operating_condition_id.clone(),
        grid_id: trace.grid_id.clone(),
        field_error_bound_record_id: trace.field_error_bound_record_id.clone(),
        uncertainty_record_id: angular_error.uncertainty_record_id.clone(),
        phase_zero_calibration_id: angular_error.phase_zero_calibration_id.clone(),
        evidence_kind: trace.evidence_kind,
        sample_count: sample_metrics.len(),
        phase_undefined_sample_count,
        minimum_guaranteed_true_field_t: minimum_field,
        maximum_field_phase_error_bound_rad: maximum_field_phase_bound,
        mechanical_phase_error_bound_rad: mechanical_phase_bound,
        phase_zero_error_bound_rad: angular_error.phase_zero_error_bound_rad,
        maximum_total_phase_error_bound_rad: maximum_total_phase_bound,
        peak_absolute_phase_error_upper_bound_rad: peak_upper,
        rms_absolute_phase_error_upper_bound_rad: rms_upper,
        p95_absolute_phase_error_upper_bound_rad: p95_upper,
        minimum_field_margin_t: minimum_field_margin,
        peak_phase_error_margin_rad: peak_margin,
        rms_phase_error_margin_rad: rms_margin,
        accepted: violations.is_empty(),
        violations,
        samples: sample_metrics,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn requirements(pole_pairs: u32, phase_rotation_sense: i8) -> BoundedEncoderRequirements {
        BoundedEncoderRequirements {
            pole_pairs,
            phase_rotation_sense,
            phase_zero_calibration_id: "phase-zero-cal-001".to_owned(),
            phase_zero_offset_rad: 0.0,
            minimum_true_in_plane_field_t: 0.1,
            maximum_peak_phase_error_rad: 0.2,
            maximum_rms_phase_error_rad: 0.15,
        }
    }

    fn angular_error(
        mechanical_angle_error_bound_rad: f64,
        phase_zero_error_bound_rad: f64,
    ) -> BoundedAngularErrorRecord {
        BoundedAngularErrorRecord {
            uncertainty_record_id: "angle-bound-record-001".to_owned(),
            trace_id: "trace-001".to_owned(),
            frame_id: "sensor-frame-001".to_owned(),
            operating_condition_id: "condition-001".to_owned(),
            grid_id: "grid-16-uniform".to_owned(),
            phase_zero_calibration_id: "phase-zero-cal-001".to_owned(),
            mechanical_angle_error_bound_rad,
            phase_zero_error_bound_rad,
        }
    }

    fn trace(pole_pairs: u32, phase_rotation_sense: i8) -> BoundedFieldTrace {
        let samples = (0..16)
            .map(|index| {
                let theta = TAU * index as f64 / 16.0;
                let phase =
                    phase_rotation_sense as f64 * pole_pairs as f64 * theta;
                BoundedFieldSample {
                    mechanical_angle_rad: theta,
                    bx_t: phase.cos(),
                    by_t: phase.sin(),
                    field_error_radius_t: 0.0,
                }
            })
            .collect();

        BoundedFieldTrace {
            trace_id: "trace-001".to_owned(),
            frame_id: "sensor-frame-001".to_owned(),
            operating_condition_id: "condition-001".to_owned(),
            grid_id: "grid-16-uniform".to_owned(),
            field_error_bound_record_id: "field-bound-record-001".to_owned(),
            evidence_kind: EvidenceKind::SyntheticScenario,
            samples,
        }
    }

    #[test]
    fn exact_trace_with_zero_bounds_is_accepted() {
        let result = evaluate_bounded_trace(
            &trace(2, 1),
            &requirements(2, 1),
            &angular_error(0.0, 0.0),
        )
        .expect("valid trace");

        assert!(result.accepted);
        assert!(result.peak_absolute_phase_error_upper_bound_rad < 1.0e-12);
        assert!((result.minimum_guaranteed_true_field_t - 1.0).abs() < 1.0e-12);
    }

    #[test]
    fn disk_tangent_bound_matches_closed_form() {
        let mut input = trace(2, 1);
        for sample in &mut input.samples {
            sample.bx_t = (2.0 * sample.mechanical_angle_rad).cos();
            sample.by_t = (2.0 * sample.mechanical_angle_rad).sin();
            sample.field_error_radius_t = 0.5;
        }

        let result = evaluate_bounded_trace(
            &input,
            &requirements(2, 1),
            &angular_error(0.0, 0.0),
        )
        .expect("valid trace");

        let expected = 0.5_f64.asin();
        assert!((result.maximum_field_phase_error_bound_rad - expected).abs() < 1.0e-12);
        assert!((result.peak_absolute_phase_error_upper_bound_rad - expected).abs() < 1.0e-12);
        assert!((result.minimum_guaranteed_true_field_t - 0.5).abs() < 1.0e-12);
    }

    #[test]
    fn mechanical_reference_bound_scales_with_pole_pairs() {
        let result = evaluate_bounded_trace(
            &trace(10, 1),
            &requirements(10, 1),
            &angular_error(0.001, 0.002),
        )
        .expect("valid trace");

        assert!((result.mechanical_phase_error_bound_rad - 0.01).abs() < 1.0e-12);
        assert!((result.maximum_total_phase_error_bound_rad - 0.012).abs() < 1.0e-12);
    }

    #[test]
    fn phase_zero_error_is_kept_as_a_separate_bound() {
        let result = evaluate_bounded_trace(
            &trace(4, 1),
            &requirements(4, 1),
            &angular_error(0.0, 0.003),
        )
        .expect("valid trace");

        assert!((result.phase_zero_error_bound_rad - 0.003).abs() < 1.0e-12);
        assert!((result.maximum_total_phase_error_bound_rad - 0.003).abs() < 1.0e-12);
    }

    #[test]
    fn reversed_phase_progression_is_supported() {
        let result = evaluate_bounded_trace(
            &trace(3, -1),
            &requirements(3, -1),
            &angular_error(0.0, 0.0),
        )
        .expect("valid reverse-direction trace");

        assert!(result.accepted);
    }

    #[test]
    fn reaching_the_origin_fails_field_and_phase_guarantees() {
        let mut input = trace(2, 1);
        for sample in &mut input.samples {
            sample.field_error_radius_t = 1.0;
        }

        let result = evaluate_bounded_trace(
            &input,
            &requirements(2, 1),
            &angular_error(0.0, 0.0),
        )
        .expect("finite but non-qualifying trace");

        assert_eq!(result.phase_undefined_sample_count, input.samples.len());
        assert_eq!(result.minimum_guaranteed_true_field_t, 0.0);
        assert_eq!(result.peak_absolute_phase_error_upper_bound_rad, PI);
        assert!(!result.accepted);
        assert!(result.violations.contains(&BoundedViolation::MinimumFieldNotGuaranteed));
        assert!(result.violations.contains(&BoundedViolation::PhaseUndefinedAtOrWithinFieldErrorDisk));
    }

    #[test]
    fn uncertainty_can_turn_a_nominal_pass_into_a_robust_failure() {
        let mut input = trace(2, 1);
        for sample in &mut input.samples {
            sample.field_error_radius_t = 0.1;
        }
        let mut req = requirements(2, 1);
        req.maximum_peak_phase_error_rad = 0.05;

        let result = evaluate_bounded_trace(
            &input,
            &req,
            &angular_error(0.0, 0.0),
        )
        .expect("valid trace");

        assert!(!result.accepted);
        assert!(result.peak_absolute_phase_error_upper_bound_rad > 0.05);
        assert!(result.violations.contains(&BoundedViolation::PeakPhaseErrorNotGuaranteed));
    }

    #[test]
    fn calibration_identity_mismatch_is_rejected() {
        let mut record = angular_error(0.0, 0.0);
        record.phase_zero_calibration_id = "different-calibration".to_owned();

        assert!(matches!(
            evaluate_bounded_trace(&trace(2, 1), &requirements(2, 1), &record),
            Err(BoundedEvaluationError::AngularCalibrationIdentityMismatch)
        ));
    }

    #[test]
    fn angular_error_record_must_match_exact_trace_identity() {
        let mut record = angular_error(0.0, 0.0);
        record.trace_id = "different-trace".to_owned();

        assert!(matches!(
            evaluate_bounded_trace(&trace(2, 1), &requirements(2, 1), &record),
            Err(BoundedEvaluationError::AngularTraceIdentityMismatch)
        ));
    }

    #[test]
    fn angular_error_record_must_match_frame_condition_and_grid() {
        let input = trace(2, 1);
        let req = requirements(2, 1);

        let mut wrong_frame = angular_error(0.0, 0.0);
        wrong_frame.frame_id = "other-frame".to_owned();
        assert!(matches!(
            evaluate_bounded_trace(&input, &req, &wrong_frame),
            Err(BoundedEvaluationError::AngularFrameIdentityMismatch)
        ));

        let mut wrong_condition = angular_error(0.0, 0.0);
        wrong_condition.operating_condition_id = "other-condition".to_owned();
        assert!(matches!(
            evaluate_bounded_trace(&input, &req, &wrong_condition),
            Err(BoundedEvaluationError::AngularConditionIdentityMismatch)
        ));

        let mut wrong_grid = angular_error(0.0, 0.0);
        wrong_grid.grid_id = "other-grid".to_owned();
        assert!(matches!(
            evaluate_bounded_trace(&input, &req, &wrong_grid),
            Err(BoundedEvaluationError::AngularGridIdentityMismatch)
        ));
    }

    #[test]
    fn nonuniform_grid_is_rejected() {
        let mut input = trace(2, 1);
        input.samples[4].mechanical_angle_rad += 0.001;

        assert!(matches!(
            evaluate_bounded_trace(&input, &requirements(2, 1), &angular_error(0.0, 0.0)),
            Err(BoundedEvaluationError::NonUniformOrIncompleteRevolution { .. })
        ));
    }

    #[test]
    fn negative_field_error_radius_is_rejected() {
        let mut input = trace(2, 1);
        input.samples[3].field_error_radius_t = -1.0;

        assert!(matches!(
            evaluate_bounded_trace(
                &input,
                &requirements(2, 1),
                &angular_error(0.0, 0.0)
            ),
            Err(BoundedEvaluationError::NegativeFieldErrorRadius { index: 3 })
        ));
    }

    #[test]
    fn error_bounds_are_not_combined_as_independent_variances() {
        let result = evaluate_bounded_trace(
            &trace(8, 1),
            &requirements(8, 1),
            &angular_error(0.01, 0.02),
        )
        .expect("valid trace");

        // Worst-case absolute errors add, irrespective of assumed correlation.
        assert!((result.maximum_total_phase_error_bound_rad - 0.10).abs() < 1.0e-12);
    }

    #[test]
    fn field_disk_bound_matches_an_independent_tangent_point() {
        let magnitude = 1.7_f64;
        let radius = 0.63_f64;
        let ratio = radius / magnitude;
        let dx = -(radius * radius) / magnitude;
        let dy = radius * (1.0 - ratio * ratio).sqrt();
        let angle = dy.atan2(magnitude + dx);
        let expected = ratio.asin();

        assert!((angle - expected).abs() < 1.0e-12);
    }
}
