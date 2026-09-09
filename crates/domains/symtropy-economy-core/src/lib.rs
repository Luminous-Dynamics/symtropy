// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Causal resource, inventory, production, and shipment flows.
//!
//! V0 enforces one important gameplay theorem: destination inventory does not
//! appear merely because a transfer was requested. Production consumes explicit
//! inputs before outputs can complete, and shipments remove cargo at departure
//! before any destination stock is created at arrival.
//!
//! This is an economic/inventory authority only. It does not prove physical
//! existence, ownership, route safety, orbital reachability, payment, or price.

use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeMap,
    error::Error,
    fmt,
};
use symtropy_game_state::StableId;

/// Exact resource + unit identity.
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub struct ResourceKey {
    /// Resource/commodity/material identity.
    pub resource_id: StableId,
    /// Exact unit identity; incompatible units never silently mix.
    pub unit_id: StableId,
}

/// Inventory/storage/production node in the economic graph.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InventoryNode {
    /// Stable node identity.
    pub id: StableId,
    /// Physical/logical location owned by another world domain.
    pub location_id: StableId,
    /// Optional operating institution/actor. This is not ownership.
    pub operator_id: Option<StableId>,
}

/// Explicit starting inventory used only when constructing a new economic world.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GenesisBalance {
    pub node_id: StableId,
    pub resource: ResourceKey,
    pub amount: u64,
}

/// Deterministic recipe transforming explicit inputs into explicit outputs.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRecipe {
    pub id: StableId,
    /// Inputs required per batch.
    pub inputs: BTreeMap<ResourceKey, u64>,
    /// Outputs produced per batch.
    pub outputs: BTreeMap<ResourceKey, u64>,
    /// Minimum canonical ticks between run start and completion.
    pub duration_ticks: u64,
    /// Optional facility/capability class interpreted by a higher layer.
    pub facility_class_id: Option<StableId>,
    /// Causal-history event that registered/authorized the recipe content.
    pub source_event_id: StableId,
}

/// Started production run. Inputs have already left inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessRun {
    pub id: StableId,
    pub recipe_id: StableId,
    pub node_id: StableId,
    pub batches: u64,
    pub started_tick: u64,
    pub source_event_id: StableId,
}

/// Immutable completion record. Outputs enter inventory only on completion.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProcessCompletion {
    pub id: StableId,
    pub run_id: StableId,
    pub completed_tick: u64,
    pub source_event_id: StableId,
}

/// Shipment after cargo has left source inventory.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipmentDeparture {
    pub id: StableId,
    pub source_node_id: StableId,
    pub destination_node_id: StableId,
    pub resource: ResourceKey,
    pub amount: u64,
    pub departed_tick: u64,
    /// Earliest arrival allowed by the transport/orbital/path provider.
    pub earliest_arrival_tick: u64,
    /// Optional carrier asset identity owned by another domain.
    pub carrier_asset_id: Option<StableId>,
    pub source_event_id: StableId,
}

/// Immutable successful arrival. Destination inventory is credited here.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipmentArrival {
    pub id: StableId,
    pub shipment_id: StableId,
    pub arrived_tick: u64,
    pub source_event_id: StableId,
}

/// Immutable loss/destruction/seizure sink for cargo already in transit.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ShipmentLoss {
    pub id: StableId,
    pub shipment_id: StableId,
    pub lost_tick: u64,
    /// Higher layers can classify piracy, accident, spoilage, destruction, etc.
    pub cause: String,
    pub source_event_id: StableId,
}

/// Current lifecycle state derivable from immutable shipment records.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ShipmentState {
    InTransit,
    Arrived,
    Lost,
}

/// Headless economic state with append-only process/shipment provenance.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct EconomyWorld {
    nodes: BTreeMap<StableId, InventoryNode>,
    balances: BTreeMap<(StableId, ResourceKey), u64>,
    recipes: BTreeMap<StableId, ProcessRecipe>,
    runs: BTreeMap<StableId, ProcessRun>,
    completions: BTreeMap<StableId, ProcessCompletion>,
    completion_by_run: BTreeMap<StableId, StableId>,
    shipments: BTreeMap<StableId, ShipmentDeparture>,
    arrivals: BTreeMap<StableId, ShipmentArrival>,
    arrival_by_shipment: BTreeMap<StableId, StableId>,
    losses: BTreeMap<StableId, ShipmentLoss>,
    loss_by_shipment: BTreeMap<StableId, StableId>,
}

