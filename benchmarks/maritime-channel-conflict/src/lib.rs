#![forbid(unsafe_code)]

//! Evidence-bounded interval verification for channel and basin movement conflicts.
//!
//! This crate consumes the selected fleet plan. It does not schedule movements,
//! infer navigability, simulate vessel dynamics, calculate collision risk, or
//! authorize a transit. Its graph checks only declared edge/direction/time
//! policies and conflict rules supplied by the caller.

use std::collections::{BTreeMap, BTreeSet};

use maritime_service_chain_contract::TickWindow;
use maritime_service_chain_fleet::{FleetPlan, OperationPlacement};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TopologyReviewState {
    Reviewed,
    Unknown,
    Unsupported,
}

/// A direction policy valid over one half-open time window.
///
/// Multiple windows for an edge must be ordered and non-overlapping. A movement
/// must fit wholly inside one window in this v0 profile.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DirectionWindow {
    pub window: TickWindow,
    pub allowed_direction_ids: Vec<String>,
}

/// A graph edge is a versioned abstract segment-to-edge binding.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelEdge {
    pub segment_id: String,
    pub edge_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub route_revision: String,
    pub direction_windows: Vec<DirectionWindow>,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub struct ConflictMember {
    pub edge_id: String,
    pub direction_id: String,
}

/// A symmetric conflict relation between two edge/direction members.
///
/// minimum_headway_ticks is the required separation between the end of one
/// half-open occupancy interval and the start of the next. Overlap is always a
/// violation, even when minimum headway is zero. Identical members model
/// following movements on one edge and direction.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictRule {
    pub rule_id: String,
    pub conflict_zone_id: String,
    pub member_a: ConflictMember,
    pub member_b: ConflictMember,
    pub minimum_headway_ticks: u64,
}

/// Frozen graph profile. Reviewed is an intake/review state, not proof that
/// topology is physically complete, current, licensed or safe to navigate.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ConflictGraph {
    pub graph_id: String,
    pub route_revision: String,
    pub time_basis_id: String,
    pub topology_state: TopologyReviewState,
    pub edges: Vec<ChannelEdge>,
    pub conflict_rules: Vec<ConflictRule>,
}

/// Binds one exact fleet operation to a directed graph edge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelMovement {
    pub movement_id: String,
    pub operation: OperationPlacement,
    pub edge_id: String,
    pub direction_id: String,
    pub route_revision: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelConflictWorkload {
    pub graph: ConflictGraph,
    /// Exactly one movement is required for each fleet-plan operation whose
    /// segment ID is mapped by the declared graph.
    pub movements: Vec<ChannelMovement>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChannelDisposition {
    ReadyForComparison,
    NeedsEvidence,
    Infeasible,
    Unsupported,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord)]
pub enum ChannelViolation {
    MissingMovement {
        demand_id: String,
        segment_id: String,
    },
    DirectionWindowMissing {
        movement_id: String,
        edge_id: String,
    },
    DirectionNotAllowed {
        movement_id: String,
        edge_id: String,
        direction_id: String,
    },
    ConflictHeadwayViolation {
        rule_id: String,
        first_movement_id: String,
        second_movement_id: String,
        overlaps: bool,
        actual_gap_ticks: u64,
        required_gap_ticks: u64,
    },
}

