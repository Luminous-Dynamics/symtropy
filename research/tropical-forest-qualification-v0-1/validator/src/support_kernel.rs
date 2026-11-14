// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Small exact support-intersection kernel for synthetic axis-aligned rectangles.
//!
//! Coordinates must already be expressed in one declared planar coordinate system.
//! This module does not transform CRS, repair geometry, authenticate assets, or
//! estimate positional uncertainty. It keeps geometric coverage separate from
//! valid-data coverage and rejects overlapping source supports to avoid double counting.

use std::fmt;

const RELATIVE_TOLERANCE: f64 = 1e-12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Rect {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

impl Rect {
    pub fn new(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Result<Self, SupportError> {
        let values = [min_x, min_y, max_x, max_y];
        if !values.iter().all(|value| value.is_finite())
            || max_x <= min_x
            || max_y <= min_y
        {
            return Err(SupportError::InvalidGeometry);
        }
        let rect = Self {
            min_x,
            min_y,
            max_x,
            max_y,
        };
        if !rect.area().is_finite() || rect.area() <= 0.0 {
            return Err(SupportError::InvalidGeometry);
        }
        Ok(rect)
    }

    pub fn area(self) -> f64 {
        (self.max_x - self.min_x) * (self.max_y - self.min_y)
    }

    pub fn intersection_area(self, other: Self) -> f64 {
        let width = self.max_x.min(other.max_x) - self.min_x.max(other.min_x);
        let height = self.max_y.min(other.max_y) - self.min_y.max(other.min_y);
        if width <= 0.0 || height <= 0.0 {
            0.0
        } else {
            width * height
        }
    }

    fn overlaps_with_positive_area(self, other: Self) -> bool {
        self.intersection_area(other) > 0.0
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupportError {
    InvalidGeometry,
    EmptyCoordinateReference,
    CoordinateReferenceMismatch,
    EmptySampleIdentity,
    OverlappingSourceSupports,
    CoverageExceedsTarget,
    InvalidShiftBounds,
}

impl fmt::Display for SupportError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SupportError {}

#[derive(Clone, Debug, PartialEq)]
pub struct SupportSample {
    pub sample_id: String,
    pub crs: String,
    pub bounds: Rect,
    pub valid: bool,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageDisposition {
    OutOfSupport,
    PartialGeometricCoverage,
    GeometricallyCompleteButDataIncomplete,
    CompleteValidCoverage,
}

impl CoverageDisposition {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OutOfSupport => "OUT_OF_SUPPORT",
            Self::PartialGeometricCoverage => "PARTIAL_GEOMETRIC_COVERAGE",
            Self::GeometricallyCompleteButDataIncomplete => {
                "GEOMETRICALLY_COMPLETE_BUT_DATA_INCOMPLETE"
            }
            Self::CompleteValidCoverage => "COMPLETE_VALID_COVERAGE",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SupportReport {
    pub target_area: f64,
    pub geometric_overlap_area: f64,
    pub geometric_coverage_fraction: f64,
    pub valid_data_area: f64,
    pub valid_data_coverage_fraction: f64,
    pub disposition: CoverageDisposition,
}


#[derive(Clone, Copy, Debug, PartialEq)]
pub struct RegistrationShiftBounds {
    pub min_dx: f64,
    pub max_dx: f64,
    pub min_dy: f64,
    pub max_dy: f64,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct OverlapUncertaintyBounds {
    pub minimum_overlap_area_m2: f64,
    pub maximum_overlap_area_m2: f64,
    pub minimum_coverage_fraction: f64,
    pub maximum_coverage_fraction: f64,
}

/// Bound rectangle overlap under every translation inside an axis-aligned shift box.
///
/// The extrema of one-dimensional interval overlap occur at the shift interval endpoints
/// or where rectangle edges align. The two-dimensional area envelope is the product of
/// the independently bounded x/y overlap lengths. If the true x/y error is correlated,
/// this rectangular shift box is conservative and may include impossible translations.
/// This is a deterministic worst/best-case envelope, not a probability distribution.
pub fn bound_rect_overlap_under_translation(
    target: Rect,
    source: Rect,
    shift: RegistrationShiftBounds,
) -> Result<OverlapUncertaintyBounds, SupportError> {
    let bounds = [shift.min_dx, shift.max_dx, shift.min_dy, shift.max_dy];
    if !bounds.iter().all(|value| value.is_finite())
        || shift.min_dx > shift.max_dx
        || shift.min_dy > shift.max_dy
    {
        return Err(SupportError::InvalidShiftBounds);
    }

    let x_overlap = axis_overlap_bounds(
        target.min_x,
        target.max_x,
        source.min_x,
        source.max_x,
        shift.min_dx,
        shift.max_dx,
    );
    let y_overlap = axis_overlap_bounds(
        target.min_y,
        target.max_y,
        source.min_y,
        source.max_y,
        shift.min_dy,
        shift.max_dy,
    );

    let target_area = target.area();
    let minimum_overlap_area_m2 = x_overlap.0 * y_overlap.0;
    let maximum_overlap_area_m2 = x_overlap.1 * y_overlap.1;
    Ok(OverlapUncertaintyBounds {
        minimum_overlap_area_m2,
        maximum_overlap_area_m2,
        minimum_coverage_fraction: minimum_overlap_area_m2 / target_area,
        maximum_coverage_fraction: maximum_overlap_area_m2 / target_area,
    })
}

fn axis_overlap_bounds(
    target_min: f64,
    target_max: f64,
    source_min: f64,
    source_max: f64,
    min_shift: f64,
    max_shift: f64,
) -> (f64, f64) {
    let critical_shifts = [
        min_shift,
        max_shift,
        target_min - source_max,
        target_min - source_min,
        target_max - source_max,
        target_max - source_min,
    ];

    let mut minimum = f64::INFINITY;
    let mut maximum: f64 = 0.0;
    for shift in critical_shifts {
        if shift < min_shift || shift > max_shift {
            continue;
        }
        let overlap = (target_max.min(source_max + shift)
            - target_min.max(source_min + shift))
        .max(0.0);
        minimum = minimum.min(overlap);
        maximum = maximum.max(overlap);
    }
    (minimum, maximum)
}

/// Qualify support overlap for non-overlapping source rectangles in one pre-transformed CRS.
///
/// Geometric coverage counts every source footprint, including cloud/no-data footprints.
/// Valid-data coverage counts only samples explicitly marked valid. No extrapolation is
/// performed for the unobserved remainder of the target support.
pub fn qualify_rectangular_support(
    target_crs: &str,
    target: Rect,
    samples: &[SupportSample],
) -> Result<SupportReport, SupportError> {
    if target_crs.trim().is_empty() {
        return Err(SupportError::EmptyCoordinateReference);
    }

    for sample in samples {
        if sample.sample_id.trim().is_empty() {
            return Err(SupportError::EmptySampleIdentity);
        }
        if sample.crs != target_crs {
            return Err(SupportError::CoordinateReferenceMismatch);
        }
    }

    for (left_index, left) in samples.iter().enumerate() {
        for right in samples.iter().skip(left_index + 1) {
            if left.bounds.overlaps_with_positive_area(right.bounds) {
                return Err(SupportError::OverlappingSourceSupports);
            }
        }
    }

    let target_area = target.area();
    if !target_area.is_finite() || target_area <= 0.0 {
        return Err(SupportError::InvalidGeometry);
    }

    let mut geometric_overlap_area = 0.0;
    let mut valid_data_area = 0.0;
    for sample in samples {
        let overlap = target.intersection_area(sample.bounds);
        geometric_overlap_area += overlap;
        if sample.valid {
            valid_data_area += overlap;
        }
    }

    let tolerance = target_area.max(1.0) * RELATIVE_TOLERANCE;
    if geometric_overlap_area > target_area + tolerance
        || valid_data_area > geometric_overlap_area + tolerance
    {
        return Err(SupportError::CoverageExceedsTarget);
    }

    let geometric_coverage_fraction =
        (geometric_overlap_area / target_area).clamp(0.0, 1.0);
    let valid_data_coverage_fraction = (valid_data_area / target_area).clamp(0.0, 1.0);
    let disposition = if geometric_overlap_area <= tolerance {
        CoverageDisposition::OutOfSupport
    } else if geometric_coverage_fraction < 1.0 - RELATIVE_TOLERANCE {
        CoverageDisposition::PartialGeometricCoverage
    } else if valid_data_coverage_fraction < 1.0 - RELATIVE_TOLERANCE {
        CoverageDisposition::GeometricallyCompleteButDataIncomplete
    } else {
        CoverageDisposition::CompleteValidCoverage
    };

    Ok(SupportReport {
        target_area,
        geometric_overlap_area,
        geometric_coverage_fraction,
        valid_data_area,
        valid_data_coverage_fraction,
        disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect(min_x: f64, min_y: f64, max_x: f64, max_y: f64) -> Rect {
        Rect::new(min_x, min_y, max_x, max_y).expect("test rectangle is valid")
    }

    fn sample(id: &str, bounds: Rect, valid: bool) -> SupportSample {
        SupportSample {
            sample_id: id.to_string(),
            crs: "LOCAL_METRE_GRID_A".to_string(),
            bounds,
            valid,
        }
    }

    #[test]
    fn identical_support_is_complete() {
        let target = rect(0.0, 0.0, 10.0, 10.0);
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            target,
            &[sample("a", target, true)],
        )
        .expect("same support qualifies");
        assert_eq!(report.geometric_coverage_fraction, 1.0);
        assert_eq!(report.valid_data_coverage_fraction, 1.0);
        assert_eq!(report.disposition, CoverageDisposition::CompleteValidCoverage);
    }

    #[test]
    fn target_inside_source_is_complete() {
        let target = rect(2.0, 2.0, 8.0, 8.0);
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            target,
            &[sample("a", rect(0.0, 0.0, 10.0, 10.0), true)],
        )
        .expect("containing source qualifies");
        assert_eq!(report.geometric_overlap_area, 36.0);
        assert_eq!(report.geometric_coverage_fraction, 1.0);
    }

    #[test]
    fn partial_edge_overlap_reports_fraction_not_extrapolation() {
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[sample("a", rect(5.0, 0.0, 15.0, 10.0), true)],
        )
        .expect("partial overlap qualifies as partial");
        assert_eq!(report.geometric_overlap_area, 50.0);
        assert_eq!(report.geometric_coverage_fraction, 0.5);
        assert_eq!(report.disposition, CoverageDisposition::PartialGeometricCoverage);
    }

    #[test]
    fn disjoint_support_is_not_absence() {
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[sample("a", rect(20.0, 20.0, 30.0, 30.0), true)],
        )
        .expect("disjoint support returns an out-of-support report");
        assert_eq!(report.geometric_coverage_fraction, 0.0);
        assert_eq!(report.disposition, CoverageDisposition::OutOfSupport);
    }

    #[test]
    fn two_non_overlapping_tiles_can_cover_target() {
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                sample("west", rect(0.0, 0.0, 5.0, 10.0), true),
                sample("east", rect(5.0, 0.0, 10.0, 10.0), true),
            ],
        )
        .expect("disjoint tiles qualify");
        assert_eq!(report.geometric_coverage_fraction, 1.0);
        assert_eq!(report.valid_data_coverage_fraction, 1.0);
    }

    #[test]
    fn fragmented_support_reports_partial_geometric_coverage() {
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                sample("northwest", rect(0.0, 5.0, 5.0, 10.0), true),
                sample("southeast", rect(5.0, 0.0, 10.0, 5.0), true),
            ],
        )
        .expect("fragmented supports qualify");
        assert_eq!(report.geometric_overlap_area, 50.0);
        assert_eq!(report.geometric_coverage_fraction, 0.5);
        assert_eq!(report.disposition, CoverageDisposition::PartialGeometricCoverage);
    }

    #[test]
    fn invalid_data_keeps_geometric_coverage_separate() {
        let target = rect(0.0, 0.0, 10.0, 10.0);
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            target,
            &[sample("cloud", target, false)],
        )
        .expect("masked support still has geometry");
        assert_eq!(report.geometric_coverage_fraction, 1.0);
        assert_eq!(report.valid_data_coverage_fraction, 0.0);
        assert_eq!(
            report.disposition,
            CoverageDisposition::GeometricallyCompleteButDataIncomplete
        );
    }

    #[test]
    fn mixed_validity_keeps_valid_fraction_separate() {
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                sample("valid", rect(0.0, 0.0, 5.0, 10.0), true),
                sample("masked", rect(5.0, 0.0, 10.0, 10.0), false),
            ],
        )
        .expect("mixed validity qualifies");
        assert_eq!(report.geometric_coverage_fraction, 1.0);
        assert_eq!(report.valid_data_coverage_fraction, 0.5);
    }

    #[test]
    fn registration_shift_changes_intersection_not_target_area() {
        let target = rect(0.0, 0.0, 10.0, 10.0);
        let aligned = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            target,
            &[sample("aligned", rect(0.0, 0.0, 10.0, 10.0), true)],
        )
        .expect("aligned report");
        let shifted = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            target,
            &[sample("shifted", rect(1.0, 0.0, 11.0, 10.0), true)],
        )
        .expect("shifted report");
        assert_eq!(aligned.target_area, shifted.target_area);
        assert_eq!(shifted.geometric_coverage_fraction, 0.9);
    }

    #[test]
    fn coordinate_reference_mismatch_is_rejected() {
        let error = qualify_rectangular_support(
            "EPSG:4326",
            rect(0.0, 0.0, 10.0, 10.0),
            &[sample("a", rect(0.0, 0.0, 10.0, 10.0), true)],
        )
        .expect_err("mismatched CRS must fail closed");
        assert_eq!(error, SupportError::CoordinateReferenceMismatch);
    }

    #[test]
    fn overlapping_source_supports_are_rejected_to_prevent_double_counting() {
        let error = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                sample("a", rect(0.0, 0.0, 6.0, 10.0), true),
                sample("b", rect(4.0, 0.0, 10.0, 10.0), true),
            ],
        )
        .expect_err("overlapping samples must not be silently summed");
        assert_eq!(error, SupportError::OverlappingSourceSupports);
    }

    #[test]
    fn degenerate_and_non_finite_rectangles_are_rejected() {
        assert_eq!(
            Rect::new(0.0, 0.0, 0.0, 10.0),
            Err(SupportError::InvalidGeometry)
        );
        assert_eq!(
            Rect::new(0.0, 0.0, f64::INFINITY, 10.0),
            Err(SupportError::InvalidGeometry)
        );
    }

    #[test]
    fn touching_support_edges_do_not_count_as_area_overlap() {
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[
                sample("left", rect(0.0, 0.0, 5.0, 10.0), true),
                sample("right", rect(5.0, 0.0, 10.0, 10.0), true),
            ],
        )
        .expect("edge-touching non-overlapping cells qualify");
        assert_eq!(report.geometric_coverage_fraction, 1.0);
    }


    #[test]
    fn registration_uncertainty_zero_shift_matches_exact_overlap() {
        let result = bound_rect_overlap_under_translation(
            rect(0.0, 0.0, 10.0, 10.0),
            rect(1.0, 1.0, 11.0, 11.0),
            RegistrationShiftBounds {
                min_dx: 0.0,
                max_dx: 0.0,
                min_dy: 0.0,
                max_dy: 0.0,
            },
        )
        .expect("zero-shift envelope");
        assert_eq!(result.minimum_overlap_area_m2, 81.0);
        assert_eq!(result.maximum_overlap_area_m2, 81.0);
        assert_eq!(result.minimum_coverage_fraction, 0.81);
        assert_eq!(result.maximum_coverage_fraction, 0.81);
    }

    #[test]
    fn registration_uncertainty_bounds_full_target_overlap() {
        let result = bound_rect_overlap_under_translation(
            rect(0.0, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, 10.0, 10.0),
            RegistrationShiftBounds {
                min_dx: -1.0,
                max_dx: 1.0,
                min_dy: -1.0,
                max_dy: 1.0,
            },
        )
        .expect("bounded shift");
        assert_eq!(result.minimum_overlap_area_m2, 81.0);
        assert_eq!(result.maximum_overlap_area_m2, 100.0);
    }

    #[test]
    fn registration_uncertainty_can_remove_guaranteed_coverage() {
        let result = bound_rect_overlap_under_translation(
            rect(0.0, 0.0, 10.0, 10.0),
            rect(10.0, 0.0, 20.0, 10.0),
            RegistrationShiftBounds {
                min_dx: -1.0,
                max_dx: 0.0,
                min_dy: 0.0,
                max_dy: 0.0,
            },
        )
        .expect("edge uncertainty");
        assert_eq!(result.minimum_overlap_area_m2, 0.0);
        assert_eq!(result.maximum_overlap_area_m2, 10.0);
    }

    #[test]
    fn registration_uncertainty_large_shift_has_zero_minimum_and_bounded_maximum() {
        let result = bound_rect_overlap_under_translation(
            rect(0.0, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, 10.0, 10.0),
            RegistrationShiftBounds {
                min_dx: -20.0,
                max_dx: 20.0,
                min_dy: -20.0,
                max_dy: 20.0,
            },
        )
        .expect("large shift");
        assert_eq!(result.minimum_overlap_area_m2, 0.0);
        assert_eq!(result.maximum_overlap_area_m2, 100.0);
    }

    #[test]
    fn registration_uncertainty_rejects_inverted_shift_bounds() {
        let result = bound_rect_overlap_under_translation(
            rect(0.0, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, 10.0, 10.0),
            RegistrationShiftBounds {
                min_dx: 1.0,
                max_dx: -1.0,
                min_dy: 0.0,
                max_dy: 0.0,
            },
        );
        assert_eq!(result, Err(SupportError::InvalidShiftBounds));
    }

    #[test]
    fn registration_uncertainty_rejects_non_finite_shift_bounds() {
        let result = bound_rect_overlap_under_translation(
            rect(0.0, 0.0, 10.0, 10.0),
            rect(0.0, 0.0, 10.0, 10.0),
            RegistrationShiftBounds {
                min_dx: f64::NEG_INFINITY,
                max_dx: 1.0,
                min_dy: 0.0,
                max_dy: 0.0,
            },
        );
        assert_eq!(result, Err(SupportError::InvalidShiftBounds));
    }

    #[test]
    fn empty_sample_list_is_unobserved_not_zero() {
        let report = qualify_rectangular_support(
            "LOCAL_METRE_GRID_A",
            rect(0.0, 0.0, 10.0, 10.0),
            &[],
        )
        .expect("empty coverage is an explicit report");
        assert_eq!(report.valid_data_coverage_fraction, 0.0);
        assert_eq!(report.disposition, CoverageDisposition::OutOfSupport);
    }
}
