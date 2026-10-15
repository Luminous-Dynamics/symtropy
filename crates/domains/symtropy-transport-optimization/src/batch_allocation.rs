// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Bounded, deterministic assignment of multiple demands to explicit route candidates.
//!
//! This is an exact search over the candidate routes supplied by the caller, not a
//! complete network-flow solver. Candidate-set completeness is not inferred.

use super::{
    AggregateMetrics, Demand, Edge, EdgeOperation, Mode, ModelError, Network, QuantityUnit, Route,
    RouteMetrics,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

/// Caller-declared coverage of the route-candidate pool for one demand.
///
/// The exhaustive variant is an assertion by the route generator, not a proof
/// produced by this selector. Its domain ID should identify a versioned,
/// machine-readable search-domain definition. Consumers needing network-wide
/// optimality must independently validate that definition and its coverage.
#[derive(Clone, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum CandidatePoolCompleteness {
    /// The caller asserts the pool exhausts a specifically named search domain.
    ExhaustiveWithinDeclaredSearch { search_domain_id: String },
    /// Only a single-demand Pareto frontier was retained; bottleneck-useful
    /// dominated routes may have been removed.
    ParetoFrontierOnly,
    /// The pool comes from a bounded heuristic/subset of candidate routes.
    HeuristicSubset,
    /// Coverage has not been established.
    Unknown,
}

/// Machine-readable per-demand coverage echoed from input into the result.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemandCandidateCompleteness {
    pub demand_id: String,
    pub completeness: CandidatePoolCompleteness,
}

/// Scope over which the returned optimum is exact after a completed search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BatchOptimalityScope {
    /// Exact over the candidate routes explicitly supplied to this call only.
    ExactOverSuppliedCandidatePools,
    /// Every pool was caller-labelled exhaustive for its declared search domain.
    /// This echoes caller assertions; it does not prove those domains cover the
    /// whole network or the real-world feasible route set.
    ExactOverCallerAssertedSearchDomains,
}

/// The single quantity-scaled metric that the batch allocator minimizes.
///
/// Distance and elapsed time are deliberately omitted: the current route model
/// represents them per trip, not per unit of cargo. They are reported as sums of
/// selected route-level measurements but are not silently reinterpreted as
/// tonne-kilometres, passenger-kilometres, or fleet makespan.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum BatchObjective {
    OperatorCostMinorUnits,
    UserCashCostMinorUnits,
    ExternalCostMinorUnits,
    EnergyWattHours,
    GreenhouseGasGramsCo2e,
    SafetyRiskMicroUnits,
    DisruptionRiskMicroUnits,
}

impl BatchObjective {
    fn score(self, metrics: &AggregateMetrics) -> u64 {
        match self {
            Self::OperatorCostMinorUnits => metrics.operator_cost_minor_units,
            Self::UserCashCostMinorUnits => metrics.user_cash_cost_minor_units,
            Self::ExternalCostMinorUnits => metrics.external_cost_minor_units,
            Self::EnergyWattHours => metrics.energy_wh,
            Self::GreenhouseGasGramsCo2e => metrics.ghg_grams_co2e,
            Self::SafetyRiskMicroUnits => metrics.safety_risk_micro_units,
            Self::DisruptionRiskMicroUnits => metrics.disruption_risk_micro_units,
        }
    }
}

/// All candidate routes for one indivisible demand.
///
/// The caller supplies the candidate pool explicitly. The allocator verifies each
/// route against the network, but cannot prove that the pool contains every
/// feasible route. The coverage assertion is retained in the result report.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DemandRouteCandidates {
    pub demand: Demand,
    pub routes: Vec<Route>,
    pub completeness: CandidatePoolCompleteness,
}

/// Explicit hard limit on the finite batch-assignment search.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BatchAllocationLimits {
    /// Maximum route-choice attempts and search states, including rejected
    /// capacity choices. The solver errors rather than return a partial optimum.
    pub max_search_steps: usize,
}

impl Default for BatchAllocationLimits {
    fn default() -> Self {
        Self {
            max_search_steps: 100_000,
        }
    }
}

/// One selected route, indexed by the stable demand identity.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BatchRouteAssignment {
    pub demand_id: String,
    pub route: Route,
}

/// Total use of one static network edge across the selected batch.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedEdgeCapacityUse {
    pub edge_id: String,
    pub quantity_unit: QuantityUnit,
    pub quantity_used: u64,
    pub capacity_limit: u64,
    pub demand_ids: Vec<String>,
}

