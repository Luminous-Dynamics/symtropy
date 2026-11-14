// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only qualification of synthetic water-network observability evidence.
//! This evaluator checks state and sensor evidence contracts; it does not solve hydraulics.

use serde_json::{Value, json};
use std::collections::{BTreeMap, BTreeSet};

const CORPUS_ID: &str = "water-network-observability-qualification-v0.1";
const ORACLE_ID: &str = "water-network-observability-disposition-oracle-v0.1";
const EXPECTED_SCENARIOS: usize = 12;

const REQUIRED_STATES: [&str; 4] = [
    "tank_level_1",
    "tank_level_2",
    "junction_pressure_1",
    "junction_pressure_2",
];

const FAILURE_DOMAINS: [&str; 6] = [
    "physical_interface",
    "calibration_lineage",
    "power_supply",
    "communications_path",
    "clock_domain",
    "measurement_principle",
];

#[derive(Clone, Debug)]
struct Sensor {
    sensor_id: String,
    state_id: String,
    condition_state: String,
    binding_status: String,
    calibration_status: String,
    time_status: String,
    dependencies: Vec<(String, String)>,
}

#[derive(Clone, Debug)]
struct Estimator {
    status: String,
    qualification_claimed: bool,
    covered_state_ids: Vec<String>,
    estimator_ref: Option<String>,
    model_scope_ref: Option<String>,
    calibration_ref: Option<String>,
    heldout_ref: Option<String>,
    ood_test_ref: Option<String>,
    validity_domain_ref: Option<String>,
    interval_coverage: Option<f64>,
    calibration_sample_count: Option<u64>,
    max_interval_width: Option<f64>,
    condition_number_bound: Option<f64>,
}

fn get_str<'a>(value: &'a Value, key: &str) -> Option<&'a str> {
    value.get(key).and_then(Value::as_str)
}

fn required_str(
    value: &Value,
    key: &str,
    context: &str,
    errors: &mut Vec<String>,
) -> Option<String> {
    match get_str(value, key) {
        Some(text) if !text.trim().is_empty() => Some(text.to_owned()),
        _ => {
            errors.push(format!("{context}: missing or invalid string {key}"));
            None
        }
    }
}

fn finite(value: &Value, key: &str) -> Option<f64> {
    value
        .get(key)
        .and_then(Value::as_f64)
        .filter(|number| number.is_finite())
}

fn optional_number(
    value: &Value,
    key: &str,
    context: &str,
    errors: &mut Vec<String>,
) -> Option<f64> {
    match value.get(key) {
        None => {
            errors.push(format!("{context}: missing field {key}"));
            None
        }
        Some(item) if item.is_null() => None,
        Some(item) => match item.as_f64().filter(|number| number.is_finite()) {
            Some(number) => Some(number),
            None => {
                errors.push(format!("{context}: {key} must be finite or null"));
                None
            }
        },
    }
}

fn optional_u64(value: &Value, key: &str, context: &str, errors: &mut Vec<String>) -> Option<u64> {
    match value.get(key) {
        None => {
            errors.push(format!("{context}: missing field {key}"));
            None
        }
        Some(item) if item.is_null() => None,
        Some(item) => match item.as_u64() {
            Some(number) => Some(number),
            None => {
                errors.push(format!(
                    "{context}: {key} must be an unsigned integer or null"
                ));
                None
            }
        },
    }
}

fn optional_ref(
    value: &Value,
    key: &str,
    context: &str,
    errors: &mut Vec<String>,
) -> Option<String> {
    match value.get(key) {
        None => {
            errors.push(format!("{context}: missing field {key}"));
            None
        }
        Some(item) if item.is_null() => None,
        Some(item) => match item.as_str().filter(|text| !text.trim().is_empty()) {
            Some(text) => Some(text.to_owned()),
            None => {
                errors.push(format!(
                    "{context}: {key} must be a non-empty string or null"
                ));
                None
            }
        },
    }
}

fn string_array(value: &Value, key: &str, context: &str, errors: &mut Vec<String>) -> Vec<String> {
    let Some(items) = value.get(key).and_then(Value::as_array) else {
        errors.push(format!("{context}: {key} must be an array"));
        return Vec::new();
    };
    items
        .iter()
        .enumerate()
        .filter_map(|(index, item)| match item.as_str() {
            Some(text) if !text.trim().is_empty() => Some(text.to_owned()),
            _ => {
                errors.push(format!(
                    "{context}: {key}[{index}] must be a non-empty string"
                ));
                None
            }
        })
        .collect()
}

fn is_one_of(value: &str, choices: &[&str]) -> bool {
    choices.contains(&value)
}

