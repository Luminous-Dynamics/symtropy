// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Fail-closed N-dimensional harmonic-oscillator convergence and stability gate.
//!
//! This is complementary to free_fall_convergence_gate: it exercises a
//! conservative restoring force, rather than constant acceleration.
//!
//! For x'' + omega^2*x = 0, the continuous solution is known exactly. The
//! velocity-first semi-implicit Euler update also preserves the modified
//! discrete energy
//!
//!   E_tilde = m/2 * (|v|^2 + omega^2*|x|^2 - h*omega^2*x.dot(v))
//!
//! in exact arithmetic. This quadratic form is positive definite only when
//! 0 < h*omega < 2. The gate checks first-order state-error convergence,
//! modified-energy conservation, numerical health, and the stability
//! boundary in 2D, 3D, and 4D.
//!
//! This contract applies only to the idealized no-contact, undamped,
//! unclamped, isotropic oscillator used here. It does not qualify contact,
//! friction, asymmetric inertia, or arbitrary time-varying forces.
//!
//! Run with:
//! cargo run --release -p symtropy-physics --example harmonic_oscillator_convergence_gate

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const OMEGA: f64 = 1.7;
const MASS: f64 = 1.0;
const DURATION: f64 = 1.0;
const SPHERE_RADIUS: f64 = 0.1;
const INITIAL_POSITION: [f64; 4] = [1.0, 0.5, -0.25, 0.125];
const INITIAL_VELOCITY: [f64; 4] = [0.3, -0.2, 0.1, -0.05];
const TIMESTEPS: [f64; 4] = [1.0 / 30.0, 1.0 / 60.0, 1.0 / 120.0, 1.0 / 240.0];

const STATE_ORDER_TOLERANCE: f64 = 0.05;
const MODIFIED_ENERGY_RELATIVE_TOLERANCE: f64 = 1.0e-10;
const NUMERICAL_HEALTH_TOLERANCE: f64 = 1.0e-10;

fn stable_timestep(dt: f64) -> bool {
    dt.is_finite() && dt > 0.0 && OMEGA * dt < 2.0
}

fn modified_energy<const D: usize>(
    position: &SVector<f64, D>,
    velocity: &SVector<f64, D>,
    dt: f64,
) -> f64 {
    0.5 * MASS
        * (velocity.norm_squared()
            + OMEGA.powi(2) * position.norm_squared()
            - dt * OMEGA.powi(2) * position.dot(velocity))
}

#[derive(Clone, Copy)]
struct CaseResult {
    dimension: usize,
    dt: f64,
    steps: usize,
    duration: f64,
    state_error: f64,
    modified_energy_relative_drift: f64,
    numerical_health: bool,
    duration_matches: bool,
    stability_condition: bool,
}

fn run_case<const D: usize>(dt: f64) -> CaseResult {
    assert!(D >= 2 && D <= 4, "this gate declares initial conditions through 4D");
    assert!(stable_timestep(dt), "unstable or invalid timestep supplied to run_case");

    let initial_position = SVector::<f64, D>::from_fn(|i, _| INITIAL_POSITION[i]);
    let initial_velocity = SVector::<f64, D>::from_fn(|i, _| INITIAL_VELOCITY[i]);
    let point = Point::<D>::new(std::array::from_fn(|i| initial_position[i]));

    let mut world = PhysicsWorld::<D>::new(SVector::zeros());
    let body_handle = world.add_sphere(point, SPHERE_RADIUS, MASS);
    {
        let body = world.body_mut(body_handle).expect("body inserted above");
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
        body.linear_velocity = initial_velocity;
    }

    let before = world.invariant_snapshot();
    let initial_modified_energy = modified_energy(&initial_position, &initial_velocity, dt);
    let steps = (DURATION / dt).round() as usize;

    for _ in 0..steps {
        let position = world
            .body(body_handle)
            .expect("body remains in world")
            .transform
            .translation
            .0;
        let spring_force = position * (-MASS * OMEGA.powi(2));
        world
            .body_mut(body_handle)
            .expect("body remains in world")
            .apply_force(spring_force);
        world.step(dt);
    }

    let duration = steps as f64 * dt;
    let after = world.invariant_snapshot();
    let body = world.body(body_handle).expect("body remains in world");
    let final_position = body.transform.translation.0;
    let final_velocity = body.linear_velocity;

    let phase = OMEGA * duration;
    let exact_position =
        initial_position * phase.cos() + initial_velocity * (phase.sin() / OMEGA);
    let exact_velocity =
        initial_velocity * phase.cos() - initial_position * (OMEGA * phase.sin());

    // Nondimensionalize velocity error by omega before combining it with
    // position error, so the norm does not mix units.
    let position_error = (final_position - exact_position).norm();
    let velocity_error = (final_velocity - exact_velocity).norm() / OMEGA;
    let state_error = position_error.hypot(velocity_error);

    let final_modified_energy = modified_energy(&final_position, &final_velocity, dt);
    let modified_energy_relative_drift =
        ((final_modified_energy - initial_modified_energy) / initial_modified_energy).abs();

    let numerical_health = before.is_numerically_healthy(NUMERICAL_HEALTH_TOLERANCE)
        && after.is_numerically_healthy(NUMERICAL_HEALTH_TOLERANCE)
        && before.non_finite_body_count == 0
        && after.non_finite_body_count == 0
        && final_position.iter().all(|value| value.is_finite())
        && final_velocity.iter().all(|value| value.is_finite());

    CaseResult {
        dimension: D,
        dt,
        steps,
        duration,
        state_error,
        modified_energy_relative_drift,
        numerical_health,
        duration_matches: (duration - DURATION).abs() <= 1.0e-12,
        stability_condition: stable_timestep(dt),
    }
}

