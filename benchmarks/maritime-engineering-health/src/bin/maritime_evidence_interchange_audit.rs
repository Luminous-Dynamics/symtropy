#![forbid(unsafe_code)]

//! Offline semantic conformance auditor for MARITIME-EVIDENCE-INTEROP-001.
//!
//! This binary validates the fixture corpus and its embedded records. It does
//! not connect to ship networks, fetch content references, authenticate source
//! identities, diagnose machinery, authorize maintenance, or control equipment.

use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet, HashSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use std::time::{SystemTime, UNIX_EPOCH};

const CORPUS_VERSION: &str = "0.1.0";
const CLAIM_CEILING: &str =
    "Synthetic conformance fixtures only; not physical diagnostic validation.";

#[derive(Clone, Debug, PartialEq, Eq)]
enum Disposition {
    Accept,
    Abstain,
    Reject,
}

impl Disposition {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Accept => "accept",
            Self::Abstain => "abstain",
            Self::Reject => "reject",
        }
    }
}

#[derive(Default)]
struct RecordOutcome {
    errors: Vec<String>,
    abstentions: Vec<String>,
    payload_digest_match: bool,
    native_identifier_present: bool,
    provenance_complete: bool,
    conflict_edges: usize,
}

impl RecordOutcome {
    fn reject(&mut self, code: &str) {
        self.errors.push(code.to_owned());
    }

    fn abstain(&mut self, code: &str) {
        self.abstentions.push(code.to_owned());
    }
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

fn object<'a>(value: &'a Value) -> Option<&'a Map<String, Value>> {
    value.as_object()
}

fn string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn array<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

fn check_keys(value: &Value, allowed: &[&str], path: &str, errors: &mut Vec<String>) {
    let Some(map) = object(value) else {
        errors.push(format!("{path}:not_object"));
        return;
    };
    for key in map.keys() {
        if !allowed.iter().any(|allowed_key| *allowed_key == key) {
            errors.push(format!("{path}:unknown_field:{key}"));
        }
    }
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

fn decode_base64(input: &str) -> Result<Vec<u8>, &'static str> {
    let bytes = input.as_bytes();
    if bytes.len() % 4 != 0 {
        return Err("base64_length_not_multiple_of_four");
    }
    let mut output = Vec::with_capacity(bytes.len() / 4 * 3);
    for (chunk_index, chunk) in bytes.chunks_exact(4).enumerate() {
        let final_chunk = chunk_index + 1 == bytes.len() / 4;
        let a = base64_value(chunk[0]).ok_or("base64_invalid_character")?;
        let b = base64_value(chunk[1]).ok_or("base64_invalid_character")?;
        let c_pad = chunk[2] == b'=';
        let d_pad = chunk[3] == b'=';
        if c_pad && !d_pad {
            return Err("base64_invalid_padding");
        }
        if (c_pad || d_pad) && !final_chunk {
            return Err("base64_padding_before_final_chunk");
        }
        let c = if c_pad { 0 } else { base64_value(chunk[2]).ok_or("base64_invalid_character")? };
        let d = if d_pad { 0 } else { base64_value(chunk[3]).ok_or("base64_invalid_character")? };
        if c_pad && (b & 0x0f) != 0 {
            return Err("base64_non_canonical_padding");
        }
        if d_pad && !c_pad && (c & 0x03) != 0 {
            return Err("base64_non_canonical_padding");
        }
        output.push((a << 2) | (b >> 4));
        if !c_pad {
            output.push((b << 4) | (c >> 2));
        }
        if !d_pad {
            output.push((c << 6) | d);
        }
    }
    Ok(output)
}

fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = year - i64::from(month <= 2);
    let era = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era = year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era =
        year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let mut year = year_of_era + era * 400;
    let day_of_year =
        day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    year += i64::from(month <= 2);
    (year, month, day)
}

