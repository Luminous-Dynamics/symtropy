use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const CORPUS_SCHEMA: &str = "personal-family-vehicle-scenario-corpus/0.1";
const CORPUS_STATUS: &str = "synthetic-research-fixture-not-validated-physics";
const REQUIRED_IDS: [&str; 21] = [
    "PV01_NOMINAL_DRY",
    "PV02_WET_LOW_MU",
    "PV03_FRICTION_TRANSITION",
    "PV04_SPLIT_MU",
    "PV05_PAYLOAD_CG_SHIFT",
    "PV06_OVERLOAD",
    "PV07_BRAKE_FADE",
    "PV08_REGEN_FRICTION_HANDOFF",
    "PV09_ROUGH_ROAD",
    "PV10_WHEEL_UNLOADING",
    "PV11_TIRE_REVISION_CHANGE",
    "PV12_STEERING_DEGRADATION",
    "PV13_SENSOR_BIAS_STALE_CALIBRATION",
    "PV14_POWERTRAIN_DERATING",
    "PV15_OVERDUE_MAINTENANCE",
    "PV16_INCOMPATIBLE_PART",
    "PV17_INVALID_UNITS",
    "PV18_MISSING_PROVENANCE",
    "PV19_SOLVER_IDENTITY_MUTATION",
    "PV20_NONCONVERGED_SOLVER",
    "PV21_OUT_OF_DOMAIN",
];

const REQUIRED_NEGATIVE_IDS: [&str; 9] = [
    "PV06_OVERLOAD",
    "PV11_TIRE_REVISION_CHANGE",
    "PV13_SENSOR_BIAS_STALE_CALIBRATION",
    "PV16_INCOMPATIBLE_PART",
    "PV17_INVALID_UNITS",
    "PV18_MISSING_PROVENANCE",
    "PV19_SOLVER_IDENTITY_MUTATION",
    "PV20_NONCONVERGED_SOLVER",
    "PV21_OUT_OF_DOMAIN",
];
const REQUIRED_SOURCE_IDS: [&str; 10] = [
    "SRC_EPA_DYNAMOMETER_SCHEDULES",
    "SRC_NHTSA_ADS_SCENARIO_FRAMEWORK",
    "SRC_SAE_J1555_201910",
    "SRC_ISO_21448_2022",
    "SRC_ISO_26262_PART_3_2018",
    "SRC_SA_MIB_REGISTRATION",
    "SRC_SA_NRCs_LETTER_OF_AUTHORITY",
    "SRC_SA_ROADWORTHINESS",
    "SRC_UN_R155",
    "SRC_UN_R156",
];

type AuditResult<T> = Result<T, Vec<String>>;

#[derive(Debug, PartialEq, Eq)]
struct ScenarioSummary {
    case_count: usize,
    negative_control_count: usize,
    sha256: String,
}

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    case_count: usize,
    negative_control_count: usize,
    sha256: String,
    source_registry_count: usize,
    source_registry_sha256: String,
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn case_by_id<'a>(cases: &'a [Value], id: &str) -> Option<&'a Value> {
    cases.iter().find(|case| string_at(case, "id") == Some(id))
}

#[cfg(test)]
fn case_by_id_mut<'a>(cases: &'a mut [Value], id: &str) -> Option<&'a mut Value> {
    cases.iter_mut().find(|case| string_at(case, "id") == Some(id))
}

fn has_invariant(case: &Value, invariant: &str) -> bool {
    case.get("required_invariants")
        .and_then(Value::as_array)
        .is_some_and(|values| values.iter().any(|value| value.as_str() == Some(invariant)))
}

fn check_exact_keys(value: &Value, expected: &[&str], at: &str, errors: &mut Vec<String>) {
    let Some(object) = value.as_object() else {
        errors.push(format!("{at}: expected an object"));
        return;
    };

    let expected_set: BTreeSet<&str> = expected.iter().copied().collect();
    let actual_set: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected_set.difference(&actual_set).copied().collect();
    let unknown: Vec<_> = actual_set.difference(&expected_set).copied().collect();

    if !missing.is_empty() || !unknown.is_empty() {
        errors.push(format!(
            "{at}: missing keys {missing:?}; unexpected keys {unknown:?}"
        ));
    }
}

