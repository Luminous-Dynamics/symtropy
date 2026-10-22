use serde_json::{json, Map, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
type Check<T> = Result<T, String>;

#[derive(Debug)]
struct Contracts {
    candidate_ids: BTreeSet<String>,
    scenario_metrics: BTreeMap<String, BTreeSet<String>>,
    scenario_gate_ids: BTreeMap<String, BTreeSet<String>>,
    metric_ids: BTreeSet<String>,
    metric_unit_options: BTreeMap<String, BTreeSet<String>>,
    metric_qualifying_values: BTreeMap<String, BTreeSet<String>>,
    contract_digests: BTreeMap<String, String>,
}

#[derive(Debug)]
struct LoadedRecords {
    records: Vec<(PathBuf, Value)>,
    errors: Vec<String>,
    input_sha256: BTreeMap<String, String>,
}

fn object<'a>(v: &'a Value, at: &str) -> Check<&'a Map<String, Value>> {
    v.as_object().ok_or_else(|| format!("{at}: expected object"))
}
fn field<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(v, at)?.get(key).ok_or_else(|| format!("{at}: missing field {key}"))
}
fn text(v: &Value, at: &str) -> Check<String> {
    v.as_str().map(str::to_owned).ok_or_else(|| format!("{at}: expected string"))
}
fn req_text(v: &Value, key: &str, at: &str) -> Check<String> {
    text(field(v, key, at)?, &format!("{at}.{key}"))
}
fn array_field<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a Vec<Value>> {
    field(v, key, at)?.as_array().ok_or_else(|| format!("{at}.{key}: expected array"))
}
fn missing_keys(v: &Value, required: &[&str], at: &str, errors: &mut Vec<String>) {
    match object(v, at) {
        Ok(o) => for key in required {
            if !o.contains_key(*key) { errors.push(format!("{at}: missing required key {key}")); }
        },
        Err(e) => errors.push(e)
    }
}
fn reject_unknown_keys(v: &Value, allowed: &[&str], at: &str, errors: &mut Vec<String>) {
    match object(v, at) {
        Ok(o) => for key in o.keys() {
            if !allowed.contains(&key.as_str()) { errors.push(format!("{at}: unknown key {key}")); }
        },
        Err(e) => errors.push(e)
    }
}
fn valid_sha256(s: &str) -> bool {
    s.len() == 64 && s.bytes().all(|b| b.is_ascii_hexdigit())
}
fn sha256_hex(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn load_json_with_digest(path: &Path) -> Check<(Value, String)> {
    let raw = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let digest = sha256_hex(&raw);
    let value = serde_json::from_slice(&raw)
        .map_err(|e| format!("{}: invalid JSON: {e}", path.display()))?;
    Ok((value, digest))
}

fn load_contracts(dir: &Path) -> Check<Contracts> {
    let scenarios_path = dir.join("KITCHEN_STATION_SCENARIOS_V1.json");
    let registry_path = dir.join("KITCHEN_STATION_METRIC_REGISTRY_V1.json");
    let schema_path = dir.join("KITCHEN_STATION_RECORD_SCHEMA_V1.json");
    let design_path = dir.join("KITCHEN_STATION_DESIGN_SPACE_V1.json");
    let (scenarios_doc, scenarios_digest) = load_json_with_digest(&scenarios_path)?;
    let (registry, registry_digest) = load_json_with_digest(&registry_path)?;
    let (schema, schema_digest) = load_json_with_digest(&schema_path)?;
    let (design, design_digest) = load_json_with_digest(&design_path)?;
    let contract_digests = [
        ("KITCHEN_STATION_SCENARIOS_V1.json".to_owned(), scenarios_digest),
        ("KITCHEN_STATION_METRIC_REGISTRY_V1.json".to_owned(), registry_digest),
        ("KITCHEN_STATION_RECORD_SCHEMA_V1.json".to_owned(), schema_digest),
        ("KITCHEN_STATION_DESIGN_SPACE_V1.json".to_owned(), design_digest),
    ].into_iter().collect::<BTreeMap<_, _>>();
    if schema.get("$schema").and_then(Value::as_str) != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err(format!("{}: unexpected JSON Schema draft", schema_path.display()));
    }
    let scenario_array = scenarios_doc.get("scenarios").and_then(Value::as_array)
        .ok_or_else(|| format!("{}: scenarios must be an array", scenarios_path.display()))?;
    let registry_map = registry.get("metrics").and_then(Value::as_object)
        .ok_or_else(|| format!("{}: metrics must be an object", registry_path.display()))?;
    let candidate_array = design.get("candidate_families").and_then(Value::as_array)
        .ok_or_else(|| format!("{}: candidate_families must be an array", design_path.display()))?;

    let mut scenario_metrics = BTreeMap::new();
    let mut scenario_gate_ids = BTreeMap::new();
    for scenario in scenario_array {
        let id = scenario.get("scenario_id").and_then(Value::as_str)
            .ok_or_else(|| "scenario missing scenario_id".to_string())?.to_owned();
        let metrics = scenario.get("primary_metrics").and_then(Value::as_array)
            .ok_or_else(|| format!("{id}: primary_metrics must be an array"))?
            .iter().map(|m| m.as_str().map(str::to_owned)
                .ok_or_else(|| format!("{id}: metric ID must be a string")))
            .collect::<Check<BTreeSet<_>>>()?;
        if metrics.is_empty() { return Err(format!("{id}: no primary metrics")); }
        let gates = scenario.get("required_safety_gate_ids").and_then(Value::as_array)
            .ok_or_else(|| format!("{id}: required_safety_gate_ids must be an array"))?
            .iter().map(|g| g.as_str().map(str::to_owned)
                .ok_or_else(|| format!("{id}: gate ID must be a string")))
            .collect::<Check<BTreeSet<_>>>()?;
        if gates.is_empty() { return Err(format!("{id}: no required safety gates")); }
        let gate_count = scenario.get("required_safety_gate_ids").and_then(Value::as_array).map(Vec::len).unwrap_or(0);
        if gate_count != gates.len() { return Err(format!("{id}: duplicate required safety gate ID")); }
        if scenario_metrics.insert(id.clone(), metrics).is_some() { return Err(format!("duplicate scenario ID {id}")); }
        scenario_gate_ids.insert(id, gates);
    }
    let mut candidate_ids = BTreeSet::new();
    for candidate in candidate_array {
        let id = candidate.get("candidate_id").and_then(Value::as_str)
            .ok_or_else(|| "candidate missing candidate_id".to_string())?.to_owned();
        if !candidate_ids.insert(id.clone()) { return Err(format!("duplicate candidate ID {id}")); }
    }
    let metric_ids = registry_map.keys().cloned().collect::<BTreeSet<_>>();
    let mut metric_unit_options = BTreeMap::new();
    let mut metric_qualifying_values = BTreeMap::new();
    for (id, metric) in registry_map {
        let options = metric.get("unit_options").and_then(Value::as_array)
            .ok_or_else(|| format!("metric {id} lacks machine-readable unit_options"))?
            .iter().map(|v| v.as_str().map(str::to_owned)
                .ok_or_else(|| format!("metric {id} unit_options must contain strings")))
            .collect::<Check<BTreeSet<_>>>()?;
        if options.is_empty() { return Err(format!("metric {id} has no unit options")); }
        metric_unit_options.insert(id.clone(), options);
        if let Some(qualifying) = metric.get("qualifying_values") {
            let values = qualifying.as_array()
                .ok_or_else(|| format!("metric {id} qualifying_values must be an array"))?
                .iter().map(|v| v.as_str().map(str::to_owned)
                    .ok_or_else(|| format!("metric {id} qualifying_values must contain strings")))
                .collect::<Check<BTreeSet<_>>>()?;
            if values.is_empty() { return Err(format!("metric {id} qualifying_values cannot be empty")); }
            metric_qualifying_values.insert(id.clone(), values);
        }
    }
    for (scenario, metrics) in &scenario_metrics {
        for metric in metrics {
            if !metric_ids.contains(metric) { return Err(format!("scenario {scenario} has unregistered metric {metric}")); }
        }
    }
    Ok(Contracts { candidate_ids, scenario_metrics, scenario_gate_ids, metric_ids, metric_unit_options, metric_qualifying_values, contract_digests })
}

