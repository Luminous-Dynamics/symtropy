// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Matched-cost microbenchmarks for skew-lattice closest-image and AABB image
//! candidate generation. Every timed corpus is preflighted against a bounded
//! brute-force integer-image oracle before Criterion timings are registered.
//!
//! Run:
//!   cargo bench -p symtropy-physics --bench triclinic_periodic_geometry
//!
//! The output reports geometry correctness preflight separately from timing.
//! Criterion timing is not a correctness proof or an end-to-end physics claim.

use criterion::{
    BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main,
};
use nalgebra::{SMatrix, SVector};
use std::collections::HashSet;
use symtropy_math::LatticePeriodicDomain;
use symtropy_physics::{
    Aabb, image_offsets_for_aabb_pair_lattice,
};

const PRECHECK_RELATIVE_TOLERANCE: f64 = 2.0e-10;
const SEARCH_BUDGET: usize = 100_000;
const ORACLE_SPAN: i64 = 8;
const CORPUS_SIZE: usize = 96;

fn xorshift_next(state: &mut u64) -> f64 {
    *state ^= *state << 13;
    *state ^= *state >> 7;
    *state ^= *state << 17;
    (*state as f64) / (u64::MAX as f64)
}

fn query_corpus_2d(seed: u64, count: usize) -> Vec<SVector<f64, 2>> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            SVector::from([
                xorshift_next(&mut state) * 48.0 - 24.0,
                xorshift_next(&mut state) * 48.0 - 24.0,
            ])
        })
        .collect()
}

fn query_corpus_3d(seed: u64, count: usize) -> Vec<SVector<f64, 3>> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            SVector::from([
                xorshift_next(&mut state) * 48.0 - 24.0,
                xorshift_next(&mut state) * 48.0 - 24.0,
                xorshift_next(&mut state) * 48.0 - 24.0,
            ])
        })
        .collect()
}

fn brute_force_minimum_image<const D: usize>(
    domain: &LatticePeriodicDomain<D>,
    displacement: &SVector<f64, D>,
    span: i64,
) -> SVector<f64, D> {
    let mut best = *displacement;
    let mut best_distance_squared = best.norm_squared();
    let mut offsets = vec![[0_i64; D]];

    for axis in 0..D {
        if !domain.periodic_axes()[axis] {
            continue;
        }
        let mut next = Vec::with_capacity(offsets.len() * (2 * span + 1) as usize);
        for existing in offsets {
            for value in -span..=span {
                let mut candidate = existing;
                candidate[axis] = value;
                next.push(candidate);
            }
        }
        offsets = next;
    }

    for offset in offsets {
        let mut coefficients = SVector::<f64, D>::zeros();
        for axis in 0..D {
            coefficients[axis] = offset[axis] as f64;
        }
        let candidate = displacement - domain.basis() * coefficients;
        let distance_squared = candidate.norm_squared();
        if distance_squared < best_distance_squared {
            best = candidate;
            best_distance_squared = distance_squared;
        }
    }

    best
}

fn assert_same_minimum_image<const D: usize>(
    produced: &SVector<f64, D>,
    oracle: &SVector<f64, D>,
    case: &str,
) {
    let produced_squared = produced.norm_squared();
    let oracle_squared = oracle.norm_squared();
    let tolerance = PRECHECK_RELATIVE_TOLERANCE * oracle_squared.max(1.0);
    assert!(
        (produced_squared - oracle_squared).abs() <= tolerance,
        "minimum-image preflight mismatch in {case}: produced={produced_squared}, oracle={oracle_squared}"
    );
}

