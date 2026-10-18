//! Pinned EnergyPlus adapter-envelope and receipt gate.
//!
//! This crate does not invoke EnergyPlus and does not validate building physics.
//! It checks provenance, input/output contract completeness, declared run status,
//! and artifact integrity when an evidence root is supplied.

use jsonschema::validator_for;
use serde::Serialize;
use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fs;
use std::path::{Component, Path, PathBuf};

pub const EMBEDDED_SPEC: &str = include_str!(
    "../../../docs/research/spatial-design/ENERGYPLUS_ADAPTER_ENVELOPE_V0_1.json"
);
pub const EMBEDDED_SPEC_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/ENERGYPLUS_ADAPTER_ENVELOPE_V0_1.schema.json"
);
pub const EMBEDDED_RECEIPT_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/ENERGYPLUS_RUN_RECEIPT_V0_1.schema.json"
);
pub const EMBEDDED_FIXTURE_CORPUS: &str = include_str!(
    "../../../docs/research/spatial-design/ENERGYPLUS_RUN_RECEIPT_FIXTURES_V0_1.json"
);
pub const EMBEDDED_FIXTURE_SCHEMA: &str = include_str!(
    "../../../docs/research/spatial-design/ENERGYPLUS_RUN_RECEIPT_FIXTURES_V0_1.schema.json"
);

