// SPDX-License-Identifier: AGPL-3.0-or-later
//! Maritime adapter for Symtropy's reusable pairwise sensor-observability kernel.
//! This module performs maritime identity/provenance admission and delegates
//! interval, availability, dependency-cut and independent-principle semantics
//! to symtropy-sensor-observability-bench. It does not duplicate that algorithm.

use maritime_observability_evidence_validator::verify_sha256;
use serde_json::{json, Value};
use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    process::ExitCode,
};
use symtropy_sensor_observability_bench::{
    classify_pair_signatures, ChannelPairSignature, PairAvailability, PairEvidenceClass,
    PairEvidenceRole,
};

const VERSION: &str = "0.1.0";
const KERNEL_REV: &str = "3c4a86de830a24bbf97c7d7fe30d79de299999c5";
#[cfg(test)]
const FIXTURE: &str = include_str!("../../../../benchmarks/maritime-observability/pairwise-observability-case.v0.1.json");

type Errors = Vec<Value>;

#[derive(Clone, Debug)]
struct Channel {
    unit: String,
    principle_group: String,
    dependencies: BTreeSet<String>,
    raw: Value,
}

#[derive(Clone, Copy)]
struct EvaluationContext<'a> {
    evidence_class: PairEvidenceClass,
    asset_sha256: &'a str,
    regime_qualified: bool,
    max_timestamp_uncertainty_ms: f64,
}

type ValidatedPairCase = (
    String,
    String,
    String,
    PairEvidenceClass,
    BTreeMap<String, Channel>,
    Vec<Value>,
    Vec<Value>,
);

fn push(errors: &mut Errors, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(json!({"code":code,"path":path,"detail":detail.into()}));
}

fn obj(v: &Value, path: &str, errors: &mut Errors) -> bool {
    if !v.is_object() {
        push(errors,"EXPECTED_OBJECT",path,"expected object");
        false
    } else { true }
}

fn exact_keys(v: &Value, required: &[&str], optional: &[&str], path: &str, errors: &mut Errors) {
    if !obj(v,path,errors) { return; }
    let map=v.as_object().expect("object checked");
    let allowed:BTreeSet<&str>=required.iter().chain(optional.iter()).copied().collect();
    for key in required {
        if !map.contains_key(*key) {
            push(errors,"MISSING_KEY",&format!("{path}.{key}"),"required key missing");
        }
    }
    for key in map.keys() {
        if !allowed.contains(key.as_str()) {
            push(errors,"UNKNOWN_KEY",&format!("{path}.{key}"),"unknown key rejected");
        }
    }
}

fn text<'a>(v:&'a Value,key:&str)->Option<&'a str>{
    v.get(key).and_then(Value::as_str).map(str::trim).filter(|s|!s.is_empty())
}
fn finite(v:&Value,key:&str)->Option<f64>{
    v.get(key).and_then(Value::as_f64).filter(|x|x.is_finite())
}
fn sha256(s:&str)->bool{
    s.len()==64 && s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))
}
fn string_array(v:&Value,key:&str,path:&str,errors:&mut Errors)->BTreeSet<String>{
    let mut out=BTreeSet::new();
    let Some(items)=v.get(key).and_then(Value::as_array) else {
        push(errors,"EXPECTED_ARRAY",&format!("{path}.{key}"),"expected array");
        return out;
    };
    for (i,item) in items.iter().enumerate(){
        let Some(s)=item.as_str().filter(|s|!s.trim().is_empty()) else {
            push(errors,"INVALID_ARRAY_ITEM",&format!("{path}.{key}[{i}]"),"expected non-empty string");
            continue;
        };
        if !out.insert(s.to_owned()){
            push(errors,"DUPLICATE_ARRAY_ITEM",&format!("{path}.{key}[{i}]"),"duplicate item");
        }
    }
    out
}
fn interval(v:&Value,key:&str,path:&str,errors:&mut Errors)->Option<(f64,f64)>{
    let Some(value)=v.get(key) else {
        push(errors,"MISSING_INTERVAL",&format!("{path}.{key}"),"interval field required");
        return None;
    };
    if value.is_null(){ return None; }
    let Some(items)=value.as_array() else {
        push(errors,"INTERVAL_SHAPE",&format!("{path}.{key}"),"expected null or two-number closed interval");
        return None;
    };
    if items.len()!=2 {
        push(errors,"INTERVAL_LENGTH",&format!("{path}.{key}"),"interval must contain exactly two endpoints");
        return None;
    }
    let (Some(lower),Some(upper))=(items[0].as_f64(),items[1].as_f64()) else {
        push(errors,"INTERVAL_NONFINITE",&format!("{path}.{key}"),"interval endpoints must be finite numbers");
        return None;
    };
    if !lower.is_finite()||!upper.is_finite()||lower>upper {
        push(errors,"INTERVAL_ORDER",&format!("{path}.{key}"),"interval must be finite and lower <= upper");
        return None;
    }
    Some((lower,upper))
}

