// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::{Capsule, ConvexHull, HyperBox, Point, Sphere, Transform};
use symtropy_physics::broadphase::{self, BroadphasePair, StaticBroadphase};
use symtropy_physics::gjk;
use symtropy_physics::raycast;
use symtropy_physics::{BodyHandle, BodyType, PhysicsWorld, RigidBody};

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

// ═══ Static-heavy broadphase baselines ═══

fn static_heavy_broadphase_scene(static_count: usize, moving_count: usize) -> Vec<RigidBody<3>> {
    let mut bodies = Vec::with_capacity(static_count + moving_count);

    // Static bodies are widely separated. Each moving body is colocated with
    // one selected static body, producing a small, deterministic candidate set.
    for index in 0..static_count {
        let x = index as f64 * 3.0;
        bodies.push(RigidBody::<3>::static_body(
            BodyHandle(index),
            Point::new([x, 0.0, 0.0]),
            Box::new(HyperBox::<3>::cube(0.5)),
        ));
    }

    for index in 0..moving_count {
        let static_index = index * static_count / moving_count.max(1);
        let x = static_index as f64 * 3.0;
        bodies.push(RigidBody::<3>::new(
            BodyHandle(static_count + index),
            BodyType::Dynamic,
            Transform::from_translation(Point::new([x, 0.0, 0.0])),
            Box::new(HyperBox::<3>::cube(0.5)),
            1.0,
            SVector::from_element(1.0),
        ));
    }

    bodies
}

#[derive(Clone, Copy)]
struct OracleAabb {
    min: SVector<f64, 3>,
    max: SVector<f64, 3>,
}

// This fixture-specific reference deliberately computes a HyperBox AABB by
// transforming all eight vertices. It does not call support_aabb or
// Aabb::from_body, so a support-map/AABB bug cannot be shared by the oracle.
fn oracle_hyperbox_aabb(body: &RigidBody<3>) -> OracleAabb {
    let cube = body
        .collider
        .as_any()
        .downcast_ref::<HyperBox<3>>()
        .expect("static-heavy oracle fixture uses HyperBox colliders");
    assert_eq!(cube.half_extents, [0.5; 3]);

    let mut min = SVector::<f64, 3>::from_element(f64::INFINITY);
    let mut max = SVector::<f64, 3>::from_element(f64::NEG_INFINITY);

    for corner in 0..8 {
        let local = Point::new(std::array::from_fn(|axis| {
            if corner & (1 << axis) == 0 {
                -cube.half_extents[axis]
            } else {
                cube.half_extents[axis]
            }
        }));
        let world = body.transform.transform_point(&local).0;

        for axis in 0..3 {
            min[axis] = min[axis].min(world[axis]);
            max[axis] = max[axis].max(world[axis]);
        }
    }

    OracleAabb { min, max }
}

fn oracle_aabbs_overlap(a: &OracleAabb, b: &OracleAabb) -> bool {
    for axis in 0..3 {
        if a.max[axis] < b.min[axis] || a.min[axis] > b.max[axis] {
            return false;
        }
    }
    true
}

fn exhaustive_aabb_pair_oracle(bodies: &[RigidBody<3>]) -> Vec<BroadphasePair> {
    let aabbs: Vec<_> = bodies.iter().map(oracle_hyperbox_aabb).collect();
    let mut pairs = Vec::new();

    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            let a = &bodies[i];
            let b = &bodies[j];

            if a.body_type == BodyType::Static && b.body_type == BodyType::Static {
                continue;
            }

            let filters_allow_pair = (a.collision_group & b.collision_mask != 0)
                && (b.collision_group & a.collision_mask != 0);
            if !filters_allow_pair || !oracle_aabbs_overlap(&aabbs[i], &aabbs[j]) {
                continue;
            }

            let (handle_a, handle_b) = if a.handle < b.handle {
                (a.handle, b.handle)
            } else {
                (b.handle, a.handle)
            };
            pairs.push(BroadphasePair(handle_a, handle_b));
        }
    }

    pairs.sort_unstable_by_key(|pair| (pair.0, pair.1));
    pairs.dedup();
    pairs
}

