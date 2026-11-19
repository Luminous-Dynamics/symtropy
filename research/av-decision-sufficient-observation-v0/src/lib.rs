// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Independent known-answer audit for the finite AV decision-value fixtures.
//!
//! This crate calculates EIG and EVSI for the declared finite synthetic
//! observation models. It is not an acquisition planner and must be run only
//! in the evaluator environment, never in the agent process.

#![forbid(unsafe_code)]
#![warn(missing_docs)]

use std::collections::{BTreeMap, BTreeSet};

use serde_json::{json, Value};

const BENCHMARK_ID: &str = "av-decision-sufficient-observation-v0";
const TOLERANCE: f64 = 1.0e-9;

fn required<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing required field {key:?}"))
}

fn string(value: &Value, context: &str) -> Result<String, String> {
    value
        .as_str()
        .map(str::to_owned)
        .ok_or_else(|| format!("{context} must be a string"))
}

fn number(value: &Value, context: &str) -> Result<f64, String> {
    value
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("{context} must be a finite number"))
}

fn array<'a>(value: &'a Value, context: &str) -> Result<&'a [Value], String> {
    value
        .as_array()
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{context} must be an array"))
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= TOLERANCE
}

const AGENT_FORBIDDEN_KEYS: &[&str] = &[
    "world_truth",
    "evaluator_truth",
    "realized_hypothesis_id",
    "realized_outcomes",
    "actual_dependency_groups",
    "actual_arrival_tick",
    "reference_transcript",
    "oracle_label",
    "hidden_state",
    "future_frames",
    "future_observations",
];

fn reject_agent_truth_fields(value: &Value, path: &str) -> Result<(), String> {
    match value {
        Value::Object(object) => {
            for (key, child) in object {
                if AGENT_FORBIDDEN_KEYS.contains(&key.as_str()) {
                    return Err(format!("evaluator-only field {key:?} leaked into agent data at {path}"));
                }
                reject_agent_truth_fields(child, &format!("{path}.{key}"))?;
            }
        }
        Value::Array(items) => {
            for (index, child) in items.iter().enumerate() {
                reject_agent_truth_fields(child, &format!("{path}[{index}]"))?;
            }
        }
        _ => {}
    }
    Ok(())
}

fn entropy(probabilities: impl Iterator<Item = f64>) -> f64 {
    probabilities
        .filter(|p| *p > 0.0)
        .map(|p| -p * p.ln())
        .sum()
}

#[derive(Clone, Debug)]
struct ObservationEvaluation {
    eig_nats: f64,
    current_risk: f64,
    post_observation_risk: f64,
    evsi: f64,
    best_current_decision: String,
}

fn best_decision(
    posterior: &[f64],
    losses: &[Vec<f64>],
    decisions: &[String],
) -> Result<(String, f64), String> {
    if posterior.len() != losses.len() || posterior.is_empty() {
        return Err("posterior/loss row count mismatch".to_owned());
    }
    let decision_count = decisions.len();
    if decision_count == 0 || losses.iter().any(|row| row.len() != decision_count) {
        return Err("decision/loss matrix dimensions are invalid".to_owned());
    }

    let mut best_index = 0;
    let mut best_risk = f64::INFINITY;
    for decision_index in 0..decision_count {
        let risk = posterior
            .iter()
            .zip(losses.iter())
            .map(|(probability, row)| probability * row[decision_index])
            .sum::<f64>();
        if risk < best_risk - TOLERANCE {
            best_index = decision_index;
            best_risk = risk;
        }
    }
    Ok((decisions[best_index].clone(), best_risk))
}

