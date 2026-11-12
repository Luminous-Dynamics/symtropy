//! Bounded exhaustive oracle for tiny, integer-time textile schedules.
//!
//! This implementation is deliberately separate from the schedule validator:
//! it uses its own recursive search and partial-feasibility checks. It supports
//! only explicit zero-duration machine setup transitions. Nonzero setup is
//! rejected as unsupported rather than modeled as free time or labor.

use std::collections::BTreeSet;

use crate::{Assignment, Interval, Machine, Operation, Problem, Schedule};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OracleConfig {
    /// Latest permitted makespan, in the problem's common integer tick.
    pub max_horizon_ticks: u64,
    /// Hard cap across all attempted horizons. Exceeding it returns no result.
    pub max_nodes: u64,
    /// Maximum operation count supported by this exhaustive reference.
    pub max_operations: usize,
}

impl Default for OracleConfig {
    fn default() -> Self {
        Self {
            max_horizon_ticks: 24,
            max_nodes: 1_000_000,
            max_operations: 8,
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OracleError {
    InvalidConfig(&'static str),
    TooManyOperations { actual: usize, maximum: usize },
    DuplicateIdentity { kind: &'static str, id: String },
    ZeroDurationOperation { operation_id: String },
    NonZeroSetupUnsupported { machine_id: String },
    SearchBudgetExceeded { explored_nodes: u64, limit: u64 },
    NoFeasibleScheduleWithinHorizon { maximum_horizon_ticks: u64 },
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OracleResult {
    pub minimum_makespan_ticks: u64,
    pub explored_nodes: u64,
    pub schedule: Schedule,
}

struct Search<'a> {
    problem: &'a Problem,
    horizon: u64,
    max_nodes: u64,
    explored_nodes: u64,
    assigned: Vec<bool>,
    assignments: Vec<Assignment>,
    result: Option<Schedule>,
    budget_exceeded: bool,
}

impl Search<'_> {
    fn visit(&mut self) {
        if self.result.is_some() || self.budget_exceeded {
            return;
        }
        self.explored_nodes = self.explored_nodes.saturating_add(1);
        if self.explored_nodes > self.max_nodes {
            self.budget_exceeded = true;
            return;
        }

        if self.assignments.len() == self.problem.operations.len() {
            let mut assignments = self.assignments.clone();
            assignments.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));
            self.result = Some(Schedule { assignments });
            return;
        }

        let mut ready = Vec::<usize>::new();
        for (index, operation) in self.problem.operations.iter().enumerate() {
            if self.assigned[index] {
                continue;
            }
            let predecessors_ready = self
                .problem
                .precedences
                .iter()
                .filter(|edge| edge.after_operation_id == operation.id)
                .all(|edge| {
                    self.problem
                        .operations
                        .iter()
                        .position(|item| item.id == edge.before_operation_id)
                        .is_some_and(|before_index| self.assigned[before_index])
                });
            if predecessors_ready {
                ready.push(index);
            }
        }
        ready.sort_by_key(|index| self.problem.operations[*index].id.as_str());

        // Every precedence-respecting topological order is explored.
        for operation_index in ready {
            if self.result.is_some() || self.budget_exceeded {
                return;
            }
            let operation = self.problem.operations[operation_index].clone();

            let mut machines = self.problem.machines.clone();
            machines.sort_by_key(|machine| machine.id.clone());
            let mut operators = self.problem.operators.clone();
            operators.sort_by_key(|operator| operator.id.clone());

            for machine in machines {
                if !machine.capabilities.contains(&operation.required_capability) {
                    continue;
                }
                for operator in &operators {
                    if !operator.skills.contains(&operation.required_skill) {
                        continue;
                    }
                    if operation
                        .required_certification
                        .as_ref()
                        .is_some_and(|required| !operator.certifications.contains(required))
                    {
                        continue;
                    }

                    let Some(last_start) = self.horizon.checked_sub(operation.duration_ticks)
                    else {
                        continue;
                    };
                    if operation.release_tick > last_start {
                        continue;
                    }

                    for start_tick in operation.release_tick..=last_start {
                        let Some(finish_tick) = start_tick.checked_add(operation.duration_ticks)
                        else {
                            continue;
                        };
                        if operation
                            .due_tick
                            .is_some_and(|due_tick| finish_tick > due_tick)
                        {
                            continue;
                        }
                        if !interval_available(
                            &machine.availability,
                            start_tick,
                            finish_tick,
                        ) || !interval_available(
                            &operator.availability,
                            start_tick,
                            finish_tick,
                        ) {
                            continue;
                        }
                        if self.resource_conflict(
                            &machine.id,
                            &operator.id,
                            start_tick,
                            finish_tick,
                        ) {
                            continue;
                        }
                        if !self.precedence_satisfied(&operation.id, start_tick) {
                            continue;
                        }
                        if !zero_setup_transitions_valid(
                            &machine,
                            &operation,
                            &self.assignments,
                            &self.problem.operations,
                            start_tick,
                        ) {
                            continue;
                        }

                        self.assigned[operation_index] = true;
                        self.assignments.push(Assignment {
                            operation_id: operation.id.clone(),
                            machine_id: machine.id.clone(),
                            operator_id: operator.id.clone(),
                            start_tick,
                            finish_tick,
                        });
                        self.visit();
                        self.assignments.pop();
                        self.assigned[operation_index] = false;

                        if self.result.is_some() || self.budget_exceeded {
                            return;
                        }
                    }
                }
            }
        }
    }

    fn resource_conflict(
        &self,
        machine_id: &str,
        operator_id: &str,
        start_tick: u64,
        finish_tick: u64,
    ) -> bool {
        self.assignments.iter().any(|existing| {
            let overlaps = start_tick < existing.finish_tick
                && existing.start_tick < finish_tick;
            overlaps
                && (existing.machine_id == machine_id
                    || existing.operator_id == operator_id)
        })
    }

    fn precedence_satisfied(&self, operation_id: &str, start_tick: u64) -> bool {
        self.problem
            .precedences
            .iter()
            .filter(|edge| edge.after_operation_id == operation_id)
            .all(|edge| {
                let Some(predecessor) = self
                    .assignments
                    .iter()
                    .find(|assignment| assignment.operation_id == edge.before_operation_id)
                else {
                    return false;
                };
                predecessor
                    .finish_tick
                    .checked_add(edge.minimum_lag_ticks)
                    .is_some_and(|ready_tick| ready_tick <= start_tick)
            })
    }
}

fn interval_available(windows: &[Interval], start: u64, finish: u64) -> bool {
    windows.iter().any(|window| {
        window.start_tick < window.end_tick
            && window.start_tick <= start
            && finish <= window.end_tick
    })
}

fn zero_setup_transitions_valid(
    machine: &Machine,
    candidate: &Operation,
    assignments: &[Assignment],
    operations: &[Operation],
    candidate_start: u64,
) -> bool {
    let mut on_machine = Vec::<(u64, u64, &Operation)>::new();

    for assignment in assignments
        .iter()
        .filter(|assignment| assignment.machine_id == machine.id)
    {
        let Some(operation) = operations
            .iter()
            .find(|operation| operation.id == assignment.operation_id)
        else {
            return false;
        };
        on_machine.push((assignment.start_tick, assignment.finish_tick, operation));
    }

    let Some(candidate_finish) = candidate_start.checked_add(candidate.duration_ticks) else {
        return false;
    };
    on_machine.push((candidate_start, candidate_finish, candidate));
    on_machine.sort_by_key(|(start, finish, operation)| {
        (*start, *finish, operation.id.as_str())
    });

    let mut previous_family = machine.initial_family.as_str();
    let mut previous_finish = 0_u64;
    for (start, finish, operation) in on_machine {
        if start < previous_finish {
            return false;
        }
        if !has_zero_setup_rule(machine, previous_family, &operation.product_family) {
            return false;
        }
        previous_family = operation.product_family.as_str();
        previous_finish = finish;
    }
    true
}

fn has_zero_setup_rule(machine: &Machine, from: &str, to: &str) -> bool {
    let mut matching = machine
        .setup_rules
        .iter()
        .filter(|rule| rule.from_family == from && rule.to_family == to);
    matches!(
        (matching.next(), matching.next()),
        (Some(rule), None) if rule.duration_ticks == 0
    )
}

fn check_unique<'a>(
    kind: &'static str,
    values: impl Iterator<Item = &'a str>,
) -> Result<(), OracleError> {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value) {
            return Err(OracleError::DuplicateIdentity {
                kind,
                id: value.to_owned(),
            });
        }
    }
    Ok(())
}

