//! Independent witness and numerical separation verifier for absolute-time CCD.
//!
//! This module does not call the producer's active-set minimizer. It reconstructs
//! world-space points directly from the original time-stamped motion spans. A
//! contact disposition is backed by a concrete common-time witness. A separated
//! disposition is backed by a numerical box-KKT check for the convex squared
//! distance objective over (time, moving-axis, obstacle-axis) in [0, 1]^3.
//!
//! KKT checking is a numerical optimality check, not directed-rounding arithmetic,
//! a formal proof, a physical safety function, or an authenticated attestation.

use crate::{CapsuleMotionSpan, TimedCoTranslatingClosestApproach, Vec3};

const CONTACT_EPSILON_MM: f64 = 1.0e-9;
const CANONICAL_DOMAIN: &[u8] = b"LUMINOUS-TIMED-CAPSULE-CCD-QUERY\0";
const CANONICAL_VERSION: u16 = 1;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct TimedCcdVerifierConfig {
    pub point_tolerance_mm: f64,
    pub distance_tolerance_mm: f64,
    pub time_offset_tolerance_ms: f64,
    pub parameter_tolerance: f64,
    pub kkt_gradient_tolerance_mm2: f64,
}

impl Default for TimedCcdVerifierConfig {
    fn default() -> Self {
        Self {
            point_tolerance_mm: 1.0e-7,
            distance_tolerance_mm: 1.0e-7,
            time_offset_tolerance_ms: 1.0e-7,
            parameter_tolerance: 1.0e-10,
            kkt_gradient_tolerance_mm2: 1.0e-6,
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimedCcdVerificationDisposition {
    ContactWitness,
    TerminalBoundaryCandidate,
    SeparatedKktChecked,
    Unresolved,
    NoTemporalOverlap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimedCcdVerificationReason {
    None,
    NoPositiveTemporalOverlap,
    ReportPresentWithoutTemporalOverlap,
    ContextMismatch,
    ClockUncertaintyUnsupported,
    MissingReportedResult,
    ReportedIntervalMismatch,
    InvalidReportedParameter,
    ReportedValueMismatch,
    KktConditionsFailed,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TimedCcdVerificationReceipt {
    pub canonical_source_query: Vec<u8>,
    pub reported_result: Option<TimedCoTranslatingClosestApproach>,
    pub config: TimedCcdVerifierConfig,
    pub disposition: TimedCcdVerificationDisposition,
    pub reason: TimedCcdVerificationReason,
    pub overlap_start_ms: Option<u64>,
    pub overlap_end_ms: Option<u64>,
    pub max_feasibility_residual: f64,
    pub max_point_residual_mm: f64,
    pub distance_residual_mm: f64,
    pub time_offset_residual_ms: f64,
    pub kkt_checked: bool,
    pub max_kkt_gradient_residual_mm2: f64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TimedCcdVerifierError {
    EmptyIdentity(&'static str),
    InvalidTimeInterval,
    InvalidGeometry,
    InvalidConfiguration,
    CanonicalLengthOverflow,
    ReceiptMismatch,
}

#[derive(Clone, Copy, Debug)]
struct V3 {
    x: f64,
    y: f64,
    z: f64,
}

impl V3 {
    fn from_public(value: Vec3) -> Self {
        Self {
            x: value.x,
            y: value.y,
            z: value.z,
        }
    }

    fn add(self, rhs: Self) -> Self {
        Self {
            x: self.x + rhs.x,
            y: self.y + rhs.y,
            z: self.z + rhs.z,
        }
    }

    fn sub(self, rhs: Self) -> Self {
        Self {
            x: self.x - rhs.x,
            y: self.y - rhs.y,
            z: self.z - rhs.z,
        }
    }

    fn scale(self, scalar: f64) -> Self {
        Self {
            x: self.x * scalar,
            y: self.y * scalar,
            z: self.z * scalar,
        }
    }

    fn dot(self, rhs: Self) -> f64 {
        self.x * rhs.x + self.y * rhs.y + self.z * rhs.z
    }

    fn norm(self) -> f64 {
        self.dot(self).sqrt()
    }

    fn finite(self) -> bool {
        self.x.is_finite() && self.y.is_finite() && self.z.is_finite()
    }
}

/// Independently check a timed CCD query result.
///
/// Pass None only when the source spans have no positive-duration overlap.
/// For overlapping spans, a missing result is Unresolved, never Clear.
pub fn verify_timed_co_translating_result(
    moving: &CapsuleMotionSpan,
    obstacle: &CapsuleMotionSpan,
    reported: Option<&TimedCoTranslatingClosestApproach>,
    config: TimedCcdVerifierConfig,
) -> Result<TimedCcdVerificationReceipt, TimedCcdVerifierError> {
    validate_config(config)?;
    validate_span(moving)?;
    validate_span(obstacle)?;
    let canonical_source_query = canonical_query_bytes(moving, obstacle)?;

    let mut receipt = TimedCcdVerificationReceipt {
        canonical_source_query,
        reported_result: reported.copied(),
        config,
        disposition: TimedCcdVerificationDisposition::Unresolved,
        reason: TimedCcdVerificationReason::None,
        overlap_start_ms: None,
        overlap_end_ms: None,
        max_feasibility_residual: 0.0,
        max_point_residual_mm: 0.0,
        distance_residual_mm: 0.0,
        time_offset_residual_ms: 0.0,
        kkt_checked: false,
        max_kkt_gradient_residual_mm2: 0.0,
    };

    if moving.site_epoch != obstacle.site_epoch
        || moving.frame_id != obstacle.frame_id
        || moving.clock_domain_id != obstacle.clock_domain_id
    {
        receipt.reason = TimedCcdVerificationReason::ContextMismatch;
        return Ok(receipt);
    }

    let overlap_start = moving.start_ms.max(obstacle.start_ms);
    let overlap_end = moving.end_ms.min(obstacle.end_ms);
    if overlap_start >= overlap_end {
        if reported.is_none() {
            receipt.disposition = TimedCcdVerificationDisposition::NoTemporalOverlap;
            receipt.reason = TimedCcdVerificationReason::NoPositiveTemporalOverlap;
        } else {
            receipt.reason = TimedCcdVerificationReason::ReportPresentWithoutTemporalOverlap;
        }
        return Ok(receipt);
    }

    receipt.overlap_start_ms = Some(overlap_start);
    receipt.overlap_end_ms = Some(overlap_end);

    // Timed CCD v0.1 rejects non-zero clock uncertainty. Preserve that boundary.
    if moving.clock_uncertainty_ms != 0 || obstacle.clock_uncertainty_ms != 0 {
        receipt.reason = TimedCcdVerificationReason::ClockUncertaintyUnsupported;
        return Ok(receipt);
    }

    let Some(reported) = reported else {
        receipt.reason = TimedCcdVerificationReason::MissingReportedResult;
        return Ok(receipt);
    };

    if reported.overlap_start_ms != overlap_start || reported.overlap_end_ms != overlap_end {
        receipt.reason = TimedCcdVerificationReason::ReportedIntervalMismatch;
        return Ok(receipt);
    }

    let time = reported.closest_time_parameter;
    let moving_axis = reported.closest.moving_axis_parameter;
    let obstacle_axis = reported.closest.obstacle_axis_parameter;
    if !time.is_finite() || !moving_axis.is_finite() || !obstacle_axis.is_finite() {
        receipt.reason = TimedCcdVerificationReason::InvalidReportedParameter;
        return Ok(receipt);
    }

    let feasibility = [
        bound_violation(time),
        bound_violation(moving_axis),
        bound_violation(obstacle_axis),
        (reported.closest.time_parameter - time).abs(),
    ]
    .into_iter()
    .fold(0.0_f64, f64::max);
    receipt.max_feasibility_residual = feasibility;
    if feasibility > config.parameter_tolerance {
        receipt.reason = TimedCcdVerificationReason::InvalidReportedParameter;
        return Ok(receipt);
    }

    let t = time.clamp(0.0, 1.0);
    let u = moving_axis.clamp(0.0, 1.0);
    let v = obstacle_axis.clamp(0.0, 1.0);
    let duration = (overlap_end - overlap_start) as f64;
    let moving_fraction = elapsed_fraction(moving, overlap_start, duration, t);
    let obstacle_fraction = elapsed_fraction(obstacle, overlap_start, duration, t);

    let moving_start = V3::from_public(moving.capsule_at_start.start)
        .add(V3::from_public(moving.translation).scale(moving_fraction));
    let moving_end = V3::from_public(moving.capsule_at_start.end)
        .add(V3::from_public(moving.translation).scale(moving_fraction));
    let obstacle_start = V3::from_public(obstacle.capsule_at_start.start)
        .add(V3::from_public(obstacle.translation).scale(obstacle_fraction));
    let obstacle_end = V3::from_public(obstacle.capsule_at_start.end)
        .add(V3::from_public(obstacle.translation).scale(obstacle_fraction));

    let moving_point = moving_start.add(moving_end.sub(moving_start).scale(u));
    let obstacle_point = obstacle_start.add(obstacle_end.sub(obstacle_start).scale(v));
    let distance = moving_point.sub(obstacle_point).norm();
    let reported_moving_point = V3::from_public(reported.closest.moving_point);
    let reported_obstacle_point = V3::from_public(reported.closest.obstacle_point);
    let reported_distance = reported.closest.distance_mm;

    let moving_point_residual = moving_point.sub(reported_moving_point).norm();
    let obstacle_point_residual = obstacle_point.sub(reported_obstacle_point).norm();
    receipt.max_point_residual_mm = moving_point_residual.max(obstacle_point_residual);
    receipt.distance_residual_mm = (distance - reported_distance).abs();
    receipt.time_offset_residual_ms = (duration * t - reported.closest_time_offset_ms).abs();

    let expected_terminal = time == 1.0;
    let terminal_flag_mismatch = reported.closest_at_terminal_limit != expected_terminal;
    let radius_sum = moving.capsule_at_start.radius_mm + obstacle.capsule_at_start.radius_mm;
    let values_finite = moving_start.finite()
        && moving_end.finite()
        && obstacle_start.finite()
        && obstacle_end.finite()
        && moving_point.finite()
        && obstacle_point.finite()
        && reported_moving_point.finite()
        && reported_obstacle_point.finite()
        && distance.is_finite()
        && reported_distance.is_finite()
        && reported.closest.time_parameter.is_finite()
        && radius_sum.is_finite()
        && reported_distance >= 0.0
        && reported.closest_time_offset_ms.is_finite();
    if !values_finite
        || terminal_flag_mismatch
        || receipt.max_point_residual_mm > config.point_tolerance_mm
        || receipt.distance_residual_mm > config.distance_tolerance_mm
        || receipt.time_offset_residual_ms > config.time_offset_tolerance_ms
    {
        receipt.reason = TimedCcdVerificationReason::ReportedValueMismatch;
        return Ok(receipt);
    }

    let contact_threshold = radius_sum + CONTACT_EPSILON_MM;
    if distance <= contact_threshold {
        receipt.disposition = if expected_terminal && distance >= radius_sum {
            TimedCcdVerificationDisposition::TerminalBoundaryCandidate
        } else {
            TimedCcdVerificationDisposition::ContactWitness
        };
        return Ok(receipt);
    }

    // The squared-distance objective is convex in (t, u, v). Its KKT signs
    // are sufficient for a global minimum in the exact model. This floating
    // implementation checks residuals only; it is not an interval proof.
    let relative = moving_point.sub(obstacle_point);
    let moving_axis_vector = moving_end.sub(moving_start);
    let obstacle_axis_vector = obstacle_end.sub(obstacle_start);
    let moving_time_vector = V3::from_public(moving.translation)
        .scale(duration / (moving.end_ms - moving.start_ms) as f64);
    let obstacle_time_vector = V3::from_public(obstacle.translation)
        .scale(duration / (obstacle.end_ms - obstacle.start_ms) as f64);
    let derivatives = [
        moving_time_vector.sub(obstacle_time_vector),
        moving_axis_vector,
        obstacle_axis_vector.scale(-1.0),
    ];
    let parameters = [t, u, v];
    let mut max_kkt_residual = 0.0_f64;

    receipt.kkt_checked = true;
    for (parameter, derivative) in parameters.into_iter().zip(derivatives) {
        let gradient = 2.0 * relative.dot(derivative);
        if !gradient.is_finite() {
            receipt.max_kkt_gradient_residual_mm2 = f64::INFINITY;
            receipt.reason = TimedCcdVerificationReason::KktConditionsFailed;
            return Ok(receipt);
        }
        let residual = if parameter <= config.parameter_tolerance {
            (-gradient).max(0.0)
        } else if parameter >= 1.0 - config.parameter_tolerance {
            gradient.max(0.0)
        } else {
            gradient.abs()
        };
        max_kkt_residual = max_kkt_residual.max(residual);
    }

    receipt.max_kkt_gradient_residual_mm2 = max_kkt_residual;
    if max_kkt_residual <= config.kkt_gradient_tolerance_mm2 {
        receipt.disposition = TimedCcdVerificationDisposition::SeparatedKktChecked;
    } else {
        receipt.reason = TimedCcdVerificationReason::KktConditionsFailed;
    }
    Ok(receipt)
}

/// Recompute a stored receipt from exact source spans and reported result.
pub fn replay_timed_ccd_verification_receipt(
    moving: &CapsuleMotionSpan,
    obstacle: &CapsuleMotionSpan,
    receipt: &TimedCcdVerificationReceipt,
) -> Result<(), TimedCcdVerifierError> {
    let replayed = verify_timed_co_translating_result(
        moving,
        obstacle,
        receipt.reported_result.as_ref(),
        receipt.config,
    )?;
    if replayed == *receipt {
        Ok(())
    } else {
        Err(TimedCcdVerifierError::ReceiptMismatch)
    }
}

fn validate_config(config: TimedCcdVerifierConfig) -> Result<(), TimedCcdVerifierError> {
    for value in [
        config.point_tolerance_mm,
        config.distance_tolerance_mm,
        config.time_offset_tolerance_ms,
        config.parameter_tolerance,
        config.kkt_gradient_tolerance_mm2,
    ] {
        if !value.is_finite() || value < 0.0 {
            return Err(TimedCcdVerifierError::InvalidConfiguration);
        }
    }
    Ok(())
}

fn validate_span(span: &CapsuleMotionSpan) -> Result<(), TimedCcdVerifierError> {
    if span.start_ms >= span.end_ms {
        return Err(TimedCcdVerifierError::InvalidTimeInterval);
    }
    for (name, value) in [
        ("site_epoch", span.site_epoch.as_str()),
        ("frame_id", span.frame_id.as_str()),
        ("clock_domain_id", span.clock_domain_id.as_str()),
    ] {
        if value.trim().is_empty() {
            return Err(TimedCcdVerifierError::EmptyIdentity(name));
        }
    }
    let finite = |v: Vec3| v.x.is_finite() && v.y.is_finite() && v.z.is_finite();
    if !span.capsule_at_start.radius_mm.is_finite()
        || span.capsule_at_start.radius_mm < 0.0
        || !finite(span.capsule_at_start.start)
        || !finite(span.capsule_at_start.end)
        || !finite(span.translation)
    {
        return Err(TimedCcdVerifierError::InvalidGeometry);
    }
    Ok(())
}

fn elapsed_fraction(
    span: &CapsuleMotionSpan,
    overlap_start_ms: u64,
    overlap_duration_ms: f64,
    normalized_overlap_time: f64,
) -> f64 {
    // Subtract integer timestamps before conversion to f64 to avoid losing
    // precision by converting large absolute monotonic timestamps.
    let elapsed_at_start = (overlap_start_ms - span.start_ms) as f64;
    let elapsed = elapsed_at_start + overlap_duration_ms * normalized_overlap_time;
    elapsed / (span.end_ms - span.start_ms) as f64
}

fn bound_violation(value: f64) -> f64 {
    (-value).max(0.0).max((value - 1.0).max(0.0))
}

fn canonical_query_bytes(
    moving: &CapsuleMotionSpan,
    obstacle: &CapsuleMotionSpan,
) -> Result<Vec<u8>, TimedCcdVerifierError> {
    let mut bytes = Vec::with_capacity(256);
    bytes.extend_from_slice(CANONICAL_DOMAIN);
    bytes.extend_from_slice(&CANONICAL_VERSION.to_le_bytes());
    encode_span(&mut bytes, moving)?;
    encode_span(&mut bytes, obstacle)?;
    Ok(bytes)
}

fn encode_span(bytes: &mut Vec<u8>, span: &CapsuleMotionSpan) -> Result<(), TimedCcdVerifierError> {
    bytes.extend_from_slice(&span.start_ms.to_le_bytes());
    bytes.extend_from_slice(&span.end_ms.to_le_bytes());
    bytes.extend_from_slice(&span.clock_uncertainty_ms.to_le_bytes());
    encode_string(bytes, &span.site_epoch)?;
    encode_string(bytes, &span.frame_id)?;
    encode_string(bytes, &span.clock_domain_id)?;
    encode_vec3(bytes, span.capsule_at_start.start)?;
    encode_vec3(bytes, span.capsule_at_start.end)?;
    encode_f64(bytes, span.capsule_at_start.radius_mm)?;
    encode_vec3(bytes, span.translation)?;
    Ok(())
}

fn encode_string(bytes: &mut Vec<u8>, value: &str) -> Result<(), TimedCcdVerifierError> {
    let length =
        u64::try_from(value.len()).map_err(|_| TimedCcdVerifierError::CanonicalLengthOverflow)?;
    bytes.extend_from_slice(&length.to_le_bytes());
    bytes.extend_from_slice(value.as_bytes());
    Ok(())
}

fn encode_vec3(bytes: &mut Vec<u8>, value: Vec3) -> Result<(), TimedCcdVerifierError> {
    encode_f64(bytes, value.x)?;
    encode_f64(bytes, value.y)?;
    encode_f64(bytes, value.z)
}

fn encode_f64(bytes: &mut Vec<u8>, value: f64) -> Result<(), TimedCcdVerifierError> {
    if !value.is_finite() {
        return Err(TimedCcdVerifierError::InvalidGeometry);
    }
    bytes.extend_from_slice(&value.to_bits().to_le_bytes());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Capsule, CoTranslatingCapsuleClosestApproach, timed_co_translating_capsule_vs_capsule,
    };

    fn point(x: f64, y: f64, z: f64) -> Vec3 {
        Vec3::new(x, y, z)
    }

    fn capsule(x: f64, radius: f64) -> Capsule {
        Capsule::new(point(x, -1.0, 0.0), point(x, 1.0, 0.0), radius).unwrap()
    }

    fn span(x: f64, dx: f64, start_ms: u64, end_ms: u64) -> CapsuleMotionSpan {
        CapsuleMotionSpan {
            capsule_at_start: capsule(x, 0.5),
            translation: point(dx, 0.0, 0.0),
            start_ms,
            end_ms,
            site_epoch: "site-epoch-7".into(),
            frame_id: "site-frame".into(),
            clock_domain_id: "site-clock".into(),
            clock_uncertainty_ms: 0,
        }
    }

    fn verify_pair(
        moving: &CapsuleMotionSpan,
        obstacle: &CapsuleMotionSpan,
    ) -> TimedCcdVerificationReceipt {
        let reported = timed_co_translating_capsule_vs_capsule(moving, obstacle).ok();
        verify_timed_co_translating_result(
            moving,
            obstacle,
            reported.as_ref(),
            TimedCcdVerifierConfig::default(),
        )
        .unwrap()
    }

    #[test]
    fn recomputes_an_interior_contact_witness() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 0, 1_000);
        let receipt = verify_pair(&moving, &obstacle);
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::ContactWitness
        );
        assert!(receipt.max_point_residual_mm <= 1.0e-7);
    }

    #[test]
    fn verifies_a_numerically_separated_kkt_result() {
        let moving = span(0.0, 0.0, 0, 1_000);
        let obstacle = span(5.0, 0.0, 0, 1_000);
        let receipt = verify_pair(&moving, &obstacle);
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::SeparatedKktChecked
        );
        assert!(receipt.max_kkt_gradient_residual_mm2 <= 1.0e-6);
    }

    #[test]
    fn contact_only_at_half_open_terminal_limit_is_not_an_attained_witness() {
        let moving = span(-10.0, 9.0, 0, 1_000);
        let obstacle = span(0.0, 0.0, 0, 1_000);
        let receipt = verify_pair(&moving, &obstacle);
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::TerminalBoundaryCandidate
        );
    }

    #[test]
    fn no_temporal_overlap_is_not_reported_as_geometric_clearance() {
        let moving = span(-10.0, 0.0, 0, 100);
        let obstacle = span(10.0, 0.0, 100, 200);
        let receipt = verify_pair(&moving, &obstacle);
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::NoTemporalOverlap
        );
    }

