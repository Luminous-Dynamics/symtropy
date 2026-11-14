//! Conservative interval propagation for a synthetic, bounded-velocity profile.
//!
//! This is a set-integration primitive, not an aircraft performance model.
//! It encloses occupancy only when the caller supplies velocity contribution
//! intervals that are valid over each complete time slice. It does not derive
//! those intervals from bank angle, airspeed, wind fields, or aircraft dynamics.
//!
//! All positions are metres, velocities metres/second, and time is UTC Unix
//! milliseconds. The frame and vertical datum are opaque identifiers that must
//! match the upstream model's declared semantics. This crate does not transform
//! coordinates or authenticate source digests.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::fmt;

/// Largest integer exactly representable by binary64.
const MAX_EXACT_INTEGER_MS: u64 = 9_007_199_254_740_991;

/// A finite closed interval with validated endpoint ordering.
///
/// The fields are private so callers cannot construct NaN, infinity, or
/// reversed intervals through a struct literal.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    /// Construct a finite closed interval.
    pub fn new(lower: f64, upper: f64) -> Result<Self, ReachabilityError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(ReachabilityError::NonFiniteInput);
        }
        if lower > upper {
            return Err(ReachabilityError::InvalidInterval);
        }
        Ok(Self { lower, upper })
    }

    /// Construct a singleton interval.
    pub fn point(value: f64) -> Result<Self, ReachabilityError> {
        Self::new(value, value)
    }

    /// Inclusive lower endpoint.
    pub fn lower(self) -> f64 {
        self.lower
    }

    /// Inclusive upper endpoint.
    pub fn upper(self) -> f64 {
        self.upper
    }

    /// Outward-rounded interval addition.
    pub fn checked_add(self, other: Self) -> Result<Self, ReachabilityError> {
        let lower = self.lower + other.lower;
        let upper = self.upper + other.upper;
        Self::outward(lower, upper)
    }

    /// Outward-rounded interval multiplication.
    pub fn checked_mul(self, other: Self) -> Result<Self, ReachabilityError> {
        let products = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(ReachabilityError::ArithmeticOverflow);
        }
        let mut lower = f64::INFINITY;
        let mut upper = f64::NEG_INFINITY;
        for value in products {
            lower = lower.min(next_down(value));
            upper = upper.max(next_up(value));
        }
        Self::new(lower, upper).map_err(|_| ReachabilityError::ArithmeticOverflow)
    }

    fn outward(lower_nearest: f64, upper_nearest: f64) -> Result<Self, ReachabilityError> {
        if !lower_nearest.is_finite() || !upper_nearest.is_finite() {
            return Err(ReachabilityError::ArithmeticOverflow);
        }
        Self::new(next_down(lower_nearest), next_up(upper_nearest))
            .map_err(|_| ReachabilityError::ArithmeticOverflow)
    }
}

/// Outward-rounded, additive contributions to one axis' velocity.
///
/// Every field is a signed interval contribution in metres/second. Keeping
/// these components separate preserves attribution in the request even though
/// this first integrator uses their conservative sum for propagation.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AxisVelocityEnvelope {
    /// Nominal velocity contribution.
    pub nominal_mps: Interval,
    /// Contribution from uncertainty in the estimated initial state.
    pub state_uncertainty_mps: Interval,
    /// Contribution from aircraft-model and parameter uncertainty.
    pub model_uncertainty_mps: Interval,
    /// Contribution from the declared control envelope.
    pub control_mps: Interval,
    /// Contribution from environmental disturbances such as wind.
    pub disturbance_mps: Interval,
}

impl AxisVelocityEnvelope {
    fn total(self) -> Result<Interval, ReachabilityError> {
        self.nominal_mps
            .checked_add(self.state_uncertainty_mps)?
            .checked_add(self.model_uncertainty_mps)?
            .checked_add(self.control_mps)?
            .checked_add(self.disturbance_mps)
    }
}

/// Velocity envelope for one entire time step.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityEnvelope {
    /// East-axis velocity contributions.
    pub east: AxisVelocityEnvelope,
    /// North-axis velocity contributions.
    pub north: AxisVelocityEnvelope,
    /// Up-axis velocity contributions.
    pub up: AxisVelocityEnvelope,
}

/// Initial position set in one local Cartesian frame.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct InitialPosition {
    /// East position interval, metres.
    pub east_m: Interval,
    /// North position interval, metres.
    pub north_m: Interval,
    /// Up position interval, metres from the declared vertical datum.
    pub up_m: Interval,
}

