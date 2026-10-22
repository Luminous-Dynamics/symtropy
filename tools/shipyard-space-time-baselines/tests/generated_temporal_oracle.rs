//! Deterministic generated temporal-staging stress family with an exact cell-set oracle.
//!
//! Unlike the static stress test, block occupancy intervals differ and an exclusion rectangle is
//! active only during a declared time interval. The exact oracle recursively assigns integer-cell
//! masks and checks time-overlapping occupancy independently of the production spatial verifier.

use shipyard_space_time::{ExclusionZone, Footprint, Rect, TimeRange, verify};
use shipyard_space_time_baselines::{
    BaselineErrorCode, PlacementRequest, first_fit_bottom_left, first_fit_decreasing_bottom_left,
    first_fit_heuristic_portfolio, first_fit_portfolio,
};

const YARD_WIDTH: i64 = 4;
const YARD_HEIGHT: i64 = 4;
const CASES: usize = 512;
const WINDOWS: [(u64, u64); 5] = [(0, 10), (0, 5), (5, 10), (2, 7), (7, 10)];

fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn pick(state: &mut u32, bound: u32) -> u32 {
    next_u32(state) % bound
}

fn cell_mask(rect: Rect) -> u32 {
    let mut mask = 0u32;
    for y in rect.min_y..rect.max_y {
        for x in rect.min_x..rect.max_x {
            let index = y * YARD_WIDTH + x;
            mask |= 1u32 << u32::try_from(index).expect("cell index is non-negative");
        }
    }
    mask
}

fn overlaps_in_time(left: TimeRange, right: TimeRange) -> bool {
    left.start_tick < right.end_tick && right.start_tick < left.end_tick
}

fn exact_pack_exists(
    requests: &[PlacementRequest],
    index: usize,
    occupied: &[(u32, TimeRange)],
    yard: Rect,
    zone_mask: u32,
    zone_active: TimeRange,
) -> bool {
    if index == requests.len() {
        return true;
    }

    let request = &requests[index];
    if request.width <= 0
        || request.height <= 0
        || request.width > YARD_WIDTH
        || request.height > YARD_HEIGHT
        || !request.occupied.is_valid()
    {
        return false;
    }

    for min_y in yard.min_y..=yard.max_y - request.height {
        for min_x in yard.min_x..=yard.max_x - request.width {
            let candidate_rect = Rect {
                min_x,
                min_y,
                max_x: min_x + request.width,
                max_y: min_y + request.height,
            };
            let candidate_mask = cell_mask(candidate_rect);

            if overlaps_in_time(request.occupied, zone_active) && candidate_mask & zone_mask != 0 {
                continue;
            }
            if occupied.iter().any(|(mask, interval)| {
                overlaps_in_time(request.occupied, *interval) && candidate_mask & mask != 0
            }) {
                continue;
            }

            let mut next_occupied = occupied.to_vec();
            next_occupied.push((candidate_mask, request.occupied));
            if exact_pack_exists(
                requests,
                index + 1,
                &next_occupied,
                yard,
                zone_mask,
                zone_active,
            ) {
                return true;
            }
        }
    }
    false
}

fn add_legacy_boundaries(
    xs: &mut Vec<i64>,
    ys: &mut Vec<i64>,
    rect: Rect,
    width: i64,
    height: i64,
) {
    xs.extend([
        rect.min_x.checked_sub(width),
        Some(rect.min_x),
        rect.max_x.checked_sub(width),
        Some(rect.max_x),
    ].into_iter().flatten());
    ys.extend([
        rect.min_y.checked_sub(height),
        Some(rect.min_y),
        rect.max_y.checked_sub(height),
        Some(rect.max_y),
    ].into_iter().flatten());
}