fn artifact_is_remote(uri: &str) -> bool {
    ["https://", "http://", "urn:", "data:"].iter().any(|p| uri.starts_with(p))
}
fn validate_artifact(v: &Value, at: &str, records_dir: &Path, errors: &mut Vec<String>) {
    missing_keys(v, &["uri", "sha256", "media_type"], at, errors);
    reject_unknown_keys(v, &["uri", "sha256", "media_type", "note"], at, errors);
    let uri = match req_text(v, "uri", at) { Ok(x) => x, Err(e) => { errors.push(e); return; } };
    let digest = match req_text(v, "sha256", at) { Ok(x) => x, Err(e) => { errors.push(e); return; } };
    if !valid_sha256(&digest) { errors.push(format!("{at}: SHA-256 must be 64 hexadecimal characters")); return; }
    if req_text(v, "media_type", at).map(|s| s.trim().is_empty()).unwrap_or(true) {
        errors.push(format!("{at}: media_type must not be empty"));
    }
    // Network resources are not fetched. A relative URI is a local artifact path and must exist and match.
    if !artifact_is_remote(&uri) {
        let path = records_dir.join(&uri);
        match fs::read(&path) {
            Ok(bytes) => {
                let actual = sha256_hex(&bytes);
                if !actual.eq_ignore_ascii_case(&digest) {
                    errors.push(format!("{at}: SHA-256 mismatch for local artifact {}", path.display()));
                }
            }
            Err(e) => errors.push(format!("{at}: local artifact {} unavailable: {e}", path.display()))
        }
    }
}
fn validate_uncertainty(v: &Value, at: &str, errors: &mut Vec<String>) {
    missing_keys(v, &["kind", "value", "unit", "method"], at, errors);
    reject_unknown_keys(v, &["kind", "value", "unit", "method", "coverage_factor"], at, errors);
    let kinds = ["standard", "expanded", "confidence_interval", "resolution_bound", "other"];
    match req_text(v, "kind", at) {
        Ok(k) if kinds.contains(&k.as_str()) => {},
        Ok(k) => errors.push(format!("{at}: unknown uncertainty kind {k}")),
        Err(e) => errors.push(e)
    }
    match field(v, "value", at).and_then(|x| x.as_f64().ok_or_else(|| format!("{at}.value: expected number"))) {
        Ok(x) if x.is_finite() && x >= 0.0 => {},
        Ok(_) => errors.push(format!("{at}.value: must be finite and non-negative")),
        Err(e) => errors.push(e)
    }
    for key in ["unit", "method"] {
        match req_text(v, key, at) {
            Ok(s) if !s.trim().is_empty() => {},
            Ok(_) => errors.push(format!("{at}.{key}: must not be empty")),
            Err(e) => errors.push(e)
        }
    }
    if let Some(cf) = v.get("coverage_factor") {
        if !cf.is_null() && cf.as_f64().is_none_or(|x| !x.is_finite() || x <= 0.0) {
            errors.push(format!("{at}.coverage_factor: must be null or positive"));
        }
    }
}

