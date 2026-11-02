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

/// A leaf restricted to the strict separating regime p_leaf = 0.
#[derive(Clone, Debug)]
struct SeparatingLeafMessage {
    /// B in the block partition W = [[C, B], [B^T, A]].
    separator_coupling: Matrix3<f64>,
    free_velocity: SVector<f64, 3>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SeparatingLeafClassification {
    /// p_n = 0 and u_n is strictly positive beyond the declared tolerance.
    StrictlySeparating,
    /// p_n = 0 and u_n is zero or indistinguishable from zero at this tolerance.
    /// This is a boundary stratum, not a strict separating branch.
    BoundaryWithinTolerance,
    /// p_n = 0 but u_n is strictly negative: this separating regime is invalid.
    SignoriniViolation,
}

/// A conservative interval whose endpoints enclose the exact arithmetic
/// result for the represented binary64 inputs.
#[derive(Clone, Copy, Debug, PartialEq)]
struct OutwardInterval {
    lower: f64,
    upper: f64,
}

impl OutwardInterval {
    fn point(value: f64) -> Result<Self, &'static str> {
        if !value.is_finite() {
            return Err("interval point must be finite");
        }
        Ok(Self {
            lower: value,
            upper: value,
        })
    }

    fn from_bounds(lower: f64, upper: f64) -> Result<Self, &'static str> {
        if !lower.is_finite() || !upper.is_finite() || lower > upper {
            return Err("interval bounds must be finite and ordered");
        }
        Ok(Self { lower, upper })
    }

    fn outward_add(self, other: Self) -> Result<Self, &'static str> {
        let lower = self.lower + other.lower;
        let upper = self.upper + other.upper;
        if !lower.is_finite() || !upper.is_finite() {
            return Err("interval addition overflowed");
        }
        let enclosure = Self {
            lower: lower.next_down(),
            upper: upper.next_up(),
        };
        if !enclosure.lower.is_finite() || !enclosure.upper.is_finite() {
            return Err("outward interval addition reached a non-finite endpoint");
        }
        Ok(enclosure)
    }

    fn outward_mul(self, other: Self) -> Result<Self, &'static str> {
        let products = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        if !products.iter().all(|value| value.is_finite()) {
            return Err("interval multiplication overflowed or became non-finite");
        }
        let minimum = products.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        let enclosure = Self {
            lower: minimum.next_down(),
            upper: maximum.next_up(),
        };
        if !enclosure.lower.is_finite() || !enclosure.upper.is_finite() {
            return Err("outward interval multiplication reached a non-finite endpoint");
        }
        Ok(enclosure)
    }
}

#[derive(Clone, Debug)]
struct SeparatorImpulseBox {
    lower: SVector<f64, 3>,
    upper: SVector<f64, 3>,
}

impl SeparatorImpulseBox {
    fn new(lower: SVector<f64, 3>, upper: SVector<f64, 3>) -> Result<Self, &'static str> {
        for axis in 0..3 {
            OutwardInterval::from_bounds(lower[axis], upper[axis])?;
        }
        Ok(Self { lower, upper })
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SeparatingBoxClassification {
    StrictlySeparating,
    SignoriniViolation,
    BoundaryOrInconclusive,
}

#[derive(Clone, Debug)]
struct SeparatingBoxResponse {
    impulse: SVector<f64, 3>,
    post_velocity_bounds: [OutwardInterval; 3],
    classification: SeparatingBoxClassification,
}

#[derive(Clone, Debug)]
struct SeparatingLeafResponse {
    impulse: SVector<f64, 3>,
    post_velocity: SVector<f64, 3>,
    classification: SeparatingLeafClassification,
    normal_velocity_margin: f64,
}

impl SeparatingLeafMessage {
    /// A strict separating leaf has p_leaf = 0, so it contributes no
    /// separator impulse, no Schur fill, and no free-velocity correction.
    fn schur_terms(&self) -> Result<(Matrix3<f64>, SVector<f64, 3>), &'static str> {
        if !self
            .separator_coupling
            .iter()
            .all(|value| value.is_finite())
            || !self.free_velocity.iter().all(|value| value.is_finite())
        {
            return Err("separating leaf message contains invalid or non-finite data");
        }
        Ok((Matrix3::zeros(), SVector::<f64, 3>::zeros()))
    }

