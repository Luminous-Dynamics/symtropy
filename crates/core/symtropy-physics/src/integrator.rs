// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Semi-implicit Euler integrator for ND rigid body dynamics.
//!
//! Angular velocity is a bivector (element of `so(D)`) integrated through the
//! rotation-group exponential map. This is exact for simultaneous independent
//! rotation planes (including 4D double rotations); it does not rely on the
//! single-plane Rodrigues formula.

use crate::body::RigidBody;
use nalgebra::SVector;
use std::sync::atomic::{AtomicU64, Ordering};
use symtropy_math::{Bivector, Point, Rotor};

/// Legacy counter for historical NaN/Inf zeroing events. The strict integrator
/// now rejects non-finite inputs/candidates instead of silently zeroing them.
pub static NAN_ZEROED_COUNT: AtomicU64 = AtomicU64::new(0);
/// Count of failures applying validated bivector rotational inertia.
pub static ROTATIONAL_INERTIA_FAILURE_COUNT: AtomicU64 = AtomicU64::new(0);

/// Return the number of singular/invalid rotational-inertia operations.
#[inline]
pub fn rotational_inertia_failure_count() -> u64 {
    ROTATIONAL_INERTIA_FAILURE_COUNT.load(Ordering::Relaxed)
}

/// Return the total number of NaN/Inf zeroing events since program start.
#[inline]
pub fn nan_zeroed_count() -> u64 {
    NAN_ZEROED_COUNT.load(Ordering::Relaxed)
}

/// Errors returned by the strict, transactional rigid-body integrator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationError {
    InvalidTimeStep,
    InvalidParameters,
    NonFiniteInput,
    NonFiniteState,
    RotationalInertia(crate::inertia::RotationalInertiaError),
}

impl From<crate::inertia::RotationalInertiaError> for IntegrationError {
    fn from(error: crate::inertia::RotationalInertiaError) -> Self {
        Self::RotationalInertia(error)
    }
}

/// Count of rejected integration attempts caused by invalid numerical inputs
/// or non-finite candidate state. This is not a substitute for the typed error.
pub static INTEGRATION_FAILURE_COUNT: AtomicU64 = AtomicU64::new(0);

#[inline]
pub fn integration_failure_count() -> u64 {
    INTEGRATION_FAILURE_COUNT.load(Ordering::Relaxed)
}

#[inline]
fn reject_integration(error: IntegrationError) -> IntegrationError {
    INTEGRATION_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
    error
}

/// Explicit speed bounds for real-time integration.
///
/// The scientific/default path uses `unbounded()`: no hidden numeric velocity
/// caps are applied. Applications with a declared operating envelope can opt
/// into linear and angular speed caps in their own unit system.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntegrationLimits {
    max_linear_speed: Option<f64>,
    max_angular_speed: Option<f64>,
}

/// Invalid explicit integration speed bound.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntegrationLimitError {
    NonFiniteOrNonPositive,
}

impl IntegrationLimits {
    /// No artificial speed bounds. This is the default scientific policy.
    pub const fn unbounded() -> Self {
        Self {
            max_linear_speed: None,
            max_angular_speed: None,
        }
    }

    /// Construct explicit speed bounds in the caller's length/time units.
    ///
    /// Either bound may be None. Supplied values must be finite and positive.
    pub fn with_speed_caps(
        max_linear_speed: Option<f64>,
        max_angular_speed: Option<f64>,
    ) -> Result<Self, IntegrationLimitError> {
        if max_linear_speed
            .into_iter()
            .chain(max_angular_speed)
            .any(|value| !value.is_finite() || value <= 0.0)
        {
            return Err(IntegrationLimitError::NonFiniteOrNonPositive);
        }
        Ok(Self {
            max_linear_speed,
            max_angular_speed,
        })
    }

    pub const fn max_linear_speed(&self) -> Option<f64> {
        self.max_linear_speed
    }

    pub const fn max_angular_speed(&self) -> Option<f64> {
        self.max_angular_speed
    }
}

/// Integrate a single rigid body forward by dt seconds.
///
/// Semi-implicit Euler: update velocity first, then position.
/// Symplecticity improves long-term geometric behavior for suitable Hamiltonian systems,
/// but it does not imply unconditional stability: the timestep must still resolve
/// the system's fastest relevant timescale.
pub fn integrate<const D: usize>(body: &mut RigidBody<D>, gravity: &SVector<f64, D>, dt: f64) {
    // Compatibility entry point. On an invalid inertia solve, the attempted
    // step is not committed and its accumulators are discarded.
    if try_integrate(body, gravity, dt).is_err() {
        body.clear_accumulators();
    }
}

/// Transactional integration entry point.
///
/// Invalid timestep/parameters, non-finite input, singular mass properties,
/// and non-finite candidate state are returned as typed errors. Candidate
/// state is committed only after all components validate; on error the body's
/// state and force/torque accumulators remain unchanged.
pub fn try_integrate<const D: usize>(
    body: &mut RigidBody<D>,
    gravity: &SVector<f64, D>,
    dt: f64,
) -> Result<(), IntegrationError> {
    try_integrate_with_limits(body, gravity, dt, &IntegrationLimits::unbounded())
}

