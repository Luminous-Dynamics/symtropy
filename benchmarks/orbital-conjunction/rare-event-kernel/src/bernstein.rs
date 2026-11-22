// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bernstein-form enclosures for polynomial relative-position trajectories.
//!
//! Power-basis polynomials can have very loose interval-Horner ranges because
//! repeated occurrences of time are treated independently. Transforming the
//! polynomial to Bernstein form on a closed interval gives a convex-hull
//! enclosure. de Casteljau subdivision tightens that enclosure while
//! preserving the complete time-window contract.
//!
//! Coefficient intervals must already enclose the exact polynomial
//! coefficients and any caller-supplied Taylor / model remainder. This module
//! does not construct that remainder, validate nonlinear dynamics, or provide
//! formally machine-verified arithmetic. Its adjacent-float expansion is
//! numerical bookkeeping and must not be described as a validated Taylor model.

use crate::{Interval, KernelError, TrajectoryContactClassification};

/// Global minimum-distance bounds for a family of interval-coefficient
/// polynomial trajectories over a complete closed time window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BernsteinTrajectoryReport {
    /// Three-way contact disposition. A contact result has a common-time
    /// witness; a no-contact result requires every time leaf to be excluded.
    pub classification: TrajectoryContactClassification,
    /// Lower bound on the global minimum distance over the processed window.
    /// This is zero when the time partition was left incomplete by the budget.
    pub minimum_distance_lower_bound: f64,
    /// Uniform upper bound on the minimum distance for every represented
    /// coefficient choice, obtained by evaluating at a common candidate time.
    pub minimum_distance_upper_bound: f64,
    /// Candidate time attaining the reported uniform upper witness.
    pub upper_bound_witness_time: f64,
    /// Number of Bernstein time cells processed.
    pub visited_time_intervals: usize,
    /// Number of leaves proven strictly outside the collision ball.
    pub excluded_time_intervals: usize,
    /// Number of unresolved leaves plus unprocessed cells left by the budget.
    pub unresolved_time_intervals: usize,
    /// True when the traversal budget left part of the partition unprocessed.
    pub budget_exhausted: bool,
    /// True when every queued cell was processed (unresolved max-depth leaves
    /// may still remain when this field is true).
    pub partition_complete: bool,
}

#[derive(Debug, Clone, Copy)]
/// Internal interval bound used by Bernstein and first-contact enclosures.
pub(crate) struct Bound {
    lower: f64,
    upper: f64,
}

impl Bound {
    fn point(value: f64) -> Result<Self, KernelError> {
        if !value.is_finite() {
            return Err(KernelError::NonFiniteValue);
        }
        Ok(Self {
            lower: value,
            upper: value,
        })
    }

    fn from_interval(value: Interval) -> Self {
        Self {
            lower: value.lower(),
            upper: value.upper(),
        }
    }

    fn add(self, other: Self) -> Result<Self, KernelError> {
        let lower = self.lower + other.lower;
        let upper = self.upper + other.upper;
        Self::outward(lower, upper)
    }

    fn subtract(self, other: Self) -> Result<Self, KernelError> {
        let lower = self.lower - other.upper;
        let upper = self.upper - other.lower;
        Self::outward(lower, upper)
    }

    fn multiply(self, other: Self) -> Result<Self, KernelError> {
        let products = [
            self.lower * other.lower,
            self.lower * other.upper,
            self.upper * other.lower,
            self.upper * other.upper,
        ];
        if products.iter().any(|value| !value.is_finite()) {
            return Err(KernelError::NumericalOverflow);
        }
        let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
        let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
        Self::outward(lower, upper)
    }

    fn scale(self, value: f64) -> Result<Self, KernelError> {
        self.multiply(Self::point(value)?)
    }

    fn divide_positive_scalar(self, value: f64) -> Result<Self, KernelError> {
        if !value.is_finite() || value <= 0.0 {
            return Err(KernelError::NumericalOverflow);
        }
        Self::outward(self.lower / value, self.upper / value)
    }

