// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded, deterministic single-stage textile production scheduling.
//!
//! This is a planning primitive, not a factory simulator or impact calculator.
//! All times and resource quantities are integer ticks in caller-declared units.
//! Failed or unknown hard gates block a job; they are never traded against
//! throughput, energy, water, or delivery objectives.

use std::collections::{BTreeMap, BTreeSet};
use std::error::Error;
use std::fmt;

pub const EXHAUSTIVE_MAX_JOBS: usize = 7;
pub const EXHAUSTIVE_MAX_MACHINES: usize = 4;
pub const EXHAUSTIVE_NODE_LIMIT: u64 = 2_000_000;

/// Evidence status for a non-compensatory job requirement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum GateOutcome {
    Pass { evidence_ref: String },
    Fail { evidence_ref: String },
    Unknown { reason: String },
}

/// A required pre-dispatch readiness gate, such as process qualification,
/// material compatibility, safety authorization, or conservation approval.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GateEvidence {
    pub id: String,
    pub outcome: GateOutcome,
}

/// One required production lot or garment-making operation.
///
/// The profile and functional-unit identifiers preserve context for later
/// evaluation. They are not converted to a shared value score. A rare-use
/// garment is not penalized for being rare-use.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TextileJob {
    pub id: String,
    pub profile_id: String,
    pub functional_unit_id: String,
    pub setup_family: String,
    pub required_capabilities: BTreeSet<String>,
    pub release_tick: u64,
    pub due_tick: u64,
    /// Optional non-negotiable latest completion tick.
    pub hard_finish_by_tick: Option<u64>,
    pub processing_ticks: u64,
    pub energy_units: u64,
    pub water_units: u64,
    pub process_evidence_ref: String,
    /// Preconditions for releasing the job to scheduling. Post-run product
    /// acceptance must be measured separately; a planned finish is not acceptance.
    pub required_gates: Vec<GateEvidence>,
}

/// A machine or work cell and its currently configured setup family.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Machine {
    pub id: String,
    pub capabilities: BTreeSet<String>,
    pub initial_setup_family: String,
    pub available_tick: u64,
    pub capability_evidence_ref: String,
}

/// Measured/declared cost of switching one machine between setup families.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SetupTransition {
    pub machine_id: String,
    pub from_family: String,
    pub to_family: String,
    pub duration_ticks: u64,
    pub energy_units: u64,
    pub water_units: u64,
    pub evidence_ref: String,
}

/// One planning horizon with exact resource units and hard total budgets.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleProblem {
    pub id: String,
    pub time_unit_id: String,
    pub energy_unit_id: String,
    pub water_unit_id: String,
    pub energy_budget_units: u64,
    pub water_budget_units: u64,
    pub jobs: Vec<TextileJob>,
    pub machines: Vec<Machine>,
    pub setup_transitions: Vec<SetupTransition>,
}

/// A job's actual interval on one chosen machine, including setup.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduledOperation {
    pub job_id: String,
    pub profile_id: String,
    pub functional_unit_id: String,
    pub machine_id: String,
    pub setup_from_family: String,
    pub setup_to_family: String,
    pub setup_start_tick: u64,
    pub setup_finish_tick: u64,
    pub process_start_tick: u64,
    pub finish_tick: u64,
    pub setup_duration_ticks: u64,
    pub setup_energy_units: u64,
    pub setup_water_units: u64,
}

/// All quantities are minimized when comparing feasible schedules. No
/// weighted sum is used; the returned frontier exposes trade-offs explicitly.
#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct ScheduleMetrics {
    pub late_jobs: u64,
    pub total_tardiness_ticks: u64,
    pub makespan_tick: u64,
    pub energy_units: u64,
    pub water_units: u64,
    pub setup_duration_ticks: u64,
}

/// One representative schedule for a unique objective vector.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScheduleCandidate {
    pub problem_id: String,
    pub time_unit_id: String,
    pub energy_unit_id: String,
    pub water_unit_id: String,
    pub operations: Vec<ScheduledOperation>,
    pub metrics: ScheduleMetrics,
}

/// Independent single-objective optima across the enumerated feasible set.
/// These are lower bounds, not necessarily jointly attainable by one schedule.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ObjectiveOptima {
    pub minimum_late_jobs: u64,
    pub minimum_total_tardiness_ticks: u64,
    pub minimum_makespan_tick: u64,
    pub minimum_energy_units: u64,
    pub minimum_water_units: u64,
    pub minimum_setup_duration_ticks: u64,
}

/// Exact result for bounded tiny instances. Larger cases should use dispatch
/// baselines and a separately budgeted solver, not silently truncate search.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ExhaustiveReport {
    pub problem_id: String,
    pub time_unit_id: String,
    pub energy_unit_id: String,
    pub water_unit_id: String,
    pub pareto_frontier: Vec<ScheduleCandidate>,
    pub objective_optima: ObjectiveOptima,
    /// Number of feasible complete assignments/orderings visited, including
    /// schedules sharing the same objective vector.
    pub complete_schedules: u64,
    pub explored_nodes: u64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum DispatchRule {
    EarliestDue,
    LowestSetup,
    LowestEnergy,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ScheduleError {
    EmptyProblemId,
    EmptyTimeUnitId,
    EmptyEnergyUnitId,
    EmptyWaterUnitId,
    EmptyJobs,
    EmptyMachines,
    EmptyJobId,
    DuplicateJobId { job_id: String },
    EmptyProfileId { job_id: String },
    EmptyFunctionalUnitId { job_id: String },
    EmptyJobSetupFamily { job_id: String },
    ZeroProcessingTime { job_id: String },
    MissingProcessEvidence { job_id: String },
    NoRequiredGates { job_id: String },
    EmptyGateId { job_id: String },
    DuplicateGateId { job_id: String, gate_id: String },
    MissingGateEvidence { job_id: String, gate_id: String },
    EmptyUnknownGateReason { job_id: String, gate_id: String },
    BlockedJob { job_id: String, gate_id: String },
    UnverifiedJob { job_id: String, gate_id: String },
    EmptyMachineId,
    DuplicateMachineId { machine_id: String },
    EmptyMachineSetupFamily { machine_id: String },
    MissingCapabilityEvidence { machine_id: String },
    EmptyTransitionField,
    UnknownTransitionMachine { machine_id: String },
    MissingTransitionEvidence { machine_id: String, from_family: String, to_family: String },
    DuplicateTransition { machine_id: String, from_family: String, to_family: String },
    NoEligibleMachine { job_id: String },
    MissingSetupTransition { machine_id: String, from_family: String, to_family: String },
    ProcessEnergyOverBudget,
    ProcessWaterOverBudget,
    ArithmeticOverflow { field: &'static str },
    TooManyJobsForExhaustive { actual: usize, maximum: usize },
    TooManyMachinesForExhaustive { actual: usize, maximum: usize },
    SearchNodeLimitExceeded { limit: u64 },
    NoFeasibleSchedule,
    NoFeasibleDispatch { rule: DispatchRule },
}

impl fmt::Display for ScheduleError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "textile scheduler error: {self:?}")
    }
}

