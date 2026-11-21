// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent contract checks for the deterministic Mk0 scenario event stream.
//!
//! This verifies only the internal synthetic event contract. It does not verify
//! physical fabrication, dimensional conformity, machine safety, or productive
//! closure. Those require separate evidence and physical qualification.

use crate::mk0::{
    Mk0RecordKind, Mk0RecordPayload, Mk0ScenarioEvent, Mk0ScenarioReport, Mk0Station,
};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;

/// Machine-readable reasons why an Mk0 event stream is internally inconsistent.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Mk0VerificationViolation {
    InvalidEnergyReserve,
    InvalidMaterialRequirement,
    EmptyRecordId,
    DuplicateRecordId,
    EventTimeRegressed,
    PayloadKindMismatch,
    PayloadIdMismatch,
    EventStationMismatch,
    MissingWorkOrder,
    DuplicateWorkOrder,
    WorkOrderIdentityMismatch,
    InvalidMaterialBatch,
    MaterialTotalMismatch,
    MissingEnergyWindow,
    DuplicateEnergyWindow,
    EnergyWindowMismatch,
    MissingPrintClaim,
    DuplicatePrintClaim,
    UnexpectedPrintClaim,
    PrintWorkOrderMismatch,
    MissingDelivery,
    DuplicateDelivery,
    UnexpectedDelivery,
    DeliveryWorkOrderMismatch,
    DeliveryRouteMismatch,
    MissingHandoff,
    DuplicateHandoff,
    UnexpectedHandoff,
    HandoffWorkOrderMismatch,
    HandoffReceiverMismatch,
    InvalidMaintenanceFlag,
    CausalOrderViolation,
    ReportCountersMismatch,
    SuccessClaimMismatch,
}

/// Result of checking the Mk0 synthetic event contract.
///
/// `contract_valid` means the event stream is internally coherent and its
/// summary fields agree with the events. `synthetic_run_completed` means that
/// this coherent synthetic run reports a completed work-order handoff. Neither
/// field is evidence that a physical part was manufactured or accepted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Mk0VerificationReport {
    pub contract_valid: bool,
    pub synthetic_run_completed: bool,
    pub observed_material_grams: u32,
    pub observed_work_orders: u32,
    pub observed_handoff_receipts: u32,
    pub violations: Vec<Mk0VerificationViolation>,
}

