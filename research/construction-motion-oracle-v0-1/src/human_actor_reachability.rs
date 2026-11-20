//! Privacy-preserving conservative reachability tubes for human site occupants.
//!
//! This module converts a caller-validated human-track observation into a
//! time-indexed outer occupancy envelope. It uses only an opaque actor token,
//! position/velocity bounds and a declared body/tool extent; it performs no
//! identity, biometric, emotion, intent or behaviour inference.
//!
//! Missing or incomplete tracking coverage is an error, never an empty-space
//! result. The module does not perform sensing, authenticate receipts, estimate
//! human motion bounds or authorize robot motion.

const NUMERIC_RADIUS_GUARD_MM: f64 = 1.0;

#[derive(Clone, Debug, PartialEq)]
pub struct HumanActorReachabilityProfile {
    pub profile_sha256: String,
    pub qualification_receipt_sha256: String,
    pub tracking_configuration_sha256: String,
    pub site_frame_sha256: String,
    pub scenario_context_sha256: String,
    /// Qualified upper bound for the supported task/site class.
    pub maximum_human_speed_mm_s: f64,
    /// Qualified upper bound on acceleration magnitude.
    pub maximum_human_acceleration_mm_s2: f64,
    pub maximum_tracking_latency_ms: u64,
    pub maximum_clock_mapping_uncertainty_ms: u64,
    pub maximum_observation_age_ms: u64,
    pub maximum_prediction_horizon_ms: u64,
    pub slice_duration_ms: u64,
    pub maximum_slice_count: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HumanActorObservation {
    /// Ephemeral pseudonymous token. No personal identity or biometric field.
    pub actor_token_sha256: String,
    pub observation_receipt_sha256: String,
    pub frame_sha256: String,
    pub scenario_context_sha256: String,
    pub observed_at_site_tick: u64,
    pub position_x_mm: f64,
    pub position_y_mm: f64,
    pub position_z_mm: f64,
    /// Upper bound on position error as a Euclidean radius (mm).
    pub position_error_upper_mm: f64,
    /// Upper bound on speed magnitude at observation time (mm/s).
    pub speed_upper_mm_s: f64,
    /// Conservative body plus currently carried-tool extent radius (mm).
    pub body_extent_radius_upper_mm: f64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct HumanTrackingCoverageReceipt {
    pub receipt_sha256: String,
    pub tracking_configuration_sha256: String,
    pub frame_sha256: String,
    pub scenario_context_sha256: String,
    /// False means some accessible area is not adequately observed.
    pub coverage_complete: bool,
    pub unresolved_coverage_region_count: u64,
    pub unresolved_actor_count: u64,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct HumanReachabilityRequest {
    pub now_site_tick: u64,
    pub prediction_horizon_ms: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HumanReachabilityError {
    InvalidProfileDigest,
    InvalidQualificationReceiptDigest,
    InvalidTrackingConfigurationDigest,
    InvalidFrameDigest,
    InvalidScenarioContextDigest,
    InvalidCoverageReceiptDigest,
    InvalidActorTokenDigest,
    InvalidObservationReceiptDigest,
    ProfileHasInvalidSpeedBound,
    ProfileHasInvalidAccelerationBound,
    ProfileHasInvalidTimingBound,
    ProfileHasInvalidSliceBound,
    ObservationFrameMismatch,
    ObservationScenarioMismatch,
    CoverageContextMismatch,
    IncompleteTrackingCoverage,
    ObservationFromFuture,
    ObservationIsStale,
    ActorSpeedOutsideQualifiedDomain,
    InvalidObservationPosition,
    InvalidPositionUncertainty,
    InvalidBodyExtent,
    InvalidRequestedPredictionHorizon,
    PredictionSliceLimitExceeded,
    SiteTickOverflow,
    NonFiniteIntermediate,
    RadiusBoundOverflow,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct HumanReachabilitySlice {
    pub start_site_tick: u64,
    pub end_site_tick: u64,
    pub center_x_mm: f64,
    pub center_y_mm: f64,
    pub center_z_mm: f64,
    /// Outward-rounded sphere radius; covers all modeled positions in the slice.
    pub radius_upper_mm: u64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct HumanReachabilityTube {
    pub actor_token_sha256: [u8; 32],
    pub profile_sha256: [u8; 32],
    pub qualification_receipt_sha256: [u8; 32],
    pub observation_receipt_sha256: [u8; 32],
    pub coverage_receipt_sha256: [u8; 32],
    pub tracking_configuration_sha256: [u8; 32],
    pub frame_sha256: [u8; 32],
    pub scenario_context_sha256: [u8; 32],
    pub observation_age_ms: u64,
    pub effective_initial_age_ms: u64,
    pub prediction_start_site_tick: u64,
    pub valid_until_site_tick: u64,
    pub maximum_radius_upper_mm: u64,
    pub slices: Vec<HumanReachabilitySlice>,
}

/// Build a conservative, time-sliced spherical occupancy tube for one tracked
/// human actor. Every slice encloses a ball around the last observed position;
/// the radius grows by the maximum reachable travel since observation, plus
/// tracking latency, clock uncertainty, position error and body/tool extent.
///
/// The speed and acceleration values are external qualification bounds. This
/// function does not predict intent or assume that a person will continue in a
/// particular direction.
pub fn build_human_reachability_tube(
    profile: &HumanActorReachabilityProfile,
    observation: &HumanActorObservation,
    coverage: &HumanTrackingCoverageReceipt,
    request: HumanReachabilityRequest,
) -> Result<HumanReachabilityTube, HumanReachabilityError> {
    validate_profile(profile)?;
    validate_observation(observation)?;
    validate_coverage(profile, coverage)?;

    if observation.frame_sha256 != profile.site_frame_sha256 {
        return Err(HumanReachabilityError::ObservationFrameMismatch);
    }
    if observation.scenario_context_sha256 != profile.scenario_context_sha256 {
        return Err(HumanReachabilityError::ObservationScenarioMismatch);
    }
    if observation.speed_upper_mm_s > profile.maximum_human_speed_mm_s {
        return Err(HumanReachabilityError::ActorSpeedOutsideQualifiedDomain);
    }
    if observation.observed_at_site_tick
        > request
            .now_site_tick
            .saturating_add(profile.maximum_clock_mapping_uncertainty_ms)
    {
        return Err(HumanReachabilityError::ObservationFromFuture);
    }

    let observation_age_ms = request
        .now_site_tick
        .saturating_sub(observation.observed_at_site_tick);
    if observation_age_ms > profile.maximum_observation_age_ms {
        return Err(HumanReachabilityError::ObservationIsStale);
    }
    if request.prediction_horizon_ms == 0
        || request.prediction_horizon_ms > profile.maximum_prediction_horizon_ms
    {
        return Err(HumanReachabilityError::InvalidRequestedPredictionHorizon);
    }

    let slice_count_u64 = request
        .prediction_horizon_ms
        .checked_add(profile.slice_duration_ms - 1)
        .ok_or(HumanReachabilityError::PredictionSliceLimitExceeded)?
        / profile.slice_duration_ms;
    let slice_count = u32::try_from(slice_count_u64)
        .map_err(|_| HumanReachabilityError::PredictionSliceLimitExceeded)?;
    if slice_count == 0 || slice_count > profile.maximum_slice_count {
        return Err(HumanReachabilityError::PredictionSliceLimitExceeded);
    }

    let effective_initial_age_ms = observation_age_ms
        .checked_add(profile.maximum_tracking_latency_ms)
        .and_then(|value| value.checked_add(profile.maximum_clock_mapping_uncertainty_ms))
        .ok_or(HumanReachabilityError::SiteTickOverflow)?;
    let mut slices = Vec::with_capacity(slice_count as usize);
    let mut maximum_radius_upper_mm = 0_u64;

    for index in 0..slice_count {
        let start_offset_ms = u64::from(index)
            .checked_mul(profile.slice_duration_ms)
            .ok_or(HumanReachabilityError::SiteTickOverflow)?;
        let end_offset_ms = start_offset_ms
            .checked_add(profile.slice_duration_ms)
            .ok_or(HumanReachabilityError::SiteTickOverflow)?
            .min(request.prediction_horizon_ms);
        let start_site_tick = request
            .now_site_tick
            .checked_add(start_offset_ms)
            .ok_or(HumanReachabilityError::SiteTickOverflow)?;
        let end_site_tick = request
            .now_site_tick
            .checked_add(end_offset_ms)
            .ok_or(HumanReachabilityError::SiteTickOverflow)?;

        // Use the slice end for the complete half-open interval. The reachable
        // radius is non-decreasing, so this encloses every earlier instant too.
        let total_elapsed_ms = effective_initial_age_ms
            .checked_add(end_offset_ms)
            .ok_or(HumanReachabilityError::SiteTickOverflow)?;
        let travel_seconds = total_elapsed_ms as f64 / 1_000.0;
        let travel_distance_mm = maximum_reachable_distance_mm(
            observation.speed_upper_mm_s,
            profile.maximum_human_acceleration_mm_s2,
            profile.maximum_human_speed_mm_s,
            travel_seconds,
        );
        let unrounded_radius_mm = observation.body_extent_radius_upper_mm
            + observation.position_error_upper_mm
            + travel_distance_mm;
        if !unrounded_radius_mm.is_finite() || unrounded_radius_mm < 0.0 {
            return Err(HumanReachabilityError::NonFiniteIntermediate);
        }
        let radius_upper_mm = outward_radius_bound(unrounded_radius_mm)
            .ok_or(HumanReachabilityError::RadiusBoundOverflow)?;
        maximum_radius_upper_mm = maximum_radius_upper_mm.max(radius_upper_mm);
        slices.push(HumanReachabilitySlice {
            start_site_tick,
            end_site_tick,
            center_x_mm: observation.position_x_mm,
            center_y_mm: observation.position_y_mm,
            center_z_mm: observation.position_z_mm,
            radius_upper_mm,
        });
    }

    Ok(HumanReachabilityTube {
        actor_token_sha256: decode_sha256(&observation.actor_token_sha256)
            .ok_or(HumanReachabilityError::InvalidActorTokenDigest)?,
        profile_sha256: decode_sha256(&profile.profile_sha256)
            .ok_or(HumanReachabilityError::InvalidProfileDigest)?,
        qualification_receipt_sha256: decode_sha256(&profile.qualification_receipt_sha256)
            .ok_or(HumanReachabilityError::InvalidQualificationReceiptDigest)?,
        observation_receipt_sha256: decode_sha256(&observation.observation_receipt_sha256)
            .ok_or(HumanReachabilityError::InvalidObservationReceiptDigest)?,
        coverage_receipt_sha256: decode_sha256(&coverage.receipt_sha256)
            .ok_or(HumanReachabilityError::InvalidCoverageReceiptDigest)?,
        tracking_configuration_sha256: decode_sha256(&profile.tracking_configuration_sha256)
            .ok_or(HumanReachabilityError::InvalidTrackingConfigurationDigest)?,
        frame_sha256: decode_sha256(&profile.site_frame_sha256)
            .ok_or(HumanReachabilityError::InvalidFrameDigest)?,
        scenario_context_sha256: decode_sha256(&profile.scenario_context_sha256)
            .ok_or(HumanReachabilityError::InvalidScenarioContextDigest)?,
        observation_age_ms,
        effective_initial_age_ms,
        prediction_start_site_tick: request.now_site_tick,
        valid_until_site_tick: request
            .now_site_tick
            .checked_add(request.prediction_horizon_ms)
            .ok_or(HumanReachabilityError::SiteTickOverflow)?,
        maximum_radius_upper_mm,
        slices,
    })
}

fn validate_profile(
    profile: &HumanActorReachabilityProfile,
) -> Result<(), HumanReachabilityError> {
    if decode_sha256(&profile.profile_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidProfileDigest);
    }
    if decode_sha256(&profile.qualification_receipt_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidQualificationReceiptDigest);
    }
    if decode_sha256(&profile.tracking_configuration_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidTrackingConfigurationDigest);
    }
    if decode_sha256(&profile.site_frame_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidFrameDigest);
    }
    if decode_sha256(&profile.scenario_context_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidScenarioContextDigest);
    }
    if !profile.maximum_human_speed_mm_s.is_finite()
        || profile.maximum_human_speed_mm_s <= 0.0
    {
        return Err(HumanReachabilityError::ProfileHasInvalidSpeedBound);
    }
    if !profile.maximum_human_acceleration_mm_s2.is_finite()
        || profile.maximum_human_acceleration_mm_s2 < 0.0
    {
        return Err(HumanReachabilityError::ProfileHasInvalidAccelerationBound);
    }
    if profile.maximum_prediction_horizon_ms == 0
        || profile.maximum_observation_age_ms == 0
        || profile.slice_duration_ms == 0
        || profile.maximum_slice_count == 0
    {
        return Err(HumanReachabilityError::ProfileHasInvalidTimingBound);
    }
    if profile.maximum_tracking_latency_ms == 0
        || profile.maximum_clock_mapping_uncertainty_ms == 0
    {
        return Err(HumanReachabilityError::ProfileHasInvalidTimingBound);
    }
    if profile.slice_duration_ms > profile.maximum_prediction_horizon_ms
        && profile.maximum_slice_count == 0
    {
        return Err(HumanReachabilityError::ProfileHasInvalidSliceBound);
    }
    Ok(())
}

fn validate_observation(
    observation: &HumanActorObservation,
) -> Result<(), HumanReachabilityError> {
    if decode_sha256(&observation.actor_token_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidActorTokenDigest);
    }
    if decode_sha256(&observation.observation_receipt_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidObservationReceiptDigest);
    }
    if ![
        observation.position_x_mm,
        observation.position_y_mm,
        observation.position_z_mm,
    ]
    .into_iter()
    .all(f64::is_finite)
    {
        return Err(HumanReachabilityError::InvalidObservationPosition);
    }
    if !observation.position_error_upper_mm.is_finite()
        || observation.position_error_upper_mm < 0.0
    {
        return Err(HumanReachabilityError::InvalidPositionUncertainty);
    }
    if !observation.speed_upper_mm_s.is_finite() || observation.speed_upper_mm_s < 0.0 {
        return Err(HumanReachabilityError::ActorSpeedOutsideQualifiedDomain);
    }
    if !observation.body_extent_radius_upper_mm.is_finite()
        || observation.body_extent_radius_upper_mm <= 0.0
    {
        return Err(HumanReachabilityError::InvalidBodyExtent);
    }
    Ok(())
}