/// Deterministic assignment report for the supplied route-candidate pools.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SharedCapacityBatchReport {
    pub objective: BatchObjective,
    pub objective_value: u64,
    /// Exactness scope, limited by the supplied candidate-pool coverage assertions.
    pub optimality_scope: BatchOptimalityScope,
    /// Sorted by demand ID; echoes each input coverage assertion for independent review.
    pub candidate_completeness: Vec<DemandCandidateCompleteness>,
    pub total_metrics: AggregateMetrics,
    /// Sorted by demand ID.
    pub assignments: Vec<BatchRouteAssignment>,
    /// Sorted by edge ID; each shared edge is accounted for once.
    pub capacity_uses: Vec<SharedEdgeCapacityUse>,
    pub search_steps: usize,
}

/// Errors are fail-closed: no partial assignment is returned for invalid inputs,
/// arithmetic overflow, infeasibility, or exhaustion of the declared search cap.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchAllocationError {
    InvalidSearchLimit,
    EmptyBatch,
    DuplicateDemandIdentity(String),
    NoCandidateRoutes(String),
    InvalidCandidateSearchDomain { demand_id: String },
    DuplicateCandidateRoute {
        demand_id: String,
        edge_ids: Vec<String>,
    },
    InvalidCandidateRoute {
        demand_id: String,
        route_index: usize,
        reason: &'static str,
    },
    NoFeasibleBatchAllocation,
    SearchBudgetExceeded {
        limit: usize,
    },
    ArithmeticOverflow,
    Model(ModelError),
}

impl fmt::Display for BatchAllocationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidSearchLimit => write!(f, "batch search limit must be positive"),
            Self::EmptyBatch => write!(f, "batch must contain at least one demand"),
            Self::DuplicateDemandIdentity(id) => {
                write!(f, "batch repeats demand identity {id}")
            }
            Self::NoCandidateRoutes(id) => {
                write!(f, "demand {id} has no candidate routes")
            }
            Self::InvalidCandidateSearchDomain { demand_id } => write!(
                f,
                "exhaustive candidate claim for demand {demand_id} requires a non-empty search-domain ID"
            ),
            Self::DuplicateCandidateRoute {
                demand_id,
                edge_ids,
            } => write!(f, "demand {demand_id} repeats candidate route {edge_ids:?}"),
            Self::InvalidCandidateRoute {
                demand_id,
                route_index,
                reason,
            } => write!(
                f,
                "candidate route {route_index} for demand {demand_id} is invalid: {reason}"
            ),
            Self::NoFeasibleBatchAllocation => write!(
                f,
                "no candidate-route assignment satisfies shared edge capacities"
            ),
            Self::SearchBudgetExceeded { limit } => write!(
                f,
                "batch route-assignment search exceeded its {limit}-step limit"
            ),
            Self::ArithmeticOverflow => write!(f, "batch route or capacity arithmetic overflowed"),
            Self::Model(error) => write!(f, "network or demand validation failed: {error}"),
        }
    }
}

impl std::error::Error for BatchAllocationError {}

#[derive(Clone, Copy)]
struct EdgeCapacity {
    capacity: u64,
}

struct BestPlan {
    objective_value: u64,
    signature: Vec<Vec<String>>,
    total_metrics: AggregateMetrics,
    routes: Vec<Route>,
}

struct SearchContext<'a> {
    candidates: &'a [DemandRouteCandidates],
    edge_capacities: BTreeMap<String, EdgeCapacity>,
    objective: BatchObjective,
    limits: BatchAllocationLimits,
    search_steps: usize,
    best: Option<BestPlan>,
}

