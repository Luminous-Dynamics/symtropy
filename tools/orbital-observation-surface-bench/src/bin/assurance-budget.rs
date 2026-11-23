use orbital_observation_surface_bench::assurance_budget::{AssuranceBudgetCorpus, evaluate};
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
        eprintln!("usage: assurance-budget <corpus.json> --source-commit <sha>");
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
            eprintln!("failed to read assurance budget corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: AssuranceBudgetCorpus = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to parse assurance budget corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let evaluation = match evaluate(&corpus) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("assurance budget evaluation failed: {error}");
            return ExitCode::from(1);
        }
    };
    let receipt = json!({
        "receipt_schema":"luminous.orbital-assurance-budget.receipt.v0.1",
        "source_commit":source_commit,
        "corpus_sha256":sha256_hex(&raw),
        "result":evaluation.result,
        "evidence_class":evaluation.evidence_class,
        "claim_ceiling":evaluation.claim_ceiling,
        "assurance_budget_id":evaluation.assurance_budget_id,
        "requested_scope":evaluation.requested_scope,
        "requested_scope_id":evaluation.requested_scope_id,
        "status":evaluation.status,
        "total_alpha":evaluation.total_alpha,
        "alpha_allocated":evaluation.alpha_allocated,
        "alpha_used":evaluation.alpha_used,
        "alpha_remaining":evaluation.alpha_remaining,
        "joint_lower_confidence":evaluation.joint_lower_confidence,
        "missing_required_classes":evaluation.missing_required_classes,
        "noncomposable_claim_ids":evaluation.noncomposable_claim_ids,
        "claim_reports":evaluation.claim_reports,
        "independence_assumed":evaluation.independence_assumed,
        "physical_qualification_granted":evaluation.physical_qualification_granted,
        "authority_granted":evaluation.authority_granted
    });
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => {
            println!("{output}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("failed to serialize assurance budget receipt: {error}");
            ExitCode::from(1)
        }
    }
}
