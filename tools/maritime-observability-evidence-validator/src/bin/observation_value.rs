// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only expected value of sample information for already-available
//! maritime diagnostic observations. It never initiates a measurement,
//! controller excitation, actuation, maintenance action or mode transition.
#![forbid(unsafe_code)]
use serde_json::{json, Value};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";
const FIXTURE: &str = include_str!("../../../../benchmarks/maritime-observability/observation-value-fixture.v0.1.json");

type Errors = Vec<Value>;
type Probabilities = BTreeMap<String, f64>;
type Decision = (String, Probabilities);

fn text<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str).map(str::trim).filter(|s| !s.is_empty())
}
fn finite(value: &Value) -> Option<f64> {
    value.as_f64().filter(|number| number.is_finite())
}
fn sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}
fn push(errors: &mut Errors, code: &str, path: &str, detail: impl Into<String>) {
    errors.push(json!({"code":code,"path":path,"detail":detail.into()}));
}
fn keys(value: &Value, required: &[&str], path: &str, errors: &mut Errors) {
    let Some(map) = value.as_object() else {
        push(errors,"EXPECTED_OBJECT",path,"expected object");
        return;
    };
    let allowed: BTreeSet<&str> = required.iter().copied().collect();
    for key in required {
        if !map.contains_key(*key) {
            push(errors,"MISSING_KEY",&format!("{path}.{key}"),"required key missing");
        }
    }
    for key in map.keys() {
        if !allowed.contains(key.as_str()) {
            push(errors,"UNKNOWN_KEY",&format!("{path}.{key}"),"unknown key rejected");
        }
    }
}
fn string_set(value: &Value, key: &str, path: &str, errors: &mut Errors) -> BTreeSet<String> {
    let mut output = BTreeSet::new();
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        push(errors,"EXPECTED_ARRAY",&format!("{path}.{key}"),"expected array");
        return output;
    };
    for (index,item) in items.iter().enumerate() {
        let Some(item) = item.as_str().filter(|item| !item.trim().is_empty()) else {
            push(errors,"INVALID_ARRAY_ITEM",&format!("{path}.{key}[{index}]"),"expected non-empty string");
            continue;
        };
        if !output.insert(item.to_owned()) {
            push(errors,"DUPLICATE_ARRAY_ITEM",&format!("{path}.{key}[{index}]"),"duplicate item");
        }
    }
    output
}
fn best_decision(priors: &Probabilities, decisions: &[Decision]) -> (String, f64) {
    let mut best_id = String::new();
    let mut best_risk = f64::INFINITY;
    for (decision_id,losses) in decisions {
        let risk = priors.iter()
            .map(|(hypothesis,probability)| probability * losses[hypothesis])
            .sum::<f64>();
        if risk < best_risk {
            best_id = decision_id.clone();
            best_risk = risk;
        }
    }
    (best_id,best_risk)
}
fn normalize_sum(values: impl Iterator<Item=f64>) -> f64 {
    values.sum::<f64>()
}

