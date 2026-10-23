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

        // Vertex tests alone miss polygons whose vertices all lie on the other
        // boundary while their edges chord through its interior (for example,
        // a diamond inscribed in a square). Exact doubled-coordinate midpoints
        // are rational points on each edge and distinguish that case.
        for index in 0..self.vertices.len() {
            let next = (index + 1) % self.vertices.len();
            match midpoint_location(self.vertices[index], self.vertices[next], &other.vertices)? {
                PointLocation::Inside => return Ok(PolygonRelation::PositiveAreaOverlap),
                PointLocation::Boundary => boundary_contact = true,
                PointLocation::Outside => {}
            }
        }
        for index in 0..other.vertices.len() {
            let next = (index + 1) % other.vertices.len();
            match midpoint_location(other.vertices[index], other.vertices[next], &self.vertices)? {
                PointLocation::Inside => return Ok(PolygonRelation::PositiveAreaOverlap),
                PointLocation::Boundary => boundary_contact = true,
                PointLocation::Outside => {}
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

fn midpoint_location(
    edge_start: Point,
    edge_end: Point,
    polygon: &[Point],
) -> Result<PointLocation, GeometryError> {
    // Coordinates are doubled, so (a+b) is the exact midpoint numerator.
    // i64 coordinates are promoted before addition/multiplication.
    let px2 = i128::from(edge_start.x) + i128::from(edge_end.x);
    let py2 = i128::from(edge_start.y) + i128::from(edge_end.y);
    let mut inside = false;

    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        let ax2 = i128::from(a.x) * 2;
        let ay2 = i128::from(a.y) * 2;
        let bx2 = i128::from(b.x) * 2;
        let by2 = i128::from(b.y) * 2;
        let dx = i128::from(b.x) - i128::from(a.x);
        let dy = i128::from(b.y) - i128::from(a.y);
        let point_dx = px2
            .checked_sub(ax2)
            .ok_or(GeometryError::ArithmeticOverflow)?;
        let point_dy = py2
            .checked_sub(ay2)
            .ok_or(GeometryError::ArithmeticOverflow)?;
        let left = dx
            .checked_mul(point_dy)
            .ok_or(GeometryError::ArithmeticOverflow)?;
        let right = dy
            .checked_mul(point_dx)
            .ok_or(GeometryError::ArithmeticOverflow)?;
        let orientation2 = left
            .checked_sub(right)
            .ok_or(GeometryError::ArithmeticOverflow)?;

        if orientation2 == 0
            && px2 >= ax2.min(bx2)
            && px2 <= ax2.max(bx2)
            && py2 >= ay2.min(by2)
            && py2 <= ay2.max(by2)
        {
            return Ok(PointLocation::Boundary);
        }

        if (ay2 > py2) != (by2 > py2) {
            let crosses_right = if b.y > a.y {
                orientation2 > 0
            } else {
                orientation2 < 0
            };
            if crosses_right {
                inside = !inside;
            }
        }
    }

    Ok(if inside { PointLocation::Inside } else { PointLocation::Outside })
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
        let inscribed_diamond = polygon(&[(2, 0), (4, 2), (2, 4), (0, 2)]);
        let square = rectangle(0, 0, 4, 4);
        assert_eq!(
            square.relation(&inscribed_diamond),
            Ok(PolygonRelation::PositiveAreaOverlap)
        );
        assert_eq!(
            inscribed_diamond.relation(&square),
            Ok(PolygonRelation::PositiveAreaOverlap)
        );
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

    // Independent raster oracle for orthogonal integer-grid polygons. The
    // oracle classifies positive-area overlap by shared unit cells and boundary
    // contact by inclusive axis-aligned segment checks; it does not call any
    // production orientation, segment, point-location, or polygon-relation API.
    fn raster_inside(vertices: &[(i64, i64)], cell_x: i64, cell_y: i64) -> bool {
        let point_x2 = i128::from(cell_x) * 2 + 1;
        let point_y2 = i128::from(cell_y) * 2 + 1;
        let mut inside = false;
        for index in 0..vertices.len() {
            let a = vertices[index];
            let b = vertices[(index + 1) % vertices.len()];
            let ay2 = i128::from(a.1) * 2;
            let by2 = i128::from(b.1) * 2;
            if (ay2 > point_y2) == (by2 > point_y2) {
                continue;
            }
            let ax2 = i128::from(a.0) * 2;
            let bx2 = i128::from(b.0) * 2;
            let dx = i128::from(b.0) - i128::from(a.0);
            let dy = i128::from(b.1) - i128::from(a.1);
            let cross_twice = dx * (point_y2 - ay2) - dy * (point_x2 - ax2);
            let crosses_right = if b.1 > a.1 {
                cross_twice > 0
            } else {
                cross_twice < 0
            };
            if crosses_right {
                inside = !inside;
            }
        }
        inside
    }

    fn raster_cells(vertices: &[(i64, i64)]) -> BTreeSet<(i64, i64)> {
        let min_x = vertices.iter().map(|point| point.0).min().unwrap();
        let max_x = vertices.iter().map(|point| point.0).max().unwrap();
        let min_y = vertices.iter().map(|point| point.1).min().unwrap();
        let max_y = vertices.iter().map(|point| point.1).max().unwrap();
        let mut cells = BTreeSet::new();
        for x in min_x..max_x {
            for y in min_y..max_y {
                if raster_inside(vertices, x, y) {
                    cells.insert((x, y));
                }
            }
        }
        cells
    }

    fn closed_interval_overlap(a0: i64, a1: i64, b0: i64, b1: i64) -> bool {
        a0.min(a1) <= b0.max(b1) && b0.min(b1) <= a0.max(a1)
    }

    fn independent_boundary_contact(
        first: &[(i64, i64)],
        second: &[(i64, i64)],
    ) -> bool {
        for first_index in 0..first.len() {
            let a = first[first_index];
            let b = first[(first_index + 1) % first.len()];
            for second_index in 0..second.len() {
                let c = second[second_index];
                let d = second[(second_index + 1) % second.len()];
                let first_horizontal = a.1 == b.1;
                let second_horizontal = c.1 == d.1;
                let contact = match (first_horizontal, second_horizontal) {
                    (true, true) => {
                        a.1 == c.1 && closed_interval_overlap(a.0, b.0, c.0, d.0)
                    }
                    (false, false) => {
                        a.0 == c.0 && closed_interval_overlap(a.1, b.1, c.1, d.1)
                    }
                    (true, false) => {
                        closed_interval_overlap(a.0, b.0, c.0, d.0)
                            && closed_interval_overlap(a.1, b.1, c.1, d.1)
                    }
                    (false, true) => {
                        closed_interval_overlap(a.0, b.0, c.0, d.0)
                            && closed_interval_overlap(a.1, b.1, c.1, d.1)
                    }
                };
                if contact {
                    return true;
                }
            }
        }
        false
    }

    fn orthogonal_oracle(
        first: &[(i64, i64)],
        second: &[(i64, i64)],
    ) -> PolygonRelation {
        let first_cells = raster_cells(first);
        let second_cells = raster_cells(second);
        if !first_cells.is_disjoint(&second_cells) {
            PolygonRelation::PositiveAreaOverlap
        } else if independent_boundary_contact(first, second) {
            PolygonRelation::BoundaryContact
        } else {
            PolygonRelation::Disjoint
        }
    }

    #[test]
    fn concave_orthogonal_polygons_match_independent_raster_and_segment_oracle() {
        let shapes: &[&[(i64, i64)]] = &[
            &[(0, 0), (3, 0), (3, 1), (1, 1), (1, 3), (0, 3)],
            &[(0, 0), (4, 0), (4, 4), (3, 4), (3, 1), (1, 1), (1, 4), (0, 4)],
            &[(0, 0), (3, 0), (3, 1), (2, 1), (2, 2), (1, 2), (1, 3), (0, 3)],
        ];
        let mut corpus: Vec<(Vec<(i64, i64)>, Polygon)> = Vec::new();
        for shape in shapes {
            for shift_x in 0..=2 {
                for shift_y in 0..=2 {
                    let mut vertices: Vec<(i64, i64)> = shape
                        .iter()
                        .map(|(x, y)| (*x + shift_x, *y + shift_y))
                        .collect();
                    let polygon = Polygon::new(
                        vertices.iter().map(|(x, y)| p(*x, *y)).collect(),
                    )
                    .expect("authored orthogonal shape should be valid");
                    corpus.push((vertices.clone(), polygon));

                    vertices.reverse();
                    let reversed = Polygon::new(
                        vertices.iter().map(|(x, y)| p(*x, *y)).collect(),
                    )
                    .expect("reversed simple shape should remain valid");
                    corpus.push((vertices, reversed));
                }
            }
        }

        for (first_vertices, first_polygon) in &corpus {
            for (second_vertices, second_polygon) in &corpus {
                let expected = orthogonal_oracle(first_vertices, second_vertices);
                assert_eq!(
                    first_polygon.relation(second_polygon),
                    Ok(expected),
                    "independent oracle mismatch: first={first_vertices:?}, second={second_vertices:?}"
                );
                assert_eq!(
                    first_polygon.relation(second_polygon),
                    second_polygon.relation(first_polygon),
                    "polygon relation must be symmetric"
                );
            }
        }
    }

    #[test]
    fn raster_oracle_has_frozen_positive_negative_and_contact_controls() {
        let l_shape = [(0, 0), (3, 0), (3, 1), (1, 1), (1, 3), (0, 3)];
        let disjoint = [(4, 0), (6, 0), (6, 2), (4, 2)];
        let boundary_touch = [(3, 0), (4, 0), (4, 1), (3, 1)];
        let overlap = [(0, 2), (1, 2), (1, 3), (0, 3)];
        assert_eq!(orthogonal_oracle(&l_shape, &disjoint), PolygonRelation::Disjoint);
        assert_eq!(orthogonal_oracle(&l_shape, &boundary_touch), PolygonRelation::BoundaryContact);
        assert_eq!(orthogonal_oracle(&l_shape, &overlap), PolygonRelation::PositiveAreaOverlap);
    }

    #[test]
    fn concave_footprints_obey_half_open_clearance_and_boundary_contact_rules() {
        let l_shape = polygon(&[(0, 0), (3, 0), (3, 1), (1, 1), (1, 3), (0, 3)]);
        let same_shape = polygon(&[(0, 0), (3, 0), (3, 1), (1, 1), (1, 3), (0, 3)]);
        let outgoing = TimeWindow::new(0, 5).unwrap();
        let incoming = TimeWindow::new(5, 10).unwrap();
        let delayed = TimeWindow::new(4, 10).unwrap();
        assert_eq!(
            reservations_conflict(&l_shape, outgoing, &same_shape, incoming),
            Ok(false)
        );
        assert_eq!(
            reservations_conflict(&l_shape, outgoing, &same_shape, delayed),
            Ok(true)
        );

        // Positive-area geometry is disjoint here, but the vertical edges
        // share a boundary; the V0 policy conservatively treats contact as a
        // conflict while the reservations overlap in time.
        let edge_neighbor = rectangle(3, 0, 4, 1);
        assert_eq!(
            l_shape.relation(&edge_neighbor),
            Ok(PolygonRelation::BoundaryContact)
        );
        assert_eq!(
            reservations_conflict(&l_shape, outgoing, &edge_neighbor, TimeWindow::new(1, 4).unwrap()),
            Ok(true)
        );
    }

    #[test]
    fn positive_area_overlap_is_symmetric_for_adversarial_concave_shapes() {
        let l_shape = polygon(&[(0, 0), (4, 0), (4, 1), (1, 1), (1, 4), (0, 4)]);
        let bar = rectangle(0, 2, 3, 3);
        assert_eq!(l_shape.relation(&bar), Ok(PolygonRelation::PositiveAreaOverlap));
        assert_eq!(bar.relation(&l_shape), Ok(PolygonRelation::PositiveAreaOverlap));
    }
}
