// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later
//
// Exact finite-state value-of-information audit. It measures expected decision
// value under supplied priors, utility assumptions and signal likelihoods.
// It does not infer source truth or establish causal impact.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet},
    env, fs, process,
};

const SCHEMA_BYTES: &[u8] = include_bytes!("../../../docs/schemas/pirb-voi-v0.1.schema.json");
#[cfg(test)]
const FIXTURE_BYTES: &[u8] = include_bytes!("../../../docs/fixtures/pirb-voi-synthetic-v0.1.json");
const EPSILON: f64 = 1.0e-10;

type Distribution = BTreeMap<String, f64>;
type ActionUtilities = BTreeMap<String, Distribution>;

fn sha256_hex(bytes: &[u8]) -> String {
    hex::encode(Sha256::digest(bytes))
}

fn option_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("argument {name} requires a value"))
}

fn string_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn number_at(value: &Value, key: &str) -> Option<f64> {
    value.get(key).and_then(Value::as_f64)
}

fn integer_at(value: &Value, key: &str) -> Option<i64> {
    value.get(key).and_then(Value::as_i64)
}

fn is_placeholder_digest(digest: &str) -> bool {
    digest.chars().all(|ch| ch == '0') || digest.chars().all(|ch| ch == '1')
}

fn parse_prior(registration: &Value) -> Distribution {
    registration["states"]
        .as_array()
        .expect("schema validation requires states")
        .iter()
        .map(|state| {
            (
                string_at(state, "state_id")
                    .expect("schema validation requires state_id")
                    .to_owned(),
                number_at(state, "prior_probability")
                    .expect("schema validation requires prior_probability"),
            )
        })
        .collect()
}

fn parse_actions(registration: &Value) -> ActionUtilities {
    registration["actions"]
        .as_array()
        .expect("schema validation requires actions")
        .iter()
        .map(|action| {
            let action_id = string_at(action, "action_id")
                .expect("schema validation requires action_id")
                .to_owned();
            let utilities = action["state_utilities"]
                .as_array()
                .expect("schema validation requires state_utilities")
                .iter()
                .map(|row| {
                    (
                        string_at(row, "state_id")
                            .expect("schema validation requires utility state_id")
                            .to_owned(),
                        number_at(row, "utility").expect("schema validation requires utility"),
                    )
                })
                .collect();
            (action_id, utilities)
        })
        .collect()
}

fn expected_utility(prior: &Distribution, utilities: &Distribution) -> f64 {
    prior
        .iter()
        .map(|(state_id, probability)| probability * utilities[state_id])
        .sum()
}

fn best_action(prior: &Distribution, actions: &ActionUtilities) -> (String, f64) {
    let mut best: Option<(String, f64)> = None;
    // BTreeMap iteration provides lexical tie-breaking: equal-valued actions do
    // not replace the first action already selected.
    for (action_id, utilities) in actions {
        let value = expected_utility(prior, utilities);
        if best.as_ref().is_none_or(|(_, best_value)| value > *best_value) {
            best = Some((action_id.clone(), value));
        }
    }
    best.expect("schema validation requires at least one action")
}

fn shannon_entropy(probabilities: impl IntoIterator<Item = f64>) -> f64 {
    probabilities
        .into_iter()
        .filter(|probability| *probability > 0.0)
        .map(|probability| -probability * probability.ln())
        .sum()
}

fn posterior_digest(posterior: &Distribution) -> String {
    let canonical = posterior
        .iter()
        .map(|(state_id, probability)| format!("{state_id}={probability:.17}"))
        .collect::<Vec<_>>()
        .join("\n");
    sha256_hex(canonical.as_bytes())
}

#[derive(Clone)]
struct SourceEvaluation {
    source_id: String,
    admissible: bool,
    acquisition_cost: f64,
    delay_penalty: f64,
    signal_entropy_nats: Option<f64>,
    gross_evsi: Option<f64>,
    net_evsi: Option<f64>,
    report: Value,
}

