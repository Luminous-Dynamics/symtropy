use orbital_assurance_bundle_bench::coalition_consequence_binding::{Corpus, evaluate};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{env, fs, process::ExitCode};

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let fixture = args.next().ok_or("missing fixture path")?;
    if args.next().as_deref() != Some("--source-commit") {
        return Err("expected --source-commit".into());
    }
    let commit = args.next().ok_or("missing source commit")?;
    if args.next().is_some() {
        return Err("unexpected trailing arguments".into());
    }
    let valid_commit = commit.len() == 40
        && commit
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase());
    if !valid_commit {
        return Err("source commit must be 40 lowercase hex characters".into());
    }

    let bytes = fs::read(&fixture).map_err(|error| format!("cannot read fixture: {error}"))?;
    let corpus: Corpus =
        serde_json::from_slice(&bytes).map_err(|error| format!("cannot parse fixture: {error}"))?;
    let evaluation = evaluate(&corpus)?;
    let fixture_sha256 = Sha256::digest(&bytes)
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    let result = evaluation.result.clone();

    let receipt = json!({
        "receipt_schema":
            "luminous.orbital-coalition-consequence-binding.receipt.v0.1",
        "source_commit": commit,
        "fixture_sha256": fixture_sha256,
        "result": result,
        "evaluation": evaluation,
        "authority_granted": false
    });
    let serialized = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("cannot serialize receipt: {error}"))?;
    println!("{serialized}");

    if result == "pass" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("coalition-consequence-binding: {error}");
            ExitCode::FAILURE
        }
    }
}
