use orbital_observation_surface_bench::{evaluate, observations_only};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn main() -> ExitCode {
    let mut arguments = env::args().skip(1);
    let Some(corpus_path) = arguments.next() else {
        eprintln!(
            "usage: orbital-observation-surface-bench <corpus.json> \
             --source-commit <sha> --observations-output <path>"
        );
        return ExitCode::from(2);
    };

    let mut source_commit = None;
    let mut observations_output = None;
    while let Some(argument) = arguments.next() {
        match argument.as_str() {
            "--source-commit" => source_commit = arguments.next(),
            "--observations-output" => observations_output = arguments.next(),
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
    let Some(observations_output) = observations_output else {
        eprintln!("missing required --observations-output argument");
        return ExitCode::from(2);
    };

    let raw = match fs::read(&corpus_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to read corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus = match serde_json::from_slice(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to parse observation-surface corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match evaluate(&corpus) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("observation-surface evaluation failed: {error}");
            return ExitCode::from(1);
        }
    };

    let corpus_sha256 = sha256_hex(&raw);
    let observations = observations_only(&report, &source_commit, &corpus_sha256);
    let observations_bytes = match serde_json::to_vec_pretty(&observations) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to serialize observed event stream: {error}");
            return ExitCode::from(1);
        }
    };
    if let Some(parent) = Path::new(&observations_output).parent()
        && !parent.as_os_str().is_empty()
        && let Err(error) = fs::create_dir_all(parent)
    {
        eprintln!("failed to create observation output directory: {error}");
        return ExitCode::from(1);
    }
    if let Err(error) = fs::write(&observations_output, &observations_bytes) {
        eprintln!("failed to write observed event stream: {error}");
        return ExitCode::from(1);
    }
    let observations_sha256 = sha256_hex(&observations_bytes);

    let receipt = json!({
        "receipt_schema": "luminous.orbital-observation-surface.receipt.v0.1",
        "source_commit": source_commit,
        "corpus_sha256": corpus_sha256,
        "observations_sha256": observations_sha256,
        "result": report.result,
        "report": report,
        "physical_reliability_claim_granted": false,
        "qualification_granted": false,
        "authority_granted": false
    });
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => {
            println!("{output}");
            if receipt["result"] == "pass" {
                ExitCode::SUCCESS
            } else {
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("failed to serialize receipt: {error}");
            ExitCode::from(1)
        }
    }
}