fn channel_is_measured_and_qualified(channel:&Channel,context:EvaluationContext<'_>)->bool{
    let v=&channel.raw;
    text(v,"source_role")==Some("measured")
        && v["installed"].as_bool()==Some(true)
        && text(v,"quality_state")==Some("current")
        && text(v,"calibration_state")==Some("current")
        && text(v,"timebase_qualification_state")==Some("qualified")
        && text(v,"timebase_id").is_some()
        && finite(v,"timestamp_uncertainty_ms").is_some_and(|x|x>=0.0&&x<=context.max_timestamp_uncertainty_ms)
        && v["timebase_qualification_sha256"].as_str().is_some_and(sha256)
        && text(v,"configuration_sha256")==Some(context.asset_sha256)
        && v["derived_from_channel_ids"].as_array().is_some_and(|a|a.is_empty())
}

fn evidence_role(raw:Option<&str>,channel:&Channel,context:EvaluationContext<'_>,common_timebase:Option<&str>,common_clock_digest:Option<&str>)->PairEvidenceRole{
    match raw {
        Some("SYNTHETIC_CONFORMANCE_FIXTURE")
            if context.evidence_class==PairEvidenceClass::SyntheticConformanceFixture
                && text(&channel.raw,"source_role")==Some("synthetic_fixture")
                && channel.raw["configuration_sha256"].as_str()==Some(context.asset_sha256)
                && text(&channel.raw,"quality_state")==Some("current")
            => PairEvidenceRole::SyntheticConformanceFixture,
        Some("INDEPENDENT_MEASUREMENT")
            if context.evidence_class==PairEvidenceClass::Measured
                && context.regime_qualified
                && channel_is_measured_and_qualified(channel,context)
                && text(&channel.raw,"timebase_id")==common_timebase
                && channel.raw["timebase_qualification_sha256"].as_str()==common_clock_digest
            => PairEvidenceRole::IndependentMeasurement,
        Some("DERIVED")=>PairEvidenceRole::Derived,
        Some("RECONSTRUCTED")=>PairEvidenceRole::Reconstructed,
        Some("SIMULATED")=>PairEvidenceRole::Simulated,
        Some("EXPERT_AUTHORED")=>PairEvidenceRole::ExpertAuthored,
        _=>PairEvidenceRole::Unknown,
    }
}

fn availability(raw:Option<&str>)->Option<PairAvailability>{
    match raw {
        Some("VALID")=>Some(PairAvailability::Valid),
        Some("DECLARED_FAULT_UNAVAILABLE")=>Some(PairAvailability::DeclaredFaultUnavailable),
        Some("UNEXPECTED_UNAVAILABLE")=>Some(PairAvailability::UnexpectedUnavailable),
        Some("UNKNOWN")=>Some(PairAvailability::Unknown),
        _=>None,
    }
}

