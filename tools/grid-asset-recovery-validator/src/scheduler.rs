use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ResourceRequirement {
    pub resource_id: String,
    pub units: u32,
}

#[derive(Clone, Debug)]
pub struct TaskSpec {
    pub event_id: String,
    pub task_id: String,
    pub depends_on: Vec<String>,
    pub release_hour: f64,
    pub duration_hours: f64,
    pub resource_requirements: Vec<ResourceRequirement>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ScheduledTask {
    pub event_id: String,
    pub task_id: String,
    pub start_hour: f64,
    pub end_hour: f64,
}

#[derive(Clone, Debug)]
struct ResourceInterval {
    start: f64,
    end: f64,
    units: u32,
}

fn key(event_id: &str, task_id: &str) -> String {
    format!("{event_id}::{task_id}")
}

fn earliest_resource_slot(
    ready_hour: f64,
    duration_hours: f64,
    requirements: &[ResourceRequirement],
    capacities: &BTreeMap<String, u32>,
    allocations: &BTreeMap<String, Vec<ResourceInterval>>,
) -> Result<f64, String> {
    if requirements.is_empty() {
        return Ok(ready_hour);
    }

    let mut seen = BTreeSet::new();
    let mut candidates = vec![ready_hour];
    for requirement in requirements {
        if !seen.insert(requirement.resource_id.as_str()) {
            return Err(format!(
                "duplicate resource requirement {}",
                requirement.resource_id
            ));
        }
        let Some(capacity) = capacities.get(&requirement.resource_id).copied() else {
            return Err(format!("unknown resource {}", requirement.resource_id));
        };
        if requirement.units == 0 || requirement.units > capacity {
            return Err(format!(
                "resource {} requires {} units but capacity is {}",
                requirement.resource_id, requirement.units, capacity
            ));
        }
        if let Some(intervals) = allocations.get(&requirement.resource_id) {
            candidates.extend(
                intervals
                    .iter()
                    .filter(|item| item.end >= ready_hour)
                    .map(|item| item.end),
            );
        }
    }
    candidates.sort_by(f64::total_cmp);
    candidates.dedup_by(|a, b| (*a - *b).abs() <= 1e-9);

    for candidate in candidates.iter().copied() {
        let finish = candidate + duration_hours;
        let mut all_feasible = true;

        for requirement in requirements {
            let intervals = allocations
                .get(&requirement.resource_id)
                .cloned()
                .unwrap_or_default();
            let mut boundaries = vec![candidate];
            boundaries.extend(
                intervals
                    .iter()
                    .filter(|item| item.start > candidate && item.start < finish)
                    .map(|item| item.start),
            );
            boundaries.sort_by(f64::total_cmp);
            boundaries.dedup_by(|a, b| (*a - *b).abs() <= 1e-9);

            let Some(capacity) = capacities.get(&requirement.resource_id).copied() else {
                return Err(format!("unknown resource {}", requirement.resource_id));
            };
            let feasible = boundaries.iter().all(|time| {
                let used: u32 = intervals
                    .iter()
                    .filter(|item| item.start <= *time && item.end > *time)
                    .map(|item| item.units)
                    .sum();
                used.saturating_add(requirement.units) <= capacity
            });
            if !feasible {
                all_feasible = false;
                break;
            }
        }

        if all_feasible {
            return Ok(candidate);
        }
    }

    Ok(candidates.last().copied().unwrap_or(ready_hour))
}

/// Deterministic non-preemptive list scheduler.
///
/// This is a transparent greedy baseline, not an optimal RCPSP solver.
/// It schedules the currently-ready task with the earliest feasible start,
/// then breaks ties by event ID and task ID. Exact/MILP/CP-SAT comparisons
/// should be added before making optimality claims.
pub fn schedule(
    tasks: &[TaskSpec],
    capacities: &BTreeMap<String, u32>,
) -> Result<Vec<ScheduledTask>, Vec<String>> {
    let mut errors = Vec::new();
    let mut task_by_key = BTreeMap::new();

    for task in tasks {
        if task.event_id.trim().is_empty() || task.task_id.trim().is_empty() {
            errors.push("task eventId and taskId must not be empty".to_string());
            continue;
        }
        if !task.release_hour.is_finite()
            || task.release_hour < 0.0
            || !task.duration_hours.is_finite()
            || task.duration_hours <= 0.0
        {
            errors.push(format!(
                "{} task {} has invalid release or duration",
                task.event_id, task.task_id
            ));
        }
        let task_key = key(&task.event_id, &task.task_id);
        if task_by_key.insert(task_key.clone(), task).is_some() {
            errors.push(format!("duplicate task ID {task_key}"));
        }
        let mut required_resources = BTreeSet::new();
        for requirement in &task.resource_requirements {
            if !required_resources.insert(requirement.resource_id.as_str()) {
                errors.push(format!(
                    "{} task {} repeats resource requirement {}",
                    task.event_id, task.task_id, requirement.resource_id
                ));
            }
            match capacities.get(&requirement.resource_id) {
                None => errors.push(format!(
                    "{} task {} references unknown resource {}",
                    task.event_id, task.task_id, requirement.resource_id
                )),
                Some(capacity) if requirement.units == 0 || requirement.units > *capacity => {
                    errors.push(format!(
                        "{} task {} requires {} units of {}, capacity is {}",
                        task.event_id, task.task_id, requirement.units,
                        requirement.resource_id, capacity
                    ));
                }
                Some(_) => {}
            }
        }
    }

    for task in task_by_key.values() {
        let mut dependencies = BTreeSet::new();
        for dependency in &task.depends_on {
            if dependency == &task.task_id {
                errors.push(format!(
                    "{} task {} cannot depend on itself",
                    task.event_id, task.task_id
                ));
            }
            if !dependencies.insert(dependency) {
                errors.push(format!(
                    "{} task {} has duplicate dependency {}",
                    task.event_id, task.task_id, dependency
                ));
            }
            if !task_by_key.contains_key(&key(&task.event_id, dependency)) {
                errors.push(format!(
                    "{} task {} has unknown dependency {}",
                    task.event_id, task.task_id, dependency
                ));
            }
        }
    }

    if !errors.is_empty() {
        errors.sort();
        return Err(errors);
    }

    let mut unscheduled: BTreeSet<String> = task_by_key.keys().cloned().collect();
    let mut scheduled_by_key: BTreeMap<String, ScheduledTask> = BTreeMap::new();
    let mut resource_allocations: BTreeMap<String, Vec<ResourceInterval>> = BTreeMap::new();

    while !unscheduled.is_empty() {
        let mut candidates: Vec<(f64, String)> = Vec::new();

        for task_key in &unscheduled {
            let task = task_by_key[task_key];
            let mut ready_hour = task.release_hour;
            let mut ready = true;

            for dependency in &task.depends_on {
                let dependency_key = key(&task.event_id, dependency);
                if let Some(scheduled_dependency) = scheduled_by_key.get(&dependency_key) {
                    ready_hour = ready_hour.max(scheduled_dependency.end_hour);
                } else {
                    ready = false;
                    break;
                }
            }

            if !ready {
                continue;
            }

            let feasible_start = earliest_resource_slot(
                ready_hour,
                task.duration_hours,
                &task.resource_requirements,
                capacities,
                &resource_allocations,
            );
            match feasible_start {
                Ok(start) => candidates.push((start, task_key.clone())),
                Err(error) => errors.push(error),
            }
        }

        if !errors.is_empty() {
            errors.sort();
            return Err(errors);
        }
        if candidates.is_empty() {
            return Err(vec![
                "task dependency graph contains a cycle or unresolved dependency".to_string(),
            ]);
        }

        candidates.sort_by(|left, right| {
            left.0
                .total_cmp(&right.0)
                .then_with(|| left.1.cmp(&right.1))
        });
        let (start_hour, selected_key) = candidates.remove(0);
        let task = task_by_key[&selected_key];
        let end_hour = start_hour + task.duration_hours;

        for requirement in &task.resource_requirements {
            resource_allocations
                .entry(requirement.resource_id.clone())
                .or_default()
                .push(ResourceInterval {
                    start: start_hour,
                    end: end_hour,
                    units: requirement.units,
                });
        }

        scheduled_by_key.insert(
            selected_key,
            ScheduledTask {
                event_id: task.event_id.clone(),
                task_id: task.task_id.clone(),
                start_hour,
                end_hour,
            },
        );
        unscheduled.remove(&key(&task.event_id, &task.task_id));
    }

    Ok(scheduled_by_key.into_values().collect())
}

fn exact_search(
    task_by_key: &BTreeMap<String, &TaskSpec>,
    unscheduled: &mut BTreeSet<String>,
    scheduled: &mut BTreeMap<String, ScheduledTask>,
    resource_allocations: &mut BTreeMap<String, Vec<ResourceInterval>>,
    capacities: &BTreeMap<String, u32>,
    best: &mut Option<(f64, Vec<ScheduledTask>)>,
) {
    if unscheduled.is_empty() {
        let makespan = scheduled
            .values()
            .map(|task| task.end_hour)
            .fold(0.0, f64::max);
        if best
            .as_ref()
            .is_none_or(|(best_makespan, _)| makespan < *best_makespan)
        {
            let mut exact_schedule: Vec<_> = scheduled.values().cloned().collect();
            exact_schedule.sort_by(|left, right| {
                left.start_hour
                    .total_cmp(&right.start_hour)
                    .then_with(|| left.event_id.cmp(&right.event_id))
                    .then_with(|| left.task_id.cmp(&right.task_id))
            });
            *best = Some((makespan, exact_schedule));
        }
        return;
    }

    let partial_makespan = scheduled
        .values()
        .map(|task| task.end_hour)
        .fold(0.0, f64::max);
    if best
        .as_ref()
        .is_some_and(|(best_makespan, _)| partial_makespan >= *best_makespan)
    {
        return;
    }

    let mut candidates: Vec<(f64, String)> = Vec::new();
    for task_key in unscheduled.iter() {
        let task = task_by_key[task_key];
        let mut ready_hour = task.release_hour;
        let mut ready = true;
        for dependency in &task.depends_on {
            let dependency_key = key(&task.event_id, dependency);
            if let Some(predecessor) = scheduled.get(&dependency_key) {
                ready_hour = ready_hour.max(predecessor.end_hour);
            } else {
                ready = false;
                break;
            }
        }
        if !ready {
            continue;
        }

        let start_hour = match earliest_resource_slot(
            ready_hour,
            task.duration_hours,
            &task.resource_requirements,
            capacities,
            resource_allocations,
        ) {
            Ok(start) => start,
            Err(_) => continue,
        };
        candidates.push((start_hour, task_key.clone()));
    }

    candidates.sort_by(|left, right| {
        left.0
            .total_cmp(&right.0)
            .then_with(|| left.1.cmp(&right.1))
    });

    for (start_hour, selected_key) in candidates {
        let task = task_by_key[&selected_key];
        let end_hour = start_hour + task.duration_hours;
        scheduled.insert(
            selected_key.clone(),
            ScheduledTask {
                event_id: task.event_id.clone(),
                task_id: task.task_id.clone(),
                start_hour,
                end_hour,
            },
        );
        for requirement in &task.resource_requirements {
            resource_allocations
                .entry(requirement.resource_id.clone())
                .or_default()
                .push(ResourceInterval {
                    start: start_hour,
                    end: end_hour,
                    units: requirement.units,
                });
        }
        unscheduled.remove(&selected_key);

        exact_search(
            task_by_key,
            unscheduled,
            scheduled,
            resource_allocations,
            capacities,
            best,
        );

        unscheduled.insert(selected_key.clone());
        scheduled.remove(&selected_key);
        for requirement in &task.resource_requirements {
            let remove_resource =
                if let Some(intervals) = resource_allocations.get_mut(&requirement.resource_id) {
                    intervals.pop();
                    intervals.is_empty()
                } else {
                    false
                };
            if remove_resource {
                resource_allocations.remove(&requirement.resource_id);
            }
        }
    }
}

/// Exhaustively enumerates precedence-feasible serial schedules for tiny cases.
///
/// This bounded oracle is intended for differential tests and small exact
/// references. It refuses cases above ten tasks; use CP-SAT/MILP or a validated
/// specialised solver for larger instances.
pub fn exact_small_schedule(
    tasks: &[TaskSpec],
    capacities: &BTreeMap<String, u32>,
) -> Result<Vec<ScheduledTask>, Vec<String>> {
    const MAX_EXACT_TASKS: usize = 10;
    if tasks.is_empty() || tasks.len() > MAX_EXACT_TASKS {
        return Err(vec![format!(
            "exact small scheduler requires 1..={MAX_EXACT_TASKS} tasks"
        )]);
    }

    let greedy = schedule(tasks, capacities)?;
    let greedy_makespan = greedy
        .iter()
        .map(|task| task.end_hour)
        .fold(0.0, f64::max);
    let task_by_key: BTreeMap<String, &TaskSpec> = tasks
        .iter()
        .map(|task| (key(&task.event_id, &task.task_id), task))
        .collect();
    let mut unscheduled: BTreeSet<String> = task_by_key.keys().cloned().collect();
    let mut scheduled = BTreeMap::new();
    let mut resource_allocations = BTreeMap::new();
    let mut best = Some((greedy_makespan, greedy));

    exact_search(
        &task_by_key,
        &mut unscheduled,
        &mut scheduled,
        &mut resource_allocations,
        capacities,
        &mut best,
    );

    best.map(|(_, tasks)| tasks)
        .ok_or_else(|| vec!["no feasible exact schedule found".to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn spec(
        event_id: &str,
        task_id: &str,
        depends_on: &[&str],
        release_hour: f64,
        duration_hours: f64,
        resource_id: Option<&str>,
        resource_units: u32,
    ) -> TaskSpec {
        TaskSpec {
            event_id: event_id.to_string(),
            task_id: task_id.to_string(),
            depends_on: depends_on.iter().map(|x| x.to_string()).collect(),
            release_hour,
            duration_hours,
            resource_requirements: resource_id
                .map(|resource_id| ResourceRequirement {
                    resource_id: resource_id.to_string(),
                    units: resource_units,
                })
                .into_iter()
                .collect(),
        }
    }


    #[test]
    fn exact_small_oracle_improves_over_greedy_task_order() {
        let tasks = vec![
            spec("A", "a1", &[], 0.0, 10.0, Some("r1"), 1),
            spec("A", "a2", &["a1"], 0.0, 10.0, Some("r2"), 1),
            spec("B", "b1", &[], 0.0, 1.0, Some("r1"), 1),
            spec("B", "b2", &["b1"], 0.0, 100.0, Some("r2"), 1),
        ];
        let capacities = BTreeMap::from([
            ("r1".to_string(), 1),
            ("r2".to_string(), 1),
        ]);
        let greedy = schedule(&tasks, &capacities).expect("greedy schedule");
        let exact = exact_small_schedule(&tasks, &capacities).expect("exact schedule");
        let greedy_makespan = greedy.iter().map(|x| x.end_hour).fold(0.0, f64::max);
        let exact_makespan = exact.iter().map(|x| x.end_hour).fold(0.0, f64::max);
        assert_eq!(greedy_makespan, 120.0);
        assert_eq!(exact_makespan, 111.0);
        assert!(exact_makespan < greedy_makespan);
    }

    #[test]
    fn exact_small_oracle_rejects_oversized_cases() {
        let tasks = (0..11)
            .map(|index| {
                let id = format!("t{index}");
                spec("E", &id, &[], 0.0, 1.0, None, 0)
            })
            .collect::<Vec<_>>();
        assert!(exact_small_schedule(&tasks, &BTreeMap::new())
            .unwrap_err()[0]
            .contains("requires 1..=10 tasks"));
    }

    #[test]
    fn allows_independent_tasks_to_run_in_parallel() {
        let tasks = vec![
            spec("E1", "detect", &[], 0.0, 2.0, None, 0),
            spec("E1", "crew", &["detect"], 0.0, 4.0, None, 0),
            spec("E1", "source", &["detect"], 0.0, 1.0, None, 0),
            spec("E1", "transport", &["source"], 0.0, 12.0, None, 0),
            spec("E1", "repair", &["crew", "transport"], 0.0, 10.0, Some("crew"), 1),
        ];
        let capacities = BTreeMap::from([("crew".to_string(), 1)]);
        let result = schedule(&tasks, &capacities).expect("schedule should be feasible");
        let repair = result.iter().find(|x| x.task_id == "repair").unwrap();
        assert_eq!(repair.start_hour, 15.0);
        assert_eq!(repair.end_hour, 25.0);
    }

    #[test]
    fn serializes_overlapping_crew_assignments() {
        let tasks = vec![
            spec("E1", "repair", &[], 0.0, 10.0, Some("crew"), 1),
            spec("E2", "repair", &[], 5.0, 10.0, Some("crew"), 1),
        ];
        let capacities = BTreeMap::from([("crew".to_string(), 1)]);
        let result = schedule(&tasks, &capacities).expect("scheduler should place both jobs");
        let second = result.iter().find(|x| x.event_id == "E2").unwrap();
        assert_eq!(second.start_hour, 10.0);
        assert_eq!(second.end_hour, 20.0);
    }

    #[test]
    fn waits_until_all_simultaneous_resources_are_available() {
        let mut block = spec("A", "block", &[], 0.0, 5.0, Some("crane"), 1);
        let mut repair = spec("B", "repair", &[], 0.0, 2.0, None, 0);
        repair.resource_requirements = vec![
            ResourceRequirement {
                resource_id: "crew".to_string(),
                units: 1,
            },
            ResourceRequirement {
                resource_id: "crane".to_string(),
                units: 1,
            },
            ResourceRequirement {
                resource_id: "test_rig".to_string(),
                units: 1,
            },
        ];
        let tasks = vec![block.clone(), repair.clone()];
        let capacities = BTreeMap::from([
            ("crew".to_string(), 1),
            ("crane".to_string(), 1),
            ("test_rig".to_string(), 1),
        ]);
        let result = schedule(&tasks, &capacities).expect("multi-resource schedule");
        let repair = result.iter().find(|x| x.event_id == "B").unwrap();
        assert_eq!(repair.start_hour, 5.0);
        assert_eq!(repair.end_hour, 7.0);
    }

    #[test]
    fn rejects_dependency_cycles() {
        let tasks = vec![
            spec("E1", "a", &["b"], 0.0, 1.0, None, 0),
            spec("E1", "b", &["a"], 0.0, 1.0, None, 0),
        ];
        assert!(schedule(&tasks, &BTreeMap::new()).unwrap_err()[0].contains("cycle"));
    }

    #[test]
    fn rejects_unknown_dependencies() {
        let tasks = vec![spec("E1", "a", &["missing"], 0.0, 1.0, None, 0)];
        assert!(schedule(&tasks, &BTreeMap::new()).unwrap_err()[0].contains("unknown dependency"));
    }
}
