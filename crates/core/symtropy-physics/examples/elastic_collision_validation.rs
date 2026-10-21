// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Analytical head-on collision validation for symtropy-physics.
//!
//! Run with:
//! cargo run --locked -p symtropy-physics --example elastic_collision_validation
//!
//! This checks a narrow translational sphere-sphere collision model. It does
//! not validate frictional impacts, off-centre angular dynamics, contact with
//! static geometry, or the separate physical-thermodynamics ledger.

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::PhysicsWorld;

const DT: f64 = 1.0 / 120.0;
const MAX_STEPS: usize = 2_000;
const VELOCITY_TOLERANCE: f64 = 1e-6;
const MOMENTUM_TOLERANCE: f64 = 1e-8;
const ENERGY_TOLERANCE: f64 = 1e-8;

#[derive(Clone, Copy, Debug)]
struct CollisionCase {
    name: &'static str,
    mass_a: f64,
    mass_b: f64,
    velocity_a_initial: f64,
    velocity_b_initial: f64,
    restitution: f64,
}

const CASES: [CollisionCase; 3] = [
    CollisionCase {
        name: "equal_mass_elastic",
        mass_a: 1.0,
        mass_b: 1.0,
        velocity_a_initial: 1.0,
        velocity_b_initial: -1.0,
        restitution: 1.0,
    },
    CollisionCase {
        name: "unequal_mass_elastic",
        mass_a: 2.0,
        mass_b: 1.0,
        velocity_a_initial: 1.0,
        velocity_b_initial: -0.5,
        restitution: 1.0,
    },
    CollisionCase {
        name: "equal_mass_inelastic",
        mass_a: 1.0,
        mass_b: 1.0,
        velocity_a_initial: 1.0,
        velocity_b_initial: -1.0,
        restitution: 0.0,
    },
];

#[derive(Clone, Copy, Debug)]
struct CollisionResult {
    contact_step: usize,
    velocity_a_expected: f64,
    velocity_b_expected: f64,
    velocity_a_observed: f64,
    velocity_b_observed: f64,
    momentum_initial: f64,
    momentum_final: f64,
    kinetic_energy_initial: f64,
    kinetic_energy_final: f64,
    kinetic_energy_expected: f64,
    passed: bool,
}

fn expected_velocities(case: CollisionCase) -> (f64, f64) {
    let m1 = case.mass_a;
    let m2 = case.mass_b;
    let u1 = case.velocity_a_initial;
    let u2 = case.velocity_b_initial;
    let e = case.restitution;
    let total_mass = m1 + m2;

    (
        ((m1 - e * m2) * u1 + (1.0 + e) * m2 * u2) / total_mass,
        ((m2 - e * m1) * u2 + (1.0 + e) * m1 * u1) / total_mass,
    )
}

fn run_case(case: CollisionCase) -> Result<CollisionResult, String> {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    // Isolate the restitution law from overlap-correction work. Baumgarte
    // stabilization has its own energy-injection/penetration contract and is
    // intentionally outside this analytical velocity benchmark.
    world.baumgarte = 0.0;
    let body_a = world.add_sphere(Point::new([-1.0, 0.0, 0.0]), 0.5, case.mass_a);
    let body_b = world.add_sphere(Point::new([1.0, 0.0, 0.0]), 0.5, case.mass_b);

    for (handle, velocity) in [
        (body_a, case.velocity_a_initial),
        (body_b, case.velocity_b_initial),
    ] {
        let body = world
            .body_mut(handle)
            .ok_or_else(|| format!("{}: inserted body handle was not found", case.name))?;
        body.linear_velocity = SVector::from([velocity, 0.0, 0.0]);
        body.restitution = case.restitution;
        body.friction = 0.0;
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;
    }

    let initial = world.invariant_snapshot();
    let mut contact_step = None;
    for step in 1..=MAX_STEPS {
        world.step(DT);
        if !world.contacts.is_empty() {
            contact_step = Some(step);
            break;
        }
    }
    let contact_step = contact_step.ok_or_else(|| {
        format!(
            "{}: no collision detected within {MAX_STEPS} steps",
            case.name
        )
    })?;

    let observed_a = world
        .body(body_a)
        .ok_or_else(|| format!("{}: body A disappeared", case.name))?
        .linear_velocity[0];
    let observed_b = world
        .body(body_b)
        .ok_or_else(|| format!("{}: body B disappeared", case.name))?
        .linear_velocity[0];
    let final_state = world.invariant_snapshot();

    let (expected_a, expected_b) = expected_velocities(case);
    let expected_energy =
        0.5 * case.mass_a * expected_a * expected_a + 0.5 * case.mass_b * expected_b * expected_b;
    let velocity_error = (observed_a - expected_a)
        .abs()
        .max((observed_b - expected_b).abs());
    let momentum_error = (final_state.linear_momentum[0] - initial.linear_momentum[0]).abs();
    let energy_error = (final_state.kinetic_energy - expected_energy).abs();
    let is_elastic = (case.restitution - 1.0).abs() <= f64::EPSILON;
    let energy_regime_ok = if is_elastic {
        (final_state.kinetic_energy - initial.kinetic_energy).abs() <= ENERGY_TOLERANCE
    } else {
        final_state.kinetic_energy < initial.kinetic_energy - 0.5
    };
    let finite = [
        observed_a,
        observed_b,
        expected_a,
        expected_b,
        initial.linear_momentum[0],
        final_state.linear_momentum[0],
        initial.kinetic_energy,
        final_state.kinetic_energy,
        expected_energy,
    ]
    .iter()
    .all(|value| value.is_finite());

    let passed = finite
        && velocity_error <= VELOCITY_TOLERANCE
        && momentum_error <= MOMENTUM_TOLERANCE
        && energy_error <= ENERGY_TOLERANCE
        && energy_regime_ok;

    Ok(CollisionResult {
        contact_step,
        velocity_a_expected: expected_a,
        velocity_b_expected: expected_b,
        velocity_a_observed: observed_a,
        velocity_b_observed: observed_b,
        momentum_initial: initial.linear_momentum[0],
        momentum_final: final_state.linear_momentum[0],
        kinetic_energy_initial: initial.kinetic_energy,
        kinetic_energy_final: final_state.kinetic_energy,
        kinetic_energy_expected: expected_energy,
        passed,
    })
}

