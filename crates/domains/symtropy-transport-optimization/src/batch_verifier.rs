// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Independent integrity verification for shared-capacity batch reports.
//!
//! This verifier checks candidate membership, route facts, aggregate arithmetic,
//! shared-edge conservation, and honest optimality-scope metadata. It does not
//! independently prove that a supplied candidate pool is exhaustive or that an
//! allocator's plan is globally optimal; that requires a separately trusted
//! search-domain definition and an independent optimization/reference solver.

use super::{
    AggregateMetrics, BatchAllocationLimits, BatchOptimalityScope, BatchObjective,
    CandidatePoolCompleteness, Demand, DemandCandidateCompleteness, DemandRouteCandidates,
    Edge, EdgeMetrics, EdgeOperation, ModelError, Mode, Network, QuantityUnit, Route,
    RouteMetrics, SharedCapacityBatchReport, SharedEdgeCapacityUse,
};
use std::collections::{BTreeMap, BTreeSet};
use std::fmt;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum BatchReportVerificationError {
    InvalidInput(&'static str),
    InvalidReport(&'static str),
    InvalidDemandIdentity(String),
    InvalidCandidateRoute {
        demand_id: String,
        reason: &'static str,
    },
    ArithmeticOverflow,
    Model(ModelError),
}

impl fmt::Display for BatchReportVerificationError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(reason) => write!(f, "batch verifier input is invalid: {reason}"),
            Self::InvalidReport(reason) => write!(f, "batch report is invalid: {reason}"),
            Self::InvalidDemandIdentity(id) => {
                write!(f, "batch verifier found duplicate demand identity {id}")
            }
            Self::InvalidCandidateRoute { demand_id, reason } => {
                write!(f, "route for demand {demand_id} is invalid: {reason}")
            }
            Self::ArithmeticOverflow => write!(f, "batch verification arithmetic overflowed"),
            Self::Model(error) => write!(f, "batch verification model validation failed: {error}"),
        }
    }
}

impl std::error::Error for BatchReportVerificationError {}

