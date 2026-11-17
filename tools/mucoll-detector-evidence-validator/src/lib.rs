// SPDX-License-Identifier: AGPL-3.0-or-later

use serde_json::{Map, Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

#[derive(Debug, Clone)]
pub struct ValidationInputs {
    pub report: PathBuf,
    pub reco_input: PathBuf,
    pub submission: PathBuf,
    pub generation_input: PathBuf,
    pub fragments_dir: PathBuf,
    pub producer_dir: PathBuf,
    pub aggregator_source: PathBuf,
    pub sample_label: String,
}

#[derive(Debug, Clone)]
struct FileIdentity {
    name: String,
    bytes: u64,
    sha256: String,
}

pub fn validate_package(inputs: &ValidationInputs) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();

    let report: Value = read_json(&inputs.report, "aggregate report", &mut errors)?;
    let submission: Value = read_json(&inputs.submission, "submission record", &mut errors)?;

    if !errors.is_empty() {
        return Err(errors);
    }

    let reco_identity = file_identity(&inputs.reco_input, "RECO input", &mut errors)?;
    let generation_identity =
        file_identity(&inputs.generation_input, "generation input", &mut errors)?;
    let submission_identity =
        file_identity(&inputs.submission, "submission record", &mut errors)?;
    let aggregator_identity =
        file_identity(&inputs.aggregator_source, "aggregator source", &mut errors)?;

    let report_schema = report.get("schema_version").and_then(Value::as_u64);
    if report_schema != Some(1) {
        errors.push(format!(
            "aggregate report schema_version must be 1, found {report_schema:?}"
        ));
    }

    let stack_version = report
        .pointer("/software/mucoll_stack_version")
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty());
    if stack_version.is_none() {
        errors.push("aggregate report has no non-empty mucoll-stack version".to_owned());
    }

    let sample = report
        .get("samples")
        .and_then(|value| value.get(&inputs.sample_label));
    let Some(sample) = sample else {
        errors.push(format!(
            "aggregate report does not contain sample label {:?}",
            inputs.sample_label
        ));
        return Err(errors);
    };

    if sample.get("label").and_then(Value::as_str) != Some(inputs.sample_label.as_str()) {
        errors.push("sample label field does not match its samples map key".to_owned());
    }

    let event_count = sample.get("event_count").and_then(Value::as_u64);
    if event_count.is_none() {
        errors.push("sample.event_count must be an unsigned integer".to_owned());
    }

    validate_report_reco_identity(sample, &reco_identity, &mut errors)?;
    validate_submission(
        &submission,
        &submission_identity,
        &generation_identity,
        event_count,
        &inputs.sample_label,
        &mut errors,
    )?;
    validate_report_provenance(
        &report,
        &submission,
        &submission_identity,
        &aggregator_identity,
        &inputs.sample_label,
        &mut errors,
    )?;

    let studies = sample
        .get("studies")
        .and_then(Value::as_object)
        .ok_or_else(|| vec!["sample.studies must be an object".to_owned()])?;

    if studies.is_empty() {
        errors.push("sample.studies must contain at least one metric fragment".to_owned());
    }

    let mut validated_studies = Vec::with_capacity(studies.len());
    let mut unknown_metric_values = 0_u64;
    let mut study_names = BTreeSet::new();

    for (study_key, study_report) in studies {
        if !study_names.insert(study_key.clone()) {
            errors.push(format!("duplicate study name {study_key:?}"));
            continue;
        }

        match validate_study(
            study_key,
            study_report,
            event_count,
            &reco_identity,
            &inputs.fragments_dir,
            &inputs.producer_dir,
        ) {
            Ok((study_receipt, null_count)) => {
                unknown_metric_values += null_count;
                validated_studies.push(study_receipt);
            }
            Err(study_errors) => errors.extend(study_errors),
        }
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let github_run = report
        .pointer("/provenance/github_actions/run_id")
        .and_then(Value::as_str)
        .map(str::to_owned);

    Ok(json!({
        "validator": "mucoll-detector-evidence-validator",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "evidence_package_status": "valid",
        "technical_status": "success",
        "metric_integrity_status": "valid",
        "physics_reference_status": "not_evaluated",
        "qualification_decision": "abstain",
        "sample": {
            "label": inputs.sample_label,
            "event_count": event_count,
            "reco_input": identity_value(&reco_identity),
            "generation_input": identity_value(&generation_identity),
            "geometry": submission.get("geometry"),
            "particle": submission.get("particle"),
            "pdg": submission.get("pdg"),
            "mucoll_stack_version": stack_version,
            "github_run_id": github_run
        },
        "provenance": {
            "submission": identity_value(&submission_identity),
            "aggregator": identity_value(&aggregator_identity),
            "benchmark_repository": report.pointer("/provenance/benchmark_repository"),
            "container": submission.get("container"),
            "revisions": submission.get("revisions")
        },
        "studies": validated_studies,
        "unknown_metric_value_count": unknown_metric_values,
        "physics_reference": {
            "status": "not_evaluated",
            "reason": "No approved high-statistics reference profile and physics tolerance set was supplied.",
            "pass_fail_thresholds_applied": false
        },
        "claim_ceiling": "Metric package integrity and provenance only; no detector-physics validation or performance qualification."
    }))
}

