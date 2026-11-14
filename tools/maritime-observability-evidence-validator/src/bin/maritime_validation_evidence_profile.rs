//! Machine-checkable industrial validation evidence profile.
#![forbid(unsafe_code)]
use maritime_observability_evidence_validator::verify_sha256;
use serde_json::{json, Value};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
type Errors = Vec<Value>;

fn text<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}
fn boolean(v: &Value, k: &str) -> Option<bool> { v.get(k).and_then(Value::as_bool) }
fn sha256(s: &str) -> bool { s.len() == 64 && s.bytes().all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b)) }
fn push(e: &mut Errors, c: &str, p: &str, d: impl Into<String>) { e.push(json!({"code":c,"path":p,"detail":d.into()})); }
fn exact(v: &Value, req: &[&str], path: &str, e: &mut Errors) {
    let Some(o) = v.as_object() else { push(e,"EXPECTED_OBJECT",path,"expected object"); return; };
    let allowed: BTreeSet<&str> = req.iter().copied().collect();
    for k in req { if !o.contains_key(*k) { push(e,"MISSING_KEY",&format!("{path}.{k}"),"required field missing"); } }
    for k in o.keys() { if !allowed.contains(k.as_str()) { push(e,"UNKNOWN_KEY",&format!("{path}.{k}"),"unknown field rejected"); } }
}
fn arank(s: &str) -> Option<u8> { match s { "A0"=>Some(0),"A1"=>Some(1),"A2"=>Some(2),"A3"=>Some(3),"A4"=>Some(4),_=>None } }
fn brank(s: &str) -> Option<u8> { match s { "B0"=>Some(0),"B1"=>Some(1),"B2"=>Some(2),"B3"=>Some(3),"B4"=>Some(4),_=>None } }
fn aname(n: u8) -> &'static str { match n {0=>"A0",1=>"A1",2=>"A2",3=>"A3",_=>"A4"} }
fn bname(n: u8) -> &'static str { match n {0=>"B0",1=>"B1",2=>"B2",3=>"B3",_=>"B4"} }
fn ids(units: &[Value], part: &str, key: &str) -> BTreeSet<String> {
    units.iter().filter(|u|text(u,"partition")==Some(part)).filter_map(|u|text(u,key).map(str::to_owned)).collect()
}
fn all_ids(units: &[Value], key: &str) -> BTreeSet<String> {
    units.iter().filter_map(|u|text(u,key).map(str::to_owned)).collect()
}
fn disjoint(a: &BTreeSet<String>, b: &BTreeSet<String>) -> bool { a.is_disjoint(b) }
fn unseen(a: &BTreeSet<String>, b: &BTreeSet<String>) -> bool { a.iter().any(|x|!b.contains(x)) }
fn physical_ceiling(u: &[Value], unit: &str) -> Result<u8,String> {
    let dr=ids(u,"DEVELOPMENT","recording_id"); let er=ids(u,"EVALUATION","recording_id");
    let dc=ids(u,"DEVELOPMENT","component_id"); let ec=ids(u,"EVALUATION","component_id");
    let dm=ids(u,"DEVELOPMENT","machine_id"); let em=ids(u,"EVALUATION","machine_id");
    let ds=ids(u,"DEVELOPMENT","site_id"); let es=ids(u,"EVALUATION","site_id");
    match unit {
        "window" => Ok(0),
        "recording" if !dr.is_empty() && !er.is_empty() && disjoint(&dr,&er)
            && all_ids(u,"component_id").len()==1 && all_ids(u,"machine_id").len()==1 && all_ids(u,"site_id").len()==1 => Ok(1),
        "component" if !dc.is_empty() && !ec.is_empty() && disjoint(&dc,&ec)
            && all_ids(u,"machine_id").len()==1 && all_ids(u,"site_id").len()==1 => Ok(2),
        "machine" if !dm.is_empty() && !em.is_empty() && disjoint(&dm,&em)
            && all_ids(u,"site_id").len()==1 => Ok(3),
        "site" if !ds.is_empty() && !es.is_empty() && disjoint(&ds,&es) => Ok(4),
        "recording"|"component"|"machine"|"site" => Err("development/evaluation identities do not support the declared physical holdout".into()),
        _ => Err("partition_unit must be window, recording, component, machine or site".into())
    }
}
fn domain_ceiling(u: &[Value]) -> u8 {
    let site=unseen(&ids(u,"EVALUATION","site_id"),&ids(u,"DEVELOPMENT","site_id"));
    let machine=unseen(&ids(u,"EVALUATION","machine_id"),&ids(u,"DEVELOPMENT","machine_id"));
    let sensor=unseen(&ids(u,"EVALUATION","sensor_configuration_sha256"),&ids(u,"DEVELOPMENT","sensor_configuration_sha256"));
    let rig=unseen(&ids(u,"EVALUATION","test_rig_id"),&ids(u,"DEVELOPMENT","test_rig_id"));
    let condition=unseen(&ids(u,"EVALUATION","operating_condition_sha256"),&ids(u,"DEVELOPMENT","operating_condition_sha256"));
    if site {4} else if machine {3} else if sensor || rig {2} else if condition {1} else {0}
}
fn leakage(a: &Value, unit: &str) -> Result<&'static str,String> {
    let grouping=boolean(a,"grouping_before_segmentation").ok_or("grouping_before_segmentation must be boolean")?;
    let overlap=boolean(a,"overlapping_samples_across_partitions").ok_or("overlapping_samples_across_partitions must be boolean")?;
    let thresholds=boolean(a,"evaluation_used_for_thresholds").ok_or("evaluation_used_for_thresholds must be boolean")?;
    let prep=text(a,"preprocessing_fit_scope").ok_or("preprocessing_fit_scope required")?;
    let selection=text(a,"model_selection_data").ok_or("model_selection_data required")?;
    let concerns=a["documented_concerns"].as_array().ok_or("documented_concerns must be array")?;
    if overlap || !grouping || prep=="ALL_DATA" || selection=="TEST_USED" || thresholds { return Ok("HIGH"); }
    if prep=="UNKNOWN" || selection=="UNKNOWN" { return Ok("UNCLEAR"); }
    if !concerns.is_empty() || prep=="TRAIN_PLUS_VALIDATION" { return Ok("SOME_CONCERNS"); }
    if unit!="window" && prep=="TRAIN_ONLY" && matches!(selection,"TRAIN_ONLY"|"VALIDATION_ONLY") { return Ok("LOW"); }
    Ok("UNCLEAR")
}
fn validate_profile(p: &Value, i: usize) -> Result<Value,Errors> {
    let mut e=Vec::new(); let path=format!("$.profiles[{i}]");
    exact(p,&["profile_id","physical_independence","domain_shift","target_domain_access","unit_records","leakage_audit","claim_ceiling"],&path,&mut e);
    if text(p,"profile_id").is_none(){push(&mut e,"PROFILE_ID_REQUIRED",&format!("{path}.profile_id"),"profile ID required");}
    let phy=&p["physical_independence"]; exact(phy,&["declared_level","partition_unit"],&format!("{path}.physical_independence"),&mut e);
    let da=text(phy,"declared_level").unwrap_or(""); let unit=text(phy,"partition_unit").unwrap_or(""); let dar=arank(da);
    if dar.is_none(){push(&mut e,"PHYSICAL_LEVEL_INVALID",&format!("{path}.physical_independence.declared_level"),"expected A0..A4");}
    let dom=&p["domain_shift"]; exact(dom,&["declared_level"],&format!("{path}.domain_shift"),&mut e);
    let db=text(dom,"declared_level").unwrap_or(""); let dbr=brank(db);
    if dbr.is_none(){push(&mut e,"DOMAIN_LEVEL_INVALID",&format!("{path}.domain_shift.declared_level"),"expected B0..B4");}
    if !matches!(text(p,"target_domain_access"),Some("NONE"|"UNLABELED_ADAPTATION"|"LABELED_ADAPTATION"|"SELF_SUPERVISED_PRETRAINING"|"UNCLEAR")) {
        push(&mut e,"TARGET_ACCESS_INVALID",&format!("{path}.target_domain_access"),"invalid target-domain access state");
    }
    let Some(units)=p["unit_records"].as_array() else { push(&mut e,"UNIT_RECORDS_REQUIRED",&format!("{path}.unit_records"),"unit_records array required"); return Err(e); };
    if units.len()<2 || units.len()>10000 { push(&mut e,"UNIT_RECORD_COUNT_INVALID",&format!("{path}.unit_records"),"unit records must be 2..=10000"); }
    let mut seen=BTreeSet::new();
    for (j,u) in units.iter().enumerate() {
        let up=format!("{path}.unit_records[{j}]");
        exact(u,&["unit_id","partition","recording_id","component_id","machine_id","site_id","operating_condition_sha256","sensor_configuration_sha256","test_rig_id","artifact_sha256"],&up,&mut e);
        for k in ["unit_id","recording_id","component_id","machine_id","site_id","test_rig_id"] {
            if text(u,k).is_none(){push(&mut e,"UNIT_IDENTITY_REQUIRED",&format!("{up}.{k}"),"non-empty identity required");}
        }
        if !matches!(text(u,"partition"),Some("DEVELOPMENT"|"EVALUATION")){push(&mut e,"PARTITION_INVALID",&format!("{up}.partition"),"partition must be DEVELOPMENT or EVALUATION");}
        if !seen.insert(text(u,"unit_id").unwrap_or("").to_owned()){push(&mut e,"DUPLICATE_UNIT_ID",&format!("{up}.unit_id"),"unit IDs must be unique");}
        for k in ["operating_condition_sha256","sensor_configuration_sha256","artifact_sha256"] {
            if !u.get(k).and_then(Value::as_str).is_some_and(sha256){push(&mut e,"UNIT_DIGEST_INVALID",&format!("{up}.{k}"),"expected lowercase SHA-256");}
        }
        let identity=match unit {"window"=>"unit_id","recording"=>"recording_id","component"=>"component_id","machine"=>"machine_id","site"=>"site_id",_=>"unit_id"};
        if text(u,"unit_id").is_some() && text(u,identity).is_some() && text(u,"unit_id")!=text(u,identity) {
            push(&mut e,"PARTITION_UNIT_IDENTITY_MISMATCH",&format!("{up}.unit_id"),"unit ID must equal partition identity");
        }
    }
    if ids(units,"DEVELOPMENT","unit_id").is_empty() || ids(units,"EVALUATION","unit_id").is_empty() {
        push(&mut e,"BOTH_PARTITIONS_REQUIRED",&format!("{path}.unit_records"),"at least one development and one evaluation unit are required");
    }
    let audit=&p["leakage_audit"];
    exact(audit,&["grouping_before_segmentation","overlapping_samples_across_partitions","preprocessing_fit_scope","model_selection_data","evaluation_used_for_thresholds","documented_concerns","split_manifest_sha256"],&format!("{path}.leakage_audit"),&mut e);
    if !matches!(text(audit,"preprocessing_fit_scope"),Some("TRAIN_ONLY"|"TRAIN_PLUS_VALIDATION"|"ALL_DATA"|"UNKNOWN")) {
        push(&mut e,"PREPROCESSING_SCOPE_INVALID",&format!("{path}.leakage_audit.preprocessing_fit_scope"),"invalid preprocessing scope");
    }
    if !matches!(text(audit,"model_selection_data"),Some("TRAIN_ONLY"|"VALIDATION_ONLY"|"TEST_USED"|"UNKNOWN")) {
        push(&mut e,"MODEL_SELECTION_SCOPE_INVALID",&format!("{path}.leakage_audit.model_selection_data"),"invalid model-selection scope");
    }
    if audit["documented_concerns"].as_array().is_none(){push(&mut e,"DOCUMENTED_CONCERNS_INVALID",&format!("{path}.leakage_audit.documented_concerns"),"expected array");}
    if !audit.get("split_manifest_sha256").and_then(Value::as_str).is_some_and(sha256){push(&mut e,"SPLIT_MANIFEST_DIGEST_INVALID",&format!("{path}.leakage_audit.split_manifest_sha256"),"expected SHA-256");}
    if text(p,"claim_ceiling").is_none(){push(&mut e,"CLAIM_CEILING_REQUIRED",&format!("{path}.claim_ceiling"),"claim ceiling required");}
    if !e.is_empty(){return Err(e);}
    let ca=physical_ceiling(units,unit).map_err(|d|vec![json!({"code":"PHYSICAL_INDEPENDENCE_NOT_SUPPORTED","path":format!("{path}.physical_independence"),"detail":d})])?;
    let cb=domain_ceiling(units);
    if dar.is_some_and(|n|n>ca){push(&mut e,"PHYSICAL_INDEPENDENCE_OVERCLAIM",&format!("{path}.physical_independence.declared_level"),format!("declared {da} exceeds structural ceiling {}",aname(ca)));}
    if dbr.is_some_and(|n|n>cb){push(&mut e,"DOMAIN_SHIFT_OVERCLAIM",&format!("{path}.domain_shift.declared_level"),format!("declared {db} exceeds structural ceiling {}",bname(cb)));}
    if !e.is_empty(){return Err(e);}
    let leak=leakage(audit,unit).map_err(|d|vec![json!({"code":"LEAKAGE_AUDIT_INVALID","path":format!("{path}.leakage_audit"),"detail":d})])?;
    Ok(json!({"profile_id":text(p,"profile_id").unwrap_or(""),"declared_physical_independence_level":da,"computed_physical_independence_ceiling":aname(ca),"declared_domain_shift_level":db,"computed_domain_shift_ceiling":bname(cb),"target_domain_access":text(p,"target_domain_access").unwrap_or(""),"leakage_risk":leak,"physical_evidence_verified":false,"source_artifact_payloads_verified":false,"authority_granted":false,"claim_ceiling":text(p,"claim_ceiling").unwrap_or("")}))
}
fn evaluate(v:&Value)->Result<Value,Errors>{
    let mut e=Vec::new(); exact(v,&["schema_version","corpus_id","evidence_class","profiles","authority_boundary","claim_ceiling"],"$",&mut e);
    if text(v,"schema_version")!=Some(VERSION){push(&mut e,"SCHEMA_VERSION","$.schema_version","expected 0.1.0");}
    if text(v,"evidence_class")!=Some("SYNTHETIC_KNOWN_TRUTH"){push(&mut e,"EVIDENCE_CLASS","$.evidence_class","synthetic known-truth fixture only");}
    let boundary=&v["authority_boundary"]; exact(boundary,&["read_only","mutation_performed","authorization_granted"],"$.authority_boundary",&mut e);
    if boolean(boundary,"read_only")!=Some(true)||boolean(boundary,"mutation_performed")!=Some(false)||boolean(boundary,"authorization_granted")!=Some(false){push(&mut e,"AUTHORITY_BOUNDARY_VIOLATION","$.authority_boundary","read-only true; mutation and authorization false required");}
    if text(v,"corpus_id").is_none()||text(v,"claim_ceiling").is_none(){push(&mut e,"CORPUS_IDENTITY_REQUIRED","$","corpus ID and claim ceiling required");}
    let Some(items)=v["profiles"].as_array()else{push(&mut e,"PROFILES_REQUIRED","$.profiles","profiles array required");return Err(e);};
    if items.is_empty()||items.len()>1000{push(&mut e,"PROFILE_COUNT_INVALID","$.profiles","profile count must be 1..=1000");}
    if !e.is_empty(){return Err(e);}
    let mut seen=BTreeSet::new(); let mut results=Vec::new();
    for (i,p) in items.iter().enumerate(){let id=text(p,"profile_id").unwrap_or("").to_owned();if !seen.insert(id){return Err(vec![json!({"code":"DUPLICATE_PROFILE_ID","path":format!("$.profiles[{i}].profile_id"),"detail":"profile IDs must be unique"})]);}results.push(validate_profile(p,i)?);}
    let count=|level:&str|results.iter().filter(|p|p["leakage_risk"].as_str()==Some(level)).count();
    Ok(json!({"result":"pass","validator_version":VERSION,"corpus_id":text(v,"corpus_id").unwrap_or(""),"profile_count":results.len(),"physical_evidence_verified":false,"source_artifact_payloads_verified":false,"leakage_risk_counts":{"LOW":count("LOW"),"SOME_CONCERNS":count("SOME_CONCERNS"),"HIGH":count("HIGH"),"UNCLEAR":count("UNCLEAR")},"profiles":results,"claim_ceiling":text(v,"claim_ceiling").unwrap_or("")}))
}
fn run(v:&Value,source:&str,digest:&str)->Result<Value,Errors>{
    if source.len()!=40||!source.bytes().all(|b|b.is_ascii_hexdigit())||!sha256(digest){return Err(vec![json!({"code":"INVOCATION_DIGEST_INVALID","path":"$","detail":"invalid source commit or corpus SHA-256"})]);}
    let mut r=evaluate(v)?;r["source_commit"]=json!(source);r["corpus_sha256"]=json!(digest);Ok(r)
}
fn main()->ExitCode{
    let a:Vec<String>=env::args().collect();
    if a.len()!=6||a[2]!="--source-commit"||a[4]!="--profile-sha256"{eprintln!("usage: maritime_validation_evidence_profile <corpus.json> --source-commit <40-hex> --profile-sha256 <64-hex>");return ExitCode::from(2);}
    let raw=match fs::read_to_string(&a[1]){Ok(v)=>v,Err(e)=>{eprintln!("read corpus: {e}");return ExitCode::FAILURE;}};
    if let Err(e)=verify_sha256(raw.as_bytes(),&a[5]){eprintln!("profile digest verification failed: {e}");return ExitCode::FAILURE;}
    let v:Value=match serde_json::from_str(&raw){Ok(v)=>v,Err(e)=>{eprintln!("parse corpus: {e}");return ExitCode::FAILURE;}};
    match run(&v,&a[3],&a[5]){Ok(r)=>{println!("{}",serde_json::to_string_pretty(&r).unwrap_or_else(|_|"{\"result\":\"fail\"}".into()));ExitCode::SUCCESS},Err(e)=>{eprintln!("profile rejected: {}",serde_json::to_string(&e).unwrap_or_else(|_|"invalid".into()));ExitCode::FAILURE}}
}
#[cfg(test)]
mod tests{
    use super::*;
    fn fixture()->Value{serde_json::from_str(include_str!("../../../../benchmarks/maritime-observability/validation-evidence-profiles.v0.1.json")).expect("fixture")}
    fn result()->Value{evaluate(&fixture()).expect("profiles evaluate")}
    fn item<'a>(r:&'a Value,id:&str)->&'a Value{r["profiles"].as_array().unwrap().iter().find(|p|p["profile_id"]==id).unwrap()}
    #[test]fn six_profiles_reconcile_a_b_and_leakage(){let r=result();assert_eq!(r["profile_count"],6);assert_eq!(item(&r,"A0-window-random-split")["computed_physical_independence_ceiling"],"A0");assert_eq!(item(&r,"A0-window-random-split")["computed_domain_shift_ceiling"],"B0");assert_eq!(item(&r,"A0-window-random-split")["leakage_risk"],"HIGH");assert_eq!(item(&r,"A1-recording-holdout-operating-shift")["computed_physical_independence_ceiling"],"A1");assert_eq!(item(&r,"A1-recording-holdout-operating-shift")["computed_domain_shift_ceiling"],"B1");assert_eq!(item(&r,"A2-component-holdout-sensor-shift-adapted")["computed_physical_independence_ceiling"],"A2");assert_eq!(item(&r,"A2-component-holdout-sensor-shift-adapted")["computed_domain_shift_ceiling"],"B2");assert_eq!(item(&r,"A3-machine-holdout-with-concern")["computed_physical_independence_ceiling"],"A3");assert_eq!(item(&r,"A3-machine-holdout-with-concern")["computed_domain_shift_ceiling"],"B3");assert_eq!(item(&r,"A4-site-holdout")["computed_physical_independence_ceiling"],"A4");assert_eq!(item(&r,"A4-site-holdout")["computed_domain_shift_ceiling"],"B4");assert_eq!(item(&r,"A1-incomplete-reporting")["leakage_risk"],"UNCLEAR");}
    #[test]
    fn unlabeled_adaptation_is_not_automatically_leakage() {
        let evaluated = result();
        let p = item(&evaluated, "A2-component-holdout-sensor-shift-adapted");
        assert_eq!(p["target_domain_access"], "UNLABELED_ADAPTATION");
        assert_eq!(p["leakage_risk"], "LOW");
    }
    #[test]fn documented_concern_is_some_concerns(){assert_eq!(item(&result(),"A3-machine-holdout-with-concern")["leakage_risk"],"SOME_CONCERNS");}
    #[test]fn recording_holdout_cannot_claim_site_independence(){let mut v=fixture();v["profiles"][1]["physical_independence"]["declared_level"]=json!("A4");let e=evaluate(&v).unwrap_err();assert!(e.iter().any(|x|x["code"]=="PHYSICAL_INDEPENDENCE_OVERCLAIM"));}
    #[test]fn a0_window_profile_requires_both_development_and_evaluation_partitions(){let mut v=fixture();v["profiles"][0]["unit_records"][1]["partition"]=json!("DEVELOPMENT");let e=evaluate(&v).unwrap_err();assert!(e.iter().any(|x|x["code"]=="BOTH_PARTITIONS_REQUIRED"));}
    #[test]
    fn labeled_target_adaptation_is_not_automatically_leakage() {
        let mut fixture = fixture();
        fixture["profiles"][3]["leakage_audit"]["preprocessing_fit_scope"] = json!("TRAIN_ONLY");
        fixture["profiles"][3]["leakage_audit"]["documented_concerns"] = json!([]);
        let evaluated = evaluate(&fixture).unwrap();
        let p = item(&evaluated, "A3-machine-holdout-with-concern");
        assert_eq!(p["target_domain_access"], "LABELED_ADAPTATION");
        assert_eq!(p["leakage_risk"], "LOW");
    }
    #[test]fn test_tuning_is_high_leakage(){let mut v=fixture();v["profiles"][3]["leakage_audit"]["model_selection_data"]=json!("TEST_USED");assert_eq!(item(&evaluate(&v).unwrap(),"A3-machine-holdout-with-concern")["leakage_risk"],"HIGH");}
    #[test]fn global_preprocessing_is_high_leakage(){let mut v=fixture();v["profiles"][1]["leakage_audit"]["preprocessing_fit_scope"]=json!("ALL_DATA");assert_eq!(item(&evaluate(&v).unwrap(),"A1-recording-holdout-operating-shift")["leakage_risk"],"HIGH");}
}
