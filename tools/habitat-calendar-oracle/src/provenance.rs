// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;

pub const FIXTURE_SCHEMA: &str = "luminous.habitat_calendar_provenance_fixture.v0.1";
pub const REPORT_SCHEMA: &str = "luminous.habitat_calendar_provenance_report.v0.1";
pub const FROZEN_FIXTURE_SHA256: &str =
    "7717acc7932c79ee056fc9ed33b46bd8fd4af2821204e952f82984c019a4128a";
const CLAIM_CEILING: &str =
    "calendar_artifact_metadata_and_payload_digest_integrity_only_not_timezone_conversion_conformance";

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Fixture {
    schema_version: String,
    suite_id: String,
    synthetic_only: bool,
    claim_ceiling: String,
    templates: Vec<Template>,
    negative_controls: Vec<NegativeControl>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Template {
    case_id: String,
    manifest: CalendarManifest,
    expected_status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct NegativeControl {
    case_id: String,
    template_case_id: String,
    mutation: Mutation,
    expected_status: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct Mutation {
    op: String,
    field: String,
    value: Value,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CalendarManifest {
    source_uri: Option<String>,
    source_revision: Option<String>,
    source_payload: Option<String>,
    source_sha256: Option<String>,
    iana_zone: Option<String>,
    tzdb_release: Option<String>,
    tzdb_payload: Option<String>,
    tzdb_sha256: Option<String>,
    time_basis: Option<String>,
    expansion_start_utc_epoch_seconds: Option<i64>,
    expansion_end_utc_epoch_seconds: Option<i64>,
    ambiguous_local_time_policy: Option<String>,
    nonexistent_local_time_policy: Option<String>,
    parser_version: Option<String>,
    compiler_version: Option<String>,
    expanded_intervals_payload: Option<String>,
    expanded_intervals_sha256: Option<String>,
    expanded_interval_count: Option<usize>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProvenanceCaseResult {
    pub case_id: String,
    pub status: String,
    pub reason: Option<String>,
    pub expected_status: String,
    pub expectation_match: bool,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
pub struct ProvenanceReport {
    pub schema_version: String,
    pub suite_id: String,
    pub input_sha256: String,
    pub input_bytes: usize,
    pub accepted_template_count: usize,
    pub negative_control_count: usize,
    pub rejected_negative_controls: usize,
    pub all_expected_statuses_match: bool,
    pub claim_ceiling: String,
    pub cases: Vec<ProvenanceCaseResult>,
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

fn required<'a>(value: &'a Option<String>, name: &str) -> Result<&'a str, String> {
    value
        .as_deref()
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("missing_or_empty:{name}"))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn verify_payload_digest(
    payload: &Option<String>,
    digest: &Option<String>,
    payload_name: &str,
    digest_name: &str,
) -> Result<(), String> {
    let payload = required(payload, payload_name)?;
    let digest = required(digest, digest_name)?;
    if !is_sha256(digest) {
        return Err(format!("invalid_sha256_encoding:{digest_name}"));
    }
    if sha256_hex(payload.as_bytes()) != digest {
        return Err(format!("payload_digest_mismatch:{payload_name}"));
    }
    Ok(())
}

fn is_iana_zone_id(value: &str) -> bool {
    if value == "UTC" || value == "Etc/UTC" {
        return true;
    }
    if value.starts_with('/') || value.ends_with('/') || value.contains(':') {
        return false;
    }
    if !value.contains('/') || value.contains("..") {
        return false;
    }
    value.split('/').all(|component| {
        !component.is_empty()
            && component.bytes().all(|byte| {
                byte.is_ascii_alphanumeric() || matches!(byte, b'_' | b'-' | b'+' | b'.')
            })
    })
}

fn is_tzdb_release(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 5
        && bytes[..4].iter().all(u8::is_ascii_digit)
        && bytes[4..]
            .iter()
            .all(|byte| byte.is_ascii_lowercase())
}

fn validate_manifest(manifest: &CalendarManifest) -> Result<(), String> {
    let source_uri = required(&manifest.source_uri, "source_uri")?;
    if !(source_uri.starts_with("https://") || source_uri.starts_with("synthetic://")) {
        return Err("unsupported_source_uri_scheme".to_owned());
    }
    required(&manifest.source_revision, "source_revision")?;
    verify_payload_digest(
        &manifest.source_payload,
        &manifest.source_sha256,
        "source_payload",
        "source_sha256",
    )?;

    let zone = required(&manifest.iana_zone, "iana_zone")?;
    if !is_iana_zone_id(zone) {
        return Err("iana_zone_must_be_a_region_identifier_not_a_fixed_offset".to_owned());
    }

    let tzdb_release = required(&manifest.tzdb_release, "tzdb_release")?;
    if !is_tzdb_release(tzdb_release) {
        return Err("tzdb_release_must_be_an_explicit_pinned_release".to_owned());
    }
    verify_payload_digest(
        &manifest.tzdb_payload,
        &manifest.tzdb_sha256,
        "tzdb_payload",
        "tzdb_sha256",
    )?;

    let time_basis = required(&manifest.time_basis, "time_basis")?;
    if time_basis != "IANA_LOCAL_WALL_CLOCK" {
        return Err("unsupported_or_floating_time_basis".to_owned());
    }
    let start = manifest
        .expansion_start_utc_epoch_seconds
        .ok_or_else(|| "missing_or_empty:expansion_start_utc_epoch_seconds".to_owned())?;
    let end = manifest
        .expansion_end_utc_epoch_seconds
        .ok_or_else(|| "missing_or_empty:expansion_end_utc_epoch_seconds".to_owned())?;
    if start < 0 || end <= start {
        return Err("invalid_expansion_horizon".to_owned());
    }

    let ambiguous = required(
        &manifest.ambiguous_local_time_policy,
        "ambiguous_local_time_policy",
    )?;
    if !matches!(ambiguous, "REJECT" | "RFC5545_FIRST_OCCURRENCE") {
        return Err("unknown_ambiguous_local_time_policy".to_owned());
    }
    let nonexistent = required(
        &manifest.nonexistent_local_time_policy,
        "nonexistent_local_time_policy",
    )?;
    if !matches!(nonexistent, "REJECT" | "RFC5545_OFFSET_BEFORE_GAP") {
        return Err("unknown_nonexistent_local_time_policy".to_owned());
    }

    required(&manifest.parser_version, "parser_version")?;
    required(&manifest.compiler_version, "compiler_version")?;
    verify_payload_digest(
        &manifest.expanded_intervals_payload,
        &manifest.expanded_intervals_sha256,
        "expanded_intervals_payload",
        "expanded_intervals_sha256",
    )?;
    if manifest.expanded_interval_count.unwrap_or(0) == 0 {
        return Err("missing_or_empty:expanded_interval_count".to_owned());
    }
    Ok(())
}

fn result(
    case_id: String,
    expected_status: &str,
    validation: Result<(), String>,
) -> ProvenanceCaseResult {
    let (status, reason) = match validation {
        Ok(()) => (
            "METADATA_PROVENANCE_PASS_NOT_TZDB_CONFORMANCE".to_owned(),
            None,
        ),
        Err(reason) => ("REJECTED".to_owned(), Some(reason)),
    };
    ProvenanceCaseResult {
        case_id,
        expectation_match: status == expected_status,
        status,
        reason,
        expected_status: expected_status.to_owned(),
    }
}

/// Validates immutable calendar source metadata and hashes, not time-zone conversion behavior.
pub fn evaluate_fixture_bytes(bytes: &[u8]) -> Result<ProvenanceReport, String> {
    let fixture: Fixture = serde_json::from_slice(bytes)
        .map_err(|error| format!("calendar provenance fixture rejected: {error}"))?;
    if fixture.schema_version != FIXTURE_SCHEMA
        || fixture.suite_id.trim().is_empty()
        || !fixture.synthetic_only
        || fixture.claim_ceiling != CLAIM_CEILING
    {
        return Err("unsupported calendar provenance fixture identity or claim ceiling".to_owned());
    }
    if fixture.templates.is_empty() || fixture.negative_controls.is_empty() {
        return Err("fixture must include valid templates and negative controls".to_owned());
    }

    let mut names = BTreeSet::new();
    let mut report_cases = Vec::new();
    for template in &fixture.templates {
        if template.case_id.trim().is_empty() || !names.insert(template.case_id.as_str()) {
            return Err("template IDs must be unique and nonempty".to_owned());
        }
        report_cases.push(result(
            template.case_id.clone(),
            &template.expected_status,
            validate_manifest(&template.manifest),
        ));
    }

    let mut rejected_negative_controls = 0usize;
    for control in &fixture.negative_controls {
        if control.case_id.trim().is_empty() || !names.insert(control.case_id.as_str()) {
            return Err("negative-control IDs must be unique and nonempty".to_owned());
        }
        let template = fixture
            .templates
            .iter()
            .find(|template| template.case_id == control.template_case_id)
            .ok_or_else(|| {
                format!(
                    "negative control {} references unknown template {}",
                    control.case_id, control.template_case_id
                )
            })?;
        let mut value = serde_json::to_value(&template.manifest)
            .map_err(|error| format!("could not serialize test manifest: {error}"))?;
        let object = value
            .as_object_mut()
            .ok_or_else(|| "internal calendar manifest was not an object".to_owned())?;

        match control.mutation.op.as_str() {
            "replace" => {
                if !object.contains_key(&control.mutation.field) {
                    return Err(format!(
                        "negative control {} mutates an unknown replacement field",
                        control.case_id
                    ));
                }
                object.insert(
                    control.mutation.field.clone(),
                    control.mutation.value.clone(),
                );
            }
            "add_unknown_field" => {
                if object.contains_key(&control.mutation.field) {
                    return Err(format!(
                        "negative control {} did not add a new field",
                        control.case_id
                    ));
                }
                object.insert(
                    control.mutation.field.clone(),
                    control.mutation.value.clone(),
                );
            }
            other => {
                return Err(format!(
                    "negative control {} has unsupported mutation operation {other}",
                    control.case_id
                ));
            }
        }

        let validation = match serde_json::from_value::<CalendarManifest>(value) {
            Ok(manifest) => validate_manifest(&manifest),
            Err(error) => Err(format!("strict_deserialization_rejected:{error}")),
        };
        let case_result = result(control.case_id.clone(), &control.expected_status, validation);
        if case_result.status == "REJECTED" {
            rejected_negative_controls += 1;
        }
        report_cases.push(case_result);
    }

    let all_expected_statuses_match = report_cases.iter().all(|case| case.expectation_match);
    Ok(ProvenanceReport {
        schema_version: REPORT_SCHEMA.to_owned(),
        suite_id: fixture.suite_id,
        input_sha256: sha256_hex(bytes),
        input_bytes: bytes.len(),
        accepted_template_count: fixture.templates.len(),
        negative_control_count: fixture.negative_controls.len(),
        rejected_negative_controls,
        all_expected_statuses_match,
        claim_ceiling: CLAIM_CEILING.to_owned(),
        cases: report_cases,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    const FIXTURE: &[u8] =
        include_bytes!("../../../docs/research/fixtures/HABITAT_CALENDAR_PROVENANCE_V0_1.json");

    #[test]
    fn frozen_provenance_fixture_passes_all_metadata_controls() {
        assert_eq!(sha256_hex(FIXTURE), FROZEN_FIXTURE_SHA256);
        let report = evaluate_fixture_bytes(FIXTURE).expect("fixture should be valid");
        assert_eq!(report.accepted_template_count, 2);
        assert_eq!(report.negative_control_count, 8);
        assert_eq!(report.rejected_negative_controls, 8);
        assert!(report.all_expected_statuses_match);
        assert!(report
            .cases
            .iter()
            .take(2)
            .all(|case| case.status == "METADATA_PROVENANCE_PASS_NOT_TZDB_CONFORMANCE"));
    }

    #[test]
    fn changed_source_payload_cannot_reuse_its_old_digest() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["templates"][0]["manifest"]["source_payload"] =
            Value::String("revised-hours=09:00-17:00".to_owned());
        // This changes an accepted template, so the fixture expectations no longer pass.
        let changed = serde_json::to_vec(&value).expect("serialized fixture");
        let report = evaluate_fixture_bytes(&changed).expect("changed fixture remains parseable");
        assert!(!report.all_expected_statuses_match);
        assert!(report.cases[0]
            .reason
            .as_deref()
            .unwrap_or_default()
            .contains("payload_digest_mismatch"));
    }

    #[test]
    fn unknown_manifest_fields_fail_closed() {
        let mut value: Value = serde_json::from_slice(FIXTURE).expect("fixture JSON");
        value["templates"][0]["manifest"]["surprise"] = Value::Bool(true);
        let changed = serde_json::to_vec(&value).expect("serialized fixture");
        assert!(evaluate_fixture_bytes(&changed).is_err());
    }
}
