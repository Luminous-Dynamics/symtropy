//! Exact small-instance continuous-budget adversary for a fixed one-AGV
//! task order in the simplified QC–AGV synchronization model.
//!
//! This is deliberately a bounded reference oracle (at most four tasks), not
//! the paper's multi-AGV route-assignment MILP. It enumerates handoff active
//! regions and L1 sign orthants, then solves each resulting bounded LP by
//! vertex enumeration. Its result is exact up to the stated floating-point
//! tolerances for this affine/piecewise-affine model.

use crate::sync::{SyncInstance, SyncRoute, SyncTask};
use std::collections::BTreeMap;

const EPS: f64 = 1.0e-8;
const COST_TOLERANCE: f64 = 1.0e-6;
pub const MAX_CONTINUOUS_REFERENCE_TASKS: usize = 4;

#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousRouteResult {
    pub route_id: String,
    pub nominal_cost: f64,
    pub worst_case_cost: f64,
    pub witness: Vec<f64>,
    pub worst_case_qc_wait: f64,
    pub worst_case_agv_wait: f64,
    pub worst_case_makespan: f64,
    pub active_regions_examined: usize,
    pub feasible_vertices_examined: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousTournamentResult {
    pub gamma: f64,
    pub nominal_best_route: String,
    pub robust_best_route: String,
    pub routes: Vec<ContinuousRouteResult>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContinuousError {
    EmptyTasks,
    TooManyTasks(usize),
    EmptyRoutes,
    InvalidGamma(f64),
    InvalidWeight,
    DuplicateTask(String),
    DuplicateRoute(String),
    UnknownTask(String),
    RouteTaskSetMismatch(String),
    TravelVectorLength(String),
    InvalidServiceBand(String),
    UnknownQcPredecessor(String, String),
    QcPredecessorMismatch(String, String),
    PrecedenceUnavailable(String, String),
    NoFeasibleActiveRegion,
    NumericalMismatch { lp_cost: f64, replay_cost: f64 },
}

impl std::fmt::Display for ContinuousError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for ContinuousError {}

#[derive(Clone, Debug)]
struct Affine {
    constant: f64,
    coefficients: Vec<f64>,
}

impl Affine {
    fn constant(value: f64, dimensions: usize) -> Self {
        Self {
            constant: value,
            coefficients: vec![0.0; dimensions],
        }
    }

    fn add(&self, other: &Self) -> Self {
        Self {
            constant: self.constant + other.constant,
            coefficients: self
                .coefficients
                .iter()
                .zip(&other.coefficients)
                .map(|(left, right)| left + right)
                .collect(),
        }
    }

    fn sub(&self, other: &Self) -> Self {
        Self {
            constant: self.constant - other.constant,
            coefficients: self
                .coefficients
                .iter()
                .zip(&other.coefficients)
                .map(|(left, right)| left - right)
                .collect(),
        }
    }
}

#[derive(Clone, Debug)]
struct LinearConstraint {
    coefficients: Vec<f64>,
    upper_bound: f64,
}

#[derive(Clone, Debug)]
struct AffineRouteModel {
    objective: Affine,
    branch_constraints: Vec<LinearConstraint>,
}

#[derive(Clone, Debug)]
struct Replay {
    objective: f64,
    qc_wait: f64,
    agv_wait: f64,
    makespan: f64,
}

fn build_affine_route_model(
    tasks: &BTreeMap<String, SyncTask>,
    route: &SyncRoute,
    branch_mask: usize,
) -> Result<AffineRouteModel, ContinuousError> {
    let dimensions = route.task_order.len();
    let mut qc_finish: BTreeMap<String, Affine> = BTreeMap::new();
    let mut vehicle_release: BTreeMap<String, Affine> = BTreeMap::new();
    let mut objective = Affine::constant(0.0, dimensions);
    let mut branch_constraints = Vec::with_capacity(dimensions);

    for (index, task_id) in route.task_order.iter().enumerate() {
        let task = tasks
            .get(task_id)
            .ok_or_else(|| ContinuousError::UnknownTask(task_id.clone()))?;

        if task.nominal_service <= task.service_deviation
            || task.nominal_service.checked_add(task.service_deviation).is_none()
        {
            return Err(ContinuousError::InvalidServiceBand(task_id.clone()));
        }

        let qc_ready = match &task.qc_predecessor {
            Some(predecessor_id) => qc_finish
                .get(predecessor_id)
                .cloned()
                .ok_or_else(|| {
                    ContinuousError::PrecedenceUnavailable(
                        task_id.clone(),
                        predecessor_id.clone(),
                    )
                })?,
            None => Affine::constant(f64::from(task.qc_release_at), dimensions),
        };

        let agv_ready = if index == 0 {
            Affine::constant(f64::from(route.initial_travel), dimensions)
        } else {
            let previous_id = &route.task_order[index - 1];
            vehicle_release
                .get(previous_id)
                .ok_or_else(|| ContinuousError::UnknownTask(previous_id.clone()))?
                .add(&Affine::constant(
                    f64::from(route.inter_task_travel[index - 1]),
                    dimensions,
                ))
        };

        let difference = qc_ready.sub(&agv_ready);
        let choose_agv = (branch_mask & (1usize << index)) != 0;

        // Under the transformed orthant coordinates u_j = sign_j * x_j,
        // x_j >= 0. If QC wins, qc_ready - agv_ready >= 0; otherwise
        // agv_ready - qc_ready >= 0. These are the active-region LP faces.
        let branch_constraint = if choose_agv {
            LinearConstraint {
                coefficients: difference.coefficients.clone(),
                upper_bound: -difference.constant,
            }
        } else {
            LinearConstraint {
                coefficients: difference
                    .coefficients
                    .iter()
                    .map(|coefficient| -*coefficient)
                    .collect(),
                upper_bound: difference.constant,
            }
        };
        branch_constraints.push(branch_constraint);

        let service_index = index;
        let mut service = Affine::constant(f64::from(task.nominal_service), dimensions);
        service.coefficients[service_index] = f64::from(task.service_deviation);

        let start = if choose_agv {
            agv_ready.clone()
        } else {
            qc_ready.clone()
        };
        let finish = start.add(&service);

        let qc_wait = start.sub(&qc_ready);
        let agv_wait = start.sub(&agv_ready);
        objective = objective
            .add(&Affine {
                constant: qc_wait.constant * 1.0,
                coefficients: qc_wait
                    .coefficients
                    .iter()
                    .map(|value| value * 1.0)
                    .collect(),
            })
            .add(&Affine {
                constant: agv_wait.constant * 1.0,
                coefficients: agv_wait
                    .coefficients
                    .iter()
                    .map(|value| value * 1.0)
                    .collect(),
            });

        qc_finish.insert(task_id.clone(), finish.clone());
        // The simplified sync reference holds a vehicle through service before
        // the next travel leg. Loading-specific release semantics are a later
        // extension and are not silently mixed into this model.
        vehicle_release.insert(task_id.clone(), finish);
    }

    Ok(AffineRouteModel {
        objective,
        branch_constraints,
    })
}

fn solve_square_system(matrix: &[Vec<f64>], rhs: &[f64]) -> Option<Vec<f64>> {
    let n = rhs.len();
    if matrix.len() != n || matrix.iter().any(|row| row.len() != n) {
        return None;
    }
    let mut augmented: Vec<Vec<f64>> = matrix
        .iter()
        .zip(rhs)
        .map(|(row, value)| {
            let mut result = row.clone();
            result.push(*value);
            result
        })
        .collect();

    for column in 0..n {
        let pivot = (column..n).max_by(|left, right| {
            augmented[*left][column]
                .abs()
                .total_cmp(&augmented[*right][column].abs())
        })?;
        if augmented[pivot][column].abs() <= EPS {
            return None;
        }
        augmented.swap(column, pivot);

        let divisor = augmented[column][column];
        for item in column..=n {
            augmented[column][item] /= divisor;
        }

        for row in 0..n {
            if row == column {
                continue;
            }
            let factor = augmented[row][column];
            if factor.abs() <= EPS {
                continue;
            }
            for item in column..=n {
                augmented[row][item] -= factor * augmented[column][item];
            }
        }
    }

    Some(augmented.into_iter().map(|row| row[n]).collect())
}

fn feasible(point: &[f64], constraints: &[LinearConstraint]) -> bool {
    constraints.iter().all(|constraint| {
        let value: f64 = constraint
            .coefficients
            .iter()
            .zip(point)
            .map(|(coefficient, coordinate)| coefficient * coordinate)
            .sum();
        value <= constraint.upper_bound + COST_TOLERANCE
    })
}

fn enumerate_combinations(
    constraints: &[LinearConstraint],
    dimensions: usize,
    callback: &mut impl FnMut(&[usize]),
) {
    fn recurse(
        constraints: &[LinearConstraint],
        dimensions: usize,
        start: usize,
        selected: &mut Vec<usize>,
        callback: &mut impl FnMut(&[usize]),
    ) {
        if selected.len() == dimensions {
            callback(selected);
            return;
        }
        let remaining = dimensions - selected.len();
        if constraints.len() < remaining {
            return;
        }
        let last_start = constraints.len() - remaining;
        for index in start..=last_start {
            selected.push(index);
            recurse(constraints, dimensions, index + 1, selected, callback);
            selected.pop();
        }
    }

    recurse(
        constraints,
        dimensions,
        0,
        &mut Vec::with_capacity(dimensions),
        callback,
    );
}

fn replay_witness(
    tasks: &BTreeMap<String, SyncTask>,
    route: &SyncRoute,
    witness: &[f64],
    qc_wait_weight: f64,
    agv_wait_weight: f64,
) -> Result<Replay, ContinuousError> {
    let mut qc_finish = BTreeMap::new();
    let mut vehicle_release = BTreeMap::new();
    let mut qc_wait = 0.0;
    let mut agv_wait = 0.0;
    let mut makespan: f64 = 0.0;

    for (index, task_id) in route.task_order.iter().enumerate() {
        let task = tasks
            .get(task_id)
            .ok_or_else(|| ContinuousError::UnknownTask(task_id.clone()))?;
        let qc_ready = match &task.qc_predecessor {
            Some(predecessor_id) => *qc_finish.get(predecessor_id).ok_or_else(|| {
                ContinuousError::PrecedenceUnavailable(task_id.clone(), predecessor_id.clone())
            })?,
            None => f64::from(task.qc_release_at),
        };
        let agv_ready = if index == 0 {
            f64::from(route.initial_travel)
        } else {
            vehicle_release[&route.task_order[index - 1]]
                + f64::from(route.inter_task_travel[index - 1])
        };
        let service_start = qc_ready.max(agv_ready);
        let service_time =
            f64::from(task.nominal_service) + f64::from(task.service_deviation) * witness[index];
        if service_time < 1.0 - EPS {
            return Err(ContinuousError::InvalidServiceBand(task_id.clone()));
        }
        let finish = service_start + service_time;
        qc_wait += service_start - qc_ready;
        agv_wait += service_start - agv_ready;
        makespan = makespan.max(finish);
        qc_finish.insert(task_id.clone(), finish);
        vehicle_release.insert(task_id.clone(), finish);
    }

    Ok(Replay {
        objective: qc_wait_weight * qc_wait + agv_wait_weight * agv_wait,
        qc_wait,
        agv_wait,
        makespan,
    })
}

fn evaluate_fixed_route(
    instance: &SyncInstance,
    route: &SyncRoute,
    gamma: f64,
) -> Result<ContinuousRouteResult, ContinuousError> {
    let task_count = instance.tasks.len();
    let dimensions = route.task_order.len();
    if task_count == 0 || dimensions == 0 {
        return Err(ContinuousError::EmptyTasks);
    }
    if dimensions > MAX_CONTINUOUS_REFERENCE_TASKS {
        return Err(ContinuousError::TooManyTasks(dimensions));
    }
    if (gamma < 0.0) || (gamma > dimensions as f64) || !gamma.is_finite() {
        return Err(ContinuousError::InvalidGamma(gamma));
    }
    if instance.qc_wait_weight > u64::MAX / 2 || instance.agv_wait_weight > u64::MAX / 2 {
        return Err(ContinuousError::InvalidWeight);
    }
    if route.inter_task_travel.len() != dimensions.saturating_sub(1) {
        return Err(ContinuousError::TravelVectorLength(route.route_id.clone()));
    }

    let mut tasks = BTreeMap::new();
    for task in &instance.tasks {
        if tasks.insert(task.task_id.clone(), task).is_some() {
            return Err(ContinuousError::DuplicateTask(task.task_id.clone()));
        }
    }
    if tasks.len() != dimensions {
        return Err(ContinuousError::RouteTaskSetMismatch(route.route_id.clone()));
    }
    let mut seen = std::collections::BTreeSet::new();
    for task_id in &route.task_order {
        if !tasks.contains_key(task_id) {
            return Err(ContinuousError::UnknownTask(task_id.clone()));
        }
        if !seen.insert(task_id.clone()) {
            return Err(ContinuousError::DuplicateTask(task_id.clone()));
        }
    }
    if seen.len() != tasks.len() {
        return Err(ContinuousError::RouteTaskSetMismatch(route.route_id.clone()));
    }

    for task in instance.tasks.iter() {
        if task.nominal_service <= task.service_deviation {
            return Err(ContinuousError::InvalidServiceBand(task.task_id.clone()));
        }
        if let Some(predecessor_id) = &task.qc_predecessor {
            let predecessor = tasks
                .get(predecessor_id)
                .ok_or_else(|| {
                    ContinuousError::UnknownQcPredecessor(
                        task.task_id.clone(),
                        predecessor_id.clone(),
                    )
                })?;
            if predecessor.qc_id != task.qc_id {
                return Err(ContinuousError::QcPredecessorMismatch(
                    task.task_id.clone(),
                    predecessor_id.clone(),
                ));
            }
        }
    }

    let task_values: BTreeMap<String, SyncTask> = instance
        .tasks
        .iter()
        .map(|task| (task.task_id.clone(), task.clone()))
        .collect();
    let signs_count = 1usize << dimensions;
    let branches_count = 1usize << dimensions;
    let qc_weight = instance.qc_wait_weight as f64;
    let agv_weight = instance.agv_wait_weight as f64;

    let mut best: Option<(f64, Vec<f64>, usize, usize)> = None;
    let mut active_regions_examined = 0usize;
    let mut feasible_vertices_examined = 0usize;

    for sign_mask in 0..signs_count {
        let signs: Vec<f64> = (0..dimensions)
            .map(|index| if (sign_mask & (1usize << index)) == 0 { 1.0 } else { -1.0 })
            .collect();

        for branch_mask in 0..branches_count {
            let model = build_affine_route_model(&task_values, route, branch_mask)?;
            let mut constraints = Vec::with_capacity(3 * dimensions + 1);

            for index in 0..dimensions {
                let mut lower = vec![0.0; dimensions];
                lower[index] = -1.0;
                constraints.push(LinearConstraint {
                    coefficients: lower,
                    upper_bound: 0.0,
                });
                let mut upper = vec![0.0; dimensions];
                upper[index] = 1.0;
                constraints.push(LinearConstraint {
                    coefficients: upper,
                    upper_bound: 1.0,
                });
            }
            constraints.push(LinearConstraint {
                coefficients: vec![1.0; dimensions],
                upper_bound: gamma,
            });

            for branch in &model.branch_constraints {
                let coefficients = branch
                    .coefficients
                    .iter()
                    .zip(&signs)
                    .map(|(coefficient, sign)| coefficient * sign)
                    .collect();
                constraints.push(LinearConstraint {
                    coefficients,
                    upper_bound: branch.upper_bound,
                });
            }

            let objective_coefficients: Vec<f64> = model
                .objective
                .coefficients
                .iter()
                .zip(&signs)
                .map(|(coefficient, sign)| coefficient * sign)
                .collect();

            let mut local_best: Option<(f64, Vec<f64>)> = None;
            enumerate_combinations(&constraints, dimensions, &mut |indices| {
                active_regions_examined += 1;
                let matrix: Vec<Vec<f64>> = indices
                    .iter()
                    .map(|index| constraints[*index].coefficients.clone())
                    .collect();
                let rhs: Vec<f64> = indices
                    .iter()
                    .map(|index| constraints[*index].upper_bound)
                    .collect();
                let Some(point) = solve_square_system(&matrix, &rhs) else {
                    return;
                };
                if !feasible(&point, &constraints) {
                    return;
                }
                feasible_vertices_examined += 1;
                let cost = model.objective.constant
                    + objective_coefficients
                        .iter()
                        .zip(&point)
                        .map(|(coefficient, coordinate)| coefficient * coordinate)
                        .sum::<f64>();
                let replace = match &local_best {
                    None => true,
                    Some((current_cost, current_point)) => {
                        cost > *current_cost + EPS
                            || ((cost - *current_cost).abs() <= EPS && point < *current_point)
                    }
                };
                if replace {
                    local_best = Some((cost, point));
                }
            });

            if let Some((local_cost, point)) = local_best {
                let witness: Vec<f64> = point
                    .iter()
                    .zip(&signs)
                    .map(|(coordinate, sign)| coordinate * sign)
                    .collect();
                let replace = match &best {
                    None => true,
                    Some((current_cost, current_witness, _, _)) => {
                        local_cost > *current_cost + EPS
                            || ((local_cost - *current_cost).abs() <= EPS
                                && witness < *current_witness)
                    }
                };
                if replace {
                    best = Some((
                        local_cost,
                        witness,
                        sign_mask,
                        branch_mask,
                    ));
                }
            }
        }
    }

    let (worst_case_cost, witness, _, _) =
        best.ok_or(ContinuousError::NoFeasibleActiveRegion)?;
    let replay = replay_witness(
        &task_values,
        route,
        &witness,
        qc_weight,
        agv_weight,
    )?;
    if (replay.objective - worst_case_cost).abs() > COST_TOLERANCE {
        return Err(ContinuousError::NumericalMismatch {
            lp_cost: worst_case_cost,
            replay_cost: replay.objective,
        });
    }
    let nominal = replay_witness(
        &task_values,
        route,
        &vec![0.0; dimensions],
        qc_weight,
        agv_weight,
    )?;

    Ok(ContinuousRouteResult {
        route_id: route.route_id.clone(),
        nominal_cost: nominal.objective,
        worst_case_cost,
        witness,
        worst_case_qc_wait: replay.qc_wait,
        worst_case_agv_wait: replay.agv_wait,
        worst_case_makespan: replay.makespan,
        active_regions_examined,
        feasible_vertices_examined,
    })
}

/// Evaluates each fixed AGV route over a continuous two-sided L1 service-time
/// uncertainty set. The active-region LP is solved by exhaustive vertex
/// enumeration. This is an exact small-instance reference for <=4 tasks, not
/// a scalable S6-B solver or a multi-AGV master optimizer.
pub fn evaluate_continuous_sync_tournament(
    instance: &SyncInstance,
    gamma: f64,
) -> Result<ContinuousTournamentResult, ContinuousError> {
    if instance.routes.is_empty() {
        return Err(ContinuousError::EmptyRoutes);
    }
    if instance.tasks.len() > MAX_CONTINUOUS_REFERENCE_TASKS {
        return Err(ContinuousError::TooManyTasks(instance.tasks.len()));
    }
    if gamma < 0.0 || gamma > instance.tasks.len() as f64 || !gamma.is_finite() {
        return Err(ContinuousError::InvalidGamma(gamma));
    }
    let validation = SyncInstance {
        gamma: 0,
        ..instance.clone()
    };
    crate::sync::evaluate_sync_tournament(&validation)
        .map_err(|_| ContinuousError::InvalidGamma(gamma))?;

    let mut routes = Vec::new();
    for route in &instance.routes {
        routes.push(evaluate_fixed_route(instance, route, gamma)?);
    }
    routes.sort_by(|left, right| left.route_id.cmp(&right.route_id));

    let nominal_best_route = routes
        .iter()
        .min_by(|left, right| {
            left.nominal_cost
                .total_cmp(&right.nominal_cost)
                .then_with(|| left.route_id.cmp(&right.route_id))
        })
        .ok_or(ContinuousError::EmptyRoutes)?
        .route_id
        .clone();
    let robust_best_route = routes
        .iter()
        .min_by(|left, right| {
            left.worst_case_cost
                .total_cmp(&right.worst_case_cost)
                .then_with(|| left.route_id.cmp(&right.route_id))
        })
        .ok_or(ContinuousError::EmptyRoutes)?
        .route_id
        .clone();

    Ok(ContinuousTournamentResult {
        gamma,
        nominal_best_route,
        robust_best_route,
        routes,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tiny_instance() -> SyncInstance {
        SyncInstance {
            instance_id: "CONTINUOUS-SYNC".to_owned(),
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
            gamma: 1,
        }
    }

    #[test]
    fn gamma_zero_matches_nominal_tournament() {
        let instance = tiny_instance();
        let result = evaluate_continuous_sync_tournament(&instance, 0.0).unwrap();
        assert_eq!(result.nominal_best_route, result.robust_best_route);
        for route in result.routes {
            assert!((route.nominal_cost - route.worst_case_cost).abs() < COST_TOLERANCE);
            assert!(route.witness.iter().all(|value| value.abs() < EPS));
        }
    }

    #[test]
    fn continuous_gamma_witness_stays_inside_fractional_budget() {
        let result = evaluate_continuous_sync_tournament(&tiny_instance(), 0.5).unwrap();
        for route in result.routes {
            assert!(route.witness.iter().all(|value| value.abs() <= 1.0 + EPS));
            assert!(route.witness.iter().map(|value| value.abs()).sum::<f64>() <= 0.5 + EPS);
            assert!(route.worst_case_cost + EPS >= route.nominal_cost);
        }
    }

    #[test]
    fn continuous_worst_case_dominates_discrete_scenario_subset() {
        let instance = tiny_instance();
        let discrete = crate::sync::evaluate_sync_tournament(&instance).unwrap();
        let continuous = evaluate_continuous_sync_tournament(&instance, 1.0).unwrap();
        for route in &continuous.routes {
            let discrete_route = discrete
                .routes
                .iter()
                .find(|item| item.route_id == route.route_id)
                .unwrap();
            assert!(
                route.worst_case_cost + COST_TOLERANCE
                    >= discrete_route.worst_case.weighted_wait_cost as f64
            );
        }
    }

    #[test]
    fn exact_reference_caps_task_count_before_combinatorial_growth() {
        let mut instance = tiny_instance();
        for index in 0..3 {
            instance.tasks.push(SyncTask {
                task_id: format!("X{index}"),
                qc_id: format!("QC-X{index}"),
                qc_release_at: 0,
                nominal_service: 2,
                service_deviation: 1,
                qc_predecessor: None,
            });
            for route in &mut instance.routes {
                route.task_order.push(format!("X{index}"));
                route.inter_task_travel.push(0);
            }
        }
        assert_eq!(
            evaluate_continuous_sync_tournament(&instance, 1.0),
            Err(ContinuousError::TooManyTasks(5))
        );
    }
}
