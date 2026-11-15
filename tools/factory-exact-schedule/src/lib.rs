// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact tiny-instance multi-job scheduling oracle.
//!
//! The model has fixed machine assignments, ordered job routes, machine-exclusive
//! capacity, job release times and sequence-dependent machine setup transitions.
//! It is a benchmark oracle, not a production dispatch controller.

use std::collections::{BTreeSet, VecDeque};

pub const SCHEMA_VERSION: &str = "factory-jobshop-instance/0.1";
pub const MAX_EXACT_OPERATIONS: usize = 9;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SetupRule {
    pub from_family: &'static str,
    pub to_family: &'static str,
    pub ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Machine {
    pub id: &'static str,
    pub initial_family: &'static str,
    pub setup_rules: Vec<SetupRule>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub id: &'static str,
    pub machine_id: &'static str,
    pub family: &'static str,
    pub processing_ticks: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Job {
    pub id: &'static str,
    pub release_ticks: u64,
    pub operations: Vec<Operation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub schema_version: &'static str,
    pub instance_id: &'static str,
    pub machines: Vec<Machine>,
    pub jobs: Vec<Job>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InputFailure {
    UnsupportedSchema,
    EmptyInstanceId,
    EmptyMachineSet,
    EmptyJobSet,
    EmptyMachineId,
    DuplicateMachineId { machine_id: &'static str },
    EmptyInitialFamily { machine_id: &'static str },
    EmptyJobId,
    DuplicateJobId { job_id: &'static str },
    EmptyJobRoute { job_id: &'static str },
    EmptyOperationId,
    DuplicateOperationId { operation_id: &'static str },
    EmptyOperationFamily { operation_id: &'static str },
    ZeroProcessingTime { operation_id: &'static str },
    UnknownMachine { operation_id: &'static str, machine_id: &'static str },
    EmptySetupFamily { machine_id: &'static str },
    DuplicateSetupRule { machine_id: &'static str, from_family: &'static str, to_family: &'static str },
    MissingSetupRule { machine_id: &'static str, from_family: &'static str, to_family: &'static str },
    TooManyOperations { count: usize, maximum: usize },
    DurationEnvelopeOverflow,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ScheduledOperation {
    pub operation_id: &'static str,
    pub job_id: &'static str,
    pub machine_id: &'static str,
    pub sequence_index: usize,
    pub setup_start: u64,
    pub setup_ticks: u64,
    pub processing_start: u64,
    pub processing_end: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MachineOrder {
    pub machine_id: &'static str,
    pub operation_ids: Vec<&'static str>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Schedule {
    pub operations: Vec<ScheduledOperation>,
    pub machine_orders: Vec<MachineOrder>,
    pub makespan: u64,
    pub total_setup_ticks: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SearchStats {
    pub machine_order_combinations: u64,
    pub cyclic_combinations: u64,
    pub feasible_combinations: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OracleStatus {
    Exact,
    InvalidInput,
    NoFeasibleSchedule,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OracleResult {
    pub status: OracleStatus,
    pub optimality_proven: bool,
    pub input_failures: Vec<InputFailure>,
    pub optimal_schedule: Option<Schedule>,
    pub spt_schedule: Option<Schedule>,
    pub gap_ticks: Option<u64>,
    pub gap_parts_per_million: Option<u64>,
    pub stats: SearchStats,
}

#[derive(Debug, Clone)]
struct FlatOperation {
    id: &'static str,
    job_id: &'static str,
    machine_index: usize,
    machine_id: &'static str,
    family: &'static str,
    duration: u64,
    job_index: usize,
}

#[derive(Debug, Clone)]
struct EvaluatedCandidate {
    schedule: Schedule,
    machine_orders: Vec<Vec<usize>>,
}

fn setup_ticks(machine: &Machine, from: &str, to: &str) -> Option<u64> {
    machine.setup_rules.iter()
        .find(|rule| rule.from_family == from && rule.to_family == to)
        .map(|rule| rule.ticks)
}

fn validate(instance: &Instance) -> Vec<InputFailure> {
    let mut failures = Vec::new();
    if instance.schema_version != SCHEMA_VERSION {
        failures.push(InputFailure::UnsupportedSchema);
    }
    if instance.instance_id.trim().is_empty() {
        failures.push(InputFailure::EmptyInstanceId);
    }
    if instance.machines.is_empty() {
        failures.push(InputFailure::EmptyMachineSet);
    }
    if instance.jobs.is_empty() {
        failures.push(InputFailure::EmptyJobSet);
    }

    let mut machine_ids = BTreeSet::new();
    for machine in &instance.machines {
        if machine.id.trim().is_empty() {
            failures.push(InputFailure::EmptyMachineId);
        } else if !machine_ids.insert(machine.id) {
            failures.push(InputFailure::DuplicateMachineId { machine_id: machine.id });
        }
        if machine.initial_family.trim().is_empty() {
            failures.push(InputFailure::EmptyInitialFamily { machine_id: machine.id });
        }
        let mut rules = BTreeSet::new();
        for rule in &machine.setup_rules {
            if rule.from_family.trim().is_empty() || rule.to_family.trim().is_empty() {
                failures.push(InputFailure::EmptySetupFamily { machine_id: machine.id });
            }
            if !rules.insert((rule.from_family, rule.to_family)) {
                failures.push(InputFailure::DuplicateSetupRule {
                    machine_id: machine.id,
                    from_family: rule.from_family,
                    to_family: rule.to_family,
                });
            }
        }
    }

    let mut job_ids = BTreeSet::new();
    let mut operation_ids = BTreeSet::new();
    let mut operation_count = 0;
    for job in &instance.jobs {
        if job.id.trim().is_empty() {
            failures.push(InputFailure::EmptyJobId);
        } else if !job_ids.insert(job.id) {
            failures.push(InputFailure::DuplicateJobId { job_id: job.id });
        }
        if job.operations.is_empty() {
            failures.push(InputFailure::EmptyJobRoute { job_id: job.id });
        }
        for operation in &job.operations {
            operation_count += 1;
            if operation.id.trim().is_empty() {
                failures.push(InputFailure::EmptyOperationId);
            } else if !operation_ids.insert(operation.id) {
                failures.push(InputFailure::DuplicateOperationId { operation_id: operation.id });
            }
            if operation.family.trim().is_empty() {
                failures.push(InputFailure::EmptyOperationFamily { operation_id: operation.id });
            }
            if operation.processing_ticks == 0 {
                failures.push(InputFailure::ZeroProcessingTime { operation_id: operation.id });
            }
            if !instance.machines.iter().any(|machine| machine.id == operation.machine_id) {
                failures.push(InputFailure::UnknownMachine {
                    operation_id: operation.id,
                    machine_id: operation.machine_id,
                });
            }
        }
    }
    if operation_count > MAX_EXACT_OPERATIONS {
        failures.push(InputFailure::TooManyOperations {
            count: operation_count,
            maximum: MAX_EXACT_OPERATIONS,
        });
    }

    // A conservative upper bound keeps every serial SPT and exact schedule
    // timestamp representable; arithmetic overflow is an input error, not a
    // cyclic machine order or an unreported wraparound.
    let total_processing = instance.jobs.iter().flat_map(|job| job.operations.iter())
        .try_fold(0u64, |sum, operation| sum.checked_add(operation.processing_ticks));
    let mut total_max_setup = Some(0u64);
    for job in &instance.jobs {
        for operation in &job.operations {
            let maximum = instance.machines.iter().find(|machine| machine.id == operation.machine_id)
                .and_then(|machine| machine.setup_rules.iter().map(|rule| rule.ticks).max())
                .unwrap_or(0);
            total_max_setup = total_max_setup.and_then(|sum| sum.checked_add(maximum));
        }
    }
    let maximum_release = instance.jobs.iter().map(|job| job.release_ticks).max().unwrap_or(0);
    if total_processing
        .and_then(|processing| total_max_setup.and_then(|setup| processing.checked_add(setup)))
        .and_then(|duration| maximum_release.checked_add(duration))
        .is_none()
    {
        failures.push(InputFailure::DurationEnvelopeOverflow);
    }

    // Every permutation can place any operation family directly after any other
    // family on its machine; therefore all such transitions must be declared.
    for machine in &instance.machines {
        let families: Vec<&'static str> = instance.jobs.iter()
            .flat_map(|job| job.operations.iter())
            .filter(|operation| operation.machine_id == machine.id)
            .map(|operation| operation.family)
            .collect();
        if families.is_empty() {
            continue;
        }
        let mut unique_families = BTreeSet::new();
        for family in families {
            unique_families.insert(family);
        }
        for to in &unique_families {
            if setup_ticks(machine, machine.initial_family, to).is_none() {
                failures.push(InputFailure::MissingSetupRule {
                    machine_id: machine.id,
                    from_family: machine.initial_family,
                    to_family: to,
                });
            }
            for from in &unique_families {
                if setup_ticks(machine, from, to).is_none() {
                    failures.push(InputFailure::MissingSetupRule {
                        machine_id: machine.id,
                        from_family: from,
                        to_family: to,
                    });
                }
            }
        }
    }

    failures.sort_by_key(|failure| format!("{failure:?}"));
    failures.dedup();
    failures
}

fn flatten(instance: &Instance) -> (Vec<FlatOperation>, Vec<Vec<usize>>, Vec<Vec<usize>>) {
    let machine_indices: Vec<(&'static str, usize)> = instance.machines.iter()
        .enumerate().map(|(index, machine)| (machine.id, index)).collect();
    let mut flat = Vec::new();
    let mut jobs = Vec::with_capacity(instance.jobs.len());
    let mut machines = vec![Vec::new(); instance.machines.len()];

    for (job_index, job) in instance.jobs.iter().enumerate() {
        let mut job_ops = Vec::with_capacity(job.operations.len());
        for (job_position, operation) in job.operations.iter().enumerate() {
            let machine_index = machine_indices.iter()
                .find(|(id, _)| *id == operation.machine_id)
                .map(|(_, index)| *index)
                .unwrap_or(0);
            let flat_index = flat.len();
            flat.push(FlatOperation {
                id: operation.id,
                job_id: job.id,
                machine_index,
                machine_id: operation.machine_id,
                family: operation.family,
                duration: operation.processing_ticks,
                job_index,
            });
            job_ops.push(flat_index);
            machines[machine_index].push(flat_index);
        }
        jobs.push(job_ops);
    }
    (flat, jobs, machines)
}

fn enumerate_permutations(values: &[usize]) -> Vec<Vec<usize>> {
    fn visit(values: &[usize], used: &mut [bool], current: &mut Vec<usize>, output: &mut Vec<Vec<usize>>) {
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
    visit(values, &mut vec![false; values.len()], &mut Vec::with_capacity(values.len()), &mut output);
    output
}

fn evaluate_machine_orders(
    instance: &Instance,
    operations: &[FlatOperation],
    job_sequences: &[Vec<usize>],
    machine_orders: &[Vec<usize>],
) -> Option<Schedule> {
    let count = operations.len();
    let mut outgoing = vec![Vec::<(usize, u64)>::new(); count];
    let mut indegree = vec![0usize; count];
    let mut earliest = vec![0u64; count];
    let mut setup_for_operation = vec![0u64; count];
    let mut sequence_for_operation = vec![0usize; count];

    for (job_index, sequence) in job_sequences.iter().enumerate() {
        if let Some(first) = sequence.first() {
            earliest[*first] = instance.jobs[job_index].release_ticks;
        }
        for pair in sequence.windows(2) {
            let from = pair[0];
            let to = pair[1];
            add_edge(&mut outgoing, &mut indegree, from, to, operations[from].duration);
        }
    }

    let mut machine_orders_public = Vec::with_capacity(machine_orders.len());
    for (machine_index, sequence) in machine_orders.iter().enumerate() {
        let machine = &instance.machines[machine_index];
        let mut public_ids = Vec::with_capacity(sequence.len());
        let Some(first) = sequence.first() else {
            machine_orders_public.push(MachineOrder { machine_id: machine.id, operation_ids: public_ids });
            continue;
        };

        let first_operation = &operations[*first];
        let initial_setup = setup_ticks(machine, machine.initial_family, first_operation.family)?;
        earliest[*first] = earliest[*first].max(initial_setup);
        setup_for_operation[*first] = initial_setup;
        sequence_for_operation[*first] = 0;
        public_ids.push(first_operation.id);

        for (sequence_index, pair) in sequence.windows(2).enumerate() {
            let from = pair[0];
            let to = pair[1];
            let transition_setup = setup_ticks(machine, operations[from].family, operations[to].family)?;
            let edge_weight = operations[from].duration.checked_add(transition_setup)?;
            add_edge(&mut outgoing, &mut indegree, from, to, edge_weight);
            setup_for_operation[to] = transition_setup;
            sequence_for_operation[to] = sequence_index + 1;
            public_ids.push(operations[to].id);
        }
        machine_orders_public.push(MachineOrder { machine_id: machine.id, operation_ids: public_ids });
    }

    // Kahn topological traversal. A cycle means the chosen machine orders
    // contradict at least one job precedence chain and are infeasible.
    let mut ready = VecDeque::new();
    for (index, degree) in indegree.iter().enumerate() {
        if *degree == 0 {
            ready.push_back(index);
        }
    }
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
    if visited != count {
        return None;
    }

    let mut scheduled = Vec::with_capacity(count);
    let mut makespan = 0u64;
    let mut total_setup_ticks = 0u64;
    for (index, operation) in operations.iter().enumerate() {
        let processing_start = earliest[index];
        let processing_end = processing_start.checked_add(operation.duration)?;
        let setup = setup_for_operation[index];
        let setup_start = processing_start.checked_sub(setup)?;
        makespan = makespan.max(processing_end);
        total_setup_ticks = total_setup_ticks.checked_add(setup)?;
        scheduled.push(ScheduledOperation {
            operation_id: operation.id,
            job_id: operation.job_id,
            machine_id: operation.machine_id,
            sequence_index: sequence_for_operation[index],
            setup_start,
            setup_ticks: setup,
            processing_start,
            processing_end,
        });
    }
    scheduled.sort_by_key(|operation| (operation.job_id, operation.operation_id));

    Some(Schedule {
        operations: scheduled,
        machine_orders: machine_orders_public,
        makespan,
        total_setup_ticks,
    })
}

fn add_edge(
    outgoing: &mut [Vec<(usize, u64)>],
    indegree: &mut [usize],
    from: usize,
    to: usize,
    weight: u64,
) {
    outgoing[from].push((to, weight));
    indegree[to] += 1;
}

fn candidate_is_better(candidate: &EvaluatedCandidate, incumbent: &EvaluatedCandidate) -> bool {
    (
        candidate.schedule.makespan,
        candidate.schedule.total_setup_ticks,
        &candidate.machine_orders,
    ) < (
        incumbent.schedule.makespan,
        incumbent.schedule.total_setup_ticks,
        &incumbent.machine_orders,
    )
}

#[allow(clippy::too_many_arguments)]
fn enumerate_machine_order_products(
    instance: &Instance,
    operations: &[FlatOperation],
    job_sequences: &[Vec<usize>],
    permutations: &[Vec<Vec<usize>>],
    machine_index: usize,
    current: &mut Vec<Vec<usize>>,
    stats: &mut SearchStats,
    best: &mut Option<EvaluatedCandidate>,
) {
    if machine_index == permutations.len() {
        stats.machine_order_combinations += 1;
        if let Some(schedule) = evaluate_machine_orders(instance, operations, job_sequences, current) {
            stats.feasible_combinations += 1;
            let candidate = EvaluatedCandidate { schedule, machine_orders: current.clone() };
            if best.as_ref().is_none_or(|incumbent| candidate_is_better(&candidate, incumbent)) {
                *best = Some(candidate);
            }
        } else {
            stats.cyclic_combinations += 1;
        }
        return;
    }

    for order in &permutations[machine_index] {
        current.push(order.clone());
        enumerate_machine_order_products(
            instance, operations, job_sequences, permutations,
            machine_index + 1, current, stats, best,
        );
        current.pop();
    }
}

/// Exhaustively enumerate every machine-order permutation and prove the
/// minimum makespan for supported fixed-route instances of at most nine ops.
pub fn solve_exact(instance: &Instance) -> OracleResult {
    let input_failures = validate(instance);
    if !input_failures.is_empty() {
        return OracleResult {
            status: OracleStatus::InvalidInput,
            optimality_proven: false,
            input_failures,
            optimal_schedule: None,
            spt_schedule: None,
            gap_ticks: None,
            gap_parts_per_million: None,
            stats: SearchStats { machine_order_combinations: 0, cyclic_combinations: 0, feasible_combinations: 0 },
        };
    }

    let (operations, job_sequences, machine_operations) = flatten(instance);
    let permutations: Vec<Vec<Vec<usize>>> = machine_operations.iter()
        .map(|machine_ops| enumerate_permutations(machine_ops))
        .collect();
    let mut stats = SearchStats { machine_order_combinations: 0, cyclic_combinations: 0, feasible_combinations: 0 };
    let mut best = None;
    enumerate_machine_order_products(
        instance, &operations, &job_sequences, &permutations,
        0, &mut Vec::with_capacity(instance.machines.len()), &mut stats, &mut best,
    );

    let Some(optimal) = best.map(|candidate| candidate.schedule) else {
        return OracleResult {
            status: OracleStatus::NoFeasibleSchedule,
            optimality_proven: false,
            input_failures: Vec::new(),
            optimal_schedule: None,
            spt_schedule: None,
            gap_ticks: None,
            gap_parts_per_million: None,
            stats,
        };
    };

    let spt = solve_spt(instance, &operations, &job_sequences);
    let gap_ticks = spt.makespan.saturating_sub(optimal.makespan);
    let gap_parts_per_million = if optimal.makespan == 0 {
        None
    } else {
        Some(gap_ticks.saturating_mul(1_000_000) / optimal.makespan)
    };

    OracleResult {
        status: OracleStatus::Exact,
        optimality_proven: true,
        input_failures: Vec::new(),
        optimal_schedule: Some(optimal),
        spt_schedule: Some(spt),
        gap_ticks: Some(gap_ticks),
        gap_parts_per_million,
        stats,
    }
}

/// Deterministic shortest-processing-time dispatch baseline.
fn solve_spt(instance: &Instance, operations: &[FlatOperation], job_sequences: &[Vec<usize>]) -> Schedule {
    let mut next_position = vec![0usize; job_sequences.len()];
    let mut job_ready: Vec<u64> = instance.jobs.iter().map(|job| job.release_ticks).collect();
    let mut machine_ready = vec![0u64; instance.machines.len()];
    let mut last_family: Vec<&'static str> = instance.machines.iter()
        .map(|machine| machine.initial_family).collect();
    let mut machine_orders = vec![Vec::<usize>::new(); instance.machines.len()];
    let mut scheduled = Vec::with_capacity(operations.len());
    let mut total_setup_ticks = 0u64;
    let mut makespan = 0u64;

    while scheduled.len() < operations.len() {
        let selected = job_sequences.iter().enumerate()
            .filter_map(|(job_index, sequence)| {
                let position = next_position[job_index];
                sequence.get(position).map(|operation_index| *operation_index)
            })
            .min_by_key(|operation_index| {
                let operation = &operations[*operation_index];
                (operation.duration, operation.job_id, operation.id)
            })
            .expect("validated non-empty jobs must have a ready operation");

        let operation = &operations[selected];
        let machine = &instance.machines[operation.machine_index];
        let setup = setup_ticks(machine, last_family[operation.machine_index], operation.family)
            .expect("validated all machine family transitions");
        let operation_ready = job_ready[operation.job_index];
        let processing_start = operation_ready.max(machine_ready[operation.machine_index] + setup);
        let setup_start = processing_start - setup;
        let processing_end = processing_start + operation.duration;

        scheduled.push(ScheduledOperation {
            operation_id: operation.id,
            job_id: operation.job_id,
            machine_id: operation.machine_id,
            sequence_index: machine_orders[operation.machine_index].len(),
            setup_start,
            setup_ticks: setup,
            processing_start,
            processing_end,
        });
        machine_orders[operation.machine_index].push(selected);
        machine_ready[operation.machine_index] = processing_end;
        last_family[operation.machine_index] = operation.family;
        job_ready[operation.job_index] = processing_end;
        next_position[operation.job_index] += 1;
        total_setup_ticks += setup;
        makespan = makespan.max(processing_end);
    }

    scheduled.sort_by_key(|operation| (operation.job_id, operation.operation_id));
    let machine_orders = machine_orders.into_iter().enumerate().map(|(index, sequence)| MachineOrder {
        machine_id: instance.machines[index].id,
        operation_ids: sequence.iter().map(|operation_index| operations[*operation_index].id).collect(),
    }).collect();

    Schedule { operations: scheduled, machine_orders, makespan, total_setup_ticks }
}

pub fn verify_schedule(instance: &Instance, schedule: &Schedule) -> bool {
    let mut by_id = std::collections::BTreeMap::new();
    for operation in &schedule.operations {
        if by_id.insert(operation.operation_id, operation).is_some() {
            return false;
        }
        let Some(source) = instance.jobs.iter().flat_map(|job| job.operations.iter())
            .find(|source| source.id == operation.operation_id) else { return false; };
        let Some(expected_end) = operation.processing_start.checked_add(source.processing_ticks) else {
            return false;
        };
        if operation.processing_end != expected_end
            || operation.setup_start.checked_add(operation.setup_ticks) != Some(operation.processing_start)
            || operation.machine_id != source.machine_id
        {
            return false;
        }
    }

    let expected_count: usize = instance.jobs.iter().map(|job| job.operations.len()).sum();
    if by_id.len() != expected_count || schedule.machine_orders.len() != instance.machines.len() {
        return false;
    }

    for job in &instance.jobs {
        let mut previous_end = job.release_ticks;
        for source in &job.operations {
            let Some(scheduled) = by_id.get(source.id) else { return false; };
            if scheduled.processing_start < previous_end {
                return false;
            }
            previous_end = scheduled.processing_end;
        }
    }

    let mut ordered_ids = BTreeSet::new();
    let mut recomputed_setup = 0u64;
    for machine in &instance.machines {
        let mut matching_orders = schedule.machine_orders.iter()
            .filter(|order| order.machine_id == machine.id);
        let Some(order) = matching_orders.next() else { return false; };
        if matching_orders.next().is_some() {
            return false;
        }
        let mut previous_end = 0u64;
        let mut previous_family = machine.initial_family;
        for (sequence_index, operation_id) in order.operation_ids.iter().enumerate() {
            if !ordered_ids.insert(*operation_id) {
                return false;
            }
            let Some(scheduled) = by_id.get(operation_id) else { return false; };
            let Some(source) = instance.jobs.iter().flat_map(|job| job.operations.iter())
                .find(|source| source.id == *operation_id) else { return false; };
            if scheduled.machine_id != machine.id || scheduled.sequence_index != sequence_index {
                return false;
            }
            let Some(expected_setup) = setup_ticks(machine, previous_family, source.family) else {
                return false;
            };
            if expected_setup != scheduled.setup_ticks || scheduled.setup_start < previous_end {
                return false;
            }
            recomputed_setup = match recomputed_setup.checked_add(expected_setup) {
                Some(value) => value,
                None => return false,
            };
            previous_end = scheduled.processing_end;
            previous_family = source.family;
        }
    }

    if ordered_ids.len() != expected_count || recomputed_setup != schedule.total_setup_ticks {
        return false;
    }
    let recomputed_makespan = schedule.operations.iter()
        .map(|operation| operation.processing_end).max().unwrap_or(0);
    recomputed_makespan == schedule.makespan
}

pub fn reference_instance() -> Instance {
    let setup_rule = SetupRule { from_family: "F", to_family: "F", ticks: 0 };
    let machines = ["M0", "M1", "M2"].into_iter().map(|id| Machine {
        id,
        initial_family: "F",
        setup_rules: vec![setup_rule],
    }).collect();

    let job = |id, operations: Vec<Operation>| Job { id, release_ticks: 0, operations };
    let op = |id, machine_id, processing_ticks| Operation {
        id, machine_id, family: "F", processing_ticks,
    };

    Instance {
        schema_version: SCHEMA_VERSION,
        instance_id: "JSSP-3X3-SPT-GAP-001",
        machines,
        jobs: vec![
            job("JOB-A", vec![op("A-1", "M2", 9), op("A-2", "M0", 2), op("A-3", "M1", 10)]),
            job("JOB-B", vec![op("B-1", "M2", 7), op("B-2", "M1", 1), op("B-3", "M0", 1)]),
            job("JOB-C", vec![op("C-1", "M0", 2), op("C-2", "M2", 4), op("C-3", "M1", 4)]),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_reference_makespan_is_twenty_six() {
        let result = solve_exact(&reference_instance());
        assert_eq!(result.status, OracleStatus::Exact);
        assert!(result.optimality_proven);
        assert_eq!(result.optimal_schedule.as_ref().unwrap().makespan, 26);
        assert_eq!(result.stats.machine_order_combinations, 216);
        assert_eq!(result.stats.cyclic_combinations, 128);
        assert_eq!(result.stats.feasible_combinations, 88);
    }

    #[test]
    fn spt_baseline_is_feasible_but_eight_ticks_worse() {
        let result = solve_exact(&reference_instance());
        let exact = result.optimal_schedule.unwrap();
        let spt = result.spt_schedule.unwrap();
        assert_eq!(exact.makespan, 26);
        assert_eq!(spt.makespan, 34);
        assert_eq!(result.gap_ticks, Some(8));
        assert_eq!(result.gap_parts_per_million, Some(307_692));
        assert!(verify_schedule(&reference_instance(), &spt));
    }

    #[test]
    fn exact_certificate_satisfies_precedence_and_machine_capacity() {
        let instance = reference_instance();
        let result = solve_exact(&instance);
        let schedule = result.optimal_schedule.unwrap();
        assert!(verify_schedule(&instance, &schedule));
        assert_eq!(schedule.operations.len(), 9);
    }

    #[test]
    fn sequence_dependent_setup_is_included_in_exact_makespan() {
        let mut instance = reference_instance();
        for machine in &mut instance.machines {
            machine.initial_family = "A";
            machine.setup_rules = vec![
                SetupRule { from_family: "A", to_family: "A", ticks: 0 },
                SetupRule { from_family: "A", to_family: "B", ticks: 2 },
                SetupRule { from_family: "B", to_family: "A", ticks: 3 },
                SetupRule { from_family: "B", to_family: "B", ticks: 0 },
            ];
        }
        for job in &mut instance.jobs {
            for operation in &mut job.operations {
                operation.family = if operation.id.ends_with('1') { "B" } else { "A" };
            }
        }
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::Exact);
        assert!(result.optimal_schedule.unwrap().makespan >= 26);
    }

    #[test]
    fn release_time_is_respected() {
        let mut instance = reference_instance();
        instance.jobs[0].release_ticks = 50;
        let result = solve_exact(&instance);
        let schedule = result.optimal_schedule.unwrap();
        let first = schedule.operations.iter().find(|operation| operation.operation_id == "A-1").unwrap();
        assert!(first.processing_start >= 50);
        assert!(verify_schedule(&instance, &schedule));
    }

    #[test]
    fn release_plus_work_overflow_is_rejected_before_search() {
        let mut instance = reference_instance();
        instance.jobs[0].release_ticks = u64::MAX;
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(result.input_failures.contains(&InputFailure::DurationEnvelopeOverflow));
        assert!(!result.optimality_proven);
    }

    #[test]
    fn unsupported_instance_size_is_not_claimed_exact() {
        let mut instance = reference_instance();
        instance.jobs[0].operations.push(Operation {
            id: "A-4", machine_id: "M0", family: "F", processing_ticks: 1,
        });
        instance.machines[0].setup_rules.push(SetupRule { from_family: "F", to_family: "F", ticks: 0 });
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(!result.optimality_proven);
        assert!(result.input_failures.iter().any(|failure| matches!(failure, InputFailure::TooManyOperations { .. })));
    }

    #[test]
    fn duplicate_operation_identity_is_rejected() {
        let mut instance = reference_instance();
        instance.jobs[0].operations[0].id = "A-2";
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(result.input_failures.iter().any(|failure| matches!(failure, InputFailure::DuplicateOperationId { .. })));
    }

    #[test]
    fn unknown_machine_is_rejected() {
        let mut instance = reference_instance();
        instance.jobs[0].operations[0].machine_id = "MISSING";
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(result.input_failures.iter().any(|failure| matches!(failure, InputFailure::UnknownMachine { .. })));
    }

    #[test]
    fn missing_setup_transition_is_rejected() {
        let mut instance = reference_instance();
        instance.machines[0].setup_rules.clear();
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(result.input_failures.iter().any(|failure| matches!(failure, InputFailure::MissingSetupRule { .. })));
    }

    #[test]
    fn setup_time_changes_dispatch_schedule() {
        let mut instance = reference_instance();
        for machine in &mut instance.machines {
            machine.setup_rules = vec![
                SetupRule { from_family: "F", to_family: "F", ticks: 2 },
            ];
        }
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::Exact);
        assert!(result.optimal_schedule.unwrap().makespan > 26);
    }

    #[test]
    fn verifier_rejects_incomplete_machine_order_certificate() {
        let instance = reference_instance();
        let result = solve_exact(&instance);
        let mut schedule = result.optimal_schedule.unwrap();
        schedule.machine_orders[0].operation_ids.pop();
        assert!(!verify_schedule(&instance, &schedule));
    }

    #[test]
    fn verifier_rejects_timestamp_tampering() {
        let instance = reference_instance();
        let result = solve_exact(&instance);
        let mut schedule = result.optimal_schedule.unwrap();
        schedule.operations[0].processing_end += 1;
        assert!(!verify_schedule(&instance, &schedule));
    }

    #[test]
    fn deterministic_exact_result_is_reproducible() {
        assert_eq!(solve_exact(&reference_instance()), solve_exact(&reference_instance()));
    }

    #[test]
    fn invalid_schema_fails_closed() {
        let mut instance = reference_instance();
        instance.schema_version = "future/unsupported";
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(!result.optimality_proven);
    }

    #[test]
    fn duplicate_machine_identity_is_rejected() {
        let mut instance = reference_instance();
        instance.machines[1].id = "M0";
        let result = solve_exact(&instance);
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(result.input_failures.iter().any(|failure| matches!(failure, InputFailure::DuplicateMachineId { .. })));
    }

    #[test]
    fn no_job_can_start_before_release_time() {
        let mut instance = reference_instance();
        instance.jobs[1].release_ticks = 100;
        let result = solve_exact(&instance);
        let schedule = result.optimal_schedule.unwrap();
        let b1 = schedule.operations.iter().find(|operation| operation.operation_id == "B-1").unwrap();
        assert!(b1.processing_start >= 100);
        assert!(verify_schedule(&instance, &schedule));
    }

    #[test]
    fn operation_overflow_is_not_silently_wrapped() {
        let mut instance = reference_instance();
        instance.jobs[0].operations[0].processing_ticks = u64::MAX;
        let result = solve_exact(&instance);
        // The conservative duration envelope rejects the input before search;
        // timestamps are never allowed to wrap or masquerade as a cycle.
        assert_eq!(result.status, OracleStatus::InvalidInput);
        assert!(result.input_failures.contains(&InputFailure::DurationEnvelopeOverflow));
    }
}