fn validate_metric(v: &Value, at: &str, contracts: &Contracts, records_dir: &Path, errors: &mut Vec<String>) {
    const KEYS: &[&str] = &[
        "metric_id", "metric_status", "value", "unit", "uncertainty", "sample_count",
        "method_id", "instrument_ids", "artifact_refs", "formula_id",
        "input_metric_record_ids", "status_reason", "notes"
    ];
    missing_keys(v, &["metric_id", "metric_status", "unit"], at, errors);
    reject_unknown_keys(v, KEYS, at, errors);
    let id = match req_text(v, "metric_id", at) { Ok(x) => x, Err(e) => { errors.push(e); return; } };
    if !contracts.metric_ids.contains(&id) { errors.push(format!("{at}: unregistered metric ID {id}")); }
    let unit = match req_text(v, "unit", at) {
        Ok(s) if !s.trim().is_empty() => s,
        Ok(_) => { errors.push(format!("{at}: unit is empty")); return; },
        Err(e) => { errors.push(e); return; }
    };
    match contracts.metric_unit_options.get(&id) {
        Some(options) if options.contains("protocol-defined") => {},
        Some(options) if options.contains(&unit) => {},
        Some(options) => errors.push(format!("{at}: unit {unit} is not one of registered options {options:?}")),
        None => errors.push(format!("{at}: no unit contract registered for metric {id}"))
    }
    let status = match req_text(v, "metric_status", at) { Ok(x) => x, Err(e) => { errors.push(e); return; } };
    match status.as_str() {
        "measured" | "derived" => {
            match field(v, "value", at) {
                Ok(x) if !x.is_null() => if x.as_f64().is_some_and(|n| !n.is_finite()) {
                    errors.push(format!("{at}: non-finite numeric value"));
                },
                Ok(_) => errors.push(format!("{at}: measured/derived value cannot be null")),
                Err(e) => errors.push(e)
            }
            match field(v, "uncertainty", at) {
                Ok(u) => validate_uncertainty(u, &format!("{at}.uncertainty"), errors),
                Err(e) => errors.push(e)
            }
            match field(v, "sample_count", at).and_then(|x| x.as_u64().ok_or_else(|| format!("{at}: sample_count must be positive integer"))) {
                Ok(n) if n > 0 => {},
                Ok(_) => errors.push(format!("{at}: sample_count must be positive")),
                Err(e) => errors.push(e)
            }
            let artifacts = match array_field(v, "artifact_refs", at) {
                Ok(a) if !a.is_empty() => a,
                Ok(_) => { errors.push(format!("{at}: measured/derived metric requires artifact_refs")); return; },
                Err(e) => { errors.push(e); return; }
            };
            for (i, a) in artifacts.iter().enumerate() {
                validate_artifact(a, &format!("{at}.artifact_refs[{i}]"), records_dir, errors);
            }
            if status == "measured" {
                match req_text(v, "method_id", at) {
                    Ok(s) if !s.trim().is_empty() => {},
                    Ok(_) => errors.push(format!("{at}: method_id must not be empty")),
                    Err(e) => errors.push(e)
                }
                match array_field(v, "instrument_ids", at) {
                    Ok(xs) if !xs.iter().any(|x| x.as_str().is_none_or(|s| s.trim().is_empty())) => {},
                    Ok(_) => errors.push(format!("{at}: instrument_ids must be strings")),
                    Err(e) => errors.push(e)
                }
            } else {
                match req_text(v, "formula_id", at) {
                    Ok(s) if !s.trim().is_empty() => {},
                    Ok(_) => errors.push(format!("{at}: formula_id must not be empty")),
                    Err(e) => errors.push(e)
                }
                match array_field(v, "input_metric_record_ids", at) {
                    Ok(xs) if !xs.is_empty() && !xs.iter().any(|x| x.as_str().is_none_or(|s| s.trim().is_empty())) => {},
                    Ok(_) => errors.push(format!("{at}: input_metric_record_ids must be non-empty strings")),
                    Err(e) => errors.push(e)
                }
            }
        }
        "missing" | "invalid" | "blocked" | "not_applicable" => {
            match req_text(v, "status_reason", at) {
                Ok(s) if s.trim().len() >= 3 => {},
                Ok(_) => errors.push(format!("{at}: status_reason must explain the status")),
                Err(e) => errors.push(e)
            }
            if v.get("value").is_some_and(|x| !x.is_null()) {
                errors.push(format!("{at}: non-result status must not carry a value"));
            }
        }
        _ => errors.push(format!("{at}: invalid metric_status {status}"))
    }
}

