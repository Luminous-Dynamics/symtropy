// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{DVector, Matrix2, SVector};
use symtropy_math::{Bivector, Point, Rotor};
use symtropy_physics::{
    BivectorInertia, BodyHandle, FrictionlessContactRow, RigidBody, assemble_delassus,
    solve_frictionless_lcp, solve_planar_friction_cone,
};

fn fixture(
    row_count: usize,
) -> (
    Vec<RigidBody<3>>,
    Vec<FrictionlessContactRow<3>>,
    nalgebra::DMatrix<f64>,
    DVector<f64>,
) {
    let mut body_a = RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
    body_a.set_rotational_inertia_operator(
        BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(1.5, 1.0, 0.6))
            .expect("positive ellipsoid mass properties"),
    );

    let mut body_b =
        RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.0, 0.0, 0.0]), 0.5, 1.5);
    body_b.transform.rotation = Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.31);
    body_b.set_rotational_inertia_operator(
        BivectorInertia::<3>::uniform_solid_ellipsoid(1.5, SVector::new(0.8, 1.2, 1.7))
            .expect("positive ellipsoid mass properties"),
    );

    let bodies = vec![body_a, body_b];
    let mut rows = Vec::with_capacity(row_count);
    for index in 0..row_count {
        let angle = (index as f64 + 0.5) * std::f64::consts::TAU / row_count as f64;
        let normal = SVector::new(angle.cos(), angle.sin(), 0.0);
        let offset_a = SVector::new(
            0.25 * angle.sin(),
            -0.2 * angle.cos(),
            0.15 * (2.0 * angle).sin(),
        );
        let offset_b = SVector::new(
            -0.2 * angle.sin(),
            0.25 * angle.cos(),
            0.15 * (2.0 * angle).cos(),
        );
        rows.push(FrictionlessContactRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a,
            offset_b,
            normal,
        });
    }

    let matrix =
        assemble_delassus(&bodies, &rows, 1e-12).expect("well-conditioned benchmark contact rows");
    let target_impulses = DVector::from_vec(vec![0.1; row_count]);
    let q = -(&matrix * target_impulses);
    (bodies, rows, matrix, q)
}

fn bench_delassus_assembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("delassus_assembly_asymmetric_3d");
    for row_count in [1_usize, 2, 4, 8] {
        let (bodies, rows, _, _) = fixture(row_count);
        group.bench_with_input(
            BenchmarkId::new("rows", row_count),
            &row_count,
            |bench, _| {
                bench.iter(|| {
                    black_box(
                        assemble_delassus(black_box(&bodies), black_box(&rows), 1e-12)
                            .expect("valid Delassus fixture"),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_reference_lcp(c: &mut Criterion) {
    let mut group = c.benchmark_group("reference_lcp_asymmetric_3d");
    for row_count in [1_usize, 2, 4, 8] {
        let (_, _, matrix, q) = fixture(row_count);
        group.bench_with_input(
            BenchmarkId::new("active_set_rows", row_count),
            &row_count,
            |bench, _| {
                bench.iter(|| {
                    black_box(
                        solve_frictionless_lcp(black_box(&matrix), black_box(&q), 1e-10)
                            .expect("constructed feasible LCP"),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_planar_friction_cone(c: &mut Criterion) {
    let effective_mass = Matrix2::new(2.0, 0.25, 0.25, 1.5);
    let free_velocity = SVector::new(-1.0, -0.8);
    c.bench_function("planar_coulomb_reference_single_contact", |bench| {
        bench.iter(|| {
            black_box(
                solve_planar_friction_cone(
                    black_box(&effective_mass),
                    black_box(&free_velocity),
                    0.5,
                    1e-12,
                )
                .expect("valid planar contact fixture"),
            )
        });
    });
}

criterion_group!(
    benches,
    bench_delassus_assembly,
    bench_reference_lcp,
    bench_planar_friction_cone,
);
criterion_main!(benches);
