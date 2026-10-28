use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, process};

const DEFAULT_STUDY: &str =
    "docs/medical-robotics/careframe-design-space-study-fixture-v0.1.json";
const DEFAULT_SCENARIOS: &str =
    "docs/medical-robotics/careframe-design-space-scenarios-v0.1.json";
const STUDY_SOURCE: &[u8] =
    include_bytes!("../../../docs/medical-robotics/careframe-design-space-study-fixture-v0.1.json");
const SCENARIOS_SOURCE: &[u8] =
    include_bytes!("../../../docs/medical-robotics/careframe-design-space-scenarios-v0.1.json");
const AUDITOR_SOURCE: &[u8] = include_bytes!("main.rs");
const CLAIM_CEILING: &str =
    "Pareto selection of submitted benchmark metrics only. No device safety, clinical performance or regulatory claim.";
const SCENARIO_CLAIM_CEILING: &str = "synthetic_fixture_only";

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<Vec<_>>()
        .join("")
}

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn value_string<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn value_u64(value: &Value, key: &str) -> Option<u64> {
    value.get(key).and_then(Value::as_u64)
}

fn value_f64(value: &Value, key: &str) -> Option<f64> {
    value.get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
}

fn value_bool(value: &Value, key: &str) -> Option<bool> {
    value.get(key).and_then(Value::as_bool)
}

fn exact_fields(value: &Value, required: &[&str], allowed: &[&str], path: &str) -> Result<(), String> {
    let Some(object) = value.as_object() else {
        return Err(format!("{path}: expected JSON object"));
    };
    for key in required {
        if !object.contains_key(*key) {
            return Err(format!("{path}: required field '{key}' is missing"));
        }
    }
    if let Some(unknown) = object.keys().find(|key| !allowed.contains(&key.as_str())) {
        return Err(format!("{path}: unknown field '{unknown}'"));
    }
    Ok(())
}

fn require_token(value: Option<&str>, path: &str) -> Result<String, String> {
    let Some(value) = value else {
        return Err(format!("{path}: expected string"));
    };
    let token_ok = !value.trim().is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"._-".contains(&byte));
    if !token_ok {
        return Err(format!("{path}: invalid stable identifier"));
    }
    Ok(value.to_owned())
}

fn require_sha(value: Option<&str>, path: &str) -> Result<String, String> {
    let Some(value) = value else {
        return Err(format!("{path}: expected SHA-256 string"));
    };
    if !valid_sha256(value) {
        return Err(format!("{path}: expected 64 lowercase hexadecimal characters"));
    }
    Ok(value.to_owned())
}

#[derive(Debug, Clone)]
struct Metrics {
    task_success_rate: f64,
    interventions_per_100_tasks: f64,
    p95_recovery_ms: f64,
    energy_wh_per_attempt: f64,
    maintenance_hours_per_100_tasks: f64,
}

impl Metrics {
    fn dominates(&self, other: &Self) -> bool {
        let no_worse = self.task_success_rate >= other.task_success_rate
            && self.interventions_per_100_tasks <= other.interventions_per_100_tasks
            && self.p95_recovery_ms <= other.p95_recovery_ms
            && self.energy_wh_per_attempt <= other.energy_wh_per_attempt
            && self.maintenance_hours_per_100_tasks <= other.maintenance_hours_per_100_tasks;
        let strictly_better = self.task_success_rate > other.task_success_rate
            || self.interventions_per_100_tasks < other.interventions_per_100_tasks
            || self.p95_recovery_ms < other.p95_recovery_ms
            || self.energy_wh_per_attempt < other.energy_wh_per_attempt
            || self.maintenance_hours_per_100_tasks < other.maintenance_hours_per_100_tasks;
        no_worse && strictly_better
    }

