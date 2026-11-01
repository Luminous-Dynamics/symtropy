// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Independent test-only exact Sturm root counter for the coupled Coulomb quartic.
//!
//! This verifier deliberately does not share the production derivative-partition
//! algorithm. Every f64 coefficient is decoded into its exact dyadic rational;
//! a rational Sturm chain then counts distinct real roots and isolates them in
//! rational intervals. The result certifies the polynomial represented by the
//! input binary64 coefficients, not the unrounded physical parameters.

use crate::coupled_friction_sturm_support::{
    Polynomial, Rational, cauchy_bound, count_open, derivative, evaluate, exact_f64, is_zero,
    isolate_recursive, move_endpoint_off_root, normalize_positive, one_sided_sign, polynomial_add,
    polynomial_multiply, polynomial_scale, remainder, sturm_chain, sturm_isolate_rational, trim,
    variations,
};
use super::{real_polynomial_roots, sliding_quartic};
use nalgebra::{Matrix3, SVector};
use num_bigint::BigInt;
use num_traits::{One, Signed, ToPrimitive, Zero};

/// Returns disjoint rational intervals for the exact dyadic values of f64
/// coefficients, not for the decimal text used to display those values.
fn sturm_isolate_f64(
    coefficients_ascending: &[f64],
) -> Result<Vec<(Rational, Rational)>, &'static str> {
    let poly = coefficients_ascending
        .iter()
        .map(|coefficient| exact_f64(*coefficient))
        .collect::<Result<Polynomial, _>>()?;
    sturm_isolate_rational(poly)
}

/// Check the projective half-angle point t = infinity (theta = pi)
/// directly from exact dyadic contact inputs and the unreduced sliding KKT
/// equations. The quartic's leading coefficient is the angular equation at
/// this projective point; the impulse checks below prevent a spurious angular
/// direction from being reported as an admissible contact solution.
fn exact_projective_pi_candidate(
    w: &Matrix3<f64>,
    q: &SVector<f64, 3>,
    mu: f64,
) -> Result<bool, &'static str> {
    let a = exact_f64(w[(0, 0)])?;
    let d1 = exact_f64(w[(0, 1)])?;
    let d2 = exact_f64(w[(0, 2)])?;
    let qn = exact_f64(q[0])?;
    let q1 = exact_f64(q[1])?;
    let q2 = exact_f64(q[2])?;
    let mu = exact_f64(mu)?;
    let a11 = exact_f64(w[(1, 1)])?;
    let a12 = exact_f64(w[(1, 2)])?;

    if mu <= Rational::zero() {
        return Ok(false);
    }
    let denominator = a + mu.clone() * d1.clone();
    if denominator.is_zero() {
        return Ok(false);
    }
    let lambda = -qn / denominator;
    if lambda <= Rational::zero() {
        return Ok(false);
    }
    let tangent_one = mu.clone() * lambda.clone();
    let post_tangent_two = q2 + lambda.clone() * d2 + tangent_one.clone() * a12;
    if !post_tangent_two.is_zero() {
        return Ok(false);
    }
    let post_tangent_one = q1 + lambda.clone() * d1 + tangent_one.clone() * a11;
    let disk_multiplier = -post_tangent_one / tangent_one.clone();
    if disk_multiplier < Rational::zero() {
        return Ok(false);
    }

    // Independently evaluate the full impulse kinetic-energy change:
    // Delta K = p^T q + 1/2 p^T W p, with p=(lambda, mu*lambda, 0).
    let wp_normal = a.clone() * lambda.clone() + d1.clone() * tangent_one.clone();
    let wp_tangent_one = d1 * lambda.clone() + a11 * tangent_one.clone();
    let energy_linear = lambda.clone() * qn + tangent_one.clone() * q1;
    let energy_quadratic = lambda.clone() * wp_normal + tangent_one.clone() * wp_tangent_one;
    let energy_delta = energy_linear + energy_quadratic / Rational::from_integer(BigInt::from(2));
    Ok(energy_delta <= Rational::zero())
}

