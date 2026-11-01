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

use crate::coupled_friction::{CoupledCoulombRegime, solve_coupled_signorini_coulomb};
use nalgebra::{Matrix3, SMatrix, SVector};

type Matrix9 = SMatrix<f64, 9, 9>;
type Matrix12 = SMatrix<f64, 12, 12>;
type Jacobian9x12 = SMatrix<f64, 9, 12>;
type Vector9 = SVector<f64, 9>;
type Vector12 = SVector<f64, 12>;

const FRICTION: f64 = 0.5;
const COUPLING: f64 = -0.5;
const TOLERANCE: f64 = 1.0e-12;

#[derive(Clone, Debug)]
struct StickingLeafMessage {
    self_operator: Matrix3<f64>,
    separator_coupling: Matrix3<f64>,
    free_velocity: SVector<f64, 3>,
    friction_coefficient: f64,
}

#[derive(Clone, Debug)]
struct StickingLeafResponse {
    impulse: SVector<f64, 3>,
    post_velocity: SVector<f64, 3>,
    strict_disk_margin: f64,
}

impl StickingLeafMessage {
    /// Return the affine impulse response p_leaf = offset + gain * p_separator.
    ///
    /// The fixed-size SPD Cholesky factorization solves A offset = -q_leaf
    /// and A gain = -B^T. Rank-aware fibres are a separate response-message lane.
    fn affine_response_coefficients(
        &self,
    ) -> Result<(SVector<f64, 3>, Matrix3<f64>), &'static str> {
        if !self.self_operator.iter().all(|value| value.is_finite())
            || !self
                .separator_coupling
                .iter()
                .all(|value| value.is_finite())
            || !self.free_velocity.iter().all(|value| value.is_finite())
            || !self.friction_coefficient.is_finite()
            || self.friction_coefficient < 0.0
        {
            return Err("sticking leaf message contains invalid or non-finite data");
        }
        let factor = self
            .self_operator
            .cholesky()
            .ok_or("sticking leaf self-block is not positive definite")?;
        let offset = -factor.solve(&self.free_velocity);
        let separator_gain = -factor.solve(&self.separator_coupling.transpose());
        Ok((offset, separator_gain))
    }

    /// Return the block-Schur terms for this regime-conditioned leaf.
    ///
    /// A p_leaf + B^T p_separator + q_leaf = 0 gives
    /// W_eff = C - B A^-1 B^T and q_eff = q_separator - B A^-1 q_leaf.
    /// Using offset=-A^-1 q_leaf and gain=-A^-1 B^T:
    /// B A^-1 B^T = -B gain and B A^-1 q_leaf = -B offset.
    fn schur_terms(&self) -> Result<(Matrix3<f64>, SVector<f64, 3>), &'static str> {
        let (offset, separator_gain) = self.affine_response_coefficients()?;
        Ok((
            -(self.separator_coupling * separator_gain),
            -(self.separator_coupling * offset),
        ))
    }

    /// Evaluate the leaf's set-valued response relation at one separator impulse.
    ///
    /// The affine response is retained only where the leaf is strictly loaded,
    /// strictly inside its Coulomb disk, and zero post-contact velocity holds.
    fn response_at(
        &self,
        separator_impulse: &SVector<f64, 3>,
        tolerance: f64,
    ) -> Result<Option<StickingLeafResponse>, &'static str> {
        if !tolerance.is_finite()
            || tolerance <= 0.0
            || !separator_impulse.iter().all(|value| value.is_finite())
        {
            return Err("sticking leaf response received invalid or non-finite input");
        }
        let (offset, separator_gain) = self.affine_response_coefficients()?;
        let impulse = offset + separator_gain * separator_impulse;
        let post_velocity = self.free_velocity
            + self.separator_coupling.transpose() * separator_impulse
            + self.self_operator * impulse;
        let strict_disk_margin =
            self.friction_coefficient * impulse[0] - impulse.fixed_rows::<2>(1).norm();
        let impulse_tolerance = tolerance * impulse.norm().max(f64::MIN_POSITIVE) * 500.0;
        let velocity_scale = (self.free_velocity.norm()
            + self.separator_coupling.norm() * separator_impulse.norm()
            + self.self_operator.norm() * impulse.norm())
        .max(f64::MIN_POSITIVE);
        let velocity_tolerance = tolerance * velocity_scale * 500.0;

        // None means this valid message has no strict-sticking response at the
        // supplied separator impulse. Invalid data and unsupported self-blocks
        // are errors, so callers must not conflate domain exclusion with failure.
        if impulse[0] <= impulse_tolerance
            || strict_disk_margin <= impulse_tolerance
            || post_velocity.norm() > velocity_tolerance
        {
            return Ok(None);
        }

        Ok(Some(StickingLeafResponse {
            impulse,
            post_velocity,
            strict_disk_margin,
        }))
    }
}

