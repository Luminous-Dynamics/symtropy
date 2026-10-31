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
fn exact_sliding_quartic_from_rational_inputs(
    a: Rational,
    d1: Rational,
    d2: Rational,
    qn: Rational,
    q1: Rational,
    q2: Rational,
    mu: Rational,
    a11: Rational,
    a12: Rational,
    a22: Rational,
) -> Polynomial {
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
    polynomial_add(&polynomial_add(&first, &second), &third)
}

fn exact_sliding_quartic_from_inputs(
    w: &Matrix3<f64>,
    q: &SVector<f64, 3>,
    mu: f64,
) -> Result<Polynomial, &'static str> {
    Ok(exact_sliding_quartic_from_rational_inputs(
        exact_f64(w[(0, 0)])?,
        exact_f64(w[(0, 1)])?,
        exact_f64(w[(0, 2)])?,
        exact_f64(q[0])?,
        exact_f64(q[1])?,
        exact_f64(q[2])?,
        exact_f64(mu)?,
        exact_f64(w[(1, 1)])?,
        exact_f64(w[(1, 2)])?,
        exact_f64(w[(2, 2)])?,
    ))
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


#[derive(Clone, Debug, PartialEq, Eq)]
struct ExactInterval {
    lower: Rational,
    upper: Rational,
}

impl ExactInterval {
    fn new(lower: Rational, upper: Rational) -> Result<Self, &'static str> {
        if lower > upper {
            return Err("interval lower endpoint exceeds upper endpoint");
        }
        Ok(Self { lower, upper })
    }

    fn point(value: Rational) -> Self {
        Self {
            lower: value.clone(),
            upper: value,
        }
    }

    fn from_f64_bounds(lower: f64, upper: f64) -> Result<Self, &'static str> {
        Self::new(exact_f64(lower)?, exact_f64(upper)?)
    }

    fn zero() -> Self {
        Self::point(Rational::zero())
    }

    fn one() -> Self {
        Self::point(Rational::one())
    }

    fn contains_zero(&self) -> bool {
        self.lower <= Rational::zero() && self.upper >= Rational::zero()
    }

    fn excludes_zero(&self) -> bool {
        self.upper < Rational::zero() || self.lower > Rational::zero()
    }

    fn add(&self, rhs: &Self) -> Self {
        Self {
            lower: self.lower.clone() + rhs.lower.clone(),
            upper: self.upper.clone() + rhs.upper.clone(),
        }
    }

    fn neg(&self) -> Self {
        Self {
            lower: -self.upper.clone(),
            upper: -self.lower.clone(),
        }
    }

    fn mul(&self, rhs: &Self) -> Self {
        let products = [
            self.lower.clone() * rhs.lower.clone(),
            self.lower.clone() * rhs.upper.clone(),
            self.upper.clone() * rhs.lower.clone(),
            self.upper.clone() * rhs.upper.clone(),
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
        Self { lower, upper }
    }

    fn pow(&self, exponent: u32) -> Self {
        if exponent == 0 {
            return Self::one();
        }
        let mut result = Self::one();
        for _ in 0..exponent {
            result = result.mul(self);
        }
        result
    }

    fn abs_upper(&self) -> Rational {
        self.lower.abs().max(self.upper.abs())
    }

    fn midpoint(&self) -> Rational {
        (self.lower.clone() + self.upper.clone())
            / Rational::from_integer(BigInt::from(2))
    }
}

type IntervalPolynomial = Vec<ExactInterval>;

fn interval_polynomial_add(
    lhs: &[ExactInterval],
    rhs: &[ExactInterval],
) -> IntervalPolynomial {
    let mut sum = vec![ExactInterval::zero(); lhs.len().max(rhs.len())];
    for (index, coefficient) in lhs.iter().enumerate() {
        sum[index] = sum[index].add(coefficient);
    }
    for (index, coefficient) in rhs.iter().enumerate() {
        sum[index] = sum[index].add(coefficient);
    }
    sum
}

