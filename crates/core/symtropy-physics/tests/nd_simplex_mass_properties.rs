// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Higher-dimensional exact simplex mass-property qualification.
//!
//! These tests extend the analytic simplex fixtures beyond D=2,3,4 and
//! qualify coherent unit scaling and large-coordinate translation.

use nalgebra::SVector;
use symtropy_physics::inertia::BivectorInertia;

fn factorial(n: usize) -> f64 {
    (2..=n).fold(1.0, |product, value| product * value as f64)
}

fn unit_simplex_vertices<const D: usize>() -> Vec<SVector<f64, D>> {
    let mut vertices = Vec::with_capacity(D + 1);
    vertices.push(SVector::<f64, D>::zeros());
    for axis in 0..D {
        let mut vertex = SVector::<f64, D>::zeros();
        vertex[axis] = 1.0;
        vertices.push(vertex);
    }
    vertices
}

fn assert_unit_simplex_analytic<const D: usize>() {
    let density = factorial(D);
    let properties = BivectorInertia::<D>::from_simplices(
        [(unit_simplex_vertices::<D>(), density)],
        1e-12,
    )
    .unwrap_or_else(|error| panic!("D={D}: {error:?}"));

    assert!((properties.total_mass - 1.0).abs() < 2e-11);
    let expected_center = SVector::<f64, D>::from_element(1.0 / (D + 1) as f64);
    assert!(
        (properties.center_of_mass - expected_center).norm() < 2e-11,
        "D={D}: center={:?}",
        properties.center_of_mass
    );

    // Uniform unit D-simplex at unit mass:
    // S_ii = D / ((D+1)^2 (D+2)); S_ij = -1 / ((D+1)^2 (D+2)).
    let denominator = ((D + 1) * (D + 1) * (D + 2)) as f64;
    let expected_diagonal = D as f64 / denominator;
    let expected_off_diagonal = -1.0 / denominator;
    let second_moment = properties.inertia.second_moment();
    for i in 0..D {
        assert!(
            (second_moment[(i, i)] - expected_diagonal).abs() < 2e-11,
            "D={D}, diagonal ({i},{i})={}",
            second_moment[(i, i)]
        );
        for j in 0..D {
            if i != j {
                assert!(
                    (second_moment[(i, j)] - expected_off_diagonal).abs() < 2e-11,
                    "D={D}, off-diagonal ({i},{j})={}",
                    second_moment[(i, j)]
                );
            }
        }
    }
}

#[test]
fn exact_unit_simplex_formula_holds_in_dimensions_five_six_and_nine() {
    assert_unit_simplex_analytic::<5>();
    assert_unit_simplex_analytic::<6>();
    assert_unit_simplex_analytic::<9>();
}

#[test]
fn four_dimensional_simplex_is_covariant_under_coherent_unit_scaling() {
    let original = BivectorInertia::<4>::from_simplices(
        [(unit_simplex_vertices::<4>(), 24.0)],
        1e-12,
    )
    .unwrap();

    let length_scale = 1e-3;
    let density_scale = length_scale.powi(-4);
    let scaled_vertices = unit_simplex_vertices::<4>()
        .into_iter()
        .map(|vertex| vertex * length_scale)
        .collect();
    let scaled = BivectorInertia::<4>::from_simplices(
        [(scaled_vertices, 24.0 * density_scale)],
        1e-12,
    )
    .unwrap();

    assert!((scaled.total_mass - original.total_mass).abs() < 2e-11);
    assert!(
        (scaled.center_of_mass / length_scale - original.center_of_mass).norm() < 2e-11
    );
    assert!(
        (scaled.inertia.second_moment()
            / (length_scale * length_scale)
            - original.inertia.second_moment())
        .norm()
            < 2e-11
    );
}

#[test]
fn four_dimensional_simplex_retains_centered_moments_at_large_world_offset() {
    let offset = SVector::<f64, 4>::new(1.0e12, -1.0e12, 5.0e11, 2.0e12);
    let translated_vertices = unit_simplex_vertices::<4>()
        .into_iter()
        .map(|vertex| vertex + offset)
        .collect();
    let translated = BivectorInertia::<4>::from_simplices(
        [(translated_vertices, 24.0)],
        1e-12,
    )
    .unwrap();
    let origin = BivectorInertia::<4>::from_simplices(
        [(unit_simplex_vertices::<4>(), 24.0)],
        1e-12,
    )
    .unwrap();

    assert!(
        (translated.inertia.second_moment() - origin.inertia.second_moment()).norm() < 2e-11
    );
    let relative_center = translated.center_of_mass - offset;
    assert!((relative_center - origin.center_of_mass).norm() < 2e-4);
}

#[test]
fn simplex_cell_order_does_not_change_mass_properties() {
    let first = unit_simplex_vertices::<3>();
    let second = first
        .iter()
        .map(|vertex| {
            let mut moved = *vertex;
            moved[0] += 1.0;
            moved
        })
        .collect::<Vec<_>>();
    let cells = vec![(first.clone(), 2.0), (second.clone(), 5.0)];
    let reversed = vec![(second, 5.0), (first, 2.0)];

    let direct = BivectorInertia::<3>::from_simplices(cells, 1e-12).unwrap();
    let permuted = BivectorInertia::<3>::from_simplices(reversed, 1e-12).unwrap();
    assert!((direct.total_mass - permuted.total_mass).abs() < 1e-12);
    assert!((direct.center_of_mass - permuted.center_of_mass).norm() < 1e-12);
    assert!(
        (direct.inertia.second_moment() - permuted.inertia.second_moment()).norm() < 1e-12
    );
}
