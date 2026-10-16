// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::{Capsule, ConvexHull, HyperBox, Point, Sphere};
use symtropy_physics::PhysicsWorld;
use symtropy_physics::broadphase::{
    BroadphasePair, StaticBroadphase, find_pairs, find_pairs_incremental,
};
use symtropy_physics::gjk;
use symtropy_physics::raycast;
use symtropy_physics::{BodyHandle, RigidBody};

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

// ═══ Static-heavy broadphase crossover ═══

#[derive(Clone, Copy)]
enum StaticBroadphaseLayout {
    Grid,
    Clustered,
}

impl StaticBroadphaseLayout {
    fn label(self) -> &'static str {
        match self {
            Self::Grid => "grid",
            Self::Clustered => "clustered",
        }
    }
}

fn static_heavy_position(
    index: usize,
    static_count: usize,
    layout: StaticBroadphaseLayout,
) -> Point<3> {
    match layout {
        StaticBroadphaseLayout::Grid => {
            let side = (static_count as f64).sqrt().ceil() as usize;
            Point::new([
                (index % side) as f64 * 3.0,
                0.0,
                (index / side) as f64 * 3.0,
            ])
        }
        StaticBroadphaseLayout::Clustered => {
            let cluster = index % 8;
            let local = index / 8;
            Point::new([
                (cluster % 4) as f64 * 80.0 + (local % 40) as f64 * 0.75,
                0.0,
                (cluster / 4) as f64 * 80.0 + (local / 40) as f64 * 0.75,
            ])
        }
    }
}

fn static_heavy_fixture(
    static_count: usize,
    dynamic_count: usize,
    layout: StaticBroadphaseLayout,
) -> (Vec<RigidBody<3>>, StaticBroadphase<3>) {
    let mut bodies = Vec::with_capacity(static_count + dynamic_count);
    for index in 0..static_count {
        bodies.push(RigidBody::<3>::static_body(
            BodyHandle(index),
            static_heavy_position(index, static_count, layout),
            Box::new(Sphere::new(Point::origin(), 0.5)),
        ));
    }

    // Active bodies overlap selected static bodies. The same deterministic
    // fixture is reused by both implementations; no RNG or setup work runs
    // inside the timed closures.
    for index in 0..dynamic_count {
        let target_index = ((index + 1) * static_count) / (dynamic_count + 1);
        bodies.push(RigidBody::<3>::dynamic_sphere(
            BodyHandle(static_count + index),
            static_heavy_position(target_index, static_count, layout),
            0.5,
            1.0,
        ));
    }

    let mut static_bp = StaticBroadphase::new();
    static_bp.rebuild(&bodies);
    (bodies, static_bp)
}

fn canonical_broadphase_pairs(mut pairs: Vec<BroadphasePair>) -> Vec<(BodyHandle, BodyHandle)> {
    assert!(
        pairs.iter().all(|pair| pair.0 < pair.1),
        "broadphase pair handles must be normalized"
    );
    pairs.sort_unstable_by_key(|pair| (pair.0, pair.1));
    assert!(
        pairs
            .windows(2)
            .all(|window| (window[0].0, window[0].1) < (window[1].0, window[1].1)),
        "broadphase returned duplicate pairs"
    );
    pairs.into_iter().map(|pair| (pair.0, pair.1)).collect()
}

fn bench_broadphase_static_heavy_crossover(c: &mut Criterion) {
    let mut group = c.benchmark_group("broadphase_static_heavy");
    for layout in [
        StaticBroadphaseLayout::Grid,
        StaticBroadphaseLayout::Clustered,
    ] {
        for static_count in [1_000usize, 10_000usize] {
            for dynamic_count in [8usize, 32usize] {
                let (bodies, static_bp) = static_heavy_fixture(static_count, dynamic_count, layout);
                let full_pairs = canonical_broadphase_pairs(find_pairs(&bodies));
                let incremental_pairs =
                    canonical_broadphase_pairs(find_pairs_incremental(&bodies, &static_bp));
                assert_eq!(
                    incremental_pairs,
                    full_pairs,
                    "incremental/full-LBVH pair mismatch: {} static, {} dynamic, {} layout",
                    static_count,
                    dynamic_count,
                    layout.label(),
                );

                let case = format!(
                    "{}_static_{}_dynamic_{}",
                    layout.label(),
                    static_count,
                    dynamic_count
                );
                group.bench_function(BenchmarkId::new("full_lbvh", &case), |b| {
                    b.iter(|| black_box(find_pairs(black_box(&bodies))));
                });
                group.bench_function(BenchmarkId::new("incremental_steady", &case), |b| {
                    b.iter(|| {
                        black_box(find_pairs_incremental(
                            black_box(&bodies),
                            black_box(&static_bp),
                        ))
                    });
                });
                group.bench_function(BenchmarkId::new("static_cache_rebuild", &case), |b| {
                    let mut cache = StaticBroadphase::new();
                    cache.rebuild(&bodies);
                    b.iter(|| {
                        cache.rebuild(black_box(&bodies));
                        black_box(cache.len());
                    });
                });
            }
        }
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
    // Static-heavy broadphase: exact pair-set equivalence is checked before timing.
    bench_broadphase_static_heavy_crossover,
);
criterion_main!(benches);
