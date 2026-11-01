// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Frozen two-contact coupled Coulomb multiple-solution witness.
//!
//! This is a test-only falsification fixture for PHYS-MULTICONTACT-COULOMB-001.
//! It does not search for solutions or claim completeness. It verifies three
//! distinct, admissible impulses for one full SPD 6x6 Delassus matrix with
//! nonzero cross-contact blocks. The witness was found by an independent
//! double-precision nonlinear prototype and is frozen here for Rust replay.

use nalgebra::{SMatrix, SVector};

type Matrix6 = SMatrix<f64, 6, 6>;
type Vector6 = SVector<f64, 6>;

const MU_1: f64 = 1.383_289_774_658_335;
const MU_2: f64 = 1.0;
const TOL: f64 = 2.0e-9;

#[derive(Clone, Copy)]
enum ContactOneRegime {
    Sticking,
    Sliding,
}

fn fixture() -> (Matrix6, Vector6) {
    let w = Matrix6::from_row_slice(&[
        0.59497432,
        -0.39054620,
        -0.63542426,
        -0.0015969863578854011,
        -0.05030946883687662,
        0.1029609275985498,
        -0.39054620,
        1.65052340,
        0.80910089,
        0.019271919047677163,
        0.025741864981422207,
        -0.008_317_067_196_577_608,
        -0.63542426,
        0.80910089,
        0.94522735,
        0.002331483384856203,
        0.013469705454618652,
        0.007_804_283_595_085_108,
        -0.0015969863578854011,
        0.019271919047677163,
        0.002331483384856203,
        1.0,
        0.0,
        0.0,
        -0.05030946883687662,
        0.025741864981422207,
        0.013469705454618652,
        0.0,
        1.0,
        0.0,
        0.1029609275985498,
        -0.008_317_067_196_577_608,
        0.007_804_283_595_085_108,
        0.0,
        0.0,
        1.0,
    ]);
    let q = Vector6::new(-0.10770302, 1.49149027, -0.95561905, -1.0, 0.1, -0.1);
    (w, q)
}

fn candidate_impulses() -> [Vector6; 3] {
    [
        Vector6::new(
            8.62138207099778,
            -3.8288167608242003,
            10.084652431088083,
            1.0640446766313634,
            0.2964607388684519,
            -0.8982135090246709,
        ),
        Vector6::new(
            2.635473725062579,
            -1.6299849931593697,
            3.260938855010074,
            1.028018929666454,
            0.030624250979954602,
            -0.2103568057229951,
        ),
        Vector6::new(
            0.2475960198151996,
            -0.25149711759758553,
            0.23249392235397026,
            1.0047001838399274,
            -0.08420119556665649,
            0.07060111719912938,
        ),
    ]
}

fn validate_candidate(
    w: &Matrix6,
    q: &Vector6,
    p: &Vector6,
    regime: ContactOneRegime,
    tau_1: f64,
) -> (f64, f64, f64) {
    let u = q + w * p;
    let lambda_1 = p[0];
    let lambda_2 = p[3];
    let tangent_2_norm = (p[4] * p[4] + p[5] * p[5]).sqrt();
    let tangent_1_norm = (p[1] * p[1] + p[2] * p[2]).sqrt();

    // Both contacts are loaded and satisfy the normal complementarity branch.
    assert!(lambda_1 > 0.0);
    assert!(lambda_2 > 0.0);
    assert!(u[0].abs() < TOL);
    assert!(u[3].abs() < TOL);
    assert!((lambda_1 * u[0]).abs() < TOL);
    assert!((lambda_2 * u[3]).abs() < TOL);

    // Contact 1 is either strictly sticking or strictly sliding.
    match regime {
        ContactOneRegime::Sticking => {
            assert!(u[1].abs() < TOL);
            assert!(u[2].abs() < TOL);
            assert!(MU_1 * lambda_1 - tangent_1_norm > 1.0);
            assert_eq!(tau_1, 0.0);
        }
        ContactOneRegime::Sliding => {
            assert!((tangent_1_norm - MU_1 * lambda_1).abs() < TOL);
            assert!(tau_1 > 0.0);
            assert!((u[1] + tau_1 * p[1]).abs() < TOL);
            assert!((u[2] + tau_1 * p[2]).abs() < TOL);
        }
    }

    // Contact 2 is strictly sticking for all three distinct island solutions.
    assert!(u[4].abs() < TOL);
    assert!(u[5].abs() < TOL);
    assert!(MU_2 * lambda_2 - tangent_2_norm > 0.1);

    // The complete island impulse is dissipative under the frozen law.
    let kinetic_energy_delta = p.dot(q) + 0.5 * p.dot(&(w * p));
    assert!(kinetic_energy_delta < -1.0e-2);

    let cross_contact_energy_term = p[0] * (w[(0, 3)] * p[3] + w[(0, 4)] * p[4] + w[(0, 5)] * p[5])
        + p[1] * (w[(1, 3)] * p[3] + w[(1, 4)] * p[4] + w[(1, 5)] * p[5])
        + p[2] * (w[(2, 3)] * p[3] + w[(2, 4)] * p[4] + w[(2, 5)] * p[5])
        + p[3] * (w[(3, 0)] * p[0] + w[(3, 1)] * p[1] + w[(3, 2)] * p[2])
        + p[4] * (w[(4, 0)] * p[0] + w[(4, 1)] * p[1] + w[(4, 2)] * p[2])
        + p[5] * (w[(5, 0)] * p[0] + w[(5, 1)] * p[1] + w[(5, 2)] * p[2]);
    assert!(cross_contact_energy_term.abs() > 1.0e-8);

    (kinetic_energy_delta, cross_contact_energy_term, u.norm())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupled_spd_two_contact_system_has_three_distinct_admissible_impulses() {
        let (w, q) = fixture();

        // Symmetry and strict positive definiteness are checked on the full
        // coupled matrix, not on two separate 3x3 diagonal blocks.
        assert!((w - w.transpose()).norm() < 1.0e-15);
        assert!(w.cholesky().is_some());

        let mut cross_norm_squared = 0.0;
        for row in 0..3 {
            for column in 3..6 {
                cross_norm_squared += w[(row, column)] * w[(row, column)];
            }
        }
        let cross_block_norm = cross_norm_squared.sqrt();
        assert!(cross_block_norm > 0.12);

        let candidates = candidate_impulses();
        let regimes = [
            (ContactOneRegime::Sticking, 0.0),
            (ContactOneRegime::Sliding, 0.2654416783961581),
            (ContactOneRegime::Sliding, 4.709435913523664),
        ];
        let mut energy_deltas = Vec::new();
        let mut cross_work = Vec::new();

        for (candidate, (regime, tau_1)) in candidates.iter().zip(regimes) {
            let (energy, cross, _) = validate_candidate(&w, &q, candidate, regime, tau_1);
            energy_deltas.push(energy);
            cross_work.push(cross);
        }

        // The solutions are physically distinct, not duplicate parameterizations
        // of the same stick/slip boundary impulse.
        for i in 0..candidates.len() {
            for j in i + 1..candidates.len() {
                assert!((candidates[i] - candidates[j]).norm() > 1.0);
            }
        }

        // The independently computed island energies differ by branch and all
        // remain strictly dissipative; cross-contact contributions are nonzero.
        assert!(energy_deltas[0] < -8.5);
        assert!(energy_deltas[1] < -5.0);
        assert!(energy_deltas[2] < -1.0);
        assert!(cross_work.iter().all(|value| *value < -1.0e-3));
    }
}

// -----------------------------------------------------------------------------
// Regime-restricted two-contact Schur-complement reduction
// -----------------------------------------------------------------------------
//
// If contact 2 is sticking, u_2 = 0 determines p_2 exactly from p_1.
// Eliminating p_2 gives a 3x3 effective Signorini-Coulomb problem at contact
// 1. This is an algebraic reduction for one declared regime assignment, not
// a complete two-contact island solver.

#[cfg(test)]
mod schur_reduction_tests {
    use super::*;
    use crate::coupled_friction::{
        CoupledCoulombRegime, CoupledCoulombStatus, solve_coupled_signorini_coulomb,
    };
    use nalgebra::Matrix3;

    #[derive(Clone, Debug)]
    struct SchurCandidate {
        impulse: Vector6,
        contact_one_regime: CoupledCoulombRegime,
        contact_one_tau: f64,
        kinetic_energy_delta: f64,
        max_velocity_residual: f64,
    }

    #[derive(Clone, Debug)]
    struct SchurOutcome {
        reduced_status: CoupledCoulombStatus,
        candidates: Vec<SchurCandidate>,
    }

    /// Eliminate contact 2 under the strict sticking hypothesis.
    ///
    /// With W = [[A,B],[B^T,C]], p_2 = -C^-1(q_2+B^T p_1),
    /// W_eff = A-B C^-1 B^T, q_eff = q_1-B C^-1 q_2.
    fn solve_with_contact_two_sticking(
        w: &Matrix6,
        q: &Vector6,
        mu_1: f64,
        mu_2: f64,
        tolerance: f64,
    ) -> Result<SchurOutcome, &'static str> {
        if !w.iter().all(|value| value.is_finite())
            || !q.iter().all(|value| value.is_finite())
            || !mu_1.is_finite()
            || !mu_2.is_finite()
            || mu_1 < 0.0
            || mu_2 < 0.0
            || !tolerance.is_finite()
            || tolerance <= 0.0
        {
            return Err("invalid or non-finite two-contact input");
        }

        let scale = w
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if scale == 0.0 {
            return Err("zero two-contact Delassus matrix");
        }
        let asymmetry = Matrix6::from_fn(|i, j| w[(i, j)] - w[(j, i)]);
        if asymmetry.norm() > tolerance * scale * 10.0 {
            return Err("two-contact Delassus matrix is not symmetric");
        }
        let scaled_symmetric_w = Matrix6::from_fn(|i, j| 0.5 * (w[(i, j)] + w[(j, i)]) / scale);
        let full_minimum_eigenvalue = nalgebra::linalg::SymmetricEigen::new(scaled_symmetric_w)
            .eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        if full_minimum_eigenvalue <= tolerance {
            return Err("two-contact Delassus matrix is not sufficiently positive definite");
        }
        if w.cholesky().is_none() {
            return Err("two-contact Delassus matrix is not positive definite");
        }

        let a = Matrix3::from_fn(|i, j| w[(i, j)]);
        let b = Matrix3::from_fn(|i, j| w[(i, j + 3)]);
        let c = Matrix3::from_fn(|i, j| w[(i + 3, j + 3)]);
        let q_1 = SVector::<f64, 3>::new(q[0], q[1], q[2]);
        let q_2 = SVector::<f64, 3>::new(q[3], q[4], q[5]);

        let c_scale = c
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if c_scale == 0.0 {
            return Err("sticking contact principal block is zero");
        }
        let scaled_c = Matrix3::from_fn(|i, j| 0.5 * (c[(i, j)] + c[(j, i)]) / c_scale);
        let c_minimum_eigenvalue = nalgebra::linalg::SymmetricEigen::new(scaled_c)
            .eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        if c_minimum_eigenvalue <= tolerance {
            return Err("sticking contact principal block is ill-conditioned");
        }
        let c_factor = c
            .cholesky()
            .ok_or("sticking contact principal block is not SPD")?;
        let c_inverse_b_transpose = c_factor.solve(&b.transpose());
        let c_inverse_q_2 = c_factor.solve(&q_2);
        let effective_mass = a - b * c_inverse_b_transpose;
        let effective_velocity = q_1 - b * c_inverse_q_2;

        let reduced =
            solve_coupled_signorini_coulomb(&effective_mass, &effective_velocity, mu_1, tolerance)
                .map_err(|_| "reduced single-contact solver refused the Schur system")?;

        // Never forward a partially enumerated set from an inconclusive
        // reduced solve as if it were a candidate set for this regime.
        if reduced.status == CoupledCoulombStatus::Inconclusive {
            return Ok(SchurOutcome {
                reduced_status: reduced.status,
                candidates: Vec::new(),
            });
        }

        let mut candidates = Vec::new();
        for candidate in &reduced.candidates {
            let p_1 = candidate.impulse;
            let p_2 = -c_factor.solve(&(q_2 + b.transpose() * p_1));
            let impulse = Vector6::new(p_1[0], p_1[1], p_1[2], p_2[0], p_2[1], p_2[2]);
            let post_velocity = q + w * impulse;
            let u_1 = SVector::<f64, 3>::new(post_velocity[0], post_velocity[1], post_velocity[2]);
            let u_2 = SVector::<f64, 3>::new(post_velocity[3], post_velocity[4], post_velocity[5]);
            let p_1_tangent_norm = p_1.fixed_rows::<2>(1).norm();
            let p_2_tangent_norm = p_2.fixed_rows::<2>(1).norm();
            let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
            let velocity_scale = (scale * impulse.norm())
                .max(q.norm())
                .max(f64::MIN_POSITIVE);
            let impulse_tolerance = tolerance * impulse_scale * 250.0;
            let velocity_tolerance = tolerance * velocity_scale * 250.0;

            // The eliminated contact must be strictly loaded, exactly sticking
            // up to the declared normalized residual budget, and inside its disk.
            if p_2[0] <= impulse_tolerance
                || u_2.norm() > velocity_tolerance
                || mu_2 * p_2[0] - p_2_tangent_norm <= impulse_tolerance
            {
                continue;
            }

            // Recheck contact 1's declared regime against the reconstructed full
            // 6x6 post-impulse velocity, not merely against the reduced system.
            if p_1[0] < -impulse_tolerance {
                continue;
            }
            let normal_valid = if p_1[0] > impulse_tolerance {
                u_1[0].abs() <= velocity_tolerance
            } else {
                u_1[0] >= -velocity_tolerance
            };
            if !normal_valid {
                continue;
            }

            let tangent_velocity = u_1.fixed_rows::<2>(1);
            let contact_one_tangent_residual = match candidate.regime {
                CoupledCoulombRegime::Separating => {
                    if p_1.norm() > impulse_tolerance {
                        continue;
                    }
                    0.0
                }
                CoupledCoulombRegime::Frictionless => {
                    if mu_1 != 0.0 || p_1_tangent_norm > impulse_tolerance {
                        continue;
                    }
                    0.0
                }
                CoupledCoulombRegime::Sticking => {
                    if p_1[0] <= impulse_tolerance
                        || mu_1 * p_1[0] - p_1_tangent_norm <= impulse_tolerance
                    {
                        continue;
                    }
                    tangent_velocity.norm()
                }
                CoupledCoulombRegime::Sliding => {
                    if (p_1_tangent_norm - mu_1 * p_1[0]).abs() > impulse_tolerance
                        || candidate.disk_multiplier <= 0.0
                    {
                        continue;
                    }
                    (tangent_velocity + candidate.disk_multiplier * p_1.fixed_rows::<2>(1)).norm()
                }
            };
            if contact_one_tangent_residual > velocity_tolerance {
                continue;
            }

            let kinetic_energy_delta = impulse.dot(q) + 0.5 * impulse.dot(&(w * impulse));
            if kinetic_energy_delta > tolerance * (1.0 + q.norm() * impulse.norm()) * 250.0 {
                continue;
            }
            let normal_residual = if p_1[0] > impulse_tolerance {
                u_1[0].abs()
            } else {
                (-u_1[0]).max(0.0)
            };
            candidates.push(SchurCandidate {
                impulse,
                contact_one_regime: candidate.regime,
                contact_one_tau: candidate.disk_multiplier,
                kinetic_energy_delta,
                max_velocity_residual: u_2
                    .norm()
                    .max(normal_residual)
                    .max(contact_one_tangent_residual),
            });
        }

        // Deduplicate only at the requested numerical comparison scale. This
        // is candidate hygiene, not a proof of root completeness.
        candidates.sort_by(|left, right| left.impulse[0].total_cmp(&right.impulse[0]));
        candidates.dedup_by(|left, right| {
            (left.impulse - right.impulse).norm()
                <= tolerance * 1000.0 * (1.0 + left.impulse.norm().max(right.impulse.norm()))
        });

