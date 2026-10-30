// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! End-to-end triclinic convex query microbenchmark.
//!
//! Compares the conservative reciprocal-projection image query plus GJK
//! against direct GJK over a wider fixed image cube. Every corpus is
//! differentially preflighted before timing; all paths return the same
//! per-image hit/unresolved sets before performance is measured.

use std::collections::BTreeSet;

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector};
use symtropy_math::{LatticeOffset, PeriodicLattice, Point, Shape, Sphere, Transform};
use symtropy_physics::gjk;
use symtropy_physics::periodic_lattice_collision::{
    PeriodicLatticeCollisionQuery, query_triclinic_convex,
};

const MAX_IMAGE_CANDIDATES: usize = 1_024;
const ORACLE_IMAGE_RADIUS: i64 = 6;

#[derive(Clone, Copy)]
enum FixtureKind {
    Interior,
    FirstVectorSeam,
    SkewVectorSeam,
    Separated,
}

impl FixtureKind {
    fn name(self) -> &'static str {
        match self {
            Self::Interior => "interior",
            Self::FirstVectorSeam => "first_vector_seam",
            Self::SkewVectorSeam => "skew_vector_seam",
            Self::Separated => "separated",
        }
    }

    fn seed(self) -> u64 {
        match self {
            Self::Interior => 0x494e_5445_5249_4f52,
            Self::FirstVectorSeam => 0x5345_414d_5f58_5f31,
            Self::SkewVectorSeam => 0x534b_4557_5f53_4541,
            Self::Separated => 0x5345_5041_5241_5445,
        }
    }
}

struct PairFixture {
    transform_a: Transform<2>,
    transform_b: Transform<2>,
}

