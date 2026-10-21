//! Deterministic generated stress family checked by an independent 25-bit occupancy oracle.
//!
//! Exact feasibility is computed from bitsets for individual integer lattice cells. It does not
//! call the production rectangle intersection helper or candidate-coordinate generator.

use shipyard_space_time::{Rect, TimeRange, verify};
use shipyard_space_time_baselines::{
    BaselineErrorCode, PlacementRequest, first_fit_bottom_left,
    first_fit_decreasing_bottom_left, first_fit_heuristic_portfolio, first_fit_portfolio,
};

const YARD_WIDTH: i64 = 5;
const YARD_HEIGHT: i64 = 5;
const CASES: usize = 512;

fn next_u32(state: &mut u32) -> u32 {
    *state ^= *state << 13;
    *state ^= *state >> 17;
    *state ^= *state << 5;
    *state
}

fn next_dimension(state: &mut u32) -> i64 {
    i64::from(1 + next_u32(state) % 4)
}

fn cell_mask(min_x: i64, min_y: i64, width: i64, height: i64) -> u32 {
    let mut mask = 0u32;
    for dy in 0..height {
        for dx in 0..width {
            let cell_index = (min_y + dy) * YARD_WIDTH + min_x + dx;
            mask |= 1u32 << u32::try_from(cell_index).expect("cell index is non-negative");
        }
    }
    mask
}

fn exact_pack_exists(requests: &[PlacementRequest], index: usize, occupied: u32) -> bool {
    if index == requests.len() {
        return true;
    }

    let request = &requests[index];
    if request.width <= 0
        || request.height <= 0
        || request.width > YARD_WIDTH
        || request.height > YARD_HEIGHT
    {
        return false;
    }

    for min_y in 0..=YARD_HEIGHT - request.height {
        for min_x in 0..=YARD_WIDTH - request.width {
            let candidate = cell_mask(min_x, min_y, request.width, request.height);
            if occupied & candidate != 0 {
                continue;
            }
            if exact_pack_exists(requests, index + 1, occupied | candidate) {
                return true;
            }
        }
    }

    false
}

