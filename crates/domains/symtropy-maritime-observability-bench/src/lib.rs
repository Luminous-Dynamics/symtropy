//! Maritime adapter over the reusable domain-neutral sensor observability evaluator.
//! This crate owns maritime identity/dependency mapping and report translation;
//! pairwise interval separation remains in symtropy-sensor-observability-bench.
#![forbid(unsafe_code)]
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use symtropy_sensor_observability_bench::{evaluate_case, EvaluationReport, Violation};

type Check<T> = Result<T, Vec<Violation>>;

fn err(code:&str,path:&str,detail:impl Into<String>)->Vec<Violation>{
    vec![Violation{code:code.into(),path:path.into(),detail:detail.into()}]
}
fn txt<'a>(v:&'a Value,k:&str,p:&str)->Check<&'a str>{
    v.get(k).and_then(Value::as_str).filter(|s|!s.trim().is_empty())
        .ok_or_else(||err("MISSING_OR_INVALID_TEXT",&format!("{p}.{k}"),"expected non-empty string"))
}
fn boolean(v:&Value,k:&str,p:&str)->Check<bool>{
    v.get(k).and_then(Value::as_bool).ok_or_else(||err("INVALID_BOOLEAN",&format!("{p}.{k}"),"expected boolean"))
}
fn array<'a>(v:&'a Value,k:&str,p:&str)->Check<&'a Vec<Value>>{
    v.get(k).and_then(Value::as_array).ok_or_else(||err("INVALID_ARRAY",&format!("{p}.{k}"),"expected array"))
}
fn set(v:&Value,k:&str,p:&str)->Check<BTreeSet<String>>{
    let mut out=BTreeSet::new();
    for (i,item) in array(v,k,p)?.iter().enumerate(){
        let s=item.as_str().filter(|s|!s.trim().is_empty()).ok_or_else(||err("INVALID_ARRAY_MEMBER",&format!("{p}.{k}[{i}]"),"expected non-empty string"))?;
        if !out.insert(s.to_owned()){return Err(err("DUPLICATE_ARRAY_MEMBER",&format!("{p}.{k}[{i}]"),"duplicate identity"));}
    }
    Ok(out)
}
fn hex64(s:&str)->bool{s.len()==64&&s.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b))}
fn validate_input(input:&Value)->Check<()>{
    if !input.is_object(){return Err(err("EXPECTED_OBJECT","$","case must be an object"));}
    if txt(input,"schema_version","$")?!="0.1.0"{return Err(err("SCHEMA_VERSION","$.schema_version","expected 0.1.0"));}
    if !matches!(txt(input,"evidence_class","$")?,"SYNTHETIC_CONFORMANCE_FIXTURE"|"MEASURED"){
        return Err(err("EVIDENCE_CLASS","$.evidence_class","unsupported evidence class"));
    }
    for k in ["assessment_id","generated_at","claim_ceiling"]{let _=txt(input,k,"$")?;}
    let asset=&input["asset"];
    for k in ["asset_id","asset_class","engine_model","configuration_revision"]{let _=txt(asset,k,"$.asset")?;}
    if !hex64(txt(asset,"configuration_sha256","$.asset")?){return Err(err("CONFIG_SHA256","$.asset.configuration_sha256","expected lowercase SHA-256"));}
    let regime=&input["operating_domain"];
    for k in ["regime_id","description","qualification_state","transient_state"]{let _=txt(regime,k,"$.operating_domain")?;}
    let digests=&input["evidence_digests"];
    for k in ["sensor_topology_sha256","fault_model_sha256","evaluator_sha256"]{
        if !hex64(txt(digests,k,"$.evidence_digests")?){return Err(err("EVIDENCE_SHA256",&format!("$.evidence_digests.{k}"),"expected lowercase SHA-256"));}
    }
    for k in ["dataset_sha256","split_manifest_sha256","model_sha256"]{
        if let Some(v)=digests.get(k).filter(|v|!v.is_null()){
            if !v.as_str().is_some_and(hex64){return Err(err("OPTIONAL_SHA256",&format!("$.evidence_digests.{k}"),"expected null or lowercase SHA-256"));}
        }
    }
    let profile=&input["layout_profile"];
    for k in ["layout_profile_ref","state_taxonomy_ref","required_pair_set_ref"]{let _=txt(profile,k,"$.layout_profile")?;}
    let min=profile["minimum_independent_principles_for_critical_pairs"].as_u64().filter(|n|*n>0)
        .ok_or_else(||err("INDEPENDENCE_THRESHOLD","$.layout_profile.minimum_independent_principles_for_critical_pairs","must be positive"))?;
    let sensors=array(input,"sensors","$")?;
    if sensors.is_empty(){return Err(err("EMPTY_SENSOR_INVENTORY","$.sensors","at least one sensor required"));}
    let mut sensor_ids=BTreeSet::new();let mut channel_ids=BTreeSet::new();let mut dependencies=BTreeSet::new();
    for (i,s) in sensors.iter().enumerate(){
        let p=format!("$.sensors[{i}]");
        let sid=txt(s,"sensor_id",&p)?;let cid=txt(s,"channel_id",&p)?;
        if !sensor_ids.insert(sid.to_owned())||!channel_ids.insert(cid.to_owned()){return Err(err("DUPLICATE_SENSOR_CHANNEL_ID",&p,"IDs must be unique"));}
        for k in ["measurement_principle_ref","measurand","technique","unit","physical_location_ref","calibration_ref","calibration_state","quality_state","source_role","source_artifact_ref"]{let _=txt(s,k,&p)?;}
        let _=boolean(s,"installed",&p)?;
        dependencies.extend(set(s,"dependency_groups",&p)?);
        let derived=set(s,"derived_from_channel_ids",&p)?;
        if !matches!(txt(s,"source_role",&p)?,"measured"|"derived"|"reconstructed"|"simulated"|"synthetic"){return Err(err("SENSOR_SOURCE_ROLE",&format!("{p}.source_role"),"unsupported source role"));}
        if !matches!(txt(s,"quality_state",&p)?,"current"|"missing"|"stale"|"rejected"|"conflicted"|"unknown"){return Err(err("SENSOR_QUALITY",&format!("{p}.quality_state"),"unsupported quality state"));}
        if !matches!(txt(s,"calibration_state",&p)?,"current"|"expired"|"unknown"|"not_applicable"){return Err(err("CALIBRATION_STATE",&format!("{p}.calibration_state"),"unsupported calibration state"));}
        if derived.iter().any(|id|!channel_ids.contains(id)){/* checked against full channel set below */}
    }
    for (i,s) in sensors.iter().enumerate(){
        for id in set(s,"derived_from_channel_ids",&format!("$.sensors[{i}]"))?{
            if !channel_ids.contains(&id){return Err(err("UNKNOWN_DERIVATION_PARENT",&format!("$.sensors[{i}].derived_from_channel_ids"),id));}
        }
    }
    let states=array(input,"states","$")?;
    if states.len()<2{return Err(err("STATE_COUNT","$.states","at least two states required"));}
    let mut state_ids=BTreeSet::new();
    for (i,s) in states.iter().enumerate(){
        let p=format!("$.states[{i}");let id=txt(s,"state_id",&p)?;
        if !state_ids.insert(id.to_owned()){return Err(err("DUPLICATE_STATE_ID",&format!("{p}.state_id"),"state IDs must be unique"));}
        if !matches!(txt(s,"state_class",&p)?,"PROCESS_STATE"|"SENSOR_FAULT"|"OUT_OF_DOMAIN"){return Err(err("STATE_CLASS",&format!("{p}.state_class"),"unsupported state class"));}
        let faulted=set(s,"faulted_sensor_ids",&p)?;
        if txt(s,"state_class",&p)?=="SENSOR_FAULT"&&faulted.is_empty(){return Err(err("SENSOR_FAULT_WITHOUT_SENSOR",&p,"sensor fault must identify affected sensor"));}
        if txt(s,"state_class",&p)!="SENSOR_FAULT"&&!faulted.is_empty(){return Err(err("FAULTED_SENSOR_ON_NON_SENSOR_STATE",&p,"only SENSOR_FAULT may name sensor IDs"));}
        if faulted.iter().any(|id|!sensor_ids.contains(id)){return Err(err("UNKNOWN_FAULTED_SENSOR",&p,"unknown sensor ID"));}
    }
    let pairs=array(input,"required_fault_pairs","$")?;
    if pairs.is_empty(){return Err(err("EMPTY_PAIR_SET","$.required_fault_pairs","at least one pair required"));}
    let mut pair_ids=BTreeSet::new();
    for (i,pair) in pairs.iter().enumerate(){
        let p=format!("$.required_fault_pairs[{i}]");let id=txt(pair,"pair_id",&p)?;
        if !pair_ids.insert(id.to_owned()){return Err(err("DUPLICATE_PAIR_ID",&p,"pair IDs must be unique"));}
        let a=txt(pair,"fault_a",&p)?;let b=txt(pair,"fault_b",&p)?;
        if a==b||!state_ids.contains(a)||!state_ids.contains(b){return Err(err("INVALID_FAULT_PAIR",&p,"pair must reference distinct declared states"));}
        let _=boolean(pair,"critical",&p)?;
        if pair["minimum_independent_principles"].as_u64().is_none_or(|n|n<min){return Err(err("PAIR_THRESHOLD_BELOW_PROFILE",&p,"pair minimum cannot undercut layout profile"));}
    }
    let responses=array(input,"responses","$")?;
    let installed_channels:BTreeSet<String>=sensors.iter().filter(|s|s["installed"]==true).filter_map(|s|s["channel_id"].as_str().map(str::to_owned)).collect();
    let mut seen=BTreeSet::new();
    for (i,r) in responses.iter().enumerate(){
        let p=format!("$.responses[{i}]");let state=txt(r,"state_id",&p)?;let ch=txt(r,"channel_id",&p)?;
        if !state_ids.contains(state)||!channel_ids.contains(ch){return Err(err("UNKNOWN_RESPONSE_IDENTITY",&p,"unknown state/channel"));}
        if !seen.insert((state.to_owned(),ch.to_owned())){return Err(err("DUPLICATE_RESPONSE_CELL",&p,"duplicate state/channel response"));}
        let availability=txt(r,"availability",&p)?;
        if !matches!(availability,"VALID"|"DEGRADED"|"MISSING"|"UNKNOWN"){return Err(err("RESPONSE_AVAILABILITY",&p,"unknown availability"));}
        for k in ["unit","evidence_role","measurement_window_ref","artifact_id","adjudication_ref"]{let _=txt(r,k,&p)?;}
        if !hex64(txt(r,"source_sha256",&p)?){return Err(err("RESPONSE_SHA256",&p,"expected lowercase SHA-256"));}
        if availability=="VALID"{
            let lo=r["value_lower"].as_f64().filter(|x|x.is_finite());let hi=r["value_upper"].as_f64().filter(|x|x.is_finite());
            if lo.zip(hi).is_none_or(|(a,b)|a>b){return Err(err("RESPONSE_INTERVAL",&p,"VALID requires finite ordered interval"));}
        }else if !r["value_lower"].is_null()||!r["value_upper"].is_null(){return Err(err("UNAVAILABLE_RESPONSE_HAS_VALUES",&p,"non-valid response must not carry a numeric interval"));}
        let sensor=sensors.iter().find(|s|s["channel_id"]==ch).unwrap();
        if !installed_channels.contains(ch)&&availability=="VALID"{return Err(err("UNINSTALLED_SENSOR_HAS_MEASUREMENT",&p,"uninstalled sensor cannot be a valid measurement"));}
        if txt(sensor,"unit","$.sensors[]")?!=txt(r,"unit",&p)?{return Err(err("RESPONSE_UNIT_MISMATCH",&p,"response unit must match sensor"));}
    }
    for state in states{for s in sensors.iter().filter(|s|s["installed"]==true){
        let key=(txt(state,"state_id","$.states[]")?.to_owned(),txt(s,"channel_id","$.sensors[]")?.to_owned());
        if !seen.contains(&key){return Err(err("INCOMPLETE_RESPONSE_MATRIX","$.responses",format!("missing response for {} / {}",key.0,key.1)));}
    }}
    let scenarios=array(input,"common_cause_scenarios","$")?;let mut scenario_ids=BTreeSet::new();
    for (i,s) in scenarios.iter().enumerate(){
        let p=format!("$.common_cause_scenarios[{i}]");let id=txt(s,"scenario_id",&p)?;
        if !scenario_ids.insert(id.to_owned()){return Err(err("DUPLICATE_SCENARIO_ID",&p,"scenario IDs must be unique"));}
        let removed_s=set(s,"removed_sensor_ids",&p)?;let removed_d=set(s,"removed_dependency_groups",&p)?;
        if removed_s.is_empty()&&removed_d.is_empty(){return Err(err("EMPTY_SENSOR_LOSS_SCENARIO",&p,"remove at least one sensor/dependency group"));}
        if removed_s.iter().any(|x|!sensor_ids.contains(x)){return Err(err("UNKNOWN_REMOVED_SENSOR",&p,"unknown removed sensor"));}
        if removed_d.iter().any(|x|!dependencies.contains(x)){return Err(err("UNKNOWN_REMOVED_DEPENDENCY",&p,"unknown dependency group"));}
    }
    Ok(())
}

