// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Analytic qualification tests for the semi-implicit Euler update.
//!
//! These tests pin the discrete method separately from contact, damping,
//! angular inertia and solver behavior. They do not qualify the full physics
//! world or imply stability for arbitrary Hamiltonian systems.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::integrator::integrate;
use symtropy_physics::{BodyHandle, RigidBody};

#[test]
fn constant_acceleration_matches_the_exact_discrete_update() {
    let mass = 2.0;
    let dt = 0.005;
    let steps = 37usize;
    let force = SVector::from([4.0, -6.0, 2.0]);
    let gravity = SVector::<f64, 3>::zeros();

    let mut body =
        RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([0.3, -0.2, 0.7]), 0.1, mass);
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;
    body.linear_velocity = SVector::from([0.2, -0.3, 0.4]);

    let x0 = body.transform.translation.0;
    let v0 = body.linear_velocity;
    let acceleration = force / mass;

    for _ in 0..steps {
        body.apply_force(force);
        integrate(&mut body, &gravity, dt);
    }

    let n = steps as f64;
    let elapsed = n * dt;
    let expected_velocity = v0 + acceleration * elapsed;
    let expected_position =
        x0 + v0 * elapsed + acceleration * (0.5 * dt * dt * n * (n + 1.0));

    assert!(
        (body.linear_velocity - expected_velocity).norm() < 1.0e-12,
        "velocity {:?} differs from discrete reference {:?}",
        body.linear_velocity,
        expected_velocity
    );
    assert!(
        (body.transform.translation.0 - expected_position).norm() < 1.0e-12,
        "position {:?} differs from discrete reference {:?}",
        body.transform.translation.0,
        expected_position
    );
}

fn free_fall_position_error(dt: f64) -> f64 {
    let total_time = 1.0;
    let steps = (total_time / dt).round() as usize;
    let elapsed = steps as f64 * dt;
    let gravity = SVector::from([0.0, -9.81, 0.0]);
    let initial_position = SVector::from([0.1, 2.0, -0.2]);
    let initial_velocity = SVector::from([0.4, -0.3, 0.1]);

    let mut body = RigidBody::<3>::dynamic_sphere(
        BodyHandle(2),
        Point::new([initial_position[0], initial_position[1], initial_position[2]]),
        0.1,
        1.0,
    );
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;
    body.linear_velocity = initial_velocity;

    for _ in 0..steps {
        integrate(&mut body, &gravity, dt);
    }

    let exact_position =
        initial_position + initial_velocity * elapsed + gravity * (0.5 * elapsed * elapsed);
    (body.transform.translation.0 - exact_position).norm()
}

#[test]
fn free_fall_position_error_has_first_order_global_convergence() {
    let coarse_error = free_fall_position_error(0.02);
    let fine_error = free_fall_position_error(0.01);
    let observed_ratio = coarse_error / fine_error;

    assert!(
        (observed_ratio - 2.0).abs() < 1.0e-10,
        "halving dt should halve the global position error for constant acceleration;          coarse={coarse_error:e}, fine={fine_error:e}, ratio={observed_ratio}"
    );
}

fn harmonic_oscillator_state(dt: f64, steps: usize) -> (f64, f64, f64) {
    let omega: f64 = 1.0;
    let gravity = SVector::<f64, 2>::zeros();
    let mut body = RigidBody::<2>::dynamic_sphere(BodyHandle(3), Point::origin(), 0.1, 1.0);
    body.linear_damping = 0.0;
    body.angular_damping = 0.0;
    body.transform.translation.0[0] = 1.0;

    let mut maximum_abs_position: f64 = 0.0;
    for _ in 0..steps {
        let position = body.transform.translation.0;
        body.apply_force(-position * (omega * omega));
        integrate(&mut body, &gravity, dt);
        maximum_abs_position = maximum_abs_position.max(body.transform.translation.0[0].abs());
    }

    (
        body.transform.translation.0[0],
        body.linear_velocity[0],
        maximum_abs_position,
    )
}

#[test]
fn harmonic_oscillator_exposes_the_semi_implicit_euler_timestep_boundary() {
    // For x'' + omega^2 x = 0, the linear update is stable only for
    // 0 < omega * dt < 2. These are tests of this oscillator, not a universal
    // stability theorem for Hamiltonian systems.
    let stable_dt = 1.9;
    let (stable_x, stable_v, stable_max_x) = harmonic_oscillator_state(stable_dt, 300);
    assert!(stable_x.is_finite() && stable_v.is_finite());
    assert!(
        stable_max_x < 10.0,
        "state grew outside the stable oscillator envelope: max |x|={stable_max_x}"
    );

    let unstable_dt = 2.1;
    let (unstable_x, unstable_v, _) = harmonic_oscillator_state(unstable_dt, 7);
    assert!(unstable_x.is_finite() && unstable_v.is_finite());
    assert!(
        unstable_x.abs() > 150.0,
        "dt above the oscillator stability boundary did not expose growth: x={unstable_x}"
    );
}
