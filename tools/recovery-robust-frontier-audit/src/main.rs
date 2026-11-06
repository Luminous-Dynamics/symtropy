use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const EMBEDDED_CORPUS: &str =
    include_str!("../../../docs/tech/recovery-robust-frontier-corpus-v0.1.json");

#[derive(Debug, Clone)]
struct Interval {
    lower: i64,
    upper: i64,
}

#[derive(Debug, Clone)]
struct Plan {
    id: String,
    feasible: bool,
    intervals: BTreeMap<String, Interval>,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct ObjectiveMargin {
    objective_id: String,
    unit: String,
    dominator_upper: i64,
    dominated_lower: i64,
    margin: i64,
}

#[derive(Debug, Serialize, Clone, PartialEq, Eq)]
struct DominanceCertificate {
    dominator: String,
    dominated: String,
    objective_margins: Vec<ObjectiveMargin>,
    theorem: String,
}

#[derive(Debug, Serialize, Clone)]
struct GateDecision {
    gate: String,
    status: String,
    reason_code: String,
}

#[derive(Debug, Serialize)]
struct IneligiblePlan {
    id: String,
    gate_decisions: Vec<GateDecision>,
    reasons: Vec<String>,
}

#[derive(Debug, Serialize)]
struct AuditReceipt {
    result: String,
    schema_version: String,
    algorithm_id: String,
    profile_id: String,
    case_id: String,
    corpus_sha256: String,
    source_commit: Option<String>,
    plan_count: usize,
    objective_count: usize,
    feasible_plan_count: usize,
    feasible_plan_ids: Vec<String>,
    ineligible_plans: Vec<IneligiblePlan>,
    frontier: Vec<String>,
    dominance_certificates: Vec<DominanceCertificate>,
    expected_frontier_matches: bool,
    expected_certificates_match: bool,
    all_plan_only: bool,
    claim_ceiling: String,
    errors: Vec<String>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect::<Vec<_>>().join("")
}

fn nonempty_string<'a>(value: Option<&'a Value>) -> Option<&'a str> {
    value.and_then(Value::as_str).filter(|value| !value.trim().is_empty())
}

fn count(map: &mut BTreeMap<String, usize>, key: impl Into<String>) {
    *map.entry(key.into()).or_insert(0) += 1;
}