impl Error for ScheduleError {}

#[derive(Clone)]
struct PreparedProblem {
    id: String,
    time_unit_id: String,
    energy_unit_id: String,
    water_unit_id: String,
    energy_budget_units: u64,
    water_budget_units: u64,
    jobs: Vec<TextileJob>,
    machines: Vec<Machine>,
    transitions: BTreeMap<(String, String, String), SetupTransition>,
    eligible_machines: Vec<Vec<usize>>,
    base_energy_units: u64,
    base_water_units: u64,
}

#[derive(Clone, Debug)]
struct MachineState {
    available_tick: u64,
    setup_family: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct DispatchKey {
    primary: u64,
    secondary: u64,
    tertiary: u64,
    quaternary: u64,
    quinary: u64,
    job_id: String,
    machine_id: String,
}

struct DispatchChoice {
    key: DispatchKey,
    job_index: usize,
    machine_index: usize,
    next_state: MachineState,
    operation: ScheduledOperation,
    next_setup_energy: u64,
    next_setup_water: u64,
}

/// Build a deterministic Pareto frontier by enumerating every schedule for a
/// tiny instance. Every required job must be included exactly once.
pub fn solve_exhaustive(problem: &ScheduleProblem) -> Result<ExhaustiveReport, ScheduleError> {
    let prepared = prepare(problem)?;
    if prepared.jobs.len() > EXHAUSTIVE_MAX_JOBS {
        return Err(ScheduleError::TooManyJobsForExhaustive {
            actual: prepared.jobs.len(),
            maximum: EXHAUSTIVE_MAX_JOBS,
        });
    }
    if prepared.machines.len() > EXHAUSTIVE_MAX_MACHINES {
        return Err(ScheduleError::TooManyMachinesForExhaustive {
            actual: prepared.machines.len(),
            maximum: EXHAUSTIVE_MAX_MACHINES,
        });
    }

    let mut used = vec![false; prepared.jobs.len()];
    let mut states: Vec<MachineState> = prepared
        .machines
        .iter()
        .map(|machine| MachineState {
            available_tick: machine.available_tick,
            setup_family: machine.initial_setup_family.clone(),
        })
        .collect();
    let mut operations = Vec::with_capacity(prepared.jobs.len());
    let mut schedules_by_metrics = BTreeMap::new();
    let mut complete_schedules = 0_u64;
    let mut explored_nodes = 0_u64;

    enumerate_machine_sequences(
        &prepared,
        0,
        &mut used,
        &mut states,
        &mut operations,
        0,
        0,
        &mut explored_nodes,
        &mut complete_schedules,
        &mut schedules_by_metrics,
    )?;

    if schedules_by_metrics.is_empty() {
        return Err(ScheduleError::NoFeasibleSchedule);
    }

    let objective_optima = calculate_optima(schedules_by_metrics.keys());
    let all_schedules: Vec<ScheduleCandidate> =
        schedules_by_metrics.into_values().collect();
    let mut pareto_frontier = Vec::new();

    for candidate in &all_schedules {
        let is_dominated = all_schedules.iter().any(|other| {
            other.metrics != candidate.metrics && dominates(&other.metrics, &candidate.metrics)
        });
        if !is_dominated {
            pareto_frontier.push(candidate.clone());
        }
    }
    pareto_frontier.sort_by(|left, right| left.metrics.cmp(&right.metrics));

    Ok(ExhaustiveReport {
        problem_id: prepared.id.clone(),
        time_unit_id: prepared.time_unit_id.clone(),
        energy_unit_id: prepared.energy_unit_id.clone(),
        water_unit_id: prepared.water_unit_id.clone(),
        pareto_frontier,
        objective_optima,
        complete_schedules,
        explored_nodes,
    })
}

/// Produce a deterministic feasible dispatch baseline. This is a heuristic,
/// not an optimality claim; compare its score with the exhaustive optima on
/// tiny instances before using it as a scalable baseline.
pub fn dispatch_schedule(
    problem: &ScheduleProblem,
    rule: DispatchRule,
) -> Result<ScheduleCandidate, ScheduleError> {
    let prepared = prepare(problem)?;
    let mut used = vec![false; prepared.jobs.len()];
    let mut states: Vec<MachineState> = prepared
        .machines
        .iter()
        .map(|machine| MachineState {
            available_tick: machine.available_tick,
            setup_family: machine.initial_setup_family.clone(),
        })
        .collect();
    let mut operations = Vec::with_capacity(prepared.jobs.len());
    let mut setup_energy = 0_u64;
    let mut setup_water = 0_u64;

    while used.iter().any(|is_used| !is_used) {
        let mut choices = Vec::new();
        for (job_index, job) in prepared.jobs.iter().enumerate() {
            if used[job_index] {
                continue;
            }
            for &machine_index in &prepared.eligible_machines[job_index] {
                let machine = &prepared.machines[machine_index];
                let state = &states[machine_index];
                let transition = transition_for(
                    &prepared,
                    &machine.id,
                    &state.setup_family,
                    &job.setup_family,
                )?;
                let operation = make_operation(
                    job,
                    machine,
                    state,
                    transition,
                )?;
                if job
                    .hard_finish_by_tick
                    .is_some_and(|latest| operation.finish_tick > latest)
                {
                    continue;
                }
                let next_setup_energy = checked_add(
                    setup_energy,
                    transition.energy_units,
                    "setup energy",
                )?;
                let next_setup_water = checked_add(
                    setup_water,
                    transition.water_units,
                    "setup water",
                )?;
                if !within_budgets(&prepared, next_setup_energy, next_setup_water)? {
                    continue;
                }

                let key = dispatch_key(rule, job, &operation, transition)?;
                choices.push(DispatchChoice {
                    key,
                    job_index,
                    machine_index,
                    next_state: MachineState {
                        available_tick: operation.finish_tick,
                        setup_family: job.setup_family.clone(),
                    },
                    operation,
                    next_setup_energy,
                    next_setup_water,
                });
            }
        }

        choices.sort_by(|left, right| left.key.cmp(&right.key));
        let Some(choice) = choices.into_iter().next() else {
            return Err(ScheduleError::NoFeasibleDispatch { rule });
        };

        used[choice.job_index] = true;
        states[choice.machine_index] = choice.next_state;
        operations.push(choice.operation);
        setup_energy = choice.next_setup_energy;
        setup_water = choice.next_setup_water;
    }

    let candidate = make_candidate(&prepared, operations, setup_energy, setup_water)?
        .ok_or(ScheduleError::NoFeasibleDispatch { rule })?;
    Ok(candidate)
}

fn prepare(problem: &ScheduleProblem) -> Result<PreparedProblem, ScheduleError> {
    if problem.id.trim().is_empty() {
        return Err(ScheduleError::EmptyProblemId);
    }
    if problem.time_unit_id.trim().is_empty() {
        return Err(ScheduleError::EmptyTimeUnitId);
    }
    if problem.energy_unit_id.trim().is_empty() {
        return Err(ScheduleError::EmptyEnergyUnitId);
    }
    if problem.water_unit_id.trim().is_empty() {
        return Err(ScheduleError::EmptyWaterUnitId);
    }
    if problem.jobs.is_empty() {
        return Err(ScheduleError::EmptyJobs);
    }
    if problem.machines.is_empty() {
        return Err(ScheduleError::EmptyMachines);
    }

    let mut jobs = problem.jobs.clone();
    jobs.sort_by(|left, right| left.id.cmp(&right.id));
    for job in &mut jobs {
        job.required_gates.sort_by(|left, right| left.id.cmp(&right.id));
    }
    let mut seen_job_ids = BTreeSet::new();
    let mut base_energy_units = 0_u64;
    let mut base_water_units = 0_u64;

    for job in &jobs {
        if job.id.trim().is_empty() {
            return Err(ScheduleError::EmptyJobId);
        }
        if !seen_job_ids.insert(job.id.as_str()) {
            return Err(ScheduleError::DuplicateJobId {
                job_id: job.id.clone(),
            });
        }
        if job.profile_id.trim().is_empty() {
            return Err(ScheduleError::EmptyProfileId {
                job_id: job.id.clone(),
            });
        }
        if job.functional_unit_id.trim().is_empty() {
            return Err(ScheduleError::EmptyFunctionalUnitId {
                job_id: job.id.clone(),
            });
        }
        if job.setup_family.trim().is_empty() {
            return Err(ScheduleError::EmptyJobSetupFamily {
                job_id: job.id.clone(),
            });
        }
        if job.processing_ticks == 0 {
            return Err(ScheduleError::ZeroProcessingTime {
                job_id: job.id.clone(),
            });
        }
        if job.process_evidence_ref.trim().is_empty() {
            return Err(ScheduleError::MissingProcessEvidence {
                job_id: job.id.clone(),
            });
        }
        if job.required_gates.is_empty() {
            return Err(ScheduleError::NoRequiredGates {
                job_id: job.id.clone(),
            });
        }

        let mut gate_ids = BTreeSet::new();
        for gate in &job.required_gates {
            if gate.id.trim().is_empty() {
                return Err(ScheduleError::EmptyGateId {
                    job_id: job.id.clone(),
                });
            }
            if !gate_ids.insert(gate.id.as_str()) {
                return Err(ScheduleError::DuplicateGateId {
                    job_id: job.id.clone(),
                    gate_id: gate.id.clone(),
                });
            }
            match &gate.outcome {
                GateOutcome::Pass { evidence_ref } => {
                    if evidence_ref.trim().is_empty() {
                        return Err(ScheduleError::MissingGateEvidence {
                            job_id: job.id.clone(),
                            gate_id: gate.id.clone(),
                        });
                    }
                }
                GateOutcome::Fail { evidence_ref } => {
                    if evidence_ref.trim().is_empty() {
                        return Err(ScheduleError::MissingGateEvidence {
                            job_id: job.id.clone(),
                            gate_id: gate.id.clone(),
                        });
                    }
                    return Err(ScheduleError::BlockedJob {
                        job_id: job.id.clone(),
                        gate_id: gate.id.clone(),
                    });
                }
                GateOutcome::Unknown { reason } => {
                    if reason.trim().is_empty() {
                        return Err(ScheduleError::EmptyUnknownGateReason {
                            job_id: job.id.clone(),
                            gate_id: gate.id.clone(),
                        });
                    }
                    return Err(ScheduleError::UnverifiedJob {
                        job_id: job.id.clone(),
                        gate_id: gate.id.clone(),
                    });
                }
            }
        }

        base_energy_units = checked_add(
            base_energy_units,
            job.energy_units,
            "base process energy",
        )?;
        base_water_units = checked_add(
            base_water_units,
            job.water_units,
            "base process water",
        )?;
    }

    let mut machines = problem.machines.clone();
    machines.sort_by(|left, right| left.id.cmp(&right.id));
    let mut machine_ids = BTreeSet::new();
    for machine in &machines {
        if machine.id.trim().is_empty() {
            return Err(ScheduleError::EmptyMachineId);
        }
        if !machine_ids.insert(machine.id.as_str()) {
            return Err(ScheduleError::DuplicateMachineId {
                machine_id: machine.id.clone(),
            });
        }
        if machine.initial_setup_family.trim().is_empty() {
            return Err(ScheduleError::EmptyMachineSetupFamily {
                machine_id: machine.id.clone(),
            });
        }
        if machine.capability_evidence_ref.trim().is_empty() {
            return Err(ScheduleError::MissingCapabilityEvidence {
                machine_id: machine.id.clone(),
            });
        }
    }

    if base_energy_units > problem.energy_budget_units {
        return Err(ScheduleError::ProcessEnergyOverBudget);
    }
    if base_water_units > problem.water_budget_units {
        return Err(ScheduleError::ProcessWaterOverBudget);
    }

    let known_machines: BTreeSet<&str> = machines.iter().map(|item| item.id.as_str()).collect();
    let mut transitions = BTreeMap::new();
    for transition in &problem.setup_transitions {
        if transition.machine_id.trim().is_empty()
            || transition.from_family.trim().is_empty()
            || transition.to_family.trim().is_empty()
        {
            return Err(ScheduleError::EmptyTransitionField);
        }
        if !known_machines.contains(transition.machine_id.as_str()) {
            return Err(ScheduleError::UnknownTransitionMachine {
                machine_id: transition.machine_id.clone(),
            });
        }
        if transition.evidence_ref.trim().is_empty() {
            return Err(ScheduleError::MissingTransitionEvidence {
                machine_id: transition.machine_id.clone(),
                from_family: transition.from_family.clone(),
                to_family: transition.to_family.clone(),
            });
        }
        let key = (
            transition.machine_id.clone(),
            transition.from_family.clone(),
            transition.to_family.clone(),
        );
        if transitions.insert(key, transition.clone()).is_some() {
            return Err(ScheduleError::DuplicateTransition {
                machine_id: transition.machine_id.clone(),
                from_family: transition.from_family.clone(),
                to_family: transition.to_family.clone(),
            });
        }
    }

    let mut eligible_machines = Vec::with_capacity(jobs.len());
    for job in &jobs {
        let eligible: Vec<usize> = machines
            .iter()
            .enumerate()
            .filter_map(|(index, machine)| {
                job.required_capabilities
                    .is_subset(&machine.capabilities)
                    .then_some(index)
            })
            .collect();
        if eligible.is_empty() {
            return Err(ScheduleError::NoEligibleMachine {
                job_id: job.id.clone(),
            });
        }
        eligible_machines.push(eligible);
    }

    // A missing setup edge means unknown process cost, not a free changeover.
    // Require complete transition rows for each setup family actually usable
    // on each eligible machine; same-family transitions are implicitly zero.
    for (machine_index, machine) in machines.iter().enumerate() {
        let mut target_families = BTreeSet::new();
        for (job_index, job) in jobs.iter().enumerate() {
            if eligible_machines[job_index].contains(&machine_index) {
                target_families.insert(job.setup_family.clone());
            }
        }
        let mut source_families = target_families.clone();
        source_families.insert(machine.initial_setup_family.clone());
        for from_family in &source_families {
            for to_family in &target_families {
                if from_family != to_family
                    && !transitions.contains_key(&(
                        machine.id.clone(),
                        from_family.clone(),
                        to_family.clone(),
                    ))
                {
                    return Err(ScheduleError::MissingSetupTransition {
                        machine_id: machine.id.clone(),
                        from_family: from_family.clone(),
                        to_family: to_family.clone(),
                    });
                }
            }
        }
    }

    Ok(PreparedProblem {
        id: problem.id.clone(),
        time_unit_id: problem.time_unit_id.clone(),
        energy_unit_id: problem.energy_unit_id.clone(),
        water_unit_id: problem.water_unit_id.clone(),
        energy_budget_units: problem.energy_budget_units,
        water_budget_units: problem.water_budget_units,
        jobs,
        machines,
        transitions,
        eligible_machines,
        base_energy_units,
        base_water_units,
    })
}

fn transition_for(
    problem: &PreparedProblem,
    machine_id: &str,
    from_family: &str,
    to_family: &str,
) -> Result<TransitionUse, ScheduleError> {
    if from_family == to_family {
        return Ok(TransitionUse {
            duration_ticks: 0,
            energy_units: 0,
            water_units: 0,
        });
    }
    let transition = problem
        .transitions
        .get(&(
            machine_id.to_owned(),
            from_family.to_owned(),
            to_family.to_owned(),
        ))
        .ok_or_else(|| ScheduleError::MissingSetupTransition {
            machine_id: machine_id.to_owned(),
            from_family: from_family.to_owned(),
            to_family: to_family.to_owned(),
        })?;
    Ok(TransitionUse {
        duration_ticks: transition.duration_ticks,
        energy_units: transition.energy_units,
        water_units: transition.water_units,
    })
}

#[derive(Clone, Copy)]
struct TransitionUse {
    duration_ticks: u64,
    energy_units: u64,
    water_units: u64,
}

fn make_operation(
    job: &TextileJob,
    machine: &Machine,
    state: &MachineState,
    transition: TransitionUse,
) -> Result<ScheduledOperation, ScheduleError> {
    let setup_finish_tick = checked_add(
        state.available_tick,
        transition.duration_ticks,
        "setup finish tick",
    )?;
    let process_start_tick = job.release_tick.max(setup_finish_tick);
    let finish_tick = checked_add(
        process_start_tick,
        job.processing_ticks,
        "job finish tick",
    )?;
    Ok(ScheduledOperation {
        job_id: job.id.clone(),
        profile_id: job.profile_id.clone(),
        functional_unit_id: job.functional_unit_id.clone(),
        machine_id: machine.id.clone(),
        setup_from_family: state.setup_family.clone(),
        setup_to_family: job.setup_family.clone(),
        setup_start_tick: state.available_tick,
        setup_finish_tick,
        process_start_tick,
        finish_tick,
        setup_duration_ticks: transition.duration_ticks,
        setup_energy_units: transition.energy_units,
        setup_water_units: transition.water_units,
    })
}

fn within_budgets(
    problem: &PreparedProblem,
    setup_energy_units: u64,
    setup_water_units: u64,
) -> Result<bool, ScheduleError> {
    let energy = checked_add(
        problem.base_energy_units,
        setup_energy_units,
        "total energy",
    )?;
    let water = checked_add(
        problem.base_water_units,
        setup_water_units,
        "total water",
    )?;
    Ok(energy <= problem.energy_budget_units && water <= problem.water_budget_units)
}

fn make_candidate(
    problem: &PreparedProblem,
    mut operations: Vec<ScheduledOperation>,
    setup_energy_units: u64,
    setup_water_units: u64,
) -> Result<Option<ScheduleCandidate>, ScheduleError> {
    if !within_budgets(problem, setup_energy_units, setup_water_units)? {
        return Ok(None);
    }

    let mut late_jobs = 0_u64;
    let mut total_tardiness_ticks = 0_u64;
    let mut makespan_tick = 0_u64;
    let mut setup_duration_ticks = 0_u64;

    for operation in &operations {
        let job = problem
            .jobs
            .iter()
            .find(|job| job.id == operation.job_id)
            .ok_or_else(|| ScheduleError::EmptyJobId)?;
        if operation.finish_tick > job.due_tick {
            late_jobs = checked_add(late_jobs, 1, "late job count")?;
            total_tardiness_ticks = checked_add(
                total_tardiness_ticks,
                operation.finish_tick - job.due_tick,
                "total tardiness",
            )?;
        }
        makespan_tick = makespan_tick.max(operation.finish_tick);
        setup_duration_ticks = checked_add(
            setup_duration_ticks,
            operation.setup_duration_ticks,
            "total setup duration",
        )?;
    }
    operations.sort_by(|left, right| {
        left.machine_id
            .cmp(&right.machine_id)
            .then(left.setup_start_tick.cmp(&right.setup_start_tick))
            .then(left.job_id.cmp(&right.job_id))
    });

    Ok(Some(ScheduleCandidate {
        problem_id: problem.id.clone(),
        time_unit_id: problem.time_unit_id.clone(),
        energy_unit_id: problem.energy_unit_id.clone(),
        water_unit_id: problem.water_unit_id.clone(),
        operations,
        metrics: ScheduleMetrics {
            late_jobs,
            total_tardiness_ticks,
            makespan_tick,
            energy_units: checked_add(
                problem.base_energy_units,
                setup_energy_units,
                "total energy",
            )?,
            water_units: checked_add(
                problem.base_water_units,
                setup_water_units,
                "total water",
            )?,
            setup_duration_ticks,
        },
    }))
}

fn enumerate_machine_sequences(
    problem: &PreparedProblem,
    machine_index: usize,
    used: &mut [bool],
    states: &mut [MachineState],
    operations: &mut Vec<ScheduledOperation>,
    setup_energy_units: u64,
    setup_water_units: u64,
    explored_nodes: &mut u64,
    complete_schedules: &mut u64,
    schedules_by_metrics: &mut BTreeMap<ScheduleMetrics, ScheduleCandidate>,
) -> Result<(), ScheduleError> {
    *explored_nodes = checked_add(*explored_nodes, 1, "search node count")?;
    if *explored_nodes > EXHAUSTIVE_NODE_LIMIT {
        return Err(ScheduleError::SearchNodeLimitExceeded {
            limit: EXHAUSTIVE_NODE_LIMIT,
        });
    }

    if used.iter().all(|is_used| *is_used) {
        if let Some(candidate) = make_candidate(
            problem,
            operations.clone(),
            setup_energy_units,
            setup_water_units,
        )? {
            *complete_schedules = checked_add(*complete_schedules, 1, "complete schedule count")?;
            schedules_by_metrics
                .entry(candidate.metrics.clone())
                .or_insert(candidate);
        }
        return Ok(());
    }
    if machine_index >= problem.machines.len() {
        return Ok(());
    }

    // Finish this machine's sequence now, then let later machines handle any
    // remaining jobs. This avoids duplicate schedules caused by cross-machine
    // interleavings while still enumerating every per-machine order.
    enumerate_machine_sequences(
        problem,
        machine_index + 1,
        used,
        states,
        operations,
        setup_energy_units,
        setup_water_units,
        explored_nodes,
        complete_schedules,
        schedules_by_metrics,
    )?;

    let machine = &problem.machines[machine_index];
    for job_index in 0..problem.jobs.len() {
        if used[job_index] || !problem.eligible_machines[job_index].contains(&machine_index) {
            continue;
        }
        let job = &problem.jobs[job_index];
        let current_state = states[machine_index].clone();
        let transition = transition_for(
            problem,
            &machine.id,
            &current_state.setup_family,
            &job.setup_family,
        )?;
        let operation = make_operation(job, machine, &current_state, transition)?;
        if job
            .hard_finish_by_tick
            .is_some_and(|latest| operation.finish_tick > latest)
        {
            continue;
        }

        let next_setup_energy = checked_add(
            setup_energy_units,
            transition.energy_units,
            "setup energy",
        )?;
        let next_setup_water = checked_add(
            setup_water_units,
            transition.water_units,
            "setup water",
        )?;
        if !within_budgets(problem, next_setup_energy, next_setup_water)? {
            continue;
        }

        used[job_index] = true;
        states[machine_index] = MachineState {
            available_tick: operation.finish_tick,
            setup_family: job.setup_family.clone(),
        };
        operations.push(operation);
        let branch_result = enumerate_machine_sequences(
            problem,
            machine_index,
            used,
            states,
            operations,
            next_setup_energy,
            next_setup_water,
            explored_nodes,
            complete_schedules,
            schedules_by_metrics,
        );
        operations.pop();
        states[machine_index] = current_state;
        used[job_index] = false;
        branch_result?;
    }
    Ok(())
}

fn calculate_optima<'a>(
    metrics: impl Iterator<Item = &'a ScheduleMetrics>,
) -> ObjectiveOptima {
    let scores: Vec<&ScheduleMetrics> = metrics.collect();
    ObjectiveOptima {
        minimum_late_jobs: scores.iter().map(|score| score.late_jobs).min().unwrap_or(0),
        minimum_total_tardiness_ticks: scores
            .iter()
            .map(|score| score.total_tardiness_ticks)
            .min()
            .unwrap_or(0),
        minimum_makespan_tick: scores
            .iter()
            .map(|score| score.makespan_tick)
            .min()
            .unwrap_or(0),
        minimum_energy_units: scores
            .iter()
            .map(|score| score.energy_units)
            .min()
            .unwrap_or(0),
        minimum_water_units: scores
            .iter()
            .map(|score| score.water_units)
            .min()
            .unwrap_or(0),
        minimum_setup_duration_ticks: scores
            .iter()
            .map(|score| score.setup_duration_ticks)
            .min()
            .unwrap_or(0),
    }
}