fn source_is_admissible(
    source: &Value,
    evidence_scope: &str,
    decision_deadline: i64,
) -> Result<Option<&'static str>, Vec<String>> {
    let source_id = string_at(source, "source_id").unwrap_or("<missing>");
    let status = string_at(source, "source_status").unwrap_or_default();
    if evidence_scope == "authorized_historical" && status != "authorized" {
        return Err(vec![format!(
            "historical source {source_id} must have source_status=authorized"
        )]);
    }
    if status == "denied" || status == "pending" {
        return Ok(Some("source_not_authorized_or_not_cleared"));
    }
    if status != "authorized" && status != "not_applicable" {
        return Err(vec![format!(
            "source {source_id} has an unsupported source_status"
        )]);
    }
    if evidence_scope == "synthetic" && status == "authorized" {
        // An explicitly authorized synthetic source is acceptable, though
        // source provenance remains a declared input rather than authentication.
    }
    let available_by = integer_at(source, "available_by_index").unwrap_or_default();
    if available_by > decision_deadline {
        return Ok(Some("arrives_after_decision_deadline"));
    }
    Ok(None)
}

fn evaluate_source(
    source: &Value,
    evidence_scope: &str,
    decision_deadline: i64,
    prior: &Distribution,
    actions: &ActionUtilities,
    baseline_action_id: &str,
    baseline_utility: f64,
) -> Result<SourceEvaluation, Vec<String>> {
    let source_id = string_at(source, "source_id")
        .expect("schema validation requires source_id")
        .to_owned();
    let acquisition_cost = number_at(source, "acquisition_cost")
        .expect("schema validation requires acquisition_cost");
    let delay_penalty =
        number_at(source, "delay_penalty").expect("schema validation requires delay_penalty");
    let inadmissible_reason =
        source_is_admissible(source, evidence_scope, decision_deadline)?;
    if let Some(reason) = inadmissible_reason {
        return Ok(SourceEvaluation {
            source_id: source_id.clone(),
            admissible: false,
            acquisition_cost,
            delay_penalty,
            signal_entropy_nats: None,
            gross_evsi: None,
            net_evsi: None,
            report: json!({
                "source_id": source_id,
                "admissible": false,
                "inadmissible_reason": reason,
                "available_by_index": source["available_by_index"],
                "acquisition_cost": acquisition_cost,
                "delay_penalty": delay_penalty,
                "signal_entropy_nats": Value::Null,
                "current_best_action_id": baseline_action_id,
                "current_best_expected_utility": baseline_utility,
                "expected_post_information_utility": Value::Null,
                "gross_evsi": Value::Null,
                "net_evsi": Value::Null,
                "signal_diagnostics": []
            }),
        });
    }

    let state_ids = prior.keys().cloned().collect::<Vec<_>>();
    let signals = source["signals"]
        .as_array()
        .expect("schema validation requires signals");
    let mut signal_probabilities = Vec::new();
    let mut signal_diagnostics = Vec::new();
    let mut expected_post_information_utility = 0.0;

    for signal in signals {
        let signal_id = string_at(signal, "signal_id")
            .expect("schema validation requires signal_id");
        let likelihoods = signal["state_likelihoods"]
            .as_array()
            .expect("schema validation requires state_likelihoods")
            .iter()
            .map(|row| {
                (
                    string_at(row, "state_id")
                        .expect("schema validation requires likelihood state_id")
                        .to_owned(),
                    number_at(row, "probability")
                        .expect("schema validation requires likelihood probability"),
                )
            })
            .collect::<BTreeMap<_, _>>();
        let signal_probability = state_ids
            .iter()
            .map(|state_id| prior[state_id] * likelihoods[state_id])
            .sum::<f64>();
        if signal_probability <= 0.0 {
            return Err(vec![format!(
                "source {source_id} has zero-probability signal {signal_id} under the registered prior"
            )]);
        }

        let posterior = state_ids
            .iter()
            .map(|state_id| {
                (
                    state_id.clone(),
                    prior[state_id] * likelihoods[state_id] / signal_probability,
                )
            })
            .collect::<Distribution>();
        let posterior_sum = posterior.values().sum::<f64>();
        if (posterior_sum - 1.0).abs() > EPSILON {
            return Err(vec![format!(
                "source {source_id} signal {signal_id} produces a non-normalized posterior"
            )]);
        }

        let (action_id, action_utility) = best_action(&posterior, actions);
        expected_post_information_utility += signal_probability * action_utility;
        signal_probabilities.push(signal_probability);
        signal_diagnostics.push(json!({
            "signal_id": signal_id,
            "signal_probability": signal_probability,
            "posterior_sha256": posterior_digest(&posterior),
            "best_action_id": action_id,
            "best_action_expected_utility": action_utility
        }));
    }

    let signal_entropy_nats = shannon_entropy(signal_probabilities);
    let gross_raw = expected_post_information_utility - baseline_utility;
    if gross_raw < -EPSILON {
        return Err(vec![format!(
            "source {source_id} produced negative gross EVSI beyond numerical tolerance"
        )]);
    }
    let gross_evsi = gross_raw.max(0.0);
    let net_evsi = gross_evsi - acquisition_cost - delay_penalty;
    Ok(SourceEvaluation {
        source_id: source_id.clone(),
        admissible: true,
        acquisition_cost,
        delay_penalty,
        signal_entropy_nats: Some(signal_entropy_nats),
        gross_evsi: Some(gross_evsi),
        net_evsi: Some(net_evsi),
        report: json!({
            "source_id": source_id,
            "admissible": true,
            "inadmissible_reason": Value::Null,
            "available_by_index": source["available_by_index"],
            "acquisition_cost": acquisition_cost,
            "delay_penalty": delay_penalty,
            "signal_entropy_nats": signal_entropy_nats,
            "current_best_action_id": baseline_action_id,
            "current_best_expected_utility": baseline_utility,
            "expected_post_information_utility": expected_post_information_utility,
            "gross_evsi": gross_evsi,
            "net_evsi": net_evsi,
            "recommendation": if net_evsi > 0.0 { "acquire_for_review" } else { "do_not_acquire_under_registered_assumptions" },
            "signal_diagnostics": signal_diagnostics
        }),
    })
}

