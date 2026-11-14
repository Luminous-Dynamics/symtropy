// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only expected value of sample information for already-available
//! maritime diagnostic observations. It never initiates a measurement,
//! controller excitation, actuation, maintenance action or mode transition.
#![forbid(unsafe_code)]
use maritime_observability_evidence_validator::verify_sha256;
use serde_json::{json, Value};
use std::{collections::{BTreeMap, BTreeSet}, env, fs, process::ExitCode};

const VERSION: &str = "0.1.0";

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

fn score_likelihood_model(
    scenario_id: &str,
    model_sha256: &str,
    priors: &Probabilities,
    decisions: &[Decision],
    likelihoods: &BTreeMap<String,Probabilities>,
    prior_risk: f64,
    cost: f64,
) -> Result<Value,String> {
    let mut expected_posterior_risk = 0.0;
    let mut outcome_receipts = Vec::new();
    for (outcome_id,row) in likelihoods {
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
        let (decision_id,posterior_risk) = best_decision(&posterior,decisions);
        expected_posterior_risk += predictive_probability * posterior_risk;
        outcome_receipts.push(json!({
            "outcome_id":outcome_id,"predictive_probability":predictive_probability,
            "posterior_probabilities":posterior,"best_decision_id":decision_id,
            "posterior_bayes_risk":posterior_risk
        }));
    }
    let raw_gross_evsi = prior_risk - expected_posterior_risk;
    if raw_gross_evsi < -1e-9 {
        return Err(format!("scenario {scenario_id}: posterior risk exceeds prior risk"));
    }
    let gross_evsi = raw_gross_evsi.max(0.0);
    let net_evsi = gross_evsi - cost;
    Ok(json!({
        "scenario_id":scenario_id,"likelihood_model_sha256":model_sha256,
        "prior_bayes_risk":prior_risk,"expected_posterior_bayes_risk":expected_posterior_risk,
        "gross_evsi":gross_evsi,"acquisition_cost":cost,"net_evsi":net_evsi,
        "outcomes":outcome_receipts
    }))
}

