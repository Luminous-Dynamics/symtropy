use orbital_observation_surface_bench::joint_loss_scenarios::{DecisionCorpus, evaluate};
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
    let mut arguments = env::args().skip(1);
    let Some(corpus_path) = arguments.next() else {
        eprintln!("usage: joint-loss-scenarios <corpus.json> --source-commit <sha>");
        return ExitCode::from(2);
    };
    let mut source_commit = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--source-commit" => source_commit = arguments.next(),
            _ => {
                eprintln!("unknown argument: {argument}");
                return ExitCode::from(2);
            }
        }
    }
    let Some(source_commit) = source_commit else {
        eprintln!("missing required --source-commit argument");
        return ExitCode::from(2);
    };
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        eprintln!("source commit must be a lowercase 40-character hexadecimal SHA");
        return ExitCode::from(2);
    }
    let raw = match fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read joint-loss corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: DecisionCorpus = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to parse joint-loss corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let evaluation = match evaluate(&corpus) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("joint-loss evaluation failed: {error}");
            return ExitCode::from(1);
        }
    };
    let receipt = json!({
        "receipt_schema":"luminous.orbital-joint-loss-scenario.receipt.v0.1",
        "source_commit":source_commit,
        "corpus_sha256":sha256_hex(&raw),
        "result":evaluation.result,
        "evidence_class":evaluation.evidence_class,
        "claim_ceiling":evaluation.claim_ceiling,
        "summary":evaluation.summary,
        "cases":evaluation.cases,
        "physical_qualification_granted":evaluation.physical_qualification_granted,
        "authority_granted":evaluation.authority_granted
    });
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to serialize joint-loss receipt: {error}");
            ExitCode::from(1)
        }
    }
}