/// Independently derive the half-angle quartic from exact dyadic W, q and mu.
/// This expands the vector equation in polynomial form rather than copying the
/// production's closed-form coefficient array.
fn exact_sliding_quartic_from_inputs(
    w: &Matrix3<f64>,
    q: &SVector<f64, 3>,
    mu: f64,
) -> Result<Polynomial, &'static str> {
    let a = exact_f64(w[(0, 0)])?;
    let d1 = exact_f64(w[(0, 1)])?;
    let d2 = exact_f64(w[(0, 2)])?;
    let qn = exact_f64(q[0])?;
    let q1 = exact_f64(q[1])?;
    let q2 = exact_f64(q[2])?;
    let mu = exact_f64(mu)?;
    let a11 = exact_f64(w[(1, 1)])?;
    let a12 = exact_f64(w[(1, 2)])?;
    let a22 = exact_f64(w[(2, 2)])?;

    // s numerator = (1-t^2, 2t), s_perp numerator = (-2t, 1-t^2).
    let one_plus_t2 = vec![Rational::one(), Rational::zero(), Rational::one()];
    let one_minus_t2 = vec![Rational::one(), Rational::zero(), -Rational::one()];
    let two_t = vec![Rational::zero(), Rational::from_integer(BigInt::from(2))];
    let negative_one = -Rational::one();
    let negative_mu = -mu.clone();
    let minus_two_t = polynomial_scale(&two_t, &negative_one);

    let d_numerator = polynomial_add(
        &polynomial_scale(&one_plus_t2, &a),
        &polynomial_scale(
            &polynomial_add(
                &polynomial_scale(&one_minus_t2, &d1),
                &polynomial_scale(&two_t, &d2),
            ),
            &negative_mu,
        ),
    );
    let q_tangent_perp = polynomial_add(
        &polynomial_scale(&minus_two_t, &q1),
        &polynomial_scale(&one_minus_t2, &q2),
    );
    let d_perp = polynomial_add(
        &polynomial_scale(&minus_two_t, &d1),
        &polynomial_scale(&one_minus_t2, &d2),
    );
    let a_s_first = polynomial_add(
        &polynomial_scale(&one_minus_t2, &a11),
        &polynomial_scale(&two_t, &a12),
    );
    let a_s_second = polynomial_add(
        &polynomial_scale(&one_minus_t2, &a12),
        &polynomial_scale(&two_t, &a22),
    );
    let perp_dot_a_s = polynomial_add(
        &polynomial_multiply(&minus_two_t, &a_s_first),
        &polynomial_multiply(&one_minus_t2, &a_s_second),
    );

    let first = polynomial_multiply(&d_numerator, &q_tangent_perp);
    let negative_qn = -qn.clone();
    let second = polynomial_scale(&polynomial_multiply(&d_perp, &one_plus_t2), &negative_qn);
    let third_factor = mu * qn;
    let third = polynomial_scale(&perp_dot_a_s, &third_factor);
    Ok(polynomial_add(&polynomial_add(&first, &second), &third))
}

fn polynomial_from_roots(roots: &[Rational]) -> Polynomial {
    let mut polynomial = vec![Rational::one()];
    for root in roots {
        let mut next = vec![Rational::zero(); polynomial.len() + 1];
        for (power, coefficient) in polynomial.iter().enumerate() {
            next[power] = next[power].clone() - coefficient.clone() * root.clone();
            next[power + 1] = next[power + 1].clone() + coefficient.clone();
        }
        polynomial = next;
    }
    polynomial
}

fn rationals_to_f64(poly: &[Rational]) -> Vec<f64> {
    poly.iter()
        .map(|coefficient| coefficient.to_f64().unwrap())
        .collect()
}

fn assert_f64_coefficients_match_exact(rounded: &[f64], exact: &[Rational]) {
    let rounded = rounded
        .iter()
        .map(|coefficient| exact_f64(*coefficient).unwrap())
        .collect::<Polynomial>();
    let scale = exact
        .iter()
        .fold(Rational::one(), |sum, coefficient| sum + coefficient.abs());
    let tolerance = exact_f64(1.0e-13).unwrap();
    for index in 0..rounded.len().max(exact.len()) {
        let rounded_coefficient = rounded.get(index).cloned().unwrap_or_else(Rational::zero);
        let exact_coefficient = exact.get(index).cloned().unwrap_or_else(Rational::zero);
        let difference = (rounded_coefficient - exact_coefficient).abs();
        assert!(
            difference <= tolerance.clone() * scale.clone(),
            "rounded quartic coefficient {index} differs from exact dyadic derivation"
        );
    }
}

