// SPDX-License-Identifier: AGPL-3.0-or-later
//! Synthetic storage/autonomy sizing arithmetic for source-specific WaterLoop circuits.
//!
//! This evaluates a frozen demand/inflow trace. It does not select a vessel,
//! pump, pipe network, plumbing configuration or site-specific installation.

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::PathBuf;

const CASE_COUNT: usize = 13;
const SCHEMA_VERSION: &str = "waterloop-sizing-t0-v0.1";
const CLAIM_CEILING: &str =
    "synthetic_storage_sizing_and_pump_power_arithmetic_only_no_site_or_equipment_qualification";

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing field: {key}"))
}

fn text(value: &Value, key: &str) -> Result<String, String> {
    field(value, key)?
        .as_str()
        .map(str::to_owned)
        .filter(|text| !text.trim().is_empty())
        .ok_or_else(|| format!("{key}: expected non-empty string"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("{key}: expected finite number"))
}

fn boolean(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("{key}: expected boolean"))
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key}: expected array"))
}

fn numeric_array(value: &Value, key: &str) -> Result<Vec<f64>, String> {
    let items = array(value, key)?;
    if items.is_empty() {
        return Err(format!("{key}: expected at least one time step"));
    }
    items
        .iter()
        .enumerate()
        .map(|(index, item)| {
            item.as_f64()
                .filter(|number| number.is_finite())
                .ok_or_else(|| format!("{key}[{index}]: expected finite numeric volume"))
        })
        .collect()
}

fn nonnegative(value: f64, name: &str) -> Result<f64, String> {
    if value < 0.0 {
        Err(format!("{name}: must be non-negative"))
    } else {
        Ok(value)
    }
}

fn positive(value: f64, name: &str) -> Result<f64, String> {
    if value <= 0.0 {
        Err(format!("{name}: must be positive"))
    } else {
        Ok(value)
    }
}

#[derive(Debug)]
struct Simulation {
    delivered: Vec<f64>,
    unmet: Vec<f64>,
    overflow: Vec<f64>,
    end_storage: Vec<f64>,
    total_delivered: f64,
    total_unmet: f64,
    total_overflow: f64,
    feasible: bool,
}

fn simulate(
    capacity: f64,
    demand: &[f64],
    inflow: &[f64],
    tolerance: f64,
) -> Simulation {
    let mut storage = capacity;
    let mut delivered = Vec::with_capacity(demand.len());
    let mut unmet = Vec::with_capacity(demand.len());
    let mut overflow = Vec::with_capacity(demand.len());
    let mut end_storage = Vec::with_capacity(demand.len());

    for (required, admitted) in demand.iter().zip(inflow) {
        let available = storage + admitted;
        let spill = (available - capacity).max(0.0);
        let usable_before_demand = available.min(capacity).max(0.0);
        let served = required.min(usable_before_demand);
        let shortfall = (required - served).max(0.0);
        storage = (usable_before_demand - served).max(0.0);

        delivered.push(served);
        unmet.push(shortfall);
        overflow.push(spill);
        end_storage.push(storage);
    }

    let total_delivered = delivered.iter().sum::<f64>();
    let total_unmet = unmet.iter().sum::<f64>();
    let total_overflow = overflow.iter().sum::<f64>();

    Simulation {
        delivered,
        unmet,
        overflow,
        end_storage,
        total_delivered,
        total_unmet,
        total_overflow,
        feasible: total_unmet <= tolerance,
    }
}

