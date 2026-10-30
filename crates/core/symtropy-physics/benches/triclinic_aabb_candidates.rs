// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Differential benchmark for triclinic AABB lattice-image candidate queries.
//!
//! The brute-force path directly translates every image in a frozen integer
//! cube. An untimed preflight first proves that the production candidate set
//! contains every true brute-force overlap. Conservative extras are counted.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector};
use symtropy_math::PeriodicLattice;
use symtropy_physics::broadphase::Aabb;
use symtropy_physics::periodic_lattice_broadphase::lattice_image_offsets_for_aabb_pair;

const MAX_CANDIDATES: usize = 4_096;
const ORACLE_RADIUS_2D: i64 = 10;
const ORACLE_RADIUS_3D: i64 = 8;

fn next_random(state: &mut u64) -> u64 {
    *state = state
        .wrapping_mul(6_364_136_223_846_793_005)
        .wrapping_add(1_442_695_040_888_963_407);
    *state
}

fn random_range(state: &mut u64, low: f64, high: f64) -> f64 {
    let bits = next_random(state) >> 11;
    low + (high - low) * (bits as f64 / ((1_u64 << 53) as f64))
}

fn aabb2(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
    Aabb {
        min: SVector::from(min),
        max: SVector::from(max),
    }
}

fn aabb3(min: [f64; 3], max: [f64; 3]) -> Aabb<3> {
    Aabb {
        min: SVector::from(min),
        max: SVector::from(max),
    }
}

fn overlap<const D: usize>(a: &Aabb<D>, b: &Aabb<D>) -> bool {
    (0..D).all(|axis| a.max[axis] >= b.min[axis] && a.min[axis] <= b.max[axis])
}

fn brute_force_2d(
    a: &Aabb<2>,
    b: &Aabb<2>,
    basis: &SMatrix<f64, 2, 2>,
) -> usize {
    let mut count = 0;
    for x in -ORACLE_RADIUS_2D..=ORACLE_RADIUS_2D {
        for y in -ORACLE_RADIUS_2D..=ORACLE_RADIUS_2D {
            let shift = basis * SVector::from([x as f64, y as f64]);
            let translated = Aabb {
                min: b.min + shift,
                max: b.max + shift,
            };
            count += usize::from(overlap(a, &translated));
        }
    }
    count
}

fn brute_force_3d(
    a: &Aabb<3>,
    b: &Aabb<3>,
    basis: &SMatrix<f64, 3, 3>,
) -> usize {
    let mut count = 0;
    for x in -ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D {
        for y in -ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D {
            for z in -ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D {
                let shift = basis * SVector::from([x as f64, y as f64, z as f64]);
                let translated = Aabb {
                    min: b.min + shift,
                    max: b.max + shift,
                };
                count += usize::from(overlap(a, &translated));
            }
        }
    }
    count
}

fn corpus_2d(count: usize, seed: u64) -> Vec<(Aabb<2>, Aabb<2>)> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            let ax = random_range(&mut state, -3.0, 3.0);
            let ay = random_range(&mut state, -3.0, 3.0);
            let bx = random_range(&mut state, -3.0, 3.0);
            let by = random_range(&mut state, -3.0, 3.0);
            let aw = random_range(&mut state, 0.1, 0.9);
            let ah = random_range(&mut state, 0.1, 0.9);
            let bw = random_range(&mut state, 0.1, 0.9);
            let bh = random_range(&mut state, 0.1, 0.9);
            (
                aabb2([ax, ay], [ax + aw, ay + ah]),
                aabb2([bx, by], [bx + bw, by + bh]),
            )
        })
        .collect()
}

fn corpus_3d(count: usize, seed: u64) -> Vec<(Aabb<3>, Aabb<3>)> {
    let mut state = seed;
    (0..count)
        .map(|_| {
            let ax = random_range(&mut state, -2.5, 2.5);
            let ay = random_range(&mut state, -2.5, 2.5);
            let az = random_range(&mut state, -2.5, 2.5);
            let bx = random_range(&mut state, -2.5, 2.5);
            let by = random_range(&mut state, -2.5, 2.5);
            let bz = random_range(&mut state, -2.5, 2.5);
            let aw = random_range(&mut state, 0.1, 0.7);
            let ah = random_range(&mut state, 0.1, 0.7);
            let ad = random_range(&mut state, 0.1, 0.7);
            let bw = random_range(&mut state, 0.1, 0.7);
            let bh = random_range(&mut state, 0.1, 0.7);
            let bd = random_range(&mut state, 0.1, 0.7);
            (
                aabb3([ax, ay, az], [ax + aw, ay + ah, az + ad]),
                aabb3([bx, by, bz], [bx + bw, by + bh, bz + bd]),
            )
        })
        .collect()
}

fn run_projection_2d(
    lattice: &PeriodicLattice<2>,
    corpus: &[(Aabb<2>, Aabb<2>)],
) -> usize {
    corpus.iter().map(|(a, b)| {
        lattice_image_offsets_for_aabb_pair(lattice, a, b, MAX_CANDIDATES)
            .expect("frozen 2D corpus must fit budget")
            .len()
    }).sum()
}

fn run_brute_2d(corpus: &[(Aabb<2>, Aabb<2>)], basis: &SMatrix<f64, 2, 2>) -> usize {
    corpus.iter().map(|(a, b)| brute_force_2d(a, b, basis)).sum()
}

