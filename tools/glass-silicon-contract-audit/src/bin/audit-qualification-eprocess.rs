use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, HashSet};
use std::env;
use std::fs;
use std::process::ExitCode;

const BENCHMARK_ID: &str = "glass-anytime-eprocess-qualification-synthetic-v0-1";
const CLAIM_CEILING: &str = "Synthetic Bernoulli mixture-e-process mechanics only; no real-material, plant-yield, independence, adaptive-ledger-integrity, or industrial-performance claim is validated.";
const TARGET_SUCCESS_PROBABILITY: f64 = 0.95;
const FAMILYWISE_ALPHA: f64 = 0.05;
const MAX_CANDIDATES: usize = 100;
const MAX_TRIALS_PER_CANDIDATE: usize = 1000;
const MAX_EVENTS: usize = 100_000;

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn close(left: f64, right: f64) -> bool {
    left.is_finite() && right.is_finite() && (left - right).abs() <= 1e-9
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
    if left == f64::NEG_INFINITY { return right; }
    if right == f64::NEG_INFINITY { return left; }
    let maximum = left.max(right);
    maximum + ((left - maximum).exp() + (right - maximum).exp()).ln()
}

fn log_binomial_upper_tail(n: usize, successes: usize, p: f64) -> f64 {
    if successes == 0 { return 0.0; }
    if successes > n || p <= 0.0 { return f64::NEG_INFINITY; }
    if p >= 1.0 { return 0.0; }
    let mut log_term = log_binomial_pmf(n, successes, p);
    let mut log_sum = log_term;
    for k in successes..n {
        log_term += ((n - k) as f64).ln() - ((k + 1) as f64).ln() + p.ln() - (-p).ln_1p();
        log_sum = log_add_exp(log_sum, log_term);
    }
    log_sum.min(0.0)
}

fn log_binomial_lower_tail(n: usize, successes: usize, p: f64) -> f64 {
    if successes >= n { return 0.0; }
    if p <= 0.0 { return 0.0; }
    if p >= 1.0 { return f64::NEG_INFINITY; }
    let mut log_term = log_binomial_pmf(n, successes, p);
    let mut log_sum = log_term;
    for k in (1..=successes).rev() {
        log_term += (k as f64).ln() - ((n - k + 1) as f64).ln() + (-p).ln_1p() - p.ln();
        log_sum = log_add_exp(log_sum, log_term);
    }
    log_sum.min(0.0)
}

fn log_factorial(n: usize) -> f64 {
    (1..=n).map(|k| (k as f64).ln()).sum()
}

fn log_beta_integer(successes: usize, failures: usize) -> f64 {
    log_factorial(successes) + log_factorial(failures) - log_factorial(successes + failures + 1)
}

// Uniform mixture of likelihood ratios for q in (p0, 1). It is a nonnegative
// supermartingale under every Bernoulli p <= p0.
fn log_e_upper(n: usize, successes: usize, p0: f64) -> f64 {
    let failures = n - successes;
    let log_beta = log_beta_integer(successes, failures);
    // Beta(x+1, n-x+1) survival at p0 equals P[Binomial(n+1,p0) <= x].
    let log_beta_survival = log_binomial_lower_tail(n + 1, successes, p0);
    log_beta + log_beta_survival
        - (1.0 - p0).ln()
        - (successes as f64) * p0.ln()
        - (failures as f64) * (1.0 - p0).ln()
}

// Uniform mixture of likelihood ratios for q in (0, p0). It is a nonnegative
// supermartingale under every Bernoulli p >= p0.
fn log_e_lower(n: usize, successes: usize, p0: f64) -> f64 {
    let failures = n - successes;
    let log_beta = log_beta_integer(successes, failures);
    // Beta(x+1, n-x+1) CDF at p0 equals P[Binomial(n+1,p0) >= x+1].
    let log_beta_cdf = log_binomial_upper_tail(n + 1, successes + 1, p0);
    log_beta + log_beta_cdf
        - p0.ln()
        - (successes as f64) * p0.ln()
        - (failures as f64) * (1.0 - p0).ln()
}

fn candidate_alpha(registration_index: usize) -> f64 {
    FAMILYWISE_ALPHA / ((registration_index as f64) * ((registration_index + 1) as f64))
}

#[derive(Clone)]
struct CandidateSpec {
    digest: String,
    registration_index: usize,
    trials_seen: usize,
    successes: usize,
    terminal_decision: Option<String>,
    terminal_event_seq: Option<usize>,
    last_log_e_upper: Option<f64>,
    last_log_e_lower: Option<f64>,
}

