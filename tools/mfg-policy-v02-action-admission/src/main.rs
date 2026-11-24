use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

const FRAME_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_decision_frame.schema.json");
const PROPOSAL_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_action_proposal.schema.json");
const RECEIPT_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_policy_v02_action_admission_receipt.schema.json"
);

const RECEIPT_SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/mfg-policy-v0.2-action-admission-receipt";
const CLAIM_CEILING: &str = "Synthetic single-action admissibility only; this receipt is not schedule feasibility, source authentication, machine authority, or execution authorization.";

#[derive(Debug, Clone, Copy, Serialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
enum AdmissionStatus {
    Admissible,
    Hold,
    Rejected,
}

#[derive(Debug, Serialize)]
struct AdmissionReceipt {
    schema_id: &'static str,
    benchmark_id: String,
    case_id: String,
    decision_epoch: u64,
    decision_frame_digest: String,
    operation_id: String,
    resource_id: String,
    status: AdmissionStatus,
    reason_codes: Vec<String>,
    qualification_disposition: Option<String>,
    processing_duration_ticks: Option<u64>,
    setup_duration_ticks: Option<u64>,
    inspection_method_id: Option<String>,
    capacity_units_required: u64,
    claim_ceiling: &'static str,
}

fn fail(message: impl Into<String>) -> String {
    message.into()
}

fn validate_schema(instance: &Value, source: &str, label: &str) -> Result<(), String> {
    let schema: Value = serde_json::from_str(source)
        .map_err(|error| fail(format!("{label}_schema_parse:{error}")))?;
    let validator = jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|error| fail(format!("{label}_schema_compile:{error}")))?;
    validator
        .validate(instance)
        .map_err(|error| fail(format!("{label}_schema_validation:{error}")))
}

fn required_string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| fail(format!("{context}_{key}_missing_or_invalid")))
}

fn required_array<'a>(
    value: &'a Value,
    key: &str,
    context: &str,
) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| fail(format!("{context}_{key}_missing_or_invalid")))
}

fn required_u64(value: &Value, key: &str, context: &str) -> Result<u64, String> {
    value
        .get(key)
        .and_then(Value::as_u64)
        .ok_or_else(|| fail(format!("{context}_{key}_missing_or_invalid")))
}