fn validate_record(v: &Value, source: &str, records_dir: &Path, contracts: &Contracts) -> Vec<String> {
    const REQUIRED: &[&str] = &[
        "schema_version", "record_id", "candidate_id", "scenario_id", "trial_id", "run_status",
        "evidence_class", "recorded_at", "conditions", "candidate_spec", "protocol", "metrics",
        "safety_gates", "deviations", "claim_limit"
    ];
    const TOP_KEYS: &[&str] = &[
        "schema_version", "record_id", "candidate_id", "scenario_id", "trial_id", "run_status",
        "evidence_class", "recorded_at", "operator_code", "conditions", "candidate_spec", "protocol",
        "metrics", "safety_gates", "raw_artifacts", "deviations", "exclusion_reason", "claim_limit"
    ];
    let mut errors = Vec::new();
    missing_keys(v, REQUIRED, source, &mut errors);
    reject_unknown_keys(v, TOP_KEYS, source, &mut errors);
    if req_text(v, "schema_version", source).unwrap_or_default() != "1.0.0" {
        errors.push(format!("{source}: schema_version must be 1.0.0"));
    }

    let candidate_id = match req_text(v, "candidate_id", source) { Ok(x) => x, Err(e) => { errors.push(e); String::new() } };
    if !contracts.candidate_ids.contains(&candidate_id) { errors.push(format!("{source}: unknown candidate {candidate_id}")); }
    let scenario_id = match req_text(v, "scenario_id", source) { Ok(x) => x, Err(e) => { errors.push(e); String::new() } };
    let expected = contracts.scenario_metrics.get(&scenario_id);
    if expected.is_none() { errors.push(format!("{source}: unknown scenario {scenario_id}")); }

    let run_status = match req_text(v, "run_status", source) { Ok(x) => x, Err(e) => { errors.push(e); String::new() } };
    if !["complete", "not_run", "blocked", "invalid"].contains(&run_status.as_str()) {
        errors.push(format!("{source}: invalid run_status {run_status}"));
    }
    if run_status != "complete" {
        match req_text(v, "exclusion_reason", source) {
            Ok(s) if s.trim().len() >= 3 => {},
            Ok(_) => errors.push(format!("{source}: exclusion_reason is required for non-complete run")),
            Err(e) => errors.push(e)
        }
    }

    let evidence = match req_text(v, "evidence_class", source) { Ok(x) => x, Err(e) => { errors.push(e); String::new() } };
    if !["physical_observation", "derived_from_physical", "simulation", "synthetic_fixture", "literature", "documentation"].contains(&evidence.as_str()) {
        errors.push(format!("{source}: invalid evidence_class {evidence}"));
    }
    match req_text(v, "recorded_at", source) {
        Ok(s) if s.contains('T') && (s.ends_with('Z') || s.rfind('+').is_some() || s.rfind('-').is_some_and(|i| i > s.find('T').unwrap_or(0))) => {},
        Ok(_) => errors.push(format!("{source}: recorded_at must be offset-aware ISO timestamp")),
        Err(e) => errors.push(e)
    }
    match field(v, "conditions", source) {
        Ok(x) if x.as_object().is_some_and(|o| !o.is_empty()) => {},
        Ok(_) => errors.push(format!("{source}: conditions must be a non-empty object")),
        Err(e) => errors.push(e)
    }

    for (name, keys, label) in [
        ("candidate_spec", &["revision", "sha256"][..], "revision"),
        ("protocol", &["version", "sha256"][..], "version")
    ] {
        match field(v, name, source) {
            Ok(x) => {
                missing_keys(x, keys, &format!("{source}.{name}"), &mut errors);
                reject_unknown_keys(x, keys, &format!("{source}.{name}"), &mut errors);
                match req_text(x, "sha256", &format!("{source}.{name}")) {
                    Ok(s) if valid_sha256(&s) => {},
                    Ok(_) => errors.push(format!("{source}.{name}: invalid SHA-256")),
                    Err(e) => errors.push(e)
                }
                match req_text(x, label, &format!("{source}.{name}")) {
                    Ok(s) if !s.trim().is_empty() => {},
                    Ok(_) => errors.push(format!("{source}.{name}.{label} is empty")),
                    Err(e) => errors.push(e)
                }
            },
            Err(e) => errors.push(e)
        }
    }

    match array_field(v, "metrics", source) {
        Ok(metrics) => {
            let mut seen = BTreeSet::new();
            for (i, metric) in metrics.iter().enumerate() {
                let at = format!("{source}.metrics[{i}]");
                let id = metric.get("metric_id").and_then(Value::as_str).unwrap_or("");
                if !seen.insert(id.to_owned()) { errors.push(format!("{at}: duplicate metric ID {id}")); }
                validate_metric(metric, &at, contracts, records_dir, &mut errors);
            }
            if let Some(expected) = expected {
                for id in expected {
                    if !seen.contains(id) { errors.push(format!("{source}: required metric {id} absent; use an explicit status")); }
                }
            }
            if run_status != "complete" && metrics.iter().any(|m| matches!(
                m.get("metric_status").and_then(Value::as_str), Some("measured" | "derived")
            )) {
                errors.push(format!("{source}: non-complete run cannot contain measured/derived outcome metrics"));
            }
        },
        Err(e) => errors.push(e)
    }
    match array_field(v, "safety_gates", source) {
        Ok(gates) => {
            let mut actual_gate_ids = BTreeSet::new();
            for (i, gate) in gates.iter().enumerate() {
                let at = format!("{source}.safety_gates[{i}]");
                missing_keys(gate, &["gate_id", "status", "basis"], &at, &mut errors);
                reject_unknown_keys(gate, &["gate_id", "status", "basis", "artifact_refs"], &at, &mut errors);
                match req_text(gate, "gate_id", &at) {
                    Ok(id) if !id.trim().is_empty() => {
                        if !actual_gate_ids.insert(id.clone()) {
                            errors.push(format!("{at}: duplicate safety gate ID {id}"));
                        }
                    },
                    Ok(_) => errors.push(format!("{at}.gate_id is empty")),
                    Err(e) => errors.push(e)
                }
                match req_text(gate, "basis", &at) {
                    Ok(s) if !s.trim().is_empty() => {},
                    Ok(_) => errors.push(format!("{at}.basis is empty")),
                    Err(e) => errors.push(e)
                }
                match req_text(gate, "status", &at) {
                    Ok(s) if ["pass", "fail", "unknown", "not_applicable"].contains(&s.as_str()) => {},
                    Ok(s) => errors.push(format!("{at}: invalid safety gate status {s}")),
                    Err(e) => errors.push(e)
                }
            }
            if let Some(expected) = contracts.scenario_gate_ids.get(&scenario_id) {
                for required_id in expected {
                    if !actual_gate_ids.contains(required_id) {
                        errors.push(format!("{source}: required safety gate {required_id} is missing; explicitly status it"));
                    }
                }
                for actual_id in actual_gate_ids.difference(expected) {
                    errors.push(format!("{source}: unregistered safety gate {actual_id} for scenario {scenario_id}"));
                }
            }
        },
        Err(e) => errors.push(e)
    }

    if let Some(raw) = v.get("raw_artifacts") {
        match raw.as_array() {
            Some(items) => for (i, item) in items.iter().enumerate() {
                validate_artifact(item, &format!("{source}.raw_artifacts[{i}]"), records_dir, &mut errors)
            },
            None => errors.push(format!("{source}: raw_artifacts must be an array"))
        }
    }
    match array_field(v, "deviations", source) {
        Ok(xs) => for (i, x) in xs.iter().enumerate() {
            if x.as_str().is_none() { errors.push(format!("{source}.deviations[{i}] must be a string")); }
        },
        Err(e) => errors.push(e)
    }
    match req_text(v, "claim_limit", source) {
        Ok(s) if !s.trim().is_empty() => {},
        Ok(_) => errors.push(format!("{source}: claim_limit is empty")),
        Err(e) => errors.push(e)
    }
    errors
}

