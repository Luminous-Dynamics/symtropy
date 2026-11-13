// SPDX-License-Identifier: AGPL-3.0-or-later
use community_water_service_floor_validator::{evaluate, sha256_hex};
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::PathBuf;

fn read_json(path: &PathBuf) -> Result<(Vec<u8>, Value), String> {
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
        return Err("usage: community-water-service-floor-validator [SCENARIOS] [SCHEMA] [ORACLE] [RECEIPT]".to_owned());
    }
    let scenarios_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "research/water-service-engineering/community-service-floors/scenarios-v0.1.json",
    ));
    let schema_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "research/water-service-engineering/community-service-floors/scenarios-v0.1.schema.json",
    ));
    let oracle_path = PathBuf::from(args.get(2).map(String::as_str).unwrap_or(
        "research/water-service-engineering/community-service-floors/disposition-oracle-v0.1.json",
    ));
    let receipt_path = PathBuf::from(args.get(3).map(String::as_str).unwrap_or(
        "artifacts/community-water-service-floor/receipt.json",
    ));

    let (scenarios_bytes, scenarios) = read_json(&scenarios_path)?;
    let (schema_bytes, schema) = read_json(&schema_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let evaluation = evaluate(&scenarios, &schema, &oracle)
        .map_err(|errors| errors.join("\n"))?;

    let manifest_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let manifest_path = manifest_dir.join("Cargo.toml");
    let lock_path = manifest_dir.join("Cargo.lock");
    let manifest_bytes = fs::read(&manifest_path)
        .map_err(|error| format!("read {}: {error}", manifest_path.display()))?;
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("read {}: {error}", lock_path.display()))?;
    let revision = env::var("COMMUNITY_SERVICE_FLOOR_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok()
        .filter(|value| valid_revision(value));
    let receipt = json!({
        "schema_version": "community-water-service-floor-run-v0.1",
        "verification_state": evaluation["verification_state"],
        "qualification_status": "NotRun",
        "implementation_revision": revision,
        "scenarios_sha256": sha256_hex(&scenarios_bytes),
        "schema_sha256": sha256_hex(&schema_bytes),
        "oracle_sha256": sha256_hex(&oracle_bytes),
        "cargo_manifest_sha256": sha256_hex(&manifest_bytes),
        "cargo_lock_sha256": sha256_hex(&lock_bytes),
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
            eprintln!("COMMUNITY_WATER_SERVICE_FLOOR_ERROR: {error}");
            std::process::exit(1);
        }
    }
}
