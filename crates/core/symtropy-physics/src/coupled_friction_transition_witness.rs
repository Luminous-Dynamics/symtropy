// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Exact coupled two-contact Coulomb stick/slip transition witness.
//!
//! This test-only analytic fixture isolates a regime boundary in a genuinely
//! coupled 6x6 Delassus system. It is a known-branch witness, not a solver and
//! not a proof that the complete solution set has been enumerated.

use nalgebra::{SMatrix, SVector};

type Matrix6 = SMatrix<f64, 6, 6>;
type Vector6 = SVector<f64, 6>;

const MU_2: f64 = 1.0;
const RESIDUAL_TOLERANCE: f64 = 1.0e-11;
const STICK_SLIP_THRESHOLD: f64 = 0.5;

/// W = [[I, I/4], [I/4, I]], with q chosen so p* is the all-sticking
/// impulse. Every matrix/vector entry is an exactly representable dyadic.
fn fixture() -> (Matrix6, Vector6, Vector6) {
    let mut w = Matrix6::identity();
    for axis in 0..3 {
        w[(axis, axis + 3)] = 0.25;
        w[(axis + 3, axis)] = 0.25;
    }

    let sticking_impulse = Vector6::new(10.0, 3.0, 4.0, 10.0, 3.0, 4.0);
    let free_velocity = -(w * sticking_impulse);
    (w, free_velocity, sticking_impulse)
}

/// Closed-form mixed branch for 0 < mu_1 <= 1/2. At the upper endpoint
/// this equals the all-sticking impulse and lies exactly on the transition.
fn mixed_sliding_sticking_impulse(mu_1: f64) -> Vector6 {
    Vector6::new(
        10.0,
        6.0 * mu_1,
        8.0 * mu_1,
        10.0,
        15.0 / 4.0 - 3.0 * mu_1 / 2.0,
        5.0 - 2.0 * mu_1,
    )
}

fn disk_margin_squared(mu: f64, impulse: &Vector6, offset: usize) -> f64 {
    mu * mu * impulse[offset] * impulse[offset]
        - impulse[offset + 1] * impulse[offset + 1]
        - impulse[offset + 2] * impulse[offset + 2]
}

fn kinetic_energy_delta(w: &Matrix6, q: &Vector6, p: &Vector6) -> f64 {
    p.dot(q) + 0.5 * p.dot(&(w * p))
}

