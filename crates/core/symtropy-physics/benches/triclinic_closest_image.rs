// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Differential benchmark for full-rank triclinic closest-image queries.
//!
//! The brute-force oracle enumerates a fixed integer-offset cube. The frozen
//! corpora stay inside that cube; an untimed preflight compares exact image
//! choices and distances before any timings are recorded.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector};
use symtropy_math::PeriodicLattice;

const MAX_NODES: usize = 100_000;
const ORACLE_RADIUS_2D: i64 = 10;
const ORACLE_RADIUS_3D: i64 = 7;

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn random_range(state: &mut u64, lower: f64, upper: f64) -> f64 {
    let bits = next_random(state) >> 11;
    lower + (upper - lower) * (bits as f64 / ((1_u64 << 53) as f64))
}

fn brute_force_2d(
    displacement: &SVector<f64, 2>,
    basis: &SMatrix<f64, 2, 2>,
) -> (f64, [i64; 2]) {
    let mut best: Option<(f64, [i64; 2])> = None;
    for x in -ORACLE_RADIUS_2D..=ORACLE_RADIUS_2D {
        for y in -ORACLE_RADIUS_2D..=ORACLE_RADIUS_2D {
            let offset = [x, y];
            let candidate =
                displacement + basis * SVector::from([x as f64, y as f64]);
            let distance_squared = candidate.norm_squared();
            let replace = best.is_none_or(|current| {
                distance_squared < current.0
                    || (distance_squared == current.0 && offset < current.1)
            });
            if replace {
                best = Some((distance_squared, offset));
            }
        }
    }
    best.expect("nonempty brute-force image window")
}

fn brute_force_3d(
    displacement: &SVector<f64, 3>,
    basis: &SMatrix<f64, 3, 3>,
) -> (f64, [i64; 3]) {
    let mut best: Option<(f64, [i64; 3])> = None;
    for x in -ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D {
        for y in -ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D {
            for z in -ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D {
                let offset = [x, y, z];
                let candidate = displacement
                    + basis * SVector::from([x as f64, y as f64, z as f64]);
                let distance_squared = candidate.norm_squared();
                let replace = best.is_none_or(|current| {
                    distance_squared < current.0
                        || (distance_squared == current.0 && offset < current.1)
                });
                if replace {
                    best = Some((distance_squared, offset));
                }
            }
        }
    }
    best.expect("nonempty brute-force image window")
}

fn make_corpus_2d(count: usize, seed: u64) -> Vec<SVector<f64, 2>> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            SVector::from([
                random_range(&mut state, -9.0, 9.0),
                random_range(&mut state, -9.0, 9.0),
            ])
        })
        .collect()
}

fn make_corpus_3d(count: usize, seed: u64) -> Vec<SVector<f64, 3>> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            SVector::from([
                random_range(&mut state, -6.0, 6.0),
                random_range(&mut state, -6.0, 6.0),
                random_range(&mut state, -6.0, 6.0),
            ])
        })
        .collect()
}

fn run_2d(
    lattice: &PeriodicLattice<2>,
    corpus: &[SVector<f64, 2>],
) -> usize {
    corpus.iter().fold(0_usize, |nodes, displacement| {
        nodes
            + lattice
                .minimum_image_displacement(displacement, MAX_NODES)
                .expect("benchmark query must fit the node budget")
                .search_nodes
    })
}

fn run_3d(
    lattice: &PeriodicLattice<3>,
    corpus: &[SVector<f64, 3>],
) -> usize {
    corpus.iter().fold(0_usize, |nodes, displacement| {
        nodes
            + lattice
                .minimum_image_displacement(displacement, MAX_NODES)
                .expect("benchmark query must fit the node budget")
                .search_nodes
    })
}

fn preflight_2d(
    label: &str,
    lattice: &PeriodicLattice<2>,
    corpus: &[SVector<f64, 2>],
) {
    let mut total_nodes = 0_usize;
    let mut max_nodes = 0_usize;
    for (case, displacement) in corpus.iter().enumerate() {
        let actual = lattice
            .minimum_image_displacement(displacement, MAX_NODES)
            .unwrap_or_else(|error| panic!("{label} case={case}: {error}"));
        let expected = brute_force_2d(displacement, lattice.basis());
        assert_eq!(
            actual.image_offset.periods,
            expected.1,
            "{label} image mismatch at case {case}"
        );
        assert!(
            (actual.distance_squared - expected.0).abs() <= 1e-10,
            "{label} distance mismatch at case {case}: {} vs {}",
            actual.distance_squared,
            expected.0
        );
        total_nodes += actual.search_nodes;
        max_nodes = max_nodes.max(actual.search_nodes);
    }
    eprintln!(
        "TRICLINIC_CVP_PREFLIGHT,v1,dimension=2,basis={},queries={},mean_nodes={:.3},max_nodes={},oracle_candidates_per_query={}",
        label,
        corpus.len(),
        total_nodes as f64 / corpus.len() as f64,
        max_nodes,
        (2 * ORACLE_RADIUS_2D as usize + 1).pow(2)
    );
}

