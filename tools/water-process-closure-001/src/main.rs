use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::{Path, PathBuf};
use water_process_closure_001::{evaluate_corpus, CaseResult, InputCorpus, Oracle};

#[derive(Debug, Serialize)]
struct Mismatch {
    case_id: String,
    expected: CaseResult,
    actual: CaseResult,
}

#[derive(Debug, Serialize)]
struct Receipt {
    schema_version: &'static str,
    verification_state: &'static str,
    qualification_status: &'static str,
    implementation_revision: Option<String>,
    input_sha256: String,
    oracle_sha256: String,
    result_count: usize,
    matched_case_count: usize,
    mismatches: Vec<Mismatch>,
    results: Vec<CaseResult>,
    claim_ceiling: &'static str,
    nonclaims: Vec<&'static str>,
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn valid_revision(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<(Vec<u8>, T), String> {
    let bytes = fs::read(path).map_err(|err| format!("read {}: {err}", path.display()))?;
    let value = serde_json::from_slice(&bytes)
        .map_err(|err| format!("parse {}: {err}", path.display()))?;
    Ok((bytes, value))
}

fn run() -> Result<i32, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 3 {
        return Err(
            "usage: water-process-closure-001 [INPUT_JSON] [ORACLE_JSON] [RECEIPT_JSON]"
                .to_owned(),
        );
    }
    let input_path = PathBuf::from(args.first().map(String::as_str).unwrap_or(
        "fixtures/water-process-closure-001-inputs-v1.json",
    ));
    let oracle_path = PathBuf::from(args.get(1).map(String::as_str).unwrap_or(
        "fixtures/water-process-closure-001-oracle-v1.json",
    ));
    let receipt_path = args.get(2).map(PathBuf::from);

    let (input_bytes, corpus): (Vec<u8>, InputCorpus) = read_json(&input_path)?;
    let (oracle_bytes, oracle): (Vec<u8>, Oracle) = read_json(&oracle_path)?;
    if oracle.schema_version != "water-process-closure-001-oracle-v1"
        || oracle.qualification_status != "NotRun"
        || oracle.claim_ceiling != "WaterProcessAccountingContractOnly"
        || !oracle.independently_hand_calculated
    {
        return Err("oracle identity or claim ceiling is invalid".to_owned());
    }

    let actual = evaluate_corpus(&corpus)?;
    let mut expected = oracle.results;
    expected.sort_by(|a, b| a.case_id.cmp(&b.case_id));
    let mut mismatches = Vec::new();
    for row in &actual {
        match expected.iter().find(|candidate| candidate.case_id == row.case_id) {
            Some(expected_row) if expected_row == row => {}
            Some(expected_row) => mismatches.push(Mismatch {
                case_id: row.case_id.clone(),
                expected: expected_row.clone(),
                actual: row.clone(),
            }),
            None => return Err(format!("oracle has no row for case {}", row.case_id)),
        }
    }
    if actual.len() != expected.len() {
        return Err(format!(
            "oracle/input result count mismatch: input={}, oracle={}",
            actual.len(),
            expected.len()
        ));
    }

    let raw_revision = env::var("WATER_PROC_CLOSURE_REVISION")
        .or_else(|_| env::var("GITHUB_SHA"))
        .ok();
    let revision = raw_revision.filter(|value| valid_revision(value));
    let matches = mismatches.is_empty();
    let verification_state = if !matches {
        "OracleMismatch"
    } else if revision.is_some() {
        "PassedExactRevision"
    } else {
        "PassedLocalUnpinned"
    };
    let receipt = Receipt {
        schema_version: "water-process-closure-001-run-v1",
        verification_state,
        qualification_status: "NotRun",
        implementation_revision: revision,
        input_sha256: sha256_hex(&input_bytes),
        oracle_sha256: sha256_hex(&oracle_bytes),
        result_count: actual.len(),
        matched_case_count: actual.len().saturating_sub(mismatches.len()),
        mismatches,
        results: actual,
        claim_ceiling: "WaterProcessAccountingContractOnly",
        nonclaims: vec![
            "Synthetic accounting reference only; not a treatment simulation or plant qualification.",
            "Mass closure does not establish contaminant removal, microbial safety, reuse eligibility, or discharge compliance.",
            "The revision field is supplied by the execution wrapper; a digest is identity evidence, not source truth.",
            "This reference evaluator does not authorize physical or operational actions.",
        ],
    };
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|err| format!("serialize receipt: {err}"))? + "\n";
    if let Some(path) = receipt_path {
        if let Some(parent) = path.parent() {
            fs::create_dir_all(parent)
                .map_err(|err| format!("create {}: {err}", parent.display()))?;
        }
        fs::write(&path, serialized.as_bytes())
            .map_err(|err| format!("write {}: {err}", path.display()))?;
    }
    println!("{serialized}");
    if matches { Ok(0) } else { Ok(2) }
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("WATER_PROCESS_CLOSURE_ERROR: {error}");
            std::process::exit(1);
        }
    }
}
