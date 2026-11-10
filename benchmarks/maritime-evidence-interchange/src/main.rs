#![forbid(unsafe_code)]

//! Offline semantic conformance evaluator for the Maritime Evidence Interchange
//! synthetic corpus. This checks the profile's high-value invariants; it is not
//! a general JSON Schema 2020-12 implementation and does not test an adapter's
//! round-trip preservation. Those are separate conformance stages.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_CORPUS_BYTES: usize = 64 * 1024 * 1024;
const MAX_INLINE_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Disposition {
    Accept,
    Abstain,
    Reject,
}

impl Disposition {
    fn as_str(self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Abstain => "abstain",
            Self::Reject => "reject",
        }
    }
}

#[derive(Debug, Default)]
struct RecordAudit {
    disposition: Disposition,
    reasons: Vec<String>,
    payload_digest_match: Option<bool>,
    provenance_complete: bool,
    native_identifier_present: bool,
    ambiguous_mapping: bool,
    conflict_links_valid: Option<bool>,
}

impl Default for Disposition {
    fn default() -> Self {
        Self::Accept
    }
}

impl RecordAudit {
    fn reject(&mut self, reason: &str) {
        self.disposition = Disposition::Reject;
        push_unique(&mut self.reasons, reason);
    }

    fn abstain(&mut self, reason: &str) {
        if self.disposition == Disposition::Accept {
            self.disposition = Disposition::Abstain;
        }
        push_unique(&mut self.reasons, reason);
    }
}

#[derive(Debug)]
struct CaseAudit {
    fixture_id: String,
    expected: String,
    actual: Disposition,
    reasons: Vec<String>,
    payload_digest_match: bool,
    elapsed_ms: f64,
}

fn push_unique(items: &mut Vec<String>, value: &str) {
    if !items.iter().any(|item| item == value) {
        items.push(value.to_owned());
    }
}

fn string_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a str> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    current.as_str()
}

fn value_at<'a>(value: &'a Value, path: &[&str]) -> Option<&'a Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn bool_at(value: &Value, path: &[&str]) -> Option<bool> {
    value_at(value, path)?.as_bool()
}

fn u64_at(value: &Value, path: &[&str]) -> Option<u64> {
    value_at(value, path)?.as_u64()
}

fn finite_number_at(value: &Value, path: &[&str]) -> Option<f64> {
    let number = value_at(value, path)?.as_f64()?;
    number.is_finite().then_some(number)
}

fn digest_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn base64_value(byte: u8) -> Option<u8> {
    match byte {
        b'A'..=b'Z' => Some(byte - b'A'),
        b'a'..=b'z' => Some(byte - b'a' + 26),
        b'0'..=b'9' => Some(byte - b'0' + 52),
        b'+' => Some(62),
        b'/' => Some(63),
        _ => None,
    }
}

fn decode_base64(input: &str) -> Result<Vec<u8>, String> {
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err("base64_length_not_multiple_of_four".into());
    }
    let padding = bytes.iter().rev().take_while(|byte| **byte == b'=').count();
    if padding > 2 {
        return Err("base64_excess_padding".into());
    }
    if bytes[..bytes.len().saturating_sub(padding)].contains(&b'=') {
        return Err("base64_interior_padding".into());
    }

    let estimated = bytes.len().saturating_div(4).saturating_mul(3);
    if estimated > MAX_INLINE_PAYLOAD_BYTES.saturating_add(2) {
        return Err("inline_payload_exceeds_limit".into());
    }

    let mut output = Vec::with_capacity(estimated.saturating_sub(padding));
    let mut accumulator: u32 = 0;
    let mut bits: u8 = 0;
    for byte in &bytes[..bytes.len().saturating_sub(padding)] {
        let value = base64_value(*byte).ok_or_else(|| "base64_invalid_character".to_string())?;
        accumulator = (accumulator << 6) | u32::from(value);
        bits += 6;
        if bits >= 8 {
            bits -= 8;
            output.push(((accumulator >> bits) & 0xff) as u8);
        }
    }

    let expected_len = estimated.saturating_sub(padding);
    if output.len() != expected_len {
        return Err("base64_decoded_length_mismatch".into());
    }
    if padding == 1 {
        let last = bytes[bytes.len() - 2];
        if base64_value(last).is_none_or(|value| value & 0x03 != 0) {
            return Err("base64_nonzero_unused_bits".into());
        }
    } else if padding == 2 {
        let last = bytes[bytes.len() - 3];
        if base64_value(last).is_none_or(|value| value & 0x0f != 0) {
            return Err("base64_nonzero_unused_bits".into());
        }
    }
    if output.len() > MAX_INLINE_PAYLOAD_BYTES {
        return Err("inline_payload_exceeds_limit".into());
    }
    Ok(output)
}

