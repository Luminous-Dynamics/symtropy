use serde_json::{json, Value};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const FROZEN: &str = include_str!("../../../benchmarks/maritime-control-response/control-response-corpus-v0.1.json");
type Check<T> = Result<T, String>;

fn object<'a>(v: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    v.as_object().ok_or_else(|| format!("{at}: expected object"))
}
fn exact_keys(v: &Value, keys: &[&str], at: &str) -> Check<()> {
    let map = object(v, at)?;
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual: BTreeSet<&str> = map.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let unknown: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !unknown.is_empty() {
        return Err(format!("{at}: missing keys {missing:?}; unknown keys {unknown:?}"));
    }
    Ok(())
}
fn field<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a Value> {
    object(v, at)?.get(key).ok_or_else(|| format!("{at}: missing {key}"))
}
fn string<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    field(v, key, at)?.as_str().filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{at}.{key}: expected non-empty string"))
}
fn number(v: &Value, key: &str, at: &str) -> Check<f64> {
    let n = field(v, key, at)?.as_f64().ok_or_else(|| format!("{at}.{key}: expected number"))?;
    if !n.is_finite() { return Err(format!("{at}.{key}: non-finite number")); }
    Ok(n)
}
fn unsigned(v: &Value, key: &str, at: &str) -> Check<u64> {
    field(v, key, at)?.as_u64().ok_or_else(|| format!("{at}.{key}: expected non-negative integer"))
}
fn boolean(v: &Value, key: &str, at: &str) -> Check<bool> {
    field(v, key, at)?.as_bool().ok_or_else(|| format!("{at}.{key}: expected boolean"))
}
fn string_set(v: &Value, key: &str, at: &str) -> Check<BTreeSet<String>> {
    let values = field(v, key, at)?.as_array().ok_or_else(|| format!("{at}.{key}: expected array"))?;
    let mut result = BTreeSet::new();
    for item in values {
        let s = item.as_str().filter(|s| !s.trim().is_empty())
            .ok_or_else(|| format!("{at}.{key}: expected non-empty strings"))?;
        if !result.insert(s.to_owned()) { return Err(format!("{at}.{key}: duplicate item")); }
    }
    Ok(result)
}
fn quality(v: &Value, at: &str) -> Check<&'static str> {
    match string(v, "quality", at)? {
        "CURRENT" => Ok("CURRENT"), "MISSING" => Ok("MISSING"), "STALE" => Ok("STALE"),
        "REJECTED" => Ok("REJECTED"), "CONFLICTED" => Ok("CONFLICTED"), "UNKNOWN" => Ok("UNKNOWN"),
        other => Err(format!("{at}.quality: unknown quality {other}")),
    }
}
#[derive(Debug)]
struct Signal {
    role: String, value: f64, unit: String, observed_at_ms: u64,
    quality: String, dependencies: BTreeSet<String>, derived_from: BTreeSet<String>,
}
fn signal(v: &Value, at: &str, response: bool) -> Check<Signal> {
    let _signal_id = string(v, "signal_id", at)?;
    if response {
        exact_keys(v, &["signal_id","role","value","unit","observed_at_ms","quality","dependency_groups","derived_from_signal_ids"], at)?;
    } else {
        exact_keys(v, &["signal_id","role","value","unit","observed_at_ms","quality","dependency_groups"], at)?;
    }
    let derived = if response { string_set(v,"derived_from_signal_ids",at)? } else { BTreeSet::new() };
    Ok(Signal {
        role: string(v,"role",at)?.to_owned(), value: number(v,"value",at)?,
        unit: string(v,"unit",at)?.to_owned(), observed_at_ms: unsigned(v,"observed_at_ms",at)?,
        quality: quality(v,at)?.to_owned(), dependencies: string_set(v,"dependency_groups",at)?,
        derived_from: derived,
    })
}
fn overlaps(a: &BTreeSet<String>, b: &BTreeSet<String>) -> bool {
    a.iter().any(|item| b.contains(item))
}
fn result(disposition: &str, reasons: &[&str], feedback: &str) -> Value {
    json!({"disposition":disposition,"reason_codes":reasons,"fault_isolation":"NOT_ATTEMPTED",
        "actuator_delivery_confirmation":"NOT_ESTABLISHED","actuator_feedback_state":feedback})
}
fn evaluate_case(case: &Value, max_age: u64, max_horizon: u64) -> Check<(String, Value)> {
    let at = "$.cases[]";
    exact_keys(case,&["case_id","asset_configuration_id","regime_id","regime_qualified","prediction_time_ms","command","context","response","response_envelope","actuator_feedback","expected_disposition"],at)?;
    let id = string(case,"case_id",at)?.to_owned();
    let prediction = unsigned(case,"prediction_time_ms",at)?;
    let configuration = string(case,"asset_configuration_id",at)?;
    let regime = string(case,"regime_id",at)?;
    let regime_qualified = boolean(case,"regime_qualified",at)?;
    let command = signal(field(case,"command",at)?,&format!("{at}.command"),false)?;
    let context = signal(field(case,"context",at)?,&format!("{at}.context"),false)?;
    let response = signal(field(case,"response",at)?,&format!("{at}.response"),true)?;
    if command.role != "CONTROL_INTENT" || context.role != "DISTURBANCE_CONTEXT" || response.role != "PHYSICAL_RESPONSE" {
        return Ok((id,result("INVALID_CONTRACT",&["SIGNAL_ROLE_MISMATCH"],"NOT_ASSESSED")));
    }
    let feedback = field(case,"actuator_feedback",at)?;
    let feedback_state = if feedback.is_null() { "FEEDBACK_UNAVAILABLE" } else {
        let fb = signal(feedback,&format!("{at}.actuator_feedback"),false)?;
        if fb.role != "ACTUATOR_FEEDBACK" {
            return Ok((id,result("INVALID_CONTRACT",&["ACTUATOR_FEEDBACK_ROLE_MISMATCH"],"FEEDBACK_ROLE_INVALID")));
        }
        if fb.observed_at_ms > response.observed_at_ms {
            return Ok((id,result("INVALID_CONTRACT",&["FUTURE_ACTUATOR_FEEDBACK"],"FEEDBACK_TIMESTAMP_INVALID")));
        }
        if fb.quality == "CURRENT" { "FEEDBACK_PRESENT_NOT_DELIVERY_PROOF" } else { "FEEDBACK_NOT_CURRENT" }
    };
    let envelope = field(case,"response_envelope",at)?;
    exact_keys(envelope,&["model_id","configuration_id","regime_id","issued_at_ms","trained_through_ms","qualified","lower","mean","upper","unit"],&format!("{at}.response_envelope"))?;
    let model_id = string(envelope,"model_id",at)?;
    let envelope_config = string(envelope,"configuration_id",at)?;
    let envelope_regime = string(envelope,"regime_id",at)?;
    let issued = unsigned(envelope,"issued_at_ms",at)?;
    let trained = unsigned(envelope,"trained_through_ms",at)?;
    let model_qualified = boolean(envelope,"qualified",at)?;
    let lower = number(envelope,"lower",at)?;
    let mean = number(envelope,"mean",at)?;
    let upper = number(envelope,"upper",at)?;
    let envelope_unit = string(envelope,"unit",at)?;
    if lower > mean || mean > upper {
        return Ok((id,result("INVALID_CONTRACT",&["INVALID_RESPONSE_INTERVAL"],feedback_state)));
    }
    if issued > prediction || trained > prediction {
        return Ok((id,result("INVALID_CONTRACT",&["FUTURE_MODEL_INFORMATION"],feedback_state)));
    }
    if command.observed_at_ms > prediction || context.observed_at_ms > prediction {
        return Ok((id,result("INVALID_CONTRACT",&["FUTURE_COMMAND_OR_CONTEXT_LEAKAGE"],feedback_state)));
    }
    if response.observed_at_ms < prediction || response.observed_at_ms - prediction > max_horizon {
        return Ok((id,result("INVALID_CONTRACT",&["RESPONSE_OUTSIDE_PREDICTION_HORIZON"],feedback_state)));
    }
    if response.unit != envelope_unit {
        return Ok((id,result("INVALID_CONTRACT",&["RESPONSE_ENVELOPE_UNIT_MISMATCH"],feedback_state)));
    }
    if field(case,"expected_disposition",at)?.as_str().is_none() { return Err(format!("{id}: expected disposition must be a string")); }
    if prediction.saturating_sub(command.observed_at_ms) > max_age || prediction.saturating_sub(context.observed_at_ms) > max_age {
        return Ok((id,result("NOT_EVALUATED",&["INPUT_TOO_OLD"],feedback_state)));
    }
    if command.quality != "CURRENT" || context.quality != "CURRENT" || response.quality != "CURRENT" {
        return Ok((id,result("NOT_EVALUATED",&["INPUT_QUALITY_NOT_CURRENT"],feedback_state)));
    }
    if !response.derived_from.is_empty() {
        return Ok((id,result("NOT_EVALUATED",&["RESPONSE_HAS_DERIVED_PARENTS"],feedback_state)));
    }
    if !regime_qualified || !model_qualified {
        return Ok((id,result("NOT_EVALUATED",&["REGIME_OR_MODEL_NOT_QUALIFIED"],feedback_state)));
    }
    if configuration != envelope_config || regime != envelope_regime {
        return Ok((id,result("NOT_EVALUATED",&["CONFIGURATION_OR_REGIME_MISMATCH"],feedback_state)));
    }
    if overlaps(&command.dependencies,&context.dependencies)
        || overlaps(&command.dependencies,&response.dependencies)
        || overlaps(&context.dependencies,&response.dependencies) {
        return Ok((id,result("NOT_EVALUATED",&["SHARED_COMMON_CAUSE_DEPENDENCY"],feedback_state)));
    }
    let disposition = if response.value >= lower && response.value <= upper {
        "RESPONSE_CONSISTENT"
    } else { "RESPONSE_MISMATCH_UNISOLATED" };
    let reasons: &[&str] = if disposition == "RESPONSE_CONSISTENT" { &[] }
        else { &["OBSERVED_RESPONSE_OUTSIDE_CONDITIONED_ENVELOPE"] };
    let mut out = result(disposition,reasons,feedback_state);
    out["response_envelope_model_id"] = json!(model_id);
    out["observed_response"] = json!(response.value);
    out["expected_response_mean"] = json!(mean);
    out["expected_response_interval"] = json!([lower,upper]);
    out["command_value"] = json!(command.value);
    out["context_value"] = json!(context.value);
    Ok((id,out))
}
fn pair_check(pair: &Value, outcomes: &BTreeMap<String,Value>, cases: &BTreeMap<String,&Value>) -> Check<Value> {
    let at = "$.counterfactual_pairs[]";
    let kind = string(pair,"kind",at)?;
    let ids = field(pair,"case_ids",at)?.as_array().ok_or_else(|| format!("{at}.case_ids: expected array"))?;
    if ids.len()!=2 { return Err(format!("{at}: exactly two case IDs required")); }
    let a_id=ids[0].as_str().ok_or_else(|| format!("{at}: case ID must be text"))?;
    let b_id=ids[1].as_str().ok_or_else(|| format!("{at}: case ID must be text"))?;
    let a=*cases.get(a_id).ok_or_else(|| format!("{at}: unknown case {a_id}"))?;
    let b=*cases.get(b_id).ok_or_else(|| format!("{at}: unknown case {b_id}"))?;
    for id in [a_id,b_id] {
        if outcomes.get(id).and_then(|v|v["disposition"].as_str()) != Some("RESPONSE_CONSISTENT") {
            return Err(format!("{at}: pair member {id} is not response-consistent"));
        }
    }
    let ca=field(a,"command",at)?; let cb=field(b,"command",at)?;
    let xa=field(a,"context",at)?; let xb=field(b,"context",at)?;
    let ra=field(a,"response",at)?; let rb=field(b,"response",at)?;
    let ea=field(a,"response_envelope",at)?; let eb=field(b,"response_envelope",at)?;
    let delta=|x:f64,y:f64|(x-y).abs();
    match kind {
        "SAME_COMMAND_DIFFERENT_CONTEXT" => {
            exact_keys(pair,&["pair_id","kind","case_ids","minimum_context_delta","minimum_expected_response_delta","minimum_observed_response_delta"],at)?;
            let same=number(ca,"value",at)?==number(cb,"value",at)? && string(ca,"unit",at)?==string(cb,"unit",at)?;
            if !same || string(xa,"unit",at)?!=string(xb,"unit",at)?
                || delta(number(xa,"value",at)?,number(xb,"value",at)?) < number(pair,"minimum_context_delta",at)?
                || delta(number(ea,"mean",at)?,number(eb,"mean",at)?) < number(pair,"minimum_expected_response_delta",at)?
                || delta(number(ra,"value",at)?,number(rb,"value",at)?) < number(pair,"minimum_observed_response_delta",at)? {
                return Err(format!("{at}: same-command/different-context pair failed"));
            }
        }
        "DIFFERENT_COMMAND_SAME_RESPONSE" => {
            exact_keys(pair,&["pair_id","kind","case_ids","minimum_command_delta","maximum_expected_response_delta","maximum_observed_response_delta"],at)?;
            if string(ca,"unit",at)?!=string(cb,"unit",at)? || string(ra,"unit",at)?!=string(rb,"unit",at)?
                || delta(number(ca,"value",at)?,number(cb,"value",at)?) < number(pair,"minimum_command_delta",at)?
                || delta(number(ea,"mean",at)?,number(eb,"mean",at)?) > number(pair,"maximum_expected_response_delta",at)?
                || delta(number(ra,"value",at)?,number(rb,"value",at)?) > number(pair,"maximum_observed_response_delta",at)? {
                return Err(format!("{at}: different-command/same-response pair failed"));
            }
        }
        other => return Err(format!("{at}: unknown pair kind {other}")),
    }
    Ok(json!({"pair_id":string(pair,"pair_id",at)?,"kind":kind,"status":"PASS","case_ids":[a_id,b_id]}))
}
fn valid_hex(s:&str,n:usize)->bool { s.len()==n && s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)) }
fn run(path:&str,source:&str,digest:&str)->Check<Value> {
    if !valid_hex(source,40)||!valid_hex(digest,64) { return Err("invalid source commit or corpus digest".to_owned()); }
    let raw=fs::read_to_string(path).map_err(|e|format!("read corpus: {e}"))?;
    let corpus:Value=serde_json::from_str(&raw).map_err(|e|format!("parse corpus: {e}"))?;
    exact_keys(&corpus,&["schema_version","corpus_id","evidence_class","max_input_age_ms","max_response_horizon_ms","cases","counterfactual_pairs","authority_boundary","claim_ceiling"],"$")?;
    if string(&corpus,"schema_version","$")? != VERSION || string(&corpus,"evidence_class","$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err("wrong schema version or evidence class".to_owned());
    }
    let authority=field(&corpus,"authority_boundary","$")?;
    exact_keys(authority,&["read_only","controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"],"$.authority_boundary")?;
    if !boolean(authority,"read_only","$.authority_boundary")? { return Err("read_only must be true".to_owned()); }
    for key in ["controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"] {
        if boolean(authority,key,"$.authority_boundary")? { return Err(format!("authority boundary violation: {key}")); }
    }
    let max_age=unsigned(&corpus,"max_input_age_ms","$")?;
    let max_horizon=unsigned(&corpus,"max_response_horizon_ms","$")?;
    let items=field(&corpus,"cases","$")?.as_array().ok_or_else(||"cases must be array".to_owned())?;
    if items.is_empty()||items.len()>10000 { return Err("case count outside 1..=10000".to_owned()); }
    let mut outcomes=BTreeMap::new(); let mut case_map=BTreeMap::new();
    for case in items {
        let (id,out)=evaluate_case(case,max_age,max_horizon)?;
        let expected=string(case,"expected_disposition","$.cases[]")?;
        if out["disposition"].as_str()!=Some(expected) { return Err(format!("{id}: expected {expected}, got {}",out["disposition"])); }
        if case_map.insert(id.clone(),case).is_some() { return Err(format!("duplicate case ID {id}")); }
        outcomes.insert(id,out);
    }
    let pairs=field(&corpus,"counterfactual_pairs","$")?.as_array().ok_or_else(||"counterfactual_pairs must be array".to_owned())?;
    let mut pair_receipts=Vec::new(); let mut pair_ids=BTreeSet::new();
    for pair in pairs {
        let id=string(pair,"pair_id","$.counterfactual_pairs[]")?.to_owned();
        if !pair_ids.insert(id.clone()) { return Err(format!("duplicate pair ID {id}")); }
        pair_receipts.push(pair_check(pair,&outcomes,&case_map)?);
    }
    let case_results:Vec<Value>=outcomes.iter().map(|(id,out)|json!({"case_id":id,"result":out})).collect();
    Ok(json!({"result":"pass","validator_version":VERSION,"corpus_id":string(&corpus,"corpus_id","$")?,
        "source_commit":source,"corpus_sha256":digest,"scenario_count":items.len(),
        "counterfactual_pair_count":pair_receipts.len(),"cases":case_results,
        "counterfactual_pairs":pair_receipts,"claim_ceiling":string(&corpus,"claim_ceiling","$")?}))
}
fn main()->ExitCode {
    let a:Vec<String>=env::args().collect();
    if a.len()!=6||a[2]!="--source-commit"||a[4]!="--corpus-sha256" {
        eprintln!("usage: evaluator <corpus.json> --source-commit <40-hex> --corpus-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    match run(&a[1],&a[3],&a[5]) {
        Ok(v)=>{println!("{}",serde_json::to_string_pretty(&v).unwrap_or_else(|_|"{\"result\":\"fail\"}".to_owned()));ExitCode::SUCCESS}
        Err(e)=>{eprintln!("benchmark failed closed: {e}");ExitCode::FAILURE}
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn corpus()->Value { serde_json::from_str(FROZEN).expect("frozen fixture JSON") }
    #[test]
    fn all_sixteen_synthetic_cases_match_known_truth() {
        let c=corpus(); let items=c["cases"].as_array().expect("cases"); assert_eq!(items.len(),16);
        for case in items {
            let (id,out)=evaluate_case(case,2000,5000).expect("case evaluates");
            assert_eq!(out["disposition"],case["expected_disposition"],"{id}");
        }
    }
    #[test]
    fn both_counterfactual_pairs_pass() {
        let c=corpus(); let mut outcomes=BTreeMap::new(); let mut cases=BTreeMap::new();
        for case in c["cases"].as_array().unwrap() {
            let (id,out)=evaluate_case(case,2000,5000).unwrap(); outcomes.insert(id.clone(),out); cases.insert(id,case);
        }
        let pairs=c["counterfactual_pairs"].as_array().unwrap(); assert_eq!(pairs.len(),2);
        for pair in pairs { assert_eq!(pair_check(pair,&outcomes,&cases).unwrap()["status"],"PASS"); }
    }
    #[test]
    fn future_trained_model_and_command_fail_closed() {
        let mut c=corpus(); c["cases"][0]["response_envelope"]["trained_through_ms"]=json!(2000);
        assert_eq!(evaluate_case(&c["cases"][0],2000,5000).unwrap().1["disposition"],"INVALID_CONTRACT");
        c=corpus(); c["cases"][0]["command"]["observed_at_ms"]=json!(2000);
        assert_eq!(evaluate_case(&c["cases"][0],2000,5000).unwrap().1["disposition"],"INVALID_CONTRACT");
    }
    #[test]
    fn role_promotion_fails_closed() {
        let mut c=corpus(); c["cases"][0]["response"]["role"]=json!("CONTROL_INTENT");
        assert_eq!(evaluate_case(&c["cases"][0],2000,5000).unwrap().1["disposition"],"INVALID_CONTRACT");
    }
    #[test]
    fn derived_response_is_not_independent() {
        let mut c=corpus(); c["cases"][0]["response"]["derived_from_signal_ids"]=json!(["fuel_index_command"]);
        assert_eq!(evaluate_case(&c["cases"][0],2000,5000).unwrap().1["disposition"],"NOT_EVALUATED");
    }
    #[test]
    fn common_cause_withdraws_qualification() {
        let mut c=corpus(); c["cases"][0]["response"]["dependency_groups"]=json!(["controller-power"]);
        assert_eq!(evaluate_case(&c["cases"][0],2000,5000).unwrap().1["disposition"],"NOT_EVALUATED");
    }
    #[test]
    fn command_never_confirms_delivery_or_fault() {
        let c=corpus(); let out=evaluate_case(&c["cases"][0],2000,5000).unwrap().1;
        assert_eq!(out["fault_isolation"],"NOT_ATTEMPTED"); assert_eq!(out["actuator_delivery_confirmation"],"NOT_ESTABLISHED");
    }
    #[test]
    fn out_of_envelope_is_unisolated() {
        let c=corpus(); let out=evaluate_case(&c["cases"][5],2000,5000).unwrap().1;
        assert_eq!(out["disposition"],"RESPONSE_MISMATCH_UNISOLATED"); assert_eq!(out["actuator_delivery_confirmation"],"NOT_ESTABLISHED");
    }
}
