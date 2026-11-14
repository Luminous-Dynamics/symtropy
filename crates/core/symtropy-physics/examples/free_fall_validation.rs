// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Analytical convergence study for semi-implicit Euler under uniform gravity.
//!
//! Run with:
//! cargo run --release --example free_fall_validation
//!
//! Standard output is machine-readable CSV. The executable returns a non-zero
//! exit status when the discrete trajectory departs from the analytical
//! semi-implicit-Euler solution, the state becomes unhealthy, or the observed
//! continuous-solution position error stops converging at first order.
//!
//! This validates one narrow translation-only case. It is not evidence for
//! collision handling, articulated mechanisms, structural analysis, or
//! general physical validation.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const DISCRETE_POSITION_TOLERANCE: f64 = 1e-8;
const VELOCITY_TOLERANCE: f64 = 1e-9;
const ROTATION_TOLERANCE: f64 = 1e-10;
const CONVERGENCE_RATIO_MIN: f64 = 1.8;
const CONVERGENCE_RATIO_MAX: f64 = 2.2;

struct CaseResult {
    dt: f64,
    steps: usize,
    duration: f64,
    continuous_position_error: f64,
    discrete_position_error: f64,
    velocity_error: f64,
    relative_energy_drift: f64,
    max_rotation_orthogonality_error: f64,
    max_rotation_determinant_error: f64,
    non_finite_bodies: usize,
    pass: bool,
}

fn main() {
    let mut failures = 0usize;
    let mut results = Vec::new();

    for dt in [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0] {
        results.push(run_case(dt, 1.0));
    }

    println!(
        "dt,steps,duration,continuous_position_error,discrete_position_error,velocity_error,relative_energy_drift,max_rotation_orthogonality_error,max_rotation_determinant_error,non_finite_bodies,pass"
    );

    for result in &results {
        println!(
            "{:.17},{},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{},{}",
            result.dt,
            result.steps,
            result.duration,
            result.continuous_position_error,
            result.discrete_position_error,
            result.velocity_error,
            result.relative_energy_drift,
            result.max_rotation_orthogonality_error,
            result.max_rotation_determinant_error,
            result.non_finite_bodies,
            result.pass
        );
        failures += usize::from(!result.pass);
    }

    // Semi-implicit Euler is first-order accurate in position for this
    // constant-acceleration case. Halving dt should approximately halve the
    // error against the continuous solution; it must also match the exact
    // discrete trajectory to floating-point tolerance.
    eprintln!("check,coarse_dt,fine_dt,error_ratio,pass");
    for pair in results.windows(2) {
        let coarse = &pair[0];
        let fine = &pair[1];
        let ratio = coarse.continuous_position_error / fine.continuous_position_error;
        let pass =
            ratio.is_finite() && ratio >= CONVERGENCE_RATIO_MIN && ratio <= CONVERGENCE_RATIO_MAX;
        eprintln!(
            "position_error_halving,{:.17},{:.17},{:.17},{}",
            coarse.dt, fine.dt, ratio, pass
        );
        failures += usize::from(!pass);
    }

    if failures != 0 {
        eprintln!("{failures} free-fall validation check(s) failed");
        std::process::exit(1);
    }
}

fn run_case(dt: f64, requested_duration: f64) -> CaseResult {
    let gravity = SVector::from([0.0, -9.81, 0.0]);
    let initial_position = Point::new([0.0, 100.0, 0.0]);
    let mut world = PhysicsWorld::<3>::new(gravity);
    let body_handle = world.add_sphere(initial_position, 0.5, 1.0);
    {
        let body = world
            .body_mut(body_handle)
            .expect("body inserted immediately above");
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
    let body = world
        .body(body_handle)
        .expect("body remains in world during free fall");

    let observed_y = body.transform.translation.coord(1);
    let observed_vy = body.linear_velocity[1];

    // Closed-form continuous solution: y(t) = y0 + 1/2*g*t^2.
    let continuous_y = initial_position.coord(1) + 0.5 * gravity[1] * duration * duration;
    let continuous_vy = gravity[1] * duration;

    // Exact discrete solution for velocity-first semi-implicit Euler with
    // constant acceleration and zero initial velocity:
    // y_n = y_0 + g*dt^2*n*(n+1)/2; v_n = g*n*dt.
    let n = steps as f64;
    let discrete_y = initial_position.coord(1) + gravity[1] * dt * dt * n * (n + 1.0) * 0.5;
    let discrete_vy = gravity[1] * duration;

    let continuous_position_error = (observed_y - continuous_y).abs();
    let discrete_position_error = (observed_y - discrete_y).abs();
    let velocity_error = (observed_vy - continuous_vy).abs();

    let healthy = after.non_finite_body_count == 0
        && after.max_rotation_orthogonality_error <= ROTATION_TOLERANCE
        && after.max_rotation_determinant_error <= ROTATION_TOLERANCE
        && drift.relative_energy_drift.is_finite()
        && observed_y.is_finite()
        && observed_vy.is_finite()
        && discrete_vy.is_finite();

    let pass = healthy
        && discrete_position_error <= DISCRETE_POSITION_TOLERANCE
        && velocity_error <= VELOCITY_TOLERANCE;

    CaseResult {
        dt,
        steps,
        duration,
        continuous_position_error,
        discrete_position_error,
        velocity_error,
        relative_energy_drift: drift.relative_energy_drift,
        max_rotation_orthogonality_error: after.max_rotation_orthogonality_error,
        max_rotation_determinant_error: after.max_rotation_determinant_error,
        non_finite_bodies: after.non_finite_body_count,
        pass,
    }
}
