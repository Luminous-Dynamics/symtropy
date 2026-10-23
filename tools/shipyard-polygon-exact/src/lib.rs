use shipyard_polygon_geometry::{GeometryError, Point, Polygon};
use shipyard_polygon_layout_baseline::{BlockRequest, LayoutError, Policy, Yard, first_fit};
use std::collections::BTreeSet;
use std::fmt;

const MAX_REQUESTS: usize = 4;
const MAX_YARD_DIMENSION: i64 = 5;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExactPlacement {
    pub block_id: String,
    pub origin: Point,
    pub footprint: Polygon,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SolveOutcome {
    /// A valid layout was found inside the bounded finite model.
    Feasible {
        placements: Vec<ExactPlacement>,
        candidate_checks: usize,
    },
    /// Every relevant candidate branch was exhausted under this exact model.
    /// This means no complete layout exists for the supplied fixed-orientation
    /// polygons, yard, contact policy and integer translation lattice.
    ProvenInfeasible { candidate_checks: usize },
    /// The solver reached the declared candidate-evaluation budget before
    /// finding a layout or exhausting every branch. This is not infeasibility.
    BudgetExceeded {
        candidate_checks: usize,
        budget: usize,
    },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExactError {
    YardOutsideTinyModel { width: i64, height: i64 },
    TooManyRequests { count: usize, maximum: usize },
    ZeroBudget,
    DuplicateBlockId(String),
    BlockTooLarge { id: String, width: i64, height: i64 },
    CoordinateOverflow,
    Geometry(GeometryError),
    Layout(LayoutError),
}

impl fmt::Display for ExactError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::YardOutsideTinyModel { width, height } => write!(
                f,
                "exact oracle only supports yards up to {MAX_YARD_DIMENSION}x{MAX_YARD_DIMENSION}, got {width}x{height}"
            ),
            Self::TooManyRequests { count, maximum } => {
                write!(
                    f,
                    "exact oracle received {count} requests; maximum is {maximum}"
                )
            }
            Self::ZeroBudget => write!(f, "candidate-check budget must be positive"),
            Self::DuplicateBlockId(id) => write!(f, "duplicate block ID {id}"),
            Self::BlockTooLarge { id, width, height } => {
                write!(f, "block {id} footprint {width}x{height} exceeds yard")
            }
            Self::CoordinateOverflow => write!(f, "translated polygon coordinate overflow"),
            Self::Geometry(error) => write!(f, "polygon geometry: {error}"),
            Self::Layout(error) => write!(f, "layout input: {error}"),
        }
    }
}

impl std::error::Error for ExactError {}

impl From<GeometryError> for ExactError {
    fn from(value: GeometryError) -> Self {
        Self::Geometry(value)
    }
}

