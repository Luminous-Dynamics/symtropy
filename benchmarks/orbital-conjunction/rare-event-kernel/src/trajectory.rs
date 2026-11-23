// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Conservative time-window classification for interval-enclosed trajectory polynomials.
//!
//! The classifier accepts coordinate-wise polynomial coefficients that are
//! already supplied as closed intervals. It evaluates each polynomial over a
//! closed time interval with outward-expanded interval Horner arithmetic.
//! Time subdivision can prove:
//! - no contact, when every time cell has minimum possible distance > HBR;
//! - guaranteed contact, when one time cell is wholly inside HBR for every
//!   coefficient choice;
//! - ambiguous, when the budget / depth is exhausted or the enclosure keeps
//!   intersecting the collision boundary.
//!
//! Coefficient intervals must themselves enclose the exact trajectory
//! coefficients and any Taylor remainder over the declared state chart. This
//! module does not construct or validate that remainder, prove floating-point
//! operations formally, or model nonlinear dynamics. Ambiguous is a first-class
//! outcome and must escalate to a higher-fidelity evaluator.

use crate::{Interval, KernelError};

/// Classification of a family of interval-enclosed polynomial trajectories.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrajectoryContactClassification {
    /// Every represented trajectory remains strictly outside the collision ball.
    DefinitelyNoContact,
    /// Every represented trajectory enters the collision ball during one
    /// common, nonempty time interval.
    DefinitelyContact,
    /// The enclosures cannot establish either outcome within the declared budget.
    Ambiguous,
}

/// Result of classifying a complete closed time window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrajectoryContactReport {
    /// The universal contact disposition for the supplied coefficient intervals.
    pub classification: TrajectoryContactClassification,
    /// Number of time intervals whose polynomial enclosure was evaluated.
    pub visited_time_intervals: usize,
    /// Number of time intervals proven strictly outside the collision ball.
    pub excluded_time_intervals: usize,
    /// Number of time intervals retained because contact could not be decided.
    pub unresolved_time_intervals: usize,
    /// True when the interval budget left part of the time window unprocessed.
    pub budget_exhausted: bool,
    /// A common interval over which all represented trajectories are inside
    /// the collision ball, when contact was proved.
    pub guaranteed_contact_interval: Option<(f64, f64)>,
}

#[derive(Debug, Clone, Copy)]
struct TimeCell {
    lower: f64,
    upper: f64,
    depth: usize,
}

/// Classify contact over a full time window for interval-enclosed polynomial
/// relative-position coordinates.
///
/// coefficients[axis][power] is the closed interval containing the
/// coefficient of t^power for one relative-position coordinate. Coefficients
/// are in ascending power order. Different axes may have different polynomial
/// degrees.
///
/// The intervals must enclose the exact polynomial coefficients and any
/// approximation remainder over the complete state chart being classified.
/// Correlations between coefficients may be omitted: treating each interval
/// independently forms a superset and therefore may make the result ambiguous,
/// but must not make a valid outside/contact proof less conservative.
///
/// The classifier partitions the entire closed interval [time_lower,
/// time_upper]. A time cell is excluded only when its lower distance bound is
/// strictly greater than hard_body_radius. Contact is proved only when one
/// complete time cell has an upper distance bound strictly below the radius.
/// Tangency and boundary overlap remain ambiguous.
///
/// This is a local-polynomial component, not a complete Taylor-model or
/// validated-ODE implementation. Its interval arithmetic uses adjacent-float
/// widening and is not formally machine-verified.
pub fn classify_polynomial_contact(
    coefficients: &[Vec<Interval>],
    time_lower: f64,
    time_upper: f64,
    hard_body_radius: f64,
    max_time_intervals: usize,
    max_depth: usize,
) -> Result<TrajectoryContactReport, KernelError> {
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

    let mut stack = vec![TimeCell {
        lower: time_lower,
        upper: time_upper,
        depth: 0,
    }];
    let mut visited_time_intervals = 0_usize;
    let mut excluded_time_intervals = 0_usize;
    let mut unresolved_time_intervals = 0_usize;
    let mut budget_exhausted = false;

    while let Some(cell) = stack.pop() {
        if visited_time_intervals >= max_time_intervals {
            budget_exhausted = true;
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(1 + stack.len())
                .ok_or(KernelError::CounterOverflow)?;
            break;
        }
        visited_time_intervals = visited_time_intervals
            .checked_add(1)
            .ok_or(KernelError::CounterOverflow)?;

        let time = Interval::new(cell.lower, cell.upper)?;
        let mut coordinate_ranges = Vec::with_capacity(coefficients.len());
        for axis_coefficients in coefficients {
            coordinate_ranges.push(evaluate_interval_polynomial(axis_coefficients, time)?);
        }
        let (distance_lower, distance_upper) = euclidean_norm_bounds(&coordinate_ranges)?;

        if distance_lower > hard_body_radius {
            excluded_time_intervals = excluded_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            continue;
        }

        if distance_upper < hard_body_radius {
            return Ok(TrajectoryContactReport {
                classification: TrajectoryContactClassification::DefinitelyContact,
                visited_time_intervals,
                excluded_time_intervals,
                unresolved_time_intervals,
                budget_exhausted: false,
                guaranteed_contact_interval: Some((cell.lower, cell.upper)),
            });
        }

        if cell.depth >= max_depth {
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            continue;
        }

        let midpoint = 0.5 * cell.lower + 0.5 * cell.upper;
        if !midpoint.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        if midpoint <= cell.lower || midpoint >= cell.upper {
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            continue;
        }

        // Stable left-first traversal: push right before left.
        stack.push(TimeCell {
            lower: midpoint,
            upper: cell.upper,
            depth: cell.depth + 1,
        });
        stack.push(TimeCell {
            lower: cell.lower,
            upper: midpoint,
            depth: cell.depth + 1,
        });
    }

    let classification = if unresolved_time_intervals > 0 || budget_exhausted {
        TrajectoryContactClassification::Ambiguous
    } else {
        TrajectoryContactClassification::DefinitelyNoContact
    };

    Ok(TrajectoryContactReport {
        classification,
        visited_time_intervals,
        excluded_time_intervals,
        unresolved_time_intervals,
        budget_exhausted,
        guaranteed_contact_interval: None,
    })
}

