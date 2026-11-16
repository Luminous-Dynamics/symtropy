use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:garment-material-passport:0.1.0";
#[cfg(test)]
const SCHEMA_TEXT: &str =
    include_str!("../../../../schemas/laundry/garment_material_passport_v0_1.schema.json");
#[cfg(test)]
const PROFILE_TEXT: &str =
    include_str!("../../../../fixtures/laundry/garment_material_passport_v0_1.json");

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

fn array_has(values: Option<&Value>, expected: &str) -> bool {
    values.and_then(Value::as_array).is_some_and(|items| {
        items.iter().any(|item| item.as_str() == Some(expected))
    })
}

fn semantic_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if text_at(profile, "program_id") != Some("GARMENT-MATERIAL-PASSPORT-001") {
        errors.push("program_id must remain GARMENT-MATERIAL-PASSPORT-001".to_owned());
    }
    if text_at(profile, "status")
        != Some("benchmark_contract_proposal_no_physical_measurements")
    {
        errors.push("profile must remain proposal-only until physical scan evidence exists".to_owned());
    }
    if text_at(profile, "claim_ceiling") != Some("no_model_or_process_qualification_claim") {
        errors.push("profile cannot claim model or process qualification".to_owned());
    }

    let datasets = profile.get("datasets").and_then(Value::as_array);
    let mut dataset_ids = BTreeSet::new();
    if let Some(datasets) = datasets {
        for dataset in datasets {
            let id = text_at(dataset, "id").unwrap_or("<missing>");
            if !dataset_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate dataset identity: {id}"));
            }
            if dataset.get("not_our_measurement").and_then(Value::as_bool) != Some(true) {
                errors.push(format!("{id}: external corpus cannot be presented as our physical measurement"));
            }
        }
    } else {
        errors.push("datasets must be an array".to_owned());
    }

    let external = profile.get("external_reference_results").and_then(Value::as_array);
    let mut reference_ids = BTreeSet::new();
    if let Some(results) = external {
        for result in results {
            let id = text_at(result, "id").unwrap_or("<missing>");
            if !reference_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate external reference result: {id}"));
            }
            if result.get("is_our_result").and_then(Value::as_bool) != Some(false) {
                errors.push(format!("{id}: literature result cannot be relabelled as our result"));
            }
            let dataset_id = text_at(result, "dataset_id").unwrap_or("");
            if !dataset_ids.contains(dataset_id) {
                errors.push(format!("{id}: external result references an unknown dataset"));
            }
        }
    } else {
        errors.push("external_reference_results must be an array".to_owned());
    }

    let validation = profile.get("validation_policy");
    let split_fields = validation
        .and_then(|value| value.get("split_group_fields"))
        .and_then(Value::as_array);
    for field in ["physical_specimen_id", "source_lot_id", "acquisition_session_id"] {
        if !split_fields.is_some_and(|items| items.iter().any(|item| item.as_str() == Some(field))) {
            errors.push(format!("validation split must group by {field}"));
        }
    }
    if validation
        .and_then(|value| value.get("random_spectral_pixel_split_allowed"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("random spectral-pixel splitting is prohibited".to_owned());
    }
    if validation
        .and_then(|value| value.get("instrument_holdout_required"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("instrument holdout is required".to_owned());
    }
    if validation
        .and_then(|value| value.get("chemical_ground_truth_required_for_blend_claim"))
        .and_then(Value::as_bool)
        != Some(true)
    {
        errors.push("blend regression claims require independent chemical ground truth".to_owned());
    }

    let metrics = profile.get("metrics").and_then(Value::as_array);
    let mut metric_ids = BTreeSet::new();
    if let Some(metrics) = metrics {
        for metric in metrics {
            let id = text_at(metric, "id").unwrap_or("<missing>");
            if !metric_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate metric identity: {id}"));
            }
            if text_at(metric, "status") != Some("proposal_target_only") {
                errors.push(format!("{id}: metric target cannot be promoted to a result"));
            }
        }
    }

    for required in [
        "ood_detection_and_abstention",
        "multi_region_and_multilayer_uncertainty",
        "no_nir_only_process_suitability_claim",
        "no_ai_actuation_or_interlock_override",
        "dataset_license_and_provenance",
    ] {
        if !array_has(profile.get("hard_gates"), required) {
            errors.push(format!("required hard gate is missing: {required}"));
        }
    }

    let governance = profile.get("governance");
    if governance
        .and_then(|value| value.get("physical_scan_observations"))
        .and_then(Value::as_u64)
        != Some(0)
    {
        errors.push("physical scan observation count must remain zero".to_owned());
    }
    if governance
        .and_then(|value| value.get("trained_models"))
        .and_then(Value::as_u64)
        != Some(0)
    {
        errors.push("trained model count cannot be fabricated".to_owned());
    }
    if governance
        .and_then(|value| value.get("product_claims"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("product qualification claims cannot be promoted".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "care_route_authority")
        != Some("advisory_material_estimate_human_gate")
    {
        errors.push("material estimate must remain advisory with a human gate".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "symthaea_authority")
        != Some("advisory_only_no_actuation")
    {
        errors.push("Symthaea must remain advisory-only, without physical actuation".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "physical_interlocks")
        != Some("independent_deterministic_device_controls")
    {
        errors.push("physical interlocks must remain independent deterministic controls".to_owned());
    }
    if profile
        .get("scan_records")
        .and_then(Value::as_array)
        .is_none_or(|records| !records.is_empty())
    {
        errors.push("proposal fixture must not contain fabricated scan records".to_owned());
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
        .map(|error| format!("schema validation: {error}"))
        .collect();
    errors.extend(semantic_errors(&profile));
    if !errors.is_empty() {
        return Err(errors);
    }

    Ok(json!({
        "audit": "GARMENT_MATERIAL_PASSPORT",
        "status": "PASS_BENCHMARK_CONTRACT_ONLY",
        "schema_id": SCHEMA_ID,
        "dataset_count": profile["datasets"].as_array().map_or(0, Vec::len),
        "metric_count": profile["metrics"].as_array().map_or(0, Vec::len),
        "external_reference_count": profile["external_reference_results"].as_array().map_or(0, Vec::len),
        "physical_scan_observations": 0,
        "trained_models": 0,
        "product_claims": false,
        "schema_sha256": digest(schema_bytes),
        "profile_sha256": digest(profile_bytes)
    }))
}

fn argument_value(args: &[String], name: &str) -> Result<String, String> {
    let mut found = args
        .windows(2)
        .filter(|pair| pair[0] == name)
        .map(|pair| pair[1].clone());
    let value = found
        .next()
        .ok_or_else(|| format!("missing required argument {name}"))?;
    if found.next().is_some() {
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
            eprintln!("garment material passport audit failed:");
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
    fn benchmark_contract_passes_without_fabricated_scans() {
        let (schema, profile) = fixture();
        assert!(is_valid(&schema, &profile));
        assert_eq!(profile["datasets"].as_array().expect("datasets").len(), 3);
        assert_eq!(profile["scan_records"].as_array().expect("scans").len(), 0);
    }

    #[test]
    fn random_pixel_split_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["validation_policy"]["random_spectral_pixel_split_allowed"] =
            Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn missing_specimen_grouping_is_rejected() {
        let (schema, mut profile) = fixture();
        profile["validation_policy"]["split_group_fields"] =
            serde_json::json!(["acquisition_session_id"]);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn external_published_result_cannot_be_claimed_as_ours() {
        let (schema, mut profile) = fixture();
        profile["external_reference_results"][0]["is_our_result"] =
            Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn material_prediction_cannot_authorize_dry_cleaning() {
        let (schema, mut profile) = fixture();
        profile["governance"]["care_route_authority"] =
            Value::String("nir_only_automatic_dry_clean_approval".to_owned());
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
    fn synthetic_scans_cannot_be_inserted_in_proposal_fixture() {
        let (schema, mut profile) = fixture();
        profile["scan_records"] = serde_json::json!([{"specimen_id":"SYNTHETIC-1"}]);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn model_count_cannot_be_fabricated() {
        let (schema, mut profile) = fixture();
        profile["governance"]["trained_models"] = serde_json::json!(1);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn required_ood_gate_cannot_be_removed() {
        let (schema, mut profile) = fixture();
        profile["hard_gates"]
            .as_array_mut()
            .expect("hard gates")
            .retain(|gate| gate.as_str() != Some("ood_detection_and_abstention"));
        assert!(!is_valid(&schema, &profile));
    }
}