/// Independently checks a batch report against its network and declared input.
///
/// The checks are deliberately recomputed from network edges and demand facts
/// rather than trusting the report's totals or the allocator's capacity-use
/// table. A successful result establishes report integrity and feasibility for
/// the supplied static-edge model only. It is not an optimality certificate,
/// a reservation, a live-capacity assertion, or a dispatch authorization.
pub fn verify_shared_capacity_batch_report(
    network: &Network,
    candidate_sets: &[DemandRouteCandidates],
    objective: BatchObjective,
    limits: BatchAllocationLimits,
    report: &SharedCapacityBatchReport,
) -> Result<(), BatchReportVerificationError> {
    network
        .validate()
        .map_err(BatchReportVerificationError::Model)?;
    if candidate_sets.is_empty() {
        return Err(BatchReportVerificationError::InvalidInput(
            "candidate batch is empty",
        ));
    }
    if limits.max_search_steps == 0 {
        return Err(BatchReportVerificationError::InvalidInput(
            "search-step limit must be positive",
        ));
    }
    if report.objective != objective {
        return Err(BatchReportVerificationError::InvalidReport(
            "reported objective differs from requested objective",
        ));
    }
    if report.search_steps == 0 || report.search_steps > limits.max_search_steps {
        return Err(BatchReportVerificationError::InvalidReport(
            "search-step count is zero or exceeds the declared limit",
        ));
    }

    let edges_by_id = network
        .edges
        .iter()
        .map(|edge| (edge.id.as_str(), edge))
        .collect::<BTreeMap<_, _>>();
    let mut demands_by_id = BTreeMap::new();

    // Verify every declared candidate route, not just the route that was chosen.
    // Otherwise an invalid or duplicated input pool could be hidden by the report.
    for candidate_set in candidate_sets {
        candidate_set
            .demand
            .validate(network)
            .map_err(BatchReportVerificationError::Model)?;
        let demand_id = candidate_set.demand.id.as_str();
        if demands_by_id.insert(demand_id, candidate_set).is_some() {
            return Err(BatchReportVerificationError::InvalidDemandIdentity(
                candidate_set.demand.id.clone(),
            ));
        }
        if candidate_set.routes.is_empty() {
            return Err(BatchReportVerificationError::InvalidInput(
                "a demand has no candidate routes",
            ));
        }
        if let CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch {
            search_domain_id,
        } = &candidate_set.completeness
        {
            if search_domain_id.trim().is_empty() {
                return Err(BatchReportVerificationError::InvalidInput(
                    "an exhaustive candidate claim lacks a search-domain ID",
                ));
            }
        }

        let mut route_signatures = BTreeSet::new();
        for route in &candidate_set.routes {
            verify_route_independently(
                &candidate_set.demand,
                route,
                &edges_by_id,
            )
            .map_err(|reason| BatchReportVerificationError::InvalidCandidateRoute {
                demand_id: candidate_set.demand.id.clone(),
                reason,
            })?;
            if !route_signatures.insert(route.edge_ids.clone()) {
                return Err(BatchReportVerificationError::InvalidInput(
                    "a demand repeats a candidate route signature",
                ));
            }
            calculate_route_totals(route, candidate_set.demand.quantity)?;
        }
    }

    let expected_completeness = demands_by_id
        .values()
        .map(|candidate_set| DemandCandidateCompleteness {
            demand_id: candidate_set.demand.id.clone(),
            completeness: candidate_set.completeness.clone(),
        })
        .collect::<Vec<_>>();
    if report.candidate_completeness != expected_completeness {
        return Err(BatchReportVerificationError::InvalidReport(
            "candidate coverage metadata does not match the declared inputs",
        ));
    }
    let expected_scope = if demands_by_id.values().all(|candidate_set| {
        matches!(
            &candidate_set.completeness,
            CandidatePoolCompleteness::ExhaustiveWithinDeclaredSearch { .. }
        )
    }) {
        BatchOptimalityScope::ExactOverCallerAssertedSearchDomains
    } else {
        BatchOptimalityScope::ExactOverSuppliedCandidatePools
    };
    if report.optimality_scope != expected_scope {
        return Err(BatchReportVerificationError::InvalidReport(
            "optimality scope overstates or contradicts candidate-pool coverage",
        ));
    }

    if report.assignments.len() != demands_by_id.len() {
        return Err(BatchReportVerificationError::InvalidReport(
            "report does not assign exactly one route to every demand",
        ));
    }

    let mut total_metrics = zero_totals();
    let mut edge_usage: BTreeMap<String, (u64, QuantityUnit, u64, Vec<String>)> =
        BTreeMap::new();
    let mut seen_assignment_ids = BTreeSet::new();

    // The expected demand map is sorted; the result contract also requires
    // assignments to be sorted by demand identity.
    for (assignment, (demand_id, candidate_set)) in
        report.assignments.iter().zip(demands_by_id.iter())
    {
        if assignment.demand_id != *demand_id
            || !seen_assignment_ids.insert(assignment.demand_id.as_str())
        {
            return Err(BatchReportVerificationError::InvalidReport(
                "assignments are not unique and sorted by demand ID",
            ));
        }
        if !candidate_set.routes.contains(&assignment.route) {
            return Err(BatchReportVerificationError::InvalidReport(
                "selected route is not present in the supplied candidate pool",
            ));
        }

        // Recompute route totals and edge usage independently from the report.
        let route_totals = calculate_route_totals(
            &assignment.route,
            candidate_set.demand.quantity,
        )?;
        add_totals(&mut total_metrics, &route_totals)?;

        for edge_id in &assignment.route.edge_ids {
            let edge = edges_by_id.get(edge_id.as_str()).ok_or(
                BatchReportVerificationError::InvalidReport(
                    "selected route references an unknown edge",
                ),
            )?;
            let entry = edge_usage.entry(edge_id.clone()).or_insert_with(|| {
                (0, edge.capacity_unit, edge.capacity, Vec::new())
            });
            if entry.1 != candidate_set.demand.unit || entry.1 != edge.capacity_unit {
                return Err(BatchReportVerificationError::InvalidReport(
                    "shared edge usage combines incompatible capacity units",
                ));
            }
            entry.0 = checked_add(entry.0, candidate_set.demand.quantity)?;
            entry.3.push(candidate_set.demand.id.clone());
        }
    }

    if report.total_metrics != total_metrics {
        return Err(BatchReportVerificationError::InvalidReport(
            "aggregate metrics do not equal independently recomputed totals",
        ));
    }
    let expected_objective_value = objective_value(objective, &total_metrics);
    if report.objective_value != expected_objective_value {
        return Err(BatchReportVerificationError::InvalidReport(
            "objective value does not equal the selected metric in aggregate totals",
        ));
    }

    let mut expected_capacity_uses = Vec::with_capacity(edge_usage.len());
    for (edge_id, (quantity_used, quantity_unit, capacity_limit, demand_ids)) in edge_usage {
        if quantity_used > capacity_limit {
            return Err(BatchReportVerificationError::InvalidReport(
                "selected routes exceed a shared edge's static capacity",
            ));
        }
        expected_capacity_uses.push(SharedEdgeCapacityUse {
            edge_id,
            quantity_unit,
            quantity_used,
            capacity_limit,
            demand_ids,
        });
    }
    if report.capacity_uses != expected_capacity_uses {
        return Err(BatchReportVerificationError::InvalidReport(
            "edge-capacity usage does not match independently recomputed conservation",
        ));
    }
    Ok(())
}

