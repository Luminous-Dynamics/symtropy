// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Certified closest-lattice-vector benchmark for triclinic cells.
//!
//! The brute-force path is an independent bounded integer-offset oracle for
//! the frozen fixture envelope. Every corpus is checked before timing. The
//! benchmark reports certification candidate counts; no speedup is inferred
//! outside these matrices and displacement distributions.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector};
use symtropy_math::PeriodicLattice;

const MAX_CANDIDATES: usize = 729;
const ORACLE_RADIUS: i64 = 12;

fn next_u64(state: &mut u64) -> u64 {
    *state = (*state)
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

fn make_corpus<const D: usize>(
    lattice: &PeriodicLattice<D>,
    count: usize,
    seed: u64,
) -> Vec<SVector<f64, D>> {
    let mut state = seed;
    let mut corpus = Vec::with_capacity(count);
    for case in 0..count {
        let fractional = if case % 4 == 0 {
            // Deliberately place points near fractional Voronoi boundaries,
            // where component rounding is least trustworthy in a skew cell.
            SVector::<f64, D>::from(std::array::from_fn(|axis| {
                let half = if axis % 2 == 0 { 0.5 } else { -0.5 };
                half + range(&mut state, -0.03, 0.03)
            }))
        } else {
            SVector::<f64, D>::from(std::array::from_fn(|_| {
                range(&mut state, -10.0, 10.0)
            }))
        };
        corpus.push(lattice.basis() * fractional);
    }
    corpus
}

fn brute_force<const D: usize>(
    lattice: &PeriodicLattice<D>,
    displacement: &SVector<f64, D>,
    radius: i64,
) -> f64 {

    let fractional = lattice.inverse_basis() * displacement;
    let center = std::array::from_fn(|axis| fractional[axis].round() as i64);
    let mut offset = [0_i64; D];
    let mut best_squared = f64::INFINITY;

    // Enumerate a fixed cube around the rounded fractional coordinates. This
    // deliberately does not call the production shell enumeration.
    fn visit_centered<const D: usize>(
        axis: usize,
        center: &[i64; D],
        offset: &mut [i64; D],
        lattice: &PeriodicLattice<D>,
        displacement: &SVector<f64, D>,
        radius: i64,
        best_squared: &mut f64,
    ) {
        if axis == D {
            let translation = lattice
                .offset_vector(symtropy_math::LatticeOffset::new(*offset))
                .expect("oracle offset must be valid");
            let residual = displacement - translation;
            *best_squared = best_squared.min(residual.norm_squared());
            return;
        }
        for value in (center[axis] - radius)..=(center[axis] + radius) {
            offset[axis] = value;
            visit_centered(
                axis + 1,
                center,
                offset,
                lattice,
                displacement,
                radius,
                best_squared,
            );
        }
    }

    visit_centered(
        0,
        &center,
        &mut offset,
        lattice,
        displacement,
        radius,
        &mut best_squared,
    );
    best_squared.sqrt()
}

fn preflight<const D: usize>(
    label: &str,
    lattice: &PeriodicLattice<D>,
    corpus: &[SVector<f64, D>],
) -> (usize, f64) {
    let mut candidate_total = 0_usize;
    let mut max_candidates = 0_usize;
    let mut max_error = 0.0_f64;
    let mut max_distance_error_bound = 0.0_f64;
    let mut ambiguous_queries = 0_usize;
    let mut max_ambiguous_images = 0_usize;
    for (case, displacement) in corpus.iter().enumerate() {
        let result = lattice
            .minimum_image_displacement(displacement, MAX_CANDIDATES)
            .unwrap_or_else(|error| panic!("{label} case {case}: {error}"));
        let oracle = brute_force(lattice, displacement, ORACLE_RADIUS);
        let error = (result.displacement.norm() - oracle).abs();
        assert!(
            error <= 1e-9,
            "{label} case {case}: certified={} oracle={} error={error}",
            result.displacement.norm(),
            oracle
        );
        assert!(
            result.unsearched_distance_lower_bound
                > result.best_candidate_distance_upper_bound,
            "certificate lower bound must exceed all evaluated candidate upper bounds"
        );
        if !result.ambiguous_image_offsets.is_empty() {
            ambiguous_queries += 1;
        }
        max_ambiguous_images =
            max_ambiguous_images.max(result.ambiguous_image_offsets.len());
        candidate_total += result.candidates_evaluated;
        max_candidates = max_candidates.max(result.candidates_evaluated);
        max_error = max_error.max(error);
        max_distance_error_bound =
            max_distance_error_bound.max(result.nearest_distance_error_bound);
    }
    let mean_candidates = candidate_total / corpus.len().max(1);
    eprintln!(
        "TRICLINIC_CVP_PREFLIGHT,v3,fixture={label},queries={},mean_candidates={},max_candidates={},max_error={:.6e},max_distance_error_bound={:.6e},inverse_residual_upper_bound={:.6e},ambiguous_queries={},max_ambiguous_images={},oracle_radius={ORACLE_RADIUS}",
        corpus.len(),
        mean_candidates,
        max_candidates,
        max_error,
        max_distance_error_bound,
        lattice.inverse_residual_upper_bound(),
        ambiguous_queries,
        max_ambiguous_images
    );
    (mean_candidates, max_error)
}

fn bench_2d(c: &mut Criterion) {
    let basis = SMatrix::<f64, 2, 2>::new(10.0, 8.5, 0.0, 3.0);
    let lattice =
        PeriodicLattice::new(SVector::zeros(), basis, [true, true]).expect("2D basis valid");
    let mut group = c.benchmark_group("triclinic_minimum_image_2d");
    for count in [64_usize, 256, 1_024] {
        let corpus = make_corpus(&lattice, count, 0x5452_4943_4c49_4e49);
        preflight("skew2d", &lattice, &corpus);
        group.bench_with_input(
            BenchmarkId::new("certified_shell", count),
            &corpus,
            |b, corpus| {
                b.iter(|| {
                    for displacement in corpus {
                        black_box(
                            lattice
                                .minimum_image_displacement(black_box(displacement), MAX_CANDIDATES)
                                .expect("preflight corpus must remain certifiable"),
                        );
                    }
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("brute_force_25x25", count),
            &corpus,
            |b, corpus| {
                b.iter(|| {
                    for displacement in corpus {
                        black_box(brute_force(&lattice, black_box(displacement), ORACLE_RADIUS));
                    }
                })
            },
        );
    }
    group.finish();
}

fn bench_3d(c: &mut Criterion) {
    let basis = SMatrix::<f64, 3, 3>::new(
        10.0, 2.0, -1.0,
        0.0, 9.0, 1.0,
        0.0, 0.0, 8.0,
    );
    let lattice =
        PeriodicLattice::new(SVector::zeros(), basis, [true, true, true]).expect("3D basis valid");
    let mut group = c.benchmark_group("triclinic_minimum_image_3d");
    for count in [16_usize, 64, 128] {
        let corpus = make_corpus(&lattice, count, 0x3344_4455_6666_7777);
        preflight("skew3d", &lattice, &corpus);
        group.bench_with_input(
            BenchmarkId::new("certified_shell", count),
            &corpus,
            |b, corpus| {
                b.iter(|| {
                    for displacement in corpus {
                        black_box(
                            lattice
                                .minimum_image_displacement(black_box(displacement), MAX_CANDIDATES)
                                .expect("preflight corpus must remain certifiable"),
                        );
                    }
                })
            },
        );
        group.bench_with_input(
            BenchmarkId::new("brute_force_25x25x25", count),
            &corpus,
            |b, corpus| {
                b.iter(|| {
                    for displacement in corpus {
                        black_box(brute_force(&lattice, black_box(displacement), ORACLE_RADIUS));
                    }
                })
            },
        );
    }
    group.finish();
}

criterion_group!(benches, bench_2d, bench_3d);
criterion_main!(benches);
