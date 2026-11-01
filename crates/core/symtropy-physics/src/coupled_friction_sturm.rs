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
use nalgebra::{Matrix3, SMatrix, SVector};
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
    // Use a strict enclosure, not the sharp non-strict Cauchy bound: no
    // polynomial root may sit on either outer endpoint.
    Ok(Rational::from_integer(BigInt::from(2)) + largest)
}

/// Move a known exact-root endpoint a rational distance into the
/// adjacent root-free gap. The resulting endpoint is non-root, and the open
/// interval between the old and new endpoints is proven to contain no root.
fn move_endpoint_off_root(
    poly: &[Rational],
    chain: &[Polynomial],
    endpoint: &Rational,
    toward: &Rational,
    endpoint_is_lower: bool,
) -> Result<Rational, &'static str> {
    for exponent in 1..=512_usize {
        let denominator = Rational::from_integer(BigInt::one() << exponent);
        let candidate =
            endpoint.clone() + (toward.clone() - endpoint.clone()) / denominator;
        if evaluate(poly, &candidate).is_zero() {
            continue;
        }
        let empty_gap = if endpoint_is_lower {
            count_open(chain, endpoint, &candidate)?
        } else {
            count_open(chain, &candidate, endpoint)?
        };
        if empty_gap == 0 {
            return Ok(candidate);
        }
    }
    Err("could not move a rational interval endpoint off an exact root")
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
        let isolated_lower = if evaluate(poly, &lower).is_zero() {
            move_endpoint_off_root(poly, chain, &lower, &upper, true)?
        } else {
            lower
        };
        let isolated_upper = if evaluate(poly, &upper).is_zero() {
            move_endpoint_off_root(poly, chain, &upper, &isolated_lower, false)?
        } else {
            upper
        };
        if evaluate(poly, &isolated_lower).is_zero()
            || evaluate(poly, &isolated_upper).is_zero()
            || count_open(chain, &isolated_lower, &isolated_upper)? != 1
        {
            return Err("isolating interval endpoint/count certificate failed");
        }
        intervals.push((isolated_lower, isolated_upper));
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

fn sturm_isolate_rational(
    mut poly: Polynomial,
) -> Result<Vec<(Rational, Rational)>, &'static str> {
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
    let mut previous_upper: Option<&Rational> = None;
    for (lower, upper) in &intervals {
        if let Some(previous) = previous_upper {
            if previous >= lower {
                return Err("Sturm isolating intervals are not pairwise disjoint");
            }
        }
        if lower == upper {
            if !evaluate(&poly, lower).is_zero() {
                return Err("point interval does not evaluate to an exact root");
            }
        } else if lower > upper
            || evaluate(&poly, lower).is_zero()
            || evaluate(&poly, upper).is_zero()
            || count_open(&chain, lower, upper)? != 1
        {
            return Err("open isolating interval lacks non-root endpoints or unit Sturm count");
        }
        previous_upper = Some(upper);
    }
    Ok(intervals)
}

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
    let post_tangent_two =
        q2 + lambda.clone() * d2 + tangent_one.clone() * a12;
    if !post_tangent_two.is_zero() {
        return Ok(false);
    }
    let post_tangent_one =
        q1 + lambda.clone() * d1 + tangent_one.clone() * a11;
    let disk_multiplier = -post_tangent_one / tangent_one.clone();
    if disk_multiplier < Rational::zero() {
        return Ok(false);
    }

    // Independently evaluate the full impulse kinetic-energy change:
    // Delta K = p^T q + 1/2 p^T W p, with p=(lambda, mu*lambda, 0).
    let wp_normal = a.clone() * lambda.clone() + d1.clone() * tangent_one.clone();
    let wp_tangent_one = d1 * lambda.clone() + a11 * tangent_one.clone();
    let energy_linear = lambda.clone() * qn + tangent_one.clone() * q1;
    let energy_quadratic =
        lambda.clone() * wp_normal + tangent_one.clone() * wp_tangent_one;
    let energy_delta =
        energy_linear + energy_quadratic / Rational::from_integer(BigInt::from(2));
    Ok(energy_delta <= Rational::zero())
}

fn polynomial_add(lhs: &[Rational], rhs: &[Rational]) -> Polynomial {
    let mut sum = vec![Rational::zero(); lhs.len().max(rhs.len())];
    for (index, coefficient) in lhs.iter().enumerate() {
        sum[index] += coefficient.clone();
    }
    for (index, coefficient) in rhs.iter().enumerate() {
        sum[index] += coefficient.clone();
    }
    trim(&mut sum);
    sum
}