impl EconomyWorld {
    /// Creates a world from an explicit genesis inventory cut.
    ///
    /// There is deliberately no public `mint()` API after construction.
    pub fn from_genesis(
        nodes: impl IntoIterator<Item = InventoryNode>,
        balances: impl IntoIterator<Item = GenesisBalance>,
    ) -> Result<Self, EconomyError> {
        let mut node_map = BTreeMap::new();
        for node in nodes {
            if node_map.insert(node.id.clone(), node).is_some() {
                return Err(EconomyError::DuplicateNode);
            }
        }

        let mut balance_map = BTreeMap::new();
        for balance in balances {
            if !node_map.contains_key(&balance.node_id) {
                return Err(EconomyError::UnknownNode(balance.node_id));
            }
            if balance.amount == 0 {
                return Err(EconomyError::ZeroAmount);
            }
            let key = (balance.node_id, balance.resource);
            if balance_map.insert(key, balance.amount).is_some() {
                return Err(EconomyError::DuplicateGenesisBalance);
            }
        }

        Ok(Self {
            nodes: node_map,
            balances: balance_map,
            recipes: BTreeMap::new(),
            runs: BTreeMap::new(),
            completions: BTreeMap::new(),
            completion_by_run: BTreeMap::new(),
            shipments: BTreeMap::new(),
            arrivals: BTreeMap::new(),
            arrival_by_shipment: BTreeMap::new(),
            losses: BTreeMap::new(),
            loss_by_shipment: BTreeMap::new(),
        })
    }

    /// Adds an empty inventory node after genesis.
    pub fn register_node(&mut self, node: InventoryNode) -> Result<(), EconomyError> {
        if self.nodes.contains_key(&node.id) {
            return Err(EconomyError::DuplicateNode);
        }
        self.nodes.insert(node.id.clone(), node);
        Ok(())
    }

    /// Returns current inventory quantity for one exact node/resource/unit.
    pub fn balance(&self, node_id: &StableId, resource: &ResourceKey) -> u64 {
        self.balances
            .get(&(node_id.clone(), resource.clone()))
            .copied()
            .unwrap_or(0)
    }

    /// Registers one recipe. Free-output recipes are rejected in V0: extraction
    /// must consume an explicit reserve/feedstock/energy/input resource supplied by
    /// the relevant world adapter.
    pub fn register_recipe(&mut self, recipe: ProcessRecipe) -> Result<(), EconomyError> {
        if self.recipes.contains_key(&recipe.id) {
            return Err(EconomyError::DuplicateRecipe(recipe.id));
        }
        validate_recipe(&recipe)?;
        self.recipes.insert(recipe.id.clone(), recipe);
        Ok(())
    }

    pub fn recipe(&self, recipe_id: &StableId) -> Option<&ProcessRecipe> {
        self.recipes.get(recipe_id)
    }

    /// Starts production atomically by removing every required input.
    ///
    /// If any required input is unavailable or multiplication overflows, no input
    /// balance is changed and no run record is created.
    pub fn start_process(&mut self, run: ProcessRun) -> Result<(), EconomyError> {
        if self.runs.contains_key(&run.id) {
            return Err(EconomyError::DuplicateProcessRun(run.id));
        }
        if run.batches == 0 {
            return Err(EconomyError::ZeroBatches);
        }
        if !self.nodes.contains_key(&run.node_id) {
            return Err(EconomyError::UnknownNode(run.node_id));
        }
        let recipe = self
            .recipes
            .get(&run.recipe_id)
            .ok_or_else(|| EconomyError::UnknownRecipe(run.recipe_id.clone()))?;

        let mut debits = Vec::with_capacity(recipe.inputs.len());
        for (resource, per_batch) in &recipe.inputs {
            let required = per_batch
                .checked_mul(run.batches)
                .ok_or(EconomyError::AmountOverflow)?;
            let current = self.balance(&run.node_id, resource);
            if current < required {
                return Err(EconomyError::InsufficientBalance {
                    node_id: run.node_id.clone(),
                    resource: resource.clone(),
                    available: current,
                    required,
                });
            }
            debits.push((resource.clone(), current - required));
        }

        for (resource, remaining) in debits {
            set_balance(&mut self.balances, &run.node_id, resource, remaining);
        }
        self.runs.insert(run.id.clone(), run);
        Ok(())
    }

