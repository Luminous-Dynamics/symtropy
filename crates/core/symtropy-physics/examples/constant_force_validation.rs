// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Analytical force-acceleration convergence study.
//!
//! Run with:
//! cargo run --release --example constant_force_validation
//!
//! Repeatedly applies a constant force to an isolated body and compares the
//! result with both the exact discrete semi-implicit-Euler trajectory and the
//! continuous constant-acceleration solution. CSV measurements go to stdout;
//! convergence checks and failures go to stderr.
//!
//! This is a narrow translational integration check, not validation of
//! collision, contact, articulated, structural, fatigue, or material models.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const DISCRETE_POSITION_TOLERANCE: f64 = 1e-8;
const VELOCITY_TOLERANCE: f64 = 1e-9;
const CONVERGENCE_RATIO_MIN: f64 = 1.8;
const CONVERGENCE_RATIO_MAX: f64 = 2.2;

struct CaseResult {
    dt: f64,
    steps: usize,
    duration: f64,
    continuous_position_error: f64,
    discrete_position_error: f64,
    velocity_error: f64,
    pass: bool,
}

fn main() {
    let mut failures = 0usize;
    let results: Vec<CaseResult> = [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0]
        .into_iter()
        .map(|dt| run_case(dt, 1.0))
        .collect();

    println!("dt,steps,duration,continuous_position_error,discrete_position_error,velocity_error,pass");
    for result in &results {
        println!(
            "{:.17},{},{:.17},{:.17},{:.17},{:.17},{}",
            result.dt,
            result.steps,
            result.duration,
            result.continuous_position_error,
            result.discrete_position_error,
            result.velocity_error,
            result.pass
        );
        failures += usize::from(!result.pass);
    }

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
        eprintln!("{failures} constant-force validation check(s) failed");
        std::process::exit(1);
    }
}

fn run_case(dt: f64, requested_duration: f64) -> CaseResult {
    let mass = 2.0;
    let force = SVector::from([4.0, -3.0, 2.0]);
    let acceleration = force / mass;
    let initial_velocity = SVector::from([0.5, -0.25, 0.75]);
    let initial_position = SVector::from([1.0, -2.0, 0.5]);

    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::new([1.0, -2.0, 0.5]), 0.25, mass);
    world
        .body_mut(handle)
        .expect("body was inserted immediately above")
        .linear_velocity = initial_velocity;

    let steps = (requested_duration / dt).round() as usize;
    for _ in 0..steps {
        // Force accumulators are cleared by each integration step; applying the
        // force every tick represents a constant externally applied force.
        world
            .body_mut(handle)
            .expect("body remains in world during force integration")
            .apply_force(force);
        world.step(dt);
    }

    let duration = steps as f64 * dt;
    let body = world
        .body(handle)
        .expect("body remains in world during force integration");
    let observed_position = body.transform.translation.0;
    let observed_velocity = body.linear_velocity;

    let continuous_position =
        initial_position + initial_velocity * duration + acceleration * (0.5 * duration * duration);
    let expected_velocity = initial_velocity + acceleration * duration;

    let n = steps as f64;
    let discrete_position = initial_position
        + initial_velocity * duration
        + acceleration * (dt * dt * n * (n + 1.0) * 0.5);

    let continuous_position_error = (observed_position - continuous_position).norm();
    let discrete_position_error = (observed_position - discrete_position).norm();
    let velocity_error = (observed_velocity - expected_velocity).norm();

    let pass = observed_position.iter().all(|value| value.is_finite())
        && observed_velocity.iter().all(|value| value.is_finite())
        && discrete_position_error <= DISCRETE_POSITION_TOLERANCE
        && velocity_error <= VELOCITY_TOLERANCE;

    CaseResult {
        dt,
        steps,
        duration,
        continuous_position_error,
        discrete_position_error,
        velocity_error,
        pass,
    }
}
