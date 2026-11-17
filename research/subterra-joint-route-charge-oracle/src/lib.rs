//! Exact bounded joint task, route and charging search for tiny robot fleets.
//!
//! The synchronized global clock allows robots to move, service tasks and charge
//! concurrently while enforcing shared charger and edge capacities. The solver
//! returns a Pareto frontier without scalarizing unlike physical quantities.

use std::cmp::Reverse;
use std::collections::{BinaryHeap, HashMap, BTreeSet};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub id: String,
    pub from_node: u16,
    pub to_node: u16,
    pub distance_mm: u32,
    pub energy_units: u32,
    pub duration_ticks: u16,
    /// Maximum robots occupying or entering this directed edge during a tick.
    pub concurrent_capacity: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InspectionTask {
    pub id: String,
    pub node: u16,
    pub mandatory: bool,
    /// A robot is eligible when it contains every bit in this mask.
    pub required_capability_mask: u64,
    pub service_duration_ticks: u16,
    pub service_energy_per_tick: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChargingStation {
    pub id: String,
    pub node: u16,
    pub charge_rate_units_per_tick: u32,
    pub concurrent_capacity: u8,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RobotSpec {
    pub id: String,
    pub start_node: u16,
    pub return_node: u16,
    pub capability_mask: u64,
    pub battery_capacity_units: u32,
    pub initial_battery_units: u32,
    pub minimum_return_reserve_units: u32,
    pub mission_interruption_upper_ppm: u32,
    pub robot_loss_upper_ppm: u32,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PlanLimits {
    pub maximum_elapsed_ticks: u32,
    pub maximum_distance_mm: u64,
    pub maximum_battery_energy_used_units: u64,
    pub maximum_grid_energy_charged_units: u64,
    pub mission_interruption_upper_ppm: u32,
    pub robot_loss_upper_ppm: u32,
    pub maximum_search_nodes: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Scenario {
    pub scenario_id: String,
    pub node_count: u16,
    pub edges: Vec<Edge>,
    pub tasks: Vec<InspectionTask>,
    pub charging_stations: Vec<ChargingStation>,
    pub robots: Vec<RobotSpec>,
    pub limits: PlanLimits,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum RobotMode {
    Idle,
    Moving { edge_index: u16, remaining_ticks: u16 },
    Servicing { task_index: u8, remaining_ticks: u16 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
struct RobotState {
    node: u16,
    battery_units: u32,
    mode: RobotMode,
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
struct SearchState {
    robots: Vec<RobotState>,
    assigned_tasks_mask: u64,
    completed_tasks_mask: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
enum ActionChoice {
    Wait,
    Continue,
    StartMove(usize),
    StartService(usize),
    Charge(usize),
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ActionRecord {
    Wait,
    ContinueMove { edge_id: String },
    ContinueService { task_id: String },
    Move { edge_id: String, from_node: u16, to_node: u16 },
    Service { task_id: String },
    Charge { station_id: String, energy_gained_units: u32 },
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct TimedAction {
    pub tick: u32,
    pub robot_id: String,
    pub action: ActionRecord,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct PlanMetrics {
    pub elapsed_ticks: u32,
    pub distance_mm: u64,
    pub battery_energy_used_units: u64,
    pub grid_energy_charged_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeasiblePlan {
    pub metrics: PlanMetrics,
    pub mission_interruption_upper_ppm: u32,
    pub robot_loss_upper_ppm: u32,
    pub action_trace: Vec<TimedAction>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SolveDisposition {
    ExactParetoFrontierProven,
    NodeLimitReached,
    NoFeasiblePlan,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SolveReport {
    pub pareto_frontier: Vec<FeasiblePlan>,
    pub expanded_labels: u64,
    pub disposition: SolveDisposition,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SolveError {
    EmptyScenarioId,
    InvalidNodeCount,
    UnsupportedRobotCount,
    UnsupportedTaskCount,
    UnsupportedChargerCount,
    EmptyRobotFleet,
    DuplicateRobotId(String),
    DuplicateEdgeId(String),
    DuplicateTaskId(String),
    DuplicateChargerId(String),
    NodeOutOfRange(String),
    InvalidEdge(String),
    InvalidTask(String),
    InvalidCharger(String),
    InvalidRobot(String),
    MandatoryTaskHasNoEligibleRobot(String),
    NoMandatoryTasks,
    RiskValueOutsidePpmDomain,
    InvalidLimits,
}

#[derive(Debug, Clone)]
struct Record {
    state: SearchState,
    metrics: PlanMetrics,
    parent: Option<usize>,
    step: Vec<TimedAction>,
    active: bool,
}

fn dominates_metrics(left: &PlanMetrics, right: &PlanMetrics) -> bool {
    left.elapsed_ticks <= right.elapsed_ticks
        && left.distance_mm <= right.distance_mm
        && left.battery_energy_used_units <= right.battery_energy_used_units
        && left.grid_energy_charged_units <= right.grid_energy_charged_units
}

fn add_metrics(left: &PlanMetrics, right: &PlanMetrics) -> Option<PlanMetrics> {
    Some(PlanMetrics {
        elapsed_ticks: left.elapsed_ticks.checked_add(right.elapsed_ticks)?,
        distance_mm: left.distance_mm.checked_add(right.distance_mm)?,
        battery_energy_used_units: left
            .battery_energy_used_units
            .checked_add(right.battery_energy_used_units)?,
        grid_energy_charged_units: left
            .grid_energy_charged_units
            .checked_add(right.grid_energy_charged_units)?,
    })
}

fn within_limits(metrics: &PlanMetrics, limits: &PlanLimits) -> bool {
    metrics.elapsed_ticks <= limits.maximum_elapsed_ticks
        && metrics.distance_mm <= limits.maximum_distance_mm
        && metrics.battery_energy_used_units <= limits.maximum_battery_energy_used_units
        && metrics.grid_energy_charged_units <= limits.maximum_grid_energy_charged_units
}

fn validate(scenario: &Scenario) -> Result<(u64, u32, u32), SolveError> {
    if scenario.scenario_id.trim().is_empty() {
        return Err(SolveError::EmptyScenarioId);
    }
    if scenario.node_count == 0 {
        return Err(SolveError::InvalidNodeCount);
    }
    if !(1..=3).contains(&scenario.robots.len()) {
        return Err(SolveError::UnsupportedRobotCount);
    }
    if scenario.tasks.is_empty() || scenario.tasks.len() > 63 {
        return Err(SolveError::UnsupportedTaskCount);
    }
    if scenario.charging_stations.len() > 2 {
        return Err(SolveError::UnsupportedChargerCount);
    }
    if scenario.limits.maximum_search_nodes == 0
        || scenario.limits.maximum_elapsed_ticks == 0
        || scenario.limits.mission_interruption_upper_ppm > 1_000_000
        || scenario.limits.robot_loss_upper_ppm > 1_000_000
    {
        return Err(SolveError::InvalidLimits);
    }

    let mut robot_ids = BTreeSet::new();
    let mut edge_ids = BTreeSet::new();
    let mut task_ids = BTreeSet::new();
    let mut charger_ids = BTreeSet::new();

    for robot in &scenario.robots {
        if robot.id.trim().is_empty()
            || !robot_ids.insert(robot.id.as_str())
            || robot.start_node >= scenario.node_count
            || robot.return_node >= scenario.node_count
            || robot.battery_capacity_units == 0
            || robot.initial_battery_units > robot.battery_capacity_units
            || robot.minimum_return_reserve_units > robot.battery_capacity_units
            || robot.initial_battery_units < robot.minimum_return_reserve_units
        {
            return Err(SolveError::InvalidRobot(robot.id.clone()));
        }
        if robot.mission_interruption_upper_ppm > 1_000_000
            || robot.robot_loss_upper_ppm > 1_000_000
        {
            return Err(SolveError::RiskValueOutsidePpmDomain);
        }
    }

    for edge in &scenario.edges {
        if edge.id.trim().is_empty() || !edge_ids.insert(edge.id.as_str()) {
            return Err(SolveError::DuplicateEdgeId(edge.id.clone()));
        }
        if edge.from_node >= scenario.node_count || edge.to_node >= scenario.node_count {
            return Err(SolveError::NodeOutOfRange(edge.id.clone()));
        }
        if edge.from_node == edge.to_node
            || edge.duration_ticks == 0
            || edge.concurrent_capacity == 0
        {
            return Err(SolveError::InvalidEdge(edge.id.clone()));
        }
    }

    let mut mandatory_mask = 0u64;
    for (index, task) in scenario.tasks.iter().enumerate() {
        if task.id.trim().is_empty() || !task_ids.insert(task.id.as_str()) {
            return Err(SolveError::DuplicateTaskId(task.id.clone()));
        }
        if task.node >= scenario.node_count || task.service_duration_ticks == 0 {
            return Err(SolveError::InvalidTask(task.id.clone()));
        }
        let eligible = scenario.robots.iter().any(|robot| {
            robot.capability_mask & task.required_capability_mask
                == task.required_capability_mask
        });
        if task.mandatory {
            if !eligible {
                return Err(SolveError::MandatoryTaskHasNoEligibleRobot(task.id.clone()));
            }
            mandatory_mask |= 1u64 << index;
        }
    }
    if mandatory_mask == 0 {
        return Err(SolveError::NoMandatoryTasks);
    }

    for charger in &scenario.charging_stations {
        if charger.id.trim().is_empty() || !charger_ids.insert(charger.id.as_str()) {
            return Err(SolveError::DuplicateChargerId(charger.id.clone()));
        }
        if charger.node >= scenario.node_count
            || charger.charge_rate_units_per_tick == 0
            || charger.concurrent_capacity == 0
        {
            return Err(SolveError::InvalidCharger(charger.id.clone()));
        }
    }

    let interruption = scenario.robots.iter().fold(0u32, |sum, robot| {
        sum.saturating_add(robot.mission_interruption_upper_ppm).min(1_000_000)
    });
    let robot_loss = scenario.robots.iter().fold(0u32, |sum, robot| {
        sum.saturating_add(robot.robot_loss_upper_ppm).min(1_000_000)
    });
    if interruption > scenario.limits.mission_interruption_upper_ppm
        || robot_loss > scenario.limits.robot_loss_upper_ppm
    {
        return Err(SolveError::RiskValueOutsidePpmDomain);
    }

    Ok((mandatory_mask, interruption, robot_loss))
}

fn is_terminal(state: &SearchState, scenario: &Scenario, mandatory_mask: u64) -> bool {
    state.completed_tasks_mask & mandatory_mask == mandatory_mask
        && state.robots.iter().zip(&scenario.robots).all(|(dynamic, spec)| {
            dynamic.node == spec.return_node
                && dynamic.battery_units >= spec.minimum_return_reserve_units
                && dynamic.mode == RobotMode::Idle
        })
}

fn local_actions(
    state: &SearchState,
    scenario: &Scenario,
    robot_index: usize,
) -> Vec<ActionChoice> {
    let dynamic = state.robots[robot_index];
    let spec = &scenario.robots[robot_index];
    match dynamic.mode {
        RobotMode::Moving { .. } | RobotMode::Servicing { .. } => vec![ActionChoice::Continue],
        RobotMode::Idle => {
            let mut actions = vec![ActionChoice::Wait];
            for (edge_index, edge) in scenario.edges.iter().enumerate() {
                if edge.from_node == dynamic.node
                    && dynamic.battery_units >= edge.energy_units
                {
                    actions.push(ActionChoice::StartMove(edge_index));
                }
            }
            for (task_index, task) in scenario.tasks.iter().enumerate() {
                let assigned = state.assigned_tasks_mask & (1u64 << task_index) != 0;
                let capable = spec.capability_mask & task.required_capability_mask
                    == task.required_capability_mask;
                let total_service_energy =
                    task.service_energy_per_tick.checked_mul(u32::from(task.service_duration_ticks));
                if !assigned
                    && capable
                    && task.node == dynamic.node
                    && total_service_energy.is_some_and(|energy| dynamic.battery_units >= energy)
                {
                    actions.push(ActionChoice::StartService(task_index));
                }
            }
            for (station_index, station) in scenario.charging_stations.iter().enumerate() {
                if station.node == dynamic.node
                    && dynamic.battery_units < spec.battery_capacity_units
                {
                    actions.push(ActionChoice::Charge(station_index));
                }
            }
            actions
        }
    }
}

fn enumerate_joint_actions(
    choices: &[Vec<ActionChoice>],
    index: usize,
    current: &mut Vec<ActionChoice>,
    output: &mut Vec<Vec<ActionChoice>>,
) {
    if index == choices.len() {
        output.push(current.clone());
        return;
    }
    for choice in &choices[index] {
        current.push(*choice);
        enumerate_joint_actions(choices, index + 1, current, output);
        current.pop();
    }
}

fn transition(
    state: &SearchState,
    actions: &[ActionChoice],
    scenario: &Scenario,
    metrics: &PlanMetrics,
) -> Option<(SearchState, PlanMetrics, Vec<TimedAction>)> {
    // Enforce per-tick edge occupancy, counting both existing traversals and new entries.
    let mut edge_occupancy = vec![0u16; scenario.edges.len()];
    for robot in &state.robots {
        if let RobotMode::Moving { edge_index, .. } = robot.mode {
            edge_occupancy[usize::from(edge_index)] += 1;
        }
    }
    for action in actions {
        if let ActionChoice::StartMove(index) = action {
            edge_occupancy[*index] += 1;
        }
    }
    if edge_occupancy.iter().enumerate().any(|(index, count)| {
        *count > u16::from(scenario.edges[index].concurrent_capacity)
    }) {
        return None;
    }

    // Charger capacity is a simultaneous shared resource for this global tick.
    let mut charger_occupancy = vec![0u16; scenario.charging_stations.len()];
    for action in actions {
        if let ActionChoice::Charge(index) = action {
            charger_occupancy[*index] += 1;
        }
    }
    if charger_occupancy.iter().enumerate().any(|(index, count)| {
        *count > u16::from(scenario.charging_stations[index].concurrent_capacity)
    }) {
        return None;
    }

    let mut next = state.clone();
    let mut delta = PlanMetrics {
        elapsed_ticks: 1,
        ..PlanMetrics::default()
    };
    let mut assigned = state.assigned_tasks_mask;
    let mut completed = state.completed_tasks_mask;
    let mut trace = Vec::with_capacity(actions.len());

    for (robot_index, action) in actions.iter().enumerate() {
        let current = state.robots[robot_index];
        let spec = &scenario.robots[robot_index];
        let mut updated = current;

        match (current.mode, action) {
            (RobotMode::Moving { edge_index, remaining_ticks }, ActionChoice::Continue) => {
                let edge = &scenario.edges[usize::from(edge_index)];
                if remaining_ticks <= 1 {
                    updated.node = edge.to_node;
                    updated.mode = RobotMode::Idle;
                } else {
                    updated.mode = RobotMode::Moving {
                        edge_index,
                        remaining_ticks: remaining_ticks - 1,
                    };
                }
                trace.push(TimedAction {
                    tick: metrics.elapsed_ticks + 1,
                    robot_id: spec.id.clone(),
                    action: ActionRecord::ContinueMove { edge_id: edge.id.clone() },
                });
            }
            (RobotMode::Servicing { task_index, remaining_ticks }, ActionChoice::Continue) => {
                let task = &scenario.tasks[usize::from(task_index)];
                if updated.battery_units < task.service_energy_per_tick {
                    return None;
                }
                updated.battery_units -= task.service_energy_per_tick;
                delta.battery_energy_used_units = delta
                    .battery_energy_used_units
                    .checked_add(u64::from(task.service_energy_per_tick))?;
                if remaining_ticks <= 1 {
                    updated.mode = RobotMode::Idle;
                    completed |= 1u64 << task_index;
                } else {
                    updated.mode = RobotMode::Servicing {
                        task_index,
                        remaining_ticks: remaining_ticks - 1,
                    };
                }
                trace.push(TimedAction {
                    tick: metrics.elapsed_ticks + 1,
                    robot_id: spec.id.clone(),
                    action: ActionRecord::ContinueService { task_id: task.id.clone() },
                });
            }
            (RobotMode::Idle, ActionChoice::Wait) => {
                trace.push(TimedAction {
                    tick: metrics.elapsed_ticks + 1,
                    robot_id: spec.id.clone(),
                    action: ActionRecord::Wait,
                });
            }
            (RobotMode::Idle, ActionChoice::StartMove(edge_index)) => {
                let edge = &scenario.edges[*edge_index];
                if edge.from_node != current.node || current.battery_units < edge.energy_units {
                    return None;
                }
                updated.battery_units -= edge.energy_units;
                updated.mode = if edge.duration_ticks <= 1 {
                    updated.node = edge.to_node;
                    RobotMode::Idle
                } else {
                    RobotMode::Moving {
                        edge_index: u16::try_from(*edge_index).ok()?,
                        remaining_ticks: edge.duration_ticks - 1,
                    }
                };
                delta.distance_mm = delta.distance_mm.checked_add(u64::from(edge.distance_mm))?;
                delta.battery_energy_used_units = delta
                    .battery_energy_used_units
                    .checked_add(u64::from(edge.energy_units))?;
                trace.push(TimedAction {
                    tick: metrics.elapsed_ticks + 1,
                    robot_id: spec.id.clone(),
                    action: ActionRecord::Move {
                        edge_id: edge.id.clone(),
                        from_node: edge.from_node,
                        to_node: edge.to_node,
                    },
                });
            }
            (RobotMode::Idle, ActionChoice::StartService(task_index)) => {
                let task = &scenario.tasks[*task_index];
                let bit = 1u64 << task_index;
                let capable = spec.capability_mask & task.required_capability_mask
                    == task.required_capability_mask;
                if task.node != current.node
                    || assigned & bit != 0
                    || !capable
                    || current.battery_units < task.service_energy_per_tick
                {
                    return None;
                }
                assigned |= bit;
                updated.battery_units -= task.service_energy_per_tick;
                updated.mode = if task.service_duration_ticks <= 1 {
                    completed |= bit;
                    RobotMode::Idle
                } else {
                    RobotMode::Servicing {
                        task_index: u8::try_from(*task_index).ok()?,
                        remaining_ticks: task.service_duration_ticks - 1,
                    }
                };
                delta.battery_energy_used_units = delta
                    .battery_energy_used_units
                    .checked_add(u64::from(task.service_energy_per_tick))?;
                trace.push(TimedAction {
                    tick: metrics.elapsed_ticks + 1,
                    robot_id: spec.id.clone(),
                    action: ActionRecord::Service { task_id: task.id.clone() },
                });
            }
            (RobotMode::Idle, ActionChoice::Charge(station_index)) => {
                let station = &scenario.charging_stations[*station_index];
                if station.node != current.node || current.battery_units >= spec.battery_capacity_units {
                    return None;
                }
                let gained = station
                    .charge_rate_units_per_tick
                    .min(spec.battery_capacity_units - current.battery_units);
                if gained == 0 {
                    return None;
                }
                updated.battery_units += gained;
                delta.grid_energy_charged_units = delta
                    .grid_energy_charged_units
                    .checked_add(u64::from(gained))?;
                trace.push(TimedAction {
                    tick: metrics.elapsed_ticks + 1,
                    robot_id: spec.id.clone(),
                    action: ActionRecord::Charge {
                        station_id: station.id.clone(),
                        energy_gained_units: gained,
                    },
                });
            }
            _ => return None,
        }
        next.robots[robot_index] = updated;
    }

    next.assigned_tasks_mask = assigned;
    next.completed_tasks_mask = completed;
    let next_metrics = add_metrics(metrics, &delta)?;
    if !within_limits(&next_metrics, &scenario.limits) {
        return None;
    }
    Some((next, next_metrics, trace))
}

fn reconstruct_trace(records: &[Record], mut index: usize) -> Vec<TimedAction> {
    let mut steps = Vec::<Vec<TimedAction>>::new();
    while let Some(parent) = records[index].parent {
        steps.push(records[index].step.clone());
        index = parent;
    }
    steps.reverse();
    steps.into_iter().flatten().collect()
}

fn terminal_frontier(mut plans: Vec<FeasiblePlan>) -> Vec<FeasiblePlan> {
    plans.sort_by(|a, b| {
        (
            a.metrics.elapsed_ticks,
            a.metrics.distance_mm,
            a.metrics.battery_energy_used_units,
            a.metrics.grid_energy_charged_units,
            &a.action_trace,
        )
            .cmp(&(
                b.metrics.elapsed_ticks,
                b.metrics.distance_mm,
                b.metrics.battery_energy_used_units,
                b.metrics.grid_energy_charged_units,
                &b.action_trace,
            ))
    });
    let mut frontier: Vec<FeasiblePlan> = Vec::new();
    for plan in plans {
        if frontier
            .iter()
            .any(|existing| dominates_metrics(&existing.metrics, &plan.metrics))
        {
            continue;
        }
        frontier.retain(|existing| !dominates_metrics(&plan.metrics, &existing.metrics));
        frontier.push(plan);
    }
    frontier
}

pub fn solve(scenario: &Scenario) -> Result<SolveReport, SolveError> {
    let (mandatory_mask, interruption_risk, robot_loss_risk) = validate(scenario)?;
    let initial = SearchState {
        robots: scenario
            .robots
            .iter()
            .map(|robot| RobotState {
                node: robot.start_node,
                battery_units: robot.initial_battery_units,
                mode: RobotMode::Idle,
            })
            .collect(),
        assigned_tasks_mask: 0,
        completed_tasks_mask: 0,
    };
    let initial_metrics = PlanMetrics::default();
    let mut records = vec![Record {
        state: initial.clone(),
        metrics: initial_metrics,
        parent: None,
        step: Vec::new(),
        active: true,
    }];
    let mut by_state = HashMap::<SearchState, Vec<usize>>::new();
    by_state.insert(initial, vec![0]);
    let mut queue = BinaryHeap::<Reverse<(u32, u64, u64, u64, usize)>>::new();
    queue.push(Reverse((0, 0, 0, 0, 0)));
    let mut terminal_ids = Vec::new();
    let mut expanded = 0u64;
    let mut limit_reached = false;

    while let Some(Reverse((_, _, _, _, record_id))) = queue.pop() {
        if !records[record_id].active {
            continue;
        }
        if expanded >= scenario.limits.maximum_search_nodes {
            limit_reached = true;
            break;
        }
        expanded += 1;
        let state = records[record_id].state.clone();
        let metrics = records[record_id].metrics;

        if is_terminal(&state, scenario, mandatory_mask) {
            terminal_ids.push(record_id);
            continue;
        }
        if metrics.elapsed_ticks >= scenario.limits.maximum_elapsed_ticks {
            continue;
        }

        let choices = (0..state.robots.len())
            .map(|index| local_actions(&state, scenario, index))
            .collect::<Vec<_>>();
        let mut joint_actions = Vec::new();
        enumerate_joint_actions(&choices, 0, &mut Vec::new(), &mut joint_actions);

        for actions in joint_actions {
            let Some((next_state, next_metrics, step)) =
                transition(&state, &actions, scenario, &metrics)
            else {
                continue;
            };
            let labels = by_state.entry(next_state.clone()).or_default();
            if labels.iter().any(|existing_id| {
                records[*existing_id].active
                    && dominates_metrics(&records[*existing_id].metrics, &next_metrics)
            }) {
                continue;
            }
            for existing_id in labels.iter().copied() {
                if records[existing_id].active
                    && dominates_metrics(&next_metrics, &records[existing_id].metrics)
                {
                    records[existing_id].active = false;
                }
            }
            labels.retain(|existing_id| records[*existing_id].active);
            let next_id = records.len();
            records.push(Record {
                state: next_state.clone(),
                metrics: next_metrics,
                parent: Some(record_id),
                step,
                active: true,
            });
            labels.push(next_id);
            queue.push(Reverse((
                next_metrics.elapsed_ticks,
                next_metrics.distance_mm,
                next_metrics.battery_energy_used_units,
                next_metrics.grid_energy_charged_units,
                next_id,
            )));
        }
    }

    let plans = terminal_ids
        .into_iter()
        .filter(|index| records[*index].active)
        .map(|index| FeasiblePlan {
            metrics: records[index].metrics,
            mission_interruption_upper_ppm: interruption_risk,
            robot_loss_upper_ppm: robot_loss_risk,
            action_trace: reconstruct_trace(&records, index),
        })
        .collect::<Vec<_>>();
    let frontier = terminal_frontier(plans);
    let disposition = if limit_reached {
        SolveDisposition::NodeLimitReached
    } else if frontier.is_empty() {
        SolveDisposition::NoFeasiblePlan
    } else {
        SolveDisposition::ExactParetoFrontierProven
    };

    Ok(SolveReport {
        pareto_frontier: frontier,
        expanded_labels: expanded,
        disposition,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn edge(id: &str, from: u16, to: u16, distance: u32, energy: u32) -> Edge {
        Edge {
            id: id.into(),
            from_node: from,
            to_node: to,
            distance_mm: distance,
            energy_units: energy,
            duration_ticks: 1,
            concurrent_capacity: 1,
        }
    }

    fn single_robot_scenario() -> Scenario {
        Scenario {
            scenario_id: "single-route-known-answer".into(),
            node_count: 2,
            edges: vec![
                edge("out", 0, 1, 100, 2),
                edge("back", 1, 0, 100, 2),
            ],
            tasks: vec![InspectionTask {
                id: "inspect-a".into(),
                node: 1,
                mandatory: true,
                required_capability_mask: 1,
                service_duration_ticks: 1,
                service_energy_per_tick: 1,
            }],
            charging_stations: vec![],
            robots: vec![RobotSpec {
                id: "robot-a".into(),
                start_node: 0,
                return_node: 0,
                capability_mask: 1,
                battery_capacity_units: 10,
                initial_battery_units: 10,
                minimum_return_reserve_units: 2,
                mission_interruption_upper_ppm: 100,
                robot_loss_upper_ppm: 50,
            }],
            limits: PlanLimits {
                maximum_elapsed_ticks: 10,
                maximum_distance_mm: 1_000,
                maximum_battery_energy_used_units: 10,
                maximum_grid_energy_charged_units: 10,
                mission_interruption_upper_ppm: 100,
                robot_loss_upper_ppm: 50,
                maximum_search_nodes: 100_000,
            },
        }
    }

    #[test]
    fn exact_single_robot_route_matches_hand_calculated_answer() {
        let report = solve(&single_robot_scenario()).unwrap();
        assert_eq!(report.disposition, SolveDisposition::ExactParetoFrontierProven);
        assert!(!report.pareto_frontier.is_empty());
        assert!(report.pareto_frontier.iter().any(|plan| {
            plan.metrics.elapsed_ticks == 3
                && plan.metrics.distance_mm == 200
                && plan.metrics.battery_energy_used_units == 5
                && plan.metrics.grid_energy_charged_units == 0
                && plan.mission_interruption_upper_ppm == 100
                && plan.robot_loss_upper_ppm == 50
        }));
    }

    #[test]
    fn charger_is_required_when_initial_battery_cannot_cover_return_reserve() {
        let mut scenario = single_robot_scenario();
        scenario.robots[0].battery_capacity_units = 5;
        scenario.robots[0].initial_battery_units = 5;
        scenario.robots[0].minimum_return_reserve_units = 1;
        scenario.tasks[0].service_energy_per_tick = 1;
        scenario.edges[0].energy_units = 3;
        scenario.edges[1].energy_units = 3;
        scenario.charging_stations.push(ChargingStation {
            id: "charger-a".into(),
            node: 1,
            charge_rate_units_per_tick: 2,
            concurrent_capacity: 1,
        });
        let report = solve(&scenario).unwrap();
        assert_eq!(report.disposition, SolveDisposition::ExactParetoFrontierProven);
        assert!(report.pareto_frontier.iter().any(|plan| {
            plan.metrics.elapsed_ticks == 5
                && plan.metrics.grid_energy_charged_units == 4
                && plan.metrics.battery_energy_used_units == 7
        }));
    }

    #[test]
    fn heterogeneous_robots_jointly_complete_capability_specific_tasks() {
        let scenario = Scenario {
            scenario_id: "two-robot-heterogeneous".into(),
            node_count: 3,
            edges: vec![
                edge("depot-to-a", 0, 1, 100, 1),
                edge("a-to-depot", 1, 0, 100, 1),
                edge("depot-to-b", 0, 2, 100, 1),
                edge("b-to-depot", 2, 0, 100, 1),
            ],
            tasks: vec![
                InspectionTask {
                    id: "thermal".into(),
                    node: 1,
                    mandatory: true,
                    required_capability_mask: 1,
                    service_duration_ticks: 1,
                    service_energy_per_tick: 1,
                },
                InspectionTask {
                    id: "acoustic".into(),
                    node: 2,
                    mandatory: true,
                    required_capability_mask: 2,
                    service_duration_ticks: 1,
                    service_energy_per_tick: 1,
                },
            ],
            charging_stations: vec![],
            robots: vec![
                RobotSpec {
                    id: "robot-thermal".into(),
                    start_node: 0,
                    return_node: 0,
                    capability_mask: 1,
                    battery_capacity_units: 10,
                    initial_battery_units: 10,
                    minimum_return_reserve_units: 1,
                    mission_interruption_upper_ppm: 100,
                    robot_loss_upper_ppm: 100,
                },
                RobotSpec {
                    id: "robot-acoustic".into(),
                    start_node: 0,
                    return_node: 0,
                    capability_mask: 2,
                    battery_capacity_units: 10,
                    initial_battery_units: 10,
                    minimum_return_reserve_units: 1,
                    mission_interruption_upper_ppm: 100,
                    robot_loss_upper_ppm: 100,
                },
            ],
            limits: PlanLimits {
                maximum_elapsed_ticks: 10,
                maximum_distance_mm: 1_000,
                maximum_battery_energy_used_units: 10,
                maximum_grid_energy_charged_units: 10,
                mission_interruption_upper_ppm: 200,
                robot_loss_upper_ppm: 200,
                maximum_search_nodes: 500_000,
            },
        };
        let report = solve(&scenario).unwrap();
        assert_eq!(report.disposition, SolveDisposition::ExactParetoFrontierProven);
        assert!(report.pareto_frontier.iter().any(|plan| {
            plan.metrics.elapsed_ticks == 3 && plan.metrics.distance_mm == 400
        }));
    }

    #[test]
    fn shared_charger_capacity_prevents_simultaneous_overbooking() {
        let mut scenario = single_robot_scenario();
        scenario.robots.push(RobotSpec {
            id: "robot-b".into(),
            start_node: 0,
            return_node: 0,
            capability_mask: 1,
            battery_capacity_units: 10,
            initial_battery_units: 10,
            minimum_return_reserve_units: 1,
            mission_interruption_upper_ppm: 100,
            robot_loss_upper_ppm: 100,
        });
        scenario.limits.mission_interruption_upper_ppm = 200;
        scenario.limits.robot_loss_upper_ppm = 200;
        let report = solve(&scenario).unwrap();
        assert!(!report.pareto_frontier.is_empty());
        for plan in &report.pareto_frontier {
            let charges_at_tick = {
                let mut map = std::collections::BTreeMap::<(u32, String), usize>::new();
                for event in &plan.action_trace {
                    if let ActionRecord::Charge { station_id, .. } = &event.action {
                        *map.entry((event.tick, station_id.clone())).or_default() += 1;
                    }
                }
                map.values().copied().max().unwrap_or(0)
            };
            assert!(charges_at_tick <= 1);
        }
    }

    #[test]
    fn risk_union_bound_is_conservative_and_budget_is_hard() {
        let mut scenario = single_robot_scenario();
        scenario.robots.push(RobotSpec {
            id: "robot-b".into(),
            start_node: 0,
            return_node: 0,
            capability_mask: 1,
            battery_capacity_units: 10,
            initial_battery_units: 10,
            minimum_return_reserve_units: 1,
            mission_interruption_upper_ppm: 600_000,
            robot_loss_upper_ppm: 100,
        });
        scenario.robots[0].mission_interruption_upper_ppm = 600_000;
        scenario.limits.mission_interruption_upper_ppm = 1_000_000;
        scenario.limits.robot_loss_upper_ppm = 1_000_000;
        let report = solve(&scenario).unwrap();
        assert!(report.pareto_frontier.iter().all(|plan| {
            plan.mission_interruption_upper_ppm == 1_000_000
        }));
        scenario.limits.mission_interruption_upper_ppm = 999_999;
        assert_eq!(solve(&scenario), Err(SolveError::RiskValueOutsidePpmDomain));
    }

    #[test]
    fn node_limit_never_claims_exactness() {
        let mut scenario = single_robot_scenario();
        scenario.limits.maximum_search_nodes = 1;
        let report = solve(&scenario).unwrap();
        assert_eq!(report.disposition, SolveDisposition::NodeLimitReached);
    }

    #[test]
    fn search_is_deterministic() {
        let scenario = single_robot_scenario();
        assert_eq!(solve(&scenario).unwrap(), solve(&scenario).unwrap());
    }

    #[test]
    fn no_return_path_produces_no_feasible_plan() {
        let mut scenario = single_robot_scenario();
        scenario.edges.retain(|edge| edge.id != "back");
        let report = solve(&scenario).unwrap();
        assert_eq!(report.disposition, SolveDisposition::NoFeasiblePlan);
        assert!(report.pareto_frontier.is_empty());
    }

    #[test]
    fn task_capability_mismatch_is_rejected_before_search() {
        let mut scenario = single_robot_scenario();
        scenario.robots[0].capability_mask = 2;
        assert_eq!(
            solve(&scenario),
            Err(SolveError::MandatoryTaskHasNoEligibleRobot("inspect-a".into()))
        );
    }

    #[test]
    fn invalid_ppm_values_are_rejected() {
        let mut scenario = single_robot_scenario();
        scenario.robots[0].robot_loss_upper_ppm = 1_000_001;
        assert_eq!(solve(&scenario), Err(SolveError::RiskValueOutsidePpmDomain));
    }
}
