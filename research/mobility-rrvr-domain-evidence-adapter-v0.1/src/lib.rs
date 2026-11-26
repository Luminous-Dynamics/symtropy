// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::Serialize;
use sha2::{Digest, Sha256};

use mobility_rrvr_evidence_capsule::{
    Claim, ClaimState, ClaimType, EvidenceBundle, EvidenceCapsule, EvidenceDomain,
    EvidencePayload, EvidenceReport, EvidenceSource, SourceRole, TimeInterval, evaluate_json,
    seal_bundle,
};
use mobility_rrvr_fault_harness::{
    ArmResult, CampaignReport, Scenario, TraceEntry, known_scenarios, run_campaign,
};

const ADAPTER_SCHEMA_VERSION: &str = "0.1";
const MODEL_PROFILE_ID: &str = "rrvr-process-fault-harness-v0.1";
const MODEL_PROFILE_DESCRIPTION: &str =
    "Deterministic logical-tick process model; no simulator or physical instrumentation.";
const CLAIM_CEILING: &str =
    "Process-model evidence only; no simulator, HIL, or physical robot result.";

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelTraceEntry {
    pub tick: u64,
    pub sequence: u64,
    pub event: String,
    pub detail: String,
}

impl From<&TraceEntry> for ModelTraceEntry {
    fn from(entry: &TraceEntry) -> Self {
        Self {
            tick: entry.tick,
            sequence: entry.sequence,
            event: entry.event.clone(),
            detail: entry.detail.clone(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ModelArmOutcome {
    pub pair_id: String,
    pub arm_id: String,
    pub frozen_input_id: String,
    pub original_command_id: String,
    pub accepted_epoch: u64,
    pub queue_generation: u64,
    pub accepted_high_water: u64,
    pub retired_high_water: u64,
    pub outstanding_n: u32,
    pub in_flight_n: u32,
    pub actual_effect_count: u32,
    pub reported_disposition: String,
    pub fresh_attempt_admitted: bool,
    pub unsafe_admission: bool,
    pub false_no_effect: bool,
    pub false_applied: bool,
    pub false_hold: bool,
    pub endpoint_fenced: bool,
    pub temporal_closure_issued: bool,
    pub temporal_closure_valid: bool,
    pub stale_effect_after_fence: bool,
    pub duplicate_one_shot_effect: bool,
    pub unresolved_outcome: bool,
}

impl From<&ArmResult> for ModelArmOutcome {
    fn from(result: &ArmResult) -> Self {
        Self {
            pair_id: result.pair_id.clone(),
            arm_id: result.arm.as_str().to_owned(),
            frozen_input_id: result.frozen_input_id.clone(),
            original_command_id: result.original_command_id.clone(),
            accepted_epoch: result.accepted_epoch,
            queue_generation: result.queue_generation,
            accepted_high_water: result.accepted_high_water,
            retired_high_water: result.retired_high_water,
            outstanding_n: result.outstanding_n,
            in_flight_n: result.in_flight_n,
            actual_effect_count: result.actual_effect_count,
            reported_disposition: result.reported_disposition.as_str().to_owned(),
            fresh_attempt_admitted: result.fresh_attempt_admitted,
            unsafe_admission: result.unsafe_admission,
            false_no_effect: result.false_no_effect,
            false_applied: result.false_applied,
            false_hold: result.false_hold,
            endpoint_fenced: result.endpoint_fenced,
            temporal_closure_issued: result.temporal_closure_issued,
            temporal_closure_valid: result.temporal_closure_valid,
            stale_effect_after_fence: result.stale_effect_after_fence,
            duplicate_one_shot_effect: result.duplicate_one_shot_effect,
            unresolved_outcome: result.unresolved_outcome,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ArmEvidenceEnvelope {
    pub outcome: ModelArmOutcome,
    pub trace_sha256: String,
    pub trace: Vec<ModelTraceEntry>,
    pub evidence_bundle: EvidenceBundle,
    pub evidence_report: EvidenceReport,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CampaignEnvelope {
    pub schema_version: String,
    pub evidence_domain: EvidenceDomain,
    pub campaign_id: String,
    pub model_profile_id: String,
    pub model_profile_sha256: String,
    pub claim_ceiling: String,
    pub arms: Vec<ArmEvidenceEnvelope>,
    pub artifact_sha256: String,
}

#[derive(Serialize)]
struct CampaignEnvelopeMaterial<'a> {
    schema_version: &'a str,
    evidence_domain: EvidenceDomain,
    campaign_id: &'a str,
    model_profile_id: &'a str,
    model_profile_sha256: &'a str,
    claim_ceiling: &'a str,
    arms: &'a [ArmEvidenceEnvelope],
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut output, "{byte:02x}");
    }
    output
}

fn digest_with_domain(domain: &str, bytes: &[u8]) -> String {
    let mut material = Vec::with_capacity(domain.len() + 1 + bytes.len());
    material.extend_from_slice(domain.as_bytes());
    material.push(0);
    material.extend_from_slice(bytes);
    sha256_hex(&material)
}

fn profile_sha256() -> String {
    digest_with_domain(
        "rrvr-model-profile-v0.1",
        format!("{MODEL_PROFILE_ID}\n{MODEL_PROFILE_DESCRIPTION}").as_bytes(),
    )
}

fn model_source(source_id: String, profile_sha256: &str) -> EvidenceSource {
    EvidenceSource {
        source_id,
        principal_id: MODEL_PROFILE_ID.to_owned(),
        role: SourceRole::ModelOracle,
        independence_group: "single-correlated-process-model".to_owned(),
        profile_sha256: profile_sha256.to_owned(),
    }
}

fn logical_time(clock_id: &str, tick: u64) -> (TimeInterval, TimeInterval) {
    let interval = TimeInterval {
        clock_id: clock_id.to_owned(),
        earliest_ns: tick,
        latest_ns: tick,
    };
    (interval.clone(), interval)
}

struct CapsuleScope<'a> {
    episode_id: &'a str,
    attempt_id: &'a str,
    command_id: &'a str,
    profile_sha256: &'a str,
    clock_id: &'a str,
}

struct CapsuleDraft {
    evidence_id: String,
    source_id: String,
    parent_evidence_ids: Vec<String>,
    ingest_sequence: u64,
    tick: u64,
    payload: EvidencePayload,
}

fn capsule(scope: &CapsuleScope<'_>, draft: CapsuleDraft) -> EvidenceCapsule {
    let (event_time, observation_time) = logical_time(scope.clock_id, draft.tick);
    EvidenceCapsule {
        evidence_id: draft.evidence_id,
        episode_id: scope.episode_id.to_owned(),
        attempt_id: scope.attempt_id.to_owned(),
        command_id: scope.command_id.to_owned(),
        source: model_source(draft.source_id, scope.profile_sha256),
        parent_evidence_ids: draft.parent_evidence_ids,
        event_time,
        observation_time,
        ingest_sequence: draft.ingest_sequence,
        payload: draft.payload,
        content_sha256: String::new(),
    }
}

fn trace_event<'a>(
    trace: &'a [ModelTraceEntry],
    event: &str,
) -> Result<&'a ModelTraceEntry, String> {
    trace
        .iter()
        .find(|entry| entry.event == event)
        .ok_or_else(|| format!("MODEL_TRACE_EVENT_MISSING: {event}"))
}

struct ClaimScope<'a> {
    episode_id: &'a str,
    attempt_id: &'a str,
    command_id: &'a str,
}

struct ClaimSpec {
    claim_id: String,
    claim_type: ClaimType,
    asserted_value: bool,
    subject_id: String,
    required_epoch: Option<u64>,
    required_queue_generation: Option<u64>,
    expected_holder_ids: Vec<String>,
    expected_edge_ids: Vec<String>,
    decision_cutoff_ingest_sequence: u64,
    evidence_ids: Vec<String>,
}

fn make_claim(scope: &ClaimScope<'_>, spec: ClaimSpec) -> Claim {
    Claim {
        claim_id: spec.claim_id,
        episode_id: scope.episode_id.to_owned(),
        attempt_id: scope.attempt_id.to_owned(),
        command_id: scope.command_id.to_owned(),
        claim_type: spec.claim_type,
        asserted_value: spec.asserted_value,
        state: ClaimState::Asserted,
        subject_id: spec.subject_id,
        required_epoch: spec.required_epoch,
        required_queue_generation: spec.required_queue_generation,
        observation_window: None,
        expected_holder_ids: spec.expected_holder_ids,
        expected_edge_ids: spec.expected_edge_ids,
        decision_cutoff_ingest_sequence: spec.decision_cutoff_ingest_sequence,
        evidence_ids: spec.evidence_ids,
        evidence_snapshot_sha256: String::new(),
    }
}

fn adapt_arm(
    campaign_id: &str,
    scenario: &Scenario,
    result: &ArmResult,
) -> Result<ArmEvidenceEnvelope, String> {
    let trace: Vec<ModelTraceEntry> = result.trace.iter().map(ModelTraceEntry::from).collect();
    let trace_bytes = serde_json::to_vec(&trace)
        .map_err(|error| format!("MODEL_TRACE_SERIALIZATION_FAILED: {error}"))?;
    let trace_sha256 = digest_with_domain("rrvr-model-trace-v0.1", &trace_bytes);
    let profile_digest = profile_sha256();
    let prefix = format!("{}-{}", scenario.scenario_id, result.arm.as_str());
    let attempt_id = format!("{prefix}-attempt-1");
    let clock_id = format!("logical:{}:{}", scenario.scenario_id, result.arm.as_str());
    let fence_event = trace_event(&trace, "fence")?;
    let closure_event = trace_event(&trace, "temporal_closure")?;

    let capsule_scope = CapsuleScope {
        episode_id: scenario.scenario_id,
        attempt_id: &attempt_id,
        command_id: &result.original_command_id,
        profile_sha256: &profile_digest,
        clock_id: &clock_id,
    };

    // These are vertices in the deterministic process model's abstract graph,
    // not an asserted inventory of physical robot command paths.
    let holder_ids = vec!["actuator".to_owned(), "controller".to_owned()];
    let edge_ids = vec!["controller-to-actuator".to_owned()];
    let actuator_id = format!("{prefix}-endpoint-actuator");
    let controller_id = format!("{prefix}-endpoint-controller");
    let edge_evidence_id = format!("{prefix}-edge-controller-to-actuator");
    let inventory_id = format!("{prefix}-inventory");

    let endpoint_payload = |holder_id: &str| EvidencePayload::EndpointFenceReceipt {
        holder_id: holder_id.to_owned(),
        accepted_epoch: result.accepted_epoch,
        queue_generation: result.queue_generation,
        accepted_high_water: result.accepted_high_water,
        retired_high_water: result.retired_high_water,
        outstanding_n: result.outstanding_n,
        in_flight_n: result.in_flight_n,
        terminal_actuator_fenced: result.endpoint_fenced,
    };

    let fence_source_id = format!(
        "model-trace-event-{}-{}",
        fence_event.sequence, fence_event.event
    );
    let closure_source_id = format!(
        "model-trace-event-{}-{}",
        closure_event.sequence, closure_event.event
    );

    let actuator_receipt = capsule(
        &capsule_scope,
        CapsuleDraft {
            evidence_id: actuator_id.clone(),
            source_id: fence_source_id.clone(),
            parent_evidence_ids: Vec::new(),
            ingest_sequence: 1,
            tick: fence_event.tick,
            payload: endpoint_payload("actuator"),
        },
    );
    let controller_receipt = capsule(
        &capsule_scope,
        CapsuleDraft {
            evidence_id: controller_id.clone(),
            source_id: fence_source_id,
            parent_evidence_ids: Vec::new(),
            ingest_sequence: 2,
            tick: fence_event.tick,
            payload: endpoint_payload("controller"),
        },
    );
    let edge_receipt = capsule(
        &capsule_scope,
        CapsuleDraft {
            evidence_id: edge_evidence_id.clone(),
            source_id: closure_source_id.clone(),
            parent_evidence_ids: vec![actuator_id.clone(), controller_id.clone()],
            ingest_sequence: 3,
            tick: closure_event.tick,
            payload: EvidencePayload::EdgeClosureReceipt {
                edge_id: edge_ids[0].clone(),
                source_holder_id: "controller".to_owned(),
                target_holder_id: "actuator".to_owned(),
                authority_epoch: result.accepted_epoch,
                queue_generation: result.queue_generation,
                old_epoch_enqueue_closed: result.endpoint_fenced,
            },
        },
    );
    let inventory_complete =
        scenario.holder_inventory_complete && scenario.edge_inventory_complete;
    let inventory_receipt = capsule(
        &capsule_scope,
        CapsuleDraft {
            evidence_id: inventory_id.clone(),
            source_id: closure_source_id,
            parent_evidence_ids: vec![
                edge_evidence_id.clone(),
                actuator_id.clone(),
                controller_id.clone(),
            ],
            ingest_sequence: 4,
            tick: closure_event.tick,
            payload: EvidencePayload::HolderInventory {
                complete: inventory_complete,
                holder_ids: holder_ids.clone(),
                edge_ids: edge_ids.clone(),
            },
        },
    );

    let mut evidence_ids = vec![
        actuator_id.clone(),
        controller_id.clone(),
        edge_evidence_id.clone(),
        inventory_id,
    ];
    evidence_ids.sort();

    let claim_scope = ClaimScope {
        episode_id: scenario.scenario_id,
        attempt_id: &attempt_id,
        command_id: &result.original_command_id,
    };
    let endpoint_claim = make_claim(
        &claim_scope,
        ClaimSpec {
            claim_id: format!("{prefix}-endpoint-fenced"),
            claim_type: ClaimType::EndpointFenced,
            asserted_value: result.endpoint_fenced,
            subject_id: "actuator".to_owned(),
            required_epoch: Some(result.accepted_epoch),
            required_queue_generation: Some(result.queue_generation),
            expected_holder_ids: Vec::new(),
            expected_edge_ids: Vec::new(),
            decision_cutoff_ingest_sequence: 4,
            evidence_ids: vec![actuator_id],
        },
    );
    let closure_claim = make_claim(
        &claim_scope,
        ClaimSpec {
            claim_id: format!("{prefix}-temporal-closure"),
            claim_type: ClaimType::TemporalClosure,
            asserted_value: result.temporal_closure_valid,
            subject_id: "old-epoch-command".to_owned(),
            required_epoch: Some(result.accepted_epoch),
            required_queue_generation: Some(result.queue_generation),
            expected_holder_ids: holder_ids,
            expected_edge_ids: edge_ids,
            decision_cutoff_ingest_sequence: 4,
            evidence_ids,
        },
    );

    let mut evidence_bundle = EvidenceBundle {
        schema_version: "0.1".to_owned(),
        benchmark: "RRVR-EVIDENCE-CAPSULES".to_owned(),
        campaign_id: campaign_id.to_owned(),
        claim_ceiling: CLAIM_CEILING.to_owned(),
        evidence_domain: Some(EvidenceDomain::ProcessModel),
        evidence: vec![
            actuator_receipt,
            controller_receipt,
            edge_receipt,
            inventory_receipt,
        ],
        claims: vec![endpoint_claim, closure_claim],
    };
    seal_bundle(&mut evidence_bundle)?;

    let bundle_json = serde_json::to_string(&evidence_bundle)
        .map_err(|error| format!("EVIDENCE_BUNDLE_SERIALIZATION_FAILED: {error}"))?;
    let evidence_report = evaluate_json(&bundle_json)?;

    Ok(ArmEvidenceEnvelope {
        outcome: ModelArmOutcome::from(result),
        trace_sha256,
        trace,
        evidence_bundle,
        evidence_report,
    })
}

fn unsigned_material<'a>(
    campaign_id: &'a str,
    model_profile_sha256: &'a str,
    claim_ceiling: &'a str,
    arms: &'a [ArmEvidenceEnvelope],
) -> CampaignEnvelopeMaterial<'a> {
    CampaignEnvelopeMaterial {
        schema_version: ADAPTER_SCHEMA_VERSION,
        evidence_domain: EvidenceDomain::ProcessModel,
        campaign_id,
        model_profile_id: MODEL_PROFILE_ID,
        model_profile_sha256,
        claim_ceiling,
        arms,
    }
}

