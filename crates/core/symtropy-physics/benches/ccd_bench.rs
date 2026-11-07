// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Reproducible throughput baseline for analytical CCD primitives.
//!
//! The cases are frozen at benchmark setup and cover a family of sphere-sphere
//! sweeps with varying range and grazing offset, plus sphere-halfspace sweeps.
//! Criterion reports queries per second. This is an internal
//! throughput baseline; cross-engine claims require the same case corpus,
//! tolerances, units and result semantics in the external adapter.

use criterion::{BenchmarkId, Criterion, Throughput, black_box, criterion_group, criterion_main};
use nalgebra::SVector;
use symtropy_physics::ccd::{try_sphere_halfspace, try_sphere_sphere};

const CASE_COUNT: usize = 512;

#[derive(Clone)]
struct SphereCase {
    pos_a: SVector<f64, 3>,
    vel_a: SVector<f64, 3>,
    radius_a: f64,
    pos_b: SVector<f64, 3>,
    vel_b: SVector<f64, 3>,
    radius_b: f64,
    dt: f64,
}

#[derive(Clone)]
struct HalfspaceCase {
    position: SVector<f64, 3>,
    velocity: SVector<f64, 3>,
    radius: f64,
    normal: SVector<f64, 3>,
    offset: f64,
    dt: f64,
}

fn sphere_cases() -> Vec<SphereCase> {
    (0..CASE_COUNT)
        .map(|index| SphereCase {
            pos_a: SVector::zeros(),
            vel_a: SVector::from([30.0, 0.0, 0.0]),
            radius_a: 0.75,
            pos_b: SVector::from([80.0 + index as f64 * 0.05, (index % 17) as f64 * 0.05, 0.0]),
            vel_b: SVector::from([-1.0, 0.0, 0.0]),
            radius_b: 0.25,
            dt: 10.0,
        })
        .collect()
}

fn halfspace_cases() -> Vec<HalfspaceCase> {
    (0..CASE_COUNT)
        .map(|index| HalfspaceCase {
            position: SVector::from([index as f64 * 0.1, 50.0 + index as f64 * 0.01, 0.0]),
            velocity: SVector::from([0.0, -25.0, 0.0]),
            radius: 0.5,
            normal: SVector::from([0.0, 1.0, 0.0]),
            offset: 0.0,
            dt: 10.0,
        })
        .collect()
}

fn bench_ccd(c: &mut Criterion) {
    let sphere_cases = sphere_cases();
    let halfspace_cases = halfspace_cases();

    let mut group = c.benchmark_group("ccd_analytic_queries");
    group.throughput(Throughput::Elements(CASE_COUNT as u64));

    group.bench_function(
        BenchmarkId::new("sphere_sphere_mixed_512", CASE_COUNT),
        |bench| {
            bench.iter(|| {
                let mut hits = 0_u64;
                let mut toi_sum = 0.0;
                for case in &sphere_cases {
                    if let Some(hit) = try_sphere_sphere(
                        &case.pos_a,
                        &case.vel_a,
                        case.radius_a,
                        &case.pos_b,
                        &case.vel_b,
                        case.radius_b,
                        case.dt,
                    )
                    .expect("frozen finite CCD inputs must be valid")
                    {
                        hits += 1;
                        toi_sum += hit.toi;
                    }
                }
                black_box((hits, toi_sum))
            });
        },
    );

    group.bench_function(
        BenchmarkId::new("sphere_halfspace_mixed_512", CASE_COUNT),
        |bench| {
            bench.iter(|| {
                let mut hits = 0_u64;
                let mut toi_sum = 0.0;
                for case in &halfspace_cases {
                    if let Some(hit) = try_sphere_halfspace(
                        &case.position,
                        &case.velocity,
                        case.radius,
                        &case.normal,
                        case.offset,
                        case.dt,
                    )
                    .expect("frozen finite CCD inputs must be valid")
                    {
                        hits += 1;
                        toi_sum += hit.toi;
                    }
                }
                black_box((hits, toi_sum))
            });
        },
    );

    group.finish();
}

criterion_group!(benches, bench_ccd);
criterion_main!(benches);
