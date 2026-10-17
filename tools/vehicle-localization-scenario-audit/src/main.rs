use jsonschema::draft202012;
use jsonschema::Validator;
use serde_json::Value;
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

const SCHEMA_ID: &str = "urn:luminous-dynamics:mobility:south-africa-vehicle-localization-matrix:0.1";
const MATRIX_VERSION: &str = "south-africa-vehicle-localization-scenario-matrix/0.1";
const REQUIRED_ROUTES: [&str; 3] = [
    "import_complete_vehicle",
    "import_qualified_modules_local_service",
    "local_value_add_conditional",
];
const REQUIRED_VOLUMES: [(&str, u64); 3] = [
    ("pilot_250", 250),
    ("small_series_2500", 2_500),
    ("scale_25000", 25_000),
];
const REQUIRED_CONDITIONS: [&str; 2] = ["stable_supply", "supply_fx_stress"];
const REQUIRED_NEGATIVE_CONTROLS: [&str; 5] = [
    "LC01_UNCONFIRMED_INCENTIVE_AS_ZERO_COST",
    "LC02_UNQUALIFIED_SAFETY_CRITICAL_LOCAL_SUBSTITUTE",
    "LC03_SKD_ANNOUNCEMENT_TREATED_AS_COMPLETE_LEGAL_RULE",
    "LC04_ADVERTISED_FROM_PRICE_AS_LANDED_COST",
    "LC05_VOLUME_TIER_AS_FORECAST",
];

type Errors = Vec<String>;
type AuditResult<T> = Result<T, Errors>;

#[derive(Debug, PartialEq, Eq)]
struct AuditSummary {
    scenario_count: usize,
    route_count: usize,
    volume_tier_count: usize,
    supply_condition_count: usize,
    negative_control_count: usize,
    schema_sha256: String,
    matrix_sha256: String,
}

fn text_at<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn array_at<'a>(value: &'a Value, key: &str) -> Option<&'a Vec<Value>> {
    value.get(key).and_then(Value::as_array)
}

fn exact_string_set(items: &[Value], at: &str, errors: &mut Errors) -> BTreeSet<String> {
    let mut seen = BTreeSet::new();
    for (index, item) in items.iter().enumerate() {
        match item.as_str() {
            Some(value) if !value.trim().is_empty() => {
                if !seen.insert(value.to_owned()) {
                    errors.push(format!("{at}[{index}]: duplicate value {value}"));
                }
            }
            _ => errors.push(format!("{at}[{index}]: expected non-empty string")),
        }
    }
    seen
}

fn close_to(actual: Option<f64>, expected: f64) -> bool {
    actual.is_some_and(|value| (value - expected).abs() < 1.0e-9)
}

fn schema_validator(schema: &Value) -> Result<Validator, String> {
    if text_at(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema") {
        return Err("schema must declare JSON Schema Draft 2020-12".to_owned());
    }
    if text_at(schema, "$id") != Some(SCHEMA_ID) {
        return Err(format!("schema $id must be {SCHEMA_ID}"));
    }
    if !draft202012::meta::is_valid(schema) {
        return Err("schema does not validate against Draft 2020-12 meta-schema".to_owned());
    }
    draft202012::options()
        .should_validate_formats(true)
        .build(schema)
        .map_err(|error| format!("cannot compile localization schema: {error}"))
}

fn schema_errors(validator: &Validator, instance: &Value) -> Vec<String> {
    validator.iter_errors(instance).map(|error| error.to_string()).collect()
}