fn compute_storage(input: &Value) -> Result<Value, String> {
    let demand_circuit = text(input, "demand_circuit_id")?;
    let storage_circuit = text(input, "storage_circuit_id")?;
    let demand_profile = text(input, "demand_use_profile_id")?;
    let storage_profile = text(input, "storage_use_profile_id")?;
    let demand_timebase = text(input, "demand_timebase_id")?;
    let inflow_timebase = text(input, "inflow_timebase_id")?;
    if demand_circuit != storage_circuit {
        return Ok(json!({
            "sizing_disposition": "REJECT",
            "reason": "CIRCUIT_IDENTITY_MISMATCH",
            "demand_circuit_id": demand_circuit,
            "storage_circuit_id": storage_circuit,
            "minimum_required_usable_capacity_m3": null,
            "gross_installed_volume_m3": null
        }));
    }
    if demand_profile != storage_profile {
        return Ok(json!({
            "sizing_disposition": "REJECT",
            "reason": "INTENDED_USE_PROFILE_MISMATCH",
            "demand_use_profile_id": demand_profile,
            "storage_use_profile_id": storage_profile,
            "minimum_required_usable_capacity_m3": null,
            "gross_installed_volume_m3": null
        }));
    }
    if demand_timebase != inflow_timebase {
        return Ok(json!({
            "sizing_disposition": "REJECT",
            "reason": "TIMEBASE_MISMATCH",
            "demand_timebase_id": demand_timebase,
            "inflow_timebase_id": inflow_timebase,
            "minimum_required_usable_capacity_m3": null,
            "gross_installed_volume_m3": null
        }));
    }

    let demand = numeric_array(input, "demand_m3_per_step")?;
    let raw_inflow = numeric_array(input, "raw_source_inflow_m3_per_step")?;
    if demand.len() != raw_inflow.len() {
        return Ok(json!({
            "sizing_disposition": "REJECT",
            "reason": "DEMAND_INFLOW_TRACE_LENGTH_MISMATCH",
            "demand_step_count": demand.len(),
            "inflow_step_count": raw_inflow.len(),
            "minimum_required_usable_capacity_m3": null,
            "gross_installed_volume_m3": null
        }));
    }
    for (index, value) in demand.iter().enumerate() {
        nonnegative(*value, &format!("demand_m3_per_step[{index}]"))?;
    }
    for (index, value) in raw_inflow.iter().enumerate() {
        nonnegative(*value, &format!("raw_source_inflow_m3_per_step[{index}]"))?;
    }

    let time_step_hours = positive(number(input, "time_step_hours")?, "time_step_hours")?;
    let tolerance = positive(number(input, "capacity_search_tolerance_m3")?, "capacity_search_tolerance_m3")?;
    let max_iterations = field(input, "max_search_iterations")?
        .as_u64()
        .filter(|value| *value > 0)
        .ok_or_else(|| "max_search_iterations: expected positive integer".to_owned())?
        as usize;

    let quality_admitted = boolean(input, "source_quality_admitted")?;
    let metrology_admitted = boolean(input, "source_measurement_admitted")?;
    let demand_admitted = boolean(input, "demand_measurement_admitted")?;
    if !demand_admitted {
        return Ok(json!({
            "sizing_disposition": "INDETERMINATE",
            "reason": "DEMAND_MEASUREMENT_NOT_ADMITTED",
            "raw_source_inflow_total_m3": raw_inflow.iter().sum::<f64>(),
            "admitted_source_inflow_total_m3": 0.0,
            "rejected_source_inflow_total_m3": raw_inflow.iter().sum::<f64>(),
            "minimum_required_usable_capacity_m3": null,
            "gross_installed_volume_m3": null
        }));
    }

    let source_admitted = quality_admitted && metrology_admitted;
    let admitted_inflow = if source_admitted {
        raw_inflow.clone()
    } else {
        vec![0.0; raw_inflow.len()]
    };
    let raw_total = raw_inflow.iter().sum::<f64>();
    let admitted_total = admitted_inflow.iter().sum::<f64>();
    let rejected_total = raw_total - admitted_total;
    let total_demand = demand.iter().sum::<f64>();

    let dead_volume = nonnegative(number(input, "dead_volume_m3")?, "dead_volume_m3")?;
    let freeboard = nonnegative(number(input, "freeboard_m3")?, "freeboard_m3")?;
    let uncertainty = nonnegative(number(input, "measurement_uncertainty_allowance_m3")?, "measurement_uncertainty_allowance_m3")?;
    let contingency = nonnegative(number(input, "contingency_allowance_m3")?, "contingency_allowance_m3")?;
    let declared_max = match input.get("max_usable_capacity_m3") {
        Some(Value::Null) | None => None,
        Some(_) => Some(nonnegative(number(input, "max_usable_capacity_m3")?, "max_usable_capacity_m3")?),
    };

    let mut iterations = 0_usize;
    let (minimum_capacity, search_gap) = if total_demand == 0.0 {
        (0.0, 0.0)
    } else {
        let mut lower = 0.0_f64;
        let mut upper = total_demand;
        if !simulate(upper, &demand, &admitted_inflow, tolerance).feasible {
            return Err("internal sizing invariant failed: total demand must be a feasible upper bound".to_owned());
        }
        while upper - lower > tolerance && iterations < max_iterations {
            let middle = (lower + upper) / 2.0;
            if simulate(middle, &demand, &admitted_inflow, tolerance).feasible {
                upper = middle;
            } else {
                lower = middle;
            }
            iterations += 1;
        }
        (upper, upper - lower)
    };

    let gross_volume = minimum_capacity + dead_volume + freeboard + uncertainty + contingency;
    let result = simulate(minimum_capacity, &demand, &admitted_inflow, tolerance);
    let search_converged = search_gap <= tolerance;
    let capacity_limit_failed = declared_max.is_some_and(|limit| minimum_capacity > limit + tolerance);
    let declared_max_trace = declared_max
        .map(|limit| simulate(limit, &demand, &admitted_inflow, tolerance));

    Ok(json!({
        "sizing_disposition": if !search_converged { "INDETERMINATE_SEARCH_TOLERANCE_NOT_REACHED" } else if capacity_limit_failed { "INFEASIBLE_DECLARED_CAPACITY_LIMIT" } else { "SIZED_WITHIN_DECLARED_TOLERANCE" },
        "reason": if !search_converged { "BINARY_SEARCH_ITERATION_LIMIT_REACHED" } else if capacity_limit_failed { "MINIMUM_REQUIRED_CAPACITY_EXCEEDS_DECLARED_MAXIMUM" } else if !source_admitted { "UNADMITTED_SOURCE_INFLOW_EXCLUDED_FROM_CREDIT" } else { "TIME_RESOLVED_ADMITTED_DEMAND_AND_INFLOW_TRACE" },
        "circuit_id": storage_circuit,
        "use_profile_id": storage_profile,
        "timebase_id": demand_timebase,
        "time_step_hours": time_step_hours,
        "horizon_hours": time_step_hours * demand.len() as f64,
        "time_step_count": demand.len(),
        "source_quality_admitted": quality_admitted,
        "source_measurement_admitted": metrology_admitted,
        "demand_measurement_admitted": demand_admitted,
        "raw_source_inflow_total_m3": raw_total,
        "admitted_source_inflow_total_m3": admitted_total,
        "rejected_source_inflow_total_m3": rejected_total,
        "total_demand_m3": total_demand,
        "minimum_required_usable_capacity_m3": minimum_capacity,
        "gross_installed_volume_m3": gross_volume,
        "dead_volume_m3": dead_volume,
        "freeboard_m3": freeboard,
        "measurement_uncertainty_allowance_m3": uncertainty,
        "contingency_allowance_m3": contingency,
        "capacity_search_tolerance_m3": tolerance,
        "capacity_search_gap_m3": search_gap,
        "search_iteration_count": iterations,
        "search_converged": search_converged,
        "delivered_m3_per_step": result.delivered,
        "unmet_m3_per_step": result.unmet,
        "overflow_m3_per_step": result.overflow,
        "end_storage_m3_per_step": result.end_storage,
        "total_delivered_m3": result.total_delivered,
        "total_unmet_m3": result.total_unmet,
        "total_overflow_m3": result.total_overflow,
        "minimum_capacity_feasible_within_tolerance": result.feasible,
        "declared_max_usable_capacity_m3": declared_max,
        "declared_max_capacity_unmet_m3": declared_max_trace.as_ref().map(|trace| trace.total_unmet),
        "declared_max_capacity_overflow_m3": declared_max_trace.as_ref().map(|trace| trace.total_overflow),
        "source_inflow_credit_disposition": if source_admitted { "ADMITTED" } else { "EXCLUDED" }
    }))
}