    /// Completes a run and credits outputs only after recipe duration has elapsed.
    pub fn complete_process(
        &mut self,
        completion: ProcessCompletion,
    ) -> Result<(), EconomyError> {
        if self.completions.contains_key(&completion.id) {
            return Err(EconomyError::DuplicateProcessCompletion(completion.id));
        }
        let run = self
            .runs
            .get(&completion.run_id)
            .ok_or_else(|| EconomyError::UnknownProcessRun(completion.run_id.clone()))?;
        if self.completion_by_run.contains_key(&completion.run_id) {
            return Err(EconomyError::ProcessAlreadyCompleted(
                completion.run_id,
            ));
        }
        let recipe = self
            .recipes
            .get(&run.recipe_id)
            .expect("registered run references retained recipe");
        let earliest = run
            .started_tick
            .checked_add(recipe.duration_ticks)
            .ok_or(EconomyError::TickOverflow)?;
        if completion.completed_tick < earliest {
            return Err(EconomyError::ProcessCompletedTooEarly {
                run_id: completion.run_id,
                earliest_tick: earliest,
                attempted_tick: completion.completed_tick,
            });
        }

        let mut credits = Vec::with_capacity(recipe.outputs.len());
        for (resource, per_batch) in &recipe.outputs {
            let produced = per_batch
                .checked_mul(run.batches)
                .ok_or(EconomyError::AmountOverflow)?;
            let current = self.balance(&run.node_id, resource);
            let updated = current
                .checked_add(produced)
                .ok_or(EconomyError::AmountOverflow)?;
            credits.push((resource.clone(), updated));
        }
        for (resource, updated) in credits {
            set_balance(&mut self.balances, &run.node_id, resource, updated);
        }

        self.completion_by_run
            .insert(completion.run_id.clone(), completion.id.clone());
        self.completions
            .insert(completion.id.clone(), completion);
        Ok(())
    }

    pub fn process_run(&self, run_id: &StableId) -> Option<&ProcessRun> {
        self.runs.get(run_id)
    }

    pub fn completion_for_run(&self, run_id: &StableId) -> Option<&ProcessCompletion> {
        self.completion_by_run
            .get(run_id)
            .and_then(|id| self.completions.get(id))
    }

    /// Departs a shipment by atomically debiting source inventory.
    pub fn depart_shipment(
        &mut self,
        shipment: ShipmentDeparture,
    ) -> Result<(), EconomyError> {
        if self.shipments.contains_key(&shipment.id) {
            return Err(EconomyError::DuplicateShipment(shipment.id));
        }
        if shipment.amount == 0 {
            return Err(EconomyError::ZeroAmount);
        }
        if shipment.source_node_id == shipment.destination_node_id {
            return Err(EconomyError::SameShipmentEndpoint(shipment.source_node_id));
        }
        if !self.nodes.contains_key(&shipment.source_node_id) {
            return Err(EconomyError::UnknownNode(shipment.source_node_id));
        }
        if !self.nodes.contains_key(&shipment.destination_node_id) {
            return Err(EconomyError::UnknownNode(shipment.destination_node_id));
        }
        if shipment.earliest_arrival_tick < shipment.departed_tick {
            return Err(EconomyError::ArrivalBeforeDeparture {
                shipment_id: shipment.id,
                departed_tick: shipment.departed_tick,
                earliest_arrival_tick: shipment.earliest_arrival_tick,
            });
        }
        let current = self.balance(&shipment.source_node_id, &shipment.resource);
        if current < shipment.amount {
            return Err(EconomyError::InsufficientBalance {
                node_id: shipment.source_node_id,
                resource: shipment.resource,
                available: current,
                required: shipment.amount,
            });
        }
        let remaining = current - shipment.amount;
        set_balance(
            &mut self.balances,
            &shipment.source_node_id,
            shipment.resource.clone(),
            remaining,
        );
        self.shipments.insert(shipment.id.clone(), shipment);
        Ok(())
    }

