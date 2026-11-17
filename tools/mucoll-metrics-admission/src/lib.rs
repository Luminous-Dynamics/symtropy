// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

const POLICY_SCHEMA: &str = "0.1.0";
const REPORT_SCHEMA: u64 = 1;
const FRAGMENT_SCHEMA: u64 = 1;

pub fn audit_files(
    report_path: &Path,
    policy_path: &Path,
    input_path: &Path,
    fragments_dir: &Path,
    submission_path: &Path,
) -> Result<Value, Vec<String>> {
    let report_bytes = fs::read(report_path).map_err(|error| {
        vec![format!("cannot read aggregate report {}: {error}", report_path.display())]
    })?;
    let policy_bytes = fs::read(policy_path).map_err(|error| {
        vec![format!("cannot read admission policy {}: {error}", policy_path.display())]
    })?;
    let input_bytes = fs::read(input_path).map_err(|error| {
        vec![format!("cannot read RECO input {}: {error}", input_path.display())]
    })?;
    let submission_bytes = fs::read(submission_path).map_err(|error| {
        vec![format!("cannot read submission provenance {}: {error}", submission_path.display())]
    })?;

    let report: Value = serde_json::from_slice(&report_bytes)
        .map_err(|error| vec![format!("aggregate report is not valid JSON: {error}")])?;
    let policy: Value = serde_json::from_slice(&policy_bytes)
        .map_err(|error| vec![format!("admission policy is not valid JSON: {error}")])?;

    let expected_studies = policy
        .get("expected_studies")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["policy.expected_studies must be an array".to_owned()])?;
    let sample = report
        .pointer("/samples")
        .and_then(Value::as_object)
        .and_then(|samples| {
            if samples.len() == 1 {
                samples.values().next()
            } else {
                None
            }
        });
    let reported_studies = sample.and_then(|value| value.get("studies")).and_then(Value::as_object);

    let mut fragments = BTreeMap::new();
    if let Some(studies) = reported_studies {
        for study in expected_studies.iter().filter_map(Value::as_str) {
            let file_name = studies
                .get(study)
                .and_then(|value| value.pointer("/fragment/name"))
                .and_then(Value::as_str);
            if let Some(file_name) = file_name {
                if safe_basename(file_name) {
                    match fs::read(fragments_dir.join(file_name)) {
                        Ok(bytes) => {
                            fragments.insert(study.to_owned(), bytes);
                        }
                        Err(error) => {
                            return Err(vec![format!(
                                "cannot read fragment {file_name:?}: {error}"
                            )]);
                        }
                    }
                }
            }
        }
    }

    let submission: Value = serde_json::from_slice(&submission_bytes)
        .map_err(|error| vec![format!("submission provenance is not valid JSON: {error}")])?;

    audit_bytes(
        &report,
        &policy,
        &input_bytes,
        &fragments,
        &submission,
        &submission_bytes,
    )
}

