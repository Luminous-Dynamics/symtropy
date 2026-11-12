//! Deterministic discrete-event feasibility replay for finite shipyard tasks.
//!
//! This crate checks task precedence, minimum lags and simultaneous resource
//! capacity. It does not model a real shipyard calendar, travel geometry,
//! stochastic breakdowns, welding physics or production quality.

use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceCapacity {
    pub resource_id: String,
    pub capacity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceDemand {
    pub resource_id: String,
    pub quantity: u32,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Task {
    pub task_id: String,
    pub duration_ticks: u64,
    pub priority: u32,
    pub demands: Vec<ResourceDemand>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Dependency {
    pub predecessor_id: String,
    pub successor_id: String,
    pub minimum_lag_ticks: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleInput {
    pub resources: Vec<ResourceCapacity>,
    pub tasks: Vec<Task>,
    pub dependencies: Vec<Dependency>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EventKind {
    Completed,
    Started,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventRecord {
    pub tick: u64,
    pub kind: EventKind,
    pub task_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TaskTiming {
    pub task_id: String,
    pub start_tick: u64,
    pub finish_tick: u64,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScheduleResult {
    pub task_timings: Vec<TaskTiming>,
    pub events: Vec<EventRecord>,
    pub makespan_ticks: u64,
    pub claim_ceiling: &'static str,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ScheduleError {
    EmptyIdentity,
    ZeroResourceCapacity,
    DuplicateResourceIdentity,
    DuplicateTaskIdentity,
    ZeroTaskDuration,
    TaskHasNoResourceDemand,
    DuplicateTaskResourceDemand,
    ZeroResourceDemand,
    UnknownResource,
    ResourceDemandExceedsCapacity,
    UnknownDependencyTask,
    SelfDependency,
    DuplicateDependency,
    PrecedenceCycle,
    TimeOverflow,
    ResourceAccountingOverflow,
    Deadlock,
}

fn is_empty(value: &str) -> bool {
    value.trim().is_empty()
}

fn validate(input: &ScheduleInput) -> Result<(), ScheduleError> {
    let mut capacities = BTreeMap::new();
    for resource in &input.resources {
        if is_empty(&resource.resource_id) {
            return Err(ScheduleError::EmptyIdentity);
        }
        if resource.capacity == 0 {
            return Err(ScheduleError::ZeroResourceCapacity);
        }
        if capacities
            .insert(resource.resource_id.as_str(), resource.capacity)
            .is_some()
        {
            return Err(ScheduleError::DuplicateResourceIdentity);
        }
    }

    let mut task_ids = HashSet::new();
    for task in &input.tasks {
        if is_empty(&task.task_id) {
            return Err(ScheduleError::EmptyIdentity);
        }
        if !task_ids.insert(task.task_id.as_str()) {
            return Err(ScheduleError::DuplicateTaskIdentity);
        }
        if task.duration_ticks == 0 {
            return Err(ScheduleError::ZeroTaskDuration);
        }
        if task.demands.is_empty() {
            return Err(ScheduleError::TaskHasNoResourceDemand);
        }
        let mut demanded_resources = HashSet::new();
        for demand in &task.demands {
            if is_empty(&demand.resource_id) {
                return Err(ScheduleError::EmptyIdentity);
            }
            if !demanded_resources.insert(demand.resource_id.as_str()) {
                return Err(ScheduleError::DuplicateTaskResourceDemand);
            }
            if demand.quantity == 0 {
                return Err(ScheduleError::ZeroResourceDemand);
            }
            let Some(capacity) = capacities.get(demand.resource_id.as_str()) else {
                return Err(ScheduleError::UnknownResource);
            };
            if demand.quantity > *capacity {
                return Err(ScheduleError::ResourceDemandExceedsCapacity);
            }
        }
    }

    let mut dependency_pairs = HashSet::new();
    for dependency in &input.dependencies {
        if !task_ids.contains(dependency.predecessor_id.as_str())
            || !task_ids.contains(dependency.successor_id.as_str())
        {
            return Err(ScheduleError::UnknownDependencyTask);
        }
        if dependency.predecessor_id == dependency.successor_id {
            return Err(ScheduleError::SelfDependency);
        }
        if !dependency_pairs.insert((
            dependency.predecessor_id.as_str(),
            dependency.successor_id.as_str(),
        )) {
            return Err(ScheduleError::DuplicateDependency);
        }
    }

    validate_acyclic(input)?;
    Ok(())
}

fn validate_acyclic(input: &ScheduleInput) -> Result<(), ScheduleError> {
    let mut indegree: HashMap<&str, usize> = input
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), 0))
        .collect();
    let mut successors: HashMap<&str, Vec<&str>> = HashMap::new();

    for dependency in &input.dependencies {
        *indegree
            .get_mut(dependency.successor_id.as_str())
            .expect("validated successor identity") += 1;
        successors
            .entry(dependency.predecessor_id.as_str())
            .or_default()
            .push(dependency.successor_id.as_str());
    }

    let mut ready: BTreeSet<&str> = indegree
        .iter()
        .filter_map(|(task_id, degree)| (*degree == 0).then_some(*task_id))
        .collect();
    let mut visited = 0usize;

    while let Some(task_id) = ready.pop_first() {
        visited += 1;
        if let Some(children) = successors.get(task_id) {
            for child in children {
                let degree = indegree
                    .get_mut(child)
                    .expect("validated successor identity");
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(child);
                }
            }
        }
    }

    if visited == input.tasks.len() {
        Ok(())
    } else {
        Err(ScheduleError::PrecedenceCycle)
    }
}

fn dependency_ready_tick(
    task_id: &str,
    dependencies: &[Dependency],
    timings: &HashMap<String, TaskTiming>,
) -> Result<Option<u64>, ScheduleError> {
    let mut ready_tick = 0u64;
    for dependency in dependencies
        .iter()
        .filter(|dependency| dependency.successor_id == task_id)
    {
        let Some(predecessor) = timings.get(&dependency.predecessor_id) else {
            return Ok(None);
        };
        let release_tick = predecessor
            .finish_tick
            .checked_add(dependency.minimum_lag_ticks)
            .ok_or(ScheduleError::TimeOverflow)?;
        ready_tick = ready_tick.max(release_tick);
    }
    Ok(Some(ready_tick))
}

fn can_acquire(
    task: &Task,
    capacities: &BTreeMap<String, u32>,
    used: &BTreeMap<String, u32>,
) -> bool {
    task.demands.iter().all(|demand| {
        let capacity = capacities
            .get(&demand.resource_id)
            .copied()
            .unwrap_or_default();
        let in_use = used.get(&demand.resource_id).copied().unwrap_or_default();
        in_use
            .checked_add(demand.quantity)
            .is_some_and(|total| total <= capacity)
    })
}

fn acquire(
    task: &Task,
    used: &mut BTreeMap<String, u32>,
) -> Result<(), ScheduleError> {
    for demand in &task.demands {
        let in_use = used.entry(demand.resource_id.clone()).or_default();
        *in_use = in_use
            .checked_add(demand.quantity)
            .ok_or(ScheduleError::ResourceAccountingOverflow)?;
    }
    Ok(())
}

fn release(
    task: &Task,
    used: &mut BTreeMap<String, u32>,
) -> Result<(), ScheduleError> {
    for demand in &task.demands {
        let in_use = used
            .get_mut(&demand.resource_id)
            .ok_or(ScheduleError::ResourceAccountingOverflow)?;
        *in_use = in_use
            .checked_sub(demand.quantity)
            .ok_or(ScheduleError::ResourceAccountingOverflow)?;
    }
    Ok(())
}

/// Run a deterministic, non-preemptive event simulation.
///
/// Dispatch is priority-first, then task-ID lexical order. A ready task starts
/// only when every requested resource can be acquired simultaneously. Blocked
/// tasks do not prevent a lower-priority independent task from starting.
/// Completion events at a tick are processed before new start events at that
/// same tick. All time values are symbolic integer ticks.
pub fn simulate(input: &ScheduleInput) -> Result<ScheduleResult, ScheduleError> {
    validate(input)?;

    let capacities: BTreeMap<String, u32> = input
        .resources
        .iter()
        .map(|resource| (resource.resource_id.clone(), resource.capacity))
        .collect();
    let tasks_by_id: HashMap<&str, &Task> = input
        .tasks
        .iter()
        .map(|task| (task.task_id.as_str(), task))
        .collect();

    let mut now = 0u64;
    let mut completed: HashSet<String> = HashSet::new();
    let mut timings: HashMap<String, TaskTiming> = HashMap::new();
    let mut running: HashSet<String> = HashSet::new();
    let mut used: BTreeMap<String, u32> = BTreeMap::new();
    let mut events = Vec::new();

    while completed.len() < input.tasks.len() {
        let mut finishing: Vec<String> = running
            .iter()
            .filter_map(|task_id| {
                timings
                    .get(task_id)
                    .filter(|timing| timing.finish_tick == now)
                    .map(|_| task_id.clone())
            })
            .collect();
        finishing.sort();

        for task_id in finishing {
            let task = tasks_by_id
                .get(task_id.as_str())
                .copied()
                .ok_or(ScheduleError::UnknownDependencyTask)?;
            release(task, &mut used)?;
            running.remove(&task_id);
            completed.insert(task_id.clone());
            events.push(EventRecord {
                tick: now,
                kind: EventKind::Completed,
                task_id,
            });
        }

        loop {
            let mut ready: Vec<&Task> = input
                .tasks
                .iter()
                .filter(|task| !timings.contains_key(&task.task_id))
                .filter_map(|task| {
                    match dependency_ready_tick(&task.task_id, &input.dependencies, &timings) {
                        Ok(Some(tick)) if tick <= now => Some(Ok(task)),
                        Ok(Some(_)) | Ok(None) => None,
                        Err(error) => Some(Err(error)),
                    }
                })
                .collect::<Result<Vec<_>, _>>()?;

            ready.sort_by(|left, right| {
                left.priority
                    .cmp(&right.priority)
                    .then_with(|| left.task_id.cmp(&right.task_id))
            });

            let Some(task) = ready.into_iter().find(|task| can_acquire(task, &capacities, &used))
            else {
                break;
            };

            let finish_tick = now
                .checked_add(task.duration_ticks)
                .ok_or(ScheduleError::TimeOverflow)?;
            acquire(task, &mut used)?;
            running.insert(task.task_id.clone());
            timings.insert(
                task.task_id.clone(),
                TaskTiming {
                    task_id: task.task_id.clone(),
                    start_tick: now,
                    finish_tick,
                },
            );
            events.push(EventRecord {
                tick: now,
                kind: EventKind::Started,
                task_id: task.task_id.clone(),
            });
        }

        if completed.len() == input.tasks.len() {
            break;
        }

        let next_completion = running
            .iter()
            .filter_map(|task_id| timings.get(task_id).map(|timing| timing.finish_tick))
            .min();

        let mut next_dependency_release = None;
        for task in input
            .tasks
            .iter()
            .filter(|task| !timings.contains_key(&task.task_id))
        {
            if let Some(release_tick) =
                dependency_ready_tick(&task.task_id, &input.dependencies, &timings)?
            {
                if release_tick > now {
                    next_dependency_release = Some(
                        next_dependency_release.map_or(release_tick, |current: u64| {
                            current.min(release_tick)
                        }),
                    );
                }
            }
        }

        let Some(next_tick) = [next_completion, next_dependency_release]
            .into_iter()
            .flatten()
            .min()
        else {
            return Err(ScheduleError::Deadlock);
        };

        if next_tick <= now {
            return Err(ScheduleError::Deadlock);
        }
        now = next_tick;
    }

    let mut task_timings: Vec<TaskTiming> = timings.into_values().collect();
    task_timings.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    let makespan_ticks = task_timings
        .iter()
        .map(|timing| timing.finish_tick)
        .max()
        .unwrap_or_default();

    Ok(ScheduleResult {
        task_timings,
        events,
        makespan_ticks,
        claim_ceiling: "DETERMINISTIC_SYMBOLIC_RESOURCE_SCHEDULE_ONLY",
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn resource(resource_id: &str, capacity: u32) -> ResourceCapacity {
        ResourceCapacity {
            resource_id: resource_id.to_owned(),
            capacity,
        }
    }

    fn task(
        task_id: &str,
        duration_ticks: u64,
        priority: u32,
        demands: &[(&str, u32)],
    ) -> Task {
        Task {
            task_id: task_id.to_owned(),
            duration_ticks,
            priority,
            demands: demands
                .iter()
                .map(|(resource_id, quantity)| ResourceDemand {
                    resource_id: (*resource_id).to_owned(),
                    quantity: *quantity,
                })
                .collect(),
        }
    }

    fn input(
        resources: Vec<ResourceCapacity>,
        tasks: Vec<Task>,
        dependencies: Vec<Dependency>,
    ) -> ScheduleInput {
        ScheduleInput {
            resources,
            tasks,
            dependencies,
        }
    }

    fn timing(result: &ScheduleResult, task_id: &str) -> &TaskTiming {
        result
            .task_timings
            .iter()
            .find(|timing| timing.task_id == task_id)
            .expect("task timing exists")
    }

    #[test]
    fn independent_crane_task_overlaps_single_robot_sequence() {
        let scenario = input(
            vec![resource("robot", 1), resource("crane", 1)],
            vec![
                task("A", 5, 1, &[("robot", 1)]),
                task("B", 3, 2, &[("robot", 1)]),
                task("C", 4, 3, &[("crane", 1)]),
            ],
            vec![],
        );
        let result = simulate(&scenario).expect("schedule is feasible");
        assert_eq!(timing(&result, "A").start_tick, 0);
        assert_eq!(timing(&result, "A").finish_tick, 5);
        assert_eq!(timing(&result, "B").start_tick, 5);
        assert_eq!(timing(&result, "C").start_tick, 0);
        assert_eq!(result.makespan_ticks, 8);
    }

    #[test]
    fn minimum_precedence_lag_delays_successor_release() {
        let scenario = input(
            vec![resource("robot", 2)],
            vec![
                task("A", 3, 1, &[("robot", 1)]),
                task("B", 2, 1, &[("robot", 1)]),
            ],
            vec![Dependency {
                predecessor_id: "A".to_owned(),
                successor_id: "B".to_owned(),
                minimum_lag_ticks: 2,
            }],
        );
        let result = simulate(&scenario).expect("schedule is feasible");
        assert_eq!(timing(&result, "A").finish_tick, 3);
        assert_eq!(timing(&result, "B").start_tick, 5);
        assert_eq!(result.makespan_ticks, 7);
    }

    #[test]
    fn multi_resource_acquisition_is_atomic() {
        let scenario = input(
            vec![resource("robot", 1), resource("crane", 1)],
            vec![
                task("A", 4, 1, &[("robot", 1), ("crane", 1)]),
                task("B", 2, 2, &[("robot", 1)]),
            ],
            vec![],
        );
        let result = simulate(&scenario).expect("schedule is feasible");
        assert_eq!(timing(&result, "A").start_tick, 0);
        assert_eq!(timing(&result, "B").start_tick, 4);
        assert_eq!(result.makespan_ticks, 6);
    }

    #[test]
    fn resource_capacity_two_allows_two_tasks_but_not_three() {
        let scenario = input(
            vec![resource("operator", 2)],
            vec![
                task("A", 4, 1, &[("operator", 1)]),
                task("B", 4, 2, &[("operator", 1)]),
                task("C", 4, 3, &[("operator", 1)]),
            ],
            vec![],
        );
        let result = simulate(&scenario).expect("schedule is feasible");
        assert_eq!(timing(&result, "A").start_tick, 0);
        assert_eq!(timing(&result, "B").start_tick, 0);
        assert_eq!(timing(&result, "C").start_tick, 4);
        assert_eq!(result.makespan_ticks, 8);
    }

    #[test]
    fn priority_then_task_id_is_the_deterministic_tie_break() {
        let scenario = input(
            vec![resource("robot", 1)],
            vec![
                task("B", 2, 1, &[("robot", 1)]),
                task("A", 2, 1, &[("robot", 1)]),
            ],
            vec![],
        );
        let result = simulate(&scenario).expect("schedule is feasible");
        assert_eq!(timing(&result, "A").start_tick, 0);
        assert_eq!(timing(&result, "B").start_tick, 2);
    }

    #[test]
    fn simultaneous_completions_are_logged_before_new_starts() {
        let scenario = input(
            vec![resource("robot", 2)],
            vec![
                task("A", 2, 1, &[("robot", 1)]),
                task("B", 2, 2, &[("robot", 1)]),
                task("C", 1, 3, &[("robot", 2)]),
            ],
            vec![],
        );
        let result = simulate(&scenario).expect("schedule is feasible");
        let at_tick_two: Vec<(EventKind, &str)> = result
            .events
            .iter()
            .filter(|event| event.tick == 2)
            .map(|event| (event.kind, event.task_id.as_str()))
            .collect();
        assert_eq!(
            at_tick_two,
            vec![
                (EventKind::Completed, "A"),
                (EventKind::Completed, "B"),
                (EventKind::Started, "C"),
            ]
        );
    }

    #[test]
    fn repeated_runs_are_byte_stable_at_the_structured_result_level() {
        let scenario = input(
            vec![resource("robot", 1), resource("crane", 1)],
            vec![
                task("A", 5, 1, &[("robot", 1)]),
                task("B", 3, 2, &[("robot", 1)]),
                task("C", 4, 3, &[("crane", 1)]),
            ],
            vec![],
        );
        assert_eq!(simulate(&scenario), simulate(&scenario));
    }

    #[test]
    fn input_order_does_not_change_schedule_when_priority_and_ids_are_fixed() {
        let first = input(
            vec![resource("robot", 1), resource("crane", 1)],
            vec![
                task("A", 5, 1, &[("robot", 1)]),
                task("B", 3, 2, &[("robot", 1)]),
                task("C", 4, 3, &[("crane", 1)]),
            ],
            vec![],
        );
        let second = input(
            vec![resource("crane", 1), resource("robot", 1)],
            vec![
                task("C", 4, 3, &[("crane", 1)]),
                task("B", 3, 2, &[("robot", 1)]),
                task("A", 5, 1, &[("robot", 1)]),
            ],
            vec![],
        );
        assert_eq!(simulate(&first), simulate(&second));
    }

    #[test]
    fn cycle_is_rejected_before_event_replay() {
        let scenario = input(
            vec![resource("robot", 1)],
            vec![
                task("A", 1, 1, &[("robot", 1)]),
                task("B", 1, 2, &[("robot", 1)]),
            ],
            vec![
                Dependency {
                    predecessor_id: "A".to_owned(),
                    successor_id: "B".to_owned(),
                    minimum_lag_ticks: 0,
                },
                Dependency {
                    predecessor_id: "B".to_owned(),
                    successor_id: "A".to_owned(),
                    minimum_lag_ticks: 0,
                },
            ],
        );
        assert_eq!(simulate(&scenario), Err(ScheduleError::PrecedenceCycle));
    }

    #[test]
    fn unknown_dependency_task_is_rejected() {
        let scenario = input(
            vec![resource("robot", 1)],
            vec![task("A", 1, 1, &[("robot", 1)])],
            vec![Dependency {
                predecessor_id: "MISSING".to_owned(),
                successor_id: "A".to_owned(),
                minimum_lag_ticks: 0,
            }],
        );
        assert_eq!(simulate(&scenario), Err(ScheduleError::UnknownDependencyTask));
    }

    #[test]
    fn task_demand_above_resource_capacity_is_rejected() {
        let scenario = input(
            vec![resource("robot", 1)],
            vec![task("A", 1, 1, &[("robot", 2)])],
            vec![],
        );
        assert_eq!(
            simulate(&scenario),
            Err(ScheduleError::ResourceDemandExceedsCapacity)
        );
    }

    #[test]
    fn duplicate_task_identity_is_rejected() {
        let scenario = input(
            vec![resource("robot", 2)],
            vec![
                task("A", 1, 1, &[("robot", 1)]),
                task("A", 2, 2, &[("robot", 1)]),
            ],
            vec![],
        );
        assert_eq!(simulate(&scenario), Err(ScheduleError::DuplicateTaskIdentity));
    }

    #[test]
    fn duplicate_resource_identity_is_rejected() {
        let scenario = input(
            vec![resource("robot", 1), resource("robot", 2)],
            vec![task("A", 1, 1, &[("robot", 1)])],
            vec![],
        );
        assert_eq!(
            simulate(&scenario),
            Err(ScheduleError::DuplicateResourceIdentity)
        );
    }

    #[test]
    fn zero_duration_is_rejected() {
        let scenario = input(
            vec![resource("robot", 1)],
            vec![task("A", 0, 1, &[("robot", 1)])],
            vec![],
        );
        assert_eq!(simulate(&scenario), Err(ScheduleError::ZeroTaskDuration));
    }

    #[test]
    fn duplicate_resource_demand_is_rejected() {
        let scenario = input(
            vec![resource("robot", 2)],
            vec![task("A", 1, 1, &[("robot", 1), ("robot", 1)])],
            vec![],
        );
        assert_eq!(
            simulate(&scenario),
            Err(ScheduleError::DuplicateTaskResourceDemand)
        );
    }

    #[test]
    fn time_overflow_is_rejected() {
        let scenario = input(
            vec![resource("robot", 1)],
            vec![
                task("A", u64::MAX, 1, &[("robot", 1)]),
                task("B", 1, 2, &[("robot", 1)]),
            ],
            vec![],
        );
        assert_eq!(simulate(&scenario), Err(ScheduleError::TimeOverflow));
    }

    #[test]
    fn dependency_lag_overflow_is_rejected() {
        let scenario = input(
            vec![resource("robot", 1)],
            vec![
                task("A", 1, 1, &[("robot", 1)]),
                task("B", 1, 2, &[("robot", 1)]),
            ],
            vec![Dependency {
                predecessor_id: "A".to_owned(),
                successor_id: "B".to_owned(),
                minimum_lag_ticks: u64::MAX,
            }],
        );
        assert_eq!(simulate(&scenario), Err(ScheduleError::TimeOverflow));
    }

    #[test]
    fn task_never_starts_before_every_predecessor_lag_is_satisfied() {
        let scenario = input(
            vec![resource("robot", 3)],
            vec![
                task("A", 3, 1, &[("robot", 1)]),
                task("B", 5, 2, &[("robot", 1)]),
                task("C", 1, 3, &[("robot", 1)]),
            ],
            vec![
                Dependency {
                    predecessor_id: "A".to_owned(),
                    successor_id: "C".to_owned(),
                    minimum_lag_ticks: 2,
                },
                Dependency {
                    predecessor_id: "B".to_owned(),
                    successor_id: "C".to_owned(),
                    minimum_lag_ticks: 1,
                },
            ],
        );
        let result = simulate(&scenario).expect("schedule is feasible");
        assert_eq!(timing(&result, "C").start_tick, 6);
    }
}