    /// Arrives a shipment and only then credits destination inventory.
    pub fn arrive_shipment(&mut self, arrival: ShipmentArrival) -> Result<(), EconomyError> {
        if self.arrivals.contains_key(&arrival.id) {
            return Err(EconomyError::DuplicateShipmentArrival(arrival.id));
        }
        let shipment = self
            .shipments
            .get(&arrival.shipment_id)
            .ok_or_else(|| EconomyError::UnknownShipment(arrival.shipment_id.clone()))?;
        if self.arrival_by_shipment.contains_key(&arrival.shipment_id)
            || self.loss_by_shipment.contains_key(&arrival.shipment_id)
        {
            return Err(EconomyError::ShipmentAlreadySettled(arrival.shipment_id));
        }
        if arrival.arrived_tick < shipment.earliest_arrival_tick {
            return Err(EconomyError::ShipmentArrivedTooEarly {
                shipment_id: arrival.shipment_id,
                earliest_tick: shipment.earliest_arrival_tick,
                attempted_tick: arrival.arrived_tick,
            });
        }
        let current = self.balance(&shipment.destination_node_id, &shipment.resource);
        let updated = current
            .checked_add(shipment.amount)
            .ok_or(EconomyError::AmountOverflow)?;
        set_balance(
            &mut self.balances,
            &shipment.destination_node_id,
            shipment.resource.clone(),
            updated,
        );
        self.arrival_by_shipment
            .insert(arrival.shipment_id.clone(), arrival.id.clone());
        self.arrivals.insert(arrival.id.clone(), arrival);
        Ok(())
    }

    /// Explicitly settles an in-transit shipment as lost without crediting destination stock.
    pub fn lose_shipment(&mut self, loss: ShipmentLoss) -> Result<(), EconomyError> {
        if self.losses.contains_key(&loss.id) {
            return Err(EconomyError::DuplicateShipmentLoss(loss.id));
        }
        let shipment = self
            .shipments
            .get(&loss.shipment_id)
            .ok_or_else(|| EconomyError::UnknownShipment(loss.shipment_id.clone()))?;
        if self.arrival_by_shipment.contains_key(&loss.shipment_id)
            || self.loss_by_shipment.contains_key(&loss.shipment_id)
        {
            return Err(EconomyError::ShipmentAlreadySettled(loss.shipment_id));
        }
        if loss.lost_tick < shipment.departed_tick {
            return Err(EconomyError::ShipmentLostBeforeDeparture {
                shipment_id: loss.shipment_id,
                departed_tick: shipment.departed_tick,
                lost_tick: loss.lost_tick,
            });
        }
        if loss.cause.trim().is_empty() {
            return Err(EconomyError::EmptyLossCause(loss.id));
        }
        self.loss_by_shipment
            .insert(loss.shipment_id.clone(), loss.id.clone());
        self.losses.insert(loss.id.clone(), loss);
        Ok(())
    }

    pub fn shipment(&self, shipment_id: &StableId) -> Option<&ShipmentDeparture> {
        self.shipments.get(shipment_id)
    }

    pub fn shipment_state(&self, shipment_id: &StableId) -> Option<ShipmentState> {
        self.shipments.get(shipment_id).map(|_| {
            if self.arrival_by_shipment.contains_key(shipment_id) {
                ShipmentState::Arrived
            } else if self.loss_by_shipment.contains_key(shipment_id) {
                ShipmentState::Lost
            } else {
                ShipmentState::InTransit
            }
        })
    }
}

fn set_balance(
    balances: &mut BTreeMap<(StableId, ResourceKey), u64>,
    node_id: &StableId,
    resource: ResourceKey,
    amount: u64,
) {
    let key = (node_id.clone(), resource);
    if amount == 0 {
        balances.remove(&key);
    } else {
        balances.insert(key, amount);
    }
}

fn validate_recipe(recipe: &ProcessRecipe) -> Result<(), EconomyError> {
    if recipe.inputs.is_empty() || recipe.outputs.is_empty() {
        return Err(EconomyError::FreeOrEmptyRecipe(recipe.id.clone()));
    }
    if recipe.duration_ticks == 0 {
        return Err(EconomyError::ZeroDurationRecipe(recipe.id.clone()));
    }
    if recipe.inputs.values().any(|amount| *amount == 0)
        || recipe.outputs.values().any(|amount| *amount == 0)
    {
        return Err(EconomyError::ZeroRecipeAmount(recipe.id.clone()));
    }
    Ok(())
}

