//! Transparent first-fit/bottom-left baseline for bounded shipyard staging layouts.
//!
//! This module is a benchmark baseline, not an optimizer or operational planner. It preserves
//! the caller's request order, tests a deterministic set of boundary-aligned placements, and
//! sends the result through the separate shipyard-space-time verifier.
//!
//! V0 supports axis-aligned rectangular blocks, finite rectangular yard bounds, half-open
//! integer-tick occupancy, and rectangular exclusion zones. It does not model irregular shapes,
//! rotation, transfer routes, lifting, crane constraints, stacking, floor bearing, or safety.

use std::collections::BTreeSet;

use shipyard_space_time::{ExclusionZone, Footprint, Placement, Rect, Scenario, TimeRange, verify};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlacementRequest {
    pub reservation_id: String,
    pub block_id: String,
    /// Required width in integer lattice units. Must be positive.
    pub width: i64,
    /// Required height in integer lattice units. Must be positive.
    pub height: i64,
    pub occupied: TimeRange,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BaselineErrorCode {
    EmptyRequestSet,
    InvalidYardBounds,
    EmptyReservationId,
    DuplicateReservationId,
    EmptyBlockId,
    DuplicateBlockId,
    InvalidDimensions,
    InvalidTimeRange,
    EmptyExclusionZoneId,
    DuplicateExclusionZoneId,
    InvalidExclusionZoneTime,
    InvalidExclusionZoneFootprint,
    UnsupportedExclusionGeometry,
    OutOfBoundsExclusionZone,
    NoFeasiblePlacement,
    AllPoliciesFailed,
    ExactSearchProvenInfeasible,
    ExactSearchBudgetExceeded,
    VerifierRejected,
}

impl BaselineErrorCode {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::EmptyRequestSet => "EmptyRequestSet",
            Self::InvalidYardBounds => "InvalidYardBounds",
            Self::EmptyReservationId => "EmptyReservationId",
            Self::DuplicateReservationId => "DuplicateReservationId",
            Self::EmptyBlockId => "EmptyBlockId",
            Self::DuplicateBlockId => "DuplicateBlockId",
            Self::InvalidDimensions => "InvalidDimensions",
            Self::InvalidTimeRange => "InvalidTimeRange",
            Self::EmptyExclusionZoneId => "EmptyExclusionZoneId",
            Self::DuplicateExclusionZoneId => "DuplicateExclusionZoneId",
            Self::InvalidExclusionZoneTime => "InvalidExclusionZoneTime",
            Self::InvalidExclusionZoneFootprint => "InvalidExclusionZoneFootprint",
            Self::UnsupportedExclusionGeometry => "UnsupportedExclusionGeometry",
            Self::OutOfBoundsExclusionZone => "OutOfBoundsExclusionZone",
            Self::NoFeasiblePlacement => "NoFeasiblePlacement",
            Self::AllPoliciesFailed => "AllPoliciesFailed",
            Self::ExactSearchProvenInfeasible => "ExactSearchProvenInfeasible",
            Self::ExactSearchBudgetExceeded => "ExactSearchBudgetExceeded",
            Self::VerifierRejected => "VerifierRejected",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineError {
    pub code: BaselineErrorCode,
    pub subject_id: String,
    pub detail: String,
}

fn add_error(
    errors: &mut Vec<BaselineError>,
    code: BaselineErrorCode,
    subject_id: impl Into<String>,
    detail: impl Into<String>,
) {
    errors.push(BaselineError {
        code,
        subject_id: subject_id.into(),
        detail: detail.into(),
    });
}

fn is_nonempty(value: &str) -> bool {
    !value.trim().is_empty()
}

fn request_rect(request: &PlacementRequest, min_x: i64, min_y: i64) -> Option<Rect> {
    Some(Rect {
        min_x,
        min_y,
        max_x: min_x.checked_add(request.width)?,
        max_y: min_y.checked_add(request.height)?,
    })
}

/// Generate deterministic candidate lower-left coordinates.
///
/// Boundaries of already reserved rectangles and exclusions are included on both sides of
/// each obstacle. This is a transparent bottom-left-style candidate set, not an exhaustive
/// enumeration of every integer coordinate in the yard and not a proof of global optimality.
fn candidate_coordinates(
    yard: Rect,
    request: &PlacementRequest,
    existing: &[Placement],
    zones: &[ExclusionZone],
) -> (Vec<i64>, Vec<i64>) {
    let mut xs = vec![yard.min_x];
    let mut ys = vec![yard.min_y];

    for footprint in existing
        .iter()
        .filter(|placement| placement.occupied.overlaps(request.occupied))
        .map(|placement| &placement.footprint)
        .chain(
            zones
                .iter()
                .filter(|zone| zone.active.overlaps(request.occupied))
                .map(|zone| &zone.footprint),
        )
    {
        if let Footprint::AxisAlignedRect(rect) = footprint {
            xs.extend(
                [
                    rect.min_x.checked_sub(request.width),
                    Some(rect.min_x),
                    rect.max_x.checked_sub(request.width),
                    Some(rect.max_x),
                ]
                .into_iter()
                .flatten(),
            );
            ys.extend(
                [
                    rect.min_y.checked_sub(request.height),
                    Some(rect.min_y),
                    rect.max_y.checked_sub(request.height),
                    Some(rect.max_y),
                ]
                .into_iter()
                .flatten(),
            );
        }
    }

    xs.retain(|x| *x >= yard.min_x && *x < yard.max_x);
    ys.retain(|y| *y >= yard.min_y && *y < yard.max_y);
    xs.sort_unstable();
    xs.dedup();
    ys.sort_unstable();
    ys.dedup();
    (xs, ys)
}

fn validate_inputs(
    yard: Rect,
    requests: &[PlacementRequest],
    zones: &[ExclusionZone],
) -> Vec<BaselineError> {
    let mut errors = Vec::new();
    if !yard.is_valid() {
        add_error(
            &mut errors,
            BaselineErrorCode::InvalidYardBounds,
            "yard",
            "yard bounds must have positive width and height",
        );
    }
    if requests.is_empty() {
        add_error(
            &mut errors,
            BaselineErrorCode::EmptyRequestSet,
            "yard",
            "an empty request set is not a qualified placement plan",
        );
    }

    let mut reservation_ids = BTreeSet::new();
    let mut block_ids = BTreeSet::new();
    for request in requests {
        if !is_nonempty(&request.reservation_id) {
            add_error(
                &mut errors,
                BaselineErrorCode::EmptyReservationId,
                "",
                "reservation ID must not be empty",
            );
        } else if !reservation_ids.insert(request.reservation_id.as_str()) {
            add_error(
                &mut errors,
                BaselineErrorCode::DuplicateReservationId,
                &request.reservation_id,
                "reservation IDs must be unique",
            );
        }

        if !is_nonempty(&request.block_id) {
            add_error(
                &mut errors,
                BaselineErrorCode::EmptyBlockId,
                &request.reservation_id,
                "block ID must not be empty",
            );
        } else if !block_ids.insert(request.block_id.as_str()) {
            add_error(
                &mut errors,
                BaselineErrorCode::DuplicateBlockId,
                &request.block_id,
                "each V0 request must refer to a distinct block",
            );
        }

        if request.width <= 0 || request.height <= 0 {
            add_error(
                &mut errors,
                BaselineErrorCode::InvalidDimensions,
                &request.reservation_id,
                "width and height must be positive integer lattice units",
            );
        }
        if !request.occupied.is_valid() {
            add_error(
                &mut errors,
                BaselineErrorCode::InvalidTimeRange,
                &request.reservation_id,
                "occupancy interval must be a non-empty half-open time range",
            );
        }
    }

    let mut zone_ids = BTreeSet::new();
    for zone in zones {
        if !is_nonempty(&zone.id) {
            add_error(
                &mut errors,
                BaselineErrorCode::EmptyExclusionZoneId,
                "",
                "exclusion-zone ID must not be empty",
            );
        } else if !zone_ids.insert(zone.id.as_str()) {
            add_error(
                &mut errors,
                BaselineErrorCode::DuplicateExclusionZoneId,
                &zone.id,
                "exclusion-zone IDs must be unique",
            );
        }
        if !zone.active.is_valid() {
            add_error(
                &mut errors,
                BaselineErrorCode::InvalidExclusionZoneTime,
                &zone.id,
                "exclusion-zone active interval must be non-empty",
            );
        }
        match &zone.footprint {
            Footprint::AxisAlignedRect(rect) if !rect.is_valid() => add_error(
                &mut errors,
                BaselineErrorCode::InvalidExclusionZoneFootprint,
                &zone.id,
                "exclusion-zone rectangle is invalid",
            ),
            Footprint::AxisAlignedRect(rect) if yard.is_valid() && !yard.contains(*rect) => {
                add_error(
                    &mut errors,
                    BaselineErrorCode::OutOfBoundsExclusionZone,
                    &zone.id,
                    "exclusion-zone rectangle must fit inside the declared yard",
                )
            }
            Footprint::Unsupported { .. } => add_error(
                &mut errors,
                BaselineErrorCode::UnsupportedExclusionGeometry,
                &zone.id,
                "V0 cannot plan around unsupported exclusion geometry",
            ),
            Footprint::AxisAlignedRect(_) => {}
        }
    }

    errors
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum PortfolioPolicy {
    CallerOrder,
    DecreasingArea,
    DecreasingMaximumSide,
    DecreasingHeight,
    DecreasingWidth,
    DecreasingPerimeter,
    ExactTinySearch,
}

impl PortfolioPolicy {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::CallerOrder => "caller-order",
            Self::DecreasingArea => "decreasing-area",
            Self::DecreasingMaximumSide => "decreasing-maximum-side",
            Self::DecreasingHeight => "decreasing-height",
            Self::DecreasingWidth => "decreasing-width",
            Self::DecreasingPerimeter => "decreasing-perimeter",
            Self::ExactTinySearch => "exact-tiny-search",
        }
    }
}

#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct PortfolioScore {
    /// Sum over event intervals of (active-placement bounding-envelope area × duration).
    /// Smaller is more compact, but does not account for access routes, safety envelopes, or
    /// future unscheduled blocks; it is only a deterministic baseline tie-break objective.
    pub bounding_area_time: u128,
    /// Stable final tie-break: reservation ID, min_x, min_y.
    pub canonical_placements: Vec<(String, i64, i64)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PortfolioResult {
    pub scenario: Scenario,
    pub selected_policy: PortfolioPolicy,
    pub policies_attempted: usize,
    /// Number of heuristic policies that produced accepted candidates (zero for exact fallback).
    pub feasible_candidates: usize,
    pub exact_fallback_used: bool,
    pub exact_nodes_examined: usize,
    pub score: PortfolioScore,
}

fn order_requests(requests: &[PlacementRequest], policy: PortfolioPolicy) -> Vec<PlacementRequest> {
    let mut ordered = requests.to_vec();
    match policy {
        PortfolioPolicy::CallerOrder => {}
        PortfolioPolicy::DecreasingArea => ordered.sort_by(|left, right| {
            let left_area = i128::from(left.width) * i128::from(left.height);
            let right_area = i128::from(right.width) * i128::from(right.height);
            right_area
                .cmp(&left_area)
                .then_with(|| {
                    right
                        .width
                        .max(right.height)
                        .cmp(&left.width.max(left.height))
                })
                .then_with(|| right.width.cmp(&left.width))
                .then_with(|| right.height.cmp(&left.height))
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        }),
        PortfolioPolicy::DecreasingMaximumSide => ordered.sort_by(|left, right| {
            right
                .width
                .max(right.height)
                .cmp(&left.width.max(left.height))
                .then_with(|| {
                    (i128::from(right.width) * i128::from(right.height))
                        .cmp(&(i128::from(left.width) * i128::from(left.height)))
                })
                .then_with(|| right.width.cmp(&left.width))
                .then_with(|| right.height.cmp(&left.height))
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        }),
        PortfolioPolicy::DecreasingHeight => ordered.sort_by(|left, right| {
            right
                .height
                .cmp(&left.height)
                .then_with(|| right.width.cmp(&left.width))
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        }),
        PortfolioPolicy::DecreasingWidth => ordered.sort_by(|left, right| {
            right
                .width
                .cmp(&left.width)
                .then_with(|| right.height.cmp(&left.height))
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        }),
        PortfolioPolicy::DecreasingPerimeter => ordered.sort_by(|left, right| {
            (i128::from(right.width) + i128::from(right.height))
                .cmp(&(i128::from(left.width) + i128::from(left.height)))
                .then_with(|| {
                    (i128::from(right.width) * i128::from(right.height))
                        .cmp(&(i128::from(left.width) * i128::from(left.height)))
                })
                .then_with(|| left.reservation_id.cmp(&right.reservation_id))
        }),
        PortfolioPolicy::ExactTinySearch => {}
    }
    ordered
}

fn portfolio_score(scenario: &Scenario) -> PortfolioScore {
    let mut boundaries: Vec<u64> = scenario
        .placements
        .iter()
        .flat_map(|placement| [placement.occupied.start_tick, placement.occupied.end_tick])
        .collect();
    boundaries.sort_unstable();
    boundaries.dedup();

    let mut bounding_area_time = 0u128;
    for pair in boundaries.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        if start >= end {
            continue;
        }
        let mut bounds: Option<Rect> = None;
        for placement in &scenario.placements {
            if placement.occupied.start_tick > start || placement.occupied.end_tick < end {
                continue;
            }
            let Footprint::AxisAlignedRect(rect) = &placement.footprint else {
                continue;
            };
            bounds = Some(match bounds {
                None => *rect,
                Some(old) => Rect {
                    min_x: old.min_x.min(rect.min_x),
                    min_y: old.min_y.min(rect.min_y),
                    max_x: old.max_x.max(rect.max_x),
                    max_y: old.max_y.max(rect.max_y),
                },
            });
        }
        if let Some(rect) = bounds {
            let width = (i128::from(rect.max_x) - i128::from(rect.min_x)) as u128;
            let height = (i128::from(rect.max_y) - i128::from(rect.min_y)) as u128;
            let duration = u128::from(end - start);
            let extent = width.saturating_mul(height).saturating_mul(duration);
            bounding_area_time = bounding_area_time.saturating_add(extent);
        }
    }

    let mut canonical_placements: Vec<(String, i64, i64)> = scenario
        .placements
        .iter()
        .map(|placement| {
            let (x, y) = match &placement.footprint {
                Footprint::AxisAlignedRect(rect) => (rect.min_x, rect.min_y),
                Footprint::Unsupported { .. } => (i64::MAX, i64::MAX),
            };
            (placement.reservation_id.clone(), x, y)
        })
        .collect();
    canonical_placements.sort();

    PortfolioScore {
        bounding_area_time,
        canonical_placements,
    }
}

