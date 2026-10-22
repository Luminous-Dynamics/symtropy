use serde_json::{Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};

const MANIFEST_VERSION: &str = "shipyard-benchmark-manifest/v0";
const REPORT_VERSION: &str = "shipyard-benchmark-report/v0";

const MANIFEST_SCHEMA: &str =
    include_str!("../../shipyard-benchmark-contract/benchmark-manifest.schema.json");
const REPORT_SCHEMA: &str =
    include_str!("../../shipyard-benchmark-contract/run-report.schema.json");

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AuditSummary {
    pub case_count: usize,
    pub verified_feasible_count: usize,
    pub verified_infeasible_count: usize,
    pub unknown_count: usize,
    pub timeout_count: usize,
    pub not_run_count: usize,
    pub invalid_output_count: usize,
    pub false_feasible_count: usize,
    pub false_infeasible_count: usize,
}

pub fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut out = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

/// Check internal schema structure (JSON syntax, Draft 2020-12 identifier,
/// required/property consistency). This is intentionally not a general-purpose
/// JSON Schema implementation; semantic validation below enforces the fields
/// and cross-field invariants used by this contract.
pub fn validate_embedded_schemas() -> Result<(), Vec<String>> {
    let mut errors = Vec::new();
    for (name, source, required_fields) in [
        (
            "benchmark manifest schema",
            MANIFEST_SCHEMA,
            &[
                "schema_version",
                "benchmark_id",
                "scope",
                "source",
                "protocol",
                "candidate",
                "baselines",
                "oracle",
                "metrics",
                "metric_definitions",
            ][..],
        ),
        (
            "run report schema",
            REPORT_SCHEMA,
            &[
                "schema_version",
                "benchmark_id",
                "manifest_sha256",
                "run_metadata",
                "case_results",
                "summary",
            ][..],
        ),
    ] {
        let schema: Value = match serde_json::from_str(source) {
            Ok(value) => value,
            Err(error) => {
                errors.push(format!("{name}: invalid JSON: {error}"));
                continue;
            }
        };
        if schema.get("$schema").and_then(Value::as_str)
            != Some("https://json-schema.org/draft/2020-12/schema")
        {
            errors.push(format!("{name}: wrong or missing Draft 2020-12 identifier"));
        }
        let properties = schema.get("properties").and_then(Value::as_object);
        let required = schema.get("required").and_then(Value::as_array);
        let (Some(properties), Some(required)) = (properties, required) else {
            errors.push(format!("{name}: root must declare object properties and required"));
            continue;
        };
        let required: BTreeSet<&str> = required.iter().filter_map(Value::as_str).collect();
        for field in required_fields {
            if !properties.contains_key(*field) || !required.contains(*field) {
                errors.push(format!("{name}: required field {field} lacks a property or requirement"));
            }
        }
        for field in required {
            if !properties.contains_key(field) {
                errors.push(format!("{name}: required field {field} has no property schema"));
            }
        }
        if name == "run report schema" {
            let case_required = schema
                .pointer("/properties/case_results/items/required")
                .and_then(Value::as_array);
            if !case_required.is_some_and(|fields| {
                fields.iter().any(|field| field.as_str() == Some("reference_outcome"))
            }) {
                errors.push(format!("{name}: case schema must require reference_outcome"));
            }
        }
    }
    errors.sort();
    errors.dedup();
    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

pub fn audit_bytes(manifest_bytes: &[u8], report_bytes: &[u8]) -> Result<AuditSummary, Vec<String>> {
    let mut errors = validate_embedded_schemas().err().unwrap_or_default();
    let manifest: Value = match serde_json::from_slice(manifest_bytes) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("manifest: invalid JSON: {error}"));
            return Err(sorted(errors));
        }
    };
    let report: Value = match serde_json::from_slice(report_bytes) {
        Ok(value) => value,
        Err(error) => {
            errors.push(format!("report: invalid JSON: {error}"));
            return Err(sorted(errors));
        }
    };

    validate_manifest(&manifest, &mut errors);
    let summary = validate_report(&manifest, &report, manifest_bytes, &mut errors);
    if errors.is_empty() {
        Ok(summary)
    } else {
        Err(sorted(errors))
    }
}