fn evaluate_interval_polynomial(
    coefficients: &[Interval],
    time: Interval,
) -> Result<Interval, KernelError> {
    let mut value = *coefficients.last().ok_or(KernelError::InvalidPolynomial)?;
    for coefficient in coefficients[..coefficients.len() - 1].iter().rev() {
        value = interval_add(interval_multiply(value, time)?, *coefficient)?;
    }
    Ok(value)
}

fn interval_add(left: Interval, right: Interval) -> Result<Interval, KernelError> {
    let lower = left.lower() + right.lower();
    let upper = left.upper() + right.upper();
    if !lower.is_finite() || !upper.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    outward_interval(lower, upper)
}

fn interval_multiply(left: Interval, right: Interval) -> Result<Interval, KernelError> {
    let products = [
        left.lower() * right.lower(),
        left.lower() * right.upper(),
        left.upper() * right.lower(),
        left.upper() * right.upper(),
    ];
    if products.iter().any(|value| !value.is_finite()) {
        return Err(KernelError::NumericalOverflow);
    }
    let lower = products.iter().copied().fold(f64::INFINITY, f64::min);
    let upper = products.iter().copied().fold(f64::NEG_INFINITY, f64::max);
    outward_interval(lower, upper)
}

fn outward_interval(lower: f64, upper: f64) -> Result<Interval, KernelError> {
    let widened_lower = next_down(lower);
    let widened_upper = next_up(upper);
    if !widened_lower.is_finite() || !widened_upper.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Interval::new(widened_lower, widened_upper)
}