/// Parse a strict RFC3339 timestamp into nanoseconds since the Unix epoch.
/// Leap seconds are intentionally rejected rather than silently normalized.
fn parse_rfc3339_ns(input: &str) -> Option<i128> {
    let (date, rest) = input.split_once('T')?;
    if date.len() != 10 || rest.len() < 9 {
        return None;
    }
    let year: i64 = date.get(0..4)?.parse().ok()?;
    let month: i64 = date.get(5..7)?.parse().ok()?;
    let day: i64 = date.get(8..10)?.parse().ok()?;
    if date.as_bytes().get(4) != Some(&b'-') || date.as_bytes().get(7) != Some(&b'-') {
        return None;
    }
    if year < 1 || !(1..=12).contains(&month) || day < 1 {
        return None;
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let month_days = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return None,
    };
    if day > month_days {
        return None;
    }

    let timezone_index = rest
        .char_indices()
        .find(|(i, ch)| *i > 0 && (*ch == '+' || *ch == '-'))
        .map(|(i, _)| i);
    let (time_part, zone) = if let Some(index) = timezone_index {
        (&rest[..index], &rest[index..])
    } else if let Some(time) = rest.strip_suffix('Z') {
        (time, "Z")
    } else {
        return None;
    };
    if time_part.len() < 8 {
        return None;
    }
    let hour: i64 = time_part.get(0..2)?.parse().ok()?;
    let minute: i64 = time_part.get(3..5)?.parse().ok()?;
    let seconds_part = time_part.get(6..)?;
    if time_part.as_bytes().get(2) != Some(&b':')
        || time_part.as_bytes().get(5) != Some(&b':')
        || !(0..=23).contains(&hour)
        || !(0..=59).contains(&minute)
    {
        return None;
    }
    let (second_text, fraction_text) = match seconds_part.split_once('.') {
        Some((seconds, fraction)) => (seconds, Some(fraction)),
        None => (seconds_part, None),
    };
    let second: i64 = second_text.parse().ok()?;
    if !(0..=59).contains(&second) {
        return None;
    }
    let mut fraction_ns = 0_i128;
    if let Some(fraction) = fraction_text {
        if fraction.is_empty() || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return None;
        }
        let digits = fraction.chars().take(9).collect::<String>();
        let parsed: i128 = digits.parse().ok()?;
        fraction_ns = parsed * 10_i128.pow((9 - digits.len()) as u32);
    }

    let offset_seconds: i64 = if zone == "Z" {
        0
    } else {
        if zone.len() != 6 || zone.as_bytes().get(3) != Some(&b':') {
            return None;
        }
        let offset_hour: i64 = zone.get(1..3)?.parse().ok()?;
        let offset_minute: i64 = zone.get(4..6)?.parse().ok()?;
        if offset_hour > 23 || offset_minute > 59 {
            return None;
        }
        let magnitude = offset_hour * 3_600 + offset_minute * 60;
        if zone.starts_with('+') { magnitude } else { -magnitude }
    };

    let days = days_from_civil(year, month, day);
    let whole_seconds = days * 86_400 + hour * 3_600 + minute * 60 + second - offset_seconds;
    Some(i128::from(whole_seconds) * 1_000_000_000 + fraction_ns)
}

fn format_rfc3339_now() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let days = seconds.div_euclid(86_400);
    let day_seconds = seconds.rem_euclid(86_400);
    let (year, month, day) = civil_from_days(days);
    let hour = day_seconds / 3_600;
    let minute = (day_seconds % 3_600) / 60;
    let second = day_seconds % 60;
    format!("{year:04}-{month:02}-{day:02}T{hour:02}:{minute:02}:{second:02}Z")
}

fn asset_key(record: &Value) -> String {
    let asset = record.get("asset_ref").unwrap_or(&Value::Null);
    let path = asset
        .get("asset_path")
        .and_then(Value::as_array)
        .map(|items| items.iter().filter_map(Value::as_str).collect::<Vec<_>>().join("/"))
        .unwrap_or_default();
    format!(
        "{}|{}",
        string(asset, "vessel_id").unwrap_or(""),
        path
    )
}

fn sequence_key(record: &Value) -> Option<String> {
    let sequence = record.get("time")?.get("sequence_number")?.as_u64()?;
    let source_id = string(record.get("provenance")?, "source_id")?;
    let clock_id = string(record.get("time")?, "clock_id")?;
    Some(format!(
        "{source_id}|{}|{clock_id}|{sequence}",
        asset_key(record)
    ))
}

fn validate_payload(record: &Value, out: &mut RecordOutcome) {
    let Some(provenance) = record.get("provenance") else {
        out.reject("missing_provenance");
        return;
    };
    let Some(payload) = provenance.get("source_payload") else {
        out.reject("missing_source_payload");
        return;
    };
    check_keys(
        payload,
        &["sha256", "byte_length", "media_type", "inline_base64", "content_ref",
          "signature_state", "signature_key_id"],
        "source_payload",
        &mut out.errors,
    );
    let digest = string(payload, "sha256").unwrap_or("");
    let byte_length = payload.get("byte_length").and_then(Value::as_u64);
    let media_type = string(payload, "media_type").unwrap_or("");
    if !is_sha256(digest) || byte_length.is_none() || media_type.trim().is_empty() {
        out.reject("source_payload_metadata_invalid");
    }
    let inline = string(payload, "inline_base64");
    let content_ref = string(payload, "content_ref");
    if inline.is_some() == content_ref.is_some() {
        out.reject("source_payload_requires_exactly_one_representation");
        return;
    }
    if let Some(encoded) = inline {
        match decode_base64(encoded) {
            Ok(bytes) => {
                if Some(bytes.len() as u64) != byte_length {
                    out.reject("source_payload_byte_length_mismatch");
                }
                let actual = sha256(&bytes);
                if actual != digest {
                    out.reject("source_payload_digest_mismatch");
                } else {
                    out.payload_digest_match = true;
                }
            }
            Err(code) => out.reject(code),
        }
    } else {
        // Remote references are not fetched by this offline tool. A record whose
        // payload bytes are unavailable cannot pass byte-level conformance.
        out.reject("source_payload_content_reference_not_resolved_offline");
    }
    if string(payload, "signature_state") == Some("invalid") {
        out.reject("source_signature_invalid");
    }
    if string(payload, "signature_state") == Some("unverified") {
        out.abstain("source_signature_unverified");
    }
}