fn parse_sensor(
    value: &Value,
    scenario_id: &str,
    required: &BTreeSet<String>,
    errors: &mut Vec<String>,
) -> Option<Sensor> {
    let context = format!("{scenario_id}/sensor");
    let sensor_id = required_str(value, "sensor_id", &context, errors)?;
    let state_id = required_str(value, "state_id", &context, errors)?;
    if !required.contains(&state_id) {
        errors.push(format!(
            "{context}/{sensor_id}: state_id {state_id} is not required by the profile"
        ));
    }

    let measurement_principle = required_str(value, "measurement_principle", &context, errors)?;
    if !is_one_of(
        &measurement_principle,
        &[
            "radar_level",
            "ultrasonic_level",
            "float_level_switch",
            "pressure_transducer",
            "fiber_optic_pressure",
        ],
    ) {
        errors.push(format!(
            "{context}/{sensor_id}: unsupported measurement principle"
        ));
    }

    let condition_state = required_str(value, "condition_state", &context, errors)?;
    if !is_one_of(
        &condition_state,
        &[
            "available",
            "failed",
            "stale",
            "clock_invalid",
            "calibration_invalid",
        ],
    ) {
        errors.push(format!(
            "{context}/{sensor_id}: unsupported condition_state"
        ));
    }
    let binding_status = required_str(value, "binding_status", &context, errors)?;
    if !is_one_of(&binding_status, &["verified", "unverified"]) {
        errors.push(format!("{context}/{sensor_id}: unsupported binding_status"));
    }
    let calibration_status = required_str(value, "calibration_status", &context, errors)?;
    if !is_one_of(&calibration_status, &["current", "expired", "unknown"]) {
        errors.push(format!(
            "{context}/{sensor_id}: unsupported calibration_status"
        ));
    }
    let time_status = required_str(value, "time_status", &context, errors)?;
    if !is_one_of(&time_status, &["synchronized", "unsynchronized", "unknown"]) {
        errors.push(format!("{context}/{sensor_id}: unsupported time_status"));
    }

    let dependencies = value.get("dependencies").and_then(Value::as_array);
    let Some(dependencies) = dependencies else {
        errors.push(format!(
            "{context}/{sensor_id}: dependencies must be an array"
        ));
        return None;
    };
    let mut domain_types = BTreeSet::new();
    let mut parsed_dependencies = Vec::new();
    for dependency in dependencies {
        let domain_type = required_str(dependency, "domain_type", &context, errors)?;
        let domain_id = required_str(dependency, "domain_id", &context, errors)?;
        if !FAILURE_DOMAINS.contains(&domain_type.as_str()) {
            errors.push(format!(
                "{context}/{sensor_id}: unsupported dependency domain {domain_type}"
            ));
        }
        if !domain_types.insert(domain_type.clone()) {
            errors.push(format!(
                "{context}/{sensor_id}: duplicate dependency domain type {domain_type}"
            ));
        }
        parsed_dependencies.push((domain_type, domain_id));
    }
    if domain_types.len() != FAILURE_DOMAINS.len()
        || FAILURE_DOMAINS
            .iter()
            .any(|domain| !domain_types.contains(*domain))
    {
        errors.push(format!(
            "{context}/{sensor_id}: each of the six failure-domain types must appear exactly once"
        ));
    }

    Some(Sensor {
        sensor_id,
        state_id,
        condition_state,
        binding_status,
        calibration_status,
        time_status,
        dependencies: parsed_dependencies,
    })
}

fn parse_estimator(value: &Value, context: &str, errors: &mut Vec<String>) -> Option<Estimator> {
    let status = required_str(value, "status", context, errors)?;
    if !is_one_of(
        &status,
        &[
            "not_used",
            "synthetic_qualified_contract",
            "ill_conditioned",
            "not_tested",
        ],
    ) {
        errors.push(format!("{context}: unsupported estimator status {status}"));
    }
    let qualification_claimed = value
        .get("qualification_claimed")
        .and_then(Value::as_bool)
        .unwrap_or_else(|| {
            errors.push(format!(
                "{context}: qualification_claimed must be a boolean"
            ));
            true
        });
    if qualification_claimed {
        errors.push(format!(
            "{context}: synthetic estimator cannot claim physical qualification"
        ));
    }

    let covered_state_ids = string_array(value, "covered_state_ids", context, errors);
    let covered_set: BTreeSet<String> = covered_state_ids.iter().cloned().collect();
    if covered_set.len() != covered_state_ids.len() {
        errors.push(format!("{context}: covered_state_ids contains duplicates"));
    }
    if status == "not_used" && !covered_state_ids.is_empty() {
        errors.push(format!(
            "{context}: not_used estimator must have an empty covered_state_ids set"
        ));
    }
    if status == "synthetic_qualified_contract" && covered_state_ids.is_empty() {
        errors.push(format!(
            "{context}: synthetic qualified estimator must name covered_state_ids"
        ));
    }

    let estimator_ref = optional_ref(value, "estimator_ref", context, errors);
    let model_scope_ref = optional_ref(value, "model_scope_ref", context, errors);
    let calibration_ref = optional_ref(value, "calibration_ref", context, errors);
    let heldout_ref = optional_ref(value, "heldout_ref", context, errors);
    let ood_test_ref = optional_ref(value, "ood_test_ref", context, errors);
    let validity_domain_ref = optional_ref(value, "validity_domain_ref", context, errors);
    let interval_coverage = optional_number(value, "interval_coverage", context, errors);
    let calibration_sample_count = optional_u64(value, "calibration_sample_count", context, errors);
    let max_interval_width = optional_number(value, "max_interval_width", context, errors);
    let condition_number_bound = optional_number(value, "condition_number_bound", context, errors);

    if status == "not_used" {
        let refs_absent = [
            estimator_ref.as_ref(),
            model_scope_ref.as_ref(),
            calibration_ref.as_ref(),
            heldout_ref.as_ref(),
            ood_test_ref.as_ref(),
            validity_domain_ref.as_ref(),
        ]
        .iter()
        .all(|item| item.is_none());
        let metrics_absent = interval_coverage.is_none()
            && calibration_sample_count.is_none()
            && max_interval_width.is_none()
            && condition_number_bound.is_none();
        if !refs_absent || !metrics_absent {
            errors.push(format!(
                "{context}: not_used estimator must have null references and metrics"
            ));
        }
    }

    Some(Estimator {
        status,
        qualification_claimed,
        covered_state_ids,
        estimator_ref,
        model_scope_ref,
        calibration_ref,
        heldout_ref,
        ood_test_ref,
        validity_domain_ref,
        interval_coverage,
        calibration_sample_count,
        max_interval_width,
        condition_number_bound,
    })
}