    /// Evaluate the separating response at one separator impulse.
    ///
    /// The local post-contact velocity is u_L = q_L + B^T p_S because
    /// p_L=0. Its normal component determines whether this regime is strict,
    /// boundary-only, or violates Signorini. Tangential velocity is retained
    /// for the original-system replay, but the zero-radius Coulomb disk forces
    /// the tangential impulse to zero.
    /// Enclose the entire affine response over an axis-aligned separator box.
    ///
    /// Each multiply and add widens its endpoints by one adjacent binary64
    /// value. A strict regime is reported only when the complete normal
    /// velocity interval has a strict sign. If zero is included, the result
    /// remains boundary/inconclusive; midpoint signs are never used.
    fn response_over_box(
        &self,
        separator_box: &SeparatorImpulseBox,
    ) -> Result<SeparatingBoxResponse, &'static str> {
        self.schur_terms()?;
        let mut post_velocity_bounds = [OutwardInterval::point(0.0)?; 3];
        for component in 0..3 {
            let mut velocity = OutwardInterval::point(self.free_velocity[component])?;
            for separator_axis in 0..3 {
                let coefficient =
                    OutwardInterval::point(self.separator_coupling[(separator_axis, component)])?;
                let separator_coordinate = OutwardInterval::from_bounds(
                    separator_box.lower[separator_axis],
                    separator_box.upper[separator_axis],
                )?;
                velocity = velocity.outward_add(coefficient.outward_mul(separator_coordinate)?)?;
            }
            post_velocity_bounds[component] = velocity;
        }

        let normal = post_velocity_bounds[0];
        let classification = if normal.lower > 0.0 {
            SeparatingBoxClassification::StrictlySeparating
        } else if normal.upper < 0.0 {
            SeparatingBoxClassification::SignoriniViolation
        } else {
            SeparatingBoxClassification::BoundaryOrInconclusive
        };

        Ok(SeparatingBoxResponse {
            impulse: SVector::<f64, 3>::zeros(),
            post_velocity_bounds,
            classification,
        })
    }

    fn response_at(
        &self,
        separator_impulse: &SVector<f64, 3>,
        tolerance: f64,
    ) -> Result<SeparatingLeafResponse, &'static str> {
        if !tolerance.is_finite()
            || tolerance <= 0.0
            || !separator_impulse.iter().all(|value| value.is_finite())
        {
            return Err("separating leaf response received invalid or non-finite input");
        }
        let (operator_contribution, free_velocity_contribution) = self.schur_terms()?;
        debug_assert_eq!(operator_contribution, Matrix3::zeros());
        debug_assert_eq!(free_velocity_contribution, SVector::<f64, 3>::zeros());

        let post_velocity =
            self.free_velocity + self.separator_coupling.transpose() * separator_impulse;
        let velocity_scale = (self.free_velocity.norm()
            + self.separator_coupling.norm() * separator_impulse.norm())
        .max(f64::MIN_POSITIVE);
        let normal_tolerance = tolerance * velocity_scale * 500.0;
        let normal_velocity = post_velocity[0];
        let classification = if normal_velocity > normal_tolerance {
            SeparatingLeafClassification::StrictlySeparating
        } else if normal_velocity < -normal_tolerance {
            SeparatingLeafClassification::SignoriniViolation
        } else {
            SeparatingLeafClassification::BoundaryWithinTolerance
        };

        Ok(SeparatingLeafResponse {
            impulse: SVector::<f64, 3>::zeros(),
            post_velocity,
            classification,
            normal_velocity_margin: normal_velocity,
        })
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

fn star_contact_jacobian() -> Jacobian9x12 {
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
    jacobian
}

