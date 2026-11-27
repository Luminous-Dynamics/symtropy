#![forbid(unsafe_code)]

//! Explicit discrete-time inventory, handling, transit and service semantics.
//!
//! This research profile compiles one conserved commodity into the shared
//! single-commodity flow kernel. Arrivals occur at the start of their declared
//! period. Transfers have at least one period of transit. Inventory carry,
//! outbound handling and beneficiary issue are distinct capacity gates.
//!
//! This compiler does not authenticate inputs, infer event ordering, model
//! cross-node common resources, perform grade conversion, or authorize actions.

use single_commodity_flow_kernel::{FlowEdge, FlowGraph, FlowNode, MAX_EDGES, MAX_NODES};
use std::collections::BTreeSet;

pub const TEMPORAL_PROFILE_SCHEMA: &str =
    "safrb-temporal-inventory-handling-arrival-at-period-start-v0.1";

const SOURCE_NODE: &str = "safrb-temporal/source";
const SINK_NODE: &str = "safrb-temporal/sink";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalNode {
    pub node_id: String,
    /// Maximum inventory allowed to remain at the end of each period.
    /// This v0.1 profile holds the limit constant throughout the horizon.
    pub storage_capacity: u64,
    /// Aggregate per-period capacity for all outbound transfer departures.
    /// This v0.1 profile holds the limit constant throughout the horizon.
    pub handling_capacity: u64,
    /// Aggregate per-period capacity for beneficiary/service withdrawals.
    /// This v0.1 profile holds the limit constant throughout the horizon.
    pub service_issue_capacity: u64,
    /// Stock physically present at the opening boundary of period zero.
    pub opening_inventory: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalReceipt {
    pub receipt_id: String,
    pub node_id: String,
    /// Receipts are available at the start of this period.
    pub available_period: u64,
    pub quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalTransfer {
    pub transfer_id: String,
    pub from_node_id: String,
    pub to_node_id: String,
    pub departure_period: u64,
    /// Zero-period and same-period transfers are outside this profile.
    pub transit_periods: u64,
    pub route_capacity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalService {
    pub service_id: String,
    pub node_id: String,
    pub period: u64,
    /// Maximum amount this service may receive in the modeled period.
    /// This is a request/ceiling, not a mandatory minimum-delivery constraint.
    pub requested_quantity: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TemporalProfile {
    pub schema_version: String,
    pub case_id: String,
    pub commodity_ref: String,
    pub capacity_unit_ref: String,
    pub quantity_boundary_ref: String,
    /// Immutable reference defining period zero, period duration and calendar.
    /// The compiler binds this reference into the graph digest but does not
    /// dereference or authenticate it.
    pub period_timebase_ref: String,
    /// The final modeled period. Periods are the inclusive range 0..=horizon.
    pub horizon_last_period: u64,
    pub nodes: Vec<TemporalNode>,
    pub receipts: Vec<TemporalReceipt>,
    pub transfers: Vec<TemporalTransfer>,
    pub services: Vec<TemporalService>,
}

fn state_node_id(node_id: &str, period: u64) -> String {
    format!("safrb-temporal/state/{}/{node_id}/{period}", node_id.len())
}

fn handling_node_id(node_id: &str, period: u64) -> String {
    format!(
        "safrb-temporal/handling/{}/{node_id}/{period}",
        node_id.len()
    )
}

fn issue_node_id(node_id: &str, period: u64) -> String {
    format!("safrb-temporal/issue/{}/{node_id}/{period}", node_id.len())
}

fn storage_edge_id(node_id: &str, period: u64) -> String {
    format!("storage-carry/{}/{node_id}/{period}", node_id.len())
}

fn handling_edge_id(node_id: &str, period: u64) -> String {
    format!("handling-gate/{}/{node_id}/{period}", node_id.len())
}

fn issue_edge_id(node_id: &str, period: u64) -> String {
    format!("service-issue-gate/{}/{node_id}/{period}", node_id.len())
}

fn source_edge_id(kind: &str, id: &str) -> String {
    format!("{kind}/{}/{id}", id.len())
}

fn require_nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("{label} must not be empty"));
    }
    Ok(())
}

fn ensure_unique<'a>(values: impl IntoIterator<Item = &'a str>, label: &str) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        require_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(format!("duplicate {label}: {value}"));
        }
    }
    Ok(())
}

