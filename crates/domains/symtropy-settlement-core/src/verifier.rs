// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent consistency checker for synthetic temporal settlement receipts.
//!
//! The verifier does not run the simulator, authorize a policy, or validate
//! physical reality. It checks whether a retained receipt is internally
//! consistent, tamper-evident under the declared serialization, and complete
//! against the timeline fields emitted by the v1 report contract.

use crate::{
    CLAIM_CEILING, FloorStatus, SettlementError, SettlementReport,
    timeline::{
        FloorRecoverySummary, SettlementTimelineReport, SettlementTimelineReportPayload,
        SettlementTickReceipt, TIMELINE_CLAIM_CEILING,
    },
};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

pub fn verify_timeline_report(
    report: &SettlementTimelineReport,
) -> Result<(), SettlementError> {
    let payload = &report.payload;
    if payload.schema_version != crate::timeline::SETTLEMENT_TIMELINE_SCHEMA_VERSION {
        return Err(SettlementError(
            "timeline report has unsupported schema version".into(),
        ));
    }
    if payload.claim_ceiling != TIMELINE_CLAIM_CEILING {
        return Err(SettlementError(
            "timeline report claim ceiling is missing or widened".into(),
        ));
    }
    if !is_sha256_hex(&payload.timeline_input_sha256) {
        return Err(SettlementError(
            "timeline input digest is not 64 lowercase hexadecimal characters".into(),
        ));
    }
    let encoded_payload = serde_json::to_vec(payload)
        .map_err(|error| SettlementError(format!("serialize timeline receipt: {error}")))?;
    if sha256_hex(&encoded_payload) != report.payload_sha256 {
        return Err(SettlementError(
            "timeline report payload digest mismatch".into(),
        ));
    }

    let expected_tick_count = payload
        .horizon_ticks
        .checked_add(1)
        .ok_or_else(|| SettlementError("timeline horizon overflow".into()))?;
    let actual_tick_count = u64::try_from(payload.ticks.len())
        .map_err(|_| SettlementError("timeline receipt count does not fit u64".into()))?;
    if actual_tick_count != expected_tick_count {
        return Err(SettlementError(format!(
            "timeline has {actual_tick_count} tick receipts, expected {expected_tick_count}"
        )));
    }
    if payload.stability_window_ticks == 0
        || payload.stability_window_ticks > expected_tick_count
    {
        return Err(SettlementError(
            "timeline receipt has invalid stability-window length".into(),
        ));
    }

    let mut global_event_ids = BTreeSet::new();
    let mut prior_stocks = BTreeMap::<String, u64>::new();
    let mut stable_supply_contracts = BTreeMap::<String, (crate::ResourceKind, String, u64)>::new();
    let mut stable_component_generations = BTreeMap::<String, u64>::new();
    let mut stable_floor_ids = BTreeSet::<String>::new();
    let mut stable_demand_ids = BTreeSet::<String>::new();
    let mut stable_scenario_id: Option<String> = None;

    for (index, tick) in payload.ticks.iter().enumerate() {
        let expected_tick = u64::try_from(index)
            .map_err(|_| SettlementError("tick index does not fit u64".into()))?;
        if tick.tick != expected_tick {
            return Err(SettlementError(format!(
                "tick receipt order gap: index {expected_tick}, receipt says {}",
                tick.tick
            )));
        }
        let mut tick_event_ids = BTreeSet::new();
        let mut prior_sequence = None;
        let mut declared_inflows = BTreeMap::<String, u64>::new();
        for event in &tick.applied_events {
            if event.tick != tick.tick {
                return Err(SettlementError(format!(
                    "event {} is attributed to the wrong tick",
                    event.event_id
                )));
            }
            if !tick_event_ids.insert(event.event_id.clone())
                || !global_event_ids.insert(event.event_id.clone())
            {
                return Err(SettlementError(format!(
                    "duplicate applied event ID {}",
                    event.event_id
                )));
            }
            if prior_sequence.is_some_and(|sequence| sequence >= event.sequence) {
                return Err(SettlementError(format!(
                    "event sequence is not strictly increasing at tick {}",
                    tick.tick
                )));
            }
            prior_sequence = Some(event.sequence);
            if !is_sha256_hex(&event.event_sha256) {
                return Err(SettlementError(format!(
                    "event {} has malformed payload digest",
                    event.event_id
                )));
            }
            let encoded_event = serde_json::to_vec(&event.event)
                .map_err(|error| SettlementError(format!("serialize event receipt: {error}")))?;
            if sha256_hex(&encoded_event) != event.event_sha256 {
                return Err(SettlementError(format!(
                    "event {} payload digest mismatch",
                    event.event_id
                )));
            }
            if let crate::timeline::SettlementEventKind::ResourceInflow { supply_id, units } =
                &event.event
            {
                let total = declared_inflows.entry(supply_id.clone()).or_default();
                *total = total
                    .checked_add(*units)
                    .ok_or_else(|| SettlementError("event inflow total overflow".into()))?;
            }
        }

        verify_snapshot(&tick.snapshot)?;
        if stable_scenario_id
            .as_ref()
            .is_some_and(|scenario_id| scenario_id != &tick.snapshot.payload.scenario_id)
        {
            return Err(SettlementError(format!(
                "scenario identity changed at tick {}",
                tick.tick
            )));
        }
        if tick.snapshot.payload.scenario_id != payload.scenario_id {
            return Err(SettlementError(format!(
                "snapshot scenario identity mismatches timeline at tick {}",
                tick.tick
            )));
        }
        if stable_scenario_id.is_none() {
            stable_scenario_id = Some(tick.snapshot.payload.scenario_id.clone());
        }

        let current_demand_ids: BTreeSet<String> = tick
            .snapshot
            .payload
            .resource_allocations
            .iter()
            .map(|allocation| allocation.demand_id.clone())
            .collect();
        if current_demand_ids.len() != tick.snapshot.payload.resource_allocations.len() {
            return Err(SettlementError(format!(
                "duplicate demand allocation at tick {}",
                tick.tick
            )));
        }
        if index == 0 {
            stable_demand_ids = current_demand_ids;
        } else if stable_demand_ids != current_demand_ids {
            return Err(SettlementError(format!(
                "demand set changed at tick {}",
                tick.tick
            )));
        }

        let component_ids: BTreeSet<String> = tick
            .snapshot
            .payload
            .component_results
            .iter()
            .map(|component| component.component_id.as_str().to_string())
            .collect();
        if component_ids.len() != tick.snapshot.payload.component_results.len() {
            return Err(SettlementError(format!(
                "duplicate component result at tick {}",
                tick.tick
            )));
        }
        let current_generations: BTreeMap<String, u64> = tick
            .snapshot
            .payload
            .component_results
            .iter()
            .map(|component| {
                (
                    component.component_id.as_str().to_string(),
                    component.generation,
                )
            })
            .collect();
        for component in &tick.snapshot.payload.component_results {
            let output_consistent = match &component.effective_state {
                crate::ComponentState::Available => component.effective_output_units.is_some(),
                crate::ComponentState::Failed => {
                    component.effective_output_units == Some(0)
                }
                crate::ComponentState::Unknown { .. } => {
                    component.effective_output_units.is_none()
                }
            };
            if !output_consistent {
                return Err(SettlementError(format!(
                    "component output/state mismatch for {} at tick {}",
                    component.component_id, tick.tick
                )));
            }
        }
        if index == 0 {
            stable_component_generations = current_generations;
        } else if stable_component_generations != current_generations {
            return Err(SettlementError(format!(
                "component identity/generation set changed at tick {}",
                tick.tick
            )));
        }
        for event in &tick.applied_events {
            if let crate::timeline::SettlementEventKind::ComponentStateChanged {
                component_id,
                generation,
                ..
            } = &event.event
            {
                if current_generations.get(component_id.as_str()) != Some(generation) {
                    return Err(SettlementError(format!(
                        "event {} references an unrepresented component generation",
                        event.event_id
                    )));
                }
            }
        }

        let current_floor_ids: BTreeSet<String> = tick
            .snapshot
            .payload
            .service_floors
            .iter()
            .map(|floor| floor.floor_id.clone())
            .collect();
        if current_floor_ids.len() != tick.snapshot.payload.service_floors.len() {
            return Err(SettlementError(format!(
                "duplicate floor result at tick {}",
                tick.tick
            )));
        }
        for floor in &tick.snapshot.payload.service_floors {
            let component = tick
                .snapshot
                .payload
                .component_results
                .iter()
                .find(|component| component.component_id == floor.component_id)
                .ok_or_else(|| {
                    SettlementError(format!(
                        "floor {} references a missing component result",
                        floor.floor_id
                    ))
                })?;

            let mut proven_breach = matches!(
                &component.effective_state,
                crate::ComponentState::Failed
            );
            let mut indeterminate = matches!(
                &component.effective_state,
                crate::ComponentState::Unknown { .. }
            ) || floor.effective_output_units.is_none();

            if floor
                .effective_output_units
                .is_some_and(|units| units < floor.minimum_output_units)
            {
                proven_breach = true;
            }

            for demand_id in &floor.required_demand_ids {
                let allocation = tick
                    .snapshot
                    .payload
                    .resource_allocations
                    .iter()
                    .find(|allocation| allocation.demand_id == *demand_id)
                    .ok_or_else(|| {
                        SettlementError(format!(
                            "floor {} references missing demand {}",
                            floor.floor_id, demand_id
                        ))
                    })?;
                match &allocation.outcome {
                    crate::DemandOutcome::Shortfall
                    | crate::DemandOutcome::ConsumerUnavailable => {
                        proven_breach = true;
                    }
                    crate::DemandOutcome::Indeterminate
                    | crate::DemandOutcome::ConsumerStateUnknown
                    | crate::DemandOutcome::PoolBlockedByUnknownConsumer => {
                        indeterminate = true;
                    }
                    crate::DemandOutcome::FullySatisfied
                    | crate::DemandOutcome::MinimumSatisfied => {}
                }
            }

            let status_consistent = match floor.status {
                FloorStatus::Met => !proven_breach && !indeterminate,
                FloorStatus::Breached => proven_breach,
                FloorStatus::Blocked => !proven_breach && indeterminate,
            };
            if !status_consistent {
                return Err(SettlementError(format!(
                    "service floor status disagrees with supporting evidence for {} at tick {}",
                    floor.floor_id, tick.tick
                )));
            }
        }
        if index == 0 {
            stable_floor_ids = current_floor_ids;
        } else if stable_floor_ids != current_floor_ids {
            return Err(SettlementError(format!(
                "service floor set changed at tick {}",
                tick.tick
            )));
        }

        let mut transition_supply_ids = BTreeSet::new();
        let mut consumed_by_supply = BTreeMap::<String, u64>::new();
        let mut transition_by_supply = BTreeMap::new();
        for transition in &tick.resource_transitions {
            if !transition_supply_ids.insert(transition.supply_id.clone()) {
                return Err(SettlementError(format!(
                    "duplicate stock transition for {} at tick {}",
                    transition.supply_id, tick.tick
                )));
            }
            if transition.unit_code != transition.resource_kind.synthetic_unit_code() {
                return Err(SettlementError(format!(
                    "resource unit mismatch for {} at tick {}",
                    transition.supply_id, tick.tick
                )));
            }
            if !transition.conserves() {
                return Err(SettlementError(format!(
                    "resource conservation failed for {} at tick {}",
                    transition.supply_id, tick.tick
                )));
            }
            let expected_inflow = declared_inflows
                .get(&transition.supply_id)
                .copied()
                .unwrap_or(0);
            if transition.inflow_units != expected_inflow {
                return Err(SettlementError(format!(
                    "event receipts and inflow ledger disagree for {} at tick {}",
                    transition.supply_id, tick.tick
                )));
            }
            if transition.closing_units < transition.protected_reserve_units {
                return Err(SettlementError(format!(
                    "protected reserve breached for {} at tick {}",
                    transition.supply_id, tick.tick
                )));
            }

            let contract = (
                transition.resource_kind,
                transition.unit_code.clone(),
                transition.protected_reserve_units,
            );
            if index == 0 {
                stable_supply_contracts.insert(transition.supply_id.clone(), contract);
            } else if stable_supply_contracts.get(&transition.supply_id) != Some(&contract) {
                return Err(SettlementError(format!(
                    "resource contract changed for {} at tick {}",
                    transition.supply_id, tick.tick
                )));
            }

            if let Some(previous_closing) = prior_stocks.get(&transition.supply_id) {
                if transition.opening_units != *previous_closing {
                    return Err(SettlementError(format!(
                        "stock continuity failed for {} at tick {}",
                        transition.supply_id, tick.tick
                    )));
                }
            } else if index > 0 {
                return Err(SettlementError(format!(
                    "new resource supply {} appeared after tick zero",
                    transition.supply_id
                )));
            }
            prior_stocks.insert(transition.supply_id.clone(), transition.closing_units);
            transition_by_supply.insert(transition.supply_id.clone(), transition);
        }

        if declared_inflows
            .keys()
            .any(|supply_id| !transition_supply_ids.contains(supply_id))
        {
            return Err(SettlementError(format!(
                "event inflow references a supply missing from tick {} receipt",
                tick.tick
            )));
        }
        if index == 0 {
            if stable_supply_contracts.len() != tick.resource_transitions.len() {
                return Err(SettlementError("invalid initial supply transition set".into()));
            }
        } else if transition_supply_ids.len() != stable_supply_contracts.len() {
            return Err(SettlementError(format!(
                "resource transition set incomplete at tick {}",
                tick.tick
            )));
        }

        for allocation in &tick.snapshot.payload.resource_allocations {
            if allocation.unit_code != allocation.resource_kind.synthetic_unit_code() {
                return Err(SettlementError(format!(
                    "allocation unit mismatch for demand {} at tick {}",
                    allocation.demand_id, tick.tick
                )));
            }
            let mut allocation_supply_ids = BTreeSet::new();
            let allocated_sum = allocation.sources.iter().try_fold(
                0_u64,
                |sum, source| {
                    if !allocation_supply_ids.insert(source.supply_id.clone()) {
                        return Err(SettlementError(format!(
                            "duplicate source {} in demand {} at tick {}",
                            source.supply_id, allocation.demand_id, tick.tick
                        )));
                    }
                    let transition = transition_by_supply
                        .get(&source.supply_id)
                        .ok_or_else(|| {
                            SettlementError(format!(
                                "allocation references missing supply {} at tick {}",
                                source.supply_id, tick.tick
                            ))
                        })?;
                    if transition.resource_kind != allocation.resource_kind {
                        return Err(SettlementError(format!(
                            "resource kind mismatch for demand {} at tick {}",
                            allocation.demand_id, tick.tick
                        )));
                    }
                    let consumed = consumed_by_supply
                        .entry(source.supply_id.clone())
                        .or_default();
                    *consumed = consumed.checked_add(source.units).ok_or_else(|| {
                        SettlementError("allocation ledger overflow".into())
                    })?;
                    sum.checked_add(source.units)
                        .ok_or_else(|| SettlementError("allocation sum overflow".into()))
                },
            )?;
            if allocated_sum != allocation.allocated_known_units {
                return Err(SettlementError(format!(
                    "allocation sources do not sum to demand {} at tick {}",
                    allocation.demand_id, tick.tick
                )));
            }
            if allocation.allocated_known_units > allocation.requested_units {
                return Err(SettlementError(format!(
                    "demand {} exceeds requested units at tick {}",
                    allocation.demand_id, tick.tick
                )));
            }
        }

        for transition in &tick.resource_transitions {
            let recorded_consumption = consumed_by_supply
                .get(&transition.supply_id)
                .copied()
                .unwrap_or(0);
            if recorded_consumption != transition.consumed_units {
                return Err(SettlementError(format!(
                    "allocation/transition consumption mismatch for {} at tick {}",
                    transition.supply_id, tick.tick
                )));
            }
        }
    }

    let independently_derived_recovery = derive_recovery_summaries(
        &payload.ticks,
        payload.stability_window_ticks,
        &stable_floor_ids,
    )?;
    if independently_derived_recovery != payload.floor_recovery {
        return Err(SettlementError(
            "floor recovery summary does not match per-tick floor trajectories".into(),
        ));
    }

    Ok(())
}

