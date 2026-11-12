//! Maritime adapter over the reusable domain-neutral sensor observability evaluator.
//! This crate owns maritime identity/dependency mapping and report translation;
//! pairwise interval separation remains in symtropy-sensor-observability-bench.
#![forbid(unsafe_code)]
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};
use symtropy_sensor_observability_bench::{classify_pair_signatures, evaluate_case, ChannelPairSignature, EvaluationReport, PairAvailability, PairEvidenceClass, PairEvidenceRole, Violation};

type Check<T> = Result<T, Vec<Violation>>;

fn err(code:&str,path:&str,detail:impl Into<String>)->Vec<Violation>{
    vec![Violation{code:code.into(),path:path.into(),detail:detail.into()}]
}
fn allowed_keys(v:&Value,keys:&[&str],path:&str)->Check<()>{
    let Some(object)=v.as_object() else{
        return Err(err("EXPECTED_OBJECT",path,"expected object"));
    };
    let allowed:BTreeSet<&str>=keys.iter().copied().collect();
    if let Some(extra)=object.keys().find(|key|!allowed.contains(key.as_str())){
        return Err(err("UNKNOWN_FIELD",&format!("{path}.{extra}"),"field is not declared by the versioned case contract"));
    }
    Ok(())
}
fn pair_key(a:&str,b:&str)->String{
    if a<=b{format!("{a}|{b}")}else{format!("{b}|{a}")}
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

fn failure_cut_bounds(profile:&Value)->Check<(usize,usize)>{
    let atom_limit=profile.get("maximum_failure_atoms_for_exact_cut_analysis").and_then(Value::as_u64);
    let order_limit=profile.get("maximum_failure_cut_order").and_then(Value::as_u64);
    let (atoms,order)=match (atom_limit,order_limit){
        (None,None)=>(12usize,2usize),
        (Some(a),Some(o))=>(a as usize,o as usize),
        _=>return Err(err("INCOMPLETE_FAILURE_CUT_BOUNDS","$.layout_profile","maximum failure atom and cut-order bounds must be supplied together")),
    };
    if atoms==0||atoms>16||order==0||order>2||order>atoms{
        return Err(err("INVALID_FAILURE_CUT_BOUNDS","$.layout_profile","v0.1 exact analysis permits 1..=16 atoms and cut order 1..=2"));
    }
    Ok((atoms,order))
}
fn sensor_timebase_qualified(sensor:&Value,domain:&Value,profile:&Value,evidence_class:&str)->bool{
    let Some(expected_id)=domain["measurement_timebase_id"].as_str() else{return false};
    let Some(actual_id)=sensor["timebase_id"].as_str() else{return false};
    let Some(clock_domain)=sensor["clock_domain"].as_str().filter(|s|!s.trim().is_empty()) else{return false};
    let Some(state)=sensor["timebase_qualification_state"].as_str() else{return false};
    let Some(source)=sensor["timestamp_uncertainty_source"].as_str() else{return false};
    let Some(uncertainty)=sensor["timestamp_uncertainty_ms"].as_f64().filter(|x|x.is_finite()&&*x>=0.0) else{return false};
    let Some(maximum)=profile["maximum_timestamp_uncertainty_ms"].as_f64().filter(|x|x.is_finite()&&*x>=0.0) else{return false};
    let Some(digest)=sensor["timebase_qualification_sha256"].as_str() else{return false};
    let Some(artifact)=sensor["timebase_qualification_artifact_id"].as_str().filter(|s|!s.trim().is_empty()) else{return false};
    let role_matches=if evidence_class=="SYNTHETIC_CONFORMANCE_FIXTURE"{
        state=="synthetic_conformance" && source=="synthetic_fixture"
    }else{
        state=="qualified" && matches!(source,"measured_offset_bound"|"instrument_specification"|"vendor_specification")
    };
    !clock_domain.is_empty() && !artifact.is_empty()
        && actual_id==expected_id && role_matches
        && uncertainty<=maximum && hex64(digest)
        && domain["qualification_state"]=="qualified"
}

fn validate_clock_qualification(input:&Value,clock:&Value,clock_sha256:&str)->Check<()>{
    let p="$.clock_qualification";
    allowed_keys(clock,&["schema_version","qualification_id","evidence_class","timebase_id","clock_domain",
        "declared_timestamp_uncertainty_ms","layout_maximum_timestamp_uncertainty_ms","uncertainty_source",
        "mapping_method","hardware_clock_measurements_present","claim_ceiling"],p)?;
    if txt(clock,"schema_version",p)?!="0.1.0"{return Err(err("CLOCK_SCHEMA_VERSION",p,"expected schema 0.1.0"));}
    if !hex64(clock_sha256){return Err(err("CLOCK_FILE_SHA256","$.execution.clock_qualification_sha256","expected lowercase SHA-256"));}
    let qual_id=txt(clock,"qualification_id",p)?;
    let tb_id=txt(clock,"timebase_id",p)?;
    let clock_domain=txt(clock,"clock_domain",p)?;
    let evidence_class=txt(input,"evidence_class","$")?;
    if txt(clock,"evidence_class",p)?!=evidence_class{return Err(err("CLOCK_EVIDENCE_CLASS_MISMATCH",p,"clock and case evidence classes differ"));}
    if txt(&input["operating_domain"],"measurement_timebase_id","$.operating_domain")?!=tb_id{
        return Err(err("CLOCK_TIMEBASE_ID_MISMATCH",p,"clock timebase differs from operating domain"));
    }
    let domain_max=input["operating_domain"]["maximum_timestamp_uncertainty_ms"].as_f64()
        .filter(|x|x.is_finite()&&*x>=0.0).ok_or_else(||err("DOMAIN_TIMEBASE_LIMIT","$.operating_domain.maximum_timestamp_uncertainty_ms","invalid bound"))?;
    let profile_max=input["layout_profile"]["maximum_timestamp_uncertainty_ms"].as_f64()
        .filter(|x|x.is_finite()&&*x>=0.0).ok_or_else(||err("PROFILE_TIMEBASE_LIMIT","$.layout_profile.maximum_timestamp_uncertainty_ms","invalid bound"))?;
    let clock_max=clock["layout_maximum_timestamp_uncertainty_ms"].as_f64()
        .filter(|x|x.is_finite()&&*x>=0.0).ok_or_else(||err("CLOCK_LAYOUT_LIMIT",p,"invalid bound"))?;
    let declared=clock["declared_timestamp_uncertainty_ms"].as_f64()
        .filter(|x|x.is_finite()&&*x>=0.0).ok_or_else(||err("CLOCK_DECLARED_UNCERTAINTY",p,"invalid bound"))?;
    if (domain_max-profile_max).abs()>1e-9||(profile_max-clock_max).abs()>1e-9{
        return Err(err("TIMEBASE_LIMIT_MISMATCH",p,"operating-domain, layout-profile and clock-artifact maxima must match"));
    }
    let hardware=boolean(clock,"hardware_clock_measurements_present",p)?;
    let uncertainty_source=txt(clock,"uncertainty_source",p)?;
    let mapping_method=txt(clock,"mapping_method",p)?;
    let _=txt(clock,"claim_ceiling",p)?;
    let synthetic=evidence_class=="SYNTHETIC_CONFORMANCE_FIXTURE";
    if synthetic{
        if uncertainty_source!="synthetic_fixture"||mapping_method!="identity_mapping_within_one_declared_synthetic_clock_domain"||hardware{
            return Err(err("SYNTHETIC_CLOCK_CLAIM_MISMATCH",p,"synthetic clock must declare identity mapping and no hardware measurements"));
        }
    }else{
        if uncertainty_source=="synthetic_fixture"||mapping_method=="identity_mapping_within_one_declared_synthetic_clock_domain"{
            return Err(err("MEASURED_CLOCK_CLAIM_MISMATCH",p,"measured case cannot use synthetic clock semantics"));
        }
        if !matches!(uncertainty_source,"measured_offset_bound"|"instrument_specification"|"vendor_specification"){
            return Err(err("MEASURED_CLOCK_UNCERTAINTY_SOURCE",p,"unsupported measured clock uncertainty source"));
        }
        if uncertainty_source=="measured_offset_bound"&&!hardware{
            return Err(err("CLOCK_MEASUREMENT_MISSING",p,"measured offset bound requires recorded hardware clock measurements"));
        }
    }
    let mut installed=0usize;
    for (i,sensor) in input["sensors"].as_array().ok_or_else(||err("SENSOR_INVENTORY","$.sensors","expected array"))?.iter().enumerate(){
        if sensor["installed"]!=true{continue;}
        installed+=1;
        let at=format!("$.sensors[{i}]");
        let state=txt(sensor,"timebase_qualification_state",&at)?;
        // An installed sensor can be present while its own clock mapping is
        // unavailable. Preserve it in the report, but never let it contribute
        // physical evidence until independently qualified.
        if state=="unqualified"||state=="unknown"{
            continue;
        }
        if (synthetic&&state!="synthetic_conformance")||(!synthetic&&state!="qualified"){
            return Err(err("CLOCK_SENSOR_QUALIFICATION_STATE",&at,"sensor qualification state differs from artifact class"));
        }
        if txt(sensor,"timebase_id",&at)?!=tb_id
            ||txt(sensor,"timebase_qualification_artifact_id",&at)?!=qual_id
            ||txt(sensor,"timebase_qualification_sha256",&at)?!=clock_sha256
            ||txt(sensor,"timestamp_uncertainty_source",&at)?!=uncertainty_source{
            return Err(err("CLOCK_ARTIFACT_SENSOR_BINDING",&at,"qualified sensor does not bind the exact clock qualification artifact"));
        }
        let uncertainty=sensor["timestamp_uncertainty_ms"].as_f64().filter(|x|x.is_finite()&&*x>=0.0)
            .ok_or_else(||err("CLOCK_SENSOR_UNCERTAINTY",&at,"qualified installed sensor needs finite timestamp uncertainty"))?;
        if uncertainty+1e-9<declared{
            return Err(err("CLOCK_SENSOR_UNCERTAINTY_UNDERSTATED",&at,"sensor uncertainty cannot understate the common-timebase artifact bound"));
        }
        if txt(sensor,"clock_domain",&at)?!=clock_domain{
            return Err(err("CLOCK_DOMAIN_MISMATCH",&at,"v0.1 clock artifact declares one source domain; every qualified sensor must bind that exact domain"));
        }
    }
    if installed==0{return Err(err("NO_INSTALLED_CLOCKED_SENSORS","$.sensors","at least one installed sensor must bind the common timebase"));}
    Ok(())
}


fn validate_trajectory_signature_cases(input:&Value)->Check<()>{
    let cases=array(input,"trajectory_signature_cases","$")?;
    let sensors=input["sensors"].as_array().expect("validated sensors");
    let states=input["states"].as_array().expect("validated states");
    let pairs=input["required_fault_pairs"].as_array().expect("validated pairs");
    let responses=input["responses"].as_array().expect("validated responses");
    let mut case_ids=BTreeSet::new();
    let mut paired_faults=BTreeSet::new();
    for (i,c) in cases.iter().enumerate(){
        let p=format!("$.trajectory_signature_cases[{i}]");
        allowed_keys(c,&["signature_case_id","pair_id","fault_a","fault_b","event_alignment_ref","event_alignment_state",
            "event_alignment_record_id","event_alignment_record_sha256","timebase_id","alignment_uncertainty_ms",
            "sample_period_ms","minimum_separated_duration_ms","minimum_independent_principles","trajectory_channels"],&p)?;
        let id=txt(c,"signature_case_id",&p)?;
        if !case_ids.insert(id.to_owned()){return Err(err("DUPLICATE_TRAJECTORY_SIGNATURE_CASE",&p,"signature case IDs must be unique"));}
        let pair_id=txt(c,"pair_id",&p)?;
        let fault_a=txt(c,"fault_a",&p)?;
        let fault_b=txt(c,"fault_b",&p)?;
        if fault_a==fault_b||!states.iter().any(|s|s["state_id"]==fault_a)||!states.iter().any(|s|s["state_id"]==fault_b){
            return Err(err("TRAJECTORY_FAULT_PAIR",&p,"trajectory pair must reference two distinct declared states"));
        }
        let declaration=pairs.iter().find(|pair|pair["pair_id"]==pair_id
            &&pair["fault_a"]==fault_a&&pair["fault_b"]==fault_b)
            .ok_or_else(||err("TRAJECTORY_PAIR_NOT_DECLARED",&p,"trajectory case must bind the exact ordered required-fault-pair declaration"))?;
        let semantic_key=pair_key(fault_a,fault_b);
        if !paired_faults.insert(semantic_key){return Err(err("DUPLICATE_TRAJECTORY_FAULT_RELATION",&p,"only one trajectory case per unordered fault pair is permitted in v0.1"));}
        let _=declaration;
        let _=txt(c,"event_alignment_ref",&p)?;
        let _=txt(c,"timebase_id",&p)?;
        let alignment_state=txt(c,"event_alignment_state",&p)?;
        if !matches!(alignment_state,"synthetic_conformance"|"independently_reviewed"|"unreviewed"|"unknown"){
            return Err(err("TRAJECTORY_ALIGNMENT_STATE",&p,"unsupported event-alignment evidence state"));
        }
        let record_id=c["event_alignment_record_id"].as_str().filter(|s|!s.trim().is_empty());
        let record_sha=c["event_alignment_record_sha256"].as_str();
        if !c["event_alignment_record_sha256"].is_null()&&record_sha.is_none_or(|s|!hex64(s)){
            return Err(err("TRAJECTORY_ALIGNMENT_SHA256",&p,"alignment digest must be null or lowercase SHA-256"));
        }
        match txt(input,"evidence_class","$")?{
            "SYNTHETIC_CONFORMANCE_FIXTURE"=>{
                if alignment_state=="independently_reviewed"{
                    return Err(err("SYNTHETIC_TRAJECTORY_ALIGNMENT_OVERCLAIM",&p,"synthetic fixture cannot claim independent event-alignment review"));
                }
                if record_id.is_some()||record_sha.is_some(){
                    return Err(err("SYNTHETIC_TRAJECTORY_ALIGNMENT_RECORD",&p,"synthetic alignment cannot carry a physical review record"));
                }
            }
            "MEASURED"=>{
                if alignment_state=="independently_reviewed"{
                    if record_id.is_none()||record_sha.is_none_or(|s|!hex64(s)){
                        return Err(err("INCOMPLETE_TRAJECTORY_ALIGNMENT_REVIEW",&p,"measured trajectory alignment requires review record ID and SHA-256"));
                    }
                }else if record_id.is_some()||record_sha.is_some(){
                    return Err(err("UNREVIEWED_TRAJECTORY_ALIGNMENT_RECORD",&p,"unreviewed alignment cannot carry a review record"));
                }
            }
            _=>unreachable!()
        }
        let alignment_uncertainty=c["alignment_uncertainty_ms"].as_f64()
            .filter(|x|x.is_finite()&&*x>=0.0)
            .ok_or_else(||err("TRAJECTORY_ALIGNMENT_UNCERTAINTY",&p,"alignment uncertainty must be finite and non-negative"))?;
        let sample_period=c["sample_period_ms"].as_u64().filter(|x|*x>0)
            .ok_or_else(||err("TRAJECTORY_SAMPLE_PERIOD",&p,"sample period must be a positive integer"))?;
        let minimum_duration=c["minimum_separated_duration_ms"].as_f64()
            .filter(|x|x.is_finite()&&*x>0.0)
            .ok_or_else(||err("TRAJECTORY_MINIMUM_DURATION",&p,"minimum separated duration must be finite and positive"))?;
        let minimum_principles=c["minimum_independent_principles"].as_u64().filter(|x|*x>0)
            .ok_or_else(||err("TRAJECTORY_PRINCIPLE_THRESHOLD",&p,"minimum independent principles must be positive"))?;
        let profile_min=input["layout_profile"]["minimum_independent_principles_for_critical_pairs"].as_u64().unwrap_or(1);
        if declaration["critical"]==true&&minimum_principles<profile_min{
            return Err(err("TRAJECTORY_THRESHOLD_BELOW_PROFILE",&p,"critical trajectory pair cannot undercut the layout profile"));
        }
        let channels=array(c,"trajectory_channels",&p)?;
        if channels.is_empty(){return Err(err("EMPTY_TRAJECTORY_CHANNELS",&p,"at least one temporal channel is required"));}
        let mut sensor_seen=BTreeSet::new();
        for (j,ch) in channels.iter().enumerate(){
            let at=format!("{p}.trajectory_channels[{j}]");
            allowed_keys(ch,&["sensor_id","channel_id","measurement_window_ref_a","measurement_window_ref_b","artifact_id_a","artifact_id_b","samples_a","samples_b"],&at)?;
            let sensor_id=txt(ch,"sensor_id",&at)?;
            let channel_id=txt(ch,"channel_id",&at)?;
            if !sensor_seen.insert(sensor_id.to_owned()){return Err(err("DUPLICATE_TRAJECTORY_SENSOR",&at,"one trajectory channel per sensor is permitted in v0.1"));}
            let _sensor=sensors.iter().find(|s|s["sensor_id"]==sensor_id&&s["channel_id"]==channel_id)
                .ok_or_else(||err("UNKNOWN_TRAJECTORY_SENSOR",&at,"trajectory sensor/channel binding does not resolve"))?;
            let window_a=txt(ch,"measurement_window_ref_a",&at)?;
            let window_b=txt(ch,"measurement_window_ref_b",&at)?;
            let artifact_a=txt(ch,"artifact_id_a",&at)?;
            let artifact_b=txt(ch,"artifact_id_b",&at)?;
            let response_a=responses.iter().find(|r|r["state_id"]==fault_a&&r["channel_id"]==channel_id)
                .ok_or_else(||err("MISSING_TRAJECTORY_RESPONSE_A",&at,"static state-A response not found"))?;
            let response_b=responses.iter().find(|r|r["state_id"]==fault_b&&r["channel_id"]==channel_id)
                .ok_or_else(||err("MISSING_TRAJECTORY_RESPONSE_B",&at,"static state-B response not found"))?;
            if response_a["measurement_window_ref"]!=window_a||response_b["measurement_window_ref"]!=window_b
                ||response_a["artifact_id"]!=artifact_a||response_b["artifact_id"]!=artifact_b{
                return Err(err("TRAJECTORY_SOURCE_BINDING_MISMATCH",&at,"trajectory must bind the exact static response windows and source artifact identities"));
            }
            let a=array(ch,"samples_a",&at)?;
            let b=array(ch,"samples_b",&at)?;
            if a.len()<2||a.len()!=b.len(){return Err(err("TRAJECTORY_SAMPLE_COUNT",&at,"state A/B trajectories need equal sample counts of at least two"));}
            let mut previous=None;
            for (k,(sa,sb)) in a.iter().zip(b.iter()).enumerate(){
                let pa=format!("{at}.samples_a[{k}]");
                let pb=format!("{at}.samples_b[{k}]");
                allowed_keys(sa,&["offset_ms","value_lower","value_upper"],&pa)?;
                allowed_keys(sb,&["offset_ms","value_lower","value_upper"],&pb)?;
                let ta=sa["offset_ms"].as_u64().ok_or_else(||err("TRAJECTORY_OFFSET",&pa,"offset must be a non-negative integer"))?;
                let tb=sb["offset_ms"].as_u64().ok_or_else(||err("TRAJECTORY_OFFSET",&pb,"offset must be a non-negative integer"))?;
                if ta!=tb{return Err(err("TRAJECTORY_OFFSET_MISMATCH",&at,"state A/B samples must share identical event-relative offsets"));}
                if previous.is_some_and(|prev|ta<=prev||ta-prev!=sample_period){
                    return Err(err("TRAJECTORY_SAMPLE_GRID",&at,"sample offsets must be strictly increasing on the declared regular grid"));
                }
                previous=Some(ta);
                for (sample,path) in [(sa,&pa),(sb,&pb)]{
                    let lo=sample["value_lower"].as_f64().filter(|x|x.is_finite());
                    let hi=sample["value_upper"].as_f64().filter(|x|x.is_finite());
                    if lo.zip(hi).is_none_or(|(lo,hi)|lo>hi){return Err(err("TRAJECTORY_VALUE_INTERVAL",path,"trajectory sample needs a finite ordered interval"));}
                }
            }
            if previous.unwrap_or(0) as f64-a[0]["offset_ms"].as_f64().unwrap_or(0.0)<minimum_duration{
                return Err(err("TRAJECTORY_DURATION_UNSUPPORTED",&at,"trajectory does not contain enough time span for its minimum separation duration"));
            }
            if alignment_uncertainty+1e-9<0.0{return Err(err("TRAJECTORY_TIME_GRID",&at,"invalid alignment uncertainty"));}
        }
    }
    Ok(())
}

fn validate_input(input:&Value)->Check<()>{
    if !input.is_object(){return Err(err("EXPECTED_OBJECT","$","case must be an object"));}
    allowed_keys(input,&["schema_version","assessment_id","generated_at","evidence_class","asset","operating_domain","evidence_digests","layout_profile","sensors","states","required_fault_pairs","responses","common_cause_scenarios","trajectory_signature_cases","claim_ceiling"],"$")?;
    if txt(input,"schema_version","$")?!="0.1.0"{return Err(err("SCHEMA_VERSION","$.schema_version","expected 0.1.0"));}
    if !matches!(txt(input,"evidence_class","$")?,"SYNTHETIC_CONFORMANCE_FIXTURE"|"MEASURED"){
        return Err(err("EVIDENCE_CLASS","$.evidence_class","unsupported evidence class"));
    }
    for k in ["assessment_id","generated_at","claim_ceiling"]{let _=txt(input,k,"$")?;}
    let asset=&input["asset"];
    allowed_keys(asset,&["asset_id","asset_class","vessel_id","engine_id","engine_model","configuration_revision","configuration_sha256"],"$.asset")?;
    for k in ["asset_id","asset_class","engine_model","configuration_revision"]{let _=txt(asset,k,"$.asset")?;}
    if !hex64(txt(asset,"configuration_sha256","$.asset")?){return Err(err("CONFIG_SHA256","$.asset.configuration_sha256","expected lowercase SHA-256"));}
    let regime=&input["operating_domain"];
    allowed_keys(regime,&["regime_id","description","qualification_state","load_percent","speed_rpm","transient_state","measurement_timebase_id","maximum_timestamp_uncertainty_ms"],"$.operating_domain")?;
    for k in ["regime_id","description","qualification_state","transient_state","measurement_timebase_id"]{let _=txt(regime,k,"$.operating_domain")?;}
    if !matches!(txt(regime,"qualification_state","$.operating_domain")?,"qualified"|"unsupported"|"not_evaluated"){
        return Err(err("OPERATING_DOMAIN_QUALIFICATION","$.operating_domain.qualification_state","unsupported qualification state"));
    }
    if regime["maximum_timestamp_uncertainty_ms"].as_f64().is_none_or(|x|!x.is_finite()||x<0.0){
        return Err(err("TIMEBASE_UNCERTAINTY_LIMIT","$.operating_domain.maximum_timestamp_uncertainty_ms","must be a finite non-negative millisecond bound"));
    }
    let digests=&input["evidence_digests"];
    allowed_keys(digests,&["sensor_topology_sha256","fault_model_sha256","evaluator_sha256","dataset_sha256","split_manifest_sha256","model_sha256"],"$.evidence_digests")?;
    for k in ["sensor_topology_sha256","fault_model_sha256","evaluator_sha256","dataset_sha256","split_manifest_sha256","model_sha256"]{
        if !digests.get(k).is_some_and(Value::is_null){
            return Err(err("UNBOUND_EVIDENCE_DIGEST",&format!("$.evidence_digests.{k}"),
                "v0.1 has no separate artifact bytes for this digest; keep it null rather than inventing a SHA-256"));
        }
    }
    let profile=&input["layout_profile"];
    allowed_keys(profile,&["layout_profile_ref","state_taxonomy_ref","required_pair_set_ref","reference_state_id","minimum_independent_principles_for_critical_pairs","maximum_timestamp_uncertainty_ms","minimum_common_window_overlap_ms","maximum_failure_atoms_for_exact_cut_analysis","maximum_failure_cut_order"],"$.layout_profile")?;
    for k in ["layout_profile_ref","state_taxonomy_ref","required_pair_set_ref","reference_state_id"]{let _=txt(profile,k,"$.layout_profile")?;}
    let min=profile["minimum_independent_principles_for_critical_pairs"].as_u64().filter(|n|*n>0)
        .ok_or_else(||err("INDEPENDENCE_THRESHOLD","$.layout_profile.minimum_independent_principles_for_critical_pairs","must be positive"))?;
    if profile["maximum_timestamp_uncertainty_ms"].as_f64().is_none_or(|x|!x.is_finite()||x<0.0){
        return Err(err("PROFILE_TIMEBASE_LIMIT","$.layout_profile.maximum_timestamp_uncertainty_ms","must be a finite non-negative millisecond bound"));
    }
    if profile["minimum_common_window_overlap_ms"].as_f64().is_none_or(|x|!x.is_finite()||x<=0.0){
        return Err(err("MINIMUM_COMMON_WINDOW_OVERLAP","$.layout_profile.minimum_common_window_overlap_ms","must be a finite positive millisecond duration"));
    }
    let _=failure_cut_bounds(profile)?;
    let domain_max=regime["maximum_timestamp_uncertainty_ms"].as_f64().unwrap();
    let profile_max=profile["maximum_timestamp_uncertainty_ms"].as_f64().unwrap();
    if (domain_max-profile_max).abs()>1e-9{
        return Err(err("TIMEBASE_LIMIT_MISMATCH","$.operating_domain.maximum_timestamp_uncertainty_ms","operating-domain and layout-profile bounds must match"));
    }
    let sensors=array(input,"sensors","$")?;
    if sensors.is_empty(){return Err(err("EMPTY_SENSOR_INVENTORY","$.sensors","at least one sensor required"));}
    let mut sensor_ids=BTreeSet::new();let mut channel_ids=BTreeSet::new();let mut dependencies=BTreeSet::new();
    for (i,s) in sensors.iter().enumerate(){
        let p=format!("$.sensors[{i}]");
        allowed_keys(s,&["sensor_id","channel_id","installed","measurement_principle_ref","measurand","technique","unit","physical_location_ref","calibration_ref","calibration_state","quality_state","source_role","dependency_groups","dependency_review_state","dependency_review_record_id","dependency_review_record_sha256","derived_from_channel_ids","source_artifact_ref","clock_domain","timebase_id","timestamp_uncertainty_ms","timestamp_uncertainty_source","timebase_qualification_state","timebase_qualification_artifact_id","timebase_qualification_sha256"],&p)?;
        let sid=txt(s,"sensor_id",&p)?;let cid=txt(s,"channel_id",&p)?;
        if !sensor_ids.insert(sid.to_owned())||!channel_ids.insert(cid.to_owned()){return Err(err("DUPLICATE_SENSOR_CHANNEL_ID",&p,"IDs must be unique"));}
        for k in ["measurement_principle_ref","measurand","technique","unit","physical_location_ref","calibration_ref","calibration_state","quality_state","source_role","source_artifact_ref","clock_domain","timebase_id","timestamp_uncertainty_source","timebase_qualification_state"]{let _=txt(s,k,&p)?;}
        if !matches!(txt(s,"timestamp_uncertainty_source",&p)?,"measured_offset_bound"|"instrument_specification"|"vendor_specification"|"estimated"|"unknown"|"synthetic_fixture"){
            return Err(err("TIMESTAMP_UNCERTAINTY_SOURCE",&format!("{p}.timestamp_uncertainty_source"),"unsupported uncertainty provenance"));
        }
        if !matches!(txt(s,"timebase_qualification_state",&p)?,"qualified"|"synthetic_conformance"|"unqualified"|"unknown"){
            return Err(err("TIMEBASE_QUALIFICATION_STATE",&format!("{p}.timebase_qualification_state"),"unsupported timebase qualification state"));
        }
        if !s["timestamp_uncertainty_ms"].is_null()
            && s["timestamp_uncertainty_ms"].as_f64().is_none_or(|x|!x.is_finite()||x<0.0){
            return Err(err("TIMESTAMP_UNCERTAINTY",&format!("{p}.timestamp_uncertainty_ms"),"uncertainty must be null or finite and non-negative"));
        }
        if !s["timebase_qualification_sha256"].is_null()
            && !s["timebase_qualification_sha256"].as_str().is_some_and(hex64){
            return Err(err("TIMEBASE_QUALIFICATION_SHA256",&format!("{p}.timebase_qualification_sha256"),"digest must be null or lowercase SHA-256"));
        }
        if !s["timebase_qualification_artifact_id"].is_null()
            && s["timebase_qualification_artifact_id"].as_str().is_none_or(|x|x.trim().is_empty()){
            return Err(err("TIMEBASE_QUALIFICATION_ARTIFACT",&format!("{p}.timebase_qualification_artifact_id"),"artifact ID must be null or non-empty"));
        }
        if matches!(txt(s,"timebase_qualification_state",&p)?,"qualified"|"synthetic_conformance")
            && (s["timestamp_uncertainty_ms"].as_f64().is_none()
                || s["timebase_qualification_sha256"].as_str().is_none_or(|x|!hex64(x))
                || s["timebase_qualification_artifact_id"].as_str().is_none_or(|x|x.trim().is_empty())){
            return Err(err("INCOMPLETE_TIMEBASE_QUALIFICATION",&p,"qualified timebase requires a numeric uncertainty bound and exact artifact ID/SHA-256"));
        }
        let installed=boolean(s,"installed",&p)?;
        let dep_groups=set(s,"dependency_groups",&p)?;
        if installed&&dep_groups.is_empty(){
            return Err(err("INSTALLED_SENSOR_WITHOUT_DEPENDENCIES",&p,"every installed sensor must declare its reviewed dependency groups"));
        }
        dependencies.extend(dep_groups);
        let review_state=txt(s,"dependency_review_state",&p)?;
        if !matches!(review_state,"synthetic_conformance"|"independently_reviewed"|"unreviewed"|"unknown"){
            return Err(err("DEPENDENCY_REVIEW_STATE",&format!("{p}.dependency_review_state"),"unsupported dependency-review state"));
        }
        let review_id=s["dependency_review_record_id"].as_str().filter(|x|!x.trim().is_empty());
        let review_sha=s["dependency_review_record_sha256"].as_str();
        if !s["dependency_review_record_sha256"].is_null()&&review_sha.is_none_or(|x|!hex64(x)){
            return Err(err("DEPENDENCY_REVIEW_SHA256",&format!("{p}.dependency_review_record_sha256"),"digest must be null or lowercase SHA-256"));
        }
        match txt(input,"evidence_class","$")? {
            "SYNTHETIC_CONFORMANCE_FIXTURE" => {
                if review_state=="independently_reviewed"{
                    return Err(err("SYNTHETIC_DEPENDENCY_REVIEW_OVERCLAIM",&format!("{p}.dependency_review_state"),"synthetic fixtures cannot claim independent physical dependency review"));
                }
                if review_state=="synthetic_conformance"{
                    if review_id.is_some()||review_sha.is_some(){
                        return Err(err("SYNTHETIC_DEPENDENCY_REVIEW_BINDING",&p,"synthetic conformance review must not carry a physical review record"));
                    }
                }else if review_id.is_some()||review_sha.is_some(){
                    return Err(err("UNREVIEWED_DEPENDENCY_HAS_RECORD",&p,"unreviewed dependency state must not carry a review record"));
                }
            }
            "MEASURED" => {
                if review_state=="independently_reviewed"{
                    if review_id.is_none()||review_sha.is_none_or(|x|!hex64(x)){
                        return Err(err("INCOMPLETE_DEPENDENCY_REVIEW",&p,"independent dependency review requires exact record ID and SHA-256"));
                    }
                }else if review_id.is_some()||review_sha.is_some(){
                    return Err(err("UNREVIEWED_DEPENDENCY_HAS_RECORD",&p,"unreviewed dependency state must not carry a review record"));
                }
            }
            _=>unreachable!()
        }
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
        let p=format!("$.states[{i}]");
        allowed_keys(s,&["state_id","state_class","faulted_sensor_ids"],&p)?;
        let id=txt(s,"state_id",&p)?;
        if !state_ids.insert(id.to_owned()){return Err(err("DUPLICATE_STATE_ID",&format!("{p}.state_id"),"state IDs must be unique"));}
        if !matches!(txt(s,"state_class",&p)?,"PROCESS_STATE"|"SENSOR_FAULT"|"OUT_OF_DOMAIN"){return Err(err("STATE_CLASS",&format!("{p}.state_class"),"unsupported state class"));}
        let faulted=set(s,"faulted_sensor_ids",&p)?;
        if txt(s,"state_class",&p)?=="SENSOR_FAULT"&&faulted.is_empty(){return Err(err("SENSOR_FAULT_WITHOUT_SENSOR",&p,"sensor fault must identify affected sensor"));}
        if txt(s,"state_class",&p)?!="SENSOR_FAULT"&&!faulted.is_empty(){return Err(err("FAULTED_SENSOR_ON_NON_SENSOR_STATE",&p,"only SENSOR_FAULT may name sensor IDs"));}
        if faulted.iter().any(|id|!sensor_ids.contains(id)){return Err(err("UNKNOWN_FAULTED_SENSOR",&p,"unknown sensor ID"));}
    }
    let pairs=array(input,"required_fault_pairs","$")?;
    if pairs.is_empty(){return Err(err("EMPTY_PAIR_SET","$.required_fault_pairs","at least one pair required"));}
    let mut pair_ids=BTreeSet::new();
    let mut semantic_pair_ids=BTreeSet::new();
    for (i,pair) in pairs.iter().enumerate(){
        let p=format!("$.required_fault_pairs[{i}]");
        allowed_keys(pair,&["pair_id","fault_a","fault_b","critical","minimum_independent_principles"],&p)?;
        let id=txt(pair,"pair_id",&p)?;
        if !pair_ids.insert(id.to_owned()){return Err(err("DUPLICATE_PAIR_ID",&p,"pair IDs must be unique"));}
        let a=txt(pair,"fault_a",&p)?;let b=txt(pair,"fault_b",&p)?;
        if a==b||!state_ids.contains(a)||!state_ids.contains(b){return Err(err("INVALID_FAULT_PAIR",&p,"pair must reference distinct declared states"));}
        if !semantic_pair_ids.insert(pair_key(a,b)){return Err(err("DUPLICATE_FAULT_RELATION",&p,"unordered fault pair may only be declared once"));}
        let _=boolean(pair,"critical",&p)?;
        let pair_min=pair["minimum_independent_principles"].as_u64().filter(|n|*n>0)
            .ok_or_else(||err("PAIR_THRESHOLD_INVALID",&p,"pair minimum must be positive"))?;
        if boolean(pair,"critical",&p)? && pair_min<min{return Err(err("PAIR_THRESHOLD_BELOW_PROFILE",&p,"critical pair minimum cannot undercut layout profile"));}
    }
    let reference_state=txt(profile,"reference_state_id","$.layout_profile")?;
    let reference=states.iter().find(|s|s["state_id"]==reference_state)
        .ok_or_else(||err("UNKNOWN_REFERENCE_STATE","$.layout_profile.reference_state_id","reference state must exist"))?;
    if txt(reference,"state_class","$.states[]")?!="PROCESS_STATE"{
        return Err(err("INVALID_REFERENCE_STATE","$.layout_profile.reference_state_id","reference state must be a PROCESS_STATE"));
    }
    // Require every pair among in-domain states, plus the declared reference vs each OOD state.
    let mut expected_pairs=BTreeSet::new();
    let in_domain:Vec<&str>=states.iter().filter(|s|s["state_class"]!="OUT_OF_DOMAIN")
        .map(|s|s["state_id"].as_str().unwrap()).collect();
    for i in 0..in_domain.len(){for j in (i+1)..in_domain.len(){expected_pairs.insert(pair_key(in_domain[i],in_domain[j]));}}
    for state in states.iter().filter(|s|s["state_class"]=="OUT_OF_DOMAIN"){
        expected_pairs.insert(pair_key(reference_state,txt(state,"state_id","$.states[]")?));
    }
    let responses=array(input,"responses","$")?;
    let installed_channels:BTreeSet<String>=sensors.iter().filter(|s|s["installed"]==true).filter_map(|s|s["channel_id"].as_str().map(str::to_owned)).collect();
    let mut seen=BTreeSet::new();
    let mut response_ids=BTreeSet::new();
    let mut measurement_window_by_state:BTreeMap<String,String>=BTreeMap::new();
    for (i,r) in responses.iter().enumerate(){
        let p=format!("$.responses[{i}]");
        allowed_keys(r,&["response_id","state_id","channel_id","availability","value_lower","value_upper","unit","evidence_role","measurement_window_ref","window_start_ms","window_end_ms","artifact_id","source_sha256","adjudication_ref"],&p)?;
        let response_id=txt(r,"response_id",&p)?;
        if !response_ids.insert(response_id.to_owned()){return Err(err("DUPLICATE_RESPONSE_ID",&p,"response IDs must be unique"));}
        let state=txt(r,"state_id",&p)?;let ch=txt(r,"channel_id",&p)?;
        if !state_ids.contains(state)||!channel_ids.contains(ch){return Err(err("UNKNOWN_RESPONSE_IDENTITY",&p,"unknown state/channel"));}
        if !seen.insert((state.to_owned(),ch.to_owned())){return Err(err("DUPLICATE_RESPONSE_CELL",&p,"duplicate state/channel response"));}
        let availability=txt(r,"availability",&p)?;
        if !matches!(availability,"VALID"|"DEGRADED"|"MISSING"|"UNKNOWN"){return Err(err("RESPONSE_AVAILABILITY",&p,"unknown availability"));}
        for k in ["unit","evidence_role","measurement_window_ref","artifact_id","adjudication_ref"]{let _=txt(r,k,&p)?;}
        let window_start=r["window_start_ms"].as_f64().filter(|x|x.is_finite()&&*x>=0.0)
            .ok_or_else(||err("RESPONSE_WINDOW_START",&format!("{p}.window_start_ms"),"expected finite non-negative timestamp"))?;
        let window_end=r["window_end_ms"].as_f64().filter(|x|x.is_finite()&&*x>=0.0)
            .ok_or_else(||err("RESPONSE_WINDOW_END",&format!("{p}.window_end_ms"),"expected finite non-negative timestamp"))?;
        if window_end<=window_start{return Err(err("INVALID_RESPONSE_WINDOW",&p,"measurement window end must be strictly later than start"));}
        let window=txt(r,"measurement_window_ref",&p)?;
        match measurement_window_by_state.get(state) {
            Some(existing) if existing!=window => {
                return Err(err("INCOHERENT_STATE_MEASUREMENT_WINDOW",&p,
                    "all sensor responses for one fault state must refer to the same synchronized measurement window"));
            }
            None => { measurement_window_by_state.insert(state.to_owned(),window.to_owned()); }
            _ => {}
        }
        if !matches!(txt(r,"evidence_role",&p)?,"INDEPENDENT_MEASUREMENT"|"SYNTHETIC_CONFORMANCE_FIXTURE"|"MODEL_ESTIMATE"|"OPERATOR_REPORTED"){
            return Err(err("RESPONSE_EVIDENCE_ROLE",&p,"unsupported response evidence role"));
        }
        if !hex64(txt(r,"source_sha256",&p)?){return Err(err("RESPONSE_SHA256",&p,"expected lowercase SHA-256"));}
        if availability=="VALID"{
            let lo=r["value_lower"].as_f64().filter(|x|x.is_finite());let hi=r["value_upper"].as_f64().filter(|x|x.is_finite());
            if lo.zip(hi).is_none_or(|(a,b)|a>b){return Err(err("RESPONSE_INTERVAL",&p,"VALID requires finite ordered interval"));}
        }else if !r["value_lower"].is_null()||!r["value_upper"].is_null(){return Err(err("UNAVAILABLE_RESPONSE_HAS_VALUES",&p,"non-valid response must not carry a numeric interval"));}
        let sensor=sensors.iter().find(|s|s["channel_id"]==ch).unwrap();
        if !installed_channels.contains(ch){return Err(err("UNINSTALLED_SENSOR_HAS_RESPONSE",&p,"uninstalled sensor must not contribute response records"));}
        if txt(sensor,"unit","$.sensors[]")?!=txt(r,"unit",&p)?{return Err(err("RESPONSE_UNIT_MISMATCH",&p,"response unit must match sensor"));}
    }
    for state in states{for s in sensors.iter().filter(|s|s["installed"]==true){
        let key=(txt(state,"state_id","$.states[]")?.to_owned(),txt(s,"channel_id","$.sensors[]")?.to_owned());
        if !seen.contains(&key){return Err(err("INCOMPLETE_RESPONSE_MATRIX","$.responses",format!("missing response for {} / {}",key.0,key.1)));}
    }}
    let actual_pairs:BTreeSet<String>=pairs.iter().map(|p|{
        pair_key(p["fault_a"].as_str().unwrap(),p["fault_b"].as_str().unwrap())
    }).collect();
    if actual_pairs!=expected_pairs{
        return Err(err("INCOMPLETE_REQUIRED_PAIR_SET","$.required_fault_pairs","declared pairs must cover all in-domain state pairs plus reference-vs-OOD pairs exactly"));
    }
    let scenarios=array(input,"common_cause_scenarios","$")?;let mut scenario_ids=BTreeSet::new();
    let mut covered_dependency_groups=BTreeSet::new();
    for (i,s) in scenarios.iter().enumerate(){
        let p=format!("$.common_cause_scenarios[{i}]");
        allowed_keys(s,&["scenario_id","removed_sensor_ids","removed_dependency_groups"],&p)?;
        let id=txt(s,"scenario_id",&p)?;
        if !scenario_ids.insert(id.to_owned()){return Err(err("DUPLICATE_SCENARIO_ID",&p,"scenario IDs must be unique"));}
        let removed_s=set(s,"removed_sensor_ids",&p)?;let removed_d=set(s,"removed_dependency_groups",&p)?;
        if removed_s.is_empty()&&removed_d.is_empty(){return Err(err("EMPTY_SENSOR_LOSS_SCENARIO",&p,"remove at least one sensor/dependency group"));}
        if removed_s.iter().any(|x|!sensor_ids.contains(x)){return Err(err("UNKNOWN_REMOVED_SENSOR",&p,"unknown removed sensor"));}
        if removed_d.iter().any(|x|!dependencies.contains(x)){return Err(err("UNKNOWN_REMOVED_DEPENDENCY",&p,"unknown dependency group"));}
        covered_dependency_groups.extend(removed_d);
    }
    let mut dependency_member_counts:BTreeMap<String,usize>=BTreeMap::new();
    for sensor in sensors.iter().filter(|s|s["installed"]==true){
        for dependency in set(sensor,"dependency_groups","$.sensors[]")?{
            *dependency_member_counts.entry(dependency).or_default()+=1;
        }
    }
    let missing_shared:Vec<String>=dependency_member_counts.iter()
        .filter(|(group,count)|**count>1&&!covered_dependency_groups.contains(*group))
        .map(|(group,_)|group.clone()).collect();
    if !missing_shared.is_empty(){
        return Err(err("SHARED_DEPENDENCY_OUTAGE_NOT_COVERED","$.common_cause_scenarios",
            format!("every shared installed dependency requires at least one outage scenario: {}",missing_shared.join(", "))));
    }
    validate_trajectory_signature_cases(input)?;
    Ok(())
}

/// A response window is usable only if every admitted channel has a
/// guaranteed common acquisition interval after its own clock uncertainty is
/// applied. Matching a window identifier alone is not temporal evidence.
fn measurement_window_assessments(input:&Value,removed_sensors:&BTreeSet<String>,removed_dependencies:&BTreeSet<String>)->(Vec<Value>,BTreeSet<String>){
    let evidence_class=input["evidence_class"].as_str().unwrap_or("");
    let domain=&input["operating_domain"];
    let profile=&input["layout_profile"];
    let minimum=profile["minimum_common_window_overlap_ms"].as_f64().unwrap_or(f64::INFINITY);
    let mut reports=Vec::new();
    let mut unqualified_states=BTreeSet::new();
    for state in input["states"].as_array().unwrap(){
        let state_id=state["state_id"].as_str().unwrap_or("");
        let mut starts=Vec::new();let mut ends=Vec::new();let mut eligible_channels=Vec::new();
        for response in input["responses"].as_array().unwrap().iter()
            .filter(|r|r["state_id"].as_str()==Some(state_id)&&r["availability"]=="VALID"){
            let channel_id=response["channel_id"].as_str().unwrap_or("");
            let Some(sensor)=input["sensors"].as_array().unwrap().iter().find(|s|s["channel_id"].as_str()==Some(channel_id)) else{continue};
            if sensor["installed"]!=true{continue;}
            let sensor_id=sensor["sensor_id"].as_str().unwrap_or("");
            let dep=sensor["dependency_groups"].as_array().cloned().unwrap_or_default();
            if removed_sensors.contains(sensor_id)||dep.iter().filter_map(Value::as_str).any(|g|removed_dependencies.contains(g)){continue;}
            let dependency_reviewed=match evidence_class{
                "SYNTHETIC_CONFORMANCE_FIXTURE"=>sensor["dependency_review_state"]=="synthetic_conformance",
                "MEASURED"=>sensor["dependency_review_state"]=="independently_reviewed"
                    &&sensor["dependency_review_record_id"].as_str().is_some_and(|x|!x.trim().is_empty())
                    &&sensor["dependency_review_record_sha256"].as_str().is_some_and(hex64),
                _=>false
            };
            let derived=sensor["derived_from_channel_ids"].as_array().is_some_and(|a|!a.is_empty());
            let role_ok=match evidence_class{
                "SYNTHETIC_CONFORMANCE_FIXTURE"=>sensor["source_role"]=="synthetic"&&response["evidence_role"]=="SYNTHETIC_CONFORMANCE_FIXTURE"&&!derived,
                "MEASURED"=>sensor["source_role"]=="measured"&&response["evidence_role"]=="INDEPENDENT_MEASUREMENT"&&!derived,
                _=>false
            };
            let eligible=sensor["quality_state"]=="current"
                &&matches!(sensor["calibration_state"].as_str(),Some("current"|"not_applicable"))
                &&domain["qualification_state"]=="qualified"
                &&sensor_timebase_qualified(sensor,domain,profile,evidence_class)
                &&dependency_reviewed&&role_ok;
            if !eligible{continue;}
            let uncertainty=sensor["timestamp_uncertainty_ms"].as_f64().unwrap_or(f64::INFINITY);
            let start=response["window_start_ms"].as_f64().unwrap_or(f64::NAN);
            let end=response["window_end_ms"].as_f64().unwrap_or(f64::NAN);
            let guaranteed_start=start+uncertainty;
            let guaranteed_end=end-uncertainty;
            if !guaranteed_start.is_finite()||!guaranteed_end.is_finite()||guaranteed_end<=guaranteed_start{continue;}
            starts.push(guaranteed_start);ends.push(guaranteed_end);eligible_channels.push(channel_id.to_owned());
        }
        let (common_start,common_end,overlap)=if starts.is_empty(){(None,None,0.0)}else{
            let start=starts.iter().copied().fold(f64::NEG_INFINITY,f64::max);
            let end=ends.iter().copied().fold(f64::INFINITY,f64::min);
            (Some(start),Some(end),(end-start).max(0.0))
        };
        let qualified=!starts.is_empty()&&overlap+1e-9>=minimum;
        if !qualified{unqualified_states.insert(state_id.to_owned());}
        let window_ref=input["responses"].as_array().unwrap().iter().find(|r|r["state_id"].as_str()==Some(state_id))
            .and_then(|r|r["measurement_window_ref"].as_str()).unwrap_or("");
        reports.push(json!({
            "state_id":state_id,"measurement_window_ref":window_ref,
            "status":if qualified{"qualified"}else{"not_evaluated"},
            "eligible_channel_ids":eligible_channels,"required_common_overlap_ms":minimum,
            "guaranteed_common_start_ms":if qualified{json!(common_start)}else{Value::Null},
            "guaranteed_common_end_ms":if qualified{json!(common_end)}else{Value::Null},
            "guaranteed_common_overlap_ms":overlap
        }));
    }
    (reports,unqualified_states)
}

fn make_parent_and_case(input:&Value,removed_sensors:&BTreeSet<String>,removed_dependencies:&BTreeSet<String>,unaligned_states:&BTreeSet<String>)->Check<(Value,Value)>{
    let asset=&input["asset"];let regime=&input["operating_domain"];
    let family=format!("engine-family:{}",txt(asset,"engine_model","$.asset")?);
    let config=format!("asset-config-sha256:{}",txt(asset,"configuration_sha256","$.asset")?);
    let domain=format!("operating-domain:{}",txt(regime,"regime_id","$.operating_domain")?);
    // The v0.1 input deliberately leaves evidence_digests null until external
    // artifact bytes are available. Use a declared profile identity here; do
    // not fabricate a sensor-topology SHA-256 from a name or null value.
    let installation=format!("installation-profile:{}:{}", 
        txt(&input["layout_profile"],"layout_profile_ref","$.layout_profile")?,
        txt(asset,"configuration_revision","$.asset")?);
    let parent_id=format!("asset:{}",txt(asset,"asset_id","$.asset")?);
    let interval=format!("regime:{}",txt(regime,"regime_id","$.operating_domain")?);
    let trial=format!("configuration:{}",txt(asset,"configuration_revision","$.asset")?);
    let identity=json!({"asset_family_ref":family,"asset_configuration_ref":config,"operating_domain_ref":domain,"installation_profile_ref":installation});
    let parent=json!({"case_id":parent_id,"intervals":[{"interval_id":interval,"trial_id":trial}],"profile":identity});
    let parent_context=json!({"parent_case_id":parent_id,"parent_interval_id":interval,"trial_id":trial,
        "asset_family_ref":family,"asset_configuration_ref":config,"operating_domain_ref":domain,"installation_profile_ref":installation});
    let sensors=input["sensors"].as_array().unwrap();
    let installed:Vec<&Value>=sensors.iter().filter(|s|s["installed"]==true).collect();
    let time_domain=&input["operating_domain"];
    let layout_profile=&input["layout_profile"];
    let domain_qualified=txt(time_domain,"qualification_state","$.operating_domain")?=="qualified";
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
        let dependency_reviewed=match (evidence_class,txt(sensor,"dependency_review_state","$.sensors[]")?) {
            ("SYNTHETIC_CONFORMANCE_FIXTURE","synthetic_conformance")=>true,
            ("MEASURED","independently_reviewed")=>sensor["dependency_review_record_id"].as_str().is_some_and(|x|!x.trim().is_empty())
                &&sensor["dependency_review_record_sha256"].as_str().is_some_and(hex64),
            _=>false
        };
        let ok=txt(sensor,"quality_state","$.sensors[]")?=="current"
            && matches!(txt(sensor,"calibration_state","$.sensors[]")?,"current"|"not_applicable")
            && domain_qualified
            && sensor_timebase_qualified(sensor,time_domain,layout_profile,evidence_class)
            && dependency_reviewed;
        let deps=set(sensor,"dependency_groups","$.sensors[]")?;
        let scenario_removed=removed_sensors.contains(sid)||deps.iter().any(|g|removed_dependencies.contains(g));
        let raw_availability=txt(r,"availability","$.responses[]")?;
        let state=input["states"].as_array().unwrap().iter().find(|s|s["state_id"]==r["state_id"]).unwrap();
        let faulted= set(state,"faulted_sensor_ids","$.states[]")?;
        let declared_sensor_fault=faulted.contains(sid);
        let availability=if scenario_removed{"MISSING"}
            else if declared_sensor_fault&&matches!(raw_availability,"MISSING"|"DEGRADED"){raw_availability}
            else if unaligned_states.contains(r["state_id"].as_str().unwrap_or(""))||!ok{"UNKNOWN"}
            else{raw_availability};
        let role=txt(sensor,"source_role","$.sensors[]")?;
        let derived=set(sensor,"derived_from_channel_ids","$.sensors[]")?;
        let response_role=txt(r,"evidence_role","$.responses[]")?;
        let admissible_role=ok && ((evidence_class=="SYNTHETIC_CONFORMANCE_FIXTURE"
                &&role=="synthetic"&&response_role=="SYNTHETIC_CONFORMANCE_FIXTURE"&&derived.is_empty())
            ||(evidence_class=="MEASURED"&&role=="measured"&&response_role=="INDEPENDENT_MEASUREMENT"&&derived.is_empty()));
        let availability=if !admissible_role&&availability=="VALID"{"UNKNOWN"}else{availability};
        let (lo,hi)=if availability=="VALID"{(r["value_lower"].clone(),r["value_upper"].clone())}else{(Value::Null,Value::Null)};
        let evidence_role=if admissible_role{
            if evidence_class=="SYNTHETIC_CONFORMANCE_FIXTURE"{"SYNTHETIC_CONFORMANCE_FIXTURE"}else{"INDEPENDENT_MEASUREMENT"}
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
    let channel=response["channel_id"].as_str().unwrap_or("");
    let sensor=input["sensors"].as_array().unwrap().iter().find(|s|s["channel_id"]==channel);
    match response["evidence_role"].as_str(){
        Some("MODEL_ESTIMATE")=>"model_estimate",
        Some("OPERATOR_REPORTED")=>"operator_reported",
        _=>match sensor.and_then(|s|s["source_role"].as_str()){
            Some("derived")=>"derived",
            Some("reconstructed")=>"reconstructed",
            Some("simulated")=>"simulated",
            Some("synthetic")=>"synthetic",
            Some("measured") if response["evidence_role"]=="INDEPENDENT_MEASUREMENT"=>"measured",
            _=>"expert_authored"
        }
    }
}
fn pair_assessments(input:&Value,report:&EvaluationReport)->Vec<Value>{
    let declarations=input["required_fault_pairs"].as_array().unwrap();
    let responses=input["responses"].as_array().unwrap();
    report.pair_results.iter().map(|result|{
        let declaration=declarations.iter().find(|p|p["pair_id"]==result.pair_id).unwrap();
        let mut grouped:BTreeMap<(String,String,String),BTreeSet<String>>=BTreeMap::new();
        for response in responses.iter().filter(|r|
            (r["state_id"]==result.state_a_id||r["state_id"]==result.state_b_id)
            &&result.discriminating_channel_ids.iter().any(|c|r["channel_id"].as_str()==Some(c.as_str()))
        ){
            let artifact=response["artifact_id"].as_str().unwrap_or("").to_owned();
            let sha=response["source_sha256"].as_str().unwrap_or("").to_owned();
            let role=evidence_role_name(input,response).to_owned();
            let channel=response["channel_id"].as_str().unwrap_or("").to_owned();
            grouped.entry((artifact,sha,role)).or_default().insert(channel);
        }
        let refs:Vec<Value>=grouped.into_iter().map(|((artifact,sha,role),channels)|json!({
            "artifact_id":artifact,"sha256":sha,"evidence_role":role,"channel_ids":channels.into_iter().collect::<Vec<_>>()
        })).collect();
        json!({"fault_a":declaration["fault_a"],"fault_b":declaration["fault_b"],"state":pair_state(&result.status),"method":"structural","evidence_refs":refs})
    }).collect()
}

#[derive(Clone,Debug)]
struct FailureAtom{
    kind:String,
    identity:String,
}
fn append_failure_combinations(
    atom_count:usize,
    order:usize,
    start:usize,
    selected:&mut Vec<usize>,
    out:&mut Vec<Vec<usize>>,
){
    if selected.len()==order{
        out.push(selected.clone());
        return;
    }
    for index in start..atom_count{
        selected.push(index);
        append_failure_combinations(atom_count,order,index+1,selected,out);
        selected.pop();
    }
}

/// Exact bounded resilience analysis over individual installed-sensor outages
/// and shared dependency-group outages. Singleton dependency groups are
/// represented by their sensor atom to avoid duplicate names for the same cut.
fn resilience_analysis(input:&Value,base:&EvaluationReport,base_windows:&[Value],base_trajectory_reports:&[Value])->Check<Value>{
    let profile=&input["layout_profile"];
    let (declared_atom_limit,max_order)=failure_cut_bounds(profile)?;
    let mut atoms=Vec::new();
    let sensors=input["sensors"].as_array().expect("validated sensors");
    let mut dependency_counts:BTreeMap<String,usize>=BTreeMap::new();
    for sensor in sensors.iter().filter(|s|s["installed"]==true){
        atoms.push(FailureAtom{
            kind:"sensor".to_owned(),
            identity:txt(sensor,"sensor_id","$.sensors[]")?.to_owned(),
        });
        for group in set(sensor,"dependency_groups","$.sensors[]")?{
            *dependency_counts.entry(group).or_default()+=1;
        }
    }
    for (group,count) in dependency_counts{
        if count>=2{
            atoms.push(FailureAtom{kind:"dependency_group".to_owned(),identity:group});
        }
    }
    atoms.sort_by(|a,b|a.kind.cmp(&b.kind).then_with(||a.identity.cmp(&b.identity)));
    if atoms.len()>declared_atom_limit{
        return Err(err("EXACT_FAILURE_ATOM_LIMIT_EXCEEDED",
            "$.layout_profile.maximum_failure_atoms_for_exact_cut_analysis",
            format!("{} failure atoms exceed declared exact-search bound {declared_atom_limit}; no heuristic fallback is permitted",atoms.len())));
    }
    if atoms.is_empty()||max_order>atoms.len(){
        return Err(err("EMPTY_OR_INVALID_FAILURE_ATOM_CATALOG","$.sensors",
            "exact resilience analysis requires a non-empty bounded failure-atom catalog"));
    }
    let catalog:Vec<Value>=atoms.iter().map(|a|json!({"kind":&a.kind,"identity":&a.identity})).collect();
    let mut combinations=Vec::new();
    for order in 1..=max_order{
        append_failure_combinations(atoms.len(),order,0,&mut Vec::new(),&mut combinations);
    }
    let scenario_count=combinations.len();
    let critical_pairs:Vec<_>=base.pair_results.iter().filter(|p|p.critical).collect();
    let mut minimal_by_pair:BTreeMap<String,Vec<Vec<usize>>>=BTreeMap::new();
    for pair in critical_pairs.iter().filter(|p|p.status=="IDENTIFIABLE"){
        minimal_by_pair.insert(pair_key(&pair.state_a_id,&pair.state_b_id),Vec::new());
    }
    let mut minimal_by_trajectory:BTreeMap<String,Vec<Vec<usize>>>=BTreeMap::new();
    for trajectory in base_trajectory_reports.iter().filter(|p|p["trajectory_state"]=="isolable"){
        let id=txt(trajectory,"signature_case_id","$.trajectory_pair_assessments[]")?.to_owned();
        minimal_by_trajectory.insert(id,Vec::new());
    }

    // Enumerate by increasing cardinality: a cut is retained only when none
    // of its already-recorded strict subsets is itself a cut.
    for combination in combinations{
        let mut removed_sensors=BTreeSet::new();
        let mut removed_dependencies=BTreeSet::new();
        for index in &combination{
            let atom=&atoms[*index];
            match atom.kind.as_str(){
                "sensor"=>{removed_sensors.insert(atom.identity.clone());}
                "dependency_group"=>{removed_dependencies.insert(atom.identity.clone());}
                _=>return Err(err("UNKNOWN_FAILURE_ATOM_KIND","$.layout_profile","invalid internal failure-atom kind")),
            }
        }
        let (scenario_windows,unaligned)=measurement_window_assessments(input,&removed_sensors,&removed_dependencies);
        let (adapted,parent)=make_parent_and_case(input,&removed_sensors,&removed_dependencies,&unaligned)?;
        let replay=evaluate_case(&adapted,&parent)?;
        let scenario_trajectories=trajectory_pair_assessments(
            input,&replay,&scenario_windows,&removed_sensors,&removed_dependencies
        )?;
        for pair in critical_pairs.iter().filter(|p|p.status=="IDENTIFIABLE"){
            if pair_status(&replay,&pair.state_a_id,&pair.state_b_id)!=Some("IDENTIFIABLE"){
                let cuts=minimal_by_pair.get_mut(&pair_key(&pair.state_a_id,&pair.state_b_id)).expect("baseline-qualified critical pair");
                let has_smaller_subset=cuts.iter().any(|existing|existing.iter().all(|i|combination.contains(i)));
                if !has_smaller_subset{cuts.push(combination.clone());}
            }
        }
        for baseline_trajectory in base_trajectory_reports.iter().filter(|p|p["trajectory_state"]=="isolable"){
            let trajectory_id=baseline_trajectory["signature_case_id"].as_str().expect("trajectory case ID");
            let replayed=scenario_trajectories.iter().find(|p|p["signature_case_id"]==trajectory_id)
                .ok_or_else(||err("MISSING_REPLAYED_TRAJECTORY","$.trajectory_pair_assessments",trajectory_id))?;
            if replayed["trajectory_state"]!="isolable"{
                let cuts=minimal_by_trajectory.get_mut(trajectory_id).expect("baseline-qualified temporal pair");
                let has_smaller_subset=cuts.iter().any(|existing|existing.iter().all(|i|combination.contains(i)));
                if !has_smaller_subset{cuts.push(combination.clone());}
            }
        }
    }

    let mut pair_results=Vec::new();
    let mut baseline_qualified=0usize;
    let mut cut_found=0usize;
    let mut no_cut=0usize;
    let mut baseline_not_qualified=0usize;
    for pair in critical_pairs{
        let baseline_state=match pair.status.as_str(){
            "IDENTIFIABLE"=>"isolable",
            "NOT_IDENTIFIABLE"=>"non_isolable",
            _=>"not_evaluated",
        };
        let key=pair_key(&pair.state_a_id,&pair.state_b_id);
        let cuts=minimal_by_pair.get(&key).cloned().unwrap_or_default();
        let minimum_cut_order=cuts.iter().map(Vec::len).min();
        let status;
        let maximum_tolerated;
        let tolerated_lower_bound;
        if pair.status!="IDENTIFIABLE"{
            status="baseline_not_qualified";
            maximum_tolerated=Value::Null;
            tolerated_lower_bound=Value::Null;
            baseline_not_qualified+=1;
        }else if let Some(order)=minimum_cut_order{
            status="cut_found_within_bound";
            maximum_tolerated=json!(order-1);
            tolerated_lower_bound=Value::Null;
            baseline_qualified+=1;
            cut_found+=1;
        }else{
            status="no_cut_found_within_bound";
            maximum_tolerated=Value::Null;
            tolerated_lower_bound=json!(max_order);
            baseline_qualified+=1;
            no_cut+=1;
        }
        let minimal_cut_sets:Vec<Value>=cuts.iter().map(|cut|{
            let failure_atoms:Vec<Value>=cut.iter().map(|i|json!({
                "kind":&atoms[*i].kind,"identity":&atoms[*i].identity
            })).collect();
            json!({"order":cut.len(),"failure_atoms":failure_atoms})
        }).collect();
        pair_results.push(json!({
            "fault_a":&pair.state_a_id,
            "fault_b":&pair.state_b_id,
            "baseline_state":baseline_state,
            "status":status,
            "minimum_cut_order":minimum_cut_order,
            "maximum_tolerated_failure_atoms":maximum_tolerated,
            "tolerated_failure_atoms_lower_bound":tolerated_lower_bound,
            "minimal_cut_sets":minimal_cut_sets
        }));
    }
    let mut trajectory_pair_results=Vec::new();
    let mut trajectory_baseline_qualified=0usize;
    let mut trajectory_cut_found=0usize;
    let mut trajectory_no_cut=0usize;
    let mut trajectory_baseline_not_qualified=0usize;
    for trajectory in base_trajectory_reports{
        let id=txt(trajectory,"signature_case_id","$.trajectory_pair_assessments[]")?;
        let cuts=minimal_by_trajectory.get(id).cloned().unwrap_or_default();
        let minimum_cut_order=cuts.iter().map(Vec::len).min();
        let (status,maximum_tolerated,tolerated_lower_bound)=if trajectory["trajectory_state"]!="isolable"{
            trajectory_baseline_not_qualified+=1;
            ("baseline_not_qualified",Value::Null,Value::Null)
        }else if let Some(order)=minimum_cut_order{
            trajectory_baseline_qualified+=1;
            trajectory_cut_found+=1;
            ("cut_found_within_bound",json!(order-1),Value::Null)
        }else{
            trajectory_baseline_qualified+=1;
            trajectory_no_cut+=1;
            ("no_cut_found_within_bound",Value::Null,json!(max_order))
        };
        let minimal_cut_sets:Vec<Value>=cuts.iter().map(|cut|{
            let failure_atoms:Vec<Value>=cut.iter().map(|i|json!({
                "kind":&atoms[*i].kind,"identity":&atoms[*i].identity
            })).collect();
            json!({"order":cut.len(),"failure_atoms":failure_atoms})
        }).collect();
        trajectory_pair_results.push(json!({
            "signature_case_id":id,
            "pair_id":trajectory["pair_id"],
            "fault_a":trajectory["fault_a"],
            "fault_b":trajectory["fault_b"],
            "baseline_static_state":trajectory["static_pair_state"],
            "baseline_trajectory_state":trajectory["trajectory_state"],
            "status":status,
            "minimum_cut_order":minimum_cut_order,
            "maximum_tolerated_failure_atoms":maximum_tolerated,
            "tolerated_failure_atoms_lower_bound":tolerated_lower_bound,
            "minimal_cut_sets":minimal_cut_sets
        }));
    }
    Ok(json!({
        "analysis_method":"EXACT_SENSOR_AND_SHARED_DEPENDENCY_CUT_ENUMERATION_V0_1",
        "failure_atom_count":atoms.len(),
        "failure_atom_catalog":catalog,
        "maximum_failure_cut_order":max_order,
        "failure_scenario_count":scenario_count,
        "critical_pair_count":critical_pairs_count(base),
        "baseline_qualified_critical_pair_count":baseline_qualified,
        "cut_found_critical_pair_count":cut_found,
        "no_cut_found_within_bound_critical_pair_count":no_cut,
        "baseline_not_qualified_critical_pair_count":baseline_not_qualified,
        "pair_results":pair_results,
        "trajectory_pair_count":base_trajectory_reports.len(),
        "trajectory_baseline_qualified_count":trajectory_baseline_qualified,
        "trajectory_cut_found_count":trajectory_cut_found,
        "trajectory_no_cut_found_within_bound_count":trajectory_no_cut,
        "trajectory_baseline_not_qualified_count":trajectory_baseline_not_qualified,
        "trajectory_pair_results":trajectory_pair_results,
        "claim_ceiling":"Exact enumeration of declared single-sensor and shared-dependency failure combinations up to the stated cut order. Failure atoms are modeled outage hypotheses, not probabilistically independent events. No reliability probability, physical failure rate, hardware independence or resilience outside the enumerated configuration and cardinality bound is inferred."
    }))
}
fn critical_pairs_count(report:&EvaluationReport)->usize{
    report.pair_results.iter().filter(|p|p.critical).count()
}

fn pair_status<'a>(report:&'a EvaluationReport,a:&str,b:&str)->Option<&'a str>{
    report.pair_results.iter().find(|p|
        (p.state_a_id==a&&p.state_b_id==b)||(p.state_a_id==b&&p.state_b_id==a)
    ).map(|p|p.status.as_str())
}
fn signature_evidence(input:&Value,fault_id:&str)->Vec<Value>{
    let mut grouped:BTreeMap<(String,String,String),BTreeSet<String>>=BTreeMap::new();
    for response in input["responses"].as_array().unwrap().iter().filter(|r|r["state_id"]==fault_id){
        let artifact=response["artifact_id"].as_str().unwrap_or("").to_owned();
        let sha=response["source_sha256"].as_str().unwrap_or("").to_owned();
        let role=evidence_role_name(input,response).to_owned();
        let channel=response["channel_id"].as_str().unwrap_or("").to_owned();
        grouped.entry((artifact,sha,role)).or_default().insert(channel);
    }
    grouped.into_iter().map(|((artifact,sha,role),channels)|json!({
        "artifact_id":artifact,"sha256":sha,"evidence_role":role,"channel_ids":channels.into_iter().collect::<Vec<_>>()
    })).collect()
}

