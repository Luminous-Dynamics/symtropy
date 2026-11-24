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
const SNAPSHOT_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_policy_v02_operational_state_snapshot.schema.json"
);
const REPLAY_RECEIPT_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_replay_receipt.schema.json");

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
    visible_source_event_ids: BTreeSet<String>,
}

#[derive(Debug, Clone)]
struct LiveOperationState {
    status: String,
    remaining_processing_ticks: u64,
    committed_resource_id: Option<String>,
}

#[derive(Debug, Clone)]
struct LiveResourceState {
    current_setup_family: String,
    active_operation_id: Option<String>,
    availability_status: String,
    capacity_units_total: u64,
    capacity_units_used: u64,
    capacity_units_remaining: u64,
}

#[derive(Debug, Clone)]
struct LiveAdmissionState {
    operations: BTreeMap<String, LiveOperationState>,
    resources: BTreeMap<String, LiveResourceState>,
    inspection_capacity_remaining: u64,
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
        visible_source_event_ids: evidence_receipts,
    })
}

fn valid_digest(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_replay_receipt_chain(
    replay_receipt: &Value,
    frame: &Value,
    decision_epoch: u64,
) -> Result<Value, String> {
    validate_schema(
        replay_receipt,
        REPLAY_RECEIPT_SCHEMA_JSON,
        "replay_receipt",
    )?;
    let benchmark_id = required_string(frame, "benchmark_id", "decision_frame")?;
    let case_id = required_string(frame, "case_id", "decision_frame")?;
    if required_string(replay_receipt, "status", "replay_receipt")? != "VERIFIED"
        || required_string(replay_receipt, "benchmark_id", "replay_receipt")? != benchmark_id
        || required_string(replay_receipt, "case_id", "replay_receipt")? != case_id
    {
        return Err(fail("replay_receipt_identity_or_status_invalid"));
    }

    let ledger_digest = required_string(replay_receipt, "ledger_digest", "replay_receipt")?;
    if !valid_digest(ledger_digest) {
        return Err(fail("replay_receipt_ledger_digest_invalid"));
    }
    if !required_array(replay_receipt, "violation_ledger", "replay_receipt")?.is_empty() {
        return Err(fail("replay_receipt_contains_violations"));
    }

    let snapshots = required_array(replay_receipt, "snapshots", "replay_receipt")?;
    if snapshots.len() as u64 != decision_epoch + 1 {
        return Err(fail("replay_receipt_snapshot_chain_length_mismatch"));
    }

    let mut previous_snapshot_digest: Option<String> = None;
    let mut previous_transition_ids = Vec::<String>::new();
    let mut previous_event_ids = BTreeSet::<String>::new();
    let mut last_digest = None;

    for (expected_epoch, snapshot) in snapshots.iter().enumerate() {
        validate_schema(snapshot, SNAPSHOT_SCHEMA_JSON, "operational_state_snapshot")?;
        if required_string(snapshot, "benchmark_id", "operational_state_snapshot")?
            != benchmark_id
            || required_string(snapshot, "case_id", "operational_state_snapshot")? != case_id
            || required_u64(snapshot, "decision_epoch", "operational_state_snapshot")?
                != expected_epoch as u64
        {
            return Err(fail("replay_receipt_snapshot_identity_or_order_invalid"));
        }

        match previous_snapshot_digest.as_deref() {
            None => {
                if !snapshot
                    .get("previous_snapshot_digest")
                    .is_some_and(Value::is_null)
                {
                    return Err(fail("replay_receipt_genesis_snapshot_has_parent"));
                }
            }
            Some(expected_parent) => {
                if required_string(
                    snapshot,
                    "previous_snapshot_digest",
                    "operational_state_snapshot",
                )? != expected_parent
                {
                    return Err(fail("replay_receipt_snapshot_parent_link_mismatch"));
                }
            }
        }

        let mut snapshot_body = snapshot.clone();
        snapshot_body
            .as_object_mut()
            .ok_or_else(|| fail("snapshot_must_be_object"))?
            .remove("snapshot_digest");
        let recomputed_digest = digest_value(&snapshot_body)?;
        if required_string(snapshot, "snapshot_digest", "operational_state_snapshot")?
            != recomputed_digest
        {
            return Err(fail("replay_receipt_snapshot_digest_mismatch"));
        }

        let transition_ids = required_array(
            snapshot,
            "applied_transition_ids",
            "operational_state_snapshot",
        )?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| fail("replay_receipt_transition_id_invalid"))
        })
        .collect::<Result<Vec<_>, _>>()?;
        if transition_ids.len() < previous_transition_ids.len()
            || transition_ids[..previous_transition_ids.len()] != previous_transition_ids
        {
            return Err(fail("replay_receipt_transition_lineage_not_monotonic"));
        }
        previous_transition_ids = transition_ids;

        let event_cut = snapshot
            .get("event_cut")
            .ok_or_else(|| fail("snapshot_event_cut_missing"))?;
        if required_u64(event_cut, "decision_epoch", "snapshot_event_cut")?
            != expected_epoch as u64
        {
            return Err(fail("replay_receipt_event_cut_epoch_mismatch"));
        }
        let event_ids = required_array(
            event_cut,
            "visible_source_event_ids",
            "snapshot_event_cut",
        )?
        .iter()
        .map(|value| {
            value
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| fail("replay_receipt_event_id_invalid"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
        if !previous_event_ids.is_subset(&event_ids) {
            return Err(fail("replay_receipt_event_cut_not_monotonic"));
        }
        previous_event_ids = event_ids;

        previous_snapshot_digest = Some(recomputed_digest.clone());
        last_digest = Some(recomputed_digest);
    }

    let last_digest = last_digest.ok_or_else(|| fail("replay_receipt_snapshot_chain_empty"))?;
    if required_string(replay_receipt, "final_snapshot_digest", "replay_receipt")?
        != last_digest
    {
        return Err(fail("replay_receipt_final_snapshot_digest_mismatch"));
    }

    let target = snapshots
        .last()
        .ok_or_else(|| fail("replay_receipt_snapshot_chain_empty"))?;
    let frame_digest = digest_value(frame)?;
    if required_string(target, "decision_frame_digest", "operational_state_snapshot")?
        != frame_digest
    {
        return Err(fail("replay_receipt_target_frame_digest_mismatch"));
    }
    Ok(target.clone())
}

fn initial_live_state(
    static_case: &Value,
    resources: &BTreeMap<String, &Value>,
    operations: &BTreeMap<String, (&Value, &Value)>,
    visible: &VisibleEvidenceState,
    decision_epoch: u64,
) -> Result<LiveAdmissionState, String> {
    let mut operation_states = BTreeMap::new();
    let mut resource_states = BTreeMap::new();
    let mut active_by_resource = BTreeMap::<String, String>::new();

    for (operation_id, (_, operation)) in operations {
        let status = required_string(operation, "initial_status", "operation")?.to_string();
        let remaining = required_u64(operation, "remaining_processing_ticks", "operation")?;
        let committed = operation
            .get("committed_resource_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        match status.as_str() {
            "WAITING" if remaining == 0 && committed.is_none() => {}
            "IN_PROCESS" if remaining > 0 && committed.is_some() => {
                let resource_id = committed.as_ref().expect("checked Some");
                if active_by_resource
                    .insert(resource_id.clone(), operation_id.clone())
                    .is_some()
                {
                    return Err(fail("initial_resource_multiple_active_operations"));
                }
            }
            "COMPLETED" if remaining == 0 && committed.is_none() => {}
            _ => return Err(fail("initial_operation_state_inconsistent")),
        }
        operation_states.insert(
            operation_id.clone(),
            LiveOperationState {
                status,
                remaining_processing_ticks: remaining,
                committed_resource_id: committed,
            },
        );
    }

    for (resource_id, resource) in resources {
        let calendar = required_array(resource, "capacity_units_by_epoch", "resource")?;
        let capacity_total = calendar
            .get(decision_epoch as usize)
            .and_then(Value::as_u64)
            .ok_or_else(|| fail("resource_capacity_for_decision_epoch_missing"))?;
        let availability = visible
            .resource_status_by_id
            .get(resource_id)
            .map(|status| {
                if status == "FAILED" {
                    "FAILED"
                } else {
                    "AVAILABLE"
                }
            })
            .unwrap_or("AVAILABLE")
            .to_string();
        let active_operation_id = active_by_resource.get(resource_id).cloned();
        let capacity_used = u64::from(
            active_operation_id.is_some() && capacity_total > 0 && availability == "AVAILABLE",
        );
        resource_states.insert(
            resource_id.clone(),
            LiveResourceState {
                current_setup_family: required_string(
                    resource,
                    "initial_setup_family",
                    "resource",
                )?
                .to_string(),
                active_operation_id,
                availability_status: availability,
                capacity_units_total: capacity_total,
                capacity_units_used: capacity_used,
                capacity_units_remaining: capacity_total.saturating_sub(capacity_used),
            },
        );
    }

    let station = static_case
        .get("inspection_station")
        .ok_or_else(|| fail("inspection_station_missing"))?;
    let station_calendar =
        required_array(station, "capacity_units_by_epoch", "inspection_station")?;
    let mut inspection_capacity_total = station_calendar
        .get(decision_epoch as usize)
        .and_then(Value::as_u64)
        .ok_or_else(|| fail("inspection_capacity_for_decision_epoch_missing"))?;
    if let Some(override_capacity) = visible.inspection_capacity_override {
        if override_capacity > inspection_capacity_total {
            return Err(fail("inspection_capacity_event_exceeds_static_bound"));
        }
        inspection_capacity_total = override_capacity;
    }

    Ok(LiveAdmissionState {
        operations: operation_states,
        resources: resource_states,
        inspection_capacity_remaining: inspection_capacity_total,
    })
}

struct LiveSnapshotValidationContext<'a> {
    frame: &'a Value,
    static_case: &'a Value,
    resources: &'a BTreeMap<String, &'a Value>,
    operations: &'a BTreeMap<String, (&'a Value, &'a Value)>,
    visible: &'a VisibleEvidenceState,
    decision_frame_digest: &'a str,
    decision_epoch: u64,
}