pub fn adapt_campaign(campaign: &CampaignReport) -> Result<CampaignEnvelope, String> {
    let scenarios = known_scenarios();
    let mut arms = Vec::with_capacity(campaign.pairs.len() * 2);
    for pair in &campaign.pairs {
        let scenario = scenarios
            .iter()
            .find(|scenario| scenario.scenario_id == pair.pair_id)
            .ok_or_else(|| format!("SCENARIO_MANIFEST_ENTRY_MISSING: {}", pair.pair_id))?;
        arms.push(adapt_arm(campaign.campaign_id, scenario, &pair.native)?);
        arms.push(adapt_arm(campaign.campaign_id, scenario, &pair.rrvr)?);
    }

    let model_digest = profile_sha256();
    let material = unsigned_material(campaign.campaign_id, &model_digest, CLAIM_CEILING, &arms);
    let material_bytes = serde_json::to_vec(&material)
        .map_err(|error| format!("CAMPAIGN_ENVELOPE_SERIALIZATION_FAILED: {error}"))?;
    let artifact_sha256 =
        digest_with_domain("rrvr-process-model-campaign-envelope-v0.1", &material_bytes);

    let envelope = CampaignEnvelope {
        schema_version: ADAPTER_SCHEMA_VERSION.to_owned(),
        evidence_domain: EvidenceDomain::ProcessModel,
        campaign_id: campaign.campaign_id.to_owned(),
        model_profile_id: MODEL_PROFILE_ID.to_owned(),
        model_profile_sha256: model_digest,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        arms,
        artifact_sha256,
    };
    validate_envelope(&envelope)?;
    Ok(envelope)
}

