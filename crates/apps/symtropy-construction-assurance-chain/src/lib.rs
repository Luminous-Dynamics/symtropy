// SPDX-License-Identifier: AGPL-3.0-or-later
//! End-to-end handoff conformance for construction-robot assurance stages.
//! This crate validates scenario contracts; it does not execute robot safety logic.

use serde_json::Value;
use std::collections::{BTreeMap, HashMap, HashSet};

const SCHEMA: &str = include_str!(
    "../../../../research/construction-robot-assurance-chain-v0-1/schema/assurance-chain-scenario-v0.1.schema.json"
);
#[cfg(test)]
const VALID: &str = include_str!(
    "../../../../research/construction-robot-assurance-chain-v0-1/fixtures/valid/valid-synthetic-clear-command-chain.json"
);
#[cfg(test)]
const UNKNOWN: &str = include_str!(
    "../../../../research/construction-robot-assurance-chain-v0-1/fixtures/invalid/unknown-observation-blocks-command.json"
);
#[cfg(test)]
const STALE: &str = include_str!(
    "../../../../research/construction-robot-assurance-chain-v0-1/fixtures/invalid/new-epoch-revokes-stale-lease.json"
);
#[cfg(test)]
const BENCH_FIELD: &str = include_str!(
    "../../../../research/construction-robot-assurance-chain-v0-1/fixtures/invalid/bench-evidence-cannot-support-field-claim.json"
);

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScenarioSummary {
    pub scenario_id: String,
    pub stage_count: usize,
    pub command_count: usize,
    pub claim_count: usize,
    pub external_input_count: usize,
}

fn text<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("{path}.{key} must be a string"))
}

fn array<'a>(value: &'a Value, key: &str, path: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("{path}.{key} must be an array"))
}

fn number(value: &Value, key: &str, path: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{path}.{key} must be a non-negative integer"))
}

fn unique_ids<'a>(items: impl Iterator<Item = Option<&'a str>>, label: &str) -> Result<(), String> {
    let mut seen = HashSet::new();
    for value in items {
        let id = value.ok_or_else(|| format!("{label} contains a missing/non-string id"))?;
        if id.trim().is_empty() || !seen.insert(id.to_owned()) {
            return Err(format!("{label} contains an empty or duplicate id: {id}"));
        }
    }
    Ok(())
}

fn stage_is_ancestor(
    stage_id: &str,
    current: &Value,
    by_id: &HashMap<String, usize>,
    stages: &[Value],
) -> bool {
    let mut pending: Vec<String> = current
        .get("predecessor_stage_ids")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .map(str::to_owned)
        .collect();
    let mut seen = HashSet::new();
    while let Some(candidate) = pending.pop() {
        if candidate == stage_id {
            return true;
        }
        if !seen.insert(candidate.clone()) {
            continue;
        }
        if let Some(index) = by_id.get(&candidate)
            && let Some(parents) = stages[*index]
                .get("predecessor_stage_ids")
                .and_then(Value::as_array)
        {
            pending.extend(parents.iter().filter_map(Value::as_str).map(str::to_owned));
        }
    }
    false
}

fn validate_stage_dispositions(stages: &[Value]) -> Result<(), String> {
    for stage in stages {
        let id = text(stage, "stage_id", "stage")?;
        let kind = text(stage, "stage_kind", &format!("stage[{id}]"))?;
        let disposition = text(stage, "expected_disposition", &format!("stage[{id}]"))?;
        let allowed = match kind {
            "layout_input_admission" => matches!(
                disposition,
                "admissible_for_named_layout_input"
                    | "inadmissible"
                    | "indeterminate"
                    | "not_evaluated"
            ),
            "observation_evaluation" => matches!(
                disposition,
                "verified_clear" | "occupied_or_changed" | "unknown_or_unobserved" | "conflicting"
            ),
            "worksite_epoch_transition" => matches!(
                disposition,
                "active"
                    | "requalification_required"
                    | "safe_hold"
                    | "context_stale"
                    | "conflicting"
                    | "aborted"
            ),
            "qualification_lease" => matches!(
                disposition,
                "active" | "requalification_required" | "safe_hold" | "context_stale" | "aborted"
            ),
            "command_authorization" => matches!(disposition, "authorized" | "rejected"),
            "physical_outcome_receipt" => {
                matches!(disposition, "measured" | "indeterminate" | "not_evaluated")
            }
            "claim_admissibility" => matches!(
                disposition,
                "admissible_for_named_claim"
                    | "inadmissible_for_named_claim"
                    | "indeterminate"
                    | "not_evaluated"
            ),
            other => return Err(format!("stage[{id}] has unsupported stage kind {other}")),
        };
        if !allowed {
            return Err(format!(
                "stage[{id}] disposition {disposition} is not valid for stage kind {kind}"
            ));
        }
    }
    Ok(())
}

