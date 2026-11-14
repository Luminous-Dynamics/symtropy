// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROGRAMME: &str = "WATER-TRANSIENT-SOLVER-QUAL-001";
const PROFILE: &str = "LUMINOUS-WATER-TRANSIENT-T0-SYNTHETIC";
const INPUT_CEILING: &str =
    "analytical_arithmetic_and_applicability_only_no_solver_or_physical_qualification";
const ORACLE_CEILING: &str =
    "expected_analytical_contract_outputs_no_solver_or_physical_qualification";
const CASE_COUNT: usize = 10;

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing JSON field: {key}"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("expected string at {key}"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|v| v.is_finite())
        .ok_or_else(|| format!("expected finite number at {key}"))
}

fn boolean(value: &Value, key: &str) -> Result<bool, String> {
    field(value, key)?
        .as_bool()
        .ok_or_else(|| format!("expected boolean at {key}"))
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
}

fn close(left: f64, right: f64) -> bool {
    let scale = left.abs().max(right.abs()).max(1.0);
    (left - right).abs() <= 1.0e-9 * scale
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(a), Value::Number(b)) => {
            matches!((a.as_f64(), b.as_f64()), (Some(x), Some(y)) if close(x, y))
        }
        (Value::Array(a), Value::Array(b)) => {
            a.len() == b.len()
                && a.iter()
                    .zip(b)
                    .all(|(left, right)| compare_json(left, right))
        }
        (Value::Object(a), Value::Object(b)) => {
            a.len() == b.len()
                && a.iter()
                    .all(|(key, value)| b.get(key).is_some_and(|other| compare_json(value, other)))
        }
        _ => actual == expected,
    }
}

fn case_id(case: &Value) -> Result<&str, String> {
    text(case, "case_id")
}

fn wave_times(inputs: &Value) -> Result<(f64, f64), String> {
    let length = number(inputs, "pipe_length_m")?;
    let speed = number(inputs, "wave_speed_m_s")?;
    require(length > 0.0, "pipe length must be positive")?;
    require(speed > 0.0, "wave speed must be positive")?;
    let one_way = length / speed;
    Ok((one_way, 2.0 * one_way))
}

fn joukowsky(inputs: &Value) -> Result<(f64, f64), String> {
    let density = number(inputs, "density_kg_m3")?;
    let gravity = number(inputs, "gravity_m_s2")?;
    let speed = number(inputs, "wave_speed_m_s")?;
    let delta_v = number(inputs, "velocity_change_m_s")?;
    require(density > 0.0, "density must be positive")?;
    require(gravity > 0.0, "gravity must be positive")?;
    require(speed > 0.0, "wave speed must be positive")?;
    let pressure = density * speed * delta_v;
    let head = pressure / (density * gravity);
    Ok((pressure, head))
}

