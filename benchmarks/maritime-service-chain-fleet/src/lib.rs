//! Exact tiny-instance multi-demand scheduler over shared corridor resources.
//!
//! The oracle is deliberately small and effects-disabled. It composes already
//! validated one-demand service-chain cases; it does not duplicate vessel,
//! navigability, lock-eligibility, hydrodynamic, port-call, or participant
//! authority validation.

use std::collections::{BTreeMap, BTreeSet};

use maritime_service_chain_contract::{
    CapacityWindow, EvidenceRef, Resource, ResourceKind, ResourceUse, ResourceUnit,
    ServiceChainCase, TickWindow, ValidationError, VoyageAlternative, validate_case,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ResourceAccountingMode {
    /// Capacity is consumed simultaneously while an operation is active.
    ConcurrentCapacity,
    /// Capacity is a cumulative quota for one declared accounting window.
    CumulativeWindowQuota,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedResourcePolicy {
    pub resource_id: String,
    pub accounting_mode: ResourceAccountingMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetWorkload {
    /// Names the common integer-tick epoch used by every case in this workload.
    pub time_basis_id: String,
    pub cases: Vec<ServiceChainCase>,
    /// Every resource appearing in any case must have exactly one explicit policy.
    pub resource_policies: Vec<SharedResourcePolicy>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchTermination {
    ExhaustiveComplete,
    BudgetExhausted,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OperationPlacement {
    pub demand_id: String,
    pub participant_id: String,
    pub alternative_id: String,
    pub segment_id: String,
    pub start_tick: u64,
    pub end_tick: u64,
    pub resource_uses: Vec<ResourceUse>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DemandSelection {
    pub demand_id: String,
    pub participant_id: String,
    pub alternative_id: String,
    pub delivery_tick: u64,
    pub service_amount: u64,
    pub service_unit: maritime_service_chain_contract::ServiceUnit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FleetObjective {
    pub makespan_tick: u64,
    pub total_delivery_ticks: u128,
    pub total_idle_wait_ticks: u128,
    pub water_ml: u128,
    pub propulsion_wh: u128,
    pub auxiliary_wh: u128,
    pub shore_power_import_wh: u128,
    pub inland_transfer_wh: u128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetPlan {
    pub selections: Vec<DemandSelection>,
    pub operations: Vec<OperationPlacement>,
    pub objective: FleetObjective,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FleetResult {
    pub frontier: Vec<FleetPlan>,
    pub states_explored: u64,
    pub termination: SearchTermination,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FleetError {
    MissingTimeBasis,
    FewerThanTwoDemands,
    DuplicateCaseId(String),
    DuplicateDemandId(String),
    MismatchedCorridor,
    MismatchedDecisionCutoff,
    MismatchedTickResolution,
    DuplicateResourcePolicy(String),
    MissingResourcePolicy(String),
    UnknownResourcePolicy(String),
    ConflictingResourceSnapshot(String),
    InvalidResourceEvidenceSnapshot(String),
    ResourceAccountingModeMismatch(String),
    InvalidSearchBudget,
    InvalidCase {
        case_id: String,
        error: ValidationError,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResourceSnapshot {
    resource: Resource,
    evidence: Vec<EvidenceRef>,
    accounting_mode: ResourceAccountingMode,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct FleetContext {
    resources: BTreeMap<String, ResourceSnapshot>,
    case_order: Vec<usize>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct PartialState {
    operations: Vec<OperationPlacement>,
    selections: Vec<DemandSelection>,
    total_idle_wait_ticks: u128,
    selected_energy: EnergyTotals,
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct EnergyTotals {
    propulsion_wh: u128,
    auxiliary_wh: u128,
    shore_power_import_wh: u128,
    inland_transfer_wh: u128,
}

impl EnergyTotals {
    fn add_alternative(&mut self, alternative: &VoyageAlternative) {
        self.propulsion_wh += u128::from(alternative.energy.propulsion_wh);
        self.auxiliary_wh += u128::from(alternative.energy.auxiliary_wh);
        self.shore_power_import_wh += u128::from(alternative.energy.shore_power_import_wh);
        self.inland_transfer_wh += u128::from(alternative.energy.inland_transfer_wh);
    }
}

fn canonical_resource(resource: &Resource) -> Resource {
    let mut canonical = resource.clone();
    canonical.evidence_ids.sort();
    canonical
}

fn resource_evidence_snapshot(
    case: &ServiceChainCase,
    resource: &Resource,
) -> Result<Vec<EvidenceRef>, FleetError> {
    let mut evidence = Vec::with_capacity(resource.evidence_ids.len());
    for evidence_id in &resource.evidence_ids {
        let Some(item) = case
            .evidence
            .iter()
            .find(|item| item.evidence_id == *evidence_id)
        else {
            return Err(FleetError::InvalidResourceEvidenceSnapshot(
                resource.resource_id.clone(),
            ));
        };
        evidence.push(item.clone());
    }
    evidence.sort_by(|left, right| left.evidence_id.cmp(&right.evidence_id));
    Ok(evidence)
}

fn validate_workload(workload: &FleetWorkload) -> Result<FleetContext, FleetError> {
    if workload.time_basis_id.trim().is_empty() {
        return Err(FleetError::MissingTimeBasis);
    }
    if workload.cases.len() < 2 {
        return Err(FleetError::FewerThanTwoDemands);
    }

    let first = &workload.cases[0];
    let mut case_ids = BTreeSet::new();
    let mut demand_ids = BTreeSet::new();
    let mut policy_by_resource = BTreeMap::new();
    for policy in &workload.resource_policies {
        if policy_by_resource
            .insert(policy.resource_id.clone(), policy.accounting_mode)
            .is_some()
        {
            return Err(FleetError::DuplicateResourcePolicy(
                policy.resource_id.clone(),
            ));
        }
    }

    let mut resources = BTreeMap::<String, ResourceSnapshot>::new();
    for case in &workload.cases {
        validate_case(case).map_err(|error| FleetError::InvalidCase {
            case_id: case.case_id.clone(),
            error,
        })?;
        if !case_ids.insert(case.case_id.clone()) {
            return Err(FleetError::DuplicateCaseId(case.case_id.clone()));
        }
        if !demand_ids.insert(case.demand.demand_id.clone()) {
            return Err(FleetError::DuplicateDemandId(
                case.demand.demand_id.clone(),
            ));
        }
        if case.corridor_id != first.corridor_id {
            return Err(FleetError::MismatchedCorridor);
        }
        if case.decision_tick != first.decision_tick {
            return Err(FleetError::MismatchedDecisionCutoff);
        }
        if case.tick_seconds != first.tick_seconds {
            return Err(FleetError::MismatchedTickResolution);
        }

        for resource in &case.resources {
            let Some(accounting_mode) = policy_by_resource.get(&resource.resource_id).copied()
            else {
                return Err(FleetError::MissingResourcePolicy(
                    resource.resource_id.clone(),
                ));
            };
            let expected_mode = match resource.kind {
                ResourceKind::WaterQuota | ResourceKind::ShorePower => {
                    ResourceAccountingMode::CumulativeWindowQuota
                }
                _ => ResourceAccountingMode::ConcurrentCapacity,
            };
            if accounting_mode != expected_mode {
                return Err(FleetError::ResourceAccountingModeMismatch(
                    resource.resource_id.clone(),
                ));
            }
            let snapshot = ResourceSnapshot {
                resource: canonical_resource(resource),
                evidence: resource_evidence_snapshot(case, resource)?,
                accounting_mode,
            };
            match resources.get(&resource.resource_id) {
                Some(existing) if existing != &snapshot => {
                    return Err(FleetError::ConflictingResourceSnapshot(
                        resource.resource_id.clone(),
                    ));
                }
                Some(_) => {}
                None => {
                    resources.insert(resource.resource_id.clone(), snapshot);
                }
            }
        }
    }

    for policy in policy_by_resource.keys() {
        if !resources.contains_key(policy) {
            return Err(FleetError::UnknownResourcePolicy(policy.clone()));
        }
    }

    let mut case_order: Vec<usize> = (0..workload.cases.len()).collect();
    case_order.sort_by(|left, right| {
        workload.cases[*left]
            .demand
            .demand_id
            .cmp(&workload.cases[*right].demand.demand_id)
    });
    Ok(FleetContext {
        resources,
        case_order,
    })
}

fn operation_interval(start_tick: u64, duration_ticks: u64) -> Option<TickWindow> {
    let end_tick = start_tick.checked_add(duration_ticks)?;
    let interval = TickWindow {
        start_tick,
        end_tick,
    };
    interval.is_valid().then_some(interval)
}

fn covering_window_index(
    resource: &Resource,
    interval: TickWindow,
) -> Option<usize> {
    resource
        .availability
        .iter()
        .position(|window| window.window.covers(interval))
}

fn resource_operation_fits_alone(
    context: &FleetContext,
    use_record: &ResourceUse,
    interval: TickWindow,
) -> bool {
    let Some(snapshot) = context.resources.get(&use_record.resource_id) else {
        return false;
    };
    if snapshot.resource.capacity_unit != use_record.unit {
        return false;
    }
    let Some(window_index) = covering_window_index(&snapshot.resource, interval) else {
        return false;
    };
    let capacity = snapshot.resource.availability[window_index].capacity;
    use_record.amount <= capacity
}

fn concurrent_capacity_fits(
    snapshot: &ResourceSnapshot,
    operations: &[OperationPlacement],
) -> bool {
    let mut grouped = BTreeMap::<usize, Vec<(u64, bool, u64)>>::new();
    for operation in operations {
        for usage in &operation.resource_uses {
            if usage.resource_id != snapshot.resource.resource_id {
                continue;
            }
            if usage.unit != snapshot.resource.capacity_unit {
                return false;
            }
            let Some(interval) = operation_interval(
                operation.start_tick,
                operation.end_tick.saturating_sub(operation.start_tick),
            ) else {
                return false;
            };
            let Some(window_index) = covering_window_index(&snapshot.resource, interval) else {
                return false;
            };
            grouped.entry(window_index).or_default().extend([
                (interval.start_tick, false, usage.amount),
                (interval.end_tick, true, usage.amount),
            ]);
        }
    }

    for (window_index, mut events) in grouped {
        events.sort_by_key(|(tick, is_end, _)| (*tick, !*is_end));
        let capacity = snapshot.resource.availability[window_index].capacity;
        let mut active = 0_u128;
        for (_, is_end, amount) in events {
            if is_end {
                active = active.saturating_sub(u128::from(amount));
            } else {
                active += u128::from(amount);
                if active > u128::from(capacity) {
                    return false;
                }
            }
        }
    }
    true
}

fn cumulative_quota_fits(
    snapshot: &ResourceSnapshot,
    operations: &[OperationPlacement],
) -> bool {
    let mut used_by_window = vec![0_u128; snapshot.resource.availability.len()];
    for operation in operations {
        for usage in &operation.resource_uses {
            if usage.resource_id != snapshot.resource.resource_id {
                continue;
            }
            if usage.unit != snapshot.resource.capacity_unit {
                return false;
            }
            let Some(interval) = operation_interval(
                operation.start_tick,
                operation.end_tick.saturating_sub(operation.start_tick),
            ) else {
                return false;
            };
            let Some(window_index) = covering_window_index(&snapshot.resource, interval) else {
                return false;
            };
            used_by_window[window_index] += u128::from(usage.amount);
        }
    }

    used_by_window
        .iter()
        .zip(&snapshot.resource.availability)
        .all(|(used, window)| *used <= u128::from(window.capacity))
}

fn shared_resources_fit(
    context: &FleetContext,
    operations: &[OperationPlacement],
) -> bool {
    context.resources.values().all(|snapshot| match snapshot.accounting_mode {
        ResourceAccountingMode::ConcurrentCapacity => {
            concurrent_capacity_fits(snapshot, operations)
        }
        ResourceAccountingMode::CumulativeWindowQuota => {
            cumulative_quota_fits(snapshot, operations)
        }
    })
}

fn candidate_operation_fits(
    context: &FleetContext,
    case: &ServiceChainCase,
    segment_index: usize,
    start_tick: u64,
    duration_ticks: u64,
    current_operations: &[OperationPlacement],
) -> Option<OperationPlacement> {
    let interval = operation_interval(start_tick, duration_ticks)?;
    if interval.end_tick >= case.demand.delivery_window.end_tick {
        return None;
    }
    let segment = case.route_segments.get(segment_index)?;
    for usage in &segment.resource_uses {
        if !resource_operation_fits_alone(context, usage, interval) {
            return None;
        }
    }
    let placement = OperationPlacement {
        demand_id: case.demand.demand_id.clone(),
        participant_id: case.demand.participant_id.clone(),
        alternative_id: String::new(),
        segment_id: segment.segment_id.clone(),
        start_tick,
        end_tick: interval.end_tick,
        resource_uses: segment.resource_uses.clone(),
    };
    let mut trial = current_operations.to_vec();
    trial.push(placement.clone());
    if !shared_resources_fit(context, &trial) {
        return None;
    }
    Some(placement)
}

fn objective_for(state: &PartialState) -> FleetObjective {
    let makespan_tick = state
        .selections
        .iter()
        .map(|selection| selection.delivery_tick)
        .max()
        .unwrap_or(0);
    let total_delivery_ticks = state
        .selections
        .iter()
        .map(|selection| u128::from(selection.delivery_tick))
        .sum();
    let water_ml = state
        .operations
        .iter()
        .flat_map(|operation| operation.resource_uses.iter())
        .filter(|usage| usage.unit == ResourceUnit::Millilitres)
        .map(|usage| u128::from(usage.amount))
        .sum();

    FleetObjective {
        makespan_tick,
        total_delivery_ticks,
        total_idle_wait_ticks: state.total_idle_wait_ticks,
        water_ml,
        propulsion_wh: state.selected_energy.propulsion_wh,
        auxiliary_wh: state.selected_energy.auxiliary_wh,
        shore_power_import_wh: state.selected_energy.shore_power_import_wh,
        inland_transfer_wh: state.selected_energy.inland_transfer_wh,
    }
}

fn dominates(left: FleetObjective, right: FleetObjective) -> bool {
    let no_worse = left.makespan_tick <= right.makespan_tick
        && left.total_delivery_ticks <= right.total_delivery_ticks
        && left.total_idle_wait_ticks <= right.total_idle_wait_ticks
        && left.water_ml <= right.water_ml
        && left.propulsion_wh <= right.propulsion_wh
        && left.auxiliary_wh <= right.auxiliary_wh
        && left.shore_power_import_wh <= right.shore_power_import_wh
        && left.inland_transfer_wh <= right.inland_transfer_wh;
    let strictly_better = left.makespan_tick < right.makespan_tick
        || left.total_delivery_ticks < right.total_delivery_ticks
        || left.total_idle_wait_ticks < right.total_idle_wait_ticks
        || left.water_ml < right.water_ml
        || left.propulsion_wh < right.propulsion_wh
        || left.auxiliary_wh < right.auxiliary_wh
        || left.shore_power_import_wh < right.shore_power_import_wh
        || left.inland_transfer_wh < right.inland_transfer_wh;
    no_worse && strictly_better
}

fn pareto_frontier(mut plans: Vec<FleetPlan>) -> Vec<FleetPlan> {
    plans.sort_by_key(|plan| {
        (
            plan.objective.makespan_tick,
            plan.objective.total_delivery_ticks,
            plan.objective.total_idle_wait_ticks,
            plan.objective.water_ml,
            plan.objective.propulsion_wh,
            plan.objective.auxiliary_wh,
            plan.objective.shore_power_import_wh,
            plan.objective.inland_transfer_wh,
        )
    });
    plans.dedup();
    let mut frontier = Vec::new();
    for candidate in plans {
        if frontier
            .iter()
            .any(|known: &FleetPlan| dominates(known.objective, candidate.objective))
        {
            continue;
        }
        frontier.retain(|known| !dominates(candidate.objective, known.objective));
        frontier.push(candidate);
    }
    frontier
}

fn enumerate_segment_placements(
    context: &FleetContext,
    case: &ServiceChainCase,
    alternative: &VoyageAlternative,
    demand_index: usize,
    workload: &FleetWorkload,
    next_demand_index: usize,
    segment_index: usize,
    previous_end_tick: Option<u64>,
    placements_for_demand: &mut Vec<OperationPlacement>,
    state: &mut PartialState,
    states_explored: &mut u64,
    max_states: u64,
    budget_exhausted: &mut bool,
    completed_plans: &mut Vec<FleetPlan>,
) {
    if *budget_exhausted {
        return;
    }
    if segment_index == alternative.operation_timings.len() {
        let Some(last) = placements_for_demand.last() else {
            return;
        };
        if last.end_tick < case.demand.delivery_window.start_tick {
            return;
        }
        let selection = DemandSelection {
            demand_id: case.demand.demand_id.clone(),
            participant_id: case.demand.participant_id.clone(),
            alternative_id: alternative.alternative_id.clone(),
            delivery_tick: last.end_tick,
            service_amount: case.demand.requested_amount,
            service_unit: case.demand.service_unit,
        };
        state.selections.push(selection);
        state.selected_energy.add_alternative(alternative);

        enumerate_demands(
            context,
            workload,
            next_demand_index,
            state,
            states_explored,
            max_states,
            budget_exhausted,
            completed_plans,
        );

        state.selected_energy.propulsion_wh -= u128::from(alternative.energy.propulsion_wh);
        state.selected_energy.auxiliary_wh -= u128::from(alternative.energy.auxiliary_wh);
        state.selected_energy.shore_power_import_wh -=
            u128::from(alternative.energy.shore_power_import_wh);
        state.selected_energy.inland_transfer_wh -=
            u128::from(alternative.energy.inland_transfer_wh);
        state.selections.pop();
        return;
    }

    let timing = &alternative.operation_timings[segment_index];
    let Some(route_index) = case
        .route_segments
        .iter()
        .position(|segment| segment.segment_id == timing.segment_id)
    else {
        return;
    };

    let (mut start_tick, maximum_start_tick) = if segment_index == 0 {
        let Some(latest_arrival_tick) = alternative.arrival_window.end_tick.checked_sub(1) else {
            return;
        };
        let Some(start) = latest_arrival_tick.checked_sub(timing.duration_ticks) else {
            return;
        };
        if start < case.decision_tick {
            return;
        }
        (start, start)
    } else {
        let Some(earliest) = previous_end_tick else {
            return;
        };
        let Some(latest) = case
            .demand
            .delivery_window
            .end_tick
            .checked_sub(timing.duration_ticks)
            .and_then(|tick| tick.checked_sub(1))
        else {
            return;
        };
        if earliest > latest {
            return;
        }
        (earliest, latest)
    };

    loop {
        if *states_explored >= max_states {
            *budget_exhausted = true;
            return;
        }
        *states_explored += 1;

        if let Some(mut placement) = candidate_operation_fits(
            context,
            case,
            route_index,
            start_tick,
            timing.duration_ticks,
            &state.operations,
        ) {
            placement.alternative_id = alternative.alternative_id.clone();
            let Some(previous_end) = previous_end_tick else {
                placements_for_demand.push(placement.clone());
                state.operations.push(placement);
                enumerate_segment_placements(
                    context,
                    case,
                    alternative,
                    demand_index,
                    workload,
                    next_demand_index,
                    segment_index + 1,
                    Some(start_tick + timing.duration_ticks),
                    placements_for_demand,
                    state,
                    states_explored,
                    max_states,
                    budget_exhausted,
                    completed_plans,
                );
                state.operations.pop();
                placements_for_demand.pop();
                if *budget_exhausted {
                    return;
                }
                if start_tick == maximum_start_tick {
                    break;
                }
            } else {
                state.total_idle_wait_ticks += u128::from(start_tick - previous_end);
                placements_for_demand.push(placement.clone());
                state.operations.push(placement);
                enumerate_segment_placements(
                    context,
                    case,
                    alternative,
                    demand_index,
                    workload,
                    next_demand_index,
                    segment_index + 1,
                    Some(start_tick + timing.duration_ticks),
                    placements_for_demand,
                    state,
                    states_explored,
                    max_states,
                    budget_exhausted,
                    completed_plans,
                );
                state.operations.pop();
                placements_for_demand.pop();
                state.total_idle_wait_ticks -= u128::from(start_tick - previous_end);
                if *budget_exhausted {
                    return;
                }
                if start_tick == maximum_start_tick {
                    break;
                }
            }
        }
        if start_tick == maximum_start_tick {
            break;
        }
        let Some(next) = start_tick.checked_add(1) else {
            break;
        };
        start_tick = next;
    }
    let _ = demand_index;
}

fn enumerate_demands(
    context: &FleetContext,
    workload: &FleetWorkload,
    demand_index: usize,
    state: &mut PartialState,
    states_explored: &mut u64,
    max_states: u64,
    budget_exhausted: &mut bool,
    completed_plans: &mut Vec<FleetPlan>,
) {
    if *budget_exhausted {
        return;
    }
    if demand_index == workload.cases.len() {
        completed_plans.push(FleetPlan {
            selections: state.selections.clone(),
            operations: state.operations.clone(),
            objective: objective_for(state),
        });
        return;
    }
    let case_index = context.case_order[demand_index];
    let case = &workload.cases[case_index];
    let mut alternatives: Vec<&VoyageAlternative> = case.voyage_alternatives.iter().collect();
    alternatives.sort_by(|left, right| left.alternative_id.cmp(&right.alternative_id));
    for alternative in alternatives {
        enumerate_segment_placements(
            context,
            case,
            alternative,
            demand_index,
            workload,
            demand_index + 1,
            0,
            None,
            &mut Vec::new(),
            state,
            states_explored,
            max_states,
            budget_exhausted,
            completed_plans,
        );
        if *budget_exhausted {
            return;
        }
    }
}

/// Exhaustively schedule every demand over the shared resources in a tiny
/// validated corridor workload. Empty, complete frontier means joint infeasibility;
/// budget exhaustion is explicitly non-proven.
pub fn exact_fleet_oracle(
    workload: &FleetWorkload,
    max_states: u64,
) -> Result<FleetResult, FleetError> {
    if max_states == 0 {
        return Err(FleetError::InvalidSearchBudget);
    }
    let context = validate_workload(workload)?;
    let mut state = PartialState {
        operations: Vec::new(),
        selections: Vec::new(),
        total_idle_wait_ticks: 0,
        selected_energy: EnergyTotals::default(),
    };
    let mut states_explored = 0;
    let mut budget_exhausted = false;
    let mut plans = Vec::new();
    enumerate_demands(
        &context,
        workload,
        0,
        &mut state,
        &mut states_explored,
        max_states,
        &mut budget_exhausted,
        &mut plans,
    );
    Ok(FleetResult {
        frontier: pareto_frontier(plans),
        states_explored,
        termination: if budget_exhausted {
            SearchTermination::BudgetExhausted
        } else {
            SearchTermination::ExhaustiveComplete
        },
    })
}

// Testable accounting kernels are public so their invariants can be validated
// independently before constructing a large multi-demand scenario fixture.
pub fn check_concurrent_capacity(
    resource: &Resource,
    operations: &[OperationPlacement],
) -> bool {
    concurrent_capacity_fits(
        &ResourceSnapshot {
            resource: canonical_resource(resource),
            evidence: Vec::new(),
            accounting_mode: ResourceAccountingMode::ConcurrentCapacity,
        },
        operations,
    )
}

pub fn check_cumulative_quota(
    resource: &Resource,
    operations: &[OperationPlacement],
) -> bool {
    cumulative_quota_fits(
        &ResourceSnapshot {
            resource: canonical_resource(resource),
            evidence: Vec::new(),
            accounting_mode: ResourceAccountingMode::CumulativeWindowQuota,
        },
        operations,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn window(start_tick: u64, end_tick: u64, capacity: u64) -> CapacityWindow {
        CapacityWindow {
            window: TickWindow {
                start_tick,
                end_tick,
            },
            capacity,
        }
    }

    fn operation(
        demand_id: &str,
        start_tick: u64,
        end_tick: u64,
        amount: u64,
        unit: ResourceUnit,
    ) -> OperationPlacement {
        OperationPlacement {
            demand_id: demand_id.into(),
            participant_id: format!("participant-{demand_id}"),
            alternative_id: "alternative-1".into(),
            segment_id: "lock-segment".into(),
            start_tick,
            end_tick,
            resource_uses: vec![ResourceUse {
                resource_id: "lock-01".into(),
                amount,
                unit,
            }],
        }
    }

    fn resource(kind: ResourceKind, unit: ResourceUnit, capacity: u64) -> Resource {
        Resource {
            resource_id: "lock-01".into(),
            kind,
            capacity_unit: unit,
            availability: vec![window(0, 20, capacity)],
            evidence_ids: vec!["capacity-evidence".into()],
        }
    }

    #[test]
    fn concurrent_capacity_rejects_overlapping_lockages() {
        let resource = resource(ResourceKind::Lock, ResourceUnit::Lockages, 1);
        let operations = vec![
            operation("demand-a", 5, 10, 1, ResourceUnit::Lockages),
            operation("demand-b", 7, 12, 1, ResourceUnit::Lockages),
        ];
        assert!(!check_concurrent_capacity(&resource, &operations));
    }

    #[test]
    fn concurrent_capacity_allows_exact_half_open_handoff() {
        let resource = resource(ResourceKind::Lock, ResourceUnit::Lockages, 1);
        let operations = vec![
            operation("demand-a", 5, 10, 1, ResourceUnit::Lockages),
            operation("demand-b", 10, 15, 1, ResourceUnit::Lockages),
        ];
        assert!(check_concurrent_capacity(&resource, &operations));
    }

    #[test]
    fn concurrent_capacity_respects_capacity_greater_than_one() {
        let resource = resource(ResourceKind::Berth, ResourceUnit::BerthSlots, 2);
        let operations = vec![
            operation("demand-a", 5, 10, 1, ResourceUnit::BerthSlots),
            operation("demand-b", 7, 12, 1, ResourceUnit::BerthSlots),
        ];
        assert!(check_concurrent_capacity(&resource, &operations));
    }

    #[test]
    fn cumulative_quota_sums_non_overlapping_and_overlapping_operations() {
        let resource = resource(ResourceKind::WaterQuota, ResourceUnit::Millilitres, 10_000);
        let operations = vec![
            operation("demand-a", 5, 10, 6_000, ResourceUnit::Millilitres),
            operation("demand-b", 12, 16, 5_000, ResourceUnit::Millilitres),
        ];
        assert!(!check_cumulative_quota(&resource, &operations));
    }

    #[test]
    fn cumulative_quota_uses_each_declared_accounting_window() {
        let resource = Resource {
            resource_id: "lock-01".into(),
            kind: ResourceKind::WaterQuota,
            capacity_unit: ResourceUnit::Millilitres,
            availability: vec![window(0, 10, 5_000), window(10, 20, 5_000)],
            evidence_ids: vec!["capacity-evidence".into()],
        };
        let operations = vec![
            operation("demand-a", 1, 5, 4_000, ResourceUnit::Millilitres),
            operation("demand-b", 11, 15, 4_000, ResourceUnit::Millilitres),
        ];
        assert!(check_cumulative_quota(&resource, &operations));
    }

    #[test]
    fn cumulative_quota_requires_each_operation_inside_one_quota_window() {
        let resource = Resource {
            resource_id: "lock-01".into(),
            kind: ResourceKind::WaterQuota,
            capacity_unit: ResourceUnit::Millilitres,
            availability: vec![window(0, 10, 5_000), window(10, 20, 5_000)],
            evidence_ids: vec!["capacity-evidence".into()],
        };
        let operations = vec![operation("demand-a", 8, 12, 1_000, ResourceUnit::Millilitres)];
        assert!(!check_cumulative_quota(&resource, &operations));
    }

    #[test]
    fn concurrent_capacity_checks_each_non_overlapping_capacity_window() {
        let resource = Resource {
            resource_id: "lock-01".into(),
            kind: ResourceKind::Lock,
            capacity_unit: ResourceUnit::Lockages,
            availability: vec![window(0, 10, 1), window(10, 20, 2)],
            evidence_ids: vec!["capacity-evidence".into()],
        };
        let operations = vec![
            operation("demand-a", 10, 15, 1, ResourceUnit::Lockages),
            operation("demand-b", 11, 14, 1, ResourceUnit::Lockages),
            operation("demand-c", 12, 13, 1, ResourceUnit::Lockages),
        ];
        assert!(!check_concurrent_capacity(&resource, &operations));
    }
}
