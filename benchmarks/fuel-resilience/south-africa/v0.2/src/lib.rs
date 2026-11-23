#![forbid(unsafe_code)]

//! SAFRB v0.2: grade-aware, time-expanded fuel-network accounting oracle.
//!
//! This crate evaluates a supplied dispatch plan. It is deliberately not an
//! optimizer, hydraulic solver, live-data connector, or operational controller.

use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Grade {
    D50,
    D500,
}

impl Grade {
    pub const ALL: [Self; 2] = [Self::D50, Self::D500];

    pub const fn index(self) -> usize {
        match self {
            Self::D50 => 0,
            Self::D500 => 1,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvidenceClass {
    Measured,
    Reported,
    Derived,
    Forecast,
    Scenario,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Evidence {
    pub source_id: String,
    pub source_uri: String,
    pub evidence_class: EvidenceClass,
    pub raw_sha256: Option<String>,
}

impl Evidence {
    pub fn validate(&self) -> Result<(), String> {
        if self.source_id.trim().is_empty() {
            return Err("evidence source_id must not be empty".into());
        }
        if self.source_uri.trim().is_empty() {
            return Err("evidence source_uri must not be empty".into());
        }
        if let Some(digest) = &self.raw_sha256 {
            if digest.len() != 64 || !digest.bytes().all(|b| b.is_ascii_hexdigit()) {
                return Err("raw_sha256 must be exactly 64 hexadecimal characters".into());
            }
        }
        Ok(())
    }

    /// Structural check only: a measured claim must carry a SHA-256-shaped
    /// reference. This does not verify raw bytes, a signature, instrument
    /// calibration, sampling location or the physical truth of the claim.
    pub fn is_measured_and_digest_bound(&self) -> bool {
        self.evidence_class == EvidenceClass::Measured
            && self.raw_sha256.as_ref().is_some_and(|d| {
                d.len() == 64 && d.bytes().all(|b| b.is_ascii_hexdigit())
            })
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StockObservation {
    pub observation_id: String,
    pub grade: Grade,
    pub volume_l: u64,
    pub evidence: Evidence,
    pub quality_released: bool,
    pub physically_accessible: bool,
}

impl StockObservation {
    pub fn validate(&self) -> Result<(), String> {
        if self.observation_id.trim().is_empty() {
            return Err("observation_id must not be empty".into());
        }
        self.evidence.validate()
    }

    /// A volume is usable only when measurement, provenance digest, product
    /// release and physical accessibility are all present.
    pub fn qualifies_as_usable(&self) -> bool {
        self.evidence.is_measured_and_digest_bound()
            && self.quality_released
            && self.physically_accessible
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NodeKind {
    Port,
    Refinery,
    CoastalTerminal,
    PipelineStation,
    InlandTerminal,
    DemandZone,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Node {
    pub id: String,
    pub kind: NodeKind,
    /// Usable tank capacity by grade. D50 and D500 are never pooled.
    pub capacity_l: [u64; 2],
    pub daily_production_l: [u64; 2],
    pub daily_demand_l: [u64; 2],
    pub opening_observations: Vec<StockObservation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Edge {
    pub id: String,
    pub from: String,
    pub to: String,
    /// Product-specific daily transfer capacity.
    pub capacity_l_per_day: [u64; 2],
    pub grade_allowed: [bool; 2],
    /// Minimum one-day transit is required in v0.2. A future version may
    /// support within-day transfer events with an explicit event ordering.
    pub transit_days: u16,
    pub transit_loss_bps: u16,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CargoMilestone {
    Nominated,
    Loaded,
    Departed,
    InTransit,
    Arrived,
    Discharged,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CargoEvent {
    pub event_id: String,
    pub day: u16,
    pub destination_node: String,
    pub grade: Grade,
    pub volume_l: u64,
    pub milestone: CargoMilestone,
    pub evidence: Evidence,
    pub quality_released: bool,
    pub physically_accessible: bool,
}

impl CargoEvent {
    /// Accept a discharged event as an input to the simulation. Operational
    /// measured events need a digest reference. Synthetic scenario receipts
    /// are accepted only under the explicit fixture namespace; they are never
    /// opening "verified usable" inventory.
    pub fn accepted_as_simulated_receipt(&self) -> bool {
        let evidence_allowed = match self.evidence.evidence_class {
            EvidenceClass::Measured => self.evidence.is_measured_and_digest_bound(),
            EvidenceClass::Scenario => self.evidence.source_uri.starts_with("urn:safrb:fixture:"),
            EvidenceClass::Reported | EvidenceClass::Derived | EvidenceClass::Forecast => false,
        };
        self.milestone == CargoMilestone::Discharged
            && evidence_allowed
            && self.quality_released
            && self.physically_accessible
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Dispatch {
    /// Zero-based simulation day.
    pub day: u16,
    pub edge_id: String,
    pub grade: Grade,
    pub volume_l: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Model {
    pub scenario_id: String,
    pub horizon_days: u16,
    pub nodes: Vec<Node>,
    pub edges: Vec<Edge>,
    pub cargo_events: Vec<CargoEvent>,
    /// A planner supplies this schedule. The simulator verifies it; it does
    /// not silently repair infeasible dispatches.
    pub dispatches: Vec<Dispatch>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct NodeDayResult {
    pub day: u16,
    pub node_id: String,
    pub grade: Grade,
    pub opening_l: u64,
    pub arrival_l: u64,
    pub cargo_receipt_l: u64,
    pub production_l: u64,
    pub demand_l: u64,
    pub delivered_l: u64,
    pub unserved_l: u64,
    pub dispatch_l: u64,
    pub closing_l: u64,
    pub balance_residual_l: i128,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ResultSummary {
    pub scenario_id: String,
    pub node_days: Vec<NodeDayResult>,
    pub opening_inventory_l: u128,
    pub accepted_cargo_receipts_l: u128,
    pub domestic_production_l: u128,
    pub delivered_demand_l: u128,
    pub unserved_demand_l: u128,
    pub transit_loss_l: u128,
    pub closing_node_inventory_l: u128,
    pub closing_in_transit_l: u128,
    pub excluded_cargo_volume_l: u128,
    pub network_balance_residual_l: i128,
}

#[derive(Debug, Clone)]
struct PendingArrival {
    arrival_day: u16,
    node_index: usize,
    grade: Grade,
    volume_l: u64,
}

fn add_checked(target: &mut u64, value: u64, context: &str) -> Result<(), String> {
    *target = target
        .checked_add(value)
        .ok_or_else(|| format!("{context} overflowed u64"))?;
    Ok(())
}

impl Model {
    pub fn validate(&self) -> Result<(), String> {
        if self.scenario_id.trim().is_empty() {
            return Err("scenario_id must not be empty".into());
        }
        if self.horizon_days == 0 {
            return Err("horizon_days must be greater than zero".into());
        }
        if self.nodes.is_empty() {
            return Err("network must contain at least one node".into());
        }

        let mut node_ids = BTreeSet::new();
        let mut observation_ids = BTreeSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() {
                return Err("node id must not be empty".into());
            }
            if !node_ids.insert(node.id.as_str()) {
                return Err(format!("duplicate node id: {}", node.id));
            }

            let mut opening = [0_u64; 2];
            for observation in &node.opening_observations {
                observation.validate()?;
                if !observation_ids.insert(observation.observation_id.as_str()) {
                    return Err(format!(
                        "duplicate inventory observation id: {}",
                        observation.observation_id
                    ));
                }
                if observation.qualifies_as_usable() {
                    add_checked(
                        &mut opening[observation.grade.index()],
                        observation.volume_l,
                        "opening inventory",
                    )?;
                }
            }
            for grade in Grade::ALL {
                let index = grade.index();
                if opening[index] > node.capacity_l[index] {
                    return Err(format!(
                        "verified opening {grade:?} inventory exceeds capacity at {}",
                        node.id
                    ));
                }
            }
        }

        let mut edge_ids = BTreeSet::new();
        let mut edge_by_id = BTreeMap::new();
        for (index, edge) in self.edges.iter().enumerate() {
            if edge.id.trim().is_empty() {
                return Err("edge id must not be empty".into());
            }
            if !edge_ids.insert(edge.id.as_str()) {
                return Err(format!("duplicate edge id: {}", edge.id));
            }
            if !node_ids.contains(edge.from.as_str()) || !node_ids.contains(edge.to.as_str()) {
                return Err(format!("edge {} references an unknown node", edge.id));
            }
            if edge.from == edge.to {
                return Err(format!("edge {} must not be a self-loop", edge.id));
            }
            if edge.transit_days == 0 {
                return Err(format!("edge {} transit_days must be at least one", edge.id));
            }
            if edge.transit_loss_bps > 10_000 {
                return Err(format!("edge {} transit_loss_bps must be in 0..=10000", edge.id));
            }
            if !edge.grade_allowed.iter().any(|allowed| *allowed) {
                return Err(format!("edge {} allows no product grade", edge.id));
            }
            edge_by_id.insert(edge.id.as_str(), index);
        }

        let mut cargo_ids = BTreeSet::new();
        for event in &self.cargo_events {
            if event.event_id.trim().is_empty() {
                return Err("cargo event_id must not be empty".into());
            }
            if !cargo_ids.insert(event.event_id.as_str()) {
                return Err(format!("duplicate cargo event_id: {}", event.event_id));
            }
            if event.day >= self.horizon_days {
                return Err(format!("cargo event {} is outside the horizon", event.event_id));
            }
            if !node_ids.contains(event.destination_node.as_str()) {
                return Err(format!("cargo event {} has unknown destination", event.event_id));
            }
            event.evidence.validate()?;
        }

        let mut edge_day_grade = BTreeMap::<(u16, String, Grade), u64>::new();
        for dispatch in &self.dispatches {
            if dispatch.day >= self.horizon_days {
                return Err(format!("dispatch on day {} is outside the horizon", dispatch.day));
            }
            let Some(edge_index) = edge_by_id.get(dispatch.edge_id.as_str()) else {
                return Err(format!("dispatch references unknown edge: {}", dispatch.edge_id));
            };
            let edge = &self.edges[*edge_index];
            if !edge.grade_allowed[dispatch.grade.index()] {
                return Err(format!(
                    "edge {} does not support {:?}",
                    edge.id, dispatch.grade
                ));
            }
            let key = (dispatch.day, dispatch.edge_id.clone(), dispatch.grade);
            let total = edge_day_grade.entry(key).or_insert(0);
            add_checked(total, dispatch.volume_l, "edge/day/grade dispatch")?;
            if *total > edge.capacity_l_per_day[dispatch.grade.index()] {
                return Err(format!(
                    "dispatch exceeds {:?} daily capacity on edge {} day {}",
                    dispatch.grade, edge.id, dispatch.day
                ));
            }
        }

        Ok(())
    }

    pub fn simulate(&self) -> Result<ResultSummary, String> {
        self.validate()?;

        let node_index: BTreeMap<&str, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(index, node)| (node.id.as_str(), index))
            .collect();
        let edge_index: BTreeMap<&str, usize> = self
            .edges
            .iter()
            .enumerate()
            .map(|(index, edge)| (edge.id.as_str(), index))
            .collect();

        let mut stock = vec![[0_u64; 2]; self.nodes.len()];
        let mut opening_inventory_l = 0_u128;
        for (index, node) in self.nodes.iter().enumerate() {
            for observation in &node.opening_observations {
                if observation.qualifies_as_usable() {
                    add_checked(
                        &mut stock[index][observation.grade.index()],
                        observation.volume_l,
                        "opening stock",
                    )?;
                    opening_inventory_l += u128::from(observation.volume_l);
                }
            }
        }

        let mut dispatch_by_day = vec![Vec::<&Dispatch>::new(); usize::from(self.horizon_days)];
        for dispatch in &self.dispatches {
            dispatch_by_day[usize::from(dispatch.day)].push(dispatch);
        }
        let mut cargo_by_day = vec![Vec::<&CargoEvent>::new(); usize::from(self.horizon_days)];
        for event in &self.cargo_events {
            cargo_by_day[usize::from(event.day)].push(event);
        }

        let mut pending = Vec::<PendingArrival>::new();
        let mut node_days = Vec::new();
        let mut accepted_cargo_receipts_l = 0_u128;
        let mut domestic_production_l = 0_u128;
        let mut delivered_demand_l = 0_u128;
        let mut unserved_demand_l = 0_u128;
        let mut transit_loss_l = 0_u128;
        let mut excluded_cargo_volume_l = 0_u128;

        for day in 0..self.horizon_days {
            let opening = stock.clone();
            let mut arrivals = vec![[0_u64; 2]; self.nodes.len()];
            let mut cargo_receipts = vec![[0_u64; 2]; self.nodes.len()];
            let mut production = vec![[0_u64; 2]; self.nodes.len()];
            let mut delivered = vec![[0_u64; 2]; self.nodes.len()];
            let mut unserved = vec![[0_u64; 2]; self.nodes.len()];
            let mut dispatched = vec![[0_u64; 2]; self.nodes.len()];

            let mut future = Vec::new();
            for arrival in pending.drain(..) {
                if arrival.arrival_day == day {
                    let grade_index = arrival.grade.index();
                    add_checked(
                        &mut stock[arrival.node_index][grade_index],
                        arrival.volume_l,
                        "arriving inventory",
                    )?;
                    add_checked(
                        &mut arrivals[arrival.node_index][grade_index],
                        arrival.volume_l,
                        "daily arrivals",
                    )?;
                } else {
                    future.push(arrival);
                }
            }
            pending = future;

            for event in &cargo_by_day[usize::from(day)] {
                if event.accepted_as_simulated_receipt() {
                    let index = node_index[event.destination_node.as_str()];
                    let grade_index = event.grade.index();
                    add_checked(
                        &mut stock[index][grade_index],
                        event.volume_l,
                        "accepted cargo receipt",
                    )?;
                    add_checked(
                        &mut cargo_receipts[index][grade_index],
                        event.volume_l,
                        "daily accepted cargo receipt",
                    )?;
                    accepted_cargo_receipts_l += u128::from(event.volume_l);
                } else {
                    excluded_cargo_volume_l += u128::from(event.volume_l);
                }
            }

            // Arrival and cargo receipts also consume physical tank capacity.
            // Check immediately, before satisfying demand or dispatching stock.
            for (index, node) in self.nodes.iter().enumerate() {
                for grade in Grade::ALL {
                    let grade_index = grade.index();
                    if stock[index][grade_index] > node.capacity_l[grade_index] {
                        return Err(format!(
                            "tank capacity exceeded by receipts at node {} grade {:?} day {}",
                            node.id,
                            grade,
                            day + 1
                        ));
                    }
                }
            }

            for (index, node) in self.nodes.iter().enumerate() {
                for grade in Grade::ALL {
                    let grade_index = grade.index();
                    let volume = node.daily_production_l[grade_index];
                    add_checked(
                        &mut stock[index][grade_index],
                        volume,
                        "domestic production",
                    )?;
                    production[index][grade_index] = volume;
                    domestic_production_l += u128::from(volume);
                    if stock[index][grade_index] > node.capacity_l[grade_index] {
                        return Err(format!(
                            "tank capacity exceeded at node {} grade {:?} day {}",
                            node.id,
                            grade,
                            day + 1
                        ));
                    }
                }
            }

            // Serve the node's own demand before dispatching its remaining stock.
            for (index, node) in self.nodes.iter().enumerate() {
                for grade in Grade::ALL {
                    let grade_index = grade.index();
                    let demand = node.daily_demand_l[grade_index];
                    let served = stock[index][grade_index].min(demand);
                    stock[index][grade_index] -= served;
                    delivered[index][grade_index] = served;
                    unserved[index][grade_index] = demand - served;
                    delivered_demand_l += u128::from(served);
                    unserved_demand_l += u128::from(demand - served);
                }
            }

            for dispatch in &dispatch_by_day[usize::from(day)] {
                let edge = &self.edges[edge_index[dispatch.edge_id.as_str()]];
                let source = node_index[edge.from.as_str()];
                let grade_index = dispatch.grade.index();
                if stock[source][grade_index] < dispatch.volume_l {
                    return Err(format!(
                        "infeasible dispatch: node {} lacks {:?} stock on day {} for edge {}",
                        edge.from,
                        dispatch.grade,
                        day + 1,
                        edge.id
                    ));
                }
                stock[source][grade_index] -= dispatch.volume_l;
                add_checked(
                    &mut dispatched[source][grade_index],
                    dispatch.volume_l,
                    "daily node dispatch",
                )?;

                let loss = (u128::from(dispatch.volume_l)
                    * u128::from(edge.transit_loss_bps)
                    / 10_000) as u64;
                let delivered_volume = dispatch.volume_l - loss;
                transit_loss_l += u128::from(loss);
                let arrival_day = day
                    .checked_add(edge.transit_days)
                    .ok_or_else(|| "arrival day overflowed u16".to_string())?;
                pending.push(PendingArrival {
                    arrival_day,
                    node_index: node_index[edge.to.as_str()],
                    grade: dispatch.grade,
                    volume_l: delivered_volume,
                });
            }

            for (index, node) in self.nodes.iter().enumerate() {
                for grade in Grade::ALL {
                    let grade_index = grade.index();
                    let residual = i128::from(opening[index][grade_index])
                        + i128::from(arrivals[index][grade_index])
                        + i128::from(cargo_receipts[index][grade_index])
                        + i128::from(production[index][grade_index])
                        - i128::from(delivered[index][grade_index])
                        - i128::from(dispatched[index][grade_index])
                        - i128::from(stock[index][grade_index]);

                    node_days.push(NodeDayResult {
                        day: day + 1,
                        node_id: node.id.clone(),
                        grade,
                        opening_l: opening[index][grade_index],
                        arrival_l: arrivals[index][grade_index],
                        cargo_receipt_l: cargo_receipts[index][grade_index],
                        production_l: production[index][grade_index],
                        demand_l: node.daily_demand_l[grade_index],
                        delivered_l: delivered[index][grade_index],
                        unserved_l: unserved[index][grade_index],
                        dispatch_l: dispatched[index][grade_index],
                        closing_l: stock[index][grade_index],
                        balance_residual_l: residual,
                    });
                }
            }
        }

        let closing_node_inventory_l: u128 = stock
            .iter()
            .flat_map(|by_grade| by_grade.iter())
            .map(|volume| u128::from(*volume))
            .sum();
        let closing_in_transit_l: u128 = pending
            .iter()
            .map(|arrival| u128::from(arrival.volume_l))
            .sum();
        let network_inputs = opening_inventory_l
            + accepted_cargo_receipts_l
            + domestic_production_l;
        let network_outputs = delivered_demand_l
            + transit_loss_l
            + closing_node_inventory_l
            + closing_in_transit_l;
        let network_balance_residual_l = i128::try_from(network_inputs)
            .map_err(|_| "network input total exceeds i128".to_string())?
            - i128::try_from(network_outputs)
                .map_err(|_| "network output total exceeds i128".to_string())?;

        if network_balance_residual_l != 0 {
            return Err(format!(
                "network conservation failed: residual {} L",
                network_balance_residual_l
            ));
        }

        Ok(ResultSummary {
            scenario_id: self.scenario_id.clone(),
            node_days,
            opening_inventory_l,
            accepted_cargo_receipts_l,
            domestic_production_l,
            delivered_demand_l,
            unserved_demand_l,
            transit_loss_l,
            closing_node_inventory_l,
            closing_in_transit_l,
            excluded_cargo_volume_l,
            network_balance_residual_l,
        })
    }
}

/// Small, synthetic chain used by the executable known-answer case.
/// Volumes and capacities are arbitrary mathematical fixtures, not South
/// African infrastructure measurements.
pub fn base_case() -> Model {
    let nodes = vec![
        Node {
            id: "port".into(),
            kind: NodeKind::Port,
            capacity_l: [1_000, 1_000],
            daily_production_l: [0, 0],
            daily_demand_l: [0, 0],
            opening_observations: vec![],
        },
        Node {
            id: "coastal-terminal".into(),
            kind: NodeKind::CoastalTerminal,
            capacity_l: [1_000, 1_000],
            daily_production_l: [0, 0],
            daily_demand_l: [0, 0],
            opening_observations: vec![],
        },
        Node {
            id: "inland-demand-zone".into(),
            kind: NodeKind::DemandZone,
            capacity_l: [1_000, 1_000],
            daily_production_l: [0, 0],
            daily_demand_l: [10, 0],
            opening_observations: vec![],
        },
    ];
    let edges = vec![
        Edge {
            id: "port-to-terminal".into(),
            from: "port".into(),
            to: "coastal-terminal".into(),
            capacity_l_per_day: [100, 0],
            grade_allowed: [true, false],
            transit_days: 1,
            transit_loss_bps: 0,
        },
        Edge {
            id: "terminal-to-inland".into(),
            from: "coastal-terminal".into(),
            to: "inland-demand-zone".into(),
            capacity_l_per_day: [100, 0],
            grade_allowed: [true, false],
            transit_days: 1,
            transit_loss_bps: 0,
        },
    ];
    let cargo_events = vec![CargoEvent {
        event_id: "cargo-1".into(),
        day: 0,
        destination_node: "port".into(),
        grade: Grade::D50,
        volume_l: 100,
        milestone: CargoMilestone::Discharged,
        evidence: Evidence {
            source_id: "synthetic-fixture".into(),
            source_uri: "urn:safrb:fixture:cargo-1".into(),
            evidence_class: EvidenceClass::Scenario,
            raw_sha256: None,
        },
        quality_released: true,
        physically_accessible: true,
    }];
    let dispatches = vec![
        Dispatch {
            day: 0,
            edge_id: "port-to-terminal".into(),
            grade: Grade::D50,
            volume_l: 100,
        },
        Dispatch {
            day: 1,
            edge_id: "terminal-to-inland".into(),
            grade: Grade::D50,
            volume_l: 80,
        },
    ];
    Model {
        scenario_id: "synthetic-three-node-chain".into(),
        horizon_days: 3,
        nodes,
        edges,
        cargo_events,
        dispatches,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured_evidence() -> Evidence {
        Evidence {
            source_id: "test-meter".into(),
            source_uri: "urn:test:meter:1".into(),
            evidence_class: EvidenceClass::Measured,
            raw_sha256: Some("a".repeat(64)),
        }
    }

    fn verified_stock(id: &str, grade: Grade, volume_l: u64) -> StockObservation {
        StockObservation {
            observation_id: id.into(),
            grade,
            volume_l,
            evidence: measured_evidence(),
            quality_released: true,
            physically_accessible: true,
        }
    }

    fn qualified_cargo(id: &str, day: u16, node: &str, volume_l: u64) -> CargoEvent {
        CargoEvent {
            event_id: id.into(),
            day,
            destination_node: node.into(),
            grade: Grade::D50,
            volume_l,
            milestone: CargoMilestone::Discharged,
            evidence: measured_evidence(),
            quality_released: true,
            physically_accessible: true,
        }
    }

    fn base_model() -> Model {
        let nodes = vec![
            Node {
                id: "port".into(),
                kind: NodeKind::Port,
                capacity_l: [1_000, 1_000],
                daily_production_l: [0, 0],
                daily_demand_l: [0, 0],
                opening_observations: vec![],
            },
            Node {
                id: "coastal-terminal".into(),
                kind: NodeKind::CoastalTerminal,
                capacity_l: [1_000, 1_000],
                daily_production_l: [0, 0],
                daily_demand_l: [0, 0],
                opening_observations: vec![],
            },
            Node {
                id: "inland-demand-zone".into(),
                kind: NodeKind::DemandZone,
                capacity_l: [1_000, 1_000],
                daily_production_l: [0, 0],
                daily_demand_l: [10, 0],
                opening_observations: vec![],
            },
        ];
        let edges = vec![
            Edge {
                id: "port-to-terminal".into(),
                from: "port".into(),
                to: "coastal-terminal".into(),
                capacity_l_per_day: [100, 0],
                grade_allowed: [true, false],
                transit_days: 1,
                transit_loss_bps: 0,
            },
            Edge {
                id: "terminal-to-inland".into(),
                from: "coastal-terminal".into(),
                to: "inland-demand-zone".into(),
                capacity_l_per_day: [100, 0],
                grade_allowed: [true, false],
                transit_days: 1,
                transit_loss_bps: 0,
            },
        ];
        Model {
            scenario_id: "synthetic-three-node-chain".into(),
            horizon_days: 3,
            nodes,
            edges,
            cargo_events: vec![qualified_cargo("cargo-1", 0, "port", 100)],
            dispatches: vec![
                Dispatch {
                    day: 0,
                    edge_id: "port-to-terminal".into(),
                    grade: Grade::D50,
                    volume_l: 100,
                },
                Dispatch {
                    day: 1,
                    edge_id: "terminal-to-inland".into(),
                    grade: Grade::D50,
                    volume_l: 80,
                },
            ],
        }
    }

    #[test]
    fn only_measured_digest_bound_released_accessible_stock_is_usable() {
        let mut node = base_model().nodes.remove(0);
        node.opening_observations = vec![
            verified_stock("good", Grade::D50, 100),
            StockObservation {
                observation_id: "reported".into(),
                grade: Grade::D50,
                volume_l: 200,
                evidence: Evidence {
                    source_id: "monthly-report".into(),
                    source_uri: "urn:test:report".into(),
                    evidence_class: EvidenceClass::Reported,
                    raw_sha256: None,
                },
                quality_released: true,
                physically_accessible: true,
            },
            StockObservation {
                observation_id: "off-spec".into(),
                grade: Grade::D50,
                volume_l: 300,
                evidence: measured_evidence(),
                quality_released: false,
                physically_accessible: true,
            },
        ];
        let mut model = base_model();
        model.nodes[0] = node;
        model.cargo_events.clear();
        model.dispatches.clear();

        let result = model.simulate().unwrap();
        assert_eq!(result.opening_inventory_l, 100);
        assert_eq!(result.closing_node_inventory_l, 100);
        assert_eq!(result.network_balance_residual_l, 0);
    }

    #[test]
    fn discharged_qualified_cargo_is_counted_but_arrived_cargo_is_not() {
        let mut model = base_model();
        model.cargo_events.push(CargoEvent {
            event_id: "not-discharged".into(),
            day: 0,
            destination_node: "port".into(),
            grade: Grade::D50,
            volume_l: 500,
            milestone: CargoMilestone::Arrived,
            evidence: measured_evidence(),
            quality_released: true,
            physically_accessible: true,
        });
        let result = model.simulate().unwrap();
        assert_eq!(result.accepted_cargo_receipts_l, 100);
        assert_eq!(result.excluded_cargo_volume_l, 500);
    }

    #[test]
    fn time_expanded_transit_and_node_balances_conserve_mass() {
        let result = base_model().simulate().unwrap();
        assert_eq!(result.network_balance_residual_l, 0);
        assert!(result.node_days.iter().all(|day| day.balance_residual_l == 0));
        assert_eq!(result.delivered_demand_l, 10);
        assert_eq!(result.unserved_demand_l, 20);
        assert_eq!(result.closing_node_inventory_l, 90);
        assert_eq!(result.closing_in_transit_l, 0);
    }

    #[test]
    fn grade_incompatibility_is_rejected() {
        let mut model = base_model();
        model.dispatches[0].grade = Grade::D500;
        assert!(model.validate().is_err());
    }

    #[test]
    fn dispatch_cannot_exceed_available_stock() {
        let mut model = base_model();
        model.dispatches[0].volume_l = 101;
        model.edges[0].capacity_l_per_day[0] = 1_000;
        assert!(model.simulate().is_err());
    }

    #[test]
    fn d50_and_d500_inventory_are_not_pooled() {
        let mut model = base_model();
        model.nodes[0].opening_observations = vec![
            verified_stock("d50", Grade::D50, 60),
            verified_stock("d500", Grade::D500, 40),
        ];
        model.cargo_events.clear();
        model.dispatches.clear();
        let result = model.simulate().unwrap();
        assert_eq!(result.opening_inventory_l, 100);
        assert_eq!(result.closing_node_inventory_l, 100);
        assert_eq!(result.node_days.iter().filter(|r| r.grade == Grade::D50).map(|r| r.closing_l).sum::<u64>(), 60);
        assert_eq!(result.node_days.iter().filter(|r| r.grade == Grade::D500).map(|r| r.closing_l).sum::<u64>(), 40);
    }

    #[test]
    fn destination_tank_overflow_fails_closed() {
        let mut model = base_model();
        model.nodes[1].capacity_l[0] = 50;
        assert!(model.simulate().is_err());
    }

    #[test]
    fn duplicate_cargo_ids_are_rejected() {
        let mut model = base_model();
        let duplicate = model.cargo_events[0].clone();
        model.cargo_events.push(duplicate);
        assert!(model.validate().is_err());
    }

    #[test]
    fn duplicate_opening_observation_ids_are_rejected() {
        let mut model = base_model();
        model.nodes[0].opening_observations = vec![
            verified_stock("same-id", Grade::D50, 10),
            verified_stock("same-id", Grade::D500, 10),
        ];
        assert!(model.validate().is_err());
    }
}