/// Evaluate a fixed portfolio of deterministic first-fit orderings and choose the most compact
/// independently verified result by lexicographic (bounding-envelope area-time, canonical
/// placement) score. Exhausting the portfolio is a heuristic failure, NOT proof that no feasible
/// packing exists. All candidate plans use the same independent verifier.
pub fn first_fit_heuristic_portfolio(
    yard_bounds: Rect,
    requests: &[PlacementRequest],
    exclusion_zones: &[ExclusionZone],
) -> Result<PortfolioResult, Vec<BaselineError>> {
    let input_errors = validate_inputs(yard_bounds, requests, exclusion_zones);
    if !input_errors.is_empty() {
        return Err(input_errors);
    }

    let policies = [
        PortfolioPolicy::CallerOrder,
        PortfolioPolicy::DecreasingArea,
        PortfolioPolicy::DecreasingMaximumSide,
        PortfolioPolicy::DecreasingHeight,
        PortfolioPolicy::DecreasingWidth,
        PortfolioPolicy::DecreasingPerimeter,
    ];
    let mut best: Option<(PortfolioPolicy, Scenario, PortfolioScore)> = None;
    let mut feasible_candidates = 0usize;

    for policy in policies {
        let ordered = order_requests(requests, policy);
        if let Ok(mut scenario) = first_fit_bottom_left(yard_bounds, &ordered, exclusion_zones) {
            feasible_candidates += 1;
            scenario
                .placements
                .sort_by(|left, right| left.reservation_id.cmp(&right.reservation_id));
            scenario
                .transfers
                .sort_by(|left, right| left.id.cmp(&right.id));
            scenario
                .exclusion_zones
                .sort_by(|left, right| left.id.cmp(&right.id));
            let score = portfolio_score(&scenario);
            if best
                .as_ref()
                .map_or(true, |(_, _, best_score)| score.cmp(best_score).is_lt())
            {
                best = Some((policy, scenario, score));
            }
        }
    }

    match best {
        Some((selected_policy, scenario, score)) => Ok(PortfolioResult {
            scenario,
            selected_policy,
            policies_attempted: policies.len(),
            feasible_candidates,
            exact_fallback_used: false,
            exact_nodes_examined: 0,
            score,
        }),
        None => Err(vec![BaselineError {
            code: BaselineErrorCode::AllPoliciesFailed,
            subject_id: "yard".to_owned(),
            detail:
                "no heuristic policy produced a verified plan; this is not proof of infeasibility"
                    .to_owned(),
        }]),
    }
}