#[derive(Clone, Debug)]
struct StarCandidate {
    impulse: Vector9,
    post_velocity: Vector9,
    separator_regime: CoupledCoulombRegime,
    separator_tau: f64,
    energy_delta: f64,
    effective_operator: Matrix3<f64>,
    effective_free_velocity: SVector<f64, 3>,
    leaf_disk_margins: [f64; 2],
}

fn star_body_generated_fixture(first_leaf_tangent: f64) -> (Matrix9, Vector9) {
    // Four translational bodies A/B/C/D, each with mass 2 I. The three
    // contacts are (B,C), (A,B), and (C,D), so contact 0 is the separator
    // and contacts 1/2 are leaves. Shared-body Jacobian support creates the
    // two separator/leaf blocks; the leaf-to-leaf block is structurally zero.
    let mut jacobian = Jacobian9x12::zeros();
    for axis in 0..3 {
        // Separator contact 0: v_B - v_C.
        jacobian[(axis, axis + 3)] = 1.0;
        jacobian[(axis, axis + 6)] = -1.0;
        // Leaf contact 1: v_A - v_B.
        jacobian[(axis + 3, axis)] = 1.0;
        jacobian[(axis + 3, axis + 3)] = -1.0;
        // Leaf contact 2: v_C - v_D.
        jacobian[(axis + 6, axis + 6)] = 1.0;
        jacobian[(axis + 6, axis + 9)] = -1.0;
    }

    let inverse_mass = Matrix12::identity() * 0.5;
    let w = jacobian * inverse_mass * jacobian.transpose();
    let free_body_velocity = Vector12::from_row_slice(&[
        -0.5,
        first_leaf_tangent,
        0.0,
        0.0,
        0.0,
        0.0,
        0.0,
        -1.0,
        0.0,
        0.5,
        -0.75,
        0.0,
    ]);
    let q = jacobian * free_body_velocity;
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

    // Validate the body-support-derived star sparsity rather than inferring
    // zeros by thresholding: leaf self-blocks are I and their cross block is 0.
    for row in 0..3 {
        for column in 0..3 {
            if w[(row + 3, column + 6)] != 0.0
                || w[(row + 6, column + 3)] != 0.0
                || w[(row + 3, column + 3)] != if row == column { 1.0 } else { 0.0 }
                || w[(row + 6, column + 6)] != if row == column { 1.0 } else { 0.0 }
                || w[(row, column + 3)] != if row == column { COUPLING } else { 0.0 }
                || w[(row, column + 6)] != if row == column { COUPLING } else { 0.0 }
            {
                return Err("input does not match the declared three-contact star structure");
            }
        }
    }

    let block = |row_offset: usize, column_offset: usize| {
        Matrix3::from_fn(|row, column| w[(row + row_offset, column + column_offset)])
    };
    let vector = |offset: usize| SVector::<f64, 3>::new(q[offset], q[offset + 1], q[offset + 2]);
    let separator_block = block(0, 0);
    let mut effective_operator = separator_block;
    let mut effective_free_velocity = vector(0);
    let messages = [
        StickingLeafMessage {
            self_operator: block(3, 3),
            separator_coupling: block(0, 3),
            free_velocity: vector(3),
            friction_coefficient: FRICTION,
        },
        StickingLeafMessage {
            self_operator: block(6, 6),
            separator_coupling: block(0, 6),
            free_velocity: vector(6),
            friction_coefficient: FRICTION,
        },
    ];

    // Each leaf contributes a regime-conditioned response message over the
    // same separator impulse. Their Schur terms update the reduced interface.
    for message in &messages {
        let (operator_contribution, free_velocity_contribution) = message.schur_terms()?;
        effective_operator -= operator_contribution;
        effective_free_velocity -= free_velocity_contribution;
    }

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
        let Some(leaf_1) = messages[0].response_at(&p_0, TOLERANCE)? else {
            continue;
        };
        let Some(leaf_2) = messages[1].response_at(&p_0, TOLERANCE)? else {
            continue;
        };
        let impulse = Vector9::from_row_slice(&[
            p_0[0],
            p_0[1],
            p_0[2],
            leaf_1.impulse[0],
            leaf_1.impulse[1],
            leaf_1.impulse[2],
            leaf_2.impulse[0],
            leaf_2.impulse[1],
            leaf_2.impulse[2],
        ]);
        let post_velocity = q + w * impulse;

        let impulse_scale = impulse.norm().max(f64::MIN_POSITIVE);
        let velocity_scale = (w.norm() * impulse.norm())
            .max(q.norm())
            .max(f64::MIN_POSITIVE);
        let impulse_tolerance = TOLERANCE * impulse_scale * 500.0;
        let velocity_tolerance = TOLERANCE * velocity_scale * 500.0;
        let leaf_1_velocity =
            SVector::<f64, 3>::new(post_velocity[3], post_velocity[4], post_velocity[5]);
        let leaf_2_velocity =
            SVector::<f64, 3>::new(post_velocity[6], post_velocity[7], post_velocity[8]);

        if (leaf_1_velocity - leaf_1.post_velocity).norm() > velocity_tolerance
            || (leaf_2_velocity - leaf_2.post_velocity).norm() > velocity_tolerance
            || leaf_1_velocity.norm() > velocity_tolerance
            || leaf_2_velocity.norm() > velocity_tolerance
        {
            continue;
        }

        let separator_velocity =
            SVector::<f64, 3>::new(post_velocity[0], post_velocity[1], post_velocity[2]);
        let separator_tau = separator.disk_multiplier;
        let separator_kkt =
            separator_velocity.fixed_rows::<2>(1) + separator_tau * p_0.fixed_rows::<2>(1);
        if separator.regime != CoupledCoulombRegime::Sliding
            || p_0[0] <= impulse_tolerance
            || (FRICTION * p_0[0] - p_0.fixed_rows::<2>(1).norm()).abs() > impulse_tolerance
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
            leaf_disk_margins: [leaf_1.strict_disk_margin, leaf_2.strict_disk_margin],
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
    let (w, q) = star_body_generated_fixture(-0.25);
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

    assert!((candidate.effective_operator[(0, 0)] - 0.5).abs() < 1.0e-12);
    assert!((candidate.effective_free_velocity[0] + 0.5).abs() < 1.0e-12);
    assert!((candidate.effective_free_velocity[1] - 0.75).abs() < 1.0e-12);

    assert!(candidate.post_velocity.fixed_rows::<3>(0).norm() > 0.49);
    assert!(candidate.post_velocity[0].abs() < 1.0e-9);
    assert!(candidate.post_velocity[1] > 0.49);
    assert!(candidate.post_velocity.fixed_rows::<3>(3).norm() < 1.0e-9);
    assert!(candidate.post_velocity.fixed_rows::<3>(6).norm() < 1.0e-9);
    assert!((candidate.separator_tau - 1.0).abs() < 1.0e-8);
    assert!(
        candidate
            .leaf_disk_margins
            .iter()
            .all(|margin| *margin > 0.49)
    );
    assert!((candidate.energy_delta + 0.875).abs() < 1.0e-9);
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
    let (w, q) = star_body_generated_fixture(-0.25);
    let original = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    let (permuted_w, permuted_q) = swap_star_leaves(&w, &q);
    let permuted = solve_star_with_strict_sticking_leaves(&permuted_w, &permuted_q).unwrap();

    assert_eq!(original.len(), 1);
    assert_eq!(permuted.len(), 1);
    assert!(
        (original[0].impulse.fixed_rows::<3>(0) - permuted[0].impulse.fixed_rows::<3>(0)).norm()
            < 1.0e-10
    );
    assert!(
        (original[0].impulse.fixed_rows::<3>(3) - permuted[0].impulse.fixed_rows::<3>(6)).norm()
            < 1.0e-10
    );
    assert!(
        (original[0].impulse.fixed_rows::<3>(6) - permuted[0].impulse.fixed_rows::<3>(3)).norm()
            < 1.0e-10
    );
}

