use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use std::{env, fs, path::Path};

const EMBEDDED_CORPUS: &str =
    include_str!("../../../fixtures/mobility_robustness/mobility_interval_scenarios_v0_1.json");
const CLAIM_CEILING: &str =
    "synthetic_interval_frontiers_are_not_market_certainty_or_recommendations";

#[derive(Clone, Debug)]
struct MetricDef {
    unit: String,
    direction: String,
}

#[derive(Clone, Debug)]
struct Candidate {
    id: String,
    gate_disposition: String,
    failed_gates: Vec<String>,
    unknown_gates: Vec<String>,
    unknown_metrics: Vec<String>,
    metrics: BTreeMap<String, Option<(f64, f64)>>,
}

fn exact_keys(value: &Value, required: &[&str]) -> bool {
    let Some(object) = value.as_object() else { return false; };
    object.len() == required.len() && required.iter().all(|key| object.contains_key(*key))
}

fn push(errors: &mut Vec<String>, path: &str, message: &str) {
    errors.push(format!("{path}: {message}"));
}

fn nonempty(value: &Value, key: &str, path: &str, errors: &mut Vec<String>) -> Option<String> {
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

fn audit_input(root: &Value) -> Result<(BTreeMap<String, MetricDef>, Vec<Candidate>), Vec<String>> {
    let mut errors = Vec::new();
    if !exact_keys(root, &[
        "schema_version", "benchmark_id", "provenance", "claim_ceiling",
        "scenario", "gate_definitions", "metric_definitions", "candidates"
    ]) {
        push(&mut errors, "root", "missing or unknown root fields");
    }
    if root.get("schema_version").and_then(Value::as_str) != Some("0.1") {
        push(&mut errors, "schema_version", "expected 0.1");
    }
    if root.get("benchmark_id").and_then(Value::as_str) != Some("mobility-interval-robustness-v0.1") {
        push(&mut errors, "benchmark_id", "unexpected benchmark identity");
    }
    if root.get("claim_ceiling").and_then(Value::as_str) != Some(CLAIM_CEILING) {
        push(&mut errors, "claim_ceiling", "missing or incompatible claim ceiling");
    }

    let provenance = &root["provenance"];
    if !exact_keys(provenance, &["kind", "external_market_data_included", "notes"]) {
        push(&mut errors, "provenance", "missing or unknown fields");
    }
    if provenance.get("kind").and_then(Value::as_str) != Some("synthetic_only") {
        push(&mut errors, "provenance.kind", "only synthetic fixtures are admitted");
    }
    if provenance.get("external_market_data_included").and_then(Value::as_bool) != Some(false) {
        push(&mut errors, "provenance.external_market_data_included", "must be false");
    }
    nonempty(provenance, "notes", "provenance", &mut errors);

    let scenario = &root["scenario"];
    let scenario_keys = ["scenario_id", "context_id", "service_unit", "intended_user_segment"];
    if !exact_keys(scenario, &scenario_keys) {
        push(&mut errors, "scenario", "missing or unknown fields");
    }
    for key in scenario_keys {
        nonempty(scenario, key, "scenario", &mut errors);
    }

    let mut gate_ids = BTreeSet::new();
    match root.get("gate_definitions").and_then(Value::as_array) {
        Some(defs) if !defs.is_empty() => {
            for (i, def) in defs.iter().enumerate() {
                let path = format!("gate_definitions[{i}]");
                if !exact_keys(def, &["gate_id", "meaning", "hard_gate"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                if let Some(id) = nonempty(def, "gate_id", &path, &mut errors) {
                    if !gate_ids.insert(id) {
                        push(&mut errors, &path, "duplicate gate ID");
                    }
                }
                nonempty(def, "meaning", &path, &mut errors);
                if def.get("hard_gate").and_then(Value::as_bool) != Some(true) {
                    push(&mut errors, &format!("{path}.hard_gate"), "all declared gates must be hard gates");
                }
            }
        }
        _ => push(&mut errors, "gate_definitions", "expected non-empty array"),
    }

    let mut metric_defs: BTreeMap<String, MetricDef> = BTreeMap::new();
    match root.get("metric_definitions").and_then(Value::as_array) {
        Some(defs) if defs.len() >= 2 => {
            for (i, def) in defs.iter().enumerate() {
                let path = format!("metric_definitions[{i}]");
                if !exact_keys(def, &["metric_id", "unit", "direction", "meaning"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                let id = nonempty(def, "metric_id", &path, &mut errors);
                let unit = nonempty(def, "unit", &path, &mut errors);
                let direction = nonempty(def, "direction", &path, &mut errors);
                nonempty(def, "meaning", &path, &mut errors);
                if let Some(d) = direction.as_deref() {
                    if !["minimize", "maximize"].contains(&d) {
                        push(&mut errors, &format!("{path}.direction"), "expected minimize or maximize");
                    }
                }
                if let (Some(id), Some(unit), Some(direction)) = (id, unit, direction) {
                    if metric_defs.insert(id.clone(), MetricDef { unit, direction }).is_some() {
                        push(&mut errors, &path, &format!("duplicate metric ID {id}"));
                    }
                }
            }
        }
        _ => push(&mut errors, "metric_definitions", "expected at least two metric definitions"),
    }

    let mut candidates = Vec::new();
    let mut candidate_ids = BTreeSet::new();
    match root.get("candidates").and_then(Value::as_array) {
        Some(input_candidates) if input_candidates.len() >= 2 => {
            for (i, candidate) in input_candidates.iter().enumerate() {
                let path = format!("candidates[{i}]");
                if !exact_keys(candidate, &["candidate_id", "bundle_description", "gates", "metrics"]) {
                    push(&mut errors, &path, "missing or unknown fields");
                }
                let id = nonempty(candidate, "candidate_id", &path, &mut errors)
                    .unwrap_or_else(|| format!("invalid-candidate-{i}"));
                if !candidate_ids.insert(id.clone()) {
                    push(&mut errors, &format!("{path}.candidate_id"), "duplicate candidate ID");
                }
                nonempty(candidate, "bundle_description", &path, &mut errors);

                let mut failed_gates = BTreeSet::new();
                let mut unknown_gates = BTreeSet::new();
                let mut seen_gates = BTreeSet::new();
                match candidate.get("gates").and_then(Value::as_array) {
                    Some(gates) => {
                        for (j, gate) in gates.iter().enumerate() {
                            let gate_path = format!("{path}.gates[{j}]");
                            if !exact_keys(gate, &["gate_id", "status"]) {
                                push(&mut errors, &gate_path, "missing or unknown fields");
                            }
                            let gate_id = nonempty(gate, "gate_id", &gate_path, &mut errors);
                            let status = gate.get("status").and_then(Value::as_str);
                            if let Some(gid) = gate_id {
                                if !gate_ids.contains(&gid) {
                                    push(&mut errors, &gate_path, "undeclared gate ID");
                                }
                                if !seen_gates.insert(gid.clone()) {
                                    push(&mut errors, &gate_path, "duplicate gate ID");
                                }
                                match status {
                                    Some("pass") => {}
                                    Some("fail") => { failed_gates.insert(gid); }
                                    Some("unknown") => { unknown_gates.insert(gid); }
                                    _ => push(&mut errors, &format!("{gate_path}.status"), "expected pass, fail, or unknown"),
                                }
                            }
                        }
                    }
                    None => push(&mut errors, &format!("{path}.gates"), "expected array"),
                }
                if seen_gates != gate_ids {
                    push(&mut errors, &format!("{path}.gates"), "must contain each declared gate exactly once");
                }

                let mut metrics: BTreeMap<String, Option<(f64, f64)>> = BTreeMap::new();
                let mut seen_metrics = BTreeSet::new();
                let mut unknown_metrics = BTreeSet::new();
                match candidate.get("metrics").and_then(Value::as_array) {
                    Some(input_metrics) => {
                        for (j, metric) in input_metrics.iter().enumerate() {
                            let metric_path = format!("{path}.metrics[{j}]");
                            if !exact_keys(metric, &["metric_id", "unit", "status", "lower", "upper"]) {
                                push(&mut errors, &metric_path, "missing or unknown fields");
                            }
                            let metric_id = nonempty(metric, "metric_id", &metric_path, &mut errors);
                            let unit = nonempty(metric, "unit", &metric_path, &mut errors);
                            let status = metric.get("status").and_then(Value::as_str);
                            let mut interval = None;
                            if let Some(mid) = metric_id {
                                if !seen_metrics.insert(mid.clone()) {
                                    push(&mut errors, &metric_path, "duplicate metric ID");
                                }
                                match metric_defs.get(&mid) {
                                    None => push(&mut errors, &metric_path, "undeclared metric ID"),
                                    Some(def) => {
                                        if unit.as_deref() != Some(def.unit.as_str()) {
                                            push(&mut errors, &format!("{metric_path}.unit"), "unit does not match metric definition");
                                        }
                                        match status {
                                            Some("synthetic_interval") => {
                                                let lo = metric.get("lower").and_then(Value::as_f64);
                                                let hi = metric.get("upper").and_then(Value::as_f64);
                                                match (lo, hi) {
                                                    (Some(lo), Some(hi)) if lo.is_finite() && hi.is_finite() && lo <= hi => {
                                                        interval = Some((lo, hi));
                                                    }
                                                    (Some(lo), Some(hi)) if !lo.is_finite() || !hi.is_finite() => {
                                                        push(&mut errors, &metric_path, "interval endpoints must be finite");
                                                    }
                                                    (Some(_), Some(_)) => push(&mut errors, &metric_path, "lower endpoint must be <= upper endpoint"),
                                                    _ => push(&mut errors, &metric_path, "synthetic_interval requires numeric lower and upper endpoints"),
                                                }
                                            }
                                            Some("unknown") => {
                                                if !metric.get("lower").is_some_and(Value::is_null)
                                                    || !metric.get("upper").is_some_and(Value::is_null) {
                                                    push(&mut errors, &metric_path, "unknown intervals must use null endpoints");
                                                }
                                                unknown_metrics.insert(mid.clone());
                                            }
                                            _ => push(&mut errors, &format!("{metric_path}.status"), "expected synthetic_interval or unknown"),
                                        }
                                        metrics.insert(mid, interval);
                                    }
                                }
                            }
                        }
                    }
                    None => push(&mut errors, &format!("{path}.metrics"), "expected array"),
                }
                let declared_metric_ids: BTreeSet<String> = metric_defs.keys().cloned().collect();
                if seen_metrics != declared_metric_ids {
                    push(&mut errors, &format!("{path}.metrics"), "must contain each declared metric exactly once");
                }

                let disposition = if !failed_gates.is_empty() {
                    "blocked"
                } else if !unknown_gates.is_empty() {
                    "needs_evidence"
                } else if seen_gates.len() == gate_ids.len() {
                    "eligible"
                } else {
                    "needs_evidence"
                };
                candidates.push(Candidate {
                    id,
                    gate_disposition: disposition.to_string(),
                    failed_gates: failed_gates.into_iter().collect(),
                    unknown_gates: unknown_gates.into_iter().collect(),
                    unknown_metrics: unknown_metrics.into_iter().collect(),
                    metrics,
                });
            }
        }
        _ => push(&mut errors, "candidates", "expected at least two candidates"),
    }

    if errors.is_empty() {
        Ok((metric_defs, candidates))
    } else {
        errors.sort();
        Err(errors)
    }
}

fn fully_comparable(candidate: &Candidate, metric_count: usize) -> bool {
    candidate.metrics.len() == metric_count && candidate.metrics.values().all(Option::is_some)
}

fn dominates(a: &Candidate, b: &Candidate, defs: &BTreeMap<String, MetricDef>, robust: bool) -> bool {
    let mut strict = false;
    for (id, def) in defs {
        let Some((a_lo, a_hi)) = a.metrics.get(id).copied().flatten() else { return false; };
        let Some((b_lo, b_hi)) = b.metrics.get(id).copied().flatten() else { return false; };
        match (def.direction.as_str(), robust) {
            ("minimize", true) => {
                if a_hi > b_lo { return false; }
                if a_hi < b_lo { strict = true; }
            }
            ("maximize", true) => {
                if a_lo < b_hi { return false; }
                if a_lo > b_hi { strict = true; }
            }
            ("minimize", false) => {
                if a_lo > b_hi { return false; }
                if a_lo < b_hi { strict = true; }
            }
            ("maximize", false) => {
                if a_hi < b_lo { return false; }
                if a_hi > b_lo { strict = true; }
            }
            _ => return false,
        }
    }
    strict
}

fn order_is_ambiguous(a: (f64, f64), b: (f64, f64)) -> bool {
    // A can be strictly below B for some values and strictly above B for others.
    a.0 < b.1 && a.1 > b.0
}

fn evaluate(root: &Value) -> Result<Value, Vec<String>> {
    let (defs, mut candidates) = audit_input(root)?;
    candidates.sort_by(|a, b| a.id.cmp(&b.id));
    let eligible: Vec<&Candidate> = candidates.iter()
        .filter(|c| c.gate_disposition == "eligible" && fully_comparable(c, defs.len()))
        .collect();

    let mut robust_edges: Vec<(String, String)> = Vec::new();
    let mut possible_edges: Vec<(String, String)> = Vec::new();
    let mut robust_dominators: BTreeMap<String, Vec<String>> = BTreeMap::new();
    let mut possible_dominators: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for a in &eligible {
        for b in &eligible {
            if a.id == b.id { continue; }
            if dominates(a, b, &defs, true) {
                robust_edges.push((a.id.clone(), b.id.clone()));
                robust_dominators.entry(b.id.clone()).or_default().push(a.id.clone());
            }
            if dominates(a, b, &defs, false) {
                possible_edges.push((a.id.clone(), b.id.clone()));
                possible_dominators.entry(b.id.clone()).or_default().push(a.id.clone());
            }
        }
    }

    let robust_frontier: Vec<String> = eligible.iter()
        .filter(|c| !robust_dominators.contains_key(&c.id))
        .map(|c| c.id.clone()).collect();
    let stable_frontier: Vec<String> = eligible.iter()
        .filter(|c| !possible_dominators.contains_key(&c.id))
        .map(|c| c.id.clone()).collect();
    let robust_set: BTreeSet<String> = robust_frontier.iter().cloned().collect();
    let stable_set: BTreeSet<String> = stable_frontier.iter().cloned().collect();
    let uncertain_frontier: Vec<String> = robust_set.difference(&stable_set).cloned().collect();

    let mut measurement_priorities: Vec<Value> = Vec::new();
    for (metric_id, def) in &defs {
        let mut unresolved_pairs = 0usize;
        for i in 0..eligible.len() {
            for j in (i + 1)..eligible.len() {
                let a = eligible[i].metrics.get(metric_id).copied().flatten().unwrap();
                let b = eligible[j].metrics.get(metric_id).copied().flatten().unwrap();
                if order_is_ambiguous(a, b) { unresolved_pairs += 1; }
            }
        }
        measurement_priorities.push(json!({
            "metric_id": metric_id,
            "unit": def.unit,
            "direction": def.direction,
            "overlapping_interval_pair_count": unresolved_pairs
        }));
    }
    measurement_priorities.sort_by(|a, b| {
        let ac = a["overlapping_interval_pair_count"].as_u64().unwrap_or(0);
        let bc = b["overlapping_interval_pair_count"].as_u64().unwrap_or(0);
        bc.cmp(&ac).then_with(|| a["metric_id"].as_str().cmp(&b["metric_id"].as_str()))
    });

    let mut candidate_reports = Vec::new();
    for c in &candidates {
        let comparable = c.gate_disposition == "eligible" && fully_comparable(c, defs.len());
        let status = if c.gate_disposition == "blocked" {
            "blocked_hard_gate"
        } else if c.gate_disposition == "needs_evidence" {
            "needs_gate_evidence"
        } else if !fully_comparable(c, defs.len()) {
            "not_rankable_incomplete_intervals"
        } else if stable_set.contains(&c.id) {
            "stable_frontier_candidate"
        } else if robust_set.contains(&c.id) {
            "robust_frontier_candidate"
        } else {
            "robustly_dominated"
        };
        let metric_values: BTreeMap<String, Value> = c.metrics.iter().map(|(id, value)| {
            let v = (*value).map(|(lo, hi)| json!({"lower":lo,"upper":hi})).unwrap_or(Value::Null);
            (id.clone(), v)
        }).collect();
        candidate_reports.push(json!({
            "candidate_id": c.id,
            "gate_disposition": c.gate_disposition,
            "ranking_status": status,
            "failed_gate_ids": c.failed_gates,
            "unknown_gate_ids": c.unknown_gates,
            "unknown_metric_ids": c.unknown_metrics,
            "metric_intervals": metric_values,
            "robustly_dominated_by_candidate_ids": robust_dominators.get(&c.id).cloned().unwrap_or_default(),
            "possibly_dominated_by_candidate_ids": possible_dominators.get(&c.id).cloned().unwrap_or_default(),
            "included_in_comparison_set": comparable
        }));
    }
    candidate_reports.sort_by(|a, b| a["candidate_id"].as_str().cmp(&b["candidate_id"].as_str()));
    let excluded: Vec<String> = candidates.iter()
        .filter(|c| !(c.gate_disposition == "eligible" && fully_comparable(c, defs.len())))
        .map(|c| c.id.clone()).collect();

    Ok(json!({
        "report_version": "0.1",
        "benchmark_id": "mobility-interval-robustness-v0.1",
        "scenario_id": root["scenario"]["scenario_id"],
        "claim_ceiling": CLAIM_CEILING,
        "candidate_count": candidates.len(),
        "comparable_candidate_count": eligible.len(),
        "excluded_candidate_ids": excluded,
        "robust_frontier_candidate_ids": robust_frontier,
        "stable_frontier_candidate_ids": stable_frontier,
        "uncertain_frontier_membership_candidate_ids": uncertain_frontier,
        "robust_dominance_edge_count": robust_edges.len(),
        "possible_dominance_edge_count": possible_edges.len(),
        "measurement_priorities": measurement_priorities,
        "candidate_reports": candidate_reports,
        "weighted_utility_score_produced": false,
        "probability_model_used": false,
        "evpi_calculated": false,
        "interpretation": "Intervals are deterministic bounds, not probabilities. The frontiers use a rectangular-interval assumption and do not model cross-metric correlation. Measurement priorities count pairwise metric orderings that can reverse within intervals; they are not EVPI or an optimized survey design."
    }))
}

fn main() {
    let mut args = env::args().skip(1);
    let path = args.next().unwrap_or_else(|| "fixtures/mobility_robustness/mobility_interval_scenarios_v0_1.json".to_string());
    if args.next().is_some() {
        eprintln!("usage: mobility-interval-audit [path/to/fixture.json]");
        std::process::exit(2);
    }
    let source = if path == "-" {
        EMBEDDED_CORPUS.to_string()
    } else if Path::new(&path).exists() {
        match fs::read_to_string(&path) {
            Ok(s) => s,
            Err(e) => { eprintln!("FAIL: unable to read {path}: {e}"); std::process::exit(2); }
        }
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

    fn report(root: &Value) -> Value {
        evaluate(root).expect("fixture evaluates")
    }

    fn set_interval(candidate: &mut Value, metric_id: &str, lo: f64, hi: f64) {
        let metric = candidate["metrics"].as_array_mut().unwrap().iter_mut()
            .find(|m| m["metric_id"] == metric_id).unwrap();
        metric["status"] = json!("synthetic_interval");
        metric["lower"] = json!(lo);
        metric["upper"] = json!(hi);
    }

    #[test]
    fn fixture_excludes_blocked_unknown_and_incomplete_candidates() {
        let out = report(&fixture());
        let robust = out["robust_frontier_candidate_ids"].as_array().unwrap();
        let stable = out["stable_frontier_candidate_ids"].as_array().unwrap();
        for id in [
            "06-uncertified-low-price-import",
            "07-unconfirmed-energy-and-repair",
            "08-incomplete-budget-evidence",
        ] {
            assert!(!robust.iter().any(|v| v == id), "{id} must not reach robust frontier");
            assert!(!stable.iter().any(|v| v == id), "{id} must not reach stable frontier");
        }
        assert_eq!(out["candidate_count"], 8);
        assert_eq!(out["comparable_candidate_count"], 5);
        assert_eq!(robust.len(), 5);
        assert!(stable.is_empty(), "wide synthetic bounds should not claim a universally stable candidate");
        let priorities = out["measurement_priorities"].as_array().unwrap();
        assert_eq!(priorities[0]["metric_id"], "essential_trip_coverage");
        assert_eq!(priorities[0]["overlapping_interval_pair_count"], 10);
        assert_eq!(priorities[1]["metric_id"], "p90_access_time_minutes");
        assert_eq!(priorities[1]["overlapping_interval_pair_count"], 10);
        assert_eq!(out["weighted_utility_score_produced"], false);
        assert_eq!(out["probability_model_used"], false);
        assert_eq!(out["evpi_calculated"], false);
    }

    #[test]
    fn separated_intervals_prove_robust_and_stable_dominance() {
        let mut root = fixture();
        root["candidates"].as_array_mut().unwrap().truncate(2);
        let a_values = [
            ("monthly_budget_burden_ratio", 0.1, 0.2),
            ("essential_trip_coverage", 0.8, 0.9),
            ("repair_downtime_hours_month", 1.0, 2.0),
            ("p90_access_time_minutes", 10.0, 20.0),
            ("lifecycle_resource_index", 0.1, 0.2),
        ];
        let b_values = [
            ("monthly_budget_burden_ratio", 0.4, 0.5),
            ("essential_trip_coverage", 0.6, 0.7),
            ("repair_downtime_hours_month", 3.0, 4.0),
            ("p90_access_time_minutes", 30.0, 40.0),
            ("lifecycle_resource_index", 0.3, 0.4),
        ];
        for (candidate, values) in root["candidates"].as_array_mut().unwrap().iter_mut().zip([a_values, b_values]) {
            for (id, lo, hi) in values {
                set_interval(candidate, id, lo, hi);
            }
        }
        let out = report(&root);
        assert_eq!(out["robust_frontier_candidate_ids"].as_array().unwrap().len(), 1);
        assert_eq!(out["stable_frontier_candidate_ids"].as_array().unwrap().len(), 1);
        assert_eq!(out["robust_frontier_candidate_ids"][0], "01-compact-ev-plus-shared-charging");
        assert_eq!(out["stable_frontier_candidate_ids"][0], "01-compact-ev-plus-shared-charging");
        assert_eq!(out["robust_dominance_edge_count"], 1);
        assert_eq!(out["possible_dominance_edge_count"], 1);
    }

    #[test]
    fn crossing_intervals_are_not_misreported_as_probability_or_evpi() {
        let mut root = fixture();
        root["candidates"].as_array_mut().unwrap().truncate(2);
        set_interval(&mut root["candidates"][0], "monthly_budget_burden_ratio", 0.1, 0.8);
        set_interval(&mut root["candidates"][1], "monthly_budget_burden_ratio", 0.4, 0.7);
        let out = report(&root);
        assert_eq!(out["probability_model_used"], false);
        assert_eq!(out["evpi_calculated"], false);
        assert!(out["uncertain_frontier_membership_candidate_ids"].as_array().unwrap().len() <= 2);
    }

    #[test]
    fn reversed_intervals_fail_closed() {
        let mut root = fixture();
        set_interval(&mut root["candidates"][0], "monthly_budget_burden_ratio", 0.8, 0.2);
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn undeclared_metric_and_duplicate_candidate_ids_fail_closed() {
        let mut root = fixture();
        root["candidates"][0]["metrics"][0]["metric_id"] = json!("unknown_metric");
        assert!(evaluate(&root).is_err());
        let mut root = fixture();
        let first_id = root["candidates"][0]["candidate_id"].clone();
        root["candidates"][1]["candidate_id"] = first_id;
        assert!(evaluate(&root).is_err());
    }

    #[test]
    fn reordered_input_produces_byte_identical_json_report() {
        let mut root = fixture();
        let before = serde_json::to_string(&report(&root)).unwrap();
        root["candidates"].as_array_mut().unwrap().reverse();
        root["gate_definitions"].as_array_mut().unwrap().reverse();
        root["metric_definitions"].as_array_mut().unwrap().reverse();
        for c in root["candidates"].as_array_mut().unwrap() {
            c["gates"].as_array_mut().unwrap().reverse();
            c["metrics"].as_array_mut().unwrap().reverse();
        }
        let after = serde_json::to_string(&report(&root)).unwrap();
        assert_eq!(before, after);
    }

    fn exact_oracle(points: &[[f64; 3]]) -> BTreeSet<usize> {
        let mut frontier = BTreeSet::new();
        for i in 0..points.len() {
            let dominated = (0..points.len()).any(|j| {
                if i == j { return false; }
                let no_worse = points[j][0] <= points[i][0]
                    && points[j][1] >= points[i][1]
                    && points[j][2] <= points[i][2];
                let strict = points[j][0] < points[i][0]
                    || points[j][1] > points[i][1]
                    || points[j][2] < points[i][2];
                no_worse && strict
            });
            if !dominated { frontier.insert(i); }
        }
        frontier
    }

    #[test]
    fn degenerate_intervals_match_independent_oracle_for_all_70_binary_cube_subsets() {
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
                        for (i, corner) in chosen.iter().enumerate() {
                            let candidate = &mut root["candidates"][i];
                            candidate["candidate_id"] = json!(format!("corner-{corner:03}"));
                            let point = corners[*corner];
                            let assignments = [
                                ("monthly_budget_burden_ratio", point[0]),
                                ("essential_trip_coverage", point[1]),
                                ("repair_downtime_hours_month", point[2]),
                                ("p90_access_time_minutes", 1.0),
                                ("lifecycle_resource_index", 1.0),
                            ];
                            for (id, value) in assignments {
                                set_interval(candidate, id, value, value);
                            }
                        }
                        let points: Vec<[f64; 3]> = chosen.iter().map(|i| corners[*i]).collect();
                        let expected_indices = exact_oracle(&points);
                        let expected: BTreeSet<String> = expected_indices.iter()
                            .map(|i| format!("corner-{:03}", chosen[*i])).collect();
                        let output = report(&root);
                        let actual: BTreeSet<String> = output["robust_frontier_candidate_ids"].as_array().unwrap()
                            .iter().map(|v| v.as_str().unwrap().to_string()).collect();
                        let stable: BTreeSet<String> = output["stable_frontier_candidate_ids"].as_array().unwrap()
                            .iter().map(|v| v.as_str().unwrap().to_string()).collect();
                        assert_eq!(actual, expected, "robust frontier mismatch for {chosen:?}");
                        assert_eq!(stable, expected, "stable frontier mismatch for {chosen:?}");
                    }
                }
            }
        }
    }
}