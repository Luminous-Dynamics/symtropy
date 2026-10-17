// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic, headless settlement dependency and resource-allocation contracts.
//!
//! This is a synthetic contract kernel, not a physical city or habitat model.
//! Resource quantities are integer, resource-kind-specific synthetic units. A
//! dependency edge means "operational prerequisite", not measured physical flow.
//! Unknown evidence is never silently converted into a healthy/zero-loss value.

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::{error::Error, fmt};

pub mod timeline;
pub mod observation;
pub mod policy;
pub mod policy_trace;
pub mod verifier;

pub const SETTLEMENT_SCHEMA_VERSION: u32 = 1;
pub const CLAIM_CEILING: &str =
    "synthetic_dependency_and_resource_allocation_only_not_physical_model";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SettlementError(pub String);

impl fmt::Display for SettlementError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        self.0.fmt(formatter)
    }
}

impl Error for SettlementError {}

fn valid_identifier(value: &str, field: &str) -> Result<(), SettlementError> {
    let valid = !value.is_empty()
        && value.len() <= 96
        && value.bytes().all(|byte| {
            byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'-' | b'_' | b':')
        });
    if valid {
        Ok(())
    } else {
        Err(SettlementError(format!("invalid {field}: {value:?}")))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ComponentId(String);

impl ComponentId {
    pub fn parse(value: impl Into<String>) -> Result<Self, SettlementError> {
        let value = value.into();
        valid_identifier(&value, "component ID")?;
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for ComponentId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DependencyMode {
    /// Every declared upstream dependency must be operational.
    AllOf,
    /// At least one declared upstream dependency must be operational.
    AnyOf,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum UnknownCause {
    StaleTelemetry,
    MissingTelemetry,
    ConflictingTelemetry,
    UnvalidatedModel,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "state", rename_all = "snake_case", deny_unknown_fields)]
pub enum ComponentState {
    Available,
    Failed,
    Unknown { cause: UnknownCause },
}

impl ComponentState {
    fn is_available(&self) -> bool {
        matches!(self, Self::Available)
    }

    fn is_failed(&self) -> bool {
        matches!(self, Self::Failed)
    }

    fn unknown_cause(&self) -> Option<UnknownCause> {
        match self {
            Self::Unknown { cause } => Some(cause.clone()),
            Self::Available | Self::Failed => None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Component {
    pub id: ComponentId,
    pub generation: u64,
    /// Unit of this component's own output, not a unit shared by its dependencies.
    pub output_unit: String,
    pub nominal_output_units: u64,
    pub input_mode: DependencyMode,
    pub observed_state: ComponentState,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Dependency {
    pub upstream_component: ComponentId,
    pub upstream_generation: u64,
    pub downstream_component: ComponentId,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ResourceKind {
    ElectricalEnergy,
    PotableWater,
    CoolingCapacity,
    MedicalSupplies,
    RepairParts,
}

impl ResourceKind {
    /// Explicitly synthetic units. These are not kWh, litres, kW-thermal, or
    /// any other calibrated engineering unit.
    pub const fn synthetic_unit_code(self) -> &'static str {
        match self {
            Self::ElectricalEnergy => "synthetic.electrical-energy-units",
            Self::PotableWater => "synthetic.potable-water-units",
            Self::CoolingCapacity => "synthetic.cooling-capacity-units",
            Self::MedicalSupplies => "synthetic.medical-supply-units",
            Self::RepairParts => "synthetic.repair-part-units",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceSupply {
    pub supply_id: String,
    /// Explicitly scopes which demands may compete for this supply.
    pub allocation_pool_id: String,
    /// Higher values are preferred before demand allocation; ties use supply ID.
    pub allocation_priority: u16,
    pub source_component: ComponentId,
    pub resource_kind: ResourceKind,
    pub capacity_units: u64,
    /// Protected reserve is not available for routine allocation.
    pub protected_reserve_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceDemand {
    pub demand_id: String,
    /// Must match a declared allocation pool; resource pools never merge implicitly.
    pub allocation_pool_id: String,
    pub consumer_component: ComponentId,
    pub resource_kind: ResourceKind,
    pub requested_units: u64,
    pub minimum_units: u64,
    /// Higher values are allocated first. Ties are broken by demand ID.
    pub priority: u16,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceFloor {
    pub floor_id: String,
    pub component_id: ComponentId,
    pub output_unit: String,
    pub minimum_output_units: u64,
    /// Resource demands that must also meet their minimum for this floor to hold.
    pub required_demand_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ScenarioInput {
    pub schema_version: u32,
    pub scenario_id: String,
    pub components: Vec<Component>,
    pub dependencies: Vec<Dependency>,
    pub resource_supplies: Vec<ResourceSupply>,
    pub resource_demands: Vec<ResourceDemand>,
    pub service_floors: Vec<ServiceFloor>,
}

impl ScenarioInput {
    /// Returns a canonically ordered copy. Duplicate records are retained so
    /// validation can reject them; this is not a deduplication step.
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.components.sort_by(|a, b| a.id.cmp(&b.id));
        canonical.dependencies.sort();
        canonical
            .resource_supplies
            .sort_by(|a, b| a.supply_id.cmp(&b.supply_id));
        canonical
            .resource_demands
            .sort_by(|a, b| a.demand_id.cmp(&b.demand_id));
        canonical
            .service_floors
            .sort_by(|a, b| a.floor_id.cmp(&b.floor_id));
        for floor in &mut canonical.service_floors {
            floor.required_demand_ids.sort();
        }
        canonical
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DemandOutcome {
    FullySatisfied,
    MinimumSatisfied,
    Shortfall,
    /// Known supply is insufficient; unknown supply might change the result.
    Indeterminate,
    ConsumerUnavailable,
    ConsumerStateUnknown,
    /// Another consumer in this pool has unknown state, so a shared inventory
    /// cannot be safely assigned to known consumers without more evidence.
    PoolBlockedByUnknownConsumer,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SourceAllocation {
    pub supply_id: String,
    pub units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceAllocation {
    pub demand_id: String,
    pub consumer_component: ComponentId,
    pub resource_kind: ResourceKind,
    pub unit_code: String,
    pub requested_units: u64,
    pub minimum_units: u64,
    pub allocated_known_units: u64,
    /// Remaining pool-wide unknown-capacity upper bound before this demand's
    /// priority decision. This is not a resource allocation or an availability claim.
    pub unresolved_potential_units: u64,
    pub sources: Vec<SourceAllocation>,
    pub outcome: DemandOutcome,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum FloorStatus {
    Met,
    Breached,
    Blocked,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ComponentResult {
    pub component_id: ComponentId,
    pub generation: u64,
    pub output_unit: String,
    pub effective_state: ComponentState,
    /// None means output is not known. Unknown must not be serialized as zero.
    pub effective_output_units: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ServiceFloorResult {
    pub floor_id: String,
    pub component_id: ComponentId,
    pub unit_code: String,
    pub minimum_output_units: u64,
    pub effective_output_units: Option<u64>,
    pub required_demand_ids: Vec<String>,
    pub status: FloorStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementReportPayload {
    pub schema_version: u32,
    pub scenario_id: String,
    pub claim_ceiling: String,
    pub canonical_input_sha256: String,
    pub component_results: Vec<ComponentResult>,
    pub resource_allocations: Vec<ResourceAllocation>,
    pub service_floors: Vec<ServiceFloorResult>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementReport {
    pub payload: SettlementReportPayload,
    /// SHA-256 over compact JSON serialization of payload; integrity linkage,
    /// not authentication, source truth, community authorization, or safety.
    pub payload_sha256: String,
}

pub fn evaluate_scenario(input: &ScenarioInput) -> Result<SettlementReport, SettlementError> {
    let canonical = input.canonicalized();
    let components_by_id = validate_input(&canonical)?;
    let topo = topological_order(&canonical)?;
    let input_bytes = serde_json::to_vec(&canonical)
        .map_err(|error| SettlementError(format!("serialize canonical input: {error}")))?;
    let canonical_input_sha256 = sha256_hex(&input_bytes);

    let mut effective_states = BTreeMap::new();
    for component_id in topo {
        let component = components_by_id
            .get(&component_id)
            .copied()
            .ok_or_else(|| SettlementError("topology returned unknown component".into()))?;
        let upstream_states: Vec<ComponentState> = canonical
            .dependencies
            .iter()
            .filter(|edge| edge.downstream_component == component_id)
            .map(|edge| {
                effective_states
                    .get(&edge.upstream_component)
                    .cloned()
                    .ok_or_else(|| SettlementError("dependency was not evaluated first".into()))
            })
            .collect::<Result<_, _>>()?;
        let state = propagate_state(
            &component.observed_state,
            component.input_mode,
            &upstream_states,
        );
        effective_states.insert(component_id, state);
    }

    let mut component_results = Vec::with_capacity(canonical.components.len());
    for component in &canonical.components {
        let state = effective_states
            .get(&component.id)
            .cloned()
            .ok_or_else(|| SettlementError("component state missing after evaluation".into()))?;
        let effective_output_units = match state {
            ComponentState::Available => Some(component.nominal_output_units),
            ComponentState::Failed => Some(0),
            ComponentState::Unknown { .. } => None,
        };
        component_results.push(ComponentResult {
            component_id: component.id.clone(),
            generation: component.generation,
            output_unit: component.output_unit.clone(),
            effective_state: state,
            effective_output_units,
        });
    }

    let resource_allocations = allocate_resources(&canonical, &effective_states)?;
    let allocations_by_id: BTreeMap<String, &ResourceAllocation> = resource_allocations
        .iter()
        .map(|allocation| (allocation.demand_id.clone(), allocation))
        .collect();

    let mut service_floors = Vec::with_capacity(canonical.service_floors.len());
    for floor in &canonical.service_floors {
        let component = components_by_id
            .get(&floor.component_id)
            .copied()
            .ok_or_else(|| SettlementError("service floor component missing".into()))?;
        let state = effective_states
            .get(&floor.component_id)
            .ok_or_else(|| SettlementError("service floor state missing".into()))?;
        let output = match state {
            ComponentState::Available => Some(component.nominal_output_units),
            ComponentState::Failed => Some(0),
            ComponentState::Unknown { .. } => None,
        };
        let required_outcomes: Vec<&DemandOutcome> = floor
            .required_demand_ids
            .iter()
            .filter_map(|demand_id| allocations_by_id.get(demand_id).map(|x| &x.outcome))
            .collect();

        let proven_breach = matches!(state, ComponentState::Failed)
            || output.is_some_and(|units| units < floor.minimum_output_units)
            || required_outcomes.iter().any(|outcome| {
                matches!(
                    outcome,
                    DemandOutcome::Shortfall | DemandOutcome::ConsumerUnavailable
                )
            });
        let indeterminate = matches!(state, ComponentState::Unknown { .. })
            || required_outcomes.iter().any(|outcome| {
                matches!(
                    outcome,
                    DemandOutcome::Indeterminate
                        | DemandOutcome::ConsumerStateUnknown
                        | DemandOutcome::PoolBlockedByUnknownConsumer
                )
            });
        let status = if proven_breach {
            FloorStatus::Breached
        } else if indeterminate {
            FloorStatus::Blocked
        } else {
            FloorStatus::Met
        };

        service_floors.push(ServiceFloorResult {
            floor_id: floor.floor_id.clone(),
            component_id: floor.component_id.clone(),
            unit_code: floor.output_unit.clone(),
            minimum_output_units: floor.minimum_output_units,
            effective_output_units: output,
            required_demand_ids: floor.required_demand_ids.clone(),
            status,
        });
    }

    let payload = SettlementReportPayload {
        schema_version: SETTLEMENT_SCHEMA_VERSION,
        scenario_id: canonical.scenario_id,
        claim_ceiling: CLAIM_CEILING.into(),
        canonical_input_sha256,
        component_results,
        resource_allocations,
        service_floors,
    };
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|error| SettlementError(format!("serialize settlement report: {error}")))?;
    let payload_sha256 = sha256_hex(&payload_bytes);
    Ok(SettlementReport {
        payload,
        payload_sha256,
    })
}

fn propagate_state(
    local: &ComponentState,
    mode: DependencyMode,
    upstream: &[ComponentState],
) -> ComponentState {
    if matches!(local, ComponentState::Failed) {
        return ComponentState::Failed;
    }
    if upstream.is_empty() {
        return local.clone();
    }

    match mode {
        DependencyMode::AllOf => {
            if upstream.iter().any(ComponentState::is_failed) {
                return ComponentState::Failed;
            }
            if let Some(cause) = local.unknown_cause() {
                return ComponentState::Unknown { cause };
            }
            if let Some(cause) = upstream.iter().find_map(ComponentState::unknown_cause) {
                return ComponentState::Unknown { cause };
            }
            ComponentState::Available
        }
        DependencyMode::AnyOf => {
            if upstream.iter().all(ComponentState::is_failed) {
                return ComponentState::Failed;
            }
            if let Some(cause) = local.unknown_cause() {
                return ComponentState::Unknown { cause };
            }
            if upstream.iter().any(ComponentState::is_available) {
                return ComponentState::Available;
            }
            if let Some(cause) = upstream.iter().find_map(ComponentState::unknown_cause) {
                return ComponentState::Unknown { cause };
            }
            ComponentState::Failed
        }
    }
}

fn validate_input(
    input: &ScenarioInput,
) -> Result<BTreeMap<ComponentId, &Component>, SettlementError> {
    if input.schema_version != SETTLEMENT_SCHEMA_VERSION {
        return Err(SettlementError(format!(
            "unsupported settlement schema version {}",
            input.schema_version
        )));
    }
    valid_identifier(&input.scenario_id, "scenario ID")?;

    let mut components = BTreeMap::new();
    for component in &input.components {
        valid_identifier(component.id.as_str(), "component ID")?;
        valid_identifier(&component.output_unit, "component output unit")?;
        if component.generation == 0 {
            return Err(SettlementError(format!(
                "component {} has generation zero",
                component.id
            )));
        }
        if components.insert(component.id.clone(), component).is_some() {
            return Err(SettlementError(format!(
                "duplicate component ID {}",
                component.id
            )));
        }
    }
    if components.is_empty() {
        return Err(SettlementError("scenario has no components".into()));
    }

    let mut edges = BTreeSet::new();
    for edge in &input.dependencies {
        let upstream = components
            .get(&edge.upstream_component)
            .copied()
            .ok_or_else(|| {
                SettlementError(format!(
                    "dependency source {} is missing",
                    edge.upstream_component
                ))
            })?;
        if !components.contains_key(&edge.downstream_component) {
            return Err(SettlementError(format!(
                "dependency target {} is missing",
                edge.downstream_component
            )));
        }
        if edge.upstream_component == edge.downstream_component {
            return Err(SettlementError(format!(
                "self-dependency on {}",
                edge.upstream_component
            )));
        }
        if upstream.generation != edge.upstream_generation {
            return Err(SettlementError(format!(
                "dependency generation mismatch for {}: expected {}, input generation {}",
                edge.upstream_component, upstream.generation, edge.upstream_generation
            )));
        }
        if !edges.insert((
            edge.upstream_component.clone(),
            edge.downstream_component.clone(),
        )) {
            return Err(SettlementError(format!(
                "duplicate dependency {} -> {}",
                edge.upstream_component, edge.downstream_component
            )));
        }
    }

    let mut supply_ids = BTreeSet::new();
    for supply in &input.resource_supplies {
        valid_identifier(&supply.supply_id, "supply ID")?;
        if !components.contains_key(&supply.source_component) {
            return Err(SettlementError(format!(
                "supply {} references missing source {}",
                supply.supply_id, supply.source_component
            )));
        }
        valid_identifier(&supply.allocation_pool_id, "resource allocation pool ID")?;
        if !supply_ids.insert(supply.supply_id.clone()) {
            return Err(SettlementError(format!(
                "duplicate supply ID {}",
                supply.supply_id
            )));
        }
        if supply.protected_reserve_units > supply.capacity_units {
            return Err(SettlementError(format!(
                "protected reserve exceeds capacity for {}",
                supply.supply_id
            )));
        }
    }

    let mut demand_ids = BTreeSet::new();
    for demand in &input.resource_demands {
        valid_identifier(&demand.demand_id, "demand ID")?;
        valid_identifier(&demand.allocation_pool_id, "resource allocation pool ID")?;
        if !components.contains_key(&demand.consumer_component) {
            return Err(SettlementError(format!(
                "demand {} references missing consumer {}",
                demand.demand_id, demand.consumer_component
            )));
        }
        if !demand_ids.insert(demand.demand_id.clone()) {
            return Err(SettlementError(format!(
                "duplicate demand ID {}",
                demand.demand_id
            )));
        }
        if demand.requested_units == 0
            || demand.minimum_units == 0
            || demand.minimum_units > demand.requested_units
        {
            return Err(SettlementError(format!(
                "invalid requested/minimum units for demand {}",
                demand.demand_id
            )));
        }
    }

    let mut floor_ids = BTreeSet::new();
    for floor in &input.service_floors {
        valid_identifier(&floor.floor_id, "service floor ID")?;
        valid_identifier(&floor.output_unit, "service floor output unit")?;
        if !floor_ids.insert(floor.floor_id.clone()) {
            return Err(SettlementError(format!(
                "duplicate service floor ID {}",
                floor.floor_id
            )));
        }
        let component = components.get(&floor.component_id).copied().ok_or_else(|| {
            SettlementError(format!(
                "service floor {} references missing component {}",
                floor.floor_id, floor.component_id
            ))
        })?;
        if component.output_unit != floor.output_unit {
            return Err(SettlementError(format!(
                "unit mismatch at floor {}: component uses {}, floor declares {}",
                floor.floor_id, component.output_unit, floor.output_unit
            )));
        }
        if floor.minimum_output_units == 0 {
            return Err(SettlementError(format!(
                "service floor {} has a zero minimum",
                floor.floor_id
            )));
        }
        let mut required_ids = BTreeSet::new();
        for demand_id in &floor.required_demand_ids {
            if !demand_ids.contains(demand_id) {
                return Err(SettlementError(format!(
                    "floor {} references missing demand {}",
                    floor.floor_id, demand_id
                )));
            }
            if !required_ids.insert(demand_id) {
                return Err(SettlementError(format!(
                    "floor {} repeats demand {}",
                    floor.floor_id, demand_id
                )));
            }
        }
    }
    Ok(components)
}

fn topological_order(input: &ScenarioInput) -> Result<Vec<ComponentId>, SettlementError> {
    let mut indegree: BTreeMap<ComponentId, usize> = input
        .components
        .iter()
        .map(|component| (component.id.clone(), 0))
        .collect();
    let mut outgoing: BTreeMap<ComponentId, BTreeSet<ComponentId>> = input
        .components
        .iter()
        .map(|component| (component.id.clone(), BTreeSet::new()))
        .collect();

    for edge in &input.dependencies {
        *indegree
            .get_mut(&edge.downstream_component)
            .ok_or_else(|| SettlementError("dependency target missing during sort".into()))? += 1;
        outgoing
            .get_mut(&edge.upstream_component)
            .ok_or_else(|| SettlementError("dependency source missing during sort".into()))?
            .insert(edge.downstream_component.clone());
    }

    let mut ready: BTreeSet<ComponentId> = indegree
        .iter()
        .filter(|(_, degree)| **degree == 0)
        .map(|(component_id, _)| component_id.clone())
        .collect();
    let mut order = Vec::with_capacity(input.components.len());

    while let Some(component_id) = ready.pop_first() {
        order.push(component_id.clone());
        if let Some(targets) = outgoing.get(&component_id) {
            for target in targets {
                let degree = indegree
                    .get_mut(target)
                    .ok_or_else(|| SettlementError("topology target missing".into()))?;
                *degree -= 1;
                if *degree == 0 {
                    ready.insert(target.clone());
                }
            }
        }
    }

    if order.len() != input.components.len() {
        return Err(SettlementError(
            "dependency cycle detected; v1 requires an acyclic graph".into(),
        ));
    }
    Ok(order)
}

fn allocate_resources(
    input: &ScenarioInput,
    effective_states: &BTreeMap<ComponentId, ComponentState>,
) -> Result<Vec<ResourceAllocation>, SettlementError> {
    let mut outcomes = Vec::with_capacity(input.resource_demands.len());
    let mut resource_pools = BTreeSet::<(ResourceKind, String)>::new();
    for supply in &input.resource_supplies {
        resource_pools.insert((supply.resource_kind, supply.allocation_pool_id.clone()));
    }
    for demand in &input.resource_demands {
        resource_pools.insert((demand.resource_kind, demand.allocation_pool_id.clone()));
    }

    for (kind, pool_id) in resource_pools {
        let mut supplies: Vec<&ResourceSupply> = input
            .resource_supplies
            .iter()
            .filter(|supply| {
                supply.resource_kind == kind && supply.allocation_pool_id == pool_id
            })
            .collect();
        supplies.sort_by(|a, b| {
            b.allocation_priority
                .cmp(&a.allocation_priority)
                .then_with(|| a.supply_id.cmp(&b.supply_id))
        });
        let demands: Vec<&ResourceDemand> = input
            .resource_demands
            .iter()
            .filter(|demand| {
                demand.resource_kind == kind && demand.allocation_pool_id == pool_id
            })
            .collect();

        let mut known_remaining = BTreeMap::<String, u64>::new();
        let mut unresolved_remaining = 0_u64;
        for supply in &supplies {
            let available = supply.capacity_units - supply.protected_reserve_units;
            let source_state = effective_states
                .get(&supply.source_component)
                .ok_or_else(|| SettlementError("resource source state missing".into()))?;
            match source_state {
                ComponentState::Available => {
                    known_remaining.insert(supply.supply_id.clone(), available);
                }
                ComponentState::Failed => {}
                ComponentState::Unknown { .. } => {
                    unresolved_remaining = unresolved_remaining
                        .checked_add(available)
                        .ok_or_else(|| SettlementError("unknown resource total overflow".into()))?;
                }
            }
        }

        let mut ordered_demands = demands;
        ordered_demands.sort_by(|a, b| {
            b.priority
                .cmp(&a.priority)
                .then_with(|| a.demand_id.cmp(&b.demand_id))
        });

        // A stale/unknown consumer may still be operating and competing for the
        // same shared inventory. Until its state is resolved, fail closed for
        // the whole affected pool instead of letting known consumers consume
        // resources that may be required by that unknown higher/lower-priority
        // consumer. The scenario remains evaluable, but pool allocations are
        // explicitly blocked rather than guessed.
        let has_unknown_consumer = ordered_demands.iter().any(|demand| {
            effective_states
                .get(&demand.consumer_component)
                .is_some_and(|state| matches!(state, ComponentState::Unknown { .. }))
        });
        let has_potential_supply = known_remaining.values().any(|units| *units > 0)
            || unresolved_remaining > 0;
        if has_unknown_consumer && has_potential_supply {
            let unresolved_at_pool_start = unresolved_remaining;
            for demand in ordered_demands {
                let consumer_state = effective_states
                    .get(&demand.consumer_component)
                    .ok_or_else(|| SettlementError("resource consumer state missing".into()))?;
                let outcome = match consumer_state {
                    ComponentState::Failed => DemandOutcome::ConsumerUnavailable,
                    ComponentState::Unknown { .. } => DemandOutcome::ConsumerStateUnknown,
                    ComponentState::Available => DemandOutcome::PoolBlockedByUnknownConsumer,
                };
                outcomes.push(ResourceAllocation {
                    demand_id: demand.demand_id.clone(),
                    consumer_component: demand.consumer_component.clone(),
                    resource_kind: kind,
                    unit_code: kind.synthetic_unit_code().into(),
                    requested_units: demand.requested_units,
                    minimum_units: demand.minimum_units,
                    allocated_known_units: 0,
                    unresolved_potential_units: unresolved_at_pool_start,
                    sources: Vec::new(),
                    outcome,
                });
            }
            continue;
        }

        for demand in ordered_demands {
            let consumer_state = effective_states
                .get(&demand.consumer_component)
                .ok_or_else(|| SettlementError("resource consumer state missing".into()))?;
            let mut sources = Vec::new();
            let mut allocated = 0_u64;
            // Snapshot before the demand consumes any hypothetical upper-bound
            // capacity from unknown supplies.
            let unresolved_before_demand = unresolved_remaining;
            let outcome = match consumer_state {
                ComponentState::Failed => DemandOutcome::ConsumerUnavailable,
                ComponentState::Unknown { .. } => DemandOutcome::ConsumerStateUnknown,
                ComponentState::Available => {
                    for supply in &supplies {
                        let Some(remaining) = known_remaining.get_mut(&supply.supply_id) else {
                            continue;
                        };
                        if *remaining == 0 || allocated >= demand.requested_units {
                            continue;
                        }
                        let take = (*remaining).min(demand.requested_units - allocated);
                        *remaining -= take;
                        allocated = allocated.checked_add(take).ok_or_else(|| {
                            SettlementError("resource allocation total overflow".into())
                        })?;
                        if take > 0 {
                            sources.push(SourceAllocation {
                                supply_id: supply.supply_id.clone(),
                                units: take,
                            });
                        }
                    }
                    if allocated >= demand.requested_units {
                        DemandOutcome::FullySatisfied
                    } else if allocated >= demand.minimum_units {
                        DemandOutcome::MinimumSatisfied
                    } else {
                        // Unknown capacity is not allocated or represented as available.
                        // For the best-case status test, however, the upper bound is
                        // a shared pool: evaluate it in the declared demand priority
                        // order and do not let the same unknown units make several
                        // incompatible demands all appear individually feasible.
                        let possible_gap = demand.minimum_units - allocated;
                        let potentially_coverable = unresolved_remaining.min(possible_gap);
                        unresolved_remaining -= potentially_coverable;
                        if potentially_coverable == possible_gap {
                            DemandOutcome::Indeterminate
                        } else {
                            DemandOutcome::Shortfall
                        }
                    }
                }
            };

            outcomes.push(ResourceAllocation {
                demand_id: demand.demand_id.clone(),
                consumer_component: demand.consumer_component.clone(),
                resource_kind: kind,
                unit_code: kind.synthetic_unit_code().into(),
                requested_units: demand.requested_units,
                minimum_units: demand.minimum_units,
                allocated_known_units: allocated,
                unresolved_potential_units: unresolved_before_demand,
                sources,
                outcome,
            });
        }
    }

    outcomes.sort_by(|a, b| a.demand_id.cmp(&b.demand_id));
    Ok(outcomes)
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    for byte in digest.iter().copied() {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> ComponentId {
        ComponentId::parse(value).unwrap()
    }

    fn component(
        value: &str,
        unit: &str,
        output_units: u64,
        input_mode: DependencyMode,
        observed_state: ComponentState,
    ) -> Component {
        Component {
            id: id(value),
            generation: 1,
            output_unit: unit.into(),
            nominal_output_units: output_units,
            input_mode,
            observed_state,
        }
    }

    fn edge(upstream: &str, downstream: &str) -> Dependency {
        Dependency {
            upstream_component: id(upstream),
            upstream_generation: 1,
            downstream_component: id(downstream),
        }
    }

    fn fixture() -> ScenarioInput {
        let available = ComponentState::Available;
        ScenarioInput {
            schema_version: SETTLEMENT_SCHEMA_VERSION,
            scenario_id: "synthetic-city-smoke-v1".into(),
            // Deliberately unsorted; evaluation canonicalizes inputs before hashing.
            components: vec![
                component(
                    "water-service",
                    "service.water.delivery.points",
                    100,
                    DependencyMode::AllOf,
                    available.clone(),
                ),
                component(
                    "grid",
                    "component.electricity.availability",
                    100,
                    DependencyMode::AllOf,
                    available.clone(),
                ),
                component(
                    "clinic-service",
                    "service.clinic.points",
                    100,
                    DependencyMode::AllOf,
                    available.clone(),
                ),
                component(
                    "pump",
                    "service.water.pump.points",
                    100,
                    DependencyMode::AnyOf,
                    available.clone(),
                ),
                component(
                    "reservoir",
                    "component.water-storage.availability",
                    100,
                    DependencyMode::AllOf,
                    available.clone(),
                ),
                component(
                    "backup",
                    "component.backup-power.availability",
                    100,
                    DependencyMode::AllOf,
                    available.clone(),
                ),
                component(
                    "clinic-cooling",
                    "service.cooling.points",
                    100,
                    DependencyMode::AnyOf,
                    available,
                ),
            ],
            dependencies: vec![
                edge("grid", "pump"),
                edge("backup", "pump"),
                edge("grid", "clinic-cooling"),
                edge("backup", "clinic-cooling"),
                edge("pump", "water-service"),
                edge("clinic-cooling", "clinic-service"),
            ],
            resource_supplies: vec![
                ResourceSupply {
                    supply_id: "grid-energy".into(),
                    allocation_pool_id: "electrical-common".into(),
                    allocation_priority: 100,
                    source_component: id("grid"),
                    resource_kind: ResourceKind::ElectricalEnergy,
                    capacity_units: 100,
                    protected_reserve_units: 20,
                },
                ResourceSupply {
                    supply_id: "backup-energy".into(),
                    allocation_pool_id: "electrical-common".into(),
                    allocation_priority: 10,
                    source_component: id("backup"),
                    resource_kind: ResourceKind::ElectricalEnergy,
                    capacity_units: 80,
                    protected_reserve_units: 20,
                },
                ResourceSupply {
                    supply_id: "reservoir-water".into(),
                    allocation_pool_id: "potable-water".into(),
                    allocation_priority: 100,
                    source_component: id("reservoir"),
                    resource_kind: ResourceKind::PotableWater,
                    capacity_units: 100,
                    protected_reserve_units: 20,
                },
            ],
            resource_demands: vec![
                ResourceDemand {
                    demand_id: "clinic-power".into(),
                    allocation_pool_id: "electrical-common".into(),
                    consumer_component: id("clinic-cooling"),
                    resource_kind: ResourceKind::ElectricalEnergy,
                    requested_units: 60,
                    minimum_units: 50,
                    priority: 90,
                },
                ResourceDemand {
                    demand_id: "pump-power".into(),
                    allocation_pool_id: "electrical-common".into(),
                    consumer_component: id("pump"),
                    resource_kind: ResourceKind::ElectricalEnergy,
                    requested_units: 50,
                    minimum_units: 40,
                    priority: 80,
                },
                ResourceDemand {
                    demand_id: "water-supply".into(),
                    allocation_pool_id: "potable-water".into(),
                    consumer_component: id("water-service"),
                    resource_kind: ResourceKind::PotableWater,
                    requested_units: 50,
                    minimum_units: 40,
                    priority: 100,
                },
            ],
            service_floors: vec![
                ServiceFloor {
                    floor_id: "water-floor".into(),
                    component_id: id("water-service"),
                    output_unit: "service.water.delivery.points".into(),
                    minimum_output_units: 70,
                    required_demand_ids: vec![
                        "pump-power".into(),
                        "water-supply".into(),
                    ],
                },
                ServiceFloor {
                    floor_id: "clinic-floor".into(),
                    component_id: id("clinic-service"),
                    output_unit: "service.clinic.points".into(),
                    minimum_output_units: 70,
                    required_demand_ids: vec!["clinic-power".into()],
                },
            ],
        }
    }

    fn floor<'a>(report: &'a SettlementReport, floor_id: &str) -> &'a ServiceFloorResult {
        report
            .payload
            .service_floors
            .iter()
            .find(|result| result.floor_id == floor_id)
            .unwrap()
    }

    fn demand<'a>(report: &'a SettlementReport, demand_id: &str) -> &'a ResourceAllocation {
        report
            .payload
            .resource_allocations
            .iter()
            .find(|result| result.demand_id == demand_id)
            .unwrap()
    }

    #[test]
    fn healthy_fixture_meets_both_protected_floors() {
        let report = evaluate_scenario(&fixture()).unwrap();
        assert_eq!(floor(&report, "water-floor").status, FloorStatus::Met);
        assert_eq!(floor(&report, "clinic-floor").status, FloorStatus::Met);
        assert_eq!(demand(&report, "pump-power").outcome, DemandOutcome::FullySatisfied);
        assert_eq!(
            report.payload.claim_ceiling,
            CLAIM_CEILING,
            "the synthetic claim boundary must remain visible"
        );
    }

    #[test]
    fn declared_supply_priority_not_lexicographic_id_controls_source_choice() {
        let report = evaluate_scenario(&fixture()).unwrap();
        let clinic = demand(&report, "clinic-power");
        assert_eq!(clinic.allocated_known_units, 60);
        assert_eq!(clinic.sources.len(), 1);
        assert_eq!(clinic.sources[0].supply_id, "grid-energy");
    }

    #[test]
    fn grid_loss_cascades_and_does_not_hide_water_floor_breach() {
        let mut input = fixture();
        input
            .components
            .iter_mut()
            .find(|component| component.id == id("grid"))
            .unwrap()
            .observed_state = ComponentState::Failed;

        let report = evaluate_scenario(&input).unwrap();
        assert_eq!(floor(&report, "clinic-floor").status, FloorStatus::Met);
        assert_eq!(floor(&report, "water-floor").status, FloorStatus::Breached);
        assert_eq!(demand(&report, "pump-power").outcome, DemandOutcome::Shortfall);
    }

    #[test]
    fn stale_power_evidence_has_shared_uncertainty_budget_and_never_becomes_zero() {
        let mut input = fixture();
        input
            .components
            .iter_mut()
            .find(|component| component.id == id("grid"))
            .unwrap()
            .observed_state = ComponentState::Unknown {
            cause: UnknownCause::StaleTelemetry,
        };
        // Remove known backup capacity so only the grid's bounded unknown
        // capacity remains. The higher-priority clinic demand gets the first
        // hypothetical uncertainty budget; it cannot be double-counted for the
        // pump as well.
        let backup = input
            .resource_supplies
            .iter_mut()
            .find(|supply| supply.supply_id == "backup-energy")
            .unwrap();
        backup.capacity_units = 20;
        backup.protected_reserve_units = 20;

        let report = evaluate_scenario(&input).unwrap();
        assert_eq!(
            demand(&report, "clinic-power").outcome,
            DemandOutcome::Indeterminate
        );
        assert_eq!(demand(&report, "pump-power").outcome, DemandOutcome::Shortfall);
        assert_eq!(floor(&report, "clinic-floor").status, FloorStatus::Blocked);
        assert_eq!(floor(&report, "water-floor").status, FloorStatus::Breached);
        assert_eq!(
            report
                .payload
                .component_results
                .iter()
                .find(|component| component.component_id == id("grid"))
                .unwrap()
                .effective_output_units,
            None
        );
    }

    #[test]
    fn backup_can_cover_grid_loss_when_its_declared_capacity_is_sufficient() {
        let mut input = fixture();
        input
            .components
            .iter_mut()
            .find(|component| component.id == id("grid"))
            .unwrap()
            .observed_state = ComponentState::Failed;
        let backup = input
            .resource_supplies
            .iter_mut()
            .find(|supply| supply.supply_id == "backup-energy")
            .unwrap();
        backup.capacity_units = 200;
        backup.protected_reserve_units = 20;

        let report = evaluate_scenario(&input).unwrap();
        assert_eq!(floor(&report, "clinic-floor").status, FloorStatus::Met);
        assert_eq!(floor(&report, "water-floor").status, FloorStatus::Met);
    }

    #[test]
    fn protected_reserves_are_never_allocated() {
        let input = fixture();
        let report = evaluate_scenario(&input).unwrap();
        let mut consumed = BTreeMap::<String, u64>::new();
        for allocation in &report.payload.resource_allocations {
            for source in &allocation.sources {
                *consumed.entry(source.supply_id.clone()).or_default() += source.units;
            }
        }
        for supply in &input.resource_supplies {
            let used = consumed.get(&supply.supply_id).copied().unwrap_or(0);
            let available = supply.capacity_units - supply.protected_reserve_units;
            assert!(
                used <= available,
                "{} allocated {used} above non-reserve capacity {available}",
                supply.supply_id
            );
        }
    }

    #[test]
    fn resource_kinds_do_not_cross_substitute() {
        let mut input = fixture();
        for supply in &mut input.resource_supplies {
            if supply.resource_kind == ResourceKind::ElectricalEnergy {
                supply.resource_kind = ResourceKind::PotableWater;
            }
        }
        let report = evaluate_scenario(&input).unwrap();
        assert_eq!(demand(&report, "pump-power").outcome, DemandOutcome::Shortfall);
        assert_eq!(floor(&report, "clinic-floor").status, FloorStatus::Breached);
    }

    #[test]
    fn stale_generation_is_rejected() {
        let mut input = fixture();
        input.dependencies[0].upstream_generation = 99;
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn dependency_cycles_are_rejected_fail_closed() {
        let mut input = fixture();
        input.dependencies.push(edge("water-service", "grid"));
        assert!(evaluate_scenario(&input).is_err());
    }

    #[test]
    fn canonical_report_digest_ignores_input_record_order() {
        let input = fixture();
        let first = evaluate_scenario(&input).unwrap();
        let mut reordered = input;
        reordered.components.reverse();
        reordered.dependencies.reverse();
        reordered.resource_supplies.reverse();
        reordered.resource_demands.reverse();
        reordered.service_floors.reverse();
        for floor in &mut reordered.service_floors {
            floor.required_demand_ids.reverse();
        }
        let second = evaluate_scenario(&reordered).unwrap();
        assert_eq!(first.payload.canonical_input_sha256, second.payload.canonical_input_sha256);
        assert_eq!(first.payload_sha256, second.payload_sha256);
    }

    #[test]
    fn duplicate_ids_and_unit_mismatch_are_rejected() {
        let mut duplicate = fixture();
        duplicate.components.push(duplicate.components[0].clone());
        assert!(evaluate_scenario(&duplicate).is_err());

        let mut wrong_unit = fixture();
        wrong_unit.service_floors[0].output_unit = "synthetic.other-unit".into();
        assert!(evaluate_scenario(&wrong_unit).is_err());
    }

    #[test]
    fn same_kind_supply_cannot_cross_an_explicit_allocation_pool() {
        let mut input = fixture();
        input
            .resource_demands
            .iter_mut()
            .find(|demand| demand.demand_id == "pump-power")
            .unwrap()
            .allocation_pool_id = "isolated-electrical".into();

        let report = evaluate_scenario(&input).unwrap();
        assert_eq!(
            demand(&report, "pump-power").allocated_known_units,
            0,
            "power from a different allocation pool must not be borrowed implicitly"
        );
        assert_eq!(demand(&report, "pump-power").outcome, DemandOutcome::Shortfall);
        assert_eq!(floor(&report, "water-floor").status, FloorStatus::Breached);
    }

    #[test]
    fn unknown_consumer_blocks_shared_pool_allocation_fail_closed() {
        let mut input = fixture();
        input
            .components
            .iter_mut()
            .find(|component| component.id == id("clinic-cooling"))
            .unwrap()
            .observed_state = ComponentState::Unknown {
            cause: UnknownCause::ConflictingTelemetry,
        };

        let report = evaluate_scenario(&input).unwrap();
        assert_eq!(
            demand(&report, "clinic-power").outcome,
            DemandOutcome::ConsumerStateUnknown
        );
        assert_eq!(
            demand(&report, "pump-power").outcome,
            DemandOutcome::PoolBlockedByUnknownConsumer
        );
        assert_eq!(floor(&report, "clinic-floor").status, FloorStatus::Blocked);
        assert_eq!(floor(&report, "water-floor").status, FloorStatus::Blocked);
        assert_eq!(demand(&report, "pump-power").allocated_known_units, 0);
    }

    #[test]
    fn unknown_resource_capacity_is_not_double_counted_across_demands() {
        let mut input = fixture();
        input
            .components
            .iter_mut()
            .find(|component| component.id == id("grid"))
            .unwrap()
            .observed_state = ComponentState::Unknown {
            cause: UnknownCause::StaleTelemetry,
        };
        let backup = input
            .resource_supplies
            .iter_mut()
            .find(|supply| supply.supply_id == "backup-energy")
            .unwrap();
        backup.capacity_units = 20;
        backup.protected_reserve_units = 20;
        let grid = input
            .resource_supplies
            .iter_mut()
            .find(|supply| supply.supply_id == "grid-energy")
            .unwrap();
        grid.capacity_units = 80;
        grid.protected_reserve_units = 20;

        let report = evaluate_scenario(&input).unwrap();
        assert_eq!(
            demand(&report, "clinic-power").outcome,
            DemandOutcome::Indeterminate
        );
        assert_eq!(demand(&report, "pump-power").outcome, DemandOutcome::Shortfall);
        assert_eq!(
            demand(&report, "clinic-power").unresolved_potential_units,
            60
        );
        assert_eq!(
            demand(&report, "pump-power").unresolved_potential_units,
            10
        );
    }

    #[test]
    fn resource_allocations_reconcile_to_named_supplies_and_pools() {
        let input = fixture();
        let report = evaluate_scenario(&input).unwrap();
        let supplies: BTreeMap<&str, &ResourceSupply> = input
            .resource_supplies
            .iter()
            .map(|supply| (supply.supply_id.as_str(), supply))
            .collect();
        let demands: BTreeMap<&str, &ResourceDemand> = input
            .resource_demands
            .iter()
            .map(|demand| (demand.demand_id.as_str(), demand))
            .collect();
        let mut consumed_by_supply = BTreeMap::<String, u64>::new();

        for allocation in &report.payload.resource_allocations {
            let source_total = allocation.sources.iter().try_fold(0_u64, |sum, source| {
                let supply = supplies
                    .get(source.supply_id.as_str())
                    .expect("allocation must reference an exact declared supply");
                let demand = demands
                    .get(allocation.demand_id.as_str())
                    .expect("allocation must reference an exact declared demand");
                assert_eq!(supply.resource_kind, allocation.resource_kind);
                assert_eq!(supply.allocation_pool_id, demand.allocation_pool_id);
                assert_eq!(supply.resource_kind, demand.resource_kind);
                assert_eq!(allocation.unit_code, allocation.resource_kind.synthetic_unit_code());
                let total = sum.checked_add(source.units).expect("source sum must not overflow");
                *consumed_by_supply.entry(source.supply_id.clone()).or_default() += source.units;
                Ok::<u64, SettlementError>(total)
            }).unwrap();
            assert_eq!(source_total, allocation.allocated_known_units);
            assert!(allocation.allocated_known_units <= allocation.requested_units);
        }

        for supply in &input.resource_supplies {
            let consumed = consumed_by_supply
                .get(&supply.supply_id)
                .copied()
                .unwrap_or(0);
            assert!(
                consumed <= supply.capacity_units - supply.protected_reserve_units,
                "resource use must not cross declared capacity or protected reserve"
            );
        }
    }

    #[test]
    fn input_contract_rejects_unknown_fields_at_top_level_and_nested_records() {
        let mut top_level = serde_json::to_value(fixture()).unwrap();
        top_level["unreviewed_field"] = serde_json::json!(true);
        assert!(serde_json::from_value::<ScenarioInput>(top_level).is_err());

        let mut nested = serde_json::to_value(fixture()).unwrap();
        nested["components"][0]["unreviewed_capacity_override"] =
            serde_json::json!(999_999_u64);
        assert!(serde_json::from_value::<ScenarioInput>(nested).is_err());

        let mut state = serde_json::to_value(fixture()).unwrap();
        state["components"][0]["observed_state"]["unreviewed_override"] =
            serde_json::json!(true);
        assert!(serde_json::from_value::<ScenarioInput>(state).is_err());
    }

    #[test]
    fn unsupported_schema_version_is_rejected() {
        let mut input = fixture();
        input.schema_version += 1;
        assert!(evaluate_scenario(&input).is_err());
    }
}
