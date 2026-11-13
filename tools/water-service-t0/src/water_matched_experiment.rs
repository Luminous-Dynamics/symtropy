// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only input-parity admission for paired water benchmark runs.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS: &str = "water-matched-experiment-pairs-v0.1";
const ORACLE: &str = "water-matched-experiment-admission-oracle-v0.1";
const PROTOCOL: &str = "WATER-MATCHED-EXPERIMENT-0.1";
const COUNT: usize = 10;
const DEPLOYMENT_FIELDS: [&str; 20] = [
    "scenario_ref", "network_id", "topology_revision", "topology_digest",
    "geometry_revision", "geometry_digest", "demand_trace_digest",
    "disturbance_trace_digest", "random_seed", "sensor_set",
    "sampling_period_seconds", "missingness_profile_ref", "time_grid_ref",
    "unit_system", "scoring_horizon_seconds", "service_floor_profile_digest",
    "required_service_area_ids", "actuator_envelope_digest",
    "local_protection_contract_digest", "decision_deadline_ms",
];
const RESOURCE_FIELDS: [&str; 10] = [
    "training_sample_count", "training_source_population_ref", "split_receipt_ref",
    "offline_compute_seconds", "offline_memory_mb", "hyperparameter_trials",
    "engineer_hours", "online_cpu_ms_p95", "online_memory_mb",
    "communication_bytes_per_decision",
];

fn strv<'a>(v: &'a Value, k: &str) -> Option<&'a str> {
    v.get(k).and_then(Value::as_str)
}
fn reqstr(v: &Value, k: &str, c: &str) -> Result<String, String> {
    strv(v, k).filter(|s| !s.trim().is_empty()).map(str::to_owned)
        .ok_or_else(|| format!("{c}: missing or invalid {k}"))
}
fn one_of(v: &str, choices: &[&str]) -> bool { choices.contains(&v) }

