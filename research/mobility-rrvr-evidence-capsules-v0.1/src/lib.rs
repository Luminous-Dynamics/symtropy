// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

const SCHEMA_VERSION: &str = "0.1";
const BENCHMARK_NAME: &str = "RRVR-EVIDENCE-CAPSULES";

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum SourceRole {
    FaultInjector,
    PhysicalObserver,
    IndependentAdjudicator,
    Endpoint,
    NetworkMonitor,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceSource {
    pub source_id: String,
    pub principal_id: String,
    pub role: SourceRole,
    pub independence_group: String,
    pub profile_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct TimeInterval {
    pub clock_id: String,
    pub earliest_ns: u64,
    pub latest_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ObservationWindow {
    pub clock_id: String,
    pub start_ns: u64,
    pub end_ns: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CoverageWindow {
    pub clock_id: String,
    pub start_earliest_ns: u64,
    pub start_latest_ns: u64,
    pub end_earliest_ns: u64,
    pub end_latest_ns: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ObservationFinding {
    FaultPresent,
    FaultAbsent,
    EffectPresent,
    EffectAbsent,
    StaleEffectPresent,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum EvidencePayload {
    FaultExecutionReceipt {
        fault_id: String,
        delivered: bool,
    },
    Observation {
        subject_id: String,
        finding: ObservationFinding,
        coverage: Option<CoverageWindow>,
        coverage_complete: bool,
        raw_data_sha256: String,
        calibration_sha256: String,
    },
    EndpointFenceReceipt {
        holder_id: String,
        accepted_epoch: u64,
        queue_generation: u64,
        accepted_high_water: u64,
        retired_high_water: u64,
        outstanding_n: u32,
        in_flight_n: u32,
        terminal_actuator_fenced: bool,
    },
    EdgeClosureReceipt {
        edge_id: String,
        source_holder_id: String,
        target_holder_id: String,
        old_epoch_enqueue_closed: bool,
    },
    HolderInventory {
        complete: bool,
        holder_ids: Vec<String>,
        edge_ids: Vec<String>,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceCapsule {
    pub evidence_id: String,
    pub episode_id: String,
    pub attempt_id: String,
    pub command_id: String,
    pub source: EvidenceSource,
    pub parent_evidence_ids: Vec<String>,
    pub event_time: TimeInterval,
    pub observation_time: TimeInterval,
    pub ingest_sequence: u64,
    pub payload: EvidencePayload,
    pub content_sha256: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimType {
    FaultDelivered,
    EffectApplied,
    NoEffect,
    EndpointFenced,
    TemporalClosure,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ClaimState {
    Asserted,
    Unknown,
    Disputed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Claim {
    pub claim_id: String,
    pub episode_id: String,
    pub attempt_id: String,
    pub command_id: String,
    pub claim_type: ClaimType,
    pub asserted_value: bool,
    pub state: ClaimState,
    pub subject_id: String,
    pub required_epoch: Option<u64>,
    pub required_queue_generation: Option<u64>,
    pub observation_window: Option<ObservationWindow>,
    pub expected_holder_ids: Vec<String>,
    pub expected_edge_ids: Vec<String>,
    pub decision_cutoff_ingest_sequence: u64,
    pub evidence_ids: Vec<String>,
    pub evidence_snapshot_sha256: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct EvidenceBundle {
    pub schema_version: String,
    pub benchmark: String,
    pub campaign_id: String,
    pub claim_ceiling: String,
    pub evidence: Vec<EvidenceCapsule>,
    pub claims: Vec<Claim>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum AssessmentStatus {
    Qualified,
    Contradicted,
    Unresolved,
    Disputed,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ClaimAssessment {
    pub claim_id: String,
    pub claim_type: ClaimType,
    pub status: AssessmentStatus,
    pub reason_codes: Vec<String>,
    pub late_evidence_excluded_n: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct EvidenceReport {
    pub schema_version: String,
    pub benchmark: String,
    pub campaign_id: String,
    pub evidence_n: usize,
    pub claim_n: usize,
    pub qualified_n: usize,
    pub contradicted_n: usize,
    pub unresolved_n: usize,
    pub disputed_n: usize,
    pub claim_assessments: Vec<ClaimAssessment>,
    pub claim_ceiling: String,
}

pub fn evaluate_json(input: &str) -> Result<EvidenceReport, String> {
    let bundle: EvidenceBundle =
        serde_json::from_str(input).map_err(|error| format!("BUNDLE_JSON_INVALID: {error}"))?;
    validate_bundle(&bundle)?;
    Ok(evaluate_bundle(&bundle))
}

fn validate_bundle(bundle: &EvidenceBundle) -> Result<(), String> {
    if bundle.schema_version != SCHEMA_VERSION {
        return Err(format!(
            "SCHEMA_VERSION_UNSUPPORTED: expected {SCHEMA_VERSION}, got {}",
            bundle.schema_version
        ));
    }
    if bundle.benchmark != BENCHMARK_NAME {
        return Err(format!(
            "BENCHMARK_NAME_MISMATCH: expected {BENCHMARK_NAME}, got {}",
            bundle.benchmark
        ));
    }
    require_text("campaign_id", &bundle.campaign_id)?;
    if bundle.claim_ceiling.trim().chars().count() < 40 {
        return Err("CLAIM_CEILING_TOO_SHORT".to_owned());
    }
    if bundle.evidence.is_empty() {
        return Err("NO_EVIDENCE_CAPSULES".to_owned());
    }
    if bundle.claims.is_empty() {
        return Err("NO_CLAIMS".to_owned());
    }

    let mut by_id: BTreeMap<&str, &EvidenceCapsule> = BTreeMap::new();
    let mut ingest_sequences = BTreeSet::new();
    for capsule in &bundle.evidence {
        require_text("evidence_id", &capsule.evidence_id)?;
        require_text("episode_id", &capsule.episode_id)?;
        require_text("attempt_id", &capsule.attempt_id)?;
        require_text("command_id", &capsule.command_id)?;
        require_text("source_id", &capsule.source.source_id)?;
        require_text("principal_id", &capsule.source.principal_id)?;
        require_text("independence_group", &capsule.source.independence_group)?;
        require_digest("source.profile_sha256", &capsule.source.profile_sha256)?;
        validate_time("event_time", &capsule.event_time)?;
        validate_time("observation_time", &capsule.observation_time)?;
        if capsule.event_time.clock_id != capsule.observation_time.clock_id {
            return Err(format!(
                "CLOCK_DOMAIN_MISMATCH: {}",
                capsule.evidence_id
            ));
        }
        if capsule.observation_time.earliest_ns < capsule.event_time.latest_ns {
            return Err(format!(
                "TIME_ORDER_UNCERTAIN: {}",
                capsule.evidence_id
            ));
        }
        if capsule.parent_evidence_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(format!(
                "PARENT_IDS_NOT_STRICTLY_SORTED: {}",
                capsule.evidence_id
            ));
        }
        validate_payload(capsule)?;
        if !ingest_sequences.insert(capsule.ingest_sequence) {
            return Err(format!(
                "DUPLICATE_INGEST_SEQUENCE: {}",
                capsule.ingest_sequence
            ));
        }
        if !is_sha256(&capsule.content_sha256) {
            return Err(format!("INVALID_CONTENT_SHA256: {}", capsule.evidence_id));
        }
        let expected_digest = capsule_digest(capsule)?;
        if expected_digest != capsule.content_sha256 {
            return Err(format!(
                "EVIDENCE_DIGEST_MISMATCH: {}",
                capsule.evidence_id
            ));
        }
        if by_id.insert(capsule.evidence_id.as_str(), capsule).is_some() {
            return Err(format!("DUPLICATE_EVIDENCE_ID: {}", capsule.evidence_id));
        }
    }

    for capsule in &bundle.evidence {
        for parent_id in &capsule.parent_evidence_ids {
            let parent = by_id
                .get(parent_id.as_str())
                .ok_or_else(|| format!("PARENT_EVIDENCE_MISSING: {parent_id}"))?;
            if parent.ingest_sequence >= capsule.ingest_sequence {
                return Err(format!(
                    "PROVENANCE_ORDER_INVALID: parent={} child={}",
                    parent_id, capsule.evidence_id
                ));
            }
            if parent.episode_id != capsule.episode_id
                || parent.attempt_id != capsule.attempt_id
                || parent.command_id != capsule.command_id
            {
                return Err(format!(
                    "PROVENANCE_SCOPE_MISMATCH: parent={} child={}",
                    parent_id, capsule.evidence_id
                ));
            }
        }
    }

    let mut claim_ids = BTreeSet::new();
    for claim in &bundle.claims {
        require_text("claim_id", &claim.claim_id)?;
        require_text("claim.episode_id", &claim.episode_id)?;
        require_text("claim.attempt_id", &claim.attempt_id)?;
        require_text("claim.command_id", &claim.command_id)?;
        require_text("claim.subject_id", &claim.subject_id)?;
        require_digest("claim.evidence_snapshot_sha256", &claim.evidence_snapshot_sha256)?;
        if !claim_ids.insert(claim.claim_id.as_str()) {
            return Err(format!("DUPLICATE_CLAIM_ID: {}", claim.claim_id));
        }
        if claim.evidence_ids.windows(2).any(|pair| pair[0] >= pair[1]) {
            return Err(format!(
                "CLAIM_EVIDENCE_IDS_NOT_STRICTLY_SORTED: {}",
                claim.claim_id
            ));
        }
        if claim.evidence_ids.is_empty() && claim.state == ClaimState::Asserted {
            return Err(format!("ASSERTED_CLAIM_WITHOUT_EVIDENCE: {}", claim.claim_id));
        }
        for evidence_id in &claim.evidence_ids {
            let capsule = by_id
                .get(evidence_id.as_str())
                .ok_or_else(|| format!("CLAIM_EVIDENCE_MISSING: {evidence_id}"))?;
            if capsule.ingest_sequence > claim.decision_cutoff_ingest_sequence {
                return Err(format!(
                    "EVIDENCE_AFTER_DECISION_CUTOFF: claim={} evidence={}",
                    claim.claim_id, evidence_id
                ));
            }
            if capsule.episode_id != claim.episode_id
                || capsule.attempt_id != claim.attempt_id
                || capsule.command_id != claim.command_id
            {
                return Err(format!(
                    "CLAIM_EVIDENCE_SCOPE_MISMATCH: claim={} evidence={}",
                    claim.claim_id, evidence_id
                ));
            }
        }
        let expected_snapshot = snapshot_digest(&claim.evidence_ids, &by_id)?;
        if expected_snapshot != claim.evidence_snapshot_sha256 {
            return Err(format!("CLAIM_SNAPSHOT_DIGEST_MISMATCH: {}", claim.claim_id));
        }
        validate_claim_shape(claim)?;
    }

    Ok(())
}

fn validate_payload(capsule: &EvidenceCapsule) -> Result<(), String> {
    match &capsule.payload {
        EvidencePayload::FaultExecutionReceipt { fault_id, .. } => {
            require_text("fault_id", fault_id)?;
            if capsule.source.role != SourceRole::FaultInjector {
                return Err(format!(
                    "FAULT_RECEIPT_SOURCE_ROLE_INVALID: {}",
                    capsule.evidence_id
                ));
            }
        }
        EvidencePayload::Observation {
            subject_id,
            coverage,
            coverage_complete,
            raw_data_sha256,
            calibration_sha256,
            ..
        } => {
            require_text("observation.subject_id", subject_id)?;
            require_digest("observation.raw_data_sha256", raw_data_sha256)?;
            require_digest("observation.calibration_sha256", calibration_sha256)?;
            if !matches!(
                capsule.source.role,
                SourceRole::PhysicalObserver | SourceRole::IndependentAdjudicator
            ) {
                return Err(format!(
                    "OBSERVATION_SOURCE_ROLE_INVALID: {}",
                    capsule.evidence_id
                ));
            }
            if let Some(window) = coverage {
                if window.clock_id.trim().is_empty()
                    || window.start_earliest_ns > window.start_latest_ns
                    || window.end_earliest_ns > window.end_latest_ns
                    || window.start_latest_ns > window.end_earliest_ns
                {
                    return Err(format!(
                        "OBSERVATION_COVERAGE_INVALID: {}",
                        capsule.evidence_id
                    ));
                }
            } else if *coverage_complete {
                return Err(format!(
                    "COMPLETE_COVERAGE_WITHOUT_WINDOW: {}",
                    capsule.evidence_id
                ));
            }
        }
        EvidencePayload::EndpointFenceReceipt { holder_id, .. } => {
            require_text("endpoint.holder_id", holder_id)?;
            if capsule.source.role != SourceRole::Endpoint {
                return Err(format!(
                    "ENDPOINT_RECEIPT_SOURCE_ROLE_INVALID: {}",
                    capsule.evidence_id
                ));
            }
        }
        EvidencePayload::EdgeClosureReceipt {
            edge_id,
            source_holder_id,
            target_holder_id,
            ..
        } => {
            require_text("edge.edge_id", edge_id)?;
            require_text("edge.source_holder_id", source_holder_id)?;
            require_text("edge.target_holder_id", target_holder_id)?;
            if !matches!(
                capsule.source.role,
                SourceRole::NetworkMonitor | SourceRole::Endpoint
            ) {
                return Err(format!(
                    "EDGE_RECEIPT_SOURCE_ROLE_INVALID: {}",
                    capsule.evidence_id
                ));
            }
        }
        EvidencePayload::HolderInventory {
            holder_ids, edge_ids, ..
        } => {
            if !matches!(
                capsule.source.role,
                SourceRole::NetworkMonitor | SourceRole::IndependentAdjudicator
            ) {
                return Err(format!(
                    "INVENTORY_SOURCE_ROLE_INVALID: {}",
                    capsule.evidence_id
                ));
            }
            validate_sorted_ids("holder_ids", holder_ids)?;
            validate_sorted_ids("edge_ids", edge_ids)?;
        }
    }
    Ok(())
}

fn validate_claim_shape(claim: &Claim) -> Result<(), String> {
    match claim.claim_type {
        ClaimType::FaultDelivered => {
            if claim.observation_window.is_none() && !claim.asserted_value {
                return Err(format!(
                    "FAULT_ABSENCE_REQUIRES_WINDOW: {}",
                    claim.claim_id
                ));
            }
        }
        ClaimType::EffectApplied => {
            if !claim.asserted_value && claim.observation_window.is_none() {
                return Err(format!(
                    "EFFECT_ABSENCE_REQUIRES_WINDOW: {}",
                    claim.claim_id
                ));
            }
        }
        ClaimType::NoEffect => {
            if claim.observation_window.is_none() {
                return Err(format!(
                    "NO_EFFECT_REQUIRES_WINDOW: {}",
                    claim.claim_id
                ));
            }
        }
        ClaimType::EndpointFenced => {
            if claim.required_epoch.is_none() || claim.required_queue_generation.is_none() {
                return Err(format!(
                    "ENDPOINT_FENCE_REQUIRES_EPOCH: {}",
                    claim.claim_id
                ));
            }
        }
        ClaimType::TemporalClosure => {
            if claim.required_epoch.is_none()
                || claim.required_queue_generation.is_none()
                || claim.expected_holder_ids.is_empty()
                || claim.expected_edge_ids.is_empty()
            {
                return Err(format!(
                    "TEMPORAL_CLOSURE_SCOPE_INCOMPLETE: {}",
                    claim.claim_id
                ));
            }
            validate_sorted_ids("expected_holder_ids", &claim.expected_holder_ids)?;
            validate_sorted_ids("expected_edge_ids", &claim.expected_edge_ids)?;
        }
    }
    if let Some(window) = &claim.observation_window {
        if window.clock_id.trim().is_empty() || window.start_ns >= window.end_ns {
            return Err(format!(
                "CLAIM_OBSERVATION_WINDOW_INVALID: {}",
                claim.claim_id
            ));
        }
    }
    if claim.claim_type != ClaimType::TemporalClosure
        && (!claim.expected_holder_ids.is_empty() || !claim.expected_edge_ids.is_empty())
    {
        return Err(format!(
            "UNEXPECTED_CLOSURE_INVENTORY_SCOPE: {}",
            claim.claim_id
        ));
    }
    if !matches!(claim.claim_type, ClaimType::EndpointFenced | ClaimType::TemporalClosure)
        && claim.required_queue_generation.is_some()
    {
        return Err(format!(
            "UNEXPECTED_QUEUE_GENERATION_SCOPE: {}",
            claim.claim_id
        ));
    }
    Ok(())
}

fn validate_sorted_ids(name: &str, ids: &[String]) -> Result<(), String> {
    if ids.iter().any(|value| value.trim().is_empty())
        || ids.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(format!("IDS_NOT_STRICTLY_SORTED: {name}"));
    }
    Ok(())
}

fn validate_time(name: &str, time: &TimeInterval) -> Result<(), String> {
    if time.clock_id.trim().is_empty() || time.earliest_ns > time.latest_ns {
        return Err(format!("TIME_INTERVAL_INVALID: {name}"));
    }
    Ok(())
}

fn require_text(name: &str, value: &str) -> Result<(), String> {
    if value.trim().is_empty() {
        Err(format!("EMPTY_FIELD: {name}"))
    } else {
        Ok(())
    }
}

fn require_digest(name: &str, value: &str) -> Result<(), String> {
    if is_sha256(value) {
        Ok(())
    } else {
        Err(format!("INVALID_SHA256: {name}"))
    }
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn hash_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn capsule_digest(capsule: &EvidenceCapsule) -> Result<String, String> {
    let mut material = capsule.clone();
    material.content_sha256.clear();
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| format!("CAPSULE_CANONICALIZATION_FAILED: {error}"))?;
    Ok(hash_hex(&bytes))
}

fn snapshot_digest(
    evidence_ids: &[String],
    by_id: &BTreeMap<&str, &EvidenceCapsule>,
) -> Result<String, String> {
    let mut material: Vec<(&str, &str)> = Vec::with_capacity(evidence_ids.len());
    for evidence_id in evidence_ids {
        let capsule = by_id
            .get(evidence_id.as_str())
            .ok_or_else(|| format!("CLAIM_EVIDENCE_MISSING: {evidence_id}"))?;
        material.push((capsule.evidence_id.as_str(), capsule.content_sha256.as_str()));
    }
    let bytes = serde_json::to_vec(&material)
        .map_err(|error| format!("SNAPSHOT_CANONICALIZATION_FAILED: {error}"))?;
    Ok(hash_hex(&bytes))
}

fn evaluate_bundle(bundle: &EvidenceBundle) -> EvidenceReport {
    let by_id: BTreeMap<&str, &EvidenceCapsule> = bundle
        .evidence
        .iter()
        .map(|capsule| (capsule.evidence_id.as_str(), capsule))
        .collect();
    let claim_assessments: Vec<ClaimAssessment> = bundle
        .claims
        .iter()
        .map(|claim| assess_claim(claim, &bundle.evidence, &by_id))
        .collect();

    let count = |status| {
        claim_assessments
            .iter()
            .filter(|assessment| assessment.status == status)
            .count()
    };

    EvidenceReport {
        schema_version: SCHEMA_VERSION.to_owned(),
        benchmark: BENCHMARK_NAME.to_owned(),
        campaign_id: bundle.campaign_id.clone(),
        evidence_n: bundle.evidence.len(),
        claim_n: bundle.claims.len(),
        qualified_n: count(AssessmentStatus::Qualified),
        contradicted_n: count(AssessmentStatus::Contradicted),
        unresolved_n: count(AssessmentStatus::Unresolved),
        disputed_n: count(AssessmentStatus::Disputed),
        claim_assessments,
        claim_ceiling: bundle.claim_ceiling.clone(),
    }
}

fn assess_claim(
    claim: &Claim,
    all_evidence: &[EvidenceCapsule],
    by_id: &BTreeMap<&str, &EvidenceCapsule>,
) -> ClaimAssessment {
    let selected: Vec<&EvidenceCapsule> = claim
        .evidence_ids
        .iter()
        .filter_map(|id| by_id.get(id.as_str()).copied())
        .collect();
    let selected_ids: BTreeSet<&str> = claim.evidence_ids.iter().map(String::as_str).collect();
    let late_evidence_excluded_n = all_evidence
        .iter()
        .filter(|capsule| {
            capsule.episode_id == claim.episode_id
                && capsule.attempt_id == claim.attempt_id
                && capsule.command_id == claim.command_id
                && capsule.ingest_sequence > claim.decision_cutoff_ingest_sequence
                && !selected_ids.contains(capsule.evidence_id.as_str())
        })
        .count();

    let (status, reason_codes) = if claim.state == ClaimState::Unknown {
        (AssessmentStatus::Unresolved, vec!["CLAIM_STATE_UNKNOWN".to_owned()])
    } else if claim.state == ClaimState::Disputed {
        (AssessmentStatus::Disputed, vec!["CLAIM_STATE_DISPUTED".to_owned()])
    } else {
        evaluate_asserted_claim(claim, &selected)
    };

    ClaimAssessment {
        claim_id: claim.claim_id.clone(),
        claim_type: claim.claim_type,
        status,
        reason_codes,
        late_evidence_excluded_n,
    }
}

fn evaluate_asserted_claim(
    claim: &Claim,
    evidence: &[&EvidenceCapsule],
) -> (AssessmentStatus, Vec<String>) {
    match claim.claim_type {
        ClaimType::FaultDelivered => evaluate_fault_delivered(claim, evidence),
        ClaimType::EffectApplied | ClaimType::NoEffect => {
            evaluate_effect_claim(claim, evidence)
        }
        ClaimType::EndpointFenced => evaluate_endpoint_fenced(claim, evidence),
        ClaimType::TemporalClosure => evaluate_temporal_closure(claim, evidence),
    }
}

fn evaluate_fault_delivered(
    claim: &Claim,
    evidence: &[&EvidenceCapsule],
) -> (AssessmentStatus, Vec<String>) {
    let receipts: Vec<(&EvidenceCapsule, bool)> = evidence
        .iter()
        .filter_map(|capsule| match &capsule.payload {
            EvidencePayload::FaultExecutionReceipt { fault_id, delivered }
                if fault_id == &claim.subject_id =>
            {
                Some((*capsule, *delivered))
            }
            _ => None,
        })
        .collect();
    let observers: Vec<(&EvidenceCapsule, ObservationFinding, bool)> = evidence
        .iter()
        .filter_map(|capsule| match &capsule.payload {
            EvidencePayload::Observation {
                subject_id,
                finding,
                coverage,
                coverage_complete,
                ..
            } if subject_id == &claim.subject_id
                && matches!(
                    finding,
                    ObservationFinding::FaultPresent | ObservationFinding::FaultAbsent
                ) =>
            {
                Some((*capsule, *finding, coverage_is_complete(coverage.as_ref(), *coverage_complete, claim)))
            }
            _ => None,
        })
        .collect();

    if receipts.is_empty() || observers.is_empty() {
        return unresolved("FAULT_DELIVERY_EVIDENCE_INCOMPLETE");
    }
    let injector_groups: BTreeSet<&str> = receipts
        .iter()
        .map(|(capsule, _)| capsule.source.independence_group.as_str())
        .collect();
    if observers.iter().any(|(capsule, _, _)| {
        capsule.source.role != SourceRole::PhysicalObserver
            || injector_groups.contains(capsule.source.independence_group.as_str())
    }) {
        return unresolved("FAULT_OBSERVER_NOT_INDEPENDENT");
    }
    if receipts.iter().any(|(_, delivered)| *delivered != receipts[0].1) {
        return disputed("FAULT_RECEIPTS_CONTRADICT");
    }
    let findings: BTreeSet<ObservationFinding> =
        observers.iter().map(|(_, finding, _)| *finding).collect();
    if findings.len() > 1 {
        return disputed("FAULT_OBSERVATIONS_CONTRADICT");
    }
    let expected = if claim.asserted_value {
        ObservationFinding::FaultPresent
    } else {
        ObservationFinding::FaultAbsent
    };
    if expected == ObservationFinding::FaultAbsent
        && observers.iter().any(|(_, _, covered)| !covered)
    {
        return unresolved("FAULT_ABSENCE_COVERAGE_INCOMPLETE");
    }
    let receipt_matches = receipts[0].1 == claim.asserted_value;
    if findings.contains(&expected) && receipt_matches {
        qualified("FAULT_DELIVERY_CORROBORATED")
    } else {
        contradicted("FAULT_DELIVERY_NOT_CORROBORATED")
    }
}

fn evaluate_effect_claim(
    claim: &Claim,
    evidence: &[&EvidenceCapsule],
) -> (AssessmentStatus, Vec<String>) {
    let expected = match (claim.claim_type, claim.asserted_value) {
        (ClaimType::EffectApplied, true) | (ClaimType::NoEffect, false) => {
            ObservationFinding::EffectPresent
        }
        (ClaimType::EffectApplied, false) | (ClaimType::NoEffect, true) => {
            ObservationFinding::EffectAbsent
        }
        _ => return unresolved("EFFECT_CLAIM_TYPE_INVALID"),
    };

    let observations: Vec<(&EvidenceCapsule, ObservationFinding, bool)> = evidence
        .iter()
        .filter_map(|capsule| match &capsule.payload {
            EvidencePayload::Observation {
                subject_id,
                finding,
                coverage,
                coverage_complete,
                ..
            } if subject_id == &claim.subject_id
                && matches!(
                    finding,
                    ObservationFinding::EffectPresent | ObservationFinding::EffectAbsent
                ) =>
            {
                Some((*capsule, *finding, coverage_is_complete(coverage.as_ref(), *coverage_complete, claim)))
            }
            _ => None,
        })
        .collect();
    if observations.is_empty() {
        return unresolved("PHYSICAL_EFFECT_OBSERVATION_MISSING");
    }
    let findings: BTreeSet<ObservationFinding> =
        observations.iter().map(|(_, finding, _)| *finding).collect();
    if findings.len() > 1 {
        return disputed("PHYSICAL_EFFECT_OBSERVATIONS_CONTRADICT");
    }
    if expected == ObservationFinding::EffectAbsent
        && observations.iter().any(|(_, _, covered)| !covered)
    {
        return unresolved("NO_EFFECT_COVERAGE_INCOMPLETE");
    }
    if findings.contains(&expected) {
        qualified("PHYSICAL_EFFECT_CLAIM_CORROBORATED")
    } else {
        contradicted("PHYSICAL_EFFECT_CLAIM_REFUTED")
    }
}

fn coverage_is_complete(
    coverage: Option<&CoverageWindow>,
    coverage_complete: bool,
    claim: &Claim,
) -> bool {
    let (Some(coverage), Some(expected)) = (coverage, claim.observation_window.as_ref()) else {
        return false;
    };
    coverage_complete
        && coverage.clock_id == expected.clock_id
        && coverage.start_earliest_ns <= coverage.start_latest_ns
        && coverage.end_earliest_ns <= coverage.end_latest_ns
        && coverage.start_latest_ns <= expected.start_ns
        && coverage.end_earliest_ns >= expected.end_ns
}

fn endpoint_is_fenced(
    capsule: &EvidenceCapsule,
    holder_id: &str,
    required_epoch: u64,
    required_queue_generation: u64,
) -> Option<bool> {
    match &capsule.payload {
        EvidencePayload::EndpointFenceReceipt {
            holder_id: observed_holder,
            accepted_epoch,
            queue_generation,
            accepted_high_water,
            retired_high_water,
            outstanding_n,
            in_flight_n,
            terminal_actuator_fenced,
            ..
        } if observed_holder == holder_id && capsule.source.role == SourceRole::Endpoint => {
            Some(
                *accepted_epoch >= required_epoch
                    && *queue_generation == required_queue_generation
                    && accepted_high_water == retired_high_water
                    && *outstanding_n == 0
                    && *in_flight_n == 0
                    && *terminal_actuator_fenced,
            )
        }
        _ => None,
    }
}

fn evaluate_endpoint_fenced(
    claim: &Claim,
    evidence: &[&EvidenceCapsule],
) -> (AssessmentStatus, Vec<String>) {
    let (Some(required_epoch), Some(required_queue_generation)) =
        (claim.required_epoch, claim.required_queue_generation)
    else {
        return unresolved("ENDPOINT_FENCE_PROFILE_INCOMPLETE");
    };
    let states: Vec<bool> = evidence
        .iter()
        .filter_map(|capsule| {
            endpoint_is_fenced(
                capsule,
                &claim.subject_id,
                required_epoch,
                required_queue_generation,
            )
        })
        .collect();
    if states.is_empty() {
        return unresolved("ENDPOINT_FENCE_RECEIPT_MISSING");
    }
    if states.iter().any(|state| *state != states[0]) {
        return disputed("ENDPOINT_FENCE_RECEIPTS_CONTRADICT");
    }
    if states[0] == claim.asserted_value {
        qualified("ENDPOINT_FENCE_STATE_CORROBORATED")
    } else {
        contradicted("ENDPOINT_FENCE_STATE_REFUTED")
    }
}

fn evaluate_temporal_closure(
    claim: &Claim,
    evidence: &[&EvidenceCapsule],
) -> (AssessmentStatus, Vec<String>) {
    let (Some(required_epoch), Some(required_queue_generation)) =
        (claim.required_epoch, claim.required_queue_generation)
    else {
        return unresolved("TEMPORAL_CLOSURE_PROFILE_INCOMPLETE");
    };
    let inventories: Vec<(&EvidenceCapsule, bool, &Vec<String>, &Vec<String>)> = evidence
        .iter()
        .filter_map(|capsule| match &capsule.payload {
            EvidencePayload::HolderInventory {
                complete,
                holder_ids,
                edge_ids,
            } => Some((*capsule, *complete, holder_ids, edge_ids)),
            _ => None,
        })
        .collect();
    if inventories.is_empty() {
        return unresolved("HOLDER_INVENTORY_MISSING");
    }
    if inventories.iter().any(|(_, complete, _, _)| !complete) {
        return unresolved("HOLDER_INVENTORY_INCOMPLETE");
    }
    let expected_holders: BTreeSet<&str> =
        claim.expected_holder_ids.iter().map(String::as_str).collect();
    let expected_edges: BTreeSet<&str> =
        claim.expected_edge_ids.iter().map(String::as_str).collect();
    let inventories_match = inventories.iter().all(|(_, _, holders, edges)| {
        holders.iter().map(String::as_str).collect::<BTreeSet<_>>() == expected_holders
            && edges.iter().map(String::as_str).collect::<BTreeSet<_>>() == expected_edges
    });
    if !inventories_match {
        return contradicted("HOLDER_INVENTORY_SCOPE_MISMATCH");
    }

    let mut holder_states = Vec::new();
    for holder_id in &claim.expected_holder_ids {
        let states: Vec<bool> = evidence
            .iter()
            .filter_map(|capsule| {
                endpoint_is_fenced(
                    capsule,
                    holder_id,
                    required_epoch,
                    required_queue_generation,
                )
            })
            .collect();
        if states.is_empty() {
            return unresolved("HOLDER_FENCE_RECEIPT_MISSING");
        }
        if states.iter().any(|state| *state != states[0]) {
            return disputed("HOLDER_FENCE_RECEIPTS_CONTRADICT");
        }
        holder_states.push(states[0]);
    }

    let mut edge_states = Vec::new();
    for edge_id in &claim.expected_edge_ids {
        let states: Vec<bool> = evidence
            .iter()
            .filter_map(|capsule| match &capsule.payload {
                EvidencePayload::EdgeClosureReceipt {
                    edge_id: observed_id,
                    old_epoch_enqueue_closed,
                    source_holder_id,
                    target_holder_id,
                } if observed_id == edge_id
                    && claim.expected_holder_ids.contains(source_holder_id)
                    && claim.expected_holder_ids.contains(target_holder_id)
                    && matches!(
                        capsule.source.role,
                        SourceRole::NetworkMonitor | SourceRole::Endpoint
                    ) =>
                {
                    Some(*old_epoch_enqueue_closed)
                }
                _ => None,
            })
            .collect();
        if states.is_empty() {
            return unresolved("EDGE_CLOSURE_RECEIPT_MISSING");
        }
        if states.iter().any(|state| *state != states[0]) {
            return disputed("EDGE_CLOSURE_RECEIPTS_CONTRADICT");
        }
        edge_states.push(states[0]);
    }

    let stale_effect_observed = evidence.iter().any(|capsule| {
        matches!(
            &capsule.payload,
            EvidencePayload::Observation {
                subject_id,
                finding: ObservationFinding::StaleEffectPresent,
                ..
            } if subject_id == &claim.subject_id
        )
    });
    let observed_closed = holder_states.iter().all(|state| *state)
        && edge_states.iter().all(|state| *state)
        && !stale_effect_observed;
    if observed_closed == claim.asserted_value {
        qualified("TEMPORAL_CLOSURE_CORROBORATED")
    } else {
        contradicted("TEMPORAL_CLOSURE_REFUTED")
    }
}

fn qualified(reason: &str) -> (AssessmentStatus, Vec<String>) {
    (AssessmentStatus::Qualified, vec![reason.to_owned()])
}

fn contradicted(reason: &str) -> (AssessmentStatus, Vec<String>) {
    (AssessmentStatus::Contradicted, vec![reason.to_owned()])
}

fn unresolved(reason: &str) -> (AssessmentStatus, Vec<String>) {
    (AssessmentStatus::Unresolved, vec![reason.to_owned()])
}

fn disputed(reason: &str) -> (AssessmentStatus, Vec<String>) {
    (AssessmentStatus::Disputed, vec![reason.to_owned()])
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../fixtures/evidence_bundle_v0_1.json");

    fn parsed_fixture() -> EvidenceBundle {
        serde_json::from_str(FIXTURE).expect("fixture must parse")
    }

    fn reseal_bundle(bundle: &mut EvidenceBundle) {
        for capsule in &mut bundle.evidence {
            capsule.content_sha256.clear();
            capsule.content_sha256 =
                capsule_digest(capsule).expect("capsule digest must serialize");
        }
        let by_id: BTreeMap<&str, &EvidenceCapsule> = bundle
            .evidence
            .iter()
            .map(|capsule| (capsule.evidence_id.as_str(), capsule))
            .collect();
        for claim in &mut bundle.claims {
            claim.evidence_snapshot_sha256 =
                snapshot_digest(&claim.evidence_ids, &by_id).expect("snapshot must serialize");
        }
    }

    fn base_capsule(
        evidence_id: &str,
        role: SourceRole,
        group: &str,
        ingest_sequence: u64,
        payload: EvidencePayload,
    ) -> EvidenceCapsule {
        EvidenceCapsule {
            evidence_id: evidence_id.to_owned(),
            episode_id: "episode-1".to_owned(),
            attempt_id: "attempt-1".to_owned(),
            command_id: "command-1".to_owned(),
            source: EvidenceSource {
                source_id: format!("source-{evidence_id}"),
                principal_id: format!("principal-{evidence_id}"),
                role,
                independence_group: group.to_owned(),
                profile_sha256: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".to_owned(),
            },
            parent_evidence_ids: Vec::new(),
            event_time: TimeInterval {
                clock_id: "robot-clock".to_owned(),
                earliest_ns: 100,
                latest_ns: 110,
            },
            observation_time: TimeInterval {
                clock_id: "robot-clock".to_owned(),
                earliest_ns: 120,
                latest_ns: 130,
            },
            ingest_sequence,
            payload,
            content_sha256: String::new(),
        }
    }

    fn claim_for(
        claim_id: &str,
        claim_type: ClaimType,
        subject_id: &str,
        asserted_value: bool,
        evidence_ids: Vec<String>,
    ) -> Claim {
        Claim {
            claim_id: claim_id.to_owned(),
            episode_id: "episode-1".to_owned(),
            attempt_id: "attempt-1".to_owned(),
            command_id: "command-1".to_owned(),
            claim_type,
            asserted_value,
            state: ClaimState::Asserted,
            subject_id: subject_id.to_owned(),
            required_epoch: None,
            required_queue_generation: None,
            observation_window: None,
            expected_holder_ids: Vec::new(),
            expected_edge_ids: Vec::new(),
            decision_cutoff_ingest_sequence: 100,
            evidence_ids,
            evidence_snapshot_sha256: String::new(),
        }
    }

    fn bundle_with(evidence: Vec<EvidenceCapsule>, mut claims: Vec<Claim>) -> EvidenceBundle {
        let mut bundle = EvidenceBundle {
            schema_version: SCHEMA_VERSION.to_owned(),
            benchmark: BENCHMARK_NAME.to_owned(),
            campaign_id: "synthetic-capsule-test".to_owned(),
            claim_ceiling: "Synthetic validator behavior only; no physical evidence.".to_owned(),
            evidence,
            claims: Vec::new(),
        };
        for capsule in &mut bundle.evidence {
            capsule.content_sha256 = capsule_digest(capsule).expect("capsule digest");
        }
        let by_id: BTreeMap<&str, &EvidenceCapsule> = bundle
            .evidence
            .iter()
            .map(|capsule| (capsule.evidence_id.as_str(), capsule))
            .collect();
        for claim in &mut claims {
            claim.evidence_snapshot_sha256 =
                snapshot_digest(&claim.evidence_ids, &by_id).expect("snapshot digest");
        }
        bundle.claims = claims;
        bundle
    }

    #[test]
    fn synthetic_known_answer_fixture_qualifies() {
        let report = evaluate_json(FIXTURE).expect("fixture should validate");
        assert_eq!(report.evidence_n, 1);
        assert_eq!(report.claim_n, 1);
        assert_eq!(report.qualified_n, 1);
        assert_eq!(report.unresolved_n, 0);
    }

    #[test]
    fn content_digest_tampering_is_rejected() {
        let changed = FIXTURE.replacen(
            "3128b1a1892cba39e4a6838a1ae3fb73c26ed470761566c50b1bfa6728b79328",
            "0128b1a1892cba39e4a6838a1ae3fb73c26ed470761566c50b1bfa6728b79328",
            1,
        );
        let error = evaluate_json(&changed).expect_err("mutated digest must reject");
        assert!(error.starts_with("EVIDENCE_DIGEST_MISMATCH"));
    }

    #[test]
    fn unknown_claim_state_stays_unresolved() {
        let mut bundle = parsed_fixture();
        bundle.claims[0].state = ClaimState::Unknown;
        reseal_bundle(&mut bundle);
        let serialized = serde_json::to_string(&bundle).expect("bundle serialization");
        let report = evaluate_json(&serialized).expect("unknown remains valid input");
        assert_eq!(report.unresolved_n, 1);
        assert_eq!(report.qualified_n, 0);
    }

    #[test]
    fn late_evidence_does_not_rewrite_prior_claim() {
        let mut bundle = parsed_fixture();
        let mut late = bundle.evidence[0].clone();
        late.evidence_id = "evidence-late".to_owned();
        late.ingest_sequence = 101;
        late.parent_evidence_ids.clear();
        late.content_sha256.clear();
        late.content_sha256 = capsule_digest(&late).expect("late evidence digest");
        bundle.evidence.push(late);
        let serialized = serde_json::to_string(&bundle).expect("bundle serialization");
        let report = evaluate_json(&serialized).expect("late evidence remains outside cutoff");
        assert_eq!(report.claim_assessments[0].status, AssessmentStatus::Qualified);
        assert_eq!(report.claim_assessments[0].late_evidence_excluded_n, 1);
    }

    #[test]
    fn no_effect_requires_continuous_observation_coverage() {
        let mut bundle = parsed_fixture();
        let EvidencePayload::Observation {
            finding,
            coverage,
            coverage_complete,
            ..
        } = &mut bundle.evidence[0].payload
        else {
            panic!("fixture must contain an observation");
        };
        *finding = ObservationFinding::EffectAbsent;
        *coverage = None;
        *coverage_complete = false;
        bundle.claims[0].claim_type = ClaimType::NoEffect;
        bundle.claims[0].asserted_value = true;
        bundle.claims[0].observation_window = Some(ObservationWindow {
            clock_id: "robot-clock".to_owned(),
            start_ns: 100,
            end_ns: 200,
        });
        reseal_bundle(&mut bundle);
        let serialized = serde_json::to_string(&bundle).expect("bundle serialization");
        let report = evaluate_json(&serialized).expect("missing coverage is unresolved");
        assert_eq!(report.unresolved_n, 1);
    }

    #[test]
    fn fault_delivery_rejects_same_independence_group() {
        let receipt = base_capsule(
            "fault-receipt",
            SourceRole::FaultInjector,
            "shared-system",
            1,
            EvidencePayload::FaultExecutionReceipt {
                fault_id: "fault-1".to_owned(),
                delivered: true,
            },
        );
        let observation = base_capsule(
            "fault-observation",
            SourceRole::PhysicalObserver,
            "shared-system",
            2,
            EvidencePayload::Observation {
                subject_id: "fault-1".to_owned(),
                finding: ObservationFinding::FaultPresent,
                coverage: None,
                coverage_complete: false,
                raw_data_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
                calibration_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned(),
            },
        );
        let mut claim = claim_for(
            "fault-delivered",
            ClaimType::FaultDelivered,
            "fault-1",
            true,
            vec!["fault-observation".to_owned(), "fault-receipt".to_owned()],
        );
        claim.decision_cutoff_ingest_sequence = 2;
        let bundle = bundle_with(vec![receipt, observation], vec![claim]);
        let serialized = serde_json::to_string(&bundle).expect("bundle serialization");
        let report = evaluate_json(&serialized).expect("pseudo-independent evidence is data");
        assert_eq!(report.unresolved_n, 1);
        assert_eq!(
            report.claim_assessments[0].reason_codes[0],
            "FAULT_OBSERVER_NOT_INDEPENDENT"
        );
    }

    #[test]
    fn missing_parent_is_rejected() {
        let mut bundle = parsed_fixture();
        bundle.evidence[0].parent_evidence_ids = vec!["not-present".to_owned()];
        reseal_bundle(&mut bundle);
        let serialized = serde_json::to_string(&bundle).expect("bundle serialization");
        let error = evaluate_json(&serialized).expect_err("missing parent must reject");
        assert!(error.starts_with("PARENT_EVIDENCE_MISSING"));
    }
    #[test]
    fn fault_delivery_accepts_separate_injector_and_observer_domains() {
        let receipt = base_capsule(
            "fault-receipt",
            SourceRole::FaultInjector,
            "injector-domain",
            1,
            EvidencePayload::FaultExecutionReceipt {
                fault_id: "fault-1".to_owned(),
                delivered: true,
            },
        );
        let mut observation = base_capsule(
            "fault-observation",
            SourceRole::PhysicalObserver,
            "observer-domain",
            2,
            EvidencePayload::Observation {
                subject_id: "fault-1".to_owned(),
                finding: ObservationFinding::FaultPresent,
                coverage: None,
                coverage_complete: false,
                raw_data_sha256: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".to_owned(),
                calibration_sha256: "cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc".to_owned(),
            },
        );
        observation.parent_evidence_ids = vec!["fault-receipt".to_owned()];
        let mut claim = claim_for(
            "fault-delivered",
            ClaimType::FaultDelivered,
            "fault-1",
            true,
            vec!["fault-observation".to_owned(), "fault-receipt".to_owned()],
        );
        claim.decision_cutoff_ingest_sequence = 2;
        let bundle = bundle_with(vec![receipt, observation], vec![claim]);
        let serialized = serde_json::to_string(&bundle).expect("bundle serialization");
        let report =
            evaluate_json(&serialized).expect("independent delivery evidence should qualify");
        assert_eq!(report.qualified_n, 1);
    }

    #[test]
    fn temporal_closure_requires_every_inventoried_holder() {
        let endpoint = |id: &str, holder: &str, sequence| {
            base_capsule(
                id,
                SourceRole::Endpoint,
                &format!("endpoint-{holder}"),
                sequence,
                EvidencePayload::EndpointFenceReceipt {
                    holder_id: holder.to_owned(),
                    accepted_epoch: 7,
                    queue_generation: 3,
                    accepted_high_water: 12,
                    retired_high_water: 12,
                    outstanding_n: 0,
                    in_flight_n: 0,
                    terminal_actuator_fenced: true,
                },
            )
        };
        let controller = endpoint("ep-controller", "controller", 1);
        let actuator = endpoint("ep-actuator", "actuator", 2);
        let mut edge = base_capsule(
            "edge-1",
            SourceRole::NetworkMonitor,
            "network-monitor-domain",
            3,
            EvidencePayload::EdgeClosureReceipt {
                edge_id: "edge-1".to_owned(),
                source_holder_id: "controller".to_owned(),
                target_holder_id: "actuator".to_owned(),
                old_epoch_enqueue_closed: true,
            },
        );
        edge.parent_evidence_ids = vec!["ep-controller".to_owned()];
        let mut inventory = base_capsule(
            "inventory",
            SourceRole::IndependentAdjudicator,
            "inventory-audit-domain",
            4,
            EvidencePayload::HolderInventory {
                complete: true,
                holder_ids: vec!["actuator".to_owned(), "controller".to_owned()],
                edge_ids: vec!["edge-1".to_owned()],
            },
        );
        inventory.parent_evidence_ids = vec![
            "edge-1".to_owned(),
            "ep-actuator".to_owned(),
            "ep-controller".to_owned(),
        ];
        let mut claim = claim_for(
            "closure-claim",
            ClaimType::TemporalClosure,
            "old-epoch-6",
            true,
            vec![
                "edge-1".to_owned(),
                "ep-actuator".to_owned(),
                "ep-controller".to_owned(),
                "inventory".to_owned(),
            ],
        );
        claim.required_epoch = Some(7);
        claim.required_queue_generation = Some(3);
        claim.expected_holder_ids = vec!["actuator".to_owned(), "controller".to_owned()];
        claim.expected_edge_ids = vec!["edge-1".to_owned()];
        claim.decision_cutoff_ingest_sequence = 4;
        let mut bundle = bundle_with(vec![controller, actuator, edge, inventory], vec![claim]);
        let serialized = serde_json::to_string(&bundle).expect("bundle serialization");
        let complete = evaluate_json(&serialized).expect("complete closure should qualify");
        assert_eq!(complete.qualified_n, 1);

        bundle.claims[0]
            .evidence_ids
            .retain(|evidence_id| evidence_id != "ep-actuator");
        reseal_bundle(&mut bundle);
        let incomplete = serde_json::to_string(&bundle).expect("bundle serialization");
        let report = evaluate_json(&incomplete).expect("missing holder evidence is unresolved");
        assert_eq!(report.unresolved_n, 1);
    }
}