/// Recheck the candidate against the original full 6x6 contact law. Contact
/// 2 is strictly sticking in every member of this family.
fn assert_full_island_recheck(w: &Matrix6, q: &Vector6, p: &Vector6) -> Vector6 {
    let post_velocity = q + w * p;

    for normal in [0, 3] {
        assert!(p[normal] > 0.0, "contact normal impulse must be positive");
        assert!(
            post_velocity[normal].abs() < RESIDUAL_TOLERANCE,
            "loaded contact must have zero post-normal velocity"
        );
        assert!(
            (p[normal] * post_velocity[normal]).abs() < RESIDUAL_TOLERANCE,
            "normal complementarity residual"
        );
    }

    assert!(
        post_velocity[4].abs() < RESIDUAL_TOLERANCE && post_velocity[5].abs() < RESIDUAL_TOLERANCE,
        "contact 2 must stick in both tangent directions"
    );
    assert!(
        disk_margin_squared(MU_2, p, 3) > 0.0,
        "contact 2 must remain strictly inside its Coulomb disk"
    );
    assert!(
        kinetic_energy_delta(w, q, p) < 0.0,
        "the full island impulse must be dissipative"
    );

    post_velocity
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn full_delassus_fixture_is_spd_and_cross_contact_coupling_is_nonzero() {
        let (w, _, _) = fixture();

        assert!((w - w.transpose()).norm() == 0.0);
        assert!(w.cholesky().is_some());

        let mut cross_norm_squared = 0.0;
        for row in 0..3 {
            for column in 3..6 {
                cross_norm_squared += w[(row, column)] * w[(row, column)];
            }
        }
        assert!(cross_norm_squared > 0.0);
        assert!((cross_norm_squared.sqrt() - (3.0_f64.sqrt() / 4.0)).abs() < 1.0e-15);
    }

    #[test]
    fn mixed_sliding_sticking_branch_replays_on_the_sliding_side() {
        let (w, q, _) = fixture();

        for mu_1 in [0.125, 0.25, 0.375] {
            assert!(mu_1 > 0.0 && mu_1 < STICK_SLIP_THRESHOLD);
            let p = mixed_sliding_sticking_impulse(mu_1);
            let post_velocity = assert_full_island_recheck(&w, &q, &p);

            let tau_1 = 15.0 / (32.0 * mu_1) * (1.0 - 2.0 * mu_1);
            assert!(tau_1 > 0.0, "strict sliding requires tau > 0");
            assert!(
                disk_margin_squared(mu_1, &p, 0).abs() < RESIDUAL_TOLERANCE,
                "sliding impulse lies on contact 1's Coulomb disk boundary"
            );
            assert!(
                (post_velocity[1] + tau_1 * p[1]).abs() < RESIDUAL_TOLERANCE
                    && (post_velocity[2] + tau_1 * p[2]).abs() < RESIDUAL_TOLERANCE,
                "contact 1 sliding KKT stationarity"
            );
        }
    }

    #[test]
    fn exact_half_friction_is_a_transition_not_strict_stick_or_strict_slide() {
        let (w, q, sticking_impulse) = fixture();
        let mu_1 = STICK_SLIP_THRESHOLD;
        let p = mixed_sliding_sticking_impulse(mu_1);
        let post_velocity = assert_full_island_recheck(&w, &q, &p);

        assert_eq!(p, sticking_impulse);
        assert_eq!(post_velocity, Vector6::zeros());
        assert_eq!(disk_margin_squared(mu_1, &p, 0), 0.0);
        assert_eq!(disk_margin_squared(MU_2, &p, 3), 75.0);

        let limiting_tau = 15.0 / (32.0 * mu_1) * (1.0 - 2.0 * mu_1);
        assert_eq!(limiting_tau, 0.0);

        assert_eq!(disk_margin_squared(mu_1, &p, 0), 0.0);
        assert_eq!(limiting_tau, 0.0);
    }

    #[test]
    fn all_sticking_branch_is_strictly_interior_above_the_transition() {
        let (w, q, p) = fixture();
        let mu_1 = 0.75;
        let post_velocity = assert_full_island_recheck(&w, &q, &p);

        assert_eq!(post_velocity, Vector6::zeros());
        assert!(disk_margin_squared(mu_1, &p, 0) > 0.0);
        assert!(disk_margin_squared(MU_2, &p, 3) > 0.0);
    }

    #[test]
    fn negative_controls_reject_wrong_strict_regime_labels() {
        let (_, _, sticking_impulse) = fixture();

        assert!(disk_margin_squared(0.375, &sticking_impulse, 0) < 0.0);

        let continued_tau = 15.0 / (32.0 * 0.625) * (1.0 - 2.0 * 0.625);
        assert!(continued_tau < 0.0);
        assert!(disk_margin_squared(0.625, &sticking_impulse, 0) > 0.0);
    }

    #[test]
    fn deleting_the_cross_block_does_not_reproduce_the_coupled_branch() {
        let (w, q, _) = fixture();
        let p = mixed_sliding_sticking_impulse(0.25);
        let coupled_velocity = q + w * p;
        let uncoupled_velocity = q + Matrix6::identity() * p;

        assert!(coupled_velocity.fixed_rows::<3>(3).norm() < RESIDUAL_TOLERANCE);
        assert!(
            uncoupled_velocity.fixed_rows::<3>(3).norm() > 1.0,
            "block-diagonal approximation must not masquerade as the coupled result"
        );
    }
}