fn make_parent_and_case(input:&Value,removed_sensors:&BTreeSet<String>,removed_dependencies:&BTreeSet<String>)->Check<(Value,Value)>{
    let asset=&input["asset"];let regime=&input["operating_domain"];let digests=&input["evidence_digests"];
    let family=format!("engine-family:{}",txt(asset,"engine_model","$.asset")?);
    let config=format!("asset-config-sha256:{}",txt(asset,"configuration_sha256","$.asset")?);
    let domain=format!("operating-domain:{}",txt(regime,"regime_id","$.operating_domain")?);
    let installation=format!("sensor-topology-sha256:{}",txt(digests,"sensor_topology_sha256","$.evidence_digests")?);
    let parent_id=format!("asset:{}",txt(asset,"asset_id","$.asset")?);
    let interval=format!("regime:{}",txt(regime,"regime_id","$.operating_domain")?);
    let trial=format!("configuration:{}",txt(asset,"configuration_revision","$.asset")?);
    let identity=json!({"asset_family_ref":family,"asset_configuration_ref":config,"operating_domain_ref":domain,"installation_profile_ref":installation});
    let parent=json!({"case_id":parent_id,"intervals":[{"interval_id":interval,"trial_id":trial}],"profile":identity});
    let parent_context=json!({"parent_case_id":parent_id,"parent_interval_id":interval,"trial_id":trial,
        "asset_family_ref":family,"asset_configuration_ref":config,"operating_domain_ref":domain,"installation_profile_ref":installation});
    let sensors=input["sensors"].as_array().unwrap();
    let installed:Vec<&Value>=sensors.iter().filter(|s|s["installed"]==true).collect();
    let mut channel_map=BTreeMap::new();let mut channels=Vec::new();
    for s in &installed{
        let sid=txt(s,"sensor_id","$.sensors[]")?;let cid=txt(s,"channel_id","$.sensors[]")?;
        channel_map.insert(sid.to_owned(),cid.to_owned());
        let deps=set(s,"dependency_groups","$.sensors[]")?;
        let dependency_domain=format!("dependency-profile:{sid}:{}",deps.into_iter().collect::<Vec<_>>().join("|"));
        channels.push(json!({
            "channel_id":cid,"measurand":txt(s,"measurand","$.sensors[]")?,"technique":txt(s,"technique","$.sensors[]")?,
            "unit":txt(s,"unit","$.sensors[]")?,"physical_location_ref":txt(s,"physical_location_ref","$.sensors[]")?,
            "calibration_ref":txt(s,"calibration_ref","$.sensors[]")?,"independence_group_ref":txt(s,"measurement_principle_ref","$.sensors[]")?,
            "dependency_domain_ref":dependency_domain,"domain":identity,"source_artifact_ref":txt(s,"source_artifact_ref","$.sensors[]")?
        }));
    }
    if channels.is_empty(){return Err(err("NO_INSTALLED_SENSORS","$.sensors","at least one installed sensor required"));}
    let mut states=Vec::new();
    for s in input["states"].as_array().unwrap(){
        let mut faulted=Vec::new();
        for sensor_id in set(s,"faulted_sensor_ids","$.states[]")?{
            let channel=channel_map.get(&sensor_id).ok_or_else(||err("FAULTED_SENSOR_NOT_INSTALLED","$.states[].faulted_sensor_ids",format!("{sensor_id} is not installed")))?;
            faulted.push(channel.clone());
        }
        states.push(json!({"state_id":txt(s,"state_id","$.states[]")?,"state_class":txt(s,"state_class","$.states[]")?,"faulted_channel_ids":faulted}));
    }
    let required_pairs:Vec<Value>=input["required_fault_pairs"].as_array().unwrap().iter().map(|p|json!({
        "pair_id":p["pair_id"],"state_a_id":p["fault_a"],"state_b_id":p["fault_b"],"critical":p["critical"],
        "minimum_independent_principles":p["minimum_independent_principles"]
    })).collect();
    let evidence_class=txt(input,"evidence_class","$")?;
    let mut responses=Vec::new();
    for r in input["responses"].as_array().unwrap(){
        let cid=txt(r,"channel_id","$.responses[]")?;
        let Some(sensor)=installed.iter().find(|s|s["channel_id"]==cid) else {continue};
        let sid=txt(sensor,"sensor_id","$.sensors[]")?;
        let ok=txt(sensor,"quality_state","$.sensors[]")?=="current"
            && matches!(txt(sensor,"calibration_state","$.sensors[]")?,"current"|"not_applicable");
        let deps=set(sensor,"dependency_groups","$.sensors[]")?;
        let scenario_removed=removed_sensors.contains(sid)||deps.iter().any(|g|removed_dependencies.contains(g));
        let availability=if scenario_removed{"MISSING"}else if !ok{"UNKNOWN"}else{txt(r,"availability","$.responses[]")?};
        let (lo,hi)=if availability=="VALID"{(r["value_lower"].clone(),r["value_upper"].clone())}else{(Value::Null,Value::Null)};
        let role=txt(sensor,"source_role","$.sensors[]")?;
        let derived=set(sensor,"derived_from_channel_ids","$.sensors[]")?;
        let evidence_role=if evidence_class=="SYNTHETIC_CONFORMANCE_FIXTURE"&&role=="synthetic"&&derived.is_empty(){
            "SYNTHETIC_CONFORMANCE_FIXTURE"
        }else if evidence_class=="MEASURED"&&ok&&role=="measured"&&derived.is_empty(){
            "INDEPENDENT_MEASUREMENT"
        }else{"MODEL_ESTIMATE"};
        responses.push(json!({
            "response_id":r["response_id"],"state_id":r["state_id"],"channel_id":cid,"availability":availability,
            "value_lower":lo,"value_upper":hi,"unit":r["unit"],"evidence_role":evidence_role,"domain":identity,
            "measurement_window_ref":r["measurement_window_ref"],"source_ref":r["artifact_id"],"adjudication_ref":r["adjudication_ref"]
        }));
    }
    let case=json!({"schema_version":1,"case_id":input["assessment_id"],"evidence_class":evidence_class,
        "parent_context":parent_context,"measurement_system_case_id":installation,"layout_profile":input["layout_profile"],
        "channels":channels,"states":states,"required_state_pairs":required_pairs,"responses":responses});
    Ok((case,parent))
}