fn validate_study(
    study_key: &str,
    study_report: &Value,
    event_count: Option<u64>,
    reco_identity: &FileIdentity,
    fragments_dir: &Path,
    producer_dir: &Path,
) -> Result<(Value, u64), Vec<String>> {
    let mut errors = Vec::new();

    let fragment_meta = study_report
        .get("fragment")
        .ok_or_else(|| vec![format!("study {study_key:?} has no fragment identity")])?;
    let fragment_name = required_string(fragment_meta, "name", study_key, &mut errors);
    let fragment_sha = required_string(fragment_meta, "sha256", study_key, &mut errors);

    let producer_meta = study_report
        .get("producer")
        .ok_or_else(|| vec![format!("study {study_key:?} has no producer identity")])?;
    let producer_name = required_string(producer_meta, "name", study_key, &mut errors);
    let producer_sha = required_string(producer_meta, "sha256", study_key, &mut errors);

    let selected_report = study_report
        .get("selected_event_count")
        .and_then(Value::as_u64);
    if selected_report.is_none() {
        errors.push(format!(
            "study {study_key:?} selected_event_count must be an unsigned integer"
        ));
    }

    let configuration_report = study_report
        .get("configuration")
        .and_then(Value::as_object);
    if configuration_report.is_none() {
        errors.push(format!(
            "study {study_key:?} configuration must be an object"
        ));
    }

    let metrics_report = study_report.get("metrics").and_then(Value::as_object);
    if metrics_report.is_none() {
        errors.push(format!("study {study_key:?} metrics must be an object"));
    }

    let fragment_path = fragment_name
        .as_deref()
        .and_then(|name| confined_child(fragments_dir, name, &mut errors));
    let producer_path = producer_name
        .as_deref()
        .and_then(|name| confined_child(producer_dir, name, &mut errors));

    let fragment_bytes = fragment_path
        .as_ref()
        .and_then(|path| read_bytes(path, &format!("{study_key} metric fragment"), &mut errors));
    if let (Some(bytes), Some(declared)) = (&fragment_bytes, fragment_sha.as_deref()) {
        let actual = sha256_hex(bytes);
        if actual != declared {
            errors.push(format!(
                "study {study_key:?} fragment SHA-256 mismatch: declared {declared}, computed {actual}"
            ));
        }
    }

    let producer_identity = producer_path
        .as_ref()
        .and_then(|path| file_identity(path, &format!("{study_key} producer"), &mut errors));
    if let (Some(identity), Some(declared)) = (&producer_identity, producer_sha.as_deref()) {
        if identity.sha256 != declared {
            errors.push(format!(
                "study {study_key:?} producer SHA-256 mismatch: declared {declared}, computed {}",
                identity.sha256
            ));
        }
    }

    let fragment_value = fragment_bytes
        .as_deref()
        .and_then(|bytes| parse_json(bytes, &format!("{study_key} fragment"), &mut errors));

    let mut unknown_metric_values = 0_u64;
    if let Some(fragment) = fragment_value.as_ref() {
        validate_fragment(
            study_key,
            fragment,
            event_count,
            selected_report,
            reco_identity,
            producer_meta,
            configuration_report,
            metrics_report,
            &mut unknown_metric_values,
            &mut errors,
        );
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    Ok((
        json!({
            "study": study_key,
            "fragment": {
                "name": fragment_name,
                "sha256": fragment_sha
            },
            "producer": producer_meta,
            "selected_event_count": selected_report,
            "configuration": configuration_report,
            "metrics": metrics_report
        }),
        unknown_metric_values,
    ))
}

fn validate_fragment(
    study_key: &str,
    fragment: &Value,
    report_event_count: Option<u64>,
    report_selected_count: Option<u64>,
    reco_identity: &FileIdentity,
    report_producer: &Value,
    report_configuration: Option<&Map<String, Value>>,
    report_metrics: Option<&Map<String, Value>>,
    unknown_metric_values: &mut u64,
    errors: &mut Vec<String>,
) {
    if fragment.get("schema_version").and_then(Value::as_u64) != Some(1) {
        errors.push(format!(
            "study {study_key:?} fragment schema_version must be 1"
        ));
    }

    if fragment.get("study").and_then(Value::as_str) != Some(study_key) {
        errors.push(format!(
            "study {study_key:?} fragment study identity does not match report key"
        ));
    }

    let fragment_input = fragment.get("input");
    let input_name = fragment_input
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str)
        .and_then(|path| Path::new(path).file_name())
        .and_then(|name| name.to_str());
    if input_name != Some(reco_identity.name.as_str()) {
        errors.push(format!(
            "study {study_key:?} fragment input basename does not match RECO input"
        ));
    }

    let input_bytes = fragment_input
        .and_then(|value| value.get("bytes"))
        .and_then(Value::as_u64);
    if input_bytes != Some(reco_identity.bytes) {
        errors.push(format!(
            "study {study_key:?} fragment input byte size does not match RECO input"
        ));
    }

    let events = fragment.get("events");
    let fragment_total = events.and_then(|value| value.get("total")).and_then(Value::as_u64);
    let fragment_selected = events
        .and_then(|value| value.get("selected"))
        .and_then(Value::as_u64);

    if fragment_total != report_event_count {
        errors.push(format!(
            "study {study_key:?} fragment event total differs from aggregate sample"
        ));
    }
    if fragment_selected != report_selected_count {
        errors.push(format!(
            "study {study_key:?} fragment selected-event count differs from aggregate report"
        ));
    }
    if let (Some(total), Some(selected)) = (fragment_total, fragment_selected) {
        if selected > total {
            errors.push(format!(
                "study {study_key:?} selected-event count exceeds total events"
            ));
        }
    }

    let producer = fragment.get("producer");
    if producer != Some(report_producer) {
        errors.push(format!(
            "study {study_key:?} producer metadata differs between fragment and aggregate report"
        ));
    }

    if fragment.get("configuration").and_then(Value::as_object) != report_configuration {
        errors.push(format!(
            "study {study_key:?} configuration differs between fragment and aggregate report"
        ));
    }

    if fragment.get("metrics").and_then(Value::as_object) != report_metrics {
        errors.push(format!(
            "study {study_key:?} metrics differ between fragment and aggregate report"
        ));
    }

    if let Some(metrics) = fragment.get("metrics") {
        validate_metric_values(metrics, &format!("study {study_key:?} metrics"), unknown_metric_values, errors);
    }
}

