// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Independent exact-arithmetic Sturm oracle for the coupled Coulomb quartic.
//!
//! This module is test-only. It interprets each binary64 input as its exact
//! dyadic rational value, reconstructs the angular polynomial over Q, removes
//! repeated factors, and counts and isolates distinct finite real roots using
//! a Sturm sequence. It deliberately does not call the runtime solver's
//! floating-point derivative-partition root finder.

use nalgebra::{Matrix3, SVector};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Rational = BigRational;
type Polynomial = Vec<Rational>;

#[derive(Clone, Debug)]
struct RootCertificate {
    finite_real_root_count: usize,
    intervals: Vec<(Rational, Rational)>,
    projective_pi_root: bool,
}

fn exact_binary64(value: f64) -> Option<Rational> {
    if !value.is_finite() {
        return None;
    }
    let bits = value.to_bits();
    let negative = (bits >> 63) != 0;
    let exponent = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);

    if exponent == 0 && fraction == 0 {
        return Some(Rational::zero());
    }

    let (mantissa, exponent2) = if exponent == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, exponent - 1023 - 52)
    };

    let mut numerator = BigInt::from(mantissa);
    if negative {
        numerator = -numerator;
    }
    if exponent2 >= 0 {
        numerator <<= exponent2 as usize;
        Some(Rational::from_integer(numerator))
    } else {
        let denominator = BigInt::one() << (-exponent2) as usize;
        Some(Rational::new(numerator, denominator))
    }
}

fn linear_combination(terms: &[(i64, Rational)]) -> Rational {
    terms.iter().fold(Rational::zero(), |sum, (factor, term)| {
        sum + Rational::from_integer(BigInt::from(*factor)) * term.clone()
    })
}

fn product(a: &Rational, b: &Rational) -> Rational {
    a.clone() * b.clone()
}

// Independent exact reconstruction of the quartic from the angular equation.
// Coefficients are in ascending powers of t = tan(theta / 2).
fn exact_sliding_quartic(
    w: &Matrix3<f64>,
    q: &SVector<f64, 3>,
    mu: f64,
) -> Option<Polynomial> {
    let r = |value: f64| exact_binary64(value);
    let a = r(w[(0, 0)])?;
    let d1 = r(w[(0, 1)])?;
    let d2 = r(w[(0, 2)])?;
    let qn = r(q[0])?;
    let q1 = r(q[1])?;
    let q2 = r(q[2])?;
    let a11 = r(w[(1, 1)])?;
    let a12 = r(w[(1, 2)])?;
    let a22 = r(w[(2, 2)])?;
    let mu = r(mu)?;

    let mu_qn = product(&mu, &qn);
    let d1_mu = product(&d1, &mu);
    let d2_mu = product(&d2, &mu);
    let two = Rational::from_integer(BigInt::from(2_i64));

    let c0 = linear_combination(&[
        (1, product(&mu_qn, &a12)),
        (1, product(&a, &q2)),
        (-1, product(&d2, &qn)),
        (-1, product(&d1_mu, &q2)),
    ]);
    let c1 = two.clone() * linear_combination(&[
        (-1, product(&mu_qn, &a11)),
        (1, product(&mu_qn, &a22)),
        (-1, product(&a, &q1)),
        (1, product(&d1, &qn)),
        (1, product(&d1_mu, &q1)),
        (-1, product(&d2_mu, &q2)),
    ]);
    let c2 = linear_combination(&[
        (-6, product(&mu_qn, &a12)),
        (2, product(&d1_mu, &q2)),
        (4, product(&d2_mu, &q1)),
    ]);
    let c3 = two * linear_combination(&[
        (1, product(&mu_qn, &a11)),
        (-1, product(&mu_qn, &a22)),
        (-1, product(&a, &q1)),
        (1, product(&d1, &qn)),
        (-1, product(&d1_mu, &q1)),
        (1, product(&d2_mu, &q2)),
    ]);
    let c4 = linear_combination(&[
        (1, product(&mu_qn, &a12)),
        (-1, product(&a, &q2)),
        (1, product(&d2, &qn)),
        (-1, product(&d1_mu, &q2)),
    ]);
    Some(vec![c0, c1, c2, c3, c4])
}

