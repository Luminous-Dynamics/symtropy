// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Budgeted residual-event-mass exploration outside declared geometric charts.
//!
//! The algorithm partitions a finite standard-normal cube. A cell is removed
//! from the residual ledger only when it is wholly contained in a caller-
//! declared chart box or the affine collision-ball test proves it event-free.
//! Every unresolved cell contributes a conservative density-times-volume
//! upper bound; the probability outside the cube contributes a separate
//! Gaussian tail bound.
//!
//! This is conditional on a fixed standard-normal nominal model and static
//! affine event geometry. It is not a proposal-support proof inside charts,
//! a nonlinear trajectory enclosure, or a formally machine-verified result.

use crate::{
    CoverageCell, CoverageMassUpperBound, KernelError,
    affine_cell_excludes_collision_ball, standard_normal_cube_tail_probability_upper_bound,
};

/// Configuration for the static affine residual-mass search.
#[derive(Debug, Clone, Copy)]
pub struct AffineResidualMassConfig<'a> {
    /// Offset of the affine relative-position map.
    pub offset: &'a [f64],
    /// Row-major affine matrix mapping standard-normal coordinates to relative position.
    pub matrix: &'a [f64],
    /// Closed collision-ball radius in the relative-position coordinate units.
    pub hard_body_radius: f64,
    /// Closed boxes already assigned to independently discovered geometric charts.
    ///
    /// A cell is treated as chart-covered only when the whole cell is contained
    /// in at least one supplied box. Partial overlap is never silently removed.
    pub covered_chart_boxes: &'a [CoverageCell],
    /// Radius of the finite standard-normal cube [-radius, radius]^d.
    pub cube_radius: f64,
    /// Maximum number of cells the traversal may classify.
    pub max_cells: usize,
    /// Maximum subdivision depth from the root cube.
    pub max_depth: usize,
}

/// Counts and residual-mass bound returned by the affine coverage search.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct AffineResidualMassReport {
    /// Number of cells for which a classification decision was attempted.
    pub visited_cells: usize,
    /// Cells wholly contained in one or more caller-declared chart boxes.
    pub chart_covered_cells: usize,
    /// Cells proven disjoint from the static collision ball.
    pub event_free_cells: usize,
    /// Cells retained because the event could not be excluded or chart-covered.
    pub unresolved_cells: usize,
    /// True when the cell budget prevented further subdivision.
    pub budget_exhausted: bool,
    /// Upper bound on event mass outside the supplied chart boxes, including
    /// the Gaussian probability outside the finite cube.
    pub residual_event_mass_upper_bound: f64,
}

