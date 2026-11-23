// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector, SymmetricEigen};
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

#[derive(Clone, Copy)]
struct BenchStaticEntry {
    body_index: usize,
    aabb: OracleAabb,
}

#[derive(Clone, Copy)]
struct BenchStaticBvhNode {
    aabb: OracleAabb,
    left: Option<usize>,
    right: Option<usize>,
    leaf_start: usize,
    leaf_end: usize,
}

/// Benchmark-only static hierarchy prototype. It is intentionally separate
/// from production broadphase code so its build/query tradeoffs can be
/// measured before any production algorithm is selected.
struct BenchStaticBvh {
    nodes: Vec<BenchStaticBvhNode>,
    ordered_body_indices: Vec<usize>,
    ordered_aabbs: Vec<OracleAabb>,
    root: Option<usize>,
}

impl BenchStaticBvh {
    const LEAF_SIZE: usize = 8;

    fn build(bodies: &[RigidBody<3>]) -> Self {
        let entries: Vec<_> = bodies
            .iter()
            .enumerate()
            .filter(|(_, body)| body.body_type == BodyType::Static)
            .map(|(body_index, body)| BenchStaticEntry {
                body_index,
                aabb: oracle_hyperbox_aabb(body),
            })
            .collect();

        let mut tree = Self {
            nodes: Vec::with_capacity(entries.len().saturating_mul(2)),
            ordered_body_indices: Vec::with_capacity(entries.len()),
            ordered_aabbs: Vec::with_capacity(entries.len()),
            root: None,
        };

        if !entries.is_empty() {
            let mut entries = entries;
            tree.root = Some(tree.build_node(&mut entries, bodies));
        }

        tree
    }

    fn build_node(&mut self, entries: &mut [BenchStaticEntry], bodies: &[RigidBody<3>]) -> usize {
        let mut bounds = entries[0].aabb;
        for entry in &entries[1..] {
            bounds = oracle_aabb_union(&bounds, &entry.aabb);
        }

        let node_index = self.nodes.len();
        self.nodes.push(BenchStaticBvhNode {
            aabb: bounds,
            left: None,
            right: None,
            leaf_start: 0,
            leaf_end: 0,
        });

        if entries.len() <= Self::LEAF_SIZE {
            let start = self.ordered_body_indices.len();
            for entry in entries.iter() {
                self.ordered_body_indices.push(entry.body_index);
                self.ordered_aabbs.push(entry.aabb);
            }
            self.nodes[node_index].leaf_start = start;
            self.nodes[node_index].leaf_end = self.ordered_body_indices.len();
            return node_index;
        }

        let axis = (0..3)
            .max_by(|a, b| {
                let extent_a = bounds.max[*a] - bounds.min[*a];
                let extent_b = bounds.max[*b] - bounds.min[*b];
                extent_a.total_cmp(&extent_b)
            })
            .unwrap_or(0);

        entries.sort_by(|a, b| {
            let center_a = (a.aabb.min[axis] + a.aabb.max[axis]) * 0.5;
            let center_b = (b.aabb.min[axis] + b.aabb.max[axis]) * 0.5;
            center_a.total_cmp(&center_b).then_with(|| {
                bodies[a.body_index]
                    .handle
                    .cmp(&bodies[b.body_index].handle)
            })
        });

        let middle = entries.len() / 2;
        let (left_entries, right_entries) = entries.split_at_mut(middle);
        let left = self.build_node(left_entries, bodies);
        let right = self.build_node(right_entries, bodies);
        self.nodes[node_index].left = Some(left);
        self.nodes[node_index].right = Some(right);
        node_index
    }

