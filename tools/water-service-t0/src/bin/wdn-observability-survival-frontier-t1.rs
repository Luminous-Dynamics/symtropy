// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic, unweighted combinatorial sensor-layout resilience evaluator.
//! This is not a reliability model or sensor-placement optimizer.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::Path;

const INPUT_VERSION: &str = "wdn-observability-survival-frontier-t1-v0.1";
const ORACLE_VERSION: &str = "wdn-observability-survival-frontier-t1-oracle-v0.1";
const BENCHMARK_ID: &str = "WATER-OBSERVABILITY-SURVIVAL-FRONTIER-001";
const DOMAIN_TYPES: [&str; 6] = [
    "physical_interface",
    "calibration_lineage",
    "power_supply",
    "communications_path",
    "clock_domain",
    "measurement_principle",
];
const STATE_COUNT: usize = 4;
const LAYOUT_COUNT: usize = 12;
const MAX_FAILURE_ORDER: usize = 2;

#[derive(Clone, Debug)]
struct Dependency {
    domain_type: String,
    domain_id: String,
}

impl Dependency {
    fn component_id(&self) -> String {
        format!("domain:{}:{}", self.domain_type, self.domain_id)
    }
}

#[derive(Clone, Debug)]
struct Sensor {
    sensor_id: String,
    state_id: String,
    condition: String,
    binding_status: String,
    calibration_status: String,
    time_status: String,
    dependencies: Vec<Dependency>,
}

impl Sensor {
    fn is_baseline_eligible(&self) -> bool {
        self.condition == "healthy"
            && self.binding_status == "valid"
            && self.calibration_status == "current"
            && self.time_status == "current"
    }