fn validate_stage_graph(stages: &[Value]) -> Result<(HashMap<String, usize>, usize), String> {
    if stages.is_empty() {
        return Err("scenario must contain at least one stage".to_owned());
    }
    unique_ids(
        stages
            .iter()
            .map(|stage| stage.get("stage_id").and_then(Value::as_str)),
        "stage ids",
    )?;

    let mut by_id = HashMap::new();
    let mut previous_sequence = None;
    for (index, stage) in stages.iter().enumerate() {
        let id = text(stage, "stage_id", "stage")?.to_owned();
        let sequence = number(stage, "sequence", &format!("stage[{id}]"))?;
        if previous_sequence.is_some_and(|previous| sequence <= previous) {
            return Err(format!(
                "stage sequence must increase strictly; {id} has {sequence}"
            ));
        }
        previous_sequence = Some(sequence);
        by_id.insert(id, index);
    }

    for (index, stage) in stages.iter().enumerate() {
        let id = text(stage, "stage_id", "stage")?;
        for predecessor in array(stage, "predecessor_stage_ids", &format!("stage[{id}]"))? {
            let predecessor = predecessor
                .as_str()
                .ok_or_else(|| format!("stage[{id}] predecessor must be a string"))?;
            let predecessor_index = by_id.get(predecessor).ok_or_else(|| {
                format!("stage[{id}] references unknown predecessor {predecessor}")
            })?;
            if *predecessor_index >= index {
                return Err(format!(
                    "stage[{id}] predecessor {predecessor} must appear earlier"
                ));
            }
        }
    }
    Ok((by_id, stages.len()))
}

fn validate_artifact_lineage(
    stages: &[Value],
    by_id: &HashMap<String, usize>,
) -> Result<usize, String> {
    let mut producers: BTreeMap<String, (String, usize)> = BTreeMap::new();
    for (index, stage) in stages.iter().enumerate() {
        let stage_id = text(stage, "stage_id", "stage")?;
        for artifact in array(stage, "produced_artifacts", &format!("stage[{stage_id}]"))? {
            let artifact_id = text(artifact, "artifact_id", "produced artifact")?.to_owned();
            let digest = text(artifact, "sha256", "produced artifact")?.to_owned();
            if producers
                .insert(artifact_id.clone(), (digest, index))
                .is_some()
            {
                return Err(format!("artifact {artifact_id} has more than one producer"));
            }
        }
    }

    let mut external_inputs = 0;
    for (index, stage) in stages.iter().enumerate() {
        let stage_id = text(stage, "stage_id", "stage")?;
        let predecessors = array(
            stage,
            "predecessor_stage_ids",
            &format!("stage[{stage_id}]"),
        )?;
        for consumed in array(stage, "consumed_artifacts", &format!("stage[{stage_id}]"))? {
            let artifact_id = text(consumed, "artifact_id", "consumed artifact")?;
            let digest = text(consumed, "sha256", "consumed artifact")?;
            match producers.get(artifact_id) {
                Some((produced_digest, producer_index)) => {
                    if produced_digest != digest {
                        return Err(format!(
                            "stage[{stage_id}] consumes artifact {artifact_id} with a digest different from its producer"
                        ));
                    }
                    if *producer_index >= index {
                        return Err(format!(
                            "stage[{stage_id}] consumes artifact {artifact_id} before it is produced"
                        ));
                    }
                    let producer_id = text(&stages[*producer_index], "stage_id", "producer stage")?;
                    if !stage_is_ancestor(producer_id, stage, by_id, stages) {
                        return Err(format!(
                            "stage[{stage_id}] consumes artifact {artifact_id} from a non-ancestor stage"
                        ));
                    }
                }
                None if predecessors.is_empty() => {
                    // Root stages may consume externally supplied, content-addressed inputs.
                    external_inputs += 1;
                }
                None => {
                    return Err(format!(
                        "stage[{stage_id}] consumes unknown artifact {artifact_id} despite having predecessors"
                    ));
                }
            }
        }
    }
    Ok(external_inputs)
}

