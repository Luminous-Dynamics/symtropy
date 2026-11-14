use jsonschema::draft202012;
use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:mobility:continuity:scenario:0.1";
const VERSION: &str = "household-mobility-continuity-benchmark/0.1";
const ARCHITECTURES: [&str; 3] = [
    "single_vehicle_baseline",
    "single_vehicle_repairability_oriented",
    "household_mobility_bundle",
];
const STRESSORS: [&str; 8] = [
    "nominal",
    "repair_delay",
    "critical_part_unavailability",
    "fuel_price_shock",
    "cashflow_contraction",
    "repair_delay_plus_fuel_price_shock",
    "critical_part_plus_cashflow_contraction",
    "combined_supply_fuel_cashflow_shock",
];
const NEGATIVE_CONTROLS: [(&str, &str); 5] = [
    ("NC-01", "safety_hold_bypassed"),
    ("NC-02", "unknown_lifecycle_cost_imputed_as_zero"),
    ("NC-03", "synthetic_result_relabelled_as_observed"),
    ("NC-04", "unavailable_fallback_counted_as_served"),
    ("NC-05", "advertised_from_price_treated_as_lifecycle_cost"),
];

type Errors = Vec<String>;

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    scenario_count: usize,
    architecture_count: usize,
    stressor_count: usize,
    negative_control_count: usize,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn close_to(value: Option<f64>, expected: f64) -> bool {
    value.is_some_and(|actual| (actual - expected).abs() < 1.0e-9)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("schema $id must be {SCHEMA_ID}"));
    }
    if !draft202012::meta::is_valid(schema) {
        return Err("schema does not validate against the Draft 2020-12 meta-schema".to_owned());
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile mobility-continuity schema: {error}"))
}

fn ids_from_array(
    value: &Value,
    key: &str,
    errors: &mut Errors,
) -> BTreeSet<String> {
    let mut ids = BTreeSet::new();
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{key}: expected array"));
        return ids;
    };
    for (index, item) in items.iter().enumerate() {
        match text_at(item, "id") {
            Some(id) if !id.trim().is_empty() => {
                if !ids.insert(id.to_owned()) {
                    errors.push(format!("{key}[{index}]: duplicate id {id}"));
                }
            }
            _ => errors.push(format!("{key}[{index}]: missing non-empty id")),
        }
    }
    ids
}

fn check_number(
    value: &Value,
    path: &str,
    expected: f64,
    errors: &mut Errors,
) {
    if !close_to(value.as_f64(), expected) {
        errors.push(format!("{path}: expected frozen v0.1 fixture value {expected}"));
    }
}

fn detects_anti_pattern(control: &Value) -> bool {
    let Some(input) = control.get("adversarial_input") else {
        return false;
    };
    match text_at(control, "anti_pattern") {
        Some("safety_hold_bypassed") => {
            input.get("active_safety_hold").and_then(Value::as_bool) == Some(true)
                && input.get("dispatch_allowed").and_then(Value::as_bool) == Some(true)
        }
        Some("unknown_lifecycle_cost_imputed_as_zero") => {
            text_at(input, "lifecycle_cost_status") == Some("unknown")
                && close_to(input.get("reported_lifecycle_cost_r").and_then(Value::as_f64), 0.0)
        }
        Some("synthetic_result_relabelled_as_observed") => {
            text_at(input, "input_evidence_class") == Some("synthetic_scenario_input")
                && text_at(input, "claimed_output_evidence_class")
                    == Some("observed_field_measurement")
        }
        Some("unavailable_fallback_counted_as_served") => {
            input.get("fallback_available").and_then(Value::as_bool) == Some(false)
                && input.get("trips_demanded").and_then(Value::as_u64).unwrap_or(0) > 0
                && input.get("trips_counted_served").and_then(Value::as_u64).unwrap_or(0) > 0
        }
        Some("advertised_from_price_treated_as_lifecycle_cost") => {
            text_at(input, "price_basis") == Some("advertised_from")
                && input.get("claimed_as_lifecycle_cost").and_then(Value::as_bool) == Some(true)
        }
        _ => false,
    }
}