        Ok(SchurOutcome {
            reduced_status: reduced.status,
            candidates,
        })
    }

    #[test]
    fn schur_reduction_replays_all_three_frozen_contact_two_sticking_branches() {
        let (w, q) = fixture();
        let outcome = solve_with_contact_two_sticking(&w, &q, MU_1, MU_2, 1.0e-12).unwrap();

        assert_eq!(
            outcome.reduced_status,
            CoupledCoulombStatus::MultipleSolutions
        );
        assert_eq!(outcome.candidates.len(), 3);

        let references = candidate_impulses();
        for expected in references {
            let match_candidate = outcome
                .candidates
                .iter()
                .find(|candidate| (candidate.impulse - expected).norm() < 2.0e-6);
            assert!(
                match_candidate.is_some(),
                "Schur reduction missed frozen full-island branch {expected:?}; found={:?}",
                outcome
                    .candidates
                    .iter()
                    .map(|candidate| &candidate.impulse)
                    .collect::<Vec<_>>()
            );
        }

        let mut sticking = 0;
        let mut sliding = 0;
        for candidate in &outcome.candidates {
            assert!(candidate.kinetic_energy_delta < -1.0e-2);
            assert!(candidate.max_velocity_residual < 1.0e-8);
            match candidate.contact_one_regime {
                CoupledCoulombRegime::Sticking => sticking += 1,
                CoupledCoulombRegime::Sliding => {
                    sliding += 1;
                    assert!(candidate.contact_one_tau > 0.0);
                }
                _ => panic!("unexpected contact-one regime in frozen witness"),
            }
        }
        assert_eq!(sticking, 1);
        assert_eq!(sliding, 2);
    }

    #[test]
    fn schur_reduction_is_symmetric_under_contact_block_permutation() {
        let (w, q) = fixture();
        let forward = solve_with_contact_two_sticking(&w, &q, MU_1, MU_2, 1.0e-12).unwrap();

        // The witness has contact 2 sticking in all branches. Permuting the
        // contact blocks turns contact 1 into the eliminated sticking block
        // only for the all-sticking solution; compare that branch explicitly.
        let sticking = forward
            .candidates
            .iter()
            .find(|candidate| candidate.contact_one_regime == CoupledCoulombRegime::Sticking)
            .expect("all-sticking branch");
        let permuted_w = Matrix6::from_fn(|i, j| {
            w[(
                if i < 3 { i + 3 } else { i - 3 },
                if j < 3 { j + 3 } else { j - 3 },
            )]
        });
        let permuted_q = Vector6::from_fn(|i, _| q[if i < 3 { i + 3 } else { i - 3 }]);
        let reverse =
            solve_with_contact_two_sticking(&permuted_w, &permuted_q, MU_2, MU_1, 1.0e-12).unwrap();
        let expected = Vector6::new(
            sticking.impulse[3],
            sticking.impulse[4],
            sticking.impulse[5],
            sticking.impulse[0],
            sticking.impulse[1],
            sticking.impulse[2],
        );
        assert!(reverse.candidates.iter().any(|candidate| {
            candidate.contact_one_regime == CoupledCoulombRegime::Sticking
                && (candidate.impulse - expected).norm() < 2.0e-6
        }));
    }

    #[test]
    fn schur_reduction_refuses_a_near_singular_sticking_block() {
        let mut w = Matrix6::identity();
        w[(5, 5)] = 1.0e-14;
        let q = Vector6::new(-1.0, 0.1, -0.1, -1.0, 0.1, -0.1);
        let result = solve_with_contact_two_sticking(&w, &q, 0.5, 0.5, 1.0e-12);
        assert_eq!(
            result.unwrap_err(),
            "two-contact Delassus matrix is not sufficiently positive definite"
        );
    }

    #[test]
    fn schur_reduction_refuses_candidates_outside_the_eliminated_contact_disk() {
        let (w, q) = fixture();
        let outcome = solve_with_contact_two_sticking(&w, &q, MU_1, 0.01, 1.0e-12).unwrap();
        assert_eq!(
            outcome.reduced_status,
            CoupledCoulombStatus::MultipleSolutions
        );
        assert!(outcome.candidates.is_empty());
    }

    /// Generate candidates when contact 2 is strictly separating.
    ///
    /// Since p_2 = 0, solve contact 1 on its principal 3x3 block, reconstruct
    /// the complete 6D post-contact velocity, and then prove u_n,2 > 0.
    /// Contact 2's tangential post-velocity is unrestricted on this branch.
    fn solve_with_contact_two_separating(
        w: &Matrix6,
        q: &Vector6,
        mu_1: f64,
        mu_2: f64,
        tolerance: f64,
    ) -> Result<SchurOutcome, &'static str> {
        if !w.iter().all(|value| value.is_finite())
            || !q.iter().all(|value| value.is_finite())
            || !mu_1.is_finite()
            || !mu_2.is_finite()
            || mu_1 < 0.0
            || mu_2 < 0.0
            || !tolerance.is_finite()
            || tolerance <= 0.0
        {
            return Err("invalid or non-finite two-contact input");
        }

        let scale = w
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        if scale == 0.0 {
            return Err("zero two-contact Delassus matrix");
        }
        let asymmetry = Matrix6::from_fn(|i, j| w[(i, j)] - w[(j, i)]);
        if asymmetry.norm() > tolerance * scale * 10.0 {
            return Err("two-contact Delassus matrix is not symmetric");
        }
        let symmetric_w = (w + w.transpose()) * 0.5;
        let scaled_w = symmetric_w / scale;
        let minimum_eigenvalue = nalgebra::linalg::SymmetricEigen::new(scaled_w)
            .eigenvalues
            .iter()
            .fold(f64::INFINITY, |minimum, value| minimum.min(*value));
        if minimum_eigenvalue <= tolerance {
            return Err("two-contact Delassus matrix is not sufficiently positive definite");
        }
        if w.cholesky().is_none() {
            return Err("two-contact Delassus matrix is not positive definite");
        }

        let a = Matrix3::from_fn(|i, j| w[(i, j)]);
        let q_1 = SVector::<f64, 3>::new(q[0], q[1], q[2]);
        let reduced = solve_coupled_signorini_coulomb(&a, &q_1, mu_1, tolerance)
            .map_err(|_| "active-contact single-contact solver refused the principal block")?;

        if reduced.status == CoupledCoulombStatus::Inconclusive {
            return Ok(SchurOutcome {
                reduced_status: CoupledCoulombStatus::Inconclusive,
                candidates: Vec::new(),
            });
        }

        let mut candidates = Vec::new();
        for candidate in &reduced.candidates {
            let p_1 = candidate.impulse;
            let impulse = Vector6::new(p_1[0], p_1[1], p_1[2], 0.0, 0.0, 0.0);
            let post_velocity = q + w * impulse;
            let u_1 = SVector::<f64, 3>::new(post_velocity[0], post_velocity[1], post_velocity[2]);
            let u_2 = SVector::<f64, 3>::new(post_velocity[3], post_velocity[4], post_velocity[5]);

            let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
            let velocity_scale = (scale * impulse.norm())
                .max(q.norm())
                .max(f64::MIN_POSITIVE);
            let impulse_tolerance = tolerance * impulse_scale * 250.0;
            let velocity_tolerance = tolerance * velocity_scale * 250.0;

            // Contact 2 is strictly separating: its complete impulse is zero
            // and the reconstructed normal velocity is strictly positive.
            if u_2[0] <= velocity_tolerance {
                continue;
            }

            if p_1[0] < -impulse_tolerance {
                continue;
            }
            let p_1_tangent_norm = p_1.fixed_rows::<2>(1).norm();
            let u_1_tangent = u_1.fixed_rows::<2>(1);
            let (normal_residual, tangent_residual) = match candidate.regime {
                CoupledCoulombRegime::Separating => {
                    if p_1.norm() > impulse_tolerance || u_1[0] <= velocity_tolerance {
                        continue;
                    }
                    (0.0, 0.0)
                }
                CoupledCoulombRegime::Frictionless => {
                    if mu_1 != 0.0
                        || p_1[0] <= impulse_tolerance
                        || p_1_tangent_norm > impulse_tolerance
                        || u_1[0].abs() > velocity_tolerance
                    {
                        continue;
                    }
                    (u_1[0].abs(), 0.0)
                }
                CoupledCoulombRegime::Sticking => {
                    if p_1[0] <= impulse_tolerance
                        || mu_1 * p_1[0] - p_1_tangent_norm <= impulse_tolerance
                        || u_1[0].abs() > velocity_tolerance
                        || u_1_tangent.norm() > velocity_tolerance
                    {
                        continue;
                    }
                    (u_1[0].abs(), u_1_tangent.norm())
                }
                CoupledCoulombRegime::Sliding => {
                    let disk_residual = (p_1_tangent_norm - mu_1 * p_1[0]).abs();
                    let stationarity =
                        (u_1_tangent + candidate.disk_multiplier * p_1.fixed_rows::<2>(1)).norm();
                    if p_1[0] <= impulse_tolerance
                        || disk_residual > impulse_tolerance
                        || candidate.disk_multiplier <= 0.0
                        || stationarity > velocity_tolerance
                        || u_1[0].abs() > velocity_tolerance
                    {
                        continue;
                    }
                    (u_1[0].abs(), stationarity)
                }
            };

            let kinetic_energy_delta = impulse.dot(q) + 0.5 * impulse.dot(&(w * impulse));
            let energy_tolerance = tolerance * (1.0 + q.norm() * impulse.norm()) * 250.0;
            if kinetic_energy_delta > energy_tolerance {
                continue;
            }

            candidates.push(SchurCandidate {
                impulse,
                contact_one_regime: candidate.regime,
                contact_one_tau: candidate.disk_multiplier,
                kinetic_energy_delta,
                max_velocity_residual: normal_residual
                    .max(tangent_residual)
                    .max((-u_2[0]).max(0.0)),
            });
        }

        let reduced_status = if candidates.is_empty() {
            // Candidate rejection cannot prove that the complete coupled
            // problem has no solution; preserve the fail-closed claim ceiling.
            CoupledCoulombStatus::Inconclusive
        } else {
            reduced.status
        };
        Ok(SchurOutcome {
            reduced_status,
            candidates,
        })
    }

    fn contact_two_separating_fixture(
        q_1: SVector<f64, 3>,
        contact_two_normal_free_velocity: f64,
    ) -> (Matrix6, Vector6) {
        let mut w = Matrix6::identity();
        for axis in 0..3 {
            w[(axis, axis + 3)] = 0.1;
            w[(axis + 3, axis)] = 0.1;
        }
        let q = Vector6::new(
            q_1[0],
            q_1[1],
            q_1[2],
            contact_two_normal_free_velocity,
            0.2,
            -0.3,
        );
        (w, q)
    }

    #[test]
    fn separating_contact_two_reduction_recovers_all_contact_one_regimes() {
        let cases = [
            (
                SVector::<f64, 3>::new(0.5, 0.2, -0.1),
                CoupledCoulombRegime::Separating,
            ),
            (
                SVector::<f64, 3>::new(-1.0, 0.1, 0.1),
                CoupledCoulombRegime::Sticking,
            ),
            (
                SVector::<f64, 3>::new(-1.0, 1.0, 0.0),
                CoupledCoulombRegime::Sliding,
            ),
        ];

        for (q_1, expected_regime) in cases {
            let (w, q) = contact_two_separating_fixture(q_1, 0.25);
            let outcome = solve_with_contact_two_separating(&w, &q, 0.5, 0.8, 1.0e-12).unwrap();

            assert_eq!(
                outcome.reduced_status,
                CoupledCoulombStatus::UniqueWithinTolerance
            );
            assert_eq!(outcome.candidates.len(), 1);
            let candidate = &outcome.candidates[0];
            assert_eq!(candidate.contact_one_regime, expected_regime);
            assert!(candidate.impulse.fixed_rows::<3>(3).norm() == 0.0);

            let full_velocity = q + w * candidate.impulse;
            assert!(full_velocity[3] > 0.0);
            assert!(candidate.max_velocity_residual < 1.0e-8);
            assert!(candidate.kinetic_energy_delta <= 1.0e-10);
        }
    }

    #[test]
    fn separating_contact_two_reduction_rejects_inward_full_island_velocity() {
        let (w, q) = contact_two_separating_fixture(SVector::<f64, 3>::new(-1.0, 0.1, 0.1), -0.2);
        let outcome = solve_with_contact_two_separating(&w, &q, 0.5, 0.8, 1.0e-12).unwrap();

        assert_eq!(outcome.reduced_status, CoupledCoulombStatus::Inconclusive);
        assert!(outcome.candidates.is_empty());
    }

    #[test]
    fn separating_contact_two_reduction_retains_cross_contact_coupling() {
        let (w, q) = contact_two_separating_fixture(SVector::<f64, 3>::new(-1.0, 0.1, 0.1), 0.25);
        let outcome = solve_with_contact_two_separating(&w, &q, 0.5, 0.8, 1.0e-12).unwrap();
        let candidate = &outcome.candidates[0];

        let coupled_velocity = q + w * candidate.impulse;
        let diagonal_only = Matrix6::identity();
        let uncoupled_velocity = q + diagonal_only * candidate.impulse;

        assert!(coupled_velocity[3] > 0.0);
        assert!((coupled_velocity[3] - uncoupled_velocity[3]).abs() > 1.0e-3);
    }

    #[derive(Clone, Debug)]
    struct MirroredSchurCandidate {
        impulse: Vector6,
        contact_two_regime: CoupledCoulombRegime,
        contact_two_tau: f64,
        kinetic_energy_delta: f64,
        max_velocity_residual: f64,
    }

    #[derive(Clone, Debug)]
    struct MirroredSchurOutcome {
        reduced_status: CoupledCoulombStatus,
        candidates: Vec<MirroredSchurCandidate>,
    }

    fn swap_contact_blocks(w: &Matrix6, q: &Vector6) -> (Matrix6, Vector6) {
        let swap = |index: usize| if index < 3 { index + 3 } else { index - 3 };
        let permuted_w = Matrix6::from_fn(|i, j| w[(swap(i), swap(j))]);
        let permuted_q = Vector6::from_fn(|i, _| q[swap(i)]);
        (permuted_w, permuted_q)
    }

    fn swap_impulse_blocks(impulse: &Vector6) -> Vector6 {
        Vector6::new(
            impulse[3], impulse[4], impulse[5], impulse[0], impulse[1], impulse[2],
        )
    }

    /// Mirror the contact-2-separating reduction by permuting contact blocks.
    ///
    /// The returned contact_two_regime names the active (non-eliminated)
    /// original contact. Contact 1 has p_1 = 0 and must separate strictly
    /// after reconstruction against the original full 6x6 island.
    fn solve_with_contact_one_separating(
        w: &Matrix6,
        q: &Vector6,
        mu_1: f64,
        mu_2: f64,
        tolerance: f64,
    ) -> Result<MirroredSchurOutcome, &'static str> {
        let (permuted_w, permuted_q) = swap_contact_blocks(w, q);
        let inner =
            solve_with_contact_two_separating(&permuted_w, &permuted_q, mu_2, mu_1, tolerance)?;

        let scale = w
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let mut candidates = Vec::new();
        for candidate in inner.candidates {
            let impulse = swap_impulse_blocks(&candidate.impulse);
            let post_velocity = q + w * impulse;
            let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
            let velocity_scale = (scale * impulse.norm())
                .max(q.norm())
                .max(f64::MIN_POSITIVE);
            let impulse_tolerance = tolerance * impulse_scale * 250.0;
            let velocity_tolerance = tolerance * velocity_scale * 250.0;

            // Contact 1 is strictly separating in the original coordinate order.
            if impulse.fixed_rows::<3>(0).norm() > impulse_tolerance
                || post_velocity[0] <= velocity_tolerance
            {
                continue;
            }

            let kinetic_energy_delta = impulse.dot(q) + 0.5 * impulse.dot(&(w * impulse));
            let energy_tolerance = tolerance * (1.0 + q.norm() * impulse.norm()) * 250.0;
            if kinetic_energy_delta > energy_tolerance {
                continue;
            }

            candidates.push(MirroredSchurCandidate {
                impulse,
                contact_two_regime: candidate.contact_one_regime,
                contact_two_tau: candidate.contact_one_tau,
                kinetic_energy_delta,
                max_velocity_residual: candidate.max_velocity_residual,
            });
        }

        let reduced_status = if candidates.is_empty() {
            CoupledCoulombStatus::Inconclusive
        } else {
            inner.reduced_status
        };
        Ok(MirroredSchurOutcome {
            reduced_status,
            candidates,
        })
    }

    /// Mirror contact-2 sticking elimination to the case where contact 1 sticks.
    ///
    /// After permuting contact blocks, the existing Schur reducer eliminates
    /// the original contact-1 block. The returned candidates are unpermuted and
    /// independently rechecked for strict contact-1 sticking and full-island energy.
    fn solve_with_contact_one_sticking(
        w: &Matrix6,
        q: &Vector6,
        mu_1: f64,
        mu_2: f64,
        tolerance: f64,
    ) -> Result<MirroredSchurOutcome, &'static str> {
        let (permuted_w, permuted_q) = swap_contact_blocks(w, q);
        let inner =
            solve_with_contact_two_sticking(&permuted_w, &permuted_q, mu_2, mu_1, tolerance)?;

        let scale = w
            .iter()
            .fold(0.0_f64, |maximum, value| maximum.max(value.abs()));
        let mut candidates = Vec::new();
        for candidate in inner.candidates {
            let impulse = swap_impulse_blocks(&candidate.impulse);
            let post_velocity = q + w * impulse;
            let p_1 = SVector::<f64, 3>::new(impulse[0], impulse[1], impulse[2]);
            let u_1 = SVector::<f64, 3>::new(post_velocity[0], post_velocity[1], post_velocity[2]);
            let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
            let velocity_scale = (scale * impulse.norm())
                .max(q.norm())
                .max(f64::MIN_POSITIVE);
            let impulse_tolerance = tolerance * impulse_scale * 250.0;
            let velocity_tolerance = tolerance * velocity_scale * 250.0;
            let tangent_impulse_norm = p_1.fixed_rows::<2>(1).norm();

            // Contact 1 must be loaded, stationary, and strictly inside its disk.
            if p_1[0] <= impulse_tolerance
                || u_1.norm() > velocity_tolerance
                || mu_1 * p_1[0] - tangent_impulse_norm <= impulse_tolerance
            {
                continue;
            }

            let kinetic_energy_delta = impulse.dot(q) + 0.5 * impulse.dot(&(w * impulse));
            let energy_tolerance = tolerance * (1.0 + q.norm() * impulse.norm()) * 250.0;
            if kinetic_energy_delta > energy_tolerance {
                continue;
            }

            candidates.push(MirroredSchurCandidate {
                impulse,
                contact_two_regime: candidate.contact_one_regime,
                contact_two_tau: candidate.contact_one_tau,
                kinetic_energy_delta,
                max_velocity_residual: candidate.max_velocity_residual.max(u_1.norm()),
            });
        }

        let reduced_status = if candidates.is_empty() {
            CoupledCoulombStatus::Inconclusive
        } else {
            inner.reduced_status
        };
        Ok(MirroredSchurOutcome {
            reduced_status,
            candidates,
        })
    }

    fn contact_one_separating_contact_two_sliding_fixture(
        contact_one_normal_free_velocity: f64,
    ) -> (Matrix6, Vector6) {
        let mut w = Matrix6::identity();
        for axis in 0..3 {
            w[(axis, axis + 3)] = 0.1;
            w[(axis + 3, axis)] = 0.1;
        }
        let q = Vector6::new(contact_one_normal_free_velocity, 0.2, -0.1, -1.0, 1.0, 0.0);
        (w, q)
    }

    fn contact_one_sticking_contact_two_sliding_fixture() -> (Matrix6, Vector6) {
        let mut w = Matrix6::identity();
        for axis in 0..3 {
            w[(axis, axis + 3)] = 0.1;
            w[(axis + 3, axis)] = 0.1;
        }
        let q = Vector6::new(-10.1, 0.05, 0.0, -2.0, 1.0, 0.0);
        (w, q)
    }

    #[test]
    fn contact_one_separating_reduction_recovers_contact_two_sliding() {
        let (w, q) = contact_one_separating_contact_two_sliding_fixture(1.0);
        let outcome = solve_with_contact_one_separating(&w, &q, 0.3, 0.5, 1.0e-12).unwrap();

        assert_eq!(
            outcome.reduced_status,
            CoupledCoulombStatus::UniqueWithinTolerance
        );
        assert_eq!(outcome.candidates.len(), 1);
        let candidate = &outcome.candidates[0];
        assert_eq!(candidate.contact_two_regime, CoupledCoulombRegime::Sliding);
        assert!(candidate.contact_two_tau > 0.0);
        assert!(candidate.max_velocity_residual < 1.0e-8);
        assert!(candidate.impulse.fixed_rows::<3>(0).norm() == 0.0);

        let post_velocity = q + w * candidate.impulse;
        assert!(post_velocity[0] > 0.0);
        assert!(post_velocity[3].abs() < 1.0e-8);
        assert!(
            (post_velocity[4] + candidate.contact_two_tau * candidate.impulse[4]).abs() < 1.0e-8
        );
        assert!(
            (post_velocity[5] + candidate.contact_two_tau * candidate.impulse[5]).abs() < 1.0e-8
        );
        assert!(candidate.kinetic_energy_delta < 0.0);
    }

    #[test]
    fn contact_one_separating_reduction_rejects_inward_full_island_velocity() {
        let (w, q) = contact_one_separating_contact_two_sliding_fixture(-0.2);
        let outcome = solve_with_contact_one_separating(&w, &q, 0.3, 0.5, 1.0e-12).unwrap();

        assert_eq!(outcome.reduced_status, CoupledCoulombStatus::Inconclusive);
        assert!(outcome.candidates.is_empty());
    }

    #[test]
    fn contact_one_sticking_reduction_recovers_contact_two_sliding() {
        let (w, q) = contact_one_sticking_contact_two_sliding_fixture();
        let outcome = solve_with_contact_one_sticking(&w, &q, 0.2, 0.5, 1.0e-12).unwrap();

        assert_eq!(
            outcome.reduced_status,
            CoupledCoulombStatus::UniqueWithinTolerance
        );
        assert_eq!(outcome.candidates.len(), 1);
        let candidate = &outcome.candidates[0];
        assert_eq!(candidate.contact_two_regime, CoupledCoulombRegime::Sliding);
        assert!(candidate.contact_two_tau > 0.0);
        assert!(candidate.max_velocity_residual < 1.0e-8);

        let p = candidate.impulse;
        assert!((p[0] - 10.0).abs() < 1.0e-8);
        assert!(p.fixed_rows::<2>(1).norm() < 1.0e-8);
        assert!((p[3] - 1.0).abs() < 1.0e-8);
        assert!((p[4] + 0.5).abs() < 1.0e-8);
        let post_velocity = q + w * p;
        assert!(post_velocity.fixed_rows::<3>(0).norm() < 1.0e-8);
        assert!(post_velocity[3].abs() < 1.0e-8);
        assert!((post_velocity[4] + candidate.contact_two_tau * p[4]).abs() < 1.0e-8);
        assert!((post_velocity[5] + candidate.contact_two_tau * p[5]).abs() < 1.0e-8);
        assert!(0.2 * p[0] - p.fixed_rows::<2>(1).norm() > 1.0);
        assert!(candidate.kinetic_energy_delta < 0.0);
    }

    fn both_sliding_coupled_fixture() -> (Matrix6, Vector6, Vector6) {
        let mut w = Matrix6::identity();
        let coupling = 0.25;
        for axis in 0..3 {
            w[(axis, axis + 3)] = coupling;
            w[(axis + 3, axis)] = coupling;
        }

        let free_velocity = Vector6::new(-1.25, 1.25, 0.0, -1.25, 1.25, 0.0);
        let impulse = Vector6::new(1.0, -0.5, 0.0, 1.0, -0.5, 0.0);
        (w, free_velocity, impulse)
    }

    #[test]
    fn both_contacts_can_slide_in_a_genuinely_coupled_spd_island() {
        let (w, q, p) = both_sliding_coupled_fixture();
        let mu_1 = 0.5;
        let mu_2 = 0.5;
        let tau_1 = 1.25;
        let tau_2 = 1.25;

        assert!((w - w.transpose()).norm() == 0.0);
        assert!(w.cholesky().is_some());
        assert_eq!(w[(0, 3)], 0.25);
        assert_eq!(w[(3, 0)], 0.25);

        // Both contacts are loaded and normal-post-velocity is exactly zero.
        let u = q + w * p;
        assert_eq!(u, Vector6::new(0.0, 0.625, 0.0, 0.0, 0.625, 0.0));
        assert!(p[0] > 0.0 && p[3] > 0.0);
        assert_eq!(p[0] * u[0], 0.0);
        assert_eq!(p[3] * u[3], 0.0);

        // Both impulses lie exactly on their local circular Coulomb boundaries.
        let disk_margin_1 = (mu_1 * p[0]).powi(2) - p[1].powi(2) - p[2].powi(2);
        let disk_margin_2 = (mu_2 * p[3]).powi(2) - p[4].powi(2) - p[5].powi(2);
        assert_eq!(disk_margin_1, 0.0);
        assert_eq!(disk_margin_2, 0.0);

        // Both sliding KKT multipliers are strictly positive.
        assert!(tau_1 > 0.0 && tau_2 > 0.0);
        assert_eq!(u[1] + tau_1 * p[1], 0.0);
        assert_eq!(u[2] + tau_1 * p[2], 0.0);
        assert_eq!(u[4] + tau_2 * p[4], 0.0);
        assert_eq!(u[5] + tau_2 * p[5], 0.0);

        // Full-island energy includes the off-diagonal cross-contact work.
        let energy_delta = p.dot(&q) + 0.5 * p.dot(&(w * p));
        assert_eq!(energy_delta, -2.1875);
        assert!(energy_delta < 0.0);
    }

    fn asymmetric_both_sliding_fixture() -> (Matrix6, Vector6, Vector6) {
        let cross = Matrix3::from_row_slice(&[
            1.0 / 8.0,
            1.0 / 32.0,
            -1.0 / 64.0,
            1.0 / 64.0,
            1.0 / 16.0,
            1.0 / 32.0,
            -1.0 / 32.0,
            1.0 / 64.0,
            1.0 / 16.0,
        ]);
        let mut w = Matrix6::identity();
        for row in 0..3 {
            for column in 0..3 {
                w[(row, column + 3)] = cross[(row, column)];
                w[(column + 3, row)] = cross[(row, column)];
            }
        }

        // Two distinct rational sliding directions: (3/5,4/5) and (5/13,12/13).
        let q = Vector6::new(
            -745.0 / 64.0,
            441.0 / 64.0,
            1257.0 / 128.0,
            -917.0 / 64.0,
            119.0 / 16.0,
            37.0 / 2.0,
        );
        let p = Vector6::new(10.0, -3.0, -4.0, 13.0, -2.5, -6.0);
        (w, q, p)
    }

    #[test]
    fn asymmetric_cross_contact_block_supports_two_distinct_sliding_directions() {
        let (w, q, p) = asymmetric_both_sliding_fixture();
        let mu_1: f64 = 0.5;
        let mu_2: f64 = 0.5;
        let tau_1: f64 = 1.25;
        let tau_2: f64 = 2.0;

        assert!((w - w.transpose()).norm() == 0.0);
        assert!(w.cholesky().is_some());

        // The upper cross-contact block is intentionally non-symmetric.
        assert_eq!(w[(0, 4)], 1.0 / 32.0);
        assert_eq!(w[(1, 3)], 1.0 / 64.0);
        assert_ne!(w[(0, 4)], w[(1, 3)]);

        let u = q + w * p;
        assert_eq!(u, Vector6::new(0.0, 3.75, 5.0, 0.0, 5.0, 12.0));

        assert!(p[0] > 0.0 && p[3] > 0.0);
        assert_eq!(p[0] * u[0], 0.0);
        assert_eq!(p[3] * u[3], 0.0);

        let disk_margin_1 = (mu_1 * p[0]).powi(2) - p[1].powi(2) - p[2].powi(2);
        let disk_margin_2 = (mu_2 * p[3]).powi(2) - p[4].powi(2) - p[5].powi(2);
        assert_eq!(disk_margin_1, 0.0);
        assert_eq!(disk_margin_2, 0.0);

        assert!(tau_1 > 0.0 && tau_2 > 0.0);
        assert_eq!(u[1] + tau_1 * p[1], 0.0);
        assert_eq!(u[2] + tau_1 * p[2], 0.0);
        assert_eq!(u[4] + tau_2 * p[4], 0.0);
        assert_eq!(u[5] + tau_2 * p[5], 0.0);

        let energy_delta = p.dot(&q) + 0.5 * p.dot(&(w * p));
        assert_eq!(energy_delta, -19455.0 / 64.0);
        assert!(energy_delta < 0.0);
    }

    #[test]
    fn asymmetric_double_sliding_fixture_has_two_distinct_admissible_branches() {
        const SCALE: f64 = 1_048_576.0;
        let w_numerators = [
            380_770.0,
            -33_295.0,
            274_566.0,
            145_422.0,
            341_094.0,
            356_864.0,
            -33_295.0,
            1_048_576.0,
            -338_214.0,
            -131_501.0,
            237_374.0,
            -415_672.0,
            274_566.0,
            -338_214.0,
            1_001_976.0,
            75_838.0,
            564_175.0,
            587_762.0,
            145_422.0,
            -131_501.0,
            75_838.0,
            812_094.0,
            -148_076.0,
            106_473.0,
            341_094.0,
            237_374.0,
            564_175.0,
            -148_076.0,
            769_147.0,
            341_152.0,
            356_864.0,
            -415_672.0,
            587_762.0,
            106_473.0,
            341_152.0,
            854_965.0,
        ];
        let w = Matrix6::from_row_slice(&w_numerators) / SCALE;
        let q = Vector6::from_row_slice(&[
            -109_587.0,
            -7_143_822.0,
            8_213_295.0,
            -3_226_388.0,
            7_075_809.0,
            1_695_813.0,
        ]) / SCALE;
        let mu = [1_956_059.0 / SCALE, 380_758.0 / SCALE];

        // Both cross-contact blocks are retained, with a genuinely
        // non-symmetric upper B block: B[0,1] != B[1,0].
        assert!((w - w.transpose()).norm() == 0.0);
        assert!(w.cholesky().is_some());
        assert_ne!(w[(0, 4)], w[(1, 3)]);

        let candidates = [
            (
                Vector6::new(
                    4.608_844_012_230_929_5,
                    5.277_782_770_150_771,
                    -6.786_946_962_897_831,
                    4.290_844_256_133_09,
                    -1.396_836_622_802_999_4,
                    0.690_278_538_078_889_7,
                ),
                [0.117_529_947_136_987_67, 3.138_813_543_506_759],
            ),
            (
                Vector6::new(
                    4.538_161_725_266_375_5,
                    5.222_183_363_528_369,
                    -6.663_076_668_053_674,
                    4.284_143_418_088_06,
                    -1.402_416_114_964_019_3,
                    0.673_267_915_990_852_9,
                ),
                [0.135_438_734_728_748, 3.142_289_304_836_215_7],
            ),
        ];

        for (p, tau) in candidates {
            let u = q + w * p;
            for (contact, offset) in [0_usize, 3_usize].into_iter().enumerate() {
                let lambda = p[offset];
                let tangent_norm = (p[offset + 1].powi(2) + p[offset + 2].powi(2)).sqrt();
                let friction_radius = mu[contact] * lambda;
                let disk_margin = friction_radius.powi(2) - tangent_norm.powi(2);
                let stationarity_1 = u[offset + 1] + tau[contact] * p[offset + 1];
                let stationarity_2 = u[offset + 2] + tau[contact] * p[offset + 2];

                assert!(lambda > 0.0);
                assert!(u[offset].abs() < 1.0e-8);
                assert!(disk_margin.abs() < 1.0e-8);
                assert!(tau[contact] > 0.0);
                assert!(stationarity_1.abs() < 1.0e-8);
                assert!(stationarity_2.abs() < 1.0e-8);
            }

            let energy_delta = p.dot(&q) + 0.5 * p.dot(&(w * p));
            assert!(energy_delta < -60.0);
        }

        let impulse_separation = (candidates[0].0 - candidates[1].0).norm();
        assert!(impulse_separation > 0.1 && impulse_separation < 0.2);

        // These are two isolated numerical witnesses for the same exactly
        // dyadic W/q/mu input. This is not yet a certified complete root count.
    }
}