fn latest_stage_before<'a>(
    stages: &'a [Value],
    kind: &str,
    before_index: usize,
) -> Option<(usize, &'a Value)> {
    stages
        .iter()
        .take(before_index)
        .enumerate()
        .rev()
        .find(|(_, stage)| stage.get("stage_kind").and_then(Value::as_str) == Some(kind))
}

fn referenced_stage<'a>(
    stage_id: &str,
    expected_kind: &str,
    command_stage: &Value,
    command_index: usize,
    by_id: &HashMap<String, usize>,
    stages: &'a [Value],
) -> Result<(usize, &'a Value), String> {
    let index = *by_id
        .get(stage_id)
        .ok_or_else(|| format!("command references unknown stage {stage_id}"))?;
    let stage = &stages[index];
    if stage.get("stage_kind").and_then(Value::as_str) != Some(expected_kind) {
        return Err(format!(
            "command reference {stage_id} is not a {expected_kind} stage"
        ));
    }
    if index >= command_index {
        return Err(format!("command references future stage {stage_id}"));
    }
    if !stage_is_ancestor(stage_id, command_stage, by_id, stages) {
        return Err(format!(
            "command stage does not causally descend from referenced stage {stage_id}"
        ));
    }
    Ok((index, stage))
}

fn require_stage_outputs_consumed(
    source: &Value,
    consumer: &Value,
    source_id: &str,
    consumer_id: &str,
) -> Result<(), String> {
    let outputs = array(source, "produced_artifacts", &format!("stage[{source_id}]"))?;
    let consumed = array(
        consumer,
        "consumed_artifacts",
        &format!("stage[{consumer_id}]"),
    )?;
    if outputs.is_empty() {
        return Err(format!(
            "command stage {consumer_id} references {source_id}, which produced no artifact"
        ));
    }
    for output in outputs {
        let artifact_id = text(output, "artifact_id", "source artifact")?;
        let digest = text(output, "sha256", "source artifact")?;
        let found = consumed.iter().any(|item| {
            item.get("artifact_id").and_then(Value::as_str) == Some(artifact_id)
                && item.get("sha256").and_then(Value::as_str) == Some(digest)
        });
        if !found {
            return Err(format!(
                "command stage {consumer_id} does not consume exact output {artifact_id} from {source_id}"
            ));
        }
    }
    Ok(())
}

fn sorted_reason_codes(value: &Value, path: &str) -> Result<Vec<String>, String> {
    let values = array(value, "expected_reason_codes", path)?;
    let mut result = values
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{path}.expected_reason_codes must contain strings"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    result.sort();
    Ok(result)
}

