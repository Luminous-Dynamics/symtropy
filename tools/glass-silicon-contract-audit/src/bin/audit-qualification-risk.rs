use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::HashSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-qualification-risk-synthetic-v0-1";
const CLAIM_CEILING: &str = "Synthetic Bernoulli qualification-gate mechanics only; no real-material, plant-yield, independence, or industrial-performance claim is validated.";
const TARGET_SUCCESS_PROBABILITY: f64 = 0.95;
const CONFIDENCE_LEVEL: f64 = 0.95;
const MAX_CANDIDATES: usize = 100;
const MAX_TRIALS: usize = 1000;

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

fn required_nonempty_str(value: &Value, field: &str, path: &str, report: &mut Value) -> Option<String> {
    match value.get(field).and_then(Value::as_str).map(str::trim).filter(|text| !text.is_empty()) {
        Some(text) => Some(text.to_owned()),
        None => {
            fail(report, "STRING_REQUIRED", &format!("{path}.{field} must be a non-empty string"));
            None
        }
    }
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

fn binomial_pmf(n: usize, k: usize, p: f64) -> f64 {
    log_binomial_pmf(n, k, p).exp()
}

// P[X >= k] for X ~ Binomial(n, p). Trial counts are contract-bounded at 1,000.
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

// Exact one-sided Clopper-Pearson lower limit: solve P_p[X >= successes] = alpha.
fn clopper_pearson_lower(n: usize, successes: usize, alpha: f64) -> f64 {
    if successes == 0 || n == 0 {
        return 0.0;
    }
    let mut low = 0.0;
    let mut high = 1.0;
    for _ in 0..80 {
        let mid = low + (high - low) / 2.0;
        if binomial_upper_tail(n, successes, mid) >= alpha {
            high = mid;
        } else {
            low = mid;
        }
    }
    low + (high - low) / 2.0
}

// Two-sided exact Clopper-Pearson interval. The two tails each receive alpha / 2.
fn clopper_pearson_interval(n: usize, successes: usize, confidence: f64) -> (f64, f64) {
    if n == 0 {
        return (0.0, 1.0);
    }
    let alpha_each_tail = (1.0 - confidence) / 2.0;
    let lower = clopper_pearson_lower(n, successes, alpha_each_tail);
    let failures = n - successes;
    let upper = 1.0 - clopper_pearson_lower(n, failures, alpha_each_tail);
    (lower.clamp(0.0, 1.0), upper.clamp(0.0, 1.0))
}

fn validate_and_decide(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let mut report = json!({
        "schema_version": 1,
        "status": "PASS",
        "audit_kind": "synthetic_binomial_qualification_risk_gate",
        "benchmark_id": BENCHMARK_ID,
        "input_sha256": sha256(public_raw),
        "oracle_sha256": sha256(oracle_raw),
        "target_success_probability": TARGET_SUCCESS_PROBABILITY,
        "confidence_level": CONFIDENCE_LEVEL,
        "candidate_count": 0,
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
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "target_success_probability", "confidence_level", "candidates"],
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
    let confidence = required_probability(&public, "confidence_level", "public", &mut report);
    if target.is_some_and(|value| !close(value, TARGET_SUCCESS_PROBABILITY))
        || confidence.is_some_and(|value| !close(value, CONFIDENCE_LEVEL))
    {
        fail(&mut report, "STATISTICAL_CONTRACT_CHANGED", "v0.1 freezes both target success probability and confidence level at 0.95");
    }

    let Some(candidates) = public.get("candidates").and_then(Value::as_array) else {
        fail(&mut report, "CANDIDATES_REQUIRED", "public.candidates must be an array");
        return report;
    };
    if candidates.is_empty() || candidates.len() > MAX_CANDIDATES {
        fail(&mut report, "CANDIDATE_COUNT_INVALID", "candidate count must be between one and 100");
    }
    let mut registered = Vec::new();
    let mut candidate_ids = HashSet::new();
    for (index, candidate) in candidates.iter().enumerate() {
        let path = format!("public.candidates[{index}]");
        only_keys(candidate, &["candidate_id"], &path, &mut report);
        if let Some(candidate_id) = required_nonempty_str(candidate, "candidate_id", &path, &mut report) {
            if !candidate_ids.insert(candidate_id.clone()) {
                fail(&mut report, "CANDIDATE_ID_DUPLICATE", &format!("duplicate candidate ID {candidate_id}"));
            }
            registered.push(candidate_id);
        }
    }

    let Some(outcomes) = oracle.get("outcomes").and_then(Value::as_array) else {
        fail(&mut report, "OUTCOMES_REQUIRED", "oracle.outcomes must be an array");
        return report;
    };
    let mut outcomes_by_id = std::collections::BTreeMap::<String, Vec<bool>>::new();
    for (index, outcome) in outcomes.iter().enumerate() {
        let path = format!("oracle.outcomes[{index}]");
        only_keys(outcome, &["candidate_id", "trial_results"], &path, &mut report);
        let Some(candidate_id) = required_nonempty_str(outcome, "candidate_id", &path, &mut report) else { continue };
        let Some(trials) = outcome.get("trial_results").and_then(Value::as_array) else {
            fail(&mut report, "TRIALS_REQUIRED", &format!("{path}.trial_results must be an array"));
            continue;
        };
        if trials.len() > MAX_TRIALS {
            fail(&mut report, "TRIAL_COUNT_INVALID", &format!("{path} exceeds the {MAX_TRIALS} trial cap"));
            continue;
        }
        let mut results = Vec::with_capacity(trials.len());
        let mut all_boolean = true;
        for (trial_index, trial) in trials.iter().enumerate() {
            match trial.as_bool() {
                Some(result) => results.push(result),
                None => {
                    all_boolean = false;
                    fail(&mut report, "TRIAL_RESULT_INVALID", &format!("{path}.trial_results[{trial_index}] must be boolean"));
                }
            }
        }
        if !candidate_ids.contains(&candidate_id) {
            fail(&mut report, "OUTCOME_CANDIDATE_UNREGISTERED", &format!("oracle outcome {candidate_id} is not in the public candidate list"));
        }
        if outcomes_by_id.contains_key(&candidate_id) {
            fail(&mut report, "OUTCOME_CANDIDATE_DUPLICATE", &format!("duplicate outcome record for {candidate_id}"));
        } else if all_boolean {
            outcomes_by_id.insert(candidate_id, results);
        }
    }
    if outcomes.len() != registered.len() {
        fail(&mut report, "OUTCOME_COVERAGE_INVALID", "oracle must contain exactly one outcome record per public candidate");
    }
    for candidate_id in &registered {
        if !outcomes_by_id.contains_key(candidate_id) {
            fail(&mut report, "OUTCOME_MISSING", &format!("oracle has no complete outcome record for {candidate_id}"));
        }
    }
    if outcomes_by_id.keys().any(|id| !candidate_ids.contains(id)) {
        fail(&mut report, "OUTCOME_COVERAGE_INVALID", "oracle outcome set differs from the public candidate set");
    }
    if report["status"] == "FAIL" {
        return report;
    }

    let mut decisions = Vec::new();
    for candidate_id in &registered {
        let trials = &outcomes_by_id[candidate_id];
        let sample_count = trials.len();
        let successes = trials.iter().filter(|&&passed| passed).count();
        let empirical_rate = if sample_count == 0 { Value::Null } else { json!(successes as f64 / sample_count as f64) };
        let (lower, upper) = clopper_pearson_interval(sample_count, successes, CONFIDENCE_LEVEL);
        let disposition = if lower >= TARGET_SUCCESS_PROBABILITY {
            "QUALIFIED"
        } else if upper < TARGET_SUCCESS_PROBABILITY {
            "NOT_QUALIFIED"
        } else {
            "UNRESOLVED"
        };
        let interpretation = match disposition {
            "QUALIFIED" => "The full two-sided exact confidence interval clears the required success probability.",
            "NOT_QUALIFIED" => "The full two-sided exact confidence interval is below the required success probability.",
            _ if sample_count == 0 => "No independent Bernoulli trials were supplied; the candidate remains unresolved.",
            _ => "The confidence interval overlaps the required success probability; gather more representative evidence.",
        };
        decisions.push(json!({
            "candidate_id": candidate_id,
            "trial_count": sample_count,
            "successes": successes,
            "empirical_success_rate": empirical_rate,
            "confidence_interval": {"lower": lower, "upper": upper},
            "disposition": disposition,
            "interpretation": interpretation
        }));
    }
    report["candidate_count"] = json!(decisions.len());
    report["decisions"] = json!(decisions);
    report
}

fn replay(report_raw: &[u8], public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let expected = validate_and_decide(public_raw, oracle_raw);
    let supplied: Value = match serde_json::from_slice(report_raw) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "status": "FAIL",
                "audit_kind": "synthetic_binomial_qualification_replay",
                "report_sha256": sha256(report_raw),
                "input_sha256": sha256(public_raw),
                "oracle_sha256": sha256(oracle_raw),
                "findings": [{"code": "REPORT_JSON_INVALID", "detail": error.to_string()}]
            });
        }
    };
    let matches = supplied == expected && expected["status"] == "PASS";
    json!({
        "status": if matches { "PASS" } else { "FAIL" },
        "audit_kind": "synthetic_binomial_qualification_replay",
        "benchmark_id": BENCHMARK_ID,
        "report_sha256": sha256(report_raw),
        "input_sha256": sha256(public_raw),
        "oracle_sha256": sha256(oracle_raw),
        "findings": if matches { json!([]) } else { json!([{"code":"REPORT_REPLAY_MISMATCH","detail":"the supplied report differs from the deterministic result for the exact input and oracle bytes"}]) }
    })
}