fn validate_report_reco_identity(
    sample: &Value,
    reco_identity: &FileIdentity,
    errors: &mut Vec<String>,
) -> Result<(), Vec<String>> {
    let input = sample.get("input");
    let name = input.and_then(|value| value.get("name")).and_then(Value::as_str);
    let bytes = input.and_then(|value| value.get("bytes")).and_then(Value::as_u64);
    let sha = input.and_then(|value| value.get("sha256")).and_then(Value::as_str);

    if name != Some(reco_identity.name.as_str()) {
        errors.push("aggregate sample RECO input name does not match supplied file".to_owned());
    }
    if bytes != Some(reco_identity.bytes) {
        errors.push("aggregate sample RECO input byte size does not match supplied file".to_owned());
    }
    if sha != Some(reco_identity.sha256.as_str()) {
        errors.push("aggregate sample RECO input SHA-256 does not match supplied file".to_owned());
    }
    Ok(())
}

fn validate_submission(
    submission: &Value,
    submission_identity: &FileIdentity,
    generation_identity: &FileIdentity,
    event_count: Option<u64>,
    sample_label: &str,
    errors: &mut Vec<String>,
) -> Result<(), Vec<String>> {
    if submission.get("schema_version").and_then(Value::as_u64) != Some(1) {
        errors.push("submission schema_version must be 1".to_owned());
    }

    let input = submission.get("input");
    let input_path = input
        .and_then(|value| value.get("path"))
        .and_then(Value::as_str);
    let input_name = input_path.and_then(|path| Path::new(path).file_name()).and_then(|n| n.to_str());
    if input_name != Some(generation_identity.name.as_str()) {
        errors.push("submission generation input name does not match supplied file".to_owned());
    }
    if input.and_then(|v| v.get("bytes")).and_then(Value::as_u64) != Some(generation_identity.bytes) {
        errors.push("submission generation input byte size mismatch".to_owned());
    }
    if input.and_then(|v| v.get("sha256")).and_then(Value::as_str)
        != Some(generation_identity.sha256.as_str())
    {
        errors.push("submission generation input SHA-256 mismatch".to_owned());
    }

    if submission.get("events").and_then(Value::as_u64) != event_count {
        errors.push("submission event count does not match aggregate sample".to_owned());
    }

    let particle = submission.get("particle").and_then(Value::as_str);
    let geometry = submission.get("geometry").and_then(Value::as_str);
    let container = submission.get("container");
    let container_ref = container.and_then(|v| v.get("reference")).and_then(Value::as_str);
    let image_id = container.and_then(|v| v.get("image_id")).and_then(Value::as_str);
    let platform = container.and_then(|v| v.get("platform")).and_then(Value::as_str);
    let pdg = submission.get("pdg").and_then(Value::as_i64);

    if particle.is_none_or(str::is_empty) || geometry.is_none_or(str::is_empty) {
        errors.push("submission particle and geometry must be non-empty".to_owned());
    }
    if container_ref.is_none_or(str::is_empty)
        || image_id.is_none_or(str::is_empty)
        || platform.is_none_or(str::is_empty)
    {
        errors.push("submission container reference, image ID and platform are required".to_owned());
    }
    if pdg.is_none() {
        errors.push("submission PDG code must be an integer".to_owned());
    }

    if let Some(revisions) = submission.get("revisions") {
        if revisions.get("mucoll_benchmarks").and_then(Value::as_str).is_none_or(str::is_empty)
            || revisions.get("mucoll_spack").and_then(Value::as_str).is_none_or(str::is_empty)
        {
            errors.push("submission mucoll-benchmarks and mucoll-spack revisions are required".to_owned());
        }
    } else {
        errors.push("submission revisions object is required".to_owned());
    }

    if submission.get("input").and_then(|v| v.get("source")).and_then(Value::as_str).is_none_or(str::is_empty) {
        errors.push("submission input source URI is required".to_owned());
    }

    if sample_label != particle.unwrap_or_default() {
        // Label is often a particle label ("pion"), so reject only when both are explicit but inconsistent.
        if particle.is_none() {
            errors.push("submission particle is missing".to_owned());
        }
    }

    let _ = submission_identity;
    Ok(())
}