fn sorted(mut errors: Vec<String>) -> Vec<String> {
    errors.sort();
    errors.dedup();
    errors
}

fn object<'a>(
    value: &'a Value,
    path: &str,
    allowed: &[&str],
    required: &[&str],
    errors: &mut Vec<String>,
) -> Option<&'a Map<String, Value>> {
    let Some(map) = value.as_object() else {
        errors.push(format!("{path}: expected object"));
        return None;
    };
    for key in map.keys() {
        if !allowed.contains(&key.as_str()) {
            errors.push(format!("{path}.{key}: unknown field"));
        }
    }
    for key in required {
        if !map.contains_key(*key) {
            errors.push(format!("{path}.{key}: required field missing"));
        }
    }
    Some(map)
}

fn nonempty_string(value: Option<&Value>, path: &str, errors: &mut Vec<String>) -> Option<String> {
    match value.and_then(Value::as_str) {
        Some(value) if !value.trim().is_empty() => Some(value.to_owned()),
        _ => {
            errors.push(format!("{path}: expected non-empty string"));
            None
        }
    }
}

fn one_of(value: Option<&Value>, path: &str, choices: &[&str], errors: &mut Vec<String>) {
    match value.and_then(Value::as_str) {
        Some(value) if choices.contains(&value) => {}
        _ => errors.push(format!("{path}: expected one of {}", choices.join(", "))),
    }
}

fn hash(value: Option<&Value>, path: &str, lengths: &[usize], errors: &mut Vec<String>) {
    match value.and_then(Value::as_str) {
        Some(value)
            if lengths.contains(&value.len())
                && value.bytes().all(|byte| byte.is_ascii_hexdigit()) => {}
        _ => errors.push(format!("{path}: expected hexadecimal digest/revision")),
    }
}

fn positive_integer(value: Option<&Value>, path: &str, allow_zero: bool, errors: &mut Vec<String>) {
    match value.and_then(Value::as_i64) {
        Some(value) if if allow_zero { value >= 0 } else { value > 0 } => {}
        _ => errors.push(format!("{path}: expected {}integer", if allow_zero { "non-negative " } else { "positive " })),
    }
}

fn strings_array<'a>(
    value: Option<&'a Value>,
    path: &str,
    allow_empty: bool,
    errors: &mut Vec<String>,
) -> Vec<&'a str> {
    let Some(items) = value.and_then(Value::as_array) else {
        errors.push(format!("{path}: expected array of strings"));
        return Vec::new();
    };
    if !allow_empty && items.is_empty() {
        errors.push(format!("{path}: array must not be empty"));
    }
    let mut out = Vec::with_capacity(items.len());
    let mut seen = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        match item.as_str() {
            Some(value) if !value.trim().is_empty() => {
                if !seen.insert(value) {
                    errors.push(format!("{path}[{index}]: duplicate value {value}"));
                }
                out.push(value);
            }
            _ => errors.push(format!("{path}[{index}]: expected non-empty string")),
        }
    }
    out
}