fn star_body_generated_fixture(first_leaf_tangent: f64) -> (Matrix9, Vector9) {
    // Four translational bodies A/B/C/D, each with mass 2 I. The three
    // contacts are (B,C), (A,B), and (C,D), so contact 0 is the separator
    // and contacts 1/2 are leaves. Shared-body Jacobian support creates the
    // two separator/leaf blocks; the leaf-to-leaf block is structurally zero.
    let jacobian = star_contact_jacobian();
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
fn separating_leaf_message_preserves_strict_boundary_and_signorini_strata() {
    let message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.25, 0.25)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.125, -0.25),
    };
    let separator_inside = SVector::<f64, 3>::new(1.0, -0.5, 0.0);
    let strict = message.response_at(&separator_inside, TOLERANCE).unwrap();
    assert_eq!(
        strict.classification,
        SeparatingLeafClassification::StrictlySeparating
    );
    assert!((strict.normal_velocity_margin - 0.5).abs() < 1.0e-12);
    assert_eq!(strict.impulse, SVector::<f64, 3>::zeros());
    assert!((strict.post_velocity[1] - 0.0).abs() < 1.0e-12);
    assert!((strict.post_velocity[2] + 0.25).abs() < 1.0e-12);

    // Moving the separator normal impulse to 2 reaches u_n=0 exactly.
    // The contact remains complementarity-admissible, but is not a strict
    // separating branch and must be retained as a boundary stratum.
    let separator_boundary = SVector::<f64, 3>::new(2.0, -0.5, 0.0);
    let boundary = message.response_at(&separator_boundary, TOLERANCE).unwrap();
    assert_eq!(
        boundary.classification,
        SeparatingLeafClassification::BoundaryWithinTolerance
    );
    assert!(boundary.normal_velocity_margin.abs() <= 1.0e-12);
    assert_eq!(boundary.impulse, SVector::<f64, 3>::zeros());

    // Beyond the boundary, p_n=0 yields a strict Signorini violation.
    let separator_outside = SVector::<f64, 3>::new(3.0, -0.5, 0.0);
    let outside = message.response_at(&separator_outside, TOLERANCE).unwrap();
    assert_eq!(
        outside.classification,
        SeparatingLeafClassification::SignoriniViolation
    );
    assert!(outside.normal_velocity_margin < -0.49);
    assert_eq!(outside.impulse, SVector::<f64, 3>::zeros());

    let (schur_operator, schur_free_velocity) = message.schur_terms().unwrap();
    assert_eq!(schur_operator, Matrix3::zeros());
    assert_eq!(schur_free_velocity, SVector::<f64, 3>::zeros());

    assert!(message.response_at(&separator_inside, 0.0).is_err());
    assert!(
        message
            .response_at(&SVector::<f64, 3>::new(f64::NAN, 0.0, 0.0), TOLERANCE)
            .is_err()
    );
}