    fn as_json(&self) -> Value {
        json!({
            "task_success_rate": self.task_success_rate,
            "interventions_per_100_tasks": self.interventions_per_100_tasks,
            "p95_recovery_ms": self.p95_recovery_ms,
            "energy_wh_per_attempt": self.energy_wh_per_attempt,
            "maintenance_hours_per_100_tasks": self.maintenance_hours_per_100_tasks
        })
    }
}

#[derive(Debug, Clone)]
struct Candidate {
    id: String,
    embodiment: String,
    metrics: Metrics,
    gate_findings: Vec<String>,
}

#[derive(Debug)]
struct ScoreReport {
    study_id: String,
    evidence_class: String,
    scenario_sha256: String,
    candidate_count: usize,
    eligible_count: usize,
    pareto_front: Vec<String>,
    candidate_rows: Vec<Value>,
    blocked_candidates: Vec<Value>,
}

fn validate_scenario_set(scenario_bytes: &[u8], expected_sha: &str) -> Result<(), Vec<String>> {
    let suite: Value = serde_json::from_slice(scenario_bytes)
        .map_err(|error| vec![format!("SCENARIO_JSON_INVALID: {error}")])?;
    let allowed = ["suite_id", "suite_version", "claim_ceiling", "scenarios"];
    exact_fields(&suite, &allowed, &allowed, "scenario_set")
        .map_err(|error| vec![error])?;
    if value_string(&suite, "suite_id") != Some("CAREFRAME_DESIGN_SPACE_SCENARIOS")
        || value_string(&suite, "suite_version") != Some("0.1.0")
        || value_string(&suite, "claim_ceiling") != Some(SCENARIO_CLAIM_CEILING)
    {
        return Err(vec!["SCENARIO_SUITE_METADATA_UNSUPPORTED".to_owned()]);
    }
    if sha256_hex(scenario_bytes) != expected_sha {
        return Err(vec!["SCENARIO_SET_DIGEST_MISMATCH".to_owned()]);
    }
    let Some(scenarios) = suite.get("scenarios").and_then(Value::as_array) else {
        return Err(vec!["SCENARIOS_MUST_BE_ARRAY".to_owned()]);
    };
    if scenarios.len() < 2 || scenarios.len() > 256 {
        return Err(vec!["SCENARIO_COUNT_OUT_OF_RANGE".to_owned()]);
    }
    let mut ids = BTreeSet::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        exact_fields(
            scenario,
            &["scenario_id", "kind"],
            &["scenario_id", "kind"],
            &format!("scenario_set.scenarios[{index}]"),
        )
        .map_err(|error| vec![error])?;
        let id = require_token(
            value_string(scenario, "scenario_id"),
            &format!("scenario_set.scenarios[{index}].scenario_id"),
        )
        .map_err(|error| vec![error])?;
        if !ids.insert(id) {
            return Err(vec!["SCENARIO_ID_DUPLICATE".to_owned()]);
        }
        if !["nominal", "authority_mutation", "privacy_mutation", "environment_shift"]
            .contains(&value_string(scenario, "kind").unwrap_or(""))
        {
            return Err(vec!["SCENARIO_KIND_UNSUPPORTED".to_owned()]);
        }
    }
    Ok(())
}

