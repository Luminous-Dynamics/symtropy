// SPDX-License-Identifier: AGPL-3.0-or-later
use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use water_service_t0::evaluate;

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
            "usage: water-service-t0 [INPUTS] [MANIFEST] [NETWORK_MANIFEST] [ORACLE] [RECEIPT]"
                .to_owned(),
        );
    }

    let input_path = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or("research/water-service-engineering/t0-known-answer-vectors-v0.1.json"),
    );
    let manifest_path = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("research/water-service-engineering/benchmark-manifest-v0.1.json"),
    );
    let network_manifest_path =
        PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
            "research/water-service-engineering/network-diagnostics-manifest-v0.1.json",
        ));
    let oracle_path = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("research/water-service-engineering/t0-known-answer-oracle-v0.1.json"),
    );
    let receipt_path = PathBuf::from(
        args.get(4)
            .map(String::as_str)
            .unwrap_or("artifacts/water-service-t0/receipt.json"),
    );

    let (input_bytes, inputs) = read_json(&input_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let (network_bytes, network_manifest) = read_json(&network_manifest_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;

    let evaluation = evaluate(&inputs, &manifest, &network_manifest, &oracle)?;
    let matched = evaluation["matched_case_count"].as_u64().unwrap_or(0);
    let total = evaluation["result_count"].as_u64().unwrap_or(0);
    let no_mismatches = evaluation["mismatches"]
        .as_array()
        .is_some_and(Vec::is_empty);
    let success = total == 23 && matched == 23 && no_mismatches;

    let raw_revision = env::var("WATER_SERVICE_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|value| valid_revision(value));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock_bytes =
        fs::read(&lock_path).map_err(|error| format!("read {}: {error}", lock_path.display()))?;

    let receipt = json!({
        "schema_version": "water-service-t0-run-v0.1",
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
        "manifest_sha256": sha256(&manifest_bytes),
        "network_manifest_sha256": sha256(&network_bytes),
        "oracle_sha256": sha256(&oracle_bytes),
        "cargo_lock_sha256": sha256(&lock_bytes),
        "result_count": total,
        "matched_case_count": matched,
        "mismatches": evaluation["mismatches"].clone(),
        "results": evaluation["results"].clone(),
        "evidence_class": "synthetic_only",
        "claim_ceiling": "arithmetic_and_contract_semantics_only_no_physical_or_health_claim",
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
            eprintln!("WATER_SERVICE_T0_ERROR: {error}");
            std::process::exit(1);
        }
    }
}