/// Reconstruct the pre-fix policy: all prior placements and zones contribute candidate
/// boundaries even when their time windows do not overlap the request. The cell masks used
/// for collision checks are intentionally separate from the production verifier.
fn legacy_time_blind_first_fit(
    yard: Rect,
    requests: &[PlacementRequest],
    zones: &[ExclusionZone],
) -> bool {
    let mut placed: Vec<(Rect, TimeRange)> = Vec::new();

    for request in requests {
        let mut xs = vec![yard.min_x];
        let mut ys = vec![yard.min_y];
        for (rect, _) in &placed {
            add_legacy_boundaries(&mut xs, &mut ys, *rect, request.width, request.height);
        }
        for zone in zones {
            match &zone.footprint {
                Footprint::AxisAlignedRect(rect) => {
                    add_legacy_boundaries(&mut xs, &mut ys, *rect, request.width, request.height);
                }
                Footprint::Unsupported { .. } => return false,
            }
        }

        xs.retain(|x| *x >= yard.min_x && *x < yard.max_x);
        ys.retain(|y| *y >= yard.min_y && *y < yard.max_y);
        xs.sort_unstable();
        xs.dedup();
        ys.sort_unstable();
        ys.dedup();

        let mut selected = None;
        'candidate: for y in &ys {
            for x in &xs {
                let Some(max_x) = x.checked_add(request.width) else {
                    continue;
                };
                let Some(max_y) = y.checked_add(request.height) else {
                    continue;
                };
                let rect = Rect {
                    min_x: *x,
                    min_y: *y,
                    max_x,
                    max_y,
                };
                if !yard.contains(rect) {
                    continue;
                }

                let mask = cell_mask(rect);
                if zones.iter().any(|zone| {
                    overlaps_in_time(request.occupied, zone.active)
                        && match &zone.footprint {
                            Footprint::AxisAlignedRect(zone_rect) => {
                                mask & cell_mask(*zone_rect) != 0
                            }
                            Footprint::Unsupported { .. } => true,
                        }
                }) {
                    continue;
                }
                if placed.iter().any(|(prior_rect, interval)| {
                    overlaps_in_time(request.occupied, *interval)
                        && mask & cell_mask(*prior_rect) != 0
                }) {
                    continue 'candidate;
                }

                selected = Some(rect);
                break 'candidate;
            }
        }

        let Some(rect) = selected else {
            return false;
        };
        placed.push((rect, request.occupied));
    }
    true
}

fn legacy_policy_orders(requests: &[PlacementRequest]) -> Vec<Vec<PlacementRequest>> {
    let area = |request: &PlacementRequest| {
        i128::from(request.width) * i128::from(request.height)
    };
    let max_side = |request: &PlacementRequest| request.width.max(request.height);
    let mut orders = vec![requests.to_vec()];

    let mut decreasing_area = requests.to_vec();
    decreasing_area.sort_by(|left, right| {
        area(right)
            .cmp(&area(left))
            .then_with(|| max_side(right).cmp(&max_side(left)))
            .then_with(|| right.width.cmp(&left.width))
            .then_with(|| right.height.cmp(&left.height))
            .then_with(|| left.reservation_id.cmp(&right.reservation_id))
    });
    orders.push(decreasing_area);

    let mut maximum_side = requests.to_vec();
    maximum_side.sort_by(|left, right| {
        max_side(right)
            .cmp(&max_side(left))
            .then_with(|| area(right).cmp(&area(left)))
            .then_with(|| right.width.cmp(&left.width))
            .then_with(|| right.height.cmp(&left.height))
            .then_with(|| left.reservation_id.cmp(&right.reservation_id))
    });
    orders.push(maximum_side);

    let mut height = requests.to_vec();
    height.sort_by(|left, right| {
        right
            .height
            .cmp(&left.height)
            .then_with(|| right.width.cmp(&left.width))
            .then_with(|| left.reservation_id.cmp(&right.reservation_id))
    });
    orders.push(height);

    let mut width = requests.to_vec();
    width.sort_by(|left, right| {
        right
            .width
            .cmp(&left.width)
            .then_with(|| right.height.cmp(&left.height))
            .then_with(|| left.reservation_id.cmp(&right.reservation_id))
    });
    orders.push(width);

    let mut perimeter = requests.to_vec();
    perimeter.sort_by(|left, right| {
        (i128::from(right.width) + i128::from(right.height))
            .cmp(&(i128::from(left.width) + i128::from(left.height)))
            .then_with(|| area(right).cmp(&area(left)))
            .then_with(|| left.reservation_id.cmp(&right.reservation_id))
    });
    orders.push(perimeter);
    orders
}