fn compute_pump_power(input: &Value) -> Result<Value, String> {
    let flow = positive(number(input, "design_flow_m3_per_hour")?, "design_flow_m3_per_hour")?;
    let head = positive(number(input, "total_dynamic_head_m")?, "total_dynamic_head_m")?;
    let density = positive(number(input, "water_density_kg_per_m3")?, "water_density_kg_per_m3")?;
    let gravity = positive(number(input, "gravity_m_per_s2")?, "gravity_m_per_s2")?;
    let efficiency_admitted = boolean(input, "pump_efficiency_evidence_admitted")?;
    let efficiency = number(input, "pump_efficiency")?;
    if !(0.0 < efficiency && efficiency <= 1.0) {
        return Ok(json!({
            "pump_power_disposition": "REJECT",
            "reason": "PUMP_EFFICIENCY_OUTSIDE_ZERO_ONE",
            "pump_efficiency": efficiency,
            "hydraulic_power_kw": null,
            "electrical_input_lower_bound_kw": null
        }));
    }
    let flow_m3_per_s = flow / 3600.0;
    let hydraulic_watts = density * gravity * flow_m3_per_s * head;
    let hydraulic_kw = hydraulic_watts / 1000.0;
    let electrical_lower_bound = if efficiency_admitted {
        Some(hydraulic_kw / efficiency)
    } else {
        None
    };
    Ok(json!({
        "pump_power_disposition": if efficiency_admitted { "ADMITTED_LOWER_BOUND_ARITHMETIC" } else { "HYDRAULIC_POWER_ONLY_EFFICIENCY_NOT_ADMITTED" },
        "reason": if efficiency_admitted { "DECLARED_EFFICIENCY_INPUT_ADMITTED" } else { "EFFICIENCY_EVIDENCE_NOT_ADMITTED" },
        "design_flow_m3_per_hour": flow,
        "flow_m3_per_second": flow_m3_per_s,
        "total_dynamic_head_m": head,
        "water_density_kg_per_m3": density,
        "gravity_m_per_s2": gravity,
        "pump_efficiency": efficiency,
        "pump_efficiency_evidence_admitted": efficiency_admitted,
        "hydraulic_power_kw": hydraulic_kw,
        "electrical_input_lower_bound_kw": electrical_lower_bound,
        "nonclaims": [
            "This result is an idealized hydraulic/electrical power lower-bound calculation only.",
            "It does not select a pump or establish pump curve, operating point, NPSH, transient, motor/drive, piping, control or field duty."
        ]
    }))
}