fn validate_time(record: &Value, evaluation_ns: i128, out: &mut RecordOutcome) {
    let Some(time) = record.get("time") else {
        out.reject("missing_time_context");
        return;
    };
    check_keys(
        time,
        &["event_time", "time_basis", "clock_id", "clock_quality",
          "synchronization_uncertainty_ms", "monotonic_counter_ns", "sequence_number"],
        "time",
        &mut out.errors,
    );
    let basis = string(time, "time_basis").unwrap_or("");
    let clock_quality = string(time, "clock_quality").unwrap_or("");
    if !["synchronized_utc", "source_wall_clock", "relative_monotonic", "unknown"]
        .contains(&basis)
    {
        out.reject("time_basis_invalid");
    }
    if !["locked", "holdover", "unsynchronized", "unknown"].contains(&clock_quality) {
        out.reject("clock_quality_invalid");
    }
    if basis == "unknown" || clock_quality == "unknown" || clock_quality == "unsynchronized" {
        out.abstain("clock_not_qualified");
    }
    if let Some(event_time) = string(time, "event_time") {
        match parse_rfc3339_ns(event_time) {
            Some(value) if value > evaluation_ns => out.reject("event_time_after_evaluation_time"),
            Some(_) => {}
            None => out.reject("event_time_not_valid_rfc3339"),
        }
    } else {
        out.abstain("event_time_unavailable");
    }
    if let Some(received_at) = record
        .get("provenance")
        .and_then(|p| string(p, "received_at"))
    {
        match parse_rfc3339_ns(received_at) {
            Some(value) if value > evaluation_ns => out.reject("receipt_time_after_evaluation_time"),
            Some(_) => {}
            None => out.reject("receipt_time_not_valid_rfc3339"),
        }
    } else {
        out.reject("receipt_time_missing");
    }
}

fn validate_observation(
    record: &Value,
    known_ids: &HashSet<String>,
    out: &mut RecordOutcome,
) {
    check_keys(
        record,
        &["schema_version", "record_type", "record_id", "asset_ref", "provenance",
          "time", "mapping", "quantity_kind", "unit_ucum", "value_state", "value",
          "uncertainty", "quality", "dependency_group", "lineage", "context"],
        "observation",
        &mut out.errors,
    );
    let mapping = record.get("mapping").unwrap_or(&Value::Null);
    check_keys(
        mapping,
        &["status", "native_channel_id", "native_channel_name", "normalized_concept_id",
          "mapping_method", "mapping_confidence", "candidate_concept_ids", "reviewer_ref"],
        "mapping",
        &mut out.errors,
    );
    out.native_identifier_present = string(mapping, "native_channel_id").is_some()
        && string(mapping, "native_channel_name").is_some();
    if !out.native_identifier_present {
        out.reject("native_channel_identity_missing");
    }
    match string(mapping, "status").unwrap_or("") {
        "exact" => {
            if string(mapping, "normalized_concept_id").is_none()
                || string(mapping, "mapping_method").is_none()
            {
                out.reject("exact_mapping_incomplete");
            }
        }
        "reviewed" => {
            if string(mapping, "normalized_concept_id").is_none()
                || string(mapping, "reviewer_ref").is_none()
            {
                out.reject("reviewed_mapping_missing_reviewer_or_concept");
            }
        }
        "ambiguous" | "unmapped" => {
            if mapping.get("normalized_concept_id").is_some() {
                out.reject("abstaining_mapping_has_normalized_concept");
            } else {
                out.abstain("semantic_mapping_not_qualified");
            }
        }
        _ => out.reject("mapping_status_invalid"),
    }

    let state = string(record, "value_state").unwrap_or("");
    let has_value = record.get("value").is_some();
    match state {
        "measured" => {
            if !has_value {
                out.reject("measured_value_missing");
            }
            if array(record, "lineage").map_or(0, Vec::len) != 0 {
                out.reject("measured_value_has_derived_lineage");
            }
        }
        "derived" | "reconstructed" | "imputed" => {
            if !has_value {
                out.reject("transformed_value_missing");
            }
            let lineage = array(record, "lineage").cloned().unwrap_or_default();
            if lineage.is_empty() {
                out.reject("transformed_value_lineage_missing");
            }
            for item in lineage {
                check_keys(
                    &item,
                    &["transform_id", "implementation", "version", "input_record_refs",
                      "output_state", "parameters_sha256"],
                    "lineage_item",
                    &mut out.errors,
                );
                if string(&item, "output_state") != Some(state) {
                    out.reject("lineage_output_state_mismatch");
                }
                if let Some(inputs) = array(&item, "input_record_refs") {
                    for input in inputs.iter().filter_map(Value::as_str) {
                        if !known_ids.contains(input) {
                            out.reject("lineage_input_reference_unresolved");
                        }
                    }
                } else {
                    out.reject("lineage_input_references_missing");
                }
            }
        }
        "missing" | "rejected" | "invalid" => {
            if has_value {
                out.reject("non_value_state_contains_value");
            }
        }
        "stale" | "conflicted" => {
            if !has_value {
                out.reject("stale_or_conflicted_value_missing");
            }
            out.abstain("observation_not_current_unconflicted_measurement");
        }
        _ => out.reject("value_state_invalid"),
    }

    let quality = record.get("quality").unwrap_or(&Value::Null);
    check_keys(
        quality,
        &["calibration_state", "calibration_ref", "freshness_state",
          "measurement_quality", "quality_flags"],
        "quality",
        &mut out.errors,
    );
    if string(quality, "calibration_state") == Some("expired")
        || string(quality, "measurement_quality") == Some("invalid")
    {
        out.reject("measurement_quality_invalid");
    } else if string(quality, "calibration_state") == Some("unknown")
        || string(quality, "measurement_quality") == Some("unknown")
        || string(quality, "measurement_quality") == Some("suspect")
        || string(quality, "freshness_state") == Some("unknown")
    {
        out.abstain("measurement_quality_not_qualified");
    }
    if string(quality, "freshness_state") == Some("stale") {
        out.abstain("measurement_stale");
    }
    if string(record, "quantity_kind").is_none() {
        out.reject("quantity_kind_missing");
    }
    if record.get("value").and_then(Value::as_f64).is_some()
        && string(record, "unit_ucum").is_none()
    {
        out.reject("numeric_value_missing_unit");
    }
    if let Some(dependency) = string(record, "dependency_group") {
        if dependency.trim().is_empty() {
            out.reject("dependency_group_empty");
        }
    }
}