    fn divide_positive(self, denominator: Self) -> Result<Self, KernelError> {
        if denominator.lower <= 0.0 {
            return Err(KernelError::NumericalOverflow);
        }
        let reciprocal = Self::outward(1.0 / denominator.upper, 1.0 / denominator.lower)?;
        self.multiply(reciprocal)
    }

    fn midpoint(self, other: Self) -> Result<Self, KernelError> {
        self.scale(0.5)?.add(other.scale(0.5)?)
    }

    fn outward(lower: f64, upper: f64) -> Result<Self, KernelError> {
        if !lower.is_finite() || !upper.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        let widened_lower = next_down(lower);
        let widened_upper = next_up(upper);
        if !widened_lower.is_finite() || !widened_upper.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        Ok(Self {
            lower: widened_lower,
            upper: widened_upper,
        })
    }
}

#[derive(Debug, Clone)]
struct TimeCell {
    lower: f64,
    upper: f64,
    depth: usize,
    coordinates: Vec<Vec<Bound>>,
}

/// Enclose the global minimum distance of interval-coefficient polynomial
/// relative-position coordinates over a complete closed time window.
///
/// coefficients[axis][power] gives the interval coefficient of t^power in
/// ascending power order. The coefficient intervals must contain the exact
/// coefficients and any approximation remainder for every represented
/// trajectory. Independent coefficient intervals may over-approximate
/// correlations; this can increase ambiguity but does not narrow the set.
///
/// The function transforms each coordinate to Bernstein form over the full
/// time window. A polynomial's values lie in the convex hull of its Bernstein
/// coefficients. It adaptively bisects with de Casteljau subdivision and
/// evaluates common candidate times to obtain a uniform upper witness.
///
/// DefinitelyNoContact requires every leaf's Euclidean norm lower bound to be
/// strictly greater than hard_body_radius. DefinitelyContact requires one
/// candidate time whose Euclidean norm upper bound is strictly less than the
/// radius for every represented coefficient choice. Tangency, unresolved
/// leaves, or budget exhaustion remain Ambiguous.
///
/// This is a polynomial enclosure, not a Taylor-model constructor or a
/// validated nonlinear ODE solver. Floating-point operations are widened by
/// one adjacent representable value; that arithmetic is not formally verified.
pub fn enclose_polynomial_minimum_distance(
    coefficients: &[Vec<Interval>],
    time_lower: f64,
    time_upper: f64,
    hard_body_radius: f64,
    max_time_intervals: usize,
    max_depth: usize,
) -> Result<BernsteinTrajectoryReport, KernelError> {
    if coefficients.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    if coefficients.iter().any(Vec::is_empty) {
        return Err(KernelError::InvalidPolynomial);
    }
    if !time_lower.is_finite() || !time_upper.is_finite() {
        return Err(KernelError::NonFiniteValue);
    }
    if time_lower > time_upper {
        return Err(KernelError::InvalidInterval);
    }
    if !hard_body_radius.is_finite() || hard_body_radius < 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    if max_time_intervals == 0 {
        return Err(KernelError::InvalidTrajectoryBudget);
    }

    let mut root_coordinates = Vec::with_capacity(coefficients.len());
    for axis in coefficients {
        root_coordinates.push(power_to_bernstein(axis, time_lower, time_upper)?);
    }

    let mut stack = vec![TimeCell {
        lower: time_lower,
        upper: time_upper,
        depth: 0,
        coordinates: root_coordinates,
    }];
    let mut visited_time_intervals = 0_usize;
    let mut excluded_time_intervals = 0_usize;
    let mut unresolved_time_intervals = 0_usize;
    let mut minimum_distance_lower_bound = f64::INFINITY;
    let mut minimum_distance_upper_bound = f64::INFINITY;
    let mut upper_bound_witness_time = time_lower;
    let mut budget_exhausted = false;

    while !stack.is_empty() {
        if visited_time_intervals >= max_time_intervals {
            budget_exhausted = true;
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(stack.len())
                .ok_or(KernelError::CounterOverflow)?;
            break;
        }
        let cell = stack.pop().ok_or(KernelError::InvalidInterval)?;
        visited_time_intervals = visited_time_intervals
            .checked_add(1)
            .ok_or(KernelError::CounterOverflow)?;

        // Any fixed time in the cell is an upper witness for the minimum.
        // The interval evaluation encloses every represented coefficient
        // choice at that same time.
        let midpoint = safe_midpoint(cell.lower, cell.upper)?;
        for (unit_time, absolute_time) in [(0.0, cell.lower), (0.5, midpoint), (1.0, cell.upper)] {
            let mut point_coordinates = Vec::with_capacity(cell.coordinates.len());
            for axis in &cell.coordinates {
                point_coordinates.push(evaluate_bernstein(axis, unit_time)?);
            }
            let (_, candidate_upper) = norm_bounds(&point_coordinates)?;
            if candidate_upper < minimum_distance_upper_bound {
                minimum_distance_upper_bound = candidate_upper;
                upper_bound_witness_time = absolute_time;
            }
        }

        let mut coordinate_ranges = Vec::with_capacity(cell.coordinates.len());
        for axis in &cell.coordinates {
            coordinate_ranges.push(bernstein_hull(axis)?);
        }
        let (cell_distance_lower, _) = norm_bounds(&coordinate_ranges)?;

        if cell_distance_lower > hard_body_radius {
            minimum_distance_lower_bound = minimum_distance_lower_bound.min(cell_distance_lower);
            excluded_time_intervals = excluded_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            continue;
        }

        if cell.depth >= max_depth {
            minimum_distance_lower_bound = minimum_distance_lower_bound.min(cell_distance_lower);
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            continue;
        }

        if midpoint <= cell.lower || midpoint >= cell.upper {
            minimum_distance_lower_bound = minimum_distance_lower_bound.min(cell_distance_lower);
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            continue;
        }

        let mut left_coordinates = Vec::with_capacity(cell.coordinates.len());
        let mut right_coordinates = Vec::with_capacity(cell.coordinates.len());
        for axis in &cell.coordinates {
            let (left, right) = split_bernstein_half(axis)?;
            left_coordinates.push(left);
            right_coordinates.push(right);
        }

        // Stable left-first traversal: push right before left.
        stack.push(TimeCell {
            lower: midpoint,
            upper: cell.upper,
            depth: cell.depth + 1,
            coordinates: right_coordinates,
        });
        stack.push(TimeCell {
            lower: cell.lower,
            upper: midpoint,
            depth: cell.depth + 1,
            coordinates: left_coordinates,
        });
    }

    let partition_complete = stack.is_empty();
    if budget_exhausted {
        // Unprocessed intervals may contain an event anywhere in the
        // coordinate space. Zero is the only unconditional distance lower
        // bound for those cells.
        minimum_distance_lower_bound = 0.0;
    }
    if !minimum_distance_lower_bound.is_finite() {
        // This can only occur for an empty traversal, which validation above
        // forbids, but keep the report fail-closed if invariants change.
        minimum_distance_lower_bound = 0.0;
    }

    let classification = if minimum_distance_upper_bound < hard_body_radius {
        // One common candidate time is inside HBR for every coefficient choice.
        TrajectoryContactClassification::DefinitelyContact
    } else if !budget_exhausted
        && unresolved_time_intervals == 0
        && minimum_distance_lower_bound > hard_body_radius
    {
        TrajectoryContactClassification::DefinitelyNoContact
    } else {
        TrajectoryContactClassification::Ambiguous
    };

    Ok(BernsteinTrajectoryReport {
        classification,
        minimum_distance_lower_bound,
        minimum_distance_upper_bound,
        upper_bound_witness_time,
        visited_time_intervals,
        excluded_time_intervals,
        unresolved_time_intervals,
        budget_exhausted,
        partition_complete,
    })
}