fn evaluate(input: &Value) -> Result<Value,Errors> {
    let mut errors = Vec::new();
    keys(input,&[
        "schema_version","assessment_id","evidence_class","hypotheses","prior_ambiguity_scenarios",
        "decisions","existing_evidence_dependency_groups","candidates","authority_boundary","claim_ceiling"
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

    // Cross nominal/alternative priors with nominal/alternative likelihoods.
    struct PriorScenario {
        id: String,
        model_sha256: String,
        probabilities: Probabilities,
    }
    let prior_values = input["prior_ambiguity_scenarios"].as_array().cloned().unwrap_or_default();
    if prior_values.len() < 2 || prior_values.len() > 20 {
        push(&mut errors,"PRIOR_SCENARIO_COUNT","$.prior_ambiguity_scenarios",
            "expected 2..=20 separately qualified alternative prior distributions");
    }
    let mut prior_scenario_ids = BTreeSet::new();
    let mut prior_scenario_hashes = BTreeSet::new();
    let mut prior_scenarios = Vec::new();
    for (index,scenario) in prior_values.iter().enumerate() {
        let path = format!("$.prior_ambiguity_scenarios[{index}]");
        keys(scenario,&["scenario_id","prior_model_sha256","qualification_state","prior_probabilities"],&path,&mut errors);
        let Some(id) = text(scenario,"scenario_id") else {
            push(&mut errors,"PRIOR_SCENARIO_ID_REQUIRED",&path,"prior scenario ID required");
            continue;
        };
        if !prior_scenario_ids.insert(id.to_owned()) {
            push(&mut errors,"DUPLICATE_PRIOR_SCENARIO_ID",&path,"prior scenario IDs must be unique");
        }
        let Some(digest) = text(scenario,"prior_model_sha256") else {
            push(&mut errors,"PRIOR_SCENARIO_SHA_REQUIRED",&path,"prior model SHA-256 required");
            continue;
        };
        if !sha256(digest) {
            push(&mut errors,"PRIOR_SCENARIO_SHA_INVALID",&path,"expected lowercase SHA-256");
        }
        if !prior_scenario_hashes.insert(digest.to_owned()) {
            push(&mut errors,"DUPLICATE_PRIOR_SCENARIO_SHA",&path,"prior model digests must be unique");
        }
        if text(scenario,"qualification_state") != Some("QUALIFIED") {
            push(&mut errors,"PRIOR_SCENARIO_NOT_QUALIFIED",&path,
                "every prior ambiguity scenario must be independently qualified");
        }
        let Some(probability_map) = scenario["prior_probabilities"].as_object() else {
            push(&mut errors,"PRIOR_SCENARIO_MAP_REQUIRED",&format!("{path}.prior_probabilities"),
                "prior probability map required");
            continue;
        };
        let actual: BTreeSet<String> = probability_map.keys().cloned().collect();
        if actual != hypothesis_ids {
            push(&mut errors,"PRIOR_SCENARIO_HYPOTHESIS_SET_MISMATCH",&path,
                "prior scenario must name every hypothesis exactly once");
        }
        let mut probabilities = BTreeMap::new();
        for (hypothesis,value) in probability_map {
            let Some(probability) = finite(value) else {
                push(&mut errors,"PRIOR_SCENARIO_PROBABILITY_INVALID",
                    &format!("{path}.prior_probabilities.{hypothesis}"),"finite probability required");
                continue;
            };
            if !(0.0..=1.0).contains(&probability) {
                push(&mut errors,"PRIOR_SCENARIO_PROBABILITY_RANGE",
                    &format!("{path}.prior_probabilities.{hypothesis}"),"probability must be in [0,1]");
            }
            probabilities.insert(hypothesis.clone(),probability);
        }
        if (normalize_sum(probabilities.values().copied()) - 1.0).abs() > 1e-9 {
            push(&mut errors,"PRIOR_SCENARIO_NORMALIZATION",&format!("{path}.prior_probabilities"),
                "prior scenario probabilities must sum to one within 1e-9");
        }
        prior_scenarios.push(PriorScenario {
            id:id.to_owned(),model_sha256:digest.to_owned(),probabilities,
        });
    }

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

    struct LikelihoodScenario {
        id: String,
        model_sha256: String,
        likelihoods: BTreeMap<String,Probabilities>,
    }
    struct Candidate {
        id: String,
        eligible: bool,
        reasons: Vec<String>,
        cost: f64,
        nominal_model_sha256: String,
        likelihoods: BTreeMap<String,Probabilities>,
        scenarios: Vec<LikelihoodScenario>,
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
            "acquisition_cost","outcomes","likelihood_scenarios"
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
        if !matches!(evidence_role, "MEASURED_EXISTING_RECORD"|"CALIBRATION_RECORD"|"DERIVED"|"RECONSTRUCTED"|"SIMULATED") {
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
        // Require a finite set of separately qualified, digest-bound alternatives.
        let scenario_values = candidate["likelihood_scenarios"].as_array().cloned().unwrap_or_default();
        if scenario_values.len() < 2 || scenario_values.len() > 20 {
            push(&mut errors,"LIKELIHOOD_SCENARIO_COUNT",&format!("{path}.likelihood_scenarios"),"expected 2..=20 alternative models");
        }
        let mut scenario_ids = BTreeSet::new();
        let mut scenario_hashes = BTreeSet::new();
        let mut parsed_scenarios = Vec::new();
        for (scenario_index,scenario) in scenario_values.iter().enumerate() {
            let scenario_path = format!("{path}.likelihood_scenarios[{scenario_index}]");
            keys(scenario,&["scenario_id","likelihood_model_sha256","qualification_state","outcomes"],&scenario_path,&mut errors);
            let Some(scenario_id) = text(scenario,"scenario_id") else {
                push(&mut errors,"LIKELIHOOD_SCENARIO_ID_REQUIRED",&scenario_path,"scenario ID required");
                continue;
            };
            if !scenario_ids.insert(scenario_id.to_owned()) {
                push(&mut errors,"DUPLICATE_LIKELIHOOD_SCENARIO_ID",&scenario_path,"scenario IDs must be unique");
            }
            let Some(scenario_sha) = text(scenario,"likelihood_model_sha256") else {
                push(&mut errors,"LIKELIHOOD_SCENARIO_SHA_REQUIRED",&scenario_path,"scenario SHA-256 required");
                continue;
            };
            if !sha256(scenario_sha) {
                push(&mut errors,"LIKELIHOOD_SCENARIO_SHA_INVALID",&scenario_path,"expected lowercase SHA-256");
            }
            if !scenario_hashes.insert(scenario_sha.to_owned()) {
                push(&mut errors,"DUPLICATE_LIKELIHOOD_SCENARIO_SHA",&scenario_path,"scenario model digests must be unique");
            }
            if text(scenario,"qualification_state") != Some("QUALIFIED") {
                reasons.insert("LIKELIHOOD_SCENARIO_NOT_QUALIFIED");
            }
            let scenario_outcomes = scenario["outcomes"].as_array().cloned().unwrap_or_default();
            if scenario_outcomes.len() != outcome_ids.len() {
                push(&mut errors,"LIKELIHOOD_SCENARIO_OUTCOME_COUNT",&format!("{scenario_path}.outcomes"),"scenario must preserve the nominal outcome set");
            }
            let mut scenario_outcome_ids = BTreeSet::new();
            let mut scenario_likelihoods: BTreeMap<String,Probabilities> = BTreeMap::new();
            let mut scenario_sums: Probabilities = hypothesis_ids.iter().map(|id|(id.clone(),0.0)).collect();
            for (outcome_index,scenario_outcome) in scenario_outcomes.iter().enumerate() {
                let outcome_path = format!("{scenario_path}.outcomes[{outcome_index}]");
                keys(scenario_outcome,&["outcome_id","likelihoods"],&outcome_path,&mut errors);
                let Some(outcome_id) = text(scenario_outcome,"outcome_id") else {
                    push(&mut errors,"LIKELIHOOD_SCENARIO_OUTCOME_ID_REQUIRED",&outcome_path,"outcome ID required");
                    continue;
                };
                if !scenario_outcome_ids.insert(outcome_id.to_owned()) {
                    push(&mut errors,"DUPLICATE_LIKELIHOOD_SCENARIO_OUTCOME",&outcome_path,"scenario outcome IDs must be unique");
                }
                let Some(map) = scenario_outcome["likelihoods"].as_object() else {
                    push(&mut errors,"LIKELIHOOD_SCENARIO_MAP_REQUIRED",&outcome_path,"likelihood map required");
                    continue;
                };
                let actual: BTreeSet<String> = map.keys().cloned().collect();
                if actual != hypothesis_ids {
                    push(&mut errors,"LIKELIHOOD_SCENARIO_HYPOTHESIS_SET_MISMATCH",&outcome_path,"scenario must name every hypothesis");
                }
                let mut row = BTreeMap::new();
                for (hypothesis,value) in map {
                    let Some(probability) = finite(value) else {
                        push(&mut errors,"LIKELIHOOD_SCENARIO_VALUE_INVALID",&format!("{outcome_path}.likelihoods.{hypothesis}"),"finite likelihood required");
                        continue;
                    };
                    if !(0.0..=1.0).contains(&probability) {
                        push(&mut errors,"LIKELIHOOD_SCENARIO_VALUE_RANGE",&format!("{outcome_path}.likelihoods.{hypothesis}"),"likelihood must be in [0,1]");
                    }
                    *scenario_sums.entry(hypothesis.clone()).or_default() += probability;
                    row.insert(hypothesis.clone(),probability);
                }
                scenario_likelihoods.insert(outcome_id.to_owned(),row);
            }
            if scenario_outcome_ids != outcome_ids {
                push(&mut errors,"LIKELIHOOD_SCENARIO_OUTCOME_SET_MISMATCH",&format!("{scenario_path}.outcomes"),"scenario outcome IDs must exactly match nominal outcomes");
            }
            for (hypothesis,sum) in &scenario_sums {
                if (sum-1.0).abs() > 1e-9 {
                    push(&mut errors,"LIKELIHOOD_SCENARIO_NORMALIZATION",&format!("{scenario_path}.outcomes"),format!("scenario likelihoods for {hypothesis} must sum to one"));
                }
            }
            parsed_scenarios.push(LikelihoodScenario {
                id:scenario_id.to_owned(),
                model_sha256:scenario_sha.to_owned(),
                likelihoods:scenario_likelihoods,
            });
        }
        if text(candidate,"availability_state") != Some("AVAILABLE") { reasons.insert("OBSERVATION_NOT_AVAILABLE"); }
        if text(candidate,"measurement_quality_state") != Some("CURRENT") { reasons.insert("MEASUREMENT_NOT_CURRENT"); }
        if !matches!(text(candidate,"sensor_calibration_state"),Some("CURRENT"|"NOT_APPLICABLE")) { reasons.insert("CALIBRATION_NOT_CURRENT"); }
        if text(candidate,"timebase_qualification_state") != Some("QUALIFIED") { reasons.insert("TIMEBASE_NOT_QUALIFIED"); }
        if text(candidate,"likelihood_model_qualification_state") != Some("QUALIFIED") { reasons.insert("LIKELIHOOD_MODEL_NOT_QUALIFIED"); }
        if evidence_role == "RECONSTRUCTED" { reasons.insert("RECONSTRUCTED_EVIDENCE_NOT_INDEPENDENT"); }
        if !matches!(evidence_role,"MEASURED_EXISTING_RECORD"|"CALIBRATION_RECORD") { reasons.insert("EVIDENCE_NOT_MEASURED_OR_CALIBRATION_RECORD"); }
        if dependencies.iter().any(|dependency| existing_dependencies.contains(dependency)) {
            reasons.insert("SHARED_DEPENDENCY_WITH_EXISTING_EVIDENCE");
        }
        parsed_candidates.push(Candidate{
            id:id.to_owned(),eligible:reasons.is_empty(),reasons:reasons.into_iter().map(str::to_owned).collect(),
            cost,nominal_model_sha256:text(candidate,"likelihood_model_sha256").unwrap_or("").to_owned(),
            likelihoods,scenarios:parsed_scenarios
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
                "gross_evsi":Value::Null,"acquisition_cost":candidate.cost,"net_evsi":Value::Null,
                "robust_net_evsi_min":Value::Null,"robust_net_evsi_max":Value::Null,
                "likelihood_scenario_count":Value::Null,
                "prior_ambiguity_scenario_count":Value::Null,
                "joint_scenario_count":Value::Null
            }));
            continue;
        }
        let mut joint_scenarios = Vec::new();
        let mut nominal = None;
        let mut prior_ensemble = vec![PriorScenario {
            id:"NOMINAL_PRIOR".to_owned(),model_sha256:String::new(),probabilities:priors.clone(),
        }];
        prior_ensemble.extend(prior_scenarios.iter().map(|scenario|PriorScenario {
            id:scenario.id.clone(),model_sha256:scenario.model_sha256.clone(),
            probabilities:scenario.probabilities.clone(),
        }));
        for prior_scenario in &prior_ensemble {
            let scenario_prior_risk = best_decision(&prior_scenario.probabilities,&decisions).1;
            let mut likelihood_models: Vec<(String,String,&BTreeMap<String,Probabilities>)> = vec![(
                "NOMINAL".to_owned(),candidate.nominal_model_sha256.clone(),&candidate.likelihoods,
            )];
            likelihood_models.extend(candidate.scenarios.iter().map(|scenario|(
                scenario.id.clone(),scenario.model_sha256.clone(),&scenario.likelihoods
            )));
            for (likelihood_id,likelihood_sha,likelihoods) in likelihood_models {
                let joint_id = format!("{}::{}",prior_scenario.id,likelihood_id);
                let mut scored = score_likelihood_model(
                    &joint_id,&likelihood_sha,&prior_scenario.probabilities,&decisions,
                    likelihoods,scenario_prior_risk,candidate.cost
                ).map_err(|detail|vec![json!({
                    "code":"INVALID_JOINT_EVSI_SCENARIO",
                    "path":format!("$.candidates[{}].joint_scenarios.{}",candidate.id,joint_id),
                    "detail":detail
                })])?;
                scored["prior_scenario_id"] = json!(prior_scenario.id);
                scored["prior_model_sha256"] = if prior_scenario.model_sha256.is_empty() {
                    Value::Null
                } else { json!(prior_scenario.model_sha256) };
                scored["likelihood_scenario_id"] = json!(likelihood_id);
                if prior_scenario.id == "NOMINAL_PRIOR" && likelihood_id == "NOMINAL" {
                    nominal = Some(scored.clone());
                }
                joint_scenarios.push(scored);
            }
        }
        let nominal = nominal.ok_or_else(||vec![json!({
            "code":"NOMINAL_JOINT_SCENARIO_MISSING",
            "path":format!("$.candidates[{}]",candidate.id),
            "detail":"nominal prior × nominal likelihood scenario was not evaluated"
        })])?;
        let robust_min = joint_scenarios.iter()
            .map(|model|model["net_evsi"].as_f64().unwrap_or(f64::NEG_INFINITY))
            .fold(f64::INFINITY,f64::min);
        let robust_max = joint_scenarios.iter()
            .map(|model|model["net_evsi"].as_f64().unwrap_or(f64::NEG_INFINITY))
            .fold(f64::NEG_INFINITY,f64::max);
        let worst_case_scenario_id = joint_scenarios.iter()
            .min_by(|left,right|left["net_evsi"].as_f64().unwrap_or(f64::INFINITY)
                .total_cmp(&right["net_evsi"].as_f64().unwrap_or(f64::INFINITY)))
            .map(|model|format!("{}::{}",model["prior_scenario_id"].as_str().unwrap_or("UNKNOWN"),
                model["likelihood_scenario_id"].as_str().unwrap_or("UNKNOWN")))
            .unwrap_or_else(||"UNKNOWN".to_owned());
        let status = if robust_min > 1e-12 {
            positive_count += 1;"ROBUST_POSITIVE_NET_VALUE"
        } else {"NOT_ROBUST_POSITIVE_NET_VALUE"};
        evaluated_count += 1;
        candidate_receipts.push(json!({
            "candidate_id":candidate.id,"status":status,"reason_codes":[],
            "prior_bayes_risk":prior_risk,
            "expected_posterior_bayes_risk":nominal["expected_posterior_bayes_risk"],
            "gross_evsi":nominal["gross_evsi"],"acquisition_cost":candidate.cost,
            "net_evsi":nominal["net_evsi"],"robust_net_evsi_min":robust_min,
            "robust_net_evsi_max":robust_max,"worst_case_scenario_id":worst_case_scenario_id,
            "likelihood_scenario_count":candidate.scenarios.len()+1,
            "prior_ambiguity_scenario_count":prior_scenarios.len(),
            "joint_scenario_count":joint_scenarios.len(),"scenario_results":joint_scenarios
        }));
    }
    candidate_receipts.sort_by(|left,right| {
        let left_invalid = left["status"] == "NOT_EVALUATED";
        let right_invalid = right["status"] == "NOT_EVALUATED";
        match (left_invalid,right_invalid) {
            (false,true) => std::cmp::Ordering::Less,
            (true,false) => std::cmp::Ordering::Greater,
            (false,false) => right["robust_net_evsi_min"].as_f64().unwrap_or(f64::NEG_INFINITY)
                .total_cmp(&left["robust_net_evsi_min"].as_f64().unwrap_or(f64::NEG_INFINITY))
                .then_with(|| left["candidate_id"].as_str().cmp(&right["candidate_id"].as_str())),
            (true,true) => left["candidate_id"].as_str().cmp(&right["candidate_id"].as_str())
        }
    });
    let not_evaluated_count = candidate_receipts.iter().filter(|item|item["status"]=="NOT_EVALUATED").count();
    Ok(json!({
        "result":"pass","evaluator_version":VERSION,"assessment_id":text(input,"assessment_id").unwrap_or(""),
        "hypothesis_count":priors.len(),"decision_count":decisions.len(),"candidate_count":candidates.len(),
        "prior_ambiguity_scenario_count":prior_scenarios.len(),
        "joint_scenario_rule":"MAXIMIN_OVER_NOMINAL_AND_QUALIFIED_PRIOR_SET_CROSSED_WITH_NOMINAL_AND_QUALIFIED_LIKELIHOOD_SET",
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
    verify_sha256(raw.as_bytes(),digest)?;
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
    fn fixture()->Value{serde_json::from_str(include_str!("../../../../benchmarks/maritime-observability/observation-value-fixture.v0.1.json")).expect("EVSI fixture parses")}
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
        assert_eq!(report["ranked_candidates"][0]["status"],"ROBUST_POSITIVE_NET_VALUE");
        let net=report["ranked_candidates"][0]["net_evsi"].as_f64().unwrap();
        assert!((net-0.0295).abs()<1e-9);
    }
    #[test]
    fn non_positive_candidate_is_not_recommended(){
        let report=evaluate(&fixture()).unwrap();
        assert_eq!(report["ranked_candidates"][1]["candidate_id"],"independent-exhaust-temperature-record");
        assert_eq!(report["ranked_candidates"][1]["status"],"NOT_ROBUST_POSITIVE_NET_VALUE");
    }
    #[test]
    fn derived_or_shared_evidence_is_not_scored(){
        let report=evaluate(&fixture()).unwrap();
        let derived=report["ranked_candidates"].as_array().unwrap().iter().find(|item|item["candidate_id"]=="derived-load-estimate").unwrap();
        assert_eq!(derived["status"],"NOT_EVALUATED");
        assert!(derived["reason_codes"].as_array().unwrap().iter().any(|reason|reason=="SHARED_DEPENDENCY_WITH_EXISTING_EVIDENCE"));
        assert!(derived["reason_codes"].as_array().unwrap().iter().any(|reason|reason=="EVIDENCE_NOT_MEASURED_OR_CALIBRATION_RECORD"));
    }
    #[test]
    fn reconstructed_candidate_is_not_scored_as_new_independent_observation(){
        let mut input=fixture();
        input["candidates"][0]["evidence_role"]=json!("RECONSTRUCTED");
        let report=evaluate(&input).unwrap();
        let candidate=report["ranked_candidates"].as_array().unwrap()
            .iter().find(|item|item["candidate_id"]=="needle-lift-existing-trace").unwrap();
        assert_eq!(candidate["status"],"NOT_EVALUATED");
        assert!(candidate["reason_codes"].as_array().unwrap()
            .iter().any(|code|code=="RECONSTRUCTED_EVIDENCE_NOT_INDEPENDENT"));
    }
    #[test]
    fn stale_or_expired_record_is_not_scored(){
        let report=evaluate(&fixture()).unwrap();
        let stale=report["ranked_candidates"].as_array().unwrap().iter().find(|item|item["candidate_id"]=="stale-thermocouple-record").unwrap();
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
    fn prior_ambiguity_scenarios_must_normalize_and_bind_unique_models(){
        let mut input=fixture();
        input["prior_ambiguity_scenarios"][0]["prior_probabilities"]["FUEL_RESTRICTION"]=json!(0.99);
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="PRIOR_SCENARIO_NORMALIZATION"));
        input=fixture();
        let digest=input["prior_ambiguity_scenarios"][0]["prior_model_sha256"].clone();
        input["prior_ambiguity_scenarios"][1]["prior_model_sha256"]=digest;
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="DUPLICATE_PRIOR_SCENARIO_SHA"));
    }
    #[test]
    fn nominally_positive_measurement_abstains_under_plausible_prior_ambiguity(){
        let mut input=fixture();
        let initial=evaluate(&input).unwrap();
        let initial_candidate=initial["ranked_candidates"].as_array().unwrap().iter()
            .find(|item|item["candidate_id"]=="needle-lift-existing-trace").unwrap();
        assert!(initial_candidate["net_evsi"].as_f64().unwrap()>0.0);
        assert_eq!(initial_candidate["status"],"ROBUST_POSITIVE_NET_VALUE");
        input["prior_ambiguity_scenarios"][0]["prior_probabilities"]["FUEL_RESTRICTION"]=json!(0.42);
        input["prior_ambiguity_scenarios"][0]["prior_probabilities"]["INJECTOR_DEGRADATION"]=json!(0.42);
        input["prior_ambiguity_scenarios"][0]["prior_probabilities"]["SENSOR_BIAS"]=json!(0.16);
        let changed=evaluate(&input).unwrap();
        let candidate=changed["ranked_candidates"].as_array().unwrap().iter()
            .find(|item|item["candidate_id"]=="needle-lift-existing-trace").unwrap();
        assert!(candidate["net_evsi"].as_f64().unwrap()>0.0);
        assert!(candidate["robust_net_evsi_min"].as_f64().unwrap()<=0.0);
        assert_eq!(candidate["status"],"NOT_ROBUST_POSITIVE_NET_VALUE");
        assert_eq!(candidate["joint_scenario_count"],9);
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
    #[test]
    fn nominal_value_must_survive_all_likelihood_scenarios(){
        let mut input=fixture();
        let initial=evaluate(&input).unwrap();
        assert_eq!(initial["ranked_candidates"][0]["status"],"ROBUST_POSITIVE_NET_VALUE");
        assert!(initial["ranked_candidates"][0]["robust_net_evsi_min"].as_f64().unwrap()>0.0);
        for (index,value) in [0.6,0.3,0.1].iter().enumerate(){
            for hypothesis in ["FUEL_RESTRICTION","INJECTOR_DEGRADATION","SENSOR_BIAS"]{
                input["candidates"][0]["likelihood_scenarios"][0]["outcomes"][index]["likelihoods"][hypothesis]=json!(value);
            }
        }
        let changed=evaluate(&input).unwrap();
        let candidate=changed["ranked_candidates"].as_array().unwrap().iter()
            .find(|item|item["candidate_id"]=="needle-lift-existing-trace").unwrap();
        assert_eq!(candidate["status"],"NOT_ROBUST_POSITIVE_NET_VALUE");
        assert!(candidate["net_evsi"].as_f64().unwrap()>0.0);
        assert!(candidate["robust_net_evsi_min"].as_f64().unwrap()<=0.0);
    }
    #[test]
    fn unqualified_likelihood_scenario_is_not_scored(){
        let mut input=fixture();
        input["candidates"][0]["likelihood_scenarios"][0]["qualification_state"]=json!("UNKNOWN");
        let report=evaluate(&input).unwrap();
        let candidate=report["ranked_candidates"].as_array().unwrap().iter()
            .find(|item|item["candidate_id"]=="needle-lift-existing-trace").unwrap();
        assert_eq!(candidate["status"],"NOT_EVALUATED");
        assert!(candidate["reason_codes"].as_array().unwrap().iter().any(|code|code=="LIKELIHOOD_SCENARIO_NOT_QUALIFIED"));
    }
    #[test]
    fn likelihood_scenario_probabilities_must_normalize(){
        let mut input=fixture();
        input["candidates"][0]["likelihood_scenarios"][0]["outcomes"][0]["likelihoods"]["FUEL_RESTRICTION"]=json!(0.99);
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="LIKELIHOOD_SCENARIO_NORMALIZATION"));
    }
    #[test]
    fn duplicate_likelihood_scenario_digests_are_rejected(){
        let mut input=fixture();
        let digest=input["candidates"][0]["likelihood_scenarios"][0]["likelihood_model_sha256"].clone();
        input["candidates"][0]["likelihood_scenarios"][1]["likelihood_model_sha256"]=digest;
        assert!(evaluate(&input).unwrap_err().iter().any(|error|error["code"]=="DUPLICATE_LIKELIHOOD_SCENARIO_SHA"));
    }
}
