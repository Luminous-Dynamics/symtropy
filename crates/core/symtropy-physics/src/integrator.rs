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

/// Return the total number of NaN/Inf zeroing events since program start.
#[inline]
pub fn nan_zeroed_count() -> u64 {
    NAN_ZEROED_COUNT.load(Ordering::Relaxed)
}

/// Integrate a single rigid body forward by dt seconds.
///
/// Semi-implicit Euler: update velocity first, then position.
///
/// For conservative separable Hamiltonian systems the undamped update is
/// symplectic, but symplecticity is not unconditional stability. Stability and
/// error depend on the system and timestep. For the linear harmonic oscillator
/// the linear stability region requires 0 < omega * dt < 2. Damping, contacts,
/// force clamping and finite precision further modify the simulated dynamics.
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

    // Angular acceleration = torque / inertia (scalar approximation)
    // Per-axis inertia: use mean of principal moments for each bivector plane.
    // For spheres (isotropic), this is exact. For asymmetric bodies, it's the
    // average inertia of the two axes spanning each rotation plane.
    //
    // TODO(isotropic-inertia): this scalar mean makes ALL rotation isotropic,
    // which is only physically correct for spheres. A non-spherical body
    // (box, capsule, compound shape) will rotate as if it had a sphere's mass
    // distribution — e.g. a long thin rod will resist tumbling about its long
    // axis exactly as much as about a short axis, which is wrong. Fixing this
    // properly requires per-bivector-plane inertia (a full D×D inertia tensor
    // or at least per-plane scalars derived from the collider's actual mass
    // distribution) plumbed through `RigidBody`'s `inertia`/`inv_inertia`
    // fields (currently `SVector<f64, D>`, one scalar per *axis*, then
    // averaged pairwise here — not one scalar per *bivector plane*, and not
    // tracked as the body rotates, i.e. no world-space inertia tensor
    // update). That's a larger data-layout change to `body.rs` than is safe
    // to make alongside the contact-solver fixes in this change; deferred.
    let inv_i_avg = body.inv_inertia.sum() / D as f64;
    let ang_accel = body.torque_accumulator.scale(inv_i_avg);

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
pub fn apply_angular_impulse<const D: usize>(body: &mut RigidBody<D>, impulse: &Bivector<D>) {
    if body.is_dynamic() {
        // TODO(isotropic-inertia): same scalar-mean-inertia limitation as in
        // `integrate` above — see that TODO for why this is only exact for
        // spheres and what a real fix would require.
        let inv_i_avg = body.inv_inertia.sum() / D as f64;
        body.angular_velocity = body.angular_velocity.add(&impulse.scale(inv_i_avg));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::body::BodyHandle;

    fn run_free_fall(dt: f64, duration: f64) -> RigidBody<3> {
        let steps = (duration / dt).round() as usize;
        assert!((steps as f64 * dt - duration).abs() < 1e-12);
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(90), Point::new([0.0, 10.0, 0.0]), 0.1, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.linear_velocity = SVector::from([0.0, 1.5, 0.0]);
        let gravity = SVector::from([0.0, -9.81, 0.0]);

        for _ in 0..steps {
            integrate(&mut body, &gravity, dt);
        }
        body
    }

    #[test]
    fn free_fall_matches_the_exact_semi_implicit_discrete_recurrence() {
        let dt = 0.01;
        let duration = 1.0;
        let y0 = 10.0;
        let v0 = 1.5;
        let gravity = -9.81;
        let body = run_free_fall(dt, duration);

        // v_n = v_0 + n*g*h
        // y_n = y_0 + n*v_0*h + g*h^2*n*(n+1)/2
        // The extra +g*h*duration/2 is the expected semi-implicit
        // discretization term relative to the continuous trajectory.
        let steps = (duration / dt).round() as f64;
        let expected_velocity = v0 + gravity * duration;
        let expected_position =
            y0 + steps * v0 * dt + gravity * dt * dt * steps * (steps + 1.0) / 2.0;

        assert!(
            (body.linear_velocity[1] - expected_velocity).abs() < 1e-11,
            "velocity={} expected={expected_velocity}",
            body.linear_velocity[1]
        );
        assert!(
            (body.transform.translation.0[1] - expected_position).abs() < 1e-10,
            "position={} expected={expected_position}",
            body.transform.translation.0[1]
        );
    }

    #[test]
    fn free_fall_position_error_has_first_order_timestep_convergence() {
        let duration = 1.0;
        let y0 = 10.0;
        let v0 = 1.5;
        let gravity = -9.81;
        let exact_position = y0 + v0 * duration + 0.5 * gravity * duration * duration;

        let error = |dt| {
            let body = run_free_fall(dt, duration);
            (body.transform.translation.0[1] - exact_position).abs()
        };
        let coarse_error = error(0.1);
        let medium_error = error(0.05);
        let fine_error = error(0.025);

        let coarse_order = (coarse_error / medium_error).log2();
        let medium_order = (medium_error / fine_error).log2();

        assert!(
            (coarse_order - 1.0).abs() < 1e-10,
            "coarse observed order={coarse_order}"
        );
        assert!(
            (medium_order - 1.0).abs() < 1e-10,
            "medium observed order={medium_order}"
        );
    }

    #[test]
    fn undamped_harmonic_oscillator_has_bounded_energy_inside_stability_region() {
        let omega: f64 = 2.0;
        let dt: f64 = 0.05;
        assert!(omega * dt < 2.0);

        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(91), Point::new([1.0, 0.0, 0.0]), 0.1, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        let zero_gravity = SVector::zeros();
        let initial_energy = 0.5 * omega * omega;

        let mut minimum_energy = f64::INFINITY;
        let mut maximum_energy = f64::NEG_INFINITY;
        for _ in 0..2_000 {
            let x = body.transform.translation.0[0];
            body.apply_force(SVector::from([-omega * omega * x, 0.0, 0.0]));
            integrate(&mut body, &zero_gravity, dt);

            let x = body.transform.translation.0[0];
            let v = body.linear_velocity[0];
            let energy = 0.5 * v * v + 0.5 * omega * omega * x * x;
            minimum_energy = minimum_energy.min(energy);
            maximum_energy = maximum_energy.max(energy);
        }

        let maximum_relative_deviation = ((maximum_energy - initial_energy)
            .abs()
            .max((minimum_energy - initial_energy).abs()))
            / initial_energy;
        assert!(
            maximum_relative_deviation < 0.06,
            "energy envelope deviation={maximum_relative_deviation}"
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