const EXPECTED_FIXTURE_COUNT: usize = 10;
const MAX_ARTIFACT_BYTES: u64 = 512 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct Finding {
    pub code: String,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SpecValidationSummary {
    pub benchmark_id: Option<String>,
    pub version: Option<String>,
    pub spec_sha256: String,
    pub schema_sha256: String,
    pub passed: bool,
    pub findings: Vec<Finding>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct GateResult {
    pub receipt_id: String,
    pub disposition: String,
    pub blocker_codes: Vec<String>,
    pub schema_valid: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct FixtureResult {
    pub fixture_id: String,
    pub expected_disposition: String,
    pub actual_disposition: String,
    pub expected_blocker_codes: Vec<String>,
    pub actual_blocker_codes: Vec<String>,
    pub passed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct FixtureValidationSummary {
    pub benchmark_id: Option<String>,
    pub version: Option<String>,
    pub corpus_sha256: String,
    pub fixture_schema_sha256: String,
    pub receipt_schema_sha256: String,
    pub spec_sha256: String,
    pub fixtures_seen: usize,
    pub fixtures_passed: usize,
    pub passed: bool,
    pub results: Vec<FixtureResult>,
    pub findings: Vec<Finding>,
}

impl SpecValidationSummary {
    fn new(spec_bytes: &[u8], schema_bytes: &[u8]) -> Self {
        Self {
            benchmark_id: None,
            version: None,
            spec_sha256: sha256_hex(spec_bytes),
            schema_sha256: sha256_hex(schema_bytes),
            passed: false,
            findings: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, path: impl Into<String>, detail: impl Into<String>) {
        self.findings.push(Finding {
            code: code.to_owned(),
            path: path.into(),
            detail: detail.into(),
        });
    }
}

impl FixtureValidationSummary {
    fn new(
        corpus_bytes: &[u8],
        fixture_schema_bytes: &[u8],
        receipt_schema_bytes: &[u8],
        spec_bytes: &[u8],
    ) -> Self {
        Self {
            benchmark_id: None,
            version: None,
            corpus_sha256: sha256_hex(corpus_bytes),
            fixture_schema_sha256: sha256_hex(fixture_schema_bytes),
            receipt_schema_sha256: sha256_hex(receipt_schema_bytes),
            spec_sha256: sha256_hex(spec_bytes),
            fixtures_seen: 0,
            fixtures_passed: 0,
            passed: false,
            results: Vec::new(),
            findings: Vec::new(),
        }
    }

    fn fail(&mut self, code: &str, path: impl Into<String>, detail: impl Into<String>) {
        self.findings.push(Finding {
            code: code.to_owned(),
            path: path.into(),
            detail: detail.into(),
        });
    }
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn parse_json(bytes: &[u8]) -> Result<Value, String> {
    serde_json::from_slice(bytes).map_err(|error| error.to_string())
}

fn string_field<'a>(object: &'a Map<String, Value>, key: &str) -> Option<&'a str> {
    object.get(key).and_then(Value::as_str)
}

fn nested_string<'a>(root: &'a Value, parent: &str, child: &str) -> Option<&'a str> {
    let mut current = root;
    for segment in parent.split('.') {
        current = current.get(segment)?;
    }
    current.get(child)?.as_str()
}

fn add_schema_findings(
    value: &Value,
    schema_bytes: &[u8],
    prefix: &str,
) -> Result<Vec<Finding>, String> {
    let schema: Value = parse_json(schema_bytes)
        .map_err(|error| format!("{prefix}_SCHEMA_JSON_INVALID: {error}"))?;
    let validator = validator_for(&schema)
        .map_err(|error| format!("{prefix}_SCHEMA_COMPILE_FAILED: {error}"))?;
    Ok(validator
        .iter_errors(value)
        .map(|error| Finding {
            code: format!("{prefix}_SCHEMA_VALIDATION_FAILED"),
            path: error.instance_path().to_string(),
            detail: error.to_string(),
        })
        .collect())
}

/// Validate the static adapter envelope against its schema and freeze the A0
/// release identity in executable semantics as well as machine-readable schema.
pub fn validate_spec_bytes(spec_bytes: &[u8], schema_bytes: &[u8]) -> SpecValidationSummary {
    let mut summary = SpecValidationSummary::new(spec_bytes, schema_bytes);
    let spec = match parse_json(spec_bytes) {
        Ok(value) => value,
        Err(error) => {
            summary.fail("SPEC_JSON_INVALID", "$", error);
            return summary;
        }
    };

    if let Some(object) = spec.as_object() {
        summary.benchmark_id = string_field(object, "benchmark_id").map(ToOwned::to_owned);
        summary.version = string_field(object, "version").map(ToOwned::to_owned);
    }

    match add_schema_findings(&spec, schema_bytes, "SPEC") {
        Ok(findings) => summary.findings.extend(findings),
        Err(detail) => {
            summary.fail("SPEC_SCHEMA_UNAVAILABLE", "$", detail);
            return summary;
        }
    }

    if nested_string(&spec, "engine", "version") != Some("26.2.0")
        || nested_string(&spec, "engine", "git_tag") != Some("v26.2.0")
    {
        summary.fail(
            "ENGINE_RELEASE_PIN_MISMATCH",
            "$.engine",
            "A0 is frozen to the public EnergyPlus v26.2.0 release.",
        );
    }
    if nested_string(&spec, "engine", "source_commit_sha")
        != Some("4bd7a1f26fb749a83257855aba6760e26c8a3c8f")
    {
        summary.fail(
            "ENGINE_SOURCE_PIN_MISMATCH",
            "$.engine.source_commit_sha",
            "Source commit differs from the reviewed v26.2.0 release tag.",
        );
    }
    if nested_string(&spec, "engine.package", "sha256")
        != Some("e602dbe95235e853f0f51d8e99702da4a951fdb4473ebe7f463bedd4f70532e8")
    {
        summary.fail(
            "ENGINE_PACKAGE_PIN_MISMATCH",
            "$.engine.package.sha256",
            "Package digest differs from the reviewed Ubuntu 24.04 x86_64 release asset.",
        );
    }

    summary.findings.sort_by(|left, right| {
        (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
    });
    summary.passed = summary.findings.is_empty();
    summary
}

fn normalized_codes(values: impl IntoIterator<Item = String>) -> Vec<String> {
    let mut codes: Vec<String> = values.into_iter().collect();
    codes.sort();
    codes.dedup();
    codes
}

fn gate_for_codes(codes: &[String]) -> &'static str {
    if codes.iter().any(|code| {
        matches!(
            code.as_str(),
            "RUN_NOT_COMPLETED"
                | "PROCESS_EXIT_NONZERO"
                | "SOLVER_NOT_CONVERGED"
                | "SOLVER_SEVERE_ERRORS"
                | "SOLVER_FATAL_ERRORS"
                | "PARTIAL_OUTPUTS"
        )
    }) {
        "blocked"
    } else if codes.is_empty() {
        "eligible_for_reference_comparison"
    } else {
        "unknown"
    }
}

fn code_suffix(value: &str) -> String {
    let mut suffix = String::new();
    let mut underscore_pending = false;
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            if underscore_pending && !suffix.is_empty() {
                suffix.push('_');
            }
            suffix.push(character.to_ascii_uppercase());
            underscore_pending = false;
        } else {
            underscore_pending = true;
        }
    }
    suffix
}