/// Structural/transactional failures in economic flow state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum EconomyError {
    DuplicateNode,
    UnknownNode(StableId),
    DuplicateGenesisBalance,
    ZeroAmount,
    DuplicateRecipe(StableId),
    UnknownRecipe(StableId),
    FreeOrEmptyRecipe(StableId),
    ZeroDurationRecipe(StableId),
    ZeroRecipeAmount(StableId),
    DuplicateProcessRun(StableId),
    UnknownProcessRun(StableId),
    DuplicateProcessCompletion(StableId),
    ProcessAlreadyCompleted(StableId),
    ProcessCompletedTooEarly {
        run_id: StableId,
        earliest_tick: u64,
        attempted_tick: u64,
    },
    ZeroBatches,
    InsufficientBalance {
        node_id: StableId,
        resource: ResourceKey,
        available: u64,
        required: u64,
    },
    AmountOverflow,
    TickOverflow,
    DuplicateShipment(StableId),
    UnknownShipment(StableId),
    SameShipmentEndpoint(StableId),
    ArrivalBeforeDeparture {
        shipment_id: StableId,
        departed_tick: u64,
        earliest_arrival_tick: u64,
    },
    DuplicateShipmentArrival(StableId),
    DuplicateShipmentLoss(StableId),
    ShipmentAlreadySettled(StableId),
    ShipmentArrivedTooEarly {
        shipment_id: StableId,
        earliest_tick: u64,
        attempted_tick: u64,
    },
    ShipmentLostBeforeDeparture {
        shipment_id: StableId,
        departed_tick: u64,
        lost_tick: u64,
    },
    EmptyLossCause(StableId),
}

impl fmt::Display for EconomyError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::DuplicateNode => formatter.write_str("inventory node already exists"),
            Self::UnknownNode(id) => write!(formatter, "unknown inventory node {id}"),
            Self::DuplicateGenesisBalance => formatter.write_str("duplicate genesis node/resource balance"),
            Self::ZeroAmount => formatter.write_str("economic amount must be non-zero"),
            Self::DuplicateRecipe(id) => write!(formatter, "process recipe {id} already exists"),
            Self::UnknownRecipe(id) => write!(formatter, "unknown process recipe {id}"),
            Self::FreeOrEmptyRecipe(id) => write!(formatter, "recipe {id} must have explicit input and output"),
            Self::ZeroDurationRecipe(id) => write!(formatter, "recipe {id} has zero duration"),
            Self::ZeroRecipeAmount(id) => write!(formatter, "recipe {id} contains a zero input/output amount"),
            Self::DuplicateProcessRun(id) => write!(formatter, "process run {id} already exists"),
            Self::UnknownProcessRun(id) => write!(formatter, "unknown process run {id}"),
            Self::DuplicateProcessCompletion(id) => write!(formatter, "process completion {id} already exists"),
            Self::ProcessAlreadyCompleted(id) => write!(formatter, "process run {id} is already completed"),
            Self::ProcessCompletedTooEarly { run_id, earliest_tick, attempted_tick } => write!(
                formatter,
                "process run {run_id} cannot complete before {earliest_tick}; attempted {attempted_tick}"
            ),
            Self::ZeroBatches => formatter.write_str("process run must contain at least one batch"),
            Self::InsufficientBalance { node_id, resource, available, required } => write!(
                formatter,
                "node {node_id} has {available} of {resource:?}, requires {required}"
            ),
            Self::AmountOverflow => formatter.write_str("economic amount overflow"),
            Self::TickOverflow => formatter.write_str("economic tick overflow"),
            Self::DuplicateShipment(id) => write!(formatter, "shipment {id} already exists"),
            Self::UnknownShipment(id) => write!(formatter, "unknown shipment {id}"),
            Self::SameShipmentEndpoint(id) => write!(formatter, "shipment source and destination are both {id}"),
            Self::ArrivalBeforeDeparture { shipment_id, departed_tick, earliest_arrival_tick } => write!(
                formatter,
                "shipment {shipment_id} departs at {departed_tick} but earliest arrival is {earliest_arrival_tick}"
            ),
            Self::DuplicateShipmentArrival(id) => write!(formatter, "shipment arrival {id} already exists"),
            Self::DuplicateShipmentLoss(id) => write!(formatter, "shipment loss {id} already exists"),
            Self::ShipmentAlreadySettled(id) => write!(formatter, "shipment {id} is already arrived/lost"),
            Self::ShipmentArrivedTooEarly { shipment_id, earliest_tick, attempted_tick } => write!(
                formatter,
                "shipment {shipment_id} cannot arrive before {earliest_tick}; attempted {attempted_tick}"
            ),
            Self::ShipmentLostBeforeDeparture { shipment_id, departed_tick, lost_tick } => write!(
                formatter,
                "shipment {shipment_id} departs at {departed_tick} and cannot be lost at {lost_tick}"
            ),
            Self::EmptyLossCause(id) => write!(formatter, "shipment loss {id} has empty cause"),
        }
    }
}