fn estimator_qualified_scope(
    estimator: &Estimator,
    profile: &Value,
    required_states: &BTreeSet<String>,
    context: &str,
    errors: &mut Vec<String>,
) -> BTreeSet<String> {
    if estimator.status != "synthetic_qualified_contract" {
        return BTreeSet::new();
    }

    let required_refs = [
        ("estimator_ref", estimator.estimator_ref.as_ref()),
        ("model_scope_ref", estimator.model_scope_ref.as_ref()),
        ("calibration_ref", estimator.calibration_ref.as_ref()),
        ("heldout_ref", estimator.heldout_ref.as_ref()),
        ("ood_test_ref", estimator.ood_test_ref.as_ref()),
        (
            "validity_domain_ref",
            estimator.validity_domain_ref.as_ref(),
        ),
    ];
    for (name, reference) in required_refs {
        if reference.is_none() {
            errors.push(format!(
                "{context}: synthetic qualified estimator is missing {name}"
            ));
        }
    }
    if estimator.qualification_claimed {
        errors.push(format!(
            "{context}: estimator qualification claim must remain false"
        ));
    }

    let covered: BTreeSet<String> = estimator.covered_state_ids.iter().cloned().collect();
    if covered.is_empty() || covered.iter().any(|state| !required_states.contains(state)) {
        errors.push(format!(
            "{context}: estimator covered_state_ids must be a non-empty subset of required states"
        ));
    }

    let minimum_coverage = finite(profile, "minimum_estimator_coverage").unwrap_or(f64::NAN);
    let maximum_coverage = finite(profile, "maximum_estimator_coverage").unwrap_or(f64::NAN);
    let minimum_samples = profile
        .get("minimum_calibration_samples")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let maximum_width = finite(profile, "maximum_interval_width").unwrap_or(f64::NAN);
    let maximum_condition = finite(profile, "maximum_condition_number").unwrap_or(f64::NAN);

    let valid = estimator
        .interval_coverage
        .is_some_and(|value| value >= minimum_coverage && value <= maximum_coverage)
        && estimator
            .calibration_sample_count
            .is_some_and(|value| value >= minimum_samples)
        && estimator
            .max_interval_width
            .is_some_and(|value| value > 0.0 && value <= maximum_width)
        && estimator
            .condition_number_bound
            .is_some_and(|value| value.is_finite() && value > 0.0 && value <= maximum_condition);

    if !valid {
        errors.push(format!(
            "{context}: synthetic_qualified_contract does not meet coverage/sample/width/conditioning gates"
        ));
    }
    let references_present = required_refs
        .iter()
        .all(|(_, reference)| reference.is_some());
    if valid && references_present && !estimator.qualification_claimed {
        covered
    } else {
        BTreeSet::new()
    }
}

fn topology_is_valid(topology: &Value, errors: &mut Vec<String>, context: &str) -> bool {
    let _topology_ref = required_str(topology, "topology_ref", context, errors);
    let topology_revision = required_str(topology, "topology_revision", context, errors);
    let topology_digest = required_str(topology, "topology_digest", context, errors);
    let certificate_revision =
        required_str(topology, "certificate_topology_revision", context, errors);
    let certificate_digest = required_str(topology, "certificate_topology_digest", context, errors);
    let certificate_status = required_str(topology, "certificate_status", context, errors);
    let mapping_status = required_str(topology, "state_mapping_status", context, errors);

    if !is_one_of(
        certificate_status.as_deref().unwrap_or(""),
        &[
            "verified_for_exact_topology",
            "revision_mismatch",
            "not_verified",
        ],
    ) {
        errors.push(format!("{context}: unsupported certificate_status"));
    }
    if !is_one_of(
        mapping_status.as_deref().unwrap_or(""),
        &["complete", "incomplete"],
    ) {
        errors.push(format!("{context}: unsupported state_mapping_status"));
    }

    let same_revision = topology_revision == certificate_revision;
    let same_digest = topology_digest == certificate_digest;
    match certificate_status.as_deref() {
        Some("verified_for_exact_topology") => {
            if !same_revision || !same_digest || mapping_status.as_deref() != Some("complete") {
                errors.push(format!(
                    "{context}: verified_for_exact_topology requires equal revision/digest and complete state mapping"
                ));
                false
            } else {
                true
            }
        }
        Some("revision_mismatch") => {
            if same_revision && same_digest {
                errors.push(format!(
                    "{context}: revision_mismatch requires a topology revision or digest difference"
                ));
            }
            false
        }
        _ => false,
    }
}