    fn query_pairs(&self, bodies: &[RigidBody<3>]) -> Vec<BroadphasePair> {
        let dynamic: Vec<_> = bodies
            .iter()
            .enumerate()
            .filter(|(_, body)| body.body_type == BodyType::Dynamic)
            .map(|(index, body)| (index, oracle_hyperbox_aabb(body)))
            .collect();
        let mut pairs = Vec::new();

        // Dynamic-dynamic candidates remain a direct pair loop in this
        // prototype, matching the current incremental path's partition.
        for i in 0..dynamic.len() {
            for j in (i + 1)..dynamic.len() {
                let (body_a, aabb_a) = dynamic[i];
                let (body_b, aabb_b) = dynamic[j];
                if should_collide_bench(&bodies[body_a], &bodies[body_b])
                    && oracle_aabbs_overlap(&aabb_a, &aabb_b)
                {
                    pairs.push(canonical_broadphase_pair(
                        bodies[body_a].handle,
                        bodies[body_b].handle,
                    ));
                }
            }
        }

        if let Some(root) = self.root {
            // Reuse one bounded-depth traversal stack across dynamic bodies so
            // the query benchmark does not pay one heap allocation per body.
            let mut stack = Vec::with_capacity(64);
            for (dynamic_index, dynamic_aabb) in &dynamic {
                stack.clear();
                stack.push(root);
                while let Some(node_index) = stack.pop() {
                    let node = &self.nodes[node_index];
                    if !oracle_aabbs_overlap(dynamic_aabb, &node.aabb) {
                        continue;
                    }

                    match (node.left, node.right) {
                        (Some(left), Some(right)) => {
                            stack.push(left);
                            stack.push(right);
                        }
                        (None, None) => {
                            for leaf_index in node.leaf_start..node.leaf_end {
                                let static_index = self.ordered_body_indices[leaf_index];
                                let static_body = &bodies[static_index];
                                if should_collide_bench(&bodies[*dynamic_index], static_body)
                                    && oracle_aabbs_overlap(
                                        dynamic_aabb,
                                        &self.ordered_aabbs[leaf_index],
                                    )
                                {
                                    pairs.push(canonical_broadphase_pair(
                                        bodies[*dynamic_index].handle,
                                        static_body.handle,
                                    ));
                                }
                            }
                        }
                        _ => panic!("static BVH node has exactly one child"),
                    }
                }
            }
        }

        pairs.sort_unstable_by_key(|pair| (pair.0, pair.1));
        pairs.dedup();
        pairs
    }
}

fn oracle_aabb_union(a: &OracleAabb, b: &OracleAabb) -> OracleAabb {
    let mut min = a.min;
    let mut max = a.max;
    for axis in 0..3 {
        min[axis] = min[axis].min(b.min[axis]);
        max[axis] = max[axis].max(b.max[axis]);
    }
    OracleAabb { min, max }
}

fn should_collide_bench(a: &RigidBody<3>, b: &RigidBody<3>) -> bool {
    (a.collision_group & b.collision_mask != 0) && (b.collision_group & a.collision_mask != 0)
}

fn canonical_broadphase_pair(a: BodyHandle, b: BodyHandle) -> BroadphasePair {
    if a < b {
        BroadphasePair(a, b)
    } else {
        BroadphasePair(b, a)
    }
}

// ═══ Benchmark-only PCA-SAP baseline; not eligible for production selection without matched multi-family evidence ═══

#[derive(Clone)]
struct BenchPcaSapEntry {
    body_index: usize,
    min_projection: f64,
    max_projection: f64,
    aabb: broadphase::Aabb<3>,
}

/// Benchmark-only PCA sweep-and-prune implementation.
///
/// The sweep axis is the principal eigenvector of the covariance matrix of
/// world-AABB centers, matching the high-level axis-selection idea documented
/// by MuJoCo. Projection intervals are computed conservatively from each
/// world AABB, and full 3D AABB overlap is checked before emitting a pair.
///
/// This is a benchmark comparator, not production code and not a replacement
/// for the independent pair oracle.
struct BenchPcaSap {
    axis: SVector<f64, 3>,
    entries: Vec<BenchPcaSapEntry>,
}

impl BenchPcaSap {
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

