use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, path::Path};

const CORPUS: &str =
    include_str!("../../../fixtures/extreme_environment/extreme_environment_platform_scenarios_v0_1.json");
const CLAIM_CEILING: &str =
    "synthetic_readiness_audit_is_not_a_physical_validation_or_deployment_authorization";

#[derive(Clone, Debug)]
struct MetricDef {
    unit: String,
}

#[derive(Clone, Debug)]
struct ScenarioDef {
    environment_id: String,
    mission_class: String,
    gate_ids: BTreeSet<String>,
    metric_ids: BTreeSet<String>,
}

#[derive(Clone, Debug)]
struct Assessment {
    id: String,
    scenario_id: String,
    platform_id: String,
    disposition: String,
    failed_gates: Vec<String>,
    unknown_gates: Vec<String>,
    unknown_metrics: Vec<String>,
    metrics: BTreeMap<String, Option<(f64, f64)>>,
}

fn exact_keys(value: &Value, expected: &[&str]) -> bool {
    let Some(object) = value.as_object() else { return false; };
    object.len() == expected.len() && expected.iter().all(|key| object.contains_key(*key))
}

fn push(errors: &mut Vec<String>, path: &str, message: &str) {
    errors.push(format!("{path}: {message}"));
}

fn nonempty(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Option<String> {
    match value.get(key).and_then(Value::as_str) {
        Some(text) if !text.trim().is_empty() => Some(text.to_string()),
        Some(_) => {
            push(errors, &format!("{path}.{key}"), "must not be empty");
            None
        }
        None => {
            push(errors, &format!("{path}.{key}"), "expected string");
            None
        }
    }
}

fn string_set(value: &Value, path: &str, errors: &mut Vec<String>) -> BTreeSet<String> {
    let mut found = BTreeSet::new();
    match value.as_array() {
        Some(items) => {
            for (index, item) in items.iter().enumerate() {
                match item.as_str() {
                    Some(text) if !text.trim().is_empty() => {
                        if !found.insert(text.to_string()) {
                            push(errors, &format!("{path}[{index}]"), "duplicate identifier");
                        }
                    }
                    _ => push(errors, &format!("{path}[{index}]"), "expected non-empty string"),
                }
            }
        }
        None => push(errors, path, "expected array"),
    }
    found
}

fn nonempty_string_array(value: &Value, path: &str, errors: &mut Vec<String>) {
    match value.as_array() {
        Some(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                if !item.as_str().is_some_and(|text| !text.trim().is_empty()) {
                    push(errors, &format!("{path}[{index}]"), "expected non-empty string");
                }
            }
        }
        Some(_) => push(errors, path, "expected non-empty array"),
        None => push(errors, path, "expected array"),
    }
}

fn add_id(
    value: &Value,
    key: &str,
    path: &str,
    seen: &mut BTreeSet<String>,
    errors: &mut Vec<String>,
) -> Option<String> {
    let id = nonempty(value, key, path, errors)?;
    if !seen.insert(id.clone()) {
        push(errors, &format!("{path}.{key}"), "duplicate identifier");
    }
    Some(id)
}