/// Find the exact minimum makespan for a bounded tiny problem.
///
/// The search enumerates all precedence-respecting operation orders, eligible
/// machine/operator pairs and integer start ticks for each candidate horizon.
/// v0.1 supports only explicitly declared zero-duration setup transitions;
/// nonzero setup is rejected before search.
pub fn solve(
    problem: &Problem,
    config: OracleConfig,
) -> Result<OracleResult, OracleError> {
    if config.max_nodes == 0 {
        return Err(OracleError::InvalidConfig("max_nodes must be positive"));
    }
    if config.max_operations == 0 {
        return Err(OracleError::InvalidConfig("max_operations must be positive"));
    }
    if problem.operations.len() > config.max_operations {
        return Err(OracleError::TooManyOperations {
            actual: problem.operations.len(),
            maximum: config.max_operations,
        });
    }

    check_unique("operation", problem.operations.iter().map(|item| item.id.as_str()))?;
    check_unique("machine", problem.machines.iter().map(|item| item.id.as_str()))?;
    check_unique("operator", problem.operators.iter().map(|item| item.id.as_str()))?;

    for operation in &problem.operations {
        if operation.duration_ticks == 0 {
            return Err(OracleError::ZeroDurationOperation {
                operation_id: operation.id.clone(),
            });
        }
    }
    for machine in &problem.machines {
        if machine.setup_rules.iter().any(|rule| rule.duration_ticks > 0) {
            return Err(OracleError::NonZeroSetupUnsupported {
                machine_id: machine.id.clone(),
            });
        }
    }

    let mut explored_nodes = 0_u64;
    let minimum_horizon = problem
        .operations
        .iter()
        .map(|operation| operation.duration_ticks)
        .max()
        .unwrap_or(0);

    for horizon in minimum_horizon..=config.max_horizon_ticks {
        let mut search = Search {
            problem,
            horizon,
            max_nodes: config.max_nodes.saturating_sub(explored_nodes),
            explored_nodes: 0,
            assigned: vec![false; problem.operations.len()],
            assignments: Vec::new(),
            result: None,
            budget_exceeded: false,
        };
        search.visit();
        explored_nodes = explored_nodes.saturating_add(search.explored_nodes);

        if search.budget_exceeded || explored_nodes > config.max_nodes {
            return Err(OracleError::SearchBudgetExceeded {
                explored_nodes,
                limit: config.max_nodes,
            });
        }
        if let Some(schedule) = search.result {
            return Ok(OracleResult {
                minimum_makespan_ticks: horizon,
                explored_nodes,
                schedule,
            });
        }
    }

    Err(OracleError::NoFeasibleScheduleWithinHorizon {
        maximum_horizon_ticks: config.max_horizon_ticks,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::two_job_four_stage_case;

    #[test]
    fn exact_oracle_finds_minimum_for_two_competing_cut_operations() {
        let (mut problem, _) = two_job_four_stage_case();
        problem.operations.retain(|operation| {
            operation.id == "cut-a" || operation.id == "cut-b"
        });
        problem.precedences.clear();

        let result = solve(
            &problem,
            OracleConfig {
                max_horizon_ticks: 8,
                max_nodes: 50_000,
                max_operations: 4,
            },
        )
        .expect("tiny instance should be solvable");

        assert_eq!(result.minimum_makespan_ticks, 5);
        assert_eq!(result.schedule.assignments.len(), 2);
        let a = result
            .schedule
            .assignments
            .iter()
            .find(|assignment| assignment.operation_id == "cut-a")
            .unwrap();
        let b = result
            .schedule
            .assignments
            .iter()
            .find(|assignment| assignment.operation_id == "cut-b")
            .unwrap();
        assert!(a.finish_tick <= b.start_tick || b.finish_tick <= a.start_tick);
    }

    #[test]
    fn nonzero_setup_is_rejected_as_unsupported() {
        let (mut problem, _) = two_job_four_stage_case();
        problem.machines[0].setup_rules[0].duration_ticks = 1;
        assert!(matches!(
            solve(&problem, OracleConfig::default()),
            Err(OracleError::NonZeroSetupUnsupported { .. })
        ));
    }

    #[test]
    fn exhausted_search_budget_does_not_return_an_unproven_result() {
        let (mut problem, _) = two_job_four_stage_case();
        problem.operations.retain(|operation| {
            operation.id == "cut-a" || operation.id == "cut-b"
        });
        problem.precedences.clear();

        assert!(matches!(
            solve(
                &problem,
                OracleConfig {
                    max_horizon_ticks: 8,
                    max_nodes: 1,
                    max_operations: 4,
                }
            ),
            Err(OracleError::SearchBudgetExceeded { .. })
        ));
    }
}