#[test]
fn generated_four_block_family_pins_exact_feasibility_and_policy_coverage() {
    let yard = Rect {
        min_x: 0,
        min_y: 0,
        max_x: YARD_WIDTH,
        max_y: YARD_HEIGHT,
    };
    let occupancy = TimeRange {
        start_tick: 0,
        end_tick: 10,
    };
    let mut state = 0x5eed_c0deu32;

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
    let mut exact_combined_solved = 0usize;
    let mut exact_fallback_recovered = 0usize;
    let mut exact_proven_infeasible = 0usize;
    let mut exact_budget_exhausted = 0usize;
    let mut exact_combined_false_feasible = 0usize;

    for case_index in 0..CASES {
        let requests: Vec<PlacementRequest> = (0..4)
            .map(|index| PlacementRequest {
                reservation_id: format!("r-{index}"),
                block_id: format!("block-{index}"),
                width: next_dimension(&mut state),
                height: next_dimension(&mut state),
                occupied: occupancy,
            })
            .collect();

        let exact = exact_pack_exists(&requests, 0, 0);
        if exact {
            exact_feasible += 1;
        }

        match first_fit_bottom_left(yard, &requests, &[]) {
            Ok(plan) => {
                caller_solved += 1;
                if !exact {
                    caller_false_feasible += 1;
                }
                assert!(
                    verify(&plan).is_feasible(),
                    "independent spatial verifier rejected caller-order baseline: {plan:?}"
                );
            }
            Err(_) if exact => caller_missed += 1,
            Err(_) => caller_rejected_infeasible += 1,
        }

        match first_fit_decreasing_bottom_left(yard, &requests, &[]) {
            Ok(plan) => {
                decreasing_solved += 1;
                if !exact {
                    decreasing_false_feasible += 1;
                }
                assert!(
                    verify(&plan).is_feasible(),
                    "independent spatial verifier rejected decreasing-area baseline: {plan:?}"
                );
            }
            Err(_) if exact => decreasing_missed += 1,
            Err(_) => decreasing_rejected_infeasible += 1,
        }

        match first_fit_heuristic_portfolio(yard, &requests, &[]) {
            Ok(result) => {
                portfolio_solved += 1;
                if !exact {
                    portfolio_false_feasible += 1;
                }
                assert_eq!(result.policies_attempted, 6);
                assert!(result.feasible_candidates > 0);
                assert!(
                    verify(&result.scenario).is_feasible(),
                    "independent spatial verifier rejected portfolio output: {result:?}"
                );
            }
            Err(errors) => {
                if case_index == 48 || case_index == 244 {
                    assert!(exact, "pinned portfolio miss must remain exactly feasible");
                    assert_eq!(
                        errors[0].code.as_str(),
                        "AllPoliciesFailed",
                        "portfolio exhaustion must not be described as proven infeasibility"
                    );
                }
                if exact {
                    portfolio_missed += 1;
                } else {
                    portfolio_rejected_infeasible += 1;
                }
            },
        }

        match first_fit_portfolio(yard, &requests, &[]) {
            Ok(result) => {
                exact_combined_solved += 1;
                if !exact {
                    exact_combined_false_feasible += 1;
                }
                if result.exact_fallback_used {
                    exact_fallback_recovered += 1;
                    assert_eq!(
                        result.selected_policy.as_str(),
                        "exact-tiny-search"
                    );
                    assert!(result.exact_nodes_examined <= 100_000);
                } else {
                    assert!(result.feasible_candidates > 0);
                }
                assert!(
                    verify(&result.scenario).is_feasible(),
                    "independent verifier rejected combined portfolio output: {result:?}"
                );
            }
            Err(errors) => match errors[0].code {
                BaselineErrorCode::ExactSearchProvenInfeasible => {
                    assert!(!exact, "exact search claimed a feasible case impossible");
                    exact_proven_infeasible += 1;
                }
                BaselineErrorCode::ExactSearchBudgetExceeded => {
                    exact_budget_exhausted += 1;
                    assert!(exact, "budget exhaustion can be feasible or infeasible");
                }
                other => panic!("unexpected combined portfolio failure: {other:?}"),
            },
        }
    }

    assert_eq!(CASES, 512, "generated case family size changed");
    assert_eq!(exact_feasible, 205, "exact generated feasibility changed");
    assert_eq!(caller_solved, 179, "caller-order baseline coverage changed");
    assert_eq!(caller_missed, 26, "caller-order missed-feasible count changed");
    assert_eq!(caller_rejected_infeasible, 307, "caller-order infeasibility count changed");
    assert_eq!(caller_false_feasible, 0, "caller-order false-feasible plans detected");
    assert_eq!(decreasing_solved, 197, "decreasing-area baseline coverage changed");
    assert_eq!(decreasing_missed, 8, "decreasing-area missed-feasible count changed");
    assert_eq!(
        decreasing_rejected_infeasible, 307,
        "decreasing-area infeasibility count changed"
    );
    assert_eq!(
        decreasing_false_feasible, 0,
        "decreasing-area false-feasible plans detected"
    );
    assert_eq!(portfolio_solved, 203, "portfolio coverage changed");
    assert_eq!(portfolio_missed, 2, "portfolio missed-feasible count changed");
    assert_eq!(
        portfolio_rejected_infeasible, 307,
        "portfolio infeasibility rejection count changed"
    );
    assert_eq!(
        portfolio_false_feasible, 0,
        "portfolio false-feasible plans detected"
    );
    assert_eq!(exact_combined_solved, 205, "exact-repaired coverage changed");
    assert_eq!(exact_fallback_recovered, 2, "exact fallback recovery count changed");
    assert_eq!(exact_proven_infeasible, 307, "proven-infeasible count changed");
    assert_eq!(exact_budget_exhausted, 0, "exact search budget changed");
    assert_eq!(
        exact_combined_false_feasible, 0,
        "exact-repaired false-feasible plans detected"
    );
}