fn metadata_ref(kind: &str, id: &str) -> String {
    format!("{TEMPORAL_PROFILE_SCHEMA}/{kind}/{id}")
}

fn bound_quantity_boundary(profile: &TemporalProfile) -> String {
    let boundary = &profile.quantity_boundary_ref;
    let schema = &profile.schema_version;
    let case_id = &profile.case_id;
    let timebase = &profile.period_timebase_ref;
    format!(
        "{}:{}|{}:{}|{}:{}|{}:{}",
        boundary.len(),
        boundary,
        schema.len(),
        schema,
        case_id.len(),
        case_id,
        timebase.len(),
        timebase
    )
}

/// Compile the explicit period-start temporal profile to a graph-bound model.
///
/// Semantics:
/// - Opening inventory and receipts enter their node's inventory state at the
///   start of the declared period.
/// - A node's carry edge is independently capped by end-of-period storage.
/// - All outbound transfers in one node-period share one handling gate.
/// - All service withdrawals in one node-period share a separate issue gate.
/// - A transfer departs through the origin handling gate and arrives at the
///   destination inventory state after at least one whole period.
/// - No transfer can arrive and depart in the same period.
///
/// The graph proves only the compiled single-commodity network. It does not
/// prove that supplied capacities, event times or service obligations are true.
pub fn compile_temporal_profile(profile: &TemporalProfile) -> Result<FlowGraph, String> {
    require_nonempty(&profile.case_id, "case_id")?;
    require_nonempty(&profile.commodity_ref, "commodity_ref")?;
    require_nonempty(&profile.capacity_unit_ref, "capacity_unit_ref")?;
    require_nonempty(&profile.quantity_boundary_ref, "quantity_boundary_ref")?;
    require_nonempty(&profile.period_timebase_ref, "period_timebase_ref")?;
    if profile.schema_version != TEMPORAL_PROFILE_SCHEMA {
        return Err(format!(
            "unsupported temporal profile schema: {}",
            profile.schema_version
        ));
    }
    if profile.nodes.is_empty() {
        return Err("temporal profile must declare at least one physical node".to_string());
    }

    ensure_unique(
        profile.nodes.iter().map(|node| node.node_id.as_str()),
        "node_id",
    )?;
    ensure_unique(
        profile
            .receipts
            .iter()
            .map(|receipt| receipt.receipt_id.as_str()),
        "receipt_id",
    )?;
    ensure_unique(
        profile
            .transfers
            .iter()
            .map(|transfer| transfer.transfer_id.as_str()),
        "transfer_id",
    )?;
    ensure_unique(
        profile
            .services
            .iter()
            .map(|service| service.service_id.as_str()),
        "service_id",
    )?;

    let node_ids = profile
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<BTreeSet<_>>();

    for node in &profile.nodes {
        if node.opening_inventory > node.storage_capacity {
            return Err(format!(
                "opening inventory exceeds usable storage for node {}",
                node.node_id
            ));
        }
    }

    for receipt in &profile.receipts {
        if !node_ids.contains(receipt.node_id.as_str()) {
            return Err(format!(
                "receipt {} references unknown node {}",
                receipt.receipt_id, receipt.node_id
            ));
        }
        if receipt.available_period > profile.horizon_last_period {
            return Err(format!(
                "receipt {} is outside the declared horizon",
                receipt.receipt_id
            ));
        }
    }

    for transfer in &profile.transfers {
        if !node_ids.contains(transfer.from_node_id.as_str())
            || !node_ids.contains(transfer.to_node_id.as_str())
        {
            return Err(format!(
                "transfer {} references an unknown endpoint",
                transfer.transfer_id
            ));
        }
        if transfer.transit_periods == 0 {
            return Err(format!(
                "transfer {} has zero transit; same-period movement is unsupported",
                transfer.transfer_id
            ));
        }
        let arrival_period = transfer
            .departure_period
            .checked_add(transfer.transit_periods)
            .ok_or_else(|| {
                format!(
                    "transfer {} arrival period overflowed u64",
                    transfer.transfer_id
                )
            })?;
        if transfer.departure_period >= profile.horizon_last_period
            || arrival_period > profile.horizon_last_period
        {
            return Err(format!(
                "transfer {} departs or arrives outside the declared horizon",
                transfer.transfer_id
            ));
        }
    }

    for service in &profile.services {
        if !node_ids.contains(service.node_id.as_str()) {
            return Err(format!(
                "service {} references unknown node {}",
                service.service_id, service.node_id
            ));
        }
        if service.period > profile.horizon_last_period {
            return Err(format!(
                "service {} is outside the declared horizon",
                service.service_id
            ));
        }
    }

    let period_count_u64 = profile
        .horizon_last_period
        .checked_add(1)
        .ok_or_else(|| "temporal horizon period count overflowed u64".to_string())?;
    let period_count = usize::try_from(period_count_u64)
        .map_err(|_| "temporal horizon does not fit addressable memory".to_string())?;
    let horizon_periods = usize::try_from(profile.horizon_last_period)
        .map_err(|_| "temporal horizon does not fit addressable memory".to_string())?;

    let state_count = profile
        .nodes
        .len()
        .checked_mul(period_count)
        .ok_or_else(|| "temporal state count overflowed usize".to_string())?;
    let graph_node_count = state_count
        .checked_mul(3)
        .and_then(|count| count.checked_add(2))
        .ok_or_else(|| "temporal graph node count overflowed usize".to_string())?;
    if graph_node_count > MAX_NODES {
        return Err(format!(
            "temporal expansion requires {graph_node_count} nodes; kernel limit is {MAX_NODES}"
        ));
    }

    let carry_edge_count = profile
        .nodes
        .len()
        .checked_mul(horizon_periods)
        .ok_or_else(|| "temporal carry edge count overflowed usize".to_string())?;
    let graph_edge_count = state_count
        .checked_mul(2)
        .and_then(|count| count.checked_add(carry_edge_count))
        .and_then(|count| count.checked_add(profile.nodes.len()))
        .and_then(|count| count.checked_add(profile.receipts.len()))
        .and_then(|count| count.checked_add(profile.transfers.len()))
        .and_then(|count| count.checked_add(profile.services.len()))
        .ok_or_else(|| "temporal graph edge count overflowed usize".to_string())?;
    if graph_edge_count > MAX_EDGES {
        return Err(format!(
            "temporal expansion requires {graph_edge_count} edges; kernel limit is {MAX_EDGES}"
        ));
    }

    let mut physical_nodes = profile.nodes.iter().collect::<Vec<_>>();
    physical_nodes.sort_by(|left, right| left.node_id.cmp(&right.node_id));

    let mut graph_nodes = Vec::with_capacity(graph_node_count);
    graph_nodes.push(FlowNode::new(SOURCE_NODE));
    graph_nodes.push(FlowNode::new(SINK_NODE));

    for node in &physical_nodes {
        for period in 0..period_count_u64 {
            graph_nodes.push(FlowNode::new(state_node_id(&node.node_id, period)));
            graph_nodes.push(FlowNode::new(handling_node_id(&node.node_id, period)));
            graph_nodes.push(FlowNode::new(issue_node_id(&node.node_id, period)));
        }
    }

    let mut graph_edges = Vec::with_capacity(graph_edge_count);
    for node in &physical_nodes {
        for period in 0..period_count_u64 {
            let state = state_node_id(&node.node_id, period);
            let handling = handling_node_id(&node.node_id, period);
            let issue = issue_node_id(&node.node_id, period);

            let handling_id = handling_edge_id(&node.node_id, period);
            graph_edges.push(FlowEdge::new(
                handling_id.clone(),
                state.clone(),
                handling,
                node.handling_capacity,
                Some(metadata_ref("handling", &handling_id)),
            ));

            let issue_id = issue_edge_id(&node.node_id, period);
            graph_edges.push(FlowEdge::new(
                issue_id.clone(),
                state.clone(),
                issue,
                node.service_issue_capacity,
                Some(metadata_ref("service-issue", &issue_id)),
            ));

            if period < profile.horizon_last_period {
                let next_state = state_node_id(&node.node_id, period + 1);
                let carry_id = storage_edge_id(&node.node_id, period);
                graph_edges.push(FlowEdge::new(
                    carry_id.clone(),
                    state,
                    next_state,
                    node.storage_capacity,
                    Some(metadata_ref("storage-carry", &carry_id)),
                ));
            }
        }

        let opening_id = source_edge_id("opening-inventory", &node.node_id);
        graph_edges.push(FlowEdge::new(
            opening_id.clone(),
            SOURCE_NODE,
            state_node_id(&node.node_id, 0),
            node.opening_inventory,
            Some(metadata_ref("opening-inventory", &opening_id)),
        ));
    }

    let mut receipts = profile.receipts.iter().collect::<Vec<_>>();
    receipts.sort_by(|left, right| left.receipt_id.cmp(&right.receipt_id));
    for receipt in receipts {
        let edge_id = source_edge_id("receipt", &receipt.receipt_id);
        graph_edges.push(FlowEdge::new(
            edge_id.clone(),
            SOURCE_NODE,
            state_node_id(&receipt.node_id, receipt.available_period),
            receipt.quantity,
            Some(metadata_ref("receipt", &edge_id)),
        ));
    }

    let mut transfers = profile.transfers.iter().collect::<Vec<_>>();
    transfers.sort_by(|left, right| left.transfer_id.cmp(&right.transfer_id));
    for transfer in transfers {
        let edge_id = source_edge_id("transfer", &transfer.transfer_id);
        let arrival_period = transfer
            .departure_period
            .checked_add(transfer.transit_periods)
            .ok_or_else(|| {
                format!(
                    "transfer {} arrival period overflowed u64",
                    transfer.transfer_id
                )
            })?;
        graph_edges.push(FlowEdge::new(
            edge_id.clone(),
            handling_node_id(&transfer.from_node_id, transfer.departure_period),
            state_node_id(&transfer.to_node_id, arrival_period),
            transfer.route_capacity,
            Some(metadata_ref("transfer", &edge_id)),
        ));
    }

    let mut services = profile.services.iter().collect::<Vec<_>>();
    services.sort_by(|left, right| left.service_id.cmp(&right.service_id));
    for service in services {
        let edge_id = source_edge_id("service", &service.service_id);
        graph_edges.push(FlowEdge::new(
            edge_id.clone(),
            issue_node_id(&service.node_id, service.period),
            SINK_NODE,
            service.requested_quantity,
            Some(metadata_ref("service", &edge_id)),
        ));
    }

    Ok(FlowGraph {
        commodity_ref: profile.commodity_ref.clone(),
        capacity_unit_ref: profile.capacity_unit_ref.clone(),
        quantity_boundary_ref: bound_quantity_boundary(profile),
        nodes: graph_nodes,
        edges: graph_edges,
        source: SOURCE_NODE.to_string(),
        sink: SINK_NODE.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use single_commodity_flow_kernel::{FlowCertificateBundle, FlowResult};

    fn node(
        id: &str,
        storage_capacity: u64,
        handling_capacity: u64,
        service_issue_capacity: u64,
    ) -> TemporalNode {
        TemporalNode {
            node_id: id.to_string(),
            storage_capacity,
            handling_capacity,
            service_issue_capacity,
            opening_inventory: 0,
        }
    }

    fn profile(
        horizon_last_period: u64,
        nodes: Vec<TemporalNode>,
        receipts: Vec<TemporalReceipt>,
        transfers: Vec<TemporalTransfer>,
        services: Vec<TemporalService>,
    ) -> TemporalProfile {
        TemporalProfile {
            schema_version: TEMPORAL_PROFILE_SCHEMA.to_string(),
            case_id: "synthetic-temporal-profile-test".to_string(),
            commodity_ref: "diesel".to_string(),
            capacity_unit_ref: "litre".to_string(),
            quantity_boundary_ref: "deliverable-fuel-at-beneficiary-boundary".to_string(),
            period_timebase_ref: "synthetic:period-zero-and-one-period".to_string(),
            horizon_last_period,
            nodes,
            receipts,
            transfers,
            services,
        }
    }

    fn verified_bundle(profile: &TemporalProfile) -> FlowCertificateBundle {
        let graph = compile_temporal_profile(profile).unwrap();
        let result = graph.solve().unwrap();
        let bundle = FlowCertificateBundle::new(graph, result).unwrap();
        bundle.verify().unwrap();
        crate::temporal_verify::verify_temporal_certificate(profile, &bundle).unwrap();
        bundle
    }

    fn verified_result(profile: &TemporalProfile) -> FlowResult {
        verified_bundle(profile).result
    }

    fn flow_for(result: &FlowResult, edge_id: &str) -> u64 {
        result
            .edge_flows
            .iter()
            .find(|edge| edge.id == edge_id)
            .unwrap_or_else(|| panic!("missing edge witness {edge_id}"))
            .flow
    }

    fn next_small(state: &mut u64, bound: u64) -> u64 {
        *state = state
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        (*state >> 32) % bound
    }

    fn exhaustive_parallel_service_max(limits: &[u64], shared_ceiling: u64) -> u64 {
        fn enumerate(
            limits: &[u64],
            index: usize,
            total: u64,
            shared_ceiling: u64,
            best: &mut u64,
        ) {
            if index == limits.len() {
                if total <= shared_ceiling {
                    *best = (*best).max(total);
                }
                return;
            }
            for amount in 0..=limits[index] {
                let Some(next_total) = total.checked_add(amount) else {
                    continue;
                };
                if next_total <= shared_ceiling {
                    enumerate(
                        limits,
                        index + 1,
                        next_total,
                        shared_ceiling,
                        best,
                    );
                }
            }
        }

        let mut best = 0;
        enumerate(limits, 0, 0, shared_ceiling, &mut best);
        best
    }

    #[test]
    fn seeded_temporal_profiles_match_exhaustive_physical_service_oracle() {
        // This oracle is expressed in physical quantities rather than by
        // rebuilding or calling the temporal graph compiler. Every transfer
        // has a distinct destination, so the exhaustive variables are each
        // beneficiary's delivered quantity. Their sum is constrained by the
        // source stock and one shared origin handling limit.
        for seed in 0..250_u64 {
            let mut state = 0x5AFB_2026_D15C_A11_u64 ^ seed;
            let beneficiary_count = 2 + next_small(&mut state, 3) as usize;
            let supply = next_small(&mut state, 7);
            let handling = next_small(&mut state, 7);
            let shared_ceiling = supply.min(handling);

            let mut nodes = vec![node("origin-yard", 0, handling, 0)];
            let receipts = vec![TemporalReceipt {
                receipt_id: format!("receipt-{seed}"),
                node_id: "origin-yard".to_string(),
                available_period: 0,
                quantity: supply,
            }];
            let mut transfers = Vec::new();
            let mut services = Vec::new();
            let mut exhaustive_limits = Vec::new();

            for index in 0..beneficiary_count {
                let destination_id = format!("beneficiary-{index}");
                let route_capacity = next_small(&mut state, 4);
                let issue_capacity = next_small(&mut state, 4);
                let requested_quantity = next_small(&mut state, 4);
                nodes.push(node(&destination_id, 0, 0, issue_capacity));
                transfers.push(TemporalTransfer {
                    transfer_id: format!("route-{seed}-{index}"),
                    from_node_id: "origin-yard".to_string(),
                    to_node_id: destination_id.clone(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity,
                });
                services.push(TemporalService {
                    service_id: format!("service-{seed}-{index}"),
                    node_id: destination_id,
                    period: 1,
                    requested_quantity,
                });
                exhaustive_limits.push(
                    route_capacity.min(issue_capacity).min(requested_quantity),
                );
            }

            let mut model = profile(1, nodes, receipts, transfers, services);
            model.case_id = format!("seeded-temporal-physical-oracle-{seed}");

            let expected = exhaustive_parallel_service_max(
                &exhaustive_limits,
                shared_ceiling,
            );
            let bundle = verified_bundle(&model);
            assert_eq!(
                bundle.result.max_flow,
                expected,
                "seed {seed}: expected physical-oracle delivery {expected}, got {}",
                bundle.result.max_flow
            );
        }
    }

    #[test]
    fn cross_dock_can_exceed_end_of_period_storage_without_exceeding_handling() {
        let mut cross_dock = node("cross-dock", 500, 1_000, 0);
        cross_dock.opening_inventory = 0;
        let destination = node("beneficiary-depot", 500, 0, 1_000);
        let model = profile(
            1,
            vec![cross_dock, destination],
            vec![TemporalReceipt {
                receipt_id: "receipt-1000".to_string(),
                node_id: "cross-dock".to_string(),
                available_period: 0,
                quantity: 1_000,
            }],
            vec![TemporalTransfer {
                transfer_id: "cross-dock-to-depot".to_string(),
                from_node_id: "cross-dock".to_string(),
                to_node_id: "beneficiary-depot".to_string(),
                departure_period: 0,
                transit_periods: 1,
                route_capacity: 1_000,
            }],
            vec![TemporalService {
                service_id: "depot-service".to_string(),
                node_id: "beneficiary-depot".to_string(),
                period: 1,
                requested_quantity: 1_000,
            }],
        );

        let result = verified_result(&model);
        assert_eq!(result.max_flow, 1_000);
        assert_eq!(result.min_cut.capacity, 1_000);
        assert_eq!(flow_for(&result, &storage_edge_id("cross-dock", 0)), 0);
        assert_eq!(flow_for(&result, &handling_edge_id("cross-dock", 0)), 1_000);
    }

    #[test]
    fn carried_inventory_is_limited_by_storage_not_handling_capacity() {
        let yard = node("yard", 500, 1_000, 0);
        let depot = node("depot", 500, 0, 1_000);
        let model = profile(
            2,
            vec![yard, depot],
            vec![TemporalReceipt {
                receipt_id: "receipt-700".to_string(),
                node_id: "yard".to_string(),
                available_period: 0,
                quantity: 700,
            }],
            vec![TemporalTransfer {
                transfer_id: "yard-to-depot".to_string(),
                from_node_id: "yard".to_string(),
                to_node_id: "depot".to_string(),
                departure_period: 1,
                transit_periods: 1,
                route_capacity: 700,
            }],
            vec![TemporalService {
                service_id: "depot-demand".to_string(),
                node_id: "depot".to_string(),
                period: 2,
                requested_quantity: 700,
            }],
        );

        let result = verified_result(&model);
        assert_eq!(result.max_flow, 500);
        assert_eq!(flow_for(&result, &storage_edge_id("yard", 0)), 500);
    }

    #[test]
    fn outbound_handling_capacity_limits_all_routes_from_a_node_period() {
        let yard = node("yard", 2_000, 400, 0);
        let depot = node("depot", 0, 0, 1_000);
        let model = profile(
            1,
            vec![yard, depot],
            vec![TemporalReceipt {
                receipt_id: "receipt-1000".to_string(),
                node_id: "yard".to_string(),
                available_period: 0,
                quantity: 1_000,
            }],
            vec![TemporalTransfer {
                transfer_id: "yard-route".to_string(),
                from_node_id: "yard".to_string(),
                to_node_id: "depot".to_string(),
                departure_period: 0,
                transit_periods: 1,
                route_capacity: 1_000,
            }],
            vec![TemporalService {
                service_id: "depot-service".to_string(),
                node_id: "depot".to_string(),
                period: 1,
                requested_quantity: 1_000,
            }],
        );

        let result = verified_result(&model);
        assert_eq!(result.max_flow, 400);
        assert_eq!(flow_for(&result, &handling_edge_id("yard", 0)), 400);
    }

    #[test]
    fn parallel_departures_share_one_node_handling_gate() {
        let yard = node("yard", 1_000, 500, 0);
        let depot_a = node("depot-a", 0, 0, 1_000);
        let depot_b = node("depot-b", 0, 0, 1_000);
        let model = profile(
            1,
            vec![yard, depot_a, depot_b],
            vec![TemporalReceipt {
                receipt_id: "receipt-800".to_string(),
                node_id: "yard".to_string(),
                available_period: 0,
                quantity: 800,
            }],
            vec![
                TemporalTransfer {
                    transfer_id: "route-a".to_string(),
                    from_node_id: "yard".to_string(),
                    to_node_id: "depot-a".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: 500,
                },
                TemporalTransfer {
                    transfer_id: "route-b".to_string(),
                    from_node_id: "yard".to_string(),
                    to_node_id: "depot-b".to_string(),
                    departure_period: 0,
                    transit_periods: 1,
                    route_capacity: 500,
                },
            ],
            vec![
                TemporalService {
                    service_id: "service-a".to_string(),
                    node_id: "depot-a".to_string(),
                    period: 1,
                    requested_quantity: 500,
                },
                TemporalService {
                    service_id: "service-b".to_string(),
                    node_id: "depot-b".to_string(),
                    period: 1,
                    requested_quantity: 500,
                },
            ],
        );

        let result = verified_result(&model);
        assert_eq!(result.max_flow, 500);
        assert_eq!(flow_for(&result, &handling_edge_id("yard", 0)), 500);
        assert_eq!(
            flow_for(&result, &source_edge_id("service", "service-a"))
                + flow_for(&result, &source_edge_id("service", "service-b")),
            500
        );
    }

    #[test]
    fn route_arrival_is_not_available_before_its_declared_period() {
        let source = node("source-yard", 1_000, 1_000, 0);
        let destination = node("destination", 0, 0, 1_000);
        let base = profile(
            2,
            vec![source, destination],
            vec![TemporalReceipt {
                receipt_id: "receipt".to_string(),
                node_id: "source-yard".to_string(),
                available_period: 0,
                quantity: 100,
            }],
            vec![TemporalTransfer {
                transfer_id: "two-period-route".to_string(),
                from_node_id: "source-yard".to_string(),
                to_node_id: "destination".to_string(),
                departure_period: 0,
                transit_periods: 2,
                route_capacity: 100,
            }],
            vec![TemporalService {
                service_id: "too-early".to_string(),
                node_id: "destination".to_string(),
                period: 1,
                requested_quantity: 100,
            }],
        );

        let early_result = verified_result(&base);
        assert_eq!(early_result.max_flow, 0);

        let mut available = base;
        available.services.push(TemporalService {
            service_id: "on-time".to_string(),
            node_id: "destination".to_string(),
            period: 2,
            requested_quantity: 100,
        });
        let on_time_result = verified_result(&available);
        assert_eq!(on_time_result.max_flow, 100);
        assert_eq!(
            flow_for(&on_time_result, &source_edge_id("service", "on-time")),
            100
        );
        assert_eq!(
            flow_for(&on_time_result, &source_edge_id("service", "too-early")),
            0
        );
    }

    #[test]
    fn validated_opening_inventory_can_fund_same_period_service_issue() {
        let mut yard = node("yard", 100, 0, 100);
        yard.opening_inventory = 80;
        let model = profile(
            0,
            vec![yard],
            vec![],
            vec![],
            vec![TemporalService {
                service_id: "same-period-service".to_string(),
                node_id: "yard".to_string(),
                period: 0,
                requested_quantity: 80,
            }],
        );

        let result = verified_result(&model);
        assert_eq!(result.max_flow, 80);
        assert_eq!(
            flow_for(
                &result,
                &source_edge_id("opening-inventory", "yard")
            ),
            80
        );
    }

    #[test]
    fn opening_inventory_above_usable_storage_is_rejected() {
        let mut yard = node("yard", 500, 1_000, 0);
        yard.opening_inventory = 600;
        let model = profile(0, vec![yard], vec![], vec![], vec![]);
        assert!(
            compile_temporal_profile(&model)
                .unwrap_err()
                .contains("opening inventory exceeds usable storage")
        );
    }

    #[test]
    fn same_period_transfer_is_rejected_instead_of_inferred() {
        let model = profile(
            1,
            vec![node("a", 100, 100, 0), node("b", 100, 0, 100)],
            vec![],
            vec![TemporalTransfer {
                transfer_id: "zero-transit".to_string(),
                from_node_id: "a".to_string(),
                to_node_id: "b".to_string(),
                departure_period: 0,
                transit_periods: 0,
                route_capacity: 100,
            }],
            vec![],
        );
        assert!(
            compile_temporal_profile(&model)
                .unwrap_err()
                .contains("same-period movement is unsupported")
        );
    }

    #[test]
    fn extreme_horizon_and_transit_arithmetic_fail_closed() {
        let mut huge_horizon = profile(u64::MAX, vec![node("a", 0, 0, 0)], vec![], vec![], vec![]);
        assert!(
            compile_temporal_profile(&huge_horizon)
                .unwrap_err()
                .contains("period count overflowed")
        );

        huge_horizon.horizon_last_period = u64::MAX - 1;
        huge_horizon.transfers.push(TemporalTransfer {
            transfer_id: "overflowing-arrival".to_string(),
            from_node_id: "a".to_string(),
            to_node_id: "a".to_string(),
            departure_period: u64::MAX - 1,
            transit_periods: 2,
            route_capacity: 1,
        });
        assert!(
            compile_temporal_profile(&huge_horizon)
                .unwrap_err()
                .contains("arrival period overflowed")
        );
    }

    #[test]
    fn expansion_budget_is_checked_before_graph_allocation() {
        let model = profile(
            1_000,
            vec![node("a", 1, 1, 1), node("b", 1, 1, 1)],
            vec![],
            vec![],
            vec![],
        );
        assert!(
            compile_temporal_profile(&model)
                .unwrap_err()
                .contains("kernel limit")
        );
    }

    #[test]
    fn case_and_timebase_are_bound_to_graph_identity() {
        let original = profile(
            0,
            vec![node("yard", 100, 100, 100)],
            vec![TemporalReceipt {
                receipt_id: "receipt".to_string(),
                node_id: "yard".to_string(),
                available_period: 0,
                quantity: 10,
            }],
            vec![],
            vec![TemporalService {
                service_id: "service".to_string(),
                node_id: "yard".to_string(),
                period: 0,
                requested_quantity: 10,
            }],
        );
        let original_bytes = compile_temporal_profile(&original)
            .unwrap()
            .canonical_bytes()
            .unwrap();

        let mut other_case = original.clone();
        other_case.case_id = "different-case".to_string();
        let other_case_bytes = compile_temporal_profile(&other_case)
            .unwrap()
            .canonical_bytes()
            .unwrap();
        assert_ne!(original_bytes, other_case_bytes);

        let mut other_timebase = original;
        other_timebase.period_timebase_ref = "synthetic:alternate-calendar".to_string();
        let other_timebase_bytes = compile_temporal_profile(&other_timebase)
            .unwrap()
            .canonical_bytes()
            .unwrap();
        assert_ne!(original_bytes, other_timebase_bytes);
    }

    #[test]
    fn independent_verifier_rejects_certificate_for_mutated_receipt_capacity() {
        let mut model = profile(
            0,
            vec![node("yard", 100, 100, 100)],
            vec![TemporalReceipt {
                receipt_id: "receipt".to_string(),
                node_id: "yard".to_string(),
                available_period: 0,
                quantity: 10,
            }],
            vec![],
            vec![TemporalService {
                service_id: "service".to_string(),
                node_id: "yard".to_string(),
                period: 0,
                requested_quantity: 10,
            }],
        );
        let bundle = verified_bundle(&model);
        model.receipts[0].quantity = 11;
        assert!(
            crate::temporal_verify::verify_temporal_certificate(&model, &bundle)
                .unwrap_err()
                .contains("graph mapping mismatch")
        );
    }

    #[test]
    fn independent_verifier_rejects_certificate_from_other_timebase() {
        let mut model = profile(
            0,
            vec![node("yard", 100, 100, 100)],
            vec![TemporalReceipt {
                receipt_id: "receipt".to_string(),
                node_id: "yard".to_string(),
                available_period: 0,
                quantity: 10,
            }],
            vec![],
            vec![TemporalService {
                service_id: "service".to_string(),
                node_id: "yard".to_string(),
                period: 0,
                requested_quantity: 10,
            }],
        );
        let bundle = verified_bundle(&model);
        model.period_timebase_ref = "synthetic:other-period-zero".to_string();
        assert!(
            crate::temporal_verify::verify_temporal_certificate(&model, &bundle)
                .unwrap_err()
                .contains("model-context binding mismatch")
        );
    }

    #[test]
    fn profile_input_order_does_not_change_canonical_graph_bytes() {
        let mut first = profile(
            1,
            vec![node("b", 10, 10, 10), node("a", 10, 10, 10)],
            vec![
                TemporalReceipt {
                    receipt_id: "receipt-b".to_string(),
                    node_id: "b".to_string(),
                    available_period: 0,
                    quantity: 2,
                },
                TemporalReceipt {
                    receipt_id: "receipt-a".to_string(),
                    node_id: "a".to_string(),
                    available_period: 0,
                    quantity: 3,
                },
            ],
            vec![],
            vec![],
        );
        let left = compile_temporal_profile(&first)
            .unwrap()
            .canonical_bytes()
            .unwrap();
        first.nodes.reverse();
        first.receipts.reverse();
        first.transfers.reverse();
        first.services.reverse();
        let right = compile_temporal_profile(&first)
            .unwrap()
            .canonical_bytes()
            .unwrap();
        assert_eq!(left, right);
    }
}