/// Convert an interval power-basis polynomial to Bernstein coefficients on a time cell.
pub(crate) fn power_to_bernstein(
    coefficients: &[Interval],
    time_lower: f64,
    time_upper: f64,
) -> Result<Vec<Bound>, KernelError> {
    if coefficients.is_empty() {
        return Err(KernelError::InvalidPolynomial);
    }
    let degree = coefficients.len() - 1;
    let origin = Bound::point(time_lower)?;
    let width = Bound::point(time_upper)?.subtract(origin)?;

    let mut origin_powers = Vec::with_capacity(degree + 1);
    let mut width_powers = Vec::with_capacity(degree + 1);
    origin_powers.push(Bound::point(1.0)?);
    width_powers.push(Bound::point(1.0)?);
    for power in 1..=degree {
        origin_powers.push(origin_powers[power - 1].multiply(origin)?);
        width_powers.push(width_powers[power - 1].multiply(width)?);
    }

    // First form q(u) = p(time_lower + (time_upper-time_lower) * u)
    // in the power basis, then convert q to Bernstein form of the same degree.
    let mut transformed_power = Vec::with_capacity(degree + 1);
    for power in 0..=degree {
        let mut sum = Bound::point(0.0)?;
        for source_power in power..=degree {
            let term = Bound::from_interval(coefficients[source_power])
                .multiply(binomial_bound(source_power, power)?)?
                .multiply(origin_powers[source_power - power])?
                .multiply(width_powers[power])?;
            sum = sum.add(term)?;
        }
        transformed_power.push(sum);
    }

    let mut bernstein = Vec::with_capacity(degree + 1);
    for index in 0..=degree {
        let mut sum = Bound::point(0.0)?;
        for (power, coefficient) in transformed_power.iter().enumerate().take(index + 1) {
            let numerator = binomial_bound(index, power)?;
            let denominator = binomial_bound(degree, power)?;
            let ratio = numerator.divide_positive(denominator)?;
            sum = sum.add(coefficient.multiply(ratio)?)?;
        }
        bernstein.push(sum);
    }
    Ok(bernstein)
}