fn legacy_time_blind_portfolio_solves(
    yard: Rect,
    requests: &[PlacementRequest],
    zones: &[ExclusionZone],
) -> bool {
    legacy_policy_orders(requests)
        .iter()
        .any(|ordered| legacy_time_blind_first_fit(yard, ordered, zones))
}

fn generated_case(state: &mut u32) -> (Vec<PlacementRequest>, ExclusionZone) {
    let mut requests = Vec::new();
    for index in 0..4 {
        let width = 1 + i64::from(pick(state, 3));
        let height = 1 + i64::from(pick(state, 3));
        let (start_tick, end_tick) = WINDOWS[pick(state, 5) as usize];
        requests.push(PlacementRequest {
            reservation_id: format!("r-{index}"),
            block_id: format!("block-{index}"),
            width,
            height,
            occupied: TimeRange {
                start_tick,
                end_tick,
            },
        });
    }

    let zone_width = 1 + i64::from(pick(state, 2));
    let zone_height = 1 + i64::from(pick(state, 2));
    let zone_x = i64::from(pick(
        state,
        u32::try_from(YARD_WIDTH - zone_width + 1).unwrap(),
    ));
    let zone_y = i64::from(pick(
        state,
        u32::try_from(YARD_HEIGHT - zone_height + 1).unwrap(),
    ));
    let (zone_start, zone_end) = WINDOWS[pick(state, 3) as usize];

    (
        requests,
        ExclusionZone {
            id: "zone-1".to_owned(),
            footprint: Footprint::AxisAlignedRect(Rect {
                min_x: zone_x,
                min_y: zone_y,
                max_x: zone_x + zone_width,
                max_y: zone_y + zone_height,
            }),
            active: TimeRange {
                start_tick: zone_start,
                end_tick: zone_end,
            },
        },
    )
}

