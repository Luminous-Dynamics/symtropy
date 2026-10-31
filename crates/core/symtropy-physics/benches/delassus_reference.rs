// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{DVector, Matrix2, SVector};
use symtropy_math::{Bivector, Point, Rotor};
use symtropy_physics::{
    BivectorInertia, BodyHandle, ContactFrame, ContactWrenchFrictionLimits,
    ContactAdjointError, ContactWrenchImpulse, ContactWrenchRow, FrictionlessContactRow, RigidBody,
    assemble_contact_wrench_delassus, assemble_contact_wrench_free_velocity,
    assemble_delassus, differentiate_fixed_active_set_lcp, solve_frictionless_lcp,
    solve_planar_friction_cone,
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

fn contact_wrench_fixture(row_count: usize) -> (Vec<RigidBody<3>>, Vec<ContactWrenchRow<3>>) {
    let mut body_a =
        RigidBody::<3>::dynamic_sphere(BodyHandle(0), Point::origin(), 0.5, 1.0);
    body_a.set_rotational_inertia_operator(
        BivectorInertia::<3>::uniform_solid_ellipsoid(1.0, SVector::new(1.5, 1.0, 0.6))
            .expect("positive ellipsoid mass properties"),
    );
    let mut body_b =
        RigidBody::<3>::dynamic_sphere(BodyHandle(1), Point::new([1.0, 0.0, 0.0]), 0.5, 1.5);
    body_b.transform.rotation =
        Rotor::from_plane_angle(&Bivector::<3>::unit_plane(0, 2), 0.31);
    body_b.set_rotational_inertia_operator(
        BivectorInertia::<3>::uniform_solid_ellipsoid(1.5, SVector::new(0.8, 1.2, 1.7))
            .expect("positive ellipsoid mass properties"),
    );

    let bodies = vec![body_a, body_b];
    let mut rows = Vec::with_capacity(row_count);
    for index in 0..row_count {
        let angle = (index as f64 + 0.5) * std::f64::consts::TAU / row_count as f64;
        let normal = SVector::new(angle.cos(), angle.sin(), 0.2).normalize();
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
        rows.push(ContactWrenchRow {
            body_a: BodyHandle(0),
            body_b: BodyHandle(1),
            offset_a,
            offset_b,
            frame: ContactFrame::<3>::new(normal, 1e-12)
                .expect("finite nonzero benchmark contact normal"),
        });
    }

    (bodies, rows)
}

fn bench_contact_wrench_assembly(c: &mut Criterion) {
    let mut group = c.benchmark_group("contact_wrench_delassus_assembly_asymmetric_3d");
    for row_count in [1_usize, 2, 4] {
        let (bodies, rows) = contact_wrench_fixture(row_count);
        group.bench_with_input(
            BenchmarkId::new("contact_patches", row_count),
            &row_count,
            |bench, _| {
                bench.iter(|| {
                    black_box(
                        assemble_contact_wrench_delassus(
                            black_box(&bodies),
                            black_box(&rows),
                            1e-12,
                        )
                        .expect("valid contact-wrench fixture"),
                    )
                });
            },
        );
    }
    group.finish();
}

fn bench_contact_wrench_free_velocity(c: &mut Criterion) {
    let mut group = c.benchmark_group("contact_wrench_free_velocity_projection_asymmetric_3d");
    for row_count in [1_usize, 2, 4] {
        let (bodies, rows) = contact_wrench_fixture(row_count);
        group.bench_with_input(
            BenchmarkId::new("contact_patches", row_count),
            &row_count,
            |bench, _| {
                bench.iter(|| {
                    black_box(
                        assemble_contact_wrench_free_velocity(
                            black_box(&bodies),
                            black_box(&rows),
                        )
                        .expect("valid contact-wrench free-velocity fixture"),
                    )
                });
            },
        );
    }
    group.finish();
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

fn bench_nd_contact_frame_and_wrench(c: &mut Criterion) {
    let normal = SVector::<f64, 4>::new(0.2, -0.3, 0.4, 0.1);
    let frame = ContactFrame::<4>::new(normal, 1e-12).expect("valid 4D normal");
    let mut angular = Bivector::<4>::zero();
    for i in 0..4 {
        for j in (i + 1)..4 {
            angular.set(i, j, (i as f64 + 1.0) * 0.3 - (j as f64 + 0.5) * 0.2);
        }
    }
    let limits =
        ContactWrenchFrictionLimits::new(0.5, 0.02, 0.03, 0.4, 0.2).expect("valid friction model");
    let wrench = ContactWrenchImpulse::<4> {
        normal: 10.0,
        tangential: frame.tangent_basis()[0] * 5.0,
        rolling: vec![0.06, 0.04, 0.0],
        torsional: vec![0.04, 0.02, 0.0],
    };

    c.bench_function("contact_frame_construct_4d", |bench| {
        bench.iter(|| {
            black_box(
                ContactFrame::<4>::new(black_box(normal), 1e-12)
                    .expect("valid 4D frame construction"),
            )
        });
    });
    c.bench_function("contact_frame_decompose_bivector_4d", |bench| {
        bench.iter(|| {
            black_box(
                frame
                    .decompose_angular(black_box(&angular))
                    .expect("complete contact frame basis"),
            )
        });
    });
    c.bench_function("contact_wrench_validate_independent_channels_4d", |bench| {
        bench.iter(|| {
            black_box(
                wrench
                    .validate(black_box(&frame), black_box(&limits), 1e-12)
                    .expect("admissible independent-channel 4D wrench"),
            )
        });
    });
    let ellipsoidal_wrench = ContactWrenchImpulse::<4> {
        normal: 10.0,
        tangential: frame.tangent_basis()[0] * 1.0,
        rolling: vec![0.01, 0.0, 0.0],
        torsional: vec![0.005, 0.0, 0.0],
    };
    c.bench_function("contact_wrench_validate_coupled_ellipsoid_4d", |bench| {
        bench.iter(|| {
            black_box(
                ellipsoidal_wrench
                    .validate_ellipsoidal_friction_budget(
                        black_box(&frame),
                        black_box(&limits),
                        1e-12,
                    )
                    .expect("admissible coupled-ellipsoid 4D wrench"),
            )
        });
    });
}

fn bench_implicit_contact_adjoint(c: &mut Criterion) {
    let mut group = c.benchmark_group("implicit_contact_adjoint_asymmetric_3d");
    for row_count in [1_usize, 2, 4, 8] {
        let (_, _, matrix, q) = fixture(row_count);
        let solution = solve_frictionless_lcp(&matrix, &q, 1e-10)
            .expect("constructed feasible LCP for adjoint benchmark");
        let matrix_derivative = nalgebra::DMatrix::identity(row_count, row_count) * 0.01;
        let q_derivative = DVector::zeros(row_count);
        let objective_gradient = DVector::from_element(row_count, 1.0);
        let result = differentiate_fixed_active_set_lcp(
            &matrix,
            &q,
            &solution,
            &matrix_derivative,
            &q_derivative,
            &objective_gradient,
            0.0,
            1e-10,
            1e-10,
            1e-12,
        );
        assert!(
            matches!(result, Ok(_) | Err(ContactAdjointError::IllConditionedActiveBlock)),
            "unexpected adjoint fixture rejection: {result:?}"
        );
        group.bench_with_input(
            BenchmarkId::new("active_rows", row_count),
            &row_count,
            |bench, _| {
                bench.iter(|| {
                    black_box(differentiate_fixed_active_set_lcp(
                        black_box(&matrix),
                        black_box(&q),
                        black_box(&solution),
                        black_box(&matrix_derivative),
                        black_box(&q_derivative),
                        black_box(&objective_gradient),
                        0.0,
                        1e-10,
                        1e-10,
                        1e-12,
                    ))
                });
            },
        );
    }
    group.finish();
}

criterion_group!(
    benches,
    bench_delassus_assembly,
    bench_contact_wrench_assembly,
    bench_contact_wrench_free_velocity,
    bench_reference_lcp,
    bench_implicit_contact_adjoint,
    bench_planar_friction_cone,
    bench_nd_contact_frame_and_wrench,
);
criterion_main!(benches);
