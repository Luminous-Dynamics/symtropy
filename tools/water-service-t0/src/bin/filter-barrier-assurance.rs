// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use water_service_t0::evaluate_filter_barrier_assurance;

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
            "usage: filter-barrier-assurance [INPUTS] [ORACLE] [SCHEMA] [MANIFEST] [RECEIPT]"
                .to_owned(),
        );
    }
    let input_path =
        PathBuf::from(args.first().map(String::as_str).unwrap_or(
            "research/water-service-engineering/filter-barrier-assurance-cases-v0.1.json",
        ));
    let oracle_path =
        PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
            "research/water-service-engineering/filter-barrier-assurance-oracle-v0.1.json",
        ));
    let schema_path = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("research/water-service-engineering/filter-barrier-assurance.schema.json"),
    );
    let manifest_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or(
        "research/water-service-engineering/filter-barrier-assurance-manifest-v0.1.json",
    ));
    let receipt_path = PathBuf::from(
        args.get(4)
            .map(String::as_str)
            .unwrap_or("artifacts/filter-barrier-assurance/receipt.json"),
    );

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let (schema_bytes, schema) = read_json(&schema_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;

    if manifest["benchmark_id"] != "FILTER-BARRIER-ASSURANCE-M0-001"
        || manifest["case_count"] != 9
        || manifest["evidence_ceiling"] != "no_filter_or_water_safety_claim"
        || schema["$schema"] != "https://json-schema.org/draft/2020-12/schema"
    {
        return Err(
            "filter barrier schema/manifest identity or evidence ceiling mismatch".to_owned(),
        );
    }

    let evaluation = evaluate_filter_barrier_assurance(&inputs, &oracle)?;
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"]
        .as_array()
        .is_some_and(Vec::is_empty);
    let all_non_authorizing = evaluation["results"].as_array().is_some_and(|rows| {
        rows.iter().all(|row| {
            !row["computed"]["potability_or_compliance_claim_allowed"]
                .as_bool()
                .unwrap_or(true)
        })
    });
    let success = total == 9 && matched == 9 && no_mismatches && all_non_authorizing;

    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes =
        fs::read(&lock_path).map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let raw_revision = env::var("FILTER_BARRIER_ASSURANCE_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|value| valid_revision(value));
    let receipt = json!({
        "schema_version": "filter-barrier-assurance-run-v0.1",
        "verification_state": if !success {
            "OracleMismatchOrSafetyGuardFailure"
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
        "all_potability_compliance_claims_false": all_non_authorizing,
        "mismatches": evaluation["mismatches"].clone(),
        "results": evaluation["results"].clone(),
        "evidence_class": "synthetic_barrier_state_contract_fixture",
        "evidence_ceiling": "synthetic_contract_semantics_only_no_filter_or_water_safety_claim",
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
            eprintln!("FILTER_BARRIER_ASSURANCE_ERROR: {error}");
            std::process::exit(1);
        }
    }
}