impl SearchContext<'_> {
    fn tick(&mut self) -> Result<(), BatchAllocationError> {
        if self.search_steps >= self.limits.max_search_steps {
            return Err(BatchAllocationError::SearchBudgetExceeded {
                limit: self.limits.max_search_steps,
            });
        }
        self.search_steps += 1;
        Ok(())
    }

    fn visit(
        &mut self,
        index: usize,
        used: &mut BTreeMap<String, u64>,
        chosen: &mut Vec<Route>,
        totals: AggregateMetrics,
    ) -> Result<(), BatchAllocationError> {
        self.tick()?;
        if index == self.candidates.len() {
            let signature = chosen
                .iter()
                .map(|route| route.edge_ids.clone())
                .collect::<Vec<_>>();
            let objective_value = self.objective.score(&totals);
            let replace = match &self.best {
                None => true,
                Some(best) => {
                    objective_value < best.objective_value
                        || (objective_value == best.objective_value && signature < best.signature)
                }
            };
            if replace {
                self.best = Some(BestPlan {
                    objective_value,
                    signature,
                    total_metrics: totals,
                    routes: chosen.clone(),
                });
            }
            return Ok(());
        }

        // Clone the local candidate list so recursive mutable search does not
        // hold an immutable borrow into self across the next search level.
        let demand_id = self.candidates[index].demand.id.clone();
        let quantity = self.candidates[index].demand.quantity;
        let routes = self.candidates[index].routes.clone();
        for (route_index, route) in routes.iter().enumerate() {
            self.tick()?;
            if !self.route_fits(&demand_id, route_index, route, quantity, used)? {
                continue;
            }

            let route_metrics = route
                .aggregate(quantity)
                .map_err(|_| BatchAllocationError::ArithmeticOverflow)?;
            let next_totals = add_aggregate_metrics(&totals, &route_metrics)?;
            let previous =
                self.reserve_route(&demand_id, route_index, route, quantity, used)?;
            chosen.push(route.clone());
            let result = self.visit(index + 1, used, chosen, next_totals);
            chosen.pop();
            self.restore_route(used, previous);
            result?;
        }
        Ok(())
    }

    fn route_fits(
        &self,
        demand_id: &str,
        route_index: usize,
        route: &Route,
        quantity: u64,
        used: &BTreeMap<String, u64>,
    ) -> Result<bool, BatchAllocationError> {
        for edge_id in &route.edge_ids {
            let Some(edge) = self.edge_capacities.get(edge_id) else {
                return Err(BatchAllocationError::InvalidCandidateRoute {
                    demand_id: demand_id.to_owned(),
                    route_index,
                    reason: "route references an unknown network edge",
                });
            };
            let current = used.get(edge_id).copied().unwrap_or(0);
            let next = current
                .checked_add(quantity)
                .ok_or(BatchAllocationError::ArithmeticOverflow)?;
            if next > edge.capacity {
                return Ok(false);
            }
        }
        Ok(true)
    }

    fn reserve_route(
        &self,
        demand_id: &str,
        route_index: usize,
        route: &Route,
        quantity: u64,
        used: &mut BTreeMap<String, u64>,
    ) -> Result<Vec<(String, Option<u64>)>, BatchAllocationError> {
        let mut previous_values = Vec::with_capacity(route.edge_ids.len());
        for edge_id in &route.edge_ids {
            if !self.edge_capacities.contains_key(edge_id) {
                return Err(BatchAllocationError::InvalidCandidateRoute {
                    demand_id: demand_id.to_owned(),
                    route_index,
                    reason: "route references an unknown network edge",
                });
            }
            let previous = used.get(edge_id).copied();
            let next = previous
                .unwrap_or(0)
                .checked_add(quantity)
                .ok_or(BatchAllocationError::ArithmeticOverflow)?;
            used.insert(edge_id.clone(), next);
            previous_values.push((edge_id.clone(), previous));
        }
        Ok(previous_values)
    }

    fn restore_route(
        &self,
        used: &mut BTreeMap<String, u64>,
        previous_values: Vec<(String, Option<u64>)>,
    ) {
        for (edge_id, previous) in previous_values.into_iter().rev() {
            match previous {
                Some(quantity) => {
                    used.insert(edge_id, quantity);
                }
                None => {
                    used.remove(&edge_id);
                }
            }
        }
    }
}