fn pair_state(status:&str)->&'static str{
    match status{"IDENTIFIABLE"=>"isolable","NOT_IDENTIFIABLE"=>"non_isolable",_=>"not_evaluated"}
}
fn evidence_role_name(input:&Value,response:&Value)->&'static str{
    if input["evidence_class"]=="SYNTHETIC_CONFORMANCE_FIXTURE"&&response["evidence_role"]=="SYNTHETIC_CONFORMANCE_FIXTURE"{"synthetic"}
    else if response["evidence_role"]=="INDEPENDENT_MEASUREMENT"{"measured"}
    else if response["evidence_role"]=="MODEL_ESTIMATE"{"simulated"}else{"expert_authored"}
}
fn pair_assessments(input:&Value,report:&EvaluationReport)->Vec<Value>{
    let declarations=input["required_fault_pairs"].as_array().unwrap();
    let responses=input["responses"].as_array().unwrap();
    report.pair_results.iter().map(|result|{
        let declaration=declarations.iter().find(|p|p["pair_id"]==result.pair_id).unwrap();
        let mut seen=BTreeSet::new();
        let refs:Vec<Value>=responses.iter().filter(|r|{
            (r["state_id"]==result.state_a_id||r["state_id"]==result.state_b_id)
            &&result.discriminating_channel_ids.iter().any(|c|r["channel_id"].as_str()==Some(c.as_str()))
        }).filter_map(|r|{
            let key=format!("{}:{}",r["artifact_id"].as_str().unwrap_or(""),r["source_sha256"].as_str().unwrap_or(""));
            if !seen.insert(key){return None;}
            Some(json!({"artifact_id":r["artifact_id"],"sha256":r["source_sha256"],"evidence_role":evidence_role_name(input,r),"channel_ids":[r["channel_id"]]}))
        }).collect();
        json!({"fault_a":declaration["fault_a"],"fault_b":declaration["fault_b"],"state":pair_state(&result.status),"method":"structural","evidence_refs":refs})
    }).collect()
}
fn make_report(input:&Value,base:&EvaluationReport,scenarios:Vec<Value>)->Value{
    let asset=&input["asset"];let domain=&input["operating_domain"];let digests=&input["evidence_digests"];
    let sensors:Vec<Value>=input["sensors"].as_array().unwrap().iter().map(|s|json!({
        "sensor_id":s["sensor_id"],"channel_id":s["channel_id"],"installed":s["installed"],
        "measurement_principle":s["measurement_principle_ref"],"unit":s["unit"],"source_role":s["source_role"],
        "quality_state":s["quality_state"],"calibration_state":s["calibration_state"],
        "dependency_groups":s["dependency_groups"],"derived_from_channel_ids":s["derived_from_channel_ids"]
    })).collect();
    let stratum=domain["regime_id"].as_str().unwrap_or("unknown");
    let metric=|id:&str,value:usize|json!({"metric_id":id,"value":value,"unit":"count","stratum":stratum,"sample_count":0});
    json!({
        "schema_version":"0.1.0","assessment_id":input["assessment_id"],"generated_at":input["generated_at"],
        "asset":{"asset_id":asset["asset_id"],"asset_class":asset["asset_class"],"vessel_id":asset["vessel_id"],"engine_id":asset["engine_id"],"configuration_revision":asset["configuration_revision"],"configuration_sha256":asset["configuration_sha256"]},
        "operating_domain":{"regime_id":domain["regime_id"],"description":domain["description"],"qualification_state":domain["qualification_state"],"load_percent":domain["load_percent"],"speed_rpm":domain["speed_rpm"],"transient_state":domain["transient_state"]},
        "evidence_digests":{"sensor_topology_sha256":digests["sensor_topology_sha256"],"fault_model_sha256":digests["fault_model_sha256"],"evaluator_sha256":digests["evaluator_sha256"],"dataset_sha256":digests["dataset_sha256"],"split_manifest_sha256":digests["split_manifest_sha256"],"model_sha256":digests["model_sha256"]},
        "sensor_inventory":sensors,"fault_assessments":[],"pair_assessments":pair_assessments(input,base),"sensor_loss_scenarios":scenarios,
        "metrics":[metric("installed_sensor_count",base.channel_count),metric("independent_evidence_group_count",base.independence_group_count),
            metric("state_hypothesis_count",base.state_count),metric("required_fault_pair_count",base.required_pair_count),
            metric("identifiable_fault_pair_count",base.identifiable_pair_count),metric("non_identifiable_fault_pair_count",base.not_identifiable_pair_count),
            metric("insufficient_evidence_fault_pair_count",base.insufficient_evidence_pair_count),metric("critical_unidentifiable_fault_pair_count",base.critical_unidentifiable_pair_count)],
        "authority_boundary":{"read_only":true,"actuation":false,"maintenance_authorization":false,"mode_transition_authorization":false,"restriction_clearance":false},
        "claim_ceiling":"Pairwise structural diagnosability only for the exact declared asset/configuration, installed sensors, dependency groups, operating domain, response intervals and evidence class. No individual-fault detectability, trained model accuracy, physical engine validation, shipboard safety, maintenance authorization, ODD clearance or permission to operate."
    })
}