/// Independent route walk; this intentionally does not call the allocator's
/// private route validator.
fn verify_route_independently(
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
        return Err("empty route does not join the demand endpoints");
    }

    let mut current_node = demand.origin.as_str();
    let mut current_mode: Option<Mode> = None;
    let mut transfer_count = 0_u32;
    let mut metrics = RouteMetrics::default();
    let mut seen_edges = BTreeSet::new();
    let mut seen_states = BTreeSet::from([(demand.origin.clone(), None)]);

    for (position, edge_id) in route.edge_ids.iter().enumerate() {
        if !seen_edges.insert(edge_id.as_str()) {
            return Err("route repeats an edge");
        }
        let edge = edges_by_id
            .get(edge_id.as_str())
            .copied()
            .ok_or("route references an unknown edge")?;
        if edge.from != current_node {
            return Err("route edges are not connected in listed order");
        }
        if edge.capacity_unit != demand.unit || edge.capacity < demand.quantity {
            return Err("edge unit or per-demand capacity is incompatible");
        }

        let next_mode = match edge.operation {
            EdgeOperation::Movement { mode } => {
                if current_mode.is_some_and(|previous| previous != mode) {
                    return Err("transport mode changes without a transfer edge");
                }
                mode
            }
            EdgeOperation::Transfer { from, to } => {
                if current_mode != Some(from) {
                    return Err("transfer edge does not match the previous mode");
                }
                transfer_count = transfer_count
                    .checked_add(1)
                    .ok_or("transfer count overflows")?;
                if demand
                    .max_transfers
                    .is_some_and(|maximum| transfer_count > maximum)
                {
                    return Err("route exceeds the demand transfer limit");
                }
                to
            }
        };

        metrics = add_edge_metrics(metrics, edge.metrics)?;
        if demand
            .max_elapsed_seconds
            .is_some_and(|maximum| metrics.elapsed_seconds > maximum)
        {
            return Err("route exceeds the demand time limit");
        }
        if edge.to == demand.origin {
            return Err("route returns to its origin");
        }
        if edge.to == demand.destination && position + 1 != route.edge_ids.len() {
            return Err("route continues after reaching its destination");
        }
        current_node = edge.to.as_str();
        if !seen_states.insert((current_node.to_owned(), Some(next_mode))) {
            return Err("route repeats a node/mode state");
        }
        current_mode = Some(next_mode);
    }

    if current_node != demand.destination.as_str() {
        return Err("route does not terminate at the demand destination");
    }
    if transfer_count != route.transfer_count {
        return Err("declared transfer count differs from route edges");
    }
    if metrics != route.metrics_per_unit {
        return Err("route metrics differ from the values independently summed from edges");
    }
    Ok(())
}

