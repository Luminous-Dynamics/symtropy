//! Evaluate a frozen selective-driving decision dataset.
//!
//! Usage:
//!   selective-driving-competence <input.json> <report.json>
//!
//! Exit code 0 means the declared records passed structural checks and metrics
//! were computed. It is not a safety approval or an authorization to drive.

use selective_driving_competence::EvaluationInput;
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

const MAX_INPUT_BYTES: u64 = 256 * 1024 * 1024;

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: selective-driving-competence <input.json> <report.json>".into());
    }
    let input_path = Path::new(&args[0]);
    let report_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("input must be a regular file no larger than 256 MiB".into());
    }
    let encoded = fs::read_to_string(input_path)?;
    let rendered_report = evaluate_json(&encoded)
        .map_err(|error| format!("evaluation rejected: {error}"))?;
    let report_value: serde_json::Value = serde_json::from_str(&rendered_report)?;
    if let Some(parent) = report_path.parent().filter(|path| !path.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(report_path, rendered_report)?;
    println!(
        "selective-driving evaluation: {} decisions, {:.3} actual coverage, AURC={:.6}; report={}",
        report_value["observed_decision_count"].as_u64().unwrap_or(0),
        report_value["actual_autonomy_coverage"].as_f64().unwrap_or(0.0),
        report_value["risk_coverage_area_failure_rate"].as_f64().unwrap_or(0.0),
        report_path.display()
    );
    Ok(())
}

fn evaluate_json(input: &str) -> Result<String, String> {
    let evaluation = EvaluationInput::from_json(input)?;
    let report = evaluation
        .evaluate()
        .map_err(|error| format!("evaluation rejected: {error}"))?;
    serde_json::to_string_pretty(&report)
        .map_err(|error| format!("cannot serialize evaluation report: {error}"))
}

fn main() {
    if let Err(error) = run() {
        eprintln!("selective-driving-competence: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn valid_input() -> String {
        json!({
            "schema_version": 1,
            "protocol_id": "cli-test-v1",
            "expected_episode_ids": ["episode-1"],
            "expected_decision_ids": ["decision-1"],
            "calibration_bin_count": 5,
            "fixed_coverage_target": 0.5,
            "fixed_risk_limit": 0.1,
            "bootstrap_replicates": 200,
            "bootstrap_seed": 7,
            "decisions": [{
                "decision_id": "decision-1",
                "episode_id": "episode-1",
                "scenario_family": "urban-daylight",
                "prediction_timestamp_us": 1000,
                "outcome_timestamp_us": 2000,
                "outcome_status": "observed",
                "predicted_failure_probability": 0.02,
                "observed_failure": false,
                "observed_loss": 0.0,
                "policy_input_fields": [{
                    "field_id": "front-camera",
                    "source_role": "policy_visible",
                    "available_timestamp_us": 900
                }],
                "abstained": false,
                "abstention_timestamp_us": null,
                "decision_deadline_us": null,
                "fallback_succeeded": null,
                "fallback_duration_ms": null,
                "human_interventions": 0,
                "inference_latency_ms": 12.0,
                "inference_energy_mj": 4.0,
                "peak_memory_bytes": 1024
            }]
        })
        .to_string()
    }

    #[test]
    fn cli_emits_machine_readable_risk_coverage_report() {
        let report = evaluate_json(&valid_input()).unwrap();
        let parsed: serde_json::Value = serde_json::from_str(&report).unwrap();
        assert_eq!(parsed["observed_decision_count"], 1);
        assert_eq!(parsed["protocol_id"], "cli-test-v1");
        assert!(parsed["risk_coverage_area_failure_rate"].is_number());
        assert_eq!(parsed["actual_autonomy_coverage"], 1.0);
    }

    #[test]
    fn cli_rejects_unknown_or_missing_outcome_fields() {
        let mut value: serde_json::Value = serde_json::from_str(&valid_input()).unwrap();
        value["unexpected_privilege"] = json!("ground-truth");
        assert!(evaluate_json(&value.to_string()).is_err());

        let mut missing: serde_json::Value = serde_json::from_str(&valid_input()).unwrap();
        missing["decisions"][0]["outcome_status"] = json!("missing");
        assert!(evaluate_json(&missing.to_string()).is_err());
    }
}
