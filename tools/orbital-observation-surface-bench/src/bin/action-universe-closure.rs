use orbital_observation_surface_bench::action_universe_closure::{Corpus, evaluate};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

fn sha256_hex(bytes: &[u8]) -> String {
    Sha256::digest(bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(corpus_path) = args.next() else {
        eprintln!("usage: action-universe-closure <corpus.json> --source-commit <sha>");
        return ExitCode::from(2);
    };
    let mut source_commit = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--source-commit" => source_commit = args.next(),
            _ => {
                eprintln!("unknown argument: {arg}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(source_commit) = source_commit else {
        eprintln!("missing required --source-commit argument");
        return ExitCode::from(2);
    };
    if source_commit.len() != 40
        || !source_commit.bytes().all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        eprintln!("source commit must be a lowercase 40-character hexadecimal SHA");
        return ExitCode::from(2);
    }

    let raw = match fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read action-closure corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: Corpus = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to parse action-closure corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let evaluation = match evaluate(&corpus) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("action-universe closure evaluation failed: {error}");
            return ExitCode::from(1);
        }
    };
    let receipt = json!({
        "receipt_schema": "luminous.orbital-action-closure.receipt.v0.1",
        "source_commit": source_commit,
        "corpus_sha256": sha256_hex(&raw),
        "result": evaluation.result,
        "evidence_class": evaluation.evidence_class,
        "claim_ceiling": evaluation.claim_ceiling,
        "summary": evaluation.summary,
        "cases": evaluation.cases,
        "physical_qualification_granted": false,
        "authority_granted": false
    });
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to serialize action-closure receipt: {error}");
            ExitCode::from(1)
        }
    }
}