/// One declared bounded-velocity interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct VelocityStep {
    /// Inclusive start time, UTC Unix milliseconds.
    pub start_utc_ms: u64,
    /// Exclusive end time, UTC Unix milliseconds.
    pub end_utc_ms: u64,
    /// Additive, separately attributable velocity contributions valid for the
    /// entire declared interval.
    pub velocity: VelocityEnvelope,
}

/// Identity and applicability metadata for the caller's model profile.
///
/// A SHA-256 string is syntax-checked only. This crate does not verify the
/// source bytes, signature, publisher, physical validity, or regulatory scope.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProfileIdentity {
    /// Stable profile identifier.
    pub profile_id: String,
    /// Exact profile revision.
    pub revision: String,
    /// Local horizontal coordinate-frame identifier.
    pub horizontal_frame_id: String,
    /// Vertical datum identifier.
    pub vertical_datum_id: String,
    /// SHA-256 syntax for the profile artifact.
    pub profile_sha256: String,
}

impl ProfileIdentity {
    fn validate(&self) -> Result<(), ReachabilityError> {
        if self.profile_id.trim().is_empty()
            || self.revision.trim().is_empty()
            || self.horizontal_frame_id.trim().is_empty()
            || self.vertical_datum_id.trim().is_empty()
        {
            return Err(ReachabilityError::InvalidProfile);
        }
        if !is_sha256(&self.profile_sha256) {
            return Err(ReachabilityError::InvalidDigest);
        }
        Ok(())
    }
}

/// Request to propagate a bounded position set through bounded velocity slices.
#[derive(Debug, Clone, PartialEq)]
pub struct ReachabilityRequest {
    /// Synthetic model profile identity.
    pub profile: ProfileIdentity,
    /// Initial position set.
    pub initial_position: InitialPosition,
    /// Contiguous velocity envelopes covering the requested time horizon.
    pub steps: Vec<VelocityStep>,
}

/// Conservative occupancy box for one half-open time interval.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TubeSlice {
    /// Inclusive start, UTC Unix milliseconds.
    pub start_utc_ms: u64,
    /// Exclusive end, UTC Unix milliseconds.
    pub end_utc_ms: u64,
    /// East occupancy interval, metres.
    pub east_m: Interval,
    /// North occupancy interval, metres.
    pub north_m: Interval,
    /// Up occupancy interval, metres from the declared vertical datum.
    pub up_m: Interval,
}

/// Reachable occupancy tube under the supplied bounded-velocity contract.
///
/// Each slice encloses all positions reachable at any time in that slice,
/// assuming the supplied axis velocity bounds hold throughout it. The result
/// may be very conservative because axis and time dependencies are discarded.
#[derive(Debug, Clone, PartialEq)]
pub struct ReachabilityTube {
    /// Model profile identity carried through the output.
    pub profile: ProfileIdentity,
    /// Ordered, contiguous occupancy slices.
    pub slices: Vec<TubeSlice>,
    /// Enclosed position set at the final time boundary.
    pub final_position: InitialPosition,
}

/// Typed failure. A failure never implies that a path is clear.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReachabilityError {
    /// An interval endpoint is not finite.
    NonFiniteInput,
    /// Interval lower endpoint exceeds its upper endpoint.
    InvalidInterval,
    /// An operation exceeded the finite binary64 domain.
    ArithmeticOverflow,
    /// A model profile is missing a required identifier.
    InvalidProfile,
    /// A digest is not exactly 64 hexadecimal characters.
    InvalidDigest,
    /// A time interval is invalid or outside the exact integer domain.
    InvalidTime,
    /// Adjacent time steps are not exactly contiguous.
    NonContiguousSteps,
    /// No propagation steps were supplied.
    EmptySteps,
}

impl fmt::Display for ReachabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for ReachabilityError {}

