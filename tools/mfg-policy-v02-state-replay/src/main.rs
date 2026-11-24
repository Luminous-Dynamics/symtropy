use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::process::ExitCode;

const LEDGER_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_execution_ledger.schema.json");
const FRAME_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_decision_frame.schema.json");
const PROPOSAL_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_action_proposal.schema.json");
const ADMISSION_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_policy_v02_action_admission_receipt.schema.json"
);
const SNAPSHOT_SCHEMA_JSON: &str = include_str!(
    "../../../schemas/manufacturing/mfg_policy_v02_operational_state_snapshot.schema.json"
);
const REPLAY_RECEIPT_SCHEMA_JSON: &str =
    include_str!("../../../schemas/manufacturing/mfg_policy_v02_replay_receipt.schema.json");

const SNAPSHOT_SCHEMA_ID: &str =
    "https://luminousdynamics.io/schemas/mfg-policy-v0.2-operational-state-snapshot";
const REPLAY_SCHEMA_ID: &str = "https://luminousdynamics.io/schemas/mfg-policy-v0.2-replay-receipt";
const SNAPSHOT_CLAIM_CEILING: &str = "Synthetic replay-derived operational state only; not physical truth, source authentication, production authorization or whole-factory feasibility.";
const REPLAY_CLAIM_CEILING: &str = "Synthetic event-sourced schedule-state replay; no physical execution, source authentication, product acceptance, ISO conformance or SOTA superiority.";
const ADMISSION_CLAIM_CEILING: &str = "Synthetic single-action admissibility only; this receipt is not schedule feasibility, source authentication, machine authority, or execution authorization.";
const TRANSITION_VERSION: &str = "mfg-policy-v02-state-replay/0.1.0";

#[derive(Debug, Clone, Serialize)]
struct OperationState {
    operation_id: String,
    status: String,
    remaining_processing_ticks: u64,
    committed_resource_id: Option<String>,
    quality_disposition: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
struct ResourceState {
    resource_id: String,
    current_setup_family: String,
    active_operation_id: Option<String>,
    availability_status: String,
    capacity_units_total: u64,
    capacity_units_used: u64,
    capacity_units_remaining: u64,
}

#[derive(Debug, Clone, Serialize)]
struct InspectionStationState {
    station_id: String,
    capacity_units_total: u64,
    capacity_units_used: u64,
    capacity_units_remaining: u64,
}

#[derive(Debug, Clone, Serialize)]
struct EventCut {
    decision_epoch: u64,
    visible_source_event_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
struct Snapshot {
    schema_id: &'static str,
    benchmark_id: String,
    case_id: String,
    decision_epoch: u64,
    previous_snapshot_digest: Option<String>,
    decision_frame_digest: String,
    visible_history_digest: String,
    transition_version: &'static str,
    operations: Vec<OperationState>,
    resources: Vec<ResourceState>,
    inspection_station: InspectionStationState,
    event_cut: EventCut,
    applied_transition_ids: Vec<String>,
    snapshot_digest: String,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone, Serialize)]
struct Violation {
    decision_epoch: Option<u64>,
    transition_id: Option<String>,
    code: String,
}

#[derive(Debug, Clone, Serialize)]
struct ReplayReceipt {
    schema_id: &'static str,
    benchmark_id: String,
    case_id: String,
    status: String,
    ledger_digest: String,
    snapshots: Vec<Snapshot>,
    violation_ledger: Vec<Violation>,
    final_snapshot_digest: Option<String>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Clone)]
struct RuntimeResource {
    resource_id: String,
    current_setup_family: String,
    active_operation_id: Option<String>,
    availability_status: String,
    capacity_units_total: u64,
    capacity_units_used: u64,
}

#[derive(Debug, Clone)]
struct RuntimeInspection {
    station_id: String,
    capacity_units_total: u64,
    capacity_units_used: u64,
}

#[derive(Debug, Clone)]
struct RuntimeState {
    operations: BTreeMap<String, OperationState>,
    resources: BTreeMap<String, RuntimeResource>,
    inspection_station: RuntimeInspection,
    applied_transition_ids: Vec<String>,
    seen_transition_ids: BTreeSet<String>,
}

struct FrameEvidence {
    qualifications: BTreeMap<(String, String), String>,
    resource_availability: BTreeMap<String, String>,
    inspection_capacity_override: Option<u64>,
    source_event_ids: Vec<String>,
}

fn violation(
    code: impl Into<String>,
    epoch: Option<u64>,
    transition_id: Option<&str>,
) -> Violation {
    Violation {
        decision_epoch: epoch,
        transition_id: transition_id.map(str::to_string),
        code: code.into(),
    }
}

fn validate_schema(
    instance: &Value,
    schema_source: &str,
    label: &str,
    epoch: Option<u64>,
    transition_id: Option<&str>,
) -> Result<(), Violation> {
    let schema: Value = serde_json::from_str(schema_source)
        .map_err(|_| violation(format!("{label}_schema_parse"), epoch, transition_id))?;
    let validator = jsonschema::draft202012::options()
        .offline()
        .build(&schema)
        .map_err(|_| violation(format!("{label}_schema_compile"), epoch, transition_id))?;
    validator
        .validate(instance)
        .map_err(|_| violation(format!("{label}_schema_validation"), epoch, transition_id))
}

fn required_string<'a>(
    value: &'a Value,
    key: &str,
    context: &str,
    epoch: Option<u64>,
    transition_id: Option<&str>,
) -> Result<&'a str, Violation> {
    value
        .get(key)
        .and_then(Value::as_str)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| {
            violation(
                format!("{context}_{key}_missing_or_invalid"),
                epoch,
                transition_id,
            )
        })
}

fn required_array<'a>(
    value: &'a Value,
    key: &str,
    context: &str,
    epoch: Option<u64>,
    transition_id: Option<&str>,
) -> Result<&'a Vec<Value>, Violation> {
    value.get(key).and_then(Value::as_array).ok_or_else(|| {
        violation(
            format!("{context}_{key}_missing_or_invalid"),
            epoch,
            transition_id,
        )
    })
}

fn required_u64(
    value: &Value,
    key: &str,
    context: &str,
    epoch: Option<u64>,
    transition_id: Option<&str>,
) -> Result<u64, Violation> {
    value.get(key).and_then(Value::as_u64).ok_or_else(|| {
        violation(
            format!("{context}_{key}_missing_or_invalid"),
            epoch,
            transition_id,
        )
    })
}

fn digest_value(
    value: &Value,
    epoch: Option<u64>,
    transition_id: Option<&str>,
) -> Result<String, Violation> {
    let bytes = serde_json::to_vec(value)
        .map_err(|_| violation("canonical_json_serialization_failed", epoch, transition_id))?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn static_case<'a>(frame: &'a Value, epoch: u64) -> Result<&'a Value, Violation> {
    frame
        .get("static_case")
        .ok_or_else(|| violation("decision_frame_static_case_missing", Some(epoch), None))
}

fn index_jobs_and_operations(
    case: &Value,
    epoch: u64,
) -> Result<(BTreeMap<String, Value>, BTreeMap<String, String>), Violation> {
    let jobs = required_array(case, "jobs", "static_case", Some(epoch), None)?;
    let mut job_ids = BTreeSet::new();
    let mut operations = BTreeMap::new();
    let mut owner_by_operation = BTreeMap::new();

    for job in jobs {
        let job_id = required_string(job, "job_id", "job", Some(epoch), None)?;
        if !job_ids.insert(job_id.to_string()) {
            return Err(violation("job_id_duplicate", Some(epoch), None));
        }
        for operation in required_array(job, "operations", "job", Some(epoch), None)? {
            let operation_id =
                required_string(operation, "operation_id", "operation", Some(epoch), None)?;
            if operations
                .insert(operation_id.to_string(), operation.clone())
                .is_some()
            {
                return Err(violation("operation_id_duplicate", Some(epoch), None));
            }
            owner_by_operation.insert(operation_id.to_string(), job_id.to_string());
        }
    }

    for operation in operations.values() {
        for predecessor in required_array(
            operation,
            "predecessor_operation_ids",
            "operation",
            Some(epoch),
            None,
        )? {
            let predecessor_id = predecessor
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| violation("predecessor_operation_id_invalid", Some(epoch), None))?;
            if !operations.contains_key(predecessor_id) {
                return Err(violation(
                    "predecessor_operation_unknown",
                    Some(epoch),
                    None,
                ));
            }
        }
    }

    Ok((operations, owner_by_operation))
}