fn parse_candidate(
    value: &Value,
    index: usize,
    evidence_class: &str,
    scenario_sha: &str,
    attempts_budget: u64,
    time_budget: u64,
) -> Result<Candidate, String> {
    const CANDIDATE_FIELDS: &[&str] = &[
        "candidate_id",
        "embodiment",
        "evidence_class",
        "scenario_set_sha256",
        "attempted_tasks",
        "successful_tasks",
        "allocated_time_budget_seconds",
        "intervention_count",
        "p95_recovery_ms",
        "energy_wh_per_attempt",
        "maintenance_hours_per_100_tasks",
        "hard_gates",
    ];
    exact_fields(
        value,
        CANDIDATE_FIELDS,
        CANDIDATE_FIELDS,
        &format!("candidates[{index}]"),
    )?;
    let id = require_token(
        value_string(value, "candidate_id"),
        &format!("candidates[{index}].candidate_id"),
    )?;
    let embodiment = require_token(
        value_string(value, "embodiment"),
        &format!("candidates[{index}].embodiment"),
    )?;
    if value_string(value, "evidence_class") != Some(evidence_class) {
        return Err(format!("candidates[{index}]: evidence class differs from study"));
    }
    if value_string(value, "scenario_set_sha256") != Some(scenario_sha) {
        return Err(format!("candidates[{index}]: scenario-set digest differs from study"));
    }
    const EMBODIMENTS: &[&str] = &[
        "wheeled_upper_body_humanoid",
        "wheeled_mobile_manipulator",
        "fixed_base_manipulator",
        "biped_humanoid",
    ];
    if !EMBODIMENTS.contains(&embodiment.as_str()) {
        return Err(format!("candidates[{index}]: embodiment is unsupported"));
    }
    let attempts = value_u64(value, "attempted_tasks")
        .ok_or_else(|| format!("candidates[{index}].attempted_tasks: expected unsigned integer"))?;
    let successes = value_u64(value, "successful_tasks")
        .ok_or_else(|| format!("candidates[{index}].successful_tasks: expected unsigned integer"))?;
    let allocated_time = value_u64(value, "allocated_time_budget_seconds")
        .ok_or_else(|| format!("candidates[{index}].allocated_time_budget_seconds: expected unsigned integer"))?;
    let interventions = value_u64(value, "intervention_count")
        .ok_or_else(|| format!("candidates[{index}].intervention_count: expected unsigned integer"))?;
    if attempts != attempts_budget || allocated_time != time_budget {
        return Err(format!("candidates[{index}]: candidate received a different evaluation budget"));
    }
    if successes > attempts || attempts == 0 {
        return Err(format!("candidates[{index}]: success count exceeds attempts or denominator is zero"));
    }
    let p95 = value_u64(value, "p95_recovery_ms")
        .ok_or_else(|| format!("candidates[{index}].p95_recovery_ms: expected unsigned integer"))?;
    if p95 > 3_600_000 {
        return Err(format!("candidates[{index}]: p95 recovery time exceeds the declared reportable range"));
    }
    let energy = value_f64(value, "energy_wh_per_attempt")
        .filter(|value| *value > 0.0 && *value <= 100_000.0)
        .ok_or_else(|| format!("candidates[{index}].energy_wh_per_attempt: invalid finite positive value"))?;
    let maintenance = value_f64(value, "maintenance_hours_per_100_tasks")
        .filter(|value| *value >= 0.0 && *value <= 100_000.0)
        .ok_or_else(|| format!("candidates[{index}].maintenance_hours_per_100_tasks: invalid finite non-negative value"))?;

    let gates = value.get("hard_gates")
        .ok_or_else(|| format!("candidates[{index}].hard_gates: required object missing"))?;
    const GATE_FIELDS: &[&str] = &[
        "authority_escalations_blocked",
        "stale_revision_mutations_blocked",
        "privacy_revocation_blocked",
        "safe_stop_fault_injection_passed",
        "unsafe_eligible_count",
        "physical_actuation_authorized",
        "patient_contact_authorized",
        "clinical_use_authorized",
    ];
    exact_fields(
        gates,
        GATE_FIELDS,
        GATE_FIELDS,
        &format!("candidates[{index}].hard_gates"),
    )?;
    let mut gate_findings = Vec::new();
    for (key, finding) in [
        ("authority_escalations_blocked", "AUTHORITY_ESCALATION_NOT_BLOCKED"),
        ("stale_revision_mutations_blocked", "STALE_REVISION_MUTATION_NOT_BLOCKED"),
        ("privacy_revocation_blocked", "PRIVACY_REVOCATION_NOT_BLOCKED"),
        ("safe_stop_fault_injection_passed", "SAFE_STOP_FAULT_CASE_FAILED"),
    ] {
        if value_bool(gates, key) != Some(true) {
            gate_findings.push(finding.to_owned());
        }
    }
    let unsafe_count = value_u64(gates, "unsafe_eligible_count")
        .ok_or_else(|| format!("candidates[{index}].hard_gates.unsafe_eligible_count: expected unsigned integer"))?;
    if unsafe_count != 0 {
        gate_findings.push("UNSAFE_ELIGIBLE_OUTCOME_OBSERVED".to_owned());
    }
    for (key, finding) in [
        ("physical_actuation_authorized", "PHYSICAL_ACTUATION_AUTHORITY_FORBIDDEN"),
        ("patient_contact_authorized", "PATIENT_CONTACT_AUTHORITY_FORBIDDEN"),
        ("clinical_use_authorized", "CLINICAL_USE_AUTHORITY_FORBIDDEN"),
    ] {
        if value_bool(gates, key) != Some(false) {
            gate_findings.push(finding.to_owned());
        }
    }
    let metrics = Metrics {
        task_success_rate: successes as f64 / attempts as f64,
        interventions_per_100_tasks: interventions as f64 * 100.0 / attempts as f64,
        p95_recovery_ms: p95 as f64,
        energy_wh_per_attempt: energy,
        maintenance_hours_per_100_tasks: maintenance,
    };
    Ok(Candidate { id, embodiment, metrics, gate_findings })
}

