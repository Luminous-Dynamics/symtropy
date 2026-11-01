// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Test-only first witness for set-valued Schur reduction on a sparse contact graph.
//!
//! The fixture is a three-contact star: one separator contact coupled to two
//! mutually independent leaves. The leaves are conditionally eliminated on
//! strict sticking branches; the separator retains the full Schur-reduced
//! Signorini-Coulomb problem. Every reconstructed candidate is checked against
//! the original 9x9 contact law and full-island energy.

use crate::coupled_friction::{
    CoupledCoulombRegime, solve_coupled_signorini_coulomb,
};
use nalgebra::{Matrix3, SMatrix, SVector};

type Matrix9 = SMatrix<f64, 9, 9>;
type Vector9 = SVector<f64, 9>;

const FRICTION: f64 = 0.5;
const COUPLING: f64 = 0.25;
const TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug)]
struct StarCandidate {
    impulse: Vector9,
    post_velocity: Vector9,
    separator_regime: CoupledCoulombRegime,
    separator_tau: f64,
    energy_delta: f64,
    effective_operator: Matrix3<f64>,
    effective_free_velocity: SVector<f64, 3>,
}

fn star_delassus_and_free_velocity(first_leaf_tangent: f64) -> (Matrix9, Vector9) {
    let mut w = Matrix9::identity();
    for axis in 0..3 {
        w[(axis, axis + 3)] = COUPLING;
        w[(axis + 3, axis)] = COUPLING;
        w[(axis, axis + 6)] = COUPLING;
        w[(axis + 6, axis)] = COUPLING;
    }

    // Contact order: separator 0, leaf 1, leaf 2.
    // The leaf-to-leaf 3x3 block is exactly zero.
    let q = Vector9::new(
        -1.5,
        1.0,
        0.0,
        -1.25,
        first_leaf_tangent,
        0.0,
        -1.25,
        0.125,
        0.0,
    );
    (w, q)
}

fn solve_star_with_strict_sticking_leaves(
    w: &Matrix9,
    q: &Vector9,
) -> Result<Vec<StarCandidate>, &'static str> {
    if !w.iter().all(|value| value.is_finite()) || !q.iter().all(|value| value.is_finite()) {
        return Err("non-finite star contact input");
    }
    if (w - w.transpose()).norm() > TOLERANCE || w.cholesky().is_none() {
        return Err("star Delassus operator must be symmetric positive definite");
    }

    // Validate the declared star sparsity rather than inferring zeros by
    // thresholding: both leaf diagonal blocks are I and their cross block is 0.
    for row in 0..3 {
        for column in 0..3 {
            if w[(row + 3, column + 6)] != 0.0
                || w[(row + 6, column + 3)] != 0.0
                || w[(row + 3, column + 3)] != f64::from(row == column)
                || w[(row + 6, column + 6)] != f64::from(row == column)
                || w[(row, column + 3)] != if row == column { COUPLING } else { 0.0 }
                || w[(row, column + 6)] != if row == column { COUPLING } else { 0.0 }
            {
                return Err("input does not match the declared three-contact star structure");
            }
        }
    }

    // Each leaf has A_i = I and coupling B_i = rho I to the separator.
    // Eliminate p_i = -(q_i + B_i^T p_0) under the declared sticking regime.
    let effective_operator = Matrix3::identity() * (1.0 - 2.0 * COUPLING * COUPLING);
    let q_separator = SVector::<f64, 3>::new(q[0], q[1], q[2]);
    let q_leaf_1 = SVector::<f64, 3>::new(q[3], q[4], q[5]);
    let q_leaf_2 = SVector::<f64, 3>::new(q[6], q[7], q[8]);
    let effective_free_velocity =
        q_separator - COUPLING * q_leaf_1 - COUPLING * q_leaf_2;

    let reduced = solve_coupled_signorini_coulomb(
        &effective_operator,
        &effective_free_velocity,
        FRICTION,
        TOLERANCE,
    )
    .map_err(|_| "reduced separator problem was rejected")?;

    let mut candidates = Vec::new();
    for separator in &reduced.candidates {
        let p_0 = separator.impulse;
        let p_1 = -q_leaf_1 - COUPLING * p_0;
        let p_2 = -q_leaf_2 - COUPLING * p_0;
        let impulse = Vector9::new(
            p_0[0], p_0[1], p_0[2], p_1[0], p_1[1], p_1[2], p_2[0], p_2[1], p_2[2],
        );
        let post_velocity = q + w * impulse;

        let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
        let velocity_scale = (w.norm() * impulse.norm())
            .max(q.norm())
            .max(f64::MIN_POSITIVE);
        let impulse_tolerance = TOLERANCE * impulse_scale * 500.0;
        let velocity_tolerance = TOLERANCE * velocity_scale * 500.0;

        let leaf_1 = SVector::<f64, 3>::new(impulse[3], impulse[4], impulse[5]);
        let leaf_2 = SVector::<f64, 3>::new(impulse[6], impulse[7], impulse[8]);
        let leaf_1_velocity =
            SVector::<f64, 3>::new(post_velocity[3], post_velocity[4], post_velocity[5]);
        let leaf_2_velocity =
            SVector::<f64, 3>::new(post_velocity[6], post_velocity[7], post_velocity[8]);

        let leaf_1_margin = FRICTION * leaf_1[0] - leaf_1.fixed_rows::<2>(1).norm();
        let leaf_2_margin = FRICTION * leaf_2[0] - leaf_2.fixed_rows::<2>(1).norm();
        if leaf_1[0] <= impulse_tolerance
            || leaf_2[0] <= impulse_tolerance
            || leaf_1_margin <= impulse_tolerance
            || leaf_2_margin <= impulse_tolerance
            || leaf_1_velocity.norm() > velocity_tolerance
            || leaf_2_velocity.norm() > velocity_tolerance
        {
            continue;
        }

        let separator_velocity = SVector::<f64, 3>::new(
            post_velocity[0],
            post_velocity[1],
            post_velocity[2],
        );
        let separator_tau = separator.disk_multiplier;
        let separator_kkt = separator_velocity.fixed_rows::<2>(1)
            + separator_tau * p_0.fixed_rows::<2>(1);
        if separator.regime != CoupledCoulombRegime::Sliding
            || p_0[0] <= impulse_tolerance
            || (FRICTION * p_0[0] - p_0.fixed_rows::<2>(1).norm()).abs()
                > impulse_tolerance
            || separator_velocity[0].abs() > velocity_tolerance
            || separator_tau <= 0.0
            || separator_kkt.norm() > velocity_tolerance
        {
            continue;
        }

        let energy_delta = impulse.dot(q) + 0.5 * impulse.dot(&(w * impulse));
        let energy_tolerance = TOLERANCE * (1.0 + q.norm() * impulse.norm()) * 500.0;
        if energy_delta > energy_tolerance {
            continue;
        }

        candidates.push(StarCandidate {
            impulse,
            post_velocity,
            separator_regime: separator.regime,
            separator_tau,
            energy_delta,
            effective_operator,
            effective_free_velocity,
        });
    }
    Ok(candidates)
}

