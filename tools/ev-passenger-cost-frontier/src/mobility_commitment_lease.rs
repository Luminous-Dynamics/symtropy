// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Proposal-only admission check for cross-domain resource capability leases.
//!
//! The budget supplied to this module is already net of mission and recovery
//! reserves. This module prevents firm external promises from reusing that
//! remaining capability. An unresolved lease remains reserved until evidence
//! establishes a terminal release. No ledger mutation or external authority is
//! granted here.

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_RESOURCES: usize = 4;
const MAX_EXISTING_COMMITMENTS: usize = 32;
const MAX_PROTECTED_WINDOWS: usize = 64;
const MAX_HORIZON_SLOTS: u8 = 16;
const MAX_POWER_W: u32 = 100_000_000;
const MAX_ENERGY_WH: u64 = 1_000_000_000_000;
const CLAIM_CEILING: &str = "Deterministic model-level commitment admission only; no market, dispatch, physical mutation, flight, custody or settlement authority.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommitmentState {
    FirmOffered,
    Accepted,
    Executing,
    Unresolved,
    Rejected,
    Expired,
    Cancelled,
    Settled,
}

impl CommitmentState {
    fn reserves_capability(self) -> bool {
        matches!(
            self,
            Self::FirmOffered | Self::Accepted | Self::Executing | Self::Unresolved
        )
    }

    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Expired | Self::Cancelled | Self::Settled
        )
    }
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProtectedClaimKind {
    Mission,
    Recovery,
    Maintenance,
    SafetyHold,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseResourceBudget {
    pub resource_id: String,
    pub configuration_digest: String,
    pub safe_grid_energy_budget_wh: u64,
    pub safe_grid_power_budget_w_by_slot: Vec<u32>,
    pub budget_evidence: EvidenceEnvelope,
    pub site_connection_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ProtectedResourceWindow {
    pub window_id: String,
    pub resource_id: String,
    pub claim_kind: ProtectedClaimKind,
    pub start_slot: u8,
    pub end_slot: u8,
    pub claim_reference: String,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LeaseResourceAllocation {
    pub resource_id: String,
    pub configuration_digest: String,
    /// Full-horizon arrays. Values outside [start_slot, end_slot) must be zero.
    pub power_w_by_slot: Vec<u32>,
    /// Full-horizon energy allocations. These are not inferred from power.
    pub energy_wh_by_slot: Vec<u64>,
    pub allocation_evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExistingCommitment {
    pub commitment_id: String,
    pub idempotency_key: String,
    pub state: CommitmentState,
    pub start_slot: u8,
    pub end_slot: u8,
    pub expires_at_unix_ms: u64,
    pub state_evidence: EvidenceEnvelope,
    pub resource_allocations: Vec<LeaseResourceAllocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CandidateLease {
    pub commitment_id: String,
    pub idempotency_key: String,
    pub start_slot: u8,
    pub end_slot: u8,
    pub expires_at_unix_ms: u64,
    pub resource_allocations: Vec<LeaseResourceAllocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MobilityCommitmentLeaseInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub site_configuration_digest: String,
    pub decision_time_unix_ms: u64,
    pub slot_duration_seconds: u32,
    pub horizon_slots: u8,
    pub flexibility_report_digest: String,
    pub recovery_report_digest: String,
    pub report_valid_until_unix_ms: u64,
    pub flexibility_report_evidence: EvidenceEnvelope,
    pub recovery_report_evidence: EvidenceEnvelope,
    pub resource_budgets: Vec<LeaseResourceBudget>,
    pub protected_windows: Vec<ProtectedResourceWindow>,
    pub existing_commitments: Vec<ExistingCommitment>,
    pub candidate_lease: CandidateLease,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ResourceLeaseMargin {
    pub resource_id: String,
    pub safe_grid_energy_budget_wh: u64,
    pub committed_energy_wh: u64,
    pub remaining_energy_wh: i64,
    pub committed_power_w_by_slot: Vec<u32>,
    pub remaining_power_w_by_slot: Vec<i64>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct MobilityCommitmentLeaseReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub site_id: String,
    pub decision_time_unix_ms: u64,
    pub horizon_slots: u8,
    pub candidate_commitment_id: String,
    pub active_commitment_count: usize,
    pub terminal_commitment_count: usize,
    pub candidate_model_admissible: bool,
    pub resource_margins: Vec<ResourceLeaseMargin>,
    pub protected_window_conflicts: Vec<String>,
    pub findings: Vec<String>,
    pub ledger_mutation_performed: bool,
    pub market_authority_granted: bool,
    pub dispatch_authority_granted: bool,
    pub physical_mutation_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

fn add_error(errors: &mut Vec<String>, message: impl Into<String>) {
    errors.push(message.into());
}

fn is_sha256_digest(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn slot_start_ms(input: &MobilityCommitmentLeaseInput, slot: u8) -> Option<u64> {
    u64::from(slot)
        .checked_mul(u64::from(input.slot_duration_seconds))?
        .checked_mul(1_000)?
        .checked_add(input.decision_time_unix_ms)
}

fn slot_end_exclusive_ms(input: &MobilityCommitmentLeaseInput, slot: u8) -> Option<u64> {
    u64::from(slot)
        .checked_add(1)?
        .checked_mul(u64::from(input.slot_duration_seconds))?
        .checked_mul(1_000)?
        .checked_add(input.decision_time_unix_ms)
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
        (Some(observed), Some(valid_from), Some(valid_until)) => {
            valid_from <= observed
                && observed <= valid_until
                && observed <= decision_time_unix_ms
                && valid_from <= interval_start_unix_ms
                && interval_end_exclusive_unix_ms.saturating_sub(1) <= valid_until
        }
        _ => false,
    }
}

fn evidence_current_at(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    decision_time_unix_ms: u64,
) -> bool {
    evidence_covers(
        evidence,
        asset_id,
        configuration_digest,
        decision_time_unix_ms,
        decision_time_unix_ms,
        decision_time_unix_ms.saturating_add(1),
    )
}

fn allocation_has_usage(allocation: &LeaseResourceAllocation) -> bool {
    allocation
        .power_w_by_slot
        .iter()
        .any(|power| *power > 0)
        || allocation.energy_wh_by_slot.iter().any(|energy| *energy > 0)
}

fn validate_allocation(
    allocation: &LeaseResourceAllocation,
    commitment_id: &str,
    start_slot: u8,
    end_slot: u8,
    input: &MobilityCommitmentLeaseInput,
    budget_map: &BTreeMap<&str, &LeaseResourceBudget>,
    errors: &mut Vec<String>,
) {
    let Some(budget) = budget_map.get(allocation.resource_id.as_str()) else {
        add_error(
            errors,
            format!("allocation_resource_unknown:{commitment_id}:{}", allocation.resource_id),
        );
        return;
    };

    if start_slot >= end_slot
        || end_slot > input.horizon_slots
        || allocation.configuration_digest != budget.configuration_digest
        || allocation.power_w_by_slot.len() != usize::from(input.horizon_slots)
        || allocation.energy_wh_by_slot.len() != usize::from(input.horizon_slots)
        || !allocation_has_usage(allocation)
    {
        add_error(
            errors,
            format!("allocation_identity_or_horizon_invalid:{commitment_id}:{}", allocation.resource_id),
        );
        return;
    }

    let Some(interval_start) = slot_start_ms(input, start_slot) else {
        add_error(errors, format!("allocation_start_time_overflow:{commitment_id}"));
        return;
    };
    let Some(interval_end) = slot_end_exclusive_ms(input, end_slot - 1) else {
        add_error(errors, format!("allocation_end_time_overflow:{commitment_id}"));
        return;
    };
    if !evidence_covers(
        &allocation.allocation_evidence,
        &allocation.resource_id,
        &allocation.configuration_digest,
        input.decision_time_unix_ms,
        interval_start,
        interval_end,
    ) {
        add_error(
            errors,
            format!("allocation_evidence_not_admissible:{commitment_id}:{}", allocation.resource_id),
        );
    }

    for slot in 0..input.horizon_slots {
        let index = usize::from(slot);
        if allocation.power_w_by_slot[index] > MAX_POWER_W
            || allocation.energy_wh_by_slot[index] > MAX_ENERGY_WH
            || ((slot < start_slot || slot >= end_slot)
                && (allocation.power_w_by_slot[index] != 0
                    || allocation.energy_wh_by_slot[index] != 0))
        {
            add_error(
                errors,
                format!("allocation_slot_value_or_window_invalid:{commitment_id}:{}:{slot}", allocation.resource_id),
            );
        }
    }
}

pub fn evaluate_mobility_commitment_lease(
    input: &MobilityCommitmentLeaseInput,
) -> Result<MobilityCommitmentLeaseReport, Vec<String>> {
    let mut errors = Vec::new();

    if input.schema_version != "0.1.0"
        || input.benchmark_id != "MOBILITY-COMMITMENT-LEASE-001"
        || input.site_id.trim().is_empty()
        || input.site_configuration_digest.trim().is_empty()
        || input.decision_time_unix_ms == 0
        || input.slot_duration_seconds == 0
        || input.horizon_slots == 0
        || input.horizon_slots > MAX_HORIZON_SLOTS
        || input.slot_duration_seconds > 86_400
        || input.claim_ceiling.trim().is_empty()
        || !is_sha256_digest(&input.flexibility_report_digest)
        || !is_sha256_digest(&input.recovery_report_digest)
        || input.report_valid_until_unix_ms <= input.decision_time_unix_ms
    {
        add_error(&mut errors, "unsupported_identity_digest_or_horizon");
    }

    if input.resource_budgets.is_empty()
        || input.resource_budgets.len() > MAX_RESOURCES
        || input.existing_commitments.len() > MAX_EXISTING_COMMITMENTS
        || input.protected_windows.len() > MAX_PROTECTED_WINDOWS
    {
        add_error(&mut errors, "resource_or_commitment_bound_invalid");
    }

    if !evidence_covers(
        &input.flexibility_report_evidence,
        &input.site_id,
        &input.site_configuration_digest,
        input.decision_time_unix_ms,
        input.decision_time_unix_ms,
        input.report_valid_until_unix_ms,
    ) || input.flexibility_report_evidence.evidence_reference != input.flexibility_report_digest
    {
        add_error(&mut errors, "flexibility_report_evidence_not_bound_to_digest");
    }
    if !evidence_covers(
        &input.recovery_report_evidence,
        &input.site_id,
        &input.site_configuration_digest,
        input.decision_time_unix_ms,
        input.decision_time_unix_ms,
        input.report_valid_until_unix_ms,
    ) || input.recovery_report_evidence.evidence_reference != input.recovery_report_digest
    {
        add_error(&mut errors, "recovery_report_evidence_not_bound_to_digest");
    }

    let mut budget_map = BTreeMap::new();
    for budget in &input.resource_budgets {
        if budget.resource_id.trim().is_empty()
            || budget.configuration_digest.trim().is_empty()
            || budget.safe_grid_energy_budget_wh > MAX_ENERGY_WH
            || budget.safe_grid_power_budget_w_by_slot.len() != usize::from(input.horizon_slots)
            || budget
                .safe_grid_power_budget_w_by_slot
                .iter()
                .any(|power| *power > MAX_POWER_W)
            || budget_map.insert(budget.resource_id.as_str(), budget).is_some()
        {
            add_error(
                &mut errors,
                format!("resource_budget_identity_or_horizon_invalid:{}", budget.resource_id),
            );
            continue;
        }

        if !evidence_covers(
            &budget.budget_evidence,
            &budget.resource_id,
            &budget.configuration_digest,
            input.decision_time_unix_ms,
            input.decision_time_unix_ms,
            input.report_valid_until_unix_ms,
        ) {
            add_error(
                &mut errors,
                format!("resource_budget_evidence_invalid:{}", budget.resource_id),
            );
        }
        if !evidence_covers(
            &budget.site_connection_evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
            input.decision_time_unix_ms,
            input.report_valid_until_unix_ms,
        ) {
            add_error(
                &mut errors,
                format!("resource_site_connection_evidence_invalid:{}", budget.resource_id),
            );
        }
    }

    let candidate = &input.candidate_lease;
    if candidate.commitment_id.trim().is_empty()
        || candidate.idempotency_key.trim().is_empty()
        || candidate.start_slot >= candidate.end_slot
        || candidate.end_slot > input.horizon_slots
        || candidate.expires_at_unix_ms <= input.decision_time_unix_ms
        || candidate.expires_at_unix_ms > input.report_valid_until_unix_ms
        || candidate.start_slot >= input.horizon_slots
        || candidate.end_slot > input.horizon_slots
        || candidate.resource_allocations.is_empty()
        || candidate
            .start_slot
            .checked_sub(0)
            .and_then(|slot| slot_start_ms(input, slot))
            .is_none_or(|start_time| candidate.expires_at_unix_ms > start_time)
    {
        add_error(&mut errors, "candidate_lease_identity_window_or_expiry_invalid");
    }

    let mut commitment_ids = BTreeSet::new();
    let mut idempotency_keys = BTreeSet::new();
    if !commitment_ids.insert(candidate.commitment_id.as_str()) {
        add_error(&mut errors, "candidate_commitment_id_empty_or_duplicate");
    }
    if !idempotency_keys.insert(candidate.idempotency_key.as_str()) {
        add_error(&mut errors, "candidate_idempotency_key_empty_or_duplicate");
    }

    let mut candidate_resource_ids = BTreeSet::new();
    for allocation in &candidate.resource_allocations {
        if !candidate_resource_ids.insert(allocation.resource_id.as_str()) {
            add_error(
                &mut errors,
                format!("candidate_resource_allocation_duplicate:{}", allocation.resource_id),
            );
        }
        validate_allocation(
            allocation,
            &candidate.commitment_id,
            candidate.start_slot,
            candidate.end_slot,
            input,
            &budget_map,
            &mut errors,
        );
    }

    let mut active_commitment_count = 0_usize;
    let mut terminal_commitment_count = 0_usize;

    let mut sorted_existing = input.existing_commitments.clone();
    sorted_existing.sort_by(|a, b| a.commitment_id.cmp(&b.commitment_id));
    for commitment in &sorted_existing {
        if commitment.commitment_id.trim().is_empty()
            || commitment.idempotency_key.trim().is_empty()
            || !commitment_ids.insert(commitment.commitment_id.as_str())
            || !idempotency_keys.insert(commitment.idempotency_key.as_str())
        {
            add_error(
                &mut errors,
                format!("existing_commitment_identity_or_idempotency_reused:{}", commitment.commitment_id),
            );
            continue;
        }

        if commitment.start_slot >= commitment.end_slot
            || commitment.end_slot > input.horizon_slots
            || commitment.resource_allocations.is_empty()
        {
            add_error(
                &mut errors,
                format!("existing_commitment_window_invalid:{}", commitment.commitment_id),
            );
            continue;
        }

        if !evidence_current_at(
            &commitment.state_evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
        ) {
            add_error(
                &mut errors,
                format!("commitment_state_evidence_invalid:{}", commitment.commitment_id),
            );
        }

        if commitment.state.is_terminal()
            && !matches!(
                commitment.state_evidence.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            )
        {
            add_error(
                &mut errors,
                format!("terminal_commitment_release_requires_source_grade_evidence:{}", commitment.commitment_id),
            );
        }

        if commitment.state == CommitmentState::FirmOffered
            && (commitment.expires_at_unix_ms <= input.decision_time_unix_ms
                || commitment.expires_at_unix_ms > input.report_valid_until_unix_ms)
        {
            add_error(
                &mut errors,
                format!("firm_offer_expiry_invalid:{}", commitment.commitment_id),
            );
        }

        let mut resource_ids = BTreeSet::new();
        for allocation in &commitment.resource_allocations {
            if !resource_ids.insert(allocation.resource_id.as_str()) {
                add_error(
                    &mut errors,
                    format!("existing_commitment_resource_duplicate:{}:{}", commitment.commitment_id, allocation.resource_id),
                );
            }
            validate_allocation(
                allocation,
                &commitment.commitment_id,
                commitment.start_slot,
                commitment.end_slot,
                input,
                &budget_map,
                &mut errors,
            );
        }

        if commitment.state.reserves_capability() {
            active_commitment_count += 1;
        } else if commitment.state.is_terminal() {
            terminal_commitment_count += 1;
        }
    }

    let mut protected_window_conflicts = BTreeSet::new();
    let mut sorted_windows = input.protected_windows.clone();
    sorted_windows.sort_by(|a, b| a.window_id.cmp(&b.window_id));
    let mut window_ids = BTreeSet::new();

    for window in &sorted_windows {
        let Some(budget) = budget_map.get(window.resource_id.as_str()) else {
            add_error(
                &mut errors,
                format!("protected_window_resource_unknown:{}", window.window_id),
            );
            continue;
        };
        if window.window_id.trim().is_empty()
            || !window_ids.insert(window.window_id.as_str())
            || window.start_slot >= window.end_slot
            || window.end_slot > input.horizon_slots
            || window.claim_reference.trim().is_empty()
        {
            add_error(
                &mut errors,
                format!("protected_window_identity_or_interval_invalid:{}", window.window_id),
            );
            continue;
        }
        let Some(start) = slot_start_ms(input, window.start_slot) else {
            add_error(&mut errors, format!("protected_window_start_time_overflow:{}", window.window_id));
            continue;
        };
        let Some(end) = slot_end_exclusive_ms(input, window.end_slot - 1) else {
            add_error(&mut errors, format!("protected_window_end_time_overflow:{}", window.window_id));
            continue;
        };
        if !evidence_covers(
            &window.evidence,
            &window.resource_id,
            &budget.configuration_digest,
            input.decision_time_unix_ms,
            start,
            end,
        ) {
            add_error(
                &mut errors,
                format!("protected_window_evidence_invalid:{}", window.window_id),
            );
        }

        if window.resource_id.trim().is_empty() {
            add_error(&mut errors, "protected_window_resource_empty");
        }
        if candidate.resource_allocations.iter().any(|allocation| {
            allocation.resource_id == window.resource_id
                && (window.start_slot..window.end_slot).any(|slot| {
                    allocation.power_w_by_slot[usize::from(slot)] > 0
                        || allocation.energy_wh_by_slot[usize::from(slot)] > 0
                })
        }) {
            protected_window_conflicts.insert(format!(
                "{}:{}:{:?}",
                window.window_id, window.resource_id, window.claim_kind
            ));
        }
    }

    let mut active_aggregates: BTreeMap<String, (Vec<u64>, Vec<u64>)> = BTreeMap::new();
    for commitment in &sorted_existing {
        if !commitment.state.reserves_capability() {
            continue;
        }
        for allocation in &commitment.resource_allocations {
            let (energy, power) = active_aggregates
                .entry(allocation.resource_id.clone())
                .or_insert_with(|| {
                    (
                        vec![0_u64; usize::from(input.horizon_slots)],
                        vec![0_u64; usize::from(input.horizon_slots)],
                    )
                });
            for slot in 0..usize::from(input.horizon_slots) {
                let energy_value = allocation.energy_wh_by_slot.get(slot).copied().unwrap_or(0);
            let power_value = allocation.power_w_by_slot.get(slot).copied().unwrap_or(0);
            energy[slot] = match energy[slot].checked_add(energy_value) {
                    Some(value) => value,
                    None => {
                        add_error(&mut errors, "active_commitment_energy_overflow");
                        energy[slot]
                    }
                };
                power[slot] = match power[slot].checked_add(u64::from(power_value)) {
                    Some(value) => value,
                    None => {
                        add_error(&mut errors, "active_commitment_power_overflow");
                        power[slot]
                    }
                };
            }
        }
    }

    for allocation in &candidate.resource_allocations {
        let (energy, power) = active_aggregates
            .entry(allocation.resource_id.clone())
            .or_insert_with(|| {
                (
                    vec![0_u64; usize::from(input.horizon_slots)],
                    vec![0_u64; usize::from(input.horizon_slots)],
                )
            });
        for slot in 0..usize::from(input.horizon_slots) {
            let energy_value = allocation.energy_wh_by_slot.get(slot).copied().unwrap_or(0);
            let power_value = allocation.power_w_by_slot.get(slot).copied().unwrap_or(0);
            energy[slot] = match energy[slot].checked_add(energy_value) {
                Some(value) => value,
                None => {
                    add_error(&mut errors, "candidate_commitment_energy_overflow");
                    energy[slot]
                }
            };
            power[slot] = match power[slot].checked_add(u64::from(power_value)) {
                Some(value) => value,
                None => {
                    add_error(&mut errors, "candidate_commitment_power_overflow");
                    power[slot]
                }
            };
        }
    }

    if !protected_window_conflicts.is_empty() {
        add_error(&mut errors, "candidate_overlaps_protected_mobility_or_maintenance_window");
    }

    let mut resource_margins = Vec::new();
    for budget in &input.resource_budgets {
        let (energy, power) = active_aggregates
            .get(&budget.resource_id)
            .cloned()
            .unwrap_or_else(|| {
                (
                    vec![0_u64; usize::from(input.horizon_slots)],
                    vec![0_u64; usize::from(input.horizon_slots)],
                )
            });
        let mut energy_used = 0_u64;
        let mut power_margins = Vec::new();
        let mut power_feasible = true;
        for slot in 0..usize::from(input.horizon_slots) {
            energy_used = match energy_used.checked_add(energy[slot]) {
                Some(value) => value,
                None => {
                    add_error(&mut errors, "resource_horizon_energy_overflow");
                    energy_used
                }
            };
            let limit = u64::from(
                budget
                    .safe_grid_power_budget_w_by_slot
                    .get(slot)
                    .copied()
                    .unwrap_or(0),
            );
            if power[slot] > limit {
                power_feasible = false;
            }
            let margin = if limit >= power[slot] {
                i64::try_from(limit - power[slot]).unwrap_or(i64::MAX)
            } else {
                -i64::try_from(power[slot] - limit).unwrap_or(i64::MAX)
            };
            power_margins.push(margin);
        }
        let energy_margin = if budget.safe_grid_energy_budget_wh >= energy_used {
            i64::try_from(budget.safe_grid_energy_budget_wh - energy_used).unwrap_or(i64::MAX)
        } else {
            -i64::try_from(energy_used - budget.safe_grid_energy_budget_wh).unwrap_or(i64::MAX)
        };
        if energy_used > budget.safe_grid_energy_budget_wh || !power_feasible {
            add_error(
                &mut errors,
                format!("resource_capability_overcommitted:{}", budget.resource_id),
            );
        }
        resource_margins.push(ResourceLeaseMargin {
            resource_id: budget.resource_id.clone(),
            safe_grid_energy_budget_wh: budget.safe_grid_energy_budget_wh,
            committed_energy_wh: energy_used,
            remaining_energy_wh: energy_margin,
            committed_power_w_by_slot: power.iter().map(|value| u32::try_from(*value).unwrap_or(u32::MAX)).collect(),
            remaining_power_w_by_slot: power_margins,
        });
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    resource_margins.sort_by(|a, b| a.resource_id.cmp(&b.resource_id));
    let candidate_model_admissible = true;

    Ok(MobilityCommitmentLeaseReport {
        schema_version: input.schema_version.clone(),
        benchmark_id: input.benchmark_id.clone(),
        site_id: input.site_id.clone(),
        decision_time_unix_ms: input.decision_time_unix_ms,
        horizon_slots: input.horizon_slots,
        candidate_commitment_id: candidate.commitment_id.clone(),
        active_commitment_count,
        terminal_commitment_count,
        candidate_model_admissible,
        resource_margins,
        protected_window_conflicts: protected_window_conflicts.into_iter().collect(),
        findings: vec![
            "candidate_is_model_admissible_but_not_written_to_any_ledger".to_owned(),
            "terminal_states_release_capacity_only_with_measured_or_independent_test_evidence".to_owned(),
            "unresolved_commitments_continue_to_reserve_resource_capacity".to_owned(),
        ],
        ledger_mutation_performed: false,
        market_authority_granted: false,
        dispatch_authority_granted: false,
        physical_mutation_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}