pub fn compute_case(case: &Value) -> Result<Value, String> {
    let case_id = text(case, "case_id")?;
    let inputs = field(case, "inputs")?;
    match case_id.as_str() {
        "WLS-T0-001-balanced-demand-inflow"
        | "WLS-T0-002-time-shifted-inflow"
        | "WLS-T0-003-unqualified-source-no-credit"
        | "WLS-T0-008-volume-allowances-separate"
        | "WLS-T0-009-declared-capacity-limit-infeasible"
        | "WLS-T0-010-zero-demand-zero-capacity" => compute_storage(inputs),
        "WLS-T0-004-circuit-mismatch-reject"
        | "WLS-T0-005-use-profile-mismatch-reject"
        | "WLS-T0-006-negative-demand-reject"
        | "WLS-T0-007-trace-length-mismatch-reject"
        | "WLS-T0-011-timebase-mismatch-reject"
        | "WLS-T0-013-unadmitted-demand-measurement" => compute_storage(inputs),
        "WLS-T0-012-pump-hydraulic-power-lower-bound"
        | "WLS-T0-014-pump-efficiency-invalid-reject" => compute_pump_power(inputs),
        _ => Err(format!("unsupported WaterLoop sizing case ID: {case_id}")),
    }
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(actual), Value::Number(expected)) => {
            match (actual.as_f64(), expected.as_f64()) {
                (Some(actual), Some(expected)) => {
                    let scale = actual.abs().max(expected.abs()).max(1.0);
                    (actual - expected).abs() <= 1.0e-9 * scale
                }
                _ => false,
            }
        }
        (Value::Array(actual), Value::Array(expected)) => {
            actual.len() == expected.len()
                && actual
                    .iter()
                    .zip(expected)
                    .all(|(actual, expected)| compare_json(actual, expected))
        }
        (Value::Object(actual), Value::Object(expected)) => {
            actual.len() == expected.len()
                && actual.iter().all(|(key, value)| {
                    expected
                        .get(key)
                        .is_some_and(|other| compare_json(value, other))
                })
        }
        _ => actual == expected,
    }
}

