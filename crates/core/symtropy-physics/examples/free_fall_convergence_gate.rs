// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Fail-closed analytical convergence gate for semi-implicit Euler.
//!
//! Runs the same closed-form free-fall problem in 2D, 3D, and 4D at four
//! timesteps. For constant acceleration and zero initial velocity,
//! semi-implicit Euler has an exact first-order position error:
//!
//!     |x_n - x_exact(T)| = 0.5 * |g| * T * dt
//!
//! Velocity is exact up to floating-point accumulation. The mechanical-energy
//! drift has the corresponding closed form:
//!
//!     E_n - E_0 = -0.5 * m * |g|^2 * T * dt
//!
//! This example exits non-zero if the measured values, observed convergence
//! order, or numerical-health diagnostics violate those analytical contracts.
//!
//! Run with:
//! `cargo run --release -p symtropy-physics --example free_fall_convergence_gate`

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const GRAVITY_MAGNITUDE: f64 = 9.81;
const INITIAL_HEIGHT: f64 = 100.0;
const SPHERE_RADIUS: f64 = 0.5;
const MASS: f64 = 1.0;
const DURATION: f64 = 1.0;
const TIMESTEPS: [f64; 4] = [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0];
const POSITION_TOLERANCE: f64 = 1.0e-9;
const VELOCITY_TOLERANCE: f64 = 1.0e-10;
const ENERGY_TOLERANCE: f64 = 1.0e-8;
const ROTATION_TOLERANCE: f64 = 1.0e-10;
const ORDER_TOLERANCE: f64 = 0.02;

#[derive(Clone, Copy)]
struct CaseResult {
    dimension: usize,
    dt: f64,
    steps: usize,
    duration: f64,
    position_error: f64,
    expected_position_error: f64,
    velocity_error: f64,
    energy_drift: f64,
    expected_energy_drift: f64,
    numerical_health: bool,
    duration_matches: bool,
}

fn run_case<const D: usize>(dt: f64) -> CaseResult {
    assert!(D >= 2, "free-fall axis requires at least two dimensions");

    let mut gravity = SVector::<f64, D>::zeros();
    gravity[1] = -GRAVITY_MAGNITUDE;

    let mut initial_coordinates = [0.0; D];
    initial_coordinates[1] = INITIAL_HEIGHT;
    let initial_position = Point::<D>::new(initial_coordinates);

    let mut world = PhysicsWorld::<D>::new(gravity);
    let body_handle = world.add_sphere(initial_position, SPHERE_RADIUS, MASS);
    {
        let body = world.body_mut(body_handle).expect("body inserted above");
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
    }

    let before = world.invariant_snapshot();
    let steps = (DURATION / dt).round() as usize;
    for _ in 0..steps {
        world.step(dt);
    }

    let duration = steps as f64 * dt;
    let after = world.invariant_snapshot();
    let drift = before.drift_to(&after);
    let body = world.body(body_handle).expect("body remains in world");

    // Closed-form continuous solution at the actual simulated duration.
    let expected_y = INITIAL_HEIGHT + 0.5 * gravity[1] * duration * duration;
    let expected_vy = gravity[1] * duration;
    let position_error = (body.transform.translation.coord(1) - expected_y).abs();
    let velocity_error = (body.linear_velocity[1] - expected_vy).abs();

    // Exact truncation-error predictions for this integrator and scenario.
    let expected_position_error =
        0.5 * GRAVITY_MAGNITUDE * duration * dt;
    let expected_energy_drift =
        -0.5 * MASS * GRAVITY_MAGNITUDE.powi(2) * duration * dt;

    let numerical_health = before.is_numerically_healthy(ROTATION_TOLERANCE)
        && after.is_numerically_healthy(ROTATION_TOLERANCE)
        && before.non_finite_body_count == 0
        && after.non_finite_body_count == 0;
    let duration_matches = (duration - DURATION).abs() <= 1.0e-12;

    CaseResult {
        dimension: D,
        dt,
        steps,
        duration,
        position_error,
        expected_position_error,
        velocity_error,
        energy_drift: drift.absolute_energy_drift,
        expected_energy_drift,
        numerical_health,
        duration_matches,
    }
}

fn validate_dimension<const D: usize>() -> usize {
    let mut failures = 0usize;
    let mut previous: Option<CaseResult> = None;

    for result in TIMESTEPS.map(run_case::<D>) {
        let expected_error_matches =
            (result.position_error - result.expected_position_error).abs()
                <= POSITION_TOLERANCE;
        let velocity_matches = result.velocity_error <= VELOCITY_TOLERANCE;
        let energy_matches =
            (result.energy_drift - result.expected_energy_drift).abs() <= ENERGY_TOLERANCE;

        let observed_order = previous.map(|prior| {
            (prior.position_error / result.position_error).ln()
                / (prior.dt / result.dt).ln()
        });
        let order_matches = observed_order
            .map(|order| (order - 1.0).abs() <= ORDER_TOLERANCE)
            .unwrap_or(true);

        let pass = expected_error_matches
            && velocity_matches
            && energy_matches
            && result.numerical_health
            && result.duration_matches
            && order_matches;

        println!(
            "{},{:.17},{},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{},{}",
            result.dimension,
            result.dt,
            result.steps,
            result.duration,
            result.position_error,
            result.expected_position_error,
            result.velocity_error,
            result.energy_drift,
            result.expected_energy_drift,
            observed_order
                .map(|order| format!("{order:.6}"))
                .unwrap_or_else(|| "NA".to_string()),
            if pass { "PASS" } else { "FAIL" }
        );

        if !pass {
            failures += 1;
            eprintln!(
                "dimension={} dt={:.17}: position={}, velocity={}, energy={}, health={}, duration={}, order={}",
                result.dimension,
                result.dt,
                expected_error_matches,
                velocity_matches,
                energy_matches,
                result.numerical_health,
                result.duration_matches,
                order_matches
            );
        }

        previous = Some(result);
    }

    failures
}

fn main() {
    println!(
        "dimension,dt,steps,duration,position_error,expected_position_error,velocity_error,energy_drift,expected_energy_drift,observed_position_order,pass"
    );

    let failures = validate_dimension::<2>()
        + validate_dimension::<3>()
        + validate_dimension::<4>();

    if failures != 0 {
        eprintln!("{failures} analytical free-fall convergence case(s) failed");
        std::process::exit(1);
    }
    eprintln!(
        "PASS: semi-implicit Euler first-order position convergence and energy-drift contracts verified in 2D, 3D, and 4D"
    );
}
