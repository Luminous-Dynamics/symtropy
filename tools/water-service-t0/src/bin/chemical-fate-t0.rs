// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROGRAMME: &str = "WATER-INTRUSION-CHEMICAL-FATE-T0";
const PROFILE: &str = "LUMINOUS-WATER-CHEMICAL-FATE-T0";
const INPUT_CEILING: &str =
    "analytical_reaction_and_model_scope_contracts_only_no_physical_chemical_qualification";
const CASE_COUNT: usize = 12;

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
        .filter(|value| value.is_finite())
        .ok_or_else(|| format!("expected finite number at {key}"))
}

fn require(condition: bool, message: &str) -> Result<(), String> {
    if condition {
        Ok(())
    } else {
        Err(message.to_owned())
    }
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
                && left
                    .iter()
                    .zip(right)
                    .all(|(a, b)| compare_json(a, b))
        }
        (Value::Object(left), Value::Object(right)) => {
            left.len() == right.len()
                && left.iter().all(|(key, value)| {
                    right
                        .get(key)
                        .is_some_and(|other| compare_json(value, other))
                })
        }
        _ => actual == expected,
    }
}

fn case_id(case: &Value) -> Result<&str, String> {
    text(case, "case_id")
}

fn invalid(reason: &str) -> Value {
    json!({
        "chemical_disposition": "INVALID_INPUT",
        "reason": reason,
        "qualification_status": "NotRun",
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn indeterminate(reason: &str) -> Value {
    json!({
        "chemical_disposition": "INDETERMINATE",
        "reason": reason,
        "qualification_status": "NotRun",
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn unsupported(reason: &str) -> Value {
    json!({
        "chemical_disposition": "UNSUPPORTED_MODEL_SCOPE",
        "reason": reason,
        "qualification_status": "NotRun",
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn compute(case: &Value) -> Result<Value, String> {
    let inputs = field(case, "inputs")?;
    match case_id(case)? {
        "CF-T0-001-first-order-decay-known-answer" => {
            let c0 = number(inputs, "initial_parent_concentration_mg_l")?;
            let rate = number(inputs, "first_order_rate_h_inv")?;
            let time = number(inputs, "elapsed_time_h")?;
            if c0 < 0.0 || rate < 0.0 || time < 0.0 {
                return Ok(invalid("NEGATIVE_CONCENTRATION_OR_RATE"));
            }
            let remaining = c0 * (-rate * time).exp();
            let transformed = c0 - remaining;
            Ok(json!({
                "chemical_disposition": "ANALYTICAL_FIRST_ORDER_KNOWN_ANSWER",
                "parent_remaining_concentration_mg_l": remaining,
                "parent_transformed_concentration_mg_l": transformed,
                "qualification_status": "NotRun",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "CF-T0-002-sequential-reaction-unequal-rate-constants" => {
            let initial = number(inputs, "initial_A_mg_l")?;
            let k1 = number(inputs, "k1_h_inv")?;
            let k2 = number(inputs, "k2_h_inv")?;
            let time = number(inputs, "elapsed_time_h")?;
            if initial < 0.0 || k1 < 0.0 || k2 < 0.0 || time < 0.0 {
                return Ok(invalid("NEGATIVE_CONCENTRATION_OR_RATE"));
            }
            require(!close(k1, k2), "case requires unequal rate constants")?;
            require(k2 > 0.0, "second rate constant must be positive")?;
            let a = initial * (-k1 * time).exp();
            let b = initial * k1 / (k2 - k1)
                * ((-k1 * time).exp() - (-k2 * time).exp());
            let c = initial - a - b;
            require(
                a >= -1.0e-10 && b >= -1.0e-10 && c >= -1.0e-10,
                "sequential reaction solution produced a negative species concentration",
            )?;
            Ok(json!({
                "chemical_disposition": "ANALYTICAL_SEQUENTIAL_REACTION_KNOWN_ANSWER",
                "A_mg_l": a,
                "B_mg_l": b,
                "C_mg_l": c,
                "conserved_moiety_total_mg_l": a + b + c,
                "qualification_status": "NotRun",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "CF-T0-003-sequential-reaction-equal-rate-limit" => {
            let initial = number(inputs, "initial_A_mg_l")?;
            let k1 = number(inputs, "k1_h_inv")?;
            let k2 = number(inputs, "k2_h_inv")?;
            let time = number(inputs, "elapsed_time_h")?;
            if initial < 0.0 || k1 < 0.0 || k2 < 0.0 || time < 0.0 {
                return Ok(invalid("NEGATIVE_CONCENTRATION_OR_RATE"));
            }
            require(close(k1, k2), "case requires equal rate constants")?;
            let rate = 0.5 * (k1 + k2);
            let survival = (-rate * time).exp();
            let a = initial * survival;
            let b = initial * rate * time * survival;
            let c = initial - a - b;
            require(
                a >= -1.0e-10 && b >= -1.0e-10 && c >= -1.0e-10,
                "equal-rate sequential solution produced negative concentration",
            )?;
            Ok(json!({
                "chemical_disposition": "ANALYTICAL_SEQUENTIAL_REACTION_EQUAL_RATE_LIMIT",
                "A_mg_l": a,
                "B_mg_l": b,
                "C_mg_l": c,
                "conserved_moiety_total_mg_l": a + b + c,
                "qualification_status": "NotRun",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "CF-T0-004-langmuir-bulk-wall-equilibrium" => {
            let concentration = number(inputs, "bulk_concentration_mg_l")?;
            let maximum = number(inputs, "maximum_surface_loading_mg_m2")?;
            let equilibrium = number(inputs, "adsorption_equilibrium_l_mg")?;
            if concentration < 0.0 || maximum < 0.0 || equilibrium < 0.0 {
                return Ok(invalid("NEGATIVE_CONCENTRATION_OR_EQUILIBRIUM_PARAMETER"));
            }
            let denominator = 1.0 + equilibrium * concentration;
            require(denominator > 0.0, "Langmuir denominator must be positive")?;
            let loading = maximum * equilibrium * concentration / denominator;
            Ok(json!({
                "chemical_disposition": "ANALYTICAL_LANGMUIR_EQUILIBRIUM_KNOWN_ANSWER",
                "surface_loading_mg_m2": loading,
                "qualification_status": "NotRun",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "CF-T0-005-conserved-moiety-ledger-complete" => {
            let initial = number(inputs, "initial_moiety_mass_mg")?;
            let parent = number(inputs, "remaining_parent_moiety_mass_mg")?;
            let products = number(inputs, "measured_product_moiety_mass_mg")?;
            let wall = number(inputs, "wall_bound_moiety_mass_mg")?;
            let other = number(inputs, "other_measured_moiety_mass_mg")?;
            if [initial, parent, products, wall, other]
                .iter()
                .any(|value| *value < 0.0)
            {
                return Ok(invalid("NEGATIVE_MOIETY_MASS"));
            }
            let recovered = parent + products + wall + other;
            let residual = initial - recovered;
            require(
                residual.abs() <= 1.0e-9 * initial.abs().max(1.0),
                "case requires a closed conserved-moiety ledger",
            )?;
            Ok(json!({
                "mass_balance_disposition": "MOIETY_BALANCE_CLOSED_WITHIN_DECLARED_ARITHMETIC_TOLERANCE",
                "initial_moiety_mass_mg": initial,
                "recovered_moiety_mass_mg": recovered,
                "residual_moiety_mass_mg": residual,
                "qualification_status": "NotRun",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "CF-T0-006-missing-residual-stream-does-not-force-closure" => {
            let initial = number(inputs, "initial_moiety_mass_mg")?;
            let parent = number(inputs, "remaining_parent_moiety_mass_mg")?;
            let products = number(inputs, "measured_product_moiety_mass_mg")?;
            let wall = number(inputs, "wall_bound_moiety_mass_mg")?;
            require(
                initial >= 0.0 && parent >= 0.0 && products >= 0.0 && wall >= 0.0,
                "known mass terms must be non-negative",
            )?;
            require(
                field(inputs, "other_measured_moiety_mass_mg")?.is_null(),
                "case requires an unmeasured residual stream",
            )?;
            let measured = parent + products + wall;
            let unresolved = initial - measured;
            require(unresolved >= 0.0, "measured moiety mass exceeds initial mass")?;
            Ok(json!({
                "mass_balance_disposition": "MASS_BALANCE_UNRESOLVED",
                "reason": "DECLARED_RESIDUAL_STREAM_UNMEASURED",
                "measured_moiety_mass_mg": measured,
                "initial_moiety_mass_mg": initial,
                "unresolved_moiety_mass_mg": unresolved,
                "qualification_status": "NotRun",
                "potability_claim_permitted": false,
                "release_authorized": false
            }))
        }
        "CF-T0-007-missing-water-matrix-temperature-and-ph" => {
            let matrix = field(inputs, "water_matrix_id")?;
            let temperature = field(inputs, "temperature_c")?;
            let ph = field(inputs, "ph")?;
            if matrix.is_null() || temperature.is_null() || ph.is_null() {
                return Ok(indeterminate("WATER_MATRIX_TEMPERATURE_AND_PH_REQUIRED"));
            }
            Ok(indeterminate("CASE_CONTRACT_EXPECTED_MISSING_MATRIX_FIELDS"))
        }
        "CF-T0-008-missing-reaction-kinetic-parameter" => {
            if field(inputs, "first_order_rate_h_inv")?.is_null() {
                return Ok(indeterminate("REACTION_PARAMETER_UNAVAILABLE"));
            }
            Ok(indeterminate("CASE_CONTRACT_EXPECTED_MISSING_REACTION_PARAMETER"))
        }
        "CF-T0-009-wall-species-requested-in-mixed-tank" => {
            require(text(inputs, "model_id")? == "EPANET-MSX-2.0", "case requires EPANET-MSX")?;
            require(text(inputs, "species_phase")? == "wall", "case requires a wall species")?;
            require(text(inputs, "process_domain")? == "storage_tank", "case requires storage tank")?;
            Ok(unsupported(
                "EPANET_MSX_WALL_SPECIES_NOT_DEFINED_IN_STORAGE_TANKS",
            ))
        }
        "CF-T0-010-pressure-driven-input-required-but-not-supplied-by-msx" => {
            require(text(inputs, "model_id")? == "EPANET-MSX-2.0", "case requires EPANET-MSX")?;
            require(
                text(inputs, "hydraulic_engine")? == "EPANET-2.2",
                "case requires the declared EPANET 2.2 hydraulic engine",
            )?;
            require(
                text(inputs, "hydraulic_mode")? == "demand_driven_extended_period",
                "case requires the declared demand-driven hydraulic mode",
            )?;
            require(
                text(inputs, "required_hydraulic_mode")? == "pressure_driven_transient",
                "case requires a pressure-driven transient hydraulic input",
            )?;
            require(
                field(inputs, "pressure_history_digest_marker")?.is_null(),
                "case requires no pressure-history marker",
            )?;
            Ok(unsupported(
                "MSX_HYDRAULIC_INPUT_NOT_PRESSURE_DRIVEN_TRANSIENT",
            ))
        }
        "CF-T0-011-dispersion-required-but-disabled" => {
            require(bool_field(inputs, "dispersion_required")?, "case requires dispersion")?;
            require(!bool_field(inputs, "dispersion_enabled")?, "case requires dispersion disabled")?;
            Ok(unsupported("REQUIRED_DISPERSION_OPTION_DISABLED"))
        }
        "CF-T0-012-negative-concentration-or-rate" => {
            let concentration = number(inputs, "initial_parent_concentration_mg_l")?;
            let rate = number(inputs, "first_order_rate_h_inv")?;
            let time = number(inputs, "elapsed_time_h")?;
            require(
                concentration < 0.0 || rate < 0.0 || time < 0.0,
                "case requires a negative concentration, rate or elapsed time",
            )?;
            Ok(invalid("NEGATIVE_CONCENTRATION_OR_RATE"))
        }
        other => Err(format!("unrecognized chemical-fate T0 case: {other}")),
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

fn ids(cases: &[Value]) -> Result<Vec<String>, String> {
    cases
        .iter()
        .map(|case| case_id(case).map(str::to_owned))
        .collect()
}

fn validate_same_ids(left: &[String], right: &[String], label: &str) -> Result<(), String> {
    let left_set: BTreeSet<&str> = left.iter().map(String::as_str).collect();
    let right_set: BTreeSet<&str> = right.iter().map(String::as_str).collect();
    require(left.len() == left_set.len(), "duplicate input case IDs")?;
    require(right.len() == right_set.len(), "duplicate reference case IDs")?;
    require(left_set == right_set, &format!("{label} case identities differ"))
}

fn validate_contract(inputs: &Value, oracle: &Value, manifest: &Value) -> Result<(), String> {
    require(text(inputs, "schema_version")? == "0.1.0", "input schema mismatch")?;
    require(text(inputs, "programme")? == PROGRAMME, "input programme mismatch")?;
    require(text(inputs, "evidence_class")? == "synthetic_only", "input evidence class mismatch")?;
    require(
        text(inputs, "evidence_ceiling")?
            == "analytical_reaction_and_model_scope_contracts_only_no_physical_chemical_qualification",
        "input evidence ceiling mismatch",
    )?;
    let profile = field(inputs, "profile")?;
    require(text(profile, "profile_id")? == PROFILE, "input profile mismatch")?;
    require(text(profile, "revision")? == "0.1", "input profile revision mismatch")?;
    require(
        field(profile, "all_values_are_fictional")?.as_bool() == Some(true),
        "T0 inputs must be declared fictional",
    )?;
    let units = field(profile, "units")?;
    for (key, unit) in [
        ("concentration", "mg/L"),
        ("rate", "1/h"),
        ("time", "h"),
        ("surface_loading", "mg/m2"),
        ("adsorption_equilibrium", "L/mg"),
        ("mass", "mg"),
        ("ph", "dimensionless"),
        ("temperature", "degC"),
        ("flow", "L/s"),
    ] {
        require(text(units, key)? == unit, &format!("unit mismatch for {key}"))?;
    }

    require(text(oracle, "schema_version")? == "0.1.0", "oracle schema mismatch")?;
    require(text(oracle, "programme")? == PROGRAMME, "oracle programme mismatch")?;
    require(
        text(oracle, "evidence_class")? == "synthetic_expected_outputs_only",
        "oracle evidence class mismatch",
    )?;
    require(
        text(oracle, "input_profile_id")? == PROFILE,
        "oracle profile mismatch",
    )?;
    require(
        text(manifest, "schema_version")? == "0.1.0",
        "manifest schema mismatch",
    )?;
    require(
        text(manifest, "programme_id")? == PROGRAMME,
        "manifest programme mismatch",
    )?;
    require(
        text(manifest, "input_file")? == "chemical-fate-t0-cases-v0.1.json",
        "manifest input filename mismatch",
    )?;
    require(
        text(manifest, "oracle_file")? == "chemical-fate-t0-oracle-v0.1.json",
        "manifest oracle filename mismatch",
    )?;
    require(
        text(manifest, "comparator_register_file")?
            == "chemical-fate-comparator-register-v0.1.json",
        "manifest comparator register filename mismatch",
    )?;
    require(
        number(manifest, "scenario_count")? == CASE_COUNT as f64,
        "manifest scenario count mismatch",
    )?;
    let input_cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    require(input_cases.len() == CASE_COUNT, "input case count mismatch")?;
    require(oracle_cases.len() == CASE_COUNT, "oracle case count mismatch")?;
    let input_ids = ids(input_cases)?;
    let oracle_ids = ids(oracle_cases)?;
    let manifest_ids = read_ids(manifest, "scenario_ids")?;
    validate_same_ids(&input_ids, &oracle_ids, "input/oracle")?;
    validate_same_ids(&input_ids, &manifest_ids, "input/manifest")?;

    for row in oracle_cases {
        let expected = field(row, "expected")?;
        require(
            field(expected, "potability_claim_permitted")?.as_bool() == Some(false),
            "oracle cannot permit potability",
        )?;
        require(
            field(expected, "release_authorized")?.as_bool() == Some(false),
            "oracle cannot authorize release",
        )?;
    }
    Ok(())
}

pub fn evaluate(
    inputs: &Value,
    oracle: &Value,
    manifest: &Value,
) -> Result<Value, String> {
    validate_contract(inputs, oracle, manifest)?;
    let input_cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    let mut expected = BTreeMap::new();
    for row in oracle_cases {
        let id = case_id(row)?.to_owned();
        let expected_row = field(row, "expected")?.clone();
        require(
            expected.insert(id.clone(), expected_row).is_none(),
            &format!("duplicate oracle case: {id}"),
        )?;
    }

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
        results.push(json!({
            "case_id": id,
            "matched": matched,
            "computed": computed
        }));
    }
    results.sort_by(|left, right| left["case_id"].as_str().cmp(&right["case_id"].as_str()));
    let matched = results.len() - mismatches.len();
    Ok(json!({
        "schema_version": "chemical-fate-t0-evaluation-v0.1",
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
            "Analytical equations and model-scope dispositions only; no chemical model is qualified.",
            "No reaction coefficient is validated for a real water matrix or pipe material.",
            "No concentration result establishes contaminant identity, toxicity, pathogen risk, consumer dose or potability.",
            "This evaluator does not authorize treatment, isolation, restart, notification or water release."
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
        return Err(
            "usage: chemical-fate-t0 [INPUTS] [ORACLE] [MANIFEST] [RECEIPT]".to_owned(),
        );
    }
    let input_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/chemical-fate-t0-cases-v0.1.json",
    ));
    let oracle_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/chemical-fate-t0-oracle-v0.1.json",
    ));
    let manifest_path = PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/manifest-v0.1.json",
    ));
    let receipt_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or(
        "artifacts/water-chemical-fate-t0/receipt.json",
    ));

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&inputs, &oracle, &manifest)?;
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let success = total == CASE_COUNT as u64 && matched == CASE_COUNT as u64 && no_mismatches;

    let revision = env::var("WATER_CHEMICAL_FATE_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;

    let receipt = json!({
        "schema_version": "water-chemical-fate-t0-run-v0.1",
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
            eprintln!("WATER_CHEMICAL_FATE_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn load(name: &str) -> Value {
        let source = match name {
            "inputs" => include_str!(
                "../../../../research/water-intrusion-coupling/chemical-fate-t0-cases-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-intrusion-coupling/chemical-fate-t0-oracle-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-intrusion-coupling/manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(source).expect("chemical fate fixture must be valid JSON")
    }

    fn evaluated() -> Value {
        evaluate(&load("inputs"), &load("oracle"), &load("manifest"))
            .expect("chemical fate T0 evaluation must complete")
    }

    #[test]
    fn all_twelve_chemical_fate_cases_match_frozen_oracle() {
        let result = evaluated();
        assert_eq!(result["result_count"], CASE_COUNT);
        assert_eq!(result["matched_case_count"], CASE_COUNT);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn first_order_decay_matches_closed_form() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter()
            .find(|row| row["case_id"] == "CF-T0-001-first-order-decay-known-answer")
            .unwrap();
        assert!(
            (row["computed"]["parent_remaining_concentration_mg_l"].as_f64().unwrap()
                - 1.5163266492815834).abs() < 1.0e-9
        );
    }

    #[test]
    fn sequential_reaction_conserves_declared_moiety() {
        let result = evaluated();
        for id in [
            "CF-T0-002-sequential-reaction-unequal-rate-constants",
            "CF-T0-003-sequential-reaction-equal-rate-limit",
        ] {
            let row = result["results"].as_array().unwrap()
                .iter().find(|row| row["case_id"] == id).unwrap();
            assert!((row["computed"]["conserved_moiety_total_mg_l"].as_f64().unwrap() - 1.0).abs() < 1.0e-9);
        }
    }

    #[test]
    fn langmuir_bulk_wall_equilibrium_matches_known_answer() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "CF-T0-004-langmuir-bulk-wall-equilibrium")
            .unwrap();
        assert_eq!(row["computed"]["surface_loading_mg_m2"], 5.0);
    }

    #[test]
    fn missing_residual_stream_is_not_replaced_with_zero() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "CF-T0-006-missing-residual-stream-does-not-force-closure")
            .unwrap();
        assert_eq!(row["computed"]["mass_balance_disposition"], "MASS_BALANCE_UNRESOLVED");
        assert_eq!(row["computed"]["unresolved_moiety_mass_mg"], 0.5);
    }

    #[test]
    fn missing_matrix_and_reaction_parameter_remain_indeterminate() {
        let result = evaluated();
        for (id, reason) in [
            ("CF-T0-007-missing-water-matrix-temperature-and-ph", "WATER_MATRIX_TEMPERATURE_AND_PH_REQUIRED"),
            ("CF-T0-008-missing-reaction-kinetic-parameter", "REACTION_PARAMETER_UNAVAILABLE"),
        ] {
            let row = result["results"].as_array().unwrap()
                .iter().find(|row| row["case_id"] == id).unwrap();
            assert_eq!(row["computed"]["chemical_disposition"], "INDETERMINATE");
            assert_eq!(row["computed"]["reason"], reason);
        }
    }

    #[test]
    fn msx_tank_wall_species_are_rejected() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "CF-T0-009-wall-species-requested-in-mixed-tank")
            .unwrap();
        assert_eq!(row["computed"]["chemical_disposition"], "UNSUPPORTED_MODEL_SCOPE");
    }

    #[test]
    fn msx_demand_driven_hydraulics_are_not_transient_intrusion_hydraulics() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "CF-T0-010-pressure-driven-input-required-but-not-supplied-by-msx")
            .unwrap();
        assert_eq!(row["computed"]["reason"], "MSX_HYDRAULIC_INPUT_NOT_PRESSURE_DRIVEN_TRANSIENT");
    }

    #[test]
    fn required_dispersion_cannot_be_silently_disabled() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "CF-T0-011-dispersion-required-but-disabled")
            .unwrap();
        assert_eq!(row["computed"]["reason"], "REQUIRED_DISPERSION_OPTION_DISABLED");
    }

    #[test]
    fn negative_concentration_and_rate_are_invalid() {
        let result = evaluated();
        let row = result["results"].as_array().unwrap()
            .iter().find(|row| row["case_id"] == "CF-T0-012-negative-concentration-or-rate")
            .unwrap();
        assert_eq!(row["computed"]["chemical_disposition"], "INVALID_INPUT");
    }

    #[test]
    fn tampered_oracle_cannot_authorize_release() {
        let mut oracle = load("oracle");
        oracle["cases"][0]["expected"]["release_authorized"] = json!(true);
        assert!(evaluate(&load("inputs"), &oracle, &load("manifest")).is_err());
    }

    #[test]
    fn duplicate_input_case_identity_is_rejected() {
        let mut inputs = load("inputs");
        let duplicate = inputs["cases"][0].clone();
        inputs["cases"].as_array_mut().unwrap().push(duplicate);
        assert!(evaluate(&inputs, &load("oracle"), &load("manifest")).is_err());
    }
}
