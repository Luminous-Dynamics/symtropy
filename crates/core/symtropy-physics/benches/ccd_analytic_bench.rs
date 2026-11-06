// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Microbenchmark for checked analytical CCD primitive queries.
//!
//! Measures the primitive query cost only. It does not include broadphase,
//! world integration, contact solving, or prove tunnelling prevention.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_physics::ccd::{
    sphere_halfspace, sphere_halfspace_checked, sphere_sphere, sphere_sphere_checked,
};

fn bench_analytic_ccd(c: &mut Criterion) {
    let mut group = c.benchmark_group("analytic_ccd_query");

    let pos_a = SVector::from([0.0, 0.0, 0.0]);
    let vel_a = SVector::from([10.0, 0.0, 0.0]);
    let pos_b = SVector::from([5.0, 0.0, 0.0]);
    let vel_b = SVector::from([-10.0, 0.0, 0.0]);

    group.bench_function("sphere_sphere_legacy_wrapper", |bench| {
        bench.iter(|| {
            black_box(sphere_sphere(
                black_box(&pos_a),
                black_box(&vel_a),
                black_box(0.5),
                black_box(&pos_b),
                black_box(&vel_b),
                black_box(0.5),
                black_box(1.0),
            ))
        });
    });
    group.bench_function("sphere_sphere_checked", |bench| {
        bench.iter(|| {
            black_box(sphere_sphere_checked(
                black_box(&pos_a),
                black_box(&vel_a),
                black_box(0.5),
                black_box(&pos_b),
                black_box(&vel_b),
                black_box(0.5),
                black_box(1.0),
            ))
        });
    });

    let plane_pos = SVector::from([0.0, 10.0, 0.0]);
    let plane_vel = SVector::from([0.0, -100.0, 0.0]);
    let unit_normal = SVector::from([0.0, 1.0, 0.0]);
    let non_unit_normal = SVector::from([0.0, 10.0, 0.0]);

    group.bench_function("sphere_halfspace_legacy_wrapper", |bench| {
        bench.iter(|| {
            black_box(sphere_halfspace(
                black_box(&plane_pos),
                black_box(&plane_vel),
                black_box(0.5),
                black_box(&unit_normal),
                black_box(0.0),
                black_box(1.0),
            ))
        });
    });
    group.bench_function("sphere_halfspace_checked_unit_normal", |bench| {
        bench.iter(|| {
            black_box(sphere_halfspace_checked(
                black_box(&plane_pos),
                black_box(&plane_vel),
                black_box(0.5),
                black_box(&unit_normal),
                black_box(0.0),
                black_box(1.0),
            ))
        });
    });
    group.bench_function("sphere_halfspace_checked_non_unit_normal", |bench| {
        bench.iter(|| {
            black_box(sphere_halfspace_checked(
                black_box(&plane_pos),
                black_box(&plane_vel),
                black_box(0.5),
                black_box(&non_unit_normal),
                black_box(0.0),
                black_box(1.0),
            ))
        });
    });

    for scale in [1e-6, 1.0, 1e6] {
        let p_a = SVector::from([0.0, 0.0, 0.0]);
        let v_a = SVector::from([10.0 * scale, 0.0, 0.0]);
        let p_b = SVector::from([5.0 * scale, 0.0, 0.0]);
        let v_b = SVector::from([-10.0 * scale, 0.0, 0.0]);
        group.bench_function(
            BenchmarkId::new("sphere_sphere_checked_scale", scale.to_string()),
            |bench| {
                bench.iter(|| {
                    black_box(sphere_sphere_checked(
                        black_box(&p_a),
                        black_box(&v_a),
                        black_box(0.5 * scale),
                        black_box(&p_b),
                        black_box(&v_b),
                        black_box(0.5 * scale),
                        black_box(1.0),
                    ))
                });
            },
        );
    }

    group.finish();
}

criterion_group!(benches, bench_analytic_ccd);
criterion_main!(benches);
