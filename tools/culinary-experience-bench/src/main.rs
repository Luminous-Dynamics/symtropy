use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const VERSION: &str = env!("CARGO_PKG_VERSION");
type Check<T> = Result<T, String>;

#[derive(Debug)]
struct ContractFiles {
    scenarios: Value,
    metrics: Value,
    digests: BTreeMap<String, String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_json_digest(path: &Path) -> Check<(Value, String)> {
    let bytes = fs::read(path).map_err(|e| format!("{}: {e}", path.display()))?;
    let digest = sha256_hex(&bytes);
    let value = serde_json::from_slice::<Value>(&bytes)
        .map_err(|e| format!("{}: invalid JSON: {e}", path.display()))?;
    Ok((value, digest))
}

fn load_contracts(dir: &Path) -> Check<ContractFiles> {
    let scenarios_path = dir.join("CULINARY_EXPERIENCE_SCENARIOS_V1.json");
    let metrics_path = dir.join("CULINARY_EXPERIENCE_METRICS_V1.json");
    let (scenarios, scenarios_digest) = read_json_digest(&scenarios_path)?;
    let (metrics, metrics_digest) = read_json_digest(&metrics_path)?;
    let digests = [
        ("CULINARY_EXPERIENCE_SCENARIOS_V1.json".to_owned(), scenarios_digest),
        ("CULINARY_EXPERIENCE_METRICS_V1.json".to_owned(), metrics_digest),
    ].into_iter().collect();
    Ok(ContractFiles { scenarios, metrics, digests })
}

fn string_field<'a>(v: &'a Value, field: &str) -> Option<&'a str> {
    v.get(field).and_then(Value::as_str)
}

fn strings_array(v: &Value, field: &str, at: &str, errors: &mut Vec<String>) -> Vec<String> {
    match v.get(field).and_then(Value::as_array) {
        Some(values) => {
            let mut out = Vec::new();
            for (index, value) in values.iter().enumerate() {
                match value.as_str() {
                    Some(s) if !s.trim().is_empty() => out.push(s.to_owned()),
                    _ => errors.push(format!("{at}.{field}[{index}] must be a non-empty string")),
                }
            }
            out
        }
        None => {
            errors.push(format!("{at}.{field} must be an array"));
            Vec::new()
        }
    }
}

fn check_nonempty_string(v: &Value, key: &str, at: &str, errors: &mut Vec<String>) {
    match string_field(v, key) {
        Some(value) if !value.trim().is_empty() => {}
        _ => errors.push(format!("{at}.{key} must be a non-empty string")),
    }
}

fn check_unique(values: &[String], at: &str, errors: &mut Vec<String>) {
    let mut seen = BTreeSet::new();
    for value in values {
        if !seen.insert(value.clone()) {
            errors.push(format!("{at}: duplicate ID {value}"));
        }
    }
}