pub fn adapt_known_campaign() -> Result<CampaignEnvelope, String> {
    adapt_campaign(&run_campaign())
}

pub fn validate_envelope(envelope: &CampaignEnvelope) -> Result<(), String> {
    if envelope.schema_version != ADAPTER_SCHEMA_VERSION {
        return Err("ADAPTER_SCHEMA_VERSION_UNSUPPORTED".to_owned());
    }
    if envelope.evidence_domain != EvidenceDomain::ProcessModel {
        return Err("ADAPTER_DOMAIN_MISMATCH".to_owned());
    }
    if envelope.model_profile_id != MODEL_PROFILE_ID
        || envelope.model_profile_sha256 != profile_sha256()
    {
        return Err("MODEL_PROFILE_MISMATCH".to_owned());
    }
    if envelope.claim_ceiling != CLAIM_CEILING {
        return Err("CLAIM_CEILING_MISMATCH".to_owned());
    }
    if envelope.arms.is_empty() || !envelope.arms.len().is_multiple_of(2) {
        return Err("PROCESS_MODEL_ARM_CARDINALITY_INVALID".to_owned());
    }

    for arm in &envelope.arms {
        if arm.evidence_bundle.evidence_domain != Some(EvidenceDomain::ProcessModel) {
            return Err("NESTED_EVIDENCE_DOMAIN_MISMATCH".to_owned());
        }
        if arm.evidence_bundle.evidence.iter().any(|capsule| {
            capsule.source.role != SourceRole::ModelOracle
                || capsule.source.independence_group != "single-correlated-process-model"
        }) {
            return Err("MODEL_SOURCE_PROVENANCE_MISMATCH".to_owned());
        }
        if arm.evidence_bundle.claims.iter().any(|claim| {
            !matches!(
                claim.claim_type,
                ClaimType::EndpointFenced | ClaimType::TemporalClosure
            )
        }) {
            return Err("PROCESS_MODEL_CLAIM_TYPE_FORBIDDEN".to_owned());
        }
        let trace_bytes = serde_json::to_vec(&arm.trace)
            .map_err(|error| format!("MODEL_TRACE_SERIALIZATION_FAILED: {error}"))?;
        let expected_trace_digest = digest_with_domain("rrvr-model-trace-v0.1", &trace_bytes);
        if expected_trace_digest != arm.trace_sha256 {
            return Err("MODEL_TRACE_DIGEST_MISMATCH".to_owned());
        }
        let bundle_json = serde_json::to_string(&arm.evidence_bundle)
            .map_err(|error| format!("EVIDENCE_BUNDLE_SERIALIZATION_FAILED: {error}"))?;
        let assessed = evaluate_json(&bundle_json)?;
        if assessed != arm.evidence_report {
            return Err("EVIDENCE_ASSESSMENT_REPORT_MISMATCH".to_owned());
        }
        if arm.outcome.pair_id != arm.evidence_bundle.evidence[0].episode_id {
            return Err("MODEL_OUTCOME_EVIDENCE_SCOPE_MISMATCH".to_owned());
        }
        if arm.outcome.original_command_id != arm.evidence_bundle.evidence[0].command_id {
            return Err("MODEL_COMMAND_ID_SCOPE_MISMATCH".to_owned());
        }
    }

    let material = unsigned_material(
        &envelope.campaign_id,
        &envelope.model_profile_sha256,
        &envelope.claim_ceiling,
        &envelope.arms,
    );
    let material_bytes = serde_json::to_vec(&material)
        .map_err(|error| format!("CAMPAIGN_ENVELOPE_SERIALIZATION_FAILED: {error}"))?;
    let expected = digest_with_domain("rrvr-process-model-campaign-envelope-v0.1", &material_bytes);
    if expected != envelope.artifact_sha256 {
        return Err("CAMPAIGN_ENVELOPE_DIGEST_MISMATCH".to_owned());
    }
    Ok(())
}

