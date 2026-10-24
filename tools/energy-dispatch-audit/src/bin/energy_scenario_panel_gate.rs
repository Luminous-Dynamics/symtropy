use std::{env, error::Error, fs, io, path::Path};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use symtropy_energy_dispatch_audit::DispatchObjective;

fn sha256_hex(bytes: &[u8]) -> String { format!("{:x}", Sha256::digest(bytes)) }
fn invalid(message: impl Into<String>) -> Box<dyn Error> { io::Error::new(io::ErrorKind::InvalidData, message.into()).into() }
fn req_str<'a>(v: &'a Value, k: &str) -> Result<&'a str, Box<dyn Error>> {
    v.get(k).and_then(Value::as_str).ok_or_else(|| invalid(format!("missing string field {k}")))
}
fn req_u64(v: &Value, k: &str) -> Result<u64, Box<dyn Error>> {
    v.get(k).and_then(Value::as_u64).ok_or_else(|| invalid(format!("missing unsigned field {k}")))
}
fn object_order(v: &DispatchObjective) -> (u64,u64,u64,u64,u64,std::cmp::Reverse<u16>) {
    (v.essential_unserved_quanta,v.flexible_unserved_quanta,v.operating_cost_units,v.grid_carbon_grams,v.curtailed_pv_quanta,std::cmp::Reverse(v.terminal_soc_quanta))
}
fn parse_objectives(v: &Value) -> Result<Vec<DispatchObjective>, Box<dyn Error>> {
    let arr=v.as_array().ok_or_else(|| invalid("objective list must be array"))?;
    let mut out=arr.iter().map(|x|serde_json::from_value::<DispatchObjective>(x.clone())).collect::<Result<Vec<_>,_>>()?;
    out.sort_by_key(object_order); Ok(out)
}
fn weakly_dominates(a:&DispatchObjective,b:&DispatchObjective)->bool {
    a.essential_unserved_quanta<=b.essential_unserved_quanta
    && a.flexible_unserved_quanta<=b.flexible_unserved_quanta
    && a.operating_cost_units<=b.operating_cost_units
    && a.grid_carbon_grams<=b.grid_carbon_grams
    && a.curtailed_pv_quanta<=b.curtailed_pv_quanta
    && a.terminal_soc_quanta>=b.terminal_soc_quanta
}
fn derived_frontier(all:&[DispatchObjective])->Vec<DispatchObjective>{
    let mut unique=all.to_vec(); unique.sort(); unique.dedup();
    let mut f=unique.iter().filter(|a|!unique.iter().any(|b|b!=*a&&weakly_dominates(b,a))).cloned().collect::<Vec<_>>();
    f.sort_by_key(object_order); f
}
fn set_pointer(v:&mut Value,path:&str,replacement:&Value)->Result<(),Box<dyn Error>>{
    *v.pointer_mut(path).ok_or_else(||invalid(format!("missing mutation path {path}")))?=replacement.clone(); Ok(())
}
fn verify_transforms(manifest:&Value,cases:&[Value])->Result<(),Box<dyn Error>>{
    let base_id=req_str(manifest,"base_case_id")?;
    let base=cases.iter().find(|x|x["case_id"].as_str()==Some(base_id)).ok_or_else(||invalid("base case missing"))?;
    let base_req=&base["request"];
    for item in cases {
        let id=req_str(item,"case_id")?;
        if req_str(&item["request"],"case_id")?!=id || item["request"]["synthetic"]!=true {return Err(invalid(format!("request identity/synthetic boundary mismatch for {id}")));}
        let muts=item["mutations"].as_array().ok_or_else(||invalid(format!("mutation list missing for {id}")))?;
        if item["parent_case_id"].is_null() {
            if id!=base_id || !muts.is_empty() || item["request"]!=*base_req {return Err(invalid("only unchanged base case may be parentless"));}
            continue;
        }
        if item["parent_case_id"].as_str()!=Some(base_id) {return Err(invalid(format!("unexpected parent for {id}")));}
        let mut reconstructed=base_req.clone(); let mut paths=BTreeSet::new();
        for m in muts {
            let path=req_str(m,"path")?;
            if !paths.insert(path.to_owned()) {return Err(invalid(format!("duplicate mutation path {path}")));}
            let before=m.get("before").ok_or_else(||invalid("mutation before missing"))?;
            let after=m.get("after").ok_or_else(||invalid("mutation after missing"))?;
            if reconstructed.pointer(path)!=Some(before) {return Err(invalid(format!("mutation precondition mismatch at {path} in {id}")));}
            set_pointer(&mut reconstructed,path,after)?;
        }
        if reconstructed!=item["request"] {return Err(invalid(format!("undeclared request change in {id}")));}
    }
    Ok(())
}
fn validate_results(item:&Value,dir:&Path,manifest_digest:&str)->Result<Value,Box<dyn Error>>{
    let id=req_str(item,"case_id")?;
    let read=|name:&str| -> Result<Vec<u8>,Box<dyn Error>> { Ok(fs::read(dir.join(name))?) };
    let req_bytes=read("request.json")?; let ans_bytes=read("known-answer.json")?;
    let dispatch_bytes=read("dispatch-report.json")?; let audit_bytes=read("independent-audit.json")?;
    let log_bytes=read("qualification.log")?;
    let request:Value=serde_json::from_slice(&req_bytes)?; let answer:Value=serde_json::from_slice(&ans_bytes)?;
    let dispatch:Value=serde_json::from_slice(&dispatch_bytes)?; let audit:Value=serde_json::from_slice(&audit_bytes)?;
    let log=std::str::from_utf8(&log_bytes)?;
    if request!=item["request"] || answer!=item["known_answer"] {return Err(invalid(format!("materialized request/answer differs from frozen manifest for {id}")));}
    let req_hash=sha256_hex(&req_bytes); let ans_hash=sha256_hex(&ans_bytes); let dispatch_hash=sha256_hex(&dispatch_bytes); let audit_hash=sha256_hex(&audit_bytes); let log_hash=sha256_hex(&log_bytes);
    if req_str(&answer,"schema_version")?!="energy-dispatch-known-answer-v0.1"
        || req_str(&answer,"case_id")?!=id || req_str(&dispatch["receipt"],"case_id")?!=id
        || req_str(&dispatch["assessment"],"case_id")?!=id || req_str(&audit,"case_id")?!=id
        || req_str(&dispatch["receipt"],"input_sha256")?!=req_hash
        || req_str(&audit,"request_sha256")?!=req_hash || req_str(&audit,"run_report_sha256")?!=dispatch_hash
        || req_u64(&dispatch["receipt"],"input_bytes")?!=req_bytes.len() as u64
        || req_u64(&dispatch["assessment"],"feasible_complete_schedules")?!=req_u64(&answer,"feasible_complete_schedules")?
        || req_u64(&audit,"reference_feasible_complete_schedules")?!=req_u64(&answer,"feasible_complete_schedules")?
        || req_str(&audit,"status")?!="pass" || req_str(&audit,"reference_frontier_status")?!="pass"
        || req_str(&audit,"objective_claim_status")?!="verified_for_bounded_model_case"
        || dispatch["assessment"]["search_is_exact"]!=true || dispatch["assessment"]["pareto_frontier_complete"]!=true
    {return Err(invalid(format!("report identity/hash/status mismatch for {id}")));}
    let alt=dispatch["assessment"]["pareto_alternatives"].as_array().ok_or_else(||invalid("dispatch alternatives missing"))?;
    let solver=parse_objectives(&Value::Array(alt.iter().map(|a|a["objective"].clone()).collect()))?;
    let reference=parse_objectives(&audit["reference_frontier_objectives"])?;
    let expected=parse_objectives(&answer["pareto_frontier"])?;
    let all=parse_objectives(&answer["feasible_schedule_objectives"])?;
    let mut distinct=all.clone(); distinct.sort(); distinct.dedup();
    if all.len() as u64!=req_u64(&answer,"feasible_complete_schedules")?
        || distinct.len() as u64!=req_u64(&answer,"distinct_objective_count")?
        || derived_frontier(&all)!=expected || solver!=expected || reference!=expected
        || req_u64(&dispatch["assessment"],"pareto_frontier_count")?!=solver.len() as u64
    {return Err(invalid(format!("known/solver/reference objective frontier disagreement for {id}")));}
    let checks=audit["checks"].as_array().ok_or_else(||invalid("audit checks missing"))?;
    if checks.is_empty()||checks.iter().any(|c|c["status"]!="pass"){return Err(invalid(format!("audit has non-pass checks for {id}")));}
    if !log.contains(&format!("PASS: actual Sol-Atlas scenario {id} matches"))
        || !log.contains(&format!("request_sha256={req_hash}"))
        || !log.contains(&format!("dispatch_report_sha256={dispatch_hash}"))
        || !log.contains(&format!("independent_audit_sha256={audit_hash}"))
    {return Err(invalid(format!("gate log does not bind report hashes for {id}")));}
    let mut frontier=solver.clone(); frontier.sort_by_key(object_order);
    Ok(json!({
        "case_id":id,"label":req_str(item,"label")?,"scenario_family":req_str(item,"scenario_family")?,
        "panel_manifest_sha256":manifest_digest,
        "request_sha256":req_hash,"known_answer_sha256":ans_hash,"dispatch_report_sha256":dispatch_hash,
        "independent_audit_sha256":audit_hash,"qualification_log_sha256":log_hash,
        "feasible_complete_schedules":req_u64(&answer,"feasible_complete_schedules")?,
        "distinct_objective_count":req_u64(&answer,"distinct_objective_count")?,
        "pareto_frontier_count":expected.len(),"audit_check_count":checks.len(),
        "dispatch_ns":fs::read_to_string(dir.join("dispatch.elapsed_ns"))?.trim().parse::<u64>()?,
        "audit_ns":fs::read_to_string(dir.join("audit.elapsed_ns"))?.trim().parse::<u64>()?,
        "known_answer_gate_ns":fs::read_to_string(dir.join("known-answer-gate.elapsed_ns"))?.trim().parse::<u64>()?,
        "mutations":item["mutations"].clone(),"solver_frontier":frontier,"independent_frontier":reference
    }))
}
fn run()->Result<(),Box<dyn Error>>{
    let args=env::args().collect::<Vec<_>>();
    if args.len()!=4{return Err(io::Error::new(io::ErrorKind::InvalidInput,"usage: energy_scenario_panel_gate <panel-manifest.json> <results-root> <panel-report.json>").into());}
    let manifest_bytes=fs::read(&args[1])?; let manifest:Value=serde_json::from_slice(&manifest_bytes)?;
    if req_str(&manifest,"schema_version")?!="energy-scenario-panel-v0.1" || req_str(&manifest,"status")?!="frozen_synthetic_correctness_panel_not_site_calibrated"{return Err(invalid("unsupported panel schema or claim boundary"));}
    let cases=manifest["cases"].as_array().ok_or_else(||invalid("panel cases missing"))?;
    if cases.is_empty()||cases.len()>64||req_u64(&manifest,"case_count")?!=cases.len() as u64{return Err(invalid("panel case count invalid"));}
    let mut ids=BTreeSet::new(); for c in cases {if !ids.insert(req_str(c,"case_id")?.to_owned()){return Err(invalid("duplicate panel case ID"));}}
    verify_transforms(&manifest,cases)?;
    let panel_hash=sha256_hex(&manifest_bytes); let root=std::path::PathBuf::from(&args[2]);
    let mut rows=Vec::new(); for c in cases {let id=req_str(c,"case_id")?;rows.push(validate_results(c,&root.join(id),&panel_hash)?);}
    let report=json!({
      "schema_version":"energy-scenario-panel-report-v0.1","panel_id":req_str(&manifest,"panel_id")?,
      "manifest_sha256":panel_hash,"base_case_id":req_str(&manifest,"base_case_id")?,
      "claim_ceiling":"Synthetic finite-model robustness panel only; not a calibrated site resilience or cost-savings result.",
      "case_count":rows.len(),"passing_case_count":rows.len(),"all_passed":true,"cases":rows
    });
    let output=serde_json::to_vec_pretty(&report)?;fs::write(&args[3],&output)?;
    println!("PASS: scenario panel verified {} actual Sol-Atlas dispatch/audit cases",rows.len());
    println!("manifest_sha256={}",sha256_hex(&manifest_bytes));println!("panel_report_sha256={}",sha256_hex(&output));Ok(())
}
fn main(){if let Err(e)=run(){eprintln!("energy scenario panel gate failed: {e}");std::process::exit(2);}}
#[cfg(test)]
mod tests{
 use super::*;
 const PANEL:&str=include_str!("../../fixtures/energy-scenario-panel-v0.1.json");
 #[test] fn frozen_scenario_mutations_are_exact(){
   let m:Value=serde_json::from_str(PANEL).expect("panel parses");let c=m["cases"].as_array().unwrap();
   let mut ids=BTreeSet::new();for x in c{assert!(ids.insert(x["case_id"].as_str().unwrap().to_owned()));}
   verify_transforms(&m,c).expect("each scenario matches declared mutations");
 }
 #[test] fn undeclared_scenario_mutation_is_rejected(){
   let mut m:Value=serde_json::from_str(PANEL).expect("panel parses");
   let cases=m["cases"].as_array_mut().unwrap();
   let item=cases.iter_mut().find(|x|x["case_id"]=="synthetic-three-interval-pv-loss").unwrap();
   item["request"]["intervals"][1]["grid_import_limit_quanta"]=json!(0);
   let frozen_cases=cases.clone();
   assert!(verify_transforms(&m,&frozen_cases).is_err());
 }
}
