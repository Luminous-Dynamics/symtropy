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

use super::{real_polynomial_roots, sliding_quartic};
use nalgebra::{Matrix3, SVector};
use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, ToPrimitive, Zero};

type Rational = BigRational;
type Polynomial = Vec<Rational>;

fn exact_f64(value: f64) -> Result<Rational, &'static str> {
    if !value.is_finite() {
        return Err("non-finite binary64 coefficient");
    }
    let bits = value.to_bits();
    let negative = (bits >> 63) != 0;
    let exponent_bits = ((bits >> 52) & 0x7ff) as i32;
    let fraction = bits & ((1_u64 << 52) - 1);
    let (mantissa, exponent) = if exponent_bits == 0 {
        (fraction, -1074)
    } else {
        ((1_u64 << 52) | fraction, exponent_bits - 1023 - 52)
    };
    if mantissa == 0 {
        return Ok(Rational::zero());
    }
    let mut numerator = BigInt::from(mantissa);
    if negative {
        numerator = -numerator;
    }
    if exponent >= 0 {
        numerator <<= exponent as usize;
        Ok(Rational::from_integer(numerator))
    } else {
        let denominator = BigInt::one() << (-exponent) as usize;
        Ok(Rational::new(numerator, denominator))
    }
}

fn trim(poly: &mut Polynomial) {
    while poly.last().is_some_and(|coefficient| coefficient.is_zero()) {
        poly.pop();
    }
}

fn is_zero(poly: &[Rational]) -> bool {
    poly.iter().all(|coefficient| coefficient.is_zero())
}

fn derivative(poly: &[Rational]) -> Polynomial {
    if poly.len() <= 1 {
        return Vec::new();
    }
    poly.iter()
        .enumerate()
        .skip(1)
        .map(|(power, coefficient)| {
            coefficient.clone() * Rational::from_integer(BigInt::from(power))
        })
        .collect()
}

fn evaluate(poly: &[Rational], x: &Rational) -> Rational {
    poly.iter().rev().fold(Rational::zero(), |value, coefficient| {
        value * x.clone() + coefficient.clone()
    })
}

fn normalize_positive(poly: &mut Polynomial) {
    trim(poly);
    if let Some(leading) = poly.last() {
        let scale = leading.abs();
        if !scale.is_zero() {
            for coefficient in poly.iter_mut() {
                *coefficient = coefficient.clone() / scale.clone();
            }
        }
    }
}

fn remainder(dividend: &[Rational], divisor: &[Rational]) -> Result<Polynomial, &'static str> {
    if is_zero(divisor) {
        return Err("zero divisor in exact polynomial remainder");
    }
    let mut rest = dividend.to_vec();
    trim(&mut rest);
    while !is_zero(&rest) && rest.len() >= divisor.len() {
        let shift = rest.len() - divisor.len();
        let factor = rest.last().ok_or("empty polynomial remainder")?.clone()
            / divisor.last().ok_or("empty polynomial divisor")?.clone();
        for (index, coefficient) in divisor.iter().enumerate() {
            let slot = index + shift;
            rest[slot] = rest[slot].clone() - factor.clone() * coefficient.clone();
        }
        trim(&mut rest);
    }
    Ok(rest)
}

fn sturm_chain(input: &[Rational]) -> Result<Vec<Polynomial>, &'static str> {
    let mut first = input.to_vec();
    trim(&mut first);
    if is_zero(&first) {
        return Err("zero polynomial has no finite isolated root set");
    }
    if first.len() == 1 {
        return Ok(vec![first]);
    }
    normalize_positive(&mut first);
    let mut second = derivative(&first);
    normalize_positive(&mut second);
    let mut sequence = vec![first, second];
    loop {
        let n = sequence.len();
        let mut next = remainder(&sequence[n - 2], &sequence[n - 1])?;
        for coefficient in &mut next {
            *coefficient = -coefficient.clone();
        }
        trim(&mut next);
        if is_zero(&next) {
            break;
        }
        normalize_positive(&mut next);
        sequence.push(next);
    }
    Ok(sequence)
}

/// Exact sign immediately to one side of x; side is -1 (left) or +1 (right).
fn one_sided_sign(poly: &[Rational], x: &Rational, side: i8) -> Result<i8, &'static str> {
    let mut current = poly.to_vec();
    let mut derivative_order = 0_usize;
    loop {
        if current.is_empty() {
            return Err("identically zero Sturm member");
        }
        let value = evaluate(&current, x);
        if !value.is_zero() {
            let mut sign = if value.is_positive() { 1 } else { -1 };
            if side < 0 && derivative_order % 2 == 1 {
                sign = -sign;
            }
            return Ok(sign);
        }
        current = derivative(&current);
        derivative_order += 1;
    }
}

fn variations(sequence: &[Polynomial], x: &Rational, side: i8) -> Result<usize, &'static str> {
    let signs = sequence
        .iter()
        .map(|poly| one_sided_sign(poly, x, side))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(signs.windows(2).filter(|pair| pair[0] != pair[1]).count())
}

/// Number of distinct real roots in the open interval (lower, upper).
fn count_open(
    sequence: &[Polynomial],
    lower: &Rational,
    upper: &Rational,
) -> Result<usize, &'static str> {
    if lower >= upper {
        return Ok(0);
    }
    let left = variations(sequence, lower, 1)?;
    let right = variations(sequence, upper, -1)?;
    left.checked_sub(right)
        .ok_or("Sturm variation count increased across an interval")
}