fn verify_snapshot(snapshot: &SettlementReport) -> Result<(), SettlementError> {
    if snapshot.payload.schema_version != crate::SETTLEMENT_SCHEMA_VERSION {
        return Err(SettlementError(
            "snapshot has unsupported settlement schema version".into(),
        ));
    }
    if snapshot.payload.claim_ceiling != CLAIM_CEILING {
        return Err(SettlementError(
            "snapshot claim ceiling is missing or widened".into(),
        ));
    }
    if !is_sha256_hex(&snapshot.payload.canonical_input_sha256) {
        return Err(SettlementError(
            "snapshot input digest is malformed".into(),
        ));
    }
    let encoded = serde_json::to_vec(&snapshot.payload)
        .map_err(|error| SettlementError(format!("serialize snapshot receipt: {error}")))?;
    if sha256_hex(&encoded) != snapshot.payload_sha256 {
        return Err(SettlementError("snapshot payload digest mismatch".into()));
    }
    Ok(())
}

fn derive_recovery_summaries(
    ticks: &[SettlementTickReceipt],
    stability_window_ticks: u64,
    floor_ids: &BTreeSet<String>,
) -> Result<Vec<FloorRecoverySummary>, SettlementError> {
    let window = usize::try_from(stability_window_ticks)
        .map_err(|_| SettlementError("stability window does not fit usize".into()))?;
    let mut summaries = Vec::with_capacity(floor_ids.len());

    for floor_id in floor_ids {
        let floor_points: Vec<&FloorStatus> = ticks
            .iter()
            .map(|tick| {
                tick.snapshot
                    .payload
                    .service_floors
                    .iter()
                    .find(|floor| floor.floor_id == *floor_id)
                    .map(|floor| &floor.status)
                    .ok_or_else(|| {
                        SettlementError(format!(
                            "missing floor {floor_id} in temporal receipt"
                        ))
                    })
            })
            .collect::<Result<_, _>>()?;

        let first_loss = floor_points
            .iter()
            .position(|status| **status != FloorStatus::Met);
        let restoration = match first_loss {
            Some(loss) => floor_points
                .iter()
                .enumerate()
                .skip(loss + 1)
                .find(|(_, status)| **status == FloorStatus::Met)
                .map(|(index, _)| index),
            None => None,
        };

        let stable_start = first_loss.and_then(|loss| {
            let last_start = floor_points.len().checked_sub(window)?;
            if loss + 1 > last_start {
                return None;
            }
            (loss + 1..=last_start).find(|start| {
                floor_points[*start..*start + window]
                    .iter()
                    .all(|status| **status == FloorStatus::Met)
            })
        });
        let later_non_met_ticks = stable_start
            .map(|start| {
                floor_points
                    .iter()
                    .enumerate()
                    .skip(start + window)
                    .filter(|(_, status)| **status != FloorStatus::Met)
                    .map(|(index, _)| ticks[index].tick)
                    .collect()
            })
            .unwrap_or_default();

        summaries.push(FloorRecoverySummary {
            floor_id: floor_id.clone(),
            first_non_met_tick: first_loss.map(|index| ticks[index].tick),
            first_restoration_tick: restoration.map(|index| ticks[index].tick),
            stable_recovery_tick: stable_start.map(|index| ticks[index].tick),
            non_met_tick_count: floor_points
                .iter()
                .filter(|status| **status != FloorStatus::Met)
                .count() as u64,
            later_non_met_ticks,
        });
    }
    Ok(summaries)
}

fn is_sha256_hex(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
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