fn parse_decision_problem(
    episode: &Value,
) -> Result<(Vec<String>, Vec<f64>, Vec<String>, Vec<Vec<f64>>), String> {
    let hypotheses = array(required(required(episode, "public_belief")?, "hypotheses")?, "hypotheses")?;
    let decisions = array(
        required(required(episode, "task")?, "terminal_decisions")?,
        "terminal_decisions",
    )?;
    let loss_rows = array(required(episode, "decision_loss")?, "decision_loss")?;

    let mut hypothesis_ids = Vec::with_capacity(hypotheses.len());
    let mut priors = Vec::with_capacity(hypotheses.len());
    let mut prior_total = 0.0;
    for hypothesis in hypotheses {
        let id = string(required(hypothesis, "id")?, "hypothesis.id")?;
        if hypothesis_ids.contains(&id) {
            return Err(format!("duplicate hypothesis ID {id:?}"));
        }
        let prior = number(required(hypothesis, "prior")?, "hypothesis.prior")?;
        if !(0.0..=1.0).contains(&prior) {
            return Err(format!("prior for {id:?} is outside [0, 1]"));
        }
        hypothesis_ids.push(id);
        priors.push(prior);
        prior_total += prior;
    }
    if !close(prior_total, 1.0) {
        return Err(format!("hypothesis priors sum to {prior_total}, not 1"));
    }

    let decision_ids: Vec<String> = decisions
        .iter()
        .map(|value| string(value, "terminal_decision"))
        .collect::<Result<_, _>>()?;
    if decision_ids.is_empty() {
        return Err("terminal decision catalogue is empty".to_owned());
    }

    let mut losses = vec![vec![f64::NAN; decision_ids.len()]; hypothesis_ids.len()];
    for row in loss_rows {
        let hypothesis_id = string(required(row, "hypothesis_id")?, "loss.hypothesis_id")?;
        let decision_id = string(required(row, "decision_id")?, "loss.decision_id")?;
        let hypothesis_index = hypothesis_ids
            .iter()
            .position(|candidate| candidate == &hypothesis_id)
            .ok_or_else(|| format!("loss references unknown hypothesis {hypothesis_id:?}"))?;
        let decision_index = decision_ids
            .iter()
            .position(|candidate| candidate == &decision_id)
            .ok_or_else(|| format!("loss references unknown decision {decision_id:?}"))?;
        let loss = number(required(row, "loss")?, "loss.value")?;
        if loss < 0.0 {
            return Err("decision losses must be non-negative".to_owned());
        }
        if losses[hypothesis_index][decision_index].is_finite() {
            return Err(format!("duplicate loss cell ({hypothesis_id:?}, {decision_id:?})"));
        }
        losses[hypothesis_index][decision_index] = loss;
    }
    if losses.iter().flatten().any(|loss| !loss.is_finite()) {
        return Err("decision loss matrix is incomplete".to_owned());
    }

    Ok((hypothesis_ids, priors, decision_ids, losses))
}

