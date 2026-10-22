//! Bounded exact 2D simple-polygon geometry for future irregular ship-block scheduling.
//!
//! This isolated prototype does not yet extend shipyard-space-time::Footprint. It establishes a
//! separately testable geometry kernel first. Boundary-only contact is legal; positive-area
//! intersection is overlap. Coordinates are integer yard units bounded to ±1,000,000,000 so all
//! orientation and ear-clipping arithmetic has a declared exact range.

use shipyard_space_time::Rect;

pub const MAX_COORDINATE_MAGNITUDE: i64 = 1_000_000_000;
pub const MAX_POLYGON_VERTICES: usize = 128;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct Point2 {
    pub x: i64,
    pub y: i64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SimplePolygon {
    pub vertices: Vec<Point2>,
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum GeometryError {
    InvalidBounds,
    TooFewVertices,
    TooManyVertices,
    CoordinateMagnitudeExceeded,
    RepeatedVertex,
    ZeroArea,
    SelfIntersection,
    ArithmeticOverflow,
    TriangulationFailed,
}

impl GeometryError {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::InvalidBounds => "InvalidBounds",
            Self::TooFewVertices => "TooFewVertices",
            Self::TooManyVertices => "TooManyVertices",
            Self::CoordinateMagnitudeExceeded => "CoordinateMagnitudeExceeded",
            Self::RepeatedVertex => "RepeatedVertex",
            Self::ZeroArea => "ZeroArea",
            Self::SelfIntersection => "SelfIntersection",
            Self::ArithmeticOverflow => "ArithmeticOverflow",
            Self::TriangulationFailed => "TriangulationFailed",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PointLocation {
    Inside,
    Boundary,
    Outside,
}

fn within_coordinate_bound(point: Point2) -> bool {
    (-MAX_COORDINATE_MAGNITUDE..=MAX_COORDINATE_MAGNITUDE).contains(&point.x)
        && (-MAX_COORDINATE_MAGNITUDE..=MAX_COORDINATE_MAGNITUDE).contains(&point.y)
}

fn cross(a: Point2, b: Point2, c: Point2) -> i128 {
    (i128::from(b.x) - i128::from(a.x))
        * (i128::from(c.y) - i128::from(a.y))
        - (i128::from(b.y) - i128::from(a.y))
            * (i128::from(c.x) - i128::from(a.x))
}

fn dot_turn(a: Point2, b: Point2, c: Point2) -> i128 {
    (i128::from(b.x) - i128::from(a.x))
        * (i128::from(c.x) - i128::from(b.x))
        + (i128::from(b.y) - i128::from(a.y))
            * (i128::from(c.y) - i128::from(b.y))
}

fn on_segment(point: Point2, a: Point2, b: Point2) -> bool {
    cross(a, b, point) == 0
        && point.x >= a.x.min(b.x)
        && point.x <= a.x.max(b.x)
        && point.y >= a.y.min(b.y)
        && point.y <= a.y.max(b.y)
}

fn orientation_sign(value: i128) -> i8 {
    if value > 0 {
        1
    } else if value < 0 {
        -1
    } else {
        0
    }
}

fn segments_intersect_inclusive(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let o1 = orientation_sign(cross(a, b, c));
    let o2 = orientation_sign(cross(a, b, d));
    let o3 = orientation_sign(cross(c, d, a));
    let o4 = orientation_sign(cross(c, d, b));

    if o1 != 0 && o2 != 0 && o1 != o2 && o3 != 0 && o4 != 0 && o3 != o4 {
        return true;
    }

    (o1 == 0 && on_segment(c, a, b))
        || (o2 == 0 && on_segment(d, a, b))
        || (o3 == 0 && on_segment(a, c, d))
        || (o4 == 0 && on_segment(b, c, d))
}

fn segments_properly_intersect(a: Point2, b: Point2, c: Point2, d: Point2) -> bool {
    let o1 = orientation_sign(cross(a, b, c));
    let o2 = orientation_sign(cross(a, b, d));
    let o3 = orientation_sign(cross(c, d, a));
    let o4 = orientation_sign(cross(c, d, b));
    o1 != 0 && o2 != 0 && o1 != o2 && o3 != 0 && o4 != 0 && o3 != o4
}

fn doubled_point_location(px2: i128, py2: i128, polygon: &[Point2]) -> PointLocation {
    let mut inside = false;
    for index in 0..polygon.len() {
        let a = polygon[index];
        let b = polygon[(index + 1) % polygon.len()];
        let ax2 = i128::from(a.x) * 2;
        let ay2 = i128::from(a.y) * 2;
        let bx2 = i128::from(b.x) * 2;
        let by2 = i128::from(b.y) * 2;

        let cross_scaled = (bx2 - ax2) * (py2 - ay2) - (by2 - ay2) * (px2 - ax2);
        if cross_scaled == 0
            && px2 >= ax2.min(bx2)
            && px2 <= ax2.max(bx2)
            && py2 >= ay2.min(by2)
            && py2 <= ay2.max(by2)
        {
            return PointLocation::Boundary;
        }

        if (ay2 > py2) != (by2 > py2) {
            // With this cross-product orientation, a ray crossing to the right has opposite
            // signs depending on whether the segment rises or falls.
            if (by2 > ay2 && cross_scaled < 0) || (by2 < ay2 && cross_scaled > 0) {
                inside = !inside;
            }
        }
    }
    if inside {
        PointLocation::Inside
    } else {
        PointLocation::Outside
    }
}

fn doubled_point_in_triangle(px2: i128, py2: i128, triangle: [Point2; 3]) -> bool {
    let p = [
        triangle[0],
        triangle[1],
        triangle[2],
    ];
    let mut signs = [0_i8; 3];
    for index in 0..3 {
        let a = p[index];
        let b = p[(index + 1) % 3];
        let ax2 = i128::from(a.x) * 2;
        let ay2 = i128::from(a.y) * 2;
        let bx2 = i128::from(b.x) * 2;
        let by2 = i128::from(b.y) * 2;
        signs[index] = orientation_sign(
            (bx2 - ax2) * (py2 - ay2) - (by2 - ay2) * (px2 - ax2),
        );
    }
    signs.iter().all(|sign| *sign >= 0) || signs.iter().all(|sign| *sign <= 0)
}

impl SimplePolygon {
    pub fn new(vertices: Vec<Point2>) -> Result<Self, GeometryError> {
        let polygon = Self { vertices };
        polygon.validate()?;
        Ok(polygon)
    }

    /// Validate a simple closed ring, rejecting self-touching and self-crossing polygons.
    pub fn validate(&self) -> Result<(), GeometryError> {
        let count = self.vertices.len();
        if count < 3 {
            return Err(GeometryError::TooFewVertices);
        }
        if count > MAX_POLYGON_VERTICES {
            return Err(GeometryError::TooManyVertices);
        }
        if self.vertices.iter().any(|point| !within_coordinate_bound(*point)) {
            return Err(GeometryError::CoordinateMagnitudeExceeded);
        }

        let mut unique = std::collections::BTreeSet::new();
        for point in &self.vertices {
            if !unique.insert(*point) {
                return Err(GeometryError::RepeatedVertex);
            }
        }

        let area2 = self.signed_area2();
        if area2 == 0 {
            return Err(GeometryError::ZeroArea);
        }

        // Adjacent edges may continue straight but cannot reverse and overlap each other.
        for index in 0..count {
            let prior = self.vertices[(index + count - 1) % count];
            let current = self.vertices[index];
            let next = self.vertices[(index + 1) % count];
            if cross(prior, current, next) == 0 && dot_turn(prior, current, next) < 0 {
                return Err(GeometryError::SelfIntersection);
            }
        }

        // Non-adjacent edges may neither cross nor touch. Adjacent edge pairs share only their
        // common endpoint, except the reverse-overlap case rejected above.
        for left_index in 0..count {
            let left_next = (left_index + 1) % count;
            for right_index in (left_index + 1)..count {
                let right_next = (right_index + 1) % count;
                if left_index == right_index
                    || left_next == right_index
                    || right_next == left_index
                {
                    continue;
                }
                if segments_intersect_inclusive(
                    self.vertices[left_index],
                    self.vertices[left_next],
                    self.vertices[right_index],
                    self.vertices[right_next],
                ) {
                    return Err(GeometryError::SelfIntersection);
                }
            }
        }
        Ok(())
    }

    pub fn signed_area2(&self) -> i128 {
        let mut area = 0_i128;
        for index in 0..self.vertices.len() {
            let a = self.vertices[index];
            let b = self.vertices[(index + 1) % self.vertices.len()];
            area += i128::from(a.x) * i128::from(b.y)
                - i128::from(a.y) * i128::from(b.x);
        }
        area
    }

    pub fn bounds(&self) -> Result<Rect, GeometryError> {
        self.validate()?;
        let min_x = self.vertices.iter().map(|point| point.x).min().ok_or(
            GeometryError::TooFewVertices,
        )?;
        let max_x = self.vertices.iter().map(|point| point.x).max().ok_or(
            GeometryError::TooFewVertices,
        )?;
        let min_y = self.vertices.iter().map(|point| point.y).min().ok_or(
            GeometryError::TooFewVertices,
        )?;
        let max_y = self.vertices.iter().map(|point| point.y).max().ok_or(
            GeometryError::TooFewVertices,
        )?;
        let bounds = Rect {
            min_x,
            min_y,
            max_x,
            max_y,
        };
        if bounds.is_valid() {
            Ok(bounds)
        } else {
            Err(GeometryError::ZeroArea)
        }
    }

    /// Since a rectangle is convex, checking every vertex proves the polygon stays inside it.
    pub fn is_within_rect(&self, rect: Rect) -> Result<bool, GeometryError> {
        self.validate()?;
        if !rect.is_valid() {
            return Err(GeometryError::InvalidBounds);
        }
        Ok(self.vertices.iter().all(|point| {
            point.x >= rect.min_x
                && point.x <= rect.max_x
                && point.y >= rect.min_y
                && point.y <= rect.max_y
        }))
    }

    /// True when the polygon interiors share positive area. Edge/corner touching alone is legal.
    pub fn interior_overlaps(&self, other: &Self) -> Result<bool, GeometryError> {
        let left_triangles = self.triangulate()?;
        let right_triangles = other.triangulate()?;
        for left in &left_triangles {
            for right in &right_triangles {
                if triangles_overlap_interior(*left, *right) {
                    return Ok(true);
                }
            }
        }
        Ok(false)
    }

    pub fn interior_overlaps_rect(&self, rect: Rect) -> Result<bool, GeometryError> {
        if !rect.is_valid() {
            return Err(GeometryError::InvalidBounds);
        }
        let rectangle = Self::new(vec![
            Point2 { x: rect.min_x, y: rect.min_y },
            Point2 { x: rect.max_x, y: rect.min_y },
            Point2 { x: rect.max_x, y: rect.max_y },
            Point2 { x: rect.min_x, y: rect.max_y },
        ])?;
        self.interior_overlaps(&rectangle)
    }

    fn normalized_vertices(&self) -> Result<Vec<Point2>, GeometryError> {
        self.validate()?;
        let mut vertices = self.vertices.clone();
        loop {
            if vertices.len() <= 3 {
                break;
            }
            let mut removable = None;
            for index in 0..vertices.len() {
                let prior = vertices[(index + vertices.len() - 1) % vertices.len()];
                let current = vertices[index];
                let next = vertices[(index + 1) % vertices.len()];
                if cross(prior, current, next) == 0 && dot_turn(prior, current, next) > 0 {
                    removable = Some(index);
                    break;
                }
            }
            if let Some(index) = removable {
                vertices.remove(index);
            } else {
                break;
            }
        }
        if vertices.len() < 3 {
            return Err(GeometryError::ZeroArea);
        }
        Ok(vertices)
    }

    fn triangulate(&self) -> Result<Vec<[Point2; 3]>, GeometryError> {
        let vertices = self.normalized_vertices()?;
        let orientation = orientation_sign(self.signed_area2());
        if orientation == 0 {
            return Err(GeometryError::ZeroArea);
        }
        let mut indices: Vec<usize> = (0..vertices.len()).collect();
        let mut triangles = Vec::with_capacity(vertices.len() - 2);
        let mut iterations = 0usize;
        let iteration_limit = vertices.len().saturating_mul(vertices.len());

        while indices.len() > 3 {
            let mut ear_found = false;
            for cursor in 0..indices.len() {
                let previous = indices[(cursor + indices.len() - 1) % indices.len()];
                let current = indices[cursor];
                let next = indices[(cursor + 1) % indices.len()];
                if orientation_sign(cross(
                    vertices[previous],
                    vertices[current],
                    vertices[next],
                )) != orientation
                {
                    continue;
                }

                let triangle = [
                    vertices[previous],
                    vertices[current],
                    vertices[next],
                ];
                let contains_other_vertex = indices.iter().copied().any(|candidate| {
                    candidate != previous
                        && candidate != current
                        && candidate != next
                        && doubled_point_in_triangle(
                            i128::from(vertices[candidate].x) * 2,
                            i128::from(vertices[candidate].y) * 2,
                            triangle,
                        )
                });
                if contains_other_vertex {
                    continue;
                }

                triangles.push(triangle);
                indices.remove(cursor);
                ear_found = true;
                break;
            }
            iterations += 1;
            if !ear_found || iterations > iteration_limit {
                return Err(GeometryError::TriangulationFailed);
            }
        }

        triangles.push([
            vertices[indices[0]],
            vertices[indices[1]],
            vertices[indices[2]],
        ]);
        Ok(triangles)
    }
}

fn point_location(point: Point2, polygon: &[Point2]) -> PointLocation {
    doubled_point_location(
        i128::from(point.x) * 2,
        i128::from(point.y) * 2,
        polygon,
    )
}

fn triangles_overlap_interior(left: [Point2; 3], right: [Point2; 3]) -> bool {
    for left_index in 0..3 {
        let a = left[left_index];
        let b = left[(left_index + 1) % 3];
        for right_index in 0..3 {
            let c = right[right_index];
            let d = right[(right_index + 1) % 3];
            if segments_properly_intersect(a, b, c, d) {
                return true;
            }
        }
    }

    if left
        .iter()
        .any(|point| point_location(*point, &right) == PointLocation::Inside)
        || right
            .iter()
            .any(|point| point_location(*point, &left) == PointLocation::Inside)
    {
        return true;
    }

    // Identical triangles have no strictly interior vertex and no proper edge crossing.
    let left_area = cross(left[0], left[1], left[2]).abs();
    let right_area = cross(right[0], right[1], right[2]).abs();
    left_area == right_area
        && left
            .iter()
            .all(|point| point_location(*point, &right) == PointLocation::Boundary)
        && right
            .iter()
            .all(|point| point_location(*point, &left) == PointLocation::Boundary)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn poly(points: &[(i64, i64)]) -> SimplePolygon {
        SimplePolygon::new(
            points
                .iter()
                .map(|(x, y)| Point2 { x: *x, y: *y })
                .collect(),
        )
        .expect("fixture polygon should be valid")
    }

    #[test]
    fn concave_l_shape_overlaps_its_arms_but_not_its_missing_corner() {
        let l_shape = poly(&[(0, 0), (4, 0), (4, 1), (1, 1), (1, 4), (0, 4)]);
        assert!(
            l_shape
                .interior_overlaps_rect(Rect { min_x: 0, min_y: 2, max_x: 1, max_y: 3 })
                .unwrap()
        );
        assert!(
            !l_shape
                .interior_overlaps_rect(Rect { min_x: 2, min_y: 2, max_x: 3, max_y: 3 })
                .unwrap()
        );
    }

    #[test]
    fn positive_area_crossing_and_containment_are_overlap() {
        let triangle = poly(&[(0, 0), (6, 0), (3, 6)]);
        let crossing = poly(&[(2, 2), (5, 2), (4, 5)]);
        let contained = poly(&[(2, 1), (4, 1), (3, 2)]);
        assert!(triangle.interior_overlaps(&crossing).unwrap());
        assert!(triangle.interior_overlaps(&contained).unwrap());
    }

    #[test]
    fn common_edges_and_corner_contact_do_not_count_as_overlap() {
        let left = poly(&[(0, 0), (2, 0), (2, 2), (0, 2)]);
        let edge_touch = poly(&[(2, 0), (4, 0), (4, 2), (2, 2)]);
        let corner_touch = poly(&[(2, 2), (4, 2), (4, 4), (2, 4)]);
        assert!(!left.interior_overlaps(&edge_touch).unwrap());
        assert!(!left.interior_overlaps(&corner_touch).unwrap());
    }

    #[test]
    fn same_polygon_is_overlap_under_cyclic_shift_and_reversed_winding() {
        let first = poly(&[(0, 0), (5, 0), (5, 3), (0, 3)]);
        let shifted = poly(&[(5, 3), (0, 3), (0, 0), (5, 0)]);
        let reversed = poly(&[(0, 3), (5, 3), (5, 0), (0, 0)]);
        assert!(first.interior_overlaps(&shifted).unwrap());
        assert!(first.interior_overlaps(&reversed).unwrap());
    }

    #[test]
    fn rejects_self_crossing_degenerate_and_out_of_range_polygons() {
        let bow_tie = SimplePolygon {
            vertices: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 4, y: 4 },
                Point2 { x: 0, y: 4 },
                Point2 { x: 4, y: 0 },
            ],
        };
        assert_eq!(bow_tie.validate(), Err(GeometryError::ZeroArea));

        let repeated = SimplePolygon {
            vertices: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: 4, y: 0 },
                Point2 { x: 4, y: 4 },
                Point2 { x: 4, y: 0 },
            ],
        };
        assert_eq!(repeated.validate(), Err(GeometryError::RepeatedVertex));

        let outside = SimplePolygon {
            vertices: vec![
                Point2 { x: 0, y: 0 },
                Point2 { x: MAX_COORDINATE_MAGNITUDE + 1, y: 0 },
                Point2 { x: 0, y: 3 },
            ],
        };
        assert_eq!(
            outside.validate(),
            Err(GeometryError::CoordinateMagnitudeExceeded)
        );
    }

    #[test]
    fn collinear_boundary_vertices_normalize_without_changing_overlap() {
        let with_midpoint = poly(&[(0, 0), (2, 0), (4, 0), (4, 4), (0, 4)]);
        let plain = poly(&[(0, 0), (4, 0), (4, 4), (0, 4)]);
        assert!(with_midpoint.interior_overlaps(&plain).unwrap());
    }

    #[test]
    fn polygon_containment_in_bounds_uses_every_vertex() {
        let inside = poly(&[(1, 1), (8, 1), (3, 8)]);
        let outside = poly(&[(1, 1), (11, 1), (3, 8)]);
        let bounds = Rect { min_x: 0, min_y: 0, max_x: 10, max_y: 10 };
        assert!(inside.is_within_rect(bounds).unwrap());
        assert!(!outside.is_within_rect(bounds).unwrap());
    }
}
