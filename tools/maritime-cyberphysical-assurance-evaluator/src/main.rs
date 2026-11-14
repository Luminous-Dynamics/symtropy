use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const FIXTURE: &str = include_str!("../../../benchmarks/maritime-cyberphysical-assurance/corpus-v0.1.json");
type Check<T> = Result<T,String>;

fn digest(bytes:&[u8])->String { format!("{:x}",Sha256::digest(bytes)) }
fn valid_sha(s:&str)->bool { s.len()==64 && s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)) }
fn object(v:&Value,at:&str)->Check<&serde_json::Map<String,Value>> {
    v.as_object().ok_or_else(||format!("{at}: expected object"))
}
fn exact(v:&Value,keys:&[&str],at:&str)->Check<()> {
    let m=object(v,at)?;
    let expected:BTreeSet<&str>=keys.iter().copied().collect();
    let actual:BTreeSet<&str>=m.keys().map(String::as_str).collect();
    let missing:Vec<_>=expected.difference(&actual).copied().collect();
    let extra:Vec<_>=actual.difference(&expected).copied().collect();
    if !missing.is_empty()||!extra.is_empty(){return Err(format!("{at}: missing {missing:?}, unexpected {extra:?}"));}
    Ok(())
}
fn field<'a>(v:&'a Value,k:&str,at:&str)->Check<&'a Value>{
    object(v,at)?.get(k).ok_or_else(||format!("{at}: missing {k}"))
}
fn txt<'a>(v:&'a Value,k:&str,at:&str)->Check<&'a str>{
    field(v,k,at)?.as_str().filter(|s|!s.trim().is_empty()).ok_or_else(||format!("{at}.{k}: non-empty string required"))
}
fn num(v:&Value,k:&str,at:&str)->Check<f64>{
    let x=field(v,k,at)?.as_f64().ok_or_else(||format!("{at}.{k}: finite number required"))?;
    if !x.is_finite(){return Err(format!("{at}.{k}: non-finite number"));}
    Ok(x)
}
fn uint(v:&Value,k:&str,at:&str)->Check<u64>{
    field(v,k,at)?.as_u64().ok_or_else(||format!("{at}.{k}: nonnegative integer required"))
}
fn boolv(v:&Value,k:&str,at:&str)->Check<bool>{
    field(v,k,at)?.as_bool().ok_or_else(||format!("{at}.{k}: boolean required"))
}
fn ids(v:&Value,k:&str,at:&str)->Check<BTreeSet<String>>{
    let a=field(v,k,at)?.as_array().ok_or_else(||format!("{at}.{k}: array required"))?;
    let mut out=BTreeSet::new();
    for item in a {
        let id=item.as_str().filter(|s|!s.trim().is_empty()).ok_or_else(||format!("{at}.{k}: non-empty string required"))?;
        if !out.insert(id.to_owned()){return Err(format!("{at}.{k}: duplicate ID {id}"));}
    }
    Ok(out)
}
fn bounds(t:u64,u:u64)->Option<(u64,u64)> { Some((t.checked_sub(u)?,t.checked_add(u)?)) }

