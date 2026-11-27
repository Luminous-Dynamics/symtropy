#![forbid(unsafe_code)]

//! Independent domain-level verifier for the v0.1 temporal flow profile.
//!
//! This module intentionally reconstructs node identities, edge identities,
//! capacities, time transitions and physical balance equations separately
//! from the temporal profile compiler. It never invokes the compiler or solver.
//! The embedded kernel bundle is independently verified first; this module
//! then checks that its graph and witness faithfully encode the profile.

use crate::temporal_profile::{
    TemporalProfile, TemporalService, TemporalTransfer, TEMPORAL_PROFILE_SCHEMA,
};
use single_commodity_flow_kernel::{
    EdgeFlow, FlowCertificateBundle, FlowEdge, MAX_EDGES, MAX_NODES,
};
use std::collections::{BTreeMap, BTreeSet};

const VERIFY_SOURCE: &str = "safrb-temporal/source";
const VERIFY_SINK: &str = "safrb-temporal/sink";

#[derive(Debug, Clone, PartialEq, Eq)]
struct ExpectedEdge {
    from: String,
    to: String,
    capacity: u64,
    metadata_ref: Option<String>,
}

fn state_id(node_id: &str, period: u64) -> String {
    format!(
        "safrb-temporal/state/{}/{node_id}/{period}",
        node_id.len()
    )
}

fn handling_id(node_id: &str, period: u64) -> String {
    format!(
        "safrb-temporal/handling/{}/{node_id}/{period}",
        node_id.len()
    )
}

fn issue_id(node_id: &str, period: u64) -> String {
    format!("safrb-temporal/issue/{}/{node_id}/{period}", node_id.len())
}

fn carry_edge_id(node_id: &str, period: u64) -> String {
    format!("storage-carry/{}/{node_id}/{period}", node_id.len())
}

fn handling_edge_id(node_id: &str, period: u64) -> String {
    format!("handling-gate/{}/{node_id}/{period}", node_id.len())
}

fn issue_edge_id(node_id: &str, period: u64) -> String {
    format!(
        "service-issue-gate/{}/{node_id}/{period}",
        node_id.len()
    )
}

fn source_edge_id(kind: &str, id: &str) -> String {
    format!("{kind}/{}/{id}", id.len())
}

fn metadata(kind: &str, id: &str) -> Option<String> {
    Some(format!("{TEMPORAL_PROFILE_SCHEMA}/{kind}/{id}"))
}

fn bound_boundary(profile: &TemporalProfile) -> String {
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

fn check_nonempty(value: &str, label: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        return Err(format!("temporal verifier: {label} must not be empty"));
    }
    Ok(())
}

fn check_unique<'a>(
    values: impl IntoIterator<Item = &'a str>,
    label: &str,
) -> Result<(), String> {
    let mut seen = BTreeSet::new();
    for value in values {
        check_nonempty(value, label)?;
        if !seen.insert(value) {
            return Err(format!("temporal verifier: duplicate {label}: {value}"));
        }
    }
    Ok(())
}

fn add_expected_edge(
    edges: &mut BTreeMap<String, ExpectedEdge>,
    id: String,
    from: String,
    to: String,
    capacity: u64,
    kind: &str,
) -> Result<(), String> {
    let edge = ExpectedEdge {
        from,
        to,
        capacity,
        metadata_ref: metadata(kind, &id),
    };
    if edges.insert(id.clone(), edge).is_some() {
        return Err(format!("temporal verifier: duplicate expected edge {id}"));
    }
    Ok(())
}

fn checked_add(total: u64, value: u64, label: &str) -> Result<u64, String> {
    total
        .checked_add(value)
        .ok_or_else(|| format!("temporal verifier: {label} overflowed u64"))
}

fn witness_flow(flows: &BTreeMap<String, u64>, edge_id: &str) -> Result<u64, String> {
    flows
        .get(edge_id)
        .copied()
        .ok_or_else(|| format!("temporal verifier: witness omitted edge {edge_id}"))
}

