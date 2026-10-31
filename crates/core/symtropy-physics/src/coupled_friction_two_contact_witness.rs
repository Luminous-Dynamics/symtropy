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
        -0.000039924658947135029,
        -0.0012577367209219154,
        0.0025740231899637452,
        -0.39054620,
        1.65052340,
        0.80910089,
        0.00048179797619192906,
        0.00064354662453555516,
        -0.00020792667991444019,
        -0.63542426,
        0.80910089,
        0.94522735,
        0.000058287084621405077,
        0.00033674263636546630,
        0.00019510708987712771,
        -0.000039924658947135029,
        0.00048179797619192906,
        0.000058287084621405077,
        1.0,
        0.0,
        0.0,
        -0.0012577367209219154,
        0.00064354662453555516,
        0.00033674263636546630,
        0.0,
        1.0,
        0.0,
        0.0025740231899637452,
        -0.00020792667991444019,
        0.00019510708987712771,
        0.0,
        0.0,
        1.0,
    ]);
    let q = Vector6::new(
        -0.10770302,
        1.49149027,
        -0.95561905,
        -1.0,
        0.1,
        -0.1,
    );
    (w, q)
}

fn candidate_impulses() -> [Vector6; 3] {
    [
        Vector6::new(
            7.815345510903483,
            -3.662506987009205,
            9.399827492910301,
            1.0015287249177007,
            -0.0909786816467600,
            0.0772876135118944,
        ),
        Vector6::new(
            3.0858247093883393,
            -1.840334930132206,
            3.8514965848212723,
            1.0007853776365874,
            -0.0962314667302979,
            0.0909229066155821,
        ),
        Vector6::new(
            0.2946100695600148,
            -0.2907300604870012,
            0.2855829594605342,
            1.0001351895631858,
            -0.0995385277067534,
            0.0991254970525885,
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
    let tangent_1 = Vector6::new(0.0, p[1], p[2], 0.0, 0.0, 0.0).fixed_rows::<2>(1).into_owned();
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
            assert!(MU_1 * lambda_1 - tangent_1_norm > 0.5);
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
    assert!(MU_2 * lambda_2 - tangent_2_norm > 0.8);

    // The complete island impulse is dissipative under the frozen law.
    let kinetic_energy_delta = p.dot(q) + 0.5 * p.dot(&(w * p));
    assert!(kinetic_energy_delta < -1.0e-2);

    let cross_contact_power = p[0] * (w[(0, 3)] * p[3] + w[(0, 4)] * p[4] + w[(0, 5)] * p[5])
        + p[1] * (w[(1, 3)] * p[3] + w[(1, 4)] * p[4] + w[(1, 5)] * p[5])
        + p[2] * (w[(2, 3)] * p[3] + w[(2, 4)] * p[4] + w[(2, 5)] * p[5])
        + p[3] * (w[(3, 0)] * p[0] + w[(3, 1)] * p[1] + w[(3, 2)] * p[2])
        + p[4] * (w[(4, 0)] * p[0] + w[(4, 1)] * p[1] + w[(4, 2)] * p[2])
        + p[5] * (w[(5, 0)] * p[0] + w[(5, 1)] * p[1] + w[(5, 2)] * p[2]);
    assert!(cross_contact_power.abs() > 1.0e-8);

    (kinetic_energy_delta, cross_contact_power, u.norm())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn coupled_spd_two_contact_system_has_three_distinct_admissible_impulses() {
        let (w, q) = fixture();

        // Symmetry and strict positive definiteness are checked on the full
        // coupled matrix, not on two separate 3x3 diagonal blocks.
        assert!((&w - w.transpose()).norm() < 1.0e-15);
        assert!(w.clone().cholesky().is_some());

        let mut cross_norm_squared = 0.0;
        for row in 0..3 {
            for column in 3..6 {
                cross_norm_squared += w[(row, column)] * w[(row, column)];
            }
        }
        let cross_block_norm = cross_norm_squared.sqrt();
        assert!(cross_block_norm > 3.0e-3);

        let candidates = candidate_impulses();
        let regimes = [
            (ContactOneRegime::Sticking, 0.0),
            (ContactOneRegime::Sliding, 0.19858773537606256),
            (ContactOneRegime::Sliding, 3.880015978755019),
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
        assert!(energy_deltas[0] < -8.0);
        assert!(energy_deltas[1] < -5.0);
        assert!(energy_deltas[2] < -1.0);
        assert!(cross_work.iter().any(|value| *value > 1.0e-4));
        assert!(cross_work.iter().any(|value| *value < -1.0e-7));
    }
}