fn dominates(left: &ScheduleMetrics, right: &ScheduleMetrics) -> bool {
    let no_worse = left.late_jobs <= right.late_jobs
        && left.total_tardiness_ticks <= right.total_tardiness_ticks
        && left.makespan_tick <= right.makespan_tick
        && left.energy_units <= right.energy_units
        && left.water_units <= right.water_units
        && left.setup_duration_ticks <= right.setup_duration_ticks;
    let strictly_better = left.late_jobs < right.late_jobs
        || left.total_tardiness_ticks < right.total_tardiness_ticks
        || left.makespan_tick < right.makespan_tick
        || left.energy_units < right.energy_units
        || left.water_units < right.water_units
        || left.setup_duration_ticks < right.setup_duration_ticks;
    no_worse && strictly_better
}

fn dispatch_key(
    rule: DispatchRule,
    job: &TextileJob,
    operation: &ScheduledOperation,
    transition: TransitionUse,
) -> Result<DispatchKey, ScheduleError> {
    let setup = transition.duration_ticks;
    let energy = checked_add(job.energy_units, transition.energy_units, "dispatch energy key")?;
    let water = checked_add(job.water_units, transition.water_units, "dispatch water key")?;
    let due = job.due_tick;
    let finish = operation.finish_tick;
    let (primary, secondary, tertiary, quaternary, quinary) = match rule {
        DispatchRule::EarliestDue => (due, finish, setup, energy, water),
        DispatchRule::LowestSetup => (setup, due, finish, energy, water),
        DispatchRule::LowestEnergy => (energy, water, due, setup, finish),
    };
    Ok(DispatchKey {
        primary,
        secondary,
        tertiary,
        quaternary,
        quinary,
        job_id: job.id.clone(),
        machine_id: operation.machine_id.clone(),
    })
}

