//! Reproducible contact quality/performance campaign for two canonical 3D scenes.
//!
//! Run with:
//!   cargo run --release -p symtropy-physics --example contact_quality_campaign
//!
//! Scene construction is excluded from measured step time. Reported energy
//! change is observational only: the current ledger does not yet account for
//! all external, normal-contact, damping and stabilization work.

use std::time::Instant;

use nalgebra::SVector;
use symtropy_math::{HalfSpace, Point};
use symtropy_physics::body::{BodyHandle, RigidBody};
use symtropy_physics::PhysicsWorld;

const DT: f64 = 1.0 / 120.0;
const STEPS_PER_RUN: usize = 120;
const WARMUP_RUNS: usize = 3;
const MEASURED_RUNS: usize = 11;

#[derive(Clone, Copy)]
struct RunMetrics {
    median_step_ns: u128,
    p95_step_ns: u128,
    final_contact_count: usize,
    max_penetration: f64,
    mechanical_energy_delta: f64,
    max_rotation_error: f64,
    non_finite_body_count: usize,
    mass_property_mismatch_count: usize,
}

fn make_world(body_count: usize, stacked: bool) -> PhysicsWorld<3> {
    let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
    world.solver_iterations = 8;

    let plane = RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(HalfSpace::<3>::new(SVector::new(0.0, 1.0, 0.0), 0.0)),
    );
    world.add_body(plane);

    for index in 0..body_count {
        let position = if stacked {
            Point::new([0.0, 0.9 + index as f64 * 0.9, 0.0])
        } else {
            let x = (index % 50) as f64 * 1.1;
            let z = (index / 50) as f64 * 1.1;
            Point::new([x, 0.49, z])
        };
        let handle = world.add_sphere(position, 0.5, 1.0);
        let body = world.body_mut(handle).expect("new body handle is valid");
        body.linear_velocity = SVector::new(0.0, -0.5, 0.0);
        body.friction = 0.5;
        body.restitution = 0.0;
    }
    world
}

fn percentile(values: &[u128], percentile: f64) -> u128 {
    let index = ((values.len() - 1) as f64 * percentile).ceil() as usize;
    values[index]
}

fn run_once(body_count: usize, stacked: bool) -> RunMetrics {
    let mut world = make_world(body_count, stacked);
    let before = world.invariant_snapshot();
    let mut step_ns = Vec::with_capacity(STEPS_PER_RUN);
    let mut max_penetration = 0.0_f64;
    let mut max_rotation_error = 0.0_f64;

    for _ in 0..STEPS_PER_RUN {
        let start = Instant::now();
        world.try_step(DT).expect("contact campaign step must commit");
        step_ns.push(start.elapsed().as_nanos());
        let snapshot = world.invariant_snapshot();
        max_penetration = max_penetration.max(snapshot.max_penetration_depth);
        max_rotation_error = max_rotation_error
            .max(snapshot.max_rotation_orthogonality_error)
            .max(snapshot.max_rotation_determinant_error);
        assert!(
            snapshot.is_numerically_healthy(1e-8),
            "contact campaign produced unhealthy post-step state"
        );
    }

    step_ns.sort_unstable();
    let after = world.invariant_snapshot();
    RunMetrics {
        median_step_ns: percentile(&step_ns, 0.5),
        p95_step_ns: percentile(&step_ns, 0.95),
        final_contact_count: world.contacts.len(),
        max_penetration,
        mechanical_energy_delta: after.mechanical_energy - before.mechanical_energy,
        max_rotation_error,
        non_finite_body_count: after.non_finite_body_count,
        mass_property_mismatch_count: after.mass_property_mismatch_count,
    }
}

fn campaign(name: &str, body_count: usize, stacked: bool) {
    let mut samples = Vec::with_capacity(MEASURED_RUNS);
    for run in 0..(WARMUP_RUNS + MEASURED_RUNS) {
        let result = run_once(body_count, stacked);
        if run >= WARMUP_RUNS {
            samples.push(result);
        }
    }

    let mut median_step_ns: Vec<_> = samples.iter().map(|row| row.median_step_ns).collect();
    let mut p95_step_ns: Vec<_> = samples.iter().map(|row| row.p95_step_ns).collect();
    median_step_ns.sort_unstable();
    p95_step_ns.sort_unstable();

    let mut contact_counts: Vec<_> = samples.iter().map(|row| row.final_contact_count).collect();
    contact_counts.sort_unstable();
    let mut energy_deltas: Vec<_> = samples
        .iter()
        .map(|row| row.mechanical_energy_delta)
        .collect();
    energy_deltas.sort_by(f64::total_cmp);
    let max_penetration = samples
        .iter()
        .map(|row| row.max_penetration)
        .fold(0.0_f64, f64::max);
    let max_rotation_error = samples
        .iter()
        .map(|row| row.max_rotation_error)
        .fold(0.0_f64, f64::max);
    let max_non_finite = samples
        .iter()
        .map(|row| row.non_finite_body_count)
        .max()
        .unwrap_or(0);
    let max_mass_mismatches = samples
        .iter()
        .map(|row| row.mass_property_mismatch_count)
        .max()
        .unwrap_or(0);
    println!(
        "{name},{body_count},{STEPS_PER_RUN},{DT:.9e},{},{},{},{:.9e},{:.9e},{:.9e},{},{},{}",
        median_step_ns[MEASURED_RUNS / 2],
        p95_step_ns[MEASURED_RUNS / 2],
        contact_counts[MEASURED_RUNS / 2],
        max_penetration,
        energy_deltas[MEASURED_RUNS / 2],
        max_rotation_error,
        max_non_finite,
        max_mass_mismatches,
        MEASURED_RUNS
    );
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
        "scenario,bodies,steps,dt,median_step_ns,p95_step_ns,final_contacts,max_penetration,mechanical_energy_delta,max_rotation_error,non_finite_bodies,mass_mismatches,repeats"
    );

    campaign("independent_plane_1000", 1000, false);
    campaign("single_stack_100", 100, true);
}