impl Error for EconomyError {}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(value: &str) -> StableId {
        StableId::parse(value).expect("test id is valid")
    }

    fn resource(name: &str) -> ResourceKey {
        ResourceKey {
            resource_id: id(&format!("resource:{name}")),
            unit_id: id("unit:kg"),
        }
    }

    fn world_with_source(amount: u64, key: &ResourceKey) -> EconomyWorld {
        EconomyWorld::from_genesis(
            [
                InventoryNode {
                    id: id("node:source"),
                    location_id: id("place:source"),
                    operator_id: None,
                },
                InventoryNode {
                    id: id("node:destination"),
                    location_id: id("place:destination"),
                    operator_id: None,
                },
            ],
            [GenesisBalance {
                node_id: id("node:source"),
                resource: key.clone(),
                amount,
            }],
        )
        .expect("genesis")
    }

    #[test]
    fn shipment_cargo_exists_at_neither_inventory_while_in_transit() {
        let fuel = resource("fuel");
        let mut world = world_with_source(1_000, &fuel);
        world
            .depart_shipment(ShipmentDeparture {
                id: id("shipment:fuel-1"),
                source_node_id: id("node:source"),
                destination_node_id: id("node:destination"),
                resource: fuel.clone(),
                amount: 300,
                departed_tick: 10,
                earliest_arrival_tick: 20,
                carrier_asset_id: Some(id("asset:freighter-7")),
                source_event_id: id("event:departure"),
            })
            .expect("depart");

        assert_eq!(world.balance(&id("node:source"), &fuel), 700);
        assert_eq!(world.balance(&id("node:destination"), &fuel), 0);
        assert_eq!(
            world.shipment_state(&id("shipment:fuel-1")),
            Some(ShipmentState::InTransit)
        );
        assert!(matches!(
            world.arrive_shipment(ShipmentArrival {
                id: id("arrival:early"),
                shipment_id: id("shipment:fuel-1"),
                arrived_tick: 19,
                source_event_id: id("event:early"),
            }),
            Err(EconomyError::ShipmentArrivedTooEarly { .. })
        ));
        assert_eq!(world.balance(&id("node:destination"), &fuel), 0);

        world
            .arrive_shipment(ShipmentArrival {
                id: id("arrival:fuel-1"),
                shipment_id: id("shipment:fuel-1"),
                arrived_tick: 20,
                source_event_id: id("event:arrival"),
            })
            .expect("arrive");
        assert_eq!(world.balance(&id("node:destination"), &fuel), 300);
        assert_eq!(
            world.shipment_state(&id("shipment:fuel-1")),
            Some(ShipmentState::Arrived)
        );
    }

    #[test]
    fn lost_shipment_never_credits_destination() {
        let food = resource("food");
        let mut world = world_with_source(500, &food);
        world
            .depart_shipment(ShipmentDeparture {
                id: id("shipment:food"),
                source_node_id: id("node:source"),
                destination_node_id: id("node:destination"),
                resource: food.clone(),
                amount: 200,
                departed_tick: 1,
                earliest_arrival_tick: 5,
                carrier_asset_id: None,
                source_event_id: id("event:depart"),
            })
            .expect("depart");
        world
            .lose_shipment(ShipmentLoss {
                id: id("loss:food"),
                shipment_id: id("shipment:food"),
                lost_tick: 3,
                cause: "storm-loss".into(),
                source_event_id: id("event:loss"),
            })
            .expect("loss");
        assert_eq!(world.balance(&id("node:destination"), &food), 0);
        assert_eq!(
            world.shipment_state(&id("shipment:food")),
            Some(ShipmentState::Lost)
        );
        assert!(matches!(
            world.arrive_shipment(ShipmentArrival {
                id: id("arrival:impossible"),
                shipment_id: id("shipment:food"),
                arrived_tick: 10,
                source_event_id: id("event:impossible"),
            }),
            Err(EconomyError::ShipmentAlreadySettled(_))
        ));
    }

    #[test]
    fn production_consumes_inputs_before_outputs_exist() {
        let ore = resource("ore");
        let steel = resource("steel");
        let mut world = EconomyWorld::from_genesis(
            [InventoryNode {
                id: id("node:mill"),
                location_id: id("place:mill"),
                operator_id: None,
            }],
            [GenesisBalance {
                node_id: id("node:mill"),
                resource: ore.clone(),
                amount: 10,
            }],
        )
        .expect("genesis");
        world
            .register_recipe(ProcessRecipe {
                id: id("recipe:steel"),
                inputs: BTreeMap::from([(ore.clone(), 2)]),
                outputs: BTreeMap::from([(steel.clone(), 1)]),
                duration_ticks: 5,
                facility_class_id: Some(id("facility:mill")),
                source_event_id: id("event:recipe"),
            })
            .expect("recipe");
        world
            .start_process(ProcessRun {
                id: id("run:steel"),
                recipe_id: id("recipe:steel"),
                node_id: id("node:mill"),
                batches: 3,
                started_tick: 10,
                source_event_id: id("event:start"),
            })
            .expect("start");

        assert_eq!(world.balance(&id("node:mill"), &ore), 4);
        assert_eq!(world.balance(&id("node:mill"), &steel), 0);
        assert!(matches!(
            world.complete_process(ProcessCompletion {
                id: id("completion:early"),
                run_id: id("run:steel"),
                completed_tick: 14,
                source_event_id: id("event:early"),
            }),
            Err(EconomyError::ProcessCompletedTooEarly { .. })
        ));
        assert_eq!(world.balance(&id("node:mill"), &steel), 0);

        world
            .complete_process(ProcessCompletion {
                id: id("completion:steel"),
                run_id: id("run:steel"),
                completed_tick: 15,
                source_event_id: id("event:complete"),
            })
            .expect("complete");
        assert_eq!(world.balance(&id("node:mill"), &steel), 3);
    }

    #[test]
    fn insufficient_process_input_is_atomic() {
        let ore = resource("ore");
        let steel = resource("steel");
        let mut world = EconomyWorld::from_genesis(
            [InventoryNode {
                id: id("node:mill"),
                location_id: id("place:mill"),
                operator_id: None,
            }],
            [GenesisBalance {
                node_id: id("node:mill"),
                resource: ore.clone(),
                amount: 3,
            }],
        )
        .expect("genesis");
        world
            .register_recipe(ProcessRecipe {
                id: id("recipe:steel"),
                inputs: BTreeMap::from([(ore.clone(), 2)]),
                outputs: BTreeMap::from([(steel, 1)]),
                duration_ticks: 5,
                facility_class_id: None,
                source_event_id: id("event:recipe"),
            })
            .expect("recipe");
        assert!(matches!(
            world.start_process(ProcessRun {
                id: id("run:too-large"),
                recipe_id: id("recipe:steel"),
                node_id: id("node:mill"),
                batches: 2,
                started_tick: 1,
                source_event_id: id("event:start"),
            }),
            Err(EconomyError::InsufficientBalance { .. })
        ));
        assert_eq!(world.balance(&id("node:mill"), &ore), 3);
        assert!(world.process_run(&id("run:too-large")).is_none());
    }

    #[test]
    fn free_output_recipe_is_rejected() {
        let steel = resource("steel");
        let mut world = EconomyWorld::from_genesis(
            [InventoryNode {
                id: id("node:mill"),
                location_id: id("place:mill"),
                operator_id: None,
            }],
            [],
        )
        .expect("genesis");
        assert!(matches!(
            world.register_recipe(ProcessRecipe {
                id: id("recipe:free-steel"),
                inputs: BTreeMap::new(),
                outputs: BTreeMap::from([(steel, 1)]),
                duration_ticks: 1,
                facility_class_id: None,
                source_event_id: id("event:bad-recipe"),
            }),
            Err(EconomyError::FreeOrEmptyRecipe(_))
        ));
    }
}