fn preflight_3d(
    lattice: &PeriodicLattice<3>,
    corpus: &[SVector<f64, 3>],
) {
    let mut total_nodes = 0_usize;
    let mut max_nodes = 0_usize;
    for (case, displacement) in corpus.iter().enumerate() {
        let actual = lattice
            .minimum_image_displacement(displacement, MAX_NODES)
            .unwrap_or_else(|error| panic!("3d case={case}: {error}"));
        let expected = brute_force_3d(displacement, lattice.basis());
        assert_eq!(
            actual.image_offset.periods,
            expected.1,
            "3D image mismatch at case {case}"
        );
        assert!(
            (actual.distance_squared - expected.0).abs() <= 1e-10,
            "3D distance mismatch at case {case}: {} vs {}",
            actual.distance_squared,
            expected.0
        );
        total_nodes += actual.search_nodes;
        max_nodes = max_nodes.max(actual.search_nodes);
    }
    eprintln!(
        "TRICLINIC_CVP_PREFLIGHT,v1,dimension=3,basis=moderate_skew,queries={},mean_nodes={:.3},max_nodes={},oracle_candidates_per_query={}",
        corpus.len(),
        total_nodes as f64 / corpus.len() as f64,
        max_nodes,
        (2 * ORACLE_RADIUS_3D as usize + 1).pow(3)
    );
}

fn bench_triclinic_closest_image(c: &mut Criterion) {
    let mut group = c.benchmark_group("triclinic_closest_image");

    let basis_fixtures = [
        (
            "orthogonal",
            SMatrix::<f64, 2, 2>::from_row_slice(&[10.0, 0.0, 0.0, 9.0]),
        ),
        (
            "moderate_skew",
            SMatrix::<f64, 2, 2>::from_row_slice(&[10.0, 4.5, 0.0, 8.0]),
        ),
        (
            "strong_accepted_skew",
            SMatrix::<f64, 2, 2>::from_row_slice(&[10.0, 7.8, 0.0, 4.0]),
        ),
    ];

    for (label, basis) in basis_fixtures {
        let lattice = PeriodicLattice::new(SVector::zeros(), basis.clone())
            .expect("benchmark basis must pass lattice validation");
        for count in [64_usize, 256, 1_024] {
            let corpus = make_corpus_2d(count, 0x5452_4943_4c49_4e49);
            preflight_2d(label, &lattice, &corpus);
            black_box(run_2d(&lattice, &corpus));

            group.bench_with_input(
                BenchmarkId::new(
                    format!("qr_sphere_decode/{label}"),
                    count,
                ),
                &corpus,
                |b, corpus| b.iter(|| black_box(run_2d(&lattice, black_box(corpus)))),
            );
            group.bench_with_input(
                BenchmarkId::new(
                    format!("brute_force_2d_441/{label}"),
                    count,
                ),
                &corpus,
                |b, corpus| {
                    b.iter(|| {
                        black_box(
                            corpus
                                .iter()
                                .map(|displacement| brute_force_2d(displacement, &basis).0)
                                .sum::<f64>(),
                        )
                    })
                },
            );
        }
    }

    let basis_3d = SMatrix::<f64, 3, 3>::from_row_slice(&[
        10.0, 3.5, 1.0,
        0.0, 8.0, 2.0,
        0.0, 0.0, 7.0,
    ]);
    let lattice_3d =
        PeriodicLattice::new(SVector::zeros(), basis_3d.clone()).expect("3D benchmark basis");
    for count in [32_usize, 64, 128] {
        let corpus = make_corpus_3d(count, 0x3344_5452_4943_4c49);
        preflight_3d(&lattice_3d, &corpus);
        black_box(run_3d(&lattice_3d, &corpus));

        group.bench_with_input(
            BenchmarkId::new("qr_sphere_decode_3d_moderate_skew", count),
            &corpus,
            |b, corpus| b.iter(|| black_box(run_3d(&lattice_3d, black_box(corpus)))),
        );
        group.bench_with_input(
            BenchmarkId::new("brute_force_3d_3375", count),
            &corpus,
            |b, corpus| {
                b.iter(|| {
                    black_box(
                        corpus
                            .iter()
                            .map(|displacement| brute_force_3d(displacement, &basis_3d).0)
                            .sum::<f64>(),
                    )
                })
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_triclinic_closest_image);
criterion_main!(benches);