fn evaluate_receipt_metadata(receipt: &Value, spec: &Value, extra_blockers: &[String]) -> GateResult {
    let mut codes: BTreeSet<String> = extra_blockers.iter().cloned().collect();
    let receipt_id = receipt
        .get("receipt_id")
        .and_then(Value::as_str)
        .unwrap_or("unknown-receipt")
        .to_owned();

    for (field, expected, code) in [
        ("name", Some("EnergyPlus"), "ENGINE_NAME_MISMATCH"),
        ("version", nested_string(&spec, "engine", "version"), "ENGINE_VERSION_MISMATCH"),
        ("git_tag", nested_string(&spec, "engine", "git_tag"), "ENGINE_TAG_MISMATCH"),
        (
            "source_commit_sha",
            nested_string(&spec, "engine", "source_commit_sha"),
            "ENGINE_SOURCE_COMMIT_MISMATCH",
        ),
        (
            "package_sha256",
            nested_string(&spec, "engine.package", "sha256"),
            "ENGINE_PACKAGE_DIGEST_MISMATCH",
        ),
        (
            "platform",
            nested_string(&spec, "engine.package", "platform"),
            "ENGINE_PLATFORM_MISMATCH",
        ),
    ] {
        if nested_string(receipt, "engine", field) != expected {
            codes.insert(code.to_owned());
        }
    }

    if nested_string(&receipt, "input", "format") != Some("epJSON") {
        codes.insert("INPUT_FORMAT_MISMATCH".to_owned());
    }
    if nested_string(&receipt, "input", "schema_revision")
        != nested_string(&spec, "input_schema", "revision")
    {
        codes.insert("INPUT_SCHEMA_REVISION_MISMATCH".to_owned());
    }
    if nested_string(&receipt, "input", "unit_system")
        != nested_string(&spec, "input_schema", "unit_system")
    {
        codes.insert("INPUT_UNIT_SYSTEM_MISMATCH".to_owned());
    }

    if let Some(input) = receipt.get("input").and_then(Value::as_object) {
        let timestep = input.get("timestep_minutes").and_then(Value::as_u64).unwrap_or(0);
        if ![1, 2, 3, 4, 5, 6, 10, 12, 15, 20, 30, 60].contains(&timestep) {
            codes.insert("TIMESTEP_NOT_DIVISOR_OF_HOUR".to_owned());
        }
        let start = string_field(input, "run_start_date").unwrap_or("");
        let end = string_field(input, "run_end_date").unwrap_or("");
        if start > end {
            codes.insert("RUN_DATE_RANGE_INVALID".to_owned());
        }
        for (field, code) in [
            ("model_sha256", "MODEL_DIGEST_MISSING"),
            ("geometry_sha256", "GEOMETRY_DIGEST_MISSING"),
            ("weather_sha256", "WEATHER_DIGEST_MISSING"),
            ("schedule_sha256", "SCHEDULE_DIGEST_MISSING"),
            ("configuration_sha256", "CONFIGURATION_DIGEST_MISSING"),
            ("schema_sha256", "INPUT_SCHEMA_DIGEST_MISSING"),
        ] {
            if string_field(input, field).is_none() {
                codes.insert(code.to_owned());
            }
        }
        if string_field(input, "authoritative_geometry_revision").is_none() {
            codes.insert("GEOMETRY_REVISION_MISSING".to_owned());
        }
    }

    let execution = receipt.get("execution").and_then(Value::as_object);
    if let Some(execution) = execution {
        let status = string_field(execution, "run_status").unwrap_or("unknown");
        let exit_code = execution.get("exit_code").and_then(Value::as_i64);
        let converged = execution.get("converged").and_then(Value::as_bool);

        if status != "completed" {
            codes.insert("RUN_NOT_COMPLETED".to_owned());
        }
        if status == "partial" || execution.get("partial_outputs").and_then(Value::as_bool) == Some(true) {
            codes.insert("PARTIAL_OUTPUTS".to_owned());
        }
        match exit_code {
            Some(0) => {}
            Some(_) => {
                codes.insert("PROCESS_EXIT_NONZERO".to_owned());
            }
            None => {
                codes.insert("PROCESS_EXIT_UNKNOWN".to_owned());
            }
        }
        match converged {
            Some(true) => {}
            Some(false) => {
                codes.insert("SOLVER_NOT_CONVERGED".to_owned());
            }
            None => {
                codes.insert("SOLVER_CONVERGENCE_UNKNOWN".to_owned());
            }
        }
        if execution.get("severe_error_count").and_then(Value::as_u64).unwrap_or(0) > 0 {
            codes.insert("SOLVER_SEVERE_ERRORS".to_owned());
        }
        if execution.get("fatal_error_count").and_then(Value::as_u64).unwrap_or(0) > 0 {
            codes.insert("SOLVER_FATAL_ERRORS".to_owned());
        }
        if execution.get("run_log_sha256").and_then(Value::as_str).is_none() {
            codes.insert("RUN_LOG_DIGEST_MISSING".to_owned());
        }
    } else {
        codes.insert("EXECUTION_STATUS_MISSING".to_owned());
    }

    let outputs = receipt.get("outputs").and_then(Value::as_array);
    if let Some(outputs) = outputs {
        let mut seen = BTreeSet::new();
        for output in outputs {
            let name = output.get("name").and_then(Value::as_str).unwrap_or("");
            if !seen.insert(name.to_owned()) {
                codes.insert(format!("OUTPUT_DUPLICATE:{}", code_suffix(name)));
            }
            if output.get("sample_count").and_then(Value::as_u64).unwrap_or(0) == 0 {
                codes.insert(format!("OUTPUT_SAMPLE_COUNT_ZERO:{}", code_suffix(name)));
            }
            if output.get("artifact_sha256").and_then(Value::as_str).is_none() {
                codes.insert(format!("OUTPUT_DIGEST_MISSING:{}", code_suffix(name)));
            }
        }

        if let Some(required_outputs) = spec.get("required_outputs").and_then(Value::as_array) {
            for required in required_outputs {
                let name = required.get("name").and_then(Value::as_str).unwrap_or("");
                let matching: Vec<&Value> = outputs
                    .iter()
                    .filter(|output| output.get("name").and_then(Value::as_str) == Some(name))
                    .collect();
                if matching.is_empty() {
                    codes.insert(format!("REQUIRED_OUTPUT_MISSING:{}", code_suffix(name)));
                    continue;
                }
                let output = matching[0];
                for (field, mismatch_code) in [
                    ("kind", "OUTPUT_KIND_MISMATCH"),
                    ("units", "OUTPUT_UNITS_MISMATCH"),
                    ("report_frequency", "OUTPUT_REPORT_FREQUENCY_MISMATCH"),
                ] {
                    if output.get(field) != required.get(field) {
                        codes.insert(format!("{mismatch_code}:{}", code_suffix(name)));
                    }
                }
            }
        }
    } else {
        codes.insert("OUTPUTS_MISSING".to_owned());
    }

    let base_codes = normalized_codes(codes.into_iter());
    let base_gate = gate_for_codes(&base_codes);
    let claimed_gate = receipt
        .get("claimed_gate")
        .and_then(Value::as_str)
        .unwrap_or("unknown");
    let claimed_codes = receipt
        .get("claimed_blocker_codes")
        .and_then(Value::as_array)
        .map(|values| {
            values
                .iter()
                .filter_map(Value::as_str)
                .map(ToOwned::to_owned)
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let claim_gate_mismatch = claimed_gate != base_gate;
    let claim_codes_mismatch = normalized_codes(claimed_codes) != base_codes;

    let mut final_codes = base_codes;
    if claim_codes_mismatch {
        final_codes.push("CLAIMED_BLOCKER_CODES_MISMATCH".to_owned());
    }
    if claim_gate_mismatch {
        final_codes.push("CLAIMED_GATE_MISMATCH".to_owned());
    }
    let final_codes = normalized_codes(final_codes);
    let disposition = gate_for_codes(&final_codes);
    GateResult {
        receipt_id,
        disposition: disposition.to_owned(),
        blocker_codes: final_codes,
        schema_valid: true,
    }

fn relative_path_is_safe(path: &str) -> bool {
    let parsed = Path::new(path);
    !path.is_empty()
        && !parsed.is_absolute()
        && !path.contains('\\')
        && !parsed.components().any(|component| {
            matches!(
                component,
                Component::ParentDir | Component::RootDir | Component::Prefix(_)
            )
        })
}

fn verify_artifact(
    root: &Path,
    relative_path: &str,
    expected_digest: &str,
    label: &str,
) -> Option<String> {
    if !relative_path_is_safe(relative_path) {
        return Some(format!("ARTIFACT_PATH_UNSAFE:{}", code_suffix(label)));
    }
    let root = match root.canonicalize() {
        Ok(root) => root,
        Err(_) => return Some("EVIDENCE_ROOT_UNAVAILABLE".to_owned()),
    };
    let joined: PathBuf = root.join(relative_path);
    let canonical = match joined.canonicalize() {
        Ok(path) => path,
        Err(_) => return Some(format!("ARTIFACT_MISSING:{}", code_suffix(label))),
    };
    if !canonical.starts_with(&root) {
        return Some(format!("ARTIFACT_PATH_UNSAFE:{}", code_suffix(label)));
    }
    let metadata = match fs::metadata(&canonical) {
        Ok(metadata) => metadata,
        Err(_) => return Some(format!("ARTIFACT_MISSING:{}", code_suffix(label))),
    };
    if !metadata.is_file() {
        return Some(format!("ARTIFACT_NOT_FILE:{}", code_suffix(label)));
    }
    if metadata.len() > MAX_ARTIFACT_BYTES {
        return Some(format!("ARTIFACT_TOO_LARGE:{}", code_suffix(label)));
    }
    let bytes = match fs::read(&canonical) {
        Ok(bytes) => bytes,
        Err(_) => return Some(format!("ARTIFACT_READ_FAILED:{}", code_suffix(label))),
    };
    if sha256_hex(&bytes) != expected_digest {
        return Some(format!("ARTIFACT_DIGEST_MISMATCH:{}", code_suffix(label)));
    }
    None
}

fn verify_receipt_artifacts(receipt: &Value, root: &Path) -> Vec<String> {
    let mut codes = BTreeSet::new();
    let Some(input) = receipt.get("input").and_then(Value::as_object) else {
        return vec!["RECEIPT_INPUT_MISSING".to_owned()];
    };

    for (path_field, digest_field, label) in [
        ("schema_artifact_path", "schema_sha256", "input_schema"),
        ("model_artifact_path", "model_sha256", "model"),
        ("geometry_artifact_path", "geometry_sha256", "geometry_manifest"),
        ("weather_artifact_path", "weather_sha256", "weather"),
        ("schedule_artifact_path", "schedule_sha256", "schedules"),
        ("configuration_artifact_path", "configuration_sha256", "run_configuration"),
    ] {
        let Some(path) = string_field(input, path_field) else {
            codes.insert(format!("ARTIFACT_REFERENCE_MISSING:{}", code_suffix(label)));
            continue;
        };
        let Some(digest) = string_field(input, digest_field) else {
            codes.insert(format!("ARTIFACT_REFERENCE_MISSING:{}", code_suffix(label)));
            continue;
        };
        if let Some(code) = verify_artifact(root, path, digest, label) {
            codes.insert(code);
        }
    }

    if let Some(execution) = receipt.get("execution").and_then(Value::as_object) {
        match (
            execution.get("run_log_artifact_path").and_then(Value::as_str),
            execution.get("run_log_sha256").and_then(Value::as_str),
        ) {
            (Some(path), Some(digest)) => {
                if let Some(code) = verify_artifact(root, path, digest, "run_log") {
                    codes.insert(code);
                }
            }
            _ => {
                codes.insert("ARTIFACT_REFERENCE_MISSING:RUN_LOG".to_owned());
            }
        }
    }

    if let Some(outputs) = receipt.get("outputs").and_then(Value::as_array) {
        for output in outputs {
            let name = output.get("name").and_then(Value::as_str).unwrap_or("unnamed_output");
            match (
                output.get("artifact_path").and_then(Value::as_str),
                output.get("artifact_sha256").and_then(Value::as_str),
            ) {
                (Some(path), Some(digest)) => {
                    if let Some(code) = verify_artifact(root, path, digest, name) {
                        codes.insert(code);
                    }
                }
                _ => {
                    codes.insert(format!("ARTIFACT_REFERENCE_MISSING:{}", code_suffix(name)));
                }
            }
        }
    }
    codes.into_iter().collect()
}

pub fn validate_receipt_bytes(
    receipt_bytes: &[u8],
    spec_bytes: &[u8],
    receipt_schema_bytes: &[u8],
    artifact_root: &Path,
) -> GateResult {
    let receipt = match parse_json(receipt_bytes) {
        Ok(receipt) => receipt,
        Err(_) => {
            return GateResult {
                receipt_id: "invalid-receipt".to_owned(),
                disposition: "unknown".to_owned(),
                blocker_codes: vec!["RECEIPT_JSON_INVALID".to_owned()],
                schema_valid: false,
            };
        }
    };
    let schema_errors = match add_schema_findings(&receipt, receipt_schema_bytes, "RECEIPT") {
        Ok(findings) => findings,
        Err(_) => {
            return GateResult {
                receipt_id: receipt.get("receipt_id").and_then(Value::as_str).unwrap_or("unknown").to_owned(),
                disposition: "unknown".to_owned(),
                blocker_codes: vec!["RECEIPT_SCHEMA_UNAVAILABLE".to_owned()],
                schema_valid: false,
            };
        }
    };
    if !schema_errors.is_empty() {
        return GateResult {
            receipt_id: receipt.get("receipt_id").and_then(Value::as_str).unwrap_or("unknown").to_owned(),
            disposition: "unknown".to_owned(),
            blocker_codes: vec!["RECEIPT_SCHEMA_INVALID".to_owned()],
            schema_valid: false,
        };
    }
    let spec = match parse_json(spec_bytes) {
        Ok(spec) => spec,
        Err(_) => {
            return GateResult {
                receipt_id: receipt.get("receipt_id").and_then(Value::as_str).unwrap_or("unknown").to_owned(),
                disposition: "unknown".to_owned(),
                blocker_codes: vec!["ADAPTER_SPEC_INVALID".to_owned()],
                schema_valid: true,
            };
        }
    };
    let artifact_codes = verify_receipt_artifacts(&receipt, artifact_root);
    evaluate_receipt_metadata(&receipt, &spec, &artifact_codes)
}

pub fn validate_fixture_corpus_bytes(
    corpus_bytes: &[u8],
    fixture_schema_bytes: &[u8],
    receipt_schema_bytes: &[u8],
    spec_bytes: &[u8],
    spec_schema_bytes: &[u8],
) -> FixtureValidationSummary {
    let mut summary = FixtureValidationSummary::new(
        corpus_bytes,
        fixture_schema_bytes,
        receipt_schema_bytes,
        spec_bytes,
    );
    let spec_result = validate_spec_bytes(spec_bytes, spec_schema_bytes);
    if !spec_result.passed {
        summary.fail(
            "ADAPTER_SPEC_INVALID",
            "$",
            format!("Embedded adapter spec has {} finding(s).", spec_result.findings.len()),
        );
        summary.passed = false;
        return summary;
    }

    let corpus = match parse_json(corpus_bytes) {
        Ok(corpus) => corpus,
        Err(error) => {
            summary.fail("FIXTURE_CORPUS_JSON_INVALID", "$", error);
            return summary;
        }
    };
    match add_schema_findings(&corpus, fixture_schema_bytes, "FIXTURE_CORPUS") {
        Ok(findings) => summary.findings.extend(findings),
        Err(detail) => {
            summary.fail("FIXTURE_SCHEMA_UNAVAILABLE", "$", detail);
            return summary;
        }
    }
    if !summary.findings.is_empty() {
        return summary;
    }

    if let Some(root) = corpus.as_object() {
        summary.benchmark_id = string_field(root, "benchmark_id").map(ToOwned::to_owned);
        summary.version = string_field(root, "version").map(ToOwned::to_owned);
    }
    let fixtures = match corpus.get("fixtures").and_then(Value::as_array) {
        Some(fixtures) => fixtures,
        None => {
            summary.fail("FIXTURE_LIST_MISSING", "$.fixtures", "Fixture list is required.");
            return summary;
        }
    };
    summary.fixtures_seen = fixtures.len();

    let mut ids = BTreeSet::new();
    for (index, fixture) in fixtures.iter().enumerate() {
        let path = format!("$.fixtures[{index}]");
        let fixture_object = match fixture.as_object() {
            Some(object) => object,
            None => {
                summary.fail("FIXTURE_INVALID", &path, "Fixture must be an object.");
                continue;
            }
        };
        let id = string_field(fixture_object, "fixture_id").unwrap_or("");
        let expected_id = format!("EPLUS-F-{:03}", index + 1);
        if id != expected_id || !ids.insert(id.to_owned()) {
            summary.fail(
                "FIXTURE_ID_ORDER_OR_DUPLICATE",
                format!("{path}.fixture_id"),
                format!("Expected unique sequential ID {expected_id:?}, got {id:?}."),
            );
        }
        let receipt = match fixture_object.get("receipt") {
            Some(receipt) => receipt,
            None => {
                summary.fail("FIXTURE_RECEIPT_MISSING", format!("{path}.receipt"), "Receipt is required.");
                continue;
            }
        };
        let receipt_bytes = match serde_json::to_vec(receipt) {
            Ok(bytes) => bytes,
            Err(error) => {
                summary.fail("FIXTURE_RECEIPT_SERIALIZE_FAILED", format!("{path}.receipt"), error.to_string());
                continue;
            }
        };
        match add_schema_findings(&receipt, receipt_schema_bytes, "RECEIPT") {
            Ok(findings) if findings.is_empty() => {}
            Ok(findings) => {
                for finding in findings {
                    summary.fail(
                        "FIXTURE_RECEIPT_SCHEMA_INVALID",
                        format!("{path}.receipt{}", finding.path),
                        finding.detail,
                    );
                }
                continue;
            }
            Err(detail) => {
                summary.fail(
                    "FIXTURE_RECEIPT_SCHEMA_UNAVAILABLE",
                    format!("{path}.receipt"),
                    detail,
                );
                continue;
            }
        }
        let spec_value: Value = match parse_json(spec_bytes) {
            Ok(spec_value) => spec_value,
            Err(error) => {
                summary.fail("ADAPTER_SPEC_INVALID", "$", error);
                continue;
            }
        };
        let result = evaluate_receipt_metadata(&receipt, &spec_value, &[]);
        let expected_gate = string_field(fixture_object, "expected_gate").unwrap_or("unknown");
        let expected_codes = fixture_object
            .get("expected_blocker_codes")
            .and_then(Value::as_array)
            .map(|values| values.iter().filter_map(Value::as_str).map(ToOwned::to_owned).collect::<Vec<_>>())
            .unwrap_or_default();
        let expected_codes = normalized_codes(expected_codes);
        let passed = result.schema_valid
            && result.disposition == expected_gate
            && result.blocker_codes == expected_codes;
        if !passed {
            summary.fail(
                "FIXTURE_EXPECTATION_MISMATCH",
                &path,
                format!(
                    "Expected gate {:?} / codes {:?}, computed {:?} / {:?}.",
                    expected_gate, expected_codes, result.disposition, result.blocker_codes
                ),
            );
        } else {
            summary.fixtures_passed += 1;
        }
        summary.results.push(FixtureResult {
            fixture_id: id.to_owned(),
            expected_disposition: expected_gate.to_owned(),
            actual_disposition: result.disposition,
            expected_blocker_codes: expected_codes,
            actual_blocker_codes: result.blocker_codes,
            passed,
        });
    }

    summary.results.sort_by(|left, right| left.fixture_id.cmp(&right.fixture_id));
    summary.findings.sort_by(|left, right| {
        (&left.path, &left.code, &left.detail).cmp(&(&right.path, &right.code, &right.detail))
    });
    summary.passed =
        summary.findings.is_empty() && summary.fixtures_passed == summary.fixtures_seen;
    summary
}

#[cfg(test)]
mod tests {
    use super::*;

    fn has_finding(summary: &SpecValidationSummary, code: &str) -> bool {
        summary.findings.iter().any(|finding| finding.code == code)
    }

    #[test]
    fn pinned_adapter_envelope_passes_schema_and_semantics() {
        let summary = validate_spec_bytes(EMBEDDED_SPEC.as_bytes(), EMBEDDED_SPEC_SCHEMA.as_bytes());
        assert!(summary.passed, "unexpected findings: {:?}", summary.findings);
        assert_eq!(summary.benchmark_id.as_deref(), Some("ENERGYPLUS-ADAPTER-ENVELOPE"));
        assert_eq!(summary.version.as_deref(), Some("0.1.0"));
        assert_eq!(summary.spec_sha256.len(), 64);
    }

    #[test]
    fn spec_schema_rejects_an_unreviewed_engine_version() {
        let mut spec: Value = serde_json::from_str(EMBEDDED_SPEC).expect("valid spec");
        spec["engine"]["version"] = Value::String("latest".to_owned());
        let bytes = serde_json::to_vec(&spec).expect("serialize mutation");
        let summary = validate_spec_bytes(&bytes, EMBEDDED_SPEC_SCHEMA.as_bytes());
        assert!(!summary.passed);
    }

    #[test]
    fn all_ten_receipt_fixtures_match_the_declared_gate() {
        let summary = validate_fixture_corpus_bytes(
            EMBEDDED_FIXTURE_CORPUS.as_bytes(),
            EMBEDDED_FIXTURE_SCHEMA.as_bytes(),
            EMBEDDED_RECEIPT_SCHEMA.as_bytes(),
            EMBEDDED_SPEC.as_bytes(),
            EMBEDDED_SPEC_SCHEMA.as_bytes(),
        );
        assert!(summary.passed, "unexpected findings: {:?}", summary.findings);
        assert_eq!(summary.fixtures_seen, EXPECTED_FIXTURE_COUNT);
        assert_eq!(summary.fixtures_passed, EXPECTED_FIXTURE_COUNT);
    }

    #[test]
    fn non_convergence_is_blocked_and_not_reported_as_zero() {
        let corpus: Value = serde_json::from_str(EMBEDDED_FIXTURE_CORPUS).unwrap();
        let mut receipt = corpus["fixtures"][0]["receipt"].clone();
        let spec: Value = serde_json::from_str(EMBEDDED_SPEC).unwrap();
        receipt["execution"]["converged"] = Value::Bool(false);
        receipt["claimed_gate"] = Value::String("blocked".to_owned());
        receipt["claimed_blocker_codes"] = serde_json::json!(["SOLVER_NOT_CONVERGED"]);
        let result = evaluate_receipt_metadata(&receipt, &spec, &[]);
        assert_eq!(result.disposition, "blocked");
        assert_eq!(result.blocker_codes, vec!["SOLVER_NOT_CONVERGED"]);
    }

    fn evaluate_fixture(index: usize) -> GateResult {
        let corpus: Value = serde_json::from_str(EMBEDDED_FIXTURE_CORPUS).unwrap();
        let spec: Value = serde_json::from_str(EMBEDDED_SPEC).unwrap();
        evaluate_receipt_metadata(&corpus["fixtures"][index]["receipt"], &spec, &[])
    }

    #[test]
    fn mismatched_package_digest_remains_unknown() {
        let result = evaluate_fixture(1);
        assert_eq!(result.disposition, "unknown");
        assert_eq!(result.blocker_codes, vec!["ENGINE_PACKAGE_DIGEST_MISMATCH"]);
    }

    #[test]
    fn incomplete_output_bundle_remains_unknown() {
        let result = evaluate_fixture(6);
        assert_eq!(result.disposition, "unknown");
        assert_eq!(
            result.blocker_codes,
            vec!["REQUIRED_OUTPUT_MISSING:ZONE_AIR_TEMPERATURE"]
        );
    }

    #[test]
    fn a_claimed_gate_cannot_override_computed_evidence() {
        let result = evaluate_fixture(9);
        assert_eq!(result.disposition, "unknown");
        assert_eq!(
            result.blocker_codes,
            vec![
                "CLAIMED_BLOCKER_CODES_MISMATCH".to_owned(),
                "CLAIMED_GATE_MISMATCH".to_owned()
            ]
        );
    }

    #[test]
    fn unsafe_relative_artifact_paths_are_rejected() {
        assert!(!relative_path_is_safe("../outside.txt"));
        assert!(!relative_path_is_safe("/absolute/path"));
        assert!(!relative_path_is_safe("folder/../../secret"));
        assert!(relative_path_is_safe("inputs/model.epJSON"));
    }

    fn write_hashed_artifact(root: &Path, relative: &str) -> String {
        let path = root.join(relative);
        fs::create_dir_all(path.parent().expect("artifact parent")).unwrap();
        let bytes = format!("synthetic artifact fixture: {relative}");
        fs::write(&path, bytes.as_bytes()).unwrap();
        sha256_hex(bytes.as_bytes())
    }

    #[test]
    fn artifact_verifier_checks_exact_bytes_and_rejects_path_escape() {
        let root = std::env::temp_dir().join(format!(
            "energyplus-qualifier-artifacts-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();
        let digest = write_hashed_artifact(&root, "inputs/model.epJSON");
        assert_eq!(
            verify_artifact(&root, "inputs/model.epJSON", &digest, "model"),
            None
        );
        assert_eq!(
            verify_artifact(&root, "inputs/model.epJSON", &sha256_hex(b"wrong"), "model"),
            Some("ARTIFACT_DIGEST_MISMATCH:MODEL".to_owned())
        );
        assert_eq!(
            verify_artifact(&root, "../outside", &digest, "model"),
            Some("ARTIFACT_PATH_UNSAFE:MODEL".to_owned())
        );
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn external_receipt_is_eligible_only_when_all_referenced_artifacts_hash_match() {
        let root = std::env::temp_dir().join(format!(
            "energyplus-qualifier-valid-receipt-{}",
            std::process::id()
        ));
        let _ = fs::remove_dir_all(&root);
        fs::create_dir_all(&root).unwrap();

        let corpus: Value = serde_json::from_str(EMBEDDED_FIXTURE_CORPUS).unwrap();
        let mut receipt = corpus["fixtures"][0]["receipt"].clone();
        let input_paths = [
            ("schema_artifact_path", "schema_sha256"),
            ("model_artifact_path", "model_sha256"),
            ("geometry_artifact_path", "geometry_sha256"),
            ("weather_artifact_path", "weather_sha256"),
            ("schedule_artifact_path", "schedule_sha256"),
            ("configuration_artifact_path", "configuration_sha256"),
        ];
        for (path_field, digest_field) in input_paths {
            let path = receipt["input"][path_field]
                .as_str()
                .expect("input artifact path");
            let digest = write_hashed_artifact(&root, path);
            receipt["input"][digest_field] = Value::String(digest);
        }
        let log_path = receipt["execution"]["run_log_artifact_path"]
            .as_str()
            .expect("run log path");
        let log_digest = write_hashed_artifact(&root, log_path);
        receipt["execution"]["run_log_sha256"] = Value::String(log_digest);
        for output in receipt["outputs"].as_array_mut().unwrap() {
            let path = output["artifact_path"].as_str().expect("output path");
            let digest = write_hashed_artifact(&root, path);
            output["artifact_sha256"] = Value::String(digest);
        }

        let receipt_bytes = serde_json::to_vec(&receipt).unwrap();
        let eligible = validate_receipt_bytes(
            &receipt_bytes,
            EMBEDDED_SPEC.as_bytes(),
            EMBEDDED_RECEIPT_SCHEMA.as_bytes(),
            &root,
        );
        assert_eq!(eligible.disposition, "eligible_for_reference_comparison");
        assert!(eligible.blocker_codes.is_empty());

        let changed_output = root.join("outputs/zone-air-temperature.csv");
        fs::write(&changed_output, b"tampered after hashing").unwrap();
        let tampered = validate_receipt_bytes(
            &receipt_bytes,
            EMBEDDED_SPEC.as_bytes(),
            EMBEDDED_RECEIPT_SCHEMA.as_bytes(),
            &root,
        );
        assert_eq!(tampered.disposition, "unknown");
        assert!(tampered
            .blocker_codes
            .contains(&"ARTIFACT_DIGEST_MISMATCH:ZONE_AIR_TEMPERATURE".to_owned()));
        let _ = fs::remove_dir_all(&root);
    }

    #[test]
    fn sha256_receipt_hash_is_deterministic() {
        assert_eq!(
            sha256_hex(b"EnergyPlus A0 receipt fixture"),
            sha256_hex(b"EnergyPlus A0 receipt fixture")
        );
        assert_eq!(sha256_hex(b"EnergyPlus A0 receipt fixture").len(), 64);
    }
}