fn interval_polynomial_scale(
    poly: &[ExactInterval],
    factor: &ExactInterval,
) -> IntervalPolynomial {
    poly.iter().map(|coefficient| coefficient.mul(factor)).collect()
}

fn interval_polynomial_multiply(
    lhs: &[ExactInterval],
    rhs: &[ExactInterval],
) -> IntervalPolynomial {
    if lhs.is_empty() || rhs.is_empty() {
        return Vec::new();
    }
    let mut product = vec![ExactInterval::zero(); lhs.len() + rhs.len() - 1];
    for (left_power, left) in lhs.iter().enumerate() {
        for (right_power, right) in rhs.iter().enumerate() {
            let index = left_power + right_power;
            product[index] = product[index].add(&left.mul(right));
        }
    }
    product
}

/// Interval expansion of the half-angle vector equation. Every operation is
/// exact rational interval arithmetic; dependency overestimation is allowed,
/// false exclusion of a possible coefficient is not.
fn interval_sliding_quartic(
    a: &ExactInterval,
    d1: &ExactInterval,
    d2: &ExactInterval,
    qn: &ExactInterval,
    q1: &ExactInterval,
    q2: &ExactInterval,
    mu: &ExactInterval,
    a11: &ExactInterval,
    a12: &ExactInterval,
    a22: &ExactInterval,
) -> IntervalPolynomial {
    let one_plus_t2 = vec![
        ExactInterval::one(),
        ExactInterval::zero(),
        ExactInterval::one(),
    ];
    let one_minus_t2 = vec![
        ExactInterval::one(),
        ExactInterval::zero(),
        ExactInterval::one().neg(),
    ];
    let two_t = vec![
        ExactInterval::zero(),
        ExactInterval::point(Rational::from_integer(BigInt::from(2))),
    ];
    let minus_two_t = interval_polynomial_scale(
        &two_t,
        &ExactInterval::one().neg(),
    );

    let d_linear = interval_polynomial_add(
        &interval_polynomial_scale(&one_minus_t2, d1),
        &interval_polynomial_scale(&two_t, d2),
    );
    let d_numerator = interval_polynomial_add(
        &interval_polynomial_scale(&one_plus_t2, a),
        &interval_polynomial_scale(&d_linear, &mu.neg()),
    );

    let q_tangent_perp = interval_polynomial_add(
        &interval_polynomial_scale(&minus_two_t, q1),
        &interval_polynomial_scale(&one_minus_t2, q2),
    );
    let d_perp = interval_polynomial_add(
        &interval_polynomial_scale(&minus_two_t, d1),
        &interval_polynomial_scale(&one_minus_t2, d2),
    );

    let a_s_first = interval_polynomial_add(
        &interval_polynomial_scale(&one_minus_t2, a11),
        &interval_polynomial_scale(&two_t, a12),
    );
    let a_s_second = interval_polynomial_add(
        &interval_polynomial_scale(&one_minus_t2, a12),
        &interval_polynomial_scale(&two_t, a22),
    );
    let perp_dot_a_s = interval_polynomial_add(
        &interval_polynomial_multiply(&minus_two_t, &a_s_first),
        &interval_polynomial_multiply(&one_minus_t2, &a_s_second),
    );

    let first = interval_polynomial_multiply(&d_numerator, &q_tangent_perp);
    let second = interval_polynomial_scale(
        &interval_polynomial_multiply(&d_perp, &one_plus_t2),
        &qn.neg(),
    );
    let third = interval_polynomial_scale(
        &perp_dot_a_s,
        &mu.mul(qn),
    );
    interval_polynomial_add(&interval_polynomial_add(&first, &second), &third)
}

fn interval_monomial(
    integer_scale: i64,
    factors: &[(&ExactInterval, u32)],
) -> ExactInterval {
    let mut value = ExactInterval::point(Rational::from_integer(BigInt::from(integer_scale)));
    for (factor, power) in factors {
        value = value.mul(&factor.pow(*power));
    }
    value
}

