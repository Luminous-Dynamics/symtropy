// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root
//! Small outward-rounded interval primitives for numerical geometry.
//!
//! These operations enclose the exact real result of each scalar operation
//! over finite binary64 endpoints, provided the target uses IEEE-754 binary64
//! operations with round-to-nearest and does not enable flush-to-zero or
//! unsafe fast-math transformations. Every operation is widened by one
//! representable value after the hardware operation. Underflow is therefore
//! conservative; overflow is reported as an error rather than converted into
//! a finite certificate.
//!
//! This module is deliberately not wired into `support_aabb` or BVH pruning.
//! It does not by itself enclose the error of an arbitrary `Shape::support`,
//! rotor evaluation, normalization, trigonometric function, or compound-shape
//! implementation. Those paths need shape-/transform-specific interval
//! evaluation before a broadphase bound may claim to enclose a collider.

/// Failure to construct or evaluate a finite interval.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntervalError {
    /// An input endpoint was NaN or infinite.
    NonFiniteInput,
    /// The lower endpoint exceeds the upper endpoint.
    ReversedEndpoints,
    /// An operation overflowed, or its outward expansion became infinite.
    ArithmeticOverflow,
}

/// A closed interval over finite binary64 values.
///
/// Endpoints are private so callers cannot construct a reversed or non-finite
/// interval without going through the checked constructor.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    /// Construct a finite, ordered, closed interval.
    pub fn new(lower: f64, upper: f64) -> Result<Self, IntervalError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalError::NonFiniteInput);
        }
        if lower > upper {
            return Err(IntervalError::ReversedEndpoints);
        }
        Ok(Self { lower, upper })
    }

    /// Construct the exact singleton containing one finite binary64 value.
    pub fn point(value: f64) -> Result<Self, IntervalError> {
        Self::new(value, value)
    }

    /// Inclusive lower endpoint.
    #[inline]
    pub fn lower(self) -> f64 {
        self.lower
    }

    /// Inclusive upper endpoint.
    #[inline]
    pub fn upper(self) -> f64 {
        self.upper
    }

    /// Whether this interval contains a finite represented value.
    #[inline]
    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && self.lower <= value && value <= self.upper
    }

    /// Outward-rounded addition.
    ///
    /// On success, the result contains every exact real sum x + y for
    /// x in self and y in rhs. Overflow and non-finite intermediate results
    /// return an error so callers cannot use them as exclusion evidence.
    pub fn checked_add_outward(self, rhs: Self) -> Result<Self, IntervalError> {
        Self::from_rounded_bounds(self.lower + rhs.lower, self.upper + rhs.upper)
    }

    /// Outward-rounded subtraction.
    ///
    /// On success, the result contains every exact real difference x - y for
    /// x in self and y in rhs.
    pub fn checked_sub_outward(self, rhs: Self) -> Result<Self, IntervalError> {
        Self::from_rounded_bounds(self.lower - rhs.upper, self.upper - rhs.lower)
    }

    /// Outward-rounded multiplication.
    ///
    /// The four endpoint products bound the exact range of interval
    /// multiplication. Each product is widened before the minimum and maximum
    /// are selected. Any non-finite intermediate result fails open.
    pub fn checked_mul_outward(self, rhs: Self) -> Result<Self, IntervalError> {
        let products = [
            self.lower * rhs.lower,
            self.lower * rhs.upper,
            self.upper * rhs.lower,
            self.upper * rhs.upper,
        ];

        let mut lower = f64::INFINITY;
        let mut upper = f64::NEG_INFINITY;
        for product in products {
            if !product.is_finite() {
                return Err(IntervalError::ArithmeticOverflow);
            }
            let product_lower = next_down(product);
            let product_upper = next_up(product);
            if !product_lower.is_finite() || !product_upper.is_finite() {
                return Err(IntervalError::ArithmeticOverflow);
            }
            lower = lower.min(product_lower);
            upper = upper.max(product_upper);
        }

        Self::new(lower, upper)
    }

    #[inline]
    fn from_rounded_bounds(lower: f64, upper: f64) -> Result<Self, IntervalError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalError::ArithmeticOverflow);
        }

        let lower = next_down(lower);
        let upper = next_up(upper);
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalError::ArithmeticOverflow);
        }

        Self::new(lower, upper)
    }
}

