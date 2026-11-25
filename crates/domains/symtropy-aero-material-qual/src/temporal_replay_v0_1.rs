// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Research-only reference model for AERO temporal replay v0.1.
//!
//! This module exercises the frozen synthetic replay corpus. It is not wired
//! into the production v0.2 calibration evaluator and does not verify digital
//! signatures, RFC 3161 tokens, trusted clocks, source receipts, physical
//! measurement truth, metrological traceability, or engineering qualification.
//! The corpus is intentionally marked NOT_INDEPENDENTLY_REVIEWED.

use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

const CORPUS: &str = include_str!(
    "../../../../docs/research/aviation/examples/aero_temporal_replay_corpus_v0_1.json"
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ReplayMode {
    AsKnownAt,
    RecomputedWithCurrentTrust,
    RecomputedWithCurrentEvidenceAndTrust,
}

impl ReplayMode {
    fn expected_key(self) -> &'static str {
        match self {
            Self::AsKnownAt => "as_known_at",
            Self::RecomputedWithCurrentTrust => "recomputed_with_current_trust",
            Self::RecomputedWithCurrentEvidenceAndTrust => {
                "recomputed_with_current_evidence_and_trust"
            }
        }
    }

    fn uses_current_trust(self) -> bool {
        !matches!(self, Self::AsKnownAt)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TemporalDisposition {
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub applied_event_ids: Vec<String>,
    pub excluded_event_ids: Vec<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct AcquisitionTimeInterval {
    pub earliest_unix_ms: u64,
    pub latest_unix_ms: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReplayCoordinate {
    pub mode: String,
    pub subject_time_interval: AcquisitionTimeInterval,
    pub receiver_receipt_watermark_unix_ms: u64,
    pub evidence_checkpoint_reference: String,
    pub evidence_checkpoint_digest_sha256: String,
    pub evidence_view_digest_sha256: String,
    pub evidence_checkpoint_watermark_unix_ms: u64,
    pub observer_coverage_status: String,
    pub observer_coverage_profile_id: String,
    pub observer_coverage_receipt_reference: String,
    pub observer_coverage_receipt_digest_sha256: String,
    pub observer_coverage_watermark_unix_ms: u64,
    pub trust_status_as_of_unix_ms: u64,
    pub trust_policy_id: String,
    pub trust_policy_digest_sha256: String,
    pub verifier_profile_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct TemporalReplayReport {
    pub coordinate: ReplayCoordinate,
    pub disposition: TemporalDisposition,
}

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn nested_number(value: &Value, path: &str) -> Option<u64> {
    value.pointer(path).and_then(Value::as_u64)
}

fn event_key(event: &Value) -> (String, String) {
    (
        text(event, "source_namespace").unwrap_or_default().to_owned(),
        text(event, "event_id").unwrap_or_default().to_owned(),
    )
}

fn display_id(events: &[&Value], event: &Value) -> String {
    let id = text(event, "event_id").unwrap_or_default();
    let same_id = events
        .iter()
        .copied()
        .filter(|candidate| text(candidate, "event_id") == Some(id))
        .collect::<Vec<_>>();
    let namespaces = same_id
        .iter()
        .filter_map(|candidate| text(candidate, "source_namespace"))
        .collect::<BTreeSet<_>>();
    let payloads = same_id
        .iter()
        .filter_map(|candidate| text(candidate, "payload_sha256"))
        .collect::<BTreeSet<_>>();
    if namespaces.len() > 1 || (same_id.len() > 1 && payloads.len() == 1) {
        format!("{}:{id}", text(event, "source_namespace").unwrap_or_default())
    } else {
        id.to_owned()
    }
}

fn sorted_ids(events: &[&Value], all_events: &[&Value]) -> Vec<String> {
    events
        .iter()
        .map(|event| display_id(all_events, event))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn disposition(
    status: &str,
    reasons: impl IntoIterator<Item = String>,
    applied: &[&Value],
    excluded: &[&Value],
    all_events: &[&Value],
) -> TemporalDisposition {
    TemporalDisposition {
        disposition: status.to_owned(),
        reason_codes: reasons.into_iter().collect::<BTreeSet<_>>().into_iter().collect(),
        applied_event_ids: sorted_ids(applied, all_events),
        excluded_event_ids: sorted_ids(excluded, all_events),
    }
}

fn one_reason(
    status: &str,
    reason: &str,
    applied: &[&Value],
    excluded: &[&Value],
    all_events: &[&Value],
) -> TemporalDisposition {
    disposition(
        status,
        [reason.to_owned()],
        applied,
        excluded,
        all_events,
    )
}

fn is_effective_revocation(event: &Value, subject_time: u64) -> bool {
    matches!(text(event, "event_type"), Some("revoked" | "suspended"))
        && number(event, "effective_at_unix_ms").is_some_and(|effective| effective <= subject_time)
}

fn has_correction_cycle(events: &[&Value]) -> bool {
    let mut edges = BTreeMap::<(String, String), (String, String)>::new();
    for event in events {
        if let Some(target) = text(event, "corrects_event_id") {
            if !target.is_empty() {
                let source = event_key(event);
                edges.insert(
                    source.clone(),
                    (source.0.clone(), target.to_owned()),
                );
            }
        }
    }

    fn visit(
        node: &(String, String),
        edges: &BTreeMap<(String, String), (String, String)>,
        visiting: &mut BTreeSet<(String, String)>,
        visited: &mut BTreeSet<(String, String)>,
    ) -> bool {
        if visiting.contains(node) {
            return true;
        }
        if visited.contains(node) {
            return false;
        }
        let Some(target) = edges.get(node) else {
            return false;
        };
        visiting.insert(node.clone());
        let found = edges.contains_key(target)
            && visit(target, edges, visiting, visited);
        visiting.remove(node);
        visited.insert(node.clone());
        found
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    edges.keys().any(|node| visit(node, &edges, &mut visiting, &mut visited))
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

/// Construct the complete historical replay coordinate for a case and mode.
///
/// A report without this coordinate is not a reproducible historical result.
pub fn replay_coordinate(case: &Value, mode: ReplayMode) -> Option<ReplayCoordinate> {
    let subject_time_interval = AcquisitionTimeInterval {
        earliest_unix_ms: nested_number(case, "/acquisition_time_interval/earliest_unix_ms")?,
        latest_unix_ms: nested_number(case, "/acquisition_time_interval/latest_unix_ms")?,
    };
    if subject_time_interval.earliest_unix_ms > subject_time_interval.latest_unix_ms {
        return None;
    }
    let receipt_path = match mode {
        ReplayMode::AsKnownAt | ReplayMode::RecomputedWithCurrentTrust => {
            "/snapshot/receiver_receipt_watermark_unix_ms"
        }
        ReplayMode::RecomputedWithCurrentEvidenceAndTrust => {
            "/current_reassessment/receiver_receipt_watermark_unix_ms"
        }
    };
    let (trust_path, policy_id_path, policy_digest_path) = match mode {
        ReplayMode::AsKnownAt => (
            "/snapshot/trust_status_as_of_unix_ms",
            "/snapshot/trust_policy_id",
            "/snapshot/trust_policy_digest_sha256",
        ),
        ReplayMode::RecomputedWithCurrentTrust
        | ReplayMode::RecomputedWithCurrentEvidenceAndTrust => (
            "/current_reassessment/trust_status_as_of_unix_ms",
            "/current_reassessment/trust_policy_id",
            "/current_reassessment/trust_policy_digest_sha256",
        ),
    };
    let view_path = match mode {
        ReplayMode::AsKnownAt | ReplayMode::RecomputedWithCurrentTrust => "/snapshot",
        ReplayMode::RecomputedWithCurrentEvidenceAndTrust => "/current_reassessment",
    };
    let view_path = match mode {
        ReplayMode::AsKnownAt | ReplayMode::RecomputedWithCurrentTrust => "/snapshot",
        ReplayMode::RecomputedWithCurrentEvidenceAndTrust => "/current_reassessment",
    };
    let receiver_receipt_watermark_unix_ms = case.pointer(receipt_path)?.as_u64()?;
    let evidence_checkpoint_watermark_unix_ms = case
        .pointer(&format!("{view_path}/evidence_checkpoint_watermark_unix_ms"))?
        .as_u64()?;
    let observer_coverage_watermark_unix_ms = case
        .pointer(&format!("{view_path}/observer_coverage_watermark_unix_ms"))?
        .as_u64()?;
    let evidence_checkpoint_reference = case
        .pointer(&format!("{view_path}/evidence_checkpoint_reference"))?
        .as_str()?
        .to_owned();
    let evidence_checkpoint_digest_sha256 = case
        .pointer(&format!("{view_path}/evidence_checkpoint_digest_sha256"))?
        .as_str()?
        .to_owned();
    let evidence_view_digest_sha256 = case
        .pointer(&format!("{view_path}/evidence_view_digest_sha256"))?
        .as_str()?
        .to_owned();
    let observer_coverage_status = case
        .pointer(&format!("{view_path}/observer_coverage_status"))?
        .as_str()?
        .to_owned();
    let observer_coverage_profile_id = case
        .pointer(&format!("{view_path}/observer_coverage_profile_id"))?
        .as_str()?
        .to_owned();
    let observer_coverage_receipt_reference = case
        .pointer(&format!("{view_path}/observer_coverage_receipt_reference"))?
        .as_str()?
        .to_owned();
    let observer_coverage_receipt_digest_sha256 = case
        .pointer(&format!("{view_path}/observer_coverage_receipt_digest_sha256"))?
        .as_str()?
        .to_owned();
    let trust_status_as_of_unix_ms = case.pointer(trust_path)?.as_u64()?;
    let trust_policy_id = case.pointer(policy_id_path)?.as_str()?.to_owned();
    let trust_policy_digest_sha256 = case.pointer(policy_digest_path)?.as_str()?.to_owned();
    let verifier_profile_id = text(case, "verifier_profile_id")?.to_owned();
    if trust_policy_id.trim().is_empty()
        || verifier_profile_id.trim().is_empty()
        || !valid_sha256(&trust_policy_digest_sha256)
    {
        return None;
    }
    Some(ReplayCoordinate {
        mode: match mode {
            ReplayMode::AsKnownAt => "AS_KNOWN_AT",
            ReplayMode::RecomputedWithCurrentTrust => "RECOMPUTED_WITH_CURRENT_TRUST",
            ReplayMode::RecomputedWithCurrentEvidenceAndTrust => {
                "RECOMPUTED_WITH_CURRENT_EVIDENCE_AND_TRUST"
            }
        }
        .to_owned(),
        subject_time_interval,
        receiver_receipt_watermark_unix_ms,
        evidence_checkpoint_reference,
        evidence_checkpoint_digest_sha256,
        evidence_view_digest_sha256,
        evidence_checkpoint_watermark_unix_ms,
        observer_coverage_status,
        observer_coverage_profile_id,
        observer_coverage_receipt_reference,
        observer_coverage_receipt_digest_sha256,
        observer_coverage_watermark_unix_ms,
        trust_status_as_of_unix_ms,
        trust_policy_id,
        trust_policy_digest_sha256,
        verifier_profile_id,
    })
}

/// Evaluate a case and retain the complete three-axis replay coordinate.
///
/// This model is synthetic-only and does not authenticate the policy digest or
/// verifier binary. Those require owner-native immutable policy and execution
/// evidence in a production adapter.
pub fn evaluate_report(case: &Value, mode: ReplayMode) -> Option<TemporalReplayReport> {
    Some(TemporalReplayReport {
        coordinate: replay_coordinate(case, mode)?,
        disposition: evaluate_case(case, mode),
    })
}

/// Evaluate one synthetic temporal case at one of the three replay coordinates.
///
/// The model is deterministic and fail-closed. It intentionally treats the
/// synthetic receipt and namespace-binding labels as test inputs, not evidence
/// of real cryptographic or observer verification.
pub fn evaluate_case(case: &Value, mode: ReplayMode) -> TemporalDisposition {
    let all_events = case
        .get("events")
        .and_then(Value::as_array)
        .map(|items| items.iter().collect::<Vec<_>>())
        .unwrap_or_default();

    let Some(valid_from) = nested_number(case, "/calibration_validity/valid_from_unix_ms") else {
        return one_reason(
            "REJECT",
            "CALIBRATION_START_MISSING",
            &[],
            &all_events,
            &all_events,
        );
    };
    let Some(valid_until) = nested_number(case, "/calibration_validity/valid_until_unix_ms") else {
        return one_reason(
            "REJECT",
            "CALIBRATION_END_MISSING",
            &[],
            &all_events,
            &all_events,
        );
    };
    let Some(acquisition_start) =
        nested_number(case, "/acquisition_time_interval/earliest_unix_ms")
    else {
        return one_reason(
            "REJECT",
            "ACQUISITION_START_MISSING",
            &[],
            &all_events,
            &all_events,
        );
    };
    let Some(acquisition_end) =
        nested_number(case, "/acquisition_time_interval/latest_unix_ms")
    else {
        return one_reason(
            "REJECT",
            "ACQUISITION_END_MISSING",
            &[],
            &all_events,
            &all_events,
        );
    };

    if valid_from >= valid_until {
        return one_reason(
            "REJECT",
            "CALIBRATION_INTERVAL_INVALID",
            &[],
            &all_events,
            &all_events,
        );
    }
    if acquisition_start > acquisition_end {
        return one_reason(
            "REJECT",
            "ACQUISITION_INTERVAL_INVALID",
            &[],
            &all_events,
            &all_events,
        );
    }
    if (acquisition_start < valid_from && acquisition_end >= valid_from)
        || (acquisition_start < valid_until && acquisition_end >= valid_until)
    {
        return one_reason(
            "TIME_BOUNDARY_AMBIGUOUS",
            "ACQUISITION_INTERVAL_CROSSES_CALIBRATION_BOUNDARY",
            &[],
            &all_events,
            &all_events,
        );
    }
    if acquisition_end < valid_from || acquisition_start >= valid_until {
        return one_reason(
            "BLOCK",
            "ACQUISITION_OUTSIDE_CALIBRATION_VALIDITY",
            &[],
            &all_events,
            &all_events,
        );
    }

    let receipt_cutoff = match mode {
        ReplayMode::AsKnownAt | ReplayMode::RecomputedWithCurrentTrust => {
            nested_number(case, "/snapshot/receiver_receipt_watermark_unix_ms")
        }
        ReplayMode::RecomputedWithCurrentEvidenceAndTrust => {
            nested_number(case, "/current_reassessment/receiver_receipt_watermark_unix_ms")
        }
    };
    let Some(receipt_cutoff) = receipt_cutoff else {
        return one_reason(
            "REJECT",
            "RECEIVER_WATERMARK_MISSING",
            &[],
            &all_events,
            &all_events,
        );
    };
    let trust_cutoff = match mode {
        ReplayMode::AsKnownAt => nested_number(case, "/snapshot/trust_status_as_of_unix_ms"),
        ReplayMode::RecomputedWithCurrentTrust
        | ReplayMode::RecomputedWithCurrentEvidenceAndTrust => {
            nested_number(case, "/current_reassessment/trust_status_as_of_unix_ms")
        }
    };
    let Some(trust_cutoff) = trust_cutoff else {
        return one_reason(
            "REJECT",
            "TRUST_SNAPSHOT_MISSING",
            &[],
            &all_events,
            &all_events,
        );
    };

    let view_path = match mode {
        ReplayMode::AsKnownAt | ReplayMode::RecomputedWithCurrentTrust => "/snapshot",
        ReplayMode::RecomputedWithCurrentEvidenceAndTrust => "/current_reassessment",
    };
    let checkpoint_status = case
        .pointer(&format!("{view_path}/evidence_checkpoint_status"))
        .and_then(Value::as_str);
    match checkpoint_status {
        Some("SYNTHETIC_VALID") => {
            let checkpoint_reference = case
                .pointer(&format!("{view_path}/evidence_checkpoint_reference"))
                .and_then(Value::as_str);
            let checkpoint_digest = case
                .pointer(&format!("{view_path}/evidence_checkpoint_digest_sha256"))
                .and_then(Value::as_str);
            let view_digest = case
                .pointer(&format!("{view_path}/evidence_view_digest_sha256"))
                .and_then(Value::as_str);
            let checkpoint_watermark = case
                .pointer(&format!("{view_path}/evidence_checkpoint_watermark_unix_ms"))
                .and_then(Value::as_u64);
            if checkpoint_reference.is_none_or(str::is_empty)
                || checkpoint_digest.is_none_or(|digest| !valid_sha256(digest))
                || view_digest.is_none_or(|digest| !valid_sha256(digest))
                || checkpoint_digest != view_digest
                || checkpoint_watermark != Some(receipt_cutoff)
            {
                return one_reason(
                    "REJECT",
                    "EVIDENCE_CHECKPOINT_DIGEST_MISMATCH",
                    &[],
                    &[],
                    &all_events,
                );
            }
        }
        Some("INVALID") => {
            return one_reason(
                "REJECT",
                "EVIDENCE_CHECKPOINT_INVALID",
                &[],
                &[],
                &all_events,
            );
        }
        Some("UNVERIFIED") => {
            return one_reason(
                "TRUST_UNRESOLVED",
                "EVIDENCE_CHECKPOINT_UNVERIFIED",
                &[],
                &[],
                &all_events,
            );
        }
        _ => {
            return one_reason(
                "TRUST_UNRESOLVED",
                "EVIDENCE_CHECKPOINT_MISSING",
                &[],
                &[],
                &all_events,
            );
        }
    }

    let coverage_status = case
        .pointer(&format!("{view_path}/observer_coverage_status"))
        .and_then(Value::as_str);
    match coverage_status {
        Some("SYNTHETIC_QUALIFIED_COMPLETE") => {
            let profile_id = case
                .pointer(&format!("{view_path}/observer_coverage_profile_id"))
                .and_then(Value::as_str);
            let receipt_reference = case
                .pointer(&format!("{view_path}/observer_coverage_receipt_reference"))
                .and_then(Value::as_str);
            let receipt_digest = case
                .pointer(&format!("{view_path}/observer_coverage_receipt_digest_sha256"))
                .and_then(Value::as_str);
            let coverage_watermark = case
                .pointer(&format!("{view_path}/observer_coverage_watermark_unix_ms"))
                .and_then(Value::as_u64);
            if profile_id.is_none_or(str::is_empty)
                || receipt_reference.is_none_or(str::is_empty)
                || receipt_digest.is_none_or(|digest| !valid_sha256(digest))
                || coverage_watermark != Some(receipt_cutoff)
            {
                return one_reason(
                    "ABSTAIN",
                    "OBSERVER_COVERAGE_RECEIPT_INVALID",
                    &[],
                    &[],
                    &all_events,
                );
            }
        }
        Some("INCOMPLETE") => {
            return one_reason(
                "ABSTAIN",
                "OBSERVER_COVERAGE_INCOMPLETE",
                &[],
                &[],
                &all_events,
            );
        }
        Some("UNVERIFIED") => {
            return one_reason(
                "TRUST_UNRESOLVED",
                "OBSERVER_COVERAGE_UNQUALIFIED",
                &[],
                &[],
                &all_events,
            );
        }
        _ => {
            return one_reason(
                "TRUST_UNRESOLVED",
                "OBSERVER_COVERAGE_MISSING",
                &[],
                &[],
                &all_events,
            );
        }
    }

    // Invalid cryptographic claims are rejected when the corresponding event
    // is in the selected evidence view. This model only checks declared test
    // status and digest equality; it does not verify a real token or signature.
    let included = all_events
        .iter()
        .copied()
        .filter(|event| {
            number(event, "receiver_first_seen_at_unix_ms")
                .is_some_and(|seen| seen <= receipt_cutoff)
        })
        .collect::<Vec<_>>();
    let not_yet_received = all_events
        .iter()
        .copied()
        .filter(|event| {
            number(event, "receiver_first_seen_at_unix_ms")
                .is_some_and(|seen| seen > receipt_cutoff)
        })
        .collect::<Vec<_>>();

    if included.is_empty() && !not_yet_received.is_empty() {
        return one_reason(
            "NOT_KNOWN_AT_SNAPSHOT",
            "EVENT_RECEIVED_AFTER_SNAPSHOT",
            &[],
            &not_yet_received,
            &all_events,
        );
    }

    for event in &included {
        if text(event, "trusted_timestamp_status") == Some("invalid_digest") {
            return one_reason(
                "REJECT",
                "TIMESTAMP_IMPRINT_MISMATCH",
                &[],
                &included,
                &all_events,
            );
        }
        if text(event, "trusted_timestamp_status") == Some("invalid_token") {
            return one_reason(
                "REJECT",
                "TIMESTAMP_TOKEN_INVALID",
                &[],
                &included,
                &all_events,
            );
        }
        if text(event, "trusted_timestamp_status") == Some("valid") {
            if text(event, "trusted_timestamp_imprint_sha256")
                != text(event, "payload_sha256")
            {
                return one_reason(
                    "REJECT",
                    "TIMESTAMP_IMPRINT_MISMATCH",
                    &[],
                    &included,
                    &all_events,
                );
            }
            if number(event, "trusted_timestamp_at_unix_ms").is_none() {
                return one_reason(
                    "REJECT",
                    "TIMESTAMP_TOKEN_TIME_MISSING",
                    &[],
                    &included,
                    &all_events,
                );
            }
        }
        if text(event, "trusted_timestamp_status") == Some("absent")
            && (text(event, "trusted_timestamp_imprint_sha256").is_some()
                || number(event, "trusted_timestamp_at_unix_ms").is_some())
        {
            return one_reason(
                "REJECT",
                "ABSENT_TIMESTAMP_HAS_TOKEN_FIELDS",
                &[],
                &included,
                &all_events,
            );
        }
        if text(event, "trusted_timestamp_status") == Some("unknown") {
            return one_reason(
                "TRUST_UNRESOLVED",
                "TIMESTAMP_TOKEN_STATUS_UNKNOWN",
                &[],
                &included,
                &all_events,
            );
        }
        if text(event, "signature_status") == Some("invalid") {
            return one_reason(
                "REJECT",
                "EVENT_SIGNATURE_INVALID",
                &[],
                &included,
                &all_events,
            );
        }
        if text(event, "source_namespace_binding_status") != Some("SYNTHETIC_VALID") {
            return one_reason(
                "TRUST_UNRESOLVED",
                "SOURCE_NAMESPACE_BINDING_UNVERIFIED",
                &[],
                &included,
                &all_events,
            );
        }
        let receipt_status = text(event, "receiver_receipt_status");
        if receipt_status != Some("SYNTHETIC_VALID") {
            let reason = if receipt_status == Some("INVALID") {
                "RECEIVER_RECEIPT_INVALID"
            } else {
                "RECEIVER_RECEIPT_NOT_VERIFIABLE"
            };
            let status = if receipt_status == Some("INVALID") {
                "REJECT"
            } else {
                "TRUST_UNRESOLVED"
            };
            return one_reason(status, reason, &[], &included, &all_events);
        }
        if text(event, "receiver_receipt_reference").is_none_or(str::is_empty)
            || text(event, "receiver_receipt_digest_sha256")
                != text(event, "payload_sha256")
        {
            return one_reason(
                "REJECT",
                "RECEIVER_RECEIPT_DIGEST_MISMATCH",
                &[],
                &included,
                &all_events,
            );
        }
        if text(event, "signature_status") != Some("valid") {
            return one_reason(
                "TRUST_UNRESOLVED",
                "EVENT_SIGNATURE_STATUS_UNKNOWN",
                &[],
                &included,
                &all_events,
            );
        }
        match text(event, "issuer_authority_status") {
            Some("authorized") => {}
            Some("unauthorized") => {
                return one_reason(
                    "BLOCK",
                    "ISSUER_AUTHORITY_INVALID",
                    &[],
                    &included,
                    &all_events,
                );
            }
            _ => {
                return one_reason(
                    "TRUST_UNRESOLVED",
                    "ISSUER_AUTHORITY_UNKNOWN",
                    &[],
                    &included,
                    &all_events,
                );
            }
        }
        match text(event, "source_clock_status") {
            Some("rollback_detected") => {
                return one_reason(
                    "TRUST_UNRESOLVED",
                    "SOURCE_CLOCK_ROLLBACK",
                    &[],
                    &included,
                    &all_events,
                );
            }
            Some("consistent" | "not_applicable") => {}
            _ => {
                return one_reason(
                    "TRUST_UNRESOLVED",
                    "SOURCE_CLOCK_UNVERIFIED",
                    &[],
                    &included,
                    &all_events,
                );
            }
        }
        match text(event, "sequence_status") {
            Some("gap") => {
                return one_reason(
                    "ABSTAIN",
                    "OWNER_EVENT_SEQUENCE_GAP",
                    &[],
                    &included,
                    &all_events,
                );
            }
            Some("fork") => {
                return one_reason(
                    "ABSTAIN",
                    "OWNER_EVENT_SEQUENCE_FORK",
                    &[],
                    &included,
                    &all_events,
                );
            }
            Some("unknown") => {
                return one_reason(
                    "ABSTAIN",
                    "OWNER_EVENT_SEQUENCE_UNKNOWN",
                    &[],
                    &included,
                    &all_events,
                );
            }
            Some("complete" | "not_applicable") => {}
            _ => {
                return one_reason(
                    "ABSTAIN",
                    "OWNER_EVENT_SEQUENCE_UNKNOWN",
                    &[],
                    &included,
                    &all_events,
                );
            }
        }

        let key_status = if mode.uses_current_trust() {
            text(event, "issuer_key_status_current")
        } else {
            text(event, "issuer_key_status_at_snapshot")
        };
        match key_status {
            Some("good") => {}
            Some("compromised") => {
                let compromise_time = number(event, "compromise_effective_at_unix_ms");
                if compromise_time.is_none() {
                    return one_reason(
                        "TRUST_UNRESOLVED",
                        "SIGNER_KEY_COMPROMISE_EFFECTIVE_TIME_UNKNOWN",
                        &[],
                        &included,
                        &all_events,
                    );
                }
                if compromise_time.is_some_and(|time| time <= trust_cutoff) {
                    return one_reason(
                        "TRUST_UNRESOLVED",
                        "SIGNER_KEY_COMPROMISED_AT_TRUST_SNAPSHOT",
                        &[],
                        &included,
                        &all_events,
                    );
                }
            }
            _ => {
                return one_reason(
                    "TRUST_UNRESOLVED",
                    "SIGNER_KEY_STATUS_UNKNOWN",
                    &[],
                    &included,
                    &all_events,
                );
            }
        }

        if text(event, "issuer_identity_id").is_none_or(str::is_empty) {
            return one_reason(
                "TRUST_UNRESOLVED",
                "ISSUER_IDENTITY_MISSING",
                &[],
                &included,
                &all_events,
            );
        }
    }

    // An event ID is scoped by its source namespace. A duplicated identity
    // with different payloads is equivocation; identical repeated delivery
    // is idempotent and the semantic event is applied once.
    let mut groups = BTreeMap::<(String, String), Vec<&Value>>::new();
    for event in &included {
        groups.entry(event_key(event)).or_default().push(*event);
    }
    let mut deduplicated = Vec::<&Value>::new();
    let mut repeated_delivery = false;
    for group in groups.values() {
        let payloads = group
            .iter()
            .filter_map(|event| text(event, "payload_sha256"))
            .collect::<BTreeSet<_>>();
        let issuers = group
            .iter()
            .filter_map(|event| text(event, "issuer_identity_id"))
            .collect::<BTreeSet<_>>();
        if payloads.len() > 1 || issuers.len() > 1 {
            return one_reason(
                "ABSTAIN",
                "DUPLICATE_EVENT_ID_CONFLICT",
                &[],
                group,
                &all_events,
            );
        }
        if group.len() > 1 {
            repeated_delivery = true;
        }
        if let Some(first) = group.first() {
            deduplicated.push(*first);
        }
    }
    deduplicated.sort_by_key(|event| event_key(event));

    // Corrections resolve only within the source-scoped event identity and
    // exact certificate scope. Never guess a target by local ID alone.
    for event in &deduplicated {
        let Some(target_id) = text(event, "corrects_event_id") else {
            continue;
        };
        let namespace = text(event, "source_namespace").unwrap_or_default();
        let target = deduplicated.iter().copied().find(|candidate| {
            text(candidate, "source_namespace") == Some(namespace)
                && text(candidate, "event_id") == Some(target_id)
        });
        let Some(target) = target else {
            return one_reason(
                "ABSTAIN",
                "CORRECTION_TARGET_UNRESOLVED",
                &[],
                &[*event],
                &all_events,
            );
        };
        if text(target, "certificate_id") != text(event, "certificate_id") {
            return one_reason(
                "ABSTAIN",
                "CORRECTION_TARGET_SCOPE_MISMATCH",
                &[],
                &[*event, target],
                &all_events,
            );
        }
    }

    // Cycles are diagnosed before monotonic-order checks so that the causal
    // defect is not hidden by one of the cycle's impossible edge orderings.
    if has_correction_cycle(&deduplicated) {
        return one_reason(
            "ABSTAIN",
            "CORRECTION_CYCLE",
            &[],
            &deduplicated,
            &all_events,
        );
    }

    for event in &deduplicated {
        if text(event, "event_type") != Some("corrected")
            && text(event, "event_type") != Some("superseded")
            && text(event, "event_type") != Some("reinstated")
        {
            continue;
        }
        let target_id = text(event, "corrects_event_id").unwrap_or_default();
        let namespace = text(event, "source_namespace").unwrap_or_default();
        let target = deduplicated.iter().copied().find(|candidate| {
            text(candidate, "source_namespace") == Some(namespace)
                && text(candidate, "event_id") == Some(target_id)
        });
        let recorded_at = number(event, "issuer_recorded_at_unix_ms");
        let target_recorded_at =
            target.and_then(|value| number(value, "issuer_recorded_at_unix_ms"));
        if recorded_at
            .zip(target_recorded_at)
            .is_none_or(|(new_time, old_time)| new_time <= old_time)
        {
            return one_reason(
                "ABSTAIN",
                "CORRECTION_TARGET_NOT_STRICTLY_EARLIER",
                &[],
                &[*event],
                &all_events,
            );
        }
    }

    // A well-formed correction link is not enough to infer lifecycle
    // semantics. Until the owner defines correction/supersession state
    // transitions and their authority rules, preserve the event and abstain.
    let corrections = deduplicated
        .iter()
        .copied()
        .filter(|event| matches!(text(event, "event_type"), Some("corrected" | "superseded")))
        .collect::<Vec<_>>();
    if !corrections.is_empty() {
        return one_reason(
            "ABSTAIN",
            "CALIBRATION_CORRECTION_SEMANTICS_UNQUALIFIED",
            &[],
            &corrections,
            &all_events,
        );
    }

    // Reinstatement changes qualification state. Until the owner-native
    // lifecycle and authority semantics are qualified, never auto-restore.
    let reinstatements = deduplicated
        .iter()
        .copied()
        .filter(|event| text(event, "event_type") == Some("reinstated"))
        .collect::<Vec<_>>();
    if !reinstatements.is_empty() {
        return one_reason(
            "ABSTAIN",
            "CALIBRATION_REINSTATEMENT_SEMANTICS_UNQUALIFIED",
            &[],
            &deduplicated,
            &all_events,
        );
    }

    let effectivity_ambiguous = deduplicated
        .iter()
        .copied()
        .filter(|event| {
            number(event, "effective_at_unix_ms").is_some_and(|effective| {
                effective > acquisition_start && effective <= acquisition_end
            })
        })
        .collect::<Vec<_>>();
    if !effectivity_ambiguous.is_empty() {
        return one_reason(
            "TIME_BOUNDARY_AMBIGUOUS",
            "EVENT_EFFECTIVITY_CROSSES_ACQUISITION_INTERVAL",
            &[],
            &effectivity_ambiguous,
            &all_events,
        );
    }

    let future_events = deduplicated
        .iter()
        .copied()
        .filter(|event| {
            number(event, "effective_at_unix_ms")
                .is_some_and(|effective| effective > acquisition_end)
        })
        .collect::<Vec<_>>();
    if !future_events.is_empty() && future_events.len() == deduplicated.len() {
        return one_reason(
            "NOT_YET_EFFECTIVE",
            "EVENT_NOT_YET_EFFECTIVE",
            &[],
            &future_events,
            &all_events,
        );
    }

    let active_revocations = deduplicated
        .iter()
        .copied()
        .filter(|event| is_effective_revocation(event, acquisition_start))
        .collect::<Vec<_>>();
    if !active_revocations.is_empty() {
        let mut reasons = BTreeSet::new();
        let reason = if active_revocations.len() > 1 {
            "CALIBRATION_EVENTS_KNOWN_AND_EFFECTIVE"
        } else {
            "REVOCATION_EFFECTIVE_AND_KNOWN"
        };
        reasons.insert(reason.to_owned());
        if repeated_delivery {
            reasons.insert("REPEATED_DELIVERY_DEDUPLICATED_ONCE".to_owned());
        }
        return disposition(
            "BLOCK",
            reasons,
            &active_revocations,
            &[],
            &all_events,
        );
    }

    if !not_yet_received.is_empty() && deduplicated.is_empty() {
        return one_reason(
            "NOT_KNOWN_AT_SNAPSHOT",
            "EVENT_RECEIVED_AFTER_SNAPSHOT",
            &[],
            &not_yet_received,
            &all_events,
        );
    }

    let mut reasons = BTreeSet::new();
    if repeated_delivery {
        reasons.insert("REPEATED_DELIVERY_DEDUPLICATED_ONCE".to_owned());
    }
    disposition(
        "ACCEPTED_FOR_BINDING_ONLY",
        reasons,
        &deduplicated,
        &[],
        &all_events,
    )
}

#[cfg(test)]
mod tests {
    use super::{AcquisitionTimeInterval, CORPUS, ReplayMode, TemporalDisposition, evaluate_case};
    use serde_json::Value;

    fn expected(case: &Value, mode: ReplayMode) -> Value {
        case.get("expected")
            .and_then(|value| value.get(mode.expected_key()))
            .expect("frozen expected mode")
            .clone()
    }

    fn report_value(report: TemporalDisposition) -> Value {
        serde_json::to_value(report).expect("temporal report serializes")
    }

    #[test]
    fn frozen_temporal_corpus_matches_reference_model_in_all_three_replay_modes() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        assert_eq!(
            corpus.get("oracle_review_status").and_then(Value::as_str),
            Some("NOT_INDEPENDENTLY_REVIEWED")
        );
        let cases = corpus
            .get("cases")
            .and_then(Value::as_array)
            .expect("temporal replay cases");
        assert_eq!(cases.len(), 29);

        let modes = [
            ReplayMode::AsKnownAt,
            ReplayMode::RecomputedWithCurrentTrust,
            ReplayMode::RecomputedWithCurrentEvidenceAndTrust,
        ];
        for case in cases {
            let case_id = case.get("case_id").and_then(Value::as_str).expect("case ID");
            for mode in modes {
                assert_eq!(
                    report_value(evaluate_case(case, mode)),
                    expected(case, mode),
                    "{case_id}: {:?}",
                    mode
                );
            }
        }
    }

    #[test]
    fn exhaustive_event_order_permutations_preserve_all_three_replay_modes() {
        fn collect_permutations<T: Clone>(
            items: &mut [T],
            start: usize,
            output: &mut Vec<Vec<T>>,
        ) {
            if start == items.len() {
                output.push(items.to_vec());
                return;
            }
            for index in start..items.len() {
                items.swap(start, index);
                collect_permutations(items, start + 1, output);
                items.swap(start, index);
            }
        }

        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let cases = corpus["cases"].as_array().expect("cases");
        let modes = [
            ReplayMode::AsKnownAt,
            ReplayMode::RecomputedWithCurrentTrust,
            ReplayMode::RecomputedWithCurrentEvidenceAndTrust,
        ];
        for case in cases {
            let events = case["events"].as_array().expect("events").clone();
            let mut permutations = Vec::new();
            collect_permutations(&mut events.clone(), 0, &mut permutations);
            for mode in modes {
                let baseline = evaluate_case(case, mode);
                for permutation in &permutations {
                    let mut permuted = case.clone();
                    permuted["events"] = Value::Array(permutation.clone());
                    assert_eq!(
                        baseline,
                        evaluate_case(&permuted, mode),
                        "{}: {:?} ({} event permutations)",
                        case["case_id"].as_str().unwrap_or("unknown"),
                        mode,
                        permutations.len()
                    );
                }
            }
        }
    }

    #[test]
    fn replay_report_pins_subject_evidence_trust_policy_and_verifier_coordinates() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| case["case_id"] == "backdated-event-received-after-snapshot")
            .expect("late event control");
        let historical = super::evaluate_report(case, ReplayMode::AsKnownAt)
            .expect("historical coordinate");
        let trust_only = super::evaluate_report(case, ReplayMode::RecomputedWithCurrentTrust)
            .expect("trust-only coordinate");
        let full = super::evaluate_report(case, ReplayMode::RecomputedWithCurrentEvidenceAndTrust)
            .expect("full reassessment coordinate");

        assert_eq!(
            historical.coordinate.subject_time_interval,
            AcquisitionTimeInterval {
                earliest_unix_ms: 140,
                latest_unix_ms: 140
            }
        );
        assert_eq!(
            historical.coordinate.evidence_checkpoint_reference,
            trust_only.coordinate.evidence_checkpoint_reference
        );
        assert_eq!(
            historical.coordinate.observer_coverage_receipt_reference,
            trust_only.coordinate.observer_coverage_receipt_reference
        );
        assert_eq!(
            historical.coordinate.receiver_receipt_watermark_unix_ms,
            trust_only.coordinate.receiver_receipt_watermark_unix_ms
        );
        assert_ne!(
            historical.coordinate.trust_policy_digest_sha256,
            trust_only.coordinate.trust_policy_digest_sha256
        );
        assert!(full.coordinate.receiver_receipt_watermark_unix_ms
            > historical.coordinate.receiver_receipt_watermark_unix_ms);
        assert!(full.coordinate.trust_status_as_of_unix_ms
            > historical.coordinate.trust_status_as_of_unix_ms);
        assert_eq!(
            full.disposition.disposition,
            "BLOCK"
        );
        assert_eq!(
            historical.disposition.disposition,
            "NOT_KNOWN_AT_SNAPSHOT"
        );
    }

    #[test]
    fn late_retroactive_revocation_changes_only_a_new_full_reassessment() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"]
                    == "backdated-event-received-after-snapshot"
            })
            .expect("late event control");
        assert_eq!(
            evaluate_case(case, ReplayMode::AsKnownAt).disposition,
            "NOT_KNOWN_AT_SNAPSHOT"
        );
        assert_eq!(
            evaluate_case(case, ReplayMode::RecomputedWithCurrentTrust).disposition,
            "NOT_KNOWN_AT_SNAPSHOT"
        );
        assert_eq!(
            evaluate_case(case, ReplayMode::RecomputedWithCurrentEvidenceAndTrust).disposition,
            "BLOCK"
        );
    }

    #[test]
    fn revocation_after_acquisition_does_not_retroactively_invalidate_the_measurement() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"] == "revocation-effective-after-acquisition-before-evaluation"
            })
            .expect("post-acquisition revocation control");
        for mode in [
            ReplayMode::AsKnownAt,
            ReplayMode::RecomputedWithCurrentTrust,
            ReplayMode::RecomputedWithCurrentEvidenceAndTrust,
        ] {
            let report = evaluate_case(case, mode);
            assert_eq!(report.disposition, "NOT_YET_EFFECTIVE");
        }
    }

    #[test]
    fn revocation_effectivity_inside_acquisition_uncertainty_requires_abstention() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"] == "event-effectivity-crosses-acquisition-uncertainty-window"
            })
            .expect("acquisition/effectivity overlap control");
        for mode in [
            ReplayMode::AsKnownAt,
            ReplayMode::RecomputedWithCurrentTrust,
            ReplayMode::RecomputedWithCurrentEvidenceAndTrust,
        ] {
            let report = evaluate_case(case, mode);
            assert_eq!(report.disposition, "TIME_BOUNDARY_AMBIGUOUS");
            assert_eq!(
                report.reason_codes,
                vec!["EVENT_EFFECTIVITY_CROSSES_ACQUISITION_INTERVAL".to_owned()]
            );
        }
    }

    #[test]
    fn valid_timestamp_claim_requires_exact_payload_imprint_and_token_time() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let cases = corpus["cases"].as_array().expect("cases");
        let valid = cases
            .iter()
            .find(|case| case["case_id"] == "valid-timestamp-imprint-binds-event-payload")
            .expect("valid timestamp positive control");
        let valid_report = evaluate_case(valid, ReplayMode::AsKnownAt);
        assert_eq!(valid_report.disposition, "ACCEPTED_FOR_BINDING_ONLY");
        assert_eq!(valid_report.applied_event_ids, vec!["EV-TSA-VALID"]);

        let mismatch = cases
            .iter()
            .find(|case| {
                case["case_id"] == "valid-timestamp-label-with-mismatching-imprint-rejected"
            })
            .expect("valid label/wrong digest negative control");
        let mismatch_report = evaluate_case(mismatch, ReplayMode::AsKnownAt);
        assert_eq!(mismatch_report.disposition, "REJECT");
        assert_eq!(
            mismatch_report.reason_codes,
            vec!["TIMESTAMP_IMPRINT_MISMATCH".to_owned()]
        );
    }

    #[test]
    fn correction_and_reinstatement_transitions_fail_closed_without_owner_semantics() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let cases = corpus["cases"].as_array().expect("cases");

        let missing = cases
            .iter()
            .find(|case| case["case_id"] == "correction-target-missing-or-not-received")
            .expect("missing correction target control");
        let missing_report = evaluate_case(missing, ReplayMode::AsKnownAt);
        assert_eq!(missing_report.disposition, "ABSTAIN");
        assert_eq!(
            missing_report.reason_codes,
            vec!["CORRECTION_TARGET_UNRESOLVED".to_owned()]
        );

        let cross_scope = cases
            .iter()
            .find(|case| case["case_id"] == "correction-target-cross-certificate-scope")
            .expect("cross-certificate correction control");
        let cross_report = evaluate_case(cross_scope, ReplayMode::AsKnownAt);
        assert_eq!(cross_report.disposition, "ABSTAIN");
        assert_eq!(
            cross_report.reason_codes,
            vec!["CORRECTION_TARGET_SCOPE_MISMATCH".to_owned()]
        );

        let reinstatement = cases
            .iter()
            .find(|case| {
                case["case_id"] == "reinstatement-event-does-not-silently-restore-calibration"
            })
            .expect("reinstatement control");
        let reinstatement_report = evaluate_case(reinstatement, ReplayMode::AsKnownAt);
        assert_eq!(reinstatement_report.disposition, "ABSTAIN");
        assert_eq!(
            reinstatement_report.reason_codes,
            vec!["CALIBRATION_REINSTATEMENT_SEMANTICS_UNQUALIFIED".to_owned()]
        );
    }

    #[test]
    fn well_formed_correction_links_do_not_infer_unqualified_lifecycle_state() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let cases = corpus["cases"].as_array().expect("cases");
        let valid = cases
            .iter()
            .find(|case| {
                case["case_id"]
                    == "well-formed-correction-target-still-abstains-without-qualified-semantics"
            })
            .expect("well-formed correction control");
        let report = evaluate_case(valid, ReplayMode::AsKnownAt);
        assert_eq!(report.disposition, "ABSTAIN");
        assert_eq!(
            report.reason_codes,
            vec!["CALIBRATION_CORRECTION_SEMANTICS_UNQUALIFIED".to_owned()]
        );

        let nested = cases
            .iter()
            .find(|case| case["case_id"] == "correction-of-correction-remains-unqualified")
            .expect("nested correction control");
        let nested_report = evaluate_case(nested, ReplayMode::AsKnownAt);
        assert_eq!(nested_report.disposition, "ABSTAIN");
        assert_eq!(
            nested_report.reason_codes,
            vec!["CALIBRATION_CORRECTION_SEMANTICS_UNQUALIFIED".to_owned()]
        );
    }

    #[test]
    fn equal_record_time_correction_is_not_strictly_ordered() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"] == "correction-record-time-not-strictly-after-target-abstains"
            })
            .expect("equal-time correction control");
        let report = evaluate_case(case, ReplayMode::AsKnownAt);
        assert_eq!(report.disposition, "ABSTAIN");
        assert_eq!(
            report.reason_codes,
            vec!["CORRECTION_TARGET_NOT_STRICTLY_EARLIER".to_owned()]
        );
    }

    #[test]
    fn unverified_observer_coverage_cannot_support_a_negative_evidence_claim() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"]
                    == "observer-coverage-unqualified-cannot-support-negative-claim"
            })
            .expect("unverified observer coverage control");
        assert_eq!(
            evaluate_case(case, ReplayMode::AsKnownAt).disposition,
            "TRUST_UNRESOLVED"
        );
        assert_eq!(
            evaluate_case(case, ReplayMode::RecomputedWithCurrentTrust).disposition,
            "TRUST_UNRESOLVED"
        );
        assert_eq!(
            evaluate_case(case, ReplayMode::RecomputedWithCurrentEvidenceAndTrust).disposition,
            "ACCEPTED_FOR_BINDING_ONLY"
        );
    }

    #[test]
    fn report_coordinate_pins_checkpoint_and_coverage_evidence() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| case["case_id"] == "positive-no-event-baseline")
            .expect("positive baseline");
        let report = super::evaluate_report(case, ReplayMode::AsKnownAt)
            .expect("complete evidence coordinate");
        assert_eq!(
            report.coordinate.evidence_checkpoint_digest_sha256,
            report.coordinate.evidence_view_digest_sha256
        );
        assert_eq!(
            report.coordinate.evidence_checkpoint_watermark_unix_ms,
            report.coordinate.receiver_receipt_watermark_unix_ms
        );
        assert_eq!(
            report.coordinate.observer_coverage_watermark_unix_ms,
            report.coordinate.receiver_receipt_watermark_unix_ms
        );
        assert_eq!(
            report.coordinate.observer_coverage_status,
            "SYNTHETIC_QUALIFIED_COMPLETE"
        );
        assert!(!report.coordinate.observer_coverage_receipt_reference.is_empty());
        assert!(valid_sha256(
            &report.coordinate.observer_coverage_receipt_digest_sha256
        ));
    }

    #[test]
    fn incomplete_observer_coverage_abstains_even_when_checkpoint_is_intact() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"]
                    == "incomplete-observer-coverage-abstains-even-with-valid-checkpoint"
            })
            .expect("incomplete observer coverage control");
        for mode in [
            ReplayMode::AsKnownAt,
            ReplayMode::RecomputedWithCurrentTrust,
            ReplayMode::RecomputedWithCurrentEvidenceAndTrust,
        ] {
            assert_eq!(evaluate_case(case, mode).disposition, "ABSTAIN");
        }
    }

    #[test]
    fn evidence_checkpoint_digest_must_bind_the_selected_view() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"]
                    == "evidence-checkpoint-digest-mismatch-rejects-historical-view"
            })
            .expect("checkpoint digest mismatch control");
        assert_eq!(
            evaluate_case(case, ReplayMode::AsKnownAt).disposition,
            "REJECT"
        );
        assert_eq!(
            evaluate_case(case, ReplayMode::RecomputedWithCurrentTrust).disposition,
            "REJECT"
        );
        assert_eq!(
            evaluate_case(case, ReplayMode::RecomputedWithCurrentEvidenceAndTrust).disposition,
            "ACCEPTED_FOR_BINDING_ONLY"
        );
    }

    #[test]
    fn idempotent_repeated_delivery_never_hides_the_underlying_revocation() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"]
                    == "same-source-id-same-payload-repeated-delivery-deduplicates-once"
            })
            .expect("idempotent delivery control");
        let report = evaluate_case(case, ReplayMode::AsKnownAt);
        assert_eq!(report.disposition, "BLOCK");
        assert_eq!(report.applied_event_ids, vec!["lab-a:EV-REPEAT-1"]);
        assert!(
            report
                .reason_codes
                .contains(&"REVOCATION_EFFECTIVE_AND_KNOWN".to_owned())
        );
        assert!(
            report
                .reason_codes
                .contains(&"REPEATED_DELIVERY_DEDUPLICATED_ONCE".to_owned())
        );
    }

    #[test]
    fn an_unverified_namespace_alias_cannot_evade_source_scoped_identity() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let case = corpus["cases"]
            .as_array()
            .expect("cases")
            .iter()
            .find(|case| {
                case["case_id"]
                    == "same-issuer-cannot-bypass-identity-with-unverified-namespace-alias"
            })
            .expect("namespace alias control");
        let report = evaluate_case(case, ReplayMode::AsKnownAt);
        assert_eq!(report.disposition, "TRUST_UNRESOLVED");
        assert_eq!(
            report.reason_codes,
            vec!["SOURCE_NAMESPACE_BINDING_UNVERIFIED".to_owned()]
        );
    }
}
