// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Independent matrix-level reference fixtures for checked SO(4) logarithms.
//!
//! These tests deliberately construct SO(4) matrices from closed-form 2x2
//! rotation blocks and orthogonal conjugation. They do not use
//! Rotor::from_bivector to generate the reference endpoint, so logarithm and
//! interpolation reconstruction are checked against a separate analytic path.

use nalgebra::SMatrix;
use symtropy_math::{Rotor, so4_interpolate, so4_log};

type Matrix4 = SMatrix<f64, 4, 4>;

fn plane_rotation(first: usize, second: usize, angle: f64) -> Matrix4 {
    let mut rotation = Matrix4::identity();
    let cosine = angle.cos();
    let sine = angle.sin();
    rotation[(first, first)] = cosine;
    rotation[(first, second)] = -sine;
    rotation[(second, first)] = sine;
    rotation[(second, second)] = cosine;
    rotation
}

fn nontrivial_orthogonal_basis_change() -> Matrix4 {
    plane_rotation(0, 1, 0.37)
        * plane_rotation(2, 3, -0.53)
        * plane_rotation(0, 2, 0.29)
        * plane_rotation(1, 3, -0.19)
}

fn maximum_absolute_error(left: &Matrix4, right: &Matrix4) -> f64 {
    left.iter()
        .zip(right.iter())
        .map(|(a, b)| (a - b).abs())
        .fold(0.0_f64, f64::max)
}

fn expected_generator_matrix(first_angle: f64, second_angle: f64) -> Matrix4 {
    // Bivector::to_matrix uses positive upper-triangle components. Rotor's
    // convention exponentiates -B, hence the corresponding plane rotation
    // matrix has entries [[cos,-sin],[sin,cos]].
    let mut generator = Matrix4::zeros();
    generator[(0, 1)] = first_angle;
    generator[(1, 0)] = -first_angle;
    generator[(2, 3)] = second_angle;
    generator[(3, 2)] = -second_angle;
    generator
}

fn expected_double_rotation(first_angle: f64, second_angle: f64) -> Matrix4 {
    plane_rotation(0, 1, first_angle) * plane_rotation(2, 3, second_angle)
}

#[test]
fn so4_log_recovers_conjugated_unequal_principal_planes_from_closed_form_matrix() {
    let first_angle = 0.83_f64;
    let second_angle = -1.24_f64;
    let basis_change = nontrivial_orthogonal_basis_change();
    let basis_change_inverse = basis_change.transpose();

    let expected_generator =
        basis_change * expected_generator_matrix(first_angle, second_angle) * basis_change_inverse;
    let expected_rotation =
        basis_change * expected_double_rotation(first_angle, second_angle) * basis_change_inverse;

    let rotor = Rotor::<4>::from_matrix(expected_rotation.clone());
    let logarithm = so4_log(&rotor).expect("closed-form SO(4) rotation should admit a log");

    assert!(
        maximum_absolute_error(&logarithm.generator.to_matrix(), &expected_generator) < 2.0e-10,
        "generator reconstruction error was {}",
        maximum_absolute_error(&logarithm.generator.to_matrix(), &expected_generator)
    );
    assert!((logarithm.principal_angles_rad[0] - first_angle.abs()).abs() < 2.0e-10);
    assert!((logarithm.principal_angles_rad[1] - second_angle.abs()).abs() < 2.0e-10);

    let reconstructed = Rotor::<4>::from_bivector(&logarithm.generator);
    assert!(
        maximum_absolute_error(reconstructed.to_matrix(), &expected_rotation) < 2.0e-10
    );
}

#[test]
fn so4_interpolation_matches_closed_form_conjugated_double_rotation_at_multiple_fractions() {
    let first_angle = 0.71_f64;
    let second_angle = -1.37_f64;
    let basis_change = nontrivial_orthogonal_basis_change();
    let expected_endpoint =
        basis_change * expected_double_rotation(first_angle, second_angle) * basis_change.transpose();
    let endpoint = Rotor::<4>::from_matrix(expected_endpoint);
    let identity = Rotor::<4>::identity();

    for t in [0.0_f64, 0.125, 0.37, 0.5, 0.875, 1.0] {
        let interpolated =
            so4_interpolate(&identity, &endpoint, t).expect("finite geodesic fraction");
        let expected =
            basis_change * expected_double_rotation(first_angle * t, second_angle * t)
                * basis_change.transpose();

        assert!(
            maximum_absolute_error(interpolated.to_matrix(), &expected) < 3.0e-10,
            "SO(4) interpolation mismatch at t={t}: {}",
            maximum_absolute_error(interpolated.to_matrix(), &expected)
        );
        assert!(interpolated.is_proper_rotation(1.0e-10));
    }
}

#[test]
fn so4_log_and_interpolation_are_equivariant_under_orthogonal_basis_change() {
    let first_angle = 1.11_f64;
    let second_angle = 0.42_f64;
    let basis_change = nontrivial_orthogonal_basis_change();
    let canonical_rotation = expected_double_rotation(first_angle, second_angle);
    let conjugated_rotation =
        basis_change * canonical_rotation * basis_change.transpose();

    let canonical_log =
        so4_log(&Rotor::<4>::from_matrix(canonical_rotation.clone())).expect("canonical SO(4) log");
    let conjugated_log =
        so4_log(&Rotor::<4>::from_matrix(conjugated_rotation.clone())).expect("conjugated SO(4) log");

    let expected_conjugated_generator = basis_change
        * canonical_log.generator.to_matrix()
        * basis_change.transpose();
    assert!(
        maximum_absolute_error(
            &conjugated_log.generator.to_matrix(),
            &expected_conjugated_generator
        ) < 3.0e-10
    );

    let canonical_midpoint = so4_interpolate(
        &Rotor::<4>::identity(),
        &Rotor::<4>::from_matrix(canonical_rotation),
        0.43,
    )
    .expect("canonical midpoint");
    let conjugated_midpoint = so4_interpolate(
        &Rotor::<4>::identity(),
        &Rotor::<4>::from_matrix(conjugated_rotation),
        0.43,
    )
    .expect("conjugated midpoint");
    let expected_conjugated_midpoint = basis_change
        * canonical_midpoint.to_matrix()
        * basis_change.transpose();

    assert!(
        maximum_absolute_error(
            conjugated_midpoint.to_matrix(),
            &expected_conjugated_midpoint
        ) < 3.0e-10
    );
}
