use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:mnb-external-loop:0.1.0";
const SCHEMA_TEXT: &str =
    include_str!("../../../../schemas/laundry/mnb_external_loop_v0_1.schema.json");
const PROFILE_TEXT: &str = include_str!("../../../../fixtures/laundry/mnb_external_loop_v0_1.json");

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

fn contains_all(values: Option<&Value>, expected: &[&str]) -> bool {
    let Some(items) = values.and_then(Value::as_array) else {
        return false;
    };
    expected
        .iter()
        .all(|expected| items.iter().any(|item| item.as_str() == Some(*expected)))
}

fn semantic_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if text_at(profile, "experiment_id") != Some("LAUNDRY-MNB-LOOP-001") {
        errors.push("experiment_id must remain LAUNDRY-MNB-LOOP-001".to_owned());
    }
    if text_at(profile, "status") != Some("protocol_proposed_no_physical_runs") {
        errors.push("profile must remain protocol_proposed_no_physical_runs".to_owned());
    }
    if text_at(profile, "claim_ceiling") != Some("no_cleaning_energy_or_product_superiority_claim")
    {
        errors.push("claim ceiling cannot be promoted by a protocol audit".to_owned());
    }

    let expected_arms = ["C0", "C1", "C2", "C3", "C4", "C5"];
    let arms = profile.get("control_arms").and_then(Value::as_array);
    let mut arm_ids = BTreeSet::new();
    if let Some(arms) = arms {
        for arm in arms {
            if let Some(id) = text_at(arm, "id") {
                if !arm_ids.insert(id.to_owned()) {
                    errors.push(format!("duplicate control arm id: {id}"));
                }
            }
        }
        for expected in expected_arms {
            if !arm_ids.contains(expected) {
                errors.push(format!("required control arm is missing: {expected}"));
            }
        }
    } else {
        errors.push("control_arms must be an array".to_owned());
    }

    let required_energy = [
        "bubble_generator",
        "recirculation_pump",
        "heaters",
        "controls_and_instrumentation",
    ];
    if !contains_all(profile.get("energy_boundary"), &required_energy) {
        errors.push(
            "complete-loop energy boundary must include generator, pump, heaters and controls/instrumentation"
                .to_owned(),
        );
    }

    let required_channels = [
        "bubble_size_distribution",
        "bubble_number_concentration",
        "dissolved_gas_concentration",
        "gas_flow",
        "liquid_flow",
        "differential_pressure",
        "generator_power_and_energy",
        "pump_power_and_energy",
        "heater_power_and_energy",
        "controls_power_and_energy",
        "total_loop_energy",
        "stain_specific_cleaning_measurement",
        "rinse_residue",
        "textile_strength_colour_and_dimensions",
    ];
    if !contains_all(profile.get("measurement_channels"), &required_channels) {
        errors.push(
            "measurement channels omit a required bubble, hydraulic, energy or outcome observation"
                .to_owned(),
        );
    }

    let required_gates = [
        "cleaning_acceptance_by_stain_and_fabric",
        "rinse_residue_acceptance",
        "textile_gentleness_non_inferiority",
        "calibrated_complete_loop_energy_accounting",
        "stable_bubble_characterization_and_hydraulics",
        "safe_pressure_electrical_thermal_and_gas_containment",
        "microfiber_wastewater_and_service_burden",
    ];
    if !contains_all(profile.get("acceptance_gates"), &required_gates) {
        errors.push("all seven non-compensatory acceptance gates must remain present".to_owned());
    }

    let analysis = profile.get("analysis");
    for field in [
        "randomized_blocking_required",
        "pilot_variance_before_confirmatory_sample_size",
        "preregistered_analysis_before_unblinding",
        "blinded_cleaning_measurement_where_practicable",
        "report_confidence_intervals_and_uncertainty",
    ] {
        if analysis
            .and_then(|value| value.get(field))
            .and_then(Value::as_bool)
            != Some(true)
        {
            errors.push(format!("analysis requirement must remain enabled: {field}"));
        }
    }

    let governance = profile.get("governance");
    if governance
        .and_then(|value| value.get("physical_run_count"))
        .and_then(Value::as_u64)
        != Some(0)
    {
        errors.push("physical run count must remain zero until evidence exists".to_owned());
    }
    if governance
        .and_then(|value| value.get("result_records"))
        .and_then(Value::as_array)
        .is_none_or(|items| !items.is_empty())
    {
        errors.push("protocol-only fixture cannot contain physical result records".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "symthaea_authority")
        != Some("advisory_only_no_actuation")
    {
        errors.push("Symthaea must remain advisory-only with no actuation authority".to_owned());
    }
    if text_at(
        governance.unwrap_or(&Value::Null),
        "safety_interlock_authority",
    ) != Some("independent_deterministic_device_controls")
    {
        errors.push(
            "safety interlocks must remain independent deterministic device controls".to_owned(),
        );
    }
    if governance
        .and_then(|value| value.get("offline_protocol_required"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("offline protocol operation must remain required".to_owned());
    }
    if governance
        .and_then(|value| value.get("product_claims"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("protocol audit must not authorize product claims".to_owned());
    }

    let hypotheses = profile.get("decision_hypotheses").and_then(Value::as_array);
    if hypotheses.is_none_or(|items| items.len() < 3) {
        errors.push("at least three separate resource hypotheses are required".to_owned());
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
    let schema_errors: Vec<String> = validator
        .iter_errors(&profile)
        .map(|error| error.to_string())
        .collect();
    if !schema_errors.is_empty() {
        return Err(schema_errors);
    }

    let errors = semantic_errors(&profile);
    if !errors.is_empty() {
        return Err(errors);
    }

    let arm_count = profile["control_arms"].as_array().map_or(0, Vec::len);
    let channel_count = profile["measurement_channels"]
        .as_array()
        .map_or(0, Vec::len);
    Ok(json!({
        "audit": "MNB_EXTERNAL_LOOP_PROTOCOL",
        "status": "PASS_PROTOCOL_CONTRACT_ONLY",
        "schema_id": SCHEMA_ID,
        "experiment_id": profile["experiment_id"],
        "control_arm_count": arm_count,
        "measurement_channel_count": channel_count,
        "physical_run_count": 0,
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
            eprintln!("MNB external-loop protocol audit failed:");
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
    fn frozen_protocol_passes_schema_and_semantic_contract() {
        let (schema, profile) = fixture();
        assert!(is_valid(&schema, &profile));
        assert_eq!(profile["control_arms"].as_array().expect("arms").len(), 6);
        assert_eq!(profile["governance"]["physical_run_count"], 0);
    }

    #[test]
    fn missing_hydraulic_sham_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["control_arms"]
            .as_array_mut()
            .expect("arms")
            .retain(|arm| text_at(arm, "id") != Some("C2"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn missing_generator_energy_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["energy_boundary"]
            .as_array_mut()
            .expect("energy boundary")
            .retain(|item| item.as_str() != Some("bubble_generator"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn missing_total_loop_energy_channel_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["measurement_channels"]
            .as_array_mut()
            .expect("channels")
            .retain(|item| item.as_str() != Some("total_loop_energy"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn physical_runs_cannot_be_fabricated_by_protocol_fixture() {
        let (schema, mut profile) = fixture();
        profile["governance"]["physical_run_count"] = json!(1);
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
    fn acceptance_gates_cannot_be_removed() {
        let (schema, mut profile) = fixture();
        profile["acceptance_gates"]
            .as_array_mut()
            .expect("gates")
            .retain(|gate| gate.as_str() != Some("cleaning_acceptance_by_stain_and_fabric"));
        assert!(!is_valid(&schema, &profile));
    }
}
