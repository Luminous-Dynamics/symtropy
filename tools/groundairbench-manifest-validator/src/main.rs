// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic GroundAirBench manifest composition only; never an operating permit.
use serde_json::Value;
use std::{collections::{BTreeMap,BTreeSet},env,fs,path::Path,process::ExitCode};

const TOP:&[&str]=&["schema_version","episode_id","record_state","evidence_class","partition","scenario","inputs_sha256","policy","required_receipts","metric_contracts","expected_session_state","claim_ceiling"];
const OWNERS:&[&str]=&["SOURCE_CATALOGUE","PHYSICAL_SUPPORT","OBSERVABILITY","SESSION_LIFECYCLE","SITE_CAPACITY","ENERGY_METROLOGY","STANDARDS_APPLICABILITY"];
const CRITICAL:&[&str]=&["PHYSICAL_SUPPORT","OBSERVABILITY","SESSION_LIFECYCLE","SITE_CAPACITY","ENERGY_METROLOGY","STANDARDS_APPLICABILITY"];

#[derive(Debug,Clone,PartialEq,Eq)]
pub struct Report { pub episode_id:String, pub status:String, pub expected_session_state:String, pub receipts:usize, pub metrics:usize, pub unresolved:Vec<String> }

fn obj<'a>(v:&'a Value,p:&str)->Result<&'a serde_json::Map<String,Value>,String>{v.as_object().ok_or_else(||format!("{p}: must be an object"))}
fn arr<'a>(v:&'a Value,p:&str)->Result<&'a Vec<Value>,String>{v.as_array().ok_or_else(||format!("{p}: must be an array"))}
fn get<'a>(v:&'a Value,k:&str,p:&str)->Result<&'a Value,String>{obj(v,p)?.get(k).ok_or_else(||format!("{p}: missing {k}"))}
fn s<'a>(v:&'a Value,k:&str,p:&str)->Result<&'a str,String>{get(v,k,p)?.as_str().filter(|x|!x.trim().is_empty()&&x.trim()==*x).ok_or_else(||format!("{p}.{k}: non-empty trimmed string required"))}
fn keys(v:&Value,required:&[&str],optional:&[&str],p:&str)->Result<(),String>{
 let m=obj(v,p)?;
 for k in required {if !m.contains_key(*k){return Err(format!("{p}: missing {k}"));}}
 if let Some(k)=m.keys().find(|k|!required.contains(&k.as_str())&&!optional.contains(&k.as_str())){return Err(format!("{p}: unknown property {k}"));}
 Ok(())
}
fn digest(v:&Value,k:&str,p:&str)->Result<(),String>{
 let x=s(v,k,p)?;
 if x.len()!=64||!x.bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase()){return Err(format!("{p}.{k}: expected lowercase SHA-256"));}
 Ok(())
}
fn one_of(x:&str,values:&[&str],p:&str)->Result<(),String>{if values.contains(&x){Ok(())}else{Err(format!("{p}: unsupported value {x:?}"))}}

