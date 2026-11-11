//! JSON CLI for paired clean/perturbed deployment fault campaigns.
//!
//! Usage:
//!   paired-deployment-fault-campaign <campaign.json> <report.json>
//!
//! Exit codes: 0 = structurally valid, 1 = structurally rejected, 2 = input or
//! invocation error. A report is written for both valid and rejected campaigns.

use paired_deployment_fault_campaign::{AuditDisposition, FaultCampaign};
use serde_json::Value;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

const MAX_CAMPAIGN_BYTES: u64 = 64 * 1024 * 1024;

fn audit_value(input: &str) -> Result<(Value, bool), String> {
    let campaign = FaultCampaign::from_json(input)?;
    let report = campaign.audit();
    let valid = report.disposition == AuditDisposition::Valid;
    let value = serde_json::to_value(report)
        .map_err(|error| format!("cannot serialize audit report: {error}"))?;
    Ok((value, valid))
}

fn write_report(path: &Path, report: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(report)?)?;
    Ok(())
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err(
            "usage: paired-deployment-fault-campaign <campaign.json> <report.json>".into(),
        );
    }
    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_CAMPAIGN_BYTES {
        return Err("campaign input must be a regular file no larger than 64 MiB".into());
    }
    let input = fs::read_to_string(input_path)?;
    let (report, valid) = audit_value(&input)
        .map_err(|error| format!("campaign input rejected: {error}"))?;
    write_report(output_path, &report)?;
    println!(
        "fault campaign: {} ({} pairs, {} findings); report: {}",
        report["disposition"].as_str().unwrap_or("unknown"),
        report["observed_pair_count"].as_u64().unwrap_or(0),
        report["findings"].as_array().map_or(0, Vec::len),
        output_path.display()
    );
    Ok(if valid { 0 } else { 1 })
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("paired-deployment-fault-campaign: {error}");
            std::process::exit(2);
        }
    }
}
