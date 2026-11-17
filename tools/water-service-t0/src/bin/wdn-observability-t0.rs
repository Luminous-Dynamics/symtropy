// SPDX-License-Identifier: AGPL-3.0-or-later
//! Read-only, synthetic-first water-network observability T0 evaluator.
//! This binary checks evidence boundaries; it is not a hydraulic solver,
//! sensor-placement optimizer, state estimator, or SCADA control path.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const INPUT_VERSION: &str = "wdn-observability-t0-v0.1";
const ORACLE_VERSION: &str = "wdn-observability-t0-oracle-v0.1";
const BENCHMARK_ID: &str = "WATER-OBSERVABILITY-T0-EXECUTOR-001";
const DOMAIN_TYPES: [&str; 6] = [
    "physical_interface",
    "calibration_lineage",
    "power_supply",
    "communications_path",
    "clock_domain",
    "measurement_principle",
];
const STATE_COUNT: usize = 4;
const CASE_COUNT: usize = 12;
const STATE_DISPOSITION_COUNT: usize = CASE_COUNT * STATE_COUNT;
const OBSERVABILITY_SCHEMA_URI: &str = "https://json-schema.org/draft/2020-12/schema";

#[derive(Clone, Debug)]
struct Sensor {
    sensor_id: String,
    state_id: String,
    condition: String,
    binding_status: String,
    calibration_status: String,
    time_status: String,
    failure_domains: BTreeMap<String, String>,
}

fn field<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("{context}: missing '{key}'"))
}

fn string<'a>(value: &'a Value, key: &str, context: &str) -> Result<&'a str, String> {
    field(value, key, context)?
        .as_str()
        .ok_or_else(|| format!("{context}: '{key}' must be a string"))
}

fn number(value: &Value, key: &str, context: &str) -> Result<f64, String> {
    field(value, key, context)?
        .as_f64()
        .ok_or_else(|| format!("{context}: '{key}' must be numeric"))
}

fn boolean(value: &Value, key: &str, context: &str) -> Result<bool, String> {
    field(value, key, context)?
        .as_bool()
        .ok_or_else(|| format!("{context}: '{key}' must be boolean"))
}

fn ensure_keys(value: &Value, expected: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let wanted: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != wanted {
        return Err(format!(
            "{context}: key set differs from the frozen v0.1 contract"
        ));
    }
    Ok(())
}

fn string_set(value: &Value, context: &str) -> Result<BTreeSet<String>, String> {
    let values = value
        .as_array()
        .ok_or_else(|| format!("{context}: must be an array"))?;
    let mut output = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        let item = item
            .as_str()
            .ok_or_else(|| format!("{context}[{index}]: must be a string"))?;
        if item.trim().is_empty() {
            return Err(format!("{context}[{index}]: empty string is not allowed"));
        }
        if !output.insert(item.to_owned()) {
            return Err(format!("{context}: duplicate value '{item}'"));
        }
    }
    Ok(output)
}

