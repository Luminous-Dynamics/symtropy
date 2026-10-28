use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-selection-aware-qualification-synthetic-v0-1";
const CLAIM_CEILING: &str = "Synthetic Bonferroni-simultaneous binomial qualification mechanics only; no real-material, plant-yield, independence, or industrial-performance claim is validated.";
const TARGET_SUCCESS_PROBABILITY: f64 = 0.95;
const FAMILYWISE_CONFIDENCE_LEVEL: f64 = 0.95;
const MAX_CANDIDATES: usize = 100;
const MAX_TRIALS_PER_CANDIDATE: usize = 1000;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn close(left: f64, right: f64) -> bool {
    left.is_finite() && right.is_finite() && (left - right).abs() <= 1e-10
}

fn fail(report: &mut Value, code: &str, detail: &str) {
    report["status"] = json!("FAIL");
    if let Some(findings) = report.get_mut("findings").and_then(Value::as_array_mut) {
        findings.push(json!({"code": code, "detail": detail}));
    }
}

fn only_keys(value: &Value, allowed: &[&str], path: &str, report: &mut Value) {
    let Some(object) = value.as_object() else {
        fail(report, "OBJECT_REQUIRED", &format!("{path} must be a JSON object"));
        return;
    };
    for key in object.keys() {
        if !allowed.contains(&key.as_str()) {
            fail(report, "UNKNOWN_PROPERTY", &format!("{path}.{key} is not permitted"));
        }
    }
}

fn required_probability(value: &Value, field: &str, path: &str, report: &mut Value) -> Option<f64> {
    match value.get(field).and_then(Value::as_f64) {
        Some(number) if number.is_finite() && number > 0.0 && number < 1.0 => Some(number),
        _ => {
            fail(report, "PROBABILITY_INVALID", &format!("{path}.{field} must be strictly between zero and one"));
            None
        }
    }
}

fn required_str(value: &Value, field: &str, path: &str, report: &mut Value) -> Option<String> {
    match value.get(field).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty()) {
        Some(s) => Some(s.to_owned()),
        None => {
            fail(report, "STRING_REQUIRED", &format!("{path}.{field} must be a non-empty string"));
            None
        }
    }
}

fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
}

fn log_binomial_pmf(n: usize, k: usize, p: f64) -> f64 {
    if k > n || !(0.0..=1.0).contains(&p) {
        return f64::NEG_INFINITY;
    }
    if p == 0.0 {
        return if k == 0 { 0.0 } else { f64::NEG_INFINITY };
    }
    if p == 1.0 {
        return if k == n { 0.0 } else { f64::NEG_INFINITY };
    }
    let mut log_choose = 0.0;
    for i in 0..k {
        log_choose += ((n - i) as f64).ln() - ((i + 1) as f64).ln();
    }
    log_choose + (k as f64) * p.ln() + ((n - k) as f64) * (-p).ln_1p()
}

// P[X >= k], X ~ Binomial(n, p). n is contract-bounded at 1,000.
fn binomial_upper_tail(n: usize, k: usize, p: f64) -> f64 {
    if k == 0 {
        return 1.0;
    }
    if k > n || p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return 1.0;
    }
    let mut term = log_binomial_pmf(n, k, p).exp();
    let mut sum = term;
    for j in k..n {
        term *= ((n - j) as f64 / (j + 1) as f64) * (p / (1.0 - p));
        sum += term;
    }
    sum.clamp(0.0, 1.0)
}