fn polynomial_scale(poly: &[Rational], factor: &Rational) -> Polynomial {
    let mut scaled = poly
        .iter()
        .map(|coefficient| coefficient.clone() * factor.clone())
        .collect::<Polynomial>();
    trim(&mut scaled);
    scaled
}

fn polynomial_multiply(lhs: &[Rational], rhs: &[Rational]) -> Polynomial {
    if lhs.is_empty() || rhs.is_empty() {
        return Vec::new();
    }
    let mut product = vec![Rational::zero(); lhs.len() + rhs.len() - 1];
    for (left_power, left) in lhs.iter().enumerate() {
        for (right_power, right) in rhs.iter().enumerate() {
            let index = left_power + right_power;
            product[index] += left.clone() * right.clone();
        }
    }
    trim(&mut product);
    product
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
    let second = polynomial_scale(
        &polynomial_multiply(&d_perp, &one_plus_t2),
        &negative_qn,
    );
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
    poly.iter().map(|coefficient| coefficient.to_f64().unwrap()).collect()
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
        let scaled = coefficients.iter().map(|coefficient| coefficient * factor).collect::<Vec<_>>();
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
    let no_real_roots = vec![1.0, 1.0, 1.0, 0.0, 1.0];
    assert!(sturm_isolate_f64(&no_real_roots).unwrap().is_empty());
    assert!(real_polynomial_roots(&no_real_roots, 1.0e-12).unwrap().is_empty());

    let degree_three = vec![-6.0, 11.0, -6.0, 1.0, 0.0];
    let intervals = sturm_isolate_f64(&degree_three).unwrap();
    assert_eq!(intervals.len(), 3);
    assert_production_matches_sturm(&degree_three, 1.0e-12);
}

#[test]
fn exact_dyadic_contact_inputs_rederive_the_production_quartic() {
    let w = Matrix3::new(
        3.0, 0.4, -0.2,
        0.4, 2.0, 0.5,
        -0.2, 0.5, 1.5,
    );
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
    let w = Matrix3::new(
        2.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    );
    let q = SVector::new(-1.0, -3.0, 0.0);
    let mu = 0.2;
    let exact = exact_sliding_quartic_from_inputs(&w, &q, mu).unwrap();
    assert!(exact.len() < 5, "theta=pi is the missing leading projective root");
    let finite_roots = sturm_isolate_rational(exact.clone()).unwrap();
    assert_eq!(finite_roots.len(), 1);
    assert!(exact_projective_pi_candidate(&w, &q, mu).unwrap());
    assert_eq!(finite_roots.len() + 1, 2);

    let outcome =
        super::solve_coupled_signorini_coulomb(&w, &q, mu, 1.0e-12).unwrap();
    assert!(
        outcome.sliding_angles.iter().any(|angle| {
            (*angle - std::f64::consts::PI).abs() <= 1.0e-10
        }),
        "production path must explicitly retain the projective theta=pi candidate"
    );
    assert!(
        outcome.candidates.iter().any(|candidate| {
            candidate.sliding_angle.is_some_and(|angle| {
                (angle - std::f64::consts::PI).abs() <= 1.0e-10
            })
        }),
        "the exact projective direction must pass the unreduced impulse/KKT/energy checks"
    );
}

#[test]
fn exact_projective_theta_pi_direction_is_not_promoted_when_kkt_fails() {
    let w = Matrix3::new(
        2.0, 0.0, 0.0,
        0.0, 1.0, 0.0,
        0.0, 0.0, 1.0,
    );
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
        let exact_polynomial =
            exact_sliding_quartic_from_inputs(&w, &q, mu).unwrap_or_else(|error| {
                panic!("seed={seed}: exact quartic construction failed: {error}")
            });
        assert_f64_coefficients_match_exact(&coefficients, &exact_polynomial);
        let exact_intervals = sturm_isolate_rational(exact_polynomial).unwrap_or_else(|error| {
            panic!("seed={seed}: exact Sturm oracle failed: {error}")
        });
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
    assert!(checked >= 120, "too many inconclusive roots: {inconclusive}");
}


type RationalInterval = (Rational, Rational);