fn schema_check(schema: &Value) -> Result<(), String> {
    if strv(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema")
        || strv(schema, "type") != Some("object")
        || schema.get("additionalProperties").and_then(Value::as_bool) != Some(false) {
        return Err("schema must be a closed Draft 2020-12 object root".to_owned());
    }
    for (path, fields) in [
        (&["required"][..], &["schema_version","corpus_id","profile","pairs","interpretation_rules","claim_ceiling"][..]),
        (&["properties","pairs","items","required"][..], &["pair_id","protocol_revision","shared_deployment_inputs","scoring_profile","baseline_runs","expected_pair_disposition"][..]),
        (&["properties","pairs","items","properties","baseline_runs","items","required"][..], &["baseline_id","baseline_version_ref","configuration_digest","implementation_artifact_digest","deployment_inputs","resource_budget","execution_environment_ref","solver_or_runtime_version","data_population_ref","split_receipt_ref","future_covariate_access"][..])
    ] {
        let mut cur=schema;
        for key in path { cur=cur.get(*key).ok_or_else(||format!("schema missing path {}",path.join(".")))?; }
        let required:BTreeSet<&str>=cur.as_array().into_iter().flatten().filter_map(Value::as_str).collect();
        for field in fields { if !required.contains(field) { return Err(format!("schema must require {}",field)); } }
    }
    Ok(())
}

fn validate_deployment(v: &Value, c: &str) -> Vec<String> {
    let mut e = Vec::new();
    let Some(object) = v.as_object() else {
        return vec![format!("{c}: deployment_inputs must be an object")];
    };
    let mut allowed: BTreeSet<&str> = DEPLOYMENT_FIELDS.iter().copied().collect();
    allowed.insert("future_covariates_available");
    if object.len() != allowed.len() || object.keys().any(|key| !allowed.contains(key.as_str())) {
        e.push(format!("{c}: deployment object must contain exactly the frozen parity fields"));
    }
    for key in DEPLOYMENT_FIELDS {
        if !object.contains_key(key) {
            e.push(format!("{c}: missing deployment field {key}"));
        }
    }
    for key in [
        "scenario_ref",
        "network_id",
        "topology_revision",
        "topology_digest",
        "geometry_revision",
        "geometry_digest",
        "demand_trace_digest",
        "disturbance_trace_digest",
        "missingness_profile_ref",
        "time_grid_ref",
        "service_floor_profile_digest",
        "actuator_envelope_digest",
        "local_protection_contract_digest",
    ] {
        if !object
            .get(key)
            .and_then(Value::as_str)
            .is_some_and(|value| !value.trim().is_empty())
        {
            e.push(format!("{c}: {key} must be a non-empty string"));
        }
    }
    if object.get("random_seed").and_then(Value::as_u64).is_none() {
        e.push(format!("{c}: random_seed must be a non-negative integer"));
    }
    for key in [
        "sampling_period_seconds",
        "scoring_horizon_seconds",
        "decision_deadline_ms",
    ] {
        if !object.get(key).and_then(Value::as_u64).is_some_and(|value| value > 0) {
            e.push(format!("{c}: {key} must be a positive integer"));
        }
    }
    if strv(v, "unit_system") != Some("SI") {
        e.push(format!("{c}: unit system must be SI"));
    }
    if v.get("future_covariates_available").and_then(Value::as_bool) != Some(false) {
        e.push(format!("{c}: future covariates must be unavailable"));
    }
    for (key, description) in [
        ("sensor_set", "sensor set"),
        ("required_service_area_ids", "required areas"),
    ] {
        match v.get(key).and_then(Value::as_array) {
            Some(items) => {
                let ids: Vec<&str> = items.iter().filter_map(Value::as_str).collect();
                if ids.len() != items.len()
                    || ids.is_empty()
                    || ids.iter().copied().collect::<BTreeSet<_>>().len() != ids.len()
                {
                    e.push(format!(
                        "{c}: {description} must be non-empty, string-valued and unique"
                    ));
                }
            }
            None => e.push(format!("{c}: {description} must be an array")),
        }
    }
    e
}

fn validate_scoring(v:&Value,c:&str)->Vec<String>{
    let mut e=Vec::new();
    for k in ["scoring_profile_ref","event_metric_version","service_metric_version","uncertainty_metric_version","resource_metric_version"]{
        if reqstr(v,k,c).is_err(){e.push(format!("{c}: missing scoring field {k}"));}
    }
    if strv(v,"aggregation_policy")!=Some("no_single_compensatory_score")
        ||v.get("abstention_reported").and_then(Value::as_bool)!=Some(true)
        ||v.get("coverage_reported").and_then(Value::as_bool)!=Some(true){
        e.push(format!("{c}: scoring must report non-compensatory lanes, abstention and coverage"));
    }
    e
}

fn validate_budget(v:&Value,c:&str)->Vec<String>{
    let mut e=Vec::new();
    for k in RESOURCE_FIELDS{if v.as_object().is_none_or(|o|!o.contains_key(k)){e.push(format!("{c}: missing resource budget field {k}"));}}
    let samples=v.get("training_sample_count").and_then(Value::as_u64);
    if samples.is_none(){e.push(format!("{c}: training_sample_count must be an unsigned integer"));}
    for k in ["offline_compute_seconds","offline_memory_mb","engineer_hours","online_cpu_ms_p95","online_memory_mb"]{
        if !v.get(k).and_then(Value::as_f64).is_some_and(|n|n.is_finite()&&n>=0.0){e.push(format!("{c}: {k} must be finite and non-negative"));}
    }
    for k in ["hyperparameter_trials","communication_bytes_per_decision"]{
        if v.get(k).and_then(Value::as_u64).is_none(){e.push(format!("{c}: {k} must be a non-negative integer"));}
    }
    let source=strv(v,"training_source_population_ref").unwrap_or("");
    let split=strv(v,"split_receipt_ref").unwrap_or("");
    match samples{
        Some(0) if source!="NOT_APPLICABLE"||split!="NOT_APPLICABLE"=>e.push(format!("{c}: zero-training baseline must mark source/split NOT_APPLICABLE")),
        Some(n) if n>0 && (source.is_empty()||source=="NOT_APPLICABLE"||split.is_empty()||split=="NOT_APPLICABLE")=>e.push(format!("{c}: trained baseline requires source population and split receipt")),
        _=>{}
    }
    e
}

fn eval_pair(pair:&Value)->Value{
    let id=strv(pair,"pair_id").unwrap_or("INVALID").to_owned();
    let c=id.clone(); let mut reasons=Vec::new();
    if strv(pair,"protocol_revision")!=Some(PROTOCOL){reasons.push("protocol revision mismatch".to_owned());}
    if strv(pair,"evidence_class")!=Some("synthetic_design_fixture"){reasons.push("evidence class mismatch".to_owned());}
    let shared=pair.get("shared_deployment_inputs").unwrap_or(&Value::Null);
    reasons.extend(validate_deployment(shared,&format!("{c}/shared")));
    reasons.extend(validate_scoring(pair.get("scoring_profile").unwrap_or(&Value::Null),&format!("{c}/scoring")));
    let runs=pair.get("baseline_runs").and_then(Value::as_array);
    let Some(runs)=runs else{return json!({"pair_id":id,"pair_disposition":"reject","baseline_count":0,"baseline_ids":[],"resource_budget_disclosed":false,"mismatch_reasons":["baseline_runs must be an array"],"evidence_ceiling":"synthetic_design_only"});};
    if runs.len()<2{reasons.push("paired comparison requires at least two baseline runs".to_owned());}
    let mut ids=BTreeSet::new(); let mut all_budgets=true;
    for (i,run) in runs.iter().enumerate(){
        let rc=format!("{c}/baseline[{i}]");
        match reqstr(run,"baseline_id",&rc){Ok(id)=>if !ids.insert(id.clone()){reasons.push(format!("{rc}: duplicate baseline ID {id}"));},Err(e)=>reasons.push(e)}
        for k in ["baseline_version_ref","implementation_artifact_digest","execution_environment_ref","solver_or_runtime_version","data_population_ref","split_receipt_ref"]{
            if reqstr(run,k,&rc).is_err(){reasons.push(format!("{rc}: missing provenance field {k}"));}
        }
        if strv(run,"configuration_digest").is_none_or(str::is_empty){reasons.push(format!("{rc}: configuration digest missing"));}
        let received=run.get("deployment_inputs").unwrap_or(&Value::Null);
        if received!=shared{reasons.push(format!("{rc}: deployment input parity mismatch"));}
        reasons.extend(validate_deployment(received,&format!("{rc}/deployment")));
        if run.get("future_covariate_access").and_then(Value::as_bool)!=Some(false)
            ||received.get("future_covariates_available").and_then(Value::as_bool)!=Some(false){
            reasons.push(format!("{rc}: future covariate access is prohibited"));
        }
        let budget=run.get("resource_budget").unwrap_or(&Value::Null);
        let budget_errors=validate_budget(budget,&format!("{rc}/resource_budget"));
        if !budget_errors.is_empty(){all_budgets=false;reasons.extend(budget_errors);}
        if strv(run,"split_receipt_ref")!=strv(budget,"split_receipt_ref"){reasons.push(format!("{rc}: top-level and nested split receipt differ"));}
    }
    let ids:Vec<String>=ids.into_iter().collect();
    let disposition=if reasons.is_empty(){"admit"}else{"reject"};
    json!({"pair_id":id,"pair_disposition":disposition,"baseline_count":runs.len(),"baseline_ids":ids,"resource_budget_disclosed":all_budgets,"mismatch_reasons":reasons,"evidence_ceiling":"synthetic_design_only"})
}

pub fn evaluate(corpus:&Value,schema:&Value,oracle:&Value)->Result<Value,Vec<String>>{
    let mut errors=Vec::new();
    if let Err(e)=schema_check(schema){errors.push(e);}
    if strv(corpus,"schema_version")!=Some("0.1.0")||strv(corpus,"corpus_id")!=Some(CORPUS){errors.push("corpus identity/version mismatch".to_owned());}
    if strv(oracle,"schema_version")!=Some("0.1.0")||strv(oracle,"oracle_id")!=Some(ORACLE)||strv(oracle,"corpus_id")!=Some(CORPUS){errors.push("oracle identity/version mismatch".to_owned());}
    let profile=corpus.get("profile").unwrap_or(&Value::Null);
    if strv(profile,"profile_id")!=Some("SYNTHETIC-MATCHED-EXPERIMENT-PROFILE-V0")
        ||strv(profile,"evidence_class")!=Some("synthetic_design_profile")
        ||strv(profile,"development_budget_policy")!=Some("record_per_baseline_do_not_hide_or_force_equality")
        ||profile.get("minimum_baseline_count").and_then(Value::as_u64)!=Some(2)
        ||profile.get("values_are_normative").and_then(Value::as_bool)!=Some(false){
        errors.push("matched-experiment profile identity/policy mismatch".to_owned());
    }
    let declared: BTreeSet<&str>=profile.get("deployment_parity_fields").and_then(Value::as_array).into_iter().flatten().filter_map(Value::as_str).collect();
    if declared.len()!=DEPLOYMENT_FIELDS.len()||DEPLOYMENT_FIELDS.iter().any(|x|!declared.contains(x)){errors.push("profile deployment parity field set mismatch".to_owned());}
    let pairs=corpus.get("pairs").and_then(Value::as_array);
    let rows=oracle.get("results").and_then(Value::as_array);
    let (Some(pairs),Some(rows))=(pairs,rows)else{return Err(vec!["pairs/results must be arrays".to_owned()]);};
    if pairs.len()!=COUNT||rows.len()!=COUNT{errors.push(format!("expected {COUNT} pairs and oracle rows"));}
    let mut oracle_map:BTreeMap<String,&Value>=BTreeMap::new();
    for row in rows{
        let Some(id)=strv(row,"pair_id")else{errors.push("oracle pair_id missing".to_owned());continue;};
        if oracle_map.insert(id.to_owned(),row).is_some(){errors.push(format!("duplicate oracle pair {id}"));}
        if row.get("deployment_parity_must_be_exact").and_then(Value::as_bool)!=Some(true)
            ||row.get("development_budget_must_be_disclosed").and_then(Value::as_bool)!=Some(true)
            ||strv(row,"evidence_ceiling")!=Some("synthetic_design_only"){
            errors.push(format!("{id}: oracle parity/resource/claim contract changed"));
        }
    }
    let mut seen=BTreeSet::new(); let mut outputs=Vec::new();
    for pair in pairs{
        let Some(id)=strv(pair,"pair_id")else{errors.push("pair_id missing".to_owned());continue;};
        if !id.starts_with("MEC-")||!seen.insert(id.to_owned()){errors.push(format!("{id}: invalid or duplicate pair ID"));}
        let result=eval_pair(pair);
        let disposition=strv(&result,"pair_disposition").unwrap_or("reject");
        let expected=strv(pair,"expected_pair_disposition").unwrap_or("");
        if !is_one_of(expected,&["admit","reject"]){errors.push(format!("{id}: invalid expected disposition"));}
        if disposition!=expected{errors.push(format!("{id}: expected {expected}, recomputed {disposition}"));}
        match oracle_map.get(id){
            Some(row) if strv(row,"expected_pair_disposition")==Some(disposition)=>{},
            Some(_)=>errors.push(format!("{id}: disposition differs from separate oracle")),
            None=>errors.push(format!("{id}: missing from separate oracle"))
        }
        outputs.push(result);
    }
    if seen.len()!=COUNT||seen.len()!=oracle_map.len()||oracle_map.keys().any(|id|!seen.contains(id)){errors.push("pair/oracle ID sets differ".to_owned());}
    if !strv(corpus,"claim_ceiling").is_some_and(|v|v.to_ascii_lowercase().contains("synthetic")){errors.push("claim ceiling must remain synthetic".to_owned());}
    if errors.is_empty(){
        let admitted=outputs.iter().filter(|x|x["pair_disposition"]=="admit").count();
        Ok(json!({"schema_version":"water-matched-experiment-evaluation-v0.1","verification_state":"Matched","qualification_status":"NotRun","pair_count":pairs.len(),"admitted_pair_count":admitted,"rejected_pair_count":outputs.len()-admitted,"results":outputs,"evidence_class":"synthetic_pair_admission_semantics_only","claim_ceiling":"deployment_input_parity_and_resource_disclosure_only_no_performance_ranking","nonclaims":["No algorithm or commercial platform performance is measured.","No physical network, utility or service result is qualified.","Admitted pairs are eligible for later scoring only; they do not establish superiority."]}))
    }else{Err(errors)}
}

#[cfg(test)]
mod tests {
 use super::*;
 const CORPUS:&str=include_str!("../../../../research/water-service-engineering/matched-experiments/pairs-v0.1.json");
 const SCHEMA:&str=include_str!("../../../../research/water-service-engineering/matched-experiments/pairs-v0.1.schema.json");
 const ORACLE:&str=include_str!("../../../../research/water-service-engineering/matched-experiments/admission-oracle-v0.1.json");
 fn fixtures()->(Value,Value,Value){(serde_json::from_str(CORPUS).unwrap(),serde_json::from_str(SCHEMA).unwrap(),serde_json::from_str(ORACLE).unwrap())}
 #[test] fn ten_pair_corpus_matches_oracle(){let(c,s,o)=fixtures();let r=evaluate(&c,&s,&o).unwrap();assert_eq!(r["pair_count"],10);assert_eq!(r["admitted_pair_count"],1);assert_eq!(r["rejected_pair_count"],9);}
 #[test] fn unequal_development_budgets_are_allowed_when_disclosed(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][0]["pair_disposition"],"admit");}
 #[test] fn sensor_mismatch_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][1]["pair_disposition"],"reject");}
 #[test] fn sampling_mismatch_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][2]["pair_disposition"],"reject");}
 #[test] fn disturbance_mismatch_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][3]["pair_disposition"],"reject");}
 #[test] fn floor_mismatch_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][4]["pair_disposition"],"reject");}
 #[test] fn actuator_mismatch_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][5]["pair_disposition"],"reject");}
 #[test] fn horizon_mismatch_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][6]["pair_disposition"],"reject");}
 #[test] fn future_covariates_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][7]["pair_disposition"],"reject");}
 #[test] fn one_baseline_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][8]["pair_disposition"],"reject");}
 #[test] fn missing_configuration_rejected(){let(c,s,o)=fixtures();assert_eq!(evaluate(&c,&s,&o).unwrap()["results"][9]["pair_disposition"],"reject");}
 #[test] fn duplicate_pair_ids_rejected(){let(mut c,s,o)=fixtures();c["pairs"][1]["pair_id"]=c["pairs"][0]["pair_id"].clone();let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("invalid or duplicate pair ID")));}
 #[test] fn missing_resource_field_rejected(){let(mut c,s,o)=fixtures();c["pairs"][0]["baseline_runs"][0]["resource_budget"].as_object_mut().unwrap().remove("engineer_hours");let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("missing resource budget field engineer_hours")));}
 #[test] fn trained_baseline_requires_split_receipt(){let(mut c,s,o)=fixtures();c["pairs"][0]["baseline_runs"][1]["resource_budget"]["split_receipt_ref"]=json!("NOT_APPLICABLE");c["pairs"][0]["baseline_runs"][1]["split_receipt_ref"]=json!("NOT_APPLICABLE");let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("trained baseline requires source population and split receipt")));}
 #[test] fn deployment_object_must_equal_shared_reference(){let(mut c,s,o)=fixtures();c["pairs"][0]["baseline_runs"][1]["deployment_inputs"]["sensor_set"]=json!(["FLOW-01"]);let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("deployment input parity mismatch")));}
 #[test] fn nested_and_top_level_split_receipts_must_match(){let(mut c,s,o)=fixtures();c["pairs"][0]["baseline_runs"][1]["split_receipt_ref"]=json!("OTHER-SPLIT");let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("split receipt differ")));}
 #[test] fn future_covariate_override_is_rejected(){let(mut c,s,o)=fixtures();c["pairs"][0]["baseline_runs"][0]["future_covariate_access"]=json!(true);let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("future covariate access is prohibited")));}
 #[test] fn scoring_must_report_coverage_and_abstention(){let(mut c,s,o)=fixtures();c["pairs"][0]["scoring_profile"]["abstention_reported"]=json!(false);let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("abstention")));}
 #[test] fn modified_oracle_is_rejected(){let(c,s,mut o)=fixtures();o["results"][0]["expected_pair_disposition"]=json!("reject");let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("differs from separate oracle")));}
 #[test] fn trained_baseline_requires_population_reference(){let(mut c,s,o)=fixtures();c["pairs"][0]["baseline_runs"][1]["resource_budget"]["training_source_population_ref"]=json!("NOT_APPLICABLE");let e=evaluate(&c,&s,&o).unwrap_err();assert!(e.iter().any(|x|x.contains("trained baseline requires source population")));}
    #[test]
    fn shared_deployment_inputs_reject_unregistered_extra_fields() {
        let (mut corpus, schema, oracle) = fixtures();
        corpus["pairs"][0]["shared_deployment_inputs"]["future_demand_forecast"] = json!(0.5);
        corpus["pairs"][0]["baseline_runs"][0]["deployment_inputs"]["future_demand_forecast"] = json!(0.5);
        corpus["pairs"][0]["baseline_runs"][1]["deployment_inputs"]["future_demand_forecast"] = json!(0.5);
        let errors = evaluate(&corpus, &schema, &oracle)
            .expect_err("unregistered future field must be rejected even when shared");
        assert!(errors.iter().any(|error| error.contains("exactly the frozen parity fields")));
    }

    #[test]
    fn invalid_seed_and_zero_sampling_period_are_rejected() {
        let (mut corpus, schema, oracle) = fixtures();
        corpus["pairs"][0]["shared_deployment_inputs"]["random_seed"] = json!(-1);
        corpus["pairs"][0]["shared_deployment_inputs"]["sampling_period_seconds"] = json!(0);
        for run in corpus["pairs"][0]["baseline_runs"].as_array_mut().unwrap() {
            run["deployment_inputs"]["random_seed"] = json!(-1);
            run["deployment_inputs"]["sampling_period_seconds"] = json!(0);
        }
        let errors = evaluate(&corpus, &schema, &oracle)
            .expect_err("invalid common timing/seed fields must be rejected");
        assert!(errors.iter().any(|error| error.contains("random_seed")));
        assert!(errors.iter().any(|error| error.contains("sampling_period_seconds")));
    }

}