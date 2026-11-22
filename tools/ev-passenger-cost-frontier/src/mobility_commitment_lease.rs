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
    pub state_receipt_id: String,
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
    allocation.power_w_by_slot.iter().any(|power| *power > 0)
        || allocation
            .energy_wh_by_slot
            .iter()
            .any(|energy| *energy > 0)
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
            format!(
                "allocation_resource_unknown:{commitment_id}:{}",
                allocation.resource_id
            ),
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
            format!(
                "allocation_identity_or_horizon_invalid:{commitment_id}:{}",
                allocation.resource_id
            ),
        );
        return;
    }

    let Some(interval_start) = slot_start_ms(input, start_slot) else {
        add_error(
            errors,
            format!("allocation_start_time_overflow:{commitment_id}"),
        );
        return;
    };
    let Some(interval_end) = slot_end_exclusive_ms(input, end_slot - 1) else {
        add_error(
            errors,
            format!("allocation_end_time_overflow:{commitment_id}"),
        );
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
            format!(
                "allocation_evidence_not_admissible:{commitment_id}:{}",
                allocation.resource_id
            ),
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
                format!(
                    "allocation_slot_value_or_window_invalid:{commitment_id}:{}:{slot}",
                    allocation.resource_id
                ),
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

    let horizon_end = slot_end_exclusive_ms(input, input.horizon_slots.saturating_sub(1));
    if horizon_end.is_none_or(|end| input.report_valid_until_unix_ms < end.saturating_sub(1)) {
        add_error(
            &mut errors,
            "upstream_report_validity_does_not_cover_full_horizon",
        );
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
        add_error(
            &mut errors,
            "flexibility_report_evidence_not_bound_to_digest",
        );
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
            || budget_map
                .insert(budget.resource_id.as_str(), budget)
                .is_some()
        {
            add_error(
                &mut errors,
                format!(
                    "resource_budget_identity_or_horizon_invalid:{}",
                    budget.resource_id
                ),
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
                format!(
                    "resource_site_connection_evidence_invalid:{}",
                    budget.resource_id
                ),
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
        || slot_start_ms(input, candidate.start_slot)
            .is_none_or(|start_time| candidate.expires_at_unix_ms > start_time)
    {
        add_error(
            &mut errors,
            "candidate_lease_identity_window_or_expiry_invalid",
        );
    }

    let mut commitment_ids = BTreeSet::new();
    let mut idempotency_keys = BTreeSet::new();
    let mut state_receipt_ids = BTreeSet::new();
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
                format!(
                    "candidate_resource_allocation_duplicate:{}",
                    allocation.resource_id
                ),
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
                format!(
                    "existing_commitment_identity_or_idempotency_reused:{}",
                    commitment.commitment_id
                ),
            );
            continue;
        }

        if commitment.start_slot >= commitment.end_slot
            || commitment.end_slot > input.horizon_slots
            || commitment.resource_allocations.is_empty()
        {
            add_error(
                &mut errors,
                format!(
                    "existing_commitment_window_invalid:{}",
                    commitment.commitment_id
                ),
            );
            continue;
        }

        if commitment.state_receipt_id.trim().is_empty()
            || commitment.state_evidence.evidence_reference != commitment.state_receipt_id
            || !state_receipt_ids.insert(commitment.state_receipt_id.as_str())
        {
            add_error(
                &mut errors,
                format!(
                    "commitment_state_receipt_empty_reused_or_mismatched:{}",
                    commitment.commitment_id
                ),
            );
        }

        if !evidence_current_at(
            &commitment.state_evidence,
            &input.site_id,
            &input.site_configuration_digest,
            input.decision_time_unix_ms,
        ) {
            add_error(
                &mut errors,
                format!(
                    "commitment_state_evidence_invalid:{}",
                    commitment.commitment_id
                ),
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
                format!(
                    "terminal_commitment_release_requires_source_grade_evidence:{}",
                    commitment.commitment_id
                ),
            );
        }

        let offer_start_time = slot_start_ms(input, commitment.start_slot);
        if commitment.state == CommitmentState::FirmOffered
            && (commitment.expires_at_unix_ms <= input.decision_time_unix_ms
                || commitment.expires_at_unix_ms > input.report_valid_until_unix_ms
                || offer_start_time.is_none_or(|start| commitment.expires_at_unix_ms > start))
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
                    format!(
                        "existing_commitment_resource_duplicate:{}:{}",
                        commitment.commitment_id, allocation.resource_id
                    ),
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
                format!(
                    "protected_window_identity_or_interval_invalid:{}",
                    window.window_id
                ),
            );
            continue;
        }
        let Some(start) = slot_start_ms(input, window.start_slot) else {
            add_error(
                &mut errors,
                format!("protected_window_start_time_overflow:{}", window.window_id),
            );
            continue;
        };
        let Some(end) = slot_end_exclusive_ms(input, window.end_slot - 1) else {
            add_error(
                &mut errors,
                format!("protected_window_end_time_overflow:{}", window.window_id),
            );
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
        let candidate_conflicts = candidate.resource_allocations.iter().any(|allocation| {
            allocation.resource_id == window.resource_id
                && (window.start_slot..window.end_slot).any(|slot| {
                    allocation
                        .power_w_by_slot
                        .get(usize::from(slot))
                        .copied()
                        .unwrap_or(0)
                        > 0
                        || allocation
                            .energy_wh_by_slot
                            .get(usize::from(slot))
                            .copied()
                            .unwrap_or(0)
                            > 0
                })
        });
        let existing_conflicts = sorted_existing.iter().any(|commitment| {
            commitment.state.reserves_capability()
                && commitment.resource_allocations.iter().any(|allocation| {
                    allocation.resource_id == window.resource_id
                        && (window.start_slot..window.end_slot).any(|slot| {
                            allocation
                                .power_w_by_slot
                                .get(usize::from(slot))
                                .copied()
                                .unwrap_or(0)
                                > 0
                                || allocation
                                    .energy_wh_by_slot
                                    .get(usize::from(slot))
                                    .copied()
                                    .unwrap_or(0)
                                    > 0
                        })
                })
        });
        if candidate_conflicts || existing_conflicts {
            protected_window_conflicts.insert(format!(
                "{}:{}:{:?}",
                window.window_id, window.resource_id, window.claim_kind
            ));
            if existing_conflicts {
                add_error(
                    &mut errors,
                    format!(
                        "existing_commitment_overlaps_protected_window:{}",
                        window.window_id
                    ),
                );
            }
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
        add_error(
            &mut errors,
            "candidate_overlaps_protected_mobility_or_maintenance_window",
        );
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
            committed_power_w_by_slot: power
                .iter()
                .map(|value| u32::try_from(*value).unwrap_or(u32::MAX))
                .collect(),
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
            "terminal_states_release_capacity_only_with_measured_or_independent_test_evidence"
                .to_owned(),
            "unresolved_commitments_continue_to_reserve_resource_capacity".to_owned(),
        ],
        ledger_mutation_performed: false,
        market_authority_granted: false,
        dispatch_authority_granted: false,
        physical_mutation_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}