        // Eigenvectors are sign-ambiguous. Canonicalize the first material
        // component so the chosen sweep direction is stable under replay.
        if let Some(pivot) = axis.iter().position(|component| component.abs() > 1e-12) {
            if axis[pivot] < 0.0 {
                axis = -axis;
            }
        }
        axis
    }

    fn build(bodies: &[RigidBody<3>]) -> Self {
        let aabbs: Vec<_> = bodies
            .iter()
            .map(broadphase::Aabb::from_body)
            .collect();
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
                let projection_center = center.dot(&axis);
                let projection_radius = axis.map(|component| component.abs()).dot(&half_extent);
                BenchPcaSapEntry {
                    body_index,
                    min_projection: projection_center - projection_radius,
                    max_projection: projection_center + projection_radius,
                    aabb,
                }
            })
            .collect();

        // Stable tie-breaking by body handle makes interval ordering
        // independent of incidental sort behavior.
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
                let body_a = &bodies[previous.body_index];
                let body_b = &bodies[current.body_index];

                if body_a.body_type == BodyType::Static
                    && body_b.body_type == BodyType::Static
                {
                    continue;
                }
                if (body_a.collision_group & body_b.collision_mask == 0)
                    || (body_b.collision_group & body_a.collision_mask == 0)
                {
                    continue;
                }
                if !previous.aabb.overlaps(&current.aabb) {
                    continue;
                }

                let (handle_a, handle_b) = if body_a.handle < body_b.handle {
                    (body_a.handle, body_b.handle)
                } else {
                    (body_b.handle, body_a.handle)
                };
                pairs.push(BroadphasePair(handle_a, handle_b));
            }

            active.push(current_index);
        }

        pairs.sort_unstable_by_key(|pair| (pair.0, pair.1));
        pairs.dedup();
        pairs
    }
}