fn cauchy_bound(poly: &[Rational]) -> Result<Rational, &'static str> {
    if poly.len() <= 1 {
        return Ok(Rational::one());
    }
    let leading = poly.last().ok_or("missing leading coefficient")?.abs();
    if leading.is_zero() {
        return Err("zero leading coefficient after trim");
    }
    let mut largest = Rational::zero();
    for coefficient in &poly[..poly.len() - 1] {
        let ratio = coefficient.abs() / leading.clone();
        if ratio > largest {
            largest = ratio;
        }
    }
    Ok(Rational::one() + largest)
}

fn isolate_recursive(
    poly: &[Rational],
    chain: &[Polynomial],
    lower: Rational,
    upper: Rational,
    count: usize,
    relative_width: &Rational,
    depth: usize,
    intervals: &mut Vec<(Rational, Rational)>,
) -> Result<(), &'static str> {
    if count == 0 {
        return Ok(());
    }
    if depth > 512 {
        return Err("exact Sturm isolation exceeded 512 bisections");
    }
    let midpoint = (&lower + &upper) / Rational::from_integer(BigInt::from(2));
    let midpoint_value = evaluate(poly, &midpoint);

    // Preserve an exactly dyadic rational root as a point interval and count
    // the open subintervals independently using one-sided Sturm signs.
    if midpoint_value.is_zero() {
        let left_count = count_open(chain, &lower, &midpoint)?;
        let right_count = count_open(chain, &midpoint, &upper)?;
        if left_count + right_count + 1 != count {
            return Err("Sturm count disagrees at an exact midpoint root");
        }
        intervals.push((midpoint.clone(), midpoint.clone()));
        isolate_recursive(
            poly, chain, lower, midpoint.clone(), left_count, relative_width, depth + 1, intervals,
        )?;
        isolate_recursive(
            poly, chain, midpoint, upper, right_count, relative_width, depth + 1, intervals,
        )?;
        return Ok(());
    }

    let width = upper.clone() - lower.clone();
    let target = relative_width.clone() * (Rational::one() + midpoint.abs());
    if count == 1 && width <= target {
        intervals.push((lower, upper));
        return Ok(());
    }

    let left_count = count_open(chain, &lower, &midpoint)?;
    let right_count = count_open(chain, &midpoint, &upper)?;
    if left_count + right_count != count {
        return Err("Sturm count was not conserved during bisection");
    }
    isolate_recursive(
        poly, chain, lower, midpoint.clone(), left_count, relative_width, depth + 1, intervals,
    )?;
    isolate_recursive(
        poly, chain, midpoint, upper, right_count, relative_width, depth + 1, intervals,
    )?;
    Ok(())
}

/// Returns disjoint rational intervals, each containing exactly one distinct
/// real root of the exact polynomial represented by the supplied f64 values.
fn sturm_isolate_f64(
    coefficients_ascending: &[f64],
) -> Result<Vec<(Rational, Rational)>, &'static str> {
    let mut poly = coefficients_ascending
        .iter()
        .map(|coefficient| exact_f64(*coefficient))
        .collect::<Result<Polynomial, _>>()?;
    trim(&mut poly);
    if poly.is_empty() {
        return Err("zero polynomial");
    }
    if poly.len() == 1 {
        return Ok(Vec::new());
    }
    let chain = sturm_chain(&poly)?;
    let bound = cauchy_bound(&poly)?;
    let lower = -bound.clone();
    let upper = bound;
    let count = count_open(&chain, &lower, &upper)?;
    let relative_width = Rational::new(BigInt::one(), BigInt::one() << 72_usize);
    let mut intervals = Vec::with_capacity(count);
    isolate_recursive(
        &poly, &chain, lower, upper, count, &relative_width, 0, &mut intervals,
    )?;
    intervals.sort_by(|a, b| a.0.cmp(&b.0));
    if intervals.len() != count {
        return Err("isolated interval count does not match Sturm count");
    }
    Ok(intervals)
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
    poly.iter().map(|coefficient| coefficient.to_f64().unwrap()).collect()
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
        let scaled = coefficients.iter().map(|coefficient| coefficient * factor).collect::<Vec<_>>();
        let intervals = sturm_isolate_f64(&scaled).unwrap();
        assert_eq!(intervals.len(), expected);
        assert_production_matches_sturm(&scaled, 1.0e-12);
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
    assert!(intervals.iter().any(|(lower, upper)| lower.is_zero() && upper.is_zero()));
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
    let no_real_roots = vec![1.0, 0.0, 0.0, 0.0, 1.0];
    assert!(sturm_isolate_f64(&no_real_roots).unwrap().is_empty());
    assert!(real_polynomial_roots(&no_real_roots, 1.0e-12).unwrap().is_empty());

    let degree_three = vec![-6.0, 11.0, -6.0, 1.0, 0.0];
    let intervals = sturm_isolate_f64(&degree_three).unwrap();
    assert_eq!(intervals.len(), 3);
    assert_production_matches_sturm(&degree_three, 1.0e-12);
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
        let coefficients = sliding_quartic(&w, &q, mu);
        let exact = sturm_isolate_f64(&coefficients).unwrap_or_else(|error| {
            panic!("seed={seed}: exact Sturm oracle failed: {error}")
        });
        match real_polynomial_roots(&coefficients, 1.0e-10) {
            Ok(numerical) => {
                assert_eq!(
                    numerical.len(),
                    exact.len(),
                    "seed={seed}: exact={}, numerical={numerical:?}, coefficients={coefficients:?}",
                    exact.len()
                );
                for interval in &exact {
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
    assert!(checked >= 120, "too many inconclusive roots: {inconclusive}");
}
