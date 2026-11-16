use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:laundry-service-benchmark:0.1.0";
const SCHEMA_TEXT: &str =
    include_str!("../../../../schemas/laundry/laundry_service_benchmark_v0_1.schema.json");
const PROFILE_TEXT: &str =
    include_str!("../../../../fixtures/laundry/laundry_service_benchmark_v0_1.json");

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
        return Err("schema does not validate against the Draft 2020-12 meta-schema".to_owned());
    }
    jsonschema::draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile schema: {error}"))
}

fn semantic_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if text_at(profile, "benchmark_id") != Some("LAUNDRY-SERVICE-BENCH-001") {
        errors.push("benchmark_id must remain LAUNDRY-SERVICE-BENCH-001".to_owned());
    }
    if text_at(profile, "status") != Some("proposed_no_physical_measurements") {
        errors.push("profile must remain proposed_no_physical_measurements".to_owned());
    }
    if text_at(profile, "claim_ceiling") != Some("no_physical_performance_or_superiority_claim") {
        errors.push(
            "claim ceiling cannot be promoted without a new evidence-qualified contract".to_owned(),
        );
    }

    let functional_unit = profile.get("functional_unit");
    if functional_unit.and_then(|v| text_at(v, "id")) != Some("kg_accepted_dry_equivalent_textile")
    {
        errors.push("functional unit must be kg_accepted_dry_equivalent_textile".to_owned());
    }
    let separate_views = functional_unit
        .and_then(|v| v.get("separate_views"))
        .and_then(Value::as_array);
    let expected_views = ["washer_service", "dryer_service", "paired_laundry_service"];
    for expected in expected_views {
        if !separate_views
            .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
        {
            errors.push(format!(
                "functional unit must keep {expected} separately reportable"
            ));
        }
    }

    let control_policy = profile.get("control_policy");
    if control_policy
        .and_then(|v| v.get("minimum_model_families_per_class"))
        .and_then(Value::as_u64)
        != Some(3)
    {
        errors.push(
            "control policy must retain a minimum of three model families per class".to_owned(),
        );
    }
    if control_policy
        .and_then(|v| v.get("selected_controls"))
        .and_then(Value::as_array)
        .is_none_or(|items| !items.is_empty())
    {
        errors
            .push("desk references must not be promoted to selected physical controls".to_owned());
    }

    let metrics = profile.get("metrics").and_then(Value::as_array);
    if metrics.is_none_or(|items| items.len() < 16) {
        errors.push("at least 16 metric definitions are required".to_owned());
    }
    let mut metric_ids = BTreeSet::new();
    if let Some(items) = metrics {
        for (index, metric) in items.iter().enumerate() {
            let id = text_at(metric, "id");
            if let Some(id) = id {
                if !metric_ids.insert(id.to_owned()) {
                    errors.push(format!("duplicate metric id: {id}"));
                }
            } else {
                errors.push(format!("metrics[{index}] is missing id"));
            }
            if text_at(metric, "status") != Some("proposal_not_result") {
                errors.push(format!(
                    "metrics[{index}] target status must remain proposal_not_result"
                ));
            }
            for field in ["unit", "denominator", "method", "target_hypothesis"] {
                if text_at(metric, field).is_none_or(str::is_empty) {
                    errors.push(format!("metrics[{index}].{field} must be non-empty"));
                }
            }
            if metric
                .get("hard_guardrails")
                .and_then(Value::as_array)
                .is_none_or(Vec::is_empty)
            {
                errors.push(format!(
                    "metrics[{index}] must have at least one hard guardrail"
                ));
            }
        }
    }

    let baselines = profile.get("baselines").and_then(Value::as_array);
    if baselines.is_none_or(|items| items.len() < 2) {
        errors.push("at least one washer and one dryer desk baseline are required".to_owned());
    } else {
        let items = baselines.expect("baseline array checked above");
        let washer = items
            .iter()
            .any(|item| text_at(item, "appliance_class") == Some("front_load_washer"));
        let dryer = items
            .iter()
            .any(|item| text_at(item, "appliance_class") == Some("heat_pump_tumble_dryer"));
        if !washer || !dryer {
            errors.push(
                "desk baselines must include both a front-load washer and heat-pump dryer"
                    .to_owned(),
            );
        }
        for (index, baseline) in items.iter().enumerate() {
            if text_at(baseline, "status") != Some("desk_reference_not_selected_control") {
                errors.push(format!(
                    "baselines[{index}] cannot be promoted to a selected control"
                ));
            }
            if text_at(baseline, "source_uri").is_none_or(|uri| !uri.starts_with("https://")) {
                errors.push(format!("baselines[{index}] must retain an HTTPS source"));
            }
        }
    }

    let hard_gates = profile.get("hard_gates").and_then(Value::as_array);
    let required_gates = [
        "safety_and_conformity",
        "cleaning_acceptance",
        "rinse_acceptance",
        "textile_gentleness",
        "final_moisture_acceptance",
        "microfiber_discharge",
        "repair_role_boundary",
    ];
    for gate in required_gates {
        if !hard_gates.is_some_and(|items| items.iter().any(|item| item.as_str() == Some(gate))) {
            errors.push(format!("required non-compensatory gate is missing: {gate}"));
        }
    }

    let governance = profile.get("governance");
    if text_at(governance.unwrap_or(&Value::Null), "symthaea_authority")
        != Some("advisory_only_no_actuation")
    {
        errors.push("Symthaea must remain advisory-only with no actuation authority".to_owned());
    }
    if governance
        .and_then(|v| v.get("safety_interlock_authority"))
        .and_then(Value::as_str)
        != Some("independent_deterministic_device_controls")
    {
        errors.push(
            "safety interlocks must remain independent deterministic device controls".to_owned(),
        );
    }
    if governance
        .and_then(|v| v.get("offline_operation_required"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("offline operation must remain required".to_owned());
    }
    if governance
        .and_then(|v| v.get("physical_observations"))
        .and_then(Value::as_u64)
        != Some(0)
    {
        errors.push("physical observation count must remain zero until evidenced".to_owned());
    }
    if governance
        .and_then(|v| v.get("product_qualification_claims"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("product qualification claims must remain false".to_owned());
    }

    errors
}

fn digest(bytes: &[u8]) -> String {
    let hash = Sha256::digest(bytes);
    hash.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn audit(schema_bytes: &[u8], profile_bytes: &[u8]) -> Result<Value, Vec<String>> {
    let schema: Value = serde_json::from_slice(schema_bytes)
        .map_err(|error| vec![format!("schema JSON parse error: {error}")])?;
    let profile: Value = serde_json::from_slice(profile_bytes)
        .map_err(|error| vec![format!("profile JSON parse error: {error}")])?;

    let validator = schema_validator(&schema).map_err(|error| vec![error])?;
    let mut errors: Vec<String> = validator
        .iter_errors(&profile)
        .map(|error| format!("schema validation: {error}"))
        .collect();
    errors.extend(semantic_errors(&profile));
    if !errors.is_empty() {
        return Err(errors);
    }

    let metric_count = profile
        .get("metrics")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);
    let baseline_count = profile
        .get("baselines")
        .and_then(Value::as_array)
        .map_or(0, Vec::len);

    Ok(json!({
        "audit_status": "contract_passed",
        "claim_scope": "schema_and_semantic_contract_only",
        "benchmark_id": text_at(&profile, "benchmark_id").unwrap_or_default(),
        "profile_status": text_at(&profile, "status").unwrap_or_default(),
        "metric_count": metric_count,
        "desk_baseline_count": baseline_count,
        "physical_observations": 0,
        "product_qualification_claims": false,
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
            eprintln!("laundry service benchmark audit failed:");
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
    fn frozen_profile_passes_schema_and_semantic_contract() {
        let (schema, profile) = fixture();
        assert!(is_valid(&schema, &profile));
        assert_eq!(
            profile["metrics"].as_array().expect("metrics array").len(),
            25
        );
        let baseline_count = profile["baselines"]
            .as_array()
            .expect("baselines array")
            .len();
        assert!(baseline_count >= 2, "retain at least washer and dryer desk references");
    }

    #[test]
    fn duplicate_metric_identity_is_rejected() {
        let (schema, mut profile) = fixture();
        let first = profile["metrics"][0]["id"].clone();
        profile["metrics"][1]["id"] = first;
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn target_cannot_be_promoted_to_measured_result() {
        let (schema, mut profile) = fixture();
        profile["metrics"][0]["status"] = Value::String("measured_result".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn desk_reference_cannot_be_promoted_to_selected_control() {
        let (schema, mut profile) = fixture();
        profile["baselines"][0]["status"] = Value::String("selected_control".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn safety_gate_cannot_be_removed() {
        let (schema, mut profile) = fixture();
        profile["hard_gates"]
            .as_array_mut()
            .expect("hard gates")
            .retain(|gate| gate.as_str() != Some("safety_and_conformity"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn ai_actuation_authority_cannot_be_granted() {
        let (schema, mut profile) = fixture();
        profile["governance"]["symthaea_authority"] =
            Value::String("closed_loop_actuator_controller".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn cloud_dependency_cannot_replace_offline_operation() {
        let (schema, mut profile) = fixture();
        profile["governance"]["offline_operation_required"] = Value::Bool(false);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn metric_without_a_hard_guardrail_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["metrics"][0]["hard_guardrails"] = json!([]);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn paired_functional_unit_cannot_be_replaced_by_cycle_count() {
        let (schema, mut profile) = fixture();
        profile["functional_unit"]["id"] = Value::String("per_cycle".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn missing_physical_evidence_stays_explicitly_zero() {
        let (schema, mut profile) = fixture();
        profile["governance"]["physical_observations"] = json!(1);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn no_product_qualification_is_inferred_from_contract_validation() {
        let (schema, mut profile) = fixture();
        profile["governance"]["product_qualification_claims"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }
}