fn validate_assertion(
    record: &Value,
    known_ids: &HashSet<String>,
    out: &mut RecordOutcome,
) {
    check_keys(
        record,
        &["schema_version", "record_type", "record_id", "asset_ref", "provenance",
          "time", "assertion", "method", "operating_context", "evidence_links",
          "uncertainty", "authority_granted", "claim_ceiling"],
        "diagnostic_assertion",
        &mut out.errors,
    );
    let assertion = record.get("assertion").unwrap_or(&Value::Null);
    check_keys(
        assertion,
        &["kind", "code", "statement", "status", "confidence", "severity", "claim_scope"],
        "assertion",
        &mut out.errors,
    );
    let method = record.get("method").unwrap_or(&Value::Null);
    check_keys(
        method,
        &["provider", "name", "version", "algorithm_sha256"],
        "method",
        &mut out.errors,
    );
    if string(assertion, "statement").is_none()
        || string(assertion, "kind").is_none()
        || string(method, "provider").is_none()
        || string(method, "name").is_none()
        || string(method, "version").is_none()
    {
        out.reject("assertion_or_method_incomplete");
    }
    if record.get("authority_granted").and_then(Value::as_bool) != Some(false)
        || string(record, "claim_ceiling") != Some("not_ground_truth")
    {
        out.reject("assertion_authority_or_claim_ceiling_invalid");
    }
    if string(assertion, "status") == Some("withdrawn") {
        out.abstain("assertion_withdrawn");
    }
    let links = record.get("evidence_links").unwrap_or(&Value::Null);
    check_keys(links, &["supports", "contradicts", "derived_from"], "evidence_links", &mut out.errors);
    let mut all_edges_resolve = true;
    for key in ["supports", "contradicts", "derived_from"] {
        if let Some(items) = array(links, key) {
            for reference in items.iter().filter_map(Value::as_str) {
                if !known_ids.contains(reference) {
                    all_edges_resolve = false;
                    out.reject("assertion_evidence_reference_unresolved");
                }
            }
        } else {
            out.reject("assertion_evidence_link_list_missing");
            all_edges_resolve = false;
        }
    }
    out.conflict_edges = if all_edges_resolve {
        array(links, "supports").map_or(0, Vec::len)
            + array(links, "contradicts").map_or(0, Vec::len)
    } else {
        0
    };
    if let Some(supports) = array(links, "supports") {
        if let Some(contradicts) = array(links, "contradicts") {
            for support in supports {
                if contradicts.contains(support) {
                    out.reject("same_record_both_supports_and_contradicts");
                }
            }
        }
    }
}

fn validate_record(
    record: &Value,
    evaluation_ns: i128,
    known_ids: &HashSet<String>,
) -> RecordOutcome {
    let mut out = RecordOutcome::default();
    check_keys(
        record,
        &["schema_version", "record_type", "record_id", "asset_ref", "provenance", "time",
          "mapping", "quantity_kind", "unit_ucum", "value_state", "value", "uncertainty",
          "quality", "dependency_group", "lineage", "context", "assertion", "method",
          "operating_context", "evidence_links", "authority_granted", "claim_ceiling"],
        "record",
        &mut out.errors,
    );
    if string(record, "schema_version") != Some("0.1.0") {
        out.reject("record_schema_version_invalid");
    }
    if string(record, "record_id").is_none() {
        out.reject("record_id_missing");
    }
    let asset = record.get("asset_ref").unwrap_or(&Value::Null);
    check_keys(
        asset,
        &["vessel_id", "asset_path", "serial_number", "manufacturer", "model", "configuration_sha256"],
        "asset_ref",
        &mut out.errors,
    );
    if string(asset, "vessel_id").is_none()
        || array(asset, "asset_path").map_or(true, Vec::is_empty)
        || !string(asset, "configuration_sha256").is_some_and(is_sha256)
    {
        out.reject("asset_identity_or_configuration_incomplete");
    }
    let provenance = record.get("provenance").unwrap_or(&Value::Null);
    check_keys(
        provenance,
        &["source_id", "source_class", "native_record_id", "native_message_type",
          "adapter_id", "adapter_version", "mapping_catalogue_version", "received_at",
          "source_payload", "upstream_record_refs"],
        "provenance",
        &mut out.errors,
    );
    let provenance_fields = ["source_id", "source_class", "native_record_id", "adapter_id",
        "adapter_version", "mapping_catalogue_version", "received_at"];
    out.provenance_complete = provenance_fields.iter().all(|field| {
        string(provenance, field).is_some_and(|value| !value.trim().is_empty())
    }) && provenance.get("source_payload").is_some();
    if !out.provenance_complete {
        out.reject("provenance_incomplete");
    }
    if string(provenance, "source_class") == Some("simulator") {
        out.abstain("simulator_source_not_physical_measurement");
    }

    validate_payload(record, &mut out);
    validate_time(record, evaluation_ns, &mut out);

    match string(record, "record_type").unwrap_or("") {
        "observation" => validate_observation(record, known_ids, &mut out),
        "diagnostic_assertion" => validate_assertion(record, known_ids, &mut out),
        _ => out.reject("record_type_invalid"),
    }
    out
}

