// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::{Path, PathBuf}};

const PROGRAMME: &str = "WATER-MIXING-SENSOR-OBSERVATION-CONTRACT-001";
const CASE_COUNT: usize = 11;
const EVIDENCE_CEILING: &str = "synthetic observation-provenance and fail-closed mass-boundary contract only";

fn field<'a>(value:&'a Value,key:&str)->Result<&'a Value,String>{
    value.get(key).ok_or_else(||format!("missing field: {key}"))
}
fn text<'a>(value:&'a Value,key:&str)->Result<&'a str,String>{
    field(value,key)?.as_str().ok_or_else(||format!("{key} must be text"))
}
fn usize_value(value:&Value,key:&str)->Result<usize,String>{
    field(value,key)?.as_u64().and_then(|v|usize::try_from(v).ok()).ok_or_else(||format!("{key} must be a non-negative integer"))
}
fn array<'a>(value:&'a Value,key:&str)->Result<&'a Vec<Value>,String>{
    field(value,key)?.as_array().ok_or_else(||format!("{key} must be an array"))
}
fn valid_sha(value:&str)->bool{
    value.len()==64 && value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))
}
fn base(state:&str,reason:&str)->Value{
    json!({
        "observation_contract_disposition":state,
        "reason":reason,
        "total_channel_count":null,
        "measured_channel_count":null,
        "inferred_channel_count":null,
        "missing_channel_count":null,
        "faulted_channel_count":null,
        "unresolved_calibration_count":null,
        "unresolved_clock_count":null,
        "incomplete_window_count":null,
        "inferred_channel_ids":[],
        "missing_channel_ids":[],
        "faulted_channel_ids":[],
        "invalid_provenance_channel_ids":[],
        "mass_closure_evidence_state":state,
        "mass_closure_evidence_permitted":false,
        "physical_mass_closure_claim_permitted":false,
        "imputation_counted_as_observed":false,
        "potability_claim_permitted":false,
        "release_authorized":false
    })
}
fn classify_channel(channel:&Value)->Result<(String,String),String>{
    let channel_id=text(channel,"channel_id")?.to_owned();
    let role=text(channel,"role")?;
    if role!="inlet"&&role!="outlet"{
        return Err(format!("unsupported channel role for {channel_id}"));
    }
    let measurement=text(channel,"measurement_state")?;
    let health=text(channel,"health_state")?;
    let calibration=text(channel,"calibration_state")?;
    let clock=text(channel,"clock_state")?;
    let window=text(channel,"window_state")?;
    let sample_count=usize_value(channel,"sample_count")?;
    if sample_count==0 || measurement=="missing"{
        return Ok((channel_id,"missing".to_owned()));
    }
    if matches!(measurement,"reconstructed"|"held_last"|"model_imputed"|"synthetic_fill"){
        return Ok((channel_id,"inferred".to_owned()));
    }
    if measurement!="measured_raw"&&measurement!="measured_calibrated"{
        return Ok((channel_id,"faulted".to_owned()));
    }
    let raw_digest=channel.get("raw_record_sha256").and_then(Value::as_str).unwrap_or("");
    let calibration_digest=channel.get("calibration_record_sha256").and_then(Value::as_str).unwrap_or("");
    let clock_digest=channel.get("clock_map_sha256").and_then(Value::as_str).unwrap_or("");
    if !valid_sha(raw_digest)||!valid_sha(calibration_digest)||!valid_sha(clock_digest){
        return Ok((channel_id,"invalid_provenance".to_owned()));
    }
    if matches!(health,"fault_suspected"|"fault_confirmed"|"stuck"|"spiky"|"drifting"|"unknown"){
        return Ok((channel_id,"faulted".to_owned()));
    }
    if health!="healthy"{
        return Ok((channel_id,"faulted".to_owned()));
    }
    if calibration!="current"{
        return Ok((channel_id,"calibration_unresolved".to_owned()));
    }
    if clock!="synchronized"{
        return Ok((channel_id,"clock_unresolved".to_owned()));
    }
    if window!="complete"{
        return Ok((channel_id,"window_incomplete".to_owned()));
    }
    Ok((channel_id,"measured".to_owned()))
}
fn compute(case:&Value)->Result<Value,String>{
    let input=text(case,"case_id")?;
    let boundary=field(case,"boundary")?;
    let inlet=array(boundary,"inlets")?;
    let outlet=array(boundary,"outlets")?;
    let expected_inlets=usize_value(boundary,"expected_inlet_count")?;
    let expected_outlets=usize_value(boundary,"expected_outlet_count")?;
    if expected_inlets==0||expected_outlets==0
        ||inlet.len()!=expected_inlets||outlet.len()!=expected_outlets
    {
        return Ok(base("PARTIAL_BOUNDARY","REQUIRED_INLET_OR_OUTLET_CHANNEL_RECORD_MISSING"));
    }
    let mut ids=BTreeSet::new();
    let mut inferred=Vec::new();
    let mut missing=Vec::new();
    let mut faulted=Vec::new();
    let mut invalid=Vec::new();
    let mut calibration=Vec::new();
    let mut clocks=Vec::new();
    let mut windows=Vec::new();
    let mut measured_count=0usize;
    for channel in inlet.iter().chain(outlet.iter()){
        let id=text(channel,"channel_id")?;
        if !ids.insert(id.to_owned()){
            return Ok(base("INVALID_CONTRACT","DUPLICATE_CHANNEL_ID"));
        }
        let (channel_id,state)=classify_channel(channel)?;
        match state.as_str(){
            "measured"=>measured_count+=1,
            "inferred"=>inferred.push(channel_id),
            "missing"=>missing.push(channel_id),
            "faulted"=>faulted.push(channel_id),
            "invalid_provenance"=>invalid.push(channel_id),
            "calibration_unresolved"=>calibration.push(channel_id),
            "clock_unresolved"=>clocks.push(channel_id),
            "window_incomplete"=>windows.push(channel_id),
            _=>return Err(format!("unknown internal channel state for {channel_id}")),
        }
    }
    let total=ids.len();
    let state=if !invalid.is_empty(){"PROVENANCE_INVALID"}
        else if !missing.is_empty(){"PARTIAL_BOUNDARY"}
        else if !inferred.is_empty(){"INFERRED_MEASUREMENTS_PRESENT"}
        else if !faulted.is_empty(){"SENSOR_FAULT_UNRESOLVED"}
        else if !calibration.is_empty(){"CALIBRATION_UNRESOLVED"}
        else if !clocks.is_empty(){"CLOCK_UNRESOLVED"}
        else if !windows.is_empty(){"TEMPORAL_WINDOW_INCOMPLETE"}
        else if measured_count==total{"COMPLETE_MEASURED_BOUNDARY"}
        else {"PARTIAL_BOUNDARY"};
    let reason=match state{
        "PROVENANCE_INVALID"=>"RAW_CALIBRATION_OR_CLOCK_DIGEST_INVALID",
        "PARTIAL_BOUNDARY"=>"ONE_OR_MORE_REQUIRED_INLET_OR_OUTLET_CHANNELS_MISSING",
        "INFERRED_MEASUREMENTS_PRESENT"=>"RECONSTRUCTED_OR_IMPUTED_VALUE_CANNOT_CLOSE_PHYSICAL_MASS",
        "SENSOR_FAULT_UNRESOLVED"=>"FAULTED_OR_UNHEALTHY_CHANNEL_CANNOT_CLOSE_PHYSICAL_MASS",
        "CALIBRATION_UNRESOLVED"=>"CALIBRATION_NOT_CURRENT",
        "CLOCK_UNRESOLVED"=>"CHANNEL_CLOCK_NOT_SYNCHRONIZED",
        "TEMPORAL_WINDOW_INCOMPLETE"=>"ONE_OR_MORE_CHANNEL_WINDOWS_OR_TAILS_INCOMPLETE",
        "COMPLETE_MEASURED_BOUNDARY"=>"ALL_REQUIRED_CHANNELS_MEASURED_CALIBRATED_AND_COMPLETE",
        _=>"UNCLASSIFIED_OBSERVATION_CONTRACT"
    };
    Ok(json!({
        "observation_contract_disposition":state,
        "reason":reason,
        "total_channel_count":total,
        "measured_channel_count":measured_count,
        "inferred_channel_count":inferred.len(),
        "missing_channel_count":missing.len(),
        "faulted_channel_count":faulted.len(),
        "unresolved_calibration_count":calibration.len(),
        "unresolved_clock_count":clocks.len(),
        "incomplete_window_count":windows.len(),
        "inferred_channel_ids":inferred,
        "missing_channel_ids":missing,
        "faulted_channel_ids":faulted,
        "invalid_provenance_channel_ids":invalid,
        "mass_closure_evidence_state":state,
        "mass_closure_evidence_permitted":state=="COMPLETE_MEASURED_BOUNDARY",
        "physical_mass_closure_claim_permitted":false,
        "imputation_counted_as_observed":false,
        "potability_claim_permitted":false,
        "release_authorized":false,
        "source_case_id":input
    }))
}
fn compare(a:&Value,b:&Value)->bool{
    match(a,b){
        (Value::Number(x),Value::Number(y))=>matches!((x.as_f64(),y.as_f64()),(Some(p),Some(q)) if (p-q).abs()<=1.0e-8*p.abs().max(q.abs()).max(1.0)),
        (Value::Array(x),Value::Array(y))=>x.len()==y.len()&&x.iter().zip(y).all(|(p,q)|compare(p,q)),
        (Value::Object(x),Value::Object(y))=>x.len()==y.len()&&x.iter().all(|(k,v)|y.get(k).is_some_and(|w|compare(v,w))),
        _=>a==b
    }
}
fn evaluate(input:&Value,oracle:&Value,manifest:&Value)->Result<Value,String>{
    if text(input,"schema_version")?!="sensor-observation-contract-t0-input-v0.1"
        ||text(oracle,"schema_version")?!="sensor-observation-contract-t0-oracle-v0.1"
        ||text(manifest,"schema_version")?!="sensor-observation-contract-t0-manifest-v0.1"{
        return Err("unexpected observation-contract schema".into());
    }
    for doc in [input,oracle,manifest]{
        if text(doc,"programme_id")?!=PROGRAMME{return Err("programme identity mismatch".into());}
    }
    for key in ["physical_mass_closure_claim_permitted","potability_claim_permitted","release_authorized"]{
        if field(oracle,key)?.as_bool()!=Some(false){return Err(format!("oracle cannot set {key} true"));}
    }
    let cases=array(input,"cases")?;
    let expected=array(oracle,"cases")?;
    let ids=array(manifest,"case_ids")?;
    if cases.len()!=CASE_COUNT||expected.len()!=CASE_COUNT||usize_value(manifest,"case_count")?!=CASE_COUNT{
        return Err("frozen observation-contract case count mismatch".into());
    }
    let collect=|rows:&[Value]|->Result<BTreeSet<String>,String>{
        rows.iter().map(|v|text(v,"case_id").map(str::to_owned)).collect()
    };
    let input_ids=collect(cases)?;
    let oracle_ids=collect(expected)?;
    let manifest_ids=ids.iter().map(|v|v.as_str().map(str::to_owned).ok_or_else(||"manifest case id must be text".to_owned())).collect::<Result<BTreeSet<_>,_>>()?;
    if input_ids.len()!=cases.len()||oracle_ids.len()!=expected.len()||manifest_ids.len()!=ids.len()||input_ids!=oracle_ids||input_ids!=manifest_ids{
        return Err("input/oracle/manifest case identity mismatch".into());
    }
    let mut rows=Vec::new();
    let mut mismatches=Vec::new();
    for case in cases{
        let id=text(case,"case_id")?;
        let actual=compute(case)?;
        let expected_row=expected.iter().find(|v|v["case_id"]==id).ok_or_else(||format!("oracle missing {id}"))?;
        let wanted=field(expected_row,"expected")?;
        let matched=compare(&actual,wanted);
        if !matched{mismatches.push(json!({"case_id":id,"expected":wanted,"actual":actual}));}
        rows.push(json!({"case_id":id,"matched":matched,"result":actual}));
    }
    rows.sort_by(|a,b|a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched=rows.iter().filter(|v|v["matched"]==true).count();
    Ok(json!({
        "schema_version":"sensor-observation-contract-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty(){"MatchedFrozenOracle"}else{"OracleMismatch"},
        "evidence_class":"synthetic_only",
        "evidence_ceiling":EVIDENCE_CEILING,
        "case_count":rows.len(),
        "matched_case_count":matched,
        "mismatches":mismatches,
        "results":rows,
        "physical_mass_closure_claim_permitted":false,
        "imputation_counted_as_observed":false,
        "potability_claim_permitted":false,
        "release_authorized":false,
        "nonclaims":[
            "COMPLETE_MEASURED_BOUNDARY only means the record contract is structurally complete; it is not physical mass closure or operator qualification.",
            "The T0 does not detect sensor faults; it enforces disposition after a fault state has been supplied.",
            "Inferred or reconstructed samples may support a separate diagnostic model but are never counted as observed mass.",
            "No output establishes drinking-water safety, potability, contamination, toxicology, standards compliance or release authority."
        ]
    }))
}
fn digest(bytes:&[u8])->String{
    Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()
}
fn read_json(path:&Path)->Result<(Vec<u8>,Value),String>{
    let bytes=fs::read(path).map_err(|e|format!("read {}: {e}",path.display()))?;
    let value=serde_json::from_slice(&bytes).map_err(|e|format!("parse {}: {e}",path.display()))?;
    Ok((bytes,value))
}
fn run()->Result<i32,String>{
    let args=env::args().skip(1).collect::<Vec<_>>();
    if args.len()>4{return Err("usage: sensor-observation-contract-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into());}
    let input_path=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/sensor-observation-contract-t0-cases-v0.1.json"));
    let oracle_path=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/sensor-observation-contract-t0-oracle-v0.1.json"));
    let manifest_path=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/sensor-observation-contract-t0-manifest-v0.1.json"));
    let receipt_path=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-mixing-sensor-observation-contract-t0/receipt.json"));
    let (ib,iv)=read_json(&input_path)?;let (ob,ov)=read_json(&oracle_path)?;let (mb,mv)=read_json(&manifest_path)?;
    let evaluation=evaluate(&iv,&ov,&mv)?;
    let success=evaluation["verification_state"]=="MatchedFrozenOracle";
    let revision=env::var("WATER_MIXING_SENSOR_OBSERVATION_CONTRACT_T0_REVISION").or_else(|_|env::var("GITHUB_SHA")).ok().filter(|v|v.len()==40&&v.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)));
    let lock_path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock=fs::read(&lock_path).map_err(|e|format!("read {}: {e}",lock_path.display()))?;
    let receipt=json!({
        "schema_version":"sensor-observation-contract-t0-run-v0.1",
        "verification_state":if !success{"OracleMismatch"}else if revision.is_some(){"PassedExactRevision"}else{"PassedLocalUnpinned"},
        "implementation_revision":revision,
        "input_sha256":digest(&ib),"oracle_sha256":digest(&ob),"manifest_sha256":digest(&mb),"cargo_lock_sha256":digest(&lock),
        "case_count":evaluation["case_count"],"matched_case_count":evaluation["matched_case_count"],
        "mismatches":evaluation["mismatches"],"results":evaluation["results"],
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "physical_mass_closure_claim_permitted":false,"imputation_counted_as_observed":false,
        "potability_claim_permitted":false,"release_authorized":false
    });
    if let Some(parent)=receipt_path.parent()&& !parent.as_os_str().is_empty(){
        fs::create_dir_all(parent).map_err(|e|format!("create {}: {e}",parent.display()))?;
    }
    let bytes=serde_json::to_string_pretty(&receipt).map_err(|e|format!("serialize receipt: {e}"))?+"\n";
    fs::write(&receipt_path,bytes.as_bytes()).map_err(|e|format!("write {}: {e}",receipt_path.display()))?;
    println!("{bytes}");
    Ok(if success{0}else{2})
}
fn main(){
    match run(){Ok(code)=>std::process::exit(code),Err(error)=>{eprintln!("WATER_MIXING_OBSERVATION_CONTRACT_T0_ERROR: {error}");std::process::exit(1);}}
}
#[cfg(test)]
mod tests{
    use super::*;
    fn load(name:&str)->Value{
        let raw=match name{
            "input"=>include_str!("../../../../research/water-intrusion-coupling/sensor-observation-contract-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/sensor-observation-contract-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/sensor-observation-contract-t0-manifest-v0.1.json"),
            _=>panic!("unknown observation-contract fixture")
        };
        serde_json::from_str(raw).expect("observation-contract fixture parses")
    }
    fn result()->Value{evaluate(&load("input"),&load("oracle"),&load("manifest")).expect("observation contract evaluates")}
    fn row(id:&str)->Value{result()["results"].as_array().unwrap().iter().find(|r|r["case_id"]==id).unwrap()["result"].clone()}
    #[test]fn all_eleven_cases_match_the_frozen_oracle(){let r=result();assert_eq!(r["case_count"],CASE_COUNT);assert_eq!(r["matched_case_count"],CASE_COUNT);assert_eq!(r["verification_state"],"MatchedFrozenOracle");}
    #[test]fn complete_measured_boundary_is_only_arithmetic_ready(){let r=row("SOC-T0-001-complete-measured");assert_eq!(r["mass_closure_evidence_state"],"COMPLETE_MEASURED_BOUNDARY");assert_eq!(r["physical_mass_closure_claim_permitted"],false);}
    #[test]fn reconstructed_outlet_is_never_counted_as_observed(){let r=row("SOC-T0-002-reconstructed-outlet");assert_eq!(r["observation_contract_disposition"],"INFERRED_MEASUREMENTS_PRESENT");assert_eq!(r["imputation_counted_as_observed"],false);assert_eq!(r["mass_closure_evidence_permitted"],false);}
    #[test]fn missing_outlet_refuses_complete_boundary(){assert_eq!(row("SOC-T0-003-missing-outlet")["mass_closure_evidence_state"],"PARTIAL_BOUNDARY");}
    #[test]fn confirmed_fault_refuses_mass_closure(){assert_eq!(row("SOC-T0-004-confirmed-fault")["mass_closure_evidence_state"],"SENSOR_FAULT_UNRESOLVED");}
    #[test]fn suspected_drift_refuses_mass_closure(){assert_eq!(row("SOC-T0-005-suspected-drift")["mass_closure_evidence_state"],"SENSOR_FAULT_UNRESOLVED");}
    #[test]fn expired_calibration_refuses_mass_closure(){assert_eq!(row("SOC-T0-006-expired-calibration")["mass_closure_evidence_state"],"CALIBRATION_UNRESOLVED");}
    #[test]fn clock_uncertainty_refuses_delay_evidence(){assert_eq!(row("SOC-T0-007-clock-uncertain")["mass_closure_evidence_state"],"CLOCK_UNRESOLVED");}
    #[test]fn truncated_tail_refuses_complete_boundary(){assert_eq!(row("SOC-T0-008-truncated-tail")["mass_closure_evidence_state"],"TEMPORAL_WINDOW_INCOMPLETE");}
    #[test]fn stuck_sensor_refuses_mass_closure(){assert_eq!(row("SOC-T0-009-stuck-sensor")["mass_closure_evidence_state"],"SENSOR_FAULT_UNRESOLVED");}
    #[test]fn invalid_raw_provenance_digest_refuses_mass_closure(){assert_eq!(row("SOC-T0-010-invalid-digest")["mass_closure_evidence_state"],"PROVENANCE_INVALID");}
    #[test]
    fn missing_provenance_digest_is_classified_not_silently_accepted() {
        let mut input=load("input");
        input["cases"][0]["boundary"]["outlets"][0].as_object_mut().unwrap().remove("raw_record_sha256");
        let result=compute(&input["cases"][0]).unwrap();
        assert_eq!(result["mass_closure_evidence_state"],"PROVENANCE_INVALID");
    }

    #[test]
    fn invalid_provenance_precedes_a_simultaneous_health_fault() {
        let mut input=load("input");
        input["cases"][0]["boundary"]["outlets"][0]["health_state"]=json!("fault_confirmed");
        input["cases"][0]["boundary"]["outlets"][0]["raw_record_sha256"]=json!("invalid");
        let result=compute(&input["cases"][0]).unwrap();
        assert_eq!(result["mass_closure_evidence_state"],"PROVENANCE_INVALID");
    }
    #[test]fn all_sensor_reconstruction_never_qualifies_boundary(){let r=row("SOC-T0-011-all-inferred");assert_eq!(r["mass_closure_evidence_state"],"INFERRED_MEASUREMENTS_PRESENT");assert_eq!(r["imputation_counted_as_observed"],false);}
}