fn best_source_by<'a>(
    evaluations: &'a [SourceEvaluation],
    metric: &str,
) -> Option<&'a SourceEvaluation> {
    evaluations
        .iter()
        .filter(|evaluation| evaluation.admissible)
        .filter(|evaluation| match metric {
            "net_evsi" => evaluation.net_evsi.is_some_and(|value| value > 0.0),
            "acquisition_cost" => true,
            "signal_entropy_nats" => evaluation.signal_entropy_nats.is_some(),
            _ => false,
        })
        .max_by(|left, right| {
            let value_order = match metric {
                "net_evsi" => left.net_evsi.partial_cmp(&right.net_evsi),
                "acquisition_cost" => right.acquisition_cost.partial_cmp(&left.acquisition_cost),
                "signal_entropy_nats" => {
                    left.signal_entropy_nats.partial_cmp(&right.signal_entropy_nats)
                }
                _ => Some(Ordering::Equal),
            }
            .unwrap_or(Ordering::Equal);
            value_order.then_with(|| right.source_id.cmp(&left.source_id))
        })
}

fn semantic_errors(registration: &Value) -> Vec<String> {
    let mut errors = Vec::new();
    let origin = integer_at(registration, "forecast_origin_index").unwrap_or_default();
    let deadline = integer_at(registration, "decision_deadline_index").unwrap_or_default();
    let target_period = integer_at(registration, "target_period_index").unwrap_or_default();
    let prior_cutoff = integer_at(registration, "prior_fit_cutoff_index").unwrap_or_default();
    let likelihood_cutoff =
        integer_at(registration, "likelihood_fit_cutoff_index").unwrap_or_default();

    if deadline < origin || deadline >= target_period {
        errors.push(
            "decision deadline must be on or after forecast origin and strictly before target period"
                .to_owned(),
        );
    }
    if target_period <= origin {
        errors.push("target period must be later than forecast origin".to_owned());
    }
    if prior_cutoff >= origin {
        errors.push("prior fit cutoff must be strictly before forecast origin".to_owned());
    }
    if likelihood_cutoff >= origin {
        errors.push("likelihood fit cutoff must be strictly before forecast origin".to_owned());
    }

    let states = registration["states"]
        .as_array()
        .expect("schema validation requires states");
    let mut state_ids = BTreeSet::new();
    let mut prior_sum = 0.0;
    for state in states {
        let id = string_at(state, "state_id").unwrap_or_default();
        if !state_ids.insert(id.to_owned()) {
            errors.push(format!("state ID is duplicated: {id}"));
        }
        prior_sum += number_at(state, "prior_probability").unwrap_or_default();
    }
    if (prior_sum - 1.0).abs() > EPSILON {
        errors.push(format!("state prior probabilities sum to {prior_sum}, not 1"));
    }

    let mut action_ids = BTreeSet::new();
    for action in registration["actions"]
        .as_array()
        .expect("schema validation requires actions")
    {
        let action_id = string_at(action, "action_id").unwrap_or_default();
        if !action_ids.insert(action_id.to_owned()) {
            errors.push(format!("action ID is duplicated: {action_id}"));
        }
        let utilities = action["state_utilities"]
            .as_array()
            .expect("schema validation requires state_utilities");
        let mut utility_states = BTreeSet::new();
        for utility in utilities {
            let state_id = string_at(utility, "state_id").unwrap_or_default();
            if !state_ids.contains(state_id) {
                errors.push(format!(
                    "action {action_id} references unknown state {state_id}"
                ));
            }
            if !utility_states.insert(state_id.to_owned()) {
                errors.push(format!(
                    "action {action_id} has duplicate utility state {state_id}"
                ));
            }
        }
        if utility_states != state_ids {
            errors.push(format!(
                "action {action_id} must specify exactly one utility for every registered state"
            ));
        }
    }

    let mut source_ids = BTreeSet::new();
    for source in registration["information_sources"]
        .as_array()
        .expect("schema validation requires information_sources")
    {
        let source_id = string_at(source, "source_id").unwrap_or_default();
        if !source_ids.insert(source_id.to_owned()) {
            errors.push(format!("source ID is duplicated: {source_id}"));
        }
        let available_by = integer_at(source, "available_by_index").unwrap_or_default();
        if available_by <= origin {
            errors.push(format!(
                "source {source_id} must arrive after forecast origin; already available evidence belongs in the prior"
            ));
        }
        if string_at(&registration, "evidence_scope") == Some("authorized_historical") {
            if string_at(source, "source_status") != Some("authorized") {
                errors.push(format!(
                    "historical source {source_id} requires source_status=authorized"
                ));
            }
            for (field, label) in [
                ("source_artifact_sha256", "source artifact"),
                ("likelihood_sha256", "likelihood model"),
            ] {
                let digest = string_at(source, field).unwrap_or_default();
                if is_placeholder_digest(digest) {
                    errors.push(format!(
                        "historical source {source_id} requires a non-placeholder {label} digest"
                    ));
                }
            }
        }

        let mut signal_ids = BTreeSet::new();
        let mut likelihood_sums: BTreeMap<String, f64> =
            state_ids.iter().map(|id| (id.clone(), 0.0)).collect();
        for signal in source["signals"]
            .as_array()
            .expect("schema validation requires signals")
        {
            let signal_id = string_at(signal, "signal_id").unwrap_or_default();
            if !signal_ids.insert(signal_id.to_owned()) {
                errors.push(format!(
                    "source {source_id} has duplicate signal ID {signal_id}"
                ));
            }
            let mut likelihood_states = BTreeSet::new();
            for likelihood in signal["state_likelihoods"]
                .as_array()
                .expect("schema validation requires state_likelihoods")
            {
                let state_id = string_at(likelihood, "state_id").unwrap_or_default();
                if !state_ids.contains(state_id) {
                    errors.push(format!(
                        "source {source_id} signal {signal_id} references unknown state {state_id}"
                    ));
                }
                if !likelihood_states.insert(state_id.to_owned()) {
                    errors.push(format!(
                        "source {source_id} signal {signal_id} has duplicate likelihood state {state_id}"
                    ));
                }
                if let Some(sum) = likelihood_sums.get_mut(state_id) {
                    *sum += number_at(likelihood, "probability").unwrap_or_default();
                }
            }
            if likelihood_states != state_ids {
                errors.push(format!(
                    "source {source_id} signal {signal_id} must specify exactly one likelihood for every registered state"
                ));
            }
        }
        for (state_id, sum) in likelihood_sums {
            if (sum - 1.0).abs() > EPSILON {
                errors.push(format!(
                    "source {source_id} likelihoods for state {state_id} sum to {sum}, not 1"
                ));
            }
        }

        if let Some(prior) = registration["states"].as_array() {
            let prior_map = prior
                .iter()
                .map(|state| {
                    (
                        string_at(state, "state_id")
                            .expect("schema validation requires state_id")
                            .to_owned(),
                        number_at(state, "prior_probability")
                            .expect("schema validation requires prior_probability"),
                    )
                })
                .collect::<Distribution>();
            for signal in source["signals"]
                .as_array()
                .expect("schema validation requires signals")
            {
                let likelihood_map = signal["state_likelihoods"]
                    .as_array()
                    .expect("schema validation requires state_likelihoods")
                    .iter()
                    .map(|row| {
                        (
                            string_at(row, "state_id")
                                .expect("schema validation requires state_id")
                                .to_owned(),
                            number_at(row, "probability")
                                .expect("schema validation requires probability"),
                        )
                    })
                    .collect::<Distribution>();
                let signal_probability = prior_map
                    .iter()
                    .map(|(state_id, prior_probability)| {
                        prior_probability * likelihood_map[state_id]
                    })
                    .sum::<f64>();
                if signal_probability <= 0.0 {
                    errors.push(format!(
                        "source {source_id} signal {} has zero marginal probability under the registered prior",
                        string_at(signal, "signal_id").unwrap_or("<missing>")
                    ));
                }
            }
        }
    }

    if string_at(&registration, "evidence_scope") == Some("authorized_historical") {
        let model = &registration["model"];
        for (field, label) in [
            ("scenario_model_sha256", "scenario model"),
            ("utility_model_sha256", "utility model"),
        ] {
            if is_placeholder_digest(string_at(model, field).unwrap_or_default()) {
                errors.push(format!(
                    "historical evaluation requires a non-placeholder {label} digest"
                ));
            }
        }
        let governance = &registration["governance"];
        if string_at(governance, "data_authority_reference").is_none_or(str::is_empty)
            || string_at(governance, "disclosure_review_reference").is_none_or(str::is_empty)
        {
            errors.push(
                "historical evaluation requires non-empty authority and disclosure references"
                    .to_owned(),
            );
        }
    }

    errors.sort();
    errors
}

