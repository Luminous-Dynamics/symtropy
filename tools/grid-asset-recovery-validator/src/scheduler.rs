use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug)]
pub struct TaskSpec {
    pub event_id: String,
    pub task_id: String,
    pub depends_on: Vec<String>,
    pub release_hour: f64,
    pub duration_hours: f64,
    pub resource_id: Option<String>,
    pub resource_units: u32,
}

#[derive(Clone, Debug)]
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
    resource_id: &str,
    required_units: u32,
    capacities: &BTreeMap<String, u32>,
    allocations: &BTreeMap<String, Vec<ResourceInterval>>,
) -> Result<f64, String> {
    let Some(capacity) = capacities.get(resource_id).copied() else {
        return Err(format!("unknown resource {resource_id}"));
    };
    if required_units == 0 || required_units > capacity {
        return Err(format!(
            "resource {resource_id} requires {required_units} units but capacity is {capacity}"
        ));
    }

    let intervals = allocations.get(resource_id).cloned().unwrap_or_default();
    let mut candidates = vec![ready_hour];
    candidates.extend(
        intervals
            .iter()
            .filter(|item| item.end >= ready_hour)
            .map(|item| item.end),
    );
    candidates.sort_by(f64::total_cmp);
    candidates.dedup_by(|a, b| (*a - *b).abs() <= 1e-9);

    for candidate in candidates {
        let finish = candidate + duration_hours;
        let mut boundaries = vec![candidate];
        boundaries.extend(
            intervals
                .iter()
                .filter(|item| item.start > candidate && item.start < finish)
                .map(|item| item.start),
        );
        boundaries.sort_by(f64::total_cmp);
        boundaries.dedup_by(|a, b| (*a - *b).abs() <= 1e-9);

        let feasible = boundaries.iter().all(|time| {
            let used: u32 = intervals
                .iter()
                .filter(|item| item.start <= *time && item.end > *time)
                .map(|item| item.units)
                .sum();
            used.saturating_add(required_units) <= capacity
        });
        if feasible {
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
        if task.resource_id.is_none() && task.resource_units != 0 {
            errors.push(format!(
                "{} task {} has resourceUnits without resourceId",
                task.event_id, task.task_id
            ));
        }
        if let Some(resource_id) = &task.resource_id {
            if !capacities.contains_key(resource_id) {
                errors.push(format!(
                    "{} task {} references unknown resource {}",
                    task.event_id, task.task_id, resource_id
                ));
            }
            if task.resource_units == 0 {
                errors.push(format!(
                    "{} task {} must request at least one resource unit",
                    task.event_id, task.task_id
                ));
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

            let feasible_start = match &task.resource_id {
                Some(resource_id) => earliest_resource_slot(
                    ready_hour,
                    task.duration_hours,
                    resource_id,
                    task.resource_units,
                    capacities,
                    &resource_allocations,
                ),
                None => Ok(ready_hour),
            };
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

        if let Some(resource_id) = &task.resource_id {
            resource_allocations
                .entry(resource_id.clone())
                .or_default()
                .push(ResourceInterval {
                    start: start_hour,
                    end: end_hour,
                    units: task.resource_units,
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
            resource_id: resource_id.map(str::to_string),
            resource_units,
        }
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