fn score_study(study_bytes: &[u8], scenario_bytes: &[u8]) -> Result<ScoreReport, Vec<String>> {
    let study: Value = serde_json::from_slice(study_bytes)
        .map_err(|error| vec![format!("STUDY_JSON_INVALID: {error}")])?;
    const ROOT_FIELDS: &[&str] = &[
        "schema_version",
        "study_id",
        "evidence_class",
        "scenario_set_sha256",
        "claim_ceiling",
        "budget",
        "candidates",
    ];
    exact_fields(&study, ROOT_FIELDS, ROOT_FIELDS, "study").map_err(|error| vec![error])?;
    if value_string(&study, "schema_version") != Some("careframe-design-study-v0") {
        return Err(vec!["STUDY_SCHEMA_VERSION_UNSUPPORTED".to_owned()]);
    }
    let study_id =
        require_token(value_string(&study, "study_id"), "study.study_id").map_err(|e| vec![e])?;
    let evidence_class = value_string(&study, "evidence_class")
        .ok_or_else(|| vec!["STUDY_EVIDENCE_CLASS_REQUIRED".to_owned()])?;
    if evidence_class != "synthetic_fixture_only" {
        return Err(vec!["STUDY_EVIDENCE_CLASS_UNSUPPORTED_IN_V0".to_owned()]);
    }
    let scenario_sha = require_sha(value_string(&study, "scenario_set_sha256"), "study.scenario_set_sha256")
        .map_err(|error| vec![error])?;
    if value_string(&study, "claim_ceiling") != Some(CLAIM_CEILING) {
        return Err(vec!["STUDY_CLAIM_CEILING_UNSUPPORTED".to_owned()]);
    }
    validate_scenario_set(scenario_bytes, &scenario_sha)?;
    let budget = study.get("budget").ok_or_else(|| vec!["STUDY_BUDGET_REQUIRED".to_owned()])?;
    const BUDGET_FIELDS: &[&str] = &[
        "task_attempts_per_candidate",
        "time_budget_seconds_per_candidate",
        "same_budget_for_all_candidates",
    ];
    exact_fields(budget, BUDGET_FIELDS, BUDGET_FIELDS, "study.budget")
        .map_err(|error| vec![error])?;
    let attempts_budget = value_u64(budget, "task_attempts_per_candidate")
        .filter(|value| (1..=1_000_000).contains(value))
        .ok_or_else(|| vec!["TASK_ATTEMPTS_BUDGET_INVALID".to_owned()])?;
    let time_budget = value_u64(budget, "time_budget_seconds_per_candidate")
        .filter(|value| (1..=86_400).contains(value))
        .ok_or_else(|| vec!["TIME_BUDGET_INVALID".to_owned()])?;
    if value_bool(budget, "same_budget_for_all_candidates") != Some(true) {
        return Err(vec!["CANDIDATES_MUST_SHARE_EVALUATION_BUDGET".to_owned()]);
    }

    let candidates = study.get("candidates")
        .and_then(Value::as_array)
        .ok_or_else(|| vec!["CANDIDATES_MUST_BE_ARRAY".to_owned()])?;
    if !(2..=64).contains(&candidates.len()) {
        return Err(vec!["CANDIDATE_COUNT_OUT_OF_RANGE".to_owned()]);
    }
    let mut parsed = Vec::new();
    let mut ids = BTreeSet::new();
    for (index, value) in candidates.iter().enumerate() {
        let candidate = parse_candidate(
            value,
            index,
            evidence_class,
            &scenario_sha,
            attempts_budget,
            time_budget,
        )
        .map_err(|error| vec![error])?;
        if !ids.insert(candidate.id.clone()) {
            return Err(vec!["CANDIDATE_ID_DUPLICATE".to_owned()]);
        }
        parsed.push(candidate);
    }

    let eligible: Vec<&Candidate> = parsed.iter().filter(|candidate| candidate.gate_findings.is_empty()).collect();
    let mut pareto_front = Vec::new();
    let mut candidate_rows = Vec::new();
    let mut blocked_candidates = Vec::new();

    for candidate in &parsed {
        if !candidate.gate_findings.is_empty() {
            blocked_candidates.push(json!({
                "candidate_id": candidate.id,
                "embodiment": candidate.embodiment,
                "disposition": "BLOCKED_BY_HARD_GATE",
                "findings": candidate.gate_findings,
                "included_in_pareto_front": false
            }));
            candidate_rows.push(json!({
                "candidate_id": candidate.id,
                "embodiment": candidate.embodiment,
                "disposition": "BLOCKED_BY_HARD_GATE",
                "metrics": candidate.metrics.as_json(),
                "findings": candidate.gate_findings,
                "included_in_pareto_front": false
            }));
            continue;
        }
        let dominators: Vec<String> = eligible
            .iter()
            .filter(|other| other.id != candidate.id && other.metrics.dominates(&candidate.metrics))
            .map(|other| other.id.clone())
            .collect();
        let is_front = dominators.is_empty();
        if is_front {
            pareto_front.push(candidate.id.clone());
        }
        candidate_rows.push(json!({
            "candidate_id": candidate.id,
            "embodiment": candidate.embodiment,
            "disposition": if is_front { "PARETO_NON_DOMINATED" } else { "DOMINATED" },
            "metrics": candidate.metrics.as_json(),
            "dominated_by": dominators,
            "included_in_pareto_front": is_front
        }));
    }
    pareto_front.sort();
    candidate_rows.sort_by(|left, right| {
        value_string(left, "candidate_id").cmp(&value_string(right, "candidate_id"))
    });
    blocked_candidates.sort_by(|left, right| {
        value_string(left, "candidate_id").cmp(&value_string(right, "candidate_id"))
    });
    Ok(ScoreReport {
        study_id,
        evidence_class: evidence_class.to_owned(),
        scenario_sha256: scenario_sha,
        candidate_count: parsed.len(),
        eligible_count: eligible.len(),
        pareto_front,
        candidate_rows,
        blocked_candidates,
    })
}

