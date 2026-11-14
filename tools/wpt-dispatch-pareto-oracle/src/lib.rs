// SPDX-License-Identifier: AGPL-3.0-or-later
//! Exact, bounded, integer enumeration oracle for small WPT dispatch episodes.
//!
//! Unlike a weighted allocator, this evaluator returns the non-dominated set
//! over delivered energy, worst receiver satisfaction, Jain fairness and an
//! explicit asset-stress surrogate. Shared linear resource envelopes are
//! enforced jointly across links. This is a synthetic benchmark oracle, not
//! a production scheduler or a physical multiport solver.

use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const MAX_INTERVALS: u64 = 8;
const MAX_RECEIVERS: usize = 8;
const MAX_SOURCE_SLOTS: usize = 32;
const MAX_SEGMENT_SLOTS: usize = 32;
const MAX_RECEIVER_SLOTS: usize = 64;
const MAX_RESOURCE_SLOTS: usize = 32;
const MAX_TRANSFER_LINKS: usize = 8;
const MAX_TOTAL_REQUESTED_QUANTA: u64 = 20;
const MAX_ENUMERATION_NODES: u64 = 5_000_000;
const MAX_PARETO_POINTS: usize = 256;
const PPM: u64 = 1_000_000;

fn fail(path: &str, message: impl AsRef<str>) -> String {
    format!("{path}: {}", message.as_ref())
}

fn object<'a>(value: &'a Value, path: &str) -> Result<&'a serde_json::Map<String, Value>, String> {
    value
        .as_object()
        .ok_or_else(|| fail(path, "must be an object"))
}

fn exact_keys(value: &Value, allowed: &[&str], path: &str) -> Result<(), String> {
    for key in object(value, path)?.keys() {
        if !allowed.contains(&key.as_str()) {
            return Err(fail(path, format!("unknown property {key}")));
        }
    }
    Ok(())
}

fn field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Value, String> {
    value
        .as_object()
        .and_then(|map| map.get(key))
        .ok_or_else(|| fail(path, format!("missing property {key}")))
}

fn string_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    field(value, key, path)?
        .as_str()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-empty string"))
}

fn uint_field(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    field(value, key, path)?
        .as_u64()
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be a non-negative integer"))
}

fn array_field<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a [Value], String> {
    field(value, key, path)?
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| fail(&format!("{path}.{key}"), "must be an array"))
}

fn quanta(value: u64, quantum_mwh: u64, path: &str) -> Result<u64, String> {
    if !value.is_multiple_of(quantum_mwh) {
        return Err(fail(
            path,
            format!("energy {value} mWh is not divisible by quantum {quantum_mwh} mWh"),
        ));
    }
    Ok(value / quantum_mwh)
}

#[derive(Debug, Clone)]
struct Receiver {
    receiver_id: String,
    arrival_interval: usize,
    departure_interval: usize,
    requested_quanta: u64,
    minimum_quanta: u64,
}

#[derive(Debug, Clone)]
struct SegmentSlot {
    source_slot_index: usize,
    slot_index: usize,
}

#[derive(Debug, Clone)]
struct ResourceSlot {
    resource_id: String,
    interval_index: usize,
    capacity_units: u64,
}

#[derive(Debug, Clone)]
struct Link {
    link_id: String,
    source_slot_index: usize,
    segment_slot_index: usize,
    receiver_index: usize,
    receiver_slot_index: usize,
    interval_index: usize,
    maximum_quanta: u64,
    stress_units_per_quantum: u64,
    resource_uses: Vec<(usize, u64)>,
}

#[derive(Debug, Clone)]
struct Scenario {
    scenario_id: String,
    interval_count: usize,
    energy_quantum_mwh: u64,
    receivers: Vec<Receiver>,
    source_capacities: Vec<u64>,
    segment_capacities: Vec<u64>,
    receiver_slot_capacities: Vec<u64>,
    resource_capacities: Vec<u64>,
    resources: Vec<ResourceSlot>,
    links: Vec<Link>,
    total_requested_mwh: u64,
}