fn digest_value(value: &Value) -> Result<String, String> {
    let bytes = serde_json::to_vec(value)
        .map_err(|error| fail(format!("decision_frame_serialize:{error}")))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn mark_hold(receipt: &mut AdmissionReceipt, reason: &str) {
    if receipt.status != AdmissionStatus::Rejected {
        receipt.status = AdmissionStatus::Hold;
    }
    if !receipt
        .reason_codes
        .iter()
        .any(|existing| existing == reason)
    {
        receipt.reason_codes.push(reason.to_string());
    }
}

fn mark_rejected(receipt: &mut AdmissionReceipt, reason: &str) {
    receipt.status = AdmissionStatus::Rejected;
    if !receipt
        .reason_codes
        .iter()
        .any(|existing| existing == reason)
    {
        receipt.reason_codes.push(reason.to_string());
    }
}

fn operation_index(static_case: &Value) -> Result<BTreeMap<String, (&Value, &Value)>, String> {
    let jobs = required_array(static_case, "jobs", "static_case")?;
    let mut job_ids = BTreeSet::new();
    let mut operations = BTreeMap::new();
    for job in jobs {
        let job_id = required_string(job, "job_id", "job")?;
        if !job_ids.insert(job_id.to_string()) {
            return Err(fail("job_id_duplicate"));
        }
        for operation in required_array(job, "operations", "job")? {
            let operation_id = required_string(operation, "operation_id", "operation")?;
            if operations
                .insert(operation_id.to_string(), (job, operation))
                .is_some()
            {
                return Err(fail("operation_id_duplicate"));
            }
        }
    }
    Ok(operations)
}

fn resource_index(static_case: &Value) -> Result<BTreeMap<String, &Value>, String> {
    let resources = required_array(static_case, "resources", "static_case")?;
    let mut index = BTreeMap::new();
    for resource in resources {
        let resource_id = required_string(resource, "resource_id", "resource")?;
        if index.insert(resource_id.to_string(), resource).is_some() {
            return Err(fail("resource_id_duplicate"));
        }
    }
    Ok(index)
}

struct VisibleEvidenceState {
    qualification_by_resource_family: BTreeMap<(String, String), String>,
    resource_status_by_id: BTreeMap<String, String>,
    inspection_capacity_override: Option<u64>,
}

fn validate_visible_history(
    frame: &Value,
    static_case: &Value,
    resources: &BTreeMap<String, &Value>,
    decision_epoch: u64,
) -> Result<VisibleEvidenceState, String> {
    let mut qualification_state = BTreeMap::<(String, String), String>::new();
    let mut evidence_receipts = BTreeSet::<String>::new();

    for (resource_id, resource) in resources {
        let capabilities = required_array(resource, "capability_families", "resource")?;
        let mut capability_set = BTreeSet::new();
        for capability in capabilities {
            let capability = capability
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| fail("resource_capability_invalid"))?;
            if !capability_set.insert(capability.to_string()) {
                return Err(fail("resource_capability_duplicate"));
            }
        }

        let capacity = required_array(resource, "capacity_units_by_epoch", "resource")?;
        if capacity.len() != 4 || capacity.iter().any(|item| item.as_u64().is_none()) {
            return Err(fail("resource_capacity_calendar_invalid"));
        }

        for qualification in required_array(resource, "initial_qualifications", "resource")? {
            let family = required_string(qualification, "process_family", "qualification")?;
            let disposition = required_string(qualification, "disposition", "qualification")?;
            if !["ADMISSIBLE", "UNKNOWN", "BLOCKED"].contains(&disposition) {
                return Err(fail("qualification_disposition_invalid"));
            }
            let receipt = required_string(qualification, "evidence_receipt_id", "qualification")?;
            if !evidence_receipts.insert(receipt.to_string()) {
                return Err(fail("qualification_receipt_duplicate"));
            }
            if !capability_set.contains(family) {
                return Err(fail("qualification_process_not_supported"));
            }
            if qualification_state
                .insert(
                    (resource_id.to_string(), family.to_string()),
                    disposition.to_string(),
                )
                .is_some()
            {
                return Err(fail("qualification_scope_duplicate"));
            }
        }
    }

    let history = required_array(frame, "observable_history", "decision_frame")?;
    if history.len() as u64 != decision_epoch + 1 {
        return Err(fail("observable_history_must_end_at_decision_epoch"));
    }

    let mut visible_resource_state = BTreeMap::<String, String>::new();
    let mut inspection_capacity_override = None;
    let mut resource_event_epochs = BTreeSet::<(String, u64)>::new();
    let mut qualification_update_epochs = BTreeSet::<(String, String, u64)>::new();
    let mut inspection_event_epochs = BTreeSet::<u64>::new();
    let station_id = required_string(
        static_case
            .get("inspection_station")
            .ok_or_else(|| fail("inspection_station_missing"))?,
        "station_id",
        "inspection_station",
    )?;

    for (expected_epoch, observation) in history.iter().enumerate() {
        let actual_epoch = required_u64(observation, "epoch", "observation")?;
        if actual_epoch != expected_epoch as u64 || actual_epoch > decision_epoch {
            return Err(fail("observable_history_epoch_order_invalid"));
        }

        for event in required_array(observation, "resource_events", "observation")? {
            let event_id = required_string(event, "source_event_id", "resource_event")?;
            let resource_id = required_string(event, "resource_id", "resource_event")?;
            let kind = required_string(event, "kind", "resource_event")?;
            let occurred = required_u64(event, "occurred_at_epoch", "resource_event")?;
            let observed = required_u64(event, "observed_at_epoch", "resource_event")?;
            if !evidence_receipts.insert(event_id.to_string())
                || !resources.contains_key(resource_id)
                || observed != expected_epoch as u64
                || occurred > observed
                || !resource_event_epochs.insert((resource_id.to_string(), observed))
            {
                return Err(fail("resource_event_identity_scope_or_chronology_invalid"));
            }
            let forecast_min = event.get("recovery_forecast_min_epochs");
            let forecast_max = event.get("recovery_forecast_max_epochs");
            match kind {
                "FAILED" => {
                    let lower = forecast_min
                        .and_then(Value::as_u64)
                        .ok_or_else(|| fail("failure_recovery_forecast_missing"))?;
                    let upper = forecast_max
                        .and_then(Value::as_u64)
                        .ok_or_else(|| fail("failure_recovery_forecast_missing"))?;
                    if lower > upper {
                        return Err(fail("failure_recovery_forecast_invalid"));
                    }
                }
                "RESTORED" => {
                    if !forecast_min.is_some_and(Value::is_null)
                        || !forecast_max.is_some_and(Value::is_null)
                    {
                        return Err(fail("restoration_must_not_carry_recovery_forecast"));
                    }
                }
                _ => return Err(fail("resource_event_kind_invalid")),
            }
            visible_resource_state.insert(resource_id.to_string(), kind.to_string());
        }

        for update in required_array(observation, "qualification_updates", "observation")? {
            let receipt = required_string(update, "evidence_receipt_id", "qualification_update")?;
            let resource_id = required_string(update, "resource_id", "qualification_update")?;
            let family = required_string(update, "process_family", "qualification_update")?;
            let previous = required_string(update, "previous_disposition", "qualification_update")?;
            let next = required_string(update, "new_disposition", "qualification_update")?;
            let occurred = required_u64(update, "occurred_at_epoch", "qualification_update")?;
            let observed = required_u64(update, "observed_at_epoch", "qualification_update")?;
            if !evidence_receipts.insert(receipt.to_string())
                || observed != expected_epoch as u64
                || occurred > observed
                || !["ADMISSIBLE", "UNKNOWN", "BLOCKED"].contains(&previous)
                || !["ADMISSIBLE", "UNKNOWN", "BLOCKED"].contains(&next)
                || previous == next
                || !qualification_update_epochs.insert((
                    resource_id.to_string(),
                    family.to_string(),
                    observed,
                ))
            {
                return Err(fail("qualification_update_identity_or_chronology_invalid"));
            }
            let update_resource = resources
                .get(resource_id)
                .ok_or_else(|| fail("qualification_update_resource_unknown"))?;
            if !required_array(update_resource, "capability_families", "resource")?
                .iter()
                .any(|capability| capability.as_str() == Some(family))
            {
                return Err(fail("qualification_update_process_not_supported"));
            }
            let state = qualification_state
                .get_mut(&(resource_id.to_string(), family.to_string()))
                .ok_or_else(|| fail("qualification_update_prior_state_missing"))?;
            if state != previous {
                return Err(fail("qualification_update_prior_state_mismatch"));
            }
            *state = next.to_string();
        }

        for event in required_array(observation, "inspection_capacity_events", "observation")? {
            let event_id = required_string(event, "source_event_id", "inspection_event")?;
            let event_station = required_string(event, "station_id", "inspection_event")?;
            let occurred = required_u64(event, "occurred_at_epoch", "inspection_event")?;
            let observed = required_u64(event, "observed_at_epoch", "inspection_event")?;
            let capacity = required_u64(event, "available_capacity_units", "inspection_event")?;
            if !evidence_receipts.insert(event_id.to_string())
                || event_station != station_id
                || observed != expected_epoch as u64
                || occurred > observed
                || !inspection_event_epochs.insert(observed)
            {
                return Err(fail(
                    "inspection_event_identity_scope_or_chronology_invalid",
                ));
            }
            inspection_capacity_override = Some(capacity);
        }
    }

    let _ = static_case;
    Ok(VisibleEvidenceState {
        qualification_by_resource_family: qualification_state,
        resource_status_by_id: visible_resource_state,
        inspection_capacity_override,
    })
}

