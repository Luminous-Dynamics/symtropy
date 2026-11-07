// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Bounded route-progress -> kinematic-pose adapter for synthetic road actors.
//!
//! This crate does not own actor policy, physics integration, or human behaviour.
//! It consumes a validated policy-state speed setpoint and a scenario-authored
//! straight route, then emits a replay-verifiable pose target. Curved, segmented,
//! discontinuous, reverse and missing route geometries are deliberately outside
//! v0.1 rather than guessed from body velocity or runtime handles.

#![forbid(unsafe_code)]

use std::fmt;
use symtropy_road_actors::{
    ActorMode, ActorPolicyState, MotionBounds, ScenarioActorId, ScenarioTick,
};

pub const ROUTE_SCHEMA_ID: &str = "symtropy.road-actor-route.v0.1";
pub const STEP_SCHEMA_ID: &str = "symtropy.road-actor-path-step.v0.1";
const ROUTE_DOMAIN: &[u8] = b"symtropy.road-actor-path.route.v0.1\0";
const STEP_DOMAIN: &[u8] = b"symtropy.road-actor-path.step.v0.1\0";

#[derive(Clone, Debug, PartialEq)]
pub enum AdapterError {
    InvalidIdentifier,
    NonFiniteGeometry,
    DegenerateRoute,
    InvalidMotionProfile,
    InvalidTickClock,
    ZeroElapsedTicks,
    MissingRouteProfile,
    MissingPolicyDecisionTick,
    InvalidPolicyProfile,
    ReverseMotionUnsupported,
    SpeedOutOfBounds,
    ProgressOutOfBounds,
    NonFiniteDerivedMotion,
    PathEndWouldBeReached {
        remaining_m: f64,
        requested_distance_m: f64,
        applied_distance_m: f64,
    },
}

impl fmt::Display for AdapterError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for AdapterError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteId(String);