fn main() {
    println!(
        "case,restitution,contact_step,v_a_expected,v_a_observed,v_b_expected,v_b_observed,\
momentum_initial,momentum_final,kinetic_energy_initial,kinetic_energy_expected,\
kinetic_energy_final,pass"
    );

    let mut failures = 0usize;
    for case in CASES {
        match run_case(case) {
            Ok(result) => {
                println!(
                    "{},{:.6},{},{:.12},{:.12},{:.12},{:.12},{:.12},{:.12},\
{:.12},{:.12},{:.12},{}",
                    case.name,
                    case.restitution,
                    result.contact_step,
                    result.velocity_a_expected,
                    result.velocity_a_observed,
                    result.velocity_b_expected,
                    result.velocity_b_observed,
                    result.momentum_initial,
                    result.momentum_final,
                    result.kinetic_energy_initial,
                    result.kinetic_energy_expected,
                    result.kinetic_energy_final,
                    result.passed
                );
                failures += if result.passed { 0 } else { 1 };
            }
            Err(error) => {
                eprintln!("{error}");
                failures += 1;
            }
        }
    }

    if failures > 0 {
        eprintln!("{failures} collision validation case(s) failed");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn equal_mass_elastic_collision_swaps_velocities_and_conserves_energy() {
        let result = run_case(CASES[0]).expect("elastic collision should occur");
        assert!(result.passed, "{result:?}");
        assert!((result.velocity_a_observed + 1.0).abs() <= VELOCITY_TOLERANCE);
        assert!((result.velocity_b_observed - 1.0).abs() <= VELOCITY_TOLERANCE);
        assert!(
            (result.kinetic_energy_final - result.kinetic_energy_initial).abs() <= ENERGY_TOLERANCE
        );
    }

    #[test]
    fn unequal_mass_elastic_collision_matches_closed_form_reference() {
        let result = run_case(CASES[1]).expect("unequal-mass collision should occur");
        assert!(result.passed, "{result:?}");
        assert!(result.velocity_a_observed.abs() <= VELOCITY_TOLERANCE);
        assert!((result.velocity_b_observed - 1.5).abs() <= VELOCITY_TOLERANCE);
        assert!((result.momentum_final - result.momentum_initial).abs() <= MOMENTUM_TOLERANCE);
    }

    #[test]
    fn perfectly_inelastic_collision_conserves_momentum_but_loses_kinetic_energy() {
        let result = run_case(CASES[2]).expect("inelastic collision should occur");
        assert!(result.passed, "{result:?}");
        assert!(result.velocity_a_observed.abs() <= VELOCITY_TOLERANCE);
        assert!(result.velocity_b_observed.abs() <= VELOCITY_TOLERANCE);
        assert!(result.kinetic_energy_final < result.kinetic_energy_initial - 0.5);
        assert!((result.momentum_final - result.momentum_initial).abs() <= MOMENTUM_TOLERANCE);
    }
}