fn validate_values(scenarios_doc: &Value, metrics_doc: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    for (doc, name) in [
        (scenarios_doc, "scenario catalogue"),
        (metrics_doc, "metric catalogue"),
    ] {
        if string_field(doc, "schema_version") != Some("1.0.0") {
            errors.push(format!("{name}: schema_version must be 1.0.0"));
        }
        if string_field(doc, "programme") != Some("CULINARY-EXPERIENCE-001") {
            errors.push(format!("{name}: programme ID mismatch"));
        }
    }

    let metric_map = match metrics_doc.get("metrics").and_then(Value::as_object) {
        Some(map) => map,
        None => {
            errors.push("metric catalogue: metrics must be an object".to_owned());
            return errors;
        }
    };
    if metric_map.is_empty() {
        errors.push("metric catalogue: must contain at least one metric".to_owned());
    }
    let metric_ids = metric_map.keys().cloned().collect::<BTreeSet<_>>();

    for (id, metric) in metric_map {
        let at = format!("metrics.{id}");
        for key in ["unit", "direction", "definition", "denominator"] {
            check_nonempty_string(metric, key, &at, &mut errors);
        }
        let allowed_directions = ["higher", "lower", "target_range", "descriptive", "non_compensatory"];
        if !string_field(metric, "direction").is_some_and(|d| allowed_directions.contains(&d)) {
            errors.push(format!("{at}.direction is not a supported direction"));
        }
        if let Some(qualifying) = metric.get("qualifying_values") {
            if qualifying.as_array().is_none_or(|xs| xs.is_empty()) {
                errors.push(format!("{at}.qualifying_values must be a non-empty array when present"));
            }
        }
    }

    let evidence_classes = strings_array(metrics_doc, "evidence_classes", "metric catalogue", &mut errors);
    check_unique(&evidence_classes, "metric catalogue.evidence_classes", &mut errors);
    let allowed_evidence_classes = [
        "observed_physical", "observed_human", "derived_from_observations",
        "model_prediction", "simulation", "synthetic_fixture", "literature",
        "assumption", "unknown",
    ];
    for class in &evidence_classes {
        if !allowed_evidence_classes.contains(&class.as_str()) {
            errors.push(format!("metric catalogue: unsupported evidence class {class}"));
        }
    }

    if metrics_doc.pointer("/composite_score_policy/allowed").and_then(Value::as_bool) != Some(false) {
        errors.push("metric catalogue: universal composite score must remain disabled".to_owned());
    }

    for id in ["hard_constraint_violation_count", "unsafe_or_unresolved_recommendation_count"] {
        let qualified = metric_map.get(id)
            .and_then(|m| m.get("qualifying_values"))
            .and_then(Value::as_array);
        if !qualified.is_some_and(|xs| xs.iter().any(|v| v.as_i64() == Some(0))) {
            errors.push(format!("metrics.{id}: zero-violation acceptance value must be present"));
        }
    }

    let scenarios = match scenarios_doc.get("scenarios").and_then(Value::as_array) {
        Some(values) => values,
        None => {
            errors.push("scenario catalogue: scenarios must be an array".to_owned());
            return errors;
        }
    };
    if scenarios.is_empty() {
        errors.push("scenario catalogue: must contain at least one scenario".to_owned());
    }
    let mut scenario_ids = Vec::new();

    for (index, scenario) in scenarios.iter().enumerate() {
        let at = format!("scenarios[{index}]");
        check_nonempty_string(scenario, "scenario_id", &at, &mut errors);
        check_nonempty_string(scenario, "purpose", &at, &mut errors);
        check_nonempty_string(scenario, "context", &at, &mut errors);
        check_nonempty_string(scenario, "claim_limit", &at, &mut errors);
        if let Some(id) = string_field(scenario, "scenario_id") {
            scenario_ids.push(id.to_owned());
        }

        let shift_axes = strings_array(scenario, "shift_axes", &at, &mut errors);
        let required_metrics = strings_array(scenario, "required_metrics", &at, &mut errors);
        let required_gates = strings_array(scenario, "required_gates", &at, &mut errors);
        let evidence_needed = strings_array(scenario, "evidence_needed", &at, &mut errors);
        let adversarial = strings_array(scenario, "adversarial_controls", &at, &mut errors);

        if required_metrics.is_empty() {
            errors.push(format!("{at}: must declare required_metrics"));
        }
        if required_gates.is_empty() {
            errors.push(format!("{at}: must declare non-compensatory required_gates"));
        }
        if evidence_needed.is_empty() {
            errors.push(format!("{at}: must declare evidence_needed"));
        }
        if adversarial.is_empty() {
            errors.push(format!("{at}: must declare adversarial_controls"));
        }
        check_unique(&required_gates, &format!("{at}.required_gates"), &mut errors);
        check_unique(&required_metrics, &format!("{at}.required_metrics"), &mut errors);

        for metric_id in required_metrics {
            if !metric_ids.contains(&metric_id) {
                errors.push(format!("{at}: references unknown metric {metric_id}"));
            }
        }
        for evidence in evidence_needed {
            if !evidence_classes.contains(&evidence) {
                errors.push(format!("{at}: references unknown evidence class {evidence}"));
            }
        }
        for axis in shift_axes {
            if axis.trim().is_empty() {
                errors.push(format!("{at}: shift axis may not be empty"));
            }
        }
    }
    check_unique(&scenario_ids, "scenario catalogue.scenario_id", &mut errors);

    if scenarios_doc.get("shared_rules").and_then(Value::as_array).is_none_or(Vec::is_empty) {
        errors.push("scenario catalogue: shared_rules must be a non-empty array".to_owned());
    }
    if metrics_doc.get("comparison_principles").and_then(Value::as_array).is_none_or(Vec::is_empty) {
        errors.push("metric catalogue: comparison_principles must be a non-empty array".to_owned());
    }

    if scenarios_doc.pointer("/stretch_targets/status").and_then(Value::as_str) != Some("hypotheses_not_results") {
        errors.push("scenario catalogue: stretch targets must be labelled hypotheses_not_results".to_owned());
    }
    if metrics_doc.pointer("/proposed_hypotheses/status").and_then(Value::as_str) != Some("stretch_targets_not_results") {
        errors.push("metric catalogue: proposed hypotheses must be labelled stretch_targets_not_results".to_owned());
    }

    errors.sort();
    errors.dedup();
    errors
}