/// Bounded exact-search domain used only after the heuristic portfolio misses.
const EXACT_TINY_MAX_YARD_DIMENSION: i128 = 5;
const EXACT_TINY_MAX_REQUESTS: usize = 4;
const EXACT_TINY_NODE_BUDGET: usize = 100_000;

enum ExactSearchProgress {
    Found,
    ProvenInfeasible,
    BudgetExceeded,
}

enum ExactSearchOutcome {
    Found(Scenario, usize),
    ProvenInfeasible(usize),
    BudgetExceeded(usize),
}

fn exact_tiny_search_recursive(
    yard: Rect,
    requests: &[PlacementRequest],
    index: usize,
    placements: &mut Vec<Placement>,
    zones: &[ExclusionZone],
    nodes_examined: &mut usize,
    node_budget: usize,
) -> ExactSearchProgress {
    if index == requests.len() {
        return ExactSearchProgress::Found;
    }

    let request = &requests[index];
    let yard_width = i128::from(yard.max_x) - i128::from(yard.min_x);
    let yard_height = i128::from(yard.max_y) - i128::from(yard.min_y);
    if request.width <= 0
        || request.height <= 0
        || i128::from(request.width) > yard_width
        || i128::from(request.height) > yard_height
        || !request.occupied.is_valid()
    {
        return ExactSearchProgress::ProvenInfeasible;
    }

    for min_y in yard.min_y..=yard.max_y - request.height {
        for min_x in yard.min_x..=yard.max_x - request.width {
            if *nodes_examined >= node_budget {
                return ExactSearchProgress::BudgetExceeded;
            }
            *nodes_examined += 1;

            let candidate_rect = Rect {
                min_x,
                min_y,
                max_x: min_x + request.width,
                max_y: min_y + request.height,
            };
            let zone_conflict = zones.iter().any(|zone| {
                request.occupied.overlaps(zone.active)
                    && matches!(
                        &zone.footprint,
                        Footprint::AxisAlignedRect(zone_rect)
                            if candidate_rect.overlaps(*zone_rect)
                    )
            });
            if zone_conflict {
                continue;
            }

            let placement_conflict = placements.iter().any(|prior| {
                request.occupied.overlaps(prior.occupied)
                    && matches!(
                        &prior.footprint,
                        Footprint::AxisAlignedRect(prior_rect)
                            if candidate_rect.overlaps(*prior_rect)
                    )
            });
            if placement_conflict {
                continue;
            }

            placements.push(Placement {
                reservation_id: request.reservation_id.clone(),
                block_id: request.block_id.clone(),
                footprint: Footprint::AxisAlignedRect(candidate_rect),
                occupied: request.occupied,
            });

            match exact_tiny_search_recursive(
                yard,
                requests,
                index + 1,
                placements,
                zones,
                nodes_examined,
                node_budget,
            ) {
                ExactSearchProgress::Found => return ExactSearchProgress::Found,
                ExactSearchProgress::BudgetExceeded => return ExactSearchProgress::BudgetExceeded,
                ExactSearchProgress::ProvenInfeasible => {
                    placements.pop();
                }
            }
        }
    }
    ExactSearchProgress::ProvenInfeasible
}