fn snapshot_live_state(
    snapshot: &Value,
    context: LiveSnapshotValidationContext<'_>,
) -> Result<LiveAdmissionState, String> {
    let LiveSnapshotValidationContext {
        frame,
        static_case,
        resources,
        operations,
        visible,
        decision_frame_digest,
        decision_epoch,
    } = context;
    validate_schema(snapshot, SNAPSHOT_SCHEMA_JSON, "operational_state_snapshot")?;

    if required_string(snapshot, "benchmark_id", "operational_state_snapshot")?
        != required_string(frame, "benchmark_id", "decision_frame")?
        || required_string(snapshot, "case_id", "operational_state_snapshot")?
            != required_string(frame, "case_id", "decision_frame")?
        || required_u64(snapshot, "decision_epoch", "operational_state_snapshot")? != decision_epoch
        || required_string(
            snapshot,
            "decision_frame_digest",
            "operational_state_snapshot",
        )? != decision_frame_digest
    {
        return Err(fail("snapshot_decision_frame_identity_or_digest_mismatch"));
    }

    let history = required_array(frame, "observable_history", "decision_frame")?;
    let expected_history_digest = digest_value(&Value::Array(history.clone()))?;
    if required_string(
        snapshot,
        "visible_history_digest",
        "operational_state_snapshot",
    )? != expected_history_digest
    {
        return Err(fail("snapshot_visible_history_digest_mismatch"));
    }

    let event_cut = snapshot
        .get("event_cut")
        .ok_or_else(|| fail("snapshot_event_cut_missing"))?;
    if required_u64(event_cut, "decision_epoch", "snapshot_event_cut")? != decision_epoch {
        return Err(fail("snapshot_event_cut_epoch_mismatch"));
    }
    let ids = required_array(event_cut, "visible_source_event_ids", "snapshot_event_cut")?;
    let snapshot_ids = ids
        .iter()
        .map(|id| {
            id.as_str()
                .filter(|value| !value.trim().is_empty())
                .map(str::to_string)
                .ok_or_else(|| fail("snapshot_event_cut_id_invalid"))
        })
        .collect::<Result<BTreeSet<_>, _>>()?;
    if snapshot_ids != visible.visible_source_event_ids {
        return Err(fail("snapshot_event_cut_does_not_match_visible_evidence"));
    }

    let mut snapshot_body = snapshot.clone();
    snapshot_body
        .as_object_mut()
        .ok_or_else(|| fail("snapshot_must_be_object"))?
        .remove("snapshot_digest");
    let recomputed_snapshot_digest = digest_value(&snapshot_body)?;
    if required_string(snapshot, "snapshot_digest", "operational_state_snapshot")?
        != recomputed_snapshot_digest
    {
        return Err(fail("snapshot_digest_mismatch"));
    }

    if decision_epoch > 0 {
        let previous = required_string(
            snapshot,
            "previous_snapshot_digest",
            "operational_state_snapshot",
        )?;
        if !valid_digest(previous)
            || previous
                == required_string(snapshot, "snapshot_digest", "operational_state_snapshot")?
        {
            return Err(fail("snapshot_previous_digest_invalid"));
        }
        if required_array(
            snapshot,
            "applied_transition_ids",
            "operational_state_snapshot",
        )?
        .is_empty()
        {
            return Err(fail("snapshot_missing_applied_transition_lineage"));
        }
    }

    let snapshot_operations = required_array(snapshot, "operations", "operational_state_snapshot")?;
    let mut operation_states = BTreeMap::new();
    for item in snapshot_operations {
        let operation_id = required_string(item, "operation_id", "snapshot_operation")?;
        let status = required_string(item, "status", "snapshot_operation")?.to_string();
        let remaining = required_u64(item, "remaining_processing_ticks", "snapshot_operation")?;
        let committed = item
            .get("committed_resource_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        let quality = item.get("quality_disposition").and_then(Value::as_str);
        let (_, static_operation) = operations
            .get(operation_id)
            .ok_or_else(|| fail("snapshot_unknown_operation"))?;
        let inspection_required = static_operation
            .get("inspection_required")
            .and_then(Value::as_bool)
            .ok_or_else(|| fail("snapshot_static_inspection_requirement_missing"))?;
        let state_is_consistent = match status.as_str() {
            "WAITING" => remaining == 0 && committed.is_none() && quality.is_none(),
            "IN_PROCESS" => remaining > 0 && committed.is_some() && quality.is_none(),
            "AWAITING_INSPECTION" => {
                inspection_required && remaining == 0 && committed.is_none() && quality.is_none()
            }
            "COMPLETED" => {
                remaining == 0 && committed.is_none() && (inspection_required == quality.is_some())
            }
            _ => false,
        };
        if !state_is_consistent {
            return Err(fail("snapshot_operation_state_inconsistent"));
        }
        if operation_states
            .insert(
                operation_id.to_string(),
                LiveOperationState {
                    status,
                    remaining_processing_ticks: remaining,
                    committed_resource_id: committed,
                },
            )
            .is_some()
        {
            return Err(fail("snapshot_operation_id_duplicate"));
        }
    }
    if operation_states.len() != operations.len()
        || operations
            .keys()
            .any(|id| !operation_states.contains_key(id))
    {
        return Err(fail("snapshot_operation_set_mismatch"));
    }

    for (operation_id, (_, static_operation)) in operations {
        let operation_state = operation_states
            .get(operation_id)
            .ok_or_else(|| fail("snapshot_operation_state_missing"))?;
        if operation_state.status != "WAITING" {
            for predecessor in
                required_array(static_operation, "predecessor_operation_ids", "operation")?
            {
                let predecessor_id = predecessor
                    .as_str()
                    .filter(|value| !value.trim().is_empty())
                    .ok_or_else(|| fail("snapshot_predecessor_id_invalid"))?;
                if operation_states
                    .get(predecessor_id)
                    .is_none_or(|state| state.status != "COMPLETED")
                {
                    return Err(fail("snapshot_predecessor_state_inconsistent"));
                }
            }
        }
        if operation_state.status == "IN_PROCESS" {
            let resource_id = operation_state
                .committed_resource_id
                .as_deref()
                .ok_or_else(|| fail("snapshot_in_process_commitment_missing"))?;
            let process_family = required_string(static_operation, "process_family", "operation")?;
            let eligible = required_array(static_operation, "eligible_resource_ids", "operation")?;
            let static_resource = resources
                .get(resource_id)
                .ok_or_else(|| fail("snapshot_in_process_resource_unknown"))?;
            let capabilities = required_array(static_resource, "capability_families", "resource")?;
            if !eligible
                .iter()
                .any(|item| item.as_str() == Some(resource_id))
                || !capabilities
                    .iter()
                    .any(|item| item.as_str() == Some(process_family))
            {
                return Err(fail("snapshot_in_process_resource_not_eligible_or_capable"));
            }
        }
    }

    let mut active_by_resource = BTreeMap::<String, String>::new();
    for (operation_id, state) in &operation_states {
        if state.status == "IN_PROCESS" {
            let resource_id = state
                .committed_resource_id
                .as_ref()
                .ok_or_else(|| fail("snapshot_in_process_commitment_missing"))?;
            if !resources.contains_key(resource_id)
                || active_by_resource
                    .insert(resource_id.clone(), operation_id.clone())
                    .is_some()
            {
                return Err(fail("snapshot_resource_commitment_conflict"));
            }
        }
    }

    let snapshot_resources = required_array(snapshot, "resources", "operational_state_snapshot")?;
    let mut resource_states = BTreeMap::new();
    for item in snapshot_resources {
        let resource_id = required_string(item, "resource_id", "snapshot_resource")?;
        let static_resource = resources
            .get(resource_id)
            .ok_or_else(|| fail("snapshot_unknown_resource"))?;
        let current_setup_family =
            required_string(item, "current_setup_family", "snapshot_resource")?.to_string();
        let availability =
            required_string(item, "availability_status", "snapshot_resource")?.to_string();
        let capacity_total = required_u64(item, "capacity_units_total", "snapshot_resource")?;
        let capacity_used = required_u64(item, "capacity_units_used", "snapshot_resource")?;
        let capacity_remaining =
            required_u64(item, "capacity_units_remaining", "snapshot_resource")?;
        let static_calendar =
            required_array(static_resource, "capacity_units_by_epoch", "resource")?;
        let expected_total = static_calendar
            .get(decision_epoch as usize)
            .and_then(Value::as_u64)
            .ok_or_else(|| fail("resource_capacity_for_decision_epoch_missing"))?;
        if capacity_total != expected_total
            || capacity_total > 1
            || capacity_used > capacity_total
            || capacity_remaining != capacity_total - capacity_used
            || !["AVAILABLE", "FAILED"].contains(&availability.as_str())
        {
            return Err(fail("snapshot_resource_capacity_or_availability_invalid"));
        }
        let expected_availability = visible
            .resource_status_by_id
            .get(resource_id)
            .map(|status| {
                if status == "FAILED" {
                    "FAILED"
                } else {
                    "AVAILABLE"
                }
            })
            .unwrap_or("AVAILABLE");
        if availability != expected_availability {
            return Err(fail("snapshot_resource_availability_mismatch"));
        }

        let active_operation_id = item
            .get("active_operation_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        if active_operation_id != active_by_resource.get(resource_id).cloned() {
            return Err(fail("snapshot_active_operation_mismatch"));
        }
        let expected_used = u64::from(
            active_operation_id.is_some() && capacity_total > 0 && availability == "AVAILABLE",
        );
        if capacity_used != expected_used {
            return Err(fail("snapshot_resource_occupancy_mismatch"));
        }

        let mut known_setup_families = BTreeSet::new();
        known_setup_families.insert(
            required_string(static_resource, "initial_setup_family", "resource")?.to_string(),
        );
        for operation in operations.values().map(|(_, operation)| *operation) {
            known_setup_families
                .insert(required_string(operation, "setup_family", "operation")?.to_string());
        }
        for transition in required_array(static_case, "setup_transitions", "static_case")? {
            if transition.get("resource_id").and_then(Value::as_str) == Some(resource_id) {
                known_setup_families.insert(
                    required_string(transition, "from_setup_family", "setup_transition")?
                        .to_string(),
                );
                known_setup_families.insert(
                    required_string(transition, "to_setup_family", "setup_transition")?.to_string(),
                );
            }
        }
        if !known_setup_families.contains(&current_setup_family) {
            return Err(fail("snapshot_current_setup_family_unknown"));
        }

        if resource_states
            .insert(
                resource_id.to_string(),
                LiveResourceState {
                    current_setup_family,
                    active_operation_id,
                    availability_status: availability,
                    capacity_units_total: capacity_total,
                    capacity_units_used: capacity_used,
                    capacity_units_remaining: capacity_remaining,
                },
            )
            .is_some()
        {
            return Err(fail("snapshot_resource_id_duplicate"));
        }
    }
    if resource_states.len() != resources.len()
        || resources.keys().any(|id| !resource_states.contains_key(id))
    {
        return Err(fail("snapshot_resource_set_mismatch"));
    }

    for (operation_id, state) in &operation_states {
        if state.status == "IN_PROCESS" {
            let resource_id = state
                .committed_resource_id
                .as_deref()
                .ok_or_else(|| fail("snapshot_in_process_commitment_missing"))?;
            if resource_states
                .get(resource_id)
                .and_then(|resource| resource.active_operation_id.as_deref())
                != Some(operation_id.as_str())
            {
                return Err(fail("snapshot_operation_resource_binding_mismatch"));
            }
        }
    }

    let static_station = static_case
        .get("inspection_station")
        .ok_or_else(|| fail("inspection_station_missing"))?;
    let station_snapshot = snapshot
        .get("inspection_station")
        .ok_or_else(|| fail("snapshot_inspection_station_missing"))?;
    let station_id = required_string(
        station_snapshot,
        "station_id",
        "snapshot_inspection_station",
    )?;
    if station_id != required_string(static_station, "station_id", "inspection_station")? {
        return Err(fail("snapshot_inspection_station_identity_mismatch"));
    }
    let station_calendar = required_array(
        static_station,
        "capacity_units_by_epoch",
        "inspection_station",
    )?;
    let static_station_capacity = station_calendar
        .get(decision_epoch as usize)
        .and_then(Value::as_u64)
        .ok_or_else(|| fail("inspection_capacity_for_decision_epoch_missing"))?;
    let expected_station_capacity = visible
        .inspection_capacity_override
        .unwrap_or(static_station_capacity);
    let station_total = required_u64(
        station_snapshot,
        "capacity_units_total",
        "snapshot_inspection_station",
    )?;
    let station_used = required_u64(
        station_snapshot,
        "capacity_units_used",
        "snapshot_inspection_station",
    )?;
    let station_remaining = required_u64(
        station_snapshot,
        "capacity_units_remaining",
        "snapshot_inspection_station",
    )?;
    if station_total != expected_station_capacity
        || station_used > station_total
        || station_remaining != station_total - station_used
    {
        return Err(fail("snapshot_inspection_capacity_inconsistent"));
    }

    Ok(LiveAdmissionState {
        operations: operation_states,
        resources: resource_states,
        inspection_capacity_remaining: station_remaining,
    })
}

fn evaluate(frame: &Value, proposal: &Value) -> Result<AdmissionReceipt, String> {
    evaluate_with_replay_receipt(frame, proposal, None)
}

fn evaluate_with_replay_receipt(
    frame: &Value,
    proposal: &Value,
    replay_receipt: Option<&Value>,
) -> Result<AdmissionReceipt, String> {
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

    // A post-epoch action is never evaluated against immutable initial state.
    if decision_epoch > 0 && replay_receipt.is_none() {
        mark_hold(&mut receipt, "trusted_operational_state_snapshot_required");
        return Ok(receipt);
    }

    let operations = operation_index(static_case)?;
    let resources = resource_index(static_case)?;
    let visible_state = validate_visible_history(frame, static_case, &resources, decision_epoch)?;

    let live_state = match replay_receipt {
        Some(replay_receipt) => {
            let target_snapshot =
                match validate_replay_receipt_chain(replay_receipt, frame, decision_epoch) {
                    Ok(snapshot) => snapshot,
                    Err(error) => {
                        mark_hold(&mut receipt, &format!("replay_receipt_invalid:{error}"));
                        return Ok(receipt);
                    }
                };
            match snapshot_live_state(
                &target_snapshot,
                LiveSnapshotValidationContext {
                    frame,
                    static_case,
                    resources: &resources,
                    operations: &operations,
                    visible: &visible_state,
                    decision_frame_digest: &decision_frame_digest,
                    decision_epoch,
                },
            ) {
                Ok(state) => state,
                Err(error) => {
                    mark_hold(&mut receipt, &format!("snapshot_invalid:{error}"));
                    return Ok(receipt);
                }
            }
        },
        None if decision_epoch == 0 => initial_live_state(
            static_case,
            &resources,
            &operations,
            &visible_state,
            decision_epoch,
        )?,
        None => {
            mark_hold(&mut receipt, "trusted_operational_state_snapshot_required");
            return Ok(receipt);
        }
    };

    let Some((_, static_operation)) = operations.get(&operation_id).copied() else {
        mark_rejected(&mut receipt, "operation_not_found");
        return Ok(receipt);
    };
    let Some(static_resource) = resources.get(&resource_id).copied() else {
        mark_rejected(&mut receipt, "resource_not_found");
        return Ok(receipt);
    };
    let Some(operation_state) = live_state.operations.get(&operation_id) else {
        mark_hold(&mut receipt, "snapshot_operation_state_missing");
        return Ok(receipt);
    };
    let Some(resource_state) = live_state.resources.get(&resource_id) else {
        mark_hold(&mut receipt, "snapshot_resource_state_missing");
        return Ok(receipt);
    };

    let process_family = required_string(static_operation, "process_family", "operation")?;
    let target_setup = required_string(static_operation, "setup_family", "operation")?;
    let action_kind = required_string(proposal, "action_kind", "action_proposal")?;

    match (action_kind, operation_state.status.as_str()) {
        ("START", "WAITING") | ("CONTINUE", "IN_PROCESS") => {}
        ("START", "IN_PROCESS") => {
            mark_rejected(&mut receipt, "in_process_operation_requires_continue");
        }
        ("CONTINUE", "WAITING") => {
            mark_rejected(&mut receipt, "waiting_operation_cannot_continue");
        }
        (_, "AWAITING_INSPECTION") => {
            mark_rejected(
                &mut receipt,
                "operation_awaiting_inspection_cannot_be_dispatched",
            );
        }
        (_, "COMPLETED") => {
            mark_rejected(&mut receipt, "completed_operation_cannot_be_dispatched");
        }
        _ => mark_rejected(&mut receipt, "operation_action_state_mismatch"),
    }
    if receipt.status == AdmissionStatus::Rejected {
        return Ok(receipt);
    }

    let capabilities = required_array(static_resource, "capability_families", "resource")?;
    if !capabilities
        .iter()
        .any(|value| value.as_str() == Some(process_family))
    {
        mark_rejected(&mut receipt, "resource_lacks_process_capability");
    }
    let eligible = required_array(static_operation, "eligible_resource_ids", "operation")?;
    if !eligible
        .iter()
        .any(|value| value.as_str() == Some(resource_id.as_str()))
    {
        mark_rejected(&mut receipt, "resource_not_operation_eligible");
    }
    if receipt.status == AdmissionStatus::Rejected {
        return Ok(receipt);
    }

    if resource_state.availability_status != "AVAILABLE" {
        mark_hold(&mut receipt, "resource_has_visible_unrestored_failure");
    }
    if resource_state.capacity_units_total == 0 {
        mark_hold(
            &mut receipt,
            "resource_capacity_unavailable_in_current_epoch",
        );
    }

    if action_kind == "START" {
        if operation_state.committed_resource_id.is_some() {
            mark_rejected(
                &mut receipt,
                "waiting_operation_has_existing_resource_commitment",
            );
            return Ok(receipt);
        }
        if resource_state.active_operation_id.is_some() {
            mark_rejected(&mut receipt, "resource_already_has_active_operation");
            return Ok(receipt);
        }
        if resource_state.capacity_units_remaining == 0 {
            mark_hold(
                &mut receipt,
                "resource_capacity_unavailable_in_current_epoch",
            );
        }
        let times = static_operation
            .get("processing_time_ticks_by_resource")
            .and_then(Value::as_object)
            .ok_or_else(|| fail("operation_processing_times_missing"))?;
        let duration = times
            .get(&resource_id)
            .and_then(Value::as_u64)
            .filter(|value| *value > 0)
            .ok_or_else(|| fail("eligible_resource_processing_time_missing_or_zero"))?;
        receipt.processing_duration_ticks = Some(duration);
    } else {
        if operation_state.committed_resource_id.as_deref() != Some(resource_id.as_str())
            || resource_state.active_operation_id.as_deref() != Some(operation_id.as_str())
        {
            mark_rejected(&mut receipt, "continuation_resource_commitment_mismatch");
            return Ok(receipt);
        }
        if operation_state.remaining_processing_ticks == 0 {
            mark_rejected(&mut receipt, "continuation_has_no_remaining_work");
            return Ok(receipt);
        }
        if resource_state.capacity_units_used == 0 || resource_state.capacity_units_remaining != 0 {
            mark_hold(
                &mut receipt,
                "continuation_resource_occupancy_not_reconciled",
            );
        }
        receipt.processing_duration_ticks = Some(operation_state.remaining_processing_ticks);
    }

    let predecessors = required_array(static_operation, "predecessor_operation_ids", "operation")?;
    for predecessor in predecessors {
        let predecessor_id = predecessor
            .as_str()
            .filter(|text| !text.trim().is_empty())
            .ok_or_else(|| fail("predecessor_operation_id_invalid"))?;
        let Some(predecessor_state) = live_state.operations.get(predecessor_id) else {
            mark_hold(&mut receipt, "predecessor_live_state_missing");
            continue;
        };
        if predecessor_state.status != "COMPLETED" {
            mark_hold(&mut receipt, "predecessor_not_completed_at_decision_cutoff");
        }
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
        let setup_duration = match current_setup_duration(
            static_case,
            &resource_id,
            &resource_state.current_setup_family,
            target_setup,
        ) {
            Ok(duration) => duration,
            Err(error) if error == "setup_transition_missing" => {
                mark_rejected(&mut receipt, "setup_transition_missing");
                0
            }
            Err(error) => return Err(error),
        };
        receipt.setup_duration_ticks = Some(setup_duration);
    } else {
        receipt.setup_duration_ticks = Some(0);
    }

    let inspection_required = static_operation
        .get("inspection_required")
        .and_then(Value::as_bool)
        .ok_or_else(|| fail("operation_inspection_required_missing"))?;
    let proposed_method = proposal
        .get("inspection_method_id")
        .ok_or_else(|| fail("proposal_inspection_method_missing"))?;
    let station = static_case
        .get("inspection_station")
        .ok_or_else(|| fail("inspection_station_missing"))?;

    if inspection_required {
        let required_method =
            required_string(static_operation, "inspection_method_id", "operation")?;
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
        if live_state.inspection_capacity_remaining == 0 {
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

fn current_setup_duration(
    static_case: &Value,
    resource_id: &str,
    from_setup: &str,
    to_setup: &str,
) -> Result<u64, String> {
    if from_setup == to_setup {
        return Ok(0);
    }
    let transitions = required_array(static_case, "setup_transitions", "static_case")?;
    let matches = transitions
        .iter()
        .filter(|transition| {
            transition.get("resource_id").and_then(Value::as_str) == Some(resource_id)
                && transition.get("from_setup_family").and_then(Value::as_str) == Some(from_setup)
                && transition.get("to_setup_family").and_then(Value::as_str) == Some(to_setup)
        })
        .collect::<Vec<_>>();
    match matches.as_slice() {
        [transition] => required_u64(transition, "duration_ticks", "setup_transition"),
        [] => Err(fail("setup_transition_missing")),
        _ => Err(fail("setup_transition_duplicate")),
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 && args.len() != 4 {
        return Err(
            "usage: mfg-policy-v02-action-admission <decision-frame.json> <action-proposal.json> [operational-state-snapshot.json]"
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
    let snapshot = if args.len() == 4 {
        let bytes = std::fs::read(&args[3])
            .map_err(|error| fail(format!("operational_state_snapshot_read:{error}")))?;
        Some(
            serde_json::from_slice(&bytes)
                .map_err(|error| fail(format!("operational_state_snapshot_json:{error}")))?,
        )
    } else {
        None
    };
    let receipt = match snapshot.as_ref() {
        Some(snapshot) => evaluate_with_snapshot(&frame, &proposal, Some(snapshot))?,
        None => evaluate(&frame, &proposal)?,
    };
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

    fn frame_for_epoch(epoch: u64) -> Value {
        let mut frame = sample_frame();
        let history = frame["observable_history"].as_array_mut().unwrap();
        for next_epoch in 1..=epoch {
            history.push(json!({
                "epoch": next_epoch,
                "resource_events": [],
                "qualification_updates": [],
                "inspection_capacity_events": []
            }));
        }
        frame["decision_epoch"] = Value::from(epoch);
        frame
    }

    fn frame_epoch_one() -> Value {
        frame_for_epoch(1)
    }

    fn frame_epoch_two() -> Value {
        frame_for_epoch(2)
    }

    fn valid_snapshot_for_epoch(frame: &Value, epoch: u64, operation_two_completed: bool) -> Value {
        let frame_digest = digest_value(frame).unwrap();
        let history = frame["observable_history"].as_array().unwrap().clone();
        let history_digest = digest_value(&Value::Array(history)).unwrap();
        let mut applied_transition_ids = vec!["START-OP2-E0", "TICK-OP2-E0"];
        if epoch > 1 {
            applied_transition_ids.push("TICK-OP2-E1");
        }
        let operation_two = if operation_two_completed {
            json!({
                "operation_id": "OP-2",
                "status": "COMPLETED",
                "remaining_processing_ticks": 0,
                "committed_resource_id": null,
                "quality_disposition": null
            })
        } else {
            json!({
                "operation_id": "OP-2",
                "status": "IN_PROCESS",
                "remaining_processing_ticks": 1,
                "committed_resource_id": "RESOURCE-A",
                "quality_disposition": null
            })
        };
        let operation_three = json!({
            "operation_id": "OP-3",
            "status": "WAITING",
            "remaining_processing_ticks": 0,
            "committed_resource_id": null,
            "quality_disposition": null
        });
        let resource = json!({
            "resource_id": "RESOURCE-A",
            "current_setup_family": if operation_two_completed { "SETUP-B" } else { "SETUP-B" },
            "active_operation_id": if operation_two_completed { Value::Null } else { Value::from("OP-2") },
            "availability_status": "AVAILABLE",
            "capacity_units_total": 1,
            "capacity_units_used": if operation_two_completed { 0 } else { 1 },
            "capacity_units_remaining": if operation_two_completed { 1 } else { 0 }
        });
        let mut snapshot = json!({
            "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-operational-state-snapshot",
            "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
            "case_id": "CASE-01",
            "decision_epoch": epoch,
            "previous_snapshot_digest": "a".repeat(64),
            "decision_frame_digest": frame_digest,
            "visible_history_digest": history_digest,
            "transition_version": "mfg-policy-v02-state-replay/0.1.0",
            "operations": [
                {
                    "operation_id": "OP-1",
                    "status": "COMPLETED",
                    "remaining_processing_ticks": 0,
                    "committed_resource_id": null,
                    "quality_disposition": null
                },
                operation_two,
                operation_three
            ],
            "resources": [resource],
            "inspection_station": {
                "station_id": "INSPECT-1",
                "capacity_units_total": 1,
                "capacity_units_used": 0,
                "capacity_units_remaining": 1
            },
            "event_cut": {
                "decision_epoch": epoch,
                "visible_source_event_ids": ["Q-CUT", "Q-FORM"]
            },
            "applied_transition_ids": applied_transition_ids,
            "snapshot_digest": "",
            "claim_ceiling": "Synthetic replay-derived operational state only; not physical truth, source authentication, production authorization or whole-factory feasibility."
        });
        let mut body = snapshot.clone();
        body.as_object_mut().unwrap().remove("snapshot_digest");
        snapshot["snapshot_digest"] = Value::from(digest_value(&body).unwrap());
        snapshot
    }

    fn sample_proposal_for(frame: &Value) -> Value {
        json!({
            "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-action-proposal",
            "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
            "case_id": "CASE-01",
            "decision_epoch": frame["decision_epoch"],
            "decision_frame_digest": digest_value(frame).unwrap(),
            "operation_id": "OP-3",
            "resource_id": "RESOURCE-A",
            "action_kind": "START",
            "inspection_method_id": "DIMENSIONAL"
        })
    }

    #[test]
    fn eligible_qualified_precedent_free_action_is_admitted_with_derived_durations() {
        let receipt = {
            let frame = sample_frame();
            evaluate(&frame, &sample_proposal_for(&frame)).unwrap()
        };
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
        let receipt = evaluate(&frame, &sample_proposal_for(&frame)).unwrap();
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
        let receipt = evaluate(&frame, &sample_proposal_for(&frame)).unwrap();
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
        let receipt = evaluate(&frame, &sample_proposal_for(&frame)).unwrap();
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
        let receipt = evaluate(&frame, &sample_proposal_for(&frame)).unwrap();
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
        let mut proposal = sample_proposal_for(&frame);
        proposal["decision_epoch"] = Value::from(1);
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"trusted_operational_state_snapshot_required".to_string())
        );
    }

    #[test]
    fn uncompleted_predecessor_holds_a_dependent_start() {
        let mut frame = sample_frame();
        frame["static_case"]["jobs"][0]["operations"][0]["initial_status"] = Value::from("WAITING");
        let mut proposal = sample_proposal_for(&frame);
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
        let receipt = evaluate(&frame, &sample_proposal_for(&frame)).unwrap();
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
        let mut proposal = sample_proposal_for(&frame);
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
        let receipt = evaluate(&frame, &sample_proposal_for(&frame)).unwrap();
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
        let receipt = evaluate(&frame, &sample_proposal_for(&frame)).unwrap();
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
            evaluate(&frame, &sample_proposal_for(&frame)).unwrap_err(),
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
            evaluate(&frame, &sample_proposal_for(&frame)).unwrap_err(),
            "observable_history_must_end_at_decision_epoch"
        );
    }

    #[test]
    fn later_epoch_without_snapshot_remains_on_hold() {
        let frame = frame_epoch_one();
        let mut proposal = sample_proposal_for(&frame);
        proposal["operation_id"] = Value::from("OP-3");
        proposal["decision_epoch"] = Value::from(1);
        proposal["inspection_method_id"] = Value::Null;
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .contains(&"trusted_operational_state_snapshot_required".to_string())
        );
    }

    #[test]
    fn valid_snapshot_allows_continuation_on_live_resource_commitment() {
        let frame = frame_epoch_one();
        let snapshot = valid_snapshot_for_epoch(&frame, 1, false);
        let mut proposal = sample_proposal_for(&frame);
        proposal["operation_id"] = Value::from("OP-2");
        proposal["action_kind"] = Value::from("CONTINUE");
        proposal["decision_epoch"] = Value::from(1);
        proposal["inspection_method_id"] = Value::Null;
        let receipt = evaluate_with_snapshot(&frame, &proposal, Some(&snapshot)).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Admissible);
        assert_eq!(receipt.processing_duration_ticks, Some(1));
        assert_eq!(receipt.setup_duration_ticks, Some(0));
    }

    #[test]
    fn valid_snapshot_allows_new_start_after_live_lane_is_released() {
        let frame = frame_epoch_two();
        let snapshot = valid_snapshot_for_epoch(&frame, 2, true);
        let mut proposal = sample_proposal_for(&frame);
        proposal["operation_id"] = Value::from("OP-3");
        proposal["action_kind"] = Value::from("START");
        proposal["decision_epoch"] = Value::from(2);
        let receipt = evaluate_with_snapshot(&frame, &proposal, Some(&snapshot)).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Admissible);
        assert_eq!(receipt.processing_duration_ticks, Some(2));
        assert_eq!(receipt.setup_duration_ticks, Some(0));
    }

    #[test]
    fn tampered_snapshot_digest_holds_later_epoch_action() {
        let frame = frame_epoch_one();
        let mut snapshot = valid_snapshot_for_epoch(&frame, 1, false);
        snapshot["snapshot_digest"] = Value::from("0".repeat(64));
        let mut proposal = sample_proposal_for(&frame);
        proposal["operation_id"] = Value::from("OP-2");
        proposal["action_kind"] = Value::from("CONTINUE");
        proposal["decision_epoch"] = Value::from(1);
        proposal["inspection_method_id"] = Value::Null;
        let receipt = evaluate_with_snapshot(&frame, &proposal, Some(&snapshot)).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .iter()
                .any(|reason| reason.starts_with("snapshot_invalid:"))
        );
    }

    #[test]
    fn snapshot_cannot_change_operation_resource_commitment() {
        let frame = frame_epoch_one();
        let mut snapshot = valid_snapshot_for_epoch(&frame, 1, false);
        snapshot["resources"][0]["active_operation_id"] = Value::from("OP-3");
        let mut body = snapshot.clone();
        body.as_object_mut().unwrap().remove("snapshot_digest");
        snapshot["snapshot_digest"] = Value::from(digest_value(&body).unwrap());
        let mut proposal = sample_proposal_for(&frame);
        proposal["operation_id"] = Value::from("OP-2");
        proposal["action_kind"] = Value::from("CONTINUE");
        proposal["decision_epoch"] = Value::from(1);
        proposal["inspection_method_id"] = Value::Null;
        let receipt = evaluate_with_snapshot(&frame, &proposal, Some(&snapshot)).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Hold);
        assert!(
            receipt
                .reason_codes
                .iter()
                .any(|reason| reason.starts_with("snapshot_invalid:"))
        );
    }

    #[test]
    fn proposal_from_another_frame_digest_is_rejected() {
        let frame = sample_frame();
        let mut proposal = sample_proposal_for(&frame);
        proposal["decision_frame_digest"] = Value::from("0".repeat(64));
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Rejected);
        assert_eq!(receipt.reason_codes, vec!["proposal_frame_digest_mismatch"]);
        assert_ne!(
            receipt.decision_frame_digest,
            proposal["decision_frame_digest"]
        );
    }

    #[test]
    fn proposal_for_another_case_or_epoch_is_rejected() {
        let frame = sample_frame();
        let mut proposal = sample_proposal_for(&frame);
        proposal["case_id"] = Value::from("OTHER-CASE");
        let receipt = evaluate(&frame, &proposal).unwrap();
        assert_eq!(receipt.status, AdmissionStatus::Rejected);
        assert_eq!(
            receipt.reason_codes,
            vec!["proposal_frame_identity_mismatch"]
        );
    }
}
