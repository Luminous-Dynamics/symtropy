use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, process};

const MAX_ORACLE_TICK: u64 = 256;

#[derive(Clone, Debug, Serialize)]
struct OracleDecision {
    admitted: bool,
    next_state: Option<String>,
    reason_codes: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
struct ScenarioCheck {
    scenario_id: String,
    fixture_label_admitted: bool,
    candidate_admitted: bool,
    oracle_admitted: bool,
    candidate_next_state: Option<String>,
    oracle_next_state: Option<String>,
    candidate_reason_codes: Vec<String>,
    oracle_reason_codes: Vec<String>,
    decision_matches_oracle: bool,
    label_matches_oracle: bool,
}

#[derive(Clone, Debug, Serialize)]
struct OracleReport {
    schema_version: String,
    classification: String,
    fixture_id: String,
    fixture_sha256: String,
    candidate_report_sha256: String,
    scenarios_checked: usize,
    candidate_admissions: usize,
    oracle_admissions: usize,
    false_admission_count: usize,
    false_block_count: usize,
    full_decision_mismatch_count: usize,
    fixture_label_mismatch_count: usize,
    state_action_pairs_enumerated: usize,
    legal_state_action_pairs: usize,
    all_candidate_decisions_match: bool,
    all_candidate_admissions_are_oracle_admissible: bool,
    fail_closed_mutations_checked: usize,
    fail_closed_mutation_violation_count: usize,
    fail_closed_mutation_digest_sha256: String,
    all_fail_closed_mutations_rejected: bool,
    fail_closed_mutation_violation_examples: Vec<String>,
    protective_stop_override_checks: usize,
    protective_stop_override_violations: usize,
    claim_ceiling: String,
    scenarios: Vec<ScenarioCheck>,
}

fn main() {
    let mut args = env::args_os().skip(1);
    let fixture_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: recovery-admission-oracle <fixture.json> <candidate-report.json>");
            process::exit(2);
        }
    };
    let report_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: recovery-admission-oracle <fixture.json> <candidate-report.json>");
            process::exit(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: recovery-admission-oracle <fixture.json> <candidate-report.json>");
        process::exit(2);
    }

    let fixture_bytes = read_file(&fixture_path);
    let report_bytes = read_file(&report_path);
    let result = evaluate_reports(&fixture_bytes, &report_bytes);
    match result {
        Ok(report) => {
            let failed = !report.all_candidate_decisions_match
                || !report.all_fail_closed_mutations_rejected
                || report.protective_stop_override_violations != 0;
            match serde_json::to_string_pretty(&report) {
                Ok(json) => println!("{json}"),
                Err(error) => {
                    eprintln!("cannot serialize oracle report: {error}");
                    process::exit(2);
                }
            }
            if failed {
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("independent recovery-admission oracle failed: {error}");
            process::exit(2);
        }
    }
}

fn read_file(path: &std::ffi::OsStr) -> Vec<u8> {
    match fs::read(path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            process::exit(2);
        }
    }
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn field_string(value: &Value, key: &str, context: &str) -> Result<String, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .map(str::to_owned)
        .ok_or_else(|| format!("{context}.{key} must be a string"))
}

fn field_bool(value: &Value, key: &str, context: &str) -> Result<bool, String> {
    value
        .get(key)
        .and_then(Value::as_bool)
        .ok_or_else(|| format!("{context}.{key} must be a boolean"))
}

fn field_u64(value: &Value, key: &str, context: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| format!("{context}.{key} must be an unsigned integer"))
}

fn array_field<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a [Value], String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .map(Vec::as_slice)
        .ok_or_else(|| format!("{context}.{key} must be an array"))
}

fn string_array(value: &Value, key: &str, context: &str) -> Result<Vec<String>, String> {
    array_field(value, key, context)?
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{context}.{key}[{index}] must be a string"))
        })
        .collect()
}

fn find_by_id<'a>(items: &'a [Value], field: &str, requested: &str) -> Option<&'a Value> {
    items
        .iter()
        .find(|item| item.get(field).and_then(Value::as_str) == Some(requested))
}

fn task_items(fixture: &Value) -> Result<&[Value], String> {
    array_field(fixture, "task_graph", "fixture")
}

fn resource_items(fixture: &Value) -> Result<&[Value], String> {
    array_field(fixture, "resources", "fixture")
}

fn observation_items(fixture: &Value) -> Result<&[Value], String> {
    array_field(fixture, "disturbance_cases", "fixture")
}