fn exact_tiny_search_with_budget(
    yard_bounds: Rect,
    requests: &[PlacementRequest],
    exclusion_zones: &[ExclusionZone],
    node_budget: usize,
) -> Option<ExactSearchOutcome> {
    let yard_width = i128::from(yard_bounds.max_x) - i128::from(yard_bounds.min_x);
    let yard_height = i128::from(yard_bounds.max_y) - i128::from(yard_bounds.min_y);
    if requests.len() > EXACT_TINY_MAX_REQUESTS
        || yard_width <= 0
        || yard_height <= 0
        || yard_width > EXACT_TINY_MAX_YARD_DIMENSION
        || yard_height > EXACT_TINY_MAX_YARD_DIMENSION
    {
        return None;
    }

    let ordered = order_requests(requests, PortfolioPolicy::DecreasingArea);
    let mut placements = Vec::with_capacity(ordered.len());
    let mut nodes_examined = 0usize;
    match exact_tiny_search_recursive(
        yard_bounds,
        &ordered,
        0,
        &mut placements,
        exclusion_zones,
        &mut nodes_examined,
        node_budget,
    ) {
        ExactSearchProgress::Found => {
            let mut scenario = Scenario {
                yard_bounds: Some(yard_bounds),
                placements,
                transfers: Vec::new(),
                exclusion_zones: exclusion_zones.to_vec(),
            };
            scenario
                .placements
                .sort_by(|left, right| left.reservation_id.cmp(&right.reservation_id));
            scenario
                .exclusion_zones
                .sort_by(|left, right| left.id.cmp(&right.id));
            Some(ExactSearchOutcome::Found(scenario, nodes_examined))
        }
        ExactSearchProgress::ProvenInfeasible => {
            Some(ExactSearchOutcome::ProvenInfeasible(nodes_examined))
        }
        ExactSearchProgress::BudgetExceeded => {
            Some(ExactSearchOutcome::BudgetExceeded(nodes_examined))
        }
    }
}

