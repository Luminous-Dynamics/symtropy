// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::PathBuf;

const EXPECTED_CASE_COUNT: usize = 15;
const CORPUS_ID: &str = "water-control-robustness-scenarios-v0.1";
const ORACLE_ID: &str = "water-control-robustness-disposition-oracle-v0.1";
const HARD_GATE: &str = "must_not_issue_unqualified_supervisory_command";

fn sha256(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn read_json(path: &PathBuf) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|error| format!("read {}: {error}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|error| format!("parse {}: {error}", path.display()))?;
    Ok((bytes, value))
}

fn field<'a>(value: &'a Value, key: &str) -> Result<&'a Value, String> {
    value
        .get(key)
        .ok_or_else(|| format!("missing field: {key}"))
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    field(value, key)?
        .as_str()
        .ok_or_else(|| format!("expected string: {key}"))
}

fn number(value: &Value, key: &str) -> Result<f64, String> {
    field(value, key)?
        .as_f64()
        .filter(|number| number.is_finite())
        .ok_or_else(|| format!("expected finite number: {key}"))
}

fn strings(value: &Value, key: &str) -> Result<Vec<String>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("expected array: {key}"))?
        .iter()
        .map(|item| {
            item.as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("expected string item in {key}"))
        })
        .collect()
}