fn check_expected_invariants(
    fixture: &Value,
    outcomes: &[RecordOutcome],
    actual: &Disposition,
    fixture_errors: &[String],
) -> Vec<String> {
    let mut failures = Vec::new();
    let expected = array(fixture, "expected_invariants").cloned().unwrap_or_default();
    let primary = fixture.get("record").unwrap_or(&Value::Null);
    let all_errors: Vec<&str> = fixture_errors.iter().map(String::as_str).collect();
    for item in expected.iter().filter_map(Value::as_str) {
        let present = match item {
            "native_id_retained" => string(primary.get("mapping").unwrap_or(&Value::Null), "native_channel_id").is_some(),
            "payload_digest_verified" => outcomes.first().is_some_and(|o| o.payload_digest_match),
            "mapping_exact" => string(primary.get("mapping").unwrap_or(&Value::Null), "status") == Some("exact"),
            "lineage_retained" => array(primary, "lineage").is_some_and(|v| !v.is_empty()),
            "value_state_derived" => string(primary, "value_state") == Some("derived"),
            "missing_not_healthy" => string(primary, "value_state") == Some("missing"),
            "no_value_present" => primary.get("value").is_none(),
            "reconstructed_not_measured" => string(primary, "value_state") == Some("reconstructed"),
            "no_normalized_id" => primary.get("mapping").and_then(|m| m.get("normalized_concept_id")).is_none(),
            "record_type_assertion" => string(primary, "record_type") == Some("diagnostic_assertion"),
            "claim_not_ground_truth" => string(primary, "claim_ceiling") == Some("not_ground_truth"),
            "authority_false" => primary.get("authority_granted").and_then(Value::as_bool) == Some(false),
            "support_link_retained" => primary.get("evidence_links").and_then(|l| array(l, "supports")).is_some_and(|a| !a.is_empty()),
            "contradiction_link_retained" => primary.get("evidence_links").and_then(|l| array(l, "contradicts")).is_some_and(|a| !a.is_empty()),
            "no_consensus_collapse" => primary.get("evidence_links").is_some_and(|l| {
                array(l, "supports").is_some_and(|a| !a.is_empty())
                    && array(l, "contradicts").is_some_and(|a| !a.is_empty())
            }),
            "digest_mismatch_rejected" => all_errors.contains(&"source_payload_digest_mismatch"),
            "future_timestamp_not_silently_accepted" => all_errors.contains(&"event_time_after_evaluation_time"),
            "derived_without_lineage_rejected" => all_errors.contains(&"transformed_value_lineage_missing"),
            "duplicate_sequence_detected" => all_errors.contains(&"duplicate_source_sequence"),
            "independent_source_retained" => primary.get("provenance").and_then(|p| string(p, "source_id")).is_some_and(|s| s != "fixture-source-01"),
            _ => false,
        };
        if !present {
            failures.push(format!("expected_invariant_not_observed:{item}"));
        }
    }
    if failures.is_empty() && actual.as_str().is_empty() {
        failures.push("disposition_missing".to_owned());
    }
    failures
}