// Exact one-sided Clopper-Pearson lower endpoint for the supplied tail alpha.
fn clopper_pearson_lower(n: usize, successes: usize, tail_alpha: f64) -> f64 {
    if successes == 0 || n == 0 {
        return 0.0;
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..80 {
        let mid = low + (high - low) / 2.0;
        if binomial_upper_tail(n, successes, mid) >= tail_alpha {
            high = mid;
        } else {
            low = mid;
        }
    }
    low + (high - low) / 2.0
}

fn simultaneous_interval(n: usize, successes: usize, per_tail_alpha: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let lower = clopper_pearson_lower(n, successes, per_tail_alpha);
    let failures = n - successes;
    let upper = 1.0 - clopper_pearson_lower(n, failures, per_tail_alpha);
    (lower.clamp(0.0, 1.0), upper.clamp(0.0, 1.0))
}

fn make_report(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let mut report = json!({
        "schema_version": 1,
        "status": "PASS",
        "audit_kind": "synthetic_selection_aware_binomial_qualification",
        "benchmark_id": BENCHMARK_ID,
        "input_sha256": sha256(public_raw),
        "oracle_sha256": sha256(oracle_raw),
        "target_success_probability": TARGET_SUCCESS_PROBABILITY,
        "familywise_confidence_level": FAMILYWISE_CONFIDENCE_LEVEL,
        "adjustment": "bonferroni_simultaneous_two_sided_exact_binomial",
        "family_size": 0,
        "per_candidate_tail_alpha": 0.0,
        "decisions": [],
        "findings": [],
        "claim_ceiling": CLAIM_CEILING
    });

    let public: Value = match serde_json::from_slice(public_raw) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut report, "PUBLIC_JSON_INVALID", &error.to_string());
            return report;
        }
    };
    let oracle: Value = match serde_json::from_slice(oracle_raw) {
        Ok(value) => value,
        Err(error) => {
            fail(&mut report, "ORACLE_JSON_INVALID", &error.to_string());
            return report;
        }
    };

    only_keys(
        &public,
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "target_success_probability", "familywise_confidence_level", "candidates"],
        "public",
        &mut report,
    );
    only_keys(
        &oracle,
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "outcomes"],
        "oracle",
        &mut report,
    );

    if public.get("schema_version").and_then(Value::as_u64) != Some(1)
        || oracle.get("schema_version").and_then(Value::as_u64) != Some(1)
        || public.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || oracle.get("benchmark_id").and_then(Value::as_str) != Some(BENCHMARK_ID)
        || public.get("evidence_class").and_then(Value::as_str) != Some("synthetic")
        || oracle.get("evidence_class").and_then(Value::as_str) != Some("synthetic_evaluator_only")
    {
        fail(&mut report, "CONTRACT_IDENTITY_INVALID", "schema version, benchmark ID, or evidence class is invalid");
    }
    if public.get("claim_ceiling").and_then(Value::as_str) != Some(CLAIM_CEILING)
        || oracle.get("claim_ceiling").and_then(Value::as_str) != Some(CLAIM_CEILING)
    {
        fail(&mut report, "CLAIM_CEILING_INVALID", "the v0.1 synthetic claim ceiling is frozen");
    }

    let target = required_probability(&public, "target_success_probability", "public", &mut report);
    let family_confidence = required_probability(&public, "familywise_confidence_level", "public", &mut report);
    if target.is_some_and(|x| !close(x, TARGET_SUCCESS_PROBABILITY))
        || family_confidence.is_some_and(|x| !close(x, FAMILYWISE_CONFIDENCE_LEVEL))
    {
        fail(&mut report, "STATISTICAL_CONTRACT_CHANGED", "v0.1 freezes the target success probability and family-wise confidence at 0.95");
    }

    let Some(candidates) = public.get("candidates").and_then(Value::as_array) else {
        fail(&mut report, "CANDIDATES_REQUIRED", "public.candidates must be an array");
        return report;
    };
    if candidates.is_empty() || candidates.len() > MAX_CANDIDATES {
        fail(&mut report, "CANDIDATE_COUNT_INVALID", "candidate family size must be between one and 100");
        return report;
    }
    let mut candidate_ids = Vec::<String>::with_capacity(candidates.len());
    let mut seen_ids = HashSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let path = format!("public.candidates[{index}]");
        only_keys(candidate, &["candidate_id"], &path, &mut report);
        let Some(id) = required_str(candidate, "candidate_id", &path, &mut report) else { continue };
        if !valid_id(&id) {
            fail(&mut report, "CANDIDATE_ID_INVALID", &format!("{path}.candidate_id must match ^[a-z][a-z0-9-]*$"));
        }
        if !seen_ids.insert(id.clone()) {
            fail(&mut report, "CANDIDATE_ID_DUPLICATE", &format!("duplicate candidate ID {id}"));
        }
        candidate_ids.push(id);
    }

    let Some(outcomes) = oracle.get("outcomes").and_then(Value::as_array) else {
        fail(&mut report, "OUTCOMES_REQUIRED", "oracle.outcomes must be an array");
        return report;
    };
    let mut outcomes_by_id = BTreeMap::<String, Vec<bool>>::new();
    for (index, outcome) in outcomes.iter().enumerate() {
        let path = format!("oracle.outcomes[{index}]");
        only_keys(outcome, &["candidate_id", "trial_results"], &path, &mut report);
        let Some(id) = required_str(outcome, "candidate_id", &path, &mut report) else { continue };
        let Some(trials) = outcome.get("trial_results").and_then(Value::as_array) else {
            fail(&mut report, "TRIALS_REQUIRED", &format!("{path}.trial_results must be an array"));
            continue;
        };
        if trials.len() > MAX_TRIALS_PER_CANDIDATE {
            fail(&mut report, "TRIAL_COUNT_INVALID", &format!("{path} exceeds {MAX_TRIALS_PER_CANDIDATE} trials"));
            continue;
        }
        if !seen_ids.contains(&id) {
            fail(&mut report, "OUTCOME_CANDIDATE_UNREGISTERED", &format!("oracle outcome {id} is not in the public candidate family"));
        }
        let mut result_values = Vec::with_capacity(trials.len());
        let mut valid = true;
        for (trial_index, trial) in trials.iter().enumerate() {
            match trial.as_bool() {
                Some(value) => result_values.push(value),
                None => {
                    valid = false;
                    fail(&mut report, "TRIAL_RESULT_INVALID", &format!("{path}.trial_results[{trial_index}] must be boolean"));
                }
            }
        }
        if outcomes_by_id.contains_key(&id) {
            fail(&mut report, "OUTCOME_CANDIDATE_DUPLICATE", &format!("duplicate outcome record for {id}"));
        } else if valid {
            outcomes_by_id.insert(id, result_values);
        }
    }
    if outcomes.len() != candidate_ids.len() {
        fail(&mut report, "OUTCOME_COVERAGE_INVALID", "there must be exactly one outcome record per registered candidate");
    }
    for id in &candidate_ids {
        if !outcomes_by_id.contains_key(id) {
            fail(&mut report, "OUTCOME_MISSING", &format!("missing outcomes for registered candidate {id}"));
        }
    }
    if outcomes_by_id.keys().any(|id| !seen_ids.contains(id)) {
        fail(&mut report, "OUTCOME_COVERAGE_INVALID", "oracle outcome set differs from the preregistered public candidate family");
    }
    if report["status"] == "FAIL" {
        return report;
    }

    let family_size = candidate_ids.len();
    // Bonferroni allocates the total two-sided alpha across every candidate and both tails.
    // This gives at least 95% simultaneous coverage without assuming independence between candidates.
    let per_tail_alpha = (1.0 - FAMILYWISE_CONFIDENCE_LEVEL) / (2.0 * family_size as f64);
    let mut decisions = Vec::with_capacity(family_size);
    for id in &candidate_ids {
        let trials = &outcomes_by_id[id];
        let n = trials.len();
        let successes = trials.iter().filter(|&&passed| passed).count();
        let rate = if n == 0 { Value::Null } else { json!(successes as f64 / n as f64) };
        let (lower, upper) = simultaneous_interval(n, successes, per_tail_alpha);
        let disposition = if lower >= TARGET_SUCCESS_PROBABILITY {
            "QUALIFIED"
        } else if upper < TARGET_SUCCESS_PROBABILITY {
            "NOT_QUALIFIED"
        } else {
            "UNRESOLVED"
        };
        let interpretation = match disposition {
            "QUALIFIED" => "The simultaneous family-wise confidence interval clears the required success probability.",
            "NOT_QUALIFIED" => "The simultaneous family-wise confidence interval lies below the required success probability.",
            _ if n == 0 => "No Bernoulli trials were supplied; the candidate remains unresolved.",
            _ => "The simultaneous confidence interval overlaps the target; gather more representative evidence.",
        };
        decisions.push(json!({
            "candidate_id": id,
            "trial_count": n,
            "successes": successes,
            "empirical_success_rate": rate,
            "simultaneous_confidence_interval": {"lower": lower, "upper": upper},
            "disposition": disposition,
            "interpretation": interpretation
        }));
    }
    report["family_size"] = json!(family_size);
    report["per_candidate_tail_alpha"] = json!(per_tail_alpha);
    report["decisions"] = json!(decisions);
    report
}