fn maximal_ambiguity_cliques(adjacency:&BTreeMap<String,BTreeSet<String>>)->Vec<Vec<String>>{
    fn visit(current:&mut BTreeSet<String>,mut possible:BTreeSet<String>,mut excluded:BTreeSet<String>,
        adjacency:&BTreeMap<String,BTreeSet<String>>,output:&mut Vec<Vec<String>>){
        if possible.is_empty()&&excluded.is_empty(){if current.len()>1{output.push(current.iter().cloned().collect());}return;}
        let pivot=possible.union(&excluded).cloned().max_by_key(|node|adjacency.get(node).map(|n|possible.intersection(n).count()).unwrap_or(0));
        let empty=BTreeSet::new();
        let neighbors=pivot.as_ref().and_then(|node|adjacency.get(node)).unwrap_or(&empty);
        let candidates:Vec<String>=possible.difference(neighbors).cloned().collect();
        for node in candidates{
            let n=adjacency.get(&node).cloned().unwrap_or_default();
            current.insert(node.clone());
            visit(current,possible.intersection(&n).cloned().collect(),excluded.intersection(&n).cloned().collect(),adjacency,output);
            current.remove(&node);possible.remove(&node);excluded.insert(node);
        }
    }
    let mut out=Vec::new();
    visit(&mut BTreeSet::new(),adjacency.keys().cloned().collect(),BTreeSet::new(),adjacency,&mut out);
    out.sort();out.dedup();out
}
fn fault_assessments(input:&Value,report:&EvaluationReport)->(Vec<Value>,Vec<Value>){
    let reference=input["layout_profile"]["reference_state_id"].as_str().unwrap();
    let states=input["states"].as_array().unwrap();
    let in_domain:Vec<&str>=states.iter().filter(|s|s["state_class"]!="OUT_OF_DOMAIN"&&s["state_id"]!=reference)
        .map(|s|s["state_id"].as_str().unwrap()).collect();
    let detectable:BTreeSet<String>=in_domain.iter().copied()
        .filter(|fault|pair_status(report,reference,fault)==Some("IDENTIFIABLE"))
        .map(str::to_owned).collect();
    let mut adjacency:BTreeMap<String,BTreeSet<String>>=BTreeMap::new();
    for pair in &report.pair_results{
        if pair.status=="NOT_IDENTIFIABLE"&&detectable.contains(&pair.state_a_id)&&detectable.contains(&pair.state_b_id){
            adjacency.entry(pair.state_a_id.clone()).or_default().insert(pair.state_b_id.clone());
            adjacency.entry(pair.state_b_id.clone()).or_default().insert(pair.state_a_id.clone());
        }
    }
    // Pairwise ambiguity is not transitive: maximal cliques retain overlapping
    // ambiguity sets without merging a non-transitive chain.
    let cliques=maximal_ambiguity_cliques(&adjacency);
    let mut memberships:BTreeMap<String,Vec<String>>=BTreeMap::new();
    let mut candidates:BTreeMap<String,BTreeSet<String>>=BTreeMap::new();
    let mut classes_out=Vec::new();
    for (index,clique) in cliques.iter().enumerate(){
        let class_id=format!("AMBIGUITY_CLASS_{:04}",index+1);
        let mut pairs=Vec::new();
        for i in 0..clique.len(){for j in (i+1)..clique.len(){
            pairs.push(json!({"fault_a":clique[i],"fault_b":clique[j],"state":"non_isolable"}));
        }}
        classes_out.push(json!({"ambiguity_class_id":class_id,"candidate_fault_ids":clique,"pairwise_non_isolable_pairs":pairs}));
        for fault in clique{
            memberships.entry(fault.clone()).or_default().push(class_id.clone());
            candidates.entry(fault.clone()).or_default().extend(clique.iter().cloned());
        }
    }
    for ids in memberships.values_mut(){ids.sort();}
    let fault_out=states.iter().filter(|state|state["state_id"]!=reference).map(|state|{
        let id=state["state_id"].as_str().unwrap();
        let class=state["state_class"].as_str().unwrap();
        let classes=memberships.get(id).cloned().unwrap_or_default();
        let baseline=pair_status(report,reference,id);
        let qualification=input["operating_domain"]["qualification_state"].as_str().unwrap_or("not_evaluated");
        let disposition=if class=="OUT_OF_DOMAIN"||qualification=="unsupported"{"unsupported_regime"}
            else if qualification!="qualified"{"not_evaluated"}
            else if !classes.is_empty(){"ambiguity_class"}
            else{match baseline{
                None|Some("INSUFFICIENT_EVIDENCE")=>"not_evaluated",
                Some("NOT_IDENTIFIABLE")=>"unobservable",
                Some("IDENTIFIABLE")=>{
                    let mut unresolved=false;let mut indistinguishable=false;
                    for other in in_domain.iter().copied().filter(|x|*x!=id){
                        match pair_status(report,id,other){
                            Some("NOT_IDENTIFIABLE")=>indistinguishable=true,
                            Some("INSUFFICIENT_EVIDENCE")|None=>unresolved=true,
                            _=>{}
                        }
                    }
                    if indistinguishable{"detectable_not_isolable"}else if unresolved{"not_evaluated"}else{"observable_and_isolable"}
                },_=>"not_evaluated"
            }};
        let one=if classes.len()==1{json!(classes[0])}else{Value::Null};
        let candidates=candidates.get(id).map(|x|x.iter().cloned().collect::<Vec<_>>()).unwrap_or_default();
        json!({"fault_id":id,"state":disposition,"ambiguity_class_id":one,"ambiguity_class_ids":classes,
            "candidate_fault_ids":candidates,"signature_evidence":signature_evidence(input,id),"review_state":"human_review_required"})
    }).collect();
    (fault_out,classes_out)
}