/// Build a conservative occupancy tube from supplied bounded-velocity envelopes.
///
/// The caller is responsible for proving that each velocity envelope contains
/// every possible instantaneous velocity during its complete time step. This
/// function propagates those bounds; it does not prove the aircraft model.
pub fn propagate_bounded_velocity(
    request: &ReachabilityRequest,
) -> Result<ReachabilityTube, ReachabilityError> {
    request.profile.validate()?;
    if request.steps.is_empty() {
        return Err(ReachabilityError::EmptySteps);
    }

    let mut position = request.initial_position;
    let mut slices = Vec::with_capacity(request.steps.len());
    let mut previous_end = None;

    for step in &request.steps {
        if step.start_utc_ms >= step.end_utc_ms
            || step.start_utc_ms > MAX_EXACT_INTEGER_MS
            || step.end_utc_ms > MAX_EXACT_INTEGER_MS
        {
            return Err(ReachabilityError::InvalidTime);
        }
        if let Some(expected_start) = previous_end {
            if step.start_utc_ms != expected_start {
                return Err(ReachabilityError::NonContiguousSteps);
            }
        }

        let elapsed_ms = step.end_utc_ms - step.start_utc_ms;
        if elapsed_ms > MAX_EXACT_INTEGER_MS {
            return Err(ReachabilityError::InvalidTime);
        }
        let elapsed_nearest = elapsed_ms as f64 / 1_000.0;
        if !elapsed_nearest.is_finite() || elapsed_nearest <= 0.0 {
            return Err(ReachabilityError::InvalidTime);
        }
        let elapsed_seconds = Interval::new(
            next_down(elapsed_nearest),
            next_up(elapsed_nearest),
        )?;
        let elapsed_during_slice = Interval::new(0.0, elapsed_seconds.upper())?;

        let east_velocity = step.velocity.east.total()?;
        let north_velocity = step.velocity.north.total()?;
        let up_velocity = step.velocity.up.total()?;

        // Occupancy covers every elapsed time in [0, dt], not just the endpoint.
        let occupancy = InitialPosition {
            east_m: position
                .east_m
                .checked_add(east_velocity.checked_mul(elapsed_during_slice)?)?,
            north_m: position
                .north_m
                .checked_add(north_velocity.checked_mul(elapsed_during_slice)?)?,
            up_m: position
                .up_m
                .checked_add(up_velocity.checked_mul(elapsed_during_slice)?)?,
        };

        // Endpoint state is carried into the next contiguous interval.
        position = InitialPosition {
            east_m: position.east_m.checked_add(
                east_velocity.checked_mul(elapsed_seconds)?,
            )?,
            north_m: position.north_m.checked_add(
                north_velocity.checked_mul(elapsed_seconds)?,
            )?,
            up_m: position
                .up_m
                .checked_add(up_velocity.checked_mul(elapsed_seconds)?)?,
        };

        slices.push(TubeSlice {
            start_utc_ms: step.start_utc_ms,
            end_utc_ms: step.end_utc_ms,
            east_m: occupancy.east_m,
            north_m: occupancy.north_m,
            up_m: occupancy.up_m,
        });
        previous_end = Some(step.end_utc_ms);
    }

    Ok(ReachabilityTube {
        profile: request.profile.clone(),
        slices,
        final_position: position,
    })
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn interval(lower: f64, upper: f64) -> Interval {
        Interval::new(lower, upper).expect("test interval must be valid")
    }

    fn axis(nominal: Interval, state: Interval, disturbance: Interval) -> AxisVelocityEnvelope {
        AxisVelocityEnvelope {
            nominal_mps: nominal,
            state_uncertainty_mps: state,
            model_uncertainty_mps: interval(0.0, 0.0),
            control_mps: interval(0.0, 0.0),
            disturbance_mps: disturbance,
        }
    }

    fn profile() -> ProfileIdentity {
        ProfileIdentity {
            profile_id: "synthetic-fixed-wing-constant-velocity-v0".into(),
            revision: "0.1.0".into(),
            horizontal_frame_id: "SYNTHETIC-ENU-LOCAL-01".into(),
            vertical_datum_id: "SYNTHETIC-LOCAL-UP-01".into(),
            profile_sha256: "a".repeat(64),
        }
    }

    fn request(
        start: u64,
        end: u64,
        east: AxisVelocityEnvelope,
    ) -> ReachabilityRequest {
        ReachabilityRequest {
            profile: profile(),
            initial_position: InitialPosition {
                east_m: interval(2.0, 3.0),
                north_m: interval(-1.0, 1.0),
                up_m: interval(0.0, 2.0),
            },
            steps: vec![VelocityStep {
                start_utc_ms: start,
                end_utc_ms: end,
                velocity: VelocityEnvelope {
                    east,
                    north: axis(interval(0.0, 0.0), interval(0.0, 0.0), interval(0.0, 0.0)),
                    up: axis(interval(0.0, 0.0), interval(0.0, 0.0), interval(0.0, 0.0)),
                },
            }],
        }
    }

    #[test]
    fn integer_oracle_is_contained_for_all_bounded_east_velocities() {
        let east = axis(interval(10.0, 10.0), interval(-2.0, 2.0), interval(-1.0, 1.0));
        let tube = propagate_bounded_velocity(&request(0, 1_000, east)).unwrap();
        let slice = tube.slices[0];

        // Independent small integer oracle: every integer initial point and
        // velocity in the declared closed bounds must be inside the outer box.
        for initial in 2..=3 {
            for velocity in 7..=13 {
                let endpoint = initial + velocity;
                assert!(slice.east_m.lower() <= initial as f64);
                assert!(slice.east_m.upper() >= endpoint as f64);
                assert!(tube.final_position.east_m.lower() <= endpoint as f64);
                assert!(tube.final_position.east_m.upper() >= endpoint as f64);
            }
        }
    }

    #[test]
    fn widening_disturbance_cannot_shrink_the_tube() {
        let narrow = axis(interval(10.0, 10.0), interval(0.0, 0.0), interval(-0.5, 0.5));
        let wide = axis(interval(10.0, 10.0), interval(0.0, 0.0), interval(-2.0, 2.0));
        let narrow_tube = propagate_bounded_velocity(&request(0, 1_000, narrow)).unwrap();
        let wide_tube = propagate_bounded_velocity(&request(0, 1_000, wide)).unwrap();

        assert!(wide_tube.slices[0].east_m.lower() <= narrow_tube.slices[0].east_m.lower());
        assert!(wide_tube.slices[0].east_m.upper() >= narrow_tube.slices[0].east_m.upper());
    }

    #[test]
    fn time_origin_translation_preserves_position_tube() {
        let east = axis(interval(5.0, 6.0), interval(-0.2, 0.2), interval(-0.1, 0.1));
        let first = propagate_bounded_velocity(&request(0, 2_000, east)).unwrap();
        let shifted = propagate_bounded_velocity(&request(50_000, 52_000, east)).unwrap();

        assert_eq!(first.slices[0].east_m, shifted.slices[0].east_m);
        assert_eq!(first.final_position.east_m, shifted.final_position.east_m);
    }

    #[test]
    fn repeated_replay_is_bitwise_deterministic_for_fixed_inputs() {
        let east = axis(interval(10.0, 10.0), interval(-0.1, 0.1), interval(-0.2, 0.2));
        let input = request(0, 1_000, east);
        assert_eq!(
            propagate_bounded_velocity(&input).unwrap(),
            propagate_bounded_velocity(&input).unwrap()
        );
    }

    #[test]
    fn rejects_profile_with_invalid_digest_syntax() {
        let east = axis(interval(1.0, 1.0), interval(0.0, 0.0), interval(0.0, 0.0));
        let mut input = request(0, 1_000, east);
        input.profile.profile_sha256 = "not-a-digest".into();
        assert_eq!(
            propagate_bounded_velocity(&input),
            Err(ReachabilityError::InvalidDigest)
        );
    }

    #[test]
    fn rejects_non_contiguous_time_steps() {
        let east = axis(interval(1.0, 1.0), interval(0.0, 0.0), interval(0.0, 0.0));
        let mut input = request(0, 1_000, east);
        let second = VelocityStep {
            start_utc_ms: 1_001,
            end_utc_ms: 2_000,
            velocity: input.steps[0].velocity,
        };
        input.steps.push(second);
        assert_eq!(
            propagate_bounded_velocity(&input),
            Err(ReachabilityError::NonContiguousSteps)
        );
    }

    #[test]
    fn rejects_time_outside_exact_integer_domain() {
        let east = axis(interval(1.0, 1.0), interval(0.0, 0.0), interval(0.0, 0.0));
        let input = request(MAX_EXACT_INTEGER_MS, MAX_EXACT_INTEGER_MS + 1, east);
        assert_eq!(
            propagate_bounded_velocity(&input),
            Err(ReachabilityError::InvalidTime)
        );
    }

    #[test]
    fn rejects_empty_step_sequence() {
        let east = axis(interval(1.0, 1.0), interval(0.0, 0.0), interval(0.0, 0.0));
        let mut input = request(0, 1_000, east);
        input.steps.clear();
        assert_eq!(
            propagate_bounded_velocity(&input),
            Err(ReachabilityError::EmptySteps)
        );
    }
}