fn validate_case(case:&Value)->Result<ValidatedPairCase,Errors>{
    let mut errors=Vec::new();
    exact_keys(case,&["schema_version","case_id","evidence_class","asset","operating_domain","maximum_timestamp_uncertainty_ms","channels","pairs","dependency_cut_scenarios","authority_boundary","claim_ceiling"],&[],"$",&mut errors);
    if text(case,"schema_version")!=Some("0.1.0"){
        push(&mut errors,"SCHEMA_VERSION","$.schema_version","expected 0.1.0");
    }
    let case_id=text(case,"case_id").unwrap_or("").to_owned();
    if case_id.is_empty(){push(&mut errors,"CASE_ID_REQUIRED","$.case_id","case ID required");}
    let class=text(case,"evidence_class").unwrap_or("");
    let evidence_class=match class{
        "MEASURED"=>PairEvidenceClass::Measured,
        "SYNTHETIC_CONFORMANCE_FIXTURE"=>PairEvidenceClass::SyntheticConformanceFixture,
        _=>{push(&mut errors,"EVIDENCE_CLASS","$.evidence_class","expected MEASURED or SYNTHETIC_CONFORMANCE_FIXTURE");PairEvidenceClass::SyntheticConformanceFixture}
    };
    let asset=&case["asset"];
    exact_keys(asset,&["asset_id","configuration_revision","configuration_sha256"],&[],"$.asset",&mut errors);
    let asset_id=text(asset,"asset_id").unwrap_or("").to_owned();
    let asset_sha=text(asset,"configuration_sha256").unwrap_or("");
    if asset_id.is_empty(){push(&mut errors,"ASSET_ID_REQUIRED","$.asset.asset_id","asset ID required");}
    if !sha256(asset_sha){push(&mut errors,"CONFIGURATION_DIGEST","$.asset.configuration_sha256","expected lowercase SHA-256");}
    let regime=text(&case["operating_domain"],"regime_id").unwrap_or("").to_owned();
    exact_keys(&case["operating_domain"],&["regime_id","qualification_state"],&[],"$.operating_domain",&mut errors);
    if regime.is_empty(){push(&mut errors,"REGIME_REQUIRED","$.operating_domain.regime_id","regime ID required");}
    if !matches!(text(&case["operating_domain"],"qualification_state"),Some("qualified"|"unsupported"|"not_evaluated")){
        push(&mut errors,"REGIME_STATE","$.operating_domain.qualification_state","invalid regime state");
    }
    if finite(case,"maximum_timestamp_uncertainty_ms").is_none_or(|x|x<=0.0){
        push(&mut errors,"MAX_TIMESTAMP_UNCERTAINTY_INVALID","$.maximum_timestamp_uncertainty_ms","maximum timestamp uncertainty must be finite and positive");
    }

    let Some(items)=case["channels"].as_array() else{
        push(&mut errors,"CHANNELS_REQUIRED","$.channels","channels array required");
        return Err(errors);
    };
    if items.is_empty(){push(&mut errors,"CHANNELS_EMPTY","$.channels","at least one channel required");}
    if items.len()>4096{push(&mut errors,"CHANNEL_LIMIT","$.channels","maximum 4096 channels");}
    let mut channels=BTreeMap::new();
    for (i,item) in items.iter().enumerate(){
        let path=format!("$.channels[{i}]");
        exact_keys(item,&["sensor_id","channel_id","installed","measurement_principle","independence_group_ref","unit","source_role","quality_state","calibration_state","configuration_sha256","dependency_groups","derived_from_channel_ids","timebase_id","timestamp_uncertainty_ms","timebase_qualification_state","timebase_qualification_sha256"],&[] ,&path,&mut errors);
        let id=text(item,"channel_id").unwrap_or("").to_owned();
        if id.is_empty(){push(&mut errors,"CHANNEL_ID_REQUIRED",&format!("{path}.channel_id"),"channel ID required");continue;}
        if text(item,"sensor_id").is_none()||text(item,"measurement_principle").is_none()||text(item,"independence_group_ref").is_none()||text(item,"unit").is_none(){
            push(&mut errors,"CHANNEL_IDENTITY_REQUIRED",&path,"sensor, measurement principle, independent-principle group and unit are required");
        }
        if item["installed"].as_bool().is_none(){push(&mut errors,"INSTALLED_REQUIRED",&path,"installed must be boolean");}
        if !matches!(text(item,"source_role"),Some("measured"|"derived"|"reconstructed"|"simulated"|"synthetic_fixture")){
            push(&mut errors,"SOURCE_ROLE",&format!("{path}.source_role"),"invalid source role");
        }
        if !matches!(text(item,"quality_state"),Some("current"|"missing"|"stale"|"rejected"|"conflicted"|"unknown")){
            push(&mut errors,"QUALITY_STATE",&format!("{path}.quality_state"),"invalid quality state");
        }
        if !matches!(text(item,"calibration_state"),Some("current"|"expired"|"unknown"|"not_applicable")){
            push(&mut errors,"CALIBRATION_STATE",&format!("{path}.calibration_state"),"invalid calibration state");
        }
        if !matches!(text(item,"timebase_qualification_state"),Some("qualified"|"unqualified"|"unknown")){
            push(&mut errors,"TIMEBASE_STATE",&format!("{path}.timebase_qualification_state"),"invalid timebase state");
        }
        let deps=string_array(item,"dependency_groups",&path,&mut errors);
        let parents=string_array(item,"derived_from_channel_ids",&path,&mut errors);
        if text(item,"source_role")==Some("measured")&&!parents.is_empty(){
            push(&mut errors,"MEASURED_DERIVATION",&format!("{path}.derived_from_channel_ids"),"measured channels cannot declare derived parents");
        }
        if matches!(text(item,"source_role"),Some("derived"|"reconstructed"))&&parents.is_empty(){
            push(&mut errors,"DERIVATION_PARENT_REQUIRED",&format!("{path}.derived_from_channel_ids"),"derived/reconstructed channels need parent identities");
        }
        if text(item,"configuration_sha256")!=Some(asset_sha){
            push(&mut errors,"CHANNEL_CONFIGURATION_MISMATCH",&format!("{path}.configuration_sha256"),"channel must bind to exact asset configuration digest");
        }
        if text(item,"timebase_qualification_state")==Some("qualified")
            && (text(item,"timebase_id").is_none()
                || finite(item,"timestamp_uncertainty_ms").is_none_or(|v|v<0.0)
                || !item["timebase_qualification_sha256"].as_str().is_some_and(sha256)){
            push(&mut errors,"QUALIFIED_TIMEBASE_INCOMPLETE",&path,"qualified timebase requires ID, nonnegative uncertainty and SHA-256");
        }
        let channel=Channel{
            unit:text(item,"unit").unwrap_or("").to_owned(),
            principle_group:text(item,"independence_group_ref").unwrap_or("").to_owned(),
            dependencies:deps,
            raw:item.clone(),
        };
        if channels.insert(id.clone(),channel).is_some(){
            push(&mut errors,"DUPLICATE_CHANNEL_ID",&format!("{path}.channel_id"),"channel IDs must be unique");
        }
    }

    let Some(pairs)=case["pairs"].as_array() else{
        push(&mut errors,"PAIRS_REQUIRED","$.pairs","pairs array required");
        return Err(errors);
    };
    if pairs.is_empty(){push(&mut errors,"PAIRS_EMPTY","$.pairs","at least one pair required");}
    let mut pair_ids=BTreeSet::new();
    for (i,pair) in pairs.iter().enumerate(){
        let path=format!("$.pairs[{i}]");
        exact_keys(pair,&["pair_id","fault_a","fault_b","critical","minimum_independent_principles","measurement_window_id_a","measurement_window_id_b","responses"],&["expected_status"],&path,&mut errors);
        let id=text(pair,"pair_id").unwrap_or("");
        if id.is_empty()||!pair_ids.insert(id.to_owned()){
            push(&mut errors,"PAIR_ID_INVALID",&format!("{path}.pair_id"),"pair IDs must be unique and non-empty");
        }
        if text(pair,"fault_a").is_none()||text(pair,"fault_b").is_none()||text(pair,"fault_a")==text(pair,"fault_b"){
            push(&mut errors,"FAULT_PAIR_INVALID",&path,"pair must reference two distinct fault hypotheses");
        }
        if text(pair,"measurement_window_id_a").is_none()||text(pair,"measurement_window_id_b").is_none()
            || text(pair,"measurement_window_id_a")==text(pair,"measurement_window_id_b"){
            push(&mut errors,"MEASUREMENT_WINDOWS_INVALID",&path,"the two fault signatures must bind distinct non-empty synchronized measurement-window IDs");
        }
        if pair["critical"].as_bool().is_none(){push(&mut errors,"CRITICAL_FLAG_REQUIRED",&format!("{path}.critical"),"critical must be boolean");}
        if pair["minimum_independent_principles"].as_u64().is_none_or(|v|v==0){
            push(&mut errors,"PAIR_THRESHOLD_INVALID",&format!("{path}.minimum_independent_principles"),"minimum independent principles must be positive");
        }
        if let Some(expected)=text(pair,"expected_status")
            && !matches!(expected,"IDENTIFIABLE"|"NOT_IDENTIFIABLE"|"INSUFFICIENT_EVIDENCE"){
            push(&mut errors,"EXPECTED_STATUS_INVALID",&format!("{path}.expected_status"),"invalid expected status");
        }
        let Some(responses)=pair["responses"].as_array() else{
            push(&mut errors,"PAIR_RESPONSES_REQUIRED",&format!("{path}.responses"),"responses array required");
            continue;
        };
        let mut seen=BTreeSet::new();
        for (j,response) in responses.iter().enumerate(){
            let rpath=format!("{path}.responses[{j}]");
            exact_keys(response,&["channel_id","unit","interval_a","interval_b","availability_a","availability_b","evidence_role_a","evidence_role_b"],&[] ,&rpath,&mut errors);
            let channel_id=text(response,"channel_id").unwrap_or("");
            if !seen.insert(channel_id.to_owned()){push(&mut errors,"DUPLICATE_PAIR_CHANNEL",&rpath,"one response per channel is permitted");}
            let Some(channel)=channels.get(channel_id) else{
                push(&mut errors,"UNKNOWN_PAIR_CHANNEL",&rpath,"pair response references unknown channel");
                continue;
            };
            if text(response,"unit")!=Some(channel.unit.as_str()){
                push(&mut errors,"PAIR_CHANNEL_UNIT_MISMATCH",&format!("{rpath}.unit"),"response unit must match sensor inventory");
            }
            for (interval_key,availability_key) in [("interval_a","availability_a"),("interval_b","availability_b")]{
                let parsed=interval(response,interval_key,&rpath,&mut errors);
                let avail=text(response,availability_key);
                if !matches!(avail,Some("VALID"|"DECLARED_FAULT_UNAVAILABLE"|"UNEXPECTED_UNAVAILABLE"|"UNKNOWN")){
                    push(&mut errors,"AVAILABILITY_INVALID",&format!("{rpath}.{availability_key}"),"invalid availability state");
                }
                if avail==Some("VALID")&&parsed.is_none(){
                    push(&mut errors,"VALID_RESPONSE_INTERVAL_REQUIRED",&format!("{rpath}.{interval_key}"),"VALID response requires interval");
                }
                if avail==Some("DECLARED_FAULT_UNAVAILABLE")&&parsed.is_some(){
                    push(&mut errors,"UNAVAILABLE_RESPONSE_HAS_INTERVAL",&format!("{rpath}.{interval_key}"),"unavailable response must not carry an interval");
                }
            }
            for key in ["evidence_role_a","evidence_role_b"]{
                if !matches!(text(response,key),Some("INDEPENDENT_MEASUREMENT"|"SYNTHETIC_CONFORMANCE_FIXTURE"|"DERIVED"|"RECONSTRUCTED"|"SIMULATED"|"EXPERT_AUTHORED"|"UNKNOWN")){
                    push(&mut errors,"EVIDENCE_ROLE_INVALID",&format!("{rpath}.{key}"),"invalid evidence role");
                }
            }
        }
        let channel_ids:BTreeSet<String>=responses.iter().filter_map(|r|text(r,"channel_id").map(str::to_owned)).collect();
        let inventory_ids:BTreeSet<String>=channels.keys().cloned().collect();
        if channel_ids!=inventory_ids{
            push(&mut errors,"INCOMPLETE_PAIR_RESPONSE_MATRIX",&format!("{path}.responses"),"each pair must carry exactly one response for every declared channel");
        }
    }

    let Some(cuts)=case["dependency_cut_scenarios"].as_array() else{
        push(&mut errors,"DEPENDENCY_CUTS_REQUIRED","$.dependency_cut_scenarios","dependency cut array required");
        return Err(errors);
    };
    let mut cut_ids=BTreeSet::new();
    for (i,cut) in cuts.iter().enumerate(){
        let path=format!("$.dependency_cut_scenarios[{i}]");
        exact_keys(cut,&["scenario_id","disabled_dependency_groups"],&["expected_statuses"],&path,&mut errors);
        let id=text(cut,"scenario_id").unwrap_or("");
        if id.is_empty()||!cut_ids.insert(id.to_owned()){push(&mut errors,"CUT_ID_INVALID",&format!("{path}.scenario_id"),"cut IDs must be unique and non-empty");}
        let disabled=string_array(cut,"disabled_dependency_groups",&path,&mut errors);
        for group in &disabled{
            if !channels.values().any(|c|c.dependencies.contains(group)){
                push(&mut errors,"UNKNOWN_DEPENDENCY_CUT",&format!("{path}.disabled_dependency_groups"),format!("unknown dependency group {group}"));
            }
        }
        if let Some(expected)=cut.get("expected_statuses").and_then(Value::as_array){
            for item in expected{
                exact_keys(item,&["pair_id","status"],&[] ,&format!("{path}.expected_statuses[]"),&mut errors);
                if !pairs.iter().any(|p|text(p,"pair_id")==text(item,"pair_id")){
                    push(&mut errors,"UNKNOWN_EXPECTED_PAIR",&path,"expected status references unknown pair");
                }
                if !matches!(text(item,"status"),Some("IDENTIFIABLE"|"NOT_IDENTIFIABLE"|"INSUFFICIENT_EVIDENCE")){
                    push(&mut errors,"EXPECTED_CUT_STATUS_INVALID",&path,"invalid expected status");
                }
            }
        }
    }

    let boundary=&case["authority_boundary"];
    exact_keys(boundary,&["read_only","actuation","maintenance_authorization","mode_transition_authorization","restriction_clearance"],&[],"$.authority_boundary",&mut errors);
    if boundary["read_only"].as_bool()!=Some(true)
        || ["actuation","maintenance_authorization","mode_transition_authorization","restriction_clearance"].iter().any(|k|boundary[*k].as_bool()!=Some(false)){
        push(&mut errors,"AUTHORITY_BOUNDARY_VIOLATION","$.authority_boundary","read-only must be true; all authority booleans must be false");
    }
    if text(case,"claim_ceiling").is_none(){push(&mut errors,"CLAIM_CEILING_REQUIRED","$.claim_ceiling","claim ceiling required");}

    if !errors.is_empty(){return Err(errors);}
    Ok((case_id,asset_id,regime,evidence_class,channels,pairs.clone(),cuts.clone()))
}

