// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use water_service_t0::evaluate_membrane_observability_f0;

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
            "usage: membrane-observability-fault-f0 [INPUTS] [ORACLE] [SCHEMA] [MANIFEST] [RECEIPT]"
                .to_owned(),
        );
    }
    let input_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "research/water-service-engineering/membrane-observability-fault-cases-v0.1.json",
    ));
    let oracle_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "research/water-service-engineering/membrane-observability-fault-oracle-v0.1.json",
    ));
    let schema_path =
        PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
            "research/water-service-engineering/membrane-observability-fault.schema.json",
        ));
    let manifest_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or(
        "research/water-service-engineering/membrane-observability-fault-manifest-v0.1.json",
    ));
    let receipt_path = PathBuf::from(
        args.get(4)
            .map(String::as_str)
            .unwrap_or("artifacts/membrane-observability-fault-f0/receipt.json"),
    );

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (schema_bytes, schema) = read_json(&schema_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;

    if manifest["benchmark_id"] != "MEMBRANE-OBSERVABILITY-FAULT-F0-001"
        || manifest["scenario_count"] != 13
        || schema["$schema"] != "https://json-schema.org/draft/2020-12/schema"
        || manifest["status"] != "synthetic_residual_rule_baseline"
    {
        return Err("membrane observability F0 schema/manifest identity mismatch".to_owned());
    }

    let evaluation = evaluate_membrane_observability_f0(&inputs, &oracle)?;
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"]
        .as_array()
        .is_some_and(Vec::is_empty);
    let success = total == 13 && matched == 13 && no_mismatches;

    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes =
        fs::read(&lock_path).map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let raw_revision = env::var("MEMBRANE_OBSERVABILITY_F0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|value| valid_revision(value));
    let receipt = json!({
        "schema_version": "membrane-observability-fault-f0-run-v0.1",
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
        "schema_sha256": sha256(&schema_bytes),
        "manifest_sha256": sha256(&manifest_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "result_count": total,
        "matched_case_count": matched,
        "false_fouling_alarms_sensor_only": evaluation["false_fouling_alarms_sensor_only"].clone(),
        "false_fouling_alarms_regime_shift_only": evaluation["false_fouling_alarms_regime_shift_only"].clone(),
        "false_cip_recommendations": evaluation["false_cip_recommendations"].clone(),
        "mismatches": evaluation["mismatches"].clone(),
        "results": evaluation["results"].clone(),
        "evidence_class": "synthetic_observability_challenge",
        "evidence_ceiling": "synthetic_diagnostic_baseline_only_no_real_sensor_or_membrane_diagnosis",
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
            eprintln!("MEMBRANE_OBSERVABILITY_F0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}