fn compute(case: &Value) -> Result<Value, String> {
    let id = case_id(case)?;
    let inputs = field(case, "inputs")?;

    match id {
        "WT-T0-001-rapid-closure-joukowsky-known-answer" => {
            let (one_way, round_trip) = wave_times(inputs)?;
            let closure = number(inputs, "closure_duration_s")?;
            let initial_pressure = number(inputs, "initial_absolute_pressure_pa")?;
            require(closure >= 0.0, "closure duration must be non-negative")?;
            require(initial_pressure > 0.0, "initial absolute pressure must be positive")?;
            let ratio = closure / round_trip;
            require(
                ratio < 1.0,
                "Joukowsky case requires closure shorter than round-trip wave time",
            )?;
            let (pressure_change, head_change) = joukowsky(inputs)?;
            Ok(json!({
                "disposition": "JOUKOWSKY_APPLICABLE_WITHIN_DECLARED_ASSUMPTIONS",
                "one_way_time_s": one_way,
                "round_trip_time_s": round_trip,
                "closure_to_round_trip_ratio": ratio,
                "pressure_change_pa": pressure_change,
                "head_change_m": head_change,
                "post_event_candidate_absolute_pressure_pa": initial_pressure + pressure_change,
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-002-one-way-and-round-trip-wave-time" => {
            let (one_way, round_trip) = wave_times(inputs)?;
            Ok(json!({
                "disposition": "ANALYTICAL_WAVE_TIME_ONLY",
                "one_way_time_s": one_way,
                "round_trip_time_s": round_trip,
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-003-negative-velocity-change-sign" => {
            let (_, round_trip) = wave_times(inputs)?;
            let closure = number(inputs, "closure_duration_s")?;
            require(closure >= 0.0, "closure duration must be non-negative")?;
            require(closure / round_trip < 1.0, "signed case requires rapid closure")?;
            let delta_v = number(inputs, "velocity_change_m_s")?;
            require(delta_v < 0.0, "case requires a negative velocity change")?;
            let (pressure_change, head_change) = joukowsky(inputs)?;
            Ok(json!({
                "disposition": "JOUKOWSKY_APPLICABLE_WITHIN_DECLARED_ASSUMPTIONS",
                "pressure_change_pa": pressure_change,
                "head_change_m": head_change,
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-004-slow-closure-outside-instantaneous-assumption" => {
            let (_, round_trip) = wave_times(inputs)?;
            let closure = number(inputs, "closure_duration_s")?;
            require(
                closure > round_trip,
                "case requires closure longer than round-trip wave time",
            )?;
            let ratio = closure / round_trip;
            Ok(json!({
                "disposition": "JOUKOWSKY_INSTANTANEOUS_FORMULA_NOT_APPLICABLE",
                "round_trip_time_s": round_trip,
                "closure_to_round_trip_ratio": ratio,
                "reason": "CLOSURE_NOT_FASTER_THAN_ROUND_TRIP_WAVE_TIME",
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-005-wave-speed-missing" => {
            require(
                field(inputs, "wave_speed_m_s")?.is_null(),
                "case requires missing wave speed",
            )?;
            Ok(json!({
                "disposition": "INDETERMINATE",
                "reason": "WAVE_SPEED_UNAVAILABLE",
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-006-single-phase-formula-crosses-vapour-pressure" => {
            let (_, round_trip) = wave_times(inputs)?;
            let closure = number(inputs, "closure_duration_s")?;
            require(closure / round_trip < 1.0, "case requires rapid closure")?;
            let initial_pressure = number(inputs, "initial_absolute_pressure_pa")?;
            let vapour_pressure = number(inputs, "vapour_pressure_pa")?;
            require(initial_pressure > 0.0, "initial absolute pressure must be positive")?;
            require(vapour_pressure > 0.0, "vapour pressure must be positive")?;
            let (pressure_change, _) = joukowsky(inputs)?;
            let candidate = initial_pressure + pressure_change;
            require(
                candidate < vapour_pressure,
                "case requires candidate pressure below vapour pressure",
            )?;
            Ok(json!({
                "disposition": "OUT_OF_DOMAIN",
                "reason": "SINGLE_PHASE_CANDIDATE_CROSSES_VAPOUR_PRESSURE",
                "candidate_post_event_absolute_pressure_pa": candidate,
                "candidate_is_not_a_physical_prediction": true,
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-007-closure-time-at-round-trip-boundary" => {
            let (_, round_trip) = wave_times(inputs)?;
            let closure = number(inputs, "closure_duration_s")?;
            require(close(closure, round_trip), "case requires closure at round-trip boundary")?;
            Ok(json!({
                "disposition": "BOUNDARY_REQUIRES_TRANSIENT_REFERENCE",
                "round_trip_time_s": round_trip,
                "closure_to_round_trip_ratio": closure / round_trip,
                "reason": "CLOSURE_EQUALS_ROUND_TRIP_WAVE_TIME",
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-008-invalid-negative-pipe-length" => {
            let length = number(inputs, "pipe_length_m")?;
            require(length < 0.0, "case requires negative pipe length")?;
            Ok(json!({
                "disposition": "INVALID_INPUT",
                "reason": "PIPE_LENGTH_NOT_POSITIVE",
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-009-wave-speed-uncertainty-envelope" => {
            let length = number(inputs, "pipe_length_m")?;
            let wave_min = number(inputs, "wave_speed_min_m_s")?;
            let wave_max = number(inputs, "wave_speed_max_m_s")?;
            let density = number(inputs, "density_kg_m3")?;
            let gravity = number(inputs, "gravity_m_s2")?;
            let velocity_change = number(inputs, "velocity_change_m_s")?;
            let closure = number(inputs, "closure_duration_s")?;
            let initial_pressure = number(inputs, "initial_absolute_pressure_pa")?;

            require(length > 0.0, "pipe length must be positive")?;
            require(
                wave_min > 0.0 && wave_max > 0.0,
                "wave-speed bounds must be positive",
            )?;
            require(
                wave_min <= wave_max,
                "wave-speed interval must be ordered",
            )?;
            require(density > 0.0 && gravity > 0.0, "fluid properties must be positive")?;
            require(velocity_change >= 0.0, "this envelope case requires non-negative velocity change")?;
            require(closure >= 0.0, "closure duration must be non-negative")?;
            require(initial_pressure > 0.0, "initial absolute pressure must be positive")?;

            let pressure_min = density * wave_min * velocity_change;
            let pressure_max = density * wave_max * velocity_change;
            let one_way_min = length / wave_max;
            let one_way_max = length / wave_min;
            let round_trip_min = 2.0 * one_way_min;
            let round_trip_max = 2.0 * one_way_max;
            let ratio_min = closure / round_trip_max;
            let ratio_max = closure / round_trip_min;

            require(
                ratio_max < 1.0,
                "closure must be rapid across the complete wave-speed interval",
            )?;

            Ok(json!({
                "disposition": "UNCERTAINTY_ENVELOPE_ONLY",
                "pressure_change_min_pa": pressure_min,
                "pressure_change_max_pa": pressure_max,
                "head_change_min_m": pressure_min / (density * gravity),
                "head_change_max_m": pressure_max / (density * gravity),
                "one_way_time_min_s": one_way_min,
                "one_way_time_max_s": one_way_max,
                "round_trip_time_min_s": round_trip_min,
                "round_trip_time_max_s": round_trip_max,
                "closure_to_round_trip_ratio_min": ratio_min,
                "closure_to_round_trip_ratio_max": ratio_max,
                "post_event_candidate_absolute_pressure_min_pa": initial_pressure + pressure_min,
                "post_event_candidate_absolute_pressure_max_pa": initial_pressure + pressure_max,
                "solver_qualification_permitted": false
            }))
        }
        "WT-T0-010-reversed-wave-speed-interval" => {
            let wave_min = number(inputs, "wave_speed_min_m_s")?;
            let wave_max = number(inputs, "wave_speed_max_m_s")?;
            require(wave_min > 0.0 && wave_max > 0.0, "wave-speed bounds must be positive")?;
            require(wave_min > wave_max, "case requires a reversed wave-speed interval")?;
            Ok(json!({
                "disposition": "INVALID_INPUT",
                "reason": "WAVE_SPEED_INTERVAL_NOT_ORDERED",
                "solver_qualification_permitted": false
            }))
        }
        _ => Err(format!("unrecognized water transient T0 case: {id}")),
    }
}

fn read_ids(value: &Value, key: &str) -> Result<Vec<String>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} contains a non-string ID"))
        })
        .collect()
}

fn case_ids(cases: &[Value]) -> Result<Vec<String>, String> {
    cases
        .iter()
        .map(|case| case_id(case).map(str::to_owned))
        .collect()
}

fn validate_ids(input: &[String], other: &[String], label: &str) -> Result<(), String> {
    let input_set: BTreeSet<&str> = input.iter().map(String::as_str).collect();
    let other_set: BTreeSet<&str> = other.iter().map(String::as_str).collect();
    require(input_set.len() == input.len(), "duplicate input case ID")?;
    require(other_set.len() == other.len(), "duplicate reference case ID")?;
    require(input_set == other_set, &format!("{label} case identities differ"))
}

fn validate_contract(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    require(text(inputs, "schema_version")? == "0.1.0", "input schema mismatch")?;
    require(text(inputs, "programme")? == PROGRAMME, "input programme mismatch")?;
    require(text(inputs, "evidence_class")? == "synthetic_only", "input evidence class mismatch")?;
    require(text(inputs, "evidence_ceiling")? == INPUT_CEILING, "input evidence ceiling mismatch")?;

    let profile = field(inputs, "profile")?;
    require(text(profile, "profile_id")? == PROFILE, "input profile mismatch")?;
    require(text(profile, "revision")? == "0.1", "input profile revision mismatch")?;
    require(boolean(profile, "all_values_are_fictional")?, "inputs must be declared fictional")?;

    let units = field(profile, "units")?;
    for (key, unit) in [
        ("length", "m"),
        ("wave_speed", "m/s"),
        ("velocity_change", "m/s"),
        ("density", "kg/m3"),
        ("pressure", "Pa_absolute"),
        ("pressure_change", "Pa"),
        ("head", "m_water"),
        ("time", "s"),
        ("gravity", "m/s2"),
    ] {
        require(text(units, key)? == unit, &format!("unit mismatch at {key}"))?;
    }

    require(text(oracle, "schema_version")? == "0.1.0", "oracle schema mismatch")?;
    require(text(oracle, "programme")? == PROGRAMME, "oracle programme mismatch")?;
    require(
        text(oracle, "evidence_class")? == "synthetic_expected_outputs_only",
        "oracle evidence class mismatch",
    )?;
    require(
        text(oracle, "evidence_ceiling")? == ORACLE_CEILING,
        "oracle evidence ceiling mismatch",
    )?;
    require(text(oracle, "input_profile_id")? == PROFILE, "oracle profile mismatch")?;

    require(text(manifest, "schema_version")? == "0.1.0", "manifest schema mismatch")?;
    require(text(manifest, "programme_id")? == PROGRAMME, "manifest programme mismatch")?;
    require(
        text(manifest, "evidence_class")? == "design_and_synthetic_only",
        "manifest evidence class mismatch",
    )?;
    require(
        text(manifest, "input_file")? == "water-transient-t0-cases-v0.1.json",
        "manifest input filename mismatch",
    )?;
    require(
        text(manifest, "oracle_file")? == "water-transient-t0-oracle-v0.1.json",
        "manifest oracle filename mismatch",
    )?;
    require(
        number(manifest, "scenario_count")? == CASE_COUNT as f64,
        "manifest case count mismatch",
    )?;
    require(
        text(manifest, "evidence_ceiling")?.contains("no_transient_solver"),
        "manifest evidence ceiling too permissive",
    )?;
    Ok(())
}

pub fn evaluate(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<Value, String> {
    validate_contract(inputs, oracle, manifest)?;
    let cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    require(cases.len() == CASE_COUNT, "input case count mismatch")?;
    require(oracle_cases.len() == CASE_COUNT, "oracle case count mismatch")?;

    let input_ids = case_ids(cases)?;
    let oracle_ids = case_ids(oracle_cases)?;
    let manifest_ids = read_ids(manifest, "scenario_ids")?;
    validate_ids(&input_ids, &oracle_ids, "oracle")?;
    validate_ids(&input_ids, &manifest_ids, "manifest")?;

    let mut expected_by_id = BTreeMap::new();
    for row in oracle_cases {
        let id = case_id(row)?.to_owned();
        let expected = field(row, "expected")?;
        require(
            !boolean(expected, "solver_qualification_permitted")?,
            "oracle may not permit solver qualification",
        )?;
        require(
            expected_by_id.insert(id.clone(), expected).is_none(),
            &format!("duplicate oracle ID: {id}"),
        )?;
    }

    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
        let id = case_id(case)?;
        let computed = compute(case)?;
        let expected = expected_by_id
            .get(id)
            .ok_or_else(|| format!("oracle missing case: {id}"))?;
        let matched = compare_json(&computed, expected);
        if !matched {
            mismatches.push(json!({
                "case_id": id,
                "expected": expected,
                "computed": computed
            }));
        }
        results.push(json!({
            "case_id": id,
            "matched": matched,
            "computed": computed
        }));
    }
    results.sort_by(|a, b| a["case_id"].as_str().cmp(&b["case_id"].as_str()));
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version": "water-transient-t0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {
            "MatchedFrozenOracle"
        } else {
            "OracleMismatch"
        },
        "qualification_status": "NotRun",
        "evidence_class": "synthetic_only",
        "evidence_ceiling": INPUT_CEILING,
        "result_count": results.len(),
        "matched_case_count": matched,
        "mismatches": mismatches,
        "results": results,
        "nonclaims": [
            "Analytical identities and applicability semantics only; no transient solver is qualified.",
            "No pressure-surge design, cavitation protection, pipe safety, intrusion, water-quality, health or regulatory claim is established.",
            "A Joukowsky arithmetic result is not a general valve-closure or network transient simulation.",
            "This evaluator does not authorize physical control, isolation, repair, restart or water release."
        ]
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err("usage: water-transient-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".to_owned());
    }

    let input_path = PathBuf::from(
        args.first().map(String::as_str).unwrap_or(
            "research/water-transient-solver-qualification/water-transient-t0-cases-v0.1.json",
        ),
    );
    let oracle_path = PathBuf::from(
        args.get(1).map(String::as_str).unwrap_or(
            "research/water-transient-solver-qualification/water-transient-t0-oracle-v0.1.json",
        ),
    );
    let manifest_path = PathBuf::from(
        args.get(2).map(String::as_str).unwrap_or(
            "research/water-transient-solver-qualification/manifest-v0.1.json",
        ),
    );
    let receipt_path = PathBuf::from(
        args.get(3).map(String::as_str).unwrap_or(
            "artifacts/water-transient-t0/receipt.json",
        ),
    );

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&inputs, &oracle, &manifest)?;
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let success = total == CASE_COUNT as u64 && matched == CASE_COUNT as u64 && no_mismatches;

    let raw_revision = env::var("WATER_TRANSIENT_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;

    let receipt = json!({
        "schema_version": "water-transient-t0-run-v0.1",
        "verification_state": if !success {
            "OracleMismatch"
        } else if revision.is_some() {
            "PassedExactRevision"
        } else {
            "PassedLocalUnpinned"
        },
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "input_sha256": sha256(&input_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "result_count": total,
        "matched_case_count": matched,
        "mismatches": evaluation["mismatches"].clone(),
        "results": evaluation["results"].clone(),
        "evidence_class": "synthetic_only",
        "evidence_ceiling": INPUT_CEILING,
        "nonclaims": evaluation["nonclaims"].clone()
    });

    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)
            .map_err(|error| format!("create {}: {error}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("serialize receipt: {error}"))?
        + "\n";
    fs::write(&receipt_path, serialized.as_bytes())
        .map_err(|error| format!("write {}: {error}", receipt_path.display()))?;
    println!("{serialized}");

    Ok(if success { 0 } else { 2 })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATER_TRANSIENT_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Value {
        let fixture = match name {
            "inputs" => include_str!(
                "../../../../research/water-transient-solver-qualification/water-transient-t0-cases-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-transient-solver-qualification/water-transient-t0-oracle-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-transient-solver-qualification/manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(fixture).expect("fixture must be valid JSON")
    }

    fn result() -> Value {
        evaluate(&load("inputs"), &load("oracle"), &load("manifest"))
            .expect("T0 evaluation must complete")
    }

    #[test]
    fn all_ten_cases_match_separate_oracle() {
        let result = result();
        assert_eq!(result["result_count"], CASE_COUNT);
        assert_eq!(result["matched_case_count"], CASE_COUNT);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn rapid_closure_matches_joukowsky_identity() {
        let result = result();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WT-T0-001-rapid-closure-joukowsky-known-answer")
            .unwrap();
        assert_eq!(row["computed"]["pressure_change_pa"], 500000.0);
        assert!(!row["computed"]["solver_qualification_permitted"].as_bool().unwrap());
    }

    #[test]
    fn slow_and_boundary_closures_do_not_use_instantaneous_shortcut() {
        let result = result();
        let cases = result["results"].as_array().unwrap();
        let slow = cases
            .iter()
            .find(|row| row["case_id"] == "WT-T0-004-slow-closure-outside-instantaneous-assumption")
            .unwrap();
        let boundary = cases
            .iter()
            .find(|row| row["case_id"] == "WT-T0-007-closure-time-at-round-trip-boundary")
            .unwrap();
        assert_eq!(
            slow["computed"]["disposition"],
            "JOUKOWSKY_INSTANTANEOUS_FORMULA_NOT_APPLICABLE"
        );
        assert_eq!(
            boundary["computed"]["disposition"],
            "BOUNDARY_REQUIRES_TRANSIENT_REFERENCE"
        );
    }

    #[test]
    fn missing_wave_speed_is_indeterminate() {
        let result = result();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WT-T0-005-wave-speed-missing")
            .unwrap();
        assert_eq!(row["computed"]["disposition"], "INDETERMINATE");
    }

    #[test]
    fn vapour_pressure_crossing_is_out_of_domain() {
        let result = result();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WT-T0-006-single-phase-formula-crosses-vapour-pressure")
            .unwrap();
        assert_eq!(row["computed"]["disposition"], "OUT_OF_DOMAIN");
        assert!(row["computed"]["candidate_is_not_a_physical_prediction"]
            .as_bool()
            .unwrap());
    }

    #[test]
    fn invalid_geometry_is_not_calculated() {
        let result = result();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WT-T0-008-invalid-negative-pipe-length")
            .unwrap();
        assert_eq!(row["computed"]["disposition"], "INVALID_INPUT");
    }

    #[test]
    fn wave_speed_uncertainty_propagates_to_pressure_and_timing_bounds() {
        let result = result();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WT-T0-009-wave-speed-uncertainty-envelope")
            .unwrap();
        assert_eq!(row["computed"]["pressure_change_min_pa"], 450000.0);
        assert_eq!(row["computed"]["pressure_change_max_pa"], 550000.0);
        assert!(
            row["computed"]["one_way_time_min_s"].as_f64().unwrap()
                < row["computed"]["one_way_time_max_s"].as_f64().unwrap()
        );
        assert!(!row["computed"]["solver_qualification_permitted"].as_bool().unwrap());
    }

    #[test]
    fn reversed_wave_speed_interval_is_rejected() {
        let result = result();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WT-T0-010-reversed-wave-speed-interval")
            .unwrap();
        assert_eq!(row["computed"]["disposition"], "INVALID_INPUT");
    }

    #[test]
    fn altered_oracle_cannot_authorize_solver_qualification() {
        let mut oracle = load("oracle");
        oracle["cases"][0]["expected"]["solver_qualification_permitted"] = json!(true);
        assert!(evaluate(&load("inputs"), &oracle, &load("manifest")).is_err());
    }
}