fn validation_report(contracts: &ContractFiles, errors: &[String]) -> Value {
    let scenario_count = contracts.scenarios.get("scenarios")
        .and_then(Value::as_array).map(Vec::len).unwrap_or(0);
    let metric_count = contracts.metrics.get("metrics")
        .and_then(Value::as_object).map(|m| m.len()).unwrap_or(0);
    let gate_count = contracts.scenarios.get("scenarios")
        .and_then(Value::as_array)
        .map(|scenarios| scenarios.iter()
            .filter_map(|s| s.get("required_gates").and_then(Value::as_array))
            .map(Vec::len).sum::<usize>())
        .unwrap_or(0);
    let metric_ref_count = contracts.scenarios.get("scenarios")
        .and_then(Value::as_array)
        .map(|scenarios| scenarios.iter()
            .filter_map(|s| s.get("required_metrics").and_then(Value::as_array))
            .map(Vec::len).sum::<usize>())
        .unwrap_or(0);

    json!({
        "tool":"culinary-experience-bench",
        "version":VERSION,
        "report_kind":"contract_validation",
        "result":if errors.is_empty(){"pass"}else{"fail"},
        "scenario_count":scenario_count,
        "metric_count":metric_count,
        "required_gate_entries":gate_count,
        "scenario_metric_references":metric_ref_count,
        "contract_sha256":contracts.digests,
        "errors":errors,
        "claim_limit":"A passing report establishes contract consistency only; it is not evidence of recipe quality, food safety, sensory preference, cultural legitimacy, waste reduction or real-world performance."
    })
}

fn run() -> Check<bool> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.is_empty() || args[0] == "--help" || args[0] == "-h" {
        println!(
            "culinary-experience-bench {VERSION}\n\n\
             Commands:\n  \
             validate <directory-containing-CULINARY_EXPERIENCE-JSON-files>\n\n\
             The tool runs offline and validates the two JSON catalogues."
        );
        return Ok(true);
    }
    if args[0] != "validate" {
        return Err(format!("unknown command {}; use validate or --help", args[0]));
    }
    let dir = args.get(1).map(PathBuf::from).unwrap_or_else(|| PathBuf::from("docs/research"));
    let contracts = load_contracts(&dir)?;
    let errors = validate_values(&contracts.scenarios, &contracts.metrics);
    let report = validation_report(&contracts, &errors);
    println!("{}", serde_json::to_string_pretty(&report).map_err(|e| e.to_string())?);
    Ok(errors.is_empty())
}