fn trim(polynomial: &mut Polynomial) {
    while polynomial.last().is_some_and(|value| value.is_zero()) {
        polynomial.pop();
    }
}

fn evaluate(polynomial: &[Rational], x: &Rational) -> Rational {
    polynomial
        .iter()
        .rev()
        .fold(Rational::zero(), |value, coefficient| {
            value * x.clone() + coefficient.clone()
        })
}

fn derivative(polynomial: &[Rational]) -> Polynomial {
    polynomial
        .iter()
        .enumerate()
        .skip(1)
        .map(|(power, coefficient)| {
            coefficient.clone() * Rational::from_integer(BigInt::from(power))
        })
        .collect()
}

fn divide_with_remainder(
    dividend: &[Rational],
    divisor: &[Rational],
) -> Result<(Polynomial, Polynomial), &'static str> {
    if divisor.is_empty() {
        return Err("zero polynomial divisor");
    }
    let mut remainder = dividend.to_vec();
    trim(&mut remainder);
    if remainder.is_empty() {
        return Ok((Vec::new(), Vec::new()));
    }
    let quotient_len = remainder.len().saturating_sub(divisor.len()) + 1;
    let mut quotient = vec![Rational::zero(); quotient_len];

    while !remainder.is_empty() && remainder.len() >= divisor.len() {
        let shift = remainder.len() - divisor.len();
        let factor = remainder
            .last()
            .ok_or("missing remainder leading coefficient")?
            .clone()
            / divisor
                .last()
                .ok_or("missing divisor leading coefficient")?
                .clone();
        quotient[shift] += factor.clone();
        for (index, coefficient) in divisor.iter().enumerate() {
            let target = shift + index;
            remainder[target] =
                remainder[target].clone() - factor.clone() * coefficient.clone();
        }
        trim(&mut remainder);
    }
    trim(&mut quotient);
    Ok((quotient, remainder))
}

fn monic(mut polynomial: Polynomial) -> Polynomial {
    trim(&mut polynomial);
    if let Some(leading) = polynomial.last().cloned() {
        for coefficient in &mut polynomial {
            *coefficient /= leading.clone();
        }
    }
    polynomial
}

fn polynomial_gcd(
    mut left: Polynomial,
    mut right: Polynomial,
) -> Result<Polynomial, &'static str> {
    trim(&mut left);
    trim(&mut right);
    while !right.is_empty() {
        let (_, remainder) = divide_with_remainder(&left, &right)?;
        left = right;
        right = remainder;
    }
    Ok(monic(left))
}

fn sturm_sequence(polynomial: &[Rational]) -> Result<Vec<Polynomial>, &'static str> {
    let mut source = polynomial.to_vec();
    trim(&mut source);
    if source.is_empty() {
        return Err("zero polynomial has no finite root count");
    }
    if source.len() <= 1 {
        return Ok(vec![source]);
    }

    // Square-free reduction makes the Sturm count a count of distinct roots,
    // not a count weighted by algebraic multiplicity.
    let gcd = polynomial_gcd(source.clone(), derivative(&source))?;
    let (mut square_free, remainder) = divide_with_remainder(&source, &gcd)?;
    if !remainder.is_empty() {
        return Err("non-exact square-free quotient");
    }
    trim(&mut square_free);
    if square_free.len() <= 1 {
        return Ok(vec![square_free]);
    }

    let first_derivative = derivative(&square_free);
    let mut sequence = vec![square_free, first_derivative];
    loop {
        let length = sequence.len();
        let (_, mut remainder) =
            divide_with_remainder(&sequence[length - 2], &sequence[length - 1])?;
        if remainder.is_empty() {
            break;
        }
        for coefficient in &mut remainder {
            *coefficient = -coefficient.clone();
        }
        trim(&mut remainder);
        sequence.push(remainder);
    }
    Ok(sequence)
}