pub fn evaluate_maritime_case(input:&Value)->Check<Value>{
    validate_input(input)?;
    let empty=BTreeSet::new();
    let (case,parent)=make_parent_and_case(input,&empty,&empty)?;
    let baseline=evaluate_case(&case,&parent)?;
    let mut scenarios_out=Vec::new();
    for scenario in input["common_cause_scenarios"].as_array().unwrap(){
        let removed_sensors=set(scenario,"removed_sensor_ids","$.common_cause_scenarios[]")?;
        let removed_dependencies=set(scenario,"removed_dependency_groups","$.common_cause_scenarios[]")?;
        let (scenario_case,scenario_parent)=make_parent_and_case(input,&removed_sensors,&removed_dependencies)?;
        let evaluated=evaluate_case(&scenario_case,&scenario_parent)?;
        let after:BTreeMap<String,String>=evaluated.pair_results.iter().map(|p|(p.pair_id.clone(),p.status.clone())).collect();
        let lost:Vec<Value>=baseline.pair_results.iter().filter(|p|{
            p.status=="IDENTIFIABLE"&&after.get(&p.pair_id).is_some_and(|s|s!="IDENTIFIABLE")
        }).map(|p|json!({"fault_a":p.state_a_id,"fault_b":p.state_b_id})).collect();
        scenarios_out.push(json!({"scenario_id":scenario["scenario_id"],"removed_sensor_ids":scenario["removed_sensor_ids"],
            "removed_dependency_groups":scenario["removed_dependency_groups"],"assessment_state":"recomputed","lost_isolability_pairs":lost}));
    }
    Ok(make_report(input,&baseline,scenarios_out))
}