fn assert_production_matches_sturm(coefficients: &[f64], tolerance: f64) {
    let certified = sturm_isolate_f64(coefficients).expect("exact Sturm isolation");
    let numerical = real_polynomial_roots(coefficients, tolerance)
        .expect("production root isolation should be resolved on this fixture");
    assert_eq!(
        numerical.len(),
        certified.len(),
        "independent root count mismatch; polynomial={coefficients:?}; numerical={numerical:?}"
    );
    for interval in &certified {
        let midpoint = ((&interval.0 + &interval.1) / Rational::from_integer(BigInt::from(2)))
            .to_f64()
            .expect("root interval midpoint must fit in f64");
        let nearest = numerical
            .iter()
            .map(|root| (root - midpoint).abs())
            .fold(f64::INFINITY, f64::min);
        assert!(
            nearest <= 1.0e-7 * (1.0 + midpoint.abs()),
            "production root missed certified interval near {midpoint}; roots={numerical:?}"
        );
    }
}

#[test]
fn exact_f64_decoder_round_trips_binary64_edge_values() {
    let values = [
        0.0,
        -0.0,
        0.1,
        -3.25,
        f64::from_bits(1),
        f64::MIN_POSITIVE,
        f64::MAX,
        -f64::MAX,
    ];
    for value in values {
        let exact = exact_f64(value).unwrap();
        assert_eq!(exact.to_f64().unwrap(), value);
    }
}

#[test]
fn sturm_root_count_is_invariant_under_binary_power_and_sign_scaling() {
    let roots = [-3_i64, -1, 2, 5]
        .into_iter()
        .map(|value| Rational::from_integer(BigInt::from(value)))
        .collect::<Vec<_>>();
    let coefficients = rationals_to_f64(&polynomial_from_roots(&roots));
    let expected = sturm_isolate_f64(&coefficients).unwrap().len();
    for factor in [2.0_f64.powi(-600), -2.0_f64.powi(-600), 2.0_f64.powi(500)] {
        let scaled = coefficients
            .iter()
            .map(|coefficient| coefficient * factor)
            .collect::<Vec<_>>();
        let intervals = sturm_isolate_f64(&scaled).unwrap();
        assert_eq!(intervals.len(), expected);
        assert_production_matches_sturm(&scaled, 1.0e-12);
    }
}

#[test]
fn cauchy_bound_is_strict_and_endpoints_are_not_roots() {
    let roots = [-3_i64, -1, 2, 5]
        .into_iter()
        .map(|value| Rational::from_integer(BigInt::from(value)))
        .collect::<Vec<_>>();
    let polynomial = polynomial_from_roots(&roots);
    let bound = cauchy_bound(&polynomial).unwrap();
    assert!(!evaluate(&polynomial, &bound).is_zero());
    assert!(!evaluate(&polynomial, &-bound).is_zero());
    for root in roots {
        assert!(root.abs() < bound);
    }
}

#[test]
fn sturm_counts_four_simple_roots_and_production_matches() {
    let roots = [-3_i64, -1, 2, 5]
        .into_iter()
        .map(|value| Rational::from_integer(BigInt::from(value)))
        .collect::<Vec<_>>();
    let coefficients = rationals_to_f64(&polynomial_from_roots(&roots));
    let intervals = sturm_isolate_f64(&coefficients).unwrap();
    assert_eq!(intervals.len(), 4);
    assert_production_matches_sturm(&coefficients, 1.0e-12);
}

#[test]
fn sturm_counts_distinct_roots_for_repeated_factors() {
    let roots = [-2_i64, -2, 1, 3]
        .into_iter()
        .map(|value| Rational::from_integer(BigInt::from(value)))
        .collect::<Vec<_>>();
    let coefficients = rationals_to_f64(&polynomial_from_roots(&roots));
    let intervals = sturm_isolate_f64(&coefficients).unwrap();
    assert_eq!(intervals.len(), 3);
    // The exact oracle resolves the distinct roots. The floating candidate
    // generator refuses to promote a stationary root without exact proof.
    assert!(real_polynomial_roots(&coefficients, 1.0e-12).is_err());
}