fn validate_report_provenance(
    report: &Value,
    submission: &Value,
    submission_identity: &FileIdentity,
    aggregator_identity: &FileIdentity,
    sample_label: &str,
    errors: &mut Vec<String>,
) -> Result<(), Vec<String>> {
    let provenance = report.get("provenance");
    let production = provenance.and_then(|v| v.get("production"));
    if production.is_none() || production == Some(&Value::Null) {
        errors.push("aggregate report has no production metadata reference".to_owned());
    } else {
        if production.and_then(|v| v.get("name")).and_then(Value::as_str)
            != Some(submission_identity.name.as_str())
        {
            errors.push("aggregate production metadata filename mismatch".to_owned());
        }
        if production.and_then(|v| v.get("sha256")).and_then(Value::as_str)
            != Some(submission_identity.sha256.as_str())
        {
            errors.push("aggregate production metadata SHA-256 mismatch".to_owned());
        }
        if production.and_then(|v| v.get("metadata")) != Some(submission) {
            errors.push("aggregate production metadata body differs from supplied submission".to_owned());
        }
    }

    let aggregator = provenance.and_then(|v| v.get("aggregator"));
    if aggregator.and_then(|v| v.get("name")).and_then(Value::as_str)
        != Some(aggregator_identity.name.as_str())
    {
        errors.push("aggregate report aggregator filename mismatch".to_owned());
    }
    if aggregator.and_then(|v| v.get("sha256")).and_then(Value::as_str)
        != Some(aggregator_identity.sha256.as_str())
    {
        errors.push("aggregate report aggregator SHA-256 mismatch".to_owned());
    }

    let benchmark = provenance.and_then(|v| v.get("benchmark_repository"));
    let benchmark_revision = benchmark.and_then(|v| v.get("revision")).and_then(Value::as_str);
    let dirty = benchmark.and_then(|v| v.get("dirty")).and_then(Value::as_bool);
    if benchmark_revision.is_none_or(str::is_empty) {
        errors.push("aggregate benchmark repository revision is missing".to_owned());
    }
    if dirty != Some(false) {
        errors.push("benchmark repository must be clean; dirty or unknown state is not admitted".to_owned());
    }

    let submission_revision = submission
        .pointer("/revisions/mucoll_benchmarks")
        .and_then(Value::as_str);
    if submission_revision != benchmark_revision {
        errors.push("submission and aggregate benchmark revisions do not match".to_owned());
    }

    let github = provenance.and_then(|v| v.get("github_actions"));
    let submission_github = submission.get("github");
    for key in ["repository", "workflow", "job", "run_id", "run_attempt", "sha", "ref"] {
        let report_value = github.and_then(|v| v.get(key)).and_then(Value::as_str);
        let submission_value = submission_github.and_then(|v| v.get(key)).and_then(Value::as_str);
        if report_value != submission_value {
            errors.push(format!("GitHub provenance field {key:?} differs between report and submission"));
        }
    }

    if let Some(spack_revision) = submission
        .pointer("/revisions/mucoll_spack")
        .and_then(Value::as_str)
    {
        let github_sha = submission
            .pointer("/github/sha")
            .and_then(Value::as_str);
        if github_sha != Some(spack_revision) {
            errors.push("submission mucoll_spack revision does not match its recorded GitHub source SHA".to_owned());
        }
    } else {
        errors.push("submission mucoll_spack revision is missing".to_owned());
    }

    if sample_label.trim().is_empty() {
        errors.push("sample label must be non-empty".to_owned());
    }

    let _ = aggregator_identity;
    Ok(())
}

