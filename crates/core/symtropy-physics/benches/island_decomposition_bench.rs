// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Scaling benchmark for deterministic contact/constraint island decomposition.
//!
//! Fixture construction is outside the timed region. The benchmark measures
//! only graph decomposition and edge-to-island assignment. It does not measure
//! collision detection, solver iterations, or whole-world stepping.

use criterion::{
    BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use nalgebra::SVector;
use std::collections::{HashMap, HashSet};
use symtropy_math::Point;
use symtropy_physics::body::{BodyHandle, RigidBody};
use symtropy_physics::constraint::{Constraint, DistanceConstraint};
use symtropy_physics::contact::ContactManifold;
use symtropy_physics::island::{Island, build_islands};

struct Fixture {
    bodies: Vec<RigidBody<3>>,
    contacts: Vec<ContactManifold<3>>,
    constraints: Vec<Box<dyn Constraint<3>>>,
    handle_to_index: HashMap<BodyHandle, usize>,
}

impl Fixture {
    fn new(body_count: usize) -> Self {
        let mut bodies = Vec::with_capacity(body_count);
        let mut handle_to_index = HashMap::with_capacity(body_count);
        for index in 0..body_count {
            let handle = BodyHandle(index);
            let body = RigidBody::dynamic_sphere(
                handle,
                Point::new([index as f64 * 2.0, 0.0, 0.0]),
                0.25,
                1.0,
            );
            bodies.push(body);
            handle_to_index.insert(handle, index);
        }

        Self {
            bodies,
            contacts: Vec::new(),
            constraints: Vec::new(),
            handle_to_index,
        }
    }

    fn isolated(body_count: usize) -> Self {
        Self::new(body_count)
    }

    fn disjoint_pairs(body_count: usize) -> Self {
        let mut fixture = Self::new(body_count);
        for index in (0..body_count.saturating_sub(1)).step_by(2) {
            fixture.contacts.push(ContactManifold::single(
                BodyHandle(index),
                BodyHandle(index + 1),
                SVector::from([1.0, 0.0, 0.0]),
                SVector::zeros(),
                0.0,
            ));
        }
        fixture
    }

    fn contact_chain(body_count: usize) -> Self {
        let mut fixture = Self::new(body_count);
        for index in 0..body_count.saturating_sub(1) {
            fixture.contacts.push(ContactManifold::single(
                BodyHandle(index),
                BodyHandle(index + 1),
                SVector::from([1.0, 0.0, 0.0]),
                SVector::zeros(),
                0.0,
            ));
        }
        fixture
    }

    fn mixed_contacts_constraints(body_count: usize) -> Self {
        let mut fixture = Self::new(body_count);
        let quarter = body_count / 4;

        for index in 0..quarter {
            let left = index * 2;
            let right = left + 1;
            fixture.contacts.push(ContactManifold::single(
                BodyHandle(left),
                BodyHandle(right),
                SVector::from([1.0, 0.0, 0.0]),
                SVector::zeros(),
                0.0,
            ));
        }

        let constraint_start = body_count / 2;
        for index in 0..quarter {
            let left = constraint_start + index * 2;
            let right = left + 1;
            fixture.constraints.push(Box::new(DistanceConstraint {
                body_a: BodyHandle(left),
                body_b: BodyHandle(right),
                rest_length: 2.0,
                stiffness: 1.0,
            }));
        }

        fixture
    }

    fn decompose(&self) -> Vec<Island> {
        build_islands(
            &self.bodies,
            &self.contacts,
            &self.constraints,
            &self.handle_to_index,
        )
    }

    /// Benchmark-only reproduction of the pre-single-pass implementation:
    /// for each component, scan every contact and constraint and filter by a
    /// per-component body HashSet. The result is canonicalized by BodyHandle
    /// so output order can be compared with the candidate implementation.
    fn legacy_rescan_reference(&self) -> Vec<Island> {
        let mut union_find = ReferenceUnionFind::new(self.bodies.len());

        for contact in &self.contacts {
            if let (Some(&a), Some(&b)) = (
                self.handle_to_index.get(&contact.body_a),
                self.handle_to_index.get(&contact.body_b),
            ) {
                union_find.union(a, b);
            }
        }
        for constraint in &self.constraints {
            let (handle_a, handle_b) = constraint.bodies();
            if let (Some(&a), Some(&b)) = (
                self.handle_to_index.get(&handle_a),
                self.handle_to_index.get(&handle_b),
            ) {
                union_find.union(a, b);
            }
        }

        let mut groups: HashMap<usize, Vec<usize>> = HashMap::new();
        for body_index in 0..self.bodies.len() {
            let root = union_find.find(body_index);
            groups.entry(root).or_default().push(body_index);
        }

        let mut islands = Vec::with_capacity(groups.len());
        for body_indices in groups.into_values() {
            let body_set: HashSet<usize> = body_indices.iter().copied().collect();
            let contact_indices = self
                .contacts
                .iter()
                .enumerate()
                .filter(|(_, contact)| {
                    let a = self.handle_to_index.get(&contact.body_a).copied();
                    let b = self.handle_to_index.get(&contact.body_b).copied();
                    a.is_some_and(|index| body_set.contains(&index))
                        || b.is_some_and(|index| body_set.contains(&index))
                })
                .map(|(index, _)| index)
                .collect();
            let constraint_indices = self
                .constraints
                .iter()
                .enumerate()
                .filter(|(_, constraint)| {
                    let (handle_a, handle_b) = constraint.bodies();
                    let a = self.handle_to_index.get(&handle_a).copied();
                    let b = self.handle_to_index.get(&handle_b).copied();
                    a.is_some_and(|index| body_set.contains(&index))
                        || b.is_some_and(|index| body_set.contains(&index))
                })
                .map(|(index, _)| index)
                .collect();
            let sleeping = body_indices
                .iter()
                .all(|&index| self.bodies[index].sleeping);

            islands.push(Island {
                body_indices,
                contact_indices,
                constraint_indices,
                sleeping,
            });
        }

        islands.sort_by_key(|island| {
            island
                .body_indices
                .iter()
                .map(|&index| self.bodies[index].handle)
                .min()
        });
        islands
    }
}

struct ReferenceUnionFind {
    parent: Vec<usize>,
    rank: Vec<usize>,
}

impl ReferenceUnionFind {
    fn new(count: usize) -> Self {
        Self {
            parent: (0..count).collect(),
            rank: vec![0; count],
        }
    }

    fn find(&mut self, mut index: usize) -> usize {
        while self.parent[index] != index {
            self.parent[index] = self.parent[self.parent[index]];
            index = self.parent[index];
        }
        index
    }

    fn union(&mut self, left: usize, right: usize) {
        let root_left = self.find(left);
        let root_right = self.find(right);
        if root_left == root_right {
            return;
        }
        if self.rank[root_left] < self.rank[root_right] {
            self.parent[root_left] = root_right;
        } else if self.rank[root_left] > self.rank[root_right] {
            self.parent[root_right] = root_left;
        } else {
            self.parent[root_right] = root_left;
            self.rank[root_left] += 1;
        }
    }
}

type IslandSignature = (Vec<usize>, Vec<usize>, Vec<usize>, bool);

fn island_signature(islands: &[Island]) -> Vec<IslandSignature> {
    islands
        .iter()
        .map(|island| {
            (
                island.body_indices.clone(),
                island.contact_indices.clone(),
                island.constraint_indices.clone(),
                island.sleeping,
            )
        })
        .collect()
}

fn bench_island_decomposition(c: &mut Criterion) {
    let mut group = c.benchmark_group("island_decomposition");
    group.sample_size(20);

    let cases: [(&str, fn(usize) -> Fixture, &[usize]); 4] = [
        ("isolated", Fixture::isolated, &[1_000, 10_000, 100_000]),
        ("pairs", Fixture::disjoint_pairs, &[1_000, 10_000]),
        ("contact_chain", Fixture::contact_chain, &[1_000, 10_000]),
        (
            "mixed_edges",
            Fixture::mixed_contacts_constraints,
            &[1_000, 10_000],
        ),
    ];

    for (name, build, sizes) in cases {
        for &body_count in sizes {
            let fixture = build(body_count);
            let candidate = fixture.decompose();
            let reference = fixture.legacy_rescan_reference();
            assert_eq!(
                island_signature(&candidate),
                island_signature(&reference),
                "single-pass decomposition must preserve the previous graph partition for {name}/{body_count}"
            );

            let expected_islands = candidate.len();
            group.throughput(Throughput::Elements(body_count as u64));
            group.bench_with_input(
                BenchmarkId::new(format!("{name}_single_pass"), body_count),
                &fixture,
                |bench, fixture| {
                    bench.iter(|| {
                        let islands = fixture.decompose();
                        black_box((islands.len(), islands));
                    });
                },
            );

            // The reference deliberately preserves the former O(I*(C+K))
            // component-rescan structure. Avoid the 100k isolated case for the
            // legacy baseline, where its per-island HashSet allocations are
            // intentionally pathological; still measure the candidate there.
            if body_count <= 10_000 {
                group.bench_with_input(
                    BenchmarkId::new(format!("{name}_legacy_rescan_reference"), body_count),
                    &fixture,
                    |bench, fixture| {
                        bench.iter(|| {
                            let islands = fixture.legacy_rescan_reference();
                            black_box((islands.len(), islands));
                        });
                    },
                );
            }

            eprintln!(
                "island benchmark fixture: {name}, bodies={body_count}, expected_components={expected_islands}"
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_island_decomposition);
criterion_main!(benches);
