// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::{Capsule, ConvexHull, HyperBox, Point, Sphere};
use symtropy_physics::broadphase::Lbvh;
use symtropy_physics::gjk;
use symtropy_physics::raycast;
use symtropy_physics::{BodyHandle, PhysicsWorld, RigidBody};

fn bench_gjk_sphere_sphere(c: &mut Criterion) {
    let a = Sphere::<3>::unit();
    let b = Sphere::<3>::unit();
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_sphere_sphere_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_box_box(c: &mut Criterion) {
    let a = ConvexHull::<3>::unit_cube();
    let b = ConvexHull::<3>::unit_cube();
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_box_box_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_4d_tesseract(c: &mut Criterion) {
    let a = ConvexHull::<4>::unit_cube();
    let b = ConvexHull::<4>::unit_cube();
    let pa = SVector::from([0.0, 0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0, 0.0]);

    c.bench_function("gjk_tesseract_4d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_physics_step_10_bodies(c: &mut Criterion) {
    c.bench_function("physics_step_10_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
                for i in 0..10 {
                    let h = world.add_sphere(Point::new([i as f64 * 3.0, 10.0, 0.0]), 0.5, 1.0);
                    world.body_mut(h).unwrap().linear_velocity = SVector::from([0.0, -5.0, 0.0]);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_physics_step_100_bodies(c: &mut Criterion) {
    c.bench_function("physics_step_100_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
                for i in 0..100 {
                    let x = (i % 10) as f64 * 2.5;
                    let y = (i / 10) as f64 * 2.5 + 5.0;
                    world.add_sphere(Point::new([x, y, 0.0]), 0.5, 1.0);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

fn bench_physics_step_2d(c: &mut Criterion) {
    c.bench_function("physics_step_50_bodies_2d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<2>::new(SVector::from([0.0, -9.81]));
                for i in 0..50 {
                    let x = (i % 10) as f64 * 2.5;
                    let y = (i / 10) as f64 * 2.5 + 5.0;
                    world.add_sphere(Point::new([x, y]), 0.5, 1.0);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ═══ New shape benchmarks ═══

fn bench_gjk_capsule_capsule(c: &mut Criterion) {
    let a = Capsule::<3>::y_aligned(1.0, 0.5);
    let b = Capsule::<3>::y_aligned(1.0, 0.5);
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_capsule_capsule_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_hyperbox_hyperbox(c: &mut Criterion) {
    let a = HyperBox::<3>::cube(1.0);
    let b = HyperBox::<3>::cube(1.0);
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0]);

    c.bench_function("gjk_hyperbox_hyperbox_3d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

fn bench_gjk_hyperbox_4d(c: &mut Criterion) {
    let a = HyperBox::<4>::cube(1.0);
    let b = HyperBox::<4>::cube(1.0);
    let pa = SVector::from([0.0, 0.0, 0.0, 0.0]);
    let pb = SVector::from([1.5, 0.0, 0.0, 0.0]);

    c.bench_function("gjk_hyperbox_4d", |bench| {
        bench.iter(|| {
            black_box(gjk::intersects(&a, &pa, &b, &pb));
        });
    });
}

// ═══ EPA benchmarks ═══

fn bench_epa_sphere_sphere(c: &mut Criterion) {
    let a = Sphere::<3>::unit();
    let b = Sphere::<3>::unit();
    let pa = SVector::from([0.0, 0.0, 0.0]);
    let pb = SVector::from([0.5, 0.0, 0.0]);

    c.bench_function("epa_sphere_sphere_3d", |bench| {
        let result = gjk::intersects(&a, &pa, &b, &pb);
        bench.iter(|| {
            black_box(symtropy_physics::epa::penetration(
                &a,
                &pa,
                &b,
                &pb,
                &result.simplex,
            ));
        });
    });
}

// ═══ Raycast benchmarks ═══

fn bench_raycast_100_bodies(c: &mut Criterion) {
    c.bench_function("raycast_100_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::zeros());
                for i in 0..100 {
                    let x = (i % 10) as f64 * 3.0;
                    let z = (i / 10) as f64 * 3.0;
                    world.add_sphere(Point::new([x, 0.0, z]), 1.0, 1.0);
                }
                world
            },
            |world| {
                let origin = SVector::from([-5.0, 0.0, 15.0]);
                let dir = SVector::from([1.0, 0.0, 0.0]);
                black_box(raycast::raycast(&world, &origin, &dir, 100.0));
            },
            criterion::BatchSize::SmallInput,
        );
    });
}

// ═══ Stress tests ═══

fn bench_physics_step_500_bodies(c: &mut Criterion) {
    c.bench_function("physics_step_500_bodies_3d", |bench| {
        bench.iter_batched(
            || {
                let mut world = PhysicsWorld::<3>::new(SVector::from([0.0, -9.81, 0.0]));
                for i in 0..500 {
                    let x = (i % 25) as f64 * 2.5;
                    let y = (i / 25) as f64 * 2.5 + 5.0;
                    world.add_sphere(Point::new([x, y, 0.0]), 0.5, 1.0);
                }
                world
            },
            |mut world| {
                world.step(0.016);
                black_box(());
            },
            criterion::BatchSize::LargeInput,
        );
    });
}

// ═══ LBVH structure-quality diagnostics ═══

fn bench_lbvh_structure_diagnostics(c: &mut Criterion) {
    let mut group = c.benchmark_group("lbvh_structure_diagnostics_3d");

    for body_count in [128usize, 512, 2048] {
        let bodies: Vec<_> = (0..body_count)
            .map(|index| {
                let x = (index % 64) as f64 * 1.25;
                let y = ((index / 64) % 32) as f64 * 1.25;
                let z = (index / 2048) as f64 * 1.25;
                RigidBody::<3>::dynamic_sphere(BodyHandle(index), Point::new([x, y, z]), 0.5, 1.0)
            })
            .collect();
        let tree = Lbvh::build(&bodies);
        let receipt = tree.structure_stats();
        assert_eq!(receipt.body_count, body_count);
        assert_eq!(receipt.internal_node_count, body_count - 1);

        group.bench_function(BenchmarkId::new("structure_stats", body_count), |bench| {
            bench.iter(|| {
                black_box(tree.structure_stats());
            });
        });
    }

    group.finish();
}

criterion_group!(
    benches,
    // GJK per shape
    bench_gjk_sphere_sphere,
    bench_gjk_box_box,
    bench_gjk_capsule_capsule,
    bench_gjk_hyperbox_hyperbox,
    bench_gjk_hyperbox_4d,
    bench_gjk_4d_tesseract,
    // EPA
    bench_epa_sphere_sphere,
    // Raycast
    bench_raycast_100_bodies,
    // Physics step at scale
    bench_physics_step_10_bodies,
    bench_physics_step_100_bodies,
    bench_physics_step_500_bodies,
    bench_physics_step_2d,
    // Tree structure quality/memory diagnostics outside the simulation hot path.
    bench_lbvh_structure_diagnostics,
);
criterion_main!(benches);