fn validate_metric_values(
    value: &Value,
    path: &str,
    null_count: &mut u64,
    errors: &mut Vec<String>,
) {
    match value {
        Value::Null => *null_count += 1,
        Value::Number(number) => {
            if number.as_f64().is_some_and(|value| !value.is_finite()) {
                errors.push(format!("{path} contains a non-finite number"));
            }
        }
        Value::Array(values) => {
            for (index, value) in values.iter().enumerate() {
                validate_metric_values(value, &format!("{path}[{index}]"), null_count, errors);
            }
        }
        Value::Object(values) => {
            for (key, value) in values {
                validate_metric_values(value, &format!("{path}.{key}"), null_count, errors);
            }
        }
        Value::Bool(_) | Value::String(_) => {}
    }
}

fn read_json(path: &Path, label: &str, errors: &mut Vec<String>) -> Result<Value, Vec<String>> {
    match fs::read(path) {
        Ok(bytes) => parse_json(&bytes, label, errors).ok_or_else(|| errors.clone()),
        Err(error) => {
            errors.push(format!("cannot read {label} {}: {error}", path.display()));
            Err(errors.clone())
        }
    }
}

fn parse_json(bytes: &[u8], label: &str, errors: &mut Vec<String>) -> Option<Value> {
    match serde_json::from_slice(bytes) {
        Ok(value) => Some(value),
        Err(error) => {
            errors.push(format!("{label} is not valid JSON: {error}"));
            None
        }
    }
}

fn file_identity(path: &Path, label: &str, errors: &mut Vec<String>) -> Option<FileIdentity> {
    let bytes = read_bytes(path, label, errors)?;
    Some(FileIdentity {
        name: path.file_name()?.to_string_lossy().into_owned(),
        bytes: bytes.len() as u64,
        sha256: sha256_hex(&bytes),
    })
}

fn read_bytes(path: &Path, label: &str, errors: &mut Vec<String>) -> Option<Vec<u8>> {
    match fs::read(path) {
        Ok(bytes) => Some(bytes),
        Err(error) => {
            errors.push(format!("cannot read {label} {}: {error}", path.display()));
            None
        }
    }
}