fn evaluate_request(
    request: &Value,
    hypothesis_ids: &[String],
    priors: &[f64],
    decisions: &[String],
    losses: &[Vec<f64>],
) -> Result<ObservationEvaluation, String> {
    let rows = array(
        required(request, "predicted_observation_model")?,
        "predicted_observation_model",
    )?;
    if rows.len() != hypothesis_ids.len() {
        return Err("observation model does not cover every hypothesis".to_owned());
    }

    let mut conditional_by_hypothesis: Vec<BTreeMap<String, f64>> =
        Vec::with_capacity(hypothesis_ids.len());
    for (index, row) in rows.iter().enumerate() {
        let row_id = string(required(row, "hypothesis_id")?, "observation.hypothesis_id")?;
        if row_id != hypothesis_ids[index] {
            return Err(format!(
                "observation rows must match hypothesis order: expected {:?}, got {:?}",
                hypothesis_ids[index], row_id
            ));
        }
        let outcomes = array(required(row, "outcomes")?, "observation.outcomes")?;
        let mut probabilities = BTreeMap::new();
        let mut total = 0.0;
        for outcome in outcomes {
            let label = string(required(outcome, "outcome")?, "outcome.label")?;
            let probability = number(required(outcome, "probability")?, "outcome.probability")?;
            if !(0.0..=1.0).contains(&probability) {
                return Err(format!("outcome probability for {label:?} is outside [0, 1]"));
            }
            if probabilities.insert(label.clone(), probability).is_some() {
                return Err(format!("duplicate outcome {label:?} for hypothesis {row_id:?}"));
            }
            total += probability;
        }
        if !close(total, 1.0) {
            return Err(format!(
                "outcome probabilities for {row_id:?} sum to {total}, not 1"
            ));
        }
        conditional_by_hypothesis.push(probabilities);
    }

    let mut outcome_labels = BTreeSet::new();
    for row in &conditional_by_hypothesis {
        outcome_labels.extend(row.keys().cloned());
    }
    let mut marginal = BTreeMap::new();
    for label in &outcome_labels {
        let probability = priors
            .iter()
            .zip(conditional_by_hypothesis.iter())
            .map(|(prior, row)| prior * row.get(label).copied().unwrap_or(0.0))
            .sum::<f64>();
        marginal.insert(label.clone(), probability);
    }
    if !close(marginal.values().sum::<f64>(), 1.0) {
        return Err("marginal observation outcomes do not sum to 1".to_owned());
    }

    let outcome_entropy = entropy(marginal.values().copied());
    let conditional_entropy = priors
        .iter()
        .zip(conditional_by_hypothesis.iter())
        .map(|(prior, row)| prior * entropy(row.values().copied()))
        .sum::<f64>();
    let eig_nats = (outcome_entropy - conditional_entropy).max(0.0);

    let (best_current_decision, current_risk) = best_decision(priors, losses, decisions)?;
    let mut post_observation_risk = 0.0;
    for (label, outcome_probability) in &marginal {
        if *outcome_probability <= 0.0 {
            continue;
        }
        let posterior: Vec<f64> = priors
            .iter()
            .zip(conditional_by_hypothesis.iter())
            .map(|(prior, row)| {
                prior * row.get(label).copied().unwrap_or(0.0) / outcome_probability
            })
            .collect();
        let (_, branch_risk) = best_decision(&posterior, losses, decisions)?;
        post_observation_risk += outcome_probability * branch_risk;
    }
    let evsi = current_risk - post_observation_risk;

    Ok(ObservationEvaluation {
        eig_nats,
        current_risk,
        post_observation_risk,
        evsi,
        best_current_decision,
    })
}

fn assert_number(
    object: &Value,
    field: &str,
    actual: f64,
    context: &str,
) -> Result<(), String> {
    let expected = number(required(object, field)?, &format!("{context}.{field}"))?;
    if !close(actual, expected) {
        return Err(format!(
            "{context}.{field}: expected {expected:.12}, calculated {actual:.12}"
        ));
    }
    Ok(())
}

