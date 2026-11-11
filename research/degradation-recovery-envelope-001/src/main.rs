//! JSON CLI for paired degradation/recovery campaign auditing.
//!
//! Usage:
//!   degradation-recovery-envelope <campaign.json> <report.json>
//!
//! Exit codes: 0 valid, 1 rejected campaign (report is written), 2 input error.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use degradation_recovery_envelope::{audit_campaign, AuditReport, Campaign};
use serde_json::json;

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

fn audit_value(input: &str) -> Result<AuditReport, String> {
    let campaign: Campaign = serde_json::from_str(input)
        .map_err(|error| format!("campaign JSON does not match the typed schema: {error}"))?;
    Ok(audit_campaign(&campaign))
}

fn write_report(path: &Path, report: &AuditReport) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(report)?)?;
    Ok(())
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err(
            "usage: degradation-recovery-envelope <campaign.json> <report.json>".into(),
        );
    }
    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("campaign input must be a regular file no larger than 64 MiB".into());
    }
    let input = fs::read_to_string(input_path)?;
    let report = audit_value(&input)
        .map_err(|error| format!("campaign input rejected: {error}"))?;
    write_report(output_path, &report)?;
    println!(
        "degradation/recovery audit: {} ({} findings); report: {}",
        report.disposition,
        report.findings.len(),
        output_path.display()
    );
    Ok(if report.disposition == "valid" { 0 } else { 1 })
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("degradation-recovery-envelope: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn paired_camera_drop_fixture_is_valid() {
        let report = audit_value(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        assert_eq!(report.disposition, "valid");
        assert!(report.findings.is_empty());
        assert_eq!(report.expected_ticks, 8);
        assert_eq!(report.events[0].recovery_tick, Some(7));
    }

    #[test]
    fn cli_report_retains_each_metric_axis_separately() {
        let report = audit_value(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        let rendered = serde_json::to_value(report).unwrap();
        assert!(rendered.get("clean_control_decision_latency").is_some());
        assert!(rendered.get("faulted_decision_latency").is_some());
        assert!(rendered.get("clean_control_inference_energy").is_some());
        assert!(rendered.get("faulted_inference_energy").is_some());
        assert!(rendered.get("composite_score").is_none());
    }

    #[test]
    fn malformed_unknown_fields_are_rejected_by_typed_cli() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        value["unaccounted_simulator_privilege"] = json!("ground-truth-world-state");
        assert!(audit_value(&value.to_string()).is_err());
    }

    #[test]
    fn rejected_campaign_still_produces_a_deterministic_report() {
        let mut value: serde_json::Value =
            serde_json::from_str(include_str!("../fixtures/camera-drop-paired-v1.json")).unwrap();
        value["faulted_run"]["ticks"][2]["policy_input_accepted"] = json!(true);
        value["faulted_run"]["ticks"][2]["conditions"] = json!(["stale"]);
        value["events"][0]["kind"] = json!("stale_replay");
        let first = audit_value(&value.to_string()).unwrap();
        let second = audit_value(&value.to_string()).unwrap();
        assert_eq!(first, second);
        assert_eq!(first.disposition, "rejected");
        assert!(first.findings.iter().any(|finding| {
            finding == "forbidden_policy_input_accepted"
        }));
    }
}