fn task_by_id<'a>(fixture: &'a Value, task_id: &str) -> Result<Option<&'a Value>, String> {
    Ok(find_by_id(task_items(fixture)?, "task_id", task_id))
}

fn observation_by_id<'a>(
    fixture: &'a Value,
    observation_id: &str,
) -> Result<Option<&'a Value>, String> {
    Ok(find_by_id(
        observation_items(fixture)?,
        "case_id",
        observation_id,
    ))
}

fn ancestor_closure(fixture: &Value, task_id: &str) -> Result<BTreeSet<String>, String> {
    let mut seen = BTreeSet::new();
    let mut pending = vec![task_id.to_owned()];
    while let Some(current) = pending.pop() {
        if !seen.insert(current.clone()) {
            continue;
        }
        if let Some(task) = task_by_id(fixture, &current)? {
            pending.extend(string_array(task, "requires", "task")?);
        }
    }
    Ok(seen)
}

fn uses_resource(action: &str) -> bool {
    matches!(action, "start" | "resume" | "reassign")
}

fn uses_observations(action: &str) -> bool {
    !matches!(action, "protective_stop" | "abort")
}

fn expected_transition(state: &str, action: &str) -> Option<&'static str> {
    match (state, action) {
        ("pending" | "ready", "start") => Some("running"),
        ("running", "protective_stop") => Some("interrupted"),
        ("interrupted" | "rework_required", "resume") => Some("running"),
        ("interrupted" | "rework_required", "reassign") => Some("running"),
        ("running", "submit_for_acceptance") => Some("awaiting_evidence"),
        ("awaiting_evidence", "accept") => Some("accepted"),
        ("awaiting_evidence", "reject_inspection") => Some("rejected"),
        ("rejected", "begin_rework") => Some("rework_required"),
        (
            "pending" | "ready" | "running" | "interrupted" | "awaiting_evidence"
            | "rework_required" | "rejected",
            "abort",
        ) => Some("aborted"),
        _ => None,
    }
}

fn observation_relevant(
    observation: &Value,
    ancestors: &BTreeSet<String>,
    resource_id: Option<&str>,
) -> Result<bool, String> {
    let task_ids = string_array(observation, "affected_tasks", "disturbance")?;
    let task_relevant = task_ids.iter().any(|id| ancestors.contains(id));
    let resource_relevant = match resource_id {
        None => false,
        Some(resource) => string_array(observation, "affected_resources", "disturbance")?
            .iter()
            .any(|id| id == resource),
    };
    Ok(task_relevant || resource_relevant)
}

fn valid_evidence(
    evidence: &[Value],
    evidence_kind: &str,
    tick: u64,
    minimum_tick: u64,
) -> Result<bool, String> {
    for item in evidence {
        if field_string(item, "evidence_kind", "evidence")? == evidence_kind
            && field_bool(item, "current", "evidence")?
            && field_bool(item, "valid", "evidence")?
        {
            let evidence_tick = field_u64(item, "observed_at_tick", "evidence")?;
            if evidence_tick >= minimum_tick && evidence_tick <= tick {
                return Ok(true);
            }
        }
    }
    Ok(false)
}

fn required_evidence_tick(active: &[&Value], evidence_kind: &str) -> Result<u64, String> {
    let mut minimum = 0;
    for observation in active {
        if string_array(observation, "invalidates", "disturbance")?
            .iter()
            .any(|kind| kind == evidence_kind)
        {
            minimum = minimum.max(field_u64(observation, "observable_tick", "disturbance")?);
        }
    }
    Ok(minimum)
}

fn recovery_receipts_required(
    state: &str,
    action: &str,
    observation: &Value,
) -> Result<bool, String> {
    let case_id = field_string(observation, "case_id", "disturbance")?;
    if action == "accept" {
        return Ok(true);
    }
    if case_id == "R-007" && action == "begin_rework" {
        return Ok(false);
    }
    Ok(!(case_id == "R-007"
        && state == "rework_required"
        && matches!(action, "resume" | "reassign")))
}