    fn channel_component_id(&self) -> String {
        format!("channel:{}", self.sensor_id)
    }
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

fn ensure_keys(value: &Value, expected: &[&str], context: &str) -> Result<(), String> {
    let object = value
        .as_object()
        .ok_or_else(|| format!("{context}: must be an object"))?;
    let actual: BTreeSet<&str> = object.keys().map(String::as_str).collect();
    let required: BTreeSet<&str> = expected.iter().copied().collect();
    if actual != required {
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
    let mut result = BTreeSet::new();
    for (index, item) in values.iter().enumerate() {
        let item = item
            .as_str()
            .ok_or_else(|| format!("{context}[{index}]: must be a string"))?;
        if item.trim().is_empty() {
            return Err(format!("{context}[{index}]: empty string is not allowed"));
        }
        if !result.insert(item.to_owned()) {
            return Err(format!("{context}: duplicate value '{item}'"));
        }
    }
    Ok(result)
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn gcd(mut a: u64, mut b: u64) -> u64 {
    while b != 0 {
        let remainder = a % b;
        a = b;
        b = remainder;
    }
    a.max(1)
}

fn expected_combination_count(component_count: usize) -> Result<usize, String> {
    let pairs = component_count
        .checked_mul(component_count.saturating_sub(1))
        .and_then(|value| value.checked_div(2))
        .ok_or_else(|| "failure-combination count overflow".to_owned())?;
    1usize
        .checked_add(component_count)
        .and_then(|value| value.checked_add(pairs))
        .ok_or_else(|| "failure-combination count overflow".to_owned())
}

fn parse_sensor(
    value: &Value,
    index: usize,
    required_states: &BTreeSet<String>,
) -> Result<Sensor, String> {
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
            "failure_domains",
        ],
        &context,
    )?;
    let sensor_id = string(value, "sensor_id", &context)?.to_owned();
    if sensor_id.trim().is_empty() {
        return Err(format!("{context}: sensor_id is empty"));
    }
    let state_id = string(value, "state_id", &context)?.to_owned();
    if !required_states.contains(&state_id) {
        return Err(format!(
            "{context}: state '{state_id}' is not in the four required states"
        ));
    }
    if string(value, "measurement_principle", &context)?.trim().is_empty() {
        return Err(format!("{context}: measurement_principle is empty"));
    }
    let condition = string(value, "condition", &context)?.to_owned();
    if !["healthy", "failed"].contains(&condition.as_str()) {
        return Err(format!("{context}: unsupported condition '{condition}'"));
    }
    let binding_status = string(value, "binding_status", &context)?.to_owned();
    if !["valid", "invalid"].contains(&binding_status.as_str()) {
        return Err(format!("{context}: unsupported binding status '{binding_status}'"));
    }
    let calibration_status = string(value, "calibration_status", &context)?.to_owned();
    if !["current", "stale", "missing"].contains(&calibration_status.as_str()) {
        return Err(format!(
            "{context}: unsupported calibration status '{calibration_status}'"
        ));
    }
    let time_status = string(value, "time_status", &context)?.to_owned();
    if !["current", "invalid"].contains(&time_status.as_str()) {
        return Err(format!("{context}: unsupported time status '{time_status}'"));
    }

    let raw_domains = field(value, "failure_domains", &context)?
        .as_array()
        .ok_or_else(|| format!("{context}: failure_domains must be an array"))?;
    if raw_domains.len() != DOMAIN_TYPES.len() {
        return Err(format!(
            "{context}: exactly six failure-domain entries are required"
        ));
    }
    let mut dependencies = Vec::with_capacity(DOMAIN_TYPES.len());
    let mut seen_types = BTreeSet::new();
    for (domain_index, raw) in raw_domains.iter().enumerate() {
        let domain_context = format!("{context}.failure_domains[{domain_index}]");
        ensure_keys(raw, &["domain_type", "domain_id"], &domain_context)?;
        let domain_type = string(raw, "domain_type", &domain_context)?.to_owned();
        let domain_id = string(raw, "domain_id", &domain_context)?.to_owned();
        if !DOMAIN_TYPES.contains(&domain_type.as_str()) {
            return Err(format!("{domain_context}: unknown domain type '{domain_type}'"));
        }
        if domain_id.trim().is_empty() {
            return Err(format!("{domain_context}: domain_id is empty"));
        }
        if !seen_types.insert(domain_type.clone()) {
            return Err(format!(
                "{context}: duplicate dependency domain type '{domain_type}'"
            ));
        }
        dependencies.push(Dependency {
            domain_type,
            domain_id,
        });
    }
    let expected_domains: BTreeSet<String> =
        DOMAIN_TYPES.iter().map(|value| (*value).to_owned()).collect();
    if seen_types != expected_domains {
        return Err(format!(
            "{context}: each of the six dependency domains must appear exactly once"
        ));
    }

    Ok(Sensor {
        sensor_id,
        state_id,
        condition,
        binding_status,
        calibration_status,
        time_status,
        dependencies,
    })
}

fn combinations_of_two(components: &[String]) -> Vec<Vec<String>> {
    let mut output = Vec::new();
    for left in 0..components.len() {
        for right in (left + 1)..components.len() {
            output.push(vec![components[left].clone(), components[right].clone()]);
        }
    }
    output
}

fn failure_sets(components: &[String]) -> Vec<Vec<String>> {
    let mut output = vec![Vec::new()];
    output.extend(components.iter().map(|component| vec![component.clone()]));
    output.extend(combinations_of_two(components));
    output
}

fn contains_cut_subset(cuts: &[Vec<String>], candidate: &BTreeSet<String>) -> bool {
    cuts.iter()
        .any(|cut| cut.iter().all(|component| candidate.contains(component)))
}

fn evaluate_layout(
    layout: &Value,
    required_states: &[String],
    max_components: usize,
    max_combinations: usize,
) -> Result<Value, String> {
    ensure_keys(layout, &["layout_id", "sensors"], "layout")?;
    let layout_id = string(layout, "layout_id", "layout")?;
    let sensor_values = field(layout, "sensors", "layout")?
        .as_array()
        .ok_or_else(|| format!("{layout_id}: sensors must be an array"))?;
    if sensor_values.len() < required_states.len() || sensor_values.len() > 16 {
        return Err(format!("{layout_id}: sensor count outside the v0.1 bound"));
    }

    let required_set: BTreeSet<String> = required_states.iter().cloned().collect();
    let mut all_sensors = Vec::with_capacity(sensor_values.len());
    let mut sensor_ids = BTreeSet::new();
    for (index, value) in sensor_values.iter().enumerate() {
        let sensor = parse_sensor(value, index, &required_set)?;
        if !sensor_ids.insert(sensor.sensor_id.clone()) {
            return Err(format!(
                "{layout_id}: duplicate sensor id '{}'",
                sensor.sensor_id
            ));
        }
        all_sensors.push(sensor);
    }

    // A sensor that is already failed, stale, unbound or time-invalid is
    // excluded before defining the failure-component universe.
    let active_sensors: Vec<Sensor> = all_sensors
        .into_iter()
        .filter(Sensor::is_baseline_eligible)
        .collect();
    for state in required_states {
        if !active_sensors.iter().any(|sensor| &sensor.state_id == state) {
            return Err(format!(
                "{layout_id}: critical state '{state}' has no eligible baseline sensor"
            ));
        }
    }

    let mut component_catalog: BTreeMap<String, Value> = BTreeMap::new();
    for sensor in &active_sensors {
        let channel_id = sensor.channel_component_id();
        component_catalog.insert(
            channel_id.clone(),
            json!({
                "component_id": channel_id,
                "component_kind": "channel",
                "domain_type": null,
                "domain_id": sensor.sensor_id
            }),
        );
        for dependency in &sensor.dependencies {
            let component_id = dependency.component_id();
            component_catalog.insert(
                component_id.clone(),
                json!({
                    "component_id": component_id,
                    "component_kind": "dependency_domain",
                    "domain_type": dependency.domain_type,
                    "domain_id": dependency.domain_id
                }),
            );
        }
    }

    let components: Vec<String> = component_catalog.keys().cloned().collect();
    if components.len() > max_components {
        return Err(format!(
            "{layout_id}: active component count {} exceeds configured cap {max_components}",
            components.len()
        ));
    }
    let expected_count = expected_combination_count(components.len())?;
    if expected_count > max_combinations {
        return Err(format!(
            "{layout_id}: failure-combination count {expected_count} exceeds configured cap {max_combinations}"
        ));
    }

    let all_sets = failure_sets(&components);
    let mut summaries: BTreeMap<String, Vec<Vec<String>>> = required_states
        .iter()
        .map(|state| (state.clone(), Vec::new()))
        .collect();
    let mut first_unobservable: BTreeMap<String, Option<usize>> = required_states
        .iter()
        .map(|state| (state.clone(), None))
        .collect();
    let mut observed_by_set: BTreeMap<(Vec<String>, String), bool> = BTreeMap::new();
    let mut combinations_by_order = Vec::with_capacity(MAX_FAILURE_ORDER + 1);
    let mut state_survival: BTreeMap<String, Vec<Value>> = required_states
        .iter()
        .map(|state| (state.clone(), Vec::new()))
        .collect();

    for order in 0..=MAX_FAILURE_ORDER {
        let sets: Vec<Vec<String>> = all_sets
            .iter()
            .filter(|failure_set| failure_set.len() == order)
            .cloned()
            .collect();
        let mut observed_counts: BTreeMap<String, u64> = required_states
            .iter()
            .map(|state| (state.clone(), 0))
            .collect();
        let mut unobserved_counts: BTreeMap<String, u64> = required_states
            .iter()
            .map(|state| (state.clone(), 0))
            .collect();

        for failure_set in &sets {
            let failed: BTreeSet<String> = failure_set.iter().cloned().collect();
            let surviving: Vec<&Sensor> = active_sensors
                .iter()
                .filter(|sensor| {
                    !failed.contains(&sensor.channel_component_id())
                        && sensor
                            .dependencies
                            .iter()
                            .all(|dependency| !failed.contains(&dependency.component_id()))
                })
                .collect();

            for state in required_states {
                let observed = surviving.iter().any(|sensor| &sensor.state_id == state);
                observed_by_set.insert((failure_set.clone(), state.clone()), observed);
                if observed {
                    *observed_counts.get_mut(state).expect("state count exists") += 1;
                } else {
                    *unobserved_counts
                        .get_mut(state)
                        .expect("state count exists") += 1;
                    let existing_cuts = summaries.get_mut(state).expect("state cuts exist");
                    if !contains_cut_subset(existing_cuts, &failed) {
                        existing_cuts.push(failure_set.clone());
                        first_unobservable
                            .entry(state.clone())
                            .and_modify(|value| {
                                if value.is_none() {
                                    *value = Some(order);
                                }
                            });
                    }
                }
            }
        }

        let mut state_rows = Vec::with_capacity(required_states.len());
        for state in required_states {
            let total = sets.len() as u64;
            let observed = *observed_counts.get(state).expect("state count exists");
            let unobserved = *unobserved_counts.get(state).expect("state count exists");
            let divisor = gcd(observed, total);
            let row = json!({
                "state_id": state,
                "total_combinations": total,
                "observable_combinations": observed,
                "unobservable_combinations": unobserved,
                "survival_fraction_numerator": observed / divisor,
                "survival_fraction_denominator": total / divisor
            });
            state_survival
                .get_mut(state)
                .expect("state survival exists")
                .push(json!({
                    "failure_order": order,
                    "total_combinations": total,
                    "observable_combinations": observed,
                    "unobservable_combinations": unobserved,
                    "survival_fraction_numerator": observed / divisor,
                    "survival_fraction_denominator": total / divisor
                }));
            state_rows.push(row);
        }
        combinations_by_order.push(json!({
            "failure_order": order,
            "combination_count": sets.len(),
            "states": state_rows
        }));
    }

    let mut monotonicity_violations = 0usize;
    for state in required_states {
        for left in &all_sets {
            let left_observed = *observed_by_set
                .get(&(left.clone(), state.clone()))
                .expect("left failure set evaluated");
            if left_observed {
                continue;
            }
            for right in &all_sets {
                if left.len() <= right.len()
                    && left.iter().all(|component| right.contains(component))
                {
                    let right_observed = *observed_by_set
                        .get(&(right.clone(), state.clone()))
                        .expect("right failure set evaluated");
                    if right_observed {
                        monotonicity_violations += 1;
                    }
                }
            }
        }
    }

    let mut domain_fanout = Vec::new();
    for domain_type in DOMAIN_TYPES {
        let ids: BTreeSet<String> = active_sensors
            .iter()
            .filter_map(|sensor| {
                sensor
                    .dependencies
                    .iter()
                    .find(|dependency| dependency.domain_type == domain_type)
                    .map(|dependency| dependency.domain_id.clone())
            })
            .collect();
        for domain_id in ids {
            let mut dependents: Vec<String> = active_sensors
                .iter()
                .filter(|sensor| {
                    sensor.dependencies.iter().any(|dependency| {
                        dependency.domain_type == domain_type && dependency.domain_id == domain_id
                    })
                })
                .map(|sensor| sensor.sensor_id.clone())
                .collect();
            dependents.sort();
            domain_fanout.push(json!({
                "domain_type": domain_type,
                "domain_id": domain_id,
                "dependent_sensor_ids": dependents,
                "fanout": dependents.len()
            }));
        }
    }

    let state_summaries: Vec<Value> = required_states
        .iter()
        .map(|state| {
            let cuts = summaries.get(state).expect("state cuts exist");
            json!({
                "state_id": state,
                "first_unobservable_order": first_unobservable.get(state).expect("state order exists"),
                "minimal_cut_sets": cuts.iter().map(|components| json!({
                    "failure_order": components.len(),
                    "components": components
                })).collect::<Vec<_>>(),
                "survival_by_order": state_survival.get(state).expect("state survival exists")
            })
        })
        .collect();

    let component_values: Vec<Value> = component_catalog.into_values().collect();
    let channel_count = active_sensors.len();
    let dependency_count = components.len().saturating_sub(channel_count);

    Ok(json!({
        "layout_id": layout_id,
        "status": "evaluated",
        "component_count": components.len(),
        "channel_component_count": channel_count,
        "dependency_domain_component_count": dependency_count,
        "components": component_values,
        "shared_domain_count": domain_fanout
            .iter()
            .filter(|domain| domain["fanout"].as_u64().unwrap_or(0) > 1)
            .count(),
        "domain_fanout": domain_fanout,
        "combinations_by_order": combinations_by_order,
        "state_summaries": state_summaries,
        "monotonicity_check": if monotonicity_violations == 0 { "pass" } else { "fail" },
        "monotonicity_violation_count": monotonicity_violations,
        "probability_interpretation": "none_unweighted_combinatorial_counts"
    }))
}

fn evaluate_all(inputs: &Value, input_schema: &Value, oracle: &Value) -> Result<Value, String> {
    if string(inputs, "$schema", "input")? != "https://json-schema.org/draft/2020-12/schema"
        || string(input_schema, "$schema", "input schema")?
            != "https://json-schema.org/draft/2020-12/schema"
    {
        return Err("input and input schema must declare Draft 2020-12".to_owned());
    }
    if string(inputs, "schema_version", "input")? != INPUT_VERSION
        || string(inputs, "benchmark_id", "input")? != BENCHMARK_ID
        || string(inputs, "evidence_class", "input")? != "synthetic_only"
    {
        return Err("input schema/version/benchmark/evidence identity mismatch".to_owned());
    }
    if field(input_schema, "additionalProperties", "input schema")?.as_bool() != Some(false) {
        return Err("input schema must be closed".to_owned());
    }
    let schema_properties = field(input_schema, "properties", "input schema")?;
    if string(
        field(
            schema_properties,
            "schema_version",
            "input schema properties",
        )?,
        "const",
        "input schema properties.schema_version",
    )? != INPUT_VERSION
    {
        return Err("input schema version constant mismatch".to_owned());
    }

    let states: Vec<String> = field(inputs, "required_state_ids", "input")?
        .as_array()
        .ok_or_else(|| "required_state_ids must be an array".to_owned())?
        .iter()
        .map(|value| {
            value
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| "required_state_ids elements must be strings".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    if states.len() != STATE_COUNT || states.iter().collect::<BTreeSet<_>>().len() != STATE_COUNT {
        return Err("exactly four unique required state IDs are required".to_owned());
    }
    let input_domains = string_set(field(inputs, "domain_types", "input")?, "domain_types")?;
    let expected_domains: BTreeSet<String> =
        DOMAIN_TYPES.iter().map(|value| (*value).to_owned()).collect();
    if input_domains != expected_domains {
        return Err("input domain type set differs from six frozen domains".to_owned());
    }
    let max_order = field(inputs, "max_failure_order", "input")?
        .as_u64()
        .ok_or_else(|| "max_failure_order must be an integer".to_owned())?;
    if max_order != MAX_FAILURE_ORDER as u64 {
        return Err("v0.1 requires max_failure_order=2".to_owned());
    }
    let max_components = field(inputs, "max_components_per_layout", "input")?
        .as_u64()
        .ok_or_else(|| "max_components_per_layout must be a positive integer".to_owned())?
        as usize;
    let max_combinations = field(inputs, "max_combinations_per_layout", "input")?
        .as_u64()
        .ok_or_else(|| "max_combinations_per_layout must be a positive integer".to_owned())?
        as usize;
    if max_components == 0 || max_components > 64 || max_combinations == 0 || max_combinations > 10_000 {
        return Err("enumeration bounds are outside the safe v0.1 limit".to_owned());
    }

    ensure_keys(
        oracle,
        &["$schema", "schema_version", "benchmark_id", "layouts"],
        "oracle",
    )?;
    if string(oracle, "$schema", "oracle")?
        != "https://json-schema.org/draft/2020-12/schema"
        || string(oracle, "schema_version", "oracle")? != ORACLE_VERSION
        || string(oracle, "benchmark_id", "oracle")? != BENCHMARK_ID
    {
        return Err("oracle schema/version/benchmark identity mismatch".to_owned());
    }

    let layouts = field(inputs, "layouts", "input")?
        .as_array()
        .ok_or_else(|| "layouts must be an array".to_owned())?;
    let expected_layouts = field(oracle, "layouts", "oracle")?
        .as_array()
        .ok_or_else(|| "oracle.layouts must be an array".to_owned())?;
    if layouts.len() != LAYOUT_COUNT || expected_layouts.len() != LAYOUT_COUNT {
        return Err(format!("exactly {LAYOUT_COUNT} input/oracle layouts are required"));
    }

    let mut expected_by_id = BTreeMap::new();
    for expected in expected_layouts {
        let id = string(expected, "layout_id", "oracle layout")?.to_owned();
        if expected_by_id.insert(id.clone(), expected).is_some() {
            return Err(format!("duplicate oracle layout_id '{id}'"));
        }
    }
    let mut observed_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(LAYOUT_COUNT);
    let mut mismatches = Vec::new();
    for layout in layouts {
        let id = string(layout, "layout_id", "layout")?.to_owned();
        if !observed_ids.insert(id.clone()) {
            return Err(format!("duplicate input layout_id '{id}'"));
        }
        let actual = evaluate_layout(layout, &states, max_components, max_combinations)?;
        let expected = expected_by_id
            .get(&id)
            .ok_or_else(|| format!("input layout '{id}' missing from oracle"))?;
        if &actual != *expected {
            mismatches.push(json!({
                "layout_id": id,
                "expected": expected,
                "actual": actual.clone()
            }));
        }
        results.push(actual);
    }
    if observed_ids != expected_by_id.keys().cloned().collect() {
        return Err("input/oracle layout ID sets differ".to_owned());
    }
    let mismatch_count = mismatches.len();
    Ok(json!({
        "schema_version": "wdn-observability-survival-frontier-t1-evaluation-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "layout_count": LAYOUT_COUNT,
        "matched_layout_count": LAYOUT_COUNT - mismatch_count,
        "mismatch_count": mismatch_count,
        "mismatches": mismatches,
        "layout_results": results,
        "probability_interpretation": "none_unweighted_combinatorial_counts",
        "qualification_status": "NotRun",
        "claim_ceiling": "unweighted_combinatorial_sensor_layout_resilience_only"
    }))
}

fn read_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read '{path}': {error}"))?;
    let value = serde_json::from_slice(&bytes).map_err(|error| format!("invalid JSON '{path}': {error}"))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run(args: &[String]) -> Result<(), String> {
    if args.len() != 4 {
        return Err("usage: wdn-observability-survival-frontier-t1 INPUT_JSON INPUT_SCHEMA ORACLE_JSON RECEIPT_JSON".to_owned());
    }
    let (input_bytes, inputs) = read_json(&args[0])?;
    let (schema_bytes, schema) = read_json(&args[1])?;
    let (oracle_bytes, oracle) = read_json(&args[2])?;
    let evaluation = evaluate_all(&inputs, &schema, &oracle)?;
    let success = evaluation["layout_count"].as_u64() == Some(LAYOUT_COUNT as u64)
        && evaluation["matched_layout_count"].as_u64() == Some(LAYOUT_COUNT as u64)
        && evaluation["mismatch_count"].as_u64() == Some(0);

    let revision = env::var("WATER_OBSERVABILITY_SURVIVAL_FRONTIER_T1_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let manifest_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("Cargo.toml");
    let lock_bytes = fs::read(&lock_path).map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let manifest_bytes = fs::read(&manifest_path).map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let implementation = include_bytes!("wdn-observability-survival-frontier-t1.rs");
    let receipt = json!({
        "receipt_schema_version": "wdn-observability-survival-frontier-t1-receipt-v0.1",
        "benchmark_id": BENCHMARK_ID,
        "verification_state": if success && revision.is_some() { "PassedExactRevision" } else if success { "PassedLocalUnpinned" } else { "OracleMismatch" },
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "input_schema_sha256": sha256(&schema_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "package_manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "implementation_sha256": sha256(implementation),
        "layout_count": evaluation["layout_count"],
        "matched_layout_count": evaluation["matched_layout_count"],
        "mismatch_count": evaluation["mismatch_count"],
        "mismatches": evaluation["mismatches"],
        "probability_interpretation": "none_unweighted_combinatorial_counts",
        "claim_ceiling": "unweighted_combinatorial_sensor_layout_resilience_only"
    });
    let output = Path::new(&args[3]);
    if let Some(parent) = output.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| format!("create receipt directory: {error}"))?;
    }
    fs::write(output, serde_json::to_vec_pretty(&receipt).map_err(|error| format!("serialize receipt: {error}"))?)
        .map_err(|error| format!("write receipt '{}': {error}", output.display()))?;
    println!("{}", serde_json::to_string_pretty(&receipt).unwrap_or_default());
    if success { Ok(()) } else { Err("observability survival frontier did not match frozen oracle".to_owned()) }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if let Err(error) = run(&args) {
        eprintln!("wdn-observability-survival-frontier-t1: {error}");
        std::process::exit(1);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> (Value, Value, Value) {
        let input = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_observability_survival_frontier_input.v0.1.json"
        )).unwrap();
        let schema = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_observability_survival_frontier_input.schema.v0.1.json"
        )).unwrap();
        let oracle = serde_json::from_slice(include_bytes!(
            "../../../../docs/research/water-observability/wdn_observability_survival_frontier_oracle.v0.1.json"
        )).unwrap();
        (input, schema, oracle)
    }