fn index_resources(case: &Value, epoch: u64) -> Result<BTreeMap<String, Value>, Violation> {
    let resources = required_array(case, "resources", "static_case", Some(epoch), None)?;
    let mut index = BTreeMap::new();

    for resource in resources {
        let resource_id = required_string(resource, "resource_id", "resource", Some(epoch), None)?;
        let capacity = required_array(
            resource,
            "capacity_units_by_epoch",
            "resource",
            Some(epoch),
            None,
        )?;
        if capacity.len() != 4 || capacity.iter().any(|item| item.as_u64().is_none()) {
            return Err(violation(
                "resource_capacity_calendar_invalid",
                Some(epoch),
                None,
            ));
        }
        if capacity.iter().any(|item| item.as_u64().unwrap_or(0) > 1) {
            return Err(violation(
                "resource_capacity_above_single_lane_not_supported",
                Some(epoch),
                None,
            ));
        }
        let capabilities = required_array(
            resource,
            "capability_families",
            "resource",
            Some(epoch),
            None,
        )?;
        let mut capability_ids = BTreeSet::new();
        for capability in capabilities {
            let capability = capability
                .as_str()
                .filter(|text| !text.trim().is_empty())
                .ok_or_else(|| violation("resource_capability_invalid", Some(epoch), None))?;
            if !capability_ids.insert(capability.to_string()) {
                return Err(violation(
                    "resource_capability_duplicate",
                    Some(epoch),
                    None,
                ));
            }
        }
        if index
            .insert(resource_id.to_string(), resource.clone())
            .is_some()
        {
            return Err(violation("resource_id_duplicate", Some(epoch), None));
        }
    }

    Ok(index)
}

fn validate_frame_evidence(
    frame: &Value,
    case: &Value,
    epoch: u64,
    resources: &BTreeMap<String, Value>,
) -> Result<FrameEvidence, Violation> {
    let history = required_array(
        frame,
        "observable_history",
        "decision_frame",
        Some(epoch),
        None,
    )?;
    if history.len() as u64 != epoch + 1 {
        return Err(violation(
            "observable_history_must_end_at_decision_epoch",
            Some(epoch),
            None,
        ));
    }

    let station_id = required_string(
        case.get("inspection_station")
            .ok_or_else(|| violation("inspection_station_missing", Some(epoch), None))?,
        "station_id",
        "inspection_station",
        Some(epoch),
        None,
    )?;

    let mut qualifications = BTreeMap::new();
    let mut resource_availability = BTreeMap::new();
    let mut event_ids = BTreeSet::new();
    for (resource_id, resource) in resources {
        let capabilities = required_array(
            resource,
            "capability_families",
            "resource",
            Some(epoch),
            None,
        )?;
        let capability_set: BTreeSet<&str> =
            capabilities.iter().filter_map(Value::as_str).collect();

        for qualification in required_array(
            resource,
            "initial_qualifications",
            "resource",
            Some(epoch),
            None,
        )? {
            let family = required_string(
                qualification,
                "process_family",
                "qualification",
                Some(epoch),
                None,
            )?;
            let disposition = required_string(
                qualification,
                "disposition",
                "qualification",
                Some(epoch),
                None,
            )?;
            let receipt = required_string(
                qualification,
                "evidence_receipt_id",
                "qualification",
                Some(epoch),
                None,
            )?;
            if !capability_set.contains(family)
                || !["ADMISSIBLE", "UNKNOWN", "BLOCKED"].contains(&disposition)
                || !event_ids.insert(receipt.to_string())
                || qualifications
                    .insert(
                        (resource_id.to_string(), family.to_string()),
                        disposition.to_string(),
                    )
                    .is_some()
            {
                return Err(violation(
                    "initial_qualification_identity_or_scope_invalid",
                    Some(epoch),
                    None,
                ));
            }
        }
        resource_availability.insert(resource_id.to_string(), "AVAILABLE".to_string());
    }

    let mut inspection_capacity_override = None;
    for (expected_epoch, observation) in history.iter().enumerate() {
        if required_u64(observation, "epoch", "observation", Some(epoch), None)?
            != expected_epoch as u64
        {
            return Err(violation(
                "observable_history_epoch_order_invalid",
                Some(epoch),
                None,
            ));
        }

        for event in required_array(
            observation,
            "resource_events",
            "observation",
            Some(epoch),
            None,
        )? {
            let event_id = required_string(
                event,
                "source_event_id",
                "resource_event",
                Some(epoch),
                None,
            )?;
            let resource_id =
                required_string(event, "resource_id", "resource_event", Some(epoch), None)?;
            let kind = required_string(event, "kind", "resource_event", Some(epoch), None)?;
            let occurred = required_u64(
                event,
                "occurred_at_epoch",
                "resource_event",
                Some(epoch),
                None,
            )?;
            let observed = required_u64(
                event,
                "observed_at_epoch",
                "resource_event",
                Some(epoch),
                None,
            )?;
            if !event_ids.insert(event_id.to_string())
                || !resources.contains_key(resource_id)
                || observed != expected_epoch as u64
                || occurred > observed
            {
                return Err(violation(
                    "resource_event_identity_scope_or_chronology_invalid",
                    Some(epoch),
                    None,
                ));
            }
            match kind {
                "FAILED" => {
                    let lower = event
                        .get("recovery_forecast_min_epochs")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| {
                            violation("failure_recovery_forecast_missing", Some(epoch), None)
                        })?;
                    let upper = event
                        .get("recovery_forecast_max_epochs")
                        .and_then(Value::as_u64)
                        .ok_or_else(|| {
                            violation("failure_recovery_forecast_missing", Some(epoch), None)
                        })?;
                    if lower > upper {
                        return Err(violation(
                            "failure_recovery_forecast_invalid",
                            Some(epoch),
                            None,
                        ));
                    }
                    resource_availability.insert(resource_id.to_string(), "FAILED".to_string());
                }
                "RESTORED" => {
                    if !event
                        .get("recovery_forecast_min_epochs")
                        .is_some_and(Value::is_null)
                        || !event
                            .get("recovery_forecast_max_epochs")
                            .is_some_and(Value::is_null)
                    {
                        return Err(violation(
                            "restoration_must_not_carry_recovery_forecast",
                            Some(epoch),
                            None,
                        ));
                    }
                    resource_availability.insert(resource_id.to_string(), "AVAILABLE".to_string());
                }
                _ => return Err(violation("resource_event_kind_invalid", Some(epoch), None)),
            }
        }

        for update in required_array(
            observation,
            "qualification_updates",
            "observation",
            Some(epoch),
            None,
        )? {
            let receipt = required_string(
                update,
                "evidence_receipt_id",
                "qualification_update",
                Some(epoch),
                None,
            )?;
            let resource_id = required_string(
                update,
                "resource_id",
                "qualification_update",
                Some(epoch),
                None,
            )?;
            let family = required_string(
                update,
                "process_family",
                "qualification_update",
                Some(epoch),
                None,
            )?;
            let previous = required_string(
                update,
                "previous_disposition",
                "qualification_update",
                Some(epoch),
                None,
            )?;
            let next = required_string(
                update,
                "new_disposition",
                "qualification_update",
                Some(epoch),
                None,
            )?;
            let occurred = required_u64(
                update,
                "occurred_at_epoch",
                "qualification_update",
                Some(epoch),
                None,
            )?;
            let observed = required_u64(
                update,
                "observed_at_epoch",
                "qualification_update",
                Some(epoch),
                None,
            )?;
            let resource = resources.get(resource_id).ok_or_else(|| {
                violation("qualification_update_resource_unknown", Some(epoch), None)
            })?;
            let capabilities = required_array(
                resource,
                "capability_families",
                "resource",
                Some(epoch),
                None,
            )?;
            if !event_ids.insert(receipt.to_string())
                || observed != expected_epoch as u64
                || occurred > observed
                || !["ADMISSIBLE", "UNKNOWN", "BLOCKED"].contains(&previous)
                || !["ADMISSIBLE", "UNKNOWN", "BLOCKED"].contains(&next)
                || previous == next
                || !capabilities
                    .iter()
                    .any(|value| value.as_str() == Some(family))
            {
                return Err(violation(
                    "qualification_update_identity_scope_or_chronology_invalid",
                    Some(epoch),
                    None,
                ));
            }
            let state = qualifications
                .get_mut(&(resource_id.to_string(), family.to_string()))
                .ok_or_else(|| {
                    violation(
                        "qualification_update_prior_state_missing",
                        Some(epoch),
                        None,
                    )
                })?;
            if state != previous {
                return Err(violation(
                    "qualification_update_prior_state_mismatch",
                    Some(epoch),
                    None,
                ));
            }
            *state = next.to_string();
        }

        for event in required_array(
            observation,
            "inspection_capacity_events",
            "observation",
            Some(epoch),
            None,
        )? {
            let event_id = required_string(
                event,
                "source_event_id",
                "inspection_event",
                Some(epoch),
                None,
            )?;
            let actual_station =
                required_string(event, "station_id", "inspection_event", Some(epoch), None)?;
            let occurred = required_u64(
                event,
                "occurred_at_epoch",
                "inspection_event",
                Some(epoch),
                None,
            )?;
            let observed = required_u64(
                event,
                "observed_at_epoch",
                "inspection_event",
                Some(epoch),
                None,
            )?;
            let capacity = required_u64(
                event,
                "available_capacity_units",
                "inspection_event",
                Some(epoch),
                None,
            )?;
            if !event_ids.insert(event_id.to_string())
                || actual_station != station_id
                || observed != expected_epoch as u64
                || occurred > observed
                || inspection_capacity_override.is_some() && expected_epoch as u64 == epoch
            {
                return Err(violation(
                    "inspection_event_identity_scope_or_chronology_invalid",
                    Some(epoch),
                    None,
                ));
            }
            if expected_epoch as u64 == epoch {
                inspection_capacity_override = Some(capacity);
            }
        }
    }

    Ok(FrameEvidence {
        qualifications,
        resource_availability,
        inspection_capacity_override,
        source_event_ids: event_ids.into_iter().collect(),
    })
}

