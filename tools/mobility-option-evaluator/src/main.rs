use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../fixtures/mobility_options/mobility_option_scenarios_v0_1.json");
const CLAIM_CEILING: &str =
    "synthetic_pareto_results_are_not_real_market_affordability_or_safety_evidence";

#[derive(Clone, Debug)]
struct MetricDef {
    unit: String,
    direction: String,
}

#[derive(Clone, Debug)]
struct EvaluatedCandidate {
    id: String,
    gate_disposition: String,
    ranking_status: String,
    failed_gates: Vec<String>,
    unknown_gates: Vec<String>,
    unknown_metrics: Vec<String>,
    metrics: BTreeMap<String, f64>,
    dominated_by: Vec<String>,
}

fn has_exact_keys(value: &Value, required: &[&str]) -> bool {
    let Some(object) = value.as_object() else { return false; };
    object.len() == required.len() && required.iter().all(|key| object.contains_key(*key))
}

fn push(errors: &mut Vec<String>, path: &str, message: &str) {
    errors.push(format!("{path}: {message}"));
}

fn nonempty_string(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Option<String> {
    match value.get(key).and_then(Value::as_str) {
        Some(s) if !s.trim().is_empty() => Some(s.to_string()),
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

fn audit_input(root: &Value) -> Result<(BTreeMap<String, MetricDef>, Vec<String>, Vec<String>), Vec<String>> {
    let mut errors = Vec::new();
    const ROOT_KEYS: &[&str] = &[
        "schema_version", "benchmark_id", "provenance", "claim_ceiling",
        "scenario", "gate_definitions", "metric_definitions", "candidates"
    ];
    if !has_exact_keys(root, ROOT_KEYS) {
        push(&mut errors, "root", "missing or unknown root fields");
    }
    if root.get("schema_version").and_then(Value::as_str) != Some("0.1") {
        push(&mut errors, "schema_version", "expected 0.1");
    }
    if root.get("benchmark_id").and_then(Value::as_str) != Some("mobility-options-pareto-v0.1") {
        push(&mut errors, "benchmark_id", "unexpected benchmark identity");
    }
    if root.get("claim_ceiling").and_then(Value::as_str) != Some(CLAIM_CEILING) {
        push(&mut errors, "claim_ceiling", "missing or incompatible claim ceiling");
    }

    let provenance = &root["provenance"];
    const PROVENANCE_KEYS: &[&str] = &["kind", "external_market_data_included", "notes"];
    if !has_exact_keys(provenance, PROVENANCE_KEYS) {
        push(&mut errors, "provenance", "missing or unknown fields");
    }
    if provenance.get("kind").and_then(Value::as_str) != Some("synthetic_only") {
        push(&mut errors, "provenance.kind", "only synthetic fixtures are admitted by v0.1");
    }
    if provenance.get("external_market_data_included").and_then(Value::as_bool) != Some(false) {
        push(&mut errors, "provenance.external_market_data_included", "must be false");
    }
    nonempty_string(provenance, "notes", "provenance", &mut errors);

    let scenario = &root["scenario"];
    const SCENARIO_KEYS: &[&str] = &["scenario_id", "context_id", "service_unit", "intended_user_segment"];
    if !has_exact_keys(scenario, SCENARIO_KEYS) {
        push(&mut errors, "scenario", "missing or unknown fields");
    }
    for key in SCENARIO_KEYS {
        nonempty_string(scenario, key, "scenario", &mut errors);
    }

    let mut gate_ids = Vec::new();
    match root.get("gate_definitions").and_then(Value::as_array) {
        Some(defs) if !defs.is_empty() => {
            for (index, def) in defs.iter().enumerate() {
                let path = format!("gate_definitions[{index}]");
                if !has_exact_keys(def, &["gate_id", "meaning", "hard_gate"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                if let Some(id) = nonempty_string(def, "gate_id", &path, &mut errors) {
                    gate_ids.push(id);
                }
                nonempty_string(def, "meaning", &path, &mut errors);
                if def.get("hard_gate").and_then(Value::as_bool) != Some(true) {
                    push(&mut errors, &format!("{path}.hard_gate"), "all gates must be non-tradeable hard gates");
                }
            }
        }
        _ => push(&mut errors, "gate_definitions", "expected non-empty array"),
    }
    if gate_ids.iter().collect::<BTreeSet<_>>().len() != gate_ids.len() {
        push(&mut errors, "gate_definitions", "duplicate gate IDs");
    }

    let mut metric_defs = BTreeMap::new();
    match root.get("metric_definitions").and_then(Value::as_array) {
        Some(defs) if defs.len() >= 2 => {
            for (index, def) in defs.iter().enumerate() {
                let path = format!("metric_definitions[{index}]");
                if !has_exact_keys(def, &["metric_id", "unit", "direction", "meaning"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                let id = nonempty_string(def, "metric_id", &path, &mut errors);
                let unit = nonempty_string(def, "unit", &path, &mut errors);
                let direction = nonempty_string(def, "direction", &path, &mut errors);
                nonempty_string(def, "meaning", &path, &mut errors);
                if let Some(direction) = direction.as_deref() {
                    if !["minimize", "maximize"].contains(&direction) {
                        push(&mut errors, &format!("{path}.direction"), "expected minimize or maximize");
                    }
                }
                if let (Some(id), Some(unit), Some(direction)) = (id, unit, direction) {
                    if metric_defs.insert(id.clone(), MetricDef { unit, direction }).is_some() {
                        push(&mut errors, "metric_definitions", &format!("duplicate metric ID {id}"));
                    }
                }
            }
        }
        _ => push(&mut errors, "metric_definitions", "expected at least two metrics"),
    }

    let mut candidate_ids = Vec::new();
    match root.get("candidates").and_then(Value::as_array) {
        Some(candidates) if candidates.len() >= 2 => {
            for (index, candidate) in candidates.iter().enumerate() {
                let path = format!("candidates[{index}]");
                if !has_exact_keys(candidate, &["candidate_id", "bundle_description", "gates", "metrics"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                if let Some(id) = nonempty_string(candidate, "candidate_id", &path, &mut errors) {
                    candidate_ids.push(id);
                }
                nonempty_string(candidate, "bundle_description", &path, &mut errors);

                let mut seen_gates = BTreeSet::new();
                match candidate.get("gates").and_then(Value::as_array) {
                    Some(gates) => {
                        if gates.len() != gate_ids.len() {
                            push(&mut errors, &format!("{path}.gates"), "candidate must provide each declared hard gate exactly once");
                        }
                        for (gate_index, gate) in gates.iter().enumerate() {
                            let gate_path = format!("{path}.gates[{gate_index}]");
                            if !has_exact_keys(gate, &["gate_id", "disposition", "evidence_class"]) {
                                push(&mut errors, &gate_path, "missing or unknown fields");
                            }
                            if let Some(id) = nonempty_string(gate, "gate_id", &gate_path, &mut errors) {
                                if !gate_ids.contains(&id) {
                                    push(&mut errors, &format!("{gate_path}.gate_id"), "undeclared gate");
                                }
                                if !seen_gates.insert(id.clone()) {
                                    push(&mut errors, &format!("{gate_path}.gate_id"), "duplicate gate");
                                }
                            }
                            if !["pass", "fail", "unknown"].contains(&gate.get("disposition").and_then(Value::as_str).unwrap_or("")) {
                                push(&mut errors, &format!("{gate_path}.disposition"), "expected pass, fail, or unknown");
                            }
                            if gate.get("evidence_class").and_then(Value::as_str) != Some("synthetic_fixture_only") {
                                push(&mut errors, &format!("{gate_path}.evidence_class"), "v0.1 accepts synthetic fixture evidence only");
                            }
                        }
                        for id in &gate_ids {
                            if !seen_gates.contains(id) {
                                push(&mut errors, &format!("{path}.gates"), &format!("missing gate {id}"));
                            }
                        }
                    }
                    None => push(&mut errors, &format!("{path}.gates"), "expected array"),
                }

                let mut seen_metrics = BTreeSet::new();
                match candidate.get("metrics").and_then(Value::as_array) {
                    Some(metrics) => {
                        if metrics.len() != metric_defs.len() {
                            push(&mut errors, &format!("{path}.metrics"), "candidate must declare every metric exactly once");
                        }
                        for (metric_index, metric) in metrics.iter().enumerate() {
                            let metric_path = format!("{path}.metrics[{metric_index}]");
                            if !has_exact_keys(metric, &["metric_id", "unit", "status", "value", "evidence_class"]) {
                                push(&mut errors, &metric_path, "missing or unknown fields");
                            }
                            let id = nonempty_string(metric, "metric_id", &metric_path, &mut errors);
                            let unit = nonempty_string(metric, "unit", &metric_path, &mut errors);
                            if let Some(id) = id {
                                let Some(def) = metric_defs.get(&id) else {
                                    push(&mut errors, &format!("{metric_path}.metric_id"), "undeclared metric");
                                    continue;
                                };
                                if !seen_metrics.insert(id.clone()) {
                                    push(&mut errors, &format!("{metric_path}.metric_id"), "duplicate metric");
                                }
                                if unit.as_deref() != Some(def.unit.as_str()) {
                                    push(&mut errors, &format!("{metric_path}.unit"), "unit does not match shared definition");
                                }
                                match metric.get("status").and_then(Value::as_str) {
                                    Some("synthetic_value") => {
                                        match metric.get("value").and_then(Value::as_f64) {
                                            Some(v) if v.is_finite() => {}
                                            _ => push(&mut errors, &format!("{metric_path}.value"), "synthetic_value requires finite number"),
                                        }
                                    }
                                    Some("unknown") => {
                                        if !metric.get("value").is_some_and(Value::is_null) {
                                            push(&mut errors, &format!("{metric_path}.value"), "unknown metric must have null value");
                                        }
                                    }
                                    _ => push(&mut errors, &format!("{metric_path}.status"), "expected synthetic_value or unknown"),
                                }
                                if metric.get("evidence_class").and_then(Value::as_str) != Some("synthetic_fixture_only") {
                                    push(&mut errors, &format!("{metric_path}.evidence_class"), "v0.1 accepts synthetic fixture evidence only");
                                }
                            }
                        }
                        for id in metric_defs.keys() {
                            if !seen_metrics.contains(id) {
                                push(&mut errors, &format!("{path}.metrics"), &format!("missing metric {id}"));
                            }
                        }
                    }
                    None => push(&mut errors, &format!("{path}.metrics"), "expected array"),
                }
            }
        }
        _ => push(&mut errors, "candidates", "expected at least two candidates"),
    }
    if candidate_ids.iter().collect::<BTreeSet<_>>().len() != candidate_ids.len() {
        push(&mut errors, "candidates", "duplicate candidate IDs");
    }

    if errors.is_empty() {
        Ok((metric_defs, gate_ids, candidate_ids))
    } else {
        Err(errors)
    }
}

fn dominates(a: &BTreeMap<String, f64>, b: &BTreeMap<String, f64>, defs: &BTreeMap<String, MetricDef>) -> bool {
    let mut strictly_better = false;
    for (id, def) in defs {
        let Some(av) = a.get(id) else { return false; };
        let Some(bv) = b.get(id) else { return false; };
        match def.direction.as_str() {
            "minimize" => {
                if av > bv { return false; }
                if av < bv { strictly_better = true; }
            }
            "maximize" => {
                if av < bv { return false; }
                if av > bv { strictly_better = true; }
            }
            _ => return false,
        }
    }
    strictly_better
}

fn evaluate(root: &Value) -> Result<Value, Vec<String>> {
    let (defs, gate_ids, _) = audit_input(root)?;
    let candidates = root["candidates"].as_array().expect("validated candidates");
    let mut evaluated = Vec::new();

    for candidate in candidates {
        let id = candidate["candidate_id"].as_str().expect("validated ID").to_string();
        let gates = candidate["gates"].as_array().expect("validated gates");
        let mut failed_gates = Vec::new();
        let mut unknown_gates = Vec::new();
        for gate_id in &gate_ids {
            let gate = gates.iter().find(|g| g["gate_id"].as_str() == Some(gate_id.as_str())).expect("validated gate set");
            match gate["disposition"].as_str().expect("validated disposition") {
                "fail" => failed_gates.push(gate_id.clone()),
                "unknown" => unknown_gates.push(gate_id.clone()),
                "pass" => {}
                _ => unreachable!("validated gate disposition"),
            }
        }

        let gate_disposition = if !failed_gates.is_empty() {
            "blocked"
        } else if !unknown_gates.is_empty() {
            "needs_evidence"
        } else {
            "eligible"
        }.to_string();

        let mut metrics = BTreeMap::new();
        let mut unknown_metrics = Vec::new();
        for metric in candidate["metrics"].as_array().expect("validated metrics") {
            let metric_id = metric["metric_id"].as_str().expect("validated metric ID").to_string();
            if metric["status"] == "unknown" {
                unknown_metrics.push(metric_id);
            } else {
                metrics.insert(metric_id, metric["value"].as_f64().expect("validated number"));
            }
        }
        failed_gates.sort();
        unknown_gates.sort();
        unknown_metrics.sort();
        evaluated.push(EvaluatedCandidate {
            id,
            gate_disposition,
            ranking_status: String::new(),
            failed_gates,
            unknown_gates,
            unknown_metrics,
            metrics,
            dominated_by: Vec::new(),
        });
    }

    evaluated.sort_by(|a, b| a.id.cmp(&b.id));
    let rankable: Vec<usize> = evaluated.iter().enumerate()
        .filter(|(_, c)| c.gate_disposition == "eligible" && c.unknown_metrics.is_empty())
        .map(|(i, _)| i)
        .collect();

    for &i in &rankable {
        for &j in &rankable {
            if i != j && dominates(&evaluated[j].metrics, &evaluated[i].metrics, &defs) {
                evaluated[i].dominated_by.push(evaluated[j].id.clone());
            }
        }
        evaluated[i].dominated_by.sort();
    }

    let mut frontier_ids = Vec::new();
    for candidate in &mut evaluated {
        candidate.ranking_status = match candidate.gate_disposition.as_str() {
            "blocked" => "excluded_hard_gate_failure".to_string(),
            "needs_evidence" => "excluded_unknown_hard_gate".to_string(),
            "eligible" if !candidate.unknown_metrics.is_empty() => "not_rankable_incomplete_metrics".to_string(),
            "eligible" if candidate.dominated_by.is_empty() => {
                frontier_ids.push(candidate.id.clone());
                "pareto_frontier".to_string()
            }
            "eligible" => "dominated_within_declared_candidate_set".to_string(),
            _ => unreachable!("all dispositions assigned above"),
        };
    }
    frontier_ids.sort();

    let reports: Vec<Value> = evaluated.iter().map(|c| {
        let metric_values: BTreeMap<String, Value> = defs.keys().map(|id| {
            let candidate_metric = root["candidates"].as_array().unwrap().iter()
                .find(|source| source["candidate_id"].as_str() == Some(c.id.as_str())).unwrap();
            let metric = candidate_metric["metrics"].as_array().unwrap().iter()
                .find(|m| m["metric_id"].as_str() == Some(id.as_str())).unwrap();
            let v = metric.get("value").cloned().unwrap_or(Value::Null);
            (id.clone(), v)
        }).collect();
        json!({
            "candidate_id": c.id,
            "gate_disposition": c.gate_disposition,
            "ranking_status": c.ranking_status,
            "failed_gate_ids": c.failed_gates,
            "unknown_gate_ids": c.unknown_gates,
            "unknown_metric_ids": c.unknown_metrics,
            "metric_values": metric_values,
            "dominated_by_candidate_ids": c.dominated_by
        })
    }).collect();

    Ok(json!({
        "report_version": "0.1",
        "benchmark_id": root["benchmark_id"],
        "scenario_id": root["scenario"]["scenario_id"],
        "claim_ceiling": CLAIM_CEILING,
        "candidate_reports": reports,
        "pareto_frontier_candidate_ids": frontier_ids,
        "no_single_global_winner": true,
        "weighted_utility_score_produced": false,
        "scope_note": "Pareto membership applies only to the supplied candidate set, synthetic scenario, declared hard gates, and exact comparable metric definitions."
    }))
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "fixtures/mobility_options/mobility_option_scenarios_v0_1.json".to_string());
    if args.next().is_some() {
        eprintln!("usage: mobility-option-evaluator [path/to/fixture.json]");
        std::process::exit(2);
    }
    let source = if Path::new(&path).exists() {
        match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => { eprintln!("FAIL: unable to read {path}: {e}"); std::process::exit(2); }
        }
    } else if path == "-" {
        EMBEDDED_CORPUS.to_string()
    } else {
        eprintln!("FAIL: file not found: {path}");
        std::process::exit(2);
    };
    let input: Value = match serde_json::from_str(&source) {
        Ok(v) => v,
        Err(e) => { eprintln!("FAIL: invalid JSON: {e}"); std::process::exit(2); }
    };
    match evaluate(&input) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("report serializes")),
        Err(errors) => {
            eprintln!("FAIL: {} contract finding(s)", errors.len());
            for error in errors { eprintln!("- {error}"); }
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_str(EMBEDDED_CORPUS).expect("fixture JSON parses")
    }

    fn report_frontier(root: &Value) -> Vec<String> {
        evaluate(root).expect("fixture evaluates")["pareto_frontier_candidate_ids"]
            .as_array().expect("frontier array").iter().map(|v| v.as_str().unwrap().to_string()).collect()
    }

    #[test]
    fn fixture_keeps_hard_failures_unknowns_and_incomplete_metrics_out_of_frontier() {
        let root = fixture();
        let report = evaluate(&root).expect("valid synthetic input");
        let frontier: BTreeSet<String> = report_frontier(&root).into_iter().collect();
        for id in [
            "06-uncertified-low-price-import",
            "07-no-confirmed-energy-or-service",
            "08-low-upfront-cost-with-unknown-lifecycle-burden"
        ] {
            assert!(!frontier.contains(id), "{id} must not be in the Pareto set");
        }
        let reports = report["candidate_reports"].as_array().unwrap();
        let find = |id: &str| reports.iter().find(|x| x["candidate_id"] == id).unwrap();
        assert_eq!(find("06-uncertified-low-price-import")["gate_disposition"], "blocked");
        assert_eq!(find("07-no-confirmed-energy-or-service")["gate_disposition"], "needs_evidence");
        assert_eq!(find("08-low-upfront-cost-with-unknown-lifecycle-burden")["ranking_status"], "not_rankable_incomplete_metrics");
        assert_eq!(report["weighted_utility_score_produced"], false);
        assert_eq!(report.get("winner"), None);
    }

    #[test]
    fn failed_gate_cannot_be_traded_against_superior_metrics() {
        let mut root = fixture();
        let candidate = root["candidates"].as_array_mut().unwrap().iter_mut()
            .find(|x| x["candidate_id"] == "06-uncertified-low-price-import").unwrap();
        for metric in candidate["metrics"].as_array_mut().unwrap() {
            if metric["status"] == "synthetic_value" {
                let is_fraction = metric["unit"].as_str().unwrap() == "fraction";
                metric["value"] = Value::from(if is_fraction { 1.0 } else { 0.0 });
            }
        }
        let report = evaluate(&root).unwrap();
        let entry = report["candidate_reports"].as_array().unwrap().iter()
            .find(|x| x["candidate_id"] == "06-uncertified-low-price-import").unwrap();
        assert_eq!(entry["gate_disposition"], "blocked");
        assert_ne!(entry["ranking_status"], "pareto_frontier");
    }

    #[test]
    fn unknown_hard_gate_never_enters_comparison_set() {
        let root = fixture();
        let report = evaluate(&root).unwrap();
        let entry = report["candidate_reports"].as_array().unwrap().iter()
            .find(|x| x["candidate_id"] == "07-no-confirmed-energy-or-service").unwrap();
        assert_eq!(entry["gate_disposition"], "needs_evidence");
        assert!(entry["dominated_by_candidate_ids"].as_array().unwrap().is_empty());
    }

    #[test]
    fn unit_mismatch_rejects_the_entire_comparison() {
        let mut root = fixture();
        root["candidates"][0]["metrics"][0]["unit"] = Value::String("USD/month".to_string());
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn input_order_permutations_produce_identical_report() {
        let mut root = fixture();
        let before = serde_json::to_string(&evaluate(&root).unwrap()).unwrap();
        root["candidates"].as_array_mut().unwrap().reverse();
        for c in root["candidates"].as_array_mut().unwrap() {
            c["gates"].as_array_mut().unwrap().reverse();
            c["metrics"].as_array_mut().unwrap().reverse();
        }
        root["metric_definitions"].as_array_mut().unwrap().reverse();
        root["gate_definitions"].as_array_mut().unwrap().reverse();
        let after = serde_json::to_string(&evaluate(&root).unwrap()).unwrap();
        assert_eq!(before, after);
    }

    #[test]
    fn incomplete_metric_is_not_imputed_as_zero() {
        let root = fixture();
        let report = evaluate(&root).unwrap();
        let entry = report["candidate_reports"].as_array().unwrap().iter()
            .find(|x| x["candidate_id"] == "08-low-upfront-cost-with-unknown-lifecycle-burden").unwrap();
        assert_eq!(entry["metric_values"]["monthly_budget_burden_ratio"], Value::Null);
        assert_eq!(entry["unknown_metric_ids"][0], "monthly_budget_burden_ratio");
        assert_eq!(entry["ranking_status"], "not_rankable_incomplete_metrics");
    }

    fn independent_oracle(points: &[[f64; 3]], directions: [&str; 3]) -> Vec<usize> {
        let mut frontier = Vec::new();
        for i in 0..points.len() {
            let mut dominated = false;
            for j in 0..points.len() {
                if i == j { continue; }
                let mut every_metric_no_worse = true;
                let mut one_metric_strictly_better = false;
                for k in 0..3 {
                    match directions[k] {
                        "minimize" => {
                            if points[j][k] > points[i][k] { every_metric_no_worse = false; }
                            if points[j][k] < points[i][k] { one_metric_strictly_better = true; }
                        }
                        "maximize" => {
                            if points[j][k] < points[i][k] { every_metric_no_worse = false; }
                            if points[j][k] > points[i][k] { one_metric_strictly_better = true; }
                        }
                        _ => unreachable!()
                    }
                }
                if every_metric_no_worse && one_metric_strictly_better {
                    dominated = true;
                    break;
                }
            }
            if !dominated { frontier.push(i); }
        }
        frontier
    }

    #[test]
    fn pareto_frontier_matches_independent_oracle_for_all_four_point_subsets_of_binary_cube() {
        let corners: Vec<[f64; 3]> = (0..8).map(|mask| [
            (mask & 1) as f64,
            ((mask >> 1) & 1) as f64,
            ((mask >> 2) & 1) as f64
        ]).collect();
        for a in 0..8 {
            for b in (a + 1)..8 {
                for c in (b + 1)..8 {
                    for d in (c + 1)..8 {
                        let chosen = [a, b, c, d];
                        let mut root = fixture();
                        root["candidates"].as_array_mut().unwrap().truncate(4);
                        for (i, corner_index) in chosen.iter().enumerate() {
                            let candidate = &mut root["candidates"][i];
                            candidate["candidate_id"] = Value::String(format!("corner-{:03}", corner_index));
                            let point = corners[*corner_index];
                            let values: BTreeMap<&str, f64> = [
                                ("monthly_budget_burden_ratio", point[0]),
                                ("essential_trip_coverage", point[1]),
                                ("repair_downtime_hours_month", point[2])
                            ].into_iter().collect();
                            for metric in candidate["metrics"].as_array_mut().unwrap() {
                                if let Some(value) = values.get(metric["metric_id"].as_str().unwrap()) {
                                    metric["value"] = Value::from(*value);
                                } else {
                                    // Hold the remaining objective dimensions equal so the
                                    // independent oracle tests only the declared 3D cube.
                                    metric["value"] = Value::from(1.0);
                                }
                                metric["status"] = Value::String("synthetic_value".to_string());
                            }
                        }
                        let directions = ["minimize", "maximize", "minimize"];
                        let oracle_indices = independent_oracle(
                            &chosen.iter().map(|i| corners[*i]).collect::<Vec<_>>(),
                            directions
                        );
                        let expected: BTreeSet<String> = oracle_indices.iter()
                            .map(|i| format!("corner-{:03}", chosen[*i])).collect();
                        let actual: BTreeSet<String> = report_frontier(&root).into_iter().collect();
                        assert_eq!(actual, expected, "oracle mismatch for corner subset {chosen:?}");
                    }
                }
            }
        }
    }

    #[test]
    fn undeclared_fields_and_duplicate_candidate_ids_fail_closed() {
        let mut root = fixture();
        root["candidates"][0]["weighted_best_score"] = Value::from(999);
        assert!(evaluate(&root).is_err());

        let mut root = fixture();
        let first_id = root["candidates"][0]["candidate_id"].clone();
        root["candidates"][1]["candidate_id"] = first_id;
        assert!(evaluate(&root).is_err());
    }
}