fn validate_semantics(corpus: &Value, errors: &mut Errors) -> AuditSummary {
    if text_at(corpus, "schema_version") != Some(VERSION) {
        errors.push(format!("schema_version must be {VERSION}"));
    }

    let architecture_ids = ids_from_array(corpus, "architecture_profiles", errors);
    let expected_architectures: BTreeSet<String> =
        ARCHITECTURES.iter().map(|value| (*value).to_owned()).collect();
    if architecture_ids != expected_architectures {
        errors.push("architecture_profiles must contain exactly the three frozen archetypes".to_owned());
    }

    let stressor_ids = ids_from_array(corpus, "stress_profiles", errors);
    let expected_stressors: BTreeSet<String> =
        STRESSORS.iter().map(|value| (*value).to_owned()).collect();
    if stressor_ids != expected_stressors {
        errors.push("stress_profiles must contain exactly the eight frozen stressors including combined-shock cases".to_owned());
    }

    let expected_architecture_values: [(&str, f64, f64); 3] = [
        ("single_vehicle_baseline", 0.0, 0.0),
        ("single_vehicle_repairability_oriented", 0.15, 7.0),
        ("household_mobility_bundle", 0.55, 7.0),
    ];
    if let Some(items) = corpus.get("architecture_profiles").and_then(Value::as_array) {
        for profile in items {
            let Some(id) = text_at(profile, "id") else { continue };
            if let Some((_, fallback, coverage)) =
                expected_architecture_values.iter().find(|(expected, _, _)| *expected == id)
            {
                check_number(
                    profile.get("fallback_capacity_fraction").unwrap_or(&Value::Null),
                    &format!("architecture_profiles[{id}].fallback_capacity_fraction"),
                    *fallback,
                    errors,
                );
                check_number(
                    profile.get("stocked_critical_part_coverage_days").unwrap_or(&Value::Null),
                    &format!("architecture_profiles[{id}].stocked_critical_part_coverage_days"),
                    *coverage,
                    errors,
                );
                if text_at(profile, "parameter_status")
                    != Some("synthetic_fixture_value_not_observed")
                {
                    errors.push(format!("architecture_profiles[{id}]: values must remain marked synthetic"));
                }
            }
        }
    }

    let expected_stress_values: [(&str, f64, f64, f64, f64); 8] = [
        ("nominal", 0.0, 1.0, 1.0, 1.0),
        ("repair_delay", 21.0, 1.0, 1.0, 1.0),
        ("critical_part_unavailability", 35.0, 0.0, 1.0, 1.0),
        ("fuel_price_shock", 0.0, 1.0, 1.5, 1.0),
        ("cashflow_contraction", 0.0, 1.0, 1.0, 0.7),
        ("repair_delay_plus_fuel_price_shock", 21.0, 1.0, 1.5, 1.0),
        ("critical_part_plus_cashflow_contraction", 35.0, 0.0, 1.0, 0.7),
        ("combined_supply_fuel_cashflow_shock", 35.0, 0.0, 1.5, 0.7),
    ];
    let mut stress_values = BTreeMap::new();
    if let Some(items) = corpus.get("stress_profiles").and_then(Value::as_array) {
        for profile in items {
            let Some(id) = text_at(profile, "id") else { continue };
            if let Some((_, delay, parts, fuel, budget)) =
                expected_stress_values.iter().find(|(expected, _, _, _, _)| *expected == id)
            {
                check_number(profile.get("repair_delay_days").unwrap_or(&Value::Null), &format!("stress_profiles[{id}].repair_delay_days"), *delay, errors);
                check_number(profile.get("critical_part_fill_fraction").unwrap_or(&Value::Null), &format!("stress_profiles[{id}].critical_part_fill_fraction"), *parts, errors);
                check_number(profile.get("fuel_cost_multiplier").unwrap_or(&Value::Null), &format!("stress_profiles[{id}].fuel_cost_multiplier"), *fuel, errors);
                check_number(profile.get("household_budget_retention_fraction").unwrap_or(&Value::Null), &format!("stress_profiles[{id}].household_budget_retention_fraction"), *budget, errors);
                if text_at(profile, "parameter_status") != Some("synthetic_fixture_value_not_observed") {
                    errors.push(format!("stress_profiles[{id}]: values must remain marked synthetic"));
                }
                stress_values.insert(id.to_owned(), (*delay, *parts, *fuel, *budget));
            }
        }
    }

    if let Some(demand) = corpus.get("demand_profile") {
        check_number(demand.get("essential_trips_per_week").unwrap_or(&Value::Null), "demand_profile.essential_trips_per_week", 12.0, errors);
        check_number(demand.get("discretionary_trips_per_week").unwrap_or(&Value::Null), "demand_profile.discretionary_trips_per_week", 8.0, errors);
        if text_at(demand, "parameter_status") != Some("normalized_synthetic_workload_not_a_population_estimate") {
            errors.push("demand_profile must be labelled a normalized synthetic workload".to_owned());
        }
    } else {
        errors.push("demand_profile: missing object".to_owned());
    }

    let mut scenario_ids = BTreeSet::new();
    let mut pairs = BTreeSet::new();
    let mut scenario_count = 0usize;
    match corpus.get("scenarios").and_then(Value::as_array) {
        Some(scenarios) => {
            scenario_count = scenarios.len();
            if scenarios.len() != ARCHITECTURES.len() * STRESSORS.len() {
                errors.push(format!(
                    "scenarios: expected complete {} x {} matrix ({} cases), got {}",
                    ARCHITECTURES.len(), STRESSORS.len(),
                    ARCHITECTURES.len() * STRESSORS.len(), scenarios.len()
                ));
            }
            for (index, scenario) in scenarios.iter().enumerate() {
                let id = text_at(scenario, "scenario_id").unwrap_or("");
                let architecture = text_at(scenario, "architecture_id").unwrap_or("");
                let stressor = text_at(scenario, "stressor_id").unwrap_or("");
                if id.is_empty() || !scenario_ids.insert(id.to_owned()) {
                    errors.push(format!("scenarios[{index}]: missing or duplicate scenario_id"));
                }
                if !expected_architectures.contains(architecture) {
                    errors.push(format!("scenarios[{index}]: unknown architecture_id {architecture}"));
                }
                if !expected_stressors.contains(stressor) {
                    errors.push(format!("scenarios[{index}]: unknown stressor_id {stressor}"));
                }
                if !pairs.insert((architecture.to_owned(), stressor.to_owned())) {
                    errors.push(format!("scenarios[{index}]: duplicate architecture/stressor pair"));
                }
                if text_at(scenario, "evidence_class") != Some("synthetic_scenario_input") {
                    errors.push(format!("scenarios[{index}]: evidence must remain synthetic_scenario_input"));
                }
                if scenario.get("reported_outputs").is_none_or(|result| !result.is_null()) {
                    errors.push(format!("scenarios[{index}]: v0.1 fixture must not contain reported/claimed outputs"));
                }
                let scenario_safety = scenario.get("safety_policy");
                for (field, expected) in [
                    ("active_safety_hold_action", "deny_dispatch"),
                    ("unqualified_configuration_action", "deny_dispatch"),
                ] {
                    if scenario_safety.and_then(|value| text_at(value, field)) != Some(expected) {
                        errors.push(format!("scenarios[{index}].safety_policy.{field} must remain fail-closed"));
                    }
                }
                if let Some(expected) = stress_values.get(stressor) {
                    let parameters = scenario.get("stress_parameters");
                    let numbers = [
                        ("repair_delay_days", expected.0),
                        ("critical_part_fill_fraction", expected.1),
                        ("fuel_cost_multiplier", expected.2),
                        ("household_budget_retention_fraction", expected.3),
                    ];
                    for (key, value) in numbers {
                        let actual = parameters.and_then(|item| item.get(key)).unwrap_or(&Value::Null);
                        check_number(actual, &format!("scenarios[{index}].stress_parameters.{key}"), value, errors);
                    }
                }
            }
        }
        None => errors.push("scenarios: expected array".to_owned()),
    }

    let mut expected_pairs = BTreeSet::new();
    for architecture in ARCHITECTURES {
        for stressor in STRESSORS {
            expected_pairs.insert((architecture.to_owned(), stressor.to_owned()));
        }
    }
    if pairs != expected_pairs {
        errors.push("scenarios must cover each architecture/stressor pair exactly once".to_owned());
    }

    let mut control_ids = BTreeSet::new();
    let mut actual_controls = BTreeSet::new();
    let mut control_count = 0usize;
    match corpus.get("negative_controls").and_then(Value::as_array) {
        Some(controls) => {
            control_count = controls.len();
            if controls.len() != NEGATIVE_CONTROLS.len() {
                errors.push(format!("negative_controls: expected {} cases, got {}", NEGATIVE_CONTROLS.len(), controls.len()));
            }
            for (index, control) in controls.iter().enumerate() {
                let id = text_at(control, "id").unwrap_or("");
                let pattern = text_at(control, "anti_pattern").unwrap_or("");
                if !control_ids.insert(id.to_owned()) {
                    errors.push(format!("negative_controls[{index}]: duplicate/missing id"));
                }
                if control.get("must_reject").and_then(Value::as_bool) != Some(true) {
                    errors.push(format!("negative_controls[{index}]: every anti-pattern must be rejected"));
                }
                if text_at(control, "evidence_class") != Some("negative_control") {
                    errors.push(format!("negative_controls[{index}]: wrong evidence class"));
                }
                if !detects_anti_pattern(control) {
                    errors.push(format!(
                        "negative_controls[{index}]: declared anti-pattern is not detected by the v0.1 policy guard"
                    ));
                }
                actual_controls.insert((id.to_owned(), pattern.to_owned()));
            }
        }
        None => errors.push("negative_controls: expected array".to_owned()),
    }
    let expected_controls: BTreeSet<(String, String)> = NEGATIVE_CONTROLS
        .iter()
        .map(|(id, pattern)| ((*id).to_owned(), (*pattern).to_owned()))
        .collect();
    if actual_controls != expected_controls {
        errors.push("negative_controls do not match the five frozen v0.1 anti-patterns".to_owned());
    }

    if let Some(policy) = corpus.get("dispatch_policy") {
        if text_at(policy, "active_safety_hold_action") != Some("deny_dispatch")
            || text_at(policy, "unqualified_configuration_action") != Some("deny_dispatch")
        {
            errors.push("dispatch_policy must deny dispatch on safety hold and unqualified configuration".to_owned());
        }
        if text_at(policy, "fallback_counting_rule")
            != Some("count_only_when_legal_accessible_available_and_capacity_sufficient")
        {
            errors.push("dispatch_policy fallback counting rule drifted".to_owned());
        }
    } else {
        errors.push("dispatch_policy: missing object".to_owned());
    }

    let evidence_boundary = corpus.get("evidence_boundary");
    if evidence_boundary.and_then(|value| value.get("all_scenario_inputs_synthetic")).and_then(Value::as_bool) != Some(true)
        || evidence_boundary.and_then(|value| value.get("observed_outcomes_present")).and_then(Value::as_bool) != Some(false)
        || evidence_boundary.and_then(|value| value.get("safety_or_regulatory_approval_implied")).and_then(Value::as_bool) != Some(false)
    {
        errors.push("evidence_boundary must not promote synthetic inputs to observed outcomes or approval".to_owned());
    }

    AuditSummary {
        scenario_count,
        architecture_count: architecture_ids.len(),
        stressor_count: stressor_ids.len(),
        negative_control_count: control_count,
    }
}