fn require_string(value: &Value, key: &str, at: &str, errors: &mut Vec<String>) -> Option<String> {
    match string_at(value, key) {
        Some(text) if !text.trim().is_empty() => Some(text.to_owned()),
        Some(_) => {
            errors.push(format!("{at}.{key}: must not be empty"));
            None
        }
        None => {
            errors.push(format!("{at}.{key}: missing or not a string"));
            None
        }
    }
}

fn validate_special_case(case: &Value, errors: &mut Vec<String>) {
    let Some(id) = string_at(case, "id") else {
        return;
    };
    let inputs = case.get("inputs").unwrap_or(&Value::Null);
    let disposition = string_at(case, "expected_disposition").unwrap_or("");

    match id {
        "PV01_NOMINAL_DRY" => {
            if string_at(case, "kind") != Some("positive_control") {
                errors.push("PV01_NOMINAL_DRY: must remain the positive control".to_owned());
            }
        }
        "PV03_FRICTION_TRANSITION" => {
            let before = number_at(inputs, "road_mu_before");
            let after = number_at(inputs, "road_mu_after");
            if !matches!((before, after), (Some(a), Some(b)) if a > b && b >= 0.0 && a <= 1.0) {
                errors.push("PV03_FRICTION_TRANSITION: must declare a valid friction drop".to_owned());
            }
        }
        "PV05_PAYLOAD_CG_SHIFT" => {
            let payload = number_at(inputs, "payload_kg");
            if !matches!(payload, Some(p) if p >= 0.0 && p <= 400.0) {
                errors.push("PV05_PAYLOAD_CG_SHIFT: payload must remain within the synthetic 400 kg envelope".to_owned());
            }
            if number_at(inputs, "payload_cg_z_m").is_none() {
                errors.push("PV05_PAYLOAD_CG_SHIFT: center-of-gravity height is required".to_owned());
            }
        }
        "PV06_OVERLOAD" => {
            let payload = number_at(inputs, "payload_kg");
            let limit = number_at(inputs, "payload_profile_limit_kg");
            if !matches!((payload, limit), (Some(p), Some(l)) if p > l) || disposition != "reject_before_simulation" {
                errors.push("PV06_OVERLOAD: overload must exceed limit and be rejected before simulation".to_owned());
            }
        }
        "PV11_TIRE_REVISION_CHANGE" => {
            if string_at(inputs, "scenario_tire_revision") == string_at(inputs, "calibrated_tire_revision")
                || disposition != "reject_as_mismatched_configuration"
            {
                errors.push("PV11_TIRE_REVISION_CHANGE: mismatched tire revisions must be rejected".to_owned());
            }
        }
        "PV13_SENSOR_BIAS_STALE_CALIBRATION" => {
            let age = number_at(inputs, "calibration_age_days");
            let max_age = number_at(inputs, "max_calibration_age_days");
            if !matches!((age, max_age), (Some(a), Some(m)) if a > m) || disposition != "insufficient_evidence" {
                errors.push("PV13_SENSOR_BIAS_STALE_CALIBRATION: stale calibration must cause abstention".to_owned());
            }
        }
        "PV16_INCOMPATIBLE_PART" => {
            let installed = string_at(inputs, "installed_part_revision");
            let allowed = inputs.get("allowed_part_revisions").and_then(Value::as_array);
            let is_compatible = allowed.is_some_and(|values| values.iter().any(|value| value.as_str() == installed));
            if is_compatible || disposition != "reject_as_mismatched_configuration" {
                errors.push("PV16_INCOMPATIBLE_PART: incompatible part revision must not be accepted".to_owned());
            }
        }
        "PV17_INVALID_UNITS" => {
            let speed_unit = inputs.get("speed").and_then(|v| v.get("unit")).and_then(Value::as_str);
            if speed_unit != Some("horsepower") || disposition != "reject_before_simulation" {
                errors.push("PV17_INVALID_UNITS: invalid speed units must be rejected before simulation".to_owned());
            }
        }
        "PV18_MISSING_PROVENANCE" => {
            let solver = string_at(inputs, "solver_id");
            let source_is_null = inputs.get("parameter_source_ref").is_some_and(Value::is_null);
            if solver != Some("") || !source_is_null || disposition != "insufficient_evidence" {
                errors.push("PV18_MISSING_PROVENANCE: missing solver/source must remain insufficient evidence".to_owned());
            }
        }
        "PV19_SOLVER_IDENTITY_MUTATION" => {
            let manifest_solver = string_at(inputs, "manifest_solver_version");
            let result_solver = string_at(inputs, "result_solver_version");
            if manifest_solver == result_solver || disposition != "reject_as_tampered_evidence" {
                errors.push("PV19_SOLVER_IDENTITY_MUTATION: solver provenance mismatch must be rejected".to_owned());
            }
        }
        "PV20_NONCONVERGED_SOLVER" => {
            if string_at(inputs, "solver_status") != Some("non_converged")
                || disposition != "simulation_failure_not_promoted"
            {
                errors.push("PV20_NONCONVERGED_SOLVER: non-converged result must not be promoted".to_owned());
            }
        }
        "PV21_OUT_OF_DOMAIN" => {
            let speed = number_at(inputs, "speed_mps");
            let max_speed = number_at(inputs, "validated_speed_max_mps");
            let temperature = number_at(inputs, "ambient_temperature_c");
            let max_temperature = number_at(inputs, "validated_temperature_max_c");
            let out_of_domain = matches!((speed, max_speed), (Some(s), Some(m)) if s > m)
                || matches!((temperature, max_temperature), (Some(t), Some(m)) if t > m);
            if !out_of_domain || disposition != "insufficient_evidence" {
                errors.push("PV21_OUT_OF_DOMAIN: out-of-domain input must result in abstention".to_owned());
            }
        }
        _ => {}
    }
}

