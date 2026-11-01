// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Shared exact-rational polynomial and Sturm primitives for physics assurance tests.
//!
//! The algorithms here are intentionally independent of floating-point root
//! finders. Binary64 inputs are decoded to exact dyadic rationals; Sturm chains
//! and rational isolating intervals then certify distinct real roots.

use num_bigint::BigInt;
use num_rational::BigRational;
use num_traits::{One, Signed, Zero};

pub(crate) type Rational = BigRational;
pub(crate) type Polynomial = Vec<Rational>;

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

pub(crate) fn trim(poly: &mut Polynomial) {
    while poly.last().is_some_and(|coefficient| coefficient.is_zero()) {
        poly.pop();
    }
}

pub(crate) fn is_zero(poly: &[Rational]) -> bool {
    poly.iter().all(|coefficient| coefficient.is_zero())
}

pub(crate) fn derivative(poly: &[Rational]) -> Polynomial {
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

pub(crate) fn evaluate(poly: &[Rational], x: &Rational) -> Rational {
    poly.iter()
        .rev()
        .fold(Rational::zero(), |value, coefficient| {
            value * x.clone() + coefficient.clone()
        })
}

pub(crate) fn normalize_positive(poly: &mut Polynomial) {
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

pub(crate) fn remainder(
    dividend: &[Rational],
    divisor: &[Rational],
) -> Result<Polynomial, &'static str> {
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

pub(crate) fn sturm_chain(input: &[Rational]) -> Result<Vec<Polynomial>, &'static str> {
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
pub(crate) fn one_sided_sign(
    poly: &[Rational],
    x: &Rational,
    side: i8,
) -> Result<i8, &'static str> {
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

pub(crate) fn variations(
    sequence: &[Polynomial],
    x: &Rational,
    side: i8,
) -> Result<usize, &'static str> {
    let signs = sequence
        .iter()
        .map(|poly| one_sided_sign(poly, x, side))
        .collect::<Result<Vec<_>, _>>()?;
    Ok(signs.windows(2).filter(|pair| pair[0] != pair[1]).count())
}

/// Number of distinct real roots in the open interval (lower, upper).
pub(crate) fn count_open(
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

pub(crate) fn cauchy_bound(poly: &[Rational]) -> Result<Rational, &'static str> {
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
pub(crate) fn move_endpoint_off_root(
    poly: &[Rational],
    chain: &[Polynomial],
    endpoint: &Rational,
    toward: &Rational,
    endpoint_is_lower: bool,
) -> Result<Rational, &'static str> {
    for exponent in 1..=512_usize {
        let denominator = Rational::from_integer(BigInt::one() << exponent);
        let candidate = endpoint.clone() + (toward.clone() - endpoint.clone()) / denominator;
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

pub(crate) fn isolate_recursive(
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
            poly,
            chain,
            lower,
            midpoint.clone(),
            left_count,
            relative_width,
            depth + 1,
            intervals,
        )?;
        isolate_recursive(
            poly,
            chain,
            midpoint,
            upper,
            right_count,
            relative_width,
            depth + 1,
            intervals,
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
        poly,
        chain,
        lower,
        midpoint.clone(),
        left_count,
        relative_width,
        depth + 1,
        intervals,
    )?;
    isolate_recursive(
        poly,
        chain,
        midpoint,
        upper,
        right_count,
        relative_width,
        depth + 1,
        intervals,
    )?;
    Ok(())
}

pub(crate) fn sturm_isolate_rational(
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
        &poly,
        &chain,
        lower,
        upper,
        count,
        &relative_width,
        0,
        &mut intervals,
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

pub(crate) fn polynomial_add(lhs: &[Rational], rhs: &[Rational]) -> Polynomial {
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

pub(crate) fn polynomial_scale(poly: &[Rational], factor: &Rational) -> Polynomial {
    let mut scaled = poly
        .iter()
        .map(|coefficient| coefficient.clone() * factor.clone())
        .collect::<Polynomial>();
    trim(&mut scaled);
    scaled
}

pub(crate) fn polynomial_multiply(lhs: &[Rational], rhs: &[Rational]) -> Polynomial {
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
