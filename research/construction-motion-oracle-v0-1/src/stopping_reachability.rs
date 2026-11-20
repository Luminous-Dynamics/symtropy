//! Bounded one-dimensional stopping reachability for construction robots.
//!
//! The caller supplies independently qualified upper/lower bounds for
//! detection-to-brake latency, positive acceleration, braking jerk and net
//! deceleration. This kernel computes the corresponding jerk-limited stop
//! extension. The caller projects it along the motion tangent and sweeps the
//! complete robot/tool/payload geometry over that stop tube.
//!
//! This is a research reference, not a safety-rated braking function. Its
//! conditional bound is valid only while the supplied qualification covers the
//! exact robot, brake, payload, surface, grade and environmental conditions.

const NUMERIC_DISTANCE_GUARD_MM: f64 = 1.0;
const NUMERIC_TIME_GUARD_MS: f64 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct StoppingDynamicsQualification {
    pub profile_sha256: String,
    pub qualification_receipt_sha256: String,
    pub robot_brake_configuration_sha256: String,
    pub operating_condition_sha256: String,
    pub maximum_qualified_speed_mm_s: f64,
    pub maximum_detection_to_brake_latency_ms: u64,
    pub maximum_positive_acceleration_mm_s2: f64,
    /// Minimum guaranteed net deceleration after the jerk ramp. The bound
    /// must include grade, surface, load, brake and environmental effects.
    pub minimum_net_deceleration_mm_s2: f64,
    /// Minimum guaranteed deceleration build-up rate from zero.
    pub minimum_braking_jerk_mm_s3: f64,
    pub additional_distance_guard_mm: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoppingReachabilityRequest {
    pub speed_upper_mm_s: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum StoppingReachabilityError {
    InvalidProfileDigest,
    InvalidQualificationReceiptDigest,
    InvalidRobotBrakeConfigurationDigest,
    InvalidOperatingConditionDigest,
    InvalidQualifiedSpeed,
    InvalidLatencyBound,
    InvalidPositiveAccelerationBound,
    InvalidNetDecelerationBound,
    InvalidBrakingJerkBound,
    InvalidAdditionalDistanceGuard,
    InvalidRequestSpeed,
    SpeedOutsideQualifiedDomain,
    NonFiniteIntermediate,
    DistanceBoundOverflow,
    TimeBoundOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct StoppingReachabilityResult {
    pub qualified_profile_sha256: [u8; 32],
    pub qualification_receipt_sha256: [u8; 32],
    pub speed_at_brake_onset_mm_s: f64,
    pub latency_travel_distance_mm: f64,
    pub braking_distance_mm: f64,
    pub physical_stopping_distance_mm: f64,
    pub guarded_stopping_distance_mm: f64,
    /// Outward-rounded integer millimetres plus a 1 mm numeric guard.
    pub conservative_distance_bound_mm: u64,
    pub braking_time_ms: f64,
    /// Latency plus braking time, rounded outward and numerically guarded.
    pub conservative_stop_time_bound_ms: u64,
}

/// Calculate the straight-line stopping extension for a frozen dynamics profile.
///
/// During the maximum response latency, the robot may accelerate at the
/// qualified maximum positive acceleration. After latency, guaranteed
/// deceleration magnitude grows no slower than the qualified minimum jerk
/// until the qualified minimum net deceleration is reached, then holds at
/// least that value until stopped.
///
/// Both the triangular (stop before plateau) and trapezoidal (jerk ramp plus
/// constant-deceleration plateau) braking profiles are handled analytically.
/// Inputs are in mm, seconds, and mm-based SI derivatives. The returned integer
/// bounds round outward and add explicit numeric guards; this is not a formal
/// interval-arithmetic proof.
pub fn calculate_stopping_reachability(
    qualification: &StoppingDynamicsQualification,
    request: StoppingReachabilityRequest,
) -> Result<StoppingReachabilityResult, StoppingReachabilityError> {
    validate_qualification(qualification)?;
    if !request.speed_upper_mm_s.is_finite() || request.speed_upper_mm_s < 0.0 {
        return Err(StoppingReachabilityError::InvalidRequestSpeed);
    }
    if request.speed_upper_mm_s > qualification.maximum_qualified_speed_mm_s {
        return Err(StoppingReachabilityError::SpeedOutsideQualifiedDomain);
    }

    let latency_seconds = qualification.maximum_detection_to_brake_latency_ms as f64 / 1_000.0;
    let latency_distance_mm = request.speed_upper_mm_s * latency_seconds
        + 0.5
            * qualification.maximum_positive_acceleration_mm_s2
            * latency_seconds
            * latency_seconds;
    let speed_at_brake_onset_mm_s = request.speed_upper_mm_s
        + qualification.maximum_positive_acceleration_mm_s2 * latency_seconds;

    let min_deceleration = qualification.minimum_net_deceleration_mm_s2;
    let min_jerk = qualification.minimum_braking_jerk_mm_s3;
    let full_ramp_seconds = min_deceleration / min_jerk;
    let velocity_removed_on_full_ramp =
        0.5 * min_jerk * full_ramp_seconds * full_ramp_seconds;

    let (braking_distance_mm, braking_time_seconds) =
        if speed_at_brake_onset_mm_s <= velocity_removed_on_full_ramp {
            if speed_at_brake_onset_mm_s == 0.0 {
                (0.0, 0.0)
            } else {
                let stop_seconds = (2.0 * speed_at_brake_onset_mm_s / min_jerk).sqrt();
                let distance = speed_at_brake_onset_mm_s * stop_seconds
                    - min_jerk * stop_seconds.powi(3) / 6.0;
                (distance, stop_seconds)
            }
        } else {
            let ramp_distance = speed_at_brake_onset_mm_s * full_ramp_seconds
                - min_jerk * full_ramp_seconds.powi(3) / 6.0;
            let speed_after_ramp =
                speed_at_brake_onset_mm_s - velocity_removed_on_full_ramp;
            let plateau_distance =
                speed_after_ramp * speed_after_ramp / (2.0 * min_deceleration);
            let plateau_seconds = speed_after_ramp / min_deceleration;
            (
                ramp_distance + plateau_distance,
                full_ramp_seconds + plateau_seconds,
            )
        };

    let physical_stopping_distance_mm = latency_distance_mm + braking_distance_mm;
    let guarded_stopping_distance_mm =
        physical_stopping_distance_mm + qualification.additional_distance_guard_mm;
    let total_stop_time_ms = (latency_seconds + braking_time_seconds) * 1_000.0;

    if [
        latency_distance_mm,
        speed_at_brake_onset_mm_s,
        braking_distance_mm,
        braking_time_seconds,
        physical_stopping_distance_mm,
        guarded_stopping_distance_mm,
        total_stop_time_ms,
    ]
    .into_iter()
    .any(|value| !value.is_finite() || value < 0.0)
    {
        return Err(StoppingReachabilityError::NonFiniteIntermediate);
    }

    Ok(StoppingReachabilityResult {
        qualified_profile_sha256: decode_sha256(&qualification.profile_sha256)
            .ok_or(StoppingReachabilityError::InvalidProfileDigest)?,
        qualification_receipt_sha256: decode_sha256(
            &qualification.qualification_receipt_sha256,
        )
        .ok_or(StoppingReachabilityError::InvalidQualificationReceiptDigest)?,
        speed_at_brake_onset_mm_s,
        latency_travel_distance_mm: latency_distance_mm,
        braking_distance_mm,
        physical_stopping_distance_mm,
        guarded_stopping_distance_mm,
        conservative_distance_bound_mm: outward_integer_bound(
            guarded_stopping_distance_mm,
            NUMERIC_DISTANCE_GUARD_MM,
        )
        .ok_or(StoppingReachabilityError::DistanceBoundOverflow)?,
        braking_time_ms: braking_time_seconds * 1_000.0,
        conservative_stop_time_bound_ms: outward_integer_bound(
            total_stop_time_ms,
            NUMERIC_TIME_GUARD_MS,
        )
        .ok_or(StoppingReachabilityError::TimeBoundOverflow)?,
    })
}

fn validate_qualification(
    qualification: &StoppingDynamicsQualification,
) -> Result<(), StoppingReachabilityError> {
    if decode_sha256(&qualification.profile_sha256).is_none() {
        return Err(StoppingReachabilityError::InvalidProfileDigest);
    }
    if decode_sha256(&qualification.qualification_receipt_sha256).is_none() {
        return Err(StoppingReachabilityError::InvalidQualificationReceiptDigest);
    }
    if decode_sha256(&qualification.robot_brake_configuration_sha256).is_none() {
        return Err(StoppingReachabilityError::InvalidRobotBrakeConfigurationDigest);
    }
    if decode_sha256(&qualification.operating_condition_sha256).is_none() {
        return Err(StoppingReachabilityError::InvalidOperatingConditionDigest);
    }
    if !qualification.maximum_qualified_speed_mm_s.is_finite()
        || qualification.maximum_qualified_speed_mm_s <= 0.0
    {
        return Err(StoppingReachabilityError::InvalidQualifiedSpeed);
    }
    if qualification.maximum_detection_to_brake_latency_ms == 0 {
        return Err(StoppingReachabilityError::InvalidLatencyBound);
    }
    if !qualification.maximum_positive_acceleration_mm_s2.is_finite()
        || qualification.maximum_positive_acceleration_mm_s2 < 0.0
    {
        return Err(StoppingReachabilityError::InvalidPositiveAccelerationBound);
    }
    if !qualification.minimum_net_deceleration_mm_s2.is_finite()
        || qualification.minimum_net_deceleration_mm_s2 <= 0.0
    {
        return Err(StoppingReachabilityError::InvalidNetDecelerationBound);
    }
    if !qualification.minimum_braking_jerk_mm_s3.is_finite()
        || qualification.minimum_braking_jerk_mm_s3 <= 0.0
    {
        return Err(StoppingReachabilityError::InvalidBrakingJerkBound);
    }
    if !qualification.additional_distance_guard_mm.is_finite()
        || qualification.additional_distance_guard_mm < 0.0
    {
        return Err(StoppingReachabilityError::InvalidAdditionalDistanceGuard);
    }
    Ok(())
}

fn outward_integer_bound(value: f64, numeric_guard: f64) -> Option<u64> {
    if !value.is_finite() || value < 0.0 {
        return None;
    }
    let guarded = value + numeric_guard;
    if !guarded.is_finite() || guarded > (u64::MAX - 1) as f64 {
        return None;
    }
    Some(guarded.ceil() as u64)
}

fn decode_sha256(value: &str) -> Option<[u8; 32]> {
    if value.len() != 64 {
        return None;
    }
    let mut output = [0_u8; 32];
    for (index, chunk) in value.as_bytes().chunks_exact(2).enumerate() {
        let high = decode_hex(chunk[0])?;
        let low = decode_hex(chunk[1])?;
        output[index] = (high << 4) | low;
    }
    Some(output)
}

fn decode_hex(value: u8) -> Option<u8> {
    match value {
        b'0'..=b'9' => Some(value - b'0'),
        b'a'..=b'f' => Some(value - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(value: char) -> String {
        std::iter::repeat_n(value, 64).collect()
    }

    fn qualification() -> StoppingDynamicsQualification {
        StoppingDynamicsQualification {
            profile_sha256: digest('1'),
            qualification_receipt_sha256: digest('2'),
            robot_brake_configuration_sha256: digest('3'),
            operating_condition_sha256: digest('4'),
            maximum_qualified_speed_mm_s: 2_000.0,
            maximum_detection_to_brake_latency_ms: 100,
            maximum_positive_acceleration_mm_s2: 500.0,
            minimum_net_deceleration_mm_s2: 1_000.0,
            minimum_braking_jerk_mm_s3: 1_000.0,
            additional_distance_guard_mm: 5.0,
        }
    }

    #[test]
    fn latency_acceleration_and_jerk_ramp_are_all_included() {
        let result = calculate_stopping_reachability(
            &qualification(),
            StoppingReachabilityRequest {
                speed_upper_mm_s: 1_000.0,
            },
        )
        .unwrap();
        assert!((result.latency_travel_distance_mm - 102.5).abs() < 1.0e-9);
        assert!((result.speed_at_brake_onset_mm_s - 1_050.0).abs() < 1.0e-9);
        assert!((result.braking_distance_mm - 958.3333333333334).abs() < 1.0e-8);
        assert!((result.physical_stopping_distance_mm - 1_060.8333333333335).abs() < 1.0e-8);
        assert!((result.guarded_stopping_distance_mm - 1_065.8333333333335).abs() < 1.0e-8);
        assert_eq!(result.conservative_distance_bound_mm, 1_067);
        assert!(result.conservative_stop_time_bound_ms > 1_100);
    }

    #[test]
    fn triangular_jerk_profile_stops_before_deceleration_plateau() {
        let mut q = qualification();
        q.maximum_detection_to_brake_latency_ms = 1;
        q.maximum_positive_acceleration_mm_s2 = 0.0;
        q.minimum_net_deceleration_mm_s2 = 100.0;
        q.minimum_braking_jerk_mm_s3 = 1_000.0;
        let result = calculate_stopping_reachability(
            &q,
            StoppingReachabilityRequest { speed_upper_mm_s: 1.0 },
        )
        .unwrap();
        assert!((result.speed_at_brake_onset_mm_s - 1.0).abs() < 1.0e-12);
        assert!(result.braking_time_ms < 100.0);
        assert!(result.braking_distance_mm < 0.1);
    }

    #[test]
    fn zero_speed_with_positive_latency_acceleration_still_has_reachability() {
        let mut q = qualification();
        q.maximum_detection_to_brake_latency_ms = 200;
        q.maximum_positive_acceleration_mm_s2 = 500.0;
        let result = calculate_stopping_reachability(
            &q,
            StoppingReachabilityRequest { speed_upper_mm_s: 0.0 },
        )
        .unwrap();
        assert!((result.latency_travel_distance_mm - 10.0).abs() < 1.0e-12);
        assert!((result.speed_at_brake_onset_mm_s - 100.0).abs() < 1.0e-12);
    }

    #[test]
    fn speed_outside_the_qualified_configuration_domain_is_rejected() {
        assert_eq!(
            calculate_stopping_reachability(
                &qualification(),
                StoppingReachabilityRequest {
                    speed_upper_mm_s: 2_000.1,
                }
            ),
            Err(StoppingReachabilityError::SpeedOutsideQualifiedDomain)
        );
    }

    #[test]
    fn malformed_digests_and_nonphysical_bounds_fail_closed() {
        let mut q = qualification();
        q.qualification_receipt_sha256 = "not-a-digest".into();
        assert_eq!(
            calculate_stopping_reachability(
                &q,
                StoppingReachabilityRequest { speed_upper_mm_s: 100.0 }
            ),
            Err(StoppingReachabilityError::InvalidQualificationReceiptDigest)
        );

        let mut q = qualification();
        q.minimum_net_deceleration_mm_s2 = 0.0;
        assert_eq!(
            calculate_stopping_reachability(
                &q,
                StoppingReachabilityRequest { speed_upper_mm_s: 100.0 }
            ),
            Err(StoppingReachabilityError::InvalidNetDecelerationBound)
        );

        let mut q = qualification();
        q.minimum_braking_jerk_mm_s3 = f64::NAN;
        assert_eq!(
            calculate_stopping_reachability(
                &q,
                StoppingReachabilityRequest { speed_upper_mm_s: 100.0 }
            ),
            Err(StoppingReachabilityError::InvalidBrakingJerkBound)
        );
    }

    #[test]
    fn integer_bounds_round_outward_with_explicit_numeric_guards() {
        let mut q = qualification();
        q.maximum_detection_to_brake_latency_ms = 1;
        q.maximum_positive_acceleration_mm_s2 = 0.0;
        q.minimum_net_deceleration_mm_s2 = 1_000_000.0;
        q.minimum_braking_jerk_mm_s3 = 1.0e12;
        q.additional_distance_guard_mm = 0.0;
        let result = calculate_stopping_reachability(
            &q,
            StoppingReachabilityRequest { speed_upper_mm_s: 100.0 },
        )
        .unwrap();
        assert!(
            result.conservative_distance_bound_mm as f64
                > result.physical_stopping_distance_mm
        );
        assert!(
            result.conservative_stop_time_bound_ms as f64
                > 1.0 + result.braking_time_ms
        );
    }
}