pub fn audit_bytes(
    report: &Value,
    policy: &Value,
    input_bytes: &[u8],
    fragments: &BTreeMap<String, Vec<u8>>,
    submission: &Value,
    submission_bytes: &[u8],
) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();
    validate_policy(policy, &mut errors);

    if report.get("schema_version").and_then(Value::as_u64) != Some(REPORT_SCHEMA) {
        errors.push("aggregate report schema_version must be 1".to_owned());
    }
    if !json_numbers_are_finite(report) {
        errors.push("aggregate report contains a non-finite or unsupported numeric value".to_owned());
    }
    if !json_numbers_are_finite(submission) {
        errors.push("submission metadata contains a non-finite or unsupported numeric value".to_owned());
    }

    let stack_version = report
        .pointer("/software/mucoll_stack_version")
        .and_then(Value::as_str);
    if stack_version.is_none_or(str::is_empty) {
        errors.push("software.mucoll_stack_version is required".to_owned());
    }

    let benchmark_revision = policy
        .pointer("/benchmark_repository/revision")
        .and_then(Value::as_str);
    let report_benchmark_revision = report
        .pointer("/provenance/benchmark_repository/revision")
        .and_then(Value::as_str);
    if benchmark_revision.is_none() || report_benchmark_revision != benchmark_revision {
        errors.push("benchmark repository revision does not match the source-lock policy".to_owned());
    }
    if policy
        .pointer("/benchmark_repository/require_clean_worktree")
        .and_then(Value::as_bool)
        == Some(true)
        && report
            .pointer("/provenance/benchmark_repository/dirty")
            .and_then(Value::as_bool)
            != Some(false)
    {
        errors.push("benchmark repository working tree must be clean".to_owned());
    }

    let expected_spack_revision = policy
        .pointer("/stack_repository/revision")
        .and_then(Value::as_str);
    let submission_spack_revision = submission
        .pointer("/revisions/mucoll_spack")
        .and_then(Value::as_str);
    if expected_spack_revision.is_none() || submission_spack_revision != expected_spack_revision {
        errors.push("MuColl Spack source revision does not match the source-lock policy".to_owned());
    }
    let submission_benchmark_revision = submission
        .pointer("/revisions/mucoll_benchmarks")
        .and_then(Value::as_str);
    if benchmark_revision.is_none() || submission_benchmark_revision != benchmark_revision {
        errors.push("submission benchmark revision does not match the source-lock policy".to_owned());
    }

    let aggregator_name = report
        .pointer("/provenance/aggregator/name")
        .and_then(Value::as_str);
    if aggregator_name != Some("aggregate_metrics.py") {
        errors.push("provenance.aggregator.name must identify aggregate_metrics.py".to_owned());
    }
    validate_digest(
        report.pointer("/provenance/aggregator/sha256").and_then(Value::as_str),
        "provenance.aggregator.sha256",
        &mut errors,
    );

    let production_name = report
        .pointer("/provenance/production/name")
        .and_then(Value::as_str);
    if production_name != Some("submission.json") {
        errors.push("provenance.production.name must identify submission.json".to_owned());
    }
    let submission_sha256 = sha256_hex(submission_bytes);
    let reported_submission_sha256 = report
        .pointer("/provenance/production/sha256")
        .and_then(Value::as_str);
    if reported_submission_sha256 != Some(submission_sha256.as_str()) {
        errors.push("submission.json SHA-256 does not match aggregate production provenance".to_owned());
    }
    if report.pointer("/provenance/production/metadata") != Some(submission) {
        errors.push("embedded production metadata differs from the supplied submission.json".to_owned());
    }

    validate_submission(submission, policy, &mut errors);
    validate_github_run(report, submission, policy, &mut errors);

    if report.get("physics_thresholds").is_some()
        || report.get("physics_comparison").and_then(Value::as_str).is_some_and(|value| value != "not_evaluated")
        || report.get("qualification_decision").and_then(Value::as_str).is_some_and(|value| value != "abstain")
    {
        errors.push("report attempts a physics qualification without a separately admitted reference set".to_owned());
    }
    if policy
        .get("forbid_physics_thresholds")
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("policy must forbid physics thresholds in this admission stage".to_owned());
    }

    let samples = report.pointer("/samples").and_then(Value::as_object);
    let null_sample = Value::Null;
    let (sample_label, sample) = match samples {
        Some(samples) if samples.len() == 1 => samples.iter().next().unwrap(),
        Some(samples) => {
            errors.push(format!(
                "aggregate report must contain exactly one sample; found {}",
                samples.len()
            ));
            ("", &null_sample)
        }
        None => {
            errors.push("samples must be an object".to_owned());
            ("", &Value::Null)
        }
    };
    if !sample_label.is_empty() {
        if sample.get("label").and_then(Value::as_str) != Some(sample_label) {
            errors.push("sample label must match its samples object key".to_owned());
        }
    }

    let event_count = sample.get("event_count").and_then(Value::as_u64);
    if event_count.is_none_or(|count| count == 0) {
        errors.push("sample event_count must be a positive integer".to_owned());
    }
    if event_count != submission.get("events").and_then(Value::as_u64) {
        errors.push("generated event count in submission.json differs from reconstructed sample event count".to_owned());
    }
    if !sample_label.is_empty()
        && submission.get("particle").and_then(Value::as_str) != Some(sample_label)
    {
        errors.push("sample label differs from submitted generated particle identity".to_owned());
    }

    let input_name = sample.pointer("/input/name").and_then(Value::as_str);
    if input_name.is_none_or(str::is_empty) {
        errors.push("sample.input.name is required".to_owned());
    }
    let input_bytes_reported = sample.pointer("/input/bytes").and_then(Value::as_u64);
    if input_bytes_reported != Some(input_bytes.len() as u64) {
        errors.push(format!(
            "RECO input byte count mismatch: aggregate reports {:?}, observed {}",
            input_bytes_reported,
            input_bytes.len()
        ));
    }
    let input_sha256 = sha256_hex(input_bytes);
    if sample.pointer("/input/sha256").and_then(Value::as_str) != Some(input_sha256.as_str()) {
        errors.push("RECO input SHA-256 does not match aggregate report".to_owned());
    }

    let expected_studies = policy
        .get("expected_studies")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let mut admitted_studies = Vec::new();
    let mut observed_fragment_names = BTreeSet::new();
    let mut observed_input_path: Option<String> = None;
    for study_name in expected_studies.iter().filter_map(Value::as_str) {
        match admit_study(
            study_name,
            sample.pointer("/studies"),
            fragments.get(study_name),
            input_bytes.len() as u64,
            event_count,
            &mut observed_fragment_names,
            &mut observed_input_path,
            &mut errors,
        ) {
            Some(study) => admitted_studies.push(study),
            None => {}
        }
    }

    if errors.is_empty() {
        Ok(json!({
            "schema_version": "0.1.0",
            "policy_id": policy["policy_id"],
            "status": "artifact_integrity_passed",
            "artifact_integrity": "valid",
            "execution_receipt": "valid",
            "physics_comparison": "not_evaluated",
            "qualification_decision": "abstain",
            "sample": {
                "label": sample_label,
                "event_count": event_count,
                "reco_input_name": input_name,
                "reco_input_bytes": input_bytes.len(),
                "reco_input_sha256": input_sha256
            },
            "source_identity": {
                "benchmark_repository": policy["benchmark_repository"],
                "stack_repository": policy["stack_repository"],
                "mucoll_stack_version": stack_version,
                "container_image_id": submission.pointer("/container/image_id"),
                "container_platform": submission.pointer("/container/platform"),
                "geometry": submission.get("geometry")
            },
            "submission_sha256": submission_sha256,
            "admitted_studies": admitted_studies,
            "claim_ceiling": policy["claim_ceiling"]
        }))
    } else {
        Err(errors)
    }
}