fn main() -> ExitCode {
    match run() {
        Ok(true) => ExitCode::SUCCESS,
        Ok(false) => ExitCode::FAILURE,
        Err(error) => {
            eprintln!("culinary-experience-bench: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn docs() -> (Value, Value) {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/research");
        let scenarios = fs::read_to_string(root.join("CULINARY_EXPERIENCE_SCENARIOS_V1.json"))
            .expect("scenario catalogue readable");
        let metrics = fs::read_to_string(root.join("CULINARY_EXPERIENCE_METRICS_V1.json"))
            .expect("metric catalogue readable");
        (serde_json::from_str(&scenarios).unwrap(), serde_json::from_str(&metrics).unwrap())
    }

    #[test]
    fn live_repository_contracts_are_consistent() {
        let (scenarios, metrics) = docs();
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.is_empty(), "{errors:#?}");
        assert_eq!(scenarios["scenarios"].as_array().unwrap().len(), 13);
        assert_eq!(metrics["metrics"].as_object().unwrap().len(), 28);
    }

    #[test]
    fn every_scenario_metric_is_registered() {
        let (scenarios, metrics) = docs();
        let scenario_list = scenarios["scenarios"].as_array().unwrap();
        let metric_map = metrics["metrics"].as_object().unwrap();
        for scenario in scenario_list {
            for id in scenario["required_metrics"].as_array().unwrap() {
                assert!(metric_map.contains_key(id.as_str().unwrap()), "unregistered metric {id}");
            }
        }
    }

    #[test]
    fn missing_metric_definition_fails_closed() {
        let (scenarios, mut metrics) = docs();
        metrics["metrics"].as_object_mut().unwrap().remove("makespan_ms");
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("references unknown metric makespan_ms")));
    }

    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let (mut scenarios, metrics) = docs();
        let first = scenarios["scenarios"][0].clone();
        scenarios["scenarios"].as_array_mut().unwrap().push(first);
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("duplicate ID")));
    }

    #[test]
    fn empty_required_gate_set_is_rejected() {
        let (mut scenarios, metrics) = docs();
        scenarios["scenarios"][0]["required_gates"] = json!([]);
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("must declare non-compensatory required_gates")));
    }

    #[test]
    fn duplicate_gate_id_is_rejected() {
        let (mut scenarios, metrics) = docs();
        let gate = scenarios["scenarios"][0]["required_gates"][0].clone();
        scenarios["scenarios"][0]["required_gates"].as_array_mut().unwrap().push(gate);
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("duplicate ID")));
    }

    #[test]
    fn unknown_evidence_class_is_rejected() {
        let (mut scenarios, metrics) = docs();
        scenarios["scenarios"][0]["evidence_needed"][0] = json!("magic_proof");
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("unknown evidence class")));
    }

    #[test]
    fn universal_composite_score_is_rejected() {
        let (scenarios, mut metrics) = docs();
        metrics["composite_score_policy"]["allowed"] = json!(true);
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("universal composite score must remain disabled")));
    }

    #[test]
    fn unsafe_recommendation_requires_zero_qualifying_value() {
        let (scenarios, mut metrics) = docs();
        metrics["metrics"]["unsafe_or_unresolved_recommendation_count"]["qualifying_values"] = json!([1]);
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("zero-violation acceptance value")));
    }

    #[test]
    fn stretch_targets_cannot_be_mislabeled_as_results() {
        let (mut scenarios, metrics) = docs();
        scenarios["stretch_targets"]["status"] = json!("results");
        let errors = validate_values(&scenarios, &metrics);
        assert!(errors.iter().any(|e| e.contains("hypotheses_not_results")));
    }

    #[test]
    fn report_digests_are_sha256() {
        let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../docs/research");
        let contracts = load_contracts(&root).expect("live contract files load");
        assert_eq!(contracts.digests.len(), 2);
        assert!(contracts.digests.values().all(|s| s.len() == 64 && s.bytes().all(|c| c.is_ascii_hexdigit())));
    }
}