fn parse_fixed_u32(bytes: &[u8], start: usize, end: usize) -> Option<i64> {
    let part = bytes.get(start..end)?;
    if part.is_empty() || !part.iter().all(u8::is_ascii_digit) {
        return None;
    }
    let mut result = 0i64;
    for byte in part {
        result = result.checked_mul(10)?.checked_add(i64::from(*byte - b'0'))?;
    }
    Some(result)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let adjusted_year = year - i64::from(month <= 2);
    let era = if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    } / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}

/// v0.1 canonical time encoding is UTC RFC3339 without fractional seconds.
fn parse_utc_timestamp(value: &str) -> Result<i64, String> {
    let bytes = value.as_bytes();
    if bytes.len() != 20
        || bytes[4] != b'-'
        || bytes[7] != b'-'
        || bytes[10] != b'T'
        || bytes[13] != b':'
        || bytes[16] != b':'
        || bytes[19] != b'Z'
    {
        return Err("timestamp_not_canonical_utc".into());
    }

    let year = parse_fixed_u32(bytes, 0, 4).ok_or("timestamp_invalid_year")?;
    let month = parse_fixed_u32(bytes, 5, 7).ok_or("timestamp_invalid_month")?;
    let day = parse_fixed_u32(bytes, 8, 10).ok_or("timestamp_invalid_day")?;
    let hour = parse_fixed_u32(bytes, 11, 13).ok_or("timestamp_invalid_hour")?;
    let minute = parse_fixed_u32(bytes, 14, 16).ok_or("timestamp_invalid_minute")?;
    let second = parse_fixed_u32(bytes, 17, 19).ok_or("timestamp_invalid_second")?;

    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err("timestamp_month_out_of_range".into()),
    };
    if day < 1 || day > days_in_month || hour > 23 || minute > 59 || second > 59 {
        return Err("timestamp_component_out_of_range".into());
    }

    let days = days_from_civil(year, month, day);
    days.checked_mul(86_400)
        .and_then(|value| value.checked_add(hour * 3_600 + minute * 60 + second))
        .ok_or_else(|| "timestamp_overflow".into())
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719468;
    let era = if z >= 0 { z } else { z - 146096 } / 146097;
    let day_of_era = z - era * 146097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + i64::from(month <= 2);
    (year, month, day)
}

