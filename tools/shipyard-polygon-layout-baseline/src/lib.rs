use shipyard_polygon_geometry::{GeometryError, Point, Polygon};
use std::collections::BTreeSet;
use std::fmt;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Yard {
    width: i64,
    height: i64,
}

impl Yard {
    pub fn new(width: i64, height: i64) -> Result<Self, LayoutError> {
        if width <= 0 || height <= 0 {
            return Err(LayoutError::InvalidYard { width, height });
        }
        Ok(Self { width, height })
    }

    pub const fn width(&self) -> i64 {
        self.width
    }

    pub const fn height(&self) -> i64 {
        self.height
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BlockRequest {
    id: String,
    footprint: Polygon,
}

impl BlockRequest {
    /// Construct a request whose polygon is expressed in normalized local
    /// coordinates: minimum x and y must both be zero. V0 has no rotation or
    /// reflection search; those transformations must be modeled explicitly in a
    /// future version and pinned in the benchmark manifest.
    pub fn new(id: impl Into<String>, footprint: Polygon) -> Result<Self, LayoutError> {
        let id = id.into();
        if id.trim().is_empty() {
            return Err(LayoutError::EmptyBlockId);
        }
        let min_x = footprint.vertices().iter().map(|point| point.x).min().unwrap_or(0);
        let min_y = footprint.vertices().iter().map(|point| point.y).min().unwrap_or(0);
        if min_x != 0 || min_y != 0 {
            return Err(LayoutError::FootprintNotNormalized { min_x, min_y });
        }
        Ok(Self { id, footprint })
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    pub fn footprint(&self) -> &Polygon {
        &self.footprint
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Policy {
    /// Safe but over-conservative: any closed intersection of axis-aligned
    /// bounding boxes counts as a conflict.
    ConservativeAabb,
    /// Use the exact checked-integer polygon relation; boundary contact remains
    /// a conflict under the geometry crate's declared policy.
    ExactPolygon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Placement {
    pub block_id: String,
    pub origin: Point,
    pub footprint: Polygon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LayoutPlan {
    pub policy: Policy,
    pub placements: Vec<Placement>,
    /// Requests not placed by this deterministic heuristic. This is not a
    /// global infeasibility proof: it is only exhaustion of this request order
    /// and candidate-order policy.
    pub unplaced_ids: Vec<String>,
    /// Deterministic operation count for fair same-input comparisons. This is
    /// not a wall-clock runtime measurement.
    pub candidates_examined: usize,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LayoutError {
    InvalidYard { width: i64, height: i64 },
    EmptyBlockId,
    DuplicateBlockId(String),
    FootprintNotNormalized { min_x: i64, min_y: i64 },
    BlockTooLarge { id: String, width: i64, height: i64 },
    CoordinateOverflow,
    Geometry(GeometryError),
}

impl fmt::Display for LayoutError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidYard { width, height } => {
                write!(f, "yard dimensions must be positive, got {width}x{height}")
            }
            Self::EmptyBlockId => write!(f, "block ID must not be empty"),
            Self::DuplicateBlockId(id) => write!(f, "duplicate block ID {id}"),
            Self::FootprintNotNormalized { min_x, min_y } => {
                write!(f, "local polygon origin must be (0,0), got minimum ({min_x},{min_y})")
            }
            Self::BlockTooLarge { id, width, height } => {
                write!(f, "block {id} footprint {width}x{height} exceeds yard")
            }
            Self::CoordinateOverflow => write!(f, "translated polygon coordinate overflow"),
            Self::Geometry(error) => write!(f, "polygon geometry: {error}"),
        }
    }
}

impl std::error::Error for LayoutError {}

impl From<GeometryError> for LayoutError {
    fn from(value: GeometryError) -> Self {
        Self::Geometry(value)
    }
}

/// Deterministic y-major then x-major first-fit packing on an integer placement
/// lattice. Every candidate is checked against already placed footprints under
/// the selected collision policy. No rotation, transfers, time reuse, resources,
/// or global optimization are implied by this V0 static layout baseline.
pub fn first_fit(
    yard: Yard,
    requests: &[BlockRequest],
    policy: Policy,
) -> Result<LayoutPlan, LayoutError> {
    let mut seen = BTreeSet::new();
    for request in requests {
        if !seen.insert(request.id.clone()) {
            return Err(LayoutError::DuplicateBlockId(request.id.clone()));
        }
        let max_x = request.footprint.vertices().iter().map(|point| point.x).max().unwrap_or(0);
        let max_y = request.footprint.vertices().iter().map(|point| point.y).max().unwrap_or(0);
        if max_x > yard.width || max_y > yard.height {
            return Err(LayoutError::BlockTooLarge {
                id: request.id.clone(),
                width: max_x,
                height: max_y,
            });
        }
    }

    let mut plan = LayoutPlan {
        policy,
        placements: Vec::new(),
        unplaced_ids: Vec::new(),
        candidates_examined: 0,
    };

    for request in requests {
        let width = request.footprint.vertices().iter().map(|point| point.x).max().unwrap_or(0);
        let height = request.footprint.vertices().iter().map(|point| point.y).max().unwrap_or(0);
        let mut accepted = None;

        for y in 0..=(yard.height - height) {
            for x in 0..=(yard.width - width) {
                plan.candidates_examined += 1;
                let candidate = translate(&request.footprint, Point::new(x, y))?;
                let mut candidate_conflicts = false;
                for existing in &plan.placements {
                    let conflict = match policy {
                        Policy::ConservativeAabb => aabb_conflicts(&candidate, &existing.footprint),
                        Policy::ExactPolygon => candidate.conflicts_with(&existing.footprint)?,
                    };
                    if conflict {
                        candidate_conflicts = true;
                        break;
                    }
                }
                if candidate_conflicts {
                    continue;
                }
                accepted = Some(Placement {
                    block_id: request.id.clone(),
                    origin: Point::new(x, y),
                    footprint: candidate,
                });
                break;
            }
            if accepted.is_some() {
                break;
            }
        }

        match accepted {
            Some(placement) => plan.placements.push(placement),
            None => plan.unplaced_ids.push(request.id.clone()),
        }
    }

    Ok(plan)
}

fn translate(polygon: &Polygon, origin: Point) -> Result<Polygon, LayoutError> {
    let vertices = polygon
        .vertices()
        .iter()
        .map(|point| {
            Ok(Point::new(
                point.x.checked_add(origin.x).ok_or(LayoutError::CoordinateOverflow)?,
                point.y.checked_add(origin.y).ok_or(LayoutError::CoordinateOverflow)?,
            ))
        })
        .collect::<Result<Vec<_>, LayoutError>>()?;
    Ok(Polygon::new(vertices)?)
}

fn aabb_conflicts(first: &Polygon, second: &Polygon) -> bool {
    let bounds = |polygon: &Polygon| {
        let vertices = polygon.vertices();
        (
            vertices.iter().map(|point| point.x).min().unwrap_or(0),
            vertices.iter().map(|point| point.y).min().unwrap_or(0),
            vertices.iter().map(|point| point.x).max().unwrap_or(0),
            vertices.iter().map(|point| point.y).max().unwrap_or(0),
        )
    };
    let (ax0, ay0, ax1, ay1) = bounds(first);
    let (bx0, by0, bx1, by1) = bounds(second);
    ax0 <= bx1 && bx0 <= ax1 && ay0 <= by1 && by0 <= ay1
}

#[cfg(test)]
mod tests {
    use super::*;

    fn l_shape() -> Polygon {
        Polygon::new(vec![
            Point::new(0, 0),
            Point::new(3, 0),
            Point::new(3, 1),
            Point::new(1, 1),
            Point::new(1, 3),
            Point::new(0, 3),
        ])
        .unwrap()
    }

    fn request(id: &str) -> BlockRequest {
        BlockRequest::new(id, l_shape()).unwrap()
    }

    fn independent_cells(vertices: &[(i64, i64)]) -> BTreeSet<(i64, i64)> {
        let min_x = vertices.iter().map(|point| point.0).min().unwrap();
        let max_x = vertices.iter().map(|point| point.0).max().unwrap();
        let min_y = vertices.iter().map(|point| point.1).min().unwrap();
        let max_y = vertices.iter().map(|point| point.1).max().unwrap();
        let mut cells = BTreeSet::new();
        for x in min_x..max_x {
            for y in min_y..max_y {
                let px2 = 2 * x + 1;
                let py2 = 2 * y + 1;
                let mut inside = false;
                for index in 0..vertices.len() {
                    let a = vertices[index];
                    let b = vertices[(index + 1) % vertices.len()];
                    if (2 * a.1 > py2) == (2 * b.1 > py2) {
                        continue;
                    }
                    let cross = (b.0 - a.0) * (py2 - 2 * a.1)
                        - (b.1 - a.1) * (px2 - 2 * a.0);
                    let right = if b.1 > a.1 { cross > 0 } else { cross < 0 };
                    if right {
                        inside = !inside;
                    }
                }
                if inside {
                    cells.insert((x, y));
                }
            }
        }
        cells
    }

    fn closed_overlap(a0: i64, a1: i64, b0: i64, b1: i64) -> bool {
        a0.min(a1) <= b0.max(b1) && b0.min(b1) <= a0.max(a1)
    }

    fn independent_boundary_contact(
        first: &[(i64, i64)],
        second: &[(i64, i64)],
    ) -> bool {
        for i in 0..first.len() {
            let a = first[i];
            let b = first[(i + 1) % first.len()];
            for j in 0..second.len() {
                let c = second[j];
                let d = second[(j + 1) % second.len()];
                let first_horizontal = a.1 == b.1;
                let second_horizontal = c.1 == d.1;
                let contact = match (first_horizontal, second_horizontal) {
                    (true, true) => a.1 == c.1 && closed_overlap(a.0, b.0, c.0, d.0),
                    (false, false) => a.0 == c.0 && closed_overlap(a.1, b.1, c.1, d.1),
                    _ => closed_overlap(a.0, b.0, c.0, d.0)
                        && closed_overlap(a.1, b.1, c.1, d.1),
                };
                if contact {
                    return true;
                }
            }
        }
        false
    }

    fn independent_conflict(first: &Placement, second: &Placement) -> bool {
        let to_tuples = |polygon: &Polygon| {
            polygon.vertices().iter().map(|point| (point.x, point.y)).collect::<Vec<_>>()
        };
        let a = to_tuples(&first.footprint);
        let b = to_tuples(&second.footprint);
        !independent_cells(&a).is_disjoint(&independent_cells(&b))
            || independent_boundary_contact(&a, &b)
    }

    #[test]
    fn polygon_first_fit_finds_a_valid_two_block_layout_aabb_rejects() {
        let yard = Yard::new(6, 6).unwrap();
        let requests = vec![request("block-a"), request("block-b")];
        let aabb = first_fit(yard, &requests, Policy::ConservativeAabb).unwrap();
        let exact = first_fit(yard, &requests, Policy::ExactPolygon).unwrap();

        assert_eq!(aabb.placements.len(), 1);
        assert_eq!(aabb.unplaced_ids, vec!["block-b"]);
        assert_eq!(aabb.candidates_examined, 17);

        assert_eq!(exact.placements.len(), 2);
        assert!(exact.unplaced_ids.is_empty());
        assert_eq!(exact.candidates_examined, 12);
        assert_eq!(exact.placements[0].origin, Point::new(0, 0));
        assert_eq!(exact.placements[1].origin, Point::new(2, 2));
        assert!(!independent_conflict(&exact.placements[0], &exact.placements[1]));
    }

    #[test]
    fn repeated_inputs_produce_identical_layout_and_operation_counts() {
        let yard = Yard::new(6, 6).unwrap();
        let requests = vec![request("a"), request("b")];
        let first = first_fit(yard, &requests, Policy::ExactPolygon).unwrap();
        let second = first_fit(yard, &requests, Policy::ExactPolygon).unwrap();
        assert_eq!(first, second);
    }

    #[test]
    fn heuristic_exhaustion_is_reported_as_unplaced_not_global_infeasibility() {
        let yard = Yard::new(3, 3).unwrap();
        let requests = vec![request("a"), request("b")];
        let plan = first_fit(yard, &requests, Policy::ExactPolygon).unwrap();
        assert_eq!(plan.placements.len(), 1);
        assert_eq!(plan.unplaced_ids, vec!["b"]);
        // LayoutPlan deliberately contains no "proven infeasible" field.
    }

    #[test]
    fn duplicate_ids_and_invalid_yard_dimensions_fail_closed() {
        let duplicate = vec![request("same"), request("same")];
        assert_eq!(
            first_fit(Yard::new(6, 6).unwrap(), &duplicate, Policy::ExactPolygon),
            Err(LayoutError::DuplicateBlockId("same".to_owned()))
        );
        assert_eq!(
            Yard::new(0, 6),
            Err(LayoutError::InvalidYard { width: 0, height: 6 })
        );
    }

    #[test]
    fn non_normalized_polygon_origin_is_rejected() {
        let shifted = Polygon::new(vec![
            Point::new(1, 0), Point::new(4, 0), Point::new(4, 1),
            Point::new(2, 1), Point::new(2, 3), Point::new(1, 3),
        ])
        .unwrap();
        assert_eq!(
            BlockRequest::new("shifted", shifted),
            Err(LayoutError::FootprintNotNormalized { min_x: 1, min_y: 0 })
        );
    }
}
