#![forbid(unsafe_code)]

use geofusion_evidence_verifier::verify_source_interval;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;

fn read_json(path: &PathBuf, label: &'static str) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{}: cannot read {}: {}", label, path.display(), error))?;
    serde_json::from_str(&text)
        .map_err(|_| format!("{}: invalid JSON in {}", label, path.display()))
}

fn run() -> Result<i32, String> {
    let mut args = env::args_os().skip(1);
    let payload_path = args.next().map(PathBuf::from)
        .ok_or_else(|| {
            concat!(
                "usage: geofusion-evidence-verifier <raw-payload.json> ",
                "<evidence-envelope.json> <allocation-receipt.json>"
            )
            .to_owned()
        })?;
    let evidence_path = args.next().map(PathBuf::from)
        .ok_or_else(|| "missing evidence-envelope.json argument".to_owned())?;
    let allocation_path = args.next().map(PathBuf::from)
        .ok_or_else(|| "missing allocation-receipt.json argument".to_owned())?;
    if args.next().is_some() {
        return Err("too many arguments; expected exactly three file paths".to_owned());
    }

    let payload = fs::read(&payload_path)
        .map_err(|error| format!("cannot read raw payload {}: {}", payload_path.display(), error))?;
    let evidence = read_json(&evidence_path, "evidence envelope")?;
    let allocation = read_json(&allocation_path, "allocation receipt")?;

    let report = verify_source_interval(&payload, &evidence, &allocation)
        .map_err(|error| error.to_string())?;
    let output = serde_json::to_string_pretty(&report.to_json())
        .map_err(|error| format!("cannot serialize verification report: {error}"))?;
    println!("{output}");

    Ok(if report.qualified { 0 } else { 2 })
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("{error}");
            std::process::exit(1);
        }
    }
}
