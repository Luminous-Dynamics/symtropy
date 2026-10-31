// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Criterion benchmarks for periodic cell construction, state conversion,
//! physical affine deformation and pure integer-unimodular basis rebase.

use criterion::{BenchmarkId, Criterion, black_box, criterion_group, criterion_main};
use nalgebra::{SMatrix, SVector};
use symtropy_math::{
    FractionalCellState, PeriodicCellKinematics,
};

fn frame2() -> PeriodicCellKinematics<2> {
    PeriodicCellKinematics::new(
        7.5,
        SVector::from([3.0, -2.0]),
        SVector::from([0.2, -0.1]),
        SMatrix::<f64, 2, 2>::from_row_slice(&[2.0, 0.3, 0.0, 1.5]),
        SMatrix::<f64, 2, 2>::from_row_slice(&[0.01, 0.02, 0.0, -0.03]),
        [true, true],
    )
    .expect("fixed benchmark frame is valid")
}

fn frame3() -> PeriodicCellKinematics<3> {
    PeriodicCellKinematics::new(
        7.5,
        SVector::from([3.0, -2.0, 1.0]),
        SVector::from([0.2, -0.1, 0.05]),
        SMatrix::<f64, 3, 3>::from_row_slice(&[
            2.0, 0.4, 0.1,
            0.0, 1.5, 0.3,
            0.0, 0.0, 1.2,
        ]),
        SMatrix::<f64, 3, 3>::from_row_slice(&[
            0.01, 0.02, 0.0,
            0.0, -0.01, 0.01,
            0.0, 0.0, 0.02,
        ]),
        [true, true, true],
    )
    .expect("fixed benchmark frame is valid")
}

fn fractional2() -> FractionalCellState<2> {
    FractionalCellState {
        fractional_position: SVector::from([0.3, -0.2]),
        fractional_velocity: SVector::from([0.7, -0.4]),
    }
}

fn fractional3() -> FractionalCellState<3> {
    FractionalCellState {
        fractional_position: SVector::from([0.2, 0.4, -0.3]),
        fractional_velocity: SVector::from([1.0, -0.5, 0.25]),
    }
}

fn bench_periodic_cell_kinematics(c: &mut Criterion) {
    let mut group = c.benchmark_group("periodic_cell_kinematics");

    group.bench_function("construct/2d", |b| {
        b.iter(|| black_box(frame2()));
    });
    group.bench_function("construct/3d", |b| {
        b.iter(|| black_box(frame3()));
    });

    let frame_2d = frame2();
    let state_2d = fractional2();
    let cartesian_2d = frame_2d.to_cartesian(&state_2d).unwrap();
    group.bench_function("fractional_to_cartesian/2d", |b| {
        b.iter(|| black_box(frame_2d.to_cartesian(black_box(&state_2d)).unwrap()));
    });
    group.bench_function("cartesian_to_fractional/2d", |b| {
        b.iter(|| black_box(frame_2d.to_fractional(black_box(&cartesian_2d)).unwrap()));
    });

    let frame_3d = frame3();
    let state_3d = fractional3();
    let cartesian_3d = frame_3d.to_cartesian(&state_3d).unwrap();
    group.bench_function("fractional_to_cartesian/3d", |b| {
        b.iter(|| black_box(frame_3d.to_cartesian(black_box(&state_3d)).unwrap()));
    });
    group.bench_function("cartesian_to_fractional/3d", |b| {
        b.iter(|| black_box(frame_3d.to_fractional(black_box(&cartesian_3d)).unwrap()));
    });

    let basis_2d = SMatrix::<f64, 2, 2>::from_row_slice(&[2.0, 0.3, 0.0, 1.5]);
    let rate_2d = SMatrix::<f64, 2, 2>::from_row_slice(&[0.01, 0.02, 0.0, -0.03]);
    let shear = SMatrix::<i64, 2, 2>::from_row_slice(&[1, 3, 0, 1]);
    let shear_f64 = shear.map(|value| value as f64);
    let next_rebased = PeriodicCellKinematics::new(
        7.5,
        SVector::from([3.0, -2.0]),
        SVector::from([0.2, -0.1]),
        basis_2d * shear_f64,
        rate_2d * shear_f64,
        [true, true],
    )
    .unwrap();
    group.bench_function("pure_basis_rebase/2d", |b| {
        b.iter(|| {
            black_box(
                frame_2d
                    .basis_rebase_to(
                        &next_rebased,
                        black_box(&cartesian_2d),
                        shear.clone(),
                    )
                    .unwrap(),
            )
        });
    });

    let next_deformed = PeriodicCellKinematics::new(
        7.5,
        SVector::from([3.0, -2.0]),
        SVector::from([0.2, -0.1]),
        SMatrix::<f64, 2, 2>::from_row_slice(&[2.1, 0.5, 0.0, 1.4]),
        SMatrix::<f64, 2, 2>::from_row_slice(&[0.02, 0.01, 0.0, -0.01]),
        [true, true],
    )
    .unwrap();
    group.bench_function("physical_affine_deformation/2d", |b| {
        b.iter(|| {
            black_box(
                frame_2d
                    .affine_deform_to(&next_deformed, black_box(&cartesian_2d))
                    .unwrap(),
            )
        });
    });

    group.bench_with_input(
        BenchmarkId::new("batch_fractional_to_cartesian/2d", 4096),
        &4096_usize,
        |b, &count| {
            let states = vec![state_2d.clone(); count];
            b.iter(|| {
                let mut checksum = 0.0_f64;
                for state in black_box(&states) {
                    let value = frame_2d.to_cartesian(state).unwrap();
                    checksum += value.position.norm() + value.velocity.norm();
                }
                black_box(checksum)
            });
        },
    );

    group.finish();
}

criterion_group!(benches, bench_periodic_cell_kinematics);
criterion_main!(benches);
