use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:enzyme-care:0.1.0";
#[cfg(test)]
const SCHEMA_TEXT: &str =
    include_str!("../../../../schemas/laundry/enzyme_care_v0_1.schema.json");
#[cfg(test)]
const PROFILE_TEXT: &str =
    include_str!("../../../../fixtures/laundry/enzyme_care_v0_1.json");

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

    if text_at(profile, "program_id") != Some("ENZYME-CARE-001") {
        errors.push("program_id must remain ENZYME-CARE-001".to_owned());
    }
    if text_at(profile, "status") != Some("proposal_no_physical_measurements") {
        errors.push("profile must remain proposal-only".to_owned());
    }
    if text_at(profile, "claim_ceiling")
        != Some("no_formulation_or_garment_process_qualification")
    {
        errors.push("profile cannot claim formulation or garment-process qualification".to_owned());
    }

    let tracks = profile.get("tracks").and_then(Value::as_array);
    let mut track_ids = BTreeSet::new();
    if let Some(tracks) = tracks {
        for track in tracks {
            let id = text_at(track, "id").unwrap_or("<missing>");
            if !track_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate experimental track: {id}"));
            }
        }
    } else {
        errors.push("tracks must be an array".to_owned());
    }
    for required in [
        "wool_anti_felting_finishing",
        "stain_specific_enzyme_care",
    ] {
        if !track_ids.contains(required) {
            errors.push(format!("required independent track missing: {required}"));
        }
    }

    let comparators = profile
        .get("external_comparators")
        .and_then(Value::as_array);
    let mut comparator_ids = BTreeSet::new();
    let mut have_pcdI = false;
    let mut have_pure_wool_enzyme = false;
    let mut have_reductant = false;
    let mut have_blood_stain = false;
    let mut has_blood_60_c = false;
    if let Some(comparators) = comparators {
        for comparator in comparators {
            let id = text_at(comparator, "id").unwrap_or("<missing>");
            if !comparator_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate external comparator: {id}"));
            }
            if comparator
                .get("is_our_result")
                .and_then(Value::as_bool)
                != Some(false)
            {
                errors.push(format!("{id}: external paper result cannot be presented as ours"));
            }
            let track_id = text_at(comparator, "track_id").unwrap_or("");
            if !track_ids.contains(track_id) {
                errors.push(format!("{id}: comparator refers to an unknown track"));
            }
            let family = text_at(comparator, "treatment_family").unwrap_or("");
            let enzyme_only_claim = comparator
                .get("enzyme_only_claim")
                .and_then(Value::as_bool)
                .unwrap_or(false);
            let auxiliaries = comparator
                .get("auxiliary_chemistry")
                .and_then(Value::as_array)
                .map(Vec::as_slice)
                .unwrap_or(&[]);
            if enzyme_only_claim && !auxiliaries.is_empty() {
                errors.push(format!(
                    "{id}: enzyme-only claim cannot include auxiliary chemistry"
                ));
            }
            if !enzyme_only_claim
                && family == "enzyme_only"
                && !auxiliaries.is_empty()
            {
                errors.push(format!(
                    "{id}: enzyme-only treatment family cannot include auxiliary chemistry"
                ));
            }
            if id == "WOOL-PCDI-PREFILL-2025" {
                have_pcdI = true;
                if family != "enzyme_plus_polymer_prefill"
                    || enzyme_only_claim
                    || !auxiliaries.iter().any(|value| {
                        value.as_str().is_some_and(|text| text.contains("PCDI"))
                    })
                {
                    errors.push("PCDI comparator must be labelled polymer-assisted, not enzyme-only".to_owned());
                }
            }
            if id == "WOOL-WAVE-RIDING-PROTEASE-2025" {
                have_pure_wool_enzyme = true;
                if family != "enzyme_only" || !auxiliaries.is_empty() || !enzyme_only_claim {
                    errors.push("protease-only wool comparator must remain enzyme-only".to_owned());
                }
            }
            if family == "enzyme_plus_reductant" {
                have_reductant = true;
                if enzyme_only_claim || auxiliaries.is_empty() {
                    errors.push(format!("{id}: reductant-assisted route must expose its auxiliary"));
                }
            }
            if id == "BLOOD-PROTEASE-LIPASE-2025" {
                have_blood_stain = true;
                if track_id != "stain_specific_enzyme_care"
                    || family != "enzyme_plus_detergent"
                    || enzyme_only_claim
                {
                    errors.push("blood-stain study cannot be promoted as wool or enzyme-only evidence".to_owned());
                }
                has_blood_60_c = comparator
                    .get("reported_temperature_c")
                    .and_then(Value::as_f64)
                    == Some(60.0);
            }
        }
    } else {
        errors.push("external_comparators must be an array".to_owned());
    }
    for (present, label) in [
        (have_pcdI, "PCDI-assisted wool comparator"),
        (have_pure_wool_enzyme, "protease-only wool comparator"),
        (have_reductant, "reductant-assisted wool comparator"),
        (have_blood_stain, "blood-stain enzyme comparator"),
        (has_blood_60_c, "60 C blood-stain condition"),
    ] {
        if !present {
            errors.push(format!("required external evidence class missing: {label}"));
        }
    }

    let controls = profile.get("control_arms");
    for required in [
        "wool_untreated_reference",
        "wool_sham_auxiliary_without_enzyme",
        "wool_protease_only",
        "wool_pcdI_prefill_plus_protease",
        "wool_thioglycolate_or_gmt_plus_protease",
        "wool_bisulfite_plus_protease",
        "stain_water_only",
        "stain_matched_standard_detergent",
        "stain_detergent_plus_protease_lipase",
        "stain_inactive_enzyme_control",
    ] {
        if !contains(controls, required) {
            errors.push(format!("required control arm missing: {required}"));
        }
    }

    let standards = profile.get("standards");
    if text_at(standards.unwrap_or(&Value::Null), "domestic_wash_dry")
        != Some("ISO 6330:2021")
        || text_at(standards.unwrap_or(&Value::Null), "dimensional_change")
            != Some("ISO 5077:2007")
        || text_at(standards.unwrap_or(&Value::Null), "colour_fastness")
            != Some("ISO 105-C06:2010")
        || text_at(standards.unwrap_or(&Value::Null), "colour_fastness_role")
            != Some("colour_fastness_only_not_stain_removal_metric")
    {
        errors.push("standard method roles are incorrect or incomplete".to_owned());
    }

    let validation = profile.get("validation_policy");
    for (key, expected, label) in [
        ("independent_specimen_and_lot_holdout", true, "independent specimen and lot holdout"),
        ("random_measurement_split_allowed", false, "random measurement split prohibition"),
        ("repeated_specimen_is_independent_n", false, "repeat-measurement dependence"),
        ("swatch_only_proves_garment_washability", false, "swatch-to-garment inference prohibition"),
        ("repeat_cycles_required", true, "repeat cycle requirement"),
        ("blinded_assessment_required", true, "blinded assessment"),
        ("pilot_variance_before_sample_size", true, "pilot variance before sample-size selection"),
    ] {
        if validation.and_then(|value| value.get(key)).and_then(Value::as_bool) != Some(expected) {
            errors.push(format!("{label} policy is invalid"));
        }
    }

    let safety = profile.get("safety_policy");
    for (key, expected, label) in [
        ("closed_handling", true, "closed handling"),
        ("open_powder_or_aerosol_allowed", false, "no open powder/aerosol"),
        ("characterized_supplier_preparations", true, "characterized enzyme sourcing"),
        ("enzyme_residual_assessment", true, "enzyme residual assessment"),
        ("auxiliary_chemistry_assessment", true, "auxiliary chemistry assessment"),
        ("wastewater_assessment", true, "wastewater assessment"),
        ("live_microbe_appliance_production_allowed", false, "no live-microbe appliance production"),
        ("occupational_exposure_assessment", true, "occupational exposure assessment"),
    ] {
        if safety.and_then(|value| value.get(key)).and_then(Value::as_bool) != Some(expected) {
            errors.push(format!("{label} policy is invalid"));
        }
    }

    let metrics = profile.get("metrics").and_then(Value::as_array);
    let mut metric_ids = BTreeSet::new();
    if let Some(metrics) = metrics {
        for metric in metrics {
            let id = text_at(metric, "id").unwrap_or("<missing>");
            if !metric_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate metric ID: {id}"));
            }
            if text_at(metric, "status") != Some("proposal_target_only") {
                errors.push(format!("{id}: proposed target cannot be promoted to measured result"));
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

    let hard_gates = profile.get("hard_gates");
    for required in [
        "enzyme_only_and_auxiliary_assisted_routes_separately_labelled",
        "PCDI_reductant_and_other_auxiliaries_have_independent_residual_and_exposure_assessment",
        "stain_track_not_used_as_evidence_for_wool_washability",
        "60_degree_blood_stain_result_not_relabelled_as_cold_wash",
        "swatch_result_not_relabelled_as_garment_durability",
        "repeat_cycle_and_lot_holdout_required",
        "closed_enzyme_handling_and_occupational_exposure_control",
        "wastewater_and_whole_process_resource_boundary",
        "no_symthaea_actuation_or_interlock_override",
        "no_universal_dry_clean_replacement_claim",
    ] {
        if !contains(hard_gates, required) {
            errors.push(format!("required hard gate missing: {required}"));
        }
    }

    let governance = profile.get("governance");
    for field in ["physical_observations", "trained_models", "formulations_qualified"] {
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
        || governance
            .and_then(|value| value.get("dry_clean_replacement_claim"))
            .and_then(Value::as_bool)
            != Some(false)
    {
        errors.push("product or dry-clean-replacement claims cannot be promoted".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "symthaea_authority")
        != Some("advisory_only_no_actuation")
    {
        errors.push("Symthaea must remain advisory-only".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "physical_interlocks")
        != Some("independent_deterministic_device_controls")
    {
        errors.push("physical interlocks must remain independent".to_owned());
    }
    if text_at(governance.unwrap_or(&Value::Null), "claim_scope")
        != Some("experiment_contract_only")
    {
        errors.push("claim scope must remain experiment-contract-only".to_owned());
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
        "audit": "ENZYME_CARE",
        "status": "PASS_EXPERIMENT_CONTRACT_ONLY",
        "schema_id": SCHEMA_ID,
        "track_count": profile["tracks"].as_array().map_or(0, Vec::len),
        "external_comparator_count": profile["external_comparators"].as_array().map_or(0, Vec::len),
        "control_arm_count": profile["control_arms"].as_array().map_or(0, Vec::len),
        "metric_count": profile["metrics"].as_array().map_or(0, Vec::len),
        "physical_observations": 0,
        "formulations_qualified": 0,
        "dry_clean_replacement_claim": false,
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
            eprintln!("enzyme-care experiment audit failed:");
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
    fn experiment_contract_passes_without_fabricated_results() {
        let (schema, profile) = fixture();
        assert!(is_valid(&schema, &profile));
        assert_eq!(profile["tracks"].as_array().expect("tracks").len(), 2);
        assert_eq!(
            profile["external_comparators"]
                .as_array()
                .expect("comparators")
                .len(),
            5
        );
        assert_eq!(profile["governance"]["physical_observations"], json!(0));
    }

    #[test]
    fn pcdi_assisted_treatment_cannot_be_labelled_enzyme_only() {
        let (schema, mut profile) = fixture();
        profile["external_comparators"][0]["enzyme_only_claim"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn reductant_assisted_treatment_must_disclose_auxiliary() {
        let (schema, mut profile) = fixture();
        profile["external_comparators"][2]["auxiliary_chemistry"] = json!([]);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn protease_only_comparator_cannot_silently_add_auxiliary() {
        let (schema, mut profile) = fixture();
        profile["external_comparators"][1]["auxiliary_chemistry"] =
            json!(["undisclosed polymer"]);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn blood_stain_result_cannot_be_relabelled_as_wool_washability() {
        let (schema, mut profile) = fixture();
        profile["external_comparators"][4]["track_id"] =
            Value::String("wool_anti_felting_finishing".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn sixty_degree_stain_result_cannot_be_called_cold_wash() {
        let (schema, mut profile) = fixture();
        profile["external_comparators"][4]["reported_temperature_c"] = json!(30);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn ISO_colour_fastness_method_is_not_a_stain_removal_metric() {
        let (schema, mut profile) = fixture();
        profile["standards"]["colour_fastness_role"] =
            Value::String("stain_removal_index".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn swatch_test_does_not_qualify_whole_garment_washability() {
        let (schema, mut profile) = fixture();
        profile["validation_policy"]["swatch_only_proves_garment_washability"] =
            Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn repeated_measurements_cannot_be_counted_as_independent_specimens() {
        let (schema, mut profile) = fixture();
        profile["validation_policy"]["repeated_specimen_is_independent_n"] =
            Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn open_enzyme_powder_or_aerosol_is_not_permitted() {
        let (schema, mut profile) = fixture();
        profile["safety_policy"]["open_powder_or_aerosol_allowed"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn no_live_microbe_production_in_appliance() {
        let (schema, mut profile) = fixture();
        profile["safety_policy"]["live_microbe_appliance_production_allowed"] =
            Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn no_universal_dry_clean_replacement_claim() {
        let (schema, mut profile) = fixture();
        profile["governance"]["dry_clean_replacement_claim"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn physical_observations_cannot_be_fabricated() {
        let (schema, mut profile) = fixture();
        profile["governance"]["physical_observations"] = json!(1);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn stain_metric_cannot_omit_reference_and_guardrail_scope() {
        let (schema, mut profile) = fixture();
        profile["metrics"][6]["hard_guardrails"] = json!([]);
        assert!(!is_valid(&schema, &profile));
    }
}