fn validate_corpus(document: &Value, raw_bytes: &[u8]) -> AuditResult<ScenarioSummary> {
    let mut errors = Vec::new();
    check_exact_keys(
        document,
        &[
            "schema_version",
            "corpus_id",
            "created_date",
            "status",
            "vehicle_profile",
            "status_semantics",
            "claim_ceiling",
            "cases",
        ],
        "corpus",
        &mut errors,
    );

    if string_at(document, "schema_version") != Some(CORPUS_SCHEMA) {
        errors.push(format!("corpus.schema_version: expected {CORPUS_SCHEMA}"));
    }
    if string_at(document, "status") != Some(CORPUS_STATUS) {
        errors.push(format!("corpus.status: synthetic/non-validated status must be preserved"));
    }
    if string_at(document, "corpus_id") != Some("symtropy-personal-family-road-vehicle-synthetic-v0.1") {
        errors.push("corpus.corpus_id: unexpected corpus identity".to_owned());
    }

    let Some(profile) = document.get("vehicle_profile") else {
        errors.push("corpus.vehicle_profile: missing".to_owned());
        return Err(errors);
    };
    if string_at(profile, "id") != Some("family-road-vehicle-synthetic-v0.1") {
        errors.push("vehicle_profile.id: unexpected profile identity".to_owned());
    }
    let assumptions = profile.get("assumptions").and_then(Value::as_array);
    if !assumptions.is_some_and(|values| values.len() >= 3 && values.iter().all(Value::is_string)) {
        errors.push("vehicle_profile.assumptions: at least three explicit assumptions are required".to_owned());
    }

    let semantics = document.get("status_semantics").and_then(Value::as_object);
    if !semantics.is_some_and(|values| !values.is_empty()) {
        errors.push("status_semantics: expected non-empty object".to_owned());
    }

    let claims = document.get("claim_ceiling").and_then(Value::as_array);
    if !claims.is_some_and(|values| {
        !values.is_empty()
            && values.iter().all(Value::is_string)
            && values.iter().filter_map(Value::as_str).any(|value| value.contains("No case establishes physical braking"))
    }) {
        errors.push("claim_ceiling: explicit physical-evidence limit is required".to_owned());
    }

    let Some(cases) = document.get("cases").and_then(Value::as_array) else {
        errors.push("cases: expected array".to_owned());
        return Err(errors);
    };
    if cases.len() != REQUIRED_IDS.len() {
        errors.push(format!("cases: expected {} entries, found {}", REQUIRED_IDS.len(), cases.len()));
    }

    let mut seen_ids = BTreeSet::new();
    let mut negative_count = 0usize;
    for (index, case) in cases.iter().enumerate() {
        let at = format!("cases[{index}]");
        check_exact_keys(
            case,
            &["id", "kind", "description", "inputs", "required_invariants", "expected_disposition"],
            &at,
            &mut errors,
        );

        let Some(id) = require_string(case, "id", &at, &mut errors) else {
            continue;
        };
        if !seen_ids.insert(id.clone()) {
            errors.push(format!("{at}: duplicate case id {id}"));
        }
        if !REQUIRED_IDS.contains(&id.as_str()) {
            errors.push(format!("{at}: unknown case id {id}"));
        }
        if require_string(case, "description", &at, &mut errors).is_none() {
            continue;
        }
        let Some(kind) = require_string(case, "kind", &at, &mut errors) else {
            continue;
        };
        if !["positive_control", "stress_case", "negative_control"].contains(&kind.as_str()) {
            errors.push(format!("{at}: unknown case kind {kind}"));
        }
        if kind == "negative_control" {
            negative_count += 1;
            if !REQUIRED_NEGATIVE_IDS.contains(&id.as_str()) {
                errors.push(format!("{at}: unregistered negative control {id}"));
            }
        } else if REQUIRED_NEGATIVE_IDS.contains(&id.as_str()) {
            errors.push(format!("{at}: required negative control {id} was weakened"));
        }

        let input_ok = case.get("inputs").is_some_and(Value::is_object);
        if !input_ok {
            errors.push(format!("{at}.inputs: expected object"));
        }
        match case.get("required_invariants").and_then(Value::as_array) {
            Some(invariants) if !invariants.is_empty() => {
                let mut invariant_set = BTreeSet::new();
                for invariant in invariants {
                    match invariant.as_str() {
                        Some(text) if !text.trim().is_empty() => {
                            if !invariant_set.insert(text) {
                                errors.push(format!("{at}: duplicate invariant {text}"));
                            }
                        }
                        _ => errors.push(format!("{at}: every invariant must be a non-empty string")),
                    }
                }
            }
            _ => errors.push(format!("{at}.required_invariants: must be a non-empty array")),
        }

        if let Some(disposition) = string_at(case, "expected_disposition") {
            if !semantics.is_some_and(|values| values.contains_key(disposition)) {
                errors.push(format!("{at}: disposition {disposition} has no declared semantics"));
            }
        } else {
            errors.push(format!("{at}.expected_disposition: missing or not a string"));
        }
        validate_special_case(case, &mut errors);
    }

    for required_id in REQUIRED_IDS {
        if !seen_ids.contains(required_id) {
            errors.push(format!("cases: missing required case {required_id}"));
        }
    }
    if negative_count != REQUIRED_NEGATIVE_IDS.len() {
        errors.push(format!(
            "cases: expected {} negative controls, found {negative_count}",
            REQUIRED_NEGATIVE_IDS.len()
        ));
    }

    for (id, invariant) in [
        ("PV01_NOMINAL_DRY", "does_not_emit_physical_safety_claim"),
        ("PV06_OVERLOAD", "does_not_clamp_overload_silently"),
        ("PV17_INVALID_UNITS", "does_not_coerce_unknown_units"),
        ("PV20_NONCONVERGED_SOLVER", "retains_failure_receipt"),
        ("PV21_OUT_OF_DOMAIN", "does_not_extrapolate_silently"),
    ] {
        if let Some(case) = case_by_id(cases, id) {
            if !has_invariant(case, invariant) {
                errors.push(format!("{id}: required invariant {invariant} was removed"));
            }
        }
    }

    if errors.is_empty() {
        let sha256 = format!("{:x}", Sha256::digest(raw_bytes));
        Ok(ScenarioSummary {
            case_count: cases.len(),
            negative_control_count: negative_count,
            sha256,
        })
    } else {
        Err(errors)
    }
}


