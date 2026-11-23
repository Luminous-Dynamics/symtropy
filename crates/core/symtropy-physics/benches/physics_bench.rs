// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector, SymmetricEigen};
use symtropy_math::{Bivector, Capsule, ConvexHull, HyperBox, Point, Rotor, Sphere, Transform};
use symtropy_physics::broadphase::{self, BroadphasePair};
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

// ═══ Principal-axis sweep-and-prune reference benchmark ═══
//
// This is a benchmark-only comparator inspired by MuJoCo's documented
// principal-covariance-axis sweep-and-prune broadphase. It does not alter the
// production broadphase. Every result is checked against a transformed-vertex
// HyperBox oracle before Criterion timing begins.

#[derive(Clone, Copy)]
struct PcaOracleAabb {
    min: SVector<f64, 3>,
    max: SVector<f64, 3>,
}

fn pca_oracle_aabb(body: &RigidBody<3>) -> PcaOracleAabb {
    let shape = body
        .collider
        .as_any()
        .downcast_ref::<HyperBox<3>>()
        .expect("PCA-SAP reference scenes contain HyperBox colliders");

    let mut min = SVector::<f64, 3>::from_element(f64::INFINITY);
    let mut max = SVector::<f64, 3>::from_element(f64::NEG_INFINITY);
    for corner in 0..8 {
        let local = Point::new(std::array::from_fn(|axis| {
            if corner & (1 << axis) == 0 {
                -shape.half_extents[axis]
            } else {
                shape.half_extents[axis]
            }
        }));
        let world = body.transform.transform_point(&local).0;
        for axis in 0..3 {
            min[axis] = min[axis].min(world[axis]);
            max[axis] = max[axis].max(world[axis]);
        }
    }
    PcaOracleAabb { min, max }
}

fn pca_oracle_pairs(bodies: &[RigidBody<3>]) -> Vec<BroadphasePair> {
    let bounds: Vec<_> = bodies.iter().map(pca_oracle_aabb).collect();
    let mut pairs = Vec::new();

    for i in 0..bodies.len() {
        for j in (i + 1)..bodies.len() {
            let a = &bodies[i];
            let b = &bodies[j];
            if a.body_type == BodyType::Static && b.body_type == BodyType::Static {
                continue;
            }
            if (a.collision_group & b.collision_mask == 0)
                || (b.collision_group & a.collision_mask == 0)
            {
                continue;
            }

            let mut overlaps = true;
            for axis in 0..3 {
                if bounds[i].max[axis] < bounds[j].min[axis]
                    || bounds[j].max[axis] < bounds[i].min[axis]
                {
                    overlaps = false;
                    break;
                }
            }
            if overlaps {
                let (left, right) = if a.handle < b.handle {
                    (a.handle, b.handle)
                } else {
                    (b.handle, a.handle)
                };
                pairs.push(BroadphasePair(left, right));
            }
        }
    }

    pairs.sort_unstable_by_key(|pair| (pair.0, pair.1));
    pairs.dedup();
    pairs
}

fn pca_sap_scene(static_count: usize, dynamic_count: usize, clustered: bool) -> Vec<RigidBody<3>> {
    let mut bodies = Vec::with_capacity(static_count + dynamic_count);

    for index in 0..static_count {
        let position = if clustered {
            Point::new([
                (index % 4) as f64 * 0.2,
                ((index / 4) % 4) as f64 * 0.2,
                ((index / 16) % 4) as f64 * 0.2,
            ])
        } else {
            Point::new([index as f64 * 3.0, 0.0, 0.0])
        };
        let transform = Transform::from_translation(position);
        bodies.push(RigidBody::<3>::new(
            BodyHandle(index),
            BodyType::Static,
            transform,
            Box::new(HyperBox::<3>::cube(0.5)),
            0.0,
            SVector::zeros(),
        ));
    }

    for index in 0..dynamic_count {
        let anchor = index * static_count / dynamic_count.max(1);
        let position = if clustered {
            Point::new([
                (index % 4) as f64 * 0.1 + 0.05,
                ((index / 4) % 4) as f64 * 0.1 + 0.05,
                ((index / 16) % 4) as f64 * 0.1 + 0.05,
            ])
        } else {
            Point::new([anchor as f64 * 3.0, 0.0, 0.0])
        };
        let rotation = Rotor::from_plane_angle(
            &Bivector::unit_plane(0, 1),
            if index % 2 == 0 { 0.17 } else { -0.11 },
        );
        bodies.push(RigidBody::<3>::new(
            BodyHandle(static_count + index),
            BodyType::Dynamic,
            Transform {
                translation: position,
                rotation,
            },
            Box::new(HyperBox::<3>::cube(0.5)),
            1.0,
            SVector::from_element(1.0),
        ));
    }

    bodies
}

struct PcaSapEntry {
    body_index: usize,
    min_projection: f64,
    max_projection: f64,
    aabb: broadphase::Aabb<3>,
}

struct PcaSap {
    axis: SVector<f64, 3>,
    entries: Vec<PcaSapEntry>,
}