fn polynomial_divide_exact(
    dividend: &[Rational],
    divisor: &[Rational],
) -> Result<Polynomial, &'static str> {
    if is_zero(divisor) {
        return Err("zero divisor in exact polynomial division");
    }
    let mut rest = dividend.to_vec();
    trim(&mut rest);
    let mut quotient = vec![
        Rational::zero();
        rest.len().saturating_sub(divisor.len()).saturating_add(1)
    ];
    while !is_zero(&rest) && rest.len() >= divisor.len() {
        let shift = rest.len() - divisor.len();
        let factor = rest.last().ok_or("empty division remainder")?.clone()
            / divisor.last().ok_or("empty division divisor")?.clone();
        quotient[shift] += factor.clone();
        for (index, coefficient) in divisor.iter().enumerate() {
            let slot = index + shift;
            rest[slot] = rest[slot].clone() - factor.clone() * coefficient.clone();
        }
        trim(&mut rest);
    }
    if !is_zero(&rest) {
        return Err("polynomial division left a nonzero exact remainder");
    }
    trim(&mut quotient);
    Ok(quotient)
}

fn polynomial_gcd(
    lhs: &[Rational],
    rhs: &[Rational],
) -> Result<Polynomial, &'static str> {
    let mut a = lhs.to_vec();
    let mut b = rhs.to_vec();
    trim(&mut a);
    trim(&mut b);
    while !is_zero(&b) {
        let next = remainder(&a, &b)?;
        a = b;
        b = next;
    }
    normalize_positive(&mut a);
    Ok(a)
}

