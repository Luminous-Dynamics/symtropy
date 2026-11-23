// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Outward-rounded affine-cell geometry for rare-event coverage research.
//!
//! This module provides conservative interval enclosures for affine images of
//! finite axis-aligned cells and a lower bound on the Euclidean norm of the
//! enclosed image. It can prove that a cell cannot intersect a static collision
//! ball when the norm lower bound is strictly greater than the hard-body radius.
//!
//! The crate's residual module composes these primitives into a budgeted
//! static-affine residual-mass traversal. This module itself does not implement
//! nonlinear orbit propagation, time-dependent first contact, or a formally
//! machine-verified residual-mass certificate.

use crate::KernelError;

/// Return a conservative upper bound on the standard-normal probability
/// mass of a finite axis-aligned box.
///
/// For each axis interval, let a be its minimum absolute coordinate. Its
/// standard-normal density is bounded by 0.4 / (1 + a^2/2): the normalizer
/// (2*pi)^(-1/2) is below 0.4, and exp(t) >= 1+t. Independence makes the
/// product of these axis-wise density ceilings a joint density ceiling;
/// multiplying it by the box volume bounds the cell mass. The rational
/// constant 0.4 is deliberately above the exact standard-normal peak.
/// Widths, denominators and products are widened conservatively. This bound
/// is still intentionally loose and is for residual-mass accounting, not
/// precise integration or a machine-verified probability enclosure.
pub fn standard_normal_box_probability_upper_bound(
    lower: &[f64],
    upper: &[f64],
) -> Result<f64, KernelError> {
    if lower.is_empty() || upper.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    if lower.len() != upper.len() {
        return Err(KernelError::DimensionMismatch);
    }

    let mut probability_upper = 1.0_f64;
    for (axis_lower, axis_upper) in lower.iter().zip(upper.iter()) {
        Interval::new(*axis_lower, *axis_upper)?;
        if axis_lower == axis_upper {
            return Ok(0.0);
        }

        let width = axis_upper - axis_lower;
        if !width.is_finite() {
            return Ok(1.0);
        }
        let width_upper = next_up(width);

        let closest_to_zero = if *axis_lower <= 0.0 && *axis_upper >= 0.0 {
            0.0
        } else {
            axis_lower.abs().min(axis_upper.abs())
        };
        let half_square = 0.5 * closest_to_zero * closest_to_zero;
        // Overflow only loses the local tightening; the global 0.4 ceiling
        // remains valid and keeps the result conservative.
        let half_square_lower = if half_square.is_finite() {
            next_down(half_square).max(0.0)
        } else {
            0.0
        };
        let denominator_lower = next_down(1.0 + half_square_lower).max(1.0);
        let axis_density_upper = next_up(0.4 / denominator_lower);
        let density_factor_upper = next_up(probability_upper * axis_density_upper);
        let next_upper = next_up(density_factor_upper * width_upper);
        if !width_upper.is_finite()
            || !denominator_lower.is_finite()
            || !axis_density_upper.is_finite()
            || !density_factor_upper.is_finite()
            || !next_upper.is_finite()
            || next_upper >= 1.0
        {
            return Ok(1.0);
        }
        probability_upper = next_upper;
    }

    Ok(probability_upper.min(1.0))
}

