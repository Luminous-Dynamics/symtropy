use maritime_observability_evidence_validator::verify_sha256;
use serde_json::{json, Value};
use std::{collections::BTreeSet, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
#[cfg(test)]
const FROZEN: &str = include_str!("../../../../benchmarks/maritime-behavioural-set/behavioural-set-corpus.v0.1.json");
type Check<T> = Result<T, String>;

#[derive(Clone, Debug)]
struct BoxN { id: String, lower: Vec<f64>, upper: Vec<f64> }
#[derive(Clone, Debug)]
struct InnerRegion { id: String, bounds: BoxN, verified: bool }
#[derive(Clone, Debug)]
struct Hypothesis {
    fault_id: String, configuration_sha256: String, regime_id: String,
    model_sha256: String, solver_sha256: String, outer_certified: bool,
    outer_boxes: Vec<BoxN>, inner_regions: Vec<InnerRegion>,
}

fn object<'a>(v:&'a Value,p:&str)->Check<&'a serde_json::Map<String,Value>>{v.as_object().ok_or_else(||format!("{p}: expected object"))}
fn exact_keys(v:&Value,keys:&[&str],p:&str)->Check<()>{
    let m=object(v,p)?;let e:BTreeSet<&str>=keys.iter().copied().collect();let a:BTreeSet<&str>=m.keys().map(String::as_str).collect();
    let missing:Vec<_>=e.difference(&a).copied().collect();let unknown:Vec<_>=a.difference(&e).copied().collect();
    if !missing.is_empty()||!unknown.is_empty(){return Err(format!("{p}: missing {missing:?}; unknown {unknown:?}"));}Ok(())
}
fn field<'a>(v:&'a Value,k:&str,p:&str)->Check<&'a Value>{object(v,p)?.get(k).ok_or_else(||format!("{p}: missing {k}"))}
fn string<'a>(v:&'a Value,k:&str,p:&str)->Check<&'a str>{field(v,k,p)?.as_str().filter(|s|!s.trim().is_empty()).ok_or_else(||format!("{p}.{k}: expected non-empty string"))}
fn boolean(v:&Value,k:&str,p:&str)->Check<bool>{field(v,k,p)?.as_bool().ok_or_else(||format!("{p}.{k}: expected boolean"))}
fn valid_sha(s:&str)->bool{s.len()==64&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn sha(v:&Value,k:&str,p:&str)->Check<String>{let s=string(v,k,p)?;if !valid_sha(s){return Err(format!("{p}.{k}: expected lowercase SHA-256"));}Ok(s.to_owned())}
fn numbers(v:&Value,p:&str,n:usize)->Check<Vec<f64>>{
 let a=v.as_array().ok_or_else(||format!("{p}: expected array"))?;
 if a.len()!=n{return Err(format!("{p}: expected {n} coordinates, got {}",a.len()));}
 a.iter().enumerate().map(|(i,x)|x.as_f64().filter(|x|x.is_finite()).ok_or_else(||format!("{p}[{i}]: expected finite number"))).collect()
}
fn parse_box(v:&Value,p:&str,n:usize,idkey:&str)->Check<BoxN>{
 exact_keys(v,&[idkey,"lower","upper"],p)?;let id=string(v,idkey,p)?.to_owned();
 let lower=numbers(field(v,"lower",p)?,&format!("{p}.lower"),n)?;let upper=numbers(field(v,"upper",p)?,&format!("{p}.upper"),n)?;
 for (i,(lo,hi)) in lower.iter().zip(&upper).enumerate(){if lo>hi{return Err(format!("{p}: lower exceeds upper at coordinate {i}"));}}
 Ok(BoxN{id,lower,upper})
}
fn contained(inner:&BoxN,outer:&BoxN)->bool{inner.lower.iter().zip(&outer.lower).zip(inner.upper.iter().zip(&outer.upper)).all(|((il,ol),(iu,ou))|il>=ol&&iu<=ou)}
fn boxes_disjoint(a:&BoxN,b:&BoxN)->bool{a.upper.iter().zip(&b.lower).zip(b.upper.iter().zip(&a.lower)).any(|((au,bl),(bu,al))|au<bl||bu<al)}
fn all_outer_pairs_disjoint(a:&[BoxN],b:&[BoxN])->bool{a.iter().all(|x|b.iter().all(|y|boxes_disjoint(x,y)))}
fn witness(a:&[InnerRegion],b:&[InnerRegion])->Option<Value>{
 for l in a.iter().filter(|r|r.verified){for r in b.iter().filter(|r|r.verified){
  let mut point=Vec::new();let mut good=true;
  for ((ll,lu),(rl,ru)) in l.bounds.lower.iter().zip(&l.bounds.upper).zip(r.bounds.lower.iter().zip(&r.bounds.upper)){
   let low=ll.max(rl);let high=lu.min(ru);if low>high{good=false;break;}point.push(low+(high-low)/2.0);
  }
  if good{return Some(json!({"fault_a_inner_region_id":l.id,"fault_b_inner_region_id":r.id,"trajectory_witness":point,"witness_semantics":"COMMON_POINT_IN_TWO_VERIFIED_INNER_REGIONS"}));}
 }}None
}
fn parse_hypothesis(v:&Value,p:&str,n:usize)->Check<Hypothesis>{
 exact_keys(v,&["fault_id","configuration_sha256","regime_id","model_sha256","solver_sha256","outer_set_certificate","outer_boxes","inner_regions"],p)?;
 let fault_id=string(v,"fault_id",p)?.to_owned();let configuration_sha256=sha(v,"configuration_sha256",p)?;let regime_id=string(v,"regime_id",p)?.to_owned();
 let model_sha256=sha(v,"model_sha256",p)?;let solver_sha256=sha(v,"solver_sha256",p)?;
 let cert=field(v,"outer_set_certificate",p)?;let certp=format!("{p}.outer_set_certificate");exact_keys(cert,&["certificate_state","certificate_kind","certificate_sha256"],&certp)?;
 let state=string(cert,"certificate_state",&certp)?;let kind=string(cert,"certificate_kind",&certp)?;let _d=sha(cert,"certificate_sha256",&certp)?;
 if !matches!(state,"CERTIFIED"|"UNVERIFIED")||kind!="SYNTHETIC_KNOWN_TRUTH"{return Err(format!("{certp}: invalid certificate descriptor"));}
 let items=field(v,"outer_boxes",p)?.as_array().ok_or_else(||format!("{p}.outer_boxes: expected array"))?;
 if items.is_empty()||items.len()>256{return Err(format!("{p}.outer_boxes: count must be 1..=256"));}
 let mut outer_boxes=Vec::new();let mut outer_ids=BTreeSet::new();
 for (i,x) in items.iter().enumerate(){let b=parse_box(x,&format!("{p}.outer_boxes[{i}]"),n,"box_id")?;if !outer_ids.insert(b.id.clone()){return Err(format!("{p}: duplicate outer box {}",b.id));}outer_boxes.push(b);}
 let items=field(v,"inner_regions",p)?.as_array().ok_or_else(||format!("{p}.inner_regions: expected array"))?;
 if items.len()>256{return Err(format!("{p}.inner_regions: maximum 256"));}
 let mut inner_regions=Vec::new();let mut ids=BTreeSet::new();
 for (i,x) in items.iter().enumerate(){
  let at=format!("{p}.inner_regions[{i}]");exact_keys(x,&["region_id","lower","upper","certificate"],&at)?;
  let id=string(x,"region_id",&at)?.to_owned();if !ids.insert(id.clone()){return Err(format!("{at}: duplicate region ID"));}
  let lower=numbers(field(x,"lower",&at)?,&format!("{at}.lower"),n)?;let upper=numbers(field(x,"upper",&at)?,&format!("{at}.upper"),n)?;
  for (j,(lo,hi)) in lower.iter().zip(&upper).enumerate(){if lo>hi{return Err(format!("{at}: invalid interval at {j}"));}}
  let bounds=BoxN{id:id.clone(),lower,upper};if !outer_boxes.iter().any(|o|contained(&bounds,o)){return Err(format!("{at}: inner region is not contained in an outer enclosure"));}
  let c=field(x,"certificate",&at)?;let cp=format!("{at}.certificate");exact_keys(c,&["certificate_state","certificate_kind","certificate_sha256"],&cp)?;
  let cs=string(c,"certificate_state",&cp)?;let ck=string(c,"certificate_kind",&cp)?;let _d=sha(c,"certificate_sha256",&cp)?;
  if !matches!(cs,"VERIFIED"|"UNVERIFIED")||ck!="SYNTHETIC_KNOWN_TRUTH"{return Err(format!("{cp}: invalid certificate descriptor"));}
  inner_regions.push(InnerRegion{id,bounds,verified:cs=="VERIFIED"});
 }
 Ok(Hypothesis{fault_id,configuration_sha256,regime_id,model_sha256,solver_sha256,outer_certified:state=="CERTIFIED",outer_boxes,inner_regions})
}
fn outcome(id:&str,s:&str,r:&str)->Value{json!({"case_id":id,"disposition":s,"reason_codes":[r],"fault_isolation":"NOT_ATTEMPTED","physical_diagnosability_established":false,"mutation_performed":false,"authorization_granted":false})}
fn evaluate_inner(c:&Value)->Check<Value>{
 exact_keys(c,&["case_id","schema_version","evidence_class","asset","operating_domain","trajectory","output_dimensions","fault_a","fault_b","expected_disposition","authority_boundary","claim_ceiling"],"$")?;
 let id=string(c,"case_id","$")?;if string(c,"schema_version","$")?!=VERSION||string(c,"evidence_class","$")?!="SYNTHETIC_KNOWN_TRUTH"{return Err(format!("{id}: unsupported schema or evidence class"));}
 let a=field(c,"asset","$")?;exact_keys(a,&["asset_id","configuration_revision","configuration_sha256"],"$.asset")?;
 let _a=string(a,"asset_id","$.asset")?;let _r=string(a,"configuration_revision","$.asset")?;let asset_sha=sha(a,"configuration_sha256","$.asset")?;
 let d=field(c,"operating_domain","$")?;exact_keys(d,&["regime_id","qualification_state"],"$.operating_domain")?;let regime=string(d,"regime_id","$.operating_domain")?;let q=string(d,"qualification_state","$.operating_domain")?;
 if !matches!(q,"QUALIFIED"|"UNSUPPORTED"|"NOT_EVALUATED"){return Err(format!("{id}: invalid regime state"));}
 let t=field(c,"trajectory","$")?;exact_keys(t,&["input_trajectory_sha256","uncertainty_set_sha256","time_grid_ms"],"$.trajectory")?;
 let trajectory_sha=sha(t,"input_trajectory_sha256","$.trajectory")?;let uncertainty_sha=sha(t,"uncertainty_set_sha256","$.trajectory")?;
 let times=field(t,"time_grid_ms","$.trajectory")?.as_array().ok_or_else(||"time grid must be array".to_owned())?;
 if times.len()<2||times.len()>4096{return Err(format!("{id}: time grid count outside 2..=4096"));}
 let mut prev=None;for (i,x) in times.iter().enumerate(){let v=x.as_u64().ok_or_else(||format!("{id}: invalid time grid value at {i}"))?;if prev.is_some_and(|p|v<=p){return Err(format!("{id}: time grid is not strictly increasing"));}prev=Some(v);}
 let dims=field(c,"output_dimensions","$")?.as_array().ok_or_else(||"output dimensions must be array".to_owned())?;
 if dims.len()<2||dims.len()>256{return Err(format!("{id}: dimension count outside 2..=256"));}
 let mut dim_ids=BTreeSet::new();for x in dims{let s=x.as_str().filter(|s|!s.trim().is_empty()).ok_or_else(||format!("{id}: invalid dimension name"))?;if !dim_ids.insert(s.to_owned()){return Err(format!("{id}: duplicate dimension {s}"));}}
 let n=dims.len();let fa=parse_hypothesis(field(c,"fault_a","$")?,"$.fault_a",n)?;let fb=parse_hypothesis(field(c,"fault_b","$")?,"$.fault_b",n)?;
 if fa.fault_id==fb.fault_id{return Err(format!("{id}: hypotheses are identical"));}
 let auth=field(c,"authority_boundary","$")?;exact_keys(auth,&["read_only","controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"],"$.authority_boundary")?;
 if !boolean(auth,"read_only","$.authority_boundary")?||["controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"].iter().any(|k|auth[*k].as_bool()!=Some(false)){return Err(format!("{id}: authority boundary weakened"));}
 let expected=string(c,"expected_disposition","$")?;if !matches!(expected,"SEPARATION_PROVEN"|"INTERSECTION_WITNESS_FOUND"|"NOT_PROVEN_SEPARABLE"|"NOT_EVALUATED"|"INVALID_CONTRACT"){return Err(format!("{id}: unknown expected disposition"));}
 if q!="QUALIFIED"{return Ok(outcome(id,"NOT_EVALUATED","OPERATING_REGIME_NOT_QUALIFIED"));}
 if fa.configuration_sha256!=asset_sha||fb.configuration_sha256!=asset_sha||fa.regime_id!=regime||fb.regime_id!=regime{return Ok(outcome(id,"NOT_EVALUATED","CONFIGURATION_OR_REGIME_MISMATCH"));}
 if !fa.outer_certified||!fb.outer_certified{return Ok(outcome(id,"NOT_EVALUATED","OUTER_SET_CERTIFICATE_UNAVAILABLE"));}
 let separated=all_outer_pairs_disjoint(&fa.outer_boxes,&fb.outer_boxes);let common=witness(&fa.inner_regions,&fb.inner_regions);
 if separated&&common.is_some(){return Err(format!("{id}: inner witness conflicts with disjoint outer enclosures"));}
 let mut result=if separated{outcome(id,"SEPARATION_PROVEN","CERTIFIED_OUTER_SETS_DISJOINT")}else if let Some(w)=common{let mut r=outcome(id,"INTERSECTION_WITNESS_FOUND","COMMON_POINT_IN_VERIFIED_INNER_REGIONS");r["intersection_witness"]=w;r}else{outcome(id,"NOT_PROVEN_SEPARABLE","OUTER_APPROXIMATIONS_OVERLAP_WITHOUT_COMMON_INNER_WITNESS")};
 result["asset_configuration_sha256"]=json!(asset_sha);result["operating_regime_id"]=json!(regime);result["input_trajectory_sha256"]=json!(trajectory_sha);result["uncertainty_set_sha256"]=json!(uncertainty_sha);
 result["output_dimension_count"]=json!(n);result["time_grid_count"]=json!(times.len());result["fault_a_id"]=json!(fa.fault_id);result["fault_b_id"]=json!(fb.fault_id);
 result["fault_a_model_sha256"]=json!(fa.model_sha256);result["fault_b_model_sha256"]=json!(fb.model_sha256);result["fault_a_solver_sha256"]=json!(fa.solver_sha256);result["fault_b_solver_sha256"]=json!(fb.solver_sha256);
 result["outer_box_pair_count"]=json!(fa.outer_boxes.len()*fb.outer_boxes.len());result["verified_inner_region_pair_count"]=json!(fa.inner_regions.iter().filter(|r|r.verified).count()*fb.inner_regions.iter().filter(|r|r.verified).count());
 Ok(result)
}
fn evaluate_case(c:&Value)->Value{let id=c.get("case_id").and_then(Value::as_str).unwrap_or("UNKNOWN_CASE");evaluate_inner(c).unwrap_or_else(|e|{let mut r=outcome(id,"INVALID_CONTRACT","SET_CONTRACT_INVALID");r["validation_error"]=json!(e);r})}
fn run_corpus(raw:&str,source:&str,digest:&str)->Check<Value>{
 if source.len()!=40||!source.bytes().all(|b|b.is_ascii_hexdigit())||!valid_sha(digest){return Err("source commit or corpus digest invalid".to_owned());}
 let c:Value=serde_json::from_str(raw).map_err(|e|format!("invalid corpus JSON: {e}"))?;exact_keys(&c,&["schema_version","corpus_id","evidence_class","cases","claim_ceiling"],"$")?;
 if string(&c,"schema_version","$")?!=VERSION||string(&c,"evidence_class","$")?!="SYNTHETIC_KNOWN_TRUTH"{return Err("unsupported corpus schema or evidence class".to_owned());}
 let cases=field(&c,"cases","$")?.as_array().ok_or_else(||"cases must be array".to_owned())?;if cases.is_empty()||cases.len()>500{return Err("case count outside 1..=500".to_owned());}
 let mut ids=BTreeSet::new();let mut results=Vec::new();let mut counts=std::collections::BTreeMap::<String,usize>::new();
 for case in cases{let id=string(case,"case_id","$.cases[]")?.to_owned();if !ids.insert(id.clone()){return Err(format!("duplicate case {id}"));}let r=evaluate_case(case);let expected=string(case,"expected_disposition","$.cases[]")?;let actual=r["disposition"].as_str().ok_or_else(||format!("{id}: no disposition"))?;if actual!=expected{return Err(format!("{id}: expected {expected}, got {actual}"));}*counts.entry(actual.to_owned()).or_insert(0)+=1;results.push(r);}
 Ok(json!({"result":"pass","validator_version":VERSION,"corpus_id":string(&c,"corpus_id","$")?,"source_commit":source,"corpus_sha256":digest,"scenario_count":cases.len(),"disposition_counts":counts,"cases":results,"claim_ceiling":string(&c,"claim_ceiling","$")?}))
}
fn main()->ExitCode{
 let a:Vec<String>=env::args().collect();if a.len()!=6||a[2]!="--source-commit"||a[4]!="--corpus-sha256"{eprintln!("usage: behavioural_set_diagnosis <corpus.json> --source-commit <40-hex> --corpus-sha256 <64-hex>");return ExitCode::from(2);}
 let raw=match fs::read_to_string(&a[1]){Ok(v)=>v,Err(e)=>{eprintln!("read corpus: {e}");return ExitCode::FAILURE;}};
 if let Err(e)=verify_sha256(raw.as_bytes(),&a[5]){eprintln!("corpus digest verification failed: {e}");return ExitCode::FAILURE;}
 match run_corpus(&raw,&a[3],&a[5]){Ok(r)=>{println!("{}",serde_json::to_string_pretty(&r).unwrap_or_else(|_|"{\"result\":\"fail\"}".to_owned()));ExitCode::SUCCESS},Err(e)=>{eprintln!("behavioural-set evaluation failed closed: {e}");ExitCode::FAILURE}}
}
#[cfg(test)]
mod tests{
 use super::*;use sha2::{Digest,Sha256};
 fn frozen()->Value{serde_json::from_str(FROZEN).expect("frozen corpus parses")}
 #[test]fn all_fifteen_fixtures_match(){let c=frozen();let a=c["cases"].as_array().unwrap();assert_eq!(a.len(),15);for x in a{assert_eq!(evaluate_case(x)["disposition"],x["expected_disposition"],"{}",x["case_id"]);}}
 #[test]fn outer_overlap_without_witness_is_not_intersection(){let c=frozen();let r=evaluate_case(&c["cases"][2]);assert_eq!(r["disposition"],"NOT_PROVEN_SEPARABLE");assert!(r.get("intersection_witness").is_none());}
 #[test]fn verified_inner_regions_provide_constructive_witness(){let c=frozen();let r=evaluate_case(&c["cases"][1]);assert_eq!(r["disposition"],"INTERSECTION_WITNESS_FOUND");assert_eq!(r["intersection_witness"]["trajectory_witness"].as_array().unwrap().len(),3);}
 #[test]fn touching_closed_outer_boxes_are_not_disjoint(){let c=frozen();assert_eq!(evaluate_case(&c["cases"][6])["disposition"],"NOT_PROVEN_SEPARABLE");}
 #[test]fn union_separation_requires_every_outer_pair_disjoint(){let c=frozen();assert_eq!(evaluate_case(&c["cases"][7])["disposition"],"SEPARATION_PROVEN");assert_eq!(evaluate_case(&c["cases"][8])["disposition"],"NOT_PROVEN_SEPARABLE");}
 #[test]fn malformed_geometry_and_lineage_fail_closed(){let c=frozen();for i in [9usize,10,12,13,14]{assert_eq!(evaluate_case(&c["cases"][i])["disposition"],"INVALID_CONTRACT");}}
 #[test]fn unqualified_or_mismatched_models_are_not_evaluated(){let c=frozen();for i in [3usize,4,5]{assert_eq!(evaluate_case(&c["cases"][i])["disposition"],"NOT_EVALUATED");}}
 #[test]fn unverified_inner_regions_never_create_witnesses(){let c=frozen();let r=evaluate_case(&c["cases"][11]);assert_eq!(r["disposition"],"NOT_PROVEN_SEPARABLE");assert!(r.get("intersection_witness").is_none());}
 #[test]fn receipt_binds_exact_corpus_bytes(){let raw=FROZEN;let digest=format!("{:x}",Sha256::digest(raw.as_bytes()));let r=run_corpus(raw,&"a".repeat(40),&digest).unwrap();assert_eq!(r["result"],"pass");assert!(run_corpus(raw,&"a".repeat(40),&"0".repeat(64)).is_err());}
}