/// Use the six-policy heuristic portfolio first, then a bounded complete search for small integer
/// rectangle instances. Exact fallback is eligible only for at most four requests in a yard no
/// larger than 5×5 lattice units, with a 100,000-candidate node budget. Its outcomes distinguish
/// a verified solution, a complete proof of infeasibility, and an exhausted budget. A larger
/// model that defeats all heuristics returns AllPoliciesFailed, never a false infeasibility claim.
pub fn first_fit_portfolio(
    yard_bounds: Rect,
    requests: &[PlacementRequest],
    exclusion_zones: &[ExclusionZone],
) -> Result<PortfolioResult, Vec<BaselineError>> {
    first_fit_portfolio_with_budget(
        yard_bounds,
        requests,
        exclusion_zones,
        EXACT_TINY_NODE_BUDGET,
    )
}

fn first_fit_portfolio_with_budget(
    yard_bounds: Rect,
    requests: &[PlacementRequest],
    exclusion_zones: &[ExclusionZone],
    exact_node_budget: usize,
) -> Result<PortfolioResult, Vec<BaselineError>> {
    match first_fit_heuristic_portfolio(yard_bounds, requests, exclusion_zones) {
        Ok(result) => Ok(result),
        Err(errors)
            if errors
                .first()
                .is_some_and(|error| error.code == BaselineErrorCode::AllPoliciesFailed) =>
        {
            match exact_tiny_search_with_budget(
                yard_bounds,
                requests,
                exclusion_zones,
                exact_node_budget,
            ) {
                Some(ExactSearchOutcome::Found(scenario, nodes_examined)) => {
                    let report = verify(&scenario);
                    if !report.is_feasible() {
                        return Err(report
                            .violations
                            .into_iter()
                            .map(|violation| BaselineError {
                                code: BaselineErrorCode::VerifierRejected,
                                subject_id: violation.subject_ids.join(";"),
                                detail: format!(
                                    "independent verifier rejected exact fallback result: {}",
                                    violation.code.as_str()
                                ),
                            })
                            .collect());
                    }
                    let score = portfolio_score(&scenario);
                    Ok(PortfolioResult {
                        scenario,
                        selected_policy: PortfolioPolicy::ExactTinySearch,
                        policies_attempted: 6,
                        feasible_candidates: 0,
                        exact_fallback_used: true,
                        exact_nodes_examined: nodes_examined,
                        score,
                    })
                }
                Some(ExactSearchOutcome::ProvenInfeasible(nodes_examined)) => {
                    Err(vec![BaselineError {
                        code: BaselineErrorCode::ExactSearchProvenInfeasible,
                        subject_id: "yard".to_owned(),
                        detail: format!(
                            "exact search proved no layout exists for this bounded model after {nodes_examined} candidate tests"
                        ),
                    }])
                }
                Some(ExactSearchOutcome::BudgetExceeded(nodes_examined)) => {
                    Err(vec![BaselineError {
                        code: BaselineErrorCode::ExactSearchBudgetExceeded,
                        subject_id: "yard".to_owned(),
                        detail: format!(
                            "exact fallback budget exhausted after {nodes_examined} candidates; result remains unknown"
                        ),
                    }])
                }
                None => Err(errors),
            }
        }
        Err(errors) => Err(errors),
    }
}