fn sign_of(value: &Rational) -> i8 {
    if value.is_positive() {
        1
    } else if value.is_negative() {
        -1
    } else {
        0
    }
}

fn sign_variations(signs: impl IntoIterator<Item = i8>) -> usize {
    let mut previous = 0_i8;
    let mut variations = 0_usize;
    for sign in signs {
        if sign == 0 {
            continue;
        }
        if previous != 0 && sign != previous {
            variations += 1;
        }
        previous = sign;
    }
    variations
}

fn variations_at(sequence: &[Polynomial], x: &Rational) -> usize {
    sign_variations(sequence.iter().map(|polynomial| sign_of(&evaluate(polynomial, x))))
}

fn variations_at_infinity(sequence: &[Polynomial], negative_infinity: bool) -> usize {
    sign_variations(sequence.iter().map(|polynomial| {
        let leading = polynomial.last().expect("Sturm polynomials are nonzero");
        let mut sign = sign_of(leading);
        if negative_infinity && (polynomial.len() - 1) % 2 == 1 {
            sign = -sign;
        }
        sign
    }))
}

fn roots_between(
    sequence: &[Polynomial],
    source: &[Rational],
    lower: &Rational,
    upper: &Rational,
) -> Result<usize, &'static str> {
    if lower >= upper {
        return Err("root interval endpoints are not ordered");
    }
    if evaluate(source, lower).is_zero() || evaluate(source, upper).is_zero() {
        return Err("root interval endpoint is itself a root");
    }
    variations_at(sequence, lower)
        .checked_sub(variations_at(sequence, upper))
        .ok_or("Sturm variation count increased")
}

fn root_bound(polynomial: &[Rational]) -> Result<Rational, &'static str> {
    if polynomial.len() <= 1 {
        return Ok(Rational::zero());
    }
    let leading = polynomial.last().ok_or("missing leading coefficient")?;
    if leading.is_zero() {
        return Err("untrimmed polynomial");
    }
    let mut maximum = Rational::zero();
    for coefficient in &polynomial[..polynomial.len() - 1] {
        let ratio = coefficient.clone() / leading.clone();
        let magnitude = if ratio.is_negative() { -ratio } else { ratio };
        if magnitude > maximum {
            maximum = magnitude;
        }
    }
    Ok(Rational::one() + maximum)
}

fn split_at_nonroot(
    polynomial: &[Rational],
    lower: &Rational,
    upper: &Rational,
) -> Result<Rational, &'static str> {
    // A nonzero quartic has at most four distinct real roots. Among these six
    // distinct interior rational fractions, at least one is not a root.
    const FRACTIONS: [(i64, i64); 6] = [(1, 2), (1, 3), (2, 3), (1, 4), (3, 4), (1, 5)];
    let width = upper.clone() - lower.clone();
    for (numerator, denominator) in FRACTIONS {
        let fraction = Rational::new(BigInt::from(numerator), BigInt::from(denominator));
        let point = lower.clone() + width.clone() * fraction;
        if !evaluate(polynomial, &point).is_zero() {
            return Ok(point);
        }
    }
    Err("could not find a non-root rational split point")
}

fn isolate_recursive(
    polynomial: &[Rational],
    sequence: &[Polynomial],
    lower: Rational,
    upper: Rational,
    count: usize,
    tolerance: &Rational,
    depth: usize,
    intervals: &mut Vec<(Rational, Rational)>,
) -> Result<(), &'static str> {
    if count == 0 {
        return Ok(());
    }
    if depth > 512 {
        return Err("root isolation depth limit reached");
    }
    let left_magnitude = if lower.is_negative() { -lower.clone() } else { lower.clone() };
    let right_magnitude = if upper.is_negative() { -upper.clone() } else { upper.clone() };
    let magnitude = if left_magnitude > right_magnitude {
        left_magnitude
    } else {
        right_magnitude
    };
    let relative_width = tolerance.clone() * (Rational::one() + magnitude);
    if count == 1 && upper.clone() - lower.clone() <= relative_width {
        intervals.push((lower, upper));
        return Ok(());
    }

    let split = split_at_nonroot(polynomial, &lower, &upper)?;
    let left_count = roots_between(sequence, polynomial, &lower, &split)?;
    let right_count = roots_between(sequence, polynomial, &split, &upper)?;
    if left_count + right_count != count {
        return Err("Sturm subdivision did not preserve the parent root count");
    }
    isolate_recursive(
        polynomial,
        sequence,
        lower,
        split.clone(),
        left_count,
        tolerance,
        depth + 1,
        intervals,
    )?;
    isolate_recursive(
        polynomial,
        sequence,
        split,
        upper,
        right_count,
        tolerance,
        depth + 1,
        intervals,
    )
}