#[test]
fn separating_leaf_interval_messages_certify_only_boxes_with_a_strict_sign() {
    let message = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(-0.5, 0.25, 0.25)),
        free_velocity: SVector::<f64, 3>::new(1.0, 0.125, -0.25),
    };

    // Exact dyadic reference: u_n = 1 - (1/2) p_S,n.
    // For p_S,n in [1,5/4], its exact range is [3/8,1/2].
    let strict_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0, -0.5, -0.5),
        SVector::<f64, 3>::new(1.25, 0.5, 0.5),
    )
    .unwrap();
    let strict = message.response_over_box(&strict_box).unwrap();
    assert_eq!(
        strict.classification,
        SeparatingBoxClassification::StrictlySeparating
    );
    assert!(strict.post_velocity_bounds[0].lower <= 0.375);
    assert!(strict.post_velocity_bounds[0].upper >= 0.5);
    assert_eq!(strict.impulse, SVector::<f64, 3>::zeros());
    assert!(strict.post_velocity_bounds[1].lower <= 0.125);
    assert!(strict.post_velocity_bounds[1].upper >= 0.375);
    assert!(strict.post_velocity_bounds[2].lower <= -0.375);
    assert!(strict.post_velocity_bounds[2].upper >= -0.125);

    // Exact dyadic reference: for p_S,n in [3,13/4], u_n is [-5/8,-1/2].
    let violating_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(3.0, -0.5, -0.5),
        SVector::<f64, 3>::new(3.25, 0.5, 0.5),
    )
    .unwrap();
    let violating = message.response_over_box(&violating_box).unwrap();
    assert_eq!(
        violating.classification,
        SeparatingBoxClassification::SignoriniViolation
    );
    assert!(violating.post_velocity_bounds[0].lower <= -0.625);
    assert!(violating.post_velocity_bounds[0].upper >= -0.5);

    // This box straddles the exact transition hyperplane p_S,n=2.
    let crossing_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.875, -0.5, -0.5),
        SVector::<f64, 3>::new(2.125, 0.5, 0.5),
    )
    .unwrap();
    let crossing = message.response_over_box(&crossing_box).unwrap();
    assert_eq!(
        crossing.classification,
        SeparatingBoxClassification::BoundaryOrInconclusive
    );
    assert!(crossing.post_velocity_bounds[0].lower <= 0.0);
    assert!(crossing.post_velocity_bounds[0].upper >= 0.0);

    // A zero-radius box exactly on the transition is still not strict:
    // outward rounding encloses the exact zero from both sides.
    let boundary_box = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(2.0, 0.0, 0.0),
        SVector::<f64, 3>::new(2.0, 0.0, 0.0),
    )
    .unwrap();
    let boundary = message.response_over_box(&boundary_box).unwrap();
    assert_eq!(
        boundary.classification,
        SeparatingBoxClassification::BoundaryOrInconclusive
    );
    assert!(boundary.post_velocity_bounds[0].lower <= 0.0);
    assert!(boundary.post_velocity_bounds[0].upper >= 0.0);
}

#[test]
fn separating_leaf_interval_messages_reject_malformed_and_overflowing_boxes() {
    assert!(
        SeparatorImpulseBox::new(
            SVector::<f64, 3>::new(1.0, 0.0, 0.0),
            SVector::<f64, 3>::new(0.0, 0.0, 0.0),
        )
        .is_err()
    );
    assert!(
        SeparatorImpulseBox::new(
            SVector::<f64, 3>::new(f64::NAN, 0.0, 0.0),
            SVector::<f64, 3>::new(1.0, 0.0, 0.0),
        )
        .is_err()
    );

    let overflowing = SeparatingLeafMessage {
        separator_coupling: Matrix3::from_diagonal(&SVector::<f64, 3>::new(1.0e308, 0.0, 0.0)),
        free_velocity: SVector::<f64, 3>::zeros(),
    };
    let box_with_large_coordinate = SeparatorImpulseBox::new(
        SVector::<f64, 3>::new(1.0e308, 0.0, 0.0),
        SVector::<f64, 3>::new(1.0e308, 0.0, 0.0),
    )
    .unwrap();
    assert!(
        overflowing
            .response_over_box(&box_with_large_coordinate)
            .is_err()
    );
}