fn confined_child(root: &Path, name: &str, errors: &mut Vec<String>) -> Option<PathBuf> {
    let relative = Path::new(name);
    let is_single_component = relative.components().count() == 1
        && matches!(relative.components().next(), Some(Component::Normal(_)));
    if !is_single_component {
        errors.push(format!("artifact name must be a simple filename, found {name:?}"));
        return None;
    }

    let canonical_root = match fs::canonicalize(root) {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!("cannot resolve artifact directory {}: {error}", root.display()));
            return None;
        }
    };
    let candidate = root.join(relative);
    let canonical_candidate = match fs::canonicalize(&candidate) {
        Ok(path) => path,
        Err(error) => {
            errors.push(format!("cannot resolve artifact {}: {error}", candidate.display()));
            return None;
        }
    };
    if !canonical_candidate.starts_with(&canonical_root) {
        errors.push(format!("artifact path escapes its declared directory: {name:?}"));
        return None;
    }
    Some(canonical_candidate)
}

fn required_string(object: &Value, key: &str, study: &str, errors: &mut Vec<String>) -> Option<String> {
    object
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .map(str::to_owned)
        .or_else(|| {
            errors.push(format!("study {study:?} {key} must be a non-empty string"));
            None
        })
}

fn identity_value(identity: &FileIdentity) -> Value {
    json!({
        "name": identity.name,
        "bytes": identity.bytes,
        "sha256": identity.sha256
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut result = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(result, "{byte:02x}");
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicU64, Ordering};

    static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

    struct Fixture {
        root: PathBuf,
        inputs: ValidationInputs,
    }

    impl Fixture {
        fn new() -> Self {
            let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
            let root = std::env::temp_dir().join(format!(
                "mucoll-detector-evidence-validator-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(root.join("fragments")).expect("fragments dir");
            fs::create_dir_all(root.join("producers")).expect("producers dir");

            fs::write(root.join("gen.edm4hep.root"), b"synthetic generation input").expect("gen");
            fs::write(root.join("reco.edm4hep.root"), b"synthetic reconstruction input").expect("reco");
            fs::write(root.join("aggregate_metrics.py"), b"# synthetic aggregator").expect("aggregator");
            fs::write(root.join("producers/study_tracks.py"), b"# synthetic study producer").expect("producer");
            fs::write(root.join("producers/study_hits.py"), b"# synthetic hit producer").expect("producer");

            let generation_identity = file_identity_test(&root.join("gen.edm4hep.root"));
            let reco = file_identity_test(&root.join("reco.edm4hep.root"));
            let aggregator = file_identity_test(&root.join("aggregate_metrics.py"));
            let producer_tracks = file_identity_test(&root.join("producers/study_tracks.py"));
            let producer_hits = file_identity_test(&root.join("producers/study_hits.py"));

            let submission = json!({
                "schema_version": 1,
                "generated_utc": "2026-11-17T00:00:00Z",
                "input": {
                    "path": "gen.edm4hep.root",
                    "source": "https://example.invalid/synthetic/gen.edm4hep.root",
                    "bytes": generation_identity.bytes,
                    "sha256": generation_identity.sha256
                },
                "events": 100,
                "particle": "pion",
                "pdg": 211,
                "geometry": "MAIA_v0",
                "container": {
                    "reference": "ghcr.io/example/mucoll-sim:test",
                    "image_id": "sha256:synthetic-image-id",
                    "platform": "linux/amd64"
                },
                "revisions": {
                    "mucoll_benchmarks": "bench-commit",
                    "mucoll_spack": "spack-commit"
                },
                "github": {
                    "repository": "example/repo",
                    "workflow": "synthetic-validation",
                    "job": "reco",
                    "event": "workflow_dispatch",
                    "run_id": "123",
                    "run_attempt": "1",
                    "sha": "spack-commit",
                    "ref": "refs/heads/main"
                }
            });
            let submission_bytes = serde_json::to_vec_pretty(&submission).expect("submission json");
            fs::write(root.join("submission.json"), &submission_bytes).expect("submission");

            let fragment_tracks = json!({
                "schema_version": 1,
                "study": "tracks",
                "input": {
                    "path": "/tmp/synthetic/reco.edm4hep.root",
                    "bytes": reco.bytes
                },
                "producer": {
                    "name": producer_tracks.name,
                    "sha256": producer_tracks.sha256
                },
                "events": {
                    "total": 100,
                    "selected": 90
                },
                "configuration": {
                    "track_collection": "SiTracks",
                    "pt_min_gev": 0.5
                },
                "metrics": {
                    "efficiency": 0.91,
                    "fake_rate": 0.02,
                    "resolution": null
                }
            });
            let fragment_hits = json!({
                "schema_version": 1,
                "study": "hits",
                "input": {
                    "path": "/tmp/synthetic/reco.edm4hep.root",
                    "bytes": reco.bytes
                },
                "producer": {
                    "name": producer_hits.name,
                    "sha256": producer_hits.sha256
                },
                "events": {
                    "total": 100,
                    "selected": 100
                },
                "configuration": {
                    "hit_collection": "VXDTrackerHits",
                    "range": [0.0, 100.0]
                },
                "metrics": {
                    "occupancy_mean": 12.0,
                    "occupancy_overflow": 0.0
                }
            });
            fs::write(
                root.join("fragments/metrics_tracks.json"),
                serde_json::to_vec_pretty(&fragment_tracks).expect("tracks json"),
            )
            .expect("write tracks");
            fs::write(
                root.join("fragments/metrics_hits.json"),
                serde_json::to_vec_pretty(&fragment_hits).expect("hits json"),
            )
            .expect("write hits");

            let track_fragment_bytes = fs::read(root.join("fragments/metrics_tracks.json")).expect("tracks bytes");
            let hit_fragment_bytes = fs::read(root.join("fragments/metrics_hits.json")).expect("hits bytes");
            let submission_hash = sha256_hex(&submission_bytes);

            let report = json!({
                "schema_version": 1,
                "generated_utc": "2026-11-17T00:00:00Z",
                "software": {
                    "mucoll_stack_version": "3.1"
                },
                "provenance": {
                    "aggregator": {
                        "name": aggregator.name,
                        "sha256": aggregator.sha256
                    },
                    "benchmark_repository": {
                        "revision": "bench-commit",
                        "dirty": false
                    },
                    "production": {
                        "name": "submission.json",
                        "sha256": submission_hash,
                        "metadata": submission
                    },
                    "github_actions": {
                        "repository": "example/repo",
                        "workflow": "synthetic-validation",
                        "job": "reco",
                        "run_id": "123",
                        "run_attempt": "1",
                        "sha": "spack-commit",
                        "ref": "refs/heads/main"
                    }
                },
                "samples": {
                    "pion": {
                        "label": "pion",
                        "event_count": 100,
                        "input": {
                            "name": "reco.edm4hep.root",
                            "bytes": reco.bytes,
                            "sha256": reco.sha256
                        },
                        "studies": {
                            "tracks": {
                                "fragment": {
                                    "name": "metrics_tracks.json",
                                    "sha256": sha256_hex(&track_fragment_bytes)
                                },
                                "producer": {
                                    "name": producer_tracks.name,
                                    "sha256": producer_tracks.sha256
                                },
                                "selected_event_count": 90,
                                "configuration": fragment_tracks["configuration"],
                                "metrics": fragment_tracks["metrics"]
                            },
                            "hits": {
                                "fragment": {
                                    "name": "metrics_hits.json",
                                    "sha256": sha256_hex(&hit_fragment_bytes)
                                },
                                "producer": {
                                    "name": producer_hits.name,
                                    "sha256": producer_hits.sha256
                                },
                                "selected_event_count": 100,
                                "configuration": fragment_hits["configuration"],
                                "metrics": fragment_hits["metrics"]
                            }
                        }
                    }
                }
            });
            fs::write(
                root.join("metrics.json"),
                serde_json::to_vec_pretty(&report).expect("report json"),
            )
            .expect("write report");

            let inputs = ValidationInputs {
                report: root.join("metrics.json"),
                reco_input: root.join("reco.edm4hep.root"),
                submission: root.join("submission.json"),
                generation_input: root.join("gen.edm4hep.root"),
                fragments_dir: root.join("fragments"),
                producer_dir: root.join("producers"),
                aggregator_source: root.join("aggregate_metrics.py"),
                sample_label: "pion".to_owned(),
            };

            Self { root, inputs }
        }
    }

    impl Drop for Fixture {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    fn file_identity_test(path: &Path) -> FileIdentity {
        let bytes = fs::read(path).expect("test file");
        FileIdentity {
            name: path.file_name().unwrap().to_string_lossy().into_owned(),
            bytes: bytes.len() as u64,
            sha256: sha256_hex(&bytes),
        }
    }

    #[test]
    fn valid_synthetic_package_is_evidence_valid_but_not_physics_qualified() {
        let fixture = Fixture::new();
        let receipt = validate_package(&fixture.inputs).expect("valid package");
        assert_eq!(receipt["evidence_package_status"], "valid");
        assert_eq!(receipt["technical_status"], "success");
        assert_eq!(receipt["physics_reference_status"], "not_evaluated");
        assert_eq!(receipt["qualification_decision"], "abstain");
        assert_eq!(receipt["unknown_metric_value_count"], 1);
        assert_eq!(receipt["studies"].as_array().unwrap().len(), 2);
    }

    #[test]
    fn negative_control_reco_input_digest_mismatch_is_rejected() {
        let fixture = Fixture::new();
        fs::write(&fixture.inputs.reco_input, b"changed reconstruction input").expect("change reco");
        assert!(validate_package(&fixture.inputs).is_err());
    }

    #[test]
    fn negative_control_fragment_digest_mismatch_is_rejected() {
        let fixture = Fixture::new();
        fs::write(
            fixture.inputs.fragments_dir.join("metrics_tracks.json"),
            b"{}",
        )
        .expect("change fragment");
        assert!(validate_package(&fixture.inputs).is_err());
    }

    #[test]
    fn negative_control_producer_checksum_mismatch_is_rejected() {
        let fixture = Fixture::new();
        fs::write(
            fixture.inputs.producer_dir.join("study_tracks.py"),
            b"# modified producer",
        )
        .expect("change producer");
        assert!(validate_package(&fixture.inputs).is_err());
    }

    #[test]
    fn negative_control_submission_generation_input_mismatch_is_rejected() {
        let fixture = Fixture::new();
        fs::write(&fixture.inputs.generation_input, b"changed generation input")
            .expect("change generation input");
        assert!(validate_package(&fixture.inputs).is_err());
    }

    #[test]
    fn negative_control_dirty_benchmark_source_is_rejected() {
        let fixture = Fixture::new();
        let report_bytes = fs::read(&fixture.inputs.report).expect("report");
        let mut report: Value = serde_json::from_slice(&report_bytes).expect("report JSON");
        report["provenance"]["benchmark_repository"]["dirty"] = json!(true);
        fs::write(
            &fixture.inputs.report,
            serde_json::to_vec_pretty(&report).expect("dirty report"),
        )
        .expect("write dirty report");
        assert!(validate_package(&fixture.inputs).is_err());
    }

    #[test]
    fn negative_control_mixed_event_counts_are_rejected() {
        let fixture = Fixture::new();
        let path = fixture.inputs.fragments_dir.join("metrics_hits.json");
        let mut fragment: Value = serde_json::from_slice(&fs::read(&path).expect("hits"))
            .expect("hits JSON");
        fragment["events"]["total"] = json!(99);
        fs::write(&path, serde_json::to_vec_pretty(&fragment).expect("hits JSON"))
            .expect("write altered hits");
        assert!(validate_package(&fixture.inputs).is_err());
    }

    #[test]
    fn negative_control_path_traversal_is_rejected() {
        let fixture = Fixture::new();
        let report_bytes = fs::read(&fixture.inputs.report).expect("report");
        let mut report: Value = serde_json::from_slice(&report_bytes).expect("report JSON");
        report["samples"]["pion"]["studies"]["tracks"]["fragment"]["name"] =
            json!("../../submission.json");
        fs::write(
            &fixture.inputs.report,
            serde_json::to_vec_pretty(&report).expect("alter report"),
        )
        .expect("write altered report");
        assert!(validate_package(&fixture.inputs).is_err());
    }

    #[test]
    fn negative_control_changed_aggregator_source_is_rejected() {
        let fixture = Fixture::new();
        fs::write(&fixture.inputs.aggregator_source, b"# modified aggregator")
            .expect("change aggregator");
        assert!(validate_package(&fixture.inputs).is_err());
    }
}