fn validate_policy(policy: &Value, errors: &mut Vec<String>) {
    if policy.get("schema_version").and_then(Value::as_str) != Some(POLICY_SCHEMA) {
        errors.push("unsupported policy schema_version".to_owned());
    }
    if policy.get("policy_id").and_then(Value::as_str).is_none_or(str::is_empty) {
        errors.push("policy_id is required".to_owned());
    }
    if policy
        .pointer("/benchmark_repository/name")
        .and_then(Value::as_str)
        != Some("MuonColliderSoft/mucoll-benchmarks")
    {
        errors.push("policy benchmark repository must be MuonColliderSoft/mucoll-benchmarks".to_owned());
    }
    if policy
        .pointer("/stack_repository/name")
        .and_then(Value::as_str)
        != Some("MuonColliderSoft/mucoll-spack")
    {
        errors.push("policy stack repository must be MuonColliderSoft/mucoll-spack".to_owned());
    }
    if policy
        .pointer("/benchmark_repository/revision")
        .and_then(Value::as_str)
        .is_none_or(str::is_empty)
        || policy
            .pointer("/stack_repository/revision")
            .and_then(Value::as_str)
            .is_none_or(str::is_empty)
    {
        errors.push("policy must pin benchmark and stack repository revisions".to_owned());
    }
    if policy
        .get("expected_studies")
        .and_then(Value::as_array)
        .is_none_or(|studies| studies.is_empty())
    {
        errors.push("policy.expected_studies must contain at least one study".to_owned());
    }
    if let Some(studies) = policy.get("expected_studies").and_then(Value::as_array) {
        let names: Vec<&str> = studies.iter().filter_map(Value::as_str).collect();
        let unique: BTreeSet<&str> = names.iter().copied().collect();
        if names.len() != studies.len() || unique.len() != names.len() || names.iter().any(|name| name.is_empty()) {
            errors.push("policy.expected_studies must contain unique non-empty study names".to_owned());
        }
    }
    if policy
        .get("require_github_actions")
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("policy must require GitHub Actions run provenance".to_owned());
    }
    if policy.get("physics_thresholds").is_some() {
        errors.push("physics_thresholds are forbidden in an artifact-integrity policy".to_owned());
    }
}

