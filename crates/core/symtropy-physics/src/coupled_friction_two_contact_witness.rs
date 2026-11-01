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

        assert!((&w - w.transpose()).norm() == 0.0);
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

        assert!((&w - w.transpose()).norm() == 0.0);
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
        assert!((&w - w.transpose()).norm() == 0.0);
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

        ExactDoubleSlidePolynomials {
            determinant,
            normal_numerator_1,
            normal_numerator_2,
            tangential_equation_1,
            tangential_equation_2,
        }
    }

    fn asymmetric_rational_polynomial_fixture() -> ExactDoubleSlidePolynomials {
        // The physical inputs are all exact rationals with common denominator 128.
        let mut w = [[0_i128; 6]; 6];
        for index in 0..6 {
            w[index][index] = 128;
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
            for degree_t2 in 0..5 {
                result.coefficients[degree_t2] =
                    BigInt::from(polynomial.coefficients[degree_t1][degree_t2]);
            }
            result
        }

        fn add_assign(&mut self, other: &Self) {
            for index in 0..RESULTANT_COEFFICIENTS {
                self.coefficients[index] =
                    &self.coefficients[index] + &other.coefficients[index];
            }
        }

        fn sub_assign(&mut self, other: &Self) {
            for index in 0..RESULTANT_COEFFICIENTS {
                self.coefficients[index] =
                    &self.coefficients[index] - &other.coefficients[index];
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
                .map(|coefficient| {
                    coefficient
                        .to_string()
                        .trim_start_matches('-')
                        .len()
                })
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
                BigIntegerPolynomial::from_bivariate_t1_coefficient(
                    g1,
                    DEGREE_G1_T1 - index,
                )
            });
        let g2_coefficients: [BigIntegerPolynomial; DEGREE_G2_T1 + 1] =
            std::array::from_fn(|index| {
                BigIntegerPolynomial::from_bivariate_t1_coefficient(
                    g2,
                    DEGREE_G2_T1 - index,
                )
            });

        let mut sylvester: [[BigIntegerPolynomial; SYLVESTER_DIMENSION]; SYLVESTER_DIMENSION] =
            std::array::from_fn(|_| {
                std::array::from_fn(|_| BigIntegerPolynomial::zero())
            });

        // Two shifted rows of G1 (degree two of G2).
        for shift in 0..DEGREE_G2_T1 {
            for coefficient in 0..=DEGREE_G1_T1 {
                sylvester[shift][shift + coefficient] =
                    g1_coefficients[coefficient].clone();
            }
        }
        // Four shifted rows of G2 (degree four of G1).
        for shift in 0..DEGREE_G1_T1 {
            for coefficient in 0..=DEGREE_G2_T1 {
                sylvester[DEGREE_G2_T1 + shift][shift + coefficient] =
                    g2_coefficients[coefficient].clone();
            }
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
        accumulate_permutations(
            0,
            0,
            &mut permutation,
            &sylvester,
            &mut determinant,
        );
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
        let projected_root =
            resultant.evaluate_homogeneous_numerator(2, 3, 20);
        assert_eq!(projected_root, BigInt::from(0_u8));
    }


}
