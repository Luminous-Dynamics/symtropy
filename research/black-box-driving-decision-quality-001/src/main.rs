//! JSON CLI for the black-box driving decision-quality evaluator.
//!
//! Usage:
//!   black-box-driving-decision-quality <campaign.json> <report.json>
//!
//! Exit codes: 0 = structurally valid campaign, 1 = structurally rejected
//! campaign (a report is still written), 2 = invocation or input error.

use black_box_driving_decision_quality::{evaluate_campaign, AuditDisposition, CampaignRecord};
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

fn evaluate_json(input: &[u8]) -> Result<black_box_driving_decision_quality::DecisionQualityReport, String> {
    let campaign: CampaignRecord = serde_json::from_slice(input)
        .map_err(|error| format!("campaign JSON does not match the strict v1 schema: {error}"))?;
    Ok(evaluate_campaign(&campaign))
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: black-box-driving-decision-quality <campaign.json> <report.json>".into());
    }
    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("campaign input must be a regular file no larger than 64 MiB".into());
    }
    let input = fs::read(input_path)?;
    let report = evaluate_json(&input).map_err(|error| format!("campaign input rejected: {error}"))?;
    if let Some(parent) = output_path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "decision-quality audit: {:?}; episodes={}; decisions={}; findings={}; report={}",
        report.disposition,
        report.summary.observed_episodes,
        report.summary.observed_decisions,
        report.findings.len(),
        output_path.display()
    );
    Ok(if report.disposition == AuditDisposition::Valid { 0 } else { 1 })
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("black-box-driving-decision-quality: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn analytic_fixture_is_a_valid_json_campaign() {
        let input = include_bytes!("../examples/analytic-fixture-v1.json");
        let report = evaluate_json(input).unwrap();
        assert_eq!(report.disposition, AuditDisposition::Valid);
        assert_eq!(report.summary.expected_episodes, 1);
        assert_eq!(report.summary.observed_decisions, 1);
        assert_eq!(report.tier1_surrogate_safety.len(), 1);
        assert_eq!(report.tier2_outcome_prediction.len(), 1);
        assert_eq!(report.tier3_counterfactual_decision_quality.len(), 1);
        assert_eq!(report.tier4_systematic_weakness_diagnosis.len(), 1);
    }

    #[test]
    fn unknown_top_level_fields_are_rejected() {
        let input = br#"{
            "schema_version":1,
            "expected_episode_ids":[],
            "expected_decision_ids":[],
            "expected_surrogate_event_ids":[],
            "expected_prediction_ids":[],
            "episodes":[],
            "decisions":[],
            "surrogate_events":[],
            "outcome_predictions":[],
            "unexpected_privileged_state":true
        }"#;
        assert!(evaluate_json(input).is_err());
    }

    #[test]
    fn structurally_invalid_campaign_is_reported_not_silently_scored() {
        let input = include_bytes!("../examples/analytic-fixture-v1.json");
        let mut campaign: CampaignRecord = serde_json::from_slice(input).unwrap();
        campaign.expected_decision_ids.push("missing-decision".into());
        let report = evaluate_campaign(&campaign);
        assert_eq!(report.disposition, AuditDisposition::Rejected);
        assert!(report.tier3_counterfactual_decision_quality.is_empty());
        assert!(report.findings.iter().any(|finding| finding.contains("missing decisions")));
    }
}