#[test]
fn leaf_with_noninterior_friction_impulse_is_not_promoted_as_sticking() {
    let (w, q) = star_body_generated_fixture(-0.8);
    let candidates = solve_star_with_strict_sticking_leaves(&w, &q).unwrap();
    assert!(candidates.is_empty());
}

#[test]
fn sticking_leaf_message_returns_affine_schur_terms_and_a_local_admissibility_response() {
    let message = StickingLeafMessage {
        self_operator: Matrix3::from_diagonal(&SVector::<f64, 3>::new(2.0, 1.0, 1.0)),
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.25, 0.25)),
        free_velocity: SVector::<f64, 3>::new(-1.5, 0.125, 0.0),
        friction_coefficient: FRICTION,
    };
    let separator_impulse = SVector::<f64, 3>::new(1.0, -0.5, 0.0);

    let (response_offset, separator_gain) = message.affine_response_coefficients().unwrap();
    assert!((response_offset[0] - 0.75).abs() < 1.0e-12);
    assert!((response_offset[1] + 0.125).abs() < 1.0e-12);
    assert!((separator_gain[(0, 0)] - 0.25).abs() < 1.0e-12);
    assert!((separator_gain[(1, 1)] + 0.25).abs() < 1.0e-12);

    let (schur_operator, schur_free_velocity) = message.schur_terms().unwrap();
    assert!((schur_operator[(0, 0)] - 0.125).abs() < 1.0e-12);
    assert!((schur_operator[(1, 1)] - 0.0625).abs() < 1.0e-12);
    assert!((schur_free_velocity[0] - 0.375).abs() < 1.0e-12);
    assert!((schur_free_velocity[1] - 0.03125).abs() < 1.0e-12);

    let response = message
        .response_at(&separator_impulse, TOLERANCE)
        .unwrap()
        .expect("strictly loaded, sticking leaf response");
    assert!((response.impulse[0] - 1.0).abs() < 1.0e-12);
    assert!(response.impulse.fixed_rows::<2>(1).norm() < 1.0e-12);
    assert!(response.post_velocity.norm() < 1.0e-12);
    assert!((response.strict_disk_margin - 0.5).abs() < 1.0e-12);

    // Nearby separator impulses remain inside the same local strict-sticking
    // response region, and the message returns the corresponding affine impulse.
    let nearby_separator_impulse = SVector::<f64, 3>::new(1.1, -0.4, 0.0);
    let nearby_response = message
        .response_at(&nearby_separator_impulse, TOLERANCE)
        .unwrap()
        .expect("nearby separator impulse remains in the leaf sticking region");
    assert!((nearby_response.impulse[0] - 1.025).abs() < 1.0e-12);
    assert!((nearby_response.impulse[1] + 0.025).abs() < 1.0e-12);
    assert!(nearby_response.strict_disk_margin > 0.48);

    let reduced_c = Matrix3::identity() - schur_operator;
    let reduced_q = -SVector::<f64, 3>::new(0.0, 0.0, 0.0) - schur_free_velocity;
    assert!(
        (reduced_c - Matrix3::from_diagonal(&SVector::<f64, 3>::new(0.875, 0.9375, 0.9375))).norm()
            < 1.0e-12
    );
    assert!((reduced_q[0] + 0.375).abs() < 1.0e-12);
    assert!((reduced_q[1] + 0.03125).abs() < 1.0e-12);
}