fn read_file(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() == 5 && args[1] == "--replay" {
        let report_raw = match read_file(&args[2]) {
            Ok(value) => value,
            Err(error) => { eprintln!("{error}"); return ExitCode::from(2); }
        };
        let public_raw = match read_file(&args[3]) {
            Ok(value) => value,
            Err(error) => { eprintln!("{error}"); return ExitCode::from(2); }
        };
        let oracle_raw = match read_file(&args[4]) {
            Ok(value) => value,
            Err(error) => { eprintln!("{error}"); return ExitCode::from(2); }
        };
        let report = replay(&report_raw, &public_raw, &oracle_raw);
        let code = if report["status"] == "PASS" { 0 } else { 2 };
        println!("{}", serde_json::to_string_pretty(&report).expect("replay report serializes"));
        return ExitCode::from(code);
    }
    if args.len() != 3 {
        eprintln!("Usage:\n  audit-qualification-risk <public-task.json> <sealed-outcomes.json>\n  audit-qualification-risk --replay <report.json> <public-task.json> <sealed-outcomes.json>");
        return ExitCode::from(64);
    }
    let public_raw = match read_file(&args[1]) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::from(2); }
    };
    let oracle_raw = match read_file(&args[2]) {
        Ok(value) => value,
        Err(error) => { eprintln!("{error}"); return ExitCode::from(2); }
    };
    let report = validate_and_decide(&public_raw, &oracle_raw);
    let code = if report["status"] == "PASS" { 0 } else { 2 };
    println!("{}", serde_json::to_string_pretty(&report).expect("audit report serializes"));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC: &[u8] = include_bytes!("../../fixtures/glass-qualification-risk-task.synthetic.json");
    const ORACLE: &[u8] = include_bytes!("../../fixtures/glass-qualification-risk-oracle.synthetic.json");

    fn fixture_report() -> Value {
        validate_and_decide(PUBLIC, ORACLE)
    }

    #[test]
    fn exact_interval_has_known_all_success_boundary() {
        let (lower_72, upper_72) = clopper_pearson_interval(72, 72, 0.95);
        let (lower_71, _) = clopper_pearson_interval(71, 71, 0.95);
        assert!(close(lower_72, 0.025_f64.powf(1.0 / 72.0)));
        assert!(lower_72 > 0.95);
        assert!(lower_71 < 0.95);
        assert_eq!(upper_72, 1.0);
    }

    #[test]
    fn exact_interval_handles_zero_and_all_success_extremes() {
        let (lower_zero, upper_zero) = clopper_pearson_interval(1, 0, 0.95);
        let (lower_one, upper_one) = clopper_pearson_interval(1, 1, 0.95);
        assert_eq!(lower_zero, 0.0);
        assert!(close(upper_zero, 0.975));
        assert!(close(lower_one, 0.025));
        assert_eq!(upper_one, 1.0);
        assert_eq!(clopper_pearson_interval(0, 0, 0.95), (0.0, 1.0));
    }

    #[test]
    fn ninety_five_percent_observed_success_is_not_ninety_five_percent_confidence() {
        let (lower, upper) = clopper_pearson_interval(100, 95, 0.95);
        assert!(lower < 0.95);
        assert!(upper > 0.95);
    }

    #[test]
    fn exact_binomial_boundary_control_limits_false_qualification() {
        let n = 120usize;
        let target = TARGET_SUCCESS_PROBABILITY;
        let mut false_qualification_probability = 0.0;
        for successes in 0..=n {
            let (lower, _) = clopper_pearson_interval(n, successes, CONFIDENCE_LEVEL);
            if lower >= target {
                false_qualification_probability += binomial_pmf(n, successes, target);
            }
        }
        assert!(false_qualification_probability <= 0.025 + 1e-9);
        let below_target_probability = 0.90;
        let mut below_target_false_qualification = 0.0;
        for successes in 0..=n {
            let (lower, _) = clopper_pearson_interval(n, successes, CONFIDENCE_LEVEL);
            if lower >= target {
                below_target_false_qualification += binomial_pmf(n, successes, below_target_probability);
            }
        }
        assert!(below_target_false_qualification <= 0.025 + 1e-9);
    }

    #[test]
    fn fixture_distinguishes_qualified_rejected_and_unresolved_without_sampling_free_passes() {
        let report = fixture_report();
        assert_eq!(report["status"], "PASS");
        assert_eq!(report["candidate_count"], 5);
        let decisions = report["decisions"].as_array().unwrap();
        let get = |id: &str| decisions.iter().find(|item| item["candidate_id"] == id).unwrap();
        assert_eq!(get("lot-alpha")["disposition"], "QUALIFIED");
        assert_eq!(get("lot-beta")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-gamma")["disposition"], "NOT_QUALIFIED");
        assert_eq!(get("lot-delta")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-epsilon")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-epsilon")["confidence_interval"]["lower"], 0.0);
        assert_eq!(get("lot-epsilon")["confidence_interval"]["upper"], 1.0);
    }

    #[test]
    fn replay_rejects_forged_dispositions_and_digests() {
        let mut forged = fixture_report();
        forged["decisions"][1]["disposition"] = json!("QUALIFIED");
        let raw = serde_json::to_vec(&forged).unwrap();
        assert_eq!(replay(&raw, PUBLIC, ORACLE)["status"], "FAIL");

        let mut forged = fixture_report();
        forged["input_sha256"] = json!("0".repeat(64));
        let raw = serde_json::to_vec(&forged).unwrap();
        assert_eq!(replay(&raw, PUBLIC, ORACLE)["status"], "FAIL");
    }
}