/// Independently verify event identity, chronology, prerequisites, handoffs,
/// and report counters for one deterministic Mk0 scenario.
///
/// The verifier consumes the event stream rather than trusting the report's
/// `success`, `completed_work_orders`, or `handoff_receipts` fields.
pub fn verify_mk0_bootstrapper_report(report: &Mk0ScenarioReport) -> Mk0VerificationReport {
    let config = &report.config;
    let mut violations = Vec::new();

    let reserve_valid = config.initial_energy_reserve.is_finite()
        && (0.0..=1.0).contains(&config.initial_energy_reserve);
    if !reserve_valid {
        violations.push(Mk0VerificationViolation::InvalidEnergyReserve);
    }
    if config.subassembly.required_material_grams == 0 {
        violations.push(Mk0VerificationViolation::InvalidMaterialRequirement);
    }

    let expected_energy_allowed = reserve_valid && config.initial_energy_reserve >= 0.25;

    let mut seen_record_ids = HashSet::new();
    let mut previous_tick: Option<u32> = None;

    for event in &report.events {
        if event.record_id.trim().is_empty() {
            violations.push(Mk0VerificationViolation::EmptyRecordId);
        }
        if !seen_record_ids.insert(event.record_id.clone()) {
            violations.push(Mk0VerificationViolation::DuplicateRecordId);
        }

        if let Some(previous) = previous_tick
            && event.tick < previous
        {
            violations.push(Mk0VerificationViolation::EventTimeRegressed);
        }
        previous_tick = Some(event.tick);

        if !payload_matches_kind(event) {
            violations.push(Mk0VerificationViolation::PayloadKindMismatch);
        }
        if payload_record_id(&event.payload) != event.record_id {
            violations.push(Mk0VerificationViolation::PayloadIdMismatch);
        }
        if event.station != expected_event_station(event, config.subassembly.printer_station) {
            violations.push(Mk0VerificationViolation::EventStationMismatch);
        }
        if let Mk0RecordPayload::MaintenanceFlag(flag) = &event.payload
            && flag.reason.trim().is_empty()
        {
            violations.push(Mk0VerificationViolation::InvalidMaintenanceFlag);
        }
    }

    let work_orders: Vec<_> = report
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            Mk0RecordPayload::WorkOrder(value) => Some(value),
            _ => None,
        })
        .collect();

    if work_orders.is_empty() {
        violations.push(Mk0VerificationViolation::MissingWorkOrder);
    } else if work_orders.len() > 1 {
        violations.push(Mk0VerificationViolation::DuplicateWorkOrder);
    }

    let work_order_valid = work_orders.len() == 1
        && work_orders[0].id == config.work_order_id
        && work_orders[0].subassembly == config.subassembly.display_name
        && work_orders[0].requested_by == Mk0Station::SeedNode;

    if work_orders.len() == 1 && !work_order_valid {
        violations.push(Mk0VerificationViolation::WorkOrderIdentityMismatch);
    }

    let material_batches: Vec<_> = report
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            Mk0RecordPayload::MaterialBatch(value) => Some(value),
            _ => None,
        })
        .collect();

    let observed_material_grams = material_batches.iter().fold(0u32, |total, batch| {
        total.saturating_add(batch.available_grams)
    });
    let expected_material_grams = if config.allow_recycled_feedstock {
        config.initial_feedstock_grams.min(
            config
                .subassembly
                .required_material_grams
                .saturating_add(70),
        )
    } else {
        0
    };
    let expected_material_batches = if expected_material_grams > 0 { 1 } else { 0 };

    let material_batches_valid = material_batches.len() == expected_material_batches
        && material_batches.iter().all(|batch| {
            config.allow_recycled_feedstock
                && batch.recycled
                && batch.available_grams > 0
                && !batch.source.trim().is_empty()
        });

    if !material_batches_valid {
        violations.push(Mk0VerificationViolation::InvalidMaterialBatch);
    }
    if observed_material_grams != report.recycled_feedstock_grams
        || observed_material_grams != expected_material_grams
    {
        violations.push(Mk0VerificationViolation::MaterialTotalMismatch);
    }

    let enough_material = material_batches_valid
        && expected_material_grams > 0
        && config.subassembly.required_material_grams > 0
        && observed_material_grams >= config.subassembly.required_material_grams;

    let energy_windows: Vec<_> = report
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            Mk0RecordPayload::EnergyWindow(value) => Some(value),
            _ => None,
        })
        .collect();

    if energy_windows.is_empty() {
        violations.push(Mk0VerificationViolation::MissingEnergyWindow);
    } else if energy_windows.len() > 1 {
        violations.push(Mk0VerificationViolation::DuplicateEnergyWindow);
    }

    if report.energy_window_open != expected_energy_allowed
        || energy_windows.len() != 1
        || energy_windows
            .first()
            .is_some_and(|window| window.fabrication_allowed != expected_energy_allowed)
    {
        violations.push(Mk0VerificationViolation::EnergyWindowMismatch);
    }

    let print_claims: Vec<_> = report
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            Mk0RecordPayload::PrintClaim(value) => Some(value),
            _ => None,
        })
        .collect();

    let should_print = work_order_valid && expected_energy_allowed && enough_material;
    if should_print && print_claims.is_empty() {
        violations.push(Mk0VerificationViolation::MissingPrintClaim);
    } else if should_print && print_claims.len() > 1 {
        violations.push(Mk0VerificationViolation::DuplicatePrintClaim);
    } else if !should_print && !print_claims.is_empty() {
        violations.push(Mk0VerificationViolation::UnexpectedPrintClaim);
    }
    if print_claims.iter().any(|claim| {
        claim.work_order_id != config.work_order_id || claim.recipe_version.trim().is_empty()
    }) {
        violations.push(Mk0VerificationViolation::PrintWorkOrderMismatch);
    }

    let has_delivery_route = config.room_layout.has_route(
        config.subassembly.printer_station,
        config.subassembly.assembler_station,
    );

    let deliveries: Vec<_> = report
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            Mk0RecordPayload::DeliveryTask(value) => Some(value),
            _ => None,
        })
        .collect();

    let should_deliver = should_print && has_delivery_route;
    if should_deliver && deliveries.is_empty() {
        violations.push(Mk0VerificationViolation::MissingDelivery);
    } else if should_deliver && deliveries.len() > 1 {
        violations.push(Mk0VerificationViolation::DuplicateDelivery);
    } else if !should_deliver && !deliveries.is_empty() {
        violations.push(Mk0VerificationViolation::UnexpectedDelivery);
    }
    if deliveries
        .iter()
        .any(|delivery| delivery.work_order_id != config.work_order_id)
    {
        violations.push(Mk0VerificationViolation::DeliveryWorkOrderMismatch);
    }
    if deliveries.iter().any(|delivery| {
        delivery.from != config.subassembly.printer_station
            || delivery.to != config.subassembly.assembler_station
    }) {
        violations.push(Mk0VerificationViolation::DeliveryRouteMismatch);
    }

    let handoffs: Vec<_> = report
        .events
        .iter()
        .filter_map(|event| match &event.payload {
            Mk0RecordPayload::HandoffReceipt(value) => Some(value),
            _ => None,
        })
        .collect();

    let should_handoff = should_deliver;
    if should_handoff && handoffs.is_empty() {
        violations.push(Mk0VerificationViolation::MissingHandoff);
    } else if should_handoff && handoffs.len() > 1 {
        violations.push(Mk0VerificationViolation::DuplicateHandoff);
    } else if !should_handoff && !handoffs.is_empty() {
        violations.push(Mk0VerificationViolation::UnexpectedHandoff);
    }
    if handoffs.iter().any(|handoff| {
        handoff.work_order_id != config.work_order_id
            || handoff.completed_subassembly != config.subassembly.display_name
    }) {
        violations.push(Mk0VerificationViolation::HandoffWorkOrderMismatch);
    }
    if handoffs
        .iter()
        .any(|handoff| handoff.accepted_by != config.subassembly.assembler_station)
    {
        violations.push(Mk0VerificationViolation::HandoffReceiverMismatch);
    }

    let work_order_position = first_position(&report.events, Mk0RecordKind::WorkOrder);
    let material_position = first_position(&report.events, Mk0RecordKind::MaterialBatch);
    let print_position = first_position(&report.events, Mk0RecordKind::PrintClaim);
    let delivery_position = first_position(&report.events, Mk0RecordKind::DeliveryTask);
    let handoff_position = first_position(&report.events, Mk0RecordKind::HandoffReceipt);

    if work_order_position.is_some_and(|position| position != 0) {
        violations.push(Mk0VerificationViolation::CausalOrderViolation);
    }
    if let (Some(work_order), Some(material)) = (work_order_position, material_position)
        && material <= work_order
    {
        violations.push(Mk0VerificationViolation::CausalOrderViolation);
    }
    if let (Some(work_order), Some(print)) = (work_order_position, print_position)
        && print <= work_order
    {
        violations.push(Mk0VerificationViolation::CausalOrderViolation);
    }
    if let (Some(material), Some(print)) = (material_position, print_position)
        && print <= material
    {
        violations.push(Mk0VerificationViolation::CausalOrderViolation);
    }
    if let (Some(print), Some(delivery)) = (print_position, delivery_position)
        && delivery <= print
    {
        violations.push(Mk0VerificationViolation::CausalOrderViolation);
    }
    if let (Some(delivery), Some(handoff)) = (delivery_position, handoff_position)
        && handoff <= delivery
    {
        violations.push(Mk0VerificationViolation::CausalOrderViolation);
    }

    let observed_work_orders = work_orders.len() as u32;
    let observed_handoff_receipts = handoffs.len() as u32;
    let expected_completed_work_orders = observed_handoff_receipts.min(1);

    if report.handoff_receipts != observed_handoff_receipts
        || report.completed_work_orders != expected_completed_work_orders
    {
        violations.push(Mk0VerificationViolation::ReportCountersMismatch);
    }

    let expected_success = should_handoff && handoffs.len() == 1;
    if report.success != expected_success {
        violations.push(Mk0VerificationViolation::SuccessClaimMismatch);
    }

    let contract_valid = violations.is_empty();
    Mk0VerificationReport {
        contract_valid,
        synthetic_run_completed: contract_valid && report.success,
        observed_material_grams,
        observed_work_orders,
        observed_handoff_receipts,
        violations,
    }
}

