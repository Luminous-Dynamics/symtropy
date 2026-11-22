// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Chronological first-contact isolation for interval polynomial trajectories.
//!
//! The caller must supply coefficient intervals that enclose every represented
//! trajectory, including any approximation remainder, over the complete window.
//! This module uses Bernstein convex-hull range bounds to exclude earlier cells
//! and endpoint enclosures to prove a common outside-to-inside crossing bracket.
//!
//! Tangency and unresolved earlier cells remain ambiguous. The implementation
//! does not construct Taylor remainders, validate nonlinear dynamics, or
//! formally machine-verify floating-point arithmetic.

use crate::bernstein::{
    Bound, bernstein_hull, evaluate_bernstein, norm_bounds, power_to_bernstein,
    split_bernstein_half,
};
use crate::coverage::Interval;
use crate::{KernelError, safe_midpoint};

/// Configuration for chronological first-contact isolation on a polynomial window.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstContactIsolationConfig {
    /// Inclusive lower time bound in seconds.
    pub time_lower_seconds: f64,
    /// Inclusive upper time bound in seconds.
    pub time_upper_seconds: f64,
    /// Hard-body radius in the same distance units as the polynomial coordinates.
    pub hard_body_radius: f64,
    /// Maximum permitted width of the returned first-contact time bracket.
    pub time_width_tolerance_seconds: f64,
    /// Maximum number of time cells that may be visited.
    pub max_time_intervals: usize,
    /// Maximum subdivision depth for any time cell.
    pub max_depth: usize,
}

/// Three-way result of chronological first-contact isolation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FirstContactDisposition {
    /// Every time cell in the declared window was proven strictly outside HBR.
    DefinitelyNoContact,
    /// A common first-contact time bracket was proven for every represented
    /// coefficient choice, with every earlier time cell proven outside.
    FirstContactBracket,
    /// Contact, no-contact, or the requested bracket width could not be proved.
    Ambiguous,
}

/// Evidence and work accounting for one polynomial first-contact search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct FirstContactIsolationReport {
    /// Three-way first-contact disposition.
    pub disposition: FirstContactDisposition,
    /// Certified enclosure of the earliest contact time when disposition is
    /// FirstContactBracket.
    pub first_contact_time_bounds_seconds: Option<(f64, f64)>,
    /// Number of time cells evaluated.
    pub visited_time_intervals: usize,
    /// Number of complete time cells proven strictly outside HBR.
    pub excluded_time_intervals: usize,
    /// Number of visited cells that remained unresolved at the depth limit.
    pub unresolved_time_intervals: usize,
    /// Number of queued/later time cells not processed after a bracket or
    /// because the declared interval budget was exhausted.
    pub remaining_time_intervals: usize,
    /// Greatest subdivision depth reached.
    pub maximum_depth_reached: usize,
    /// True only when the traversal stopped because its visit budget was hit.
    pub budget_exhausted: bool,
    /// True when every time strictly earlier than the unresolved frontier or
    /// returned contact bracket was proven outside HBR.
    pub earlier_time_proven_clear: bool,
    /// True when the first-contact question was resolved (bracket or no contact).
    pub search_complete_for_first_contact: bool,
}

