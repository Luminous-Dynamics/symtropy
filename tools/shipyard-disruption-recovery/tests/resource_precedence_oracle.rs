//! Exhaustive resource/precedence oracle for 4,096 tiny operation schedules.
//!
//! The reference predicate uses explicit integer-tick sets and independently checks unary-resource
//! occupancy and finish-to-start timing. It does not call the production overlap or precedence
//! validators.

use std::collections::{BTreeMap, BTreeSet};

use shipyard_disruption_recovery::resources::{
    OperationPrecedence, ResourceReservation, ResourceSchedule, evaluate_resource_recovery,
};
use shipyard_space_time::TimeRange;

const CASES: usize = 4_096;

#[derive(Clone, Copy, Debug)]
struct Operation {
    start: u64,
    duration: u64,
    resource_index: usize,
}

impl Operation {
    fn end(self) -> u64 {
        self.start + self.duration
    }
}

fn ticks(start: u64, end: u64) -> BTreeSet<u64> {
    (start..end).collect()
}

/// Independent oracle: each occupied integer tick may be claimed by at most one operation per
/// resource, and each finish-to-start edge must satisfy the declared minimum lag.
fn exact_resource_schedule_valid(ops: [Operation; 3]) -> bool {
    let mut occupied: BTreeMap<usize, BTreeSet<u64>> = BTreeMap::new();

    for operation in ops {
        let operation_ticks = ticks(operation.start, operation.end());
        let resource_ticks = occupied.entry(operation.resource_index).or_default();
        if !resource_ticks.is_disjoint(&operation_ticks) {
            return false;
        }
        resource_ticks.extend(operation_ticks);
    }

    let [cut, weld, inspect] = ops;
    weld.start >= cut.end() && inspect.start >= weld.end() + 1
}

fn make_schedule(ops: [Operation; 3]) -> ResourceSchedule {
    let operation_ids = ["cut", "weld", "inspect"];
    let reservation_ids = [
        "cut-reservation",
        "weld-reservation",
        "inspection-reservation",
    ];

    ResourceSchedule {
        precedence_constraints: vec![
            OperationPrecedence {
                constraint_id: "cut-before-weld".to_owned(),
                predecessor_operation_id: "cut".to_owned(),
                successor_operation_id: "weld".to_owned(),
                minimum_lag_ticks: 0,
            },
            OperationPrecedence {
                constraint_id: "weld-before-inspection".to_owned(),
                predecessor_operation_id: "weld".to_owned(),
                successor_operation_id: "inspect".to_owned(),
                minimum_lag_ticks: 1,
            },
        ],
        reservations: ops
            .into_iter()
            .enumerate()
            .map(|(index, operation)| ResourceReservation {
                reservation_id: reservation_ids[index].to_owned(),
                operation_id: operation_ids[index].to_owned(),
                resource_id: format!("resource-{}", operation.resource_index),
                resource_kind: "workstation".to_owned(),
                interval: TimeRange {
                    start_tick: operation.start,
                    end_tick: operation.end(),
                },
            })
            .collect(),
    }
}

#[test]
fn exhaustive_three_operation_oracle_matches_resource_and_precedence_verifier() {
    let mut cases = 0usize;
    let mut exact_feasible = 0usize;
    let mut exact_infeasible = 0usize;
    let mut verifier_accepted = 0usize;
    let mut false_feasible = 0usize;
    let mut false_infeasible = 0usize;

    for cut_start in 0..4 {
        for cut_duration in 1..=2 {
            for cut_resource in 0..2 {
                for weld_start in 0..4 {
                    for weld_duration in 1..=2 {
                        for weld_resource in 0..2 {
                            for inspect_start in 0..4 {
                                for inspect_duration in 1..=2 {
                                    for inspect_resource in 0..2 {
                                        let operations = [
                                            Operation {
                                                start: cut_start,
                                                duration: cut_duration,
                                                resource_index: cut_resource,
                                            },
                                            Operation {
                                                start: weld_start,
                                                duration: weld_duration,
                                                resource_index: weld_resource,
                                            },
                                            Operation {
                                                start: inspect_start,
                                                duration: inspect_duration,
                                                resource_index: inspect_resource,
                                            },
                                        ];
                                        let expected = exact_resource_schedule_valid(operations);
                                        let schedule = make_schedule(operations);
                                        let report = evaluate_resource_recovery(
                                            &schedule,
                                            &schedule,
                                            0,
                                            &[],
                                        );
                                        cases += 1;

                                        if expected {
                                            exact_feasible += 1;
                                        } else {
                                            exact_infeasible += 1;
                                        }
                                        if report.accepted {
                                            verifier_accepted += 1;
                                        }
                                        if report.accepted && !expected {
                                            false_feasible += 1;
                                        }
                                        if !report.accepted && expected {
                                            false_infeasible += 1;
                                        }

                                        assert_eq!(
                                            report.metrics.candidate_feasible_after_disruption,
                                            expected,
                                            "candidate-feasibility mismatch for {operations:?}: {:?}",
                                            report.violations
                                        );
                                        assert_eq!(
                                            report.accepted, expected,
                                            "resource/precedence oracle mismatch for {operations:?}: {:?}",
                                            report.violations
                                        );
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    assert_eq!(cases, CASES, "frozen operation schedule count changed");
    assert_eq!(exact_feasible, 16, "tiny exact feasible count changed");
    assert_eq!(
        exact_infeasible, 4_080,
        "tiny exact infeasible count changed"
    );
    assert_eq!(verifier_accepted, exact_feasible);
    assert_eq!(
        false_feasible, 0,
        "resource checker produced a false-feasible case"
    );
    assert_eq!(
        false_infeasible, 0,
        "resource checker missed a feasible case"
    );
}