fn empty_report(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    json!({
        "schema_version":1,
        "status":"PASS",
        "audit_kind":"synthetic_anytime_mixture_eprocess_qualification",
        "benchmark_id":BENCHMARK_ID,
        "input_sha256":sha256(public_raw),
        "oracle_sha256":sha256(oracle_raw),
        "target_success_probability":TARGET_SUCCESS_PROBABILITY,
        "familywise_alpha":FAMILYWISE_ALPHA,
        "allocation":"alpha/[i*(i+1)] per candidate, split equally between two mixture e-processes",
        "candidate_count":0,
        "events_checked":0,
        "events":[],
        "decisions":[],
        "findings":[],
        "claim_ceiling":CLAIM_CEILING
    })
}

fn make_report(public_raw: &[u8], oracle_raw: &[u8]) -> Value {
    let mut report = empty_report(public_raw, oracle_raw);
    let public: Value = match serde_json::from_slice(public_raw) {
        Ok(v) => v,
        Err(e) => { fail(&mut report, "PUBLIC_JSON_INVALID", &e.to_string()); return report; }
    };
    let oracle: Value = match serde_json::from_slice(oracle_raw) {
        Ok(v) => v,
        Err(e) => { fail(&mut report, "ORACLE_JSON_INVALID", &e.to_string()); return report; }
    };

    only_keys(&public, &["schema_version","benchmark_id","evidence_class","claim_ceiling","target_success_probability","familywise_alpha","registration_rule","events"], "public", &mut report);
    only_keys(&oracle, &["schema_version","benchmark_id","evidence_class","claim_ceiling","outcomes"], "oracle", &mut report);
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
        fail(&mut report, "CLAIM_CEILING_INVALID", "v0.1 synthetic claim ceiling is frozen");
    }
    if !public.get("target_success_probability").and_then(Value::as_f64).is_some_and(|v| close(v,TARGET_SUCCESS_PROBABILITY))
        || !public.get("familywise_alpha").and_then(Value::as_f64).is_some_and(|v| close(v,FAMILYWISE_ALPHA))
        || public.get("registration_rule").and_then(Value::as_str) != Some("append_only_one_based_registration_order")
    {
        fail(&mut report, "STATISTICAL_CONTRACT_CHANGED", "target probability, family-wise alpha and registration rule are frozen");
    }

    let Some(public_events) = public.get("events").and_then(Value::as_array) else {
        fail(&mut report, "EVENTS_REQUIRED", "public.events must be an array");
        return report;
    };
    if public_events.is_empty() || public_events.len() > MAX_EVENTS {
        fail(&mut report, "EVENT_COUNT_INVALID", &format!("events must be between 1 and {MAX_EVENTS}"));
        return report;
    }
    let Some(oracle_items) = oracle.get("outcomes").and_then(Value::as_array) else {
        fail(&mut report, "OUTCOMES_REQUIRED", "oracle.outcomes must be an array");
        return report;
    };
    let mut oracle_by_id = BTreeMap::<String, Vec<bool>>::new();
    for (index, item) in oracle_items.iter().enumerate() {
        let path = format!("oracle.outcomes[{index}]");
        only_keys(item,&["candidate_id","trial_results"],&path,&mut report);
        let Some(id) = required_str(item,"candidate_id",&path,&mut report) else {continue};
        if !valid_id(&id) { fail(&mut report,"CANDIDATE_ID_INVALID",&format!("{path}.candidate_id is invalid")); }
        let Some(results) = item.get("trial_results").and_then(Value::as_array) else {
            fail(&mut report,"TRIALS_REQUIRED",&format!("{path}.trial_results must be an array"));
            continue;
        };
        if results.len() > MAX_TRIALS_PER_CANDIDATE {
            fail(&mut report,"TRIAL_COUNT_INVALID",&format!("{path} exceeds {MAX_TRIALS_PER_CANDIDATE} trials"));
            continue;
        }
        let mut parsed=Vec::with_capacity(results.len());
        let mut valid=true;
        for (trial_index,result) in results.iter().enumerate() {
            if let Some(value)=result.as_bool(){parsed.push(value)} else {
                valid=false;
                fail(&mut report,"TRIAL_RESULT_INVALID",&format!("{path}.trial_results[{trial_index}] must be boolean"));
            }
        }
        if oracle_by_id.contains_key(&id) {
            fail(&mut report,"OUTCOME_CANDIDATE_DUPLICATE",&format!("duplicate oracle record for {id}"));
        } else if valid {
            oracle_by_id.insert(id,parsed);
        }
    }

    let mut states=BTreeMap::<String,CandidateSpec>::new();
    let mut registration_order=Vec::<String>::new();
    let mut cursors=BTreeMap::<String,usize>::new();
    let mut receipts=Vec::<Value>::new();

    for (position,event) in public_events.iter().enumerate() {
        let path=format!("public.events[{position}]");
        let seq=position+1;
        let Some(kind)=event.get("kind").and_then(Value::as_str) else {
            fail(&mut report,"EVENT_KIND_REQUIRED",&format!("{path}.kind must be a string"));
            continue;
        };
        let event_keys:&[&str]=if kind=="register_candidate" {
            &["event_seq","kind","candidate_id","candidate_spec","candidate_spec_sha256"]
        } else {
            &["event_seq","kind","candidate_id"]
        };
        only_keys(event,event_keys,&path,&mut report);
        if event.get("event_seq").and_then(Value::as_u64)!=Some(seq as u64) {
            fail(&mut report,"EVENT_SEQUENCE_INVALID",&format!("{path}.event_seq must equal {seq}"));
        }
        let Some(id)=required_str(event,"candidate_id",&path,&mut report) else {continue};
        if !valid_id(&id) {
            fail(&mut report,"CANDIDATE_ID_INVALID",&format!("{path}.candidate_id must match ^[a-z][a-z0-9-]*$"));
        }

        match kind {
            "register_candidate" => {
                if states.contains_key(&id) {
                    fail(&mut report,"CANDIDATE_REGISTRATION_DUPLICATE",&format!("{path} registers {id} twice"));
                    continue;
                }
                if registration_order.len()>=MAX_CANDIDATES {
                    fail(&mut report,"CANDIDATE_COUNT_INVALID","candidate family exceeds 100");
                    continue;
                }
                let Some(spec)=event.get("candidate_spec") else {
                    fail(&mut report,"CANDIDATE_SPEC_REQUIRED",&format!("{path}.candidate_spec is required"));
                    continue;
                };
                only_keys(spec,&["version","features"],&format!("{path}.candidate_spec"),&mut report);
                if !spec.get("version").and_then(Value::as_str).is_some_and(|s|!s.trim().is_empty())
                    || !spec.get("features").and_then(Value::as_array).is_some_and(|xs|xs.len()==2 && xs.iter().all(|x|x.as_f64().is_some_and(f64::is_finite)))
                {
                    fail(&mut report,"CANDIDATE_SPEC_INVALID",&format!("{path}.candidate_spec requires a version and two finite synthetic features"));
                    continue;
                }
                let spec_bytes=serde_json::to_vec(spec).unwrap_or_default();
                let digest=sha256(&spec_bytes);
                if event.get("candidate_spec_sha256").and_then(Value::as_str)!=Some(digest.as_str()) {
                    fail(&mut report,"CANDIDATE_SPEC_DIGEST_MISMATCH",&format!("{path} candidate spec digest does not match the registered spec"));
                    continue;
                }
                let registration_index=registration_order.len()+1;
                registration_order.push(id.clone());
                states.insert(id.clone(),CandidateSpec{
                    digest: digest.clone(),
                    registration_index,
                    trials_seen:0,
                    successes:0,
                    terminal_decision:None,
                    terminal_event_seq:None,
                    last_log_e_upper:None,
                    last_log_e_lower:None,
                });
                cursors.insert(id.clone(),0);
                receipts.push(json!({"event_seq":seq,"kind":"register_candidate","candidate_id":id,"registration_index":registration_index,"candidate_spec_sha256":digest}));
            }
            "observe_trial" => {
                let Some(before)=states.get(&id).cloned() else {
                    fail(&mut report,"OBSERVATION_BEFORE_REGISTRATION",&format!("{path} observes {id} before registration"));
                    continue;
                };
                if let Some(disposition)=before.terminal_decision.as_deref() {
                    fail(&mut report,"OBSERVATION_AFTER_TERMINAL_DECISION",&format!("{path} observes {id} after {disposition}"));
                    continue;
                }
                if before.trials_seen>=MAX_TRIALS_PER_CANDIDATE {
                    fail(&mut report,"TRIAL_COUNT_INVALID",&format!("{path} exceeds the per-candidate trial cap"));
                    continue;
                }
                let Some(trials)=oracle_by_id.get(&id) else {
                    fail(&mut report,"OUTCOME_CANDIDATE_MISSING",&format!("oracle lacks registered candidate {id}"));
                    continue;
                };
                let cursor=cursors.get(&id).copied().unwrap_or(0);
                let Some(passed)=trials.get(cursor).copied() else {
                    fail(&mut report,"SEALED_OUTCOME_MISSING",&format!("{path} needs missing sealed trial {cursor} for {id}"));
                    continue;
                };
                let state=states.get_mut(&id).expect("registered candidate state exists");
                state.trials_seen+=1;
                if passed {state.successes+=1;}
                cursors.insert(id.clone(),cursor+1);
                let n=state.trials_seen;
                let x=state.successes;
                let registration_index=state.registration_index;
                let alpha_side=candidate_alpha(registration_index)/2.0;
                let log_upper=log_e_upper(n,x,TARGET_SUCCESS_PROBABILITY);
                let log_lower=log_e_lower(n,x,TARGET_SUCCESS_PROBABILITY);
                let log_threshold=-alpha_side.ln();
                let upper_crossed=log_upper>=log_threshold;
                let lower_crossed=log_lower>=log_threshold;
                if upper_crossed && lower_crossed {
                    fail(&mut report,"CONTRADICTORY_EPROCESS_CROSSING",&format!("{path} both directional e-processes crossed their thresholds"));
                }
                let decision=if upper_crossed && !lower_crossed {
                    Some("QUALIFIED")
                } else if lower_crossed && !upper_crossed {
                    Some("NOT_QUALIFIED")
                } else {None};
                state.last_log_e_upper=Some(log_upper);
                state.last_log_e_lower=Some(log_lower);
                if let Some(value)=decision {
                    state.terminal_decision=Some(value.to_owned());
                    state.terminal_event_seq=Some(seq);
                }
                receipts.push(json!({
                    "event_seq":seq,
                    "kind":"observe_trial",
                    "candidate_id":id,
                    "registration_index":registration_index,
                    "look_index":n,
                    "trial_passed":passed,
                    "successes_so_far":x,
                    "alpha_per_direction":alpha_side,
                    "log_e_upper":log_upper,
                    "log_e_lower":log_lower,
                    "log_threshold":log_threshold,
                    "decision":decision.unwrap_or("UNRESOLVED")
                }));
            }
            other => fail(&mut report,"EVENT_KIND_UNREGISTERED",&format!("{path} has event kind {other}")),
        }
    }

    let registered=registration_order.iter().cloned().collect::<HashSet<_>>();
    if registration_order.is_empty() {
        fail(&mut report,"CANDIDATE_REGISTRY_EMPTY","at least one candidate must be registered");
    }
    if oracle_by_id.keys().any(|id|!registered.contains(id)) {
        fail(&mut report,"ORACLE_HAS_UNREGISTERED_CANDIDATE","oracle includes a candidate absent from the public registration stream");
    }
    for id in &registration_order {
        if !oracle_by_id.contains_key(id) {
            fail(&mut report,"OUTCOME_CANDIDATE_MISSING",&format!("oracle has no outcomes for {id}"));
        }
    }
    if oracle_by_id.len()!=registration_order.len() {
        fail(&mut report,"ORACLE_COVERAGE_INVALID","oracle candidate set must match registered candidate set");
    }
    if report["status"]=="FAIL" {return report;}

    let mut decisions=Vec::new();
    for id in &registration_order {
        let state=&states[id];
        decisions.push(json!({
            "candidate_id":id,
            "candidate_spec_sha256":state.digest,
            "registration_index":state.registration_index,
            "trials_observed":state.trials_seen,
            "successes_observed":state.successes,
            "empirical_success_rate":if state.trials_seen==0 {Value::Null} else {json!(state.successes as f64/state.trials_seen as f64)},
            "disposition":state.terminal_decision.as_deref().unwrap_or("UNRESOLVED"),
            "terminal_event_seq":state.terminal_event_seq,
            "last_log_e_upper":state.last_log_e_upper,
            "last_log_e_lower":state.last_log_e_lower
        }));
    }
    report["candidate_count"]=json!(registration_order.len());
    report["events_checked"]=json!(receipts.len());
    report["events"]=json!(receipts);
    report["decisions"]=json!(decisions);
    report
}

