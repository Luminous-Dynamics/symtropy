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

use nalgebra::{SMatrix, SVector};

/// Outward enclosure of an axis-aligned box after an affine map.
///
/// The matrix coefficients and translation components are treated as the
/// exact represented binary64 inputs. For every real input point inside the
/// represented local AABB, the exact affine image is enclosed by the returned
/// finite world AABB, provided the interval arithmetic profile documented by
/// this module is upheld.
///
/// This function does not prove that the supplied matrix is itself a
/// conservative enclosure of a higher-level rotor or transform computation.
/// It only certifies the affine map defined by the supplied binary64 matrix
/// and translation.
pub fn transform_aabb_affine_outward<const D: usize>(
    matrix: &SMatrix<f64, D, D>,
    translation: &SVector<f64, D>,
    local_min: &SVector<f64, D>,
    local_max: &SVector<f64, D>,
) -> Result<(SVector<f64, D>, SVector<f64, D>), IntervalError> {
    if matrix.iter().any(|value| !value.is_finite())
        || translation.iter().any(|value| !value.is_finite())
        || local_min.iter().any(|value| !value.is_finite())
        || local_max.iter().any(|value| !value.is_finite())
    {
        return Err(IntervalError::NonFiniteInput);
    }

    if (0..D).any(|axis| local_min[axis] > local_max[axis]) {
        return Err(IntervalError::ReversedEndpoints);
    }

    let mut world_min = SVector::<f64, D>::zeros();
    let mut world_max = SVector::<f64, D>::zeros();

    for output_axis in 0..D {
        let mut image = Interval::point(translation[output_axis])?;

        for input_axis in 0..D {
            let coefficient = Interval::point(matrix[(output_axis, input_axis)])?;
            let coordinate = Interval::new(local_min[input_axis], local_max[input_axis])?;
            let contribution = coefficient.checked_mul_outward(coordinate)?;
            image = image.checked_add_outward(contribution)?;
        }

        world_min[output_axis] = image.lower();
        world_max[output_axis] = image.upper();
    }

    Ok((world_min, world_max))
}

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
    fn affine_aabb_identity_contains_represented_box_endpoints() {
        let matrix = SMatrix::<f64, 3, 3>::identity();
        let translation = SVector::from([10.0, -3.0, 0.25]);
        let local_min = SVector::from([-2.0, -1.0, 0.0]);
        let local_max = SVector::from([4.0, 5.0, 2.0]);

        let (world_min, world_max) =
            transform_aabb_affine_outward(&matrix, &translation, &local_min, &local_max).unwrap();

        for axis in 0..3 {
            assert!(world_min[axis] <= local_min[axis] + translation[axis]);
            assert!(world_max[axis] >= local_max[axis] + translation[axis]);
            assert!(world_min[axis].is_finite());
            assert!(world_max[axis].is_finite());
        }
    }

    #[test]
    fn affine_aabb_encloses_all_transformed_corners_with_rotation_and_shear() {
        let matrix = SMatrix::<f64, 3, 3>::from_row_slice(&[
            0.0, -1.0, 0.25, 1.0, 0.0, 0.5, 0.125, -0.25, 1.0,
        ]);
        let translation = SVector::from([1.25, -2.5, 0.75]);
        let local_min = SVector::from([-2.0, -0.5, 1.0]);
        let local_max = SVector::from([3.0, 2.0, 4.0]);

        let (world_min, world_max) =
            transform_aabb_affine_outward(&matrix, &translation, &local_min, &local_max).unwrap();

        // Enumerate all 2^3 vertices independently of the interval row
        // accumulation. This checks the affine hull's corners, which attain
        // every coordinate extremum for a linear map over a box.
        for corner in 0..8 {
            let point = SVector::<f64, 3>::from_fn(|axis, _| {
                if corner & (1 << axis) == 0 {
                    local_min[axis]
                } else {
                    local_max[axis]
                }
            });
            let transformed = matrix * point + translation;
            for axis in 0..3 {
                assert!(
                    world_min[axis] <= transformed[axis],
                    "axis {axis}: lower {} > corner {}",
                    world_min[axis],
                    transformed[axis]
                );
                assert!(
                    transformed[axis] <= world_max[axis],
                    "axis {axis}: corner {} > upper {}",
                    transformed[axis],
                    world_max[axis]
                );
            }
        }
    }

    #[test]
    fn affine_aabb_handles_negative_coefficients_and_degenerate_axes() {
        let matrix = SMatrix::<f64, 2, 2>::from_row_slice(&[-2.0, 0.0, 0.5, -1.0]);
        let translation = SVector::from([0.0, 3.0]);
        let local_min = SVector::from([1.0, 2.0]);
        let local_max = SVector::from([1.0, 2.0]);

        let (world_min, world_max) =
            transform_aabb_affine_outward(&matrix, &translation, &local_min, &local_max).unwrap();

        let point = matrix * local_min + translation;
        for axis in 0..2 {
            assert!(world_min[axis] <= point[axis]);
            assert!(point[axis] <= world_max[axis]);
        }
    }

    #[test]
    fn affine_aabb_seeded_four_dimensional_corner_corpus_is_contained() {
        let mut state = 0x8d26_4f31_7a90_c5e1_u64;
        let mut sample = || {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let unit = (state >> 11) as f64 * (1.0 / ((1_u64 << 53) as f64));
            (unit * 2.0) - 1.0
        };

        for case in 0..96 {
            let matrix = SMatrix::<f64, 4, 4>::from_fn(|_, _| sample() * 3.0);
            let translation = SVector::<f64, 4>::from_fn(|_, _| sample() * 100.0);
            let local_min = SVector::<f64, 4>::from_fn(|_, _| sample() * 5.0 - 5.0);
            let widths = SVector::<f64, 4>::from_fn(|_, _| sample().abs() * 4.0);
            let local_max = local_min + widths;

            let result =
                transform_aabb_affine_outward(&matrix, &translation, &local_min, &local_max);
            let (world_min, world_max) = result
                .unwrap_or_else(|error| panic!("case {case} unexpectedly failed: {error:?}"));

            for corner in 0..16 {
                let point = SVector::<f64, 4>::from_fn(|axis, _| {
                    if corner & (1 << axis) == 0 {
                        local_min[axis]
                    } else {
                        local_max[axis]
                    }
                });
                let transformed = matrix * point + translation;
                for axis in 0..4 {
                    assert!(
                        world_min[axis] <= transformed[axis],
                        "case {case}, corner {corner}, axis {axis}: lower {} > {}",
                        world_min[axis],
                        transformed[axis]
                    );
                    assert!(
                        transformed[axis] <= world_max[axis],
                        "case {case}, corner {corner}, axis {axis}: {} > upper {}",
                        transformed[axis],
                        world_max[axis]
                    );
                }
            }
        }
    }

    #[test]
    fn affine_aabb_fails_open_on_invalid_inputs_and_overflow() {
        let identity = SMatrix::<f64, 2, 2>::identity();
        let zero = SVector::<f64, 2>::zeros();
        let one = SVector::<f64, 2>::from_element(1.0);

        let mut nan_matrix = identity;
        nan_matrix[(0, 1)] = f64::NAN;
        assert_eq!(
            transform_aabb_affine_outward(&nan_matrix, &zero, &zero, &one),
            Err(IntervalError::NonFiniteInput)
        );

        let reversed_min = SVector::from([2.0, 0.0]);
        let reversed_max = SVector::from([1.0, 1.0]);
        assert_eq!(
            transform_aabb_affine_outward(&identity, &zero, &reversed_min, &reversed_max),
            Err(IntervalError::ReversedEndpoints)
        );

        let huge = SMatrix::<f64, 2, 2>::from_element(f64::MAX);
        let huge_min = SVector::<f64, 2>::from_element(1.0);
        let huge_max = SVector::<f64, 2>::from_element(2.0);
        assert_eq!(
            transform_aabb_affine_outward(&huge, &zero, &huge_min, &huge_max),
            Err(IntervalError::ArithmeticOverflow)
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
