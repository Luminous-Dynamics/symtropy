#[path = "../sumo_behavior_case.rs"]
mod evaluator;
#[path = "../sumo_behavior_xml.rs"]
mod xml;

use serde_json::{Value, json};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

const SUMO_VERSION: &str = "1.27.1";

fn hash(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: wildfire_sumo_behavior <fixture.json> <sumo-output-root> <source-commit>"
                .into(),
        );
    }
    if args[3].len() != 40 || !args[3].bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err("source commit must be a 40-character hexadecimal SHA".into());
    }
    let fixture_path = Path::new(&args[1]);
    let fixture_bytes =
        fs::read(fixture_path).map_err(|e| format!("read fixture: {e}"))?;
    let fixture: Value =
        serde_json::from_slice(&fixture_bytes).map_err(|e| format!("parse fixture: {e}"))?;
    if fixture.get("benchmark_id").and_then(Value::as_str) != Some("WILDFIRE-NETWORK-001")
        || fixture.get("classification").and_then(Value::as_str) != Some("synthetic_only")
    {
        return Err("fixture ID/classification mismatch".into());
    }
    let cases = fixture
        .get("cases")
        .and_then(Value::as_array)
        .ok_or_else(|| "fixture cases must be an array".to_owned())?;
    if cases.len() != 6 {
        return Err("expected exactly six frozen synthetic cases".into());
    }
    let root = PathBuf::from(&args[2]);
    let mut results = Vec::new();
    for case in cases {
        results.push(evaluator::evaluate(case, &root)?);
    }
    let report = json!({
        "result": "pass",
        "benchmark_id": "WILDFIRE-SUMO-BEHAVIOR-001",
        "source_commit": args[3],
        "fixture_sha256": hash(&fixture_bytes),
        "sumo_version": SUMO_VERSION,
        "case_count": results.len(),
        "cases": results,
        "claim_boundary": "SUMO-native observations over synthetic fixtures; source capacity/storage, closure, sink and accessibility semantics are not yet matched to Symtropy"
    });
    let out = root.join("sumo-behavior-summary.json");
    let bytes =
        serde_json::to_vec_pretty(&report).map_err(|e| format!("serialize summary: {e}"))?;
    fs::write(&out, &bytes).map_err(|e| format!("write {}: {e}", out.display()))?;
    println!("{}", String::from_utf8_lossy(&bytes));
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("SUMO behavior verification failed: {error}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_digest_commits_to_exact_fixture_bytes() {
        assert_eq!(hash(b"fixture"), hash(b"fixture"));
        assert_ne!(hash(b"fixture"), hash(b"fixture\n"));
    }

    #[test]
    fn pinned_sumo_version_is_explicit() {
        assert_eq!(SUMO_VERSION, "1.27.1");
    }
}
