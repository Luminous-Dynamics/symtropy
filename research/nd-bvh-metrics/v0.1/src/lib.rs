// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
// Commercial licensing: see COMMERCIAL_LICENSE.md at repository root

//! Dimension-aware AABB geometry diagnostics for broadphase research.
//!
//! These metrics describe geometry, not observed traversal cost. They must be
//! calibrated against query counters before being used by an algorithm selector.

use std::error::Error;
use std::fmt;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum AabbError {
    ZeroDimension,
    NonFiniteEndpoint,
    InvertedBounds,
}

impl fmt::Display for AabbError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::ZeroDimension => write!(f, "AABB dimension must be greater than zero"),
            Self::NonFiniteEndpoint => write!(f, "AABB endpoints must be finite"),
            Self::InvertedBounds => write!(f, "AABB minimum must not exceed maximum"),
        }
    }
}

impl Error for AabbError {}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Aabb<const D: usize> {
    min: [f64; D],
    max: [f64; D],
}

impl<const D: usize> Aabb<D> {
    pub fn new(min: [f64; D], max: [f64; D]) -> Result<Self, AabbError> {
        if D == 0 {
            return Err(AabbError::ZeroDimension);
        }
        for axis in 0..D {
            if !min[axis].is_finite() || !max[axis].is_finite() {
                return Err(AabbError::NonFiniteEndpoint);
            }
            if min[axis] > max[axis] {
                return Err(AabbError::InvertedBounds);
            }
        }
        Ok(Self { min, max })
    }

    #[must_use]
    pub fn min(&self) -> &[f64; D] {
        &self.min
    }

    #[must_use]
    pub fn max(&self) -> &[f64; D] {
        &self.max
    }

    #[must_use]
    pub fn widths(&self) -> [f64; D] {
        std::array::from_fn(|axis| self.max[axis] - self.min[axis])
    }

    /// Finite D-dimensional hypervolume. Valid degenerate boxes have volume 0.
    #[must_use]
    pub fn hypervolume(&self) -> Option<f64> {
        let mut volume = 1.0;
        for width in self.widths() {
            volume *= width;
            if !volume.is_finite() {
                return None;
            }
        }
        Some(volume)
    }

    /// Generalized codimension-one boundary content:
    /// 2 * sum_i(product_{j != i}(width_j)).
    ///
    /// This is perimeter in 2D and surface area in 3D. Its physical units
    /// depend on D; raw values are not cross-dimensional comparable.
    #[must_use]
    pub fn boundary_content(&self) -> Option<f64> {
        let widths = self.widths();
        let mut sum = 0.0;
        for omitted_axis in 0..D {
            let mut face_measure = 1.0;
            for (axis, width) in widths.iter().enumerate() {
                if axis != omitted_axis {
                    face_measure *= width;
                    if !face_measure.is_finite() {
                        return None;
                    }
                }
            }
            sum += face_measure;
            if !sum.is_finite() {
                return None;
            }
        }
        let boundary = 2.0 * sum;
        boundary.is_finite().then_some(boundary)
    }

    /// Hypervolume of the intersection. Disjoint or touching boxes return 0.
    #[must_use]
    pub fn intersection_hypervolume(&self, other: &Self) -> Option<f64> {
        let mut volume = 1.0;
        for axis in 0..D {
            let lo = self.min[axis].max(other.min[axis]);
            let hi = self.max[axis].min(other.max[axis]);
            if hi <= lo {
                return Some(0.0);
            }
            volume *= hi - lo;
            if !volume.is_finite() {
                return None;
            }
        }
        Some(volume)
    }

    /// Intersection volume divided by the smaller positive input volume.
    ///
    /// Returns None for degenerate inputs or unavailable arithmetic. A value
    /// of 0 means disjoint interiors; 1 means the smaller box is contained.
    #[must_use]
    pub fn overlap_ratio(&self, other: &Self) -> Option<f64> {
        let left_volume = self.hypervolume()?;
        let right_volume = other.hypervolume()?;
        let denominator = left_volume.min(right_volume);
        if denominator <= 0.0 {
            return None;
        }
        let ratio = self.intersection_hypervolume(other)? / denominator;
        if !ratio.is_finite() || !(0.0..=1.0).contains(&ratio) {
            return None;
        }
        Some(ratio)
    }

