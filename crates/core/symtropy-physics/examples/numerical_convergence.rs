// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Reproducible first-order convergence campaign for the baseline rigid-body integrator.
//!
//! Run with:
//!   cargo run --release -p symtropy-physics --example numerical_convergence
//!
//! This is an initial kernel benchmark, not an engineering validation claim.
//! It emits CSV so results can be archived and compared at matched scenarios.
//! Runtime is the median of repeated runs; correctness metrics are checked
//! against analytical solutions after every run.

use std::time::Instant;

use nalgebra::SVector;
use symtropy_math::Point;
use symtropy_physics::RigidBody;
use symtropy_physics::body::BodyHandle;
use symtropy_physics::integrator::integrate;

const GRAVITY: f64 = 9.81;
const STEP_SIZES: [f64; 4] = [0.1, 0.05, 0.025, 0.0125];
const WARMUP_RUNS: usize = 5;
const MEASURED_RUNS: usize = 31;

#[derive(Clone, Copy)]
struct ResultRow {
    dt: f64,
    steps: usize,
    state_error: f64,
    energy_drift: f64,
    median_elapsed_ns: u128,
}

fn run_free_fall(dt: f64) -> ResultRow {
    let steps = (1.0 / dt).round() as usize;
    assert!((steps as f64 * dt - 1.0).abs() < 1e-12);

    let gravity = SVector::from([0.0, -GRAVITY, 0.0]);
    let mut timings = Vec::with_capacity(MEASURED_RUNS);
    let mut state_error = 0.0;
    let mut energy_drift = 0.0;

    for run in 0..(WARMUP_RUNS + MEASURED_RUNS) {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([0.0, 1.0, 0.0]), 0.01, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;

        let start = Instant::now();
        for _ in 0..steps {
            integrate(&mut body, &gravity, dt);
        }
        let elapsed_ns = start.elapsed().as_nanos();

        let exact_y = 1.0 - 0.5 * GRAVITY;
        let exact_vy = -GRAVITY;
        state_error = (body.position()[1] - exact_y).hypot(body.linear_velocity[1] - exact_vy);
        let final_energy = 0.5 * body.mass * body.linear_velocity.norm_squared()
            + body.mass * GRAVITY * body.position()[1];
        let initial_energy = body.mass * GRAVITY;
        energy_drift = (final_energy - initial_energy).abs() / initial_energy;

        if run >= WARMUP_RUNS {
            timings.push(elapsed_ns);
        }
    }

    timings.sort_unstable();
    ResultRow {
        dt,
        steps,
        state_error,
        energy_drift,
        median_elapsed_ns: timings[MEASURED_RUNS / 2],
    }
}

fn run_harmonic_oscillator(dt: f64) -> ResultRow {
    // Unit mass, unit stiffness: x'' = -x, with x(0)=1 and v(0)=0.
    let steps = (1.0 / dt).round() as usize;
    assert!((steps as f64 * dt - 1.0).abs() < 1e-12);

    let zero_gravity = SVector::<f64, 3>::zeros();
    let initial_energy = 0.5;
    let mut timings = Vec::with_capacity(MEASURED_RUNS);
    let mut state_error = 0.0;
    let mut energy_drift = 0.0;

    for run in 0..(WARMUP_RUNS + MEASURED_RUNS) {
        let mut body =
            RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::new([1.0, 0.0, 0.0]), 0.01, 1.0);
        body.linear_damping = 0.0;
        body.angular_damping = 0.0;

        let start = Instant::now();
        for _ in 0..steps {
            let x = body.position()[0];
            body.apply_force(SVector::from([-x, 0.0, 0.0]));
            integrate(&mut body, &zero_gravity, dt);
        }
        let elapsed_ns = start.elapsed().as_nanos();

        let exact_x = 1.0_f64.cos();
        let exact_vx = -1.0_f64.sin();
        state_error = (body.position()[0] - exact_x).hypot(body.linear_velocity[0] - exact_vx);
        let final_energy =
            0.5 * body.linear_velocity.norm_squared() + 0.5 * body.position()[0].powi(2);
        energy_drift = (final_energy - initial_energy).abs() / initial_energy;

        if run >= WARMUP_RUNS {
            timings.push(elapsed_ns);
        }
    }

    timings.sort_unstable();
    ResultRow {
        dt,
        steps,
        state_error,
        energy_drift,
        median_elapsed_ns: timings[MEASURED_RUNS / 2],
    }
}

fn scenario_hash(name: &str, dt: f64, steps: usize) -> u64 {
    // Stable FNV-1a identifier for this exact benchmark scenario.
    // This is not a cryptographic digest; source revision is recorded separately.
    let canonical = format!("{name}|dt={dt:.17e}|steps={steps}|duration_s=1.0");
    canonical
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        })
}

fn emit_campaign(name: &str, rows: &[ResultRow]) {
    let mut previous: Option<ResultRow> = None;
    for row in rows {
        let observed_order = previous.map_or(f64::NAN, |coarse| {
            (coarse.state_error / row.state_error).ln() / (coarse.dt / row.dt).ln()
        });
        println!(
            "{name},{:.8e},{},{:.8e},{:.8e},{:.4},{},{},{},{:016x}",
            row.dt,
            row.steps,
            row.state_error,
            row.energy_drift,
            observed_order,
            MEASURED_RUNS,
            row.median_elapsed_ns,
            std::env::consts::ARCH,
            scenario_hash(name, row.dt, row.steps)
        );
        previous = Some(*row);
    }
}

fn main() {
    println!(
        "# package_version={},target_arch={},target_os={},rustc_version={},source_revision={}",
        env!("CARGO_PKG_VERSION"),
        std::env::consts::ARCH,
        std::env::consts::OS,
        option_env!("RUSTC_VERSION").unwrap_or("not-recorded"),
        option_env!("SYMTROPY_SOURCE_REV").unwrap_or("not-recorded")
    );
    println!(
        "scenario,dt,steps,state_error,relative_energy_drift,observed_order,repeats,median_elapsed_ns,target_arch,scenario_hash_fnv1a64"
    );

    let free_fall: Vec<_> = STEP_SIZES.iter().copied().map(run_free_fall).collect();
    emit_campaign("free_fall", &free_fall);

    let oscillator: Vec<_> = STEP_SIZES
        .iter()
        .copied()
        .map(run_harmonic_oscillator)
        .collect();
    emit_campaign("harmonic_oscillator", &oscillator);
}