fn audit(schema: &Value, corpus: &Value) -> Result<AuditSummary, Errors> {
    let validator = schema_validator(schema).map_err(|error| vec![error])?;
    let mut errors = Vec::new();
    for error in validator.iter_errors(corpus) {
        errors.push(format!("schema validation: {error}"));
    }
    let summary = validate_semantics(corpus, &mut errors);
    if errors.is_empty() {
        Ok(summary)
    } else {
        Err(errors)
    }
}

fn sha256(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn main() -> ExitCode {
    let mut schema_path = PathBuf::from("schemas/transport/mobility_continuity_scenario_v0_1.schema.json");
    let mut corpus_path = PathBuf::from("fixtures/personal_vehicle/mobility_continuity_scenarios_v0_1.json");
    let args: Vec<String> = env::args().skip(1).collect();
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--schema" if index + 1 < args.len() => {
                schema_path = PathBuf::from(&args[index + 1]);
                index += 2;
            }
            "--corpus" if index + 1 < args.len() => {
                corpus_path = PathBuf::from(&args[index + 1]);
                index += 2;
            }
            other => {
                eprintln!("unknown or incomplete argument: {other}\nusage: mobility-continuity-audit [--schema PATH] [--corpus PATH]");
                return ExitCode::from(2);
            }
        }
    }

    let schema_bytes = match fs::read(&schema_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read schema {}: {error}", schema_path.display());
            return ExitCode::FAILURE;
        }
    };
    let corpus_bytes = match fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read corpus {}: {error}", corpus_path.display());
            return ExitCode::FAILURE;
        }
    };
    let schema: Value = match serde_json::from_slice(&schema_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("schema is not valid JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    let corpus: Value = match serde_json::from_slice(&corpus_bytes) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("corpus is not valid JSON: {error}");
            return ExitCode::FAILURE;
        }
    };

    match audit(&schema, &corpus) {
        Ok(summary) => {
            println!("mobility-continuity benchmark contract: PASS");
            println!("schema_sha256={}", sha256(&schema_bytes));
            println!("corpus_sha256={}", sha256(&corpus_bytes));
            println!("architectures={}", summary.architecture_count);
            println!("stressors={}", summary.stressor_count);
            println!("scenario_cases={}", summary.scenario_count);
            println!("negative_controls={}", summary.negative_control_count);
            println!("evidence_ceiling=synthetic_contract_only");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("mobility-continuity benchmark contract: FAIL");
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
    use serde_json::json;

    fn frozen_values() -> (Value, Value) {
        let schema: Value = serde_json::from_str(include_str!(
            "../../../schemas/transport/mobility_continuity_scenario_v0_1.schema.json"
        ))
        .expect("schema fixture parses");
        let corpus: Value = serde_json::from_str(include_str!(
            "../../../fixtures/personal_vehicle/mobility_continuity_scenarios_v0_1.json"
        ))
        .expect("corpus fixture parses");
        (schema, corpus)
    }

    #[test]
    fn accepts_frozen_synthetic_matrix() {
        let (schema, corpus) = frozen_values();
        let result = audit(&schema, &corpus).expect("frozen contract should pass");
        assert_eq!(result.scenario_count, 24);
        assert_eq!(result.architecture_count, 3);
        assert_eq!(result.stressor_count, 8);
        assert_eq!(result.negative_control_count, 5);
    }

    #[test]
    fn rejects_missing_matrix_pair() {
        let (schema, mut corpus) = frozen_values();
        corpus["scenarios"].as_array_mut().expect("array").pop();
        assert!(audit(&schema, &corpus).is_err());
    }

    #[test]
    fn rejects_duplicate_scenario_id() {
        let (schema, mut corpus) = frozen_values();
        let first = corpus["scenarios"][0]["scenario_id"].clone();
        corpus["scenarios"][1]["scenario_id"] = first;
        assert!(audit(&schema, &corpus).is_err());
    }

    #[test]
    fn rejects_safety_hold_bypass() {
        let (schema, mut corpus) = frozen_values();
        corpus["dispatch_policy"]["active_safety_hold_action"] = json!("allow_dispatch");
        assert!(audit(&schema, &corpus).is_err());
    }

    #[test]
    fn rejects_synthetic_to_observed_promotion() {
        let (schema, mut corpus) = frozen_values();
        corpus["scenarios"][0]["evidence_class"] = json!("observed_field_measurement");
        assert!(audit(&schema, &corpus).is_err());
    }

    #[test]
    fn rejects_claimed_outputs_in_frozen_fixture() {
        let (schema, mut corpus) = frozen_values();
        corpus["scenarios"][0]["reported_outputs"] = json!({"essential_trip_fulfillment_ratio": 1.0});
        assert!(audit(&schema, &corpus).is_err());
    }

    #[test]
    fn rejects_negative_control_false_accept() {
        let (schema, mut corpus) = frozen_values();
        corpus["negative_controls"][0]["must_reject"] = json!(false);
        assert!(audit(&schema, &corpus).is_err());
    }

    #[test]
    fn rejects_synthetic_stressor_drift() {
        let (schema, mut corpus) = frozen_values();
        corpus["stress_profiles"][3]["fuel_cost_multiplier"] = json!(1.25);
        assert!(audit(&schema, &corpus).is_err());
    }

    #[test]
    fn executes_all_five_negative_control_guards() {
        let (_, corpus) = frozen_values();
        let controls = corpus["negative_controls"].as_array().expect("array");
        assert_eq!(controls.len(), 5);
        for control in controls {
            assert!(detects_anti_pattern(control), "control did not trigger: {}", text_at(control, "id").unwrap_or("unknown"));
        }
    }

    #[test]
    fn rejects_control_mutation_that_removes_the_violation() {
        let (schema, mut corpus) = frozen_values();
        let control = corpus["negative_controls"]
            .as_array_mut()
            .expect("array")
            .iter_mut()
            .find(|control| text_at(control, "id") == Some("NC-01"))
            .expect("NC-01 exists");
        control["adversarial_input"]["dispatch_allowed"] = json!(false);
        assert!(audit(&schema, &corpus).is_err());
    }
}