    /// Per-axis extent relative to a root box. A zero root width makes that
    /// axis unavailable instead of pretending the ratio is zero.
    #[must_use]
    pub fn axis_normalized_extent(&self, root: &Self) -> [Option<f64>; D] {
        let widths = self.widths();
        let root_widths = root.widths();
        std::array::from_fn(|axis| {
            if root_widths[axis] <= 0.0 {
                return None;
            }
            let ratio = widths[axis] / root_widths[axis];
            ratio.is_finite().then_some(ratio)
        })
    }
}

/// Sum of internal-node boundary contents divided by the root boundary content.
///
/// None means the root denominator or one of the required measurements is
/// degenerate, non-finite or overflowing.
#[must_use]
pub fn normalized_internal_boundary_content<const D: usize>(
    root: &Aabb<D>,
    internal_nodes: &[Aabb<D>],
) -> Option<f64> {
    let root_measure = root.boundary_content()?;
    if root_measure <= 0.0 {
        return None;
    }
    let mut sum = 0.0;
    for node in internal_nodes {
        sum += node.boundary_content()?;
        if !sum.is_finite() {
            return None;
        }
    }
    let ratio = sum / root_measure;
    ratio.is_finite().then_some(ratio)
}

/// Spearman rank correlation between a structural metric and observed p95 work.
///
/// Average ranks are used for ties. This helper requires at least five paired,
/// finite observations and returns None for constant inputs, mismatched lengths
/// or an unavailable correlation. A high correlation on a training corpus is
/// not sufficient for selector promotion; #7257 requires held-out families.
#[must_use]
pub fn spearman_rank_correlation(
    structural_metric: &[f64],
    observed_p95_work: &[f64],
) -> Option<f64> {
    if structural_metric.len() != observed_p95_work.len()
        || structural_metric.len() < 5
        || structural_metric
            .iter()
            .chain(observed_p95_work.iter())
            .any(|value| !value.is_finite())
    {
        return None;
    }

    let x = average_ranks(structural_metric)?;
    let y = average_ranks(observed_p95_work)?;
    let mean_rank = (structural_metric.len() as f64 + 1.0) / 2.0;

    let mut covariance = 0.0;
    let mut x_variance = 0.0;
    let mut y_variance = 0.0;
    for (x_rank, y_rank) in x.iter().zip(y.iter()) {
        let dx = x_rank - mean_rank;
        let dy = y_rank - mean_rank;
        covariance += dx * dy;
        x_variance += dx * dx;
        y_variance += dy * dy;
    }

    let denominator = (x_variance * y_variance).sqrt();
    if !denominator.is_finite() || denominator <= 0.0 {
        return None;
    }
    let rho = covariance / denominator;
    rho.is_finite().then_some(rho.clamp(-1.0, 1.0))
}

fn average_ranks(values: &[f64]) -> Option<Vec<f64>> {
    if values.iter().any(|value| !value.is_finite()) {
        return None;
    }
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_unstable_by(|left, right| values[*left].total_cmp(&values[*right]));

    let mut ranks = vec![0.0; values.len()];
    let mut start = 0usize;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && values[order[end]] == values[order[start]] {
            end += 1;
        }
        let average_rank = ((start + 1) as f64 + end as f64) / 2.0;
        for index in &order[start..end] {
            ranks[*index] = average_rank;
        }
        start = end;
    }
    Some(ranks)
}

/// Summary of sibling overlap measurements. None values are preserved in
/// the unavailable count and excluded from mean/max; they are never coerced
/// to zero.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlapSummary {
    pub measured_pairs: usize,
    pub unavailable_pairs: usize,
    pub mean: Option<f64>,
    pub max: Option<f64>,
}