fn validate_submission(submission: &Value, policy: &Value, errors: &mut Vec<String>) {
    if submission.get("schema_version").and_then(Value::as_u64) != Some(1) {
        errors.push("submission schema_version must be 1".to_owned());
    }
    let generated_input = submission.get("input");
    if generated_input.is_none_or(|input| {
        input.get("name").and_then(Value::as_str).is_none_or(str::is_empty)
            || input.get("source").and_then(Value::as_str).is_none_or(|source| {
                !(source.starts_with("https://") || source.starts_with("http://"))
            })
            || input.get("bytes").and_then(Value::as_u64).is_none_or(|bytes| bytes == 0)
            || input.get("sha256").and_then(Value::as_str).is_none_or(|sha| !valid_digest(sha))
    }) {
        errors.push("submission input provenance must include source URL, name, positive byte count and SHA-256".to_owned());
    }
    if submission.get("events").and_then(Value::as_u64).is_none_or(|events| events == 0) {
        errors.push("submission.events must be a positive integer".to_owned());
    }
    if submission.get("particle").and_then(Value::as_str).is_none_or(str::is_empty) {
        errors.push("submission.particle is required".to_owned());
    }
    if submission.get("pdg").and_then(Value::as_i64).is_none() {
        errors.push("submission.pdg must be an integer".to_owned());
    }
    if submission.get("geometry").and_then(Value::as_str).is_none_or(str::is_empty) {
        errors.push("submission.geometry is required".to_owned());
    }

    let image_id = submission.pointer("/container/image_id").and_then(Value::as_str);
    let immutable_id = image_id.is_some_and(|value| {
        let digest = value.strip_prefix("sha256:");
        digest.is_some_and(valid_digest)
    });
    if !immutable_id {
        errors.push("container.image_id must be a full sha256 image identity, not only a mutable tag".to_owned());
    }
    if submission.pointer("/container/reference").and_then(Value::as_str).is_none_or(str::is_empty) {
        errors.push("container.reference is required".to_owned());
    }
    let platform = submission.pointer("/container/platform").and_then(Value::as_str);
    if platform.is_none_or(|value| !matches!(value, "linux/amd64" | "linux/arm64")) {
        errors.push("container.platform must be linux/amd64 or linux/arm64".to_owned());
    }

    if submission.pointer("/revisions/mucoll_benchmarks").and_then(Value::as_str)
        != policy.pointer("/benchmark_repository/revision").and_then(Value::as_str)
    {
        errors.push("submission.revisions.mucoll_benchmarks must match the locked benchmark commit".to_owned());
    }
    if submission.pointer("/revisions/mucoll_spack").and_then(Value::as_str)
        != policy.pointer("/stack_repository/revision").and_then(Value::as_str)
    {
        errors.push("submission.revisions.mucoll_spack must match the locked stack commit".to_owned());
    }
    if submission.pointer("/github/run_id").and_then(Value::as_str).is_none_or(str::is_empty)
        || submission.pointer("/github/run_attempt").and_then(Value::as_str).is_none_or(str::is_empty)
        || submission.pointer("/github/sha").and_then(Value::as_str).is_none_or(str::is_empty)
        || submission.pointer("/github/ref").and_then(Value::as_str).is_none_or(str::is_empty)
    {
        errors.push("submission.github must include run_id, run_attempt, sha and ref".to_owned());
    }
    if submission.pointer("/github/sha").and_then(Value::as_str)
        != submission.pointer("/revisions/mucoll_spack").and_then(Value::as_str)
    {
        errors.push("submission GitHub source SHA differs from the pinned MuColl Spack revision".to_owned());
    }
}

fn validate_github_run(
    report: &Value,
    submission: &Value,
    policy: &Value,
    errors: &mut Vec<String>,
) {
    if policy.get("require_github_actions").and_then(Value::as_bool) != Some(true) {
        return;
    }
    let github = report.pointer("/provenance/github_actions");
    if github.is_none_or(|value| {
        value.get("run_id").and_then(Value::as_str).is_none_or(str::is_empty)
            || value.get("run_attempt").and_then(Value::as_str).is_none_or(str::is_empty)
            || value.get("sha").and_then(Value::as_str).is_none_or(str::is_empty)
            || value.get("ref").and_then(Value::as_str).is_none_or(str::is_empty)
    }) {
        errors.push("aggregate GitHub provenance must include run_id, run_attempt, sha and ref".to_owned());
        return;
    }
    for field in ["run_id", "run_attempt", "sha", "ref"] {
        let report_value = github.and_then(|value| value.get(field)).and_then(Value::as_str);
        let submission_value = submission.pointer(&format!("/github/{field}")).and_then(Value::as_str);
        if report_value != submission_value {
            errors.push(format!("aggregate and submission GitHub {field} values disagree"));
        }
    }
}

