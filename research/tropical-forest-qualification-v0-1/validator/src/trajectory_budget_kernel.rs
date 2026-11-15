// SPDX-License-Identifier: Apache-2.0 OR MIT
//! Process-resolved forest trajectory closure and compensating-error diagnostics.
//!
//! This bounded two-pool reference is a benchmark oracle, not a forest process model.
//! It keeps live/deadwood transfers distinct from ecosystem-boundary inputs and losses.
//! A closed stock trajectory does not establish that its underlying fluxes are correct.

use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::fmt::Write as FmtWrite;

#[derive(Clone, Debug, PartialEq)]
struct Interval {
    period_index: u64,
    period_id: String,
    live_start: f64,
    deadwood_start: f64,
    live_end: f64,
    deadwood_end: f64,
    growth: f64,
    recruitment: f64,
    mortality_to_deadwood: f64,
    live_export: f64,
    decomposition: f64,
    deadwood_export: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Balance {
    live_residual: f64,
    deadwood_residual: f64,
    total_residual: f64,
}

#[derive(Clone, Debug, PartialEq)]
struct Trace {
    intervals: Vec<Interval>,
    balances: Vec<Balance>,
    maximum_absolute_balance_residual: f64,
}

fn sha256_hex(raw: &[u8]) -> String {
    let digest = Sha256::digest(raw);
    let mut output = String::with_capacity(64);
    for byte in digest {
        write!(&mut output, "{byte:02x}").expect("writing to String cannot fail");
    }
    output
}

fn string_at(value: &Value, key: &str) -> Result<String, String> {
    value.get(key).and_then(Value::as_str).map(ToOwned::to_owned)
        .ok_or_else(|| format!("{key} must be a string"))
}

fn number_at(value: &Value, key: &str) -> Result<f64, String> {
    let number = value.get(key).and_then(Value::as_f64)
        .ok_or_else(|| format!("{key} must be a number"))?;
    if !number.is_finite() {
        return Err(format!("{key} must be finite"));
    }
    Ok(number)
}

fn parse_interval(value: &Value, label: &str) -> Result<Interval, String> {
    let period_index = value.get("period_index").and_then(Value::as_u64)
        .ok_or_else(|| format!("{label}.period_index must be a non-negative integer"))?;
    let period_id = string_at(value, "period_id")?;
    if period_id.trim().is_empty() {
        return Err(format!("{label}.period_id must not be empty"));
    }
    let interval = Interval {
        period_index,
        period_id,
        live_start: number_at(value, "live_start")?,
        deadwood_start: number_at(value, "deadwood_start")?,
        live_end: number_at(value, "live_end")?,
        deadwood_end: number_at(value, "deadwood_end")?,
        growth: number_at(value, "growth")?,
        recruitment: number_at(value, "recruitment")?,
        mortality_to_deadwood: number_at(value, "mortality_to_deadwood")?,
        live_export: number_at(value, "live_export")?,
        decomposition: number_at(value, "decomposition")?,
        deadwood_export: number_at(value, "deadwood_export")?,
    };
    for (name, number) in [
        ("live_start", interval.live_start),
        ("deadwood_start", interval.deadwood_start),
        ("live_end", interval.live_end),
        ("deadwood_end", interval.deadwood_end),
        ("growth", interval.growth),
        ("recruitment", interval.recruitment),
        ("mortality_to_deadwood", interval.mortality_to_deadwood),
        ("live_export", interval.live_export),
        ("decomposition", interval.decomposition),
        ("deadwood_export", interval.deadwood_export),
    ] {
        if number < 0.0 {
            return Err(format!("{label}.{name} must be non-negative; unknown is not zero"));
        }
    }
    Ok(interval)
}

fn read_trace(
    case: &Value,
    key: &str,
    continuity_tolerance: f64,
) -> Result<Trace, String> {
    let rows = case.get(key).and_then(Value::as_array)
        .ok_or_else(|| format!("{key} must be a non-empty array"))?;
    if rows.is_empty() {
        return Err(format!("{key} must be a non-empty array"));
    }

    let mut intervals = Vec::with_capacity(rows.len());
    let mut ids = BTreeSet::new();
    let mut previous_index = None;
    let mut previous_end = None;
    for (index, row) in rows.iter().enumerate() {
        let interval = parse_interval(row, &format!("{key}[{index}]"))?;
        if !ids.insert(interval.period_id.clone()) {
            return Err(format!("{key} contains duplicate period_id {}", interval.period_id));
        }
        if previous_index.is_some_and(|prior| interval.period_index <= prior) {
            return Err(format!("{key} period_index must be strictly increasing"));
        }
        if let Some((prior_live, prior_deadwood)) = previous_end {
            if (interval.live_start - prior_live).abs() > continuity_tolerance
                || (interval.deadwood_start - prior_deadwood).abs() > continuity_tolerance
            {
                return Err(format!("{key} is discontinuous before period {}", interval.period_id));
            }
        }
        previous_index = Some(interval.period_index);
        previous_end = Some((interval.live_end, interval.deadwood_end));
        intervals.push(interval);
    }

    let mut balances = Vec::with_capacity(intervals.len());
    let mut maximum_absolute_balance_residual: f64 = 0.0;
    for interval in &intervals {
        // Mortality is an internal transfer: it leaves the live pool and enters deadwood.
        let expected_live = interval.live_start + interval.growth + interval.recruitment
            - interval.mortality_to_deadwood - interval.live_export;
        let expected_deadwood = interval.deadwood_start + interval.mortality_to_deadwood
            - interval.decomposition - interval.deadwood_export;
        let live_residual = interval.live_end - expected_live;
        let deadwood_residual = interval.deadwood_end - expected_deadwood;
        let expected_total = interval.live_start + interval.deadwood_start
            + interval.growth + interval.recruitment
            - interval.live_export - interval.decomposition - interval.deadwood_export;
        let total_residual = interval.live_end + interval.deadwood_end - expected_total;
        for residual in [live_residual, deadwood_residual, total_residual] {
            if !residual.is_finite() {
                return Err("non-finite pool or total budget residual".to_string());
            }
            maximum_absolute_balance_residual =
                maximum_absolute_balance_residual.max(residual.abs());
        }
        balances.push(Balance { live_residual, deadwood_residual, total_residual });
    }

    Ok(Trace { intervals, balances, maximum_absolute_balance_residual })
}

fn flow_values(interval: &Interval) -> [f64; 6] {
    [
        interval.growth,
        interval.recruitment,
        interval.mortality_to_deadwood,
        interval.live_export,
        interval.decomposition,
        interval.deadwood_export,
    ]
}

fn evaluate_case(
    case: &Value,
    stock_tolerance: f64,
    flow_tolerance: f64,
    budget_tolerance: f64,
    continuity_tolerance: f64,
) -> Result<Value, String> {
    let case_id = string_at(case, "case_id")?;
    let reference = read_trace(case, "reference_intervals", continuity_tolerance)?;
    let candidate = read_trace(case, "candidate_intervals", continuity_tolerance)?;
    if reference.intervals.len() != candidate.intervals.len() {
        return Err(format!("{case_id}: reference and candidate interval counts differ"));
    }
    if reference.maximum_absolute_balance_residual > budget_tolerance {
        return Err(format!("{case_id}: reference trace does not close its declared pool budgets"));
    }

    let mut maximum_stock_absolute_error: f64 = 0.0;
    let mut stock_absolute_error_sum = 0.0;
    let mut maximum_flow_absolute_error: f64 = 0.0;
    let mut flow_absolute_error_sum = 0.0;
    let mut cumulative_flow_absolute_errors = [0.0_f64; 6];
    let mut period_results = Vec::with_capacity(reference.intervals.len());

    for (index, (observed, predicted)) in reference.intervals.iter()
        .zip(candidate.intervals.iter()).enumerate()
    {
        if observed.period_index != predicted.period_index || observed.period_id != predicted.period_id {
            return Err(format!("{case_id}: candidate period identity/order differs from reference"));
        }
        let live_start_error = (observed.live_start - predicted.live_start).abs();
        let deadwood_start_error = (observed.deadwood_start - predicted.deadwood_start).abs();
        let live_end_error = (observed.live_end - predicted.live_end).abs();
        let deadwood_end_error = (observed.deadwood_end - predicted.deadwood_end).abs();
        maximum_stock_absolute_error = maximum_stock_absolute_error
            .max(live_start_error)
            .max(deadwood_start_error)
            .max(live_end_error)
            .max(deadwood_end_error);
        stock_absolute_error_sum +=
            live_start_error + deadwood_start_error + live_end_error + deadwood_end_error;

        let observed_flows = flow_values(observed);
        let predicted_flows = flow_values(predicted);
        let mut period_flow_errors = Vec::with_capacity(6);
        for (flow_index, (observed_flow, predicted_flow)) in
            observed_flows.iter().zip(predicted_flows.iter()).enumerate()
        {
            let error = (*observed_flow - *predicted_flow).abs();
            cumulative_flow_absolute_errors[flow_index] += error;
            maximum_flow_absolute_error = maximum_flow_absolute_error.max(error);
            flow_absolute_error_sum += error;
            period_flow_errors.push(error);
        }
        let balance = &candidate.balances[index];
        period_results.push(json!({
            "period_index": predicted.period_index,
            "period_id": predicted.period_id,
            "live_start_stock_absolute_error": live_start_error,
            "deadwood_start_stock_absolute_error": deadwood_start_error,
            "live_end_stock_absolute_error": live_end_error,
            "deadwood_end_stock_absolute_error": deadwood_end_error,
            "flow_absolute_errors": {
                "growth": period_flow_errors[0],
                "recruitment": period_flow_errors[1],
                "mortality_to_deadwood": period_flow_errors[2],
                "live_export": period_flow_errors[3],
                "decomposition": period_flow_errors[4],
                "deadwood_export": period_flow_errors[5]
            },
            "candidate_pool_balance_residuals": {
                "live": balance.live_residual,
                "deadwood": balance.deadwood_residual,
                "total": balance.total_residual
            }
        }));
    }

    let candidate_not_closed = candidate.maximum_absolute_balance_residual > budget_tolerance;
    let interval_count = u32::try_from(reference.intervals.len())
        .map_err(|_| format!("{case_id}: interval count exceeds u32"))?;
    let interval_count_f64 = f64::from(interval_count);
    let disposition = if candidate_not_closed {
        "BUDGET_NOT_CLOSED"
    } else if maximum_stock_absolute_error > stock_tolerance {
        "TRAJECTORY_STATE_MISMATCH"
    } else if maximum_flow_absolute_error > flow_tolerance {
        "STOCK_FIT_WITH_PROCESS_MISMATCH"
    } else {
        "PROCESS_AND_STATE_MATCH"
    };

    let expected = case.get("expected").ok_or_else(|| format!("{case_id}.expected is required"))?;
    let expected_disposition = string_at(expected, "disposition")?;
    let expected_stock_error = number_at(expected, "max_stock_absolute_error")?;
    let expected_flow_error = number_at(expected, "max_flow_absolute_error")?;
    let expected_budget_error = number_at(expected, "candidate_max_budget_absolute_residual")?;
    let expected_checks = disposition == expected_disposition
        && (maximum_stock_absolute_error - expected_stock_error).abs() <= 1e-12
        && (maximum_flow_absolute_error - expected_flow_error).abs() <= 1e-12
        && (candidate.maximum_absolute_balance_residual - expected_budget_error).abs() <= 1e-12;

    Ok(json!({
        "case_id": case_id,
        "disposition": disposition,
        "expected_disposition": expected_disposition,
        "oracle_matches": expected_checks,
        "interval_count": reference.intervals.len(),
        "maximum_stock_absolute_error": maximum_stock_absolute_error,
        "mean_stock_absolute_error_per_pool_boundary": stock_absolute_error_sum
            / (4.0 * interval_count_f64),
        "maximum_flow_absolute_error": maximum_flow_absolute_error,
        "mean_flow_absolute_error_per_flow_period": flow_absolute_error_sum
            / (6.0 * interval_count_f64),
        "cumulative_flow_absolute_errors": {
            "growth": cumulative_flow_absolute_errors[0],
            "recruitment": cumulative_flow_absolute_errors[1],
            "mortality_to_deadwood": cumulative_flow_absolute_errors[2],
            "live_export": cumulative_flow_absolute_errors[3],
            "decomposition": cumulative_flow_absolute_errors[4],
            "deadwood_export": cumulative_flow_absolute_errors[5]
        },
        "reference_maximum_absolute_budget_residual": reference.maximum_absolute_balance_residual,
        "candidate_maximum_absolute_budget_residual": candidate.maximum_absolute_balance_residual,
        "period_results": period_results
    }))
}

/// Validate a public synthetic process-trajectory corpus and emit a source-bound receipt.
/// The reference is deliberately two-pool and makes no empirical forest claim.
pub fn validate_corpus(raw: &[u8], manifest: &Value) -> Result<Value, String> {
    let schema_version = string_at(manifest, "schema_version")?;
    let benchmark_id = string_at(manifest, "benchmark_id")?;
    let corpus_id = string_at(manifest, "corpus_id")?;
    let claim_ceiling = string_at(manifest, "claim_ceiling")?;
    if schema_version != "0.1.0"
        || benchmark_id != "LUMINOUS-TROPICAL-FOREST-ECO-QUAL"
        || corpus_id != "FOREST-PROCESS-TRAJECTORY-001"
        || !claim_ceiling.contains("does not establish empirical ecological validation")
    {
        return Err("process-trajectory corpus identity or claim ceiling is invalid".to_string());
    }

    let tolerances = manifest.get("tolerances")
        .ok_or_else(|| "tolerances object is required".to_string())?;
    let stock_tolerance = number_at(tolerances, "stock_absolute")?;
    let flow_tolerance = number_at(tolerances, "flow_absolute")?;
    let budget_tolerance = number_at(tolerances, "budget_absolute")?;
    let continuity_tolerance = number_at(tolerances, "continuity_absolute")?;
    for (name, tolerance) in [
        ("stock_absolute", stock_tolerance),
        ("flow_absolute", flow_tolerance),
        ("budget_absolute", budget_tolerance),
        ("continuity_absolute", continuity_tolerance),
    ] {
        if tolerance < 0.0 {
            return Err(format!("tolerances.{name} must be non-negative"));
        }
    }

    let cases = manifest.get("cases").and_then(Value::as_array)
        .ok_or_else(|| "cases must be an array".to_string())?;
    if cases.is_empty() {
        return Err("cases must not be empty".to_string());
    }

    let mut ids = BTreeSet::new();
    let mut results = Vec::with_capacity(cases.len());
    let mut checks_passed = 0usize;
    for case in cases {
        let case_id = string_at(case, "case_id")?;
        if !ids.insert(case_id.clone()) {
            return Err(format!("duplicate case_id {case_id}"));
        }
        let result = evaluate_case(
            case, stock_tolerance, flow_tolerance, budget_tolerance, continuity_tolerance,
        )?;
        let passed = result["oracle_matches"].as_bool() == Some(true);
        if passed {
            checks_passed += 1;
        }
        results.push(result);
    }

    let passed = checks_passed == cases.len();
    Ok(json!({
        "receipt_version": "0.1.0",
        "validator": "symtropy-tropical-forest-qualifier",
        "benchmark_id": benchmark_id,
        "corpus_id": corpus_id,
        "source_manifest_sha256": sha256_hex(raw),
        "result": if passed { "PASS_SYNTHETIC_PROCESS_TRAJECTORY" } else { "FAIL" },
        "checks_passed": checks_passed,
        "checks_total": cases.len(),
        "case_results": results,
        "external_data_ingested": false,
        "ecological_validation": "NOT_ESTABLISHED",
        "blind_evaluation": "NOT_ESTABLISHED",
        "process_parameter_calibration": "NOT_ESTABLISHED",
        "units": "SYNTHETIC_BIOMASS_UNITS_PER_DECLARED_INTERVAL"
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const FIXTURE: &str = include_str!("../../cases/forest-process-trajectory-synthetic-v0.1.json");

    fn manifest() -> Value {
        serde_json::from_str(FIXTURE).expect("valid process-trajectory fixture")
    }

    #[test]
    fn corpus_exposes_stock_identical_compensating_process_errors() {
        let raw = FIXTURE.as_bytes();
        let receipt = validate_corpus(raw, &manifest()).expect("valid corpus");
        assert_eq!(receipt["result"], "PASS_SYNTHETIC_PROCESS_TRAJECTORY");
        let alias = receipt["case_results"].as_array().unwrap().iter()
            .find(|row| row["case_id"] == "COMPENSATING-FLUXES-SAME-STOCK-TRAJECTORY")
            .unwrap();
        assert_eq!(alias["disposition"], "STOCK_FIT_WITH_PROCESS_MISMATCH");
        assert_eq!(alias["maximum_stock_absolute_error"], 0.0);
        assert_eq!(alias["maximum_flow_absolute_error"], 20.0);
    }

    #[test]
    fn corpus_distinguishes_state_error_from_process_aliasing() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("valid corpus");
        let mismatch = receipt["case_results"].as_array().unwrap().iter()
            .find(|row| row["case_id"] == "STATE-TRAJECTORY-MISMATCH")
            .unwrap();
        assert_eq!(mismatch["disposition"], "TRAJECTORY_STATE_MISMATCH");
        assert_eq!(mismatch["maximum_stock_absolute_error"], 10.0);
    }

    #[test]
    fn corpus_detects_pool_budget_nonclosure() {
        let receipt = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("valid corpus");
        let mismatch = receipt["case_results"].as_array().unwrap().iter()
            .find(|row| row["case_id"] == "POOL-BUDGET-NOT-CLOSED")
            .unwrap();
        assert_eq!(mismatch["disposition"], "BUDGET_NOT_CLOSED");
        assert_eq!(mismatch["candidate_maximum_absolute_budget_residual"], 1.0);
    }

    #[test]
    fn corpus_rejects_mutated_process_error_oracle() {
        let mut value = manifest();
        value["cases"][1]["expected"]["max_flow_absolute_error"] = json!(0.0);
        let bytes = serde_json::to_vec(&value).expect("serialize mutated fixture");
        let receipt = validate_corpus(&bytes, &value).expect("valid structure returns receipt");
        assert_eq!(receipt["result"], "FAIL");
    }

    #[test]
    fn initial_census_stock_mismatch_is_not_hidden_by_matching_final_stock() {
        let mut value = manifest();
        value["cases"][0]["candidate_intervals"][0]["live_start"] = json!(110.0);
        value["cases"][0]["candidate_intervals"][0]["growth"] = json!(10.0);
        let bytes = serde_json::to_vec(&value).expect("serialize initial stock mismatch");
        let receipt = validate_corpus(&bytes, &value).expect("valid modified trace");
        let row = receipt["case_results"].as_array().unwrap().iter()
            .find(|row| row["case_id"] == "PROCESS-AND-STOCK-MATCH")
            .unwrap();
        assert_eq!(row["disposition"], "TRAJECTORY_STATE_MISMATCH");
        assert_eq!(row["maximum_stock_absolute_error"], 10.0);
    }

    #[test]
    fn negative_flux_is_rejected_not_treated_as_unknown_zero() {
        let mut value = manifest();
        value["cases"][0]["candidate_intervals"][0]["mortality_to_deadwood"] = json!(-1.0);
        let bytes = serde_json::to_vec(&value).expect("serialize negative flux");
        assert!(validate_corpus(&bytes, &value).is_err());
    }

    #[test]
    fn reordered_periods_are_rejected() {
        let mut value = manifest();
        value["cases"][0]["candidate_intervals"][1]["period_index"] = json!(0);
        let bytes = serde_json::to_vec(&value).expect("serialize reordered periods");
        assert!(validate_corpus(&bytes, &value).is_err());
    }

    #[test]
    fn discontinuous_pool_state_is_rejected() {
        let mut value = manifest();
        value["cases"][0]["candidate_intervals"][1]["live_start"] = json!(999.0);
        let bytes = serde_json::to_vec(&value).expect("serialize discontinuous trace");
        assert!(validate_corpus(&bytes, &value).is_err());
    }

    #[test]
    fn same_input_generates_identical_source_bound_receipt() {
        let first = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("first receipt");
        let second = validate_corpus(FIXTURE.as_bytes(), &manifest()).expect("second receipt");
        assert_eq!(first, second);
        assert_eq!(
            first["source_manifest_sha256"],
            sha256_hex(FIXTURE.as_bytes())
        );
    }
}