fn initial_state(
    case: &Value,
    epoch: u64,
    resources: &BTreeMap<String, Value>,
) -> Result<RuntimeState, Violation> {
    let (operations, _) = index_jobs_and_operations(case, epoch)?;
    let mut operation_states = BTreeMap::new();
    let mut resource_states = BTreeMap::new();

    for (resource_id, resource) in resources {
        resource_states.insert(
            resource_id.to_string(),
            RuntimeResource {
                resource_id: resource_id.to_string(),
                current_setup_family: required_string(
                    resource,
                    "initial_setup_family",
                    "resource",
                    Some(epoch),
                    None,
                )?
                .to_string(),
                active_operation_id: None,
                availability_status: "AVAILABLE".to_string(),
                capacity_units_total: 0,
                capacity_units_used: 0,
            },
        );
    }

    for (operation_id, operation) in operations {
        let initial_status =
            required_string(&operation, "initial_status", "operation", Some(epoch), None)?;
        let remaining = required_u64(
            &operation,
            "remaining_processing_ticks",
            "operation",
            Some(epoch),
            None,
        )?;
        let committed = operation
            .get("committed_resource_id")
            .and_then(Value::as_str)
            .map(str::to_string);
        let status = match initial_status {
            "WAITING" if remaining == 0 && committed.is_none() => "WAITING",
            "IN_PROCESS" if remaining > 0 && committed.is_some() => "IN_PROCESS",
            "COMPLETED" if remaining == 0 && committed.is_none() => "COMPLETED",
            _ => {
                return Err(violation(
                    "initial_operation_state_inconsistent",
                    Some(epoch),
                    None,
                ));
            }
        };

        if let Some(resource_id) = committed.as_deref() {
            let resource_state = resource_states.get_mut(resource_id).ok_or_else(|| {
                violation(
                    "initial_operation_committed_resource_unknown",
                    Some(epoch),
                    None,
                )
            })?;
            if resource_state
                .active_operation_id
                .replace(operation_id.clone())
                .is_some()
            {
                return Err(violation(
                    "initial_resource_has_multiple_active_operations",
                    Some(epoch),
                    None,
                ));
            }
            let eligible = required_array(
                &operation,
                "eligible_resource_ids",
                "operation",
                Some(epoch),
                None,
            )?;
            if !eligible
                .iter()
                .any(|value| value.as_str() == Some(resource_id))
            {
                return Err(violation(
                    "initial_committed_resource_not_eligible",
                    Some(epoch),
                    None,
                ));
            }
        }

        operation_states.insert(
            operation_id.clone(),
            OperationState {
                operation_id,
                status: status.to_string(),
                remaining_processing_ticks: remaining,
                committed_resource_id: committed,
                quality_disposition: None,
            },
        );
    }

    let station = case
        .get("inspection_station")
        .ok_or_else(|| violation("inspection_station_missing", Some(epoch), None))?;
    let station_id = required_string(
        station,
        "station_id",
        "inspection_station",
        Some(epoch),
        None,
    )?;

    Ok(RuntimeState {
        operations: operation_states,
        resources: resource_states,
        inspection_station: RuntimeInspection {
            station_id: station_id.to_string(),
            capacity_units_total: 0,
            capacity_units_used: 0,
        },
        applied_transition_ids: Vec::new(),
        seen_transition_ids: BTreeSet::new(),
    })
}

fn refresh_epoch_state(
    state: &mut RuntimeState,
    case: &Value,
    evidence: &FrameEvidence,
    epoch: u64,
) -> Result<(), Violation> {
    for (resource_id, runtime) in &mut state.resources {
        let static_resource = case
            .get("resources")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("resource_id").and_then(Value::as_str) == Some(resource_id)
                })
            })
            .ok_or_else(|| violation("resource_static_definition_missing", Some(epoch), None))?;
        let calendar = required_array(
            static_resource,
            "capacity_units_by_epoch",
            "resource",
            Some(epoch),
            None,
        )?;
        let total = calendar
            .get(epoch as usize)
            .and_then(Value::as_u64)
            .ok_or_else(|| violation("resource_capacity_for_epoch_missing", Some(epoch), None))?;
        runtime.capacity_units_total = total;
        runtime.availability_status = evidence
            .resource_availability
            .get(resource_id)
            .cloned()
            .unwrap_or_else(|| "AVAILABLE".to_string());
        runtime.capacity_units_used = if runtime.active_operation_id.is_some()
            && total > 0
            && runtime.availability_status == "AVAILABLE"
        {
            1
        } else {
            0
        };
    }

    let station = case
        .get("inspection_station")
        .ok_or_else(|| violation("inspection_station_missing", Some(epoch), None))?;
    let station_calendar = required_array(
        station,
        "capacity_units_by_epoch",
        "inspection_station",
        Some(epoch),
        None,
    )?;
    let base_capacity = station_calendar
        .get(epoch as usize)
        .and_then(Value::as_u64)
        .ok_or_else(|| violation("inspection_capacity_for_epoch_missing", Some(epoch), None))?;
    if let Some(override_capacity) = evidence.inspection_capacity_override {
        if override_capacity > base_capacity {
            return Err(violation(
                "inspection_capacity_event_exceeds_static_bound",
                Some(epoch),
                None,
            ));
        }
        state.inspection_station.capacity_units_total = override_capacity;
    } else {
        state.inspection_station.capacity_units_total = base_capacity;
    }
    state.inspection_station.capacity_units_used = 0;
    Ok(())
}

fn current_setup_duration(
    case: &Value,
    resource_id: &str,
    from_setup: &str,
    to_setup: &str,
    epoch: u64,
    transition_id: &str,
) -> Result<u64, Violation> {
    if from_setup == to_setup {
        return Ok(0);
    }
    let transitions = required_array(
        case,
        "setup_transitions",
        "static_case",
        Some(epoch),
        Some(transition_id),
    )?;
    let matches: Vec<&Value> = transitions
        .iter()
        .filter(|transition| {
            transition.get("resource_id").and_then(Value::as_str) == Some(resource_id)
                && transition.get("from_setup_family").and_then(Value::as_str) == Some(from_setup)
                && transition.get("to_setup_family").and_then(Value::as_str) == Some(to_setup)
        })
        .collect();
    match matches.as_slice() {
        [transition] => required_u64(
            transition,
            "duration_ticks",
            "setup_transition",
            Some(epoch),
            Some(transition_id),
        ),
        [] => Err(violation(
            "setup_transition_missing",
            Some(epoch),
            Some(transition_id),
        )),
        _ => Err(violation(
            "setup_transition_duplicate",
            Some(epoch),
            Some(transition_id),
        )),
    }
}

fn current_frame_qualification<'a>(
    evidence: &'a FrameEvidence,
    resource_id: &str,
    process_family: &str,
) -> Option<&'a str> {
    evidence
        .qualifications
        .get(&(resource_id.to_string(), process_family.to_string()))
        .map(String::as_str)
}

fn verify_receipt_matches(
    receipt: &Value,
    proposal: &Value,
    frame: &Value,
    processing_duration: u64,
    setup_duration: u64,
    inspection_method: Option<&str>,
    epoch: u64,
    transition_id: &str,
) -> Result<(), Violation> {
    validate_schema(
        receipt,
        ADMISSION_SCHEMA_JSON,
        "admission_receipt",
        Some(epoch),
        Some(transition_id),
    )?;
    if required_string(
        receipt,
        "status",
        "admission_receipt",
        Some(epoch),
        Some(transition_id),
    )? != "ADMISSIBLE"
    {
        return Err(violation(
            "admission_receipt_not_admissible",
            Some(epoch),
            Some(transition_id),
        ));
    }
    if required_string(
        receipt,
        "benchmark_id",
        "admission_receipt",
        Some(epoch),
        Some(transition_id),
    )? != required_string(
        frame,
        "benchmark_id",
        "decision_frame",
        Some(epoch),
        Some(transition_id),
    )? || required_string(
        receipt,
        "case_id",
        "admission_receipt",
        Some(epoch),
        Some(transition_id),
    )? != required_string(
        frame,
        "case_id",
        "decision_frame",
        Some(epoch),
        Some(transition_id),
    )? || required_u64(
        receipt,
        "decision_epoch",
        "admission_receipt",
        Some(epoch),
        Some(transition_id),
    )? != epoch
        || required_string(
            receipt,
            "operation_id",
            "admission_receipt",
            Some(epoch),
            Some(transition_id),
        )? != required_string(
            proposal,
            "operation_id",
            "proposal",
            Some(epoch),
            Some(transition_id),
        )?
        || required_string(
            receipt,
            "resource_id",
            "admission_receipt",
            Some(epoch),
            Some(transition_id),
        )? != required_string(
            proposal,
            "resource_id",
            "proposal",
            Some(epoch),
            Some(transition_id),
        )?
    {
        return Err(violation(
            "admission_receipt_identity_mismatch",
            Some(epoch),
            Some(transition_id),
        ));
    }
    let reason_codes = required_array(
        receipt,
        "reason_codes",
        "admission_receipt",
        Some(epoch),
        Some(transition_id),
    )?;
    if !reason_codes.is_empty()
        || required_u64(
            receipt,
            "processing_duration_ticks",
            "admission_receipt",
            Some(epoch),
            Some(transition_id),
        )? != processing_duration
        || required_u64(
            receipt,
            "setup_duration_ticks",
            "admission_receipt",
            Some(epoch),
            Some(transition_id),
        )? != setup_duration
        || receipt
            .get("qualification_disposition")
            .and_then(Value::as_str)
            != Some("ADMISSIBLE")
        || receipt.get("inspection_method_id").and_then(Value::as_str) != inspection_method
    {
        return Err(violation(
            "admission_receipt_derived_fields_mismatch",
            Some(epoch),
            Some(transition_id),
        ));
    }
    if required_u64(
        receipt,
        "capacity_units_required",
        "admission_receipt",
        Some(epoch),
        Some(transition_id),
    )? != 1
        || required_string(
            receipt,
            "claim_ceiling",
            "admission_receipt",
            Some(epoch),
            Some(transition_id),
        )? != ADMISSION_CLAIM_CEILING
    {
        return Err(violation(
            "admission_receipt_contract_mismatch",
            Some(epoch),
            Some(transition_id),
        ));
    }
    Ok(())
}