impl From<LayoutError> for ExactError {
    fn from(value: LayoutError) -> Self {
        Self::Layout(value)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct Candidate {
    origin: Point,
    footprint: Polygon,
}

struct SearchContext {
    domains: Vec<Vec<Candidate>>,
    candidate_checks: usize,
    budget: usize,
}

struct SearchSuccess {
    placements: Vec<Option<Candidate>>,
}

#[derive(Debug)]
enum SearchResult {
    Feasible(SearchSuccess),
    Infeasible,
    BudgetExceeded,
}

/// Solve tiny polygon packing exactly using dynamic minimum-remaining-values
/// (MRV) branching. Candidate placement order inside a chosen request is
/// deterministic: y-major, then x-major. The request selected at each depth is
/// the unplaced block with the fewest candidates currently compatible with the
/// partial layout; ties preserve original input order.
///
/// This is a tiny-domain verifier/benchmark oracle, not a production optimizer.
/// It only considers fixed-orientation integer translations and the declared
/// conservative boundary-contact policy. A budget result remains unknown.
pub fn solve_exact(
    yard: Yard,
    requests: &[BlockRequest],
    candidate_check_budget: usize,
) -> Result<SolveOutcome, ExactError> {
    if yard.width() > MAX_YARD_DIMENSION || yard.height() > MAX_YARD_DIMENSION {
        return Err(ExactError::YardOutsideTinyModel {
            width: yard.width(),
            height: yard.height(),
        });
    }
    if requests.len() > MAX_REQUESTS {
        return Err(ExactError::TooManyRequests {
            count: requests.len(),
            maximum: MAX_REQUESTS,
        });
    }
    if candidate_check_budget == 0 {
        return Err(ExactError::ZeroBudget);
    }

    let mut seen = BTreeSet::new();
    let mut domains = Vec::with_capacity(requests.len());
    for request in requests {
        if !seen.insert(request.id().to_owned()) {
            return Err(ExactError::DuplicateBlockId(request.id().to_owned()));
        }
        let footprint = request.footprint();
        let width = footprint
            .vertices()
            .iter()
            .map(|point| point.x)
            .max()
            .unwrap_or(0);
        let height = footprint
            .vertices()
            .iter()
            .map(|point| point.y)
            .max()
            .unwrap_or(0);
        if width > yard.width() || height > yard.height() {
            return Err(ExactError::BlockTooLarge {
                id: request.id().to_owned(),
                width,
                height,
            });
        }

        let mut candidates = Vec::new();
        for y in 0..=(yard.height() - height) {
            for x in 0..=(yard.width() - width) {
                candidates.push(Candidate {
                    origin: Point::new(x, y),
                    footprint: translate(footprint, Point::new(x, y))?,
                });
            }
        }
        domains.push(candidates);
    }

    let mut context = SearchContext {
        domains,
        candidate_checks: 0,
        budget: candidate_check_budget,
    };
    let mut chosen = vec![None; requests.len()];
    match search(&mut context, &mut chosen, 0)? {
        SearchResult::Feasible(success) => {
            let placements = success
                .placements
                .into_iter()
                .enumerate()
                .map(|(index, candidate)| {
                    let candidate = candidate.expect("complete search must place each request");
                    ExactPlacement {
                        block_id: requests[index].id().to_owned(),
                        origin: candidate.origin,
                        footprint: candidate.footprint,
                    }
                })
                .collect();
            Ok(SolveOutcome::Feasible {
                placements,
                candidate_checks: context.candidate_checks,
            })
        }
        SearchResult::Infeasible => Ok(SolveOutcome::ProvenInfeasible {
            candidate_checks: context.candidate_checks,
        }),
        SearchResult::BudgetExceeded => Ok(SolveOutcome::BudgetExceeded {
            candidate_checks: context.candidate_checks,
            budget: candidate_check_budget,
        }),
    }
}

fn search(
    context: &mut SearchContext,
    chosen: &mut [Option<Candidate>],
    placed_count: usize,
) -> Result<SearchResult, ExactError> {
    if placed_count == chosen.len() {
        return Ok(SearchResult::Feasible(SearchSuccess {
            placements: chosen.to_vec(),
        }));
    }

    let mut best_index = None;
    let mut best_compatible: Vec<Candidate> = Vec::new();

    for request_index in 0..chosen.len() {
        if chosen[request_index].is_some() {
            continue;
        }

        let mut compatible = Vec::new();
        for candidate_index in 0..context.domains[request_index].len() {
            if context.candidate_checks >= context.budget {
                return Ok(SearchResult::BudgetExceeded);
            }
            context.candidate_checks += 1;
            let candidate = context.domains[request_index][candidate_index].clone();
            if compatible_with_chosen(&candidate, chosen)? {
                compatible.push(candidate);
            }
        }

        // If one remaining request has no legal position, the partial layout
        // cannot be completed. We do not infer global infeasibility until the
        // caller has exhausted every ancestor branch.
        if compatible.is_empty() {
            return Ok(SearchResult::Infeasible);
        }

        let improves = best_index.is_none() || compatible.len() < best_compatible.len();
        if improves {
            best_index = Some(request_index);
            best_compatible = compatible;
        }
    }

    let request_index = best_index.expect("a request remains when placed_count is incomplete");
    let mut saw_budget_exhaustion = false;
    for candidate in best_compatible {
        chosen[request_index] = Some(candidate);
        match search(context, chosen, placed_count + 1)? {
            SearchResult::Feasible(solution) => return Ok(SearchResult::Feasible(solution)),
            SearchResult::Infeasible => {}
            SearchResult::BudgetExceeded => {
                saw_budget_exhaustion = true;
                break;
            }
        }
        chosen[request_index] = None;
    }
    chosen[request_index] = None;

    if saw_budget_exhaustion {
        Ok(SearchResult::BudgetExceeded)
    } else {
        Ok(SearchResult::Infeasible)
    }
}

fn compatible_with_chosen(
    candidate: &Candidate,
    chosen: &[Option<Candidate>],
) -> Result<bool, ExactError> {
    for existing in chosen.iter().flatten() {
        if candidate.footprint.conflicts_with(&existing.footprint)? {
            return Ok(false);
        }
    }
    Ok(true)
}

fn translate(polygon: &Polygon, origin: Point) -> Result<Polygon, ExactError> {
    let vertices = polygon
        .vertices()
        .iter()
        .map(|point| {
            Ok(Point::new(
                point
                    .x
                    .checked_add(origin.x)
                    .ok_or(ExactError::CoordinateOverflow)?,
                point
                    .y
                    .checked_add(origin.y)
                    .ok_or(ExactError::CoordinateOverflow)?,
            ))
        })
        .collect::<Result<Vec<_>, ExactError>>()?;
    Ok(Polygon::new(vertices)?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn p(x: i64, y: i64) -> Point {
        Point::new(x, y)
    }

    fn l_shape() -> Polygon {
        Polygon::new(vec![p(0, 0), p(3, 0), p(3, 1), p(1, 1), p(1, 3), p(0, 3)]).unwrap()
    }

    fn square(size: i64) -> Polygon {
        Polygon::new(vec![p(0, 0), p(size, 0), p(size, size), p(0, size)]).unwrap()
    }

    fn vertical_bar() -> Polygon {
        Polygon::new(vec![p(0, 0), p(1, 0), p(1, 3), p(0, 3)]).unwrap()
    }

    fn request(id: &str, polygon: Polygon) -> BlockRequest {
        BlockRequest::new(id, polygon).unwrap()
    }

    // Independent integer-cell oracle for the orthogonal benchmark fixtures.
    // It does not call the production polygon-relation API.
    fn independent_cells(vertices: &[Point]) -> BTreeSet<(i64, i64)> {
        let min_x = vertices.iter().map(|point| point.x).min().unwrap();
        let max_x = vertices.iter().map(|point| point.x).max().unwrap();
        let min_y = vertices.iter().map(|point| point.y).min().unwrap();
        let max_y = vertices.iter().map(|point| point.y).max().unwrap();
        let mut cells = BTreeSet::new();
        for x in min_x..max_x {
            for y in min_y..max_y {
                let px2 = 2 * x + 1;
                let py2 = 2 * y + 1;
                let mut inside = false;
                for index in 0..vertices.len() {
                    let a = vertices[index];
                    let b = vertices[(index + 1) % vertices.len()];
                    if (2 * a.y > py2) == (2 * b.y > py2) {
                        continue;
                    }
                    let cross = (b.x - a.x) * (py2 - 2 * a.y) - (b.y - a.y) * (px2 - 2 * a.x);
                    let right = if b.y > a.y { cross > 0 } else { cross < 0 };
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

    fn overlap_1d(a0: i64, a1: i64, b0: i64, b1: i64) -> bool {
        a0.min(a1) <= b0.max(b1) && b0.min(b1) <= a0.max(a1)
    }

    fn independent_boundary_contact(first: &[Point], second: &[Point]) -> bool {
        for i in 0..first.len() {
            let a = first[i];
            let b = first[(i + 1) % first.len()];
            for j in 0..second.len() {
                let c = second[j];
                let d = second[(j + 1) % second.len()];
                let first_horizontal = a.y == b.y;
                let second_horizontal = c.y == d.y;
                let contact = match (first_horizontal, second_horizontal) {
                    (true, true) => a.y == c.y && overlap_1d(a.x, b.x, c.x, d.x),
                    (false, false) => a.x == c.x && overlap_1d(a.y, b.y, c.y, d.y),
                    _ => overlap_1d(a.x, b.x, c.x, d.x) && overlap_1d(a.y, b.y, c.y, d.y),
                };
                if contact {
                    return true;
                }
            }
        }
        false
    }

    fn independent_conflict(first: &Polygon, second: &Polygon) -> bool {
        let a = first.vertices();
        let b = second.vertices();
        !independent_cells(a).is_disjoint(&independent_cells(b))
            || independent_boundary_contact(a, b)
    }

    fn feasible_result(result: &SolveOutcome) -> (&[ExactPlacement], usize) {
        match result {
            SolveOutcome::Feasible {
                placements,
                candidate_checks,
            } => (placements, *candidate_checks),
            other => panic!("expected feasible result, got {other:?}"),
        }
    }

    #[test]
    fn exact_mrv_finds_the_concave_two_block_layout() {
        let yard = Yard::new(5, 5).unwrap();
        let requests = vec![request("a", l_shape()), request("b", l_shape())];
        let result = solve_exact(yard, &requests, 100_000).unwrap();
        let (placements, checks) = feasible_result(&result);
        assert_eq!(placements.len(), 2);
        assert_eq!(placements[0].origin, p(0, 0));
        assert!(!independent_conflict(
            &placements[0].footprint,
            &placements[1].footprint
        ));
        assert!(checks > 0);
    }

    #[test]
    fn exact_mrv_recovers_layout_missed_by_same_order_first_fit() {
        let yard = Yard::new(4, 4).unwrap();
        let requests = vec![
            request("small-a", square(1)),
            request("small-b", square(1)),
            request("tall-c", vertical_bar()),
        ];

        let greedy = first_fit(yard, &requests, Policy::ExactPolygon).unwrap();
        assert_eq!(greedy.placements.len(), 2);
        assert_eq!(greedy.unplaced_ids, vec!["tall-c"]);
        assert_eq!(greedy.placements[0].origin, p(0, 0));
        assert_eq!(greedy.placements[1].origin, p(2, 0));

        let result = solve_exact(yard, &requests, 100_000).unwrap();
        let (placements, checks) = feasible_result(&result);
        assert_eq!(placements.len(), 3);
        assert!(checks > 0);
        assert_eq!(placements[0].block_id, "small-a");
        assert_eq!(placements[0].origin, p(2, 0));
        assert_eq!(placements[1].block_id, "small-b");
        assert_eq!(placements[1].origin, p(2, 2));
        assert_eq!(placements[2].block_id, "tall-c");
        assert_eq!(placements[2].origin, p(0, 0));
        for i in 0..placements.len() {
            for j in (i + 1)..placements.len() {
                assert!(
                    !independent_conflict(&placements[i].footprint, &placements[j].footprint),
                    "independent oracle rejected blocks {} and {}",
                    placements[i].block_id,
                    placements[j].block_id
                );
            }
        }
    }

    #[test]
    fn exact_search_proves_single_cell_domain_infeasible() {
        let yard = Yard::new(2, 2).unwrap();
        let requests = vec![request("a", square(2)), request("b", square(2))];
        let result = solve_exact(yard, &requests, 100).unwrap();
        assert!(matches!(result, SolveOutcome::ProvenInfeasible { .. }));
    }

    #[test]
    fn budget_exhaustion_is_not_mislabeled_as_infeasibility() {
        let yard = Yard::new(5, 5).unwrap();
        let requests = vec![request("a", l_shape()), request("b", l_shape())];
        assert_eq!(
            solve_exact(yard, &requests, 1).unwrap(),
            SolveOutcome::BudgetExceeded {
                candidate_checks: 1,
                budget: 1
            }
        );
    }

    #[test]
    fn bounded_model_rejects_out_of_scope_inputs() {
        let too_large = Yard::new(6, 5).unwrap();
        assert_eq!(
            solve_exact(too_large, &[], 100),
            Err(ExactError::YardOutsideTinyModel {
                width: 6,
                height: 5
            })
        );
        let yard = Yard::new(5, 5).unwrap();
        let requests = vec![
            request("a", l_shape()),
            request("b", l_shape()),
            request("c", l_shape()),
            request("d", l_shape()),
            request("e", l_shape()),
        ];
        assert_eq!(
            solve_exact(yard, &requests, 100),
            Err(ExactError::TooManyRequests {
                count: 5,
                maximum: 4
            })
        );
    }

    #[test]
    fn duplicate_ids_and_zero_budget_fail_closed() {
        let yard = Yard::new(5, 5).unwrap();
        let duplicate = vec![request("same", l_shape()), request("same", l_shape())];
        assert_eq!(
            solve_exact(yard, &duplicate, 100),
            Err(ExactError::DuplicateBlockId("same".to_owned()))
        );
        assert_eq!(solve_exact(yard, &[], 0), Err(ExactError::ZeroBudget));
    }
}
