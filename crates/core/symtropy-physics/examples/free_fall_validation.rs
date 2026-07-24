// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Analytical convergence study for semi-implicit Euler under uniform gravity.
//!
//! Run with:
//! `cargo run --release --example free_fall_validation`
//!
//! The output is CSV so it can be committed as experiment evidence or loaded
//! directly into Python, R, Julia, a spreadsheet, or a CI regression gate.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

fn main() {
    println!(
        "dt,steps,duration,position_error,velocity_error,relative_energy_drift,\
max_rotation_orthogonality_error,max_rotation_determinant_error,non_finite_bodies"
    );

    for dt in [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0] {
        run_case(dt, 1.0);
    }
}

fn run_case(dt: f64, requested_duration: f64) {
    let gravity = SVector::from([0.0, -9.81, 0.0]);
    let initial_position = Point::new([0.0, 100.0, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);
    let body = world.add_sphere(initial_position, 0.5, 1.0);
    {
        let body = world.body_mut(body).expect("body inserted above");
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
    }

    let before = world.invariant_snapshot();
    let steps = (requested_duration / dt).round() as usize;
    for _ in 0..steps {
        world.step(dt);
    }
    let duration = steps as f64 * dt;
    let after = world.invariant_snapshot();
    let drift = before.drift_to(&after);
    let body = world.body(body).expect("body remains in world");

    let expected_y = initial_position.coord(1) + 0.5 * gravity[1] * duration * duration;
    let expected_vy = gravity[1] * duration;
    let position_error = (body.transform.translation.coord(1) - expected_y).abs();
    let velocity_error = (body.linear_velocity[1] - expected_vy).abs();

    println!(
        "{dt:.17},{steps},{duration:.17},{position_error:.17},{velocity_error:.17},\
{:.17},{:.17},{:.17},{}",
        drift.relative_energy_drift,
        drift.max_rotation_orthogonality_error,
        drift.max_rotation_determinant_error,
        drift.non_finite_body_count
    );
}