/// Isolate the earliest hard-body contact time over a complete polynomial window.
///
/// Coefficients use ascending power order for each relative-position axis. Every
/// represented trajectory must be enclosed by the supplied coefficient intervals
/// and any remainder over the complete time window.
///
/// A complete time cell is excluded only when its coordinate Bernstein hull gives
/// a distance lower bound strictly greater than the hard-body radius. A common
/// crossing bracket is emitted only when every represented trajectory is strictly
/// outside at the left endpoint and strictly inside at the right endpoint. By
/// continuity, every represented trajectory contacts within that cell. Because
/// traversal is chronological and every earlier cell has already been excluded,
/// this cell encloses the first contact time for every represented trajectory.
///
/// If every represented trajectory starts strictly inside HBR, the first-contact
/// bracket is the degenerate interval [t_start, t_start]. If the horizon-start
/// state is uncertain but all trajectories are inside by a later endpoint, a
/// bracket beginning at t_start may be returned once it meets the requested
/// width. Boundary-only contact, tangency, unresolved earlier cells, and
/// exhausted budgets remain ambiguous.
pub fn isolate_polynomial_first_contact(
    coefficients: &[Vec<Interval>],
    config: FirstContactIsolationConfig,
) -> Result<FirstContactIsolationReport, KernelError> {
    if coefficients.is_empty() {
        return Err(KernelError::EmptyDimension);
    }
    if coefficients.iter().any(Vec::is_empty) {
        return Err(KernelError::InvalidPolynomial);
    }
    if !config.time_lower_seconds.is_finite()
        || !config.time_upper_seconds.is_finite()
        || config.time_lower_seconds > config.time_upper_seconds
    {
        return Err(KernelError::InvalidInterval);
    }
    if !config.hard_body_radius.is_finite() || config.hard_body_radius <= 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    if !config.time_width_tolerance_seconds.is_finite()
        || config.time_width_tolerance_seconds <= 0.0
    {
        return Err(KernelError::InvalidTimeTolerance);
    }
    if config.max_time_intervals == 0 {
        return Err(KernelError::InvalidTrajectoryBudget);
    }

    let mut root_coordinates = Vec::with_capacity(coefficients.len());
    for axis in coefficients {
        root_coordinates.push(power_to_bernstein(
            axis,
            config.time_lower_seconds,
            config.time_upper_seconds,
        )?);
    }

    let mut stack = vec![FirstContactCell {
        lower: config.time_lower_seconds,
        upper: config.time_upper_seconds,
        depth: 0,
        coordinates: root_coordinates,
    }];
    let mut visited_time_intervals = 0_usize;
    let mut excluded_time_intervals = 0_usize;
    let mut unresolved_time_intervals = 0_usize;
    let mut maximum_depth_reached = 0_usize;

    while !stack.is_empty() {
        if visited_time_intervals >= config.max_time_intervals {
            return Ok(report(
                FirstContactDisposition::Ambiguous,
                None,
                visited_time_intervals,
                excluded_time_intervals,
                unresolved_time_intervals,
                stack.len(),
                maximum_depth_reached,
                true,
                unresolved_time_intervals == 0,
                false,
            ));
        }

        let cell = stack.pop().ok_or(KernelError::InvalidInterval)?;
        visited_time_intervals = visited_time_intervals
            .checked_add(1)
            .ok_or(KernelError::CounterOverflow)?;
        maximum_depth_reached = maximum_depth_reached.max(cell.depth);

        let mut coordinate_ranges = Vec::with_capacity(cell.coordinates.len());
        for axis in &cell.coordinates {
            coordinate_ranges.push(bernstein_hull(axis)?);
        }
        let (cell_distance_lower, _) = norm_bounds(&coordinate_ranges)?;

        if cell_distance_lower > config.hard_body_radius {
            excluded_time_intervals = excluded_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            continue;
        }

        let left_coordinates = evaluate_coordinates(&cell.coordinates, 0.0)?;
        let right_coordinates = evaluate_coordinates(&cell.coordinates, 1.0)?;
        let (left_distance_lower, left_distance_upper) = norm_bounds(&left_coordinates)?;
        let (_, right_distance_upper) = norm_bounds(&right_coordinates)?;

        let starts_at_horizon = cell.lower == config.time_lower_seconds;
        if starts_at_horizon && left_distance_upper < config.hard_body_radius {
            return Ok(report(
                FirstContactDisposition::FirstContactBracket,
                Some((
                    config.time_lower_seconds,
                    config.time_lower_seconds,
                )),
                visited_time_intervals,
                excluded_time_intervals,
                unresolved_time_intervals,
                stack
                    .len()
                    .checked_add(1)
                    .ok_or(KernelError::CounterOverflow)?,
                maximum_depth_reached,
                false,
                true,
                true,
            ));
        }

        let starts_outside = left_distance_lower > config.hard_body_radius;
        let ends_inside = right_distance_upper < config.hard_body_radius;
        let common_crossing = starts_outside && ends_inside;
        let horizon_start_bracket = starts_at_horizon && ends_inside;
        if (common_crossing || horizon_start_bracket)
            && cell.upper - cell.lower <= config.time_width_tolerance_seconds
        {
            let lower = if starts_outside {
                cell.lower
            } else {
                config.time_lower_seconds
            };
            return Ok(report(
                FirstContactDisposition::FirstContactBracket,
                Some((lower, cell.upper)),
                visited_time_intervals,
                excluded_time_intervals,
                unresolved_time_intervals,
                stack.len(),
                maximum_depth_reached,
                false,
                true,
                true,
            ));
        }

        if cell.depth >= config.max_depth {
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            return Ok(report(
                FirstContactDisposition::Ambiguous,
                None,
                visited_time_intervals,
                excluded_time_intervals,
                unresolved_time_intervals,
                stack.len(),
                maximum_depth_reached,
                false,
                true,
                false,
            ));
        }

        let midpoint = safe_midpoint(cell.lower, cell.upper)?;
        if midpoint <= cell.lower || midpoint >= cell.upper {
            unresolved_time_intervals = unresolved_time_intervals
                .checked_add(1)
                .ok_or(KernelError::CounterOverflow)?;
            return Ok(report(
                FirstContactDisposition::Ambiguous,
                None,
                visited_time_intervals,
                excluded_time_intervals,
                unresolved_time_intervals,
                stack.len(),
                maximum_depth_reached,
                false,
                true,
                false,
            ));
        }

        let mut left_children = Vec::with_capacity(cell.coordinates.len());
        let mut right_children = Vec::with_capacity(cell.coordinates.len());
        for axis in &cell.coordinates {
            let (left, right) = split_bernstein_half(axis)?;
            left_children.push(left);
            right_children.push(right);
        }

        // Stable chronological traversal: the left child is popped first.
        stack.push(FirstContactCell {
            lower: midpoint,
            upper: cell.upper,
            depth: cell.depth + 1,
            coordinates: right_children,
        });
        stack.push(FirstContactCell {
            lower: cell.lower,
            upper: midpoint,
            depth: cell.depth + 1,
            coordinates: left_children,
        });
    }

    Ok(report(
        FirstContactDisposition::DefinitelyNoContact,
        None,
        visited_time_intervals,
        excluded_time_intervals,
        unresolved_time_intervals,
        0,
        maximum_depth_reached,
        false,
        true,
        true,
    ))
}

#[derive(Debug, Clone)]
struct FirstContactCell {
    lower: f64,
    upper: f64,
    depth: usize,
    coordinates: Vec<Vec<Bound>>,
}

fn evaluate_coordinates(
    coordinates: &[Vec<Bound>],
    unit_time: f64,
) -> Result<Vec<Bound>, KernelError> {
    coordinates
        .iter()
        .map(|axis| evaluate_bernstein(axis, unit_time))
        .collect()
}

fn report(
    disposition: FirstContactDisposition,
    first_contact_time_bounds_seconds: Option<(f64, f64)>,
    visited_time_intervals: usize,
    excluded_time_intervals: usize,
    unresolved_time_intervals: usize,
    remaining_time_intervals: usize,
    maximum_depth_reached: usize,
    budget_exhausted: bool,
    earlier_time_proven_clear: bool,
    search_complete_for_first_contact: bool,
) -> FirstContactIsolationReport {
    FirstContactIsolationReport {
        disposition,
        first_contact_time_bounds_seconds,
        visited_time_intervals,
        excluded_time_intervals,
        unresolved_time_intervals,
        remaining_time_intervals,
        maximum_depth_reached,
        budget_exhausted,
        earlier_time_proven_clear,
        search_complete_for_first_contact,
    }
}