fn add_edge_metrics(
    current: RouteMetrics,
    edge: EdgeMetrics,
) -> Result<RouteMetrics, &'static str> {
    Ok(RouteMetrics {
        distance_metres: current.distance_metres.checked_add(edge.distance_metres).ok_or("route metric overflow")?,
        elapsed_seconds: current.elapsed_seconds.checked_add(edge.elapsed_seconds).ok_or("route metric overflow")?,
        operator_cost_minor_per_unit: current.operator_cost_minor_per_unit.checked_add(edge.operator_cost_minor_per_unit).ok_or("route metric overflow")?,
        user_cash_cost_minor_per_unit: current.user_cash_cost_minor_per_unit.checked_add(edge.user_cash_cost_minor_per_unit).ok_or("route metric overflow")?,
        external_cost_minor_per_unit: current.external_cost_minor_per_unit.checked_add(edge.external_cost_minor_per_unit).ok_or("route metric overflow")?,
        energy_wh_per_unit: current.energy_wh_per_unit.checked_add(edge.energy_wh_per_unit).ok_or("route metric overflow")?,
        ghg_grams_co2e_per_unit: current.ghg_grams_co2e_per_unit.checked_add(edge.ghg_grams_co2e_per_unit).ok_or("route metric overflow")?,
        safety_risk_micro_units_per_unit: current.safety_risk_micro_units_per_unit.checked_add(edge.safety_risk_micro_units_per_unit).ok_or("route metric overflow")?,
        disruption_risk_micro_units_per_unit: current.disruption_risk_micro_units_per_unit.checked_add(edge.disruption_risk_micro_units_per_unit).ok_or("route metric overflow")?,
    })
}

fn calculate_route_totals(
    route: &Route,
    quantity: u64,
) -> Result<AggregateMetrics, BatchReportVerificationError> {
    if quantity == 0 {
        return Err(BatchReportVerificationError::InvalidInput(
            "demand quantity must be positive",
        ));
    }
    let scaled = |value: u64| {
        value
            .checked_mul(quantity)
            .ok_or(BatchReportVerificationError::ArithmeticOverflow)
    };
    Ok(AggregateMetrics {
        // These remain per-trip path measures in the current model.
        distance_metres: route.metrics_per_unit.distance_metres,
        elapsed_seconds: route.metrics_per_unit.elapsed_seconds,
        operator_cost_minor_units: scaled(route.metrics_per_unit.operator_cost_minor_per_unit)?,
        user_cash_cost_minor_units: scaled(route.metrics_per_unit.user_cash_cost_minor_per_unit)?,
        external_cost_minor_units: scaled(route.metrics_per_unit.external_cost_minor_per_unit)?,
        energy_wh: scaled(route.metrics_per_unit.energy_wh_per_unit)?,
        ghg_grams_co2e: scaled(route.metrics_per_unit.ghg_grams_co2e_per_unit)?,
        safety_risk_micro_units: scaled(route.metrics_per_unit.safety_risk_micro_units_per_unit)?,
        disruption_risk_micro_units: scaled(route.metrics_per_unit.disruption_risk_micro_units_per_unit)?,
    })
}