fn apply_start_action(
    transition: &Value,
    frame: &Value,
    case: &Value,
    evidence: &FrameEvidence,
    state: &mut RuntimeState,
    epoch: u64,
    transition_id: &str,
) -> Result<(), Violation> {
    let proposal = transition.get("proposal").ok_or_else(|| {
        violation(
            "start_action_proposal_missing",
            Some(epoch),
            Some(transition_id),
        )
    })?;
    let receipt = transition.get("admission_receipt").ok_or_else(|| {
        violation(
            "start_action_admission_receipt_missing",
            Some(epoch),
            Some(transition_id),
        )
    })?;
    validate_schema(
        proposal,
        PROPOSAL_SCHEMA_JSON,
        "action_proposal",
        Some(epoch),
        Some(transition_id),
    )?;

    let operation_id = required_string(
        proposal,
        "operation_id",
        "proposal",
        Some(epoch),
        Some(transition_id),
    )?;
    let resource_id = required_string(
        proposal,
        "resource_id",
        "proposal",
        Some(epoch),
        Some(transition_id),
    )?;
    if required_string(
        proposal,
        "benchmark_id",
        "proposal",
        Some(epoch),
        Some(transition_id),
    )? != required_string(
        frame,
        "benchmark_id",
        "decision_frame",
        Some(epoch),
        Some(transition_id),
    )? || required_string(
        proposal,
        "case_id",
        "proposal",
        Some(epoch),
        Some(transition_id),
    )? != required_string(
        frame,
        "case_id",
        "decision_frame",
        Some(epoch),
        Some(transition_id),
    )? || required_u64(
        proposal,
        "decision_epoch",
        "proposal",
        Some(epoch),
        Some(transition_id),
    )? != epoch
    {
        return Err(violation(
            "proposal_decision_frame_identity_mismatch",
            Some(epoch),
            Some(transition_id),
        ));
    }

    let operation = state
        .operations
        .get(operation_id)
        .ok_or_else(|| violation("operation_not_found", Some(epoch), Some(transition_id)))?
        .clone();
    let resource = state
        .resources
        .get(resource_id)
        .ok_or_else(|| violation("resource_not_found", Some(epoch), Some(transition_id)))?
        .clone();
    let static_operation = case
        .get("jobs")
        .and_then(Value::as_array)
        .and_then(|jobs| {
            jobs.iter().find_map(|job| {
                job.get("operations")
                    .and_then(Value::as_array)
                    .and_then(|operations| {
                        operations.iter().find(|item| {
                            item.get("operation_id").and_then(Value::as_str) == Some(operation_id)
                        })
                    })
            })
        })
        .ok_or_else(|| violation("static_operation_missing", Some(epoch), Some(transition_id)))?;

    let action_kind = required_string(
        proposal,
        "action_kind",
        "proposal",
        Some(epoch),
        Some(transition_id),
    )?;
    let process_family = required_string(
        static_operation,
        "process_family",
        "operation",
        Some(epoch),
        Some(transition_id),
    )?;
    let setup_family = required_string(
        static_operation,
        "setup_family",
        "operation",
        Some(epoch),
        Some(transition_id),
    )?;

    let eligible = required_array(
        static_operation,
        "eligible_resource_ids",
        "operation",
        Some(epoch),
        Some(transition_id),
    )?;
    let capabilities = required_array(
        case.get("resources")
            .and_then(Value::as_array)
            .and_then(|items| {
                items.iter().find(|item| {
                    item.get("resource_id").and_then(Value::as_str) == Some(resource_id)
                })
            })
            .ok_or_else(|| {
                violation("static_resource_missing", Some(epoch), Some(transition_id))
            })?,
        "capability_families",
        "resource",
        Some(epoch),
        Some(transition_id),
    )?;
    if !eligible
        .iter()
        .any(|item| item.as_str() == Some(resource_id))
        || !capabilities
            .iter()
            .any(|item| item.as_str() == Some(process_family))
    {
        return Err(violation(
            "resource_not_eligible_or_capable",
            Some(epoch),
            Some(transition_id),
        ));
    }
    if resource.availability_status != "AVAILABLE" {
        return Err(violation(
            "resource_unavailable_at_action_cutoff",
            Some(epoch),
            Some(transition_id),
        ));
    }
    if action_kind == "START" && resource.active_operation_id.is_some() {
        return Err(violation(
            "resource_already_has_active_operation",
            Some(epoch),
            Some(transition_id),
        ));
    }
    if resource.capacity_units_total == 0
        || (action_kind == "START"
            && resource
                .capacity_units_total
                .saturating_sub(resource.capacity_units_used)
                == 0)
    {
        return Err(violation(
            "resource_capacity_exhausted",
            Some(epoch),
            Some(transition_id),
        ));
    }

    let qualification = current_frame_qualification(evidence, resource_id, process_family);
    if qualification != Some("ADMISSIBLE") {
        return Err(violation(
            "qualification_not_admissible_at_action_cutoff",
            Some(epoch),
            Some(transition_id),
        ));
    }

    let (processing_duration, setup_duration, inspection_method) = match action_kind {
        "START" => {
            if operation.status != "WAITING" || operation.committed_resource_id.is_some() {
                return Err(violation(
                    "start_requires_uncommitted_waiting_operation",
                    Some(epoch),
                    Some(transition_id),
                ));
            }
            let predecessors = required_array(
                static_operation,
                "predecessor_operation_ids",
                "operation",
                Some(epoch),
                Some(transition_id),
            )?;
            for predecessor in predecessors {
                let predecessor_id = predecessor.as_str().ok_or_else(|| {
                    violation(
                        "predecessor_operation_id_invalid",
                        Some(epoch),
                        Some(transition_id),
                    )
                })?;
                if state
                    .operations
                    .get(predecessor_id)
                    .is_none_or(|prior| prior.status != "COMPLETED")
                {
                    return Err(violation(
                        "predecessor_not_completed_before_start",
                        Some(epoch),
                        Some(transition_id),
                    ));
                }
            }
            let processing_times = static_operation
                .get("processing_time_ticks_by_resource")
                .and_then(Value::as_object)
                .ok_or_else(|| {
                    violation(
                        "operation_processing_times_missing",
                        Some(epoch),
                        Some(transition_id),
                    )
                })?;
            let duration = processing_times
                .get(resource_id)
                .and_then(Value::as_u64)
                .filter(|duration| *duration > 0)
                .ok_or_else(|| {
                    violation(
                        "operation_processing_duration_missing",
                        Some(epoch),
                        Some(transition_id),
                    )
                })?;
            let setup = current_setup_duration(
                case,
                resource_id,
                &resource.current_setup_family,
                setup_family,
                epoch,
                transition_id,
            )?;
            let inspection_required = static_operation
                .get("inspection_required")
                .and_then(Value::as_bool)
                .ok_or_else(|| {
                    violation(
                        "operation_inspection_required_missing",
                        Some(epoch),
                        Some(transition_id),
                    )
                })?;
            let proposal_method = proposal.get("inspection_method_id").ok_or_else(|| {
                violation(
                    "proposal_inspection_method_missing",
                    Some(epoch),
                    Some(transition_id),
                )
            })?;
            let method = if inspection_required {
                let required_method = required_string(
                    static_operation,
                    "inspection_method_id",
                    "operation",
                    Some(epoch),
                    Some(transition_id),
                )?;
                if proposal_method.as_str() != Some(required_method) {
                    return Err(violation(
                        "required_inspection_method_mismatch",
                        Some(epoch),
                        Some(transition_id),
                    ));
                }
                validate_inspection_method(
                    case,
                    required_method,
                    process_family,
                    epoch,
                    transition_id,
                )?;
                Some(required_method)
            } else {
                if !proposal_method.is_null() {
                    return Err(violation(
                        "unneeded_inspection_method_proposed",
                        Some(epoch),
                        Some(transition_id),
                    ));
                }
                None
            };
            (duration, setup, method)
        }
        "CONTINUE" => {
            if operation.status != "IN_PROCESS"
                || operation.committed_resource_id.as_deref() != Some(resource_id)
                || resource.active_operation_id.as_deref() != Some(operation_id)
            {
                return Err(violation(
                    "continuation_does_not_match_live_commitment",
                    Some(epoch),
                    Some(transition_id),
                ));
            }
            if operation.remaining_processing_ticks == 0 {
                return Err(violation(
                    "continuation_has_no_remaining_work",
                    Some(epoch),
                    Some(transition_id),
                ));
            }
            let proposal_method = proposal.get("inspection_method_id").ok_or_else(|| {
                violation(
                    "proposal_inspection_method_missing",
                    Some(epoch),
                    Some(transition_id),
                )
            })?;
            let inspection_required = static_operation
                .get("inspection_required")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let method = if inspection_required {
                let required_method = required_string(
                    static_operation,
                    "inspection_method_id",
                    "operation",
                    Some(epoch),
                    Some(transition_id),
                )?;
                if proposal_method.as_str() != Some(required_method) {
                    return Err(violation(
                        "required_inspection_method_mismatch",
                        Some(epoch),
                        Some(transition_id),
                    ));
                }
                validate_inspection_method(
                    case,
                    required_method,
                    process_family,
                    epoch,
                    transition_id,
                )?;
                Some(required_method)
            } else {
                if !proposal_method.is_null() {
                    return Err(violation(
                        "unneeded_inspection_method_proposed",
                        Some(epoch),
                        Some(transition_id),
                    ));
                }
                None
            };
            (operation.remaining_processing_ticks, 0, method)
        }
        _ => {
            return Err(violation(
                "action_kind_invalid",
                Some(epoch),
                Some(transition_id),
            ));
        }
    };

    verify_receipt_matches(
        receipt,
        proposal,
        frame,
        processing_duration,
        setup_duration,
        inspection_method,
        epoch,
        transition_id,
    )?;

    if action_kind == "START" {
        let op = state
            .operations
            .get_mut(operation_id)
            .ok_or_else(|| violation("operation_not_found", Some(epoch), Some(transition_id)))?;
        op.status = "IN_PROCESS".to_string();
        op.remaining_processing_ticks = processing_duration;
        op.committed_resource_id = Some(resource_id.to_string());

        let res = state
            .resources
            .get_mut(resource_id)
            .ok_or_else(|| violation("resource_not_found", Some(epoch), Some(transition_id)))?;
        res.current_setup_family = setup_family.to_string();
        res.active_operation_id = Some(operation_id.to_string());
        res.capacity_units_used = 1;
    } else if let Some(res) = state.resources.get_mut(resource_id) {
        res.capacity_units_used = 1;
    }

    Ok(())
}

