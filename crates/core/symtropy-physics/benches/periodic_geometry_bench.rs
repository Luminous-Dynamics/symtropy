// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Periodic broadphase candidate benchmark against an exhaustive image oracle.
//!
//! The oracle intentionally does not call the production translation or
//! interval-bound helpers. Cases cover small seam contacts, multi-image bodies,
//! and 3D image-lattice growth.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_math::PeriodicBox;
use symtropy_physics::{Aabb, image_offsets_for_aabb_pair};
use std::collections::BTreeSet;

const ORACLE_RADIUS: i64 = 12;

fn direct_overlap<const D: usize>(
    a: &Aabb<D>,
    b: &Aabb<D>,
    periods: &SVector<f64, D>,
    offset: &[i64; D],
) -> bool {
    for axis in 0..D {
        let shift = offset[axis] as f64 * periods[axis];
        if a.max[axis] < b.min[axis] + shift
            || a.min[axis] > b.max[axis] + shift
        {
            return false;
        }
    }
    true
}

fn brute_force<const D: usize>(
    a: &Aabb<D>,
    b: &Aabb<D>,
    periods: &SVector<f64, D>,
) -> BTreeSet<[i64; D]> {
    let mut output = BTreeSet::new();
    let mut offset = [0_i64; D];

    fn visit<const D: usize>(
        axis: usize,
        a: &Aabb<D>,
        b: &Aabb<D>,
        periods: &SVector<f64, D>,
        offset: &mut [i64; D],
        output: &mut BTreeSet<[i64; D]>,
    ) {
        if axis == D {
            if direct_overlap(a, b, periods, offset) {
                output.insert(*offset);
            }
            return;
        }
        for image in -ORACLE_RADIUS..=ORACLE_RADIUS {
            offset[axis] = image;
            visit(axis + 1, a, b, periods, offset, output);
        }
    }

    visit(0, a, b, periods, &mut offset, &mut output);
    output
}

fn bench_pair_2d(c: &mut Criterion) {
    let periods = SVector::from([10.0, 10.0]);
    let domain = PeriodicBox::new(SVector::zeros(), periods, [true, true]).unwrap();
    let cases = [
        (
            "seam-small",
            Aabb {
                min: SVector::from([0.0, 0.0]),
                max: SVector::from([0.5, 0.5]),
            },
            Aabb {
                min: SVector::from([9.5, 9.5]),
                max: SVector::from([10.0, 10.0]),
            },
        ),
        (
            "multi-image",
            Aabb {
                min: SVector::from([-10.0, -10.0]),
                max: SVector::from([10.0, 10.0]),
            },
            Aabb {
                min: SVector::from([-8.0, -8.0]),
                max: SVector::from([8.0, 8.0]),
            },
        ),
    ];

    let mut group = c.benchmark_group("periodic_image_enumeration/2d");
    for (name, a, b) in cases {
        let reference = brute_force(&a, &b, &periods);
        group.throughput(Throughput::Elements(reference.len() as u64));

        group.bench_with_input(BenchmarkId::new("production", name), &name, |bencher, _| {
            bencher.iter(|| {
                black_box(
                    image_offsets_for_aabb_pair(&domain, black_box(&a), black_box(&b), 4096)
                        .expect("candidate budget covers fixed benchmark case"),
                )
            });
        });

        group.bench_with_input(BenchmarkId::new("exhaustive-oracle", name), &name, |bencher, _| {
            bencher.iter(|| black_box(brute_force(black_box(&a), black_box(&b), &periods)));
        });
    }
    group.finish();
}

fn bench_pair_3d(c: &mut Criterion) {
    let periods = SVector::from([10.0, 10.0, 10.0]);
    let domain = PeriodicBox::new(SVector::zeros(), periods, [true, true, true]).unwrap();

    let a = Aabb {
        min: SVector::from([-15.0, -15.0, -15.0]),
        max: SVector::from([15.0, 15.0, 15.0]),
    };
    let b = Aabb {
        min: SVector::from([-12.0, -12.0, -12.0]),
        max: SVector::from([12.0, 12.0, 12.0]),
    };
    let reference = brute_force(&a, &b, &periods);
    assert_eq!(
        image_offsets_for_aabb_pair(&domain, &a, &b, 4096)
            .unwrap()
            .len(),
        reference.len(),
        "benchmark preflight: production candidate count differs from exhaustive oracle"
    );

    let mut group = c.benchmark_group("periodic_image_enumeration/3d");
    group.throughput(Throughput::Elements(reference.len() as u64));
    group.bench_function("production/multi-image", |bencher| {
        bencher.iter(|| {
            black_box(
                image_offsets_for_aabb_pair(&domain, black_box(&a), black_box(&b), 4096)
                    .expect("candidate budget covers fixed benchmark case"),
            )
        });
    });
    group.bench_function("exhaustive-oracle/multi-image", |bencher| {
        bencher.iter(|| black_box(brute_force(black_box(&a), black_box(&b), &periods)));
    });
    group.finish();
}

criterion_group!(benches, bench_pair_2d, bench_pair_3d);
criterion_main!(benches);
