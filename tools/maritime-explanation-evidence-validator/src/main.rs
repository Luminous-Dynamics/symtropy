use serde_json::{json, Value};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const FROZEN: &str = include_str!("../../../benchmarks/maritime-explanation/explanation-corpus-v0.1.json");
type Check<T> = Result<T, String>;

fn obj<'a>(v: &'a Value, at: &str) -> Check<&'a serde_json::Map<String, Value>> {
    v.as_object().ok_or_else(|| format!("{at}: expected object"))
}
fn exact(v: &Value, keys: &[&str], at: &str) -> Check<()> {
    let m = obj(v, at)?;
    let expected: BTreeSet<&str> = keys.iter().copied().collect();
    let actual: BTreeSet<&str> = m.keys().map(String::as_str).collect();
    let missing: Vec<_> = expected.difference(&actual).copied().collect();
    let extra: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(format!("{at}: missing {missing:?}; undeclared {extra:?}"));
    }
    Ok(())
}
fn allowed(v: &Value, required: &[&str], optional: &[&str], at: &str) -> Check<()> {
    let m = obj(v, at)?;
    let expected: BTreeSet<&str> = required.iter().chain(optional).copied().collect();
    let actual: BTreeSet<&str> = m.keys().map(String::as_str).collect();
    let missing: Vec<_> = required.iter().copied().filter(|k| !actual.contains(k)).collect();
    let extra: Vec<_> = actual.difference(&expected).copied().collect();
    if !missing.is_empty() || !extra.is_empty() {
        return Err(format!("{at}: missing {missing:?}; undeclared {extra:?}"));
    }
    Ok(())
}
fn text<'a>(v: &'a Value, key: &str, at: &str) -> Check<&'a str> {
    v.get(key).and_then(Value::as_str).filter(|s| !s.trim().is_empty())
        .ok_or_else(|| format!("{at}.{key}: expected non-empty string"))
}
fn uint(v: &Value, key: &str, at: &str) -> Check<u64> {
    v.get(key).and_then(Value::as_u64).ok_or_else(|| format!("{at}.{key}: expected unsigned integer"))
}
fn finite(v: &Value, key: &str, at: &str) -> Check<f64> {
    let n=v.get(key).and_then(Value::as_f64).ok_or_else(||format!("{at}.{key}: expected number"))?;
    if !n.is_finite(){return Err(format!("{at}.{key}: non-finite"));}
    Ok(n)
}
fn strings(v: &Value, key: &str, at: &str) -> Check<BTreeSet<String>> {
    let a=v.get(key).and_then(Value::as_array).ok_or_else(||format!("{at}.{key}: expected array"))?;
    let mut s=BTreeSet::new();
    for x in a {
        let item=x.as_str().filter(|x|!x.trim().is_empty()).ok_or_else(||format!("{at}.{key}: expected strings"))?;
        if !s.insert(item.to_owned()){return Err(format!("{at}.{key}: duplicate item"));}
    }
    Ok(s)
}
fn hex(s:&str,n:usize)->bool{s.len()==n&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn ids_ranked(e:&Value)->Check<Vec<String>>{
    let a=e["ranked_attributions"].as_array().ok_or_else(||"ranked_attributions must be array".to_owned())?;
    let mut features=BTreeSet::new();let mut ranks=BTreeSet::new();
    let mut values=Vec::new();
    for item in a{
        exact(item,&["feature_id","rank","attribution"],"ranked_attributions[]")?;
        let id=text(item,"feature_id","ranked_attributions[]")?.to_owned();
        let rank=uint(item,"rank","ranked_attributions[]")?;
        let score=finite(item,"attribution","ranked_attributions[]")?;
        if !features.insert(id.clone())||!ranks.insert(rank){return Err("duplicate feature or attribution rank".to_owned());}
        if score.abs()>1.0e12{return Err("attribution magnitude out of bound".to_owned());}
        values.push((rank,id));
    }
    values.sort_by_key(|x|x.0);
    Ok(values.into_iter().map(|x|x.1).collect())
}
fn case_result(
    case:&Value,
    signals:&BTreeMap<String,&Value>,
    features:&BTreeMap<String,&Value>,
    mechanisms:&BTreeMap<String,&Value>,
    all_cases:&[Value],
)->Check<(String,Value,Vec<String>)>{
    const REQUIRED:&[&str]=&["case_id","prediction_time_ms","diagnostic_state","evidence_overrides","explanation","expected_disposition"];
    const OPTIONAL:&[&str]=&["metamorphic_assertions","unit_transformations","measurement_perturbations"];
    allowed(case,REQUIRED,OPTIONAL,"cases[]")?;
    let id=text(case,"case_id","cases[]")?.to_owned();
    let prediction=uint(case,"prediction_time_ms","cases[]")?;
    exact(&case["diagnostic_state"],&["observability_state","admissible_fault_ids"],"diagnostic_state")?;
    let obs=text(&case["diagnostic_state"],"observability_state","diagnostic_state")?;
    let admissible: BTreeSet<String>=strings(&case["diagnostic_state"],"admissible_fault_ids","diagnostic_state")?;
    exact(&case["explanation"],&["model_id","model_sha256","explainer","explanation_signature","ranked_attributions","claims"],"explanation")?;
    exact(&case["explanation"]["explainer"],&["method","version","background_set_sha256"],"explainer")?;
    let method=text(&case["explanation"]["explainer"],"method","explainer")?;
    let signature=text(&case["explanation"],"explanation_signature","explanation")?.to_owned();
    let top=ids_ranked(&case["explanation"])?;
    let claims=case["explanation"]["claims"].as_array().ok_or_else(||format!("{id}: claims must be array"))?;
    let mut cited:BTreeSet<String>=top.iter().cloned().collect();
    for claim in claims {
        exact(claim,&["claim_id","claim_type","fault_ids","component_id","effect_id","supporting_feature_ids","independent_source_signal_ids","causal_evidence_refs","mechanism_link_id"],"claims[]")?;
        cited.extend(strings(claim,"supporting_feature_ids","claims[]")?);
    }
    let feature_refs:Vec<&Value>=cited.iter().map(|fid|features.get(fid).copied().ok_or_else(||format!("{id}: unknown feature {fid}"))).collect::<Check<_>>()?;
    // If the same explanation signature persists despite losing one of its
    // cited supporting signals, reject it rather than presenting stale certainty.
    let overrides=case["evidence_overrides"].as_array().ok_or_else(||format!("{id}: evidence_overrides must be array"))?;
    let mut withdrawn=BTreeSet::new();
    for ov in overrides{
        exact(ov,&["signal_id","quality_state","installed"],"evidence_overrides[]")?;
        let sid=text(ov,"signal_id","evidence_overrides[]")?;
        let unavailable=ov["quality_state"].as_str()!=Some("CURRENT")||ov["installed"].as_bool()!=Some(true);
        if unavailable&&feature_refs.iter().any(|f|f["signal_ids"].as_array().is_some_and(|a|a.iter().any(|x|x.as_str()==Some(sid)))){
            withdrawn.insert(sid.to_owned());
        }
    }
    if !withdrawn.is_empty()&&all_cases.iter().any(|other|{
        other["case_id"]!=id
            &&other["explanation"]["explanation_signature"].as_str()==Some(&signature)
            &&other["evidence_overrides"].as_array().is_some_and(|a|a.is_empty())
            &&other["explanation"]["claims"].as_array().is_some_and(|a|!a.is_empty())
    }){
        return Ok((id,json!({"disposition":"INVALID_UNRESPONSIVE_EXPLANATION","reason_codes":["SUPPORT_WITHDRAWN_SIGNATURE_UNCHANGED"],"accepted_feature_ids":[],"claim_results":[]}),top));
    }
    if obs=="UNSUPPORTED_REGIME"||obs=="NOT_EVALUATED"{
        return Ok((id,json!({"disposition":"NOT_EVALUATED","reason_codes":["DIAGNOSIS_NOT_QUALIFIED"],"accepted_feature_ids":[],"claim_results":[]}),top));
    }
    for f in &feature_refs{
        let fid=text(f,"feature_id","features[]")?;
        if uint(f,"available_at_ms","features[]")?>prediction{
            return Ok((id,json!({"disposition":"INVALID_FUTURE_INFORMATION","reason_codes":["FEATURE_AFTER_PREDICTION"],"accepted_feature_ids":[],"claim_results":[]}),top));
        }
        for sidv in f["signal_ids"].as_array().ok_or_else(||format!("{fid}: signal_ids missing"))?{
            let sid=sidv.as_str().ok_or_else(||format!("{fid}: bad signal ID"))?;
            let s=signals.get(sid).copied().ok_or_else(||format!("{fid}: unknown signal {sid}"))?;
            if uint(s,"available_at_ms","signals[]")?>prediction{
                return Ok((id,json!({"disposition":"INVALID_FUTURE_INFORMATION","reason_codes":["SOURCE_SIGNAL_AFTER_PREDICTION"],"accepted_feature_ids":[],"claim_results":[]}),top));
            }
            let (installed,quality)=overrides.iter().find(|ov|ov["signal_id"].as_str()==Some(sid))
                .map(|ov|(ov["installed"].as_bool().unwrap_or(false),ov["quality_state"].as_str().unwrap_or("UNKNOWN")))
                .unwrap_or((s["installed"].as_bool().unwrap_or(false),s["quality_state"].as_str().unwrap_or("UNKNOWN")));
            // A model output is available as a model artifact even though it
            // is not a physically installed sensor. Permit attribution to it;
            // later claim checks prohibit promoting it to measured mechanism
            // evidence. All physical sensor signals still require installation.
            let model_output = s["evidence_class"] == "SIMULATED"
                && s["source_kind"] == "MODEL_OUTPUT";
            if (!installed && !model_output)||quality!="CURRENT"||s["calibration_state"]=="EXPIRED"||s["calibration_state"]=="UNKNOWN"{
                return Ok((id,json!({"disposition":"INVALID_UNAVAILABLE_EVIDENCE","reason_codes":["CITED_SIGNAL_NOT_INSTALLED_CURRENT_AND_QUALIFIED"],"accepted_feature_ids":[],"claim_results":[]}),top));
            }
        }
    }
    let mut results=Vec::new();let mut claim_faults=BTreeSet::new();
    for claim in claims{
        let claim_id=text(claim,"claim_id","claims[]")?.to_owned();
        let ctype=text(claim,"claim_type","claims[]")?;
        let fids=strings(claim,"fault_ids","claims[]")?;
        claim_faults.extend(fids.iter().cloned());
        let support=strings(claim,"supporting_feature_ids","claims[]")?;
        let independent=strings(claim,"independent_source_signal_ids","claims[]")?;
        let causal=strings(claim,"causal_evidence_refs","claims[]")?;
        let mut groups=BTreeSet::new();
        // A derived source and any parent source cannot be presented as
        // independent corroboration in the same claim.
        for sid in &independent{
            let s=signals.get(sid).copied().ok_or_else(||format!("{claim_id}: unknown signal {sid}"))?;
            let parents=strings(s,"derived_from_signal_ids","signals[]")?;
            if parents.iter().any(|p|independent.contains(p)){
                return Ok((id,json!({"disposition":"INVALID_INDEPENDENCE_CLAIM","reason_codes":["DERIVED_SOURCE_AND_PARENT_DOUBLE_COUNTED"],"accepted_feature_ids":[],"claim_results":results}),top));
            }
            for group in strings(s,"dependency_groups","signals[]")?{
                if !groups.insert(group){
                    return Ok((id,json!({"disposition":"INVALID_INDEPENDENCE_CLAIM","reason_codes":["SHARED_DEPENDENCY_GROUP_DOUBLE_COUNTED"],"accepted_feature_ids":[],"claim_results":results}),top));
                }
            }
            if ctype!="MODEL_ASSOCIATION"&&s["role"]=="CONTROL_INTENT"{
                return Ok((id,json!({"disposition":"INVALID_CLAIM_ROLE_PROMOTION","reason_codes":["CONTROL_INTENT_USED_AS_PHYSICAL_SUPPORT"],"accepted_feature_ids":[],"claim_results":results}),top));
            }
        }
        if ctype=="MECHANISM_SUPPORTED"||ctype=="CAUSAL"{
            for fid in &support{
                let f=features.get(fid).copied().ok_or_else(||format!("{claim_id}: unknown feature {fid}"))?;
                if f["role"]=="SIMULATED"||f["role"]=="RECONSTRUCTED"||f["role"]=="DERIVED_RESPONSE"
                    ||f["signal_ids"].as_array().is_some_and(|a|a.iter().any(|x|x.as_str().and_then(|sid|signals.get(sid).copied()).is_none_or(|s|s["evidence_class"]!="MEASURED"))){
                    return Ok((id,json!({"disposition":"INVALID_EVIDENCE_ROLE_PROMOTION","reason_codes":["NON_MEASURED_FEATURE_CLAIMED_AS_MECHANISM"],"accepted_feature_ids":[],"claim_results":results}),top));
                }
            }
        }
        if ctype=="CAUSAL"&&(causal.is_empty()||matches!(method,"SHAP"|"LIME"|"ATTENTION_WEIGHTS"|"INTEGRATED_GRADIENTS")){
            return Ok((id,json!({"disposition":"INVALID_CAUSAL_OVERCLAIM","reason_codes":["MODEL_ATTRIBUTION_IS_NOT_CAUSAL_EVIDENCE"],"accepted_feature_ids":[],"claim_results":results}),top));
        }
        if obs=="AMBIGUITY_CLASS"&&!fids.is_empty()&&fids!=admissible{
            return Ok((id,json!({"disposition":"INVALID_UNIQUE_CAUSE_CLAIM","reason_codes":["NONISOLABLE_FAULT_SET_NARROWED"],"accepted_feature_ids":[],"claim_results":results}),top));
        }
        if let Some(link_id)=claim["mechanism_link_id"].as_str(){
            let link=mechanisms.get(link_id).copied().ok_or_else(||format!("{claim_id}: unknown mechanism link {link_id}"))?;
            if ctype=="MECHANISM_SUPPORTED"&&link["review_state"]!="INDEPENDENTLY_REVIEWED"{
                return Ok((id,json!({"disposition":"NOT_EVALUATED","reason_codes":["MECHANISM_LINK_NOT_INDEPENDENTLY_REVIEWED"],"accepted_feature_ids":[],"claim_results":results}),top));
            }
        }
        results.push(json!({"claim_id":claim_id,"status":"ACCEPTED_WITH_LIMITS","reason_codes":[]}));
    }
    let disposition=if obs=="AMBIGUITY_CLASS"{
        if admissible.iter().all(|f|claim_faults.contains(f)){"AMBIGUITY_PRESERVED"}else{"INVALID_UNIQUE_CAUSE_CLAIM"}
    }else{"ACCEPTED_WITH_LIMITS"};
    let accepted=if disposition=="ACCEPTED_WITH_LIMITS"||disposition=="AMBIGUITY_PRESERVED"{top.clone()}else{Vec::new()};
    Ok((id,json!({"disposition":disposition,"reason_codes":[],"accepted_feature_ids":accepted,"claim_results":results}),top))
}
fn run_value(c:&Value,source:&str,digest:&str)->Check<Value>{
    if !hex(source,40)||!hex(digest,64){return Err("invalid source commit or corpus digest".to_owned());}
    exact(c,&["schema_version","corpus_id","evidence_class","claim_ceiling","prediction_clock","asset","operating_domain","signals","features","mechanism_links","cases","authority_boundary"],"$")?;
    if text(c,"schema_version","$")?!=VERSION||text(c,"evidence_class","$")?!="SYNTHETIC_KNOWN_TRUTH"{return Err("unsupported schema version or evidence class".to_owned());}
    let auth=&c["authority_boundary"];
    exact(auth,&["read_only","controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"],"authority_boundary")?;
    if auth["read_only"]!=true||["controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"].iter().any(|k|auth[*k]!=false){return Err("authority boundary violation".to_owned());}
    let signal_array=c["signals"].as_array().ok_or_else(||"signals must be array".to_owned())?;
    let feature_array=c["features"].as_array().ok_or_else(||"features must be array".to_owned())?;
    let mech_array=c["mechanism_links"].as_array().ok_or_else(||"mechanism_links must be array".to_owned())?;
    let cases=c["cases"].as_array().ok_or_else(||"cases must be array".to_owned())?;
    let mut sm=BTreeMap::new();for s in signal_array{let id=text(s,"signal_id","signals[]")?.to_owned();if sm.insert(id.clone(),s).is_some(){return Err(format!("duplicate signal {id}"));}}
    let mut fm=BTreeMap::new();for f in feature_array{let id=text(f,"feature_id","features[]")?.to_owned();if fm.insert(id.clone(),f).is_some(){return Err(format!("duplicate feature {id}"));}}
    let mut mm=BTreeMap::new();for m in mech_array{let id=text(m,"mechanism_link_id","mechanism_links[]")?.to_owned();if mm.insert(id.clone(),m).is_some(){return Err(format!("duplicate mechanism {id}"));}}
    let mut case_ids=BTreeSet::new();for c in cases{let id=text(c,"case_id","cases[]")?.to_owned();if !case_ids.insert(id.clone()){return Err(format!("duplicate case {id}"));}}
    let mut outcomes=Vec::new();let mut top_map=BTreeMap::new();let mut counts=BTreeMap::<String,usize>::new();
    for cse in cases{
        let(id,out,top)=case_result(cse,&sm,&fm,&mm,cases)?;
        let expected=text(cse,"expected_disposition","cases[]")?;
        let actual=text(&out,"disposition","result")?;
        if expected!=actual{return Err(format!("{id}: expected {expected}, got {actual}"));}
        *counts.entry(actual.to_owned()).or_default()+=1;
        top_map.insert(id.clone(),top);
        outcomes.push(json!({"case_id":id,"result":out}));
    }
    let mut metas=Vec::new();let mut meta_ids=BTreeSet::new();
    for cse in cases{
        for a in cse.get("metamorphic_assertions").and_then(Value::as_array).into_iter().flatten(){
            exact(a,&["assertion_id","kind","related_case_ids","expected_relation"],"metamorphic_assertions[]")?;
            let aid=text(a,"assertion_id","metamorphic_assertions[]")?.to_owned();if !meta_ids.insert(aid.clone()){return Err(format!("duplicate assertion {aid}"));}
            let related=a["related_case_ids"].as_array().ok_or_else(||"related_case_ids missing".to_owned())?;
            if related.len()!=2{return Err(format!("{aid}: exactly two related cases required"));}
            let first=related[0].as_str().ok_or_else(||"bad related case".to_owned())?;
            let second=related[1].as_str().ok_or_else(||"bad related case".to_owned())?;
            let c1=cases.iter().find(|x|x["case_id"]==first).ok_or_else(||format!("{aid}: unknown case {first}"))?;
            let c2=cases.iter().find(|x|x["case_id"]==second).ok_or_else(||format!("{aid}: unknown case {second}"))?;
            let same=top_map.get(first)==top_map.get(second)&&c1["expected_disposition"]==c2["expected_disposition"];
            let valid=match text(a,"kind","metamorphic_assertions[]")?{
                "UNIT_INVARIANCE"=>same&&(c1.get("unit_transformations").is_some()||c2.get("unit_transformations").is_some()),
                "UNCERTAINTY_PERTURBATION_STABILITY"=>same&&[c1,c2].iter().any(|x|x["measurement_perturbations"].as_array().is_some_and(|a|!a.is_empty()&&a.iter().all(|p|p["value_delta"].as_f64().zip(p["declared_measurement_uncertainty"].as_f64()).is_some_and(|(d,u)|d.abs()<=u)))),
                _=>false
            };
            if !valid{return Err(format!("{aid}: metamorphic assertion failed"));}
            metas.push(json!({"assertion_id":aid,"status":"PASS","related_case_ids":[first,second]}));
        }
    }
    let metrics:Vec<Value>=counts.iter().map(|(d,n)|json!({"metric_id":format!("case_disposition_count:{d}"),"value":n,"unit":"cases","stratum":"synthetic_known_truth"})).collect();
    Ok(json!({"schema_version":VERSION,"assessment_id":c["corpus_id"],"execution":{"source_commit":source,"case_sha256":digest,"model_sha256":null,"explainer_sha256":null,"background_set_sha256":null},"case_results":outcomes,"metamorphic_results":metas,"metrics":metrics,"authority_boundary":auth,"claim_ceiling":c["claim_ceiling"]}))
}
fn run(path:&str,source:&str,digest:&str)->Check<Value>{
    let raw=fs::read_to_string(path).map_err(|e|format!("read corpus: {e}"))?;
    let c:Value=serde_json::from_str(&raw).map_err(|e|format!("parse corpus: {e}"))?;
    run_value(&c,source,digest)
}
fn main()->ExitCode{
    let a:Vec<String>=env::args().collect();
    if a.len()!=6||a[2]!="--source-commit"||a[4]!="--case-sha256"{
        eprintln!("usage: maritime-explanation-evidence-validator <corpus.json> --source-commit <40-hex> --case-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    match run(&a[1],&a[3],&a[5]){
        Ok(report)=>{println!("{}",serde_json::to_string_pretty(&report).unwrap_or_else(|_|"{\"result\":\"fail\"}".to_owned()));ExitCode::SUCCESS},
        Err(e)=>{eprintln!("explanation evidence validation failed closed: {e}");ExitCode::FAILURE}
    }
}
#[cfg(test)]
mod tests{
    use super::*;
    fn frozen()->Value{serde_json::from_str(FROZEN).expect("frozen corpus JSON")}
    fn report()->Value{run_value(&frozen(),"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").expect("synthetic corpus validates")}
    #[test]fn all_fourteen_cases_match_known_truth(){assert_eq!(report()["case_results"].as_array().unwrap().len(),14);}
    #[test]fn unit_and_uncertainty_metamorphic_assertions_pass(){let r=report();assert_eq!(r["metamorphic_results"].as_array().unwrap().len(),2);assert!(r["metamorphic_results"].as_array().unwrap().iter().all(|x|x["status"]=="PASS"));}
    #[test]fn authority_boundary_cannot_enable_actuation(){let mut c=frozen();c["authority_boundary"]["actuation"]=json!(true);assert!(run_value(&c,"aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa","bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb").is_err());}
    #[test]fn command_intent_cannot_be_physical_delivery(){let r=report();let c=r["case_results"].as_array().unwrap().iter().find(|x|x["case_id"]=="command-promoted-to-physical-delivery").unwrap();assert_eq!(c["result"]["disposition"],"INVALID_CLAIM_ROLE_PROMOTION");}
    #[test]fn derived_and_parent_signals_are_not_independent(){let r=report();let c=r["case_results"].as_array().unwrap().iter().find(|x|x["case_id"]=="derived-feature-double-counted-as-independent").unwrap();assert_eq!(c["result"]["disposition"],"INVALID_INDEPENDENCE_CLAIM");}
    #[test]fn ambiguity_and_causality_are_honest(){let r=report();let cases=r["case_results"].as_array().unwrap();let a=cases.iter().find(|x|x["case_id"]=="nonisolable-pair-ambiguity-preserved").unwrap();let b=cases.iter().find(|x|x["case_id"]=="nonisolable-pair-unique-root-cause-claim").unwrap();let c=cases.iter().find(|x|x["case_id"]=="shap-association-mislabelled-as-causal").unwrap();assert_eq!(a["result"]["disposition"],"AMBIGUITY_PRESERVED");assert_eq!(b["result"]["disposition"],"INVALID_UNIQUE_CAUSE_CLAIM");assert_eq!(c["result"]["disposition"],"INVALID_CAUSAL_OVERCLAIM");}
    #[test]fn withdrawn_support_must_change_the_explanation(){let r=report();let c=r["case_results"].as_array().unwrap().iter().find(|x|x["case_id"]=="explanation-unchanged-after-evidence-withdrawal").unwrap();assert_eq!(c["result"]["disposition"],"INVALID_UNRESPONSIVE_EXPLANATION");}
}