fn classify_one(pair:&Value,channels:&BTreeMap<String,Channel>,context:EvaluationContext<'_>,disabled:&BTreeSet<String>)->Result<Value,String>{
    let pair_id=text(pair,"pair_id").unwrap_or("UNKNOWN");
    let mut pair_timebases=BTreeSet::new();
    let mut pair_clock_digests=BTreeSet::new();
    for response in pair["responses"].as_array().expect("validated"){
        let channel=&channels[text(response,"channel_id").expect("validated")];
        if let Some(tb)=text(&channel.raw,"timebase_id"){pair_timebases.insert(tb.to_owned());}
        if let Some(digest)=text(&channel.raw,"timebase_qualification_sha256"){pair_clock_digests.insert(digest.to_owned());}
    }
    let common_timebase=(pair_timebases.len()==1).then(||pair_timebases.iter().next().cloned()).flatten();
    let common_clock_digest=(pair_clock_digests.len()==1).then(||pair_clock_digests.iter().next().cloned()).flatten();
    let mut signatures=Vec::new();
    for response in pair["responses"].as_array().expect("validated"){
        let channel_id=text(response,"channel_id").expect("validated");
        let channel=&channels[channel_id];
        signatures.push(ChannelPairSignature{
            channel_id:channel_id.to_owned(),
            independence_group_ref:channel.principle_group.clone(),
            dependency_groups:channel.dependencies.clone(),
            interval_a:response["interval_a"].as_array().and_then(|a|Some((a.first()?.as_f64()?,a.get(1)?.as_f64()?))),
            interval_b:response["interval_b"].as_array().and_then(|a|Some((a.first()?.as_f64()?,a.get(1)?.as_f64()?))),
            availability_a:availability(text(response,"availability_a")).expect("validated"),
            availability_b:availability(text(response,"availability_b")).expect("validated"),
            evidence_role_a:evidence_role(text(response,"evidence_role_a"),channel,context,common_timebase.as_deref(),common_clock_digest.as_deref()),
            evidence_role_b:evidence_role(text(response,"evidence_role_b"),channel,context,common_timebase.as_deref(),common_clock_digest.as_deref()),
        });
    }
    let result=classify_pair_signatures(
        &signatures,
        pair["minimum_independent_principles"].as_u64().expect("validated") as usize,
        context.evidence_class,
        disabled
    )?;
    let mut shared:BTreeMap<String,usize>=BTreeMap::new();
    for s in &signatures{for d in &s.dependency_groups{*shared.entry(d.clone()).or_insert(0)+=1;}}
    let shared_groups:Vec<String>=shared.into_iter().filter(|(_,count)|*count>1).map(|(id,_)|id).collect();
    Ok(json!({
        "pair_id":pair_id,"fault_a":text(pair,"fault_a").unwrap_or(""),"fault_b":text(pair,"fault_b").unwrap_or(""),
        "measurement_window_id_a":text(pair,"measurement_window_id_a").unwrap_or(""),
        "measurement_window_id_b":text(pair,"measurement_window_id_b").unwrap_or(""),
        "critical":pair["critical"],"status":result.status,
        "required_independent_principles":pair["minimum_independent_principles"],
        "observed_independent_principles":result.observed_independent_principles,
        "discriminating_channel_ids":result.discriminating_channel_ids,
        "independence_group_refs":result.independence_group_refs,
        "unresolved_evidence":result.unresolved_evidence,
        "shared_dependency_groups":shared_groups,
        "common_timebase_proven":common_timebase.is_some()&&common_clock_digest.is_some()
    }))
}