/// Select one route for each demand while enforcing shared static edge capacities.
///
/// The search is exhaustive over the candidate routes supplied for each demand,
/// minimizes one explicitly selected quantity-scaled metric, and breaks exact
/// objective ties lexicographically by route edge IDs in demand-ID order. It
/// rejects candidate paths whose topology, mode transitions, units, capacity,
/// transfer count, or supplied metrics disagree with the network.
///
/// Important limits:
/// - A demand is indivisible; splitting a demand across paths is not supported.
/// - This is exact only over the provided candidate pools. Pareto-only candidate
///   sets may omit dominated paths that are valuable for relieving shared edges.
/// - Capacity is static edge capacity, not a time-windowed reservation, flow rate,
///   train-path slot, or live physical availability.
/// - It is not a general multi-commodity min-cost-flow or dispatch solver.
pub fn select_shared_capacity_batch(
    network: &Network,
    candidate_sets: &[DemandRouteCandidates],
    objective: BatchObjective,
    limits: BatchAllocationLimits,
) -> Result<SharedCapacityBatchReport, BatchAllocationError> {
    network.validate().map_err(BatchAllocationError::Model)?;
    if candidate_sets.is_empty() {
        return Err(BatchAllocationError::EmptyBatch);
    }
    if limits.max_search_steps == 0 {
        return Err(BatchAllocationError::InvalidSearchLimit);
    }

    let edges_by_id: BTreeMap<&str, &Edge> = network
        .edges
        .iter()
        .map(|edge| (edge.id.as_str(), edge))
        .collect();

    let mut candidates = candidate_sets.to_vec();
    let mut demand_ids = BTreeSet::new();
    for candidate_set in &mut candidates {
        candidate_set
            .demand
            .validate(network)
            .map_err(BatchAllocationError::Model)?;
        if !demand_ids.insert(candidate_set.demand.id.clone()) {
            return Err(BatchAllocationError::DuplicateDemandIdentity(
                candidate_set.demand.id.clone(),
            ));
        }
        if candidate_set.routes.is_empty() {
            return Err(BatchAllocationError::NoCandidateRoutes(
                candidate_set.demand.id.clone(),
            ));
        }
        if let CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch { search_domain_id } =
            &candidate_set.completeness
        {
            if search_domain_id.trim().is_empty() {
                return Err(BatchAllocationError::InvalidCandidateSearchDomain {
                    demand_id: candidate_set.demand.id.clone(),
                });
            }
        }

        candidate_set.routes.sort_by(|left, right| {
            left.edge_ids
                .cmp(&right.edge_ids)
                .then_with(|| left.transfer_count.cmp(&right.transfer_count))
                .then_with(|| left.metrics_per_unit.cmp(&right.metrics_per_unit))
        });
        let mut route_signatures = BTreeSet::new();
        for (route_index, route) in candidate_set.routes.iter().enumerate() {
            validate_candidate_route(&candidate_set.demand, route, &edges_by_id).map_err(
                |reason| BatchAllocationError::InvalidCandidateRoute {
                    demand_id: candidate_set.demand.id.clone(),
                    route_index,
                    reason,
                },
            )?;
            if !route_signatures.insert(route.edge_ids.clone()) {
                return Err(BatchAllocationError::DuplicateCandidateRoute {
                    demand_id: candidate_set.demand.id.clone(),
                    edge_ids: route.edge_ids.clone(),
                });
            }
            route
                .aggregate(candidate_set.demand.quantity)
                .map_err(|_| BatchAllocationError::ArithmeticOverflow)?;
        }
    }
    candidates.sort_by(|left, right| left.demand.id.cmp(&right.demand.id));
    let candidate_completeness = candidates
        .iter()
        .map(|candidate_set| DemandCandidateCompleteness {
            demand_id: candidate_set.demand.id.clone(),
            completeness: candidate_set.completeness.clone(),
        })
        .collect::<Vec<_>>();
    let optimality_scope = if candidates.iter().all(|candidate_set| {
        matches!(
            &candidate_set.completeness,
            CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch { .. }
        )
    }) {
        BatchOptimalityScope::ExactOverCallerAssertedSearchDomains
    } else {
        BatchOptimalityScope::ExactOverSuppliedCandidatePools
    };

    let edge_capacities = network
        .edges
        .iter()
        .map(|edge| {
            (
                edge.id.clone(),
                EdgeCapacity {
                    capacity: edge.capacity,
                },
            )
        })
        .collect();

    let mut search = SearchContext {
        candidates: &candidates,
        edge_capacities,
        objective,
        limits,
        search_steps: 0,
        best: None,
    };
    search.visit(
        0,
        &mut BTreeMap::new(),
        &mut Vec::with_capacity(candidates.len()),
        zero_aggregate_metrics(),
    )?;
    let search_steps = search.search_steps;
    let best = search
        .best
        .ok_or(BatchAllocationError::NoFeasibleBatchAllocation)?;

    let mut edge_usage: BTreeMap<String, (u64, Vec<String>)> = BTreeMap::new();
    let mut assignments = Vec::with_capacity(candidates.len());
    for (candidate_set, route) in candidates.iter().zip(best.routes.iter()) {
        assignments.push(BatchRouteAssignment {
            demand_id: candidate_set.demand.id.clone(),
            route: route.clone(),
        });
        for edge_id in &route.edge_ids {
            let entry = edge_usage
                .entry(edge_id.clone())
                .or_insert_with(|| (0, Vec::new()));
            entry.0 = entry
                .0
                .checked_add(candidate_set.demand.quantity)
                .ok_or(BatchAllocationError::ArithmeticOverflow)?;
            entry.1.push(candidate_set.demand.id.clone());
        }
    }

    let mut capacity_uses = Vec::with_capacity(edge_usage.len());
    for (edge_id, (quantity_used, demand_ids)) in edge_usage {
        let edge = edges_by_id.get(edge_id.as_str()).ok_or(
            BatchAllocationError::InvalidCandidateRoute {
                demand_id: String::new(),
                route_index: 0,
                reason: "selected route references an unknown network edge",
            },
        )?;
        if quantity_used > edge.capacity {
            return Err(BatchAllocationError::NoFeasibleBatchAllocation);
        }
        capacity_uses.push(SharedEdgeCapacityUse {
            edge_id,
            quantity_unit: edge.capacity_unit,
            quantity_used,
            capacity_limit: edge.capacity,
            demand_ids,
        });
    }

    Ok(SharedCapacityBatchReport {
        objective,
        objective_value: best.objective_value,
        optimality_scope,
        candidate_completeness,
        total_metrics: best.total_metrics,
        assignments,
        capacity_uses,
        search_steps,
    })
}