fn collect_ids(rows: &[Value], label: &str) -> Result<BTreeSet<String>, String> {
    let mut ids = BTreeSet::new();
    for row in rows {
        let id = text(row, "case_id")?;
        if !ids.insert(id.clone()) {
            return Err(format!("duplicate {label} case ID: {id}"));
        }
    }
    Ok(ids)
}

pub fn evaluate(inputs: &Value, manifest: &Value, oracle: &Value) -> Result<Value, String> {
    for (name, value) in [("inputs", inputs), ("manifest", manifest), ("oracle", oracle)] {
        if text(value, "schema_version")? != SCHEMA_VERSION
            || text(value, "evidence_class")? != "synthetic_only"
            || text(value, "claim_ceiling")? != CLAIM_CEILING
        {
            return Err(format!("{name}: schema/evidence/claim ceiling mismatch"));
        }
    }
    let cases = array(inputs, "cases")?;
    let expected = array(oracle, "cases")?;
    let input_ids = collect_ids(cases, "input")?;
    let oracle_ids = collect_ids(expected, "oracle")?;
    let manifest_ids = strings(manifest, "required_case_ids")?.into_iter().collect::<std::collections::BTreeSet<_>>();
    let count = field(manifest, "case_count")?.as_u64().unwrap_or(0) as usize;
    if count != CASE_COUNT
        || cases.len() != CASE_COUNT
        || expected.len() != CASE_COUNT
        || input_ids != manifest_ids
        || input_ids != oracle_ids
    {
        return Err(format!(
            "case identity/count mismatch: inputs={}, oracle={}, manifest={}, declared={count}",
            input_ids.len(), oracle_ids.len(), manifest_ids.len()
        ));
    }
    let mut oracle_by_id = std::collections::BTreeMap::new();
    for row in expected {
        oracle_by_id.insert(text(row, "case_id")?, row);
    }
    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
        let id = text(case, "case_id")?;
        let family = text(case, "family")?;
        let actual = compute_case(case)?;
        let expected_row = oracle_by_id.get(&id).ok_or_else(|| format!("oracle missing {id}"))?;
        let expected_family = text(expected_row, "family")?;
        let expected_value = field(expected_row, "expected")?;
        let matched = family == expected_family && compare_json(&actual, expected_value);
        if !matched {
            mismatches.push(json!({
                "case_id": id,
                "family": family,
                "expected_family": expected_family,
                "expected": expected_value,
                "computed": actual
            }));
        }
        results.push(json!({"case_id": id, "family": family, "matched": matched, "computed": actual}));
    }
    results.sort_by(|left, right| left["case_id"].as_str().cmp(&right["case_id"].as_str()));
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version": "waterloop-sizing-t0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() { "MatchedFrozenOracle" } else { "OracleMismatch" },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_only",
        "claim_ceiling": CLAIM_CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read_json(path: &std::path::Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes).map_err(|error| format!("parse {}: {error}", path.display()))?;
    Ok((bytes, value))
}

