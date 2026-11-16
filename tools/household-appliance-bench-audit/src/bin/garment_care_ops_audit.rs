use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:garment-care-ops:0.1.0";
#[cfg(test)]
const SCHEMA_TEXT: &str =
    include_str!("../../../../schemas/laundry/garment_care_ops_v0_1.schema.json");
#[cfg(test)]
const PROFILE_TEXT: &str = include_str!("../../../../fixtures/laundry/garment_care_ops_v0_1.json");

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("schema $id must be {SCHEMA_ID}"));
    }
    if !jsonschema::draft202012::meta::is_valid(schema) {
        return Err("schema does not validate against Draft 2020-12 meta-schema".to_owned());
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile schema: {error}"))
}

fn contains(values: Option<&Value>, expected: &str) -> bool {
    values
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn semantic_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if text_at(profile, "program_id") != Some("GARMENT-OPS-001") {
        errors.push("program_id must remain GARMENT-OPS-001".to_owned());
    }
    if text_at(profile, "status") != Some("proposal_no_physical_observations") {
        errors.push("profile must remain proposal-only".to_owned());
    }
    if text_at(profile, "claim_ceiling")
        != Some("no_production_recorder_or_reliability_qualification")
    {
        errors.push("profile cannot make production recorder or reliability claims".to_owned());
    }

    let required_events = [
        "production_run",
        "sop_step",
        "failure_event",
        "intervention_event",
        "machine_health_event",
        "resource_event",
        "custody_event",
    ];
    let event_values = profile.get("event_types");
    let mut event_ids = BTreeSet::new();
    if let Some(items) = event_values.and_then(Value::as_array) {
        for item in items {
            if let Some(value) = item.as_str() {
                if !event_ids.insert(value.to_owned()) {
                    errors.push(format!("duplicate event type: {value}"));
                }
            }
        }
    } else {
        errors.push("event_types must be an array".to_owned());
    }
    for required in required_events {
        if !event_ids.contains(required) {
            errors.push(format!("required episode event type missing: {required}"));
        }
    }

    let required_fields = profile.get("required_event_fields");
    for required in [
        "episode_id",
        "sequence_number",
        "previous_event_digest",
        "event_digest",
        "wall_time_utc",
        "monotonic_time",
        "site_station_device_identity",
        "hardware_revision",
        "software_model_config_digest",
        "task_sop_acceptance_rubric_version",
        "destination_terminal_state",
        "artifact_digest_custody_state",
    ] {
        if !contains(required_fields, required) {
            errors.push(format!("required event field missing: {required}"));
        }
    }

    let terminal_states = profile.get("terminal_states");
    for required in [
        "accepted_destination",
        "quarantined",
        "human_handoff",
        "lost_or_damaged",
    ] {
        if !contains(terminal_states, required) {
            errors.push(format!("required terminal state missing: {required}"));
        }
    }

    let reconciliation = profile.get("reconciliation_policy");
    for (key, label) in [
        ("attempts_reconcile", "attempt reconciliation"),
        (
            "eventual_acceptance_reconciles",
            "eventual acceptance reconciliation",
        ),
        ("retry_linked_to_original", "retry linkage"),
        (
            "every_attempt_has_one_terminal_state",
            "one terminal state per attempt",
        ),
        (
            "unknown_terminal_blocks_summary",
            "unknown terminal fail-closed behavior",
        ),
        (
            "wrong_destination_is_not_success",
            "wrong destination rejection",
        ),
    ] {
        if reconciliation
            .and_then(|value| value.get(key))
            .and_then(Value::as_bool)
            != Some(true)
        {
            errors.push(format!("{label} is required"));
        }
    }

    let custody = profile.get("custody_policy");
    for (key, expected, label) in [
        ("local_first_recording", true, "local-first recording"),
        (
            "offline_episode_recording",
            true,
            "offline episode recording",
        ),
        (
            "network_required_for_local_recovery",
            false,
            "network independence",
        ),
        (
            "promoted_artifact_digest_required",
            true,
            "promoted artifact digest",
        ),
        (
            "delete_local_only_after_receipt",
            true,
            "receipt-before-delete",
        ),
        (
            "partial_upload_is_failure",
            true,
            "partial-upload failure state",
        ),
        (
            "ring_buffer_capacity_fault_is_visible",
            true,
            "ring-buffer capacity fault",
        ),
        (
            "raw_video_required_for_routine_episode",
            false,
            "routine raw-video minimization",
        ),
    ] {
        if custody
            .and_then(|value| value.get(key))
            .and_then(Value::as_bool)
            != Some(expected)
        {
            errors.push(format!("{label} policy is invalid"));
        }
    }

    let required_faults = [
        "calibration_drift",
        "gripper_wear",
        "station_layout_change",
        "lighting_shift",
        "garment_lot_substitution",
        "clock_skew",
        "network_partition",
        "ring_buffer_capacity_exhaustion",
        "partial_upload",
        "artifact_corruption",
        "sensor_dropout",
        "duplicate_sequence_number",
    ];
    for required in required_faults {
        if !contains(profile.get("fault_injection_scenarios"), required) {
            errors.push(format!(
                "required fault injection scenario missing: {required}"
            ));
        }
    }

    let failure_families = profile.get("failure_families");
    for required in [
        "intake_singulation",
        "perception_material",
        "manipulation_contact",
        "process_endpoint",
        "destination_placement",
        "machine_health",
        "human_system_interaction",
        "evidence_integrity",
        "unclassified",
    ] {
        if !contains(failure_families, required) {
            errors.push(format!("required failure family missing: {required}"));
        }
    }

    let metrics = profile.get("metrics").and_then(Value::as_array);
    let mut metric_ids = BTreeSet::new();
    if let Some(metrics) = metrics {
        for metric in metrics {
            let id = text_at(metric, "id").unwrap_or("<missing>");
            if !metric_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate metric id: {id}"));
            }
            if text_at(metric, "status") != Some("proposal_target_only") {
                errors.push(format!(
                    "{id}: target cannot be promoted to measured result"
                ));
            }
            if text_at(metric, "denominator").is_none_or(str::is_empty) {
                errors.push(format!("{id}: denominator must be explicit"));
            }
            if metric
                .get("hard_guardrails")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
            {
                errors.push(format!("{id}: at least one hard guardrail is required"));
            }
        }
    } else {
        errors.push("metrics must be an array".to_owned());
    }

    let governance = profile.get("governance");
    for field in [
        "physical_observations",
        "production_episodes",
        "trained_failure_models",
    ] {
        if governance
            .and_then(|value| value.get(field))
            .and_then(Value::as_u64)
            != Some(0)
        {
            errors.push(format!("{field} must remain zero in the proposal fixture"));
        }
    }
    if governance
        .and_then(|value| value.get("product_claims"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("product claims cannot be promoted".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "symthaea_authority")
        != Some("advisory_diagnostics_only_no_actuation")
    {
        errors.push("Symthaea must remain diagnostics-only and advisory".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "physical_interlocks")
        != Some("independent_deterministic_device_controls")
    {
        errors.push("physical interlocks must remain independent".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "claim_scope") != Some("contract_only") {
        errors.push("claim scope must remain contract-only".to_owned());
    }

    errors
}

fn digest(bytes: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(bytes);
    format!("{:x}", hasher.finalize())
}

fn audit(schema_bytes: &[u8], profile_bytes: &[u8]) -> Result<Value, Vec<String>> {
    let schema: Value = serde_json::from_slice(schema_bytes)
        .map_err(|error| vec![format!("schema JSON parse failed: {error}")])?;
    let profile: Value = serde_json::from_slice(profile_bytes)
        .map_err(|error| vec![format!("profile JSON parse failed: {error}")])?;
    let validator = schema_validator(&schema).map_err(|error| vec![error])?;
    let mut errors: Vec<String> = validator
        .iter_errors(&profile)
        .map(|error| error.to_string())
        .collect();
    errors.extend(semantic_errors(&profile));
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(json!({
        "audit": "GARMENT_CARE_OPS",
        "status": "PASS_BENCHMARK_CONTRACT_ONLY",
        "schema_id": SCHEMA_ID,
        "event_type_count": profile["event_types"].as_array().map_or(0, Vec::len),
        "metric_count": profile["metrics"].as_array().map_or(0, Vec::len),
        "failure_family_count": profile["failure_families"].as_array().map_or(0, Vec::len),
        "fault_injection_count": profile["fault_injection_scenarios"].as_array().map_or(0, Vec::len),
        "physical_observations": 0,
        "production_episodes": 0,
        "product_claims": false,
        "schema_sha256": digest(schema_bytes),
        "profile_sha256": digest(profile_bytes)
    }))
}

fn argument_value(args: &[String], name: &str) -> Result<String, String> {
    let mut matches = args
        .windows(2)
        .filter(|pair| pair[0] == name)
        .map(|pair| pair[1].clone());
    let value = matches
        .next()
        .ok_or_else(|| format!("missing required argument {name}"))?;
    if matches.next().is_some() {
        return Err(format!("argument {name} must appear exactly once"));
    }
    Ok(value)
}

fn run() -> Result<Value, Vec<String>> {
    let args: Vec<String> = env::args().skip(1).collect();
    let schema_path = argument_value(&args, "--schema").map_err(|error| vec![error])?;
    let profile_path = argument_value(&args, "--profile").map_err(|error| vec![error])?;
    let schema_bytes = fs::read(&schema_path)
        .map_err(|error| vec![format!("cannot read schema {schema_path}: {error}")])?;
    let profile_bytes = fs::read(&profile_path)
        .map_err(|error| vec![format!("cannot read profile {profile_path}: {error}")])?;
    audit(&schema_bytes, &profile_bytes)
}

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt).unwrap_or_default()
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("garment-care ops audit failed:");
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value) {
        let schema: Value = serde_json::from_str(SCHEMA_TEXT).expect("schema parses");
        let profile: Value = serde_json::from_str(PROFILE_TEXT).expect("profile parses");
        (schema, profile)
    }

    fn is_valid(schema: &Value, profile: &Value) -> bool {
        schema_validator(schema)
            .expect("schema compiles")
            .is_valid(profile)
            && semantic_errors(profile).is_empty()
    }

    #[test]
    fn proposal_contract_passes_without_fabricated_production_episodes() {
        let (schema, profile) = fixture();
        assert!(is_valid(&schema, &profile));
        assert_eq!(profile["event_types"].as_array().expect("events").len(), 7);
        assert_eq!(profile["metrics"].as_array().expect("metrics").len(), 11);
        assert_eq!(
            profile["fault_injection_scenarios"]
                .as_array()
                .expect("faults")
                .len(),
            12
        );
        assert_eq!(profile["governance"]["production_episodes"], json!(0));
    }

    #[test]
    fn network_is_not_required_for_local_episode_recording() {
        let (schema, mut profile) = fixture();
        profile["custody_policy"]["network_required_for_local_recovery"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn local_evidence_cannot_be_deleted_before_receipt() {
        let (schema, mut profile) = fixture();
        profile["custody_policy"]["delete_local_only_after_receipt"] = Value::Bool(false);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn partial_upload_must_remain_a_failure() {
        let (schema, mut profile) = fixture();
        profile["custody_policy"]["partial_upload_is_failure"] = Value::Bool(false);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn raw_video_is_not_mandatory_for_routine_episodes() {
        let (schema, mut profile) = fixture();
        profile["custody_policy"]["raw_video_required_for_routine_episode"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn duplicate_sequences_are_injected_as_a_negative_control() {
        let (schema, mut profile) = fixture();
        profile["fault_injection_scenarios"]
            .as_array_mut()
            .expect("faults")
            .retain(|value| value.as_str() != Some("duplicate_sequence_number"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn missing_terminal_state_reconciliation_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["reconciliation_policy"]["every_attempt_has_one_terminal_state"] =
            Value::Bool(false);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn retry_must_remain_linked_to_original_attempt() {
        let (schema, mut profile) = fixture();
        profile["reconciliation_policy"]["retry_linked_to_original"] = Value::Bool(false);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn unknown_failure_must_not_be_mapped_to_success() {
        let (schema, mut profile) = fixture();
        profile["failure_families"]
            .as_array_mut()
            .expect("failure families")
            .retain(|value| value.as_str() != Some("unclassified"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn symthaea_cannot_be_granted_actuation_authority() {
        let (schema, mut profile) = fixture();
        profile["governance"]["symthaea_authority"] =
            Value::String("closed_loop_actuator_controller".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn production_observations_cannot_be_fabricated() {
        let (schema, mut profile) = fixture();
        profile["governance"]["production_episodes"] = json!(1);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn broken_chain_event_field_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["required_event_fields"]
            .as_array_mut()
            .expect("fields")
            .retain(|value| value.as_str() != Some("previous_event_digest"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn removing_network_partition_control_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["fault_injection_scenarios"]
            .as_array_mut()
            .expect("faults")
            .retain(|value| value.as_str() != Some("network_partition"));
        assert!(!is_valid(&schema, &profile));
    }
}