fn validate_coverage(
    profile: &HumanActorReachabilityProfile,
    coverage: &HumanTrackingCoverageReceipt,
) -> Result<(), HumanReachabilityError> {
    if decode_sha256(&coverage.receipt_sha256).is_none() {
        return Err(HumanReachabilityError::InvalidCoverageReceiptDigest);
    }
    if coverage.tracking_configuration_sha256 != profile.tracking_configuration_sha256
        || coverage.frame_sha256 != profile.site_frame_sha256
        || coverage.scenario_context_sha256 != profile.scenario_context_sha256
    {
        return Err(HumanReachabilityError::CoverageContextMismatch);
    }
    if !coverage.coverage_complete
        || coverage.unresolved_coverage_region_count != 0
        || coverage.unresolved_actor_count != 0
    {
        return Err(HumanReachabilityError::IncompleteTrackingCoverage);
    }
    Ok(())
}

fn maximum_reachable_distance_mm(
    initial_speed_mm_s: f64,
    maximum_acceleration_mm_s2: f64,
    maximum_speed_mm_s: f64,
    elapsed_seconds: f64,
) -> f64 {
    if elapsed_seconds <= 0.0 {
        return 0.0;
    }
    if maximum_acceleration_mm_s2 == 0.0
        || initial_speed_mm_s >= maximum_speed_mm_s
    {
        return maximum_speed_mm_s.min(initial_speed_mm_s) * elapsed_seconds;
    }
    let time_to_speed_limit =
        (maximum_speed_mm_s - initial_speed_mm_s) / maximum_acceleration_mm_s2;
    if elapsed_seconds <= time_to_speed_limit {
        initial_speed_mm_s * elapsed_seconds
            + 0.5 * maximum_acceleration_mm_s2 * elapsed_seconds * elapsed_seconds
    } else {
        let acceleration_distance = initial_speed_mm_s * time_to_speed_limit
            + 0.5
                * maximum_acceleration_mm_s2
                * time_to_speed_limit
                * time_to_speed_limit;
        acceleration_distance
            + maximum_speed_mm_s * (elapsed_seconds - time_to_speed_limit)
    }
}

