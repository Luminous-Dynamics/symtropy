use std::cmp::Ordering;
use std::collections::BTreeSet;
use std::fmt;

/// Coordinates use signed integer ticks. The caller must declare the physical
/// scale (for example millimetres) in its benchmark manifest. No floating-point
/// epsilon participates in authoritative predicates.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Point {
    pub x: i64,
    pub y: i64,
}

impl Point {
    pub const fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GeometryError {
    TooFewVertices,
    ConsecutiveDuplicateVertex { index: usize },
    DuplicateVertex { first: usize, second: usize },
    ZeroArea,
    AdjacentEdgesOverlap { vertex: usize },
    SelfIntersection { first_edge: usize, second_edge: usize },
    ArithmeticOverflow,
    InvalidTimeWindow { start: i64, end: i64 },
}

impl fmt::Display for GeometryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::TooFewVertices => write!(f, "a polygon requires at least three vertices"),
            Self::ConsecutiveDuplicateVertex { index } => {
                write!(f, "consecutive duplicate vertex at index {index}")
            }
            Self::DuplicateVertex { first, second } => {
                write!(f, "duplicate vertex indices {first} and {second}")
            }
            Self::ZeroArea => write!(f, "polygon has zero signed area"),
            Self::AdjacentEdgesOverlap { vertex } => {
                write!(f, "adjacent edges overlap at vertex index {vertex}")
            }
            Self::SelfIntersection {
                first_edge,
                second_edge,
            } => write!(f, "edges {first_edge} and {second_edge} self-intersect"),
            Self::ArithmeticOverflow => write!(f, "exact integer predicate exceeded checked i128 range"),
            Self::InvalidTimeWindow { start, end } => {
                write!(f, "invalid half-open time window [{start}, {end})")
            }
        }
    }
}