#[test]
fn one_sticking_and_one_separating_leaf_reduce_to_sliding_separator() {
    // Four-body chain A-B-C-D, with v_A=(-1/2,-1/4,0),
    // v_B=(0,0,0), v_C=(0,-1,0), v_D=(-1,-1,0).
    // Derive both W and q from the same body-level J/M/v_free construction.
    let jacobian = star_contact_jacobian();
    let inverse_mass = Matrix12::identity() * 0.5;
    let w = jacobian * inverse_mass * jacobian.transpose();
    let free_body_velocity = Vector12::from_row_slice(&[
        -0.5, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0, -1.0, 0.0, -1.0, -1.0, 0.0,
    ]);
    let q = jacobian * free_body_velocity;

    let block = |row_offset: usize, column_offset: usize| {
        Matrix3::from_fn(|row, column| w[(row + row_offset, column + column_offset)])
    };
    let vector = |offset: usize| SVector::<f64, 3>::new(q[offset], q[offset + 1], q[offset + 2]);
    let separator_block = block(0, 0);

    let sticking = StickingLeafMessage {
        self_operator: block(3, 3),
        separator_coupling: block(0, 3),
        free_velocity: vector(3),
        friction_coefficient: FRICTION,
    };
    let separating = SeparatingLeafMessage {
        separator_coupling: block(0, 6),
        free_velocity: vector(6),
    };

    // The sticking leaf contributes its Schur terms. The separating leaf
    // contributes exactly zero because its impulse is identically zero.
    let (sticking_w, sticking_q) = sticking.schur_terms().unwrap();
    let (separating_w, separating_q) = separating.schur_terms().unwrap();
    let effective_operator = separator_block - sticking_w - separating_w;
    let effective_free_velocity = vector(0) - sticking_q - separating_q;
    assert!((effective_operator - Matrix3::identity() * 0.75).norm() < 1.0e-12);
    assert!((effective_free_velocity - SVector::<f64, 3>::new(-0.25, 0.875, 0.0)).norm() < 1.0e-12);

    let reduced = solve_coupled_signorini_coulomb(
        &effective_operator,
        &effective_free_velocity,
        FRICTION,
        TOLERANCE,
    )
    .unwrap();
    assert_eq!(reduced.candidates.len(), 1);
    let separator = &reduced.candidates[0];
    assert_eq!(separator.regime, CoupledCoulombRegime::Sliding);
    assert!((separator.impulse[0] - (1.0 / 3.0)).abs() < 1.0e-9);
    assert!((separator.impulse[1] + (1.0 / 6.0)).abs() < 1.0e-9);

    let sticking_response = sticking
        .response_at(&separator.impulse, TOLERANCE)
        .unwrap()
        .expect("leaf 1 remains strictly sticking");
    let separating_response = separating
        .response_at(&separator.impulse, TOLERANCE)
        .unwrap();
    assert_eq!(
        separating_response.classification,
        SeparatingLeafClassification::StrictlySeparating
    );
    assert!((separating_response.normal_velocity_margin - (5.0 / 6.0)).abs() < 1.0e-9);

    let impulse = Vector9::from_row_slice(&[
        separator.impulse[0],
        separator.impulse[1],
        separator.impulse[2],
        sticking_response.impulse[0],
        sticking_response.impulse[1],
        sticking_response.impulse[2],
        separating_response.impulse[0],
        separating_response.impulse[1],
        separating_response.impulse[2],
    ]);
    let post_velocity = q + w * impulse;

    // Replay the complete original 9D contact equations and every local law.
    assert!(post_velocity.fixed_rows::<3>(3).norm() < 1.0e-9);
    let body_impulse = jacobian.transpose() * impulse;
    let post_body_velocity = free_body_velocity + inverse_mass * body_impulse;
    let body_reconstructed_velocity = jacobian * post_body_velocity;
    assert!((body_reconstructed_velocity - post_velocity).norm() < 1.0e-12);
    assert!(impulse[3] > 0.0);
    assert!(FRICTION * impulse[3] - impulse.fixed_rows::<2>(4).norm() > 0.15);
    assert!(impulse.fixed_rows::<3>(6).norm() < 1.0e-12);
    assert!(post_velocity[6] > 0.8);
    assert!((post_velocity[7] - (1.0 / 12.0)).abs() < 1.0e-9);
    assert!(post_velocity[8].abs() < 1.0e-9);
    assert!(impulse[7].abs() < 1.0e-12);
    assert!(impulse[8].abs() < 1.0e-12);

    let separator_tau = separator.disk_multiplier;
    assert!(separator_tau > 0.0);
    assert!(post_velocity[0].abs() < 1.0e-9);
    let separator_kkt =
        post_velocity.fixed_rows::<2>(1) + separator_tau * separator.impulse.fixed_rows::<2>(1);
    assert!(separator_kkt.norm() < 1.0e-9);
    assert!(
        (FRICTION * separator.impulse[0] - separator.impulse.fixed_rows::<2>(1).norm()).abs()
            < 1.0e-9
    );

    let energy = impulse.dot(&q) + 0.5 * impulse.dot(&(w * impulse));
    assert!((energy + (1.0 / 3.0)).abs() < 1.0e-9);
    assert!(energy < 0.0);
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
