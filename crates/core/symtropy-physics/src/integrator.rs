// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Semi-implicit Euler integrator for ND rigid body dynamics.
//!
//! Stored angular velocity is a bivector (element of `so(D)`) integrated
//! through the rotation-group exponential map. The convention is paired with
//! `Rotor::from_bivector(B) = exp(-B)`: a material point at world offset `r`
//! therefore has physical angular velocity `-B*r`. Contact, friction and
//! restitution must use `angular_point_velocity`, while off-centre impulses
//! use `angular_impulse_from_point`. This is exact for simultaneous
//! independent rotation planes (including 4D double rotations); it does not
//! rely on the single-plane Rodrigues formula.

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

/// Return the total number of NaN/Inf zeroing events since program start.
#[inline]
pub fn nan_zeroed_count() -> u64 {
    NAN_ZEROED_COUNT.load(Ordering::Relaxed)
}

/**
 * Physical material-point velocity induced by the stored world-space
 * angular-velocity bivector.
 *
 * Rotor::from_bivector(B) advances orientation by exp(-B). Therefore the
 * derivative of a rotated world-space offset is -B * offset, not +B * offset.
 * This helper is the one contact, friction and restitution code must use.
 */
#[inline]
pub fn angular_point_velocity<const D: usize>(
    angular_velocity: &Bivector<D>,
    offset_world: &SVector<f64, D>,
) -> SVector<f64, D> {
    -angular_velocity.apply_to_vector(offset_world)
}

/**
 * Encode the physical angular impulse from an impulse applied at a world-space
 * offset, using the stored-bivector sign convention paired with Rotor's exp(-B).
 *
 * The resulting bivector is offset ∧ impulse. For D=3 this corresponds to the
 * conventional torque impulse r × j under Symtropy's e01/e02/e12 mapping.
 */
#[inline]
pub fn angular_impulse_from_point<const D: usize>(
    offset_world: &SVector<f64, D>,
    impulse_world: &SVector<f64, D>,
) -> Bivector<D> {
    Bivector::from_wedge(offset_world, impulse_world)
}

/// Integrate a single rigid body forward by dt seconds.
///
/// Semi-implicit Euler: update velocity first, then position.
/// This is unconditionally stable for Hamiltonian systems (symplectic).
pub fn integrate<const D: usize>(body: &mut RigidBody<D>, gravity: &SVector<f64, D>, dt: f64) {
    if !body.is_dynamic() {
        body.clear_accumulators();
        return;
    }

    // --- NaN/Inf guards on inputs ---
    // Sanitize force/torque accumulators: replace non-finite values with zero
    // to prevent NaN propagation from bad external forces.
    if !body.force_accumulator.iter().all(|v| v.is_finite()) {
        body.force_accumulator = SVector::zeros();
    }
    if !body.torque_accumulator.is_finite() {
        body.torque_accumulator = Bivector::zero();
    }

    // --- Linear dynamics ---

    // Acceleration = F/m + gravity
    let accel = body.force_accumulator * body.inv_mass + gravity;

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

    // --- Angular dynamics ---

    // Apply torque through the body's selected, explicitly reported inertia
    // model. In 3D this includes frame-aware inertia and Euler gyroscopic
    // coupling; a validated full bivector operator supports general D.
    let angular_acceleration =
        body.angular_acceleration_from_torque(&body.torque_accumulator);

    // Update angular velocity
    body.angular_velocity = body
        .angular_velocity
        .add(&angular_acceleration.scale(dt));

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
}

/// Apply a position correction to separate overlapping bodies.
pub fn separate<const D: usize>(body: &mut RigidBody<D>, correction: &SVector<f64, D>) {
    if body.is_dynamic() {
        body.transform.translation = Point(body.transform.translation.0 + correction);
    }
}

/// Apply an impulse to a body at its center of mass.
pub fn apply_impulse<const D: usize>(body: &mut RigidBody<D>, impulse: &SVector<f64, D>) {
    if body.is_dynamic() {
        body.linear_velocity += impulse * body.inv_mass;
    }
}

