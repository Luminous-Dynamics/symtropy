// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic three-axis temporal evidence replay reference kernel.
//!
//! This module executes the frozen v0.1 replay corpus under three distinct
//! evidence/trust coordinates. It consumes owner-supplied receipt, checkpoint,
//! observer and trust dispositions. It does not verify cryptographic tokens,
//! qualify clocks, create a ledger, or establish measurement/physical truth.
//! Its strongest positive output is ACCEPTED_FOR_BINDING_ONLY.

use crate::effectivity_interval_v0_1::{
    EffectivityRelation, TimeInterval, compare_effectivity_intervals,
};
use serde::Serialize;
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};

#[cfg(test)]
const CORPUS: &str = include_str!(
    "../../../../docs/research/aviation/examples/aero_temporal_replay_corpus_v0_1.json"
);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum TemporalQueryMode {
    AsKnownAt,
    RecomputedWithCurrentTrust,
    RecomputedWithCurrentEvidenceAndTrust,
}

impl TemporalQueryMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::AsKnownAt => "as_known_at",
            Self::RecomputedWithCurrentTrust => "recomputed_with_current_trust",
            Self::RecomputedWithCurrentEvidenceAndTrust => {
                "recomputed_with_current_evidence_and_trust"
            }
        }
    }

    fn current_evidence(self) -> bool {
        matches!(self, Self::RecomputedWithCurrentEvidenceAndTrust)
    }

    fn current_trust(self) -> bool {
        !matches!(self, Self::AsKnownAt)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct ReplayDisposition {
    pub mode: String,
    pub disposition: String,
    pub reason_codes: Vec<String>,
    pub applied_event_ids: Vec<String>,
    pub excluded_event_ids: Vec<String>,
}

#[derive(Default)]
struct Findings {
    reject: bool,
    trust_unresolved: bool,
    abstain: bool,
    time_ambiguous: bool,
    not_known: bool,
    not_yet_effective: bool,
    block: bool,
    reasons: BTreeSet<String>,
    applied: BTreeSet<String>,
    excluded: BTreeSet<String>,
}

impl Findings {
    fn reason(&mut self, reason: &str) {
        self.reasons.insert(reason.to_owned());
    }

    fn exclude(&mut self, event_id: &str) {
        self.excluded.insert(event_id.to_owned());
    }

    fn finish(self, mode: TemporalQueryMode) -> ReplayDisposition {
        let disposition = if self.reject {
            "REJECT"
        } else if self.trust_unresolved {
            "TRUST_UNRESOLVED"
        } else if self.abstain {
            "ABSTAIN"
        } else if self.time_ambiguous {
            "TIME_BOUNDARY_AMBIGUOUS"
        } else if self.not_known {
            "NOT_KNOWN_AT_SNAPSHOT"
        } else if self.not_yet_effective {
            "NOT_YET_EFFECTIVE"
        } else if self.block {
            "BLOCK"
        } else {
            "ACCEPTED_FOR_BINDING_ONLY"
        };
        ReplayDisposition {
            mode: mode.as_str().to_owned(),
            disposition: disposition.to_owned(),
            reason_codes: self.reasons.into_iter().collect(),
            applied_event_ids: self.applied.into_iter().collect(),
            excluded_event_ids: self.excluded.into_iter().collect(),
        }
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn unsigned(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn nonempty(value: Option<&str>) -> bool {
    value.is_some_and(|text| !text.trim().is_empty())
}

fn valid_sha256(value: Option<&str>) -> bool {
    value.is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
    })
}

fn source_key(event: &Value) -> String {
    format!(
        "{}:{}",
        string(event, "source_namespace").unwrap_or("source"),
        string(event, "event_id").unwrap_or("event")
    )
}

fn identity_label(event: &Value, colliding_local_ids: &BTreeSet<String>) -> String {
    let id = string(event, "event_id").unwrap_or("event");
    if colliding_local_ids.contains(id) {
        source_key(event)
    } else {
        id.to_owned()
    }
}

fn selected_snapshots<'a>(
    case: &'a Value,
    mode: TemporalQueryMode,
) -> Option<(&'a Value, &'a Value)> {
    let original = case.get("snapshot")?;
    let current = case.get("current_reassessment")?;
    if mode.current_evidence() {
        Some((current, current))
    } else if mode.current_trust() {
        Some((original, current))
    } else {
        Some((original, original))
    }
}

fn check_checkpoint(snapshot: &Value, findings: &mut Findings) -> Option<u64> {
    match string(snapshot, "evidence_checkpoint_status") {
        Some("SYNTHETIC_VALID") => {}
        Some("INVALID") => {
            findings.reject = true;
            findings.reason("EVIDENCE_CHECKPOINT_INVALID");
            return None;
        }
        Some("UNVERIFIED") => {
            findings.trust_unresolved = true;
            findings.reason("EVIDENCE_CHECKPOINT_UNVERIFIED");
            return None;
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("EVIDENCE_CHECKPOINT_STATUS_UNKNOWN");
            return None;
        }
    }

    let watermark = unsigned(snapshot, "receiver_receipt_watermark_unix_ms");
    let checkpoint_watermark = unsigned(snapshot, "evidence_checkpoint_watermark_unix_ms");
    let checkpoint_digest = string(snapshot, "evidence_checkpoint_digest_sha256");
    let view_digest = string(snapshot, "evidence_view_digest_sha256");
    if !nonempty(string(snapshot, "evidence_checkpoint_reference"))
        || !valid_sha256(checkpoint_digest)
        || !valid_sha256(view_digest)
        || checkpoint_digest != view_digest
        || watermark.is_none()
        || checkpoint_watermark != watermark
    {
        findings.reject = true;
        findings.reason("EVIDENCE_CHECKPOINT_DIGEST_MISMATCH");
        return None;
    }
    watermark
}

fn check_observer_coverage(snapshot: &Value, findings: &mut Findings) {
    match string(snapshot, "observer_coverage_status") {
        Some("SYNTHETIC_QUALIFIED_COMPLETE") => {
            if !nonempty(string(snapshot, "observer_coverage_profile_id"))
                || !nonempty(string(snapshot, "observer_coverage_receipt_reference"))
                || !valid_sha256(string(snapshot, "observer_coverage_receipt_digest_sha256"))
                || unsigned(snapshot, "observer_coverage_watermark_unix_ms").is_none()
            {
                findings.trust_unresolved = true;
                findings.reason("OBSERVER_COVERAGE_RECEIPT_UNVERIFIED");
            }
        }
        Some("INCOMPLETE") => {
            findings.abstain = true;
            findings.reason("OBSERVER_COVERAGE_INCOMPLETE");
        }
        Some("UNVERIFIED") => {
            findings.trust_unresolved = true;
            findings.reason("OBSERVER_COVERAGE_UNQUALIFIED");
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("OBSERVER_COVERAGE_STATUS_UNKNOWN");
        }
    }
}

fn check_acquisition_interval(case: &Value, findings: &mut Findings) -> Option<TimeInterval> {
    let acquisition = case.get("acquisition_time_interval");
    let earliest = acquisition.and_then(|v| unsigned(v, "earliest_unix_ms"));
    let latest = acquisition.and_then(|v| unsigned(v, "latest_unix_ms"));
    let valid_from = case
        .get("calibration_validity")
        .and_then(|v| unsigned(v, "valid_from_unix_ms"));
    let valid_until = case
        .get("calibration_validity")
        .and_then(|v| unsigned(v, "valid_until_unix_ms"));
    let (Some(earliest), Some(latest), Some(valid_from), Some(valid_until)) =
        (earliest, latest, valid_from, valid_until)
    else {
        findings.reject = true;
        findings.reason("TEMPORAL_QUERY_ENVELOPE_INVALID");
        return None;
    };
    if earliest > latest || valid_from >= valid_until {
        findings.reject = true;
        findings.reason("TEMPORAL_QUERY_ENVELOPE_INVALID");
        return None;
    }
    if earliest < valid_from || latest >= valid_until {
        if latest >= valid_from && earliest < valid_until {
            findings.time_ambiguous = true;
            findings.reason("ACQUISITION_INTERVAL_CROSSES_CALIBRATION_BOUNDARY");
        } else {
            findings.abstain = true;
            findings.reason("ACQUISITION_INTERVAL_OUTSIDE_CALIBRATION_VALIDITY");
        }
    }
    Some(TimeInterval {
        earliest_unix_ms: earliest,
        latest_unix_ms: latest,
    })
}

fn validate_event_evidence(event: &Value, findings: &mut Findings, identity: &str) -> bool {
    let payload = string(event, "payload_sha256");
    if !valid_sha256(payload) {
        findings.reject = true;
        findings.reason("EVENT_PAYLOAD_DIGEST_INVALID");
        findings.exclude(identity);
        return false;
    }

    match string(event, "signature_status") {
        Some("valid") => {}
        Some("invalid") => {
            findings.reject = true;
            findings.reason("EVENT_SIGNATURE_INVALID");
            findings.exclude(identity);
            return false;
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("EVENT_SIGNATURE_UNVERIFIED");
            findings.exclude(identity);
            return false;
        }
    }

    let imprint = string(event, "trusted_timestamp_imprint_sha256");
    match string(event, "trusted_timestamp_status") {
        Some("absent") => {
            if unsigned(event, "trusted_timestamp_at_unix_ms").is_some() || imprint.is_some() {
                findings.reject = true;
                findings.reason("TIMESTAMP_IMPRINT_MISMATCH");
                findings.exclude(identity);
                return false;
            }
        }
        Some("valid") => {
            if unsigned(event, "trusted_timestamp_at_unix_ms").is_none()
                || !valid_sha256(imprint)
                || imprint != payload
            {
                findings.reject = true;
                findings.reason("TIMESTAMP_IMPRINT_MISMATCH");
                findings.exclude(identity);
                return false;
            }
        }
        Some("invalid_digest") => {
            findings.reject = true;
            findings.reason("TIMESTAMP_IMPRINT_MISMATCH");
            findings.exclude(identity);
            return false;
        }
        Some("invalid_token") => {
            findings.reject = true;
            findings.reason("TIMESTAMP_TOKEN_INVALID");
            findings.exclude(identity);
            return false;
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("TIMESTAMP_EVIDENCE_UNVERIFIED");
            findings.exclude(identity);
            return false;
        }
    }

    match string(event, "receiver_receipt_status") {
        Some("SYNTHETIC_VALID") => {
            if !nonempty(string(event, "receiver_receipt_reference"))
                || string(event, "receiver_receipt_digest_sha256") != payload
            {
                findings.reject = true;
                findings.reason("RECEIVER_RECEIPT_DIGEST_MISMATCH");
                findings.exclude(identity);
                return false;
            }
        }
        Some("INVALID") => {
            findings.reject = true;
            findings.reason("RECEIVER_RECEIPT_INVALID");
            findings.exclude(identity);
            return false;
        }
        Some("UNVERIFIED" | "ABSENT") => {
            findings.trust_unresolved = true;
            findings.reason("RECEIVER_RECEIPT_NOT_VERIFIABLE");
            findings.exclude(identity);
            return false;
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("RECEIVER_RECEIPT_STATUS_UNKNOWN");
            findings.exclude(identity);
            return false;
        }
    }

    match string(event, "issuer_authority_status") {
        Some("authorized") => {}
        Some("unauthorized") => {
            findings.reject = true;
            findings.reason("ISSUER_AUTHORITY_INVALID");
            findings.exclude(identity);
            return false;
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("ISSUER_AUTHORITY_UNKNOWN");
            findings.exclude(identity);
            return false;
        }
    }

    match string(event, "sequence_status") {
        Some("complete" | "not_applicable") => {}
        Some("gap") => {
            findings.abstain = true;
            findings.reason("OWNER_EVENT_SEQUENCE_GAP");
            findings.exclude(identity);
            return false;
        }
        Some("fork") => {
            findings.abstain = true;
            findings.reason("OWNER_EVENT_SEQUENCE_FORK");
            findings.exclude(identity);
            return false;
        }
        _ => {
            findings.abstain = true;
            findings.reason("OWNER_EVENT_SEQUENCE_UNRESOLVED");
            findings.exclude(identity);
            return false;
        }
    }

    match string(event, "source_clock_status") {
        Some("consistent" | "not_applicable") => {}
        Some("rollback_detected") => {
            findings.trust_unresolved = true;
            findings.reason("SOURCE_CLOCK_ROLLBACK");
            findings.exclude(identity);
            return false;
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("SOURCE_CLOCK_UNVERIFIED");
            findings.exclude(identity);
            return false;
        }
    }
    true
}

fn check_signer_key(
    event: &Value,
    mode: TemporalQueryMode,
    trust_as_of: u64,
    findings: &mut Findings,
    identity: &str,
) -> bool {
    let field = if mode.current_trust() {
        "issuer_key_status_current"
    } else {
        "issuer_key_status_at_snapshot"
    };
    match string(event, field) {
        Some("good") => true,
        Some("compromised") => {
            let compromise_time = unsigned(event, "compromise_effective_at_unix_ms");
            if compromise_time.is_none_or(|time| time > trust_as_of) {
                findings.reason("SIGNER_KEY_COMPROMISE_EFFECTIVE_TIME_UNKNOWN");
            } else {
                findings.reason("SIGNER_KEY_COMPROMISED");
            }
            findings.trust_unresolved = true;
            findings.exclude(identity);
            false
        }
        _ => {
            findings.trust_unresolved = true;
            findings.reason("SIGNER_KEY_STATUS_UNKNOWN");
            findings.exclude(identity);
            false
        }
    }
}

fn correction_cycles(events: &[Value]) -> BTreeSet<String> {
    let by_key = events
        .iter()
        .filter_map(|event| Some((source_key(event), event)))
        .collect::<BTreeMap<_, _>>();
    let mut cycles = BTreeSet::new();
    for start in by_key.keys() {
        let mut path = Vec::<String>::new();
        let mut seen = BTreeMap::<String, usize>::new();
        let mut current = start.clone();
        loop {
            if let Some(position) = seen.get(&current).copied() {
                for key in path.iter().skip(position) {
                    cycles.insert(key.clone());
                }
                break;
            }
            let Some(event) = by_key.get(&current) else {
                break;
            };
            seen.insert(current.clone(), path.len());
            path.push(current.clone());
            let Some(target) = string(event, "corrects_event_id") else {
                break;
            };
            current = composite_key(
                string(event, "source_namespace").unwrap_or("source"),
                target,
            );
        }
    }
    cycles
}

pub fn evaluate_temporal_replay_case(case: &Value, mode: TemporalQueryMode) -> ReplayDisposition {
    let mut findings = Findings::default();
    let Some((evidence_snapshot, trust_snapshot)) = selected_snapshots(case, mode) else {
        findings.reject = true;
        findings.reason("TEMPORAL_REPLAY_SNAPSHOT_MISSING");
        return findings.finish(mode);
    };
    let Some(watermark) = check_checkpoint(evidence_snapshot, &mut findings) else {
        return findings.finish(mode);
    };
    check_observer_coverage(evidence_snapshot, &mut findings);
    let Some(acquisition) = check_acquisition_interval(case, &mut findings) else {
        return findings.finish(mode);
    };

    let trust_as_of = unsigned(trust_snapshot, "trust_status_as_of_unix_ms").unwrap_or(0);
    if !valid_sha256(string(trust_snapshot, "trust_policy_digest_sha256"))
        || !nonempty(string(trust_snapshot, "trust_policy_id"))
    {
        findings.trust_unresolved = true;
        findings.reason("TRUST_POLICY_SNAPSHOT_UNRESOLVED");
    }

    let Some(raw_events) = case.get("events").and_then(Value::as_array) else {
        findings.reject = true;
        findings.reason("TEMPORAL_REPLAY_EVENTS_INVALID");
        return findings.finish(mode);
    };

    let mut visible = Vec::<Value>::new();
    for event in raw_events {
        let identity = identity_label(event, &BTreeSet::new());
        let Some(first_seen) = unsigned(event, "receiver_first_seen_at_unix_ms") else {
            findings.trust_unresolved = true;
            findings.reason("RECEIVER_FIRST_SEEN_TIME_UNRESOLVED");
            findings.exclude(&identity);
            continue;
        };
        if first_seen > watermark {
            findings.not_known = true;
            findings.reason("EVENT_RECEIVED_AFTER_SNAPSHOT");
            findings.exclude(&identity);
            continue;
        }
        visible.push(event.clone());
    }

    let mut namespaces_by_id = BTreeMap::<String, BTreeSet<String>>::new();
    for event in &visible {
        if let (Some(id), Some(namespace)) =
            (string(event, "event_id"), string(event, "source_namespace"))
        {
            namespaces_by_id
                .entry(id.to_owned())
                .or_default()
                .insert(namespace.to_owned());
        }
    }
    let colliding_ids = namespaces_by_id
        .iter()
        .filter(|(_, namespaces)| namespaces.len() > 1)
        .map(|(id, _)| id.clone())
        .collect::<BTreeSet<_>>();

    // An unverified namespace alias invalidates every visible event from the
    // same issuer identity, not only the alias row.
    let mut bad_issuers = BTreeSet::new();
    for event in &visible {
        match string(event, "source_namespace_binding_status") {
            Some("SYNTHETIC_VALID") => {}
            Some("CONFLICT") => {
                findings.reject = true;
                findings.reason("SOURCE_NAMESPACE_BINDING_CONFLICT");
                if let Some(issuer) = string(event, "issuer_identity_id") {
                    bad_issuers.insert(issuer.to_owned());
                }
            }
            _ => {
                findings.trust_unresolved = true;
                findings.reason("SOURCE_NAMESPACE_BINDING_UNVERIFIED");
                if let Some(issuer) = string(event, "issuer_identity_id") {
                    bad_issuers.insert(issuer.to_owned());
                }
            }
        }
    }
    let mut issuer_excluded = BTreeSet::new();
    for event in &visible {
        if string(event, "issuer_identity_id").is_some_and(|issuer| bad_issuers.contains(issuer)) {
            issuer_excluded.insert(source_key(event));
            findings.exclude(&identity_label(event, &colliding_ids));
        }
    }

    let mut grouped = BTreeMap::<String, Vec<Value>>::new();
    for event in visible {
        grouped.entry(source_key(&event)).or_default().push(event);
    }
    let mut events = Vec::<Value>::new();
    let mut conflicted_keys = BTreeSet::new();
    for (key, deliveries) in grouped {
        let payloads = deliveries
            .iter()
            .filter_map(|event| string(event, "payload_sha256"))
            .collect::<BTreeSet<_>>();
        if payloads.len() > 1 {
            findings.abstain = true;
            findings.reason("DUPLICATE_EVENT_ID_CONFLICT");
            conflicted_keys.insert(key.clone());
            if let Some(event) = deliveries.first() {
                findings.exclude(&identity_label(event, &colliding_ids));
            }
            continue;
        }
        if deliveries.len() > 1 {
            findings.reason("REPEATED_DELIVERY_DEDUPLICATED_ONCE");
        }
        if let Some(first) = deliveries.into_iter().next() {
            events.push(first);
        }
    }

    let cycles = correction_cycles(&events);
    let mut suppressed_certificates = BTreeSet::new();
    for event in &events {
        let key = source_key(event);
        if cycles.contains(&key) {
            findings.abstain = true;
            findings.reason("CORRECTION_CYCLE");
            findings.exclude(&identity_label(event, &colliding_ids));
            suppressed_certificates
                .insert(string(event, "certificate_id").unwrap_or("").to_owned());
        }
    }

    let event_index = events
        .iter()
        .filter_map(|event| Some((source_key(event), event)))
        .collect::<BTreeMap<_, _>>();

    // Corrections are never interpreted as a generic latest-row replacement.
    for event in &events {
        let event_type = string(event, "event_type").unwrap_or("");
        if !matches!(event_type, "corrected" | "superseded") || cycles.contains(&source_key(event))
        {
            continue;
        }
        let identity = identity_label(event, &colliding_ids);
        let source = string(event, "source_namespace").unwrap_or("source");
        let certificate = string(event, "certificate_id").unwrap_or("");
        if let Some(target_id) = string(event, "corrects_event_id") {
            let target_key = composite_key(source, target_id);
            if let Some(target) = event_index.get(&target_key) {
                if string(target, "certificate_id") != Some(certificate) {
                    findings.abstain = true;
                    findings.reason("CORRECTION_TARGET_SCOPE_MISMATCH");
                    findings.exclude(&identity_label(target, &colliding_ids));
                    findings.exclude(&identity);
                    suppressed_certificates.insert(certificate.to_owned());
                    suppressed_certificates
                        .insert(string(target, "certificate_id").unwrap_or("").to_owned());
                    continue;
                }
                let correction_recorded = unsigned(event, "issuer_recorded_at_unix_ms");
                let target_recorded = unsigned(target, "issuer_recorded_at_unix_ms");
                if correction_recorded.is_none()
                    || target_recorded.is_none()
                    || correction_recorded <= target_recorded
                {
                    findings.abstain = true;
                    findings.reason("CORRECTION_TARGET_NOT_STRICTLY_EARLIER");
                    findings.exclude(&identity);
                    suppressed_certificates.insert(certificate.to_owned());
                    continue;
                }
                if matches!(
                    string(target, "event_type"),
                    Some("corrected" | "superseded")
                ) {
                    findings.abstain = true;
                    findings.reason("CALIBRATION_CORRECTION_SEMANTICS_UNQUALIFIED");
                    findings.exclude(&identity_label(target, &colliding_ids));
                    findings.exclude(&identity);
                    suppressed_certificates.insert(certificate.to_owned());
                    continue;
                }
                findings.abstain = true;
                findings.reason("CALIBRATION_CORRECTION_SEMANTICS_UNQUALIFIED");
                findings.exclude(&identity);
                suppressed_certificates.insert(certificate.to_owned());
            } else {
                findings.abstain = true;
                findings.reason("CORRECTION_TARGET_UNRESOLVED");
                findings.exclude(&identity);
                suppressed_certificates.insert(certificate.to_owned());
            }
        } else {
            findings.abstain = true;
            findings.reason("CORRECTION_TARGET_UNRESOLVED");
            findings.exclude(&identity);
            suppressed_certificates.insert(certificate.to_owned());
        }
    }

    for event in &events {
        if string(event, "event_type") != Some("reinstated") {
            continue;
        }
        let certificate = string(event, "certificate_id").unwrap_or("");
        suppressed_certificates.insert(certificate.to_owned());
        findings.abstain = true;
        findings.reason("CALIBRATION_REINSTATEMENT_SEMANTICS_UNQUALIFIED");
        findings.exclude(&identity_label(event, &colliding_ids));
        for prior in &events {
            if string(prior, "certificate_id") == Some(certificate)
                && string(prior, "event_type") == Some("revoked")
            {
                findings.exclude(&identity_label(prior, &colliding_ids));
            }
        }
    }

    let mut effective_event_count = 0usize;
    let mut effective_revocation_count = 0usize;
    let mut effective_suspension_count = 0usize;
    for event in &events {
        let key = source_key(event);
        let certificate = string(event, "certificate_id").unwrap_or("");
        let identity = identity_label(event, &colliding_ids);
        if issuer_excluded.contains(&key)
            || conflicted_keys.contains(&key)
            || cycles.contains(&key)
            || suppressed_certificates.contains(certificate)
        {
            continue;
        }
        if !validate_event_evidence(event, &mut findings, &identity) {
            continue;
        }
        if !check_signer_key(event, mode, trust_as_of, &mut findings, &identity) {
            continue;
        }

        let Some(effective_at) = unsigned(event, "effective_at_unix_ms") else {
            findings.abstain = true;
            findings.reason("EVENT_EFFECTIVE_TIME_UNRESOLVED");
            findings.exclude(&identity);
            continue;
        };
        let event_interval = TimeInterval {
            earliest_unix_ms: effective_at,
            latest_unix_ms: effective_at,
        };
        match compare_effectivity_intervals(event_interval, acquisition).relation {
            EffectivityRelation::DefinitelyNotYetEffective => {
                findings.not_yet_effective = true;
                findings.reason("EVENT_NOT_YET_EFFECTIVE");
                findings.exclude(&identity);
                continue;
            }
            EffectivityRelation::BoundaryAmbiguous => {
                findings.time_ambiguous = true;
                findings.reason("EVENT_EFFECTIVITY_CROSSES_ACQUISITION_INTERVAL");
                findings.exclude(&identity);
                continue;
            }
            EffectivityRelation::InvalidInterval => {
                findings.reject = true;
                findings.reason("EVENT_EFFECTIVE_TIME_INVALID");
                findings.exclude(&identity);
                continue;
            }
            EffectivityRelation::DefinitelyEffective => {}
        }

        effective_event_count += 1;
        match string(event, "event_type") {
            Some("issued") => {
                findings.applied.insert(identity);
            }
            Some("revoked") => {
                findings.applied.insert(identity);
                effective_revocation_count += 1;
            }
            Some("suspended") => {
                findings.applied.insert(identity);
                effective_suspension_count += 1;
            }
            Some("corrected" | "superseded" | "reinstated") => {}
            _ => {
                findings.abstain = true;
                findings.reason("EVENT_TYPE_UNQUALIFIED");
                findings.exclude(&identity);
            }
        }
    }

    if effective_revocation_count == 1 && effective_suspension_count == 0 {
        findings.block = true;
        findings.reason("REVOCATION_EFFECTIVE_AND_KNOWN");
    } else if effective_event_count > 1
        && (effective_revocation_count > 0 || effective_suspension_count > 0)
    {
        findings.block = true;
        findings.reason("CALIBRATION_EVENTS_KNOWN_AND_EFFECTIVE");
    } else if effective_suspension_count > 0 {
        findings.block = true;
        findings.reason("CALIBRATION_EVENTS_KNOWN_AND_EFFECTIVE");
    }

    findings.finish(mode)
}

pub fn evaluate_temporal_replay_case_all_modes(case: &Value) -> Vec<ReplayDisposition> {
    [
        TemporalQueryMode::AsKnownAt,
        TemporalQueryMode::RecomputedWithCurrentTrust,
        TemporalQueryMode::RecomputedWithCurrentEvidenceAndTrust,
    ]
    .into_iter()
    .map(|mode| evaluate_temporal_replay_case(case, mode))
    .collect()
}

#[cfg(test)]
mod tests {
    use super::{
        ReplayDisposition, TemporalQueryMode, evaluate_temporal_replay_case,
        evaluate_temporal_replay_case_all_modes,
    };
    use serde_json::Value;

    fn corpus() -> Value {
        serde_json::from_str(super::CORPUS).expect("temporal replay corpus parses")
    }

    fn assert_matches(actual: &ReplayDisposition, expected: &Value, case_id: &str) {
        assert_eq!(
            actual.disposition,
            expected["disposition"].as_str().unwrap(),
            "{case_id}: disposition"
        );
        let expected_reasons = expected["reason_codes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            actual.reason_codes, expected_reasons,
            "{case_id}: reason codes"
        );

        let expected_applied = expected["applied_event_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            actual.applied_event_ids, expected_applied,
            "{case_id}: applied event IDs"
        );

        let expected_excluded = expected["excluded_event_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect::<Vec<_>>();
        assert_eq!(
            actual.excluded_event_ids, expected_excluded,
            "{case_id}: excluded event IDs"
        );
    }

    #[test]
    fn all_frozen_cases_match_all_three_query_modes() {
        let corpus = corpus();
        assert_eq!(corpus["oracle_review_status"], "NOT_INDEPENDENTLY_REVIEWED");
        let cases = corpus["cases"].as_array().unwrap();
        assert_eq!(cases.len(), 29);

        for case in cases {
            let case_id = case["case_id"].as_str().unwrap();
            let actual = evaluate_temporal_replay_case_all_modes(case);
            for (index, mode) in [
                TemporalQueryMode::AsKnownAt,
                TemporalQueryMode::RecomputedWithCurrentTrust,
                TemporalQueryMode::RecomputedWithCurrentEvidenceAndTrust,
            ]
            .into_iter()
            .enumerate()
            {
                assert_matches(&actual[index], &case["expected"][mode.as_str()], case_id);
            }
        }
    }

    #[test]
    fn a_late_backdated_event_changes_only_the_full_retrospective_view() {
        let case = corpus()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == "backdated-event-received-after-snapshot")
            .unwrap()
            .clone();
        let result = evaluate_temporal_replay_case_all_modes(&case);
        assert_eq!(result[0].disposition, "NOT_KNOWN_AT_SNAPSHOT");
        assert_eq!(result[1].disposition, "NOT_KNOWN_AT_SNAPSHOT");
        assert_eq!(result[2].disposition, "BLOCK");
    }

    #[test]
    fn trust_only_replay_does_not_advance_the_original_evidence_cut() {
        let case = corpus()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == "backdated-event-received-after-snapshot")
            .unwrap()
            .clone();
        assert_eq!(
            evaluate_temporal_replay_case(&case, TemporalQueryMode::AsKnownAt),
            evaluate_temporal_replay_case(&case, TemporalQueryMode::RecomputedWithCurrentTrust)
        );
    }

    #[test]
    fn historical_trust_is_not_rewritten_by_current_key_compromise() {
        let case = corpus()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == "historical-policy-versus-current-key-compromise")
            .unwrap()
            .clone();
        assert_eq!(
            evaluate_temporal_replay_case(&case, TemporalQueryMode::AsKnownAt).disposition,
            "ACCEPTED_FOR_BINDING_ONLY"
        );
        assert_eq!(
            evaluate_temporal_replay_case(&case, TemporalQueryMode::RecomputedWithCurrentTrust)
            .disposition,
            "TRUST_UNRESOLVED"
        );
    }

    #[test]
    fn every_corpus_case_is_invariant_to_event_array_permutation() {
        for case in corpus()["cases"].as_array().unwrap() {
            let baseline = evaluate_temporal_replay_case_all_modes(case);
            let mut reversed = case.clone();
            let events = case["events"].as_array().unwrap();
            reversed["events"] = Value::Array(events.iter().cloned().rev().collect());
            assert_eq!(
                baseline,
                evaluate_temporal_replay_case_all_modes(&reversed),
                "{}",
                case["case_id"].as_str().unwrap()
            );
        }
    }

    #[test]
    fn issuer_recorded_time_does_not_substitute_for_receiver_first_seen() {
        let mut case = corpus()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == "known-effective-revocation")
            .unwrap()
            .clone();
        let baseline = evaluate_temporal_replay_case_all_modes(&case);
        case["events"][0]["issuer_recorded_at_unix_ms"] = Value::from(9_999_999);
        assert_eq!(baseline, evaluate_temporal_replay_case_all_modes(&case));
    }

    #[test]
    fn event_order_permutation_preserves_each_query_mode() {
        let case = corpus()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| {
                case["case_id"] == "same-local-id-in-distinct-source-namespaces-is-not-a-collision"
            })
            .unwrap()
            .clone();
        let baseline = evaluate_temporal_replay_case_all_modes(&case);
        let mut reversed = case.clone();
        let events = case["events"].as_array().unwrap();
        reversed["events"] = Value::Array(events.iter().cloned().rev().collect());
        assert_eq!(baseline, evaluate_temporal_replay_case_all_modes(&reversed));
    }
}
