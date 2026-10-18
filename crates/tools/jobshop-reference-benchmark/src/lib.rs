//! Pinned classic job-shop instances and a bounded exhaustive reference oracle.
//!
//! The parser accepts the zero-based machine/duration-pair format in the pinned
//! fixtures. The exact solver is intentionally limited to tiny instances. This
//! crate is a benchmark qualification tool, not a production scheduler.

#![forbid(unsafe_code)]

use std::fmt;

pub const MAX_INSTANCE_OPERATIONS: usize = 100_000;
pub const MAX_EXACT_OPERATIONS: usize = 12;
pub const MAX_SEARCH_NODES: u64 = 2_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Operation {
    pub machine: usize,
    pub duration: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Instance {
    pub machine_count: usize,
    pub jobs: Vec<Vec<Operation>>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Summary {
    pub job_count: usize,
    pub machine_count: usize,
    pub operation_count: usize,
    pub total_processing_time: u64,
    pub maximum_job_work: u64,
    pub maximum_machine_load: u64,
    pub lower_bound_makespan: u64,
    pub machine_loads: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExactResult {
    pub makespan: u64,
    pub lower_bound_makespan: u64,
    pub expanded_nodes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BenchmarkError {
    InvalidInteger {
        token_index: usize,
        value: String,
    },
    InvalidDimensions,
    DimensionsTooLarge,
    UnexpectedTokenCount {
        expected: usize,
        actual: usize,
    },
    MachineOutOfRange {
        job: usize,
        operation: usize,
        machine: usize,
        machine_count: usize,
    },
    ZeroDuration {
        job: usize,
        operation: usize,
    },
    InvalidInstance(&'static str),
    ArithmeticOverflow,
    ExactOperationLimit {
        count: usize,
        maximum: usize,
    },
    SearchNodeLimit {
        maximum: u64,
    },
    NoSchedule,
}

impl fmt::Display for BenchmarkError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInteger { token_index, value } => {
                write!(f, "token {token_index} is not a valid nonnegative integer: {value:?}")
            }
            Self::InvalidDimensions => write!(f, "job and machine counts must both be positive"),
            Self::DimensionsTooLarge => write!(
                f,
                "instance exceeds the configured limit of {MAX_INSTANCE_OPERATIONS} operations"
            ),
            Self::UnexpectedTokenCount { expected, actual } => write!(
                f,
                "classic-pairs format needs exactly {expected} integer tokens; found {actual}"
            ),
            Self::MachineOutOfRange {
                job,
                operation,
                machine,
                machine_count,
            } => write!(
                f,
                "job {job}, operation {operation} references machine {machine}; valid IDs are 0..{machine_count}"
            ),
            Self::ZeroDuration { job, operation } => {
                write!(f, "job {job}, operation {operation} has zero duration")
            }
            Self::InvalidInstance(message) => write!(f, "invalid instance: {message}"),
            Self::ArithmeticOverflow => write!(f, "integer scheduling arithmetic overflow"),
            Self::ExactOperationLimit { count, maximum } => write!(
                f,
                "exact oracle supports at most {maximum} operations; instance has {count}"
            ),
            Self::SearchNodeLimit { maximum } => write!(
                f,
                "exact search exceeded its {maximum}-node budget; no optimality claim emitted"
            ),
            Self::NoSchedule => write!(f, "no feasible schedule found in the bounded search"),
        }
    }
}

impl std::error::Error for BenchmarkError {}

/// Parse the pinned repository's classic format:
/// job_count machine_count, followed by one row per job containing
/// machine_id duration pairs. Machine IDs are zero-based and durations must be
/// positive integers. Blank lines and extra whitespace are accepted; extra
/// tokens, comments, and malformed rows fail closed.
pub fn parse_classic_pair_format(input: &str) -> Result<Instance, BenchmarkError> {
    let tokens: Vec<&str> = input.split_whitespace().collect();
    if tokens.len() < 2 {
        return Err(BenchmarkError::UnexpectedTokenCount {
            expected: 2,
            actual: tokens.len(),
        });
    }

    let job_count = parse_usize(tokens[0], 1)?;
    let machine_count = parse_usize(tokens[1], 2)?;
    if job_count == 0 || machine_count == 0 {
        return Err(BenchmarkError::InvalidDimensions);
    }

    let operation_count = job_count
        .checked_mul(machine_count)
        .ok_or(BenchmarkError::DimensionsTooLarge)?;
    if operation_count > MAX_INSTANCE_OPERATIONS {
        return Err(BenchmarkError::DimensionsTooLarge);
    }
    let expected_tokens = operation_count
        .checked_mul(2)
        .and_then(|value| value.checked_add(2))
        .ok_or(BenchmarkError::DimensionsTooLarge)?;
    if tokens.len() != expected_tokens {
        return Err(BenchmarkError::UnexpectedTokenCount {
            expected: expected_tokens,
            actual: tokens.len(),
        });
    }

    let mut cursor = 2;
    let mut jobs = Vec::with_capacity(job_count);
    for job_index in 0..job_count {
        let mut job = Vec::with_capacity(machine_count);
        for operation_index in 0..machine_count {
            let machine = parse_usize(tokens[cursor], cursor + 1)?;
            let duration = tokens[cursor + 1]
                .parse::<u64>()
                .map_err(|_| BenchmarkError::InvalidInteger {
                    token_index: cursor + 2,
                    value: tokens[cursor + 1].to_owned(),
                })?;
            if machine >= machine_count {
                return Err(BenchmarkError::MachineOutOfRange {
                    job: job_index,
                    operation: operation_index,
                    machine,
                    machine_count,
                });
            }
            if duration == 0 {
                return Err(BenchmarkError::ZeroDuration {
                    job: job_index,
                    operation: operation_index,
                });
            }
            job.push(Operation { machine, duration });
            cursor += 2;
        }
        jobs.push(job);
    }

    let instance = Instance {
        machine_count,
        jobs,
    };
    summarize(&instance)?;
    Ok(instance)
}

fn parse_usize(value: &str, token_index: usize) -> Result<usize, BenchmarkError> {
    value
        .parse::<usize>()
        .map_err(|_| BenchmarkError::InvalidInteger {
            token_index,
            value: value.to_owned(),
        })
}

/// Compute a workload lower bound and per-machine loads using checked integer
/// arithmetic. The bound is max(longest job workload, busiest machine load).
pub fn summarize(instance: &Instance) -> Result<Summary, BenchmarkError> {
    if instance.machine_count == 0 || instance.jobs.is_empty() {
        return Err(BenchmarkError::InvalidInstance(
            "job and machine counts must both be positive",
        ));
    }
    if instance.machine_count > MAX_INSTANCE_OPERATIONS
        || instance.jobs.len() > MAX_INSTANCE_OPERATIONS
    {
        return Err(BenchmarkError::DimensionsTooLarge);
    }

    let mut machine_loads = vec![0_u64; instance.machine_count];
    let mut total_processing_time = 0_u64;
    let mut maximum_job_work = 0_u64;
    let mut operation_count = 0_usize;

    for (job_index, job) in instance.jobs.iter().enumerate() {
        if job.is_empty() {
            return Err(BenchmarkError::InvalidInstance("jobs may not be empty"));
        }
        let mut job_work = 0_u64;
        for (operation_index, operation) in job.iter().enumerate() {
            if operation.machine >= instance.machine_count {
                return Err(BenchmarkError::MachineOutOfRange {
                    job: job_index,
                    operation: operation_index,
                    machine: operation.machine,
                    machine_count: instance.machine_count,
                });
            }
            if operation.duration == 0 {
                return Err(BenchmarkError::ZeroDuration {
                    job: job_index,
                    operation: operation_index,
                });
            }
            job_work = job_work
                .checked_add(operation.duration)
                .ok_or(BenchmarkError::ArithmeticOverflow)?;
            machine_loads[operation.machine] = machine_loads[operation.machine]
                .checked_add(operation.duration)
                .ok_or(BenchmarkError::ArithmeticOverflow)?;
            operation_count = operation_count
                .checked_add(1)
                .ok_or(BenchmarkError::ArithmeticOverflow)?;
            if operation_count > MAX_INSTANCE_OPERATIONS {
                return Err(BenchmarkError::DimensionsTooLarge);
            }
        }
        total_processing_time = total_processing_time
            .checked_add(job_work)
            .ok_or(BenchmarkError::ArithmeticOverflow)?;
        maximum_job_work = maximum_job_work.max(job_work);
    }

    let maximum_machine_load = machine_loads.iter().copied().max().unwrap_or(0);
    Ok(Summary {
        job_count: instance.jobs.len(),
        machine_count: instance.machine_count,
        operation_count,
        total_processing_time,
        maximum_job_work,
        maximum_machine_load,
        lower_bound_makespan: maximum_job_work.max(maximum_machine_load),
        machine_loads,
    })
}

/// Exhaustively search the bounded serial schedule space for the minimum
/// makespan. Each candidate operation starts at max(job-ready, machine-ready),
/// so the serial ordering adds no gratuitous delay. Instances above the
/// operation/node bounds return an error rather than a heuristic result.
pub fn solve_exact(instance: &Instance) -> Result<ExactResult, BenchmarkError> {
    let summary = summarize(instance)?;
    if summary.operation_count > MAX_EXACT_OPERATIONS {
        return Err(BenchmarkError::ExactOperationLimit {
            count: summary.operation_count,
            maximum: MAX_EXACT_OPERATIONS,
        });
    }

    let mut search = Search {
        instance,
        total_operations: summary.operation_count,
        expanded_nodes: 0,
        best_makespan: None,
    };
    let mut next_operation = vec![0_usize; instance.jobs.len()];
    let mut job_ready = vec![0_u64; instance.jobs.len()];
    let mut machine_ready = vec![0_u64; instance.machine_count];
    search.descend(
        &mut next_operation,
        &mut job_ready,
        &mut machine_ready,
        0,
        0,
    )?;

    let makespan = search.best_makespan.ok_or(BenchmarkError::NoSchedule)?;
    Ok(ExactResult {
        makespan,
        lower_bound_makespan: summary.lower_bound_makespan,
        expanded_nodes: search.expanded_nodes,
    })
}

struct Search<'a> {
    instance: &'a Instance,
    total_operations: usize,
    expanded_nodes: u64,
    best_makespan: Option<u64>,
}

impl Search<'_> {
    fn descend(
        &mut self,
        next_operation: &mut [usize],
        job_ready: &mut [u64],
        machine_ready: &mut [u64],
        completed: usize,
        current_makespan: u64,
    ) -> Result<(), BenchmarkError> {
        self.expanded_nodes = self
            .expanded_nodes
            .checked_add(1)
            .ok_or(BenchmarkError::ArithmeticOverflow)?;
        if self.expanded_nodes > MAX_SEARCH_NODES {
            return Err(BenchmarkError::SearchNodeLimit {
                maximum: MAX_SEARCH_NODES,
            });
        }

        if completed == self.total_operations {
            if self
                .best_makespan
                .is_none_or(|best| current_makespan < best)
            {
                self.best_makespan = Some(current_makespan);
            }
            return Ok(());
        }

        for job_index in 0..self.instance.jobs.len() {
            let operation_index = next_operation[job_index];
            let Some(operation) = self.instance.jobs[job_index].get(operation_index).copied()
            else {
                continue;
            };
            let start = job_ready[job_index].max(machine_ready[operation.machine]);
            let finish = start
                .checked_add(operation.duration)
                .ok_or(BenchmarkError::ArithmeticOverflow)?;
            if self
                .best_makespan
                .is_some_and(|best| finish >= best)
            {
                continue;
            }

            let old_job_ready = job_ready[job_index];
            let old_machine_ready = machine_ready[operation.machine];
            next_operation[job_index] += 1;
            job_ready[job_index] = finish;
            machine_ready[operation.machine] = finish;

            let nested = self.descend(
                next_operation,
                job_ready,
                machine_ready,
                completed + 1,
                current_makespan.max(finish),
            );

            next_operation[job_index] -= 1;
            job_ready[job_index] = old_job_ready;
            machine_ready[operation.machine] = old_machine_ready;
            nested?;
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TINY: &str = "2 2\n0 2 1 1\n1 2 0 1\n";

    #[test]
    fn tiny_fixture_has_a_known_exact_makespan() {
        let instance = parse_classic_pair_format(TINY).expect("fixture parses");
        let summary = summarize(&instance).expect("summary succeeds");
        assert_eq!(summary.job_count, 2);
        assert_eq!(summary.machine_count, 2);
        assert_eq!(summary.operation_count, 4);
        assert_eq!(summary.total_processing_time, 6);
        assert_eq!(summary.maximum_job_work, 3);
        assert_eq!(summary.maximum_machine_load, 3);
        assert_eq!(summary.lower_bound_makespan, 3);

        let result = solve_exact(&instance).expect("exact search succeeds");
        assert_eq!(result.makespan, 3);
        assert_eq!(result.lower_bound_makespan, 3);
        assert!(result.expanded_nodes > 0);
    }

    #[test]
    fn pinned_taillard_instance_parses_without_solving() {
        let instance = parse_classic_pair_format(include_str!("../fixtures/ta01.txt"))
            .expect("pinned ta01 parses");
        let summary = summarize(&instance).expect("summary succeeds");
        assert_eq!(summary.job_count, 15);
        assert_eq!(summary.machine_count, 15);
        assert_eq!(summary.operation_count, 225);
        assert!(summary.total_processing_time > 0);
        assert!(summary.lower_bound_makespan > 0);
    }

    #[test]
    fn pinned_adams_instance_parses_without_solving() {
        let instance = parse_classic_pair_format(include_str!("../fixtures/abz5.txt"))
            .expect("pinned abz5 parses");
        let summary = summarize(&instance).expect("summary succeeds");
        assert_eq!(summary.job_count, 10);
        assert_eq!(summary.machine_count, 10);
        assert_eq!(summary.operation_count, 100);
    }

    #[test]
    fn malformed_token_count_is_rejected() {
        assert!(matches!(
            parse_classic_pair_format("2 2\n0 2 1\n1 2 0 1\n"),
            Err(BenchmarkError::UnexpectedTokenCount { .. })
        ));
    }

    #[test]
    fn machine_ids_are_range_checked() {
        assert!(matches!(
            parse_classic_pair_format("1 2\n2 3 0 1\n"),
            Err(BenchmarkError::MachineOutOfRange { .. })
        ));
    }

    #[test]
    fn zero_processing_time_is_rejected() {
        assert!(matches!(
            parse_classic_pair_format("1 1\n0 0\n"),
            Err(BenchmarkError::ZeroDuration { .. })
        ));
    }

    #[test]
    fn negative_and_non_integer_tokens_are_rejected() {
        assert!(matches!(
            parse_classic_pair_format("1 1\n0 -4\n"),
            Err(BenchmarkError::InvalidInteger { .. })
        ));
        assert!(matches!(
            parse_classic_pair_format("1 1\n0 2.5\n"),
            Err(BenchmarkError::InvalidInteger { .. })
        ));
    }

    #[test]
    fn exact_solver_fails_closed_above_tiny_instance_limit() {
        let instance = parse_classic_pair_format(include_str!("../fixtures/ta01.txt"))
            .expect("pinned fixture parses");
        assert!(matches!(
            solve_exact(&instance),
            Err(BenchmarkError::ExactOperationLimit {
                count: 225,
                maximum: MAX_EXACT_OPERATIONS
            })
        ));
    }

    #[test]
    fn whitespace_is_accepted_but_trailing_tokens_are_not() {
        let instance =
            parse_classic_pair_format("  2   2\n 0 2   1 1 \n1 2 0 1  \n")
                .expect("whitespace accepted");
        assert_eq!(summarize(&instance).expect("summary").operation_count, 4);
        assert!(matches!(
            parse_classic_pair_format("1 1\n0 2 extra\n"),
            Err(BenchmarkError::UnexpectedTokenCount { .. })
        ));
    }

    #[test]
    fn arithmetic_overflow_fails_closed() {
        assert!(matches!(
            parse_classic_pair_format("1 2\n0 18446744073709551615 1 1\n"),
            Err(BenchmarkError::ArithmeticOverflow)
        ));
    }
    #[test]
    fn exact_oracle_matches_independent_machine_order_enumeration_on_twenty_cases() {
        for case_index in 0..20 {
            let machine_count = if case_index % 3 == 0 { 2 } else { 3 };
            let job_count = if case_index % 2 == 0 { 2 } else { 3 };
            let mut jobs = Vec::with_capacity(job_count);

            for job_index in 0..job_count {
                let mut route: Vec<usize> = (0..machine_count).collect();
                route.rotate_left((case_index + job_index) % machine_count);
                if (case_index + job_index) % 2 == 1 {
                    route.reverse();
                }
                let job = route
                    .into_iter()
                    .enumerate()
                    .map(|(operation_index, machine)| Operation {
                        machine,
                        duration: ((case_index * 7 + job_index * 5 + operation_index * 3) % 9
                            + 1) as u64,
                    })
                    .collect();
                jobs.push(job);
            }

            let instance = Instance {
                machine_count,
                jobs,
            };
            let expected = independent_machine_order_oracle(&instance);
            let actual = solve_exact(&instance).expect("bounded exact solver succeeds");
            assert_eq!(
                actual.makespan, expected,
                "disagreement on generated tiny case {case_index}"
            );
        }
    }

    // Independent reference formulation: enumerate each machine's operation
    // order, build job-precedence and machine-precedence edges, reject cyclic
    // combinations, and compute the longest path through the resulting DAG.
    // The production oracle instead enumerates job-ready serial decisions.
    fn independent_machine_order_oracle(instance: &Instance) -> u64 {
        use std::collections::VecDeque;

        fn enumerate_permutations(
            values: &mut Vec<usize>,
            start: usize,
            output: &mut Vec<Vec<usize>>,
        ) {
            if start == values.len() {
                output.push(values.clone());
                return;
            }
            for index in start..values.len() {
                values.swap(start, index);
                enumerate_permutations(values, start + 1, output);
                values.swap(start, index);
            }
        }

        fn add_edge(
            from: usize,
            to: usize,
            successors: &mut [Vec<usize>],
            indegree: &mut [usize],
        ) {
            successors[from].push(to);
            indegree[to] += 1;
        }

        fn evaluate_machine_orders(
            instance: &Instance,
            machine_orders: &[Vec<usize>],
        ) -> Option<u64> {
            let operation_count: usize = instance.jobs.iter().map(Vec::len).sum();
            let mut offsets = Vec::with_capacity(instance.jobs.len() + 1);
            let mut durations = Vec::with_capacity(operation_count);
            let mut successors = vec![Vec::new(); operation_count];
            let mut indegree = vec![0_usize; operation_count];
            let mut cursor = 0_usize;

            for job in &instance.jobs {
                offsets.push(cursor);
                for operation in job {
                    durations.push(operation.duration);
                    cursor += 1;
                }
            }
            offsets.push(cursor);

            for (job_index, job) in instance.jobs.iter().enumerate() {
                let first = offsets[job_index];
                for operation_index in 0..job.len().saturating_sub(1) {
                    add_edge(
                        first + operation_index,
                        first + operation_index + 1,
                        &mut successors,
                        &mut indegree,
                    );
                }
            }

            for order in machine_orders {
                for pair in order.windows(2) {
                    add_edge(
                        pair[0],
                        pair[1],
                        &mut successors,
                        &mut indegree,
                    );
                }
            }

            let mut ready = VecDeque::new();
            for (operation, degree) in indegree.iter().enumerate() {
                if *degree == 0 {
                    ready.push_back(operation);
                }
            }
            let mut start_time = vec![0_u64; operation_count];
            let mut visited = 0_usize;
            let mut makespan = 0_u64;
            while let Some(operation) = ready.pop_front() {
                visited += 1;
                let finish = start_time[operation].checked_add(durations[operation])?;
                makespan = makespan.max(finish);
                for successor in &successors[operation] {
                    start_time[*successor] = start_time[*successor].max(finish);
                    indegree[*successor] -= 1;
                    if indegree[*successor] == 0 {
                        ready.push_back(*successor);
                    }
                }
            }
            (visited == operation_count).then_some(makespan)
        }

        fn visit_machine(
            machine_index: usize,
            instance: &Instance,
            permutations: &[Vec<Vec<usize>>],
            selected: &mut Vec<Vec<usize>>,
            best: &mut Option<u64>,
        ) {
            if machine_index == permutations.len() {
                if let Some(candidate) = evaluate_machine_orders(instance, selected) {
                    if best.is_none_or(|known| candidate < known) {
                        *best = Some(candidate);
                    }
                }
                return;
            }
            for order in &permutations[machine_index] {
                selected.push(order.clone());
                visit_machine(machine_index + 1, instance, permutations, selected, best);
                selected.pop();
            }
        }

        let operation_count: usize = instance.jobs.iter().map(Vec::len).sum();
        let mut machine_members = vec![Vec::new(); instance.machine_count];
        let mut operation_id = 0_usize;
        for job in &instance.jobs {
            for operation in job {
                machine_members[operation.machine].push(operation_id);
                operation_id += 1;
            }
        }
        assert_eq!(operation_count, operation_id);

        let permutations: Vec<Vec<Vec<usize>>> = machine_members
            .into_iter()
            .map(|mut members| {
                let mut output = Vec::new();
                enumerate_permutations(&mut members, 0, &mut output);
                output
            })
            .collect();
        let mut best = None;
        visit_machine(0, instance, &permutations, &mut Vec::new(), &mut best);
        best.expect("at least one acyclic machine ordering exists")
    }

}