#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> MobilityCommitmentLeaseInput {
        serde_json::from_str(include_str!(
            "../fixtures/mobility_commitment_lease_v0_1.json"
        ))
        .expect("frozen commitment lease fixture must parse")
    }

    fn reference_admissible(input: &MobilityCommitmentLeaseInput) -> bool {
        for budget in &input.resource_budgets {
            let mut energy_used = 0_u64;
            let mut power_by_slot = vec![0_u64; usize::from(input.horizon_slots)];
            for commitment in &input.existing_commitments {
                if !commitment.state.reserves_capability() {
                    continue;
                }
                for allocation in &commitment.resource_allocations {
                    if allocation.resource_id != budget.resource_id {
                        continue;
                    }
                    energy_used += allocation.energy_wh_by_slot.iter().sum::<u64>();
                    for (slot, power) in allocation.power_w_by_slot.iter().enumerate() {
                        power_by_slot[slot] += u64::from(*power);
                    }
                }
            }
            for allocation in &input.candidate_lease.resource_allocations {
                if allocation.resource_id != budget.resource_id {
                    continue;
                }
                energy_used += allocation.energy_wh_by_slot.iter().sum::<u64>();
                for (slot, power) in allocation.power_w_by_slot.iter().enumerate() {
                    power_by_slot[slot] += u64::from(*power);
                }
            }
            if energy_used > budget.safe_grid_energy_budget_wh {
                return false;
            }
            if power_by_slot
                .iter()
                .zip(&budget.safe_grid_power_budget_w_by_slot)
                .any(|(used, limit)| *used > u64::from(*limit))
            {
                return false;
            }
        }

        !input.protected_windows.iter().any(|window| {
            input
                .candidate_lease
                .resource_allocations
                .iter()
                .any(|allocation| {
                    allocation.resource_id == window.resource_id
                        && (window.start_slot..window.end_slot).any(|slot| {
                            allocation.power_w_by_slot[usize::from(slot)] > 0
                                || allocation.energy_wh_by_slot[usize::from(slot)] > 0
                        })
                })
        })
    }

    #[test]
    fn known_answer_admits_back_to_back_lease_without_mutation_or_authority() {
        let report =
            evaluate_mobility_commitment_lease(&fixture()).expect("known answer is admissible");
        assert!(report.candidate_model_admissible);
        assert_eq!(report.active_commitment_count, 2);
        assert_eq!(report.terminal_commitment_count, 1);
        assert!(!report.ledger_mutation_performed);
        assert!(!report.market_authority_granted);
        assert!(!report.dispatch_authority_granted);
        assert!(!report.physical_mutation_authority_granted);

        let ess_01 = report
            .resource_margins
            .iter()
            .find(|margin| margin.resource_id == "ess-01")
            .expect("ess-01 margin");
        assert_eq!(ess_01.committed_energy_wh, 1_100);
        assert_eq!(ess_01.remaining_energy_wh, 400);
        assert_eq!(
            ess_01.committed_power_w_by_slot,
            vec![500, 500, 2_500, 1_500, 0, 0]
        );
    }

    #[test]
    fn unresolved_commitment_still_consumes_power_and_energy() {
        let mut input = fixture();
        input
            .candidate_lease
            .resource_allocations
            .push(LeaseResourceAllocation {
                resource_id: "ess-02".to_owned(),
                configuration_digest: "sha256:ess-02-v01".to_owned(),
                power_w_by_slot: vec![0, 0, 1_600, 0, 0, 0],
                energy_wh_by_slot: vec![0, 0, 100, 0, 0, 0],
                allocation_evidence: input.existing_commitments[1].resource_allocations[0]
                    .allocation_evidence
                    .clone(),
            });
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("unresolved lease plus candidate exceeds ess-02 slot power");
        assert!(
            errors
                .iter()
                .any(|error| error == "resource_capability_overcommitted:ess-02")
        );
    }

    #[test]
    fn cumulative_energy_is_checked_per_resource_not_pooled() {
        let mut input = fixture();
        input.candidate_lease.resource_allocations[0].energy_wh_by_slot[2] = 1_200;
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("ess-01 energy is overcommitted despite ess-02 slack");
        assert!(
            errors
                .iter()
                .any(|error| error == "resource_capability_overcommitted:ess-01")
        );
    }

    #[test]
    fn resource_power_overcommit_is_rejected() {
        let mut input = fixture();
        input.candidate_lease.resource_allocations[0].power_w_by_slot[2] = 2_600;
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("existing plus candidate power exceeds 3 kW");
        assert!(
            errors
                .iter()
                .any(|error| error == "resource_capability_overcommitted:ess-01")
        );
    }

    #[test]
    fn mission_protected_window_blocks_candidate_even_when_budget_remains() {
        let mut input = fixture();
        input.candidate_lease.end_slot = 6;
        input
            .candidate_lease
            .resource_allocations
            .push(LeaseResourceAllocation {
                resource_id: "ess-02".to_owned(),
                configuration_digest: "sha256:ess-02-v01".to_owned(),
                power_w_by_slot: vec![0, 0, 0, 0, 500, 500],
                energy_wh_by_slot: vec![0, 0, 0, 0, 50, 50],
                allocation_evidence: input.existing_commitments[1].resource_allocations[0]
                    .allocation_evidence
                    .clone(),
            });
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("mission-protected interval cannot be promised to grid service");
        assert!(errors
            .iter()
            .any(|error| error == "candidate_overlaps_protected_mobility_or_maintenance_window"));
    }

    #[test]
    fn terminal_release_requires_measured_or_independent_test_evidence() {
        let mut input = fixture();
        input.existing_commitments[2].state_evidence.evidence_class = EvidenceClass::Synthetic;
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("synthetic terminal evidence cannot release a live reservation");
        assert!(errors.iter().any(|error| {
            error == "terminal_commitment_release_requires_source_grade_evidence:grid-lease-settled-03"
        }));
    }

    #[test]
    fn idempotency_key_reuse_is_rejected() {
        let mut input = fixture();
        input.candidate_lease.idempotency_key =
            input.existing_commitments[0].idempotency_key.clone();
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("candidate key has already been used");
        assert!(errors.iter().any(|error| {
            error == "existing_commitment_identity_or_idempotency_reused:grid-lease-existing-01"
        }));
    }

    #[test]
    fn terminal_state_receipt_cannot_be_replayed_across_commitments() {
        let mut input = fixture();
        input.existing_commitments[1].state_receipt_id =
            input.existing_commitments[0].state_receipt_id.clone();
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("state receipts must be unique across commitment records");
        assert!(errors.iter().any(|error| {
            error == "commitment_state_receipt_empty_reused_or_mismatched:grid-lease-uncertain-02"
        }));
    }

    #[test]
    fn state_evidence_must_reference_its_exact_receipt() {
        let mut input = fixture();
        input.existing_commitments[2]
            .state_evidence
            .evidence_reference = "synthetic://some-other-settlement".to_owned();
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("settlement evidence cannot be detached from the lease receipt");
        assert!(errors.iter().any(|error| {
            error == "commitment_state_receipt_empty_reused_or_mismatched:grid-lease-settled-03"
        }));
    }

    #[test]
    fn upstream_report_validity_must_cover_the_full_horizon() {
        let mut input = fixture();
        input.report_valid_until_unix_ms =
            input.decision_time_unix_ms + u64::from(input.slot_duration_seconds) * 1_000;
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("a report that expires mid-horizon cannot back a later slot");
        assert!(
            errors
                .iter()
                .any(|error| { error == "upstream_report_validity_does_not_cover_full_horizon" })
        );
    }

    #[test]
    fn invalid_or_expired_report_digest_is_rejected() {
        let mut input = fixture();
        input.recovery_report_digest = "not-a-digest".to_owned();
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("source report digest must be canonical sha256 notation");
        assert!(
            errors
                .iter()
                .any(|error| error == "unsupported_identity_digest_or_horizon")
        );
    }

    #[test]
    fn firm_offer_expiring_after_its_delivery_window_starts_is_rejected() {
        let mut input = fixture();
        input.candidate_lease.expires_at_unix_ms =
            input.decision_time_unix_ms + 2 * u64::from(input.slot_duration_seconds) * 1_000 + 1;
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("a firm offer must expire before its delivery window starts");
        assert!(
            errors
                .iter()
                .any(|error| error == "candidate_lease_identity_window_or_expiry_invalid")
        );
    }

    #[test]
    fn malformed_resource_vector_is_rejected_without_indexing_panic() {
        let mut input = fixture();
        input.candidate_lease.resource_allocations[0]
            .power_w_by_slot
            .pop();
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("allocation vectors must cover the exact horizon");
        assert!(errors.iter().any(|error| {
            error.contains("allocation_identity_or_horizon_invalid:grid-lease-candidate-04:ess-01")
        }));
    }

    #[test]
    fn candidate_allocation_must_match_exact_resource_configuration() {
        let mut input = fixture();
        input.candidate_lease.resource_allocations[0].configuration_digest =
            "sha256:replaced-config".to_owned();
        let errors = evaluate_mobility_commitment_lease(&input)
            .expect_err("stale or different resource configuration cannot be reused");
        assert!(errors.iter().any(|error| {
            error.contains("allocation_identity_or_horizon_invalid:grid-lease-candidate-04:ess-01")
        }));
    }

    #[test]
    fn input_order_permutation_preserves_canonical_report() {
        let input = fixture();
        let mut permuted = fixture();
        permuted.resource_budgets.reverse();
        permuted.protected_windows.reverse();
        permuted.existing_commitments.reverse();
        for commitment in &mut permuted.existing_commitments {
            commitment.resource_allocations.reverse();
        }
        permuted.candidate_lease.resource_allocations.reverse();

        let first = evaluate_mobility_commitment_lease(&input).expect("first input");
        let second = evaluate_mobility_commitment_lease(&permuted).expect("permuted input");
        assert_eq!(
            serde_json::to_string(&first).expect("serialize first"),
            serde_json::to_string(&second).expect("serialize second")
        );
    }

    #[test]
    fn evaluator_matches_independent_reference_for_12_candidate_profiles() {
        let mut input = fixture();
        let powers = [0_u32, 1_000, 2_500, 2_600];
        let energies = [0_u64, 100, 700];
        let mut compared = 0_usize;
        let mut feasible = 0_usize;

        for power in powers {
            for energy in energies {
                input.candidate_lease.resource_allocations[0].power_w_by_slot[2] = power;
                input.candidate_lease.resource_allocations[0].energy_wh_by_slot[2] = energy;
                let expected = reference_admissible(&input);
                let actual = evaluate_mobility_commitment_lease(&input).is_ok();
                assert_eq!(actual, expected, "power={power}, energy={energy}");
                if expected {
                    feasible += 1;
                }
                compared += 1;
            }
        }
        assert_eq!(compared, 12);
        assert_eq!(feasible, 9);
    }
}