/// Transactional integration with explicit optional speed limits.
///
/// Any configured cap is applied deliberately and in the caller's unit
/// system. The unbounded default does not silently erase physical energy.
pub fn try_integrate_with_limits<const D: usize>(
    body: &mut RigidBody<D>,
    gravity: &SVector<f64, D>,
    dt: f64,
    limits: &IntegrationLimits,
) -> Result<(), IntegrationError> {
    if !body.is_dynamic() {
        body.clear_accumulators();
        return Ok(());
    }

    if !dt.is_finite() || dt <= 0.0 {
        return Err(reject_integration(IntegrationError::InvalidTimeStep));
    }
    if body.linear_damping < 0.0
        || body.angular_damping < 0.0
        || !body.linear_damping.is_finite()
        || !body.angular_damping.is_finite()
        || !body.mass.is_finite()
        || body.mass <= 0.0
        || !body.inv_mass.is_finite()
        || body.inv_mass <= 0.0
        || !body.inv_inertia.iter().all(|value| value.is_finite())
    {
        return Err(reject_integration(IntegrationError::InvalidParameters));
    }
    if !gravity.iter().all(|value| value.is_finite())
        || !body.force_accumulator.iter().all(|value| value.is_finite())
        || !body.torque_accumulator.is_finite()
        || !body.linear_velocity.iter().all(|value| value.is_finite())
        || !body.angular_velocity.is_finite()
        || !body.transform.translation.0.iter().all(|value| value.is_finite())
        || !body
            .transform
            .rotation
            .to_matrix()
            .iter()
            .all(|value| value.is_finite())
    {
        return Err(reject_integration(IntegrationError::NonFiniteInput));
    }

    // Invalid input is rejected. Never substitute zero for a force or torque:
    // the caller receives a typed failure and can choose an explicit recovery.
    let force_accumulator = body.force_accumulator;
    let torque_accumulator = body.torque_accumulator.clone();

    // Candidate state is computed locally and committed only after every
    // component has been checked. This makes integration transactional even
    // when a finite input overflows during the numerical update.

    // --- Angular dynamics ---

    // Bodies carrying validated mass properties use the full bivector
    // inertia operator below. Only legacy bodies without an operator fall
    // back to the isotropic mean-of-axis approximation.
    let ang_accel = if let Some(inertia) = &body.rotational_inertia {
        if let Err(error) = inertia.validate_body_mass(body.mass, 1e-10) {
            ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            return Err(IntegrationError::RotationalInertia(error));
        }
        let world_inertia = inertia.rotated_by(&body.transform.rotation);
        match world_inertia.angular_acceleration(&body.angular_velocity, &torque_accumulator, 1e-12)
        {
            Ok(acceleration) => acceleration,
            Err(error) => {
                ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
                return Err(IntegrationError::RotationalInertia(error));
            }
        }
    } else {
        let inv_i_avg = body.inv_inertia.sum() / D as f64;
        // Legacy isotropic fallback: the impulse-first torque accumulator
        // enters the stored B state with a negative sign.
        torque_accumulator.scale(-inv_i_avg)
    };

    // --- Linear dynamics ---

    // Acceleration = F/m + gravity
    let accel = force_accumulator * body.inv_mass + gravity;

    // Semi-implicit: velocity first, but keep all candidate values local.
    let mut linear_velocity = body.linear_velocity + accel * dt;
    let mut angular_velocity = body.angular_velocity.clone();
    let mut position = body.transform.translation.0;
    let mut rotation = body.transform.rotation.clone();

    // LTC-based damping: v(t+dt) = v(t) × exp(-dt/τ) where τ = 1/damping.
    // Frame-rate independent, never reverses velocity (exp > 0), exact for any dt.
    if body.linear_damping > 1e-15 {
        let tau = 1.0 / body.linear_damping;
        linear_velocity *= (-dt / tau).exp();
    }

    // Optional application-defined limit. The default scientific policy
    // has no hidden absolute cap, preserving dimensional similarity.
    if let Some(max_speed) = limits.max_linear_speed {
        let speed = linear_velocity.norm();
        if speed > max_speed {
            linear_velocity *= max_speed / speed;
        }
    }

    // Update position
    let displacement = linear_velocity * dt;
    position += displacement;

    // Update angular velocity
    angular_velocity = angular_velocity.add(&ang_accel.scale(dt));

    // LTC angular damping (same closed-form)
    if body.angular_damping > 1e-15 {
        let ang_tau = 1.0 / body.angular_damping;
        angular_velocity = angular_velocity.scale((-dt / ang_tau).exp());
    }

    // Optional application-defined angular-rate limit (radians/second).
    if let Some(max_speed) = limits.max_angular_speed {
        let ang_speed = angular_velocity.norm();
        if ang_speed > max_speed {
            angular_velocity = angular_velocity.scale(max_speed / ang_speed);
        }
    }

    // Integrate rotation through the SO(D) exponential map. The bivector
    // components are angular rates, so scaling by dt produces the generator's
    // angular displacement directly. This remains correct when angular motion
    // occupies several independent planes at once (e.g. a 4D double rotation).
    if angular_velocity.norm_squared() > 0.0 {
        let delta_generator = angular_velocity.scale(dt);
        let delta_rotation = Rotor::from_bivector(&delta_generator);
        rotation = delta_rotation.compose(&rotation);
    }

    // Reject non-finite candidate state without mutating the body or
    // clearing its force/torque accumulators. The world can roll back the
    // complete step and the caller can inspect the offending input.
    if !linear_velocity.iter().all(|value| value.is_finite())
        || !angular_velocity.is_finite()
        || !position.iter().all(|value| value.is_finite())
        || !rotation.to_matrix().iter().all(|value| value.is_finite())
    {
        return Err(reject_integration(IntegrationError::NonFiniteState));
    }

    body.linear_velocity = linear_velocity;
    body.angular_velocity = angular_velocity;
    body.transform.translation = Point(position);
    body.transform.rotation = rotation;

    // Clear accumulators for next frame
    body.clear_accumulators();
    Ok(())
}

/// Apply a position correction to separate overlapping bodies.
pub fn separate<const D: usize>(body: &mut RigidBody<D>, correction: &SVector<f64, D>) {
    if body.is_dynamic() {
        body.transform.translation = Point(body.transform.translation.0 + correction);
    }
}