pub fn validate(v:&Value)->Result<Report,String>{
 keys(v,TOP,&[],"$")?;
 if s(v,"schema_version","$")?!="groundairbench-episode-v0"{return Err("$.schema_version: unsupported version".into());}
 let id=s(v,"episode_id","$")?;
 s(v,"claim_ceiling","$")?;
 let state=s(v,"record_state","$")?;
 if state=="NOT_RUN"{
  if id!="NOT_RUN"||s(v,"evidence_class","$")?!="NOT_RUN"||s(v,"partition","$")?!="UNASSIGNED"||s(v,"expected_session_state","$")?!="NOT_ADMITTED"{return Err("$: invalid NOT_RUN state".into());}
  for k in ["scenario","inputs_sha256","policy"]{if !get(v,k,"$")?.is_null(){return Err(format!("$: NOT_RUN requires {k}=null"));}}
  if !arr(get(v,"required_receipts","$")?,"$.required_receipts")?.is_empty()||!arr(get(v,"metric_contracts","$")?,"$.metric_contracts")?.is_empty(){return Err("$: NOT_RUN requires empty receipts and metrics".into());}
  return Ok(Report{episode_id:id.into(),status:"NOT_RUN".into(),expected_session_state:"NOT_ADMITTED".into(),receipts:0,metrics:0,unresolved:CRITICAL.iter().map(|x|x.to_string()).collect()});
 }
 if state!="FIXTURE"{return Err("$.record_state: expected FIXTURE or NOT_RUN".into());}
 if s(v,"evidence_class","$")?!="SYNTHETIC"{return Err("$.evidence_class: fixture must be SYNTHETIC".into());}
 one_of(s(v,"partition","$")?,&["TRAIN","VALIDATION","TEST"],"$.partition")?;
 digest(v,"inputs_sha256","$")?;
 let expected=s(v,"expected_session_state","$")?;
 one_of(expected,&["NOT_ADMITTED","SYNTHETIC_ONLY_NO_POWER"],"$.expected_session_state")?;
 let sc=get(v,"scenario","$")?;
 keys(sc,&["bearer","motion","environment_tags"],&[],"$.scenario")?;
 one_of(s(sc,"bearer","$.scenario")?,&["IPT","CPT","RF_MICROWAVE","OPTICAL","CONDUCTIVE","TETHER","BATTERY_EXCHANGE"],"$.scenario.bearer")?;
 one_of(s(sc,"motion","$.scenario")?,&["DOCKED","HOVER","TRANSLATING","CLIMB_DESCENT","DYNAMIC_FLIGHT","DEPARTURE_REACQUISITION"],"$.scenario.motion")?;
 let tags=arr(get(sc,"environment_tags","$.scenario")?,"$.scenario.environment_tags")?;
 if tags.is_empty(){return Err("$.scenario.environment_tags: empty".into());}
 let mut tagset=BTreeSet::new();
 for t in tags{let x=t.as_str().filter(|x|!x.trim().is_empty()&&x.trim()==*x).ok_or("$.scenario.environment_tags: invalid tag")?;if !tagset.insert(x){return Err("$.scenario.environment_tags: duplicate tag".into());}}
 let pol=get(v,"policy","$")?;
 keys(pol,&["policy_id","revision","artifact_sha256","baseline_family"],&[],"$.policy")?;
 for k in ["policy_id","revision","baseline_family"]{s(pol,k,"$.policy")?;}
 digest(pol,"artifact_sha256","$.policy")?;

 let receipts=arr(get(v,"required_receipts","$")?,"$.required_receipts")?;
 let mut owner_status:BTreeMap<String,String>=BTreeMap::new();
 let mut ids=BTreeSet::new();
 for (i,r) in receipts.iter().enumerate(){
  let p=format!("$.required_receipts[{i}]");
  keys(r,&["owner","receipt_id","sha256","evidence_class","disposition","claim_ceiling"],&["signal_strength_proxy_uw"],&p)?;
  let owner=s(r,"owner",&p)?;
  one_of(owner,OWNERS,&format!("{p}.owner"))?;
  let rid=s(r,"receipt_id",&p)?;
  if !ids.insert(rid){return Err(format!("{p}: duplicate receipt_id"));}
  if owner_status.insert(owner.into(),s(r,"disposition",&p)?.into()).is_some(){return Err(format!("{p}: duplicate owner {owner}"));}
  digest(r,"sha256",&p)?;
  if s(r,"evidence_class",&p)?!="SYNTHETIC"{return Err(format!("{p}: synthetic fixture cannot import non-synthetic evidence"));}
  one_of(s(r,"disposition",&p)?,&["PASS","AMBIGUOUS","UNKNOWN","NOT_RUN","NOT_ADMITTED","OUT_OF_DOMAIN","NOT_APPLICABLE"],&format!("{p}.disposition"))?;
  s(r,"claim_ceiling",&p)?;
  if let Some(x)=r.get("signal_strength_proxy_uw"){if x.as_u64().is_none(){return Err(format!("{p}.signal_strength_proxy_uw: expected non-negative integer"));}}
 }
 for owner in OWNERS{if !owner_status.contains_key(*owner){return Err(format!("$.required_receipts: missing owner {owner}"));}}
 if owner_status.len()!=OWNERS.len(){return Err("$.required_receipts: owner set mismatch".into());}
 let unresolved:Vec<String>=CRITICAL.iter().filter(|o|owner_status.get(**o).map(String::as_str)!=Some("PASS")).map(|o|o.to_string()).collect();
 if !unresolved.is_empty()&&expected!="NOT_ADMITTED"{return Err("$.expected_session_state: unresolved critical receipt requires NOT_ADMITTED; signal strength cannot override ambiguity".into());}

 let metrics=arr(get(v,"metric_contracts","$")?,"$.metric_contracts")?;
 if metrics.is_empty(){return Err("$.metric_contracts: at least one metric required".into());}
 let mut mids=BTreeSet::new();
 for (i,m) in metrics.iter().enumerate(){
  let p=format!("$.metric_contracts[{i}]");
  keys(m,&["metric_id","boundary_id","unit","aggregation"],&[],&p)?;
  let mid=s(m,"metric_id",&p)?;if !mids.insert(mid){return Err(format!("{p}: duplicate metric_id"));}
  let b=s(m,"boundary_id",&p)?;let u=s(m,"unit",&p)?;
  let good=match b{"SOURCE_TO_BATTERY"=>u=="Wh","DC_TO_DC"=>matches!(u,"ratio"|"W"|"Wh"),"POSE_OBSERVABILITY"=>matches!(u,"ratio"|"mm"|"ms"|"count"),"MISSION_SERVICE"=>matches!(u,"Wh"|"ratio"|"ms"|"count"),"SITE_POWER"=>matches!(u,"W"|"Wh"),"SESSION_RELIABILITY"=>matches!(u,"ms"|"ratio"|"count"),_=>false};
  if !good{return Err(format!("{p}: unit incompatible with boundary"));}
  one_of(s(m,"aggregation",&p)?,&["NONE","SUM","MEAN","MIN","MAX","P50","P95","P99"],&format!("{p}.aggregation"))?;
 }
 Ok(Report{episode_id:id.into(),status:"COMPLETE_SYNTHETIC_FIXTURE".into(),expected_session_state:expected.into(),receipts:receipts.len(),metrics:metrics.len(),unresolved})
}
pub fn validate_json(text:&str)->Result<Report,String>{let v:Value=serde_json::from_str(text).map_err(|e|format!("invalid JSON: {e}"))?;validate(&v)}
fn run(path:&Path)->Result<(),String>{
 let text=fs::read_to_string(path).map_err(|e|format!("cannot read {}: {e}",path.display()))?;
 let r=validate_json(&text)?;
 println!("CONFORMS: {} — {}",r.episode_id,r.status);
 println!("Expected session state: {}; receipt refs: {}; metric contracts: {}",r.expected_session_state,r.receipts,r.metrics);
 println!("Unresolved critical owners: {}",if r.unresolved.is_empty(){"none".into()}else{r.unresolved.join(", ")});
 println!("CLAIM CEILING: synthetic manifest composition only; no physical, safety, compliance or flight claim");
 Ok(())
}
fn main()->ExitCode{let p=env::args().nth(1).unwrap_or_else(||"research/groundairbench-v0/ambiguous-pose.synthetic.json".into());match run(Path::new(&p)){Ok(())=>ExitCode::SUCCESS,Err(e)=>{eprintln!("NOT_CONFORMS: {e}");ExitCode::FAILURE}}}

