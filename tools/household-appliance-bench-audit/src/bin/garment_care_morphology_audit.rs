use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:garment-care-morphology:0.1.0";
#[cfg(test)]
const SCHEMA_TEXT: &str =
    include_str!("../../../../schemas/laundry/garment_care_morphology_v0_1.schema.json");
#[cfg(test)]
const PROFILE_TEXT: &str =
    include_str!("../../../../fixtures/laundry/garment_care_morphology_v0_1.json");

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

fn includes(values: Option<&Value>, required: &str) -> bool {
    values
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(required)))
}

fn semantic_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if text_at(profile, "program_id") != Some("GARMENT-CARE-MORPH-001") {
        errors.push("program_id must remain GARMENT-CARE-MORPH-001".to_owned());
    }
    if text_at(profile, "status") != Some("proposal_no_physical_measurements") {
        errors.push("candidate register must remain proposal-only".to_owned());
    }
    if text_at(profile, "claim_ceiling")
        != Some("no_cleaning_superiority_or_product_qualification_claim")
    {
        errors.push(
            "morphology audit cannot grant cleaning or product qualification claims".to_owned(),
        );
    }

    let candidates = profile.get("candidates").and_then(Value::as_array);
    let mut ids = BTreeSet::new();
    let mut cabinet_ok = false;
    let mut lco2_ok = false;
    let mut solvent_ok = false;

    if let Some(candidates) = candidates {
        for candidate in candidates {
            let id = text_at(candidate, "id").unwrap_or("<missing>");
            if !ids.insert(id.to_owned()) {
                errors.push(format!("duplicate candidate identity: {id}"));
            }

            let route = text_at(candidate, "route").unwrap_or("");
            let deployment = text_at(candidate, "deployment").unwrap_or("");
            let soil = candidate
                .get("soil_removal_claim")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let dry = candidate
                .get("dry_clean_claim")
                .and_then(Value::as_bool)
                .unwrap_or(false);

            if route == "refresh_only" && (soil || dry) {
                errors.push(format!(
                    "{id}: refresh-only cannot claim soil removal or dry cleaning"
                ));
            }
            if id == "CARE-CABINET" {
                cabinet_ok = route == "refresh_only" && !soil && !dry;
            }
            if route == "professional_lco2" {
                lco2_ok = deployment == "central_specialist" && dry;
            }
            if route == "professional_solvent" {
                solvent_ok = deployment == "central_specialist" && dry;
            }
        }
    } else {
        errors.push("candidates must be an array".to_owned());
    }

    if !cabinet_ok {
        errors.push("CARE-CABINET must remain refresh-only".to_owned());
    }
    if !lco2_ok {
        errors.push("LCO2 must remain a central specialist dry-cleaning route".to_owned());
    }
    if !solvent_ok {
        errors.push("solvent cleaning must remain a central specialist route".to_owned());
    }

    let gates = profile.get("hard_gates");
    for required in [
        "garment_material_and_construction_compatibility",
        "cleaning_and_stain_acceptance",
        "fabric_dimension_colour_strength_and_finish_acceptance",
        "chemical_or_solvent_residue_acceptance",
        "worker_and_user_exposure_control",
        "process_specific_pressure_fire_electrical_and_thermal_safety",
        "wastewater_solvent_and_microfiber_mass_balance",
        "abstention_for_unknown_or_out_of_scope_garments",
    ] {
        if !includes(gates, required) {
            errors.push(format!("required hard gate is missing: {required}"));
        }
    }

    let governance = profile.get("governance");
    let physical_observations = governance
        .and_then(|value| value.get("physical_observations"))
        .and_then(Value::as_u64);
    if physical_observations != Some(0) {
        errors.push("physical observation count must remain zero".to_owned());
    }

    let product_claims = governance
        .and_then(|value| value.get("product_claims"))
        .and_then(Value::as_bool);
    if product_claims != Some(false) {
        errors.push("product claims cannot be promoted by morphology contract".to_owned());
    }

    if text_at(governance.unwrap_or(&Value::Null), "symthaea_authority")
        != Some("advisory_only_no_actuation")
    {
        errors.push("Symthaea must remain advisory-only".to_owned());
    }

    let shared_circuits = governance
        .and_then(|value| value.get("solvent_circuits_shared"))
        .and_then(Value::as_bool);
    if shared_circuits != Some(false) {
        errors.push("solvent families must not share fluid circuits".to_owned());
    }

    let lco2_home = governance
        .and_then(|value| value.get("lco2_household_authorized"))
        .and_then(Value::as_bool);
    if lco2_home != Some(false) {
        errors.push("LCO2 is not authorized for household prototypes".to_owned());
    }

    let pce_new_equipment = governance
        .and_then(|value| value.get("pce_new_equipment_selected"))
        .and_then(Value::as_bool);
    if pce_new_equipment != Some(false) {
        errors.push("PCE cannot be selected for new equipment".to_owned());
    }

    if text_at(
        governance.unwrap_or(&Value::Null),
        "unknown_material_action",
    ) != Some("abstain_or_human_review")
    {
        errors.push("unknown material must trigger abstention or human review".to_owned());
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

    let candidate_count = profile["candidates"].as_array().map_or(0, Vec::len);
    let source_count = profile["sources"].as_array().map_or(0, Vec::len);
    Ok(json!({
        "audit": "GARMENT_CARE_MORPHOLOGY",
        "status": "PASS_MORPHOLOGY_CONTRACT_ONLY",
        "schema_id": SCHEMA_ID,
        "candidate_count": candidate_count,
        "source_count": source_count,
        "physical_observations": 0,
        "product_claims": false,
        "schema_sha256": digest(schema_bytes),
        "profile_sha256": digest(profile_bytes)
    }))
}