    #[test]
    fn a_present_result_for_non_overlapping_spans_is_unresolved() {
        let moving = span(-10.0, 0.0, 0, 100);
        let obstacle = span(10.0, 0.0, 100, 200);
        let fake = TimedCoTranslatingClosestApproach {
            overlap_start_ms: 0,
            overlap_end_ms: 1,
            closest_time_offset_ms: 0.0,
            closest_time_parameter: 0.0,
            closest_at_terminal_limit: false,
            closest: CoTranslatingCapsuleClosestApproach {
                time_parameter: 0.0,
                moving_axis_parameter: 0.0,
                obstacle_axis_parameter: 0.0,
                moving_point: point(-10.0, -1.0, 0.0),
                obstacle_point: point(10.0, -1.0, 0.0),
                distance_mm: 20.0,
            },
        };
        let receipt = verify_timed_co_translating_result(
            &moving,
            &obstacle,
            Some(&fake),
            TimedCcdVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::Unresolved
        );
    }

    #[test]
    fn tampered_world_point_cannot_become_a_contact_or_clear_result() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 0, 1_000);
        let mut reported = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();
        reported.closest.moving_point.x += 5.0;
        let receipt = verify_timed_co_translating_result(
            &moving,
            &obstacle,
            Some(&reported),
            TimedCcdVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::Unresolved
        );
        assert_eq!(
            receipt.reason,
            TimedCcdVerificationReason::ReportedValueMismatch
        );
    }

    #[test]
    fn nonzero_clock_uncertainty_abstains_instead_of_being_ignored() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let mut obstacle = span(10.0, -20.0, 0, 1_000);
        obstacle.clock_uncertainty_ms = 1;
        let receipt = verify_timed_co_translating_result(
            &moving,
            &obstacle,
            None,
            TimedCcdVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::Unresolved
        );
        assert_eq!(
            receipt.reason,
            TimedCcdVerificationReason::ClockUncertaintyUnsupported
        );
    }

    #[test]
    fn mismatched_site_frame_or_clock_domain_abstains() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let mut obstacle = span(10.0, -20.0, 0, 1_000);
        obstacle.frame_id = "other-frame".into();
        let receipt = verify_timed_co_translating_result(
            &moving,
            &obstacle,
            None,
            TimedCcdVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::Unresolved
        );
        assert_eq!(receipt.reason, TimedCcdVerificationReason::ContextMismatch);
    }

    #[test]
    fn replay_is_deterministic_and_detects_receipt_tampering() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 0, 1_000);
        let receipt = verify_pair(&moving, &obstacle);
        replay_timed_ccd_verification_receipt(&moving, &obstacle, &receipt).unwrap();

        let mut tampered = receipt;
        tampered.max_point_residual_mm = 10.0;
        assert_eq!(
            replay_timed_ccd_verification_receipt(&moving, &obstacle, &tampered),
            Err(TimedCcdVerifierError::ReceiptMismatch)
        );
    }

    #[test]
    fn canonical_query_bytes_bind_timestamps_geometry_and_context() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 0, 1_000);
        let baseline = canonical_query_bytes(&moving, &obstacle).unwrap();

        let mut changed = obstacle.clone();
        changed.end_ms += 1;
        assert_ne!(baseline, canonical_query_bytes(&moving, &changed).unwrap());

        changed = obstacle.clone();
        changed.frame_id = "another-frame".into();
        assert_ne!(baseline, canonical_query_bytes(&moving, &changed).unwrap());
    }

    #[test]
    fn inconsistent_time_offset_is_unresolved() {
        let moving = span(-10.0, 20.0, 0, 1_000);
        let obstacle = span(10.0, -20.0, 0, 1_000);
        let mut reported = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();
        reported.closest_time_offset_ms += 1.0;
        let receipt = verify_timed_co_translating_result(
            &moving,
            &obstacle,
            Some(&reported),
            TimedCcdVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::Unresolved
        );
        assert_eq!(
            receipt.reason,
            TimedCcdVerificationReason::ReportedValueMismatch
        );
    }

    #[test]
    fn a_consistent_but_non_minimizing_separated_point_fails_kkt() {
        let moving = span(0.0, 4.0, 0, 1_000);
        let obstacle = span(10.0, 0.0, 0, 1_000);
        let mut reported = timed_co_translating_capsule_vs_capsule(&moving, &obstacle).unwrap();
        let u = reported.closest.moving_axis_parameter;
        let v = reported.closest.obstacle_axis_parameter;
        reported.closest_time_parameter = 0.0;
        reported.closest_time_offset_ms = 0.0;
        reported.closest_at_terminal_limit = false;
        reported.closest.time_parameter = 0.0;
        reported.closest.moving_point = point(0.0, -1.0 + 2.0 * u, 0.0);
        reported.closest.obstacle_point = point(10.0, -1.0 + 2.0 * v, 0.0);
        reported.closest.distance_mm = 10.0;

        let receipt = verify_timed_co_translating_result(
            &moving,
            &obstacle,
            Some(&reported),
            TimedCcdVerifierConfig::default(),
        )
        .unwrap();
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::Unresolved
        );
        assert_eq!(
            receipt.reason,
            TimedCcdVerificationReason::KktConditionsFailed
        );
        assert!(receipt.kkt_checked);
        assert!(receipt.max_kkt_gradient_residual_mm2 > 1.0e-6);
    }

    #[test]
    fn strict_contact_at_terminal_limit_implies_an_attained_preterminal_witness() {
        let moving = span(-10.0, 9.5, 0, 1_000);
        let obstacle = span(0.0, 0.0, 0, 1_000);
        let receipt = verify_pair(&moving, &obstacle);
        assert_eq!(
            receipt.disposition,
            TimedCcdVerificationDisposition::ContactWitness
        );
    }

}