fn binomial_bound(n: usize, k: usize) -> Result<Bound, KernelError> {
    if k > n {
        return Bound::point(0.0);
    }
    let k = k.min(n - k);
    let mut result = Bound::point(1.0)?;
    for step in 1..=k {
        let numerator_integer = n - k + step;
        let numerator = numerator_integer as f64;
        let denominator = step as f64;
        if numerator as usize != numerator_integer || denominator as usize != step {
            return Err(KernelError::NumericalOverflow);
        }
        result = result
            .scale(numerator)?
            .divide_positive_scalar(denominator)?;
    }
    Ok(result)
}

/// Return the coordinate-wise convex hull of interval Bernstein coefficients.
pub(crate) fn bernstein_hull(coefficients: &[Bound]) -> Result<Bound, KernelError> {
    if coefficients.is_empty() {
        return Err(KernelError::InvalidPolynomial);
    }
    let lower = coefficients
        .iter()
        .map(|coefficient| coefficient.lower)
        .fold(f64::INFINITY, f64::min);
    let upper = coefficients
        .iter()
        .map(|coefficient| coefficient.upper)
        .fold(f64::NEG_INFINITY, f64::max);
    if !lower.is_finite() || !upper.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok(Bound { lower, upper })
}

/// Evaluate interval Bernstein coefficients at a normalized time in [0, 1].
pub(crate) fn evaluate_bernstein(
    coefficients: &[Bound],
    unit_time: f64,
) -> Result<Bound, KernelError> {
    if coefficients.is_empty() {
        return Err(KernelError::InvalidPolynomial);
    }
    if !unit_time.is_finite() || !(0.0..=1.0).contains(&unit_time) {
        return Err(KernelError::InvalidInterval);
    }
    if unit_time == 0.0 {
        return Ok(coefficients[0]);
    }
    if unit_time == 1.0 {
        return coefficients
            .last()
            .copied()
            .ok_or(KernelError::InvalidPolynomial);
    }
    let weight = Bound::point(unit_time)?;
    let complement = Bound::point(1.0 - unit_time)?;
    let mut work = coefficients.to_vec();
    for remaining in (1..coefficients.len()).rev() {
        for index in 0..remaining {
            work[index] = work[index]
                .multiply(complement)?
                .add(work[index + 1].multiply(weight)?)?;
        }
    }
    Ok(work[0])
}

