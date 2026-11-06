// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Checked CCD query microbenchmarks.
//!
//! These benchmarks measure the cost of analytic query execution, not the
//! world-level false-negative rate or physical validity. The matching
//! qualification corpus lives under research/physics-corpus-v0.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_physics::ccd::{
    should_sweep_sphere, sphere_halfspace, sphere_halfspace_checked, sphere_sphere,
    sphere_sphere_checked,
};

fn bench_sphere_sphere(c: &mut Criterion) {
    let mut group = c.benchmark_group("ccd_sphere_sphere");
    group.throughput(Throughput::Elements(1));

    let pos_a = SVector::from([0.0, 0.0, 0.0]);
    let vel_a = SVector::from([10.0, 0.0, 0.0]);
    let pos_b = SVector::from([5.0, 0.0, 0.0]);
    let vel_b = SVector::zeros();

    group.bench_function(
        BenchmarkId::new("checked_head_on_3d", "unit_scale"),
        |bench| {
            bench.iter(|| {
                black_box(sphere_sphere_checked(
                    black_box(&pos_a),
                    black_box(&vel_a),
                    0.5,
                    black_box(&pos_b),
                    black_box(&vel_b),
                    0.5,
                    1.0,
                ))
            });
        },
    );

    group.bench_function(
        BenchmarkId::new("compat_head_on_3d", "unit_scale"),
        |bench| {
            bench.iter(|| {
                black_box(sphere_sphere(
                    black_box(&pos_a),
                    black_box(&vel_a),
                    0.5,
                    black_box(&pos_b),
                    black_box(&vel_b),
                    0.5,
                    1.0,
                ))
            });
        },
    );

    let distant_pos_b = SVector::from([1_000_000.0, 0.0, 0.0]);
    let high_speed = SVector::from([1_000_000.0, 0.0, 0.0]);
    group.bench_function(
        BenchmarkId::new("checked_large_separation_3d", "1e6"),
        |bench| {
            bench.iter(|| {
                black_box(sphere_sphere_checked(
                    black_box(&pos_a),
                    black_box(&high_speed),
                    0.5,
                    black_box(&distant_pos_b),
                    black_box(&vel_b),
                    0.5,
                    1.0,
                ))
            });
        },
    );

    let grazing_pos_b = SVector::from([5.0, 1.0, 0.0]);
    group.bench_function(BenchmarkId::new("checked_grazing_3d", "tangent"), |bench| {
        bench.iter(|| {
            black_box(sphere_sphere_checked(
                black_box(&pos_a),
                black_box(&vel_a),
                0.5,
                black_box(&grazing_pos_b),
                black_box(&vel_b),
                0.5,
                1.0,
            ))
        });
    });

    group.finish();
}

fn bench_sphere_halfspace(c: &mut Criterion) {
    let mut group = c.benchmark_group("ccd_sphere_halfspace");
    group.throughput(Throughput::Elements(1));

    let position = SVector::from([0.0, 5.0, 0.0]);
    let velocity = SVector::from([0.0, -500.0, 0.0]);
    let unit_normal = SVector::from([0.0, 1.0, 0.0]);
    let scaled_normal = SVector::from([0.0, 17.0, 0.0]);

    group.bench_function(
        BenchmarkId::new("checked_unit_normal_3d", "scale_1"),
        |bench| {
            bench.iter(|| {
                black_box(sphere_halfspace_checked(
                    black_box(&position),
                    black_box(&velocity),
                    0.5,
                    black_box(&unit_normal),
                    0.0,
                    0.016,
                ))
            });
        },
    );

    group.bench_function(
        BenchmarkId::new("checked_scaled_normal_3d", "scale_17"),
        |bench| {
            bench.iter(|| {
                black_box(sphere_halfspace_checked(
                    black_box(&position),
                    black_box(&velocity),
                    0.5,
                    black_box(&scaled_normal),
                    0.0,
                    0.016,
                ))
            });
        },
    );

    group.bench_function(
        BenchmarkId::new("compat_unit_normal_3d", "scale_1"),
        |bench| {
            bench.iter(|| {
                black_box(sphere_halfspace(
                    black_box(&position),
                    black_box(&velocity),
                    0.5,
                    black_box(&unit_normal),
                    0.0,
                    0.016,
                ))
            });
        },
    );

    group.finish();
}

fn bench_sweep_activation(c: &mut Criterion) {
    let mut group = c.benchmark_group("ccd_sweep_activation");
    group.throughput(Throughput::Elements(1));

    group.bench_function("small_radius_sub_threshold", |bench| {
        bench.iter(|| black_box(should_sweep_sphere(1.0, 0.001, 0.016)));
    });
    group.bench_function("large_radius_low_displacement", |bench| {
        bench.iter(|| black_box(should_sweep_sphere(1.0, 0.5, 0.016)));
    });
    group.bench_function("absolute_speed_guard", |bench| {
        bench.iter(|| black_box(should_sweep_sphere(6.0, 0.5, 0.016)));
    });

    group.finish();
}

criterion_group!(
    benches,
    bench_sphere_sphere,
    bench_sphere_halfspace,
    bench_sweep_activation
);
criterion_main!(benches);