#[test]
fn sticking_leaf_message_refuses_disk_boundary_and_invalid_self_blocks() {
    let boundary = StickingLeafMessage {
        self_operator: Matrix3::identity(),
        separator_coupling: Matrix3::zeros(),
        free_velocity: SVector::<f64, 3>::new(-1.0, 0.5, 0.0),
        friction_coefficient: FRICTION,
    };
    assert!(
        boundary
            .response_at(&SVector::<f64, 3>::zeros(), TOLERANCE)
            .unwrap()
            .is_none()
    );

    let indefinite = StickingLeafMessage {
        self_operator: Matrix3::from_diagonal(&SVector::<f64, 3>::new(1.0, -1.0, 1.0)),
        separator_coupling: Matrix3::identity(),
        free_velocity: SVector::<f64, 3>::new(-1.0, 0.0, 0.0),
        friction_coefficient: FRICTION,
    };
    assert!(indefinite.schur_terms().is_err());
    assert!(
        indefinite
            .response_at(&SVector::<f64, 3>::zeros(), TOLERANCE)
            .is_err()
    );
}


#[cfg(test)]
mod exact_sticking_response_interval {
    use super::*;
    use num_bigint::BigInt;
    use num_rational::BigRational;

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct RationalInterval {
        lower: BigRational,
        upper: BigRational,
    }