#[cfg(test)]
mod double_sliding_direction_oracle {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    #[derive(Clone, Debug)]
    struct DirectionEvaluation {
        residual: [f64; 2],
        jacobian: [[f64; 2]; 2],
        impulse: Vector6,
        sliding_multipliers: [f64; 2],
        normal_determinant: f64,
    }

    /// Reduce the two normal equations and evaluate the coupled tangential
    /// direction system. This is an independent residual/Jacobian oracle,
    /// not a root finder or a completeness certificate.
    fn evaluate(
        w: &Matrix6,
        q: &Vector6,
        mu: [f64; 2],
        theta: [f64; 2],
    ) -> Option<DirectionEvaluation> {
        let (c1, s1) = (theta[0].cos(), theta[0].sin());
        let (c2, s2) = (theta[1].cos(), theta[1].sin());

        let d1 = Vector6::new(1.0, -mu[0] * c1, -mu[0] * s1, 0.0, 0.0, 0.0);
        let d2 = Vector6::new(0.0, 0.0, 0.0, 1.0, -mu[1] * c2, -mu[1] * s2);
        let dd1 = Vector6::new(0.0, mu[0] * s1, -mu[0] * c1, 0.0, 0.0, 0.0);
        let dd2 = Vector6::new(0.0, 0.0, 0.0, 0.0, mu[1] * s2, -mu[1] * c2);

        // H(theta) lambda = -q_n, using the original full-island W.
        let wd1 = w * d1;
        let wd2 = w * d2;
        let h = [[wd1[0], wd2[0]], [wd1[3], wd2[3]]];
        let determinant = h[0][0] * h[1][1] - h[0][1] * h[1][0];
        if !determinant.is_finite() || determinant.abs() <= 1.0e-12 {
            return None;
        }

        let rhs = [-q[0], -q[3]];
        let lambda = [
            (rhs[0] * h[1][1] - h[0][1] * rhs[1]) / determinant,
            (h[0][0] * rhs[1] - rhs[0] * h[1][0]) / determinant,
        ];
        if !lambda.iter().all(|value| value.is_finite() && *value > 0.0) {
            return None;
        }

        let impulse = d1 * lambda[0] + d2 * lambda[1];
        let post_velocity = q + w * impulse;
        let residual = [
            -s1 * post_velocity[1] + c1 * post_velocity[2],
            -s2 * post_velocity[4] + c2 * post_velocity[5],
        ];
        let sliding_multipliers = [
            (c1 * post_velocity[1] + s1 * post_velocity[2]) / (mu[0] * lambda[0]),
            (c2 * post_velocity[4] + s2 * post_velocity[5]) / (mu[1] * lambda[1]),
        ];

        // Analytic implicit derivative:
        // H d(lambda)/d(theta_j) = -(dH/d(theta_j)) lambda.
        let mut jacobian = [[0.0_f64; 2]; 2];
        for (direction, direction_lambda) in lambda.iter().copied().enumerate() {
            let d_direction = if direction == 0 { &dd1 } else { &dd2 };
            let wd_direction = w * d_direction;
            let dh_lambda = [
                wd_direction[0] * direction_lambda,
                wd_direction[3] * direction_lambda,
            ];
            let d_lambda = [
                (-h[1][1] * dh_lambda[0] + h[0][1] * dh_lambda[1]) / determinant,
                (h[1][0] * dh_lambda[0] - h[0][0] * dh_lambda[1]) / determinant,
            ];

            let d_impulse = d1 * d_lambda[0] + d2 * d_lambda[1] + d_direction * direction_lambda;
            let d_velocity = w * d_impulse;

            let dg1_direct = if direction == 0 {
                -c1 * post_velocity[1] - s1 * post_velocity[2]
            } else {
                0.0
            };
            let dg2_direct = if direction == 1 {
                -c2 * post_velocity[4] - s2 * post_velocity[5]
            } else {
                0.0
            };
            jacobian[0][direction] = dg1_direct - s1 * d_velocity[1] + c1 * d_velocity[2];
            jacobian[1][direction] = dg2_direct - s2 * d_velocity[4] + c2 * d_velocity[5];
        }

        Some(DirectionEvaluation {
            residual,
            jacobian,
            impulse,
            sliding_multipliers,
            normal_determinant: determinant,
        })
    }

    fn direction_from_impulse(p: &Vector6, offset: usize) -> f64 {
        (-p[offset + 2]).atan2(-p[offset + 1])
    }

    #[test]
    fn asymmetric_double_slide_branches_are_two_distinct_simple_direction_roots() {
        const SCALE: f64 = 1_048_576.0;
        let numerators = [
            380_770.0,
            -33_295.0,
            274_566.0,
            145_422.0,
            341_094.0,
            356_864.0,
            -33_295.0,
            1_048_576.0,
            -338_214.0,
            -131_501.0,
            237_374.0,
            -415_672.0,
            274_566.0,
            -338_214.0,
            1_001_976.0,
            75_838.0,
            564_175.0,
            587_762.0,
            145_422.0,
            -131_501.0,
            75_838.0,
            812_094.0,
            -148_076.0,
            106_473.0,
            341_094.0,
            237_374.0,
            564_175.0,
            -148_076.0,
            769_147.0,
            341_152.0,
            356_864.0,
            -415_672.0,
            587_762.0,
            106_473.0,
            341_152.0,
            854_965.0,
        ];
        let w = Matrix6::from_row_slice(&numerators) / SCALE;
        let q = Vector6::from_row_slice(&[
            -109_587.0,
            -7_143_822.0,
            8_213_295.0,
            -3_226_388.0,
            7_075_809.0,
            1_695_813.0,
        ]) / SCALE;
        let mu = [1_956_059.0 / SCALE, 380_758.0 / SCALE];

        let witnesses = [
            Vector6::new(
                4.608_844_012_230_929_5,
                5.277_782_770_150_771,
                -6.786_946_962_897_831,
                4.290_844_256_133_09,
                -1.396_836_622_802_999_4,
                0.690_278_538_078_889_7,
            ),
            Vector6::new(
                4.538_161_725_266_375_5,
                5.222_183_363_528_369,
                -6.663_076_668_053_674,
                4.284_143_418_088_06,
                -1.402_416_114_964_019_3,
                0.673_267_915_990_852_9,
            ),
        ];

        let mut roots = Vec::new();
        for p in witnesses {
            let theta = [direction_from_impulse(&p, 0), direction_from_impulse(&p, 3)];
            let root = evaluate(&w, &q, mu, theta).expect("normal reduction remains regular");
            assert!(root.residual[0].abs() < 1.0e-8);
            assert!(root.residual[1].abs() < 1.0e-8);
            assert!((root.impulse - p).norm() < 1.0e-7);
            assert!(root.sliding_multipliers.iter().all(|tau| *tau > 0.0));

            let jacobian_determinant = root.jacobian[0][0] * root.jacobian[1][1]
                - root.jacobian[0][1] * root.jacobian[1][0];
            assert!(jacobian_determinant.abs() > 1.0e-8);
            assert!(root.normal_determinant.abs() > 1.0e-5);

            // Independently cross-check the analytic Jacobian against
            // central finite differences at both isolated branch candidates.
            let step = 1.0e-6;
            for (direction, _) in theta.iter().enumerate() {
                let mut plus_theta = theta;
                let mut minus_theta = theta;
                plus_theta[direction] += step;
                minus_theta[direction] -= step;
                let plus = evaluate(&w, &q, mu, plus_theta).expect("positive normal branch at +h");
                let minus =
                    evaluate(&w, &q, mu, minus_theta).expect("positive normal branch at -h");
                for (residual_index, jacobian_row) in root.jacobian.iter().enumerate() {
                    let finite_difference = (plus.residual[residual_index]
                        - minus.residual[residual_index])
                        / (2.0 * step);
                    assert!((finite_difference - jacobian_row[direction]).abs() < 2.0e-6);
                }
            }
            roots.push((theta, root));
        }

        assert!((roots[0].1.impulse - roots[1].1.impulse).norm() > 0.1);
        assert!((roots[0].0[0] - roots[1].0[0]).abs() < 0.01);
        assert!((roots[0].0[1] - roots[1].0[1]).abs() < 0.02);

        // The two roots have opposite local orientation. This is a useful
        // fold/continuation target, not by itself a branch-completeness proof.
        let det_0 = roots[0].1.jacobian[0][0] * roots[0].1.jacobian[1][1]
            - roots[0].1.jacobian[0][1] * roots[0].1.jacobian[1][0];
        let det_1 = roots[1].1.jacobian[0][0] * roots[1].1.jacobian[1][1]
            - roots[1].1.jacobian[0][1] * roots[1].1.jacobian[1][0];
        assert!(det_0 * det_1 < 0.0);
    }

