// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Small outward-rounded closed intervals for validated-bound experiments.
//!
//! Each finite endpoint operation is widened by one adjacent f64. This
//! assumes IEEE-754 binary64 basic arithmetic with round-to-nearest and no
//! fast-math reassociation. Non-finite intermediates are rejected.
//!
//! This is an arithmetic primitive, not a full interval-analysis engine. It
//! does not implement interval matrices, automatic differentiation, Krawczyk
//! operators, Taylor models, or a physical Hessian bound.

use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct IntervalV1 {
    lower: f64,
    upper: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntervalErrorV1 {
    NonFiniteEndpoint,
    ReversedEndpoints,
    NonFiniteIntermediate,
    DivisionByIntervalContainingZero,
    EmptyIntersection,
}

impl fmt::Display for IntervalErrorV1 {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{self:?}")
    }
}

impl std::error::Error for IntervalErrorV1 {}

fn finite(value: f64) -> Result<f64, IntervalErrorV1> {
    if value.is_finite() {
        Ok(value)
    } else {
        Err(IntervalErrorV1::NonFiniteIntermediate)
    }
}

impl IntervalV1 {
    /// Admit a closed finite interval [lower, upper].
    pub fn new(lower: f64, upper: f64) -> Result<Self, IntervalErrorV1> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(IntervalErrorV1::NonFiniteEndpoint);
        }
        if lower > upper {
            return Err(IntervalErrorV1::ReversedEndpoints);
        }
        Ok(Self { lower, upper })
    }

    pub fn point(value: f64) -> Result<Self, IntervalErrorV1> {
        Self::new(value, value)
    }

    pub fn lower(self) -> f64 {
        self.lower
    }

    pub fn upper(self) -> f64 {
        self.upper
    }

    pub fn contains(self, value: f64) -> bool {
        value.is_finite() && self.lower <= value && value <= self.upper
    }

    pub fn contains_interval(self, other: Self) -> bool {
        self.lower <= other.lower && other.upper <= self.upper
    }

    pub fn contains_zero(self) -> bool {
        self.lower <= 0.0 && self.upper >= 0.0
    }

    pub fn is_subset_of(self, other: Self) -> bool {
        other.contains_interval(self)
    }

    pub fn add(self, other: Self) -> Result<Self, IntervalErrorV1> {
        let lower = finite((self.lower + other.lower).next_down())?;
        let upper = finite((self.upper + other.upper).next_up())?;
        Self::new(lower, upper)
    }

    pub fn sub(self, other: Self) -> Result<Self, IntervalErrorV1> {
        let lower = finite((self.lower - other.upper).next_down())?;
        let upper = finite((self.upper - other.lower).next_up())?;
        Self::new(lower, upper)
    }

    /// Four endpoint products give an inclusion enclosure for multiplication.
    pub fn mul(self, other: Self) -> Result<Self, IntervalErrorV1> {
        let products = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(IntervalErrorV1::NonFiniteIntermediate);
        }
        let minimum = products.iter().copied().fold(f64::INFINITY, f64::min);
        let maximum = products
            .iter()
            .copied()
            .fold(f64::NEG_INFINITY, f64::max);
        Self::new(finite(minimum.next_down())?, finite(maximum.next_up())?)
    }

    pub fn reciprocal(self) -> Result<Self, IntervalErrorV1> {
        if self.contains_zero() {
            return Err(IntervalErrorV1::DivisionByIntervalContainingZero);
        }
        let lower = finite((1.0 / self.upper).next_down())?;
        let upper = finite((1.0 / self.lower).next_up())?;
        Self::new(lower, upper)
    }

    pub fn div(self, denominator: Self) -> Result<Self, IntervalErrorV1> {
        self.mul(denominator.reciprocal()?)
    }

    /// Dependency-aware square; avoids negative lower bounds when crossing zero.
    pub fn square(self) -> Result<Self, IntervalErrorV1> {
        let maximum_magnitude = self.lower.abs().max(self.upper.abs());
        let maximum_square = maximum_magnitude * maximum_magnitude;
        if !maximum_square.is_finite() {
            return Err(IntervalErrorV1::NonFiniteIntermediate);
        }
        let upper = finite(maximum_square.next_up())?;

        if self.contains_zero() {
            return Self::new(0.0, upper);
        }

        let minimum_magnitude = self.lower.abs().min(self.upper.abs());
        let minimum_square = minimum_magnitude * minimum_magnitude;
        if !minimum_square.is_finite() {
            return Err(IntervalErrorV1::NonFiniteIntermediate);
        }
        let lower = finite(minimum_square.next_down())?.max(0.0);
        Self::new(lower, upper)
    }

    pub fn hull(self, other: Self) -> Result<Self, IntervalErrorV1> {
        Self::new(self.lower.min(other.lower), self.upper.max(other.upper))
    }

    pub fn intersection(self, other: Self) -> Result<Self, IntervalErrorV1> {
        let lower = self.lower.max(other.lower);
        let upper = self.upper.min(other.upper);
        if lower > upper {
            return Err(IntervalErrorV1::EmptyIntersection);
        }
        Self::new(lower, upper)
    }

    pub fn width_upper(self) -> Result<f64, IntervalErrorV1> {
        finite((self.upper - self.lower).next_up())
    }

    pub fn absolute_upper(self) -> Result<f64, IntervalErrorV1> {
        finite(self.lower.abs().max(self.upper.abs()).next_up())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn admission_rejects_nonfinite_and_reversed_endpoints() {
        assert_eq!(IntervalV1::new(f64::NAN, 1.0), Err(IntervalErrorV1::NonFiniteEndpoint));
        assert_eq!(IntervalV1::new(0.0, f64::INFINITY), Err(IntervalErrorV1::NonFiniteEndpoint));
        assert_eq!(IntervalV1::new(2.0, 1.0), Err(IntervalErrorV1::ReversedEndpoints));
    }

    #[test]
    fn addition_and_subtraction_enclose_exact_dyadic_results() {
        let left = IntervalV1::new(-1.25, 2.5).unwrap();
        let right = IntervalV1::new(0.5, 1.75).unwrap();
        let sum = left.add(right).unwrap();
        let difference = left.sub(right).unwrap();
        for a in [-1.25, 0.0, 2.5] {
            for b in [0.5, 1.0, 1.75] {
                assert!(sum.contains(a + b));
                assert!(difference.contains(a - b));
            }
        }
    }

    #[test]
    fn multiplication_encloses_all_sign_combinations() {
        let intervals = [
            IntervalV1::new(-3.0, -1.0).unwrap(),
            IntervalV1::new(-2.0, 4.0).unwrap(),
            IntervalV1::new(0.5, 2.5).unwrap(),
        ];
        for left in intervals {
            for right in intervals {
                let product = left.mul(right).unwrap();
                for a in [left.lower(), 0.5 * (left.lower() + left.upper()), left.upper()] {
                    for b in [right.lower(), 0.5 * (right.lower() + right.upper()), right.upper()] {
                        assert!(product.contains(a * b));
                    }
                }
            }
        }
    }

    #[test]
    fn square_handles_zero_crossing_without_negative_lower_bound() {
        let crossing = IntervalV1::new(-2.0, 3.0).unwrap();
        let squared = crossing.square().unwrap();
        assert_eq!(squared.lower(), 0.0);
        for value in [-2.0, -1.0, 0.0, 1.0, 3.0] {
            assert!(squared.contains(value * value));
        }
        let positive = IntervalV1::new(2.0, 3.0).unwrap().square().unwrap();
        assert!(positive.contains(4.0));
        assert!(positive.contains(9.0));
        let negative = IntervalV1::new(-3.0, -2.0).unwrap().square().unwrap();
        assert!(negative.contains(4.0));
        assert!(negative.contains(9.0));
    }

    #[test]
    fn division_encloses_positive_and_negative_quotients() {
        let numerator = IntervalV1::new(-4.0, 6.0).unwrap();
        for denominator in [
            IntervalV1::new(2.0, 4.0).unwrap(),
            IntervalV1::new(-4.0, -2.0).unwrap(),
        ] {
            let quotient = numerator.div(denominator).unwrap();
            for a in [-4.0, 0.0, 6.0] {
                for b in [denominator.lower(), denominator.upper()] {
                    assert!(quotient.contains(a / b));
                }
            }
        }
    }

    #[test]
    fn denominator_touching_or_crossing_zero_is_rejected() {
        for denominator in [
            IntervalV1::new(0.0, 1.0).unwrap(),
            IntervalV1::new(-1.0, 0.0).unwrap(),
            IntervalV1::new(-1.0, 1.0).unwrap(),
        ] {
            assert_eq!(
                IntervalV1::point(1.0).unwrap().div(denominator),
                Err(IntervalErrorV1::DivisionByIntervalContainingZero)
            );
        }
    }

    #[test]
    fn subnormal_endpoints_are_enclosed_and_reciprocal_overflow_is_rejected() {
        let tiny = f64::from_bits(1);
        let point = IntervalV1::point(tiny).unwrap();
        let square = point.square().unwrap();
        assert!(square.contains(tiny * tiny));
        assert!(square.lower() >= 0.0);
        assert_eq!(point.reciprocal(), Err(IntervalErrorV1::NonFiniteIntermediate));
    }

    #[test]
    fn signed_zero_operations_remain_ordered() {
        let zero = IntervalV1::new(-0.0, 0.0).unwrap();
        let widened = zero.add(zero).unwrap();
        assert!(widened.lower() < 0.0);
        assert!(widened.upper() > 0.0);
        assert!(widened.contains(0.0));
    }

    #[test]
    fn overflow_fails_closed() {
        let huge = IntervalV1::point(f64::MAX).unwrap();
        assert_eq!(huge.add(huge), Err(IntervalErrorV1::NonFiniteIntermediate));
        assert_eq!(
            huge.mul(IntervalV1::point(2.0).unwrap()),
            Err(IntervalErrorV1::NonFiniteIntermediate)
        );
    }

    #[test]
    fn hull_and_intersection_have_expected_set_semantics() {
        let left = IntervalV1::new(-2.0, 1.0).unwrap();
        let right = IntervalV1::new(0.0, 4.0).unwrap();
        assert_eq!(left.hull(right).unwrap(), IntervalV1::new(-2.0, 4.0).unwrap());
        assert_eq!(left.intersection(right).unwrap(), IntervalV1::new(0.0, 1.0).unwrap());
        assert_eq!(
            left.intersection(IntervalV1::new(2.0, 3.0).unwrap()),
            Err(IntervalErrorV1::EmptyIntersection)
        );
    }

    #[test]
    fn interval_expansion_is_monotone_under_hull() {
        let small = IntervalV1::new(0.25, 0.5).unwrap();
        let large = small.hull(IntervalV1::new(-0.5, 0.75).unwrap()).unwrap();
        assert!(small.is_subset_of(large));
        assert!(large.width_upper().unwrap() >= small.width_upper().unwrap());
        assert!(large.absolute_upper().unwrap() >= small.absolute_upper().unwrap());
    }
}