pub fn evaluate_and_bind_execution(input:&Value,source_commit:&str,case_sha256:&str)->Check<Value>{
    if source_commit.len()!=40||!source_commit.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)){
        return Err(err("INVALID_SOURCE_COMMIT","$.execution.source_commit","expected lowercase 40-hex commit"));
    }
    if !hex64(case_sha256){return Err(err("INVALID_CASE_DIGEST","$.execution.case_sha256","expected lowercase SHA-256"));}
    let mut report=evaluate_maritime_case(input)?;
    report["execution"]=json!({"source_commit":source_commit,"case_sha256":case_sha256});
    Ok(report)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn fixture()->Value{
        serde_json::from_str(include_str!("../../../benchmarks/maritime-observability/maritime-observability-case-v0.1.json")).expect("fixture JSON")
    }
    #[test]
    fn synthetic_maritime_case_maps_to_generic_evaluator(){
        let r=evaluate_maritime_case(&fixture()).expect("adapter evaluates");
        assert_eq!(r["pair_assessments"].as_array().unwrap().len(),5);
        assert_eq!(r["fault_assessments"].as_array().unwrap().len(),0);
        assert_eq!(r["authority_boundary"]["actuation"],false);
    }
    #[test]
    fn gateway_loss_recomputes_and_withdraws_pair_isolability(){
        let r=evaluate_maritime_case(&fixture()).expect("adapter evaluates");
        assert_eq!(r["sensor_loss_scenarios"][0]["assessment_state"],"recomputed");
        assert!(!r["sensor_loss_scenarios"][0]["lost_isolability_pairs"].as_array().unwrap().is_empty());
    }
    #[test]
    fn expired_exhaust_temperature_calibration_removes_only_its_supported_pair(){
        let mut c=fixture();c["sensors"][2]["calibration_state"]=json!("expired");
        let r=evaluate_maritime_case(&c).expect("abstention is a valid report");
        let p=r["pair_assessments"].as_array().unwrap().iter().find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]=="ENGINE_COOLING_RESTRICTION").unwrap();
        assert_ne!(p["state"],"isolable");
    }
    #[test]
    fn derived_signal_is_not_promoted_to_independent_measurement(){
        let mut c=fixture();c["sensors"][0]["source_role"]=json!("derived");
        c["sensors"][0]["derived_from_channel_ids"]=json!(["shaft-speed"]);
        let r=evaluate_maritime_case(&c).expect("derived channel remains non-independent");
        let p=r["pair_assessments"].as_array().unwrap().iter().find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]=="ENGINE_FUEL_INJECTION_FAULT").unwrap();
        assert!(!p["evidence_refs"].as_array().unwrap().iter().any(|e|e["channel_ids"][0]=="fuel-rail-pressure"));
    }
    #[test]
    fn report_preserves_configuration_and_authority_boundary(){
        let r=evaluate_maritime_case(&fixture()).unwrap();
        assert_eq!(r["asset"]["configuration_revision"],"engine-r1");
        assert_eq!(r["authority_boundary"]["read_only"],true);
        assert_eq!(r["authority_boundary"]["maintenance_authorization"],false);
        assert_eq!(r["authority_boundary"]["mode_transition_authorization"],false);
    }
    #[test]
    fn exact_execution_bindings_are_required(){
        let c=fixture();
        assert!(evaluate_and_bind_execution(&c,&"d".repeat(40),&"e".repeat(64)).is_ok());
        assert!(evaluate_and_bind_execution(&c,"not-a-commit",&"e".repeat(64)).is_err());
    }
    #[test]
    fn same_input_is_deterministic(){
        let c=fixture();assert_eq!(evaluate_maritime_case(&c).unwrap(),evaluate_maritime_case(&c).unwrap());
    }
}