fn validate_and_evaluate(input: &[u8], source_commit: &str) -> Result<Value, Vec<String>> {
    if source_commit != "UNPINNED_LOCAL_RUN"
        && (source_commit.len() != 40
            || !source_commit
                .chars()
                .all(|ch| ch.is_ascii_hexdigit() && !ch.is_ascii_uppercase()))
    {
        return Err(vec![
            "source commit must be a 40-character lowercase Git SHA or UNPINNED_LOCAL_RUN"
                .to_owned(),
        ]);
    }
    let registration: Value = serde_json::from_slice(input).map_err(|error| {
        vec![format!(
            "value-of-information registration JSON is invalid: {error}"
        )]
    })?;
    let schema: Value = serde_json::from_slice(SCHEMA_BYTES).map_err(|error| {
        vec![format!("embedded VOI schema JSON is invalid: {error}")]
    })?;
    jsonschema::draft202012::meta::validate(&schema).map_err(|error| {
        vec![format!("Draft 2020-12 schema meta-validation failed: {error}")]
    })?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|error| vec![format!("cannot build VOI schema validator: {error}")])?;
    let mut errors = validator
        .iter_errors(&registration)
        .map(|error| format!("{} at {}", error, error.instance_path()))
        .collect::<Vec<_>>();
    if !errors.is_empty() {
        errors.sort();
        return Err(errors);
    }
    let semantic = semantic_errors(&registration);
    if !semantic.is_empty() {
        return Err(semantic);
    }

    let prior = parse_prior(&registration);
    let actions = parse_actions(&registration);
    let (current_action_id, current_expected_utility) = best_action(&prior, &actions);
    let evidence_scope =
        string_at(&registration, "evidence_scope").expect("schema validated evidence_scope");
    let decision_deadline =
        integer_at(&registration, "decision_deadline_index").expect("schema validated deadline");
    let mut source_evaluations = Vec::new();
    for source in registration["information_sources"]
        .as_array()
        .expect("schema validated sources")
    {
        source_evaluations.push(evaluate_source(
            source,
            evidence_scope,
            decision_deadline,
            &prior,
            &actions,
            &current_action_id,
            current_expected_utility,
        )?);
    }

    let net_choice = best_source_by(&source_evaluations, "net_evsi");
    let cost_choice = best_source_by(&source_evaluations, "acquisition_cost");
    let entropy_choice = best_source_by(&source_evaluations, "signal_entropy_nats");
    let source_reports = source_evaluations
        .iter()
        .map(|evaluation| evaluation.report.clone())
        .collect::<Vec<_>>();

    let source_choice = |evaluation: Option<&SourceEvaluation>, metric_name: &str| {
        evaluation.map_or_else(
            || {
                json!({
                    "source_id": Value::Null,
                    metric_name: Value::Null,
                    "net_evsi": Value::Null
                })
            },
            |evaluation| {
                let metric_value = match metric_name {
                    "net_evsi" => evaluation.net_evsi,
                    "acquisition_cost" => Some(evaluation.acquisition_cost),
                    "signal_entropy_nats" => evaluation.signal_entropy_nats,
                    _ => None,
                };
                json!({
                    "source_id": evaluation.source_id,
                    metric_name: metric_value,
                    "net_evsi": evaluation.net_evsi
                })
            },
        )
    };

    let schema_sha256 = sha256_hex(SCHEMA_BYTES);
    let input_sha256 = sha256_hex(input);
    let registration_id =
        string_at(&registration, "registration_id").expect("schema validated registration_id");
    let receipt_commitment_sha256 = sha256_hex(
        format!(
            "pirb-voi-receipt-v0.1\n{registration_id}\n{source_commit}\n{schema_sha256}\n{input_sha256}"
        )
        .as_bytes(),
    );

    Ok(json!({
        "receipt_version": "0.1",
        "receipt_type": "value_of_information_audit",
        "registration_id": registration_id,
        "evidence_scope": evidence_scope,
        "target_id": registration["target_id"],
        "value_unit": registration["value_unit"],
        "forecast_origin_index": registration["forecast_origin_index"],
        "decision_deadline_index": registration["decision_deadline_index"],
        "target_period_index": registration["target_period_index"],
        "source_commit": source_commit,
        "source_commit_pinned": source_commit != "UNPINNED_LOCAL_RUN",
        "input_sha256": input_sha256,
        "schema_sha256": schema_sha256,
        "model_id": registration["model"]["model_id"],
        "scenario_model_sha256": registration["model"]["scenario_model_sha256"],
        "utility_model_sha256": registration["model"]["utility_model_sha256"],
        "state_count": prior.len(),
        "action_count": actions.len(),
        "current_best_action_id": current_action_id,
        "current_best_expected_utility": current_expected_utility,
        "source_count": source_evaluations.len(),
        "sources": source_reports,
        "strategy_baselines": {
            "highest_net_evsi_positive_source": source_choice(net_choice, "net_evsi"),
            "lowest_cost_admissible_source": source_choice(cost_choice, "acquisition_cost"),
            "highest_signal_entropy_admissible_source": source_choice(entropy_choice, "signal_entropy_nats")
        },
        "claims": {
            "schema_and_probability_contract_validated": true,
            "exact_finite_state_evsi_calculated": true,
            "entropy_is_decision_value": false,
            "source_reliability_established": false,
            "forecast_skill_improvement_established": false,
            "decision_utility_is_causal_impact": false,
            "causal_impact_established": false,
            "live_decision_authority": false,
            "person_level_targeting_authorized": false,
            "service_access_gating_authorized": false
        },
        "receipt_commitment_sha256": receipt_commitment_sha256
    }))
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let input_path = option_value(&args, "--input")?.ok_or_else(|| {
        "usage: pirb-voi-audit --input REGISTRATION.json [--source-commit SHA]".to_owned()
    })?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read VOI registration {input_path}: {error}"))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let receipt = validate_and_evaluate(&input, &source_commit).map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize VOI receipt: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Value {
        serde_json::from_slice(FIXTURE_BYTES)
            .expect("checked-in synthetic VOI fixture is valid JSON")
    }

    fn evaluate(value: &Value) -> Result<Value, Vec<String>> {
        validate_and_evaluate(
            &serde_json::to_vec(value).expect("registration serializes"),
            "0123456789abcdef0123456789abcdef01234567",
        )
    }

    fn source<'a>(receipt: &'a Value, source_id: &str) -> &'a Value {
        receipt["sources"]
            .as_array()
            .expect("receipt includes sources")
            .iter()
            .find(|value| string_at(value, "source_id") == Some(source_id))
            .expect("source is reported")
    }

    #[test]
    fn informative_signal_has_exact_golden_evsi_and_net_value() {
        let receipt = evaluate(&fixture()).expect("synthetic VOI registration is valid");
        assert_eq!(receipt["current_best_action_id"], "action-prevent");
        assert!((number_at(&receipt, "current_best_expected_utility").unwrap() - 2.0).abs() < EPSILON);
        let briefing = source(&receipt, "source-briefing");
        assert_eq!(briefing["admissible"], true);
        assert_eq!(briefing["signal_diagnostics"][0]["signal_id"], "signal-alarm");
        assert!((number_at(&briefing["signal_diagnostics"][0], "signal_probability").unwrap() - 0.24).abs() < EPSILON);
        assert!((number_at(briefing, "expected_post_information_utility").unwrap() - 4.4).abs() < EPSILON);
        assert!((number_at(briefing, "gross_evsi").unwrap() - 2.4).abs() < EPSILON);
        assert!((number_at(briefing, "net_evsi").unwrap() - 1.7).abs() < EPSILON);
    }

    #[test]
    fn entropy_and_lowest_cost_heuristics_can_choose_zero_value_information() {
        let receipt = evaluate(&fixture()).expect("synthetic VOI registration is valid");
        let random = source(&receipt, "source-random-feed");
        assert!((number_at(random, "gross_evsi").unwrap()).abs() < EPSILON);
        assert!((number_at(random, "net_evsi").unwrap() + 0.2).abs() < EPSILON);
        assert_eq!(
            receipt["strategy_baselines"]["highest_signal_entropy_admissible_source"]["source_id"],
            "source-random-feed"
        );
        assert_eq!(
            receipt["strategy_baselines"]["lowest_cost_admissible_source"]["source_id"],
            "source-random-feed"
        );
        assert_eq!(
            receipt["strategy_baselines"]["highest_net_evsi_positive_source"]["source_id"],
            "source-briefing"
        );
    }

    #[test]
    fn late_source_is_not_actionable_even_when_its_signal_is_highly_informative() {
        let receipt = evaluate(&fixture()).expect("late source is reported, not used");
        let late = source(&receipt, "source-late-review");
        assert_eq!(late["admissible"], false);
        assert_eq!(late["inadmissible_reason"], "arrives_after_decision_deadline");
        assert!(late["gross_evsi"].is_null());
        assert!(late["net_evsi"].is_null());
    }

    #[test]
    fn unnormalized_prior_is_rejected() {
        let mut value = fixture();
        value["states"][0]["prior_probability"] = json!(0.7);
        let errors = evaluate(&value).expect_err("priors must normalize");
        assert!(errors.iter().any(|error| error.contains("sum to")));
    }

    #[test]
    fn likelihood_table_must_normalize_for_every_state() {
        let mut value = fixture();
        value["information_sources"][0]["signals"][1]["state_likelihoods"][0]["probability"] =
            json!(0.8);
        let errors = evaluate(&value).expect_err("conditional signal likelihoods must normalize");
        assert!(errors.iter().any(|error| error.contains("likelihoods for state state-stable sum")));
    }

    #[test]
    fn zero_marginal_probability_signal_is_rejected_without_bayes_division() {
        let mut value = fixture();
        value["information_sources"][1]["signals"][0]["state_likelihoods"][0]["probability"] =
            json!(0.0);
        value["information_sources"][1]["signals"][0]["state_likelihoods"][1]["probability"] =
            json!(0.0);
        value["information_sources"][1]["signals"][1]["state_likelihoods"][0]["probability"] =
            json!(1.0);
        value["information_sources"][1]["signals"][1]["state_likelihoods"][1]["probability"] =
            json!(1.0);
        let errors = evaluate(&value).expect_err("zero-probability observation cannot form a posterior");
        assert!(errors.iter().any(|error| error.contains("zero marginal probability")));
    }

    #[test]
    fn fit_cutoffs_must_precede_the_forecast_origin() {
        let mut value = fixture();
        value["likelihood_fit_cutoff_index"] = json!(10);
        let errors = evaluate(&value).expect_err("future-fitted likelihoods are leakage");
        assert!(errors.iter().any(|error| error.contains("likelihood fit cutoff")));
    }

    #[test]
    fn unknown_likelihood_state_is_rejected() {
        let mut value = fixture();
        value["information_sources"][0]["signals"][0]["state_likelihoods"][0]["state_id"] =
            json!("state-not-registered");
        let errors = evaluate(&value).expect_err("all likelihood cells must map to known states");
        assert!(errors.iter().any(|error| error.contains("unknown state")));
    }

    #[test]
    fn historical_scope_requires_approved_governance_and_non_placeholder_digests() {
        let mut value = fixture();
        value["evidence_scope"] = json!("authorized_historical");
        value["governance"]["data_authority_status"] = json!("authorized");
        value["governance"]["disclosure_review_status"] = json!("approved");
        value["governance"]["independent_source_capture_verified"] = json!(true);
        value["information_sources"][0]["source_status"] = json!("authorized");
        value["information_sources"][1]["source_status"] = json!("authorized");
        value["information_sources"][2]["source_status"] = json!("authorized");
        let errors = evaluate(&value).expect_err("historical source requires authentic pinned digests");
        assert!(errors.iter().any(|error| error.contains("non-placeholder scenario model")));
    }
}
