#![forbid(unsafe_code)]

//! Independent feasibility checks for bounded multi-stage textile schedules.
//! This crate validates a proposed plan; it does not construct schedules,
//! command equipment, estimate physical outcomes, or qualify a factory.

use std::collections::{BTreeMap, BTreeSet};

pub mod fixtures;
pub mod oracle;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Interval {
    pub start_tick: u64,
    pub end_tick: u64,
}

impl Interval {
    fn is_valid(&self) -> bool {
        self.start_tick < self.end_tick
    }

    fn contains(&self, start: u64, end: u64) -> bool {
        self.start_tick <= start && end <= self.end_tick
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupRule {
    pub from_family: String,
    pub to_family: String,
    pub duration_ticks: u64,
    /// Required operator capability for nonzero setup work.
    pub required_operator_skill: Option<String>,
    pub required_operator_certification: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Machine {
    pub id: String,
    pub capabilities: BTreeSet<String>,
    pub availability: Vec<Interval>,
    pub initial_family: String,
    pub setup_rules: Vec<SetupRule>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operator {
    pub id: String,
    pub skills: BTreeSet<String>,
    pub certifications: BTreeSet<String>,
    pub availability: Vec<Interval>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Operation {
    pub id: String,
    pub job_id: String,
    pub stage_index: u32,
    pub duration_ticks: u64,
    pub release_tick: u64,
    pub due_tick: Option<u64>,
    pub required_capability: String,
    pub required_skill: String,
    pub required_certification: Option<String>,
    pub product_family: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Precedence {
    pub before_operation_id: String,
    pub after_operation_id: String,
    pub minimum_lag_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Assignment {
    pub operation_id: String,
    pub machine_id: String,
    pub operator_id: String,
    pub start_tick: u64,
    pub finish_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SetupReservation {
    pub machine_id: String,
    pub from_family: String,
    pub to_family: String,
    pub predecessor_operation_id: Option<String>,
    pub next_operation_id: String,
    pub operator_id: String,
    pub start_tick: u64,
    pub finish_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Problem {
    pub name: String,
    pub machines: Vec<Machine>,
    pub operators: Vec<Operator>,
    pub operations: Vec<Operation>,
    pub precedences: Vec<Precedence>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Schedule {
    pub assignments: Vec<Assignment>,
    pub setup_reservations: Vec<SetupReservation>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Finding {
    pub code: &'static str,
    pub entity_id: String,
    pub message: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ValidationReport {
    pub valid: bool,
    pub problem_name: String,
    pub expected_operations: usize,
    pub supplied_assignments: usize,
    pub makespan_ticks: Option<u64>,
    pub findings: Vec<Finding>,
}

fn finding(
    findings: &mut Vec<Finding>,
    code: &'static str,
    id: impl Into<String>,
    message: impl Into<String>,
) {
    findings.push(Finding {
        code,
        entity_id: id.into(),
        message: message.into(),
    });
}

fn duplicates<'a>(ids: impl Iterator<Item = &'a str>) -> Vec<String> {
    let mut counts = BTreeMap::<&str, usize>::new();
    for id in ids {
        *counts.entry(id).or_default() += 1;
    }
    counts
        .into_iter()
        .filter(|(_, count)| *count > 1)
        .map(|(id, _)| id.to_owned())
        .collect()
}

fn check_windows(id: &str, windows: &[Interval], findings: &mut Vec<Finding>) {
    let mut sorted = windows.to_vec();
    sorted.sort_by_key(|window| (window.start_tick, window.end_tick));
    for (index, window) in sorted.iter().enumerate() {
        if !window.is_valid() {
            finding(
                findings,
                "INVALID_AVAILABILITY_WINDOW",
                id,
                "availability interval is empty or reversed",
            );
        }
        if index > 0 && sorted[index - 1].end_tick > window.start_tick {
            finding(
                findings,
                "OVERLAPPING_AVAILABILITY_WINDOWS",
                id,
                "availability windows overlap and must be normalized",
            );
        }
    }
}

fn available(windows: &[Interval], start: u64, end: u64) -> bool {
    windows
        .iter()
        .any(|window| window.is_valid() && window.contains(start, end))
}

fn setup_available(
    windows: &[Interval],
    earliest_start: u64,
    latest_finish: u64,
    duration: u64,
) -> bool {
    if duration == 0 {
        return true;
    }
    if earliest_start >= latest_finish {
        return false;
    }

    windows.iter().any(|window| {
        window.is_valid() && {
            let start = window.start_tick.max(earliest_start);
            let end = window.end_tick.min(latest_finish);
            start
                .checked_add(duration)
                .is_some_and(|finish| finish <= end)
        }
    })
}

/// Validate a complete proposed schedule without invoking an optimizer.
///
/// Intervals are half-open: start is included and finish is excluded. All
/// time values use one caller-declared integer tick. Every machine setup
/// transition must be declared, including zero-duration transitions. Missing
/// setup information is unknown, not a zero-cost setup.
pub fn validate(problem: &Problem, schedule: &Schedule) -> ValidationReport {
    let mut findings = Vec::new();

    for id in duplicates(problem.machines.iter().map(|item| item.id.as_str())) {
        finding(
            &mut findings,
            "DUPLICATE_MACHINE_ID",
            id,
            "machine IDs must be unique",
        );
    }
    for id in duplicates(problem.operators.iter().map(|item| item.id.as_str())) {
        finding(
            &mut findings,
            "DUPLICATE_OPERATOR_ID",
            id,
            "operator IDs must be unique",
        );
    }
    for id in duplicates(problem.operations.iter().map(|item| item.id.as_str())) {
        finding(
            &mut findings,
            "DUPLICATE_OPERATION_ID",
            id,
            "operation IDs must be unique",
        );
    }

    let mut machines = BTreeMap::<&str, &Machine>::new();
    for machine in &problem.machines {
        machines.entry(machine.id.as_str()).or_insert(machine);
        check_windows(&machine.id, &machine.availability, &mut findings);
        let mut setup_counts = BTreeMap::<(&str, &str), usize>::new();
        for rule in &machine.setup_rules {
            *setup_counts
                .entry((rule.from_family.as_str(), rule.to_family.as_str()))
                .or_default() += 1;
        }
        for ((from, to), count) in setup_counts {
            if count > 1 {
                finding(
                    &mut findings,
                    "DUPLICATE_SETUP_RULE",
                    &machine.id,
                    format!("multiple setup rules are declared for {from} -> {to}"),
                );
            }
        }
    }

    let mut operators = BTreeMap::<&str, &Operator>::new();
    for operator in &problem.operators {
        operators.entry(operator.id.as_str()).or_insert(operator);
        check_windows(&operator.id, &operator.availability, &mut findings);
    }

    let mut operations = BTreeMap::<&str, &Operation>::new();
    for operation in &problem.operations {
        operations.entry(operation.id.as_str()).or_insert(operation);
        if operation.duration_ticks == 0 {
            finding(
                &mut findings,
                "ZERO_DURATION_OPERATION",
                &operation.id,
                "operation duration must be positive",
            );
        }
        if operation.required_capability.is_empty() || operation.required_skill.is_empty() {
            finding(
                &mut findings,
                "MISSING_REQUIRED_CAPABILITY",
                &operation.id,
                "machine capability and operator skill must be declared",
            );
        }
    }

    let mut counts = BTreeMap::<&str, usize>::new();
    let mut by_operation = BTreeMap::<&str, &Assignment>::new();
    let mut by_machine = BTreeMap::<String, Vec<(&Assignment, &Operation)>>::new();
    let mut by_operator = BTreeMap::<String, Vec<(&Assignment, &Operation)>>::new();

    for assignment in &schedule.assignments {
        *counts.entry(assignment.operation_id.as_str()).or_default() += 1;
        by_operation
            .entry(assignment.operation_id.as_str())
            .or_insert(assignment);

        let Some(operation) = operations.get(assignment.operation_id.as_str()) else {
            finding(
                &mut findings,
                "UNKNOWN_OPERATION_ASSIGNMENT",
                &assignment.operation_id,
                "assignment references an operation absent from the frozen problem",
            );
            continue;
        };
        let operation = *operation;
        let interval_valid = assignment.start_tick < assignment.finish_tick;

        if !interval_valid {
            finding(
                &mut findings,
                "INVALID_ASSIGNMENT_INTERVAL",
                &assignment.operation_id,
                "assignment finish must be greater than start",
            );
        } else {
            let actual = assignment.finish_tick - assignment.start_tick;
            if actual != operation.duration_ticks {
                finding(
                    &mut findings,
                    "DURATION_MISMATCH",
                    &assignment.operation_id,
                    format!(
                        "expected {} ticks but assignment spans {actual}",
                        operation.duration_ticks
                    ),
                );
            }
            if assignment.start_tick < operation.release_tick {
                finding(
                    &mut findings,
                    "RELEASE_TIME_VIOLATION",
                    &assignment.operation_id,
                    "operation starts before its declared release time",
                );
            }
            if operation
                .due_tick
                .is_some_and(|due| assignment.finish_tick > due)
            {
                finding(
                    &mut findings,
                    "DUE_TIME_VIOLATION",
                    &assignment.operation_id,
                    "operation finishes after its declared hard due time",
                );
            }
        }

        match machines.get(assignment.machine_id.as_str()) {
            Some(machine) => {
                if !machine
                    .capabilities
                    .contains(&operation.required_capability)
                {
                    finding(
                        &mut findings,
                        "MACHINE_CAPABILITY_MISMATCH",
                        &assignment.operation_id,
                        format!(
                            "machine {} lacks {}",
                            machine.id, operation.required_capability
                        ),
                    );
                }
                if interval_valid
                    && !available(
                        &machine.availability,
                        assignment.start_tick,
                        assignment.finish_tick,
                    )
                {
                    finding(
                        &mut findings,
                        "MACHINE_UNAVAILABLE",
                        &assignment.operation_id,
                        format!(
                            "machine {} is unavailable for the full operation interval",
                            machine.id
                        ),
                    );
                }
                if interval_valid {
                    by_machine
                        .entry(machine.id.clone())
                        .or_default()
                        .push((assignment, operation));
                }
            }
            None => finding(
                &mut findings,
                "UNKNOWN_MACHINE",
                &assignment.operation_id,
                format!("machine {} is not declared", assignment.machine_id),
            ),
        }

        match operators.get(assignment.operator_id.as_str()) {
            Some(operator) => {
                if !operator.skills.contains(&operation.required_skill) {
                    finding(
                        &mut findings,
                        "OPERATOR_SKILL_MISMATCH",
                        &assignment.operation_id,
                        format!(
                            "operator {} lacks {}",
                            operator.id, operation.required_skill
                        ),
                    );
                }
                if let Some(certification) = &operation.required_certification
                    && !operator.certifications.contains(certification)
                {
                    finding(
                        &mut findings,
                        "OPERATOR_CERTIFICATION_MISMATCH",
                        &assignment.operation_id,
                        format!("operator {} lacks {certification}", operator.id),
                    );
                }
                if interval_valid
                    && !available(
                        &operator.availability,
                        assignment.start_tick,
                        assignment.finish_tick,
                    )
                {
                    finding(
                        &mut findings,
                        "OPERATOR_UNAVAILABLE",
                        &assignment.operation_id,
                        format!(
                            "operator {} is unavailable for the full operation interval",
                            operator.id
                        ),
                    );
                }
                if interval_valid {
                    by_operator
                        .entry(operator.id.clone())
                        .or_default()
                        .push((assignment, operation));
                }
            }
            None => finding(
                &mut findings,
                "UNKNOWN_OPERATOR",
                &assignment.operation_id,
                format!("operator {} is not declared", assignment.operator_id),
            ),
        }
    }

    for operation in &problem.operations {
        match counts.get(operation.id.as_str()).copied().unwrap_or(0) {
            0 => finding(
                &mut findings,
                "MISSING_OPERATION_ASSIGNMENT",
                &operation.id,
                "every required operation must be assigned exactly once",
            ),
            1 => {}
            count => finding(
                &mut findings,
                "DUPLICATE_OPERATION_ASSIGNMENT",
                &operation.id,
                format!("operation has {count} assignments; exactly one is required"),
            ),
        }
    }

    for (machine_id, reservations) in &mut by_machine {
        reservations.sort_by_key(|(a, _)| (a.start_tick, a.finish_tick, a.operation_id.as_str()));
        for pair in reservations.windows(2) {
            if pair[1].0.start_tick < pair[0].0.finish_tick {
                finding(
                    &mut findings,
                    "MACHINE_OVERLAP",
                    machine_id,
                    format!(
                        "operations {} and {} overlap",
                        pair[0].0.operation_id, pair[1].0.operation_id
                    ),
                );
            }
        }
    }

    for (operator_id, reservations) in &mut by_operator {
        reservations.sort_by_key(|(a, _)| (a.start_tick, a.finish_tick, a.operation_id.as_str()));
        for pair in reservations.windows(2) {
            if pair[1].0.start_tick < pair[0].0.finish_tick {
                finding(
                    &mut findings,
                    "OPERATOR_OVERLAP",
                    operator_id,
                    format!(
                        "operations {} and {} overlap",
                        pair[0].0.operation_id, pair[1].0.operation_id
                    ),
                );
            }
        }
    }

    let mut used_setup_reservations = BTreeSet::<usize>::new();

    for (machine_id, reservations) in &mut by_machine {
        let Some(machine) = machines.get(machine_id.as_str()) else {
            continue;
        };
        reservations.sort_by_key(|(a, _)| (a.start_tick, a.finish_tick, a.operation_id.as_str()));
        let mut previous_finish = 0_u64;
        let mut previous_family = machine.initial_family.as_str();
        let mut previous_operation_id: Option<&str> = None;

        for (assignment, operation) in reservations.iter() {
            let matching: Vec<&SetupRule> = machine
                .setup_rules
                .iter()
                .filter(|rule| {
                    rule.from_family == previous_family
                        && rule.to_family == operation.product_family
                })
                .collect();

            match matching.as_slice() {
                [] => finding(
                    &mut findings,
                    "SETUP_RULE_UNKNOWN",
                    &assignment.operation_id,
                    format!(
                        "machine {machine_id} has no setup rule for {previous_family} -> {}",
                        operation.product_family
                    ),
                ),
                [rule] => {
                    if rule.duration_ticks > 0 {
                        match previous_finish.checked_add(rule.duration_ticks) {
                            Some(ready) if ready <= assignment.start_tick => {}
                            Some(ready) => finding(
                                &mut findings,
                                "SETUP_TIME_VIOLATION",
                                &assignment.operation_id,
                                format!(
                                    "machine {machine_id} is ready at {ready}, after operation start {}",
                                    assignment.start_tick
                                ),
                            ),
                            None => finding(
                                &mut findings,
                                "TIME_OVERFLOW",
                                &assignment.operation_id,
                                "previous finish plus setup duration overflows u64",
                            ),
                        }

                        if !setup_available(
                            &machine.availability,
                            previous_finish,
                            assignment.start_tick,
                            rule.duration_ticks,
                        ) {
                            finding(
                                &mut findings,
                                "SETUP_AVAILABILITY_VIOLATION",
                                &assignment.operation_id,
                                format!(
                                    "machine {machine_id} has no available window for the required setup"
                                ),
                            );
                        }
                    }

                    let candidates: Vec<(usize, &SetupReservation)> = schedule
                        .setup_reservations
                        .iter()
                        .enumerate()
                        .filter(|(_, reservation)| {
                            reservation.machine_id == machine_id.as_str()
                                && reservation.predecessor_operation_id.as_deref()
                                    == previous_operation_id
                                && reservation.next_operation_id == assignment.operation_id
                        })
                        .collect();

                    if rule.duration_ticks == 0 {
                        for (index, _) in &candidates {
                            used_setup_reservations.insert(*index);
                        }
                        if !candidates.is_empty() {
                            finding(
                                &mut findings,
                                "UNEXPECTED_SETUP_RESERVATION",
                                &assignment.operation_id,
                                "zero-duration setup must not reserve setup labor or machine time",
                            );
                        }
                    } else {
                        if candidates.is_empty() {
                            finding(
                                &mut findings,
                                "SETUP_RESERVATION_MISSING",
                                &assignment.operation_id,
                                "nonzero setup requires one explicit machine and operator reservation",
                            );
                        }
                        if candidates.len() > 1 {
                            finding(
                                &mut findings,
                                "DUPLICATE_SETUP_RESERVATION",
                                &assignment.operation_id,
                                "exactly one setup reservation is allowed for this machine transition",
                            );
                        }

                        if let Some((index, reservation)) = candidates.first() {
                            used_setup_reservations.insert(*index);
                            if reservation.from_family != rule.from_family
                                || reservation.to_family != rule.to_family
                            {
                                finding(
                                    &mut findings,
                                    "SETUP_RESERVATION_TRANSITION_MISMATCH",
                                    &assignment.operation_id,
                                    "setup reservation family transition does not match the machine setup rule",
                                );
                            }
                            if reservation.start_tick >= reservation.finish_tick
                                || reservation.finish_tick - reservation.start_tick
                                    != rule.duration_ticks
                            {
                                finding(
                                    &mut findings,
                                    "SETUP_RESERVATION_DURATION_MISMATCH",
                                    &assignment.operation_id,
                                    "setup reservation must have the exact declared positive duration",
                                );
                            }
                            if reservation.start_tick < previous_finish
                                || reservation.finish_tick > assignment.start_tick
                            {
                                finding(
                                    &mut findings,
                                    "SETUP_RESERVATION_OUTSIDE_TRANSITION_GAP",
                                    &assignment.operation_id,
                                    "setup reservation must fit between predecessor finish and next operation start",
                                );
                            }
                            if let Some(skill) = rule
                                .required_operator_skill
                                .as_deref()
                                .filter(|value| !value.trim().is_empty())
                            {
                                if let Some(operator) =
                                    operators.get(reservation.operator_id.as_str())
                                {
                                    if !operator.skills.contains(skill) {
                                        finding(
                                            &mut findings,
                                            "SETUP_OPERATOR_SKILL_MISMATCH",
                                            &assignment.operation_id,
                                            format!(
                                                "setup operator {} lacks required skill {skill}",
                                                operator.id
                                            ),
                                        );
                                    }
                                    if let Some(certification) =
                                        rule.required_operator_certification.as_deref()
                                        && !operator.certifications.contains(certification)
                                    {
                                        finding(
                                            &mut findings,
                                            "SETUP_OPERATOR_CERTIFICATION_MISMATCH",
                                            &assignment.operation_id,
                                            format!(
                                                "setup operator {} lacks required certification {certification}",
                                                operator.id
                                            ),
                                        );
                                    }
                                }
                            } else {
                                finding(
                                    &mut findings,
                                    "SETUP_OPERATOR_SKILL_UNDECLARED",
                                    &assignment.operation_id,
                                    "a positive-duration setup rule must declare the required operator skill",
                                );
                            }
                        }
                    }
                }
                _ => finding(
                    &mut findings,
                    "SETUP_RULE_AMBIGUOUS",
                    &assignment.operation_id,
                    format!(
                        "machine {machine_id} has multiple rules for {previous_family} -> {}",
                        operation.product_family
                    ),
                ),
            }
            previous_finish = assignment.finish_tick;
            previous_family = operation.product_family.as_str();
            previous_operation_id = Some(assignment.operation_id.as_str());
        }
    }

    for (index, reservation) in schedule.setup_reservations.iter().enumerate() {
        if !used_setup_reservations.contains(&index) {
            finding(
                &mut findings,
                "ORPHAN_SETUP_RESERVATION",
                &reservation.next_operation_id,
                "setup reservation does not correspond to a declared nonzero setup transition",
            );
        }
        if reservation.start_tick >= reservation.finish_tick {
            finding(
                &mut findings,
                "INVALID_SETUP_RESERVATION_INTERVAL",
                &reservation.next_operation_id,
                "setup reservation finish must be greater than start",
            );
        }
        if let Some(machine) = machines.get(reservation.machine_id.as_str()) {
            if reservation.start_tick < reservation.finish_tick
                && !available(
                    &machine.availability,
                    reservation.start_tick,
                    reservation.finish_tick,
                )
            {
                finding(
                    &mut findings,
                    "SETUP_MACHINE_UNAVAILABLE",
                    &reservation.next_operation_id,
                    format!(
                        "machine {} is unavailable for the full setup interval",
                        machine.id
                    ),
                );
            }
        } else {
            finding(
                &mut findings,
                "UNKNOWN_SETUP_MACHINE",
                &reservation.next_operation_id,
                format!(
                    "setup reservation references unknown machine {}",
                    reservation.machine_id
                ),
            );
        }

        if let Some(operator) = operators.get(reservation.operator_id.as_str()) {
            if reservation.start_tick < reservation.finish_tick
                && !available(
                    &operator.availability,
                    reservation.start_tick,
                    reservation.finish_tick,
                )
            {
                finding(
                    &mut findings,
                    "SETUP_OPERATOR_UNAVAILABLE",
                    &reservation.next_operation_id,
                    format!(
                        "setup operator {} is unavailable for the full setup interval",
                        operator.id
                    ),
                );
            }
            if by_operator
                .get(operator.id.as_str())
                .is_some_and(|assignments| {
                    assignments.iter().any(|(assignment, _)| {
                        reservation.start_tick < assignment.finish_tick
                            && assignment.start_tick < reservation.finish_tick
                    })
                })
            {
                finding(
                    &mut findings,
                    "SETUP_OPERATOR_OPERATION_OVERLAP",
                    &reservation.next_operation_id,
                    format!(
                        "setup operator {} is already assigned to an operation during setup",
                        operator.id
                    ),
                );
            }
        } else {
            finding(
                &mut findings,
                "UNKNOWN_SETUP_OPERATOR",
                &reservation.next_operation_id,
                format!(
                    "setup reservation references unknown operator {}",
                    reservation.operator_id
                ),
            );
        }

        for (other_index, other) in schedule.setup_reservations.iter().enumerate() {
            if other_index <= index {
                continue;
            }
            let overlap = reservation.start_tick < other.finish_tick
                && other.start_tick < reservation.finish_tick;
            if overlap && reservation.operator_id == other.operator_id {
                finding(
                    &mut findings,
                    "SETUP_OPERATOR_OVERLAP",
                    &reservation.next_operation_id,
                    format!(
                        "setup operator {} has overlapping setup reservations",
                        reservation.operator_id
                    ),
                );
            }
            if overlap && reservation.machine_id == other.machine_id {
                finding(
                    &mut findings,
                    "SETUP_MACHINE_OVERLAP",
                    &reservation.next_operation_id,
                    format!(
                        "machine {} has overlapping setup reservations",
                        reservation.machine_id
                    ),
                );
            }
        }
    }

    for edge in &problem.precedences {
        let Some(before) = by_operation.get(edge.before_operation_id.as_str()) else {
            finding(
                &mut findings,
                "PRECEDENCE_INPUT_MISSING",
                &edge.before_operation_id,
                "precedence predecessor has no assignment",
            );
            continue;
        };
        let Some(after) = by_operation.get(edge.after_operation_id.as_str()) else {
            finding(
                &mut findings,
                "PRECEDENCE_INPUT_MISSING",
                &edge.after_operation_id,
                "precedence successor has no assignment",
            );
            continue;
        };

        match before.finish_tick.checked_add(edge.minimum_lag_ticks) {
            Some(ready) if ready <= after.start_tick => {}
            Some(ready) => finding(
                &mut findings,
                "PRECEDENCE_VIOLATION",
                &edge.after_operation_id,
                format!(
                    "{} must finish by tick {ready} before {} starts at {}",
                    edge.before_operation_id, edge.after_operation_id, after.start_tick
                ),
            ),
            None => finding(
                &mut findings,
                "TIME_OVERFLOW",
                &edge.before_operation_id,
                "predecessor finish plus minimum lag overflows u64",
            ),
        }
    }

    findings.sort_by(|a, b| {
        (&a.code, &a.entity_id, &a.message).cmp(&(&b.code, &b.entity_id, &b.message))
    });
    let makespan = schedule
        .assignments
        .iter()
        .filter(|a| a.start_tick < a.finish_tick)
        .map(|a| a.finish_tick)
        .max();

    ValidationReport {
        valid: findings.is_empty(),
        problem_name: problem.name.clone(),
        expected_operations: problem.operations.len(),
        supplied_assignments: schedule.assignments.len(),
        makespan_ticks: makespan,
        findings,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::fixtures::two_job_four_stage_case;

    fn has(report: &ValidationReport, code: &str) -> bool {
        report.findings.iter().any(|item| item.code == code)
    }

    fn setup_required_case() -> (Problem, Schedule) {
        let (mut problem, mut schedule) = two_job_four_stage_case();

        for operation in problem
            .operations
            .iter_mut()
            .filter(|operation| operation.job_id == "job-b")
        {
            operation.product_family = "cotton".to_owned();
        }

        for machine in problem
            .machines
            .iter_mut()
            .filter(|machine| machine.id != "cut-line")
        {
            machine.setup_rules.push(SetupRule {
                from_family: "woven".to_owned(),
                to_family: "cotton".to_owned(),
                duration_ticks: 0,
                required_operator_skill: None,
                required_operator_certification: None,
            });
        }

        let cutting_machine = problem
            .machines
            .iter_mut()
            .find(|machine| machine.id == "cut-line")
            .unwrap();
        cutting_machine.setup_rules.push(SetupRule {
            from_family: "woven".to_owned(),
            to_family: "cotton".to_owned(),
            duration_ticks: 2,
            required_operator_skill: Some("changeover".to_owned()),
            required_operator_certification: Some("changeover-qualified".to_owned()),
        });

        problem.operators.push(Operator {
            id: "setup-tech".to_owned(),
            skills: ["changeover".to_owned()].into_iter().collect(),
            certifications: ["changeover-qualified".to_owned()].into_iter().collect(),
            availability: vec![Interval {
                start_tick: 0,
                end_tick: 30,
            }],
        });

        for (operation_id, start, finish) in [
            ("cut-b", 7, 9),
            ("sew-b", 9, 13),
            ("finish-b", 13, 15),
            ("inspect-b", 15, 16),
        ] {
            let assignment = schedule
                .assignments
                .iter_mut()
                .find(|assignment| assignment.operation_id == operation_id)
                .unwrap();
            assignment.start_tick = start;
            assignment.finish_tick = finish;
        }

        schedule.setup_reservations.push(SetupReservation {
            machine_id: "cut-line".to_owned(),
            from_family: "woven".to_owned(),
            to_family: "cotton".to_owned(),
            predecessor_operation_id: Some("cut-a".to_owned()),
            next_operation_id: "cut-b".to_owned(),
            operator_id: "setup-tech".to_owned(),
            start_tick: 3,
            finish_tick: 5,
        });

        (problem, schedule)
    }

    #[test]
    fn known_answer_case_passes() {
        let (problem, schedule) = two_job_four_stage_case();
        let report = validate(&problem, &schedule);
        assert!(report.valid, "{:#?}", report.findings);
        assert_eq!(report.expected_operations, 8);
        assert_eq!(report.supplied_assignments, 8);
        assert_eq!(report.makespan_ticks, Some(15));
    }

    #[test]
    fn missing_and_duplicate_operations_fail() {
        let (problem, mut missing) = two_job_four_stage_case();
        missing.assignments.retain(|a| a.operation_id != "cut-b");
        assert!(has(
            &validate(&problem, &missing),
            "MISSING_OPERATION_ASSIGNMENT"
        ));

        let (problem, mut duplicate) = two_job_four_stage_case();
        duplicate.assignments.push(duplicate.assignments[0].clone());
        assert!(has(
            &validate(&problem, &duplicate),
            "DUPLICATE_OPERATION_ASSIGNMENT"
        ));
    }

    #[test]
    fn duration_and_due_time_are_hard_constraints() {
        let (problem, mut schedule) = two_job_four_stage_case();
        schedule.assignments[0].finish_tick += 1;
        assert!(has(&validate(&problem, &schedule), "DURATION_MISMATCH"));

        let (mut problem, schedule) = two_job_four_stage_case();
        problem
            .operations
            .iter_mut()
            .find(|o| o.id == "sew-a")
            .unwrap()
            .due_tick = Some(4);
        assert!(has(&validate(&problem, &schedule), "DUE_TIME_VIOLATION"));
    }

    #[test]
    fn machine_and_operator_non_overlap_are_enforced() {
        let (problem, mut schedule) = two_job_four_stage_case();
        let cut_b = schedule
            .assignments
            .iter_mut()
            .find(|a| a.operation_id == "cut-b")
            .unwrap();
        cut_b.start_tick = 2;
        cut_b.finish_tick = 4;
        assert!(has(&validate(&problem, &schedule), "MACHINE_OVERLAP"));

        let (mut problem, mut schedule) = two_job_four_stage_case();
        let finisher = problem
            .operators
            .iter_mut()
            .find(|o| o.id == "finisher")
            .unwrap();
        finisher.skills.insert("sewing".to_owned());
        finisher.certifications.insert("sew-qualified".to_owned());
        schedule
            .assignments
            .iter_mut()
            .find(|a| a.operation_id == "sew-b")
            .unwrap()
            .operator_id = "finisher".to_owned();
        assert!(has(&validate(&problem, &schedule), "OPERATOR_OVERLAP"));
    }

    #[test]
    fn machine_capability_operator_skill_and_certification_are_checked() {
        let (mut problem, schedule) = two_job_four_stage_case();
        problem
            .machines
            .iter_mut()
            .find(|m| m.id == "sew-line")
            .unwrap()
            .capabilities
            .clear();
        assert!(has(
            &validate(&problem, &schedule),
            "MACHINE_CAPABILITY_MISMATCH"
        ));

        let (problem, mut schedule) = two_job_four_stage_case();
        schedule
            .assignments
            .iter_mut()
            .find(|a| a.operation_id == "cut-a")
            .unwrap()
            .operator_id = "sewer".to_owned();
        assert!(has(
            &validate(&problem, &schedule),
            "OPERATOR_SKILL_MISMATCH"
        ));
    }

    #[test]
    fn resource_availability_is_enforced() {
        let (mut problem, schedule) = two_job_four_stage_case();
        problem.machines[0].availability = vec![Interval {
            start_tick: 0,
            end_tick: 2,
        }];
        problem.operators[0].availability = vec![Interval {
            start_tick: 0,
            end_tick: 2,
        }];
        let report = validate(&problem, &schedule);
        assert!(has(&report, "MACHINE_UNAVAILABLE"));
        assert!(has(&report, "OPERATOR_UNAVAILABLE"));
    }

    #[test]
    fn missing_and_insufficient_setup_rules_fail_closed() {
        let (mut problem, schedule) = two_job_four_stage_case();
        problem.machines[1].setup_rules.clear();
        assert!(has(&validate(&problem, &schedule), "SETUP_RULE_UNKNOWN"));

        let (mut problem, schedule) = two_job_four_stage_case();
        problem.machines[0].initial_family = "knit".to_owned();
        problem.machines[0].setup_rules.push(SetupRule {
            from_family: "knit".to_owned(),
            to_family: "woven".to_owned(),
            duration_ticks: 2,
            required_operator_skill: Some("changeover".to_owned()),
            required_operator_certification: Some("changeover-qualified".to_owned()),
        });
        assert!(has(&validate(&problem, &schedule), "SETUP_TIME_VIOLATION"));
    }

    #[test]
    fn setup_must_fit_inside_machine_availability() {
        let (mut problem, mut schedule) = two_job_four_stage_case();
        let cutting_machine = problem
            .machines
            .iter_mut()
            .find(|machine| machine.id == "cut-line")
            .unwrap();
        cutting_machine.availability = vec![
            Interval {
                start_tick: 0,
                end_tick: 3,
            },
            Interval {
                start_tick: 6,
                end_tick: 30,
            },
        ];
        cutting_machine.setup_rules.push(SetupRule {
            from_family: "woven".to_owned(),
            to_family: "cotton".to_owned(),
            duration_ticks: 2,
            required_operator_skill: Some("changeover".to_owned()),
            required_operator_certification: Some("changeover-qualified".to_owned()),
        });
        problem
            .operations
            .iter_mut()
            .find(|operation| operation.id == "cut-b")
            .unwrap()
            .product_family = "cotton".to_owned();
        let cut_b = schedule
            .assignments
            .iter_mut()
            .find(|assignment| assignment.operation_id == "cut-b")
            .unwrap();
        cut_b.start_tick = 6;
        cut_b.finish_tick = 8;

        assert!(has(
            &validate(&problem, &schedule),
            "SETUP_AVAILABILITY_VIOLATION"
        ));
    }

    #[test]
    fn qualified_setup_operator_can_reserve_machine_idle_gap() {
        let (problem, schedule) = setup_required_case();
        let report = validate(&problem, &schedule);
        assert!(report.valid, "{:#?}", report.findings);
        assert_eq!(report.makespan_ticks, Some(16));
    }

    #[test]
    fn nonzero_setup_requires_exactly_one_reservation() {
        let (problem, mut schedule) = setup_required_case();
        schedule.setup_reservations.clear();
        assert!(has(
            &validate(&problem, &schedule),
            "SETUP_RESERVATION_MISSING"
        ));
    }

    #[test]
    fn setup_operator_must_hold_declared_skill_and_certification() {
        let (mut problem, schedule) = setup_required_case();
        problem
            .operators
            .iter_mut()
            .find(|operator| operator.id == "setup-tech")
            .unwrap()
            .skills
            .clear();
        assert!(has(
            &validate(&problem, &schedule),
            "SETUP_OPERATOR_SKILL_MISMATCH"
        ));

        let (mut problem, schedule) = setup_required_case();
        problem
            .operators
            .iter_mut()
            .find(|operator| operator.id == "setup-tech")
            .unwrap()
            .certifications
            .clear();
        assert!(has(
            &validate(&problem, &schedule),
            "SETUP_OPERATOR_CERTIFICATION_MISMATCH"
        ));
    }

    #[test]
    fn setup_operator_cannot_overlap_a_production_operation() {
        let (mut problem, mut schedule) = setup_required_case();
        let sewer = problem
            .operators
            .iter_mut()
            .find(|operator| operator.id == "sewer")
            .unwrap();
        sewer.skills.insert("changeover".to_owned());
        sewer
            .certifications
            .insert("changeover-qualified".to_owned());
        schedule.setup_reservations[0].operator_id = "sewer".to_owned();
        assert!(has(
            &validate(&problem, &schedule),
            "SETUP_OPERATOR_OPERATION_OVERLAP"
        ));
    }

    #[test]
    fn duplicate_setup_reservations_are_rejected() {
        let (problem, mut schedule) = setup_required_case();
        schedule
            .setup_reservations
            .push(schedule.setup_reservations[0].clone());
        assert!(has(
            &validate(&problem, &schedule),
            "DUPLICATE_SETUP_RESERVATION"
        ));
    }

    #[test]
    fn setup_machine_and_operator_must_both_be_available() {
        let (mut problem, schedule) = setup_required_case();
        let cutter = problem
            .machines
            .iter_mut()
            .find(|machine| machine.id == "cut-line")
            .unwrap();
        cutter.availability = vec![
            Interval {
                start_tick: 0,
                end_tick: 3,
            },
            Interval {
                start_tick: 5,
                end_tick: 30,
            },
        ];
        let setup_operator = problem
            .operators
            .iter_mut()
            .find(|operator| operator.id == "setup-tech")
            .unwrap();
        setup_operator.availability = vec![Interval {
            start_tick: 0,
            end_tick: 3,
        }];
        let report = validate(&problem, &schedule);
        assert!(has(&report, "SETUP_MACHINE_UNAVAILABLE"));
        assert!(has(&report, "SETUP_OPERATOR_UNAVAILABLE"));
    }

    #[test]
    fn setup_reservation_must_match_transition_and_exact_interval() {
        let (problem, mut wrong_family) = setup_required_case();
        wrong_family.setup_reservations[0].to_family = "linen".to_owned();
        assert!(has(
            &validate(&problem, &wrong_family),
            "SETUP_RESERVATION_TRANSITION_MISMATCH"
        ));

        let (problem, mut wrong_duration) = setup_required_case();
        wrong_duration.setup_reservations[0].finish_tick = 4;
        assert!(has(
            &validate(&problem, &wrong_duration),
            "SETUP_RESERVATION_DURATION_MISMATCH"
        ));

        let (problem, mut outside_gap) = setup_required_case();
        outside_gap.setup_reservations[0].start_tick = 2;
        outside_gap.setup_reservations[0].finish_tick = 4;
        assert!(has(
            &validate(&problem, &outside_gap),
            "SETUP_RESERVATION_OUTSIDE_TRANSITION_GAP"
        ));
    }

    #[test]
    fn orphan_setup_reservation_cannot_satisfy_another_transition() {
        let (problem, mut schedule) = setup_required_case();
        schedule.setup_reservations[0].predecessor_operation_id =
            Some("unrelated-operation".to_owned());
        let report = validate(&problem, &schedule);
        assert!(has(&report, "SETUP_RESERVATION_MISSING"));
        assert!(has(&report, "ORPHAN_SETUP_RESERVATION"));
    }

    #[test]
    fn zero_duration_transition_rejects_setup_labor_reservation() {
        let (problem, mut schedule) = two_job_four_stage_case();
        schedule.setup_reservations.push(SetupReservation {
            machine_id: "cut-line".to_owned(),
            from_family: "woven".to_owned(),
            to_family: "woven".to_owned(),
            predecessor_operation_id: None,
            next_operation_id: "cut-a".to_owned(),
            operator_id: "cutter".to_owned(),
            start_tick: 0,
            finish_tick: 1,
        });
        assert!(has(
            &validate(&problem, &schedule),
            "UNEXPECTED_SETUP_RESERVATION"
        ));
    }

    #[test]
    fn precedence_is_checked_independently() {
        let (problem, mut schedule) = two_job_four_stage_case();
        let sew_a = schedule
            .assignments
            .iter_mut()
            .find(|a| a.operation_id == "sew-a")
            .unwrap();
        sew_a.start_tick = 2;
        sew_a.finish_tick = 7;
        assert!(has(&validate(&problem, &schedule), "PRECEDENCE_VIOLATION"));
    }

    #[test]
    fn unknown_resources_fail_closed() {
        let (problem, mut schedule) = two_job_four_stage_case();
        schedule.assignments[0].machine_id = "unregistered-machine".to_owned();
        schedule.assignments[0].operator_id = "unregistered-operator".to_owned();
        let report = validate(&problem, &schedule);
        assert!(has(&report, "UNKNOWN_MACHINE"));
        assert!(has(&report, "UNKNOWN_OPERATOR"));
    }
}