fn validate_source_registry(document: &Value, raw_bytes: &[u8]) -> AuditResult<(usize, String)> {
    let mut errors = Vec::new();
    check_exact_keys(
        document,
        &[
            "schema_version",
            "registry_id",
            "created_date",
            "status",
            "policy",
            "sources",
            "next_evidence_gates",
            "claim_ceiling",
        ],
        "source_registry",
        &mut errors,
    );
    if string_at(document, "schema_version") != Some("vehicle-benchmark-source-registry/0.1") {
        errors.push("source_registry.schema_version: unexpected version".to_owned());
    }
    if string_at(document, "registry_id") != Some("symtropy-personal-family-vehicle-source-registry-v0.1") {
        errors.push("source_registry.registry_id: unexpected identity".to_owned());
    }
    if string_at(document, "status") != Some("source inventory only; rights/reuse and applicability require review") {
        errors.push("source_registry.status: rights/reuse review status must remain explicit".to_owned());
    }
    if !document.get("policy").is_some_and(|policy| {
        policy.get("no_vendoring_without_rights_review").and_then(Value::as_bool) == Some(true)
            && policy.get("rules").and_then(Value::as_array).is_some_and(|rules| rules.len() >= 4)
    }) {
        errors.push("source_registry.policy: rights gate and source-claim rules are required".to_owned());
    }

    let Some(sources) = document.get("sources").and_then(Value::as_array) else {
        return Err(vec!["source_registry.sources: expected array".to_owned()]);
    };
    if sources.len() != REQUIRED_SOURCE_IDS.len() {
        errors.push(format!(
            "source_registry.sources: expected {} entries, found {}",
            REQUIRED_SOURCE_IDS.len(),
            sources.len()
        ));
    }
    let mut seen = BTreeSet::new();
    for (index, source) in sources.iter().enumerate() {
        let at = format!("source_registry.sources[{index}]");
        let Some(id) = string_at(source, "id") else {
            errors.push(format!("{at}.id: missing or not a string"));
            continue;
        };
        if !seen.insert(id.to_owned()) {
            errors.push(format!("{at}: duplicate source id {id}"));
        }
        if !REQUIRED_SOURCE_IDS.contains(&id) {
            errors.push(format!("{at}: unregistered source id {id}"));
        }
        for key in ["title", "organization", "accessed_on", "role", "evidence_class", "status"] {
            if string_at(source, key).is_none_or(|value| value.trim().is_empty()) {
                errors.push(format!("{at}.{key}: missing or blank"));
            }
        }
        if !string_at(source, "url").is_some_and(|url| url.starts_with("https://")) {
            errors.push(format!("{at}.url: source must use HTTPS"));
        }
        for key in ["usable_for", "not_usable_for", "rights_and_provenance"] {
            let values = source.get(key).and_then(Value::as_array);
            if !values.is_some_and(|values| {
                !values.is_empty()
                    && values.iter().all(|value| value.as_str().is_some_and(|text| !text.trim().is_empty()))
            }) {
                errors.push(format!("{at}.{key}: explicit non-empty string list required"));
            }
        }
    }
    for id in REQUIRED_SOURCE_IDS {
        if !seen.contains(id) {
            errors.push(format!("source_registry: missing required source {id}"));
        }
    }
    if !document.get("next_evidence_gates").and_then(Value::as_array).is_some_and(|items| items.len() >= 5) {
        errors.push("source_registry.next_evidence_gates: at least five gates are required".to_owned());
    }
    let claims = document.get("claim_ceiling").and_then(Value::as_array);
    if !claims.is_some_and(|items| {
        items.iter().filter_map(Value::as_str).any(|text| text.contains("not a dataset license register"))
            && items.iter().filter_map(Value::as_str).any(|text| text.contains("contains no measured vehicle data"))
    }) {
        errors.push("source_registry.claim_ceiling: source/measurement limits must be explicit".to_owned());
    }

    if errors.is_empty() {
        Ok((sources.len(), format!("{:x}", Sha256::digest(raw_bytes))))
    } else {
        Err(errors)
    }
}

