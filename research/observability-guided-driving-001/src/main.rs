//! Bounded JSON CLI for observability-guided driving campaign audits.
//!
//! Usage:
//!   observability-guided-driving <campaign.json> <report.json>
//!
//! Exit codes: 0 valid, 1 rejected campaign (report is still written), 2
//! invocation, I/O or typed-JSON error.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use observability_guided_driving::{AuditReport, Campaign, audit_campaign};
use serde_json::json;

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

fn audit_value(input: &str) -> Result<AuditReport, String> {
    let campaign: Campaign = serde_json::from_str(input)
        .map_err(|error| format!("campaign JSON does not match the typed schema: {error}"))?;
    Ok(audit_campaign(&campaign))
}

fn write_report(path: &Path, report: &AuditReport) -> Result<(), Box<dyn Error>> {
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
        return Err("usage: observability-guided-driving <campaign.json> <report.json>".into());
    }

    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("campaign input must be a regular file no larger than 64 MiB".into());
    }

    let input = fs::read_to_string(input_path)?;
    let report =
        audit_value(&input).map_err(|error| format!("campaign input rejected: {error}"))?;
    write_report(output_path, &report)?;

    println!(
        "observability audit: {} ({} findings); report: {}",
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
            eprintln!("observability-guided-driving: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../fixtures/occluded-crosswalk-paired-v1.json"
        ))
        .expect("checked-in fixture JSON is valid")
    }

    #[test]
    fn paired_occluded_crosswalk_fixture_is_valid() {
        let report = audit_value(include_str!(
            "../fixtures/occluded-crosswalk-paired-v1.json"
        ))
        .unwrap();
        assert_eq!(report.disposition, "valid");
        assert!(report.findings.is_empty());
        assert_eq!(report.expected_pair_count, 2);
        assert_eq!(report.observed_pair_count, 2);
        assert_eq!(report.active_information_action_count, 1);
    }

    #[test]
    fn fixture_shows_better_proper_score_after_safe_reposition() {
        let report = audit_value(include_str!(
            "../fixtures/occluded-crosswalk-paired-v1.json"
        ))
        .unwrap();
        assert!(
            report
                .active_minus_passive_crossing_brier
                .is_some_and(|delta| delta < 0.0)
        );
        assert!(
            report
                .active_minus_passive_crossing_log_loss
                .is_some_and(|delta| delta < 0.0)
        );
        assert_eq!(report.unsafe_information_action_count, 0);
        assert_eq!(report.no_safe_action_violations, 0);
    }

    #[test]
    fn evaluator_report_does_not_create_a_composite_observability_score() {
        let report = audit_value(include_str!(
            "../fixtures/occluded-crosswalk-paired-v1.json"
        ))
        .unwrap();
        let value = serde_json::to_value(report).unwrap();
        assert!(value.get("passive_control").is_some());
        assert!(value.get("active_observation").is_some());
        assert!(value.get("active_cost").is_some());
        assert!(value.get("collision_or_contact_count").is_some());
        assert!(value.get("composite_score").is_none());
        assert!(value.get("observability_score").is_none());
    }

    #[test]
    fn no_safe_information_action_negative_control_is_enforced_by_cli() {
        let mut campaign = fixture();
        campaign["pairs"][1]["active_observation"]["information_action"] = json!("safe_reposition");
        campaign["pairs"][1]["active_observation"]["action_cost"] =
            campaign["pairs"][0]["active_observation"]["action_cost"].clone();
        let report = audit_value(&campaign.to_string()).unwrap();
        assert_eq!(report.disposition, "rejected");
        assert!(
            report
                .findings
                .iter()
                .any(|finding| finding == "no_safe_action_violation")
        );
    }

    #[test]
    fn unknown_privileged_fields_are_rejected_by_typed_cli() {
        let mut campaign = fixture();
        campaign["debug_hidden_actor_pose"] = json!({"x": 1, "y": 2});
        assert!(audit_value(&campaign.to_string()).is_err());
    }
}