fn polynomial_determinant(matrix: &[Vec<Polynomial>]) -> Polynomial {
    if matrix.is_empty() {
        return vec![Rational::one()];
    }
    let mut determinant = Vec::new();
    for column in 0..matrix.len() {
        let minor = matrix
            .iter()
            .skip(1)
            .map(|row| {
                row.iter()
                    .enumerate()
                    .filter_map(|(index, entry)| {
                        (index != column).then_some(entry.clone())
                    })
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let term = polynomial_multiply(
            &matrix[0][column],
            &polynomial_determinant(&minor),
        );
        let signed = if column % 2 == 0 {
            term
        } else {
            polynomial_scale(&term, &-Rational::one())
        };
        determinant = polynomial_add(&determinant, &signed);
    }
    determinant
}

fn exact_two_contact_slide_stick_polynomials(
    w_input: &SMatrix<f64, 6, 6>,
    q_input: &SVector<f64, 6>,
    mu_input: f64,
) -> Result<(Polynomial, Polynomial, Vec<Polynomial>), &'static str> {
    let mut w = vec![vec![Rational::zero(); 6]; 6];
    let mut q = vec![Rational::zero(); 6];
    for row in 0..6 {
        q[row] = exact_f64(q_input[row])?;
        for column in 0..6 {
            w[row][column] = exact_f64(w_input[(row, column)])?;
        }
    }
    let mu = exact_f64(mu_input)?;
    let zero = Rational::zero();
    let one = Rational::one();
    let two = Rational::from_integer(BigInt::from(2));
    let denominator = vec![one.clone(), zero.clone(), one.clone()];
    let one_minus_t_squared = vec![one.clone(), zero.clone(), -one.clone()];
    let twice_t = vec![zero.clone(), two];

    // The active equations are u_n1 = u_n2 = u_t21 = u_t22 = 0.
    // At a fixed angle, p is parameterized by z=(lambda_1, lambda_2, j_21, j_22):
    // p=(lambda_1, -mu*lambda_1*cos(theta), -mu*lambda_1*sin(theta),
    //    lambda_2, j_21, j_22).
    // Multiplying by 1+t^2 makes the 4x4 system polynomial.
    let active_rows = [0_usize, 3, 4, 5];
    let mut system = Vec::with_capacity(4);
    let mut rhs = Vec::with_capacity(4);
    for row in active_rows {
        let mut first_column = polynomial_scale(&denominator, &w[row][0]);
        first_column = polynomial_add(
            &first_column,
            &polynomial_scale(
                &one_minus_t_squared,
                &(-mu.clone() * w[row][1].clone()),
            ),
        );
        first_column = polynomial_add(
            &first_column,
            &polynomial_scale(&twice_t, &(-mu.clone() * w[row][2].clone())),
        );
        system.push(vec![
            first_column,
            polynomial_scale(&denominator, &w[row][3]),
            polynomial_scale(&denominator, &w[row][4]),
            polynomial_scale(&denominator, &w[row][5]),
        ]);
        rhs.push(polynomial_scale(&denominator, &(-q[row].clone())));
    }

    // Cramer's numerators preserve exact rational dependence on t.
    let determinant = polynomial_determinant(&system);
    let mut numerators = Vec::with_capacity(4);
    for column in 0..4 {
        let mut replaced = system.clone();
        for row in 0..4 {
            replaced[row][column] = rhs[row].clone();
        }
        numerators.push(polynomial_determinant(&replaced));
    }

    // Express u_t1 over the common denominator (1+t^2)*det(system).
    let mut impulse_numerators = Vec::with_capacity(6);
    impulse_numerators.push(polynomial_multiply(&denominator, &numerators[0]));
    impulse_numerators.push(polynomial_scale(
        &polynomial_multiply(&one_minus_t_squared, &numerators[0]),
        &(-mu.clone()),
    ));
    impulse_numerators.push(polynomial_scale(
        &polynomial_multiply(&twice_t, &numerators[0]),
        &(-mu),
    ));
    for numerator in numerators.iter().skip(1) {
        impulse_numerators.push(polynomial_multiply(&denominator, numerator));
    }
    let common_denominator = polynomial_multiply(&denominator, &determinant);
    let mut post_velocity_numerators = Vec::with_capacity(6);
    for row in 0..6 {
        let mut value = polynomial_scale(&common_denominator, &q[row]);
        for column in 0..6 {
            value = polynomial_add(
                &value,
                &polynomial_scale(&impulse_numerators[column], &w[row][column]),
            );
        }
        post_velocity_numerators.push(value);
    }

    // Sliding KKT requires s_perp dot u_t1 = 0. The numerator contains
    // (1+t^2)^3, which has no real zeros; remove it exactly to retain the
    // quartic whose distinct real roots are the finite angular candidates.
    let angular_numerator = polynomial_add(
        &polynomial_scale(
            &polynomial_multiply(&twice_t, &post_velocity_numerators[1]),
            &-Rational::one(),
        ),
        &polynomial_multiply(
            &one_minus_t_squared,
            &post_velocity_numerators[2],
        ),
    );
    let mut quartic = angular_numerator;
    for _ in 0..3 {
        quartic = polynomial_divide_exact(&quartic, &denominator)?;
    }
    if quartic.len() != 5 || quartic.last().is_none_or(|coefficient| coefficient.is_zero()) {
        return Err("slide/stick angular polynomial did not reduce to a proper quartic");
    }
    Ok((quartic, determinant, numerators))
}

fn interval_add(lhs: &RationalInterval, rhs: &RationalInterval) -> RationalInterval {
    (
        lhs.0.clone() + rhs.0.clone(),
        lhs.1.clone() + rhs.1.clone(),
    )
}

fn interval_neg(value: &RationalInterval) -> RationalInterval {
    (-value.1.clone(), -value.0.clone())
}

fn interval_sub(lhs: &RationalInterval, rhs: &RationalInterval) -> RationalInterval {
    interval_add(lhs, &interval_neg(rhs))
}

fn interval_mul(lhs: &RationalInterval, rhs: &RationalInterval) -> RationalInterval {
    let products = [
        lhs.0.clone() * rhs.0.clone(),
        lhs.0.clone() * rhs.1.clone(),
        lhs.1.clone() * rhs.0.clone(),
        lhs.1.clone() * rhs.1.clone(),
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
    (lower, upper)
}

fn interval_div(
    numerator: &RationalInterval,
    denominator: &RationalInterval,
) -> Result<RationalInterval, &'static str> {
    if denominator.0 <= Rational::zero() && denominator.1 >= Rational::zero() {
        return Err("interval denominator contains zero");
    }
    let reciprocal = (
        Rational::one() / denominator.1.clone(),
        Rational::one() / denominator.0.clone(),
    );
    Ok(interval_mul(numerator, &reciprocal))
}

fn interval_scale(
    value: &RationalInterval,
    scale: &Rational,
) -> RationalInterval {
    interval_mul(value, &(scale.clone(), scale.clone()))
}

fn interval_evaluate(
    polynomial: &[Rational],
    x: &RationalInterval,
) -> RationalInterval {
    polynomial.iter().rev().fold(
        (Rational::zero(), Rational::zero()),
        |value, coefficient| {
            interval_add(
                &interval_mul(&value, x),
                &(coefficient.clone(), coefficient.clone()),
            )
        },
    )
}

fn exact_symmetric_matrix_is_spd(matrix: &[Vec<Rational>]) -> bool {
    if matrix.is_empty() || matrix.iter().any(|row| row.len() != matrix.len()) {
        return false;
    }
    for row in 0..matrix.len() {
        for column in 0..matrix.len() {
            if matrix[row][column] != matrix[column][row] {
                return false;
            }
        }
    }
    for size in 1..=matrix.len() {
        let leading = matrix
            .iter()
            .take(size)
            .map(|row| {
                row.iter()
                    .take(size)
                    .map(|value| vec![value.clone()])
                    .collect::<Vec<_>>()
            })
            .collect::<Vec<_>>();
        let determinant = polynomial_determinant(&leading);
        if determinant.len() != 1 || determinant[0] <= Rational::zero() {
            return false;
        }
    }
    true
}

fn exact_two_contact_slide_stick_branch_intervals(
    w: &SMatrix<f64, 6, 6>,
    q: &SVector<f64, 6>,
    mu_1: f64,
    determinant: &[Rational],
    numerators: &[Polynomial],
    roots: &[(Rational, Rational)],
) -> Result<Vec<bool>, &'static str> {
    let mut exact_w = vec![vec![Rational::zero(); 6]; 6];
    let mut exact_q = vec![Rational::zero(); 6];
    for row in 0..6 {
        exact_q[row] = exact_f64(q[row])?;
        for column in 0..6 {
            exact_w[row][column] = exact_f64(w[(row, column)])?;
        }
    }
    let mu = exact_f64(mu_1)?;
    let half = Rational::new(BigInt::one(), BigInt::from(2));
    let one = Rational::one();
    let mut admissible = Vec::with_capacity(roots.len());

    for (lower, upper) in roots {
        let t = (lower.clone(), upper.clone());
        let det = interval_evaluate(determinant, &t);
        if det.0 <= Rational::zero() && det.1 >= Rational::zero() {
            return Err("an angular root interval overlaps a singular active-system determinant");
        }
        let z = numerators
            .iter()
            .map(|polynomial| {
                interval_div(&interval_evaluate(polynomial, &t), &det)
            })
            .collect::<Result<Vec<_>, _>>()?;
        let denominator = interval_add(
            &(one.clone(), one.clone()),
            &interval_mul(&t, &t),
        );
        let cosine = interval_div(
            &interval_sub(&(one.clone(), one.clone()), &interval_mul(&t, &t)),
            &denominator,
        )?;
        let sine = interval_div(
            &interval_scale(&t, &Rational::from_integer(BigInt::from(2))),
            &denominator,
        )?;

        let impulse = vec![
            z[0].clone(),
            interval_scale(&interval_mul(&cosine, &z[0]), &(-mu.clone())),
            interval_scale(&interval_mul(&sine, &z[0]), &(-mu.clone())),
            z[1].clone(),
            z[2].clone(),
            z[3].clone(),
        ];
        let mut velocity = Vec::with_capacity(6);
        for row in 0..6 {
            let mut value = (exact_q[row].clone(), exact_q[row].clone());
            for column in 0..6 {
                value = interval_add(
                    &value,
                    &interval_scale(&impulse[column], &exact_w[row][column]),
                );
            }
            velocity.push(value);
        }

        // These four equations define the Cramer system exactly. Their
        // outward interval evaluations must enclose zero.
        for row in [0_usize, 3, 4, 5] {
            if velocity[row].0 > Rational::zero() || velocity[row].1 < Rational::zero() {
                return Err("active normal/sticking equation is not enclosed by its root interval");
            }
        }

        let tangent_norm_squared = interval_add(
            &interval_mul(&impulse[1], &impulse[1]),
            &interval_mul(&impulse[2], &impulse[2]),
        );
        if tangent_norm_squared.0 <= Rational::zero() {
            return Err("sliding tangent impulse norm interval is not strictly positive");
        }
        let tangent_work = interval_add(
            &interval_mul(&velocity[1], &impulse[1]),
            &interval_mul(&velocity[2], &impulse[2]),
        );
        let tau = interval_div(&interval_neg(&tangent_work), &tangent_norm_squared)?;

        let disk_2_margin = interval_sub(
            &interval_mul(&impulse[3], &impulse[3]),
            &interval_add(
                &interval_mul(&impulse[4], &impulse[4]),
                &interval_mul(&impulse[5], &impulse[5]),
            ),
        );
        let mut energy = (Rational::zero(), Rational::zero());
        for row in 0..6 {
            energy = interval_add(
                &energy,
                &interval_mul(
                    &impulse[row],
                    &(exact_q[row].clone(), exact_q[row].clone()),
                ),
            );
            for column in 0..6 {
                energy = interval_add(
                    &energy,
                    &interval_scale(
                        &interval_mul(&impulse[row], &impulse[column]),
                        &(&exact_w[row][column] * half.clone()),
                    ),
                );
            }
        }

        if impulse[0].0 <= Rational::zero()
            || impulse[3].0 <= Rational::zero()
            || disk_2_margin.0 <= Rational::zero()
            || tau.0 <= Rational::zero() && tau.1 >= Rational::zero()
        {
            admissible.push(false);
        } else {
            admissible.push(tau.0 > Rational::zero() && energy.1 <= Rational::zero());
        }
    }
    Ok(admissible)
}