fn validate_candidate_route(
    demand: &Demand,
    route: &Route,
    edges_by_id: &BTreeMap<&str, &Edge>,
) -> Result<(), &'static str> {
    if route.edge_ids.is_empty() {
        if demand.origin == demand.destination
            && route.transfer_count == 0
            && route.metrics_per_unit == RouteMetrics::default()
        {
            return Ok(());
        }
        return Err("empty route does not connect distinct demand endpoints");
    }

    let mut current_node = demand.origin.as_str();
    let mut current_mode: Option<Mode> = None;
    let mut transfer_count = 0_u32;
    let mut metrics = RouteMetrics::default();
    let mut edge_ids = BTreeSet::new();
    let mut visited_states = BTreeSet::from([(demand.origin.clone(), None)]);

    for (index, edge_id) in route.edge_ids.iter().enumerate() {
        if !edge_ids.insert(edge_id.as_str()) {
            return Err("route repeats a network edge");
        }
        let Some(edge) = edges_by_id.get(edge_id.as_str()).copied() else {
            return Err("route references an unknown network edge");
        };
        if edge.from.as_str() != current_node {
            return Err("route edges are not contiguous from the demand origin");
        }
        if edge.capacity_unit != demand.unit {
            return Err("edge capacity unit does not match demand unit");
        }
        if edge.capacity < demand.quantity {
            return Err("candidate route requires more than an edge's individual capacity");
        }

        let next_mode = match edge.operation {
            EdgeOperation::Movement { mode } => {
                if current_mode.is_some_and(|previous| previous != mode) {
                    return Err("movement mode changes without an explicit transfer edge");
                }
                mode
            }
            EdgeOperation::Transfer { from, to } => {
                if current_mode != Some(from) {
                    return Err("transfer edge does not match the current transport mode");
                }
                transfer_count = transfer_count
                    .checked_add(1)
                    .ok_or("route transfer count overflows")?;
                if demand
                    .max_transfers
                    .is_some_and(|maximum| transfer_count > maximum)
                {
                    return Err("route exceeds the demand's transfer limit");
                }
                to
            }
        };

        metrics = metrics
            .checked_add(edge.metrics)
            .map_err(|_| "route metrics overflow")?;
        if demand
            .max_elapsed_seconds
            .is_some_and(|maximum| metrics.elapsed_seconds > maximum)
        {
            return Err("route exceeds the demand's elapsed-time limit");
        }
        if edge.to == demand.origin {
            return Err("route returns to its origin");
        }
        if edge.to == demand.destination && index + 1 != route.edge_ids.len() {
            return Err("route continues after reaching its destination");
        }
        current_node = edge.to.as_str();
        if !visited_states.insert((current_node.to_owned(), Some(next_mode))) {
            return Err("route repeats a node and transport-mode state");
        }
        current_mode = Some(next_mode);
    }

    if current_node != demand.destination.as_str() {
        return Err("route does not end at the demand destination");
    }
    if transfer_count != route.transfer_count {
        return Err("declared transfer count does not match route edges");
    }
    if metrics != route.metrics_per_unit {
        return Err("declared metrics do not match the route edges");
    }
    Ok(())
}