fn contains(values: &[String], expected: &str) -> bool {
    values.iter().any(|value| value == expected)
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn validate_corpus(scenarios: &Value, oracle: &Value) -> Result<Value, String> {
    if string(scenarios, "schema_version")? != "0.1.0" {
        return Err("unsupported scenario schema_version".to_owned());
    }
    if string(scenarios, "corpus_id")? != CORPUS_ID {
        return Err("unexpected scenario corpus_id".to_owned());
    }
    if string(oracle, "schema_version")? != "0.1.0" {
        return Err("unsupported oracle schema_version".to_owned());
    }
    if string(oracle, "oracle_id")? != ORACLE_ID {
        return Err("unexpected oracle_id".to_owned());
    }
    if string(oracle, "corpus_id")? != CORPUS_ID {
        return Err("oracle references a different corpus".to_owned());
    }

    let profile = field(scenarios, "profile")?;
    if string(profile, "evidence_class")? != "synthetic_design_profile" {
        return Err("profile evidence class must remain synthetic".to_owned());
    }
    if string(profile, "profile_id")? != "SYNTHETIC-SMALL-WDS-CONTROL-V0" {
        return Err("unexpected synthetic profile ID".to_owned());
    }
    if field(profile, "values_are_normative")?.as_bool() != Some(false) {
        return Err("synthetic service bounds must not be marked normative".to_owned());
    }
    let pressure_floor = number(profile, "pressure_floor_m")?;
    let pressure_ceiling = number(profile, "pressure_ceiling_m")?;
    let reserve_floor = number(profile, "storage_reserve_floor_fraction")?;
    let storage_ceiling = number(profile, "storage_ceiling_fraction")?;
    let time_step = field(profile, "time_step_seconds")?
        .as_u64()
        .ok_or_else(|| "time_step_seconds must be a positive integer".to_owned())?;
    if pressure_floor <= 0.0
        || pressure_ceiling <= pressure_floor
        || !(0.0..=1.0).contains(&reserve_floor)
        || !(0.0..=1.0).contains(&storage_ceiling)
        || reserve_floor >= storage_ceiling
        || time_step == 0
    {
        return Err("invalid synthetic service profile bounds".to_owned());
    }

    let baseline_values = field(scenarios, "baseline_families")?
        .as_array()
        .ok_or_else(|| "baseline_families must be an array".to_owned())?;
    let baseline_set: BTreeSet<&str> = baseline_values
        .iter()
        .map(|item| {
            item.as_str()
                .ok_or_else(|| "baseline family must be a string".to_owned())
        })
        .collect::<Result<_, _>>()?;
    let required_baselines = [
        "tuned_rule_or_pi",
        "hydraulic_model_mpc",
        "data_enabled_predictive_control",
        "model_free_enc_2026",
        "qualified_estimator_plus_controller",
    ];
    if baseline_set.len() != baseline_values.len()
        || required_baselines
            .iter()
            .any(|baseline| !baseline_set.contains(baseline))
    {
        return Err("baseline family set is incomplete or contains duplicates".to_owned());
    }

    let cases = field(scenarios, "scenarios")?
        .as_array()
        .ok_or_else(|| "scenarios must be an array".to_owned())?;
    let oracle_results = field(oracle, "results")?
        .as_array()
        .ok_or_else(|| "oracle results must be an array".to_owned())?;
    if cases.len() != EXPECTED_CASE_COUNT || oracle_results.len() != EXPECTED_CASE_COUNT {
        return Err(format!(
            "expected {EXPECTED_CASE_COUNT} cases and oracle rows; got {} and {}",
            cases.len(),
            oracle_results.len()
        ));
    }

    let mut oracle_by_id = BTreeMap::new();
    for row in oracle_results {
        let id = string(row, "scenario_id")?.to_owned();
        let disposition = string(row, "expected_disposition")?.to_owned();
        if oracle_by_id.insert(id.clone(), disposition).is_some() {
            return Err(format!("duplicate oracle scenario ID: {id}"));
        }
        if string(row, "hard_gate")? != HARD_GATE {
            return Err(format!("oracle hard-gate drift in {id}"));
        }
        if string(row, "evidence_ceiling")? != "synthetic_design_only" {
            return Err(format!("oracle evidence-ceiling drift in {id}"));
        }
    }

    let mut seen = BTreeSet::new();
    let mut results = Vec::with_capacity(cases.len());
    for case in cases {
        let id = string(case, "scenario_id")?.to_owned();
        if !id.starts_with("WCR-") || !seen.insert(id.clone()) {
            return Err(format!("invalid or duplicate scenario ID: {id}"));
        }
        let measurement_state = string(case, "measurement_state")?;
        let disposition = string(case, "expected_disposition")?;
        let coverage = number(case, "direct_state_coverage_fraction")?;
        let demand_peak_factor = number(case, "demand_peak_factor")?;
        if !(0.0..=1.0).contains(&coverage) || demand_peak_factor <= 0.0 {
            return Err(format!("invalid coverage or demand factor in {id}"));
        }
        let unobserved = strings(case, "unobserved_critical_variables")?;
        let disturbances = strings(case, "disturbances")?;
        let required_behaviors = strings(case, "required_behaviors")?;
        let prohibited_behaviors = strings(case, "prohibited_behaviors")?;
        if required_behaviors.is_empty() || prohibited_behaviors.is_empty() {
            return Err(format!("missing behavioral constraints in {id}"));
        }

        if coverage < 1.0 && disposition == "run_nominal" {
            return Err(format!("{id}: partial sensing cannot be admitted as nominal"));
        }
        if coverage == 1.0 && !unobserved.is_empty() {
            return Err(format!("{id}: full direct coverage conflicts with unobserved variables"));
        }
        if coverage < 1.0 && unobserved.is_empty() {
            return Err(format!("{id}: partial coverage requires named unobserved variables"));
        }
        if disposition == "run_nominal" && measurement_state != "fully_observed" {
            return Err(format!("{id}: nominal run requires fully observed state"));
        }
        if disposition == "run_with_qualified_estimator"
            && !contains(&disturbances, "qualified_estimator_with_bound")
        {
            return Err(format!("{id}: estimator disposition requires a qualified-bound marker"));
        }
        if disposition == "run_with_qualified_estimator"
            && measurement_state != "partial_with_qualified_estimator"
        {
            return Err(format!("{id}: estimator disposition has inconsistent measurement_state"));
        }
        if disposition == "stop_and_reconcile"
            && !contains(&disturbances, "topology_revision_mismatch")
            && !contains(&disturbances, "pump_2_trip")
        {
            return Err(format!(
                "{id}: stop-and-reconcile lacks a topology or compound-fault trigger"
            ));
        }
        if disposition == "local_safety_only"
            && ![
                "pump_2_trip",
                "source_level_below_declared_floor",
                "power_loss_900_seconds",
                "supervisory_link_offline_120_seconds",
            ]
            .iter()
            .any(|fault| contains(&disturbances, fault))
        {
            return Err(format!(
                "{id}: local-safety-only disposition lacks a declared trigger"
            ));
        }
        if disposition == "abstain_or_local_fallback"
            && ![
                "one_sensor_dropout",
                "pressure_sensor_bias_plus_5_percent_full_scale",
                "common_mode_pressure_bias_plus_3_percent_full_scale",
                "redundant_pressure_disagreement_10_percent_full_scale",
                "telemetry_age_120_seconds",
            ]
            .iter()
            .any(|fault| contains(&disturbances, fault))
            && measurement_state != "partial_unqualified"
        {
            return Err(format!(
                "{id}: abstention disposition lacks a sensor/timebase trigger"
            ));
        }
        if !prohibited_behaviors
            .iter()
            .any(|item| item.contains("unobservable"))
        {
            return Err(format!("{id}: unobservable-state prohibition is missing"));
        }

        let oracle_disposition = oracle_by_id
            .get(&id)
            .ok_or_else(|| format!("{id}: missing from oracle"))?;
        if disposition != oracle_disposition {
            return Err(format!("{id}: scenario/oracle disposition mismatch"));
        }

        results.push(json!({
            "scenario_id": id,
            "matched": true,
            "expected_disposition": disposition,
            "direct_state_coverage_fraction": coverage,
            "evidence_ceiling": "synthetic_design_only"
        }));
    }
    if seen.len() != oracle_by_id.len() || oracle_by_id.keys().any(|id| !seen.contains(id)) {
        return Err("scenario and oracle ID sets differ".to_owned());
    }

    Ok(json!({
        "schema_version": "water-control-robustness-evaluation-v0.1",
        "verification_state": "Matched",
        "qualification_status": "NotRun",
        "scenario_count": cases.len(),
        "matched_case_count": results.len(),
        "results": results,
        "evidence_class": "synthetic_scenario_contract_only",
        "claim_ceiling": "scenario_admission_and_expected_disposition_semantics_only_no_controller_or_physical_qualification",
        "nonclaims": [
            "No hydraulic trajectory or controller output is computed.",
            "No safety interlock, stability guarantee or physical equipment is qualified.",
            "No energy, water-saving, pressure, reliability or field performance is measured.",
            "No pump or valve actuation or utility integration is authorized."
        ]
    }))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err(
            "usage: water-control-robustness [SCENARIOS] [SCHEMA] [ORACLE] [RECEIPT]".to_owned(),
        );
    }
    let scenario_path = PathBuf::from(
        args.first().map(String::as_str).unwrap_or(
            "research/water-service-engineering/pump-control-robustness/scenarios-v0.1.json",
        ),
    );
    let schema_path = PathBuf::from(
        args.get(1).map(String::as_str).unwrap_or(
            "research/water-service-engineering/pump-control-robustness/scenarios-v0.1.schema.json",
        ),
    );
    let oracle_path = PathBuf::from(
        args.get(2).map(String::as_str).unwrap_or(
            "research/water-service-engineering/pump-control-robustness/disposition-oracle-v0.1.json",
        ),
    );
    let receipt_path = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/water-control-robustness/receipt.json"),
    );

    let (scenario_bytes, scenarios) = read_json(&scenario_path)?;
    let (schema_bytes, _schema) = read_json(&schema_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let evaluation = validate_corpus(&scenarios, &oracle)?;

    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let revision = env::var("WATER_CONTROL_ROBUSTNESS_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let receipt = json!({
        "schema_version": "water-control-robustness-run-v0.1",
        "verification_state": evaluation["verification_state"],
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "scenario_sha256": sha256(&scenario_bytes),
        "schema_sha256": sha256(&schema_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "evaluation": evaluation
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
    Ok(0)
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATER_CONTROL_ROBUSTNESS_ERROR: {error}");
            std::process::exit(1);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SCENARIOS: &str = include_str!("../../../../research/water-service-engineering/pump-control-robustness/scenarios-v0.1.json");
    const ORACLE: &str = include_str!("../../../../research/water-service-engineering/pump-control-robustness/disposition-oracle-v0.1.json");

    fn fixtures() -> (Value, Value) {
        (
            serde_json::from_str(SCENARIOS).expect("scenario fixture parses"),
            serde_json::from_str(ORACLE).expect("oracle fixture parses"),
        )
    }

    #[test]
    fn frozen_fifteen_case_corpus_matches_disposition_oracle() {
        let (scenarios, oracle) = fixtures();
        let result = validate_corpus(&scenarios, &oracle).expect("frozen corpus validates");
        assert_eq!(result["scenario_count"], EXPECTED_CASE_COUNT);
        assert_eq!(result["matched_case_count"], EXPECTED_CASE_COUNT);
        assert_eq!(result["qualification_status"], "NotRun");
    }

    #[test]
    fn partial_sensing_cannot_be_admitted_as_nominal() {
        let (mut scenarios, oracle) = fixtures();
        scenarios["scenarios"][3]["expected_disposition"] = json!("run_nominal");
        let error = validate_corpus(&scenarios, &oracle).expect_err("partial sensing must fail");
        assert!(error.contains("partial sensing"));
    }

    #[test]
    fn estimator_path_requires_explicit_qualification_marker() {
        let (mut scenarios, oracle) = fixtures();
        scenarios["scenarios"][2]["disturbances"] = json!([]);
        let error =
            validate_corpus(&scenarios, &oracle).expect_err("unqualified estimator must fail");
        assert!(error.contains("qualified-bound"));
    }

    #[test]
    fn duplicate_scenario_ids_are_rejected() {
        let (mut scenarios, oracle) = fixtures();
        let first = scenarios["scenarios"][0]["scenario_id"].clone();
        scenarios["scenarios"][1]["scenario_id"] = first;
        let error = validate_corpus(&scenarios, &oracle).expect_err("duplicate ID must fail");
        assert!(error.contains("duplicate scenario ID"));
    }

    #[test]
    fn normative_synthetic_profile_is_rejected() {
        let (mut scenarios, oracle) = fixtures();
        scenarios["profile"]["values_are_normative"] = json!(true);
        let error = validate_corpus(&scenarios, &oracle).expect_err("normative flag must fail");
        assert!(error.contains("must not be marked normative"));
    }

    #[test]
    fn modified_oracle_disposition_is_rejected() {
        let (scenarios, mut oracle) = fixtures();
        oracle["results"][8]["expected_disposition"] = json!("run_nominal");
        let error = validate_corpus(&scenarios, &oracle).expect_err("altered oracle must fail");
        assert!(error.contains("scenario/oracle disposition mismatch"));
    }
}