fn validate_inspection_method(
    case: &Value,
    method_id: &str,
    process_family: &str,
    epoch: u64,
    transition_id: &str,
) -> Result<(), Violation> {
    let station = case.get("inspection_station").ok_or_else(|| {
        violation(
            "inspection_station_missing",
            Some(epoch),
            Some(transition_id),
        )
    })?;
    let methods = required_array(
        station,
        "methods",
        "inspection_station",
        Some(epoch),
        Some(transition_id),
    )?;
    let matches: Vec<&Value> = methods
        .iter()
        .filter(|method| method.get("method_id").and_then(Value::as_str) == Some(method_id))
        .collect();
    let method = match matches.as_slice() {
        [method] => *method,
        [] => {
            return Err(violation(
                "inspection_method_not_declared",
                Some(epoch),
                Some(transition_id),
            ));
        }
        _ => {
            return Err(violation(
                "inspection_method_duplicate",
                Some(epoch),
                Some(transition_id),
            ));
        }
    };
    let applicable = required_array(
        method,
        "applicable_process_families",
        "inspection_method",
        Some(epoch),
        Some(transition_id),
    )?;
    if !applicable
        .iter()
        .any(|family| family.as_str() == Some(process_family))
    {
        return Err(violation(
            "inspection_method_not_applicable",
            Some(epoch),
            Some(transition_id),
        ));
    }
    Ok(())
}

fn apply_process_ticks(
    transition: &Value,
    case: &Value,
    state: &mut RuntimeState,
    epoch: u64,
    transition_id: &str,
) -> Result<(), Violation> {
    let operation_id = required_string(
        transition,
        "operation_id",
        "process_ticks",
        Some(epoch),
        Some(transition_id),
    )?;
    let resource_id = required_string(
        transition,
        "resource_id",
        "process_ticks",
        Some(epoch),
        Some(transition_id),
    )?;
    let processed = required_u64(
        transition,
        "processed_ticks",
        "process_ticks",
        Some(epoch),
        Some(transition_id),
    )?;
    if processed == 0 {
        return Err(violation(
            "processed_ticks_must_be_positive",
            Some(epoch),
            Some(transition_id),
        ));
    }

    let op = state
        .operations
        .get(operation_id)
        .ok_or_else(|| violation("operation_not_found", Some(epoch), Some(transition_id)))?
        .clone();
    let resource = state
        .resources
        .get(resource_id)
        .ok_or_else(|| violation("resource_not_found", Some(epoch), Some(transition_id)))?
        .clone();
    if op.status != "IN_PROCESS"
        || op.committed_resource_id.as_deref() != Some(resource_id)
        || resource.active_operation_id.as_deref() != Some(operation_id)
    {
        return Err(violation(
            "processing_does_not_match_live_resource_commitment",
            Some(epoch),
            Some(transition_id),
        ));
    }
    if resource.availability_status != "AVAILABLE" || resource.capacity_units_total == 0 {
        return Err(violation(
            "processing_resource_unavailable",
            Some(epoch),
            Some(transition_id),
        ));
    }
    if processed > op.remaining_processing_ticks {
        return Err(violation(
            "processed_ticks_exceed_remaining_work",
            Some(epoch),
            Some(transition_id),
        ));
    }

    let operation_static = find_operation(case, operation_id, epoch, transition_id)?.clone();
    let inspection_required = operation_static
        .get("inspection_required")
        .and_then(Value::as_bool)
        .ok_or_else(|| {
            violation(
                "operation_inspection_required_missing",
                Some(epoch),
                Some(transition_id),
            )
        })?;

    let op_mut = state
        .operations
        .get_mut(operation_id)
        .ok_or_else(|| violation("operation_not_found", Some(epoch), Some(transition_id)))?;
    op_mut.remaining_processing_ticks -= processed;
    if op_mut.remaining_processing_ticks == 0 {
        op_mut.committed_resource_id = None;
        op_mut.status = if inspection_required {
            "AWAITING_INSPECTION".to_string()
        } else {
            "COMPLETED".to_string()
        };
        let res_mut = state
            .resources
            .get_mut(resource_id)
            .ok_or_else(|| violation("resource_not_found", Some(epoch), Some(transition_id)))?;
        res_mut.active_operation_id = None;
    }

    Ok(())
}

fn apply_inspection_result(
    transition: &Value,
    case: &Value,
    state: &mut RuntimeState,
    epoch: u64,
    transition_id: &str,
) -> Result<(), Violation> {
    let operation_id = required_string(
        transition,
        "operation_id",
        "inspection_result",
        Some(epoch),
        Some(transition_id),
    )?;
    let method_id = required_string(
        transition,
        "method_id",
        "inspection_result",
        Some(epoch),
        Some(transition_id),
    )?;
    let disposition = required_string(
        transition,
        "disposition",
        "inspection_result",
        Some(epoch),
        Some(transition_id),
    )?;
    if !["ACCEPTED", "REJECTED", "INDETERMINATE"].contains(&disposition) {
        return Err(violation(
            "inspection_disposition_invalid",
            Some(epoch),
            Some(transition_id),
        ));
    }
    let op = state
        .operations
        .get(operation_id)
        .ok_or_else(|| violation("operation_not_found", Some(epoch), Some(transition_id)))?
        .clone();
    if op.status != "AWAITING_INSPECTION" {
        return Err(violation(
            "operation_not_awaiting_inspection",
            Some(epoch),
            Some(transition_id),
        ));
    }

    let static_operation = find_operation(case, operation_id, epoch, transition_id)?;
    let required_method = required_string(
        static_operation,
        "inspection_method_id",
        "operation",
        Some(epoch),
        Some(transition_id),
    )?;
    let process_family = required_string(
        static_operation,
        "process_family",
        "operation",
        Some(epoch),
        Some(transition_id),
    )?;
    if method_id != required_method {
        return Err(violation(
            "inspection_result_method_mismatch",
            Some(epoch),
            Some(transition_id),
        ));
    }
    validate_inspection_method(case, method_id, process_family, epoch, transition_id)?;
    if state.inspection_station.capacity_units_remaining() == 0 {
        return Err(violation(
            "inspection_capacity_exhausted",
            Some(epoch),
            Some(transition_id),
        ));
    }
    state.inspection_station.capacity_units_used += 1;
    let operation_mut = state
        .operations
        .get_mut(operation_id)
        .ok_or_else(|| violation("operation_not_found", Some(epoch), Some(transition_id)))?;
    operation_mut.status = "COMPLETED".to_string();
    operation_mut.quality_disposition = Some(disposition.to_string());
    Ok(())
}