fn bench_broadphase_static_heavy(c: &mut Criterion) {
    let mut group = c.benchmark_group("broadphase_static_heavy_3d");

    // Keep moving counts below the incremental path's 50-body fallback so
    // these measurements actually exercise the flat static-cache scan.
    for static_count in [64usize, 256, 1024, 4096] {
        for moving_count in [1usize, 8, 32] {
            let bodies = static_heavy_broadphase_scene(static_count, moving_count);

            let mut static_cache = StaticBroadphase::new();
            static_cache.rebuild(&bodies);

            // Exhaustively enumerate all AABB pairs, independently of the
            // LBVH traversal and incremental-cache traversal. Then require
            // both optimized paths to match this one-sided completeness oracle.
            let oracle_pairs = exhaustive_aabb_pair_oracle(&bodies);
            let full_pairs = broadphase::find_pairs(&bodies);
            let incremental_pairs = broadphase::find_pairs_incremental(&bodies, &static_cache);
            assert_eq!(
                full_pairs, oracle_pairs,
                "LBVH candidate mismatch for static={static_count}, moving={moving_count}"
            );
            assert_eq!(
                incremental_pairs, oracle_pairs,
                "incremental candidate mismatch for static={static_count}, moving={moving_count}"
            );

            let case = format!(
                "static_{static_count}_moving_{moving_count}_pairs_{}",
                oracle_pairs.len()
            );

            group.bench_function(
                BenchmarkId::new("flat_static_cache_query", &case),
                |bench| {
                    bench.iter(|| {
                        black_box(broadphase::find_pairs_incremental(
                            black_box(&bodies),
                            black_box(&static_cache),
                        ));
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("full_lbvh_rebuild_and_query", &case),
                |bench| {
                    bench.iter(|| {
                        black_box(broadphase::find_pairs(black_box(&bodies)));
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("static_cache_rebuild_only", &case),
                |bench| {
                    bench.iter(|| {
                        let mut cache = StaticBroadphase::new();
                        cache.rebuild(black_box(&bodies));
                        black_box(cache.len());
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("incremental_rebuild_plus_query", &case),
                |bench| {
                    bench.iter(|| {
                        let mut cache = StaticBroadphase::new();
                        cache.rebuild(black_box(&bodies));
                        black_box(broadphase::find_pairs_incremental(
                            black_box(&bodies),
                            black_box(&cache),
                        ));
                    });
                },
            );
        }
    }

    group.finish();
}

// Clustered fixtures intentionally maximize AABB overlap. They expose the
// output-sensitive worst case rather than allowing a sparse-only benchmark to
// stand in for broadphase performance across all spatial distributions.
fn clustered_static_heavy_broadphase_scene(
    static_count: usize,
    moving_count: usize,
) -> Vec<RigidBody<3>> {
    let mut bodies = Vec::with_capacity(static_count + moving_count);

    for index in 0..static_count {
        let x = (index % 4) as f64 * 0.2;
        let y = ((index / 4) % 4) as f64 * 0.2;
        let z = ((index / 16) % 4) as f64 * 0.2;
        bodies.push(RigidBody::<3>::static_body(
            BodyHandle(index),
            Point::new([x, y, z]),
            Box::new(HyperBox::<3>::cube(0.5)),
        ));
    }

    for index in 0..moving_count {
        let x = (index % 4) as f64 * 0.1 + 0.05;
        let y = ((index / 4) % 4) as f64 * 0.1 + 0.05;
        let z = ((index / 16) % 4) as f64 * 0.1 + 0.05;
        bodies.push(RigidBody::<3>::new(
            BodyHandle(static_count + index),
            BodyType::Dynamic,
            Transform::from_translation(Point::new([x, y, z])),
            Box::new(HyperBox::<3>::cube(0.5)),
            1.0,
            SVector::from_element(1.0),
        ));
    }

    bodies
}

fn bench_broadphase_stress_matrix(c: &mut Criterion) {
    let mut group = c.benchmark_group("broadphase_distribution_and_crossover_3d");

    // Dense clusters are the output-sensitive case: nearly every dynamic body
    // overlaps every cached static body. The exact expected pair set is still
    // generated by the independent eight-vertex HyperBox oracle above.
    for static_count in [128usize, 512] {
        for moving_count in [1usize, 8, 32] {
            let bodies = clustered_static_heavy_broadphase_scene(static_count, moving_count);
            let mut static_cache = StaticBroadphase::new();
            static_cache.rebuild(&bodies);

            let oracle_pairs = exhaustive_aabb_pair_oracle(&bodies);
            let full_pairs = broadphase::find_pairs(&bodies);
            let incremental_pairs = broadphase::find_pairs_incremental(&bodies, &static_cache);
            assert_eq!(
                full_pairs, oracle_pairs,
                "LBVH candidate mismatch for clustered static={static_count}, moving={moving_count}"
            );
            assert_eq!(
                incremental_pairs, oracle_pairs,
                "incremental candidate mismatch for clustered static={static_count}, moving={moving_count}"
            );

            let case = format!(
                "clustered_static_{static_count}_moving_{moving_count}_pairs_{}",
                oracle_pairs.len()
            );

            group.bench_function(
                BenchmarkId::new("flat_static_cache_query", &case),
                |bench| {
                    bench.iter(|| {
                        black_box(broadphase::find_pairs_incremental(
                            black_box(&bodies),
                            black_box(&static_cache),
                        ));
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("full_lbvh_rebuild_and_query", &case),
                |bench| {
                    bench.iter(|| {
                        black_box(broadphase::find_pairs(black_box(&bodies)));
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("static_cache_rebuild_only", &case),
                |bench| {
                    bench.iter(|| {
                        let mut cache = StaticBroadphase::new();
                        cache.rebuild(black_box(&bodies));
                        black_box(cache.len());
                    });
                },
            );
        }
    }

    // Cross the implementation's current 50-dynamic-body fallback. These
    // cases make the crossover visible instead of accidentally calling the
    // flat-cache branch "incremental" when it has actually fallen back to a
    // full LBVH over the entire scene.
    for static_count in [64usize, 256] {
        for moving_count in [64usize, 128] {
            let bodies = static_heavy_broadphase_scene(static_count, moving_count);
            let mut static_cache = StaticBroadphase::new();
            static_cache.rebuild(&bodies);

            let oracle_pairs = exhaustive_aabb_pair_oracle(&bodies);
            let full_pairs = broadphase::find_pairs(&bodies);
            let incremental_pairs = broadphase::find_pairs_incremental(&bodies, &static_cache);
            assert_eq!(
                full_pairs, oracle_pairs,
                "LBVH candidate mismatch for fallback static={static_count}, moving={moving_count}"
            );
            assert_eq!(
                incremental_pairs, oracle_pairs,
                "incremental fallback mismatch for static={static_count}, moving={moving_count}"
            );

            let case = format!(
                "fallback_static_{static_count}_moving_{moving_count}_pairs_{}",
                oracle_pairs.len()
            );

            group.bench_function(
                BenchmarkId::new("incremental_dynamic_threshold_fallback", &case),
                |bench| {
                    bench.iter(|| {
                        black_box(broadphase::find_pairs_incremental(
                            black_box(&bodies),
                            black_box(&static_cache),
                        ));
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("full_lbvh_rebuild_and_query", &case),
                |bench| {
                    bench.iter(|| {
                        black_box(broadphase::find_pairs(black_box(&bodies)));
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("static_cache_rebuild_only", &case),
                |bench| {
                    bench.iter(|| {
                        let mut cache = StaticBroadphase::new();
                        cache.rebuild(black_box(&bodies));
                        black_box(cache.len());
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("static_cache_rebuild_plus_incremental_fallback", &case),
                |bench| {
                    bench.iter(|| {
                        let mut cache = StaticBroadphase::new();
                        cache.rebuild(black_box(&bodies));
                        black_box(broadphase::find_pairs_incremental(
                            black_box(&bodies),
                            black_box(&cache),
                        ));
                    });
                },
            );
        }
    }

    group.finish();
}

fn bench_broadphase_temporal_static_reuse(c: &mut Criterion) {
    const FRAMES: usize = 16;
    let mut group = c.benchmark_group("broadphase_temporal_static_reuse_3d");

    // The static cache is built once and reused while dynamic bodies move
    // smoothly across static geometry. This measures amortized multi-tick
    // cost, not merely a favorable frozen-scene query.
    for static_count in [256usize, 1024] {
        for moving_count in [8usize, 32] {
            let frames: Vec<Vec<RigidBody<3>>> = (0..FRAMES)
                .map(|frame| {
                    let mut bodies = static_heavy_broadphase_scene(static_count, moving_count);
                    for moving_index in 0..moving_count {
                        let anchor = moving_index * static_count / moving_count.max(1);
                        let x = anchor as f64 * 3.0 + frame as f64 * 0.2 - 1.5;
                        let body = &mut bodies[static_count + moving_index];
                        body.transform = Transform::from_translation(Point::new([x, 0.0, 0.0]));
                    }
                    bodies
                })
                .collect();

            let mut static_cache = StaticBroadphase::new();
            static_cache.rebuild(&frames[0]);

            let mut total_candidates = 0usize;
            for (frame_index, bodies) in frames.iter().enumerate() {
                let oracle_pairs = exhaustive_aabb_pair_oracle(bodies);
                let full_pairs = broadphase::find_pairs(bodies);
                let incremental_pairs = broadphase::find_pairs_incremental(bodies, &static_cache);
                assert_eq!(
                    full_pairs, oracle_pairs,
                    "temporal LBVH mismatch at frame {frame_index}, static={static_count}, moving={moving_count}"
                );
                assert_eq!(
                    incremental_pairs, oracle_pairs,
                    "temporal incremental mismatch at frame {frame_index}, static={static_count}, moving={moving_count}"
                );
                total_candidates += oracle_pairs.len();
            }

            let case = format!(
                "static_{static_count}_moving_{moving_count}_frames_{FRAMES}_candidate_events_{total_candidates}"
            );

            group.bench_function(
                BenchmarkId::new("cached_static_multi_tick_queries", &case),
                |bench| {
                    bench.iter(|| {
                        let mut candidates = 0usize;
                        for bodies in &frames {
                            candidates += broadphase::find_pairs_incremental(
                                black_box(bodies),
                                black_box(&static_cache),
                            )
                            .len();
                        }
                        black_box(candidates);
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("full_lbvh_multi_tick_rebuild_queries", &case),
                |bench| {
                    bench.iter(|| {
                        let mut candidates = 0usize;
                        for bodies in &frames {
                            candidates += broadphase::find_pairs(black_box(bodies)).len();
                        }
                        black_box(candidates);
                    });
                },
            );

            group.bench_function(
                BenchmarkId::new("cache_rebuild_once_plus_multi_tick_queries", &case),
                |bench| {
                    bench.iter(|| {
                        let mut cache = StaticBroadphase::new();
                        cache.rebuild(black_box(&frames[0]));
                        let mut candidates = 0usize;
                        for bodies in &frames {
                            candidates += broadphase::find_pairs_incremental(
                                black_box(bodies),
                                black_box(&cache),
                            )
                            .len();
                        }
                        black_box(candidates);
                    });
                },
            );
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
    // Broadphase static-heavy baselines
    bench_broadphase_static_heavy,
    // Dense distributions and dynamic-count crossover/fallback
    bench_broadphase_stress_matrix,
    // Amortized static-cache reuse across moving multi-tick scenes
    bench_broadphase_temporal_static_reuse,
);
criterion_main!(benches);