fn zero_totals() -> AggregateMetrics {
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

fn add_totals(
    left: &mut AggregateMetrics,
    right: &AggregateMetrics,
) -> Result<(), BatchReportVerificationError> {
    left.distance_metres = checked_add(left.distance_metres, right.distance_metres)?;
    left.elapsed_seconds = checked_add(left.elapsed_seconds, right.elapsed_seconds)?;
    left.operator_cost_minor_units =
        checked_add(left.operator_cost_minor_units, right.operator_cost_minor_units)?;
    left.user_cash_cost_minor_units =
        checked_add(left.user_cash_cost_minor_units, right.user_cash_cost_minor_units)?;
    left.external_cost_minor_units =
        checked_add(left.external_cost_minor_units, right.external_cost_minor_units)?;
    left.energy_wh = checked_add(left.energy_wh, right.energy_wh)?;
    left.ghg_grams_co2e = checked_add(left.ghg_grams_co2e, right.ghg_grams_co2e)?;
    left.safety_risk_micro_units =
        checked_add(left.safety_risk_micro_units, right.safety_risk_micro_units)?;
    left.disruption_risk_micro_units =
        checked_add(left.disruption_risk_micro_units, right.disruption_risk_micro_units)?;
    Ok(())
}

fn checked_add(
    left: u64,
    right: u64,
) -> Result<u64, BatchReportVerificationError> {
    left.checked_add(right)
        .ok_or(BatchReportVerificationError::ArithmeticOverflow)
}

fn objective_value(objective: BatchObjective, totals: &AggregateMetrics) -> u64 {
    match objective {
        BatchObjective::OperatorCostMinorUnits => totals.operator_cost_minor_units,
        BatchObjective::UserCashCostMinorUnits => totals.user_cash_cost_minor_units,
        BatchObjective::ExternalCostMinorUnits => totals.external_cost_minor_units,
        BatchObjective::EnergyWattHours => totals.energy_wh,
        BatchObjective::GreenhouseGasGramsCo2e => totals.ghg_grams_co2e,
        BatchObjective::SafetyRiskMicroUnits => totals.safety_risk_micro_units,
        BatchObjective::DisruptionRiskMicroUnits => totals.disruption_risk_micro_units,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        select_shared_capacity_batch, EdgeMetrics as NetworkEdgeMetrics,
    };

    fn metrics(cost: u64) -> NetworkEdgeMetrics {
        NetworkEdgeMetrics {
            distance_metres: 1,
            elapsed_seconds: 1,
            operator_cost_minor_per_unit: cost,
            ..Default::default()
        }
    }

    fn network() -> Network {
        let mut network = Network::new(["A", "B", "C"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "shared",
            "A",
            "B",
            Mode::Rail,
            7,
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

    fn route(network: &Network, ids: &[&str]) -> Route {
        let mut metrics = RouteMetrics::default();
        for id in ids {
            let edge = network.edges.iter().find(|edge| edge.id == *id).unwrap();
            metrics = add_edge_metrics(metrics, edge.metrics).unwrap();
        }
        Route {
            edge_ids: ids.iter().map(|id| (*id).to_owned()).collect(),
            transfer_count: 0,
            metrics_per_unit: metrics,
        }
    }

    fn candidate(id: &str, quantity: u64, routes: Vec<Route>) -> DemandRouteCandidates {
        DemandRouteCandidates {
            demand: demand(id, quantity),
            routes,
            completeness: CandidatePoolCompleteness::Unknown,
        }
    }

    fn plan() -> (
        Network,
        Vec<DemandRouteCandidates>,
        BatchAllocationLimits,
        SharedCapacityBatchReport,
    ) {
        let network = network();
        let shared = route(&network, &["shared"]);
        let alternate = route(&network, &["alt-a", "alt-b"]);
        let candidates = vec![
            candidate("d1", 4, vec![shared.clone(), alternate.clone()]),
            candidate("d2", 4, vec![shared.clone(), alternate.clone()]),
        ];
        let limits = BatchAllocationLimits::default();
        let report = select_shared_capacity_batch(
            &network,
            &candidates,
            BatchObjective::OperatorCostMinorUnits,
            limits,
        )
        .unwrap();
        (network, candidates, limits, report)
    }

    #[test]
    fn independently_accepts_an_intact_feasible_report() {
        let (network, candidates, limits, report) = plan();
        assert_eq!(
            verify_shared_capacity_batch_report(
                &network,
                &candidates,
                BatchObjective::OperatorCostMinorUnits,
                limits,
                &report,
            ),
            Ok(())
        );
    }

    #[test]
    fn rejects_forged_edge_utilization() {
        let (network, candidates, limits, mut report) = plan();
        report.capacity_uses[0].quantity_used += 1;
        assert!(matches!(
            verify_shared_capacity_batch_report(
                &network,
                &candidates,
                BatchObjective::OperatorCostMinorUnits,
                limits,
                &report
            ),
            Err(BatchReportVerificationError::InvalidReport(
                "edge-capacity usage does not match independently recomputed conservation"
            ))
        ));
    }

    #[test]
    fn rejects_incorrect_aggregate_objective_and_metrics() {
        let (network, candidates, limits, mut report) = plan();
        report.total_metrics.operator_cost_minor_units += 1;
        assert!(matches!(
            verify_shared_capacity_batch_report(
                &network,
                &candidates,
                BatchObjective::OperatorCostMinorUnits,
                limits,
                &report
            ),
            Err(BatchReportVerificationError::InvalidReport(
                "aggregate metrics do not equal independently recomputed totals"
            ))
        ));
    }

    #[test]
    fn rejects_route_not_present_in_declared_candidate_pool() {
        let (network, _, limits, _) = plan();
        let candidates = vec![candidate(
            "d1",
            1,
            vec![route(&network, &["shared"])],
        )];
        let mut report = select_shared_capacity_batch(
            &network,
            &candidates,
            BatchObjective::OperatorCostMinorUnits,
            limits,
        )
        .unwrap();
        report.assignments[0].route = route(&network, &["alt-a", "alt-b"]);
        assert!(matches!(
            verify_shared_capacity_batch_report(
                &network,
                &candidates,
                BatchObjective::OperatorCostMinorUnits,
                limits,
                &report
            ),
            Err(BatchReportVerificationError::InvalidReport(
                "selected route is not present in the supplied candidate pool"
            ))
        ));
    }

    #[test]
    fn rejects_optimality_scope_that_overstates_candidate_coverage() {
        let (network, candidates, limits, mut report) = plan();
        report.optimality_scope = BatchOptimalityScope::ExactOverCallerAssertedSearchDomains;
        assert!(matches!(
            verify_shared_capacity_batch_report(
                &network,
                &candidates,
                BatchObjective::OperatorCostMinorUnits,
                limits,
                &report
            ),
            Err(BatchReportVerificationError::InvalidReport(
                "optimality scope overstates or contradicts candidate-pool coverage"
            ))
        ));
    }

    #[test]
    fn rejects_capacity_overuse_even_if_capacity_table_is_forged() {
        let mut network = Network::new(["A", "B"], "ZAR").unwrap();
        network.add_edge(Edge::movement(
            "only",
            "A",
            "B",
            Mode::Rail,
            5,
            QuantityUnit::Tonnes,
            metrics(1),
        ));
        let selected_route = route(&network, &["only"]);
        let candidates = vec![
            candidate("d1", 4, vec![selected_route.clone()]),
            candidate("d2", 4, vec![selected_route]),
        ];
        let limits = BatchAllocationLimits::default();
        let forged = SharedCapacityBatchReport {
            objective: BatchObjective::OperatorCostMinorUnits,
            objective_value: 8,
            optimality_scope: BatchOptimalityScope::ExactOverSuppliedCandidatePools,
            candidate_completeness: vec![
                DemandCandidateCompleteness {
                    demand_id: "d1".to_owned(),
                    completeness: CandidatePoolCompleteness::Unknown,
                },
                DemandCandidateCompleteness {
                    demand_id: "d2".to_owned(),
                    completeness: CandidatePoolCompleteness::Unknown,
                },
            ],
            total_metrics: AggregateMetrics {
                distance_metres: 2,
                elapsed_seconds: 2,
                operator_cost_minor_units: 8,
                user_cash_cost_minor_units: 0,
                external_cost_minor_units: 0,
                energy_wh: 0,
                ghg_grams_co2e: 0,
                safety_risk_micro_units: 0,
                disruption_risk_micro_units: 0,
            },
            assignments: vec![
                super::super::BatchRouteAssignment {
                    demand_id: "d1".to_owned(),
                    route: route(&network, &["only"]),
                },
                super::super::BatchRouteAssignment {
                    demand_id: "d2".to_owned(),
                    route: route(&network, &["only"]),
                },
            ],
            capacity_uses: vec![SharedEdgeCapacityUse {
                edge_id: "only".to_owned(),
                quantity_unit: QuantityUnit::Tonnes,
                quantity_used: 5,
                capacity_limit: 5,
                demand_ids: vec!["d1".to_owned(), "d2".to_owned()],
            }],
            search_steps: 1,
        };
        assert!(matches!(
            verify_shared_capacity_batch_report(
                &network,
                &candidates,
                BatchObjective::OperatorCostMinorUnits,
                limits,
                &forged
            ),
            Err(BatchReportVerificationError::InvalidReport(
                "selected routes exceed a shared edge's static capacity"
            ))
        ));
    }
}