fn payload_matches_kind(event: &Mk0ScenarioEvent) -> bool {
    matches!(
        (&event.record_kind, &event.payload),
        (Mk0RecordKind::WorkOrder, Mk0RecordPayload::WorkOrder(_))
            | (
                Mk0RecordKind::MaterialBatch,
                Mk0RecordPayload::MaterialBatch(_)
            )
            | (Mk0RecordKind::PrintClaim, Mk0RecordPayload::PrintClaim(_))
            | (
                Mk0RecordKind::DeliveryTask,
                Mk0RecordPayload::DeliveryTask(_)
            )
            | (
                Mk0RecordKind::HandoffReceipt,
                Mk0RecordPayload::HandoffReceipt(_)
            )
            | (
                Mk0RecordKind::EnergyWindow,
                Mk0RecordPayload::EnergyWindow(_)
            )
            | (
                Mk0RecordKind::MaintenanceFlag,
                Mk0RecordPayload::MaintenanceFlag(_)
            )
    )
}

fn payload_record_id(payload: &Mk0RecordPayload) -> &str {
    match payload {
        Mk0RecordPayload::WorkOrder(value) => &value.id,
        Mk0RecordPayload::MaterialBatch(value) => &value.id,
        Mk0RecordPayload::PrintClaim(value) => &value.id,
        Mk0RecordPayload::DeliveryTask(value) => &value.id,
        Mk0RecordPayload::HandoffReceipt(value) => &value.id,
        Mk0RecordPayload::EnergyWindow(value) => &value.id,
        Mk0RecordPayload::MaintenanceFlag(value) => &value.id,
    }
}

