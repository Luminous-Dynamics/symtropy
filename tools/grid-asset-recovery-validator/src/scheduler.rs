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
pub struct ResourceOutageWindow {
    pub start_hour: f64,
    pub end_hour: f64,
    pub units_unavailable: u32,
}

#[derive(Clone, Debug, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResourceProfile {
    pub available_units: u32,
    pub calendar_period_hours: f64,
    pub availability_windows: Vec<AvailabilityWindow>,
    #[serde(default)]
    pub outage_windows: Vec<ResourceOutageWindow>,
}

#[derive(Clone, Debug)]
pub struct TaskSpec {
    pub event_id: String,
    pub task_id: String,
    pub depends_on: Vec<String>,
    pub release_hour: f64,
    pub duration_hours: f64,
    pub service_weight_mw: f64,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SchedulePolicy {
    EarliestFeasibleStart,
    ServiceWeightedCriticalPath,
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
    let max_window_hours = profile
        .availability_windows
        .iter()
        .map(|window| window.end_hour - window.start_hour)
        .fold(0.0, f64::max);
    if duration_hours > max_window_hours + 1e-9 {
        return Err(format!(
            "task duration {}h exceeds every availability window for resource {}",
            duration_hours, requirement.resource_id
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
    schedule_with_policy(tasks, profiles, SchedulePolicy::EarliestFeasibleStart)
}

fn remaining_critical_path_hours(
    task_key: &str,
    task_by_key: &BTreeMap<String, &TaskSpec>,
    successors: &BTreeMap<String, Vec<String>>,
    cache: &mut BTreeMap<String, f64>,
) -> f64 {
    if let Some(value) = cache.get(task_key) {
        return *value;
    }
    let task = task_by_key[task_key];
    let downstream = successors
        .get(task_key)
        .map(|next| {
            next.iter()
                .map(|next_key| remaining_critical_path_hours(next_key, task_by_key, successors, cache))
                .fold(0.0, f64::max)
        })
        .unwrap_or(0.0);
    let value = task.duration_hours + downstream;
    cache.insert(task_key.to_string(), value);
    value
}

pub fn schedule_with_policy(
    tasks: &[TaskSpec],
    profiles: &BTreeMap<String, ResourceProfile>,
    policy: SchedulePolicy,
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
    let mut resource_allocations: BTreeMap<String, Vec<ResourceInterval>> = BTreeMap::new();
    for (resource_id, profile) in profiles {
        let mut outage_boundaries = Vec::new();
        for outage in &profile.outage_windows {
            if !outage.start_hour.is_finite()
                || !outage.end_hour.is_finite()
                || outage.start_hour < 0.0
                || outage.end_hour <= outage.start_hour
                || outage.units_unavailable == 0
                || outage.units_unavailable > profile.available_units
            {
                errors.push(format!("resource {resource_id} has invalid outage window"));
                continue;
            }
            outage_boundaries.push(outage.start_hour);
            outage_boundaries.push(outage.end_hour);
            resource_allocations
                .entry(resource_id.clone())
                .or_default()
                .push(ResourceInterval {
                    start: outage.start_hour,
                    end: outage.end_hour,
                    units: outage.units_unavailable,
                });
        }
        outage_boundaries.sort_by(f64::total_cmp);
        outage_boundaries.dedup_by(|left, right| (*left - *right).abs() <= 1e-9);
        for boundary in outage_boundaries {
            let unavailable: u32 = profile
                .outage_windows
                .iter()
                .filter(|outage| outage.start_hour <= boundary && outage.end_hour > boundary)
                .map(|outage| outage.units_unavailable)
                .sum();
            if unavailable > profile.available_units {
                errors.push(format!(
                    "resource {resource_id} has overlapping outages exceeding available capacity"
                ));
                break;
            }
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
            || !task.service_weight_mw.is_finite()
            || task.service_weight_mw < 0.0
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

    let mut successors: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut indegree: BTreeMap<String, usize> =
        task_by_key.keys().map(|task_key| (task_key.clone(), 0)).collect();
    for (task_key, task) in &task_by_key {
        for dependency in &task.depends_on {
            let dependency_key = key(&task.event_id, dependency);
            successors
                .entry(dependency_key.clone())
                .or_default()
                .push(task_key.clone());
            *indegree.get_mut(task_key).expect("task key is indexed") += 1;
        }
    }
    let mut topological_ready: BTreeSet<String> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(task_key, _)| task_key.clone())
        .collect();
    let mut visited_count = 0usize;
    while let Some(task_key) = topological_ready.pop_first() {
        visited_count += 1;
        if let Some(next_tasks) = successors.get(&task_key) {
            for next_task in next_tasks {
                let degree = indegree.get_mut(next_task).expect("successor is indexed");
                *degree -= 1;
                if *degree == 0 {
                    topological_ready.insert(next_task.clone());
                }
            }
        }
    }
    if visited_count != task_by_key.len() {
        return Err(vec![
            "task dependency graph contains a cycle or unresolved dependency".to_string(),
        ]);
    }
    let mut critical_path_cache = BTreeMap::new();

    let mut unscheduled: BTreeSet<String> = task_by_key.keys().cloned().collect();
    let mut scheduled_by_key: BTreeMap<String, ScheduledTask> = BTreeMap::new();

    while !unscheduled.is_empty() {
        let mut candidates: Vec<(f64, f64, String)> = Vec::new();

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
                Ok(start) => {
                    let priority = match policy {
                        SchedulePolicy::EarliestFeasibleStart => 0.0,
                        SchedulePolicy::ServiceWeightedCriticalPath => {
                            let critical_path = remaining_critical_path_hours(
                                task_key,
                                &task_by_key,
                                &successors,
                                &mut critical_path_cache,
                            );
                            if critical_path > 0.0 {
                                task.service_weight_mw / critical_path
                            } else {
                                0.0
                            }
                        }
                    };
                    candidates.push((start, priority, task_key.clone()));
                }
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
                .then_with(|| right.1.total_cmp(&left.1))
                .then_with(|| left.2.cmp(&right.2))
        });
        let (start_hour, _, selected_key) = candidates.remove(0);
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

/// Calculates total weighted unserved energy from a complete schedule.
///
/// Each event's service_weight_mw is its net firm shortfall while the event
/// remains unrestored. The v0.x screening model assumes that shortfall is
/// constant until the final event task completes.
pub fn weighted_unserved_energy_mwh(
    tasks: &[TaskSpec],
    scheduled: &[ScheduledTask],
) -> Result<f64, Vec<String>> {
    let mut event_meta: BTreeMap<String, (f64, f64)> = BTreeMap::new();
    for task in tasks {
        match event_meta.get(&task.event_id) {
            Some((release, weight))
                if (*release - task.release_hour).abs() > 1e-9
                    || (*weight - task.service_weight_mw).abs() > 1e-9 =>
            {
                return Err(vec![format!(
                    "event {} has inconsistent release or service weight",
                    task.event_id
                )]);
            }
            Some(_) => {}
            None => {
                event_meta.insert(
                    task.event_id.clone(),
                    (task.release_hour, task.service_weight_mw),
                );
            }
        }
    }

    let mut completion: BTreeMap<String, f64> = event_meta
        .iter()
        .map(|(event_id, (release, _))| (event_id.clone(), *release))
        .collect();
    let mut seen = BTreeSet::new();
    let task_ids: BTreeSet<String> = tasks
        .iter()
        .map(|task| key(&task.event_id, &task.task_id))
        .collect();

    for scheduled_task in scheduled {
        let task_key = key(&scheduled_task.event_id, &scheduled_task.task_id);
        if !task_ids.contains(&task_key) || !seen.insert(task_key.clone()) {
            return Err(vec![format!(
                "schedule contains unknown or duplicate task {}",
                task_key
            )]);
        }
        let Some(end) = completion.get_mut(&scheduled_task.event_id) else {
            return Err(vec![format!(
                "schedule contains unknown event {}",
                scheduled_task.event_id
            )]);
        };
        if !scheduled_task.end_hour.is_finite()
            || !scheduled_task.start_hour.is_finite()
            || scheduled_task.end_hour < scheduled_task.start_hour
        {
            return Err(vec![format!("schedule task {} has invalid interval", task_key)]);
        }
        if scheduled_task.end_hour > *end {
            *end = scheduled_task.end_hour;
        }
    }

    if seen.len() != task_ids.len() {
        return Err(vec!["schedule is missing one or more tasks".to_string()]);
    }

    let mut total = 0.0;
    for (event_id, (release, weight)) in event_meta {
        let end = completion[&event_id];
        total += weight * (end - release);
    }
    Ok(total)
}

fn exact_service_search(
    task_by_key: &BTreeMap<String, &TaskSpec>,
    unscheduled: &mut BTreeSet<String>,
    scheduled: &mut BTreeMap<String, ScheduledTask>,
    resource_allocations: &mut BTreeMap<String, Vec<ResourceInterval>>,
    profiles: &BTreeMap<String, ResourceProfile>,
    best: &mut Option<(f64, f64, Vec<ScheduledTask>)>,
) {
    if unscheduled.is_empty() {
        let scheduled_tasks: Vec<_> = scheduled.values().cloned().collect();
        let task_specs: Vec<_> = task_by_key.values().map(|task| (**task).clone()).collect();
        let objective = weighted_unserved_energy_mwh(&task_specs, &scheduled_tasks)
            .expect("complete exact-search schedule has valid service metadata");
        let makespan = scheduled_tasks
            .iter()
            .map(|task| task.end_hour)
            .fold(0.0, f64::max);
        let replace = best.as_ref().is_none_or(|(best_objective, best_makespan, _)| {
            objective < *best_objective - 1e-9
                || ((objective - *best_objective).abs() <= 1e-9
                    && makespan < *best_makespan - 1e-9)
        });
        if replace {
            let mut ordered = scheduled_tasks;
            ordered.sort_by(|left, right| {
                left.start_hour
                    .total_cmp(&right.start_hour)
                    .then_with(|| left.event_id.cmp(&right.event_id))
                    .then_with(|| left.task_id.cmp(&right.task_id))
            });
            *best = Some((objective, makespan, ordered));
        }
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

        exact_service_search(
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

/// Exhaustively minimises weighted unserved energy on tiny recovery cases.
/// This is bounded to eight tasks and assumes fixed net shortfall per event.
pub fn exact_small_service_schedule(
    tasks: &[TaskSpec],
    profiles: &BTreeMap<String, ResourceProfile>,
) -> Result<Vec<ScheduledTask>, Vec<String>> {
    const MAX_EXACT_SERVICE_TASKS: usize = 8;
    if tasks.is_empty() || tasks.len() > MAX_EXACT_SERVICE_TASKS {
        return Err(vec![format!(
            "exact service scheduler requires 1..={MAX_EXACT_SERVICE_TASKS} tasks"
        )]);
    }

    let greedy = schedule_with_policy(
        tasks,
        profiles,
        SchedulePolicy::ServiceWeightedCriticalPath,
    )?;
    let greedy_objective = weighted_unserved_energy_mwh(tasks, &greedy)?;
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
    let mut best = Some((greedy_objective, greedy_makespan, greedy));

    exact_service_search(
        &task_by_key,
        &mut unscheduled,
        &mut scheduled,
        &mut resource_allocations,
        profiles,
        &mut best,
    );

    best.map(|(_, _, tasks)| tasks)
        .ok_or_else(|| vec!["no feasible exact service schedule found".to_string()])
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
            outage_windows: Vec::new(),
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
            service_weight_mw: 1.0,
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
    fn weighted_service_objective_prioritizes_critical_outage() {
        let mut low = spec("A-low", "repair", &[], 0.0, 5.0, Some("crew"), 1);
        low.service_weight_mw = 10.0;
        let mut critical = spec("Z-critical", "repair", &[], 0.0, 10.0, Some("crew"), 1);
        critical.service_weight_mw = 100.0;
        let tasks = vec![low, critical];
        let profiles = BTreeMap::from([(
            "crew".to_string(),
            profile(1, 24.0, &[(0.0, 24.0)]),
        )]);
        let greedy = schedule(&tasks, &profiles).expect("greedy schedule");
        let service_policy = schedule_with_policy(
            &tasks,
            &profiles,
            SchedulePolicy::ServiceWeightedCriticalPath,
        )
        .expect("service-weighted heuristic");
        let exact = exact_small_service_schedule(&tasks, &profiles).expect("exact service schedule");
        let greedy_mwh = weighted_unserved_energy_mwh(&tasks, &greedy).expect("greedy objective");
        let service_policy_mwh =
            weighted_unserved_energy_mwh(&tasks, &service_policy).expect("service policy objective");
        let exact_mwh = weighted_unserved_energy_mwh(&tasks, &exact).expect("exact objective");
        assert_eq!(greedy_mwh, 1550.0);
        assert_eq!(service_policy_mwh, 1150.0);
        assert_eq!(exact_mwh, 1150.0);
        assert!(exact_mwh < greedy_mwh);
        assert_eq!(
            exact.iter().find(|task| task.event_id == "Z-critical").unwrap().start_hour,
            0.0
        );
    }

    #[test]
    fn weighted_service_objective_rejects_inconsistent_event_weights() {
        let first = spec("E1", "a", &[], 0.0, 1.0, None, 0);
        let mut second = spec("E1", "b", &[], 0.0, 1.0, None, 0);
        second.service_weight_mw = 2.0;
        assert!(weighted_unserved_energy_mwh(
            &[first, second],
            &[
                ScheduledTask { event_id: "E1".to_string(), task_id: "a".to_string(), start_hour: 0.0, end_hour: 1.0 },
                ScheduledTask { event_id: "E1".to_string(), task_id: "b".to_string(), start_hour: 1.0, end_hour: 2.0 },
            ]
        ).unwrap_err()[0].contains("inconsistent release or service weight"));
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
    fn delays_repair_for_one_off_partial_resource_outage() {
        let task = spec("E1", "repair", &[], 8.0, 4.0, Some("crew"), 1);
        let mut crew = profile(2, 24.0, &[(0.0, 24.0)]);
        crew.outage_windows.push(ResourceOutageWindow {
            start_hour: 8.0,
            end_hour: 14.0,
            units_unavailable: 1,
        });
        let profiles = BTreeMap::from([("crew".to_string(), crew)]);
        let result = schedule(&[task], &profiles).expect("one remaining crew unit is available");
        assert_eq!(result[0].start_hour, 8.0);
        assert_eq!(result[0].end_hour, 12.0);
    }

    #[test]
    fn blocks_resource_when_all_capacity_is_unavailable() {
        let task = spec("E1", "repair", &[], 8.0, 4.0, Some("crew"), 1);
        let mut crew = profile(1, 24.0, &[(0.0, 24.0)]);
        crew.outage_windows.push(ResourceOutageWindow {
            start_hour: 8.0,
            end_hour: 14.0,
            units_unavailable: 1,
        });
        let profiles = BTreeMap::from([("crew".to_string(), crew)]);
        let result = schedule(&[task], &profiles).expect("resource should recover after outage");
        assert_eq!(result[0].start_hour, 14.0);
        assert_eq!(result[0].end_hour, 18.0);
    }

    #[test]
    fn rejects_overlapping_outages_above_resource_capacity() {
        let task = spec("E1", "repair", &[], 8.0, 4.0, Some("crew"), 1);
        let mut crew = profile(1, 24.0, &[(0.0, 24.0)]);
        crew.outage_windows = vec![
            ResourceOutageWindow { start_hour: 8.0, end_hour: 12.0, units_unavailable: 1 },
            ResourceOutageWindow { start_hour: 10.0, end_hour: 14.0, units_unavailable: 1 },
        ];
        let profiles = BTreeMap::from([("crew".to_string(), crew)]);
        assert!(schedule(&[task], &profiles)
            .unwrap_err()[0]
            .contains("overlapping outages exceeding available capacity"));
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
    fn intersects_calendars_across_all_simultaneous_resources() {
        let mut task = spec("E1", "repair", &[], 0.0, 2.0, None, 0);
        task.resource_requirements = vec![
            ResourceRequirement { resource_id: "crew".to_string(), units: 1 },
            ResourceRequirement { resource_id: "crane".to_string(), units: 1 },
            ResourceRequirement { resource_id: "test_rig".to_string(), units: 1 },
        ];
        let profiles = BTreeMap::from([
            ("crew".to_string(), profile(1, 24.0, &[(0.0, 24.0)])),
            ("crane".to_string(), profile(1, 24.0, &[(8.0, 12.0)])),
            ("test_rig".to_string(), profile(1, 24.0, &[(10.0, 18.0)])),
        ]);
        let result = schedule(&[task], &profiles).expect("calendar intersection should exist");
        assert_eq!(result[0].start_hour, 10.0);
        assert_eq!(result[0].end_hour, 12.0);
    }

    #[test]
    fn rejects_task_longer_than_every_shift_window() {
        let task = spec("E1", "haul", &[], 0.0, 12.0, Some("hauler"), 1);
        let profiles = BTreeMap::from([(
            "hauler".to_string(),
            profile(1, 24.0, &[(8.0, 18.0)]),
        )]);
        assert!(schedule(&[task], &profiles)
            .unwrap_err()[0]
            .contains("exceeds every availability window"));
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