/// Place requests in deterministic decreasing-area order before using the same candidate policy.
///
/// Tie-break order is descending maximum side, width, height, then ascending reservation ID.
/// Area is computed in i128 to avoid overflow for positive i64 dimensions.
///
/// This remains a heuristic policy, not a globally optimal rectangle-packing algorithm.
pub fn first_fit_decreasing_bottom_left(
    yard_bounds: Rect,
    requests: &[PlacementRequest],
    exclusion_zones: &[ExclusionZone],
) -> Result<Scenario, Vec<BaselineError>> {
    let mut ordered = requests.to_vec();
    ordered.sort_by(|left, right| {
        let left_area = i128::from(left.width) * i128::from(left.height);
        let right_area = i128::from(right.width) * i128::from(right.height);
        let left_max_side = left.width.max(left.height);
        let right_max_side = right.width.max(right.height);

        right_area
            .cmp(&left_area)
            .then_with(|| right_max_side.cmp(&left_max_side))
            .then_with(|| right.width.cmp(&left.width))
            .then_with(|| right.height.cmp(&left.height))
            .then_with(|| left.reservation_id.cmp(&right.reservation_id))
    });

    first_fit_bottom_left(yard_bounds, &ordered, exclusion_zones)
}

/// Place rectangular requests using a deterministic first-fit/bottom-left candidate order.
///
/// Requests are processed in caller-provided order. Candidate positions are tried in ascending
/// (y, x) order; reservations may share floor space only when their half-open occupancy
/// intervals do not overlap. Exclusion zones consume space only while their active interval
/// overlaps the request. Any invalid input, exhausted candidate set, or verifier disagreement
/// fails the entire call without returning a partial plan.
///
/// The returned scenario contains only explicit geometry/time reservations. It is not an
/// optimized schedule, transfer route, crane/lift instruction or production authorization.
pub fn first_fit_bottom_left(
    yard_bounds: Rect,
    requests: &[PlacementRequest],
    exclusion_zones: &[ExclusionZone],
) -> Result<Scenario, Vec<BaselineError>> {
    let input_errors = validate_inputs(yard_bounds, requests, exclusion_zones);
    if !input_errors.is_empty() {
        return Err(input_errors);
    }

    let mut placements: Vec<Placement> = Vec::with_capacity(requests.len());
    for request in requests {
        let (xs, ys) = candidate_coordinates(yard_bounds, request, &placements, exclusion_zones);
        let mut selected = None;

        'candidate: for y in &ys {
            for x in &xs {
                let Some(rect) = request_rect(request, *x, *y) else {
                    continue;
                };
                if !yard_bounds.contains(rect) {
                    continue;
                }

                for prior in &placements {
                    if prior.occupied.overlaps(request.occupied)
                        && let Footprint::AxisAlignedRect(prior_rect) = &prior.footprint
                        && rect.overlaps(*prior_rect)
                    {
                        continue;
                    }
                }

                for zone in exclusion_zones {
                    if zone.active.overlaps(request.occupied)
                        && let Footprint::AxisAlignedRect(zone_rect) = &zone.footprint
                        && rect.overlaps(*zone_rect)
                    {
                        continue;
                    }
                }

                selected = Some(rect);
                break 'candidate;
            }
        }

        let Some(rect) = selected else {
            return Err(vec![BaselineError {
                code: BaselineErrorCode::NoFeasiblePlacement,
                subject_id: request.reservation_id.clone(),
                detail: "no candidate rectangle fits the yard and active reservations".to_owned(),
            }]);
        };

        placements.push(Placement {
            reservation_id: request.reservation_id.clone(),
            block_id: request.block_id.clone(),
            footprint: Footprint::AxisAlignedRect(rect),
            occupied: request.occupied,
        });
    }

    let scenario = Scenario {
        yard_bounds: Some(yard_bounds),
        placements,
        transfers: Vec::new(),
        exclusion_zones: exclusion_zones.to_vec(),
    };

    let report = verify(&scenario);
    if !report.is_feasible() {
        let errors = report
            .violations
            .into_iter()
            .map(|violation| BaselineError {
                code: BaselineErrorCode::VerifierRejected,
                subject_id: violation.subject_ids.join(";"),
                detail: format!("independent verifier rejected {}", violation.code.as_str()),
            })
            .collect();
        return Err(errors);
    }

    Ok(scenario)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet as CellSet;

    fn yard() -> Rect {
        Rect {
            min_x: 0,
            min_y: 0,
            max_x: 10,
            max_y: 6,
        }
    }

    fn request(
        reservation_id: &str,
        block_id: &str,
        width: i64,
        height: i64,
        start_tick: u64,
        end_tick: u64,
    ) -> PlacementRequest {
        PlacementRequest {
            reservation_id: reservation_id.to_owned(),
            block_id: block_id.to_owned(),
            width,
            height,
            occupied: TimeRange {
                start_tick,
                end_tick,
            },
        }
    }

    fn rect_of(placement: &Placement) -> Rect {
        match &placement.footprint {
            Footprint::AxisAlignedRect(rect) => *rect,
            Footprint::Unsupported { .. } => panic!("baseline must emit supported rectangles"),
        }
    }

    #[test]
    fn concurrent_rectangles_are_placed_and_independently_verifiable() {
        let requests = [
            request("r-a", "block-a", 6, 4, 0, 10),
            request("r-b", "block-b", 4, 4, 0, 10),
            request("r-c", "block-c", 4, 2, 0, 10),
        ];
        let scenario = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        assert_eq!(scenario.placements.len(), 3);
        assert!(verify(&scenario).is_feasible());

        // A small independent cell oracle checks that simultaneous placements share no cell.
        for (index, left) in scenario.placements.iter().enumerate() {
            let left_rect = rect_of(left);
            let left_cells: CellSet<(i64, i64)> = (left_rect.min_x..left_rect.max_x)
                .flat_map(|x| (left_rect.min_y..left_rect.max_y).map(move |y| (x, y)))
                .collect();
            for right in scenario.placements.iter().skip(index + 1) {
                if !left.occupied.overlaps(right.occupied) {
                    continue;
                }
                let right_rect = rect_of(right);
                let right_cells: CellSet<(i64, i64)> = (right_rect.min_x..right_rect.max_x)
                    .flat_map(|x| (right_rect.min_y..right_rect.max_y).map(move |y| (x, y)))
                    .collect();
                assert!(left_cells.is_disjoint(&right_cells));
            }
        }
    }

    #[test]
    fn inactive_temporal_objects_do_not_pollute_candidate_coordinates() {
        let request = request("new", "new-block", 2, 2, 5, 10);
        let previous = [Placement {
            reservation_id: "old".to_owned(),
            block_id: "old-block".to_owned(),
            footprint: Footprint::AxisAlignedRect(Rect {
                min_x: 6,
                min_y: 6,
                max_x: 8,
                max_y: 8,
            }),
            occupied: TimeRange {
                start_tick: 0,
                end_tick: 5,
            },
        }];
        let zones = [ExclusionZone {
            id: "inactive-zone".to_owned(),
            footprint: Footprint::AxisAlignedRect(Rect {
                min_x: 4,
                min_y: 4,
                max_x: 5,
                max_y: 5,
            }),
            active: TimeRange {
                start_tick: 0,
                end_tick: 5,
            },
        }];

        let (xs, ys) = candidate_coordinates(yard(), &request, &previous, &zones);

        assert_eq!(xs, vec![yard().min_x]);
        assert_eq!(ys, vec![yard().min_y]);
    }

    #[test]
    fn non_overlapping_time_windows_can_reuse_the_same_space() {
        let requests = [
            request("r-a", "block-a", 5, 4, 0, 4),
            request("r-b", "block-b", 5, 4, 4, 8),
        ];
        let scenario = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        assert_eq!(
            rect_of(&scenario.placements[0]),
            rect_of(&scenario.placements[1])
        );
        assert!(verify(&scenario).is_feasible());
    }

    #[test]
    fn active_exclusion_zone_pushes_first_fit_to_the_next_candidate() {
        let zones = [ExclusionZone {
            id: "zone-a".to_owned(),
            footprint: Footprint::AxisAlignedRect(Rect {
                min_x: 0,
                min_y: 0,
                max_x: 5,
                max_y: 6,
            }),
            active: TimeRange {
                start_tick: 0,
                end_tick: 10,
            },
        }];
        let scenario =
            first_fit_bottom_left(yard(), &[request("r-a", "block-a", 3, 3, 0, 5)], &zones)
                .unwrap();
        assert!(matches!(
            &scenario.placements[0].footprint,
            Footprint::AxisAlignedRect(Rect {
                min_x: 5,
                min_y: 0,
                ..
            })
        ));
        assert!(verify(&scenario).is_feasible());
    }

    #[test]
    fn full_yard_fails_instead_of_returning_a_partial_plan() {
        let requests = [
            request("r-a", "block-a", 10, 6, 0, 10),
            request("r-b", "block-b", 1, 1, 0, 10),
        ];
        let errors = first_fit_bottom_left(yard(), &requests, &[]).unwrap_err();
        assert_eq!(errors[0].code, BaselineErrorCode::NoFeasiblePlacement);
        assert_eq!(errors[0].subject_id, "r-b");
    }

    #[test]
    fn identical_inputs_produce_identical_layouts_and_errors() {
        let requests = [
            request("r-a", "block-a", 4, 3, 0, 8),
            request("r-b", "block-b", 5, 3, 0, 8),
        ];
        let left = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        let right = first_fit_bottom_left(yard(), &requests, &[]).unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn portfolio_bounding_area_time_score_matches_independent_tick_sweep() {
        let requests = [
            request("r-a", "block-a", 4, 3, 0, 4),
            request("r-b", "block-b", 3, 2, 2, 7),
            request("r-c", "block-c", 2, 2, 4, 8),
        ];
        let result = first_fit_portfolio(yard(), &requests, &[]).unwrap();

        let mut expected = 0u128;
        for tick in 0..8 {
            let mut active_rects = Vec::new();
            for placement in &result.scenario.placements {
                if placement.occupied.start_tick <= tick && tick < placement.occupied.end_tick {
                    if let Footprint::AxisAlignedRect(rect) = &placement.footprint {
                        active_rects.push(*rect);
                    }
                }
            }
            if active_rects.is_empty() {
                continue;
            }
            let min_x = active_rects.iter().map(|rect| rect.min_x).min().unwrap();
            let min_y = active_rects.iter().map(|rect| rect.min_y).min().unwrap();
            let max_x = active_rects.iter().map(|rect| rect.max_x).max().unwrap();
            let max_y = active_rects.iter().map(|rect| rect.max_y).max().unwrap();
            expected += u128::try_from((max_x - min_x) * (max_y - min_y)).unwrap();
        }

        assert_eq!(result.score.bounding_area_time, expected);
    }

    #[test]
    fn portfolio_is_input_order_invariant_and_returns_independently_verified_plan() {
        let requests = [
            request("r-a", "block-a", 1, 1, 0, 8),
            request("r-b", "block-b", 2, 2, 0, 8),
            request("r-c", "block-c", 3, 2, 0, 8),
            request("r-d", "block-d", 1, 3, 0, 8),
        ];
        let mut reversed = requests.clone();
        reversed.reverse();

        let forward = first_fit_portfolio(yard(), &requests, &[]).unwrap();
        let reverse = first_fit_portfolio(yard(), &reversed, &[]).unwrap();
        assert_eq!(forward.scenario, reverse.scenario);
        assert_eq!(forward.policies_attempted, 6);
        assert!(forward.feasible_candidates > 0);
        assert!(verify(&forward.scenario).is_feasible());
        assert!(forward.score.bounding_area_time > 0);
    }

    #[test]
    fn decreasing_area_policy_is_invariant_to_input_permutation() {
        let requests = [
            request("r-a", "block-a", 1, 1, 0, 8),
            request("r-b", "block-b", 2, 2, 0, 8),
            request("r-c", "block-c", 3, 2, 0, 8),
            request("r-d", "block-d", 1, 3, 0, 8),
        ];
        let mut reversed = requests.clone();
        reversed.reverse();

        let forward = first_fit_decreasing_bottom_left(yard(), &requests, &[]).unwrap();
        let reverse = first_fit_decreasing_bottom_left(yard(), &reversed, &[]).unwrap();
        assert_eq!(forward, reverse);
        assert!(verify(&forward).is_feasible());
    }

    #[test]
    fn exact_fallback_distinguishes_budget_exhaustion_from_infeasibility() {
        let requests = [
            request("r-a", "block-a", 3, 3, 0, 10),
            request("r-b", "block-b", 3, 3, 0, 10),
        ];
        let small_yard = Rect {
            min_x: 0,
            min_y: 0,
            max_x: 5,
            max_y: 5,
        };
        let errors = first_fit_portfolio_with_budget(small_yard, &requests, &[], 1).unwrap_err();
        assert_eq!(errors[0].code, BaselineErrorCode::ExactSearchBudgetExceeded);
    }

    #[test]
    fn exact_fallback_proves_tiny_impossibility_only_after_complete_search() {
        let small_yard = Rect {
            min_x: 0,
            min_y: 0,
            max_x: 5,
            max_y: 5,
        };
        let requests = [
            request("r-a", "block-a", 3, 3, 0, 10),
            request("r-b", "block-b", 3, 3, 0, 10),
        ];
        let errors = first_fit_portfolio(small_yard, &requests, &[]).unwrap_err();
        assert_eq!(
            errors[0].code,
            BaselineErrorCode::ExactSearchProvenInfeasible
        );
    }

    #[test]
    fn exhausted_heuristics_outside_exact_domain_do_not_claim_infeasibility() {
        let larger_yard = Rect {
            min_x: 0,
            min_y: 0,
            max_x: 6,
            max_y: 6,
        };
        let requests = [
            request("r-a", "block-a", 4, 4, 0, 10),
            request("r-b", "block-b", 4, 4, 0, 10),
            request("r-c", "block-c", 4, 4, 0, 10),
            request("r-d", "block-d", 4, 4, 0, 10),
        ];
        let errors = first_fit_portfolio(larger_yard, &requests, &[]).unwrap_err();
        assert_eq!(errors[0].code, BaselineErrorCode::AllPoliciesFailed);
    }

    #[test]
    fn empty_requests_and_unsupported_exclusions_fail_closed() {
        assert_eq!(
            first_fit_bottom_left(yard(), &[], &[]).unwrap_err()[0].code,
            BaselineErrorCode::EmptyRequestSet
        );
        let zones = [ExclusionZone {
            id: "unknown-zone".to_owned(),
            footprint: Footprint::Unsupported {
                profile: "polygon-v1".to_owned(),
            },
            active: TimeRange {
                start_tick: 0,
                end_tick: 10,
            },
        }];
        assert_eq!(
            first_fit_bottom_left(yard(), &[request("r-a", "block-a", 1, 1, 0, 2)], &zones)
                .unwrap_err()[0]
                .code,
            BaselineErrorCode::UnsupportedExclusionGeometry
        );
    }
}