fn bench_closest_image_2d(c: &mut Criterion) {
    let cases = [
        (
            "orthogonal",
            SMatrix::<f64, 2, 2>::new(10.0, 0.0, 0.0, 8.0),
        ),
        (
            "moderate_skew",
            SMatrix::<f64, 2, 2>::new(10.0, 3.0, 0.0, 8.0),
        ),
        (
            "high_skew",
            SMatrix::<f64, 2, 2>::new(10.0, 4.5, 0.0, 8.0),
        ),
    ];

    let mut group = c.benchmark_group("triclinic_closest_image_2d");
    let queries = query_corpus_2d(0xB2D0_7A31, CORPUS_SIZE);
    group.throughput(Throughput::Elements(queries.len() as u64));

    for (name, basis) in cases {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            basis,
            [true, true],
            SEARCH_BUDGET,
        )
        .expect("benchmark lattice domain");

        let mut total_candidates = 0usize;
        let mut max_candidates = 0usize;
        for query in &queries {
            let produced = domain
                .minimum_image_displacement(query)
                .expect("closest-image solver must complete in benchmark envelope");
            let oracle = brute_force_minimum_image(&domain, query, ORACLE_SPAN);
            assert_same_minimum_image(&produced.displacement, &oracle, name);
            total_candidates += produced.candidates_examined;
            max_candidates = max_candidates.max(produced.candidates_examined);
        }
        println!(
            "TRICLINIC_MIN_IMAGE_PREFLIGHT,v1,dimension=2,case={name},queries={},candidate_total={total_candidates},candidate_max={max_candidates},oracle_span={ORACLE_SPAN},disposition=pass",
            queries.len()
        );

        group.bench_with_input(
            BenchmarkId::new("bounded_inverse_interval", name),
            &queries,
            |bench, queries| {
                bench.iter(|| {
                    let mut checksum = 0.0;
                    for query in queries {
                        let result = domain
                            .minimum_image_displacement(black_box(query))
                            .expect("preflighted closest-image case");
                        checksum += result.displacement.norm_squared();
                    }
                    black_box(checksum)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("bounded_brute_force", name),
            &queries,
            |bench, queries| {
                bench.iter(|| {
                    let mut checksum = 0.0;
                    for query in queries {
                        let result =
                            brute_force_minimum_image(&domain, black_box(query), ORACLE_SPAN);
                        checksum += result.norm_squared();
                    }
                    black_box(checksum)
                });
            },
        );
    }
    group.finish();
}

fn bench_closest_image_3d(c: &mut Criterion) {
    let cases = [
        (
            "orthogonal",
            SMatrix::<f64, 3, 3>::new(
                8.0, 0.0, 0.0,
                0.0, 7.0, 0.0,
                0.0, 0.0, 6.0,
            ),
        ),
        (
            "triclinic",
            SMatrix::<f64, 3, 3>::new(
                8.0, 2.0, 1.0,
                0.0, 7.0, 2.0,
                0.0, 0.0, 6.0,
            ),
        ),
    ];

    let mut group = c.benchmark_group("triclinic_closest_image_3d");
    let queries = query_corpus_3d(0xD3A0_951B, CORPUS_SIZE);
    group.throughput(Throughput::Elements(queries.len() as u64));

    for (name, basis) in cases {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            basis,
            [true, true, true],
            SEARCH_BUDGET,
        )
        .expect("benchmark lattice domain");

        let mut total_candidates = 0usize;
        let mut max_candidates = 0usize;
        for query in &queries {
            let produced = domain
                .minimum_image_displacement(query)
                .expect("closest-image solver must complete in benchmark envelope");
            let oracle = brute_force_minimum_image(&domain, query, ORACLE_SPAN);
            assert_same_minimum_image(&produced.displacement, &oracle, name);
            total_candidates += produced.candidates_examined;
            max_candidates = max_candidates.max(produced.candidates_examined);
        }
        println!(
            "TRICLINIC_MIN_IMAGE_PREFLIGHT,v1,dimension=3,case={name},queries={},candidate_total={total_candidates},candidate_max={max_candidates},oracle_span={ORACLE_SPAN},disposition=pass",
            queries.len()
        );

        group.bench_with_input(
            BenchmarkId::new("bounded_inverse_interval", name),
            &queries,
            |bench, queries| {
                bench.iter(|| {
                    let mut checksum = 0.0;
                    for query in queries {
                        let result = domain
                            .minimum_image_displacement(black_box(query))
                            .expect("preflighted closest-image case");
                        checksum += result.displacement.norm_squared();
                    }
                    black_box(checksum)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("bounded_brute_force", name),
            &queries,
            |bench, queries| {
                bench.iter(|| {
                    let mut checksum = 0.0;
                    for query in queries {
                        let result =
                            brute_force_minimum_image(&domain, black_box(query), ORACLE_SPAN);
                        checksum += result.norm_squared();
                    }
                    black_box(checksum)
                });
            },
        );
    }
    group.finish();
}

fn aabb_2d(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
    Aabb {
        min: SVector::from(min),
        max: SVector::from(max),
    }
}

fn aabb_corpus_2d(seed: u64, count: usize) -> Vec<(Aabb<2>, Aabb<2>)> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            let a_min = [
                xorshift_next(&mut state) * 24.0 - 12.0,
                xorshift_next(&mut state) * 24.0 - 12.0,
            ];
            let a_size = [
                xorshift_next(&mut state) * 2.5 + 0.1,
                xorshift_next(&mut state) * 2.5 + 0.1,
            ];
            let b_min = [
                xorshift_next(&mut state) * 24.0 - 12.0,
                xorshift_next(&mut state) * 24.0 - 12.0,
            ];
            let b_size = [
                xorshift_next(&mut state) * 2.5 + 0.1,
                xorshift_next(&mut state) * 2.5 + 0.1,
            ];
            (
                aabb_2d(
                    a_min,
                    [a_min[0] + a_size[0], a_min[1] + a_size[1]],
                ),
                aabb_2d(
                    b_min,
                    [b_min[0] + b_size[0], b_min[1] + b_size[1]],
                ),
            )
        })
        .collect()
}

fn brute_force_aabb_images_2d(
    domain: &LatticePeriodicDomain<2>,
    a: &Aabb<2>,
    b: &Aabb<2>,
) -> HashSet<[i64; 2]> {
    let mut result = HashSet::new();
    for i in -ORACLE_SPAN..=ORACLE_SPAN {
        for j in -ORACLE_SPAN..=ORACLE_SPAN {
            let coefficients = SVector::from([i as f64, j as f64]);
            let shift = domain.basis() * coefficients;
            let shifted = Aabb {
                min: b.min + shift,
                max: b.max + shift,
            };
            if a.overlaps(&shifted) {
                result.insert([i, j]);
            }
        }
    }
    result
}

fn bench_skew_aabb_image_candidates(c: &mut Criterion) {
    let cases = [
        (
            "orthogonal",
            SMatrix::<f64, 2, 2>::new(10.0, 0.0, 0.0, 8.0),
        ),
        (
            "moderate_skew",
            SMatrix::<f64, 2, 2>::new(10.0, 3.0, 0.0, 8.0),
        ),
        (
            "high_skew",
            SMatrix::<f64, 2, 2>::new(10.0, 4.5, 0.0, 8.0),
        ),
    ];

    let mut group = c.benchmark_group("triclinic_aabb_image_candidates_2d");
    let corpus = aabb_corpus_2d(0xAABB_2026, CORPUS_SIZE);
    group.throughput(Throughput::Elements(corpus.len() as u64));

    for (name, basis) in cases {
        let domain = LatticePeriodicDomain::new(
            SVector::zeros(),
            basis,
            [true, true],
            SEARCH_BUDGET,
        )
        .expect("benchmark lattice domain");

        let mut produced_total = 0usize;
        let mut oracle_total = 0usize;
        let mut extra_total = 0usize;

        for (a, b) in &corpus {
            let produced = image_offsets_for_aabb_pair_lattice(
                &domain,
                a,
                b,
                SEARCH_BUDGET,
            )
            .expect("broadphase must complete in benchmark envelope");
            let produced_set: HashSet<_> = produced.iter().copied().collect();
            let oracle = brute_force_aabb_images_2d(&domain, a, b);
            assert!(
                oracle.is_subset(&produced_set),
                "broadphase preflight missed an image for case {name}: missing={:?}",
                oracle.difference(&produced_set).collect::<Vec<_>>()
            );
            produced_total += produced.len();
            oracle_total += oracle.len();
            extra_total += produced_set.difference(&oracle).count();
        }

        println!(
            "TRICLINIC_AABB_PREFLIGHT,v1,dimension=2,case={name},pairs={},oracle_images={oracle_total},candidate_images={produced_total},extra_images={extra_total},disposition=pass",
            corpus.len()
        );

        group.bench_with_input(
            BenchmarkId::new("inverse_interval_candidates", name),
            &corpus,
            |bench, corpus| {
                bench.iter(|| {
                    let mut count = 0usize;
                    for (a, b) in corpus {
                        count += image_offsets_for_aabb_pair_lattice(
                            &domain,
                            black_box(a),
                            black_box(b),
                            SEARCH_BUDGET,
                        )
                        .expect("preflighted broadphase case")
                        .len();
                    }
                    black_box(count)
                });
            },
        );

        group.bench_with_input(
            BenchmarkId::new("brute_force_integer_images", name),
            &corpus,
            |bench, corpus| {
                bench.iter(|| {
                    let mut count = 0usize;
                    for (a, b) in corpus {
                        count += brute_force_aabb_images_2d(
                            &domain,
                            black_box(a),
                            black_box(b),
                        )
                        .len();
                    }
                    black_box(count)
                });
            },
        );
    }
    group.finish();
}

fn benches(c: &mut Criterion) {
    bench_closest_image_2d(c);
    bench_closest_image_3d(c);
    bench_skew_aabb_image_candidates(c);
}

criterion_group!(triclinic_periodic_geometry, benches);
criterion_main!(triclinic_periodic_geometry);