/// Bound possible collision-event mass outside caller-declared chart boxes.
///
/// The search uses X(z) = offset + matrix*z for z ~ N(0, I), with the event
/// ||X(z)|| <= hard_body_radius. The finite search domain is the entire cube
/// [-cube_radius, cube_radius]^d; the complement is covered by a separate
/// Gaussian union/Chernoff upper bound.
///
/// Each cell is handled in this order:
/// 1. discard it from the residual ledger if it is wholly inside a chart box;
/// 2. discard it if affine interval geometry proves it event-free;
/// 3. otherwise subdivide until max_depth / max_cells, retaining any cell
///    that cannot be further explored in the unresolved-mass ledger.
///
/// The returned residual bound is conditional on the supplied affine map,
/// standard-normal nominal distribution, collision radius, and chart boxes.
/// A chart-covered region is not thereby proven to have accurate proposal
/// density or unbiased estimation; that is a separate qualification gate.
/// The current interval and probability arithmetic is numerical conservative
/// bookkeeping, not a formally machine-verified enclosure.
pub fn affine_collision_residual_mass_upper_bound(
    config: &AffineResidualMassConfig<'_>,
) -> Result<AffineResidualMassReport, KernelError> {
    let dimension = config.offset.len();
    if dimension == 0 {
        return Err(KernelError::EmptyDimension);
    }
    if !config.cube_radius.is_finite() || config.cube_radius <= 0.0 {
        return Err(KernelError::InvalidRadius);
    }
    if !config.hard_body_radius.is_finite() || config.hard_body_radius < 0.0 {
        return Err(KernelError::InvalidHardBodyRadius);
    }
    if config.max_cells == 0 {
        return Err(KernelError::InvalidCoverageBudget);
    }

    let root_lower = vec![-config.cube_radius; dimension];
    let root_upper = vec![config.cube_radius; dimension];
    let root = CoverageCell::new(root_lower, root_upper)?;

    // Validate the affine map and all chart boxes before producing a report.
    // Any malformed input aborts the certificate rather than being counted as
    // a geometric exclusion.
    crate::affine_box_image_bounds(
        config.offset,
        config.matrix,
        root.lower(),
        root.upper(),
    )?;
    for chart in config.covered_chart_boxes {
        if chart.lower().len() != dimension || chart.upper().len() != dimension {
            return Err(KernelError::DimensionMismatch);
        }
        if chart
            .lower()
            .iter()
            .zip(chart.upper())
            .zip(root.lower().iter().zip(root.upper()))
            .any(|((chart_lower, chart_upper), (domain_lower, domain_upper))| {
                chart_lower < domain_lower || chart_upper > domain_upper
            })
        {
            return Err(KernelError::CoverageChartOutsideDomain);
        }
    }

    let tail_upper =
        standard_normal_cube_tail_probability_upper_bound(config.cube_radius, dimension)?;
    let mut ledger = CoverageMassUpperBound::new();
    ledger.add_upper_bound(tail_upper)?;

    let mut stack = vec![root];
    let mut visited_cells = 0_usize;
    let mut chart_covered_cells = 0_usize;
    let mut event_free_cells = 0_usize;
    let mut unresolved_cells = 0_usize;
    let mut budget_exhausted = false;

    while let Some(cell) = stack.pop() {
        if visited_cells >= config.max_cells {
            // The pending stack is a disjoint partition of the remaining
            // portion of the root cube. Retain every pending cell.
            budget_exhausted = true;
            unresolved_cells += 1;
            ledger.add_unresolved_cell(&cell)?;
            for pending in stack.drain(..) {
                unresolved_cells += 1;
                ledger.add_unresolved_cell(&pending)?;
            }
            break;
        }
        visited_cells += 1;

        let fully_chart_covered = config.covered_chart_boxes.iter().any(|chart| {
            cell.lower()
                .iter()
                .zip(cell.upper())
                .zip(chart.lower().iter().zip(chart.upper()))
                .all(
                    |((cell_lower, cell_upper), (chart_lower, chart_upper))| {
                        cell_lower >= chart_lower && cell_upper <= chart_upper
                    },
                )
        });
        if fully_chart_covered {
            chart_covered_cells += 1;
            continue;
        }

        if affine_cell_excludes_collision_ball(
            config.offset,
            config.matrix,
            cell.lower(),
            cell.upper(),
            config.hard_body_radius,
        )? {
            event_free_cells += 1;
            continue;
        }

        if cell.path().len() >= config.max_depth {
            unresolved_cells += 1;
            ledger.add_unresolved_cell(&cell)?;
            continue;
        }

        match cell.split_longest_axis()? {
            Some((left, right)) => {
                // Push right first so the stable traversal visits left first.
                stack.push(right);
                stack.push(left);
            }
            None => {
                unresolved_cells += 1;
                ledger.add_unresolved_cell(&cell)?;
            }
        }
    }

    Ok(AffineResidualMassReport {
        visited_cells,
        chart_covered_cells,
        event_free_cells,
        unresolved_cells,
        budget_exhausted,
        residual_event_mass_upper_bound: ledger.upper_bound(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn chart_cover_and_event_exclusion_leave_only_cube_tail() {
        let chart =
            CoverageCell::new(vec![-0.5], vec![0.5]).expect("valid chart box");
        let report = affine_collision_residual_mass_upper_bound(
            &AffineResidualMassConfig {
                offset: &[0.0],
                matrix: &[1.0],
                hard_body_radius: 0.25,
                covered_chart_boxes: &[chart],
                cube_radius: 4.0,
                max_cells: 512,
                max_depth: 16,
            },
        )
        .expect("valid residual-mass search");

        let tail = standard_normal_cube_tail_probability_upper_bound(4.0, 1)
            .expect("valid tail bound");
        assert_eq!(report.unresolved_cells, 0);
        assert!(report.chart_covered_cells > 0);
        assert!(report.event_free_cells > 0);
        assert!(!report.budget_exhausted);
        assert!(report.residual_event_mass_upper_bound >= tail);
        assert!(report.residual_event_mass_upper_bound <= tail + 1.0e-12);
    }

    #[test]
    fn no_chart_keeps_possible_event_mass_unresolved() {
        let report = affine_collision_residual_mass_upper_bound(
            &AffineResidualMassConfig {
                offset: &[0.0],
                matrix: &[1.0],
                hard_body_radius: 0.25,
                covered_chart_boxes: &[],
                cube_radius: 4.0,
                max_cells: 1,
                max_depth: 16,
            },
        )
        .expect("valid residual-mass search");

        assert!(report.budget_exhausted);
        assert_eq!(report.visited_cells, 1);
        assert_eq!(report.unresolved_cells, 2);
        assert_eq!(report.residual_event_mass_upper_bound, 1.0);
    }

    #[test]
    fn wholly_event_free_cube_adds_no_unresolved_cell_mass() {
        let report = affine_collision_residual_mass_upper_bound(
            &AffineResidualMassConfig {
                offset: &[10.0],
                matrix: &[1.0],
                hard_body_radius: 1.0,
                covered_chart_boxes: &[],
                cube_radius: 4.0,
                max_cells: 8,
                max_depth: 8,
            },
        )
        .expect("valid residual-mass search");

        let tail = standard_normal_cube_tail_probability_upper_bound(4.0, 1)
            .expect("valid tail bound");
        assert_eq!(report.event_free_cells, 1);
        assert_eq!(report.unresolved_cells, 0);
        assert!(!report.budget_exhausted);
        assert!(report.residual_event_mass_upper_bound >= tail);
        assert!(report.residual_event_mass_upper_bound <= tail + 1.0e-12);
    }

    #[test]
    fn partially_overlapping_chart_does_not_remove_whole_cell() {
        let chart =
            CoverageCell::new(vec![0.0], vec![0.5]).expect("valid chart box");
        let report = affine_collision_residual_mass_upper_bound(
            &AffineResidualMassConfig {
                offset: &[0.0],
                matrix: &[1.0],
                hard_body_radius: 0.25,
                covered_chart_boxes: &[chart],
                cube_radius: 4.0,
                max_cells: 1,
                max_depth: 8,
            },
        )
        .expect("valid residual-mass search");

        // The initial cell is not contained in the chart, so it remains
        // unresolved when the one-cell budget is exhausted.
        assert!(report.budget_exhausted);
        assert_eq!(report.chart_covered_cells, 0);
        assert_eq!(report.unresolved_cells, 2);
        assert_eq!(report.residual_event_mass_upper_bound, 1.0);
    }

    #[test]
    fn rejects_zero_cell_budget() {
        let result = affine_collision_residual_mass_upper_bound(
            &AffineResidualMassConfig {
                offset: &[0.0],
                matrix: &[1.0],
                hard_body_radius: 1.0,
                covered_chart_boxes: &[],
                cube_radius: 2.0,
                max_cells: 0,
                max_depth: 4,
            },
        );
        assert_eq!(result, Err(KernelError::InvalidCoverageBudget));
    }

    #[test]
    fn rejects_chart_box_outside_search_cube() {
        let chart =
            CoverageCell::new(vec![-3.0], vec![-1.0]).expect("valid chart box");
        let result = affine_collision_residual_mass_upper_bound(
            &AffineResidualMassConfig {
                offset: &[0.0],
                matrix: &[1.0],
                hard_body_radius: 1.0,
                covered_chart_boxes: &[chart],
                cube_radius: 2.0,
                max_cells: 16,
                max_depth: 4,
            },
        );
        assert_eq!(result, Err(KernelError::CoverageChartOutsideDomain));
    }

    #[test]
    fn rejects_affine_dimension_mismatch_before_search() {
        let result = affine_collision_residual_mass_upper_bound(
            &AffineResidualMassConfig {
                offset: &[0.0, 0.0],
                matrix: &[1.0],
                hard_body_radius: 1.0,
                covered_chart_boxes: &[],
                cube_radius: 2.0,
                max_cells: 16,
                max_depth: 4,
            },
        );
        assert_eq!(result, Err(KernelError::InvalidMatrixLength));
    }
}
