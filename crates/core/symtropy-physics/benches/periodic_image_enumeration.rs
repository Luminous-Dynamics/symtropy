// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Periodic AABB image candidate benchmark.
//!
//! The brute-force path is a deliberately simple, independent integer-image
//! enumerator for the fixed benchmark envelope. It is a reference baseline,
//! not a generally bounded production algorithm. Corpus generation and an
//! oracle-inclusion preflight happen outside the timed region.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::{LatticeOffset, PeriodicBox};
use symtropy_physics::broadphase::Aabb;
use symtropy_physics::periodic_broadphase::image_offsets_for_aabb_pair;

const PERIOD: f64 = 10.0;
const ORACLE_IMAGE_LIMIT: i64 = 8;
const MAX_CANDIDATES: usize = 64;

#[derive(Clone, Copy)]
enum FixtureKind {
    InteriorSeparated,
    SeamCrossing,
    MultiImage,
}

impl FixtureKind {
    fn label(self) -> &'static str {
        match self {
            Self::InteriorSeparated => "interior_separated",
            Self::SeamCrossing => "seam_crossing",
            Self::MultiImage => "multi_image",
        }
    }

    fn seed(self) -> u64 {
        match self {
            Self::InteriorSeparated => 0x5045_5249_4f44_4943,
            Self::SeamCrossing => 0x5345_414d_4352_4f53,
            Self::MultiImage => 0x4d55_4c54_4949_4d47,
        }
    }
}

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

fn bounds(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
    Aabb {
        min: SVector::from(min),
        max: SVector::from(max),
    }
}

fn overlap_direct(a: &Aabb<2>, b: &Aabb<2>) -> bool {
    a.max[0] >= b.min[0]
        && a.min[0] <= b.max[0]
        && a.max[1] >= b.min[1]
        && a.min[1] <= b.max[1]
}

fn brute_force_images(
    a: &Aabb<2>,
    b: &Aabb<2>,
) -> Vec<LatticeOffset<2>> {
    let mut images = Vec::new();
    for n in -ORACLE_IMAGE_LIMIT..=ORACLE_IMAGE_LIMIT {
        let shift = n as f64 * PERIOD;
        let shifted_b = bounds(
            [b.min[0] + shift, b.min[1]],
            [b.max[0] + shift, b.max[1]],
        );
        if overlap_direct(a, &shifted_b) {
            images.push(LatticeOffset::new([n, 0]));
        }
    }
    images
}

fn make_corpus(count: usize, kind: FixtureKind) -> Vec<(Aabb<2>, Aabb<2>)> {
    let mut state = kind.seed();
    let mut corpus = Vec::with_capacity(count);

    for _ in 0..count {
        let pair = match kind {
            FixtureKind::InteriorSeparated => {
                let ax = range(&mut state, 1.0, 3.0);
                let bx = range(&mut state, 6.0, 8.0);
                let ay = range(&mut state, -1.0, 1.0);
                let by = ay + range(&mut state, 1.0, 2.0);
                let aw = range(&mut state, 0.1, 0.5);
                let bw = range(&mut state, 0.1, 0.5);
                let ah = range(&mut state, 0.1, 0.5);
                let bh = range(&mut state, 0.1, 0.5);
                (
                    bounds([ax, ay], [ax + aw, ay + ah]),
                    bounds([bx, by], [bx + bw, by + bh]),
                )
            }
            FixtureKind::SeamCrossing => {
                // By construction, B's image at n=-1 overlaps A in x;
                // y-intervals also overlap for every generated pair.
                let ax = range(&mut state, 0.0, 0.15);
                let bx = range(&mut state, 9.8, 10.0);
                let ay = range(&mut state, -1.0, 1.0);
                let by = ay + range(&mut state, -0.05, 0.05);
                let aw = range(&mut state, 0.5, 1.0);
                let bw = range(&mut state, 0.8, 1.2);
                let ah = range(&mut state, 0.2, 0.7);
                let bh = range(&mut state, 0.2, 0.7);
                (
                    bounds([ax, ay], [ax + aw, ay + ah]),
                    bounds([bx, by], [bx + bw, by + bh]),
                )
            }
            FixtureKind::MultiImage => {
                let ax = range(&mut state, -3.0, 3.0);
                let bx = range(&mut state, -3.0, 3.0);
                let ay = range(&mut state, -1.0, 1.0);
                let by = range(&mut state, -1.0, 1.0);
                let aw = range(&mut state, 18.0, 24.0);
                let bw = range(&mut state, 18.0, 24.0);
                let ah = range(&mut state, 2.0, 4.0);
                let bh = range(&mut state, 2.0, 4.0);
                (
                    bounds([ax, ay], [ax + aw, ay + ah]),
                    bounds([bx, by], [bx + bw, by + bh]),
                )
            }
        };
        corpus.push(pair);
    }
    corpus
}