fn evaluate_candidate(fixture: &Value, proposal: &Value) -> Result<OracleDecision, String> {
    let task_id = field_string(proposal, "task_id", "proposal")?;
    let state = field_string(proposal, "state", "proposal")?;
    let action = field_string(proposal, "action", "proposal")?;
    let tick = field_u64(proposal, "tick", "proposal")?;
    let resource_id = match proposal.get("resource_id") {
        Some(Value::String(id)) => Some(id.as_str()),
        Some(Value::Null) | None => None,
        _ => return Err("proposal.resource_id must be a string or null".to_string()),
    };
    let gates = proposal
        .get("gates")
        .ok_or_else(|| "proposal.gates must exist".to_string())?;
    let task = match task_by_id(fixture, &task_id)? {
        Some(task) => task,
        None => {
            return Ok(OracleDecision {
                admitted: false,
                next_state: None,
                reason_codes: vec![format!("unknown_task:{task_id}")],
            });
        }
    };

    let mut reasons = BTreeSet::new();
    let transition = expected_transition(&state, &action);
    if transition.is_none() {
        reasons.insert("invalid_state_transition".to_string());
    }
    if tick > MAX_ORACLE_TICK && uses_observations(&action) {
        reasons.insert("tick_outside_contract_horizon".to_string());
    }

    let refs = string_array(proposal, "observation_ids", "proposal")?;
    let mut active: Vec<&Value> = Vec::new();
    let mut ref_set = BTreeSet::new();
    let use_observations = uses_observations(&action);
    let contextual_resource = if uses_resource(&action) {
        resource_id
    } else {
        None
    };

    if use_observations {
        for id in &refs {
            if !ref_set.insert(id.clone()) {
                reasons.insert(format!("duplicate_observation_reference:{id}"));
            }
        }
        let ancestors = ancestor_closure(fixture, &task_id)?;
        for observation in observation_items(fixture)? {
            let visible_tick = field_u64(observation, "observable_tick", "disturbance")?;
            if visible_tick <= tick
                && observation_relevant(observation, &ancestors, contextual_resource)?
            {
                let id = field_string(observation, "case_id", "disturbance")?;
                if !ref_set.contains(&id) {
                    reasons.insert(format!("visible_observation_omitted:{id}"));
                }
            }
        }
        for id in &refs {
            match observation_by_id(fixture, id)? {
                None => {
                    reasons.insert(format!("unknown_observation:{id}"));
                }
                Some(observation) => {
                    let visible_tick = field_u64(observation, "observable_tick", "disturbance")?;
                    if tick < visible_tick {
                        reasons.insert(format!("observation_not_yet_visible:{id}"));
                    }
                    let ancestors = ancestor_closure(fixture, &task_id)?;
                    if !observation_relevant(observation, &ancestors, contextual_resource)? {
                        reasons.insert(format!("irrelevant_observation:{id}"));
                    }
                }
            }
        }
        for id in &refs {
            if let Some(observation) = observation_by_id(fixture, id)? {
                if field_u64(observation, "observable_tick", "disturbance")? <= tick {
                    active.push(observation);
                }
            }
        }
    }

    let evidence = array_field(proposal, "evidence", "proposal")?;
    let vocab = string_array(fixture, "evidence_vocabulary", "fixture")?;
    let mut evidence_ids = BTreeSet::new();
    for item in evidence {
        let id = field_string(item, "evidence_id", "evidence")?;
        let kind = field_string(item, "evidence_kind", "evidence")?;
        let observed_tick = field_u64(item, "observed_at_tick", "evidence")?;
        if id.is_empty() || !evidence_ids.insert(id.clone()) {
            reasons.insert(format!("empty_or_duplicate_evidence_id:{id}"));
        }
        if !vocab.contains(&kind) {
            reasons.insert(format!("unknown_evidence_kind:{kind}"));
        }
        if observed_tick > tick {
            reasons.insert(format!("future_dated_evidence:{kind}"));
        }
    }

    if matches!(
        action.as_str(),
        "start" | "resume" | "reassign" | "submit_for_acceptance" | "accept"
    ) {
        let receipts = array_field(proposal, "predecessor_receipts", "proposal")?;
        let required = string_array(task, "requires", "task")?;
        for predecessor_id in &required {
            let matched: Vec<&Value> = receipts
                .iter()
                .filter(|receipt| {
                    receipt.get("task_id").and_then(Value::as_str) == Some(predecessor_id)
                })
                .collect();
            if matched.is_empty() {
                reasons.insert(format!("predecessor_receipt_missing:{predecessor_id}"));
                continue;
            }
            if matched.len() > 1 {
                reasons.insert(format!("predecessor_receipt_duplicated:{predecessor_id}"));
                continue;
            }
            let receipt = matched[0];
            if field_string(receipt, "state", "predecessor_receipt")? != "accepted" {
                reasons.insert(format!("predecessor_not_accepted:{predecessor_id}"));
            }
            let predecessor = task_by_id(fixture, predecessor_id)?
                .ok_or_else(|| format!("fixture lacks predecessor task {predecessor_id}"))?;
            let accepted_kinds = string_array(predecessor, "acceptance_evidence", "task")?;
            let receipt_evidence = array_field(receipt, "evidence", "predecessor_receipt")?;
            for kind in accepted_kinds {
                let minimum = required_evidence_tick(&active, &kind)?;
                if !valid_evidence(receipt_evidence, &kind, tick, minimum)? {
                    reasons.insert(format!(
                        "predecessor_acceptance_evidence_missing_or_stale:{predecessor_id}:{kind}"
                    ));
                }
            }
        }
        for receipt in receipts {
            let receipt_task = field_string(receipt, "task_id", "predecessor_receipt")?;
            if !required.contains(&receipt_task) {
                reasons.insert(format!("unexpected_predecessor_receipt:{receipt_task}"));
            }
            for item in array_field(receipt, "evidence", "predecessor_receipt")? {
                if field_u64(item, "observed_at_tick", "predecessor_evidence")? > tick {
                    reasons.insert(format!("future_dated_predecessor_evidence:{receipt_task}"));
                }
            }
        }
    }

    if uses_resource(&action) {
        let resources = resource_items(fixture)?;
        match resource_id {
            None => {
                reasons.insert("resource_required_for_execution".to_string());
            }
            Some(id) if find_by_id(resources, "resource_id", id).is_none() => {
                reasons.insert(format!("unknown_resource:{id}"));
            }
            Some(_) => {}
        }
        for (field, code) in [
            ("resource_available", "resource_not_available"),
            ("capacity_admissible", "resource_capacity_overcommitment"),
            ("capability_current", "capability_profile_stale"),
            ("calibration_current", "calibration_stale"),
            ("localization_admissible", "localization_out_of_profile"),
            ("communication_current", "communication_state_stale"),
            (
                "energy_reserve_feasible",
                "energy_return_reserve_infeasible",
            ),
        ] {
            if !field_bool(gates, field, "proposal.gates")? {
                reasons.insert(code.to_string());
            }
        }
        if field_string(gates, "utility_envelope", "proposal.gates")? == "unknown_or_unsafe" {
            reasons.insert("unknown_utility_is_not_clear".to_string());
        }
        if matches!(
            field_string(gates, "authorization", "proposal.gates")?.as_str(),
            "pending" | "declined"
        ) {
            reasons.insert("authorization_not_granted".to_string());
        }
        if action == "reassign" {
            let previous = match proposal.get("previous_resource_id") {
                Some(Value::String(value)) => Some(value.as_str()),
                Some(Value::Null) | None => None,
                _ => {
                    return Err(
                        "proposal.previous_resource_id must be a string or null".to_string()
                    );
                }
            };
            match previous {
                None => {
                    reasons.insert("reassignment_missing_previous_resource".to_string());
                }
                Some(previous) if previous == resource_id.unwrap_or("") => {
                    reasons.insert("reassignment_did_not_change_resource".to_string());
                }
                Some(previous) if find_by_id(resources, "resource_id", previous).is_none() => {
                    reasons.insert(format!("unknown_previous_resource:{previous}"));
                }
                Some(_) => {}
            }
        }
    }

    let target_advances = matches!(
        transition,
        Some("running" | "awaiting_evidence" | "accepted" | "rework_required")
    );
    if target_advances
        && active
            .iter()
            .any(|item| item.get("case_id").and_then(Value::as_str) == Some("R-012"))
        && field_string(gates, "authorization", "proposal.gates")? != "granted"
    {
        reasons.insert("visible_approval_gate_requires_granted_authorization:R-012".to_string());
    }
    if target_advances
        && active
            .iter()
            .any(|item| item.get("case_id").and_then(Value::as_str) == Some("R-011"))
        && field_string(gates, "utility_envelope", "proposal.gates")? == "unknown_or_unsafe"
    {
        reasons.insert("visible_utility_gate_must_be_resolved:R-011".to_string());
    }

    if action == "submit_for_acceptance" && !field_bool(gates, "work_complete", "proposal.gates")? {
        reasons.insert("cannot_submit_incomplete_work_for_acceptance".to_string());
    }
    if action == "accept" {
        if !field_bool(gates, "work_complete", "proposal.gates")? {
            reasons.insert("cannot_accept_incomplete_work".to_string());
        }
        if field_string(gates, "inspection", "proposal.gates")? != "passed" {
            reasons.insert("acceptance_requires_successful_inspection".to_string());
        }
        for kind in string_array(task, "acceptance_evidence", "task")? {
            let minimum = required_evidence_tick(&active, &kind)?;
            if !valid_evidence(evidence, &kind, tick, minimum)? {
                reasons.insert(format!("acceptance_evidence_missing_or_stale:{kind}"));
            }
        }
    }
    if action == "reject_inspection"
        && field_string(gates, "inspection", "proposal.gates")? != "failed"
    {
        reasons.insert("rejection_requires_failed_inspection".to_string());
    }
    if action == "begin_rework" {
        if field_string(gates, "inspection", "proposal.gates")? != "failed" {
            reasons.insert("rework_requires_preserved_failed_inspection".to_string());
        }
        if !active
            .iter()
            .any(|item| item.get("case_id").and_then(Value::as_str) == Some("R-007"))
        {
            reasons.insert("rework_requires_visible_rejection_observation:R-007".to_string());
        }
    }
    for observation in &active {
        if !recovery_receipts_required(&state, &action, observation)? {
            continue;
        }
        let minimum = field_u64(observation, "observable_tick", "disturbance")?;
        let case_id = field_string(observation, "case_id", "disturbance")?;
        for kind in string_array(observation, "recovery_evidence", "disturbance")? {
            if !valid_evidence(evidence, &kind, tick, minimum)? {
                reasons.insert(format!(
                    "recovery_evidence_missing_or_stale:{case_id}:{kind}"
                ));
            }
        }
    }
    if action == "protective_stop" && state != "running" {
        reasons.insert("protective_stop_requires_running_state".to_string());
    }

    let reason_codes: Vec<String> = reasons.into_iter().collect();
    let admitted = reason_codes.is_empty();
    Ok(OracleDecision {
        admitted,
        next_state: if admitted {
            transition.map(str::to_owned)
        } else {
            None
        },
        reason_codes,
    })
}

