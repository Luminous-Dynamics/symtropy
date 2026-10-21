//! Independent exhaustive oracle for very small rectangular placement instances.
//!
//! The oracle uses rasterized integer-cell sets and recursive enumeration of every legal lower
//! left corner. It does not import production overlap helpers or reuse the baseline's candidate
//! coordinate generator. Its purpose is to detect false-feasible baseline outputs and expose
//! missed feasible instances before moving to larger benchmark corpora.

use std::collections::BTreeSet;

use shipyard_space_time::{Rect, TimeRange, verify};
use shipyard_space_time_baselines::{PlacementRequest, first_fit_bottom_left};

fn cells(rect: Rect) -> BTreeSet<(i64, i64)> {
    let mut result = BTreeSet::new();
    for x in rect.min_x..rect.max_x {
        for y in rect.min_y..rect.max_y {
            result.insert((x, y));
        }
    }
    result
}

fn exact_pack_exists(yard: Rect, requests: &[PlacementRequest]) -> bool {
    fn visit(
        yard: Rect,
        requests: &[PlacementRequest],
        index: usize,
        occupied: &mut Vec<(BTreeSet<(i64, i64)>, TimeRange)>,
    ) -> bool {
        if index == requests.len() {
            return true;
        }

        let request = &requests[index];
        if request.width <= 0
            || request.height <= 0
            || !request.occupied.is_valid()
            || !yard.is_valid()
            || request.width > yard.max_x - yard.min_x
            || request.height > yard.max_y - yard.min_y
        {
            return false;
        }

        let last_x = yard.max_x - request.width;
        let last_y = yard.max_y - request.height;
        for min_y in yard.min_y..=last_y {
            for min_x in yard.min_x..=last_x {
                let rect = Rect {
                    min_x,
                    min_y,
                    max_x: min_x + request.width,
                    max_y: min_y + request.height,
                };
                let candidate_cells = cells(rect);
                let conflicts = occupied.iter().any(|(prior_cells, prior_time)| {
                    request.occupied.overlaps(*prior_time)
                        && !candidate_cells.is_disjoint(prior_cells)
                });
                if conflicts {
                    continue;
                }

                occupied.push((candidate_cells, request.occupied));
                if visit(yard, requests, index + 1, occupied) {
                    return true;
                }
                occupied.pop();
            }
        }
        false
    }

    visit(yard, requests, 0, &mut Vec::new())
}

#[test]
fn exhaustive_small_instances_never_allow_a_false_feasible_first_fit_plan() {
    let yard = Rect {
        min_x: 0,
        min_y: 0,
        max_x: 4,
        max_y: 3,
    };
    let dimensions = [(1, 1), (1, 2), (2, 1), (2, 2), (3, 1), (3, 2)];
    let occupancy = TimeRange {
        start_tick: 0,
        end_tick: 10,
    };

    let mut examined = 0usize;
    let mut exact_feasible = 0usize;
    let mut baseline_solved = 0usize;
    let mut baseline_missed_feasible = 0usize;
    let mut baseline_rejected_infeasible = 0usize;

    for (a_width, a_height) in dimensions {
        for (b_width, b_height) in dimensions {
            for (c_width, c_height) in dimensions {
                let requests = [
                    PlacementRequest {
                        reservation_id: "r-a".to_owned(),
                        block_id: "block-a".to_owned(),
                        width: a_width,
                        height: a_height,
                        occupied: occupancy,
                    },
                    PlacementRequest {
                        reservation_id: "r-b".to_owned(),
                        block_id: "block-b".to_owned(),
                        width: b_width,
                        height: b_height,
                        occupied: occupancy,
                    },
                    PlacementRequest {
                        reservation_id: "r-c".to_owned(),
                        block_id: "block-c".to_owned(),
                        width: c_width,
                        height: c_height,
                        occupied: occupancy,
                    },
                ];
                let exact = exact_pack_exists(yard, &requests);
                examined += 1;
                exact_feasible += usize::from(exact);

                match first_fit_bottom_left(yard, &requests, &[]) {
                    Ok(plan) => {
                        baseline_solved += 1;
                        assert!(
                            exact,
                            "baseline produced a feasible plan for an exact-infeasible case: {:?}",
                            requests
                        );
                        assert!(
                            verify(&plan).is_feasible(),
                            "independent verifier rejected baseline output: {plan:?}"
                        );
                    }
                    Err(_) if exact => baseline_missed_feasible += 1,
                    Err(_) => baseline_rejected_infeasible += 1,
                }
            }
        }
    }

    assert_eq!(examined, 216, "frozen tiny-instance family changed");
    assert_eq!(exact_feasible, 154, "exact feasibility reference changed");
    assert_eq!(baseline_solved, 150, "first-fit reference coverage changed");
    assert_eq!(baseline_missed_feasible, 4, "first-fit miss count changed");
    assert_eq!(baseline_rejected_infeasible, 62, "infeasible rejection count changed");

    // One frozen counterexample proves first-fit is a baseline rather than an exact solver.
    // An exact arrangement exists, but the deterministic greedy sequence misses it.
    let known_miss = [
        PlacementRequest {
            reservation_id: "miss-a".to_owned(),
            block_id: "block-a".to_owned(),
            width: 1,
            height: 1,
            occupied: occupancy,
        },
        PlacementRequest {
            reservation_id: "miss-b".to_owned(),
            block_id: "block-b".to_owned(),
            width: 1,
            height: 2,
            occupied: occupancy,
        },
        PlacementRequest {
            reservation_id: "miss-c".to_owned(),
            block_id: "block-c".to_owned(),
            width: 3,
            height: 2,
            occupied: occupancy,
        },
    ];
    assert!(exact_pack_exists(yard, &known_miss));
    assert!(first_fit_bottom_left(yard, &known_miss, &[]).is_err());

    // False infeasibility is allowed for a heuristic baseline but is measured explicitly.
    // These counters make the baseline's coverage visible in test output for the frozen family.
    eprintln!(
        "tiny packing oracle: cases={examined}, exact_feasible={exact_feasible}, \
         baseline_solved={baseline_solved}, missed_feasible={baseline_missed_feasible}, \
         infeasible_rejected={baseline_rejected_infeasible}"
    );
}