fn find_operation<'a>(
    case: &'a Value,
    operation_id: &str,
    epoch: u64,
    transition_id: &str,
) -> Result<&'a Value, Violation> {
    case.get("jobs")
        .and_then(Value::as_array)
        .and_then(|jobs| {
            jobs.iter().find_map(|job| {
                job.get("operations")
                    .and_then(Value::as_array)
                    .and_then(|operations| {
                        operations.iter().find(|item| {
                            item.get("operation_id").and_then(Value::as_str) == Some(operation_id)
                        })
                    })
            })
        })
        .ok_or_else(|| violation("operation_not_found", Some(epoch), Some(transition_id)))
}

impl RuntimeInspection {
    fn capacity_units_remaining(&self) -> u64 {
        self.capacity_units_total
            .saturating_sub(self.capacity_units_used)
    }
}

fn make_snapshot(
    state: &RuntimeState,
    frame: &Value,
    case: &Value,
    epoch: u64,
    previous_snapshot_digest: Option<String>,
    evidence: &FrameEvidence,
) -> Result<Snapshot, Violation> {
    let mut operations: Vec<OperationState> = state.operations.values().cloned().collect();
    operations.sort_by(|left, right| left.operation_id.cmp(&right.operation_id));

    let mut resources = Vec::new();
    for runtime in state.resources.values() {
        resources.push(ResourceState {
            resource_id: runtime.resource_id.clone(),
            current_setup_family: runtime.current_setup_family.clone(),
            active_operation_id: runtime.active_operation_id.clone(),
            availability_status: runtime.availability_status.clone(),
            capacity_units_total: runtime.capacity_units_total,
            capacity_units_used: runtime.capacity_units_used,
            capacity_units_remaining: runtime
                .capacity_units_total
                .saturating_sub(runtime.capacity_units_used),
        });
    }
    resources.sort_by(|left, right| left.resource_id.cmp(&right.resource_id));

    let station = case
        .get("inspection_station")
        .ok_or_else(|| violation("inspection_station_missing", Some(epoch), None))?;
    let base_station_capacity = required_array(
        station,
        "capacity_units_by_epoch",
        "inspection_station",
        Some(epoch),
        None,
    )?;
    let _ = base_station_capacity;
    let history = required_array(
        frame,
        "observable_history",
        "decision_frame",
        Some(epoch),
        None,
    )?;
    let history_digest = digest_value(&Value::Array(history.clone()), Some(epoch), None)?;
    let frame_digest = digest_value(frame, Some(epoch), None)?;

    let mut snapshot = Snapshot {
        schema_id: SNAPSHOT_SCHEMA_ID,
        benchmark_id: required_string(frame, "benchmark_id", "decision_frame", Some(epoch), None)?
            .to_string(),
        case_id: required_string(frame, "case_id", "decision_frame", Some(epoch), None)?
            .to_string(),
        decision_epoch: epoch,
        previous_snapshot_digest,
        decision_frame_digest: frame_digest,
        visible_history_digest: history_digest,
        transition_version: TRANSITION_VERSION,
        operations,
        resources,
        inspection_station: InspectionStationState {
            station_id: state.inspection_station.station_id.clone(),
            capacity_units_total: state.inspection_station.capacity_units_total,
            capacity_units_used: state.inspection_station.capacity_units_used,
            capacity_units_remaining: state.inspection_station.capacity_units_remaining(),
        },
        event_cut: EventCut {
            decision_epoch: epoch,
            visible_source_event_ids: evidence.source_event_ids.clone(),
        },
        applied_transition_ids: state.applied_transition_ids.clone(),
        snapshot_digest: String::new(),
        claim_ceiling: SNAPSHOT_CLAIM_CEILING,
    };

    let mut body = serde_json::to_value(&snapshot)
        .map_err(|_| violation("snapshot_serialization_failed", Some(epoch), None))?;
    body.as_object_mut()
        .ok_or_else(|| violation("snapshot_object_expected", Some(epoch), None))?
        .remove("snapshot_digest");
    snapshot.snapshot_digest = digest_value(&body, Some(epoch), None)?;
    let snapshot_value = serde_json::to_value(&snapshot)
        .map_err(|_| violation("snapshot_serialization_failed", Some(epoch), None))?;
    validate_schema(
        &snapshot_value,
        SNAPSHOT_SCHEMA_JSON,
        "operational_state_snapshot",
        Some(epoch),
        None,
    )?;
    Ok(snapshot)
}

fn apply_transition(
    transition: &Value,
    frame: &Value,
    case: &Value,
    evidence: &FrameEvidence,
    state: &mut RuntimeState,
    epoch: u64,
) -> Result<(), Violation> {
    let transition_id =
        required_string(transition, "transition_id", "transition", Some(epoch), None)?;
    if !state.seen_transition_ids.insert(transition_id.to_string()) {
        return Err(violation(
            "transition_id_duplicate",
            Some(epoch),
            Some(transition_id),
        ));
    }
    let kind = required_string(
        transition,
        "kind",
        "transition",
        Some(epoch),
        Some(transition_id),
    )?;
    match kind {
        "START_ACTION" => apply_start_action(
            transition,
            frame,
            case,
            evidence,
            state,
            epoch,
            transition_id,
        )?,
        "PROCESS_TICKS" => apply_process_ticks(transition, case, state, epoch, transition_id)?,
        "INSPECTION_RESULT" => {
            apply_inspection_result(transition, case, state, epoch, transition_id)?
        }
        _ => {
            return Err(violation(
                "transition_kind_invalid",
                Some(epoch),
                Some(transition_id),
            ));
        }
    }
    state.applied_transition_ids.push(transition_id.to_string());
    Ok(())
}

fn replay_ledger(ledger: &Value) -> Result<ReplayReceipt, Violation> {
    validate_schema(ledger, LEDGER_SCHEMA_JSON, "execution_ledger", None, None)?;
    let benchmark_id =
        required_string(ledger, "benchmark_id", "execution_ledger", None, None)?.to_string();
    let case_id = required_string(ledger, "case_id", "execution_ledger", None, None)?.to_string();
    let steps = required_array(ledger, "steps", "execution_ledger", None, None)?;
    let first_frame = steps
        .first()
        .and_then(|step| step.get("decision_frame"))
        .ok_or_else(|| violation("initial_decision_frame_missing", Some(0), None))?;
    validate_schema(
        first_frame,
        FRAME_SCHEMA_JSON,
        "decision_frame",
        Some(0),
        None,
    )?;
    if required_string(first_frame, "benchmark_id", "decision_frame", Some(0), None)?
        != benchmark_id
        || required_string(first_frame, "case_id", "decision_frame", Some(0), None)? != case_id
        || required_u64(
            first_frame,
            "decision_epoch",
            "decision_frame",
            Some(0),
            None,
        )? != 0
    {
        return Err(violation(
            "initial_decision_frame_identity_or_epoch_invalid",
            Some(0),
            None,
        ));
    }

    let case = static_case(first_frame, 0)?;
    if required_string(case, "case_id", "static_case", Some(0), None)? != case_id {
        return Err(violation("static_case_identity_mismatch", Some(0), None));
    }
    let resources = index_resources(case, 0)?;
    let (initial_operations, _) = index_jobs_and_operations(case, 0)?;
    let mut state = initial_state(case, 0, &resources)?;
    let mut snapshots = Vec::new();
    let mut previous_frame_history: Option<Vec<Value>> = None;
    let mut previous_snapshot_digest = None;
    let mut all_transition_ids = BTreeSet::new();

    for (index, step) in steps.iter().enumerate() {
        let epoch = required_u64(step, "epoch", "step", None, None)?;
        if epoch != index as u64 {
            return Err(violation(
                "decision_epochs_must_be_contiguous_from_zero",
                Some(epoch),
                None,
            ));
        }
        let frame = step
            .get("decision_frame")
            .ok_or_else(|| violation("decision_frame_missing", Some(epoch), None))?;
        validate_schema(
            frame,
            FRAME_SCHEMA_JSON,
            "decision_frame",
            Some(epoch),
            None,
        )?;
        if required_string(frame, "benchmark_id", "decision_frame", Some(epoch), None)?
            != benchmark_id
            || required_string(frame, "case_id", "decision_frame", Some(epoch), None)? != case_id
            || required_u64(frame, "decision_epoch", "decision_frame", Some(epoch), None)? != epoch
        {
            return Err(violation(
                "decision_frame_identity_or_epoch_mismatch",
                Some(epoch),
                None,
            ));
        }
        let current_case = static_case(frame, epoch)?;
        if current_case != case {
            return Err(violation(
                "static_case_changed_during_episode",
                Some(epoch),
                None,
            ));
        }
        if index > 0 {
            let prior = previous_frame_history
                .as_ref()
                .ok_or_else(|| violation("prior_history_missing", Some(epoch), None))?;
            let current = required_array(
                frame,
                "observable_history",
                "decision_frame",
                Some(epoch),
                None,
            )?;
            if current.len() < prior.len() || current[..prior.len()] != prior[..] {
                return Err(violation(
                    "observable_history_prefix_mutated",
                    Some(epoch),
                    None,
                ));
            }
        }
        let evidence = validate_frame_evidence(frame, current_case, epoch, &resources)?;
        refresh_epoch_state(&mut state, current_case, &evidence, epoch)?;

        let transitions = required_array(step, "transitions", "step", Some(epoch), None)?;
        for transition in transitions {
            let transition_id =
                required_string(transition, "transition_id", "transition", Some(epoch), None)?;
            if !all_transition_ids.insert(transition_id.to_string()) {
                return Err(violation(
                    "transition_id_duplicate_in_ledger",
                    Some(epoch),
                    Some(transition_id),
                ));
            }
            apply_transition(
                transition,
                frame,
                current_case,
                &evidence,
                &mut state,
                epoch,
            )?;
        }

        let current_history = required_array(
            frame,
            "observable_history",
            "decision_frame",
            Some(epoch),
            None,
        )?;
        previous_frame_history = Some(current_history.clone());
        let snapshot = make_snapshot(
            &state,
            frame,
            current_case,
            epoch,
            previous_snapshot_digest.clone(),
            &evidence,
        )?;
        previous_snapshot_digest = Some(snapshot.snapshot_digest.clone());
        snapshots.push(snapshot);
    }

    let _ = initial_operations;
    let ledger_digest = digest_value(ledger, None, None)?;
    let final_snapshot_digest = snapshots
        .last()
        .map(|snapshot| snapshot.snapshot_digest.clone());
    Ok(ReplayReceipt {
        schema_id: REPLAY_SCHEMA_ID,
        benchmark_id,
        case_id,
        status: "VERIFIED".to_string(),
        ledger_digest,
        snapshots,
        violation_ledger: Vec::new(),
        final_snapshot_digest,
        claim_ceiling: REPLAY_CLAIM_CEILING,
    })
}