fn admit_study(
    study_name: &str,
    studies_value: Option<&Value>,
    fragment_bytes: Option<&Vec<u8>>,
    reco_input_bytes: u64,
    expected_event_count: Option<u64>,
    observed_fragment_names: &mut BTreeSet<String>,
    observed_input_path: &mut Option<String>,
    errors: &mut Vec<String>,
) -> Option<Value> {
    let Some(study) = studies_value.and_then(|value| value.get(study_name)) else {
        errors.push(format!("required study {study_name:?} is missing from aggregate report"));
        return None;
    };
    let Some(fragment_bytes) = fragment_bytes else {
        errors.push(format!("required fragment for study {study_name:?} is missing"));
        return None;
    };
    let fragment: Value = match serde_json::from_slice(fragment_bytes) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("fragment {study_name:?} is not valid JSON: {error}"));
            return None;
        }
    };
    if fragment.get("schema_version").and_then(Value::as_u64) != Some(FRAGMENT_SCHEMA) {
        errors.push(format!("fragment {study_name:?} schema_version must be 1"));
    }
    if fragment.get("study").and_then(Value::as_str) != Some(study_name) {
        errors.push(format!("fragment {study_name:?} has a different study identity"));
    }

    let fragment_name = study.pointer("/fragment/name").and_then(Value::as_str);
    if fragment_name.is_none_or(|name| !safe_basename(name)) {
        errors.push(format!("study {study_name:?} fragment name must be a safe basename"));
    } else if let Some(name) = fragment_name {
        if !observed_fragment_names.insert(name.to_owned()) {
            errors.push(format!("fragment basename {name:?} is reused by multiple studies"));
        }
    }

    let observed_hash = sha256_hex(fragment_bytes);
    if study.pointer("/fragment/sha256").and_then(Value::as_str) != Some(observed_hash.as_str()) {
        errors.push(format!("fragment {study_name:?} SHA-256 does not match aggregate report"));
    }

    let producer_name = fragment.pointer("/producer/name").and_then(Value::as_str);
    let producer_hash = fragment.pointer("/producer/sha256").and_then(Value::as_str);
    if producer_name.is_none_or(str::is_empty) {
        errors.push(format!("fragment {study_name:?} producer name is required"));
    }
    validate_digest(producer_hash, &format!("fragment {study_name}.producer.sha256"), errors);
    if study.get("producer") != fragment.get("producer") {
        errors.push(format!("fragment {study_name:?} producer identity differs from aggregate report"));
    }

    let input_path = fragment.pointer("/input/path").and_then(Value::as_str);
    if input_path.is_none_or(str::is_empty) {
        errors.push(format!("fragment {study_name:?} input path is required"));
    } else if let Some(path) = input_path {
        match observed_input_path {
            Some(observed) if observed != path => errors.push(format!(
                "fragment {study_name:?} refers to a different RECO input path"
            )),
            None => *observed_input_path = Some(path.to_owned()),
            _ => {}
        }
    }
    if fragment.pointer("/input/bytes").and_then(Value::as_u64) != Some(reco_input_bytes) {
        errors.push(format!("fragment {study_name:?} input byte count differs from RECO input"));
    }

    let total = fragment.pointer("/events/total").and_then(Value::as_u64);
    let selected = fragment.pointer("/events/selected").and_then(Value::as_u64);
    if total != expected_event_count {
        errors.push(format!("fragment {study_name:?} total event count differs from aggregate report"));
    }
    if selected.is_none_or(|value| total.is_none_or(|total| value > total)) {
        errors.push(format!("fragment {study_name:?} selected event count is invalid"));
    }
    if study.get("selected_event_count").and_then(Value::as_u64) != selected {
        errors.push(format!("study {study_name:?} selected event count differs from fragment"));
    }
    if study.get("configuration") != fragment.get("configuration") {
        errors.push(format!("study {study_name:?} configuration differs from fragment"));
    }
    if study.get("metrics") != fragment.get("metrics") {
        errors.push(format!("study {study_name:?} metrics differ from fragment"));
    }
    if !json_numbers_are_finite(&fragment) {
        errors.push(format!("fragment {study_name:?} contains a non-finite or unsupported numeric value"));
    }
    if !study.get("configuration").is_some_and(Value::is_object)
        || !study.get("metrics").is_some_and(Value::is_object)
    {
        errors.push(format!("study {study_name:?} must contain configuration and metrics objects"));
    }

    Some(json!({
        "study": study_name,
        "fragment": {
            "name": fragment_name,
            "sha256": observed_hash
        },
        "producer": {
            "name": producer_name,
            "sha256": producer_hash
        },
        "selected_event_count": selected
    }))
}