    #[test]
    fn canonical_twelve_layouts_match_independent_oracle() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["layout_count"], 12);
        assert_eq!(result["matched_layout_count"], 12);
        assert_eq!(result["mismatch_count"], 0);
        assert_eq!(result["qualification_status"], "NotRun");
    }

    #[test]
    fn shared_power_domain_is_one_common_cause_component() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let layout = result["layout_results"].as_array().unwrap().iter()
            .find(|x| x["layout_id"] == "SURV-T1-002-shared-power-domain").unwrap();
        assert!(layout["domain_fanout"].as_array().unwrap().iter()
            .any(|x| x["domain_type"] == "power_supply" && x["domain_id"] == "PWR-N1-SHARED" && x["fanout"] == 2));
        let state = &layout["state_summaries"][0];
        assert_eq!(state["first_unobservable_order"], 1);
        assert!(state["minimal_cut_sets"].as_array().unwrap().iter().any(|x|
            x["components"][0] == "domain:power_supply:PWR-N1-SHARED"));
    }

    #[test]
    fn independent_two_sensor_backup_requires_two_component_cut() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let layout = result["layout_results"].as_array().unwrap().iter()
            .find(|x| x["layout_id"] == "SURV-T1-003-independent-pressure-acoustic-backup").unwrap();
        assert_eq!(layout["state_summaries"][0]["first_unobservable_order"], 2);
    }

    #[test]
    fn site_wide_clock_domain_is_a_singleton_cut_for_all_states() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let layout = result["layout_results"].as_array().unwrap().iter()
            .find(|x| x["layout_id"] == "SURV-T1-006-common-clock-high-fanout").unwrap();
        assert_eq!(layout["state_summaries"].as_array().unwrap().len(), 4);
        for state in layout["state_summaries"].as_array().unwrap() {
            assert_eq!(state["first_unobservable_order"], 1);
            assert!(state["minimal_cut_sets"].as_array().unwrap().iter()
                .any(|cut| cut["components"][0] == "domain:clock_domain:CLK-SITE-WIDE"));
        }
    }

    #[test]
    fn invalid_sensor_is_excluded_before_failure_combinations_are_counted() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        let stale = result["layout_results"].as_array().unwrap().iter()
            .find(|x| x["layout_id"] == "SURV-T1-009-stale-channel-excluded").unwrap();
        let base = result["layout_results"].as_array().unwrap().iter()
            .find(|x| x["layout_id"] == "SURV-T1-001-independent-single-channel").unwrap();
        assert_eq!(stale["component_count"], base["component_count"]);
        assert!(!stale["components"].as_array().unwrap().iter()
            .any(|x| x["component_id"] == "channel:STALE1"));
    }

    #[test]
    fn permuting_sensor_input_order_does_not_change_layout_result() {
        let (mut input, schema, oracle) = fixture();
        let first = evaluate_layout(&input["layouts"][0], &input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>(), 48, 2000).unwrap();
        let sensors = input["layouts"][0]["sensors"].as_array_mut().unwrap();
        sensors.reverse();
        let second = evaluate_layout(&input["layouts"][0], &input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>(), 48, 2000).unwrap();
        assert_eq!(first, second);
        let _ = (schema, oracle);
    }

    #[test]
    fn order_two_cut_sets_are_minimal_and_monotone() {
        let (input, schema, oracle) = fixture();
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        for layout in result["layout_results"].as_array().unwrap() {
            assert_eq!(layout["monotonicity_check"], "pass");
            assert_eq!(layout["monotonicity_violation_count"], 0);
            for state in layout["state_summaries"].as_array().unwrap() {
                let mut cutsets = Vec::new();
                for cut in state["minimal_cut_sets"].as_array().unwrap() {
                    let components = cut["components"].as_array().unwrap().iter().map(|x| x.as_str().unwrap()).collect::<Vec<_>>();
                    assert!(components.len() <= 2);
                    assert!(!cutsets.iter().any(|prior: &Vec<&str>| prior.iter().all(|x| components.contains(x))));
                    cutsets.push(components);
                }
            }
        }
    }

    #[test]
    fn duplicate_sensor_id_is_rejected() {
        let (mut input, schema, oracle) = fixture();
        let duplicate = input["layouts"][0]["sensors"][0].clone();
        input["layouts"][0]["sensors"].as_array_mut().unwrap().push(duplicate);
        let states = input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>();
        assert!(evaluate_layout(&input["layouts"][0], &states, 48, 2000).unwrap_err().contains("duplicate sensor id"));
        let _ = (schema, oracle);
    }

    #[test]
    fn missing_dependency_domain_is_rejected() {
        let (mut input, _, _) = fixture();
        input["layouts"][0]["sensors"][0]["failure_domains"].as_array_mut().unwrap().pop();
        let states = input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>();
        assert!(evaluate_layout(&input["layouts"][0], &states, 48, 2000).unwrap_err().contains("exactly six"));
    }

    #[test]
    fn duplicate_dependency_domain_type_is_rejected() {
        let (mut input, _, _) = fixture();
        input["layouts"][0]["sensors"][0]["failure_domains"][5]["domain_type"] = json!("clock_domain");
        let states = input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>();
        assert!(evaluate_layout(&input["layouts"][0], &states, 48, 2000).unwrap_err().contains("duplicate dependency domain type"));
    }

    #[test]
    fn state_binding_outside_required_set_is_rejected() {
        let (mut input, _, _) = fixture();
        input["layouts"][0]["sensors"][0]["state_id"] = json!("pressure:unknown");
        let states = input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>();
        assert!(evaluate_layout(&input["layouts"][0], &states, 48, 2000).unwrap_err().contains("not in the four required"));
    }

    #[test]
    fn component_budget_is_checked_before_enumeration() {
        let (input, _, _) = fixture();
        let states = input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>();
        assert!(evaluate_layout(&input["layouts"][0], &states, 10, 2000).unwrap_err().contains("exceeds configured cap"));
    }

    #[test]
    fn combination_budget_is_checked_before_enumeration() {
        let (input, _, _) = fixture();
        let states = input["required_state_ids"].as_array().unwrap().iter().map(|x| x.as_str().unwrap().to_owned()).collect::<Vec<_>>();
        assert!(evaluate_layout(&input["layouts"][0], &states, 48, 100).unwrap_err().contains("failure-combination count"));
    }

    #[test]
    fn invalid_failure_order_is_rejected() {
        let (mut input, schema, oracle) = fixture();
        input["max_failure_order"] = json!(3);
        assert!(evaluate_all(&input, &schema, &oracle).unwrap_err().contains("max_failure_order=2"));
    }

    #[test]
    fn altered_oracle_is_reported_as_mismatch() {
        let (input, schema, mut oracle) = fixture();
        oracle["layouts"][0]["monotonicity_violation_count"] = json!(1);
        let result = evaluate_all(&input, &schema, &oracle).unwrap();
        assert_eq!(result["matched_layout_count"], 11);
        assert_eq!(result["mismatch_count"], 1);
    }
}