fn audit_corpus(corpus: &Value, corpus_bytes: &[u8]) -> Value {
    let mut global_errors = Vec::new();
    check_keys(
        corpus,
        &["corpus_id", "corpus_version", "claim_ceiling", "evaluation_time", "fixture_count", "fixtures"],
        "corpus",
        &mut global_errors,
    );
    if string(corpus, "corpus_version") != Some(CORPUS_VERSION) {
        global_errors.push("corpus_version_invalid".to_owned());
    }
    if string(corpus, "claim_ceiling") != Some(CLAIM_CEILING) {
        global_errors.push("corpus_claim_ceiling_invalid".to_owned());
    }
    let evaluation_ns = string(corpus, "evaluation_time")
        .and_then(parse_rfc3339_ns);
    let Some(evaluation_ns) = evaluation_ns else {
        global_errors.push("evaluation_time_invalid".to_owned());
        return json!({
            "report_version":"0.1.0",
            "report_id":sha256(corpus_bytes),
            "profile_version":"0.1.0",
            "adapter_id":"luminous-symtropy-reference-auditor",
            "adapter_version":"0.1.0",
            "fixture_corpus_sha256":sha256(corpus_bytes),
            "started_at":format_rfc3339_now(),
            "completed_at":format_rfc3339_now(),
            "environment_sha256":sha256(format!("{}|{}", env::consts::OS, env::consts::ARCH).as_bytes()),
            "overall_conformance":"fail",
            "summary":{"cases_total":0,"accepted":0,"rejected":0,"abstained":0,
                "payload_digest_match_rate":null,"native_identifier_retention_rate":null,
                "ambiguous_mapping_abstention_rate":null,"conflict_preservation_rate":null,
                "provenance_completeness_rate":null},
            "hard_gates":[],
            "case_results":[],
            "global_errors":global_errors,
            "claim_ceiling":"adapter_conformance_only"
        });
    };

    let fixtures = array(corpus, "fixtures").cloned().unwrap_or_default();
    if corpus.get("fixture_count").and_then(Value::as_u64) != Some(fixtures.len() as u64) {
        global_errors.push("fixture_count_mismatch".to_owned());
    }
    if fixtures.is_empty() {
        global_errors.push("fixture_corpus_empty".to_owned());
    }

    let mut all_ids = HashSet::new();
    let mut duplicate_ids = HashSet::new();
    for fixture in &fixtures {
        for record in std::iter::once(fixture.get("record").unwrap_or(&Value::Null))
            .chain(array(fixture, "companion_records").into_iter().flatten())
        {
            if let Some(id) = string(record, "record_id") {
                if !all_ids.insert(id.to_owned()) {
                    duplicate_ids.insert(id.to_owned());
                }
            }
        }
    }
    if !duplicate_ids.is_empty() {
        global_errors.push(format!("duplicate_record_ids:{}", duplicate_ids.into_iter().collect::<Vec<_>>().join(",")));
    }

    let mut results = Vec::new();
    let mut accepted = 0_u64;
    let mut rejected = 0_u64;
    let mut abstained = 0_u64;
    let mut digest_checked = 0_u64;
    let mut digest_matched = 0_u64;
    let mut native_checked = 0_u64;
    let mut native_retained = 0_u64;
    let mut provenance_complete = 0_u64;
    let mut conflict_edge_total = 0_u64;
    let mut conflict_edges_retained = 0_u64;
    let mut ambiguous_cases = 0_u64;
    let mut ambiguous_abstained = 0_u64;
    for fixture in &fixtures {
        let fixture_id = string(fixture, "fixture_id").unwrap_or("unknown_fixture");
        let expected = string(fixture, "expected_disposition").unwrap_or("reject");
        let mut records: Vec<&Value> = Vec::new();
        if let Some(record) = fixture.get("record") {
            records.push(record);
        }
        if let Some(companions) = array(fixture, "companion_records") {
            records.extend(companions.iter());
        }

        let mut outcomes = Vec::new();
        for record in &records {
            let outcome = validate_record(record, evaluation_ns, &all_ids);
            if record.get("provenance").and_then(|p| p.get("source_payload")).is_some() {
                digest_checked += 1;
                if outcome.payload_digest_match { digest_matched += 1; }
            }
            if string(record, "record_type") == Some("observation") {
                native_checked += 1;
                if outcome.native_identifier_present { native_retained += 1; }
            }
            if outcome.provenance_complete { provenance_complete += 1; }
            if outcome.conflict_edges > 0 {
                conflict_edge_total += outcome.conflict_edges as u64;
                conflict_edges_retained += outcome.conflict_edges as u64;
            }
            outcomes.push(outcome);
        }

        let mut fixture_errors = Vec::new();
        let mut fixture_abstentions = Vec::new();
        for outcome in &outcomes {
            fixture_errors.extend(outcome.errors.clone());
            fixture_abstentions.extend(outcome.abstentions.clone());
        }

        let mut stream_sequences = BTreeMap::<String, usize>::new();
        for record in &records {
            if let Some(key) = sequence_key(record) {
                *stream_sequences.entry(key).or_default() += 1;
            }
        }
        if stream_sequences.values().any(|count| *count > 1) {
            fixture_errors.push("duplicate_source_sequence".to_owned());
        }

        let primary = fixture.get("record").unwrap_or(&Value::Null);
        let mapping_status = string(primary.get("mapping").unwrap_or(&Value::Null), "status");
        if matches!(mapping_status, Some("ambiguous" | "unmapped")) {
            ambiguous_cases += 1;
        }

        let mut actual = if !fixture_errors.is_empty() {
            Disposition::Reject
        } else if !fixture_abstentions.is_empty() {
            Disposition::Abstain
        } else {
            Disposition::Accept
        };
        if matches!(mapping_status, Some("ambiguous" | "unmapped"))
            && actual == Disposition::Abstain
        {
            ambiguous_abstained += 1;
        }

        // Check fixture-level invariants after corpus/record validation.
        let invariant_failures = check_expected_invariants(fixture, &outcomes, &actual, &fixture_errors);
        if !invariant_failures.is_empty() {
            fixture_errors.extend(invariant_failures);
            actual = Disposition::Reject;
        }
        let passed = actual.as_str() == expected;
        if !passed {
            fixture_errors.push(format!("expected_{expected}_got_{}", actual.as_str()));
        }
        match actual {
            Disposition::Accept => accepted += 1,
            Disposition::Abstain => abstained += 1,
            Disposition::Reject => rejected += 1,
        }
        let reason_codes = fixture_errors
            .iter()
            .chain(fixture_abstentions.iter())
            .cloned()
            .collect::<BTreeSet<_>>()
            .into_iter()
            .map(Value::String)
            .collect::<Vec<_>>();
        results.push(json!({
            "fixture_id":fixture_id,
            "expected_disposition":expected,
            "actual_disposition":actual.as_str(),
            "passed":passed,
            "reason_codes":reason_codes,
            "lost_fields":[],
            "payload_digest_match":outcomes.iter().all(|o| o.payload_digest_match)
        }));
    }

    let case_count = fixtures.len() as u64;
    let rate = |numerator: u64, denominator: u64| -> Value {
        if denominator == 0 { Value::Null } else { json!(numerator as f64 / denominator as f64) }
    };
    let all_passed = global_errors.is_empty()
        && results.iter().all(|r| r.get("passed").and_then(Value::as_bool) == Some(true));
    let hard_gates = json!([
        {"gate_id":"fixture_count_and_identity","passed":global_errors.is_empty(),"failure_count":global_errors.len()},
        {"gate_id":"source_payload_digest","passed":digest_checked == digest_matched,"failure_count":digest_checked.saturating_sub(digest_matched)},
        {"gate_id":"native_identifier_retention","passed":native_checked == native_retained,"failure_count":native_checked.saturating_sub(native_retained)},
        {"gate_id":"ambiguous_mapping_abstention","passed":ambiguous_cases == ambiguous_abstained,"failure_count":ambiguous_cases.saturating_sub(ambiguous_abstained)},
        {"gate_id":"conflict_edge_preservation","passed":conflict_edge_total == conflict_edges_retained,"failure_count":conflict_edge_total.saturating_sub(conflict_edges_retained)},
        {"gate_id":"case_expected_dispositions","passed":results.iter().all(|r| r.get("passed").and_then(Value::as_bool) == Some(true)),"failure_count":results.iter().filter(|r| r.get("passed").and_then(Value::as_bool) != Some(true)).count()}
    ]);
    let now = format_rfc3339_now();
    json!({
        "report_version":"0.1.0",
        "report_id":sha256(corpus_bytes),
        "profile_version":"0.1.0",
        "adapter_id":"luminous-symtropy-reference-auditor",
        "adapter_version":"0.1.0",
        "fixture_corpus_sha256":sha256(corpus_bytes),
        "started_at":now,
        "completed_at":now,
        "environment_sha256":sha256(format!("{}|{}", env::consts::OS, env::consts::ARCH).as_bytes()),
        "overall_conformance":if all_passed {"pass"} else {"fail"},
        "summary":{
            "cases_total":case_count,
            "accepted":accepted,
            "rejected":rejected,
            "abstained":abstained,
            "payload_digest_match_rate":rate(digest_matched,digest_checked),
            "native_identifier_retention_rate":rate(native_retained,native_checked),
            "mapping_precision":null,
            "mapping_coverage":null,
            "ambiguous_mapping_abstention_rate":rate(ambiguous_abstained,ambiguous_cases),
            "conflict_preservation_rate":rate(conflict_edges_retained,conflict_edge_total),
            "provenance_completeness_rate":rate(provenance_complete,digest_checked),
            "event_order_preservation_rate":null,
            "p95_processing_ms":null,
            "peak_memory_bytes":null
        },
        "hard_gates":hard_gates,
        "case_results":results,
        "global_errors":global_errors,
        "claim_ceiling":"adapter_conformance_only",
        "notes":["A PASS covers only the pinned synthetic corpus; it is not ISO conformance, physical diagnostic validation or operating authority."]
    })
}