#[test]
fn generated_temporal_family_pins_exact_feasibility_and_portfolio_coverage() {
    let yard = Rect {
        min_x: 0,
        min_y: 0,
        max_x: YARD_WIDTH,
        max_y: YARD_HEIGHT,
    };
    let mut state = 0x51a7_2026u32;

    let mut exact_feasible = 0usize;
    let mut caller_solved = 0usize;
    let mut caller_missed = 0usize;
    let mut caller_rejected_infeasible = 0usize;
    let mut caller_false_feasible = 0usize;
    let mut decreasing_solved = 0usize;
    let mut decreasing_missed = 0usize;
    let mut decreasing_rejected_infeasible = 0usize;
    let mut decreasing_false_feasible = 0usize;
    let mut portfolio_solved = 0usize;
    let mut portfolio_missed = 0usize;
    let mut portfolio_rejected_infeasible = 0usize;
    let mut portfolio_false_feasible = 0usize;
    let mut legacy_time_blind_portfolio_solved = 0usize;
    let mut legacy_time_blind_false_feasible = 0usize;
    let mut exact_combined_solved = 0usize;
    let mut exact_fallback_recovered = 0usize;
    let mut exact_proven_infeasible = 0usize;
    let mut exact_budget_exhausted = 0usize;
    let mut exact_combined_false_feasible = 0usize;

    for _case in 0..CASES {
        let (requests, zone) = generated_case(&mut state);
        let zone_rect = match &zone.footprint {
            Footprint::AxisAlignedRect(rect) => *rect,
            Footprint::Unsupported { .. } => unreachable!("generator emits supported rectangles"),
        };
        let exact = exact_pack_exists(&requests, 0, &[], yard, cell_mask(zone_rect), zone.active);
        if exact {
            exact_feasible += 1;
        }

        if legacy_time_blind_portfolio_solves(yard, &requests, std::slice::from_ref(&zone)) {
            legacy_time_blind_portfolio_solved += 1;
            if !exact {
                legacy_time_blind_false_feasible += 1;
            }
        }

        match first_fit_bottom_left(yard, &requests, std::slice::from_ref(&zone)) {
            Ok(plan) => {
                caller_solved += 1;
                if !exact {
                    caller_false_feasible += 1;
                }
                assert!(verify(&plan).is_feasible());
            }
            Err(_) if exact => caller_missed += 1,
            Err(_) => caller_rejected_infeasible += 1,
        }

        match first_fit_decreasing_bottom_left(yard, &requests, std::slice::from_ref(&zone)) {
            Ok(plan) => {
                decreasing_solved += 1;
                if !exact {
                    decreasing_false_feasible += 1;
                }
                assert!(verify(&plan).is_feasible());
            }
            Err(_) if exact => decreasing_missed += 1,
            Err(_) => decreasing_rejected_infeasible += 1,
        }

        match first_fit_heuristic_portfolio(yard, &requests, std::slice::from_ref(&zone)) {
            Ok(result) => {
                portfolio_solved += 1;
                if !exact {
                    portfolio_false_feasible += 1;
                }
                assert_eq!(result.policies_attempted, 6);
                assert!(verify(&result.scenario).is_feasible());
            }
            Err(errors) => {
                assert_eq!(errors[0].code.as_str(), "AllPoliciesFailed");
                if exact {
                    portfolio_missed += 1;
                } else {
                    portfolio_rejected_infeasible += 1;
                }
            }
        }

        match first_fit_portfolio(yard, &requests, std::slice::from_ref(&zone)) {
            Ok(result) => {
                exact_combined_solved += 1;
                if !exact {
                    exact_combined_false_feasible += 1;
                }
                if result.exact_fallback_used {
                    exact_fallback_recovered += 1;
                    assert_eq!(result.selected_policy.as_str(), "exact-tiny-search");
                    assert!(result.exact_nodes_examined <= 100_000);
                } else {
                    assert!(result.feasible_candidates > 0);
                }
                assert!(verify(&result.scenario).is_feasible());
            }
            Err(errors) => match errors[0].code {
                BaselineErrorCode::ExactSearchProvenInfeasible => {
                    assert!(!exact, "exact search claimed a feasible case impossible");
                    exact_proven_infeasible += 1;
                }
                BaselineErrorCode::ExactSearchBudgetExceeded => {
                    exact_budget_exhausted += 1;
                }
                other => panic!("unexpected combined portfolio failure: {other:?}"),
            },
        }
    }

    assert_eq!(CASES, 512, "temporal/exclusion family size changed");
    assert_eq!(exact_feasible, 228, "exact temporal feasibility changed");
    assert_eq!(
        legacy_time_blind_portfolio_solved, 137,
        "legacy time-blind portfolio coverage changed"
    );
    assert_eq!(
        legacy_time_blind_false_feasible, 0,
        "legacy time-blind portfolio produced false-feasible plans"
    );
    assert_eq!(caller_solved, 168, "caller-order temporal coverage changed");
    assert_eq!(caller_missed, 60, "caller-order temporal misses changed");
    assert_eq!(
        caller_rejected_infeasible, 284,
        "caller-order infeasibility changed"
    );
    assert_eq!(
        caller_false_feasible, 0,
        "caller-order false-feasible plans detected"
    );
    assert_eq!(
        decreasing_solved, 192,
        "decreasing-area temporal coverage changed"
    );
    assert_eq!(
        decreasing_missed, 36,
        "decreasing-area temporal misses changed"
    );
    assert_eq!(
        decreasing_rejected_infeasible, 284,
        "decreasing-area infeasibility changed"
    );
    assert_eq!(
        decreasing_false_feasible, 0,
        "decreasing-area false-feasible plans detected"
    );
    assert_eq!(portfolio_solved, 216, "portfolio temporal coverage changed");
    assert_eq!(portfolio_missed, 12, "portfolio temporal misses changed");
    assert_eq!(
        portfolio_rejected_infeasible, 284,
        "portfolio temporal infeasibility changed"
    );
    assert_eq!(
        portfolio_false_feasible, 0,
        "portfolio false-feasible plans detected"
    );
    assert_eq!(
        exact_combined_solved, 228,
        "exact-repaired temporal coverage changed"
    );
    assert_eq!(
        exact_fallback_recovered, 12,
        "exact temporal recovery count changed"
    );
    assert_eq!(
        exact_proven_infeasible, 284,
        "exact temporal infeasibility count changed"
    );
    assert_eq!(
        exact_budget_exhausted, 0,
        "temporal exact-search budget changed"
    );
    assert_eq!(
        exact_combined_false_feasible, 0,
        "exact-repaired temporal false-feasible plans detected"
    );
}