fn sum_service_flows(
    services: &[TemporalService],
    flows: &BTreeMap<String, u64>,
) -> Result<u64, String> {
    let mut total = 0_u64;
    for service in services {
        let id = source_edge_id("service", &service.service_id);
        total = checked_add(
            total,
            witness_flow(flows, &id)?,
            "total beneficiary delivery",
        )?;
    }
    Ok(total)
}

fn transfer_arrival(transfer: &TemporalTransfer) -> Result<u64, String> {
    transfer
        .departure_period
        .checked_add(transfer.transit_periods)
        .ok_or_else(|| {
            format!(
                "temporal verifier: transfer {} arrival period overflowed u64",
                transfer.transfer_id
            )
        })
}

/// Verify the graph mapping, complete flow witness, and period-by-period
/// physical balance equations without calling the temporal compiler or solver.
///
/// The profile and certificate must be presented together. A certificate
/// from another case, period timebase, model version, unit, or quantity
/// boundary is rejected even when its internal max-flow/min-cut proof is valid.
pub fn verify_temporal_certificate(
    profile: &TemporalProfile,
    bundle: &FlowCertificateBundle,
) -> Result<(), String> {
    bundle
        .verify()
        .map_err(|error| format!("temporal verifier: invalid kernel bundle: {error}"))?;

    check_nonempty(&profile.case_id, "case_id")?;
    check_nonempty(&profile.commodity_ref, "commodity_ref")?;
    check_nonempty(&profile.capacity_unit_ref, "capacity_unit_ref")?;
    check_nonempty(&profile.quantity_boundary_ref, "quantity_boundary_ref")?;
    check_nonempty(&profile.period_timebase_ref, "period_timebase_ref")?;

    if profile.schema_version != TEMPORAL_PROFILE_SCHEMA {
        return Err("temporal verifier: unsupported profile schema".to_string());
    }
    if profile.nodes.is_empty() {
        return Err("temporal verifier: profile has no physical nodes".to_string());
    }

    check_unique(
        profile.nodes.iter().map(|node| node.node_id.as_str()),
        "node_id",
    )?;
    check_unique(
        profile
            .receipts
            .iter()
            .map(|receipt| receipt.receipt_id.as_str()),
        "receipt_id",
    )?;
    check_unique(
        profile
            .transfers
            .iter()
            .map(|transfer| transfer.transfer_id.as_str()),
        "transfer_id",
    )?;
    check_unique(
        profile
            .services
            .iter()
            .map(|service| service.service_id.as_str()),
        "service_id",
    )?;

    let known_nodes = profile
        .nodes
        .iter()
        .map(|node| node.node_id.as_str())
        .collect::<BTreeSet<_>>();

    for node in &profile.nodes {
        if node.opening_inventory > node.storage_capacity {
            return Err(format!(
                "temporal verifier: opening inventory exceeds storage at {}",
                node.node_id
            ));
        }
    }

    for receipt in &profile.receipts {
        if !known_nodes.contains(receipt.node_id.as_str())
            || receipt.available_period > profile.horizon_last_period
        {
            return Err(format!(
                "temporal verifier: invalid node or period for receipt {}",
                receipt.receipt_id
            ));
        }
    }

    for transfer in &profile.transfers {
        if !known_nodes.contains(transfer.from_node_id.as_str())
            || !known_nodes.contains(transfer.to_node_id.as_str())
            || transfer.transit_periods == 0
            || transfer.departure_period >= profile.horizon_last_period
        {
            return Err(format!(
                "temporal verifier: invalid endpoint or transit for transfer {}",
                transfer.transfer_id
            ));
        }
        if transfer_arrival(transfer)? > profile.horizon_last_period {
            return Err(format!(
                "temporal verifier: transfer {} arrives outside horizon",
                transfer.transfer_id
            ));
        }
    }

    for service in &profile.services {
        if !known_nodes.contains(service.node_id.as_str())
            || service.period > profile.horizon_last_period
        {
            return Err(format!(
                "temporal verifier: invalid node or period for service {}",
                service.service_id
            ));
        }
    }

    let period_count_u64 = profile
        .horizon_last_period
        .checked_add(1)
        .ok_or_else(|| "temporal verifier: period count overflowed u64".to_string())?;
    let period_count = usize::try_from(period_count_u64)
        .map_err(|_| "temporal verifier: period count does not fit usize".to_string())?;
    let carry_period_count = usize::try_from(profile.horizon_last_period)
        .map_err(|_| "temporal verifier: horizon does not fit usize".to_string())?;
    let state_count = profile
        .nodes
        .len()
        .checked_mul(period_count)
        .ok_or_else(|| "temporal verifier: state count overflowed usize".to_string())?;
    let node_count = state_count
        .checked_mul(3)
        .and_then(|count| count.checked_add(2))
        .ok_or_else(|| "temporal verifier: graph node count overflowed usize".to_string())?;
    if node_count > MAX_NODES {
        return Err("temporal verifier: graph exceeds kernel node ceiling".to_string());
    }
    let edge_count = state_count
        .checked_mul(2)
        .and_then(|count| {
            profile
                .nodes
                .len()
                .checked_mul(carry_period_count)
                .and_then(|carry| count.checked_add(carry))
        })
        .and_then(|count| count.checked_add(profile.nodes.len()))
        .and_then(|count| count.checked_add(profile.receipts.len()))
        .and_then(|count| count.checked_add(profile.transfers.len()))
        .and_then(|count| count.checked_add(profile.services.len()))
        .ok_or_else(|| "temporal verifier: graph edge count overflowed usize".to_string())?;
    if edge_count > MAX_EDGES {
        return Err("temporal verifier: graph exceeds kernel edge ceiling".to_string());
    }

    let mut expected_nodes = BTreeSet::from([VERIFY_SOURCE.to_string(), VERIFY_SINK.to_string()]);
    let mut expected_edges = BTreeMap::new();

    for node in &profile.nodes {
        for period in 0..period_count_u64 {
            let state = state_id(&node.node_id, period);
            let handling = handling_id(&node.node_id, period);
            let issue = issue_id(&node.node_id, period);
            expected_nodes.insert(state.clone());
            expected_nodes.insert(handling.clone());
            expected_nodes.insert(issue.clone());

            let handling_edge = handling_edge_id(&node.node_id, period);
            add_expected_edge(
                &mut expected_edges,
                handling_edge,
                state.clone(),
                handling,
                node.handling_capacity,
                "handling",
            )?;

            let issue_edge = issue_edge_id(&node.node_id, period);
            add_expected_edge(
                &mut expected_edges,
                issue_edge,
                state.clone(),
                issue,
                node.service_issue_capacity,
                "service-issue",
            )?;

            if period < profile.horizon_last_period {
                let carry_edge = carry_edge_id(&node.node_id, period);
                add_expected_edge(
                    &mut expected_edges,
                    carry_edge,
                    state,
                    state_id(&node.node_id, period + 1),
                    node.storage_capacity,
                    "storage-carry",
                )?;
            }
        }

        let opening_edge = source_edge_id("opening-inventory", &node.node_id);
        add_expected_edge(
            &mut expected_edges,
            opening_edge.clone(),
            VERIFY_SOURCE.to_string(),
            state_id(&node.node_id, 0),
            node.opening_inventory,
            "opening-inventory",
        )?;
    }

    for receipt in &profile.receipts {
        let edge_id = source_edge_id("receipt", &receipt.receipt_id);
        add_expected_edge(
            &mut expected_edges,
            edge_id.clone(),
            VERIFY_SOURCE.to_string(),
            state_id(&receipt.node_id, receipt.available_period),
            receipt.quantity,
            "receipt",
        )?;
    }

    for transfer in &profile.transfers {
        let edge_id = source_edge_id("transfer", &transfer.transfer_id);
        add_expected_edge(
            &mut expected_edges,
            edge_id.clone(),
            handling_id(&transfer.from_node_id, transfer.departure_period),
            state_id(&transfer.to_node_id, transfer_arrival(transfer)?),
            transfer.route_capacity,
            "transfer",
        )?;
    }

    for service in &profile.services {
        let edge_id = source_edge_id("service", &service.service_id);
        add_expected_edge(
            &mut expected_edges,
            edge_id.clone(),
            issue_id(&service.node_id, service.period),
            VERIFY_SINK.to_string(),
            service.required_quantity,
            "service",
        )?;
    }

    let graph = &bundle.graph;
    if graph.source != VERIFY_SOURCE || graph.sink != VERIFY_SINK {
        return Err("temporal verifier: graph source/sink identity mismatch".to_string());
    }
    if graph.commodity_ref != profile.commodity_ref
        || graph.capacity_unit_ref != profile.capacity_unit_ref
        || graph.quantity_boundary_ref != bound_boundary(profile)
    {
        return Err("temporal verifier: graph model-context binding mismatch".to_string());
    }

    let actual_nodes = graph
        .nodes
        .iter()
        .map(|node| node.id.clone())
        .collect::<BTreeSet<_>>();
    if actual_nodes.len() != graph.nodes.len() || actual_nodes != expected_nodes {
        return Err("temporal verifier: graph node identities do not match profile".to_string());
    }

    let mut actual_edges = BTreeMap::new();
    for edge in &graph.edges {
        if actual_edges.insert(edge.id.as_str(), edge).is_some() {
            return Err(format!(
                "temporal verifier: duplicate graph edge identity {}",
                edge.id
            ));
        }
    }
    if actual_edges.len() != expected_edges.len() {
        return Err("temporal verifier: graph edge count does not match profile".to_string());
    }
    for (edge_id, expected) in &expected_edges {
        let actual = actual_edges
            .get(edge_id.as_str())
            .ok_or_else(|| format!("temporal verifier: graph omitted edge {edge_id}"))?;
        if actual.from != expected.from
            || actual.to != expected.to
            || actual.capacity != expected.capacity
            || actual.metadata_ref != expected.metadata_ref
        {
            return Err(format!(
                "temporal verifier: graph mapping mismatch for edge {edge_id}"
            ));
        }
    }

    let mut flows = BTreeMap::new();
    for flow in &bundle.result.edge_flows {
        if flows.insert(flow.id.clone(), flow.flow).is_some() {
            return Err(format!(
                "temporal verifier: duplicate witness edge {}",
                flow.id
            ));
        }
        let graph_edge = actual_edges
            .get(flow.id.as_str())
            .ok_or_else(|| format!("temporal verifier: witness has unknown edge {}", flow.id))?;
        check_flow_arc(flow, graph_edge)?;
    }
    if flows.len() != expected_edges.len() {
        return Err("temporal verifier: witness does not cover every graph edge".to_string());
    }

    let mut total_source = 0_u64;
    for node in &profile.nodes {
        let opening_id = source_edge_id("opening-inventory", &node.node_id);
        total_source = checked_add(
            total_source,
            witness_flow(&flows, &opening_id)?,
            "total opening stock flow",
        )?;
    }
    for receipt in &profile.receipts {
        let edge_id = source_edge_id("receipt", &receipt.receipt_id);
        total_source = checked_add(
            total_source,
            witness_flow(&flows, &edge_id)?,
            "total receipt flow",
        )?;
    }

    for node in &profile.nodes {
        for period in 0..period_count_u64 {
            let mut incoming = 0_u64;
            if period == 0 {
                let opening_id = source_edge_id("opening-inventory", &node.node_id);
                incoming = checked_add(
                    incoming,
                    witness_flow(&flows, &opening_id)?,
                    "opening inventory balance",
                )?;
            } else {
                let previous_carry = carry_edge_id(&node.node_id, period - 1);
                incoming = checked_add(
                    incoming,
                    witness_flow(&flows, &previous_carry)?,
                    "carried inventory balance",
                )?;
            }

            for receipt in profile.receipts.iter().filter(|receipt| {
                receipt.node_id == node.node_id && receipt.available_period == period
            }) {
                let receipt_edge = source_edge_id("receipt", &receipt.receipt_id);
                incoming = checked_add(
                    incoming,
                    witness_flow(&flows, &receipt_edge)?,
                    "receipt-period balance",
                )?;
            }

            for transfer in profile.transfers.iter().filter(|transfer| {
                transfer.to_node_id == node.node_id
                    && transfer_arrival(transfer).ok() == Some(period)
            }) {
                let transfer_edge = source_edge_id("transfer", &transfer.transfer_id);
                incoming = checked_add(
                    incoming,
                    witness_flow(&flows, &transfer_edge)?,
                    "transfer-arrival balance",
                )?;
            }

            let handling_edge = handling_edge_id(&node.node_id, period);
            let issue_edge = issue_edge_id(&node.node_id, period);
            let mut outgoing = witness_flow(&flows, &handling_edge)?;
            let issue_flow = witness_flow(&flows, &issue_edge)?;
            outgoing = checked_add(outgoing, issue_flow, "node-period withdrawals")?;
            if period < profile.horizon_last_period {
                let carry_edge = carry_edge_id(&node.node_id, period);
                outgoing = checked_add(
                    outgoing,
                    witness_flow(&flows, &carry_edge)?,
                    "node-period carried inventory",
                )?;
            }
            if incoming != outgoing {
                return Err(format!(
                    "temporal verifier: inventory balance failed at node {} period {}: in={} out={}",
                    node.node_id, period, incoming, outgoing
                ));
            }

            let mut declared_departures = 0_u64;
            for transfer in profile.transfers.iter().filter(|transfer| {
                transfer.from_node_id == node.node_id && transfer.departure_period == period
            }) {
                let transfer_edge = source_edge_id("transfer", &transfer.transfer_id);
                declared_departures = checked_add(
                    declared_departures,
                    witness_flow(&flows, &transfer_edge)?,
                    "period handling departures",
                )?;
            }
            if declared_departures != witness_flow(&flows, &handling_edge)? {
                return Err(format!(
                    "temporal verifier: handling gate does not reconcile at node {} period {}",
                    node.node_id, period
                ));
            }

            let mut declared_service = 0_u64;
            for service in profile.services.iter().filter(|service| {
                service.node_id == node.node_id && service.period == period
            }) {
                let service_edge = source_edge_id("service", &service.service_id);
                declared_service = checked_add(
                    declared_service,
                    witness_flow(&flows, &service_edge)?,
                    "period service issue",
                )?;
            }
            if declared_service != issue_flow {
                return Err(format!(
                    "temporal verifier: service issue gate does not reconcile at node {} period {}",
                    node.node_id, period
                ));
            }
        }
    }

    let total_service = sum_service_flows(&profile.services, &flows)?;
    if total_source != bundle.result.max_flow
        || total_service != bundle.result.max_flow
        || total_source != total_service
    {
        return Err(format!(
            "temporal verifier: source/service/max-flow reconciliation failed: source={total_source}, service={total_service}, max_flow={}",
            bundle.result.max_flow
        ));
    }

    Ok(())
}

fn check_flow_arc(
    flow: &EdgeFlow,
    graph_edge: &FlowEdge,
) -> Result<(), String> {
    if flow.from != graph_edge.from
        || flow.to != graph_edge.to
        || flow.capacity != graph_edge.capacity
        || flow.metadata_ref != graph_edge.metadata_ref
        || flow.flow > flow.capacity
    {
        return Err(format!(
            "temporal verifier: witness does not match graph edge {}",
            flow.id
        ));
    }
    Ok(())
}