fn validate_manifest(manifest: &Value, errors: &mut Vec<String>) {
    let Some(root) = object(
        manifest,
        "manifest",
        &[
            "schema_version", "benchmark_id", "title", "claim_class", "scope", "source",
            "generator", "protocol", "candidate", "baselines", "oracle", "metrics",
            "metric_definitions", "notes",
        ],
        &[
            "schema_version", "benchmark_id", "title", "claim_class", "scope", "source",
            "protocol", "candidate", "baselines", "oracle", "metrics", "metric_definitions",
        ],
        errors,
    ) else {
        return;
    };

    if root.get("schema_version").and_then(Value::as_str) != Some(MANIFEST_VERSION) {
        errors.push(format!("manifest.schema_version: must equal {MANIFEST_VERSION}"));
    }
    if let Some(id) = nonempty_string(root.get("benchmark_id"), "manifest.benchmark_id", errors) {
        if id.len() < 3 || !id.bytes().all(|b| b.is_ascii_lowercase() || b.is_ascii_digit() || b".-_".contains(&b)) {
            errors.push("manifest.benchmark_id: must use lowercase ASCII letters, digits, dot, dash or underscore and contain at least three bytes".to_owned());
        }
    }
    nonempty_string(root.get("title"), "manifest.title", errors);
    one_of(
        root.get("claim_class"),
        "manifest.claim_class",
        &["semantic-correctness", "algorithmic-quality", "operational-performance", "industrial-validation"],
        errors,
    );

    if let Some(scope) = root.get("scope") {
        if let Some(scope) = object(
            scope, "manifest.scope",
            &["models", "supported_constraints", "unsupported_constraints"],
            &["models", "supported_constraints", "unsupported_constraints"], errors,
        ) {
            strings_array(scope.get("models"), "manifest.scope.models", false, errors);
            strings_array(scope.get("supported_constraints"), "manifest.scope.supported_constraints", true, errors);
            strings_array(scope.get("unsupported_constraints"), "manifest.scope.unsupported_constraints", true, errors);
            for model in scope.get("models").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str) {
                if !["rectangular-space-time", "spatial-transfers", "unary-resources", "execution-replay", "irregular-polygons", "multi-crane-physics"].contains(&model) {
                    errors.push(format!("manifest.scope.models: unsupported model {model}"));
                }
            }
        }
    }

    let generated = root.get("source")
        .and_then(|source| source.get("kind"))
        .and_then(Value::as_str) == Some("generated");
    if let Some(source) = root.get("source") {
        if let Some(source) = object(source, "manifest.source",
            &["kind", "uri", "sha256", "license_or_access_basis", "notes"],
            &["kind", "uri", "sha256"], errors) {
            one_of(source.get("kind"), "manifest.source.kind",
                &["generated", "published", "permissioned-industrial", "hand-authored"], errors);
            nonempty_string(source.get("uri"), "manifest.source.uri", errors);
            hash(source.get("sha256"), "manifest.source.sha256", &[64], errors);
            if let Some(basis) = source.get("license_or_access_basis") {
                nonempty_string(Some(basis), "manifest.source.license_or_access_basis", errors);
            }
        }
    }
    if generated && !root.contains_key("generator") {
        errors.push("manifest.generator: required when source.kind is generated".to_owned());
    }
    if let Some(generator) = root.get("generator") {
        if let Some(generator) = object(generator, "manifest.generator",
            &["name", "version", "seed", "parameters_sha256"],
            &["name", "version", "seed", "parameters_sha256"], errors) {
            nonempty_string(generator.get("name"), "manifest.generator.name", errors);
            nonempty_string(generator.get("version"), "manifest.generator.version", errors);
            positive_integer(generator.get("seed"), "manifest.generator.seed", true, errors);
            hash(generator.get("parameters_sha256"), "manifest.generator.parameters_sha256", &[64], errors);
        }
    }

    if let Some(protocol) = root.get("protocol") {
        if let Some(protocol) = object(protocol, "manifest.protocol",
            &["hardware_class", "wall_clock_budget_ms", "candidate_evaluation_budget", "expected_case_count", "repetitions", "warmup_runs", "random_seed", "timeout_policy", "notes"],
            &["hardware_class", "wall_clock_budget_ms", "candidate_evaluation_budget", "expected_case_count", "repetitions", "warmup_runs", "random_seed", "timeout_policy"], errors) {
            nonempty_string(protocol.get("hardware_class"), "manifest.protocol.hardware_class", errors);
            for field in ["wall_clock_budget_ms", "candidate_evaluation_budget", "expected_case_count", "repetitions"] {
                positive_integer(protocol.get(field), &format!("manifest.protocol.{field}"), false, errors);
            }
            for field in ["warmup_runs", "random_seed"] {
                positive_integer(protocol.get(field), &format!("manifest.protocol.{field}"), true, errors);
            }
            one_of(protocol.get("timeout_policy"), "manifest.protocol.timeout_policy",
                &["count-as-no-solution", "censored-runtime", "invalidate-run"], errors);
        }
    }

    if let Some(candidate) = root.get("candidate") {
        validate_revision_config(candidate, "manifest.candidate", errors);
    }
    if let Some(baselines) = root.get("baselines").and_then(Value::as_array) {
        if baselines.is_empty() {
            errors.push("manifest.baselines: requires at least one baseline".to_owned());
        }
        let mut names = BTreeSet::new();
        for (index, baseline) in baselines.iter().enumerate() {
            validate_revision_config(baseline, &format!("manifest.baselines[{index}]"), errors);
            if let Some(name) = baseline.get("name").and_then(Value::as_str) {
                if !names.insert(name) {
                    errors.push(format!("manifest.baselines: duplicate baseline name {name}"));
                }
            }
        }
    } else {
        errors.push("manifest.baselines: expected array".to_owned());
    }

    if let Some(oracle) = root.get("oracle") {
        if let Some(oracle) = object(oracle, "manifest.oracle",
            &["kind", "implementation_id", "revision", "source_sha256", "limitations"],
            &["kind", "implementation_id"], errors) {
            one_of(oracle.get("kind"), "manifest.oracle.kind",
                &["exact-enumeration", "independent-verifier", "published-reference", "industrial-execution-trace", "none"], errors);
            nonempty_string(oracle.get("implementation_id"), "manifest.oracle.implementation_id", errors);
            if let Some(revision) = oracle.get("revision") {
                nonempty_string(Some(revision), "manifest.oracle.revision", errors);
            }
            if let Some(source_sha) = oracle.get("source_sha256") {
                hash(Some(source_sha), "manifest.oracle.source_sha256", &[64], errors);
            }
            if let Some(limitations) = oracle.get("limitations") {
                strings_array(Some(limitations), "manifest.oracle.limitations", true, errors);
            }
        }
    }

    const METRICS: &[&str] = &[
        "false_feasible_count", "false_infeasible_count", "feasible_cases_found", "feasible_case_recall",
        "area_time_utilization", "vehicle_travel_time", "makespan", "tardiness", "recovery_horizon",
        "unaffected_plan_churn", "reservation_changes", "rehandles", "objective_regret", "runtime_ms",
        "peak_memory_bytes", "timeout_count", "bound_gap", "start_deviation_ticks",
        "completion_deviation_ticks", "missing_observation_count",
    ];
    let metric_names = strings_array(root.get("metrics"), "manifest.metrics", false, errors);
    for name in &metric_names {
        if !METRICS.contains(name) {
            errors.push(format!("manifest.metrics: unsupported metric {name}"));
        }
    }
    let mut defined_metrics = BTreeSet::new();
    if let Some(definitions) = root.get("metric_definitions").and_then(Value::as_array) {
        if definitions.is_empty() {
            errors.push("manifest.metric_definitions: must not be empty".to_owned());
        }
        for (index, definition) in definitions.iter().enumerate() {
            let path = format!("manifest.metric_definitions[{index}]");
            if let Some(definition) = object(definition, &path,
                &["name", "unit", "definition", "aggregation", "direction", "target_value", "weight", "notes"],
                &["name", "unit", "definition", "aggregation", "direction"], errors) {
                if let Some(name) = nonempty_string(definition.get("name"), &format!("{path}.name"), errors) {
                    if !defined_metrics.insert(name.clone()) {
                        errors.push(format!("{path}.name: duplicate metric definition {name}"));
                    }
                }
                nonempty_string(definition.get("unit"), &format!("{path}.unit"), errors);
                nonempty_string(definition.get("definition"), &format!("{path}.definition"), errors);
                one_of(definition.get("aggregation"), &format!("{path}.aggregation"),
                    &["sum", "mean", "median", "min", "max", "p95", "count", "per-case", "ratio"], errors);
                one_of(definition.get("direction"), &format!("{path}.direction"),
                    &["minimize", "maximize", "target", "report-only"], errors);
                if let Some(weight) = definition.get("weight") {
                    if !weight.as_f64().is_some_and(|weight| weight.is_finite() && weight >= 0.0) {
                        errors.push(format!("{path}.weight: expected non-negative finite number"));
                    }
                }
                if let Some(target) = definition.get("target_value") {
                    if !target.as_f64().is_some_and(f64::is_finite) {
                        errors.push(format!("{path}.target_value: expected finite number"));
                    }
                }
            }
        }
    } else {
        errors.push("manifest.metric_definitions: expected array".to_owned());
    }
    let declared: BTreeSet<String> = metric_names.iter().map(|name| (*name).to_owned()).collect();
    if declared != defined_metrics {
        errors.push("manifest.metric_definitions: names must exactly match manifest.metrics".to_owned());
    }
}