impl std::error::Error for GeometryError {}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SegmentRelation {
    Disjoint,
    Touch,
    ProperCrossing,
    CollinearOverlap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PolygonRelation {
    Disjoint,
    BoundaryContact,
    PositiveAreaOverlap,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PointLocation {
    Outside,
    Boundary,
    Inside,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Polygon {
    vertices: Vec<Point>,
    twice_signed_area: i128,
}

impl Polygon {
    /// Construct and validate a simple closed polygon from its non-repeated
    /// vertex cycle. Do not include the first vertex a second time at the end.
    /// A failed exact predicate rejects the geometry; it never falls back to
    /// an axis-aligned rectangle.
    pub fn new(vertices: Vec<Point>) -> Result<Self, GeometryError> {
        if vertices.len() < 3 {
            return Err(GeometryError::TooFewVertices);
        }

        for index in 0..vertices.len() {
            let next = (index + 1) % vertices.len();
            if vertices[index] == vertices[next] {
                return Err(GeometryError::ConsecutiveDuplicateVertex { index });
            }
        }

        let mut seen = BTreeSet::new();
        for (index, point) in vertices.iter().enumerate() {
            if !seen.insert(*point) {
                let first = vertices
                    .iter()
                    .position(|candidate| candidate == point)
                    .unwrap_or(index);
                return Err(GeometryError::DuplicateVertex {
                    first,
                    second: index,
                });
            }
        }

        let twice_signed_area = signed_double_area(&vertices)?;
        if twice_signed_area == 0 {
            return Err(GeometryError::ZeroArea);
        }

        let count = vertices.len();
        for vertex in 0..count {
            let previous = vertices[(vertex + count - 1) % count];
            let current = vertices[vertex];
            let next = vertices[(vertex + 1) % count];
            if orientation(previous, current, next)? == Ordering::Equal
                && !point_on_segment(current, previous, next)?
            {
                return Err(GeometryError::AdjacentEdgesOverlap { vertex });
            }
        }

        for first_edge in 0..count {
            let first_next = (first_edge + 1) % count;
            for second_edge in (first_edge + 1)..count {
                let second_next = (second_edge + 1) % count;
                if second_edge == first_next || (first_edge == 0 && second_next == 0) {
                    continue;
                }
                let relation = segment_relation(
                    vertices[first_edge],
                    vertices[first_next],
                    vertices[second_edge],
                    vertices[second_next],
                )?;
                if relation != SegmentRelation::Disjoint {
                    return Err(GeometryError::SelfIntersection {
                        first_edge,
                        second_edge,
                    });
                }
            }
        }

        Ok(Self {
            vertices,
            twice_signed_area,
        })
    }

    pub fn vertices(&self) -> &[Point] {
        &self.vertices
    }

    pub fn twice_signed_area(&self) -> i128 {
        self.twice_signed_area
    }

    /// Return a canonical vertex cycle: counter-clockwise, starting at the
    /// lexicographically smallest vertex, with redundant straight-through
    /// collinear vertices removed. Useful for stable content digests.
    pub fn canonical_vertices(&self) -> Result<Vec<Point>, GeometryError> {
        let mut points = self.vertices.clone();

        loop {
            if points.len() <= 3 {
                break;
            }
            let mut remove_index = None;
            for index in 0..points.len() {
                let previous = points[(index + points.len() - 1) % points.len()];
                let current = points[index];
                let next = points[(index + 1) % points.len()];
                if orientation(previous, current, next)? == Ordering::Equal
                    && point_on_segment(current, previous, next)?
                {
                    remove_index = Some(index);
                    break;
                }
            }
            match remove_index {
                Some(index) => {
                    points.remove(index);
                }
                None => break,
            }
        }

        let area = signed_double_area(&points)?;
        if area == 0 {
            return Err(GeometryError::ZeroArea);
        }
        if area < 0 {
            points.reverse();
        }
        let min_index = points
            .iter()
            .enumerate()
            .min_by_key(|(_, point)| **point)
            .map(|(index, _)| index)
            .ok_or(GeometryError::TooFewVertices)?;
        points.rotate_left(min_index);
        Ok(points)
    }

    pub fn locate(&self, point: Point) -> Result<PointLocation, GeometryError> {
        point_in_polygon(point, &self.vertices)
    }

    /// Classify footprint relationship. Boundary-only contact is distinct from
    /// positive-area overlap. Contact is still a conflict for conservative yard
    /// packing unless a separately declared clearance policy permits it.
    pub fn relation(&self, other: &Self) -> Result<PolygonRelation, GeometryError> {
        if self.canonical_vertices()? == other.canonical_vertices()? {
            return Ok(PolygonRelation::PositiveAreaOverlap);
        }

        let mut boundary_contact = false;
        for first_edge in 0..self.vertices.len() {
            let first_next = (first_edge + 1) % self.vertices.len();
            for second_edge in 0..other.vertices.len() {
                let second_next = (second_edge + 1) % other.vertices.len();
                match segment_relation(
                    self.vertices[first_edge],
                    self.vertices[first_next],
                    other.vertices[second_edge],
                    other.vertices[second_next],
                )? {
                    SegmentRelation::ProperCrossing => {
                        return Ok(PolygonRelation::PositiveAreaOverlap);
                    }
                    SegmentRelation::Touch | SegmentRelation::CollinearOverlap => {
                        boundary_contact = true;
                    }
                    SegmentRelation::Disjoint => {}
                }
            }
        }

        for point in &self.vertices {
            if other.locate(*point)? == PointLocation::Inside {
                return Ok(PolygonRelation::PositiveAreaOverlap);
            }
        }
        for point in &other.vertices {
            if self.locate(*point)? == PointLocation::Inside {
                return Ok(PolygonRelation::PositiveAreaOverlap);
            }
        }

        if boundary_contact {
            Ok(PolygonRelation::BoundaryContact)
        } else {
            Ok(PolygonRelation::Disjoint)
        }
    }

    /// Returns a conservative conflict when footprints overlap in positive area
    /// or touch at any boundary. Only disjoint polygons are non-conflicting.
    pub fn conflicts_with(&self, other: &Self) -> Result<bool, GeometryError> {
        Ok(self.relation(other)? != PolygonRelation::Disjoint)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct TimeWindow {
    start: i64,
    end: i64,
}

impl TimeWindow {
    pub fn new(start: i64, end: i64) -> Result<Self, GeometryError> {
        if start >= end {
            return Err(GeometryError::InvalidTimeWindow { start, end });
        }
        Ok(Self { start, end })
    }

    pub const fn start(&self) -> i64 {
        self.start
    }

    pub const fn end(&self) -> i64 {
        self.end
    }

    /// Half-open intervals that meet at one endpoint do not overlap.
    pub const fn overlaps(&self, other: &Self) -> bool {
        self.start < other.end && other.start < self.end
    }
}

/// Space-time reservation conflict. The same physical region can be reused
/// after the earlier half-open occupancy interval has ended.
pub fn reservations_conflict(
    first: &Polygon,
    first_window: TimeWindow,
    second: &Polygon,
    second_window: TimeWindow,
) -> Result<bool, GeometryError> {
    if !first_window.overlaps(&second_window) {
        return Ok(false);
    }
    first.conflicts_with(second)
}

pub fn orientation(a: Point, b: Point, c: Point) -> Result<Ordering, GeometryError> {
    let ab_x = i128::from(b.x) - i128::from(a.x);
    let ab_y = i128::from(b.y) - i128::from(a.y);
    let ac_x = i128::from(c.x) - i128::from(a.x);
    let ac_y = i128::from(c.y) - i128::from(a.y);

    let left = ab_x
        .checked_mul(ac_y)
        .ok_or(GeometryError::ArithmeticOverflow)?;
    let right = ab_y
        .checked_mul(ac_x)
        .ok_or(GeometryError::ArithmeticOverflow)?;
    let determinant = left
        .checked_sub(right)
        .ok_or(GeometryError::ArithmeticOverflow)?;
    Ok(determinant.cmp(&0))
}

pub fn segment_relation(
    a: Point,
    b: Point,
    c: Point,
    d: Point,
) -> Result<SegmentRelation, GeometryError> {
    let o1 = orientation(a, b, c)?;
    let o2 = orientation(a, b, d)?;
    let o3 = orientation(c, d, a)?;
    let o4 = orientation(c, d, b)?;

    if opposite(o1, o2) && opposite(o3, o4) {
        return Ok(SegmentRelation::ProperCrossing);
    }

    if o1 == Ordering::Equal
        && o2 == Ordering::Equal
        && o3 == Ordering::Equal
        && o4 == Ordering::Equal
    {
        let use_x = a.x != b.x || c.x != d.x;
        let (a0, a1, c0, c1) = if use_x {
            (a.x.min(b.x), a.x.max(b.x), c.x.min(d.x), c.x.max(d.x))
        } else {
            (a.y.min(b.y), a.y.max(b.y), c.y.min(d.y), c.y.max(d.y))
        };
        let low = a0.max(c0);
        let high = a1.min(c1);
        return Ok(if low > high {
            SegmentRelation::Disjoint
        } else if low == high {
            SegmentRelation::Touch
        } else {
            SegmentRelation::CollinearOverlap
        });
    }

    if (o1 == Ordering::Equal && point_on_segment(c, a, b)?)
        || (o2 == Ordering::Equal && point_on_segment(d, a, b)?)
        || (o3 == Ordering::Equal && point_on_segment(a, c, d)?)
        || (o4 == Ordering::Equal && point_on_segment(b, c, d)?)
    {
        return Ok(SegmentRelation::Touch);
    }

    Ok(SegmentRelation::Disjoint)
}

fn opposite(a: Ordering, b: Ordering) -> bool {
    matches!(
        (a, b),
        (Ordering::Less, Ordering::Greater) | (Ordering::Greater, Ordering::Less)
    )
}

fn point_on_segment(point: Point, a: Point, b: Point) -> Result<bool, GeometryError> {
    if orientation(a, b, point)? != Ordering::Equal {
        return Ok(false);
    }
    Ok(point.x >= a.x.min(b.x)
        && point.x <= a.x.max(b.x)
        && point.y >= a.y.min(b.y)
        && point.y <= a.y.max(b.y))
}

fn signed_double_area(vertices: &[Point]) -> Result<i128, GeometryError> {
    let mut sum = 0i128;
    for index in 0..vertices.len() {
        let next = (index + 1) % vertices.len();
        let left = i128::from(vertices[index].x)
            .checked_mul(i128::from(vertices[next].y))
            .ok_or(GeometryError::ArithmeticOverflow)?;
        let right = i128::from(vertices[next].x)
            .checked_mul(i128::from(vertices[index].y))
            .ok_or(GeometryError::ArithmeticOverflow)?;
        let edge = left
            .checked_sub(right)
            .ok_or(GeometryError::ArithmeticOverflow)?;
        sum = sum
            .checked_add(edge)
            .ok_or(GeometryError::ArithmeticOverflow)?;
    }
    Ok(sum)
}

fn point_in_polygon(point: Point, vertices: &[Point]) -> Result<PointLocation, GeometryError> {
    let mut inside = false;
    for index in 0..vertices.len() {
        let a = vertices[index];
        let b = vertices[(index + 1) % vertices.len()];
        if point_on_segment(point, a, b)? {
            return Ok(PointLocation::Boundary);
        }
        if (a.y > point.y) != (b.y > point.y) {
            let side = orientation(a, b, point)?;
            let crosses_right = if b.y > a.y {
                side == Ordering::Greater
            } else {
                side == Ordering::Less
            };
            if crosses_right {
                inside = !inside;
            }
        }
    }
    Ok(if inside {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point {
        Point::new(x, y)
    }

    fn polygon(points: &[(i64, i64)]) -> Polygon {
        Polygon::new(points.iter().map(|(x, y)| p(*x, *y)).collect())
            .expect("test polygon should be valid")
    }

    fn rectangle(x0: i64, y0: i64, x1: i64, y1: i64) -> Polygon {
        polygon(&[(x0, y0), (x1, y0), (x1, y1), (x0, y1)])
    }

    #[test]
    fn exact_orientation_handles_clockwise_and_counter_clockwise_points() {
        assert_eq!(orientation(p(0, 0), p(4, 0), p(2, 3)), Ok(Ordering::Greater));
        assert_eq!(orientation(p(0, 0), p(4, 0), p(2, -3)), Ok(Ordering::Less));
        assert_eq!(orientation(p(0, 0), p(4, 0), p(2, 0)), Ok(Ordering::Equal));
    }

    #[test]
    fn segment_predicate_distinguishes_crossing_touch_and_collinear_overlap() {
        assert_eq!(
            segment_relation(p(0, 0), p(4, 4), p(0, 4), p(4, 0)),
            Ok(SegmentRelation::ProperCrossing)
        );
        assert_eq!(
            segment_relation(p(0, 0), p(4, 0), p(4, 0), p(4, 3)),
            Ok(SegmentRelation::Touch)
        );
        assert_eq!(
            segment_relation(p(0, 0), p(5, 0), p(2, 0), p(7, 0)),
            Ok(SegmentRelation::CollinearOverlap)
        );
        assert_eq!(
            segment_relation(p(0, 0), p(1, 0), p(2, 0), p(3, 0)),
            Ok(SegmentRelation::Disjoint)
        );
    }

    #[test]
    fn polygon_rejects_degenerate_and_self_intersecting_inputs() {
        assert_eq!(Polygon::new(vec![p(0, 0), p(1, 1)]), Err(GeometryError::TooFewVertices));
        assert!(matches!(
            Polygon::new(vec![p(0, 0), p(2, 0), p(2, 0), p(0, 2)]),
            Err(GeometryError::ConsecutiveDuplicateVertex { .. })
        ));
        assert_eq!(
            Polygon::new(vec![p(0, 0), p(2, 0), p(4, 0)]),
            Err(GeometryError::ZeroArea)
        );
        assert!(matches!(
            Polygon::new(vec![p(0, 0), p(3, 3), p(0, 3), p(3, 0)]),
            Err(GeometryError::ZeroArea) | Err(GeometryError::SelfIntersection { .. })
        ));
        assert!(matches!(
            Polygon::new(vec![p(0, 0), p(4, 0), p(2, 0), p(2, 3), p(0, 3)]),
            Err(GeometryError::AdjacentEdgesOverlap { .. }) | Err(GeometryError::SelfIntersection { .. })
        ));
    }

    #[test]
    fn relation_distinguishes_disjoint_touch_and_positive_area_overlap() {
        let a = rectangle(0, 0, 2, 2);
        let disjoint = rectangle(3, 0, 5, 2);
        let edge_touch = rectangle(2, 0, 4, 2);
        let vertex_touch = rectangle(2, 2, 4, 4);
        let overlap = rectangle(1, 1, 3, 3);
        let contained = rectangle(1, 1, 2, 2);
        assert_eq!(a.relation(&disjoint), Ok(PolygonRelation::Disjoint));
        assert_eq!(a.relation(&edge_touch), Ok(PolygonRelation::BoundaryContact));
        assert_eq!(a.relation(&vertex_touch), Ok(PolygonRelation::BoundaryContact));
        assert_eq!(a.relation(&overlap), Ok(PolygonRelation::PositiveAreaOverlap));
        assert_eq!(a.relation(&contained), Ok(PolygonRelation::PositiveAreaOverlap));
        assert!(a.conflicts_with(&edge_touch).unwrap());
        assert!(!a.conflicts_with(&disjoint).unwrap());
    }

    #[test]
    fn canonical_form_is_invariant_to_rotation_winding_and_redundant_collinear_points() {
        let base = polygon(&[(0, 0), (4, 0), (4, 4), (0, 4)]);
        let shifted = polygon(&[(4, 4), (0, 4), (0, 0), (4, 0)]);
        let clockwise_with_collinear = polygon(&[(0, 4), (2, 4), (4, 4), (4, 0), (0, 0)]);
        assert_eq!(base.canonical_vertices(), shifted.canonical_vertices());
        assert_eq!(base.canonical_vertices(), clockwise_with_collinear.canonical_vertices());
        assert_eq!(base.relation(&shifted), Ok(PolygonRelation::PositiveAreaOverlap));
    }

    #[test]
    fn point_location_handles_concavity() {
        let concave = polygon(&[(0, 0), (4, 0), (4, 1), (1, 1), (1, 4), (0, 4)]);
        assert_eq!(concave.locate(p(3, 3)), Ok(PointLocation::Outside));
        assert_eq!(concave.locate(p(0, 2)), Ok(PointLocation::Boundary));
        assert_eq!(concave.locate(p(0, 3)), Ok(PointLocation::Boundary));
        assert_eq!(concave.locate(p(1, 3)), Ok(PointLocation::Boundary));
        assert_eq!(concave.locate(p(0, 0)), Ok(PointLocation::Boundary));
        assert_eq!(concave.locate(p(3, 0)), Ok(PointLocation::Boundary));
    }

    #[test]
    fn half_open_time_windows_allow_endpoint_reuse() {
        let outgoing = TimeWindow::new(0, 5).unwrap();
        let incoming = TimeWindow::new(5, 10).unwrap();
        let overlap = TimeWindow::new(4, 8).unwrap();
        assert!(!outgoing.overlaps(&incoming));
        assert!(outgoing.overlaps(&overlap));
        assert!(TimeWindow::new(5, 5).is_err());
        assert!(TimeWindow::new(8, 2).is_err());
    }

    #[test]
    fn space_time_reservations_can_reuse_footprint_after_clearance() {
        let footprint = rectangle(0, 0, 2, 2);
        let outgoing = TimeWindow::new(0, 5).unwrap();
        let incoming = TimeWindow::new(5, 10).unwrap();
        let late_incoming = TimeWindow::new(4, 10).unwrap();
        assert_eq!(
            reservations_conflict(&footprint, outgoing, &footprint, incoming),
            Ok(false)
        );
        assert_eq!(
            reservations_conflict(&footprint, outgoing, &footprint, late_incoming),
            Ok(true)
        );
    }

    #[test]
    fn extreme_coordinates_fail_closed_if_exact_arithmetic_overflows() {
        let a = p(i64::MIN, i64::MIN);
        let b = p(i64::MAX, i64::MIN);
        let c = p(i64::MIN, i64::MAX);
        assert_eq!(orientation(a, b, c), Err(GeometryError::ArithmeticOverflow));
    }

    #[test]
    fn independent_rectangle_cell_oracle_checks_all_small_pairs() {
        use std::collections::BTreeSet;

        let mut rectangles = Vec::new();
        for x0 in 0..4 {
            for x1 in (x0 + 1)..=4 {
                for y0 in 0..4 {
                    for y1 in (y0 + 1)..=4 {
                        rectangles.push((x0, y0, x1, y1, rectangle(x0, y0, x1, y1)));
                    }
                }
            }
        }
        let cells = |x0: i64, y0: i64, x1: i64, y1: i64| -> BTreeSet<(i64, i64)> {
            let mut occupied = BTreeSet::new();
            for x in x0..x1 {
                for y in y0..y1 {
                    occupied.insert((x, y));
                }
            }
            occupied
        };
        for (ax0, ay0, ax1, ay1, a) in &rectangles {
            let a_cells = cells(*ax0, *ay0, *ax1, *ay1);
            for (bx0, by0, bx1, by1, b) in &rectangles {
                let b_cells = cells(*bx0, *by0, *bx1, *by1);
                let positive_area = !a_cells.is_disjoint(&b_cells);
                let x_overlap_closed = *ax0 <= *bx1 && *bx0 <= *ax1;
                let y_overlap_closed = *ay0 <= *by1 && *by0 <= *ay1;
                let boundary_contact = x_overlap_closed && y_overlap_closed;
                let expected = if positive_area {
                    PolygonRelation::PositiveAreaOverlap
                } else if boundary_contact {
                    PolygonRelation::BoundaryContact
                } else {
                    PolygonRelation::Disjoint
                };
                assert_eq!(
                    a.relation(b),
                    Ok(expected),
                    "oracle mismatch: A=({ax0},{ay0},{ax1},{ay1}), B=({bx0},{by0},{bx1},{by1})"
                );
            }
        }
    }

    #[test]
    fn positive_area_overlap_is_symmetric_for_adversarial_concave_shapes() {
        let l_shape = polygon(&[(0, 0), (4, 0), (4, 1), (1, 1), (1, 4), (0, 4)]);
        let bar = rectangle(0, 2, 3, 3);
        assert_eq!(l_shape.relation(&bar), Ok(PolygonRelation::PositiveAreaOverlap));
        assert_eq!(bar.relation(&l_shape), Ok(PolygonRelation::PositiveAreaOverlap));
    }
}
