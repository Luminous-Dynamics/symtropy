// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROGRAMME: &str = "WATER-MIXING-FIDELITY-QUAL-001";
const CASE_COUNT: usize = 10;
const INPUT_CEILING: &str =
    "linear_excitation_rank_and_measurement_observability_checks_only_no_physical_mixing_qualification";

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value.get(key).ok_or_else(|| format!("missing JSON field: {key}"))
}

fn text<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("expected string at {key}"))
}

fn usize_field(value: &Value, key: &str) -> Result<usize, String> {
    field(value, key)?
        .as_u64()
        .and_then(|number| usize::try_from(number).ok())
        .ok_or_else(|| format!("expected non-negative integer at {key}"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|item| item.is_finite())
        .ok_or_else(|| format!("expected finite number at {key}"))
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition { Ok(()) } else { Err(message.to_owned()) }
}

fn base() -> Value {
    json!({
        "qualification_status": "NotRun",
        "physical_mixing_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn disposition(status: &str, reason: Option<&str>) -> Value {
    let mut result = base();
    result["design_disposition"] = json!(status);
    if let Some(reason) = reason {
        result["reason"] = json!(reason);
    }
    result
}

fn matrix_rank(matrix: &[Vec<f64>], relative_tolerance: f64) -> usize {
    let rows = matrix.len();
    let columns = matrix.first().map_or(0, Vec::len);
    if rows == 0 || columns == 0 {
        return 0;
    }
    let scale = matrix
        .iter()
        .flatten()
        .map(|value| value.abs())
        .fold(0.0_f64, f64::max)
        .max(1.0);
    let threshold = relative_tolerance * scale;
    let mut reduced = matrix.to_vec();
    let mut pivot_row = 0;

    for column in 0..columns {
        if pivot_row >= rows {
            break;
        }
        let pivot = (pivot_row..rows)
            .max_by(|left, right| {
                reduced[*left][column]
                    .abs()
                    .total_cmp(&reduced[*right][column].abs())
            })
            .unwrap_or(pivot_row);
        if reduced[pivot][column].abs() <= threshold {
            continue;
        }
        reduced.swap(pivot_row, pivot);
        let pivot_value = reduced[pivot_row][column];
        for value in &mut reduced[pivot_row][column..] {
            *value /= pivot_value;
        }

        for row in 0..rows {
            if row == pivot_row {
                continue;
            }
            let factor = reduced[row][column];
            if factor.abs() <= threshold {
                continue;
            }
            for col in column..columns {
                reduced[row][col] -= factor * reduced[pivot_row][col];
            }
        }
        pivot_row += 1;
    }
    pivot_row
}

fn compute_junction(inputs: &Value) -> Result<Value, String> {
    let inlet_count = usize_field(inputs, "inlet_count")?;
    let outlet_count = usize_field(inputs, "outlet_count")?;
    if inlet_count == 0 || outlet_count == 0 {
        return Ok(disposition("INVALID_INPUT", Some("INLET_OR_OUTLET_COUNT_ZERO")));
    }

    let matrix_value = field(inputs, "inlet_excitation_matrix_mg_s")?
        .as_array()
        .ok_or_else(|| "inlet_excitation_matrix_mg_s must be an array".to_owned())?;
    if matrix_value.len() != inlet_count {
        return Ok(disposition("REJECTED_CONTRACT", Some("EXCITATION_MATRIX_ROW_COUNT_MISMATCH")));
    }

    let mut matrix = Vec::with_capacity(inlet_count);
    let mut experiment_count = None;
    for row in matrix_value {
        let values = row
            .as_array()
            .ok_or_else(|| "excitation matrix row must be an array".to_owned())?
            .iter()
            .map(|value| {
                value
                    .as_f64()
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| "excitation matrix contains a non-finite/non-numeric value".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if let Some(expected) = experiment_count {
            if values.len() != expected {
                return Ok(disposition("REJECTED_CONTRACT", Some("EXCITATION_MATRIX_NOT_RECTANGULAR")));
            }
        } else {
            experiment_count = Some(values.len());
        }
        if values.iter().any(|value| *value < 0.0) {
            return Ok(disposition("INVALID_INPUT", Some("NEGATIVE_INLET_EXCITATION")));
        }
        matrix.push(values);
    }

    if experiment_count.unwrap_or(0) == 0 {
        return Ok(disposition("INVALID_INPUT", Some("NO_EXCITATION_EXPERIMENTS")));
    }

    let tolerance = number(inputs, "rank_relative_tolerance")?;
    if !(0.0..1.0).contains(&tolerance) {
        return Ok(disposition("INVALID_INPUT", Some("RANK_TOLERANCE_OUT_OF_RANGE")));
    }

    let sensor_indices = field(inputs, "outlet_sensor_indices")?
        .as_array()
        .ok_or_else(|| "outlet_sensor_indices must be an array".to_owned())?
        .iter()
        .map(|value| {
            value
                .as_u64()
                .and_then(|number| usize::try_from(number).ok())
                .ok_or_else(|| "outlet sensor index must be a non-negative integer".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut observed = BTreeSet::new();
    for index in sensor_indices {
        if index >= outlet_count {
            return Ok(disposition("REJECTED_CONTRACT", Some("OUTLET_SENSOR_INDEX_OUT_OF_RANGE")));
        }
        if !observed.insert(index) {
            return Ok(disposition("REJECTED_CONTRACT", Some("DUPLICATE_OUTLET_SENSOR_INDEX")));
        }
    }

    let rank = matrix_rank(&matrix, tolerance);
    let all_outlets_observed = observed.len() == outlet_count
        && (0..outlet_count).all(|index| observed.contains(&index));
    let parameter_count = inlet_count.saturating_mul(outlet_count.saturating_sub(1));

    let mut result;
    if text(inputs, "inlet_measurement_state")? != "measured" {
        result = disposition("INDETERMINATE", Some("INLET_TRACER_MASS_FLUX_NOT_MEASURED"));
    } else if !all_outlets_observed {
        result = disposition("INDETERMINATE", Some("OUTLET_OBSERVABILITY_INCOMPLETE"));
    } else if rank < inlet_count {
        result = disposition("UNIDENTIFIABLE", Some("UNIDENTIFIABLE_EXCITATION_RANK"));
    } else {
        result = disposition("IDENTIFIABLE_IN_T0_ARITHMETIC", None);
    }
    result["input_rank"] = json!(rank);
    result["required_input_rank"] = json!(inlet_count);
    result["all_outlets_observed"] = json!(all_outlets_observed);
    result["free_transfer_parameters"] = json!(parameter_count);
    Ok(result)
}

fn compute_tank(inputs: &Value) -> Result<Value, String> {
    if text(inputs, "input_waveform_state")? != "measured" {
        return Ok(disposition("INDETERMINATE", Some("RTD_INPUT_WAVEFORM_NOT_MEASURED")));
    }
    if text(inputs, "outlet_tracer_series_state")? != "measured" {
        return Ok(disposition("INDETERMINATE", Some("RTD_OUTLET_TRACER_SERIES_NOT_MEASURED")));
    }
    if text(inputs, "timebase_state")? != "aligned" {
        return Ok(disposition("INDETERMINATE", Some("RTD_TIMEBASE_NOT_ALIGNED")));
    }
    Ok(disposition("IDENTIFIABLE_IN_T0_ARITHMETIC", None))
}

fn case_id(case: &Value) -> Result<&str, String> {
    text(case, "case_id")
}

fn compute(case: &Value) -> Result<Value, String> {
    let inputs = field(case, "inputs")?;
    match text(case, "design_kind")? {
        "junction_operator" => compute_junction(inputs),
        "tank_rtd" => compute_tank(inputs),
        other => Err(format!("unknown design kind: {other}")),
    }
}

fn require_text(value: &Value, key: &str, expected: &str) -> Result<(), String> {
    if text(value, key)? == expected {
        Ok(())
    } else {
        Err(format!("unexpected {key}"))
    }
}

fn validate_contract(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    require_text(inputs, "schema_version", "mixing-identifiability-t0-input-v0.1")?;
    require_text(inputs, "programme_id", PROGRAMME)?;
    require_text(oracle, "schema_version", "mixing-identifiability-t0-oracle-v0.1")?;
    require_text(oracle, "programme_id", PROGRAMME)?;
    require_text(manifest, "programme_id", PROGRAMME)?;

    let input_cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    require(input_cases.len() == CASE_COUNT, "input case count mismatch")?;
    require(oracle_cases.len() == CASE_COUNT, "oracle case count mismatch")?;
    require(
        field(manifest, "scenario_count")?.as_u64() == Some(CASE_COUNT as u64),
        "manifest scenario count mismatch",
    )?;

    let manifest_rows = field(manifest, "scenario_ids")?
        .as_array()
        .ok_or_else(|| "manifest scenario_ids must be an array".to_owned())?;
    let manifest_ids: BTreeSet<String> = manifest_rows
        .iter()
        .map(|value| {
            value.as_str().map(str::to_owned)
                .ok_or_else(|| "manifest scenario id must be text".to_owned())
        })
        .collect::<Result<_, _>>()?;
    require(manifest_rows.len() == manifest_ids.len(), "duplicate manifest case identity")?;

    let mut input_ids = BTreeSet::new();
    let mut oracle_map = BTreeMap::new();
    for row in input_cases {
        let id = case_id(row)?.to_owned();
        require(input_ids.insert(id.clone()), &format!("duplicate input case: {id}"))?;
    }
    for row in oracle_cases {
        let id = case_id(row)?.to_owned();
        require(oracle_map.insert(id.clone(), row).is_none(), &format!("duplicate oracle case: {id}"))?;
        let expected = field(row, "expected")?;
        require(
            field(expected, "physical_mixing_qualified")?.as_bool() == Some(false),
            "oracle cannot claim physical mixing qualification",
        )?;
        require(
            field(expected, "potability_claim_permitted")?.as_bool() == Some(false),
            "oracle cannot permit potability",
        )?;
        require(
            field(expected, "release_authorized")?.as_bool() == Some(false),
            "oracle cannot authorize release",
        )?;
    }
    require(input_ids == manifest_ids, "input cases differ from manifest case identity")?;
    require(
        input_ids == oracle_map.keys().cloned().collect::<BTreeSet<_>>(),
        "input/oracle case identity mismatch",
    )?;
    Ok(())
}

pub fn evaluate(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<Value, String> {
    validate_contract(inputs, oracle, manifest)?;
    let input_cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    let expected: BTreeMap<String, Value> = oracle_cases
        .iter()
        .map(|row| Ok((case_id(row)?.to_owned(), field(row, "expected")?.clone())))
        .collect::<Result<_, String>>()?;

    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in input_cases {
        let id = case_id(case)?;
        let computed = compute(case)?;
        let expected_row = expected
            .get(id)
            .ok_or_else(|| format!("oracle missing case: {id}"))?;
        let matched = compare_json(&computed, expected_row);
        if !matched {
            mismatches.push(json!({"case_id":id,"expected":expected_row,"computed":computed}));
        }
        results.push(json!({"case_id":id,"matched":matched,"computed":computed}));
    }
    results.sort_by(|left, right| left["case_id"].as_str().cmp(&right["case_id"].as_str()));
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version":"mixing-identifiability-t0-evaluation-v0.1",
        "verification_state":if mismatches.is_empty() {"MatchedFrozenOracle"} else {"OracleMismatch"},
        "qualification_status":"NotRun",
        "evidence_class":"synthetic_only",
        "evidence_ceiling":INPUT_CEILING,
        "result_count":results.len(),
        "matched_case_count":matched,
        "mismatches":mismatches,
        "results":results,
        "nonclaims":[
            "This evaluator checks excitation rank and observation preconditions only.",
            "A full-rank design does not establish physical validity or an accurate fitted operator.",
            "Noise, conditioning, sensor response, time alignment and mass recovery require separate qualification.",
            "No result establishes potability or authorizes release."
        ]
    }))
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right)) => {
            matches!(
                (left.as_f64(), right.as_f64()),
                (Some(a), Some(b)) if (a-b).abs() <= 1.0e-9*a.abs().max(b.abs()).max(1.0)
            )
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len()==right.len() && left.iter().zip(right).all(|(a,b)|compare_json(a,b))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len()==right.len() && left.iter().all(|(key,value)|right.get(key).is_some_and(|other|compare_json(value,other)))
        }
        _=>actual==expected
    }
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte|format!("{byte:02x}")).collect()
}

fn read_json(path: &Path) -> Result<(Vec<u8>,Value),String> {
    let bytes=fs::read(path).map_err(|error|format!("read {}: {error}",path.display()))?;
    let value=serde_json::from_slice(&bytes).map_err(|error|format!("parse {}: {error}",path.display()))?;
    Ok((bytes,value))
}

fn valid_revision(value:&str)->bool {
    value.len()==40 && value.bytes().all(|byte|byte.is_ascii_digit()||(b'a'..=b'f').contains(&byte))
}

fn run()->Result<i32,String> {
    let args:Vec<String>=env::args().skip(1).collect();
    if args.len()>4 {
        return Err("usage: mixing-identifiability-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".to_owned());
    }
    let input_path=PathBuf::from(args.first().map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-identifiability-t0-cases-v0.1.json"));
    let oracle_path=PathBuf::from(args.get(1).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-identifiability-t0-oracle-v0.1.json"));
    let manifest_path=PathBuf::from(args.get(2).map(String::as_str).unwrap_or("research/water-intrusion-coupling/mixing-identifiability-manifest-v0.1.json"));
    let receipt_path=PathBuf::from(args.get(3).map(String::as_str).unwrap_or("artifacts/water-mixing-identifiability-t0/receipt.json"));

    let (input_bytes,inputs)=read_json(&input_path)?;
    let (oracle_bytes,oracle)=read_json(&oracle_path)?;
    let (manifest_bytes,manifest)=read_json(&manifest_path)?;
    let evaluation=evaluate(&inputs,&oracle,&manifest)?;
    let total=evaluation["result_count"].as_u64().unwrap_or(0);
    let matched=evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let no_mismatches=evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let success=total==CASE_COUNT as u64&&matched==CASE_COUNT as u64&&no_mismatches;
    let revision=env::var("WATER_MIXING_IDENTIFIABILITY_T0_REVISION")
        .or_else(|_|env::var("GITHUB_SHA")).ok().filter(|value|valid_revision(value));
    let lock_path=PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes=fs::read(&lock_path).map_err(|error|format!("read {}: {error}",lock_path.display()))?;

    let receipt=json!({
        "schema_version":"mixing-identifiability-t0-run-v0.1",
        "verification_state":if !success {"OracleMismatch"} else if revision.is_some() {"PassedExactRevision"} else {"PassedLocalUnpinned"},
        "qualification_status":"NotRun",
        "implementation_revision":revision,
        "input_sha256":sha256(&input_bytes),
        "oracle_sha256":sha256(&oracle_bytes),
        "manifest_sha256":sha256(&manifest_bytes),
        "cargo_lock_sha256":sha256(&lock_bytes),
        "result_count":total,
        "matched_case_count":matched,
        "mismatches":evaluation["mismatches"].clone(),
        "results":evaluation["results"].clone(),
        "evidence_class":"synthetic_only",
        "evidence_ceiling":INPUT_CEILING,
        "nonclaims":evaluation["nonclaims"].clone()
    });
    if let Some(parent)=receipt_path.parent() && !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent).map_err(|error|format!("create {}: {error}",parent.display()))?;
    }
    let serialized=serde_json::to_string_pretty(&receipt).map_err(|error|format!("serialize receipt: {error}"))?+"\n";
    fs::write(&receipt_path,serialized.as_bytes()).map_err(|error|format!("write {}: {error}",receipt_path.display()))?;
    println!("{serialized}");
    Ok(if success {0}else{2})
}

fn main() {
    match run() {
        Ok(code)=>std::process::exit(code),
        Err(error)=>{
            eprintln!("WATER_MIXING_IDENTIFIABILITY_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name:&str)->Value {
        let source=match name {
            "inputs"=>include_str!("../../../../research/water-intrusion-coupling/mixing-identifiability-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/mixing-identifiability-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/mixing-identifiability-manifest-v0.1.json"),
            _=>panic!("unknown fixture")
        };
        serde_json::from_str(source).expect("identifiability fixtures must be valid JSON")
    }

    fn evaluated()->Value {
        evaluate(&load("inputs"),&load("oracle"),&load("manifest"))
            .expect("identifiability T0 evaluation must complete")
    }

    #[test]
    fn all_ten_identifiability_cases_match_the_frozen_oracle() {
        let result=evaluated();
        assert_eq!(result["result_count"],CASE_COUNT);
        assert_eq!(result["matched_case_count"],CASE_COUNT);
        assert_eq!(result["verification_state"],"MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn orthogonal_inlet_excitation_identifies_the_t0_transfer_matrix() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="MID-T0-001-two-inlet-orthogonal-excitation-full-rank").unwrap();
        assert_eq!(row["computed"]["design_disposition"],"IDENTIFIABLE_IN_T0_ARITHMETIC");
        assert_eq!(row["computed"]["input_rank"],2);
    }

    #[test]
    fn equal_and_repeated_proportional_excitation_remain_unidentifiable() {
        let result=evaluated();
        for id in [
            "MID-T0-002-equal-simultaneous-excitation-rank-deficient",
            "MID-T0-010-rank-deficient-excitation-with-more-repeats"
        ] {
            let row=result["results"].as_array().unwrap().iter().find(|row|row["case_id"]==id).unwrap();
            assert_eq!(row["computed"]["design_disposition"],"UNIDENTIFIABLE");
            assert_eq!(row["computed"]["input_rank"],1);
        }
    }

    #[test]
    fn near_collinear_excitation_is_rejected_by_declared_rank_threshold() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="MID-T0-003-near-collinear-excitation-below-rank-threshold").unwrap();
        assert_eq!(row["computed"]["design_disposition"],"UNIDENTIFIABLE");
        assert_eq!(row["computed"]["input_rank"],1);
    }

    #[test]
    fn incomplete_outlet_and_unmeasured_inlet_inputs_cannot_pass_preflight() {
        let result=evaluated();
        for (id,reason) in [
            ("MID-T0-004-missing-outlet-observation","OUTLET_OBSERVABILITY_INCOMPLETE"),
            ("MID-T0-005-inlet-tracer-mass-flux-not-measured","INLET_TRACER_MASS_FLUX_NOT_MEASURED")
        ] {
            let row=result["results"].as_array().unwrap().iter().find(|row|row["case_id"]==id).unwrap();
            assert_eq!(row["computed"]["design_disposition"],"INDETERMINATE");
            assert_eq!(row["computed"]["reason"],reason);
        }
    }

    #[test]
    fn tank_rtd_without_measured_input_waveform_is_indeterminate() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="MID-T0-006-tank-input-waveform-not-measured").unwrap();
        assert_eq!(row["computed"]["design_disposition"],"INDETERMINATE");
        assert_eq!(row["computed"]["reason"],"RTD_INPUT_WAVEFORM_NOT_MEASURED");
    }

    #[test]
    fn duplicate_outlet_sensor_indices_are_rejected() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="MID-T0-007-duplicate-outlet-sensor-index").unwrap();
        assert_eq!(row["computed"]["design_disposition"],"REJECTED_CONTRACT");
    }

    #[test]
    fn negative_excitation_is_invalid() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="MID-T0-008-negative-inlet-excitation").unwrap();
        assert_eq!(row["computed"]["design_disposition"],"INVALID_INPUT");
    }

    #[test]
    fn three_inlet_orthogonal_excitation_scales_rank_requirement() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="MID-T0-009-three-inlet-orthogonal-excitation").unwrap();
        assert_eq!(row["computed"]["design_disposition"],"IDENTIFIABLE_IN_T0_ARITHMETIC");
        assert_eq!(row["computed"]["required_input_rank"],3);
        assert_eq!(row["computed"]["free_transfer_parameters"],3);
    }

    #[test]
    fn oracle_cannot_claim_physical_qualification_or_authorize_release() {
        let mut oracle=load("oracle");
        oracle["cases"][0]["expected"]["release_authorized"]=json!(true);
        assert!(evaluate(&load("inputs"),&oracle,&load("manifest")).is_err());
    }
}