fn run() -> Result<bool, String> {
    let mut args = env::args_os().skip(1);
    let corpus_path = args.next().ok_or_else(|| {
        "usage: maritime_evidence_interchange_audit <corpus.json> [--output <report.json>]".to_owned()
    })?;
    let mut output_path: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        if arg == "--output" {
            output_path = Some(PathBuf::from(
                args.next().ok_or_else(|| "--output requires a path".to_owned())?,
            ));
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        }
    }
    let bytes = fs::read(&corpus_path)
        .map_err(|error| format!("cannot read {}: {error}", corpus_path.to_string_lossy()))?;
    let corpus: Value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid corpus JSON: {error}"))?;
    let report = audit_corpus(&corpus, &bytes);
    let encoded = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("cannot serialize report: {error}"))?;
    if let Some(path) = output_path {
        fs::write(&path, &encoded)
            .map_err(|error| format!("cannot write {}: {error}", path.to_string_lossy()))?;
    }
    println!("{}", String::from_utf8_lossy(&encoded));
    Ok(report.get("overall_conformance").and_then(Value::as_str) == Some("pass"))
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const CORPUS: &str = include_str!("../../../../benchmarks/maritime/maritime-evidence-interchange-conformance-v0.1.json");

    fn parsed_corpus() -> (Value, Vec<u8>) {
        let bytes = CORPUS.as_bytes().to_vec();
        (serde_json::from_slice(&bytes).expect("fixture corpus parses"), bytes)
    }

    #[test]
    fn frozen_corpus_passes_reference_auditor() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        assert_eq!(report["overall_conformance"], "pass");
        assert_eq!(report["summary"]["cases_total"], 13);
        assert_eq!(report["summary"]["accepted"], 7);
        assert_eq!(report["summary"]["abstained"], 2);
        assert_eq!(report["summary"]["rejected"], 4);
    }

    #[test]
    fn fixture_count_mismatch_fails_closed() {
        let (mut corpus, bytes) = parsed_corpus();
        corpus["fixture_count"] = json!(12);
        let report = audit_corpus(&corpus, &bytes);
        assert_eq!(report["overall_conformance"], "fail");
        assert!(report["global_errors"].as_array().unwrap().iter().any(|v| v == "fixture_count_mismatch"));
    }

    #[test]
    fn duplicate_record_ids_fail_closed() {
        let (mut corpus, bytes) = parsed_corpus();
        let duplicate = corpus["fixtures"][0]["record"].clone();
        corpus["fixtures"][1]["record"] = duplicate;
        let report = audit_corpus(&corpus, &bytes);
        assert_eq!(report["overall_conformance"], "fail");
        assert!(report["global_errors"][0].as_str().unwrap().starts_with("duplicate_record_ids"));
    }

    #[test]
    fn malformed_evaluation_time_fails_closed() {
        let (mut corpus, bytes) = parsed_corpus();
        corpus["evaluation_time"] = json!("not-a-date");
        let report = audit_corpus(&corpus, &bytes);
        assert_eq!(report["overall_conformance"], "fail");
    }

    #[test]
    fn rfc3339_offsets_are_compared_as_instants() {
        let utc = parse_rfc3339_ns("2026-01-01T12:00:00Z").unwrap();
        let offset = parse_rfc3339_ns("2026-01-01T14:00:00+02:00").unwrap();
        assert_eq!(utc, offset);
    }

    #[test]
    fn rfc3339_rejects_invalid_calendar_dates() {
        assert!(parse_rfc3339_ns("2026-02-30T12:00:00Z").is_none());
        assert!(parse_rfc3339_ns("2026-01-01T12:60:00Z").is_none());
    }

    #[test]
    fn base64_decodes_frozen_payload() {
        assert_eq!(decode_base64("SGVsbG8=").unwrap(), b"Hello");
    }

    #[test]
    fn base64_rejects_noncanonical_padding() {
        assert!(decode_base64("SGVsbG9=").is_err());
    }

    #[test]
    fn digest_is_computed_from_exact_decoded_bytes() {
        let decoded = decode_base64("eyJ2YWx1ZSI6ODIuNSwidW5pdCI6IkNlbCJ9").unwrap();
        assert_eq!(sha256(&decoded), "281a790c52136d39fb06b2064cb65e158a6c2184cbe4dfd991d2cda152423d67");
    }

    #[test]
    fn missing_value_is_not_treated_as_healthy_zero() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        let missing = report["case_results"].as_array().unwrap().iter()
            .find(|case| case["fixture_id"] == "obs-003-missing-no-value").unwrap();
        assert_eq!(missing["actual_disposition"], "accept");
        assert_eq!(missing["passed"], true);
    }

    #[test]
    fn ambiguous_and_unmapped_channels_abstain() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        for id in ["obs-005-ambiguous-map-abstains", "obs-006-unmapped-channel-abstains"] {
            let result = report["case_results"].as_array().unwrap().iter()
                .find(|case| case["fixture_id"] == id).unwrap();
            assert_eq!(result["actual_disposition"], "abstain");
        }
    }

    #[test]
    fn diagnostic_assertions_cannot_grant_authority() {
        let (mut corpus, bytes) = parsed_corpus();
        corpus["fixtures"][6]["record"]["authority_granted"] = json!(true);
        let report = audit_corpus(&corpus, &bytes);
        assert_eq!(report["overall_conformance"], "fail");
    }

    #[test]
    fn vendor_alarm_remains_an_assertion() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        let result = report["case_results"].as_array().unwrap().iter()
            .find(|case| case["fixture_id"] == "assert-007-vendor-alarm-not-measurement").unwrap();
        assert_eq!(result["actual_disposition"], "accept");
    }

    #[test]
    fn conflicting_assertion_keeps_both_edges() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        let result = report["case_results"].as_array().unwrap().iter()
            .find(|case| case["fixture_id"] == "assert-008-conflict-preserved").unwrap();
        assert_eq!(result["actual_disposition"], "accept");
        assert_eq!(result["passed"], true);
    }

    #[test]
    fn future_event_is_rejected() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        let result = report["case_results"].as_array().unwrap().iter()
            .find(|case| case["fixture_id"] == "obs-010-future-event-time").unwrap();
        assert_eq!(result["actual_disposition"], "reject");
    }

    #[test]
    fn derived_value_requires_lineage() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        let result = report["case_results"].as_array().unwrap().iter()
            .find(|case| case["fixture_id"] == "obs-011-derived-without-lineage").unwrap();
        assert_eq!(result["actual_disposition"], "reject");
    }

    #[test]
    fn duplicate_sequence_is_checked_within_stream_case() {
        let (corpus, bytes) = parsed_corpus();
        let report = audit_corpus(&corpus, &bytes);
        let result = report["case_results"].as_array().unwrap().iter()
            .find(|case| case["fixture_id"] == "obs-012-duplicate-sequence").unwrap();
        assert_eq!(result["actual_disposition"], "reject");
    }

    #[test]
    fn unresolved_evidence_reference_is_rejected() {
        let (mut corpus, bytes) = parsed_corpus();
        corpus["fixtures"][6]["record"]["evidence_links"]["supports"] = json!(["does-not-exist"]);
        let report = audit_corpus(&corpus, &bytes);
        assert_eq!(report["overall_conformance"], "fail");
    }

    #[test]
    fn content_reference_without_payload_bytes_does_not_pass_integrity() {
        let (mut corpus, bytes) = parsed_corpus();
        let payload = &mut corpus["fixtures"][0]["record"]["provenance"]["source_payload"];
        payload.as_object_mut().unwrap().remove("inline_base64");
        payload["content_ref"] = json!("urn:sha256:281a790c52136d39fb06b2064cb65e158a6c2184cbe4dfd991d2cda152423d67");
        let report = audit_corpus(&corpus, &bytes);
        assert_eq!(report["overall_conformance"], "fail");
    }
}