fn validate_commands(
    scenario: &Value,
    stages: &[Value],
    by_id: &HashMap<String, usize>,
) -> Result<(), String> {
    let command_stages: Vec<&Value> = stages
        .iter()
        .filter(|stage| {
            stage.get("stage_kind").and_then(Value::as_str) == Some("command_authorization")
        })
        .collect();
    let commands = array(scenario, "command_attempts", "scenario")?;
    if command_stages.len() != commands.len() {
        return Err(format!(
            "command stage count {} differs from command attempt count {}",
            command_stages.len(),
            commands.len()
        ));
    }

    let task = scenario.get("task_binding").ok_or("task_binding missing")?;
    let task_scope = text(task, "task_scope", "task_binding")?;
    let configuration = text(task, "robot_configuration_sha256", "task_binding")?;
    let timebase = text(task, "timebase_id", "task_binding")?;
    let mut used_command_stages = HashSet::new();
    let mut previous_command_sequence = None;

    for command in commands {
        let command_id = text(command, "command_id", "command attempt")?;
        let command_sequence = number(command, "sequence", &format!("command[{command_id}]"))?;
        if previous_command_sequence.is_some_and(|previous| command_sequence <= previous) {
            return Err(format!(
                "command attempt sequence must increase strictly; {command_id} has {command_sequence}"
            ));
        }
        previous_command_sequence = Some(command_sequence);

        let command_stage_id = text(
            command,
            "command_stage_id",
            &format!("command[{command_id}]"),
        )?;
        if !used_command_stages.insert(command_stage_id.to_owned()) {
            return Err(format!(
                "multiple command attempts reference stage {command_stage_id}"
            ));
        }
        let command_index = *by_id
            .get(command_stage_id)
            .ok_or_else(|| format!("command {command_id} references unknown stage {command_stage_id}"))?;
        let command_stage = &stages[command_index];
        if command_stage.get("stage_kind").and_then(Value::as_str) != Some("command_authorization") {
            return Err(format!("command {command_id} references a non-command stage"));
        }

        let expected_authorized = command
            .get("expected_authorized")
            .and_then(Value::as_bool)
            .ok_or_else(|| format!("command {command_id} expected_authorized must be boolean"))?;
        let stage_disposition = text(command_stage, "expected_disposition", "command stage")?;
        if expected_authorized != (stage_disposition == "authorized") {
            return Err(format!(
                "command {command_id} expected authorization disagrees with its command stage"
            ));
        }

        let command_observation_id = text(
            command,
            "observation_stage_id",
            &format!("command[{command_id}]"),
        )?;
        let command_epoch_id = text(
            command,
            "epoch_stage_id",
            &format!("command[{command_id}]"),
        )?;
        let latest_observation =
            latest_stage_before(stages, "observation_evaluation", command_index)
        .ok_or_else(|| format!("command {command_id} has no prior observation stage"))?;
        let latest_epoch =
            latest_stage_before(stages, "worksite_epoch_transition", command_index)
        .ok_or_else(|| format!("command {command_id} has no prior epoch stage"))?;
        let latest_observation_id = text(latest_observation.1, "stage_id", "observation stage")?;
        let latest_epoch_id = text(latest_epoch.1, "stage_id", "epoch stage")?;
        if command_observation_id != latest_observation_id {
            return Err(format!(
                "command {command_id} does not reference the latest observation in its causal timeline"
            ));
        }
        if command_epoch_id != latest_epoch_id {
            return Err(format!(
                "command {command_id} does not reference the latest epoch in its causal timeline"
            ));
        }

        let (observation_index, observation_stage) = referenced_stage(
            command_observation_id,
            "observation_evaluation",
            command_stage,
            command_index,
            by_id,
            stages,
        )?;
        let (epoch_index, epoch_stage) = referenced_stage(
            command_epoch_id,
            "worksite_epoch_transition",
            command_stage,
            command_index,
            by_id,
            stages,
        )?;
        require_stage_outputs_consumed(
            observation_stage,
            command_stage,
            command_observation_id,
            command_stage_id,
        )?;
        require_stage_outputs_consumed(
            epoch_stage,
            command_stage,
            command_epoch_id,
            command_stage_id,
        )?;

        let latest_qualification =
            latest_stage_before(stages, "qualification_lease", command_index);
        let command_qualification_id = command
            .get("qualification_stage_id")
            .and_then(Value::as_str);
        let qualification = match (latest_qualification, command_qualification_id) {
            (Some((_, latest)), Some(reference)) => {
                let latest_id = text(latest, "stage_id", "qualification stage")?;
                if latest_id != reference {
                    return Err(format!(
                        "command {command_id} references {reference}, not the latest prior qualification"
                    ));
                }
                let (index, stage) = referenced_stage(
                    reference,
                    "qualification_lease",
                    command_stage,
                    command_index,
                    by_id,
                    stages,
                )?;
                require_stage_outputs_consumed(stage, command_stage, reference, command_stage_id)?;
                Some((index, stage))
            }
            (None, None) => None,
            (Some(_), None) => {
                return Err(format!(
                    "command {command_id} omits the latest prior qualification reference"
                ));
            }
            (None, Some(reference)) => {
                return Err(format!(
                    "command {command_id} references qualification {reference} that is not prior to it"
                ));
            }
        };

        let command_epoch = text(command, "epoch_sha256", "command attempt")?;
        let command_configuration = text(command, "configuration_sha256", "command attempt")?;
        let command_scope = text(command, "task_scope", "command attempt")?;
        let command_timebase = text(command, "timebase_id", "command attempt")?;
        let command_generation = number(
            command,
            "qualification_generation",
            &format!("command[{command_id}]"),
        )?;
        let lease_valid = command
            .get("lease_valid_at_command")
            .and_then(Value::as_bool)
            .ok_or_else(|| {
                format!("command {command_id} lease_valid_at_command must be boolean")
            })?;

        let mut blockers = Vec::new();
        let observation_disposition = text(
            observation_stage,
            "expected_disposition",
            "observation stage",
        )?;
        if observation_disposition != "verified_clear" {
            blockers.push("observation_not_verified_clear");
        }
        let epoch_disposition = text(epoch_stage, "expected_disposition", "epoch stage")?;
        if matches!(
            epoch_disposition,
            "safe_hold" | "context_stale" | "conflicting" | "aborted"
        ) {
            blockers.push("epoch_not_motion_admissible");
        }
        if observation_index >= epoch_index {
            blockers.push("observation_does_not_precede_epoch");
        }
        if let Some((qualification_index, qualification_stage)) = qualification {
            if qualification_index <= epoch_index {
                blockers.push("qualification_predates_current_epoch");
            }
            if text(
                qualification_stage,
                "expected_disposition",
                "qualification stage",
            )? != "active"
            {
                blockers.push("qualification_not_active");
            }
            let recorded_generation = number(
                qualification_stage,
                "qualification_generation",
                "qualification stage",
            )?;
            if recorded_generation != command_generation {
                blockers.push("qualification_generation_mismatch");
            }
        } else {
            blockers.push("qualification_missing");
        }
        if !lease_valid {
            blockers.push("qualification_lease_invalid_at_command");
        }
        if command_epoch != first_output_digest(epoch_stage)? {
            blockers.push("command_epoch_digest_mismatch");
        }
        if command_configuration != configuration {
            blockers.push("command_configuration_mismatch");
        }
        if command_scope != task_scope {
            blockers.push("command_task_scope_mismatch");
        }
        if command_timebase != timebase {
            blockers.push("command_timebase_mismatch");
        }
        blockers.sort_unstable();
        blockers.dedup();

        let mut attempt_reasons = sorted_reason_codes(command, &format!("command[{command_id}]"))?;
        let mut stage_reasons =
            sorted_reason_codes(command_stage, &format!("stage[{command_stage_id}]"))?;
        attempt_reasons.dedup();
        stage_reasons.dedup();
        if attempt_reasons != stage_reasons {
            return Err(format!(
                "command {command_id} reason codes differ from its command stage"
            ));
        }
        let computed_reasons: Vec<String> = blockers.into_iter().map(str::to_owned).collect();
        if attempt_reasons != computed_reasons {
            return Err(format!(
                "command {command_id} declared reason codes do not match its causal blockers; expected {:?}, got {:?}",
                computed_reasons, attempt_reasons
            ));
        }

        if expected_authorized && !computed_reasons.is_empty() {
            return Err(format!(
                "command {command_id} authorizes despite causal blockers: {:?}",
                computed_reasons
            ));
        }
        if !expected_authorized && computed_reasons.is_empty() {
            return Err(format!(
                "command {command_id} is rejected without a causal blocker"
            ));
        }
    }

    if used_command_stages.len() != command_stages.len() {
        return Err(
            "one or more command-authorization stages have no command attempt binding".to_owned(),
        );
    }
    Ok(())
}