fn validate_revision_config(value: &Value, path: &str, errors: &mut Vec<String>) {
    if let Some(value) = object(value, path, &["name", "revision", "config_sha256"], &["name", "revision", "config_sha256"], errors) {
        nonempty_string(value.get("name"), &format!("{path}.name"), errors);
        hash(value.get("revision"), &format!("{path}.revision"), &[40, 64], errors);
        hash(value.get("config_sha256"), &format!("{path}.config_sha256"), &[64], errors);
    }
}

fn validate_report(
    manifest: &Value,
    report: &Value,
    manifest_bytes: &[u8],
    errors: &mut Vec<String>,
) -> AuditSummary {
    let zero = AuditSummary {
        case_count: 0, verified_feasible_count: 0, verified_infeasible_count: 0,
        unknown_count: 0, timeout_count: 0, not_run_count: 0, invalid_output_count: 0,
        false_feasible_count: 0, false_infeasible_count: 0,
    };
    let Some(root) = object(report, "report",
        &["schema_version", "benchmark_id", "manifest_sha256", "status", "run_metadata", "case_results", "summary", "evidence"],
        &["schema_version", "benchmark_id", "manifest_sha256", "status", "run_metadata", "case_results", "summary"], errors) else {
        return zero;
    };

    if root.get("schema_version").and_then(Value::as_str) != Some(REPORT_VERSION) {
        errors.push(format!("report.schema_version: must equal {REPORT_VERSION}"));
    }
    if root.get("benchmark_id").and_then(Value::as_str) != manifest.get("benchmark_id").and_then(Value::as_str) {
        errors.push("report.benchmark_id: does not match manifest".to_owned());
    }
    let expected_manifest_sha = sha256_hex(manifest_bytes);
    if root.get("manifest_sha256").and_then(Value::as_str) != Some(expected_manifest_sha.as_str()) {
        errors.push("report.manifest_sha256: does not match the exact UTF-8 manifest bytes".to_owned());
    }
    let status = root.get("status").and_then(Value::as_str).unwrap_or("");
    one_of(root.get("status"), "report.status", &["complete", "incomplete", "invalid"], errors);
    if status == "invalid" {
        errors.push("report.status: invalid reports cannot pass audit".to_owned());
    }

    if let Some(metadata) = root.get("run_metadata") {
        if let Some(metadata) = object(metadata, "report.run_metadata",
            &["run_id", "started_at_utc", "git_commit_sha", "executable_sha256", "hardware_class", "rustc_version", "os", "kernel", "notes"],
            &["git_commit_sha", "executable_sha256", "hardware_class", "rustc_version"], errors) {
            hash(metadata.get("git_commit_sha"), "report.run_metadata.git_commit_sha", &[40, 64], errors);
            hash(metadata.get("executable_sha256"), "report.run_metadata.executable_sha256", &[64], errors);
            nonempty_string(metadata.get("hardware_class"), "report.run_metadata.hardware_class", errors);
            nonempty_string(metadata.get("rustc_version"), "report.run_metadata.rustc_version", errors);
            if metadata.get("git_commit_sha").and_then(Value::as_str)
                != manifest.pointer("/candidate/revision").and_then(Value::as_str) {
                errors.push("report.run_metadata.git_commit_sha: does not match manifest candidate revision".to_owned());
            }
        }
    }

    let mut summary = AuditSummary { case_count: 0, ..zero };
    let mut ids = BTreeSet::new();
    let cases = match root.get("case_results").and_then(Value::as_array) {
        Some(cases) => cases,
        None => {
            errors.push("report.case_results: expected array".to_owned());
            return summary;
        }
    };
    summary.case_count = cases.len();
    for (index, case) in cases.iter().enumerate() {
        let path = format!("report.case_results[{index}]");
        let Some(case) = object(case, &path,
            &["case_id", "outcome", "reference_outcome", "runtime_ms", "peak_memory_bytes", "metrics", "verifier", "notes"],
            &["case_id", "outcome", "reference_outcome", "runtime_ms", "verifier"], errors) else {
            summary.unknown_count += 1;
            continue;
        };
        if let Some(id) = nonempty_string(case.get("case_id"), &format!("{path}.case_id"), errors) {
            if !ids.insert(id.clone()) {
                errors.push(format!("{path}.case_id: duplicate case identifier {id}"));
            }
        }
        let outcome = case.get("outcome").and_then(Value::as_str).unwrap_or("");
        one_of(case.get("outcome"), &format!("{path}.outcome"),
            &["verified-feasible", "verified-infeasible", "unknown", "timeout", "invalid-output", "not-run"], errors);
        let reference = case.get("reference_outcome").and_then(Value::as_str).unwrap_or("");
        one_of(case.get("reference_outcome"), &format!("{path}.reference_outcome"),
            &["feasible", "infeasible", "unknown", "not-applicable"], errors);
        if case.get("runtime_ms").and_then(Value::as_f64).is_none_or(|value| !value.is_finite() || value < 0.0) {
            errors.push(format!("{path}.runtime_ms: expected non-negative finite number"));
        }
        if let Some(memory) = case.get("peak_memory_bytes") {
            positive_integer(Some(memory), &format!("{path}.peak_memory_bytes"), true, errors);
        }

        match outcome {
            "verified-feasible" => summary.verified_feasible_count += 1,
            "verified-infeasible" => summary.verified_infeasible_count += 1,
            "unknown" => summary.unknown_count += 1,
            "timeout" => summary.timeout_count += 1,
            "not-run" => summary.not_run_count += 1,
            "invalid-output" => summary.invalid_output_count += 1,
            _ => {}
        }
        if reference == "unknown" {
            if outcome != "unknown" && outcome != "timeout" && outcome != "not-run" && outcome != "invalid-output" {
                errors.push(format!("{path}: must not claim a ground-truth match when reference_outcome is unknown"));
            }
        }
        let expected_oracle = match (outcome, reference) {
            ("verified-feasible", "feasible") | ("verified-infeasible", "infeasible") => "match",
            ("verified-feasible", "infeasible") | ("verified-infeasible", "feasible") => "mismatch",
            (_, "not-applicable") => "not-applicable",
            _ => "unknown",
        };
        if outcome == "verified-feasible" && reference == "infeasible" {
            summary.false_feasible_count += 1;
        }
        if outcome == "verified-infeasible" && reference == "feasible" {
            summary.false_infeasible_count += 1;
        }

        if let Some(verifier) = case.get("verifier") {
            if let Some(verifier) = object(verifier, &format!("{path}.verifier"),
                &["status", "oracle_outcome", "violation_count", "oracle_id", "evidence_sha256"],
                &["status", "oracle_outcome", "violation_count"], errors) {
                one_of(verifier.get("status"), &format!("{path}.verifier.status"),
                    &["passed", "failed", "not-run", "not-applicable"], errors);
                one_of(verifier.get("oracle_outcome"), &format!("{path}.verifier.oracle_outcome"),
                    &["match", "mismatch", "unknown", "not-applicable"], errors);
                positive_integer(verifier.get("violation_count"), &format!("{path}.verifier.violation_count"), true, errors);
                if verifier.get("oracle_outcome").and_then(Value::as_str) != Some(expected_oracle) {
                    errors.push(format!("{path}.verifier.oracle_outcome: inconsistent with outcome and reference_outcome"));
                }
                if outcome == "verified-feasible" && verifier.get("status").and_then(Value::as_str) != Some("passed") {
                    errors.push(format!("{path}: verified-feasible requires verifier.status=passed"));
                }
                if verifier.get("status").and_then(Value::as_str) == Some("passed")
                    && verifier.get("violation_count").and_then(Value::as_i64) != Some(0) {
                    errors.push(format!("{path}.verifier: passed status requires zero violations"));
                }
                if verifier.get("status").and_then(Value::as_str) == Some("failed")
                    && verifier.get("violation_count").and_then(Value::as_i64).unwrap_or(0) == 0 {
                    errors.push(format!("{path}.verifier: failed status requires at least one violation"));
                }
                if let Some(evidence) = verifier.get("evidence_sha256") {
                    hash(Some(evidence), &format!("{path}.verifier.evidence_sha256"), &[64], errors);
                }
                if let Some(oracle_id) = verifier.get("oracle_id") {
                    nonempty_string(Some(oracle_id), &format!("{path}.verifier.oracle_id"), errors);
                }
            }
        }
        if outcome == "unknown" || reference == "unknown" {
            // Reference uncertainty is part of the public unknown count.
            // If outcome was already unknown, don't count the case twice.
            if outcome != "unknown" {
                summary.unknown_count += 1;
            }
        }
    }

    let expected_case_count = manifest.pointer("/protocol/expected_case_count").and_then(Value::as_u64);
    if status == "complete" {
        if expected_case_count != Some(cases.len() as u64) {
            errors.push("report.status: complete requires exactly protocol.expected_case_count case records".to_owned());
        }
        if summary.unknown_count > 0 || summary.timeout_count > 0 || summary.not_run_count > 0 || summary.invalid_output_count > 0 {
            errors.push("report.status: complete cannot contain unknown, timeout, not-run, or invalid-output cases".to_owned());
        }
        if manifest.get("claim_class").and_then(Value::as_str) == Some("semantic-correctness")
            && cases.iter().any(|case| case.get("reference_outcome").and_then(Value::as_str) == Some("unknown")) {
            errors.push("report.status: semantic-correctness completion requires a known reference outcome for every case".to_owned());
        }
    }
    if let Some(evidence) = root.get("evidence") {
        if let Some(evidence) = object(evidence, "report.evidence",
            &["report_sha256", "artifact_uri", "notes"], &[], errors) {
            if let Some(digest) = evidence.get("report_sha256") {
                hash(Some(digest), "report.evidence.report_sha256", &[64], errors);
            }
            if let Some(uri) = evidence.get("artifact_uri") {
                nonempty_string(Some(uri), "report.evidence.artifact_uri", errors);
            }
        }
    }
    validate_summary(root.get("summary"), &summary, errors);
    summary
}