fn validate_dimension<const D: usize>() -> usize {
    let mut failures = 0usize;
    let mut previous: Option<CaseResult> = None;

    for result in TIMESTEPS.map(run_case::<D>) {
        let observed_order = previous.map(|prior| {
            (prior.state_error / result.state_error).ln()
                / (prior.dt / result.dt).ln()
        });
        let order_matches = observed_order
            .map(|order| (order - 1.0).abs() <= STATE_ORDER_TOLERANCE)
            .unwrap_or(true);
        let energy_matches =
            result.modified_energy_relative_drift <= MODIFIED_ENERGY_RELATIVE_TOLERANCE;

        let pass = result.stability_condition
            && energy_matches
            && result.numerical_health
            && result.duration_matches
            && order_matches;

        println!(
            "{},{:.17},{},{:.17},{:.17},{:.17},{},{},{}",
            result.dimension,
            result.dt,
            result.steps,
            result.duration,
            result.state_error,
            result.modified_energy_relative_drift,
            observed_order
                .map(|order| format!("{order:.6}"))
                .unwrap_or_else(|| "NA".to_string()),
            result.stability_condition,
            if pass { "PASS" } else { "FAIL" }
        );

        if !pass {
            failures += 1;
            eprintln!(
                "dimension={} dt={:.17}: stable={}, energy={}, health={}, duration={}, order={}",
                result.dimension,
                result.dt,
                result.stability_condition,
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

fn validate_stability_boundary() -> usize {
    let just_below = (2.0 - 1.0e-9) / OMEGA;
    let boundary = 2.0 / OMEGA;
    let above = (2.0 + 1.0e-9) / OMEGA;
    let cases = [
        ("positive_timestep", stable_timestep(1.0 / 60.0), true),
        ("zero_timestep", stable_timestep(0.0), false),
        ("negative_timestep", stable_timestep(-1.0 / 60.0), false),
        ("nan_timestep", stable_timestep(f64::NAN), false),
        ("infinite_timestep", stable_timestep(f64::INFINITY), false),
        ("below_boundary", stable_timestep(just_below), true),
        ("at_boundary", stable_timestep(boundary), false),
        ("above_boundary", stable_timestep(above), false),
    ];

    let mut failures = 0usize;
    for (name, actual, expected) in cases {
        let pass = actual == expected;
        println!("stability_boundary,{name},{actual},{}", if pass { "PASS" } else { "FAIL" });
        if !pass {
            failures += 1;
        }
    }
    failures
}

fn main() {
    println!(
        "dimension,dt,steps,duration,state_error,modified_energy_relative_drift,observed_state_order,stable,pass"
    );

    let failures = validate_stability_boundary()
        + validate_dimension::<2>()
        + validate_dimension::<3>()
        + validate_dimension::<4>();

    if failures != 0 {
        eprintln!("{failures} harmonic-oscillator qualification case(s) failed");
        std::process::exit(1);
    }
    eprintln!(
        "PASS: harmonic-oscillator first-order convergence and modified-energy contracts verified in 2D, 3D, and 4D"
    );
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn velocity_first_stability_boundary_is_strictly_h_omega_less_than_two() {
        assert!(stable_timestep((2.0 - 1.0e-9) / OMEGA));
        assert!(!stable_timestep(2.0 / OMEGA));
        assert!(!stable_timestep((2.0 + 1.0e-9) / OMEGA));
    }

    #[test]
    fn invalid_timesteps_fail_closed() {
        assert!(!stable_timestep(0.0));
        assert!(!stable_timestep(-0.1));
        assert!(!stable_timestep(f64::NAN));
        assert!(!stable_timestep(f64::INFINITY));
    }
}
