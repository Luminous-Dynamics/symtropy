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

/// Count of NaN/Inf values silently zeroed during integration.
///
/// Incremented in both debug and release builds whenever a non-finite
/// velocity or position is detected and zeroed. Monitor this counter to
/// detect silent data corruption — a non-zero value always indicates a
/// physics or force accumulator bug upstream.
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
/// If a body carries an authoritative inertia operator and its rotational
/// solve is singular or invalid, this returns the error before mutating the
/// body's pose or velocities. The caller may decide whether to retry, reject
/// the frame, or quarantine the body.
pub fn try_integrate<const D: usize>(
    body: &mut RigidBody<D>,
    gravity: &SVector<f64, D>,
    dt: f64,
) -> Result<(), crate::inertia::RotationalInertiaError> {
    if !body.is_dynamic() {
        body.clear_accumulators();
        return Ok(());
    }

    // Sanitize into local values first. This keeps the body's state and
    // accumulators untouched if the validated inertia solve below fails.
    let force_accumulator = if body.force_accumulator.iter().all(|v| v.is_finite()) {
        body.force_accumulator.clone()
    } else {
        SVector::zeros()
    };
    let torque_accumulator = if body.torque_accumulator.is_finite() {
        body.torque_accumulator.clone()
    } else {
        Bivector::zero()
    };

    // --- Angular dynamics ---

    // Bodies carrying validated mass properties use the full bivector
    // inertia operator below. Only legacy bodies without an operator fall
    // back to the isotropic mean-of-axis approximation.
    let ang_accel = if let Some(inertia) = &body.rotational_inertia {
        if let Err(error) = inertia.validate_body_mass(body.mass, 1e-10) {
            ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
            return Err(error);
        }
        let world_inertia = inertia.rotated_by(&body.transform.rotation);
        match world_inertia.angular_acceleration(&body.angular_velocity, &torque_accumulator, 1e-12)
        {
            Ok(acceleration) => acceleration,
            Err(error) => {
                ROTATIONAL_INERTIA_FAILURE_COUNT.fetch_add(1, Ordering::Relaxed);
                return Err(error);
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

    // Semi-implicit: velocity first
    body.linear_velocity += accel * dt;

    // LTC-based damping: v(t+dt) = v(t) × exp(-dt/τ) where τ = 1/damping.
    // Frame-rate independent, never reverses velocity (exp > 0), exact for any dt.
    if body.linear_damping > 1e-15 {
        let tau = 1.0 / body.linear_damping;
        body.linear_velocity *= (-dt / tau).exp();
    }

    // Clamp velocity to prevent explosion
    let speed = body.linear_velocity.norm();
    if speed > 1000.0 {
        body.linear_velocity *= 1000.0 / speed;
    }

    // Update position
    let displacement = body.linear_velocity * dt;
    body.transform.translation = Point(body.transform.translation.0 + displacement);

    // Update angular velocity
    body.angular_velocity = body.angular_velocity.add(&ang_accel.scale(dt));

    // LTC angular damping (same closed-form)
    if body.angular_damping > 1e-15 {
        let ang_tau = 1.0 / body.angular_damping;
        body.angular_velocity = body.angular_velocity.scale((-dt / ang_tau).exp());
    }

    // Clamp angular velocity
    let ang_speed = body.angular_velocity.norm();
    if ang_speed > 100.0 {
        body.angular_velocity = body.angular_velocity.scale(100.0 / ang_speed);
    }

    // Integrate rotation through the SO(D) exponential map. The bivector
    // components are angular rates, so scaling by dt produces the generator's
    // angular displacement directly. This remains correct when angular motion
    // occupies several independent planes at once (e.g. a 4D double rotation).
    if body.angular_velocity.norm_squared() > 1e-24 {
        let delta_generator = body.angular_velocity.scale(dt);
        let delta_rotation = Rotor::from_bivector(&delta_generator);
        body.transform.rotation = delta_rotation.compose(&body.transform.rotation);
    }

    // --- Post-integration NaN/Inf assertions ---
    // If any state became non-finite despite clamping, zero the offending values
    // rather than propagating corruption through the simulation.
    if !body.linear_velocity.iter().all(|v| v.is_finite()) {
        debug_assert!(
            false,
            "NaN/Inf detected in linear_velocity after integration"
        );
        NAN_ZEROED_COUNT.fetch_add(1, Ordering::Relaxed);
        body.linear_velocity = SVector::zeros();
    }
    if !body.angular_velocity.is_finite() {
        debug_assert!(
            false,
            "NaN/Inf detected in angular_velocity after integration"
        );
        NAN_ZEROED_COUNT.fetch_add(1, Ordering::Relaxed);
        body.angular_velocity = Bivector::zero();
    }
    if !body.transform.translation.0.iter().all(|v| v.is_finite()) {
        debug_assert!(false, "NaN/Inf detected in position after integration");
        // Don't zero position — that would teleport. Just freeze velocity.
        NAN_ZEROED_COUNT.fetch_add(1, Ordering::Relaxed);
        body.linear_velocity = SVector::zeros();
    }

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
        assert_eq!(error, RotationalInertiaError::SingularInertia);
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
            Err(RotationalInertiaError::MassMismatch)
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