fn default_corpus_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/scenario_corpus_v0_1.json")
}

fn default_registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/benchmark_source_registry_v0_1.json")
}

fn read_cli() -> Result<(PathBuf, PathBuf), String> {
    let mut args = env::args_os().skip(1);
    let mut corpus_path = None;
    let mut registry_path = None;
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!("Usage: personal-vehicle-corpus-audit [--corpus PATH] [--registry PATH]");
            println!("Checks the synthetic scenario corpus and source-registry contract only; it does not run vehicle physics.");
            std::process::exit(0);
        } else if arg == "--corpus" {
            let Some(value) = args.next() else {
                return Err("--corpus requires a path".to_owned());
            };
            corpus_path = Some(PathBuf::from(value));
        } else if arg == "--registry" {
            let Some(value) = args.next() else {
                return Err("--registry requires a path".to_owned());
            };
            registry_path = Some(PathBuf::from(value));
        } else {
            return Err(format!("unknown argument: {}", arg.to_string_lossy()));
        }
    }
    Ok((
        corpus_path.unwrap_or_else(default_corpus_path),
        registry_path.unwrap_or_else(default_registry_path),
    ))
}

fn main() -> ExitCode {
    let (corpus_path, registry_path) = match read_cli() {
        Ok(paths) => paths,
        Err(error) => {
            eprintln!("CORPUS_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };
    let raw = match fs::read(&corpus_path) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("CORPUS_AUDIT_FAIL: cannot read {}: {error}", corpus_path.display());
            return ExitCode::from(2);
        }
    };
    let document: Value = match serde_json::from_slice(&raw) {
        Ok(document) => document,
        Err(error) => {
            eprintln!("CORPUS_AUDIT_FAIL: malformed scenario JSON: {error}");
            return ExitCode::from(2);
        }
    };
    let registry_raw = match fs::read(&registry_path) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("CORPUS_AUDIT_FAIL: cannot read {}: {error}", registry_path.display());
            return ExitCode::from(2);
        }
    };
    let registry: Value = match serde_json::from_slice(&registry_raw) {
        Ok(registry) => registry,
        Err(error) => {
            eprintln!("CORPUS_AUDIT_FAIL: malformed source registry JSON: {error}");
            return ExitCode::from(2);
        }
    };

    let mut errors = Vec::new();
    let summary = match validate_corpus(&document, &raw) {
        Ok(summary) => Some(summary),
        Err(corpus_errors) => {
            errors.extend(corpus_errors);
            None
        }
    };
    let registry_result = match validate_source_registry(&registry, &registry_raw) {
        Ok(result) => Some(result),
        Err(registry_errors) => {
            errors.extend(registry_errors);
            None
        }
    };

    if errors.is_empty() {
        let summary = summary.expect("summary exists when validation has no errors");
        let (source_registry_count, source_registry_sha256) =
            registry_result.expect("registry result exists when validation has no errors");
        let report = AuditSummary {
            case_count: summary.case_count,
            negative_control_count: summary.negative_control_count,
            sha256: summary.sha256,
            source_registry_count,
            source_registry_sha256,
        };
        println!(
            "CORPUS_AUDIT_PASS cases={} negative_controls={} corpus_sha256={} source_registry_entries={} source_registry_sha256={}",
            report.case_count, report.negative_control_count, report.sha256,
            report.source_registry_count, report.source_registry_sha256
        );
        println!("CLAIM_CEILING synthetic scenario/registry structure only; no physics, licensing or safety result");
        ExitCode::SUCCESS
    } else {
        eprintln!("CORPUS_AUDIT_FAIL errors={}", errors.len());
        for error in errors {
            eprintln!("- {error}");
        }
        ExitCode::from(1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn corpus() -> Value {
        serde_json::from_str(include_str!("../../../fixtures/personal_vehicle/scenario_corpus_v0_1.json"))
            .expect("repository corpus should parse as JSON")
    }

    fn validate_fixture(document: &Value) -> Result<ScenarioSummary, Vec<String>> {
        let bytes = serde_json::to_vec(document).expect("fixture serialization should succeed");
        validate_corpus(document, &bytes)
    }

    fn source_registry() -> Value {
        serde_json::from_str(include_str!("../../../fixtures/personal_vehicle/benchmark_source_registry_v0_1.json"))
            .expect("source registry should parse as JSON")
    }

    fn validate_source_registry_fixture(document: &Value) -> Result<(usize, String), Vec<String>> {
        let bytes = serde_json::to_vec(document).expect("registry serialization should succeed");
        validate_source_registry(document, &bytes)
    }

    #[test]
    fn source_registry_has_complete_reviewed_census() {
        let registry = source_registry();
        let (count, digest) = validate_source_registry_fixture(&registry).expect("source registry contract");
        assert_eq!(count, 10);
        assert_eq!(digest.len(), 64);
    }

    #[test]
    fn source_registry_cannot_hide_reuse_limits() {
        let mut registry = source_registry();
        registry["sources"][0].as_object_mut().expect("source object").remove("not_usable_for");
        assert!(validate_source_registry_fixture(&registry).is_err());
    }

    #[test]
    fn duplicate_source_ids_are_rejected() {
        let mut registry = source_registry();
        registry["sources"][1]["id"] = registry["sources"][0]["id"].clone();
        assert!(validate_source_registry_fixture(&registry).is_err());
    }

    #[test]
    fn frozen_corpus_is_valid_and_has_expected_census() {
        let result = validate_fixture(&corpus()).expect("frozen corpus should satisfy its contract");
        assert_eq!(result.case_count, 21);
        assert_eq!(result.negative_control_count, 9);
        assert_eq!(result.sha256.len(), 64);
    }

    #[test]
    fn duplicate_case_identity_is_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        cases[1]["id"] = cases[0]["id"].clone();
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn overload_cannot_be_promoted_to_eligible() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV06_OVERLOAD").expect("overload case")["expected_disposition"] =
            Value::String("eligible_for_model_evaluation".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn out_of_domain_case_must_abstain() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV21_OUT_OF_DOMAIN").expect("out-of-domain case")["expected_disposition"] =
            Value::String("eligible_for_model_evaluation".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn solver_identity_mutation_must_be_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV19_SOLVER_IDENTITY_MUTATION")
            .expect("solver mutation case")["expected_disposition"] =
            Value::String("eligible_for_model_evaluation".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn missing_safety_claim_boundary_is_rejected() {
        let mut document = corpus();
        document["claim_ceiling"] = serde_json::json!(["generic test data only"]);
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn weakened_negative_control_classification_is_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV17_INVALID_UNITS").expect("units case")["kind"] =
            Value::String("stress_case".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn invalid_speed_unit_is_rejected_if_expected_disposition_changes() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV17_INVALID_UNITS").expect("units case")["inputs"]["speed"]["unit"] =
            Value::String("m/s".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn missing_invariant_is_rejected() {
        let mut document = corpus();
        let cases = document["cases"].as_array_mut().expect("cases array");
        case_by_id_mut(cases, "PV21_OUT_OF_DOMAIN").expect("out-of-domain case")["required_invariants"] =
            serde_json::json!(["detects_out_of_domain"]);
        assert!(validate_fixture(&document).is_err());
    }
}
