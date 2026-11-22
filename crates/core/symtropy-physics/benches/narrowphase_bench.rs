// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Matched repeated-step workloads for the narrowphase work counters.
//!
//! These benchmarks deliberately keep scene construction outside the measured
//! closure and aggregate deterministic work counts over a fixed frame window.
//! They do not claim physical accuracy or contact-recycling performance.

use criterion::{BatchSize, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::{HalfSpace, HyperBox, Point, Sphere, Transform};
use symtropy_physics::{
    BodyHandle, BodyType, NarrowphaseStats, PhysicsWorld, RigidBody,
};

const DT: f64 = 1.0 / 60.0;
const FRAMES_PER_SAMPLE: usize = 32;

fn accumulate(stats: &NarrowphaseStats, totals: &mut [usize; 8]) {
    totals[0] += stats.candidate_pairs;
    totals[1] += stats.halfspace_analytic_hits;
    totals[2] += stats.box_sat_hits;
    totals[3] += stats.mesh_provider_calls;
    totals[4] += stats.gjk_calls;
    totals[5] += stats.gjk_intersections;
    totals[6] += stats.epa_calls;
    totals[7] += stats.contact_points;
}

fn run_window(mut world: PhysicsWorld<3>) -> [usize; 8] {
    let mut totals = [0usize; 8];
    for _ in 0..FRAMES_PER_SAMPLE {
        world.step(DT);
        accumulate(world.narrowphase_stats(), &mut totals);
    }
    totals
}

fn analytic_box_plane_scene(count: usize) -> PhysicsWorld<3> {
    // Gravity keeps the boxes loaded against the plane throughout the window,
    // so this measures a persistent resting-contact workload.
    let mut world = PhysicsWorld::<3>::new(SVector::new(0.0, -9.81, 0.0));
    world.solver_iterations = 4;
    world.add_body(RigidBody::<3>::static_body(
        BodyHandle(0),
        Point::origin(),
        Box::new(HalfSpace::<3>::ground(1, 0.0)),
    ));

    for index in 0..count {
        let x = index as f64 * 1.25;
        let body = RigidBody::<3>::new(
            BodyHandle(index + 1),
            BodyType::Dynamic,
            Transform::from_translation(Point::new([x, 0.5, 0.0])),
            Box::new(HyperBox::<3>::cube(0.5)),
            1.0,
            SVector::from_element(1.0),
        );
        world.add_body(body);
    }
    world
}

fn separated_sphere_aabb_candidates(pair_count: usize) -> PhysicsWorld<3> {
    let mut world = PhysicsWorld::<3>::new(SVector::zeros());
    for index in 0..pair_count {
        let x = index as f64 * 4.0;
        world.add_body(RigidBody::<3>::new(
            BodyHandle(index * 2),
            BodyType::Dynamic,
            Transform::from_translation(Point::new([x, 0.0, 0.0])),
            Box::new(Sphere::<3>::unit()),
            1.0,
            SVector::from_element(1.0),
        ));
        world.add_body(RigidBody::<3>::new(
            BodyHandle(index * 2 + 1),
            BodyType::Dynamic,
            Transform::from_translation(Point::new([x + 0.8, 0.8, 0.0])),
            Box::new(Sphere::<3>::unit()),
            1.0,
            SVector::from_element(1.0),
        ));
    }
    world
}

fn bench_analytic_box_plane_narrowphase(c: &mut Criterion) {
    c.bench_function("narrowphase_analytic_box_plane_64x32_steps", |bench| {
        bench.iter_batched(
            || analytic_box_plane_scene(64),
            |world| black_box(run_window(world)),
            BatchSize::SmallInput,
        );
    });
}

fn bench_gjk_rejected_aabb_candidates(c: &mut Criterion) {
    c.bench_function("narrowphase_gjk_rejected_aabb_candidates_32x32_steps", |bench| {
        bench.iter_batched(
            || separated_sphere_aabb_candidates(32),
            |world| black_box(run_window(world)),
            BatchSize::SmallInput,
        );
    });
}

criterion_group!(
    benches,
    bench_analytic_box_plane_narrowphase,
    bench_gjk_rejected_aabb_candidates
);
criterion_main!(benches);