#[must_use]
pub fn summarize_sibling_overlaps<const D: usize>(
    sibling_pairs: &[(Aabb<D>, Aabb<D>)],
) -> OverlapSummary {
    let mut measured_pairs = 0usize;
    let mut unavailable_pairs = 0usize;
    let mut sum = 0.0;
    let mut max: Option<f64> = None;

    for (left, right) in sibling_pairs {
        match left.overlap_ratio(right) {
            Some(ratio) => {
                measured_pairs += 1;
                sum += ratio;
                if !sum.is_finite() {
                    return OverlapSummary {
                        measured_pairs,
                        unavailable_pairs: sibling_pairs.len() - measured_pairs,
                        mean: None,
                        max: None,
                    };
                }
                max = Some(max.map_or(ratio, |current| current.max(ratio)));
            }
            None => unavailable_pairs += 1,
        }
    }

    OverlapSummary {
        measured_pairs,
        unavailable_pairs,
        mean: (measured_pairs > 0).then_some(sum / measured_pairs as f64),
        max,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn box2(min: [f64; 2], max: [f64; 2]) -> Aabb<2> {
        Aabb::new(min, max).unwrap()
    }

    fn box3(min: [f64; 3], max: [f64; 3]) -> Aabb<3> {
        Aabb::new(min, max).unwrap()
    }

    #[test]
    fn boundary_content_is_perimeter_in_two_dimensions() {
        let a = box2([0.0, 0.0], [3.0, 4.0]);
        assert_eq!(a.boundary_content(), Some(14.0));
    }

    #[test]
    fn boundary_content_is_surface_area_in_three_dimensions() {
        let a = box3([0.0, 0.0, 0.0], [2.0, 3.0, 5.0]);
        assert_eq!(a.boundary_content(), Some(62.0));
        assert_eq!(a.hypervolume(), Some(30.0));
    }

    #[test]
    fn one_dimensional_boundary_counts_both_endpoints() {
        let a = Aabb::new([-2.0], [3.0]).unwrap();
        assert_eq!(a.boundary_content(), Some(2.0));
        assert_eq!(a.hypervolume(), Some(5.0));
    }

    #[test]
    fn five_dimensional_measure_has_the_expected_units_and_value() {
        let a = Aabb::new([0.0; 5], [2.0; 5]).unwrap();
        assert_eq!(a.hypervolume(), Some(32.0));
        assert_eq!(a.boundary_content(), Some(160.0));
    }

    #[test]
    fn lower_dimensional_sheet_has_finite_boundary_content() {
        let sheet = box3([0.0, 0.0, 0.0], [2.0, 3.0, 0.0]);
        assert_eq!(sheet.hypervolume(), Some(0.0));
        assert_eq!(sheet.boundary_content(), Some(12.0));
    }

    #[test]
    fn zero_boundary_measure_root_makes_normalized_ratio_unavailable() {
        let root = box3([0.0, 0.0, 0.0], [2.0, 0.0, 0.0]);
        assert_eq!(root.boundary_content(), Some(0.0));
        assert_eq!(normalized_internal_boundary_content(&root, &[root]), None);
    }

    #[test]
    fn normalized_boundary_ratio_is_translation_invariant() {
        let root = box2([0.0, 0.0], [4.0, 4.0]);
        let child = box2([0.0, 0.0], [2.0, 2.0]);
        let shifted_root = box2([100.0, -50.0], [104.0, -46.0]);
        let shifted_child = box2([100.0, -50.0], [102.0, -48.0]);
        let first = normalized_internal_boundary_content(&root, &[child]).unwrap();
        let second = normalized_internal_boundary_content(&shifted_root, &[shifted_child]).unwrap();
        assert_eq!(first, second);
        assert_eq!(first, 0.5);
    }

    #[test]
    fn overlap_ratio_is_zero_for_disjoint_boxes_and_one_for_containment() {
        let a = box2([0.0, 0.0], [2.0, 2.0]);
        let disjoint = box2([3.0, 0.0], [5.0, 2.0]);
        let contained = box2([0.5, 0.5], [1.5, 1.5]);
        assert_eq!(a.overlap_ratio(&disjoint), Some(0.0));
        assert_eq!(a.overlap_ratio(&contained), Some(1.0));
    }

    #[test]
    fn overlap_ratio_is_fraction_of_smaller_box() {
        let a = box3([0.0, 0.0, 0.0], [2.0, 2.0, 2.0]);
        let b = box3([1.0, 1.0, 1.0], [3.0, 3.0, 3.0]);
        assert_eq!(a.overlap_ratio(&b), Some(0.125));
    }

    #[test]
    fn overlap_summary_preserves_unavailable_values() {
        let valid = box2([0.0, 0.0], [2.0, 2.0]);
        let degenerate = box2([0.0, 0.0], [0.0, 1.0]);
        let separate = box2([3.0, 0.0], [5.0, 2.0]);
        let summary =
            summarize_sibling_overlaps(&[(valid, separate), (valid, degenerate), (valid, valid)]);
        assert_eq!(summary.measured_pairs, 2);
        assert_eq!(summary.unavailable_pairs, 1);
        assert_eq!(summary.mean, Some(0.5));
        assert_eq!(summary.max, Some(1.0));
    }

    #[test]
    fn axis_normalized_extent_marks_zero_root_axes_unavailable() {
        let root = box3([0.0, 0.0, 0.0], [4.0, 2.0, 0.0]);
        let child = box3([1.0, 0.5, 0.0], [3.0, 1.5, 0.0]);
        assert_eq!(
            child.axis_normalized_extent(&root),
            [Some(0.5), Some(0.5), None]
        );
    }

    #[test]
    fn invalid_bounds_are_rejected() {
        assert_eq!(
            Aabb::new([f64::NAN, 0.0], [1.0, 1.0]).unwrap_err(),
            AabbError::NonFiniteEndpoint
        );
        assert_eq!(
            Aabb::new([2.0, 0.0], [1.0, 1.0]).unwrap_err(),
            AabbError::InvertedBounds
        );
        assert_eq!(Aabb::<0>::new([], []), Err(AabbError::ZeroDimension));
    }

    #[test]
    fn spearman_rank_correlation_detects_perfect_positive_ordering() {
        assert_eq!(
            spearman_rank_correlation(
                &[1.0, 2.0, 3.0, 4.0, 5.0],
                &[10.0, 20.0, 30.0, 40.0, 50.0],
            ),
            Some(1.0)
        );
    }

    #[test]
    fn spearman_rank_correlation_detects_inverse_ordering() {
        assert_eq!(
            spearman_rank_correlation(
                &[1.0, 2.0, 3.0, 4.0, 5.0],
                &[50.0, 40.0, 30.0, 20.0, 10.0],
            ),
            Some(-1.0)
        );
    }

    #[test]
    fn spearman_rank_correlation_uses_average_ranks_for_ties() {
        assert_eq!(
            spearman_rank_correlation(
                &[1.0, 2.0, 2.0, 4.0, 5.0],
                &[10.0, 20.0, 20.0, 40.0, 50.0],
            ),
            Some(1.0)
        );
    }

    #[test]
    fn spearman_rank_correlation_fails_closed_on_unqualified_samples() {
        assert_eq!(spearman_rank_correlation(&[1.0, 2.0], &[4.0, 5.0]), None);
        assert_eq!(
            spearman_rank_correlation(&[1.0, 2.0, 3.0, 4.0, 5.0], &[1.0, 2.0]),
            None
        );
        assert_eq!(
            spearman_rank_correlation(&[1.0, 1.0, 1.0, 1.0, 1.0], &[1.0, 2.0, 3.0, 4.0, 5.0]),
            None
        );
        assert_eq!(
            spearman_rank_correlation(
                &[1.0, 2.0, 3.0, 4.0, f64::NAN],
                &[1.0, 2.0, 3.0, 4.0, 5.0],
            ),
            None
        );
    }

    #[test]
    fn overflowing_measures_are_unavailable() {
        let huge = Aabb::new([0.0; 3], [f64::MAX; 3]).unwrap();
        assert_eq!(huge.hypervolume(), None);
        assert_eq!(huge.boundary_content(), None);
    }
}