fn validate_summary(value: Option<&Value>, actual: &AuditSummary, errors: &mut Vec<String>) {
    let Some(value) = value else {
        errors.push("report.summary: required field missing".to_owned());
        return;
    };
    let fields = [
        ("case_count", actual.case_count),
        ("verified_feasible_count", actual.verified_feasible_count),
        ("verified_infeasible_count", actual.verified_infeasible_count),
        ("unknown_count", actual.unknown_count),
        ("timeout_count", actual.timeout_count),
        ("not_run_count", actual.not_run_count),
        ("invalid_output_count", actual.invalid_output_count),
        ("false_feasible_count", actual.false_feasible_count),
        ("false_infeasible_count", actual.false_infeasible_count),
    ];
    let allowed = [
        "case_count", "verified_feasible_count", "verified_infeasible_count", "unknown_count",
        "timeout_count", "not_run_count", "invalid_output_count", "false_feasible_count",
        "false_infeasible_count", "aggregate_metrics",
    ];
    let required_fields: Vec<&str> = fields.iter().map(|(name, _)| *name).collect();
    if let Some(summary) = object(value, "report.summary", &allowed, &required_fields, errors) {
        for (field, expected) in fields {
            if summary.get(field).and_then(Value::as_u64) != Some(expected as u64) {
                errors.push(format!("report.summary.{field}: inconsistent with per-case records (expected {expected})"));
            }
        }
        if let Some(metrics) = summary.get("aggregate_metrics") {
            if !metrics.is_object() {
                errors.push("report.summary.aggregate_metrics: expected object".to_owned());
            }
        }
    }
}