fn digest_is_valid(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn nonempty_ref(value: &Value, key: &str, _context: &str) -> bool {
    value
        .get(key)
        .and_then(Value::as_str)
        .is_some_and(|reference| !reference.trim().is_empty())
}

fn parse_sensor(value: &Value, index: usize, required_states: &BTreeSet<String>) -> Result<Sensor, String> {
    let context = format!("sensors[{index}]");
    ensure_keys(
        value,
        &[
            "sensor_id",
            "state_id",
            "measurement_principle",
            "condition",
            "binding_status",
            "calibration_status",
            "time_status",
            "reading_value",
            "failure_domains",
        ],
        &context,
    )?;
    let sensor_id = string(value, "sensor_id", &context)?.to_owned();
    let state_id = string(value, "state_id", &context)?.to_owned();
    if sensor_id.trim().is_empty() {
        return Err(format!("{context}: sensor_id is empty"));
    }
    if !required_states.contains(&state_id) {
        return Err(format!(
            "{context}: sensor state '{state_id}' is outside the four required states"
        ));
    }
    if string(value, "measurement_principle", &context)?.trim().is_empty() {
        return Err(format!("{context}: measurement_principle is empty"));
    }
    let condition = string(value, "condition", &context)?.to_owned();
    if condition != "healthy" && condition != "failed" {
        return Err(format!("{context}: unsupported condition '{condition}'"));
    }
    let binding_status = string(value, "binding_status", &context)?.to_owned();
    if binding_status != "valid" && binding_status != "invalid" {
        return Err(format!(
            "{context}: unsupported binding_status '{binding_status}'"
        ));
    }
    let calibration_status = string(value, "calibration_status", &context)?.to_owned();
    if !["current", "stale", "missing"].contains(&calibration_status.as_str()) {
        return Err(format!(
            "{context}: unsupported calibration_status '{calibration_status}'"
        ));
    }
    let time_status = string(value, "time_status", &context)?.to_owned();
    if time_status != "current" && time_status != "invalid" {
        return Err(format!("{context}: unsupported time_status '{time_status}'"));
    }
    let reading = number(value, "reading_value", &context)?;
    if !reading.is_finite() {
        return Err(format!("{context}: reading_value must be finite"));
    }

    let dependencies = field(value, "failure_domains", &context)?
        .as_array()
        .ok_or_else(|| format!("{context}: failure_domains must be an array"))?;
    if dependencies.len() != DOMAIN_TYPES.len() {
        return Err(format!(
            "{context}: exactly six failure-domain entries are required"
        ));
    }
    let mut failure_domains = BTreeMap::new();
    for (domain_index, dependency) in dependencies.iter().enumerate() {
        let domain_context = format!("{context}.failure_domains[{domain_index}]");
        ensure_keys(
            dependency,
            &["domain_type", "domain_id"],
            &domain_context,
        )?;
        let domain_type = string(dependency, "domain_type", &domain_context)?;
        let domain_id = string(dependency, "domain_id", &domain_context)?;
        if !DOMAIN_TYPES.contains(&domain_type) {
            return Err(format!(
                "{domain_context}: unknown dependency domain '{domain_type}'"
            ));
        }
        if domain_id.trim().is_empty() {
            return Err(format!("{domain_context}: domain_id is empty"));
        }
        if failure_domains
            .insert(domain_type.to_owned(), domain_id.to_owned())
            .is_some()
        {
            return Err(format!(
                "{context}: duplicate dependency domain type '{domain_type}'"
            ));
        }
    }
    let observed_domains: BTreeSet<&str> = failure_domains.keys().map(String::as_str).collect();
    let expected_domains: BTreeSet<&str> = DOMAIN_TYPES.into_iter().collect();
    if observed_domains != expected_domains {
        return Err(format!(
            "{context}: failure-domain set must contain each of the six domain types exactly once"
        ));
    }

    Ok(Sensor {
        sensor_id,
        state_id,
        condition,
        binding_status,
        calibration_status,
        time_status,
        failure_domains,
    })
}

fn estimator_is_qualified(
    estimator: &Value,
    topology_revision: &str,
    topology_digest: &str,
    surviving_sensor_ids: &BTreeSet<String>,
    state_id: &str,
) -> Result<bool, String> {
    ensure_keys(
        estimator,
        &[
            "status",
            "sensor_ids",
            "state_ids",
            "topology_revision",
            "topology_digest",
            "model_scope_ref",
            "parameter_scope_ref",
            "calibration_ref",
            "heldout_ref",
            "ood_ref",
            "validity_ref",
            "qualification_claimed",
            "sample_count",
            "empirical_coverage_percent",
            "interval_width_percent",
            "condition_number",
        ],
        "estimator",
    )?;
    let status = string(estimator, "status", "estimator")?;
    if status == "not_available" {
        return Ok(false);
    }
    if status != "synthetic_qualified_contract" {
        return Err(format!("estimator: unsupported status '{status}'"));
    }

    let estimator_sensors = string_set(field(estimator, "sensor_ids", "estimator")?, "estimator.sensor_ids")?;
    let estimator_states = string_set(field(estimator, "state_ids", "estimator")?, "estimator.state_ids")?;
    let claimed = boolean(estimator, "qualification_claimed", "estimator")?;
    let sample_count = field(estimator, "sample_count", "estimator")?
        .as_u64()
        .ok_or_else(|| "estimator.sample_count must be a non-negative integer".to_owned())?;
    let coverage = number(estimator, "empirical_coverage_percent", "estimator")?;
    let width = number(estimator, "interval_width_percent", "estimator")?;
    let condition = number(estimator, "condition_number", "estimator")?;
    let estimator_revision = string(estimator, "topology_revision", "estimator")?;
    let estimator_digest = string(estimator, "topology_digest", "estimator")?;

    let references_present = [
        "model_scope_ref",
        "parameter_scope_ref",
        "calibration_ref",
        "heldout_ref",
        "ood_ref",
        "validity_ref",
    ]
    .into_iter()
    .all(|key| nonempty_ref(estimator, key, "estimator"));

    let qualified = !claimed
        && references_present
        && sample_count >= 100
        && (85.0..=95.0).contains(&coverage)
        && width > 0.0
        && width <= 20.0
        && condition > 0.0
        && condition <= 1_000_000.0
        && estimator_revision == topology_revision
        && estimator_digest == topology_digest
        && estimator_sensors == *surviving_sensor_ids
        && estimator_states.contains(state_id);
    Ok(qualified)
}

fn evaluate_scenario(
    scenario: &Value,
    required_state_ids: &[String],
) -> Result<Value, String> {
    ensure_keys(
        scenario,
        &[
            "case_id",
            "topology",
            "sensors",
            "active_failed_domains",
            "structural_certificate",
            "estimator",
        ],
        "scenario",
    )?;
    let case_id = string(scenario, "case_id", "scenario")?;
    let required_states: BTreeSet<String> = required_state_ids.iter().cloned().collect();

    let topology = field(scenario, "topology", "scenario")?;
    ensure_keys(topology, &["revision", "digest"], "topology")?;
    let topology_revision = string(topology, "revision", "topology")?;
    let topology_digest = string(topology, "digest", "topology")?;
    if topology_revision.trim().is_empty() || !digest_is_valid(topology_digest) {
        return Err(format!("{case_id}: topology revision/digest is invalid"));
    }

    let sensor_values = field(scenario, "sensors", "scenario")?
        .as_array()
        .ok_or_else(|| format!("{case_id}: sensors must be an array"))?;
    let mut sensors = Vec::with_capacity(sensor_values.len());
    let mut sensor_ids = BTreeSet::new();
    for (index, value) in sensor_values.iter().enumerate() {
        let sensor = parse_sensor(value, index, &required_states)?;
        if !sensor_ids.insert(sensor.sensor_id.clone()) {
            return Err(format!(
                "{case_id}: duplicate sensor_id '{}'",
                sensor.sensor_id
            ));
        }
        sensors.push(sensor);
    }

    let active_values = field(scenario, "active_failed_domains", "scenario")?
        .as_array()
        .ok_or_else(|| format!("{case_id}: active_failed_domains must be an array"))?;
    let mut active_domains = BTreeSet::new();
    for (index, value) in active_values.iter().enumerate() {
        let context = format!("{case_id}.active_failed_domains[{index}]");
        ensure_keys(value, &["domain_type", "domain_id"], &context)?;
        let domain_type = string(value, "domain_type", &context)?;
        let domain_id = string(value, "domain_id", &context)?;
        if !DOMAIN_TYPES.contains(&domain_type) || domain_id.trim().is_empty() {
            return Err(format!("{context}: invalid active failure domain"));
        }
        if !active_domains.insert((domain_type.to_owned(), domain_id.to_owned())) {
            return Err(format!("{context}: duplicate active failure domain"));
        }
    }

    let mut surviving_by_state: BTreeMap<String, BTreeSet<String>> = required_state_ids
        .iter()
        .map(|state| (state.clone(), BTreeSet::new()))
        .collect();
    let mut clock_invalid_by_state: BTreeMap<String, bool> = required_state_ids
        .iter()
        .map(|state| (state.clone(), false))
        .collect();

    for sensor in &sensors {
        let clock_domain = sensor
            .failure_domains
            .get("clock_domain")
            .ok_or_else(|| format!("{case_id}: sensor clock domain missing"))?;
        let clock_failed = active_domains.contains(&("clock_domain".to_owned(), clock_domain.clone()));
        if sensor.time_status != "current" || clock_failed {
            clock_invalid_by_state.insert(sensor.state_id.clone(), true);
        }
        let domain_failed = sensor.failure_domains.iter().any(|(domain_type, domain_id)| {
            active_domains.contains(&(domain_type.clone(), domain_id.clone()))
        });
        let survives = sensor.condition == "healthy"
            && sensor.binding_status == "valid"
            && sensor.calibration_status == "current"
            && sensor.time_status == "current"
            && !domain_failed;
        if survives {
            surviving_by_state
                .get_mut(&sensor.state_id)
                .ok_or_else(|| format!("{case_id}: sensor mapped to unknown state"))?
                .insert(sensor.sensor_id.clone());
        }
    }

    let mut surviving_sensor_ids = BTreeSet::new();
    for values in surviving_by_state.values() {
        surviving_sensor_ids.extend(values.iter().cloned());
    }

    let certificate = field(scenario, "structural_certificate", "scenario")?;
    ensure_keys(
        certificate,
        &[
            "status",
            "topology_revision",
            "topology_digest",
            "state_mapping_ids",
            "inferable_state_ids",
            "sensor_ids",
            "proof_ref",
        ],
        "structural_certificate",
    )?;
    let certificate_status = string(certificate, "status", "structural_certificate")?;
    let certificate_sensor_ids = string_set(
        field(certificate, "sensor_ids", "structural_certificate")?,
        "structural_certificate.sensor_ids",
    )?;
    let state_mapping_ids = string_set(
        field(certificate, "state_mapping_ids", "structural_certificate")?,
        "structural_certificate.state_mapping_ids",
    )?;
    let inferable_state_ids = string_set(
        field(certificate, "inferable_state_ids", "structural_certificate")?,
        "structural_certificate.inferable_state_ids",
    )?;
    if !inferable_state_ids.is_subset(&required_states) {
        return Err(format!("{case_id}: certificate lists an unknown inferable state"));
    }

    let certificate_disposition = if certificate_status == "missing" {
        if !field(certificate, "proof_ref", "structural_certificate")?.is_null() {
            return Err(format!("{case_id}: missing proof must not carry a proof reference"));
        }
        "missing_structural_proof"
    } else if certificate_status != "present" {
        return Err(format!(
            "{case_id}: unsupported structural certificate status '{certificate_status}'"
        ));
    } else if string(certificate, "topology_revision", "structural_certificate")?
        != topology_revision
        || string(certificate, "topology_digest", "structural_certificate")? != topology_digest
        || state_mapping_ids != required_states
    {
        "topology_invalidates_certificate"
    } else if surviving_sensor_ids.is_empty() {
        "no_surviving_sensor_channels"
    } else if certificate_sensor_ids != surviving_sensor_ids {
        "sensor_set_mismatch"
    } else {
        if !nonempty_ref(certificate, "proof_ref", "structural_certificate") {
            return Err(format!("{case_id}: present structural proof requires proof_ref"));
        }
        "usable"
    };

    let estimator = field(scenario, "estimator", "scenario")?;
    let mut state_dispositions = Vec::with_capacity(required_state_ids.len());
    let mut any_indeterminate = false;
    let mut any_unobservable = false;
    let mut any_topology_invalid = false;
    let mut any_estimator_qualified = false;
    let mut any_structural_only = false;

    for state_id in required_state_ids {
        let surviving = surviving_by_state
            .get(state_id)
            .cloned()
            .unwrap_or_default();
        let clock_invalid = *clock_invalid_by_state.get(state_id).unwrap_or(&false);
        let disposition = if !surviving.is_empty() {
            "directly_observed_current"
        } else if clock_invalid {
            any_indeterminate = true;
            "indeterminate_evidence"
        } else if certificate_disposition == "topology_invalidates_certificate" {
            any_topology_invalid = true;
            "topology_invalidates_certificate"
        } else if estimator_is_qualified(
            estimator,
            topology_revision,
            topology_digest,
            &surviving_sensor_ids,
            state_id,
        )? {
            any_estimator_qualified = true;
            "estimator_qualified_for_declared_scope"
        } else if certificate_disposition == "usable"
            && !surviving_sensor_ids.is_empty()
            && inferable_state_ids.contains(state_id)
        {
            any_structural_only = true;
            "structurally_observable_only"
        } else {
            any_unobservable = true;
            "unobservable_under_declared_faults"
        };
        state_dispositions.push(json!({
            "state_id": state_id,
            "disposition": disposition,
            "surviving_sensor_ids": surviving.iter().cloned().collect::<Vec<_>>(),
            "clock_evidence_invalid": clock_invalid
        }));
    }

    let overall = if any_indeterminate {
        "indeterminate_evidence"
    } else if any_topology_invalid
        || certificate_disposition == "topology_invalidates_certificate"
    {
        "topology_invalidates_certificate"
    } else if any_unobservable {
        "unobservable_under_declared_faults"
    } else if any_estimator_qualified {
        "estimator_qualified_for_declared_scope"
    } else if any_structural_only {
        "structurally_observable_only"
    } else {
        "directly_observed_current"
    };

    Ok(json!({
        "case_id": case_id,
        "topology_certificate_disposition": certificate_disposition,
        "structural_reuse_allowed": certificate_disposition == "usable",
        "overall_disposition": overall,
        "surviving_sensor_ids": surviving_sensor_ids.into_iter().collect::<Vec<_>>(),
        "state_dispositions": state_dispositions
    }))
}

fn evaluate_all(inputs: &Value, schema: &Value, oracle: &Value) -> Result<Value, String> {
    ensure_keys(
        inputs,
        &[
            "$schema",
            "schema_version",
            "benchmark_id",
            "evidence_class",
            "claim_ceiling",
            "required_state_ids",
            "domain_types",
            "scenarios",
        ],
        "observability input",
    )?;
    if string(inputs, "$schema", "observability input")? != OBSERVABILITY_SCHEMA_URI
        || string(schema, "$schema", "input schema")? != OBSERVABILITY_SCHEMA_URI
    {
        return Err("input/schema must use Draft 2020-12".to_owned());
    }
    if string(inputs, "schema_version", "observability input")? != INPUT_VERSION
        || string(inputs, "benchmark_id", "observability input")? != BENCHMARK_ID
        || string(inputs, "evidence_class", "observability input")? != "synthetic_only"
    {
        return Err("observability input identity/evidence class mismatch".to_owned());
    }
    if string(schema, "schema_version", "input schema")? != INPUT_VERSION {
        return Err("input schema version mismatch".to_owned());
    }
    if field(schema, "additionalProperties", "input schema")?.as_bool() != Some(false) {
        return Err("input schema must be a closed object contract".to_owned());
    }
    let required_state_ids: Vec<String> = field(inputs, "required_state_ids", "input")?
        .as_array()
        .ok_or_else(|| "required_state_ids must be an array".to_owned())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "required_state_ids values must be strings".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let required_set: BTreeSet<String> = required_state_ids.iter().cloned().collect();
    if required_state_ids.len() != STATE_COUNT || required_set.len() != STATE_COUNT {
        return Err("exactly four unique critical state IDs are required".to_owned());
    }
    let domain_values = string_set(field(inputs, "domain_types", "input")?, "domain_types")?;
    let expected_domains: BTreeSet<String> = DOMAIN_TYPES.iter().map(|value| (*value).to_owned()).collect();
    if domain_values != expected_domains {
        return Err("exactly the six frozen failure-domain types are required".to_owned());
    }

    ensure_keys(oracle, &["$schema", "schema_version", "benchmark_id", "cases"], "oracle")?;
    if string(oracle, "$schema", "oracle")? != OBSERVABILITY_SCHEMA_URI
        || string(oracle, "schema_version", "oracle")? != ORACLE_VERSION
        || string(oracle, "benchmark_id", "oracle")? != BENCHMARK_ID
    {
        return Err("oracle schema/version/benchmark identity mismatch".to_owned());
    }
    if field(oracle, "additionalProperties", "oracle").is_ok() {
        return Err("oracle payload must not carry a schema-only additionalProperties property".to_owned());
    }

    let scenarios = field(inputs, "scenarios", "input")?
        .as_array()
        .ok_or_else(|| "scenarios must be an array".to_owned())?;
    let expected_cases = field(oracle, "cases", "oracle")?
        .as_array()
        .ok_or_else(|| "oracle.cases must be an array".to_owned())?;
    if scenarios.len() != CASE_COUNT || expected_cases.len() != CASE_COUNT {
        return Err(format!(
            "exactly {CASE_COUNT} scenarios and oracle cases are required"
        ));
    }

    let mut oracle_by_id = BTreeMap::new();
    for expected in expected_cases {
        let id = string(expected, "case_id", "oracle case")?.to_owned();
        if oracle_by_id.insert(id.clone(), expected).is_some() {
            return Err(format!("duplicate oracle case_id '{id}'"));
        }
    }
    let mut input_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    let mut state_disposition_count = 0usize;

    for scenario in scenarios {
        let id = string(scenario, "case_id", "scenario")?.to_owned();
        if !input_ids.insert(id.clone()) {
            return Err(format!("duplicate input case_id '{id}'"));
        }
        let actual = evaluate_scenario(scenario, &required_state_ids)?;
        let expected = oracle_by_id
            .get(&id)
            .ok_or_else(|| format!("input case '{id}' is absent from oracle"))?;
        let expected_states = field(expected, "state_dispositions", "oracle case")?
            .as_array()
            .ok_or_else(|| format!("oracle case '{id}' state_dispositions must be an array"))?;
        state_disposition_count += expected_states.len();
        if &actual != *expected {
            mismatches.push(json!({
                "case_id": id,
                "expected": expected,
                "actual": actual
            }));
        }
        results.push(actual);
    }
    if input_ids != oracle_by_id.keys().cloned().collect() {
        return Err("input and oracle case ID sets differ".to_owned());
    }
    if state_disposition_count != STATE_DISPOSITION_COUNT {
        return Err(format!(
            "oracle must contain {STATE_DISPOSITION_COUNT} state dispositions"
        ));
    }

    let mismatch_count = mismatches.len();
    let matched_case_count = CASE_COUNT - mismatch_count;
    Ok(json!({
        "schema_version": "wdn-observability-t0-evaluation-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "case_count": CASE_COUNT,
        "matched_case_count": matched_case_count,
        "state_disposition_count": state_disposition_count,
        "mismatch_count": mismatch_count,
        "mismatches": mismatches,
        "case_results": results,
        "qualification_status": "NotRun",
        "claim_ceiling": "synthetic_observability_contract_only"
    }))
}

fn read_bytes(path: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read '{path}': {error}"))
}

fn read_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = read_bytes(path)?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid JSON in '{path}': {error}"))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run(args: &[String]) -> Result<Value, String> {
    if args.len() != 4 {
        return Err(
            "usage: wdn-observability-t0 INPUT_JSON INPUT_SCHEMA ORACLE_JSON RECEIPT_JSON".to_owned(),
        );
    }
    let (input_bytes, inputs) = read_json(&args[0])?;
    let (schema_bytes, schema) = read_json(&args[1])?;
    let (oracle_bytes, oracle) = read_json(&args[2])?;
    let evaluation = evaluate_all(&inputs, &schema, &oracle)?;
    let success = evaluation["case_count"].as_u64() == Some(CASE_COUNT as u64)
        && evaluation["matched_case_count"].as_u64() == Some(CASE_COUNT as u64)
        && evaluation["state_disposition_count"].as_u64() == Some(STATE_DISPOSITION_COUNT as u64)
        && evaluation["mismatch_count"].as_u64() == Some(0);

    let revision = env::var("WATER_OBSERVABILITY_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let implementation = include_bytes!("wdn-observability-t0.rs");
    let receipt = json!({
        "receipt_schema_version": "wdn-observability-t0-receipt-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "verification_state": if success && revision.is_some() {
            "PassedExactRevision"
        } else if success {
            "PassedLocalUnpinned"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "input_schema_sha256": sha256(&schema_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "package_manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "implementation_sha256": sha256(implementation),
        "case_count": evaluation["case_count"],
        "matched_case_count": evaluation["matched_case_count"],
        "state_disposition_count": evaluation["state_disposition_count"],
        "mismatch_count": evaluation["mismatch_count"],
        "mismatches": evaluation["mismatches"],
        "claim_ceiling": "synthetic_observability_contract_only"
    });

    let output = Path::new(&args[3]);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("cannot create receipt directory: {error}"))?;
    }
    fs::write(
        output,
        serde_json::to_vec_pretty(&receipt)
            .map_err(|error| format!("serialize receipt: {error}"))?,
    )
    .map_err(|error| format!("write receipt '{}': {error}", output.display()))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).unwrap_or_default()
    );
    if success {
        Ok(receipt)
    } else {
        Err("observability T0 evaluator did not match the frozen oracle".to_owned())
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("wdn-observability-t0: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value, Value) {
        let inputs: Value = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_observability_t0_input.v0.1.json"
        ))
        .unwrap();
        let schema: Value = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_observability_t0_input.schema.v0.1.json"
        ))
        .unwrap();
        let oracle: Value = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_observability_t0_oracle.v0.1.json"
        ))
        .unwrap();
        (inputs, schema, oracle)
    }

    fn case_result<'a>(result: &'a Value, id: &str) -> &'a Value {
        result["case_results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["case_id"] == id)
            .unwrap()
    }

    #[test]
    fn canonical_twelve_case_oracle_matches_all_48_state_dispositions() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        assert_eq!(result["case_count"], 12);
        assert_eq!(result["matched_case_count"], 12);
        assert_eq!(result["state_disposition_count"], 48);
        assert_eq!(result["mismatch_count"], 0);
        assert_eq!(result["qualification_status"], "NotRun");
    }

    #[test]
    fn independent_backup_survives_single_channel_dropout() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        let case = case_result(
            &result,
            "OBS-T0-002-single-channel-dropout-with-independent-backup",
        );
        assert_eq!(
            case["state_dispositions"][0]["disposition"],
            "directly_observed_current"
        );
        assert_eq!(case["state_dispositions"][0]["surviving_sensor_ids"][0], "S1-ALT");
    }

    #[test]
    fn common_calibration_power_and_communications_failures_remove_shared_channels() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        for id in [
            "OBS-T0-003-common-calibration-dependency",
            "OBS-T0-004-common-power-dependency",
            "OBS-T0-005-common-communications-dependency",
        ] {
            let case = case_result(&result, id);
            assert_eq!(
                case["state_dispositions"][0]["disposition"],
                "unobservable_under_declared_faults"
            );
            assert_eq!(case["state_dispositions"][0]["surviving_sensor_ids"].as_array().unwrap().len(), 0);
        }
    }

    #[test]
    fn common_clock_failure_is_indeterminate_not_normal_or_unobservable() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        let case = case_result(&result, "OBS-T0-006-common-clock-failure");
        assert_eq!(case["overall_disposition"], "indeterminate_evidence");
        assert_eq!(
            case["state_dispositions"][0]["disposition"],
            "indeterminate_evidence"
        );
    }

    #[test]
    fn stale_topology_certificate_does_not_erase_direct_instrument_readings() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        let case = case_result(&result, "OBS-T0-007-stale-topology-certificate-direct-readings-preserved");
        assert_eq!(case["topology_certificate_disposition"], "topology_invalidates_certificate");
        assert_eq!(
            case["state_dispositions"][0]["disposition"],
            "directly_observed_current"
        );
        assert_eq!(case["structural_reuse_allowed"], false);
    }

    #[test]
    fn ill_conditioned_estimator_is_not_promoted_above_structural_only() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        let case = case_result(&result, "OBS-T0-008-ill-conditioned-estimator-not-qualified");
        assert_eq!(
            case["state_dispositions"][0]["disposition"],
            "structurally_observable_only"
        );
    }

    #[test]
    fn complete_synthetic_estimator_contract_does_not_claim_physical_qualification() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        let case = case_result(&result, "OBS-T0-012-evidence-qualified-estimator-with-claim-disabled");
        assert_eq!(
            case["state_dispositions"][0]["disposition"],
            "estimator_qualified_for_declared_scope"
        );
        assert_eq!(result["qualification_status"], "NotRun");
    }

    #[test]
    fn duplicate_sensor_identity_is_rejected() {
        let (mut inputs, schema, oracle) = fixture();
        let duplicate = inputs["scenarios"][0]["sensors"][0].clone();
        inputs["scenarios"][0]["sensors"].as_array_mut().unwrap().push(duplicate);
        assert!(evaluate_all(&inputs, &schema, &oracle)
            .unwrap_err()
            .contains("duplicate sensor_id"));
    }

    #[test]
    fn missing_dependency_domain_is_rejected() {
        let (mut inputs, schema, oracle) = fixture();
        inputs["scenarios"][0]["sensors"][0]["failure_domains"]
            .as_array_mut()
            .unwrap()
            .pop();
        assert!(evaluate_all(&inputs, &schema, &oracle)
            .unwrap_err()
            .contains("exactly six failure-domain entries"));
    }

    #[test]
    fn duplicate_dependency_domain_type_is_rejected() {
        let (mut inputs, schema, oracle) = fixture();
        inputs["scenarios"][0]["sensors"][0]["failure_domains"][5]["domain_type"] =
            json!("clock_domain");
        assert!(evaluate_all(&inputs, &schema, &oracle)
            .unwrap_err()
            .contains("duplicate dependency domain type"));
    }

    #[test]
    fn sensor_mapped_outside_critical_state_set_is_rejected() {
        let (mut inputs, schema, oracle) = fixture();
        inputs["scenarios"][0]["sensors"][0]["state_id"] = json!("pressure:unknown-node");
        assert!(evaluate_all(&inputs, &schema, &oracle)
            .unwrap_err()
            .contains("outside the four required states"));
    }

    #[test]
    fn incomplete_critical_state_mapping_invalidates_structural_certificate() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        let case = case_result(&result, "OBS-T0-009-incomplete-critical-state-mapping");
        assert_eq!(case["topology_certificate_disposition"], "topology_invalidates_certificate");
        assert_eq!(
            case["state_dispositions"][3]["disposition"],
            "topology_invalidates_certificate"
        );
    }

    #[test]
    fn missing_structural_proof_cannot_support_an_unmeasured_state() {
        let (inputs, schema, oracle) = fixture();
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        let case = case_result(&result, "OBS-T0-011-missing-structural-proof");
        assert_eq!(case["topology_certificate_disposition"], "missing_structural_proof");
        assert_eq!(
            case["state_dispositions"][0]["disposition"],
            "unobservable_under_declared_faults"
        );
    }

    #[test]
    fn claimed_physical_estimator_qualification_is_not_accepted() {
        let (mut inputs, schema, oracle) = fixture();
        inputs["scenarios"][11]["estimator"]["qualification_claimed"] = json!(true);
        let scenario = inputs["scenarios"][11].clone();
        let states: Vec<String> = inputs["required_state_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
        let result = evaluate_scenario(&scenario, &states).unwrap();
        assert_eq!(
            result["state_dispositions"][0]["disposition"],
            "structurally_observable_only"
        );
        let _ = (schema, oracle);
    }

    #[test]
    fn missing_ood_reference_denies_estimator_qualification() {
        let (mut inputs, _, _) = fixture();
        inputs["scenarios"][11]["estimator"]["ood_ref"] = Value::Null;
        let states: Vec<String> = inputs["required_state_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
        let result = evaluate_scenario(&inputs["scenarios"][11], &states).unwrap();
        assert_eq!(
            result["state_dispositions"][0]["disposition"],
            "structurally_observable_only"
        );
    }

    #[test]
    fn out_of_range_interval_coverage_denies_estimator_qualification() {
        let (mut inputs, _, _) = fixture();
        inputs["scenarios"][11]["estimator"]["empirical_coverage_percent"] = json!(99.9);
        let states: Vec<String> = inputs["required_state_ids"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_owned())
            .collect();
        let result = evaluate_scenario(&inputs["scenarios"][11], &states).unwrap();
        assert_eq!(
            result["state_dispositions"][0]["disposition"],
            "structurally_observable_only"
        );
    }

    #[test]
    fn altered_frozen_oracle_is_reported_as_a_mismatch() {
        let (inputs, schema, mut oracle) = fixture();
        oracle["cases"][0]["overall_disposition"] = json!("unobservable_under_declared_faults");
        let result = evaluate_all(&inputs, &schema, &oracle).unwrap();
        assert_eq!(result["matched_case_count"], 11);
        assert_eq!(result["mismatch_count"], 1);
        assert_eq!(result["mismatches"][0]["case_id"], "OBS-T0-001-nominal-independent-direct");
    }
}
