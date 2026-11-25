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
    fn receipt_watermark_field(self) -> &'static str {
        match self {
            Self::AsKnownAt | Self::RecomputedWithCurrentTrust => {
                "snapshot/receiver_receipt_watermark_unix_ms"
            }
            Self::RecomputedWithCurrentEvidenceAndTrust => {
                "current_reassessment/receiver_receipt_watermark_unix_ms"
            }
        }
    }

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

fn sorted_ids(events: &[&Value]) -> Vec<String> {
    events
        .iter()
        .map(|event| display_id(events, event))
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
        applied_event_ids: sorted_ids(applied),
        excluded_event_ids: sorted_ids(excluded),
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
                    (source.0, target.to_owned()),
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
        let found = edges.keys().any(|candidate| {
            candidate.0 == target.0 && candidate.1 == target.1
        }) && visit(target, edges, visiting, visited);
        visiting.remove(node);
        visited.insert(node.clone());
        found
    }

    let mut visiting = BTreeSet::new();
    let mut visited = BTreeSet::new();
    edges.keys().any(|node| visit(node, &edges, &mut visiting, &mut visited))
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

    let Some(subject_time) = nested_number(case, "/snapshot/evaluated_at_unix_ms") else {
        return one_reason(
            "REJECT",
            "SUBJECT_TIME_MISSING",
            &[],
            &all_events,
            &all_events,
        );
    };
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
        let id = display_id(&all_events, event);
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
            let status = if receipt_status == Some("INVALID") { "REJECT" } else { "TRUST_UNRESOLVED" };
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
        let _ = id;
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

    // The same local event ID under distinct, trusted issuer namespaces is
    // not a collision. An unverified alias was rejected above.
    if has_correction_cycle(&deduplicated) {
        return one_reason(
            "ABSTAIN",
            "CORRECTION_CYCLE",
            &[],
            &deduplicated,
            &all_events,
        );
    }

    let effective_revocations = deduplicated
        .iter()
        .copied()
        .filter(|event| {
            number(event, "effective_at_unix_ms")
                .is_some_and(|effective| effective > subject_time)
        })
        .collect::<Vec<_>>();
    if !effective_revocations.is_empty()
        && effective_revocations.len() == deduplicated.len()
    {
        return one_reason(
            "NOT_YET_EFFECTIVE",
            "EVENT_NOT_YET_EFFECTIVE",
            &[],
            &effective_revocations,
            &all_events,
        );
    }

    let active_revocations = deduplicated
        .iter()
        .copied()
        .filter(|event| is_effective_revocation(event, subject_time))
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

    if !effective_revocations.is_empty() {
        return one_reason(
            "NOT_YET_EFFECTIVE",
            "EVENT_NOT_YET_EFFECTIVE",
            &[],
            &effective_revocations,
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
    use super::{CORPUS, ReplayMode, TemporalDisposition, evaluate_case};
    use serde_json::{Value, json};

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
        assert_eq!(cases.len(), 16);

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
    fn event_order_permutations_do_not_change_any_temporal_replay_mode() {
        let corpus: Value = serde_json::from_str(CORPUS).expect("temporal corpus parses");
        let cases = corpus["cases"].as_array().expect("cases");
        let modes = [
            ReplayMode::AsKnownAt,
            ReplayMode::RecomputedWithCurrentTrust,
            ReplayMode::RecomputedWithCurrentEvidenceAndTrust,
        ];
        for case in cases {
            for mode in modes {
                let baseline = evaluate_case(case, mode);
                let mut reversed = case.clone();
                reversed["events"]
                    .as_array_mut()
                    .expect("events array")
                    .reverse();
                assert_eq!(
                    baseline,
                    evaluate_case(&reversed, mode),
                    "{}: {:?}",
                    case["case_id"].as_str().unwrap_or("unknown"),
                    mode
                );
            }
        }
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
        assert!(report.reason_codes.contains(&"REVOCATION_EFFECTIVE_AND_KNOWN".to_owned()));
        assert!(report.reason_codes.contains(&"REPEATED_DELIVERY_DEDUPLICATED_ONCE".to_owned()));
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