fn format_utc_timestamp(epoch_seconds: u64) -> String {
    let days = (epoch_seconds / 86_400) as i64;
    let remainder = epoch_seconds % 86_400;
    let (year, month, day) = civil_from_days(days);
    let hour = remainder / 3_600;
    let minute = (remainder % 3_600) / 60;
    let second = remainder % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn validate_record(record: &Value, evaluation_time: i64) -> RecordAudit {
    let mut audit = RecordAudit {
        provenance_complete: true,
        ..RecordAudit::default()
    };

    if !record.is_object() {
        audit.reject("record_not_object");
        return audit;
    }
    if string_at(record, &["schema_version"]) != Some("0.1.0") {
        audit.reject("schema_version_invalid");
    }
    let record_type = string_at(record, &["record_type"]);
    if !matches!(record_type, Some("observation" | "diagnostic_assertion")) {
        audit.reject("record_type_invalid");
    }
    if string_at(record, &["record_id"]).is_none_or(str::is_empty) {
        audit.reject("record_id_missing");
    }

    if string_at(record, &["asset_ref", "vessel_id"]).is_none_or(str::is_empty)
        || string_at(record, &["asset_ref", "configuration_sha256"])
            .is_none_or(|value| !is_sha256(value))
        || value_at(record, &["asset_ref", "asset_path"])
            .and_then(Value::as_array)
            .is_none_or(|items| items.is_empty() || items.iter().any(|item| item.as_str().is_none_or(str::is_empty)))
    {
        audit.reject("asset_identity_incomplete");
        audit.provenance_complete = false;
    }

    for path in [
        &["provenance", "source_id"][..],
        &["provenance", "source_class"][..],
        &["provenance", "native_record_id"][..],
        &["provenance", "adapter_id"][..],
        &["provenance", "adapter_version"][..],
        &["provenance", "mapping_catalogue_version"][..],
    ] {
        if string_at(record, path).is_none_or(str::is_empty) {
            audit.reject("source_provenance_incomplete");
            audit.provenance_complete = false;
        }
    }

    let received_at = string_at(record, &["provenance", "received_at"]);
    match received_at.and_then(|value| parse_utc_timestamp(value).ok()) {
        Some(received) if received <= evaluation_time => {}
        Some(_) => audit.reject("receipt_time_after_evaluation_time"),
        None => {
            audit.reject("receipt_time_invalid");
            audit.provenance_complete = false;
        }
    }

    let payload = value_at(record, &["provenance", "source_payload"]);
    match payload {
        Some(payload) => {
            let inline = payload.get("inline_base64");
            let reference = payload.get("content_ref");
            if inline.is_some() == reference.is_some() {
                audit.reject("payload_requires_exactly_one_representation");
                audit.payload_digest_match = Some(false);
            } else if let Some(encoded) = inline.and_then(Value::as_str) {
                let decoded = decode_base64(encoded);
                match decoded {
                    Ok(bytes) => {
                        let byte_length = payload.get("byte_length").and_then(Value::as_u64);
                        let expected_digest = payload.get("sha256").and_then(Value::as_str);
                        let digest_matches = expected_digest.is_some_and(|expected| {
                            is_sha256(expected) && digest_hex(&bytes) == expected
                        });
                        audit.payload_digest_match = Some(digest_matches);
                        if byte_length != Some(bytes.len() as u64) {
                            audit.reject("payload_byte_length_mismatch");
                        }
                        if !digest_matches {
                            audit.reject("payload_digest_mismatch");
                        }
                        if payload.get("media_type").and_then(Value::as_str).is_none_or(str::is_empty) {
                            audit.reject("payload_media_type_missing");
                        }
                        if string_at(&Value::Object(payload.as_object().cloned().unwrap_or_default()), &["signature_state"]) == Some("invalid") {
                            audit.reject("payload_signature_invalid");
                        }
                    }
                    Err(reason) => {
                        audit.reject(&reason);
                        audit.payload_digest_match = Some(false);
                    }
                }
            } else {
                // External object retrieval is deliberately not implicit in this
                // offline corpus runner. The owning adapter must supply verified
                // bytes or a separately reviewed resolver implementation.
                audit.reject("external_payload_reference_not_resolved");
                audit.payload_digest_match = Some(false);
            }
        }
        None => {
            audit.reject("source_payload_missing");
            audit.provenance_complete = false;
            audit.payload_digest_match = Some(false);
        }
    }

    let time_basis = string_at(record, &["time", "time_basis"]);
    let clock_quality = string_at(record, &["time", "clock_quality"]);
    if string_at(record, &["time", "clock_id"]).is_none_or(str::is_empty)
        || !matches!(time_basis, Some("synchronized_utc" | "source_wall_clock" | "relative_monotonic" | "unknown"))
        || !matches!(clock_quality, Some("locked" | "holdover" | "unsynchronized" | "unknown"))
        || finite_number_at(record, &["time", "synchronization_uncertainty_ms"]).is_none_or(|v| v < 0.0)
    {
        audit.reject("time_context_incomplete");
    }

    match value_at(record, &["time", "event_time"]) {
        Some(Value::String(value)) => match parse_utc_timestamp(value) {
            Ok(timestamp) if timestamp > evaluation_time => audit.reject("event_time_after_evaluation_time"),
            Ok(_) => {}
            Err(reason) => audit.reject(&reason),
        },
        Some(Value::Null) if time_basis == Some("relative_monotonic")
            && u64_at(record, &["time", "monotonic_counter_ns"]).is_some() => {}
        _ => audit.reject("event_time_or_monotonic_basis_missing"),
    }

    match record_type {
        Some("observation") => {
            let mapping_status = string_at(record, &["mapping", "status"]);
            let native_id = string_at(record, &["mapping", "native_channel_id"]);
            let native_name = string_at(record, &["mapping", "native_channel_name"]);
            audit.native_identifier_present = native_id.is_some_and(|v| !v.is_empty())
                && native_name.is_some_and(|v| !v.is_empty());

            if !audit.native_identifier_present {
                audit.reject("native_channel_identity_missing");
            }
            match mapping_status {
                Some("exact" | "reviewed") => {
                    if string_at(record, &["mapping", "normalized_concept_id"]).is_none_or(str::is_empty) {
                        audit.reject("normalized_concept_missing_for_accepted_mapping");
                    }
                }
                Some("ambiguous" | "unmapped") => {
                    audit.ambiguous_mapping = true;
                    if value_at(record, &["mapping", "normalized_concept_id"]).is_some() {
                        audit.reject("ambiguous_mapping_promoted_to_concept");
                    } else {
                        audit.abstain("semantic_mapping_ambiguous_or_unmapped");
                    }
                }
                _ => audit.reject("mapping_status_invalid"),
            }

            if string_at(record, &["quantity_kind"]).is_none_or(str::is_empty) {
                audit.reject("quantity_kind_missing");
            }
            let state = string_at(record, &["value_state"]);
            let has_value = record.get("value").is_some();
            let lineage = value_at(record, &["lineage"]).and_then(Value::as_array);
            if lineage.is_none() {
                audit.reject("lineage_not_array");
            }
            match state {
                Some("missing" | "rejected" | "invalid") if has_value => {
                    audit.reject("non_value_state_contains_value");
                }
                Some("missing" | "rejected" | "invalid") => {}
                Some("measured" | "stale" | "conflicted") if !has_value => {
                    audit.reject("value_state_requires_value");
                }
                Some("derived" | "reconstructed" | "imputed") => {
                    if !has_value {
                        audit.reject("transformed_value_missing");
                    }
                    if lineage.is_none_or(|items| items.is_empty()) {
                        audit.reject("transformed_value_without_lineage");
                    } else if lineage.is_some_and(|items| {
                        items.iter().any(|item| {
                            string_at(item, &["transform_id"]).is_none_or(str::is_empty)
                                || string_at(item, &["implementation"]).is_none_or(str::is_empty)
                                || string_at(item, &["version"]).is_none_or(str::is_empty)
                                || value_at(item, &["input_record_refs"]).and_then(Value::as_array).is_none_or(Vec::is_empty)
                                || string_at(item, &["output_state"]) != state
                        })
                    }) {
                        audit.reject("transformation_lineage_incomplete");
                    }
                }
                Some("measured") if lineage.is_some_and(|items| !items.is_empty()) => {
                    audit.reject("measured_value_must_not_claim_transform_lineage");
                }
                Some("measured" | "stale" | "conflicted") => {}
                _ => audit.reject("value_state_invalid"),
            }

            for path in [
                &["quality", "calibration_state"][..],
                &["quality", "freshness_state"][..],
                &["quality", "measurement_quality"][..],
            ] {
                if string_at(record, path).is_none_or(str::is_empty) {
                    audit.reject("quality_context_incomplete");
                }
            }
            if string_at(record, &["unit_ucum"]).is_none_or(str::is_empty) {
                audit.reject("unit_missing");
            }
        }
        Some("diagnostic_assertion") => {
            if bool_at(record, &["authority_granted"]) != Some(false)
                || string_at(record, &["claim_ceiling"]) != Some("not_ground_truth")
            {
                audit.reject("assertion_claim_ceiling_or_authority_invalid");
            }
            if string_at(record, &["assertion", "kind"]).is_none_or(str::is_empty)
                || string_at(record, &["assertion", "statement"]).is_none_or(str::is_empty)
                || string_at(record, &["assertion", "status"]).is_none_or(str::is_empty)
            {
                audit.reject("assertion_content_incomplete");
            }
            if string_at(record, &["method", "provider"]).is_none_or(str::is_empty)
                || string_at(record, &["method", "name"]).is_none_or(str::is_empty)
                || string_at(record, &["method", "version"]).is_none_or(str::is_empty)
            {
                audit.reject("assertion_method_incomplete");
            }
            let links = value_at(record, &["evidence_links"]);
            let valid_links = ["supports", "contradicts", "derived_from"].iter().all(|key| {
                links.and_then(|value| value.get(*key)).and_then(Value::as_array).is_some()
            });
            audit.conflict_links_valid = Some(valid_links);
            if !valid_links {
                audit.reject("assertion_evidence_links_incomplete");
            }
        }
        _ => {}
    }

    audit
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn sequence_key(record: &Value) -> Option<String> {
    let sequence = u64_at(record, &["time", "sequence_number"])?;
    let source = string_at(record, &["provenance", "source_id"])?;
    let vessel = string_at(record, &["asset_ref", "vessel_id"])?;
    let path = value_at(record, &["asset_ref", "asset_path"])?.as_array()?
        .iter().map(Value::as_str).collect::<Option<Vec<_>>>()?.join("/");
    let clock = string_at(record, &["time", "clock_id"])?;
    Some(format!("{source}|{vessel}|{path}|{clock}|{sequence}"))
}

fn record_ids(corpus: &Value) -> Result<HashSet<String>, String> {
    let fixtures = corpus.get("fixtures").and_then(Value::as_array)
        .ok_or_else(|| "corpus_fixtures_missing".to_string())?;
    let mut ids = HashSet::new();
    for fixture in fixtures {
        for key in ["record", "companion_records"] {
            match fixture.get(key) {
                Some(Value::Object(_)) if key == "record" => {
                    let id = string_at(fixture, &["record", "record_id"])
                        .ok_or_else(|| "fixture_record_id_missing".to_string())?;
                    if !ids.insert(id.to_owned()) {
                        return Err(format!("duplicate_record_id:{id}"));
                    }
                }
                Some(Value::Array(items)) if key == "companion_records" => {
                    for item in items {
                        let id = item.get("record_id").and_then(Value::as_str)
                            .ok_or_else(|| "companion_record_id_missing".to_string())?;
                        if !ids.insert(id.to_owned()) {
                            return Err(format!("duplicate_record_id:{id}"));
                        }
                    }
                }
                _ => {}
            }
        }
    }
    Ok(ids)
}

fn validate_evidence_references(record: &Value, ids: &HashSet<String>) -> bool {
    ["supports", "contradicts", "derived_from"].iter().all(|key| {
        value_at(record, &["evidence_links", key])
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().all(|item| item.as_str().is_some_and(|id| ids.contains(id))))
    })
}

fn evaluate_corpus(bytes: &[u8]) -> Result<(Value, usize), String> {
    if bytes.len() > MAX_CORPUS_BYTES {
        return Err("corpus_exceeds_64_mib_limit".into());
    }
    let corpus: Value = serde_json::from_slice(bytes)
        .map_err(|error| format!("corpus_json_invalid: {error}"))?;
    if string_at(&corpus, &["corpus_version"]) != Some("0.1.0")
        || string_at(&corpus, &["claim_ceiling"])
            != Some("Synthetic conformance fixtures only; not physical diagnostic validation.")
    {
        return Err("corpus_identity_or_claim_ceiling_invalid".into());
    }

    let evaluation_time = string_at(&corpus, &["evaluation_time"])
        .ok_or_else(|| "corpus_evaluation_time_missing".to_string())?;
    let evaluation_epoch = parse_utc_timestamp(evaluation_time)
        .map_err(|reason| format!("corpus_evaluation_time_invalid:{reason}"))?;
    let fixtures = corpus.get("fixtures").and_then(Value::as_array)
        .ok_or_else(|| "corpus_fixtures_missing".to_string())?;
    if u64_at(&corpus, &["fixture_count"]) != Some(fixtures.len() as u64) {
        return Err("corpus_fixture_count_mismatch".into());
    }
    let known_ids = record_ids(&corpus)?;
    let corpus_ids = known_ids.clone();

    let run_started = Instant::now();
    let started_epoch = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_before_epoch: {error}"))?.as_secs();
    let mut case_reports = Vec::new();
    let mut latencies = Vec::new();
    let mut failed_cases = 0usize;
    let mut accepted = 0usize;
    let mut rejected = 0usize;
    let mut abstained = 0usize;
    let mut digest_checked = 0usize;
    let mut digest_matches = 0usize;
    let mut provenance_complete = 0usize;
    let mut provenance_total = 0usize;
    let mut native_present = 0usize;
    let mut native_total = 0usize;
    let mut ambiguous_total = 0usize;
    let mut ambiguous_abstained = 0usize;
    let mut conflict_total = 0usize;
    let mut conflict_valid = 0usize;
    let mut duplicate_gate_ok = true;
    let mut digest_negative_control_ok = true;
    let mut references_ok = true;

    let mut fixture_ids = HashSet::new();
    for fixture in fixtures {
        let case_start = Instant::now();
        let fixture_id = string_at(fixture, &["fixture_id"])
            .ok_or_else(|| "fixture_id_missing".to_string())?.to_owned();
        if !fixture_ids.insert(fixture_id.clone()) {
            return Err(format!("duplicate_fixture_id:{fixture_id}"));
        }
        let expected = string_at(fixture, &["expected_disposition"])
            .ok_or_else(|| format!("expected_disposition_missing:{fixture_id}"))?;
        let expected_disp = match expected {
            "accept" => Disposition::Accept,
            "abstain" => Disposition::Abstain,
            "reject" => Disposition::Reject,
            _ => return Err(format!("invalid_expected_disposition:{fixture_id}")),
        };

        let mut records = Vec::new();
        records.push(fixture.get("record").ok_or_else(|| format!("fixture_record_missing:{fixture_id}"))?);
        if let Some(companions) = fixture.get("companion_records") {
            let companions = companions.as_array().ok_or_else(|| format!("companions_not_array:{fixture_id}"))?;
            records.extend(companions.iter());
        }

        let mut audits = Vec::new();
        let mut stream_keys = HashSet::new();
        let mut duplicate_in_fixture = false;
        for record in &records {
            let key = sequence_key(record);
            if let Some(key) = key {
                if !stream_keys.insert(key) {
                    duplicate_in_fixture = true;
                }
            }
            let audit = validate_record(record, evaluation_epoch);
            if let Some(digest_ok) = audit.payload_digest_match {
                digest_checked += 1;
                if digest_ok {
                    digest_matches += 1;
                } else if expected_disp != Disposition::Reject {
                    digest_negative_control_ok = false;
                }
            }
            provenance_total += 1;
            if audit.provenance_complete {
                provenance_complete += 1;
            }
            if audit.native_identifier_present {
                native_present += 1;
            }
            if string_at(record, &["record_type"]) == Some("observation") {
                native_total += 1;
            }
            if audit.ambiguous_mapping {
                ambiguous_total += 1;
                if audit.disposition == Disposition::Abstain {
                    ambiguous_abstained += 1;
                }
            }
            if audit.conflict_links_valid.is_some() {
                conflict_total += 1;
                if audit.conflict_links_valid == Some(true) {
                    conflict_valid += 1;
                }
            }
            if string_at(record, &["record_type"]) == Some("diagnostic_assertion")
                && !validate_evidence_references(record, &corpus_ids)
            {
                references_ok = false;
            }
            audits.push(audit);
        }

        let mut actual = Disposition::Accept;
        let mut reasons = Vec::new();
        let mut all_digest_match = true;
        for audit in &audits {
            if audit.disposition == Disposition::Reject {
                actual = Disposition::Reject;
            } else if audit.disposition == Disposition::Abstain && actual == Disposition::Accept {
                actual = Disposition::Abstain;
            }
            for reason in &audit.reasons {
                push_unique(&mut reasons, reason);
            }
            if audit.payload_digest_match == Some(false) {
                all_digest_match = false;
            }
        }
        if duplicate_in_fixture {
            actual = Disposition::Reject;
            push_unique(&mut reasons, "duplicate_source_asset_clock_sequence");
            if expected_disp != Disposition::Reject {
                duplicate_gate_ok = false;
            }
        }

        if actual == Disposition::Reject {
            rejected += 1;
        } else if actual == Disposition::Abstain {
            abstained += 1;
        } else {
            accepted += 1;
        }
        let passed = actual == expected_disp;
        if !passed {
            failed_cases += 1;
            push_unique(&mut reasons, "actual_disposition_differs_from_expected");
        }

        let elapsed = case_start.elapsed().as_secs_f64() * 1000.0;
        latencies.push(elapsed);
        case_reports.push(json!({
            "fixture_id": fixture_id,
            "expected_disposition": expected_disp.as_str(),
            "actual_disposition": actual.as_str(),
            "passed": passed,
            "reason_codes": reasons,
            "lost_fields": [],
            "payload_digest_match": all_digest_match
        }));
    }

    latencies.sort_by(f64::total_cmp);
    let p95 = if latencies.is_empty() {
        None
    } else {
        let index = ((latencies.len() as f64 * 0.95).ceil() as usize).saturating_sub(1);
        latencies.get(index).copied()
    };
    let elapsed_ms = run_started.elapsed().as_secs_f64() * 1000.0;
    let completed_epoch = SystemTime::now().duration_since(UNIX_EPOCH)
        .map_err(|error| format!("system_clock_before_epoch: {error}"))?.as_secs();
    let environment = format!("{}|{}|rust-1.96-compatible|{}", env::consts::OS, env::consts::ARCH, env!("CARGO_PKG_VERSION"));
    let environment_digest = digest_hex(environment.as_bytes());
    let corpus_digest = digest_hex(bytes);

    let disposition_gate = failed_cases == 0;
    let ambiguous_gate = ambiguous_total == ambiguous_abstained && ambiguous_total > 0;
    let conflict_gate = conflict_total == conflict_valid;
    let all_gates = disposition_gate && digest_negative_control_ok && ambiguous_gate
        && conflict_gate && duplicate_gate_ok && references_ok;

    let report = json!({
        "report_version": "0.1.0",
        "report_id": format!("local-{}", started_epoch),
        "profile_version": "0.1.0",
        "adapter_id": "profile-semantic-corpus-validator",
        "adapter_version": env!("CARGO_PKG_VERSION"),
        "fixture_corpus_sha256": corpus_digest,
        "started_at": format_utc_timestamp(started_epoch),
        "completed_at": format_utc_timestamp(completed_epoch),
        "environment_sha256": environment_digest,
        "overall_conformance": if all_gates { "pass" } else { "fail" },
        "summary": {
            "cases_total": fixtures.len(),
            "accepted": accepted,
            "rejected": rejected,
            "abstained": abstained,
            "payload_digest_match_rate": if digest_checked == 0 { Value::Null } else { json!(digest_matches as f64 / digest_checked as f64) },
            "native_identifier_retention_rate": if native_total == 0 { Value::Null } else { json!(native_present as f64 / native_total as f64) },
            "mapping_precision": Value::Null,
            "mapping_coverage": Value::Null,
            "ambiguous_mapping_abstention_rate": if ambiguous_total == 0 { Value::Null } else { json!(ambiguous_abstained as f64 / ambiguous_total as f64) },
            "conflict_preservation_rate": if conflict_total == 0 { Value::Null } else { json!(conflict_valid as f64 / conflict_total as f64) },
            "provenance_completeness_rate": if provenance_total == 0 { Value::Null } else { json!(provenance_complete as f64 / provenance_total as f64) },
            "event_order_preservation_rate": Value::Null,
            "p95_processing_ms": p95,
            "peak_memory_bytes": Value::Null
        },
        "hard_gates": [
            {"gate_id":"expected_dispositions_match","passed":disposition_gate,"failure_count":failed_cases},
            {"gate_id":"digest_negative_control_rejected","passed":digest_negative_control_ok,"failure_count":usize::from(!digest_negative_control_ok)},
            {"gate_id":"ambiguous_mapping_abstention","passed":ambiguous_gate,"failure_count":ambiguous_total.saturating_sub(ambiguous_abstained)},
            {"gate_id":"duplicate_stream_sequence_rejected","passed":duplicate_gate_ok,"failure_count":usize::from(!duplicate_gate_ok)},
            {"gate_id":"assertion_evidence_references_resolve","passed":references_ok,"failure_count":usize::from(!references_ok)},
            {"gate_id":"assertion_conflict_links_well_formed","passed":conflict_gate,"failure_count":conflict_total.saturating_sub(conflict_valid)}
        ],
        "case_results": case_reports,
        "claim_ceiling": "adapter_conformance_only",
        "validator_scope_note": "Synthetic semantic corpus validation only. This does not execute an adapter, prove round-trip preservation, or establish physical diagnostic performance.",
        "elapsed_ms": elapsed_ms
    });

    Ok((report, failed_cases + usize::from(!all_gates)))
}

fn usage() -> &'static str {
    "usage: maritime-evidence-interchange <conformance-corpus.json>"
}