impl ChannelViolation {
    fn needs_evidence(&self) -> bool {
        matches!(
            self,
            Self::MissingMovement { .. } | Self::DirectionWindowMissing { .. }
        )
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ChannelConflictReport {
    pub disposition: ChannelDisposition,
    pub violations: Vec<ChannelViolation>,
    pub movements_checked: usize,
    pub conflict_pairs_checked: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChannelError {
    EmptyGraphId,
    EmptyRouteRevision,
    EmptyTimeBasis,
    InvalidEdge(String),
    DuplicateEdgeId(String),
    DuplicateSegmentId(String),
    InvalidDirectionWindow(String),
    DuplicateDirectionId(String),
    DuplicateConflictRule(String),
    InvalidConflictRule(String),
    UnknownConflictEdge(String),
    UnknownConflictDirection(String),
    DuplicateConflictRelation(String),
    DuplicateMovement(String),
    InvalidMovement(String),
    UnknownMovementEdge(String),
    RouteRevisionMismatch(String),
    UnknownPlanDemand(String),
    FleetSelectionMismatch(String),
    FleetOperationNotFound(String),
}

fn covers(outer: TickWindow, window: TickWindow) -> bool {
    outer.is_valid()
        && window.is_valid()
        && outer.start_tick <= window.start_tick
        && outer.end_tick >= window.end_tick
}

fn unique_nonempty(values: &[String]) -> bool {
    let mut seen = BTreeSet::new();
    values
        .iter()
        .all(|value| !value.trim().is_empty() && seen.insert(value))
}

fn direction_exists(edge: &ChannelEdge, direction_id: &str) -> bool {
    edge.direction_windows.iter().any(|window| {
        window
            .allowed_direction_ids
            .iter()
            .any(|id| id == direction_id)
    })
}

fn relation_key(
    member_a: &ConflictMember,
    member_b: &ConflictMember,
) -> (ConflictMember, ConflictMember) {
    if member_a <= member_b {
        (member_a.clone(), member_b.clone())
    } else {
        (member_b.clone(), member_a.clone())
    }
}

fn validate_graph(
    graph: &ConflictGraph,
) -> Result<BTreeMap<&str, &ChannelEdge>, ChannelError> {
    if graph.graph_id.trim().is_empty() {
        return Err(ChannelError::EmptyGraphId);
    }
    if graph.route_revision.trim().is_empty() {
        return Err(ChannelError::EmptyRouteRevision);
    }
    if graph.time_basis_id.trim().is_empty() {
        return Err(ChannelError::EmptyTimeBasis);
    }

    let mut by_edge_id = BTreeMap::new();
    let mut by_segment_id = BTreeSet::new();
    for edge in &graph.edges {
        if edge.segment_id.trim().is_empty()
            || edge.edge_id.trim().is_empty()
            || edge.from_node_id.trim().is_empty()
            || edge.to_node_id.trim().is_empty()
            || edge.from_node_id == edge.to_node_id
            || edge.route_revision.trim().is_empty()
            || edge.route_revision != graph.route_revision
            || edge.direction_windows.is_empty()
        {
            return Err(ChannelError::InvalidEdge(edge.edge_id.clone()));
        }
        if !by_segment_id.insert(edge.segment_id.as_str()) {
            return Err(ChannelError::DuplicateSegmentId(edge.segment_id.clone()));
        }
        if by_edge_id.insert(edge.edge_id.as_str(), edge).is_some() {
            return Err(ChannelError::DuplicateEdgeId(edge.edge_id.clone()));
        }

        let mut previous_end = None;
        for direction_window in &edge.direction_windows {
            if !direction_window.window.is_valid()
                || direction_window.allowed_direction_ids.is_empty()
            {
                return Err(ChannelError::InvalidDirectionWindow(edge.edge_id.clone()));
            }
            if !unique_nonempty(&direction_window.allowed_direction_ids) {
                return Err(ChannelError::DuplicateDirectionId(edge.edge_id.clone()));
            }
            if let Some(end_tick) = previous_end
                && direction_window.window.start_tick < end_tick
            {
                return Err(ChannelError::InvalidDirectionWindow(edge.edge_id.clone()));
            }
            previous_end = Some(direction_window.window.end_tick);
        }
    }

    let mut rule_ids = BTreeSet::new();
    let mut relation_keys = BTreeSet::new();
    for rule in &graph.conflict_rules {
        if rule.rule_id.trim().is_empty()
            || rule.conflict_zone_id.trim().is_empty()
            || rule.member_a.edge_id.trim().is_empty()
            || rule.member_a.direction_id.trim().is_empty()
            || rule.member_b.edge_id.trim().is_empty()
            || rule.member_b.direction_id.trim().is_empty()
        {
            return Err(ChannelError::InvalidConflictRule(rule.rule_id.clone()));
        }
        if !rule_ids.insert(rule.rule_id.as_str()) {
            return Err(ChannelError::DuplicateConflictRule(rule.rule_id.clone()));
        }

        let Some(edge_a) = by_edge_id.get(rule.member_a.edge_id.as_str()).copied() else {
            return Err(ChannelError::UnknownConflictEdge(
                rule.member_a.edge_id.clone(),
            ));
        };
        let Some(edge_b) = by_edge_id.get(rule.member_b.edge_id.as_str()).copied() else {
            return Err(ChannelError::UnknownConflictEdge(
                rule.member_b.edge_id.clone(),
            ));
        };
        if !direction_exists(edge_a, &rule.member_a.direction_id) {
            return Err(ChannelError::UnknownConflictDirection(
                rule.member_a.direction_id.clone(),
            ));
        }
        if !direction_exists(edge_b, &rule.member_b.direction_id) {
            return Err(ChannelError::UnknownConflictDirection(
                rule.member_b.direction_id.clone(),
            ));
        }

        if !relation_keys.insert(relation_key(&rule.member_a, &rule.member_b)) {
            return Err(ChannelError::DuplicateConflictRelation(
                rule.conflict_zone_id.clone(),
            ));
        }
    }

    Ok(by_edge_id)
}

fn validate_plan_selection(
    operation: &OperationPlacement,
    plan: &FleetPlan,
    selections: &BTreeMap<&str, &maritime_service_chain_fleet::DemandSelection>,
) -> Result<(), ChannelError> {
    let Some(selection) = selections.get(operation.demand_id.as_str()) else {
        return Err(ChannelError::UnknownPlanDemand(operation.demand_id.clone()));
    };
    if selection.participant_id != operation.participant_id
        || selection.alternative_id != operation.alternative_id
    {
        return Err(ChannelError::FleetSelectionMismatch(
            operation.demand_id.clone(),
        ));
    }
    if !plan
        .operations
        .iter()
        .any(|candidate| candidate == operation)
    {
        return Err(ChannelError::FleetOperationNotFound(
            operation.segment_id.clone(),
        ));
    }
    Ok(())
}

fn matches_member(movement: &ChannelMovement, member: &ConflictMember) -> bool {
    movement.edge_id == member.edge_id && movement.direction_id == member.direction_id
}

/// Return (overlap, gap) for two half-open intervals.
///
/// Adjacent intervals have a gap of zero and are valid only when the declared
/// minimum headway is zero.
fn interval_gap(first: TickWindow, second: TickWindow) -> (bool, u64) {
    if first.start_tick < second.end_tick && second.start_tick < first.end_tick {
        return (true, 0);
    }
    if first.end_tick <= second.start_tick {
        return (false, second.start_tick - first.end_tick);
    }
    (false, first.start_tick - second.end_tick)
}

fn topology_disposition(state: TopologyReviewState) -> Option<ChannelDisposition> {
    match state {
        TopologyReviewState::Reviewed => None,
        TopologyReviewState::Unknown => Some(ChannelDisposition::NeedsEvidence),
        TopologyReviewState::Unsupported => Some(ChannelDisposition::Unsupported),
    }
}

/// Independently checks an already selected fleet plan against a caller-supplied
/// interval/conflict graph. This function does not search or change the plan.
pub fn evaluate_channel_conflicts(
    plan: &FleetPlan,
    workload: &ChannelConflictWorkload,
) -> Result<ChannelConflictReport, ChannelError> {
    let graph = &workload.graph;
    let edge_by_id = validate_graph(graph)?;

    let mut selections = BTreeMap::new();
    for selection in &plan.selections {
        if selections
            .insert(selection.demand_id.as_str(), selection)
            .is_some()
        {
            return Err(ChannelError::FleetSelectionMismatch(
                selection.demand_id.clone(),
            ));
        }
    }

    let edge_by_segment: BTreeMap<&str, &ChannelEdge> = graph
        .edges
        .iter()
        .map(|edge| (edge.segment_id.as_str(), edge))
        .collect();

    let mut matched_operation_indices = BTreeSet::new();
    let mut movement_ids = BTreeSet::new();
    for movement in &workload.movements {
        if movement.movement_id.trim().is_empty()
            || movement.operation.start_tick >= movement.operation.end_tick
            || movement.direction_id.trim().is_empty()
            || movement.edge_id.trim().is_empty()
            || movement.route_revision.trim().is_empty()
        {
            return Err(ChannelError::InvalidMovement(movement.movement_id.clone()));
        }
        if !movement_ids.insert(movement.movement_id.as_str()) {
            return Err(ChannelError::DuplicateMovement(
                movement.movement_id.clone(),
            ));
        }

        let Some(edge) = edge_by_segment
            .get(movement.operation.segment_id.as_str())
            .copied()
        else {
            return Err(ChannelError::UnknownMovementEdge(movement.edge_id.clone()));
        };
        if edge.edge_id != movement.edge_id {
            return Err(ChannelError::UnknownMovementEdge(movement.edge_id.clone()));
        }
        if edge.route_revision != movement.route_revision
            || graph.route_revision != movement.route_revision
        {
            return Err(ChannelError::RouteRevisionMismatch(
                movement.movement_id.clone(),
            ));
        }
        validate_plan_selection(&movement.operation, plan, &selections)?;

        let matching_index = plan
            .operations
            .iter()
            .position(|operation| operation == &movement.operation)
            .ok_or_else(|| ChannelError::FleetOperationNotFound(movement.movement_id.clone()))?;
        if !matched_operation_indices.insert(matching_index) {
            return Err(ChannelError::DuplicateMovement(
                movement.movement_id.clone(),
            ));
        }
    }

    let mut violations = BTreeSet::new();
    for (operation_index, operation) in plan.operations.iter().enumerate() {
        if edge_by_segment.contains_key(operation.segment_id.as_str())
            && !matched_operation_indices.contains(&operation_index)
        {
            violations.insert(ChannelViolation::MissingMovement {
                demand_id: operation.demand_id.clone(),
                segment_id: operation.segment_id.clone(),
            });
        }
    }

    for movement in &workload.movements {
        let edge = edge_by_id[movement.edge_id.as_str()];
        let operation_window = TickWindow {
            start_tick: movement.operation.start_tick,
            end_tick: movement.operation.end_tick,
        };
        let covering_window = edge
            .direction_windows
            .iter()
            .find(|window| covers(window.window, operation_window));

        match covering_window {
            None => {
                violations.insert(ChannelViolation::DirectionWindowMissing {
                    movement_id: movement.movement_id.clone(),
                    edge_id: edge.edge_id.clone(),
                });
            }
            Some(window) => {
                if !window
                    .allowed_direction_ids
                    .iter()
                    .any(|allowed| allowed == &movement.direction_id)
                {
                    violations.insert(ChannelViolation::DirectionNotAllowed {
                        movement_id: movement.movement_id.clone(),
                        edge_id: edge.edge_id.clone(),
                        direction_id: movement.direction_id.clone(),
                    });
                }
            }
        }
    }

    let mut conflict_pairs_checked = 0;
    for left_index in 0..workload.movements.len() {
        for right_index in (left_index + 1)..workload.movements.len() {
            let first = &workload.movements[left_index];
            let second = &workload.movements[right_index];

            for rule in &graph.conflict_rules {
                let matches_forward =
                    matches_member(first, &rule.member_a) && matches_member(second, &rule.member_b);
                let matches_reverse =
                    matches_member(first, &rule.member_b) && matches_member(second, &rule.member_a);
                if !matches_forward && !matches_reverse {
                    continue;
                }
                conflict_pairs_checked += 1;

                let first_window = TickWindow {
                    start_tick: first.operation.start_tick,
                    end_tick: first.operation.end_tick,
                };
                let second_window = TickWindow {
                    start_tick: second.operation.start_tick,
                    end_tick: second.operation.end_tick,
                };
                let (overlaps, actual_gap_ticks) = interval_gap(first_window, second_window);
                if overlaps || actual_gap_ticks < rule.minimum_headway_ticks {
                    let (first_movement_id, second_movement_id) =
                        if first.movement_id <= second.movement_id {
                            (first.movement_id.clone(), second.movement_id.clone())
                        } else {
                            (second.movement_id.clone(), first.movement_id.clone())
                        };
                    violations.insert(ChannelViolation::ConflictHeadwayViolation {
                        rule_id: rule.rule_id.clone(),
                        first_movement_id,
                        second_movement_id,
                        overlaps,
                        actual_gap_ticks,
                        required_gap_ticks: rule.minimum_headway_ticks,
                    });
                }
            }
        }
    }

    let disposition = if let Some(state) = topology_disposition(graph.topology_state) {
        state
    } else if violations.iter().any(ChannelViolation::needs_evidence) {
        ChannelDisposition::NeedsEvidence
    } else if !violations.is_empty() {
        ChannelDisposition::Infeasible
    } else {
        ChannelDisposition::ReadyForComparison
    };

    Ok(ChannelConflictReport {
        disposition,
        violations: violations.into_iter().collect(),
        movements_checked: workload.movements.len(),
        conflict_pairs_checked,
    })
}