fn swap_star_leaves(w: &Matrix9, q: &Vector9) -> (Matrix9, Vector9) {
    let permutation = [0, 1, 2, 6, 7, 8, 3, 4, 5];
    let permuted_w = Matrix9::from_fn(|row, column| w[(permutation[row], permutation[column])]);
    let permuted_q = Vector9::from_fn(|row, _| q[permutation[row]]);
    (permuted_w, permuted_q)
}

#[test]
fn two_sticking_leaves_reduce_to_one_sliding_separator_and_reconstruct_full_island() {
    let (w, q) = star_delassus_and_free_velocity(0.125);
    assert!(w.cholesky().is_some());
    assert_eq!(w[(3, 6)], 0.0);
    assert_eq!(w[(6, 3)], 0.0);
    assert_eq!(w[(0, 3)], COUPLING);
    assert_eq!(w[(0, 6)], COUPLING);

    let candidates = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];

    assert_eq!(candidate.separator_regime, CoupledCoulombRegime::Sliding);
    assert!((candidate.impulse[0] - 1.0).abs() < 1.0e-9);
    assert!((candidate.impulse[1] + 0.5).abs() < 1.0e-9);
    assert!((candidate.impulse[3] - 1.0).abs() < 1.0e-9);
    assert!(candidate.impulse[4].abs() < 1.0e-9);
    assert!((candidate.impulse[6] - 1.0).abs() < 1.0e-9);
    assert!(candidate.impulse[7].abs() < 1.0e-9);

    assert!((candidate.effective_operator[(0, 0)] - 0.875).abs() < 1.0e-12);
    assert!((candidate.effective_free_velocity[0] + 0.875).abs() < 1.0e-12);
    assert!((candidate.effective_free_velocity[1] - 0.9375).abs() < 1.0e-12);

    assert!(candidate.post_velocity.fixed_rows::<3>(0).norm() > 0.49);
    assert!(candidate.post_velocity[0].abs() < 1.0e-9);
    assert!(candidate.post_velocity[1] > 0.49);
    assert!(candidate.post_velocity.fixed_rows::<3>(3).norm() < 1.0e-9);
    assert!(candidate.post_velocity.fixed_rows::<3>(6).norm() < 1.0e-9);
    assert!((candidate.separator_tau - 1.0).abs() < 1.0e-8);
    assert!((candidate.energy_delta + 2.375).abs() < 1.0e-9);
    assert!(candidate.energy_delta < 0.0);

    // Re-evaluate every contact law and the energy from the original 9x9 system.
    let independently_reconstructed_velocity = q + w * candidate.impulse;
    assert!((independently_reconstructed_velocity - candidate.post_velocity).norm() < 1.0e-12);
    for offset in [3_usize, 6_usize] {
        let leaf_impulse = candidate.impulse.fixed_rows::<3>(offset);
        let leaf_velocity = candidate.post_velocity.fixed_rows::<3>(offset);
        assert!(leaf_impulse[0] > 0.0);
        assert!(FRICTION * leaf_impulse[0] - leaf_impulse.fixed_rows::<2>(1).norm() > 0.49);
        assert!(leaf_velocity.norm() < 1.0e-9);
    }
}

#[test]
fn leaf_sticking_reduction_is_invariant_to_leaf_permutation() {
    let (w, q) = star_delassus_and_free_velocity(0.125);
    let original = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    let (permuted_w, permuted_q) = swap_star_leaves(&w, &q);
    let permuted = solve_star_with_strict_sticking_leaves(&permuted_w, &permuted_q).unwrap();

    assert_eq!(original.len(), 1);
    assert_eq!(permuted.len(), 1);
    assert!((original[0].impulse.fixed_rows::<3>(0) - permuted[0].impulse.fixed_rows::<3>(0)).norm() < 1.0e-10);
    assert!((original[0].impulse.fixed_rows::<3>(3) - permuted[0].impulse.fixed_rows::<3>(6)).norm() < 1.0e-10);
    assert!((original[0].impulse.fixed_rows::<3>(6) - permuted[0].impulse.fixed_rows::<3>(3)).norm() < 1.0e-10);
}

#[test]
fn leaf_with_noninterior_friction_impulse_is_not_promoted_as_sticking() {
    let (w, q) = star_delassus_and_free_velocity(0.8);
    let candidates = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    assert!(candidates.is_empty());
}
