#![forbid(unsafe_code)]

//! SAFRB v0.3: deterministic time-expanded min-cost-flow allocation baseline.
//!
//! One product grade is solved per run. This is a conventional optimization
//! baseline, not a robust/stochastic optimizer, hydraulic model, or controller.

use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Grade {
    D50,
    D500,
}

impl Grade {
    const fn index(self) -> usize {
        match self {
            Self::D50 => 0,
            Self::D500 => 1,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SupplyEvidence {
    /// Caller asserts that an upstream evidence gate has qualified the stock.
    /// This planner does not independently verify the assertion.
    VerifiedUsable,
    /// Synthetic counterfactual supply; stays labelled as scenario data.
    Scenario,
    /// Reported and forecast quantities are excluded from firm allocation.
    Reported,
    Forecast,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannerNode {
    pub id: String,
    pub storage_capacity_l: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlannerEdge {
    pub id: String,
    pub from: String,
    pub to: String,
    pub daily_capacity_l: u64,
    pub transit_days: u16,
    /// Abstract integer cost points per litre, not currency unless a sourced
    /// tariff and unit basis are explicitly supplied by the caller.
    pub cost_per_litre: u64,
    pub grade_allowed: [bool; 2],
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Supply {
    pub id: String,
    pub node_id: String,
    pub available_day: u16,
    pub volume_l: u64,
    pub evidence: SupplyEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Demand {
    pub id: String,
    pub node_id: String,
    pub due_day: u16,
    pub quantity_l: u64,
    /// Lower numeric values mean higher service priority.
    pub priority: u16,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanningProblem {
    pub scenario_id: String,
    pub grade: Grade,
    pub horizon_days: u16,
    pub nodes: Vec<PlannerNode>,
    pub edges: Vec<PlannerEdge>,
    pub supplies: Vec<Supply>,
    pub demands: Vec<Demand>,
    pub holding_cost_per_litre_day: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DemandAllocation {
    pub demand_id: String,
    pub node_id: String,
    pub due_day: u16,
    pub priority: u16,
    pub requested_l: u64,
    pub delivered_l: u64,
    pub unserved_l: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct RouteDispatch {
    pub edge_id: String,
    pub departure_day: u16,
    pub arrival_day: u16,
    pub grade: Grade,
    pub volume_l: u64,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SupplyUse {
    pub supply_id: String,
    pub node_id: String,
    pub available_day: u16,
    pub available_l: u64,
    pub allocated_l: u64,
    pub unallocated_l: u64,
    pub evidence: SupplyEvidence,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlanResult {
    pub scenario_id: String,
    pub grade: Grade,
    pub horizon_days: u16,
    pub total_requested_l: u128,
    pub total_delivered_l: u128,
    pub total_unserved_l: u128,
    pub unallocated_eligible_supply_l: u128,
    pub excluded_supply_l: u128,
    /// Priority cumulative components (highest to lowest), then abstract
    /// logistics/holding cost. This is a lexicographic objective receipt.
    pub objective_components: Vec<i128>,
    pub demand_allocations: Vec<DemandAllocation>,
    pub dispatches: Vec<RouteDispatch>,
    pub supply_uses: Vec<SupplyUse>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct Cost(Vec<i128>);

impl Cost {
    fn zero(dimensions: usize) -> Self {
        Self(vec![0; dimensions])
    }

    fn checked_add(&self, rhs: &Self) -> Result<Self, String> {
        if self.0.len() != rhs.0.len() {
            return Err("internal cost-vector dimension mismatch".into());
        }
        self.0
            .iter()
            .zip(&rhs.0)
            .map(|(a, b)| {
                a.checked_add(*b)
                    .ok_or_else(|| "objective cost overflowed i128".to_string())
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }

    fn checked_neg(&self) -> Result<Self, String> {
        self.0
            .iter()
            .map(|value| {
                value
                    .checked_neg()
                    .ok_or_else(|| "residual cost overflowed i128".to_string())
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }

    fn checked_scale(&self, factor: u64) -> Result<Self, String> {
        self.0
            .iter()
            .map(|value| {
                value
                    .checked_mul(i128::from(factor))
                    .ok_or_else(|| "scaled objective cost overflowed i128".to_string())
            })
            .collect::<Result<Vec<_>, _>>()
            .map(Self)
    }
}

impl Ord for Cost {
    fn cmp(&self, other: &Self) -> Ordering {
        self.0.cmp(&other.0)
    }
}
impl PartialOrd for Cost {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

#[derive(Clone, Debug)]
struct ResidualEdge {
    to: usize,
    reverse_index: usize,
    residual_l: u64,
    cost: Cost,
}

#[derive(Clone, Copy, Debug)]
struct ArcRef {
    from: usize,
    edge_index: usize,
    initial_l: u64,
}

impl ArcRef {
    fn flow_l(self, graph: &[Vec<ResidualEdge>]) -> Result<u64, String> {
        let edge = graph
            .get(self.from)
            .and_then(|edges| edges.get(self.edge_index))
            .ok_or_else(|| "internal arc reference is invalid".to_string())?;
        self.initial_l
            .checked_sub(edge.residual_l)
            .ok_or_else(|| "internal residual capacity exceeds initial capacity".to_string())
    }
}

fn add_edge(
    graph: &mut [Vec<ResidualEdge>],
    from: usize,
    to: usize,
    capacity_l: u64,
    cost: Cost,
) -> Result<ArcRef, String> {
    if from >= graph.len() || to >= graph.len() || from == to {
        return Err("internal graph edge has invalid endpoints".into());
    }
    let reverse_cost = cost.checked_neg()?;
    let forward_index = graph[from].len();
    let reverse_index = graph[to].len();
    graph[from].push(ResidualEdge {
        to,
        reverse_index,
        residual_l: capacity_l,
        cost,
    });
    graph[to].push(ResidualEdge {
        to: from,
        reverse_index: forward_index,
        residual_l: 0,
        cost: reverse_cost,
    });
    Ok(ArcRef {
        from,
        edge_index: forward_index,
        initial_l: capacity_l,
    })
}

fn add_total(target: &mut u128, amount: u64, label: &str) -> Result<(), String> {
    *target = target
        .checked_add(u128::from(amount))
        .ok_or_else(|| format!("{label} total overflowed u128"))?;
    Ok(())
}

fn time_node_index(
    node_index: usize,
    day: usize,
    horizon: usize,
) -> Result<(usize, usize), String> {
    let slot = node_index
        .checked_mul(horizon)
        .and_then(|v| v.checked_add(day))
        .ok_or_else(|| "time-expanded node index overflowed usize".to_string())?;
    let input = slot
        .checked_mul(2)
        .ok_or_else(|| "time-expanded input index overflowed usize".to_string())?;
    let output = input
        .checked_add(1)
        .ok_or_else(|| "time-expanded output index overflowed usize".to_string())?;
    Ok((input, output))
}

/// Successive shortest augmenting paths with Bellman–Ford over lexicographic
/// costs. Reverse arcs allow prior allocations to be rerouted.
fn min_cost_max_flow(
    graph: &mut [Vec<ResidualEdge>],
    source: usize,
    sink: usize,
    dimensions: usize,
) -> Result<(u128, Cost), String> {
    if source >= graph.len() || sink >= graph.len() || source == sink {
        return Err("internal source/sink indices are invalid".into());
    }
    let mut total_flow_l = 0_u128;
    let mut total_cost = Cost::zero(dimensions);
    let vertex_count = graph.len();

    loop {
        let mut distance = vec![None::<Cost>; vertex_count];
        let mut previous = vec![None::<(usize, usize)>; vertex_count];
        distance[source] = Some(Cost::zero(dimensions));

        for _ in 0..vertex_count.saturating_sub(1) {
            let mut changed = false;
            for from in 0..vertex_count {
                let Some(prefix) = distance[from].clone() else {
                    continue;
                };
                for (edge_index, edge) in graph[from].iter().enumerate() {
                    if edge.residual_l == 0 {
                        continue;
                    }
                    let candidate = prefix.checked_add(&edge.cost)?;
                    let improve = distance[edge.to]
                        .as_ref()
                        .is_none_or(|current| candidate < *current);
                    if improve {
                        distance[edge.to] = Some(candidate);
                        previous[edge.to] = Some((from, edge_index));
                        changed = true;
                    }
                }
            }
            if !changed {
                break;
            }
        }

        let Some(path_cost) = distance[sink].clone() else {
            break;
        };
        let mut augment = None::<u64>;
        let mut cursor = sink;
        let mut path_steps = 0_usize;
        while cursor != source {
            let (from, edge_index) = previous[cursor]
                .ok_or_else(|| "shortest path predecessor chain is incomplete".to_string())?;
            let capacity = graph[from][edge_index].residual_l;
            augment = Some(augment.map_or(capacity, |old| old.min(capacity)));
            cursor = from;
            path_steps = path_steps
                .checked_add(1)
                .ok_or_else(|| "path length overflowed usize".to_string())?;
            if path_steps > vertex_count {
                return Err("residual shortest-path predecessor cycle detected".into());
            }
        }
        let augment_l = augment.ok_or_else(|| "augmenting path has no arcs".to_string())?;
        if augment_l == 0 {
            return Err("augmenting path has zero capacity".into());
        }

        cursor = sink;
        while cursor != source {
            let (from, edge_index) = previous[cursor].ok_or_else(|| {
                "shortest path predecessor chain changed unexpectedly".to_string()
            })?;
            let (to, reverse_index) = {
                let edge = &graph[from][edge_index];
                (edge.to, edge.reverse_index)
            };
            graph[from][edge_index].residual_l -= augment_l;
            graph[to][reverse_index].residual_l = graph[to][reverse_index]
                .residual_l
                .checked_add(augment_l)
                .ok_or_else(|| "reverse residual capacity overflowed u64".to_string())?;
            cursor = from;
        }

        total_flow_l = total_flow_l
            .checked_add(u128::from(augment_l))
            .ok_or_else(|| "total flow overflowed u128".to_string())?;
        total_cost = total_cost.checked_add(&path_cost.checked_scale(augment_l)?)?;
    }
    Ok((total_flow_l, total_cost))
}

impl PlanningProblem {
    pub fn validate(&self) -> Result<(), String> {
        if self.scenario_id.trim().is_empty() {
            return Err("scenario_id must not be empty".into());
        }
        if self.horizon_days == 0 {
            return Err("horizon_days must be positive".into());
        }
        if self.nodes.is_empty() {
            return Err("planner network must contain at least one node".into());
        }
        let horizon = usize::from(self.horizon_days);
        self.nodes
            .len()
            .checked_mul(horizon)
            .and_then(|v| v.checked_mul(2))
            .and_then(|v| v.checked_add(2))
            .ok_or_else(|| "time-expanded network size overflowed usize".to_string())?;

        let mut node_ids = BTreeSet::new();
        for node in &self.nodes {
            if node.id.trim().is_empty() {
                return Err("node id must not be empty".into());
            }
            if !node_ids.insert(node.id.as_str()) {
                return Err(format!("duplicate planner node id: {}", node.id));
            }
        }
        let mut edge_ids = BTreeSet::new();
        for edge in &self.edges {
            if edge.id.trim().is_empty() {
                return Err("edge id must not be empty".into());
            }
            if !edge_ids.insert(edge.id.as_str()) {
                return Err(format!("duplicate planner edge id: {}", edge.id));
            }
            if !node_ids.contains(edge.from.as_str()) || !node_ids.contains(edge.to.as_str()) {
                return Err(format!(
                    "planner edge {} references an unknown node",
                    edge.id
                ));
            }
            if edge.from == edge.to {
                return Err(format!("planner edge {} is a self-loop", edge.id));
            }
            if edge.transit_days == 0 {
                return Err(format!(
                    "planner edge {} transit_days must be positive",
                    edge.id
                ));
            }
        }
        let mut supply_ids = BTreeSet::new();
        for supply in &self.supplies {
            if supply.id.trim().is_empty() {
                return Err("supply id must not be empty".into());
            }
            if !supply_ids.insert(supply.id.as_str()) {
                return Err(format!("duplicate supply id: {}", supply.id));
            }
            if !node_ids.contains(supply.node_id.as_str()) {
                return Err(format!("supply {} references an unknown node", supply.id));
            }
            if supply.available_day >= self.horizon_days {
                return Err(format!(
                    "supply {} is outside the planning horizon",
                    supply.id
                ));
            }
        }
        let mut demand_ids = BTreeSet::new();
        for demand in &self.demands {
            if demand.id.trim().is_empty() {
                return Err("demand id must not be empty".into());
            }
            if !demand_ids.insert(demand.id.as_str()) {
                return Err(format!("duplicate demand id: {}", demand.id));
            }
            if !node_ids.contains(demand.node_id.as_str()) {
                return Err(format!("demand {} references an unknown node", demand.id));
            }
            if demand.due_day >= self.horizon_days {
                return Err(format!(
                    "demand {} is outside the planning horizon",
                    demand.id
                ));
            }
        }
        Ok(())
    }

    /// First maximize total served demand, then lexicographically maximize
    /// service from higher to lower priority, then minimize abstract costs.
    pub fn solve(&self) -> Result<PlanResult, String> {
        self.validate()?;
        let node_index: BTreeMap<&str, usize> = self
            .nodes
            .iter()
            .enumerate()
            .map(|(i, node)| (node.id.as_str(), i))
            .collect();

        let eligible: Vec<&Supply> = self
            .supplies
            .iter()
            .filter(|s| {
                matches!(
                    s.evidence,
                    SupplyEvidence::VerifiedUsable | SupplyEvidence::Scenario
                )
            })
            .collect();
        let excluded_supply_l = self
            .supplies
            .iter()
            .filter(|s| {
                !matches!(
                    s.evidence,
                    SupplyEvidence::VerifiedUsable | SupplyEvidence::Scenario
                )
            })
            .try_fold(0_u128, |total, s| {
                total
                    .checked_add(u128::from(s.volume_l))
                    .ok_or_else(|| "excluded supply total overflowed u128".to_string())
            })?;

        let priorities: Vec<u16> = self
            .demands
            .iter()
            .map(|d| d.priority)
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let priority_index: BTreeMap<u16, usize> = priorities
            .iter()
            .enumerate()
            .map(|(i, p)| (*p, i))
            .collect();
        let cost_dimensions = priorities.len() + 1;
        let cost_index = cost_dimensions - 1;
        let zero_cost = Cost::zero(cost_dimensions);

        let horizon = usize::from(self.horizon_days);
        let time_nodes = self
            .nodes
            .len()
            .checked_mul(horizon)
            .and_then(|v| v.checked_mul(2))
            .ok_or_else(|| "time-expanded network size overflowed usize".to_string())?;
        let source = time_nodes;
        let sink = source
            .checked_add(1)
            .ok_or_else(|| "sink index overflowed usize".to_string())?;
        let mut graph = vec![Vec::<ResidualEdge>::new(); sink + 1];

        // In->out caps total stock at each node/day. All departures,
        // consumption and carry-forward must pass through this capacity.
        for (index, node) in self.nodes.iter().enumerate() {
            for day in 0..horizon {
                let (input, output) = time_node_index(index, day, horizon)?;
                add_edge(
                    &mut graph,
                    input,
                    output,
                    node.storage_capacity_l,
                    zero_cost.clone(),
                )?;
                if day + 1 < horizon {
                    let (next_input, _) = time_node_index(index, day + 1, horizon)?;
                    let mut cost = Cost::zero(cost_dimensions);
                    cost.0[cost_index] = i128::from(self.holding_cost_per_litre_day);
                    add_edge(
                        &mut graph,
                        output,
                        next_input,
                        node.storage_capacity_l,
                        cost,
                    )?;
                }
            }
        }

        let mut route_refs = Vec::<(String, u16, u16, ArcRef)>::new();
        for edge in &self.edges {
            if !edge.grade_allowed[self.grade.index()] {
                continue;
            }
            let from = node_index[edge.from.as_str()];
            let to = node_index[edge.to.as_str()];
            for departure_day in 0..self.horizon_days {
                let Some(arrival_day) = departure_day.checked_add(edge.transit_days) else {
                    continue;
                };
                if arrival_day >= self.horizon_days {
                    continue;
                }
                let (_, from_output) = time_node_index(from, usize::from(departure_day), horizon)?;
                let (to_input, _) = time_node_index(to, usize::from(arrival_day), horizon)?;
                let mut cost = Cost::zero(cost_dimensions);
                cost.0[cost_index] = i128::from(edge.cost_per_litre);
                let arc = add_edge(
                    &mut graph,
                    from_output,
                    to_input,
                    edge.daily_capacity_l,
                    cost,
                )?;
                route_refs.push((edge.id.clone(), departure_day, arrival_day, arc));
            }
        }

        let mut supply_refs = Vec::<(&Supply, ArcRef)>::new();
        for supply in &eligible {
            let index = node_index[supply.node_id.as_str()];
            let (input, _) = time_node_index(index, usize::from(supply.available_day), horizon)?;
            let arc = add_edge(
                &mut graph,
                source,
                input,
                supply.volume_l,
                zero_cost.clone(),
            )?;
            supply_refs.push((supply, arc));
        }

        let mut demand_refs = Vec::<(&Demand, ArcRef)>::new();
        for demand in &self.demands {
            let index = node_index[demand.node_id.as_str()];
            let (_, output) = time_node_index(index, usize::from(demand.due_day), horizon)?;
            let tier = priority_index[&demand.priority];
            let mut cost = Cost::zero(cost_dimensions);
            for component in 0..tier {
                cost.0[component] = 1
            }
            let arc = add_edge(&mut graph, output, sink, demand.quantity_l, cost)?;
            demand_refs.push((demand, arc));
        }

        let (_flow, objective_components) =
            min_cost_max_flow(&mut graph, source, sink, cost_dimensions)?;

        let mut demand_allocations = Vec::with_capacity(demand_refs.len());
        let mut total_requested_l = 0_u128;
        let mut total_delivered_l = 0_u128;
        let mut total_unserved_l = 0_u128;
        for (demand, arc) in demand_refs {
            let delivered_l = arc.flow_l(&graph)?;
            let unserved_l = demand
                .quantity_l
                .checked_sub(delivered_l)
                .ok_or_else(|| "demand flow exceeds requested quantity".to_string())?;
            add_total(
                &mut total_requested_l,
                demand.quantity_l,
                "requested demand",
            )?;
            add_total(&mut total_delivered_l, delivered_l, "delivered demand")?;
            add_total(&mut total_unserved_l, unserved_l, "unserved demand")?;
            demand_allocations.push(DemandAllocation {
                demand_id: demand.id.clone(),
                node_id: demand.node_id.clone(),
                due_day: demand.due_day,
                priority: demand.priority,
                requested_l: demand.quantity_l,
                delivered_l,
                unserved_l,
            });
        }

        let mut dispatches = Vec::new();
        for (edge_id, departure_day, arrival_day, arc) in route_refs {
            let volume_l = arc.flow_l(&graph)?;
            if volume_l > 0 {
                dispatches.push(RouteDispatch {
                    edge_id,
                    departure_day,
                    arrival_day,
                    grade: self.grade,
                    volume_l,
                });
            }
        }
        dispatches.sort_by(|a, b| {
            (a.departure_day, a.edge_id.as_str()).cmp(&(b.departure_day, b.edge_id.as_str()))
        });

        let mut supply_uses = Vec::with_capacity(supply_refs.len());
        let mut unallocated_eligible_supply_l = 0_u128;
        for (supply, arc) in supply_refs {
            let allocated_l = arc.flow_l(&graph)?;
            let unallocated_l = supply
                .volume_l
                .checked_sub(allocated_l)
                .ok_or_else(|| "supply flow exceeds available quantity".to_string())?;
            add_total(
                &mut unallocated_eligible_supply_l,
                unallocated_l,
                "unallocated eligible supply",
            )?;
            supply_uses.push(SupplyUse {
                supply_id: supply.id.clone(),
                node_id: supply.node_id.clone(),
                available_day: supply.available_day,
                available_l: supply.volume_l,
                allocated_l,
                unallocated_l,
                evidence: supply.evidence,
            });
        }

        Ok(PlanResult {
            scenario_id: self.scenario_id.clone(),
            grade: self.grade,
            horizon_days: self.horizon_days,
            total_requested_l,
            total_delivered_l,
            total_unserved_l,
            unallocated_eligible_supply_l,
            excluded_supply_l,
            objective_components: objective_components.0,
            demand_allocations,
            dispatches,
            supply_uses,
        })
    }
}

pub fn base_case() -> PlanningProblem {
    PlanningProblem {
        scenario_id: "synthetic-port-to-inland-allocation".into(),
        grade: Grade::D50,
        horizon_days: 3,
        nodes: vec![
            PlannerNode {
                id: "port".into(),
                storage_capacity_l: 1_000,
            },
            PlannerNode {
                id: "inland".into(),
                storage_capacity_l: 1_000,
            },
        ],
        edges: vec![PlannerEdge {
            id: "port-pipeline-inland".into(),
            from: "port".into(),
            to: "inland".into(),
            daily_capacity_l: 250,
            transit_days: 1,
            cost_per_litre: 2,
            grade_allowed: [true, false],
        }],
        supplies: vec![Supply {
            id: "synthetic-cargo".into(),
            node_id: "port".into(),
            available_day: 0,
            volume_l: 200,
            evidence: SupplyEvidence::Scenario,
        }],
        demands: vec![Demand {
            id: "critical-inland-demand".into(),
            node_id: "inland".into(),
            due_day: 1,
            quantity_l: 150,
            priority: 0,
        }],
        holding_cost_per_litre_day: 0,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn node(id: &str, capacity: u64) -> PlannerNode {
        PlannerNode {
            id: id.into(),
            storage_capacity_l: capacity,
        }
    }

    fn edge(id: &str, from: &str, to: &str, capacity: u64, transit: u16, cost: u64) -> PlannerEdge {
        PlannerEdge {
            id: id.into(),
            from: from.into(),
            to: to.into(),
            daily_capacity_l: capacity,
            transit_days: transit,
            cost_per_litre: cost,
            grade_allowed: [true, false],
        }
    }

    fn base_problem() -> PlanningProblem {
        PlanningProblem {
            scenario_id: "synthetic-test".into(),
            grade: Grade::D50,
            horizon_days: 3,
            nodes: vec![node("port", 1_000), node("inland", 1_000)],
            edges: vec![edge("port-inland", "port", "inland", 1_000, 1, 1)],
            supplies: vec![Supply {
                id: "supply".into(),
                node_id: "port".into(),
                available_day: 0,
                volume_l: 100,
                evidence: SupplyEvidence::VerifiedUsable,
            }],
            demands: vec![Demand {
                id: "demand".into(),
                node_id: "inland".into(),
                due_day: 1,
                quantity_l: 100,
                priority: 0,
            }],
            holding_cost_per_litre_day: 0,
        }
    }

    #[test]
    fn allocates_supply_through_time_expanded_route() {
        let result = base_problem().solve().unwrap();
        assert_eq!(result.total_requested_l, 100);
        assert_eq!(result.total_delivered_l, 100);
        assert_eq!(result.total_unserved_l, 0);
        assert_eq!(result.dispatches.len(), 1);
        assert_eq!(result.dispatches[0].departure_day, 0);
        assert_eq!(result.dispatches[0].arrival_day, 1);
        assert_eq!(result.dispatches[0].volume_l, 100);
    }

    #[test]
    fn chooses_lower_cost_path_within_deadline() {
        let mut problem = base_problem();
        problem.nodes.push(node("relay", 1_000));
        problem.edges = vec![
            edge("direct", "port", "inland", 100, 2, 9),
            edge("port-relay", "port", "relay", 100, 1, 1),
            edge("relay-inland", "relay", "inland", 100, 1, 1),
        ];
        problem.demands[0].due_day = 2;
        let result = problem.solve().unwrap();
        assert_eq!(result.total_delivered_l, 100);
        assert_eq!(result.dispatches.len(), 2);
        assert!(result.dispatches.iter().all(|r| r.edge_id != "direct"));
        assert!(result.dispatches.iter().any(|r| r.edge_id == "port-relay"));
        assert!(
            result
                .dispatches
                .iter()
                .any(|r| r.edge_id == "relay-inland")
        );
    }

    #[test]
    fn protects_higher_priority_demand_when_supply_is_scarce() {
        let mut problem = base_problem();
        problem.nodes.push(node("critical", 1_000));
        problem
            .edges
            .push(edge("port-critical", "port", "critical", 100, 1, 1));
        problem.demands = vec![
            Demand {
                id: "ordinary".into(),
                node_id: "inland".into(),
                due_day: 1,
                quantity_l: 100,
                priority: 2,
            },
            Demand {
                id: "critical".into(),
                node_id: "critical".into(),
                due_day: 1,
                quantity_l: 100,
                priority: 0,
            },
        ];
        let result = problem.solve().unwrap();
        let critical = result
            .demand_allocations
            .iter()
            .find(|d| d.demand_id == "critical")
            .unwrap();
        let ordinary = result
            .demand_allocations
            .iter()
            .find(|d| d.demand_id == "ordinary")
            .unwrap();
        assert_eq!(critical.delivered_l, 100);
        assert_eq!(ordinary.delivered_l, 0);
    }

    #[test]
    fn storage_capacity_limits_deliverable_volume() {
        let mut problem = base_problem();
        problem.nodes[1].storage_capacity_l = 50;
        let result = problem.solve().unwrap();
        assert_eq!(result.total_delivered_l, 50);
        assert_eq!(result.total_unserved_l, 50);
    }

    #[test]
    fn route_arriving_after_due_day_cannot_serve_demand() {
        let mut problem = base_problem();
        problem.edges[0].transit_days = 2;
        let result = problem.solve().unwrap();
        assert_eq!(result.total_delivered_l, 0);
        assert_eq!(result.total_unserved_l, 100);
        assert!(result.dispatches.is_empty());
    }

    #[test]
    fn incompatible_grade_route_is_not_used() {
        let mut problem = base_problem();
        problem.grade = Grade::D500;
        let result = problem.solve().unwrap();
        assert_eq!(result.total_delivered_l, 0);
        assert_eq!(result.total_unserved_l, 100);
        assert!(result.dispatches.is_empty());
    }

    #[test]
    fn reported_and_forecast_supply_are_excluded() {
        let mut problem = base_problem();
        problem.supplies[0].evidence = SupplyEvidence::Reported;
        problem.supplies.push(Supply {
            id: "forecast".into(),
            node_id: "port".into(),
            available_day: 0,
            volume_l: 50,
            evidence: SupplyEvidence::Forecast,
        });
        let result = problem.solve().unwrap();
        assert_eq!(result.excluded_supply_l, 150);
        assert_eq!(result.total_delivered_l, 0);
        assert_eq!(result.total_unserved_l, 100);
    }

    #[test]
    fn duplicate_identifiers_fail_closed() {
        let mut problem = base_problem();
        problem.supplies.push(problem.supplies[0].clone());
        assert!(problem.validate().is_err());
    }

    #[test]
    fn scenario_supply_remains_labelled() {
        let mut problem = base_problem();
        problem.supplies[0].evidence = SupplyEvidence::Scenario;
        let result = problem.solve().unwrap();
        assert_eq!(result.total_delivered_l, 100);
        assert_eq!(result.supply_uses[0].evidence, SupplyEvidence::Scenario);
    }
}
