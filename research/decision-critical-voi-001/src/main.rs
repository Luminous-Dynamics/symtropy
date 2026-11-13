//! JSON CLI for the decision-critical VOI allocator.
//!
//! Usage:
//!   decision-critical-voi <request.json> <report.json>
//!
//! Exit codes: 0 = request validated and report written; 2 = invalid input,
//! invalid decision context, or I/O error. Candidate rejections are represented
//! in the report and are not process errors.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use decision_critical_voi::{allocate, DecisionReport, DecisionRequest, RejectionReason};
use serde::Serialize;

const MAX_INPUT_BYTES: u64 = 32 * 1024 * 1024;

#[derive(Debug, Serialize)]
struct RejectedRequestReport {
    schema_version: u32,
    disposition: &'static str,
    rejection_reasons: Vec<&'static str>,
    claim_ceiling: &'static str,
}

#[derive(Debug, Serialize)]
#[serde(untagged)]
enum CliReport {
    Decision(DecisionReport),
    Rejected(RejectedRequestReport),
}

fn evaluate(input: &str) -> Result<CliReport, String> {
    let request: DecisionRequest = serde_json::from_str(input)
        .map_err(|error| format!("decision request does not match the typed schema: {error}"))?;
    match allocate(&request) {
        Ok(report) => Ok(CliReport::Decision(report)),
        Err(reasons) => Ok(CliReport::Rejected(RejectedRequestReport {
            schema_version: 1,
            disposition: "request_rejected",
            rejection_reasons: reasons.iter().copied().map(RejectionReason::as_str).collect(),
            claim_ceiling: "The request failed structural or decision-context validation; no candidate was selected and no query was executed.",
        })),
    }
}

fn write_report(path: &Path, report: &CliReport) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(report)?)?;
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: decision-critical-voi <request.json> <report.json>".into());
    }
    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("decision request must be a regular file no larger than 32 MiB".into());
    }
    let input = fs::read_to_string(input_path)?;
    let report = evaluate(&input).map_err(|error| format!("decision request rejected: {error}"))?;
    write_report(output_path, &report)?;
    let disposition = match &report {
        CliReport::Decision(report) => report.disposition.as_str(),
        CliReport::Rejected(report) => report.disposition,
    };
    println!(
        "decision-critical VOI: {disposition}; report: {}",
        output_path.display()
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("decision-critical-voi: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::Value;

    #[test]
    fn example_decision_request_produces_resource_separated_report() {
        let report = evaluate(include_str!("../fixtures/pedestrian-occlusion-decision-v1.json"))
            .unwrap();
        let rendered = serde_json::to_value(report).unwrap();
        assert_eq!(rendered["disposition"], "pareto_options_available");
        assert_eq!(rendered["baseline_expected_loss_units"], 50);
        assert_eq!(
            rendered["pareto_frontier_query_ids"].as_array().unwrap().len(),
            2
        );
        assert_eq!(rendered["recommended_query_id"], Value::Null);
        assert!(
            rendered["claim_ceiling"]
                .as_str()
                .unwrap()
                .contains("probability calibration")
        );
    }

    #[test]
    fn malformed_or_unknown_request_fields_are_rejected() {
        let mut request: Value = serde_json::from_str(include_str!(
            "../fixtures/pedestrian-occlusion-decision-v1.json"
        ))
        .unwrap();
        request["hidden_evaluator_future_labels"] = Value::Bool(true);
        assert!(evaluate(&request.to_string()).is_err());
    }

    #[test]
    fn invalid_current_action_is_reported_as_typed_request_rejection() {
        let mut request: Value = serde_json::from_str(include_str!(
            "../fixtures/pedestrian-occlusion-decision-v1.json"
        ))
        .unwrap();
        request["context"]["current_action_id"] = Value::String("proceed".to_string());
        let report = evaluate(&request.to_string()).unwrap();
        let rendered = serde_json::to_value(report).unwrap();
        assert_eq!(rendered["disposition"], "request_rejected");
        assert!(
            rendered["rejection_reasons"]
                .as_array()
                .unwrap()
                .contains(&Value::String("current_action_not_best_estimate".to_string()))
        );
    }
}
