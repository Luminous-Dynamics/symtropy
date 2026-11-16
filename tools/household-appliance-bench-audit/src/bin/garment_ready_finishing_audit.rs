use jsonschema::Validator;
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:garment-ready-finishing:0.1.0";
#[cfg(test)]
const SCHEMA_TEXT: &str =
    include_str!("../../../../schemas/laundry/garment_ready_finishing_v0_1.schema.json");
#[cfg(test)]
const PROFILE_TEXT: &str =
    include_str!("../../../../fixtures/laundry/garment_ready_finishing_v0_1.json");

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

fn array_has(values: Option<&Value>, expected: &str) -> bool {
    values
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| item.as_str() == Some(expected)))
}

fn semantic_errors(profile: &Value) -> Vec<String> {
    let mut errors = Vec::new();

    if text_at(profile, "program_id") != Some("GARMENT-READY-001") {
        errors.push("program_id must remain GARMENT-READY-001".to_owned());
    }
    if text_at(profile, "status") != Some("proposal_no_physical_measurements") {
        errors.push("profile must remain proposal-only".to_owned());
    }
    if text_at(profile, "claim_ceiling") != Some("no_endpoint_robot_or_product_qualification_claim")
    {
        errors.push("profile cannot grant endpoint or robot qualification claims".to_owned());
    }

    let endpoints = profile.get("endpoint_classes").and_then(Value::as_array);
    for required in ["ready_to_wear", "ready_to_store", "ready_to_iron"] {
        if !endpoints.is_some_and(|items| items.iter().any(|item| item.as_str() == Some(required)))
        {
            errors.push(format!("required endpoint class missing: {required}"));
        }
    }

    let suites = profile
        .get("benchmark_suites")
        .and_then(Value::as_array);
    let mut suite_ids = BTreeSet::new();
    if let Some(suites) = suites {
        for suite in suites {
            let id = text_at(suite, "id").unwrap_or("<missing>");
            if !suite_ids.insert(id.to_owned()) {
                errors.push(format!("duplicate benchmark suite: {id}"));
            }
            if suite
                .get("human_baseline_required")
                .and_then(Value::as_bool)
                != Some(true)
            {
                errors.push(format!("{id}: matched human baseline is required"));
            }
            let first_pass = suite
                .get("first_pass_success_target")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let stretch = suite
                .get("stretch_success_target")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if stretch < first_pass {
                errors.push(format!("{id}: stretch success target must not be below first-pass target"));
            }
            let first_throughput = suite
                .get("first_pass_human_throughput_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            let stretch_throughput = suite
                .get("stretch_human_throughput_ratio")
                .and_then(Value::as_f64)
                .unwrap_or(0.0);
            if stretch_throughput < first_throughput {
                errors.push(format!("{id}: stretch throughput target must not be below first-pass target"));
            }
            if id == "standardized_commercial_textiles"
                && (first_pass <= 0.994 || first_throughput <= 0.6)
            {
                errors.push(
                    "standardized commercial target must exceed the reported DYNA napkin baseline"
                        .to_owned(),
                );
            }
            if id == "diverse_household_garments" && first_pass <= 0.991 {
                errors.push(
                    "diverse household target must exceed the reported ACT-2 aggregate baseline"
                        .to_owned(),
                );
            }
        }
    } else {
        errors.push("benchmark_suites must be an array".to_owned());
    }
    for required in [
        "standardized_commercial_textiles",
        "diverse_household_garments",
    ] {
        if !suite_ids.contains(required) {
            errors.push(format!("required non-pooled benchmark suite missing: {required}"));
        }
    }

    let comparators = profile
        .get("external_comparators")
        .and_then(Value::as_array);
    let mut comparator_ids = BTreeSet::new();
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
                errors.push(format!("{id}: external benchmark cannot be presented as our result"));
            }
            let suite_id = text_at(comparator, "suite_id").unwrap_or("");
            if !suite_ids.contains(suite_id) {
                errors.push(format!("{id}: comparator references unknown or pooled suite"));
            }
        }
    } else {
        errors.push("external_comparators must be an array".to_owned());
    }

    let metrics = profile.get("metrics").and_then(Value::as_array);
    let mut ids = BTreeSet::new();
    if let Some(metrics) = metrics {
        for metric in metrics {
            let id = text_at(metric, "id").unwrap_or("<missing>");
            if !ids.insert(id.to_owned()) {
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
    }

    let comparison = profile.get("comparison_policy");
    for (field, label) in [
        ("same_garment_cohort_required", "same garment cohort"),
        ("same_task_definition_required", "same task definition"),
        (
            "human_baseline_required_for_robot_speed_claim",
            "human speed baseline",
        ),
        ("failed_attempts_retained", "failed attempt retention"),
        (
            "vendor_claims_are_external_only",
            "external vendor claim separation",
        ),
        ("P50_P95_time_required", "P50/P95 time reporting"),
        (
            "intervention_and_damage_accounting_required",
            "intervention/damage accounting",
        ),
    ] {
        if comparison
            .and_then(|value| value.get(field))
            .and_then(Value::as_bool)
            != Some(true)
        {
            errors.push(format!("{label} is required"));
        }
    }

    let standard = profile.get("standard_watch");
    if text_at(standard.unwrap_or(&Value::Null), "published_predecessor") != Some("ISO 15487:2018")
    {
        errors.push("published predecessor must remain ISO 15487:2018".to_owned());
    }
    if text_at(standard.unwrap_or(&Value::Null), "current_reference")
        != Some("ISO/DIS 15487:2026 Edition 4")
    {
        errors.push("current reference must identify ISO/DIS 15487:2026 Edition 4".to_owned());
    }
    if text_at(standard.unwrap_or(&Value::Null), "current_reference_status")
        != Some("draft_under_development")
    {
        errors.push("ISO/DIS 15487 cannot be relabelled as published".to_owned());
    }
    if standard
        .and_then(|value| value.get("published_standard"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("draft ISO/DIS 15487 cannot be marked published".to_owned());
    }

    for required in [
        "parent_cleaning_and_rinse_acceptance",
        "endpoint_moisture_distribution",
        "appearance_and_fabric_integrity",
        "no_lost_or_damaged_garments",
        "manual_recovery_and_emergency_stop",
        "no_hygiene_claim_from_steam_or_drying",
    ] {
        if !array_has(profile.get("hard_gates"), required) {
            errors.push(format!("required hard gate missing: {required}"));
        }
    }

    let governance = profile.get("governance");
    if governance
        .and_then(|value| value.get("physical_observations"))
        .and_then(Value::as_u64)
        != Some(0)
    {
        errors.push("physical observation count must remain zero".to_owned());
    }
    if governance
        .and_then(|value| value.get("product_claims"))
        .and_then(Value::as_bool)
        != Some(false)
    {
        errors.push("product claims cannot be promoted".to_owned());
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
    if text_at(governance.unwrap_or(&Value::Null), "contract_receipt_scope")
        != Some("benchmark_contract_only")
    {
        errors.push("audit receipt must remain contract-only".to_owned());
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
        "audit": "GARMENT_READY_FINISHING",
        "status": "PASS_BENCHMARK_CONTRACT_ONLY",
        "schema_id": SCHEMA_ID,
        "endpoint_class_count": profile["endpoint_classes"].as_array().map_or(0, Vec::len),
        "metric_count": profile["metrics"].as_array().map_or(0, Vec::len),
        "physical_observations": 0,
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
            eprintln!("garment-ready benchmark audit failed:");
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
    fn ready_state_contract_passes_with_three_explicit_endpoints() {
        let (schema, profile) = fixture();
        assert!(is_valid(&schema, &profile));
        assert_eq!(profile["metrics"].as_array().expect("metrics").len(), 10);
    }

    #[test]
    fn ready_to_iron_is_not_collapsed_into_ready_to_wear() {
        let (schema, mut profile) = fixture();
        profile["endpoint_classes"]
            .as_array_mut()
            .expect("endpoints")
            .retain(|item| item.as_str() != Some("ready_to_iron"));
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn vendor_claims_cannot_replace_same_cohort_human_baseline() {
        let (schema, mut profile) = fixture();
        profile["comparison_policy"]["human_baseline_required_for_robot_speed_claim"] =
            Value::Bool(false);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn failed_attempts_cannot_be_discarded() {
        let (schema, mut profile) = fixture();
        profile["comparison_policy"]["failed_attempts_retained"] = Value::Bool(false);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn draft_iso_standard_cannot_be_marked_published() {
        let (schema, mut profile) = fixture();
        profile["standard_watch"]["published_standard"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn iso_draft_status_cannot_be_relabelled() {
        let (schema, mut profile) = fixture();
        profile["standard_watch"]["current_reference_status"] =
            Value::String("published".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn symthaea_cannot_be_granted_physical_actuation_authority() {
        let (schema, mut profile) = fixture();
        profile["governance"]["symthaea_authority"] =
            Value::String("closed_loop_actuator_controller".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn external_robotics_results_cannot_be_claimed_as_ours() {
        let (schema, mut profile) = fixture();
        profile["external_comparators"][0]["is_our_result"] = Value::Bool(true);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn standardized_and_household_robot_tasks_cannot_be_pooled() {
        let (schema, mut profile) = fixture();
        profile["benchmark_suites"][1]["id"] =
            Value::String("standardized_commercial_textiles".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn proposed_targets_must_exceed_external_frontier() {
        let (schema, mut profile) = fixture();
        profile["benchmark_suites"][0]["first_pass_success_target"] = json!(0.99);
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn external_comparator_must_reference_a_declared_suite() {
        let (schema, mut profile) = fixture();
        profile["external_comparators"][0]["suite_id"] =
            Value::String("pooled_all_garments".to_owned());
        assert!(!is_valid(&schema, &profile));
    }

    #[test]
    fn physical_observations_cannot_be_fabricated() {
        let (schema, mut profile) = fixture();
        profile["governance"]["physical_observations"] = json!(1);
        assert!(!is_valid(&schema, &profile));
    }
}