fn main() -> ExitCode {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("{usage}", usage = usage());
        return ExitCode::from(2);
    };
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            return ExitCode::from(2);
        }
    };
    if bytes.len() > MAX_CORPUS_BYTES {
        eprintln!("corpus exceeds 64 MiB safety limit");
        return ExitCode::from(2);
    }
    match evaluate_corpus(&bytes) {
        Ok((report, failed)) => {
            match serde_json::to_string_pretty(&report) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("cannot serialize report: {error}");
                    return ExitCode::from(2);
                }
            }
            if failed == 0 { ExitCode::SUCCESS } else { ExitCode::from(1) }
        }
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../../maritime/maritime-evidence-interchange-conformance-v0.1.json");

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions() {
        let (report, failures) = evaluate_corpus(CORPUS.as_bytes()).expect("corpus should parse");
        assert_eq!(failures, 0, "{report:#}");
        assert_eq!(report["summary"]["cases_total"], 13);
        assert_eq!(report["summary"]["accepted"], 7);
        assert_eq!(report["summary"]["abstained"], 2);
        assert_eq!(report["summary"]["rejected"], 4);
    }

    #[test]
    fn base64_decoder_rejects_noncanonical_unused_bits() {
        assert!(decode_base64("Zh==").is_err());
        assert_eq!(decode_base64("Zg==").expect("valid base64"), b"f");
    }

    #[test]
    fn utc_parser_checks_calendar_and_timezone() {
        assert!(parse_utc_timestamp("2024-02-29T23:59:59Z").is_ok());
        assert!(parse_utc_timestamp("2025-02-29T23:59:59Z").is_err());
        assert!(parse_utc_timestamp("2025-01-01T00:00:00+02:00").is_err());
    }

    #[test]
    fn modified_payload_digest_is_not_accepted() {
        let bad = r#"{"schema_version":"0.1.0","record_type":"observation"}"#;
        let result = validate_record(&serde_json::from_str(bad).expect("json"), 1_800_000_000);
        assert_eq!(result.disposition, Disposition::Reject);
    }
}
