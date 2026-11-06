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
use std::collections::HashMap;
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
            let expected_islands = fixture.decompose().len();
            group.throughput(Throughput::Elements(body_count as u64));
            group.bench_with_input(
                BenchmarkId::new(name, body_count),
                &fixture,
                |bench, fixture| {
                    bench.iter(|| {
                        let islands = fixture.decompose();
                        black_box((islands.len(), islands));
                    });
                },
            );
            eprintln!(
                "island benchmark fixture: {name}, bodies={body_count}, expected_components={expected_islands}"
            );
        }
    }

    group.finish();
}

criterion_group!(benches, bench_island_decomposition);
criterion_main!(benches);