/// Recompute summary totals from case records. Exposed so producers and tests can
/// avoid copy/pasting the auditor's interpretation into report generation code.
pub fn summary_from_cases(report: &Value) -> BTreeMap<String, usize> {
    let cases = report.get("case_results").and_then(Value::as_array).cloned().unwrap_or_default();
    let mut values = BTreeMap::from([
        ("case_count".to_owned(), cases.len()),
        ("verified_feasible_count".to_owned(), 0),
        ("verified_infeasible_count".to_owned(), 0),
        ("unknown_count".to_owned(), 0),
        ("timeout_count".to_owned(), 0),
        ("not_run_count".to_owned(), 0),
        ("invalid_output_count".to_owned(), 0),
        ("false_feasible_count".to_owned(), 0),
        ("false_infeasible_count".to_owned(), 0),
    ]);
    for case in cases {
        let outcome = case.get("outcome").and_then(Value::as_str).unwrap_or("");
        let reference = case.get("reference_outcome").and_then(Value::as_str).unwrap_or("");
        match outcome {
            "verified-feasible" => *values.get_mut("verified_feasible_count").unwrap() += 1,
            "verified-infeasible" => *values.get_mut("verified_infeasible_count").unwrap() += 1,
            "unknown" => *values.get_mut("unknown_count").unwrap() += 1,
            "timeout" => *values.get_mut("timeout_count").unwrap() += 1,
            "not-run" => *values.get_mut("not_run_count").unwrap() += 1,
            "invalid-output" => *values.get_mut("invalid_output_count").unwrap() += 1,
            _ => {}
        }
        if reference == "unknown" && outcome != "unknown" {
            *values.get_mut("unknown_count").unwrap() += 1;
        }
        if outcome == "verified-feasible" && reference == "infeasible" {
            *values.get_mut("false_feasible_count").unwrap() += 1;
        }
        if outcome == "verified-infeasible" && reference == "feasible" {
            *values.get_mut("false_infeasible_count").unwrap() += 1;
        }
    }
    values
}