fn certify_polynomial(
    input: &[Rational],
    tolerance: f64,
) -> Result<RootCertificate, &'static str> {
    if input.is_empty() {
        return Err("empty polynomial");
    }
    // In the projective half-angle chart, theta = pi is t = infinity.
    // Its exact angular residual is the quartic's t^4 coefficient.
    let projective_pi_root = input.len() >= 5 && input[4].is_zero();
    let mut polynomial = input.to_vec();
    trim(&mut polynomial);
    if polynomial.is_empty() {
        return Err("identically zero angular polynomial");
    }
    let tolerance = exact_binary64(tolerance).ok_or("invalid isolation tolerance")?;
    if tolerance <= Rational::zero() {
        return Err("non-positive isolation tolerance");
    }

    let sequence = sturm_sequence(&polynomial)?;
    let negative_variations = variations_at_infinity(&sequence, true);
    let positive_variations = variations_at_infinity(&sequence, false);
    let finite_real_root_count = negative_variations
        .checked_sub(positive_variations)
        .ok_or("Sturm count at infinity was negative")?;

    let mut intervals = Vec::new();
    if finite_real_root_count > 0 {
        let bound = root_bound(&polynomial)?;
        let lower = -bound.clone();
        let upper = bound;
        if evaluate(&polynomial, &lower).is_zero() || evaluate(&polynomial, &upper).is_zero() {
            return Err("Cauchy bound endpoint unexpectedly became a root");
        }
        let bounded_count = roots_between(&sequence, &polynomial, &lower, &upper)?;
        if bounded_count != finite_real_root_count {
            return Err("Cauchy interval failed to enclose every finite real root");
        }
        isolate_recursive(
            &polynomial,
            &sequence,
            lower,
            upper,
            bounded_count,
            &tolerance,
            0,
            &mut intervals,
        )?;
    }
    intervals.sort_by(|left, right| left.0.cmp(&right.0));
    if intervals.len() != finite_real_root_count {
        return Err("isolating interval count differs from Sturm count");
    }
    for (lower, upper) in &intervals {
        if roots_between(&sequence, &polynomial, lower, upper)? != 1 {
            return Err("an isolating interval does not contain exactly one root");
        }
    }

    Ok(RootCertificate {
        finite_real_root_count,
        intervals,
        projective_pi_root,
    })
}

fn integer_polynomial(coefficients: &[i64]) -> Polynomial {
    coefficients
        .iter()
        .map(|value| Rational::from_integer(BigInt::from(*value)))
        .collect()
}

