// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exhaustive exact makespan oracle for tiny FJSPLib flexible instances.
//!
//! Enumerates every eligible-machine assignment and every per-machine operation
//! order, rejects cyclic precedence combinations, and computes earliest starts
//! by longest-path propagation. It reports EXACT only when the complete search
//! finishes within the public bounds below.

use super::{FlexibleInstance, MAX_MACHINES, MAX_OPERATIONS};
use std::collections::{BTreeSet, VecDeque};

pub const MAX_EXACT_OPERATIONS: usize = 9;
pub const MAX_ASSIGNMENT_COMBINATIONS: u64 = 100_000;
pub const MAX_MACHINE_ORDER_COMBINATIONS: u64 = 500_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExactStatus {
    Exact,
    Inconclusive,
    InvalidInput,
    NoFeasibleSchedule,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InconclusiveReason {
    TooManyOperations,
    AssignmentSearchBoundExceeded,
    MachineOrderSearchBoundExceeded,
    ArithmeticOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct ScheduledOperation {
    pub operation_id: usize,
    pub machine_id: usize,
    pub processing_ticks: u64,
    pub sequence_index: usize,
    pub start_ticks: u64,
    pub end_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct MachineOrder {
    pub machine_id: usize,
    pub operation_ids: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct FlexibleSchedule {
    // Field order defines the deterministic tie-break order.
    pub makespan: u64,
    pub selected_machines: Vec<usize>,
    pub machine_orders: Vec<MachineOrder>,
    pub operations: Vec<ScheduledOperation>,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct SearchStats {
    pub assignment_combinations: u64,
    pub machine_order_combinations: u64,
    pub cyclic_combinations: u64,
    pub feasible_combinations: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactResult {
    pub status: ExactStatus,
    pub optimality_proven: bool,
    pub inconclusive_reason: Option<InconclusiveReason>,
    pub schedule: Option<FlexibleSchedule>,
    pub stats: SearchStats,
}

fn factorial(value: usize) -> Option<u64> {
    (2..=value).try_fold(1u64, |acc, factor| acc.checked_mul(factor as u64))
}

fn assignment_count(instance: &FlexibleInstance) -> Option<u64> {
    instance
        .operations
        .iter()
        .try_fold(1u64, |count, operation| {
            count.checked_mul(operation.alternatives.len() as u64)
        })
}

fn order_count_for_assignment(
    instance: &FlexibleInstance,
    selected_machines: &[usize],
) -> Option<u64> {
    let mut per_machine = vec![0usize; instance.machine_count];
    for machine_id in selected_machines {
        let count = per_machine.get_mut(*machine_id)?;
        *count += 1;
    }
    per_machine.into_iter().try_fold(1u64, |product, count| {
        product.checked_mul(factorial(count)?)
    })
}

fn preflight_order_count(
    instance: &FlexibleInstance,
    index: usize,
    selected_machines: &mut Vec<usize>,
    total: &mut u64,
) -> bool {
    if index == instance.operations.len() {
        let Some(count) = order_count_for_assignment(instance, selected_machines) else {
            return false;
        };
        let Some(next) = total.checked_add(count) else {
            return false;
        };
        *total = next;
        return *total <= MAX_MACHINE_ORDER_COMBINATIONS;
    }

    for alternative in &instance.operations[index].alternatives {
        selected_machines.push(alternative.machine_id);
        let within_bound = preflight_order_count(instance, index + 1, selected_machines, total);
        selected_machines.pop();
        if !within_bound {
            return false;
        }
    }
    true
}

fn permutations(values: &[usize]) -> Vec<Vec<usize>> {
    fn visit(
        values: &[usize],
        used: &mut [bool],
        current: &mut Vec<usize>,
        output: &mut Vec<Vec<usize>>,
    ) {
        if current.len() == values.len() {
            output.push(current.clone());
            return;
        }
        for index in 0..values.len() {
            if !used[index] {
                used[index] = true;
                current.push(values[index]);
                visit(values, used, current, output);
                current.pop();
                used[index] = false;
            }
        }
    }

    let mut output = Vec::new();
    visit(
        values,
        &mut vec![false; values.len()],
        &mut Vec::with_capacity(values.len()),
        &mut output,
    );
    output
}

fn evaluate_machine_orders(
    instance: &FlexibleInstance,
    selected_machines: &[usize],
    machine_orders: &[Vec<usize>],
) -> Option<FlexibleSchedule> {
    let operation_count = instance.operations.len();
    let durations: Vec<u64> = instance
        .operations
        .iter()
        .enumerate()
        .map(|(index, operation)| {
            let machine_id = selected_machines[index];
            operation
                .alternatives
                .iter()
                .find(|alternative| alternative.machine_id == machine_id)
                .map(|alternative| alternative.processing_ticks)
        })
        .collect::<Option<Vec<_>>>()?;

    let mut outgoing = vec![Vec::<(usize, u64)>::new(); operation_count];
    let mut indegree = vec![0usize; operation_count];
    let mut edges = BTreeSet::new();

    // Job / assembly precedence edges.
    for edge in &instance.precedences {
        if edges.insert((edge.before, edge.after)) {
            outgoing[edge.before].push((edge.after, durations[edge.before]));
            indegree[edge.after] += 1;
        }
    }

    // Unary machine-capacity edges. Each order is a disjunctive choice.
    for order in machine_orders {
        for pair in order.windows(2) {
            let from = pair[0];
            let to = pair[1];
            if edges.insert((from, to)) {
                outgoing[from].push((to, durations[from]));
                indegree[to] += 1;
            }
        }
    }

    let mut ready = VecDeque::new();
    for (operation, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(operation);
        }
    }

    let mut earliest = vec![0u64; operation_count];
    let mut visited = 0usize;
    while let Some(from) = ready.pop_front() {
        visited += 1;
        for (to, weight) in &outgoing[from] {
            earliest[*to] = earliest[*to].max(earliest[from].checked_add(*weight)?);
            indegree[*to] -= 1;
            if indegree[*to] == 0 {
                ready.push_back(*to);
            }
        }
    }
    if visited != operation_count {
        return None;
    }

    let mut scheduled = Vec::with_capacity(operation_count);
    let mut makespan = 0u64;
    for operation_id in 0..instance.operations.len() {
        let start_ticks = earliest[operation_id];
        let end_ticks = start_ticks.checked_add(durations[operation_id])?;
        makespan = makespan.max(end_ticks);
        let machine_id = selected_machines[operation_id];
        let sequence_index = machine_orders[machine_id]
            .iter()
            .position(|candidate| *candidate == operation_id)?;
        scheduled.push(ScheduledOperation {
            operation_id,
            machine_id,
            processing_ticks: durations[operation_id],
            sequence_index,
            start_ticks,
            end_ticks,
        });
    }

    let public_orders = machine_orders
        .iter()
        .enumerate()
        .map(|(machine_id, order)| MachineOrder {
            machine_id,
            operation_ids: order.clone(),
        })
        .collect();

    Some(FlexibleSchedule {
        makespan,
        selected_machines: selected_machines.to_vec(),
        machine_orders: public_orders,
        operations: scheduled,
    })
}

fn enumerate_machine_orders(
    instance: &FlexibleInstance,
    selected_machines: &[usize],
    machine_permutations: &[Vec<Vec<usize>>],
    machine_index: usize,
    current: &mut Vec<Vec<usize>>,
    stats: &mut SearchStats,
    best: &mut Option<FlexibleSchedule>,
) -> bool {
    if machine_index == machine_permutations.len() {
        stats.machine_order_combinations += 1;
        let Some(schedule) = evaluate_machine_orders(instance, selected_machines, current) else {
            stats.cyclic_combinations += 1;
            return true;
        };
        stats.feasible_combinations += 1;
        if best.as_ref().is_none_or(|incumbent| schedule < *incumbent) {
            *best = Some(schedule);
        }
        return true;
    }

    for order in &machine_permutations[machine_index] {
        current.push(order.clone());
        let complete = enumerate_machine_orders(
            instance,
            selected_machines,
            machine_permutations,
            machine_index + 1,
            current,
            stats,
            best,
        );
        current.pop();
        if !complete {
            return false;
        }
    }
    true
}

fn enumerate_assignments(
    instance: &FlexibleInstance,
    operation_index: usize,
    selected_machines: &mut Vec<usize>,
    stats: &mut SearchStats,
    best: &mut Option<FlexibleSchedule>,
) -> bool {
    if operation_index == instance.operations.len() {
        stats.assignment_combinations += 1;
        let Some(order_count) = order_count_for_assignment(instance, selected_machines) else {
            return false;
        };
        if stats
            .machine_order_combinations
            .checked_add(order_count)
            .is_none_or(|total| total > MAX_MACHINE_ORDER_COMBINATIONS)
        {
            return false;
        }

        let mut machine_operations = vec![Vec::<usize>::new(); instance.machine_count];
        for (operation_id, machine_id) in selected_machines.iter().enumerate() {
            machine_operations[*machine_id].push(operation_id);
        }
        let machine_permutations: Vec<Vec<Vec<usize>>> = machine_operations
            .iter()
            .map(|operations| permutations(operations))
            .collect();

        return enumerate_machine_orders(
            instance,
            selected_machines,
            &machine_permutations,
            0,
            &mut Vec::with_capacity(instance.machine_count),
            stats,
            best,
        );
    }

    for alternative in &instance.operations[operation_index].alternatives {
        selected_machines.push(alternative.machine_id);
        let complete = enumerate_assignments(
            instance,
            operation_index + 1,
            selected_machines,
            stats,
            best,
        );
        selected_machines.pop();
        if !complete {
            return false;
        }
    }
    true
}

fn instance_is_valid(instance: &FlexibleInstance) -> bool {
    if instance.machine_count == 0
        || instance.machine_count > MAX_MACHINES
        || instance.operations.is_empty()
        || instance.operations.len() > MAX_OPERATIONS
    {
        return false;
    }

    for (expected_id, operation) in instance.operations.iter().enumerate() {
        if operation.id != expected_id || operation.alternatives.is_empty() {
            return false;
        }
        let mut machines = BTreeSet::new();
        for alternative in &operation.alternatives {
            if alternative.machine_id >= instance.machine_count
                || alternative.processing_ticks == 0
                || !machines.insert(alternative.machine_id)
            {
                return false;
            }
        }
    }

    let operation_count = instance.operations.len();
    let mut outgoing = vec![Vec::<usize>::new(); operation_count];
    let mut indegree = vec![0usize; operation_count];
    let mut edges = BTreeSet::new();
    for edge in &instance.precedences {
        if edge.before >= operation_count
            || edge.after >= operation_count
            || edge.before == edge.after
            || !edges.insert((edge.before, edge.after))
        {
            return false;
        }
        outgoing[edge.before].push(edge.after);
        indegree[edge.after] += 1;
    }

    let mut ready = VecDeque::new();
    for (operation_id, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(operation_id);
        }
    }
    let mut visited = 0usize;
    while let Some(operation_id) = ready.pop_front() {
        visited += 1;
        for successor in &outgoing[operation_id] {
            indegree[*successor] -= 1;
            if indegree[*successor] == 0 {
                ready.push_back(*successor);
            }
        }
    }
    visited == operation_count
}

fn duration_envelope_fits(instance: &FlexibleInstance) -> bool {
    instance
        .operations
        .iter()
        .try_fold(0u64, |sum, operation| {
            let maximum_duration = operation
                .alternatives
                .iter()
                .map(|alternative| alternative.processing_ticks)
                .max()?;
            sum.checked_add(maximum_duration)
        })
        .is_some()
}

/// Solve a tiny FJSP/FAJSP makespan instance by exhaustive machine-assignment
/// and machine-order enumeration. A search exceeding its declared envelope is
/// INCONCLUSIVE, never silently truncated and never reported as optimal.
pub fn solve_exact(instance: &FlexibleInstance) -> ExactResult {
    if !instance_is_valid(instance) {
        return ExactResult {
            status: ExactStatus::InvalidInput,
            optimality_proven: false,
            inconclusive_reason: None,
            schedule: None,
            stats: SearchStats::default(),
        };
    }
    if !duration_envelope_fits(instance) {
        return ExactResult {
            status: ExactStatus::Inconclusive,
            optimality_proven: false,
            inconclusive_reason: Some(InconclusiveReason::ArithmeticOverflow),
            schedule: None,
            stats: SearchStats::default(),
        };
    }
    if instance.operations.len() > MAX_EXACT_OPERATIONS {
        return ExactResult {
            status: ExactStatus::Inconclusive,
            optimality_proven: false,
            inconclusive_reason: Some(InconclusiveReason::TooManyOperations),
            schedule: None,
            stats: SearchStats::default(),
        };
    }

    let Some(assignments) = assignment_count(instance) else {
        return ExactResult {
            status: ExactStatus::Inconclusive,
            optimality_proven: false,
            inconclusive_reason: Some(InconclusiveReason::AssignmentSearchBoundExceeded),
            schedule: None,
            stats: SearchStats::default(),
        };
    };
    if assignments > MAX_ASSIGNMENT_COMBINATIONS {
        return ExactResult {
            status: ExactStatus::Inconclusive,
            optimality_proven: false,
            inconclusive_reason: Some(InconclusiveReason::AssignmentSearchBoundExceeded),
            schedule: None,
            stats: SearchStats::default(),
        };
    }

    let mut total_order_combinations = 0u64;
    if !preflight_order_count(
        instance,
        0,
        &mut Vec::with_capacity(instance.operations.len()),
        &mut total_order_combinations,
    ) {
        return ExactResult {
            status: ExactStatus::Inconclusive,
            optimality_proven: false,
            inconclusive_reason: Some(InconclusiveReason::MachineOrderSearchBoundExceeded),
            schedule: None,
            stats: SearchStats::default(),
        };
    }

    let mut stats = SearchStats::default();
    let mut best = None;
    let completed = enumerate_assignments(
        instance,
        0,
        &mut Vec::with_capacity(instance.operations.len()),
        &mut stats,
        &mut best,
    );
    if !completed {
        return ExactResult {
            status: ExactStatus::Inconclusive,
            optimality_proven: false,
            inconclusive_reason: Some(InconclusiveReason::ArithmeticOverflow),
            schedule: None,
            stats,
        };
    }

    match best {
        Some(schedule) => ExactResult {
            status: ExactStatus::Exact,
            optimality_proven: true,
            inconclusive_reason: None,
            schedule: Some(schedule),
            stats,
        },
        None => ExactResult {
            status: ExactStatus::NoFeasibleSchedule,
            optimality_proven: false,
            inconclusive_reason: None,
            schedule: None,
            stats,
        },
    }
}

/// Independent certificate verifier: checks complete assignment, eligible
/// machine membership, precedence, machine order/no-overlap, timestamps and
/// makespan against the supplied normalized instance.
pub fn verify_schedule(instance: &FlexibleInstance, schedule: &FlexibleSchedule) -> bool {
    if !instance_is_valid(instance) {
        return false;
    }
    if schedule.operations.len() != instance.operations.len()
        || schedule.selected_machines.len() != instance.operations.len()
        || schedule.machine_orders.len() != instance.machine_count
    {
        return false;
    }

    let mut by_id = vec![None; instance.operations.len()];
    for scheduled in &schedule.operations {
        if scheduled.operation_id >= by_id.len()
            || by_id[scheduled.operation_id].replace(scheduled).is_some()
        {
            return false;
        }
        if scheduled.machine_id != schedule.selected_machines[scheduled.operation_id] {
            return false;
        }
        if !instance.operations[scheduled.operation_id]
            .alternatives
            .iter()
            .any(|alternative| {
                alternative.machine_id == scheduled.machine_id
                    && alternative.processing_ticks == scheduled.processing_ticks
            })
        {
            return false;
        }
        if scheduled
            .start_ticks
            .checked_add(scheduled.processing_ticks)
            != Some(scheduled.end_ticks)
        {
            return false;
        }
    }

    for edge in &instance.precedences {
        let (Some(before), Some(after)) = (
            by_id.get(edge.before).and_then(|value| *value),
            by_id.get(edge.after).and_then(|value| *value),
        ) else {
            return false;
        };
        if before.end_ticks > after.start_ticks {
            return false;
        }
    }

    let mut seen = BTreeSet::new();
    for (machine_id, order) in schedule.machine_orders.iter().enumerate() {
        if order.machine_id != machine_id {
            return false;
        }
        let mut previous_end = 0u64;
        for (sequence_index, operation_id) in order.operation_ids.iter().enumerate() {
            if !seen.insert(*operation_id) {
                return false;
            }
            let Some(operation) = by_id.get(*operation_id).and_then(|value| *value) else {
                return false;
            };
            if operation.machine_id != machine_id
                || operation.sequence_index != sequence_index
                || operation.start_ticks < previous_end
            {
                return false;
            }
            previous_end = operation.end_ticks;
        }
    }
    if seen.len() != instance.operations.len() {
        return false;
    }

    let makespan = by_id
        .iter()
        .filter_map(|operation| operation.map(|op| op.end_ticks))
        .max()
        .unwrap_or(0);
    makespan == schedule.makespan
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{parse_fajsp, parse_fjsp};

    const FJSP_FATTAHI1: &str = include_str!("../fixtures/fattahi1.fjsp");

    const FAJSP_FATTAHI1: &str = include_str!("../fixtures/fattahi1.fajsp");

    #[test]
    fn fattahi1_fjsp_exact_optimum_and_search_accounting() {
        let instance = parse_fjsp("fattahi1", Some("fattahi"), FJSP_FATTAHI1).unwrap();
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::Exact);
        assert!(result.optimality_proven);
        let schedule = result.schedule.unwrap();
        assert_eq!(schedule.makespan, 66);
        assert_eq!(schedule.selected_machines, vec![1, 1, 0, 0]);
        assert_eq!(result.stats.assignment_combinations, 16);
        assert_eq!(result.stats.machine_order_combinations, 120);
        assert_eq!(result.stats.cyclic_combinations, 68);
        assert_eq!(result.stats.feasible_combinations, 52);
        assert!(verify_schedule(&instance, &schedule));
    }

    #[test]
    fn equivalent_fajsp_precedence_graph_has_same_exact_makespan() {
        let instance = parse_fajsp("fattahi1", Some("fattahi"), FAJSP_FATTAHI1).unwrap();
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::Exact);
        assert_eq!(result.schedule.unwrap().makespan, 66);
        assert_eq!(result.stats.assignment_combinations, 16);
        assert_eq!(result.stats.machine_order_combinations, 120);
    }

    #[test]
    fn assignment_bound_returns_inconclusive_before_search() {
        let mut row = String::from("9");
        for _ in 0..9 {
            row.push_str(" 10");
            for machine in 1..=10 {
                row.push_str(&format!(" {machine} 1"));
            }
        }
        let input = format!("1 10 10\n{row}\n");
        let instance = parse_fjsp("assignment-bound", None, &input).unwrap();
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::Inconclusive);
        assert_eq!(
            result.inconclusive_reason,
            Some(InconclusiveReason::AssignmentSearchBoundExceeded)
        );
        assert_eq!(result.stats.assignment_combinations, 0);
    }

    #[test]
    fn machine_order_bound_returns_inconclusive_before_partial_search() {
        let mut row = String::from("9");
        for _ in 0..9 {
            row.push_str(" 2 1 1 2 1");
        }
        let input = format!("1 2 2\n{row}\n");
        let instance = parse_fjsp("order-bound", None, &input).unwrap();
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::Inconclusive);
        assert_eq!(
            result.inconclusive_reason,
            Some(InconclusiveReason::MachineOrderSearchBoundExceeded)
        );
        assert_eq!(result.stats.assignment_combinations, 0);
        assert_eq!(result.stats.machine_order_combinations, 0);
    }

    #[test]
    fn exact_solver_returns_inconclusive_before_exceeding_operation_bound() {
        let mut instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        while instance.operations.len() <= MAX_EXACT_OPERATIONS {
            let mut extra_operation = instance.operations[0].clone();
            extra_operation.id = instance.operations.len();
            instance.operations.push(extra_operation);
        }
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::Inconclusive);
        assert!(!result.optimality_proven);
        assert_eq!(
            result.inconclusive_reason,
            Some(InconclusiveReason::TooManyOperations)
        );
        assert!(result.schedule.is_none());
    }

    #[test]
    fn duration_envelope_overflow_is_inconclusive_not_no_feasible() {
        let mut instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        instance.operations[0].alternatives[0].processing_ticks = u64::MAX;
        instance.operations[1].alternatives[0].processing_ticks = u64::MAX;
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::Inconclusive);
        assert!(!result.optimality_proven);
        assert_eq!(
            result.inconclusive_reason,
            Some(InconclusiveReason::ArithmeticOverflow)
        );
    }

    #[test]
    fn invalid_precedence_certificate_is_rejected_without_panicking() {
        let valid_instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        let result = solve_exact(&valid_instance);
        let schedule = result.schedule.unwrap();
        let mut invalid_instance = valid_instance;
        invalid_instance.precedences[0].after = usize::MAX;
        assert!(!verify_schedule(&invalid_instance, &schedule));
    }

    #[test]
    fn malformed_precedence_input_is_rejected_before_search() {
        let mut instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        instance.precedences[0].after = usize::MAX;
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::InvalidInput);
        assert!(!result.optimality_proven);
        assert_eq!(result.stats, SearchStats::default());
        assert!(result.schedule.is_none());
    }

    #[test]
    fn precedence_cycle_is_invalid_input_not_a_search_result() {
        let mut instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        instance.precedences.push(crate::Precedence {
            before: 1,
            after: 0,
            job_id: Some(0),
        });
        let result = solve_exact(&instance);
        assert_eq!(result.status, ExactStatus::InvalidInput);
        assert_eq!(result.stats, SearchStats::default());
    }

    #[test]
    fn certificate_verifier_rejects_empty_or_oversized_instance() {
        let empty = FlexibleInstance {
            parser_version: crate::PARSER_VERSION,
            source: crate::FJSPLIB_SOURCE,
            source_format: crate::SourceFormat::Fjsp,
            instance_id: "empty".to_owned(),
            source_family: None,
            machine_count: 1,
            declared_job_count: Some(0),
            average_flexibility: None,
            operations: Vec::new(),
            precedences: Vec::new(),
            canonical_fingerprint_fnv1a64: "synthetic".to_owned(),
        };
        let schedule = FlexibleSchedule {
            makespan: 0,
            selected_machines: Vec::new(),
            machine_orders: vec![MachineOrder {
                machine_id: 0,
                operation_ids: Vec::new(),
            }],
            operations: Vec::new(),
        };
        assert!(!verify_schedule(&empty, &schedule));
    }

    #[test]
    fn exact_result_is_deterministic_across_repeated_runs() {
        let instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        assert_eq!(solve_exact(&instance), solve_exact(&instance));
    }

    #[test]
    fn alternative_input_order_does_not_change_optimum_or_tie_break() {
        let mut instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        let baseline = solve_exact(&instance);
        for operation in &mut instance.operations {
            operation.alternatives.reverse();
        }
        let reordered = solve_exact(&instance);
        assert_eq!(baseline.status, reordered.status);
        assert_eq!(baseline.schedule, reordered.schedule);
        assert_eq!(baseline.stats, reordered.stats);
    }

    #[test]
    fn certificate_verifier_rejects_machine_overlap_without_job_precedence() {
        let input = "3 1 1
1 1 1 5
1 1 1 5
1 1 1 5
";
        let instance = parse_fjsp("independent-jobs", None, input).unwrap();
        let mut schedule = solve_exact(&instance).schedule.unwrap();
        assert!(verify_schedule(&instance, &schedule));
        schedule.operations[1].start_ticks = 0;
        schedule.operations[1].end_ticks = 5;
        assert!(!verify_schedule(&instance, &schedule));
    }

    #[test]
    fn schedule_certificate_tampering_is_rejected() {
        let instance = parse_fjsp("fattahi1", None, FJSP_FATTAHI1).unwrap();
        let result = solve_exact(&instance);
        let mut schedule = result.schedule.unwrap();
        schedule.operations[0].end_ticks += 1;
        assert!(!verify_schedule(&instance, &schedule));
    }
}