/// Return a conservative union-bound upper bound on the probability that an
/// independent standard-normal vector lies outside the cube [-radius, radius]^d.
///
/// For one coordinate, the Gaussian Chernoff bound gives
/// P(|Z| > radius) <= 2 exp(-radius^2 / 2). A union bound across dimensions
/// gives 2 d exp(-t), where t=radius^2/2. Since
/// exp(t) >= sum_{k=0}^{16} t^k/k! for t >= 0, the implementation lower-bounds
/// that positive polynomial with downward-rounded arithmetic and divides an
/// upward-rounded numerator by it. If the polynomial overflows, it falls back
/// to the weaker but still conservative 2d/(1+t) bound. If even the squared
/// radius is not representable, it returns the trivial upper bound one.
///
/// This is a numerical conservative bound for the stated Gaussian model, not
/// a formal machine-checked proof of the floating-point implementation.
pub fn standard_normal_cube_tail_probability_upper_bound(
    radius: f64,
    dimension: usize,
) -> Result<f64, KernelError> {
    if dimension == 0 {
        return Err(KernelError::EmptyDimension);
    }
    if !radius.is_finite() || radius <= 0.0 {
        return Err(KernelError::InvalidRadius);
    }

    let radius_squared = radius * radius;
    if !radius_squared.is_finite() {
        return Ok(1.0);
    }
    let radius_squared_lower = next_down(radius_squared).max(0.0);
    let t_lower = next_down(0.5 * radius_squared_lower).max(0.0);

    let dimension_upper = next_up(dimension as f64);
    let numerator_upper = next_up(2.0 * dimension_upper);
    if !numerator_upper.is_finite() {
        return Ok(1.0);
    }

    let mut term_lower = 1.0_f64;
    let mut polynomial_lower = 1.0_f64;
    let mut polynomial_overflow = false;
    for order in 1..=16 {
        let product = term_lower * t_lower;
        if !product.is_finite() {
            polynomial_overflow = true;
            break;
        }
        let product_lower = next_down(product).max(0.0);
        let quotient = product_lower / order as f64;
        if !quotient.is_finite() {
            polynomial_overflow = true;
            break;
        }
        term_lower = next_down(quotient).max(0.0);
        let sum = polynomial_lower + term_lower;
        if !sum.is_finite() {
            polynomial_overflow = true;
            break;
        }
        polynomial_lower = next_down(sum).max(polynomial_lower);
    }

    let denominator_lower = if polynomial_overflow {
        next_down(1.0 + t_lower).max(1.0)
    } else {
        polynomial_lower
    };
    let bound = next_up(numerator_upper / denominator_lower);
    if !bound.is_finite() {
        return Ok(1.0);
    }
    Ok(bound.min(1.0))
}

/// Conservative upper bound on the standard-normal probability mass in a
/// declared union of unresolved cells and one truncated-domain complement.
///
/// The caller supplies only cells that remain unresolved after an independent
/// geometric event-exclusion check. Overlapping cells are permitted but make
/// the bound looser. The cube-tail bound covers probability mass outside the
/// declared finite partition domain. The total is capped at one.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct CoverageMassUpperBound {
    total_upper: f64,
}

impl Default for CoverageMassUpperBound {
    fn default() -> Self {
        Self::new()
    }
}

impl CoverageMassUpperBound {
    /// Create an empty unresolved-mass ledger.
    pub fn new() -> Self {
        Self { total_upper: 0.0 }
    }

    /// Add one unresolved finite cell using the conservative density-times-volume bound.
    pub fn add_unresolved_cell(&mut self, cell: &CoverageCell) -> Result<(), KernelError> {
        let cell_upper = standard_normal_box_probability_upper_bound(cell.lower(), cell.upper())?;
        self.add_upper_bound(cell_upper)
    }

    /// Add the conservative Gaussian tail bound outside a standard-normal cube.
    pub fn add_cube_tail(&mut self, radius: f64, dimension: usize) -> Result<(), KernelError> {
        let tail_upper = standard_normal_cube_tail_probability_upper_bound(radius, dimension)?;
        self.add_upper_bound(tail_upper)
    }

    /// Add another already validated probability upper bound.
    ///
    /// This is useful when combining independent unresolved strata. The value
    /// must itself be a conservative upper bound for the declared model.
    pub fn add_upper_bound(&mut self, probability_upper: f64) -> Result<(), KernelError> {
        if !probability_upper.is_finite() || !(0.0..=1.0).contains(&probability_upper) {
            return Err(KernelError::InvalidProbability);
        }
        if probability_upper == 0.0 {
            return Ok(());
        }
        let sum = self.total_upper + probability_upper;
        self.total_upper = if !sum.is_finite() || sum >= 1.0 {
            1.0
        } else {
            next_up(sum).min(1.0)
        };
        Ok(())
    }

    /// Return the accumulated upper bound in [0, 1].
    pub fn upper_bound(&self) -> f64 {
        self.total_upper
    }
}

/// Which half of a split cell a child represents.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CellSide {
    /// Child containing the lower half of the split axis.
    Left,
    /// Child containing the upper half of the split axis.
    Right,
}