impl PcaSap {
    fn principal_axis(centers: &[SVector<f64, 3>]) -> SVector<f64, 3> {
        let fallback = SVector::<f64, 3>::new(1.0, 0.0, 0.0);
        if centers.len() < 2 {
            return fallback;
        }

        let centroid = centers
            .iter()
            .copied()
            .fold(SVector::<f64, 3>::zeros(), |sum, center| sum + center)
            / centers.len() as f64;
        let mut covariance = SMatrix::<f64, 3, 3>::zeros();
        for center in centers {
            let delta = *center - centroid;
            covariance += delta * delta.transpose();
        }

        let decomposition = SymmetricEigen::new(covariance);
        let principal_index = (0..3)
            .max_by(|left, right| {
                decomposition.eigenvalues[*left].total_cmp(&decomposition.eigenvalues[*right])
            })
            .unwrap_or(0);
        let mut axis = decomposition
            .eigenvectors
            .column(principal_index)
            .into_owned();
        let norm = axis.norm();
        if !norm.is_finite() || norm <= f64::EPSILON {
            return fallback;
        }
        axis /= norm;

        // Fix the sign ambiguity of eigenvectors for deterministic ordering.
        if let Some(pivot) = axis.iter().position(|component| component.abs() > 1e-12) {
            if axis[pivot] < 0.0 {
                axis = -axis;
            }
        }
        axis
    }

    fn build(bodies: &[RigidBody<3>]) -> Self {
        let aabbs: Vec<_> = bodies.iter().map(broadphase::Aabb::from_body).collect();
        let centers: Vec<_> = aabbs
            .iter()
            .map(|aabb| (aabb.min + aabb.max) * 0.5)
            .collect();
        let axis = Self::principal_axis(&centers);

        let mut entries: Vec<_> = aabbs
            .into_iter()
            .enumerate()
            .map(|(body_index, aabb)| {
                let center = (aabb.min + aabb.max) * 0.5;
                let half_extent = (aabb.max - aabb.min) * 0.5;
                let absolute_axis =
                    SVector::<f64, 3>::new(axis[0].abs(), axis[1].abs(), axis[2].abs());
                let projection_center = center.dot(&axis);
                let projection_radius = absolute_axis.dot(&half_extent);
                PcaSapEntry {
                    body_index,
                    min_projection: projection_center - projection_radius,
                    max_projection: projection_center + projection_radius,
                    aabb,
                }
            })
            .collect();

        entries.sort_by(|left, right| {
            left.min_projection
                .total_cmp(&right.min_projection)
                .then_with(|| {
                    bodies[left.body_index]
                        .handle
                        .cmp(&bodies[right.body_index].handle)
                })
        });

        Self { axis, entries }
    }

    fn query_pairs(&self, bodies: &[RigidBody<3>]) -> Vec<BroadphasePair> {
        let mut active: Vec<usize> = Vec::new();
        let mut pairs = Vec::new();

        for current_index in 0..self.entries.len() {
            let current = &self.entries[current_index];
            active.retain(|&active_index| {
                self.entries[active_index].max_projection >= current.min_projection
            });

            for &active_index in &active {
                let previous = &self.entries[active_index];
                let a = &bodies[previous.body_index];
                let b = &bodies[current.body_index];

                if a.body_type == BodyType::Static && b.body_type == BodyType::Static {
                    continue;
                }
                if (a.collision_group & b.collision_mask == 0)
                    || (b.collision_group & a.collision_mask == 0)
                {
                    continue;
                }
                if !previous.aabb.overlaps(&current.aabb) {
                    continue;
                }

                let (left, right) = if a.handle < b.handle {
                    (a.handle, b.handle)
                } else {
                    (b.handle, a.handle)
                };
                pairs.push(BroadphasePair(left, right));
            }

            active.push(current_index);
        }

        pairs.sort_unstable_by_key(|pair| (pair.0, pair.1));
        pairs.dedup();
        pairs
    }
}

fn bench_broadphase_pca_sap_reference(c: &mut Criterion) {
    let mut group = c.benchmark_group("broadphase_pca_sap_reference_3d");
    let scenarios = [
        ("sparse_256_static_16_dynamic", 256usize, 16usize, false),
        ("clustered_128_static_24_dynamic", 128, 24, true),
        ("sparse_64_static_64_dynamic", 64, 64, false),
    ];

    for (name, static_count, dynamic_count, clustered) in scenarios {
        let bodies = pca_sap_scene(static_count, dynamic_count, clustered);
        let expected = pca_oracle_pairs(&bodies);
        let sap = PcaSap::build(&bodies);
        assert_eq!(
            sap.query_pairs(&bodies),
            expected,
            "PCA-SAP mismatch in {name}, axis={:?}",
            sap.axis
        );
        assert_eq!(
            broadphase::find_pairs(&bodies),
            expected,
            "LBVH/oracle mismatch in {name}"
        );

        group.bench_function(BenchmarkId::new("pca_sap_build_only", name), |bench| {
            bench.iter(|| {
                black_box(PcaSap::build(black_box(&bodies)));
            });
        });
        group.bench_function(BenchmarkId::new("pca_sap_query_only", name), |bench| {
            bench.iter(|| {
                black_box(sap.query_pairs(black_box(&bodies)));
            });
        });
        group.bench_function(
            BenchmarkId::new("pca_sap_build_plus_query", name),
            |bench| {
                bench.iter(|| {
                    let candidate = PcaSap::build(black_box(&bodies));
                    black_box(candidate.query_pairs(black_box(&bodies)));
                });
            },
        );
        group.bench_function(
            BenchmarkId::new("full_lbvh_rebuild_plus_query", name),
            |bench| {
                bench.iter(|| {
                    black_box(broadphase::find_pairs(black_box(&bodies)));
                });
            },
        );
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
    // Principal-axis sweep-and-prune baseline, checked against an independent
    // transformed-vertex AABB pair oracle before Criterion timings.
    bench_broadphase_pca_sap_reference,
);
criterion_main!(benches);
