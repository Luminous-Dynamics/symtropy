//! Conservative overlap classification for uncertain room/product geometry.
//!
//! This module does not replace the fixed-geometry oracle. It evaluates a
//! Cartesian set of bounded rectangles and never promotes a possible collision
//! to "definitely clear". All lengths are in metres. The 1e-9 m epsilon is a
//! numerical predicate tolerance, not a human clearance requirement.

/// Numerical tolerance shared with the current fixed-geometry audit predicate.
///
/// This is not a furnishing gap, accessibility clearance, or construction
/// tolerance. Those belong to separately sourced and versioned rule packs.
pub const GEOMETRY_EPSILON_M: f64 = 1e-9;

/// An inclusive, finite closed interval.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ClosedRange {
    min: f64,
    max: f64,
}

impl ClosedRange {
    /// Construct an inclusive range. Non-finite and inverted bounds are rejected.
    pub fn new(min: f64, max: f64) -> Result<Self, RangeError> {
        if !min.is_finite() || !max.is_finite() {
            return Err(RangeError::NonFiniteBound);
        }
        if min > max {
            return Err(RangeError::InvertedBounds);
        }
        Ok(Self { min, max })
    }

    pub fn min(self) -> f64 {
        self.min
    }

    pub fn max(self) -> f64 {
        self.max
    }

    /// True only when both bounds describe one exact value.
    pub fn is_exact(self) -> bool {
        self.min == self.max
    }
}

/// Construction failures for interval geometry.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum RangeError {
    NonFiniteBound,
    InvertedBounds,
    NonPositiveExtent,
    NonFiniteDerivedEndpoint,
}

/// Axis-aligned rectangle whose position and extents are independently bounded.
///
/// Each rectangle represents every combination in the Cartesian product of its
/// x, y, width, and depth ranges. This can be conservative when dimensions are
/// correlated, but does not assume unrecorded correlation to declare a layout safe.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct IntervalRect {
    x: ClosedRange,
    y: ClosedRange,
    width: ClosedRange,
    depth: ClosedRange,
}

impl IntervalRect {
    pub fn new(
        x: ClosedRange,
        y: ClosedRange,
        width: ClosedRange,
        depth: ClosedRange,
    ) -> Result<Self, RangeError> {
        if width.min <= 0.0 || depth.min <= 0.0 {
            return Err(RangeError::NonPositiveExtent);
        }
        let derived_endpoints = [
            x.min + width.min,
            x.min + width.max,
            x.max + width.min,
            x.max + width.max,
            y.min + depth.min,
            y.min + depth.max,
            y.max + depth.min,
            y.max + depth.max,
        ];
        if derived_endpoints.iter().any(|value| !value.is_finite()) {
            return Err(RangeError::NonFiniteDerivedEndpoint);
        }
        Ok(Self { x, y, width, depth })
    }

    pub fn x(self) -> ClosedRange {
        self.x
    }

    pub fn y(self) -> ClosedRange {
        self.y
    }

    pub fn width(self) -> ClosedRange {
        self.width
    }

    pub fn depth(self) -> ClosedRange {
        self.depth
    }

    /// Classify overlap for all geometries represented by these bounds.
    ///
    /// * DefinitelyClear: no represented geometry pair overlaps on both axes
    ///   under the current numerical predicate.
    /// * DefinitelyOverlapping: every represented geometry pair overlaps on
    ///   both axes under that predicate.
    /// * Indeterminate: some represented pairs may overlap and others may not,
    ///   so a downstream safety gate must not treat this as clear.
    ///
    /// The method uses worst-case endpoints, not sampling or Monte Carlo.
    pub fn classify_overlap(self, other: Self) -> OverlapClassification {
        let x_clear = definitely_separated(self.x, self.width, other.x, other.width);
        let y_clear = definitely_separated(self.y, self.depth, other.y, other.depth);
        if x_clear || y_clear {
            return OverlapClassification::DefinitelyClear;
        }

        let x_overlap = definitely_overlapping(self.x, self.width, other.x, other.width);
        let y_overlap = definitely_overlapping(self.y, self.depth, other.y, other.depth);
        if x_overlap && y_overlap {
            OverlapClassification::DefinitelyOverlapping
        } else {
            OverlapClassification::Indeterminate
        }
    }
}

/// Three-way result; uncertainty is never silently collapsed into a pass.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OverlapClassification {
    DefinitelyClear,
    DefinitelyOverlapping,
    Indeterminate,
}

fn definitely_separated(
    a_start: ClosedRange,
    a_extent: ClosedRange,
    b_start: ClosedRange,
    b_extent: ClosedRange,
) -> bool {
    let a_max_end = a_start.max + a_extent.max;
    let b_max_end = b_start.max + b_extent.max;
    a_max_end <= b_start.min + GEOMETRY_EPSILON_M || b_max_end <= a_start.min + GEOMETRY_EPSILON_M
}