/// One deterministic longest-axis subdivision step.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct CoverageSplit {
    /// Axis split by the deterministic subdivision rule.
    pub axis: usize,
    /// Side represented by this child.
    pub side: CellSide,
}

/// A finite closed axis-aligned cell used by the residual-mass coverage search.
///
/// The split path is stable for a fixed root cell and deterministic longest-axis
/// policy. This structure stores geometry only; it does not attach a probability
/// mass, classification, or qualification claim to the cell.
#[derive(Debug, Clone, PartialEq)]
pub struct CoverageCell {
    lower: Vec<f64>,
    upper: Vec<f64>,
    path: Vec<CoverageSplit>,
}

impl CoverageCell {
    /// Construct a finite closed axis-aligned cell.
    pub fn new(lower: Vec<f64>, upper: Vec<f64>) -> Result<Self, KernelError> {
        if lower.is_empty() || upper.is_empty() {
            return Err(KernelError::EmptyDimension);
        }
        if lower.len() != upper.len() {
            return Err(KernelError::DimensionMismatch);
        }
        for (axis_lower, axis_upper) in lower.iter().zip(upper.iter()) {
            Interval::new(*axis_lower, *axis_upper)?;
        }
        Ok(Self {
            lower,
            upper,
            path: Vec::new(),
        })
    }

    /// Return the cell's lower coordinate vector.
    pub fn lower(&self) -> &[f64] {
        &self.lower
    }

    /// Return the cell's upper coordinate vector.
    pub fn upper(&self) -> &[f64] {
        &self.upper
    }

    /// Return the deterministic split path from the root cell.
    pub fn path(&self) -> &[CoverageSplit] {
        &self.path
    }

    /// Split the cell along its longest axis, breaking ties toward the lowest
    /// axis index. Returns None when no representable interior midpoint exists.
    ///
    /// The two closed child cells share the split boundary and together cover
    /// the parent exactly. Their intersection is a measure-zero face for a
    /// nondegenerate continuous Gaussian measure.
    pub fn split_longest_axis(&self) -> Result<Option<(Self, Self)>, KernelError> {
        let mut selected_axis = 0;
        let mut selected_half_width = -1.0_f64;

        for (axis, (lower, upper)) in self.lower.iter().zip(self.upper.iter()).enumerate() {
            let half_width = 0.5 * *upper - 0.5 * *lower;
            if !half_width.is_finite() {
                return Err(KernelError::NumericalOverflow);
            }
            if half_width > selected_half_width {
                selected_half_width = half_width;
                selected_axis = axis;
            }
        }

        if selected_half_width <= 0.0 {
            return Ok(None);
        }

        let lower_endpoint = self.lower[selected_axis];
        let upper_endpoint = self.upper[selected_axis];
        let midpoint = 0.5 * lower_endpoint + 0.5 * upper_endpoint;
        if !midpoint.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        if midpoint <= lower_endpoint || midpoint >= upper_endpoint {
            return Ok(None);
        }

        let mut left_upper = self.upper.clone();
        left_upper[selected_axis] = midpoint;
        let mut right_lower = self.lower.clone();
        right_lower[selected_axis] = midpoint;

        let mut left_path = self.path.clone();
        left_path.push(CoverageSplit {
            axis: selected_axis,
            side: CellSide::Left,
        });
        let mut right_path = self.path.clone();
        right_path.push(CoverageSplit {
            axis: selected_axis,
            side: CellSide::Right,
        });

        Ok(Some((
            Self {
                lower: self.lower.clone(),
                upper: left_upper,
                path: left_path,
            },
            Self {
                lower: right_lower,
                upper: self.upper.clone(),
                path: right_path,
            },
        )))
    }
}

/// A closed finite interval with outward-rounded endpoints.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Interval {
    lower: f64,
    upper: f64,
}