    #[derive(Clone, Copy, Debug, PartialEq, Eq)]
    struct BivariateIntegerPolynomial {
        coefficients: [[i128; 5]; 5],
    }

    impl BivariateIntegerPolynomial {
        fn zero() -> Self {
            Self {
                coefficients: [[0; 5]; 5],
            }
        }

        fn constant(value: i128) -> Self {
            let mut polynomial = Self::zero();
            polynomial.coefficients[0][0] = value;
            polynomial
        }

        fn in_t1(coefficients: [i128; 3]) -> Self {
            let mut polynomial = Self::zero();
            for (degree, coefficient) in coefficients.into_iter().enumerate() {
                polynomial.coefficients[degree][0] = coefficient;
            }
            polynomial
        }

        fn in_t2(coefficients: [i128; 3]) -> Self {
            let mut polynomial = Self::zero();
            for (degree, coefficient) in coefficients.into_iter().enumerate() {
                polynomial.coefficients[0][degree] = coefficient;
            }
            polynomial
        }

        fn add(self, other: Self) -> Self {
            let mut result = Self::zero();
            for i in 0..5 {
                for j in 0..5 {
                    result.coefficients[i][j] = self.coefficients[i][j]
                        .checked_add(other.coefficients[i][j])
                        .expect("bivariate polynomial addition fits i128");
                }
            }
            result
        }

        fn sub(self, other: Self) -> Self {
            self.add(other.scale(-1))
        }

        fn scale(self, scalar: i128) -> Self {
            let mut result = Self::zero();
            for i in 0..5 {
                for j in 0..5 {
                    result.coefficients[i][j] = self.coefficients[i][j]
                        .checked_mul(scalar)
                        .expect("bivariate polynomial scaling fits i128");
                }
            }
            result
        }

        fn mul(self, other: Self) -> Self {
            let mut result = Self::zero();
            for i in 0..5 {
                for j in 0..5 {
                    let left = self.coefficients[i][j];
                    if left == 0 {
                        continue;
                    }
                    for k in 0..5 {
                        for l in 0..5 {
                            let right = other.coefficients[k][l];
                            if right == 0 {
                                continue;
                            }
                            assert!(
                                i + k < 5 && j + l < 5,
                                "unexpected polynomial degree overflow"
                            );
                            let term = left
                                .checked_mul(right)
                                .expect("bivariate polynomial product fits i128");
                            result.coefficients[i + k][j + l] = result.coefficients[i + k][j + l]
                                .checked_add(term)
                                .expect("bivariate polynomial accumulation fits i128");
                        }
                    }
                }
            }
            result
        }

        fn max_bidegree(self) -> (usize, usize) {
            let mut degree_t1 = 0;
            let mut degree_t2 = 0;
            for i in 0..5 {
                for j in 0..5 {
                    if self.coefficients[i][j] != 0 {
                        degree_t1 = degree_t1.max(i);
                        degree_t2 = degree_t2.max(j);
                    }
                }
            }
            (degree_t1, degree_t2)
        }

        /// Evaluate at x = x_num/x_den, y = y_num/y_den without division.
        /// The return value is the exact numerator over
        /// x_den^degree_t1 * y_den^degree_t2.
        fn evaluate_homogeneous_numerator(
            self,
            x_num: i128,
            x_den: i128,
            degree_t1: usize,
            y_num: i128,
            y_den: i128,
            degree_t2: usize,
        ) -> i128 {
            let mut value = 0_i128;
            for i in 0..=degree_t1 {
                for j in 0..=degree_t2 {
                    let coefficient = self.coefficients[i][j];
                    if coefficient == 0 {
                        continue;
                    }
                    let term = coefficient
                        .checked_mul(x_num.pow(i as u32))
                        .and_then(|v| v.checked_mul(x_den.pow((degree_t1 - i) as u32)))
                        .and_then(|v| v.checked_mul(y_num.pow(j as u32)))
                        .and_then(|v| v.checked_mul(y_den.pow((degree_t2 - j) as u32)))
                        .expect("homogeneous exact polynomial evaluation fits i128");
                    value = value
                        .checked_add(term)
                        .expect("homogeneous exact polynomial sum fits i128");
                }
            }
            value
        }
    }

    fn linear_combination(
        coefficients: [i128; 3],
        values: [BivariateIntegerPolynomial; 3],
    ) -> BivariateIntegerPolynomial {
        values[0]
            .scale(coefficients[0])
            .add(values[1].scale(coefficients[1]))
            .add(values[2].scale(coefficients[2]))
    }

    struct ExactDoubleSlidePolynomials {
        determinant: BivariateIntegerPolynomial,
        normal_numerator_1: BivariateIntegerPolynomial,
        normal_numerator_2: BivariateIntegerPolynomial,
        sliding_numerator_1: BivariateIntegerPolynomial,
        sliding_numerator_2: BivariateIntegerPolynomial,
        tangential_equation_1: BivariateIntegerPolynomial,
        tangential_equation_2: BivariateIntegerPolynomial,
    }

    /// Construct the two exact half-angle direction equations from integer
    /// numerators W_n, q_n, mu_n sharing one positive denominator S.
    ///
    /// v_k = S (1 + t_k^2) d_k, where
    /// d_k = (1, -mu_k cos(theta_k), -mu_k sin(theta_k)).
    /// With H the 2x2 normal response and Delta its cleared numerator:
    ///   N1 = -q_n1 H22 + q_n2 H12
    ///   N2 =  q_n1 H21 - q_n2 H11
    ///   p_num = (v1 N1, v2 N2)
    ///   u_num = q_n Delta + W_n p_num.
    /// Multiplying the two tangential equations by the cleared denominator
    /// gives exact integer bivariate polynomials G1 and G2.
    fn build_exact_double_slide_polynomials(
        w_numerators: [[i128; 6]; 6],
        q_numerators: [i128; 6],
        friction_numerators: [i128; 2],
        common_denominator: i128,
    ) -> ExactDoubleSlidePolynomials {
        assert!(common_denominator > 0);

        let q1 = q_numerators[0];
        let q2 = q_numerators[3];
        let v1 = [
            BivariateIntegerPolynomial::in_t1([common_denominator, 0, common_denominator]),
            BivariateIntegerPolynomial::in_t1([-friction_numerators[0], 0, friction_numerators[0]]),
            BivariateIntegerPolynomial::in_t1([0, -2 * friction_numerators[0], 0]),
        ];
        let v2 = [
            BivariateIntegerPolynomial::in_t2([common_denominator, 0, common_denominator]),
            BivariateIntegerPolynomial::in_t2([-friction_numerators[1], 0, friction_numerators[1]]),
            BivariateIntegerPolynomial::in_t2([0, -2 * friction_numerators[1], 0]),
        ];

        let h11 = linear_combination(
            [w_numerators[0][0], w_numerators[0][1], w_numerators[0][2]],
            v1,
        );
        let h12 = linear_combination(
            [w_numerators[0][3], w_numerators[0][4], w_numerators[0][5]],
            v2,
        );
        let h21 = linear_combination(
            [w_numerators[3][0], w_numerators[3][1], w_numerators[3][2]],
            v1,
        );
        let h22 = linear_combination(
            [w_numerators[3][3], w_numerators[3][4], w_numerators[3][5]],
            v2,
        );

        let determinant = h11.mul(h22).sub(h12.mul(h21));
        let normal_numerator_1 = h22.scale(-q1).add(h12.scale(q2));
        let normal_numerator_2 = h21.scale(q1).sub(h11.scale(q2));

        let mut impulse_numerators = [BivariateIntegerPolynomial::zero(); 6];
        for axis in 0..3 {
            impulse_numerators[axis] = v1[axis].mul(normal_numerator_1);
            impulse_numerators[axis + 3] = v2[axis].mul(normal_numerator_2);
        }

        let mut velocity_numerators = [BivariateIntegerPolynomial::zero(); 6];
        for row in 0..6 {
            let mut coupled_impulse = BivariateIntegerPolynomial::zero();
            for column in 0..6 {
                coupled_impulse = coupled_impulse
                    .add(impulse_numerators[column].scale(w_numerators[row][column]));
            }
            velocity_numerators[row] = determinant.scale(q_numerators[row]).add(coupled_impulse);
        }

        let t1 = BivariateIntegerPolynomial::in_t1([0, 1, 0]);
        let t2 = BivariateIntegerPolynomial::in_t2([0, 1, 0]);
        let one = BivariateIntegerPolynomial::constant(1);
        let one_minus_t1_squared = one.sub(BivariateIntegerPolynomial::in_t1([0, 0, 1]));
        let one_minus_t2_squared = one.sub(BivariateIntegerPolynomial::in_t2([0, 0, 1]));

        let tangential_equation_1 = velocity_numerators[1]
            .mul(t1.scale(-2))
            .add(one_minus_t1_squared.mul(velocity_numerators[2]));
        let tangential_equation_2 = velocity_numerators[4]
            .mul(t2.scale(-2))
            .add(one_minus_t2_squared.mul(velocity_numerators[5]));
        let sliding_numerator_1 = one_minus_t1_squared
            .mul(velocity_numerators[1])
            .add(t1.scale(2).mul(velocity_numerators[2]));
        let sliding_numerator_2 = one_minus_t2_squared
            .mul(velocity_numerators[4])
            .add(t2.scale(2).mul(velocity_numerators[5]));

        ExactDoubleSlidePolynomials {
            determinant,
            normal_numerator_1,
            normal_numerator_2,
            sliding_numerator_1,
            sliding_numerator_2,
            tangential_equation_1,
            tangential_equation_2,
        }
    }

    fn asymmetric_rational_polynomial_fixture() -> ExactDoubleSlidePolynomials {
        // The physical inputs are all exact rationals with common denominator 128.
        let mut w = [[0_i128; 6]; 6];
        for (index, row) in w.iter_mut().enumerate() {
            row[index] = 128;
        }
        let cross = [[16_i128, 4, -2], [2, 8, 4], [-4, 2, 8]];
        for row in 0..3 {
            for column in 0..3 {
                w[row][column + 3] = cross[row][column];
                w[column + 3][row] = cross[row][column];
            }
        }
        let q = [-1490_i128, 882, 1257, -1834, 952, 2368];
        let mu = [64_i128, 64];
        build_exact_double_slide_polynomials(w, q, mu, 128)
    }

    #[test]
    fn exact_asymmetric_direction_polynomials_recover_a_rational_double_slide_root() {
        let polynomials = asymmetric_rational_polynomial_fixture();
        assert_eq!(polynomials.determinant.max_bidegree(), (2, 2));
        assert_eq!(polynomials.normal_numerator_1.max_bidegree(), (0, 2));
        assert_eq!(polynomials.normal_numerator_2.max_bidegree(), (2, 0));
        assert_eq!(polynomials.tangential_equation_1.max_bidegree(), (4, 2));
        assert_eq!(polynomials.tangential_equation_2.max_bidegree(), (2, 4));

        // Directions (3/5,4/5) and (5/13,12/13) correspond to
        // t1 = tan(theta1/2) = 1/2 and t2 = tan(theta2/2) = 2/3.
        // Both exact integer polynomial numerators must vanish.
        let g1 = polynomials
            .tangential_equation_1
            .evaluate_homogeneous_numerator(1, 2, 4, 2, 3, 2);
        let g2 = polynomials
            .tangential_equation_2
            .evaluate_homogeneous_numerator(1, 2, 2, 2, 3, 4);
        assert_eq!(g1, 0);
        assert_eq!(g2, 0);

        // The eliminated normal system is regular at this root, so this is
        // not an artificial zero introduced only by clearing det(H).
        let determinant = polynomials
            .determinant
            .evaluate_homogeneous_numerator(1, 2, 2, 2, 3, 2);
        assert_ne!(determinant, 0);

        // Both normal multiplier numerators have the same sign as Delta;
        // the reconstructed normal impulses are therefore strictly positive.
        let n1 = polynomials
            .normal_numerator_1
            .evaluate_homogeneous_numerator(2, 3, 0, 2, 3, 2);
        let n2 = polynomials
            .normal_numerator_2
            .evaluate_homogeneous_numerator(1, 2, 2, 2, 3, 0);
        assert!(n1 * determinant > 0);
        assert!(n2 * determinant > 0);
    }

    const RESULTANT_COEFFICIENTS: usize = 25;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct BigIntegerPolynomial {
        coefficients: [BigInt; RESULTANT_COEFFICIENTS],
    }

    impl BigIntegerPolynomial {
        fn zero() -> Self {
            Self {
                coefficients: std::array::from_fn(|_| BigInt::from(0_u8)),
            }
        }

        fn one() -> Self {
            let mut polynomial = Self::zero();
            polynomial.coefficients[0] = BigInt::from(1_u8);
            polynomial
        }

        fn from_bivariate_t1_coefficient(
            polynomial: BivariateIntegerPolynomial,
            degree_t1: usize,
        ) -> Self {
            let mut result = Self::zero();
            for (degree_t2, coefficient) in result.coefficients.iter_mut().take(5).enumerate() {
                *coefficient = BigInt::from(polynomial.coefficients[degree_t1][degree_t2]);
            }
            result
        }

        fn from_bivariate_t2_coefficient(
            polynomial: BivariateIntegerPolynomial,
            degree_t2: usize,
        ) -> Self {
            let mut result = Self::zero();
            for (degree_t1, coefficient) in result.coefficients.iter_mut().take(5).enumerate() {
                *coefficient = BigInt::from(polynomial.coefficients[degree_t1][degree_t2]);
            }
            result
        }

        fn is_zero(&self) -> bool {
            self.coefficients
                .iter()
                .all(|coefficient| coefficient == &BigInt::from(0_u8))
        }

        fn sub(&self, other: &Self) -> Self {
            let mut result = Self::zero();
            for index in 0..RESULTANT_COEFFICIENTS {
                result.coefficients[index] = &self.coefficients[index] - &other.coefficients[index];
            }
            result
        }

        fn derivative(&self) -> Self {
            let mut result = Self::zero();
            for (degree, coefficient) in self.coefficients.iter().enumerate().skip(1) {
                result.coefficients[degree - 1] = coefficient * BigInt::from(degree);
            }
            result
        }

        fn evaluate_interval(&self, value: &RationalInterval) -> RationalInterval {
            let zero = BigRational::from_integer(BigInt::from(0_u8));
            let mut result = RationalInterval::point(zero);
            for coefficient in self.coefficients.iter().rev() {
                result =
                    result
                        .mul(value)
                        .add(&RationalInterval::point(BigRational::from_integer(
                            coefficient.clone(),
                        )));
            }
            result
        }

        fn add_assign(&mut self, other: &Self) {
            for index in 0..RESULTANT_COEFFICIENTS {
                self.coefficients[index] = &self.coefficients[index] + &other.coefficients[index];
            }
        }

        fn sub_assign(&mut self, other: &Self) {
            for index in 0..RESULTANT_COEFFICIENTS {
                self.coefficients[index] = &self.coefficients[index] - &other.coefficients[index];
            }
        }

        fn mul(&self, other: &Self) -> Self {
            let mut result = Self::zero();
            for left_degree in 0..RESULTANT_COEFFICIENTS {
                if self.coefficients[left_degree] == BigInt::from(0_u8) {
                    continue;
                }
                for right_degree in 0..(RESULTANT_COEFFICIENTS - left_degree) {
                    if other.coefficients[right_degree] == BigInt::from(0_u8) {
                        continue;
                    }
                    let product =
                        &self.coefficients[left_degree] * &other.coefficients[right_degree];
                    result.coefficients[left_degree + right_degree] =
                        &result.coefficients[left_degree + right_degree] + product;
                }
            }
            result
        }

        fn degree(&self) -> Option<usize> {
            (0..RESULTANT_COEFFICIENTS)
                .rev()
                .find(|degree| self.coefficients[*degree] != BigInt::from(0_u8))
        }

        /// Exact numerator of P(num/den) after multiplication by den^degree.
        fn evaluate_homogeneous_numerator(
            &self,
            numerator: i128,
            denominator: i128,
            degree: usize,
        ) -> BigInt {
            let numerator = BigInt::from(numerator);
            let denominator = BigInt::from(denominator);
            let mut result = BigInt::from(0_u8);
            for power in 0..=degree {
                let coefficient = &self.coefficients[power];
                if coefficient == &BigInt::from(0_u8) {
                    continue;
                }
                let term = coefficient
                    * numerator.pow(power as u32)
                    * denominator.pow((degree - power) as u32);
                result += term;
            }
            result
        }