fn outward_radius_bound(radius_mm: f64) -> Option<u64> {
    let guarded = radius_mm + NUMERIC_RADIUS_GUARD_MM;
    if !guarded.is_finite() || guarded >= u64::MAX as f64 {
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

    fn profile() -> HumanActorReachabilityProfile {
        HumanActorReachabilityProfile {
            profile_sha256: digest('1'),
            qualification_receipt_sha256: digest('2'),
            tracking_configuration_sha256: digest('3'),
            site_frame_sha256: digest('4'),
            scenario_context_sha256: digest('5'),
            maximum_human_speed_mm_s: 2_000.0,
            maximum_human_acceleration_mm_s2: 1_500.0,
            maximum_tracking_latency_ms: 50,
            maximum_clock_mapping_uncertainty_ms: 10,
            maximum_observation_age_ms: 500,
            maximum_prediction_horizon_ms: 1_000,
            slice_duration_ms: 100,
            maximum_slice_count: 10,
        }
    }

    fn observation() -> HumanActorObservation {
        HumanActorObservation {
            actor_token_sha256: digest('6'),
            observation_receipt_sha256: digest('7'),
            frame_sha256: digest('4'),
            scenario_context_sha256: digest('5'),
            observed_at_site_tick: 9_900,
            position_x_mm: 10_000.0,
            position_y_mm: 20_000.0,
            position_z_mm: 0.0,
            position_error_upper_mm: 20.0,
            speed_upper_mm_s: 1_000.0,
            body_extent_radius_upper_mm: 350.0,
        }
    }

    fn coverage() -> HumanTrackingCoverageReceipt {
        HumanTrackingCoverageReceipt {
            receipt_sha256: digest('8'),
            tracking_configuration_sha256: digest('3'),
            frame_sha256: digest('4'),
            scenario_context_sha256: digest('5'),
            coverage_complete: true,
            unresolved_coverage_region_count: 0,
            unresolved_actor_count: 0,
        }
    }

    #[test]
    fn reachability_tube_includes_age_latency_clock_uncertainty_and_body_extent() {
        let tube = build_human_reachability_tube(
            &profile(),
            &observation(),
            &coverage(),
            HumanReachabilityRequest {
                now_site_tick: 10_000,
                prediction_horizon_ms: 200,
            },
        )
        .unwrap();
        assert_eq!(tube.observation_age_ms, 100);
        assert_eq!(tube.effective_initial_age_ms, 160);
        assert_eq!(tube.slices.len(), 2);
        assert_eq!(tube.slices[0].start_site_tick, 10_000);
        assert_eq!(tube.slices[0].end_site_tick, 10_100);
        assert_eq!(tube.slices[1].end_site_tick, 10_200);
        // 350 mm body + 20 mm position error + 1000 mm/s × 260 ms
        // + 1/2 × 1500 mm/s² × (0.26 s)^2 = 420.7 mm, plus 1 mm guard.
        assert_eq!(tube.slices[0].radius_upper_mm, 422);
        assert!(tube.slices[1].radius_upper_mm > tube.slices[0].radius_upper_mm);
    }

    #[test]
    fn acceleration_is_integrated_until_speed_cap_then_velocity_is_bounded() {
        let mut p = profile();
        p.maximum_human_speed_mm_s = 1_500.0;
        p.maximum_human_acceleration_mm_s2 = 500.0;
        let distance = maximum_reachable_distance_mm(1_000.0, 500.0, 1_500.0, 2.0);
        // Accelerate for 1 s (1,250 mm), then move at 1,500 mm/s for 1 s.
        assert!((distance - 2_750.0).abs() < 1.0e-9);
    }

    #[test]
    fn incomplete_coverage_or_stale_observation_is_never_empty_occupancy() {
        let mut c = coverage();
        c.coverage_complete = false;
        assert_eq!(
            build_human_reachability_tube(
                &profile(),
                &observation(),
                &c,
                HumanReachabilityRequest {
                    now_site_tick: 10_000,
                    prediction_horizon_ms: 200,
                },
            ),
            Err(HumanReachabilityError::IncompleteTrackingCoverage)
        );

        let mut o = observation();
        o.observed_at_site_tick = 9_000;
        assert_eq!(
            build_human_reachability_tube(
                &profile(),
                &o,
                &coverage(),
                HumanReachabilityRequest {
                    now_site_tick: 10_000,
                    prediction_horizon_ms: 200,
                },
            ),
            Err(HumanReachabilityError::ObservationIsStale)
        );
    }

    #[test]
    fn mismatched_frame_scenario_and_unqualified_speed_fail_closed() {
        let mut o = observation();
        o.frame_sha256 = digest('9');
        assert_eq!(
            build_human_reachability_tube(
                &profile(),
                &o,
                &coverage(),
                HumanReachabilityRequest {
                    now_site_tick: 10_000,
                    prediction_horizon_ms: 200,
                },
            ),
            Err(HumanReachabilityError::ObservationFrameMismatch)
        );

        let mut o = observation();
        o.speed_upper_mm_s = 2_000.1;
        assert_eq!(
            build_human_reachability_tube(
                &profile(),
                &o,
                &coverage(),
                HumanReachabilityRequest {
                    now_site_tick: 10_000,
                    prediction_horizon_ms: 200,
                },
            ),
            Err(HumanReachabilityError::ActorSpeedOutsideQualifiedDomain)
        );
    }

    #[test]
    fn future_observation_beyond_clock_uncertainty_and_excess_horizon_are_rejected() {
        let mut o = observation();
        o.observed_at_site_tick = 10_020;
        assert_eq!(
            build_human_reachability_tube(
                &profile(),
                &o,
                &coverage(),
                HumanReachabilityRequest {
                    now_site_tick: 10_000,
                    prediction_horizon_ms: 200,
                },
            ),
            Err(HumanReachabilityError::ObservationFromFuture)
        );

        assert_eq!(
            build_human_reachability_tube(
                &profile(),
                &observation(),
                &coverage(),
                HumanReachabilityRequest {
                    now_site_tick: 10_000,
                    prediction_horizon_ms: 1_001,
                },
            ),
            Err(HumanReachabilityError::InvalidRequestedPredictionHorizon)
        );
    }

    #[test]
    fn deterministic_generated_speed_cap_cases_never_underbound_sampled_travel() {
        let p = profile();
        let mut seed = 0x51a7_u64;
        for _case in 0..64 {
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let speed = 100.0 + (seed % 1_800) as f64;
            seed = seed.wrapping_mul(6_364_136_223_846_793_005).wrapping_add(1);
            let acceleration = (seed % 1_500) as f64;
            let mut case_profile = p.clone();
            case_profile.maximum_human_acceleration_mm_s2 = acceleration;
            let distance = maximum_reachable_distance_mm(
                speed,
                acceleration,
                case_profile.maximum_human_speed_mm_s,
                0.8,
            );
            for step in 0..=80 {
                let t = step as f64 * 0.01;
                let sampled = maximum_reachable_distance_mm(
                    speed,
                    acceleration,
                    case_profile.maximum_human_speed_mm_s,
                    t,
                );
                assert!(sampled <= distance + 1.0e-8);
            }
        }
    }
}