#[cfg(test)]
mod unit_tests {
    use super::*;

    #[test]
    fn sha256_is_standard_sha256() {
        assert_eq!(
            sha256_hex(b"abc"),
            "ba7816bf8f01cfea414140de5dae2223b00361a396177a9cb410ff61f20015ad"
        );
    }

    #[test]
    fn embedded_schemas_are_syntactically_and_structurally_consistent() {
        validate_embedded_schemas().expect("embedded schemas should parse and expose their required fields");
    }

    #[test]
    fn summary_recomputation_counts_unknown_reference_once() {
        let report = serde_json::json!({
            "case_results": [
                {"outcome":"unknown","reference_outcome":"unknown"},
                {"outcome":"verified-feasible","reference_outcome":"unknown"},
                {"outcome":"verified-feasible","reference_outcome":"feasible"},
                {"outcome":"verified-feasible","reference_outcome":"infeasible"},
                {"outcome":"verified-infeasible","reference_outcome":"feasible"},
                {"outcome":"timeout","reference_outcome":"feasible"}
            ]
        });
        let summary = summary_from_cases(&report);
        assert_eq!(summary["case_count"], 6);
        assert_eq!(summary["unknown_count"], 2);
        assert_eq!(summary["false_feasible_count"], 1);
        assert_eq!(summary["false_infeasible_count"], 1);
        assert_eq!(summary["timeout_count"], 1);
    }
}
