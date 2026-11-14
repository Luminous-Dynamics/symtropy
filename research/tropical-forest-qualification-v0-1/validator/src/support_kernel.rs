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

    for left_index in 0..samples.len() {
        for right_index in (left_index + 1)..samples.len() {
            if samples[left_index]
                .bounds
                .overlaps_with_positive_area(samples[right_index].bounds)
            {
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