fn run() -> Result<i32, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() > 4 {
        return Err("usage: waterloop-sizing-t0 [INPUTS] [MANIFEST] [ORACLE] [RECEIPT]".to_owned());
    }
    let input_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "research/water-service-engineering/waterloop-sizing-t0-v0.1.json",
    ));
    let manifest_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "research/water-service-engineering/waterloop-sizing-manifest-v0.1.json",
    ));
    let oracle_path = PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
        "research/water-service-engineering/waterloop-sizing-oracle-v0.1.json",
    ));
    let receipt_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or(
        "artifacts/waterloop-sizing-t0/receipt.json",
    ));
    let (input_bytes, inputs) = read_json(&input_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let result = evaluate(&inputs, &manifest, &oracle)?;
    let count = result["result_count"].as_u64().unwrap_or(0);
    let matched = result["matched_case_count"].as_u64().unwrap_or(0);
    let success = count == CASE_COUNT as u64
        && matched == CASE_COUNT as u64
        && result["mismatches"].as_array().is_some_and(Vec::is_empty);
    let revision = env::var("WATERLOOP_SIZING_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path).map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let receipt = json!({
        "schema_version": "waterloop-sizing-t0-run-v0.1",
        "verification_state": if !success { "OracleMismatch" } else if revision.is_some() { "PassedExactRevision" } else { "PassedLocalUnpinned" },
        "implementation_revision": revision,
        "inputs_sha256": sha256(&input_bytes),
        "manifest_sha256": sha256(&manifest_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "result_count": count,
        "matched_case_count": matched,
        "mismatches": result["mismatches"].clone(),
        "results": result["results"].clone(),
        "evidence_class": "synthetic_only",
        "qualification_status": "NotRun",
        "claim_ceiling": CLAIM_CEILING
    });
    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&receipt).map_err(|error| format!("serialize receipt: {error}"))? + "\n";
    fs::write(&receipt_path, serialized.as_bytes()).map_err(|error| format!("write {}: {error}", receipt_path.display()))?;
    println!("{serialized}");
    Ok(if success { 0 } else { 2 })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATERLOOP_SIZING_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture(name: &str) -> Value {
        let raw = match name {
            "inputs" => include_str!("../../../../research/water-service-engineering/waterloop-sizing-t0-v0.1.json"),
            "manifest" => include_str!("../../../../research/water-service-engineering/waterloop-sizing-manifest-v0.1.json"),
            "oracle" => include_str!("../../../../research/water-service-engineering/waterloop-sizing-oracle-v0.1.json"),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(raw).expect("frozen sizing fixture parses")
    }

    #[test]
    fn all_thirteen_sizing_cases_match_separate_oracle() {
        let result = evaluate(&fixture("inputs"), &fixture("manifest"), &fixture("oracle")).expect("evaluation completes");
        assert_eq!(result["result_count"], CASE_COUNT);
        assert_eq!(result["matched_case_count"], CASE_COUNT);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
    }

    #[test]
    fn balanced_trace_computes_minimum_usable_and_gross_volume() {
        let out = compute_case(&fixture("inputs")["cases"][0]).expect("balanced storage trace");
        assert!((out["minimum_required_usable_capacity_m3"].as_f64().unwrap() - 0.55).abs() < 1e-7);
        assert!((out["gross_installed_volume_m3"].as_f64().unwrap() - 0.77).abs() < 1e-7);
    }

    #[test]
    fn unqualified_inflow_is_not_credited() {
        let out = compute_case(&fixture("inputs")["cases"][2]).expect("unqualified source trace");
        assert_eq!(out["admitted_source_inflow_total_m3"], 0.0);
        assert_eq!(out["rejected_source_inflow_total_m3"], 0.4);
        assert_eq!(out["minimum_required_usable_capacity_m3"], 0.2);
    }

    #[test]
    fn zero_demand_has_zero_usable_capacity() {
        let out = compute_case(&fixture("inputs")["cases"][9]).expect("zero demand");
        assert_eq!(out["minimum_required_usable_capacity_m3"], 0.0);
        assert_eq!(out["gross_installed_volume_m3"], 0.0);
    }

    #[test]
    fn pump_power_is_not_pump_selection() {
        let out = compute_case(&fixture("inputs")["cases"][11]).expect("pump power arithmetic");
        assert!((out["hydraulic_power_kw"].as_f64().unwrap() - 0.0588399).abs() < 1e-7);
        assert!((out["electrical_input_lower_bound_kw"].as_f64().unwrap() - 0.1069816).abs() < 1e-7);
    }
}