fn audit_input(
    root: &Value,
) -> Result<(BTreeMap<String, MetricDef>, BTreeMap<String, ScenarioDef>, Vec<Assessment>), Vec<String>> {
    let mut errors = Vec::new();
    const ROOT_KEYS: &[&str] = &[
        "schema_version", "benchmark_id", "provenance", "claim_ceiling",
        "gate_definitions", "metric_definitions", "environments",
        "platform_archetypes", "mission_scenarios", "assessments",
    ];
    if !exact_keys(root, ROOT_KEYS) {
        push(&mut errors, "root", "missing or unknown fields");
    }
    if root.get("schema_version").and_then(Value::as_str) != Some("0.1") {
        push(&mut errors, "schema_version", "expected 0.1");
    }
    if root.get("benchmark_id").and_then(Value::as_str)
        != Some("extreme-environment-platform-readiness-v0.1")
    {
        push(&mut errors, "benchmark_id", "unexpected benchmark identity");
    }
    if root.get("claim_ceiling").and_then(Value::as_str) != Some(CLAIM_CEILING) {
        push(&mut errors, "claim_ceiling", "missing or incompatible claim ceiling");
    }

    let provenance = &root["provenance"];
    if !exact_keys(provenance, &["kind", "external_real_world_data_included", "notes"]) {
        push(&mut errors, "provenance", "missing or unknown fields");
    }
    if provenance.get("kind").and_then(Value::as_str) != Some("synthetic_only") {
        push(&mut errors, "provenance.kind", "only synthetic inputs are admitted");
    }
    if provenance.get("external_real_world_data_included").and_then(Value::as_bool) != Some(false) {
        push(&mut errors, "provenance.external_real_world_data_included", "must be false");
    }
    nonempty(provenance, "notes", "provenance", &mut errors);

    let mut gate_ids = BTreeSet::new();
    match root.get("gate_definitions").and_then(Value::as_array) {
        Some(defs) if !defs.is_empty() => {
            for (index, gate) in defs.iter().enumerate() {
                let path = format!("gate_definitions[{index}]");
                if !exact_keys(gate, &["gate_id", "meaning", "hard_gate"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                add_id(gate, "gate_id", &path, &mut gate_ids, &mut errors);
                nonempty(gate, "meaning", &path, &mut errors);
                if gate.get("hard_gate").and_then(Value::as_bool) != Some(true) {
                    push(&mut errors, &format!("{path}.hard_gate"), "every gate must be non-tradeable");
                }
            }
        }
        _ => push(&mut errors, "gate_definitions", "expected a non-empty array"),
    }

    let mut metric_defs: BTreeMap<String, MetricDef> = BTreeMap::new();
    match root.get("metric_definitions").and_then(Value::as_array) {
        Some(defs) if !defs.is_empty() => {
            for (index, metric) in defs.iter().enumerate() {
                let path = format!("metric_definitions[{index}]");
                if !exact_keys(metric, &["metric_id", "unit", "direction", "meaning"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                let id = nonempty(metric, "metric_id", &path, &mut errors);
                let unit = nonempty(metric, "unit", &path, &mut errors);
                let direction = nonempty(metric, "direction", &path, &mut errors);
                nonempty(metric, "meaning", &path, &mut errors);
                if let Some(d) = direction.as_deref() {
                    if !["minimize", "maximize"].contains(&d) {
                        push(&mut errors, &format!("{path}.direction"), "expected minimize or maximize");
                    }
                }
                if let (Some(id), Some(unit)) = (id, unit) {
                    if metric_defs.insert(id.clone(), MetricDef { unit }).is_some() {
                        push(&mut errors, &path, &format!("duplicate metric ID {id}"));
                    }
                }
            }
        }
        _ => push(&mut errors, "metric_definitions", "expected a non-empty array"),
    }

    let mut environment_ids = BTreeSet::new();
    match root.get("environments").and_then(Value::as_array) {
        Some(items) if items.len() >= 5 => {
            for (index, item) in items.iter().enumerate() {
                let path = format!("environments[{index}]");
                if !exact_keys(item, &[
                    "environment_id", "label", "design_constraints",
                    "ecological_no_go", "regional_caveat",
                ]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                add_id(item, "environment_id", &path, &mut environment_ids, &mut errors);
                nonempty(item, "label", &path, &mut errors);
                nonempty(item, "regional_caveat", &path, &mut errors);
                nonempty_string_array(&item["design_constraints"], &format!("{path}.design_constraints"), &mut errors);
                nonempty_string_array(&item["ecological_no_go"], &format!("{path}.ecological_no_go"), &mut errors);
            }
        }
        _ => push(&mut errors, "environments", "expected at least five environment archetypes"),
    }

    let mut platform_ids = BTreeSet::new();
    match root.get("platform_archetypes").and_then(Value::as_array) {
        Some(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                let path = format!("platform_archetypes[{index}]");
                if !exact_keys(item, &["platform_id", "platform_class", "description", "declared_limits"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                add_id(item, "platform_id", &path, &mut platform_ids, &mut errors);
                nonempty(item, "platform_class", &path, &mut errors);
                nonempty(item, "description", &path, &mut errors);
                nonempty_string_array(&item["declared_limits"], &format!("{path}.declared_limits"), &mut errors);
            }
        }
        _ => push(&mut errors, "platform_archetypes", "expected a non-empty array"),
    }

    let mut scenarios: BTreeMap<String, ScenarioDef> = BTreeMap::new();
    let mut scenario_ids = BTreeSet::new();
    match root.get("mission_scenarios").and_then(Value::as_array) {
        Some(items) if items.len() >= 5 => {
            for (index, scenario) in items.iter().enumerate() {
                let path = format!("mission_scenarios[{index}]");
                if !exact_keys(scenario, &[
                    "scenario_id", "environment_id", "mission_class",
                    "required_gate_ids", "required_metric_ids", "no_go_actions",
                ]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                let id = add_id(scenario, "scenario_id", &path, &mut scenario_ids, &mut errors);
                let env = nonempty(scenario, "environment_id", &path, &mut errors);
                let mission = nonempty(scenario, "mission_class", &path, &mut errors);
                let required_gates = string_set(&scenario["required_gate_ids"], &format!("{path}.required_gate_ids"), &mut errors);
                let required_metrics = string_set(&scenario["required_metric_ids"], &format!("{path}.required_metric_ids"), &mut errors);
                nonempty_string_array(&scenario["no_go_actions"], &format!("{path}.no_go_actions"), &mut errors);
                if required_gates.is_empty() {
                    push(&mut errors, &format!("{path}.required_gate_ids"), "must not be empty");
                }
                if required_metrics.is_empty() {
                    push(&mut errors, &format!("{path}.required_metric_ids"), "must not be empty");
                }
                if let Some(env) = env.as_deref() {
                    if !environment_ids.contains(env) {
                        push(&mut errors, &format!("{path}.environment_id"), "undeclared environment ID");
                    }
                }
                for gate in &required_gates {
                    if !gate_ids.contains(gate) {
                        push(&mut errors, &format!("{path}.required_gate_ids"), "undeclared gate ID");
                    }
                }
                for metric in &required_metrics {
                    if !metric_defs.contains_key(metric) {
                        push(&mut errors, &format!("{path}.required_metric_ids"), "undeclared metric ID");
                    }
                }
                if let (Some(id), Some(environment_id), Some(mission_class)) = (id, env, mission) {
                    if scenarios.insert(id, ScenarioDef {
                        environment_id,
                        mission_class,
                        gate_ids: required_gates,
                        metric_ids: required_metrics,
                    }).is_some() {
                        push(&mut errors, &path, "duplicate scenario ID");
                    }
                }
            }
        }
        _ => push(&mut errors, "mission_scenarios", "expected at least five mission scenarios"),
    }

    let mut assessments = Vec::new();
    let mut assessment_ids = BTreeSet::new();
    match root.get("assessments").and_then(Value::as_array) {
        Some(items) if !items.is_empty() => {
            for (index, item) in items.iter().enumerate() {
                let path = format!("assessments[{index}]");
                if !exact_keys(item, &["assessment_id", "scenario_id", "platform_id", "gate_results", "metrics"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                let id = add_id(item, "assessment_id", &path, &mut assessment_ids, &mut errors)
                    .unwrap_or_else(|| format!("invalid-assessment-{index}"));
                let scenario_id = nonempty(item, "scenario_id", &path, &mut errors)
                    .unwrap_or_else(|| "invalid-scenario".to_string());
                let platform_id = nonempty(item, "platform_id", &path, &mut errors)
                    .unwrap_or_else(|| "invalid-platform".to_string());
                let scenario = scenarios.get(&scenario_id);
                if scenario.is_none() {
                    push(&mut errors, &format!("{path}.scenario_id"), "undeclared mission scenario");
                }
                if !platform_ids.contains(&platform_id) {
                    push(&mut errors, &format!("{path}.platform_id"), "undeclared platform archetype");
                }

                let mut seen_gates = BTreeSet::new();
                let mut failed_gates = BTreeSet::new();
                let mut unknown_gates = BTreeSet::new();
                match item.get("gate_results").and_then(Value::as_array) {
                    Some(results) => {
                        for (j, result) in results.iter().enumerate() {
                            let result_path = format!("{path}.gate_results[{j}]");
                            if !exact_keys(result, &["gate_id", "status"]) {
                                push(&mut errors, &result_path, "missing or unknown fields");
                            }
                            let gate_id = nonempty(result, "gate_id", &result_path, &mut errors);
                            let status = result.get("status").and_then(Value::as_str);
                            if let Some(gate_id) = gate_id {
                                if !gate_ids.contains(&gate_id) {
                                    push(&mut errors, &result_path, "undeclared gate ID");
                                }
                                if let Some(scenario) = scenario {
                                    if !scenario.gate_ids.contains(&gate_id) {
                                        push(&mut errors, &result_path, "gate is not required for this scenario");
                                    }
                                }
                                if !seen_gates.insert(gate_id.clone()) {
                                    push(&mut errors, &result_path, "duplicate gate result");
                                }
                                match status {
                                    Some("pass") => {}
                                    Some("fail") => { failed_gates.insert(gate_id); }
                                    Some("unknown") => { unknown_gates.insert(gate_id); }
                                    _ => push(&mut errors, &format!("{result_path}.status"), "expected pass, fail, or unknown"),
                                }
                            }
                        }
                    }
                    None => push(&mut errors, &format!("{path}.gate_results"), "expected array"),
                }
                if let Some(scenario) = scenario {
                    if seen_gates != scenario.gate_ids {
                        push(&mut errors, &format!("{path}.gate_results"), "must contain every required gate exactly once");
                    }
                }

                let mut seen_metrics = BTreeSet::new();
                let mut metric_values: BTreeMap<String, Option<(f64, f64)>> = BTreeMap::new();
                let mut unknown_metrics = BTreeSet::new();
                match item.get("metrics").and_then(Value::as_array) {
                    Some(input_metrics) => {
                        for (j, metric) in input_metrics.iter().enumerate() {
                            let metric_path = format!("{path}.metrics[{j}]");
                            if !exact_keys(metric, &["metric_id", "unit", "status", "lower", "upper"]) {
                                push(&mut errors, &metric_path, "missing or unknown fields");
                            }
                            let metric_id = nonempty(metric, "metric_id", &metric_path, &mut errors);
                            let unit = nonempty(metric, "unit", &metric_path, &mut errors);
                            let status = metric.get("status").and_then(Value::as_str);
                            let mut range = None;
                            if let Some(metric_id) = metric_id {
                                if !seen_metrics.insert(metric_id.clone()) {
                                    push(&mut errors, &metric_path, "duplicate metric result");
                                }
                                if let Some(scenario) = scenario {
                                    if !scenario.metric_ids.contains(&metric_id) {
                                        push(&mut errors, &metric_path, "metric is not required for this scenario");
                                    }
                                }
                                match metric_defs.get(&metric_id) {
                                    None => push(&mut errors, &metric_path, "undeclared metric ID"),
                                    Some(def) => {
                                        if unit.as_deref() != Some(def.unit.as_str()) {
                                            push(&mut errors, &format!("{metric_path}.unit"), "unit mismatch");
                                        }
                                        match status {
                                            Some("synthetic_interval") => {
                                                let lo = metric.get("lower").and_then(Value::as_f64);
                                                let hi = metric.get("upper").and_then(Value::as_f64);
                                                match (lo, hi) {
                                                    (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() && lo <= hi => {
                                                        range = Some((lo, hi));
                                                    }
                                                    (Some(lo), Some(hi)) if !lo.is_finite() || !hi.is_finite() => {
                                                        push(&mut errors, &metric_path, "endpoints must be finite");
                                                    }
                                                    (Some(_), Some(_)) => push(&mut errors, &metric_path, "lower endpoint must be <= upper endpoint"),
                                                    _ => push(&mut errors, &metric_path, "synthetic_interval requires numeric endpoints"),
                                                }
                                            }
                                            Some("unknown") => {
                                                if !metric.get("lower").is_some_and(Value::is_null)
                                                    || !metric.get("upper").is_some_and(Value::is_null) {
                                                    push(&mut errors, &metric_path, "unknown interval must have null endpoints");
                                                }
                                                unknown_metrics.insert(metric_id.clone());
                                            }
                                            _ => push(&mut errors, &format!("{metric_path}.status"), "expected synthetic_interval or unknown"),
                                        }
                                        metric_values.insert(metric_id, range);
                                    }
                                }
                            }
                        }
                    }
                    None => push(&mut errors, &format!("{path}.metrics"), "expected array"),
                }
                if let Some(scenario) = scenario {
                    if seen_metrics != scenario.metric_ids {
                        push(&mut errors, &format!("{path}.metrics"), "must contain every required metric exactly once");
                    }
                }
                let metrics_complete = scenario.is_some_and(|s| {
                    metric_values.len() == s.metric_ids.len()
                        && s.metric_ids.iter().all(|id| metric_values.get(id).is_some_and(Option::is_some))
                });

                let disposition = if !failed_gates.is_empty() {
                    "blocked_hard_gate"
                } else if !unknown_gates.is_empty() {
                    "needs_gate_evidence"
                } else if !metrics_complete {
                    "not_assessable_incomplete_metrics"
                } else {
                    "eligible_for_human_review"
                };
                assessments.push(Assessment {
                    id,
                    scenario_id,
                    platform_id,
                    disposition: disposition.to_string(),
                    failed_gates: failed_gates.into_iter().collect(),
                    unknown_gates: unknown_gates.into_iter().collect(),
                    unknown_metrics: unknown_metrics.into_iter().collect(),
                    metrics: metric_values,
                });
            }
        }
        _ => push(&mut errors, "assessments", "expected non-empty array"),
    }

    if errors.is_empty() {
        Ok((metric_defs, scenarios, assessments))
    } else {
        errors.sort();
        Err(errors)
    }
}

fn evaluate(root: &Value) -> Result<Value, Vec<String>> {
    let (_metric_defs, scenarios, mut assessments) = audit_input(root)?;
    assessments.sort_by(|a, b| a.id.cmp(&b.id));

    let mut assessment_reports = Vec::new();
    let mut summary_by_scenario: BTreeMap<String, BTreeMap<String, usize>> = BTreeMap::new();
    for assessment in &assessments {
        *summary_by_scenario.entry(assessment.scenario_id.clone())
            .or_default().entry(assessment.disposition.clone()).or_default() += 1;
        let metric_intervals: BTreeMap<String, Value> = assessment.metrics.iter().map(|(id, range)| {
            let v = range.map(|(lower, upper)| json!({"lower": lower, "upper": upper}))
                .unwrap_or(Value::Null);
            (id.clone(), v)
        }).collect();
        assessment_reports.push(json!({
            "assessment_id": &assessment.id,
            "scenario_id": &assessment.scenario_id,
            "platform_id": &assessment.platform_id,
            "readiness_disposition": &assessment.disposition,
            "failed_gate_ids": &assessment.failed_gates,
            "unknown_gate_ids": &assessment.unknown_gates,
            "unknown_metric_ids": &assessment.unknown_metrics,
            "metric_intervals": metric_intervals,
            "operational_authorization_granted": false,
            "physical_validation_claimed": false
        }));
    }
    let scenario_summaries: Vec<Value> = scenarios.iter().map(|(id, scenario)| {
        let counts = summary_by_scenario.get(id).cloned().unwrap_or_default();
        json!({
            "scenario_id": id,
            "environment_id": scenario.environment_id,
            "mission_class": scenario.mission_class,
            "assessment_count": counts.values().sum::<usize>(),
            "eligible_for_human_review": counts.get("eligible_for_human_review").copied().unwrap_or(0),
            "blocked_hard_gate": counts.get("blocked_hard_gate").copied().unwrap_or(0),
            "needs_gate_evidence": counts.get("needs_gate_evidence").copied().unwrap_or(0),
            "not_assessable_incomplete_metrics": counts.get("not_assessable_incomplete_metrics").copied().unwrap_or(0),
            "no_universal_winner_produced": true
        })
    }).collect();
    let count = |name: &str| assessments.iter().filter(|a| a.disposition == name).count();

    Ok(json!({
        "report_version": "0.1",
        "benchmark_id": "extreme-environment-platform-readiness-v0.1",
        "claim_ceiling": CLAIM_CEILING,
        "environment_count": scenarios.values().map(|s| s.environment_id.as_str()).collect::<BTreeSet<_>>().len(),
        "mission_scenario_count": scenarios.len(),
        "platform_assessment_count": assessments.len(),
        "eligible_for_human_review_count": count("eligible_for_human_review"),
        "blocked_hard_gate_count": count("blocked_hard_gate"),
        "needs_gate_evidence_count": count("needs_gate_evidence"),
        "not_assessable_incomplete_metrics_count": count("not_assessable_incomplete_metrics"),
        "scenario_summaries": scenario_summaries,
        "assessment_reports": assessment_reports,
        "weighted_universal_score_produced": false,
        "deployment_authorized": false,
        "autonomous_hazard_handling_authorized": false,
        "interpretation": "Eligibility means only that the synthetic input contract has complete passing declared gates and intervals. It is not a platform recommendation, physical qualification, permit, safety case, or authorization to deploy."
    }))
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| {
        "fixtures/extreme_environment/extreme_environment_platform_scenarios_v0_1.json".to_string()
    });
    if args.next().is_some() {
        eprintln!("usage: extreme-environment-audit [path/to/fixture.json]");
        std::process::exit(2);
    }
    let source = if path == "-" {
        CORPUS.to_string()
    } else if Path::new(&path).exists() {
        match fs::read_to_string(&path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("FAIL: unable to read {path}: {error}");
                std::process::exit(2);
            }
        }
    } else {
        eprintln!("FAIL: file not found: {path}");
        std::process::exit(2);
    };
    let input: Value = match serde_json::from_str(&source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("FAIL: invalid JSON: {error}");
            std::process::exit(2);
        }
    };
    match evaluate(&input) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("report serializes")),
        Err(errors) => {
            eprintln!("FAIL: {} contract finding(s)", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(CORPUS).expect("embedded fixture parses")
    }

    fn output(root: &Value) -> Value {
        evaluate(root).expect("valid fixture")
    }

    fn summary_value(report: &Value, key: &str) -> usize {
        report.get(key).and_then(Value::as_u64).unwrap() as usize
    }

    #[test]
    fn fixture_dispositions_match_authored_synthetic_baseline() {
        let report = output(&fixture());
        assert_eq!(summary_value(&report, "environment_count"), 5);
        assert_eq!(summary_value(&report, "mission_scenario_count"), 5);
        assert_eq!(summary_value(&report, "platform_assessment_count"), 15);
        assert_eq!(summary_value(&report, "eligible_for_human_review_count"), 6);
        assert_eq!(summary_value(&report, "blocked_hard_gate_count"), 4);
        assert_eq!(summary_value(&report, "needs_gate_evidence_count"), 5);
        assert_eq!(summary_value(&report, "not_assessable_incomplete_metrics_count"), 0);
        assert_eq!(report["deployment_authorized"], false);
        assert_eq!(report["weighted_universal_score_produced"], false);
    }

    #[test]
    fn failed_hard_gate_blocks_candidate_even_when_metric_intervals_look_attractive() {
        let mut root = fixture();
        let item = root["assessments"].as_array_mut().unwrap().iter_mut()
            .find(|a| a["assessment_id"] == "amz-tracked-impact").unwrap();
        for metric in item["metrics"].as_array_mut().unwrap() {
            metric["lower"] = json!(0.0);
            metric["upper"] = json!(0.1);
        }
        let result = output(&root);
        let candidate = result["assessment_reports"].as_array().unwrap().iter()
            .find(|a| a["assessment_id"] == "amz-tracked-impact").unwrap();
        assert_eq!(candidate["readiness_disposition"], "blocked_hard_gate");
        assert_eq!(candidate["operational_authorization_granted"], false);
    }

    #[test]
    fn unknown_gate_never_becomes_eligible() {
        let mut root = fixture();
        let item = root["assessments"].as_array_mut().unwrap().iter_mut()
            .find(|a| a["assessment_id"] == "afg-route-unverified").unwrap();
        item["gate_results"][1]["status"] = json!("unknown");
        let result = output(&root);
        let candidate = result["assessment_reports"].as_array().unwrap().iter()
            .find(|a| a["assessment_id"] == "afg-route-unverified").unwrap();
        assert_eq!(candidate["readiness_disposition"], "needs_gate_evidence");
        assert_eq!(candidate["operational_authorization_granted"], false);
    }

    #[test]
    fn suspected_explosive_hazard_gate_unknown_is_a_hard_stop() {
        let mut root = fixture();
        let item = root["assessments"].as_array_mut().unwrap().iter_mut()
            .find(|a| a["assessment_id"] == "afg-cleared-corridor").unwrap();
        item["gate_results"][1]["status"] = json!("unknown");
        let result = output(&root);
        let candidate = result["assessment_reports"].as_array().unwrap().iter()
            .find(|a| a["assessment_id"] == "afg-cleared-corridor").unwrap();
        assert_eq!(candidate["readiness_disposition"], "needs_gate_evidence");
        assert!(candidate["unknown_gate_ids"].as_array().unwrap().contains(&json!("afg_authorized_route")));
    }

    #[test]
    fn reversed_metric_interval_fails_closed() {
        let mut root = fixture();
        root["assessments"][0]["metrics"][0]["lower"] = json!(9.0);
        root["assessments"][0]["metrics"][0]["upper"] = json!(1.0);
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn unknown_metric_is_not_imputed_and_cannot_be_assessed() {
        let mut root = fixture();
        let item = root["assessments"].as_array_mut().unwrap().iter_mut()
            .find(|a| a["assessment_id"] == "congo-rover-approved").unwrap();
        item["metrics"][0]["status"] = json!("unknown");
        item["metrics"][0]["lower"] = Value::Null;
        item["metrics"][0]["upper"] = Value::Null;
        let report = output(&root);
        let candidate = report["assessment_reports"].as_array().unwrap().iter()
            .find(|a| a["assessment_id"] == "congo-rover-approved").unwrap();
        assert_eq!(candidate["readiness_disposition"], "not_assessable_incomplete_metrics");
        assert_eq!(candidate["metric_intervals"]["payload_capacity_kg"], Value::Null);
    }

    #[test]
    fn all_243_combinations_of_five_gate_states_match_independent_oracle() {
        let mut root = fixture();
        let index = root["assessments"].as_array().unwrap().iter()
            .position(|a| a["assessment_id"] == "amz-boat-approved").unwrap();
        let original = root["assessments"][index]["gate_results"].clone();
        for code in 0..243usize {
            let mut n = code;
            let mut any_fail = false;
            let mut any_unknown = false;
            for i in 0..5 {
                let state = n % 3;
                n /= 3;
                let name = match state {
                    0 => "pass",
                    1 => "fail",
                    _ => "unknown",
                };
                root["assessments"][index]["gate_results"][i]["status"] = json!(name);
                if state == 1 { any_fail = true; }
                if state == 2 { any_unknown = true; }
            }
            let expected = if any_fail { "blocked_hard_gate" }
                else if any_unknown { "needs_gate_evidence" }
                else { "eligible_for_human_review" };
            let report = output(&root);
            let got = report["assessment_reports"].as_array().unwrap().iter()
                .find(|a| a["assessment_id"] == "amz-boat-approved").unwrap();
            assert_eq!(got["readiness_disposition"], expected, "gate-state encoding {code}");
        }
        root["assessments"][index]["gate_results"] = original;
    }

    #[test]
    fn input_order_permutations_produce_identical_serialized_report() {
        let mut root = fixture();
        let before = serde_json::to_string(&output(&root)).unwrap();
        root["environments"].as_array_mut().unwrap().reverse();
        root["platform_archetypes"].as_array_mut().unwrap().reverse();
        root["gate_definitions"].as_array_mut().unwrap().reverse();
        root["metric_definitions"].as_array_mut().unwrap().reverse();
        root["mission_scenarios"].as_array_mut().unwrap().reverse();
        root["assessments"].as_array_mut().unwrap().reverse();
        for scenario in root["mission_scenarios"].as_array_mut().unwrap() {
            scenario["required_gate_ids"].as_array_mut().unwrap().reverse();
            scenario["required_metric_ids"].as_array_mut().unwrap().reverse();
        }
        for assessment in root["assessments"].as_array_mut().unwrap() {
            assessment["gate_results"].as_array_mut().unwrap().reverse();
            assessment["metrics"].as_array_mut().unwrap().reverse();
        }
        let after = serde_json::to_string(&output(&root)).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn missing_gate_and_unknown_root_fields_fail_closed() {
        let mut root = fixture();
        root["assessments"][0]["gate_results"].as_array_mut().unwrap().pop();
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        root["deployment_recommended"] = json!(true);
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        root["mission_scenarios"][0]["no_go_actions"] = json!([]);
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        root["platform_archetypes"][0]["declared_limits"] = json!([]);
        assert!(evaluate(&root).is_err());
    }
}