/// Split an interval Bernstein polynomial at normalized time one half.
pub(crate) fn split_bernstein_half(
    coefficients: &[Bound],
) -> Result<(Vec<Bound>, Vec<Bound>), KernelError> {
    if coefficients.is_empty() {
        return Err(KernelError::InvalidPolynomial);
    }
    let degree = coefficients.len() - 1;
    let mut work = coefficients.to_vec();
    let mut left = vec![Bound::point(0.0)?; degree + 1];
    let mut right = vec![Bound::point(0.0)?; degree + 1];
    left[0] = work[0];
    right[degree] = work[degree];

    for level in 1..=degree {
        for index in 0..=(degree - level) {
            work[index] = work[index].midpoint(work[index + 1])?;
        }
        left[level] = work[0];
        right[degree - level] = work[degree - level];
    }
    Ok((left, right))
}

/// Bound the minimum and maximum Euclidean norms over interval coordinates.
pub(crate) fn norm_bounds(coordinates: &[Bound]) -> Result<(f64, f64), KernelError> {
    if coordinates.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    let mut squared_lower = 0.0_f64;
    let mut squared_upper = 0.0_f64;

    for coordinate in coordinates {
        let minimum_absolute = if coordinate.lower <= 0.0 && coordinate.upper >= 0.0 {
            0.0
        } else {
            coordinate.lower.abs().min(coordinate.upper.abs())
        };
        let maximum_absolute = coordinate.lower.abs().max(coordinate.upper.abs());
        let minimum_square = minimum_absolute * minimum_absolute;
        let maximum_square = maximum_absolute * maximum_absolute;
        if !minimum_square.is_finite() || !maximum_square.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        let lower_term = next_down(minimum_square).max(0.0);
        let upper_term = next_up(maximum_square);
        let lower_sum = squared_lower + lower_term;
        let upper_sum = squared_upper + upper_term;
        if !lower_sum.is_finite() || !upper_sum.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        squared_lower = next_down(lower_sum).max(0.0);
        squared_upper = next_up(upper_sum);
    }

    let lower = next_down(squared_lower.sqrt()).max(0.0);
    let upper = next_up(squared_upper.sqrt());
    if !lower.is_finite() || !upper.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((lower, upper))
}