fn bench_broadphase_static_bvh_tournament(c: &mut Criterion) {
    let mut group = c.benchmark_group("broadphase_static_bvh_tournament_3d");

    // The benchmark-only BVH is tested in both sparse and clustered scenes.
    // Every candidate implementation must match the independent pair oracle
    // before any timed iteration begins.
    let mut scenarios = Vec::new();
    for static_count in [256usize, 1024, 4096] {
        for moving_count in [1usize, 8, 32] {
            scenarios.push((
                format!("sparse_static_{static_count}_moving_{moving_count}"),
                static_heavy_broadphase_scene(static_count, moving_count),
            ));
        }
    }
    for static_count in [128usize, 512] {
        for moving_count in [1usize, 8, 32] {
            scenarios.push((
                format!("clustered_static_{static_count}_moving_{moving_count}"),
                clustered_static_heavy_broadphase_scene(static_count, moving_count),
            ));
        }
    }

    for (scenario, bodies) in scenarios {
        let expected = exhaustive_aabb_pair_oracle(&bodies);
        let static_tree = BenchStaticBvh::build(&bodies);
        let tree_pairs = static_tree.query_pairs(&bodies);
        assert_eq!(
            tree_pairs, expected,
            "benchmark static BVH mismatch in {scenario}"
        );

        let mut flat_cache = StaticBroadphase::new();
        flat_cache.rebuild(&bodies);
        let flat_pairs = broadphase::find_pairs_incremental(&bodies, &flat_cache);
        assert_eq!(flat_pairs, expected, "flat-cache mismatch in {scenario}");

        let full_pairs = broadphase::find_pairs(&bodies);
        assert_eq!(full_pairs, expected, "full LBVH mismatch in {scenario}");

        let pca_sap = BenchPcaSap::build(&bodies);
        let pca_sap_pairs = pca_sap.query_pairs(&bodies);
        assert_eq!(
            pca_sap_pairs, expected,
            "PCA sweep-and-prune mismatch in {scenario}, axis={:?}",
            pca_sap.axis
        );

        group.bench_function(
            BenchmarkId::new("pca_sap_query_only", &scenario),
            |bench| {
                bench.iter(|| {
                    black_box(pca_sap.query_pairs(black_box(&bodies)));
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("pca_sap_build_only", &scenario),
            |bench| {
                bench.iter(|| {
                    black_box(BenchPcaSap::build(black_box(&bodies)));
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("pca_sap_build_plus_query", &scenario),
            |bench| {
                bench.iter(|| {
                    let sap = BenchPcaSap::build(black_box(&bodies));
                    black_box(sap.query_pairs(black_box(&bodies)));
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("static_bvh_query_only", &scenario),
            |bench| {
                bench.iter(|| {
                    black_box(static_tree.query_pairs(black_box(&bodies)));
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("static_bvh_build_only", &scenario),
            |bench| {
                bench.iter(|| {
                    black_box(BenchStaticBvh::build(black_box(&bodies)));
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("static_bvh_build_plus_query", &scenario),
            |bench| {
                bench.iter(|| {
                    let tree = BenchStaticBvh::build(black_box(&bodies));
                    black_box(tree.query_pairs(black_box(&bodies)));
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("flat_static_cache_query_only", &scenario),
            |bench| {
                bench.iter(|| {
                    black_box(broadphase::find_pairs_incremental(
                        black_box(&bodies),
                        black_box(&flat_cache),
                    ));
                });
            },
        );

        group.bench_function(
            BenchmarkId::new("flat_static_cache_build_plus_query", &scenario),
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

        group.bench_function(
            BenchmarkId::new("full_lbvh_rebuild_plus_query", &scenario),
            |bench| {
                bench.iter(|| {
                    black_box(broadphase::find_pairs(black_box(&bodies)));
                });
            },
        );
    }

    group.finish();
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

fn bench_broadphase_static_churn(c: &mut Criterion) {
    const FRAMES: usize = 16;
    let mut group = c.benchmark_group("broadphase_static_churn_3d");

    // One static body changes pose on every frame. The cache cannot be reused
    // safely across that mutation, so the incremental path must rebuild its
    // static entries before each query. This is the high-churn counterexample
    // to the immutable-terrain multi-tick benchmark above.
    for static_count in [256usize, 1024] {
        for moving_count in [8usize, 32] {
            let frames: Vec<Vec<RigidBody<3>>> = (0..FRAMES)
                .map(|frame| {
                    let mut bodies = static_heavy_broadphase_scene(static_count, moving_count);
                    let changed_static = frame % static_count;
                    let body = &mut bodies[changed_static];
                    let x = changed_static as f64 * 3.0 + (frame as f64 + 1.0) * 0.05;
                    body.transform = Transform::from_translation(Point::new([x, 0.0, 0.0]));
                    bodies
                })
                .collect();

            for (frame_index, bodies) in frames.iter().enumerate() {
                let oracle_pairs = exhaustive_aabb_pair_oracle(bodies);
                assert_eq!(
                    broadphase::find_pairs(bodies),
                    oracle_pairs,
                    "static-churn LBVH mismatch at frame {frame_index}, static={static_count}, moving={moving_count}"
                );

                let mut cache = StaticBroadphase::new();
                cache.rebuild(bodies);
                assert_eq!(
                    broadphase::find_pairs_incremental(bodies, &cache),
                    oracle_pairs,
                    "static-churn incremental mismatch at frame {frame_index}, static={static_count}, moving={moving_count}"
                );
            }

            let case = format!(
                "static_{static_count}_moving_{moving_count}_frames_{FRAMES}_one_static_mutation_per_frame"
            );

            group.bench_function(
                BenchmarkId::new("static_cache_rebuild_each_tick_plus_query", &case),
                |bench| {
                    bench.iter(|| {
                        let mut candidates = 0usize;
                        let mut cache = StaticBroadphase::new();
                        for bodies in &frames {
                            cache.rebuild(black_box(bodies));
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

            group.bench_function(
                BenchmarkId::new("full_lbvh_rebuild_each_tick", &case),
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
    // Static cache invalidation under persistent geometry churn
    bench_broadphase_static_churn,
    // Benchmark-only static BVH vs flat-cache/LBVH portfolio
    bench_broadphase_static_bvh_tournament,
);
criterion_main!(benches);