impl Interval {
    /// Construct a closed interval from finite endpoints.
    pub fn new(lower: f64, upper: f64) -> Result<Self, KernelError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(KernelError::NonFiniteValue);
        }
        if lower > upper {
            return Err(KernelError::InvalidInterval);
        }
        Ok(Self { lower, upper })
    }

    /// Return the lower endpoint.
    pub fn lower(self) -> f64 {
        self.lower
    }

    /// Return the upper endpoint.
    pub fn upper(self) -> f64 {
        self.upper
    }

    fn point(value: f64) -> Result<Self, KernelError> {
        Self::new(value, value)
    }

    fn add(self, other: Self) -> Result<Self, KernelError> {
        let lower = self.lower + other.lower;
        let upper = self.upper + other.upper;
        if !lower.is_finite() || !upper.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        Self::widen(lower, upper)
    }

    fn scale(self, coefficient: f64) -> Result<Self, KernelError> {
        if !coefficient.is_finite() {
            return Err(KernelError::NonFiniteValue);
        }
        let (source_lower, source_upper) = if coefficient >= 0.0 {
            (self.lower, self.upper)
        } else {
            (self.upper, self.lower)
        };
        let lower = coefficient * source_lower;
        let upper = coefficient * source_upper;
        if !lower.is_finite() || !upper.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        Self::widen(lower, upper)
    }

    fn widen(lower: f64, upper: f64) -> Result<Self, KernelError> {
        let widened_lower = next_down(lower);
        let widened_upper = next_up(upper);
        if !widened_lower.is_finite() || !widened_upper.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        Self::new(widened_lower, widened_upper)
    }

    fn minimum_absolute_value(self) -> f64 {
        if self.lower <= 0.0 && self.upper >= 0.0 {
            0.0
        } else {
            self.lower.abs().min(self.upper.abs())
        }
    }
}

/// Enclose the affine image offset + matrix * z for every z in an axis-aligned
/// input box.
///
/// The matrix is row-major with offset.len() output rows and
/// input_lower.len() input columns. Each arithmetic operation is widened by
/// one representable f64 value in the appropriate direction. If an operation
/// overflows, the function fails closed instead of producing an exclusion.
pub fn affine_box_image_bounds(
    offset: &[f64],
    matrix: &[f64],
    input_lower: &[f64],
    input_upper: &[f64],
) -> Result<Vec<Interval>, KernelError> {
    let output_dimension = offset.len();
    let input_dimension = input_lower.len();
    if output_dimension == 0 || input_dimension == 0 {
        return Err(KernelError::EmptyDimension);
    }
    if input_upper.len() != input_dimension {
        return Err(KernelError::DimensionMismatch);
    }
    let matrix_length = output_dimension
        .checked_mul(input_dimension)
        .ok_or(KernelError::InvalidMatrixLength)?;
    if matrix.len() != matrix_length {
        return Err(KernelError::InvalidMatrixLength);
    }
    if !offset.iter().all(|value| value.is_finite())
        || !matrix.iter().all(|value| value.is_finite())
    {
        return Err(KernelError::NonFiniteValue);
    }

    let mut input_intervals = Vec::with_capacity(input_dimension);
    for (lower, upper) in input_lower.iter().zip(input_upper.iter()) {
        input_intervals.push(Interval::new(*lower, *upper)?);
    }

    let mut output = Vec::with_capacity(output_dimension);
    for (row, offset_value) in offset.iter().enumerate() {
        let mut image_interval = Interval::point(*offset_value)?;
        for (column, input_interval) in input_intervals.iter().enumerate() {
            let coefficient = matrix[row * input_dimension + column];
            image_interval = image_interval.add(input_interval.scale(coefficient)?)?;
        }
        output.push(image_interval);
    }
    Ok(output)
}

/// Return a conservative lower bound on ||offset + matrix*z|| for every z in
/// the supplied axis-aligned input box.
///
/// The result is useful for static collision-ball exclusion. It is rounded
/// down after multiplication, accumulation, and square root. If the lower
/// bound is strictly greater than HBR, no point in the input cell can lie
/// inside the collision ball of radius HBR.
pub fn affine_box_norm_lower_bound(
    offset: &[f64],
    matrix: &[f64],
    input_lower: &[f64],
    input_upper: &[f64],
) -> Result<f64, KernelError> {
    let image = affine_box_image_bounds(offset, matrix, input_lower, input_upper)?;
    let mut squared_lower_bound = 0.0_f64;

    for coordinate in image {
        let minimum_absolute = coordinate.minimum_absolute_value();
        let absolute_lower = next_down(minimum_absolute).max(0.0);
        let squared = absolute_lower * absolute_lower;
        if !squared.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        let squared_lower = next_down(squared).max(0.0);
        let accumulated = squared_lower_bound + squared_lower;
        if !accumulated.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        squared_lower_bound = next_down(accumulated).max(0.0);
    }

    let norm = squared_lower_bound.sqrt();
    if !norm.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(next_down(norm).max(0.0))
}