/// Exact interval enclosure of the quartic discriminant. The closed form is
/// expanded term-by-term with outward-exact rational interval operations.
fn exact_quartic_discriminant_via_resultant(poly: &[Rational]) -> Result<Rational, &'static str> {
    if poly.len() != 5 {
        return Err("exact resultant helper requires a quartic");
    }
    let descending = poly.iter().rev().cloned().collect::<Vec<_>>();
    let derivative_descending = vec![
        &descending[0] * Rational::from_integer(BigInt::from(4)),
        &descending[1] * Rational::from_integer(BigInt::from(3)),
        &descending[2] * Rational::from_integer(BigInt::from(2)),
        descending[3].clone(),
    ];
    let mut sylvester = vec![vec![Rational::zero(); 7]; 7];
    for shift in 0..3 {
        for (index, coefficient) in descending.iter().enumerate() {
            sylvester[shift][shift + index] = coefficient.clone();
        }
    }
    for shift in 0..4 {
        for (index, coefficient) in derivative_descending.iter().enumerate() {
            sylvester[3 + shift][shift + index] = coefficient.clone();
        }
    }

    let mut determinant = Rational::one();
    for pivot_index in 0..7 {
        let pivot_row = (pivot_index..7)
            .find(|row| !sylvester[*row][pivot_index].is_zero())
            .ok_or("singular Sylvester matrix")?;
        if pivot_row != pivot_index {
            sylvester.swap(pivot_row, pivot_index);
            determinant = -determinant;
        }
        let pivot = sylvester[pivot_index][pivot_index].clone();
        determinant *= pivot.clone();
        for row in pivot_index + 1..7 {
            if sylvester[row][pivot_index].is_zero() {
                continue;
            }
            let factor = sylvester[row][pivot_index].clone() / pivot.clone();
            for column in pivot_index + 1..7 {
                sylvester[row][column] -=
                    factor.clone() * sylvester[pivot_index][column].clone();
            }
            sylvester[row][pivot_index] = Rational::zero();
        }
    }
    let leading = poly.last().ok_or("missing quartic leading coefficient")?;
    if leading.is_zero() {
        return Err("quartic leading coefficient is zero");
    }
    // For degree four, (-1)^(4*3/2) = +1, so Disc(P)=Res(P,P')/a.
    Ok(determinant / leading.clone())
}

fn quartic_discriminant_interval(coefficients: &[ExactInterval]) -> Result<ExactInterval, &'static str> {
    if coefficients.len() != 5 {
        return Err("quartic coefficient interval vector must have length five");
    }
    let e = &coefficients[0];
    let d = &coefficients[1];
    let c = &coefficients[2];
    let b = &coefficients[3];
    let a = &coefficients[4];

    let terms = [
        interval_monomial(256, &[(a, 3), (e, 3)]),
        interval_monomial(-192, &[(a, 2), (b, 1), (d, 1), (e, 2)]),
        interval_monomial(-128, &[(a, 2), (c, 2), (e, 2)]),
        interval_monomial(144, &[(a, 2), (c, 1), (d, 2), (e, 1)]),
        interval_monomial(-27, &[(a, 2), (d, 4)]),
        interval_monomial(144, &[(a, 1), (b, 2), (c, 1), (e, 2)]),
        interval_monomial(-6, &[(a, 1), (b, 2), (d, 2), (e, 1)]),
        interval_monomial(-80, &[(a, 1), (b, 1), (c, 2), (d, 1), (e, 1)]),
        interval_monomial(18, &[(a, 1), (b, 1), (c, 1), (d, 3)]),
        interval_monomial(16, &[(a, 1), (c, 4), (e, 1)]),
        interval_monomial(-4, &[(a, 1), (c, 3), (d, 2)]),
        interval_monomial(-27, &[(b, 4), (e, 2)]),
        interval_monomial(18, &[(b, 3), (c, 1), (d, 1), (e, 1)]),
        interval_monomial(-4, &[(b, 3), (d, 3)]),
        interval_monomial(-4, &[(b, 2), (c, 3), (e, 1)]),
        interval_monomial(1, &[(b, 2), (c, 2), (d, 2)]),
    ];
    Ok(terms
        .iter()
        .fold(ExactInterval::zero(), |sum, term| sum.add(term)))
}

