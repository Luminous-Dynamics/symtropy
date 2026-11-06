//! Independent exhaustive tiny-instance temporal oracle.
//!
//! This test-only oracle reimplements segment timing and resource-calendar
//! feasibility. It is intentionally bounded to tiny finite fixtures and does
//! not call production scheduling/metric helpers.

use super::*;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum SearchTermination {
    ExhaustiveComplete,
    BudgetExhausted,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ObjectiveVector {
    delivery_tick: u64,
    idle_wait_ticks: u128,
    propulsion_wh: u128,
    auxiliary_wh: u128,
    shore_power_import_wh: u128,
    inland_transfer_wh: u128,
    water_ml: u128,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
struct OperationPlacement {
    segment_id: String,
    start_tick: u64,
    end_tick: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OraclePlan {
    alternative_id: String,
    placements: Vec<OperationPlacement>,
    objective: ObjectiveVector,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct OracleResult {
    frontier: Vec<OraclePlan>,
    states_explored: u64,
    termination: SearchTermination,
}

fn resource_use_fits(
    case: &ServiceChainCase,
    segment_index: usize,
    start_tick: u64,
    end_tick: u64,
) -> bool {
    let interval = TickWindow {
        start_tick,
        end_tick,
    };
    case.route_segments[segment_index]
        .resource_uses
        .iter()
        .all(|usage| {
            case.resources
                .iter()
                .find(|resource| resource.resource_id == usage.resource_id)
                .is_some_and(|resource| {
                    resource.availability.iter().any(|window| {
                        window.window.covers(interval) && window.capacity >= usage.amount
                    })
                })
        })
}

fn water_use_ml(case: &ServiceChainCase) -> u128 {
    case.route_segments
        .iter()
        .flat_map(|segment| segment.resource_uses.iter())
        .filter(|usage| usage.unit == ResourceUnit::Millilitres)
        .map(|usage| u128::from(usage.amount))
        .sum()
}

struct TemporalSearch<'a> {
    case: &'a ServiceChainCase,
    states_explored: u64,
    max_states: u64,
    budget_exhausted: bool,
    plans: Vec<OraclePlan>,
}

impl TemporalSearch<'_> {
    fn enumerate_segment_placements(
        &mut self,
        alternative: &VoyageAlternative,
        segment_index: usize,
        previous_end_tick: Option<u64>,
        placements: &mut Vec<OperationPlacement>,
        idle_wait_ticks: u128,
    ) {
        if self.budget_exhausted {
            return;
        }

        if segment_index == alternative.operation_timings.len() {
            let Some(last) = placements.last() else {
                return;
            };
            let delivery_tick = last.end_tick;
            if delivery_tick < self.case.demand.delivery_window.start_tick
                || delivery_tick >= self.case.demand.delivery_window.end_tick
            {
                return;
            }
            let objective = ObjectiveVector {
                delivery_tick,
                idle_wait_ticks,
                propulsion_wh: u128::from(alternative.energy.propulsion_wh),
                auxiliary_wh: u128::from(alternative.energy.auxiliary_wh),
                shore_power_import_wh: u128::from(alternative.energy.shore_power_import_wh),
                inland_transfer_wh: u128::from(alternative.energy.inland_transfer_wh),
                water_ml: water_use_ml(self.case),
            };
            self.plans.push(OraclePlan {
                alternative_id: alternative.alternative_id.clone(),
                placements: placements.clone(),
                objective,
            });
            return;
        }

        let timing = alternative.operation_timings[segment_index].clone();
        let Some(segment) = self
            .case
            .route_segments
            .iter()
            .position(|segment| segment.segment_id == timing.segment_id)
        else {
            return;
        };

        // The channel transit is anchored at the conservative latest arrival
        // tick. Subsequent operations may start at any integer tick after the
        // preceding operation, provided every resource calendar covers the full
        // half-open interval.
        let (mut start_tick, maximum_start_tick) = if segment_index == 0 {
            let Some(arrival_tick) = alternative.arrival_window.end_tick.checked_sub(1) else {
                return;
            };
            let Some(start_tick) = arrival_tick.checked_sub(timing.duration_ticks) else {
                return;
            };
            if start_tick < self.case.decision_tick {
                return;
            }
            (start_tick, start_tick)
        } else {
            let Some(earliest) = previous_end_tick else {
                return;
            };
            let Some(latest) = self
                .case
                .demand
                .delivery_window
                .end_tick
                .checked_sub(timing.duration_ticks)
                .and_then(|value| value.checked_sub(1))
            else {
                return;
            };
            if earliest > latest {
                return;
            }
            (earliest, latest)
        };

        loop {
            if self.states_explored >= self.max_states {
                self.budget_exhausted = true;
                return;
            }
            self.states_explored += 1;

            let Some(end_tick) = start_tick.checked_add(timing.duration_ticks) else {
                return;
            };
            if end_tick < self.case.demand.delivery_window.end_tick
                && resource_use_fits(self.case, segment, start_tick, end_tick)
            {
                let idle = previous_end_tick
                    .map(|previous| u128::from(start_tick - previous))
                    .unwrap_or(0);
                placements.push(OperationPlacement {
                    segment_id: timing.segment_id.clone(),
                    start_tick,
                    end_tick,
                });
                self.enumerate_segment_placements(
                    alternative,
                    segment_index + 1,
                    Some(end_tick),
                    placements,
                    idle_wait_ticks + idle,
                );
                placements.pop();
                if self.budget_exhausted {
                    return;
                }
            }

            if start_tick == maximum_start_tick {
                break;
            }
            let Some(next) = start_tick.checked_add(1) else {
                break;
            };
            start_tick = next;
        }
    }
}
fn dominates(left: ObjectiveVector, right: ObjectiveVector) -> bool {
    let no_worse = left.delivery_tick <= right.delivery_tick
        && left.idle_wait_ticks <= right.idle_wait_ticks
        && left.propulsion_wh <= right.propulsion_wh
        && left.auxiliary_wh <= right.auxiliary_wh
        && left.shore_power_import_wh <= right.shore_power_import_wh
        && left.inland_transfer_wh <= right.inland_transfer_wh
        && left.water_ml <= right.water_ml;
    let strictly_better = left.delivery_tick < right.delivery_tick
        || left.idle_wait_ticks < right.idle_wait_ticks
        || left.propulsion_wh < right.propulsion_wh
        || left.auxiliary_wh < right.auxiliary_wh
        || left.shore_power_import_wh < right.shore_power_import_wh
        || left.inland_transfer_wh < right.inland_transfer_wh
        || left.water_ml < right.water_ml;
    no_worse && strictly_better
}

fn pareto_frontier(mut candidates: Vec<OraclePlan>) -> Vec<OraclePlan> {
    candidates.sort_by_key(|plan| {
        (
            plan.objective.delivery_tick,
            plan.objective.idle_wait_ticks,
            plan.objective.propulsion_wh,
            plan.objective.auxiliary_wh,
            plan.objective.shore_power_import_wh,
            plan.objective.inland_transfer_wh,
            plan.objective.water_ml,
            plan.alternative_id.clone(),
            plan.placements.clone(),
        )
    });
    candidates.dedup();

    let mut frontier = Vec::new();
    for candidate in candidates {
        if frontier
            .iter()
            .any(|existing: &OraclePlan| dominates(existing.objective, candidate.objective))
        {
            continue;
        }
        frontier.retain(|existing| !dominates(candidate.objective, existing.objective));
        frontier.push(candidate);
    }
    frontier.sort_by_key(|plan| {
        (
            plan.objective.delivery_tick,
            plan.objective.idle_wait_ticks,
            plan.objective.propulsion_wh,
            plan.objective.auxiliary_wh,
            plan.objective.shore_power_import_wh,
            plan.objective.inland_transfer_wh,
            plan.objective.water_ml,
            plan.alternative_id.clone(),
            plan.placements.clone(),
        )
    });
    frontier
}

/// Exhaustively enumerate tiny time-expanded plans for every declared voyage
/// alternative. A budget-exhausted result is explicitly non-proven even when
/// the partial candidates happen to look non-dominated.
fn exact_temporal_oracle(
    case: &ServiceChainCase,
    max_states: u64,
) -> Result<OracleResult, ValidationError> {
    validate_case(case)?;

    let mut search = TemporalSearch {
        case,
        states_explored: 0,
        max_states,
        budget_exhausted: false,
        plans: Vec::new(),
    };

    for alternative in &case.voyage_alternatives {
        search.enumerate_segment_placements(alternative, 0, None, &mut Vec::new(), 0);
        if search.budget_exhausted {
            break;
        }
    }

    Ok(OracleResult {
        frontier: pareto_frontier(search.plans),
        states_explored: search.states_explored,
        termination: if search.budget_exhausted {
            SearchTermination::BudgetExhausted
        } else {
            SearchTermination::ExhaustiveComplete
        },
    })
}

#[test]
fn exact_oracle_returns_complete_pareto_frontier_for_synthetic_lock_to_rail_case() {
    let mut case = valid_case();
    // Keep the exhaustive proof finite while preserving both declared alternatives.
    case.demand.delivery_window.end_tick = 36;
    let result = exact_temporal_oracle(&case, 10_000).expect("valid frozen case");

    assert_eq!(result.termination, SearchTermination::ExhaustiveComplete);
    assert!(result.states_explored > 0);
    assert!(
        result.frontier.len() >= 2,
        "both declared speed alternatives must remain represented"
    );

    let fast = result
        .frontier
        .iter()
        .find(|plan| plan.alternative_id == "fast-high-energy")
        .expect("fast alternative is non-dominated");
    assert_eq!(fast.objective.delivery_tick, 33);
    assert_eq!(fast.objective.idle_wait_ticks, 3);
    assert_eq!(fast.objective.propulsion_wh, 1_800);
    assert_eq!(fast.objective.auxiliary_wh, 200);
    assert_eq!(fast.objective.shore_power_import_wh, 800);
    assert_eq!(fast.objective.inland_transfer_wh, 0);
    assert_eq!(fast.objective.water_ml, 4_000);

    let slow = result
        .frontier
        .iter()
        .find(|plan| plan.alternative_id == "slow-low-energy")
        .expect("slow alternative is non-dominated");
    assert_eq!(slow.objective.delivery_tick, 35);
    assert_eq!(slow.objective.idle_wait_ticks, 0);
    assert_eq!(slow.objective.propulsion_wh, 1_300);
    assert_eq!(slow.objective.auxiliary_wh, 200);
    assert_eq!(slow.objective.shore_power_import_wh, 800);
    assert_eq!(slow.objective.inland_transfer_wh, 0);
    assert_eq!(slow.objective.water_ml, 4_000);

    assert!(result.frontier.iter().all(|plan| {
        plan.placements
            .windows(2)
            .all(|pair| pair[0].end_tick <= pair[1].start_tick)
    }));
}

#[test]
fn oracle_budget_exhaustion_never_claims_proven_frontier() {
    let result = exact_temporal_oracle(&valid_case(), 1).expect("valid frozen case");
    assert_eq!(result.termination, SearchTermination::BudgetExhausted);
    assert_eq!(result.states_explored, 1);
}

#[test]
fn no_feasible_delivery_window_returns_an_exhaustively_empty_frontier() {
    let mut case = valid_case();
    case.demand.delivery_window.end_tick = 30;
    assert_eq!(validate_case(&case), Ok(()));

    let result = exact_temporal_oracle(&case, 10_000).expect("valid frozen case");
    assert_eq!(result.termination, SearchTermination::ExhaustiveComplete);
    assert!(result.frontier.is_empty());
}

#[test]
fn invalid_scenario_is_not_misreported_as_infeasible() {
    let mut case = valid_case();
    case.evidence
        .iter_mut()
        .find(|evidence| evidence.evidence_id == "depth")
        .expect("depth evidence exists")
        .state = EvidenceState::Revoked;
    assert_eq!(
        exact_temporal_oracle(&case, 10_000),
        Err(ValidationError::EvidenceNotCurrent(
            "depth".into(),
            EvidenceState::Revoked
        ))
    );
}

#[test]
fn lock_operation_must_fit_both_lock_and_water_quota_windows() {
    let mut case = valid_case();
    let water = case
        .resources
        .iter_mut()
        .find(|resource| resource.resource_id == "water-budget-01")
        .expect("water resource exists");
    water.availability[0].window.end_tick = 13;
    let water_evidence = case
        .evidence
        .iter_mut()
        .find(|evidence| evidence.evidence_id == "water-calendar")
        .expect("water evidence exists");
    water_evidence
        .covered_capacity_windows
        .as_mut()
        .expect("water capacity snapshot exists")[0]
        .window
        .end_tick = 13;

    assert_eq!(validate_case(&case), Ok(()));
    let result = exact_temporal_oracle(&case, 10_000).expect("valid frozen case");
    assert_eq!(result.termination, SearchTermination::ExhaustiveComplete);
    assert!(result.frontier.is_empty());
}