#[test]
fn sturm_counts_a_root_exactly_on_the_initial_midpoint() {
    let roots = [-2_i64, 0, 0, 3]
        .into_iter()
        .map(|value| Rational::from_integer(BigInt::from(value)))
        .collect::<Vec<_>>();
    let coefficients = rationals_to_f64(&polynomial_from_roots(&roots));
    let intervals = sturm_isolate_f64(&coefficients).unwrap();
    assert_eq!(intervals.len(), 3);
    assert!(
        intervals
            .iter()
            .any(|(lower, upper)| lower.is_zero() && upper.is_zero())
    );
    assert!(real_polynomial_roots(&coefficients, 1.0e-12).is_err());
}

#[test]
fn sturm_resolves_close_distinct_roots_without_merging_them() {
    let delta = Rational::new(BigInt::one(), BigInt::one() << 23_usize);
    let half = Rational::new(BigInt::one(), BigInt::from(2));
    let roots = vec![
        Rational::from_integer(BigInt::from(-2)),
        half.clone() - &delta,
        half + delta,
        Rational::from_integer(BigInt::from(3)),
    ];
    let coefficients = rationals_to_f64(&polynomial_from_roots(&roots));
    let intervals = sturm_isolate_f64(&coefficients).unwrap();
    assert_eq!(intervals.len(), 4);
    // The floating-point production solver must refuse an ambiguous near-
    // double stationary point rather than inventing one root or missing two.
    assert!(real_polynomial_roots(&coefficients, 1.0e-12).is_err());
}

#[test]
fn sturm_handles_no_real_roots_and_degree_loss() {
    let no_real_roots = vec![1.0, 1.0, 1.0, 0.0, 1.0];
    assert!(sturm_isolate_f64(&no_real_roots).unwrap().is_empty());
    assert!(
        real_polynomial_roots(&no_real_roots, 1.0e-12)
            .unwrap()
            .is_empty()
    );

    let degree_three = vec![-6.0, 11.0, -6.0, 1.0, 0.0];
    let intervals = sturm_isolate_f64(&degree_three).unwrap();
    assert_eq!(intervals.len(), 3);
    assert_production_matches_sturm(&degree_three, 1.0e-12);
}

#[test]
fn exact_dyadic_contact_inputs_rederive_the_production_quartic() {
    let w = Matrix3::new(3.0, 0.4, -0.2, 0.4, 2.0, 0.5, -0.2, 0.5, 1.5);
    let q = SVector::new(-0.8, -2.5, 0.9);
    let mu = 0.37;
    let rounded = sliding_quartic(&w, &q, mu);
    let exact = exact_sliding_quartic_from_inputs(&w, &q, mu).unwrap();
    assert_f64_coefficients_match_exact(&rounded, &exact);
    let exact_intervals = sturm_isolate_rational(exact).unwrap();
    let rounded_intervals = sturm_isolate_f64(&rounded).unwrap();
    assert_eq!(exact_intervals.len(), rounded_intervals.len());
}