/// The next representable binary64 value greater than `value`.
///
/// NaN and positive infinity are returned unchanged. Negative infinity steps
/// to the largest-magnitude finite negative value.
#[inline]
fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }

    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits + 1)
    } else {
        f64::from_bits(bits - 1)
    }
}

/// The next representable binary64 value less than `value`.
///
/// NaN and negative infinity are returned unchanged. Positive infinity steps
/// to the largest finite positive value.
#[inline]
fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits((1_u64 << 63) | 1);
    }

    let bits = value.to_bits();
    if value > 0.0 {
        f64::from_bits(bits - 1)
    } else {
        f64::from_bits(bits + 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn addition_widens_rounded_result_in_both_directions() {
        let lhs = Interval::point(0.1).unwrap();
        let rhs = Interval::point(0.2).unwrap();
        let sum = lhs.checked_add_outward(rhs).unwrap();

        // The ordinary binary64 result lies inside the outward enclosure.
        // The lower endpoint is also the adjacent representable value below
        // that result, covering the real (pre-rounding) sum.
        let rounded = 0.1_f64 + 0.2_f64;
        assert!(sum.contains(rounded));
        assert!(sum.lower() < rounded);
        assert!(sum.upper() > rounded);
    }

    #[test]
    fn subtraction_handles_cancellation_and_negative_values() {
        let lhs = Interval::point(-0.1).unwrap();
        let rhs = Interval::point(-0.2).unwrap();
        let difference = lhs.checked_sub_outward(rhs).unwrap();
        assert!(difference.contains(-0.1_f64 - (-0.2_f64)));
        assert!(difference.lower() < difference.upper());
    }

    #[test]
    fn multiplication_handles_all_sign_combinations() {
        let lhs = Interval::new(-2.0, 3.0).unwrap();
        let rhs = Interval::new(-4.0, 5.0).unwrap();
        let product = lhs.checked_mul_outward(rhs).unwrap();

        assert!(product.contains(-12.0));
        assert!(product.contains(15.0));
        assert!(product.lower() <= -12.0);
        assert!(product.upper() >= 15.0);
    }

    #[test]
    fn multiplication_underflow_expands_around_zero() {
        let least_subnormal = f64::from_bits(1);
        let product = Interval::point(least_subnormal)
            .unwrap()
            .checked_mul_outward(Interval::point(0.5).unwrap())
            .unwrap();

        // The exact product is positive but rounds to zero in binary64.
        // Both adjacent representable values conservatively enclose it.
        assert!(product.lower() < 0.0);
        assert!(product.upper() > 0.0);
        assert!(product.contains(0.0));
    }

    #[test]
    fn arithmetic_overflow_cannot_produce_a_finite_certificate() {
        let largest = Interval::point(f64::MAX).unwrap();
        assert_eq!(
            largest.checked_add_outward(largest),
            Err(IntervalError::ArithmeticOverflow)
        );
        assert_eq!(
            largest.checked_mul_outward(largest),
            Err(IntervalError::ArithmeticOverflow)
        );
    }

    #[test]
    fn constructor_rejects_non_finite_and_reversed_inputs() {
        assert_eq!(
            Interval::new(f64::NAN, 1.0),
            Err(IntervalError::NonFiniteInput)
        );
        assert_eq!(
            Interval::new(f64::NEG_INFINITY, 1.0),
            Err(IntervalError::NonFiniteInput)
        );
        assert_eq!(
            Interval::new(2.0, 1.0),
            Err(IntervalError::ReversedEndpoints)
        );
    }

    #[test]
    fn adjacent_value_helpers_handle_signed_zero_and_infinities() {
        assert_eq!(next_up(0.0).to_bits(), 1);
        assert_eq!(next_up(-0.0).to_bits(), 1);
        assert_eq!(next_down(0.0).to_bits(), (1_u64 << 63) | 1);
        assert_eq!(next_down(-0.0).to_bits(), (1_u64 << 63) | 1);
        assert_eq!(next_down(f64::INFINITY), f64::MAX);
        assert_eq!(next_up(f64::NEG_INFINITY), -f64::MAX);
    }
}
