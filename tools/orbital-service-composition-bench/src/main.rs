use orbital_service_composition_bench::{
    compose_receipt_bundle, evaluate, is_valid_source_commit, sha256_hex, Fixture,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn source_commit(args: &mut impl Iterator<Item = String>) -> Result<String, String> {
    let flag = args.next().ok_or_else(|| "missing --source-commit".to_owned())?;
    if flag != "--source-commit" {
        return Err("expected --source-commit".to_owned());
    }
    let commit = args.next().ok_or_else(|| "missing source commit value".to_owned())?;
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }
    if !is_valid_source_commit(&commit) {
        return Err("source commit must be exactly 40 hexadecimal characters".to_owned());
    }
    Ok(commit)
}

fn run_fixture(args: &mut impl Iterator<Item = String>) -> Result<ExitCode, String> {
    let fixture_path = args.next().ok_or_else(|| {
        "usage: orbital-service-composition-bench fixture <cases.json> --source-commit <40-hex-sha>"
            .to_owned()
    })?;
    let commit = source_commit(args)?;
    let bytes = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read fixture {fixture_path}: {error}"))?;
    let fixture: Fixture = serde_json::from_slice(&bytes)
        .map_err(|error| format!("invalid fixture JSON: {error}"))?;
    let report = evaluate(&fixture)?;
    let receipt = serde_json::json!({
        "receipt_schema": "luminous.orbital-service-composition.receipt.v0.1",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "source_commit": commit,
        "fixture_sha256": sha256_hex(&bytes),
        "result": report.result,
        "report": report
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize fixture receipt: {error}"))?
    );
    if receipt["result"] == "pass" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn run_compose(args: &mut impl Iterator<Item = String>) -> Result<ExitCode, String> {
    let composition_fixture_path = args.next().ok_or_else(|| {
        "usage: orbital-service-composition-bench compose <composition-cases.json> <delivery-receipt.json> <capture-receipt.json> <fluid-receipt.json> --source-commit <40-hex-sha>"
            .to_owned()
    })?;
    let delivery_path = args.next().ok_or_else(|| "missing delivery receipt path".to_owned())?;
    let capture_path = args.next().ok_or_else(|| "missing capture receipt path".to_owned())?;
    let fluid_path = args.next().ok_or_else(|| "missing fluid receipt path".to_owned())?;
    let commit = source_commit(args)?;

    let composition_fixture_bytes = fs::read(&composition_fixture_path)
        .map_err(|error| format!("cannot read composition fixture {composition_fixture_path}: {error}"))?;
    let delivery_bytes = fs::read(&delivery_path)
        .map_err(|error| format!("cannot read delivery receipt {delivery_path}: {error}"))?;
    let capture_bytes = fs::read(&capture_path)
        .map_err(|error| format!("cannot read capture receipt {capture_path}: {error}"))?;
    let fluid_bytes = fs::read(&fluid_path)
        .map_err(|error| format!("cannot read fluid receipt {fluid_path}: {error}"))?;

    let bundle = compose_receipt_bundle(
        &composition_fixture_bytes,
        &delivery_bytes,
        &capture_bytes,
        &fluid_bytes,
    )?;
    let result = bundle.result.clone();
    let receipt = serde_json::json!({
        "receipt_schema": "luminous.orbital-service-composition.cli-receipt.v0.1",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "source_commit": commit,
        "result": result,
        "bundle": bundle
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize joined receipt: {error}"))?
    );
    if receipt["result"] == "pass" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    match args.next().as_deref() {
        Some("fixture") => run_fixture(&mut args),
        Some("compose") => run_compose(&mut args),
        _ => Err(
            "usage: orbital-service-composition-bench fixture <cases.json> --source-commit <sha> | compose <cases.json> <delivery.json> <capture.json> <fluid.json> --source-commit <sha>"
                .to_owned(),
        ),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("orbital-service-composition-bench: {error}");
            ExitCode::FAILURE
        }
    }
}