pub fn envelope_json(envelope: &CampaignEnvelope) -> Result<String, String> {
    validate_envelope(envelope)?;
    serde_json::to_string_pretty(envelope)
        .map_err(|error| format!("CAMPAIGN_ENVELOPE_SERIALIZATION_FAILED: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use mobility_rrvr_evidence_capsule::AssessmentStatus;

    fn envelope() -> CampaignEnvelope {
        adapt_known_campaign().expect("known process-model campaign must adapt")
    }

    #[test]
    fn known_campaign_emits_twelve_domain_qualified_arm_records() {
        let output = envelope();
        assert_eq!(output.arms.len(), 12);
        assert_eq!(output.evidence_domain, EvidenceDomain::ProcessModel);
        assert!(validate_envelope(&output).is_ok());
    }

    #[test]
    fn every_arm_uses_model_only_claims_and_real_evidence_validator() {
        let output = envelope();
        for arm in &output.arms {
            assert_eq!(arm.evidence_report.claim_n, 2);
            assert_eq!(
                arm.evidence_bundle.evidence_domain,
                Some(EvidenceDomain::ProcessModel)
            );
            assert!(arm.evidence_bundle.evidence.iter().all(|capsule| {
                capsule.source.role == SourceRole::ModelOracle
                    && capsule.source.independence_group == "single-correlated-process-model"
            }));
            assert!(arm.evidence_bundle.claims.iter().all(|claim| {
                matches!(
                    claim.claim_type,
                    ClaimType::EndpointFenced | ClaimType::TemporalClosure
                )
            }));
            assert!(arm.trace.len() >= 12);
        }
    }

    #[test]
    fn process_model_cannot_emit_physical_effect_claims() {
        let mut bundle = envelope().arms[0].evidence_bundle.clone();
        bundle.claims[0].claim_type = ClaimType::EffectApplied;
        let error = seal_bundle(&mut bundle).expect_err("physical effect claim is forbidden");
        assert_eq!(error, "PROCESS_MODEL_CLAIM_TYPE_FORBIDDEN");
    }

    #[test]
    fn model_oracle_without_explicit_domain_is_rejected() {
        let mut bundle = envelope().arms[0].evidence_bundle.clone();
        bundle.evidence_domain = None;
        let json = serde_json::to_string(&bundle).expect("bundle serializes");
        let error = evaluate_json(&json).expect_err("model oracle requires domain");
        assert_eq!(error, "MODEL_ORACLE_DOMAIN_REQUIRED");
    }

    #[test]
    fn incomplete_graph_stays_unresolved_in_nested_evidence_assessment() {
        let output = envelope();
        let arm = output
            .arms
            .iter()
            .find(|arm| {
                arm.outcome.pair_id == "incomplete_command_graph" && arm.outcome.arm_id == "rrvr"
            })
            .expect("incomplete graph RRVR arm exists");
        assert!(arm.evidence_report.unresolved_n >= 1);
        assert_eq!(
            arm.evidence_report.claim_assessments[1].status,
            AssessmentStatus::Unresolved
        );
    }

    #[test]
    fn applied_effect_outcome_remains_model_record_not_physical_claim() {
        let output = envelope();
        let arm = output
            .arms
            .iter()
            .find(|arm| {
                arm.outcome.pair_id == "effect_then_lost_ack" && arm.outcome.arm_id == "rrvr"
            })
            .expect("applied effect RRVR arm exists");
        assert_eq!(arm.outcome.actual_effect_count, 1);
        assert_eq!(arm.outcome.reported_disposition, "applied");
        assert!(arm.evidence_bundle.claims.iter().all(|claim| {
            claim.claim_type != ClaimType::EffectApplied && claim.claim_type != ClaimType::NoEffect
        }));
    }

    #[test]
    fn trace_or_outcome_tampering_is_detected_by_envelope_digest() {
        let mut output = envelope();
        output.arms[0].trace[0].detail.push_str("-tampered");
        let error = validate_envelope(&output).expect_err("trace digest mutation rejected");
        assert_eq!(error, "MODEL_TRACE_DIGEST_MISMATCH");
    }
}
