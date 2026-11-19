use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceRequirement {
    pub resource_id: String,
    pub units: u32,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct AvailabilityWindow {
    pub start_hour: f64,
    pub end_hour: f64,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProfile {
    pub available_units: u32,
    pub calendar_period_hours: f64,
    pub availability_windows: Vec<AvailabilityWindow>,
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

fn earliest_slot_for_resource(
    ready_hour: f64,
    duration_hours: f64,
    requirement: &ResourceRequirement,
    profile: &ResourceProfile,
    allocations: &[ResourceInterval],
) -> Result<f64, String> {
    if requirement.units == 0 || requirement.units > profile.available_units {
        return Err(format!(
            "resource {} requires {} units but capacity is {}",
            requirement.resource_id, requirement.units, profile.available_units
        ));
    }
    if !profile.calendar_period_hours.is_finite()
        || profile.calendar_period_hours <= 0.0
        || profile.availability_windows.is_empty()
    {
        return Err(format!(
            "resource {} has invalid or empty availability calendar",
            requirement.resource_id
        ));
    }

    let mut period_index = (ready_hour / profile.calendar_period_hours).floor().max(0.0);
    for _ in 0..100_000 {
        let period_start = period_index * profile.calendar_period_hours;
        for window in &profile.availability_windows {
            let window_start = period_start + window.start_hour;
            let window_end = period_start + window.end_hour;
            let mut candidate = ready_hour.max(window_start);

            while candidate + duration_hours <= window_end + 1e-9 {
                let mut boundaries = vec![candidate];
                boundaries.extend(
                    allocations
                        .iter()
                        .filter(|item| item.start > candidate && item.start < candidate + duration_hours)
                        .map(|item| item.start),
                );
                boundaries.sort_by(f64::total_cmp);
                boundaries.dedup_by(|left, right| (*left - *right).abs() <= 1e-9);

                let conflict_time = boundaries.iter().copied().find(|time| {
                    let used: u32 = allocations
                        .iter()
                        .filter(|item| item.start <= *time && item.end > *time)
                        .map(|item| item.units)
                        .sum();
                    used.saturating_add(requirement.units) > profile.available_units
                });

                let Some(conflict_time) = conflict_time else {
                    return Ok(candidate);
                };
                let next_candidate = allocations
                    .iter()
                    .filter(|item| item.start <= conflict_time && item.end > conflict_time)
                    .map(|item| item.end)
                    .filter(|end| *end > candidate + 1e-9)
                    .min_by(f64::total_cmp);
                let Some(next_candidate) = next_candidate else {
                    break;
                };
                candidate = next_candidate;
            }
        }
        period_index += 1.0;
    }

    Err(format!(
        "resource {} has no feasible availability slot within scheduler search horizon",
        requirement.resource_id
    ))
}

fn earliest_resource_slot(
    ready_hour: f64,
    duration_hours: f64,
    requirements: &[ResourceRequirement],
    profiles: &BTreeMap<String, ResourceProfile>,
    allocations: &BTreeMap<String, Vec<ResourceInterval>>,
) -> Result<f64, String> {
    if requirements.is_empty() {
        return Ok(ready_hour);
    }

    let mut seen = BTreeSet::new();
    for requirement in requirements {
        if !seen.insert(requirement.resource_id.as_str()) {
            return Err(format!(
                "duplicate resource requirement {}",
                requirement.resource_id
            ));
        }
    }

    let mut candidate = ready_hour;
    for _ in 0..100_000 {
        let mut next_candidate = candidate;
        for requirement in requirements {
            let Some(profile) = profiles.get(&requirement.resource_id) else {
                return Err(format!("unknown resource {}", requirement.resource_id));
            };
            let resource_allocations = allocations
                .get(&requirement.resource_id)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            let feasible_start = earliest_slot_for_resource(
                candidate,
                duration_hours,
                requirement,
                profile,
                resource_allocations,
            )?;
            next_candidate = next_candidate.max(feasible_start);
        }
        if (next_candidate - candidate).abs() <= 1e-9 {
            return Ok(candidate);
        }
        candidate = next_candidate;
    }

    Err("multi-resource calendar intersection did not converge".to_string())
}

/// Deterministic non-preemptive list scheduler.
///
/// This is a transparent greedy baseline, not an optimal RCPSP solver.
/// It schedules the currently-ready task with the earliest feasible start,
/// then breaks ties by event ID and task ID. Exact/MILP/CP-SAT comparisons
/// should be added before making optimality claims.
pub fn schedule(
    tasks: &[TaskSpec],
    profiles: &BTreeMap<String, ResourceProfile>,
) -> Result<Vec<ScheduledTask>, Vec<String>> {
    let mut errors = Vec::new();
    for (resource_id, profile) in profiles {
        if profile.available_units == 0
            || !profile.calendar_period_hours.is_finite()
            || profile.calendar_period_hours <= 0.0
            || profile.availability_windows.is_empty()
        {
            errors.push(format!("resource {resource_id} has invalid capacity or calendar"));
            continue;
        }
        let mut previous_end = 0.0;
        for window in &profile.availability_windows {
            if !window.start_hour.is_finite()
                || !window.end_hour.is_finite()
                || window.start_hour < previous_end
                || window.end_hour <= window.start_hour
                || window.end_hour > profile.calendar_period_hours
            {
                errors.push(format!("resource {resource_id} has invalid or overlapping calendar windows"));
            }
            previous_end = window.end_hour;
        }
    }
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
            match profiles.get(&requirement.resource_id) {
                None => errors.push(format!(
                    "{} task {} references unknown resource {}",
                    task.event_id, task.task_id, requirement.resource_id
                )),
                Some(profile) if requirement.units == 0 || requirement.units > profile.available_units => {
                    errors.push(format!(
                        "{} task {} requires {} units of {}, capacity is {}",
                        task.event_id, task.task_id, requirement.units,
                        requirement.resource_id, profile.available_units
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
                profiles,
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
    profiles: &BTreeMap<String, ResourceProfile>,
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
            profiles,
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
            profiles,
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
    profiles: &BTreeMap<String, ResourceProfile>,
) -> Result<Vec<ScheduledTask>, Vec<String>> {
    const MAX_EXACT_TASKS: usize = 10;
    if tasks.is_empty() || tasks.len() > MAX_EXACT_TASKS {
        return Err(vec![format!(
            "exact small scheduler requires 1..={MAX_EXACT_TASKS} tasks"
        )]);
    }

    let greedy = schedule(tasks, profiles)?;
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
        profiles,
        &mut best,
    );

    best.map(|(_, tasks)| tasks)
        .ok_or_else(|| vec!["no feasible exact schedule found".to_string()])
}

#[cfg(test)]
mod tests {
    use super::*;

    fn profile(capacity: u32, period: f64, windows: &[(f64, f64)]) -> ResourceProfile {
        ResourceProfile {
            available_units: capacity,
            calendar_period_hours: period,
            availability_windows: windows
                .iter()
                .map(|(start_hour, end_hour)| AvailabilityWindow {
                    start_hour: *start_hour,
                    end_hour: *end_hour,
                })
                .collect(),
        }
    }

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
            ("r1".to_string(), profile(1, 24.0, &[(0.0, 24.0)])),
            ("r2".to_string(), profile(1, 24.0, &[(0.0, 24.0)])),
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
        let capacities = BTreeMap::from([("crew".to_string(), profile(1, 24.0, &[(0.0, 24.0)]))]);
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
        let capacities = BTreeMap::from([("crew".to_string(), profile(1, 24.0, &[(0.0, 24.0)]))]);
        let result = schedule(&tasks, &capacities).expect("scheduler should place both jobs");
        let second = result.iter().find(|x| x.event_id == "E2").unwrap();
        assert_eq!(second.start_hour, 10.0);
        assert_eq!(second.end_hour, 20.0);
    }

    #[test]
    fn waits_for_next_calendar_window_when_task_cannot_fit() {
        let task = spec("E1", "repair", &[], 14.0, 5.0, Some("crew"), 1);
        let profiles = BTreeMap::from([(
            "crew".to_string(),
            profile(1, 24.0, &[(8.0, 18.0)]),
        )]);
        let result = schedule(&[task], &profiles).expect("calendar should have next shift");
        assert_eq!(result[0].start_hour, 32.0);
        assert_eq!(result[0].end_hour, 37.0);
    }

    #[test]
    fn rejects_empty_resource_calendar() {
        let task = spec("E1", "repair", &[], 0.0, 2.0, Some("crew"), 1);
        let profiles = BTreeMap::from([(
            "crew".to_string(),
            profile(1, 24.0, &[]),
        )]);
        assert!(schedule(&[task], &profiles)
            .unwrap_err()[0]
            .contains("invalid capacity or calendar"));
    }

    #[test]
    fn waits_until_all_simultaneous_resources_are_available() {
        let crane_block = spec("A", "block-crane", &[], 0.0, 5.0, Some("crane"), 1);
        let test_rig_block = spec("C", "block-test-rig", &[], 0.0, 8.0, Some("test_rig"), 1);
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
        let tasks = vec![crane_block, test_rig_block, repair];
        let capacities = BTreeMap::from([
            ("crew".to_string(), profile(1, 24.0, &[(0.0, 24.0)])),
            ("crane".to_string(), profile(1, 24.0, &[(0.0, 24.0)])),
            ("test_rig".to_string(), profile(1, 24.0, &[(0.0, 24.0)])),
        ]);
        let result = schedule(&tasks, &capacities).expect("multi-resource schedule");
        let repair = result.iter().find(|x| x.event_id == "B").unwrap();
        assert_eq!(repair.start_hour, 8.0);
        assert_eq!(repair.end_hour, 10.0);
    }

    #[test]
    fn rejects_duplicate_resource_requirements() {
        let mut task = spec("E1", "repair", &[], 0.0, 2.0, None, 0);
        task.resource_requirements = vec![
            ResourceRequirement { resource_id: "crew".to_string(), units: 1 },
            ResourceRequirement { resource_id: "crew".to_string(), units: 1 },
        ];
        let capacities = BTreeMap::from([("crew".to_string(), profile(1, 24.0, &[(0.0, 24.0)]))]);
        assert!(schedule(&[task], &capacities)
            .unwrap_err()[0]
            .contains("repeats resource requirement"));
    }

    #[test]
    fn rejects_requirement_above_resource_capacity() {
        let mut task = spec("E1", "repair", &[], 0.0, 2.0, None, 0);
        task.resource_requirements = vec![
            ResourceRequirement { resource_id: "crew".to_string(), units: 2 },
        ];
        let capacities = BTreeMap::from([("crew".to_string(), profile(1, 24.0, &[(0.0, 24.0)]))]);
        assert!(schedule(&[task], &capacities)
            .unwrap_err()[0]
            .contains("requires 2 units"));
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