fn expected_event_station(event: &Mk0ScenarioEvent, printer_station: Mk0Station) -> Mk0Station {
    match &event.payload {
        Mk0RecordPayload::WorkOrder(value) => value.requested_by,
        Mk0RecordPayload::MaterialBatch(_) => Mk0Station::Detritivore,
        Mk0RecordPayload::PrintClaim(_) => printer_station,
        Mk0RecordPayload::DeliveryTask(_) => Mk0Station::Vector,
        Mk0RecordPayload::HandoffReceipt(value) => value.accepted_by,
        Mk0RecordPayload::EnergyWindow(_) => Mk0Station::Helios,
        Mk0RecordPayload::MaintenanceFlag(value) => value.station,
    }
}

fn first_position(events: &[Mk0ScenarioEvent], kind: Mk0RecordKind) -> Option<usize> {
    events.iter().position(|event| event.record_kind == kind)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mk0::{Mk0ScenarioConfig, run_mk0_bootstrapper_scenario};

    #[test]
    fn verifies_the_default_synthetic_loop() {
        let report = run_mk0_bootstrapper_scenario(Mk0ScenarioConfig::default());
        let verified = verify_mk0_bootstrapper_report(&report);

        assert!(verified.contract_valid, "{:?}", verified.violations);
        assert!(verified.synthetic_run_completed);
        assert_eq!(verified.observed_work_orders, 1);
        assert_eq!(verified.observed_handoff_receipts, 1);
    }

    #[test]
    fn insufficient_feedstock_is_a_valid_noncompletion() {
        let config = Mk0ScenarioConfig {
            initial_feedstock_grams: 50,
            ..Mk0ScenarioConfig::default()
        };
        let report = run_mk0_bootstrapper_scenario(config);
        let verified = verify_mk0_bootstrapper_report(&report);

        assert!(!report.success);
        assert!(
            !report
                .events
                .iter()
                .any(|event| event.record_kind == Mk0RecordKind::PrintClaim),
            "the simulator must not claim a print when feedstock is insufficient"
        );
        assert!(verified.contract_valid, "{:?}", verified.violations);
        assert!(!verified.synthetic_run_completed);
    }

    #[test]
    fn rejects_a_handoff_that_precedes_its_delivery() {
        let mut report = run_mk0_bootstrapper_scenario(Mk0ScenarioConfig::default());
        let delivery = report
            .events
            .iter()
            .position(|event| event.record_kind == Mk0RecordKind::DeliveryTask)
            .unwrap();
        let handoff = report
            .events
            .iter()
            .position(|event| event.record_kind == Mk0RecordKind::HandoffReceipt)
            .unwrap();
        report.events.swap(delivery, handoff);

        let verified = verify_mk0_bootstrapper_report(&report);
        assert!(!verified.contract_valid);
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::EventTimeRegressed)
        );
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::CausalOrderViolation)
        );
    }

    #[test]
    fn rejects_missing_material_evidence_for_a_reported_completion() {
        let mut report = run_mk0_bootstrapper_scenario(Mk0ScenarioConfig::default());
        report
            .events
            .retain(|event| event.record_kind != Mk0RecordKind::MaterialBatch);

        let verified = verify_mk0_bootstrapper_report(&report);
        assert!(!verified.contract_valid);
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::MaterialTotalMismatch)
        );
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::UnexpectedPrintClaim)
        );
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::SuccessClaimMismatch)
        );
    }

    #[test]
    fn rejects_duplicate_event_identity() {
        let mut report = run_mk0_bootstrapper_scenario(Mk0ScenarioConfig::default());
        report.events[1].record_id = report.events[0].record_id.clone();

        let verified = verify_mk0_bootstrapper_report(&report);
        assert!(!verified.contract_valid);
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::DuplicateRecordId)
        );
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::PayloadIdMismatch)
        );
    }

    #[test]
    fn rejects_event_kind_payload_mismatch() {
        let mut report = run_mk0_bootstrapper_scenario(Mk0ScenarioConfig::default());
        report.events[1].record_kind = Mk0RecordKind::WorkOrder;

        let verified = verify_mk0_bootstrapper_report(&report);
        assert!(!verified.contract_valid);
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::PayloadKindMismatch)
        );
    }

    #[test]
    fn rejects_tampered_summary_counters() {
        let mut report = run_mk0_bootstrapper_scenario(Mk0ScenarioConfig::default());
        report.handoff_receipts = 2;
        report.completed_work_orders = 2;

        let verified = verify_mk0_bootstrapper_report(&report);
        assert!(!verified.contract_valid);
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::ReportCountersMismatch)
        );
    }

    #[test]
    fn verifies_energy_constrained_room_as_valid_noncompletion() {
        let config = Mk0ScenarioConfig {
            initial_energy_reserve: 0.1,
            ..Mk0ScenarioConfig::default()
        };
        let report = run_mk0_bootstrapper_scenario(config);
        let verified = verify_mk0_bootstrapper_report(&report);

        assert!(!report.success);
        assert!(verified.contract_valid, "{:?}", verified.violations);
        assert!(!verified.synthetic_run_completed);
    }

    #[test]
    fn verifies_missing_delivery_route_as_valid_noncompletion() {
        let mut config = Mk0ScenarioConfig::default();
        config.room_layout.edges.clear();
        let report = run_mk0_bootstrapper_scenario(config);
        let verified = verify_mk0_bootstrapper_report(&report);

        assert!(!report.success);
        assert!(verified.contract_valid, "{:?}", verified.violations);
        assert!(!verified.synthetic_run_completed);
    }

    #[test]
    fn rejects_an_invalid_energy_reserve() {
        let config = Mk0ScenarioConfig {
            initial_energy_reserve: f32::NAN,
            ..Mk0ScenarioConfig::default()
        };
        let report = run_mk0_bootstrapper_scenario(config);
        let verified = verify_mk0_bootstrapper_report(&report);

        assert!(!verified.contract_valid);
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::InvalidEnergyReserve)
        );
    }

    #[test]
    fn rejects_duplicate_handoff_receipts() {
        let mut report = run_mk0_bootstrapper_scenario(Mk0ScenarioConfig::default());
        let mut duplicate = report
            .events
            .iter()
            .find(|event| event.record_kind == Mk0RecordKind::HandoffReceipt)
            .unwrap()
            .clone();
        duplicate.record_id = "handoff-duplicate".to_string();
        if let Mk0RecordPayload::HandoffReceipt(ref mut handoff) = duplicate.payload {
            handoff.id = duplicate.record_id.clone();
        }
        let energy_window_index = report
            .events
            .iter()
            .position(|event| event.record_kind == Mk0RecordKind::EnergyWindow)
            .unwrap();
        report.events.insert(energy_window_index, duplicate);

        let verified = verify_mk0_bootstrapper_report(&report);
        assert!(!verified.contract_valid);
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::DuplicateHandoff)
        );
        assert!(
            verified
                .violations
                .contains(&Mk0VerificationViolation::ReportCountersMismatch)
        );
    }
}