fn multiply_polynomials(left: &[Rational], right: &[Rational]) -> Polynomial {
    let mut product = vec![Rational::zero(); left.len() + right.len() - 1];
    for (i, a) in left.iter().enumerate() {
        for (j, b) in right.iter().enumerate() {
            product[i + j] += a.clone() * b.clone();
        }
    }
    trim(&mut product);
    product
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::coupled_friction::{
        solve_coupled_signorini_coulomb, CoupledCoulombOutcome, CoupledCoulombStatus,
    };

    fn assert_intervals_are_exactly_one_root(polynomial: &[Rational], cert: &RootCertificate) {
        let sequence = sturm_sequence(polynomial).unwrap();
        for (lower, upper) in &cert.intervals {
            assert!(!evaluate(polynomial, lower).is_zero());
            assert!(!evaluate(polynomial, upper).is_zero());
            assert_eq!(roots_between(&sequence, polynomial, lower, upper).unwrap(), 1);
        }
        assert_eq!(cert.intervals.len(), cert.finite_real_root_count);
        for pair in cert.intervals.windows(2) {
            assert!(pair[0].1 < pair[1].0, "root intervals overlap");
        }
    }

    fn circular_angle_error(left: f64, right: f64) -> f64 {
        let difference = (left - right).abs();
        difference.min(2.0 * std::f64::consts::PI - difference)
    }

    fn assert_float_candidates_match_exact_intervals(
        outcome: &CoupledCoulombOutcome,
        cert: &RootCertificate,
    ) {
        for (lower, upper) in &cert.intervals {
            let midpoint = (lower.clone() + upper.clone())
                / Rational::from_integer(BigInt::from(2_i64));
            let tangent = midpoint.to_f64().expect("finite interval midpoint");
            let mut angle = 2.0 * tangent.atan();
            if angle < 0.0 {
                angle += 2.0 * std::f64::consts::PI;
            }
            assert!(
                outcome
                    .sliding_angles
                    .iter()
                    .any(|candidate| circular_angle_error(*candidate, angle) < 1.0e-5),
                "no runtime angle matches Sturm interval [{lower}, {upper}]"
            );
        }
        if cert.projective_pi_root {
            assert!(
                outcome.sliding_angles.iter().any(|angle| {
                    circular_angle_error(*angle, std::f64::consts::PI) < 1.0e-8
                }),
                "runtime angle list omitted projective theta=pi root"
            );
        }
        assert_eq!(
            outcome.sliding_angles.len(),
            cert.finite_real_root_count + usize::from(cert.projective_pi_root),
            "runtime and exact distinct-root counts differ"
        );
    }

    #[test]
    fn binary64_inputs_are_interpreted_as_exact_dyadic_rationals() {
        for value in [0.1, -3.25, f64::MIN_POSITIVE, f64::from_bits(1)] {
            let rational = exact_binary64(value).unwrap();
            assert_eq!(rational.to_f64().unwrap().to_bits(), value.to_bits());
        }
        assert!(exact_binary64(-0.0).unwrap().is_zero());
        assert!(exact_binary64(f64::NAN).is_none());
        assert!(exact_binary64(f64::INFINITY).is_none());
    }

    #[test]
    fn sturm_counts_distinct_roots_not_multiplicity() {
        let x_minus_one = integer_polynomial(&[-1, 1]);
        let x_plus_two = integer_polynomial(&[2, 1]);
        let repeated = multiply_polynomials(
            &multiply_polynomials(&x_minus_one, &x_minus_one),
            &multiply_polynomials(&x_plus_two, &x_plus_two),
        );
        let cert = certify_polynomial(&repeated, 1.0e-12).unwrap();
        assert_eq!(cert.finite_real_root_count, 2);
        assert_intervals_are_exactly_one_root(&repeated, &cert);
    }

    #[test]
    fn sturm_separates_nearly_coalescing_roots() {
        let epsilon = Rational::new(BigInt::one(), BigInt::one() << 30_usize);
        let factors = [
            integer_polynomial(&[-3, 1]),
            integer_polynomial(&[-2, 1]),
            integer_polynomial(&[-1, 1]),
            vec![-(Rational::one() + epsilon), Rational::one()],
        ];
        let mut polynomial = vec![Rational::one()];
        for factor in factors {
            polynomial = multiply_polynomials(&polynomial, &factor);
        }
        let cert = certify_polynomial(&polynomial, 1.0e-13).unwrap();
        assert_eq!(cert.finite_real_root_count, 4);
        assert_intervals_are_exactly_one_root(&polynomial, &cert);
        assert!(cert.intervals[0].1 < cert.intervals[1].0);
        assert!(cert.intervals[1].0 < cert.intervals[2].0);
    }

    #[test]
    fn no_real_roots_and_projective_degree_reduction_are_counted_exactly() {
        let no_roots = integer_polynomial(&[1, 0, 1]);
        let no_root_cert = certify_polynomial(&no_roots, 1.0e-12).unwrap();
        assert_eq!(no_root_cert.finite_real_root_count, 0);
        assert!(no_root_cert.intervals.is_empty());

        // t^3 - t has three finite roots and a separate projective theta=pi
        // root because the quartic chart coefficient at t^4 is exactly zero.
        let reduced = integer_polynomial(&[0, -1, 0, 1, 0]);
        let reduced_cert = certify_polynomial(&reduced, 1.0e-12).unwrap();
        assert_eq!(reduced_cert.finite_real_root_count, 3);
        assert!(reduced_cert.projective_pi_root);
        assert_intervals_are_exactly_one_root(&reduced, &reduced_cert);
    }

    #[test]
    fn rejects_an_identically_zero_angular_polynomial() {
        assert!(certify_polynomial(&[], 1.0e-12).is_err());
        assert!(certify_polynomial(&integer_polynomial(&[0, 0, 0]), 1.0e-12).is_err());
    }

    #[test]
    fn exact_sturm_count_matches_both_frozen_multiple_solution_witnesses() {
        let cases = [
            (
                Matrix3::new(
                    0.59497432, -0.39054620, -0.63542426,
                    -0.39054620, 1.65052340, 0.80910089,
                    -0.63542426, 0.80910089, 0.94522735,
                ),
                SVector::new(-0.10770302, 1.49149027, -0.95561905),
                1.383289774658335,
            ),
            (
                Matrix3::new(
                    3.38215268, 1.97328063, 2.48164591,
                    1.97328063, 5.28809392, -1.19836457,
                    2.48164591, -1.19836457, 4.53646300,
                ),
                SVector::new(0.93301773, 0.94422924, 2.42911533),
                2.609311810828915,
            ),
        ];

        for (w, q, mu) in cases {
            let outcome = solve_coupled_signorini_coulomb(&w, &q, mu, 1.0e-11).unwrap();
            assert_eq!(outcome.status, CoupledCoulombStatus::MultipleSolutions);
            assert!(outcome.candidates.len() >= 2);

            let symmetric_w = (&w + w.transpose()) * 0.5;
            let polynomial = exact_sliding_quartic(&symmetric_w, &q, mu).unwrap();
            let cert = certify_polynomial(&polynomial, 1.0e-10).unwrap();
            assert_intervals_are_exactly_one_root(&polynomial, &cert);
            assert_float_candidates_match_exact_intervals(&outcome, &cert);
        }
    }

    #[test]
    fn deterministic_spd_panel_matches_independent_exact_root_counts() {
        for seed in 1..=64_u64 {
            let mut x = [[0.0_f64; 3]; 3];
            for i in 0..3 {
                for j in 0..3 {
                    let k = (seed * (7 + (i * 3 + j) as u64 * 11)) % 101;
                    x[i][j] = (k as f64 / 37.0) - 1.2;
                }
            }
            let m = Matrix3::new(
                x[0][0], x[0][1], x[0][2],
                x[1][0], x[1][1], x[1][2],
                x[2][0], x[2][1], x[2][2],
            );
            let w = m.transpose() * m + Matrix3::identity() * 0.3;
            let q = SVector::new(
                -0.2 - (seed % 13) as f64 / 7.0,
                (seed % 17) as f64 / 5.0 - 1.3,
                (seed % 19) as f64 / 6.0 - 1.1,
            );
            let mu = 0.05 + (seed % 11) as f64 * 0.21;
            let outcome = solve_coupled_signorini_coulomb(&w, &q, mu, 1.0e-10)
                .unwrap_or_else(|error| panic!("seed={seed}: {error:?}"));
            let symmetric_w = (&w + w.transpose()) * 0.5;
            let polynomial = exact_sliding_quartic(&symmetric_w, &q, mu).unwrap();
            let cert = certify_polynomial(&polynomial, 1.0e-9)
                .unwrap_or_else(|error| panic!("seed={seed}: {error}"));
            assert_intervals_are_exactly_one_root(&polynomial, &cert);
            assert_float_candidates_match_exact_intervals(&outcome, &cert);
        }
    }
}
