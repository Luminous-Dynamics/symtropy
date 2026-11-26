#![forbid(unsafe_code)]

//! Replayable anisotropic broadphase envelopes for fixed-orientation translation.
//!
//! Given a tight AABB and a conservative signed-displacement interval per axis,
//! this module encloses the union of the initial and every translated AABB in
//! that interval. Arithmetic is rounded outward. The receipt binds the exact
//! inputs and derivation context so stale envelopes cannot be replayed.
//!
//! This verifies the derivation from supplied motion bounds. It does not prove
//! that an upstream motion-bound producer conservatively encloses real motion.

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
pub struct Digest([u8; 32]);

impl Digest {
    #[must_use]
    pub const fn from_bytes(bytes: [u8; 32]) -> Self {
        Self(bytes)
    }

    #[must_use]
    pub const fn as_bytes(&self) -> &[u8; 32] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, Hash, PartialEq)]
pub struct ColliderIdentity {
    pub body_slot: u32,
    pub body_generation: u32,
    pub collider_id: u64,
    pub subshape_path: Vec<u32>,
}

impl ColliderIdentity {
    pub fn validate(&self) -> Result<(), EnvelopeError> {
        if self.body_generation == 0 || self.subshape_path.len() > 64 {
            return Err(EnvelopeError::InvalidIdentity);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct EnvelopeContext {
    pub world_revision: u64,
    pub frame_index: u64,
    pub horizon_steps: u32,
    pub integrator: Digest,
    pub motion_bound_producer: Digest,
    pub arithmetic_profile: Digest,
}

impl EnvelopeContext {
    pub fn validate(&self) -> Result<(), EnvelopeError> {
        if self.horizon_steps == 0 {
            return Err(EnvelopeError::InvalidHorizon);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb<const D: usize> {
    min: [f64; D],
    max: [f64; D],
}

impl<const D: usize> Aabb<D> {
    pub fn new(min: [f64; D], max: [f64; D]) -> Result<Self, EnvelopeError> {
        let value = Self { min, max };
        if D == 0 || !value.is_valid() {
            return Err(EnvelopeError::InvalidBounds);
        }
        Ok(value)
    }

    #[must_use]
    pub fn min(&self) -> &[f64; D] {
        &self.min
    }

    #[must_use]
    pub fn max(&self) -> &[f64; D] {
        &self.max
    }

    #[must_use]
    pub fn is_valid(&self) -> bool {
        (0..D).all(|axis| {
            self.min[axis].is_finite()
                && self.max[axis].is_finite()
                && self.min[axis] <= self.max[axis]
        })
    }

    #[must_use]
    pub fn contains(&self, other: &Self) -> bool {
        (0..D).all(|axis| self.min[axis] <= other.min[axis] && self.max[axis] >= other.max[axis])
    }

    #[must_use]
    pub fn contains_point(&self, point: [f64; D]) -> bool {
        (0..D).all(|axis| self.min[axis] <= point[axis] && point[axis] <= self.max[axis])
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct DisplacementInterval<const D: usize> {
    pub min: [f64; D],
    pub max: [f64; D],
}

impl<const D: usize> DisplacementInterval<D> {
    pub fn validate(&self) -> Result<(), EnvelopeError> {
        if D == 0
            || (0..D).any(|axis| {
                !self.min[axis].is_finite()
                    || !self.max[axis].is_finite()
                    || self.min[axis] > self.max[axis]
            })
        {
            return Err(EnvelopeError::InvalidDisplacementInterval);
        }
        Ok(())
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum MotionKind {
    FixedOrientationTranslation,
    Rotation,
    UnboundedOrUnknown,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeStatus {
    CertifiedFixedOrientationTranslation,
    UnsupportedRotation,
    UnboundedMotion,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum EnvelopeError {
    InvalidIdentity,
    InvalidHorizon,
    InvalidBounds,
    InvalidDisplacementInterval,
    InvalidGuard,
    InvalidTimeStep,
    InvalidAngularMotionState,
    NonFiniteArithmetic,
    UnsupportedStatus,
    MissingReceipt,
    ReplayMismatch,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnvelopeReceipt<const D: usize> {
    pub identity: ColliderIdentity,
    pub context: EnvelopeContext,
    pub motion_kind: MotionKind,
    pub tight_bounds: Aabb<D>,
    pub displacement: DisplacementInterval<D>,
    pub guard: f64,
    pub envelope: Aabb<D>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EnvelopeResult<const D: usize> {
    pub status: EnvelopeStatus,
    pub envelope: Option<Aabb<D>>,
    pub receipt: Option<EnvelopeReceipt<D>>,
}

fn outward_min(value: f64) -> Result<f64, EnvelopeError> {
    let result = value.next_down();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(EnvelopeError::NonFiniteArithmetic)
    }
}

fn outward_max(value: f64) -> Result<f64, EnvelopeError> {
    let result = value.next_up();
    if result.is_finite() {
        Ok(result)
    } else {
        Err(EnvelopeError::NonFiniteArithmetic)
    }
}

fn checked_add(left: f64, right: f64) -> Result<f64, EnvelopeError> {
    let result = left + right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(EnvelopeError::NonFiniteArithmetic)
    }
}

fn checked_sub(left: f64, right: f64) -> Result<f64, EnvelopeError> {
    let result = left - right;
    if result.is_finite() {
        Ok(result)
    } else {
        Err(EnvelopeError::NonFiniteArithmetic)
    }
}

/// Compute the outward-rounded union of the tight AABB and all its possible
/// fixed-orientation translations in the supplied axis-aligned displacement
/// interval, then expand by a nonnegative arithmetic/geometry guard.
pub fn build_envelope<const D: usize>(
    identity: ColliderIdentity,
    context: EnvelopeContext,
    motion_kind: MotionKind,
    tight_bounds: Aabb<D>,
    displacement: DisplacementInterval<D>,
    guard: f64,
) -> Result<EnvelopeResult<D>, EnvelopeError> {
    identity.validate()?;
    context.validate()?;
    if !tight_bounds.is_valid() {
        return Err(EnvelopeError::InvalidBounds);
    }
    displacement.validate()?;
    if !guard.is_finite() || guard < 0.0 {
        return Err(EnvelopeError::InvalidGuard);
    }

    let unsupported_status = match motion_kind {
        MotionKind::FixedOrientationTranslation => None,
        MotionKind::Rotation => Some(EnvelopeStatus::UnsupportedRotation),
        MotionKind::UnboundedOrUnknown => Some(EnvelopeStatus::UnboundedMotion),
    };
    if let Some(status) = unsupported_status {
        return Ok(EnvelopeResult {
            status,
            envelope: None,
            receipt: None,
        });
    }

    let mut min = [0.0; D];
    let mut max = [0.0; D];
    for (((out_min, out_max), (tight_min, tight_max)), (motion_min, motion_max)) in min
        .iter_mut()
        .zip(max.iter_mut())
        .zip(tight_bounds.min.iter().zip(tight_bounds.max.iter()))
        .zip(displacement.min.iter().zip(displacement.max.iter()))
    {
        let translated_min = outward_min(checked_add(*tight_min, *motion_min)?)?;
        let translated_max = outward_max(checked_add(*tight_max, *motion_max)?)?;

        let swept_min = (*tight_min).min(translated_min);
        let swept_max = (*tight_max).max(translated_max);
        *out_min = outward_min(checked_sub(*swept_min, guard)?)?;
        *out_max = outward_max(checked_add(*swept_max, guard)?)?;
    }

    let envelope = Aabb::new(min, max)?;
    if !envelope.contains(&tight_bounds) {
        return Err(EnvelopeError::ReplayMismatch);
    }

    let receipt = EnvelopeReceipt {
        identity,
        context,
        motion_kind,
        tight_bounds,
        displacement,
        guard,
        envelope,
    };
    Ok(EnvelopeResult {
        status: EnvelopeStatus::CertifiedFixedOrientationTranslation,
        envelope: Some(envelope),
        receipt: Some(receipt),
    })
}

/// The angular-velocity threshold matches the integrator's rotation update:
/// rotation is applied only when post-integration angular speed squared is
/// greater than 1e-24.
pub const SEMI_IMPLICIT_ROTATION_THRESHOLD_SQUARED: f64 = 1.0e-24;

/// Half of one Q16.16 quantization interval (1 / 2^17).
pub const Q16_16_HALF_STEP: f64 = 1.0 / 131_072.0;

/// Linear/angular state observed after the actual semi-implicit integrator.
/// post_linear_velocity is expected to include acceleration, damping, speed
/// clamp and any deterministic-net velocity quantization performed by the
/// caller. The position update itself used the pre-quantized velocity, so the
/// deterministic-net guard accounts for half a velocity quantum multiplied
/// by dt as well as half a position quantum.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SemiImplicitEulerMotion<const D: usize> {
    pub post_linear_velocity: [f64; D],
    pub post_angular_velocity_norm_squared: f64,
    pub dt: f64,
    pub deterministic_net_q16_16: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegratorEnvelopeReceipt<const D: usize> {
    pub base: EnvelopeReceipt<D>,
    pub motion: SemiImplicitEulerMotion<D>,
    pub caller_guard: f64,
    pub quantization_guard: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct IntegratorEnvelopeResult<const D: usize> {
    pub status: EnvelopeStatus,
    pub envelope: Option<Aabb<D>>,
    pub receipt: Option<IntegratorEnvelopeReceipt<D>>,
}

/// Produce a one-step conservative translation envelope matching Symtropy's
/// semi-implicit Euler pipeline. The supplied velocity is the post-integrator
/// value after damping and speed clamping. In deterministic-net mode it is also
/// the Q16.16-quantized velocity used by the CCD sweep. Position quantization
/// and the difference between pre/post-quantized velocity are included in the
/// guard. Changed orientation produces no translation certificate.
pub fn build_semi_implicit_euler_envelope<const D: usize>(
    identity: ColliderIdentity,
    context: EnvelopeContext,
    tight_bounds: Aabb<D>,
    motion: SemiImplicitEulerMotion<D>,
    caller_guard: f64,
) -> Result<IntegratorEnvelopeResult<D>, EnvelopeError> {
    identity.validate()?;
    context.validate()?;
    if !tight_bounds.is_valid() {
        return Err(EnvelopeError::InvalidBounds);
    }
    if !motion.dt.is_finite() || motion.dt <= 0.0 {
        return Err(EnvelopeError::InvalidTimeStep);
    }
    if !motion.post_angular_velocity_norm_squared.is_finite()
        || motion.post_angular_velocity_norm_squared < 0.0
    {
        return Err(EnvelopeError::InvalidAngularMotionState);
    }
    if motion
        .post_linear_velocity
        .iter()
        .any(|value| !value.is_finite())
    {
        return Err(EnvelopeError::NonFiniteArithmetic);
    }
    if !caller_guard.is_finite() || caller_guard < 0.0 {
        return Err(EnvelopeError::InvalidGuard);
    }

    if motion.post_angular_velocity_norm_squared > SEMI_IMPLICIT_ROTATION_THRESHOLD_SQUARED {
        return Ok(IntegratorEnvelopeResult {
            status: EnvelopeStatus::UnsupportedRotation,
            envelope: None,
            receipt: None,
        });
    }

    let mut displacement = DisplacementInterval {
        min: [0.0; D],
        max: [0.0; D],
    };
    for ((displacement_min, displacement_max), velocity) in displacement
        .min
        .iter_mut()
        .zip(displacement.max.iter_mut())
        .zip(motion.post_linear_velocity.iter())
    {
        let endpoint = checked_mul(*velocity, motion.dt)?;
        let lower = outward_min(endpoint)?;
        let upper = outward_max(endpoint)?;
        *displacement_min = 0.0_f64.min(lower);
        *displacement_max = 0.0_f64.max(upper);
    }

    let quantization_guard = if motion.deterministic_net_q16_16 {
        // The integrator advances position with pre-quantized velocity, while
        // the subsequent CCD sweep uses the post-quantized velocity. The
        // per-axis discrepancy is bounded by dt*half_step + half_step.
        outward_max(checked_mul(Q16_16_HALF_STEP, motion.dt + 1.0)?)?
    } else {
        0.0
    };
    let total_guard = outward_max(checked_add(caller_guard, quantization_guard)?)?;
    let base_result = build_envelope(
        identity,
        context,
        MotionKind::FixedOrientationTranslation,
        tight_bounds,
        displacement,
        total_guard,
    )?;
    let envelope = base_result
        .envelope
        .ok_or(EnvelopeError::UnsupportedStatus)?;
    let base = base_result.receipt.ok_or(EnvelopeError::MissingReceipt)?;

    Ok(IntegratorEnvelopeResult {
        status: EnvelopeStatus::CertifiedFixedOrientationTranslation,
        envelope: Some(envelope),
        receipt: Some(IntegratorEnvelopeReceipt {
            base,
            motion,
            caller_guard,
            quantization_guard,
        }),
    })
}

/// Replay an integrator-bound envelope against the current post-integration
/// state, dt, determinism profile and geometry. Changed source state invalidates.
pub fn verify_semi_implicit_euler_envelope<const D: usize>(
    receipt: &IntegratorEnvelopeReceipt<D>,
    identity: &ColliderIdentity,
    context: EnvelopeContext,
    tight_bounds: Aabb<D>,
    motion: SemiImplicitEulerMotion<D>,
    caller_guard: f64,
) -> Result<(), EnvelopeError> {
    if receipt.motion != motion || receipt.caller_guard != caller_guard {
        return Err(EnvelopeError::ReplayMismatch);
    }
    let replay = build_semi_implicit_euler_envelope(
        identity.clone(),
        context,
        tight_bounds,
        motion,
        caller_guard,
    )?;
    if replay.receipt.as_ref() != Some(receipt) {
        return Err(EnvelopeError::ReplayMismatch);
    }
    verify_envelope(
        &receipt.base,
        identity,
        context,
        tight_bounds,
        receipt.base.displacement,
        outward_max(checked_add(caller_guard, receipt.quantization_guard)?)?,
    )
}

fn checked_mul(left: f64, right: f64) -> Result<f64, EnvelopeError> {
    let value = left * right;
    if value.is_finite() {
        Ok(value)
    } else {
        Err(EnvelopeError::NonFiniteArithmetic)
    }
}

/// Replay a certified envelope against the current identity, context, geometry,
/// displacement interval and guard. Any changed source invalidates the receipt.
pub fn verify_envelope<const D: usize>(
    receipt: &EnvelopeReceipt<D>,
    identity: &ColliderIdentity,
    context: EnvelopeContext,
    tight_bounds: Aabb<D>,
    displacement: DisplacementInterval<D>,
    guard: f64,
) -> Result<(), EnvelopeError> {
    if &receipt.identity != identity
        || receipt.context != context
        || receipt.tight_bounds != tight_bounds
        || receipt.displacement != displacement
        || receipt.guard != guard
        || receipt.motion_kind != MotionKind::FixedOrientationTranslation
    {
        return Err(EnvelopeError::ReplayMismatch);
    }

    let replay = build_envelope(
        identity.clone(),
        context,
        MotionKind::FixedOrientationTranslation,
        tight_bounds,
        displacement,
        guard,
    )?;
    if replay.envelope != Some(receipt.envelope) {
        return Err(EnvelopeError::ReplayMismatch);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn digest(byte: u8) -> Digest {
        Digest::from_bytes([byte; 32])
    }

    fn identity() -> ColliderIdentity {
        ColliderIdentity {
            body_slot: 7,
            body_generation: 3,
            collider_id: 42,
            subshape_path: vec![1, 4, 9],
        }
    }

    fn context() -> EnvelopeContext {
        EnvelopeContext {
            world_revision: 100,
            frame_index: 11,
            horizon_steps: 4,
            integrator: digest(1),
            motion_bound_producer: digest(2),
            arithmetic_profile: digest(3),
        }
    }

    fn box2() -> Aabb<2> {
        Aabb::new([1.0, 2.0], [3.0, 4.0]).unwrap()
    }

    fn interval2() -> DisplacementInterval<2> {
        DisplacementInterval {
            min: [-2.0, -0.5],
            max: [1.0, 3.0],
        }
    }

    #[test]
    fn mixed_sign_interval_produces_directional_swept_union() {
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            box2(),
            interval2(),
            0.0,
        )
        .unwrap();
        assert_eq!(
            result.status,
            EnvelopeStatus::CertifiedFixedOrientationTranslation
        );
        let envelope = result.envelope.unwrap();
        assert!(envelope.min()[0] < -1.0);
        assert!(envelope.max()[0] > 4.0);
        assert!(envelope.min()[1] <= 1.5);
        assert!(envelope.max()[1] >= 7.0);
    }

    #[test]
    fn envelope_contains_initial_and_endpoint_boxes_for_interval_corners() {
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            box2(),
            interval2(),
            0.125,
        )
        .unwrap();
        let envelope = result.envelope.unwrap();
        assert!(envelope.contains(&box2()));

        for dx in [interval2().min[0], interval2().max[0]] {
            for dy in [interval2().min[1], interval2().max[1]] {
                let translated = Aabb::new([1.0 + dx, 2.0 + dy], [3.0 + dx, 4.0 + dy]).unwrap();
                assert!(envelope.contains(&translated));
            }
        }
    }

    #[test]
    fn one_dimensional_positive_motion_is_not_symmetric_padding() {
        let tight = Aabb::new([5.0], [7.0]).unwrap();
        let interval = DisplacementInterval {
            min: [0.0],
            max: [10.0],
        };
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            tight,
            interval,
            0.0,
        )
        .unwrap();
        let envelope = result.envelope.unwrap();
        assert!(envelope.min()[0] < 5.0);
        assert!(envelope.max()[0] > 17.0);
        assert!(envelope.max()[0] - 7.0 > 10.0);
    }

    #[test]
    fn zero_displacement_still_outward_encloses_tight_bounds() {
        let tight = Aabb::new([0.0, 0.0, 0.0], [1.0, 1.0, 1.0]).unwrap();
        let interval = DisplacementInterval {
            min: [0.0; 3],
            max: [0.0; 3],
        };
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            tight,
            interval,
            0.0,
        )
        .unwrap();
        assert!(result.envelope.unwrap().contains(&tight));
    }

    #[test]
    fn four_dimensional_envelope_is_supported_for_reference_diagnostics() {
        let tight = Aabb::new([0.0; 4], [1.0; 4]).unwrap();
        let interval = DisplacementInterval {
            min: [-1.0, 0.0, -2.0, 0.5],
            max: [1.0, 2.0, 0.0, 1.5],
        };
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            tight,
            interval,
            0.01,
        )
        .unwrap();
        assert!(result.envelope.unwrap().contains(&tight));
    }

    #[test]
    fn unsupported_rotation_and_unknown_motion_never_emit_a_certificate() {
        let rotation = build_envelope(
            identity(),
            context(),
            MotionKind::Rotation,
            box2(),
            interval2(),
            0.0,
        )
        .unwrap();
        assert_eq!(rotation.status, EnvelopeStatus::UnsupportedRotation);
        assert!(rotation.envelope.is_none());
        assert!(rotation.receipt.is_none());

        let unknown = build_envelope(
            identity(),
            context(),
            MotionKind::UnboundedOrUnknown,
            box2(),
            interval2(),
            0.0,
        )
        .unwrap();
        assert_eq!(unknown.status, EnvelopeStatus::UnboundedMotion);
        assert!(unknown.receipt.is_none());
    }

    #[test]
    fn replay_succeeds_for_exact_inputs_and_fails_after_bound_change() {
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            box2(),
            interval2(),
            0.25,
        )
        .unwrap();
        let receipt = result.receipt.unwrap();
        assert!(
            verify_envelope(&receipt, &identity(), context(), box2(), interval2(), 0.25,).is_ok()
        );

        let changed = DisplacementInterval {
            min: [-2.1, -0.5],
            max: [1.0, 3.0],
        };
        assert_eq!(
            verify_envelope(&receipt, &identity(), context(), box2(), changed, 0.25,),
            Err(EnvelopeError::ReplayMismatch)
        );
    }

    #[test]
    fn replay_fails_for_changed_identity_revision_horizon_or_arithmetic_profile() {
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            box2(),
            interval2(),
            0.25,
        )
        .unwrap();
        let receipt = result.receipt.unwrap();

        let mut changed_identity = identity();
        changed_identity.body_generation += 1;
        assert_eq!(
            verify_envelope(
                &receipt,
                &changed_identity,
                context(),
                box2(),
                interval2(),
                0.25
            ),
            Err(EnvelopeError::ReplayMismatch)
        );

        let mut changed_context = context();
        changed_context.world_revision += 1;
        assert_eq!(
            verify_envelope(
                &receipt,
                &identity(),
                changed_context,
                box2(),
                interval2(),
                0.25
            ),
            Err(EnvelopeError::ReplayMismatch)
        );

        let mut changed_context = context();
        changed_context.horizon_steps += 1;
        assert_eq!(
            verify_envelope(
                &receipt,
                &identity(),
                changed_context,
                box2(),
                interval2(),
                0.25
            ),
            Err(EnvelopeError::ReplayMismatch)
        );

        let mut changed_context = context();
        changed_context.arithmetic_profile = digest(9);
        assert_eq!(
            verify_envelope(
                &receipt,
                &identity(),
                changed_context,
                box2(),
                interval2(),
                0.25
            ),
            Err(EnvelopeError::ReplayMismatch)
        );
    }

    #[test]
    fn semi_implicit_producer_sweeps_using_post_integrator_velocity() {
        let tight = Aabb::new([1.0, 2.0], [3.0, 4.0]).unwrap();
        let motion = SemiImplicitEulerMotion {
            post_linear_velocity: [4.0, -2.0],
            post_angular_velocity_norm_squared: 0.0,
            dt: 0.25,
            deterministic_net_q16_16: false,
        };
        let result =
            build_semi_implicit_euler_envelope(identity(), context(), tight, motion, 0.0).unwrap();
        assert_eq!(
            result.status,
            EnvelopeStatus::CertifiedFixedOrientationTranslation
        );
        let envelope = result.envelope.unwrap();
        let endpoint = Aabb::new([2.0, 1.5], [4.0, 3.5]).unwrap();
        assert!(envelope.contains(&tight));
        assert!(envelope.contains(&endpoint));
    }

    #[test]
    fn deterministic_net_guard_covers_position_and_velocity_quantization() {
        let tight = Aabb::new([0.0, 0.0], [1.0, 1.0]).unwrap();
        let motion = SemiImplicitEulerMotion {
            post_linear_velocity: [10.0, 0.0],
            post_angular_velocity_norm_squared: 0.0,
            dt: 0.5,
            deterministic_net_q16_16: true,
        };
        let result =
            build_semi_implicit_euler_envelope(identity(), context(), tight, motion, 0.0).unwrap();
        let receipt = result.receipt.unwrap();
        let expected_guard = Q16_16_HALF_STEP * (motion.dt + 1.0);
        assert!(receipt.quantization_guard >= expected_guard);
        assert!(result.envelope.unwrap().contains(&tight));
    }

    #[test]
    fn post_integrator_rotation_forces_unsupported_fallback() {
        let motion = SemiImplicitEulerMotion {
            post_linear_velocity: [1.0, 0.0],
            post_angular_velocity_norm_squared: SEMI_IMPLICIT_ROTATION_THRESHOLD_SQUARED * 2.0,
            dt: 0.1,
            deterministic_net_q16_16: false,
        };
        let result =
            build_semi_implicit_euler_envelope(identity(), context(), box2(), motion, 0.0).unwrap();
        assert_eq!(result.status, EnvelopeStatus::UnsupportedRotation);
        assert!(result.envelope.is_none());
        assert!(result.receipt.is_none());
    }

    #[test]
    fn semi_implicit_receipt_replay_binds_velocity_dt_and_quantization() {
        let motion = SemiImplicitEulerMotion {
            post_linear_velocity: [1.5, -0.25],
            post_angular_velocity_norm_squared: 0.0,
            dt: 0.01,
            deterministic_net_q16_16: true,
        };
        let result =
            build_semi_implicit_euler_envelope(identity(), context(), box2(), motion, 0.001)
                .unwrap();
        let receipt = result.receipt.unwrap();
        assert!(
            verify_semi_implicit_euler_envelope(
                &receipt,
                &identity(),
                context(),
                box2(),
                motion,
                0.001,
            )
            .is_ok()
        );

        let changed_velocity = SemiImplicitEulerMotion {
            post_linear_velocity: [1.6, -0.25],
            ..motion
        };
        assert_eq!(
            verify_semi_implicit_euler_envelope(
                &receipt,
                &identity(),
                context(),
                box2(),
                changed_velocity,
                0.001,
            ),
            Err(EnvelopeError::ReplayMismatch)
        );
    }

    #[test]
    fn semi_implicit_invalid_timestep_angular_state_and_velocity_fail_closed() {
        let mut motion = SemiImplicitEulerMotion {
            post_linear_velocity: [1.0, 0.0],
            post_angular_velocity_norm_squared: 0.0,
            dt: 0.0,
            deterministic_net_q16_16: false,
        };
        assert_eq!(
            build_semi_implicit_euler_envelope(identity(), context(), box2(), motion, 0.0,),
            Err(EnvelopeError::InvalidTimeStep)
        );

        motion.dt = 0.1;
        motion.post_angular_velocity_norm_squared = f64::NAN;
        assert_eq!(
            build_semi_implicit_euler_envelope(identity(), context(), box2(), motion, 0.0,),
            Err(EnvelopeError::InvalidAngularMotionState)
        );

        motion.post_angular_velocity_norm_squared = 0.0;
        motion.post_linear_velocity = [f64::INFINITY, 0.0];
        assert_eq!(
            build_semi_implicit_euler_envelope(identity(), context(), box2(), motion, 0.0,),
            Err(EnvelopeError::NonFiniteArithmetic)
        );
    }

    #[test]
    fn invalid_bounds_intervals_guard_and_identity_fail_closed() {
        assert_eq!(
            Aabb::new([f64::NAN, 0.0], [1.0, 1.0]),
            Err(EnvelopeError::InvalidBounds)
        );
        let reversed = DisplacementInterval {
            min: [1.0, 0.0],
            max: [0.0, 1.0],
        };
        assert_eq!(
            build_envelope(
                identity(),
                context(),
                MotionKind::FixedOrientationTranslation,
                box2(),
                reversed,
                0.0,
            ),
            Err(EnvelopeError::InvalidDisplacementInterval)
        );
        assert_eq!(
            build_envelope(
                identity(),
                context(),
                MotionKind::FixedOrientationTranslation,
                box2(),
                interval2(),
                f64::NAN,
            ),
            Err(EnvelopeError::InvalidGuard)
        );
        let mut invalid_identity = identity();
        invalid_identity.body_generation = 0;
        assert_eq!(
            build_envelope(
                invalid_identity,
                context(),
                MotionKind::FixedOrientationTranslation,
                box2(),
                interval2(),
                0.0,
            ),
            Err(EnvelopeError::InvalidIdentity)
        );
    }

    #[test]
    fn outward_rounding_overflow_is_not_certified() {
        let tight = Aabb::new([f64::MAX * 0.5], [f64::MAX * 0.75]).unwrap();
        let interval = DisplacementInterval {
            min: [0.0],
            max: [f64::MAX],
        };
        assert_eq!(
            build_envelope(
                identity(),
                context(),
                MotionKind::FixedOrientationTranslation,
                tight,
                interval,
                0.0,
            ),
            Err(EnvelopeError::NonFiniteArithmetic)
        );
    }

    #[test]
    fn guard_expands_all_axes_outward_and_is_replay_bound() {
        let tight = Aabb::new([1.0, 1.0], [2.0, 2.0]).unwrap();
        let stationary = DisplacementInterval {
            min: [0.0; 2],
            max: [0.0; 2],
        };
        let result = build_envelope(
            identity(),
            context(),
            MotionKind::FixedOrientationTranslation,
            tight,
            stationary,
            0.5,
        )
        .unwrap();
        let envelope = result.envelope.unwrap();
        assert!(envelope.min()[0] < 0.5);
        assert!(envelope.min()[1] < 0.5);
        assert!(envelope.max()[0] > 2.5);
        assert!(envelope.max()[1] > 2.5);
    }
}