fn build_rejected_receipt(ledger: &Value, error: Violation) -> ReplayReceipt {
    let benchmark_id = ledger
        .get("benchmark_id")
        .and_then(Value::as_str)
        .unwrap_or("MFG-POLICY-V02-CHALLENGE-001")
        .to_string();
    let case_id = ledger
        .get("case_id")
        .and_then(Value::as_str)
        .unwrap_or("UNKNOWN-CASE")
        .to_string();
    let ledger_digest = digest_value(ledger, None, None).unwrap_or_else(|_| "0".repeat(64));
    ReplayReceipt {
        schema_id: REPLAY_SCHEMA_ID,
        benchmark_id,
        case_id,
        status: "REJECTED".to_string(),
        ledger_digest,
        snapshots: Vec::new(),
        violation_ledger: vec![error],
        final_snapshot_digest: None,
        claim_ceiling: REPLAY_CLAIM_CEILING,
    }
}

fn run() -> Result<(ReplayReceipt, bool), String> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 2 {
        return Err("usage: mfg-policy-v02-state-replay <execution-ledger.json>".into());
    }
    let bytes =
        std::fs::read(&args[1]).map_err(|error| format!("execution_ledger_read:{error}"))?;
    let ledger: Value =
        serde_json::from_slice(&bytes).map_err(|error| format!("execution_ledger_json:{error}"))?;

    let (receipt, verified) = match replay_ledger(&ledger) {
        Ok(receipt) => (receipt, true),
        Err(error) => (build_rejected_receipt(&ledger, error), false),
    };
    let output = serde_json::to_value(&receipt)
        .map_err(|error| format!("replay_receipt_serialize:{error}"))?;
    validate_schema(
        &output,
        REPLAY_RECEIPT_SCHEMA_JSON,
        "replay_receipt",
        None,
        None,
    )
    .map_err(|error| format!("replay_receipt_schema:{}", error.code))?;
    Ok((receipt, verified))
}