/// Compute a finite midpoint without overflowing on large same-sign endpoints.
pub(crate) fn safe_midpoint(lower: f64, upper: f64) -> Result<f64, KernelError> {
    let midpoint = 0.5 * lower + 0.5 * upper;
    if midpoint.is_finite() {
        Ok(midpoint)
    } else {
        Err(KernelError::NumericalOverflow)
    }
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

    fn interval(lower: f64, upper: f64) -> Interval {
        Interval::new(lower, upper).expect("valid interval")
    }

    #[test]
    fn bernstein_subdivision_proves_dependency_heavy_no_contact() {
        let report = enclose_polynomial_minimum_distance(
            &[vec![
                interval(1.0, 1.0),
                interval(0.0, 0.0),
                interval(1.0, 1.0),
            ]],
            -1.0,
            1.0,
            0.5,
            64,
            8,
        )
        .expect("valid polynomial");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::DefinitelyNoContact
        );
        assert!(report.minimum_distance_lower_bound > 0.5);
        assert!(report.partition_complete);
        assert_eq!(report.unresolved_time_intervals, 0);
        assert!(report.visited_time_intervals > 1);
    }

    #[test]
    fn bernstein_common_midpoint_witness_proves_interior_contact() {
        let report = enclose_polynomial_minimum_distance(
            &[vec![
                interval(0.0, 0.0),
                interval(0.0, 0.0),
                interval(1.0, 1.0),
            ]],
            -1.0,
            1.0,
            0.25,
            64,
            8,
        )
        .expect("valid polynomial");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
        assert!(report.minimum_distance_upper_bound < 0.25);
        assert_eq!(report.upper_bound_witness_time, 0.0);
    }

    #[test]
    fn exact_tangency_remains_ambiguous() {
        let report = enclose_polynomial_minimum_distance(
            &[vec![
                interval(1.0, 1.0),
                interval(0.0, 0.0),
                interval(1.0, 1.0),
            ]],
            -1.0,
            1.0,
            1.0,
            64,
            8,
        )
        .expect("valid polynomial");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::Ambiguous
        );
    }

    #[test]
    fn endpoint_contact_has_a_common_time_witness() {
        let report = enclose_polynomial_minimum_distance(
            &[vec![interval(0.2, 0.2), interval(1.0, 1.0)]],
            0.0,
            1.0,
            0.3,
            16,
            4,
        )
        .expect("valid polynomial");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
        assert_eq!(report.upper_bound_witness_time, 0.0);
    }

    #[test]
    fn coefficient_uncertainty_is_included_in_contact_witness() {
        let report =
            enclose_polynomial_minimum_distance(&[vec![interval(0.1, 0.2)]], -3.0, 7.0, 0.3, 16, 4)
                .expect("valid interval coefficient");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
    }

    #[test]
    fn multiple_axes_use_euclidean_distance() {
        let report = enclose_polynomial_minimum_distance(
            &[
                vec![interval(0.8, 0.8)],
                vec![interval(0.8, 0.8)],
                vec![interval(0.0, 0.0)],
            ],
            -1.0,
            1.0,
            1.0,
            16,
            4,
        )
        .expect("valid vector polynomial");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::DefinitelyNoContact
        );
    }

    #[test]
    fn negative_shifted_time_window_is_supported() {
        let report = enclose_polynomial_minimum_distance(
            &[vec![
                interval(1.0, 1.0),
                interval(0.0, 0.0),
                interval(1.0, 1.0),
            ]],
            -3.0,
            -1.0,
            0.5,
            64,
            8,
        )
        .expect("valid shifted window");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::DefinitelyNoContact
        );
    }

    #[test]
    fn budget_exhaustion_cannot_become_no_contact() {
        let report = enclose_polynomial_minimum_distance(
            &[vec![
                interval(1.0, 1.0),
                interval(0.0, 0.0),
                interval(1.0, 1.0),
            ]],
            -1.0,
            1.0,
            0.5,
            1,
            8,
        )
        .expect("valid polynomial");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::Ambiguous
        );
        assert!(report.budget_exhausted);
        assert!(!report.partition_complete);
        assert_eq!(report.minimum_distance_lower_bound, 0.0);
    }

    #[test]
    fn point_window_is_supported() {
        let report =
            enclose_polynomial_minimum_distance(&[vec![interval(0.25, 0.25)]], 2.0, 2.0, 0.5, 4, 0)
                .expect("valid point window");
        assert_eq!(
            report.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
    }

    #[test]
    fn invalid_inputs_and_zero_budget_fail_closed() {
        assert_eq!(
            enclose_polynomial_minimum_distance(&[], 0.0, 1.0, 1.0, 4, 4),
            Err(KernelError::EmptyDimension)
        );
        assert_eq!(
            enclose_polynomial_minimum_distance(&[vec![]], 0.0, 1.0, 1.0, 4, 4),
            Err(KernelError::InvalidPolynomial)
        );
        assert_eq!(
            enclose_polynomial_minimum_distance(&[vec![interval(1.0, 1.0)]], 1.0, 0.0, 1.0, 4, 4),
            Err(KernelError::InvalidInterval)
        );
        assert_eq!(
            enclose_polynomial_minimum_distance(&[vec![interval(1.0, 1.0)]], 0.0, 1.0, 1.0, 0, 4),
            Err(KernelError::InvalidTrajectoryBudget)
        );
    }

    #[test]
    fn dense_grid_values_are_inside_bernstein_hull() {
        let power = [
            interval(3.0, 3.0),
            interval(-2.0, -2.0),
            interval(5.0, 5.0),
            interval(7.0, 7.0),
        ];
        let bernstein = power_to_bernstein(&power, -2.0, 3.0).expect("conversion");
        let hull = bernstein_hull(&bernstein).expect("hull");
        for step in 0..=1000 {
            let time = -2.0 + 5.0 * step as f64 / 1000.0;
            let value = 3.0 - 2.0 * time + 5.0 * time.powi(2) + 7.0 * time.powi(3);
            assert!(value >= hull.lower && value <= hull.upper);
        }
    }
}
