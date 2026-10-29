use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-anytime-qualification-synthetic-v0-1";
const CLAIM_CEILING: &str = "Synthetic anytime-valid sequential qualification mechanics only; no real-material, plant-yield, independence, adaptive-ledger-integrity, or industrial-performance claim is validated.";
const TARGET_SUCCESS_PROBABILITY: f64 = 0.95;
const FAMILYWISE_ALPHA: f64 = 0.05;
const MAX_CANDIDATES: usize = 100;
const MAX_TRIALS_PER_CANDIDATE: usize = 1000;
const MAX_EVENTS: usize = 100_000;

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

fn valid_id(id: &str) -> bool {
    let bytes = id.as_bytes();
    !bytes.is_empty()
        && bytes[0].is_ascii_lowercase()
        && bytes.iter().all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || *byte == b'-')
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
    let terms = k.min(n - k);
    let mut log_choose = 0.0;
    for i in 0..terms {
        log_choose += ((n - i) as f64).ln() - ((i + 1) as f64).ln();
    }
    log_choose + (k as f64) * p.ln() + ((n - k) as f64) * (-p).ln_1p()
}

fn log_add_exp(left: f64, right: f64) -> f64 {
    if left == f64::NEG_INFINITY {
        return right;
    }
    if right == f64::NEG_INFINITY {
        return left;
    }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

// Log P[X >= successes] for X ~ Binomial(n, p), evaluated with log-sum-exp.
fn log_binomial_upper_tail(n: usize, successes: usize, p: f64) -> f64 {
    if successes == 0 {
        return 0.0;
    }
    if successes > n || p <= 0.0 {
        return f64::NEG_INFINITY;
    }
    if p >= 1.0 {
        return 0.0;
    }
    let mut log_term = log_binomial_pmf(n, successes, p);
    let mut log_sum = log_term;
    for k in successes..n {
        log_term += ((n - k) as f64).ln()
            - ((k + 1) as f64).ln()
            + p.ln()
            - (-p).ln_1p();
        log_sum = log_add_exp(log_sum, log_term);
    }
    log_sum.min(0.0)
}

// Log P[X <= successes] for X ~ Binomial(n, p), evaluated with log-sum-exp.
fn log_binomial_lower_tail(n: usize, successes: usize, p: f64) -> f64 {
    if successes >= n {
        return 0.0;
    }
    if p <= 0.0 {
        return 0.0;
    }
    if p >= 1.0 {
        return f64::NEG_INFINITY;
    }
    let mut log_term = log_binomial_pmf(n, successes, p);
    let mut log_sum = log_term;
    for k in (1..=successes).rev() {
        log_term += (k as f64).ln()
            - ((n - k + 1) as f64).ln()
            + (-p).ln_1p()
            - p.ln();
        log_sum = log_add_exp(log_sum, log_term);
    }
    log_sum.min(0.0)
}

// Countable alpha allocation: sum_i 1/[i(i+1)] = 1, and likewise for time t.
// Split each cell equally between false-qualification and false-rejection tails.
fn per_tail_alpha(registration_index: usize, look_index: usize) -> f64 {
    FAMILYWISE_ALPHA
        / (2.0
            * registration_index as f64
            * (registration_index + 1) as f64
            * look_index as f64
            * (look_index + 1) as f64)
}

#[derive(Clone, Debug)]
struct CandidateState {
    registration_index: usize,
    trials_seen: usize,
    successes: usize,
    terminal_decision: Option<String>,
    terminal_event_seq: Option<usize>,
    last_upper_log_p: Option<f64>,
    last_lower_log_p: Option<f64>,
}

fn empty_report(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    json!({
        "schema_version": 1,
        "status": "PASS",
        "audit_kind": "synthetic_anytime_valid_sequential_qualification",
        "benchmark_id": BENCHMARK_ID,
        "input_sha256": sha256(public_raw),
        "oracle_sha256": sha256(oracle_raw),
        "target_success_probability": TARGET_SUCCESS_PROBABILITY,
        "familywise_alpha": FAMILYWISE_ALPHA,
        "allocation": "alpha/[2*i*(i+1)*t*(t+1)] per candidate-time-tail",
        "events_checked": 0,
        "candidate_count": 0,
        "events": [],
        "decisions": [],
        "findings": [],
        "claim_ceiling": CLAIM_CEILING
    })
}

fn make_report(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let mut report = empty_report(public_raw, oracle_raw);
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
        &["schema_version", "benchmark_id", "evidence_class", "claim_ceiling", "target_success_probability", "familywise_alpha", "registration_rule", "events"],
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
    if !public.get("target_success_probability").and_then(Value::as_f64).is_some_and(|v| close(v, TARGET_SUCCESS_PROBABILITY))
        || !public.get("familywise_alpha").and_then(Value::as_f64).is_some_and(|v| close(v, FAMILYWISE_ALPHA))
        || public.get("registration_rule").and_then(Value::as_str) != Some("append_only_one_based_registration_order")
    {
        fail(&mut report, "STATISTICAL_CONTRACT_CHANGED", "target, global alpha, and append-only registration rule are frozen");
    }

    let Some(public_events) = public.get("events").and_then(Value::as_array) else {
        fail(&mut report, "EVENTS_REQUIRED", "public.events must be an array");
        return report;
    };
    if public_events.is_empty() || public_events.len() > MAX_EVENTS {
        fail(&mut report, "EVENT_COUNT_INVALID", &format!("public event count must be between 1 and {MAX_EVENTS}"));
        return report;
    }
    let Some(outcomes) = oracle.get("outcomes").and_then(Value::as_array) else {
        fail(&mut report, "OUTCOMES_REQUIRED", "oracle.outcomes must be an array");
        return report;
    };

    let mut oracle_by_id = BTreeMap::<String, Vec<bool>>::new();
    for (index, outcome) in outcomes.iter().enumerate() {
        let path = format!("oracle.outcomes[{index}]");
        only_keys(outcome, &["candidate_id", "trial_results"], &path, &mut report);
        let Some(id) = required_str(outcome, "candidate_id", &path, &mut report) else { continue };
        let Some(trials) = outcome.get("trial_results").and_then(Value::as_array) else {
            fail(&mut report, "TRIALS_REQUIRED", &format!("{path}.trial_results must be an array"));
            continue;
        };
        if !valid_id(&id) {
            fail(&mut report, "CANDIDATE_ID_INVALID", &format!("{path}.candidate_id must match ^[a-z][a-z0-9-]*$"));
        }
        if trials.len() > MAX_TRIALS_PER_CANDIDATE {
            fail(&mut report, "TRIAL_COUNT_INVALID", &format!("{path} exceeds {MAX_TRIALS_PER_CANDIDATE} trials"));
            continue;
        }
        let mut parsed = Vec::with_capacity(trials.len());
        let mut valid = true;
        for (trial_index, trial) in trials.iter().enumerate() {
            match trial.as_bool() {
                Some(value) => parsed.push(value),
                None => {
                    valid = false;
                    fail(&mut report, "TRIAL_RESULT_INVALID", &format!("{path}.trial_results[{trial_index}] must be boolean"));
                }
            }
        }
        if oracle_by_id.contains_key(&id) {
            fail(&mut report, "OUTCOME_CANDIDATE_DUPLICATE", &format!("duplicate oracle outcome record {id}"));
        } else if valid {
            oracle_by_id.insert(id, parsed);
        }
    }

    let mut states = BTreeMap::<String, CandidateState>::new();
    let mut registration_order = Vec::<String>::new();
    let mut oracle_cursors = BTreeMap::<String, usize>::new();
    let mut checked_events = Vec::<Value>::new();

    for (position, event) in public_events.iter().enumerate() {
        let path = format!("public.events[{position}]");
        only_keys(event, &["event_seq", "kind", "candidate_id"], &path, &mut report);
        let seq = position + 1;
        if event.get("event_seq").and_then(Value::as_u64) != Some(seq as u64) {
            fail(&mut report, "EVENT_SEQUENCE_INVALID", &format!("{path}.event_seq must equal {}", seq));
        }
        let Some(kind) = event.get("kind").and_then(Value::as_str) else {
            fail(&mut report, "EVENT_KIND_REQUIRED", &format!("{path}.kind must be a string"));
            continue;
        };
        let Some(id) = required_str(event, "candidate_id", &path, &mut report) else { continue };
        if !valid_id(&id) {
            fail(&mut report, "CANDIDATE_ID_INVALID", &format!("{path}.candidate_id must match ^[a-z][a-z0-9-]*$"));
        }

        match kind {
            "register_candidate" => {
                if states.contains_key(&id) {
                    fail(&mut report, "CANDIDATE_REGISTRATION_DUPLICATE", &format!("{path} registers {id} more than once"));
                    continue;
                }
                if registration_order.len() >= MAX_CANDIDATES {
                    fail(&mut report, "CANDIDATE_COUNT_INVALID", "candidate registry exceeds the frozen cap of 100");
                    continue;
                }
                let index = registration_order.len() + 1;
                registration_order.push(id.clone());
                states.insert(id.clone(), CandidateState {
                    registration_index: index,
                    trials_seen: 0,
                    successes: 0,
                    terminal_decision: None,
                    terminal_event_seq: None,
                    last_upper_log_p: None,
                    last_lower_log_p: None,
                });
                oracle_cursors.insert(id.clone(), 0);
                checked_events.push(json!({
                    "event_seq": seq,
                    "kind": "register_candidate",
                    "candidate_id": id,
                    "registration_index": index
                }));
            }
            "observe_trial" => {
                let Some(state_before) = states.get(&id).cloned() else {
                    fail(&mut report, "OBSERVATION_BEFORE_REGISTRATION", &format!("{path} observes {id} before registration"));
                    continue;
                };
                if let Some(decision) = state_before.terminal_decision.as_deref() {
                    fail(&mut report, "OBSERVATION_AFTER_TERMINAL_DECISION", &format!("{path} observes {id} after terminal decision {decision}"));
                    continue;
                }
                if state_before.trials_seen >= MAX_TRIALS_PER_CANDIDATE {
                    fail(&mut report, "TRIAL_COUNT_INVALID", &format!("{path} exceeds {MAX_TRIALS_PER_CANDIDATE} trials for {id}"));
                    continue;
                }
                let Some(outcome_trials) = oracle_by_id.get(&id) else {
                    fail(&mut report, "OUTCOME_CANDIDATE_MISSING", &format!("oracle has no outcomes for registered candidate {id}"));
                    continue;
                };
                let cursor = oracle_cursors.get(&id).copied().unwrap_or(0);
                let Some(passed) = outcome_trials.get(cursor).copied() else {
                    fail(&mut report, "SEALED_OUTCOME_MISSING", &format!("{path} requests missing sealed trial {cursor} for {id}"));
                    continue;
                };
                let state = states.get_mut(&id).expect("registered state exists");
                state.trials_seen += 1;
                if passed {
                    state.successes += 1;
                }
                oracle_cursors.insert(id.clone(), cursor + 1);
                let n = state.trials_seen;
                let x = state.successes;
                let registration_index = state.registration_index;
                let alpha = per_tail_alpha(registration_index, n);
                let log_upper = log_binomial_upper_tail(n, x, TARGET_SUCCESS_PROBABILITY);
                let log_lower = log_binomial_lower_tail(n, x, TARGET_SUCCESS_PROBABILITY);
                let log_alpha = alpha.ln();
                let decision = if log_upper <= log_alpha {
                    Some("QUALIFIED")
                } else if log_lower <= log_alpha {
                    Some("NOT_QUALIFIED")
                } else {
                    None
                };
                state.last_upper_log_p = Some(log_upper);
                state.last_lower_log_p = Some(log_lower);
                if let Some(decision) = decision {
                    state.terminal_decision = Some(decision.to_owned());
                    state.terminal_event_seq = Some(seq);
                }
                checked_events.push(json!({
                    "event_seq": seq,
                    "kind": "observe_trial",
                    "candidate_id": id,
                    "registration_index": registration_index,
                    "look_index": n,
                    "trial_passed": passed,
                    "successes_so_far": x,
                    "per_tail_alpha_budget": alpha,
                    "upper_tail_p_value": log_upper.exp(),
                    "log_upper_tail_p_value": log_upper,
                    "lower_tail_p_value": log_lower.exp(),
                    "log_lower_tail_p_value": log_lower,
                    "decision": decision.unwrap_or("UNRESOLVED")
                }));
            }
            other => {
                fail(&mut report, "EVENT_KIND_UNREGISTERED", &format!("{path} has unregistered event kind {other}"));
            }
        }
    }

    let registered_set = registration_order.iter().cloned().collect::<HashSet<_>>();
    if registration_order.is_empty() {
        fail(&mut report, "CANDIDATE_REGISTRY_EMPTY", "at least one candidate must be registered");
    }
    if oracle_by_id.keys().any(|id| !registered_set.contains(id)) {
        fail(&mut report, "ORACLE_HAS_UNREGISTERED_CANDIDATE", "oracle contains a candidate never registered in the public event stream");
    }
    for id in &registration_order {
        if !oracle_by_id.contains_key(id) {
            fail(&mut report, "OUTCOME_CANDIDATE_MISSING", &format!("oracle has no outcome record for registered candidate {id}"));
        }
    }
    if oracle_by_id.len() != registration_order.len() {
        fail(&mut report, "ORACLE_COVERAGE_INVALID", "oracle outcome set must equal the registered candidate set");
    }
    if report["status"] == "FAIL" {
        return report;
    }

    let mut decisions = Vec::with_capacity(registration_order.len());
    for id in &registration_order {
        let state = &states[id];
        let disposition = state.terminal_decision.as_deref().unwrap_or("UNRESOLVED");
        decisions.push(json!({
            "candidate_id": id,
            "registration_index": state.registration_index,
            "trials_observed": state.trials_seen,
            "successes_observed": state.successes,
            "empirical_success_rate": if state.trials_seen == 0 {Value::Null} else {json!(state.successes as f64 / state.trials_seen as f64)},
            "disposition": disposition,
            "terminal_event_seq": state.terminal_event_seq,
            "last_upper_tail_p_value": state.last_upper_log_p.map(f64::exp),
            "last_lower_tail_p_value": state.last_lower_log_p.map(f64::exp)
        }));
    }
    report["events_checked"] = json!(checked_events.len());
    report["candidate_count"] = json!(registration_order.len());
    report["events"] = json!(checked_events);
    report["decisions"] = json!(decisions);
    report
}

fn replay(report_raw: &[u8], public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let expected = make_report(public_raw, oracle_raw);
    let supplied: Value = match serde_json::from_slice(report_raw) {
        Ok(value) => value,
        Err(error) => {
            return json!({
                "status":"FAIL",
                "audit_kind":"synthetic_anytime_qualification_replay",
                "report_sha256":sha256(report_raw),
                "input_sha256":sha256(public_raw),
                "oracle_sha256":sha256(oracle_raw),
                "findings":[{"code":"REPORT_JSON_INVALID","detail":error.to_string()}]
            });
        }
    };
    let matches = supplied == expected && expected["status"] == "PASS";
    json!({
        "status":if matches {"PASS"} else {"FAIL"},
        "audit_kind":"synthetic_anytime_qualification_replay",
        "benchmark_id":BENCHMARK_ID,
        "report_sha256":sha256(report_raw),
        "input_sha256":sha256(public_raw),
        "oracle_sha256":sha256(oracle_raw),
        "findings":if matches {json!([])} else {json!([{"code":"REPORT_REPLAY_MISMATCH","detail":"supplied report differs from deterministic replay of exact task/oracle bytes"}])}
    })
}

fn read_file(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read {path}: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() == 5 && args[1] == "--replay" {
        let report_raw = match read_file(&args[2]) {Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
        let public_raw = match read_file(&args[3]) {Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
        let oracle_raw = match read_file(&args[4]) {Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
        let result = replay(&report_raw, &public_raw, &oracle_raw);
        let code = if result["status"] == "PASS" {0} else {2};
        println!("{}", serde_json::to_string_pretty(&result).expect("replay serializes"));
        return ExitCode::from(code);
    }
    if args.len() != 3 {
        eprintln!("Usage:\n  audit-qualification-anytime <public-task.json> <sealed-oracle.json>\n  audit-qualification-anytime --replay <report.json> <public-task.json> <sealed-oracle.json>");
        return ExitCode::from(64);
    }
    let public_raw = match read_file(&args[1]) {Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
    let oracle_raw = match read_file(&args[2]) {Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
    let result = make_report(&public_raw, &oracle_raw);
    let code = if result["status"] == "PASS" {0} else {2};
    println!("{}", serde_json::to_string_pretty(&result).expect("report serializes"));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests {
    use super::*;

    const PUBLIC: &[u8] = include_bytes!("../../fixtures/glass-anytime-qualification-task.synthetic.json");
    const ORACLE: &[u8] = include_bytes!("../../fixtures/glass-anytime-qualification-oracle.synthetic.json");

    #[test]
    fn exact_binomial_tail_known_answers_for_one_trial() {
        assert!(close(log_binomial_upper_tail(1, 1, 0.95).exp(), 0.95));
        assert!(close(log_binomial_lower_tail(1, 1, 0.95).exp(), 1.0));
        assert!(close(log_binomial_upper_tail(1, 0, 0.95).exp(), 1.0));
        assert!(close(log_binomial_lower_tail(1, 0, 0.95).exp(), 0.05));
    }

    #[test]
    fn moderate_exact_binomial_tails_match_independent_known_answers() {
        assert!(close(log_binomial_lower_tail(100, 85, 0.95).exp(), 0.00013585423899229444));
        assert!(close(log_binomial_upper_tail(80, 76, 0.95).exp(), 0.6288797712651971));
        assert!(close(log_binomial_lower_tail(80, 76, 0.95).exp(), 0.5715513627795555));
    }

    #[test]
    fn log_tail_keeps_extreme_probability_finite_when_raw_probability_underflows() {
        let log_probability = log_binomial_lower_tail(1000, 0, 0.95);
        assert!(log_probability.is_finite());
        assert!(close(log_probability, 1000.0 * 0.05_f64.ln()));
        assert_eq!(log_probability.exp(), 0.0);
        assert!(log_probability < per_tail_alpha(100, 1000).ln());
    }

    #[test]
    fn exact_tail_tests_respect_allocated_false_decision_probability_on_a_grid() {
        for registration_index in 1..=8 {
            for n in 1..=48 {
                let alpha = per_tail_alpha(registration_index, n);
                let log_alpha = alpha.ln();
                let mut probability_mass = log_binomial_pmf(n, 0, TARGET_SUCCESS_PROBABILITY).exp();
                let mut false_qualification_probability = 0.0;
                let mut false_rejection_probability = 0.0;
                for successes in 0..=n {
                    if log_binomial_upper_tail(n, successes, TARGET_SUCCESS_PROBABILITY) <= log_alpha {
                        false_qualification_probability += probability_mass;
                    }
                    if log_binomial_lower_tail(n, successes, TARGET_SUCCESS_PROBABILITY) <= log_alpha {
                        false_rejection_probability += probability_mass;
                    }
                    if successes < n {
                        probability_mass *= ((n - successes) as f64 / (successes + 1) as f64)
                            * (TARGET_SUCCESS_PROBABILITY / (1.0 - TARGET_SUCCESS_PROBABILITY));
                    }
                }
                assert!(false_qualification_probability <= alpha + 1e-10,
                    "false qualification {false_qualification_probability} > alpha {alpha} at candidate {registration_index}, n={n}");
                assert!(false_rejection_probability <= alpha + 1e-10,
                    "false rejection {false_rejection_probability} > alpha {alpha} at candidate {registration_index}, n={n}");
            }
        }
    }

    #[test]
    fn first_candidate_all_success_boundary_matches_the_allocated_time_budget() {
        let log_alpha_309 = per_tail_alpha(1, 309).ln();
        let log_alpha_310 = per_tail_alpha(1, 310).ln();
        assert!(log_binomial_upper_tail(309, 309, TARGET_SUCCESS_PROBABILITY) > log_alpha_309);
        assert!(log_binomial_upper_tail(310, 310, TARGET_SUCCESS_PROBABILITY) <= log_alpha_310);
    }

    #[test]
    fn countable_candidate_and_time_allocations_sum_to_at_most_global_alpha() {
        let mut total = 0.0;
        for i in 1..=1000 {
            for t in 1..=1000 {
                total += 2.0 * per_tail_alpha(i, t);
            }
        }
        assert!(total <= FAMILYWISE_ALPHA + 1e-10);
        assert!(total > FAMILYWISE_ALPHA * 0.99);
    }

    #[test]
    fn event_stream_interleaves_registration_and_sampling_and_stops_on_evidence() {
        let report: Value = make_report(PUBLIC, ORACLE);
        assert_eq!(report["status"], "PASS");
        assert_eq!(report["candidate_count"], 5);
        let decisions = report["decisions"].as_array().unwrap();
        let get = |id: &str| decisions.iter().find(|item| item["candidate_id"] == id).unwrap();
        assert_eq!(get("lot-alpha")["disposition"], "QUALIFIED");
        assert_eq!(get("lot-alpha")["trials_observed"], 310);
        assert_eq!(get("lot-beta")["disposition"], "QUALIFIED");
        assert_eq!(get("lot-beta")["trials_observed"], 334);
        assert_eq!(get("lot-gamma")["disposition"], "NOT_QUALIFIED");
        assert_eq!(get("lot-gamma")["trials_observed"], 185);
        assert_eq!(get("lot-gamma")["successes_observed"], 156);
        assert_eq!(get("lot-delta")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-delta")["trials_observed"], 80);
        assert_eq!(get("lot-epsilon")["disposition"], "UNRESOLVED");
        assert_eq!(get("lot-epsilon")["trials_observed"], 0);
        assert_eq!(report["events_checked"], 914);
    }

    #[test]
    fn later_registration_does_not_reset_or_reuse_earlier_alpha_budget() {
        assert!(per_tail_alpha(2, 1) < per_tail_alpha(1, 1));
        assert!(per_tail_alpha(5, 20) < per_tail_alpha(2, 20));
        assert!(per_tail_alpha(2, 30) < per_tail_alpha(2, 20));
    }

    #[test]
    fn rejects_observation_before_registration_terminal_reuse_and_forged_receipts() {
        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        let oracle: Value = serde_json::from_slice(ORACLE).unwrap();
        public["events"][0]["kind"] = json!("observe_trial");
        let invalid = make_report(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(invalid["status"], "FAIL");
        assert!(invalid["findings"].as_array().unwrap().iter().any(|item| item["code"] == "OBSERVATION_BEFORE_REGISTRATION"));

        let mut public: Value = serde_json::from_slice(PUBLIC).unwrap();
        // Add an extra trial for alpha after its qualification crossing.
        public["events"].as_array_mut().unwrap().insert(312, json!({"event_seq":313,"kind":"observe_trial","candidate_id":"lot-alpha"}));
        for (index, event) in public["events"].as_array_mut().unwrap().iter_mut().enumerate() {
            event["event_seq"] = json!(index + 1);
        }
        let invalid = make_report(&serde_json::to_vec(&public).unwrap(), &serde_json::to_vec(&oracle).unwrap());
        assert_eq!(invalid["status"], "FAIL");
        assert!(invalid["findings"].as_array().unwrap().iter().any(|item| item["code"] == "OBSERVATION_AFTER_TERMINAL_DECISION"));

        let mut forged: Value = make_report(PUBLIC, ORACLE);
        forged["decisions"][0]["disposition"] = json!("UNRESOLVED");
        assert_eq!(replay(&serde_json::to_vec(&forged).unwrap(), PUBLIC, ORACLE)["status"], "FAIL");
    }
}