fn main() -> ExitCode {
    match run() {
        Ok((receipt, verified)) => {
            match serde_json::to_string_pretty(&receipt) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("replay_receipt_output:{error}");
                    return ExitCode::FAILURE;
                }
            }
            if verified {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(2)
            }
        }
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

    fn frame(epoch: u64) -> Value {
        let history = (0..=epoch)
            .map(|value| {
                json!({
                    "epoch": value,
                    "resource_events": [],
                    "qualification_updates": [],
                    "inspection_capacity_events": []
                })
            })
            .collect::<Vec<_>>();
        json!({
            "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-decision-frame",
            "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
            "case_id": "CASE-REPLAY-01",
            "decision_epoch": epoch,
            "static_case": {
                "case_id": "CASE-REPLAY-01",
                "horizon_epochs": 4,
                "jobs": [
                    {"job_id": "JOB-1", "operations": [
                        {
                            "operation_id": "OP-1", "sequence": 1,
                            "process_family": "FORMING", "setup_family": "SETUP-A",
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
                            "process_family": "FINISHING", "setup_family": "SETUP-C",
                            "predecessor_operation_ids": ["OP-2"], "eligible_resource_ids": ["RESOURCE-A"],
                            "processing_time_ticks_by_resource": {"RESOURCE-A": 1},
                            "initial_status": "WAITING", "committed_resource_id": null,
                            "remaining_processing_ticks": 0, "inspection_required": true,
                            "inspection_method_id": "DIMENSIONAL"
                        }
                    ]}
                ],
                "resources": [{
                    "resource_id": "RESOURCE-A",
                    "capability_families": ["FORMING", "FINISHING"],
                    "common_cause_domain": "POWER-1",
                    "capacity_units_by_epoch": [1, 1, 1, 1],
                    "initial_setup_family": "SETUP-A",
                    "initial_qualifications": [
                        {"process_family": "FORMING", "disposition": "ADMISSIBLE", "evidence_receipt_id": "Q-FORM"},
                        {"process_family": "FINISHING", "disposition": "ADMISSIBLE", "evidence_receipt_id": "Q-FINISH"}
                    ]
                }],
                "setup_transitions": [
                    {"resource_id": "RESOURCE-A", "from_setup_family": "SETUP-A", "to_setup_family": "SETUP-B", "duration_ticks": 1},
                    {"resource_id": "RESOURCE-A", "from_setup_family": "SETUP-B", "to_setup_family": "SETUP-C", "duration_ticks": 2}
                ],
                "inspection_station": {
                    "station_id": "INSPECT-1", "capacity_units_by_epoch": [1, 1, 1, 1],
                    "methods": [{
                        "method_id": "DIMENSIONAL",
                        "applicable_process_families": ["FINISHING"],
                        "duration_ticks": 1, "cost_units": 1
                    }]
                }
            },
            "observable_history": history,
            "claim_ceiling": "Synthetic policy-input projection only; no production authority."
        })
    }

    fn admission_receipt(
        epoch: u64,
        operation_id: &str,
        processing_ticks: u64,
        setup_ticks: u64,
        inspection_method_id: Option<&str>,
    ) -> Value {
        json!({
            "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-action-admission-receipt",
            "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
            "case_id": "CASE-REPLAY-01",
            "decision_epoch": epoch,
            "operation_id": operation_id,
            "resource_id": "RESOURCE-A",
            "status": "ADMISSIBLE",
            "reason_codes": [],
            "qualification_disposition": "ADMISSIBLE",
            "processing_duration_ticks": processing_ticks,
            "setup_duration_ticks": setup_ticks,
            "inspection_method_id": inspection_method_id,
            "capacity_units_required": 1,
            "claim_ceiling": ADMISSION_CLAIM_CEILING
        })
    }

    fn start_transition(
        id: &str,
        epoch: u64,
        operation_id: &str,
        action_kind: &str,
        inspection_method_id: Option<&str>,
        processing_ticks: u64,
        setup_ticks: u64,
    ) -> Value {
        json!({
            "transition_id": id,
            "kind": "START_ACTION",
            "proposal": {
                "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-action-proposal",
                "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
                "case_id": "CASE-REPLAY-01",
                "decision_epoch": epoch,
                "operation_id": operation_id,
                "resource_id": "RESOURCE-A",
                "action_kind": action_kind,
                "inspection_method_id": inspection_method_id
            },
            "admission_receipt": admission_receipt(
                epoch,
                operation_id,
                processing_ticks,
                setup_ticks,
                inspection_method_id
            )
        })
    }

    fn ledger(steps: Vec<Value>) -> Value {
        json!({
            "schema_id": "https://luminousdynamics.io/schemas/mfg-policy-v0.2-execution-ledger",
            "benchmark_id": "MFG-POLICY-V02-CHALLENGE-001",
            "case_id": "CASE-REPLAY-01",
            "evidence_class": "synthetic_execution_trace",
            "steps": steps,
            "claim_ceiling": "Synthetic event-sourced state replay only; no physical execution, source authentication, accepted production output or schedule optimality claim."
        })
    }

    fn golden_three_epoch_ledger() -> Value {
        ledger(vec![
            json!({
                "epoch": 0,
                "decision_frame": frame(0),
                "transitions": [
                    start_transition("START-OP2", 0, "OP-2", "START", None, 2, 1),
                    {"transition_id": "TICK-OP2-E0", "kind": "PROCESS_TICKS", "operation_id": "OP-2", "resource_id": "RESOURCE-A", "processed_ticks": 1}
                ]
            }),
            json!({
                "epoch": 1,
                "decision_frame": frame(1),
                "transitions": [
                    {"transition_id": "TICK-OP2-E1", "kind": "PROCESS_TICKS", "operation_id": "OP-2", "resource_id": "RESOURCE-A", "processed_ticks": 1}
                ]
            }),
            json!({
                "epoch": 2,
                "decision_frame": frame(2),
                "transitions": [
                    start_transition("START-OP3", 2, "OP-3", "START", Some("DIMENSIONAL"), 1, 2),
                    {"transition_id": "TICK-OP3-E2", "kind": "PROCESS_TICKS", "operation_id": "OP-3", "resource_id": "RESOURCE-A", "processed_ticks": 1},
                    {"transition_id": "INSPECT-OP3-E2", "kind": "INSPECTION_RESULT", "operation_id": "OP-3", "method_id": "DIMENSIONAL", "disposition": "ACCEPTED"}
                ]
            }),
        ])
    }

    #[test]
    fn three_epoch_replay_reconstructs_precedence_setup_work_and_inspection() {
        let receipt = replay_ledger(&golden_three_epoch_ledger()).unwrap();
        assert_eq!(receipt.status, "VERIFIED");
        assert_eq!(receipt.snapshots.len(), 3);

        let epoch0 = &receipt.snapshots[0];
        let op2_e0 = epoch0
            .operations
            .iter()
            .find(|op| op.operation_id == "OP-2")
            .unwrap();
        assert_eq!(op2_e0.status, "IN_PROCESS");
        assert_eq!(op2_e0.remaining_processing_ticks, 1);
        assert_eq!(epoch0.resources[0].current_setup_family, "SETUP-B");
        assert_eq!(
            epoch0.resources[0].active_operation_id.as_deref(),
            Some("OP-2")
        );

        let epoch1 = &receipt.snapshots[1];
        let op2_e1 = epoch1
            .operations
            .iter()
            .find(|op| op.operation_id == "OP-2")
            .unwrap();
        assert_eq!(op2_e1.status, "COMPLETED");
        assert_eq!(epoch1.resources[0].active_operation_id, None);

        let epoch2 = &receipt.snapshots[2];
        let op3_e2 = epoch2
            .operations
            .iter()
            .find(|op| op.operation_id == "OP-3")
            .unwrap();
        assert_eq!(op3_e2.status, "COMPLETED");
        assert_eq!(op3_e2.quality_disposition.as_deref(), Some("ACCEPTED"));
        assert_eq!(epoch2.resources[0].current_setup_family, "SETUP-C");
        assert_eq!(epoch2.inspection_station.capacity_units_used, 1);
        assert_ne!(
            receipt.snapshots[0].snapshot_digest,
            receipt.snapshots[1].snapshot_digest
        );
        assert_eq!(
            receipt.snapshots[1].previous_snapshot_digest.as_deref(),
            Some(receipt.snapshots[0].snapshot_digest.as_str())
        );
    }

    #[test]
    fn continuation_reuses_the_existing_live_resource_commitment() {
        let mut value = golden_three_epoch_ledger();
        value["steps"][1]["transitions"] = json!([
            start_transition("CONTINUE-OP2-E1", 1, "OP-2", "CONTINUE", None, 1, 0),
            {"transition_id": "TICK-OP2-E1", "kind": "PROCESS_TICKS", "operation_id": "OP-2", "resource_id": "RESOURCE-A", "processed_ticks": 1}
        ]);
        let receipt = replay_ledger(&value).unwrap();
        let op2 = receipt.snapshots[1]
            .operations
            .iter()
            .find(|operation| operation.operation_id == "OP-2")
            .unwrap();
        assert_eq!(op2.status, "COMPLETED");
        assert_eq!(receipt.snapshots[1].resources[0].active_operation_id, None);
    }

    #[test]
    fn duplicate_start_is_rejected() {
        let mut value = golden_three_epoch_ledger();
        value["steps"][0]["transitions"]
            .as_array_mut()
            .unwrap()
            .insert(
                1,
                start_transition("DUPLICATE-START", 0, "OP-2", "START", None, 2, 1),
            );
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "resource_already_has_active_operation");
    }

    #[test]
    fn forged_processing_or_setup_duration_in_receipt_is_rejected() {
        let mut value = golden_three_epoch_ledger();
        value["steps"][0]["transitions"][0]["admission_receipt"]["processing_duration_ticks"] =
            Value::from(99_u64);
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "admission_receipt_derived_fields_mismatch");
    }

    #[test]
    fn later_frame_cannot_rewrite_an_earlier_observation_prefix() {
        let mut value = golden_three_epoch_ledger();
        value["steps"][1]["decision_frame"]["observable_history"][0]["resource_events"] = json!([{
            "source_event_id": "PAST-EVENT-SMUGGLE",
            "resource_id": "RESOURCE-A",
            "kind": "FAILED",
            "occurred_at_epoch": 0,
            "observed_at_epoch": 0,
            "common_cause_id": null,
            "recovery_forecast_min_epochs": 1,
            "recovery_forecast_max_epochs": 2
        }]);
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "observable_history_prefix_mutated");
    }

    #[test]
    fn resource_failure_at_the_visible_cutoff_blocks_start() {
        let mut value = ledger(vec![json!({
            "epoch": 0,
            "decision_frame": frame(0),
            "transitions": [start_transition("START-OP2", 0, "OP-2", "START", None, 2, 1)]
        })]);
        value["steps"][0]["decision_frame"]["observable_history"][0]["resource_events"] = json!([{
            "source_event_id": "FAIL-A-0",
            "resource_id": "RESOURCE-A",
            "kind": "FAILED",
            "occurred_at_epoch": 0,
            "observed_at_epoch": 0,
            "common_cause_id": null,
            "recovery_forecast_min_epochs": 1,
            "recovery_forecast_max_epochs": 2
        }]);
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "resource_unavailable_at_action_cutoff");
    }

    #[test]
    fn double_booking_is_rejected_before_second_action_can_mutate_state() {
        let mut value = ledger(vec![json!({
            "epoch": 0,
            "decision_frame": frame(0),
            "transitions": [
                start_transition("START-OP2", 0, "OP-2", "START", None, 2, 1),
                start_transition("START-OP3-EARLY", 0, "OP-3", "START", Some("DIMENSIONAL"), 1, 2)
            ]
        })]);
        // OP-3's predecessor is not complete; the independent replay must reject it
        // before it can reserve the already committed resource.
        value["steps"][0]["transitions"][1]["admission_receipt"]["processing_duration_ticks"] =
            Value::from(1_u64);
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "resource_already_has_active_operation");
    }

    #[test]
    fn processing_cannot_exceed_remaining_work() {
        let mut value = ledger(vec![json!({
            "epoch": 0,
            "decision_frame": frame(0),
            "transitions": [
                start_transition("START-OP2", 0, "OP-2", "START", None, 2, 1),
                {"transition_id": "OVER-PROCESS", "kind": "PROCESS_TICKS", "operation_id": "OP-2", "resource_id": "RESOURCE-A", "processed_ticks": 3}
            ]
        })]);
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "processed_ticks_exceed_remaining_work");
    }

    #[test]
    fn inspection_capacity_must_be_available() {
        let mut value = golden_three_epoch_ledger();
        value["steps"][2]["decision_frame"]["observable_history"][2]["inspection_capacity_events"] = json!([{
            "source_event_id": "INSPECTION-CAP-0",
            "station_id": "INSPECT-1",
            "available_capacity_units": 0,
            "occurred_at_epoch": 2,
            "observed_at_epoch": 2
        }]);
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "inspection_capacity_exhausted");
    }

    #[test]
    fn failed_action_receipt_is_not_a_usable_execution_transition() {
        let mut value = ledger(vec![json!({
            "epoch": 0,
            "decision_frame": frame(0),
            "transitions": [start_transition("START-OP2", 0, "OP-2", "START", None, 2, 1)]
        })]);
        value["steps"][0]["transitions"][0]["admission_receipt"]["status"] = Value::from("HOLD");
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "admission_receipt_not_admissible");
    }

    #[test]
    fn setup_change_must_match_current_resource_state() {
        let mut value = golden_three_epoch_ledger();
        value["steps"][2]["transitions"][0]["admission_receipt"]["setup_duration_ticks"] =
            Value::from(1_u64);
        let error = replay_ledger(&value).unwrap_err();
        assert_eq!(error.code, "admission_receipt_derived_fields_mismatch");
    }
}