fn is_hex_sha(value: &str, digits: usize) -> bool {
    value.len() == digits && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn string_array(value: Option<&Value>, label: &str, errors: &mut Vec<String>) -> Vec<String> {
    match value.and_then(Value::as_array) {
        Some(values) => {
            let mut output = Vec::new();
            for (index, item) in values.iter().enumerate() {
                match nonempty_string(Some(item)) {
                    Some(value) => output.push(value.to_string()),
                    None => errors.push(format!("{label}_ITEM_{}_INVALID", index + 1)),
                }
            }
            output.sort();
            if output.windows(2).any(|pair| pair[0] == pair[1]) {
                errors.push(format!("{label}_DUPLICATE_ITEMS"));
            }
            output
        }
        None => {
            errors.push(format!("{label}_MISSING_OR_NOT_ARRAY"));
            Vec::new()
        }
    }
}

fn robustly_dominates(
    a: &Plan,
    b: &Plan,
    objective_units: &BTreeMap<String, String>,
) -> Option<Vec<ObjectiveMargin>> {
    if !a.feasible || !b.feasible || a.id == b.id {
        return None;
    }

    let mut margins = Vec::new();
    let mut strictly_better = false;

    for (objective_id, unit) in objective_units {
        let a_interval = a.intervals.get(objective_id)?;
        let b_interval = b.intervals.get(objective_id)?;
        // Strict interval separation: every possible A value is no worse than
        // every possible B value for each minimize objective.
        if a_interval.upper > b_interval.lower {
            return None;
        }
        let margin = b_interval.lower - a_interval.upper;
        if margin > 0 {
            strictly_better = true;
        }
        margins.push(ObjectiveMargin {
            objective_id: objective_id.clone(),
            unit: unit.clone(),
            dominator_upper: a_interval.upper,
            dominated_lower: b_interval.lower,
            margin,
        });
    }

    if strictly_better { Some(margins) } else { None }
}

fn sorted_frontier(plans: &[Plan], objective_units: &BTreeMap<String, String>) -> (Vec<String>, Vec<DominanceCertificate>) {
    let feasible: Vec<&Plan> = plans.iter().filter(|plan| plan.feasible).collect();
    let mut dominated_by = BTreeSet::new();
    let mut certificates = Vec::new();

    for a in &feasible {
        for b in &feasible {
            if let Some(margins) = robustly_dominates(a, b, objective_units) {
                dominated_by.insert(b.id.clone());
                certificates.push(DominanceCertificate {
                    dominator: a.id.clone(),
                    dominated: b.id.clone(),
                    objective_margins: margins,
                    theorem: "For every minimize objective, dominator.upper <= dominated.lower; at least one margin is strictly positive.".to_string(),
                });
            }
        }
    }

    certificates.sort_by(|a, b| {
        a.dominator.cmp(&b.dominator)
            .then_with(|| a.dominated.cmp(&b.dominated))
    });
    let mut frontier: Vec<String> = feasible
        .iter()
        .filter(|plan| !dominated_by.contains(&plan.id))
        .map(|plan| plan.id.clone())
        .collect();
    frontier.sort();
    (frontier, certificates)
}

fn expected_dominance_pairs(root: &Value, errors: &mut Vec<String>) -> Vec<(String, String)> {
    match root.get("expected_dominance_certificates").and_then(Value::as_array) {
        Some(entries) => {
            let mut pairs = Vec::new();
            for (index, entry) in entries.iter().enumerate() {
                let label = format!("EXPECTED_CERTIFICATE_{}", index + 1);
                let dominator = nonempty_string(entry.get("dominator"));
                let dominated = nonempty_string(entry.get("dominated"));
                match (dominator, dominated) {
                    (Some(a), Some(b)) if a != b => pairs.push((a.to_string(), b.to_string())),
                    _ => errors.push(format!("{label}_INVALID")),
                }
            }
            pairs.sort();
            if pairs.windows(2).any(|pair| pair[0] == pair[1]) {
                errors.push("EXPECTED_CERTIFICATE_DUPLICATE".to_string());
            }
            pairs
        }
        None => {
            errors.push("EXPECTED_CERTIFICATES_MISSING_OR_NOT_ARRAY".to_string());
            Vec::new()
        }
    }
}

fn make_receipt(
    raw: &[u8],
    source_commit: Option<String>,
    parsed: Option<Value>,
    parse_error: Option<String>,
) -> AuditReceipt {
    let had_parse_error = parse_error.is_some();
    let mut errors = Vec::new();
    let mut objective_units: BTreeMap<String, String> = BTreeMap::new();
    let mut plans: Vec<Plan> = Vec::new();
    let mut ineligible_plans = Vec::new();
    let mut plan_ids = BTreeSet::new();
    let mut plan_count = 0usize;
    let mut profile_id = "unavailable".to_string();
    let mut case_id = "unavailable".to_string();
    let mut schema_version = "unavailable".to_string();
    let mut algorithm_id = "unavailable".to_string();
    let mut all_plan_only = true;

    if let Some(error) = parse_error {
        errors.push(format!("CORPUS_JSON_INVALID: {error}"));
    }
    if let Some(commit) = source_commit.as_deref() {
        if !is_hex_sha(commit, 40) {
            errors.push("SOURCE_COMMIT_INVALID: expected 40 hexadecimal characters".to_string());
        }
    }

    if let Some(root) = parsed.as_ref() {
        schema_version = nonempty_string(root.get("schema_version")).unwrap_or("unavailable").to_string();
        algorithm_id = nonempty_string(root.get("algorithm_id")).unwrap_or("unavailable").to_string();
        profile_id = nonempty_string(root.get("decision_profile_id")).unwrap_or("unavailable").to_string();
        case_id = nonempty_string(root.get("case_id")).unwrap_or("unavailable").to_string();

        if schema_version != "robust-recovery-frontier-v0.1" {
            errors.push("SCHEMA_VERSION_UNSUPPORTED".to_string());
        }
        if algorithm_id != "interval-box-strict-dominance-v0.1" {
            errors.push("ALGORITHM_ID_UNSUPPORTED".to_string());
        }
        if profile_id == "unavailable" || case_id == "unavailable" {
            errors.push("PROFILE_OR_CASE_ID_MISSING".to_string());
        }
        if nonempty_string(root.get("status")) != Some("synthetic-logic-fixture") {
            errors.push("CORPUS_STATUS_UNEXPECTED".to_string());
        }

        let claim = root.get("claim_ceiling");
        all_plan_only = claim.and_then(|v| v.get("expected_actuation")).and_then(Value::as_bool) == Some(false);
        if !all_plan_only {
            errors.push("ACTUATION_CLAIM_MUST_BE_FALSE".to_string());
        }
        if claim.and_then(|v| v.get("real_world_measurements")).and_then(Value::as_bool) != Some(false) {
            errors.push("REAL_WORLD_MEASUREMENTS_MUST_NOT_BE_CLAIMED".to_string());
        }
        if claim.and_then(|v| v.get("field_accuracy_claimed")).and_then(Value::as_bool) != Some(false) {
            errors.push("FIELD_ACCURACY_MUST_NOT_BE_CLAIMED".to_string());
        }
        if nonempty_string(claim.and_then(|v| v.get("execution_authority"))) != Some("NO_ACTUATION_AUTHORIZED") {
            errors.push("EXECUTION_AUTHORITY_INVALID".to_string());
        }

        match root.get("objective_schema").and_then(Value::as_array) {
            Some(objectives) if !objectives.is_empty() => {
                for (index, objective) in objectives.iter().enumerate() {
                    let label = format!("OBJECTIVE_{}", index + 1);
                    let id = nonempty_string(objective.get("id"));
                    let unit = nonempty_string(objective.get("unit"));
                    let direction = nonempty_string(objective.get("direction"));
                    if direction != Some("minimize") {
                        errors.push(format!("{label}_DIRECTION_UNSUPPORTED"));
                    }
                    match (id, unit) {
                        (Some(id), Some(unit)) => {
                            if objective_units.insert(id.to_string(), unit.to_string()).is_some() {
                                errors.push(format!("{label}_DUPLICATE_ID"));
                            }
                        }
                        _ => errors.push(format!("{label}_ID_OR_UNIT_MISSING")),
                    }
                }
            }
            _ => errors.push("OBJECTIVE_SCHEMA_MISSING_OR_EMPTY".to_string()),
        }

        let expected_feasible = string_array(root.get("expected_feasible_plan_ids"), "EXPECTED_FEASIBLE_IDS", &mut errors);
        let expected_frontier = string_array(root.get("expected_frontier"), "EXPECTED_FRONTIER", &mut errors);
        let expected_pairs = expected_dominance_pairs(root, &mut errors);

        match root.get("plans").and_then(Value::as_array) {
            Some(plan_values) if !plan_values.is_empty() => {
                plan_count = plan_values.len();
                if root.get("plan_count").and_then(Value::as_u64) != Some(plan_count as u64) {
                    errors.push(format!("PLAN_COUNT_MISMATCH: declared {}, actual {plan_count}",
                        root.get("plan_count").and_then(Value::as_u64).map(|v| v.to_string()).unwrap_or_else(|| "missing".to_string())));
                }

                for (index, value) in plan_values.iter().enumerate() {
                    let label = format!("PLAN_{}", index + 1);
                    let id = match nonempty_string(value.get("id")) {
                        Some(id) => id.to_string(),
                        None => {
                            errors.push(format!("{label}_ID_MISSING"));
                            continue;
                        }
                    };
                    if !plan_ids.insert(id.clone()) {
                        errors.push(format!("{id}: DUPLICATE_PLAN_ID"));
                    }
                    if nonempty_string(value.get("domain_profile_id")) != Some(profile_id.as_str()) {
                        errors.push(format!("{id}: DOMAIN_PROFILE_MISMATCH"));
                    }

                    let gates = match value.get("mandatory_gates").and_then(Value::as_array) {
                        Some(gates) if !gates.is_empty() => gates,
                        _ => {
                            errors.push(format!("{id}: MANDATORY_GATES_MISSING_OR_EMPTY"));
                            continue;
                        }
                    };
                    let mut gate_names = BTreeSet::new();
                    let mut plan_is_feasible = true;
                    let mut exclusion_reasons = Vec::new();
                    let mut gate_decisions = Vec::new();
                    for (gate_index, gate) in gates.iter().enumerate() {
                        let gate_label = format!("{id}: GATE_{}", gate_index + 1);
                        let gate_name = nonempty_string(gate.get("gate"));
                        let reason = nonempty_string(gate.get("reason_code"));
                        if let Some(name) = gate_name {
                            if !gate_names.insert(name.to_string()) {
                                errors.push(format!("{gate_label}_DUPLICATE_NAME"));
                            }
                        } else {
                            errors.push(format!("{gate_label}_NAME_MISSING"));
                        }
                        let status_value = nonempty_string(gate.get("status"));
                        let reason_value = reason.unwrap_or("REASON_CODE_MISSING");
                        if reason.is_none() {
                            errors.push(format!("{gate_label}_REASON_CODE_MISSING"));
                        }
                        match status_value {
                            Some("PASS") => {}
                            Some("BLOCK") => {
                                plan_is_feasible = false;
                                exclusion_reasons.push(reason_value.to_string());
                            }
                            Some("UNKNOWN") => {
                                plan_is_feasible = false;
                                exclusion_reasons.push(reason_value.to_string());
                            }
                            Some(other) => errors.push(format!("{gate_label}_STATUS_UNSUPPORTED: {other}")),
                            None => errors.push(format!("{gate_label}_STATUS_MISSING")),
                        }
                        gate_decisions.push(GateDecision {
                            gate: gate_name.unwrap_or("GATE_NAME_MISSING").to_string(),
                            status: status_value.unwrap_or("STATUS_MISSING").to_string(),
                            reason_code: reason_value.to_string(),
                        });
                    }

                    match value.get("expected_feasible").and_then(Value::as_bool) {
                        Some(expected) if expected == plan_is_feasible => {}
                        Some(_) => errors.push(format!("{id}: EXPECTED_FEASIBILITY_MISMATCH")),
                        None => errors.push(format!("{id}: EXPECTED_FEASIBILITY_MISSING")),
                    }

                    if !plan_is_feasible {
                        exclusion_reasons.sort();
                        exclusion_reasons.dedup();
                        ineligible_plans.push(IneligiblePlan {
                            id: id.clone(),
                            gate_decisions: gate_decisions.clone(),
                            reasons: exclusion_reasons,
                        });
                    }

                    let intervals = match value.get("objective_intervals").and_then(Value::as_object) {
                        Some(intervals) => intervals,
                        None => {
                            errors.push(format!("{id}: OBJECTIVE_INTERVALS_MISSING_OR_INVALID"));
                            continue;
                        }
                    };
                    for objective_id in intervals.keys() {
                        if !objective_units.contains_key(objective_id) {
                            errors.push(format!("{id}: OBJECTIVE_NOT_IN_SCHEMA: {objective_id}"));
                        }
                    }
                    for objective_id in objective_units.keys() {
                        if !intervals.contains_key(objective_id) {
                            errors.push(format!("{id}: OBJECTIVE_INTERVAL_MISSING: {objective_id}"));
                        }
                    }

                    let mut parsed_intervals = BTreeMap::new();
                    let mut intervals_valid = intervals.len() == objective_units.len();
                    for (objective_id, unit) in &objective_units {
                        let interval = match intervals.get(objective_id) {
                            Some(interval) => interval,
                            None => {
                                intervals_valid = false;
                                continue;
                            }
                        };
                        if nonempty_string(interval.get("unit")) != Some(unit.as_str()) {
                            errors.push(format!("{id}: UNIT_MISMATCH: {objective_id}"));
                            intervals_valid = false;
                        }
                        let lower = interval.get("lower").and_then(Value::as_i64);
                        let upper = interval.get("upper").and_then(Value::as_i64);
                        match (lower, upper) {
                            (Some(lower), Some(upper)) if lower >= 0 && upper >= 0 && lower <= upper => {
                                parsed_intervals.insert(objective_id.clone(), Interval { lower, upper });
                            }
                            _ => {
                                errors.push(format!("{id}: INTERVAL_BOUNDS_INVALID: {objective_id}"));
                                intervals_valid = false;
                            }
                        }
                    }

                    if intervals_valid {
                        plans.push(Plan { id, feasible: plan_is_feasible, intervals: parsed_intervals });
                    }
                }
            }
            _ => errors.push("PLANS_MISSING_OR_EMPTY".to_string()),
        }

        let mut computed_feasible: Vec<String> = plans.iter().filter(|plan| plan.feasible).map(|plan| plan.id.clone()).collect();
        computed_feasible.sort();
        if computed_feasible.is_empty() {
            errors.push("NO_FEASIBLE_PLANS".to_string());
        }
        if computed_feasible != expected_feasible {
            errors.push("EXPECTED_FEASIBLE_PLAN_SET_MISMATCH".to_string());
        }
        if objective_units.is_empty() {
            errors.push("NO_VALID_OBJECTIVES".to_string());
        }

        let (frontier, certificates) = sorted_frontier(&plans, &objective_units);
        let expected_frontier_matches = frontier == expected_frontier;
        if !expected_frontier_matches {
            errors.push("EXPECTED_FRONTIER_MISMATCH".to_string());
        }
        let actual_pairs: Vec<(String, String)> = certificates.iter()
            .map(|certificate| (certificate.dominator.clone(), certificate.dominated.clone()))
            .collect();
        let expected_certificates_match = actual_pairs == expected_pairs;
        if !expected_certificates_match {
            errors.push("EXPECTED_DOMINANCE_CERTIFICATES_MISMATCH".to_string());
        }

        ineligible_plans.sort_by(|a, b| a.id.cmp(&b.id));

        AuditReceipt {
            result: if errors.is_empty() { "pass" } else { "fail" }.to_string(),
            schema_version,
            algorithm_id,
            profile_id,
            case_id,
            corpus_sha256: sha256_hex(raw),
            source_commit,
            plan_count,
            objective_count: objective_units.len(),
            feasible_plan_count: computed_feasible.len(),
            feasible_plan_ids: computed_feasible,
            ineligible_plans,
            frontier,
            dominance_certificates: certificates,
            expected_frontier_matches,
            expected_certificates_match,
            all_plan_only,
            claim_ceiling: "NO_ACTUATION_AUTHORIZED".to_string(),
            errors,
        }
    } else if !had_parse_error {
        errors.push("CORPUS_VALUE_MISSING".to_string());
        AuditReceipt {
            result: "fail".to_string(), schema_version, algorithm_id, profile_id, case_id,
            corpus_sha256: sha256_hex(raw), source_commit, plan_count, objective_count: 0,
            feasible_plan_count: 0, feasible_plan_ids: Vec::new(), ineligible_plans: Vec::new(),
            frontier: Vec::new(), dominance_certificates: Vec::new(), expected_frontier_matches: false,
            expected_certificates_match: false, all_plan_only: false,
            claim_ceiling: "NO_ACTUATION_AUTHORIZED".to_string(), errors,
        }
    } else {
        AuditReceipt {
            result: "fail".to_string(), schema_version, algorithm_id, profile_id, case_id,
            corpus_sha256: sha256_hex(raw), source_commit, plan_count, objective_count: 0,
            feasible_plan_count: 0, feasible_plan_ids: Vec::new(), ineligible_plans: Vec::new(),
            frontier: Vec::new(), dominance_certificates: Vec::new(), expected_frontier_matches: false,
            expected_certificates_match: false, all_plan_only: false,
            claim_ceiling: "NO_ACTUATION_AUTHORIZED".to_string(), errors,
        }
    }
}

fn run() -> Result<(), String> {
    let mut path: Option<String> = None;
    let mut source_commit: Option<String> = None;
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source-commit" => source_commit = Some(args.next().ok_or_else(|| "--source-commit requires a value".to_string())?),
            "--help" | "-h" => {
                println!("Usage: recovery-robust-frontier-audit <corpus.json> [--source-commit <40-hex-sha>]");
                return Ok(());
            }
            _ if arg.starts_with('-') => return Err(format!("unknown argument: {arg}")),
            _ if path.is_none() => path = Some(arg),
            _ => return Err("provide exactly one corpus path".to_string()),
        }
    }
    let path = path.ok_or_else(|| "missing corpus path; use --help for usage".to_string())?;
    let bytes = fs::read(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let receipt = match serde_json::from_slice::<Value>(&bytes) {
        Ok(value) => make_receipt(&bytes, source_commit, Some(value), None),
        Err(error) => make_receipt(&bytes, source_commit, None, Some(error.to_string())),
    };
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|error| format!("receipt serialization failed: {error}"))?);
    if receipt.result != "pass" { return Err("robust frontier audit failed".to_string()); }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => { eprintln!("recovery-robust-frontier-audit: {error}"); ExitCode::FAILURE }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn embedded() -> Value {
        serde_json::from_str(EMBEDDED_CORPUS).expect("frozen robust-frontier corpus must parse")
    }

    fn audit(value: Value) -> AuditReceipt {
        let raw = serde_json::to_vec(&value).expect("test input serializes");
        make_receipt(&raw, None, Some(value), None)
    }

    fn independent_exhaustive_frontier(value: &Value) -> (Vec<String>, Vec<(String, String)>) {
        // Deliberately separate test oracle: derive feasibility from gate rows,
        // then enumerate each ordered plan pair and compare raw bounds directly.
        let objective_ids: Vec<String> = value["objective_schema"]
            .as_array().unwrap().iter()
            .map(|objective| objective["id"].as_str().unwrap().to_string())
            .collect();
        let plans: Vec<(String, bool, BTreeMap<String, (i64, i64)>)> = value["plans"]
            .as_array().unwrap().iter().map(|plan| {
                let id = plan["id"].as_str().unwrap().to_string();
                let feasible = plan["mandatory_gates"].as_array().unwrap().iter()
                    .all(|gate| gate["status"].as_str() == Some("PASS"));
                let mut values = BTreeMap::new();
                for objective_id in &objective_ids {
                    let interval = &plan["objective_intervals"][objective_id];
                    values.insert(
                        objective_id.clone(),
                        (interval["lower"].as_i64().unwrap(), interval["upper"].as_i64().unwrap())
                    );
                }
                (id, feasible, values)
            }).collect();

        let mut dominated = BTreeSet::new();
        let mut pairs = Vec::new();
        for (a_id, a_feasible, a_values) in &plans {
            if !a_feasible { continue; }
            for (b_id, b_feasible, b_values) in &plans {
                if !b_feasible || a_id == b_id { continue; }
                let mut every_dimension_separated = true;
                let mut one_strict_margin = false;
                for objective_id in &objective_ids {
                    let a_upper = a_values[objective_id].1;
                    let b_lower = b_values[objective_id].0;
                    if a_upper > b_lower {
                        every_dimension_separated = false;
                        break;
                    }
                    if a_upper < b_lower {
                        one_strict_margin = true;
                    }
                }
                if every_dimension_separated && one_strict_margin {
                    dominated.insert(b_id.clone());
                    pairs.push((a_id.clone(), b_id.clone()));
                }
            }
        }
        pairs.sort();
        let mut frontier: Vec<String> = plans.iter()
            .filter(|(id, feasible, _)| *feasible && !dominated.contains(id))
            .map(|(id, _, _)| id.clone())
            .collect();
        frontier.sort();
        (frontier, pairs)
    }

    #[test]
    fn frozen_corpus_passes_and_frontier_is_not_a_single_winner() {
        let receipt = make_receipt(EMBEDDED_CORPUS.as_bytes(), Some("b".repeat(40)), Some(embedded()), None);
        assert_eq!(receipt.result, "pass", "{:?}", receipt.errors);
        assert_eq!(receipt.plan_count, 6);
        assert_eq!(receipt.objective_count, 3);
        assert_eq!(receipt.feasible_plan_count, 4);
        assert_eq!(receipt.frontier, vec![
            "fast_local_specialist".to_string(),
            "local_sorted_reuse".to_string(),
            "uncertain_remote_extraction".to_string()
        ]);
        assert_eq!(receipt.dominance_certificates.len(), 1);
        assert!(receipt.all_plan_only);
        assert_eq!(receipt.claim_ceiling, "NO_ACTUATION_AUTHORIZED");
        assert!(receipt.corpus_sha256.len() == 64);
    }

    #[test]
    fn block_gate_excludes_a_nominally_cheapest_plan() {
        let receipt = audit(embedded());
        assert!(!receipt.feasible_plan_ids.contains(&"unsafe_quick_disassembly".to_string()));
        assert!(receipt.ineligible_plans.iter().any(|plan|
            plan.id == "unsafe_quick_disassembly" &&
            plan.reasons.contains(&"MANDATORY_SAFETY_REVIEW_BLOCKED".to_string())
        ));
    }

    #[test]
    fn unknown_permission_never_enters_feasible_set() {
        let receipt = audit(embedded());
        assert!(!receipt.feasible_plan_ids.contains(&"permit_unknown_airlift".to_string()));
    }

    #[test]
    fn an_interval_separated_plan_has_a_margin_certificate_for_every_objective() {
        let receipt = audit(embedded());
        let certificate = receipt.dominance_certificates.first().expect("one certificate expected");
        assert_eq!(certificate.dominator, "local_sorted_reuse");
        assert_eq!(certificate.dominated, "remote_haul_and_process");
        assert_eq!(certificate.objective_margins.len(), 3);
        assert!(certificate.objective_margins.iter().all(|margin| margin.margin >= 0));
        assert!(certificate.objective_margins.iter().any(|margin| margin.margin > 0));
    }

    #[test]
    fn production_frontier_matches_independent_exhaustive_enumerator() {
        let corpus = embedded();
        let expected = independent_exhaustive_frontier(&corpus);
        let receipt = audit(corpus);
        let actual_pairs: Vec<(String, String)> = receipt.dominance_certificates.iter()
            .map(|certificate| (certificate.dominator.clone(), certificate.dominated.clone()))
            .collect();
        assert_eq!(receipt.frontier, expected.0);
        assert_eq!(actual_pairs, expected.1);
    }

    #[test]
    fn plan_order_permutation_does_not_change_frontier_or_certificates() {
        let mut corpus = embedded();
        corpus["plans"].as_array_mut().unwrap().reverse();
        let receipt = audit(corpus);
        assert_eq!(receipt.frontier, vec![
            "fast_local_specialist".to_string(),
            "local_sorted_reuse".to_string(),
            "uncertain_remote_extraction".to_string()
        ]);
        assert_eq!(receipt.dominance_certificates.len(), 1);
    }

    #[test]
    fn adding_a_tradeoff_objective_can_remove_a_dominance_claim() {
        let mut corpus = embedded();
        corpus["objective_schema"].as_array_mut().unwrap().push(json!({
            "id":"worker_exposure", "unit":"synthetic_exposure_points",
            "direction":"minimize", "interpretation":"synthetic burden"
        }));
        let values = [
            ("local_sorted_reuse", 8, 9),
            ("remote_haul_and_process", 3, 4),
            ("fast_local_specialist", 5, 6),
            ("unsafe_quick_disassembly", 0, 0),
            ("permit_unknown_airlift", 0, 0),
            ("uncertain_remote_extraction", 0, 10)
        ];
        for (id, lower, upper) in values {
            let plan = corpus["plans"].as_array_mut().unwrap().iter_mut()
                .find(|plan| plan["id"] == id).unwrap();
            plan["objective_intervals"]["worker_exposure"] = json!({
                "unit":"synthetic_exposure_points", "lower":lower, "upper":upper
            });
        }
        let receipt = audit(corpus);
        assert!(!receipt.dominance_certificates.iter().any(|c|
            c.dominator == "local_sorted_reuse" && c.dominated == "remote_haul_and_process"
        ));
    }

    #[test]
    fn widening_the_dominators_uncertainty_cannot_create_stronger_dominance() {
        let mut corpus = embedded();
        corpus["plans"].as_array_mut().unwrap().iter_mut()
            .find(|plan| plan["id"] == "local_sorted_reuse").unwrap()
            ["objective_intervals"]["total_cost"]["upper"] = json!(200);
        let receipt = audit(corpus);
        assert!(!receipt.dominance_certificates.iter().any(|c|
            c.dominator == "local_sorted_reuse" && c.dominated == "remote_haul_and_process"
        ));
    }

    #[test]
    fn reversed_interval_is_rejected() {
        let mut corpus = embedded();
        corpus["plans"][0]["objective_intervals"]["total_cost"]["lower"] = json!(999);
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("INTERVAL_BOUNDS_INVALID")));
    }

    #[test]
    fn objective_unit_mismatch_is_rejected() {
        let mut corpus = embedded();
        corpus["plans"][0]["objective_intervals"]["total_cost"]["unit"] = json!("kilograms");
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("UNIT_MISMATCH")));
    }

    #[test]
    fn duplicate_plan_ids_are_rejected() {
        let mut corpus = embedded();
        let duplicate_id = corpus["plans"][0]["id"].clone();
        corpus["plans"][1]["id"] = duplicate_id;
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("DUPLICATE_PLAN_ID")));
    }

    #[test]
    fn profile_mismatch_is_rejected() {
        let mut corpus = embedded();
        corpus["plans"][0]["domain_profile_id"] = json!("different_profile");
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("DOMAIN_PROFILE_MISMATCH")));
    }

    #[test]
    fn empty_feasible_set_fails_closed() {
        let mut corpus = embedded();
        for plan in corpus["plans"].as_array_mut().unwrap() {
            plan["expected_feasible"] = json!(false);
            plan["mandatory_gates"][0]["status"] = json!("UNKNOWN");
        }
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("NO_FEASIBLE_PLANS")));
    }

    #[test]
    fn equal_interval_vectors_do_not_strictly_dominate_each_other() {
        let mut corpus = embedded();
        let source = corpus["plans"][0]["objective_intervals"].clone();
        corpus["plans"][2]["objective_intervals"] = source;
        let receipt = audit(corpus);
        assert!(!receipt.dominance_certificates.iter().any(|c|
            (c.dominator == "local_sorted_reuse" && c.dominated == "fast_local_specialist") ||
            (c.dominator == "fast_local_specialist" && c.dominated == "local_sorted_reuse")
        ));
    }

    #[test]
    fn unsupported_maximize_direction_is_rejected() {
        let mut corpus = embedded();
        corpus["objective_schema"][0]["direction"] = json!("maximize");
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("DIRECTION_UNSUPPORTED")));
    }

    #[test]
    fn incorrect_expected_frontier_is_rejected() {
        let mut corpus = embedded();
        corpus["expected_frontier"] = json!(["remote_haul_and_process"]);
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("EXPECTED_FRONTIER_MISMATCH")));
    }

    #[test]
    fn adding_objectives_in_different_order_preserves_frontier() {
        let mut corpus = embedded();
        corpus["objective_schema"].as_array_mut().unwrap().reverse();
        let receipt = audit(corpus);
        assert_eq!(receipt.frontier, vec![
            "fast_local_specialist".to_string(),
            "local_sorted_reuse".to_string(),
            "uncertain_remote_extraction".to_string()
        ]);
    }

    #[test]
    fn declared_plan_count_must_match_corpus() {
        let mut corpus = embedded();
        corpus["plan_count"] = json!(5);
        let receipt = audit(corpus);
        assert!(receipt.errors.iter().any(|error| error.contains("PLAN_COUNT_MISMATCH")));
    }

    #[test]
    fn bad_source_commit_is_rejected() {
        let receipt = make_receipt(EMBEDDED_CORPUS.as_bytes(), Some("not-a-commit".to_string()), Some(embedded()), None);
        assert!(receipt.errors.iter().any(|error| error.contains("SOURCE_COMMIT_INVALID")));
    }
}
