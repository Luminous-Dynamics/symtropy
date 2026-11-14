// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROGRAMME: &str = "WATER-MIXING-FIDELITY-QUAL-001";
const CASE_COUNT: usize = 15;
const INPUT_CEILING: &str =
    "analytical_mass_routing_and_discrete_rtd_arithmetic_only_no_physical_mixing_qualification";

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
        .filter(|item| item.is_finite())
        .ok_or_else(|| format!("expected finite number at {key}"))
}

fn number_array(value: &Value, key: &str) -> Result<Vec<f64>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))?
        .iter()
        .map(|item| {
            item.as_f64()
                .filter(|number| number.is_finite())
                .ok_or_else(|| format!("{key} contains a non-finite/non-numeric value"))
        })
        .collect()
}

fn close(left: f64, right: f64) -> bool {
    (left - right).abs() <= 1.0e-9 * left.abs().max(right.abs()).max(1.0)
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right)) => {
            matches!(
                (left.as_f64(), right.as_f64()),
                (Some(a), Some(b)) if close(a, b)
            )
        }
        (Value::Array(left), Value::Array(right)) => {
            left.len() == right.len()
                && left.iter().zip(right).all(|(a, b)| compare_json(a, b))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right.get(key).is_some_and(|other| compare_json(value, other))
                })
        }
        _ => actual == expected,
    }
}

