//! Small exact active-region reference for continuous-budget QC–AGV timing.
//!
//! A plan fixes QC precedence and each vehicle's task route. The event model
//! then computes service starts as max(QC-ready, AGV-ready), with direction-
//! dependent vehicle release. For at most four tasks, this reference enumerates
//! L1 sign orthants and handoff active regions, then all vertices of each
//! resulting LP. It is exact up to floating-point tolerances for this declared
//! piecewise-affine model; it is not the paper's scalable S6-B MILP.

use std::collections::{BTreeMap, BTreeSet};

const EPS: f64 = 1.0e-8;
const COST_TOLERANCE: f64 = 1.0e-6;
pub const MAX_CONTINUOUS_TASKS: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TaskDirection {
    Loading,
    Unloading,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousTask {
    pub task_id: String,
    pub qc_id: String,
    pub qc_predecessor: Option<String>,
    pub qc_release_at: f64,
    pub nominal_service: f64,
    pub service_deviation: f64,
    pub direction: TaskDirection,
    /// Post-service loaded delivery to the yard; used for unloading tasks.
    pub loaded_delivery_after: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct VehicleRoute {
    pub vehicle_id: String,
    pub task_order: Vec<String>,
    /// Travel from the previous task release, or the depot for the first task,
    /// to the current task's QC handoff.
    pub travel_before: Vec<f64>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CandidatePlan {
    pub plan_id: String,
    pub vehicle_routes: Vec<VehicleRoute>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousInstance {
    pub tasks: Vec<ContinuousTask>,
    pub plans: Vec<CandidatePlan>,
    pub qc_wait_weight: f64,
    pub agv_wait_weight: f64,
    pub gamma: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TaskEvent {
    pub task_id: String,
    pub qc_ready: f64,
    pub agv_ready: f64,
    pub service_start: f64,
    pub service_finish: f64,
    pub vehicle_release: f64,
    pub qc_wait: f64,
    pub agv_wait: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlanEvaluation {
    pub plan_id: String,
    pub nominal_cost: f64,
    pub worst_case_cost: f64,
    pub witness: Vec<(String, f64)>,
    pub worst_case_qc_wait: f64,
    pub worst_case_agv_wait: f64,
    pub worst_case_makespan: f64,
    pub worst_case_events: Vec<TaskEvent>,
    pub active_set_combinations_examined: usize,
    pub feasible_region_vertices: usize,
}

#[derive(Clone, Debug, PartialEq)]
pub struct ContinuousTournament {
    pub gamma: f64,
    pub nominal_best_plan: String,
    pub robust_best_plan: String,
    pub plans: Vec<PlanEvaluation>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum ContinuousError {
    EmptyTasks,
    EmptyPlans,
    TooManyTasks(usize),
    InvalidGamma(f64),
    InvalidObjectiveWeights,
    EmptyIdentifier,
    DuplicateTask(String),
    DuplicatePlan(String),
    DuplicateVehicle(String),
    EmptyVehicleRoute(String),
    RouteTravelLength(String),
    DuplicateRouteTask(String),
    MissingRouteTask(String),
    UnknownTask(String),
    UnknownQcPredecessor(String, String),
    QcPredecessorMismatch(String, String),
    PrecedenceCycle,
    InvalidTaskTimes(String),
    InvalidDelivery(String),
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

    fn scale(&self, factor: f64) -> Self {
        Self {
            constant: self.constant * factor,
            coefficients: self
                .coefficients
                .iter()
                .map(|value| value * factor)
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
struct PlanStructure {
    task_index: BTreeMap<String, usize>,
    topo_order: Vec<String>,
    qc_predecessor: BTreeMap<String, Option<String>>,
    vehicle_predecessor: BTreeMap<String, Option<String>>,
    travel_before: BTreeMap<String, f64>,
}

#[derive(Clone, Debug)]
struct AffinePlanModel {
    objective: Affine,
    branch_constraints: Vec<LinearConstraint>,
}

#[derive(Clone, Debug)]
struct Replay {
    objective: f64,
    qc_wait: f64,
    agv_wait: f64,
    makespan: f64,
    events: Vec<TaskEvent>,
}

fn validate_plan(
    instance: &ContinuousInstance,
    plan: &CandidatePlan,
) -> Result<PlanStructure, ContinuousError> {
    if plan.plan_id.is_empty() {
        return Err(ContinuousError::EmptyIdentifier);
    }
    let mut task_index = BTreeMap::new();
    for (index, task) in instance.tasks.iter().enumerate() {
        if task.task_id.is_empty() || task.qc_id.is_empty() {
            return Err(ContinuousError::EmptyIdentifier);
        }
        if task_index.insert(task.task_id.clone(), index).is_some() {
            return Err(ContinuousError::DuplicateTask(task.task_id.clone()));
        }
        if !task.qc_release_at.is_finite()
            || !task.nominal_service.is_finite()
            || !task.service_deviation.is_finite()
            || task.qc_release_at < 0.0
            || task.service_deviation < 0.0
            || task.nominal_service - task.service_deviation < 1.0
        {
            return Err(ContinuousError::InvalidTaskTimes(task.task_id.clone()));
        }
        if !task.loaded_delivery_after.is_finite() || task.loaded_delivery_after < 0.0 {
            return Err(ContinuousError::InvalidDelivery(task.task_id.clone()));
        }
        if task.direction == TaskDirection::Loading && task.loaded_delivery_after.abs() > EPS {
            return Err(ContinuousError::InvalidDelivery(task.task_id.clone()));
        }
    }

    let mut vehicle_ids = BTreeSet::new();
    let mut route_task_ids = BTreeSet::new();
    let mut qc_predecessor = BTreeMap::new();
    let mut vehicle_predecessor = BTreeMap::new();
    let mut travel_before = BTreeMap::new();

    for route in &plan.vehicle_routes {
        if route.vehicle_id.is_empty() {
            return Err(ContinuousError::EmptyIdentifier);
        }
        if !vehicle_ids.insert(route.vehicle_id.clone()) {
            return Err(ContinuousError::DuplicateVehicle(route.vehicle_id.clone()));
        }
        if route.task_order.is_empty() {
            return Err(ContinuousError::EmptyVehicleRoute(route.vehicle_id.clone()));
        }
        if route.travel_before.len() != route.task_order.len() {
            return Err(ContinuousError::RouteTravelLength(route.vehicle_id.clone()));
        }
        let mut previous: Option<String> = None;
        for (task_id, travel) in route.task_order.iter().zip(&route.travel_before) {
            if !travel.is_finite() || *travel < 0.0 {
                return Err(ContinuousError::InvalidTaskTimes(task_id.clone()));
            }
            if !task_index.contains_key(task_id) {
                return Err(ContinuousError::UnknownTask(task_id.clone()));
            }
            if !route_task_ids.insert(task_id.clone()) {
                return Err(ContinuousError::DuplicateRouteTask(task_id.clone()));
            }
            vehicle_predecessor.insert(task_id.clone(), previous.clone());
            travel_before.insert(task_id.clone(), *travel);
            previous = Some(task_id.clone());
        }
    }

    for task in &instance.tasks {
        if !route_task_ids.contains(&task.task_id) {
            return Err(ContinuousError::MissingRouteTask(task.task_id.clone()));
        }
        if let Some(predecessor_id) = &task.qc_predecessor {
            let predecessor_index = task_index.get(predecessor_id).ok_or_else(|| {
                ContinuousError::UnknownQcPredecessor(
                    task.task_id.clone(),
                    predecessor_id.clone(),
                )
            })?;
            let predecessor = &instance.tasks[*predecessor_index];
            if predecessor.qc_id != task.qc_id {
                return Err(ContinuousError::QcPredecessorMismatch(
                    task.task_id.clone(),
                    predecessor_id.clone(),
                ));
            }
        }
        qc_predecessor.insert(task.task_id.clone(), task.qc_predecessor.clone());
    }

    // Kahn topological order on the union of QC-precedence and vehicle-route
    // predecessor arcs. The same predecessor on both arcs is counted once.
    let mut indegree: BTreeMap<String, usize> = instance
        .tasks
        .iter()
        .map(|task| (task.task_id.clone(), 0))
        .collect();
    let mut successors: BTreeMap<String, BTreeSet<String>> = BTreeMap::new();

    for task in &instance.tasks {
        let mut predecessors = BTreeSet::new();
        if let Some(predecessor) = &task.qc_predecessor {
            predecessors.insert(predecessor.clone());
        }
        if let Some(predecessor) = vehicle_predecessor
            .get(&task.task_id)
            .and_then(|value| value.clone())
        {
            predecessors.insert(predecessor);
        }
        for predecessor in predecessors {
            *indegree.get_mut(&task.task_id).ok_or(ContinuousError::PrecedenceCycle)? += 1;
            successors
                .entry(predecessor)
                .or_default()
                .insert(task.task_id.clone());
        }
    }

    let mut ready: BTreeSet<String> = indegree
        .iter()
        .filter_map(|(task_id, degree)| (*degree == 0).then_some(task_id.clone()))
        .collect();
    let mut topo_order = Vec::with_capacity(instance.tasks.len());
    while let Some(task_id) = ready.pop_first() {
        topo_order.push(task_id.clone());
        if let Some(next_tasks) = successors.get(&task_id) {
            for next in next_tasks {
                let degree = indegree
                    .get_mut(next)
                    .ok_or(ContinuousError::PrecedenceCycle)?;
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(next.clone());
                }
            }
        }
    }
    if topo_order.len() != instance.tasks.len() {
        return Err(ContinuousError::PrecedenceCycle);
    }

    Ok(PlanStructure {
        task_index,
        topo_order,
        qc_predecessor,
        vehicle_predecessor,
        travel_before,
    })
}

fn build_affine_plan_model(
    instance: &ContinuousInstance,
    structure: &PlanStructure,
    branch_mask: usize,
) -> AffinePlanModel {
    let dimensions = instance.tasks.len();
    let mut qc_finish: BTreeMap<String, Affine> = BTreeMap::new();
    let mut vehicle_release: BTreeMap<String, Affine> = BTreeMap::new();
    let mut objective = Affine::constant(0.0, dimensions);
    let mut branch_constraints = Vec::with_capacity(dimensions);

    for task_id in &structure.topo_order {
        let index = structure.task_index[task_id];
        let task = &instance.tasks[index];

        let qc_ready = match &structure.qc_predecessor[task_id] {
            Some(predecessor_id) => qc_finish[predecessor_id].clone(),
            None => Affine::constant(task.qc_release_at, dimensions),
        };
        let agv_ready = match &structure.vehicle_predecessor[task_id] {
            Some(predecessor_id) => vehicle_release[predecessor_id]
                .add(&Affine::constant(structure.travel_before[task_id], dimensions)),
            None => Affine::constant(structure.travel_before[task_id], dimensions),
        };

        let difference = qc_ready.sub(&agv_ready);
        let choose_agv = (branch_mask & (1usize << index)) != 0;
        // u_i = sign_i*x_i. If QC wins, qc_ready-agv_ready >= 0;
        // otherwise agv_ready-qc_ready >= 0.
        let branch = if choose_agv {
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
        branch_constraints.push((index, branch));

        let mut service = Affine::constant(task.nominal_service, dimensions);
        service.coefficients[index] = task.service_deviation;
        let start = if choose_agv { agv_ready.clone() } else { qc_ready.clone() };
        let finish = start.add(&service);
        let release = match task.direction {
            TaskDirection::Loading => start.clone(),
            TaskDirection::Unloading => finish.add(&Affine::constant(
                task.loaded_delivery_after,
                dimensions,
            )),
        };

        let qc_wait = start.sub(&qc_ready).scale(instance.qc_wait_weight);
        let agv_wait = start.sub(&agv_ready).scale(instance.agv_wait_weight);
        objective = objective.add(&qc_wait).add(&agv_wait);
        qc_finish.insert(task_id.clone(), finish);
        vehicle_release.insert(task_id.clone(), release);
    }

    // Keep branch constraints in task-index order for deterministic receipts.
    branch_constraints.sort_by_key(|(index, _)| *index);
    AffinePlanModel {
        objective,
        branch_constraints: branch_constraints
            .into_iter()
            .map(|(_, constraint)| constraint)
            .collect(),
    }
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

fn replay(
    instance: &ContinuousInstance,
    structure: &PlanStructure,
    witness: &[f64],
) -> Replay {
    let mut qc_finish: BTreeMap<String, f64> = BTreeMap::new();
    let mut vehicle_release: BTreeMap<String, f64> = BTreeMap::new();
    let mut events = Vec::with_capacity(instance.tasks.len());
    let mut qc_wait_total = 0.0;
    let mut agv_wait_total = 0.0;
    let mut makespan: f64 = 0.0;

    for task_id in &structure.topo_order {
        let index = structure.task_index[task_id];
        let task = &instance.tasks[index];
        let qc_ready = match &structure.qc_predecessor[task_id] {
            Some(predecessor_id) => qc_finish[predecessor_id],
            None => task.qc_release_at,
        };
        let agv_ready = match &structure.vehicle_predecessor[task_id] {
            Some(predecessor_id) => {
                vehicle_release[predecessor_id] + structure.travel_before[task_id]
            }
            None => structure.travel_before[task_id],
        };
        let start = qc_ready.max(agv_ready);
        let service = task.nominal_service + task.service_deviation * witness[index];
        let finish = start + service;
        let release = match task.direction {
            TaskDirection::Loading => start,
            TaskDirection::Unloading => finish + task.loaded_delivery_after,
        };
        let qc_wait = start - qc_ready;
        let agv_wait = start - agv_ready;
        qc_wait_total += qc_wait;
        agv_wait_total += agv_wait;
        makespan = makespan.max(finish).max(release);
        qc_finish.insert(task_id.clone(), finish);
        vehicle_release.insert(task_id.clone(), release);
        events.push(TaskEvent {
            task_id: task_id.clone(),
            qc_ready,
            agv_ready,
            service_start: start,
            service_finish: finish,
            vehicle_release: release,
            qc_wait,
            agv_wait,
        });
    }

    Replay {
        objective: instance.qc_wait_weight * qc_wait_total
            + instance.agv_wait_weight * agv_wait_total,
        qc_wait: qc_wait_total,
        agv_wait: agv_wait_total,
        makespan,
        events,
    }
}

fn evaluate_plan(
    instance: &ContinuousInstance,
    structure: &PlanStructure,
) -> Result<PlanEvaluation, ContinuousError> {
    let dimensions = instance.tasks.len();
    let signs_count = 1usize << dimensions;
    let branches_count = 1usize << dimensions;
    let mut best: Option<(f64, Vec<f64>)> = None;
    let mut active_set_combinations_examined = 0usize;
    let mut feasible_region_vertices = 0usize;

    for sign_mask in 0..signs_count {
        let signs: Vec<f64> = (0..dimensions)
            .map(|index| if (sign_mask & (1usize << index)) == 0 { 1.0 } else { -1.0 })
            .collect();
        for branch_mask in 0..branches_count {
            let model = build_affine_plan_model(instance, structure, branch_mask);
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
                upper_bound: instance.gamma,
            });
            for branch in &model.branch_constraints {
                constraints.push(LinearConstraint {
                    coefficients: branch
                        .coefficients
                        .iter()
                        .zip(&signs)
                        .map(|(coefficient, sign)| coefficient * sign)
                        .collect(),
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

            enumerate_combinations(&constraints, dimensions, &mut |indices| {
                active_set_combinations_examined += 1;
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
                feasible_region_vertices += 1;
                let cost = model.objective.constant
                    + objective_coefficients
                        .iter()
                        .zip(&point)
                        .map(|(coefficient, coordinate)| coefficient * coordinate)
                        .sum::<f64>();
                let witness: Vec<f64> = point
                    .iter()
                    .zip(&signs)
                    .map(|(coordinate, sign)| coordinate * sign)
                    .collect();
                let replace = match &best {
                    None => true,
                    Some((current_cost, current_witness)) => {
                        cost > *current_cost + EPS
                            || ((cost - *current_cost).abs() <= EPS
                                && witness < *current_witness)
                    }
                };
                if replace {
                    best = Some((cost, witness));
                }
            });
        }
    }

    let (worst_case_cost, witness) = best.ok_or(ContinuousError::NoFeasibleActiveRegion)?;
    let worst = replay(instance, structure, &witness);
    if (worst.objective - worst_case_cost).abs() > COST_TOLERANCE {
        return Err(ContinuousError::NumericalMismatch {
            lp_cost: worst_case_cost,
            replay_cost: worst.objective,
        });
    }
    let nominal = replay(instance, structure, &vec![0.0; dimensions]);
    let witness = instance
        .tasks
        .iter()
        .zip(&witness)
        .map(|(task, value)| (task.task_id.clone(), *value))
        .collect();

    Ok(PlanEvaluation {
        plan_id: plan.plan_id.clone(),
        nominal_cost: nominal.objective,
        worst_case_cost,
        witness,
        worst_case_qc_wait: worst.qc_wait,
        worst_case_agv_wait: worst.agv_wait,
        worst_case_makespan: worst.makespan,
        worst_case_events: worst.events,
        active_set_combinations_examined,
        feasible_region_vertices,
    })
}

/// Evaluates fixed multi-vehicle route plans over the continuous two-sided L1
/// service-time uncertainty set. This reference is exact (up to numeric
/// tolerance) only for at most four tasks and the stated handoff/release model.
pub fn evaluate_continuous_tournament(
    instance: &ContinuousInstance,
) -> Result<ContinuousTournament, ContinuousError> {
    let count = instance.tasks.len();
    if count == 0 {
        return Err(ContinuousError::EmptyTasks);
    }
    if count > MAX_CONTINUOUS_TASKS {
        return Err(ContinuousError::TooManyTasks(count));
    }
    if instance.plans.is_empty() {
        return Err(ContinuousError::EmptyPlans);
    }
    if !instance.gamma.is_finite() || instance.gamma < 0.0 || instance.gamma > count as f64 {
        return Err(ContinuousError::InvalidGamma(instance.gamma));
    }
    if !instance.qc_wait_weight.is_finite()
        || !instance.agv_wait_weight.is_finite()
        || instance.qc_wait_weight < 0.0
        || instance.agv_wait_weight < 0.0
    {
        return Err(ContinuousError::InvalidObjectiveWeights);
    }

    let mut plan_ids = BTreeSet::new();
    let mut plans = Vec::new();
    for plan in &instance.plans {
        if !plan_ids.insert(plan.plan_id.clone()) {
            return Err(ContinuousError::DuplicatePlan(plan.plan_id.clone()));
        }
        let structure = validate_plan(instance, plan)?;
        plans.push(evaluate_plan(instance, &structure)?);
    }
    plans.sort_by(|left, right| left.plan_id.cmp(&right.plan_id));

    let nominal_best_plan = plans
        .iter()
        .min_by(|left, right| {
            left.nominal_cost
                .total_cmp(&right.nominal_cost)
                .then_with(|| left.plan_id.cmp(&right.plan_id))
        })
        .ok_or(ContinuousError::EmptyPlans)?
        .plan_id
        .clone();
    let robust_best_plan = plans
        .iter()
        .min_by(|left, right| {
            left.worst_case_cost
                .total_cmp(&right.worst_case_cost)
                .then_with(|| left.plan_id.cmp(&right.plan_id))
        })
        .ok_or(ContinuousError::EmptyPlans)?
        .plan_id
        .clone();

    Ok(ContinuousTournament {
        gamma: instance.gamma,
        nominal_best_plan,
        robust_best_plan,
        plans,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn instance(gamma: f64) -> ContinuousInstance {
        ContinuousInstance {
            tasks: vec![
                ContinuousTask {
                    task_id: "A".to_owned(),
                    qc_id: "QC-1".to_owned(),
                    qc_predecessor: None,
                    qc_release_at: 0.0,
                    nominal_service: 10.0,
                    service_deviation: 2.0,
                    direction: TaskDirection::Unloading,
                    loaded_delivery_after: 3.0,
                },
                ContinuousTask {
                    task_id: "B".to_owned(),
                    qc_id: "QC-2".to_owned(),
                    qc_predecessor: None,
                    qc_release_at: 0.0,
                    nominal_service: 8.0,
                    service_deviation: 3.0,
                    direction: TaskDirection::Loading,
                    loaded_delivery_after: 0.0,
                },
                ContinuousTask {
                    task_id: "C".to_owned(),
                    qc_id: "QC-1".to_owned(),
                    qc_predecessor: Some("A".to_owned()),
                    qc_release_at: 0.0,
                    nominal_service: 7.0,
                    service_deviation: 2.0,
                    direction: TaskDirection::Unloading,
                    loaded_delivery_after: 2.0,
                },
            ],
            plans: vec![
                CandidatePlan {
                    plan_id: "PLAN-ABC".to_owned(),
                    vehicle_routes: vec![
                        VehicleRoute {
                            vehicle_id: "AGV-1".to_owned(),
                            task_order: vec!["A".to_owned()],
                            travel_before: vec![0.0],
                        },
                        VehicleRoute {
                            vehicle_id: "AGV-2".to_owned(),
                            task_order: vec!["B".to_owned(), "C".to_owned()],
                            travel_before: vec![0.0, 1.0],
                        },
                    ],
                },
                CandidatePlan {
                    plan_id: "PLAN-ACB".to_owned(),
                    vehicle_routes: vec![
                        VehicleRoute {
                            vehicle_id: "AGV-1".to_owned(),
                            task_order: vec!["A".to_owned(), "C".to_owned()],
                            travel_before: vec![0.0, 0.0],
                        },
                        VehicleRoute {
                            vehicle_id: "AGV-2".to_owned(),
                            task_order: vec!["B".to_owned()],
                            travel_before: vec![0.0],
                        },
                    ],
                },
            ],
            qc_wait_weight: 5.0,
            agv_wait_weight: 1.0,
            gamma,
        }
    }

    #[test]
    fn zero_budget_is_exactly_nominal() {
        let result = evaluate_continuous_tournament(&instance(0.0)).unwrap();
        for plan in result.plans {
            assert!((plan.nominal_cost - plan.worst_case_cost).abs() < COST_TOLERANCE);
            assert!(plan.witness.iter().all(|(_, value)| value.abs() < EPS));
        }
    }

    #[test]
    fn fractional_budget_witness_satisfies_continuous_l1_set() {
        let result = evaluate_continuous_tournament(&instance(0.5)).unwrap();
        for plan in result.plans {
            assert!(
                plan.witness.iter().all(|(_, value)| value.abs() <= 1.0 + EPS)
            );
            assert!(
                plan.witness.iter().map(|(_, value)| value.abs()).sum::<f64>()
                    <= 0.5 + EPS
            );
            assert!(plan.worst_case_cost + EPS >= plan.nominal_cost);
        }
    }

    #[test]
    fn direction_dependent_loading_and_unloading_release_is_explicit() {
        let result = evaluate_continuous_tournament(&instance(1.0)).unwrap();
        for plan in result.plans {
            let loading_b = plan
                .worst_case_events
                .iter()
                .find(|event| event.task_id == "B")
                .unwrap();
            let unloading_a = plan
                .worst_case_events
                .iter()
                .find(|event| event.task_id == "A")
                .unwrap();
            assert!((loading_b.vehicle_release - loading_b.service_start).abs() < EPS);
            assert!(
                (unloading_a.vehicle_release
                    - (unloading_a.service_finish + 3.0))
                    .abs()
                    < EPS
            );
        }
    }

    #[test]
    fn continuous_solver_returns_a_replayable_adversarial_witness() {
        let result = evaluate_continuous_tournament(&instance(1.0)).unwrap();
        assert!(result
            .plans
            .iter()
            .all(|plan| plan.active_set_combinations_examined > 0
                && plan.feasible_region_vertices > 0
                && plan.worst_case_makespan >= 0.0));
        assert!(!result.nominal_best_plan.is_empty());
        assert!(!result.robust_best_plan.is_empty());
    }

    #[test]
    fn combined_qc_and_vehicle_precedence_cycles_are_rejected() {
        let mut invalid = instance(1.0);
        invalid.tasks[0].qc_predecessor = Some("C".to_owned());
        assert_eq!(
            evaluate_continuous_tournament(&invalid),
            Err(ContinuousError::PrecedenceCycle)
        );
    }

    #[test]
    fn exact_active_region_reference_caps_task_count() {
        let mut invalid = instance(1.0);
        for suffix in ["D", "E"] {
            invalid.tasks.push(ContinuousTask {
                task_id: suffix.to_owned(),
                qc_id: format!("QC-{suffix}"),
                qc_predecessor: None,
                qc_release_at: 0.0,
                nominal_service: 2.0,
                service_deviation: 1.0,
                direction: TaskDirection::Loading,
                loaded_delivery_after: 0.0,
            });
        }
        assert_eq!(
            evaluate_continuous_tournament(&invalid),
            Err(ContinuousError::TooManyTasks(4))
        );
    }
}