fn euclidean_norm_bounds(coordinates: &[Interval]) -> Result<(f64, f64), KernelError> {
    if coordinates.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    let mut squared_lower = 0.0_f64;
    let mut squared_upper = 0.0_f64;

    for coordinate in coordinates {
        let minimum_absolute = if coordinate.lower() <= 0.0 && coordinate.upper() >= 0.0 {
            0.0
        } else {
            coordinate.lower().abs().min(coordinate.upper().abs())
        };
        let maximum_absolute = coordinate.lower().abs().max(coordinate.upper().abs());

        let minimum_square = minimum_absolute * minimum_absolute;
        let maximum_square = maximum_absolute * maximum_absolute;
        if !minimum_square.is_finite() || !maximum_square.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }

        let minimum_square_lower = next_down(minimum_square).max(0.0);
        let maximum_square_upper = next_up(maximum_square);
        let next_lower = squared_lower + minimum_square_lower;
        let next_upper = squared_upper + maximum_square_upper;
        if !next_lower.is_finite() || !next_upper.is_finite() {
            return Err(KernelError::NumericalOverflow);
        }
        squared_lower = next_down(next_lower).max(0.0);
        squared_upper = next_up(next_upper);
    }

    let lower_norm = next_down(squared_lower.sqrt()).max(0.0);
    let upper_norm = next_up(squared_upper.sqrt());
    if !lower_norm.is_finite() || !upper_norm.is_finite() {
        return Err(KernelError::NumericalOverflow);
    }
    Ok((lower_norm, upper_norm))
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
        Interval::new(lower, upper).expect("valid coefficient interval")
    }

    #[test]
    fn constant_trajectory_outside_window_is_definitely_no_contact() {
        let result =
            classify_polynomial_contact(&[vec![interval(2.0, 2.0)]], -10.0, 10.0, 1.0, 64, 12)
                .expect("valid polynomial");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::DefinitelyNoContact
        );
        assert_eq!(result.excluded_time_intervals, 1);
        assert_eq!(result.unresolved_time_intervals, 0);
    }

    #[test]
    fn constant_trajectory_inside_window_is_definitely_contact() {
        let result =
            classify_polynomial_contact(&[vec![interval(0.5, 0.5)]], -2.0, 2.0, 1.0, 64, 12)
                .expect("valid polynomial");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
        assert_eq!(result.guaranteed_contact_interval, Some((-2.0, 2.0)));
    }

    #[test]
    fn crossing_trajectory_is_found_after_time_subdivision() {
        let result = classify_polynomial_contact(
            &[vec![interval(0.0, 0.0), interval(1.0, 1.0)]],
            -1.0,
            1.0,
            0.25,
            256,
            16,
        )
        .expect("valid polynomial");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
        assert!(result.guaranteed_contact_interval.is_some());
        assert!(result.visited_time_intervals > 1);
    }

    #[test]
    fn tangency_is_ambiguous_not_a_false_contact_or_exclusion() {
        let result = classify_polynomial_contact(
            &[vec![interval(0.0, 0.0), interval(1.0, 1.0)]],
            0.5,
            1.0,
            0.5,
            256,
            12,
        )
        .expect("valid polynomial");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::Ambiguous
        );
        assert!(result.unresolved_time_intervals > 0);
    }

    #[test]
    fn coefficient_uncertainty_can_still_prove_contact() {
        let result =
            classify_polynomial_contact(&[vec![interval(0.1, 0.2)]], -1.0, 1.0, 0.3, 32, 4)
                .expect("valid interval coefficient");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
    }

    #[test]
    fn multi_axis_euclidean_norm_is_used() {
        let result = classify_polynomial_contact(
            &[vec![interval(0.8, 0.8)], vec![interval(0.8, 0.8)]],
            -1.0,
            1.0,
            1.0,
            32,
            4,
        )
        .expect("valid vector polynomial");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::DefinitelyNoContact
        );
    }

    #[test]
    fn one_cell_budget_returns_ambiguous_and_accounts_for_pending_partition() {
        let result = classify_polynomial_contact(
            &[vec![interval(0.0, 0.0), interval(1.0, 1.0)]],
            -1.0,
            1.0,
            0.25,
            1,
            16,
        )
        .expect("valid polynomial");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::Ambiguous
        );
        assert!(result.budget_exhausted);
        assert_eq!(result.unresolved_time_intervals, 2);
    }

    #[test]
    fn a_point_window_is_supported() {
        let result = classify_polynomial_contact(&[vec![interval(0.5, 0.5)]], 1.0, 1.0, 1.0, 8, 0)
            .expect("valid point window");
        assert_eq!(
            result.classification,
            TrajectoryContactClassification::DefinitelyContact
        );
    }

    #[test]
    fn invalid_shapes_windows_radii_and_budgets_fail_closed() {
        assert_eq!(
            classify_polynomial_contact(&[], 0.0, 1.0, 1.0, 8, 4),
            Err(KernelError::EmptyDimension)
        );
        assert_eq!(
            classify_polynomial_contact(&[vec![]], 0.0, 1.0, 1.0, 8, 4),
            Err(KernelError::InvalidPolynomial)
        );
        assert_eq!(
            classify_polynomial_contact(&[vec![interval(1.0, 1.0)]], 1.0, 0.0, 1.0, 8, 4),
            Err(KernelError::InvalidInterval)
        );
        assert_eq!(
            classify_polynomial_contact(&[vec![interval(1.0, 1.0)]], 0.0, 1.0, f64::NAN, 8, 4),
            Err(KernelError::InvalidHardBodyRadius)
        );
        assert_eq!(
            classify_polynomial_contact(&[vec![interval(1.0, 1.0)]], 0.0, 1.0, 1.0, 0, 4),
            Err(KernelError::InvalidTrajectoryBudget)
        );
    }

    #[test]
    fn arithmetic_overflow_fails_closed() {
        assert_eq!(
            classify_polynomial_contact(&[vec![interval(f64::MAX, f64::MAX)]], 0.0, 1.0, 1.0, 8, 4),
            Err(KernelError::NumericalOverflow)
        );
    }
}