impl RouteId {
    pub fn new(value: impl Into<String>) -> Result<Self, AdapterError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CoordinateFrameId(String);

impl CoordinateFrameId {
    pub fn new(value: impl Into<String>) -> Result<Self, AdapterError> {
        let value = value.into();
        validate_identifier(&value)?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

fn validate_identifier(value: &str) -> Result<(), AdapterError> {
    if value.is_empty()
        || value.len() > 96
        || !value.is_ascii()
        || !value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_.:/@-".contains(&byte))
    {
        return Err(AdapterError::InvalidIdentifier);
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Point2 {
    pub x_m: f64,
    pub y_m: f64,
}

impl Point2 {
    pub fn new(x_m: f64, y_m: f64) -> Result<Self, AdapterError> {
        if !x_m.is_finite() || !y_m.is_finite() {
            return Err(AdapterError::NonFiniteGeometry);
        }
        Ok(Self {
            x_m: canonical_zero(x_m),
            y_m: canonical_zero(y_m),
        })
    }
}

/// A single directed line segment in a named 2D metric frame.
///
/// Requiring exactly two endpoints makes heading unambiguous. V0.1 does not
/// silently infer a polyline, curve, corner transition or route discontinuity.
#[derive(Clone, Debug, PartialEq)]
pub struct StraightRoute {
    route_id: RouteId,
    coordinate_frame: CoordinateFrameId,
    start: Point2,
    end: Point2,
    length_m: f64,
    direction: Point2,
    heading_radians: f64,
    digest: [u8; 32],
}

impl StraightRoute {
    pub fn new(
        route_id: RouteId,
        coordinate_frame: CoordinateFrameId,
        start: Point2,
        end: Point2,
    ) -> Result<Self, AdapterError> {
        let dx = end.x_m - start.x_m;
        let dy = end.y_m - start.y_m;
        if !dx.is_finite() || !dy.is_finite() {
            return Err(AdapterError::NonFiniteGeometry);
        }
        let length_m = dx.hypot(dy);
        if !length_m.is_finite() {
            return Err(AdapterError::NonFiniteGeometry);
        }
        if length_m <= 0.0 {
            return Err(AdapterError::DegenerateRoute);
        }
        let direction = Point2 {
            x_m: canonical_zero(dx / length_m),
            y_m: canonical_zero(dy / length_m),
        };
        let heading_radians = dy.atan2(dx);
        if !heading_radians.is_finite() {
            return Err(AdapterError::NonFiniteGeometry);
        }

        let mut route = Self {
            route_id,
            coordinate_frame,
            start,
            end,
            length_m,
            direction,
            heading_radians: canonical_zero(heading_radians),
            digest: [0; 32],
        };
        route.digest = *blake3::hash(&route.canonical_bytes()).as_bytes();
        Ok(route)
    }

    pub fn route_id(&self) -> &RouteId {
        &self.route_id
    }

    pub fn coordinate_frame(&self) -> &CoordinateFrameId {
        &self.coordinate_frame
    }

    pub const fn length_m(&self) -> f64 {
        self.length_m
    }

    pub const fn digest(&self) -> &[u8; 32] {
        &self.digest
    }

    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(ROUTE_DOMAIN.len() + 256);
        output.extend_from_slice(ROUTE_DOMAIN);
        push_string(&mut output, ROUTE_SCHEMA_ID);
        push_string(&mut output, self.route_id.as_str());
        push_string(&mut output, self.coordinate_frame.as_str());
        push_point(&mut output, self.start);
        push_point(&mut output, self.end);
        push_f64(&mut output, self.length_m);
        push_point(&mut output, self.direction);
        push_f64(&mut output, self.heading_radians);
        output
    }

    fn point_at(&self, progress_m: f64) -> Point2 {
        Point2 {
            x_m: canonical_zero(self.start.x_m + self.direction.x_m * progress_m),
            y_m: canonical_zero(self.start.y_m + self.direction.y_m * progress_m),
        }
    }
}

/// Acceleration/deceleration envelope applied to the requested policy setpoint.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct MotionProfile {
    max_forward_speed_mps: f64,
    max_acceleration_mps2: f64,
    max_deceleration_mps2: f64,
}

impl MotionProfile {
    pub fn new(
        policy_bounds: MotionBounds,
        max_acceleration_mps2: f64,
        max_deceleration_mps2: f64,
    ) -> Result<Self, AdapterError> {
        let max_forward_speed_mps = policy_bounds.max_forward_speed_mps();
        if !max_forward_speed_mps.is_finite()
            || max_forward_speed_mps <= 0.0
            || !max_acceleration_mps2.is_finite()
            || max_acceleration_mps2 <= 0.0
            || !max_deceleration_mps2.is_finite()
            || max_deceleration_mps2 <= 0.0
        {
            return Err(AdapterError::InvalidMotionProfile);
        }
        Ok(Self {
            max_forward_speed_mps,
            max_acceleration_mps2: canonical_zero(max_acceleration_mps2),
            max_deceleration_mps2: canonical_zero(max_deceleration_mps2),
        })
    }
}

/// Integer scenario clock. Physics interval duration is elapsed_ticks / ticks_per_second.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TickClock {
    ticks_per_second: u64,
}

impl TickClock {
    pub fn new(ticks_per_second: u64) -> Result<Self, AdapterError> {
        if ticks_per_second == 0 {
            return Err(AdapterError::InvalidTickClock);
        }
        Ok(Self { ticks_per_second })
    }

