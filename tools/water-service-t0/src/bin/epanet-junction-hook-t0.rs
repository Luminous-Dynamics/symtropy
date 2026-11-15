// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::{Path, PathBuf}};

const PROGRAMME:&str="WATER-MIXING-ENGINE-INTEROP-001";
const CASE_COUNT:usize=10;
const EVIDENCE_CEILING:&str="synthetic EPANET junction-hook snapshot contract arithmetic only";

fn field<'a>(v:&'a Value,k:&str)->Result<&'a Value,String>{v.get(k).ok_or_else(||format!("missing field: {k}"))}
fn text<'a>(v:&'a Value,k:&str)->Result<&'a str,String>{field(v,k)?.as_str().ok_or_else(||format!("{k} must be text"))}
fn num(v:&Value,k:&str)->Result<f64,String>{field(v,k)?.as_f64().filter(|x|x.is_finite()).ok_or_else(||format!("{k} must be finite numeric"))}
fn boolv(v:&Value,k:&str)->Result<bool,String>{field(v,k)?.as_bool().ok_or_else(||format!("{k} must be boolean"))}
fn arr<'a>(v:&'a Value,k:&str)->Result<&'a Vec<Value>,String>{field(v,k)?.as_array().ok_or_else(||format!("{k} must be array"))}
fn valid_sha(value:&str,n:usize)->bool{
    value.len()==n&&value.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))
}
fn base(state:&str,reason:&str)->Value{
    json!({
        "hook_disposition":state,"reason":reason,
        "incoming_segment_volume_m3":null,"incoming_segment_mass_kg":null,
        "external_source_volume_m3":null,"external_source_mass_kg":null,
        "outlet_volume_m3":null,"operator_outlet_mass_kg":null,
        "inventory_delta_volume_m3":null,"inventory_delta_mass_kg":null,
        "volume_balance_residual_m3":null,"mass_balance_residual_kg":null,
        "baseline_complete_mix_concentration_kg_per_m3":null,
        "baseline_parity_max_abs_mass_error_kg":null,
        "outlet_ports":null,
        "snapshot_contains_consumed_segment_records":false,
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    })
}
fn compute(case:&Value)->Result<Value,String>{
    let engine=text(case,"engine_name")?;
    if engine!="EPANET-2.2-fork-hook"&&engine!="OWA-EPANET-2.3-fork-hook"{
        return Ok(base("REJECTED_CONTRACT","UNSUPPORTED_ENGINE_IDENTITY"));
    }
    let rev=text(case,"engine_revision")?;
    if !valid_sha(rev,40){return Ok(base("REJECTED_CONTRACT","ENGINE_REVISION_NOT_PINNED"));}
    for key in ["geometry_sha256","operator_sha256","snapshot_sha256"]{
        if !valid_sha(text(case,key)?,64){return Ok(base("REJECTED_CONTRACT","PROVENANCE_DIGEST_INVALID"));}
    }
    let mut snapshot_material=case.clone();
    snapshot_material.as_object_mut().ok_or_else(||"hook snapshot must be an object".to_owned())?
        .remove("snapshot_sha256");
    let snapshot_bytes=serde_json::to_vec(&snapshot_material).map_err(|e|format!("serialize snapshot: {e}"))?;
    if digest(&snapshot_bytes)!=text(case,"snapshot_sha256")?{
        return Ok(base("REJECTED_CONTRACT","SNAPSHOT_DIGEST_MISMATCH"));
    }
    let node=text(case,"node_id")?;
    if node.trim().is_empty(){return Ok(base("REJECTED_CONTRACT","NODE_ID_EMPTY"));}
    let qtime=num(case,"quality_time_seconds")?;
    let qstep=num(case,"quality_step_seconds")?;
    let until_event=num(case,"time_to_hydraulic_event_seconds")?;
    if qtime<0.0||qstep<=0.0||until_event<=0.0||qstep>until_event+1.0e-12{
        return Ok(base("INVALID_INPUT","QUALITY_STEP_NOT_ALIGNED_TO_HYDRAULIC_EVENT"));
    }
    let domain=text(case,"domain_state")?;
    if domain!="in_domain"{return Ok(base("OUT_OF_DOMAIN","OPERATOR_DOMAIN_NOT_QUALIFIED_FOR_NODE_STATE"));}
    if boolv(case,"flow_reversal_present")?&&!boolv(case,"all_port_directions_observed")?{
        return Ok(base("ABSTAIN_FLOW_DIRECTION_UNOBSERVED","SIGNED_PORT_DIRECTION_INCOMPLETE"));
    }
    let sources=arr(case,"external_sources")?;
    let mut src_ids=BTreeSet::new();
    let mut src_mass=0.0;let mut src_volume=0.0;
    for s in sources{
        let id=text(s,"source_id")?;
        if !src_ids.insert(id.to_owned()){return Ok(base("REJECTED_CONTRACT","DUPLICATE_EXTERNAL_SOURCE_ID"));}
        match text(s,"source_type")?{
            "CONCEN"=>{
                let vol=num(s,"source_volume_m3")?;let conc=num(s,"concentration_kg_per_m3")?;
                if vol<0.0||conc<0.0{return Ok(base("INVALID_INPUT","NEGATIVE_CONCENTRATION_SOURCE_VALUE"));}
                src_volume+=vol;src_mass+=vol*conc;
            }
            "MASS"=>{
                let mass=num(s,"source_mass_kg")?;
                if mass<0.0{return Ok(base("INVALID_INPUT","NEGATIVE_MASS_SOURCE"));}
                src_mass+=mass;
            }
            "FLOWPACED"|"SETPOINT"=>{
                return Ok(base("UNSUPPORTED_SOURCE_SEMANTICS","FLOWPACED_OR_SETPOINT_SOURCE_NOT_IMPLEMENTED_IN_HOOK_CONTRACT"));
            }
            _=>return Ok(base("REJECTED_CONTRACT","UNKNOWN_EXTERNAL_SOURCE_TYPE"))
        }
    }
    let segments=arr(case,"consumed_segments")?;
    if segments.is_empty(){return Ok(base("INVALID_INPUT","NO_INCOMING_SEGMENT_CONTRIBUTIONS"));}
    let mut segment_ids=BTreeSet::new();
    let mut incoming_volume=0.0;let mut incoming_mass=0.0;
    for seg in segments{
        let link=text(seg,"link_id")?;let sid=text(seg,"segment_id")?;
        if !segment_ids.insert(format!("{link}::{sid}")){return Ok(base("REJECTED_CONTRACT","DUPLICATE_CONSUMED_SEGMENT_ID"));}
        if text(seg,"direction")? != "into_node"{return Ok(base("REJECTED_CONTRACT","CONSUMED_SEGMENT_DIRECTION_NOT_INTO_NODE"));}
        let vol=num(seg,"consumed_volume_m3")?;let conc=num(seg,"concentration_kg_per_m3")?;
        let total_segment_volume=num(seg,"available_segment_volume_m3")?;
        if vol<0.0||conc<0.0||total_segment_volume<0.0||vol>total_segment_volume+1.0e-12{
            return Ok(base("INVALID_INPUT","CONSUMED_SEGMENT_VOLUME_OR_QUALITY_INVALID"));
        }
        incoming_volume+=vol;incoming_mass+=vol*conc;
    }
    if !incoming_volume.is_finite()||!incoming_mass.is_finite()||!src_volume.is_finite()||!src_mass.is_finite(){
        return Ok(base("INVALID_INPUT","INCOMING_MASS_OR_VOLUME_OVERFLOW"));
    }
    let outlet_rows=arr(case,"outlet_ports")?;
    let expected_rows=arr(case,"expected_outlet_port_ids")?;
    if expected_rows.is_empty(){return Ok(base("INVALID_INPUT","EXPECTED_OUTLET_SET_EMPTY"));}
    let mut expected=BTreeSet::new();
    for x in expected_rows{
        let id=x.as_str().ok_or_else(||"expected outlet identity must be text".to_owned())?;
        if !expected.insert(id.to_owned()){return Ok(base("REJECTED_CONTRACT","DUPLICATE_EXPECTED_OUTLET_ID"));}
    }
    let mut observed=BTreeSet::new();
    let mut out_volume=0.0;let mut out_mass=0.0;
    let mut outlet_detail=Vec::new();
    for p in outlet_rows{
        let id=text(p,"port_id")?;
        if !observed.insert(id.to_owned()){return Ok(base("REJECTED_CONTRACT","DUPLICATE_OUTLET_PORT_ID"));}
        if !expected.contains(id){return Ok(base("REJECTED_CONTRACT","UNEXPECTED_OUTLET_PORT_ID"));}
        let kind=text(p,"port_kind")?;
        if !["pipe","demand","emitter","leakage"].contains(&kind){return Ok(base("REJECTED_CONTRACT","UNSUPPORTED_OUTLET_PORT_KIND"));}
        let vol=num(p,"volume_m3")?;let mass=num(p,"operator_mass_kg")?;
        if vol<0.0||mass<0.0{return Ok(base("INVALID_INPUT","NEGATIVE_OUTLET_VOLUME_OR_MASS"));}
        let conc=if vol>0.0{mass/vol}else if mass.abs()<=1.0e-12{0.0}else{return Ok(base("INVALID_INPUT","MASS_ASSIGNED_TO_ZERO_FLOW_OUTLET"));};
        if !conc.is_finite(){return Ok(base("INVALID_INPUT","OUTLET_CONCENTRATION_OVERFLOW"));}
        out_volume+=vol;out_mass+=mass;
        outlet_detail.push(json!({"port_id":id,"port_kind":kind,"volume_m3":vol,"operator_mass_kg":mass,"derived_concentration_kg_per_m3":conc}));
    }
    if observed.len()!=expected.len()||observed!=expected||!boolv(case,"all_outlet_ports_observed")?{
        return Ok(json!({
            "hook_disposition":"PARTIAL_PORT_BOUNDARY","reason":"OUTGOING_PORT_ID_SET_INCOMPLETE",
            "incoming_segment_volume_m3":incoming_volume,"incoming_segment_mass_kg":incoming_mass,
            "external_source_volume_m3":src_volume,"external_source_mass_kg":src_mass,
            "outlet_volume_m3":out_volume,"operator_outlet_mass_kg":out_mass,
            "inventory_delta_volume_m3":null,"inventory_delta_mass_kg":null,
            "volume_balance_residual_m3":null,"mass_balance_residual_kg":null,
            "baseline_complete_mix_concentration_kg_per_m3":null,"baseline_parity_max_abs_mass_error_kg":null,
            "outlet_ports":outlet_detail,"snapshot_contains_consumed_segment_records":true,
            "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
        }));
    }
    let inventory_vol=num(case,"inventory_delta_volume_m3")?;
    let inventory_mass=num(case,"inventory_delta_mass_kg")?;
    let mass_sigma=num(case,"mass_balance_standard_uncertainty_kg")?;
    let volume_sigma=num(case,"volume_balance_standard_uncertainty_m3")?;
    let k=num(case,"closure_sigma_limit")?;
    if mass_sigma<=0.0||volume_sigma<=0.0||k<=0.0||!inventory_vol.is_finite()||!inventory_mass.is_finite(){
        return Ok(base("INVALID_INPUT","BALANCE_UNCERTAINTY_OR_INVENTORY_INVALID"));
    }
    let volume_residual=incoming_volume+src_volume-out_volume-inventory_vol;
    let mass_residual=incoming_mass+src_mass-out_mass-inventory_mass;
    if !volume_residual.is_finite()||!mass_residual.is_finite(){
        return Ok(base("INVALID_INPUT","BALANCE_RESIDUAL_NONFINITE"));
    }
    let baseline=match text(case,"operator_mode")?{
        "baseline_complete_mix"=>{
            if inventory_vol.abs()>1.0e-12||inventory_mass.abs()>1.0e-12{
                return Ok(base("REJECTED_CONTRACT","BASELINE_COMPLETE_MIX_CANNOT_CLAIM_LOCAL_INVENTORY"));
            }
            let total_v=incoming_volume+src_volume;
            if total_v<=0.0{return Ok(base("INVALID_INPUT","BASELINE_INFLOW_VOLUME_NOT_POSITIVE"));}
            let c=(incoming_mass+src_mass)/total_v;
            let max_error=outlet_rows.iter().filter_map(|p|{
                let expected_mass=c*num(p,"volume_m3").ok()?;
                Some((num(p,"operator_mass_kg").ok()?-expected_mass).abs())
            }).fold(0.0_f64,f64::max);
            let tol=num(case,"baseline_parity_tolerance_kg")?;
            if tol<0.0{return Ok(base("INVALID_INPUT","BASELINE_PARITY_TOLERANCE_NEGATIVE"));}
            if max_error>tol{return Ok(json!({
                "hook_disposition":"BASELINE_PARITY_MISMATCH","reason":"OUTLET_MASS_DIFFERS_FROM_FLOW_WEIGHTED_COMPLETE_MIX",
                "incoming_segment_volume_m3":incoming_volume,"incoming_segment_mass_kg":incoming_mass,
                "external_source_volume_m3":src_volume,"external_source_mass_kg":src_mass,
                "outlet_volume_m3":out_volume,"operator_outlet_mass_kg":out_mass,
                "inventory_delta_volume_m3":inventory_vol,"inventory_delta_mass_kg":inventory_mass,
                "volume_balance_residual_m3":volume_residual,"mass_balance_residual_kg":mass_residual,
                "baseline_complete_mix_concentration_kg_per_m3":c,"baseline_parity_max_abs_mass_error_kg":max_error,
                "outlet_ports":outlet_detail,"snapshot_contains_consumed_segment_records":true,
                "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
            }));}
            Some((c,max_error))
        }
        "outlet_resolved"=>None,
        _=>return Ok(base("REJECTED_CONTRACT","UNKNOWN_OPERATOR_MODE"))
    };
    let state=if volume_residual.abs()>k*volume_sigma{
        ("OPEN_VOLUME_RESIDUAL","NODE_VOLUME_BALANCE_OUTSIDE_UNCERTAINTY")
    }else if mass_residual.abs()>k*mass_sigma{
        ("OPEN_MASS_RESIDUAL","NODE_MASS_BALANCE_OUTSIDE_UNCERTAINTY")
    }else{
        ("CLOSED_WITHIN_DECLARED_UNCERTAINTY","")
    };
    Ok(json!({
        "hook_disposition":state.0,"reason":if state.1.is_empty(){Value::Null}else{json!(state.1)},
        "incoming_segment_volume_m3":incoming_volume,"incoming_segment_mass_kg":incoming_mass,
        "external_source_volume_m3":src_volume,"external_source_mass_kg":src_mass,
        "outlet_volume_m3":out_volume,"operator_outlet_mass_kg":out_mass,
        "inventory_delta_volume_m3":inventory_vol,"inventory_delta_mass_kg":inventory_mass,
        "volume_balance_residual_m3":volume_residual,"mass_balance_residual_kg":mass_residual,
        "baseline_complete_mix_concentration_kg_per_m3":baseline.map(|x|x.0),
        "baseline_parity_max_abs_mass_error_kg":baseline.map(|x|x.1),
        "outlet_ports":outlet_detail,"snapshot_contains_consumed_segment_records":true,
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    }))
}
fn validate(input:&Value,oracle:&Value,manifest:&Value)->Result<(),String>{
    if text(input,"schema_version")? != "epanet-junction-hook-t0-input-v0.1"{return Err("unexpected input schema".into());}
    if text(oracle,"schema_version")? != "epanet-junction-hook-t0-oracle-v0.1"{return Err("unexpected oracle schema".into());}
    for x in [input,oracle,manifest]{if text(x,"programme_id")? != PROGRAMME{return Err("programme identity mismatch".into());}}
    for k in ["physical_operator_qualified","potability_claim_permitted","release_authorized"]{
        if field(oracle,k)?.as_bool()!=Some(false){return Err(format!("oracle cannot claim {k}"));}
    }
    let cases=arr(input,"cases")?;let expected=arr(oracle,"cases")?;let ids=arr(manifest,"case_ids")?;
    if cases.len()!=CASE_COUNT||expected.len()!=CASE_COUNT||usize_value(manifest,"case_count")? != CASE_COUNT{
        return Err("frozen hook case count mismatch".into());
    }
    let input_ids=ids_from(cases,"case_id")?;let oracle_ids=ids_from(expected,"case_id")?;
    let manifest_ids=ids.iter().map(|x|x.as_str().map(str::to_owned).ok_or_else(||"manifest case ID must be text".to_owned())).collect::<Result<BTreeSet<_>,_>>()?;
    if input_ids.len()!=cases.len()||oracle_ids.len()!=expected.len()||manifest_ids.len()!=ids.len()||input_ids!=oracle_ids||input_ids!=manifest_ids{
        return Err("input/oracle/manifest hook identities differ or duplicate".into());
    }
    Ok(())
}
fn ids_from(rows:&[Value],key:&str)->Result<BTreeSet<String>,String>{rows.iter().map(|x|text(x,key).map(str::to_owned)).collect()}
fn compare(a:&Value,b:&Value)->bool{
    match(a,b){
        (Value::Number(x),Value::Number(y))=>matches!((x.as_f64(),y.as_f64()),(Some(p),Some(q)) if (p-q).abs()<=1.0e-8*p.abs().max(q.abs()).max(1.0)),
        (Value::Array(x),Value::Array(y))=>x.len()==y.len()&&x.iter().zip(y).all(|(a,b)|compare(a,b)),
        (Value::Object(x),Value::Object(y))=>x.len()==y.len()&&x.iter().all(|(k,v)|y.get(k).is_some_and(|w|compare(v,w))),
        _=>a==b
    }
}
pub fn evaluate(input:&Value,oracle:&Value,manifest:&Value)->Result<Value,String>{
    validate(input,oracle,manifest)?;
    let mut expected_by_id=std::collections::BTreeMap::new();
    for x in arr(oracle,"cases")?{expected_by_id.insert(text(x,"case_id")?.to_owned(),field(x,"expected")?.clone());}
    let mut rows=Vec::new();let mut mismatches=Vec::new();
    for c in arr(input,"cases")?{
        let id=text(c,"case_id")?;let computed=compute(field(c,"snapshot")?)?;
        let expected=expected_by_id.get(id).ok_or_else(||format!("oracle missing {id}"))?;
        if !compare(&computed,expected){mismatches.push(json!({"case_id":id,"expected":expected,"computed":computed}));}
        rows.push(json!({"case_id":id,"matched":compare(&computed,expected),"result":computed}));
    }
    rows.sort_by(|a,b|a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched=rows.iter().filter(|x|x["matched"]==true).count();
    Ok(json!({
        "schema_version":"epanet-junction-hook-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty(){"MatchedFrozenOracle"}else{"OracleMismatch"},
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "case_count":rows.len(),"matched_case_count":matched,"mismatches":mismatches,"results":rows,
        "nonclaims":[
            "The snapshot schema is a proposed engine-fork hook contract; the public Toolkit does not expose consumed leading-segment records.",
            "The evaluator is arithmetic only and does not execute EPANET hydraulics, segment transport, reactions, flow reversals or a real junction.",
            "A matched mass balance does not qualify a physical mixing operator, sensor, tracer or potable system.",
            "No output establishes potability, contamination, toxicology, SANS 241 compliance or release authority."
        ]
    }))
}
fn digest(bytes:&[u8])->String{Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()}
fn read_json(path:&Path)->Result<(Vec<u8>,Value),String>{
    let bytes=fs::read(path).map_err(|e|format!("read {}: {e}",path.display()))?;
    let value=serde_json::from_slice(&bytes).map_err(|e|format!("parse {}: {e}",path.display()))?;
    Ok((bytes,value))
}
fn valid_revision(v:&str)->bool{valid_sha(v,40)}
fn run()->Result<i32,String>{
    let args:Vec<String>=env::args().skip(1).collect();
    if args.len()>4{return Err("usage: epanet-junction-hook-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into());}
    let ip=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/epanet-junction-hook-t0-cases-v0.1.json"));
    let op=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/epanet-junction-hook-t0-oracle-v0.1.json"));
    let mp=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/epanet-junction-hook-t0-manifest-v0.1.json"));
    let rp=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/epanet-junction-hook-t0/receipt.json"));
    let (ib,iv)=read_json(&ip)?;let (ob,ov)=read_json(&op)?;let (mb,mv)=read_json(&mp)?;
    let evaluation=evaluate(&iv,&ov,&mv)?;
    let success=evaluation["verification_state"]=="MatchedFrozenOracle";
    let revision=env::var("EPANET_JUNCTION_HOOK_T0_REVISION").or_else(|_|env::var("GITHUB_SHA")).ok().filter(|x|valid_revision(x));
    let lock_path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock=fs::read(&lock_path).map_err(|e|format!("read {}: {e}",lock_path.display()))?;
    let receipt=json!({
        "schema_version":"epanet-junction-hook-t0-run-v0.1",
        "verification_state":if !success{"OracleMismatch"}else if revision.is_some(){"PassedExactRevision"}else{"PassedLocalUnpinned"},
        "implementation_revision":revision,
        "input_sha256":digest(&ib),"oracle_sha256":digest(&ob),"manifest_sha256":digest(&mb),"cargo_lock_sha256":digest(&lock),
        "case_count":evaluation["case_count"],"matched_case_count":evaluation["matched_case_count"],
        "mismatches":evaluation["mismatches"],"results":evaluation["results"],
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    });
    if let Some(parent)=rp.parent(){if !parent.as_os_str().is_empty(){fs::create_dir_all(parent).map_err(|e|format!("create {}: {e}",parent.display()))?;}}
    let bytes=serde_json::to_string_pretty(&receipt).map_err(|e|format!("serialize receipt: {e}"))?+"\n";
    fs::write(&rp,bytes.as_bytes()).map_err(|e|format!("write {}: {e}",rp.display()))?;
    println!("{bytes}");
    Ok(if success{0}else{2})
}
fn main(){match run(){Ok(code)=>std::process::exit(code),Err(e)=>{eprintln!("EPANET_JUNCTION_HOOK_T0_ERROR: {e}");std::process::exit(1);}}}

#[cfg(test)]
mod tests{
    use super::*;
    fn load(name:&str)->Value{
        let raw=match name{
            "input"=>include_str!("../../../../research/water-intrusion-coupling/epanet-junction-hook-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/epanet-junction-hook-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/epanet-junction-hook-t0-manifest-v0.1.json"),
            _=>panic!("unknown fixture")
        };
        serde_json::from_str(raw).expect("hook fixture parses")
    }
    fn result()->Value{evaluate(&load("input"),&load("oracle"),&load("manifest")).expect("hook T0 runs")}
    fn row(id:&str)->Value{result()["results"].as_array().unwrap().iter().find(|x|x["case_id"]==id).unwrap()["result"].clone()}
    #[test]fn all_ten_snapshot_cases_match_frozen_oracle(){let x=result();assert_eq!(x["case_count"],CASE_COUNT);assert_eq!(x["matched_case_count"],CASE_COUNT);assert_eq!(x["verification_state"],"MatchedFrozenOracle");}
    #[test]fn complete_mix_zero_delay_matches_flow_weighted_concentration(){let x=row("EJH-T0-001-baseline-complete-mix");assert_eq!(x["hook_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");assert_eq!(x["baseline_complete_mix_concentration_kg_per_m3"],0.003);}
    #[test]fn segment_mass_uses_consumed_portion_not_full_segment_volume(){let x=row("EJH-T0-002-multiple-leading-segments");assert!((x["incoming_segment_mass_kg"].as_f64().unwrap()-0.006).abs()<1.0e-12);}
    #[test]fn supported_mass_booster_contribution_is_counted_once(){let x=row("EJH-T0-003-mass-booster");assert_eq!(x["external_source_mass_kg"],0.001);assert_eq!(x["hook_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");}
    #[test]fn concentration_source_volume_and_mass_are_both_counted(){let x=row("EJH-T0-004-concentration-source");assert_eq!(x["external_source_volume_m3"],0.1);assert!((x["external_source_mass_kg"].as_f64().unwrap()-0.0002).abs()<1e-12);}
    #[test]fn unsupported_flow_paced_source_refuses(){assert_eq!(row("EJH-T0-005-flow-paced-source-unsupported")["hook_disposition"],"UNSUPPORTED_SOURCE_SEMANTICS");}
    #[test]fn duplicate_or_unknown_ports_are_rejected(){assert_eq!(row("EJH-T0-006-duplicate-outlet-id")["hook_disposition"],"REJECTED_CONTRACT");}
    #[test]fn missing_port_boundary_has_no_closure_claim(){let x=row("EJH-T0-007-partial-outlet-port-set");assert_eq!(x["hook_disposition"],"PARTIAL_PORT_BOUNDARY");assert!(x["mass_balance_residual_kg"].is_null());}
    #[test]fn open_mass_residual_is_signed_and_not_normalized(){let x=row("EJH-T0-008-open-mass-residual");assert_eq!(x["hook_disposition"],"OPEN_MASS_RESIDUAL");assert!((x["mass_balance_residual_kg"].as_f64().unwrap()-0.002).abs()<1e-12);}
    #[test]fn open_volume_residual_is_separate_from_mass_residual(){assert_eq!(row("EJH-T0-009-open-volume-residual")["hook_disposition"],"OPEN_VOLUME_RESIDUAL");}
    #[test]fn quality_step_cannot_cross_hydraulic_event(){assert_eq!(row("EJH-T0-010-step-over-event")["hook_disposition"],"INVALID_INPUT");}
    #[test]fn zero_flow_outlet_cannot_receive_mass(){let mut x=load("input");x["cases"][0]["snapshot"]["outlet_ports"][0]["volume_m3"]=json!(0.0);assert_eq!(compute(&x["cases"][0]["snapshot"]).unwrap()["hook_disposition"],"INVALID_INPUT");}
    #[test]fn snapshot_mutation_without_digest_update_is_rejected(){
        let mut x=load("input");
        x["cases"][0]["snapshot"]["outlet_ports"][0]["operator_mass_kg"]=json!(0.5);
        let result=compute(&x["cases"][0]["snapshot"]).unwrap();
        assert_eq!(result["hook_disposition"],"REJECTED_CONTRACT");
    }
}