        fn maximum_decimal_digits(&self) -> usize {
            self.coefficients
                .iter()
                .map(|coefficient| coefficient.to_string().trim_start_matches('-').len())
                .max()
                .unwrap_or(1)
        }
    }

    /// Compute the exact Sylvester resultant Res_t1(G1,G2) using arbitrary
    /// precision integer coefficients. G1 has t1-degree four and G2 has
    /// t1-degree two, so the Sylvester matrix is 6x6 and the resultant's
    /// t2-degree is bounded by 4*4 + 2*2 = 20.
    fn exact_direction_resultant(
        g1: BivariateIntegerPolynomial,
        g2: BivariateIntegerPolynomial,
    ) -> BigIntegerPolynomial {
        const DEGREE_G1_T1: usize = 4;
        const DEGREE_G2_T1: usize = 2;
        const SYLVESTER_DIMENSION: usize = DEGREE_G1_T1 + DEGREE_G2_T1;

        let g1_coefficients: [BigIntegerPolynomial; DEGREE_G1_T1 + 1] =
            std::array::from_fn(|index| {
                BigIntegerPolynomial::from_bivariate_t1_coefficient(g1, DEGREE_G1_T1 - index)
            });
        let g2_coefficients: [BigIntegerPolynomial; DEGREE_G2_T1 + 1] =
            std::array::from_fn(|index| {
                BigIntegerPolynomial::from_bivariate_t1_coefficient(g2, DEGREE_G2_T1 - index)
            });

        let mut sylvester: [[BigIntegerPolynomial; SYLVESTER_DIMENSION]; SYLVESTER_DIMENSION] =
            std::array::from_fn(|_| std::array::from_fn(|_| BigIntegerPolynomial::zero()));

        // Two shifted rows of G1 (degree two of G2).
        for shift in 0..DEGREE_G2_T1 {
            sylvester[shift][shift..shift + DEGREE_G1_T1 + 1].clone_from_slice(&g1_coefficients);
        }
        // Four shifted rows of G2 (degree four of G1).
        for shift in 0..DEGREE_G1_T1 {
            sylvester[DEGREE_G2_T1 + shift][shift..shift + DEGREE_G2_T1 + 1]
                .clone_from_slice(&g2_coefficients);
        }

        fn accumulate_permutations(
            row: usize,
            used_columns: u8,
            permutation: &mut [usize; SYLVESTER_DIMENSION],
            matrix: &[[BigIntegerPolynomial; SYLVESTER_DIMENSION]; SYLVESTER_DIMENSION],
            determinant: &mut BigIntegerPolynomial,
        ) {
            if row == SYLVESTER_DIMENSION {
                let mut term = BigIntegerPolynomial::one();
                for matrix_row in 0..SYLVESTER_DIMENSION {
                    term = term.mul(&matrix[matrix_row][permutation[matrix_row]]);
                }
                let mut inversions = 0;
                for left in 0..SYLVESTER_DIMENSION {
                    for right in (left + 1)..SYLVESTER_DIMENSION {
                        if permutation[left] > permutation[right] {
                            inversions += 1;
                        }
                    }
                }
                if inversions % 2 == 0 {
                    determinant.add_assign(&term);
                } else {
                    determinant.sub_assign(&term);
                }
                return;
            }

            for column in 0..SYLVESTER_DIMENSION {
                let bit = 1_u8 << column;
                if used_columns & bit != 0 {
                    continue;
                }
                permutation[row] = column;
                accumulate_permutations(
                    row + 1,
                    used_columns | bit,
                    permutation,
                    matrix,
                    determinant,
                );
            }
        }

        let mut determinant = BigIntegerPolynomial::zero();
        let mut permutation = [0_usize; SYLVESTER_DIMENSION];
        accumulate_permutations(0, 0, &mut permutation, &sylvester, &mut determinant);
        determinant
    }

    #[test]
    fn exact_sylvester_resultant_has_degree_twenty_and_contains_known_direction_root() {
        let polynomials = asymmetric_rational_polynomial_fixture();
        let resultant = exact_direction_resultant(
            polynomials.tangential_equation_1,
            polynomials.tangential_equation_2,
        );

        assert_eq!(resultant.degree(), Some(20));
        assert!(resultant.maximum_decimal_digits() >= 30);

        // The exact common root pair t1=1/2, t2=2/3 forces t2=2/3
        // to be a root of Res_t1(G1,G2). The bivariate test above separately
        // verifies the t1 lift and the nonzero normal-response determinant.
        let projected_root = resultant.evaluate_homogeneous_numerator(2, 3, 20);
        assert_eq!(projected_root, BigInt::from(0_u8));
    }

    type RationalUnivariatePolynomial = Vec<BigRational>;

    fn trim_rational_polynomial(polynomial: &mut RationalUnivariatePolynomial) {
        while polynomial.len() > 1
            && polynomial.last() == Some(&BigRational::from_integer(BigInt::from(0_u8)))
        {
            polynomial.pop();
        }
    }

    fn resultant_as_rational_polynomial(
        polynomial: &BigIntegerPolynomial,
    ) -> RationalUnivariatePolynomial {
        let degree = polynomial
            .degree()
            .expect("resultant polynomial is nonzero");
        let mut result = (0..=degree)
            .map(|index| BigRational::from_integer(polynomial.coefficients[index].clone()))
            .collect::<Vec<_>>();
        trim_rational_polynomial(&mut result);
        result
    }

    fn rational_polynomial_derivative(
        polynomial: &RationalUnivariatePolynomial,
    ) -> RationalUnivariatePolynomial {
        if polynomial.len() <= 1 {
            return vec![BigRational::from_integer(BigInt::from(0_u8))];
        }
        let mut derivative = (1..polynomial.len())
            .map(|degree| {
                polynomial[degree].clone() * BigRational::from_integer(BigInt::from(degree))
            })
            .collect::<Vec<_>>();
        trim_rational_polynomial(&mut derivative);
        derivative
    }

    fn rational_polynomial_evaluate(
        polynomial: &RationalUnivariatePolynomial,
        value: &BigRational,
    ) -> BigRational {
        polynomial.iter().rev().fold(
            BigRational::from_integer(BigInt::from(0_u8)),
            |acc, coefficient| acc * value.clone() + coefficient.clone(),
        )
    }

    fn rational_polynomial_remainder(
        dividend: &RationalUnivariatePolynomial,
        divisor: &RationalUnivariatePolynomial,
    ) -> RationalUnivariatePolynomial {
        let zero = BigRational::from_integer(BigInt::from(0_u8));
        let mut remainder = dividend.clone();
        trim_rational_polynomial(&mut remainder);
        let mut divisor = divisor.clone();
        trim_rational_polynomial(&mut divisor);
        assert!(divisor.iter().any(|coefficient| coefficient != &zero));

        while remainder.len() >= divisor.len()
            && remainder.iter().any(|coefficient| coefficient != &zero)
        {
            let shift = remainder.len() - divisor.len();
            let factor = remainder.last().unwrap().clone() / divisor.last().unwrap().clone();
            for (index, divisor_coefficient) in divisor.iter().enumerate() {
                let target = shift + index;
                remainder[target] =
                    remainder[target].clone() - factor.clone() * divisor_coefficient.clone();
            }
            trim_rational_polynomial(&mut remainder);
        }
        remainder
    }

    /// Exact Sturm sequence over Q. Repeated roots are counted distinctly:
    /// Euclidean termination at the polynomial gcd is intentional.
    fn exact_sturm_sequence(
        polynomial: &RationalUnivariatePolynomial,
    ) -> Vec<RationalUnivariatePolynomial> {
        let mut first = polynomial.clone();
        trim_rational_polynomial(&mut first);
        let second = rational_polynomial_derivative(&first);
        let mut sequence = vec![first, second];

        loop {
            let length = sequence.len();
            let remainder =
                rational_polynomial_remainder(&sequence[length - 2], &sequence[length - 1]);
            if remainder
                .iter()
                .all(|coefficient| coefficient == &BigRational::from_integer(BigInt::from(0_u8)))
            {
                break;
            }
            sequence.push(
                remainder
                    .into_iter()
                    .map(|coefficient| -coefficient)
                    .collect(),
            );
        }
        sequence
    }

    fn sturm_variations_at(
        sequence: &[RationalUnivariatePolynomial],
        value: &BigRational,
    ) -> usize {
        let mut previous_sign = 0_i8;
        let mut variations = 0;
        for polynomial in sequence {
            let evaluated = rational_polynomial_evaluate(polynomial, value);
            let sign = if evaluated > BigRational::from_integer(BigInt::from(0_u8)) {
                1
            } else if evaluated < BigRational::from_integer(BigInt::from(0_u8)) {
                -1
            } else {
                0
            };
            if sign == 0 {
                continue;
            }
            if previous_sign != 0 && sign != previous_sign {
                variations += 1;
            }
            previous_sign = sign;
        }
        variations
    }

    fn sturm_root_count(
        sequence: &[RationalUnivariatePolynomial],
        left: &BigRational,
        right: &BigRational,
    ) -> usize {
        assert!(left < right);
        assert_ne!(
            rational_polynomial_evaluate(&sequence[0], left),
            BigRational::from_integer(BigInt::from(0_u8)),
            "Sturm interval left endpoint must not be a root"
        );
        assert_ne!(
            rational_polynomial_evaluate(&sequence[0], right),
            BigRational::from_integer(BigInt::from(0_u8)),
            "Sturm interval right endpoint must not be a root"
        );
        sturm_variations_at(sequence, left) - sturm_variations_at(sequence, right)
    }

    fn cauchy_integer_root_bound(polynomial: &BigIntegerPolynomial) -> BigInt {
        let degree = polynomial
            .degree()
            .expect("resultant polynomial is nonzero");
        let leading = polynomial.coefficients[degree].clone();
        let leading_abs = if leading < BigInt::from(0_u8) {
            -leading
        } else {
            leading
        };
        let mut maximum = BigInt::from(0_u8);
        for coefficient in polynomial.coefficients.iter().take(degree) {
            let absolute = if coefficient < &BigInt::from(0_u8) {
                -coefficient
            } else {
                coefficient.clone()
            };
            if absolute > maximum {
                maximum = absolute;
            }
        }
        let quotient = &maximum / &leading_abs;
        let remainder = &maximum % &leading_abs;
        let mut bound = quotient + BigInt::from(1_u8);
        if remainder != BigInt::from(0_u8) {
            bound += BigInt::from(1_u8);
        }
        bound
    }

    fn choose_nonroot_split(
        polynomial: &RationalUnivariatePolynomial,
        left: &BigRational,
        right: &BigRational,
    ) -> BigRational {
        let width = right.clone() - left.clone();
        for denominator in 2_u64.. {
            let denominator_rational = BigRational::from_integer(BigInt::from(denominator));
            let candidate = left.clone() + width.clone() / denominator_rational;
            if rational_polynomial_evaluate(polynomial, &candidate)
                != BigRational::from_integer(BigInt::from(0_u8))
            {
                return candidate;
            }
        }
        unreachable!("a finite-degree polynomial cannot vanish at every rational split")
    }

    /// Isolate every distinct real root inside the strict Cauchy bound.
    /// Endpoints are chosen away from roots; all counts come from exact Sturm
    /// variations, and returned intervals each contain exactly one distinct
    /// real root. Repeated roots are not duplicated.
    fn isolate_all_real_resultant_roots_with_width(
        polynomial: &BigIntegerPolynomial,
        target_width: BigRational,
    ) -> Vec<(BigRational, BigRational)> {
        let rational_polynomial = resultant_as_rational_polynomial(polynomial);
        let sturm = exact_sturm_sequence(&rational_polynomial);
        let mut bound = cauchy_integer_root_bound(polynomial);
        let zero = BigRational::from_integer(BigInt::from(0_u8));

        // The Cauchy bound is inclusive. Expand if its integer endpoints happen
        // to be roots so all Sturm interval boundaries remain non-roots.
        loop {
            let left_integer = -bound.clone();
            let left = BigRational::from_integer(left_integer);
            let right = BigRational::from_integer(bound.clone());
            if rational_polynomial_evaluate(&rational_polynomial, &left) != zero
                && rational_polynomial_evaluate(&rational_polynomial, &right) != zero
            {
                break;
            }
            bound += BigInt::from(1_u8);
        }

        let left = BigRational::from_integer(-bound.clone());
        let right = BigRational::from_integer(bound);
        let total_count = sturm_root_count(&sturm, &left, &right);
        let mut pending = vec![(left, right, total_count)];
        let mut isolated = Vec::new();

        while let Some((left, right, count)) = pending.pop() {
            if count == 0 {
                continue;
            }
            if count == 1 && right.clone() - left.clone() <= target_width {
                isolated.push((left, right));
                continue;
            }

            let split = choose_nonroot_split(&rational_polynomial, &left, &right);
            let left_count = sturm_root_count(&sturm, &left, &split);
            let right_count = sturm_root_count(&sturm, &split, &right);
            assert_eq!(left_count + right_count, count);
            pending.push((split.clone(), right, right_count));
            pending.push((left, split, left_count));
        }

        isolated.sort_by(|left_interval, right_interval| left_interval.0.cmp(&right_interval.0));
        isolated
    }

    fn isolate_all_real_resultant_roots(
        polynomial: &BigIntegerPolynomial,
    ) -> Vec<(BigRational, BigRational)> {
        isolate_all_real_resultant_roots_with_width(
            polynomial,
            BigRational::new(BigInt::from(1_u8), BigInt::from(1_u64 << 30)),
        )
    }