fn count_range_formula(
    domain: &PeriodicBox<2>,
    corpus: &[(Aabb<2>, Aabb<2>)],
) -> usize {
    corpus
        .iter()
        .map(|(a, b)| {
            image_offsets_for_aabb_pair(domain, a, b, MAX_CANDIDATES)
                .expect("benchmark corpus must fit the configured image budget")
                .len()
        })
        .sum()
}

fn count_brute_force(corpus: &[(Aabb<2>, Aabb<2>)]) -> usize {
    corpus
        .iter()
        .map(|(a, b)| brute_force_images(a, b).len())
        .sum()
}

fn verify_oracle_inclusion(
    domain: &PeriodicBox<2>,
    corpus: &[(Aabb<2>, Aabb<2>)],
) -> (usize, usize) {
    let mut production_count = 0_usize;
    let mut oracle_count = 0_usize;
    for (case, (a, b)) in corpus.iter().enumerate() {
        let actual = image_offsets_for_aabb_pair(domain, a, b, MAX_CANDIDATES)
            .expect("preflight image candidates must fit the budget");
        let expected = brute_force_images(a, b);
        production_count += actual.len();
        oracle_count += expected.len();
        for image in &expected {
            assert!(
                actual.contains(image),
                "range formula omitted required image in case {case}: {image:?}"
            );
        }
    }
    assert!(
        production_count >= oracle_count,
        "conservative candidate set cannot be smaller than the oracle set"
    );
    (production_count, oracle_count)
}

fn bench_periodic_image_enumeration(c: &mut Criterion) {
    let domain = PeriodicBox::new(
        SVector::zeros(),
        SVector::from([PERIOD, 0.0]),
        [true, false],
    )
    .expect("fixed benchmark domain is valid");

    let mut group = c.benchmark_group("periodic_aabb_image_enumeration");
    for count in [64_usize, 256, 1_024] {
        for kind in [
            FixtureKind::InteriorSeparated,
            FixtureKind::SeamCrossing,
            FixtureKind::MultiImage,
        ] {
            let corpus = make_corpus(count, kind);
            let (production_candidates, oracle_candidates) =
                verify_oracle_inclusion(&domain, &corpus);
            let extra_candidates = production_candidates - oracle_candidates;
            eprintln!(
                "PERIODIC_IMAGE_PREFLIGHT,v1,fixture={},pairs={},oracle_candidates={},production_candidates={},extra_candidates={}",
                kind.label(),
                count,
                oracle_candidates,
                production_candidates,
                extra_candidates
            );
            black_box(count_brute_force(&corpus));
            black_box(count_range_formula(&domain, &corpus));

            let case_label = format!("{}/pairs_{count}", kind.label());
            group.bench_with_input(
                BenchmarkId::new("range_formula", case_label.as_str()),
                &corpus,
                |b, corpus| {
                    b.iter(|| black_box(count_range_formula(&domain, black_box(corpus))))
                },
            );
            group.bench_with_input(
                BenchmarkId::new("brute_force_17_images", case_label.as_str()),
                &corpus,
                |b, corpus| b.iter(|| black_box(count_brute_force(black_box(corpus)))),
            );
        }
    }
    group.finish();
}

criterion_group!(benches, bench_periodic_image_enumeration);
criterion_main!(benches);