/// Convert stored angular velocity to the world-space velocity at an offset.
///
/// Rotor integration uses exp(-B) and left-composes the incremental rotation,
/// so the instantaneous world-space point velocity is -B*r, not B*r.
/// Keep this sign conversion centralized for contact and friction solvers.
#[inline]
pub fn angular_point_velocity<const D: usize>(
    angular_velocity: &Bivector<D>,
    offset: &SVector<f64, D>,
) -> SVector<f64, D> {
    -angular_velocity.apply_to_vector(offset)
}

/// Apply an impulse to a body at its center of mass.
pub fn apply_impulse<const D: usize>(body: &mut RigidBody<D>, impulse: &SVector<f64, D>) {
    if body.is_dynamic() {
        body.linear_velocity += impulse * body.inv_mass;
    }
}

/// Rotational contribution to the inverse effective mass at a contact.
///
/// Compatibility helper using authoritative world-frame inertia when present.
/// Singular/invalid operator failures map to zero and are counted; new solver
/// paths must use the fallible `try_rotational_inverse_effective_mass` API.
pub fn rotational_inverse_effective_mass<const D: usize>(
    body: &RigidBody<D>,
    offset: &SVector<f64, D>,
    impulse_direction: &SVector<f64, D>,
) -> f64 {
    try_rotational_inverse_effective_mass(body, offset, impulse_direction).unwrap_or(0.0)
}