fn parse_scenario(value: &Value) -> Result<Scenario, String> {
    exact_keys(
        value,
        &[
            "schema_version",
            "scenario_id",
            "evidence_kind",
            "interval_count",
            "energy_quantum_mwh",
            "source_slots",
            "segment_slots",
            "receivers",
            "receiver_slots",
            "shared_resource_slots",
            "transfer_links",
            "quality_flags",
        ],
        "$",
    )?;

    if string_field(value, "schema_version", "$")? != "wpt-dispatch-pareto-oracle-v0" {
        return Err(fail("$.schema_version", "unsupported schema version"));
    }
    let scenario_id = string_field(value, "scenario_id", "$")?.to_string();
    if string_field(value, "evidence_kind", "$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err(fail(
            "$.evidence_kind",
            "v0 admits synthetic known-truth scenarios only",
        ));
    }
    let flags = array_field(value, "quality_flags", "$")?;
    if flags.len() != 1 || flags[0].as_str() != Some("SYNTHETIC_ONLY") {
        return Err(fail(
            "$.quality_flags",
            "v0 scenarios must carry exactly SYNTHETIC_ONLY",
        ));
    }

    let interval_count = uint_field(value, "interval_count", "$")?;
    if !(1..=MAX_INTERVALS).contains(&interval_count) {
        return Err(fail(
            "$.interval_count",
            format!("must be within 1..={MAX_INTERVALS}"),
        ));
    }
    let interval_count = interval_count as usize;
    let energy_quantum_mwh = uint_field(value, "energy_quantum_mwh", "$")?;
    if energy_quantum_mwh == 0 {
        return Err(fail("$.energy_quantum_mwh", "must be positive"));
    }

    let receiver_rows = array_field(value, "receivers", "$")?;
    if receiver_rows.is_empty() || receiver_rows.len() > MAX_RECEIVERS {
        return Err(fail(
            "$.receivers",
            format!("must contain 1..={MAX_RECEIVERS} receivers"),
        ));
    }
    let mut receivers = Vec::with_capacity(receiver_rows.len());
    let mut receiver_indices = BTreeMap::new();
    let mut total_requested_quanta = 0_u64;
    for (index, row) in receiver_rows.iter().enumerate() {
        let path = format!("$.receivers[{index}]");
        exact_keys(
            row,
            &[
                "receiver_id",
                "arrival_interval",
                "departure_interval",
                "requested_service_mwh",
                "minimum_service_mwh",
            ],
            &path,
        )?;
        let receiver_id = string_field(row, "receiver_id", &path)?.to_string();
        let arrival = uint_field(row, "arrival_interval", &path)?;
        let departure = uint_field(row, "departure_interval", &path)?;
        if arrival >= departure || departure > interval_count as u64 {
            return Err(fail(
                &path,
                "receiver window must satisfy 0 <= arrival < departure <= interval_count",
            ));
        }
        let requested = quanta(
            uint_field(row, "requested_service_mwh", &path)?,
            energy_quantum_mwh,
            &format!("{path}.requested_service_mwh"),
        )?;
        let minimum = quanta(
            uint_field(row, "minimum_service_mwh", &path)?,
            energy_quantum_mwh,
            &format!("{path}.minimum_service_mwh"),
        )?;
        if minimum == 0 || minimum > requested {
            return Err(fail(
                &path,
                "minimum service must be positive and no greater than requested service",
            ));
        }
        total_requested_quanta = total_requested_quanta
            .checked_add(requested)
            .ok_or_else(|| fail("$.receivers", "total requested energy overflowed"))?;
        if receiver_indices
            .insert(receiver_id.clone(), index)
            .is_some()
        {
            return Err(fail(&path, format!("duplicate receiver_id {receiver_id}")));
        }
        receivers.push(Receiver {
            receiver_id,
            arrival_interval: arrival as usize,
            departure_interval: departure as usize,
            requested_quanta: requested,
            minimum_quanta: minimum,
        });
    }
    if total_requested_quanta > MAX_TOTAL_REQUESTED_QUANTA {
        return Err(fail(
            "$.receivers",
            format!(
                "total requested energy exceeds bounded exact-search limit of {MAX_TOTAL_REQUESTED_QUANTA} quanta"
            ),
        ));
    }

    receivers.sort_by_key(|receiver| receiver.receiver_id.clone());
    receiver_indices.clear();
    for (index, receiver) in receivers.iter().enumerate() {
        receiver_indices.insert(receiver.receiver_id.clone(), index);
    }

    let source_rows = array_field(value, "source_slots", "$")?;
    if source_rows.is_empty() || source_rows.len() > MAX_SOURCE_SLOTS {
        return Err(fail(
            "$.source_slots",
            format!("must contain 1..={MAX_SOURCE_SLOTS} slots"),
        ));
    }
    let mut source_capacities = Vec::with_capacity(source_rows.len());
    let mut source_indices = BTreeMap::new();
    for (index, row) in source_rows.iter().enumerate() {
        let path = format!("$.source_slots[{index}]");
        exact_keys(
            row,
            &[
                "source_id",
                "interval_index",
                "available_deliverable_energy_mwh",
            ],
            &path,
        )?;
        let source_id = string_field(row, "source_id", &path)?.to_string();
        let interval = uint_field(row, "interval_index", &path)?;
        if interval >= interval_count as u64 {
            return Err(fail(&path, "interval_index is outside interval_count"));
        }
        let key = (source_id.clone(), interval as usize);
        if source_indices.insert(key.clone(), index).is_some() {
            return Err(fail(
                &path,
                format!("duplicate source slot {}@{}", key.0, key.1),
            ));
        }
        source_capacities.push(quanta(
            uint_field(row, "available_deliverable_energy_mwh", &path)?,
            energy_quantum_mwh,
            &format!("{path}.available_deliverable_energy_mwh"),
        )?);
    }

    let segment_rows = array_field(value, "segment_slots", "$")?;
    if segment_rows.is_empty() || segment_rows.len() > MAX_SEGMENT_SLOTS {
        return Err(fail(
            "$.segment_slots",
            format!("must contain 1..={MAX_SEGMENT_SLOTS} slots"),
        ));
    }
    let mut segment_capacities = Vec::with_capacity(segment_rows.len());
    let mut segment_slots = BTreeMap::new();
    for (index, row) in segment_rows.iter().enumerate() {
        let path = format!("$.segment_slots[{index}]");
        exact_keys(
            row,
            &[
                "source_id",
                "segment_id",
                "interval_index",
                "available_deliverable_energy_mwh",
            ],
            &path,
        )?;
        let source_id = string_field(row, "source_id", &path)?.to_string();
        let segment_id = string_field(row, "segment_id", &path)?.to_string();
        let interval = uint_field(row, "interval_index", &path)?;
        if interval >= interval_count as u64 {
            return Err(fail(&path, "interval_index is outside interval_count"));
        }
        let source_slot_index = *source_indices
            .get(&(source_id.clone(), interval as usize))
            .ok_or_else(|| fail(&path, "segment slot references a missing source slot"))?;
        let key = (segment_id.clone(), interval as usize);
        let capacity_quanta = quanta(
            uint_field(row, "available_deliverable_energy_mwh", &path)?,
            energy_quantum_mwh,
            &format!("{path}.available_deliverable_energy_mwh"),
        )?;
        if segment_slots
            .insert(
                key.clone(),
                SegmentSlot {
                    source_slot_index,
                    slot_index: index,
                },
            )
            .is_some()
        {
            return Err(fail(
                &path,
                format!("duplicate segment slot {}@{}", key.0, key.1),
            ));
        }
        segment_capacities.push(capacity_quanta);
    }

    let receiver_slot_rows = array_field(value, "receiver_slots", "$")?;
    if receiver_slot_rows.len() > MAX_RECEIVER_SLOTS {
        return Err(fail(
            "$.receiver_slots",
            format!("must contain at most {MAX_RECEIVER_SLOTS} slots"),
        ));
    }
    let mut receiver_slot_capacities = Vec::with_capacity(receiver_slot_rows.len());
    let mut receiver_slot_indices = BTreeMap::new();
    for (index, row) in receiver_slot_rows.iter().enumerate() {
        let path = format!("$.receiver_slots[{index}]");
        exact_keys(
            row,
            &["receiver_id", "interval_index", "max_acceptance_mwh"],
            &path,
        )?;
        let receiver_id = string_field(row, "receiver_id", &path)?.to_string();
        let receiver_index = *receiver_indices
            .get(&receiver_id)
            .ok_or_else(|| fail(&path, format!("unknown receiver_id {receiver_id}")))?;
        let interval = uint_field(row, "interval_index", &path)?;
        let receiver = &receivers[receiver_index];
        if interval < receiver.arrival_interval as u64
            || interval >= receiver.departure_interval as u64
        {
            return Err(fail(
                &path,
                "receiver slot must lie inside the half-open service window",
            ));
        }
        let key = (receiver_id.clone(), interval as usize);
        if receiver_slot_indices.insert(key.clone(), index).is_some() {
            return Err(fail(
                &path,
                format!("duplicate receiver slot {}@{}", key.0, key.1),
            ));
        }
        receiver_slot_capacities.push(quanta(
            uint_field(row, "max_acceptance_mwh", &path)?,
            energy_quantum_mwh,
            &format!("{path}.max_acceptance_mwh"),
        )?);
    }

    let resource_rows = array_field(value, "shared_resource_slots", "$")?;
    if resource_rows.len() > MAX_RESOURCE_SLOTS {
        return Err(fail(
            "$.shared_resource_slots",
            format!("must contain at most {MAX_RESOURCE_SLOTS} slots"),
        ));
    }
    let mut resource_capacities = Vec::with_capacity(resource_rows.len());
    let mut resources = Vec::with_capacity(resource_rows.len());
    let mut resource_indices = BTreeMap::new();
    for (index, row) in resource_rows.iter().enumerate() {
        let path = format!("$.shared_resource_slots[{index}]");
        exact_keys(
            row,
            &["resource_id", "interval_index", "capacity_units"],
            &path,
        )?;
        let resource_id = string_field(row, "resource_id", &path)?.to_string();
        let interval = uint_field(row, "interval_index", &path)?;
        if interval >= interval_count as u64 {
            return Err(fail(&path, "interval_index is outside interval_count"));
        }
        let key = (resource_id.clone(), interval as usize);
        if resource_indices.insert(key.clone(), index).is_some() {
            return Err(fail(
                &path,
                format!("duplicate shared resource slot {}@{}", key.0, key.1),
            ));
        }
        let capacity_units = uint_field(row, "capacity_units", &path)?;
        resource_capacities.push(capacity_units);
        resources.push(ResourceSlot {
            resource_id,
            interval_index: interval as usize,
            capacity_units,
        });
    }

    let link_rows = array_field(value, "transfer_links", "$")?;
    if link_rows.is_empty() || link_rows.len() > MAX_TRANSFER_LINKS {
        return Err(fail(
            "$.transfer_links",
            format!("must contain 1..={MAX_TRANSFER_LINKS} links for bounded exact enumeration"),
        ));
    }
    let mut link_ids = BTreeSet::new();
    let mut links = Vec::with_capacity(link_rows.len());
    for (index, row) in link_rows.iter().enumerate() {
        let path = format!("$.transfer_links[{index}]");
        exact_keys(
            row,
            &[
                "link_id",
                "source_id",
                "segment_id",
                "receiver_id",
                "interval_index",
                "max_deliverable_energy_mwh",
                "asset_stress_units_per_quantum",
                "shared_resource_uses",
            ],
            &path,
        )?;
        let link_id = string_field(row, "link_id", &path)?.to_string();
        if !link_ids.insert(link_id.clone()) {
            return Err(fail(&path, format!("duplicate link_id {link_id}")));
        }
        let source_id = string_field(row, "source_id", &path)?.to_string();
        let segment_id = string_field(row, "segment_id", &path)?.to_string();
        let receiver_id = string_field(row, "receiver_id", &path)?.to_string();
        let interval = uint_field(row, "interval_index", &path)?;
        if interval >= interval_count as u64 {
            return Err(fail(&path, "interval_index is outside interval_count"));
        }
        let interval = interval as usize;
        let source_slot_index = *source_indices
            .get(&(source_id.clone(), interval))
            .ok_or_else(|| fail(&path, "transfer link references a missing source slot"))?;
        let segment = segment_slots
            .get(&(segment_id.clone(), interval))
            .ok_or_else(|| fail(&path, "transfer link references a missing segment slot"))?;
        if segment.source_slot_index != source_slot_index {
            return Err(fail(
                &path,
                "segment and transfer link source_id do not match",
            ));
        }
        let receiver_index = *receiver_indices
            .get(&receiver_id)
            .ok_or_else(|| fail(&path, format!("unknown receiver_id {receiver_id}")))?;
        let receiver = &receivers[receiver_index];
        if interval < receiver.arrival_interval || interval >= receiver.departure_interval {
            return Err(fail(
                &path,
                "transfer link is outside the receiver's half-open service window",
            ));
        }
        let receiver_slot_index = *receiver_slot_indices
            .get(&(receiver_id.clone(), interval))
            .ok_or_else(|| fail(&path, "transfer link references a missing receiver slot"))?;

        let resource_rows = array_field(row, "shared_resource_uses", &path)?;
        let mut seen_resources = BTreeSet::new();
        let mut resource_uses = Vec::with_capacity(resource_rows.len());
        for (resource_index, resource_row) in resource_rows.iter().enumerate() {
            let resource_path = format!("{path}.shared_resource_uses[{resource_index}]");
            exact_keys(
                resource_row,
                &["resource_id", "units_per_quantum"],
                &resource_path,
            )?;
            let resource_id = string_field(resource_row, "resource_id", &resource_path)?;
            if !seen_resources.insert(resource_id.to_string()) {
                return Err(fail(
                    &resource_path,
                    format!("duplicate resource use {resource_id}"),
                ));
            }
            let resource_slot_index =
                *resource_indices
                    .get(&(resource_id.to_string(), interval))
                    .ok_or_else(|| fail(&resource_path, "missing shared resource slot"))?;
            let units_per_quantum = uint_field(resource_row, "units_per_quantum", &resource_path)?;
            if units_per_quantum == 0 {
                return Err(fail(&resource_path, "units_per_quantum must be positive"));
            }
            resource_uses.push((resource_slot_index, units_per_quantum));
        }
        resource_uses.sort_by_key(|entry| entry.0);

        links.push(Link {
            link_id,
            source_slot_index,
            segment_slot_index: segment.slot_index,
            receiver_index,
            receiver_slot_index,
            interval_index: interval,
            maximum_quanta: quanta(
                uint_field(row, "max_deliverable_energy_mwh", &path)?,
                energy_quantum_mwh,
                &format!("{path}.max_deliverable_energy_mwh"),
            )?,
            stress_units_per_quantum: uint_field(row, "asset_stress_units_per_quantum", &path)?,
            resource_uses,
        });
    }
    links.sort_by_key(|link| link.link_id.clone());

    let total_requested_mwh = total_requested_quanta
        .checked_mul(energy_quantum_mwh)
        .ok_or_else(|| fail("$.receivers", "total requested mWh overflowed u64"))?;
    Ok(Scenario {
        scenario_id,
        interval_count,
        energy_quantum_mwh,
        receivers,
        source_capacities,
        segment_capacities,
        receiver_slot_capacities,
        resource_capacities,
        resources,
        links,
        total_requested_mwh,
    })
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct LinkAllocation {
    pub link_id: String,
    pub interval_index: usize,
    pub allocated_mwh: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ReceiverAllocation {
    pub receiver_id: String,
    pub requested_mwh: u64,
    pub minimum_service_mwh: u64,
    pub delivered_mwh: u64,
    pub shortfall_mwh: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SharedResourceAllocation {
    pub resource_id: String,
    pub interval_index: usize,
    pub capacity_units: u64,
    pub used_units: u64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParetoPoint {
    pub total_delivered_mwh: u64,
    pub worst_receiver_satisfaction_ppm: u64,
    pub jain_fairness_ppm: u64,
    pub total_asset_stress_units: u64,
    pub receiver_allocations: Vec<ReceiverAllocation>,
    pub shared_resource_allocations: Vec<SharedResourceAllocation>,
    /// In link_id order; exact constructive allocation witness.
    pub link_allocations: Vec<LinkAllocation>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ParetoOracleReport {
    pub scenario_id: String,
    pub interval_count: usize,
    pub energy_quantum_mwh: u64,
    pub receiver_count: usize,
    pub transfer_link_count: usize,
    pub total_requested_mwh: u64,
    pub enumeration_nodes: u64,
    pub feasible_allocations: u64,
    pub feasible: bool,
    pub pareto_frontier: Vec<ParetoPoint>,
    pub claim_ceiling: &'static str,
}

struct Search<'a> {
    scenario: &'a Scenario,
    suffix_possible: Vec<Vec<u64>>,
    allocations: Vec<u64>,
    receiver_delivered: Vec<u64>,
    nodes: u64,
    feasible_allocations: u64,
    frontier: Vec<ParetoPoint>,
}

fn objectives_dominate(left: &ParetoPoint, right: &ParetoPoint) -> bool {
    left.total_delivered_mwh >= right.total_delivered_mwh
        && left.worst_receiver_satisfaction_ppm >= right.worst_receiver_satisfaction_ppm
        && left.jain_fairness_ppm >= right.jain_fairness_ppm
        && left.total_asset_stress_units <= right.total_asset_stress_units
        && (left.total_delivered_mwh > right.total_delivered_mwh
            || left.worst_receiver_satisfaction_ppm > right.worst_receiver_satisfaction_ppm
            || left.jain_fairness_ppm > right.jain_fairness_ppm
            || left.total_asset_stress_units < right.total_asset_stress_units)
}

fn same_objectives(left: &ParetoPoint, right: &ParetoPoint) -> bool {
    left.total_delivered_mwh == right.total_delivered_mwh
        && left.worst_receiver_satisfaction_ppm == right.worst_receiver_satisfaction_ppm
        && left.jain_fairness_ppm == right.jain_fairness_ppm
        && left.total_asset_stress_units == right.total_asset_stress_units
}

fn candidate_point(
    scenario: &Scenario,
    allocations: &[u64],
    receiver_delivered: &[u64],
    stress: u64,
) -> Result<ParetoPoint, String> {
    let mut total_quanta = 0_u64;
    let mut worst_ratio = PPM;
    let mut satisfaction = Vec::with_capacity(scenario.receivers.len());
    let mut receiver_allocations = Vec::with_capacity(scenario.receivers.len());

    for (index, receiver) in scenario.receivers.iter().enumerate() {
        let delivered = receiver_delivered[index];
        if delivered < receiver.minimum_quanta {
            return Err(
                "internal enumeration reached a leaf below a receiver service floor".into(),
            );
        }
        total_quanta = total_quanta
            .checked_add(delivered)
            .ok_or_else(|| "total delivered energy overflowed".to_string())?;
        let ratio = ((delivered as u128 * PPM as u128) / receiver.requested_quanta as u128)
            .min(PPM as u128) as u64;
        worst_ratio = worst_ratio.min(ratio);
        satisfaction.push(ratio);
        receiver_allocations.push(ReceiverAllocation {
            receiver_id: receiver.receiver_id.clone(),
            requested_mwh: receiver.requested_quanta * scenario.energy_quantum_mwh,
            minimum_service_mwh: receiver.minimum_quanta * scenario.energy_quantum_mwh,
            delivered_mwh: delivered * scenario.energy_quantum_mwh,
            shortfall_mwh: (receiver.requested_quanta - delivered) * scenario.energy_quantum_mwh,
        });
    }

    let sum = satisfaction
        .iter()
        .map(|value| *value as u128)
        .sum::<u128>();
    let sum_squares = satisfaction
        .iter()
        .map(|value| (*value as u128) * (*value as u128))
        .sum::<u128>();
    let jain_denominator = (satisfaction.len() as u128)
        .checked_mul(sum_squares)
        .ok_or_else(|| "Jain denominator overflowed u128".to_string())?;
    if jain_denominator == 0 {
        return Err("Jain fairness denominator must be positive".into());
    }
    let jain = sum
        .checked_mul(sum)
        .and_then(|value| value.checked_mul(PPM as u128))
        .ok_or_else(|| "Jain numerator overflowed u128".to_string())?
        / jain_denominator;

    let mut resource_used = vec![0_u64; scenario.resources.len()];
    for (link, amount) in scenario.links.iter().zip(allocations.iter()) {
        for (resource_index, units_per_quantum) in &link.resource_uses {
            let used = amount
                .checked_mul(*units_per_quantum)
                .ok_or_else(|| "resource-use multiplication overflowed".to_string())?;
            resource_used[*resource_index] = resource_used[*resource_index]
                .checked_add(used)
                .ok_or_else(|| "resource-use total overflowed".to_string())?;
        }
    }
    let mut shared_resource_allocations = scenario
        .resources
        .iter()
        .zip(resource_used.iter())
        .map(|(resource, used_units)| SharedResourceAllocation {
            resource_id: resource.resource_id.clone(),
            interval_index: resource.interval_index,
            capacity_units: resource.capacity_units,
            used_units: *used_units,
        })
        .collect::<Vec<_>>();
    shared_resource_allocations.sort_by_key(|row| (row.resource_id.clone(), row.interval_index));

    let link_allocations = scenario
        .links
        .iter()
        .zip(allocations.iter())
        .map(|(link, amount)| LinkAllocation {
            link_id: link.link_id.clone(),
            interval_index: link.interval_index,
            allocated_mwh: amount * scenario.energy_quantum_mwh,
        })
        .collect();

    Ok(ParetoPoint {
        total_delivered_mwh: total_quanta * scenario.energy_quantum_mwh,
        worst_receiver_satisfaction_ppm: worst_ratio,
        jain_fairness_ppm: jain as u64,
        total_asset_stress_units: stress,
        receiver_allocations,
        shared_resource_allocations,
        link_allocations,
    })
}

fn insert_pareto(frontier: &mut Vec<ParetoPoint>, candidate: ParetoPoint) -> Result<(), String> {
    for existing in frontier.iter() {
        if objectives_dominate(existing, &candidate)
            || (same_objectives(existing, &candidate)
                && existing
                    .link_allocations
                    .iter()
                    .map(|row| row.allocated_mwh)
                    .cmp(
                        candidate
                            .link_allocations
                            .iter()
                            .map(|row| row.allocated_mwh),
                    )
                    != std::cmp::Ordering::Greater)
        {
            return Ok(());
        }
    }
    frontier.retain(|existing| {
        !objectives_dominate(&candidate, existing)
            && (!same_objectives(&candidate, existing)
                || candidate
                    .link_allocations
                    .iter()
                    .map(|row| row.allocated_mwh)
                    .cmp(
                        existing
                            .link_allocations
                            .iter()
                            .map(|row| row.allocated_mwh),
                    )
                    == std::cmp::Ordering::Greater)
    });
    frontier.push(candidate);
    if frontier.len() > MAX_PARETO_POINTS {
        return Err(format!(
            "exact Pareto frontier exceeds bounded output limit of {MAX_PARETO_POINTS} points"
        ));
    }
    Ok(())
}

fn enumerate(
    search: &mut Search<'_>,
    link_index: usize,
    source_remaining: &mut [u64],
    segment_remaining: &mut [u64],
    receiver_slot_remaining: &mut [u64],
    resource_remaining: &mut [u64],
    stress: u64,
) -> Result<(), String> {
    search.nodes += 1;
    if search.nodes > MAX_ENUMERATION_NODES {
        return Err(format!(
            "exact enumeration exceeded {MAX_ENUMERATION_NODES} nodes; no approximate result is returned"
        ));
    }

    for (receiver_index, receiver) in search.scenario.receivers.iter().enumerate() {
        let maximum_remaining = search.suffix_possible[link_index][receiver_index];
        if search.receiver_delivered[receiver_index] + maximum_remaining < receiver.minimum_quanta {
            return Ok(());
        }
    }

    if link_index == search.scenario.links.len() {
        if search
            .scenario
            .receivers
            .iter()
            .enumerate()
            .any(|(index, receiver)| search.receiver_delivered[index] < receiver.minimum_quanta)
        {
            return Ok(());
        }
        search.feasible_allocations += 1;
        let candidate = candidate_point(
            search.scenario,
            &search.allocations,
            &search.receiver_delivered,
            stress,
        )?;
        return insert_pareto(&mut search.frontier, candidate);
    }

    let link = &search.scenario.links[link_index];
    let receiver = &search.scenario.receivers[link.receiver_index];
    let request_remaining =
        receiver.requested_quanta - search.receiver_delivered[link.receiver_index];
    let mut maximum = link
        .maximum_quanta
        .min(source_remaining[link.source_slot_index])
        .min(segment_remaining[link.segment_slot_index])
        .min(receiver_slot_remaining[link.receiver_slot_index])
        .min(request_remaining);
    for (resource_index, units_per_quantum) in &link.resource_uses {
        maximum = maximum.min(resource_remaining[*resource_index] / *units_per_quantum);
    }

    for amount in 0..=maximum {
        let next_stress = stress
            .checked_add(
                amount
                    .checked_mul(link.stress_units_per_quantum)
                    .ok_or_else(|| "link stress multiplication overflowed".to_string())?,
            )
            .ok_or_else(|| "total asset stress overflowed".to_string())?;
        source_remaining[link.source_slot_index] -= amount;
        segment_remaining[link.segment_slot_index] -= amount;
        receiver_slot_remaining[link.receiver_slot_index] -= amount;
        search.receiver_delivered[link.receiver_index] += amount;
        for (resource_index, units_per_quantum) in &link.resource_uses {
            resource_remaining[*resource_index] -= amount * *units_per_quantum;
        }
        search.allocations[link_index] = amount;

        enumerate(
            search,
            link_index + 1,
            source_remaining,
            segment_remaining,
            receiver_slot_remaining,
            resource_remaining,
            next_stress,
        )?;

        source_remaining[link.source_slot_index] += amount;
        segment_remaining[link.segment_slot_index] += amount;
        receiver_slot_remaining[link.receiver_slot_index] += amount;
        search.receiver_delivered[link.receiver_index] -= amount;
        for (resource_index, units_per_quantum) in &link.resource_uses {
            resource_remaining[*resource_index] += amount * *units_per_quantum;
        }
    }
    Ok(())
}

/// Enumerate the exact small-instance feasible set and return its Pareto frontier.
///
/// The input energy quantum must be declared; all energy capacities, receiver
/// targets and per-link energy limits must be exact multiples of it. Shared
/// resource coefficients are integer units per energy quantum. The oracle
/// enforces source, segment, receiver-acceptance and every declared shared
/// resource constraint jointly across the selected transfer links.
pub fn evaluate_scenario(value: &Value) -> Result<ParetoOracleReport, String> {
    let scenario = parse_scenario(value)?;
    let receiver_count = scenario.receivers.len();
    let link_count = scenario.links.len();

    let mut suffix_possible = vec![vec![0_u64; receiver_count]; link_count + 1];
    for index in (0..link_count).rev() {
        suffix_possible[index] = suffix_possible[index + 1].clone();
        let link = &scenario.links[index];
        let maximum = link
            .maximum_quanta
            .min(scenario.receivers[link.receiver_index].requested_quanta);
        suffix_possible[index][link.receiver_index] = suffix_possible[index][link.receiver_index]
            .saturating_add(maximum)
            .min(scenario.receivers[link.receiver_index].requested_quanta);
    }

    let mut search = Search {
        scenario: &scenario,
        suffix_possible,
        allocations: vec![0; link_count],
        receiver_delivered: vec![0; receiver_count],
        nodes: 0,
        feasible_allocations: 0,
        frontier: Vec::new(),
    };
    let mut source_remaining = scenario.source_capacities.clone();
    let mut segment_remaining = scenario.segment_capacities.clone();
    let mut receiver_slot_remaining = scenario.receiver_slot_capacities.clone();
    let mut resource_remaining = scenario.resource_capacities.clone();

    enumerate(
        &mut search,
        0,
        &mut source_remaining,
        &mut segment_remaining,
        &mut receiver_slot_remaining,
        &mut resource_remaining,
        0,
    )?;

    search.frontier.sort_by(|left, right| {
        right
            .total_delivered_mwh
            .cmp(&left.total_delivered_mwh)
            .then_with(|| {
                right
                    .worst_receiver_satisfaction_ppm
                    .cmp(&left.worst_receiver_satisfaction_ppm)
            })
            .then_with(|| right.jain_fairness_ppm.cmp(&left.jain_fairness_ppm))
            .then_with(|| {
                left.total_asset_stress_units
                    .cmp(&right.total_asset_stress_units)
            })
    });

    Ok(ParetoOracleReport {
        scenario_id: scenario.scenario_id.clone(),
        interval_count: scenario.interval_count,
        energy_quantum_mwh: scenario.energy_quantum_mwh,
        receiver_count,
        transfer_link_count: link_count,
        total_requested_mwh: scenario.total_requested_mwh,
        enumeration_nodes: search.nodes,
        feasible_allocations: search.feasible_allocations,
        feasible: !search.frontier.is_empty(),
        pareto_frontier: search.frontier,
        claim_ceiling: "Exact Pareto enumeration for bounded synthetic integer-capacity episodes and declared linear shared-resource constraints only; no physical multiport validation, robust uncertainty guarantee, production scheduling, safety, grid-code or field-performance claim.",
    })
}

pub fn bundled_fixture() -> Result<Value, String> {
    serde_json::from_str(include_str!(
        "../../../research/wpt-dispatch-pareto-v0/scenario.synthetic-example.json"
    ))
    .map_err(|error| format!("synthetic WPT Pareto scenario JSON: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        bundled_fixture().unwrap()
    }

    #[test]
    fn known_truth_fixture_has_multiple_non_dominated_decisions() {
        let report = evaluate_scenario(&fixture()).unwrap();
        assert!(report.feasible);
        assert_eq!(report.total_requested_mwh, 10);
        assert_eq!(report.feasible_allocations, 89);
        assert_eq!(report.pareto_frontier.len(), 6);
        assert!(
            report
                .pareto_frontier
                .iter()
                .all(|point| point.total_delivered_mwh <= 8)
        );

        let low_stress = report
            .pareto_frontier
            .iter()
            .find(|point| {
                point.worst_receiver_satisfaction_ppm == 600_000
                    && point.total_asset_stress_units == 8
            })
            .unwrap();
        assert_eq!(low_stress.total_delivered_mwh, 8);
        assert_eq!(low_stress.jain_fairness_ppm, 941_176);

        let max_fair = report
            .pareto_frontier
            .iter()
            .find(|point| point.jain_fairness_ppm == PPM)
            .unwrap();
        assert_eq!(max_fair.total_delivered_mwh, 8);
        assert_eq!(max_fair.worst_receiver_satisfaction_ppm, 800_000);
        assert_eq!(max_fair.total_asset_stress_units, 11);
    }

    #[test]
    fn shared_coupling_resource_is_enforced_jointly_across_pairwise_links() {
        let report = evaluate_scenario(&fixture()).unwrap();
        for point in report.pareto_frontier {
            let resource = point
                .shared_resource_allocations
                .iter()
                .find(|row| row.resource_id == "segment-a-coupling-envelope")
                .unwrap();
            assert_eq!(resource.capacity_units, 3);
            assert!(resource.used_units <= resource.capacity_units);
            let segment_a_use = point
                .link_allocations
                .iter()
                .filter(|link| link.link_id.starts_with("a-"))
                .map(|link| link.allocated_mwh)
                .sum::<u64>();
            assert_eq!(resource.used_units, segment_a_use);
        }
    }

    #[test]
    fn impossible_individual_service_floors_return_no_feasible_point() {
        let mut scenario = fixture();
        scenario["receivers"][0]["minimum_service_mwh"] = Value::from(5);
        scenario["receivers"][1]["minimum_service_mwh"] = Value::from(5);
        let report = evaluate_scenario(&scenario).unwrap();
        assert!(!report.feasible);
        assert!(report.pareto_frontier.is_empty());
        assert_eq!(report.feasible_allocations, 0);
    }

    #[test]
    fn non_multiple_of_energy_quantum_is_rejected() {
        let mut scenario = fixture();
        scenario["energy_quantum_mwh"] = Value::from(3);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("not divisible")
        );
    }

    #[test]
    fn unknown_shared_resource_is_rejected() {
        let mut scenario = fixture();
        scenario["transfer_links"][0]["shared_resource_uses"][0]["resource_id"] =
            Value::String("unregistered-resource".into());
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("missing shared resource slot")
        );
    }

    #[test]
    fn half_open_departure_window_rejects_late_transfer() {
        let mut scenario = fixture();
        scenario["interval_count"] = Value::from(2);
        scenario["source_slots"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "source_id": "site-inverter",
                "interval_index": 1,
                "available_deliverable_energy_mwh": 8
            }));
        scenario["segment_slots"]
            .as_array_mut()
            .unwrap()
            .push(serde_json::json!({
                "source_id": "site-inverter",
                "segment_id": "segment-a",
                "interval_index": 1,
                "available_deliverable_energy_mwh": 5
            }));
        scenario["transfer_links"][0]["interval_index"] = Value::from(1);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("outside the receiver's half-open service window")
        );
    }

    #[test]
    fn duplicate_link_identity_and_unknown_properties_fail_closed() {
        let mut scenario = fixture();
        let duplicate_id = scenario["transfer_links"][0]["link_id"].clone();
        scenario["transfer_links"][1]["link_id"] = duplicate_id;
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("duplicate link_id")
        );

        let mut scenario = fixture();
        scenario["optimizer_shortcut"] = Value::Bool(true);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("unknown property")
        );
    }

    #[test]
    fn unsupported_evidence_kind_is_not_admitted() {
        let mut scenario = fixture();
        scenario["evidence_kind"] = Value::String("MEASURED".into());
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("synthetic known-truth")
        );
    }

    #[test]
    fn input_row_order_does_not_change_the_frontier_or_witness() {
        let first = evaluate_scenario(&fixture()).unwrap();
        let mut reversed = fixture();
        reversed["receivers"].as_array_mut().unwrap().reverse();
        reversed["segment_slots"].as_array_mut().unwrap().reverse();
        reversed["receiver_slots"].as_array_mut().unwrap().reverse();
        reversed["shared_resource_slots"]
            .as_array_mut()
            .unwrap()
            .reverse();
        reversed["transfer_links"].as_array_mut().unwrap().reverse();
        let second = evaluate_scenario(&reversed).unwrap();
        assert_eq!(first.pareto_frontier, second.pareto_frontier);
        assert_eq!(first.feasible_allocations, second.feasible_allocations);
    }

    #[test]
    fn pareto_frontier_has_no_dominated_or_duplicate_objective_points() {
        let report = evaluate_scenario(&fixture()).unwrap();
        for (index, left) in report.pareto_frontier.iter().enumerate() {
            for right in report.pareto_frontier.iter().skip(index + 1) {
                assert!(!objectives_dominate(left, right));
                assert!(!objectives_dominate(right, left));
                assert!(!same_objectives(left, right));
            }
        }
    }

    #[test]
    fn receiver_service_floor_must_not_exceed_request() {
        let mut scenario = fixture();
        scenario["receivers"][0]["minimum_service_mwh"] = Value::from(6);
        assert!(
            evaluate_scenario(&scenario)
                .unwrap_err()
                .contains("no greater than requested service")
        );
    }
}