#[cfg(test)]
mod tests{
 use super::*;use serde_json::json;
 const FIXTURE:&str=include_str!("../../../research/groundairbench-v0/ambiguous-pose.synthetic.json");
 const NOT_RUN:&str=include_str!("../../../research/groundairbench-v0/episode.NOT_RUN.template.json");
 fn fixture()->Value{serde_json::from_str(FIXTURE).unwrap()}
 #[test]fn ambiguous_pose_is_composed_but_not_admitted(){let r=validate_json(FIXTURE).unwrap();assert_eq!(r.status,"COMPLETE_SYNTHETIC_FIXTURE");assert_eq!(r.expected_session_state,"NOT_ADMITTED");assert!(r.unresolved.contains(&"OBSERVABILITY".into()));assert_eq!(r.receipts,7);}
 #[test]fn not_run_is_not_a_pass(){let r=validate_json(NOT_RUN).unwrap();assert_eq!(r.status,"NOT_RUN");assert_eq!(r.expected_session_state,"NOT_ADMITTED");}
 #[test]fn high_signal_proxy_cannot_override_ambiguous_pose(){let mut v=fixture();for r in v["required_receipts"].as_array_mut().unwrap(){if r["owner"]=="OBSERVABILITY"{r["signal_strength_proxy_uw"]=json!(9999999999_u64);}}let r=validate(&v).unwrap();assert_eq!(r.expected_session_state,"NOT_ADMITTED");}
 #[test]fn missing_owner_fails_closed(){let mut v=fixture();v["required_receipts"].as_array_mut().unwrap().retain(|x|x["owner"]!="ENERGY_METROLOGY");assert!(validate(&v).unwrap_err().contains("missing owner ENERGY_METROLOGY"));}
 #[test]fn duplicate_owner_fails_closed(){let mut v=fixture();let mut x=v["required_receipts"][0].clone();x["receipt_id"]=json!("second-source-catalogue-reference");x["owner"]=json!("SOURCE_CATALOGUE");v["required_receipts"].as_array_mut().unwrap().push(x);assert!(validate(&v).unwrap_err().contains("duplicate owner"));}
 #[test]fn malformed_digest_fails_closed(){let mut v=fixture();v["required_receipts"][0]["sha256"]=json!("bad");assert!(validate(&v).unwrap_err().contains("expected lowercase SHA-256"));}
 #[test]fn synthetic_fixture_rejects_field_evidence(){let mut v=fixture();v["required_receipts"][0]["evidence_class"]=json!("FIELD_MEASURED");assert!(validate(&v).unwrap_err().contains("cannot import non-synthetic"));}
 #[test]fn unknown_fields_fail_closed(){let mut v=fixture();v["unexpected"]=json!(true);assert!(validate(&v).unwrap_err().contains("unknown property unexpected"));}
 #[test]fn metric_boundary_and_unit_must_match(){let mut v=fixture();v["metric_contracts"][0]["boundary_id"]=json!("SOURCE_TO_BATTERY");v["metric_contracts"][0]["unit"]=json!("ratio");assert!(validate(&v).unwrap_err().contains("unit incompatible"));}
 #[test]fn receipt_order_does_not_change_report(){let a=validate(&fixture()).unwrap();let mut v=fixture();v["required_receipts"].as_array_mut().unwrap().reverse();assert_eq!(a,validate(&v).unwrap());}
 #[test]fn unresolved_receipt_cannot_claim_synthetic_service(){let mut v=fixture();v["expected_session_state"]=json!("SYNTHETIC_ONLY_NO_POWER");assert!(validate(&v).unwrap_err().contains("unresolved critical receipt requires NOT_ADMITTED"));}
}