#[derive(Clone, Debug)]
struct ContactInputBox {
    // Independent symmetric entries: W = [[w00,w01,w02],[w01,w11,w12],[w02,w12,w22]].
    w00: ExactInterval,
    w01: ExactInterval,
    w02: ExactInterval,
    w11: ExactInterval,
    w12: ExactInterval,
    w22: ExactInterval,
    q0: ExactInterval,
    q1: ExactInterval,
    q2: ExactInterval,
    mu: ExactInterval,
}

impl ContactInputBox {
    fn from_f64_bounds(bounds: [[f64; 2]; 10]) -> Result<Self, &'static str> {
        Ok(Self {
            w00: ExactInterval::from_f64_bounds(bounds[0][0], bounds[0][1])?,
            w01: ExactInterval::from_f64_bounds(bounds[1][0], bounds[1][1])?,
            w02: ExactInterval::from_f64_bounds(bounds[2][0], bounds[2][1])?,
            w11: ExactInterval::from_f64_bounds(bounds[3][0], bounds[3][1])?,
            w12: ExactInterval::from_f64_bounds(bounds[4][0], bounds[4][1])?,
            w22: ExactInterval::from_f64_bounds(bounds[5][0], bounds[5][1])?,
            q0: ExactInterval::from_f64_bounds(bounds[6][0], bounds[6][1])?,
            q1: ExactInterval::from_f64_bounds(bounds[7][0], bounds[7][1])?,
            q2: ExactInterval::from_f64_bounds(bounds[8][0], bounds[8][1])?,
            mu: ExactInterval::from_f64_bounds(bounds[9][0], bounds[9][1])?,
        })
    }

    fn gershgorin_spd_lower_bound(&self) -> Rational {
        let row_0 = self.w00.lower.clone()
            - self.w01.abs_upper()
            - self.w02.abs_upper();
        let row_1 = self.w11.lower.clone()
            - self.w01.abs_upper()
            - self.w12.abs_upper();
        let row_2 = self.w22.lower.clone()
            - self.w02.abs_upper()
            - self.w12.abs_upper();
        row_0.min(row_1).min(row_2)
    }

    fn representative_polynomial(&self) -> Polynomial {
        exact_sliding_quartic_from_rational_inputs(
            self.w00.midpoint(),
            self.w01.midpoint(),
            self.w02.midpoint(),
            self.q0.midpoint(),
            self.q1.midpoint(),
            self.q2.midpoint(),
            self.mu.midpoint(),
            self.w11.midpoint(),
            self.w12.midpoint(),
            self.w22.midpoint(),
        )
    }

    fn coefficient_intervals(&self) -> IntervalPolynomial {
        interval_sliding_quartic(
            &self.w00,
            &self.w01,
            &self.w02,
            &self.q0,
            &self.q1,
            &self.q2,
            &self.mu,
            &self.w11,
            &self.w12,
            &self.w22,
        )
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
enum FamilyRootStatus {
    Certified { finite_real_root_count: usize },
    NeedsSubdivision(&'static str),
    InvalidInput(&'static str),
}

#[derive(Clone, Debug)]
struct FamilyRootCertificate {
    status: FamilyRootStatus,
    coefficient_intervals: IntervalPolynomial,
    leading_coefficient: Option<ExactInterval>,
    discriminant: Option<ExactInterval>,
    spd_gershgorin_lower_bound: Rational,
}

fn certify_contact_input_box(input: &ContactInputBox) -> FamilyRootCertificate {
    let spd_lower_bound = input.gershgorin_spd_lower_bound();
    let coefficients = input.coefficient_intervals();
    let leading = coefficients.get(4).cloned();
    let discriminant = if coefficients.len() == 5 {
        quartic_discriminant_interval(&coefficients).ok()
    } else {
        None
    };

    let status = if input.mu.upper < Rational::zero() {
        FamilyRootStatus::InvalidInput("friction interval is entirely negative")
    } else if input.mu.lower <= Rational::zero() {
        FamilyRootStatus::NeedsSubdivision("friction interval includes zero")
    } else if spd_lower_bound <= Rational::zero() {
        FamilyRootStatus::NeedsSubdivision("interval Gershgorin bound does not prove SPD")
    } else if leading.as_ref().is_none_or(|coefficient| coefficient.contains_zero()) {
        FamilyRootStatus::NeedsSubdivision("leading coefficient may cross the projective boundary")
    } else if discriminant.as_ref().is_none_or(ExactInterval::contains_zero) {
        FamilyRootStatus::NeedsSubdivision("discriminant enclosure contains zero or is unavailable")
    } else {
        let representative = input.representative_polynomial();
        match sturm_isolate_rational(representative) {
            Ok(intervals) => FamilyRootStatus::Certified {
                finite_real_root_count: intervals.len(),
            },
            Err(_) => FamilyRootStatus::NeedsSubdivision("representative exact Sturm count failed"),
        }
    };

    FamilyRootCertificate {
        status,
        coefficient_intervals: coefficients,
        leading_coefficient: leading,
        discriminant,
        spd_gershgorin_lower_bound: spd_lower_bound,
    }
}

#[test]
fn interval_discriminant_formula_matches_an_independent_exact_resultant() {
    let roots = [-3_i64, -1, 2, 5]
        .into_iter()
        .map(|value| Rational::from_integer(BigInt::from(value)))
        .collect::<Vec<_>>();
    let polynomial = polynomial_from_roots(&roots);
    let exact_discriminant = exact_quartic_discriminant_via_resultant(&polynomial).unwrap();
    let interval_coefficients = polynomial
        .iter()
        .cloned()
        .map(ExactInterval::point)
        .collect::<Vec<_>>();
    let enclosed = quartic_discriminant_interval(&interval_coefficients).unwrap();
    assert_eq!(enclosed.lower, exact_discriminant);
    assert_eq!(enclosed.upper, exact_discriminant);
}

#[test]
fn interval_coefficient_enclosures_contain_seeded_exact_dyadic_samples() {
    let bounds = [
        [0.95, 1.05],
        [-0.02, 0.02],
        [-0.02, 0.02],
        [0.95, 1.05],
        [-0.02, 0.02],
        [0.95, 1.05],
        [-1.05, -0.95],
        [-3.05, -2.95],
        [0.95, 1.05],
        [0.20, 0.30],
    ];
    let input = ContactInputBox::from_f64_bounds(bounds).unwrap();
    let enclosure = input.coefficient_intervals();

    for seed in 0..128_u64 {
        let mut values = [0.0_f64; 10];
        for index in 0..10 {
            let bucket = (seed * (13 + index as u64 * 17) + 7 * index as u64) % 1009;
            let fraction = bucket as f64 / 1008.0;
            values[index] = bounds[index][0]
                + fraction * (bounds[index][1] - bounds[index][0]);
        }
        let exact = exact_sliding_quartic_from_rational_inputs(
            exact_f64(values[0]).unwrap(),
            exact_f64(values[1]).unwrap(),
            exact_f64(values[2]).unwrap(),
            exact_f64(values[6]).unwrap(),
            exact_f64(values[7]).unwrap(),
            exact_f64(values[8]).unwrap(),
            exact_f64(values[9]).unwrap(),
            exact_f64(values[3]).unwrap(),
            exact_f64(values[4]).unwrap(),
            exact_f64(values[5]).unwrap(),
        );
        assert_eq!(enclosure.len(), exact.len());
        for (index, exact_coefficient) in exact.iter().enumerate() {
            assert!(
                enclosure[index].lower <= *exact_coefficient
                    && *exact_coefficient <= enclosure[index].upper,
                "seed={seed}, coefficient={index}"
            );
        }
    }
}

#[test]
fn interval_family_certificate_accepts_a_narrow_regular_spd_box() {
    let input = ContactInputBox::from_f64_bounds([
        [0.9999, 1.0001],
        [-0.0001, 0.0001],
        [-0.0001, 0.0001],
        [0.9999, 1.0001],
        [-0.0001, 0.0001],
        [0.9999, 1.0001],
        [-1.0001, -0.9999],
        [-3.0001, -2.9999],
        [0.9999, 1.0001],
        [0.2499, 0.2501],
    ]).unwrap();
    let certificate = certify_contact_input_box(&input);
    assert!(
        matches!(certificate.status, FamilyRootStatus::Certified { finite_real_root_count: 2 }),
        "unexpected family status: {:?}, leading={:?}, discriminant={:?}",
        certificate.status,
        certificate.leading_coefficient,
        certificate.discriminant
    );
    assert!(certificate.spd_gershgorin_lower_bound > Rational::zero());
    assert!(certificate.leading_coefficient.as_ref().is_some_and(ExactInterval::excludes_zero));
    assert!(certificate.discriminant.as_ref().is_some_and(ExactInterval::excludes_zero));
    assert_eq!(certificate.coefficient_intervals.len(), 5);
}

#[test]
fn interval_family_certificate_refuses_projective_leading_coefficient_crossing() {
    let input = ContactInputBox::from_f64_bounds([
        [0.999, 1.001],
        [0.0, 0.0],
        [0.0, 0.0],
        [0.999, 1.001],
        [0.0, 0.0],
        [0.999, 1.001],
        [-1.001, -0.999],
        [-3.001, -2.999],
        [-0.001, 0.001],
        [0.249, 0.251],
    ]).unwrap();
    let certificate = certify_contact_input_box(&input);
    assert_eq!(
        certificate.status,
        FamilyRootStatus::NeedsSubdivision("leading coefficient may cross the projective boundary")
    );
}

#[test]
fn interval_family_certificate_refuses_boxes_without_an_spd_proof() {
    let input = ContactInputBox::from_f64_bounds([
        [0.9, 1.1],
        [-1.0, 1.0],
        [-1.0, 1.0],
        [0.9, 1.1],
        [-1.0, 1.0],
        [0.9, 1.1],
        [-1.1, -0.9],
        [-3.1, -2.9],
        [0.9, 1.1],
        [0.24, 0.26],
    ]).unwrap();
    let certificate = certify_contact_input_box(&input);
    assert_eq!(
        certificate.status,
        FamilyRootStatus::NeedsSubdivision("interval Gershgorin bound does not prove SPD")
    );
}

#[test]
fn interval_family_certificate_rejects_negative_friction_and_refuses_zero() {
    let negative = ContactInputBox::from_f64_bounds([
        [1.0, 1.0], [0.0, 0.0], [0.0, 0.0],
        [1.0, 1.0], [0.0, 0.0], [1.0, 1.0],
        [-1.0, -1.0], [-3.0, -3.0], [1.0, 1.0],
        [-0.2, -0.1],
    ]).unwrap();
    assert!(matches!(
        certify_contact_input_box(&negative).status,
        FamilyRootStatus::InvalidInput(_)
    ));

    let includes_zero = ContactInputBox::from_f64_bounds([
        [1.0, 1.0], [0.0, 0.0], [0.0, 0.0],
        [1.0, 1.0], [0.0, 0.0], [1.0, 1.0],
        [-1.0, -1.0], [-3.0, -3.0], [1.0, 1.0],
        [0.0, 0.2],
    ]).unwrap();
    assert_eq!(
        certify_contact_input_box(&includes_zero).status,
        FamilyRootStatus::NeedsSubdivision("friction interval includes zero")
    );
}

#[test]
fn interval_discriminant_contains_zero_for_a_wide_unresolved_box() {
    let input = ContactInputBox::from_f64_bounds([
        [0.5, 1.5],
        [-0.1, 0.1],
        [-0.1, 0.1],
        [0.5, 1.5],
        [-0.1, 0.1],
        [0.5, 1.5],
        [-1.0, -0.5],
        [-4.0, -2.0],
        [-1.0, 1.0],
        [0.1, 0.8],
    ]).unwrap();
    let certificate = certify_contact_input_box(&input);
    assert!(matches!(certificate.status, FamilyRootStatus::NeedsSubdivision(_)));
}