    pub const fn ticks_per_second(self) -> u64 {
        self.ticks_per_second
    }
}

pub struct PathStepInput<'a> {
    pub actor_id: &'a ScenarioActorId,
    pub policy_profile_id: &'a str,
    pub policy_state: ActorPolicyState,
    pub route: Option<&'a StraightRoute>,
    pub motion_profile: MotionProfile,
    pub clock: TickClock,
    pub elapsed_ticks: u64,
    pub prior_progress_m: f64,
    /// Measured non-negative forward speed at the beginning of this interval.
    pub measured_forward_speed_mps: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct KinematicPoseTarget {
    pub position: Point2,
    pub heading_radians: f64,
    pub linear_velocity_mps: Point2,
    pub angular_velocity_radians_per_second: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RouteDisposition {
    WithinDeclaredRoute,
    TargetSetpointWouldReachRouteEnd,
}

/// Complete replay record for one policy-state -> kinematic-pose projection.
///
/// The BLAKE3 digest is an unkeyed content commitment, not authentication.
#[derive(Clone, Debug, PartialEq)]
pub struct PathStepReceipt {
    pub schema_id: &'static str,
    pub actor_id: String,
    pub policy_profile_id: String,
    pub policy_mode: ActorMode,
    pub policy_decision_tick: ScenarioTick,
    pub route_id: String,
    pub coordinate_frame: String,
    pub route_digest: [u8; 32],
    pub ticks_per_second: u64,
    pub elapsed_ticks: u64,
    pub elapsed_seconds: f64,
    pub max_forward_speed_mps: f64,
    pub max_acceleration_mps2: f64,
    pub max_deceleration_mps2: f64,
    pub prior_progress_m: f64,
    pub measured_forward_speed_mps: f64,
    pub requested_forward_speed_mps: f64,
    pub applied_forward_speed_mps: f64,
    pub requested_distance_m: f64,
    pub applied_distance_m: f64,
    pub next_progress_m: f64,
    pub remaining_route_m: f64,
    pub disposition: RouteDisposition,
    pub target: KinematicPoseTarget,
    pub receipt_digest: [u8; 32],
}

impl PathStepReceipt {
    pub fn canonical_bytes(&self) -> Vec<u8> {
        let mut output = Vec::with_capacity(512);
        output.extend_from_slice(STEP_DOMAIN);
        push_string(&mut output, self.schema_id);
        push_string(&mut output, &self.actor_id);
        push_string(&mut output, &self.policy_profile_id);
        output.push(mode_tag(self.policy_mode));
        output.extend_from_slice(&self.policy_decision_tick.0.to_le_bytes());
        push_string(&mut output, &self.route_id);
        push_string(&mut output, &self.coordinate_frame);
        output.extend_from_slice(&self.route_digest);
        output.extend_from_slice(&self.ticks_per_second.to_le_bytes());
        output.extend_from_slice(&self.elapsed_ticks.to_le_bytes());
        for value in [
            self.elapsed_seconds,
            self.max_forward_speed_mps,
            self.max_acceleration_mps2,
            self.max_deceleration_mps2,
            self.prior_progress_m,
            self.measured_forward_speed_mps,
            self.requested_forward_speed_mps,
            self.applied_forward_speed_mps,
            self.requested_distance_m,
            self.applied_distance_m,
            self.next_progress_m,
            self.remaining_route_m,
        ] {
            push_f64(&mut output, value);
        }
        output.push(match self.disposition {
            RouteDisposition::WithinDeclaredRoute => 0,
            RouteDisposition::TargetSetpointWouldReachRouteEnd => 1,
        });
        push_point(&mut output, self.target.position);
        push_f64(&mut output, self.target.heading_radians);
        push_point(&mut output, self.target.linear_velocity_mps);
        push_f64(&mut output, self.target.angular_velocity_radians_per_second);
        output
    }

    pub fn recompute_digest(&self) -> [u8; 32] {
        *blake3::hash(&self.canonical_bytes()).as_bytes()
    }
}

/// Convert one already-decided actor state to a straight-route pose target.
///
/// The adapter does not choose actions, infer a route, or advance physics.
/// Its trapezoidal distance step is a declared kinematic reference assumption.
pub fn plan_path_step(input: &PathStepInput<'_>) -> Result<PathStepReceipt, AdapterError> {
    let route = input.route.ok_or(AdapterError::MissingRouteProfile)?;
    validate_identifier(input.policy_profile_id)?;

    let policy_decision_tick = input
        .policy_state
        .last_decision_tick()
        .ok_or(AdapterError::MissingPolicyDecisionTick)?;
    if input.elapsed_ticks == 0 {
        return Err(AdapterError::ZeroElapsedTicks);
    }
    if !input.prior_progress_m.is_finite()
        || input.prior_progress_m < 0.0
        || input.prior_progress_m >= route.length_m
    {
        return Err(AdapterError::ProgressOutOfBounds);
    }
    if !input.measured_forward_speed_mps.is_finite() {
        return Err(AdapterError::NonFiniteDerivedMotion);
    }
    if input.measured_forward_speed_mps < 0.0 {
        return Err(AdapterError::ReverseMotionUnsupported);
    }
    let profile = input.motion_profile;
    if input.measured_forward_speed_mps > profile.max_forward_speed_mps {
        return Err(AdapterError::SpeedOutOfBounds);
    }
    let requested_speed = input.policy_state.target_forward_speed_mps();
    if !requested_speed.is_finite()
        || requested_speed < 0.0
        || requested_speed > profile.max_forward_speed_mps
    {
        return Err(AdapterError::SpeedOutOfBounds);
    }

    let elapsed_seconds = input.elapsed_ticks as f64 / input.clock.ticks_per_second as f64;
    let acceleration_delta = profile.max_acceleration_mps2 * elapsed_seconds;
    let deceleration_delta = profile.max_deceleration_mps2 * elapsed_seconds;
    if !elapsed_seconds.is_finite()
        || elapsed_seconds <= 0.0
        || !acceleration_delta.is_finite()
        || !deceleration_delta.is_finite()
    {
        return Err(AdapterError::NonFiniteDerivedMotion);
    }

    let speed_delta = requested_speed - input.measured_forward_speed_mps;
    let applied_speed = if speed_delta >= 0.0 {
        input.measured_forward_speed_mps + speed_delta.min(acceleration_delta)
    } else {
        input.measured_forward_speed_mps - (-speed_delta).min(deceleration_delta)
    };
    let requested_distance =
        0.5 * (input.measured_forward_speed_mps + requested_speed) * elapsed_seconds;
    let applied_distance =
        0.5 * (input.measured_forward_speed_mps + applied_speed) * elapsed_seconds;
    if !applied_speed.is_finite()
        || !requested_distance.is_finite()
        || !applied_distance.is_finite()
    {
        return Err(AdapterError::NonFiniteDerivedMotion);
    }

    let remaining_route_m = route.length_m - input.prior_progress_m;
    if applied_distance >= remaining_route_m {
        return Err(AdapterError::PathEndWouldBeReached {
            remaining_m: remaining_route_m,
            requested_distance_m: requested_distance,
            applied_distance_m: applied_distance,
        });
    }

    let next_progress_m = input.prior_progress_m + applied_distance;
    let position = route.point_at(next_progress_m);
    let linear_velocity = Point2 {
        x_m: canonical_zero(route.direction.x_m * applied_speed),
        y_m: canonical_zero(route.direction.y_m * applied_speed),
    };
    let disposition = if requested_distance >= remaining_route_m {
        RouteDisposition::TargetSetpointWouldReachRouteEnd
    } else {
        RouteDisposition::WithinDeclaredRoute
    };

    let mut receipt = PathStepReceipt {
        schema_id: STEP_SCHEMA_ID,
        actor_id: input.actor_id.as_str().to_owned(),
        policy_profile_id: input.policy_profile_id.to_owned(),
        policy_mode: input.policy_state.mode(),
        policy_decision_tick,
        route_id: route.route_id.as_str().to_owned(),
        coordinate_frame: route.coordinate_frame.as_str().to_owned(),
        route_digest: *route.digest(),
        ticks_per_second: input.clock.ticks_per_second,
        elapsed_ticks: input.elapsed_ticks,
        elapsed_seconds: canonical_zero(elapsed_seconds),
        max_forward_speed_mps: profile.max_forward_speed_mps,
        max_acceleration_mps2: profile.max_acceleration_mps2,
        max_deceleration_mps2: profile.max_deceleration_mps2,
        prior_progress_m: canonical_zero(input.prior_progress_m),
        measured_forward_speed_mps: canonical_zero(input.measured_forward_speed_mps),
        requested_forward_speed_mps: canonical_zero(requested_speed),
        applied_forward_speed_mps: canonical_zero(applied_speed),
        requested_distance_m: canonical_zero(requested_distance),
        applied_distance_m: canonical_zero(applied_distance),
        next_progress_m: canonical_zero(next_progress_m),
        remaining_route_m: canonical_zero(remaining_route_m - applied_distance),
        disposition,
        target: KinematicPoseTarget {
            position,
            heading_radians: route.heading_radians,
            linear_velocity_mps: linear_velocity,
            angular_velocity_radians_per_second: 0.0,
        },
        receipt_digest: [0; 32],
    };
    receipt.receipt_digest = receipt.recompute_digest();
    Ok(receipt)
}

#[derive(Clone, Debug, PartialEq)]
pub enum VerificationError {
    Adapter(AdapterError),
    DigestMismatch,
    ProjectionMismatch,
}

pub fn verify_path_step(
    input: &PathStepInput<'_>,
    receipt: &PathStepReceipt,
) -> Result<(), VerificationError> {
    if receipt.receipt_digest != receipt.recompute_digest() {
        return Err(VerificationError::DigestMismatch);
    }
    let expected = plan_path_step(input).map_err(VerificationError::Adapter)?;
    if expected != *receipt {
        return Err(VerificationError::ProjectionMismatch);
    }
    Ok(())
}

fn canonical_zero(value: f64) -> f64 {
    if value == 0.0 { 0.0 } else { value }
}

fn push_string(output: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    let length = u32::try_from(bytes.len()).expect("identifier lengths are bounded");
    output.extend_from_slice(&length.to_le_bytes());
    output.extend_from_slice(bytes);
}

fn push_f64(output: &mut Vec<u8>, value: f64) {
    output.extend_from_slice(&canonical_zero(value).to_bits().to_le_bytes());
}

fn push_point(output: &mut Vec<u8>, point: Point2) {
    push_f64(output, point.x_m);
    push_f64(output, point.y_m);
}

fn mode_tag(mode: ActorMode) -> u8 {
    match mode {
        ActorMode::Waiting => 0,
        ActorMode::Crossing => 1,
        ActorMode::Paused => 2,
        ActorMode::Complete => 3,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use symtropy_road_actors::{ActorMode, MotionBounds, ScenarioActorId, ScenarioTick};

    fn actor() -> ScenarioActorId {
        ScenarioActorId::new("ped-17").unwrap()
    }

    fn bounds() -> MotionBounds {
        MotionBounds::new(2.0).unwrap()
    }

    fn policy_state(speed: f64, tick: Option<ScenarioTick>) -> ActorPolicyState {
        ActorPolicyState::from_scenario(ActorMode::Crossing, speed, tick, bounds()).unwrap()
    }

    fn route() -> StraightRoute {
        StraightRoute::new(
            RouteId::new("crossing-route-17").unwrap(),
            CoordinateFrameId::new("crossing-frame-17").unwrap(),
            Point2::new(0.0, 0.0).unwrap(),
            Point2::new(10.0, 0.0).unwrap(),
        )
        .unwrap()
    }

    fn profile() -> MotionProfile {
        MotionProfile::new(bounds(), 0.5, 2.0).unwrap()
    }

    fn input<'a>(
        actor: &'a ScenarioActorId,
        route: Option<&'a StraightRoute>,
        state: ActorPolicyState,
        progress_m: f64,
        measured_speed_mps: f64,
    ) -> PathStepInput<'a> {
        PathStepInput {
            actor_id: actor,
            policy_profile_id: "cautious_pause_v1",
            policy_state: state,
            route,
            motion_profile: profile(),
            clock: TickClock::new(10).unwrap(),
            elapsed_ticks: 1,
            prior_progress_m: progress_m,
            measured_forward_speed_mps: measured_speed_mps,
        }
    }

    #[test]
    fn route_digest_binds_route_id_frame_direction_and_endpoints() {
        let first = route();
        let second = StraightRoute::new(
            RouteId::new("crossing-route-18").unwrap(),
            CoordinateFrameId::new("crossing-frame-17").unwrap(),
            Point2::new(0.0, 0.0).unwrap(),
            Point2::new(10.0, 0.0).unwrap(),
        )
        .unwrap();
        let reversed = StraightRoute::new(
            RouteId::new("crossing-route-17").unwrap(),
            CoordinateFrameId::new("crossing-frame-17").unwrap(),
            Point2::new(10.0, 0.0).unwrap(),
            Point2::new(0.0, 0.0).unwrap(),
        )
        .unwrap();
        assert_ne!(first.digest(), second.digest());
        assert_ne!(first.digest(), reversed.digest());
    }

    #[test]
    fn zero_measured_velocity_uses_route_heading_not_body_velocity() {
        let actor = actor();
        let route = StraightRoute::new(
            RouteId::new("northbound").unwrap(),
            CoordinateFrameId::new("crossing-frame-17").unwrap(),
            Point2::new(1.0, 2.0).unwrap(),
            Point2::new(1.0, 12.0).unwrap(),
        )
        .unwrap();
        let input = input(
            &actor,
            Some(&route),
            policy_state(1.2, Some(ScenarioTick(9))),
            0.5,
            0.0,
        );
        let receipt = plan_path_step(&input).unwrap();

        assert!((receipt.target.heading_radians - std::f64::consts::FRAC_PI_2).abs() < 1e-12);
        assert_eq!(receipt.target.position.x_m, 1.0);
        assert!((receipt.target.position.y_m - 2.5025).abs() < 1e-12);
        assert!((receipt.applied_forward_speed_mps - 0.05).abs() < 1e-12);
        assert!((receipt.applied_distance_m - 0.0025).abs() < 1e-12);
    }

    #[test]
    fn acceleration_and_deceleration_limits_are_explicit() {
        let actor = actor();
        let route = route();
        let accelerate = input(
            &actor,
            Some(&route),
            policy_state(1.2, Some(ScenarioTick(9))),
            0.5,
            0.2,
        );
        let accelerated = plan_path_step(&accelerate).unwrap();
        assert!((accelerated.applied_forward_speed_mps - 0.25).abs() < 1e-12);
        assert!((accelerated.requested_distance_m - 0.07).abs() < 1e-12);
        assert!((accelerated.applied_distance_m - 0.0225).abs() < 1e-12);

        let decelerate = input(
            &actor,
            Some(&route),
            policy_state(0.0, Some(ScenarioTick(10))),
            0.5,
            1.2,
        );
        let decelerated = plan_path_step(&decelerate).unwrap();
        assert!((decelerated.applied_forward_speed_mps - 1.0).abs() < 1e-12);
        assert!((decelerated.applied_distance_m - 0.11).abs() < 1e-12);
    }

    #[test]
    fn rate_limited_target_records_when_requested_setpoint_reaches_path_end() {
        let actor = actor();
        let route = route();
        let request = input(
            &actor,
            Some(&route),
            policy_state(2.0, Some(ScenarioTick(11))),
            9.81,
            1.8,
        );
        let receipt = plan_path_step(&request).unwrap();
        assert_eq!(
            receipt.disposition,
            RouteDisposition::TargetSetpointWouldReachRouteEnd
        );
        assert!(receipt.applied_distance_m < 10.0 - receipt.prior_progress_m);
    }

    #[test]
    fn applied_step_that_reaches_route_end_is_rejected() {
        let actor = actor();
        let route = route();
        let request = input(
            &actor,
            Some(&route),
            policy_state(2.0, Some(ScenarioTick(11))),
            9.85,
            1.8,
        );
        assert!(matches!(
            plan_path_step(&request),
            Err(AdapterError::PathEndWouldBeReached { .. })
        ));
    }

    #[test]
    fn missing_route_or_missing_policy_decision_is_not_inferred() {
        let actor = actor();
        let route = route();
        let no_route = input(
            &actor,
            None,
            policy_state(1.0, Some(ScenarioTick(3))),
            0.0,
            0.0,
        );
        assert_eq!(
            plan_path_step(&no_route),
            Err(AdapterError::MissingRouteProfile)
        );

        let no_decision = input(&actor, Some(&route), policy_state(0.0, None), 0.0, 0.0);
        assert_eq!(
            plan_path_step(&no_decision),
            Err(AdapterError::MissingPolicyDecisionTick)
        );
    }

    #[test]
    fn reverse_motion_and_out_of_bounds_speed_are_rejected() {
        let actor = actor();
        let route = route();
        let reverse = input(
            &actor,
            Some(&route),
            policy_state(1.0, Some(ScenarioTick(3))),
            0.0,
            -0.1,
        );
        assert_eq!(
            plan_path_step(&reverse),
            Err(AdapterError::ReverseMotionUnsupported)
        );

        let faster_policy = ActorPolicyState::from_scenario(
            ActorMode::Crossing,
            2.5,
            Some(ScenarioTick(3)),
            MotionBounds::new(3.0).unwrap(),
        )
        .unwrap();
        let too_fast = input(&actor, Some(&route), faster_policy, 0.0, 0.0);
        assert_eq!(
            plan_path_step(&too_fast),
            Err(AdapterError::SpeedOutOfBounds)
        );
    }

    #[test]
    fn degenerate_geometry_and_invalid_clocks_fail_closed() {
        assert_eq!(
            StraightRoute::new(
                RouteId::new("zero").unwrap(),
                CoordinateFrameId::new("frame").unwrap(),
                Point2::new(1.0, 1.0).unwrap(),
                Point2::new(1.0, 1.0).unwrap(),
            ),
            Err(AdapterError::DegenerateRoute)
        );
        assert_eq!(TickClock::new(0), Err(AdapterError::InvalidTickClock));
    }

    #[test]
    fn receipt_replays_and_tampering_is_detected_even_after_rehash() {
        let actor = actor();
        let route = route();
        let request = input(
            &actor,
            Some(&route),
            policy_state(1.2, Some(ScenarioTick(9))),
            0.5,
            0.2,
        );
        let receipt = plan_path_step(&request).unwrap();
        assert_eq!(verify_path_step(&request, &receipt), Ok(()));

        let mut changed = receipt.clone();
        changed.target.position.x_m += 0.5;
        changed.receipt_digest = changed.recompute_digest();
        assert_eq!(
            verify_path_step(&request, &changed),
            Err(VerificationError::ProjectionMismatch)
        );

        let mut changed_digest = receipt;
        changed_digest.receipt_digest[0] ^= 1;
        assert_eq!(
            verify_path_step(&request, &changed_digest),
            Err(VerificationError::DigestMismatch)
        );
    }

    #[test]
    fn zero_length_time_and_nonfinite_motion_are_rejected() {
        let actor = actor();
        let route = route();
        let mut request = input(
            &actor,
            Some(&route),
            policy_state(1.0, Some(ScenarioTick(3))),
            0.0,
            0.0,
        );
        request.elapsed_ticks = 0;
        assert_eq!(
            plan_path_step(&request),
            Err(AdapterError::ZeroElapsedTicks)
        );
        request.elapsed_ticks = u64::MAX;
        request.clock = TickClock::new(1).unwrap();
        request.motion_profile = MotionProfile::new(bounds(), f64::MAX, f64::MAX).unwrap();
        assert_eq!(
            plan_path_step(&request),
            Err(AdapterError::NonFiniteDerivedMotion)
        );
    }
}