fn audit_oracle_references(
    agent_episodes: &[Value],
    oracle_episodes: &[Value],
) -> Result<(), String> {
    for (agent_episode, oracle_episode) in agent_episodes.iter().zip(oracle_episodes.iter()) {
        let episode_id = string(
            required(agent_episode, "episode_id")?,
            "agent.episode_id",
        )?;
        let realized_hypothesis =
            string(required(oracle_episode, "realized_hypothesis_id")?, "oracle.realized_hypothesis_id")?;
        let public_hypotheses = array(
            required(required(agent_episode, "public_belief")?, "hypotheses")?,
            "agent.public_belief.hypotheses",
        )?;
        if !public_hypotheses.iter().any(|hypothesis| {
            hypothesis.get("id").and_then(Value::as_str) == Some(realized_hypothesis.as_str())
        }) {
            return Err(format!(
                "oracle realized hypothesis {realized_hypothesis:?} is absent from agent catalogue in {episode_id}"
            ));
        }

        let requests = array(
            required(agent_episode, "candidate_requests")?,
            "agent.candidate_requests",
        )?;
        if let Some(outcomes) = oracle_episode.get("realized_outcomes").and_then(Value::as_object) {
            for (request_id, realized_outcome) in outcomes {
                let request = requests
                    .iter()
                    .find(|request| {
                        request.get("request_id").and_then(Value::as_str) == Some(request_id)
                    })
                    .ok_or_else(|| {
                        format!("oracle outcome references unknown request {request_id:?} in {episode_id}")
                    })?;
                let outcome_label = string(realized_outcome, "oracle.realized_outcome")?;
                let intentionally_open_world = episode_id
                    == "av-dso-007-uncalibrated-incomplete-outcome-partition"
                    && outcome_label == "unmodeled_outcome";
                let rows = array(
                    required(request, "predicted_observation_model")?,
                    "request.predicted_observation_model",
                )?;
                let realized_row = rows.iter().find(|row| {
                    row.get("hypothesis_id").and_then(Value::as_str)
                        == Some(realized_hypothesis.as_str())
                });
                let labels_realized_outcome = match realized_row {
                    Some(row) => array(
                        required(row, "outcomes")?,
                        "request.outcomes",
                    )?
                    .iter()
                    .any(|outcome| {
                        outcome.get("outcome").and_then(Value::as_str) == Some(outcome_label.as_str())
                            && outcome.get("probability").and_then(Value::as_f64).unwrap_or(0.0) > 0.0
                    }),
                    None => false,
                };
                if !labels_realized_outcome && !intentionally_open_world {
                    return Err(format!(
                        "oracle outcome {outcome_label:?} is unsupported under realized hypothesis for {request_id:?}"
                    ));
                }
            }
        }

        if let Some(actual_arrivals) = oracle_episode.get("actual_arrival_tick").and_then(Value::as_object) {
            for (request_id, actual_tick) in actual_arrivals {
                let actual_tick = actual_tick.as_u64().ok_or_else(|| {
                    format!("oracle arrival tick for {request_id:?} must be an integer")
                })?;
                let request = requests
                    .iter()
                    .find(|request| {
                        request.get("request_id").and_then(Value::as_str) == Some(request_id)
                    })
                    .ok_or_else(|| format!("unknown arrival request {request_id:?}"))?;
                let declared_tick = required(request, "arrival_tick")?.as_u64().ok_or_else(|| {
                    format!("agent arrival tick for {request_id:?} must be an integer")
                })?;
                if declared_tick != actual_tick {
                    return Err(format!("agent/evaluator arrival tick mismatch for {request_id:?}"));
                }
            }
        }

        if let Some(permission_truth) =
            oracle_episode.get("permission_truth").and_then(Value::as_object)
        {
            for (request_id, truth) in permission_truth {
                let truth = string(truth, "oracle.permission_truth")?;
                let request = requests
                    .iter()
                    .find(|request| {
                        request.get("request_id").and_then(Value::as_str) == Some(request_id)
                    })
                    .ok_or_else(|| format!("unknown permission request {request_id:?}"))?;
                let declared = string(required(request, "permission")?, "request.permission")?;
                let expected_permission = if truth == "not_permitted" {
                    "denied"
                } else {
                    truth.as_str()
                };
                if declared != expected_permission {
                    return Err(format!("agent/evaluator permission mismatch for {request_id:?}"));
                }
            }
        }

        if let Some(dependency_truth) =
            oracle_episode.get("actual_dependency_groups").and_then(Value::as_object)
        {
            let current_evidence = array(
                required(agent_episode, "current_evidence")?,
                "agent.current_evidence",
            )?;
            for (source_id, actual_group) in dependency_truth {
                let actual_group = string(actual_group, "oracle.dependency_group")?;
                let declared_group = current_evidence
                    .iter()
                    .find(|evidence| {
                        evidence.get("evidence_ref").and_then(Value::as_str) == Some(source_id)
                    })
                    .and_then(|evidence| evidence.get("dependency_group"))
                    .and_then(Value::as_str)
                    .or_else(|| {
                        requests
                            .iter()
                            .find(|request| {
                                request.get("request_id").and_then(Value::as_str) == Some(source_id)
                            })
                            .and_then(|request| request.get("dependency_group"))
                            .and_then(Value::as_str)
                    });
                if declared_group != Some(actual_group.as_str()) {
                    return Err(format!(
                        "agent/evaluator dependency-group mismatch for {source_id:?}"
                    ));
                }
            }
        }
    }
    Ok(())
}