fn validate_matrix(
    schema: &Value,
    matrix: &Value,
    registry: &Value,
    schema_raw: &[u8],
    matrix_raw: &[u8],
) -> AuditResult<AuditSummary> {
    let mut errors = Vec::new();
    let validator = match schema_validator(schema) {
        Ok(validator) => Some(validator),
        Err(error) => {
            errors.push(error);
            None
        }
    };
    if let Some(validator) = validator.as_ref() {
        errors.extend(
            schema_errors(validator, matrix)
                .into_iter()
                .map(|error| format!("JSON Schema instance: {error}")),
        );
    }

    if text_at(matrix, "schema_version") != Some(MATRIX_VERSION) {
        errors.push("matrix.schema_version mismatch".to_owned());
    }
    if text_at(matrix, "matrix_id") != Some("mobility-localization-sa-v0.1") {
        errors.push("matrix_id mismatch".to_owned());
    }
    if text_at(matrix, "status")
        != Some("synthetic scenario matrix for engineering/economic sensitivity only; no costed route or approval")
    {
        errors.push("status must preserve synthetic/no-approval boundary".to_owned());
    }
    if text_at(matrix, "market") != Some("South Africa") {
        errors.push("matrix market must be South Africa".to_owned());
    }

    let source_ids: BTreeSet<String> = registry
        .get("sources")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(|source| text_at(source, "id").map(str::to_owned))
        .collect();
    let required_policy_sources: BTreeSet<String> = [
        "SRC_SA_DTIC_SKD_POLICY_2026_10".to_owned(),
        "SRC_SA_SARS_APDP2".to_owned(),
    ]
    .into_iter()
    .collect();
    if !required_policy_sources.is_subset(&source_ids) {
        errors.push("source registry is missing a required primary policy source".to_owned());
    }
    let provenance_sources = array_at(
        matrix.get("provenance").unwrap_or(&Value::Null),
        "policy_source_ids",
    )
    .cloned()
    .unwrap_or_default();
    let provenance_set = exact_string_set(&provenance_sources, "provenance.policy_source_ids", &mut errors);
    if provenance_set != required_policy_sources {
        errors.push("matrix provenance must reference exactly the dtic SKD and SARS APDP2 sources".to_owned());
    }
    if !provenance_set.is_subset(&source_ids) {
        errors.push("matrix provenance contains an unregistered policy source".to_owned());
    }

    let Some(routes) = array_at(matrix, "route_definitions") else {
        return Err(vec!["route_definitions must be an array".to_owned()]);
    };
    if routes.len() != REQUIRED_ROUTES.len() {
        errors.push(format!("expected {} routes, found {}", REQUIRED_ROUTES.len(), routes.len()));
    }
    let mut route_map = BTreeMap::<String, Value>::new();
    for (index, route) in routes.iter().enumerate() {
        let at = format!("route_definitions[{index}]");
        let Some(id) = text_at(route, "id") else {
            errors.push(format!("{at}.id missing"));
            continue;
        };
        if !REQUIRED_ROUTES.contains(&id) {
            errors.push(format!("{at}: unregistered route {id}"));
        }
        if route_map.insert(id.to_owned(), route.clone()).is_some() {
            errors.push(format!("{at}: duplicate route {id}"));
        }
        let policy_refs = array_at(route, "policy_source_ids").cloned().unwrap_or_default();
        let policy_set = exact_string_set(&policy_refs, &format!("{at}.policy_source_ids"), &mut errors);
        if policy_set != required_policy_sources {
            errors.push(format!("{at}: route must retain both primary policy-source references"));
        }
        if !policy_set.is_subset(&source_ids) {
            errors.push(format!("{at}: route references a source absent from the source registry"));
        }

        let gate = text_at(route, "route_gate").unwrap_or("");
        let status = text_at(route, "localization_status").unwrap_or("");
        match id {
            "import_complete_vehicle" => {
                if gate != "market_baseline_only" || status != "not_a_local_manufacturing_case" {
                    errors.push("complete-vehicle import must remain a baseline comparator, not local production".to_owned());
                }
            }
            "import_qualified_modules_local_service" => {
                if gate != "requires_module_and_supplier_qualification"
                    || status != "conditional_engineering_route_not_approved"
                {
                    errors.push("module/local-service route must remain conditional on engineering/supplier qualification".to_owned());
                }
            }
            "local_value_add_conditional" => {
                if gate != "blocked_until_written_authority_confirmation"
                    || status != "blocked_pending_project_specific_eligibility"
                {
                    errors.push("local value-add/CKD-style route must remain blocked pending written authority confirmation".to_owned());
                }
            }
            _ => {}
        }
    }
    for id in REQUIRED_ROUTES {
        if !route_map.contains_key(id) {
            errors.push(format!("missing required route {id}"));
        }
    }

    let Some(volumes) = array_at(matrix, "volume_tiers") else {
        return Err(vec!["volume_tiers must be an array".to_owned()]);
    };
    if volumes.len() != REQUIRED_VOLUMES.len() {
        errors.push(format!("expected {} volume tiers, found {}", REQUIRED_VOLUMES.len(), volumes.len()));
    }
    let mut volume_map = BTreeMap::<String, u64>::new();
    for (index, tier) in volumes.iter().enumerate() {
        let at = format!("volume_tiers[{index}]");
        let Some(id) = text_at(tier, "id") else {
            errors.push(format!("{at}.id missing"));
            continue;
        };
        let count = tier.get("vehicles_per_year").and_then(Value::as_u64);
        if volume_map.contains_key(id) {
            errors.push(format!("{at}: duplicate volume tier {id}"));
        }
        if let Some(count) = count {
            volume_map.insert(id.to_owned(), count);
        } else {
            errors.push(format!("{at}.vehicles_per_year must be a positive integer"));
        }
        match REQUIRED_VOLUMES.iter().find(|(required_id, _)| *required_id == id) {
            Some((_, expected)) if count == Some(*expected) => {}
            Some((_, expected)) => errors.push(format!("{at}: expected {expected} vehicles/year for {id}")),
            None => errors.push(format!("{at}: unexpected volume tier {id}")),
        }
        if !text_at(tier, "note").is_some_and(|note| note.contains("not a production forecast")) {
            errors.push(format!("{at}: volume tier must be labelled as a synthetic sensitivity, not forecast"));
        }
    }

    let Some(conditions) = array_at(matrix, "supply_conditions") else {
        return Err(vec!["supply_conditions must be an array".to_owned()]);
    };
    if conditions.len() != REQUIRED_CONDITIONS.len() {
        errors.push(format!("expected {} supply conditions, found {}", REQUIRED_CONDITIONS.len(), conditions.len()));
    }
    let expected_shocks: BTreeMap<&str, (f64, f64, f64, f64)> = BTreeMap::from([
        ("stable_supply", (1.0, 1.0, 0.98, 1.0)),
        ("supply_fx_stress", (1.2, 2.0, 0.85, 0.7)),
    ]);
    let mut condition_map = BTreeMap::<String, (f64, f64, f64, f64)>::new();
    for (index, condition) in conditions.iter().enumerate() {
        let at = format!("supply_conditions[{index}]");
        let Some(id) = text_at(condition, "id") else {
            errors.push(format!("{at}.id missing"));
            continue;
        };
        let values = (
            condition.get("fx_multiplier").and_then(Value::as_f64),
            condition.get("imported_part_lead_time_multiplier").and_then(Value::as_f64),
            condition.get("available_supplier_fraction").and_then(Value::as_f64),
            condition.get("demand_multiplier").and_then(Value::as_f64),
        );
        let (Some(fx), Some(lead), Some(availability), Some(demand)) = values else {
            errors.push(format!("{at}: shock fields must be numbers"));
            continue;
        };
        if condition_map.insert(id.to_owned(), (fx, lead, availability, demand)).is_some() {
            errors.push(format!("{at}: duplicate condition {id}"));
        }
        match expected_shocks.get(id) {
            Some((efx, elead, eavailability, edemand))
                if close_to(Some(fx), *efx)
                    && close_to(Some(lead), *elead)
                    && close_to(Some(availability), *eavailability)
                    && close_to(Some(demand), *edemand) => {}
            Some(_) => errors.push(format!("{at}: synthetic sensitivity parameters changed without explicit revision")),
            None => errors.push(format!("{at}: unknown supply condition {id}")),
        }
        if text_at(condition, "status") != Some("illustrative_sensitivity_input_not_observation") {
            errors.push(format!("{at}: condition must not be mislabeled as market observation"));
        }
    }

    if matrix.get("scenario_count_expected").and_then(Value::as_u64) != Some(18) {
        errors.push("scenario_count_expected must be 18".to_owned());
    }
    let Some(scenarios) = array_at(matrix, "scenarios") else {
        return Err(vec!["scenarios must be an array".to_owned()]);
    };
    if scenarios.len() != 18 {
        errors.push(format!("expected 18 route × volume × supply scenarios, found {}", scenarios.len()));
    }
    let mut seen_scenarios = BTreeSet::new();
    let mut seen_combinations = BTreeSet::new();
    for (index, scenario) in scenarios.iter().enumerate() {
        let at = format!("scenarios[{index}]");
        let Some(id) = text_at(scenario, "id") else {
            errors.push(format!("{at}.id missing"));
            continue;
        };
        if !seen_scenarios.insert(id.to_owned()) {
            errors.push(format!("{at}: duplicate scenario ID {id}"));
        }
        let route_id = text_at(scenario, "route_id").unwrap_or("");
        let volume_id = text_at(scenario, "volume_tier_id").unwrap_or("");
        let condition_id = text_at(scenario, "supply_condition_id").unwrap_or("");
        let combination = (route_id.to_owned(), volume_id.to_owned(), condition_id.to_owned());
        if !seen_combinations.insert(combination.clone()) {
            errors.push(format!("{at}: duplicate route/volume/condition combination"));
        }
        let route = route_map.get(route_id);
        let Some(route) = route else {
            errors.push(format!("{at}: route {route_id} is undefined"));
            continue;
        };
        let volume = volume_map.get(volume_id);
        if volume.is_none() {
            errors.push(format!("{at}: volume tier {volume_id} is undefined"));
        }
        let condition = condition_map.get(condition_id);
        if condition.is_none() {
            errors.push(format!("{at}: supply condition {condition_id} is undefined"));
        }
        let expected_id = format!(
            "LOC_{}_{}_{}",
            route_id.to_uppercase(),
            volume_id.to_uppercase(),
            condition_id.to_uppercase()
        );
        if id != expected_id {
            errors.push(format!("{at}: expected stable ID {expected_id}"));
        }
        if text_at(scenario, "route_gate") != text_at(route, "route_gate")
            || text_at(scenario, "localization_status") != text_at(route, "localization_status")
        {
            errors.push(format!("{at}: route gate/status differs from route definition"));
        }
        if volume.is_some_and(|value| scenario.get("vehicles_per_year").and_then(Value::as_u64) != Some(*value)) {
            errors.push(format!("{at}: vehicle volume disagrees with tier definition"));
        }
        if !text_at(scenario, "cost_model_status")
            .is_some_and(|value| value == "not_computed_missing_dated_quotes_and_bom")
        {
            errors.push(format!("{at}: model cost status must remain unresolved until dated BOM/quotes exist"));
        }
        if let Some(costs) = scenario.get("costs").and_then(Value::as_object) {
            for (cost_name, value) in costs {
                if !value.is_null() {
                    errors.push(format!("{at}.costs.{cost_name}: must remain null until source evidence supports a value"));
                }
            }
            if costs.get("incentive_or_rebate_zar").is_none_or(|value| !value.is_null()) {
                errors.push(format!("{at}: unconfirmed incentives/rebates must remain unresolved/null"));
            }
        } else {
            errors.push(format!("{at}.costs must be an object"));
        }
        let scenario_policy_refs = array_at(scenario, "policy_source_ids").cloned().unwrap_or_default();
        let scenario_policy_set = exact_string_set(&scenario_policy_refs, &format!("{at}.policy_source_ids"), &mut errors);
        let route_policy_refs = array_at(route, "policy_source_ids").cloned().unwrap_or_default();
        let route_policy_set = exact_string_set(&route_policy_refs, "route.policy_source_ids", &mut errors);
        if scenario_policy_set != route_policy_set {
            errors.push(format!("{at}: policy source set differs from route definition"));
        }
        if let Some((fx, lead, availability, demand)) = condition {
            let inputs = scenario.get("shock_inputs").unwrap_or(&Value::Null);
            if !close_to(inputs.get("foreign_exchange_cost_multiplier").and_then(Value::as_f64), *fx)
                || !close_to(inputs.get("imported_part_lead_time_multiplier").and_then(Value::as_f64), *lead)
                || !close_to(inputs.get("assumed_supplier_availability_fraction").and_then(Value::as_f64), *availability)
                || !close_to(inputs.get("annual_demand_multiplier").and_then(Value::as_f64), *demand)
                || text_at(inputs, "input_class") != Some("illustrative_sensitivity_input_not_observation")
            {
                errors.push(format!("{at}: shock input does not match its declared supply condition"));
            }
        }
        if route_id == "local_value_add_conditional"
            && text_at(scenario, "route_gate") != Some("blocked_until_written_authority_confirmation")
        {
            errors.push(format!("{at}: local value-add path must remain blocked pending written authority confirmation"));
        }
    }
    for route in REQUIRED_ROUTES {
        for (volume, _) in REQUIRED_VOLUMES {
            for condition in REQUIRED_CONDITIONS {
                let combination = (route.to_owned(), volume.to_owned(), condition.to_owned());
                if !seen_combinations.contains(&combination) {
                    errors.push(format!("missing scenario combination {route}/{volume}/{condition}"));
                }
            }
        }
    }

    let Some(controls) = array_at(matrix, "negative_controls") else {
        return Err(vec!["negative_controls must be an array".to_owned()]);
    };
    if controls.len() != REQUIRED_NEGATIVE_CONTROLS.len() {
        errors.push(format!("expected {} negative controls, found {}", REQUIRED_NEGATIVE_CONTROLS.len(), controls.len()));
    }
    let expected_control_outcomes = BTreeMap::from([
        ("LC01_UNCONFIRMED_INCENTIVE_AS_ZERO_COST", "reject_or_mark_blocked"),
        ("LC02_UNQUALIFIED_SAFETY_CRITICAL_LOCAL_SUBSTITUTE", "reject_route_candidate"),
        ("LC03_SKD_ANNOUNCEMENT_TREATED_AS_COMPLETE_LEGAL_RULE", "reject_eligibility_claim"),
        ("LC04_ADVERTISED_FROM_PRICE_AS_LANDED_COST", "reject_cost_claim"),
        ("LC05_VOLUME_TIER_AS_FORECAST", "reject_forecast_claim"),
    ]);
    let mut seen_controls = BTreeSet::new();
    for (index, control) in controls.iter().enumerate() {
        let at = format!("negative_controls[{index}]");
        let Some(id) = text_at(control, "id") else {
            errors.push(format!("{at}.id missing"));
            continue;
        };
        if !seen_controls.insert(id.to_owned()) {
            errors.push(format!("{at}: duplicate control ID {id}"));
        }
        if !REQUIRED_NEGATIVE_CONTROLS.contains(&id) {
            errors.push(format!("{at}: unexpected control {id}"));
        }
        let outcome = text_at(control, "expected").unwrap_or("");
        if expected_control_outcomes.get(id).copied() != Some(outcome) {
            errors.push(format!("{at}: negative control outcome weakened or changed"));
        }
        if !text_at(control, "invariant").is_some_and(|value| !value.trim().is_empty()) {
            errors.push(format!("{at}: invariant is required"));
        }
    }
    for id in REQUIRED_NEGATIVE_CONTROLS {
        if !seen_controls.contains(id) {
            errors.push(format!("missing negative control {id}"));
        }
    }

    let categories = array_at(matrix, "required_cost_categories").cloned().unwrap_or_default();
    let category_set = exact_string_set(&categories, "required_cost_categories", &mut errors);
    for required in [
        "BOM and dated supplier quote",
        "landed costs and forex basis",
        "tooling and capital expenditure",
        "homologation, testing and approval",
        "program incentives only after eligibility verified",
        "lifecycle downtime and end-of-life",
    ] {
        if !category_set.contains(required) {
            errors.push(format!("required cost category missing: {required}"));
        }
    }
    let claim_ceiling = array_at(matrix, "claim_ceiling").cloned().unwrap_or_default();
    if !claim_ceiling.iter().filter_map(Value::as_str).any(|value| value.contains("Null cost fields"))
        || !claim_ceiling.iter().filter_map(Value::as_str).any(|value| value.contains("not legal advice"))
    {
        errors.push("matrix claim ceiling must preserve unresolved-cost and non-legal-advice boundaries".to_owned());
    }

    if errors.is_empty() {
        Ok(AuditSummary {
            scenario_count: scenarios.len(),
            route_count: routes.len(),
            volume_tier_count: volumes.len(),
            supply_condition_count: conditions.len(),
            negative_control_count: controls.len(),
            schema_sha256: format!("{:x}", Sha256::digest(schema_raw)),
            matrix_sha256: format!("{:x}", Sha256::digest(matrix_raw)),
        })
    } else {
        Err(errors)
    }
}