fn scenario_class_valid(value: &str) -> bool {
    [
        "nominal_independent_direct",
        "single_sensor_dropout",
        "shared_calibration_common_cause",
        "shared_power_domain_failure",
        "shared_gateway_communications_failure",
        "common_clock_domain_drift",
        "topology_revision_mismatch",
        "structural_full_numeric_ill_conditioned",
        "incomplete_critical_state_mapping",
        "independent_measurement_survives_common_cause",
        "qualified_estimator_declared_scope",
        "topology_certificate_missing",
    ]
    .contains(&value)
}

fn state_disposition(
    state_id: &str,
    sensors: &[Sensor],
    failed_domains: &BTreeSet<(String, String)>,
    topology_valid: bool,
    structural_status: &str,
    estimator_qualified_states: &BTreeSet<String>,
) -> (String, Vec<String>, Vec<String>, usize) {
    let state_sensors: Vec<&Sensor> = sensors
        .iter()
        .filter(|sensor| sensor.state_id == state_id)
        .collect();

    let mut surviving = Vec::new();
    let mut direct_sensor_ids = Vec::new();
    let mut failed_overlap_count = 0usize;
    for sensor in &state_sensors {
        direct_sensor_ids.push(sensor.sensor_id.clone());
        let has_failed_dependency = sensor
            .dependencies
            .iter()
            .any(|dependency| failed_domains.contains(dependency));
        if has_failed_dependency {
            failed_overlap_count += 1;
        }
        let survives = sensor.condition_state == "available"
            && sensor.binding_status == "verified"
            && sensor.calibration_status == "current"
            && sensor.time_status == "synchronized"
            && !has_failed_dependency;
        if survives {
            surviving.push(sensor.sensor_id.clone());
        }
    }

    if !surviving.is_empty() {
        return (
            "directly_observed_current".to_owned(),
            direct_sensor_ids,
            surviving,
            failed_overlap_count,
        );
    }

    let clock_invalid = state_sensors.iter().any(|sensor| {
        sensor.time_status != "synchronized"
            || sensor.condition_state == "clock_invalid"
            || sensor.dependencies.iter().any(|(domain_type, domain_id)| {
                domain_type == "clock_domain"
                    && failed_domains.contains(&(domain_type.clone(), domain_id.clone()))
            })
    });
    if clock_invalid {
        return (
            "indeterminate_evidence".to_owned(),
            direct_sensor_ids,
            surviving,
            failed_overlap_count,
        );
    }

    if !topology_valid {
        return (
            "topology_invalidates_certificate".to_owned(),
            direct_sensor_ids,
            surviving,
            failed_overlap_count,
        );
    }

    if structural_status == "full" {
        let disposition = if estimator_qualified_states.contains(state_id) {
            "estimator_qualified_for_declared_scope"
        } else {
            "structurally_observable_only"
        };
        return (
            disposition.to_owned(),
            direct_sensor_ids,
            surviving,
            failed_overlap_count,
        );
    }

    (
        "unobservable_under_declared_faults".to_owned(),
        direct_sensor_ids,
        surviving,
        failed_overlap_count,
    )
}

fn overall_disposition(state_dispositions: &[String], topology_valid: bool) -> String {
    if !topology_valid {
        return "topology_invalidates_certificate".to_owned();
    }
    if state_dispositions
        .iter()
        .any(|state| state == "indeterminate_evidence")
    {
        return "indeterminate_evidence".to_owned();
    }
    if state_dispositions
        .iter()
        .any(|state| state == "unobservable_under_declared_faults")
    {
        return "unobservable_under_declared_faults".to_owned();
    }
    if state_dispositions
        .iter()
        .any(|state| state == "structurally_observable_only")
    {
        return "structurally_observable_only".to_owned();
    }
    if state_dispositions
        .iter()
        .any(|state| state == "estimator_qualified_for_declared_scope")
    {
        return "estimator_qualified_all".to_owned();
    }
    "all_direct_observed".to_owned()
}