#[test]
fn exact_projective_theta_pi_root_is_counted_and_original_sliding_law_checked() {
    let w = Matrix3::new(2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    let q = SVector::new(-1.0, -3.0, 0.0);
    let mu = 0.2;
    let exact = exact_sliding_quartic_from_inputs(&w, &q, mu).unwrap();
    assert!(
        exact.len() < 5,
        "theta=pi is the missing leading projective root"
    );
    let finite_roots = sturm_isolate_rational(exact.clone()).unwrap();
    assert_eq!(finite_roots.len(), 1);
    assert!(exact_projective_pi_candidate(&w, &q, mu).unwrap());
    assert_eq!(finite_roots.len() + 1, 2);

    let outcome = super::solve_coupled_signorini_coulomb(&w, &q, mu, 1.0e-12).unwrap();
    assert!(
        outcome
            .sliding_angles
            .iter()
            .any(|angle| { (*angle - std::f64::consts::PI).abs() <= 1.0e-10 }),
        "production path must explicitly retain the projective theta=pi candidate"
    );
    assert!(
        outcome.candidates.iter().any(|candidate| {
            candidate
                .sliding_angle
                .is_some_and(|angle| (angle - std::f64::consts::PI).abs() <= 1.0e-10)
        }),
        "the exact projective direction must pass the unreduced impulse/KKT/energy checks"
    );
}

#[test]
fn exact_projective_theta_pi_direction_is_not_promoted_when_kkt_fails() {
    let w = Matrix3::new(2.0, 0.0, 0.0, 0.0, 1.0, 0.0, 0.0, 0.0, 1.0);
    let q = SVector::new(-1.0, 3.0, 0.0);
    let mu = 0.2;
    let exact = exact_sliding_quartic_from_inputs(&w, &q, mu).unwrap();
    assert!(exact.len() < 5);
    assert!(!exact_projective_pi_candidate(&w, &q, mu).unwrap());
}

#[test]
fn sturm_cross_checks_contact_quartics_over_a_deterministic_spd_panel() {
    let mut checked = 0_usize;
    let mut inconclusive = 0_usize;
    for seed in 1..=128_u64 {
        let mut x = [[0.0_f64; 3]; 3];
        for i in 0..3 {
            for j in 0..3 {
                let k = (seed * (7 + (i * 3 + j) as u64 * 11)) % 101;
                x[i][j] = (k as f64 / 37.0) - 1.2;
            }
        }
        let m = Matrix3::new(
            x[0][0], x[0][1], x[0][2], x[1][0], x[1][1], x[1][2], x[2][0], x[2][1], x[2][2],
        );
        let w = m.transpose() * m + Matrix3::identity() * 0.3;
        let q = SVector::new(
            -0.2 - (seed % 13) as f64 / 7.0,
            (seed % 17) as f64 / 5.0 - 1.3,
            (seed % 19) as f64 / 6.0 - 1.1,
        );
        let mu = 0.05 + (seed % 11) as f64 * 0.21;
        let coefficients = sliding_quartic(&w, &q, mu);
        let exact_polynomial =
            exact_sliding_quartic_from_inputs(&w, &q, mu).unwrap_or_else(|error| {
                panic!("seed={seed}: exact quartic construction failed: {error}")
            });
        assert_f64_coefficients_match_exact(&coefficients, &exact_polynomial);
        let exact_intervals = sturm_isolate_rational(exact_polynomial)
            .unwrap_or_else(|error| panic!("seed={seed}: exact Sturm oracle failed: {error}"));
        let rounded_intervals = sturm_isolate_f64(&coefficients).unwrap_or_else(|error| {
            panic!("seed={seed}: rounded-polynomial Sturm oracle failed: {error}")
        });
        assert_eq!(
            rounded_intervals.len(),
            exact_intervals.len(),
            "seed={seed}: coefficient rounding changed the distinct-root count"
        );
        match real_polynomial_roots(&coefficients, 1.0e-10) {
            Ok(numerical) => {
                assert_eq!(
                    numerical.len(),
                    exact_intervals.len(),
                    "seed={seed}: exact={}, numerical={numerical:?}, coefficients={coefficients:?}",
                    exact_intervals.len()
                );
                for interval in &exact_intervals {
                    let midpoint = ((&interval.0 + &interval.1)
                        / Rational::from_integer(BigInt::from(2)))
                    .to_f64()
                    .expect("midpoint in f64 range");
                    let nearest = numerical
                        .iter()
                        .map(|root| (root - midpoint).abs())
                        .fold(f64::INFINITY, f64::min);
                    assert!(
                        nearest <= 1.0e-6 * (1.0 + midpoint.abs()),
                        "seed={seed}: no numeric root near certified interval {midpoint}"
                    );
                }
                checked += 1;
            }
            Err(()) => inconclusive += 1,
        }
    }
    assert!(
        checked >= 120,
        "too many inconclusive roots: {inconclusive}"
    );
}

#[cfg(test)]
#[path = "coupled_friction_multicontact_sturm.rs"]
mod multicontact;