fn base() -> Value {
    json!({
        "qualification_status": "NotRun",
        "physical_mixing_qualified": false,
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn junction_refusal(disposition: &str, reason: &str) -> Value {
    let mut result = base();
    result["mixing_disposition"] = json!(disposition);
    result["reason"] = json!(reason);
    result
}

fn rtd_refusal(disposition: &str, reason: &str) -> Value {
    let mut result = base();
    result["rtd_disposition"] = json!(disposition);
    result["reason"] = json!(reason);
    result
}

fn compute_junction(inputs: &Value) -> Result<Value, String> {
    let topology = text(inputs, "topology")?;
    if !["cross", "tee", "double_tee_a", "double_tee_b"].contains(&topology) {
        return Ok(junction_refusal(
            "OUT_OF_DOMAIN",
            "JUNCTION_TOPOLOGY_NOT_IN_T0_DOMAIN",
        ));
    }

    let inlet_flows = number_array(inputs, "inlet_flow_l_s")?;
    let inlet_concentrations = number_array(inputs, "inlet_concentration_mg_l")?;
    let outlet_flows = number_array(inputs, "outlet_flow_l_s")?;
    let matrix = field(inputs, "inlet_mass_fraction_to_outlet")?
        .as_array()
        .ok_or_else(|| "inlet_mass_fraction_to_outlet must be an array".to_owned())?;

    if inlet_flows.is_empty()
        || outlet_flows.is_empty()
        || inlet_flows.len() != inlet_concentrations.len()
        || outlet_flows.len() != matrix.len()
    {
        return Ok(junction_refusal(
            "REJECTED_CONTRACT",
            "JUNCTION_ARRAY_DIMENSIONS_INVALID",
        ));
    }
    if inlet_flows.iter().any(|value| *value < 0.0)
        || outlet_flows.iter().any(|value| *value <= 0.0)
        || inlet_concentrations.iter().any(|value| *value < 0.0)
    {
        return Ok(junction_refusal("INVALID_INPUT", "NEGATIVE_OR_ZERO_FLOW_OR_CONCENTRATION"));
    }

    let mut fractions: Vec<Vec<f64>> = Vec::with_capacity(matrix.len());
    for row in matrix {
        let values = row
            .as_array()
            .ok_or_else(|| "each mass-fraction matrix row must be an array".to_owned())?
            .iter()
            .map(|item| {
                item.as_f64()
                    .filter(|number| number.is_finite())
                    .ok_or_else(|| "mass-fraction matrix contains a non-finite/non-numeric value".to_owned())
            })
            .collect::<Result<Vec<_>, _>>()?;
        if values.len() != inlet_flows.len() {
            return Ok(junction_refusal(
                "REJECTED_CONTRACT",
                "JUNCTION_MATRIX_DIMENSIONS_INVALID",
            ));
        }
        if values.iter().any(|value| *value < 0.0) {
            return Ok(junction_refusal("INVALID_INPUT", "NEGATIVE_MASS_FRACTION"));
        }
        fractions.push(values);
    }

    let inlet_total: f64 = inlet_flows.iter().sum();
    let outlet_total: f64 = outlet_flows.iter().sum();
    if inlet_total <= 0.0 || !close(inlet_total, outlet_total) {
        return Ok(junction_refusal(
            "REJECTED_CONTRACT",
            "TOTAL_INLET_OUTLET_FLOW_MISMATCH",
        ));
    }

    for inlet_index in 0..inlet_flows.len() {
        let column_sum: f64 = fractions.iter().map(|row| row[inlet_index]).sum();
        if !close(column_sum, 1.0) {
            return Ok(junction_refusal(
                "REJECTED_CONTRACT",
                "MASS_FRACTION_COLUMN_NOT_NORMALIZED",
            ));
        }
    }

    let inlet_mass_flux: Vec<f64> = inlet_flows
        .iter()
        .zip(&inlet_concentrations)
        .map(|(flow, concentration)| flow * concentration)
        .collect();
    let input_mass_flux: f64 = inlet_mass_flux.iter().sum();
    let output_mass_fluxes: Vec<f64> = fractions
        .iter()
        .map(|row| row.iter().zip(&inlet_mass_flux).map(|(p, mass)| p * mass).sum())
        .collect();
    let outlet_concentrations: Vec<f64> = output_mass_fluxes
        .iter()
        .zip(&outlet_flows)
        .map(|(mass_flux, flow)| mass_flux / flow)
        .collect();
    let output_mass_flux: f64 = output_mass_fluxes.iter().sum();
    let complete_mix_concentration = input_mass_flux / inlet_total;
    let complete_mix_reference = vec![complete_mix_concentration; outlet_flows.len()];
    let deviations: Vec<f64> = outlet_concentrations
        .iter()
        .map(|value| value - complete_mix_concentration)
        .collect();

    if !input_mass_flux.is_finite()
        || !output_mass_flux.is_finite()
        || outlet_concentrations.iter().any(|value| !value.is_finite())
    {
        return Ok(junction_refusal("INVALID_INPUT", "NON_FINITE_MASS_ROUTING_RESULT"));
    }

    let mut result = base();
    result["mixing_disposition"] = json!("ANALYTICAL_JUNCTION_OPERATOR_KNOWN_ANSWER");
    result["outlet_concentration_mg_l"] = json!(outlet_concentrations);
    result["complete_mix_reference_concentration_mg_l"] = json!(complete_mix_reference);
    result["outlet_deviation_from_complete_mix_mg_l"] = json!(deviations);
    result["input_tracer_mass_flux_mg_s"] = json!(input_mass_flux);
    result["output_tracer_mass_flux_mg_s"] = json!(output_mass_flux);
    result["mass_flux_residual_mg_s"] = json!(input_mass_flux - output_mass_flux);
    Ok(result)
}

fn compute_rtd(inputs: &Value) -> Result<Value, String> {
    let input_mass = number(inputs, "tracer_input_mass_mg")?;
    let early_threshold = number(inputs, "early_threshold_s")?;
    let tail_threshold = number(inputs, "tail_threshold_s")?;
    let low = number(inputs, "recovery_tolerance_low")?;
    let high = number(inputs, "recovery_tolerance_high")?;
    if input_mass <= 0.0 || early_threshold < 0.0 || tail_threshold <= early_threshold
        || low < 0.0 || high < low
    {
        return Ok(rtd_refusal("INVALID_INPUT", "RTD_INPUT_BOUNDS_INVALID"));
    }

    let samples = field(inputs, "samples")?
        .as_array()
        .ok_or_else(|| "samples must be an array".to_owned())?;
    if samples.len() < 2 {
        return Ok(rtd_refusal("INDETERMINATE", "TRACER_SAMPLE_COVERAGE_INSUFFICIENT"));
    }

    let mut previous_time = None;
    let mut sample_rows = Vec::with_capacity(samples.len());
    for sample in samples {
        let time = number(sample, "sample_time_s")?;
        let mass = number(sample, "tracer_mass_mg")?;
        if time < 0.0 || mass < 0.0 {
            return Ok(rtd_refusal("INVALID_INPUT", "NEGATIVE_TRACER_MASS"));
        }
        if previous_time.is_some_and(|previous| time <= previous) {
            return Ok(rtd_refusal(
                "INVALID_INPUT",
                "SAMPLE_TIMES_NOT_STRICTLY_INCREASING",
            ));
        }
        previous_time = Some(time);
        sample_rows.push((time, mass));
    }

    if text(inputs, "timebase_state")? != "aligned" {
        return Ok(rtd_refusal(
            "INDETERMINATE",
            "TRACER_TIMEBASE_NOT_ALIGNED",
        ));
    }
    if text(inputs, "calibration_state")? != "current" {
        return Ok(rtd_refusal(
            "INDETERMINATE",
            "TRACER_INSTRUMENT_CALIBRATION_NOT_CURRENT",
        ));
    }

    let recovered: f64 = sample_rows.iter().map(|(_, mass)| mass).sum();
    let recovery_fraction = recovered / input_mass;
    let unrecovered = input_mass - recovered;
    if recovery_fraction < low || recovery_fraction > high {
        let mut result = rtd_refusal(
            "INDETERMINATE",
            "TRACER_MASS_RECOVERY_OUTSIDE_TOLERANCE",
        );
        result["mass_recovery_fraction"] = json!(recovery_fraction);
        result["unrecovered_tracer_mass_mg"] = json!(unrecovered);
        result["recovered_tracer_mass_mg"] = json!(recovered);
        return Ok(result);
    }
    if recovered <= 0.0 {
        return Ok(rtd_refusal("INDETERMINATE", "NO_TRACER_MASS_RECOVERED"));
    }

    let mean: f64 = sample_rows.iter().map(|(time, mass)| time * mass).sum::<f64>() / recovered;
    let variance: f64 = sample_rows
        .iter()
        .map(|(time, mass)| mass * (time - mean).powi(2))
        .sum::<f64>()
        / recovered;
    let early_mass: f64 = sample_rows
        .iter()
        .filter(|(time, _)| *time <= early_threshold)
        .map(|(_, mass)| mass)
        .sum();
    let tail_mass: f64 = sample_rows
        .iter()
        .filter(|(time, _)| *time >= tail_threshold)
        .map(|(_, mass)| mass)
        .sum();

    let mut result = base();
    result["rtd_disposition"] = json!("RTD_ESTIMATED_SYNTHETIC_ONLY");
    result["mass_recovery_fraction"] = json!(recovery_fraction);
    result["unrecovered_tracer_mass_mg"] = json!(unrecovered);
    result["recovered_tracer_mass_mg"] = json!(recovered);
    result["mean_residence_time_s"] = json!(mean);
    result["variance_residence_time_s2"] = json!(variance);
    result["early_fraction"] = json!(early_mass / recovered);
    result["tail_fraction"] = json!(tail_mass / recovered);
    Ok(result)
}

fn compute(case: &Value) -> Result<Value, String> {
    let inputs = field(case, "inputs")?;
    match text(case, "case_kind")? {
        "junction" => compute_junction(inputs),
        "tank_rtd" => compute_rtd(inputs),
        other => Err(format!("unknown case kind: {other}")),
    }
}

fn case_id(case: &Value) -> Result<&str, String> {
    text(case, "case_id")
}

fn validate_contract(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    require_text(inputs, "schema_version", "water-mixing-fidelity-t0-input-v0.1")?;
    require_text(inputs, "programme_id", PROGRAMME)?;
    require_text(oracle, "schema_version", "water-mixing-fidelity-t0-oracle-v0.1")?;
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

    let manifest_ids: BTreeSet<String> = field(manifest, "scenario_ids")?
        .as_array()
        .ok_or_else(|| "manifest scenario_ids must be an array".to_owned())?
        .iter()
        .map(|value| value.as_str().map(str::to_owned)
            .ok_or_else(|| "manifest scenario id must be text".to_owned()))
        .collect::<Result<_, _>>()?;
    let mut input_ids = BTreeSet::new();
    let mut oracle_map = BTreeMap::new();

    for row in input_cases {
        let id = case_id(row)?.to_owned();
        require(input_ids.insert(id.clone()), &format!("duplicate input case: {id}"))?;
    }
    for row in oracle_cases {
        let id = case_id(row)?.to_owned();
        require(
            oracle_map.insert(id.clone(), row).is_none(),
            &format!("duplicate oracle case: {id}"),
        )?;
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
    require(input_ids == oracle_map.keys().cloned().collect(), "input/oracle case identity mismatch")?;
    Ok(())
}

fn require_text(value: &Value, key: &str, expected: &str) -> Result<(), String> {
    if text(value, key)? == expected {
        Ok(())
    } else {
        Err(format!("unexpected {key}"))
    }
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
            mismatches.push(json!({
                "case_id": id,
                "expected": expected_row,
                "computed": computed
            }));
        }
        results.push(json!({"case_id":id,"matched":matched,"computed":computed}));
    }
    results.sort_by(|left, right| left["case_id"].as_str().cmp(&right["case_id"].as_str()));
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version":"water-mixing-fidelity-t0-evaluation-v0.1",
        "verification_state": if mismatches.is_empty() {"MatchedFrozenOracle"} else {"OracleMismatch"},
        "qualification_status":"NotRun",
        "evidence_class":"synthetic_only",
        "evidence_ceiling":INPUT_CEILING,
        "result_count":results.len(),
        "matched_case_count":matched,
        "mismatches":mismatches,
        "results":results,
        "nonclaims":[
            "Synthetic arithmetic only; no junction or tank has been physically qualified.",
            "A transfer matrix is not inferred from geometry without calibrated observations.",
            "An RTD is not accepted outside its declared tracer mass-recovery tolerance.",
            "No result establishes contamination, potability, toxicology or consumer dose.",
            "This evaluator does not authorize treatment, isolation, restart or water release."
        ]
    }))
}

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes).iter().map(|byte| format!("{byte:02x}")).collect()
}

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value.bytes().all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err("usage: mixing-fidelity-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".to_owned());
    }
    let input_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/mixing-fidelity-t0-cases-v0.1.json",
    ));
    let oracle_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/mixing-fidelity-t0-oracle-v0.1.json",
    ));
    let manifest_path = PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/mixing-fidelity-manifest-v0.1.json",
    ));
    let receipt_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or(
        "artifacts/water-mixing-fidelity-t0/receipt.json",
    ));

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&inputs, &oracle, &manifest)?;
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let success = total == CASE_COUNT as u64 && matched == CASE_COUNT as u64 && no_mismatches;
    let revision = env::var("WATER_MIXING_FIDELITY_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;

    let receipt = json!({
        "schema_version":"water-mixing-fidelity-t0-run-v0.1",
        "verification_state": if !success {"OracleMismatch"} else if revision.is_some() {"PassedExactRevision"} else {"PassedLocalUnpinned"},
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
    if let Some(parent)=receipt_path.parent()
        && !parent.as_os_str().is_empty() {
        fs::create_dir_all(parent).map_err(|error|format!("create {}: {error}",parent.display()))?;
    }
    let serialized=serde_json::to_string_pretty(&receipt).map_err(|error|format!("serialize receipt: {error}"))?+"\n";
    fs::write(&receipt_path,serialized.as_bytes()).map_err(|error|format!("write {}: {error}",receipt_path.display()))?;
    println!("{serialized}");
    Ok(if success {0} else {2})
}

fn main() {
    match run() {
        Ok(code)=>std::process::exit(code),
        Err(error)=>{
            eprintln!("WATER_MIXING_FIDELITY_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name:&str)->Value {
        let source=match name {
            "inputs"=>include_str!("../../../../research/water-intrusion-coupling/mixing-fidelity-t0-cases-v0.1.json"),
            "oracle"=>include_str!("../../../../research/water-intrusion-coupling/mixing-fidelity-t0-oracle-v0.1.json"),
            "manifest"=>include_str!("../../../../research/water-intrusion-coupling/mixing-fidelity-manifest-v0.1.json"),
            _=>panic!("unknown fixture")
        };
        serde_json::from_str(source).expect("mixing fixture must be valid JSON")
    }
    fn evaluated()->Value {
        evaluate(&load("inputs"),&load("oracle"),&load("manifest"))
            .expect("mixing T0 evaluation must complete")
    }

    #[test]
    fn all_fifteen_mixing_cases_match_the_separate_frozen_oracle() {
        let result=evaluated();
        assert_eq!(result["result_count"],CASE_COUNT);
        assert_eq!(result["matched_case_count"],CASE_COUNT);
        assert_eq!(result["verification_state"],"MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn partial_mixing_preserves_mass_but_differs_from_complete_mixing() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="JMF-T0-003-partial-mixing-cross").unwrap();
        assert_eq!(row["computed"]["outlet_concentration_mg_l"],json!([7.5,2.5]));
        assert_eq!(row["computed"]["mass_flux_residual_mg_s"],0.0);
    }

    #[test]
    fn asymmetric_junction_flow_uses_outlet_specific_mass_routing() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="JMF-T0-004-asymmetric-flow-routing").unwrap();
        assert_eq!(row["computed"]["outlet_concentration_mg_l"],json!([11.25,3.75]));
    }

    #[test]
    fn tank_rtd_reports_early_arrival_and_tail_separately() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="RTD-T0-002-short-circuit-and-long-tail").unwrap();
        assert_eq!(row["computed"]["mean_residence_time_s"],150.5);
        assert_eq!(row["computed"]["early_fraction"],0.1);
        assert_eq!(row["computed"]["tail_fraction"],0.2);
    }

    #[test]
    fn incomplete_tracer_recovery_abstains_instead_of_renormalizing() {
        let result=evaluated();
        let row=result["results"].as_array().unwrap().iter()
            .find(|row|row["case_id"]=="RTD-T0-003-incomplete-tracer-recovery").unwrap();
        assert_eq!(row["computed"]["rtd_disposition"],"INDETERMINATE");
        assert_eq!(row["computed"]["mass_recovery_fraction"],0.6);
        assert!(row["computed"].get("mean_residence_time_s").is_none());
    }

    #[test]
    fn bad_timebase_and_expired_calibration_are_indeterminate() {
        let result=evaluated();
        for (id,reason) in [
            ("RTD-T0-006-timebase-not-aligned","TRACER_TIMEBASE_NOT_ALIGNED"),
            ("RTD-T0-007-calibration-not-current","TRACER_INSTRUMENT_CALIBRATION_NOT_CURRENT")
        ] {
            let row=result["results"].as_array().unwrap().iter().find(|row|row["case_id"]==id).unwrap();
            assert_eq!(row["computed"]["rtd_disposition"],"INDETERMINATE");
            assert_eq!(row["computed"]["reason"],reason);
        }
    }

    #[test]
    fn duplicate_input_case_identity_is_rejected() {
        let mut inputs=load("inputs");
        let duplicate=inputs["cases"][0].clone();
        inputs["cases"].as_array_mut().unwrap().push(duplicate);
        assert!(evaluate(&inputs,&load("oracle"),&load("manifest")).is_err());
    }

    #[test]
    fn oracle_cannot_claim_physical_qualification_or_authorize_release() {
        let mut oracle=load("oracle");
        oracle["cases"][0]["expected"]["release_authorized"]=json!(true);
        assert!(evaluate(&load("inputs"),&oracle,&load("manifest")).is_err());
    }
}