fn default_schema_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../schemas/transport/localization_scenario_matrix_v0_1.schema.json")
}

fn default_matrix_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/localization_scenario_matrix_v0_1.json")
}

fn default_registry_path() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../fixtures/personal_vehicle/benchmark_source_registry_v0_1.json")
}

fn main() -> ExitCode {
    let mut schema_path = default_schema_path();
    let mut matrix_path = default_matrix_path();
    let mut registry_path = default_registry_path();
    let mut args = env::args_os().skip(1);
    while let Some(arg) = args.next() {
        if arg == "--help" || arg == "-h" {
            println!("Usage: vehicle-localization-scenario-audit [--schema PATH] [--matrix PATH] [--registry PATH]");
            println!("Validates scenario coverage and fail-closed gates; does not estimate vehicle cost or provide legal advice.");
            return ExitCode::SUCCESS;
        } else if arg == "--schema" {
            let Some(path) = args.next() else {
                eprintln!("LOCALIZATION_AUDIT_FAIL: --schema requires a path");
                return ExitCode::from(2);
            };
            schema_path = PathBuf::from(path);
        } else if arg == "--matrix" {
            let Some(path) = args.next() else {
                eprintln!("LOCALIZATION_AUDIT_FAIL: --matrix requires a path");
                return ExitCode::from(2);
            };
            matrix_path = PathBuf::from(path);
        } else if arg == "--registry" {
            let Some(path) = args.next() else {
                eprintln!("LOCALIZATION_AUDIT_FAIL: --registry requires a path");
                return ExitCode::from(2);
            };
            registry_path = PathBuf::from(path);
        } else {
            eprintln!("LOCALIZATION_AUDIT_FAIL: unknown argument {}", arg.to_string_lossy());
            return ExitCode::from(2);
        }
    }

    let load = |path: &PathBuf, label: &str| -> Result<(Vec<u8>, Value), String> {
        let bytes = fs::read(path).map_err(|error| format!("cannot read {}: {error}", path.display()))?;
        let value = serde_json::from_slice(&bytes).map_err(|error| format!("invalid {label} JSON: {error}"))?;
        Ok((bytes, value))
    };

    let (schema_raw, schema) = match load(&schema_path, "schema") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("LOCALIZATION_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };
    let (matrix_raw, matrix) = match load(&matrix_path, "scenario matrix") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("LOCALIZATION_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };
    let (registry_raw, registry) = match load(&registry_path, "source registry") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("LOCALIZATION_AUDIT_FAIL: {error}");
            return ExitCode::from(2);
        }
    };
    let _ = registry_raw;

    match validate_matrix(&schema, &matrix, &registry, &schema_raw, &matrix_raw) {
        Ok(summary) => {
            println!(
                "LOCALIZATION_AUDIT_PASS scenarios={} routes={} volume_tiers={} supply_conditions={} negative_controls={} schema_sha256={} matrix_sha256={}",
                summary.scenario_count,
                summary.route_count,
                summary.volume_tier_count,
                summary.supply_condition_count,
                summary.negative_control_count,
                summary.schema_sha256,
                summary.matrix_sha256
            );
            println!("CLAIM_CEILING synthetic route coverage only; no costing, local-content, eligibility, tax/incentive, safety or commercial conclusion");
            ExitCode::SUCCESS
        }
        Err(errors) => {
            eprintln!("LOCALIZATION_AUDIT_FAIL errors={}", errors.len());
            for error in errors {
                eprintln!("- {error}");
            }
            ExitCode::from(1)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn schema() -> Value {
        serde_json::from_str(include_str!("../../../schemas/transport/localization_scenario_matrix_v0_1.schema.json"))
            .expect("localization schema JSON")
    }

    fn matrix() -> Value {
        serde_json::from_str(include_str!("../../../fixtures/personal_vehicle/localization_scenario_matrix_v0_1.json"))
            .expect("localization scenario JSON")
    }

    fn registry() -> Value {
        serde_json::from_str(include_str!("../../../fixtures/personal_vehicle/benchmark_source_registry_v0_1.json"))
            .expect("source registry JSON")
    }

    fn validate_fixture(document: &Value) -> Result<AuditSummary, Vec<String>> {
        let schema = schema();
        let registry = registry();
        let schema_bytes = serde_json::to_vec(&schema).expect("schema serialization");
        let matrix_bytes = serde_json::to_vec(document).expect("matrix serialization");
        validate_matrix(&schema, document, &registry, &schema_bytes, &matrix_bytes)
    }

    #[test]
    fn frozen_matrix_schema_and_semantic_contract_validate() {
        let result = validate_fixture(&matrix()).expect("frozen matrix should satisfy its contract");
        assert_eq!(result.scenario_count, 18);
        assert_eq!(result.route_count, 3);
        assert_eq!(result.volume_tier_count, 3);
        assert_eq!(result.supply_condition_count, 2);
        assert_eq!(result.negative_control_count, 5);
        assert_eq!(result.schema_sha256.len(), 64);
        assert_eq!(result.matrix_sha256.len(), 64);
    }

    #[test]
    fn local_value_add_route_cannot_be_promoted_without_authority_confirmation() {
        let mut document = matrix();
        document["route_definitions"][2]["route_gate"] = Value::String("market_baseline_only".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn unconfirmed_incentive_cannot_be_coerced_to_zero() {
        let mut document = matrix();
        document["scenarios"][0]["costs"]["incentive_or_rebate_zar"] = Value::from(0);
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn stress_scenario_cannot_be_relabelled_as_an_observation() {
        let mut document = matrix();
        document["supply_conditions"][1]["status"] = Value::String("measured_market_observation".to_owned());
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn baseline_price_cannot_be_promoted_to_landed_cost() {
        let mut document = matrix();
        document["scenarios"][0]["costs"]["landed_import_cost_zar"] = Value::from(182400);
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn missing_route_volume_condition_combination_is_rejected() {
        let mut document = matrix();
        document["scenarios"].as_array_mut().expect("scenarios").pop();
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn duplicate_policy_source_reference_is_rejected() {
        let mut document = matrix();
        let first = document["provenance"]["policy_source_ids"][0].clone();
        document["provenance"]["policy_source_ids"][1] = first;
        assert!(validate_fixture(&document).is_err());
    }

    #[test]
    fn unqualified_safety_critical_localization_is_rejected_by_route_gate() {
        let mut document = matrix();
        document["route_definitions"][1]["route_gate"] = Value::String("unrestricted".to_owned());
        assert!(validate_fixture(&document).is_err());
    }
}
