// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::{collections::BTreeSet, env, fs, path::{Path, PathBuf}};

const PROGRAMME: &str = "WATER-MIXING-DYNAMIC-OPERATOR-001";
const CASE_COUNT: usize = 14;
const EVIDENCE_CEILING: &str = "synthetic discrete causal-kernel arithmetic only";

type Matrix = Vec<Vec<f64>>;
type Tensor3 = Vec<Vec<Vec<f64>>>;

fn field<'a>(v:&'a Value,k:&str)->Result<&'a Value,String>{
    v.get(k).ok_or_else(||format!("missing field: {k}"))
}
fn text<'a>(v:&'a Value,k:&str)->Result<&'a str,String>{
    field(v,k)?.as_str().ok_or_else(||format!("{k} must be text"))
}
fn number(v:&Value,k:&str)->Result<f64,String>{
    field(v,k)?.as_f64().filter(|x|x.is_finite()).ok_or_else(||format!("{k} must be finite numeric"))
}
fn usize_value(v:&Value,k:&str)->Result<usize,String>{
    field(v,k)?.as_u64().and_then(|x|usize::try_from(x).ok()).ok_or_else(||format!("{k} must be a non-negative integer"))
}
fn bool_value(v:&Value,k:&str)->Result<bool,String>{
    field(v,k)?.as_bool().ok_or_else(||format!("{k} must be boolean"))
}
fn array<'a>(v:&'a Value,k:&str)->Result<&'a Vec<Value>,String>{
    field(v,k)?.as_array().ok_or_else(||format!("{k} must be an array"))
}
fn nonnegative(x:f64)->bool{x.is_finite()&&x>=0.0}
fn matrix(v:&Value,key:&str,rows:usize,cols:usize)->Result<Matrix,String>{
    let r=array(v,key)?;
    if r.len()!=rows{return Err(format!("{key} row count mismatch"));}
    let mut out=Vec::with_capacity(rows);
    for row in r{
        let vals=row.as_array().ok_or_else(||format!("{key} row must be an array"))?;
        if vals.len()!=cols{return Err(format!("{key} column count mismatch"));}
        out.push(vals.iter().map(|x|x.as_f64().filter(|n|n.is_finite()).ok_or_else(||format!("{key} contains non-finite value"))).collect::<Result<Vec<_>,_>>()?);
    }
    Ok(out)
}
fn tensor3(v:&Value,key:&str,nout:usize,nin:usize)->Result<Tensor3,String>{
    let outs=array(v,key)?;
    if outs.len()!=nout{return Err(format!("{key} outlet count mismatch"));}
    let mut result=Vec::with_capacity(nout);
    let mut lag_count=None;
    for out in outs{
        let ins=out.as_array().ok_or_else(||format!("{key} outlet row must be array"))?;
        if ins.len()!=nin{return Err(format!("{key} inlet count mismatch"));}
        let mut per_in=Vec::with_capacity(nin);
        for kernel in ins{
            let values=kernel.as_array().ok_or_else(||format!("{key} kernel must be array"))?;
            if values.is_empty(){return Err(format!("{key} kernel cannot be empty"));}
            if lag_count.is_some_and(|n|n!=values.len()){return Err(format!("{key} lag lengths differ"));}
            lag_count=Some(values.len());
            per_in.push(values.iter().map(|x|x.as_f64().filter(|n|n.is_finite()).ok_or_else(||format!("{key} contains non-finite coefficient"))).collect::<Result<Vec<_>,_>>()?);
        }
        result.push(per_in);
    }
    Ok(result)
}
fn convolution(input:&Matrix,kernel:&Tensor3)->Result<Matrix,String>{
    let ntime=input.first().map(Vec::len).unwrap_or(0);
    if ntime==0{return Err("inlet mass-flux series cannot be empty".into());}
    let nout=kernel.len();let nin=input.len();
    if nin==0||kernel.iter().any(|row|row.len()!=nin){return Err("kernel input dimensions mismatch".into());}
    let nlag=kernel[0][0].len();
    let mut output=vec![vec![0.0;ntime+nlag-1];nout];
    for j in 0..nout{for i in 0..nin{for t in 0..ntime{for lag in 0..nlag{
        output[j][t+lag]+=kernel[j][i][lag]*input[i][t];
    }}}}
    if output.iter().flatten().any(|x|!x.is_finite()){return Err("convolution overflow".into());}
    Ok(output)
}
fn convolve_sensor(physical:&Matrix,response:&Matrix)->Result<Matrix,String>{
    if physical.len()!=response.len(){return Err("sensor response outlet count mismatch".into());}
    let nout=physical.len();
    let max_len=physical.iter().map(Vec::len).max().unwrap_or(0);
    let max_lag=response.iter().map(Vec::len).max().unwrap_or(0);
    let mut result=vec![vec![0.0;max_len+max_lag-1];nout];
    for j in 0..nout{
        for t in 0..physical[j].len(){for lag in 0..response[j].len(){
            result[j][t+lag]+=physical[j][t]*response[j][lag];
        }}
    }
    if result.iter().flatten().any(|x|!x.is_finite()){return Err("sensor convolution overflow".into());}
    Ok(result)
}
fn mass(series:&Matrix,dt:f64,horizon:usize)->f64{
    series.iter().map(|row|row.iter().take(horizon).sum::<f64>()).sum::<f64>()*dt
}
fn rows_prefix(series:&Matrix,horizon:usize)->Matrix{
    series.iter().map(|row|row.iter().take(horizon).copied().collect()).collect()
}
fn base(state:&str,reason:&str)->Value{
    json!({
        "operator_disposition":state,"reason":reason,
        "physical_outlet_mass":null,"sensor_observed_outlet_mass":null,
        "known_inventory_mass":null,"signed_mass_residual":null,
        "predicted_physical_tail_mass":null,"predicted_sensor_tail_mass":null,
        "per_inlet_kernel_area":null,"predicted_physical_outlet_flux":null,
        "predicted_sensor_outlet_flux":null,"competing_model_separation":null,
        "model_discrimination_state":"NOT_EVALUATED",
        "physical_operator_qualified":false,"potability_claim_permitted":false,"release_authorized":false
    })
}
fn compute(case:&Value,minimum_separation:f64)->Result<Value,String>{
    let dt=number(case,"sample_interval_seconds")?;
    if dt<=0.0{return Ok(base("INVALID_INPUT","SAMPLE_INTERVAL_NOT_POSITIVE"));}
    let input_rows=array(case,"inlet_mass_flux")?;
    if input_rows.is_empty(){return Ok(base("INVALID_INPUT","NO_INLET_SERIES"));}
    let n_in=input_rows.len();
    let n_time=input_rows[0].as_array().map(Vec::len).unwrap_or(0);
    if n_time==0{return Ok(base("INVALID_INPUT","EMPTY_INLET_SERIES"));}
    let inlet=match matrix(case,"inlet_mass_flux",n_in,n_time){Ok(v)=>v,Err(e)=>return Ok(base("INVALID_INPUT",e.as_str()))};
    if inlet.iter().flatten().any(|x|!nonnegative(*x)){return Ok(base("INVALID_INPUT","NEGATIVE_OR_NONFINITE_INLET_MASS_FLUX"));}
    let kernel_raw=array(case,"kernel")?;
    let n_out=kernel_raw.len();
    if n_out==0{return Ok(base("INVALID_KERNEL","NO_OUTLET_KERNELS"));}
    let kernel=match tensor3(case,"kernel",n_out,n_in){Ok(v)=>v,Err(e)=>return Ok(base("INVALID_KERNEL",e.as_str()))};
    if kernel.iter().flatten().flatten().any(|x|*x<0.0){return Ok(base("INVALID_KERNEL","NEGATIVE_KERNEL_COEFFICIENT"));}
    let mut areas=vec![0.0;n_in];
    for i in 0..n_in{for j in 0..n_out{areas[i]+=kernel[j][i].iter().sum::<f64>();}}
    if areas.iter().any(|x|!x.is_finite()||*x>1.0+1.0e-9){return Ok(base("INVALID_KERNEL","PASSIVE_TRACER_KERNEL_AREA_EXCEEDS_ONE"));}
    let inventory=field(case,"known_inventory")?;
    let inventory_state=text(inventory,"state")?;
    let mut inventory_mass=0.0;
    let mut inventory_issue=None;
    match inventory_state{
        "not_measured"=>{
            if text(inventory,"kind")? != "none"{inventory_issue=Some("UNVERIFIED_INVENTORY_CLASSIFICATION");}
        }
        "measured"=>{
            if text(inventory,"kind")? != "physical_inventory"{
                inventory_issue=Some("MEASUREMENT_SYSTEM_LOSS_IS_NOT_PHYSICAL_INVENTORY");
            }else{
                let unit=text(case,"mass_unit")?;
                if text(inventory,"mass_unit")? != unit{inventory_issue=Some("INVENTORY_UNIT_MISMATCH");}
                else{
                    inventory_mass=number(inventory,"mass")?;
                    if inventory_mass<0.0{return Ok(base("INVALID_INPUT","NEGATIVE_PHYSICAL_INVENTORY"));}
                }
            }
        }
        _=>return Ok(base("REJECTED_CONTRACT","UNKNOWN_INVENTORY_STATE"))
    }
    let input_mass=inlet.iter().map(|row|row.iter().sum::<f64>()).sum::<f64>()*dt;
    if !input_mass.is_finite()||input_mass<=0.0{return Ok(base("INVALID_INPUT","TOTAL_INJECTED_MASS_NOT_POSITIVE"));}
    if text(case,"mass_unit")?.trim().is_empty(){return Ok(base("INVALID_INPUT","MASS_UNIT_EMPTY"));}
    let physical=convolution(&inlet,&kernel)?;
    let physical_mass=mass(&physical,dt,n_time);
    let physical_tail=(mass(&physical,dt,physical[0].len())-physical_mass).max(0.0);
    if !physical_mass.is_finite()||!physical_tail.is_finite(){return Ok(base("INVALID_INPUT","PHYSICAL_MASS_INTEGRAL_NONFINITE"));}
    let sensor_state=text(case,"sensor_response_state")?;
    let mut sensor:Option<Matrix>=None;
    let mut calibrated_response:Option<Matrix>=None;
    if sensor_state=="calibrated"{
        let response_rows=array(case,"sensor_response")?;
        if response_rows.len()!=n_out{return Ok(base("REJECTED_CONTRACT","SENSOR_RESPONSE_OUTLET_COUNT_MISMATCH"));}
        let mut responses=Vec::with_capacity(n_out);
        for row in response_rows{
            let vals=row.as_array().ok_or_else(||"sensor response row must be array".to_owned())?;
            if vals.is_empty(){return Ok(base("INVALID_INPUT","EMPTY_SENSOR_RESPONSE"));}
            let values=vals.iter().map(|x|x.as_f64().filter(|n|n.is_finite()).ok_or_else(||"nonfinite sensor response".to_owned())).collect::<Result<Vec<_>,_>>()?;
            if values.iter().any(|x|*x<0.0){return Ok(base("INVALID_INPUT","NEGATIVE_SENSOR_RESPONSE"));}
            let sum=values.iter().sum::<f64>();
            if (sum-1.0).abs()>1.0e-8{return Ok(base("REJECTED_CONTRACT","SENSOR_RESPONSE_MUST_HAVE_UNIT_AREA"));}
            responses.push(values);
        }
        sensor=Some(convolve_sensor(&physical,&responses)?);
        calibrated_response=Some(responses);
    }else if sensor_state!="uncalibrated"{
        return Ok(base("REJECTED_CONTRACT","UNKNOWN_SENSOR_RESPONSE_STATE"));
    }
    let sensor_mass=sensor.as_ref().map(|x|mass(x,dt,n_time));
    let sensor_tail=sensor.as_ref().map(|x|(mass(x,dt,x[0].len())-mass(x,dt,n_time)).max(0.0));
    if sensor_mass.is_some_and(|x|!x.is_finite())||sensor_tail.is_some_and(|x|!x.is_finite()){
        return Ok(base("INVALID_INPUT","SENSOR_MASS_INTEGRAL_NONFINITE"));
    }
    let sigma=number(case,"mass_residual_standard_uncertainty")?;
    let k=number(case,"closure_sigma_limit")?;
    if sigma<=0.0||k<=0.0{return Ok(base("INVALID_INPUT","MASS_UNCERTAINTY_OR_SIGMA_LIMIT_NOT_POSITIVE"));}
    let residual=sensor_mass.map(|observed_mass|input_mass-observed_mass-inventory_mass);
    if residual.is_some_and(|value|!value.is_finite()){return Ok(base("INVALID_INPUT","MASS_RESIDUAL_NONFINITE"));}
    let tail_tolerance=1.0e-9*input_mass.abs().max(f64::MIN_POSITIVE);
    let all_outlets=bool_value(case,"all_outlets_observed")?;
    let temporal=text(case,"temporal_boundary_state")?;
    let kernel_tail=text(case,"kernel_support_state")?;
    let domain=text(case,"domain_state")?;
    let reversal=bool_value(case,"flow_reversal_present")?;
    let direction_observed=bool_value(case,"flow_direction_observed")?;
    let minimum_resolvable_delay=number(case,"minimum_resolvable_delay_seconds")?;
    let minimum_delay_of_interest=number(case,"minimum_delay_of_interest_seconds")?;
    if minimum_resolvable_delay<=0.0||minimum_delay_of_interest<=0.0
        ||minimum_resolvable_delay+1.0e-12<dt{
        return Ok(base("INVALID_INPUT","TEMPORAL_RESOLUTION_CONTRACT_INVALID"));
    }
    let (state,reason)=if domain!="in_domain"{
        ("OUT_OF_DOMAIN","OPERATING_STATE_OUTSIDE_QUALIFIED_DOMAIN")
    }else if reversal&&!direction_observed{
        ("ABSTAIN_FLOW_DIRECTION_UNOBSERVED","FLOW_REVERSAL_DIRECTION_NOT_MEASURED")
    }else if minimum_resolvable_delay>minimum_delay_of_interest{
        ("TEMPORAL_RESOLUTION_INADEQUATE","SENSOR_BANDWIDTH_OR_SAMPLING_CANNOT_RESOLVE_KERNEL")
    }else if sensor_state!="calibrated"{
        ("SENSOR_RESPONSE_UNRESOLVED","SENSOR_RESPONSE_CALIBRATION_NOT_AVAILABLE")
    }else if let Some(issue)=inventory_issue{
        ("INDETERMINATE",issue)
    }else if !all_outlets{
        ("PARTIAL_OUTLET_BOUNDARY","ONE_OR_MORE_OUTLETS_NOT_OBSERVED")
    }else if temporal!="complete"||kernel_tail!="complete"||physical_tail>tail_tolerance||sensor_tail.unwrap_or(0.0)>tail_tolerance{
        ("PARTIAL_TEMPORAL_WINDOW","KERNEL_OR_SENSOR_TAIL_NOT_FULLY_OBSERVED")
    }else if residual.is_some_and(|value|value.abs()>k*sigma){
        ("OPEN_MASS_RESIDUAL","SIGNED_RESIDUAL_EXCEEDS_DECLARED_UNCERTAINTY")
    }else{
        ("CLOSED_WITHIN_DECLARED_UNCERTAINTY","")
    };
    let per_inlet_area=areas;
    let alt=if case.get("competing_kernel").is_some(){
        let alt_kernel=match tensor3(case,"competing_kernel",n_out,n_in){Ok(v)=>v,Err(e)=>return Ok(base("INVALID_KERNEL",e.as_str()))};
        if alt_kernel.iter().flatten().flatten().any(|x|*x<0.0){return Ok(base("INVALID_KERNEL","NEGATIVE_COMPETING_KERNEL_COEFFICIENT"));}
        let mut alt_area=vec![0.0;n_in];
        for i in 0..n_in{for j in 0..n_out{alt_area[i]+=alt_kernel[j][i].iter().sum::<f64>();}}
        if alt_area.iter().any(|x|*x>1.0+1.0e-9){return Ok(base("INVALID_KERNEL","COMPETING_KERNEL_AREA_EXCEEDS_ONE"));}
        let alternate=convolution(&inlet,&alt_kernel)?;
        let Some(response)=calibrated_response.as_ref() else {
            return Ok(base("SENSOR_RESPONSE_UNRESOLVED","MODEL_DISCRIMINATION_REQUIRES_CALIBRATED_SENSOR_RESPONSE"));
        };
        let alternate_measured=convolve_sensor(&alternate,response)?;
        if text(case,"observation_noise_contract")? != "independent_equal_sigma"{
            return Ok(base("REJECTED_CONTRACT","MODEL_SEPARATION_REQUIRES_DECLARED_IID_NOISE"));
        }
        let Some(observation_sigma)=case.get("per_sample_observation_sigma").and_then(Value::as_f64).filter(|x|x.is_finite()&&*x>0.0) else {
            return Ok(base("INVALID_INPUT","MODEL_SEPARATION_NOISE_SCALE_NOT_POSITIVE"));
        };
        let mut sum=0.0;
        for j in 0..n_out{for t in 0..n_time{
            let d=sensor.as_ref().expect("calibrated sensor response exists")[j][t]-alternate_measured[j][t];
            sum+=(d/observation_sigma).powi(2);
        }}
        Some(sum.sqrt())
    }else{None};
    let discrimination=match alt{
        Some(v) if v>=minimum_separation=>("SEPARATED",Some(v)),
        Some(v)=>("INSUFFICIENT",Some(v)),
        None=>("NOT_EVALUATED",None)
    };
    Ok(json!({
        "operator_disposition":state,"reason":if reason.is_empty(){Value::Null}else{json!(reason)},
        "injected_mass":input_mass,
        "physical_outlet_mass":physical_mass,
        "sensor_observed_outlet_mass":sensor_mass,
        "known_inventory_mass":if inventory_issue.is_some(){Value::Null}else{json!(inventory_mass)},
        "signed_mass_residual":residual,
        "mass_residual_standard_uncertainty":if residual.is_some(){json!(sigma)}else{Value::Null},
        "standardized_abs_mass_residual":residual.map(|value|value.abs()/sigma),
        "predicted_physical_tail_mass":physical_tail,
        "predicted_sensor_tail_mass":sensor_tail,
        "per_inlet_kernel_area":per_inlet_area,
        "predicted_physical_outlet_flux":rows_prefix(&physical,n_time),
        "predicted_sensor_outlet_flux":sensor.as_ref().map(|x|rows_prefix(x,n_time)),
        "competing_model_separation":discrimination.1,
        "model_discrimination_state":discrimination.0,
        "physical_operator_qualified":false,
        "potability_claim_permitted":false,
        "release_authorized":false
    }))
}
fn validate(input:&Value,oracle:&Value,manifest:&Value)->Result<(),String>{
    if text(input,"schema_version")? != "dynamic-kernel-t0-input-v0.1"{return Err("unexpected input schema".into());}
    if text(oracle,"schema_version")? != "dynamic-kernel-t0-oracle-v0.1"{return Err("unexpected oracle schema".into());}
    for v in [input,oracle,manifest]{if text(v,"programme_id")? != PROGRAMME{return Err("programme identity mismatch".into());}}
    for k in ["physical_operator_qualified","potability_claim_permitted","release_authorized"]{
        if field(oracle,k)?.as_bool()!=Some(false){return Err(format!("oracle cannot set {k} true"));}
    }
    let cases=array(input,"cases")?;let expected=array(oracle,"cases")?;let ids=array(manifest,"case_ids")?;
    if cases.len()!=CASE_COUNT||expected.len()!=CASE_COUNT||usize_value(manifest,"case_count")? != CASE_COUNT{
        return Err("frozen dynamic-kernel case count mismatch".into());
    }
    let collect=|rows:&[Value],key:&str|->Result<BTreeSet<String>,String>{
        rows.iter().map(|v|text(v,key).map(str::to_owned)).collect()
    };
    let input_ids=collect(cases,"case_id")?;
    let oracle_ids=collect(expected,"case_id")?;
    let manifest_ids=ids.iter().map(|x|x.as_str().map(str::to_owned).ok_or_else(||"manifest case id must be text".to_owned())).collect::<Result<BTreeSet<_>,_>>()?;
    if input_ids.len()!=cases.len()||oracle_ids.len()!=expected.len()||manifest_ids.len()!=ids.len(){return Err("duplicate dynamic-kernel case identity".into());}
    if input_ids!=oracle_ids||input_ids!=manifest_ids{return Err("input/oracle/manifest identities differ".into());}
    Ok(())
}
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
    let min_sep=number(input,"minimum_model_separation")?;
    if min_sep<0.0{return Err("minimum model separation cannot be negative".into());}
    let expected=array(oracle,"cases")?;
    let mut expected_by_id=std::collections::BTreeMap::new();
    for x in expected{expected_by_id.insert(text(x,"case_id")?.to_owned(),field(x,"expected")?.clone());}
    let mut results=Vec::new();let mut mismatches=Vec::new();
    for case in array(input,"cases")?{
        let id=text(case,"case_id")?;
        let actual=compute(field(case,"inputs")?,min_sep)?;
        let expected=expected_by_id.get(id).ok_or_else(||format!("oracle missing {id}"))?;
        if !compare(&actual,expected){mismatches.push(json!({"case_id":id,"expected":expected,"actual":actual}));}
        results.push(json!({"case_id":id,"matched":compare(&actual,expected),"result":actual}));
    }
    results.sort_by(|a,b|a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched=results.iter().filter(|x|x["matched"]==true).count();
    Ok(json!({
        "schema_version":"dynamic-kernel-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty(){"MatchedFrozenOracle"}else{"OracleMismatch"},
        "evidence_class":"synthetic_only","evidence_ceiling":EVIDENCE_CEILING,
        "case_count":results.len(),"matched_case_count":matched,
        "mismatches":mismatches,"results":results,
        "nonclaims":[
            "The discrete causal kernel is a reduced-order arithmetic fixture, not a fluid solver.",
            "A predicted delay is not physical local memory unless sensor response, sampling and input waveform are independently qualified.",
            "Kernel area below one is not renormalized; its mass remains inventory or unresolved residual.",
            "No result establishes physical mixing, sensor calibration, tracer compatibility, potability or release authority."
        ]
    }))
}
fn digest(bytes:&[u8])->String{Sha256::digest(bytes).iter().map(|b|format!("{b:02x}")).collect()}
fn read_json(p:&Path)->Result<(Vec<u8>,Value),String>{
    let b=fs::read(p).map_err(|e|format!("read {}: {e}",p.display()))?;
    let v=serde_json::from_slice(&b).map_err(|e|format!("parse {}: {e}",p.display()))?;
    Ok((b,v))
}
fn valid_revision(s:&str)->bool{s.len()==40&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn run()->Result<i32,String>{
    let args:Vec<String>=env::args().skip(1).collect();
    if args.len()>4{return Err("usage: dynamic-kernel-t0 [INPUT] [ORACLE] [MANIFEST] [RECEIPT]".into());}
    let ip=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/dynamic-kernel-t0-cases-v0.1.json"));
    let op=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/dynamic-kernel-t0-oracle-v0.1.json"));
    let mp=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/dynamic-kernel-t0-manifest-v0.1.json"));
    let rp=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-mixing-dynamic-kernel-t0/receipt.json"));
    let (ib,iv)=read_json(&ip)?;let (ob,ov)=read_json(&op)?;let (mb,mv)=read_json(&mp)?;
    let evaluation=evaluate(&iv,&ov,&mv)?;
    let success=evaluation["verification_state"]=="MatchedFrozenOracle";
    let revision=env::var("WATER_MIXING_DYNAMIC_T0_REVISION").or_else(|_|env::var("GITHUB_SHA")).ok().filter(|x|valid_revision(x));
    let lock_path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock=fs::read(&lock_path).map_err(|e|format!("read {}: {e}",lock_path.display()))?;
    let receipt=json!({
        "schema_version":"dynamic-kernel-t0-run-v0.1",
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
fn main(){match run(){Ok(c)=>std::process::exit(c),Err(e)=>{eprintln!("WATER_MIXING_DYNAMIC_T0_ERROR: {e}");std::process::exit(1);}}}

#[cfg(test)]
mod tests{
    use super::*;
    fn load(name:&str)->Value{
        let raw=match name{
            "input"=>include_str!("../../../../research/water-intrusion-coupling/dynamic-kernel-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/dynamic-kernel-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/dynamic-kernel-t0-manifest-v0.1.json"),
            _=>panic!("unknown fixture")
        };
        serde_json::from_str(raw).expect("dynamic kernel fixture parses")
    }
    fn result()->Value{evaluate(&load("input"),&load("oracle"),&load("manifest")).expect("dynamic kernel T0 runs")}
    fn row(id:&str)->Value{result()["results"].as_array().unwrap().iter().find(|x|x["case_id"]==id).unwrap()["result"].clone()}
    #[test]fn all_thirteen_cases_match_the_frozen_oracle(){let x=result();assert_eq!(x["case_count"],CASE_COUNT);assert_eq!(x["matched_case_count"],CASE_COUNT);assert_eq!(x["verification_state"],"MatchedFrozenOracle");}
    #[test]fn static_zero_delay_kernel_is_the_steady_transfer_limit(){let x=row("DK-T0-001-static-zero-delay-limit");assert_eq!(x["operator_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");assert_eq!(x["predicted_physical_outlet_flux"],json!([[0.7,0.0,0.0,0.0],[0.3,0.0,0.0,0.0]]));}
    #[test]fn delayed_kernel_preserves_outlet_specific_response(){let x=row("DK-T0-002-delayed-outlet-specific-response");assert_eq!(x["operator_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");assert_eq!(x["predicted_physical_outlet_flux"],json!([[0.0,0.0,0.4,0.0,0.0],[0.0,0.6,0.0,0.0,0.0]]));}
    #[test]fn sub_stochastic_kernel_keeps_unresolved_mass(){let x=row("DK-T0-003-substochastic-kernel-open-residual");assert_eq!(x["operator_disposition"],"OPEN_MASS_RESIDUAL");assert!((x["signed_mass_residual"].as_f64().unwrap()-0.2).abs()<1.0e-12);}
    #[test]fn truncated_tail_is_not_reported_as_closed(){assert_eq!(row("DK-T0-004-truncated-temporal-window")["operator_disposition"],"PARTIAL_TEMPORAL_WINDOW");}
    #[test]fn independently_measured_inventory_is_accounted_separately(){let x=row("DK-T0-005-measured-physical-inventory");assert_eq!(x["operator_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");assert_eq!(x["known_inventory_mass"],0.2);}
    #[test]fn uncalibrated_sensor_lag_does_not_become_physical_delay(){let x=row("DK-T0-006-sensor-response-uncalibrated");assert_eq!(x["operator_disposition"],"SENSOR_RESPONSE_UNRESOLVED");assert!(x["sensor_observed_outlet_mass"].is_null());assert!(x["signed_mass_residual"].is_null());}
    #[test]fn calibrated_sensor_response_preserves_mass_and_time_series(){let x=row("DK-T0-007-calibrated-sensor-lag");assert_eq!(x["operator_disposition"],"CLOSED_WITHIN_DECLARED_UNCERTAINTY");assert_eq!(x["predicted_physical_outlet_flux"],json!([[0.2,0.2,0.0,0.0,0.0],[0.0,0.2,0.2,0.0,0.0]]));assert_eq!(x["predicted_sensor_outlet_flux"],json!([[0.1,0.2,0.1,0.0,0.0],[0.0,0.1,0.2,0.1,0.0]]));}
    #[test]fn flow_reversal_without_direction_observation_abstains(){assert_eq!(row("DK-T0-008-flow-reversal-unobserved")["operator_disposition"],"ABSTAIN_FLOW_DIRECTION_UNOBSERVED");}
    #[test]fn out_of_domain_operator_refuses_extrapolation(){assert_eq!(row("DK-T0-009-out-of-domain")["operator_disposition"],"OUT_OF_DOMAIN");}
    #[test]fn negative_kernel_coefficients_are_refused(){assert_eq!(row("DK-T0-010-negative-kernel-coefficient")["operator_disposition"],"INVALID_KERNEL");}
    #[test]fn same_integrated_gain_can_have_distinguishable_transient_responses(){let x=row("DK-T0-011-same_steady_gain_different_delay");assert_eq!(x["model_discrimination_state"],"SEPARATED");assert!(x["competing_model_separation"].as_f64().unwrap()>1.0);}
    #[test]fn inadequate_sampling_resolution_is_explicit(){assert_eq!(row("DK-T0-012-temporal-resolution-inadequate")["operator_disposition"],"TEMPORAL_RESOLUTION_INADEQUATE");}
    #[test]fn incomplete_outlet_observation_is_not_a_complete_operator(){assert_eq!(row("DK-T0-013-partial-outlet-boundary")["operator_disposition"],"PARTIAL_OUTLET_BOUNDARY");}
    #[test]fn kernel_area_above_one_is_rejected(){let mut x=load("input");x["cases"][0]["inputs"]["kernel"][0][0][0]=json!(1.2);assert_eq!(compute(&x["cases"][0]["inputs"],0.1).unwrap()["operator_disposition"],"INVALID_KERNEL");}
    #[test]fn positive_mass_rescaling_preserves_kernel_disposition(){
        let base=row("DK-T0-001-static-zero-delay-limit");
        let scaled=row("DK-T0-014-positive-mass-time-scale-invariance");
        assert_eq!(base["operator_disposition"],scaled["operator_disposition"]);
        assert_eq!(base["per_inlet_kernel_area"],scaled["per_inlet_kernel_area"]);
        assert!((scaled["injected_mass"].as_f64().unwrap()-10.0*base["injected_mass"].as_f64().unwrap()).abs()<1.0e-9);
        assert!((scaled["mass_residual_standard_uncertainty"].as_f64().unwrap()-10.0*base["mass_residual_standard_uncertainty"].as_f64().unwrap()).abs()<1.0e-9);
    }
    #[test]fn temporal_resolution_is_derived_from_declared_sensor_limits(){
        let x=load("input");
        let c=x["cases"].as_array().unwrap().iter().find(|v|v["case_id"]=="DK-T0-012-temporal-resolution-inadequate").unwrap();
        let result=compute(&c["inputs"],1.0).unwrap();
        assert_eq!(result["operator_disposition"],"TEMPORAL_RESOLUTION_INADEQUATE");
    }
    #[test]fn transient_model_separation_requires_an_explicit_noise_contract(){
        let mut x=load("input");
        let c=x["cases"].as_array_mut().unwrap().iter_mut().find(|v|v["case_id"]=="DK-T0-011-same-steady-gain-different-delay").unwrap();
        c["inputs"].as_object_mut().unwrap().remove("observation_noise_contract");
        let result=compute(&c["inputs"],1.0).unwrap();
        assert_eq!(result["operator_disposition"],"REJECTED_CONTRACT");
    }
    #[test]fn inconsistent_kernel_lag_lengths_are_refused(){
        let mut x=load("input");
        x["cases"][0]["inputs"]["kernel"][1][0]=json!([0.3,0.0]);
        let result=compute(&x["cases"][0]["inputs"],1.0).unwrap();
        assert_eq!(result["operator_disposition"],"INVALID_KERNEL");
    }
}
