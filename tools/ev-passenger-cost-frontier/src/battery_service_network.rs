// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Exact one-epoch battery-pack allocation and capability screen.
//!
//! This v0.1 slice assigns each compatible qualified pack to at most one
//! simultaneous mission. It is not a charging scheduler, battery model,
//! flight authorization service, or return-to-service authority.

use std::collections::BTreeSet;

use serde::{Deserialize, Serialize};

const MAX_EXACT_ITEMS: usize = 6;
const CLAIM_CEILING: &str = "Exact synthetic one-epoch pack allocation only; not battery safety qualification, measured capability, charging-network optimization, dispatch authorization, return-to-service authority, airworthiness or commercial performance.";

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackState {
    Available,
    Installed,
    Reserved,
    Depleted,
    Charging,
    Cooling,
    Balancing,
    Inspection,
    Repair,
    PostTest,
    Quarantined,
    InTransit,
    Retired,
    Unknown,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceClass {
    Synthetic,
    Measured,
    IndependentTest,
    Unknown,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceEnvelope {
    pub asset_id: String,
    pub configuration_digest: String,
    pub producer_id: String,
    pub method_id: String,
    pub evidence_reference: String,
    pub evidence_class: EvidenceClass,
    pub observed_at_unix_ms: Option<u64>,
    pub valid_from_unix_ms: Option<u64>,
    pub valid_until_unix_ms: Option<u64>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackStateEvidence {
    pub evidence: EvidenceEnvelope,
    pub value: PackState,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricEvidence {
    pub evidence: EvidenceEnvelope,
    pub conservative_lower_bound: Option<u32>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BooleanEvidence {
    pub evidence: EvidenceEnvelope,
    pub value: Option<bool>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ThermalEvidence {
    pub evidence: EvidenceEnvelope,
    pub conservative_upper_bound_c: Option<i16>,
    pub qualified_max_temperature_c: Option<i16>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryPack {
    pub pack_id: String,
    pub configuration_id: String,
    pub configuration_digest: String,
    pub state_evidence: PackStateEvidence,
    pub identity_verification: BooleanEvidence,
    pub custody_resolution: BooleanEvidence,
    pub post_test: BooleanEvidence,
    pub usable_energy: MetricEvidence,
    pub peak_power: MetricEvidence,
    pub thermal: ThermalEvidence,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PackTransitionDisposition {
    Confirmed,
    Timeout,
    OutcomeUnknown,
    Reconciled,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackTransitionEvent {
    pub event_id: String,
    pub idempotency_key: String,
    pub operation_id: String,
    pub sequence: u64,
    pub event_time_unix_ms: u64,
    pub from_state: PackState,
    pub proposed_to_state: PackState,
    pub disposition: PackTransitionDisposition,
    pub evidence: EvidenceEnvelope,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PackStateJournalInput {
    pub pack_id: String,
    pub configuration_digest: String,
    pub replay_cutoff_unix_ms: u64,
    pub initial_state: PackStateEvidence,
    pub events: Vec<PackTransitionEvent>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PackStateJournalReport {
    pub pack_id: String,
    pub final_state: PackState,
    pub final_sequence: u64,
    pub reconciliation_required: bool,
    pub last_uncertain_event_id: Option<String>,
    pub accepted_event_ids: Vec<String>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MissionDemand {
    pub mission_id: String,
    pub configuration_id: String,
    pub configuration_digest: String,
    /// Includes the required operational and contingency reserve.
    pub required_energy_with_reserve_wh: u32,
    pub required_peak_power_w: u32,
    pub maximum_pack_temperature_c: i16,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BatteryServiceNetworkInput {
    pub schema_version: String,
    pub benchmark_id: String,
    pub dispatch_epoch_id: String,
    pub dispatch_time_unix_ms: u64,
    pub packs: Vec<BatteryPack>,
    pub missions: Vec<MissionDemand>,
    pub claim_ceiling: String,
}

#[derive(Debug, Clone, Copy, Deserialize, Serialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum EligibilityStatus {
    Eligible,
    Blocked,
    Indeterminate,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PairEvaluation {
    pub mission_id: String,
    pub pack_id: String,
    pub status: EligibilityStatus,
    pub energy_margin_wh: Option<u32>,
    pub findings: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct PackAssignment {
    pub mission_id: String,
    pub pack_id: String,
    pub energy_margin_wh: u32,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BatteryServiceNetworkReport {
    pub schema_version: String,
    pub benchmark_id: String,
    pub dispatch_epoch_id: String,
    pub dispatch_time_unix_ms: u64,
    pub pack_count: usize,
    pub mission_count: usize,
    pub search_nodes: usize,
    pub serviceable_mission_count: usize,
    pub assignments: Vec<PackAssignment>,
    pub unserved_missions: Vec<String>,
    pub unused_packs: Vec<String>,
    pub pair_evaluations: Vec<PairEvaluation>,
    pub status: String,
    pub all_pack_evidence_synthetic: bool,
    pub dispatch_authority_granted: bool,
    pub return_to_service_authority_granted: bool,
    pub claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct SearchBest {
    served: usize,
    total_energy_margin_wh: u64,
    selected_pack_indices: Vec<Option<usize>>,
}

struct AssignmentSearch<'a> {
    candidates: &'a [Vec<(usize, u32)>],
    packs: &'a [BatteryPack],
    nodes: usize,
    best: Option<SearchBest>,
}

impl AssignmentSearch<'_> {
    fn search(
        &mut self,
        mission_index: usize,
        selected: &mut Vec<Option<usize>>,
        used: &mut BTreeSet<usize>,
        served: usize,
        total_energy_margin_wh: u64,
    ) {
        self.nodes += 1;
        if mission_index == self.candidates.len() {
            let candidate = SearchBest {
                served,
                total_energy_margin_wh,
                selected_pack_indices: selected.clone(),
            };
            if self
                .best
                .as_ref()
                .is_none_or(|current| is_better(&candidate, current, self.packs))
            {
                self.best = Some(candidate);
            }
            return;
        }

        for candidate_index in 0..self.candidates[mission_index].len() {
            let (pack_index, energy_margin_wh) =
                self.candidates[mission_index][candidate_index];
            if used.insert(pack_index) {
                selected.push(Some(pack_index));
                self.search(
                    mission_index + 1,
                    selected,
                    used,
                    served + 1,
                    total_energy_margin_wh + u64::from(energy_margin_wh),
                );
                selected.pop();
                used.remove(&pack_index);
            }
        }

        selected.push(None);
        self.search(
            mission_index + 1,
            selected,
            used,
            served,
            total_energy_margin_wh,
        );
        selected.pop();
    }
}

fn add_finding(
    findings: &mut Vec<String>,
    status: &mut EligibilityStatus,
    next: EligibilityStatus,
    reason: &str,
) {
    findings.push(reason.to_string());
    if *status == EligibilityStatus::Blocked {
        return;
    }
    if next == EligibilityStatus::Blocked {
        *status = EligibilityStatus::Blocked;
    } else if next == EligibilityStatus::Indeterminate
        && *status == EligibilityStatus::Eligible
    {
        *status = EligibilityStatus::Indeterminate;
    }
}

fn evidence_is_admissible_for_identity(
    evidence: &EvidenceEnvelope,
    asset_id: &str,
    configuration_digest: &str,
    decision_time_unix_ms: u64,
    findings: &mut Vec<String>,
    status: &mut EligibilityStatus,
    name: &str,
) -> bool {
    let mut complete = true;
    for (field, value) in [
        ("asset_id", evidence.asset_id.as_str()),
        ("configuration_digest", evidence.configuration_digest.as_str()),
        ("producer_id", evidence.producer_id.as_str()),
        ("method_id", evidence.method_id.as_str()),
        ("evidence_reference", evidence.evidence_reference.as_str()),
    ] {
        if value.trim().is_empty() {
            add_finding(
                findings,
                status,
                EligibilityStatus::Indeterminate,
                &format!("{name}_{field}_missing"),
            );
            complete = false;
        }
    }

    if !complete {
        return false;
    }
    if evidence.asset_id != asset_id {
        add_finding(
            findings,
            status,
            EligibilityStatus::Blocked,
            &format!("{name}_asset_identity_mismatch"),
        );
        return false;
    }
    if evidence.configuration_digest != configuration_digest {
        add_finding(
            findings,
            status,
            EligibilityStatus::Blocked,
            &format!("{name}_configuration_digest_mismatch"),
        );
        return false;
    }
    if evidence.evidence_class == EvidenceClass::Unknown {
        add_finding(
            findings,
            status,
            EligibilityStatus::Indeterminate,
            &format!("{name}_evidence_class_unknown"),
        );
        return false;
    }

    let (observed_at, valid_from, valid_until) = match (
        evidence.observed_at_unix_ms,
        evidence.valid_from_unix_ms,
        evidence.valid_until_unix_ms,
    ) {
        (Some(observed_at), Some(valid_from), Some(valid_until)) => {
            (observed_at, valid_from, valid_until)
        }
        _ => {
            add_finding(
                findings,
                status,
                EligibilityStatus::Indeterminate,
                &format!("{name}_evidence_time_window_incomplete"),
            );
            return false;
        }
    };

    if valid_from > valid_until || observed_at < valid_from || observed_at > valid_until {
        add_finding(
            findings,
            status,
            EligibilityStatus::Blocked,
            &format!("{name}_evidence_time_window_malformed"),
        );
        return false;
    }
    if observed_at > decision_time_unix_ms {
        add_finding(
            findings,
            status,
            EligibilityStatus::Blocked,
            &format!("{name}_future_observation_not_admissible"),
        );
        return false;
    }
    if decision_time_unix_ms < valid_from || decision_time_unix_ms > valid_until {
        add_finding(
            findings,
            status,
            EligibilityStatus::Indeterminate,
            &format!("{name}_evidence_outside_validity_window"),
        );
        return false;
    }
    true
}

fn evidence_is_admissible(
    evidence: &EvidenceEnvelope,
    pack: &BatteryPack,
    decision_time_unix_ms: u64,
    findings: &mut Vec<String>,
    status: &mut EligibilityStatus,
    name: &str,
) -> bool {
    evidence_is_admissible_for_identity(
        evidence,
        &pack.pack_id,
        &pack.configuration_digest,
        decision_time_unix_ms,
        findings,
        status,
        name,
    )
}

fn evaluate_boolean_evidence(
    evidence: &BooleanEvidence,
    pack: &BatteryPack,
    dispatch_time_unix_ms: u64,
    name: &str,
    findings: &mut Vec<String>,
    status: &mut EligibilityStatus,
) {
    if !evidence_is_admissible(
        &evidence.evidence,
        pack,
        dispatch_time_unix_ms,
        findings,
        status,
        name,
    ) {
        return;
    }
    match evidence.value {
        Some(true) => {}
        Some(false) => add_finding(
            findings,
            status,
            EligibilityStatus::Blocked,
            &format!("{name}_not_verified_or_failed"),
        ),
        None => add_finding(
            findings,
            status,
            EligibilityStatus::Indeterminate,
            &format!("{name}_result_unknown"),
        ),
    }
}

fn evaluate_pair(
    mission: &MissionDemand,
    pack: &BatteryPack,
    dispatch_time_unix_ms: u64,
) -> PairEvaluation {
    let mut findings = Vec::new();
    let mut status = EligibilityStatus::Eligible;

    let _state_evidence_admissible = evidence_is_admissible(
        &pack.state_evidence.evidence,
        pack,
        dispatch_time_unix_ms,
        &mut findings,
        &mut status,
        "pack_state",
    );

    match pack.state_evidence.value {
        PackState::Available => {}
        PackState::Unknown => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Indeterminate,
            "pack_state_unknown",
        ),
        _ => add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Blocked,
            "pack_not_available_for_dispatch_epoch",
        ),
    }

    evaluate_boolean_evidence(
        &pack.identity_verification,
        pack,
        dispatch_time_unix_ms,
        "pack_identity",
        &mut findings,
        &mut status,
    );
    evaluate_boolean_evidence(
        &pack.custody_resolution,
        pack,
        dispatch_time_unix_ms,
        "pack_custody",
        &mut findings,
        &mut status,
    );
    evaluate_boolean_evidence(
        &pack.post_test,
        pack,
        dispatch_time_unix_ms,
        "post_test",
        &mut findings,
        &mut status,
    );

    if pack.configuration_id != mission.configuration_id
        || pack.configuration_digest != mission.configuration_digest
    {
        add_finding(
            &mut findings,
            &mut status,
            EligibilityStatus::Blocked,
            "configuration_mismatch",
        );
    }

    let energy_evidence_admissible = evidence_is_admissible(
        &pack.usable_energy.evidence,
        pack,
        dispatch_time_unix_ms,
        &mut findings,
        &mut status,
        "energy",
    );
    let power_evidence_admissible = evidence_is_admissible(
        &pack.peak_power.evidence,
        pack,
        dispatch_time_unix_ms,
        &mut findings,
        &mut status,
        "peak_power",
    );
    let thermal_evidence_admissible = evidence_is_admissible(
        &pack.thermal.evidence,
        pack,
        dispatch_time_unix_ms,
        &mut findings,
        &mut status,
        "thermal",
    );

    let energy_margin_wh = if energy_evidence_admissible {
        match pack.usable_energy.conservative_lower_bound {
            Some(value) if value >= mission.required_energy_with_reserve_wh => {
                Some(value - mission.required_energy_with_reserve_wh)
            }
            Some(_) => {
                add_finding(
                    &mut findings,
                    &mut status,
                    EligibilityStatus::Blocked,
                    "usable_energy_below_mission_plus_reserve",
                );
                None
            }
            None => {
                add_finding(
                    &mut findings,
                    &mut status,
                    EligibilityStatus::Indeterminate,
                    "usable_energy_value_missing",
                );
                None
            }
        }
    } else {
        None
    };

    if power_evidence_admissible {
        match pack.peak_power.conservative_lower_bound {
            Some(value) if value >= mission.required_peak_power_w => {}
            Some(_) => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Blocked,
                "peak_power_below_mission_requirement",
            ),
            None => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Indeterminate,
                "peak_power_value_missing",
            ),
        }
    }

    if thermal_evidence_admissible {
        match (
            pack.thermal.conservative_upper_bound_c,
            pack.thermal.qualified_max_temperature_c,
        ) {
            (Some(current), Some(qualified_max))
                if current <= mission.maximum_pack_temperature_c && current <= qualified_max => {}
            (Some(_), Some(_)) => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Blocked,
                "thermal_envelope_exceeded",
            ),
            _ => add_finding(
                &mut findings,
                &mut status,
                EligibilityStatus::Indeterminate,
                "thermal_values_missing",
            ),
        }
    }

    findings.sort();
    findings.dedup();
    PairEvaluation {
        mission_id: mission.mission_id.clone(),
        pack_id: pack.pack_id.clone(),
        status,
        energy_margin_wh: if status == EligibilityStatus::Eligible {
            energy_margin_wh
        } else {
            None
        },
        findings,
    }
}

fn transition_is_allowed(from: PackState, to: PackState) -> bool {
    use PackState::*;
    matches!(
        (from, to),
        (Available, Reserved)
            | (Reserved, Available)
            | (Reserved, Installed)
            | (Reserved, Quarantined)
            | (Installed, Depleted)
            | (Installed, Quarantined)
            | (Depleted, InTransit)
            | (Depleted, Cooling)
            | (Depleted, Inspection)
            | (InTransit, Cooling)
            | (InTransit, Inspection)
            | (InTransit, Quarantined)
            | (Cooling, Charging)
            | (Cooling, Inspection)
            | (Cooling, Quarantined)
            | (Charging, Balancing)
            | (Charging, Cooling)
            | (Charging, Quarantined)
            | (Balancing, Inspection)
            | (Balancing, Charging)
            | (Balancing, Quarantined)
            | (Inspection, Repair)
            | (Inspection, PostTest)
            | (Inspection, Quarantined)
            | (Inspection, Retired)
            | (Repair, PostTest)
            | (Repair, Quarantined)
            | (Repair, Retired)
            | (PostTest, Available)
            | (PostTest, Quarantined)
            | (PostTest, Retired)
            | (Quarantined, Inspection)
            | (Quarantined, Retired)
    )
}

pub fn replay_pack_state_journal(
    input: &PackStateJournalInput,
) -> Result<PackStateJournalReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.pack_id.trim().is_empty()
        || input.configuration_digest.trim().is_empty()
        || input.replay_cutoff_unix_ms == 0
    {
        errors.push("journal_identity_or_replay_cutoff_missing".to_string());
    }

    let initial_observation_time = input.initial_state.evidence.observed_at_unix_ms;
    if initial_observation_time.is_none()
        || initial_observation_time.is_some_and(|time| time > input.replay_cutoff_unix_ms)
    {
        errors.push("initial_state_observation_time_missing_or_after_cutoff".to_string());
    }

    let mut findings = Vec::new();
    let mut initial_status = EligibilityStatus::Eligible;
    if let Some(initial_time) = initial_observation_time {
        if !evidence_is_admissible_for_identity(
            &input.initial_state.evidence,
            &input.pack_id,
            &input.configuration_digest,
            initial_time,
            &mut findings,
            &mut initial_status,
            "initial_pack_state",
        ) {
            errors.push("initial_pack_state_evidence_not_admissible".to_string());
        }
    } else {
        errors.push("initial_pack_state_evidence_time_missing".to_string());
    }

    if input.initial_state.value == PackState::Unknown {
        findings.push("initial_pack_state_unknown_reconciliation_required".to_string());
    }

    let mut current_state = input.initial_state.value;
    let mut current_state_evidence = input.initial_state.evidence.clone();
    let mut current_sequence = 0_u64;
    let mut last_event_time = initial_observation_time.unwrap_or_default();
    let mut reconciliation_required = current_state == PackState::Unknown;
    let mut last_uncertain_event_id = None;
    let mut accepted_event_ids = Vec::new();
    let mut event_ids = BTreeSet::new();
    let mut idempotency_keys = BTreeSet::new();

    for event in &input.events {
        if event.event_id.trim().is_empty()
            || event.idempotency_key.trim().is_empty()
            || event.operation_id.trim().is_empty()
        {
            errors.push("event_identity_idempotency_or_operation_id_missing".to_string());
            break;
        }
        if !event_ids.insert(event.event_id.as_str()) {
            errors.push(format!("duplicate_event_id:{}", event.event_id));
            break;
        }
        if !idempotency_keys.insert(event.idempotency_key.as_str()) {
            errors.push(format!("duplicate_idempotency_key:{}", event.idempotency_key));
            break;
        }
        let Some(expected_sequence) = current_sequence.checked_add(1) else {
            errors.push("event_sequence_overflow".to_string());
            break;
        };
        if event.sequence != expected_sequence {
            errors.push(format!("non_contiguous_event_sequence:{}", event.event_id));
            break;
        }
        if event.event_time_unix_ms < last_event_time
            || event.event_time_unix_ms > input.replay_cutoff_unix_ms
        {
            errors.push(format!("event_time_non_monotonic_or_after_cutoff:{}", event.event_id));
            break;
        }

        let mut event_status = EligibilityStatus::Eligible;
        if !evidence_is_admissible_for_identity(
            &event.evidence,
            &input.pack_id,
            &input.configuration_digest,
            event.event_time_unix_ms,
            &mut findings,
            &mut event_status,
            "pack_transition_event",
        ) {
            errors.push(format!("event_evidence_not_admissible:{}", event.event_id));
            break;
        }
        if event.from_state != current_state {
            errors.push(format!("event_from_state_mismatch:{}", event.event_id));
            break;
        }

        match event.disposition {
            PackTransitionDisposition::Confirmed => {
                if reconciliation_required {
                    errors.push(format!("normal_event_blocked_pending_reconciliation:{}", event.event_id));
                    break;
                }
                if !transition_is_allowed(current_state, event.proposed_to_state) {
                    errors.push(format!("illegal_pack_state_transition:{}", event.event_id));
                    break;
                }
                current_state = event.proposed_to_state;
                current_state_evidence = event.evidence.clone();
            }
            PackTransitionDisposition::Timeout | PackTransitionDisposition::OutcomeUnknown => {
                if reconciliation_required {
                    errors.push(format!("second_unknown_operation_blocked:{}", event.event_id));
                    break;
                }
                reconciliation_required = true;
                last_uncertain_event_id = Some(event.event_id.clone());
                findings.push(format!("operation_outcome_unknown:{}", event.operation_id));
            }
            PackTransitionDisposition::Reconciled => {
                if !reconciliation_required {
                    errors.push(format!("unexpected_reconciliation_without_unknown_outcome:{}", event.event_id));
                    break;
                }
                if !matches!(
                    event.evidence.evidence_class,
                    EvidenceClass::Measured | EvidenceClass::IndependentTest
                ) {
                    errors.push(format!("reconciliation_requires_measured_or_independent_evidence:{}", event.event_id));
                    break;
                }
                if event.proposed_to_state == PackState::Unknown {
                    errors.push(format!("reconciliation_state_remains_unknown:{}", event.event_id));
                    break;
                }
                current_state = event.proposed_to_state;
                current_state_evidence = event.evidence.clone();
                reconciliation_required = false;
                last_uncertain_event_id = None;
                findings.push(format!("physical_state_reconciled:{}", event.event_id));
            }
        }

        current_sequence = event.sequence;
        last_event_time = event.event_time_unix_ms;
        accepted_event_ids.push(event.event_id.clone());
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    if !reconciliation_required {
        let latest_evidence_covers_cutoff = matches!(
            (
                current_state_evidence.observed_at_unix_ms,
                current_state_evidence.valid_from_unix_ms,
                current_state_evidence.valid_until_unix_ms,
            ),
            (Some(observed_at), Some(valid_from), Some(valid_until))
                if observed_at <= input.replay_cutoff_unix_ms
                    && valid_from <= input.replay_cutoff_unix_ms
                    && input.replay_cutoff_unix_ms <= valid_until
        );
        if !latest_evidence_covers_cutoff {
            reconciliation_required = true;
            findings.push("last_confirmed_pack_state_evidence_expired_at_replay_cutoff".to_string());
        }
    }

    Ok(PackStateJournalReport {
        pack_id: input.pack_id.clone(),
        final_state: current_state,
        final_sequence: current_sequence,
        reconciliation_required,
        last_uncertain_event_id,
        accepted_event_ids,
        findings,
    })
}

fn assignment_key(
    selected: &[Option<usize>],
    packs: &[BatteryPack],
) -> Vec<String> {
    selected
        .iter()
        .map(|index| match index {
            Some(index) => format!("0:{}", packs[*index].pack_id),
            None => "1:".to_string(),
        })
        .collect()
}

fn is_better(candidate: &SearchBest, current: &SearchBest, packs: &[BatteryPack]) -> bool {
    if candidate.served != current.served {
        return candidate.served > current.served;
    }
    if candidate.total_energy_margin_wh != current.total_energy_margin_wh {
        return candidate.total_energy_margin_wh < current.total_energy_margin_wh;
    }
    assignment_key(&candidate.selected_pack_indices, packs)
        < assignment_key(&current.selected_pack_indices, packs)
}


pub fn evaluate_battery_service_network(
    input: &BatteryServiceNetworkInput,
) -> Result<BatteryServiceNetworkReport, Vec<String>> {
    let mut errors = Vec::new();
    if input.schema_version != "0.1.0"
        || input.benchmark_id != "BATTERY-SERVICE-NETWORK-001"
        || input.claim_ceiling.trim().is_empty()
        || input.dispatch_epoch_id.trim().is_empty()
        || input.dispatch_time_unix_ms == 0
    {
        errors.push("unsupported_identity_or_missing_dispatch_epoch".to_string());
    }
    if input.packs.is_empty()
        || input.missions.is_empty()
        || input.packs.len() > MAX_EXACT_ITEMS
        || input.missions.len() > MAX_EXACT_ITEMS
    {
        errors.push(format!("exact_v0_1_limit_is_1_to_{MAX_EXACT_ITEMS}_packs_and_missions"));
    }

    let mut pack_ids = BTreeSet::new();
    for pack in &input.packs {
        if pack.pack_id.trim().is_empty()
            || pack.configuration_id.trim().is_empty()
            || pack.configuration_digest.trim().is_empty()
            || !pack_ids.insert(pack.pack_id.as_str())
        {
            errors.push("pack_identity_empty_or_duplicate".to_string());
        }
    }
    let mut mission_ids = BTreeSet::new();
    for mission in &input.missions {
        if mission.mission_id.trim().is_empty()
            || mission.configuration_id.trim().is_empty()
            || mission.configuration_digest.trim().is_empty()
            || mission.required_energy_with_reserve_wh == 0
            || mission.required_peak_power_w == 0
            || !mission_ids.insert(mission.mission_id.as_str())
        {
            errors.push("mission_identity_or_requirement_invalid".to_string());
        }
    }
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut packs = input.packs.clone();
    packs.sort_by(|left, right| left.pack_id.cmp(&right.pack_id));
    let mut missions = input.missions.clone();
    missions.sort_by(|left, right| left.mission_id.cmp(&right.mission_id));

    let mut pair_evaluations = Vec::new();
    let mut candidates = Vec::new();
    for mission in &missions {
        let mut mission_candidates = Vec::new();
        for (pack_index, pack) in packs.iter().enumerate() {
            let evaluation =
                evaluate_pair(mission, pack, input.dispatch_time_unix_ms);
            if evaluation.status == EligibilityStatus::Eligible {
                mission_candidates.push((
                    pack_index,
                    evaluation.energy_margin_wh.unwrap_or_default(),
                ));
            }
            pair_evaluations.push(evaluation);
        }
        mission_candidates.sort_by(|left, right| {
            left.1
                .cmp(&right.1)
                .then_with(|| packs[left.0].pack_id.cmp(&packs[right.0].pack_id))
        });
        candidates.push(mission_candidates);
    }

    let mut search = AssignmentSearch {
        candidates: &candidates,
        packs: &packs,
        nodes: 0,
        best: None,
    };
    search.search(0, &mut Vec::new(), &mut BTreeSet::new(), 0, 0);
    let search_nodes = search.nodes;
    let best = search
        .best
        .expect("validated non-empty mission set always has an assignment");

    let mut assignments = Vec::new();
    let mut unserved_missions = Vec::new();
    let mut used_pack_indices = BTreeSet::new();
    for (mission_index, selected_pack) in best.selected_pack_indices.iter().enumerate() {
        match selected_pack {
            Some(pack_index) => {
                used_pack_indices.insert(*pack_index);
                let evaluation = pair_evaluations
                    .iter()
                    .find(|item| {
                        item.mission_id == missions[mission_index].mission_id
                            && item.pack_id == packs[*pack_index].pack_id
                    })
                    .expect("selected pair must have an evaluation");
                assignments.push(PackAssignment {
                    mission_id: missions[mission_index].mission_id.clone(),
                    pack_id: packs[*pack_index].pack_id.clone(),
                    energy_margin_wh: evaluation.energy_margin_wh.unwrap_or_default(),
                });
            }
            None => unserved_missions.push(missions[mission_index].mission_id.clone()),
        }
    }

    let unused_packs = packs
        .iter()
        .enumerate()
        .filter(|(index, _)| !used_pack_indices.contains(index))
        .map(|(_, pack)| pack.pack_id.clone())
        .collect();
    pair_evaluations.sort_by(|left, right| {
        left.mission_id
            .cmp(&right.mission_id)
            .then_with(|| left.pack_id.cmp(&right.pack_id))
    });

    let status = if best.served == missions.len() {
        "all_missions_assigned"
    } else if best.served > 0 {
        "partial_service"
    } else {
        "no_serviceable_missions"
    };

    Ok(BatteryServiceNetworkReport {
        schema_version: "0.1.0".to_string(),
        benchmark_id: "BATTERY-SERVICE-NETWORK-001".to_string(),
        dispatch_epoch_id: input.dispatch_epoch_id.clone(),
        dispatch_time_unix_ms: input.dispatch_time_unix_ms,
        pack_count: packs.len(),
        mission_count: missions.len(),
        search_nodes,
        serviceable_mission_count: best.served,
        assignments,
        unserved_missions,
        unused_packs,
        pair_evaluations,
        status: status.to_string(),
        all_pack_evidence_synthetic: packs.iter().all(|pack| {
            pack.state_evidence.evidence.evidence_class == EvidenceClass::Synthetic
                && pack.identity_verification.evidence.evidence_class == EvidenceClass::Synthetic
                && pack.custody_resolution.evidence.evidence_class == EvidenceClass::Synthetic
                && pack.post_test.evidence.evidence_class == EvidenceClass::Synthetic
                && pack.usable_energy.evidence.evidence_class == EvidenceClass::Synthetic
                && pack.peak_power.evidence.evidence_class == EvidenceClass::Synthetic
                && pack.thermal.evidence.evidence_class == EvidenceClass::Synthetic
        }),
        dispatch_authority_granted: false,
        return_to_service_authority_granted: false,
        claim_ceiling: CLAIM_CEILING,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const DISPATCH_TIME_MS: u64 = 1_795_219_200_000;

    fn evidence(asset_id: &str, configuration_digest: &str, suffix: &str) -> EvidenceEnvelope {
        EvidenceEnvelope {
            asset_id: asset_id.to_string(),
            configuration_digest: configuration_digest.to_string(),
            producer_id: "urn:synthetic:producer:testbench".to_string(),
            method_id: "urn:synthetic:method:reference-test".to_string(),
            evidence_reference: format!("urn:synthetic:evidence:{suffix}"),
            evidence_class: EvidenceClass::Synthetic,
            observed_at_unix_ms: Some(DISPATCH_TIME_MS - 1_000),
            valid_from_unix_ms: Some(DISPATCH_TIME_MS - 10_000),
            valid_until_unix_ms: Some(DISPATCH_TIME_MS + 10_000),
        }
    }

    fn metric(
        asset_id: &str,
        configuration_digest: &str,
        suffix: &str,
        value: Option<u32>,
    ) -> MetricEvidence {
        MetricEvidence {
            evidence: evidence(asset_id, configuration_digest, suffix),
            conservative_lower_bound: value,
        }
    }

    fn boolean(
        asset_id: &str,
        configuration_digest: &str,
        suffix: &str,
        value: Option<bool>,
    ) -> BooleanEvidence {
        BooleanEvidence {
            evidence: evidence(asset_id, configuration_digest, suffix),
            value,
        }
    }

    fn pack(
        id: &str,
        configuration_id: &str,
        energy_wh: Option<u32>,
        peak_power_w: Option<u32>,
    ) -> BatteryPack {
        let configuration_digest = format!("urn:synthetic:configuration:{configuration_id}");
        BatteryPack {
            pack_id: id.to_string(),
            configuration_id: configuration_id.to_string(),
            configuration_digest: configuration_digest.clone(),
            state_evidence: PackStateEvidence {
                evidence: evidence(id, &configuration_digest, "state"),
                value: PackState::Available,
            },
            identity_verification: boolean(id, &configuration_digest, "identity", Some(true)),
            custody_resolution: boolean(id, &configuration_digest, "custody", Some(true)),
            post_test: boolean(id, &configuration_digest, "post-test", Some(true)),
            usable_energy: metric(id, &configuration_digest, "energy", energy_wh),
            peak_power: metric(id, &configuration_digest, "peak-power", peak_power_w),
            thermal: ThermalEvidence {
                evidence: evidence(id, &configuration_digest, "thermal"),
                conservative_upper_bound_c: Some(25),
                qualified_max_temperature_c: Some(60),
            },
        }
    }

    fn mission(
        id: &str,
        configuration_id: &str,
        energy_wh: u32,
        peak_power_w: u32,
    ) -> MissionDemand {
        MissionDemand {
            mission_id: id.to_string(),
            configuration_id: configuration_id.to_string(),
            configuration_digest: format!("urn:synthetic:configuration:{configuration_id}"),
            required_energy_with_reserve_wh: energy_wh,
            required_peak_power_w: peak_power_w,
            maximum_pack_temperature_c: 50,
        }
    }

    fn input(
        packs: Vec<BatteryPack>,
        missions: Vec<MissionDemand>,
    ) -> BatteryServiceNetworkInput {
        BatteryServiceNetworkInput {
            schema_version: "0.1.0".to_string(),
            benchmark_id: "BATTERY-SERVICE-NETWORK-001".to_string(),
            dispatch_epoch_id: "epoch-1".to_string(),
            dispatch_time_unix_ms: DISPATCH_TIME_MS,
            packs,
            missions,
            claim_ceiling: "synthetic exact allocation only".to_string(),
        }
    }

    fn pair_status(pack: &BatteryPack, mission: &MissionDemand) -> EligibilityStatus {
        evaluate_pair(mission, pack, DISPATCH_TIME_MS).status
    }

    #[test]
    fn energy_pass_does_not_override_peak_power_failure() {
        let battery = pack("p1", "uav-a", Some(2_000), Some(900));
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        let result = evaluate_pair(&demand, &battery, DISPATCH_TIME_MS);
        assert_eq!(result.status, EligibilityStatus::Blocked);
        assert!(result
            .findings
            .iter()
            .any(|finding| finding == "peak_power_below_mission_requirement"));
    }

    #[test]
    fn unknown_peak_power_is_indeterminate_not_a_pass() {
        let battery = pack("p1", "uav-a", Some(2_000), None);
        let demand = mission("m1", "uav-a", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Indeterminate);
    }

    #[test]
    fn expired_evidence_window_is_indeterminate() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.usable_energy.evidence.valid_until_unix_ms =
            Some(DISPATCH_TIME_MS - 1);
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Indeterminate
        );
    }

    #[test]
    fn future_observation_is_blocked_to_prevent_information_leakage() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.peak_power.evidence.observed_at_unix_ms = Some(DISPATCH_TIME_MS + 1);
        battery.peak_power.evidence.valid_until_unix_ms =
            Some(DISPATCH_TIME_MS + 10_000);
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn expired_pack_state_evidence_is_indeterminate() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.state_evidence.evidence.valid_until_unix_ms = Some(DISPATCH_TIME_MS - 1);
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Indeterminate
        );
    }

    #[test]
    fn state_evidence_from_another_pack_is_blocked() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.state_evidence.evidence.asset_id = "different-pack".to_string();
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn state_claim_is_carried_inside_its_evidence_envelope() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.state_evidence.value = PackState::Quarantined;
        let evaluation = evaluate_pair(
            &mission("m1", "uav-a", 1_000, 1_000),
            &battery,
            DISPATCH_TIME_MS,
        );
        assert_eq!(evaluation.status, EligibilityStatus::Blocked);
        assert!(evaluation
            .findings
            .contains(&"pack_not_available_for_dispatch_epoch".to_string()));
    }

    #[test]
    fn unknown_pack_state_is_indeterminate() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.state_evidence.value = PackState::Unknown;
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Indeterminate
        );
    }

    #[test]
    fn usable_energy_below_required_reserve_is_blocked() {
        let battery = pack("p1", "uav-a", Some(999), Some(2_000));
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn thermal_limit_exceeded_is_blocked() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.thermal.conservative_upper_bound_c = Some(51);
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn unknown_evidence_class_is_indeterminate() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.usable_energy.evidence.evidence_class = EvidenceClass::Unknown;
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Indeterminate
        );
    }

    #[test]
    fn malformed_evidence_window_is_blocked() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.thermal.evidence.valid_from_unix_ms =
            Some(DISPATCH_TIME_MS + 5_000);
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn mismatched_evidence_asset_identity_is_blocked() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.usable_energy.evidence.asset_id = "different-pack".to_string();
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn mismatched_evidence_configuration_digest_is_blocked() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.thermal.evidence.configuration_digest = "other-digest".to_string();
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn configuration_mismatch_is_blocked() {
        let battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        let demand = mission("m1", "uav-b", 1_000, 1_000);
        assert_eq!(pair_status(&battery, &demand), EligibilityStatus::Blocked);
    }

    #[test]
    fn quarantined_pack_is_never_allocated() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.state_evidence.value = PackState::Quarantined;
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn unresolved_custody_is_blocked() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.custody_resolution.value = Some(false);
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn exact_assignment_never_double_books_a_pack() {
        let packs = vec![
            pack("pack-a", "uav-a", Some(2_000), Some(2_000)),
            pack("pack-b", "uav-a", Some(1_500), Some(1_500)),
            pack("pack-c", "uav-b", Some(1_200), Some(1_800)),
        ];
        let missions = vec![
            mission("mission-1", "uav-a", 1_700, 1_800),
            mission("mission-2", "uav-a", 1_200, 1_200),
            mission("mission-3", "uav-b", 1_000, 1_500),
            mission("mission-4", "uav-a", 900, 1_000),
        ];
        let report = evaluate_battery_service_network(&input(packs, missions)).unwrap();
        assert_eq!(report.serviceable_mission_count, 3);
        assert_eq!(report.assignments.len(), 3);
        assert_eq!(report.unserved_missions, vec!["mission-4".to_string()]);
        assert_eq!(
            report.assignments,
            vec![
                PackAssignment {
                    mission_id: "mission-1".to_string(),
                    pack_id: "pack-a".to_string(),
                    energy_margin_wh: 300,
                },
                PackAssignment {
                    mission_id: "mission-2".to_string(),
                    pack_id: "pack-b".to_string(),
                    energy_margin_wh: 300,
                },
                PackAssignment {
                    mission_id: "mission-3".to_string(),
                    pack_id: "pack-c".to_string(),
                    energy_margin_wh: 200,
                },
            ]
        );
        let assigned_pack_ids: BTreeSet<_> = report
            .assignments
            .iter()
            .map(|assignment| assignment.pack_id.as_str())
            .collect();
        assert_eq!(assigned_pack_ids.len(), report.assignments.len());
        assert!(!report.dispatch_authority_granted);
        assert!(!report.return_to_service_authority_granted);
        assert_eq!(report.dispatch_time_unix_ms, DISPATCH_TIME_MS);
    }

    #[test]
    fn result_is_permutation_invariant() {
        let packs = vec![
            pack("pack-b", "uav-a", Some(1_500), Some(1_500)),
            pack("pack-a", "uav-a", Some(2_000), Some(2_000)),
        ];
        let missions = vec![
            mission("mission-b", "uav-a", 1_200, 1_200),
            mission("mission-a", "uav-a", 1_700, 1_800),
        ];
        let first =
            evaluate_battery_service_network(&input(packs.clone(), missions.clone())).unwrap();
        let mut reversed_packs = packs;
        reversed_packs.reverse();
        let mut reversed_missions = missions;
        reversed_missions.reverse();
        let second =
            evaluate_battery_service_network(&input(reversed_packs, reversed_missions)).unwrap();
        assert_eq!(first.assignments, second.assignments);
        assert_eq!(first.unserved_missions, second.unserved_missions);
    }

    #[test]
    fn failed_post_test_blocks_pack() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.post_test.value = Some(false);
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Blocked
        );
    }

    #[test]
    fn unknown_identity_is_indeterminate_not_eligible() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.identity_verification.value = None;
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Indeterminate
        );
    }

    #[test]
    fn incomplete_evidence_window_is_indeterminate() {
        let mut battery = pack("p1", "uav-a", Some(2_000), Some(2_000));
        battery.custody_resolution.evidence.observed_at_unix_ms = None;
        assert_eq!(
            pair_status(&battery, &mission("m1", "uav-a", 1_000, 1_000)),
            EligibilityStatus::Indeterminate
        );
    }

    fn journal_input() -> PackStateJournalInput {
        let pack_id = "journal-pack";
        let digest = "urn:synthetic:configuration:journal-pack";
        PackStateJournalInput {
            pack_id: pack_id.to_string(),
            configuration_digest: digest.to_string(),
            replay_cutoff_unix_ms: DISPATCH_TIME_MS + 5_000,
            initial_state: PackStateEvidence {
                evidence: evidence(pack_id, digest, "journal-initial-state"),
                value: PackState::Available,
            },
            events: Vec::new(),
        }
    }

    struct JournalEventSpec<'a> {
        sequence: u64,
        seconds_after_epoch: u64,
        event_id: &'a str,
        idempotency_key: &'a str,
        from_state: PackState,
        proposed_to_state: PackState,
        disposition: PackTransitionDisposition,
        evidence_class: EvidenceClass,
    }

    fn journal_event(spec: JournalEventSpec<'_>) -> PackTransitionEvent {
        let pack_id = "journal-pack";
        let digest = "urn:synthetic:configuration:journal-pack";
        let mut event_evidence = evidence(pack_id, digest, spec.event_id);
        event_evidence.evidence_class = spec.evidence_class;
        PackTransitionEvent {
            event_id: spec.event_id.to_string(),
            idempotency_key: spec.idempotency_key.to_string(),
            operation_id: "operation-1".to_string(),
            sequence: spec.sequence,
            event_time_unix_ms: DISPATCH_TIME_MS + spec.seconds_after_epoch * 1_000,
            from_state: spec.from_state,
            proposed_to_state: spec.proposed_to_state,
            disposition: spec.disposition,
            evidence: event_evidence,
        }
    }

    #[test]
    fn expired_last_state_evidence_requires_reconciliation_at_cutoff() {
        let mut journal = journal_input();
        journal.replay_cutoff_unix_ms = DISPATCH_TIME_MS + 20_000;
        let report = replay_pack_state_journal(&journal).unwrap();
        assert_eq!(report.final_state, PackState::Available);
        assert!(report.reconciliation_required);
        assert!(report.findings.iter().any(|finding| {
            finding == "last_confirmed_pack_state_evidence_expired_at_replay_cutoff"
        }));
    }

    #[test]
    fn confirmed_journal_event_advances_pack_state() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-reserve",
            idempotency_key: "idempotency-reserve",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let report = replay_pack_state_journal(&journal).unwrap();
        assert_eq!(report.final_state, PackState::Reserved);
        assert_eq!(report.final_sequence, 1);
        assert!(!report.reconciliation_required);
        assert_eq!(report.accepted_event_ids, vec!["event-reserve".to_string()]);
    }

    #[test]
    fn timeout_preserves_last_confirmed_state_and_locks_mutation() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-timeout",
            idempotency_key: "idempotency-timeout",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Timeout,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let report = replay_pack_state_journal(&journal).unwrap();
        assert_eq!(report.final_state, PackState::Available);
        assert!(report.reconciliation_required);
        assert_eq!(report.last_uncertain_event_id, Some("event-timeout".to_string()));
    }

    #[test]
    fn normal_transition_is_rejected_while_reconciliation_is_required() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-timeout",
            idempotency_key: "idempotency-timeout",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Timeout,
            evidence_class: EvidenceClass::Synthetic,
        }));
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 2,
            seconds_after_epoch: 2,
            event_id: "event-retry",
            idempotency_key: "idempotency-retry",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("normal_event_blocked_pending_reconciliation")));
    }

    #[test]
    fn independent_reconciliation_clears_timeout_lock() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-timeout",
            idempotency_key: "idempotency-timeout",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Timeout,
            evidence_class: EvidenceClass::Synthetic,
        }));
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 2,
            seconds_after_epoch: 2,
            event_id: "event-reconcile",
            idempotency_key: "idempotency-reconcile",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Reconciled,
            evidence_class: EvidenceClass::IndependentTest,
        }));
        let report = replay_pack_state_journal(&journal).unwrap();
        assert_eq!(report.final_state, PackState::Reserved);
        assert!(!report.reconciliation_required);
        assert_eq!(report.last_uncertain_event_id, None);
    }

    #[test]
    fn synthetic_evidence_cannot_clear_reconciliation_lock() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-timeout",
            idempotency_key: "idempotency-timeout",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Timeout,
            evidence_class: EvidenceClass::Synthetic,
        }));
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 2,
            seconds_after_epoch: 2,
            event_id: "event-reconcile",
            idempotency_key: "idempotency-reconcile",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Reconciled,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("reconciliation_requires_measured_or_independent_evidence")));
    }

    #[test]
    fn duplicate_idempotency_key_is_rejected() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-one",
            idempotency_key: "same-idempotency-key",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 2,
            seconds_after_epoch: 2,
            event_id: "event-two",
            idempotency_key: "same-idempotency-key",
            from_state: PackState::Reserved,
            proposed_to_state: PackState::Installed,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("duplicate_idempotency_key")));
    }

    #[test]
    fn non_contiguous_event_sequence_is_rejected() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 2,
            seconds_after_epoch: 1,
            event_id: "event-gap",
            idempotency_key: "idempotency-gap",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("non_contiguous_event_sequence")));
    }

    #[test]
    fn illegal_pack_state_transition_is_rejected() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-illegal",
            idempotency_key: "idempotency-illegal",
            from_state: PackState::Available,
            proposed_to_state: PackState::Charging,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("illegal_pack_state_transition")));
    }

    #[test]
    fn duplicate_event_id_is_rejected() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "duplicate-event",
            idempotency_key: "idempotency-one",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 2,
            seconds_after_epoch: 2,
            event_id: "duplicate-event",
            idempotency_key: "idempotency-two",
            from_state: PackState::Reserved,
            proposed_to_state: PackState::Installed,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("duplicate_event_id")));
    }

    #[test]
    fn event_after_replay_cutoff_is_rejected() {
        let mut journal = journal_input();
        journal.events.push(journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 6,
            event_id: "event-after-cutoff",
            idempotency_key: "idempotency-after-cutoff",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        }));
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("event_time_non_monotonic_or_after_cutoff")));
    }

    #[test]
    fn journal_rejects_wrong_pack_evidence() {
        let mut journal = journal_input();
        let mut event = journal_event(JournalEventSpec {
            sequence: 1,
            seconds_after_epoch: 1,
            event_id: "event-wrong-pack",
            idempotency_key: "idempotency-wrong-pack",
            from_state: PackState::Available,
            proposed_to_state: PackState::Reserved,
            disposition: PackTransitionDisposition::Confirmed,
            evidence_class: EvidenceClass::Synthetic,
        });
        event.evidence.asset_id = "different-pack".to_string();
        journal.events.push(event);
        let errors = replay_pack_state_journal(&journal).unwrap_err();
        assert!(errors.iter().any(|error| error.contains("event_evidence_not_admissible")));
    }

    #[test]
    fn over_limit_exact_instance_is_rejected() {
        let packs = (0..7)
            .map(|index| pack(&format!("p{index}"), "uav-a", Some(2_000), Some(2_000)))
            .collect();
        let missions = (0..7)
            .map(|index| mission(&format!("m{index}"), "uav-a", 1_000, 1_000))
            .collect();
        assert!(evaluate_battery_service_network(&input(packs, missions)).is_err());
    }
}