#[test]
fn two_contact_slide_stick_stratum_has_exactly_two_admissible_roots() {
    let w = SMatrix::<f64, 6, 6>::from_row_slice(&[
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
        -0.0083170671965776075,
        -0.63542426,
        0.80910089,
        0.94522735,
        0.002331483384856203,
        0.013469705454618652,
        0.0078042835950851083,
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
        -0.0083170671965776075,
        0.0078042835950851083,
        0.0,
        0.0,
        1.0,
    ]);
    let q = SVector::<f64, 6>::new(
        -0.10770302,
        1.49149027,
        -0.95561905,
        -1.0,
        0.1,
        -0.1,
    );
    let mu_1 = 1.383_289_774_658_335;

    let mut exact_w = vec![vec![Rational::zero(); 6]; 6];
    for row in 0..6 {
        for column in 0..6 {
            exact_w[row][column] = exact_f64(w[(row, column)]).unwrap();
        }
    }
    assert!(
        exact_symmetric_matrix_is_spd(&exact_w),
        "the exact binary64 Delassus matrix must be symmetric positive definite"
    );

    let (quartic, determinant, numerators) =
        exact_two_contact_slide_stick_polynomials(&w, &q, mu_1)
            .expect("exact dyadic slide/stick polynomial derivation");
    assert_eq!(quartic.len(), 5);
    assert!(!quartic.last().expect("quartic leading coefficient").is_zero());
    assert_eq!(
        polynomial_gcd(&quartic, &derivative(&quartic))
            .expect("square-free polynomial gcd")
            .len(),
        1,
        "the reduced angular quartic must have four simple roots"
    );

    let roots = sturm_isolate_rational(quartic.clone())
        .expect("exact Sturm isolation of the slide/stick angular quartic");
    assert_eq!(roots.len(), 4, "all finite real angular roots are isolated");

    // det(M_d) contains the known (1+t^2)^3 scaling factor. Remove it
    // before the Cramer gcd check; the remaining factor has exactly the real
    // zeros of the actual active-system determinant.
    let half_angle_denominator = vec![
        Rational::one(),
        Rational::zero(),
        Rational::one(),
    ];
    let mut singular_determinant = determinant.clone();
    for _ in 0..3 {
        singular_determinant = polynomial_divide_exact(
            &singular_determinant,
            &half_angle_denominator,
        )
        .expect("remove the exact projective denominator factor");
    }
    let singular_angles = sturm_isolate_rational(singular_determinant.clone())
        .expect("exact Sturm isolation of singular active-system angles");
    assert_eq!(singular_angles.len(), 2);
    assert_eq!(
        polynomial_gcd(&singular_determinant, &numerators[0])
            .expect("Cramer/determinant gcd")
            .len(),
        1,
        "singular angular systems have no compatible right-hand side"
    );

    // The nonzero quartic leading coefficient excludes an additional
    // projective root at theta=pi (t=infinity).
    let admissible = exact_two_contact_slide_stick_branch_intervals(
        &w,
        &q,
        mu_1,
        &quartic,
        &determinant,
        &numerators,
        &roots,
    )
    .expect("exact interval classification of every finite angular root");
    assert_eq!(admissible, vec![true, true, false, false]);
}