/// Determine whether an affine image of an input cell is provably disjoint
/// from a closed collision ball centered at the origin.
///
/// A result of Ok(true) is a conservative exclusion. Ok(false) means only
/// that this interval enclosure could not exclude the cell; the cell may or
/// may not contain an event. Equality at the hard-body radius is never
/// excluded.
pub fn affine_cell_excludes_collision_ball(
    offset: &[f64],
    matrix: &[f64],
    input_lower: &[f64],
    input_upper: &[f64],
    hard_body_radius: f64,
) -> Result<bool, KernelError> {
    if !hard_body_radius.is_finite() || hard_body_radius < 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    let lower_bound = affine_box_norm_lower_bound(offset, matrix, input_lower, input_upper)?;
    Ok(lower_bound > hard_body_radius)
}

fn next_down(value: f64) -> f64 {
    if value.is_nan() || value == f64::NEG_INFINITY {
        return value;
    }
    if value == 0.0 {
        return -f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits - 1 } else { bits + 1 })
}

fn next_up(value: f64) -> f64 {
    if value.is_nan() || value == f64::INFINITY {
        return value;
    }
    if value == 0.0 {
        return f64::from_bits(1);
    }
    let bits = value.to_bits();
    f64::from_bits(if value > 0.0 { bits + 1 } else { bits - 1 })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn affine_cell_outside_collision_ball_is_excluded() {
        assert!(
            affine_cell_excludes_collision_ball(
                &[4.0, 0.0],
                &[1.0, 0.0, 0.0, 1.0],
                &[-1.0, -1.0],
                &[1.0, 1.0],
                1.0,
            )
            .unwrap()
        );
    }

    #[test]
    fn affine_cell_intersecting_collision_ball_is_not_excluded() {
        assert!(
            !affine_cell_excludes_collision_ball(
                &[0.0, 0.0],
                &[1.0, 0.0, 0.0, 1.0],
                &[-0.5, -0.5],
                &[0.5, 0.5],
                1.0,
            )
            .unwrap()
        );
    }

    #[test]
    fn tangency_is_not_excluded() {
        assert!(
            !affine_cell_excludes_collision_ball(
                &[2.0, 0.0],
                &[1.0, 0.0, 0.0, 1.0],
                &[-1.0, 0.0],
                &[-1.0, 0.0],
                1.0,
            )
            .unwrap()
        );
    }

    #[test]
    fn negative_affine_coefficients_are_enclosed() {
        assert!(
            affine_cell_excludes_collision_ball(&[0.0], &[-1.0], &[2.0], &[3.0], 1.0,).unwrap()
        );
    }

    #[test]
    fn affine_image_bounds_are_outward_rounded() {
        let bounds = affine_box_image_bounds(&[0.0], &[1.0], &[1.0], &[1.0]).unwrap();
        assert!(bounds[0].lower() < 1.0);
        assert!(bounds[0].upper() > 1.0);
    }

    #[test]
    fn invalid_dimension_and_matrix_shapes_fail_closed() {
        assert_eq!(
            affine_box_image_bounds(&[], &[], &[], &[]),
            Err(KernelError::EmptyDimension)
        );
        assert_eq!(
            affine_box_image_bounds(&[0.0], &[], &[0.0], &[1.0]),
            Err(KernelError::InvalidMatrixLength)
        );
    }

    #[test]
    fn reversed_or_non_finite_cell_bounds_fail_closed() {
        assert_eq!(
            affine_box_image_bounds(&[0.0], &[1.0], &[1.0], &[0.0]),
            Err(KernelError::InvalidInterval)
        );
        assert_eq!(
            affine_box_image_bounds(&[0.0], &[1.0], &[f64::NEG_INFINITY], &[0.0]),
            Err(KernelError::NonFiniteValue)
        );
    }

    #[test]
    fn overflow_fails_closed_instead_of_excluding_a_cell() {
        assert_eq!(
            affine_cell_excludes_collision_ball(&[0.0], &[f64::MAX], &[2.0], &[3.0], 1.0,),
            Err(KernelError::NumericalOverflow)
        );
    }

    #[test]
    fn affine_enclosure_contains_a_grid_of_transformed_points() {
        let offset = [0.25, -0.5];
        let matrix = [2.0, -1.0, -0.5, 3.0];
        let lower = [-1.0, 0.5];
        let upper = [2.0, 1.5];
        let bounds = affine_box_image_bounds(&offset, &matrix, &lower, &upper).unwrap();

        for i in 0..=20 {
            for j in 0..=20 {
                let x = lower[0] + (upper[0] - lower[0]) * (i as f64 / 20.0);
                let y = lower[1] + (upper[1] - lower[1]) * (j as f64 / 20.0);
                let out_x = offset[0] + matrix[0] * x + matrix[1] * y;
                let out_y = offset[1] + matrix[2] * x + matrix[3] * y;
                assert!(out_x >= bounds[0].lower() && out_x <= bounds[0].upper());
                assert!(out_y >= bounds[1].lower() && out_y <= bounds[1].upper());
            }
        }
    }

    #[test]
    fn affine_norm_lower_bound_never_exceeds_a_sampled_norm() {
        let offset = [0.25, -0.5];
        let matrix = [2.0, -1.0, -0.5, 3.0];
        let lower = [-1.0, 0.5];
        let upper = [2.0, 1.5];
        let bound = affine_box_norm_lower_bound(&offset, &matrix, &lower, &upper).unwrap();

        for i in 0..=20 {
            for j in 0..=20 {
                let x = lower[0] + (upper[0] - lower[0]) * (i as f64 / 20.0);
                let y = lower[1] + (upper[1] - lower[1]) * (j as f64 / 20.0);
                let out_x = offset[0] + matrix[0] * x + matrix[1] * y;
                let out_y = offset[1] + matrix[2] * x + matrix[3] * y;
                let sampled_norm = (out_x * out_x + out_y * out_y).sqrt();
                assert!(bound <= sampled_norm);
            }
        }
    }

    #[test]
    fn coverage_cell_splits_longest_axis_and_preserves_union() {
        let cell = CoverageCell::new(vec![-1.0, -2.0], vec![1.0, 2.0]).unwrap();
        let (left, right) = cell.split_longest_axis().unwrap().unwrap();

        assert_eq!(left.lower(), &[-1.0, -2.0]);
        assert_eq!(left.upper(), &[1.0, 0.0]);
        assert_eq!(right.lower(), &[-1.0, 0.0]);
        assert_eq!(right.upper(), &[1.0, 2.0]);
        assert_eq!(left.path()[0].axis, 1);
        assert_eq!(left.path()[0].side, CellSide::Left);
        assert_eq!(right.path()[0].side, CellSide::Right);
    }

    #[test]
    fn coverage_cell_longest_axis_ties_break_toward_lowest_index() {
        let cell = CoverageCell::new(vec![-1.0, -1.0], vec![1.0, 1.0]).unwrap();
        let (left, right) = cell.split_longest_axis().unwrap().unwrap();
        assert_eq!(left.path()[0].axis, 0);
        assert_eq!(right.path()[0].axis, 0);
    }

    #[test]
    fn coverage_cell_does_not_split_without_interior_float_midpoint() {
        let lower = 1.0_f64;
        let upper = f64::from_bits(lower.to_bits() + 1);
        let cell = CoverageCell::new(vec![lower], vec![upper]).unwrap();
        assert_eq!(cell.split_longest_axis().unwrap(), None);
    }

    #[test]
    fn degenerate_coverage_cell_is_not_split() {
        let cell = CoverageCell::new(vec![1.0, -2.0], vec![1.0, -2.0]).unwrap();
        assert_eq!(cell.split_longest_axis().unwrap(), None);
    }

    #[test]
    fn coverage_cell_rejects_non_finite_or_mismatched_bounds() {
        assert_eq!(
            CoverageCell::new(vec![0.0], vec![f64::INFINITY]),
            Err(KernelError::NonFiniteValue)
        );
        assert_eq!(
            CoverageCell::new(vec![0.0, 0.0], vec![1.0]),
            Err(KernelError::DimensionMismatch)
        );
    }

    #[test]
    fn standard_normal_box_density_volume_bound_is_conservative() {
        let upper = standard_normal_box_probability_upper_bound(&[0.0], &[0.1]).unwrap();
        assert!(upper > 0.0399);
        assert!(upper < 0.0401);
    }

    #[test]
    fn standard_normal_box_mass_bound_tightens_away_from_zero() {
        let central = standard_normal_box_probability_upper_bound(&[-0.5], &[0.5]).unwrap();
        let tail = standard_normal_box_probability_upper_bound(&[3.0], &[4.0]).unwrap();
        assert!(tail < central);
        assert!(tail < 0.08);
    }

    #[test]
    fn standard_normal_box_mass_bound_handles_degenerate_axis() {
        assert_eq!(
            standard_normal_box_probability_upper_bound(&[0.0, -1.0], &[1.0, -1.0]).unwrap(),
            0.0
        );
    }

    #[test]
    fn standard_normal_box_mass_bound_rejects_invalid_cells() {
        assert_eq!(
            standard_normal_box_probability_upper_bound(&[1.0], &[0.0]),
            Err(KernelError::InvalidInterval)
        );
        assert_eq!(
            standard_normal_box_probability_upper_bound(&[0.0, 0.0], &[1.0]),
            Err(KernelError::DimensionMismatch)
        );
    }

    #[test]
    fn cube_tail_mass_bound_decreases_with_radius() {
        let moderate = standard_normal_cube_tail_probability_upper_bound(5.0, 6).unwrap();
        let large = standard_normal_cube_tail_probability_upper_bound(8.0, 6).unwrap();
        assert!(large < moderate);
        assert!(large < 1.0e-8);
    }

    #[test]
    fn cube_tail_bound_handles_small_radius_and_large_dimension_conservatively() {
        assert_eq!(
            standard_normal_cube_tail_probability_upper_bound(0.01, 3).unwrap(),
            1.0
        );
        assert!(standard_normal_cube_tail_probability_upper_bound(2.0, 12).unwrap() <= 1.0);
    }

    #[test]
    fn coverage_mass_ledger_accumulates_cells_and_tail() {
        let mut ledger = CoverageMassUpperBound::new();
        let cell_a = CoverageCell::new(vec![0.0], vec![0.1]).unwrap();
        let cell_b = CoverageCell::new(vec![0.2], vec![0.3]).unwrap();
        ledger.add_unresolved_cell(&cell_a).unwrap();
        ledger.add_unresolved_cell(&cell_b).unwrap();
        let cells_only = ledger.upper_bound();
        assert!(cells_only > 0.07);
        assert!(cells_only < 0.09);
        ledger.add_cube_tail(5.0, 1).unwrap();
        assert!(ledger.upper_bound() >= cells_only);
        assert!(ledger.upper_bound() <= 1.0);
    }

    #[test]
    fn coverage_mass_ledger_caps_sum_at_one() {
        let mut ledger = CoverageMassUpperBound::new();
        ledger.add_upper_bound(0.75).unwrap();
        ledger.add_upper_bound(0.75).unwrap();
        assert_eq!(ledger.upper_bound(), 1.0);
    }

    #[test]
    fn coverage_mass_ledger_rejects_invalid_probability_bound() {
        let mut ledger = CoverageMassUpperBound::new();
        assert_eq!(
            ledger.add_upper_bound(f64::NAN),
            Err(KernelError::InvalidProbability)
        );
    }

    #[test]
    fn invalid_hard_body_radius_fails_closed() {
        assert_eq!(
            affine_cell_excludes_collision_ball(&[2.0], &[1.0], &[0.0], &[0.0], f64::NAN,),
            Err(KernelError::InvalidHardBodyRadius)
        );
        assert_eq!(
            affine_cell_excludes_collision_ball(&[2.0], &[1.0], &[0.0], &[0.0], -1.0,),
            Err(KernelError::InvalidHardBodyRadius)
        );
    }
}