/// Fallible rotational contribution to inverse effective mass.
///
/// Unlike the compatibility helper, this distinguishes a valid zero response
/// (for example, a static body) from a singular inertia operation.
pub fn try_rotational_inverse_effective_mass<const D: usize>(
    body: &RigidBody<D>,
    offset: &SVector<f64, D>,
    impulse_direction: &SVector<f64, D>,
) -> Result<f64, crate::inertia::RotationalInertiaError> {
    if !body.is_dynamic() {
        return Ok(0.0);
    }
    let direction_norm = impulse_direction.norm();
    if !direction_norm.is_finite() || direction_norm <= 1e-15 {
        return Err(crate::inertia::RotationalInertiaError::InvalidDirection);
    }
    if let Some(inertia) = &body.rotational_inertia {
        if let Err(error) = inertia.validate_body_mass(body.mass, 1e-10) {
            ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
        let world_inertia = inertia.rotated_by(&body.transform.rotation);
        match world_inertia.rotational_inverse_effective_mass(offset, impulse_direction, 1e-12) {
            Ok(value) => Ok(value),
            Err(error) => {
                ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
                Err(error)
            }
        }
    } else {
        let direction = impulse_direction / direction_norm;
        let inv_i_avg = body.inv_inertia.sum() / D as f64;
        Ok(inv_i_avg * (offset.norm_squared() - offset.dot(&direction).powi(2)).max(0.0))
    }
}

/// Apply an angular impulse in the impulse-first bivector convention.
///
/// The stored angular state advances through Rotor::from_bivector's exp(-B)
/// map, so a positive physical angular impulse subtracts from the stored B.
/// This compatibility wrapper intentionally preserves the legacy void API;
/// callers requiring an error contract should use `try_apply_angular_impulse`.
pub fn apply_angular_impulse<const D: usize>(body: &mut RigidBody<D>, impulse: &Bivector<D>) {
    let _ = try_apply_angular_impulse(body, impulse);
}

/// Fallible angular-impulse application. On a singular/invalid authoritative
/// inertia solve, the body's angular velocity is left unchanged and the error
/// is returned to the caller.
pub fn try_apply_angular_impulse<const D: usize>(
    body: &mut RigidBody<D>,
    impulse: &Bivector<D>,
) -> Result<(), crate::inertia::RotationalInertiaError> {
    if !body.is_dynamic() {
        return Ok(());
    }
    if let Some(inertia) = &body.rotational_inertia {
        if let Err(error) = inertia.validate_body_mass(body.mass, 1e-10) {
            ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
        let world_inertia = inertia.rotated_by(&body.transform.rotation);
        match world_inertia.angular_velocity_from_momentum(impulse, 1e-12) {
            Ok(delta_state) => {
                body.angular_velocity = body.angular_velocity.add(&delta_state.scale(-1.0));
                Ok(())
            }
            Err(error) => {
                ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
                Err(error)
            }
        }
    } else {
        let inv_i_avg = body.inv_inertia.sum() / D as f64;
        body.angular_velocity = body.angular_velocity.add(&impulse.scale(-inv_i_avg));
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;

    #[test]
    fn free_fall_3d() {
        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        let gravity = SVector::from([0.0, -9.81, 0.0]);
        let dt = 0.01;

        // Simulate 1 second of free fall (100 steps)
        for _ in 0..100 {
            integrate(&mut body, &gravity, dt);
        }

        // After 1s: y ≈ -0.5 * 9.81 * 1² ≈ -4.905 (with damping, less)
        let y = body.transform.translation.coord(1);
        assert!(y < -3.0, "y = {y}, expected < -3.0");
        assert!(y > -6.0, "y = {y}, expected > -6.0 (damping bounds)");
    }

    #[test]
    fn stationary_without_forces() {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([5.0, 5.0, 5.0]), 1.0, 1.0);
        let zero_gravity = SVector::zeros();
        let dt = 0.01;

        for _ in 0..100 {
            integrate(&mut body, &zero_gravity, dt);
        }

        // Should barely move (damping causes slight drift toward zero vel)
        assert!((body.transform.translation.coord(0) - 5.0).abs() < 0.1);
    }

    #[test]
    fn applied_force_accelerates() {
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(0),
            Point::origin(),
            1.0,
            2.0, // 2 kg
        );
        let zero_gravity = SVector::zeros();
        let dt = 0.01;

        // Apply 10N in x direction for 1 second
        for _ in 0..100 {
            body.apply_force(SVector::from([10.0, 0.0, 0.0]));
            integrate(&mut body, &zero_gravity, dt);
        }

        // v ≈ F/m * t = 10/2 * 1 = 5 m/s (minus damping)
        let vx = body.linear_velocity[0];
        assert!(vx > 3.0, "vx = {vx}, expected > 3.0");
        assert!(vx < 6.0, "vx = {vx}, expected < 6.0");
    }

    #[test]
    fn angular_velocity_rotates_body() {
        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        // Set angular velocity in xy plane
        body.angular_velocity = Bivector::<3>::unit_plane(0, 1).scale(1.0);

        let zero_gravity = SVector::zeros();
        let dt = 0.01;

        for _ in 0..100 {
            integrate(&mut body, &zero_gravity, dt);
        }

        // Body should have rotated — rotation matrix should not be identity
        let mat = body.transform.rotation.to_matrix();
        let trace = mat.trace();
        // Identity has trace = 3, rotated will have trace < 3
        assert!(trace < 2.99, "trace = {trace}, body didn't rotate");
    }

    #[test]
    fn anisotropic_torque_free_integration_preserves_world_momentum_and_energy() {
        use crate::inertia::BivectorInertia;

        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        body.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap(),
        );
        body.angular_damping = 0.0;
        body.angular_velocity.set(0, 1, 0.7);
        body.angular_velocity.set(0, 2, -0.4);
        body.angular_velocity.set(1, 2, 1.1);

        let zero_gravity = SVector::<f64, 3>::zeros();
        let initial_world_inertia = body
            .rotational_inertia
            .as_ref()
            .unwrap()
            .rotated_by(&body.transform.rotation);
        let initial_momentum = initial_world_inertia
            .angular_momentum(&body.angular_velocity)
            .to_matrix();
        let initial_energy = body.kinetic_energy();

        for _ in 0..100 {
            integrate(&mut body, &zero_gravity, 1e-4);
        }

        let final_world_inertia = body
            .rotational_inertia
            .as_ref()
            .unwrap()
            .rotated_by(&body.transform.rotation);
        let final_momentum = final_world_inertia
            .angular_momentum(&body.angular_velocity)
            .to_matrix();
        let momentum_relative_error =
            (final_momentum - initial_momentum).norm() / initial_momentum.norm();
        let energy_relative_error =
            (body.kinetic_energy() - initial_energy).abs() / initial_energy.abs();

        assert!(
            momentum_relative_error < 2e-6,
            "torque-free world angular momentum drifted by {momentum_relative_error:e}"
        );
        assert!(
            energy_relative_error < 2e-6,
            "torque-free rotational energy drifted by {energy_relative_error:e}"
        );
    }

    #[test]
    fn applied_torque_rotates_offset_in_impulse_consistent_direction() {
        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;

        let offset = SVector::from([1.0, 0.0, 0.0]);
        let impulse = SVector::from([0.0, 1.0, 0.0]);
        let torque_accumulator = Bivector::from_wedge(&impulse, &offset);
        body.apply_torque(torque_accumulator);
        integrate(&mut body, &SVector::zeros(), 1e-4);

        let point_velocity = angular_point_velocity(&body.angular_velocity, &offset);
        assert!(
            point_velocity[1] > 0.0,
            "positive tangential impulse should induce positive point velocity, got {point_velocity:?}"
        );
    }

    #[test]
    fn angular_point_velocity_matches_rotor_finite_difference() {
        let omega = Bivector::<3>::unit_plane(0, 1).scale(0.7);
        let offset = SVector::from([1.2, -0.4, 0.3]);
        let dt = 1e-7;

        let delta = Rotor::from_bivector(&omega.scale(dt));
        let rotated = delta.rotate_point(&Point(offset)).0;
        let finite_difference = (rotated - offset) / dt;
        let predicted = angular_point_velocity(&omega, &offset);

        assert!(
            (finite_difference - predicted).norm() < 1e-7,
            "finite_difference={finite_difference:?}, predicted={predicted:?}"
        );
    }

    #[test]
    fn asymmetric_contact_effective_mass_matches_applied_angular_impulse() {
        use crate::inertia::BivectorInertia;

        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        body.set_rotational_inertia_operator(
            BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(2.0, 1.0, 0.5))
                .unwrap(),
        );
        body.transform.rotation = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 1), 0.43);

        let offset = SVector::new(0.4, -0.7, 0.9);
        let direction = SVector::new(0.3, 0.8, -0.2).normalize();
        let impulse_magnitude = 0.37;
        let effective_mass = rotational_inverse_effective_mass(&body, &offset, &direction);
        let before = body.angular_velocity;

        let impulse = direction * impulse_magnitude;
        let angular_impulse = Bivector::from_wedge(&impulse, &offset);
        apply_angular_impulse(&mut body, &angular_impulse);
        let angular_state_delta = body.angular_velocity.add(&before.scale(-1.0));
        let point_velocity_delta = angular_point_velocity(&angular_state_delta, &offset);
        let measured_inverse_mass = direction.dot(&point_velocity_delta) / impulse_magnitude;

        assert!(
            (measured_inverse_mass - effective_mass).abs() < 1e-11 * effective_mass.abs().max(1.0),
            "effective mass={effective_mass:e}, measured={measured_inverse_mass:e}"
        );
    }

    #[test]
    fn angular_impulse_produces_point_velocity_in_impulse_direction() {
        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        let offset = SVector::from([1.0, 0.0, 0.0]);
        let impulse = SVector::from([0.0, 1.0, 0.0]);
        let torque = Bivector::from_wedge(&impulse, &offset);

        apply_angular_impulse(&mut body, &torque);
        let rotational_delta_velocity = angular_point_velocity(&body.angular_velocity, &offset);
        let inverse_inertia = body.inv_inertia.sum() / 3.0;
        let expected =
            (impulse * offset.norm_squared() - offset * offset.dot(&impulse)) * inverse_inertia;

        assert!(
            (rotational_delta_velocity - expected).norm() < 1e-12,
            "rotational_delta_velocity={rotational_delta_velocity:?}, expected={expected:?}"
        );
    }

    #[test]
    fn impulse_changes_velocity() {
        let mut body = RigidBody::<4>::dynamic_sphere(
            BodyHandle(0),
            Point::origin(),
            1.0,
            5.0, // 5 kg
        );
        apply_impulse(&mut body, &SVector::from([50.0, 0.0, 0.0, 0.0]));
        // Δv = impulse / mass = 50/5 = 10
        assert!((body.linear_velocity[0] - 10.0).abs() < 1e-10);
    }

    #[test]
    fn energy_conservation_no_damping() {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 100.0, 0.0]), 1.0, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;

        let gravity = SVector::from([0.0, -9.81, 0.0]);
        let dt = 0.001; // Small timestep for accuracy

        let initial_energy =
            body.kinetic_energy() + body.mass * 9.81 * body.transform.translation.coord(1);

        for _ in 0..1000 {
            integrate(&mut body, &gravity, dt);
        }

        let final_energy =
            body.kinetic_energy() + body.mass * 9.81 * body.transform.translation.coord(1);
        let drift = ((final_energy - initial_energy) / initial_energy).abs();
        assert!(drift < 0.01, "energy drift {drift} > 1%");
    }

    #[test]
    fn semi_implicit_euler_free_fall_converges_at_first_order() {
        // For constant acceleration, semi-implicit Euler has a global O(dt)
        // position error. Halving dt should therefore halve the final error.
        let gravity = SVector::from([0.0, -1.0, 0.0]);
        let exact_y = -0.5_f64; // y(T=1) = 1/2 * a * T^2
        let timesteps = [0.1_f64, 0.05, 0.025, 0.0125];
        let mut errors = Vec::with_capacity(timesteps.len());

        for dt in timesteps {
            let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
            let steps = (1.0 / dt).round() as usize;
            for _ in 0..steps {
                integrate(&mut body, &gravity, dt);
            }
            errors.push((body.position()[1] - exact_y).abs());
        }

        for pair in errors.windows(2) {
            let ratio = pair[0] / pair[1];
            assert!(
                (ratio - 2.0).abs() < 1e-8,
                "halving dt should halve first-order error; observed ratio {ratio}, errors={errors:?}"
            );
        }
    }

    #[test]
    fn semi_implicit_euler_has_a_timestep_stability_limit() {
        // Unit-mass, unit-stiffness oscillator: stability requires dt * omega < 2.
        // Symplecticity does not make this method unconditionally stable.
        let zero_gravity = SVector::<f64, 3>::zeros();

        let mut stable =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([1.0, 0.0, 0.0]), 0.1, 1.0);
        stable.linear_damping = 0.0;
        let stable_dt = 0.25;
        let mut max_energy = 0.0_f64;
        for _ in 0..1_000 {
            let x = stable.position()[0];
            stable.apply_force(SVector::from([-x, 0.0, 0.0]));
            integrate(&mut stable, &zero_gravity, stable_dt);
            let energy =
                0.5 * stable.linear_velocity.norm_squared() + 0.5 * stable.position()[0].powi(2);
            max_energy = max_energy.max(energy);
        }
        assert!(
            max_energy < 0.6,
            "resolved oscillator should have bounded energy, max={max_energy}"
        );

        let mut unstable =
            RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.0, 0.0, 0.0]), 0.1, 1.0);
        unstable.linear_damping = 0.0;
        let unstable_dt = 2.1; // dt * omega > 2
        for _ in 0..8 {
            let x = unstable.position()[0];
            unstable.apply_force(SVector::from([-x, 0.0, 0.0]));
            integrate(&mut unstable, &zero_gravity, unstable_dt);
        }
        assert!(
            unstable.position()[0].abs() > 100.0,
            "unresolved oscillator should expose the explicit stability limit, x={}",
            unstable.position()[0]
        );
    }

    #[test]
    fn static_body_not_affected() {
        use symtropy_math::Sphere;
        let mut body = RigidBody::<3>::static_body(
            BodyHandle(0),
            Point::new([5.0, 5.0, 5.0]),
            Box::new(Sphere::unit()),
        );
        body.apply_force(SVector::from([1000.0, 0.0, 0.0]));
        let gravity = SVector::from([0.0, -100.0, 0.0]);
        integrate(&mut body, &gravity, 1.0);
        assert!((body.transform.translation.coord(0) - 5.0).abs() < 1e-12);
    }

    #[test]
    fn nan_force_is_sanitized() {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([1.0, 2.0, 3.0]), 1.0, 1.0);
        // Apply a NaN force — should be zeroed, not propagated
        body.apply_force(SVector::from([f64::NAN, 0.0, 0.0]));
        let gravity = SVector::zeros();
        integrate(&mut body, &gravity, 0.01);

        // Position and velocity must remain finite
        assert!(
            body.linear_velocity.iter().all(|v| v.is_finite()),
            "NaN force propagated to velocity"
        );
        assert!(
            body.transform.translation.0.iter().all(|v| v.is_finite()),
            "NaN force propagated to position"
        );
    }

    #[test]
    fn inf_force_is_sanitized() {
        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        body.apply_force(SVector::from([f64::INFINITY, 0.0, 0.0]));
        let gravity = SVector::zeros();
        integrate(&mut body, &gravity, 0.01);

        assert!(
            body.linear_velocity.iter().all(|v| v.is_finite()),
            "Inf force propagated to velocity"
        );
    }
    #[test]
    fn integrates_4d_double_rotation_with_exponential_map() {
        let mut body = RigidBody::<4>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.angular_velocity.set(0, 1, 0.7);
        body.angular_velocity.set(2, 3, -1.1);

        let initial_omega = body.angular_velocity;
        let dt = 0.25;
        integrate(&mut body, &SVector::zeros(), dt);

        let expected = Rotor::from_bivector(&initial_omega.scale(dt));
        let actual = body.transform.rotation.to_matrix();
        for i in 0..4 {
            for j in 0..4 {
                assert!(
                    (actual[(i, j)] - expected.to_matrix()[(i, j)]).abs() < 1e-11,
                    "rotation mismatch at ({i}, {j})"
                );
            }
        }
        assert!(body.transform.rotation.is_proper_rotation(1e-10));
    }

    #[test]
    fn long_double_rotation_integration_preserves_so4_invariants() {
        let mut body = RigidBody::<4>::dynamic_sphere(BodyHandle(0), Point::origin(), 1.0, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.angular_velocity.set(0, 1, 0.4);
        body.angular_velocity.set(2, 3, 0.9);

        for _ in 0..10_000 {
            integrate(&mut body, &SVector::zeros(), 0.001);
        }

        assert!(
            body.transform.rotation.is_proper_rotation(1e-9),
            "orthogonality={}, determinant={}",
            body.transform.rotation.orthogonality_error(),
            body.transform.rotation.determinant()
        );
    }

    #[test]
    fn try_integrate_is_transactional_on_singular_rotational_inertia() {
        use crate::inertia::{BivectorInertia, RotationalInertiaError};
        use nalgebra::SMatrix;

        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(7), Point::new([1.0, 2.0, 3.0]), 0.5, 2.0);
        body.linear_velocity = SVector::new(0.5, -0.25, 0.75);
        body.set_rotational_inertia_operator(
            BivectorInertia::<3>::from_second_moment(SMatrix::<f64, 3, 3>::from_diagonal(
                &SVector::new(1.0, 0.0, 0.0),
            ))
            .unwrap(),
        );
        body.apply_force(SVector::new(4.0, -2.0, 1.0));
        body.apply_torque(Bivector::<3>::unit_plane(1, 2));

        let position_before = body.transform.translation.0.clone();
        let linear_before = body.linear_velocity.clone();
        let angular_before = body.angular_velocity.clone();
        let force_before = body.force_accumulator.clone();
        let torque_before = body.torque_accumulator.clone();

        let error = try_integrate(&mut body, &SVector::new(0.0, -9.81, 0.0), 0.01).unwrap_err();
        assert_eq!(
            error,
            IntegrationError::RotationalInertia(RotationalInertiaError::SingularInertia)
        );
        assert_eq!(body.transform.translation.0, position_before);
        assert_eq!(body.linear_velocity, linear_before);
        assert_eq!(body.angular_velocity, angular_before);
        assert_eq!(body.force_accumulator, force_before);
        assert_eq!(body.torque_accumulator, torque_before);

        // The compatibility API rejects the whole step rather than advancing
        // translation while silently dropping the failed angular response.
        integrate(&mut body, &SVector::new(0.0, -9.81, 0.0), 0.01);
        assert_eq!(body.transform.translation.0, position_before);
        assert_eq!(body.linear_velocity, linear_before);
        assert_eq!(body.angular_velocity, angular_before);
        assert_eq!(body.force_accumulator.norm(), 0.0);
        assert_eq!(body.torque_accumulator.norm(), 0.0);
    }

    #[test]
    fn try_rotational_inverse_effective_mass_reports_singular_inertia() {
        use crate::inertia::{BivectorInertia, RotationalInertiaError};
        use nalgebra::SMatrix;

        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(9), Point::origin(), 0.5, 1.0);
        body.set_rotational_inertia_operator(
            BivectorInertia::<3>::from_second_moment(SMatrix::<f64, 3, 3>::from_diagonal(
                &SVector::new(1.0, 0.0, 0.0),
            ))
            .unwrap(),
        );
        let error = try_rotational_inverse_effective_mass(
            &body,
            &SVector::new(0.0, 1.0, 0.0),
            &SVector::new(0.0, 0.0, 1.0),
        )
        .unwrap_err();
        assert_eq!(error, RotationalInertiaError::SingularInertia);
    }

    #[test]
    fn mass_aware_inertia_mismatch_is_rejected_before_body_mutation() {
        use crate::inertia::{BivectorInertia, RotationalInertiaError};

        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(10), Point::new([1.0, 2.0, 3.0]), 0.5, 1.0);
        let original_mass_metadata = body
            .rotational_inertia
            .as_ref()
            .and_then(BivectorInertia::total_mass);
        assert_eq!(original_mass_metadata, Some(1.0));

        let mismatched =
            BivectorInertia::<3>::uniform_solid_ellipsoid(2.0, SVector::new(1.0, 0.8, 0.6))
                .unwrap();
        assert_eq!(
            body.try_set_rotational_inertia_operator(mismatched.clone()),
            Err(RotationalInertiaError::MassMismatch)
        );
        // The validating setter must not replace the previous valid operator.
        assert_eq!(
            body.rotational_inertia
                .as_ref()
                .and_then(BivectorInertia::total_mass),
            Some(1.0)
        );

        // Exercise the legacy setter too: simulation entry points still have
        // to detect mismatched metadata when callers bypass the new setter.
        body.set_rotational_inertia_operator(mismatched);
        body.linear_velocity = SVector::new(0.5, -0.25, 0.75);
        body.apply_force(SVector::new(4.0, -2.0, 1.0));
        body.apply_torque(Bivector::<3>::unit_plane(0, 2));

        let position_before = body.transform.translation.0.clone();
        let rotation_before = body.transform.rotation.clone();
        let linear_before = body.linear_velocity.clone();
        let angular_before = body.angular_velocity.clone();
        let force_before = body.force_accumulator.clone();
        let torque_before = body.torque_accumulator.clone();

        assert_eq!(
            try_integrate(&mut body, &SVector::zeros(), 0.01),
            Err(IntegrationError::RotationalInertia(RotationalInertiaError::MassMismatch))
        );
        assert_eq!(body.transform.translation.0, position_before);
        assert_eq!(
            body.transform.rotation.to_matrix(),
            rotation_before.to_matrix()
        );
        assert_eq!(body.linear_velocity, linear_before);
        assert_eq!(body.angular_velocity, angular_before);
        assert_eq!(body.force_accumulator, force_before);
        assert_eq!(body.torque_accumulator, torque_before);

        assert_eq!(
            try_apply_angular_impulse(&mut body, &Bivector::<3>::unit_plane(0, 1),)
            Err(RotationalInertiaError::MassMismatch)
        );
        assert_eq!(body.angular_velocity, angular_before);

        assert_eq!(
            try_rotational_inverse_effective_mass(
                &body,
                &SVector::new(0.0, 1.0, 0.0),
                &SVector::new(1.0, 0.0, 0.0),
            ),
            Err(RotationalInertiaError::MassMismatch)
        );
    }

    #[test]
    fn strict_integrator_rejects_invalid_dt_without_mutating_body() {
        for dt in [0.0, -0.01, f64::INFINITY, f64::NAN] {
            let mut body =
                RigidBody::<3>::dynamic_sphere(BodyHandle(30), Point::new([1.0, 2.0, 3.0]), 0.5, 1.0);
            body.linear_velocity = SVector::new(0.25, -0.5, 0.75);
            body.apply_force(SVector::new(1.0, 2.0, 3.0));
            let position = body.transform.translation.0;
            let rotation = body.transform.rotation.clone();
            let velocity = body.linear_velocity;
            let force = body.force_accumulator;

            assert_eq!(
                try_integrate(&mut body, &SVector::zeros(), dt),
                Err(IntegrationError::InvalidTimeStep)
            );
            assert_eq!(body.transform.translation.0, position);
            assert_eq!(body.transform.rotation.to_matrix(), rotation.to_matrix());
            assert_eq!(body.linear_velocity, velocity);
            assert_eq!(body.force_accumulator, force);
        }
    }

    #[test]
    fn strict_integrator_rejects_non_finite_inputs_without_zeroing_accumulators() {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(31), Point::origin(), 0.5, 1.0);
        body.force_accumulator = SVector::new(f64::NAN, 1.0, 0.0);
        body.torque_accumulator = Bivector::<3>::unit_plane(0, 1).scale(f64::INFINITY);
        let force_before = body.force_accumulator;
        let torque_before = body.torque_accumulator.clone();
        let position_before = body.transform.translation.0;
        let linear_before = body.linear_velocity;
        let angular_before = body.angular_velocity.clone();

        assert_eq!(
            try_integrate(&mut body, &SVector::zeros(), 0.01),
            Err(IntegrationError::NonFiniteInput)
        );
        assert!(body.force_accumulator[0].is_nan());
        assert!(!body.torque_accumulator.is_finite());
        assert!(body.torque_accumulator.to_matrix()[(0, 1)].is_infinite());
        assert_eq!(body.force_accumulator[1], force_before[1]);
        assert_eq!(body.force_accumulator[2], force_before[2]);
        assert_eq!(body.transform.translation.0, position_before);
        assert_eq!(body.linear_velocity, linear_before);
        assert_eq!(body.angular_velocity, angular_before);
        assert_eq!(body.torque_accumulator, torque_before);
    }

    #[test]
    fn strict_integrator_rejects_non_finite_candidate_without_partial_commit() {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(32), Point::origin(), 0.5, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.linear_velocity = SVector::new(f64::MAX / 2.0, 0.0, 0.0);
        body.apply_force(SVector::new(1.0, 0.0, 0.0));
        let position_before = body.transform.translation.0;
        let rotation_before = body.transform.rotation.clone();
        let velocity_before = body.linear_velocity;
        let force_before = body.force_accumulator;

        assert_eq!(
            try_integrate(&mut body, &SVector::zeros(), 4.0),
            Err(IntegrationError::NonFiniteState)
        );
        assert_eq!(body.transform.translation.0, position_before);
        assert_eq!(
            body.transform.rotation.to_matrix(),
            rotation_before.to_matrix()
        );
        assert_eq!(body.linear_velocity, velocity_before);
        assert_eq!(body.force_accumulator, force_before);
    }

    #[test]
    fn strict_integrator_rejects_non_finite_gravity_before_mutation() {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(33), Point::origin(), 0.5, 1.0);
        let initial_velocity = body.linear_velocity;
        assert_eq!(
            try_integrate(&mut body, &SVector::new(0.0, f64::NAN, 0.0), 0.01),
            Err(IntegrationError::NonFiniteInput)
        );
        assert_eq!(body.linear_velocity, initial_velocity);
    }

    fn dimensional_similarity_state(
        length_scale: f64,
        mass_scale: f64,
        time_scale: f64,
    ) -> (SVector<f64, 3>, SVector<f64, 3>, Bivector<3>, nalgebra::Matrix3<f64>, f64) {
        let mass = 2.0 * mass_scale;
        let extents = SVector::new(
            0.25 * length_scale,
            0.10 * length_scale,
            0.20 * length_scale,
        );
        let point_masses: Vec<_> = (0..8)
            .map(|mask| {
                let point = SVector::new(
                    if mask & 1 == 0 { -extents[0] } else { extents[0] },
                    if mask & 2 == 0 { -extents[1] } else { extents[1] },
                    if mask & 4 == 0 { -extents[2] } else { extents[2] },
                );
                (point, mass / 8.0)
            })
            .collect();
        let inertia = crate::inertia::BivectorInertia::<3>::from_point_masses(point_masses)
            .unwrap();
        let mut body = RigidBody::<3>::dynamic_sphere(
            BodyHandle(22),
            Point(SVector::new(0.1, 0.2, -0.3) * length_scale),
            0.35 * length_scale,
            mass,
        );
        body.set_rotational_inertia_operator(inertia);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.linear_velocity =
            SVector::new(0.4, 0.2, -0.1) * (length_scale / time_scale);
        body.angular_velocity = Bivector::<3>::unit_plane(0, 1)
            .scale(0.7 / time_scale)
            .add(&Bivector::<3>::unit_plane(1, 2).scale(-0.3 / time_scale));

        let gravity = SVector::new(0.0, -9.81, 0.0)
            * (length_scale / time_scale.powi(2));
        let dt = 0.002 * time_scale;
        for _ in 0..100 {
            body.apply_force(
                SVector::new(0.2, -0.3, 0.1)
                    * (mass_scale * length_scale / time_scale.powi(2)),
            );
            body.apply_torque(
                Bivector::<3>::unit_plane(0, 2)
                    .scale(0.05 * mass_scale * length_scale.powi(2) / time_scale.powi(2)),
            );
            try_integrate(&mut body, &gravity, dt).unwrap();
        }

        let normalized_position = body.transform.translation.0 / length_scale;
        let normalized_velocity = body.linear_velocity / (length_scale / time_scale);
        let normalized_angular_velocity = body.angular_velocity.scale(time_scale);
        let normalized_rotation = body.transform.rotation.to_matrix();
        let normalized_energy = body.kinetic_energy()
            / (mass_scale * length_scale.powi(2) / time_scale.powi(2));
        (
            normalized_position,
            normalized_velocity,
            normalized_angular_velocity,
            normalized_rotation,
            normalized_energy,
        )
    }

    #[test]
    fn rigid_body_trajectory_is_invariant_under_coherent_unit_changes() {
        let reference = dimensional_similarity_state(1.0, 1.0, 1.0);
        for (length, mass, time) in [
            (1e-6, 1e-3, 1e-3),
            (1e6, 1e3, 1e3),
            (1e-3, 1e6, 1e-2),
            (1e3, 1e-6, 1e2),
            (1e13, 1.0, 1e13),
        ] {
            let measured = dimensional_similarity_state(length, mass, time);
            assert!(
                (measured.0 - reference.0).norm() < 1e-8,
                "position similarity failed for L={length:e}, M={mass:e}, T={time:e}: {}",
                (measured.0 - reference.0).norm()
            );
            assert!(
                (measured.1 - reference.1).norm() < 1e-8,
                "linear-velocity similarity failed for L={length:e}, M={mass:e}, T={time:e}: {}",
                (measured.1 - reference.1).norm()
            );
            assert!(
                (measured.2 - reference.2).norm() < 1e-8,
                "angular-velocity similarity failed for L={length:e}, M={mass:e}, T={time:e}: {}",
                (measured.2 - reference.2).norm()
            );
            assert!(
                (measured.3 - reference.3).norm() < 1e-8,
                "rotation similarity failed for L={length:e}, M={mass:e}, T={time:e}: {}",
                (measured.3 - reference.3).norm()
            );
            assert!(
                (measured.4 - reference.4).abs() < 1e-8,
                "energy similarity failed for L={length:e}, M={mass:e}, T={time:e}: {}",
                (measured.4 - reference.4).abs()
            );
        }
    }

    #[test]
    fn unbounded_integrator_preserves_speeds_above_historical_caps() {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(20), Point::origin(), 0.5, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.linear_velocity = SVector::new(1500.0, 0.0, 0.0);
        body.angular_velocity = Bivector::<3>::unit_plane(0, 1).scale(120.0);

        try_integrate(&mut body, &SVector::zeros(), 1e-4).unwrap();

        assert!((body.linear_velocity.norm() - 1500.0).abs() < 1e-10);
        assert!((body.angular_velocity.norm() - 120.0).abs() < 1e-10);
        assert!((body.transform.translation.0[0] - 0.15).abs() < 1e-12);
    }

    #[test]
    fn explicit_realtime_speed_caps_are_applied_only_when_configured() {
        let limits = IntegrationLimits::with_speed_caps(Some(25.0), Some(8.0)).unwrap();
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(21), Point::origin(), 0.5, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.linear_velocity = SVector::new(30.0, 0.0, 0.0);
        body.angular_velocity = Bivector::<3>::unit_plane(0, 1).scale(12.0);

        try_integrate_with_limits(&mut body, &SVector::zeros(), 1e-4, &limits).unwrap();

        assert!((body.linear_velocity.norm() - 25.0).abs() < 1e-12);
        assert!((body.angular_velocity.norm() - 8.0).abs() < 1e-12);
    }

    #[test]
    fn integration_limits_reject_non_finite_or_non_positive_caps() {
        assert_eq!(
            IntegrationLimits::with_speed_caps(Some(0.0), None),
            Err(IntegrationLimitError::NonFiniteOrNonPositive)
        );
        assert_eq!(
            IntegrationLimits::with_speed_caps(None, Some(f64::INFINITY)),
            Err(IntegrationLimitError::NonFiniteOrNonPositive)
        );
    }

    #[test]
    fn try_apply_angular_impulse_reports_singular_inertia() {
        use crate::inertia::{BivectorInertia, RotationalInertiaError};
        use nalgebra::SMatrix;

        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(8), Point::origin(), 0.5, 1.0);
        body.set_rotational_inertia_operator(
            BivectorInertia::<3>::from_second_moment(SMatrix::<f64, 3, 3>::from_diagonal(
                &SVector::new(1.0, 0.0, 0.0),
            ))
            .unwrap(),
        );
        let before = body.angular_velocity.clone();
        let error =
            try_apply_angular_impulse(&mut body, &Bivector::<3>::unit_plane(1, 2)).unwrap_err();
        assert_eq!(error, RotationalInertiaError::SingularInertia);
        assert_eq!(body.angular_velocity, before);
    }
}