fn gate_qualified(v: &Value, contracts: &Contracts) -> bool {
    let Some(scenario) = v.get("scenario_id").and_then(Value::as_str) else { return false; };
    let Some(expected) = contracts.scenario_gate_ids.get(scenario) else { return false; };
    let Some(gates) = v.get("safety_gates").and_then(Value::as_array) else { return false; };
    if gates.len() != expected.len() { return false; }
    let mut actual = BTreeMap::new();
    for gate in gates {
        let (Some(id), Some(status)) = (
            gate.get("gate_id").and_then(Value::as_str),
            gate.get("status").and_then(Value::as_str)
        ) else { return false; };
        if actual.insert(id, status).is_some() { return false; }
    }
    expected.iter().all(|id| matches!(
        actual.get(id.as_str()).copied(), Some("pass" | "not_applicable")
    ))
}
fn physically_eligible(v: &Value, contracts: &Contracts) -> bool {
    let Some(scenario) = v.get("scenario_id").and_then(Value::as_str) else { return false; };
    let Some(expected) = contracts.scenario_metrics.get(scenario) else { return false; };
    let indexed = metrics_by_id(v);
    let all_primary_metrics_observed_and_qualified = expected.iter().all(|id| {
        let Some(metric) = indexed.get(id) else { return false; };
        if !matches!(metric.get("metric_status").and_then(Value::as_str), Some("measured" | "derived")) {
            return false;
        }
        if let Some(allowed) = contracts.metric_qualifying_values.get(id) {
            return metric.get("value").and_then(Value::as_str).is_some_and(|v| allowed.contains(v));
        }
        true
    });
    v.get("run_status").and_then(Value::as_str) == Some("complete")
        && matches!(v.get("evidence_class").and_then(Value::as_str),
            Some("physical_observation" | "derived_from_physical"))
        && gate_qualified(v, contracts)
        && all_primary_metrics_observed_and_qualified
}
fn read_records(dir: &Path) -> Check<LoadedRecords> {
    let mut paths = fs::read_dir(dir).map_err(|e| format!("{}: {e}", dir.display()))?
        .map(|item| item.map(|e| e.path()).map_err(|e| e.to_string()))
        .collect::<Result<Vec<_>, _>>()?;
    paths.retain(|p| p.extension().and_then(|s| s.to_str()) == Some("json"));
    paths.sort();
    let mut records = Vec::new();
    let mut errors = Vec::new();
    let mut input_sha256 = BTreeMap::new();
    for path in paths {
        match fs::read(&path) {
            Ok(raw) => {
                let digest = sha256_hex(&raw);
                if let Some(name) = path.file_name().and_then(|x| x.to_str()) {
                    input_sha256.insert(name.to_owned(), digest);
                }
                match serde_json::from_slice::<Value>(&raw) {
                    Ok(v) => records.push((path, v)),
                    Err(e) => errors.push(format!("{}: invalid JSON: {e}", path.display()))
                }
            }
            Err(e) => errors.push(format!("{}: {e}", path.display()))
        }
    }
    Ok(LoadedRecords { records, errors, input_sha256 })
}
fn validate_loaded(loaded: &LoadedRecords, dir: &Path, contracts: &Contracts) -> Vec<String> {
    let mut errors = loaded.errors.clone();
    let mut ids = BTreeSet::new();
    let mut trial_keys = BTreeSet::new();
    for (path, record) in &loaded.records {
        errors.extend(validate_record(record, &path.display().to_string(), dir, contracts));
        if let Some(id) = record.get("record_id").and_then(Value::as_str) {
            if !ids.insert(id.to_owned()) { errors.push(format!("duplicate record_id {id}")); }
        }
        if let (Some(c), Some(s), Some(t)) = (
            record.get("candidate_id").and_then(Value::as_str),
            record.get("scenario_id").and_then(Value::as_str),
            record.get("trial_id").and_then(Value::as_str)
        ) {
            if !trial_keys.insert((c.to_owned(), s.to_owned(), t.to_owned())) {
                errors.push(format!("duplicate candidate/scenario/trial key {c}/{s}/{t}"));
            }
        }
    }
    for (path, record) in &loaded.records {
        if let Some(metrics) = record.get("metrics").and_then(Value::as_array) {
            for (i, metric) in metrics.iter().enumerate() {
                if let Some(inputs) = metric.get("input_metric_record_ids").and_then(Value::as_array) {
                    for input in inputs {
                        if let Some(id) = input.as_str() {
                            if !ids.contains(id) { errors.push(format!("{} metrics[{i}] references absent record {id}", path.display())); }
                        }
                    }
                }
            }
        }
    }
    errors.sort();
    errors.dedup();
    errors
}
fn metrics_by_id(v: &Value) -> BTreeMap<String, &Value> {
    let mut out = BTreeMap::new();
    if let Some(xs) = v.get("metrics").and_then(Value::as_array) {
        for metric in xs {
            if let Some(id) = metric.get("metric_id").and_then(Value::as_str) { out.insert(id.to_owned(), metric); }
        }
    }
    out
}
fn compare_records(records: &[Value], scenario: &str, reference: &str, candidate: &str, contracts: &Contracts) -> Value {
    let Some(primary) = contracts.scenario_metrics.get(scenario) else {
        return json!({"status":"blocked_unknown_scenario","scenario_id":scenario});
    };
    if !contracts.candidate_ids.contains(reference) || !contracts.candidate_ids.contains(candidate) {
        return json!({"status":"blocked_unknown_candidate","reference":reference,"candidate":candidate});
    }
    if reference == candidate { return json!({"status":"blocked_same_candidate","candidate":candidate}); }

    let mut refs = BTreeMap::<String, &Value>::new();
    let mut cands = BTreeMap::<String, &Value>::new();
    for record in records.iter().filter(|r| r.get("scenario_id").and_then(Value::as_str) == Some(scenario) && physically_eligible(r, contracts)) {
        let trial = record.get("trial_id").and_then(Value::as_str).unwrap_or("").to_owned();
        match record.get("candidate_id").and_then(Value::as_str) {
            Some(id) if id == reference => { refs.insert(trial, record); },
            Some(id) if id == candidate => { cands.insert(trial, record); },
            _ => {}
        }
    }
    let mut matched = Vec::new();
    let mut blocked = Vec::new();
    for (trial, r) in &refs {
        let Some(c) = cands.get(trial) else { continue; };
        let cond_match = r.get("conditions") == c.get("conditions");
        let protocol_match = r.pointer("/protocol/sha256") == c.pointer("/protocol/sha256");
        if cond_match && protocol_match { matched.push(trial.clone()); }
        else {
            blocked.push(json!({"trial_id":trial,"reason":if !cond_match {"conditions_mismatch"} else {"protocol_digest_mismatch"}}));
        }
    }
    let refs_indexed: BTreeMap<_,_> = refs.iter().map(|(t,r)|(t.clone(),metrics_by_id(r))).collect();
    let cands_indexed: BTreeMap<_,_> = cands.iter().map(|(t,r)|(t.clone(),metrics_by_id(r))).collect();
    let mut metric_results = Vec::new();
    for id in primary {
        let mut unit: Option<String> = None;
        let mut deltas = Vec::<f64>::new();
        let mut rows = Vec::new();
        let mut skipped = Vec::new();
        for trial in &matched {
            let (Some(l),Some(r)) = (
                refs_indexed.get(trial).and_then(|m|m.get(id)).copied(),
                cands_indexed.get(trial).and_then(|m|m.get(id)).copied()
            ) else {
                skipped.push(json!({"trial_id":trial,"reason":"metric_absent"})); continue;
            };
            if !matches!(l.get("metric_status").and_then(Value::as_str),Some("measured"|"derived"))
                || !matches!(r.get("metric_status").and_then(Value::as_str),Some("measured"|"derived")) {
                skipped.push(json!({"trial_id":trial,"reason":"metric_not_measured_or_derived"})); continue;
            }
            let lu = l.get("unit").and_then(Value::as_str).unwrap_or("");
            let ru = r.get("unit").and_then(Value::as_str).unwrap_or("");
            if lu != ru || unit.as_ref().is_some_and(|u|u != lu) {
                skipped.push(json!({"trial_id":trial,"reason":"unit_mismatch"})); continue;
            }
            unit = Some(lu.to_owned());
            let (Some(lv),Some(rv))=(l.get("value").and_then(Value::as_f64),r.get("value").and_then(Value::as_f64)) else {
                skipped.push(json!({"trial_id":trial,"reason":"non_numeric_metric_not_compared"})); continue;
            };
            if !lv.is_finite() || !rv.is_finite() {
                skipped.push(json!({"trial_id":trial,"reason":"non_finite_value"})); continue;
            }
            let delta = rv-lv;
            deltas.push(delta);
            rows.push(json!({"trial_id":trial,"reference_value":lv,"candidate_value":rv,"candidate_minus_reference":delta,"unit":lu}));
        }
        let mean = if deltas.is_empty() { Value::Null } else { json!(deltas.iter().sum::<f64>() / deltas.len() as f64) };
        metric_results.push(json!({
            "metric_id":id,"unit":unit,"paired_count":deltas.len(),"paired_values":rows,
            "mean_candidate_minus_reference":mean,"skipped_pairs":skipped,
            "interpretation":"Descriptive paired differences only; no ranking or generalization is implied."
        }));
    }
    json!({
        "tool":"kitchen-station-bench","version":VERSION,"report_kind":"paired_comparison",
        "status":if matched.is_empty() {"blocked_no_comparable_physical_pairs"} else {"descriptive_physical_comparison"},
        "scenario_id":scenario,"reference_candidate":reference,"candidate":candidate,
        "eligible_reference_trials":refs.len(),"eligible_candidate_trials":cands.len(),
        "matched_comparable_trials":matched,"blocked_pairs":blocked,"metrics":metric_results,
        "claim_limit":"Synthetic, simulation, literature and documentation evidence is excluded. This report does not establish superiority, sanitation, microbial safety, certification or external validity."
    })
}
fn markdown_report(v: &Value) -> String {
    let status = v.get("status").and_then(Value::as_str).unwrap_or("unknown");
    let kind = v.get("report_kind").and_then(Value::as_str).unwrap_or("report");
    let mut out = format!("# Kitchen Station Benchmark {kind}\n\nStatus: {status}\n\n");
    if let Some(n)=v.get("record_count").and_then(Value::as_u64) {
        out.push_str(&format!("Records loaded: {n}\n\n"));
        if let Some(counts)=v.get("records_by_run_status").and_then(Value::as_object) {
            out.push_str("## Run status counts\n\n");
            for (k,val) in counts { out.push_str(&format!("- {k}: {}\n",val.as_u64().unwrap_or(0))); }
            out.push('\n');
        }
        if let Some(es)=v.get("errors").and_then(Value::as_array) {
            out.push_str("## Contract errors\n\n");
            if es.is_empty(){out.push_str("No contract errors reported.\n\n");}
            else { for e in es {out.push_str(&format!("- {}\n",e.as_str().unwrap_or("invalid error")));} out.push('\n');}
        }
    }
    if let Some(ms)=v.get("metrics").and_then(Value::as_array) {
        out.push_str("## Metric comparison\n\n| Metric | Unit | Pairs | Mean candidate − reference | State |\n|---|---|---:|---:|---|\n");
        for m in ms {
            let id=m.get("metric_id").and_then(Value::as_str).unwrap_or("unknown");
            let unit=m.get("unit").and_then(Value::as_str).unwrap_or("—");
            let n=m.get("paired_count").and_then(Value::as_u64).unwrap_or(0);
            let d=m.get("mean_candidate_minus_reference").and_then(Value::as_f64).map(|x|format!("{x:.6}")).unwrap_or_else(||"not comparable".into());
            out.push_str(&format!("| {id} | {unit} | {n} | {d} | descriptive only |\n"));
        }
        out.push('\n');
    }
    if let Some(limit)=v.get("claim_limit").and_then(Value::as_str) {
        out.push_str("## Claim limit\n\n"); out.push_str(limit); out.push('\n');
    }
    out
}
fn option_value(args: &[String], key: &str, default: &str) -> String {
    args.windows(2).find(|w|w[0]==key).map(|w|w[1].clone()).unwrap_or_else(||default.to_owned())
}
fn positional_path(args: &[String]) -> PathBuf {
    let mut i=0;
    while i<args.len() {
        if args[i].starts_with("--") { i+=2; }
        else { return PathBuf::from(&args[i]); }
    }
    PathBuf::from("trial-records")
}
fn print_report(v: &Value, fmt: &str) -> Check<()> {
    match fmt {
        "json" => println!("{}",serde_json::to_string_pretty(v).map_err(|e|e.to_string())?),
        "markdown" | "md" => print!("{}",markdown_report(v)),
        other => return Err(format!("unknown format {other}; use json or markdown"))
    }
    Ok(())
}
fn run() -> Check<bool> {
    let args=env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0]=="--help" || args[0]=="-h" {
        println!("kitchen-station-bench {VERSION}\n\nCommands:\n  validate <records-dir> [--contracts docs/research] [--format json|markdown]\n  report <records-dir> [--contracts docs/research] [--format json|markdown]\n  compare <records-dir> --reference REF-0 --candidate P6-BATCH-WASH --scenario SINK-BATCH-FLOW-012 [--contracts docs/research] [--format json|markdown]\n\nRemote artifact URIs are not fetched. Relative artifact paths are checked locally.");
        return Ok(true);
    }
    let command=&args[0];
    let rest=&args[1..];
    let records_dir=positional_path(rest);
    let contracts_dir=PathBuf::from(option_value(rest,"--contracts","docs/research"));
    let fmt=option_value(rest,"--format","json");
    let reference=option_value(rest,"--reference","");
    let candidate=option_value(rest,"--candidate","");
    let scenario=option_value(rest,"--scenario","");
    let contracts=load_contracts(&contracts_dir)?;
    let loaded=read_records(&records_dir)?;
    let errors=validate_loaded(&loaded,&records_dir,&contracts);
    match command.as_str() {
        "validate" | "report" => {
            let mut report={
                let mut counts=BTreeMap::<String,u64>::new();
                let mut eligible=0_u64;
                for (_,r) in &loaded.records {
                    if let Some(s)=r.get("run_status").and_then(Value::as_str){*counts.entry(s.to_owned()).or_default()+=1;}
                    if physically_eligible(r, &contracts){eligible+=1;}
                }
                json!({
                    "tool":"kitchen-station-bench","version":VERSION,"report_kind":"validation",
                    "status":if loaded.records.is_empty(){"blocked_no_records"}else if !errors.is_empty(){"invalid"}else{"contract_valid"},
                    "record_count":loaded.records.len(),"records_by_run_status":counts,
                    "physically_eligible_records":eligible,"validation_errors":errors,
                    "claim_limit":"Contract validation is not physical performance, sanitation, certification or superiority evidence."
                })
            };
            // Keep report order deterministic via serde_json's sorted-key map.
            if report.get("record_count").and_then(Value::as_u64)==Some(0){report["status"]=json!("blocked_no_records");}
            report["contract_sha256"] = json!(contracts.contract_digests);
            report["record_file_sha256"] = json!(loaded.input_sha256);
            print_report(&report,&fmt)?;
            Ok(!loaded.records.is_empty() && errors.is_empty())
        }
        "compare" => {
            if !errors.is_empty() {
                let mut report=json!({"status":"blocked_invalid_input_records","errors":errors,"claim_limit":"Invalid input records cannot be compared."});
                report["contract_sha256"] = json!(contracts.contract_digests);
                report["record_file_sha256"] = json!(loaded.input_sha256);
                print_report(&report,&fmt)?;
                return Ok(false);
            }
            let records=loaded.records.iter().map(|(_,r)|r.clone()).collect::<Vec<_>>();
            let mut report=compare_records(&records,&scenario,&reference,&candidate,&contracts);
            report["contract_sha256"] = json!(contracts.contract_digests);
            report["record_file_sha256"] = json!(loaded.input_sha256);
            let okay=report.get("status").and_then(Value::as_str)==Some("descriptive_physical_comparison");
            print_report(&report,&fmt)?;
            Ok(okay)
        }
        other => Err(format!("unknown command {other}; run with --help"))
    }
}
fn main()->ExitCode {
    match run() {
        Ok(true)=>ExitCode::SUCCESS,
        Ok(false)=>ExitCode::FAILURE,
        Err(e)=>{eprintln!("kitchen-station-bench: {e}");ExitCode::FAILURE}
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn contracts()->Contracts{
        Contracts{
            candidate_ids:["REF-0","P1-DRAINBACK"].into_iter().map(str::to_owned).collect(),
            scenario_metrics:[("SINK-TEST-001".into(),["water_l","cleaning_endpoint"].into_iter().map(str::to_owned).collect())].into_iter().collect(),
            scenario_gate_ids:[("SINK-TEST-001".into(),["GATE-1".to_owned()].into_iter().collect())].into_iter().collect(),
            metric_ids:["water_l","cleaning_endpoint"].into_iter().map(str::to_owned).collect(),
            metric_unit_options:[
                ("water_l".to_owned(), ["L/task".to_owned()].into_iter().collect()),
                ("cleaning_endpoint".to_owned(), ["1".to_owned()].into_iter().collect())
            ].into_iter().collect(),
            metric_qualifying_values:[("cleaning_endpoint".to_owned(), ["pass".to_owned()].into_iter().collect())].into_iter().collect(),
            contract_digests:BTreeMap::new()
        }
    }
    fn artifact()->Value{json!({"uri":"https://example.invalid/data.json","sha256":"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","media_type":"application/json"})}
    fn measured(id:&str,value:Value,unit:&str)->Value{
        json!({"metric_id":id,"metric_status":"measured","value":value,"unit":unit,
            "uncertainty":{"kind":"standard","value":0.01,"unit":unit,"method":"repeatability"},
            "sample_count":3,"method_id":"METHOD-1","instrument_ids":["METER-1"],"artifact_refs":[artifact()]})
    }
    fn record()->Value{
        json!({
            "schema_version":"1.0.0","record_id":"REC-0001","candidate_id":"REF-0","scenario_id":"SINK-TEST-001","trial_id":"T-1",
            "run_status":"complete","evidence_class":"physical_observation","recorded_at":"2026-10-20T12:00:00Z",
            "conditions":{"pressure_kpa":30,"water_temp_c":20},
            "candidate_spec":{"revision":"A","sha256":"bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"},
            "protocol":{"version":"1.0","sha256":"cccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccccc"},
            "metrics":[measured("water_l",json!(3.0),"L/task"),measured("cleaning_endpoint",json!("pass"),"1")],
            "safety_gates":[{"gate_id":"GATE-1","status":"pass","basis":"test fixture"}],
            "raw_artifacts":[],"deviations":[],"exclusion_reason":null,"claim_limit":"Test fixture only, not a real trial."
        })
    }
    #[test] fn accepts_well_formed_record_contract(){let e=validate_record(&record(),"fixture",Path::new("."),&contracts());assert!(e.is_empty(),"{e:#?}");}
    #[test] fn rejects_missing_primary_metric(){let mut r=record();r["metrics"]=json!([measured("water_l",json!(3.0),"L/task")]);let e=validate_record(&r,"fixture",Path::new("."),&contracts());assert!(e.iter().any(|x|x.contains("required metric cleaning_endpoint absent")));}
    #[test] fn rejects_unknown_candidate(){let mut r=record();r["candidate_id"]=json!("MYSTERY");let e=validate_record(&r,"fixture",Path::new("."),&contracts());assert!(e.iter().any(|x|x.contains("unknown candidate")));}
    #[test] fn nonresult_metric_requires_reason_and_no_value(){let mut r=record();r["metrics"][0]=json!({"metric_id":"water_l","metric_status":"missing","value":3.0,"unit":"L/task"});let e=validate_record(&r,"fixture",Path::new("."),&contracts());assert!(e.iter().any(|x|x.contains("status_reason")));assert!(e.iter().any(|x|x.contains("non-result status")));}
    #[test] fn unknown_safety_gate_blocks_qualification(){let mut r=record();r["safety_gates"][0]["status"]=json!("unknown");assert!(!physically_eligible(&r, &contracts()));r["safety_gates"][0]["status"]=json!("pass");assert!(physically_eligible(&r, &contracts()));}
    #[test] fn synthetic_evidence_is_not_physically_eligible(){let mut r=record();r["evidence_class"]=json!("synthetic_fixture");assert!(!physically_eligible(&r, &contracts()));}
    #[test] fn blocked_run_requires_explicit_reason_and_blocked_metric_statuses(){let mut r=record();r["run_status"]=json!("blocked");r["exclusion_reason"]=json!("meter unavailable");r["metrics"]=json!([
        {"metric_id":"water_l","metric_status":"blocked","unit":"L/task","status_reason":"meter unavailable"},
        {"metric_id":"cleaning_endpoint","metric_status":"blocked","unit":"1","status_reason":"trial could not start"}]);let e=validate_record(&r,"fixture",Path::new("."),&contracts());assert!(e.is_empty(),"{e:#?}");}
    #[test] fn rejects_unknown_record_key(){let mut r=record();r["magic_pass"]=json!(true);let e=validate_record(&r,"fixture",Path::new("."),&contracts());assert!(e.iter().any(|x|x.contains("unknown key magic_pass")));}
    #[test] fn sha256_requires_exact_hex_digest(){assert!(valid_sha256(&"a".repeat(64)));assert!(!valid_sha256(&"a".repeat(63)));assert!(!valid_sha256(&"g".repeat(64)));}
    #[test] fn synthetic_only_records_cannot_be_compared(){let mut l=record();let mut r=record();l["trial_id"]=json!("T-1");r["trial_id"]=json!("T-1");r["candidate_id"]=json!("P1-DRAINBACK");r["evidence_class"]=json!("synthetic_fixture");let report=compare_records(&[l,r],"SINK-TEST-001","REF-0","P1-DRAINBACK",&contracts());assert_eq!(report["status"].as_str(),Some("blocked_no_comparable_physical_pairs"));}
    #[test] fn mismatched_conditions_block_pair(){let mut l=record();let mut r=record();l["trial_id"]=json!("T-1");r["trial_id"]=json!("T-1");r["candidate_id"]=json!("P1-DRAINBACK");r["conditions"]["pressure_kpa"]=json!(35);let report=compare_records(&[l,r],"SINK-TEST-001","REF-0","P1-DRAINBACK",&contracts());assert_eq!(report["status"].as_str(),Some("blocked_no_comparable_physical_pairs"));}
    #[test] fn paired_delta_is_candidate_minus_reference(){let mut l=record();let mut r=record();l["trial_id"]=json!("T-1");r["trial_id"]=json!("T-1");r["candidate_id"]=json!("P1-DRAINBACK");r["metrics"][0]["value"]=json!(2.5);let report=compare_records(&[l,r],"SINK-TEST-001","REF-0","P1-DRAINBACK",&contracts());assert_eq!(report["status"].as_str(),Some("descriptive_physical_comparison"));let m=report["metrics"].as_array().unwrap().iter().find(|x|x["metric_id"]=="water_l").unwrap();assert_eq!(m["mean_candidate_minus_reference"].as_f64(),Some(-0.5));}
    #[test]
    fn repository_contracts_cover_all_primary_metrics() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/research");
        let loaded = load_contracts(&root).expect("repository contracts should be mutually consistent");
        assert!(loaded.scenario_metrics.len() >= 13);
        assert!(loaded.candidate_ids.len() >= 8);
        assert!(loaded.metric_ids.len() >= 67);
        assert_eq!(loaded.metric_unit_options.len(), loaded.metric_ids.len());
        assert!(loaded.metric_unit_options.values().all(|opts| !opts.is_empty()));
        assert!(loaded.metric_qualifying_values.contains_key("cleaning_endpoint"));
        assert_eq!(loaded.contract_digests.len(), 4);
        assert!(loaded.contract_digests.values().all(|h| valid_sha256(h)));
        assert!(loaded.scenario_metrics.contains_key("SINK-PRERINSE-FORCE-013"));
        assert!(loaded.scenario_gate_ids.len() >= 13);
        assert!(loaded.scenario_gate_ids.values().all(|g| !g.is_empty()));
        assert!(loaded.candidate_ids.contains("P7-COMMERCIAL-PRERINSE"));
    }

    #[test]
    fn record_with_missing_primary_endpoint_is_not_physically_eligible() {
        let mut r = record();
        r["metrics"][1]["metric_status"] = json!("blocked");
        r["metrics"][1]["value"] = Value::Null;
        r["metrics"][1]["status_reason"] = json!("endpoint measurement unavailable");
        assert!(!physically_eligible(&r, &contracts()));
    }

    #[test]
    fn rejects_unit_not_allowed_by_metric_registry() {
        let mut r = record();
        r["metrics"][0]["unit"] = json!("bananas");
        let errors = validate_record(&r, "fixture", Path::new("."), &contracts());
        assert!(errors.iter().any(|e| e.contains("unit bananas is not one of registered options")));
    }

    #[test]
    fn missing_required_safety_gate_blocks_record_and_physical_eligibility() {
        let mut r = record();
        r["safety_gates"] = json!([]);
        let errors = validate_record(&r, "fixture", Path::new("."), &contracts());
        assert!(errors.iter().any(|e| e.contains("required safety gate GATE-1 is missing")));
        assert!(!physically_eligible(&r, &contracts()));
    }

    #[test]
    fn unknown_required_safety_gate_blocks_physical_eligibility_even_if_others_pass() {
        let mut r = record();
        r["safety_gates"] = json!([
            {"gate_id":"GATE-1","status":"pass","basis":"test"},
            {"gate_id":"GATE-2","status":"unknown","basis":"unregistered"}
        ]);
        assert!(!physically_eligible(&r, &contracts()));
    }

    #[test]
    fn live_scenarios_declare_unique_required_safety_gates() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/research");
        let contracts = load_contracts(&root).expect("valid live contracts");
        for (scenario, gates) in contracts.scenario_gate_ids {
            assert!(!gates.is_empty(), "{scenario} must specify required safety gates");
            assert!(gates.iter().all(|g| g.starts_with("GATE-")), "{scenario} gate IDs need namespace");
        }
    }

    #[test]
    fn failed_categorical_cleaning_endpoint_blocks_eligibility_even_if_gates_pass() {
        let mut r = record();
        r["metrics"][1]["value"] = json!("fail");
        assert!(!physically_eligible(&r, &contracts()));
    }

    #[test]
    fn qualifying_categorical_value_is_enforced_from_registry() {
        let mut r = record();
        r["metrics"][1]["value"] = json!("pass");
        assert!(physically_eligible(&r, &contracts()));
        r["metrics"][1]["value"] = json!("met");
        assert!(!physically_eligible(&r, &contracts()));
    }

}
