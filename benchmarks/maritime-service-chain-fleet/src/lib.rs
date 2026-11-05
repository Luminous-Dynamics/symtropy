//! Exact tiny-instance multi-demand scheduler over shared corridor resources.
//!
//! The oracle is deliberately small and effects-disabled. It composes already
//! validated one-demand service-chain cases; it does not duplicate vessel,
//! navigability, lock-eligibility, hydrodynamic, port-call, or participant
//! authority validation.

use std::collections::{BTreeMap, BTreeSet};

use maritime_service_chain_contract::{
    EvidenceRef, Resource, ResourceKind, ResourceUnit, ResourceUse, ServiceChainCase, TickWindow,
    ValidationError, VoyageAlternative, validate_case,
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

struct SearchEngine<'a> {
    context: FleetContext,
    workload: &'a FleetWorkload,
    partial: PartialState,
    states_explored: u64,
    max_states: u64,
    budget_exhausted: bool,
    completed_plans: Vec<FleetPlan>,
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
            return Err(FleetError::DuplicateDemandId(case.demand.demand_id.clone()));
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

fn covering_window_index(resource: &Resource, interval: TickWindow) -> Option<usize> {
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

fn cumulative_quota_fits(snapshot: &ResourceSnapshot, operations: &[OperationPlacement]) -> bool {
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

fn shared_resources_fit(context: &FleetContext, operations: &[OperationPlacement]) -> bool {
    context
        .resources
        .values()
        .all(|snapshot| match snapshot.accounting_mode {
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
    let mut resource_uses = segment.resource_uses.clone();
    resource_uses.sort_by(|left, right| {
        left.resource_id
            .cmp(&right.resource_id)
            .then_with(|| left.amount.cmp(&right.amount))
    });
    let placement = OperationPlacement {
        demand_id: case.demand.demand_id.clone(),
        participant_id: case.demand.participant_id.clone(),
        alternative_id: String::new(),
        segment_id: segment.segment_id.clone(),
        start_tick,
        end_tick: interval.end_tick,
        resource_uses,
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
    engine: &mut SearchEngine<'_>,
    case: &ServiceChainCase,
    alternative: &VoyageAlternative,
    next_demand_index: usize,
    segment_index: usize,
    previous_end_tick: Option<u64>,
    placements_for_demand: &mut Vec<OperationPlacement>,
) {
    if engine.budget_exhausted {
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
        engine.partial.selections.push(selection);
        engine.partial.selected_energy.add_alternative(alternative);

        enumerate_demands(engine, next_demand_index);

        engine.partial.selected_energy.propulsion_wh -=
            u128::from(alternative.energy.propulsion_wh);
        engine.partial.selected_energy.auxiliary_wh -= u128::from(alternative.energy.auxiliary_wh);
        engine.partial.selected_energy.shore_power_import_wh -=
            u128::from(alternative.energy.shore_power_import_wh);
        engine.partial.selected_energy.inland_transfer_wh -=
            u128::from(alternative.energy.inland_transfer_wh);
        engine.partial.selections.pop();
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
        if engine.states_explored >= engine.max_states {
            engine.budget_exhausted = true;
            return;
        }
        engine.states_explored += 1;

        if let Some(mut placement) = candidate_operation_fits(
            &engine.context,
            case,
            route_index,
            start_tick,
            timing.duration_ticks,
            &engine.partial.operations,
        ) {
            placement.alternative_id = alternative.alternative_id.clone();
            if let Some(previous_end) = previous_end_tick {
                engine.partial.total_idle_wait_ticks += u128::from(start_tick - previous_end);
            }
            placements_for_demand.push(placement.clone());
            engine.partial.operations.push(placement);
            enumerate_segment_placements(
                engine,
                case,
                alternative,
                next_demand_index,
                segment_index + 1,
                Some(start_tick + timing.duration_ticks),
                placements_for_demand,
            );
            engine.partial.operations.pop();
            placements_for_demand.pop();
            if let Some(previous_end) = previous_end_tick {
                engine.partial.total_idle_wait_ticks -= u128::from(start_tick - previous_end);
            }
            if engine.budget_exhausted {
                return;
            }
            if start_tick == maximum_start_tick {
                break;
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
}

fn enumerate_demands(engine: &mut SearchEngine<'_>, demand_index: usize) {
    if engine.budget_exhausted {
        return;
    }
    if demand_index == engine.workload.cases.len() {
        engine.completed_plans.push(FleetPlan {
            selections: engine.partial.selections.clone(),
            operations: engine.partial.operations.clone(),
            objective: objective_for(&engine.partial),
        });
        return;
    }

    // Own the case/alternative fixtures for the recursive call so the mutable
    // engine can safely carry the budget, partial plan and frontier.
    let case_index = engine.context.case_order[demand_index];
    let case = engine.workload.cases[case_index].clone();
    let mut alternatives = case.voyage_alternatives.clone();
    alternatives.sort_by(|left, right| left.alternative_id.cmp(&right.alternative_id));

    for alternative in alternatives {
        enumerate_segment_placements(
            engine,
            &case,
            &alternative,
            demand_index + 1,
            0,
            None,
            &mut Vec::new(),
        );
        if engine.budget_exhausted {
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
    let mut engine = SearchEngine {
        context,
        workload,
        partial: PartialState {
            operations: Vec::new(),
            selections: Vec::new(),
            total_idle_wait_ticks: 0,
            selected_energy: EnergyTotals::default(),
        },
        states_explored: 0,
        max_states,
        budget_exhausted: false,
        completed_plans: Vec::new(),
    };
    enumerate_demands(&mut engine, 0);
    Ok(FleetResult {
        frontier: pareto_frontier(engine.completed_plans),
        states_explored: engine.states_explored,
        termination: if engine.budget_exhausted {
            SearchTermination::BudgetExhausted
        } else {
            SearchTermination::ExhaustiveComplete
        },
    })
}

// Testable accounting kernels are public so their invariants can be validated
// independently before constructing a large multi-demand scenario fixture.
pub fn check_concurrent_capacity(resource: &Resource, operations: &[OperationPlacement]) -> bool {
    concurrent_capacity_fits(
        &ResourceSnapshot {
            resource: canonical_resource(resource),
            evidence: Vec::new(),
            accounting_mode: ResourceAccountingMode::ConcurrentCapacity,
        },
        operations,
    )
}

pub fn check_cumulative_quota(resource: &Resource, operations: &[OperationPlacement]) -> bool {
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
    use maritime_service_chain_contract::CapacityWindow;

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

    fn evidence(
        id: &str,
        owner: maritime_service_chain_contract::EvidenceOwner,
        role: maritime_service_chain_contract::EvidenceRole,
        subject: &str,
        covered_window: Option<TickWindow>,
    ) -> EvidenceRef {
        EvidenceRef {
            evidence_id: id.into(),
            owner,
            role,
            source_id: format!("fixture:{id}"),
            source_revision: "fleet-fixture-v1".into(),
            snapshot_ref: format!("snapshot:{id}"),
            subject_id: subject.into(),
            observed_at_tick: 0,
            valid_from_tick: 0,
            valid_through_tick: Some(100),
            covered_window,
            covered_resource_unit: None,
            covered_capacity_windows: None,
            state: maritime_service_chain_contract::EvidenceState::Current,
            claim_ceiling: "synthetic fleet fixture only".into(),
        }
    }

    fn capacity_evidence(
        id: &str,
        subject: &str,
        unit: ResourceUnit,
        availability: Vec<CapacityWindow>,
    ) -> EvidenceRef {
        let mut item = evidence(
            id,
            maritime_service_chain_contract::EvidenceOwner::Symtropy,
            maritime_service_chain_contract::EvidenceRole::ResourceCapacity,
            subject,
            None,
        );
        item.covered_resource_unit = Some(unit);
        item.covered_capacity_windows = Some(availability);
        item
    }

    fn valid_fleet_case(
        case_id: &str,
        demand_id: &str,
        participant_id: &str,
        vessel_id: &str,
    ) -> ServiceChainCase {
        use maritime_service_chain_contract::{
            CaseClass, ConstraintDisposition, ConstraintKind, Demand, EnergyLedger, EvidenceOwner,
            EvidenceRole, HardConstraint, Node, NodeKind, RouteSegment, SegmentKind,
            SegmentOperationTiming, ServiceUnit,
        };

        let arrival_fast = TickWindow {
            start_tick: 10,
            end_tick: 13,
        };
        let arrival_slow = TickWindow {
            start_tick: 15,
            end_tick: 18,
        };
        let route_ids = vec![
            "channel-01".to_owned(),
            "lock-01".to_owned(),
            "approach-01".to_owned(),
            "turnaround-01".to_owned(),
            "rail-01".to_owned(),
        ];

        let evidence = vec![
            evidence(
                "topology",
                EvidenceOwner::SolAtlas,
                EvidenceRole::SpatialTopology,
                "corridor-01",
                None,
            ),
            evidence(
                "depth",
                EvidenceOwner::SolAtlas,
                EvidenceRole::Navigability,
                "channel-01",
                None,
            ),
            evidence(
                "approach-depth",
                EvidenceOwner::SolAtlas,
                EvidenceRole::Navigability,
                "approach-01",
                None,
            ),
            evidence(
                "data-rights",
                EvidenceOwner::External,
                EvidenceRole::DataRights,
                case_id,
                None,
            ),
            evidence(
                &format!("vessel-capability-{vessel_id}"),
                EvidenceOwner::Symthaea,
                EvidenceRole::VesselEngineering,
                vessel_id,
                None,
            ),
            evidence(
                &format!("vessel-performance-{vessel_id}"),
                EvidenceOwner::Symthaea,
                EvidenceRole::VesselPerformance,
                vessel_id,
                None,
            ),
            evidence(
                "arrival-fast",
                EvidenceOwner::Symthaea,
                EvidenceRole::ArrivalWindow,
                "lock-approach",
                Some(arrival_fast),
            ),
            evidence(
                "arrival-slow",
                EvidenceOwner::Symthaea,
                EvidenceRole::ArrivalWindow,
                "lock-approach",
                Some(arrival_slow),
            ),
            evidence(
                "lock-compatibility",
                EvidenceOwner::Symtropy,
                EvidenceRole::LockCompatibility,
                "lock-01",
                None,
            ),
            evidence(
                "lock-water",
                EvidenceOwner::Symtropy,
                EvidenceRole::LockCycleWater,
                "lock-01",
                None,
            ),
            capacity_evidence(
                "channel-calendar",
                "channel-window-01",
                ResourceUnit::TransitSlots,
                vec![
                    window(10, 12, 2),
                    window(14, 15, 2),
                    window(15, 17, 2),
                    window(19, 20, 2),
                ],
            ),
            capacity_evidence(
                "lock-calendar",
                "lock-01",
                ResourceUnit::Lockages,
                vec![window(12, 14, 1), window(17, 19, 1)],
            ),
            capacity_evidence(
                "water-calendar",
                "water-budget-01",
                ResourceUnit::Millilitres,
                vec![window(12, 40, 10_000)],
            ),
            capacity_evidence(
                "berth-calendar",
                "berth-01",
                ResourceUnit::BerthSlots,
                vec![window(18, 28, 1), window(28, 38, 1)],
            ),
            capacity_evidence(
                "shore-calendar",
                "shore-01",
                ResourceUnit::WattHours,
                vec![window(18, 40, 2_000)],
            ),
            capacity_evidence(
                "handling-calendar",
                "terminal-01",
                ResourceUnit::Tonnes,
                vec![window(18, 40, 20)],
            ),
            capacity_evidence(
                "rail-calendar",
                "rail-01",
                ResourceUnit::Tonnes,
                vec![window(28, 33, 10), window(38, 43, 10)],
            ),
            evidence(
                "environment",
                EvidenceOwner::External,
                EvidenceRole::EnvironmentalWindow,
                "corridor-01",
                None,
            ),
            evidence(
                "participant-auth",
                EvidenceOwner::Mycelix,
                EvidenceRole::ParticipantAuthorization,
                participant_id,
                None,
            ),
        ];

        let nodes = vec![
            Node {
                node_id: "origin".into(),
                kind: NodeKind::Origin,
            },
            Node {
                node_id: "lock-approach".into(),
                kind: NodeKind::LockApproach,
            },
            Node {
                node_id: "lock-exit".into(),
                kind: NodeKind::LockExit,
            },
            Node {
                node_id: "berth".into(),
                kind: NodeKind::Berth,
            },
            Node {
                node_id: "railhead".into(),
                kind: NodeKind::TransferTerminal,
            },
            Node {
                node_id: "destination".into(),
                kind: NodeKind::Destination,
            },
        ];

        let resources = vec![
            Resource {
                resource_id: "channel-window-01".into(),
                kind: ResourceKind::ChannelWindow,
                capacity_unit: ResourceUnit::TransitSlots,
                availability: vec![
                    window(10, 12, 2),
                    window(14, 15, 2),
                    window(15, 17, 2),
                    window(19, 20, 2),
                ],
                evidence_ids: vec!["channel-calendar".into()],
            },
            Resource {
                resource_id: "lock-01".into(),
                kind: ResourceKind::Lock,
                capacity_unit: ResourceUnit::Lockages,
                availability: vec![window(12, 14, 1), window(17, 19, 1)],
                evidence_ids: vec!["lock-calendar".into()],
            },
            Resource {
                resource_id: "water-budget-01".into(),
                kind: ResourceKind::WaterQuota,
                capacity_unit: ResourceUnit::Millilitres,
                availability: vec![window(12, 40, 10_000)],
                evidence_ids: vec!["water-calendar".into()],
            },
            Resource {
                resource_id: "berth-01".into(),
                kind: ResourceKind::Berth,
                capacity_unit: ResourceUnit::BerthSlots,
                availability: vec![window(18, 28, 1), window(28, 38, 1)],
                evidence_ids: vec!["berth-calendar".into()],
            },
            Resource {
                resource_id: "shore-01".into(),
                kind: ResourceKind::ShorePower,
                capacity_unit: ResourceUnit::WattHours,
                availability: vec![window(18, 40, 2_000)],
                evidence_ids: vec!["shore-calendar".into()],
            },
            Resource {
                resource_id: "terminal-01".into(),
                kind: ResourceKind::TerminalHandling,
                capacity_unit: ResourceUnit::Tonnes,
                availability: vec![window(18, 40, 20)],
                evidence_ids: vec!["handling-calendar".into()],
            },
            Resource {
                resource_id: "rail-01".into(),
                kind: ResourceKind::InlandTransfer,
                capacity_unit: ResourceUnit::Tonnes,
                availability: vec![window(28, 33, 10), window(38, 43, 10)],
                evidence_ids: vec!["rail-calendar".into()],
            },
        ];

        let route_segments = vec![
            RouteSegment {
                segment_id: "channel-01".into(),
                from_node_id: "origin".into(),
                to_node_id: "lock-approach".into(),
                kind: SegmentKind::ChannelTransit,
                evidence_ids: vec!["topology".into(), "depth".into(), "channel-calendar".into()],
                resource_uses: vec![ResourceUse {
                    resource_id: "channel-window-01".into(),
                    amount: 1,
                    unit: ResourceUnit::TransitSlots,
                }],
            },
            RouteSegment {
                segment_id: "lock-01".into(),
                from_node_id: "lock-approach".into(),
                to_node_id: "lock-exit".into(),
                kind: SegmentKind::Lockage,
                evidence_ids: vec![
                    "lock-compatibility".into(),
                    "lock-water".into(),
                    "lock-calendar".into(),
                    "water-calendar".into(),
                ],
                resource_uses: vec![
                    ResourceUse {
                        resource_id: "lock-01".into(),
                        amount: 1,
                        unit: ResourceUnit::Lockages,
                    },
                    ResourceUse {
                        resource_id: "water-budget-01".into(),
                        amount: 4_000,
                        unit: ResourceUnit::Millilitres,
                    },
                ],
            },
            RouteSegment {
                segment_id: "approach-01".into(),
                from_node_id: "lock-exit".into(),
                to_node_id: "berth".into(),
                kind: SegmentKind::PortApproach,
                evidence_ids: vec!["approach-depth".into(), "channel-calendar".into()],
                resource_uses: vec![ResourceUse {
                    resource_id: "channel-window-01".into(),
                    amount: 1,
                    unit: ResourceUnit::TransitSlots,
                }],
            },
            RouteSegment {
                segment_id: "turnaround-01".into(),
                from_node_id: "berth".into(),
                to_node_id: "railhead".into(),
                kind: SegmentKind::BerthHandling,
                evidence_ids: vec![
                    "berth-calendar".into(),
                    "handling-calendar".into(),
                    "shore-calendar".into(),
                ],
                resource_uses: vec![
                    ResourceUse {
                        resource_id: "berth-01".into(),
                        amount: 1,
                        unit: ResourceUnit::BerthSlots,
                    },
                    ResourceUse {
                        resource_id: "terminal-01".into(),
                        amount: 10,
                        unit: ResourceUnit::Tonnes,
                    },
                    ResourceUse {
                        resource_id: "shore-01".into(),
                        amount: 800,
                        unit: ResourceUnit::WattHours,
                    },
                ],
            },
            RouteSegment {
                segment_id: "rail-01".into(),
                from_node_id: "railhead".into(),
                to_node_id: "destination".into(),
                kind: SegmentKind::InlandTransfer,
                evidence_ids: vec!["rail-calendar".into()],
                resource_uses: vec![ResourceUse {
                    resource_id: "rail-01".into(),
                    amount: 10,
                    unit: ResourceUnit::Tonnes,
                }],
            },
        ];

        let timings = vec![
            SegmentOperationTiming {
                segment_id: "channel-01".into(),
                duration_ticks: 2,
            },
            SegmentOperationTiming {
                segment_id: "lock-01".into(),
                duration_ticks: 2,
            },
            SegmentOperationTiming {
                segment_id: "approach-01".into(),
                duration_ticks: 1,
            },
            SegmentOperationTiming {
                segment_id: "turnaround-01".into(),
                duration_ticks: 10,
            },
            SegmentOperationTiming {
                segment_id: "rail-01".into(),
                duration_ticks: 5,
            },
        ];
        let alternatives = vec![
            VoyageAlternative {
                alternative_id: "fast-high-energy".into(),
                vessel_id: vessel_id.into(),
                vessel_revision: "vessel-r1".into(),
                speed_profile_id: "speed-fast".into(),
                cargo_capacity_amount: 12,
                cargo_capacity_unit: ServiceUnit::Tonnes,
                arrival_node_id: "lock-approach".into(),
                arrival_window: arrival_fast,
                arrival_evidence_id: "arrival-fast".into(),
                evidence_ids: vec![
                    format!("vessel-capability-{vessel_id}"),
                    format!("vessel-performance-{vessel_id}"),
                ],
                route_segment_ids: route_ids.clone(),
                operation_timings: timings.clone(),
                energy: EnergyLedger {
                    propulsion_wh: 1_800,
                    auxiliary_wh: 200,
                    shore_power_import_wh: 800,
                    inland_transfer_wh: 0,
                },
            },
            VoyageAlternative {
                alternative_id: "slow-low-energy".into(),
                vessel_id: vessel_id.into(),
                vessel_revision: "vessel-r1".into(),
                speed_profile_id: "speed-slow".into(),
                cargo_capacity_amount: 12,
                cargo_capacity_unit: ServiceUnit::Tonnes,
                arrival_node_id: "lock-approach".into(),
                arrival_window: arrival_slow,
                arrival_evidence_id: "arrival-slow".into(),
                evidence_ids: vec![
                    format!("vessel-capability-{vessel_id}"),
                    format!("vessel-performance-{vessel_id}"),
                ],
                route_segment_ids: route_ids,
                operation_timings: timings,
                energy: EnergyLedger {
                    propulsion_wh: 1_300,
                    auxiliary_wh: 200,
                    shore_power_import_wh: 800,
                    inland_transfer_wh: 0,
                },
            },
        ];

        ServiceChainCase {
            schema_version: "0.1.0".into(),
            case_id: case_id.into(),
            name: format!("Fleet fixture {case_id}"),
            class: CaseClass::Synthetic,
            decision_tick: 0,
            maximum_evidence_age_ticks: 20,
            tick_seconds: 60,
            corridor_id: "corridor-01".into(),
            origin_node_id: "origin".into(),
            destination_node_id: "destination".into(),
            demand: Demand {
                demand_id: demand_id.into(),
                participant_id: participant_id.into(),
                origin_node_id: "origin".into(),
                destination_node_id: "destination".into(),
                requested_amount: 10,
                service_unit: ServiceUnit::Tonnes,
                delivery_window: TickWindow {
                    start_tick: 0,
                    end_tick: 80,
                },
                participant_commitment_ref: Some(format!("mycelix:commitment:{demand_id}")),
                cargo_custody_ref: None,
            },
            nodes,
            route_segments,
            resources,
            voyage_alternatives: alternatives,
            evidence,
            hard_constraints: vec![
                HardConstraint {
                    constraint_id: "constraint-navigability".into(),
                    kind: ConstraintKind::Navigability,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("depth".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-lock".into(),
                    kind: ConstraintKind::LockEligibility,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("lock-compatibility".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-environment".into(),
                    kind: ConstraintKind::EnvironmentalFlow,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("environment".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-water".into(),
                    kind: ConstraintKind::WaterBudget,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("lock-water".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-energy".into(),
                    kind: ConstraintKind::EnergyCapacity,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("shore-calendar".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-berth".into(),
                    kind: ConstraintKind::BerthCapacity,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("berth-calendar".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-data-rights".into(),
                    kind: ConstraintKind::DataRights,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("data-rights".into()),
                },
                HardConstraint {
                    constraint_id: "constraint-participant".into(),
                    kind: ConstraintKind::ParticipantAuthorization,
                    disposition: ConstraintDisposition::Satisfied,
                    evidence_id: Some("participant-auth".into()),
                },
            ],
            port_call_projection_ref: Some(format!("pco-fixture:{case_id}")),
        }
    }

    fn fleet_workload(cases: Vec<ServiceChainCase>) -> FleetWorkload {
        let policies = cases[0]
            .resources
            .iter()
            .map(|resource| SharedResourcePolicy {
                resource_id: resource.resource_id.clone(),
                accounting_mode: match resource.kind {
                    ResourceKind::WaterQuota | ResourceKind::ShorePower => {
                        ResourceAccountingMode::CumulativeWindowQuota
                    }
                    _ => ResourceAccountingMode::ConcurrentCapacity,
                },
            })
            .collect();
        FleetWorkload {
            time_basis_id: "corridor-01-utc-minute-ticks-v1".into(),
            cases,
            resource_policies: policies,
        }
    }

    #[test]
    fn multi_demand_oracle_schedules_two_vessels_when_lock_slots_are_sequential() {
        let mut first = valid_fleet_case("case-a", "demand-a", "participant-a", "vessel-a");
        first
            .voyage_alternatives
            .retain(|alternative| alternative.alternative_id == "fast-high-energy");
        let mut second = valid_fleet_case("case-b", "demand-b", "participant-b", "vessel-b");
        second
            .voyage_alternatives
            .retain(|alternative| alternative.alternative_id == "slow-low-energy");
        let workload = fleet_workload(vec![first, second]);

        let result = exact_fleet_oracle(&workload, 50_000).expect("valid joint workload");
        assert_eq!(result.termination, SearchTermination::ExhaustiveComplete);
        assert!(!result.frontier.is_empty());
        for plan in &result.frontier {
            assert_eq!(plan.selections.len(), 2);
            assert_eq!(
                plan.selections
                    .iter()
                    .map(|selection| selection.demand_id.as_str())
                    .collect::<Vec<_>>(),
                vec!["demand-a", "demand-b"]
            );
            assert!(
                plan.operations
                    .iter()
                    .any(|operation| operation.demand_id == "demand-a")
            );
            assert!(
                plan.operations
                    .iter()
                    .any(|operation| operation.demand_id == "demand-b")
            );
        }
    }

    #[test]
    fn multi_demand_oracle_rejects_joint_lockage_conflict_before_deadline() {
        let mut first = valid_fleet_case("case-a", "demand-a", "participant-a", "vessel-a");
        first
            .voyage_alternatives
            .retain(|alternative| alternative.alternative_id == "fast-high-energy");
        let mut second = valid_fleet_case("case-b", "demand-b", "participant-b", "vessel-b");
        second
            .voyage_alternatives
            .retain(|alternative| alternative.alternative_id == "fast-high-energy");
        first.demand.delivery_window.end_tick = 43;
        second.demand.delivery_window.end_tick = 43;
        let workload = fleet_workload(vec![first, second]);

        let result = exact_fleet_oracle(&workload, 50_000).expect("valid joint workload");
        assert_eq!(result.termination, SearchTermination::ExhaustiveComplete);
        assert!(result.frontier.is_empty());
    }

    #[test]
    fn multi_demand_oracle_is_input_permutation_independent() {
        let mut first = valid_fleet_case("case-a", "demand-a", "participant-a", "vessel-a");
        first
            .voyage_alternatives
            .retain(|alternative| alternative.alternative_id == "fast-high-energy");
        let mut second = valid_fleet_case("case-b", "demand-b", "participant-b", "vessel-b");
        second
            .voyage_alternatives
            .retain(|alternative| alternative.alternative_id == "slow-low-energy");
        let forward =
            exact_fleet_oracle(&fleet_workload(vec![first.clone(), second.clone()]), 50_000)
                .expect("valid joint workload");
        let reverse = exact_fleet_oracle(&fleet_workload(vec![second, first]), 50_000)
            .expect("valid joint workload");
        assert_eq!(forward, reverse);
    }

    #[test]
    fn multi_demand_oracle_rejects_conflicting_resource_snapshots() {
        let first = valid_fleet_case("case-a", "demand-a", "participant-a", "vessel-a");
        let mut second = valid_fleet_case("case-b", "demand-b", "participant-b", "vessel-b");
        second
            .resources
            .iter_mut()
            .find(|resource| resource.resource_id == "lock-01")
            .unwrap()
            .availability[0]
            .capacity = 2;
        second
            .evidence
            .iter_mut()
            .find(|item| item.evidence_id == "lock-calendar")
            .unwrap()
            .covered_capacity_windows
            .as_mut()
            .unwrap()[0]
            .capacity = 2;
        let error = exact_fleet_oracle(&fleet_workload(vec![first, second]), 50_000)
            .expect_err("resource snapshots must be identical");
        assert_eq!(
            error,
            FleetError::ConflictingResourceSnapshot("lock-01".into())
        );
    }

    #[test]
    fn multi_demand_oracle_rejects_semantically_wrong_accounting_mode() {
        let first = valid_fleet_case("case-a", "demand-a", "participant-a", "vessel-a");
        let mut workload = fleet_workload(vec![
            first.clone(),
            valid_fleet_case("case-b", "demand-b", "participant-b", "vessel-b"),
        ]);
        workload
            .resource_policies
            .iter_mut()
            .find(|policy| policy.resource_id == "water-budget-01")
            .unwrap()
            .accounting_mode = ResourceAccountingMode::ConcurrentCapacity;
        let error =
            exact_fleet_oracle(&workload, 50_000).expect_err("water must be a cumulative quota");
        assert_eq!(
            error,
            FleetError::ResourceAccountingModeMismatch("water-budget-01".into())
        );
    }

    #[test]
    fn multi_demand_oracle_reports_budget_exhaustion_as_unproven() {
        let first = valid_fleet_case("case-a", "demand-a", "participant-a", "vessel-a");
        let second = valid_fleet_case("case-b", "demand-b", "participant-b", "vessel-b");
        let result = exact_fleet_oracle(&fleet_workload(vec![first, second]), 1)
            .expect("valid joint workload");
        assert_eq!(result.termination, SearchTermination::BudgetExhausted);
        assert_eq!(result.states_explored, 1);
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
        let operations = vec![operation(
            "demand-a",
            8,
            12,
            1_000,
            ResourceUnit::Millilitres,
        )];
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
