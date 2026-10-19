// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Harmonic-oscillator stability and convergence study for semi-implicit Euler.
//!
//! Run with:
//! cargo run --release --example harmonic_oscillator_validation
//!
//! Compares a one-dimensional Hooke-law oscillator against the analytical
//! continuous solution over four timestep refinements. The check verifies
//! first-order convergence in position and second-order velocity convergence, bounded amplitude in
//! the known linearly stable regime, and finite state. It also checks that
//! the observed peak displacement does not exceed the analytical amplitude
//! bound for this exact velocity-first update.
//!
//! This is a translational, undamped, collision-free reference case. It does
//! not validate rotational inertia, contact, joints, or arbitrary Hamiltonian
//! systems. Stability conditions for this fixture must not be generalized to
//! unrelated models or timesteps.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const MASS: f64 = 1.0;
const SPRING_K: f64 = 4.0;
const OMEGA: f64 = 2.0; // sqrt(SPRING_K / MASS)
const DURATION: f64 = 2.0;
const POSITION_TOLERANCE: f64 = 1e-8;
const VELOCITY_TOLERANCE: f64 = 1e-8;
const POSITION_CONVERGENCE_RATIO_MIN: f64 = 1.75;
const POSITION_CONVERGENCE_RATIO_MAX: f64 = 2.25;
const VELOCITY_CONVERGENCE_RATIO_MIN: f64 = 3.5;
const VELOCITY_CONVERGENCE_RATIO_MAX: f64 = 4.5;
const AMPLITUDE_RELATIVE_TOLERANCE: f64 = 1e-6;

struct CaseResult {
    dt: f64,
    steps: usize,
    duration: f64,
    position_error: f64,
    velocity_error: f64,
    discrete_position_error: f64,
    discrete_velocity_error: f64,
    maximum_abs_position: f64,
    theoretical_discrete_amplitude_bound: f64,
    pass: bool,
}

fn main() {
    let mut failures = 0usize;
    let results: Vec<CaseResult> = [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0]
        .into_iter()
        .map(|dt| run_case(dt, DURATION))
        .collect();

    println!("dt,steps,duration,continuous_position_error,continuous_velocity_error,discrete_position_error,discrete_velocity_error,max_abs_position,discrete_amplitude_bound,pass");
    for result in &results {
        println!(
            "{:.17},{},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{:.17},{}",
            result.dt,
            result.steps,
            result.duration,
            result.position_error,
            result.velocity_error,
            result.discrete_position_error,
            result.discrete_velocity_error,
            result.maximum_abs_position,
            result.theoretical_discrete_amplitude_bound,
            result.pass
        );
        failures += usize::from(!result.pass);
    }

    eprintln!("check,coarse_dt,fine_dt,position_error_ratio,velocity_error_ratio,pass");
    for pair in results.windows(2) {
        let coarse = &pair[0];
        let fine = &pair[1];
        let position_ratio = coarse.position_error / fine.position_error;
        let velocity_ratio = coarse.velocity_error / fine.velocity_error;
        let pass = ratio_in_band(
            position_ratio,
            POSITION_CONVERGENCE_RATIO_MIN,
            POSITION_CONVERGENCE_RATIO_MAX,
        ) && ratio_in_band(
            velocity_ratio,
            VELOCITY_CONVERGENCE_RATIO_MIN,
            VELOCITY_CONVERGENCE_RATIO_MAX,
        );
        eprintln!(
            "first_order_convergence,{:.17},{:.17},{:.17},{:.17},{}",
            coarse.dt, fine.dt, position_ratio, velocity_ratio, pass
        );
        failures += usize::from(!pass);
    }

    if failures != 0 {
        eprintln!("{failures} harmonic-oscillator validation check(s) failed");
        std::process::exit(1);
    }
}

fn ratio_in_band(ratio: f64, minimum: f64, maximum: f64) -> bool {
    ratio.is_finite() && ratio >= minimum && ratio <= maximum
}

fn run_case(dt: f64, requested_duration: f64) -> CaseResult {
    let steps = (requested_duration / dt).round() as usize;
    let duration = steps as f64 * dt;
    let h = OMEGA * dt;

    // Velocity-first symplectic Euler is linearly stable for this undamped
    // harmonic oscillator when 0 < omega*dt < 2. This gate does not assert
    // unconditional stability outside the declared fixture and step range.
    let linearly_stable = h > 0.0 && h < 2.0;
    let amplitude_bound = if linearly_stable {
        1.0 / (1.0 - h * h / 4.0).sqrt()
    } else {
        f64::INFINITY
    };

    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    let handle = world.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.05, MASS);
    {
        let body = world
            .body_mut(handle)
            .expect("body inserted immediately above");
        body.linear_velocity = SVector::zeros();
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
    }

    let mut maximum_abs_position = 1.0_f64;
    // Independent scalar implementation of the declared discrete update.
    // This is a code-path differential check, not a fully independent physical
    // model; continuous-solution convergence remains a separate check.
    let mut reference_x = 1.0_f64;
    let mut reference_v = 0.0_f64;
    for _ in 0..steps {
        let x = world
            .body(handle)
            .expect("body remains in world")
            .transform
            .translation
            .coord(0);
        let force = SVector::from([-SPRING_K * x, 0.0, 0.0]);
        world
            .body_mut(handle)
            .expect("body remains in world")
            .apply_force(force);
        world.step(dt);

        reference_v += (-SPRING_K / MASS) * reference_x * dt;
        reference_x += reference_v * dt;

        let next_x = world
            .body(handle)
            .expect("body remains in world")
            .transform
            .translation
            .coord(0);
        maximum_abs_position = maximum_abs_position.max(next_x.abs());
    }

    let body = world.body(handle).expect("body remains in world");
    let observed_x = body.transform.translation.coord(0);
    let observed_vx = body.linear_velocity[0];

    // Exact analytical solution for x(0)=1, v(0)=0.
    let expected_x = (OMEGA * duration).cos();
    let expected_vx = -OMEGA * (OMEGA * duration).sin();
    let position_error = (observed_x - expected_x).abs();
    let velocity_error = (observed_vx - expected_vx).abs();
    let discrete_position_error = (observed_x - reference_x).abs();
    let discrete_velocity_error = (observed_vx - reference_v).abs();

    let amplitude_pass = linearly_stable
        && maximum_abs_position
            <= amplitude_bound * (1.0 + AMPLITUDE_RELATIVE_TOLERANCE);
    let pass = linearly_stable
        && observed_x.is_finite()
        && observed_vx.is_finite()
        && maximum_abs_position.is_finite()
        && position_error.is_finite()
        && velocity_error.is_finite()
        && discrete_position_error.is_finite()
        && discrete_velocity_error.is_finite()
        && discrete_position_error <= POSITION_TOLERANCE
        && discrete_velocity_error <= VELOCITY_TOLERANCE
        && amplitude_pass;

    CaseResult {
        dt,
        steps,
        duration,
        position_error,
        velocity_error,
        discrete_position_error,
        discrete_velocity_error,
        maximum_abs_position,
        theoretical_discrete_amplitude_bound: amplitude_bound,
        pass,
    }
}