fn check_episode_ids(
    agent_episodes: &[Value],
    oracle_episodes: &[Value],
    expected_episodes: &[Value],
) -> Result<(), String> {
    if agent_episodes.len() != oracle_episodes.len()
        || agent_episodes.len() != expected_episodes.len()
    {
        return Err("agent/oracle/expected episode counts differ".to_owned());
    }
    for index in 0..agent_episodes.len() {
        let agent_id = string(required(&agent_episodes[index], "episode_id")?, "agent.episode_id")?;
        let oracle_id = string(required(&oracle_episodes[index], "episode_id")?, "oracle.episode_id")?;
        let expected_id = string(
            required(&expected_episodes[index], "episode_id")?,
            "expected.episode_id",
        )?;
        if agent_id != oracle_id || agent_id != expected_id {
            return Err(format!("episode ID mismatch at row {index}: {agent_id:?}"));
        }
    }
    Ok(())
}

/// Audit agent/evaluator separation metadata and recompute finite EIG/EVSI known answers.
///
/// This is a benchmark fixture verifier, not a request-selection policy. The hidden
/// evaluator file must be supplied only to this separate evaluator-side process.
pub fn audit_fixtures(
    agent: &Value,
    oracle: &Value,
    expected: &Value,
) -> Result<Value, String> {
    for (name, root) in [("agent", agent), ("oracle", oracle), ("expected", expected)] {
        if required(root, "schema_version")?.as_u64() != Some(1) {
            return Err(format!("{name} fixture has an unsupported schema version"));
        }
        if string(required(root, "benchmark_id")?, &format!("{name}.benchmark_id"))?
            != BENCHMARK_ID
        {
            return Err(format!("{name} fixture has the wrong benchmark ID"));
        }
    }

    reject_agent_truth_fields(agent, "agent")?;
    reject_agent_truth_fields(expected, "expected")?;

    let agent_episodes = array(required(agent, "episodes")?, "agent.episodes")?;
    let oracle_episodes = array(required(oracle, "episodes")?, "oracle.episodes")?;
    let expected_episodes = array(required(expected, "episodes")?, "expected.episodes")?;
    check_episode_ids(agent_episodes, oracle_episodes, expected_episodes)?;
    audit_oracle_references(agent_episodes, oracle_episodes)?;

    if agent_episodes.len() != 7 {
        return Err(format!(
            "expected seven frozen episodes, found {}",
            agent_episodes.len()
        ));
    }

    let first = &agent_episodes[0];
    let expected_first = &expected_episodes[0];
    let (hypothesis_ids, priors, decisions, losses) = parse_decision_problem(first)?;
    let (current_decision, current_risk) = best_decision(&priors, &losses, &decisions)?;
    let stated_decision = string(required(first, "current_best_decision")?, "current_best_decision")?;
    if current_decision != stated_decision {
        return Err(format!(
            "current best decision mismatch: stated {stated_decision:?}, calculated {current_decision:?}"
        ));
    }
    assert_number(first, "current_expected_loss", current_risk, "agent.episode[0]")?;
    if string(
        required(expected_first, "current_best_decision")?,
        "expected.current_best_decision",
    )? != current_decision
    {
        return Err("known-answer current decision differs from calculated decision".to_owned());
    }
    assert_number(
        expected_first,
        "current_expected_loss",
        current_risk,
        "expected.episode[0]",
    )?;

    let request_array = array(required(first, "candidate_requests")?, "candidate_requests")?;
    let mut calculated_requests = Vec::with_capacity(request_array.len());
    for request in request_array {
        let request_id = string(required(request, "request_id")?, "request_id")?;
        let result = evaluate_request(request, &hypothesis_ids, &priors, &decisions, &losses)?;
        let expected_request = required(expected_first, &request_id)?;
        assert_number(expected_request, "eig_nats", result.eig_nats, &request_id)?;
        assert_number(
            expected_request,
            "post_expected_loss",
            result.post_observation_risk,
            &request_id,
        )?;
        assert_number(expected_request, "evsi", result.evsi, &request_id)?;
        let utility_cost = number(
            required(required(request, "cost")?, "utility_cost")?,
            &format!("{request_id}.cost.utility_cost"),
        )?;
        let net_evsi = result.evsi - utility_cost;
        assert_number(expected_request, "net_evsi", net_evsi, &request_id)?;
        calculated_requests.push(json!({
            "request_id": request_id,
            "eig_nats": result.eig_nats,
            "current_expected_loss": result.current_risk,
            "post_expected_loss": result.post_observation_risk,
            "evsi": result.evsi,
            "utility_cost": utility_cost,
            "net_evsi": net_evsi
        }));
    }

    if calculated_requests.len() != 2 {
        return Err("episode 1 must contain the two paired EIG/EVSI counterexample requests".to_owned());
    }

    let subtype_eig = number(
        required(&calculated_requests[0], "eig_nats")?,
        "subtype_detail.eig_nats",
    )?;
    let hazard_eig = number(
        required(&calculated_requests[1], "eig_nats")?,
        "hazard_cue.eig_nats",
    )?;
    let subtype_net = number(
        required(&calculated_requests[0], "net_evsi")?,
        "subtype_detail.net_evsi",
    )?;
    let hazard_net = number(
        required(&calculated_requests[1], "net_evsi")?,
        "hazard_cue.net_evsi",
    )?;
    if !(subtype_eig > hazard_eig && hazard_net > subtype_net && hazard_net > 0.0) {
        return Err("high-EIG/zero-EVSI versus lower-EIG/positive-EVSI counterexample did not hold".to_owned());
    }

    // Robust stop: both hypotheses must independently prefer the same terminal action.
    let robust_stop = &agent_episodes[1];
    let (ids, priors, decisions, losses) = parse_decision_problem(robust_stop)?;
    let mut per_hypothesis_actions = Vec::with_capacity(ids.len());
    for index in 0..ids.len() {
        let mut point_prior = vec![0.0; ids.len()];
        point_prior[index] = 1.0;
        per_hypothesis_actions.push(best_decision(&point_prior, &losses, &decisions)?.0);
    }
    if per_hypothesis_actions.iter().collect::<BTreeSet<_>>().len() != 1 {
        return Err("robust-stop fixture does not have one common terminal action".to_owned());
    }
    let robust_requests = array(required(robust_stop, "candidate_requests")?, "robust_stop.requests")?;
    for request in robust_requests {
        let result = evaluate_request(request, &ids, &priors, &decisions, &losses)?;
        if !close(result.evsi, 0.0) {
            return Err("robust-stop request unexpectedly has positive EVSI".to_owned());
        }
    }

    // A request arriving after the deadline cannot improve the current response window.
    let late = &agent_episodes[2];
    let deadline = required(required(late, "task")?, "decision_deadline_tick")?
        .as_u64()
        .ok_or_else(|| "late episode deadline must be an integer".to_owned())?;
    let late_requests = array(required(late, "candidate_requests")?, "late.requests")?;
    for request in late_requests {
        if let Some(arrival) = request.get("arrival_tick").and_then(Value::as_u64) {
            if arrival <= deadline {
                return Err("late-arrival fixture does not arrive after its deadline".to_owned());
            }
        }
    }

    // Permission and resource limits are hard gates, not utility penalties.
    let budget_episode = &agent_episodes[3];
    let budget = required(budget_episode, "request_budget")?;
    let max_latency = number(required(budget, "max_latency_ticks")?, "max_latency_ticks")?;
    let max_bytes = number(required(budget, "max_bytes")?, "max_bytes")?;
    let max_energy = number(required(budget, "max_energy_units")?, "max_energy_units")?;
    let budget_requests = array(required(budget_episode, "candidate_requests")?, "budget.requests")?;
    let mut saw_ineligible_over_budget = false;
    for request in budget_requests {
        let cost = required(request, "cost")?;
        let exceeds = number(required(cost, "latency_ticks")?, "latency")? > max_latency
            || number(required(cost, "bytes")?, "bytes")? > max_bytes
            || number(required(cost, "energy_units")?, "energy")? > max_energy;
        if exceeds {
            saw_ineligible_over_budget = true;
        }
    }
    if !saw_ineligible_over_budget {
        return Err("budget fixture contains no request excluded by its hard budget".to_owned());
    }

    let privacy_requests = array(
        required(&agent_episodes[4], "candidate_requests")?,
        "privacy.requests",
    )?;
    if !privacy_requests.iter().any(|request| {
        request.get("permission").and_then(Value::as_str) == Some("denied")
    }) {
        return Err("privacy fixture has no explicitly denied request".to_owned());
    }

    let dependence_episode = &agent_episodes[5];
    let observed_group = string(
        required(
            array(required(dependence_episode, "current_evidence")?, "current_evidence")?
                .first()
                .ok_or_else(|| "dependence fixture needs current evidence".to_owned())?,
            "dependency_group",
        )?,
        "current_evidence.dependency_group",
    )?;
    let derived_request = array(
        required(dependence_episode, "candidate_requests")?,
        "dependence.requests",
    )?
    .first()
    .ok_or_else(|| "dependence fixture needs a derived request".to_owned())?;
    let derived_group = string(
        required(derived_request, "dependency_group")?,
        "derived_request.dependency_group",
    )?;
    if observed_group != derived_group
        || derived_request.get("derived_from_evidence").is_none()
    {
        return Err("common-cause dependency is not explicitly represented".to_owned());
    }

    let uncalibrated = &agent_episodes[6];
    if required(required(uncalibrated, "public_belief")?, "status")?.as_str()
        != Some("uncalibrated")
    {
        return Err("open-world fixture must declare uncalibrated belief status".to_owned());
    }
    let open_hypothesis = array(
        required(required(uncalibrated, "public_belief")?, "hypotheses")?,
        "uncalibrated.hypotheses",
    )?
    .iter()
    .any(|h| h.get("id").and_then(Value::as_str) == Some("open_world_other"));
    let uncalibrated_request = array(
        required(uncalibrated, "candidate_requests")?,
        "uncalibrated.requests",
    )?
    .first()
    .ok_or_else(|| "uncalibrated episode needs a request".to_owned())?;
    let model_has_open_hypothesis = array(
        required(uncalibrated_request, "predicted_observation_model")?,
        "uncalibrated observation model",
    )?
    .iter()
    .any(|row| row.get("hypothesis_id").and_then(Value::as_str) == Some("open_world_other"));
    if !open_hypothesis || model_has_open_hypothesis {
        return Err("uncalibrated fixture must expose an omitted open-world outcome".to_owned());
    }

    Ok(json!({
        "schema_version": 1,
        "benchmark_id": BENCHMARK_ID,
        "status": "fixture_integrity_and_finite_known_answers_pass",
        "claim_ceiling": "synthetic fixture arithmetic and metadata only",
        "episode_count": agent_episodes.len(),
        "verified": {
            "episode_id_alignment": true,
            "complete_finite_decision_loss_matrix": true,
            "high_eig_zero_evsi_vs_lower_eig_positive_evsi": true,
            "robust_stop_with_residual_uncertainty": true,
            "late_observation_after_deadline": true,
            "hard_budget_exclusion": true,
            "permission_denial": true,
            "common_cause_dependency": true,
            "uncalibrated_open_world_incompleteness": true
        },
        "episode_001_requests": calculated_requests
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixtures() -> (Value, Value, Value) {
        let agent: Value = serde_json::from_str(include_str!(
            "../fixtures/agent/agent-inputs-v1.json"
        ))
        .unwrap();
        let oracle: Value = serde_json::from_str(include_str!(
            "../fixtures/evaluator-only/evaluator-oracle-v1.json"
        ))
        .unwrap();
        let expected: Value = serde_json::from_str(include_str!(
            "../fixtures/expected/known-answer-scorecard-v1.json"
        ))
        .unwrap();
        (agent, oracle, expected)
    }

    #[test]
    fn all_frozen_fixtures_pass_the_independent_known_answer_audit() {
        let (agent, oracle, expected) = fixtures();
        let report = audit_fixtures(&agent, &oracle, &expected).unwrap();
        assert_eq!(
            report.get("status").and_then(Value::as_str),
            Some("fixture_integrity_and_finite_known_answers_pass")
        );
        assert_eq!(
            report.get("episode_count").and_then(Value::as_u64),
            Some(7)
        );
    }

    #[test]
    fn the_lower_eig_request_has_the_higher_positive_net_evsi() {
        let (agent, oracle, expected) = fixtures();
        let report = audit_fixtures(&agent, &oracle, &expected).unwrap();
        let requests = report.get("episode_001_requests").unwrap().as_array().unwrap();
        let subtype = &requests[0];
        let cue = &requests[1];
        assert!(
            subtype["eig_nats"].as_f64().unwrap()
                > cue["eig_nats"].as_f64().unwrap()
        );
        assert!(
            cue["net_evsi"].as_f64().unwrap()
                > subtype["net_evsi"].as_f64().unwrap()
        );
        assert!(cue["net_evsi"].as_f64().unwrap() > 0.0);
    }

    #[test]
    fn evaluator_fixture_is_required_but_never_used_as_agent_policy_input() {
        let (agent, oracle, expected) = fixtures();
        assert!(oracle["episodes"][0].get("realized_hypothesis_id").is_some());
        assert!(agent["episodes"][0].get("realized_hypothesis_id").is_none());
        assert!(expected["episodes"][0].get("realized_hypothesis_id").is_none());
        audit_fixtures(&agent, &oracle, &expected).unwrap();
    }

    #[test]
    fn evaluator_only_fields_are_rejected_from_agent_and_expected_fixtures() {
        let (mut agent, oracle, expected) = fixtures();
        agent["episodes"][0]["realized_hypothesis_id"] = Value::String("H1".to_owned());
        assert!(audit_fixtures(&agent, &oracle, &expected).is_err());

        let (agent, oracle, mut expected) = fixtures();
        expected["episodes"][0]["world_truth"] = json!({"entity_id":"hidden-entity"});
        assert!(audit_fixtures(&agent, &oracle, &expected).is_err());
    }

    #[test]
    fn incomplete_outcome_partition_must_remain_out_of_quantified_lane() {
        let (agent, oracle, expected) = fixtures();
        let report = audit_fixtures(&agent, &oracle, &expected).unwrap();
        assert_eq!(report["verified"]["uncalibrated_open_world_incompleteness"], true);
        assert_eq!(
            agent["episodes"][6]["public_belief"]["status"].as_str(),
            Some("uncalibrated")
        );
    }

    #[test]
    fn permission_and_budget_controls_are_separate_gates() {
        let (agent, oracle, expected) = fixtures();
        let report = audit_fixtures(&agent, &oracle, &expected).unwrap();
        assert_eq!(report["verified"]["permission_denial"], true);
        assert_eq!(report["verified"]["hard_budget_exclusion"], true);
        assert_eq!(
            agent["episodes"][4]["candidate_requests"][0]["permission"].as_str(),
            Some("denied")
        );
    }
}
