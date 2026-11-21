use orbital_reliability_plp_trend_bench::{Corpus, evaluate};
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    let mut output = String::with_capacity(64);
    for byte in digest {
        use std::fmt::Write as _;
        let _ = write!(output, "{byte:02x}");
    }
    output
}

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let fixture_path = args.next().ok_or_else(|| {
        "usage: orbital-reliability-plp-trend <cases.json> --source-commit <40-hex-sha>".to_owned()
    })?;
    if args.next().as_deref() != Some("--source-commit") {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args
        .next()
        .ok_or_else(|| "missing source commit value".to_owned())?;
    if source_commit.len() != 40
        || !source_commit
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err("source commit must be exactly 40 lowercase hexadecimal characters".to_owned());
    }
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }

    let input = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read PLP trend corpus {fixture_path}: {error}"))?;
    let corpus: Corpus = serde_json::from_slice(&input)
        .map_err(|error| format!("invalid PLP trend corpus JSON: {error}"))?;
    let report = evaluate(&corpus)?;
    let receipt = json!({
        "receipt_schema": "luminous.orbital-reliability-plp-trend.receipt.v0.1",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "source_commit": source_commit,
        "fixture_sha256": sha256_hex(&input),
        "result": report.result,
        "report": report,
        "fit_acceptance_granted": false,
        "reliability_claim_granted": false,
        "authority_granted": false
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize PLP trend receipt: {error}"))?
    );
    if receipt["result"] == "pass" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("orbital-reliability-plp-trend: {error}");
            ExitCode::FAILURE
        }
    }
}