pub fn evaluate(scenarios: &Value, schema: &Value, oracle: &Value) -> Result<Value, Vec<String>> {
    let mut errors = Vec::new();

    if get_str(scenarios, "schema_version") != Some("0.1.0")
        || get_str(scenarios, "corpus_id") != Some(CORPUS_ID)
    {
        errors.push("scenario corpus identity/version mismatch".to_owned());
    }
    if get_str(oracle, "schema_version") != Some("0.1.0")
        || get_str(oracle, "oracle_id") != Some(ORACLE_ID)
        || get_str(oracle, "corpus_id") != Some(CORPUS_ID)
    {
        errors.push("oracle identity/version mismatch".to_owned());
    }
    if get_str(schema, "$schema") != Some("https://json-schema.org/draft/2020-12/schema")
        || get_str(schema, "type") != Some("object")
        || schema.get("additionalProperties").and_then(Value::as_bool) != Some(false)
    {
        errors.push("schema must declare a closed Draft 2020-12 object root".to_owned());
    }
    let schema_required: BTreeSet<&str> = schema
        .get("required")
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for key in [
        "schema_version",
        "corpus_id",
        "profile",
        "scenarios",
        "interpretation_rules",
        "claim_ceiling",
    ] {
        if !schema_required.contains(key) {
            errors.push(format!("schema root must require {key}"));
        }
    }
    let scenario_required: BTreeSet<&str> = schema
        .get("properties")
        .and_then(|value| value.get("scenarios"))
        .and_then(|value| value.get("items"))
        .and_then(|value| value.get("required"))
        .and_then(Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(Value::as_str)
        .collect();
    for key in [
        "scenario_id",
        "scenario_class",
        "required_state_ids",
        "topology",
        "sensors",
        "failed_domains",
        "structural_observability",
        "numerical_estimator",
        "note",
    ] {
        if !scenario_required.contains(key) {
            errors.push(format!("schema scenario item must require {key}"));
        }
    }

    let profile = scenarios.get("profile").unwrap_or(&Value::Null);
    if get_str(profile, "profile_id") != Some("SYNTHETIC-OBSERVABILITY-PROFILE-V0")
        || get_str(profile, "evidence_class") != Some("synthetic_design_profile")
        || profile.get("values_are_normative").and_then(Value::as_bool) != Some(false)
    {
        errors.push("profile identity/evidence/non-normative marker mismatch".to_owned());
    }
    let required_states = string_array(profile, "required_state_ids", "profile", &mut errors);
    let required_set: BTreeSet<String> = required_states.iter().cloned().collect();
    let canonical_states: BTreeSet<String> = REQUIRED_STATES
        .iter()
        .map(|state| (*state).to_owned())
        .collect();
    if required_set != canonical_states || required_states.len() != REQUIRED_STATES.len() {
        errors.push(
            "profile required_state_ids must equal the four frozen critical states".to_owned(),
        );
    }

    let minimum_coverage = finite(profile, "minimum_estimator_coverage");
    let maximum_coverage = finite(profile, "maximum_estimator_coverage");
    let minimum_samples = profile
        .get("minimum_calibration_samples")
        .and_then(Value::as_u64);
    let maximum_width = finite(profile, "maximum_interval_width");
    let maximum_condition = finite(profile, "maximum_condition_number");
    if !minimum_coverage.is_some_and(|value| (0.0..=1.0).contains(&value))
        || !maximum_coverage.is_some_and(|value| (0.0..=1.0).contains(&value))
        || !matches!((minimum_coverage, maximum_coverage), (Some(minimum), Some(maximum)) if minimum <= maximum)
        || !minimum_samples.is_some_and(|value| value > 0)
        || !maximum_width.is_some_and(|value| value > 0.0 && value <= 1.0)
        || !maximum_condition.is_some_and(|value| value > 0.0)
    {
        errors.push("invalid synthetic estimator thresholds in profile".to_owned());
    }

    let Some(cases) = scenarios.get("scenarios").and_then(Value::as_array) else {
        return Err(vec!["scenarios must be an array".to_owned()]);
    };
    let Some(oracle_rows) = oracle.get("results").and_then(Value::as_array) else {
        return Err(vec!["oracle results must be an array".to_owned()]);
    };
    if cases.len() != EXPECTED_SCENARIOS || oracle_rows.len() != EXPECTED_SCENARIOS {
        errors.push(format!(
            "expected {EXPECTED_SCENARIOS} scenarios and oracle rows, found {} and {}",
            cases.len(),
            oracle_rows.len()
        ));
    }

    let mut oracle_by_id: BTreeMap<String, &Value> = BTreeMap::new();
    for row in oracle_rows {
        let Some(id) = get_str(row, "scenario_id") else {
            errors.push("oracle row is missing scenario_id".to_owned());
            continue;
        };
        if oracle_by_id.insert(id.to_owned(), row).is_some() {
            errors.push(format!("duplicate oracle scenario_id {id}"));
        }
    }

    let mut scenario_ids = BTreeSet::new();
    let mut results = Vec::new();

    for case in cases {
        let Some(scenario_id) = get_str(case, "scenario_id") else {
            errors.push("scenario is missing scenario_id".to_owned());
            continue;
        };
        let context = scenario_id.to_owned();
        let class = get_str(case, "scenario_class").unwrap_or("");
        if !scenario_class_valid(class) {
            errors.push(format!("{context}: unsupported scenario_class {class}"));
        }
        if !scenario_id.starts_with("OBS-") || !scenario_ids.insert(scenario_id.to_owned()) {
            errors.push(format!("{context}: invalid or duplicate scenario ID"));
        }

        let case_states = string_array(case, "required_state_ids", &context, &mut errors);
        let case_state_set: BTreeSet<String> = case_states.iter().cloned().collect();
        if case_state_set != required_set || case_states.len() != required_set.len() {
            errors.push(format!(
                "{context}: required_state_ids do not match frozen profile"
            ));
        }

        let topology = case.get("topology").unwrap_or(&Value::Null);
        let topology_valid = topology_is_valid(topology, &mut errors, &context);
        let structural = case.get("structural_observability").unwrap_or(&Value::Null);
        let structural_status = required_str(structural, "status", &context, &mut errors)
            .unwrap_or_else(|| "not_proven".to_owned());
        if !is_one_of(&structural_status, &["full", "partial", "not_proven"]) {
            errors.push(format!(
                "{context}: unsupported structural observability status"
            ));
        }
        let proof_ref = optional_ref(structural, "proof_ref", &context, &mut errors);
        match structural_status.as_str() {
            "full" | "partial" if proof_ref.is_none() => {
                errors.push(format!(
                    "{context}: full/partial structural result requires a proof reference"
                ));
            }
            "not_proven" if proof_ref.is_some() => {
                errors.push(format!(
                    "{context}: not_proven structural result must not carry a proof reference"
                ));
            }
            _ => {}
        }

        let estimator = parse_estimator(
            case.get("numerical_estimator").unwrap_or(&Value::Null),
            &context,
            &mut errors,
        );
        if let Some(estimator) = estimator.as_ref() {
            let covered: BTreeSet<String> = estimator.covered_state_ids.iter().cloned().collect();
            if covered.len() != estimator.covered_state_ids.len() {
                errors.push(format!(
                    "{context}: estimator covered_state_ids contains duplicates"
                ));
            }
            if covered
                .iter()
                .any(|state_id| !required_set.contains(state_id))
            {
                errors.push(format!(
                    "{context}: estimator scope references a non-required state"
                ));
            }
        }
        let estimator_qualified_states =
            estimator.as_ref().map_or_else(BTreeSet::new, |estimator| {
                estimator_qualified_scope(estimator, profile, &required_set, &context, &mut errors)
            });

        let failed_rows = case.get("failed_domains").and_then(Value::as_array);
        let mut failed_domains = BTreeSet::new();
        if let Some(failed_rows) = failed_rows {
            for row in failed_rows {
                let domain_type = required_str(row, "domain_type", &context, &mut errors);
                let domain_id = required_str(row, "domain_id", &context, &mut errors);
                if let (Some(domain_type), Some(domain_id)) = (domain_type, domain_id) {
                    if !FAILURE_DOMAINS.contains(&domain_type.as_str()) {
                        errors.push(format!(
                            "{context}: unsupported failed domain type {domain_type}"
                        ));
                    }
                    if !failed_domains.insert((domain_type.clone(), domain_id.clone())) {
                        errors.push(format!(
                            "{context}: duplicate failed domain {domain_type}/{domain_id}"
                        ));
                    }
                }
            }
        } else {
            errors.push(format!("{context}: failed_domains must be an array"));
        }

        let sensor_rows = case.get("sensors").and_then(Value::as_array);
        let Some(sensor_rows) = sensor_rows else {
            errors.push(format!("{context}: sensors must be an array"));
            continue;
        };
        let mut sensors = Vec::new();
        let mut sensor_ids = BTreeSet::new();
        for row in sensor_rows {
            if let Some(sensor) = parse_sensor(row, scenario_id, &required_set, &mut errors) {
                if !sensor_ids.insert(sensor.sensor_id.clone()) {
                    errors.push(format!(
                        "{context}: duplicate sensor_id {}",
                        sensor.sensor_id
                    ));
                }
                sensors.push(sensor);
            }
        }

        let mut state_results = Vec::new();
        let mut state_dispositions = Vec::new();
        for state_id in REQUIRED_STATES {
            let (disposition, direct_sensor_ids, surviving_sensor_ids, failed_overlap_count) =
                state_disposition(
                    state_id,
                    &sensors,
                    &failed_domains,
                    topology_valid,
                    &structural_status,
                    &estimator_qualified_states,
                );
            state_dispositions.push(disposition.clone());
            state_results.push(json!({
                "state_id": state_id,
                "disposition": disposition,
                "direct_sensor_ids": direct_sensor_ids,
                "surviving_direct_sensor_ids": surviving_sensor_ids,
                "failed_dependency_sensor_count": failed_overlap_count
            }));
        }

        let overall = overall_disposition(&state_dispositions, topology_valid);
        let oracle_row = oracle_by_id.get(scenario_id).copied();
        if oracle_row.is_none() {
            errors.push(format!("{context}: scenario missing from separate oracle"));
        } else if let Some(oracle_row) = oracle_row {
            let expected_overall = get_str(oracle_row, "overall_disposition").unwrap_or("");
            if expected_overall != overall {
                errors.push(format!(
                    "{context}: overall disposition expected {expected_overall}, recomputed {overall}"
                ));
            }
            if oracle_row
                .get("structural_proof_reusable")
                .and_then(Value::as_bool)
                != Some(topology_valid)
            {
                errors.push(format!(
                    "{context}: structural_proof_reusable differs from exact topology binding"
                ));
            }
            let expected_states = oracle_row
                .get("state_dispositions")
                .and_then(Value::as_array);
            let mut expected_by_state = BTreeMap::new();
            if let Some(rows) = expected_states {
                for row in rows {
                    let state_id = get_str(row, "state_id");
                    let disposition = get_str(row, "disposition");
                    match (state_id, disposition) {
                        (Some(state_id), Some(disposition)) => {
                            if expected_by_state
                                .insert(state_id.to_owned(), disposition.to_owned())
                                .is_some()
                            {
                                errors.push(format!(
                                    "{context}: duplicate state in oracle {state_id}"
                                ));
                            }
                        }
                        _ => {
                            errors.push(format!("{context}: malformed state disposition in oracle"))
                        }
                    }
                }
            } else {
                errors.push(format!(
                    "{context}: oracle state_dispositions must be an array"
                ));
            }
            for (index, state_id) in REQUIRED_STATES.iter().enumerate() {
                if expected_by_state.get(*state_id) != Some(&state_dispositions[index]) {
                    errors.push(format!(
                        "{context}/{state_id}: state disposition differs from separate oracle"
                    ));
                }
            }
            if expected_by_state.len() != REQUIRED_STATES.len() {
                errors.push(format!(
                    "{context}: oracle must contain exactly four unique states"
                ));
            }
        }

        results.push(json!({
            "scenario_id": scenario_id,
            "verification_state": "Matched",
            "overall_disposition": overall,
            "structural_proof_reusable": topology_valid,
            "state_results": state_results,
            "evidence_ceiling": "synthetic_design_only"
        }));
    }

    if scenario_ids.len() != EXPECTED_SCENARIOS
        || scenario_ids.len() != oracle_by_id.len()
        || oracle_by_id.keys().any(|id| !scenario_ids.contains(id))
    {
        errors.push("scenario/oracle case ID sets are not exactly equal".to_owned());
    }
    if !get_str(scenarios, "claim_ceiling")
        .is_some_and(|value| value.to_ascii_lowercase().contains("synthetic"))
    {
        errors.push("scenario claim ceiling must remain synthetic".to_owned());
    }

    if errors.is_empty() {
        Ok(json!({
            "schema_version": "water-network-observability-evaluation-v0.1",
            "verification_state": "Matched",
            "qualification_status": "NotRun",
            "scenario_count": cases.len(),
            "matched_scenario_count": results.len(),
            "required_state_count": REQUIRED_STATES.len(),
            "results": results,
            "evidence_class": "synthetic_observability_semantics_only",
            "claim_ceiling": "sensor_path_and_observability_contract_semantics_only_no_physical_qualification",
            "nonclaims": [
                "No real network topology or sensor layout is qualified.",
                "No hydraulic simulation or numerical state estimation is executed.",
                "No pump, valve, SCADA or utility operation is authorized.",
                "No service floor, customer outcome or water-quality result is established."
            ]
        }))
    } else {
        Err(errors)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIOS: &str = include_str!(
        "../../../research/water-service-engineering/observability-qualification/scenarios-v0.1.json"
    );
    const SCHEMA: &str = include_str!(
        "../../../research/water-service-engineering/observability-qualification/scenarios-v0.1.schema.json"
    );
    const ORACLE: &str = include_str!(
        "../../../research/water-service-engineering/observability-qualification/disposition-oracle-v0.1.json"
    );

    fn fixtures() -> (Value, Value, Value) {
        (
            serde_json::from_str(SCENARIOS).expect("scenarios parse"),
            serde_json::from_str(SCHEMA).expect("schema parses"),
            serde_json::from_str(ORACLE).expect("oracle parses"),
        )
    }

    #[test]
    fn all_twelve_cases_and_forty_eight_state_dispositions_match_oracle() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("frozen corpus validates");
        assert_eq!(value["scenario_count"], EXPECTED_SCENARIOS);
        assert_eq!(value["matched_scenario_count"], EXPECTED_SCENARIOS);
        assert_eq!(value["required_state_count"], REQUIRED_STATES.len());
        assert_eq!(value["qualification_status"], "NotRun");
    }

    #[test]
    fn duplicate_sensor_id_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["sensors"][1]["sensor_id"] =
            scenarios["scenarios"][0]["sensors"][0]["sensor_id"].clone();
        let errors =
            evaluate(&scenarios, &schema, &oracle).expect_err("duplicate sensor ID must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate sensor_id"))
        );
    }

    #[test]
    fn missing_failure_domain_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["sensors"][0]["dependencies"]
            .as_array_mut()
            .expect("dependencies")
            .pop();
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("missing domain must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("six failure-domain types"))
        );
    }

    #[test]
    fn duplicate_failure_domain_type_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["sensors"][0]["dependencies"][5]["domain_type"] =
            json!("clock_domain");
        let errors =
            evaluate(&scenarios, &schema, &oracle).expect_err("duplicate domain must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate dependency domain type"))
        );
    }

    #[test]
    fn sensor_cannot_bind_to_unknown_required_state() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["sensors"][0]["state_id"] = json!("unregistered_state");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("unknown state must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("is not required by the profile"))
        );
    }

    #[test]
    fn shared_calibration_domain_removes_all_dependent_channels() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = &value["results"][2];
        assert_eq!(
            case["state_results"][1]["surviving_direct_sensor_ids"],
            json!([])
        );
        assert_eq!(
            case["state_results"][1]["disposition"],
            "unobservable_under_declared_faults"
        );
    }

    #[test]
    fn shared_power_domain_removes_all_dependent_channels() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(
            value["results"][3]["state_results"][1]["surviving_direct_sensor_ids"],
            json!([])
        );
    }

    #[test]
    fn shared_communications_domain_removes_all_dependent_channels() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(
            value["results"][4]["state_results"][1]["surviving_direct_sensor_ids"],
            json!([])
        );
    }

    #[test]
    fn clock_domain_failure_is_indeterminate_not_current_or_unobservable() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(
            value["results"][5]["state_results"][1]["disposition"],
            "indeterminate_evidence"
        );
        assert_eq!(
            value["results"][5]["overall_disposition"],
            "indeterminate_evidence"
        );
    }

    #[test]
    fn topology_revision_mismatch_invalidates_proof_but_preserves_direct_readings() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = &value["results"][6];
        assert_eq!(
            case["overall_disposition"],
            "topology_invalidates_certificate"
        );
        assert_eq!(
            case["state_results"][0]["disposition"],
            "directly_observed_current"
        );
        assert_eq!(case["structural_proof_reusable"], false);
    }

    #[test]
    fn missing_topology_digest_match_invalidates_certificate() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["topology"]["certificate_topology_digest"] =
            json!("sha256:other-digest");
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("digest mismatch must fail with stale status");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("verified_for_exact_topology"))
        );
    }

    #[test]
    fn incomplete_state_mapping_invalidates_model_derived_state_only() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        let case = &value["results"][8];
        assert_eq!(
            case["overall_disposition"],
            "topology_invalidates_certificate"
        );
        assert_eq!(
            case["state_results"][0]["disposition"],
            "directly_observed_current"
        );
        assert_eq!(
            case["state_results"][3]["disposition"],
            "topology_invalidates_certificate"
        );
    }

    #[test]
    fn structural_observability_without_numeric_quality_is_not_estimator_qualification() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(
            value["results"][7]["state_results"][1]["disposition"],
            "structurally_observable_only"
        );
    }

    #[test]
    fn qualified_estimator_contract_is_explicitly_synthetic() {
        let (scenarios, schema, oracle) = fixtures();
        let value = evaluate(&scenarios, &schema, &oracle).expect("fixture validates");
        assert_eq!(
            value["results"][10]["overall_disposition"],
            "estimator_qualified_all"
        );
        assert_eq!(value["qualification_status"], "NotRun");
    }

    #[test]
    fn estimator_requires_holdout_reference() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][10]["numerical_estimator"]["heldout_ref"] = Value::Null;
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("holdout is required");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("missing heldout_ref"))
        );
    }

    #[test]
    fn estimator_cannot_claim_physical_qualification() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][10]["numerical_estimator"]["qualification_claimed"] = json!(true);
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("physical qualification claim must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("cannot claim physical qualification"))
        );
    }

    #[test]
    fn estimator_interval_coverage_outside_contract_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][10]["numerical_estimator"]["interval_coverage"] = json!(0.99);
        let errors =
            evaluate(&scenarios, &schema, &oracle).expect_err("out-of-range coverage must fail");
        assert!(
            errors
                .iter()
                .any(|error| error
                    .contains("does not meet coverage/sample/width/conditioning gates"))
        );
    }

    #[test]
    fn altered_overall_oracle_is_rejected() {
        let (scenarios, schema, mut oracle) = fixtures();
        oracle["results"][0]["overall_disposition"] = json!("unobservable_under_declared_faults");
        let errors = evaluate(&scenarios, &schema, &oracle).expect_err("altered oracle must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("overall disposition expected"))
        );
    }

    #[test]
    fn duplicate_oracle_state_is_rejected() {
        let (scenarios, schema, mut oracle) = fixtures();
        oracle["results"][0]["state_dispositions"][3]["state_id"] = json!("junction_pressure_1");
        let errors =
            evaluate(&scenarios, &schema, &oracle).expect_err("duplicate oracle state must fail");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("duplicate state in oracle"))
        );
    }

    #[test]
    fn active_failure_domain_must_remove_the_sensor_even_if_condition_is_available() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["failed_domains"] = json!([
            {"domain_type":"power_supply","domain_id":"PWR-S-TL1"}
        ]);
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("active power failure must change the direct state result");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("state disposition differs from separate oracle"))
        );
    }
    #[test]
    fn estimator_can_only_qualify_explicitly_covered_states() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][10]["numerical_estimator"]["covered_state_ids"] =
            json!(["junction_pressure_1"]);
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("estimator cannot qualify a state outside its declared scope");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("state disposition differs from separate oracle"))
        );
    }

    #[test]
    fn not_used_estimator_cannot_claim_any_state_scope() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["numerical_estimator"]["covered_state_ids"] =
            json!(["tank_level_1"]);
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("not_used estimator cannot cover states");
        assert!(errors.iter().any(|error| {
            error.contains("not_used estimator must have an empty covered_state_ids set")
        }));
    }

    #[test]
    fn estimator_scope_cannot_name_a_non_required_state_even_when_not_used() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["numerical_estimator"]["covered_state_ids"] =
            json!(["unregistered_state"]);
        let errors = evaluate(&scenarios, &schema, &oracle)
            .expect_err("estimator scope must be part of the required state universe");
        assert!(
            errors
                .iter()
                .any(|error| error.contains("non-required state"))
        );
    }

    #[test]
    fn missing_topology_identity_is_rejected() {
        let (mut scenarios, schema, oracle) = fixtures();
        scenarios["scenarios"][0]["topology"]["topology_ref"] = Value::Null;
        let errors =
            evaluate(&scenarios, &schema, &oracle).expect_err("topology identity is mandatory");
        assert!(errors.iter().any(|error| error.contains("topology_ref")));
    }
}