fn longest_separated_trajectory_span(samples_a:&[Value],samples_b:&[Value],sample_period_ms:u64)->f64{
    let mut run_start:Option<u64>=None;
    let mut run_end:Option<u64>=None;
    let mut longest=0.0f64;
    for (a,b) in samples_a.iter().zip(samples_b.iter()){
        let offset=a["offset_ms"].as_u64().unwrap();
        let separated=a["value_upper"].as_f64().unwrap()<b["value_lower"].as_f64().unwrap()
            ||b["value_upper"].as_f64().unwrap()<a["value_lower"].as_f64().unwrap();
        if separated{
            if run_end.is_some_and(|previous|offset==previous.saturating_add(sample_period_ms)){
                run_end=Some(offset);
            }else{
                run_start=Some(offset);
                run_end=Some(offset);
            }
            longest=longest.max(offset.saturating_sub(run_start.unwrap_or(offset)) as f64);
        }else{
            run_start=None;
            run_end=None;
        }
    }
    longest
}
fn trajectory_pair_assessments(
    input:&Value,
    base:&EvaluationReport,
    base_windows:&[Value],
    removed_sensors:&BTreeSet<String>,
    removed_dependencies:&BTreeSet<String>,
)->Check<Vec<Value>>{
    let evidence_class=match txt(input,"evidence_class","$")?{
        "SYNTHETIC_CONFORMANCE_FIXTURE"=>PairEvidenceClass::SyntheticConformanceFixture,
        "MEASURED"=>PairEvidenceClass::Measured,
        _=>return Err(err("TRAJECTORY_EVIDENCE_CLASS","$.evidence_class","unsupported trajectory evidence class")),
    };
    let evidence_class_name=match evidence_class{
        PairEvidenceClass::SyntheticConformanceFixture=>"SYNTHETIC_CONFORMANCE_FIXTURE",
        PairEvidenceClass::Measured=>"MEASURED",
    };
    let domain=&input["operating_domain"];
    let profile=&input["layout_profile"];
    let sensors=input["sensors"].as_array().expect("validated sensors");
    let responses=input["responses"].as_array().expect("validated responses");
    let maximum_uncertainty=profile["maximum_timestamp_uncertainty_ms"].as_f64().unwrap_or(0.0);
    let mut reports=Vec::new();
    for trajectory in input["trajectory_signature_cases"].as_array().expect("validated trajectories"){
        let id=txt(trajectory,"signature_case_id","$.trajectory_signature_cases[]")?;
        let pair_id=txt(trajectory,"pair_id","$.trajectory_signature_cases[]")?;
        let fault_a=txt(trajectory,"fault_a","$.trajectory_signature_cases[]")?;
        let fault_b=txt(trajectory,"fault_b","$.trajectory_signature_cases[]")?;
        let static_pair_state=match pair_status(base,fault_a,fault_b){
            Some("IDENTIFIABLE")=>"isolable",
            Some("NOT_IDENTIFIABLE")=>"non_isolable",
            _=>"not_evaluated",
        };
        let alignment_state=txt(trajectory,"event_alignment_state","$.trajectory_signature_cases[]")?;
        let alignment_evidence_ok=match (evidence_class,alignment_state){
            (PairEvidenceClass::SyntheticConformanceFixture,"synthetic_conformance")=>trajectory["event_alignment_record_id"].is_null()&&trajectory["event_alignment_record_sha256"].is_null(),
            (PairEvidenceClass::Measured,"independently_reviewed")=>trajectory["event_alignment_record_id"].as_str().is_some_and(|s|!s.trim().is_empty())
                &&trajectory["event_alignment_record_sha256"].as_str().is_some_and(hex64),
            _=>false,
        };
        let timebase_id=txt(trajectory,"timebase_id","$.trajectory_signature_cases[]")?;
        let timebase_matches=timebase_id==domain["measurement_timebase_id"].as_str().unwrap_or("");
        let alignment_uncertainty=trajectory["alignment_uncertainty_ms"].as_f64().unwrap_or(f64::INFINITY);
        let period=trajectory["sample_period_ms"].as_f64().unwrap_or(0.0);
        let minimum_duration=trajectory["minimum_separated_duration_ms"].as_f64().unwrap_or(f64::INFINITY);
        let mut signatures=Vec::new();
        let mut channel_reports=Vec::new();
        for channel in trajectory["trajectory_channels"].as_array().expect("validated trajectory channels"){
            let sensor_id=txt(channel,"sensor_id","$.trajectory_signature_cases[].trajectory_channels[]")?;
            let channel_id=txt(channel,"channel_id","$.trajectory_signature_cases[].trajectory_channels[]")?;
            let sensor=sensors.iter().find(|s|s["sensor_id"]==sensor_id&&s["channel_id"]==channel_id).expect("validated sensor binding");
            let sensor_uncertainty=sensor["timestamp_uncertainty_ms"].as_f64().unwrap_or(f64::INFINITY);
            let combined_uncertainty=alignment_uncertainty+sensor_uncertainty;
            let dependencies=set(sensor,"dependency_groups","$.sensors[]")?;
            let sensor_or_dependency_removed=removed_sensors.contains(sensor_id)
                ||dependencies.iter().any(|dependency|removed_dependencies.contains(dependency));
            let sensor_timebase_ok=sensor_timebase_qualified(sensor,domain,profile,evidence_class_name);
            let dependency_reviewed=match evidence_class{
                PairEvidenceClass::SyntheticConformanceFixture=>sensor["dependency_review_state"]=="synthetic_conformance",
                PairEvidenceClass::Measured=>sensor["dependency_review_state"]=="independently_reviewed"
                    &&sensor["dependency_review_record_id"].as_str().is_some_and(|s|!s.trim().is_empty())
                    &&sensor["dependency_review_record_sha256"].as_str().is_some_and(hex64),
            };
            let response_a=responses.iter().find(|r|r["state_id"]==fault_a&&r["channel_id"]==channel_id).expect("validated static response A");
            let response_b=responses.iter().find(|r|r["state_id"]==fault_b&&r["channel_id"]==channel_id).expect("validated static response B");
            let role_ok=match evidence_class{
                PairEvidenceClass::SyntheticConformanceFixture=>sensor["source_role"]=="synthetic"
                    &&response_a["evidence_role"]=="SYNTHETIC_CONFORMANCE_FIXTURE"&&response_b["evidence_role"]=="SYNTHETIC_CONFORMANCE_FIXTURE",
                PairEvidenceClass::Measured=>sensor["source_role"]=="measured"
                    &&response_a["evidence_role"]=="INDEPENDENT_MEASUREMENT"&&response_b["evidence_role"]=="INDEPENDENT_MEASUREMENT",
            } && sensor["derived_from_channel_ids"].as_array().is_none_or(|a|a.is_empty())
                &&sensor["quality_state"]=="current"
                &&matches!(sensor["calibration_state"].as_str(),Some("current"|"not_applicable"))
                &&dependency_reviewed;
            let window_a=base_windows.iter().find(|w|w["state_id"]==fault_a).is_some_and(|w|w["status"]=="qualified");
            let window_b=base_windows.iter().find(|w|w["state_id"]==fault_b).is_some_and(|w|w["status"]=="qualified");
            let alignment_ok=alignment_evidence_ok&&timebase_matches&&alignment_uncertainty.is_finite()
                &&combined_uncertainty<=maximum_uncertainty&&period>2.0*combined_uncertainty;
            let qualified=role_ok&&sensor_timebase_ok&&window_a&&window_b&&alignment_ok
                &&!sensor_or_dependency_removed;
            let samples_a=channel["samples_a"].as_array().expect("validated samples A");
            let samples_b=channel["samples_b"].as_array().expect("validated samples B");
            let separated_span=longest_separated_trajectory_span(samples_a,samples_b,trajectory["sample_period_ms"].as_u64().unwrap());
            let temporal_ok=qualified&&separated_span+1e-9>=minimum_duration;
            let principle=txt(sensor,"measurement_principle_ref","$.sensors[]")?;
            let evidence_role=if !qualified{PairEvidenceRole::Unknown}else{match evidence_class{
                PairEvidenceClass::SyntheticConformanceFixture=>PairEvidenceRole::SyntheticConformanceFixture,
                PairEvidenceClass::Measured=>PairEvidenceRole::IndependentMeasurement,
            }};
            if qualified&&temporal_ok{
                for (a,b) in samples_a.iter().zip(samples_b.iter()){
                    let offset=a["offset_ms"].as_u64().unwrap();
                    signatures.push(ChannelPairSignature{
                        channel_id:format!("{channel_id}@{offset}ms"),
                        independence_group_ref:principle.to_owned(),
                        dependency_groups:dependencies.clone(),
                        interval_a:Some((a["value_lower"].as_f64().unwrap(),a["value_upper"].as_f64().unwrap())),
                        interval_b:Some((b["value_lower"].as_f64().unwrap(),b["value_upper"].as_f64().unwrap())),
                        availability_a:PairAvailability::Valid,
                        availability_b:PairAvailability::Valid,
                        evidence_role_a:evidence_role,
                        evidence_role_b:evidence_role,
                    });
                }
            }else if !qualified{
                for sample in samples_a{
                    let offset=sample["offset_ms"].as_u64().unwrap();
                    signatures.push(ChannelPairSignature{
                        channel_id:format!("{channel_id}@{offset}ms"),
                        independence_group_ref:principle.to_owned(),
                        dependency_groups:dependencies.clone(),
                        interval_a:None,interval_b:None,
                        availability_a:PairAvailability::Unknown,
                        availability_b:PairAvailability::Unknown,
                        evidence_role_a:evidence_role,
                        evidence_role_b:evidence_role,
                    });
                }
            }
            channel_reports.push(json!({
                "sensor_id":sensor_id,
                "channel_id":channel_id,
                "measurement_principle_ref":principle,
                "combined_alignment_uncertainty_ms":combined_uncertainty,
                "longest_separated_sample_span_ms":separated_span,
                "minimum_required_separated_duration_ms":minimum_duration,
                "qualified_temporal_separation":temporal_ok,
                "reason":if temporal_ok{"SUSTAINED_TIME_ALIGNED_INTERVAL_SEPARATION"}
                    else if !qualified{"TIMEBASE_ALIGNMENT_OR_SOURCE_QUALIFICATION_NOT_PROVEN"}
                    else{"NO_SUSTAINED_SEPARATION_FOR_REQUIRED_DURATION"}
            }));
        }
        let minimum=trajectory["minimum_independent_principles"].as_u64().unwrap() as usize;
        let classification=classify_pair_signatures(&signatures,minimum,evidence_class,&BTreeSet::new())
            .map_err(|e|err("TRAJECTORY_KERNEL_ERROR","$.trajectory_signature_cases",e))?;
        let trajectory_state=match classification.status.as_str(){
            "IDENTIFIABLE"=>"isolable",
            "NOT_IDENTIFIABLE"=>"non_isolable",
            _=>"not_evaluated",
        };
        let supplementary=if static_pair_state=="non_isolable"&&trajectory_state=="isolable"{
            "STATIC_AMBIGUITY_RESOLVED_BY_TEMPORAL_SIGNATURE"
        }else{"NO_STATIC_CONCLUSION_UPGRADED"};
        reports.push(json!({
            "signature_case_id":id,
            "pair_id":pair_id,
            "fault_a":fault_a,
            "fault_b":fault_b,
            "static_pair_state":static_pair_state,
            "trajectory_state":trajectory_state,
            "independent_measurement_principle_count":classification.observed_independent_principles,
            "minimum_independent_principles":minimum,
            "qualifying_measurement_principles":classification.independence_group_refs,
            "trajectory_channels":channel_reports,
            "supplementary_interpretation":supplementary,
            "claim_ceiling":"Trajectory separation applies only to the declared aligned relative-time samples, channels, event reference, timestamp uncertainty and evidence role. Static interval diagnosability is reported separately and is never overwritten. Synthetic trajectory separation is software conformance only, not measured engine diagnosis."
        }));
    }
    Ok(reports)
}