fn all_state_action_pairs() -> (usize, usize) {
    let states = [
        "pending",
        "ready",
        "running",
        "interrupted",
        "awaiting_evidence",
        "rework_required",
        "accepted",
        "rejected",
        "aborted",
    ];
    let actions = [
        "start",
        "protective_stop",
        "resume",
        "reassign",
        "submit_for_acceptance",
        "accept",
        "reject_inspection",
        "begin_rework",
        "abort",
    ];
    let legal = states
        .iter()
        .flat_map(|state| actions.iter().map(move |action| (*state, *action)))
        .filter(|(state, action)| expected_transition(state, action).is_some())
        .count();
    (states.len() * actions.len(), legal)
}

fn evaluate_reports(fixture_bytes: &[u8], report_bytes: &[u8]) -> Result<OracleReport, String> {
    let fixture: Value = serde_json::from_slice(fixture_bytes)
        .map_err(|error| format!("cannot parse fixture: {error}"))?;
    let candidate_report: Value = serde_json::from_slice(report_bytes)
        .map_err(|error| format!("cannot parse candidate report: {error}"))?;

    if field_string(&fixture, "schema_version", "fixture")? != "civ-infra-fleet-resilience/0.1"
        || field_string(&fixture, "classification", "fixture")? != "synthetic_only"
    {
        return Err("fixture schema or synthetic-only classification is invalid".to_string());
    }
    let fixture_id = field_string(&fixture, "fixture_id", "fixture")?;
    let candidate_fixture_id = field_string(&candidate_report, "fixture_id", "candidate_report")?;
    if candidate_fixture_id != fixture_id {
        return Err("candidate report refers to a different fixture".to_string());
    }
    let fixture_digest = sha256_hex(fixture_bytes);
    if field_string(&candidate_report, "fixture_sha256", "candidate_report")? != fixture_digest {
        return Err(
            "candidate report fixture digest does not match exact fixture bytes".to_string(),
        );
    }
    if field_string(&candidate_report, "schema_version", "candidate_report")?
        != "luminous.civ_infra.recovery_admission_contract.v0.1"
    {
        return Err("candidate report schema is unexpected".to_string());
    }
    if field_string(&candidate_report, "classification", "candidate_report")? != "synthetic_only" {
        return Err("candidate report exceeds synthetic-only scope".to_string());
    }
    if field_bool(
        &candidate_report,
        "policy_simulation_executed",
        "candidate_report",
    )? || field_u64(
        &candidate_report,
        "policy_runs_performed",
        "candidate_report",
    )? != 0
        || field_bool(
            &candidate_report,
            "immutable_history_store_executed",
            "candidate_report",
        )?
        || !field_bool(
            &candidate_report,
            "gate_inputs_are_synthetic_assertions",
            "candidate_report",
        )?
    {
        return Err("candidate report contradicts the synthetic-only claim ceiling".to_string());
    }
    if field_bool(
        &candidate_report,
        "physical_control_executed",
        "candidate_report",
    )? || field_bool(&candidate_report, "dispatch_authorized", "candidate_report")?
        || field_bool(
            &candidate_report,
            "input_receipts_cryptographically_verified",
            "candidate_report",
        )?
    {
        return Err("candidate report exceeds its declared claim ceiling".to_string());
    }

    let source_scenarios = array_field(&candidate_report, "scenario_results", "candidate_report")?;
    if source_scenarios.is_empty() {
        return Err("candidate report contains no scenarios".to_string());
    }

    let mut checks = Vec::with_capacity(source_scenarios.len());
    let mut candidate_admissions = 0;
    let mut oracle_admissions = 0;
    let mut false_admission_count = 0;
    let mut false_block_count = 0;
    let mut full_decision_mismatch_count = 0;
    let mut fixture_label_mismatch_count = 0;

    for (index, item) in source_scenarios.iter().enumerate() {
        let context = format!("scenario_results[{index}]");
        let scenario_id = field_string(item, "scenario_id", &context)?;
        let fixture_label_admitted = field_bool(item, "expected_admitted", &context)?;
        let candidate_admitted = field_bool(item, "admitted", &context)?;
        let candidate_next_state = match item.get("next_state") {
            Some(Value::String(value)) => Some(value.clone()),
            Some(Value::Null) | None => None,
            _ => return Err(format!("{context}.next_state must be a string or null")),
        };
        let candidate_reason_codes = string_array(item, "reason_codes", &context)?;
        let proposal = item
            .get("proposal")
            .ok_or_else(|| format!("{context}.proposal is missing"))?;
        let oracle = evaluate_candidate(&fixture, proposal)?;

        if candidate_admitted {
            candidate_admissions += 1;
        }
        if oracle.admitted {
            oracle_admissions += 1;
        }
        if candidate_admitted && !oracle.admitted {
            false_admission_count += 1;
        }
        if !candidate_admitted && oracle.admitted {
            false_block_count += 1;
        }
        if fixture_label_admitted != oracle.admitted {
            fixture_label_mismatch_count += 1;
        }
        let decision_matches_oracle = candidate_admitted == oracle.admitted
            && candidate_next_state == oracle.next_state
            && candidate_reason_codes == oracle.reason_codes;
        if !decision_matches_oracle {
            full_decision_mismatch_count += 1;
        }
        checks.push(ScenarioCheck {
            scenario_id,
            fixture_label_admitted,
            candidate_admitted,
            oracle_admitted: oracle.admitted,
            candidate_next_state,
            oracle_next_state: oracle.next_state,
            candidate_reason_codes,
            oracle_reason_codes: oracle.reason_codes,
            decision_matches_oracle,
            label_matches_oracle: fixture_label_admitted == oracle.admitted,
        });
    }

    let (state_action_pairs_enumerated, legal_state_action_pairs) = all_state_action_pairs();
    let all_candidate_decisions_match = full_decision_mismatch_count == 0;
    Ok(OracleReport {
        schema_version: "luminous.civ_infra.recovery_admission_oracle.v0.1".to_string(),
        classification: "synthetic_only".to_string(),
        fixture_id,
        fixture_sha256: fixture_digest,
        candidate_report_sha256: sha256_hex(report_bytes),
        scenarios_checked: checks.len(),
        candidate_admissions,
        oracle_admissions,
        false_admission_count,
        false_block_count,
        full_decision_mismatch_count,
        fixture_label_mismatch_count,
        state_action_pairs_enumerated,
        legal_state_action_pairs,
        all_candidate_decisions_match,
        all_candidate_admissions_are_oracle_admissible: false_admission_count == 0,
        claim_ceiling: "Independent source-level semantic cross-check of synthetic report proposals only; it is not physical validation, evidence authentication, dispatch authorization, or an exhaustive oracle over all task/resource/evidence states.".to_string(),
        scenarios: checks,
    })
}