fn evaluate(frame: &Value, proposal: &Value) -> Result<AdmissionReceipt, String> {
    validate_schema(frame, FRAME_SCHEMA_JSON, "decision_frame")?;
    validate_schema(proposal, PROPOSAL_SCHEMA_JSON, "action_proposal")?;
    let decision_frame_digest = digest_value(frame)?;

    let benchmark_id = required_string(frame, "benchmark_id", "decision_frame")?.to_string();
    let case_id = required_string(frame, "case_id", "decision_frame")?.to_string();
    let decision_epoch = required_u64(frame, "decision_epoch", "decision_frame")?;
    let operation_id = required_string(proposal, "operation_id", "action_proposal")?.to_string();
    let resource_id = required_string(proposal, "resource_id", "action_proposal")?.to_string();

    let mut receipt = AdmissionReceipt {
        schema_id: RECEIPT_SCHEMA_ID,
        benchmark_id: benchmark_id.clone(),
        case_id: case_id.clone(),
        decision_epoch,
        decision_frame_digest: decision_frame_digest.clone(),
        operation_id: operation_id.clone(),
        resource_id: resource_id.clone(),
        status: AdmissionStatus::Admissible,
        reason_codes: Vec::new(),
        qualification_disposition: None,
        processing_duration_ticks: None,
        setup_duration_ticks: None,
        inspection_method_id: None,
        capacity_units_required: 1,
        claim_ceiling: CLAIM_CEILING,
    };

    if required_string(proposal, "benchmark_id", "action_proposal")? != benchmark_id
        || required_string(proposal, "case_id", "action_proposal")? != case_id
        || required_u64(proposal, "decision_epoch", "action_proposal")? != decision_epoch
    {
        mark_rejected(&mut receipt, "proposal_frame_identity_mismatch");
        return Ok(receipt);
    }
    if required_string(proposal, "decision_frame_digest", "action_proposal")?
        != decision_frame_digest
    {
        mark_rejected(&mut receipt, "proposal_frame_digest_mismatch");
        return Ok(receipt);
    }

    let static_case = frame
        .get("static_case")
        .ok_or_else(|| fail("decision_frame_static_case_missing"))?;
    if required_string(static_case, "case_id", "static_case")? != case_id {
        mark_rejected(&mut receipt, "static_case_identity_mismatch");
        return Ok(receipt);
    }

    // This first slice has no simulator-maintained post-dispatch operation/setup snapshot.
    // Never re-admit work in a later epoch from the immutable initial state.
    if decision_epoch != 0 {
        mark_hold(
            &mut receipt,
            "dynamic_operation_state_snapshot_not_yet_supported",
        );
        return Ok(receipt);
    }

    let operations = operation_index(static_case)?;
    let resources = resource_index(static_case)?;
    let visible_state = validate_visible_history(frame, static_case, &resources, decision_epoch)?;

    let Some((_, operation)) = operations.get(&operation_id).copied() else {
        mark_rejected(&mut receipt, "operation_not_found");
        return Ok(receipt);
    };
    let Some(resource) = resources.get(&resource_id).copied() else {
        mark_rejected(&mut receipt, "resource_not_found");
        return Ok(receipt);
    };

    let process_family = required_string(operation, "process_family", "operation")?;
    let action_kind = required_string(proposal, "action_kind", "action_proposal")?;
    let initial_status = required_string(operation, "initial_status", "operation")?;
    match (action_kind, initial_status) {
        ("START", "WAITING") | ("CONTINUE", "IN_PROCESS") => {}
        ("START", "IN_PROCESS") => {
            mark_rejected(&mut receipt, "in_process_operation_requires_continue");
        }
        ("CONTINUE", "WAITING") => {
            mark_rejected(&mut receipt, "waiting_operation_cannot_continue");
        }
        (_, "COMPLETED") => mark_rejected(&mut receipt, "completed_operation_cannot_be_dispatched"),
        _ => mark_rejected(&mut receipt, "operation_action_state_mismatch"),
    }
    if receipt.status == AdmissionStatus::Rejected {
        return Ok(receipt);
    }

    let capabilities = required_array(resource, "capability_families", "resource")?;
    if !capabilities
        .iter()
        .any(|value| value.as_str() == Some(process_family))
    {
        mark_rejected(&mut receipt, "resource_lacks_process_capability");
    }
    let eligible = required_array(operation, "eligible_resource_ids", "operation")?;
    if !eligible
        .iter()
        .any(|value| value.as_str() == Some(resource_id.as_str()))
    {
        mark_rejected(&mut receipt, "resource_not_operation_eligible");
    }
    if receipt.status == AdmissionStatus::Rejected {
        return Ok(receipt);
    }

    if action_kind == "CONTINUE" {
        let committed = operation
            .get("committed_resource_id")
            .and_then(Value::as_str);
        if committed != Some(resource_id.as_str()) {
            mark_rejected(&mut receipt, "continuation_resource_commitment_mismatch");
            return Ok(receipt);
        }
        let remaining = required_u64(operation, "remaining_processing_ticks", "operation")?;
        if remaining == 0 {
            mark_rejected(&mut receipt, "continuation_has_no_remaining_work");
            return Ok(receipt);
        }
        receipt.processing_duration_ticks = Some(remaining);
    } else {
        if !operation
            .get("committed_resource_id")
            .is_none_or(Value::is_null)
        {
            mark_rejected(
                &mut receipt,
                "waiting_operation_has_existing_resource_commitment",
            );
            return Ok(receipt);
        }
        let remaining = required_u64(operation, "remaining_processing_ticks", "operation")?;
        if remaining != 0 {
            mark_rejected(
                &mut receipt,
                "waiting_operation_has_unmodeled_remaining_work",
            );
            return Ok(receipt);
        }
        let times = operation
            .get("processing_time_ticks_by_resource")
            .and_then(Value::as_object)
            .ok_or_else(|| fail("operation_processing_times_missing"))?;
        let duration = times
            .get(&resource_id)
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| fail("eligible_resource_processing_time_missing_or_zero"))?;
        receipt.processing_duration_ticks = Some(duration);
    }

    let predecessors = required_array(operation, "predecessor_operation_ids", "operation")?;
    for predecessor in predecessors {
        let predecessor_id = predecessor
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| fail("predecessor_operation_id_invalid"))?;
        let Some((_, predecessor_operation)) = operations.get(predecessor_id).copied() else {
            return Err(fail("predecessor_operation_unknown"));
        };
        if required_string(predecessor_operation, "initial_status", "predecessor")? != "COMPLETED" {
            mark_hold(&mut receipt, "predecessor_not_completed_at_decision_cutoff");
        }
    }

    let capacity = required_array(resource, "capacity_units_by_epoch", "resource")?;
    let current_capacity = capacity
        .get(decision_epoch as usize)
        .and_then(Value::as_u64)
        .ok_or_else(|| fail("resource_capacity_for_decision_epoch_missing"))?;
    if current_capacity == 0 {
        mark_hold(
            &mut receipt,
            "resource_capacity_unavailable_in_current_epoch",
        );
    }
    if visible_state
        .resource_status_by_id
        .get(&resource_id)
        .is_some_and(|state| state == "FAILED")
    {
        mark_hold(&mut receipt, "resource_has_visible_unrestored_failure");
    }

    let qualification = visible_state
        .qualification_by_resource_family
        .get(&(resource_id.clone(), process_family.to_string()));
    match qualification.map(String::as_str) {
        Some("ADMISSIBLE") => {
            receipt.qualification_disposition = Some("ADMISSIBLE".to_string());
        }
        Some("UNKNOWN") => {
            receipt.qualification_disposition = Some("UNKNOWN".to_string());
            mark_hold(&mut receipt, "qualification_unknown_at_decision_cutoff");
        }
        Some("BLOCKED") => {
            receipt.qualification_disposition = Some("BLOCKED".to_string());
            mark_rejected(&mut receipt, "qualification_blocked_at_decision_cutoff");
        }
        None => mark_hold(
            &mut receipt,
            "qualification_state_missing_at_decision_cutoff",
        ),
        Some(_) => return Err(fail("qualification_disposition_invalid")),
    }

    if action_kind == "START" {
        let current_setup = required_string(resource, "initial_setup_family", "resource")?;
        let target_setup = required_string(operation, "setup_family", "operation")?;
        let setup_duration = if current_setup == target_setup {
            0
        } else {
            let transitions = required_array(static_case, "setup_transitions", "static_case")?;
            let matching = transitions
                .iter()
                .filter(|transition| {
                    transition.get("resource_id").and_then(Value::as_str)
                        == Some(resource_id.as_str())
                        && transition.get("from_setup_family").and_then(Value::as_str)
                            == Some(current_setup)
                        && transition.get("to_setup_family").and_then(Value::as_str)
                            == Some(target_setup)
                })
                .collect::<Vec<_>>();
            match matching.as_slice() {
                [transition] => required_u64(transition, "duration_ticks", "setup_transition")?,
                [] => {
                    mark_rejected(&mut receipt, "setup_transition_missing");
                    0
                }
                _ => return Err(fail("setup_transition_duplicate")),
            }
        };
        receipt.setup_duration_ticks = Some(setup_duration);
    } else {
        receipt.setup_duration_ticks = Some(0);
    }

    let inspection_required = operation
        .get("inspection_required")
        .and_then(Value::as_bool)
        .ok_or_else(|| fail("operation_inspection_required_missing"))?;
    let proposed_method = proposal
        .get("inspection_method_id")
        .ok_or_else(|| fail("proposal_inspection_method_missing"))?;
    let station = static_case
        .get("inspection_station")
        .ok_or_else(|| fail("inspection_station_missing"))?;
    let station_capacity =
        required_array(station, "capacity_units_by_epoch", "inspection_station")?;
    let mut current_inspection_capacity = station_capacity
        .get(decision_epoch as usize)
        .and_then(Value::as_u64)
        .ok_or_else(|| fail("inspection_capacity_for_decision_epoch_missing"))?;
    if let Some(override_capacity) = visible_state.inspection_capacity_override {
        current_inspection_capacity = override_capacity;
    }

    if inspection_required {
        let required_method = required_string(operation, "inspection_method_id", "operation")?;
        if proposed_method.as_str() != Some(required_method) {
            mark_rejected(&mut receipt, "required_inspection_method_mismatch");
        } else {
            let methods = required_array(station, "methods", "inspection_station")?;
            let matching = methods
                .iter()
                .filter(|method| {
                    method.get("method_id").and_then(Value::as_str) == Some(required_method)
                })
                .collect::<Vec<_>>();
            let method = match matching.as_slice() {
                [method] => *method,
                [] => {
                    mark_rejected(&mut receipt, "required_inspection_method_not_declared");
                    return Ok(receipt);
                }
                _ => return Err(fail("inspection_method_id_duplicate")),
            };
            let applicable =
                required_array(method, "applicable_process_families", "inspection_method")?;
            if !applicable
                .iter()
                .any(|family| family.as_str() == Some(process_family))
            {
                mark_rejected(&mut receipt, "inspection_method_not_applicable_to_process");
            } else {
                receipt.inspection_method_id = Some(required_method.to_string());
            }
        }
        if current_inspection_capacity == 0 {
            mark_hold(
                &mut receipt,
                "inspection_capacity_unavailable_in_current_epoch",
            );
        }
    } else if !proposed_method.is_null() {
        mark_rejected(&mut receipt, "unneeded_inspection_method_proposed");
    }

    Ok(receipt)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 {
        return Err(
            "usage: mfg-policy-v02-action-admission <decision-frame.json> <action-proposal.json>"
                .into(),
        );
    }
    let frame_bytes =
        std::fs::read(&args[1]).map_err(|error| fail(format!("decision_frame_read:{error}")))?;
    let proposal_bytes =
        std::fs::read(&args[2]).map_err(|error| fail(format!("action_proposal_read:{error}")))?;
    let frame: Value = serde_json::from_slice(&frame_bytes)
        .map_err(|error| fail(format!("decision_frame_json:{error}")))?;
    let proposal: Value = serde_json::from_slice(&proposal_bytes)
        .map_err(|error| fail(format!("action_proposal_json:{error}")))?;
    let receipt = evaluate(&frame, &proposal)?;
    let receipt_value = serde_json::to_value(&receipt)
        .map_err(|error| fail(format!("admission_receipt_serialize:{error}")))?;
    validate_schema(
        &receipt_value,
        RECEIPT_SCHEMA_JSON,
        "action_admission_receipt",
    )?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| fail(format!("admission_receipt_json:{error}")))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample_frame() -> Value {
        json!({
            "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-decision-frame",
            "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
            "case_id": "CASE-01",
            "decision_epoch": 0,
            "static_case": {
                "case_id": "CASE-01",
                "horizon_epochs": 4,
                "jobs": [
                    {"job_id": "JOB-1", "operations": [
                        {
                            "operation_id": "OP-1", "sequence": 1,
                            "process_family": "CUTTING", "setup_family": "SETUP-A",
                            "predecessor_operation_ids": [], "eligible_resource_ids": ["RESOURCE-A"],
                            "processing_time_ticks_by_resource": {"RESOURCE-A": 1},
                            "initial_status": "COMPLETED", "committed_resource_id": null,
                            "remaining_processing_ticks": 0, "inspection_required": false,
                            "inspection_method_id": null
                        },
                        {
                            "operation_id": "OP-2", "sequence": 2,
                            "process_family": "FORMING", "setup_family": "SETUP-B",
                            "predecessor_operation_ids": ["OP-1"], "eligible_resource_ids": ["RESOURCE-A"],
                            "processing_time_ticks_by_resource": {"RESOURCE-A": 2},
                            "initial_status": "WAITING", "committed_resource_id": null,
                            "remaining_processing_ticks": 0, "inspection_required": false,
                            "inspection_method_id": null
                        }
                    ]},
                    {"job_id": "JOB-2", "operations": [
                        {
                            "operation_id": "OP-3", "sequence": 1,
                            "process_family": "CUTTING", "setup_family": "SETUP-B",
                            "predecessor_operation_ids": [], "eligible_resource_ids": ["RESOURCE-A"],
                            "processing_time_ticks_by_resource": {"RESOURCE-A": 2},
                            "initial_status": "WAITING", "committed_resource_id": null,
                            "remaining_processing_ticks": 0, "inspection_required": true,
                            "inspection_method_id": "DIMENSIONAL"
                        }
                    ]}
                ],
                "resources": [{
                    "resource_id": "RESOURCE-A",
                    "capability_families": ["CUTTING", "FORMING"],
                    "common_cause_domain": "POWER-1",
                    "capacity_units_by_epoch": [1, 1, 1, 1],
                    "initial_setup_family": "SETUP-A",
                    "initial_qualifications": [
                        {"process_family": "CUTTING", "disposition": "ADMISSIBLE", "evidence_receipt_id": "Q-CUT"},
                        {"process_family": "FORMING", "disposition": "ADMISSIBLE", "evidence_receipt_id": "Q-FORM"}
                    ]
                }],
                "setup_transitions": [{
                    "resource_id": "RESOURCE-A", "from_setup_family": "SETUP-A",
                    "to_setup_family": "SETUP-B", "duration_ticks": 2
                }],
                "inspection_station": {
                    "station_id": "INSPECT-1", "capacity_units_by_epoch": [1, 1, 1, 1],
                    "methods": [{
                        "method_id": "DIMENSIONAL",
                        "applicable_process_families": ["CUTTING"],
                        "duration_ticks": 1, "cost_units": 1
                    }]
                }
            },
            "observable_history": [{
                "epoch": 0, "resource_events": [], "qualification_updates": [],
                "inspection_capacity_events": []
            }],
            "claim_ceiling": "Synthetic policy-input projection only; no physical authority."
        })
    }

    fn sample_proposal() -> Value {
        json!({
            "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-action-proposal",
            "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
            "case_id": "CASE-01",
            "decision_epoch": 0,
            "decision_frame_digest": digest_value(&sample_frame()).unwrap(),
            "operation_id": "OP-3",
            "resource_id": "RESOURCE-A",
            "action_kind": "START",
            "inspection_method_id": "DIMENSIONAL"
        })
    }

    #[test]
    fn eligible_qualified_precedent_free_action_is_admitted_with_derived_durations() {
        let receipt = evaluate(&sample_frame(), &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Admissible);
        assert_eq!(receipt.processing_duration_ticks, Some(2));
        assert_eq!(receipt.setup_duration_ticks, Some(2));
        assert_eq!(
            receipt.qualification_disposition.as_deref(),
            Some("ADMISSIBLE")
        );
        assert_eq!(receipt.inspection_method_id.as_deref(), Some("DIMENSIONAL"));
        assert!(receipt.reason_codes.is_empty());
    }

    #[test]
    fn unknown_qualification_holds_instead_of_admitting() {
        let mut frame = sample_frame();
        frame["static_case"]["resources"][0]["initial_qualifications"][0]["disposition"] =
            Value::from("UNKNOWN");
        let receipt = evaluate(&frame, &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"qualification_unknown_at_decision_cutoff".to_string())
        );
    }

    #[test]
    fn visible_scoped_receipt_can_admit_an_initially_unknown_route() {
        let mut frame = sample_frame();
        frame["static_case"]["resources"][0]["initial_qualifications"][0]["disposition"] =
            Value::from("UNKNOWN");
        frame["observable_history"][0]["qualification_updates"] = json!([{
            "evidence_receipt_id": "Q-CUT-LATE",
            "resource_id": "RESOURCE-A",
            "process_family": "CUTTING",
            "previous_disposition": "UNKNOWN",
            "new_disposition": "ADMISSIBLE",
            "occurred_at_epoch": 0,
            "observed_at_epoch": 0
        }]);
        let receipt = evaluate(&frame, &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Admissible);
        assert_eq!(
            receipt.qualification_disposition.as_deref(),
            Some("ADMISSIBLE")
        );
    }

    #[test]
    fn blocked_qualification_rejects() {
        let mut frame = sample_frame();
        frame["static_case"]["resources"][0]["initial_qualifications"][0]["disposition"] =
            Value::from("BLOCKED");
        let receipt = evaluate(&frame, &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Rejected);
        assert!(
            receipt
                .reason_codes
                .contains(&"qualification_blocked_at_decision_cutoff".to_string())
        );
    }

    #[test]
    fn visible_resource_failure_holds_at_the_first_decision_cutoff() {
        let mut frame = sample_frame();
        frame["observable_history"][0]["resource_events"] = json!([{
            "source_event_id": "FAIL-AT-EPOCH-0",
            "resource_id": "RESOURCE-A",
            "kind": "FAILED",
            "occurred_at_epoch": 0,
            "observed_at_epoch": 0,
            "common_cause_id": null,
            "recovery_forecast_min_epochs": 1,
            "recovery_forecast_max_epochs": 2
        }]);
        let receipt = evaluate(&frame, &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"resource_has_visible_unrestored_failure".to_string())
        );
    }

    #[test]
    fn later_epoch_requires_live_operation_state_even_after_restoration() {
        let mut frame = sample_frame();
        frame["decision_epoch"] = Value::from(1);
        frame["observable_history"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "epoch": 1, "resource_events": [{
                    "source_event_id": "FAIL-1", "resource_id": "RESOURCE-A", "kind": "FAILED",
                    "occurred_at_epoch": 0, "observed_at_epoch": 1,
                    "common_cause_id": null, "recovery_forecast_min_epochs": 1,
                    "recovery_forecast_max_epochs": 2
                }, {
                    "source_event_id": "RESTORE-1", "resource_id": "RESOURCE-A", "kind": "RESTORED",
                    "occurred_at_epoch": 1, "observed_at_epoch": 1,
                    "common_cause_id": null, "recovery_forecast_min_epochs": null,
                    "recovery_forecast_max_epochs": null
                }],
                "qualification_updates": [], "inspection_capacity_events": []
            }));
        let mut proposal = sample_proposal();
        proposal["decision_epoch"] = Value::from(1);
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"dynamic_operation_state_snapshot_not_yet_supported".to_string())
        );
    }

    #[test]
    fn uncompleted_predecessor_holds_a_dependent_start() {
        let mut frame = sample_frame();
        frame["static_case"]["jobs"][0]["operations"][0]["initial_status"] = Value::from("WAITING");
        let mut proposal = sample_proposal();
        proposal["operation_id"] = Value::from("OP-2");
        proposal["inspection_method_id"] = Value::Null;
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"predecessor_not_completed_at_decision_cutoff".to_string())
        );
    }

    #[test]
    fn resource_failure_and_capacity_are_checked_independently() {
        let mut frame = sample_frame();
        frame["static_case"]["resources"][0]["capacity_units_by_epoch"][0] = Value::from(0_u64);
        let receipt = evaluate(&frame, &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"resource_capacity_unavailable_in_current_epoch".to_string())
        );
    }

    #[test]
    fn committed_operation_must_continue_on_its_committed_resource() {
        let mut frame = sample_frame();
        let operation = &mut frame["static_case"]["jobs"][1]["operations"][0];
        operation["initial_status"] = Value::from("IN_PROCESS");
        operation["committed_resource_id"] = Value::from("RESOURCE-A");
        operation["remaining_processing_ticks"] = Value::from(3_u64);
        let mut proposal = sample_proposal();
        proposal["action_kind"] = Value::from("CONTINUE");
        proposal["inspection_method_id"] = Value::from("DIMENSIONAL");
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Admissible);
        assert_eq!(receipt.processing_duration_ticks, Some(3));
    }

    #[test]
    fn inspection_capacity_shortfall_holds_required_inspection() {
        let mut frame = sample_frame();
        frame["observable_history"][0]["inspection_capacity_events"] = json!([{
            "source_event_id": "INSPECT-CAP-0", "station_id": "INSPECT-1",
            "available_capacity_units": 0, "occurred_at_epoch": 0, "observed_at_epoch": 0
        }]);
        let receipt = evaluate(&frame, &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"inspection_capacity_unavailable_in_current_epoch".to_string())
        );
    }

    #[test]
    fn missing_setup_transition_rejects() {
        let mut frame = sample_frame();
        frame["static_case"]["setup_transitions"] = json!([]);
        let receipt = evaluate(&frame, &sample_proposal()).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Rejected);
        assert!(
            receipt
                .reason_codes
                .contains(&"setup_transition_missing".to_string())
        );
    }

    #[test]
    fn duplicate_operation_identity_fails_closed() {
        let mut frame = sample_frame();
        let duplicate = frame["static_case"]["jobs"][1]["operations"][0].clone();
        frame["static_case"]["jobs"][0]["operations"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        assert_eq!(
            evaluate(&frame, &sample_proposal()).unwrap_err(),
            "operation_id_duplicate"
        );
    }

    #[test]
    fn future_observation_cannot_be_smuggled_into_decision_frame() {
        let mut frame = sample_frame();
        frame["observable_history"]
            .as_array_mut()
            .unwrap()
            .push(json!({
                "epoch": 1, "resource_events": [], "qualification_updates": [],
                "inspection_capacity_events": []
            }));
        assert_eq!(
            evaluate(&frame, &sample_proposal()).unwrap_err(),
            "observable_history_must_end_at_decision_epoch"
        );
    }

    #[test]
    fn proposal_from_another_frame_digest_is_rejected() {
        let frame = sample_frame();
        let mut proposal = sample_proposal();
        proposal["decision_frame_digest"] = Value::from("0".repeat(64));
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Rejected);
        assert_eq!(receipt.reason_codes, vec!["proposal_frame_digest_mismatch"]);
        assert_ne!(receipt.decision_frame_digest, proposal["decision_frame_digest"]);
    }

    #[test]
    fn proposal_for_another_case_or_epoch_is_rejected() {
        let frame = sample_frame();
        let mut proposal = sample_proposal();
        proposal["case_id"] = Value::from("OTHER-CASE");
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Rejected);
        assert_eq!(
            receipt.reason_codes,
            vec!["proposal_frame_identity_mismatch"]
        );
    }
}