fn make_report(input:&Value,base:&EvaluationReport,window_assessments:Vec<Value>,scenarios:Vec<Value>,resilience:Value,trajectory_reports:Vec<Value>)->Value{
    let asset=&input["asset"];let domain=&input["operating_domain"];let digests=&input["evidence_digests"];
    let sensors:Vec<Value>=input["sensors"].as_array().unwrap().iter().map(|s|json!({
        "sensor_id":s["sensor_id"],"channel_id":s["channel_id"],"installed":s["installed"],
        "measurement_principle":s["measurement_principle_ref"],"physical_location":s["physical_location_ref"],
        "unit":s["unit"],"source_role":s["source_role"],"quality_state":s["quality_state"],
        "calibration_state":s["calibration_state"],"clock_domain":s["clock_domain"],
        "dependency_groups":s["dependency_groups"],"dependency_review_state":s["dependency_review_state"],
        "dependency_review_record_id":s["dependency_review_record_id"],
        "dependency_review_record_sha256":s["dependency_review_record_sha256"],
        "derived_from_channel_ids":s["derived_from_channel_ids"],
        "timebase_id":s["timebase_id"],"timestamp_uncertainty_ms":s["timestamp_uncertainty_ms"],
        "timestamp_uncertainty_source":s["timestamp_uncertainty_source"],
        "timebase_qualification_state":s["timebase_qualification_state"],
        "timebase_qualification_artifact_id":s["timebase_qualification_artifact_id"],
        "timebase_qualification_sha256":s["timebase_qualification_sha256"]
    })).collect();
    let stratum=domain["regime_id"].as_str().unwrap_or("unknown");
    let profile=&input["layout_profile"];
    let evidence_class=input["evidence_class"].as_str().unwrap_or("");
    let timebase_qualified_count=input["sensors"].as_array().unwrap().iter()
        .filter(|s|s["installed"]==true&&sensor_timebase_qualified(s,domain,profile,evidence_class)).count();
    let dependency_reviewed_count=input["sensors"].as_array().unwrap().iter()
        .filter(|s|s["installed"]==true&&match (evidence_class,s["dependency_review_state"].as_str()) {
            ("SYNTHETIC_CONFORMANCE_FIXTURE",Some("synthetic_conformance"))=>true,
            ("MEASURED",Some("independently_reviewed"))=>s["dependency_review_record_id"].as_str().is_some_and(|x|!x.trim().is_empty())
                &&s["dependency_review_record_sha256"].as_str().is_some_and(hex64),
            _=>false
        }).count();
    let installed_count=input["sensors"].as_array().unwrap().iter().filter(|s|s["installed"]==true).count();
    let (fault_report,ambiguity_report)=fault_assessments(input,base);
    let metric=|id:&str,value:usize|json!({"metric_id":id,"value":value,"unit":"count","stratum":stratum,"sample_count":0});
    json!({
        "schema_version":"0.1.0","assessment_id":input["assessment_id"],"generated_at":input["generated_at"],
        "asset":{"asset_id":asset["asset_id"],"asset_class":asset["asset_class"],"vessel_id":asset["vessel_id"],"engine_id":asset["engine_id"],"configuration_revision":asset["configuration_revision"],"configuration_sha256":asset["configuration_sha256"]},
        "operating_domain":{"regime_id":domain["regime_id"],"description":domain["description"],"qualification_state":domain["qualification_state"],"load_percent":domain["load_percent"],"speed_rpm":domain["speed_rpm"],"transient_state":domain["transient_state"],"measurement_timebase_id":domain["measurement_timebase_id"],"maximum_timestamp_uncertainty_ms":profile["maximum_timestamp_uncertainty_ms"]},
        "evidence_digests":{"sensor_topology_sha256":digests["sensor_topology_sha256"],"fault_model_sha256":digests["fault_model_sha256"],"evaluator_sha256":digests["evaluator_sha256"],"dataset_sha256":digests["dataset_sha256"],"split_manifest_sha256":digests["split_manifest_sha256"],"model_sha256":digests["model_sha256"]},
        "sensor_inventory":sensors,"fault_assessments":fault_report,"ambiguity_classes":ambiguity_report,
        "measurement_window_assessments":window_assessments,
        "pair_assessments":pair_assessments(input,base),"sensor_loss_scenarios":scenarios,
        "resilience_analysis":resilience,
        "trajectory_pair_assessments":trajectory_reports,
        "metrics":[metric("installed_sensor_count",base.channel_count),metric("independent_evidence_group_count",base.independence_group_count),
            metric("state_hypothesis_count",base.state_count),metric("required_fault_pair_count",base.required_pair_count),
            metric("identifiable_fault_pair_count",base.identifiable_pair_count),metric("non_identifiable_fault_pair_count",base.not_identifiable_pair_count),
            metric("insufficient_evidence_fault_pair_count",base.insufficient_evidence_pair_count),metric("critical_unidentifiable_fault_pair_count",base.critical_unidentifiable_pair_count),
            metric("timebase_qualified_installed_sensor_count",timebase_qualified_count),
            metric("timebase_unqualified_installed_sensor_count",installed_count.saturating_sub(timebase_qualified_count)),
            metric("dependency_reviewed_installed_sensor_count",dependency_reviewed_count),
            metric("dependency_unreviewed_installed_sensor_count",installed_count.saturating_sub(dependency_reviewed_count))],
        "authority_boundary":{"read_only":true,"actuation":false,"maintenance_authorization":false,"mode_transition_authorization":false,"restriction_clearance":false},
        "claim_ceiling":"Structural detectability against the declared reference and pairwise fault isolability for the exact declared asset/configuration, installed sensors, dependency groups, operating domain, response intervals, and guaranteed common acquisition-window overlap after per-sensor timestamp uncertainty. Matching measurement-window identifiers alone are not sufficient. This does not measure empirical event-detection accuracy or trained-model performance. Timebase hashes bind declared artifact references but do not authenticate the clock qualification process or prove the physical synchronization network. Synthetic timebase evidence remains synthetic only. No physical engine validation, shipboard safety, maintenance authorization, ODD clearance or permission to operate is established."
    })
}