/// Apply an angular impulse (as a bivector) to a body.
/// Apply a world-space angular impulse through the body's declared
/// rotational inertia model.
pub fn apply_angular_impulse<const D: usize>(body: &mut RigidBody<D>, impulse: &Bivector<D>) {
    if body.is_dynamic() {
        let delta_omega = body.apply_inverse_angular_inertia(impulse);
        body.angular_velocity = body.angular_velocity.add(&delta_omega);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;

    fn measured_rotor_point_velocity(
        generator: &Bivector<3>,
        offset: &SVector<f64, 3>,
        dt: f64,
    ) -> SVector<f64, 3> {
        let delta = Rotor::from_bivector(&generator.scale(dt));
        (delta.rotate_vector(offset) - offset) / dt
    }

    #[test]
    fn physical_point_velocity_matches_rotor_finite_difference_for_all_3d_planes() {
        let offset = SVector::from([0.37, -0.61, 0.82]);
        let mut generators = Vec::new();
        for (i, j) in [(0, 1), (0, 2), (1, 2)] {
            let mut generator = Bivector::<3>::zero();
            generator.set(i, j, 0.73);
            generators.push(generator);
        }
        let mut generic = Bivector::<3>::zero();
        generic.set(0, 1, 0.41);
        generic.set(0, 2, -0.63);
        generic.set(1, 2, 0.29);
        generators.push(generic);

        for (case, generator) in generators.iter().enumerate() {
            let expected = angular_point_velocity(generator, &offset);
            let coarse = measured_rotor_point_velocity(generator, &offset, 1.0e-3);
            let fine = measured_rotor_point_velocity(generator, &offset, 5.0e-4);
            let coarse_error = (coarse - expected).norm();
            let fine_error = (fine - expected).norm();

            assert!(
                fine_error < coarse_error * 0.6,
                "finite-difference error did not converge for case {case}: coarse={coarse_error:e}, fine={fine_error:e}"
            );
            assert!(
                fine_error < 5.0e-4,
                "point-velocity sign/magnitude mismatch for case {case}: error={fine_error:e}"
            );
        }
    }

    #[test]
    fn angular_point_velocity_and_impulse_are_rotation_covariant() {
        let mut angular_velocity = Bivector::<3>::zero();
        angular_velocity.set(0, 1, 0.41);
        angular_velocity.set(0, 2, -0.63);
        angular_velocity.set(1, 2, 0.29);
        let offset = SVector::from([0.7, -0.2, 1.1]);
        let impulse = SVector::from([-0.4, 0.9, 0.3]);

        let mut frame_generator = Bivector::<3>::zero();
        frame_generator.set(0, 1, -0.37);
        frame_generator.set(0, 2, 0.58);
        frame_generator.set(1, 2, 0.22);
        let frame_change = Rotor::<3>::from_bivector(&frame_generator);
        let rotation = *frame_change.to_matrix();

        let original_point_velocity = angular_point_velocity(&angular_velocity, &offset);
        let rotated_angular_matrix = rotation * angular_velocity.to_matrix() * rotation.transpose();
        let rotated_angular_velocity = Bivector::<3>::from_matrix(&rotated_angular_matrix);
        let rotated_offset = frame_change.rotate_vector(&offset);
        let rotated_point_velocity =
            angular_point_velocity(&rotated_angular_velocity, &rotated_offset);
        let expected_rotated_velocity = frame_change.rotate_vector(&original_point_velocity);
        assert!(
            (rotated_point_velocity - expected_rotated_velocity).norm() < 2.0e-12,
            "physical point velocity did not transform covariantly"
        );

        let original_impulse = angular_impulse_from_point(&offset, &impulse);
        let rotated_offset = frame_change.rotate_vector(&offset);
        let rotated_impulse = frame_change.rotate_vector(&impulse);
        let transformed_impulse = angular_impulse_from_point(&rotated_offset, &rotated_impulse);
        let expected_impulse_matrix =
            rotation * original_impulse.to_matrix() * rotation.transpose();
        let expected_impulse = Bivector::<3>::from_matrix(&expected_impulse_matrix);
        assert!(
            transformed_impulse
                .add(&expected_impulse.scale(-1.0))
                .norm()
                < 2.0e-12,
            "off-centre angular impulse did not transform covariantly"
        );
    }

    #[test]
    fn off_centre_impulse_matches_rotor_motion_and_physical_torque_sign() {
        let mut body = RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::origin(), 1.0, 1.0);
        body.angular_damping = 0.0;

        let offset = SVector::from([1.0, 0.0, 0.0]);
        let impulse = SVector::from([0.0, 1.0, 0.0]);
        let angular_impulse = angular_impulse_from_point(&offset, &impulse);

        // r × j is +z, represented by positive e01 in Symtropy's bivector
        // convention. The old j ∧ r ordering produced the opposite sign.
        assert!((angular_impulse.get(0, 1) - 1.0).abs() < 1.0e-14);
        assert!(angular_impulse.get(0, 2).abs() < 1.0e-14);
        assert!(angular_impulse.get(1, 2).abs() < 1.0e-14);

        apply_angular_impulse(&mut body, &angular_impulse);
        assert!(body.angular_velocity.get(0, 1) > 0.0);

        let expected_unit_inertia_response =
            impulse * offset.norm_squared() - offset * offset.dot(&impulse);
        let inverse_inertia = body.inv_inertia[0];
        let predicted_point_velocity = angular_point_velocity(&body.angular_velocity, &offset);
        assert!(
            (predicted_point_velocity - expected_unit_inertia_response * inverse_inertia).norm()
                < 1.0e-12,
            "angular impulse did not produce the expected r × j response"
        );

        // Compare the production integrator's actual Transform::transform_point
        // trajectory with the contact-point velocity prediction.
        let material_point = Point::new([offset[0], offset[1], offset[2]]);
        let before = body.transform.transform_point(&material_point);
        let dt = 1.0e-7;
        integrate(&mut body, &SVector::zeros(), dt);
        let after = body.transform.transform_point(&material_point);
        let measured = (after.0 - before.0) / dt;
        assert!(
            (measured - predicted_point_velocity).norm() < 2.0e-6,
            "off-centre impulse / Rotor / contact-point velocity disagreement: measured={measured:?}, predicted={predicted_point_velocity:?}"
        );
    }

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
}