fn replay(report_raw:&[u8],public_raw:&[u8],oracle_raw:&[u8])->Value {
    let expected=make_report(public_raw,oracle_raw);
    let supplied:Value=match serde_json::from_slice(report_raw) {
        Ok(v)=>v,
        Err(e)=>return json!({"status":"FAIL","audit_kind":"synthetic_anytime_eprocess_qualification_replay","report_sha256":sha256(report_raw),"input_sha256":sha256(public_raw),"oracle_sha256":sha256(oracle_raw),"findings":[{"code":"REPORT_JSON_INVALID","detail":e.to_string()}]})
    };
    let matches=supplied==expected && expected["status"]=="PASS";
    json!({
        "status":if matches {"PASS"} else {"FAIL"},
        "audit_kind":"synthetic_anytime_eprocess_qualification_replay",
        "benchmark_id":BENCHMARK_ID,
        "report_sha256":sha256(report_raw),
        "input_sha256":sha256(public_raw),
        "oracle_sha256":sha256(oracle_raw),
        "findings":if matches {json!([])} else {json!([{"code":"REPORT_REPLAY_MISMATCH","detail":"report differs from deterministic replay of exact task and oracle bytes"}])}
    })
}

fn read_file(path:&str)->Result<Vec<u8>,String>{fs::read(path).map_err(|e|format!("cannot read {path}: {e}"))}