pub fn evaluate_maritime_case(input:&Value,clock:&Value,clock_sha256:&str)->Check<Value>{
    validate_input(input)?;
    validate_clock_qualification(input,clock,clock_sha256)?;
    let empty_sensors=BTreeSet::new();
    let empty_dependencies=BTreeSet::new();
    let (base_windows,base_unaligned)=measurement_window_assessments(input,&empty_sensors,&empty_dependencies);
    let (base_case,parent)=make_parent_and_case(input,&empty_sensors,&empty_dependencies,&base_unaligned)?;
    let base=evaluate_case(&base_case,&parent)?;
    let mut scenarios=Vec::new();
    for scenario in input["common_cause_scenarios"].as_array().unwrap(){
        let removed_sensors=set(scenario,"removed_sensor_ids","$.common_cause_scenarios[]")?;
        let removed_dependencies=set(scenario,"removed_dependency_groups","$.common_cause_scenarios[]")?;
        let (scenario_windows,scenario_unaligned)=measurement_window_assessments(input,&removed_sensors,&removed_dependencies);
        let (case,scenario_parent)=make_parent_and_case(input,&removed_sensors,&removed_dependencies,&scenario_unaligned)?;
        let replay=evaluate_case(&case,&scenario_parent)?;
        let lost:Vec<Value>=base.pair_results.iter()
            .filter(|p|p.status=="IDENTIFIABLE")
            .filter(|p|pair_status(&replay,&p.state_a_id,&p.state_b_id)!=Some("IDENTIFIABLE"))
            .map(|p|json!({"fault_a":p.state_a_id,"fault_b":p.state_b_id}))
            .collect();
        scenarios.push(json!({
            "scenario_id":scenario["scenario_id"],
            "removed_sensor_ids":removed_sensors,
            "removed_dependency_groups":removed_dependencies,
            "assessment_state":"recomputed",
            "measurement_window_assessments":scenario_windows,
            "lost_isolability_pairs":lost
        }));
    }
    let trajectory_reports=trajectory_pair_assessments(
        input,&base,&base_windows,&empty_sensors,&empty_dependencies
    )?;
    let resilience=resilience_analysis(input,&base,&base_windows,&trajectory_reports)?;
    Ok(make_report(input,&base,base_windows,scenarios,resilience,trajectory_reports))
}

