use serde_json::{json,Value};
use std::{collections::{BTreeMap,BTreeSet},env,fs,process::ExitCode};

const VERSION:&str="0.1.0";
const FIXTURE:&str=include_str!("../../../benchmarks/maritime-behavioral-set/trajectory-corpus-v0.1.json");

#[derive(Clone,Debug)]
struct Tube{
 id:String,principle:String,group:String,unit:String,deps:BTreeSet<String>,timebase:String,
 grid:Vec<i64>,uncertainty:Vec<f64>,slope:Vec<f64>,lower:Vec<f64>,upper:Vec<f64>
}
#[derive(Clone,Debug)]
struct PrincipleResult{
 name:String,status:String,marginal:Option<bool>,margin:Option<f64>,group:String,deps:BTreeSet<String>,cross_pairs:usize
}
fn txt<'a>(v:&'a Value,k:&str)->Option<&'a str>{v.get(k).and_then(Value::as_str).map(str::trim).filter(|s|!s.is_empty())}
fn num(v:&Value)->Option<f64>{v.as_f64().filter(|x|x.is_finite())}
fn sha(v:&Value,k:&str)->bool{v.get(k).and_then(Value::as_str).is_some_and(|s|s.len()==64&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)))}
fn arr_num(v:&Value,k:&str)->Result<Vec<f64>,String>{
 let a=v.get(k).and_then(Value::as_array).ok_or_else(||format!("{k}: expected array"))?;
 a.iter().enumerate().map(|(i,x)|num(x).ok_or_else(||format!("{k}[{i}]: expected finite number"))).collect()
}
fn arr_time(v:&Value,k:&str)->Result<Vec<i64>,String>{
 let a=v.get(k).and_then(Value::as_array).ok_or_else(||format!("{k}: expected array"))?;
 a.iter().enumerate().map(|(i,x)|x.as_i64().ok_or_else(||format!("{k}[{i}]: expected integer milliseconds"))).collect()
}
fn string_set(v:&Value,k:&str)->Result<BTreeSet<String>,String>{
 let a=v.get(k).and_then(Value::as_array).ok_or_else(||format!("{k}: expected array"))?;
 let mut out=BTreeSet::new();
 for (i,x) in a.iter().enumerate(){let s=x.as_str().filter(|s|!s.trim().is_empty()).ok_or_else(||format!("{k}[{i}]: expected text"))?;if !out.insert(s.to_owned()){return Err(format!("{k}[{i}]: duplicate"));}}
 Ok(out)
}
fn exact(v:&Value,keys:&[&str],where_:&str)->Result<(),String>{
 let m=v.as_object().ok_or_else(||format!("{where_}: expected object"))?;
 let allowed:BTreeSet<&str>=keys.iter().copied().collect();
 for k in keys{if !m.contains_key(*k){return Err(format!("{where_}: missing {k}"));}}
 for k in m.keys(){if !allowed.contains(k.as_str()){return Err(format!("{where_}: unknown key {k}"));}}
 Ok(())
}
fn parse_tube(v:&Value,where_:&str,case_timebase:&str,max_unc:f64)->Result<Tube,String>{
 exact(v,&["tube_id","measurement_principle","independence_group_id","value_unit","dependency_groups","timebase_id","sample_offsets_ms","timestamp_uncertainty_ms","maximum_abs_slope_per_ms","value_lower","value_upper","evidence_role","artifact_sha256"],where_)?;
 let id=txt(v,"tube_id").ok_or_else(||format!("{where_}: tube_id missing"))?.to_owned();
 let principle=txt(v,"measurement_principle").ok_or_else(||format!("{where_}: measurement_principle missing"))?.to_owned();
 let group=txt(v,"independence_group_id").ok_or_else(||format!("{where_}: independence_group_id missing"))?.to_owned();
 let unit=txt(v,"value_unit").ok_or_else(||format!("{where_}: value_unit missing"))?.to_owned();
 let timebase=txt(v,"timebase_id").ok_or_else(||format!("{where_}: timebase_id missing"))?.to_owned();
 if timebase!=case_timebase{return Err(format!("{where_}: tube timebase mismatch"));}
 if txt(v,"evidence_role")!=Some("SYNTHETIC_CONFORMANCE_FIXTURE"){return Err(format!("{where_}: v0.1 accepts synthetic fixtures only"));}
 if !sha(v,"artifact_sha256"){return Err(format!("{where_}: invalid artifact SHA-256"));}
 let deps=string_set(v,"dependency_groups")?;if deps.is_empty(){return Err(format!("{where_}: dependency_groups must not be empty"));}
 let grid=arr_time(v,"sample_offsets_ms")?;
 let uncertainty=arr_num(v,"timestamp_uncertainty_ms")?;
 let slope=arr_num(v,"maximum_abs_slope_per_ms")?;
 let lower=arr_num(v,"value_lower")?;let upper=arr_num(v,"value_upper")?;
 let n=grid.len();
 if n<2||grid.windows(2).any(|w|w[0]>=w[1]){return Err(format!("{where_}: sample grid must contain >=2 strictly increasing offsets; no DTW or reordering"));}
 if [uncertainty.len(),slope.len(),lower.len(),upper.len()].iter().any(|x|*x!=n){return Err(format!("{where_}: trajectory vector lengths differ"));}
 if uncertainty.iter().any(|x|*x<0.0||*x>max_unc){return Err(format!("{where_}: timestamp uncertainty outside case bound"));}
 if slope.iter().any(|x|*x<0.0){return Err(format!("{where_}: negative maximum slope bound"));}
 if lower.iter().zip(&upper).any(|(l,u)|l>u){return Err(format!("{where_}: lower bound exceeds upper bound"));}
 Ok(Tube{id,principle,group,unit,deps,timebase,grid,uncertainty,slope,lower,upper})
}
fn expanded(t:&Tube)->(Vec<f64>,Vec<f64>){
 let lo=t.lower.iter().enumerate().map(|(i,x)|x-t.slope[i]*t.uncertainty[i]).collect();
 let hi=t.upper.iter().enumerate().map(|(i,x)|x+t.slope[i]*t.uncertainty[i]).collect();(lo,hi)
}
fn tube_gap(a:&Tube,b:&Tube)->f64{
 let (al,au)=expanded(a);let (bl,bu)=expanded(b);let mut max_gap=0.0f64;
 for i in 0..al.len(){let gap=if au[i]<bl[i]{bl[i]-au[i]}else if bu[i]<al[i]{al[i]-bu[i]}else{0.0};max_gap=max_gap.max(gap);}
 max_gap
}
fn marginal_separated(a:&[&Tube],b:&[&Tube])->bool{
 if a.is_empty()||b.is_empty(){return false;}
 let n=a[0].grid.len();
 for i in 0..n{
  let mut al=f64::INFINITY;let mut au=f64::NEG_INFINITY;let mut bl=f64::INFINITY;let mut bu=f64::NEG_INFINITY;
  for t in a{let (l,u)=expanded(t);al=al.min(l[i]);au=au.max(u[i]);}
  for t in b{let (l,u)=expanded(t);bl=bl.min(l[i]);bu=bu.max(u[i]);}
  if au<bl||bu<al{return true;}
 }
 false
}
fn principle_result(name:&str,a:&[&Tube],b:&[&Tube],case_timebase:&str)->PrincipleResult{
 if a.is_empty()||b.is_empty(){return PrincipleResult{name:name.into(),status:"NOT_EVALUATED".into(),marginal:None,margin:None,group:String::new(),deps:BTreeSet::new(),cross_pairs:0};}
 let grids:BTreeSet<Vec<i64>>=a.iter().chain(b).map(|t|t.grid.clone()).collect();
 let timebases:BTreeSet<String>=a.iter().chain(b).map(|t|t.timebase.clone()).collect();
 let groups:BTreeSet<String>=a.iter().chain(b).map(|t|t.group.clone()).collect();
 let units:BTreeSet<String>=a.iter().chain(b).map(|t|t.unit.clone()).collect();
 if grids.len()!=1||timebases.len()!=1||timebases.iter().next().map(String::as_str)!=Some(case_timebase)||groups.len()!=1||units.len()!=1{
  return PrincipleResult{name:name.into(),status:"NOT_EVALUATED".into(),marginal:None,margin:None,group:String::new(),deps:BTreeSet::new(),cross_pairs:0};
 }
 let mut deps=BTreeSet::new();for t in a.iter().chain(b){deps.extend(t.deps.iter().cloned());}
 let mut robust=f64::INFINITY;let mut cross=0usize;let mut all_disjoint=true;
 for ta in a{for tb in b{cross+=1;let gap=tube_gap(ta,tb);robust=robust.min(gap);if gap<=0.0{all_disjoint=false;}}}
 PrincipleResult{name:name.into(),status:if all_disjoint{"DISJOINT"}else{"OVERLAPPING"}.into(),
  marginal:Some(marginal_separated(a,b)),margin:Some(if robust.is_finite(){robust}else{0.0}),
  group:groups.into_iter().next().unwrap_or_default(),deps,cross_pairs:cross}
}
fn evaluate_pair(pair:&Value,hyp:&BTreeMap<String,Vec<Tube>>,timebase:&Value)->Value{
 let id=txt(pair,"pair_id").unwrap_or("unknown");let fa=txt(pair,"fault_a").unwrap_or("");let fb=txt(pair,"fault_b").unwrap_or("");
 let min=pair["minimum_independent_principles"].as_u64().unwrap_or(1) as usize;
 let tb=txt(timebase,"timebase_id").unwrap_or("");
 if txt(timebase,"qualification_state")!=Some("qualified")||!sha(timebase,"qualification_sha256"){
  return json!({"pair_id":id,"status":"NOT_EVALUATED","reason_codes":["TIMEBASE_NOT_QUALIFIED"],"independent_disjoint_principle_count":0,"minimum_independent_principles":min,"marginal_envelope_separated":null,"principles":[]});
 }
 let Some(a_tubes)=hyp.get(fa)else{return json!({"pair_id":id,"status":"NOT_EVALUATED","reason_codes":["FAULT_A_TUBES_MISSING"],"principles":[]});};
 let Some(b_tubes)=hyp.get(fb)else{return json!({"pair_id":id,"status":"NOT_EVALUATED","reason_codes":["FAULT_B_TUBES_MISSING"],"principles":[]});};
 let names:BTreeSet<String>=a_tubes.iter().chain(b_tubes).map(|t|t.principle.clone()).collect();
 let mut results=Vec::new();
 for name in names{let a:Vec<&Tube>=a_tubes.iter().filter(|t|t.principle==name).collect();let b:Vec<&Tube>=b_tubes.iter().filter(|t|t.principle==name).collect();results.push(principle_result(&name,&a,&b,tb));}
 let mut accepted_deps=BTreeSet::new();let mut accepted_groups=BTreeSet::new();let mut count=0usize;let mut overlap=0usize;let mut marginal=false;let mut robust=f64::INFINITY;
 for r in &results{
  if r.marginal==Some(true){marginal=true;}
  if r.status=="OVERLAPPING"{overlap+=1;}
  if r.status=="DISJOINT"{
   let dependency_overlap=r.deps.iter().any(|d|accepted_deps.contains(d));
   if !dependency_overlap&&accepted_groups.insert(r.group.clone()){count+=1;robust=robust.min(r.margin.unwrap_or(0.0));accepted_deps.extend(r.deps.iter().cloned());}
  }
 }
 let status=if count>=min{"BEHAVIORALLY_SEPARABLE"}else if results.iter().all(|r|r.status=="NOT_EVALUATED"){"NOT_EVALUATED"}else if overlap>0{"BEHAVIORALLY_AMBIGUOUS"}else{"INSUFFICIENT_INDEPENDENT_PRINCIPLES"};
 let mut reasons=Vec::new();if overlap>0{reasons.push("TRAJECTORY_SET_OVERLAP");}if count<min&&results.iter().any(|r|r.status=="DISJOINT"){reasons.push("INDEPENDENT_PRINCIPLE_THRESHOLD_NOT_MET");}if results.iter().any(|r|r.status=="NOT_EVALUATED"){reasons.push("ONE_OR_MORE_PRINCIPLES_NOT_EVALUATED");}
 json!({"pair_id":id,"status":status,"reason_codes":reasons,"independent_disjoint_principle_count":count,
  "minimum_independent_principles":min,"marginal_envelope_separated":if results.iter().any(|r|r.marginal.is_some()){json!(marginal)}else{Value::Null},
  "minimum_robust_trajectory_separation_margin":if robust.is_finite(){json!(robust)}else{Value::Null},
  "principles":results.iter().map(|r|json!({"measurement_principle":r.name.clone(),"status":r.status.clone(),"marginal_envelope_separated":r.marginal,"robust_margin":r.margin,"independence_group_id":r.group.clone(),"dependency_groups":r.deps.clone(),"cross_hypothesis_tube_pairs":r.cross_pairs})).collect::<Vec<_>>(),
  "claim_boundary":"Finite synthetic trajectory-tube comparison only; no physical fault diagnosis or vessel qualification is established."})
}
fn validate_corpus(corpus:&Value)->Result<Value,String>{
 exact(corpus,&["schema_version","corpus_id","evidence_class","cases","authority_boundary","claim_ceiling"],"$")?;
 if txt(corpus,"schema_version")!=Some(VERSION)||txt(corpus,"evidence_class")!=Some("SYNTHETIC_KNOWN_TRUTH"){return Err("unsupported schema version or evidence class".into());}
 let auth=&corpus["authority_boundary"];if auth["read_only"].as_bool()!=Some(true){return Err("read_only must be true".into());}
 for k in ["controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"]{if auth[k].as_bool()!=Some(false){return Err(format!("authority boundary violation: {k}"));}}
 let cases=corpus["cases"].as_array().ok_or("cases must be array")?;if cases.is_empty()||cases.len()>1000{return Err("case count outside 1..=1000".into());}
 let mut case_ids=BTreeSet::new();let mut receipts=Vec::new();let mut pair_total=0usize;
 for (ci,c) in cases.iter().enumerate(){
  let path=format!("$.cases[{ci}]");exact(c,&["case_id","configuration_id","operating_profile_id","timebase","maximum_timestamp_uncertainty_ms","hypotheses","required_pairs"],&path)?;
  let case_id=txt(c,"case_id").ok_or("case_id missing")?;if !case_ids.insert(case_id.to_owned()){return Err(format!("{path}: duplicate case ID"));}
  let max_unc=num(&c["maximum_timestamp_uncertainty_ms"]).filter(|x|*x>=0.0).ok_or("maximum timestamp uncertainty invalid")?;
  let tb=&c["timebase"];exact(tb,&["timebase_id","qualification_state","qualification_sha256"],&format!("{path}.timebase"))?;
  if txt(tb,"timebase_id").is_none()||!sha(tb,"qualification_sha256"){return Err(format!("{path}: timebase identity/digest invalid"));}
  let hs=c["hypotheses"].as_array().ok_or("hypotheses must be array")?;if hs.len()<2{return Err(format!("{path}: at least two hypotheses required"));}
  let mut hypotheses:BTreeMap<String,Vec<Tube>>=BTreeMap::new();let mut tube_ids=BTreeSet::new();let mut tube_total=0usize;
  for (hi,h) in hs.iter().enumerate(){
   let hp=format!("{path}.hypotheses[{hi}]");exact(h,&["fault_id","trajectory_tubes"],&hp)?;
   let fault=txt(h,"fault_id").ok_or("fault_id missing")?.to_owned();if hypotheses.contains_key(&fault){return Err(format!("{hp}: duplicate fault ID"));}
   let list=h["trajectory_tubes"].as_array().ok_or("trajectory_tubes must be array")?;if list.is_empty(){return Err(format!("{hp}: no trajectory tubes"));}
   let mut tubes=Vec::new();
   for (ti,t) in list.iter().enumerate(){tube_total+=1;let parsed=parse_tube(t,&format!("{hp}.trajectory_tubes[{ti}]"),txt(tb,"timebase_id").unwrap(),max_unc)?;if !tube_ids.insert(parsed.id.clone()){return Err(format!("{hp}: duplicate tube ID"));}tubes.push(parsed);}
   hypotheses.insert(fault,tubes);
  }
  if tube_total>4096{return Err(format!("{path}: more than 4096 tubes"));}
  let pairs=c["required_pairs"].as_array().ok_or("required_pairs must be array")?;if pairs.is_empty(){return Err(format!("{path}: required pairs missing"));}
  let mut pair_ids=BTreeSet::new();let mut results=Vec::new();
  for p in pairs{
   exact(p,&["pair_id","fault_a","fault_b","minimum_independent_principles","expected_status","expected_marginal_envelope_separated"],"required_pair")?;
   let id=txt(p,"pair_id").ok_or("pair_id missing")?;if !pair_ids.insert(id.to_owned()){return Err("duplicate pair ID".into());}
   let a=txt(p,"fault_a").ok_or("fault_a missing")?;let b=txt(p,"fault_b").ok_or("fault_b missing")?;
   if a==b||!hypotheses.contains_key(a)||!hypotheses.contains_key(b){return Err(format!("pair {id}: hypothesis refs invalid"));}
   let min=p["minimum_independent_principles"].as_u64().filter(|x|*x>0).ok_or("minimum independent principles must be positive")?;
   let r=evaluate_pair(p,&hypotheses,tb);
   if r["status"].as_str()!=txt(p,"expected_status"){return Err(format!("pair {id}: expected {}, got {}",txt(p,"expected_status").unwrap_or(""),r["status"]));}
   if !p["expected_marginal_envelope_separated"].is_null()&&r["marginal_envelope_separated"]!=p["expected_marginal_envelope_separated"]{return Err(format!("pair {id}: marginal baseline mismatch"));}
   if min as usize>tube_total{return Err(format!("pair {id}: impossible principle threshold"));}results.push(r);
  }
  pair_total+=results.len();
  receipts.push(json!({"case_id":case_id,"configuration_id":txt(c,"configuration_id"),"operating_profile_id":txt(c,"operating_profile_id"),"pair_results":results}));
 }
 Ok(json!({"result":"pass","validator_version":VERSION,"corpus_id":txt(corpus,"corpus_id"),"case_count":cases.len(),"pair_count":pair_total,"cases":receipts,
  "claim_ceiling":txt(corpus,"claim_ceiling").unwrap_or("")}))
}
fn run(path:&str,source:&str,digest:&str)->Result<Value,String>{
 if source.len()!=40||!source.bytes().all(|b|b.is_ascii_hexdigit())||digest.len()!=64||!digest.bytes().all(|b|b.is_ascii_hexdigit()){return Err("invalid source commit/corpus digest".into());}
 let raw=fs::read_to_string(path).map_err(|e|format!("read corpus: {e}"))?;let c:Value=serde_json::from_str(&raw).map_err(|e|format!("parse corpus: {e}"))?;
 let mut r=validate_corpus(&c)?;r["source_commit"]=json!(source);r["corpus_sha256"]=json!(digest);Ok(r)
}
fn main()->ExitCode{
 let a:Vec<String>=env::args().collect();if a.len()!=6||a[2]!="--source-commit"||a[4]!="--corpus-sha256"{eprintln!("usage: evaluator <corpus.json> --source-commit <40-hex> --corpus-sha256 <64-hex>");return ExitCode::from(2);}
 match run(&a[1],&a[3],&a[5]){Ok(r)=>{println!("{}",serde_json::to_string_pretty(&r).unwrap_or_else(|_|"{\"result\":\"fail\"}".into()));ExitCode::SUCCESS},Err(e)=>{eprintln!("behavioral-set benchmark failed closed: {e}");ExitCode::FAILURE}}
}
#[cfg(test)]
mod tests{
 use super::*;
 fn fixture()->Value{serde_json::from_str(FIXTURE).unwrap()}
 fn hypotheses(case:&Value)->BTreeMap<String,Vec<Tube>>{
  let timebase=txt(&case["timebase"],"timebase_id").unwrap();let max=num(&case["maximum_timestamp_uncertainty_ms"]).unwrap();let mut out=BTreeMap::new();
  for h in case["hypotheses"].as_array().unwrap(){let mut tubes=Vec::new();for t in h["trajectory_tubes"].as_array().unwrap(){tubes.push(parse_tube(t,"test",timebase,max).unwrap());}out.insert(txt(h,"fault_id").unwrap().to_owned(),tubes);}out
 }
 fn pair(c:&Value,i:usize)->&Value{&c["cases"][0]["required_pairs"][i]}
 #[test]fn all_frozen_cases_match_expected(){let r=validate_corpus(&fixture()).unwrap();assert_eq!(r["case_count"],7);assert_eq!(r["pair_count"],7);}
 #[test]fn temporal_correlation_beats_marginal_envelope(){let c=fixture();let case=&c["cases"][0];let r=evaluate_pair(pair(&c,0),&hypotheses(case),&case["timebase"]);assert_eq!(r["status"],"BEHAVIORALLY_SEPARABLE");assert_eq!(r["marginal_envelope_separated"],false);}
 #[test]fn one_overlapping_tube_pair_blocks_unique_isolation(){let c=fixture();let case=&c["cases"][0];let r=evaluate_pair(pair(&c,1),&hypotheses(case),&case["timebase"]);assert_eq!(r["status"],"BEHAVIORALLY_AMBIGUOUS");}
 #[test]fn timestamp_uncertainty_expands_tubes_and_erases_separation(){let c=fixture();let case=&c["cases"][0];let r=evaluate_pair(pair(&c,2),&hypotheses(case),&case["timebase"]);assert_eq!(r["status"],"BEHAVIORALLY_AMBIGUOUS");}
 #[test]fn unqualified_timebase_is_not_evaluated(){let c=fixture();let case=&c["cases"][0];let r=evaluate_pair(pair(&c,3),&hypotheses(case),&case["timebase"]);assert_eq!(r["status"],"NOT_EVALUATED");}
 #[test]fn mismatched_sample_grid_is_not_time_warped(){let c=fixture();let case=&c["cases"][0];let r=evaluate_pair(pair(&c,4),&hypotheses(case),&case["timebase"]);assert_eq!(r["status"],"NOT_EVALUATED");}
 #[test]fn common_cause_does_not_count_as_two_independent_principles(){let c=fixture();let case=&c["cases"][0];let r=evaluate_pair(pair(&c,5),&hypotheses(case),&case["timebase"]);assert_eq!(r["status"],"INSUFFICIENT_INDEPENDENT_PRINCIPLES");}
 #[test]fn post_hoc_phase_alignment_is_not_permitted(){let c=fixture();let case=&c["cases"][0];let r=evaluate_pair(pair(&c,6),&hypotheses(case),&case["timebase"]);assert_eq!(r["status"],"NOT_EVALUATED");}
 #[test]fn nonmonotonic_sample_grid_is_rejected(){let mut c=fixture();c["cases"][0]["hypotheses"][0]["trajectory_tubes"][0]["sample_offsets_ms"]=json!([0,100,50]);assert!(validate_corpus(&c).is_err());}
 #[test]fn invalid_interval_is_rejected(){let mut c=fixture();c["cases"][0]["hypotheses"][0]["trajectory_tubes"][0]["value_lower"]=json!([2.0,2.0]);c["cases"][0]["hypotheses"][0]["trajectory_tubes"][0]["value_upper"]=json!([1.0,1.0]);assert!(validate_corpus(&c).is_err());}
}
