// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Deterministic temporal extension for the synthetic settlement contract.
//!
//! This module is an integer-resource discrete-event model, not a physical city
//! or habitat simulator. Ticks are synthetic model time. Component-state events
//! are declared scenario facts, not observations. Resource inflows, per-tick
//! demand allocation and closing stocks are reconciled exactly.

use super::{
    ComponentId, ComponentState, FloorStatus, ResourceKind, ScenarioInput, SettlementError,
    SettlementReport, ServiceFloorResult, evaluate_scenario, sha256_hex, valid_identifier,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

pub const SETTLEMENT_TIMELINE_SCHEMA_VERSION: u32 = 1;
pub const TIMELINE_CLAIM_CEILING: &str =
    "synthetic_temporal_resource_conservation_and_recovery_not_physical_model";
const MAX_TIMELINE_HORIZON_TICKS: u64 = 10_000;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SupplyStockLimit {
    pub supply_id: String,
    /// Maximum total stock, including the protected reserve.
    pub maximum_stock_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum SettlementEventKind {
    /// Changes the declared observation state for one exact component generation.
    ComponentStateChanged {
        component_id: ComponentId,
        generation: u64,
        state: ComponentState,
    },
    /// Adds synthetic stock through an explicit scenario boundary inflow.
    ResourceInflow {
        supply_id: String,
        units: u64,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementEvent {
    pub event_id: String,
    /// Discrete synthetic model tick. Tick zero is the initial evaluation.
    pub tick: u64,
    /// Explicit order for multiple events at one tick; unique per tick.
    pub sequence: u32,
    pub event: SettlementEventKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementTimelineInput {
    pub schema_version: u32,
    /// The static contract evaluated at tick zero, with capacity fields used
    /// as initial total stocks for this temporal profile.
    pub initial_scenario: ScenarioInput,
    /// The horizon is inclusive: the kernel evaluates ticks 0 through horizon.
    pub horizon_ticks: u64,
    /// Number of consecutive all-floor-met ticks required to claim stable recovery.
    pub stability_window_ticks: u64,
    /// Exactly one maximum-stock declaration is required per resource supply.
    pub stock_limits: Vec<SupplyStockLimit>,
    pub events: Vec<SettlementEvent>,
}

impl SettlementTimelineInput {
    /// Canonicalizes list ordering without removing duplicates; validation still
    /// rejects duplicate identities and conflicting same-tick sequence numbers.
    pub fn canonicalized(&self) -> Self {
        let mut canonical = self.clone();
        canonical.initial_scenario = self.initial_scenario.canonicalized();
        canonical
            .stock_limits
            .sort_by(|a, b| a.supply_id.cmp(&b.supply_id));
        canonical.events.sort_by(|a, b| {
            a.tick
                .cmp(&b.tick)
                .then_with(|| a.sequence.cmp(&b.sequence))
                .then_with(|| a.event_id.cmp(&b.event_id))
        });
        canonical
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceStockTransition {
    pub supply_id: String,
    pub resource_kind: ResourceKind,
    pub unit_code: String,
    pub opening_units: u64,
    pub inflow_units: u64,
    pub consumed_units: u64,
    pub closing_units: u64,
    pub protected_reserve_units: u64,
}

impl ResourceStockTransition {
    /// Integer conservation check: opening + inflow - consumption = closing.
    pub fn conserves(&self) -> bool {
        self.opening_units
            .checked_add(self.inflow_units)
            .and_then(|units| units.checked_sub(self.consumed_units))
            == Some(self.closing_units)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct AppliedEventReceipt {
    pub event_id: String,
    pub tick: u64,
    pub sequence: u32,
    pub event: SettlementEventKind,
    /// Digest of the exact serialized event payload. This is an integrity link,
    /// not a signature or proof that the event describes physical reality.
    pub event_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementTickReceipt {
    pub tick: u64,
    pub applied_events: Vec<AppliedEventReceipt>,
    /// Snapshot evaluation uses the state and stock after this tick's events,
    /// before its reported resource allocation is deducted from closing stock.
    pub snapshot: SettlementReport,
    /// Exact stock ledger transitions for this tick, including declared inflows.
    pub resource_transitions: Vec<ResourceStockTransition>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FloorRecoverySummary {
    pub floor_id: String,
    pub first_non_met_tick: Option<u64>,
    pub first_restoration_tick: Option<u64>,
    /// Earliest post-loss tick beginning a full consecutive stable window.
    pub stable_recovery_tick: Option<u64>,
    pub non_met_tick_count: u64,
    /// Non-Met ticks after the first stable window has completed.
    pub later_non_met_ticks: Vec<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementTimelineReportPayload {
    pub schema_version: u32,
    pub scenario_id: String,
    pub horizon_ticks: u64,
    pub stability_window_ticks: u64,
    pub claim_ceiling: String,
    pub timeline_input_sha256: String,
    pub ticks: Vec<SettlementTickReceipt>,
    pub floor_recovery: Vec<FloorRecoverySummary>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SettlementTimelineReport {
    pub payload: SettlementTimelineReportPayload,
    /// SHA-256 over compact serialized payload bytes; not a signature or proof
    /// that the scenario's inputs describe physical reality.
    pub payload_sha256: String,
}

pub fn validate_timeline(input: &SettlementTimelineInput) -> Result<(), SettlementError> {
    let canonical = input.canonicalized();
    validate_canonical_timeline(&canonical)
}

pub fn evaluate_timeline(
    input: &SettlementTimelineInput,
) -> Result<SettlementTimelineReport, SettlementError> {
    let canonical = input.canonicalized();
    validate_canonical_timeline(&canonical)?;

    let input_bytes = serde_json::to_vec(&canonical)
        .map_err(|error| SettlementError(format!("serialize timeline input: {error}")))?;
    let timeline_input_sha256 = sha256_hex(&input_bytes);
    let stock_limits: BTreeMap<String, u64> = canonical
        .stock_limits
        .iter()
        .map(|limit| (limit.supply_id.clone(), limit.maximum_stock_units))
        .collect();

    let mut scenario = canonical.initial_scenario.canonicalized();
    let mut tick_receipts = Vec::with_capacity((canonical.horizon_ticks + 1) as usize);
    let mut event_cursor = 0_usize;

    for tick in 0..=canonical.horizon_ticks {
        let opening_units: BTreeMap<String, u64> = scenario
            .resource_supplies
            .iter()
            .map(|supply| (supply.supply_id.clone(), supply.capacity_units))
            .collect();
        let mut inflow_units = BTreeMap::<String, u64>::new();
        let mut applied_events = Vec::new();

        while event_cursor < canonical.events.len()
            && canonical.events[event_cursor].tick == tick
        {
            let event = &canonical.events[event_cursor];
            match &event.event {
                SettlementEventKind::ComponentStateChanged {
                    component_id,
                    generation,
                    state,
                } => {
                    let component = scenario
                        .components
                        .iter_mut()
                        .find(|component| component.id == *component_id)
                        .ok_or_else(|| {
                            SettlementError(format!(
                                "timeline component {} disappeared after validation",
                                component_id
                            ))
                        })?;
                    if component.generation != *generation {
                        return Err(SettlementError(format!(
                            "timeline event {} has stale component generation",
                            event.event_id
                        )));
                    }
                    component.observed_state = state.clone();
                }
                SettlementEventKind::ResourceInflow { supply_id, units } => {
                    let supply = scenario
                        .resource_supplies
                        .iter_mut()
                        .find(|supply| supply.supply_id == *supply_id)
                        .ok_or_else(|| {
                            SettlementError(format!(
                                "timeline supply {} disappeared after validation",
                                supply_id
                            ))
                        })?;
                    let next_stock = supply.capacity_units.checked_add(*units).ok_or_else(|| {
                        SettlementError(format!(
                            "resource stock overflow at event {}",
                            event.event_id
                        ))
                    })?;
                    let maximum = stock_limits
                        .get(supply_id)
                        .copied()
                        .ok_or_else(|| {
                            SettlementError("stock limit missing after validation".into())
                        })?;
                    if next_stock > maximum {
                        return Err(SettlementError(format!(
                            "resource inflow {} exceeds maximum stock for {}",
                            event.event_id, supply_id
                        )));
                    }
                    supply.capacity_units = next_stock;
                    let cumulative = inflow_units.entry(supply_id.clone()).or_default();
                    *cumulative = cumulative.checked_add(*units).ok_or_else(|| {
                        SettlementError(format!(
                            "resource inflow ledger overflow at event {}",
                            event.event_id
                        ))
                    })?;
                }
            }
            let event_bytes = serde_json::to_vec(&event.event)
                .map_err(|error| SettlementError(format!("serialize event payload: {error}")))?;
            applied_events.push(AppliedEventReceipt {
                event_id: event.event_id.clone(),
                tick: event.tick,
                sequence: event.sequence,
                event: event.event.clone(),
                event_sha256: sha256_hex(&event_bytes),
            });
            event_cursor += 1;
        }

        let snapshot = evaluate_scenario(&scenario)?;
        let mut consumed_by_supply = BTreeMap::<String, u64>::new();
        for allocation in &snapshot.payload.resource_allocations {
            for source in &allocation.sources {
                let consumed = consumed_by_supply
                    .entry(source.supply_id.clone())
                    .or_default();
                *consumed = consumed.checked_add(source.units).ok_or_else(|| {
                    SettlementError("resource consumption ledger overflow".into())
                })?;
            }
        }

        let mut resource_transitions = Vec::with_capacity(scenario.resource_supplies.len());
        for supply in &mut scenario.resource_supplies {
            let opening = opening_units
                .get(&supply.supply_id)
                .copied()
                .ok_or_else(|| SettlementError("opening stock missing".into()))?;
            let inflow = inflow_units.get(&supply.supply_id).copied().unwrap_or(0);
            let consumed = consumed_by_supply
                .get(&supply.supply_id)
                .copied()
                .unwrap_or(0);
            let after_inflow = opening.checked_add(inflow).ok_or_else(|| {
                SettlementError(format!("opening/inflow overflow for {}", supply.supply_id))
            })?;
            if after_inflow != supply.capacity_units {
                return Err(SettlementError(format!(
                    "stock transition does not reconcile before allocation for {}",
                    supply.supply_id
                )));
            }
            let closing = after_inflow.checked_sub(consumed).ok_or_else(|| {
                SettlementError(format!(
                    "consumption exceeds stock for {}",
                    supply.supply_id
                ))
            })?;
            if closing < supply.protected_reserve_units {
                return Err(SettlementError(format!(
                    "allocation breached protected reserve for {}",
                    supply.supply_id
                )));
            }

            resource_transitions.push(ResourceStockTransition {
                supply_id: supply.supply_id.clone(),
                resource_kind: supply.resource_kind,
                unit_code: supply.resource_kind.synthetic_unit_code().into(),
                opening_units: opening,
                inflow_units: inflow,
                consumed_units: consumed,
                closing_units: closing,
                protected_reserve_units: supply.protected_reserve_units,
            });
            supply.capacity_units = closing;
        }

        resource_transitions.sort_by(|a, b| a.supply_id.cmp(&b.supply_id));
        if resource_transitions.iter().any(|transition| !transition.conserves()) {
            return Err(SettlementError(format!(
                "resource conservation check failed at tick {tick}"
            )));
        }
        tick_receipts.push(SettlementTickReceipt {
            tick,
            applied_events,
            snapshot,
            resource_transitions,
        });
    }

    let floor_recovery = summarize_floor_recovery(
        &tick_receipts,
        canonical.stability_window_ticks,
    )?;
    let payload = SettlementTimelineReportPayload {
        schema_version: SETTLEMENT_TIMELINE_SCHEMA_VERSION,
        scenario_id: canonical.initial_scenario.scenario_id,
        horizon_ticks: canonical.horizon_ticks,
        stability_window_ticks: canonical.stability_window_ticks,
        claim_ceiling: TIMELINE_CLAIM_CEILING.into(),
        timeline_input_sha256,
        ticks: tick_receipts,
        floor_recovery,
    };
    let payload_bytes = serde_json::to_vec(&payload)
        .map_err(|error| SettlementError(format!("serialize timeline report: {error}")))?;
    let payload_sha256 = sha256_hex(&payload_bytes);
    Ok(SettlementTimelineReport {
        payload,
        payload_sha256,
    })
}

fn validate_canonical_timeline(
    input: &SettlementTimelineInput,
) -> Result<(), SettlementError> {
    if input.schema_version != SETTLEMENT_TIMELINE_SCHEMA_VERSION {
        return Err(SettlementError(format!(
            "unsupported settlement timeline schema version {}",
            input.schema_version
        )));
    }
    if input.horizon_ticks > MAX_TIMELINE_HORIZON_TICKS {
        return Err(SettlementError(format!(
            "timeline horizon exceeds bounded maximum {MAX_TIMELINE_HORIZON_TICKS}"
        )));
    }
    if input.stability_window_ticks == 0
        || input.stability_window_ticks > input.horizon_ticks.saturating_add(1)
    {
        return Err(SettlementError(
            "stability window must fit within the inclusive timeline horizon".into(),
        ));
    }

    // Reuse the v1 contract validator rather than creating parallel component,
    // dependency, resource-pool, reserve or service-floor semantics.
    evaluate_scenario(&input.initial_scenario)?;

    let components: BTreeMap<ComponentId, u64> = input
        .initial_scenario
        .components
        .iter()
        .map(|component| (component.id.clone(), component.generation))
        .collect();
    let supplies: BTreeMap<&str, &super::ResourceSupply> = input
        .initial_scenario
        .resource_supplies
        .iter()
        .map(|supply| (supply.supply_id.as_str(), supply))
        .collect();
    let mut limits = BTreeSet::new();
    for limit in &input.stock_limits {
        valid_identifier(&limit.supply_id, "stock-limit supply ID")?;
        if !limits.insert(limit.supply_id.clone()) {
            return Err(SettlementError(format!(
                "duplicate stock limit for {}",
                limit.supply_id
            )));
        }
        let supply = supplies.get(limit.supply_id.as_str()).ok_or_else(|| {
            SettlementError(format!("stock limit references unknown supply {}", limit.supply_id))
        })?;
        if limit.maximum_stock_units < supply.capacity_units {
            return Err(SettlementError(format!(
                "maximum stock is below initial stock for {}",
                limit.supply_id
            )));
        }
    }
    if limits.len() != supplies.len() {
        return Err(SettlementError(
            "every resource supply must have exactly one stock limit".into(),
        ));
    }

    let mut event_ids = BTreeSet::new();
    let mut event_sequences = BTreeSet::new();
    for event in &input.events {
        valid_identifier(&event.event_id, "settlement event ID")?;
        if !event_ids.insert(event.event_id.clone()) {
            return Err(SettlementError(format!(
                "duplicate settlement event ID {}",
                event.event_id
            )));
        }
        if event.tick > input.horizon_ticks {
            return Err(SettlementError(format!(
                "event {} falls beyond the timeline horizon",
                event.event_id
            )));
        }
        if !event_sequences.insert((event.tick, event.sequence)) {
            return Err(SettlementError(format!(
                "duplicate event sequence {} at tick {}",
                event.sequence, event.tick
            )));
        }
        match &event.event {
            SettlementEventKind::ComponentStateChanged {
                component_id,
                generation,
                ..
            } => {
                let current_generation = components.get(component_id).ok_or_else(|| {
                    SettlementError(format!(
                        "event {} references unknown component {}",
                        event.event_id, component_id
                    ))
                })?;
                if current_generation != generation {
                    return Err(SettlementError(format!(
                        "event {} references stale component generation",
                        event.event_id
                    )));
                }
            }
            SettlementEventKind::ResourceInflow { supply_id, units } => {
                valid_identifier(supply_id, "resource inflow supply ID")?;
                if !supplies.contains_key(supply_id.as_str()) {
                    return Err(SettlementError(format!(
                        "event {} references unknown supply {}",
                        event.event_id, supply_id
                    )));
                }
                if *units == 0 {
                    return Err(SettlementError(format!(
                        "event {} has a zero resource inflow",
                        event.event_id
                    )));
                }
            }
        }
    }

    Ok(())
}

fn summarize_floor_recovery(
    ticks: &[SettlementTickReceipt],
    stability_window_ticks: u64,
) -> Result<Vec<FloorRecoverySummary>, SettlementError> {
    let first_tick = ticks
        .first()
        .ok_or_else(|| SettlementError("timeline produced no ticks".into()))?;
    let mut floor_ids: Vec<String> = first_tick
        .snapshot
        .payload
        .service_floors
        .iter()
        .map(|floor| floor.floor_id.clone())
        .collect();
    floor_ids.sort();

    let window = usize::try_from(stability_window_ticks)
        .map_err(|_| SettlementError("stability window does not fit usize".into()))?;
    let mut summaries = Vec::with_capacity(floor_ids.len());
    for floor_id in floor_ids {
        let floor_points: Vec<&ServiceFloorResult> = ticks
            .iter()
            .map(|tick| {
                tick.snapshot
                    .payload
                    .service_floors
                    .iter()
                    .find(|floor| floor.floor_id == floor_id)
                    .ok_or_else(|| {
                        SettlementError(format!(
                            "floor {} missing from a tick receipt",
                            floor_id
                        ))
                    })
            })
            .collect::<Result<_, _>>()?;
        let first_loss_index = floor_points
            .iter()
            .position(|floor| floor.status != FloorStatus::Met);
        let first_restoration_index = first_loss_index.and_then(|loss_index| {
            floor_points
                .iter()
                .enumerate()
                .skip(loss_index + 1)
                .find(|(_, floor)| floor.status == FloorStatus::Met)
                .map(|(index, _)| index)
        });

        let stable_index = first_loss_index.and_then(|loss_index| {
            let last_window_start = floor_points.len().checked_sub(window)?;
            if loss_index + 1 > last_window_start {
                return None;
            }
            (loss_index + 1..=last_window_start).find(|start| {
                floor_points[*start..*start + window]
                    .iter()
                    .all(|floor| floor.status == FloorStatus::Met)
            })
        });
        let later_non_met_ticks = stable_index
            .map(|stable_start| {
                let first_after_window = stable_start + window;
                floor_points
                    .iter()
                    .enumerate()
                    .skip(first_after_window)
                    .filter(|(_, floor)| floor.status != FloorStatus::Met)
                    .map(|(index, _)| ticks[index].tick)
                    .collect()
            })
            .unwrap_or_default();

        summaries.push(FloorRecoverySummary {
            floor_id,
            first_non_met_tick: first_loss_index.map(|index| ticks[index].tick),
            first_restoration_tick: first_restoration_index.map(|index| ticks[index].tick),
            stable_recovery_tick: stable_index.map(|index| ticks[index].tick),
            non_met_tick_count: floor_points
                .iter()
                .filter(|floor| floor.status != FloorStatus::Met)
                .count() as u64,
            later_non_met_ticks,
        });
    }
    Ok(summaries)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        Component, Dependency, DependencyMode, ResourceDemand, ResourceSupply, ServiceFloor,
        SETTLEMENT_SCHEMA_VERSION,
    };

    fn id(value: &str) -> ComponentId {
        ComponentId::parse(value).unwrap()
    }

    fn component(
        value: &str,
        unit: &str,
        mode: DependencyMode,
        state: ComponentState,
    ) -> Component {
        Component {
            id: id(value),
            generation: 1,
            output_unit: unit.into(),
            nominal_output_units: 100,
            input_mode: mode,
            observed_state: state,
        }
    }

    fn edge(upstream: &str, downstream: &str) -> Dependency {
        Dependency {
            upstream_component: id(upstream),
            upstream_generation: 1,
            downstream_component: id(downstream),
        }
    }

    fn fixture() -> SettlementTimelineInput {
        let available = ComponentState::Available;
        let initial_scenario = ScenarioInput {
            schema_version: SETTLEMENT_SCHEMA_VERSION,
            scenario_id: "temporal-water-smoke-v1".into(),
            components: vec![
                component(
                    "grid",
                    "component.grid.availability",
                    DependencyMode::AllOf,
                    available.clone(),
                ),
                component(
                    "backup",
                    "component.backup.availability",
                    DependencyMode::AllOf,
                    available.clone(),
                ),
                component(
                    "pump",
                    "service.water.pump.points",
                    DependencyMode::AnyOf,
                    available.clone(),
                ),
                component(
                    "water-service",
                    "service.water.delivery.points",
                    DependencyMode::AllOf,
                    available,
                ),
            ],
            dependencies: vec![
                edge("grid", "pump"),
                edge("backup", "pump"),
                edge("pump", "water-service"),
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
                    capacity_units: 40,
                    protected_reserve_units: 20,
                },
                ResourceSupply {
                    supply_id: "reservoir-water".into(),
                    allocation_pool_id: "potable-water".into(),
                    allocation_priority: 100,
                    source_component: id("water-service"),
                    resource_kind: ResourceKind::PotableWater,
                    capacity_units: 200,
                    protected_reserve_units: 20,
                },
            ],
            resource_demands: vec![
                ResourceDemand {
                    demand_id: "pump-power".into(),
                    allocation_pool_id: "electrical-common".into(),
                    consumer_component: id("pump"),
                    resource_kind: ResourceKind::ElectricalEnergy,
                    requested_units: 40,
                    minimum_units: 30,
                    priority: 100,
                },
                ResourceDemand {
                    demand_id: "water-supply".into(),
                    allocation_pool_id: "potable-water".into(),
                    consumer_component: id("water-service"),
                    resource_kind: ResourceKind::PotableWater,
                    requested_units: 20,
                    minimum_units: 10,
                    priority: 90,
                },
            ],
            service_floors: vec![ServiceFloor {
                floor_id: "water-floor".into(),
                component_id: id("water-service"),
                output_unit: "service.water.delivery.points".into(),
                minimum_output_units: 70,
                required_demand_ids: vec!["pump-power".into(), "water-supply".into()],
            }],
        };

        SettlementTimelineInput {
            schema_version: SETTLEMENT_TIMELINE_SCHEMA_VERSION,
            initial_scenario,
            horizon_ticks: 4,
            stability_window_ticks: 2,
            stock_limits: vec![
                SupplyStockLimit {
                    supply_id: "grid-energy".into(),
                    maximum_stock_units: 100,
                },
                SupplyStockLimit {
                    supply_id: "backup-energy".into(),
                    maximum_stock_units: 40,
                },
                SupplyStockLimit {
                    supply_id: "reservoir-water".into(),
                    maximum_stock_units: 200,
                },
            ],
            events: vec![
                SettlementEvent {
                    event_id: "grid-down".into(),
                    tick: 1,
                    sequence: 0,
                    event: SettlementEventKind::ComponentStateChanged {
                        component_id: id("grid"),
                        generation: 1,
                        state: ComponentState::Failed,
                    },
                },
                SettlementEvent {
                    event_id: "grid-up".into(),
                    tick: 3,
                    sequence: 0,
                    event: SettlementEventKind::ComponentStateChanged {
                        component_id: id("grid"),
                        generation: 1,
                        state: ComponentState::Available,
                    },
                },
                SettlementEvent {
                    event_id: "grid-replenished".into(),
                    tick: 4,
                    sequence: 0,
                    event: SettlementEventKind::ResourceInflow {
                        supply_id: "grid-energy".into(),
                        units: 40,
                    },
                },
            ],
        }
    }

    fn water_floor(report: &SettlementTimelineReport, tick: u64) -> &ServiceFloorResult {
        &report.payload.ticks[tick as usize]
            .snapshot
            .payload
            .service_floors[0]
    }

    #[test]
    fn resource_ledger_conserves_stock_and_measures_stable_recovery() {
        let report = evaluate_timeline(&fixture()).unwrap();
        assert_eq!(report.payload.ticks.len(), 5);
        assert_eq!(water_floor(&report, 0).status, FloorStatus::Met);
        assert_eq!(water_floor(&report, 1).status, FloorStatus::Breached);
        assert_eq!(water_floor(&report, 2).status, FloorStatus::Breached);
        assert_eq!(water_floor(&report, 3).status, FloorStatus::Met);
        assert_eq!(water_floor(&report, 4).status, FloorStatus::Met);

        let recovery = report
            .payload
            .floor_recovery
            .iter()
            .find(|floor| floor.floor_id == "water-floor")
            .unwrap();
        assert_eq!(recovery.first_non_met_tick, Some(1));
        assert_eq!(recovery.first_restoration_tick, Some(3));
        assert_eq!(recovery.stable_recovery_tick, Some(3));
        assert_eq!(recovery.later_non_met_ticks, Vec::<u64>::new());

        let grid_tick_4 = report.payload.ticks[4]
            .resource_transitions
            .iter()
            .find(|transition| transition.supply_id == "grid-energy")
            .unwrap();
        assert_eq!(grid_tick_4.opening_units, 20);
        assert_eq!(grid_tick_4.inflow_units, 40);
        assert_eq!(grid_tick_4.consumed_units, 40);
        assert_eq!(grid_tick_4.closing_units, 20);
        assert!(report.payload.ticks.iter().flat_map(|tick| &tick.resource_transitions).all(
            ResourceStockTransition::conserves
        ));
    }

    #[test]
    fn timeline_digest_is_independent_of_input_event_order() {
        let input = fixture();
        let first = evaluate_timeline(&input).unwrap();
        let mut reordered = input;
        reordered.events.reverse();
        reordered.stock_limits.reverse();
        reordered.initial_scenario.components.reverse();
        reordered.initial_scenario.dependencies.reverse();
        reordered.initial_scenario.resource_supplies.reverse();
        reordered.initial_scenario.resource_demands.reverse();
        reordered.initial_scenario.service_floors.reverse();
        let second = evaluate_timeline(&reordered).unwrap();
        assert_eq!(first.payload.timeline_input_sha256, second.payload.timeline_input_sha256);
        assert_eq!(first.payload_sha256, second.payload_sha256);
    }

    #[test]
    fn later_floor_failure_is_retained_after_stable_recovery() {
        let mut input = fixture();
        input.horizon_ticks = 5;
        input.events.push(SettlementEvent {
            event_id: "grid-down-again".into(),
            tick: 5,
            sequence: 0,
            event: SettlementEventKind::ComponentStateChanged {
                component_id: id("grid"),
                generation: 1,
                state: ComponentState::Failed,
            },
        });

        let report = evaluate_timeline(&input).unwrap();
        let recovery = report
            .payload
            .floor_recovery
            .iter()
            .find(|floor| floor.floor_id == "water-floor")
            .unwrap();
        assert_eq!(recovery.first_non_met_tick, Some(1));
        assert_eq!(recovery.first_restoration_tick, Some(3));
        assert_eq!(recovery.stable_recovery_tick, Some(3));
        assert_eq!(recovery.later_non_met_ticks, vec![5]);
        assert_eq!(water_floor(&report, 5).status, FloorStatus::Breached);
    }

    #[test]
    fn stale_timeline_evidence_blocks_floor_and_can_recover_when_refreshed() {
        let mut input = fixture();
        input.horizon_ticks = 3;
        let grid_supply = input
            .initial_scenario
            .resource_supplies
            .iter_mut()
            .find(|supply| supply.supply_id == "grid-energy")
            .unwrap();
        grid_supply.capacity_units = 180;
        let grid_limit = input
            .stock_limits
            .iter_mut()
            .find(|limit| limit.supply_id == "grid-energy")
            .unwrap();
        grid_limit.maximum_stock_units = 180;
        input.events = vec![
            SettlementEvent {
                event_id: "grid-stale".into(),
                tick: 1,
                sequence: 0,
                event: SettlementEventKind::ComponentStateChanged {
                    component_id: id("grid"),
                    generation: 1,
                    state: ComponentState::Unknown {
                        cause: super::super::UnknownCause::StaleTelemetry,
                    },
                },
            },
            SettlementEvent {
                event_id: "grid-evidence-refreshed".into(),
                tick: 2,
                sequence: 0,
                event: SettlementEventKind::ComponentStateChanged {
                    component_id: id("grid"),
                    generation: 1,
                    state: ComponentState::Available,
                },
            },
        ];

        let report = evaluate_timeline(&input).unwrap();
        assert_eq!(water_floor(&report, 0).status, FloorStatus::Met);
        assert_eq!(water_floor(&report, 1).status, FloorStatus::Blocked);
        assert_eq!(water_floor(&report, 2).status, FloorStatus::Met);
        let recovery = &report.payload.floor_recovery[0];
        assert_eq!(recovery.first_non_met_tick, Some(1));
        assert_eq!(recovery.first_restoration_tick, Some(2));
        assert_eq!(recovery.stable_recovery_tick, Some(2));
    }

    #[test]
    fn independent_verifier_accepts_an_untampered_timeline_report() {
        let report = evaluate_timeline(&fixture()).unwrap();
        assert!(crate::verifier::verify_timeline_report(&report).is_ok());
    }

    #[test]
    fn independent_verifier_rejects_conservation_tampering_even_if_digest_is_recomputed() {
        let mut report = evaluate_timeline(&fixture()).unwrap();
        report.payload.ticks[0].resource_transitions[0].closing_units += 1;
        let encoded = serde_json::to_vec(&report.payload).unwrap();
        report.payload_sha256 = super::super::sha256_hex(&encoded);
        assert!(crate::verifier::verify_timeline_report(&report).is_err());
    }

    #[test]
    fn independent_verifier_rederives_recovery_summary() {
        let mut report = evaluate_timeline(&fixture()).unwrap();
        let recovery = report
            .payload
            .floor_recovery
            .iter_mut()
            .find(|floor| floor.floor_id == "water-floor")
            .unwrap();
        recovery.stable_recovery_tick = Some(4);
        let encoded = serde_json::to_vec(&report.payload).unwrap();
        report.payload_sha256 = super::super::sha256_hex(&encoded);
        assert!(crate::verifier::verify_timeline_report(&report).is_err());
    }

    #[test]
    fn independent_verifier_reconciles_event_inflow_after_digests_are_recomputed() {
        let mut report = evaluate_timeline(&fixture()).unwrap();
        let event = report.payload.ticks[4].applied_events.first_mut().unwrap();
        match &mut event.event {
            SettlementEventKind::ResourceInflow { units, .. } => *units -= 1,
            SettlementEventKind::ComponentStateChanged { .. } => {
                panic!("fixture event at tick four should be an inflow")
            }
        }
        let event_bytes = serde_json::to_vec(&event.event).unwrap();
        event.event_sha256 = super::super::sha256_hex(&event_bytes);
        let payload_bytes = serde_json::to_vec(&report.payload).unwrap();
        report.payload_sha256 = super::super::sha256_hex(&payload_bytes);
        assert!(crate::verifier::verify_timeline_report(&report).is_err());
    }

    #[test]
    fn duplicate_event_sequence_at_one_tick_is_rejected() {
        let mut input = fixture();
        input.events.push(SettlementEvent {
            event_id: "grid-unknown".into(),
            tick: 1,
            sequence: 0,
            event: SettlementEventKind::ComponentStateChanged {
                component_id: id("grid"),
                generation: 1,
                state: ComponentState::Unknown {
                    cause: super::super::UnknownCause::StaleTelemetry,
                },
            },
        });
        assert!(evaluate_timeline(&input).is_err());
    }

    #[test]
    fn inflow_above_maximum_stock_is_rejected() {
        let mut input = fixture();
        input.events.push(SettlementEvent {
            event_id: "overfill-grid".into(),
            tick: 4,
            sequence: 1,
            event: SettlementEventKind::ResourceInflow {
                supply_id: "grid-energy".into(),
                units: 81,
            },
        });
        assert!(evaluate_timeline(&input).is_err());
    }

    #[test]
    fn stale_component_generation_and_unknown_fields_are_rejected() {
        let mut stale = fixture();
        if let SettlementEventKind::ComponentStateChanged { generation, .. } =
            &mut stale.events[0].event
        {
            *generation = 44;
        }
        assert!(evaluate_timeline(&stale).is_err());

        let mut json = serde_json::to_value(fixture()).unwrap();
        json["unreviewed_policy"] = serde_json::json!("optimistic");
        assert!(serde_json::from_value::<SettlementTimelineInput>(json).is_err());
    }
}