    #[test]
    fn exact_sturm_isolates_all_four_real_projected_roots_of_the_rational_fixture() {
        let polynomials = asymmetric_rational_polynomial_fixture();
        let resultant = exact_direction_resultant(
            polynomials.tangential_equation_1,
            polynomials.tangential_equation_2,
        );
        let isolated = isolate_all_real_resultant_roots(&resultant);

        assert_eq!(resultant.degree(), Some(20));
        assert_eq!(isolated.len(), 4);

        let known_root = BigRational::new(BigInt::from(2_u8), BigInt::from(3_u8));
        let matching_intervals = isolated
            .iter()
            .filter(|(left, right)| left < &known_root && &known_root < right)
            .count();
        assert_eq!(matching_intervals, 1);

        for (left, right) in &isolated {
            assert!(left < right);
            assert!(
                right.clone() - left.clone()
                    <= BigRational::new(BigInt::from(1_u8), BigInt::from(1_u64 << 30))
            );
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct RationalInterval {
        lower: BigRational,
        upper: BigRational,
    }

    impl RationalInterval {
        fn new(lower: BigRational, upper: BigRational) -> Self {
            assert!(lower <= upper);
            Self { lower, upper }
        }

        fn point(value: BigRational) -> Self {
            Self {
                lower: value.clone(),
                upper: value,
            }
        }

        fn symmetric(center: &BigRational, radius: &BigRational) -> Self {
            assert!(radius > &BigRational::from_integer(BigInt::from(0_u8)));
            Self::new(
                center.clone() - radius.clone(),
                center.clone() + radius.clone(),
            )
        }

        fn zero_centered(radius: &BigRational) -> Self {
            Self::new(-radius.clone(), radius.clone())
        }

        fn is_point(&self) -> bool {
            self.lower == self.upper
        }

        fn excludes_zero(&self) -> bool {
            self.upper < BigRational::from_integer(BigInt::from(0_u8))
                || self.lower > BigRational::from_integer(BigInt::from(0_u8))
        }

        fn is_strictly_positive(&self) -> bool {
            self.lower > BigRational::from_integer(BigInt::from(0_u8))
        }

        fn is_strictly_negative(&self) -> bool {
            self.upper < BigRational::from_integer(BigInt::from(0_u8))
        }

        fn is_strict_subset_of(&self, other: &Self) -> bool {
            self.lower > other.lower && self.upper < other.upper
        }

        fn add(&self, other: &Self) -> Self {
            Self::new(
                self.lower.clone() + other.lower.clone(),
                self.upper.clone() + other.upper.clone(),
            )
        }

        fn neg(&self) -> Self {
            Self::new(-self.upper.clone(), -self.lower.clone())
        }

        fn sub(&self, other: &Self) -> Self {
            self.add(&other.neg())
        }

        fn mul(&self, other: &Self) -> Self {
            let products = [
                self.lower.clone() * other.lower.clone(),
                self.lower.clone() * other.upper.clone(),
                self.upper.clone() * other.lower.clone(),
                self.upper.clone() * other.upper.clone(),
            ];
            let mut lower = products[0].clone();
            let mut upper = products[0].clone();
            for value in products.iter().skip(1) {
                if value < &lower {
                    lower = value.clone();
                }
                if value > &upper {
                    upper = value.clone();
                }
            }
            Self::new(lower, upper)
        }

        fn scale(&self, scalar: &BigRational) -> Self {
            self.mul(&Self::point(scalar.clone()))
        }

        fn reciprocal(&self) -> Option<Self> {
            if !self.excludes_zero() {
                return None;
            }
            let one = BigRational::from_integer(BigInt::from(1_u8));
            Some(Self::new(
                one.clone() / self.upper.clone(),
                one / self.lower.clone(),
            ))
        }

        fn div(&self, denominator: &Self) -> Option<Self> {
            Some(self.mul(&denominator.reciprocal()?))
        }
    }

    impl BivariateIntegerPolynomial {
        fn partial_t1(&self) -> Self {
            let mut derivative = Self::zero();
            for degree_t1 in 1..5 {
                for degree_t2 in 0..5 {
                    derivative.coefficients[degree_t1 - 1][degree_t2] = self.coefficients
                        [degree_t1][degree_t2]
                        .checked_mul(degree_t1 as i128)
                        .expect("t1 derivative coefficient fits i128");
                }
            }
            derivative
        }

        fn partial_t2(&self) -> Self {
            let mut derivative = Self::zero();
            for degree_t1 in 0..5 {
                for degree_t2 in 1..5 {
                    derivative.coefficients[degree_t1][degree_t2 - 1] = self.coefficients
                        [degree_t1][degree_t2]
                        .checked_mul(degree_t2 as i128)
                        .expect("t2 derivative coefficient fits i128");
                }
            }
            derivative
        }

        /// Natural interval extension using exact rational interval arithmetic.
        /// The small coefficient grid is evaluated as a sum of monomial ranges;
        /// it is conservative, deterministic and contains the polynomial image.
        fn evaluate_interval(
            &self,
            t1: &RationalInterval,
            t2: &RationalInterval,
        ) -> RationalInterval {
            let zero = BigRational::from_integer(BigInt::from(0_u8));
            let one = BigRational::from_integer(BigInt::from(1_u8));
            let mut t1_powers: [RationalInterval; 5] =
                std::array::from_fn(|_| RationalInterval::point(one.clone()));
            let mut t2_powers: [RationalInterval; 5] =
                std::array::from_fn(|_| RationalInterval::point(one.clone()));
            for degree in 1..5 {
                t1_powers[degree] = t1_powers[degree - 1].mul(t1);
                t2_powers[degree] = t2_powers[degree - 1].mul(t2);
            }

            let mut result = RationalInterval::point(zero);
            for (degree_t1, t1_power) in t1_powers.iter().enumerate() {
                for (degree_t2, t2_power) in t2_powers.iter().enumerate() {
                    let coefficient = self.coefficients[degree_t1][degree_t2];
                    if coefficient == 0 {
                        continue;
                    }
                    let term = t1_power
                        .mul(t2_power)
                        .scale(&BigRational::from_integer(BigInt::from(coefficient)));
                    result = result.add(&term);
                }
            }
            result
        }
    }

    #[derive(Clone, Debug)]
    struct KrawczykRootBox {
        domain: [RationalInterval; 2],
        image: [RationalInterval; 2],
        normal_determinant: RationalInterval,
        normal_multiplier_ratios: [RationalInterval; 2],
        sliding_multiplier_sign_ratios: [RationalInterval; 2],
    }

    /// Certify a unique root of the reduced double-sliding polynomial system
    /// in a rational box and certify strict physical admissibility throughout
    /// that same box.
    ///
    /// The center must be an exact common root. The Krawczyk image is computed
    /// from an exact rational inverse of J(center) and an exact interval
    /// enclosure J(X). The strict inclusion K(X) subset int(X) proves one
    /// unique zero in X. Separate interval ratios prove Delta != 0, lambda > 0
    /// and tau > 0 throughout X.
    fn certify_exact_double_slide_root_box(
        polynomials: &ExactDoubleSlidePolynomials,
        center: [BigRational; 2],
        radius: BigRational,
        friction_numerators: [i128; 2],
        common_denominator: i128,
    ) -> Option<KrawczykRootBox> {
        if common_denominator <= 0 || friction_numerators.iter().any(|value| *value <= 0) {
            return None;
        }

        let domain = [
            RationalInterval::symmetric(&center[0], &radius),
            RationalInterval::symmetric(&center[1], &radius),
        ];
        let g = [
            &polynomials.tangential_equation_1,
            &polynomials.tangential_equation_2,
        ];
        let dg = [
            [g[0].partial_t1(), g[0].partial_t2()],
            [g[1].partial_t1(), g[1].partial_t2()],
        ];

        let center_box = [
            RationalInterval::point(center[0].clone()),
            RationalInterval::point(center[1].clone()),
        ];
        let center_residual = [
            g[0].evaluate_interval(&center_box[0], &center_box[1]),
            g[1].evaluate_interval(&center_box[0], &center_box[1]),
        ];
        if center_residual.iter().any(|value| !value.is_point()) {
            return None;
        }

        let zero_rational = BigRational::from_integer(BigInt::from(0_u8));
        let mut jacobian_at_center: [[BigRational; 2]; 2] =
            std::array::from_fn(|_| std::array::from_fn(|_| zero_rational.clone()));
        let mut jacobian_interval = [
            [
                RationalInterval::point(BigRational::from_integer(BigInt::from(0_u8))),
                RationalInterval::point(BigRational::from_integer(BigInt::from(0_u8))),
            ],
            [
                RationalInterval::point(BigRational::from_integer(BigInt::from(0_u8))),
                RationalInterval::point(BigRational::from_integer(BigInt::from(0_u8))),
            ],
        ];
        for row in 0..2 {
            for column in 0..2 {
                jacobian_at_center[row][column] = dg[row][column]
                    .evaluate_interval(&center_box[0], &center_box[1])
                    .lower
                    .clone();
                jacobian_interval[row][column] =
                    dg[row][column].evaluate_interval(&domain[0], &domain[1]);
            }
        }

        let determinant_jacobian = jacobian_at_center[0][0].clone()
            * jacobian_at_center[1][1].clone()
            - jacobian_at_center[0][1].clone() * jacobian_at_center[1][0].clone();
        if determinant_jacobian == BigRational::from_integer(BigInt::from(0_u8)) {
            return None;
        }
        let inverse = [
            [
                jacobian_at_center[1][1].clone() / determinant_jacobian.clone(),
                -jacobian_at_center[0][1].clone() / determinant_jacobian.clone(),
            ],
            [
                -jacobian_at_center[1][0].clone() / determinant_jacobian.clone(),
                jacobian_at_center[0][0].clone() / determinant_jacobian,
            ],
        ];

        let zero = BigRational::from_integer(BigInt::from(0_u8));
        let one = BigRational::from_integer(BigInt::from(1_u8));
        let mut krawczyk_matrix = [
            [
                RationalInterval::point(zero.clone()),
                RationalInterval::point(zero.clone()),
            ],
            [
                RationalInterval::point(zero.clone()),
                RationalInterval::point(zero.clone()),
            ],
        ];
        for row in 0..2 {
            for column in 0..2 {
                let product = jacobian_interval[0][column]
                    .scale(&inverse[row][0])
                    .add(&jacobian_interval[1][column].scale(&inverse[row][1]));
                let identity = if row == column {
                    one.clone()
                } else {
                    zero.clone()
                };
                krawczyk_matrix[row][column] = RationalInterval::point(identity).sub(&product);
            }
        }

        let centered_domain = [
            RationalInterval::zero_centered(&radius),
            RationalInterval::zero_centered(&radius),
        ];
        let correction_0 = inverse[0][0].clone() * center_residual[0].lower.clone()
            + inverse[0][1].clone() * center_residual[1].lower.clone();
        let correction_1 = inverse[1][0].clone() * center_residual[0].lower.clone()
            + inverse[1][1].clone() * center_residual[1].lower.clone();
        let mut image = [
            RationalInterval::point(center[0].clone() - correction_0),
            RationalInterval::point(center[1].clone() - correction_1),
        ];
        for row in 0..2 {
            for column in 0..2 {
                image[row] =
                    image[row].add(&krawczyk_matrix[row][column].mul(&centered_domain[column]));
            }
        }
        if !image[0].is_strict_subset_of(&domain[0]) || !image[1].is_strict_subset_of(&domain[1]) {
            return None;
        }

        let normal_determinant = polynomials
            .determinant
            .evaluate_interval(&domain[0], &domain[1]);
        if !normal_determinant.excludes_zero() {
            return None;
        }
        let normal_multiplier_ratios = [
            polynomials
                .normal_numerator_1
                .evaluate_interval(&domain[0], &domain[1])
                .div(&normal_determinant)?,
            polynomials
                .normal_numerator_2
                .evaluate_interval(&domain[0], &domain[1])
                .div(&normal_determinant)?,
        ];
        if normal_multiplier_ratios
            .iter()
            .any(|ratio| !ratio.is_strictly_positive())
        {
            return None;
        }

        // tau_k has the sign of sliding_numerator_k / normal_numerator_k:
        // all omitted factors are positive (S, mu_n and (1+t_k^2)^2).
        let sliding_multiplier_sign_ratios = [
            polynomials
                .sliding_numerator_1
                .evaluate_interval(&domain[0], &domain[1])
                .div(
                    &polynomials
                        .normal_numerator_1
                        .evaluate_interval(&domain[0], &domain[1]),
                )?,
            polynomials
                .sliding_numerator_2
                .evaluate_interval(&domain[0], &domain[1])
                .div(
                    &polynomials
                        .normal_numerator_2
                        .evaluate_interval(&domain[0], &domain[1]),
                )?,
        ];
        if sliding_multiplier_sign_ratios
            .iter()
            .any(|ratio| !ratio.is_strictly_positive())
        {
            return None;
        }

        Some(KrawczykRootBox {
            domain,
            image,
            normal_determinant,
            normal_multiplier_ratios,
            sliding_multiplier_sign_ratios,
        })
    }

    #[test]
    fn exact_rational_double_slide_root_has_krawczyk_and_physical_admissibility_certificate() {
        let polynomials = asymmetric_rational_polynomial_fixture();
        let center = [
            BigRational::new(BigInt::from(1_u8), BigInt::from(2_u8)),
            BigRational::new(BigInt::from(2_u8), BigInt::from(3_u8)),
        ];
        let radius = BigRational::new(BigInt::from(1_u8), BigInt::from(4096_u16));
        let certificate =
            certify_exact_double_slide_root_box(&polynomials, center, radius, [64, 64], 128)
                .expect("exact Krawczyk inclusion and strict contact-law margins");

        assert!(certificate.image[0].is_strict_subset_of(&certificate.domain[0]));
        assert!(certificate.image[1].is_strict_subset_of(&certificate.domain[1]));
        assert!(certificate.normal_determinant.excludes_zero());
        assert!(
            certificate
                .normal_multiplier_ratios
                .iter()
                .all(|ratio| ratio.is_strictly_positive())
        );
        assert!(
            certificate
                .sliding_multiplier_sign_ratios
                .iter()
                .all(|ratio| ratio.is_strictly_positive())
        );
    }

    fn asymmetric_dyadic_double_slide_polynomials() -> ExactDoubleSlidePolynomials {
        const SCALE: i128 = 1_048_576;
        let flattened_w = [
            380_770_i128,
            -33_295,
            274_566,
            145_422,
            341_094,
            356_864,
            -33_295,
            1_048_576,
            -338_214,
            -131_501,
            237_374,
            -415_672,
            274_566,
            -338_214,
            1_001_976,
            75_838,
            564_175,
            587_762,
            145_422,
            -131_501,
            75_838,
            812_094,
            -148_076,
            106_473,
            341_094,
            237_374,
            564_175,
            -148_076,
            769_147,
            341_152,
            356_864,
            -415_672,
            587_762,
            106_473,
            341_152,
            854_965,
        ];
        let w =
            std::array::from_fn(|row| std::array::from_fn(|column| flattened_w[row * 6 + column]));
        let q = [
            -109_587_i128,
            -7_143_822,
            8_213_295,
            -3_226_388,
            7_075_809,
            1_695_813,
        ];
        build_exact_double_slide_polynomials(w, q, [1_956_059, 380_758], SCALE)
    }

    #[test]
    fn asymmetric_dyadic_fixture_has_two_disjoint_krawczyk_certified_double_slide_roots() {
        let polynomials = asymmetric_dyadic_double_slide_polynomials();
        assert_eq!(polynomials.tangential_equation_1.max_bidegree(), (4, 2));
        assert_eq!(polynomials.tangential_equation_2.max_bidegree(), (2, 4));

        let denominator = BigInt::from(1_000_000_000_000_000_i128);
        let centers = [
            [
                BigRational::new(
                    BigInt::from(2_044_412_562_120_879_i128),
                    denominator.clone(),
                ),
                BigRational::new(BigInt::from(-233_602_787_384_141_i128), denominator.clone()),
            ],
            [
                BigRational::new(
                    BigInt::from(2_054_286_197_054_429_i128),
                    denominator.clone(),
                ),
                BigRational::new(BigInt::from(-227_603_740_377_268_i128), denominator.clone()),
            ],
        ];
        let radius = BigRational::new(BigInt::from(1_u8), BigInt::from(100_000_u32));
        let certificates = centers.map(|center| {
            certify_exact_double_slide_root_box(
                &polynomials,
                center,
                radius.clone(),
                [1_956_059, 380_758],
                1_048_576,
            )
            .expect("strict Krawczyk inclusion and physical sliding margins")
        });

        // The two certified boxes are disjoint in both direction coordinates.
        assert!(certificates[0].domain[0].upper < certificates[1].domain[0].lower);
        assert!(certificates[0].domain[1].upper < certificates[1].domain[1].lower);

        for certificate in &certificates {
            assert!(certificate.image[0].is_strict_subset_of(&certificate.domain[0]));
            assert!(certificate.image[1].is_strict_subset_of(&certificate.domain[1]));
            assert!(certificate.normal_determinant.excludes_zero());
            assert!(
                certificate
                    .normal_multiplier_ratios
                    .iter()
                    .all(|ratio| ratio.is_strictly_positive())
            );
            assert!(
                certificate
                    .sliding_multiplier_sign_ratios
                    .iter()
                    .all(|ratio| ratio.is_strictly_positive())
            );
        }
    }

    fn trim_big_integer_polynomial_vector(coefficients: &mut Vec<BigIntegerPolynomial>) {
        while coefficients
            .last()
            .is_some_and(BigIntegerPolynomial::is_zero)
        {
            coefficients.pop();
        }
    }

    /// Compute the pseudo-remainder of G1(t1,t2) by G2(t1,t2), viewing
    /// both as polynomials in t1 over Z[t2]. On the regular resultant
    /// stratum, the resulting degree-one polynomial R0(t2)+R1(t2)t1
    /// vanishes at every common root pair. This is an exact lifting relation.
    fn exact_subresultant_linear_lift(
        g1: BivariateIntegerPolynomial,
        g2: BivariateIntegerPolynomial,
    ) -> Option<(BigIntegerPolynomial, BigIntegerPolynomial)> {
        let mut remainder = (0..=4)
            .map(|degree_t1| BigIntegerPolynomial::from_bivariate_t1_coefficient(g1, degree_t1))
            .collect::<Vec<_>>();
        let divisor = (0..=2)
            .map(|degree_t1| BigIntegerPolynomial::from_bivariate_t1_coefficient(g2, degree_t1))
            .collect::<Vec<_>>();
        if divisor.len() != 3 || divisor.last().is_some_and(BigIntegerPolynomial::is_zero) {
            return None;
        }

        while remainder.len() >= divisor.len() {
            let shift = remainder.len() - divisor.len();
            let remainder_leading = remainder.last()?.clone();
            let divisor_leading = divisor.last()?.clone();
            let mut next = remainder
                .iter()
                .map(|coefficient| coefficient.mul(&divisor_leading))
                .collect::<Vec<_>>();
            let required_len = shift + divisor.len();
            next.resize_with(required_len, BigIntegerPolynomial::zero);

            for (index, divisor_coefficient) in divisor.iter().enumerate() {
                let product = remainder_leading.mul(divisor_coefficient);
                next[shift + index] = next[shift + index].sub(&product);
            }
            trim_big_integer_polynomial_vector(&mut next);
            if next.len() >= remainder.len() {
                return None;
            }
            remainder = next;
        }

        if remainder.len() != 2 || remainder[1].is_zero() {
            return None;
        }
        Some((remainder[0].clone(), remainder[1].clone()))
    }

    fn rational_polynomial_gcd_is_constant(
        left: &RationalUnivariatePolynomial,
        right: &RationalUnivariatePolynomial,
    ) -> bool {
        let zero = BigRational::from_integer(BigInt::from(0_u8));
        let mut a = left.clone();
        let mut b = right.clone();
        trim_rational_polynomial(&mut a);
        trim_rational_polynomial(&mut b);
        if a.iter().all(|coefficient| coefficient == &zero)
            || b.iter().all(|coefficient| coefficient == &zero)
        {
            return false;
        }

        while b.iter().any(|coefficient| coefficient != &zero) {
            let remainder = rational_polynomial_remainder(&a, &b);
            a = b;
            b = remainder;
            trim_rational_polynomial(&mut b);
        }
        trim_rational_polynomial(&mut a);
        a.len() == 1
    }

    #[test]
    fn exact_subresultant_lifts_all_four_real_projected_roots_and_classifies_physics() {
        let polynomials = asymmetric_rational_polynomial_fixture();
        let resultant = exact_direction_resultant(
            polynomials.tangential_equation_1,
            polynomials.tangential_equation_2,
        );
        assert_eq!(resultant.degree(), Some(20));

        let subresultant = exact_subresultant_linear_lift(
            polynomials.tangential_equation_1,
            polynomials.tangential_equation_2,
        )
        .expect("regular degree-one subresultant lift");
        let (lift_numerator, lift_denominator) = subresultant;

        // Isolate all distinct real projected roots to 2^-80. The narrower
        // exact boxes keep the subresultant lift and physical sign intervals
        // sharp even though its integer coefficients are large.
        let projected_roots = isolate_all_real_resultant_roots_with_width(
            &resultant,
            BigRational::new(BigInt::from(1_u8), BigInt::from(1_u8) << 80),
        );
        assert_eq!(projected_roots.len(), 4);

        // A nonzero leading coefficient in each t1 polynomial prevents a
        // degree-drop artifact from being mistaken for a common finite root.
        // The resultant derivative further proves each projected root is simple.
        let resultant_derivative = resultant.derivative();
        let leading_g1_t1 = BigIntegerPolynomial::from_bivariate_t1_coefficient(
            polynomials.tangential_equation_1,
            4,
        );
        let leading_g2_t1 = BigIntegerPolynomial::from_bivariate_t1_coefficient(
            polynomials.tangential_equation_2,
            2,
        );

        // The linear subresultant denominator and both leading coefficients
        // must exclude zero over every projected root interval. Then the
        // unique lift t1=-R0(t2)/R1(t2) is real and finite.
        let mut physical_root_count = 0;
        let mut lifted_boxes = Vec::new();
        for t2_interval in &projected_roots {
            let t2 = RationalInterval::new(t2_interval.0.clone(), t2_interval.1.clone());
            let r0 = lift_numerator.evaluate_interval(&t2);
            let r1 = lift_denominator.evaluate_interval(&t2);
            let lc1 = leading_g1_t1.evaluate_interval(&t2);
            let lc2 = leading_g2_t1.evaluate_interval(&t2);
            let projected_derivative = resultant_derivative.evaluate_interval(&t2);

            assert!(projected_derivative.excludes_zero());
            assert!(r1.excludes_zero());
            assert!(lc1.excludes_zero());
            assert!(lc2.excludes_zero());

            let t1 = r0.neg().div(&r1).expect("nonzero subresultant denominator");
            let delta = polynomials.determinant.evaluate_interval(&t1, &t2);
            assert!(delta.excludes_zero());

            let normal_1 = polynomials
                .normal_numerator_1
                .evaluate_interval(&t1, &t2)
                .div(&delta)
                .expect("normal determinant excludes zero");
            let normal_2 = polynomials
                .normal_numerator_2
                .evaluate_interval(&t1, &t2)
                .div(&delta)
                .expect("normal determinant excludes zero");
            assert!(normal_1.is_strictly_positive());
            assert!(normal_2.is_strictly_positive());

            let tau_1 = polynomials
                .sliding_numerator_1
                .evaluate_interval(&t1, &t2)
                .div(&polynomials.normal_numerator_1.evaluate_interval(&t1, &t2))
                .expect("normal numerator excludes zero");
            let tau_2 = polynomials
                .sliding_numerator_2
                .evaluate_interval(&t1, &t2)
                .div(&polynomials.normal_numerator_2.evaluate_interval(&t1, &t2))
                .expect("normal numerator excludes zero");

            let g1_t1 = polynomials.tangential_equation_1.partial_t1();
            let g1_t2 = polynomials.tangential_equation_1.partial_t2();
            let g2_t1 = polynomials.tangential_equation_2.partial_t1();
            let g2_t2 = polynomials.tangential_equation_2.partial_t2();
            let jacobian_determinant = g1_t1
                .evaluate_interval(&t1, &t2)
                .mul(&g2_t2.evaluate_interval(&t1, &t2))
                .sub(
                    &g1_t2
                        .evaluate_interval(&t1, &t2)
                        .mul(&g2_t1.evaluate_interval(&t1, &t2)),
                );
            assert!(
                jacobian_determinant.excludes_zero(),
                "all four lifted direction roots are simple"
            );

            if tau_1.is_strictly_positive() && tau_2.is_strictly_positive() {
                physical_root_count += 1;
            } else {
                assert!(
                    tau_1.is_strictly_negative() || tau_2.is_strictly_negative(),
                    "all nonphysical projected roots must violate a strict sliding KKT sign"
                );
            }
            lifted_boxes.push((t1, t2));
        }

        assert_eq!(
            physical_root_count, 1,
            "exactly one of the four finite real algebraic root pairs is strictly Coulomb-admissible"
        );

        // The exact rational root (1/2,2/3) lies in exactly one lifted pair box.
        let half = BigRational::new(BigInt::from(1_u8), BigInt::from(2_u8));
        let two_thirds = BigRational::new(BigInt::from(2_u8), BigInt::from(3_u8));
        let matching = lifted_boxes
            .iter()
            .filter(|(t1, t2)| {
                t1.lower < half && half < t1.upper && t2.lower < two_thirds && two_thirds < t2.upper
            })
            .count();
        assert_eq!(matching, 1);

        // The normal compatibility numerator is a positive quadratic in t2
        // with negative discriminant. Thus Delta=0 cannot be compatible with
        // the two normal equations at any real finite direction pair.
        let n1 = &polynomials.normal_numerator_1.coefficients[0];
        assert_ne!(n1[2], 0);
        assert!(n1[2] > 0);
        let discriminant = n1[1] * n1[1] - 4 * n1[2] * n1[0];
        assert!(discriminant < 0);

        // Exact projective charts: at t1=infinity, the leading t1
        // coefficients of G1/G2 are coprime in t2; at t2=infinity, the
        // leading t2 coefficients are coprime in t1. Hence neither boundary
        // chart contains a common root. The projective corner is also excluded.
        let g1_t2_leading = BigIntegerPolynomial::from_bivariate_t2_coefficient(
            polynomials.tangential_equation_1,
            2,
        );
        let g2_t2_leading = BigIntegerPolynomial::from_bivariate_t2_coefficient(
            polynomials.tangential_equation_2,
            4,
        );

        let t1_infinity_gcd = rational_polynomial_gcd_is_constant(
            &resultant_as_rational_polynomial(&leading_g1_t1),
            &resultant_as_rational_polynomial(&leading_g2_t1),
        );
        let t2_infinity_gcd = rational_polynomial_gcd_is_constant(
            &resultant_as_rational_polynomial(&g1_t2_leading),
            &resultant_as_rational_polynomial(&g2_t2_leading),
        );
        assert!(t1_infinity_gcd);
        assert!(t2_infinity_gcd);
        assert_ne!(polynomials.tangential_equation_1.coefficients[4][2], 0);
        assert_ne!(polynomials.tangential_equation_2.coefficients[2][4], 0);
    }

    fn exact_quadratic_power_table(coefficients: [i128; 3]) -> Vec<[BigRational; 2]> {
        let zero = BigRational::from_integer(BigInt::from(0_u8));
        let one = BigRational::from_integer(BigInt::from(1_u8));
        let constant = BigRational::from_integer(BigInt::from(coefficients[0]));
        let linear = BigRational::from_integer(BigInt::from(coefficients[1]));
        let quadratic = BigRational::from_integer(BigInt::from(coefficients[2]));
        assert_ne!(quadratic, zero);

        let alpha = -linear / quadratic.clone();
        let beta = -constant / quadratic;
        let mut powers = vec![[one.clone(), zero.clone()], [zero, one]];
        for degree in 2..5 {
            let previous = powers[degree - 1].clone();
            let previous_two = powers[degree - 2].clone();
            powers.push([
                alpha.clone() * previous[0].clone() + beta.clone() * previous_two[0].clone(),
                alpha.clone() * previous[1].clone() + beta.clone() * previous_two[1].clone(),
            ]);
        }
        powers
    }

    /// Reduce an integer bivariate polynomial modulo Q(t1)=0 and A(t2)=0.
    /// A zero 2x2 remainder proves exact vanishing at every pairing of their
    /// algebraic roots, without substituting floating approximations.
    fn reduce_bivariate_mod_quadratics(
        polynomial: &BivariateIntegerPolynomial,
        q_t1: [i128; 3],
        a_t2: [i128; 3],
    ) -> [[BigRational; 2]; 2] {
        let t1_powers = exact_quadratic_power_table(q_t1);
        let t2_powers = exact_quadratic_power_table(a_t2);
        let zero = BigRational::from_integer(BigInt::from(0_u8));
        let mut remainder: [[BigRational; 2]; 2] =
            std::array::from_fn(|_| std::array::from_fn(|_| zero.clone()));

        for (degree_t1, t1_power) in t1_powers.iter().enumerate() {
            for (degree_t2, t2_power) in t2_powers.iter().enumerate() {
                let coefficient = BigRational::from_integer(BigInt::from(
                    polynomial.coefficients[degree_t1][degree_t2],
                ));
                if coefficient == zero {
                    continue;
                }
                for reduced_t1 in 0..2 {
                    for reduced_t2 in 0..2 {
                        remainder[reduced_t1][reduced_t2] += coefficient.clone()
                            * t1_power[reduced_t1].clone()
                            * t2_power[reduced_t2].clone();
                    }
                }
            }
        }
        remainder
    }

    fn interval_from_integer_ratio(numerator: i128, denominator: i128) -> RationalInterval {
        RationalInterval::point(BigRational::new(
            BigInt::from(numerator),
            BigInt::from(denominator),
        ))
    }

    fn interval_sum(values: impl IntoIterator<Item = RationalInterval>) -> RationalInterval {
        values.into_iter().fold(
            RationalInterval::point(BigRational::from_integer(BigInt::from(0_u8))),
            |sum, value| sum.add(&value),
        )
    }

    fn compatible_singular_normal_fibre_intervals(
        t1: &RationalInterval,
        t2: &RationalInterval,
    ) -> Option<(RationalInterval, RationalInterval)> {
        const SCALE: i128 = 1_048_576;
        let flattened_w = [
            380_770_i128,
            -33_295,
            274_566,
            145_422,
            341_094,
            356_864,
            -33_295,
            1_048_576,
            -338_214,
            -131_501,
            237_374,
            -415_672,
            274_566,
            -338_214,
            1_001_976,
            75_838,
            564_175,
            587_762,
            145_422,
            -131_501,
            75_838,
            812_094,
            -148_076,
            106_473,
            341_094,
            237_374,
            564_175,
            -148_076,
            769_147,
            341_152,
            356_864,
            -415_672,
            587_762,
            106_473,
            341_152,
            854_965,
        ];
        let w_numerators: [[i128; 6]; 6] =
            std::array::from_fn(|row| std::array::from_fn(|column| flattened_w[row * 6 + column]));
        let q_numerators = [
            -109_587_i128,
            -7_143_822,
            8_213_295,
            -3_226_388,
            7_075_809,
            1_695_813,
        ];
        let mu_numerators = [1_956_059_i128, 380_758_i128];

        let one = RationalInterval::point(BigRational::from_integer(BigInt::from(1_u8)));
        let two = RationalInterval::point(BigRational::from_integer(BigInt::from(2_u8)));
        let t1_squared = t1.mul(t1);
        let t2_squared = t2.mul(t2);
        let t1_denominator = one.add(&t1_squared);
        let t2_denominator = one.add(&t2_squared);
        let c1 = one.sub(&t1_squared).div(&t1_denominator)?;
        let s1 = two.mul(t1).div(&t1_denominator)?;
        let c2 = one.sub(&t2_squared).div(&t2_denominator)?;
        let s2 = two.mul(t2).div(&t2_denominator)?;

        let mu1 = interval_from_integer_ratio(mu_numerators[0], SCALE);
        let mu2 = interval_from_integer_ratio(mu_numerators[1], SCALE);
        let d1 = [
            one.clone(),
            c1.scale(&(-mu1.lower.clone())),
            s1.scale(&(-mu1.lower.clone())),
        ];
        let d2 = [
            one.clone(),
            c2.scale(&(-mu2.lower.clone())),
            s2.scale(&(-mu2.lower.clone())),
        ];

        let w: [[RationalInterval; 6]; 6] = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                interval_from_integer_ratio(w_numerators[row][column], SCALE)
            })
        });
        let q: [RationalInterval; 6] =
            std::array::from_fn(|index| interval_from_integer_ratio(q_numerators[index], SCALE));

        // The first normal equation has a nonzero pivot and parameterizes
        // the complete compatible fibre as lambda_1 = a + b * alpha,
        // lambda_2 = alpha. Rank one and compatibility are proved separately
        // by the exact quotient-ring identities in the calling test.
        let h11 = interval_sum((0..3).map(|axis| w[0][axis].mul(&d1[axis])));
        let h12 = interval_sum((0..3).map(|axis| w[0][axis + 3].mul(&d2[axis])));
        if !h11.excludes_zero() {
            return None;
        }
        let lambda1_base = q[0].neg().div(&h11)?;
        let lambda1_slope = h12.neg().div(&h11)?;

        let mut impulse_base: [RationalInterval; 6] = std::array::from_fn(|_| {
            RationalInterval::point(BigRational::from_integer(BigInt::from(0_u8)))
        });
        let mut impulse_slope = impulse_base.clone();
        for axis in 0..3 {
            impulse_base[axis] = d1[axis].mul(&lambda1_base);
            impulse_slope[axis] = d1[axis].mul(&lambda1_slope);
            impulse_slope[axis + 3] = d2[axis].clone();
        }

        let mut velocity_base = q.clone();
        let mut velocity_slope = impulse_base.clone();
        for row in 0..6 {
            velocity_base[row] = velocity_base[row].add(&interval_sum(
                (0..6).map(|column| w[row][column].mul(&impulse_base[column])),
            ));
            velocity_slope[row] =
                interval_sum((0..6).map(|column| w[row][column].mul(&impulse_slope[column])));
        }

        // Each original tangential direction equation is affine in alpha.
        let g1_intercept = s1
            .neg()
            .mul(&velocity_base[1])
            .add(&c1.mul(&velocity_base[2]));
        let g1_slope = s1
            .neg()
            .mul(&velocity_slope[1])
            .add(&c1.mul(&velocity_slope[2]));
        let g2_intercept = s2
            .neg()
            .mul(&velocity_base[4])
            .add(&c2.mul(&velocity_base[5]));
        let g2_slope = s2
            .neg()
            .mul(&velocity_slope[4])
            .add(&c2.mul(&velocity_slope[5]));

        if !g1_slope.excludes_zero() || !g2_slope.excludes_zero() {
            return None;
        }
        let alpha_from_contact_1 = g1_intercept.neg().div(&g1_slope)?;
        let alpha_from_contact_2 = g2_intercept.neg().div(&g2_slope)?;
        Some((alpha_from_contact_1, alpha_from_contact_2))
    }

    fn prove_dyadic_delta_zero_stratum_has_no_common_tangential_multiplier_fibre() {
        const A_T2: [i128; 3] = [
            -6_635_580_832_593_066,
            -216_977_548_891_540_199,
            205_966_130_065_945_770,
        ];
        const Q_T1: [i128; 3] = [
            1_453_414_006_334_584_503,
            -3_433_061_893_381_690_836,
            1_089_540_914_828_380_489,
        ];

        let polynomials = asymmetric_dyadic_double_slide_polynomials();

        // Exact ideal-membership checks establish Delta=N1=N2=0 at every
        // algebraic pair A(t2)=Q(t1)=0, rather than merely near zero.
        for polynomial in [
            &polynomials.normal_numerator_1,
            &polynomials.normal_numerator_2,
            &polynomials.determinant,
        ] {
            let remainder = reduce_bivariate_mod_quadratics(polynomial, Q_T1, A_T2);
            assert!(
                remainder.iter().flatten().all(
                    |coefficient| coefficient == &BigRational::from_integer(BigInt::from(0_u8))
                )
            );
        }

        let mut a_t2 = BigIntegerPolynomial::zero();
        for (index, coefficient) in A_T2.into_iter().enumerate() {
            a_t2.coefficients[index] = BigInt::from(coefficient);
        }
        let mut q_t1 = BigIntegerPolynomial::zero();
        for (index, coefficient) in Q_T1.into_iter().enumerate() {
            q_t1.coefficients[index] = BigInt::from(coefficient);
        }
        let root_width = BigRational::new(BigInt::from(1_u8), BigInt::from(1_u8) << 100);
        let t2_roots = isolate_all_real_resultant_roots_with_width(&a_t2, root_width.clone());
        let t1_roots = isolate_all_real_resultant_roots_with_width(&q_t1, root_width);
        assert_eq!(t2_roots.len(), 2);
        assert_eq!(t1_roots.len(), 2);

        let mut pair_count = 0;
        for t1_bounds in &t1_roots {
            for t2_bounds in &t2_roots {
                let t1 = RationalInterval::new(t1_bounds.0.clone(), t1_bounds.1.clone());
                let t2 = RationalInterval::new(t2_bounds.0.clone(), t2_bounds.1.clone());

                // A nonzero h11 pivot plus exact Delta=0 gives rank(H)=1.
                // Exact vanishing of both Cramer numerators proves range
                // compatibility of H lambda = -q_n.
                let (alpha_1, alpha_2) = compatible_singular_normal_fibre_intervals(&t1, &t2)
                    .expect("a regular normal-row pivot and both tangential slopes");

                assert!(
                    alpha_1.upper < alpha_2.lower || alpha_2.upper < alpha_1.lower,
                    "the two contacts require disjoint values of the same fibre parameter"
                );
                pair_count += 1;
            }
        }
        assert_eq!(pair_count, 4);
    }

    #[test]
    fn dyadic_delta_zero_stratum_has_no_common_tangential_multiplier_fibre() {
        prove_dyadic_delta_zero_stratum_has_no_common_tangential_multiplier_fibre();
    }

    fn exact_quadratic_integer_polynomial(coefficients: [i128; 3]) -> BigIntegerPolynomial {
        let mut polynomial = BigIntegerPolynomial::zero();
        for (degree, coefficient) in coefficients.into_iter().enumerate() {
            polynomial.coefficients[degree] = BigInt::from(coefficient);
        }
        polynomial
    }

    fn rational_interval_midpoint(interval: &RationalInterval) -> BigRational {
        (interval.lower.clone() + interval.upper.clone())
            / BigRational::from_integer(BigInt::from(2_u8))
    }

    fn rational_intervals_overlap(left: &RationalInterval, right: &RationalInterval) -> bool {
        left.lower <= right.upper && right.lower <= left.upper
    }

    #[test]
    fn dyadic_double_slide_fixture_has_exact_global_finite_and_projective_accounting() {
        const A_T2: [i128; 3] = [
            -6_635_580_832_593_066,
            -216_977_548_891_540_199,
            205_966_130_065_945_770,
        ];
        const Q_T1: [i128; 3] = [
            1_453_414_006_334_584_503,
            -3_433_061_893_381_690_836,
            1_089_540_914_828_380_489,
        ];
        const FRICTION_NUMERATORS: [i128; 2] = [1_956_059, 380_758];
        const COMMON_DENOMINATOR: i128 = 1_048_576;

        let polynomials = asymmetric_dyadic_double_slide_polynomials();
        let resultant = exact_direction_resultant(
            polynomials.tangential_equation_1,
            polynomials.tangential_equation_2,
        );
        assert_eq!(resultant.degree(), Some(20));

        let root_width = BigRational::new(BigInt::from(1_u8), BigInt::from(1_u8) << 80);
        let projected_roots =
            isolate_all_real_resultant_roots_with_width(&resultant, root_width.clone());
        assert_eq!(
            projected_roots.len(),
            10,
            "the full Cauchy/Sturm domain contains ten distinct finite real projections"
        );

        // The exact degree-one subresultant is a lift relation on the regular
        // stratum. A zero denominator is not discarded: it is the singular
        // normal-response fibre treated by the #3256 proof below.
        let (lift_numerator, lift_denominator) = exact_subresultant_linear_lift(
            polynomials.tangential_equation_1,
            polynomials.tangential_equation_2,
        )
        .expect("the exact pseudo-remainder has a linear t1 subresultant");

        let resultant_derivative = resultant.derivative();
        let resultant_second_derivative = resultant_derivative.derivative();
        let leading_g1_t1 = BigIntegerPolynomial::from_bivariate_t1_coefficient(
            polynomials.tangential_equation_1,
            4,
        );
        let leading_g2_t1 = BigIntegerPolynomial::from_bivariate_t1_coefficient(
            polynomials.tangential_equation_2,
            2,
        );
        let leading_g1_t2 = BigIntegerPolynomial::from_bivariate_t2_coefficient(
            polynomials.tangential_equation_1,
            2,
        );
        let leading_g2_t2 = BigIntegerPolynomial::from_bivariate_t2_coefficient(
            polynomials.tangential_equation_2,
            4,
        );

        let a_polynomial = exact_quadratic_integer_polynomial(A_T2);
        let q_polynomial = exact_quadratic_integer_polynomial(Q_T1);
        let a_roots =
            isolate_all_real_resultant_roots_with_width(&a_polynomial, root_width.clone());
        let q_roots =
            isolate_all_real_resultant_roots_with_width(&q_polynomial, root_width.clone());
        assert_eq!(a_roots.len(), 2);
        assert_eq!(q_roots.len(), 2);

        let mut regular_projection_count = 0;
        let mut singular_projection_count = 0;
        let mut physical_root_boxes: Vec<[RationalInterval; 2]> = Vec::new();
        let mut nonphysical_projection_count = 0;

        for (left, right) in &projected_roots {
            let t2 = RationalInterval::new(left.clone(), right.clone());
            let r0 = lift_numerator.evaluate_interval(&t2);
            let r1 = lift_denominator.evaluate_interval(&t2);
            let first_derivative = resultant_derivative.evaluate_interval(&t2);

            if r1.excludes_zero() {
                regular_projection_count += 1;

                // A simple projected root and finite non-degenerate leading
                // coefficients make the degree-one subresultant a unique real
                // t1 lift. The two leading coefficients cannot both vanish on
                // this box, so no root is being dropped to t1=infinity.
                assert!(first_derivative.excludes_zero());
                let leading_one = leading_g1_t1.evaluate_interval(&t2);
                let leading_two = leading_g2_t1.evaluate_interval(&t2);
                assert!(
                    leading_one.excludes_zero() || leading_two.excludes_zero(),
                    "a regular projected root cannot be a common t1-degree drop"
                );

                let t1 = r0
                    .neg()
                    .div(&r1)
                    .expect("the exact subresultant denominator excludes zero");
                let delta = polynomials.determinant.evaluate_interval(&t1, &t2);
                assert!(
                    delta.excludes_zero(),
                    "a regular lift must stay outside the singular normal-response stratum"
                );

                let normal_one = polynomials
                    .normal_numerator_1
                    .evaluate_interval(&t1, &t2)
                    .div(&delta)
                    .expect("the normal determinant is bounded away from zero");
                let normal_two = polynomials
                    .normal_numerator_2
                    .evaluate_interval(&t1, &t2)
                    .div(&delta)
                    .expect("the normal determinant is bounded away from zero");
                assert!(
                    normal_one.is_strictly_positive() || normal_one.is_strictly_negative(),
                    "normal impulse 1 sign must be resolved on every regular lift"
                );
                assert!(
                    normal_two.is_strictly_positive() || normal_two.is_strictly_negative(),
                    "normal impulse 2 sign must be resolved on every regular lift"
                );

                let tau_one = polynomials
                    .sliding_numerator_1
                    .evaluate_interval(&t1, &t2)
                    .div(&polynomials.normal_numerator_1.evaluate_interval(&t1, &t2))
                    .expect("normal numerator 1 is sign definite");
                let tau_two = polynomials
                    .sliding_numerator_2
                    .evaluate_interval(&t1, &t2)
                    .div(&polynomials.normal_numerator_2.evaluate_interval(&t1, &t2))
                    .expect("normal numerator 2 is sign definite");
                assert!(
                    tau_one.is_strictly_positive() || tau_one.is_strictly_negative(),
                    "sliding multiplier 1 sign must be resolved on every regular lift"
                );
                assert!(
                    tau_two.is_strictly_positive() || tau_two.is_strictly_negative(),
                    "sliding multiplier 2 sign must be resolved on every regular lift"
                );

                let admissible = normal_one.is_strictly_positive()
                    && normal_two.is_strictly_positive()
                    && tau_one.is_strictly_positive()
                    && tau_two.is_strictly_positive();

                if admissible {
                    let center = [
                        rational_interval_midpoint(&t1),
                        rational_interval_midpoint(&t2),
                    ];
                    let radius = BigRational::new(BigInt::from(1_u8), BigInt::from(100_000_u32));
                    let certificate = certify_exact_double_slide_root_box(
                        &polynomials,
                        center,
                        radius,
                        FRICTION_NUMERATORS,
                        COMMON_DENOMINATOR,
                    )
                    .expect(
                        "every admissible regular lift must have a unique physical Krawczyk box",
                    );

                    assert!(t1.is_strict_subset_of(&certificate.domain[0]));
                    assert!(t2.is_strict_subset_of(&certificate.domain[1]));
                    // The constructor only returns after strict Krawczyk
                    // inclusion K(X) subset int(X). Replay the defining
                    // component-wise inclusion here; no separate contraction
                    // row-sum field is part of the certificate contract.
                    assert!(certificate.image[0].is_strict_subset_of(&certificate.domain[0]));
                    assert!(certificate.image[1].is_strict_subset_of(&certificate.domain[1]));
                    assert!(certificate.normal_determinant.excludes_zero());
                    physical_root_boxes.push([t1, t2]);
                } else {
                    nonphysical_projection_count += 1;
                    assert!(
                        normal_one.is_strictly_negative()
                            || normal_two.is_strictly_negative()
                            || tau_one.is_strictly_negative()
                            || tau_two.is_strictly_negative(),
                        "every nonphysical regular projection must have a strict KKT sign defeater"
                    );
                }
            } else {
                singular_projection_count += 1;

                // The subresultant numerator must vanish with its denominator;
                // these are not lost finite lifts, but the rank-one normal
                // compatibility stratum handled by the exact fibre proof.
                assert!(!r0.excludes_zero());
                assert!(
                    resultant_second_derivative
                        .evaluate_interval(&t2)
                        .excludes_zero(),
                    "the singular projections have exact multiplicity two"
                );
                assert_eq!(
                    a_roots
                        .iter()
                        .filter(|(a_left, a_right)| {
                            let a_interval = RationalInterval::new(a_left.clone(), a_right.clone());
                            rational_intervals_overlap(&a_interval, &t2)
                        })
                        .count(),
                    1,
                    "each singular projection must match exactly one root of A(t2)"
                );
            }
        }

        assert_eq!(regular_projection_count, 8);
        assert_eq!(singular_projection_count, 2);
        assert_eq!(nonphysical_projection_count, 6);
        assert_eq!(physical_root_boxes.len(), 2);

        // The two locally unique physical boxes must be disjoint. Their
        // isolation boxes came from two different subresultant projections.
        assert!(
            physical_root_boxes[0][0].upper < physical_root_boxes[1][0].lower
                || physical_root_boxes[1][0].upper < physical_root_boxes[0][0].lower
        );
        assert!(
            physical_root_boxes[0][1].upper < physical_root_boxes[1][1].lower
                || physical_root_boxes[1][1].upper < physical_root_boxes[0][1].lower
        );

        // Exact projective-chart exclusion. The leading t1 coefficient
        // polynomials and leading t2 coefficient polynomials have constant
        // gcds, so there is no common root on either infinity chart. The
        // nonzero corner coefficients also exclude the projective corner.
        assert!(rational_polynomial_gcd_is_constant(
            &resultant_as_rational_polynomial(&leading_g1_t1),
            &resultant_as_rational_polynomial(&leading_g2_t1),
        ));
        assert!(rational_polynomial_gcd_is_constant(
            &resultant_as_rational_polynomial(&leading_g1_t2),
            &resultant_as_rational_polynomial(&leading_g2_t2),
        ));
        assert_ne!(polynomials.tangential_equation_1.coefficients[4][2], 0);
        assert_ne!(polynomials.tangential_equation_2.coefficients[2][4], 0);

        // Route all four compatible singular direction pairs through the
        // exact rank-one normal fibre and two-contact tangential exclusion.
        // The four pairings are not four candidate impulses: none admits a
        // common physical fibre parameter.
        assert_eq!(q_roots.len() * a_roots.len(), 4);
        prove_dyadic_delta_zero_stratum_has_no_common_tangential_multiplier_fibre();
    }

    #[test]
    fn low_treewidth_three_contact_chain_lifts_both_certified_double_slide_branches() {
        const SCALE: i128 = 1_048_576;
        const FLATTENED_W0: [i128; 36] = [
            380_770, -33_295, 274_566, 145_422, 341_094, 356_864, -33_295, 1_048_576, -338_214,
            -131_501, 237_374, -415_672, 274_566, -338_214, 1_001_976, 75_838, 564_175, 587_762,
            145_422, -131_501, 75_838, 812_094, -148_076, 106_473, 341_094, 237_374, 564_175,
            -148_076, 769_147, 341_152, 356_864, -415_672, 587_762, 106_473, 341_152, 854_965,
        ];
        const Q0_NUMERATORS: [i128; 6] = [
            -109_587, -7_143_822, 8_213_295, -3_226_388, 7_075_809, 1_695_813,
        ];
        const FRICTION_NUMERATORS: [i128; 2] = [1_956_059, 380_758];

        let scale = BigInt::from(SCALE);
        let zero = BigRational::from_integer(BigInt::from(0_u8));
        let one = BigRational::from_integer(BigInt::from(1_u8));
        let w0: [[BigRational; 6]; 6] = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                BigRational::new(BigInt::from(FLATTENED_W0[row * 6 + column]), scale.clone())
            })
        });
        let q0: [BigRational; 6] = std::array::from_fn(|index| {
            BigRational::new(BigInt::from(Q0_NUMERATORS[index]), scale.clone())
        });
        for row in 0..6 {
            for column in 0..6 {
                assert_eq!(w0[row][column], w0[column][row]);
            }
        }

        // Exact rational LDL^T proves W0 is SPD. Therefore the full 9x9
        // block operator below is SPD by its exact Schur complement W0.
        let mut lower = vec![vec![zero.clone(); 6]; 6];
        let mut pivots: Vec<BigRational> = Vec::with_capacity(6);
        for row in 0..6 {
            lower[row][row] = one.clone();
        }
        for column in 0..6 {
            let mut pivot = w0[column][column].clone();
            for previous in 0..column {
                pivot -= lower[column][previous].clone()
                    * lower[column][previous].clone()
                    * pivots[previous].clone();
            }
            assert!(pivot > zero, "every exact LDL pivot must be positive");
            pivots.push(pivot.clone());

            for row in (column + 1)..6 {
                let mut remainder = w0[row][column].clone();
                for previous in 0..column {
                    remainder -= lower[row][previous].clone()
                        * lower[column][previous].clone()
                        * pivots[previous].clone();
                }
                lower[row][column] = remainder / pivot.clone();
            }
        }

        // Add a leaf contact at contact 3. B has no support on contact 1 and
        // equals epsilon I on contact 2; C=I and q_L=(-10,0,0).
        let epsilon = BigRational::new(BigInt::from(1_u8), BigInt::from(1_024_u16));
        let epsilon_squared = epsilon.clone() * epsilon.clone();
        let b: [[BigRational; 3]; 6] = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                if row >= 3 && row - 3 == column {
                    epsilon.clone()
                } else {
                    zero.clone()
                }
            })
        });
        let q_leaf = [
            BigRational::from_integer(BigInt::from(-10_i8)),
            zero.clone(),
            zero.clone(),
        ];
        let w3: [[BigRational; 9]; 9] = std::array::from_fn(|row| {
            std::array::from_fn(|column| {
                if row < 6 && column < 6 {
                    let mut value = w0[row][column].clone();
                    if row >= 3 && column >= 3 && row == column {
                        value += epsilon_squared.clone();
                    }
                    value
                } else if row < 6 && column >= 6 {
                    b[row][column - 6].clone()
                } else if row >= 6 && column < 6 {
                    b[column][row - 6].clone()
                } else if row == column {
                    one.clone()
                } else {
                    zero.clone()
                }
            })
        });
        let q3: [BigRational; 9] = std::array::from_fn(|row| {
            if row < 6 {
                let mut value = q0[row].clone();
                for leaf_coordinate in 0..3 {
                    value += b[row][leaf_coordinate].clone() * q_leaf[leaf_coordinate].clone();
                }
                value
            } else {
                q_leaf[row - 6].clone()
            }
        });

        // The graph is exactly 1--2--3: W13 is structurally zero, W12 and
        // W23 are nonzero. The Schur complement of C=I is exactly W0.
        assert!((0..3).any(|row| { (3..6).any(|column| w0[row][column] != zero) }));
        for row in 0..9 {
            for column in 0..9 {
                assert_eq!(w3[row][column], w3[column][row]);
            }
        }
        for contact_one_coordinate in 0..3 {
            for leaf_coordinate in 0..3 {
                assert_eq!(w3[contact_one_coordinate][6 + leaf_coordinate], zero);
            }
        }
        for coordinate in 0..3 {
            assert_eq!(w3[3 + coordinate][6 + coordinate], epsilon);
        }
        for row in 0..6 {
            for column in 0..6 {
                let mut schur_entry = w3[row][column].clone();
                for leaf_coordinate in 0..3 {
                    schur_entry -=
                        b[row][leaf_coordinate].clone() * b[column][leaf_coordinate].clone();
                }
                assert_eq!(schur_entry, w0[row][column]);
            }
        }
        assert_eq!(q3[6], q_leaf[0]);
        assert_eq!(q3[7], q_leaf[1]);
        assert_eq!(q3[8], q_leaf[2]);

        // Reuse the two disjoint exact Krawczyk boxes from the dyadic
        // double-slide fixture. Each box certifies one unique reduced branch.
        let polynomials = asymmetric_dyadic_double_slide_polynomials();
        let center_denominator = BigInt::from(1_000_000_000_000_000_i128);
        let centers = [
            [
                BigRational::new(
                    BigInt::from(2_044_412_562_120_879_i128),
                    center_denominator.clone(),
                ),
                BigRational::new(
                    BigInt::from(-233_602_787_384_141_i128),
                    center_denominator.clone(),
                ),
            ],
            [
                BigRational::new(
                    BigInt::from(2_054_286_197_054_429_i128),
                    center_denominator.clone(),
                ),
                BigRational::new(
                    BigInt::from(-227_603_740_377_268_i128),
                    center_denominator.clone(),
                ),
            ],
        ];
        let radius = BigRational::new(BigInt::from(1_u8), BigInt::from(100_000_u32));
        let certificates = centers.map(|center| {
            certify_exact_double_slide_root_box(
                &polynomials,
                center,
                radius.clone(),
                FRICTION_NUMERATORS,
                SCALE,
            )
            .expect("each known branch has a strict Krawczyk inclusion")
        });
        assert!(
            certificates[0].domain[0].upper < certificates[1].domain[0].lower
                || certificates[1].domain[0].upper < certificates[0].domain[0].lower
        );
        assert!(
            certificates[0].domain[1].upper < certificates[1].domain[1].lower
                || certificates[1].domain[1].upper < certificates[0].domain[1].lower
        );

        let mu1 = BigRational::new(BigInt::from(FRICTION_NUMERATORS[0]), scale.clone());
        let mu2 = BigRational::new(BigInt::from(FRICTION_NUMERATORS[1]), scale.clone());
        let two = BigRational::from_integer(BigInt::from(2_u8));
        let tangent_impulse = |direction: &RationalInterval,
                               friction: &BigRational,
                               normal: &RationalInterval|
         -> [RationalInterval; 2] {
            let direction_squared = direction.mul(direction);
            let denominator = RationalInterval::point(one.clone()).add(&direction_squared);
            let cosine = RationalInterval::point(one.clone())
                .sub(&direction_squared)
                .div(&denominator)
                .expect("1+t^2 is strictly positive");
            let sine = direction
                .scale(&two)
                .div(&denominator)
                .expect("1+t^2 is strictly positive");
            let friction_radius = normal.scale(friction);
            [
                friction_radius.mul(&cosine).neg(),
                friction_radius.mul(&sine).neg(),
            ]
        };

        for certificate in &certificates {
            assert!(certificate.image[0].is_strict_subset_of(&certificate.domain[0]));
            assert!(certificate.image[1].is_strict_subset_of(&certificate.domain[1]));
            assert!(certificate.normal_determinant.excludes_zero());
            assert!(
                certificate
                    .normal_multiplier_ratios
                    .iter()
                    .all(RationalInterval::is_strictly_positive)
            );
            assert!(
                certificate
                    .sliding_multiplier_sign_ratios
                    .iter()
                    .all(RationalInterval::is_strictly_positive)
            );

            let lambda1 = certificate.normal_multiplier_ratios[0].clone();
            let lambda2 = certificate.normal_multiplier_ratios[1].clone();
            let tangent1 = tangent_impulse(&certificate.domain[0], &mu1, &lambda1);
            let tangent2 = tangent_impulse(&certificate.domain[1], &mu2, &lambda2);
            let p0 = [
                lambda1,
                tangent1[0].clone(),
                tangent1[1].clone(),
                lambda2,
                tangent2[0].clone(),
                tangent2[1].clone(),
            ];

            // The leaf is reconstructed by p_L = -q_L - B^T p_0.
            // With mu_L=1, prove strict normal loading and strict disk interior
            // over the complete Krawczyk root box using rational intervals.
            let ten = RationalInterval::point(BigRational::from_integer(BigInt::from(10_i8)));
            let leaf_normal = ten.sub(&p0[3].scale(&epsilon));
            let leaf_tangent_1 = p0[4].scale(&epsilon).neg();
            let leaf_tangent_2 = p0[5].scale(&epsilon).neg();
            let leaf_disk_margin_squared = leaf_normal
                .mul(&leaf_normal)
                .sub(&leaf_tangent_1.mul(&leaf_tangent_1))
                .sub(&leaf_tangent_2.mul(&leaf_tangent_2));
            assert!(leaf_normal.is_strictly_positive());
            assert!(leaf_disk_margin_squared.is_strictly_positive());

            // Exact elimination gives q_L+B^T p_0+C p_L=0 and reduces
            // u_0 to q_0+W_0 p_0. The original two-contact Krawczyk proof
            // therefore supplies the first six contact equations unchanged.
            // Replay the reduced full-island energy by interval arithmetic.
            let mut energy0 = RationalInterval::point(zero.clone());
            for row in 0..6 {
                energy0 = energy0.add(&p0[row].scale(&q0[row]));
                for column in 0..6 {
                    let half_w = w0[row][column].clone() / two.clone();
                    energy0 = energy0.add(&p0[row].mul(&p0[column]).scale(&half_w));
                }
            }
            assert!(energy0.upper < zero);
            // Delta K_3 = Delta K_0 - 1/2 q_L^T C^{-1} q_L = Delta K_0 - 50.
            let energy3 = energy0.sub(&RationalInterval::point(BigRational::from_integer(
                BigInt::from(50_i8),
            )));
            assert!(energy3.upper < zero);
        }
    }
}