pub fn evaluate_and_bind_execution(input:&Value,clock:&Value,source_commit:&str,case_sha256:&str,clock_sha256:&str)->Check<Value>{
    if source_commit.len()!=40||!source_commit.bytes().all(|b|b.is_ascii_digit()||(b'a'..=b'f').contains(&b)){
        return Err(err("INVALID_SOURCE_COMMIT","$.execution.source_commit","expected lowercase 40-hex commit"));
    }
    if !hex64(case_sha256){return Err(err("INVALID_CASE_DIGEST","$.execution.case_sha256","expected lowercase SHA-256"));}
    let mut report=evaluate_maritime_case(input,clock,clock_sha256)?;
    report["execution"]=json!({"source_commit":source_commit,"case_sha256":case_sha256,
        "clock_qualification_id":clock["qualification_id"],"clock_qualification_sha256":clock_sha256,"timebase_id":clock["timebase_id"]});
    Ok(report)
}

#[cfg(test)]
mod tests{
    use super::*;
    fn fixture()->Value{
        serde_json::from_str(include_str!("../../../../benchmarks/maritime-observability/maritime-observability-case-v0.1.json")).expect("fixture JSON")
    }
    fn clock_fixture()->Value{
        serde_json::from_str(include_str!("../../../../benchmarks/maritime-observability/synthetic-clock-qualification-v0.1.json")).expect("clock fixture JSON")
    }
    fn clock_sha()->&'static str{"b4d2b0bae0b963afc4612bf7b0b4822e8d8d66cff4c810a2bde6f348d85f0e15"}
    fn eval(input:&Value)->Check<Value>{evaluate_maritime_case(input,&clock_fixture(),clock_sha())}
    #[test]
    fn synthetic_maritime_case_maps_to_generic_evaluator(){
        let r=eval(&fixture()).expect("adapter evaluates");
        assert_eq!(r["pair_assessments"].as_array().unwrap().len(),11);
        assert_eq!(r["fault_assessments"].as_array().unwrap().len(),5);
        assert_eq!(r["sensor_loss_scenarios"].as_array().unwrap().len(),4);
        assert_eq!(r["metrics"].as_array().unwrap().iter().find(|m|m["metric_id"]=="dependency_reviewed_installed_sensor_count").unwrap()["value"],5);
        assert_eq!(r["authority_boundary"]["actuation"],false);
    }
    #[test]
    fn null_evidence_digests_remain_unbound_profile_identity_is_not_a_hash() {
        let c=fixture();
        assert!(c["evidence_digests"]["sensor_topology_sha256"].is_null());
        let (adapted,parent)=make_parent_and_case(&c,&BTreeSet::new(),&BTreeSet::new(),&BTreeSet::new()).unwrap();
        let installation=parent["profile"]["installation_profile_ref"].as_str().unwrap();
        assert!(installation.starts_with("installation-profile:"));
        assert!(!installation.starts_with("sensor-topology-sha256:"));
        assert_eq!(adapted["parent_context"]["installation_profile_ref"],parent["profile"]["installation_profile_ref"]);
    }
    #[test]
    fn gateway_loss_recomputes_and_withdraws_pair_isolability(){
        let r=eval(&fixture()).expect("adapter evaluates");
        assert_eq!(r["sensor_loss_scenarios"][0]["assessment_state"],"recomputed");
        assert!(!r["sensor_loss_scenarios"][0]["lost_isolability_pairs"].as_array().unwrap().is_empty());
    }
    #[test]
    fn expired_exhaust_temperature_calibration_removes_that_channel_but_other_principles_preserve_isolation(){
        let mut c=fixture();c["sensors"][2]["calibration_state"]=json!("expired");
        let r=eval(&c).expect("abstention is a valid report");
        let p=r["pair_assessments"].as_array().unwrap().iter().find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]=="ENGINE_COOLING_RESTRICTION").unwrap();
        // Speed, rack-position and vibration remain independent evidence.
        assert_eq!(p["state"],"isolable");
        assert!(!p["evidence_refs"].as_array().unwrap().iter().any(|e|
            e["channel_ids"].as_array().unwrap().contains(&json!("exhaust-temperature"))));
    }
    #[test]
    fn derived_signal_is_not_promoted_to_independent_measurement(){
        let mut c=fixture();c["sensors"][0]["source_role"]=json!("derived");
        c["sensors"][0]["derived_from_channel_ids"]=json!(["shaft-speed"]);
        let r=eval(&c).expect("derived channel remains non-independent");
        let p=r["pair_assessments"].as_array().unwrap().iter().find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]=="ENGINE_FUEL_INJECTION_FAULT").unwrap();
        assert!(!p["evidence_refs"].as_array().unwrap().iter().any(|e|e["channel_ids"][0]=="fuel-rail-pressure"));
    }
    #[test]
    fn per_fault_output_distinguishes_detection_from_isolation_and_preserves_ambiguity(){
        let r=eval(&fixture()).unwrap();
        let assessments=r["fault_assessments"].as_array().unwrap();
        let find=|id:&str|assessments.iter().find(|x|x["fault_id"]==id).unwrap();
        assert_eq!(find("ENGINE_FUEL_INJECTION_FAULT")["state"],"observable_and_isolable");
        assert_eq!(find("ENGINE_TURBOCHARGER_DEGRADATION")["state"],"ambiguity_class");
        assert_eq!(find("ENGINE_COOLING_RESTRICTION")["state"],"ambiguity_class");
        assert_eq!(find("FUEL_PRESSURE_SENSOR_FAULT")["state"],"unobservable");
        assert_eq!(find("ENGINE_UNKNOWN_OUT_OF_DOMAIN")["state"],"unsupported_regime");
        assert_eq!(find("ENGINE_TURBOCHARGER_DEGRADATION")["ambiguity_class_id"],find("ENGINE_COOLING_RESTRICTION")["ambiguity_class_id"]);
        assert!(find("ENGINE_TURBOCHARGER_DEGRADATION")["candidate_fault_ids"].as_array().unwrap().contains(&json!("ENGINE_COOLING_RESTRICTION")));
    }
    #[test]
    fn simulated_response_role_is_not_promoted_from_sensor_inventory(){
        let mut c=fixture();
        for response in c["responses"].as_array_mut().unwrap().iter_mut().filter(|r|r["state_id"]=="ENGINE_FUEL_INJECTION_FAULT"){
            response["evidence_role"]=json!("MODEL_ESTIMATE");
        }
        let r=eval(&c).unwrap();
        let p=r["pair_assessments"].as_array().unwrap().iter().find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]=="ENGINE_FUEL_INJECTION_FAULT").unwrap();
        assert_eq!(p["state"],"not_evaluated");
    }
    #[test]
    fn unknown_fields_and_duplicate_response_ids_fail_closed(){
        let mut c=fixture();c["responses"][0]["unreviewed_override"]=json!(true);
        assert!(eval(&c).is_err());
        c=fixture();c["responses"][1]["response_id"]=c["responses"][0]["response_id"].clone();
        assert!(eval(&c).is_err());
    }
    #[test]
    fn reversed_duplicate_fault_pair_is_rejected(){
        let mut c=fixture();
        let mut duplicate=c["required_fault_pairs"][0].clone();
        let a=duplicate["fault_a"].clone();duplicate["fault_a"]=duplicate["fault_b"].clone();duplicate["fault_b"]=a;
        duplicate["pair_id"]=json!("reversed-duplicate");
        c["required_fault_pairs"].as_array_mut().unwrap().push(duplicate);
        assert!(eval(&c).is_err());
    }
    #[test]
    fn responses_for_each_fault_state_must_share_one_measurement_window(){
        let mut c=fixture();
        c["responses"][1]["measurement_window_ref"]=json!("window:wrong-clock-window");
        let errors=eval(&c).expect_err("mixed response windows must not be combined");
        assert!(errors.iter().any(|e|e.code=="INCOHERENT_STATE_MEASUREMENT_WINDOW"));
    }
    #[test]
    fn required_pair_matrix_must_cover_fault_isolation_not_only_normal_detection(){
        let mut c=fixture();c["required_fault_pairs"].as_array_mut().unwrap().pop();
        assert!(eval(&c).is_err());
    }
    #[test]
    fn report_preserves_configuration_and_authority_boundary(){
        let r=eval(&fixture()).unwrap();
        assert_eq!(r["asset"]["configuration_revision"],"engine-r1");
        assert_eq!(r["authority_boundary"]["read_only"],true);
        assert_eq!(r["authority_boundary"]["maintenance_authorization"],false);
        assert_eq!(r["authority_boundary"]["mode_transition_authorization"],false);
    }
    #[test]
    fn exact_execution_bindings_are_required(){
        let c=fixture();
        assert!(evaluate_and_bind_execution(&c,&clock_fixture(),&"d".repeat(40),&"e".repeat(64),clock_sha()).is_ok());
        assert!(evaluate_and_bind_execution(&c,&clock_fixture(),"not-a-commit",&"e".repeat(64),clock_sha()).is_err());
    }
    #[test]
    fn unqualified_sensor_timebase_is_excluded_from_physical_evidence(){
        let mut c=fixture();
        c["sensors"][0]["timebase_qualification_state"]=json!("unknown");
        c["sensors"][0]["timebase_qualification_sha256"]=Value::Null;
        c["sensors"][0]["timebase_qualification_artifact_id"]=Value::Null;
        c["sensors"][0]["timestamp_uncertainty_ms"]=Value::Null;
        c["sensors"][0]["timebase_id"]=json!("unmapped-clock-domain");
        let r=eval(&c).unwrap();
        assert_eq!(r["metrics"].as_array().unwrap().iter()
            .find(|m|m["metric_id"]=="timebase_unqualified_installed_sensor_count").unwrap()["value"],1);
        assert!(!r["pair_assessments"].as_array().unwrap().iter()
            .flat_map(|p|p["evidence_refs"].as_array().unwrap())
            .any(|e|e["channel_ids"].as_array().unwrap().contains(&json!("fuel-rail-pressure"))));
    }
    #[test]
    fn uncertainty_above_declared_layout_bound_is_not_qualified(){
        let mut c=fixture();
        c["sensors"][0]["timestamp_uncertainty_ms"]=json!(6.0);
        let r=eval(&c).unwrap();
        assert_eq!(r["metrics"].as_array().unwrap().iter()
            .find(|m|m["metric_id"]=="timebase_unqualified_installed_sensor_count").unwrap()["value"],1);
        assert!(!r["pair_assessments"].as_array().unwrap().iter()
            .flat_map(|p|p["evidence_refs"].as_array().unwrap())
            .any(|e|e["channel_ids"].as_array().unwrap().contains(&json!("fuel-rail-pressure"))));
    }
    #[test]
    fn unqualified_operating_domain_does_not_emit_fault_diagnoses(){
        let mut c=fixture();c["operating_domain"]["qualification_state"]=json!("not_evaluated");
        let r=eval(&c).unwrap();
        for fault in r["fault_assessments"].as_array().unwrap(){
            if fault["fault_id"]!="ENGINE_UNKNOWN_OUT_OF_DOMAIN" {
                assert_eq!(fault["state"],"not_evaluated");
            }
        }
        assert_eq!(r["pair_assessments"].as_array().unwrap().iter().filter(|p|p["state"]=="not_evaluated").count(),11);
    }
    #[test]
    fn model_estimate_role_is_preserved_in_report_not_renamed_simulation(){
        let mut c=fixture();
        for response in c["responses"].as_array_mut().unwrap().iter_mut()
            .filter(|r|r["state_id"]=="ENGINE_FUEL_INJECTION_FAULT"){
            response["evidence_role"]=json!("MODEL_ESTIMATE");
        }
        let r=eval(&c).unwrap();
        let fault=r["fault_assessments"].as_array().unwrap().iter()
            .find(|f|f["fault_id"]=="ENGINE_FUEL_INJECTION_FAULT").unwrap();
        assert!(fault["signature_evidence"].as_array().unwrap()
            .iter().any(|e|e["evidence_role"]=="model_estimate"));
    }

    #[test]
    fn clock_artifact_semantics_and_hash_must_match_every_installed_sensor(){
        let c=fixture();let mut clock=clock_fixture();
        clock["timebase_id"]=json!("other-timebase");
        assert!(evaluate_maritime_case(&c,&clock,clock_sha()).is_err());
        clock=clock_fixture();clock["layout_maximum_timestamp_uncertainty_ms"]=json!(10);
        assert!(evaluate_maritime_case(&c,&clock,clock_sha()).is_err());
        clock=clock_fixture();assert!(evaluate_maritime_case(&c,&clock,&"f".repeat(64)).is_err());
    }
    #[test]
    fn synthetic_identity_mapping_requires_one_declared_clock_domain(){
        let mut c=fixture();c["sensors"][1]["clock_domain"]=json!("different-sensor-clock");
        assert!(eval(&c).is_err());
    }
    #[test]
    fn operating_and_layout_uncertainty_limits_cannot_diverge(){
        let mut c=fixture();c["operating_domain"]["maximum_timestamp_uncertainty_ms"]=json!(10);
        assert!(eval(&c).is_err());
    }
    #[test]
    fn ambiguity_sets_are_pairwise_cliques_not_connected_components(){
        let mut c=fixture();
        let ranges=[
            ("ENGINE_NORMAL_STEADY",10.0,11.0),
            ("ENGINE_TURBOCHARGER_DEGRADATION",0.0,2.0),
            ("ENGINE_FUEL_INJECTION_FAULT",1.0,3.0),
            ("ENGINE_COOLING_RESTRICTION",2.5,4.0)
        ];
        for response in c["responses"].as_array_mut().unwrap(){
            if let Some((_,lo,hi))=ranges.iter().find(|(id,_,_)|response["state_id"]==*id){
                response["value_lower"]=json!(lo);response["value_upper"]=json!(hi);
            }
        }
        let report=eval(&c).unwrap();
        let classes=report["ambiguity_classes"].as_array().unwrap();
        assert_eq!(classes.len(),2);
        assert!(classes.iter().all(|class|class["candidate_fault_ids"].as_array().unwrap().len()==2));
        let faults=report["fault_assessments"].as_array().unwrap();
        let get=|id:&str|faults.iter().find(|fault|fault["fault_id"]==id).unwrap();
        assert_eq!(get("ENGINE_FUEL_INJECTION_FAULT")["ambiguity_class_ids"].as_array().unwrap().len(),2);
        assert!(get("ENGINE_FUEL_INJECTION_FAULT")["ambiguity_class_id"].is_null());
        assert!(!get("ENGINE_TURBOCHARGER_DEGRADATION")["candidate_fault_ids"].as_array().unwrap().contains(&json!("ENGINE_COOLING_RESTRICTION")));
        assert!(!get("ENGINE_COOLING_RESTRICTION")["candidate_fault_ids"].as_array().unwrap().contains(&json!("ENGINE_TURBOCHARGER_DEGRADATION")));
    }
    #[test]
    fn unreviewed_dependency_map_withdraws_only_affected_sensor_evidence(){
        let mut c=fixture();
        c["sensors"][0]["dependency_review_state"]=json!("unknown");
        let r=eval(&c).expect("unreviewed sensor is preserved but excluded");
        let pair=r["pair_assessments"].as_array().unwrap().iter()
            .find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]=="ENGINE_FUEL_INJECTION_FAULT").unwrap();
        // The fuel-pressure evidence is withdrawn, but the other qualified
        // independent principles still make this particular pair isolable.
        assert_eq!(pair["state"],"isolable");
        assert!(!pair["evidence_refs"].as_array().unwrap().iter()
            .any(|e|e["channel_ids"].as_array().unwrap().contains(&json!("fuel-rail-pressure"))));
        assert_eq!(r["metrics"].as_array().unwrap().iter()
            .find(|m|m["metric_id"]=="dependency_unreviewed_installed_sensor_count").unwrap()["value"],1);
    }
    #[test]
    fn synthetic_fixture_cannot_claim_independent_physical_dependency_review(){
        let mut c=fixture();
        c["sensors"][0]["dependency_review_state"]=json!("independently_reviewed");
        c["sensors"][0]["dependency_review_record_id"]=json!("fake-review");
        c["sensors"][0]["dependency_review_record_sha256"]=json!("a".repeat(64));
        assert!(eval(&c).is_err());
    }
    #[test]
    fn installed_sensor_without_dependency_groups_is_rejected(){
        let mut c=fixture();
        c["sensors"][0]["dependency_groups"]=json!([]);
        assert!(eval(&c).is_err());
    }
    #[test]
    fn every_shared_dependency_requires_an_explicit_outage_scenario(){
        let mut c=fixture();
        c["common_cause_scenarios"].as_array_mut().unwrap().retain(|s|s["scenario_id"]!="shared-clock-loss");
        c["common_cause_scenarios"].as_array_mut().unwrap().retain(|s|s["scenario_id"]!="gateway-and-clock-loss");
        assert!(eval(&c).is_err());
    }
    #[test]
    fn combined_gateway_and_clock_loss_recomputes_all_base_isolable_pairs(){
        let r=eval(&fixture()).unwrap();
        let scenario=r["sensor_loss_scenarios"].as_array().unwrap().iter()
            .find(|s|s["scenario_id"]=="gateway-and-clock-loss").unwrap();
        assert_eq!(scenario["assessment_state"],"recomputed");
        assert_eq!(scenario["lost_isolability_pairs"].as_array().unwrap().len(),8);
    }
    #[test]
    fn common_window_report_uses_guaranteed_overlap_after_sensor_clock_uncertainty(){
        let r=eval(&fixture()).unwrap();
        let windows=r["measurement_window_assessments"].as_array().unwrap();
        assert_eq!(windows.len(),6);
        let in_domain:Vec<&Value>=windows.iter().filter(|w|w["state_id"]!="ENGINE_UNKNOWN_OUT_OF_DOMAIN").collect();
        assert_eq!(in_domain.len(),5);
        assert!(in_domain.iter().all(|w|w["status"]=="qualified"));
        assert!(in_domain.iter().all(|w|w["guaranteed_common_overlap_ms"].as_f64().unwrap()>=w["required_common_overlap_ms"].as_f64().unwrap()));
        assert_eq!(windows.iter().find(|w|w["state_id"]=="ENGINE_UNKNOWN_OUT_OF_DOMAIN").unwrap()["status"],"not_evaluated");
        assert_eq!(windows[0]["guaranteed_common_overlap_ms"],json!(9998.0));
    }
    #[test]
    fn matching_window_ids_without_guaranteed_temporal_overlap_are_not_admitted(){
        let mut c=fixture();let fault="ENGINE_FUEL_INJECTION_FAULT";
        for response in c["responses"].as_array_mut().unwrap().iter_mut().filter(|r|r["state_id"]==fault&&r["channel_id"]=="fuel-rail-pressure"){
            response["window_start_ms"]=json!(response["window_start_ms"].as_f64().unwrap()+7000.0);
            response["window_end_ms"]=json!(response["window_end_ms"].as_f64().unwrap()+7000.0);
        }
        let r=eval(&c).unwrap();
        let w=r["measurement_window_assessments"].as_array().unwrap().iter().find(|w|w["state_id"]==fault).unwrap();
        assert_eq!(w["status"],"not_evaluated");
        let pair=r["pair_assessments"].as_array().unwrap().iter().find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]==fault).unwrap();
        assert_eq!(pair["state"],"not_evaluated");
    }
    #[test]
    fn excluding_the_unaligned_sensor_recomputes_and_can_restore_window_coherence(){
        let mut c=fixture();let fault="ENGINE_FUEL_INJECTION_FAULT";
        for response in c["responses"].as_array_mut().unwrap().iter_mut().filter(|r|r["state_id"]==fault&&r["channel_id"]=="fuel-rail-pressure"){
            response["window_start_ms"]=json!(response["window_start_ms"].as_f64().unwrap()+7000.0);
            response["window_end_ms"]=json!(response["window_end_ms"].as_f64().unwrap()+7000.0);
        }
        let r=eval(&c).unwrap();
        let scenario=r["sensor_loss_scenarios"].as_array().unwrap().iter().find(|s|s["scenario_id"]=="fuel-pressure-transducer-loss").unwrap();
        let w=scenario["measurement_window_assessments"].as_array().unwrap().iter().find(|w|w["state_id"]==fault).unwrap();
        assert_eq!(w["status"],"qualified");
    }
    #[test]
    fn uncertainty_reduces_guaranteed_overlap_and_boundary_is_inclusive(){
        let mut c=fixture();c["layout_profile"]["minimum_common_window_overlap_ms"]=json!(10000.0);
        let r=eval(&c).unwrap();assert_eq!(r["measurement_window_assessments"][0]["status"],"not_evaluated");
        c["layout_profile"]["minimum_common_window_overlap_ms"]=json!(9998.0);
        let r=eval(&c).unwrap();assert_eq!(r["measurement_window_assessments"][0]["status"],"qualified");
    }
    #[test]
    fn measurement_window_timestamps_must_be_ordered_and_finite(){
        let mut c=fixture();c["responses"][0]["window_end_ms"]=c["responses"][0]["window_start_ms"].clone();
        assert!(eval(&c).is_err());
        c=fixture();c["responses"][0]["window_start_ms"]=json!(-1.0);assert!(eval(&c).is_err());
    }

    #[test]
    fn exact_resilience_enumerates_sensor_and_shared_dependency_singletons_and_pairs(){
        let r=eval(&fixture()).unwrap();
        let x=&r["resilience_analysis"];
        assert_eq!(x["failure_atom_count"],7);
        assert_eq!(x["maximum_failure_cut_order"],2);
        assert_eq!(x["failure_scenario_count"],28);
        assert_eq!(x["critical_pair_count"],3);
        assert_eq!(x["baseline_qualified_critical_pair_count"],2);
        assert_eq!(x["cut_found_critical_pair_count"],2);
        assert_eq!(x["baseline_not_qualified_critical_pair_count"],1);
        let pairs=x["pair_results"].as_array().unwrap();
        let injection=pairs.iter().find(|p|p["fault_a"]=="ENGINE_NORMAL_STEADY"&&p["fault_b"]=="ENGINE_FUEL_INJECTION_FAULT").unwrap();
        assert_eq!(injection["minimum_cut_order"],1);
        assert_eq!(injection["maximum_tolerated_failure_atoms"],0);
        let cut_groups:BTreeSet<String>=injection["minimal_cut_sets"].as_array().unwrap().iter()
            .flat_map(|cut|cut["failure_atoms"].as_array().unwrap())
            .filter(|atom|atom["kind"]=="dependency_group")
            .map(|atom|atom["identity"].as_str().unwrap().to_owned()).collect();
        assert_eq!(cut_groups,BTreeSet::from([
            "clock:ptp-engine-1".to_owned(),
            "gateway:engine-io-1".to_owned()
        ]));
        let sensor_fault=pairs.iter().find(|p|p["fault_b"]=="FUEL_PRESSURE_SENSOR_FAULT").unwrap();
        assert_eq!(sensor_fault["status"],"baseline_not_qualified");
        assert!(sensor_fault["minimum_cut_order"].is_null());
    }
    #[test]
    fn exact_resilience_cut_sets_are_inclusion_minimal(){
        let r=eval(&fixture()).unwrap();
        for pair in r["resilience_analysis"]["pair_results"].as_array().unwrap(){
            let cuts=pair["minimal_cut_sets"].as_array().unwrap();
            let sets:Vec<BTreeSet<String>>=cuts.iter().map(|cut|
                cut["failure_atoms"].as_array().unwrap().iter().map(|atom|
                    format!("{}:{}",atom["kind"].as_str().unwrap(),atom["identity"].as_str().unwrap())
                ).collect()
            ).collect();
            for (i,a) in sets.iter().enumerate(){
                for (j,b) in sets.iter().enumerate(){
                    if i!=j{assert!(!(a.len()<b.len()&&a.is_subset(b)),"cut set {i} is a strict subset of {j}");}
                }
            }
        }
    }
    #[test]
    fn exact_resilience_search_bound_fails_closed_instead_of_approximating(){
        let mut c=fixture();
        c["layout_profile"]["maximum_failure_atoms_for_exact_cut_analysis"]=json!(6);
        let errors=eval(&c).expect_err("seven atoms exceed declared exact bound");
        assert!(errors.iter().any(|e|e.code=="EXACT_FAILURE_ATOM_LIMIT_EXCEEDED"));
    }
    #[test]
    fn exact_resilience_v01_rejects_cut_order_above_two(){
        let mut c=fixture();
        c["layout_profile"]["maximum_failure_cut_order"]=json!(3);
        let errors=eval(&c).expect_err("v0.1 must not approximate order three");
        assert!(errors.iter().any(|e|e.code=="INVALID_FAILURE_CUT_BOUNDS"));
    }


    #[test]
    fn aligned_trajectories_resolve_static_turbo_cooling_ambiguity_without_overwriting_static_result(){
        let r=eval(&fixture()).unwrap();
        let static_pair=r["pair_assessments"].as_array().unwrap().iter()
            .find(|p|p["fault_a"]=="ENGINE_TURBOCHARGER_DEGRADATION"&&p["fault_b"]=="ENGINE_COOLING_RESTRICTION").unwrap();
        assert_eq!(static_pair["state"],"non_isolable");
        let temporal=r["trajectory_pair_assessments"].as_array().unwrap().iter()
            .find(|p|p["pair_id"]=="engine_turbocharger_degradation-vs-engine_cooling_restriction").unwrap();
        assert_eq!(temporal["static_pair_state"],"non_isolable");
        assert_eq!(temporal["trajectory_state"],"isolable");
        assert_eq!(temporal["independent_measurement_principle_count"],2);
        assert_eq!(temporal["supplementary_interpretation"],"STATIC_AMBIGUITY_RESOLVED_BY_TEMPORAL_SIGNATURE");
        assert!(temporal["trajectory_channels"].as_array().unwrap().iter().all(|c|c["qualified_temporal_separation"]==true));
    }
    #[test]
    fn trajectory_sample_grid_mismatch_is_rejected(){
        let mut c=fixture();
        c["trajectory_signature_cases"][0]["trajectory_channels"][0]["samples_b"][2]["offset_ms"]=json!(2500);
        let errors=eval(&c).expect_err("unmatched event-relative offsets must fail closed");
        assert!(errors.iter().any(|e|e.code=="TRAJECTORY_OFFSET_MISMATCH"));
    }
    #[test]
    fn one_transient_separated_sample_does_not_qualify_as_sustained_temporal_signature(){
        let mut c=fixture();
        for ch in c["trajectory_signature_cases"][0]["trajectory_channels"].as_array_mut().unwrap(){
            let n=ch["samples_b"].as_array().unwrap().len();
            for i in 0..n { ch["samples_b"][i]=ch["samples_a"][i].clone(); }
            ch["samples_b"][0]["value_lower"]=json!(ch["samples_a"][0]["value_lower"].as_f64().unwrap()+5.0);
            ch["samples_b"][0]["value_upper"]=json!(ch["samples_a"][0]["value_upper"].as_f64().unwrap()+5.0);
        }
        let r=eval(&c).unwrap();
        let temporal=&r["trajectory_pair_assessments"][0];
        assert_eq!(temporal["trajectory_state"],"non_isolable");
        assert_eq!(temporal["independent_measurement_principle_count"],0);
    }
    #[test]
    fn unproven_event_alignment_yields_not_evaluated_not_static_promotion(){
        let mut c=fixture();
        c["trajectory_signature_cases"][0]["event_alignment_state"]=json!("unknown");
        let r=eval(&c).unwrap();
        assert_eq!(r["trajectory_pair_assessments"][0]["trajectory_state"],"not_evaluated");
        assert_eq!(r["trajectory_pair_assessments"][0]["static_pair_state"],"non_isolable");
    }
    #[test]
    fn trajectory_sensor_principles_are_counted_once_even_with_many_time_samples(){
        let mut c=fixture();
        c["sensors"][4]["measurement_principle_ref"]=c["sensors"][1]["measurement_principle_ref"].clone();
        let r=eval(&c).unwrap();
        assert_eq!(r["trajectory_pair_assessments"][0]["trajectory_state"],"non_isolable");
        assert_eq!(r["trajectory_pair_assessments"][0]["independent_measurement_principle_count"],1);
    }
    #[test]
    fn model_estimate_static_responses_cannot_support_measured_temporal_signature(){
        let mut c=fixture();
        for response in c["responses"].as_array_mut().unwrap().iter_mut().filter(|r|
            (r["state_id"]=="ENGINE_TURBOCHARGER_DEGRADATION"||r["state_id"]=="ENGINE_COOLING_RESTRICTION")
            &&(r["channel_id"]=="shaft-speed"||r["channel_id"]=="vibration-rms")){
            response["evidence_role"]=json!("MODEL_ESTIMATE");
        }
        let r=eval(&c).unwrap();
        assert_eq!(r["trajectory_pair_assessments"][0]["trajectory_state"],"not_evaluated");
    }


    #[test]
    fn temporal_resilience_uses_the_same_exact_28_failure_scenarios_and_finds_four_minimal_cuts(){
        let r=eval(&fixture()).unwrap();
        let resilience=&r["resilience_analysis"];
        assert_eq!(resilience["failure_scenario_count"],28);
        assert_eq!(resilience["trajectory_pair_count"],1);
        assert_eq!(resilience["trajectory_baseline_qualified_count"],1);
        assert_eq!(resilience["trajectory_cut_found_count"],1);
        assert_eq!(resilience["trajectory_no_cut_found_within_bound_count"],0);
        assert_eq!(resilience["trajectory_baseline_not_qualified_count"],0);
        let temporal=&resilience["trajectory_pair_results"][0];
        assert_eq!(temporal["baseline_static_state"],"non_isolable");
        assert_eq!(temporal["baseline_trajectory_state"],"isolable");
        assert_eq!(temporal["status"],"cut_found_within_bound");
        assert_eq!(temporal["minimum_cut_order"],1);
        assert_eq!(temporal["maximum_tolerated_failure_atoms"],0);
        let cut_atoms:BTreeSet<String>=temporal["minimal_cut_sets"].as_array().unwrap().iter()
            .flat_map(|cut|cut["failure_atoms"].as_array().unwrap())
            .map(|atom|format!("{}:{}",atom["kind"].as_str().unwrap(),atom["identity"].as_str().unwrap()))
            .collect();
        assert_eq!(cut_atoms,BTreeSet::from([
            "sensor:crankshaft-speed-sensor".to_owned(),
            "sensor:engine-vibration-sensor".to_owned(),
            "dependency_group:clock:ptp-engine-1".to_owned(),
            "dependency_group:gateway:engine-io-1".to_owned()
        ]));
        assert_eq!(temporal["minimal_cut_sets"].as_array().unwrap().len(),4);
        assert!(temporal["minimal_cut_sets"].as_array().unwrap().iter().all(|cut|cut["order"]==1));
    }
    #[test]
    fn losing_non_temporal_sensor_does_not_destroy_temporal_pair_isolation(){
        let r=eval(&fixture()).unwrap();
        let temporal=&r["resilience_analysis"]["trajectory_pair_results"][0];
        let cuts:BTreeSet<String>=temporal["minimal_cut_sets"].as_array().unwrap().iter()
            .flat_map(|cut|cut["failure_atoms"].as_array().unwrap())
            .map(|atom|atom["identity"].as_str().unwrap().to_owned())
            .collect();
        assert!(!cuts.contains("fuel-rail-pressure-sensor"));
        assert!(!cuts.contains("exhaust-thermocouple"));
        assert!(!cuts.contains("rack-position-transducer"));
    }

    #[test]
    fn same_input_is_deterministic(){
        let c=fixture();assert_eq!(eval(&c).unwrap(),eval(&c).unwrap());
    }
}