fn args_value(args: &[String], name: &str) -> Result<String, String> {
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
    let schema_path = args_value(&args, "--schema").map_err(|error| vec![error])?;
    let profile_path = args_value(&args, "--profile").map_err(|error| vec![error])?;
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
            eprintln!("garment-care morphology audit failed:");
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

    fn valid(schema: &Value, profile: &Value) -> bool {
        schema_validator(schema)
            .expect("schema compiles")
            .is_valid(profile)
            && semantic_errors(profile).is_empty()
    }

    #[test]
    fn base_morphology_contract_passes() {
        let (schema, profile) = fixture();
        assert!(valid(&schema, &profile));
        assert_eq!(
            profile["candidates"].as_array().expect("candidates").len(),
            8
        );
    }

    #[test]
    fn refresh_cabinet_cannot_claim_cleaning_or_dry_cleaning() {
        let (schema, mut profile) = fixture();
        profile["candidates"][1]["soil_removal_claim"] = Value::Bool(true);
        profile["candidates"][1]["dry_clean_claim"] = Value::Bool(true);
        assert!(!valid(&schema, &profile));
    }

    #[test]
    fn lco2_cannot_be_household_prototype() {
        let (schema, mut profile) = fixture();
        profile["candidates"][4]["deployment"] = Value::String("home".to_owned());
        assert!(!valid(&schema, &profile));
    }

    #[test]
    fn solvent_circuits_cannot_be_shared() {
        let (schema, mut profile) = fixture();
        profile["governance"]["solvent_circuits_shared"] = Value::Bool(true);
        assert!(!valid(&schema, &profile));
    }

    #[test]
    fn pce_cannot_be_selected_for_new_equipment() {
        let (schema, mut profile) = fixture();
        profile["governance"]["pce_new_equipment_selected"] = Value::Bool(true);
        assert!(!valid(&schema, &profile));
    }

    #[test]
    fn unknown_material_requires_abstention() {
        let (schema, mut profile) = fixture();
        profile["governance"]["unknown_material_action"] =
            Value::String("infer_and_execute".to_owned());
        assert!(!valid(&schema, &profile));
    }

    #[test]
    fn symthaea_cannot_control_physical_process() {
        let (schema, mut profile) = fixture();
        profile["governance"]["symthaea_authority"] =
            Value::String("closed_loop_process_controller".to_owned());
        assert!(!valid(&schema, &profile));
    }

    #[test]
    fn physical_observations_cannot_be_fabricated() {
        let (schema, mut profile) = fixture();
        profile["governance"]["physical_observations"] = json!(1);
        assert!(!valid(&schema, &profile));
    }
}