fn load_file(path: &str) -> Result<Vec<u8>, String> {
    let bytes = fs::read(path).map_err(|error| format!("failed to read '{path}': {error}"))?;
    if bytes.len() > 4 * 1024 * 1024 {
        return Err(format!("'{path}' exceeds the 4 MiB input limit"));
    }
    Ok(bytes)
}

fn main() {
    let mut args = env::args().skip(1);
    let study_path = args.next().unwrap_or_else(|| DEFAULT_STUDY.to_owned());
    let Some(scenario_path) = args.next() else {
        eprintln!("usage: careframe-design-space STUDY_JSON SCENARIO_SET_JSON");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: careframe-design-space STUDY_JSON SCENARIO_SET_JSON");
        process::exit(2);
    }
    let study_bytes = match load_file(&study_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    let scenario_bytes = match load_file(&scenario_path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };
    if study_path == DEFAULT_STUDY && sha256_hex(&study_bytes) != sha256_hex(STUDY_SOURCE) {
        eprintln!("runtime study fixture differs from the bytes embedded in the build");
        process::exit(2);
    }
    if scenario_path == DEFAULT_SCENARIOS
        && sha256_hex(&scenario_bytes) != sha256_hex(SCENARIOS_SOURCE)
    {
        eprintln!("runtime scenario fixture differs from the bytes embedded in the build");
        process::exit(2);
    }
    let report = match score_study(&study_bytes, &scenario_bytes) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors {
                eprintln!("REJECT: {error}");
            }
            process::exit(2);
        }
    };
    let no_eligible = report.eligible_count == 0;
    let receipt = json!({
        "receipt_schema": "careframe-design-space-receipt-v0",
        "tested_revision": env::var("CAREFRAME_TESTED_SHA")
            .unwrap_or_else(|_| "UNBOUND_LOCAL_RUN".to_owned()),
        "study_id": report.study_id,
        "evidence_class": report.evidence_class,
        "study_sha256": sha256_hex(&study_bytes),
        "scenario_set_sha256": report.scenario_sha256,
        "scenario_file_sha256": sha256_hex(&scenario_bytes),
        "auditor_source_sha256": sha256_hex(AUDITOR_SOURCE),
        "candidate_count": report.candidate_count,
        "eligible_count": report.eligible_count,
        "blocked_candidates": report.blocked_candidates,
        "candidate_results": report.candidate_rows,
        "pareto_front": report.pareto_front,
        "aggregate_score_used": false,
        "physical_actuation_authorized": false,
        "patient_contact_authorized": false,
        "clinical_use_authorized": false,
        "claim_ceiling": CLAIM_CEILING
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).expect("receipt serialization cannot fail")
    );
    if no_eligible {
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn study_bytes() -> &'static [u8] {
        include_bytes!("../../../docs/medical-robotics/careframe-design-space-study-fixture-v0.1.json")
    }

    fn scenario_bytes() -> &'static [u8] {
        include_bytes!("../../../docs/medical-robotics/careframe-design-space-scenarios-v0.1.json")
    }

    fn parsed_study() -> Value {
        serde_json::from_slice(study_bytes()).expect("study fixture parses")
    }

    #[test]
    fn frozen_fixture_produces_a_non_compensatory_pareto_front() {
        let report = score_study(study_bytes(), scenario_bytes()).expect("frozen study is valid");
        assert_eq!(report.candidate_count, 4);
        assert_eq!(report.eligible_count, 3);
        assert_eq!(
            report.pareto_front,
            vec![
                "wheeled_mobile_manipulator".to_owned(),
                "wheeled_upper_body_humanoid".to_owned()
            ]
        );
        let blocked = report
            .blocked_candidates
            .iter()
            .find(|candidate| value_string(candidate, "candidate_id") == Some("biped_humanoid"))
            .expect("the biped candidate fails a hard gate");
        assert_eq!(value_string(blocked, "disposition"), Some("BLOCKED_BY_HARD_GATE"));
        assert!(report.candidate_rows.iter().any(|candidate| {
            value_string(candidate, "candidate_id") == Some("fixed_base_manipulator")
                && value_string(candidate, "disposition") == Some("DOMINATED")
        }));
    }

    #[test]
    fn scenario_digest_mismatch_rejects_the_entire_study() {
        let mut study = parsed_study();
        study["scenario_set_sha256"] = json!("0000000000000000000000000000000000000000000000000000000000000000");
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        let errors = score_study(&bytes, scenario_bytes()).expect_err("mismatch must reject");
        assert!(errors.iter().any(|error| error == "SCENARIO_SET_DIGEST_MISMATCH"));
    }

    #[test]
    fn unknown_fields_reject_at_root_candidate_and_gate_levels() {
        let mut study = parsed_study();
        study["unreviewed_weighted_score"] = json!(999);
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        assert!(score_study(&bytes, scenario_bytes()).is_err());

        let mut study = parsed_study();
        study["candidates"][0]["freeform_claim"] = json!("safe");
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        assert!(score_study(&bytes, scenario_bytes()).is_err());

        let mut study = parsed_study();
        study["candidates"][0]["hard_gates"]["allow_clinical_autonomy"] = json!(true);
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        assert!(score_study(&bytes, scenario_bytes()).is_err());
    }

    #[test]
    fn budget_mismatch_and_zero_denominators_reject() {
        let mut study = parsed_study();
        study["candidates"][0]["attempted_tasks"] = json!(99);
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        assert!(score_study(&bytes, scenario_bytes()).is_err());

        let mut study = parsed_study();
        study["candidates"][0]["successful_tasks"] = json!(101);
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        assert!(score_study(&bytes, scenario_bytes()).is_err());

        let mut study = parsed_study();
        study["budget"]["task_attempts_per_candidate"] = json!(0);
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        assert!(score_study(&bytes, scenario_bytes()).is_err());
    }

    #[test]
    fn failed_hard_gate_excludes_an_otherwise_dominant_candidate() {
        let mut study = parsed_study();
        study["candidates"][0]["successful_tasks"] = json!(100);
        study["candidates"][0]["intervention_count"] = json!(0);
        study["candidates"][0]["p95_recovery_ms"] = json!(0);
        study["candidates"][0]["energy_wh_per_attempt"] = json!(1);
        study["candidates"][0]["maintenance_hours_per_100_tasks"] = json!(0);
        study["candidates"][0]["hard_gates"]["safe_stop_fault_injection_passed"] = json!(false);
        let bytes = serde_json::to_vec(&study).expect("mutated study serializes");
        let report = score_study(&bytes, scenario_bytes()).expect("candidate failure is reportable");
        assert!(!report.pareto_front.contains(&"wheeled_upper_body_humanoid".to_owned()));
        assert!(report.blocked_candidates.iter().any(|candidate| {
            value_string(candidate, "candidate_id") == Some("wheeled_upper_body_humanoid")
        }));
    }

    #[test]
    fn scenario_must_have_unique_ids_and_known_kinds() {
        let suite: Value = serde_json::from_slice(scenario_bytes()).expect("scenario parses");
        let mut duplicate = suite.clone();
        duplicate["scenarios"][1]["scenario_id"] = duplicate["scenarios"][0]["scenario_id"].clone();
        let bytes = serde_json::to_vec(&duplicate).expect("scenario serializes");
        assert!(validate_scenario_set(&bytes, &sha256_hex(&bytes)).is_err());

        let mut unknown_kind = suite;
        unknown_kind["scenarios"][0]["kind"] = json!("physical_trial");
        let bytes = serde_json::to_vec(&unknown_kind).expect("scenario serializes");
        assert!(validate_scenario_set(&bytes, &sha256_hex(&bytes)).is_err());
    }

    #[test]
    fn receipts_bind_exact_study_scenario_and_source_bytes() {
        assert_eq!(sha256_hex(study_bytes()).len(), 64);
        assert_eq!(sha256_hex(scenario_bytes()).len(), 64);
        assert_eq!(sha256_hex(AUDITOR_SOURCE).len(), 64);
        assert!(valid_sha256(&sha256_hex(study_bytes())));
        assert!(!valid_sha256("ABCDEF"));
    }
}