fn safe_basename(name: &str) -> bool {
    !name.is_empty()
        && name != "."
        && name != ".."
        && !name.contains('/')
        && !name.contains('\\')
        && Path::new(name).file_name().and_then(|value| value.to_str()) == Some(name)
}

fn validate_digest(value: Option<&str>, field: &str, errors: &mut Vec<String>) {
    if value.is_none_or(|digest| !valid_digest(digest)) {
        errors.push(format!("{field} must be a lowercase 64-character SHA-256 digest"));
    }
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn json_numbers_are_finite(value: &Value) -> bool {
    match value {
        Value::Number(number) => number.as_f64().is_some_and(f64::is_finite),
        Value::Array(items) => items.iter().all(json_numbers_are_finite),
        Value::Object(object) => object.values().all(json_numbers_are_finite),
        _ => true,
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(digest.len() * 2);
    for byte in digest {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    const BENCHMARK_REV: &str = "f1d1cddc412fa037dd21449af90c97ce4d16f293";
    const SPACK_REV: &str = "553debf49dd25628054e95f3a24d1d067d5ef1ed";
    const RECO_BYTES: &[u8] = b"immutable reco test artifact";

    struct Fixture {
        policy: Value,
        report: Value,
        submission: Value,
        submission_bytes: Vec<u8>,
        fragments: BTreeMap<String, Vec<u8>>,
    }

    fn digest(bytes: &[u8]) -> String {
        sha256_hex(bytes)
    }

    fn fixture() -> Fixture {
        let policy = json!({
            "schema_version":"0.1.0",
            "policy_id":"test-policy",
            "benchmark_repository":{"name":"MuonColliderSoft/mucoll-benchmarks","revision":BENCHMARK_REV,"require_clean_worktree":true},
            "stack_repository":{"name":"MuonColliderSoft/mucoll-spack","revision":SPACK_REV},
            "expected_studies":["tracks","seeds","hits","photons"],
            "require_github_actions":true,
            "forbid_physics_thresholds":true,
            "claim_ceiling":"artifact integrity only"
        });
        let submission = json!({
            "schema_version":1,
            "generated_utc":"2026-11-17T00:00:00+00:00",
            "input":{"path":"gen.edm4hep.root","source":"https://example.test/gen.edm4hep.root","bytes":4096,"sha256":digest(b"generated input")},
            "events":100,
            "particle":"muon",
            "pdg":-13,
            "geometry":"MAIA_v0",
            "container":{"reference":"ghcr.io/example/mucoll-sim:3.1","image_id":format!("sha256:{}", "a".repeat(64)),"platform":"linux/amd64"},
            "revisions":{"mucoll_benchmarks":BENCHMARK_REV,"mucoll_spack":SPACK_REV},
            "github":{"run_id":"1234","run_attempt":"1","sha":"0123456789012345678901234567890123456789","ref":"refs/heads/main"}
        });
        let submission_bytes = serde_json::to_vec(&submission).expect("serialize submission");
        let mut fragments = BTreeMap::new();
        let mut studies = serde_json::Map::new();
        for study in ["tracks", "seeds", "hits", "photons"] {
            let file_name = format!("metrics_{study}.json");
            let (metrics, configuration) = match study {
                "tracks" => (json!({"tracking_efficiency":0.91,"fake_rate":0.02}), json!({"collections":{"truth":"MCParticles","tracks":"Tracks"}})),
                "seeds" => (json!({"matched_fraction":0.94}), json!({"collections":{"truth":"MCParticles","seeds":"SiTracks_Refitted"}})),
                "hits" => (json!({"total_hit_multiplicity":{"count":100,"mean":25.0,"stddev":3.0}}), json!({"collections":{"tracker_hits":["VertexBarrelCollection"]}})),
                _ => (json!({"reconstruction_efficiency":0.88}), json!({"collections":{"truth":"MCParticles","pfos":"PandoraPFOs"}})),
            };
            let fragment = json!({
                "schema_version":1,
                "study":study,
                "input":{"path":"/work/reco.edm4hep.root","bytes":RECO_BYTES.len()},
                "producer":{"name":format!("study_{study}.py"),"sha256":"b".repeat(64)},
                "events":{"total":100,"selected":95},
                "configuration":configuration,
                "metrics":metrics
            });
            let bytes = serde_json::to_vec(&fragment).expect("serialize fragment");
            let sha = digest(&bytes);
            fragments.insert(study.to_owned(), bytes);
            studies.insert(study.to_owned(), json!({
                "fragment":{"name":file_name,"sha256":sha},
                "producer":fragment["producer"],
                "selected_event_count":95,
                "configuration":fragment["configuration"],
                "metrics":fragment["metrics"]
            }));
        }
        let report = json!({
            "schema_version":1,
            "generated_utc":"2026-11-17T00:00:00+00:00",
            "software":{"mucoll_stack_version":"3.1"},
            "provenance":{
                "aggregator":{"name":"aggregate_metrics.py","sha256":"c".repeat(64)},
                "benchmark_repository":{"revision":BENCHMARK_REV,"dirty":false},
                "production":{"name":"submission.json","sha256":digest(&submission_bytes),"metadata":submission},
                "github_actions":{"run_id":"1234","run_attempt":"1","sha":"0123456789012345678901234567890123456789","ref":"refs/heads/main"}
            },
            "samples":{
                "muon":{
                    "label":"muon",
                    "event_count":100,
                    "input":{"name":"reco.edm4hep.root","bytes":RECO_BYTES.len(),"sha256":digest(RECO_BYTES)},
                    "studies":studies
                }
            }
        });
        Fixture {policy,report,submission,submission_bytes,fragments}
    }

    fn audit(fixture: &Fixture, input: &[u8]) -> Result<Value, Vec<String>> {
        audit_bytes(&fixture.report,&fixture.policy,input,&fixture.fragments,&fixture.submission,&fixture.submission_bytes)
    }

    #[test]
    fn valid_upstream_shaped_report_is_admitted_without_physics_claim() {
        let f=fixture();
        let receipt=audit(&f,RECO_BYTES).expect("valid report");
        assert_eq!(receipt["status"],"artifact_integrity_passed");
        assert_eq!(receipt["artifact_integrity"],"valid");
        assert_eq!(receipt["physics_comparison"],"not_evaluated");
        assert_eq!(receipt["qualification_decision"],"abstain");
        assert_eq!(receipt["admitted_studies"].as_array().expect("studies").len(),4);
    }

    #[test]
    fn negative_control_changed_reco_input_fails_digest() {
        let f=fixture();
        let errors=audit(&f,b"tampered reco input").expect_err("changed RECO input must fail");
        assert!(errors.iter().any(|e|e.contains("SHA-256")));
    }

    #[test]
    fn negative_control_changed_submission_fails_digest_or_embedded_identity() {
        let mut f=fixture();
        f.submission_bytes.push(b' ');
        let errors=audit(&f,RECO_BYTES).expect_err("changed submission must fail");
        assert!(errors.iter().any(|e|e.contains("submission.json SHA-256")));
    }

    #[test]
    fn negative_control_missing_fragment_is_rejected() {
        let mut f=fixture();
        f.fragments.remove("hits");
        let errors=audit(&f,RECO_BYTES).expect_err("missing hits fragment must fail");
        assert!(errors.iter().any(|e|e.contains("fragment for study")));
    }

    #[test]
    fn negative_control_tampered_fragment_fails_digest() {
        let mut f=fixture();
        f.fragments.get_mut("tracks").expect("tracks").push(b' ');
        let errors=audit(&f,RECO_BYTES).expect_err("changed fragment must fail");
        assert!(errors.iter().any(|e|e.contains("SHA-256")));
    }

    #[test]
    fn negative_control_stale_benchmark_revision_is_rejected() {
        let mut f=fixture();
        f.report["provenance"]["benchmark_repository"]["revision"]=json!("deadbeef");
        let errors=audit(&f,RECO_BYTES).expect_err("stale benchmark commit must fail");
        assert!(errors.iter().any(|e|e.contains("source-lock policy")));
    }

    #[test]
    fn negative_control_dirty_benchmark_repository_is_rejected() {
        let mut f=fixture();
        f.report["provenance"]["benchmark_repository"]["dirty"]=json!(true);
        let errors=audit(&f,RECO_BYTES).expect_err("dirty source must fail");
        assert!(errors.iter().any(|e|e.contains("working tree must be clean")));
    }

    #[test]
    fn negative_control_event_count_disagreement_is_rejected() {
        let mut f=fixture();
        f.report["samples"]["muon"]["event_count"]=json!(99);
        let errors=audit(&f,RECO_BYTES).expect_err("event disagreement must fail");
        assert!(errors.iter().any(|e|e.contains("total event count")));
    }

    #[test]
    fn negative_control_mutable_container_identity_is_rejected() {
        let mut f=fixture();
        f.submission["container"]["image_id"]=json!("ghcr.io/example/mucoll-sim:latest");
        f.submission_bytes=serde_json::to_vec(&f.submission).expect("serialize");
        f.report["provenance"]["production"]["metadata"]=f.submission.clone();
        f.report["provenance"]["production"]["sha256"]=json!(digest(&f.submission_bytes));
        let errors=audit(&f,RECO_BYTES).expect_err("mutable image tag must fail");
        assert!(errors.iter().any(|e|e.contains("full sha256 image identity")));
    }

    #[test]
    fn negative_control_missing_geometry_is_rejected() {
        let mut f=fixture();
        f.submission["geometry"]=json!("");
        f.submission_bytes=serde_json::to_vec(&f.submission).expect("serialize");
        f.report["provenance"]["production"]["metadata"]=f.submission.clone();
        f.report["provenance"]["production"]["sha256"]=json!(digest(&f.submission_bytes));
        let errors=audit(&f,RECO_BYTES).expect_err("missing geometry must fail");
        assert!(errors.iter().any(|e|e.contains("geometry is required")));
    }

    #[test]
    fn negative_control_physics_thresholds_cannot_be_added() {
        let mut f=fixture();
        f.policy["physics_thresholds"]=json!({"tracking_efficiency_min":0.9});
        let errors=audit(&f,RECO_BYTES).expect_err("threshold policy must fail");
        assert!(errors.iter().any(|e|e.contains("physics_thresholds are forbidden")));
    }

    #[test]
    fn negative_control_report_cannot_claim_physics_pass() {
        let mut f=fixture();
        f.report["qualification_decision"]=json!("accept");
        f.report["physics_comparison"]=json!("pass");
        let errors=audit(&f,RECO_BYTES).expect_err("unauthorized physics pass must fail");
        assert!(errors.iter().any(|e|e.contains("attempts a physics qualification")));
    }

    #[test]
    fn negative_control_missing_github_attempt_is_rejected() {
        let mut f=fixture();
        f.report["provenance"]["github_actions"]["run_attempt"]=Value::Null;
        let errors=audit(&f,RECO_BYTES).expect_err("missing run attempt must fail");
        assert!(errors.iter().any(|e|e.contains("GitHub provenance")));
    }

    #[test]
    fn negative_control_fragment_path_traversal_is_rejected() {
        let mut f=fixture();
        f.report["samples"]["muon"]["studies"]["tracks"]["fragment"]["name"]=json!("../metrics_tracks.json");
        let errors=audit(&f,RECO_BYTES).expect_err("unsafe fragment basename must fail");
        assert!(errors.iter().any(|e|e.contains("safe basename")));
    }

    #[test]
    fn null_metric_values_remain_admissible_as_unavailable() {
        let mut f=fixture();
        let bytes={
            let fragment=f.fragments.get_mut("tracks").expect("tracks");
            let mut value:Value=serde_json::from_slice(fragment).expect("fragment JSON");
            value["metrics"]["tracking_efficiency"]=Value::Null;
            let bytes=serde_json::to_vec(&value).expect("serialize");
            *fragment=bytes.clone();
            bytes
        };
        f.report["samples"]["muon"]["studies"]["tracks"]["fragment"]["sha256"]=json!(digest(&bytes));
        f.report["samples"]["muon"]["studies"]["tracks"]["metrics"]["tracking_efficiency"]=Value::Null;
        let receipt=audit(&f,RECO_BYTES).expect("null means unavailable, not zero");
        assert_eq!(receipt["physics_comparison"],"not_evaluated");
    }
}
