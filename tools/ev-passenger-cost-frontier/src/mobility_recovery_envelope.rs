// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proposal-only, per-resource energy recovery and repeated-disturbance verifier.
//!
//! Charging energy is rounded down, withdrawals are rounded up, and reserve
//! floors are checked at every slot boundary. This module never actuates assets.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_RESOURCES: usize = 4;
const MAX_SCENARIOS: usize = 4;
const MAX_CHANNELS: usize = 8;
const MAX_HORIZON_SLOTS: u8 = 16;
const MAX_SLOT_DURATION_SECONDS: u32 = 86_400;
const MAX_POWER_W: u32 = 100_000_000;
const MAX_ENERGY_WH: u64 = 1_000_000_000_000;
const CLAIM_CEILING: &str = "Finite-scenario model-level state-recovery verification only; no battery safety qualification, charging command, grid dispatch permission, flight authorization, return-to-service authority or commercial performance claim.";

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryResource {
    pub resource_id: String,
    pub configuration_digest: String,
    pub initial_energy_lower_bound_wh: u64,
    pub energy_capacity_lower_bound_wh: u64,
    pub max_charge_input_power_lower_bound_w: u32,
    pub max_discharge_output_power_lower_bound_w: u32,
    pub charge_efficiency_lower_bound_permille: u16,
    pub discharge_efficiency_lower_bound_permille: u16,
    pub initial_energy_evidence: EvidenceEnvelope,
    pub capacity_evidence: EvidenceEnvelope,
    pub charge_power_evidence: EvidenceEnvelope,
    pub discharge_power_evidence: EvidenceEnvelope,
    pub efficiency_evidence: EvidenceEnvelope,
    pub site_connection_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryChargerChannel {
    pub channel_id: String,
    pub configuration_digest: String,
    pub max_input_power_lower_bound_w: u32,
    pub compatible_resource_ids: Vec<String>,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryChannelAvailability {
    pub channel_id: String,
    pub evidence: EvidenceEnvelope,
    pub available_by_slot: Vec<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryResourceObligation {
    pub resource_id: String,
    pub evidence: EvidenceEnvelope,
    pub mission_reserve_floor_wh_by_slot: Vec<u64>,
    pub inventory_reserve_floor_wh_by_slot: Vec<u64>,
    pub safety_outage_reserve_floor_wh_by_slot: Vec<u64>,
    pub uncertainty_reserve_floor_wh_by_slot: Vec<u64>,
    pub mission_energy_withdrawal_wh_by_slot: Vec<u64>,
    pub disturbance_energy_withdrawal_wh_by_slot: Vec<u64>,
    pub recovery_target_energy_wh: u64,
    pub recovery_deadline_slot: u8,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryScenario {
    pub scenario_id: String,
    pub evidence: EvidenceEnvelope,
    pub resource_obligations: Vec<RecoveryResourceObligation>,
    pub channel_availability: Vec<RecoveryChannelAvailability>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryPccSlot {
    pub slot: u8,
    pub base_site_load_w: u32,
    pub scheduled_mobility_charging_load_w: u32,
    pub import_limit_w: u32,
    pub base_load_evidence: EvidenceEnvelope,
    pub scheduled_charging_load_evidence: EvidenceEnvelope,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RecoveryScheduleSlot {
    pub resource_id: String,
    pub slot: u8,
    pub recovery_charge_input_power_w: u32,
    pub grid_discharge_output_power_w: u32,
    pub charger_channel_id: Option<String>,
    pub queue_evidence_reference: String,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityRecoveryInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub decision_time_unix_ms: u64,
    pub slot_duration_seconds: u32,
    pub horizon_slots: u8,
    pub flexibility_report_reference: String,
    pub site_evidence: EvidenceEnvelope,
    pub allocation_evidence: EvidenceEnvelope,
    pub charging_queue_evidence: EvidenceEnvelope,
    pub site_meter_evidence: EvidenceEnvelope,
    pub resources: Vec<RecoveryResource>,
    pub charger_channels: Vec<RecoveryChargerChannel>,
    pub scenarios: Vec<RecoveryScenario>,
    pub pcc_slots: Vec<RecoveryPccSlot>,
    pub recovery_schedule: Vec<RecoveryScheduleSlot>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResourceRecoveryTrajectory {
    pub resource_id: String,
    pub energy_after_slot_wh: Vec<i64>,
    pub reserve_floor_by_slot_wh: Vec<u64>,
    pub reserve_margin_by_slot_wh: Vec<i64>,
    pub minimum_reserve_margin_wh: i64,
    pub recovery_target_energy_wh: u64,
    pub recovery_deadline_slot: u8,
    pub recovery_complete_by_deadline: bool,
    pub first_failed_slot: Option<u8>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct RecoveryScenarioEvaluation {
    pub scenario_id: String,
    pub resource_trajectories: Vec<ResourceRecoveryTrajectory>,
    pub site_import_power_by_slot_w: Vec<i64>,
    pub scenario_feasible: bool,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MobilityRecoveryReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub decision_time_unix_ms: u64,
    pub horizon_slots: u8,
    pub resource_count: usize,
    pub scenario_count: usize,
    pub flexibility_report_reference: String,
    pub candidate_model_feasible: bool,
    pub all_evidence_source_grade: bool,
    pub all_evidence_synthetic: bool,
    pub dispatch_authority_granted: bool,
    pub export_authority_granted: bool,
    pub findings: Vec<String>,
    pub scenario_results: Vec<RecoveryScenarioEvaluation>,
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
    horizon_end_exclusive_unix_ms: u64,
) -> bool {
    if evidence.asset_id != asset_id
        || evidence.configuration_digest != configuration_digest
        || evidence.asset_id.trim().is_empty()
        || evidence.configuration_digest.trim().is_empty()
        || evidence.producer_id.trim().is_empty()
        || evidence.method_id.trim().is_empty()
        || evidence.evidence_reference.trim().is_empty()
        || evidence.evidence_class == EvidenceClass::Unknown
    {
        return false;
    }

    match (
        evidence.observed_at_unix_ms,
        evidence.valid_from_unix_ms,
        evidence.valid_until_unix_ms,
    ) {
        (Some(observed), Some(valid_from), Some(valid_until)) => {
            valid_from <= observed
                && observed <= valid_until
                && observed <= decision_time_unix_ms
                && valid_from <= decision_time_unix_ms
                && horizon_end_exclusive_unix_ms.saturating_sub(1) <= valid_until
        }
        _ => false,
    }
}

fn push_evidence_class(
    evidence: &EvidenceEnvelope,
    all_source_grade: &mut bool,
    all_synthetic: &mut bool,
) {
    *all_source_grade &= matches!(
        evidence.evidence_class,
        EvidenceClass::Measured | EvidenceClass::IndependentTest
    );
    *all_synthetic &= evidence.evidence_class == EvidenceClass::Synthetic;
}

fn charge_energy_lower_bound_wh(
    power_w: u32,
    duration_seconds: u32,
    efficiency_permille: u16,
) -> Option<u64> {
    let numerator = u128::from(power_w)
        .checked_mul(u128::from(duration_seconds))?
        .checked_mul(u128::from(efficiency_permille))?;
    u64::try_from(numerator / 3_600_000).ok()
}

fn discharge_energy_upper_bound_wh(
    power_w: u32,
    duration_seconds: u32,
    efficiency_permille: u16,
) -> Option<u64> {
    if efficiency_permille == 0 {
        return None;
    }
    let numerator = u128::from(power_w)
        .checked_mul(u128::from(duration_seconds))?
        .checked_mul(1_000)?;
    let denominator = 3_600_u128.checked_mul(u128::from(efficiency_permille))?;
    let rounded_up = numerator.checked_add(denominator - 1)? / denominator;
    u64::try_from(rounded_up).ok()
}

fn reserve_floor_wh(obligation: &RecoveryResourceObligation, slot: usize) -> Option<u64> {
    obligation.mission_reserve_floor_wh_by_slot[slot]
        .checked_add(obligation.inventory_reserve_floor_wh_by_slot[slot])?
        .checked_add(obligation.safety_outage_reserve_floor_wh_by_slot[slot])?
        .checked_add(obligation.uncertainty_reserve_floor_wh_by_slot[slot])
}

fn signed_margin(limit: u64, requested: u64) -> i64 {
    if limit >= requested {
        i64::try_from(limit - requested).unwrap_or(i64::MAX)
    } else {
        -i64::try_from(requested - limit).unwrap_or(i64::MAX)
    }
}

fn slot_end_unix_ms(
    decision_time_unix_ms: u64,
    duration_seconds: u32,
    slot: u8,
) -> Option<u64> {
    u64::from(slot)
        .checked_add(1)?
        .checked_mul(u64::from(duration_seconds))?
        .checked_mul(1_000)?
        .checked_add(decision_time_unix_ms)
}

pub fn evaluate_mobility_recovery(
    input: &MobilityRecoveryInput,
) -> Result<MobilityRecoveryReport, Vec<String>> {
    let mut errors = Vec::new();

    if input.schema_version != "0.2.0"
        || input.benchmark_id != "MOBILITY-RESERVE-RECOVERY-001"
        || input.site_id.trim().is_empty()
        || input.site_configuration_digest.trim().is_empty()
        || input.flexibility_report_reference.trim().is_empty()
        || input.claim_ceiling.trim().is_empty()
        || input.decision_time_unix_ms == 0
        || input.slot_duration_seconds == 0
        || input.slot_duration_seconds > MAX_SLOT_DURATION_SECONDS
        || input.horizon_slots == 0
        || input.horizon_slots > MAX_HORIZON_SLOTS
    {
        add_error(&mut errors, "unsupported_identity_or_time_horizon");
    }

    if input.resources.is_empty()
        || input.resources.len() > MAX_RESOURCES
        || input.scenarios.is_empty()
        || input.scenarios.len() > MAX_SCENARIOS
        || input.charger_channels.is_empty()
        || input.charger_channels.len() > MAX_CHANNELS
        || input.pcc_slots.len() != usize::from(input.horizon_slots)
        || input.recovery_schedule.len()
            != input.resources.len() * usize::from(input.horizon_slots)
    {
        add_error(&mut errors, "resource_scenario_channel_or_horizon_bound_invalid");
    }

    let horizon_end = slot_end_unix_ms(
        input.decision_time_unix_ms,
        input.slot_duration_seconds,
        input.horizon_slots - 1,
    );
    let Some(horizon_end) = horizon_end else {
        add_error(&mut errors, "horizon_time_overflow");
        return Err(errors);
    };

    let site_evidence_refs = [
        ("site", &input.site_evidence),
        ("allocation", &input.allocation_evidence),
        ("charging_queue", &input.charging_queue_evidence),
        ("site_meter", &input.site_meter_evidence),
    ];
    let mut all_source_grade = true;
    let mut all_synthetic = true;
    for (label, evidence) in site_evidence_refs {
        if !evidence_covers(
            evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            horizon_end,
        ) {
            add_error(&mut errors, format!("{label}_evidence_not_admissible"));
        }
        push_evidence_class(evidence, &mut all_source_grade, &mut all_synthetic);
    }

    let mut resource_map = BTreeMap::new();
    for resource in &input.resources {
        if resource.resource_id.trim().is_empty()
            || resource.configuration_digest.trim().is_empty()
            || resource.initial_energy_lower_bound_wh > MAX_ENERGY_WH
            || resource.energy_capacity_lower_bound_wh == 0
            || resource.energy_capacity_lower_bound_wh > MAX_ENERGY_WH
            || resource.initial_energy_lower_bound_wh > resource.energy_capacity_lower_bound_wh
            || resource.max_charge_input_power_lower_bound_w == 0
            || resource.max_discharge_output_power_lower_bound_w == 0
            || resource.max_charge_input_power_lower_bound_w > MAX_POWER_W
            || resource.max_discharge_output_power_lower_bound_w > MAX_POWER_W
            || !(1..=1_000).contains(&resource.charge_efficiency_lower_bound_permille)
            || !(1..=1_000).contains(&resource.discharge_efficiency_lower_bound_permille)
            || resource_map
                .insert(resource.resource_id.as_str(), resource)
                .is_some()
        {
            add_error(
                &mut errors,
                format!("resource_identity_or_capability_invalid:{}", resource.resource_id),
            );
            continue;
        }

        for (label, evidence) in [
            ("initial_energy", &resource.initial_energy_evidence),
            ("capacity", &resource.capacity_evidence),
            ("charge_power", &resource.charge_power_evidence),
            ("discharge_power", &resource.discharge_power_evidence),
            ("efficiency", &resource.efficiency_evidence),
        ] {
            if !evidence_covers(
                evidence,
                &resource.resource_id,
                &resource.configuration_digest,
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
    }

    let mut channel_map = BTreeMap::new();
    for channel in &input.charger_channels {
        if channel.channel_id.trim().is_empty()
            || channel.configuration_digest.trim().is_empty()
            || channel.max_input_power_lower_bound_w == 0
            || channel.max_input_power_lower_bound_w > MAX_POWER_W
            || channel.compatible_resource_ids.is_empty()
            || channel.compatible_resource_ids.iter().any(|id| {
                id.trim().is_empty() || !resource_map.contains_key(id.as_str())
            })
            || channel_map
                .insert(channel.channel_id.as_str(), channel)
                .is_some()
        {
            add_error(
                &mut errors,
                format!("charger_channel_identity_or_compatibility_invalid:{}", channel.channel_id),
            );
            continue;
        }
        if !evidence_covers(
            &channel.evidence,
            &channel.channel_id,
            &channel.configuration_digest,
            input.decision_time_unix_ms,
            horizon_end,
        ) {
            add_error(
                &mut errors,
                format!("charger_channel_evidence_not_admissible:{}", channel.channel_id),
            );
        }
        push_evidence_class(&channel.evidence, &mut all_source_grade, &mut all_synthetic);
    }

    let mut sorted_schedule = input.recovery_schedule.clone();
    sorted_schedule.sort_by(|a, b| (&a.resource_id, a.slot).cmp(&(&b.resource_id, b.slot)));
    let mut schedule_map = BTreeMap::new();
    let mut channel_occupancy = BTreeMap::new();
    for item in &sorted_schedule {
        let resource_power_valid = resource_map
            .get(item.resource_id.as_str())
            .is_some_and(|resource| {
                item.recovery_charge_input_power_w
                    <= resource.max_charge_input_power_lower_bound_w
                    && item.grid_discharge_output_power_w
                        <= resource.max_discharge_output_power_lower_bound_w
            });
        if !resource_map.contains_key(item.resource_id.as_str())
            || item.slot >= input.horizon_slots
            || item.recovery_charge_input_power_w > MAX_POWER_W
            || item.grid_discharge_output_power_w > MAX_POWER_W
            || !resource_power_valid
            || item.queue_evidence_reference != input.charging_queue_evidence.evidence_reference
            || (item.recovery_charge_input_power_w > 0 && item.grid_discharge_output_power_w > 0)
        {
            add_error(
                &mut errors,
                format!("recovery_schedule_identity_power_or_queue_invalid:{}:{}", item.resource_id, item.slot),
            );
            continue;
        }

        if schedule_map
            .insert((item.resource_id.as_str(), item.slot), item)
            .is_some()
        {
            add_error(
                &mut errors,
                format!("recovery_schedule_duplicate:{}:{}", item.resource_id, item.slot),
            );
            continue;
        }

        match (item.recovery_charge_input_power_w, item.charger_channel_id.as_deref()) {
            (0, None) => {}
            (0, Some(_)) => add_error(
                &mut errors,
                format!("idle_schedule_has_charger_channel:{}:{}", item.resource_id, item.slot),
            ),
            (_, None) => add_error(
                &mut errors,
                format!("recovery_charge_missing_charger_channel:{}:{}", item.resource_id, item.slot),
            ),
            (charge_power, Some(channel_id)) => {
                let Some(channel) = channel_map.get(channel_id) else {
                    add_error(
                        &mut errors,
                        format!("recovery_schedule_unknown_channel:{channel_id}"),
                    );
                    continue;
                };
                let resource = resource_map[item.resource_id.as_str()];
                if charge_power > resource.max_charge_input_power_lower_bound_w
                    || charge_power > channel.max_input_power_lower_bound_w
                    || !channel
                        .compatible_resource_ids
                        .iter()
                        .any(|id| id == &item.resource_id)
                {
                    add_error(
                        &mut errors,
                        format!("recovery_schedule_channel_incompatible:{}:{channel_id}", item.resource_id),
                    );
                }
                if channel_occupancy
                    .insert((channel_id, item.slot), item.resource_id.as_str())
                    .is_some()
                {
                    add_error(
                        &mut errors,
                        format!("recovery_charger_channel_double_booked:{channel_id}:{}", item.slot),
                    );
                }
            }
        }
    }

    for resource in &input.resources {
        for slot in 0..input.horizon_slots {
            if !schedule_map.contains_key(&(resource.resource_id.as_str(), slot)) {
                add_error(
                    &mut errors,
                    format!("recovery_schedule_slot_missing:{}:{slot}", resource.resource_id),
                );
            }
        }
    }

    let mut sorted_pcc = input.pcc_slots.clone();
    sorted_pcc.sort_by_key(|slot| slot.slot);
    let mut pcc_by_slot = BTreeMap::new();
    for (index, pcc) in sorted_pcc.iter().enumerate() {
        let slot_end = slot_end_unix_ms(
            input.decision_time_unix_ms,
            input.slot_duration_seconds,
            pcc.slot,
        );
        let slot_start = if pcc.slot == 0 {
            Some(input.decision_time_unix_ms)
        } else {
            slot_end_unix_ms(
                input.decision_time_unix_ms,
                input.slot_duration_seconds,
                pcc.slot - 1,
            )
        };
        let slot_evidence_valid = matches!(
            (slot_start, slot_end),
            (Some(start), Some(end))
                if [ &pcc.base_load_evidence, &pcc.scheduled_charging_load_evidence, &pcc.evidence]
                    .iter()
                    .all(|evidence| evidence_covers(
                        evidence,
                        &input.site_id,
                        &input.site_configuration_digest,
                        input.decision_time_unix_ms,
                        end,
                    ))
                    && pcc.base_load_evidence.evidence_reference == input.site_meter_evidence.evidence_reference
                    && pcc.scheduled_charging_load_evidence.evidence_reference == input.charging_queue_evidence.evidence_reference
                    && start < end
        );
        if pcc.slot != index as u8
            || pcc.base_site_load_w > MAX_POWER_W
            || pcc.scheduled_mobility_charging_load_w > MAX_POWER_W
            || pcc.import_limit_w > MAX_POWER_W
            || !slot_evidence_valid
            || pcc_by_slot.insert(pcc.slot, pcc).is_some()
        {
            add_error(&mut errors, format!("recovery_pcc_slot_or_evidence_invalid:{}", pcc.slot));
        }
        for evidence in [
            &pcc.base_load_evidence,
            &pcc.scheduled_charging_load_evidence,
            &pcc.evidence,
        ] {
            push_evidence_class(evidence, &mut all_source_grade, &mut all_synthetic);
        }
    }

    let mut sorted_scenarios = input.scenarios.clone();
    sorted_scenarios.sort_by(|a, b| a.scenario_id.cmp(&b.scenario_id));
    let mut scenario_ids = BTreeSet::new();
    let mut scenario_results = Vec::new();

    for scenario in &sorted_scenarios {
        if scenario.scenario_id.trim().is_empty()
            || !scenario_ids.insert(scenario.scenario_id.as_str())
        {
            add_error(&mut errors, "recovery_scenario_identity_duplicate_or_empty");
            continue;
        }
        if !evidence_covers(
            &scenario.evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            horizon_end,
        ) {
            add_error(
                &mut errors,
                format!("recovery_scenario_evidence_not_admissible:{}", scenario.scenario_id),
            );
        }
        push_evidence_class(&scenario.evidence, &mut all_source_grade, &mut all_synthetic);

        if scenario.resource_obligations.len() != input.resources.len()
            || scenario.channel_availability.len() != input.charger_channels.len()
        {
            add_error(
                &mut errors,
                format!("recovery_scenario_coverage_incomplete:{}", scenario.scenario_id),
            );
        }

        let mut obligations_by_resource = BTreeMap::new();
        for obligation in &scenario.resource_obligations {
            if !resource_map.contains_key(obligation.resource_id.as_str())
                || obligations_by_resource
                    .insert(obligation.resource_id.as_str(), obligation)
                    .is_some()
            {
                add_error(
                    &mut errors,
                    format!("recovery_obligation_resource_duplicate_or_unknown:{}", scenario.scenario_id),
                );
                continue;
            }
            let correct_len = usize::from(input.horizon_slots);
            let resource = resource_map[obligation.resource_id.as_str()];
            let values_within_bound = obligation
                .mission_reserve_floor_wh_by_slot
                .iter()
                .chain(&obligation.inventory_reserve_floor_wh_by_slot)
                .chain(&obligation.safety_outage_reserve_floor_wh_by_slot)
                .chain(&obligation.uncertainty_reserve_floor_wh_by_slot)
                .chain(&obligation.mission_energy_withdrawal_wh_by_slot)
                .chain(&obligation.disturbance_energy_withdrawal_wh_by_slot)
                .all(|value| *value <= MAX_ENERGY_WH);
            if obligation.mission_reserve_floor_wh_by_slot.len() != correct_len
                || obligation.inventory_reserve_floor_wh_by_slot.len() != correct_len
                || obligation.safety_outage_reserve_floor_wh_by_slot.len() != correct_len
                || obligation.uncertainty_reserve_floor_wh_by_slot.len() != correct_len
                || obligation.mission_energy_withdrawal_wh_by_slot.len() != correct_len
                || obligation.disturbance_energy_withdrawal_wh_by_slot.len() != correct_len
                || !values_within_bound
                || obligation.recovery_target_energy_wh > resource.energy_capacity_lower_bound_wh
                || obligation.recovery_deadline_slot >= input.horizon_slots
            {
                add_error(
                    &mut errors,
                    format!("recovery_obligation_horizon_or_deadline_invalid:{}", obligation.resource_id),
                );
                obligations_by_resource.remove(obligation.resource_id.as_str());
                continue;
            }
            if !evidence_covers(
                &obligation.evidence,
                &resource.resource_id,
                &resource.configuration_digest,
                input.decision_time_unix_ms,
                horizon_end,
            ) {
                add_error(
                    &mut errors,
                    format!("recovery_obligation_evidence_not_admissible:{}", obligation.resource_id),
                );
            }
            push_evidence_class(&obligation.evidence, &mut all_source_grade, &mut all_synthetic);
        }

        let mut channel_availability_by_id = BTreeMap::new();
        for availability in &scenario.channel_availability {
            let channel_evidence_valid = channel_map
                .get(availability.channel_id.as_str())
                .is_some_and(|channel| {
                    evidence_covers(
                        &availability.evidence,
                        &channel.channel_id,
                        &channel.configuration_digest,
                        input.decision_time_unix_ms,
                        horizon_end,
                    )
                });
            if !channel_map.contains_key(availability.channel_id.as_str())
                || availability.available_by_slot.len() != usize::from(input.horizon_slots)
                || !channel_evidence_valid
                || channel_availability_by_id
                    .insert(availability.channel_id.as_str(), availability)
                    .is_some()
            {
                add_error(
                    &mut errors,
                    format!("recovery_channel_availability_invalid:{}", scenario.scenario_id),
                );
            }
            push_evidence_class(
                &availability.evidence,
                &mut all_source_grade,
                &mut all_synthetic,
            );
        }

        let mut scenario_findings = BTreeSet::new();
        let mut resource_trajectories = Vec::new();
        let mut site_import_by_slot = vec![0_i64; usize::from(input.horizon_slots)];

        for slot in 0..input.horizon_slots {
            let pcc = pcc_by_slot.get(&slot);
            let Some(pcc) = pcc else {
                scenario_findings.insert(format!("pcc_slot_missing:{slot}"));
                continue;
            };
            let mut recovery_charge_power = 0_u64;
            let mut grid_discharge_power = 0_u64;
            for resource in &input.resources {
                let Some(item) = schedule_map.get(&(resource.resource_id.as_str(), slot)) else {
                    continue;
                };
                let charge_active = if item.recovery_charge_input_power_w == 0 {
                    false
                } else if let Some(channel_id) = item.charger_channel_id.as_deref() {
                    channel_availability_by_id
                        .get(channel_id)
                        .and_then(|availability| availability.available_by_slot.get(usize::from(slot)))
                        .copied()
                        .unwrap_or(false)
                } else {
                    false
                };
                if item.recovery_charge_input_power_w > 0 && !charge_active {
                    scenario_findings.insert(format!(
                        "charger_channel_unavailable:{}:{slot}",
                        item.charger_channel_id.as_deref().unwrap_or("unknown")
                    ));
                }
                if charge_active {
                    recovery_charge_power = recovery_charge_power
                        .saturating_add(u64::from(item.recovery_charge_input_power_w));
                }
                grid_discharge_power = grid_discharge_power
                    .saturating_add(u64::from(item.grid_discharge_output_power_w));
            }
            let site_import = i64::from(pcc.base_site_load_w)
                + i64::from(pcc.scheduled_mobility_charging_load_w)
                + i64::try_from(recovery_charge_power).unwrap_or(i64::MAX)
                - i64::try_from(grid_discharge_power).unwrap_or(i64::MAX);
            site_import_by_slot[usize::from(slot)] = site_import;
            if site_import < 0 {
                scenario_findings.insert(format!("pcc_export_not_in_v0_2a_scope:{slot}"));
            } else if site_import > i64::from(pcc.import_limit_w) {
                scenario_findings.insert(format!("pcc_import_limit_exceeded:{slot}"));
            }
        }

        for resource in &input.resources {
            let Some(obligation) = obligations_by_resource.get(resource.resource_id.as_str()) else {
                scenario_findings.insert(format!("resource_obligation_missing:{}", resource.resource_id));
                continue;
            };
            let mut energy = i64::try_from(resource.initial_energy_lower_bound_wh)
                .unwrap_or(i64::MAX);
            let mut energy_after_slot = Vec::new();
            let mut reserve_floor_by_slot = Vec::new();
            let mut reserve_margin_by_slot = Vec::new();
            let mut minimum_reserve_margin = i64::MAX;
            let mut resource_findings = BTreeSet::new();
            let mut first_failed_slot = None;
            let mut recovery_complete = false;

            for slot in 0..input.horizon_slots {
                let index = usize::from(slot);
                let item = schedule_map.get(&(resource.resource_id.as_str(), slot));
                let charge_power = item.map_or(0, |item| item.recovery_charge_input_power_w);
                let discharge_power = item.map_or(0, |item| item.grid_discharge_output_power_w);
                let charge_active = if charge_power == 0 {
                    false
                } else {
                    item.and_then(|item| item.charger_channel_id.as_deref())
                        .and_then(|channel_id| {
                            channel_availability_by_id
                                .get(channel_id)
                                .and_then(|availability| availability.available_by_slot.get(index))
                        })
                        .copied()
                        .unwrap_or(false)
                };

                let charge_wh = if charge_active {
                    charge_energy_lower_bound_wh(
                        charge_power,
                        input.slot_duration_seconds,
                        resource.charge_efficiency_lower_bound_permille,
                    )
                } else {
                    Some(0)
                };
                let discharge_wh = discharge_energy_upper_bound_wh(
                    discharge_power,
                    input.slot_duration_seconds,
                    resource.discharge_efficiency_lower_bound_permille,
                );
                let (Some(charge_wh), Some(discharge_wh)) = (charge_wh, discharge_wh) else {
                    add_error(&mut errors, "recovery_energy_arithmetic_overflow");
                    break;
                };
                let withdrawal_wh = obligation.mission_energy_withdrawal_wh_by_slot[index]
                    .checked_add(obligation.disturbance_energy_withdrawal_wh_by_slot[index]);
                let Some(withdrawal_wh) = withdrawal_wh else {
                    add_error(&mut errors, "recovery_withdrawal_sum_overflow");
                    break;
                };
                let next_energy = i128::from(energy)
                    + i128::from(charge_wh)
                    - i128::from(discharge_wh)
                    - i128::from(withdrawal_wh);
                if next_energy > i128::from(MAX_ENERGY_WH) || next_energy < -i128::from(MAX_ENERGY_WH) {
                    add_error(&mut errors, format!("recovery_state_out_of_bounds:{}:{slot}", resource.resource_id));
                    break;
                }
                energy = next_energy as i64;
                if energy > i64::try_from(resource.energy_capacity_lower_bound_wh).unwrap_or(i64::MAX) {
                    first_failed_slot.get_or_insert(slot);
                    resource_findings.insert(format!("energy_capacity_exceeded:{slot}"));
                }
                energy_after_slot.push(energy);

                let Some(floor_wh) = reserve_floor_wh(obligation, index) else {
                    add_error(&mut errors, "recovery_reserve_floor_overflow");
                    break;
                };
                reserve_floor_by_slot.push(floor_wh);
                let margin = energy - i64::try_from(floor_wh).unwrap_or(i64::MAX);
                reserve_margin_by_slot.push(margin);
                minimum_reserve_margin = minimum_reserve_margin.min(margin);
                if margin < 0 {
                    first_failed_slot.get_or_insert(slot);
                    resource_findings.insert(format!("mission_or_reserve_floor_violated:{slot}"));
                }
                if slot == obligation.recovery_deadline_slot {
                    recovery_complete = energy >= i64::try_from(obligation.recovery_target_energy_wh).unwrap_or(i64::MAX);
                    if !recovery_complete {
                        first_failed_slot.get_or_insert(slot);
                        resource_findings.insert(format!("reserve_not_restored_by_deadline:{slot}"));
                    }
                }
            }

            if energy_after_slot.len() != usize::from(input.horizon_slots) {
                scenario_findings.insert(format!("resource_trajectory_incomplete:{}", resource.resource_id));
            }
            if !recovery_complete {
                resource_findings.insert("recovery_target_not_confirmed".to_owned());
            }
            if minimum_reserve_margin == i64::MAX {
                minimum_reserve_margin = i64::MIN;
            }
            if !resource_findings.is_empty() {
                scenario_findings.extend(
                    resource_findings
                        .iter()
                        .map(|finding| format!("{}:{finding}", resource.resource_id)),
                );
            }

            resource_trajectories.push(ResourceRecoveryTrajectory {
                resource_id: resource.resource_id.clone(),
                energy_after_slot_wh: energy_after_slot,
                reserve_floor_by_slot_wh: reserve_floor_by_slot,
                reserve_margin_by_slot_wh: reserve_margin_by_slot,
                minimum_reserve_margin_wh: minimum_reserve_margin,
                recovery_target_energy_wh: obligation.recovery_target_energy_wh,
                recovery_deadline_slot: obligation.recovery_deadline_slot,
                recovery_complete_by_deadline: recovery_complete,
                first_failed_slot,
                findings: resource_findings.into_iter().collect(),
            });
        }

        resource_trajectories.sort_by(|a, b| a.resource_id.cmp(&b.resource_id));
        let scenario_feasible = scenario_findings.is_empty()
            && resource_trajectories.iter().all(|trajectory| {
                trajectory.first_failed_slot.is_none()
                    && trajectory.recovery_complete_by_deadline
                    && trajectory.energy_after_slot_wh.len() == usize::from(input.horizon_slots)
            });
        scenario_results.push(RecoveryScenarioEvaluation {
            scenario_id: scenario.scenario_id.clone(),
            resource_trajectories,
            site_import_power_by_slot_w: site_import_by_slot,
            scenario_feasible,
            findings: scenario_findings.into_iter().collect(),
        });
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let candidate_model_feasible = !scenario_results.is_empty()
        && scenario_results.iter().all(|result| result.scenario_feasible);
    let mut findings = BTreeSet::new();
    for result in &scenario_results {
        for finding in &result.findings {
            findings.insert(finding.clone());
        }
    }
    if !all_source_grade {
        findings.insert("evidence_not_entirely_measured_or_independent_test".to_owned());
    }

    Ok(MobilityRecoveryReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        site_id: input.site_id.clone(),
        decision_time_unix_ms: input.decision_time_unix_ms,
        horizon_slots: input.horizon_slots,
        resource_count: input.resources.len(),
        scenario_count: input.scenarios.len(),
        flexibility_report_reference: input.flexibility_report_reference.clone(),
        candidate_model_feasible,
        all_evidence_source_grade: all_source_grade,
        all_evidence_synthetic: all_synthetic,
        dispatch_authority_granted: false,
        export_authority_granted: false,
        findings: findings.into_iter().collect(),
        scenario_results,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MobilityRecoveryInput {
        serde_json::from_str(include_str!("../fixtures/mobility_recovery_envelope_v0_2.json"))
            .expect("frozen recovery fixture must parse")
    }

    #[test]
    fn charge_energy_rounds_down_and_discharge_energy_rounds_up() {
        assert_eq!(charge_energy_lower_bound_wh(1_000, 900, 900), Some(225));
        assert_eq!(charge_energy_lower_bound_wh(500, 900, 900), Some(112));
        assert_eq!(discharge_energy_upper_bound_wh(1_000, 900, 900), Some(278));
    }

    #[test]
    fn repeated_shock_fixture_recovers_both_assets_before_deadline() {
        let report = evaluate_mobility_recovery(&fixture()).expect("fixture is structurally valid");
        assert!(report.candidate_model_feasible);
        assert_eq!(report.scenario_results.len(), 2);
        assert!(report.all_evidence_synthetic);
        assert!(!report.dispatch_authority_granted);
        assert!(!report.export_authority_granted);
        for scenario in &report.scenario_results {
            assert!(scenario.scenario_feasible);
            for resource in &scenario.resource_trajectories {
                assert!(resource.recovery_complete_by_deadline);
                assert!(resource.minimum_reserve_margin_wh >= 0);
                assert_eq!(resource.energy_after_slot_wh.len(), 8);
            }
        }
    }

    #[test]
    fn missing_recovery_charge_fails_the_deadline() {
        let mut input = fixture();
        for item in &mut input.recovery_schedule {
            if item.slot == 5 {
                item.recovery_charge_input_power_w = 0;
                item.charger_channel_id = None;
            }
        }
        let report = evaluate_mobility_recovery(&input).expect("valid plan shape");
        assert!(!report.candidate_model_feasible);
        assert!(report.scenario_results.iter().any(|scenario| {
            scenario.findings.iter().any(|finding| finding.contains("reserve_not_restored_by_deadline"))
        }));
    }

    #[test]
    fn second_shock_can_break_a_previously_recovering_asset() {
        let mut input = fixture();
        for scenario in &mut input.scenarios {
            if scenario.scenario_id == "second-shock-before-recovery" {
                let obligation = scenario
                    .resource_obligations
                    .iter_mut()
                    .find(|obligation| obligation.resource_id == "ess-01")
                    .expect("ess-01 obligation");
                obligation.disturbance_energy_withdrawal_wh_by_slot[4] = 600;
            }
        }
        let report = evaluate_mobility_recovery(&input).expect("valid scenario shape");
        assert!(!report.candidate_model_feasible);
        assert!(report.scenario_results.iter().any(|scenario| {
            scenario.findings.iter().any(|finding| finding.contains("mission_or_reserve_floor_violated"))
        }));
    }

    #[test]
    fn charger_outage_does_not_create_recovery_energy() {
        let mut input = fixture();
        for scenario in &mut input.scenarios {
            if scenario.scenario_id == "second-shock-before-recovery" {
                let channel = scenario
                    .channel_availability
                    .iter_mut()
                    .find(|channel| channel.channel_id == "channel-a")
                    .expect("channel-a availability");
                channel.available_by_slot[3] = false;
            }
        }
        let report = evaluate_mobility_recovery(&input).expect("valid scenario shape");
        assert!(!report.candidate_model_feasible);
        assert!(report.scenario_results.iter().any(|scenario| {
            scenario.findings.iter().any(|finding| finding.contains("charger_channel_unavailable"))
        }));
    }

    #[test]
    fn duplicate_charger_channel_booking_is_rejected() {
        let mut input = fixture();
        input
            .recovery_schedule
            .iter_mut()
            .find(|item| item.resource_id == "ess-02" && item.slot == 2)
            .expect("ess-02 slot 2")
            .charger_channel_id = Some("channel-a".to_owned());
        let result = evaluate_mobility_recovery(&input);
        assert!(result.is_err());
    }

    #[test]
    fn simultaneous_charge_and_discharge_is_rejected() {
        let mut input = fixture();
        let item = input
            .recovery_schedule
            .iter_mut()
            .find(|item| item.resource_id == "ess-01" && item.slot == 2)
            .expect("ess-01 slot 2");
        item.grid_discharge_output_power_w = 100;
        let result = evaluate_mobility_recovery(&input);
        assert!(result.is_err());
    }

    #[test]
    fn insufficient_pcc_headroom_fails_closed() {
        let mut input = fixture();
        input.pcc_slots[2].import_limit_w = 2_000;
        let report = evaluate_mobility_recovery(&input).expect("valid PCC data");
        assert!(!report.candidate_model_feasible);
        assert!(report.scenario_results.iter().all(|scenario| {
            scenario.findings.iter().any(|finding| finding.contains("pcc_import_limit_exceeded:2"))
        }));
    }

    #[test]
    fn input_order_permutation_preserves_canonical_report() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.resources.reverse();
        permuted.charger_channels.reverse();
        permuted.scenarios.reverse();
        permuted.pcc_slots.reverse();
        permuted.recovery_schedule.reverse();
        for scenario in &mut permuted.scenarios {
            scenario.resource_obligations.reverse();
            scenario.channel_availability.reverse();
        }
        let a = evaluate_mobility_recovery(&input).expect("first input");
        let b = evaluate_mobility_recovery(&permuted).expect("permuted input");
        assert_eq!(
            serde_json::to_string(&a).expect("serialize first"),
            serde_json::to_string(&b).expect("serialize second")
        );
    }
}