fn zero_aggregate_metrics() -> AggregateMetrics {
    AggregateMetrics {
        distance_metres: 0,
        elapsed_seconds: 0,
        operator_cost_minor_units: 0,
        user_cash_cost_minor_units: 0,
        external_cost_minor_units: 0,
        energy_wh: 0,
        ghg_grams_co2e: 0,
        safety_risk_micro_units: 0,
        disruption_risk_micro_units: 0,
    }
}

fn add_aggregate_metrics(
    left: &AggregateMetrics,
    right: &AggregateMetrics,
) -> Result<AggregateMetrics, BatchAllocationError> {
    let add = |a: u64, b: u64| {
        a.checked_add(b)
            .ok_or(BatchAllocationError::ArithmeticOverflow)
    };
    Ok(AggregateMetrics {
        distance_metres: add(left.distance_metres, right.distance_metres)?,
        elapsed_seconds: add(left.elapsed_seconds, right.elapsed_seconds)?,
        operator_cost_minor_units: add(
            left.operator_cost_minor_units,
            right.operator_cost_minor_units,
        )?,
        user_cash_cost_minor_units: add(
            left.user_cash_cost_minor_units,
            right.user_cash_cost_minor_units,
        )?,
        external_cost_minor_units: add(
            left.external_cost_minor_units,
            right.external_cost_minor_units,
        )?,
        energy_wh: add(left.energy_wh, right.energy_wh)?,
        ghg_grams_co2e: add(left.ghg_grams_co2e, right.ghg_grams_co2e)?,
        safety_risk_micro_units: add(left.safety_risk_micro_units, right.safety_risk_micro_units)?,
        disruption_risk_micro_units: add(
            left.disruption_risk_micro_units,
            right.disruption_risk_micro_units,
        )?,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics(operator_cost: u64) -> super::super::EdgeMetrics {
        super::super::EdgeMetrics {
            distance_metres: 1,
            elapsed_seconds: 1,
            operator_cost_minor_per_unit: operator_cost,
            ..Default::default()
        }
    }

    fn demand(id: &str, quantity: u64) -> Demand {
        Demand {
            id: id.to_owned(),
            origin: "A".to_owned(),
            destination: "B".to_owned(),
            quantity,
            unit: QuantityUnit::Tonnes,
            max_elapsed_seconds: None,
            max_transfers: None,
        }
    }

    fn network(shared_capacity: u64) -> Network {
        let mut network = Network::new(["A", "B", "C"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "shared",
            "A",
            "B",
            Mode::Rail,
            shared_capacity,
            QuantityUnit::Tonnes,
            metrics(1),
        ));
        network.add_edge(Edge::movement(
            "alt-a",
            "A",
            "C",
            Mode::Rail,
            10,
            QuantityUnit::Tonnes,
            metrics(5),
        ));
        network.add_edge(Edge::movement(
            "alt-b",
            "C",
            "B",
            Mode::Rail,
            10,
            QuantityUnit::Tonnes,
            metrics(5),
        ));
        network
    }

    fn route(network: &Network, edge_ids: &[&str]) -> Route {
        let mut metrics = RouteMetrics::default();
        for edge_id in edge_ids {
            let edge = network
                .edges
                .iter()
                .find(|edge| edge.id == *edge_id)
                .unwrap();
            metrics = metrics.checked_add(edge.metrics).unwrap();
        }
        Route {
            edge_ids: edge_ids.iter().map(|id| (*id).to_owned()).collect(),
            transfer_count: 0,
            metrics_per_unit: metrics,
        }
    }

    fn routes_for(network: &Network) -> (Route, Route) {
        (
            route(network, &["shared"]),
            route(network, &["alt-a", "alt-b"]),
        )
    }

    fn candidate(id: &str, quantity: u64, routes: Vec<Route>) -> DemandRouteCandidates {
        candidate_with_completeness(
            id,
            quantity,
            routes,
            CandidatePoolCompleteness::Unknown,
        )
    }

    fn candidate_with_completeness(
        id: &str,
        quantity: u64,
        routes: Vec<Route>,
        completeness: CandidatePoolCompleteness,
    ) -> DemandRouteCandidates {
        DemandRouteCandidates {
            demand: demand(id, quantity),
            routes,
            completeness,
        }
    }

    #[test]
    fn batch_avoids_double_spending_a_shared_bottleneck() {
        let network = network(7);
        let (shared, alternate) = routes_for(&network);
        let report = select_shared_capacity_batch(
            &network,
            &[
                candidate("d1", 4, vec![shared.clone(), alternate.clone()]),
                candidate("d2", 4, vec![shared.clone(), alternate.clone()]),
            ],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        )
        .unwrap();

        assert_eq!(report.objective_value, 44);
        assert_eq!(report.total_metrics.operator_cost_minor_units, 44);
        assert_eq!(report.assignments.len(), 2);
        assert_eq!(report.assignments[0].demand_id, "d1");
        assert_eq!(
            report.assignments[0].route.edge_ids,
            vec!["alt-a".to_owned(), "alt-b".to_owned()]
        );
        assert_eq!(report.assignments[1].demand_id, "d2");
        assert_eq!(report.assignments[1].route.edge_ids, vec!["shared"]);

        let shared_use = report
            .capacity_uses
            .iter()
            .find(|usage| usage.edge_id == "shared")
            .unwrap();
        assert_eq!(shared_use.quantity_used, 4);
        assert_eq!(shared_use.capacity_limit, 7);
        assert_eq!(shared_use.demand_ids, vec!["d2".to_owned()]);
    }

    #[test]
    fn different_approaches_cannot_double_spend_a_shared_downstream_corridor() {
        let mut network = Network::new(["A", "X", "Hub", "B", "Alt"], "ZAR").unwrap();
        for (id, from, to, cost, capacity) in [
            ("approach-a", "A", "Hub", 1, 10),
            ("approach-x", "X", "Hub", 1, 10),
            ("shared", "Hub", "B", 1, 7),
            ("alt-a", "A", "Alt", 5, 10),
            ("alt-x", "X", "Alt", 5, 10),
            ("alt-final", "Alt", "B", 5, 10),
        ] {
            network.add_edge(Edge::movement(
                id,
                from,
                to,
                Mode::Rail,
                capacity,
                QuantityUnit::Tonnes,
                metrics(cost),
            ));
        }

        let mut first = demand("d1", 4);
        first.destination = "B".to_owned();
        let mut second = demand("d2", 4);
        second.origin = "X".to_owned();
        second.destination = "B".to_owned();

        let report = select_shared_capacity_batch(
            &network,
            &[
                DemandRouteCandidates {
                    demand: first,
                    routes: vec![
                        route(&network, &["approach-a", "shared"]),
                        route(&network, &["alt-a", "alt-final"]),
                    ],
                    completeness: CandidatePoolCompleteness::Unknown,
                },
                DemandRouteCandidates {
                    demand: second,
                    routes: vec![
                        route(&network, &["approach-x", "shared"]),
                        route(&network, &["alt-x", "alt-final"]),
                    ],
                    completeness: CandidatePoolCompleteness::Unknown,
                },
            ],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        )
        .unwrap();

        let shared = report
            .capacity_uses
            .iter()
            .find(|usage| usage.edge_id == "shared")
            .unwrap();
        assert_eq!(shared.quantity_used, 4);
        assert_eq!(shared.capacity_limit, 7);
        assert_eq!(shared.demand_ids.len(), 1);
        assert_eq!(report.assignments.len(), 2);
        assert!(report
            .assignments
            .iter()
            .all(|assignment| !assignment.route.edge_ids.is_empty()));
    }

    #[test]
    fn infeasible_batch_is_rejected_instead_of_returning_a_partial_assignment() {
        let network = network(7);
        let (shared, _) = routes_for(&network);
        let result = select_shared_capacity_batch(
            &network,
            &[
                candidate("d1", 4, vec![shared.clone()]),
                candidate("d2", 4, vec![shared]),
            ],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        );
        assert_eq!(result, Err(BatchAllocationError::NoFeasibleBatchAllocation));
    }

    #[test]
    fn result_is_deterministic_under_input_candidate_reordering() {
        let network = network(7);
        let (shared, alternate) = routes_for(&network);
        let forward = [
            candidate("d1", 4, vec![shared.clone(), alternate.clone()]),
            candidate("d2", 4, vec![alternate.clone(), shared.clone()]),
        ];
        let reverse = [
            candidate("d2", 4, vec![shared, alternate.clone()]),
            candidate("d1", 4, vec![alternate, route(&network, &["shared"])]),
        ];
        let left = select_shared_capacity_batch(
            &network,
            &forward,
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        )
        .unwrap();
        let right = select_shared_capacity_batch(
            &network,
            &reverse,
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        )
        .unwrap();
        assert_eq!(left, right);
    }

    #[test]
    fn inconsistent_route_metrics_are_rejected() {
        let network = network(10);
        let mut invalid = route(&network, &["shared"]);
        invalid.metrics_per_unit = RouteMetrics::default();
        let result = select_shared_capacity_batch(
            &network,
            &[candidate("d1", 1, vec![invalid])],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        );
        assert!(matches!(
            result,
            Err(BatchAllocationError::InvalidCandidateRoute {
                reason: "declared metrics do not match the route edges",
                ..
            })
        ));
    }

    #[test]
    fn search_budget_exhaustion_returns_no_partial_optimum() {
        let network = network(10);
        let (shared, _) = routes_for(&network);
        let result = select_shared_capacity_batch(
            &network,
            &[candidate("d1", 1, vec![shared])],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits {
                max_search_steps: 1,
            },
        );
        assert_eq!(
            result,
            Err(BatchAllocationError::SearchBudgetExceeded { limit: 1 })
        );
    }

    #[test]
    fn overlapping_quantity_overflow_fails_closed() {
        let mut network = Network::new(["A", "B"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "only-edge",
            "A",
            "B",
            Mode::Rail,
            u64::MAX,
            QuantityUnit::Tonnes,
            super::super::EdgeMetrics::default(),
        ));
        let only_route = route(&network, &["only-edge"]);
        let result = select_shared_capacity_batch(
            &network,
            &[
                candidate("d1", u64::MAX, vec![only_route.clone()]),
                candidate("d2", 1, vec![only_route]),
            ],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        );
        assert_eq!(result, Err(BatchAllocationError::ArithmeticOverflow));
    }

    #[test]
    fn candidate_coverage_is_echoed_and_limits_optimality_scope() {
        let network = network(7);
        let (shared, alternate) = routes_for(&network);
        let report = select_shared_capacity_batch(
            &network,
            &[
                candidate_with_completeness(
                    "d1",
                    4,
                    vec![shared.clone(), alternate.clone()],
                    CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch {
                        search_domain_id: "synthetic-network-v1-simple-paths".to_owned(),
                    },
                ),
                candidate_with_completeness(
                    "d2",
                    4,
                    vec![shared],
                    CandidatePoolCompleteness::ParetoFrontierOnly,
                ),
            ],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        )
        .unwrap();

        assert_eq!(
            report.optimality_scope,
            BatchOptimalityScope::ExactOverSuppliedCandidatePools
        );
        assert_eq!(
            report.candidate_completeness,
            vec![
                DemandCandidateCompleteness {
                    demand_id: "d1".to_owned(),
                    completeness: CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch {
                        search_domain_id: "synthetic-network-v1-simple-paths".to_owned(),
                    },
                },
                DemandCandidateCompleteness {
                    demand_id: "d2".to_owned(),
                    completeness: CandidatePoolCompleteness::ParetoFrontierOnly,
                },
            ]
        );
    }

    #[test]
    fn exhaustive_domain_scope_is_explicitly_caller_asserted() {
        let network = network(10);
        let (shared, alternate) = routes_for(&network);
        let report = select_shared_capacity_batch(
            &network,
            &[candidate_with_completeness(
                "d1",
                1,
                vec![shared, alternate],
                CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch {
                    search_domain_id: "synthetic-network-v1-simple-paths".to_owned(),
                },
            )],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        )
        .unwrap();

        assert_eq!(
            report.optimality_scope,
            BatchOptimalityScope::ExactOverCallerAssertedSearchDomains
        );
        assert_eq!(report.candidate_completeness.len(), 1);
        assert_eq!(report.candidate_completeness[0].demand_id, "d1");
    }

    #[test]
    fn exhaustive_candidate_pool_requires_search_domain_identity() {
        let network = network(10);
        let (shared, _) = routes_for(&network);
        let result = select_shared_capacity_batch(
            &network,
            &[candidate_with_completeness(
                "d1",
                1,
                vec![shared],
                CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch {
                    search_domain_id: "  ".to_owned(),
                },
            )],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        );

        assert_eq!(
            result,
            Err(BatchAllocationError::InvalidCandidateSearchDomain {
                demand_id: "d1".to_owned(),
            })
        );
    }

    #[test]
    fn duplicate_demand_identity_is_rejected() {
        let network = network(10);
        let (shared, _) = routes_for(&network);
        let result = select_shared_capacity_batch(
            &network,
            &[
                candidate("same", 1, vec![shared.clone()]),
                candidate("same", 1, vec![shared]),
            ],
            BatchObjective::OperatorCostMinorUnits,
            BatchAllocationLimits::default(),
        );
        assert_eq!(
            result,
            Err(BatchAllocationError::DuplicateDemandIdentity(
                "same".to_owned()
            ))
        );
    }
}