fn checked_add(left: u64, right: u64, field: &'static str) -> Result<u64, ScheduleError> {
    left.checked_add(right)
        .ok_or(ScheduleError::ArithmeticOverflow { field })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn passing_gate() -> GateEvidence {
        GateEvidence {
            id: "process-readiness".to_owned(),
            outcome: GateOutcome::Pass {
                evidence_ref: "fixture:process-readiness-pass".to_owned(),
            },
        }
    }

    fn job(
        id: &str,
        profile_id: &str,
        family: &str,
        required_capabilities: &[&str],
        release_tick: u64,
        due_tick: u64,
        hard_finish_by_tick: Option<u64>,
        processing_ticks: u64,
        energy_units: u64,
        water_units: u64,
    ) -> TextileJob {
        let functional_unit_id = match profile_id {
            "everyday-v0" => "wear-care-cycle-v0",
            "couture-decorative-v0" => "event-service-v0",
            _ => "declared-unit-v0",
        };
        TextileJob {
            id: id.to_owned(),
            profile_id: profile_id.to_owned(),
            functional_unit_id: functional_unit_id.to_owned(),
            setup_family: family.to_owned(),
            required_capabilities: required_capabilities
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            release_tick,
            due_tick,
            hard_finish_by_tick,
            processing_ticks,
            energy_units,
            water_units,
            process_evidence_ref: format!("fixture:process:{id}"),
            required_gates: vec![passing_gate()],
        }
    }

    fn machine(id: &str, capabilities: &[&str], initial_family: &str) -> Machine {
        Machine {
            id: id.to_owned(),
            capabilities: capabilities
                .iter()
                .map(|value| (*value).to_owned())
                .collect(),
            initial_setup_family: initial_family.to_owned(),
            available_tick: 0,
            capability_evidence_ref: format!("fixture:capability:{id}"),
        }
    }

    fn transition(machine_id: &str, from: &str, to: &str, ticks: u64) -> SetupTransition {
        SetupTransition {
            machine_id: machine_id.to_owned(),
            from_family: from.to_owned(),
            to_family: to.to_owned(),
            duration_ticks: ticks,
            energy_units: ticks,
            water_units: ticks.min(1),
            evidence_ref: format!("fixture:setup:{machine_id}:{from}:{to}"),
        }
    }

    fn sample_problem() -> ScheduleProblem {
        ScheduleProblem {
            id: "decorative-garment-sample".to_owned(),
            time_unit_id: "minute-tick-v0".to_owned(),
            energy_unit_id: "energy-unit-v0".to_owned(),
            water_unit_id: "water-unit-v0".to_owned(),
            energy_budget_units: 12,
            water_budget_units: 6,
            jobs: vec![
                job(
                    "everyday-shirt",
                    "everyday-v0",
                    "cotton",
                    &["sew"],
                    0,
                    4,
                    None,
                    2,
                    4,
                    2,
                ),
                job(
                    "beaded-occasion-garment",
                    "couture-decorative-v0",
                    "embellished",
                    &["hand-finish"],
                    0,
                    3,
                    Some(8),
                    3,
                    2,
                    1,
                ),
            ],
            machines: vec![
                machine("cell-a", &["sew", "hand-finish"], "cotton"),
                machine("cell-b", &["sew"], "cotton"),
            ],
            setup_transitions: vec![
                transition("cell-a", "cotton", "embellished", 2),
                transition("cell-a", "embellished", "cotton", 1),
            ],
        }
    }


    fn independently_validate_candidate(
        problem: &ScheduleProblem,
        candidate: &ScheduleCandidate,
    ) {
        assert_eq!(candidate.problem_id, problem.id);
        assert_eq!(candidate.time_unit_id, problem.time_unit_id);
        assert_eq!(candidate.energy_unit_id, problem.energy_unit_id);
        assert_eq!(candidate.water_unit_id, problem.water_unit_id);

        let jobs: BTreeMap<&str, &TextileJob> = problem
            .jobs
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect();
        let machines: BTreeMap<&str, &Machine> = problem
            .machines
            .iter()
            .map(|item| (item.id.as_str(), item))
            .collect();
        let mut seen_jobs = BTreeSet::new();
        let mut machine_finish: BTreeMap<String, u64> = BTreeMap::new();
        let mut machine_family: BTreeMap<String, String> = problem
            .machines
            .iter()
            .map(|machine| {
                (
                    machine.id.clone(),
                    machine.initial_setup_family.clone(),
                )
            })
            .collect();
        let mut energy = 0_u64;
        let mut water = 0_u64;
        let mut late_jobs = 0_u64;
        let mut tardiness = 0_u64;
        let mut makespan = 0_u64;
        let mut setup_ticks = 0_u64;

        for operation in &candidate.operations {
            assert!(seen_jobs.insert(operation.job_id.as_str()));
            let job = jobs.get(operation.job_id.as_str()).expect("known job");
            let machine = machines
                .get(operation.machine_id.as_str())
                .expect("known machine");
            assert_eq!(operation.profile_id, job.profile_id);
            assert_eq!(operation.functional_unit_id, job.functional_unit_id);
            assert!(job.required_capabilities.is_subset(&machine.capabilities));
            assert!(job.required_gates.iter().all(|gate| {
                match &gate.outcome {
                    GateOutcome::Pass { evidence_ref } => !evidence_ref.trim().is_empty(),
                    GateOutcome::Fail { .. } | GateOutcome::Unknown { .. } => false,
                }
            }));

            let expected_from = machine_family
                .get(&machine.id)
                .expect("machine has current setup family");
            assert_eq!(&operation.setup_from_family, expected_from);
            assert_eq!(operation.setup_to_family, job.setup_family);
            let (expected_setup, expected_energy, expected_water) =
                if operation.setup_from_family == operation.setup_to_family {
                    (0_u64, 0_u64, 0_u64)
                } else {
                    let transition = problem
                        .setup_transitions
                        .iter()
                        .find(|transition| {
                            transition.machine_id == machine.id
                                && transition.from_family == operation.setup_from_family
                                && transition.to_family == operation.setup_to_family
                        })
                        .expect("non-identical setup has declared transition evidence");
                    assert!(!transition.evidence_ref.trim().is_empty());
                    (
                        transition.duration_ticks,
                        transition.energy_units,
                        transition.water_units,
                    )
                };
            assert_eq!(operation.setup_duration_ticks, expected_setup);
            assert_eq!(operation.setup_energy_units, expected_energy);
            assert_eq!(operation.setup_water_units, expected_water);

            let previous_finish = machine_finish
                .get(&machine.id)
                .copied()
                .unwrap_or(machine.available_tick);
            assert_eq!(operation.setup_start_tick, previous_finish);
            assert_eq!(
                operation.setup_finish_tick,
                operation.setup_start_tick + expected_setup
            );
            assert_eq!(
                operation.process_start_tick,
                job.release_tick.max(operation.setup_finish_tick)
            );
            assert_eq!(
                operation.finish_tick,
                operation.process_start_tick + job.processing_ticks
            );
            assert!(job
                .hard_finish_by_tick
                .is_none_or(|latest| operation.finish_tick <= latest));

            machine_finish.insert(machine.id.clone(), operation.finish_tick);
            machine_family.insert(machine.id.clone(), job.setup_family.clone());
            energy += job.energy_units + expected_energy;
            water += job.water_units + expected_water;
            setup_ticks += expected_setup;
            makespan = makespan.max(operation.finish_tick);
            if operation.finish_tick > job.due_tick {
                late_jobs += 1;
                tardiness += operation.finish_tick - job.due_tick;
            }
        }

        assert_eq!(seen_jobs.len(), problem.jobs.len());
        assert_eq!(energy, candidate.metrics.energy_units);
        assert_eq!(water, candidate.metrics.water_units);
        assert_eq!(setup_ticks, candidate.metrics.setup_duration_ticks);
        assert_eq!(makespan, candidate.metrics.makespan_tick);
        assert_eq!(late_jobs, candidate.metrics.late_jobs);
        assert_eq!(tardiness, candidate.metrics.total_tardiness_ticks);
        assert!(energy <= problem.energy_budget_units);
        assert!(water <= problem.water_budget_units);
    }


    fn independent_tiny_oracle(problem: &ScheduleProblem) -> BTreeSet<ScheduleMetrics> {
        fn visit_orders(
            problem: &ScheduleProblem,
            prefix: &mut Vec<usize>,
            remaining: &mut Vec<usize>,
            results: &mut BTreeSet<ScheduleMetrics>,
        ) {
            if remaining.is_empty() {
                let mut states: Vec<MachineState> = problem
                    .machines
                    .iter()
                    .map(|machine| MachineState {
                        available_tick: machine.available_tick,
                        setup_family: machine.initial_setup_family.clone(),
                    })
                    .collect();
                let mut finishes = vec![None; problem.jobs.len()];
                visit_machine_assignments(
                    problem,
                    prefix,
                    0,
                    &mut states,
                    &mut finishes,
                    0,
                    0,
                    0,
                    results,
                );
                return;
            }

            for index in 0..remaining.len() {
                let job_index = remaining.remove(index);
                prefix.push(job_index);
                visit_orders(problem, prefix, remaining, results);
                prefix.pop();
                remaining.insert(index, job_index);
            }
        }

        fn visit_machine_assignments(
            problem: &ScheduleProblem,
            order: &[usize],
            position: usize,
            states: &mut [MachineState],
            finishes: &mut [Option<u64>],
            energy_units: u64,
            water_units: u64,
            setup_ticks: u64,
            results: &mut BTreeSet<ScheduleMetrics>,
        ) {
            if position == order.len() {
                if energy_units > problem.energy_budget_units
                    || water_units > problem.water_budget_units
                    || finishes.iter().any(|finish| finish.is_none())
                {
                    return;
                }
                let mut late_jobs = 0_u64;
                let mut tardiness_ticks = 0_u64;
                let mut makespan_tick = 0_u64;
                for (index, job) in problem.jobs.iter().enumerate() {
                    let finish = finishes[index].expect("all jobs scheduled");
                    makespan_tick = makespan_tick.max(finish);
                    if finish > job.due_tick {
                        late_jobs += 1;
                        tardiness_ticks += finish - job.due_tick;
                    }
                }
                results.insert(ScheduleMetrics {
                    late_jobs,
                    total_tardiness_ticks: tardiness_ticks,
                    makespan_tick,
                    energy_units,
                    water_units,
                    setup_duration_ticks: setup_ticks,
                });
                return;
            }

            let job_index = order[position];
            let job = &problem.jobs[job_index];
            for (machine_index, machine) in problem.machines.iter().enumerate() {
                if !job.required_capabilities.is_subset(&machine.capabilities) {
                    continue;
                }
                let state = &states[machine_index];
                let (setup_duration, setup_energy, setup_water) =
                    if state.setup_family == job.setup_family {
                        (0_u64, 0_u64, 0_u64)
                    } else {
                        let Some(edge) = problem.setup_transitions.iter().find(|edge| {
                            edge.machine_id == machine.id
                                && edge.from_family == state.setup_family
                                && edge.to_family == job.setup_family
                        }) else {
                            continue;
                        };
                        (edge.duration_ticks, edge.energy_units, edge.water_units)
                    };

                let next_energy = energy_units + job.energy_units + setup_energy;
                let next_water = water_units + job.water_units + setup_water;
                let next_setup_ticks = setup_ticks + setup_duration;
                if next_energy > problem.energy_budget_units
                    || next_water > problem.water_budget_units
                {
                    continue;
                }
                let setup_finish = state.available_tick + setup_duration;
                let process_start = job.release_tick.max(setup_finish);
                let finish = process_start + job.processing_ticks;
                if job.hard_finish_by_tick.is_some_and(|latest| finish > latest) {
                    continue;
                }

                let old_state = states[machine_index].clone();
                let old_finish = finishes[job_index];
                states[machine_index] = MachineState {
                    available_tick: finish,
                    setup_family: job.setup_family.clone(),
                };
                finishes[job_index] = Some(finish);
                visit_machine_assignments(
                    problem,
                    order,
                    position + 1,
                    states,
                    finishes,
                    next_energy,
                    next_water,
                    next_setup_ticks,
                    results,
                );
                states[machine_index] = old_state;
                finishes[job_index] = old_finish;
            }
        }

        let mut remaining: Vec<usize> = (0..problem.jobs.len()).collect();
        let mut prefix = Vec::with_capacity(problem.jobs.len());
        let mut results = BTreeSet::new();
        visit_orders(problem, &mut prefix, &mut remaining, &mut results);
        results
    }

    fn oracle_dominates(left: &ScheduleMetrics, right: &ScheduleMetrics) -> bool {
        let all_no_worse = left.late_jobs <= right.late_jobs
            && left.total_tardiness_ticks <= right.total_tardiness_ticks
            && left.makespan_tick <= right.makespan_tick
            && left.energy_units <= right.energy_units
            && left.water_units <= right.water_units
            && left.setup_duration_ticks <= right.setup_duration_ticks;
        let one_better = left.late_jobs < right.late_jobs
            || left.total_tardiness_ticks < right.total_tardiness_ticks
            || left.makespan_tick < right.makespan_tick
            || left.energy_units < right.energy_units
            || left.water_units < right.water_units
            || left.setup_duration_ticks < right.setup_duration_ticks;
        all_no_worse && one_better
    }

    #[test]
    fn exhaustive_solver_returns_feasible_frontier_for_decorative_garment() {
        let problem = sample_problem();
        let report = solve_exhaustive(&problem).expect("small fixture should be feasible");

        assert!(report.complete_schedules > 0);
        assert!(!report.pareto_frontier.is_empty());
        assert!(report.pareto_frontier.iter().all(|candidate| {
            candidate.operations.len() == problem.jobs.len()
                && candidate.metrics.energy_units <= problem.energy_budget_units
                && candidate.metrics.water_units <= problem.water_budget_units
        }));
        assert!(report.pareto_frontier.iter().any(|candidate| {
            candidate.operations.iter().any(|operation| {
                operation.profile_id == "couture-decorative-v0"
                    && operation.finish_tick <= 8
            })
        }));
        for candidate in &report.pareto_frontier {
            independently_validate_candidate(&problem, candidate);
        }
        for (index, left) in report.pareto_frontier.iter().enumerate() {
            for right in report.pareto_frontier.iter().skip(index + 1) {
                assert!(!dominates(&left.metrics, &right.metrics));
                assert!(!dominates(&right.metrics, &left.metrics));
            }
        }
    }

    #[test]
    fn failed_gate_blocks_scheduling_instead_of_being_traded_away() {
        let mut problem = sample_problem();
        problem.jobs[0].required_gates[0].outcome = GateOutcome::Fail {
            evidence_ref: "fixture:failed-readiness-check".to_owned(),
        };

        assert!(matches!(
            solve_exhaustive(&problem),
            Err(ScheduleError::BlockedJob { .. })
        ));
    }

    #[test]
    fn unknown_gate_is_not_treated_as_pass() {
        let mut problem = sample_problem();
        problem.jobs[0].required_gates[0].outcome = GateOutcome::Unknown {
            reason: "conservation review is incomplete".to_owned(),
        };

        assert!(matches!(
            dispatch_schedule(&problem, DispatchRule::EarliestDue),
            Err(ScheduleError::UnverifiedJob { .. })
        ));
    }

    #[test]
    fn missing_changeover_evidence_is_not_a_zero_cost_setup() {
        let mut problem = sample_problem();
        problem.setup_transitions.clear();

        assert!(matches!(
            solve_exhaustive(&problem),
            Err(ScheduleError::MissingSetupTransition { .. })
        ));
    }

    #[test]
    fn exact_search_rejects_instances_above_its_declared_bound() {
        let mut problem = sample_problem();
        problem.energy_budget_units = 1_000;
        problem.water_budget_units = 1_000;
        let template = problem.jobs[0].clone();
        for index in 0..EXHAUSTIVE_MAX_JOBS {
            let mut extra = template.clone();
            extra.id = format!("extra-{index}");
            problem.jobs.push(extra);
        }

        assert!(matches!(
            solve_exhaustive(&problem),
            Err(ScheduleError::TooManyJobsForExhaustive { .. })
        ));
    }

    #[test]
    fn dispatch_baselines_are_deterministic_and_respect_hard_finish_windows() {
        let problem = sample_problem();
        for rule in [
            DispatchRule::EarliestDue,
            DispatchRule::LowestSetup,
            DispatchRule::LowestEnergy,
        ] {
            let first = dispatch_schedule(&problem, rule).expect("fixture dispatch should work");
            let second = dispatch_schedule(&problem, rule).expect("same fixture should work");
            assert_eq!(first, second);
            independently_validate_candidate(&problem, &first);
            assert!(first.operations.iter().all(|operation| {
                problem
                    .jobs
                    .iter()
                    .find(|job| job.id == operation.job_id)
                    .and_then(|job| job.hard_finish_by_tick)
                    .is_none_or(|latest| operation.finish_tick <= latest)
            }));
        }
    }

    #[test]
    fn input_order_does_not_change_exact_report() {
        let problem = sample_problem();
        let first = solve_exhaustive(&problem).expect("fixture is feasible");
        let mut reversed = problem.clone();
        reversed.jobs.reverse();
        reversed.machines.reverse();
        reversed.setup_transitions.reverse();
        let second = solve_exhaustive(&reversed).expect("reordered fixture is feasible");
        assert_eq!(first, second);
    }

    #[test]
    fn hard_finish_window_cannot_be_traded_for_a_better_objective() {
        let mut problem = sample_problem();
        problem.jobs.iter_mut()
            .find(|job| job.id == "beaded-occasion-garment")
            .expect("decorative job exists")
            .hard_finish_by_tick = Some(2);

        assert!(matches!(
            solve_exhaustive(&problem),
            Err(ScheduleError::NoFeasibleSchedule)
        ));
    }


    #[test]
    fn exhaustive_frontier_matches_independent_permutation_and_assignment_oracle() {
        let problem = sample_problem();
        let report = solve_exhaustive(&problem).expect("fixture is feasible");
        let oracle_scores = independent_tiny_oracle(&problem);
        assert!(!oracle_scores.is_empty());

        let expected_frontier: BTreeSet<ScheduleMetrics> = oracle_scores
            .iter()
            .filter(|candidate| {
                !oracle_scores.iter().any(|other| {
                    other != *candidate && oracle_dominates(other, candidate)
                })
            })
            .cloned()
            .collect();
        let actual_frontier: BTreeSet<ScheduleMetrics> = report
            .pareto_frontier
            .iter()
            .map(|candidate| candidate.metrics.clone())
            .collect();
        assert_eq!(actual_frontier, expected_frontier);

        let expected_optima = ObjectiveOptima {
            minimum_late_jobs: oracle_scores.iter().map(|score| score.late_jobs).min().unwrap(),
            minimum_total_tardiness_ticks: oracle_scores
                .iter()
                .map(|score| score.total_tardiness_ticks)
                .min()
                .unwrap(),
            minimum_makespan_tick: oracle_scores
                .iter()
                .map(|score| score.makespan_tick)
                .min()
                .unwrap(),
            minimum_energy_units: oracle_scores
                .iter()
                .map(|score| score.energy_units)
                .min()
                .unwrap(),
            minimum_water_units: oracle_scores
                .iter()
                .map(|score| score.water_units)
                .min()
                .unwrap(),
            minimum_setup_duration_ticks: oracle_scores
                .iter()
                .map(|score| score.setup_duration_ticks)
                .min()
                .unwrap(),
        };
        assert_eq!(report.objective_optima, expected_optima);
    }


    #[test]
    fn hard_resource_budget_cannot_be_traded_for_delivery() {
        let mut problem = sample_problem();
        // The jobs consume six energy units before any setup. The decorative
        // changeover needs additional evidenced energy, so this budget is infeasible.
        problem.energy_budget_units = 6;

        assert!(matches!(
            solve_exhaustive(&problem),
            Err(ScheduleError::NoFeasibleSchedule)
        ));
        assert!(matches!(
            dispatch_schedule(&problem, DispatchRule::EarliestDue),
            Err(ScheduleError::NoFeasibleDispatch {
                rule: DispatchRule::EarliestDue
            })
        ));
    }


    #[test]
    fn archive_display_job_requires_conservation_capability_and_retains_service_unit() {
        let problem = ScheduleProblem {
            id: "archive-display-treatment".to_owned(),
            time_unit_id: "minute-tick-v0".to_owned(),
            energy_unit_id: "energy-unit-v0".to_owned(),
            water_unit_id: "water-unit-v0".to_owned(),
            energy_budget_units: 2,
            water_budget_units: 1,
            jobs: vec![TextileJob {
                id: "archive-support-preparation".to_owned(),
                profile_id: "archive-display-v0".to_owned(),
                functional_unit_id: "conservation-year-v0".to_owned(),
                setup_family: "conservation".to_owned(),
                required_capabilities: ["conservation-safe", "low-light"]
                    .iter()
                    .map(|value| (*value).to_owned())
                    .collect(),
                release_tick: 20,
                due_tick: 30,
                hard_finish_by_tick: Some(25),
                processing_ticks: 4,
                energy_units: 2,
                water_units: 1,
                process_evidence_ref: "fixture:archive-procedure-v1".to_owned(),
                required_gates: vec![GateEvidence {
                    id: "conservation-authorization".to_owned(),
                    outcome: GateOutcome::Pass {
                        evidence_ref: "fixture:conservator-authorization".to_owned(),
                    },
                }],
            }],
            machines: vec![
                machine("general-line", &["sew"], "standard"),
                machine("conservation-bench", &["conservation-safe", "low-light"], "conservation"),
            ],
            setup_transitions: vec![],
        };

        let exact = solve_exhaustive(&problem).expect("qualified conservation job is feasible");
        assert_eq!(exact.complete_schedules, 1);
        assert_eq!(exact.pareto_frontier.len(), 1);
        independently_validate_candidate(&problem, &exact.pareto_frontier[0]);
        let operation = &exact.pareto_frontier[0].operations[0];
        assert_eq!(operation.machine_id, "conservation-bench");
        assert_eq!(operation.profile_id, "archive-display-v0");
        assert_eq!(operation.functional_unit_id, "conservation-year-v0");

        let dispatched = dispatch_schedule(&problem, DispatchRule::EarliestDue)
            .expect("the same hard constraints admit deterministic dispatch");
        independently_validate_candidate(&problem, &dispatched);
        assert_eq!(dispatched.operations[0].machine_id, "conservation-bench");
    }

}