fn main()->ExitCode{
    let args:Vec<String>=env::args().collect();
    if args.len()==5 && args[1]=="--replay"{
        let r=match read_file(&args[2]){Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
        let p=match read_file(&args[3]){Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
        let o=match read_file(&args[4]){Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
        let report=replay(&r,&p,&o);
        let code=if report["status"]=="PASS"{0}else{2};
        println!("{}",serde_json::to_string_pretty(&report).expect("replay serializes"));
        return ExitCode::from(code);
    }
    if args.len()!=3{
        eprintln!("Usage:\\n  audit-qualification-eprocess <public-task.json> <sealed-oracle.json>\\n  audit-qualification-eprocess --replay <report.json> <public-task.json> <sealed-oracle.json>");
        return ExitCode::from(64);
    }
    let p=match read_file(&args[1]){Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
    let o=match read_file(&args[2]){Ok(v)=>v,Err(e)=>{eprintln!("{e}");return ExitCode::from(2)}};
    let report=make_report(&p,&o);
    let code=if report["status"]=="PASS"{0}else{2};
    println!("{}",serde_json::to_string_pretty(&report).expect("report serializes"));
    ExitCode::from(code)
}

#[cfg(test)]
mod tests{
    use super::*;
    const PUBLIC:&[u8]=include_bytes!("../../fixtures/glass-eprocess-qualification-task.synthetic.json");
    const ORACLE:&[u8]=include_bytes!("../../fixtures/glass-eprocess-qualification-oracle.synthetic.json");

    fn fixture()->Value{make_report(PUBLIC,ORACLE)}

    #[test]
    fn one_trial_mixture_e_values_have_known_answers(){
        assert!(close(log_e_upper(1,1,0.95).exp(),1.0263157894736842));
        assert!(close(log_e_lower(1,1,0.95).exp(),0.5));
        assert!(close(log_e_upper(1,0,0.95).exp(),0.5));
        assert!(close(log_e_lower(1,0,0.95).exp(),10.5));
        assert!(close(log_e_upper(0,0,0.95),0.0));
        assert!(close(log_e_lower(0,0,0.95),0.0));
    }

    #[test]
    fn mixture_e_process_martingale_identity_holds_under_the_boundary(){
        let p0=TARGET_SUCCESS_PROBABILITY;
        for n in 0..=48{
            for x in 0..=n{
                let upper=log_e_upper(n,x,p0).exp();
                let upper_failure=log_e_upper(n+1,x,p0).exp();
                let upper_success=log_e_upper(n+1,x+1,p0).exp();
                assert!(close((1.0-p0)*upper_failure+p0*upper_success,upper),
                    "upper e-process martingale identity failed at n={n}, x={x}");
                let lower=log_e_lower(n,x,p0).exp();
                let lower_failure=log_e_lower(n+1,x,p0).exp();
                let lower_success=log_e_lower(n+1,x+1,p0).exp();
                assert!(close((1.0-p0)*lower_failure+p0*lower_success,lower),
                    "lower e-process martingale identity failed at n={n}, x={x}");
            }
        }
    }

    #[test]
    fn e_process_crossing_is_more_efficient_than_alpha_spending_on_same_all_pass_streams(){
        let alpha_spend_threshold=|i:usize,n:usize| 0.05/(2.0*i as f64*(i+1) as f64*n as f64*(n+1) as f64);
        let e_threshold=|i:usize| candidate_alpha(i)/2.0;
        let mut alpha_first=None;
        let mut e_first=None;
        for n in 1..=500{
            let log_e=log_e_upper(n,n,0.95);
            if alpha_first.is_none() && 0.95_f64.powi(n as i32)<=alpha_spend_threshold(1,n){
                alpha_first=Some(n);
            }
            if e_first.is_none() && log_e>=-e_threshold(1).ln(){
                e_first=Some(n);
            }
        }
        assert_eq!(alpha_first,Some(310));
        assert_eq!(e_first,Some(121));
        assert!(121*100<310*40);
    }

    #[test]
    fn fixture_stops_only_after_valid_mixture_evidence(){
        let report=fixture();
        assert_eq!(report["status"],"PASS");
        assert_eq!(report["candidate_count"],5);
        let decisions=report["decisions"].as_array().unwrap();
        let get=|id:&str| decisions.iter().find(|d|d["candidate_id"]==id).unwrap();
        assert_eq!(get("lot-alpha")["disposition"],"QUALIFIED");
        assert_eq!(get("lot-alpha")["trials_observed"],121);
        assert_eq!(get("lot-beta")["disposition"],"QUALIFIED");
        assert_eq!(get("lot-beta")["trials_observed"],146);
        assert_eq!(get("lot-gamma")["disposition"],"NOT_QUALIFIED");
        assert_eq!(get("lot-gamma")["trials_observed"],106);
        assert_eq!(get("lot-gamma")["successes_observed"],89);
        assert_eq!(get("lot-delta")["disposition"],"UNRESOLVED");
        assert_eq!(get("lot-delta")["trials_observed"],80);
        assert_eq!(get("lot-epsilon")["disposition"],"UNRESOLVED");
        assert_eq!(get("lot-epsilon")["trials_observed"],0);
        assert_eq!(report["events_checked"],458);
    }

    #[test]
    fn candidate_registration_budgets_sum_to_global_alpha(){
        let total:f64=(1..10000).map(|i|candidate_alpha(i)).sum();
        assert!(total<FAMILYWISE_ALPHA+1e-10);
        assert!(total>FAMILYWISE_ALPHA*0.999);
    }

    #[test]
    fn altered_candidate_spec_or_forged_report_is_rejected(){
        let mut task:Value=serde_json::from_slice(PUBLIC).unwrap();
        let oracle:Value=serde_json::from_slice(ORACLE).unwrap();
        task["events"][0]["candidate_spec"]["features"][0]=json!(999.0);
        let invalid=make_report(&serde_json::to_vec(&task).unwrap(),&serde_json::to_vec(&oracle).unwrap());
        assert_eq!(invalid["status"],"FAIL");
        assert!(invalid["findings"].as_array().unwrap().iter().any(|x|x["code"]=="CANDIDATE_SPEC_DIGEST_MISMATCH"));

        let mut forged=fixture();
        forged["decisions"][0]["disposition"]=json!("UNRESOLVED");
        assert_eq!(replay(&serde_json::to_vec(&forged).unwrap(),PUBLIC,ORACLE)["status"],"FAIL");
    }

    #[test]
    fn observations_before_registration_and_after_terminal_decision_fail_closed(){
        let mut task:Value=serde_json::from_slice(PUBLIC).unwrap();
        let oracle:Value=serde_json::from_slice(ORACLE).unwrap();
        task["events"].as_array_mut().unwrap().remove(0);
        for (i,event) in task["events"].as_array_mut().unwrap().iter_mut().enumerate(){event["event_seq"]=json!(i+1);}
        let invalid=make_report(&serde_json::to_vec(&task).unwrap(),&serde_json::to_vec(&oracle).unwrap());
        assert_eq!(invalid["status"],"FAIL");

        let mut forged=fixture();
        // The report replay exact-compares all fields, including event order and terminal sample count.
        forged["events"][1]["decision"]=json!("UNRESOLVED");
        assert_eq!(replay(&serde_json::to_vec(&forged).unwrap(),PUBLIC,ORACLE)["status"],"FAIL");
    }
}