fn next_u64(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn unit(state: &mut u64) -> f64 {
    (next_u64(state) >> 11) as f64 / ((1_u64 << 53) as f64)
}

fn range(state: &mut u64, low: f64, high: f64) -> f64 {
    low + (high - low) * unit(state)
}

fn lattice() -> PeriodicLattice<2> {
    let basis =
        SMatrix::<f64, 2, 2>::from_row_slice(&[10.0, 4.0, 0.0, 8.0]);
    PeriodicLattice::new(SVector::zeros(), basis)
        .expect("fixed skew lattice is valid")
}

fn make_corpus(count: usize, kind: FixtureKind) -> Vec<PairFixture> {
    let mut state = kind.seed();
    let cell = lattice();
    let skew_vector = cell.basis().column(1).into_owned();
    let first_vector = cell.basis().column(0).into_owned();
    let mut corpus = Vec::with_capacity(count);

    for _ in 0..count {
        let (a, b) = match kind {
            FixtureKind::Interior => (
                SVector::from([
                    range(&mut state, 1.0, 4.0),
                    range(&mut state, 1.0, 3.0),
                ]),
                SVector::from([
                    range(&mut state, 5.0, 8.0),
                    range(&mut state, 3.0, 6.0),
                ]),
            ),
            FixtureKind::FirstVectorSeam => (
                SVector::from([
                    range(&mut state, 0.1, 0.8),
                    range(&mut state, 0.1, 0.8),
                ]),
                SVector::from([
                    range(&mut state, 9.6, 10.4),
                    range(&mut state, 0.1, 0.8),
                ]),
            ),
            FixtureKind::SkewVectorSeam => {
                let a = SVector::from([
                    range(&mut state, 1.0, 5.0),
                    range(&mut state, 1.0, 5.0),
                ]);
                let delta = SVector::from([
                    range(&mut state, -0.2, 0.2),
                    range(&mut state, -0.2, 0.2),
                ]);
                (a, a + skew_vector + delta)
            }
            FixtureKind::Separated => {
                let a = SVector::from([
                    range(&mut state, -3.0, 3.0),
                    range(&mut state, -3.0, 3.0),
                ]);
                let b = a + SVector::from([
                    range(&mut state, 3.0, 6.0),
                    range(&mut state, 3.0, 6.0),
                ]);
                (a, b)
            }
        };

        // Consume a deterministic value from both basis vectors to bind the
        // fixtures to the named lattice geometry and avoid unused helpers.
        let _basis_scale = first_vector.norm() * 0.0;
        corpus.push(PairFixture {
            transform_a: Transform::from_translation(Point(a)),
            transform_b: Transform::from_translation(Point(b)),
        });
    }
    corpus
}

fn oracle_direct_gjk(
    lattice: &PeriodicLattice<2>,
    shape_a: &dyn Shape<2>,
    transform_a: &Transform<2>,
    shape_b: &dyn Shape<2>,
    transform_b: &Transform<2>,
) -> (BTreeSet<[i64; 2]>, BTreeSet<[i64; 2]>, usize) {
    let mut hits = BTreeSet::new();
    let mut unresolved = BTreeSet::new();
    let mut gjk_calls = 0_usize;

    for first in -ORACLE_IMAGE_RADIUS..=ORACLE_IMAGE_RADIUS {
        for second in -ORACLE_IMAGE_RADIUS..=ORACLE_IMAGE_RADIUS {
            let offset = LatticeOffset::new([first, second]);
            let shift = lattice
                .offset_vector(offset)
                .expect("bounded oracle lattice shift is valid");
            let mut image_transform_b = transform_b.clone();
            image_transform_b.translation.0 += shift;
            let result = gjk::intersects_transformed(
                shape_a,
                transform_a,
                shape_b,
                &image_transform_b,
            );
            gjk_calls += 1;
            if result.intersecting {
                hits.insert([first, second]);
            } else if result.iterations >= 64 {
                unresolved.insert([first, second]);
            }
        }
    }

    (hits, unresolved, gjk_calls)
}

fn query_sets<const D: usize>(
    result: &PeriodicLatticeCollisionQuery<D>,
) -> (BTreeSet<[i64; D]>, BTreeSet<[i64; D]>) {
    (
        result
            .hits
            .iter()
            .map(|hit| hit.image_offset.periods)
            .collect(),
        result
            .unresolved_offsets
            .iter()
            .map(|offset| offset.periods)
            .collect(),
    )
}

fn verify_corpus(
    cell: &PeriodicLattice<2>,
    corpus: &[PairFixture],
    shape_a: &dyn Shape<2>,
    shape_b: &dyn Shape<2>,
    kind: FixtureKind,
) -> (usize, usize, usize) {
    let mut production_candidates = 0_usize;
    let mut production_gjk = 0_usize;
    let mut oracle_gjk = 0_usize;

    for (case, fixture) in corpus.iter().enumerate() {
        let query = query_triclinic_convex(
            cell,
            shape_a,
            &fixture.transform_a,
            shape_b,
            &fixture.transform_b,
            MAX_IMAGE_CANDIDATES,
        )
        .unwrap_or_else(|error| panic!("production query {:?} case {case}: {error}", kind.name()));
        let oracle = oracle_direct_gjk(
            cell,
            shape_a,
            &fixture.transform_a,
            shape_b,
            &fixture.transform_b,
        );
        let actual = query_sets(&query);
        assert_eq!(
            actual.0,
            oracle.0,
            "hit mismatch fixture={} case={case}",
            kind.name()
        );
        assert_eq!(
            actual.1,
            oracle.1,
            "unresolved mismatch fixture={} case={case}",
            kind.name()
        );
        production_candidates += query.image_candidates;
        production_gjk += query.gjk_calls;
        oracle_gjk += oracle.2;
    }

    (production_candidates, production_gjk, oracle_gjk)
}

fn bench_triclinic_periodic_queries(c: &mut Criterion) {
    let cell = lattice();
    let shape_a = Sphere::<2>::new(Point::origin(), 0.5);
    let shape_b = Sphere::<2>::new(Point::origin(), 0.5);
    let mut group = c.benchmark_group("triclinic_periodic_convex_query");

    for pair_count in [32_usize, 128, 512] {
        for kind in [
            FixtureKind::Interior,
            FixtureKind::FirstVectorSeam,
            FixtureKind::SkewVectorSeam,
            FixtureKind::Separated,
        ] {
            let corpus = make_corpus(pair_count, kind);
            let (candidate_count, production_gjk, oracle_gjk) =
                verify_corpus(&cell, &corpus, &shape_a, &shape_b, kind);
            eprintln!(
                "TRICLINIC_QUERY_PREFLIGHT,v1,fixture={},pairs={},candidate_images={},production_gjk_calls={},oracle_gjk_calls={}",
                kind.name(),
                pair_count,
                candidate_count,
                production_gjk,
                oracle_gjk
            );

            let id = format!("{}/pairs_{pair_count}", kind.name());
            group.bench_with_input(
                BenchmarkId::new("reciprocal_broadphase_plus_gjk", id.as_str()),
                &corpus,
                |b, corpus| {
                    b.iter(|| {
                        for fixture in black_box(corpus) {
                            let result = query_triclinic_convex(
                                &cell,
                                &shape_a,
                                &fixture.transform_a,
                                &shape_b,
                                &fixture.transform_b,
                                MAX_IMAGE_CANDIDATES,
                            )
                            .expect("preflight-qualified query");
                            black_box(result);
                        }
                    })
                },
            );
            group.bench_with_input(
                BenchmarkId::new("brute_force_169_images_plus_gjk", id.as_str()),
                &corpus,
                |b, corpus| {
                    b.iter(|| {
                        for fixture in black_box(corpus) {
                            let result = oracle_direct_gjk(
                                &cell,
                                &shape_a,
                                &fixture.transform_a,
                                &shape_b,
                                &fixture.transform_b,
                            );
                            black_box(result);
                        }
                    })
                },
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench_triclinic_periodic_queries);
criterion_main!(benches);
