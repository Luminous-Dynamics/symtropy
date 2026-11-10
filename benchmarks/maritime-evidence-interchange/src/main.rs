#![forbid(unsafe_code)]

//! Offline semantic conformance evaluator for the Maritime Evidence Interchange
//! synthetic corpus. This checks the profile's high-value invariants; it is not
//! a general JSON Schema 2020-12 implementation and does not test an adapter's
//! round-trip preservation. Those are separate conformance stages.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fmt::Write as FmtWrite;
use std::fs;
use std::process::ExitCode;
use std::time::{Instant, SystemTime, UNIX_EPOCH};

const MAX_CORPUS_BYTES: usize = 64 * 1024 * 1024;
const MAX_INLINE_PAYLOAD_BYTES: usize = 16 * 1024 * 1024;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Disposition {
    #[default]
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
    evidence_links_resolved: Option<bool>,
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
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing hexadecimal digest to String cannot fail");
    }
    output
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
    if !bytes.len().is_multiple_of(4) {
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
    let adjusted_year = year - if month <= 2 { 1 } else { 0 };
    let era = (if adjusted_year >= 0 {
        adjusted_year
    } else {
        adjusted_year - 399
    }) / 400;
    let year_of_era = adjusted_year - era * 400;
    let adjusted_month = month + if month > 2 { -3 } else { 9 };
    let day_of_year = (153 * adjusted_month + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146097 + day_of_era - 719468
}

/// Parse RFC3339 instants with explicit offsets and at most nanosecond precision.
fn parse_rfc3339_ns(value: &str) -> Result<i128, String> {
    let (date, rest) = value.split_once('T')
        .ok_or_else(|| "timestamp_missing_separator".to_owned())?;
    if date.len() != 10 || date.as_bytes().get(4) != Some(&b'-')
        || date.as_bytes().get(7) != Some(&b'-')
    {
        return Err("timestamp_date_not_rfc3339".into());
    }
    let bytes = date.as_bytes();
    let year = parse_fixed_u32(bytes, 0, 4).ok_or("timestamp_invalid_year")?;
    let month = parse_fixed_u32(bytes, 5, 7).ok_or("timestamp_invalid_month")?;
    let day = parse_fixed_u32(bytes, 8, 10).ok_or("timestamp_invalid_day")?;
    if year < 1 || !(1..=12).contains(&month) || day < 1 {
        return Err("timestamp_date_component_out_of_range".into());
    }
    let leap = year % 4 == 0 && (year % 100 != 0 || year % 400 == 0);
    let days_in_month = match month {
        1 | 3 | 5 | 7 | 8 | 10 | 12 => 31,
        4 | 6 | 9 | 11 => 30,
        2 if leap => 29,
        2 => 28,
        _ => return Err("timestamp_month_out_of_range".into()),
    };
    if day > days_in_month {
        return Err("timestamp_day_out_of_range".into());
    }

    let timezone_index = rest.char_indices()
        .find(|(index, character)| *index > 0 && (*character == '+' || *character == '-'))
        .map(|(index, _)| index);
    let (time_part, zone) = if let Some(index) = timezone_index {
        (&rest[..index], &rest[index..])
    } else if let Some(time) = rest.strip_suffix('Z') {
        (time, "Z")
    } else {
        return Err("timestamp_timezone_missing".into());
    };
    if time_part.len() < 8 || time_part.as_bytes().get(2) != Some(&b':')
        || time_part.as_bytes().get(5) != Some(&b':')
    {
        return Err("timestamp_time_not_rfc3339".into());
    }
    let time_bytes = time_part.as_bytes();
    let hour = parse_fixed_u32(time_bytes, 0, 2).ok_or("timestamp_invalid_hour")?;
    let minute = parse_fixed_u32(time_bytes, 3, 5).ok_or("timestamp_invalid_minute")?;
    let seconds_part = time_part.get(6..).ok_or("timestamp_invalid_second")?;
    let (second_text, fraction) = match seconds_part.split_once('.') {
        Some((whole, fractional)) => (whole, Some(fractional)),
        None => (seconds_part, None),
    };
    if second_text.len() != 2 {
        return Err("timestamp_second_must_have_two_digits".into());
    }
    let second: i64 = second_text.parse().map_err(|_| "timestamp_invalid_second")?;
    if hour > 23 || minute > 59 || !(0..=59).contains(&second) {
        return Err("timestamp_time_component_out_of_range".into());
    }
    let fraction_ns = if let Some(fraction) = fraction {
        if fraction.is_empty() || fraction.len() > 9 || !fraction.bytes().all(|b| b.is_ascii_digit()) {
            return Err("timestamp_fraction_precision_or_syntax_invalid".into());
        }
        let parsed: i128 = fraction.parse().map_err(|_| "timestamp_fraction_invalid")?;
        parsed * 10_i128.pow((9 - fraction.len()) as u32)
    } else {
        0
    };

    let offset_seconds: i64 = if zone == "Z" {
        0
    } else {
        if zone.len() != 6 || zone.as_bytes().get(3) != Some(&b':') {
            return Err("timestamp_offset_not_rfc3339".into());
        }
        if zone == "-00:00" {
            return Err("timestamp_offset_unknown_minus_zero".into());
        }
        let offset_hour = parse_fixed_u32(zone.as_bytes(), 1, 3)
            .ok_or("timestamp_offset_hour_invalid")?;
        let offset_minute = parse_fixed_u32(zone.as_bytes(), 4, 6)
            .ok_or("timestamp_offset_minute_invalid")?;
        if offset_hour > 23 || offset_minute > 59 {
            return Err("timestamp_offset_out_of_range".into());
        }
        let magnitude = offset_hour * 3_600 + offset_minute * 60;
        if zone.starts_with('+') { magnitude } else { -magnitude }
    };

    let days = days_from_civil(year, month, day);
    let whole_seconds = days * 86_400 + hour * 3_600 + minute * 60 + second - offset_seconds;
    Ok(i128::from(whole_seconds) * 1_000_000_000 + fraction_ns)
}

fn civil_from_days(days_since_epoch: i64) -> (i64, i64, i64) {
    let z = days_since_epoch + 719468;
    let era = (if z >= 0 { z } else { z - 146096 }) / 146097;
    let day_of_era = z - era * 146097;
    let year_of_era = (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_prime = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_prime + 2) / 5 + 1;
    let month = month_prime + if month_prime < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
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

fn has_only_fields(value: &Value, allowed: &[&str]) -> bool {
    value.as_object().is_some_and(|object| {
        object.keys().all(|key| allowed.iter().any(|field| *field == key.as_str()))
    })
}

fn validate_record(record: &Value, evaluation_time: i128) -> RecordAudit {
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

    let observation_fields = [
        "schema_version", "record_type", "record_id", "asset_ref", "provenance", "time",
        "mapping", "quantity_kind", "unit_ucum", "value_state", "value", "uncertainty",
        "quality", "dependency_group", "lineage", "context",
    ];
    let assertion_fields = [
        "schema_version", "record_type", "record_id", "asset_ref", "provenance", "time",
        "assertion", "method", "operating_context", "evidence_links", "uncertainty",
        "authority_granted", "claim_ceiling",
    ];
    let record_shape_valid = match record_type {
        Some("observation") => has_only_fields(record, &observation_fields),
        Some("diagnostic_assertion") => has_only_fields(record, &assertion_fields),
        _ => false,
    };
    if !record_shape_valid {
        audit.reject("record_unknown_or_type_incompatible_field");
    }
    if !has_only_fields(
        value_at(record, &["asset_ref"]).unwrap_or(&Value::Null),
        &["vessel_id", "asset_path", "serial_number", "manufacturer", "model", "configuration_sha256"],
    ) {
        audit.reject("asset_ref_unknown_field_or_shape");
    }
    if !has_only_fields(
        value_at(record, &["provenance"]).unwrap_or(&Value::Null),
        &["source_id", "source_class", "native_record_id", "native_message_type", "adapter_id",
          "adapter_version", "mapping_catalogue_version", "received_at", "source_payload",
          "upstream_record_refs"],
    ) {
        audit.reject("provenance_unknown_field_or_shape");
    }
    if !has_only_fields(
        value_at(record, &["time"]).unwrap_or(&Value::Null),
        &["event_time", "time_basis", "clock_id", "clock_quality", "synchronization_uncertainty_ms",
          "monotonic_counter_ns", "sequence_number"],
    ) {
        audit.reject("time_unknown_field_or_shape");
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
    match received_at.and_then(|value| parse_rfc3339_ns(value).ok()) {
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
            if !has_only_fields(
                payload,
                &["sha256", "byte_length", "media_type", "inline_base64", "content_ref",
                  "signature_state", "signature_key_id"],
            ) {
                audit.reject("source_payload_unknown_field_or_shape");
            }
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
                        match payload.get("signature_state").and_then(Value::as_str) {
                            Some("invalid") => audit.reject("payload_signature_invalid"),
                            Some("unverified") => audit.abstain("payload_signature_unverified"),
                            Some(value) if !["verified", "not_present"].contains(&value) => {
                                audit.reject("payload_signature_state_invalid");
                            }
                            _ => {}
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
    if time_basis == Some("unknown") || clock_quality == Some("unknown")
        || clock_quality == Some("unsynchronized")
    {
        audit.abstain("clock_not_qualified_for_temporal_decision");
    }
    if string_at(record, &["time", "clock_id"]).is_none_or(str::is_empty)
        || !matches!(time_basis, Some("synchronized_utc" | "source_wall_clock" | "relative_monotonic" | "unknown"))
        || !matches!(clock_quality, Some("locked" | "holdover" | "unsynchronized" | "unknown"))
        || finite_number_at(record, &["time", "synchronization_uncertainty_ms"]).is_none_or(|v| v < 0.0)
    {
        audit.reject("time_context_incomplete");
    }

    match value_at(record, &["time", "event_time"]) {
        Some(Value::String(value)) => match parse_rfc3339_ns(value) {
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
            if !has_only_fields(
                value_at(record, &["mapping"]).unwrap_or(&Value::Null),
                &["status", "native_channel_id", "native_channel_name", "normalized_concept_id",
                  "mapping_method", "mapping_confidence", "candidate_concept_ids", "reviewer_ref"],
            ) {
                audit.reject("mapping_unknown_field_or_shape");
            }
            if !has_only_fields(
                value_at(record, &["quality"]).unwrap_or(&Value::Null),
                &["calibration_state", "calibration_ref", "freshness_state", "measurement_quality", "quality_flags"],
            ) {
                audit.reject("quality_unknown_field_or_shape");
            }
            let mapping_status = string_at(record, &["mapping", "status"]);
            let native_id = string_at(record, &["mapping", "native_channel_id"]);
            let native_name = string_at(record, &["mapping", "native_channel_name"]);
            audit.native_identifier_present = native_id.is_some_and(|v| !v.is_empty())
                && native_name.is_some_and(|v| !v.is_empty());

            if !audit.native_identifier_present {
                audit.reject("native_channel_identity_missing");
            }
            match mapping_status {
                Some("exact") => {
                    if string_at(record, &["mapping", "normalized_concept_id"]).is_none_or(str::is_empty)
                        || string_at(record, &["mapping", "mapping_method"]).is_none_or(str::is_empty)
                    {
                        audit.reject("exact_mapping_incomplete");
                    }
                }
                Some("reviewed") => {
                    if string_at(record, &["mapping", "normalized_concept_id"]).is_none_or(str::is_empty)
                        || string_at(record, &["mapping", "reviewer_ref"]).is_none_or(str::is_empty)
                    {
                        audit.reject("reviewed_mapping_missing_reviewer_or_concept");
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
                            !has_only_fields(
                                item,
                                &["transform_id", "implementation", "version", "input_record_refs",
                                  "output_state", "parameters_sha256"],
                            )
                                || string_at(item, &["transform_id"]).is_none_or(str::is_empty)
                                || string_at(item, &["implementation"]).is_none_or(str::is_empty)
                                || string_at(item, &["version"]).is_none_or(str::is_empty)
                                || value_at(item, &["input_record_refs"]).and_then(Value::as_array)
                                    .is_none_or(|refs| refs.is_empty() || refs.iter().any(|r| r.as_str().is_none_or(str::is_empty)))
                                || string_at(item, &["output_state"]) != state
                        })
                    }) {
                        audit.reject("transformation_lineage_incomplete");
                    }
                }
                Some("measured") if lineage.is_some_and(|items| !items.is_empty()) => {
                    audit.reject("measured_value_must_not_claim_transform_lineage");
                }
                Some("stale" | "conflicted") => {
                    audit.abstain("observation_stale_or_conflicted");
                }
                Some("rejected" | "invalid") => {
                    audit.abstain("observation_explicitly_rejected_or_invalid");
                }
                Some("measured") => {}
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
            if value_at(record, &["value"]).and_then(Value::as_f64).is_some()
                && string_at(record, &["unit_ucum"]).is_none_or(str::is_empty)
            {
                audit.reject("unit_missing_for_numeric_value");
            }
            let calibration = string_at(record, &["quality", "calibration_state"]);
            let freshness = string_at(record, &["quality", "freshness_state"]);
            let measurement = string_at(record, &["quality", "measurement_quality"]);
            if calibration == Some("expired") || measurement == Some("invalid") {
                audit.reject("measurement_quality_invalid");
            } else if matches!(calibration, Some("not_calibrated" | "unknown"))
                || matches!(freshness, Some("stale" | "unknown"))
                || matches!(measurement, Some("degraded" | "suspect" | "unknown"))
            {
                audit.abstain("measurement_quality_not_qualified");
            }
        }
        Some("diagnostic_assertion") => {
            if !has_only_fields(
                value_at(record, &["assertion"]).unwrap_or(&Value::Null),
                &["kind", "code", "statement", "status", "confidence", "severity", "claim_scope"],
            ) || !has_only_fields(
                value_at(record, &["method"]).unwrap_or(&Value::Null),
                &["provider", "name", "version", "algorithm_sha256"],
            ) || !has_only_fields(
                value_at(record, &["evidence_links"]).unwrap_or(&Value::Null),
                &["supports", "contradicts", "derived_from"],
            ) {
                audit.reject("assertion_unknown_field_or_shape");
            }
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
            audit.evidence_links_resolved = Some(valid_links);
            if !valid_links {
                audit.reject("assertion_evidence_links_incomplete");
            }
            if string_at(record, &["assertion", "status"]) == Some("withdrawn") {
                audit.abstain("assertion_withdrawn");
            }
            if string_at(record, &["assertion", "status"]).is_some_and(|status| {
                !["source_reported", "reviewed", "disputed", "withdrawn"].contains(&status)
            }) {
                audit.reject("assertion_status_invalid");
            }
            if let (Some(supports), Some(contradicts)) = (
                value_at(record, &["evidence_links", "supports"]).and_then(Value::as_array),
                value_at(record, &["evidence_links", "contradicts"]).and_then(Value::as_array),
            ) {
                if supports.iter().any(|item| contradicts.contains(item)) {
                    audit.reject("assertion_same_evidence_supports_and_contradicts");
                    audit.evidence_links_resolved = Some(false);
                }
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

fn validate_lineage_references(record: &Value, ids: &HashSet<String>) -> bool {
    if string_at(record, &["record_type"]) != Some("observation")
        || !matches!(string_at(record, &["value_state"]), Some("derived" | "reconstructed" | "imputed"))
    {
        return true;
    }
    let Some(lineage) = value_at(record, &["lineage"]).and_then(Value::as_array) else {
        return false;
    };
    lineage.iter().all(|item| {
        value_at(item, &["input_record_refs"])
            .and_then(Value::as_array)
            .is_some_and(|references| {
                !references.is_empty()
                    && references.iter().all(|reference| {
                        reference.as_str().is_some_and(|id| ids.contains(id))
                    })
            })
    })
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
    if !has_only_fields(
        &corpus,
        &["corpus_id", "corpus_version", "claim_ceiling", "evaluation_time", "fixture_count", "fixtures"],
    ) {
        return Err("corpus_unknown_field_or_shape".into());
    }
    if string_at(&corpus, &["corpus_version"]) != Some("0.1.0")
        || string_at(&corpus, &["claim_ceiling"])
            != Some("Synthetic conformance fixtures only; not physical diagnostic validation.")
    {
        return Err("corpus_identity_or_claim_ceiling_invalid".into());
    }

    let evaluation_time = string_at(&corpus, &["evaluation_time"])
        .ok_or_else(|| "corpus_evaluation_time_missing".to_string())?;
    let evaluation_epoch = parse_rfc3339_ns(evaluation_time)
        .map_err(|reason| format!("corpus_evaluation_time_invalid:{reason}"))?;
    let fixtures = corpus.get("fixtures").and_then(Value::as_array)
        .ok_or_else(|| "corpus_fixtures_missing".to_string())?;
    if u64_at(&corpus, &["fixture_count"]) != Some(fixtures.len() as u64) {
        return Err("corpus_fixture_count_mismatch".into());
    }
    let corpus_ids = record_ids(&corpus)?;

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
    let mut evidence_link_total = 0usize;
    let mut evidence_link_resolved = 0usize;
    let mut duplicate_gate_ok = true;
    let mut duplicate_seen = false;
    let mut digest_negative_control_ok = true;
    let mut digest_mismatch_seen = false;
    let mut references_ok = true;

    let mut fixture_ids = HashSet::new();
    for fixture in fixtures {
        let case_start = Instant::now();
        if !has_only_fields(
            fixture,
            &["fixture_id", "expected_disposition", "expected_invariants", "record", "companion_records"],
        ) {
            return Err("fixture_unknown_field_or_shape".into());
        }
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
            let mut audit = validate_record(record, evaluation_epoch);
            if !validate_lineage_references(record, &corpus_ids) {
                audit.reject("lineage_input_reference_unresolved");
                references_ok = false;
            }
            if string_at(record, &["record_type"]) == Some("diagnostic_assertion")
                && !validate_evidence_references(record, &corpus_ids)
            {
                audit.reject("assertion_evidence_reference_unresolved");
                audit.evidence_links_resolved = Some(false);
                references_ok = false;
            }
            if let Some(digest_ok) = audit.payload_digest_match {
                digest_checked += 1;
                if digest_ok {
                    digest_matches += 1;
                } else {
                    digest_mismatch_seen = true;
                    if expected_disp != Disposition::Reject {
                        digest_negative_control_ok = false;
                    }
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
            if audit.evidence_links_resolved.is_some() {
                evidence_link_total += 1;
                if audit.evidence_links_resolved == Some(true) {
                    evidence_link_resolved += 1;
                }
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
            duplicate_seen = true;
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
    let evidence_link_resolution_gate = evidence_link_total == evidence_link_resolved;
    let digest_gate = digest_negative_control_ok && digest_mismatch_seen;
    let duplicate_gate = duplicate_gate_ok && duplicate_seen;
    let native_gate = native_total == native_present && native_total > 0;
    let provenance_gate = provenance_total > 0 && provenance_complete == provenance_total;
    let all_gates = disposition_gate && digest_gate && ambiguous_gate
        && evidence_link_resolution_gate && duplicate_gate && references_ok && native_gate && provenance_gate;

    let report = json!({
        "report_version": "0.1.0",
        "report_id": format!("local-{}", started_epoch),
        "profile_version": "0.1.0",
        "validator_id": "maritime-evidence-interchange-semantic-validator",
        "validator_version": env!("CARGO_PKG_VERSION"),
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
            "native_identifier_completeness_rate": if native_total == 0 { Value::Null } else { json!(native_present as f64 / native_total as f64) },
            "mapping_precision": Value::Null,
            "mapping_coverage": Value::Null,
            "ambiguous_mapping_abstention_rate": if ambiguous_total == 0 { Value::Null } else { json!(ambiguous_abstained as f64 / ambiguous_total as f64) },
            "assertion_evidence_link_resolution_rate": if evidence_link_total == 0 { Value::Null } else { json!(evidence_link_resolved as f64 / evidence_link_total as f64) },
            "input_provenance_completeness_rate": if provenance_total == 0 { Value::Null } else { json!(provenance_complete as f64 / provenance_total as f64) },
            "event_order_preservation_rate": Value::Null,
            "p95_processing_ms": p95,
            "peak_memory_bytes": Value::Null
        },
        "hard_gates": [
            {"gate_id":"expected_dispositions_match","passed":disposition_gate,"failure_count":failed_cases},
            {"gate_id":"digest_negative_control_rejected","passed":digest_gate,"failure_count":if digest_gate { 0usize } else { 1usize }},
            {"gate_id":"ambiguous_mapping_abstention","passed":ambiguous_gate,"failure_count":ambiguous_total.saturating_sub(ambiguous_abstained)},
            {"gate_id":"duplicate_stream_sequence_rejected","passed":duplicate_gate,"failure_count":if duplicate_gate { 0usize } else { 1usize }},
            {"gate_id":"assertion_evidence_references_resolve","passed":references_ok,"failure_count":if references_ok { 0usize } else { 1usize }},
            {"gate_id":"native_channel_identifiers_present","passed":native_gate,"failure_count":if native_gate { 0usize } else { 1usize }},
            {"gate_id":"provenance_completeness","passed":provenance_gate,"failure_count":provenance_total.saturating_sub(provenance_complete)},
            {"gate_id":"assertion_conflict_links_well_formed","passed":evidence_link_resolution_gate,"failure_count":evidence_link_total.saturating_sub(evidence_link_resolved)}
        ],
        "case_results": case_reports,
        "claim_ceiling": "synthetic_profile_semantics_only",
        "validator_scope_note": "Synthetic semantic corpus validation only. This does not execute an adapter, prove round-trip preservation, or establish physical diagnostic performance.",
        "elapsed_ms": elapsed_ms
    });

    Ok((report, failed_cases + if all_gates { 0usize } else { 1usize }))
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

    fn evaluate_mutated(
        mutate: impl FnOnce(&mut Value),
    ) -> Result<(Value, usize), String> {
        let mut corpus: Value = serde_json::from_str(CORPUS).expect("fixture corpus parses");
        mutate(&mut corpus);
        let bytes = serde_json::to_vec(&corpus).expect("mutated corpus serializes");
        evaluate_corpus(&bytes)
    }

    fn case<'a>(report: &'a Value, id: &str) -> &'a Value {
        report["case_results"].as_array().expect("case results array")
            .iter()
            .find(|result| result["fixture_id"] == id)
            .unwrap_or_else(|| panic!("missing case result {id}"))
    }

    #[test]
    fn frozen_corpus_matches_all_expected_dispositions() {
        let (report, failures) = evaluate_corpus(CORPUS.as_bytes()).expect("corpus should parse");
        assert_eq!(failures, 0, "{report:#}");
        assert_eq!(report["summary"]["cases_total"], 13);
        assert_eq!(report["summary"]["accepted"], 7);
        assert_eq!(report["summary"]["abstained"], 2);
        assert_eq!(report["summary"]["rejected"], 4);
        assert_eq!(report["summary"]["native_identifier_completeness_rate"], 1.0);
        assert_eq!(report["summary"]["assertion_evidence_link_resolution_rate"], 1.0);
    }

    #[test]
    fn profile_schemas_parse_and_corpus_ref_targets_record_schema() {
        let record_schema: Value = serde_json::from_str(include_str!("../../../docs/tech/MARITIME_EVIDENCE_INTERCHANGE_V0_1.schema.json")).unwrap();
        let report_schema: Value = serde_json::from_str(include_str!("../../../docs/tech/MARITIME_EVIDENCE_INTERCHANGE_CONFORMANCE_REPORT_V0_1.schema.json")).unwrap();
        let corpus_schema: Value = serde_json::from_str(include_str!("../../../docs/tech/MARITIME_EVIDENCE_INTERCHANGE_CORPUS_V0_1.schema.json")).unwrap();
        assert_eq!(corpus_schema["$defs"]["record"]["$ref"], record_schema["$id"]);
        assert!(report_schema["properties"]["validator_id"].is_object());
        assert!(report_schema["properties"]["validator_version"].is_object());
    }

    #[test]
    fn base64_decoder_rejects_noncanonical_unused_bits() {
        assert!(decode_base64("Zh==").is_err());
        assert_eq!(decode_base64("Zg==").expect("valid base64"), b"f");
    }

    #[test]
    fn rfc3339_offsets_are_compared_as_instants() {
        let utc = parse_rfc3339_ns("2024-12-31T22:00:00Z").unwrap();
        let offset = parse_rfc3339_ns("2025-01-01T00:00:00+02:00").unwrap();
        assert_eq!(utc, offset);
    }

    #[test]
    fn rfc3339_fractional_seconds_are_preserved() {
        let whole = parse_rfc3339_ns("2026-01-01T12:00:00Z").unwrap();
        let fraction = parse_rfc3339_ns("2026-01-01T12:00:00.125Z").unwrap();
        assert_eq!(fraction - whole, 125_000_000);
    }

    #[test]
    fn rfc3339_parser_rejects_invalid_calendar_dates_and_precision() {
        assert!(parse_rfc3339_ns("2025-02-29T23:59:59Z").is_err());
        assert!(parse_rfc3339_ns("2025-01-01T00:00:00.1234567890Z").is_err());
        assert!(parse_rfc3339_ns("2025-01-01T00:00:00").is_err());
    }

    #[test]
    fn ambiguous_and_unmapped_channels_abstain() {
        let (report, failures) = evaluate_corpus(CORPUS.as_bytes()).unwrap();
        assert_eq!(failures, 0);
        assert_eq!(case(&report, "obs-005-ambiguous-map-abstains")["actual_disposition"], "abstain");
        assert_eq!(case(&report, "obs-006-unmapped-channel-abstains")["actual_disposition"], "abstain");
    }

    #[test]
    fn missing_measurement_is_not_converted_to_zero_or_healthy() {
        let (report, failures) = evaluate_corpus(CORPUS.as_bytes()).unwrap();
        assert_eq!(failures, 0);
        assert_eq!(case(&report, "obs-003-missing-no-value")["actual_disposition"], "accept");
        assert_eq!(case(&report, "obs-003-missing-no-value")["payload_digest_match"], true);
    }

    #[test]
    fn unknown_clock_quality_forces_abstention() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][0]["record"]["time"]["clock_quality"] = json!("unknown");
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-001-measured-exact")["actual_disposition"], "abstain");
    }

    #[test]
    fn stale_measurement_quality_forces_abstention() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][0]["record"]["quality"]["freshness_state"] = json!("stale");
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-001-measured-exact")["actual_disposition"], "abstain");
    }

    #[test]
    fn derived_values_require_transformation_lineage() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][1]["record"]["lineage"] = json!([]);
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-002-derived-with-lineage")["actual_disposition"], "reject");
    }

    #[test]
    fn derived_lineage_inputs_must_resolve_to_corpus_records() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][1]["record"]["lineage"][0]["input_record_refs"] = json!(["missing-record"]);
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-002-derived-with-lineage")["actual_disposition"], "reject");
    }

    #[test]
    fn future_event_time_is_rejected() {
        let (report, failures) = evaluate_corpus(CORPUS.as_bytes()).unwrap();
        assert_eq!(failures, 0);
        assert_eq!(case(&report, "obs-010-future-event-time")["actual_disposition"], "reject");
    }

    #[test]
    fn modified_payload_digest_is_rejected_by_negative_control() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][8]["record"]["provenance"]["source_payload"]["sha256"] =
                json!("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb");
        }).unwrap();
        assert_eq!(failures, 0, "{report:#}");
        assert_eq!(case(&report, "obs-009-bad-payload-digest")["actual_disposition"], "reject");
        assert_eq!(case(&report, "obs-009-bad-payload-digest")["payload_digest_match"], false);
    }

    #[test]
    fn unverified_source_signature_forces_abstention() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][0]["record"]["provenance"]["source_payload"]["signature_state"] =
                json!("unverified");
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-001-measured-exact")["actual_disposition"], "abstain");
    }

    #[test]
    fn invalid_source_signature_is_rejected() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][0]["record"]["provenance"]["source_payload"]["signature_state"] =
                json!("invalid");
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-001-measured-exact")["actual_disposition"], "reject");
    }

    #[test]
    fn unknown_record_fields_are_rejected() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][0]["record"]["unexpected_field"] = json!(true);
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-001-measured-exact")["actual_disposition"], "reject");
    }

    #[test]
    fn duplicate_stream_sequence_is_rejected() {
        let (report, failures) = evaluate_corpus(CORPUS.as_bytes()).unwrap();
        assert_eq!(failures, 0);
        assert_eq!(case(&report, "obs-012-duplicate-sequence")["actual_disposition"], "reject");
    }

    #[test]
    fn diagnostic_assertion_cannot_grant_authority() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][6]["record"]["authority_granted"] = json!(true);
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "assert-007-vendor-alarm-not-measurement")["actual_disposition"], "reject");
    }

    #[test]
    fn assertion_evidence_references_must_resolve() {
        let (report, failures) = evaluate_mutated(|corpus| {
            corpus["fixtures"][6]["record"]["evidence_links"]["supports"] = json!(["unknown-record"]);
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "assert-007-vendor-alarm-not-measurement")["actual_disposition"], "reject");
    }

    #[test]
    fn remote_payload_reference_does_not_pass_offline_integrity() {
        let (report, failures) = evaluate_mutated(|corpus| {
            let payload = corpus["fixtures"][0]["record"]["provenance"]["source_payload"]
                .as_object_mut().unwrap();
            payload.remove("inline_base64");
            payload.insert("content_ref".into(), json!("urn:sha256:281a790c52136d39fb06b2064cb65e158a6c2184cbe4dfd991d2cda152423d67"));
        }).unwrap();
        assert!(failures > 0);
        assert_eq!(case(&report, "obs-001-measured-exact")["actual_disposition"], "reject");
    }

    #[test]
    fn duplicate_record_ids_fail_closed() {
        let mut corpus: Value = serde_json::from_str(CORPUS).unwrap();
        corpus["fixtures"][1]["record"]["record_id"] = json!("obs-001");
        let bytes = serde_json::to_vec(&corpus).unwrap();
        assert!(evaluate_corpus(&bytes).is_err());
    }

    #[test]
    fn fixture_count_mismatch_fails_closed() {
        let mut corpus: Value = serde_json::from_str(CORPUS).unwrap();
        corpus["fixture_count"] = json!(12);
        let bytes = serde_json::to_vec(&corpus).unwrap();
        assert!(evaluate_corpus(&bytes).is_err());
    }
}