    impl RationalInterval {
        fn new(
            lower: BigRational,
            upper: BigRational,
        ) -> Result<Self, &'static str> {
            if lower > upper {
                return Err("rational interval lower endpoint exceeds upper endpoint");
            }
            Ok(Self { lower, upper })
        }

        fn point(value: BigRational) -> Self {
            Self {
                lower: value.clone(),
                upper: value,
            }
        }

        fn add(&self, other: &Self) -> Self {
            Self {
                lower: self.lower.clone() + other.lower.clone(),
                upper: self.upper.clone() + other.upper.clone(),
            }
        }

        fn scale(&self, factor: &BigRational) -> Self {
            let zero = rational(0, 1);
            if factor >= &zero {
                Self {
                    lower: self.lower.clone() * factor.clone(),
                    upper: self.upper.clone() * factor.clone(),
                }
            } else {
                Self {
                    lower: self.upper.clone() * factor.clone(),
                    upper: self.lower.clone() * factor.clone(),
                }
            }
        }

        fn maximum_absolute_value(&self) -> BigRational {
            let zero = rational(0, 1);
            let lower_absolute = if self.lower < zero {
                -self.lower.clone()
            } else {
                self.lower.clone()
            };
            let upper_absolute = if self.upper < zero {
                -self.upper.clone()
            } else {
                self.upper.clone()
            };
            lower_absolute.max(upper_absolute)
        }
    }

    #[derive(Clone, Debug, PartialEq, Eq)]
    struct StrictStickingBoxCertificate {
        leaf_impulse: [RationalInterval; 3],
        normal_impulse_lower: BigRational,
        squared_disk_margin_lower: BigRational,
    }

    fn rational(numerator: i64, denominator: i64) -> BigRational {
        BigRational::new(BigInt::from(numerator), BigInt::from(denominator))
    }

    /// Certify the affine sticking response over an entire separator-impulse
    /// box. The componentwise interval image encloses every leaf impulse in
    /// the box. A positive squared disk lower bound proves strict interior for
    /// every point, not just the centre or sampled corners.
    fn certify_strict_sticking_box(
        offset: &[BigRational; 3],
        separator_gain: &[[BigRational; 3]; 3],
        friction: &BigRational,
        separator_box: &[RationalInterval; 3],
    ) -> Result<Option<StrictStickingBoxCertificate>, &'static str> {
        let zero = rational(0, 1);
        if friction < &zero {
            return Err("friction coefficient cannot be negative");
        }
        if separator_box
            .iter()
            .any(|interval| interval.lower > interval.upper)
        {
            return Err("separator box has a reversed interval");
        }

        let leaf_impulse = std::array::from_fn(|row| {
            let mut image = RationalInterval::point(offset[row].clone());
            for column in 0..3 {
                image = image.add(&separator_box[column].scale(&separator_gain[row][column]));
            }
            image
        });

        let normal_impulse_lower = leaf_impulse[0].lower.clone();
        if normal_impulse_lower <= zero {
            return Ok(None);
        }

        let friction_radius_lower = friction.clone() * normal_impulse_lower.clone();
        let tangent_1_absolute_upper = leaf_impulse[1].maximum_absolute_value();
        let tangent_2_absolute_upper = leaf_impulse[2].maximum_absolute_value();
        let squared_disk_margin_lower = friction_radius_lower.clone()
            * friction_radius_lower
            - tangent_1_absolute_upper.clone() * tangent_1_absolute_upper
            - tangent_2_absolute_upper.clone() * tangent_2_absolute_upper;

        if squared_disk_margin_lower <= zero {
            return Ok(None);
        }

        Ok(Some(StrictStickingBoxCertificate {
            leaf_impulse,
            normal_impulse_lower,
            squared_disk_margin_lower,
        }))
    }

    fn exact_leaf_affine_fixture() -> (
        [BigRational; 3],
        [[BigRational; 3]; 3],
        BigRational,
    ) {
        (
            [rational(3, 4), rational(-1, 8), rational(0, 1)],
            [
                [rational(1, 4), rational(0, 1), rational(0, 1)],
                [rational(0, 1), rational(-1, 4), rational(0, 1)],
                [rational(0, 1), rational(0, 1), rational(-1, 4)],
            ],
            rational(1, 2),
        )
    }

    #[test]
    fn exact_affine_message_certifies_strict_sticking_over_a_separator_box() {
        let (offset, gain, friction) = exact_leaf_affine_fixture();
        let separator_box = [
            RationalInterval::new(rational(99, 100), rational(101, 100)).unwrap(),
            RationalInterval::new(rational(-51, 100), rational(-49, 100)).unwrap(),
            RationalInterval::new(rational(-1, 100), rational(1, 100)).unwrap(),
        ];

        let certificate =
            certify_strict_sticking_box(&offset, &gain, &friction, &separator_box)
                .unwrap()
                .expect("entire separator box is strictly inside the leaf sticking domain");

        assert_eq!(certificate.normal_impulse_lower, rational(399, 400));
        assert_eq!(certificate.leaf_impulse[0].upper, rational(401, 400));
        assert_eq!(certificate.leaf_impulse[1].lower, rational(-1, 400));
        assert_eq!(certificate.leaf_impulse[1].upper, rational(1, 400));
        assert_eq!(certificate.leaf_impulse[2].lower, rational(-1, 400));
        assert_eq!(certificate.leaf_impulse[2].upper, rational(1, 400));
        assert!(certificate.squared_disk_margin_lower > rational(0, 1));
    }

    #[test]
    fn exact_affine_message_abstains_on_boundary_crossing_and_outside_boxes() {
        let (offset, gain, friction) = exact_leaf_affine_fixture();

        let boundary_crossing_box = [
            RationalInterval::new(rational(99, 100), rational(101, 100)).unwrap(),
            RationalInterval::new(rational(149, 100), rational(151, 100)).unwrap(),
            RationalInterval::point(rational(0, 1)),
        ];
        assert!(
            certify_strict_sticking_box(&offset, &gain, &friction, &boundary_crossing_box)
                .unwrap()
                .is_none()
        );

        let outside_disk_box = [
            RationalInterval::point(rational(1, 1)),
            RationalInterval::new(rational(199, 100), rational(201, 100)).unwrap(),
            RationalInterval::point(rational(0, 1)),
        ];
        assert!(
            certify_strict_sticking_box(&offset, &gain, &friction, &outside_disk_box)
                .unwrap()
                .is_none()
        );
    }

    #[test]
    fn exact_affine_message_rejects_invalid_interval_and_friction_inputs() {
        let (offset, gain, friction) = exact_leaf_affine_fixture();
        assert!(RationalInterval::new(rational(2, 1), rational(1, 1)).is_err());

        let separator_box = [
            RationalInterval::point(rational(1, 1)),
            RationalInterval::point(rational(-1, 2)),
            RationalInterval::point(rational(0, 1)),
        ];
        assert!(
            certify_strict_sticking_box(
                &offset,
                &gain,
                &rational(-1, 2),
                &separator_box
            )
            .is_err()
        );
        assert!(certify_strict_sticking_box(&offset, &gain, &friction, &separator_box)
            .unwrap()
            .is_some());
    }
}