fn validate_claims(
    scenario: &Value,
    stages: &[Value],
    by_id: &HashMap<String, usize>,
) -> Result<(), String> {
    let mut claim_ids = HashSet::new();
    for claim in array(scenario, "claim_expectations", "scenario")? {
        let id = text(claim, "claim_id", "claim")?.to_owned();
        if id.trim().is_empty() || !claim_ids.insert(id.clone()) {
            return Err(format!("claim ids contain an empty or duplicate id: {id}"));
        }
        let kind = text(claim, "claim_kind", &format!("claim[{id}]"))?;
        let origin = text(claim, "evidence_origin", &format!("claim[{id}]"))?;
        let scope = text(claim, "intended_scope", &format!("claim[{id}]"))?;
        let decision = text(claim, "expected_decision", &format!("claim[{id}]"))?;

        if matches!(scope, "field" | "cross_site")
            && origin != "field_observed"
            && decision == "admissible_for_named_claim"
        {
            return Err(format!(
                "claim[{id}] promotes {origin} evidence to {scope} scope"
            ));
        }
        if kind == "field_human_detection"
            && scope == "field"
            && origin != "field_observed"
            && decision == "admissible_for_named_claim"
        {
            return Err(format!(
                "field human-detection claim[{id}] has non-field evidence origin"
            ));
        }
        let supporting = array(claim, "supporting_stage_ids", &format!("claim[{id}]"))?;
        for stage_id in supporting {
            let stage_id = stage_id
                .as_str()
                .ok_or_else(|| format!("claim[{id}] stage reference must be string"))?;
            if !by_id.contains_key(stage_id) {
                return Err(format!("claim[{id}] references unknown stage {stage_id}"));
            }
        }

        let claim_stage = stages.iter().find(|stage| {
            stage.get("stage_kind").and_then(Value::as_str) == Some("claim_admissibility")
                && claim
                    .get("supporting_stage_ids")
                    .and_then(Value::as_array)
                    .is_some_and(|ids| {
                        ids.iter()
                            .any(|v| v.as_str() == stage.get("stage_id").and_then(Value::as_str))
                    })
        });
        if let Some(stage) = claim_stage {
            let expected = match decision {
                "admissible_for_named_claim" => "admissible_for_named_claim",
                "inadmissible_for_named_claim" => "inadmissible_for_named_claim",
                "indeterminate" => "indeterminate",
                "not_evaluated" => "not_evaluated",
                other => return Err(format!("claim[{id}] has unknown decision {other}")),
            };
            if text(stage, "expected_disposition", "claim stage")? != expected {
                return Err(format!(
                    "claim[{id}] expected decision differs from its claim stage"
                ));
            }
        }
    }
    Ok(())
}