fn replay(report_raw: &[u8], public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let expected = make_report(public_raw, oracle_raw);
    let supplied: Value = match serde_json::from_slice(report_raw) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "status": "FAIL",
                "audit_kind": "synthetic_selection_aware_qualification_replay",
                "report_sha256": sha256(report_raw),
                "input_sha256": sha256(public_raw),
                "oracle_sha256": sha256(oracle_raw),
                "findings": [{"code":"REPORT_JSON_INVALID","detail":error.to_string()}]
            });
        }
    };
    let matches = supplied == expected && expected["status"] == "PASS";
    json!({
        "status": if matches {"PASS"} else {"FAIL"},
        "audit_kind": "synthetic_selection_aware_qualification_replay",
        "benchmark_id": BENCHMARK_ID,
        "report_sha256": sha256(report_raw),
        "input_sha256": sha256(public_raw),
        "oracle_sha256": sha256(oracle_raw),
        "findings": if matches {json!([])} else {json!([{"code":"REPORT_REPLAY_MISMATCH","detail":"the supplied report differs from the deterministic result for the exact task and oracle bytes"}])}
    })
}

fn read_file(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() == 5 && args[1] == "--replay" {
        let report_raw = match read_file(&args[2]) { Ok(value) => value, Err(error) => { eprintln!("{error}"); return ExitCode::from(2); } };
        let public_raw = match read_file(&args[3]) { Ok(value) => value, Err(error) => { eprintln!("{error}"); return ExitCode::from(2); } };
        let oracle_raw = match read_file(&args[4]) { Ok(value) => value, Err(error) => { eprintln!("{error}"); return ExitCode::from(2); } };
        let result = replay(&report_raw, &public_raw, &oracle_raw);
        let code = if result["status"] == "PASS" { 0 } else { 2 };
        println!("{}", serde_json::to_string_pretty(&result).expect("replay serializes"));
        return ExitCode::from(code);
    }
    if args.len() != 3 {
        eprintln!("Usage:\n  audit-qualification-familywise <public-task.json> <sealed-outcomes.json>\n  audit-qualification-familywise --replay <report.json> <public-task.json> <sealed-outcomes.json>");
        return ExitCode::from(64);
    }
    let public_raw = match read_file(&args[1]) { Ok(value) => value, Err(error) => { eprintln!("{error}"); return ExitCode::from(2); } };
    let oracle_raw = match read_file(&args[2]) { Ok(value) => value, Err(error) => { eprintln!("{error}"); return ExitCode::from(2); } };
    let result = make_report(&public_raw, &oracle_raw);
    let code = if result["status"] == "PASS" { 0 } else { 2 };
    println!("{}", serde_json::to_string_pretty(&result).expect("report serializes"));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC: &[u8] = include_bytes!("../../fixtures/glass-selection-aware-qualification-task.synthetic.json");
    const ORACLE: &[u8] = include_bytes!("../../fixtures/glass-selection-aware-qualification-oracle.synthetic.json");

    fn report() -> Value {
        make_report(PUBLIC, ORACLE)
    }

    #[test]
    fn familywise_all_success_boundary_is_stricter_than_per_candidate_boundary() {
        let family_size = 5.0;
        let per_tail_alpha = (1.0 - FAMILYWISE_CONFIDENCE_LEVEL) / (2.0 * family_size);
        let lower_104 = simultaneous_interval(104, 104, per_tail_alpha).0;
        let lower_103 = simultaneous_interval(103, 103, per_tail_alpha).0;
        assert!(close(lower_104, per_tail_alpha.powf(1.0 / 104.0)));
        assert!(lower_104 > TARGET_SUCCESS_PROBABILITY);
        assert!(lower_103 < TARGET_SUCCESS_PROBABILITY);
    }

    #[test]
    fn five_candidates_require_more_evidence_than_one_candidate() {
        let nominal_lower_72 = clopper_pearson_lower(72, 72, (1.0 - FAMILYWISE_CONFIDENCE_LEVEL) / 2.0);
        let adjusted_lower_72 = simultaneous_interval(72, 72, (1.0 - FAMILYWISE_CONFIDENCE_LEVEL) / (2.0 * 5.0)).0;
        assert!(nominal_lower_72 > TARGET_SUCCESS_PROBABILITY);
        assert!(adjusted_lower_72 < TARGET_SUCCESS_PROBABILITY);
    }

    #[test]
    fn fixture_demonstrates_familywise_qualified_unresolved_and_rejected_cases() {
        let value = report();
        assert_eq!(value["status"], "PASS");
        assert_eq!(value["family_size"], 5);
        assert!(close(value["per_candidate_tail_alpha"].as_f64().unwrap(), 0.005));
        let decisions = value["decisions"].as_array().unwrap();
        let get = |id: &str| decisions.iter().find(|entry| entry["candidate_id"] == id).unwrap();
        assert_eq!(get("lot-alpha")["disposition"], "QUALIFIED");
        assert_eq!(get("lot-beta")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-gamma")["disposition"], "NOT_QUALIFIED");
        assert_eq!(get("lot-delta")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-epsilon")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-epsilon")["simultaneous_confidence_interval"], json!({"lower":0.0,"upper":1.0}));
    }

    #[test]
    fn familywise_false_qualification_probability_is_bounded_at_the_target() {
        let m = 5.0;
        let n = 120usize;
        let alpha_tail = (1.0 - FAMILYWISE_CONFIDENCE_LEVEL) / (2.0 * m);
        let mut low = 0usize;
        let mut high = n + 1;
        while low < high {
            let mid = low + (high - low) / 2;
            if mid <= n && simultaneous_interval(n, mid, alpha_tail).0 >= TARGET_SUCCESS_PROBABILITY {
                high = mid;
            } else {
                low = mid + 1;
            }
        }
        let min_qualifying_successes = low;
        let per_candidate_false_qualification = if min_qualifying_successes > n {
            0.0
        } else {
            binomial_upper_tail(n, min_qualifying_successes, TARGET_SUCCESS_PROBABILITY)
        };
        assert!(m * per_candidate_false_qualification <= 0.05 + 1e-9);
    }

    #[test]
    fn single_candidate_family_matches_exact_interval_known_answer() {
        let per_tail_alpha = (1.0 - FAMILYWISE_CONFIDENCE_LEVEL) / 2.0;
        let (lower, upper) = simultaneous_interval(20, 19, per_tail_alpha);
        assert!(close(lower, 0.7512672372279723));
        assert!(close(upper, 0.9987349105020502));
    }

    #[test]
    fn rejects_unknown_fields_bad_ids_missing_outcomes_and_forged_replay() {
        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        let oracle: Value = serde_json::from_slice(ORACLE).unwrap();
        public["unregistered_field"] = json!(true);
        let invalid = make_report(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(invalid["status"], "FAIL");
        assert!(invalid["findings"].as_array().unwrap().iter().any(|item| item["code"] == "UNKNOWN_PROPERTY"));

        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        public["candidates"][0]["candidate_id"] = json!("Bad Candidate");
        let invalid = make_report(&serde_json::to_vec(&public).unwrap(), ORACLE);
        assert_eq!(invalid["status"], "FAIL");
        assert!(invalid["findings"].as_array().unwrap().iter().any(|item| item["code"] == "CANDIDATE_ID_INVALID"));

        let mut oracle: Value = serde_json::from_slice(ORACLE).unwrap();
        oracle["outcomes"].pop();
        let invalid = make_report(PUBLIC, &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(invalid["status"], "FAIL");
        assert!(invalid["findings"].as_array().unwrap().iter().any(|item| item["code"] == "OUTCOME_COVERAGE_INVALID"));

        let mut forged = report();
        forged["decisions"][1]["disposition"] = json!("QUALIFIED");
        let raw = serde_json::to_vec(&forged).unwrap();
        assert_eq!(replay(&raw, PUBLIC, ORACLE)["status"], "FAIL");
    }
}
