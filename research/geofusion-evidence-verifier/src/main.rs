#![forbid(unsafe_code)]

use geofusion_evidence_verifier::{
    SourceIntervalInput, verify_source_interval, verify_source_interval_batch,
};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const USAGE: &str = concat!(
    "usage: geofusion-evidence-verifier <raw-payload.json> ",
    "<evidence-envelope.json> <allocation-receipt.json>\n",
    "   or: geofusion-evidence-verifier --batch <batch-manifest.json>"
);

fn read_json(path: &Path, label: &'static str) -> Result<Value, String> {
    let text = fs::read_to_string(path)
        .map_err(|error| format!("{label}: cannot read {}: {error}", path.display()))?;
    serde_json::from_str(&text)
        .map_err(|_| format!("{label}: invalid JSON in {}", path.display()))
}

fn required_path<'a>(
    value: &'a Value,
    key: &'static str,
) -> Result<&'a Path, String> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .filter(|value| !value.trim().is_empty())
        .ok_or_else(|| format!("batch manifest: missing or invalid {key}"))?;
    Ok(Path::new(text))
}

fn run_single(
    payload_path: &Path,
    evidence_path: &Path,
    allocation_path: &Path,
) -> Result<i32, String> {
    let payload = fs::read(payload_path)
        .map_err(|error| format!("cannot read raw payload {}: {error}", payload_path.display()))?;
    let evidence = read_json(evidence_path, "evidence envelope")?;
    let allocation = read_json(allocation_path, "allocation receipt")?;

    let report = verify_source_interval(&payload, &evidence, &allocation)
        .map_err(|error| error.to_string())?;
    let output = serde_json::to_string_pretty(&report.to_json())
        .map_err(|error| format!("cannot serialize verification report: {error}"))?;
    println!("{output}");

    Ok(if report.qualified { 0 } else { 2 })
}

fn run_batch(manifest_path: &Path) -> Result<i32, String> {
    let manifest = read_json(manifest_path, "batch manifest")?;
    if manifest.get("schema_version").and_then(Value::as_str) != Some("0.1.0") {
        return Err("batch manifest: unsupported schema_version".to_owned());
    }
    let items = manifest
        .get("items")
        .and_then(Value::as_array)
        .ok_or_else(|| "batch manifest: items must be an array".to_owned())?;
    if items.len() > 256 {
        return Err("batch manifest: maximum 256 source intervals".to_owned());
    }

    let mut payloads = Vec::<Vec<u8>>::with_capacity(items.len());
    let mut evidences = Vec::<Value>::with_capacity(items.len());
    let mut allocations = Vec::<Value>::with_capacity(items.len());

    for (index, item) in items.iter().enumerate() {
        let payload_path = required_path(item, "payload_path")?;
        let evidence_path = required_path(item, "evidence_path")?;
        let allocation_path = required_path(item, "allocation_path")?;

        payloads.push(fs::read(payload_path).map_err(|error| {
            format!(
                "batch item {index}: cannot read payload {}: {error}",
                payload_path.display()
            )
        })?);
        evidences.push(read_json(evidence_path, "batch evidence envelope")?);
        allocations.push(read_json(allocation_path, "batch allocation receipt")?);
    }

    let inputs: Vec<SourceIntervalInput<'_>> = (0..items.len())
        .map(|index| SourceIntervalInput {
            payload_bytes: &payloads[index],
            evidence: &evidences[index],
            allocation: &allocations[index],
        })
        .collect();

    let reports = verify_source_interval_batch(&inputs)
        .map_err(|error| error.to_string())?;
    let all_qualified = reports.iter().all(|report| report.qualified);
    let output = Value::Array(reports.iter().map(|report| report.to_json()).collect());
    let output = serde_json::to_string_pretty(&output)
        .map_err(|error| format!("cannot serialize batch report: {error}"))?;
    println!("{output}");

    Ok(if all_qualified { 0 } else { 2 })
}

fn run() -> Result<i32, String> {
    let mut args = env::args_os().skip(1);
    let first = args
        .next()
        .ok_or_else(|| USAGE.to_owned())?;

    if first.to_string_lossy() == "--batch" {
        let manifest_path = args
            .next()
            .map(PathBuf::from)
            .ok_or_else(|| "missing batch-manifest.json argument".to_owned())?;
        if args.next().is_some() {
            return Err("too many arguments for batch mode".to_owned());
        }
        return run_batch(&manifest_path);
    }

    let payload_path = PathBuf::from(first);
    let evidence_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing evidence-envelope.json argument".to_owned())?;
    let allocation_path = args
        .next()
        .map(PathBuf::from)
        .ok_or_else(|| "missing allocation-receipt.json argument".to_owned())?;
    if args.next().is_some() {
        return Err("too many arguments; expected exactly three file paths".to_owned());
    }

    run_single(&payload_path, &evidence_path, &allocation_path)
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
