// SPDX-License-Identifier: AGPL-3.0-or-later
use hydro_contextual_t0::evaluate;
use serde_json::{Value, json};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

fn read_json(path: &Path) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|e| format!("read {}: {e}", path.display()))?;
    let value =
        serde_json::from_slice(&bytes).map_err(|e| format!("parse {}: {e}", path.display()))?;
    Ok((bytes, value))
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 4 {
        return Err("usage: hydro-contextual-t0 [INPUTS] [MANIFEST] [ORACLE] [RECEIPT]".to_owned());
    }
    let input_path = PathBuf::from(
        args.first()
            .map(String::as_str)
            .unwrap_or("research/hydrology/hydro-contextual-t0-v0.1.json"),
    );
    let manifest_path = PathBuf::from(
        args.get(1)
            .map(String::as_str)
            .unwrap_or("research/hydrology/hydro-contextual-manifest-v0.1.json"),
    );
    let oracle_path = PathBuf::from(
        args.get(2)
            .map(String::as_str)
            .unwrap_or("research/hydrology/hydro-contextual-oracle-v0.1.json"),
    );
    let receipt_path = PathBuf::from(
        args.get(3)
            .map(String::as_str)
            .unwrap_or("artifacts/hydro-contextual-t0/receipt.json"),
    );
    let (input_bytes, inputs) = read_json(&input_path)?;
    let (manifest_bytes, manifest) = read_json(&manifest_path)?;
    let (oracle_bytes, oracle) = read_json(&oracle_path)?;
    let result = evaluate(&inputs, &manifest, &oracle)?;
    let count = result["result_count"].as_u64().unwrap_or(0);
    let matched = result["matched_case_count"].as_u64().unwrap_or(0);
    let success =
        count == 37 && matched == 37 && result["mismatches"].as_array().is_some_and(Vec::is_empty);
    let raw_revision = env::var("HYDRO_CONTEXTUAL_T0_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|s| valid_revision(s));
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../Cargo.lock");
    let lock = fs::read(&lock_path).map_err(|e| format!("read {}: {e}", lock_path.display()))?;
    let receipt = json!({
      "schema_version":"hydro-contextual-t0-run-v0.1",
      "verification_state":if !success{"OracleMismatch"}else if revision.is_some(){"PassedExactRevision"}else{"PassedLocalUnpinned"},
      "qualification_status":"NotRun",
      "implementation_revision":revision,
      "inputs_sha256":hydro_contextual_t0::sha256(&input_bytes),
      "manifest_sha256":hydro_contextual_t0::sha256(&manifest_bytes),
      "oracle_sha256":hydro_contextual_t0::sha256(&oracle_bytes),
      "cargo_lock_sha256":hydro_contextual_t0::sha256(&lock),
      "result_count":count,
      "matched_case_count":matched,
      "mismatches":result["mismatches"].clone(),
      "results":result["results"].clone(),
      "evidence_class":"synthetic_only",
      "claim_ceiling":hydro_contextual_t0::CLAIM_CEILING,
      "nonclaims":result["nonclaims"].clone()
    });
    if let Some(parent) = receipt_path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent).map_err(|e| format!("create {}: {e}", parent.display()))?;
    }
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|e| format!("serialize receipt: {e}"))?
        + "\n";
    fs::write(&receipt_path, serialized.as_bytes())
        .map_err(|e| format!("write {}: {e}", receipt_path.display()))?;
    println!("{serialized}");
    Ok(if success { 0 } else { 2 })
}
fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(e) => {
            eprintln!("HYDRO_CONTEXTUAL_T0_ERROR: {e}");
            std::process::exit(1);
        }
    }
}