fn definitely_overlapping(
    a_start: ClosedRange,
    a_extent: ClosedRange,
    b_start: ClosedRange,
    b_extent: ClosedRange,
) -> bool {
    a_start.max < b_start.min + b_extent.min - GEOMETRY_EPSILON_M
        && b_start.max < a_start.min + a_extent.min - GEOMETRY_EPSILON_M
}

#[cfg(test)]
mod tests {
    use super::{ClosedRange, IntervalRect, OverlapClassification, RangeError};

    fn range(min: f64, max: f64) -> ClosedRange {
        ClosedRange::new(min, max).expect("test ranges are valid")
    }

    fn rect(x: (f64, f64), y: (f64, f64), width: (f64, f64), depth: (f64, f64)) -> IntervalRect {
        IntervalRect::new(
            range(x.0, x.1),
            range(y.0, y.1),
            range(width.0, width.1),
            range(depth.0, depth.1),
        )
        .expect("test rectangle is valid")
    }

    #[test]
    fn fixed_rectangles_that_touch_at_an_edge_are_clear() {
        let a = rect((0.0, 0.0), (0.0, 0.0), (1.0, 1.0), (1.0, 1.0));
        let b = rect((1.0, 1.0), (0.0, 0.0), (0.5, 0.5), (1.0, 1.0));
        assert_eq!(
            a.classify_overlap(b),
            OverlapClassification::DefinitelyClear
        );
        assert_eq!(
            b.classify_overlap(a),
            OverlapClassification::DefinitelyClear
        );
    }

    #[test]
    fn fixed_rectangles_with_real_overlap_are_definitely_overlapping() {
        let a = rect((0.0, 0.0), (0.0, 0.0), (1.0, 1.0), (1.0, 1.0));
        let b = rect((0.25, 0.25), (0.25, 0.25), (1.0, 1.0), (1.0, 1.0));
        assert_eq!(
            a.classify_overlap(b),
            OverlapClassification::DefinitelyOverlapping
        );
    }

    #[test]
    fn possible_collision_from_position_uncertainty_is_indeterminate() {
        let a = rect((0.0, 0.0), (0.0, 0.0), (1.0, 1.0), (1.0, 1.0));
        let b = rect((0.9, 1.1), (0.0, 0.0), (0.2, 0.2), (1.0, 1.0));
        assert_eq!(a.classify_overlap(b), OverlapClassification::Indeterminate);
    }

    #[test]
    fn possible_collision_from_product_dimension_uncertainty_is_indeterminate() {
        let a = rect((0.0, 0.0), (0.0, 0.0), (0.8, 1.2), (1.0, 1.0));
        let b = rect((1.0, 1.0), (0.0, 0.0), (0.2, 0.2), (1.0, 1.0));
        assert_eq!(a.classify_overlap(b), OverlapClassification::Indeterminate);
    }

    #[test]
    fn separation_on_one_axis_proves_clear_even_if_the_other_is_uncertain() {
        let a = rect((0.0, 0.5), (0.0, 0.0), (0.8, 1.2), (1.0, 1.0));
        let b = rect((0.6, 0.9), (2.0, 2.1), (0.2, 0.4), (0.2, 0.3));
        assert_eq!(
            a.classify_overlap(b),
            OverlapClassification::DefinitelyClear
        );
    }

    #[test]
    fn widening_uncertainty_does_not_create_a_false_clear_result() {
        let a = rect((0.0, 0.0), (0.0, 0.0), (1.0, 1.0), (1.0, 1.0));
        let exact_other = rect((0.9, 0.9), (0.0, 0.0), (0.2, 0.2), (1.0, 1.0));
        let widened_other = rect((0.9, 1.2), (0.0, 0.0), (0.2, 0.2), (1.0, 1.0));
        assert_eq!(
            a.classify_overlap(exact_other),
            OverlapClassification::DefinitelyOverlapping
        );
        assert_eq!(
            a.classify_overlap(widened_other),
            OverlapClassification::Indeterminate
        );
    }

    #[test]
    fn invalid_ranges_and_nonpositive_extents_are_rejected() {
        assert_eq!(
            ClosedRange::new(f64::NAN, 1.0),
            Err(RangeError::NonFiniteBound)
        );
        assert_eq!(ClosedRange::new(2.0, 1.0), Err(RangeError::InvertedBounds));
        let error = IntervalRect::new(
            range(0.0, 0.0),
            range(0.0, 0.0),
            range(0.0, 0.0),
            range(1.0, 1.0),
        );
        assert_eq!(error, Err(RangeError::NonPositiveExtent));

        let huge = range(f64::MAX * 0.75, f64::MAX * 0.75);
        let overflow = IntervalRect::new(
            huge,
            range(0.0, 0.0),
            range(f64::MAX * 0.75, f64::MAX * 0.75),
            range(1.0, 1.0),
        );
        assert_eq!(overflow, Err(RangeError::NonFiniteDerivedEndpoint));
    }
}