/// Validate a scenario document against the embedded Draft 2020-12 schema and
/// cross-stage handoff invariants. This is a conformance harness, not the
/// implementation of any upstream safety or evidence evaluator.
pub fn validate_scenario(raw: &str) -> Result<ScenarioSummary, String> {
    let schema: Value = serde_json::from_str(SCHEMA)
        .map_err(|e| format!("embedded scenario schema is invalid JSON: {e}"))?;
    if schema.get("$schema").and_then(Value::as_str)
        != Some("https://json-schema.org/draft/2020-12/schema")
    {
        return Err("scenario schema must declare Draft 2020-12".to_owned());
    }
    jsonschema::draft202012::meta::validate(&schema)
        .map_err(|e| format!("scenario schema failed meta-validation: {e}"))?;
    let validator = jsonschema::draft202012::new(&schema)
        .map_err(|e| format!("cannot compile scenario schema: {e}"))?;
    let value: Value =
        serde_json::from_str(raw).map_err(|e| format!("scenario is invalid JSON: {e}"))?;
    if let Err(errors) = validator.validate(&value) {
        let details: Vec<String> = validator
            .iter_errors(&value)
            .map(|e| format!("{e} at {}", e.instance_path()))
            .collect();
        return Err(if details.is_empty() {
            errors.to_string()
        } else {
            details.join("\n")
        });
    }
    let scenario_id = text(&value, "scenario_id", "scenario")?.to_owned();
    let stages = array(&value, "stages", "scenario")?;
    validate_stage_dispositions(stages)?;
    let (by_id, stage_count) = validate_stage_graph(stages)?;
    let external_input_count = validate_artifact_lineage(stages, &by_id)?;
    validate_commands(&value, stages, &by_id)?;
    validate_claims(&value, stages, &by_id)?;

    let units = array(&value, "statistical_units", "scenario")?;
    unique_ids(
        units
            .iter()
            .map(|unit| unit.get("unit_id").and_then(Value::as_str)),
        "statistical unit ids",
    )?;

    Ok(ScenarioSummary {
        scenario_id,
        stage_count,
        command_count: array(&value, "command_attempts", "scenario")?.len(),
        claim_count: array(&value, "claim_expectations", "scenario")?.len(),
        external_input_count,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_disposition_must_match_stage_kind() {
        let mut scenario: Value = serde_json::from_str(VALID).expect("valid scenario JSON");
        scenario["stages"][1]["expected_disposition"] = serde_json::json!("authorized");
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    const MULTI: &str = include_str!(
        "../../../../research/construction-robot-assurance-chain-v0-1/fixtures/valid/multi-command-frontier-valid-before-later-change.json"
    );

    #[test]
    fn valid_synthetic_chain_is_consistent() {
        let summary = validate_scenario(VALID).expect("valid chain");
        assert_eq!(summary.scenario_id, "valid-synthetic-clear-command-chain");
        assert_eq!(summary.stage_count, 7);
        assert_eq!(summary.command_count, 1);
        assert_eq!(summary.external_input_count, 1);
    }

    #[test]
    fn earlier_command_remains_valid_before_a_later_epoch_change() {
        let summary = validate_scenario(MULTI).expect("multi-command causal frontier");
        assert_eq!(summary.command_count, 2);
    }

    #[test]
    fn command_cannot_reference_a_future_epoch() {
        let mut scenario: Value = serde_json::from_str(MULTI).expect("multi-command scenario");
        scenario["command_attempts"][0]["epoch_stage_id"] = serde_json::json!("epoch-2");
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn command_cannot_authorize_with_qualification_from_before_latest_epoch() {
        let mut scenario: Value = serde_json::from_str(MULTI).expect("multi-command scenario");
        scenario["command_attempts"][1]["expected_authorized"] = serde_json::json!(true);
        scenario["command_attempts"][1]["lease_valid_at_command"] = serde_json::json!(true);
        scenario["command_attempts"][1]["expected_reason_codes"] = serde_json::json!([]);
        scenario["stages"][7]["expected_disposition"] = serde_json::json!("authorized");
        scenario["stages"][7]["expected_reason_codes"] = serde_json::json!([]);
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn unknown_observation_blocks_command() {
        let summary = validate_scenario(UNKNOWN).expect("well-formed fail-closed scenario");
        assert_eq!(summary.scenario_id, "unknown-observation-blocks-command");
    }

    #[test]
    fn new_epoch_after_qualification_revokes_old_lease() {
        let summary = validate_scenario(STALE).expect("well-formed stale-lease scenario");
        assert_eq!(summary.scenario_id, "new-epoch-revokes-stale-lease");
    }

    #[test]
    fn bench_evidence_cannot_be_declared_admissible_for_field_claim() {
        let summary = validate_scenario(BENCH_FIELD).expect("well-formed claim-scope scenario");
        assert_eq!(
            summary.scenario_id,
            "bench-evidence-cannot-support-field-human-detection"
        );
    }

    #[test]
    fn command_reason_codes_must_match_computed_causal_blockers_and_stage_receipt() {
        let mut scenario: Value = serde_json::from_str(VALID).expect("valid scenario JSON");
        scenario["command_attempts"][0]["expected_reason_codes"] =
            serde_json::json!(["qualification_missing"]);
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn consumed_artifact_cannot_change_digest() {
        let mut scenario: Value = serde_json::from_str(VALID).expect("valid scenario JSON");
        scenario["stages"][1]["consumed_artifacts"][0]["sha256"] =
            serde_json::json!("0".repeat(64));
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn consumed_artifact_must_be_from_an_ancestor_stage() {
        let mut scenario: Value = serde_json::from_str(VALID).expect("valid scenario JSON");
        scenario["stages"][1]["predecessor_stage_ids"] = serde_json::json!([]);
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn new_epoch_after_qualification_rejects_the_old_lease() {
        let mut scenario: Value = serde_json::from_str(STALE).expect("stale scenario JSON");
        scenario["command_attempts"][0]["expected_authorized"] = serde_json::json!(true);
        scenario["command_attempts"][0]["lease_valid_at_command"] = serde_json::json!(true);
        scenario["command_attempts"][0]["expected_reason_codes"] = serde_json::json!([]);
        scenario["stages"][5]["expected_disposition"] = serde_json::json!("authorized");
        scenario["stages"][5]["expected_reason_codes"] = serde_json::json!([]);
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn command_cannot_use_wrong_configuration_digest() {
        let mut scenario: Value = serde_json::from_str(VALID).expect("valid scenario JSON");
        scenario["command_attempts"][0]["configuration_sha256"] = serde_json::json!("0".repeat(64));
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn field_claim_cannot_be_promoted_from_bench_evidence() {
        let mut scenario: Value = serde_json::from_str(BENCH_FIELD).expect("scenario JSON");
        scenario["claim_expectations"][0]["expected_decision"] =
            serde_json::json!("admissible_for_named_claim");
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }

    #[test]
    fn duplicate_stage_ids_are_rejected() {
        let mut scenario: Value = serde_json::from_str(VALID).expect("valid scenario JSON");
        scenario["stages"][1]["stage_id"] = serde_json::json!("input");
        assert!(validate_scenario(&scenario.to_string()).is_err());
    }
}
