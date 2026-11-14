// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::collections::{BTreeMap, BTreeSet};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const PROGRAMME: &str = "WATER-INTRUSION-COUPLING-QUAL-001";
const PROFILE: &str = "LUMINOUS-WATER-INTRUSION-COUPLING-T0";
const INPUT_CEILING: &str =
    "interface_and_provenance_contracts_only_no_physical_model_qualification";
const CASE_COUNT: usize = 8;
const REQUIRED_STAGES: [&str; 5] = [
    "initial_state",
    "transient_pressure",
    "defect_ingress",
    "network_transport",
    "chemical_fate",
];
const REQUIRED_OBSERVATIONS: [&str; 3] = [
    "pressure_trace",
    "signed_defect_flow",
    "tracer_sample",
];

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

fn bool_field(value: &Value, key: &str) -> Result<bool, String> {
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

fn valid_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn compare_json(actual: &Value, expected: &Value) -> bool {
    match (actual, expected) {
        (Value::Number(left), Value::Number(right)) => {
            matches!(
                (left.as_f64(), right.as_f64()),
                (Some(a), Some(b))
                    if (a - b).abs() <= 1.0e-9 * a.abs().max(b.abs()).max(1.0)
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

fn indeterminate(reason: &str) -> Value {
    json!({
        "interface_disposition": "INDETERMINATE",
        "reason": reason,
        "qualification_status": "NotRun",
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn rejected(reason: &str) -> Value {
    json!({
        "interface_disposition": "REJECTED_CONTRACT",
        "reason": reason,
        "qualification_status": "NotRun",
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn out_of_domain(reason: &str) -> Value {
    json!({
        "interface_disposition": "OUT_OF_DOMAIN",
        "reason": reason,
        "qualification_status": "NotRun",
        "potability_claim_permitted": false,
        "release_authorized": false
    })
}

fn expected_dependencies(stage_id: &str) -> Option<&'static [&'static str]> {
    match stage_id {
        "initial_state" => Some(&[]),
        "transient_pressure" => Some(&["initial_state"]),
        "defect_ingress" => Some(&["transient_pressure"]),
        "network_transport" => Some(&["transient_pressure", "defect_ingress"]),
        "chemical_fate" => Some(&["network_transport"]),
        _ => None,
    }
}

fn stage_map(stages: &[Value]) -> Result<BTreeMap<String, Value>, String> {
    let mut map = BTreeMap::new();
    for stage in stages {
        let id = text(stage, "stage_id")?.to_owned();
        require(
            expected_dependencies(&id).is_some(),
            &format!("unknown model stage: {id}"),
        )?;
        require(
            map.insert(id.clone(), stage.clone()).is_none(),
            &format!("duplicate model stage: {id}"),
        )?;
    }
    Ok(map)
}

fn read_string_array(value: &Value, key: &str) -> Result<Vec<String>, String> {
    field(value, key)?
        .as_array()
        .ok_or_else(|| format!("{key} must be an array"))?
        .iter()
        .map(|entry| {
            entry
                .as_str()
                .map(str::to_owned)
                .ok_or_else(|| format!("{key} contains a non-string entry"))
        })
        .collect()
}

fn verify_stage_chain(stages: &BTreeMap<String, Value>) -> Result<Option<Value>, String> {
    for required in REQUIRED_STAGES {
        if !stages.contains_key(required) {
            if required == "chemical_fate" {
                return Ok(Some(indeterminate(
                    "CHEMICAL_FATE_NOT_MODELLED_NOT_INERT",
                )));
            }
            return Ok(Some(indeterminate("REQUIRED_UPSTREAM_STAGE_MISSING")));
        }
    }

    let initial = stages
        .get("initial_state")
        .ok_or_else(|| "initial_state disappeared from stage map".to_owned())?;
    let timebase = text(initial, "timebase_id")?;

    for stage_id in REQUIRED_STAGES {
        let stage = stages
            .get(stage_id)
            .ok_or_else(|| format!("missing stage after contract check: {stage_id}"))?;
        require(!text(stage, "model_id")?.is_empty(), "model_id must not be empty")?;
        require(
            !text(stage, "model_revision")?.is_empty(),
            "model_revision must not be empty",
        )?;

        let output_digest = text(stage, "output_digest")?;
        require(
            valid_sha256(output_digest),
            &format!("invalid output digest for stage {stage_id}"),
        )?;

        let stage_timebase = text(stage, "timebase_id")?;
        if stage_timebase != timebase {
            return Ok(Some(indeterminate("STAGE_TIMEBASE_MISMATCH")));
        }
        let clock_uncertainty = number(stage, "clock_uncertainty_ms")?;
        require(
            clock_uncertainty >= 0.0,
            "clock uncertainty must be non-negative",
        )?;

        match text(stage, "validity_state")? {
            "in_domain" => {}
            "out_of_domain" => {
                return Ok(Some(out_of_domain("UPSTREAM_STAGE_OUT_OF_DOMAIN")));
            }
            _ => {
                return Ok(Some(indeterminate("UPSTREAM_STAGE_UNSUPPORTED")));
            }
        }

        match text(stage, "uncertainty_state")? {
            "bounded" => {}
            "unbounded" => {
                return Ok(Some(indeterminate("UNCERTAINTY_UNBOUNDED")));
            }
            _ => {
                return Ok(Some(indeterminate("UNCERTAINTY_STATE_UNKNOWN")));
            }
        }

        let coverage = number(stage, "uncertainty_coverage_percent")?;
        require(
            (0.0..=100.0).contains(&coverage),
            "uncertainty coverage must be in [0, 100]",
        )?;

        let dependencies = expected_dependencies(stage_id)
            .ok_or_else(|| format!("no dependency contract for stage {stage_id}"))?;
        let consumed = field(stage, "consumed_upstream_outputs")?
            .as_object()
            .ok_or_else(|| "consumed_upstream_outputs must be an object".to_owned())?;
        if consumed.len() != dependencies.len() {
            return Ok(Some(rejected("UPSTREAM_OUTPUT_DIGEST_MISMATCH")));
        }

        for dependency in dependencies {
            let upstream = stages
                .get(*dependency)
                .ok_or_else(|| format!("missing dependency stage: {dependency}"))?;
            let expected_digest = text(upstream, "output_digest")?;
            let consumed_digest = consumed
                .get(*dependency)
                .and_then(Value::as_str)
                .ok_or_else(|| format!("missing consumed digest for {dependency}"))?;
            if consumed_digest != expected_digest {
                return Ok(Some(rejected("UPSTREAM_OUTPUT_DIGEST_MISMATCH")));
            }
        }
    }

    Ok(None)
}

fn verify_observations(
    observations: &[Value],
    timebase: &str,
) -> Result<Option<Value>, String> {
    let mut by_kind = BTreeMap::new();
    for observation in observations {
        let kind = text(observation, "kind")?.to_owned();
        require(
            REQUIRED_OBSERVATIONS.contains(&kind.as_str()),
            &format!("unknown observation kind: {kind}"),
        )?;
        require(
            by_kind.insert(kind.clone(), observation.clone()).is_none(),
            &format!("duplicate observation kind: {kind}"),
        )?;
    }

    for required in REQUIRED_OBSERVATIONS {
        if !by_kind.contains_key(required) {
            return Ok(Some(indeterminate("REQUIRED_MEASURED_OBSERVATION_MISSING")));
        }
    }

    for kind in REQUIRED_OBSERVATIONS {
        let observation = by_kind
            .get(kind)
            .ok_or_else(|| format!("missing observation after validation: {kind}"))?;
        if text(observation, "evidence_class")? != "measured" {
            return Ok(Some(indeterminate(
                "MODEL_OUTPUT_CANNOT_SUBSTITUTE_FOR_MEASURED_OBSERVATION",
            )));
        }
        let Some(digest) = observation
            .get("raw_data_digest")
            .and_then(Value::as_str)
        else {
            return Ok(Some(indeterminate("OBSERVATION_PROVENANCE_INCOMPLETE")));
        };
        if !valid_sha256(digest) {
            return Ok(Some(indeterminate("OBSERVATION_PROVENANCE_INCOMPLETE")));
        }
        if text(observation, "timebase_id")? != timebase {
            return Ok(Some(indeterminate("OBSERVATION_TIMEBASE_MISMATCH")));
        }
        if text(observation, "calibration_state")? != "current"
            || text(observation, "method_state")? != "valid"
        {
            return Ok(Some(indeterminate("OBSERVATION_CALIBRATION_OR_METHOD_INVALID")));
        }
        if kind == "tracer_sample" {
            require(
                number(observation, "sample_time_s")? >= 0.0,
                "tracer sample time must be non-negative",
            )?;
            require(
                text(observation, "sample_result")? == "detected",
                "synthetic measured tracer sample must be detected",
            )?;
        } else {
            require(
                number(observation, "event_time_s")? >= 0.0,
                "pressure/flow observation event time must be non-negative",
            )?;
        }
    }
    Ok(None)
}

fn compute(case: &Value) -> Result<Value, String> {
    let inputs = field(case, "inputs")?;
    let stages = field(inputs, "stages")?
        .as_array()
        .ok_or_else(|| "stages must be an array".to_owned())?;
    let supplied_stage_order = stages
        .iter()
        .map(|stage| text(stage, "stage_id").map(str::to_owned))
        .collect::<Result<Vec<_>, _>>()?;
    let mut previous_position: Option<usize> = None;
    for supplied_id in &supplied_stage_order {
        let Some(position) = REQUIRED_STAGES
            .iter()
            .position(|expected| expected == supplied_id)
        else {
            return Ok(rejected("MODEL_STAGE_ORDER_MISMATCH"));
        };
        if previous_position.is_some_and(|previous| position <= previous) {
            return Ok(rejected("MODEL_STAGE_ORDER_MISMATCH"));
        }
        previous_position = Some(position);
    }
    let observations = field(inputs, "observations")?
        .as_array()
        .ok_or_else(|| "observations must be an array".to_owned())?;
    let map = stage_map(stages)?;

    if let Some(disposition) = verify_stage_chain(&map)? {
        return Ok(disposition);
    }
    let timebase = text(
        map.get("initial_state")
            .ok_or_else(|| "initial_state missing".to_owned())?,
        "timebase_id",
    )?;
    if let Some(disposition) = verify_observations(observations, timebase)? {
        return Ok(disposition);
    }

    Ok(json!({
        "interface_disposition": "CHAIN_CONTRACT_VALID_FOR_BOUNDED_MODEL_COMPARISON",
        "qualification_status": "NotRun",
        "stage_count": map.len(),
        "measured_observation_count": observations.len(),
        "potability_claim_permitted": false,
        "release_authorized": false
    }))
}

fn ids(cases: &[Value]) -> Result<Vec<String>, String> {
    cases
        .iter()
        .map(|case| case_id(case).map(str::to_owned))
        .collect()
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

fn validate_same_ids(left: &[String], right: &[String], label: &str) -> Result<(), String> {
    let left_set: BTreeSet<&str> = left.iter().map(String::as_str).collect();
    let right_set: BTreeSet<&str> = right.iter().map(String::as_str).collect();
    require(left.len() == left_set.len(), "duplicate input case IDs")?;
    require(right.len() == right_set.len(), "duplicate reference case IDs")?;
    require(left_set == right_set, &format!("{label} case identities differ"))
}

fn validate_contract(
    inputs: &Value,
    oracle: &Value,
    contract: &Value,
    manifest: &Value,
) -> Result<(), String> {
    require(text(inputs, "schema_version")? == "0.1.0", "input schema mismatch")?;
    require(text(inputs, "programme")? == PROGRAMME, "input programme mismatch")?;
    require(text(inputs, "evidence_class")? == "synthetic_only", "input evidence class mismatch")?;
    require(
        text(inputs, "evidence_ceiling")? == INPUT_CEILING,
        "input evidence ceiling mismatch",
    )?;
    let profile = field(inputs, "profile")?;
    require(text(profile, "profile_id")? == PROFILE, "input profile mismatch")?;
    require(text(profile, "revision")? == "0.1", "input profile revision mismatch")?;
    require(
        bool_field(profile, "all_values_are_fictional")?,
        "T0 inputs must be declared fictional",
    )?;
    let units = field(profile, "units")?;
    for (key, expected) in [
        ("pressure", "kPa_gauge"),
        ("head", "m_water"),
        ("flow", "L/s"),
        ("signed_flow", "L/s"),
        ("concentration", "mg/L"),
        ("time", "s"),
        ("clock_uncertainty", "ms"),
    ] {
        require(text(units, key)? == expected, &format!("unit mismatch for {key}"))?;
    }

    require(text(oracle, "schema_version")? == "0.1.0", "oracle schema mismatch")?;
    require(text(oracle, "programme")? == PROGRAMME, "oracle programme mismatch")?;
    require(
        text(oracle, "evidence_class")? == "synthetic_expected_outputs_only",
        "oracle evidence class mismatch",
    )?;
    require(
        text(oracle, "evidence_ceiling")?
            == "interface_contract_semantics_only_no_physical_qualification",
        "oracle evidence ceiling mismatch",
    )?;
    require(text(oracle, "input_profile_id")? == PROFILE, "oracle profile mismatch")?;

    require(text(contract, "schema_version")? == "0.1.0", "interface contract schema mismatch")?;
    require(
        text(contract, "programme_id")? == PROGRAMME,
        "interface contract programme mismatch",
    )?;
    require(
        text(contract, "interface_contract_id")? == "LUMINOUS-WATER-INTRUSION-CHAIN-0.1",
        "interface contract identity mismatch",
    )?;
    let stage_ids = read_ids(contract, "required_stage_order")?;
    require(
        stage_ids == REQUIRED_STAGES.iter().map(|v| v.to_string()).collect::<Vec<_>>(),
        "required stage order mismatch",
    )?;
    let observation_ids = read_ids(contract, "required_observation_kinds")?;
    require(
        observation_ids
            == REQUIRED_OBSERVATIONS
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>(),
        "required observation kinds mismatch",
    )?;

    require(text(manifest, "schema_version")? == "0.1.0", "manifest schema mismatch")?;
    require(text(manifest, "programme_id")? == PROGRAMME, "manifest programme mismatch")?;
    require(
        text(manifest, "evidence_class")? == "design_and_synthetic_only",
        "manifest evidence class mismatch",
    )?;
    require(
        text(manifest, "evidence_ceiling")?
            .contains("no_physical_model_qualification"),
        "manifest evidence ceiling too permissive",
    )?;
    let manifest_stage_order = read_ids(manifest, "required_stage_order")?;
    let expected_stage_order = REQUIRED_STAGES
        .iter()
        .map(|stage| stage.to_string())
        .collect::<Vec<_>>();
    require(
        manifest_stage_order == expected_stage_order,
        "manifest required stage order mismatch",
    )?;
    let manifest_observations = read_ids(manifest, "required_observation_kinds")?;
    let expected_observations = REQUIRED_OBSERVATIONS
        .iter()
        .map(|kind| kind.to_string())
        .collect::<Vec<_>>();
    require(
        manifest_observations == expected_observations,
        "manifest required observation kinds mismatch",
    )?;
    require(
        text(manifest, "input_file")? == "coupling-t0-cases-v0.1.json",
        "manifest input filename mismatch",
    )?;
    require(
        text(manifest, "oracle_file")? == "coupling-t0-oracle-v0.1.json",
        "manifest oracle filename mismatch",
    )?;
    require(
        text(manifest, "interface_contract_file")? == "model-coupling-interface-contract-v0.1.json",
        "manifest interface contract filename mismatch",
    )?;
    require(
        number(manifest, "scenario_count")? == CASE_COUNT as f64,
        "manifest scenario count mismatch",
    )?;
    let manifest_ids = read_ids(manifest, "scenario_ids")?;
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
    validate_same_ids(&input_ids, &oracle_ids, "input/oracle")?;
    validate_same_ids(&input_ids, &manifest_ids, "input/manifest")?;

    for row in oracle_cases {
        let expected = field(row, "expected")?;
        require(
            !bool_field(expected, "potability_claim_permitted")?,
            "oracle cannot permit potability",
        )?;
        require(
            !bool_field(expected, "release_authorized")?,
            "oracle cannot authorize release",
        )?;
    }
    Ok(())
}

pub fn evaluate(
    inputs: &Value,
    oracle: &Value,
    contract: &Value,
    manifest: &Value,
) -> Result<Value, String> {
    validate_contract(inputs, oracle, contract, manifest)?;
    let cases = field(inputs, "cases")?
        .as_array()
        .ok_or_else(|| "input cases must be an array".to_owned())?;
    let oracle_cases = field(oracle, "cases")?
        .as_array()
        .ok_or_else(|| "oracle cases must be an array".to_owned())?;
    let mut expected = BTreeMap::new();
    for row in oracle_cases {
        let id = case_id(row)?.to_owned();
        let expected_row = field(row, "expected")?;
        require(
            expected.insert(id.clone(), expected_row.clone()).is_none(),
            &format!("duplicate oracle case: {id}"),
        )?;
    }

    let mut results = Vec::with_capacity(CASE_COUNT);
    let mut mismatches = Vec::new();
    for case in cases {
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
        "schema_version": "water-intrusion-coupling-t0-evaluation-v0.1",
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
            "A valid interface chain does not qualify any upstream hydraulic, ingress, transport or chemical model.",
            "Model outputs cannot substitute for measured pressure, signed physical defect flow or laboratory tracer observations.",
            "No contaminant, pathogen, health, toxicity, potability, SANS compliance or field claim is established.",
            "This evaluator does not authorize isolation, treatment, flushing, restart, notification or water release."
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
    if args.len() > 5 {
        return Err(
            "usage: intrusion-coupling-t0 [INPUTS] [ORACLE] [CONTRACT] [MANIFEST] [RECEIPT]"
                .to_owned(),
        );
    }

    let input_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/coupling-t0-cases-v0.1.json",
    ));
    let oracle_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/coupling-t0-oracle-v0.1.json",
    ));
    let contract_path = PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/model-coupling-interface-contract-v0.1.json",
    ));
    let manifest_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or(
        "research/water-intrusion-coupling/manifest-v0.1.json",
    ));
    let receipt_path = PathBuf::from(args.get(4).map(String::as_str).unwrap_or(
        "artifacts/water-intrusion-coupling-t0/receipt.json",
    ));

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (contract_bytes, contract) = read_json(&contract_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let evaluation = evaluate(&inputs, &oracle, &contract, &manifest)?;

    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"].as_array().is_some_and(Vec::is_empty);
    let success = total == CASE_COUNT as u64 && matched == CASE_COUNT as u64 && no_mismatches;

    let revision = env::var("WATER_INTRUSION_COUPLING_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;

    let receipt = json!({
        "schema_version": "water-intrusion-coupling-t0-run-v0.1",
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
        "contract_sha256": sha256(&contract_bytes),
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
            eprintln!("WATER_INTRUSION_COUPLING_T0_ERROR: {error}");
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
                "../../../../research/water-intrusion-coupling/coupling-t0-cases-v0.1.json"
            ),
            "oracle" => include_str!(
                "../../../../research/water-intrusion-coupling/coupling-t0-oracle-v0.1.json"
            ),
            "contract" => include_str!(
                "../../../../research/water-intrusion-coupling/model-coupling-interface-contract-v0.1.json"
            ),
            "manifest" => include_str!(
                "../../../../research/water-intrusion-coupling/manifest-v0.1.json"
            ),
            _ => panic!("unknown fixture"),
        };
        serde_json::from_str(source).expect("fixture must be valid JSON")
    }

    fn evaluated() -> Value {
        evaluate(
            &load("inputs"),
            &load("oracle"),
            &load("contract"),
            &load("manifest"),
        )
        .expect("coupling T0 evaluation must complete")
    }

    #[test]
    fn all_eight_coupling_cases_match_the_frozen_oracle() {
        let result = evaluated();
        assert_eq!(result["result_count"], CASE_COUNT);
        assert_eq!(result["matched_case_count"], CASE_COUNT);
        assert_eq!(result["verification_state"], "MatchedFrozenOracle");
        assert!(result["mismatches"].as_array().unwrap().is_empty());
    }

    #[test]
    fn valid_chain_is_only_accepted_for_bounded_model_comparison() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WC-T0-001-valid-five-stage-evidence-chain")
            .unwrap();
        assert_eq!(
            row["computed"]["interface_disposition"],
            "CHAIN_CONTRACT_VALID_FOR_BOUNDED_MODEL_COMPARISON"
        );
        assert_eq!(row["computed"]["qualification_status"], "NotRun");
    }

    #[test]
    fn modelled_pressure_cannot_substitute_for_measured_pressure() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["case_id"]
                    == "WC-T0-002-modelled-pressure-cannot-substitute-for-measured-pressure"
            })
            .unwrap();
        assert_eq!(row["computed"]["interface_disposition"], "INDETERMINATE");
    }

    #[test]
    fn timebase_mismatch_is_not_silently_aligned() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| {
                row["case_id"] == "WC-T0-003-timebase-mismatch-between-pressure-and-tracer"
            })
            .unwrap();
        assert_eq!(
            row["computed"]["reason"],
            "OBSERVATION_TIMEBASE_MISMATCH"
        );
    }

    #[test]
    fn upstream_digest_mismatch_rejects_the_chain() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WC-T0-004-upstream-output-digest-mismatch")
            .unwrap();
        assert_eq!(
            row["computed"]["interface_disposition"],
            "REJECTED_CONTRACT"
        );
    }

    #[test]
    fn out_of_domain_transient_blocks_downstream_use() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WC-T0-005-transient-stage-out-of-domain")
            .unwrap();
        assert_eq!(row["computed"]["interface_disposition"], "OUT_OF_DOMAIN");
    }

    #[test]
    fn missing_chemical_fate_is_not_assumed_inert() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WC-T0-006-chemical-fate-stage-missing")
            .unwrap();
        assert_eq!(
            row["computed"]["reason"],
            "CHEMICAL_FATE_NOT_MODELLED_NOT_INERT"
        );
    }

    #[test]
    fn unbounded_ingress_uncertainty_propagates_downstream() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WC-T0-007-ingress-uncertainty-unbounded")
            .unwrap();
        assert_eq!(row["computed"]["reason"], "UNCERTAINTY_UNBOUNDED");
    }

    #[test]
    fn raw_observation_digest_is_required() {
        let result = evaluated();
        let row = result["results"]
            .as_array()
            .unwrap()
            .iter()
            .find(|row| row["case_id"] == "WC-T0-008-observation-raw-digest-missing")
            .unwrap();
        assert_eq!(
            row["computed"]["reason"],
            "OBSERVATION_PROVENANCE_INCOMPLETE"
        );
    }

    #[test]
    fn duplicate_model_stage_is_rejected() {
        let mut inputs = load("inputs");
        let duplicate = inputs["cases"][0]["inputs"]["stages"][0].clone();
        inputs["cases"][0]["inputs"]["stages"]
            .as_array_mut()
            .unwrap()
            .push(duplicate);
        let evaluation = evaluate(
            &inputs,
            &load("oracle"),
            &load("contract"),
            &load("manifest"),
        )
        .expect("malformed stage order should yield a rejected case");
        assert_eq!(evaluation["verification_state"], "OracleMismatch");
    }

    #[test]
    fn reordered_model_stages_are_not_silently_accepted() {
        let mut inputs = load("inputs");
        inputs["cases"][0]["inputs"]["stages"]
            .as_array_mut()
            .unwrap()
            .swap(0, 1);
        let row = compute(&inputs["cases"][0]).unwrap();
        assert_eq!(
            row["interface_disposition"],
            "REJECTED_CONTRACT"
        );
        assert_eq!(row["reason"], "MODEL_STAGE_ORDER_MISMATCH");
    }

    #[test]
    fn tampered_oracle_cannot_authorize_release() {
        let mut oracle = load("oracle");
        oracle["cases"][0]["expected"]["release_authorized"] = json!(true);
        assert!(evaluate(
            &load("inputs"),
            &oracle,
            &load("contract"),
            &load("manifest")
        )
        .is_err());
    }
}