fn evaluate(input: &Value) -> Result<Value,Errors> {
    let mut errors = Vec::new();
    keys(input,&[
        "schema_version","assessment_id","evidence_class","hypotheses","decisions",
        "existing_evidence_dependency_groups","candidates","authority_boundary","claim_ceiling"
    ],"$",&mut errors);
    if text(input,"schema_version") != Some(VERSION) {
        push(&mut errors,"SCHEMA_VERSION","$.schema_version","expected 0.1.0");
    }
    if text(input,"evidence_class") != Some("SYNTHETIC_KNOWN_TRUTH") {
        push(&mut errors,"EVIDENCE_CLASS","$.evidence_class","P0 evaluator accepts explicitly synthetic known-truth fixtures only");
    }
    for key in ["assessment_id","claim_ceiling"] {
        if text(input,key).is_none() {
            push(&mut errors,"REQUIRED_TEXT",&format!("$.{key}"),"non-empty string required");
        }
    }

    let boundary = &input["authority_boundary"];
    keys(boundary,&[
        "read_only","new_measurement_initiated","controller_write","actuation",
        "maintenance_authorization","mode_transition","restriction_clearance"
    ],"$.authority_boundary",&mut errors);
    if boundary["read_only"].as_bool() != Some(true) {
        push(&mut errors,"READ_ONLY_REQUIRED","$.authority_boundary.read_only","must be true");
    }
    for field in ["new_measurement_initiated","controller_write","actuation","maintenance_authorization","mode_transition","restriction_clearance"] {
        if boundary[field].as_bool() != Some(false) {
            push(&mut errors,"AUTHORITY_BOUNDARY_VIOLATION",&format!("$.authority_boundary.{field}"),"must be false");
        }
    }

    let hypothesis_values = input["hypotheses"].as_array().cloned().unwrap_or_default();
    if hypothesis_values.len() < 2 || hypothesis_values.len() > 100 {
        push(&mut errors,"HYPOTHESIS_COUNT","$.hypotheses","expected 2..=100 hypotheses");
    }
    let mut priors: Probabilities = BTreeMap::new();
    for (index,hypothesis) in hypothesis_values.iter().enumerate() {
        let path = format!("$.hypotheses[{index}]");
        keys(hypothesis,&["hypothesis_id","prior_probability"],&path,&mut errors);
        let Some(id) = text(hypothesis,"hypothesis_id") else {
            push(&mut errors,"HYPOTHESIS_ID_REQUIRED",&path,"hypothesis ID required");
            continue;
        };
        let Some(probability) = finite(&hypothesis["prior_probability"]) else {
            push(&mut errors,"PRIOR_INVALID",&format!("{path}.prior_probability"),"finite prior probability required");
            continue;
        };
        if !(0.0..=1.0).contains(&probability) {
            push(&mut errors,"PRIOR_OUT_OF_RANGE",&format!("{path}.prior_probability"),"prior must be in [0,1]");
        }
        if priors.insert(id.to_owned(),probability).is_some() {
            push(&mut errors,"DUPLICATE_HYPOTHESIS_ID",&format!("{path}.hypothesis_id"),"hypothesis IDs must be unique");
        }
    }
    if (normalize_sum(priors.values().copied()) - 1.0).abs() > 1e-9 {
        push(&mut errors,"PRIOR_NORMALIZATION","$.hypotheses","prior probabilities must sum to one within 1e-9");
    }
    let hypothesis_ids: BTreeSet<String> = priors.keys().cloned().collect();

    let decision_values = input["decisions"].as_array().cloned().unwrap_or_default();
    if decision_values.is_empty() || decision_values.len() > 100 {
        push(&mut errors,"DECISION_COUNT","$.decisions","expected 1..=100 decisions");
    }
    let mut decision_ids = BTreeSet::new();
    let mut decisions: Vec<Decision> = Vec::new();
    for (index,decision) in decision_values.iter().enumerate() {
        let path = format!("$.decisions[{index}]");
        keys(decision,&["decision_id","losses"],&path,&mut errors);
        let Some(id) = text(decision,"decision_id") else {
            push(&mut errors,"DECISION_ID_REQUIRED",&path,"decision ID required");
            continue;
        };
        if !decision_ids.insert(id.to_owned()) {
            push(&mut errors,"DUPLICATE_DECISION_ID",&path,"decision IDs must be unique");
        }
        let Some(loss_map) = decision["losses"].as_object() else {
            push(&mut errors,"LOSS_MAP_REQUIRED",&format!("{path}.losses"),"loss map object required");
            continue;
        };
        let actual: BTreeSet<String> = loss_map.keys().cloned().collect();
        if actual != hypothesis_ids {
            push(&mut errors,"LOSS_HYPOTHESIS_SET_MISMATCH",&format!("{path}.losses"),"must declare exactly one loss for each hypothesis");
        }
        let mut losses = BTreeMap::new();
        for (hypothesis,value) in loss_map {
            let Some(loss) = finite(value) else {
                push(&mut errors,"LOSS_INVALID",&format!("{path}.losses.{hypothesis}"),"finite loss required");
                continue;
            };
            if loss < 0.0 {
                push(&mut errors,"LOSS_NEGATIVE",&format!("{path}.losses.{hypothesis}"),"loss must be non-negative");
            }
            losses.insert(hypothesis.clone(),loss);
        }
        decisions.push((id.to_owned(),losses));
    }

    let existing_dependencies = string_set(input,"existing_evidence_dependency_groups","$",&mut errors);
    if existing_dependencies.is_empty() {
        push(&mut errors,"EXISTING_DEPENDENCIES_REQUIRED","$.existing_evidence_dependency_groups","at least one dependency group required");
    }
    let candidates = input["candidates"].as_array().cloned().unwrap_or_default();
    if candidates.is_empty() || candidates.len() > 1000 {
        push(&mut errors,"CANDIDATE_COUNT","$.candidates","expected 1..=1000 candidates");
    }

    struct Candidate {
        id: String,
        eligible: bool,
        reasons: Vec<String>,
        cost: f64,
        likelihoods: BTreeMap<String,Probabilities>,
    }
    let mut parsed_candidates = Vec::new();
    let mut candidate_ids = BTreeSet::new();

    for (index,candidate) in candidates.iter().enumerate() {
        let path = format!("$.candidates[{index}]");
        keys(candidate,&[
            "candidate_id","observation_kind","evidence_role","read_only","already_available",
            "availability_state","measurement_quality_state","sensor_calibration_state",
            "timebase_id","timebase_qualification_state","timebase_qualification_sha256",
            "dependency_groups","likelihood_model_qualification_state","likelihood_model_sha256",
            "acquisition_cost","outcomes"
        ],&path,&mut errors);
        let Some(id) = text(candidate,"candidate_id") else {
            push(&mut errors,"CANDIDATE_ID_REQUIRED",&path,"candidate ID required");
            continue;
        };
        if !candidate_ids.insert(id.to_owned()) {
            push(&mut errors,"DUPLICATE_CANDIDATE_ID",&path,"candidate IDs must be unique");
        }
        if !matches!(text(candidate,"observation_kind"),Some(
            "EXISTING_CHANNEL_REVIEW"|"ARCHIVED_SIGNAL_RETRIEVAL"|"CALIBRATION_RECORD_REVIEW"|"HUMAN_EXPERT_REVIEW"
        )) {
            push(&mut errors,"OBSERVATION_KIND_INVALID",&format!("{path}.observation_kind"),"only passive existing evidence kinds are allowed");
        }
        if candidate["read_only"].as_bool() != Some(true) || candidate["already_available"].as_bool() != Some(true) {
            push(&mut errors,"ACTIVE_OBSERVATION_FORBIDDEN",&path,"candidate must be read-only and already available");
        }
        let evidence_role = text(candidate,"evidence_role").unwrap_or("");
        if !matches!(evidence_role, "MEASURED_EXISTING_RECORD"|"CALIBRATION_RECORD"|"DERIVED"|"SIMULATED") {
            push(&mut errors,"EVIDENCE_ROLE_INVALID",&format!("{path}.evidence_role"),"unsupported evidence role");
        }
        if !matches!(text(candidate,"availability_state"),Some("AVAILABLE"|"MISSING"|"STALE"|"UNKNOWN")) {
            push(&mut errors,"AVAILABILITY_INVALID",&format!("{path}.availability_state"),"unsupported availability state");
        }
        if !matches!(text(candidate,"measurement_quality_state"),Some("CURRENT"|"STALE"|"REJECTED"|"CONFLICTED"|"UNKNOWN")) {
            push(&mut errors,"QUALITY_INVALID",&format!("{path}.measurement_quality_state"),"unsupported measurement quality");
        }
        if !matches!(text(candidate,"sensor_calibration_state"),Some("CURRENT"|"NOT_APPLICABLE"|"EXPIRED"|"UNKNOWN")) {
            push(&mut errors,"CALIBRATION_INVALID",&format!("{path}.sensor_calibration_state"),"unsupported calibration state");
        }
        if !matches!(text(candidate,"timebase_qualification_state"),Some("QUALIFIED"|"UNQUALIFIED"|"UNKNOWN")) {
            push(&mut errors,"TIMEBASE_STATE_INVALID",&format!("{path}.timebase_qualification_state"),"unsupported timebase state");
        }
        if text(candidate,"timebase_id").is_none() || !candidate["timebase_qualification_sha256"].as_str().is_some_and(sha256) {
            push(&mut errors,"TIMEBASE_EVIDENCE_INVALID",&path,"timebase ID and qualification SHA-256 required");
        }
        if !matches!(text(candidate,"likelihood_model_qualification_state"),Some("QUALIFIED"|"UNQUALIFIED"|"UNKNOWN"))
            || !candidate["likelihood_model_sha256"].as_str().is_some_and(sha256) {
            push(&mut errors,"LIKELIHOOD_MODEL_EVIDENCE_INVALID",&path,"qualified likelihood model and SHA-256 required");
        }
        let Some(cost) = finite(&candidate["acquisition_cost"]) else {
            push(&mut errors,"COST_INVALID",&format!("{path}.acquisition_cost"),"finite acquisition/review cost required");
            continue;
        };
        if cost < 0.0 {
            push(&mut errors,"COST_NEGATIVE",&format!("{path}.acquisition_cost"),"cost must be non-negative");
        }
        let dependencies = string_set(candidate,"dependency_groups",&path,&mut errors);
        if dependencies.is_empty() {
            push(&mut errors,"CANDIDATE_DEPENDENCIES_REQUIRED",&path,"at least one dependency group required");
        }

        let outcomes = candidate["outcomes"].as_array().cloned().unwrap_or_default();
        if outcomes.len() < 2 || outcomes.len() > 100 {
            push(&mut errors,"OUTCOME_COUNT",&format!("{path}.outcomes"),"expected 2..=100 outcomes");
        }
        let mut outcome_ids = BTreeSet::new();
        let mut likelihoods: BTreeMap<String,Probabilities> = BTreeMap::new();
        let mut likelihood_sums: Probabilities = hypothesis_ids.iter().map(|id|(id.clone(),0.0)).collect();

        for (outcome_index,outcome) in outcomes.iter().enumerate() {
            let outcome_path = format!("{path}.outcomes[{outcome_index}]");
            keys(outcome,&["outcome_id","likelihoods"],&outcome_path,&mut errors);
            let Some(outcome_id) = text(outcome,"outcome_id") else {
                push(&mut errors,"OUTCOME_ID_REQUIRED",&outcome_path,"outcome ID required");
                continue;
            };
            if !outcome_ids.insert(outcome_id.to_owned()) {
                push(&mut errors,"DUPLICATE_OUTCOME_ID",&outcome_path,"outcome IDs must be unique");
            }
            let Some(map) = outcome["likelihoods"].as_object() else {
                push(&mut errors,"LIKELIHOOD_MAP_REQUIRED",&outcome_path,"likelihood map object required");
                continue;
            };
            let actual: BTreeSet<String> = map.keys().cloned().collect();
            if actual != hypothesis_ids {
                push(&mut errors,"LIKELIHOOD_HYPOTHESIS_SET_MISMATCH",&outcome_path,"likelihoods must name every hypothesis exactly once");
            }
            let mut row = BTreeMap::new();
            for (hypothesis,value) in map {
                let Some(probability) = finite(value) else {
                    push(&mut errors,"LIKELIHOOD_INVALID",&format!("{outcome_path}.likelihoods.{hypothesis}"),"finite likelihood required");
                    continue;
                };
                if !(0.0..=1.0).contains(&probability) {
                    push(&mut errors,"LIKELIHOOD_OUT_OF_RANGE",&format!("{outcome_path}.likelihoods.{hypothesis}"),"likelihood must be in [0,1]");
                }
                *likelihood_sums.entry(hypothesis.clone()).or_default() += probability;
                row.insert(hypothesis.clone(),probability);
            }
            likelihoods.insert(outcome_id.to_owned(),row);
        }
        for (hypothesis,sum) in &likelihood_sums {
            if (sum-1.0).abs() > 1e-9 {
                push(&mut errors,"LIKELIHOOD_NORMALIZATION",&format!("{path}.outcomes"),format!("likelihoods across outcomes for {hypothesis} must sum to one"));
            }
        }

        let mut reasons = BTreeSet::new();
        if text(candidate,"availability_state") != Some("AVAILABLE") { reasons.insert("OBSERVATION_NOT_AVAILABLE"); }
        if text(candidate,"measurement_quality_state") != Some("CURRENT") { reasons.insert("MEASUREMENT_NOT_CURRENT"); }
        if !matches!(text(candidate,"sensor_calibration_state"),Some("CURRENT"|"NOT_APPLICABLE")) { reasons.insert("CALIBRATION_NOT_CURRENT"); }
        if text(candidate,"timebase_qualification_state") != Some("QUALIFIED") { reasons.insert("TIMEBASE_NOT_QUALIFIED"); }
        if text(candidate,"likelihood_model_qualification_state") != Some("QUALIFIED") { reasons.insert("LIKELIHOOD_MODEL_NOT_QUALIFIED"); }
        if !matches!(evidence_role,"MEASURED_EXISTING_RECORD"|"CALIBRATION_RECORD") { reasons.insert("EVIDENCE_NOT_MEASURED_OR_CALIBRATION_RECORD"); }
        if dependencies.iter().any(|dependency| existing_dependencies.contains(dependency)) {
            reasons.insert("SHARED_DEPENDENCY_WITH_EXISTING_EVIDENCE");
        }
        parsed_candidates.push(Candidate{
            id:id.to_owned(),eligible:reasons.is_empty(),reasons:reasons.into_iter().map(str::to_owned).collect(),
            cost,likelihoods
        });
    }

    if !errors.is_empty() {
        return Err(errors);
    }

    let (baseline_decision_id,prior_risk) = best_decision(&priors,&decisions);
    let mut candidate_receipts = Vec::new();
    let mut evaluated_count = 0usize;
    let mut positive_count = 0usize;

    for candidate in parsed_candidates {
        if !candidate.eligible {
            candidate_receipts.push(json!({
                "candidate_id":candidate.id,"status":"NOT_EVALUATED","reason_codes":candidate.reasons,
                "gross_evsi":Value::Null,"acquisition_cost":candidate.cost,"net_evsi":Value::Null
            }));
            continue;
        }

        let mut expected_posterior_risk = 0.0;
        let mut outcome_receipts = Vec::new();
        for (outcome_id,row) in &candidate.likelihoods {
            let predictive_probability = priors.iter()
                .map(|(hypothesis,prior)| prior * row[hypothesis])
                .sum::<f64>();
            if predictive_probability <= 1e-15 {
                outcome_receipts.push(json!({
                    "outcome_id":outcome_id,"predictive_probability":predictive_probability,
                    "posterior_probabilities":{},"best_decision_id":Value::Null,"posterior_bayes_risk":Value::Null
                }));
                continue;
            }
            let posterior: Probabilities = priors.iter()
                .map(|(hypothesis,prior)|(hypothesis.clone(),prior*row[hypothesis]/predictive_probability))
                .collect();
            let (decision_id,posterior_risk) = best_decision(&posterior,&decisions);
            expected_posterior_risk += predictive_probability * posterior_risk;
            outcome_receipts.push(json!({
                "outcome_id":outcome_id,"predictive_probability":predictive_probability,
                "posterior_probabilities":posterior,"best_decision_id":decision_id,
                "posterior_bayes_risk":posterior_risk
            }));
        }
        let raw_gross_evsi = prior_risk - expected_posterior_risk;
        if raw_gross_evsi < -1e-9 {
            return Err(vec![json!({
                "code":"NEGATIVE_GROSS_EVSI","path":format!("$.candidates[{}]",candidate.id),
                "detail":"expected posterior Bayes risk exceeds prior Bayes risk; likelihood model is invalid"
            })]);
        }
        let gross_evsi = raw_gross_evsi.max(0.0);
        let net_evsi = gross_evsi - candidate.cost;
        let status = if net_evsi > 1e-12 { positive_count += 1; "POSITIVE_NET_VALUE" } else { "NON_POSITIVE_NET_VALUE" };
        evaluated_count += 1;
        candidate_receipts.push(json!({
            "candidate_id":candidate.id,"status":status,"reason_codes":[],
            "prior_bayes_risk":prior_risk,"expected_posterior_bayes_risk":expected_posterior_risk,
            "gross_evsi":gross_evsi,"acquisition_cost":candidate.cost,"net_evsi":net_evsi,
            "outcomes":outcome_receipts
        }));
    }
    candidate_receipts.sort_by(|left,right| {
        let left_invalid = left["status"] == "NOT_EVALUATED";
        let right_invalid = right["status"] == "NOT_EVALUATED";
        match (left_invalid,right_invalid) {
            (false,true) => std::cmp::Ordering::Less,
            (true,false) => std::cmp::Ordering::Greater,
            (false,false) => right["net_evsi"].as_f64().unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&left["net_evsi"].as_f64().unwrap_or(f64::NEG_INFINITY))
                .then_with(|| left["candidate_id"].as_str().cmp(&right["candidate_id"].as_str())),
            (true,true) => left["candidate_id"].as_str().cmp(&right["candidate_id"].as_str())
        }
    });
    let not_evaluated_count = candidate_receipts.iter().filter(|item|item["status"]=="NOT_EVALUATED").count();
    Ok(json!({
        "result":"pass","evaluator_version":VERSION,"assessment_id":text(input,"assessment_id").unwrap_or(""),
        "hypothesis_count":priors.len(),"decision_count":decisions.len(),"candidate_count":candidates.len(),
        "baseline_decision_id":baseline_decision_id,"prior_bayes_risk":prior_risk,
        "evaluated_candidate_count":evaluated_count,"not_evaluated_candidate_count":not_evaluated_count,
        "positive_net_value_candidate_count":positive_count,"ranked_candidates":candidate_receipts,
        "mutation_performed":false,"authorization_granted":false,
        "claim_ceiling":text(input,"claim_ceiling").unwrap_or("")
    }))
}
fn run(path:&str,source:&str,digest:&str)->Result<Value,String>{
    if source.len()!=40 || !source.bytes().all(|byte|byte.is_ascii_hexdigit()) || !sha256(digest) {
        return Err("invalid source commit or case SHA-256".to_owned());
    }
    let raw=fs::read_to_string(path).map_err(|error|format!("read case: {error}"))?;
    let input:Value=serde_json::from_str(&raw).map_err(|error|format!("parse case: {error}"))?;
    match evaluate(&input) {
        Ok(mut receipt) => {
            receipt["source_commit"]=json!(source);
            receipt["case_sha256"]=json!(digest);
            Ok(receipt)
        }
        Err(errors) => Err(serde_json::to_string(&json!({"violations":errors}))
            .unwrap_or_else(|_|"EVSI evaluation failed".to_owned()))
    }
}
fn main()->ExitCode{
    let args:Vec<String>=env::args().collect();
    if args.len()!=6 || args[2]!="--source-commit" || args[4]!="--case-sha256" {
        eprintln!("usage: observation-value <case.json> --source-commit <40-hex> --case-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    match run(&args[1],&args[3],&args[5]) {
        Ok(receipt) => {
            println!("{}",serde_json::to_string_pretty(&receipt).unwrap_or_else(|_|"{\"result\":\"fail\"}".to_owned()));
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("observation value evaluation failed closed: {error}");
            ExitCode::FAILURE
        }
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    fn fixture()->Value{serde_json::from_str(FIXTURE).expect("EVSI fixture parses")}
    #[test]
    fn frozen_fixture_is_valid_and_non_mutating(){
        let report=evaluate(&fixture()).unwrap();
        assert_eq!(report["candidate_count"],4);
        assert_eq!(report["evaluated_candidate_count"],2);
        assert_eq!(report["not_evaluated_candidate_count"],2);
        assert_eq!(report["positive_net_value_candidate_count"],1);
        assert_eq!(report["mutation_performed"],false);
        assert_eq!(report["authorization_granted"],false);
    }
    #[test]
    fn positive_evsi_candidate_ranks_first(){
        let report=evaluate(&fixture()).unwrap();
        assert_eq!(report["ranked_candidates"][0]["candidate_id"],"needle-lift-existing-trace");
        assert_eq!(report["ranked_candidates"][0]["status"],"POSITIVE_NET_VALUE");
        let net=report["ranked_candidates"][0]["net_evsi"].as_f64().unwrap();
        assert!((net-0.0295).abs()<1e-9);
    }
    #[test]
    fn non_positive_candidate_is_not_recommended(){
        let report=evaluate(&fixture()).unwrap();
        assert_eq!(report["ranked_candidates"][1]["candidate_id"],"independent-exhaust-temperature-record");
        assert_eq!(report["ranked_candidates"][1]["status"],"NON_POSITIVE_NET_VALUE");
    }
    #[test]
    fn derived_or_shared_evidence_is_not_scored(){
        let report=evaluate(&fixture()).unwrap();
        let derived=report["ranked_candidates"].iter().find(|item|item["candidate_id"]=="derived-load-estimate").unwrap();
        assert_eq!(derived["status"],"NOT_EVALUATED");
        assert!(derived["reason_codes"].as_array().unwrap().iter().any(|reason|reason=="SHARED_DEPENDENCY_WITH_EXISTING_EVIDENCE"));
        assert!(derived["reason_codes"].as_array().unwrap().iter().any(|reason|reason=="EVIDENCE_NOT_MEASURED_OR_CALIBRATION_RECORD"));
    }
    #[test]
    fn stale_or_expired_record_is_not_scored(){
        let report=evaluate(&fixture()).unwrap();
        let stale=report["ranked_candidates"].iter().find(|item|item["candidate_id"]=="stale-thermocouple-record").unwrap();
        assert_eq!(stale["status"],"NOT_EVALUATED");
    }
    #[test]
    fn active_excitation_is_rejected(){
        let mut input=fixture();
        input["candidates"][0]["observation_kind"]=json!("ACTIVE_EXCITATION");
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="OBSERVATION_KIND_INVALID"));
        input=fixture();
        input["candidates"][0]["read_only"]=json!(false);
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="ACTIVE_OBSERVATION_FORBIDDEN"));
    }
    #[test]
    fn prior_probabilities_must_normalize(){
        let mut input=fixture();
        input["hypotheses"][0]["prior_probability"]=json!(0.9);
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="PRIOR_NORMALIZATION"));
    }
    #[test]
    fn likelihoods_must_normalize_for_each_hypothesis(){
        let mut input=fixture();
        input["candidates"][0]["outcomes"][0]["likelihoods"]["FUEL_RESTRICTION"]=json!(0.99);
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="LIKELIHOOD_NORMALIZATION"));
    }
    #[test]
    fn authority_boundary_cannot_be_weakened(){
        let mut input=fixture();
        input["authority_boundary"]["controller_write"]=json!(true);
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="AUTHORITY_BOUNDARY_VIOLATION"));
    }
}