fn preflight_2d(
    label: &str,
    lattice: &PeriodicLattice<2>,
    corpus: &[(Aabb<2>, Aabb<2>)],
    basis: &SMatrix<f64, 2, 2>,
) -> (usize, usize) {
    let mut production = 0;
    let mut oracle = 0;
    for (case, (a, b)) in corpus.iter().enumerate() {
        let actual = lattice_image_offsets_for_aabb_pair(lattice, a, b, MAX_CANDIDATES)
            .unwrap_or_else(|error| panic!("{label} case={case}: {error}"));
        let actual_set = actual.iter().map(|offset| offset.periods).collect::<std::collections::BTreeSet<_>>();
        let expected_count = brute_force_2d(a, b, basis);
        let expected_set = (-ORACLE_RADIUS_2D..=ORACLE_RADIUS_2D)
            .flat_map(|x| (-ORACLE_RADIUS_2D..=ORACLE_RADIUS_2D).map(move |y| [x, y]))
            .filter(|offset| {
                let shift = basis * SVector::from([offset[0] as f64, offset[1] as f64]);
                let translated = Aabb {
                    min: b.min + shift,
                    max: b.max + shift,
                };
                overlap(a, &translated)
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(
            actual_set.is_superset(&expected_set),
            "{label} omitted a true image in case={case}"
        );
        production += actual.len();
        oracle += expected_count;
    }
    assert!(production >= oracle);
    eprintln!(
        "TRICLINIC_AABB_PREFLIGHT,v1,dimension=2,basis={},pairs={},oracle_overlaps={},production_candidates={},extras={}",
        label,
        corpus.len(),
        oracle,
        production,
        production - oracle
    );
    (production, oracle)
}

fn preflight_3d(
    lattice: &PeriodicLattice<3>,
    corpus: &[(Aabb<3>, Aabb<3>)],
    basis: &SMatrix<f64, 3, 3>,
) -> (usize, usize) {
    let mut production = 0;
    let mut oracle = 0;
    for (case, (a, b)) in corpus.iter().enumerate() {
        let actual = lattice_image_offsets_for_aabb_pair(lattice, a, b, MAX_CANDIDATES)
            .unwrap_or_else(|error| panic!("3D case={case}: {error}"));
        let actual_set = actual.iter().map(|offset| offset.periods).collect::<std::collections::BTreeSet<_>>();
        let expected_count = brute_force_3d(a, b, basis);
        let expected_set = (-ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D)
            .flat_map(|x| (-ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D)
                .flat_map(move |y| (-ORACLE_RADIUS_3D..=ORACLE_RADIUS_3D).map(move |z| [x,y,z])))
            .filter(|offset| {
                let shift = basis * SVector::from([
                    offset[0] as f64,
                    offset[1] as f64,
                    offset[2] as f64,
                ]);
                let translated = Aabb {
                    min: b.min + shift,
                    max: b.max + shift,
                };
                overlap(a, &translated)
            })
            .collect::<std::collections::BTreeSet<_>>();
        assert!(actual_set.is_superset(&expected_set), "3D missed image case={case}");
        production += actual.len();
        oracle += expected_count;
    }
    assert!(production >= oracle);
    eprintln!(
        "TRICLINIC_AABB_PREFLIGHT,v1,dimension=3,basis=moderate_skew,pairs={},oracle_overlaps={},production_candidates={},extras={}",
        corpus.len(),
        oracle,
        production,
        production - oracle
    );
    (production, oracle)
}

fn bench_triclinic_aabb_candidates(c: &mut Criterion) {
    let mut group = c.benchmark_group("triclinic_aabb_lattice_images");
    let bases = [
        ("moderate_skew", SMatrix::<f64, 2, 2>::from_row_slice(&[2.0, 0.8, 0.0, 1.5])),
        ("stronger_skew", SMatrix::<f64, 2, 2>::from_row_slice(&[2.0, 1.35, 0.0, 1.1])),
    ];

    for (label, basis) in bases {
        let lattice = PeriodicLattice::new(SVector::zeros(), basis.clone()).unwrap();
        for count in [64_usize, 256, 1_024] {
            let corpus = corpus_2d(count, 0x5452_4943_4c49_4e49);
            preflight_2d(label, &lattice, &corpus, &basis);
            black_box(run_projection_2d(&lattice, &corpus));
            black_box(run_brute_2d(&corpus, &basis));

            group.bench_with_input(
                BenchmarkId::new(format!("projection/{label}"), count),
                &corpus,
                |b, corpus| b.iter(|| black_box(run_projection_2d(&lattice, black_box(corpus)))),
            );
            group.bench_with_input(
                BenchmarkId::new(format!("brute_force_2d/{label}"), count),
                &corpus,
                |b, corpus| b.iter(|| black_box(run_brute_2d(black_box(corpus), &basis))),
            );
        }
    }

    let basis_3d = SMatrix::<f64, 3, 3>::from_row_slice(&[
        2.0, 0.7, 0.2,
        0.0, 1.6, 0.5,
        0.0, 0.0, 1.3,
    ]);
    let lattice_3d = PeriodicLattice::new(SVector::zeros(), basis_3d.clone()).unwrap();
    for count in [32_usize, 64, 128] {
        let corpus = corpus_3d(count, 0x3344_5452_4943_4c49);
        preflight_3d(&lattice_3d, &corpus, &basis_3d);
        group.bench_with_input(
            BenchmarkId::new("projection_3d", count),
            &corpus,
            |b, corpus| {
                b.iter(|| {
                    black_box(corpus.iter().map(|(a,b)| {
                        lattice_image_offsets_for_aabb_pair(&lattice_3d, a, b, MAX_CANDIDATES)
                            .expect("preflight budget")
                            .len()
                    }).sum::<usize>())
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("brute_force_3d", count),
            &corpus,
            |b, corpus| b.iter(|| black_box(corpus.iter().map(|(a,b)| brute_force_3d(a,b,&basis_3d)).sum::<usize>())),
        );
    }
    group.finish();
}

criterion_group!(benches, bench_triclinic_aabb_candidates);
criterion_main!(benches);
