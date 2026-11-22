// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Time-coupled, mobility-first flexibility envelope verifier.
//!
//! This is a proposal-only feasibility screen. It checks a supplied storage
//! dispatch profile against resource-specific reserve buckets, correlated
//! reserve scenarios, time-slot power limits and PCC import/export constraints.
//! It is not a dispatch optimizer, meter, battery model or electrical controller.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_RESOURCES: usize = 4;
const MAX_SCENARIOS: usize = 4;
const MAX_HORIZON_SLOTS: u8 = 8;
const MAX_SLOT_DURATION_SECONDS: u32 = 86_400;
const MAX_POWER_W: u32 = 100_000_000;
const CLAIM_CEILING: &str = "Deterministic model-level reserve-envelope verification only; no grid export permission, electrical dispatch authority, battery safety qualification, flight authorization, return-to-service authority or commercial performance claim.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum FlexResourceKind {
    StationaryEss,
    QualifiedBidirectionalStorage,
    AircraftPropulsionPack,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct FlexStorageResource {
    pub resource_id: String,
    pub configuration_digest: String,
    pub resource_kind: FlexResourceKind,
    pub available_energy_lower_bound_wh: u64,
    pub max_discharge_power_lower_bound_w: u32,
    pub energy_evidence: EvidenceEnvelope,
    pub power_evidence: EvidenceEnvelope,
    pub availability_evidence: EvidenceEnvelope,
    pub interface_qualification_evidence: EvidenceEnvelope,
    pub site_connection_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceReserve {
    pub resource_id: String,
    pub mission_reserve_receipt_id: String,
    pub mission_reserve_evidence: EvidenceEnvelope,
    pub inventory_buffer_receipt_id: String,
    pub inventory_buffer_evidence: EvidenceEnvelope,
    pub safety_outage_reserve_receipt_id: String,
    pub safety_outage_reserve_evidence: EvidenceEnvelope,
    pub uncertainty_reserve_receipt_id: String,
    pub uncertainty_reserve_evidence: EvidenceEnvelope,
    pub mission_energy_reserve_wh: u64,
    pub inventory_buffer_energy_wh: u64,
    pub safety_outage_reserve_energy_wh: u64,
    pub uncertainty_energy_allowance_wh: u64,
    pub mission_power_reserve_w_by_slot: Vec<u32>,
    pub inventory_buffer_power_w_by_slot: Vec<u32>,
    pub safety_outage_power_reserve_w_by_slot: Vec<u32>,
    pub uncertainty_power_allowance_w_by_slot: Vec<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReserveScenario {
    pub scenario_id: String,
    pub evidence: EvidenceEnvelope,
    pub resource_reserves: Vec<ResourceReserve>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PccSlot {
    pub slot: u8,
    pub base_site_load_w: u32,
    pub scheduled_mobility_charging_load_w: u32,
    pub import_limit_w: u32,
    pub export_limit_w: u32,
    pub base_load_evidence: EvidenceEnvelope,
    pub scheduled_charging_load_evidence: EvidenceEnvelope,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateDispatchSlot {
    pub slot: u8,
    pub requested_discharge_power_w: u32,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityFlexibilityInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub decision_time_unix_ms: u64,
    pub slot_duration_seconds: u32,
    pub horizon_slots: u8,
    pub site_evidence: EvidenceEnvelope,
    pub allocation_evidence: EvidenceEnvelope,
    pub charging_queue_evidence: EvidenceEnvelope,
    pub site_meter_evidence: EvidenceEnvelope,
    pub storage_resources: Vec<FlexStorageResource>,
    pub reserve_scenarios: Vec<ReserveScenario>,
    pub pcc_slots: Vec<PccSlot>,
    pub candidate_dispatch: Vec<CandidateDispatchSlot>,
    pub export_permission: Option<crate::battery_service_network::BooleanEvidence>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SlotPowerMargin {
    pub slot: u8,
    pub worst_case_power_ceiling_w: u32,
    pub requested_discharge_power_w: u32,
    pub power_margin_w: i64,
    pub pcc_net_import_w: i64,
    pub pcc_import_limit_w: u32,
    pub pcc_export_limit_w: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ScenarioEvaluation {
    pub scenario_id: String,
    pub residual_energy_budget_wh: u64,
    pub candidate_energy_wh: u64,
    pub energy_margin_wh: i64,
    pub residual_power_by_slot_w: Vec<u32>,
    pub reserve_feasible: bool,
    pub energy_feasible: bool,
    pub power_feasible: bool,
    pub pcc_feasible: bool,
    pub model_feasible: bool,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MobilityFlexibilityReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub decision_time_unix_ms: u64,
    pub horizon_slots: u8,
    pub storage_resource_count: usize,
    pub reserve_scenario_count: usize,
    pub total_available_energy_lower_bound_wh: u64,
    pub candidate_energy_wh: u64,
    pub worst_case_residual_energy_budget_wh: u64,
    pub worst_case_energy_margin_wh: i64,
    pub slot_power_margins: Vec<SlotPowerMargin>,
    pub scenario_results: Vec<ScenarioEvaluation>,
    pub candidate_model_feasible: bool,
    pub all_evidence_source_grade: bool,
    pub all_evidence_synthetic: bool,
    pub export_permission_evidenced: bool,
    pub dispatch_authority_granted: bool,
    pub export_authority_granted: bool,
    pub findings: Vec<String>,
    pub claim_ceiling: &'static str,
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

fn evidence_covers(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    decision_time_unix_ms: u64,
    interval_start_unix_ms: u64,
    interval_end_exclusive_unix_ms: u64,
) -> bool {
    if evidence.asset_id != asset_id
        || evidence.configuration_digest != configuration_digest
        || evidence.asset_id.trim().is_empty()
        || evidence.configuration_digest.trim().is_empty()
        || evidence.producer_id.trim().is_empty()
        || evidence.method_id.trim().is_empty()
        || evidence.evidence_reference.trim().is_empty()
        || evidence.evidence_class == EvidenceClass::Unknown
        || interval_end_exclusive_unix_ms <= interval_start_unix_ms
    {
        return false;
    }

    match (
        evidence.observed_at_unix_ms,
        evidence.valid_from_unix_ms,
        evidence.valid_until_unix_ms,
    ) {
        (Some(observed_at), Some(valid_from), Some(valid_until)) => {
            valid_from <= observed_at
                && observed_at <= valid_until
                && observed_at <= decision_time_unix_ms
                && valid_from <= interval_start_unix_ms
                && interval_end_exclusive_unix_ms.saturating_sub(1) <= valid_until
        }
        _ => false,
    }
}

fn slot_time_ms(
    decision_time_unix_ms: u64,
    slot_duration_seconds: u32,
    slot: u8,
) -> Option<u64> {
    u64::from(slot)
        .checked_mul(u64::from(slot_duration_seconds))?
        .checked_mul(1_000)?
        .checked_add(decision_time_unix_ms)
}

fn checked_sum(values: impl IntoIterator<Item = u64>) -> Option<u64> {
    values
        .into_iter()
        .try_fold(0_u64, |total, value| total.checked_add(value))
}

fn evidence_is_source_grade(evidence: &EvidenceEnvelope) -> bool {
    matches!(
        evidence.evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    )
}

fn push_evidence_class(
    evidence: &EvidenceEnvelope,
    all_source_grade: &mut bool,
    all_synthetic: &mut bool,
) {
    *all_source_grade &= evidence_is_source_grade(evidence);
    *all_synthetic &= evidence.evidence_class == EvidenceClass::Synthetic;
}

fn reserve_energy_wh(reserve: &ResourceReserve) -> Option<u64> {
    checked_sum([
        reserve.mission_energy_reserve_wh,
        reserve.inventory_buffer_energy_wh,
        reserve.safety_outage_reserve_energy_wh,
        reserve.uncertainty_energy_allowance_wh,
    ])
}

fn reserve_power_w(reserve: &ResourceReserve, slot: usize) -> Option<u32> {
    reserve.mission_power_reserve_w_by_slot[slot]
        .checked_add(reserve.inventory_buffer_power_w_by_slot[slot])?
        .checked_add(reserve.safety_outage_power_reserve_w_by_slot[slot])?
        .checked_add(reserve.uncertainty_power_allowance_w_by_slot[slot])
}

fn candidate_energy_wh(
    profile: &[CandidateDispatchSlot],
    slot_duration_seconds: u32,
) -> Option<u64> {
    profile.iter().try_fold(0_u64, |total, item| {
        let watt_seconds = u64::from(item.requested_discharge_power_w)
            .checked_mul(u64::from(slot_duration_seconds))?;
        // Round each interval upward so fractional Wh never understate use.
        let interval_wh = watt_seconds.checked_add(3_599)? / 3_600;
        total.checked_add(interval_wh)
    })
}

fn signed_margin(limit: u64, requested: u64) -> i64 {
    if limit >= requested {
        i64::try_from(limit - requested).unwrap_or(i64::MAX)
    } else {
        -i64::try_from(requested - limit).unwrap_or(i64::MAX)
    }
}

pub fn evaluate_mobility_flexibility(
    input: &MobilityFlexibilityInput,
) -> Result<MobilityFlexibilityReport, Vec<String>> {
    let mut errors = Vec::new();

    if input.schema_version != "0.1.0"
        || input.benchmark_id != "MOBILITY-FLEX-ENVELOPE-001"
        || input.site_id.trim().is_empty()
        || input.site_configuration_digest.trim().is_empty()
        || input.claim_ceiling.trim().is_empty()
        || input.decision_time_unix_ms == 0
        || input.slot_duration_seconds == 0
        || input.slot_duration_seconds > MAX_SLOT_DURATION_SECONDS
        || input.horizon_slots == 0
        || input.horizon_slots > MAX_HORIZON_SLOTS
    {
        add_error(&mut errors, "unsupported_identity_or_time_horizon");
    }

    if input.storage_resources.is_empty()
        || input.storage_resources.len() > MAX_RESOURCES
        || input.reserve_scenarios.is_empty()
        || input.reserve_scenarios.len() > MAX_SCENARIOS
        || input.pcc_slots.len() != usize::from(input.horizon_slots)
        || input.candidate_dispatch.len() != usize::from(input.horizon_slots)
    {
        add_error(&mut errors, "resource_scenario_or_horizon_bound_invalid");
    }

    let horizon_end = slot_time_ms(
        input.decision_time_unix_ms,
        input.slot_duration_seconds,
        input.horizon_slots,
    );
    let Some(horizon_end) = horizon_end else {
        add_error(&mut errors, "horizon_time_overflow");
        return Err(errors);
    };

    for (name, evidence) in [
        ("site", &input.site_evidence),
        ("allocation", &input.allocation_evidence),
        ("charging_queue", &input.charging_queue_evidence),
        ("site_meter", &input.site_meter_evidence),
    ] {
        if !evidence_covers(
            evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            input.decision_time_unix_ms,
            horizon_end,
        ) {
            add_error(&mut errors, format!("{name}_evidence_not_admissible_for_horizon"));
        }
    }

    let mut all_source_grade = true;
    let mut all_synthetic = true;
    for evidence in [
        &input.site_evidence,
        &input.allocation_evidence,
        &input.charging_queue_evidence,
        &input.site_meter_evidence,
    ] {
        push_evidence_class(evidence, &mut all_source_grade, &mut all_synthetic);
    }

    let mut resource_map = BTreeMap::new();
    let mut total_available_energy = 0_u64;
    for resource in &input.storage_resources {
        if resource.resource_id.trim().is_empty()
            || resource.configuration_digest.trim().is_empty()
            || resource.available_energy_lower_bound_wh == 0
            || resource.max_discharge_power_lower_bound_w == 0
            || resource.max_discharge_power_lower_bound_w > MAX_POWER_W
            || resource.resource_kind == FlexResourceKind::AircraftPropulsionPack
            || resource_map.insert(resource.resource_id.as_str(), resource).is_some()
        {
            add_error(
                &mut errors,
                format!("storage_identity_capability_or_class_invalid:{}", resource.resource_id),
            );
            continue;
        }

        for (label, evidence) in [
            ("energy", &resource.energy_evidence),
            ("power", &resource.power_evidence),
            ("availability", &resource.availability_evidence),
            ("interface", &resource.interface_qualification_evidence),
        ] {
            if !evidence_covers(
                evidence,
                &resource.resource_id,
                &resource.configuration_digest,
                input.decision_time_unix_ms,
                input.decision_time_unix_ms,
                horizon_end,
            ) {
                add_error(
                    &mut errors,
                    format!("{label}_evidence_not_admissible:{}", resource.resource_id),
                );
            }
            push_evidence_class(evidence, &mut all_source_grade, &mut all_synthetic);
        }
        if !evidence_covers(
            &resource.site_connection_evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            input.decision_time_unix_ms,
            horizon_end,
        ) {
            add_error(
                &mut errors,
                format!("site_connection_evidence_not_admissible:{}", resource.resource_id),
            );
        }
        push_evidence_class(
            &resource.site_connection_evidence,
            &mut all_source_grade,
            &mut all_synthetic,
        );

        if resource.resource_kind == FlexResourceKind::QualifiedBidirectionalStorage
            && resource.interface_qualification_evidence.evidence_class
                != EvidenceClass::IndependentTest
        {
            add_error(
                &mut errors,
                format!("bidirectional_interface_requires_independent_test:{}", resource.resource_id),
            );
        }

        total_available_energy = match total_available_energy
            .checked_add(resource.available_energy_lower_bound_wh)
        {
            Some(total) => total,
            None => {
                add_error(&mut errors, "total_available_energy_overflow");
                total_available_energy
            }
        };
    }

    if let Some(permission) = &input.export_permission {
        if !evidence_covers(
            &permission.evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            input.decision_time_unix_ms,
            horizon_end,
        ) {
            add_error(&mut errors, "export_permission_evidence_not_admissible");
        }
        if permission.value.is_none() {
            add_error(&mut errors, "export_permission_present_but_unknown");
        }
        if permission.value == Some(true)
            && matches!(
                permission.evidence.evidence_class,
                EvidenceClass::Synthetic | EvidenceClass::Unknown
            )
        {
            add_error(&mut errors, "export_permission_is_not_source_grade");
        }
        push_evidence_class(&permission.evidence, &mut all_source_grade, &mut all_synthetic);
    }
    let export_permitted = input
        .export_permission
        .as_ref()
        .is_some_and(|permission| {
            permission.value == Some(true)
                && matches!(
                    permission.evidence.evidence_class,
                    EvidenceClass::Measured | EvidenceClass::IndependentTest
                )
        });

    let mut sorted_pcc = input.pcc_slots.clone();
    sorted_pcc.sort_by_key(|slot| slot.slot);
    let mut pcc_by_slot = BTreeMap::new();
    for (index, pcc) in sorted_pcc.iter().enumerate() {
        let interval_start = slot_time_ms(
            input.decision_time_unix_ms,
            input.slot_duration_seconds,
            pcc.slot,
        );
        let interval_end = pcc
            .slot
            .checked_add(1)
            .and_then(|end_slot| slot_time_ms(input.decision_time_unix_ms, input.slot_duration_seconds, end_slot));
        let pcc_evidence_valid = matches!(
            (interval_start, interval_end),
            (Some(start), Some(end)) if evidence_covers(
                &pcc.evidence,
                &input.site_id,
                &input.site_configuration_digest,
                input.decision_time_unix_ms,
                start,
                end,
            )
        );
        let slot_evidence_valid = matches!(
            (interval_start, interval_end),
            (Some(start), Some(end)) if evidence_covers(
                &pcc.base_load_evidence,
                &input.site_id,
                &input.site_configuration_digest,
                input.decision_time_unix_ms,
                start,
                end,
            ) && evidence_covers(
                &pcc.scheduled_charging_load_evidence,
                &input.site_id,
                &input.site_configuration_digest,
                input.decision_time_unix_ms,
                start,
                end,
            )
        );
        let load_receipts_match = pcc.base_load_evidence.evidence_reference
            == input.site_meter_evidence.evidence_reference
            && pcc.scheduled_charging_load_evidence.evidence_reference
                == input.charging_queue_evidence.evidence_reference;
        if pcc.slot != index as u8
            || pcc.import_limit_w > MAX_POWER_W
            || pcc.export_limit_w > MAX_POWER_W
            || pcc.base_site_load_w > MAX_POWER_W
            || pcc.scheduled_mobility_charging_load_w > MAX_POWER_W
            || !pcc_evidence_valid
            || !slot_evidence_valid
            || !load_receipts_match
            || pcc_by_slot.insert(pcc.slot, pcc).is_some()
        {
            add_error(&mut errors, format!("pcc_slot_or_evidence_invalid:{}", pcc.slot));
        }
        push_evidence_class(&pcc.evidence, &mut all_source_grade, &mut all_synthetic);
        push_evidence_class(&pcc.base_load_evidence, &mut all_source_grade, &mut all_synthetic);
        push_evidence_class(&pcc.scheduled_charging_load_evidence, &mut all_source_grade, &mut all_synthetic);
    }

    let mut sorted_profile = input.candidate_dispatch.clone();
    sorted_profile.sort_by_key(|slot| slot.slot);
    let mut profile_by_slot = BTreeMap::new();
    for (index, item) in sorted_profile.iter().enumerate() {
        if item.slot != index as u8
            || item.requested_discharge_power_w > MAX_POWER_W
            || profile_by_slot.insert(item.slot, item).is_some()
        {
            add_error(
                &mut errors,
                format!("candidate_dispatch_slot_invalid:{}", item.slot),
            );
        }
    }

    let mut sorted_scenarios = input.reserve_scenarios.clone();
    sorted_scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let mut scenario_ids = BTreeSet::new();
    let mut scenario_results = Vec::new();
    for scenario in &sorted_scenarios {
        let mut findings = Vec::new();
        let mut reserve_feasible = true;
        if scenario.scenario_id.trim().is_empty() || !scenario_ids.insert(scenario.scenario_id.as_str()) {
            add_error(&mut errors, "reserve_scenario_identity_duplicate_or_empty");
            continue;
        }
        if !evidence_covers(
            &scenario.evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            input.decision_time_unix_ms,
            horizon_end,
        ) {
            add_error(
                &mut errors,
                format!("reserve_scenario_evidence_not_admissible:{}", scenario.scenario_id),
            );
        }
        push_evidence_class(&scenario.evidence, &mut all_source_grade, &mut all_synthetic);

        if scenario.resource_reserves.len() != input.storage_resources.len() {
            add_error(
                &mut errors,
                format!("scenario_resource_coverage_incomplete:{}", scenario.scenario_id),
            );
        }

        let mut reserves_by_resource = BTreeMap::new();
        let mut reserve_receipt_ids = BTreeSet::new();
        for reserve in &scenario.resource_reserves {
            if reserve.resource_id.trim().is_empty()
                || !resource_map.contains_key(reserve.resource_id.as_str())
                || reserves_by_resource
                    .insert(reserve.resource_id.as_str(), reserve)
                    .is_some()
            {
                add_error(
                    &mut errors,
                    format!("scenario_resource_reserve_duplicate:{}", scenario.scenario_id),
                );
                continue;
            }
            for receipt_id in [
                &reserve.mission_reserve_receipt_id,
                &reserve.inventory_buffer_receipt_id,
                &reserve.safety_outage_reserve_receipt_id,
                &reserve.uncertainty_reserve_receipt_id,
            ] {
                if receipt_id.trim().is_empty() || !reserve_receipt_ids.insert(receipt_id.as_str()) {
                    add_error(
                        &mut errors,
                        format!("reserve_receipt_id_empty_or_reused:{}", scenario.scenario_id),
                    );
                }
            }

            let Some(resource) = resource_map.get(reserve.resource_id.as_str()) else {
                continue;
            };
            for (label, receipt_id, evidence) in [
                ("mission", &reserve.mission_reserve_receipt_id, &reserve.mission_reserve_evidence),
                ("inventory", &reserve.inventory_buffer_receipt_id, &reserve.inventory_buffer_evidence),
                ("safety_outage", &reserve.safety_outage_reserve_receipt_id, &reserve.safety_outage_reserve_evidence),
                ("uncertainty", &reserve.uncertainty_reserve_receipt_id, &reserve.uncertainty_reserve_evidence),
            ] {
                if evidence.evidence_reference.as_str() != receipt_id.as_str()
                    || !evidence_covers(
                        evidence,
                        &resource.resource_id,
                        &resource.configuration_digest,
                        input.decision_time_unix_ms,
                        input.decision_time_unix_ms,
                        horizon_end,
                    )
                {
                    add_error(
                        &mut errors,
                        format!("reserve_bucket_evidence_not_bound:{label}:{}", reserve.resource_id),
                    );
                }
                push_evidence_class(evidence, &mut all_source_grade, &mut all_synthetic);
            }

            let correct_len = usize::from(input.horizon_slots);
            if reserve.mission_power_reserve_w_by_slot.len() != correct_len
                || reserve.inventory_buffer_power_w_by_slot.len() != correct_len
                || reserve.safety_outage_power_reserve_w_by_slot.len() != correct_len
                || reserve.uncertainty_power_allowance_w_by_slot.len() != correct_len
            {
                add_error(
                    &mut errors,
                    format!("scenario_power_reserve_horizon_mismatch:{}", reserve.resource_id),
                );
                continue;
            }
        }

        let mut residual_energy = 0_u64;
        let mut residual_power_by_slot = vec![0_u32; usize::from(input.horizon_slots)];
        for resource in &input.storage_resources {
            let Some(reserve) = reserves_by_resource.get(resource.resource_id.as_str()) else {
                reserve_feasible = false;
                findings.push(format!("resource_reserve_missing:{}", resource.resource_id));
                continue;
            };

            let Some(reserved_energy) = reserve_energy_wh(reserve) else {
                add_error(
                    &mut errors,
                    format!("reserve_energy_overflow:{}", resource.resource_id),
                );
                reserve_feasible = false;
                continue;
            };
            if reserved_energy > resource.available_energy_lower_bound_wh {
                reserve_feasible = false;
                findings.push(format!("energy_reserve_exceeds_available:{}", resource.resource_id));
            } else {
                residual_energy = match residual_energy
                    .checked_add(resource.available_energy_lower_bound_wh - reserved_energy)
                {
                    Some(value) => value,
                    None => {
                        add_error(&mut errors, "residual_energy_overflow");
                        residual_energy
                    }
                };
            }

            for slot in 0..usize::from(input.horizon_slots) {
                let Some(reserved_power) = reserve_power_w(reserve, slot) else {
                    add_error(
                        &mut errors,
                        format!("reserve_power_overflow:{}:{slot}", resource.resource_id),
                    );
                    reserve_feasible = false;
                    continue;
                };
                if reserved_power > resource.max_discharge_power_lower_bound_w {
                    reserve_feasible = false;
                    findings.push(format!(
                        "power_reserve_exceeds_available:{}:{slot}",
                        resource.resource_id
                    ));
                    continue;
                }
                let residual = resource.max_discharge_power_lower_bound_w - reserved_power;
                residual_power_by_slot[slot] = match residual_power_by_slot[slot].checked_add(residual) {
                    Some(value) => value,
                    None => {
                        add_error(&mut errors, "residual_power_overflow");
                        residual_power_by_slot[slot]
                    }
                };
            }
        }

        let mut energy_feasible = reserve_feasible;
        let candidate_energy = candidate_energy_wh(
            &sorted_profile,
            input.slot_duration_seconds,
        );
        let Some(candidate_energy) = candidate_energy else {
            add_error(&mut errors, "candidate_energy_arithmetic_overflow");
            return Err(errors);
        };
        if candidate_energy > residual_energy {
            energy_feasible = false;
            findings.push("candidate_exceeds_horizon_residual_energy".to_owned());
        }

        let mut power_feasible = reserve_feasible;
        let mut pcc_feasible = true;
        for slot in 0..usize::from(input.horizon_slots) {
            let slot_id = slot as u8;
            let Some(candidate) = profile_by_slot.get(&slot_id) else {
                pcc_feasible = false;
                power_feasible = false;
                continue;
            };
            let requested = candidate.requested_discharge_power_w;
            if requested > residual_power_by_slot[slot] {
                power_feasible = false;
                findings.push(format!("candidate_exceeds_residual_power:{slot_id}"));
            }

            let Some(pcc) = pcc_by_slot.get(&slot_id) else {
                pcc_feasible = false;
                continue;
            };
            let site_load = i64::from(pcc.base_site_load_w)
                + i64::from(pcc.scheduled_mobility_charging_load_w);
            let net_import = site_load - i64::from(requested);
            if net_import > i64::from(pcc.import_limit_w) {
                pcc_feasible = false;
                findings.push(format!("pcc_import_limit_exceeded:{slot_id}"));
            }
            if net_import < 0 {
                let export_w = net_import.unsigned_abs();
                if !export_permitted {
                    pcc_feasible = false;
                    findings.push(format!("pcc_export_not_permitted:{slot_id}"));
                } else if export_w > u64::from(pcc.export_limit_w) {
                    pcc_feasible = false;
                    findings.push(format!("pcc_export_limit_exceeded:{slot_id}"));
                }
            }
        }

        let model_feasible = reserve_feasible && energy_feasible && power_feasible && pcc_feasible;
        findings.sort();
        findings.dedup();
        scenario_results.push(ScenarioEvaluation {
            scenario_id: scenario.scenario_id.clone(),
            residual_energy_budget_wh: residual_energy,
            candidate_energy_wh: candidate_energy,
            energy_margin_wh: signed_margin(residual_energy, candidate_energy),
            residual_power_by_slot_w: residual_power_by_slot,
            reserve_feasible,
            energy_feasible,
            power_feasible,
            pcc_feasible,
            model_feasible,
            findings,
        });
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let candidate_energy = candidate_energy_wh(&sorted_profile, input.slot_duration_seconds)
        .ok_or_else(|| vec!["candidate_energy_arithmetic_overflow".to_owned()])?;
    let worst_case_energy_budget = scenario_results
        .iter()
        .map(|result| result.residual_energy_budget_wh)
        .min()
        .unwrap_or(0);
    let worst_case_energy_margin = signed_margin(worst_case_energy_budget, candidate_energy);

    let mut slot_power_margins = Vec::new();
    let mut aggregate_findings = BTreeSet::new();
    for result in &scenario_results {
        for finding in &result.findings {
            aggregate_findings.insert(finding.clone());
        }
    }
    for slot in 0..usize::from(input.horizon_slots) {
        let slot_id = slot as u8;
        let worst_case_ceiling = scenario_results
            .iter()
            .map(|result| result.residual_power_by_slot_w[slot])
            .min()
            .unwrap_or(0);
        let requested = profile_by_slot
            .get(&slot_id)
            .map_or(0, |candidate| candidate.requested_discharge_power_w);
        let pcc = pcc_by_slot.get(&slot_id);
        let site_load = pcc.map_or(0_i64, |pcc| {
            i64::from(pcc.base_site_load_w) + i64::from(pcc.scheduled_mobility_charging_load_w)
        });
        let net_import = site_load - i64::from(requested);
        slot_power_margins.push(SlotPowerMargin {
            slot: slot_id,
            worst_case_power_ceiling_w: worst_case_ceiling,
            requested_discharge_power_w: requested,
            power_margin_w: signed_margin(u64::from(worst_case_ceiling), u64::from(requested)),
            pcc_net_import_w: net_import,
            pcc_import_limit_w: pcc.map_or(0, |value| value.import_limit_w),
            pcc_export_limit_w: pcc.map_or(0, |value| value.export_limit_w),
        });
    }

    let candidate_model_feasible = !scenario_results.is_empty()
        && scenario_results.iter().all(|result| result.model_feasible);
    if !candidate_model_feasible && aggregate_findings.is_empty() {
        aggregate_findings.insert("candidate_not_feasible_in_all_scenarios".to_owned());
    }
    if !all_source_grade {
        aggregate_findings.insert("evidence_not_entirely_measured_or_independent_test".to_owned());
    }
    if !export_permitted {
        aggregate_findings.insert("grid_export_not_authorized_by_input_evidence".to_owned());
    }

    Ok(MobilityFlexibilityReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        site_id: input.site_id.clone(),
        decision_time_unix_ms: input.decision_time_unix_ms,
        horizon_slots: input.horizon_slots,
        storage_resource_count: input.storage_resources.len(),
        reserve_scenario_count: input.reserve_scenarios.len(),
        total_available_energy_lower_bound_wh: total_available_energy,
        candidate_energy_wh: candidate_energy,
        worst_case_residual_energy_budget_wh: worst_case_energy_budget,
        worst_case_energy_margin_wh: worst_case_energy_margin,
        slot_power_margins,
        scenario_results,
        candidate_model_feasible,
        all_evidence_source_grade: all_source_grade,
        all_evidence_synthetic: all_synthetic,
        export_permission_evidenced: export_permitted,
        dispatch_authority_granted: false,
        export_authority_granted: false,
        findings: aggregate_findings.into_iter().collect(),
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MobilityFlexibilityInput {
        serde_json::from_str(include_str!("../fixtures/mobility_flexibility_envelope_v0_1.json"))
            .expect("frozen mobility flexibility fixture must parse")
    }

    #[test]
    fn known_answer_is_feasible_but_proposal_only() {
        let report = evaluate_mobility_flexibility(&fixture()).expect("fixture is structurally valid");
        assert!(report.candidate_model_feasible);
        assert_eq!(report.total_available_energy_lower_bound_wh, 4_000);
        assert_eq!(report.candidate_energy_wh, 1_875);
        assert_eq!(report.worst_case_residual_energy_budget_wh, 2_000);
        assert_eq!(report.worst_case_energy_margin_wh, 125);
        assert_eq!(report.scenario_results.len(), 2);
        assert!(report.all_evidence_synthetic);
        assert!(!report.all_evidence_source_grade);
        assert!(!report.dispatch_authority_granted);
        assert!(!report.export_authority_granted);
    }

    #[test]
    fn aggregate_energy_cannot_hide_one_slot_power_violation() {
        let mut input = fixture();
        input.candidate_dispatch[1].requested_discharge_power_w = 5_000;
        let report = evaluate_mobility_flexibility(&input).expect("valid input");
        assert!(!report.candidate_model_feasible);
        assert!(report.findings.iter().any(|item| item == "candidate_exceeds_residual_power:1"));
    }

    #[test]
    fn per_slot_power_cannot_hide_horizon_energy_overdraw() {
        let mut input = fixture();
        for item in &mut input.candidate_dispatch {
            item.requested_discharge_power_w = 3_000;
        }
        let report = evaluate_mobility_flexibility(&input).expect("valid input");
        assert!(!report.candidate_model_feasible);
        assert!(report.findings.iter().any(|item| item == "candidate_exceeds_horizon_residual_energy"));
    }

    #[test]
    fn every_reserve_scenario_must_pass() {
        let mut input = fixture();
        input.reserve_scenarios[1].resource_reserves[0].mission_energy_reserve_wh = 4_000;
        let report = evaluate_mobility_flexibility(&input).expect("valid input");
        assert!(!report.candidate_model_feasible);
        assert!(report.scenario_results.iter().any(|result| {
            result.scenario_id == "high-demand-low-renewable"
                && !result.reserve_feasible
        }));
    }

    #[test]
    fn no_export_is_inferred_from_a_positive_energy_residual() {
        let mut input = fixture();
        input.candidate_dispatch[0].requested_discharge_power_w = 4_500;
        let report = evaluate_mobility_flexibility(&input).expect("valid input");
        assert!(!report.candidate_model_feasible);
        assert!(report.findings.iter().any(|item| item == "pcc_export_not_permitted:0"));
    }

    #[test]
    fn synthetic_export_permission_cannot_authorize_export() {
        let mut input = fixture();
        input.candidate_dispatch[0].requested_discharge_power_w = 4_100;
        input.export_permission = Some(crate::battery_service_network::BooleanEvidence {
            evidence: input.site_evidence.clone(),
            value: Some(true),
        });
        let result = evaluate_mobility_flexibility(&input);
        assert!(
            result
                .expect_err("synthetic permission evidence must be rejected")
                .iter()
                .any(|item| item == "export_permission_is_not_source_grade")
        );
    }

    #[test]
    fn propulsion_pack_is_never_admitted_as_grid_storage() {
        let mut input = fixture();
        input.storage_resources[0].resource_kind = FlexResourceKind::AircraftPropulsionPack;
        let result = evaluate_mobility_flexibility(&input);
        assert!(result.is_err());
    }

    #[test]
    fn stale_upstream_evidence_fails_closed() {
        let mut input = fixture();
        input.charging_queue_evidence.valid_until_unix_ms = Some(input.decision_time_unix_ms);
        let result = evaluate_mobility_flexibility(&input);
        assert!(result.is_err());
    }

    #[test]
    fn reserve_bucket_evidence_must_match_its_receipt_id() {
        let mut input = fixture();
        input.reserve_scenarios[0].resource_reserves[0]
            .mission_reserve_evidence
            .evidence_reference = "synthetic://wrong-reserve-receipt".to_owned();
        let result = evaluate_mobility_flexibility(&input);
        assert!(result.is_err());
    }

    #[test]
    fn scheduled_charging_load_must_match_the_queue_receipt() {
        let mut input = fixture();
        input.pcc_slots[0]
            .scheduled_charging_load_evidence
            .evidence_reference = "synthetic://wrong-queue-report".to_owned();
        let result = evaluate_mobility_flexibility(&input);
        assert!(result.is_err());
    }

    #[test]
    fn duplicate_reserve_receipt_id_is_rejected_within_a_scenario() {
        let mut input = fixture();
        let duplicate = input.reserve_scenarios[0].resource_reserves[0]
            .inventory_buffer_receipt_id
            .clone();
        input.reserve_scenarios[0].resource_reserves[1].inventory_buffer_receipt_id = duplicate;
        let result = evaluate_mobility_flexibility(&input);
        assert!(result.is_err());
    }

    #[test]
    fn storage_must_be_bound_to_the_same_site_configuration() {
        let mut input = fixture();
        input.storage_resources[0].site_connection_evidence.configuration_digest =
            "sha256:other-site-config".to_owned();
        let result = evaluate_mobility_flexibility(&input);
        assert!(result.is_err());
    }

    #[test]
    fn input_order_permutation_preserves_canonical_report() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.storage_resources.reverse();
        permuted.reserve_scenarios.reverse();
        permuted.pcc_slots.reverse();
        permuted.candidate_dispatch.reverse();
        let a = evaluate_mobility_flexibility(&input).expect("first input");
        let b = evaluate_mobility_flexibility(&permuted).expect("permuted input");
        assert_eq!(
            serde_json::to_string(&a).expect("serialize first"),
            serde_json::to_string(&b).expect("serialize second")
        );
    }
}