fn run_case(case:&Value,source:&str,digest:&str)->Result<Value,String>{
    if source.len()!=40||!source.bytes().all(|b|b.is_ascii_hexdigit())||!sha256(digest){
        return Err("invalid source commit or case SHA-256".into());
    }
    let (case_id,asset_id,regime,evidence_class,channels,pairs,cuts)=
        validate_case(case).map_err(|e|serde_json::to_string(&json!({"violations":e})).unwrap_or_else(|_|"case validation failed".into()))?;
    let asset_sha=text(&case["asset"],"configuration_sha256").unwrap_or("");
    let regime_qualified=text(&case["operating_domain"],"qualification_state")==Some("qualified");
    let max_timestamp_uncertainty=finite(case,"maximum_timestamp_uncertainty_ms").unwrap_or(0.0);
    let context=EvaluationContext{
        evidence_class,
        asset_sha256:asset_sha,
        regime_qualified,
        max_timestamp_uncertainty_ms:max_timestamp_uncertainty,
    };
    let no_cut=BTreeSet::new();
    let mut base_results=Vec::new();
    for pair in &pairs{base_results.push(classify_one(pair,&channels,context,&no_cut)?);}
    for pair in &pairs{
        if let Some(expected)=text(pair,"expected_status"){
            let actual=base_results.iter().find(|r|r["pair_id"]==pair["pair_id"]).and_then(|r|r["status"].as_str()).unwrap_or("");
            if actual!=expected{return Err(format!("{}: expected {expected}, received {actual}",text(pair,"pair_id").unwrap_or("pair")));}
        }
    }
    let mut cut_results=Vec::new();
    for cut in &cuts{
        let disabled=string_array(cut,"disabled_dependency_groups","$",&mut Vec::new());
        let mut results=Vec::new();
        for pair in &pairs{
            let result=classify_one(pair,&channels,context,&disabled)?;
            results.push(result);
        }
        if let Some(expected)=cut.get("expected_statuses").and_then(Value::as_array){
            for e in expected{
                let id=text(e,"pair_id").unwrap_or("");
                let expected_status=text(e,"status").unwrap_or("");
                let actual=results.iter().find(|r|r["pair_id"]==id).and_then(|r|r["status"].as_str()).unwrap_or("");
                if expected_status!=actual{return Err(format!("cut {} pair {id}: expected {expected_status}, got {actual}",text(cut,"scenario_id").unwrap_or("")));}
            }
        }
        cut_results.push(json!({"scenario_id":text(cut,"scenario_id").unwrap_or(""),"disabled_dependency_groups":disabled,"pair_results":results}));
    }
    let identifiable=base_results.iter().filter(|r|r["status"]=="IDENTIFIABLE").count();
    let not_identifiable=base_results.iter().filter(|r|r["status"]=="NOT_IDENTIFIABLE").count();
    let insufficient=base_results.iter().filter(|r|r["status"]=="INSUFFICIENT_EVIDENCE").count();
    Ok(json!({
        "result":"pass","adapter_version":VERSION,
        "reused_kernel":{"crate":"symtropy-sensor-observability-bench","git_revision":KERNEL_REV},
        "source_commit":source,"case_sha256":digest,"case_id":case_id,"asset_id":asset_id,"regime_id":regime,
        "evidence_class":text(case,"evidence_class").unwrap_or(""),
        "pair_count":pairs.len(),"identifiable_pair_count":identifiable,
        "not_identifiable_pair_count":not_identifiable,"insufficient_evidence_pair_count":insufficient,
        "base_pair_results":base_results,"dependency_cut_results":cut_results,
        "pair_separability_computed_by_reused_kernel":true,
        "physical_diagnosability_established":false,
        "mutation_performed":false,"authorization_granted":false,
        "claim_ceiling":text(case,"claim_ceiling").unwrap_or("")
    }))
}

