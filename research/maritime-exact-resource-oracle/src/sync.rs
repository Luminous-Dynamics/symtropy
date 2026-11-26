//! Exact QC–AGV event synchronization for small fixed-order microinstances.
//!
//! This module models handoff as an event recurrence:
//! service_start = max(QC-ready, AGV-ready).
//! Both waiting components and the uncertainty witness are reported. Candidate
//! AGV routes are here-and-now choices; the service-time scenario changes only
//! event times, not task assignment or sequence.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncTask {
    pub task_id: String,
    pub qc_id: String,
    /// Initial availability time when there is no QC predecessor.
    pub qc_release_at: u32,
    pub nominal_service: u32,
    pub service_deviation: u32,
    pub qc_predecessor: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncRoute {
    pub route_id: String,
    /// Fixed AGV task order.
    pub task_order: Vec<String>,
    /// AGV travel time to the first task.
    pub initial_travel: u32,
    /// Travel time between each consecutive pair, length = task_order.len() - 1.
    pub inter_task_travel: Vec<u32>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncInstance {
    pub instance_id: String,
    pub tasks: Vec<SyncTask>,
    pub routes: Vec<SyncRoute>,
    pub qc_wait_weight: u64,
    pub agv_wait_weight: u64,
    /// L1 budget over discrete deviations {-1, 0, +1}.
    pub gamma: usize,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TaskTiming {
    pub task_id: String,
    pub qc_ready: u64,
    pub agv_ready: u64,
    pub service_start: u64,
    pub service_finish: u64,
    pub qc_wait: u64,
    pub agv_wait: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ScenarioEvaluation {
    pub route_id: String,
    pub scenario: Vec<i8>,
    pub total_qc_wait: u64,
    pub total_agv_wait: u64,
    pub weighted_wait_cost: u64,
    pub makespan: u64,
    pub task_timings: Vec<TaskTiming>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteRobustResult {
    pub route_id: String,
    pub nominal: ScenarioEvaluation,
    pub worst_case: ScenarioEvaluation,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SyncTournamentResult {
    pub instance_id: String,
    pub gamma: usize,
    pub uncertainty_scenario_count: usize,
    pub routes: Vec<RouteRobustResult>,
    pub nominal_best_route: String,
    pub robust_best_route: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SyncError {
    EmptyIdentifier,
    EmptyTasks,
    EmptyRoutes,
    TooManyTasksForExactEnumeration(usize),
    InvalidGamma {
        gamma: usize,
        task_count: usize,
    },
    DuplicateTask(String),
    DuplicateRoute(String),
    InvalidServiceBand {
        task_id: String,
    },
    UnknownTask {
        route_id: String,
        task_id: String,
    },
    DuplicateTaskInRoute {
        route_id: String,
        task_id: String,
    },
    RouteTaskSetMismatch {
        route_id: String,
    },
    TravelVectorLength {
        route_id: String,
        expected: usize,
        actual: usize,
    },
    UnknownQcPredecessor {
        task_id: String,
        predecessor_id: String,
    },
    QcPredecessorMismatch {
        task_id: String,
        predecessor_id: String,
    },
    PrecedenceNotAvailable {
        route_id: String,
        task_id: String,
        predecessor_id: String,
    },
    ArithmeticOverflow,
}

impl std::fmt::Display for SyncError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for SyncError {}

fn validate_instance(instance: &SyncInstance) -> Result<BTreeMap<String, SyncTask>, SyncError> {
    if instance.instance_id.is_empty() {
        return Err(SyncError::EmptyIdentifier);
    }
    if instance.tasks.is_empty() {
        return Err(SyncError::EmptyTasks);
    }
    if instance.routes.is_empty() {
        return Err(SyncError::EmptyRoutes);
    }
    if instance.tasks.len() > 12 {
        return Err(SyncError::TooManyTasksForExactEnumeration(
            instance.tasks.len(),
        ));
    }
    if instance.gamma > instance.tasks.len() {
        return Err(SyncError::InvalidGamma {
            gamma: instance.gamma,
            task_count: instance.tasks.len(),
        });
    }

    let mut tasks = BTreeMap::new();
    for task in &instance.tasks {
        if task.task_id.is_empty() || task.qc_id.is_empty() {
            return Err(SyncError::EmptyIdentifier);
        }
        if task.nominal_service <= task.service_deviation
            || task
                .nominal_service
                .checked_add(task.service_deviation)
                .is_none()
        {
            return Err(SyncError::InvalidServiceBand {
                task_id: task.task_id.clone(),
            });
        }
        if tasks.insert(task.task_id.clone(), task.clone()).is_some() {
            return Err(SyncError::DuplicateTask(task.task_id.clone()));
        }
    }

    let mut route_ids = BTreeSet::new();
    for route in &instance.routes {
        if route.route_id.is_empty() {
            return Err(SyncError::EmptyIdentifier);
        }
        if !route_ids.insert(route.route_id.clone()) {
            return Err(SyncError::DuplicateRoute(route.route_id.clone()));
        }
    }

    for task in tasks.values() {
        if let Some(predecessor_id) = &task.qc_predecessor {
            let predecessor =
                tasks
                    .get(predecessor_id)
                    .ok_or_else(|| SyncError::UnknownQcPredecessor {
                        task_id: task.task_id.clone(),
                        predecessor_id: predecessor_id.clone(),
                    })?;
            if predecessor.qc_id != task.qc_id {
                return Err(SyncError::QcPredecessorMismatch {
                    task_id: task.task_id.clone(),
                    predecessor_id: predecessor_id.clone(),
                });
            }
        }
    }

    Ok(tasks)
}

fn validate_route(route: &SyncRoute, tasks: &BTreeMap<String, SyncTask>) -> Result<(), SyncError> {
    let expected_travel = route.task_order.len().saturating_sub(1);
    if route.inter_task_travel.len() != expected_travel {
        return Err(SyncError::TravelVectorLength {
            route_id: route.route_id.clone(),
            expected: expected_travel,
            actual: route.inter_task_travel.len(),
        });
    }

    let mut seen = BTreeSet::new();
    for task_id in &route.task_order {
        if !tasks.contains_key(task_id) {
            return Err(SyncError::UnknownTask {
                route_id: route.route_id.clone(),
                task_id: task_id.clone(),
            });
        }
        if !seen.insert(task_id.clone()) {
            return Err(SyncError::DuplicateTaskInRoute {
                route_id: route.route_id.clone(),
                task_id: task_id.clone(),
            });
        }
    }
    if seen.len() != tasks.len() {
        return Err(SyncError::RouteTaskSetMismatch {
            route_id: route.route_id.clone(),
        });
    }
    Ok(())
}

fn uncertainty_vectors(task_count: usize, gamma: usize) -> Vec<Vec<i8>> {
    fn recurse(
        index: usize,
        used_budget: usize,
        task_count: usize,
        gamma: usize,
        current: &mut Vec<i8>,
        output: &mut Vec<Vec<i8>>,
    ) {
        if index == task_count {
            output.push(current.clone());
            return;
        }

        for value in [-1_i8, 0, 1] {
            let next_budget = used_budget + usize::from(value.unsigned_abs());
            if next_budget <= gamma {
                current.push(value);
                recurse(index + 1, next_budget, task_count, gamma, current, output);
                current.pop();
            }
        }
    }

    let mut output = Vec::new();
    recurse(
        0,
        0,
        task_count,
        gamma,
        &mut Vec::with_capacity(task_count),
        &mut output,
    );
    output.sort();
    output
}

fn checked_add(left: u64, right: u64) -> Result<u64, SyncError> {
    left.checked_add(right).ok_or(SyncError::ArithmeticOverflow)
}

fn checked_mul(left: u64, right: u64) -> Result<u64, SyncError> {
    left.checked_mul(right).ok_or(SyncError::ArithmeticOverflow)
}

fn simulate(
    tasks: &BTreeMap<String, SyncTask>,
    route: &SyncRoute,
    scenario: &[i8],
    qc_wait_weight: u64,
    agv_wait_weight: u64,
) -> Result<ScenarioEvaluation, SyncError> {
    let mut timings: BTreeMap<String, TaskTiming> = BTreeMap::new();
    let mut total_qc_wait = 0_u64;
    let mut total_agv_wait = 0_u64;
    let mut makespan = 0_u64;

    for (index, task_id) in route.task_order.iter().enumerate() {
        let task = tasks.get(task_id).ok_or_else(|| SyncError::UnknownTask {
            route_id: route.route_id.clone(),
            task_id: task_id.clone(),
        })?;

        let qc_ready = match &task.qc_predecessor {
            Some(predecessor_id) => {
                timings
                    .get(predecessor_id)
                    .ok_or_else(|| SyncError::PrecedenceNotAvailable {
                        route_id: route.route_id.clone(),
                        task_id: task_id.clone(),
                        predecessor_id: predecessor_id.clone(),
                    })?
                    .service_finish
            }
            None => u64::from(task.qc_release_at),
        };

        let agv_ready = if index == 0 {
            u64::from(route.initial_travel)
        } else {
            let predecessor_id = &route.task_order[index - 1];
            let predecessor =
                timings
                    .get(predecessor_id)
                    .ok_or_else(|| SyncError::UnknownTask {
                        route_id: route.route_id.clone(),
                        task_id: predecessor_id.clone(),
                    })?;
            checked_add(
                predecessor.service_finish,
                u64::from(route.inter_task_travel[index - 1]),
            )?
        };

        let service_start = qc_ready.max(agv_ready);
        let qc_wait = service_start - qc_ready;
        let agv_wait = service_start - agv_ready;
        let deviation_index = tasks
            .keys()
            .position(|candidate_id| candidate_id == task_id)
            .ok_or_else(|| SyncError::UnknownTask {
                route_id: route.route_id.clone(),
                task_id: task_id.clone(),
            })?;
        let deviation = *scenario
            .get(deviation_index)
            .ok_or(SyncError::ArithmeticOverflow)?;
        let service_time = if deviation < 0 {
            task.nominal_service - task.service_deviation
        } else if deviation > 0 {
            task.nominal_service + task.service_deviation
        } else {
            task.nominal_service
        };
        let service_finish = checked_add(service_start, u64::from(service_time))?;

        total_qc_wait = checked_add(total_qc_wait, qc_wait)?;
        total_agv_wait = checked_add(total_agv_wait, agv_wait)?;
        makespan = makespan.max(service_finish);

        timings.insert(
            task_id.clone(),
            TaskTiming {
                task_id: task_id.clone(),
                qc_ready,
                agv_ready,
                service_start,
                service_finish,
                qc_wait,
                agv_wait,
            },
        );
    }

    let weighted_wait_cost = checked_add(
        checked_mul(total_qc_wait, qc_wait_weight)?,
        checked_mul(total_agv_wait, agv_wait_weight)?,
    )?;

    let task_timings = route
        .task_order
        .iter()
        .filter_map(|task_id| timings.get(task_id).cloned())
        .collect();

    Ok(ScenarioEvaluation {
        route_id: route.route_id.clone(),
        scenario: scenario.to_vec(),
        total_qc_wait,
        total_agv_wait,
        weighted_wait_cost,
        makespan,
        task_timings,
    })
}

fn evaluate_route(
    tasks: &BTreeMap<String, SyncTask>,
    route: &SyncRoute,
    scenarios: &[Vec<i8>],
    qc_wait_weight: u64,
    agv_wait_weight: u64,
) -> Result<RouteRobustResult, SyncError> {
    let zero_scenario = vec![0_i8; tasks.len()];
    let nominal = simulate(
        tasks,
        route,
        &zero_scenario,
        qc_wait_weight,
        agv_wait_weight,
    )?;

    let mut worst_case: Option<ScenarioEvaluation> = None;
    for scenario in scenarios {
        let evaluation = simulate(tasks, route, scenario, qc_wait_weight, agv_wait_weight)?;
        let should_replace = match &worst_case {
            None => true,
            Some(current) => {
                evaluation.weighted_wait_cost > current.weighted_wait_cost
                    || (evaluation.weighted_wait_cost == current.weighted_wait_cost
                        && evaluation.scenario < current.scenario)
            }
        };
        if should_replace {
            worst_case = Some(evaluation);
        }
    }

    Ok(RouteRobustResult {
        route_id: route.route_id.clone(),
        nominal,
        worst_case: worst_case.ok_or(SyncError::EmptyTasks)?,
    })
}

/// Exhaustively evaluates each fixed AGV route over all discrete service-time
/// deviation vectors in {-1,0,+1} with L1 norm <= Gamma.
///
/// This is exact for the declared finite set; it is not a continuous-budget
/// robust guarantee. Service starts are derived from max(QC-ready, AGV-ready),
/// and no discretionary idle time is introduced.
pub fn evaluate_sync_tournament(
    instance: &SyncInstance,
) -> Result<SyncTournamentResult, SyncError> {
    let tasks = validate_instance(instance)?;
    let scenarios = uncertainty_vectors(tasks.len(), instance.gamma);

    let mut routes = Vec::new();
    for route in &instance.routes {
        validate_route(route, &tasks)?;
        routes.push(evaluate_route(
            &tasks,
            route,
            &scenarios,
            instance.qc_wait_weight,
            instance.agv_wait_weight,
        )?);
    }
    routes.sort_by(|left, right| left.route_id.cmp(&right.route_id));

    let nominal_best_route = routes
        .iter()
        .min_by(|left, right| {
            left.nominal
                .weighted_wait_cost
                .cmp(&right.nominal.weighted_wait_cost)
                .then_with(|| left.route_id.cmp(&right.route_id))
        })
        .ok_or(SyncError::EmptyRoutes)?
        .route_id
        .clone();

    let robust_best_route = routes
        .iter()
        .min_by(|left, right| {
            left.worst_case
                .weighted_wait_cost
                .cmp(&right.worst_case.weighted_wait_cost)
                .then_with(|| left.route_id.cmp(&right.route_id))
        })
        .ok_or(SyncError::EmptyRoutes)?
        .route_id
        .clone();

    Ok(SyncTournamentResult {
        instance_id: instance.instance_id.clone(),
        gamma: instance.gamma,
        uncertainty_scenario_count: scenarios.len(),
        routes,
        nominal_best_route,
        robust_best_route,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_instance(gamma: usize) -> SyncInstance {
        SyncInstance {
            instance_id: "SYNC-1".to_owned(),
            tasks: vec![
                SyncTask {
                    task_id: "A".to_owned(),
                    qc_id: "QC-A".to_owned(),
                    qc_release_at: 0,
                    nominal_service: 3,
                    service_deviation: 1,
                    qc_predecessor: None,
                },
                SyncTask {
                    task_id: "B".to_owned(),
                    qc_id: "QC-B".to_owned(),
                    qc_release_at: 4,
                    nominal_service: 4,
                    service_deviation: 3,
                    qc_predecessor: None,
                },
            ],
            routes: vec![
                SyncRoute {
                    route_id: "AB".to_owned(),
                    task_order: vec!["A".to_owned(), "B".to_owned()],
                    initial_travel: 5,
                    inter_task_travel: vec![0],
                },
                SyncRoute {
                    route_id: "BA".to_owned(),
                    task_order: vec!["B".to_owned(), "A".to_owned()],
                    initial_travel: 0,
                    inter_task_travel: vec![0],
                },
            ],
            qc_wait_weight: 5,
            agv_wait_weight: 1,
            gamma,
        }
    }

    #[test]
    fn handoff_start_is_max_of_qc_and_agv_readiness() {
        let result = evaluate_sync_tournament(&tiny_instance(0)).unwrap();
        let route = result
            .routes
            .iter()
            .find(|item| item.route_id == "AB")
            .unwrap();
        assert_eq!(route.nominal.task_timings[0].qc_ready, 0);
        assert_eq!(route.nominal.task_timings[0].agv_ready, 5);
        assert_eq!(route.nominal.task_timings[0].service_start, 5);
        assert_eq!(route.nominal.task_timings[0].qc_wait, 5);
        assert_eq!(route.nominal.task_timings[0].agv_wait, 0);
        assert_eq!(route.nominal.task_timings[1].qc_ready, 4);
        assert_eq!(route.nominal.task_timings[1].agv_ready, 8);
        assert_eq!(route.nominal.task_timings[1].qc_wait, 4);
    }

    #[test]
    fn nominal_and_robust_route_can_differ_under_uncertain_qc_service() {
        let result = evaluate_sync_tournament(&tiny_instance(1)).unwrap();
        assert_eq!(result.nominal_best_route, "BA");
        assert_eq!(result.robust_best_route, "AB");
    }

    #[test]
    fn worst_case_receipt_contains_the_service_time_witness() {
        let result = evaluate_sync_tournament(&tiny_instance(1)).unwrap();
        let route = result
            .routes
            .iter()
            .find(|item| item.route_id == "BA")
            .unwrap();
        assert_eq!(route.nominal.weighted_wait_cost, 44);
        assert_eq!(route.worst_case.weighted_wait_cost, 59);
        assert_eq!(route.worst_case.total_qc_wait, 11);
        assert_eq!(route.worst_case.total_agv_wait, 4);
        assert_eq!(route.worst_case.scenario, vec![0, 1]);
    }

    #[test]
    fn gamma_zero_contains_only_the_nominal_scenario() {
        let result = evaluate_sync_tournament(&tiny_instance(0)).unwrap();
        assert_eq!(result.uncertainty_scenario_count, 1);
        assert_eq!(result.nominal_best_route, result.robust_best_route);
    }

    #[test]
    fn gamma_one_excludes_two_simultaneous_nonzero_deviations() {
        let result = evaluate_sync_tournament(&tiny_instance(1)).unwrap();
        assert_eq!(result.uncertainty_scenario_count, 5);
    }

    #[test]
    fn qc_predecessor_delay_propagates_to_successor() {
        let mut instance = tiny_instance(0);
        instance.tasks.push(SyncTask {
            task_id: "C".to_owned(),
            qc_id: "QC-A".to_owned(),
            qc_release_at: 0,
            nominal_service: 2,
            service_deviation: 1,
            qc_predecessor: Some("A".to_owned()),
        });
        for route in &mut instance.routes {
            route.task_order.push("C".to_owned());
            route.inter_task_travel.push(0);
        }
        let result = evaluate_sync_tournament(&instance).unwrap();
        let route = result
            .routes
            .iter()
            .find(|item| item.route_id == "AB")
            .unwrap();
        let c = route
            .nominal
            .task_timings
            .iter()
            .find(|timing| timing.task_id == "C")
            .unwrap();
        assert!(c.qc_ready >= 8);
    }

    #[test]
    fn route_with_task_omission_is_rejected() {
        let mut instance = tiny_instance(0);
        instance.routes[0].task_order.pop();
        instance.routes[0].inter_task_travel.clear();
        assert!(matches!(
            evaluate_sync_tournament(&instance),
            Err(SyncError::RouteTaskSetMismatch { .. })
        ));
    }

    #[test]
    fn invalid_service_band_is_rejected() {
        let mut instance = tiny_instance(1);
        instance.tasks[0].nominal_service = 1;
        instance.tasks[0].service_deviation = 1;
        assert!(matches!(
            evaluate_sync_tournament(&instance),
            Err(SyncError::InvalidServiceBand { .. })
        ));
    }

    #[test]
    fn fixed_qc_predecessor_must_be_available_before_task() {
        let mut instance = tiny_instance(0);
        instance.tasks.push(SyncTask {
            task_id: "C".to_owned(),
            qc_id: "QC-A".to_owned(),
            qc_release_at: 0,
            nominal_service: 2,
            service_deviation: 1,
            qc_predecessor: Some("A".to_owned()),
        });
        // C appears before A in the AGV sequence, so the fixed event order
        // would create an unavailable predecessor at the required handoff.
        instance.routes[0].task_order = vec!["C".to_owned(), "A".to_owned(), "B".to_owned()];
        instance.routes[0].inter_task_travel = vec![0, 0];
        assert!(matches!(
            evaluate_sync_tournament(&instance),
            Err(SyncError::PrecedenceNotAvailable { .. })
        ));
    }
}
