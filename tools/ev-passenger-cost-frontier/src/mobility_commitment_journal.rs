// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Append-only capability commitment event reducer with optimistic CAS.
//!
//! This journal owns external promises, not physical battery state. It is
//! storage-agnostic; the in-memory adapter is a concurrency test harness only.
//! SHA-256 event chaining detects accidental or malicious event mutation, but
//! does not authenticate producers or establish distributed consensus.

use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

use crate::battery_service_network::{EvidenceClass, EvidenceEnvelope};

const MAX_RESOURCES: usize = 4;
const MAX_HORIZON_SLOTS: u8 = 16;
const MAX_EVENTS: usize = 4_096;
const MAX_POWER_W: u32 = 100_000_000;
const MAX_ENERGY_WH: u64 = 1_000_000_000_000;
const CLAIM_CEILING: &str = "Append-only model-level promise journal with local CAS semantics only; no durable storage, distributed consensus, market settlement, physical dispatch, battery state or actuation authority.";

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
    fn is_terminal(self) -> bool {
        matches!(
            self,
            Self::Rejected | Self::Expired | Self::Cancelled | Self::Settled
        )
    }

}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum JournalEventKind {
    OfferCreated,
    Accepted,
    Executing,
    Unresolved,
    Settled,
    Rejected,
    Expired,
    Cancelled,
    Reconciled,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct JournalResourceAllocation {
    pub resource_id: String,
    pub configuration_digest: String,
    pub power_w_by_slot: Vec<u32>,
    pub energy_wh_by_slot: Vec<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct CommitmentContract {
    pub site_id: String,
    pub site_configuration_digest: String,
    pub slot_duration_seconds: u32,
    pub horizon_slots: u8,
    pub start_slot: u8,
    pub end_slot: u8,
    pub expires_at_unix_ms: u64,
    pub flexibility_report_digest: String,
    pub recovery_report_digest: String,
    pub report_valid_until_unix_ms: u64,
    pub resource_allocations: Vec<JournalResourceAllocation>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ResourceTerminalEvidence {
    pub resource_id: String,
    pub configuration_digest: String,
    pub receipt_id: String,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
pub struct AppendRequest {
    pub expected_version: u64,
    pub event_id: String,
    pub commitment_id: String,
    pub idempotency_key: String,
    pub kind: JournalEventKind,
    pub resolved_state: Option<CommitmentState>,
    pub event_time_unix_ms: u64,
    pub event_evidence: EvidenceEnvelope,
    pub resource_terminal_evidence: Vec<ResourceTerminalEvidence>,
    pub contract: Option<CommitmentContract>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CommitmentEvent {
    pub sequence: u64,
    pub expected_version: u64,
    pub event_id: String,
    pub commitment_id: String,
    pub idempotency_key: String,
    pub payload_digest: String,
    pub kind: JournalEventKind,
    pub resolved_state: Option<CommitmentState>,
    pub resulting_state: CommitmentState,
    pub event_time_unix_ms: u64,
    pub previous_event_digest: Option<String>,
    pub event_evidence: EvidenceEnvelope,
    pub resource_terminal_evidence: Vec<ResourceTerminalEvidence>,
    pub contract: Option<CommitmentContract>,
    pub event_digest: String,
}

#[derive(Debug, Clone, Default, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalSnapshot {
    pub version: u64,
    pub events: Vec<CommitmentEvent>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct CommitmentReceipt {
    pub sequence: u64,
    pub event_id: String,
    pub commitment_id: String,
    pub resulting_state: CommitmentState,
    pub payload_digest: String,
    pub event_digest: String,
    pub version_after: u64,
    pub idempotent_replay: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppendResult {
    pub snapshot: JournalSnapshot,
    pub receipt: CommitmentReceipt,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct ReplayedCommitment {
    pub state: CommitmentState,
    pub payload_digest: String,
    pub contract: CommitmentContract,
    pub last_sequence: u64,
    pub last_event_digest: String,
}

#[derive(Debug, Clone, Default, Serialize, PartialEq, Eq)]
pub struct ReplayState {
    pub version: u64,
    pub commitments: BTreeMap<String, ReplayedCommitment>,
    pub idempotency_receipts: BTreeMap<String, CommitmentReceipt>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct JournalScenario {
    pub schema_version: String,
    pub benchmark_id: String,
    pub decision_time_unix_ms: u64,
    pub initial_snapshot: JournalSnapshot,
    pub commands: Vec<AppendRequest>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct JournalScenarioReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub initial_version: u64,
    pub final_version: u64,
    pub event_count: usize,
    pub commitment_states: BTreeMap<String, CommitmentState>,
    pub last_event_digest: Option<String>,
    pub replay_digest: String,
    pub unresolved_commitments: Vec<String>,
    pub terminal_commitments: Vec<String>,
    pub ledger_mutation_performed: bool,
    pub market_authority_granted: bool,
    pub dispatch_authority_granted: bool,
    pub physical_mutation_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(71);
    output.push_str("sha256:");
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

pub fn digest_contract(contract: &CommitmentContract) -> Result<String, String> {
    let bytes = serde_json::to_vec(contract)
        .map_err(|error| format!("contract_serialization_failed:{error}"))?;
    Ok(sha256_hex(&bytes))
}

fn digest_event(event: &CommitmentEvent) -> Result<String, String> {
    let mut unsigned = event.clone();
    unsigned.event_digest.clear();
    let bytes = serde_json::to_vec(&unsigned)
        .map_err(|error| format!("event_serialization_failed:{error}"))?;
    Ok(sha256_hex(&bytes))
}

fn is_sha256(value: &str) -> bool {
    let Some(hex) = value.strip_prefix("sha256:") else {
        return false;
    };
    hex.len() == 64 && hex.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn slot_start_ms(contract: &CommitmentContract, decision_time_unix_ms: u64, slot: u8) -> Option<u64> {
    u64::from(slot)
        .checked_mul(u64::from(contract.slot_duration_seconds))?
        .checked_mul(1_000)?
        .checked_add(decision_time_unix_ms)
}

fn horizon_end_exclusive_ms(
    contract: &CommitmentContract,
    decision_time_unix_ms: u64,
) -> Option<u64> {
    u64::from(contract.horizon_slots)
        .checked_mul(u64::from(contract.slot_duration_seconds))?
        .checked_mul(1_000)?
        .checked_add(decision_time_unix_ms)
}

fn validate_contract(
    contract: &CommitmentContract,
    decision_time_unix_ms: u64,
) -> Result<(), String> {
    if contract.site_id.trim().is_empty()
        || contract.site_configuration_digest.trim().is_empty()
        || contract.slot_duration_seconds == 0
        || contract.horizon_slots == 0
        || contract.horizon_slots > MAX_HORIZON_SLOTS
        || contract.slot_duration_seconds > 86_400
        || contract.start_slot >= contract.end_slot
        || contract.end_slot > contract.horizon_slots
        || contract.resource_allocations.is_empty()
        || contract.resource_allocations.len() > MAX_RESOURCES
        || !is_sha256(&contract.flexibility_report_digest)
        || !is_sha256(&contract.recovery_report_digest)
    {
        return Err("contract_identity_or_horizon_invalid".to_owned());
    }

    let horizon_end = horizon_end_exclusive_ms(contract, decision_time_unix_ms)
        .ok_or_else(|| "contract_horizon_time_overflow".to_owned())?;
    if contract.report_valid_until_unix_ms < horizon_end.saturating_sub(1) {
        return Err("upstream_reports_do_not_cover_contract_horizon".to_owned());
    }
    let delivery_start = slot_start_ms(contract, decision_time_unix_ms, contract.start_slot)
        .ok_or_else(|| "contract_delivery_start_overflow".to_owned())?;
    if contract.expires_at_unix_ms <= decision_time_unix_ms
        || contract.expires_at_unix_ms > contract.report_valid_until_unix_ms
        || contract.expires_at_unix_ms > delivery_start
    {
        return Err("contract_expiry_not_before_delivery_or_report_expiry".to_owned());
    }

    let mut resources = BTreeSet::new();
    for allocation in &contract.resource_allocations {
        if allocation.resource_id.trim().is_empty()
            || allocation.configuration_digest.trim().is_empty()
            || !resources.insert(allocation.resource_id.as_str())
            || allocation.power_w_by_slot.len() != usize::from(contract.horizon_slots)
            || allocation.energy_wh_by_slot.len() != usize::from(contract.horizon_slots)
            || allocation.power_w_by_slot.iter().any(|power| *power > MAX_POWER_W)
            || allocation.energy_wh_by_slot.iter().any(|energy| *energy > MAX_ENERGY_WH)
            || (!allocation.power_w_by_slot.iter().any(|power| *power > 0)
                && !allocation.energy_wh_by_slot.iter().any(|energy| *energy > 0))
        {
            return Err(format!(
                "contract_resource_allocation_invalid:{}",
                allocation.resource_id
            ));
        }
        for slot in 0..contract.horizon_slots {
            if (slot < contract.start_slot || slot >= contract.end_slot)
                && (allocation.power_w_by_slot[usize::from(slot)] > 0
                    || allocation.energy_wh_by_slot[usize::from(slot)] > 0)
            {
                return Err(format!(
                    "contract_allocation_outside_half_open_window:{}:{slot}",
                    allocation.resource_id
                ));
            }
        }
    }
    Ok(())
}

fn validate_evidence(
    evidence: &EvidenceEnvelope,
    expected_asset_id: &str,
    expected_configuration_digest: &str,
    expected_reference: &str,
    event_time_unix_ms: u64,
    decision_cutoff_unix_ms: u64,
    allow_unknown: bool,
) -> bool {
    if evidence.asset_id != expected_asset_id
        || evidence.configuration_digest != expected_configuration_digest
        || evidence.evidence_reference != expected_reference
        || evidence.asset_id.trim().is_empty()
        || evidence.configuration_digest.trim().is_empty()
        || evidence.producer_id.trim().is_empty()
        || evidence.method_id.trim().is_empty()
        || evidence.evidence_reference.trim().is_empty()
        || (!allow_unknown && evidence.evidence_class == EvidenceClass::Unknown)
    {
        return false;
    }
    match (
        evidence.observed_at_unix_ms,
        evidence.valid_from_unix_ms,
        evidence.valid_until_unix_ms,
    ) {
        (Some(observed), Some(valid_from), Some(valid_until)) => {
            valid_from <= event_time_unix_ms
                && event_time_unix_ms <= observed
                && observed <= decision_cutoff_unix_ms
                && event_time_unix_ms <= valid_until
        }
        _ => false,
    }
}

fn release_requires_resource_evidence(
    kind: JournalEventKind,
    resolved_state: Option<CommitmentState>,
) -> bool {
    match kind {
        JournalEventKind::Settled
        | JournalEventKind::Rejected
        | JournalEventKind::Expired
        | JournalEventKind::Cancelled => true,
        JournalEventKind::Reconciled => resolved_state.is_some_and(CommitmentState::is_terminal),
        _ => false,
    }
}

fn resulting_state(
    kind: JournalEventKind,
    prior: Option<CommitmentState>,
    resolved_state: Option<CommitmentState>,
) -> Result<CommitmentState, String> {
    use CommitmentState as State;
    use JournalEventKind as Kind;

    match (kind, prior, resolved_state) {
        (Kind::OfferCreated, None, None) => Ok(State::FirmOffered),
        (Kind::Accepted, Some(State::FirmOffered), None) => Ok(State::Accepted),
        (Kind::Executing, Some(State::Accepted), None) => Ok(State::Executing),
        (
            Kind::Unresolved,
            Some(State::FirmOffered | State::Accepted | State::Executing),
            None,
        ) => Ok(State::Unresolved),
        (Kind::Settled, Some(State::Executing), None) => Ok(State::Settled),
        (Kind::Rejected, Some(State::FirmOffered), None) => Ok(State::Rejected),
        (Kind::Expired, Some(State::FirmOffered), None) => Ok(State::Expired),
        (Kind::Cancelled, Some(State::Accepted | State::Executing), None) => Ok(State::Cancelled),
        (Kind::Reconciled, Some(State::Unresolved), Some(target))
            if matches!(
                target,
                State::Accepted
                    | State::Executing
                    | State::Settled
                    | State::Rejected
                    | State::Expired
                    | State::Cancelled
            ) =>
        {
            Ok(target)
        }
        _ => Err("invalid_commitment_state_transition".to_owned()),
    }
}

fn validate_resource_terminal_evidence(
    resources: &[JournalResourceAllocation],
    evidence: &[ResourceTerminalEvidence],
    event_time_unix_ms: u64,
    decision_cutoff_unix_ms: u64,
) -> Result<(), String> {
    if resources.len() != evidence.len() {
        return Err("terminal_resource_evidence_coverage_incomplete".to_owned());
    }
    let expected: BTreeMap<_, _> = resources
        .iter()
        .map(|resource| (resource.resource_id.as_str(), resource.configuration_digest.as_str()))
        .collect();
    let mut seen = BTreeSet::new();
    for item in evidence {
        if !seen.insert(item.resource_id.as_str())
            || expected.get(item.resource_id.as_str()).copied()
                != Some(item.configuration_digest.as_str())
            || item.receipt_id.trim().is_empty()
            || item.evidence.evidence_reference != item.receipt_id
            || !matches!(
                item.evidence.evidence_class,
                EvidenceClass::Measured | EvidenceClass::IndependentTest
            )
            || !validate_evidence(
                &item.evidence,
                &item.resource_id,
                &item.configuration_digest,
                &item.receipt_id,
                event_time_unix_ms,
                decision_cutoff_unix_ms,
                false,
            )
        {
            return Err(format!("terminal_resource_evidence_invalid:{}", item.resource_id));
        }
    }
    Ok(())
}

pub fn replay_journal(
    snapshot: &JournalSnapshot,
    decision_cutoff_unix_ms: u64,
) -> Result<ReplayState, String> {
    if snapshot.events.len() > MAX_EVENTS
        || u64::try_from(snapshot.events.len()).ok() != Some(snapshot.version)
    {
        return Err("journal_version_event_count_mismatch".to_owned());
    }

    let mut state = ReplayState {
        version: 0,
        commitments: BTreeMap::new(),
        idempotency_receipts: BTreeMap::new(),
    };
    let mut event_ids = BTreeSet::new();
    let mut previous_digest: Option<String> = None;
    let mut previous_event_time = 0_u64;

    for (index, event) in snapshot.events.iter().enumerate() {
        let expected_sequence = u64::try_from(index)
            .ok()
            .and_then(|value| value.checked_add(1))
            .ok_or_else(|| "journal_sequence_overflow".to_owned())?;
        if event.sequence != expected_sequence
            || event.expected_version != expected_sequence - 1
            || event.previous_event_digest != previous_digest
            || event.event_id.trim().is_empty()
            || event.commitment_id.trim().is_empty()
            || event.idempotency_key.trim().is_empty()
            || !event_ids.insert(event.event_id.as_str())
            || state.idempotency_receipts.contains_key(&event.idempotency_key)
            || event.event_time_unix_ms < previous_event_time
            || event.event_time_unix_ms > decision_cutoff_unix_ms
            || !is_sha256(&event.payload_digest)
        {
            return Err(format!("journal_event_order_identity_or_link_invalid:{}", event.sequence));
        }

        let event_digest = digest_event(event)?;
        if event.event_digest != event_digest {
            return Err(format!("journal_event_digest_mismatch:{}", event.sequence));
        }

        let prior = state.commitments.get(&event.commitment_id);
        let prior_state = prior.map(|item| item.state);
        let target_state = resulting_state(event.kind, prior_state, event.resolved_state)?;
        if target_state != event.resulting_state {
            return Err(format!("journal_resulting_state_mismatch:{}", event.sequence));
        }

        let contract = if event.kind == JournalEventKind::OfferCreated {
            if prior.is_some() {
                return Err("commitment_offer_created_more_than_once".to_owned());
            }
            let contract = event
                .contract
                .as_ref()
                .ok_or_else(|| "offer_created_contract_missing".to_owned())?;
            validate_contract(contract, event_time_origin(contract, event.event_time_unix_ms)?)?;
            let payload_digest = digest_contract(contract)?;
            if payload_digest != event.payload_digest {
                return Err("offer_created_payload_digest_mismatch".to_owned());
            }
            Some(contract.clone())
        } else {
            if event.contract.is_some() {
                return Err("contract_must_not_be_repeated_after_offer_created".to_owned());
            }
            let prior = prior.ok_or_else(|| "commitment_transition_without_offer".to_owned())?;
            if prior.payload_digest != event.payload_digest {
                return Err("commitment_payload_digest_changed_during_lifecycle".to_owned());
            }
            Some(prior.contract.clone())
        }
        .expect("all validated event paths provide a contract");

        if !validate_evidence(
            &event.event_evidence,
            &event.commitment_id,
            &contract.site_configuration_digest,
            &event.event_id,
            event.event_time_unix_ms,
            decision_cutoff_unix_ms,
            event.kind == JournalEventKind::Unresolved,
        ) {
            return Err(format!("journal_event_evidence_invalid:{}", event.sequence));
        }

        if event.kind == JournalEventKind::Expired
            && event.event_time_unix_ms < contract.expires_at_unix_ms
        {
            return Err("commitment_expired_before_expiry_time".to_owned());
        }

        if release_requires_resource_evidence(event.kind, event.resolved_state) {
            validate_resource_terminal_evidence(
                &contract.resource_allocations,
                &event.resource_terminal_evidence,
                event.event_time_unix_ms,
                decision_cutoff_unix_ms,
            )?;
        } else if event.kind == JournalEventKind::Reconciled {
            validate_resource_terminal_evidence(
                &contract.resource_allocations,
                &event.resource_terminal_evidence,
                event.event_time_unix_ms,
                decision_cutoff_unix_ms,
            )?;
        } else if !event.resource_terminal_evidence.is_empty() {
            return Err(format!("unexpected_resource_terminal_evidence:{}", event.sequence));
        }

        if event.kind == JournalEventKind::Reconciled
            && event.resolved_state.is_none()
        {
            return Err("reconciliation_target_missing".to_owned());
        } else if event.kind != JournalEventKind::Reconciled
            && event.resolved_state.is_some()
        {
            return Err("reconciliation_target_on_non_reconciliation_event".to_owned());
        }

        let receipt = CommitmentReceipt {
            sequence: event.sequence,
            event_id: event.event_id.clone(),
            commitment_id: event.commitment_id.clone(),
            resulting_state: event.resulting_state,
            payload_digest: event.payload_digest.clone(),
            event_digest: event.event_digest.clone(),
            version_after: event.sequence,
            idempotent_replay: false,
        };
        state.idempotency_receipts.insert(event.idempotency_key.clone(), receipt);
        state.commitments.insert(
            event.commitment_id.clone(),
            ReplayedCommitment {
                state: target_state,
                payload_digest: event.payload_digest.clone(),
                contract,
                last_sequence: event.sequence,
                last_event_digest: event.event_digest.clone(),
            },
        );
        state.version = event.sequence;
        previous_digest = Some(event.event_digest.clone());
        previous_event_time = event.event_time_unix_ms;
    }

    Ok(state)
}

fn event_time_origin(contract: &CommitmentContract, event_time: u64) -> Result<u64, String> {
    if event_time < contract.expires_at_unix_ms {
        Ok(event_time)
    } else {
        Err("offer_created_after_contract_expiry".to_owned())
    }
}

pub fn append_event(
    snapshot: &JournalSnapshot,
    request: &AppendRequest,
    decision_cutoff_unix_ms: u64,
) -> Result<AppendResult, String> {
    let replayed = replay_journal(snapshot, decision_cutoff_unix_ms)?;
    if request.event_id.trim().is_empty()
        || request.commitment_id.trim().is_empty()
        || request.idempotency_key.trim().is_empty()
        || request.event_time_unix_ms > decision_cutoff_unix_ms
    {
        return Err("append_request_identity_or_time_invalid".to_owned());
    }

    let candidate_payload_digest = if request.kind == JournalEventKind::OfferCreated {
        let contract = request
            .contract
            .as_ref()
            .ok_or_else(|| "offer_created_contract_missing".to_owned())?;
        validate_contract(contract, event_time_origin(contract, request.event_time_unix_ms)?)?;
        digest_contract(contract)?
    } else {
        if request.contract.is_some() {
            return Err("contract_must_only_appear_on_offer_created".to_owned());
        }
        replayed
            .commitments
            .get(&request.commitment_id)
            .map(|item| item.payload_digest.clone())
            .ok_or_else(|| "commitment_transition_without_offer".to_owned())?
    };

    if let Some(existing) = replayed.idempotency_receipts.get(&request.idempotency_key) {
        let existing_event = snapshot
            .events
            .get(usize::try_from(existing.sequence - 1).map_err(|_| "receipt_sequence_invalid")?)
            .ok_or_else(|| "idempotency_receipt_event_missing".to_owned())?;
        if existing_event.commitment_id != request.commitment_id
            || existing_event.payload_digest != candidate_payload_digest
            || existing_event.kind != request.kind
            || existing_event.resolved_state != request.resolved_state
        {
            return Err("idempotency_key_payload_or_kind_conflict".to_owned());
        }
        let mut receipt = existing.clone();
        receipt.idempotent_replay = true;
        return Ok(AppendResult {
            snapshot: snapshot.clone(),
            receipt,
        });
    }

    if request.expected_version != snapshot.version {
        return Err(format!(
            "version_conflict:expected_{}:actual_{}",
            request.expected_version, snapshot.version
        ));
    }
    if snapshot.events.len() >= MAX_EVENTS {
        return Err("journal_event_limit_reached".to_owned());
    }
    if snapshot.events.iter().any(|event| event.event_id == request.event_id) {
        return Err("duplicate_event_id".to_owned());
    }

    let prior = replayed.commitments.get(&request.commitment_id);
    let resulting_state = resulting_state(
        request.kind,
        prior.map(|item| item.state),
        request.resolved_state,
    )?;
    let contract = if request.kind == JournalEventKind::OfferCreated {
        request.contract.clone()
    } else {
        Some(
            prior
                .ok_or_else(|| "commitment_transition_without_offer".to_owned())?
                .contract
                .clone(),
        )
    }
    .expect("validated append path has contract");

    if !validate_evidence(
        &request.event_evidence,
        &request.commitment_id,
        &contract.site_configuration_digest,
        &request.event_id,
        request.event_time_unix_ms,
        decision_cutoff_unix_ms,
        request.kind == JournalEventKind::Unresolved,
    ) {
        return Err("append_event_evidence_invalid".to_owned());
    }

    if request.kind == JournalEventKind::Expired
        && request.event_time_unix_ms < contract.expires_at_unix_ms
    {
        return Err("commitment_expired_before_expiry_time".to_owned());
    }

    if release_requires_resource_evidence(request.kind, request.resolved_state)
        || request.kind == JournalEventKind::Reconciled
    {
        validate_resource_terminal_evidence(
            &contract.resource_allocations,
            &request.resource_terminal_evidence,
            request.event_time_unix_ms,
            decision_cutoff_unix_ms,
        )?;
    } else if !request.resource_terminal_evidence.is_empty() {
        return Err("unexpected_resource_terminal_evidence".to_owned());
    }

    let previous_event_digest = snapshot.events.last().map(|event| event.event_digest.clone());
    let sequence = snapshot
        .version
        .checked_add(1)
        .ok_or_else(|| "journal_version_overflow".to_owned())?;
    let mut event = CommitmentEvent {
        sequence,
        expected_version: snapshot.version,
        event_id: request.event_id.clone(),
        commitment_id: request.commitment_id.clone(),
        idempotency_key: request.idempotency_key.clone(),
        payload_digest: candidate_payload_digest,
        kind: request.kind,
        resolved_state: request.resolved_state,
        resulting_state,
        event_time_unix_ms: request.event_time_unix_ms,
        previous_event_digest,
        event_evidence: request.event_evidence.clone(),
        resource_terminal_evidence: request.resource_terminal_evidence.clone(),
        contract: if request.kind == JournalEventKind::OfferCreated {
            Some(contract)
        } else {
            None
        },
        event_digest: String::new(),
    };
    event.event_digest = digest_event(&event)?;

    let mut updated = snapshot.clone();
    updated.version = sequence;
    updated.events.push(event.clone());
    replay_journal(&updated, decision_cutoff_unix_ms)?;

    Ok(AppendResult {
        snapshot: updated,
        receipt: CommitmentReceipt {
            sequence,
            event_id: event.event_id,
            commitment_id: event.commitment_id,
            resulting_state: event.resulting_state,
            payload_digest: event.payload_digest,
            event_digest: event.event_digest,
            version_after: sequence,
            idempotent_replay: false,
        },
    })
}

/// A local mutex-backed adapter used to test atomic compare-and-append.
///
/// It is not durable and does not provide distributed consensus.
#[derive(Debug, Default)]
pub struct InMemoryCommitmentJournal {
    snapshot: Mutex<JournalSnapshot>,
}

impl InMemoryCommitmentJournal {
    pub fn new(snapshot: JournalSnapshot) -> Self {
        Self {
            snapshot: Mutex::new(snapshot),
        }
    }

    pub fn snapshot(&self) -> Result<JournalSnapshot, String> {
        self.snapshot
            .lock()
            .map(|snapshot| snapshot.clone())
            .map_err(|_| "journal_mutex_poisoned".to_owned())
    }

    pub fn append(
        &self,
        request: &AppendRequest,
        decision_cutoff_unix_ms: u64,
    ) -> Result<CommitmentReceipt, String> {
        let mut guard = self
            .snapshot
            .lock()
            .map_err(|_| "journal_mutex_poisoned".to_owned())?;
        let result = append_event(&guard, request, decision_cutoff_unix_ms)?;
        if !result.receipt.idempotent_replay {
            *guard = result.snapshot;
        }
        Ok(result.receipt)
    }
}

pub fn evaluate_journal_scenario(
    scenario: &JournalScenario,
) -> Result<JournalScenarioReport, String> {
    if scenario.schema_version != "0.1.0"
        || scenario.benchmark_id != "MOBILITY-COMMITMENT-JOURNAL-001"
        || scenario.decision_time_unix_ms == 0
        || scenario.claim_ceiling.trim().is_empty()
        || scenario.commands.is_empty()
        || scenario.commands.len() > 64
    {
        return Err("journal_scenario_identity_or_bounds_invalid".to_owned());
    }

    let initial = replay_journal(&scenario.initial_snapshot, scenario.decision_time_unix_ms)?;
    let initial_version = initial.version;
    let mut snapshot = scenario.initial_snapshot.clone();
    for request in &scenario.commands {
        let result = append_event(&snapshot, request, scenario.decision_time_unix_ms)?;
        snapshot = result.snapshot;
    }
    let replayed = replay_journal(&snapshot, scenario.decision_time_unix_ms)?;
    let repeated = replay_journal(&snapshot, scenario.decision_time_unix_ms)?;
    if replayed != repeated {
        return Err("journal_replay_not_deterministic".to_owned());
    }

    let mut commitment_states = BTreeMap::new();
    let mut unresolved_commitments = Vec::new();
    let mut terminal_commitments = Vec::new();
    for (commitment_id, commitment) in &replayed.commitments {
        commitment_states.insert(commitment_id.clone(), commitment.state);
        if commitment.state == CommitmentState::Unresolved {
            unresolved_commitments.push(commitment_id.clone());
        }
        if commitment.state.is_terminal() {
            terminal_commitments.push(commitment_id.clone());
        }
    }

    let last_event_digest = snapshot.events.last().map(|event| event.event_digest.clone());
    let replay_digest = sha256_hex(
        &serde_json::to_vec(&snapshot)
            .map_err(|error| format!("snapshot_serialization_failed:{error}"))?,
    );

    Ok(JournalScenarioReport {
        schema_version: scenario.schema_version.clone(),
        benchmark_id: scenario.benchmark_id.clone(),
        initial_version,
        final_version: snapshot.version,
        event_count: snapshot.events.len(),
        commitment_states,
        last_event_digest,
        replay_digest,
        unresolved_commitments,
        terminal_commitments,
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
    use std::sync::{Arc, Barrier};
    use std::thread;

    fn scenario() -> JournalScenario {
        serde_json::from_str(include_str!("../fixtures/mobility_commitment_journal_v0_1.json"))
            .expect("frozen journal scenario must parse")
    }

    fn first_request() -> AppendRequest {
        scenario().commands[0].clone()
    }

    fn append_all() -> JournalSnapshot {
        let scenario = scenario();
        let mut snapshot = scenario.initial_snapshot;
        for request in &scenario.commands {
            snapshot = append_event(&snapshot, request, scenario.decision_time_unix_ms)
                .expect("fixture append must pass")
                .snapshot;
        }
        snapshot
    }

    fn terminal_reconciliation_request(
        snapshot: &JournalSnapshot,
        decision_cutoff_unix_ms: u64,
        grade: EvidenceClass,
    ) -> AppendRequest {
        let contract = replay_journal(snapshot, decision_cutoff_unix_ms)
            .expect("snapshot replay")
            .commitments["lease-001"]
            .contract
            .clone();
        let event_time = decision_cutoff_unix_ms - 100;
        let event_id = "evt-reconcile-settled".to_owned();
        let event_evidence = EvidenceEnvelope {
            asset_id: "lease-001".to_owned(),
            configuration_digest: contract.site_configuration_digest.clone(),
            producer_id: "test-harness".to_owned(),
            method_id: "independent-reconciliation".to_owned(),
            evidence_reference: event_id.clone(),
            evidence_class: grade,
            observed_at_unix_ms: Some(event_time + 1),
            valid_from_unix_ms: Some(event_time),
            valid_until_unix_ms: Some(decision_cutoff_unix_ms + 10_000),
        };
        let resource_terminal_evidence = contract
            .resource_allocations
            .iter()
            .map(|allocation| {
                let receipt_id = format!("receipt-reconcile-{}", allocation.resource_id);
                ResourceTerminalEvidence {
                    resource_id: allocation.resource_id.clone(),
                    configuration_digest: allocation.configuration_digest.clone(),
                    receipt_id: receipt_id.clone(),
                    evidence: EvidenceEnvelope {
                        asset_id: allocation.resource_id.clone(),
                        configuration_digest: allocation.configuration_digest.clone(),
                        producer_id: "test-harness".to_owned(),
                        method_id: "independent-resource-reconciliation".to_owned(),
                        evidence_reference: receipt_id,
                        evidence_class: grade,
                        observed_at_unix_ms: Some(event_time + 1),
                        valid_from_unix_ms: Some(event_time),
                        valid_until_unix_ms: Some(decision_cutoff_unix_ms + 10_000),
                    },
                }
            })
            .collect();
        AppendRequest {
            expected_version: snapshot.version,
            event_id,
            commitment_id: "lease-001".to_owned(),
            idempotency_key: "idem-reconcile-settled".to_owned(),
            kind: JournalEventKind::Reconciled,
            resolved_state: Some(CommitmentState::Settled),
            event_time_unix_ms: event_time,
            event_evidence,
            resource_terminal_evidence,
            contract: None,
        }
    }

    #[test]
    fn fixture_replays_to_unresolved_and_is_deterministic() {
        let input = scenario();
        let report = evaluate_journal_scenario(&input).expect("fixture scenario");
        assert_eq!(report.final_version, 4);
        assert_eq!(report.event_count, 4);
        assert_eq!(
            report.commitment_states["lease-001"],
            CommitmentState::Unresolved
        );
        assert_eq!(report.unresolved_commitments, vec!["lease-001"]);
        assert!(report.terminal_commitments.is_empty());
        assert!(!report.ledger_mutation_performed);
        assert!(!report.market_authority_granted);
        assert!(!report.dispatch_authority_granted);
        assert!(!report.physical_mutation_authority_granted);
        let snapshot = append_all();
        assert_eq!(
            replay_journal(&snapshot, input.decision_time_unix_ms).expect("first replay"),
            replay_journal(&snapshot, input.decision_time_unix_ms).expect("second replay")
        );
    }

    #[test]
    fn two_writers_at_same_version_have_exactly_one_winner() {
        let input = scenario();
        let cutoff = input.decision_time_unix_ms;
        let first = first_request();
        let mut second = first.clone();
        second.event_id = "evt-competitor".to_owned();
        second.commitment_id = "lease-competitor".to_owned();
        second.idempotency_key = "idem-competitor".to_owned();
        second.event_evidence.asset_id = second.commitment_id.clone();
        second.event_evidence.evidence_reference = second.event_id.clone();
        second.expected_version = 0;

        let journal = Arc::new(InMemoryCommitmentJournal::default());
        let barrier = Arc::new(Barrier::new(3));
        let j1 = Arc::clone(&journal);
        let b1 = Arc::clone(&barrier);
        let r1 = first.clone();
        let t1 = thread::spawn(move || {
            b1.wait();
            j1.append(&r1, cutoff)
        });
        let j2 = Arc::clone(&journal);
        let b2 = Arc::clone(&barrier);
        let r2 = second;
        let t2 = thread::spawn(move || {
            b2.wait();
            j2.append(&r2, cutoff)
        });
        barrier.wait();
        let results = [t1.join().expect("writer one"), t2.join().expect("writer two")];
        assert_eq!(results.iter().filter(|result| result.is_ok()).count(), 1);
        assert_eq!(
            results
                .iter()
                .filter(|result| result.as_ref().is_err_and(|error| error.starts_with("version_conflict")))
                .count(),
            1
        );
        assert_eq!(journal.snapshot().expect("snapshot").version, 1);
    }

    #[test]
    fn identical_retry_returns_original_receipt_without_second_append() {
        let input = scenario();
        let request = first_request();
        let journal = InMemoryCommitmentJournal::default();
        let first = journal
            .append(&request, input.decision_time_unix_ms)
            .expect("first append");
        let retry = journal
            .append(&request, input.decision_time_unix_ms)
            .expect("identical retry");
        assert!(retry.idempotent_replay);
        assert_eq!(first.event_digest, retry.event_digest);
        assert_eq!(first.sequence, retry.sequence);
        assert_eq!(journal.snapshot().expect("snapshot").version, 1);
    }

    #[test]
    fn same_idempotency_key_with_changed_payload_is_rejected() {
        let input = scenario();
        let request = first_request();
        let journal = InMemoryCommitmentJournal::default();
        journal
            .append(&request, input.decision_time_unix_ms)
            .expect("first append");
        let mut changed = request;
        changed.contract.as_mut().expect("contract").resource_allocations[0]
            .energy_wh_by_slot[1] += 1;
        let result = journal.append(&changed, input.decision_time_unix_ms);
        assert_eq!(
            result.expect_err("changed payload conflict"),
            "idempotency_key_payload_or_kind_conflict"
        );
        assert_eq!(journal.snapshot().expect("snapshot").version, 1);
    }

    #[test]
    fn unresolved_cannot_be_released_without_reconciliation() {
        let input = scenario();
        let snapshot = append_all();
        let mut request = terminal_reconciliation_request(
            &snapshot,
            input.decision_time_unix_ms,
            EvidenceClass::Synthetic,
        );
        request.resource_terminal_evidence.clear();
        let error = append_event(&snapshot, &request, input.decision_time_unix_ms)
            .expect_err("terminal reconciliation requires every resource receipt");
        assert_eq!(error, "terminal_resource_evidence_coverage_incomplete");
    }

    #[test]
    fn synthetic_resource_evidence_cannot_release_unresolved_reservation() {
        let input = scenario();
        let snapshot = append_all();
        let request = terminal_reconciliation_request(
            &snapshot,
            input.decision_time_unix_ms,
            EvidenceClass::Synthetic,
        );
        let error = append_event(&snapshot, &request, input.decision_time_unix_ms)
            .expect_err("synthetic release evidence forbidden");
        assert!(error.starts_with("terminal_resource_evidence_invalid"));
    }

    #[test]
    fn independent_reconciliation_can_release_unresolved_lease() {
        let input = scenario();
        let snapshot = append_all();
        let request = terminal_reconciliation_request(
            &snapshot,
            input.decision_time_unix_ms,
            EvidenceClass::IndependentTest,
        );
        let result = append_event(&snapshot, &request, input.decision_time_unix_ms)
            .expect("independent reconciliation");
        let replay = replay_journal(&result.snapshot, input.decision_time_unix_ms)
            .expect("replay after reconciliation");
        assert_eq!(replay.commitments["lease-001"].state, CommitmentState::Settled);
        assert_eq!(result.snapshot.version, 5);
    }

    #[test]
    fn direct_settlement_from_unresolved_is_rejected() {
        let input = scenario();
        let snapshot = append_all();
        let mut request = terminal_reconciliation_request(
            &snapshot,
            input.decision_time_unix_ms,
            EvidenceClass::IndependentTest,
        );
        request.kind = JournalEventKind::Settled;
        request.resolved_state = None;
        let error = append_event(&snapshot, &request, input.decision_time_unix_ms)
            .expect_err("unresolved must be reconciled explicitly");
        assert_eq!(error, "invalid_commitment_state_transition");
    }

    #[test]
    fn stale_version_writer_is_rejected() {
        let input = scenario();
        let request = first_request();
        let mut snapshot = JournalSnapshot::default();
        snapshot = append_event(&snapshot, &request, input.decision_time_unix_ms)
            .expect("first append")
            .snapshot;
        let mut stale = first_request();
        stale.event_id = "evt-stale-writer".to_owned();
        stale.idempotency_key = "idem-stale-writer".to_owned();
        stale.event_evidence.evidence_reference = stale.event_id.clone();
        let error = append_event(&snapshot, &stale, input.decision_time_unix_ms)
            .expect_err("a distinct writer at the old version must lose CAS");
        assert!(error.starts_with("version_conflict"));
    }

    #[test]
    fn broken_event_digest_is_rejected_on_replay() {
        let input = scenario();
        let mut snapshot = append_all();
        snapshot.events[0].event_digest = "sha256:".to_owned() + &"0".repeat(64);
        let error = replay_journal(&snapshot, input.decision_time_unix_ms)
            .expect_err("changed event digest");
        assert!(error.starts_with("journal_event_digest_mismatch"));
    }

    #[test]
    fn previous_digest_link_mutation_is_rejected_on_replay() {
        let input = scenario();
        let mut snapshot = append_all();
        snapshot.events[1].previous_event_digest = Some("sha256:".to_owned() + &"0".repeat(64));
        let error = replay_journal(&snapshot, input.decision_time_unix_ms)
            .expect_err("broken event link");
        assert!(error.starts_with("journal_event_order_identity_or_link_invalid"));
    }

    #[test]
    fn event_order_permutation_is_rejected() {
        let input = scenario();
        let mut snapshot = append_all();
        snapshot.events.swap(1, 2);
        let error = replay_journal(&snapshot, input.decision_time_unix_ms)
            .expect_err("reordered append-only events");
        assert!(error.starts_with("journal_event_order_identity_or_link_invalid"));
    }
}