fn main()->ExitCode{
    let a:Vec<String>=env::args().collect();
    if a.len()!=6||a[2]!="--source-commit"||a[4]!="--case-sha256"{
        eprintln!("usage: maritime_pairwise_observability <case.json> --source-commit <40-hex> --case-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    let raw=match fs::read_to_string(&a[1]){Ok(v)=>v,Err(e)=>{eprintln!("read case: {e}");return ExitCode::FAILURE;}};
    if let Err(e)=verify_sha256(raw.as_bytes(),&a[5]){eprintln!("case digest verification failed: {e}");return ExitCode::FAILURE;}
    let case:Value=match serde_json::from_str(&raw){Ok(v)=>v,Err(e)=>{eprintln!("parse case: {e}");return ExitCode::FAILURE;}};
    match run_case(&case,&a[3],&a[5]){
        Ok(v)=>{println!("{}",serde_json::to_string_pretty(&v).unwrap_or_else(|_|"{\"result\":\"fail\"}".into()));ExitCode::SUCCESS}
        Err(e)=>{eprintln!("maritime pairwise observability failed closed: {e}");ExitCode::FAILURE}
    }
}

#[cfg(test)]
mod tests{
    use super::*;
    fn fixture()->Value{serde_json::from_str(FIXTURE).expect("fixture parses")}
    fn run(v:&Value)->Value{
        // Mutations intentionally alter signatures and evidence. The frozen
        // fixture expectations are checked separately, not reused here.
        let mut candidate=v.clone();
        if let Some(pairs)=candidate.get_mut("pairs").and_then(Value::as_array_mut){
            for pair in pairs{
                if let Some(object)=pair.as_object_mut(){object.remove("expected_status");}
            }
        }
        if let Some(cuts)=candidate.get_mut("dependency_cut_scenarios").and_then(Value::as_array_mut){
            for cut in cuts{
                if let Some(object)=cut.as_object_mut(){object.remove("expected_statuses");}
            }
        }
        run_case(&candidate,&"a".repeat(40),&"b".repeat(64)).expect("mutated case evaluates")
    }
    fn status<'a>(v:&'a Value,id:&str)->&'a str{
        v["base_pair_results"].as_array().unwrap().iter().find(|p|p["pair_id"]==id).unwrap()["status"].as_str().unwrap()
    }
    #[test]
    fn frozen_fixture_expected_statuses_are_enforced(){
        let v=fixture();
        run_case(&v,&"a".repeat(40),&"b".repeat(64))
            .expect("unmodified fixture must match declared base and dependency-cut outcomes");
    }
    #[test]
    fn maritime_adapter_reuses_generic_pair_kernel(){
        let r=run(&fixture());
        assert_eq!(r["reused_kernel"]["git_revision"],KERNEL_REV);
        assert_eq!(r["pair_separability_computed_by_reused_kernel"],true);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"IDENTIFIABLE");
        assert_eq!(r["base_pair_results"][0]["observed_independent_principles"],3);
    }
    #[test]
    fn gateway_loss_recomputes_identifiability(){
        let r=run(&fixture());
        let cut=&r["dependency_cut_results"][0]["pair_results"][0];
        assert_eq!(cut["status"],"NOT_IDENTIFIABLE");
        assert_eq!(cut["observed_independent_principles"],1);
    }
    #[test]
    fn pressure_power_loss_preserves_two_other_principles(){
        let r=run(&fixture());
        let cut=&r["dependency_cut_results"][1]["pair_results"][0];
        assert_eq!(cut["status"],"IDENTIFIABLE");
        assert_eq!(cut["observed_independent_principles"],2);
    }
    #[test]
    fn de_duplicated_principle_groups_not_channel_count(){
        let mut v=fixture();
        v["channels"][2]["independence_group_ref"]=json!("pressure-principle");
        let r=run(&v);
        assert_eq!(r["base_pair_results"][0]["observed_independent_principles"],2);
    }
    #[test]
    fn expired_calibration_cannot_contribute(){
        // Calibration is meaningful for measured evidence. Use a measured
        // case so an expired certificate makes one independent principle
        // inadmissible; synthetic fixtures correctly use not_applicable.
        let mut v=measured_case();
        v["pairs"][0]["minimum_independent_principles"]=json!(3);
        v["channels"][2]["calibration_state"]=json!("expired");
        let r=run(&v);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"INSUFFICIENT_EVIDENCE");
    }
    #[test]
    fn derived_channel_cannot_contribute_as_independent_measurement(){
        let mut v=fixture();
        v["pairs"][0]["minimum_independent_principles"]=json!(3);
        v["channels"][3]["source_role"]=json!("derived");
        v["channels"][3]["derived_from_channel_ids"]=json!(["shaft-speed"]);
        v["pairs"][0]["responses"][3]["evidence_role_a"]=json!("DERIVED");
        v["pairs"][0]["responses"][3]["evidence_role_b"]=json!("DERIVED");
        let r=run(&v);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"INSUFFICIENT_EVIDENCE");
    }
    #[test]
    fn reconstructed_channel_cannot_contribute_as_independent_measurement(){
        let mut v=fixture();
        v["pairs"][0]["minimum_independent_principles"]=json!(3);
        v["channels"][3]["source_role"]=json!("reconstructed");
        v["channels"][3]["derived_from_channel_ids"]=json!(["shaft-speed"]);
        v["pairs"][0]["responses"][3]["evidence_role_a"]=json!("RECONSTRUCTED");
        v["pairs"][0]["responses"][3]["evidence_role_b"]=json!("RECONSTRUCTED");
        let r=run(&v);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"INSUFFICIENT_EVIDENCE");
    }
    #[test]
    fn overlapping_intervals_do_not_discriminate(){
        let mut v=fixture();
        for response in v["pairs"][0]["responses"].as_array_mut().unwrap(){
            response["interval_b"]=response["interval_a"].clone();
        }
        let r=run(&v);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"NOT_IDENTIFIABLE");
    }
    #[test]
    fn wrong_configuration_is_rejected_before_kernel_call(){
        let mut v=fixture();
        v["channels"][0]["configuration_sha256"]=json!("c".repeat(64));
        assert!(validate_case(&v).is_err());
    }
    #[test]
    fn unit_mismatch_is_rejected_before_kernel_call(){
        let mut v=fixture();
        v["pairs"][0]["responses"][0]["unit"]=json!("psi");
        assert!(validate_case(&v).is_err());
    }
    fn measured_case()->Value{
        let mut v=fixture();
        v["evidence_class"]=json!("MEASURED");
        for channel in v["channels"].as_array_mut().unwrap(){
            channel["source_role"]=json!("measured");
            channel["calibration_state"]=json!("current");
        }
        for response in v["pairs"][0]["responses"].as_array_mut().unwrap(){
            response["evidence_role_a"]=json!("INDEPENDENT_MEASUREMENT");
            response["evidence_role_b"]=json!("INDEPENDENT_MEASUREMENT");
        }
        v
    }
    #[test]
    fn measured_case_requires_shared_qualified_timebase(){
        let mut v=measured_case();
        v["channels"][2]["timebase_id"]=json!("second-clock");
        let r=run(&v);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"INSUFFICIENT_EVIDENCE");
    }
    #[test]
    fn measured_case_requires_qualified_regime(){
        let mut v=measured_case();
        v["operating_domain"]["qualification_state"]=json!("unsupported");
        let r=run(&v);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"INSUFFICIENT_EVIDENCE");
    }
    #[test]
    fn measured_case_enforces_timestamp_uncertainty_bound(){
        let mut v=measured_case();
        v["pairs"][0]["minimum_independent_principles"]=json!(3);
        v["channels"][2]["timestamp_uncertainty_ms"]=json!(8.0);
        let r=run(&v);
        assert_eq!(status(&r,"fuel-restriction-vs-injector-degradation"),"INSUFFICIENT_EVIDENCE");
    }
    #[test]
    fn same_fault_window_cannot_be_compared_to_itself(){
        let mut v=fixture();
        v["pairs"][0]["measurement_window_id_b"]=v["pairs"][0]["measurement_window_id_a"].clone();
        assert!(validate_case(&v).is_err());
    }
    #[test]
    fn authority_boundary_is_non_authoritative(){
        let r=run(&fixture());
        assert_eq!(r["mutation_performed"],false);
        assert_eq!(r["authorization_granted"],false);
        assert_eq!(r["physical_diagnosability_established"],false);
    }
}