#[derive(Clone)]
struct EventResult {
    case_id:String, class_id:String, label_status:String, false_unique:bool,
    missed_components:usize, false_positive_components:usize,
    sample_count:u64, episode_count:u64,
    delay_bounds_ms:Option<(u64,u64)>, ens:f64, critical_availability:f64, recovery:f64, reserve:f64,
}
fn taxonomy_map(corpus:&Value)->Check<BTreeMap<String,(BTreeSet<String>,BTreeSet<String>)>>{
    let list=field(corpus,"taxonomy","$")?.as_array().ok_or("taxonomy must be an array")?;
    if list.len()!=12{return Err(format!("taxonomy must contain 12 classes; got {}",list.len()));}
    let expected=[
        "S0_NORMAL_REFERENCE","S1_PROPULSION_LOAD_STEP","S2_G2_GENERATOR_TRIP",
        "S3_G1_GENERATOR_DERATING","S4_BESS_CAPACITY_DEGRADATION","S5_BESS_THERMAL_LIMITATION",
        "S6_CONVERTER_EFFICIENCY_DEGRADATION","S7_DC_LINK_VOLTAGE_SENSOR_BIAS",
        "S8_CURRENT_SENSOR_DRIFT","S9_PACKET_LOSS_DELAY",
        "S10_COMBINED_G1_DERATING_VOLTAGE_BIAS","S11_EMERGENCY_ISLANDED_OPERATION"
    ];
    let mut map=BTreeMap::new();
    for (i,item) in list.iter().enumerate(){
        let at=format!("$.taxonomy[{i}]");
        exact(item,&["class_id","event_kind","expected_physical_fault_ids","expected_information_fault_ids"],&at)?;
        let id=txt(item,"class_id",&at)?.to_owned();
        if id!=expected[i]{return Err(format!("{at}.class_id: expected {}, got {id}",expected[i]));}
        let physical=ids(item,"expected_physical_fault_ids",&at)?;
        let information=ids(item,"expected_information_fault_ids",&at)?;
        if physical.iter().any(|x|information.contains(x)){return Err(format!("{at}: fault ID appears in both evidence domains"));}
        if map.insert(id.clone(),(physical,information)).is_some(){return Err(format!("{at}: duplicate class {id}"));}
    }
    Ok(map)
}
fn evaluate_case(case:&Value,truth:&BTreeMap<String,(BTreeSet<String>,BTreeSet<String>)>,asset_sha:&str,timebase_id:&str,timebase_sha:&str,max_uncertainty:u64)->Check<EventResult>{
    let at="$.cases[]";
    exact(case,&["case_id","class_id","evidence_role","measurement_window","truth","diagnosis","timing","recommendation","consequences"],at)?;
    let case_id=txt(case,"case_id",at)?.to_owned();
    let class_id=txt(case,"class_id",at)?.to_owned();
    if txt(case,"evidence_role",at)?!="SYNTHETIC_KNOWN_TRUTH"{return Err(format!("{case_id}: evidence role not synthetic"));}
    let (expected_physical,expected_info)=truth.get(&class_id).ok_or_else(||format!("{case_id}: unknown taxonomy class {class_id}"))?;
    let measurement=field(case,"measurement_window",at)?;
    exact(measurement,&["asset_configuration_sha256","timebase_id","timebase_qualification_state","timebase_qualification_sha256","quality_state","sample_interval_ms","sample_count","independent_episode_count","episode_definition_id","episode_definition_sha256"],&format!("{at}.measurement_window"))?;
    if txt(measurement,"asset_configuration_sha256",at)?!=asset_sha
        ||txt(measurement,"timebase_id",at)?!=timebase_id
        ||txt(measurement,"timebase_qualification_state",at)?!="QUALIFIED"
        ||!valid_sha(txt(measurement,"timebase_qualification_sha256",at)?)
        ||txt(measurement,"quality_state",at)?!="CURRENT" {
        return Err(format!("{case_id}: configuration/timebase/measurement-quality admission failed"));
    }
    let sample_interval=uint(measurement,"sample_interval_ms",at)?;
    let sample_count=uint(measurement,"sample_count",at)?;
    let episode_count=uint(measurement,"independent_episode_count",at)?;
    if sample_interval==0||sample_count==0||episode_count!=1||episode_count>sample_count{
        return Err(format!("{case_id}: synthetic episode and sample counts invalid"));
    }
    if txt(measurement,"timebase_qualification_sha256",at)?!=timebase_sha
        ||!valid_sha(txt(measurement,"episode_definition_sha256",at)?)
        ||txt(measurement,"episode_definition_id",at)?.is_empty(){
        return Err(format!("{case_id}: timebase/episode-definition identity does not match corpus"));
    }
    let truth_obj=field(case,"truth",at)?;
    exact(truth_obj,&["physical_fault_ids","information_fault_ids"],&format!("{at}.truth"))?;
    let truth_p=ids(truth_obj,"physical_fault_ids",at)?;
    let truth_i=ids(truth_obj,"information_fault_ids",at)?;
    if &truth_p!=expected_physical||&truth_i!=expected_info{return Err(format!("{case_id}: case truth disagrees with frozen taxonomy"));}

    let diag=field(case,"diagnosis",at)?;
    exact(diag,&["reported_event_class_id","reported_physical_fault_ids","reported_information_fault_ids","disposition"],&format!("{at}.diagnosis"))?;
    let reported_class=txt(diag,"reported_event_class_id",at)?;
    let reported_p=ids(diag,"reported_physical_fault_ids",at)?;
    let reported_i=ids(diag,"reported_information_fault_ids",at)?;
    let expected_disposition=match expected_physical.len()+expected_info.len(){
        0=>"MODE_OR_DISTURBANCE_CLASS",1=>"SINGLE_FAULT",_=>"COMPOUND_FAULT_SET"
    };
    if txt(diag,"disposition",at)?!=expected_disposition{return Err(format!("{case_id}: diagnosis disposition inconsistent with truth-set cardinality"));}
    let truth_all:BTreeSet<String>=expected_physical.union(expected_info).cloned().collect();
    let reported_all:BTreeSet<String>=reported_p.union(&reported_i).cloned().collect();
    let false_unique=truth_all.len()>1&&reported_all.len()==1;
    let missed=truth_all.difference(&reported_all).count();
    let extra=reported_all.difference(&truth_all).count();
    let label_status=if reported_class!=class_id{"EVENT_CLASS_MISMATCH"}
        else if false_unique{"FALSE_UNIQUE_CAUSE"}
        else if missed>0{"MISSED_COMPONENT"}
        else if extra>0{"FALSE_POSITIVE_OR_MISATTRIBUTED"}
        else{"EXACT_SET_MATCH"};

    let timing=field(case,"timing",at)?;
    exact(timing,&["event_onset_ms","event_onset_uncertainty_ms","diagnostic_emitted_ms","diagnostic_timestamp_uncertainty_ms","algorithm_compute_time_ms","command_path_instrumented","sensor_to_command_latency_ms"],&format!("{at}.timing"))?;
    if num(timing,"algorithm_compute_time_ms",at)?<0.0{return Err(format!("{case_id}: algorithm compute time cannot be negative"));}
    let command_instrumented=boolv(timing,"command_path_instrumented",at)?;
    let latency=field(timing,"sensor_to_command_latency_ms",at)?;
    if !command_instrumented&&!latency.is_null(){return Err(format!("{case_id}: sensor-to-command latency claimed without instrumented path"));}
    if command_instrumented&&latency.as_f64().is_none_or(|v|!v.is_finite()||v<0.0){
        return Err(format!("{case_id}: instrumented command path requires a nonnegative latency"));
    }
    let onset_value=field(timing,"event_onset_ms",at)?;
    let delay_bounds=if onset_value.is_null(){
        if !reported_all.is_empty(){return Err(format!("{case_id}: no-onset baseline cannot report faults"));}
        if !field(timing,"event_onset_uncertainty_ms",at)?.is_null()
            ||!field(timing,"diagnostic_emitted_ms",at)?.is_null()
            ||!field(timing,"diagnostic_timestamp_uncertainty_ms",at)?.is_null(){
            return Err(format!("{case_id}: baseline must not claim detection timing"));
        }
        None
    }else{
        let onset=onset_value.as_u64().ok_or_else(||format!("{case_id}: onset timestamp must be a nonnegative integer"))?;
        let onset_u=field(timing,"event_onset_uncertainty_ms",at)?.as_u64().ok_or_else(||format!("{case_id}: onset uncertainty required"))?;
        let detected=uint(timing,"diagnostic_emitted_ms",at)?;
        let detected_u=uint(timing,"diagnostic_timestamp_uncertainty_ms",at)?;
        if onset_u>max_uncertainty||detected_u>max_uncertainty{return Err(format!("{case_id}: timestamp uncertainty exceeds bound"));}
        let (on_lo,on_hi)=bounds(onset,onset_u).ok_or_else(||format!("{case_id}: onset interval overflow/underflow"))?;
        let (de_lo,de_hi)=bounds(detected,detected_u).ok_or_else(||format!("{case_id}: diagnosis interval overflow/underflow"))?;
        if de_lo<=on_hi{return Err(format!("{case_id}: detection order not proven after onset"));}
        Some((de_lo-on_hi,de_hi-on_lo))
    };
    let recommendation=field(case,"recommendation",at)?;
    exact(recommendation,&["advisory_code","authority_scope"],&format!("{at}.recommendation"))?;
    if !matches!(txt(recommendation,"advisory_code",at)?,"MONITOR"|"HUMAN_REVIEW"|"REVIEW_EXISTING_RECORDS")
        ||txt(recommendation,"authority_scope",at)?!="ADVISORY_ONLY" {
        return Err(format!("{case_id}: advisory crosses authority boundary"));
    }
    let consequence=field(case,"consequences",at)?;
    exact(consequence,&["energy_not_supplied_kwh","critical_load_availability_percent","recovery_time_s","reserve_margin_kw"],&format!("{at}.consequences"))?;
    let ens=num(consequence,"energy_not_supplied_kwh",at)?;
    let critical=num(consequence,"critical_load_availability_percent",at)?;
    let recovery=num(consequence,"recovery_time_s",at)?;
    let reserve=num(consequence,"reserve_margin_kw",at)?;
    if ens<0.0||!(0.0..=100.0).contains(&critical)||recovery<0.0{return Err(format!("{case_id}: consequence values outside declared ranges"));}
    Ok(EventResult{
        case_id,class_id,label_status:label_status.to_owned(),false_unique,
        missed_components:missed,false_positive_components:extra,sample_count,episode_count,
        delay_bounds_ms:delay_bounds,
        ens,critical_availability:critical,recovery,reserve
    })
}
fn evaluate(corpus:&Value,source_commit:&str,input_sha:&str)->Check<Value>{
    if source_commit.len()!=40||!source_commit.bytes().all(|b|b.is_ascii_hexdigit())||!valid_sha(input_sha){return Err("invalid source commit or corpus digest".into());}
    exact(corpus,&["schema_version","corpus_id","evidence_class","asset","timebase","maximum_timestamp_uncertainty_ms","taxonomy","cases","authority_boundary","claim_ceiling"],"$")?;
    if txt(corpus,"schema_version","$")? != VERSION || txt(corpus,"evidence_class","$")? != "SYNTHETIC_KNOWN_TRUTH" {
        return Err("unsupported schema version or evidence class".into());
    }
    if txt(corpus,"corpus_id","$")? != "MARITIME-CYBER-PHYSICAL-ASSURANCE-SYNTHETIC-001" { return Err("unexpected corpus ID".into()); }
    let asset=field(corpus,"asset","$")?;
    exact(asset,&["asset_id","configuration_revision","configuration_sha256"],"$.asset")?;
    let asset_sha=txt(asset,"configuration_sha256","$.asset")?;
    if txt(asset,"asset_id","$.asset")?.is_empty()||txt(asset,"configuration_revision","$.asset")?.is_empty()||!valid_sha(asset_sha){
        return Err("invalid asset identity/configuration digest".into());
    }
    let clock=field(corpus,"timebase","$")?;
    exact(clock,&["timebase_id","qualification_state","qualification_id","qualification_sha256"],"$.timebase")?;
    let timebase=txt(clock,"timebase_id","$.timebase")?;
    let timebase_sha=txt(clock,"qualification_sha256","$.timebase")?;
    if txt(clock,"qualification_state","$.timebase")? != "QUALIFIED"
        || txt(clock,"qualification_id","$.timebase")?.is_empty()
        || !valid_sha(timebase_sha) {
        return Err("common timebase is not qualified".into());
    }
    let max_uncertainty=uint(corpus,"maximum_timestamp_uncertainty_ms","$")?;
    if max_uncertainty==0{return Err("timestamp uncertainty bound must be positive".into());}
    let taxonomy=taxonomy_map(corpus)?;
    let cases=field(corpus,"cases","$")?.as_array().ok_or("cases must be an array")?;
    if cases.len()!=12{return Err(format!("exactly 12 frozen event classes required; got {}",cases.len()));}
    let mut seen=BTreeSet::new();
    let mut results=Vec::new();
    for case in cases{
        let id=txt(case,"case_id","$.cases[]")?.to_owned();
        if !seen.insert(id.clone()){return Err(format!("duplicate case ID {id}"));}
        let result=evaluate_case(case,&taxonomy,asset_sha,timebase,timebase_sha,max_uncertainty)?;
        if result.label_status!="EXACT_SET_MATCH"{return Err(format!("{}: diagnosis set mismatch {}",result.case_id,result.label_status));}
        results.push(result);
    }
    let classes:BTreeSet<String>=results.iter().map(|r|r.class_id.clone()).collect();
    if classes.len()!=12||taxonomy.keys().any(|id|!classes.contains(id)){return Err("every taxonomy class must be represented exactly once".into());}
    let boundary=field(corpus,"authority_boundary","$")?;
    exact(boundary,&["read_only","measurement_initiated","controller_write","generator_command","bess_command","load_shedding","topology_switching","actuation","maintenance_authorization","mode_transition_authorization","restriction_clearance"],"$.authority_boundary")?;
    if !boolv(boundary,"read_only","$.authority_boundary")?{return Err("read_only must be true".into());}
    for key in ["measurement_initiated","controller_write","generator_command","bess_command","load_shedding","topology_switching","actuation","maintenance_authorization","mode_transition_authorization","restriction_clearance"]{
        if boolv(boundary,key,"$.authority_boundary")?{return Err(format!("authority boundary violation: {key}"));}
    }
    let count=results.len() as f64;
    let mean_ens=results.iter().map(|r|r.ens).sum::<f64>()/count;
    let mean_recovery=results.iter().map(|r|r.recovery).sum::<f64>()/count;
    let min_critical=results.iter().map(|r|r.critical_availability).fold(f64::INFINITY,f64::min);
    let mean_reserve=results.iter().map(|r|r.reserve).sum::<f64>()/count;
    let mut delays:Vec<u64>=results.iter().filter_map(|r|r.delay_bounds_ms.map(|(lo,hi)|(lo+hi)/2)).collect();
    delays.sort_unstable();
    let median_delay=if delays.is_empty(){Value::Null}else{json!(delays[delays.len()/2])};
    let case_receipts:Vec<Value>=results.iter().map(|r|json!({
        "case_id":r.case_id,"class_id":r.class_id,"diagnosis_status":r.label_status,
        "raw_sample_count":r.sample_count,"independent_episode_count":r.episode_count,
        "false_unique_cause":r.false_unique,"missed_components":r.missed_components,
        "false_positive_components":r.false_positive_components,
        "detection_delay_interval_ms":r.delay_bounds_ms.map(|(a,b)|json!([a,b])).unwrap_or(Value::Null),
        "energy_not_supplied_kwh":r.ens,"critical_load_availability_percent":r.critical_availability,
        "recovery_time_s":r.recovery,"reserve_margin_kw":r.reserve
    })).collect();
    Ok(json!({
        "result":"pass","evaluator_version":VERSION,"corpus_id":txt(corpus,"corpus_id","$")?,
        "source_commit":source_commit,"corpus_sha256":input_sha,
        "scenario_count":results.len(),"taxonomy_class_count":taxonomy.len(),
        "exact_diagnosis_set_match_count":results.len(),
        "raw_sample_count":results.iter().map(|r|r.sample_count).sum::<u64>(),
        "independent_episode_count":results.iter().map(|r|r.episode_count).sum::<u64>(),
        "false_unique_cause_count":results.iter().filter(|r|r.false_unique).count(),
        "missed_component_count":results.iter().map(|r|r.missed_components).sum::<usize>(),
        "false_positive_component_count":results.iter().map(|r|r.false_positive_components).sum::<usize>(),
        "median_detection_delay_ms":median_delay,
        "mean_energy_not_supplied_kwh":mean_ens,
        "minimum_critical_load_availability_percent":min_critical,
        "mean_recovery_time_s":mean_recovery,"mean_reserve_margin_kw":mean_reserve,
        "sensor_to_command_latency":"NOT_MEASURED",
        "algorithm_compute_time_is_not_end_to_end_latency":true,
        "source_artifact_payloads_verified":false,
        "case_results":case_receipts,"authority_boundary":boundary,
        "claim_ceiling":txt(corpus,"claim_ceiling","$")?
    }))
}
fn main()->ExitCode{
    let args:Vec<String>=env::args().collect();
    if args.len()!=6||args[2]!="--source-commit"||args[4]!="--corpus-sha256"{
        eprintln!("usage: maritime-cyberphysical-assurance-evaluator <corpus.json> --source-commit <40-hex> --corpus-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    let raw=match fs::read_to_string(&args[1]){Ok(v)=>v,Err(e)=>{eprintln!("read corpus: {e}");return ExitCode::FAILURE;}};
    let actual=digest(raw.as_bytes());
    if actual!=args[5]{eprintln!("corpus SHA-256 mismatch: expected {}, computed {actual}",args[5]);return ExitCode::FAILURE;}
    let corpus:Value=match serde_json::from_str(&raw){Ok(v)=>v,Err(e)=>{eprintln!("parse corpus: {e}");return ExitCode::FAILURE;}};
    match evaluate(&corpus,&args[3],&actual){
        Ok(receipt)=>{println!("{}",serde_json::to_string_pretty(&receipt).unwrap_or_else(|_|"{\"result\":\"fail\"}".into()));ExitCode::SUCCESS}
        Err(e)=>{eprintln!("cyber-physical assurance failed closed: {e}");ExitCode::FAILURE}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    fn corpus()->Value{serde_json::from_str(FIXTURE).expect("fixture parses")}
    fn eval(v:&Value)->Check<Value>{evaluate(v,&"a".repeat(40),&"b".repeat(64))}
    #[test]
    fn frozen_twelve_class_suite_has_exact_known_truth(){
        let r=eval(&corpus()).unwrap();
        assert_eq!(r["scenario_count"],12);assert_eq!(r["taxonomy_class_count"],12);
        assert_eq!(r["exact_diagnosis_set_match_count"],12);assert_eq!(r["false_unique_cause_count"],0);
        assert_eq!(r["missed_component_count"],0);
    }
    #[test]
    fn compound_physical_and_information_fault_is_not_collapsed(){
        let mut c=corpus();c["cases"][10]["diagnosis"]["reported_information_fault_ids"]=json!([]);
        assert!(eval(&c).unwrap_err().contains("FALSE_UNIQUE_CAUSE"));
    }
    #[test]
    fn missed_component_is_reported(){
        let mut c=corpus();c["cases"][3]["diagnosis"]["reported_physical_fault_ids"]=json!([]);
        assert!(eval(&c).unwrap_err().contains("MISSED_COMPONENT"));
    }
    #[test]
    fn command_path_latency_cannot_be_inferred(){
        let mut c=corpus();c["cases"][2]["timing"]["sensor_to_command_latency_ms"]=json!(2.0);
        assert!(eval(&c).unwrap_err().contains("sensor-to-command latency claimed without instrumented path"));
    }
    #[test]
    fn command_path_is_explicitly_not_measured(){
        let r=eval(&corpus()).unwrap();
        assert_eq!(r["sensor_to_command_latency"],"NOT_MEASURED");
        assert_eq!(r["algorithm_compute_time_is_not_end_to_end_latency"],true);
    }
    #[test]
    fn raw_samples_are_not_counted_as_independent_episodes(){
        let r=eval(&corpus()).unwrap();
        assert_eq!(r["raw_sample_count"],5400);
        assert_eq!(r["independent_episode_count"],12);
    }

    #[test]
    fn future_or_overlapping_diagnostic_time_is_rejected(){
        let mut c=corpus();c["cases"][2]["timing"]["diagnostic_emitted_ms"]=json!(c["cases"][2]["timing"]["event_onset_ms"]);
        assert!(eval(&c).unwrap_err().contains("detection order not proven"));
    }
    #[test]
    fn asset_configuration_mismatch_is_rejected(){
        let mut c=corpus();c["cases"][2]["measurement_window"]["asset_configuration_sha256"]=json!("d".repeat(64));
        assert!(eval(&c).unwrap_err().contains("configuration/timebase/measurement-quality admission failed"));
    }
    #[test]
    fn case_clock_certificate_must_match_corpus_certificate(){
        let mut c=corpus();
        c["cases"][2]["measurement_window"]["timebase_qualification_sha256"]=json!("d".repeat(64));
        assert!(eval(&c).unwrap_err().contains("timebase/episode-definition identity does not match corpus"));
    }
    #[test]
    fn authority_boundary_violation_is_rejected(){
        let mut c=corpus();c["authority_boundary"]["bess_command"]=json!(true);
        assert!(eval(&c).unwrap_err().contains("authority boundary violation"));
    }
    #[test]
    fn duplicate_scenario_identity_is_rejected(){
        let mut c=corpus();c["cases"][1]["case_id"]=c["cases"][0]["case_id"].clone();
        assert!(eval(&c).unwrap_err().contains("duplicate case ID"));
    }
}
