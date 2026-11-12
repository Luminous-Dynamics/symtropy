//! JSON command-line adapter for decision-time action assurance.
//!
//! Usage:
//!   decision-time-assurance <request.json> <report.json>
//!
//! Exit codes:
//!   0 - nominal or explicitly restricted authorization issued
//!   1 - fallback required; report still written
//!   2 - malformed input, I/O, or invocation error
//!
//! This process does not execute the proposed action.

use decision_time_action_assurance::{
    evaluate, evidence_bundle_sha256, ActionProposal, EvidenceBundle, GateDecision, GatePolicy,
    GateEvaluation,
};
use serde::{Deserialize, Serialize};
use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

const MAX_REQUEST_BYTES: u64 = 16 * 1024 * 1024;

#[derive(Debug, Clone, Deserialize)]
#[serde(deny_unknown_fields)]
struct GateRequest {
    schema_version: u32,
    decision_time_us: u64,
    bundle: EvidenceBundle,
    proposal: ActionProposal,
    policy: GatePolicy,
}

#[derive(Debug, Clone, Serialize)]
struct GateReport {
    schema_version: u32,
    status: String,
    decision_time_us: u64,
    evidence_bundle_sha256: String,
    evaluation: GateEvaluation,
    claim_ceiling: String,
}

fn evaluate_request(input: &str) -> Result<GateReport, String> {
    let request: GateRequest = serde_json::from_str(input)
        .map_err(|error| format!("request JSON does not match the strict schema: {error}"))?;
    if request.schema_version != 1 {
        return Err(format!(
            "unsupported request schema version {}",
            request.schema_version
        ));
    }
    let bundle_digest = evidence_bundle_sha256(&request.bundle)?;
    let evaluation = evaluate(
        &request.bundle,
        &request.proposal,
        &request.policy,
        request.decision_time_us,
    )?;
    let status = match evaluation.decision {
        GateDecision::Authorized => "authorized",
        GateDecision::AuthorizedRestricted => "authorized_restricted",
        GateDecision::FallbackRequired => "fallback_required",
    };
    Ok(GateReport {
        schema_version: 1,
        status: status.to_string(),
        decision_time_us: request.decision_time_us,
        evidence_bundle_sha256: bundle_digest,
        evaluation,
        claim_ceiling: "Finite typed-record and time-window consistency only. Digests are unkeyed integrity commitments, not publisher authentication. A valid envelope does not prove semantic correctness or that an action is safe. No actuator is invoked.".to_string(),
    })
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: decision-time-assurance <request.json> <report.json>".into());
    }
    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_REQUEST_BYTES {
        return Err("request must be a regular file no larger than 16 MiB".into());
    }
    let input = fs::read_to_string(input_path)?;
    let report = evaluate_request(&input)
        .map_err(|error| format!("decision-time assurance input rejected: {error}"))?;
    if let Some(parent) = output_path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent)?;
    }
    fs::write(output_path, serde_json::to_vec_pretty(&report)?)?;
    println!(
        "decision-time assurance: {} (profile result: {:?}); report: {}",
        report.status,
        report.evaluation.decision,
        output_path.display()
    );
    Ok(if report.evaluation.decision == GateDecision::FallbackRequired {
        1
    } else {
        0
    })
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("decision-time-assurance: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use decision_time_action_assurance::{
        ActionClass, EvidenceHealth, EvidenceProfile, FallbackDirective, Modality,
        ModalityRequirement, ObservationDisposition, ObservationSample,
    };

    fn request_json() -> String {
        let mut samples = Vec::new();
        for (modality, sensor_id) in [
            (Modality::CameraFront, "front"),
            (Modality::VehicleState, "ego-state"),
        ] {
            for sequence in 1..=3 {
                let capture = 950_000 + (sequence - 1) * 25_000;
                samples.push(ObservationSample {
                    modality,
                    sensor_id: sensor_id.to_string(),
                    sequence,
                    capture_timestamp_us: capture,
                    ready_timestamp_us: capture + 500,
                    delivered_timestamp_us: capture + 1_000,
                    clock_id: "mono-1".to_string(),
                    payload_sha256: format!("{:064x}", sequence + 1),
                    calibration_sha256: Some("c".repeat(64)),
                    health: EvidenceHealth::Healthy,
                    disposition: ObservationDisposition::Fresh,
                });
            }
        }
        let bundle = EvidenceBundle {
            run_id: "run-1".to_string(),
            episode_id: "episode-1".to_string(),
            agent_id: "ego".to_string(),
            clock_id: "mono-1".to_string(),
            samples,
            missing_events: Vec::new(),
        };
        let proposal = ActionProposal {
            run_id: bundle.run_id.clone(),
            episode_id: bundle.episode_id.clone(),
            agent_id: bundle.agent_id.clone(),
            clock_id: bundle.clock_id.clone(),
            proposal_id: "proposal-1".to_string(),
            action_class: ActionClass::Routine,
            action_sha256: "a".repeat(64),
            source_evidence_bundle_sha256: evidence_bundle_sha256(&bundle).unwrap(),
            issued_at_us: 1_010_000,
            expires_at_us: 1_090_000,
        };
        let make_requirement = |modality, sensor_id: &str| ModalityRequirement {
            modality,
            sensor_id: sensor_id.to_string(),
            minimum_samples: 2,
            max_capture_age_us: 100_000,
            max_processing_delay_us: 5_000,
            max_delivery_delay_us: 8_000,
            max_inter_sample_gap_us: 60_000,
            require_healthy: true,
            allow_delayed: false,
        };
        let policy = GatePolicy {
            policy_id: "cli-policy-v1".to_string(),
            nominal: EvidenceProfile {
                profile_id: "nominal-v1".to_string(),
                allowed_action_classes: vec![ActionClass::Routine],
                requirements: vec![
                    make_requirement(Modality::CameraFront, "front"),
                    make_requirement(Modality::VehicleState, "ego-state"),
                ],
                max_inter_sensor_skew_us: 20_000,
                max_proposal_age_us: 30_000,
                max_authorization_lifetime_us: 60_000,
            },
            restricted: None,
            fallback: FallbackDirective::RequestIndependentFallback,
        };
        serde_json::json!({
            "schema_version": 1,
            "decision_time_us": 1_020_000,
            "bundle": bundle,
            "proposal": proposal,
            "policy": policy
        })
        .to_string()
    }

    #[test]
    fn request_report_schemas_and_benchmark_manifest_are_valid_json() {
        let request_schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/gate-request-v1.schema.json")).unwrap();
        let report_schema: serde_json::Value =
            serde_json::from_str(include_str!("../schemas/gate-report-v1.schema.json")).unwrap();
        let manifest: serde_json::Value =
            serde_json::from_str(include_str!("../benchmark_manifest_v0_1.json")).unwrap();
        assert_eq!(request_schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
        assert_eq!(report_schema["$schema"], "https://json-schema.org/draft/2020-12/schema");
        assert_eq!(manifest["status"], "specification_and_reference_implementation_only_no_benchmark_runs");
        assert_eq!(manifest["prospective_targets"]["status"], "not_measured");
    }

    #[test]
    fn cli_emits_authorization_report_for_valid_request() {
        let report = evaluate_request(&request_json()).unwrap();
        assert_eq!(report.status, "authorized");
        assert_eq!(
            report.evaluation.decision,
            GateDecision::Authorized
        );
        assert!(report.evaluation.envelope.is_some());
    }

    #[test]
    fn cli_rejects_unknown_top_level_fields() {
        let mut input: serde_json::Value =
            serde_json::from_str(&request_json()).unwrap();
        input["unexpected_execution_command"] = serde_json::json!("steer-now");
        assert!(evaluate_request(&input.to_string()).is_err());
    }

    #[test]
    fn cli_reports_fallback_instead_of_authorization_for_missing_camera() {
        let mut input: serde_json::Value =
            serde_json::from_str(&request_json()).unwrap();
        input["bundle"]["samples"] = serde_json::Value::Array(
            input["bundle"]["samples"]
                .as_array()
                .unwrap()
                .iter()
                .filter(|sample| sample["modality"] != "camera_front")
                .cloned()
                .collect(),
        );
        let bundle: EvidenceBundle =
            serde_json::from_value(input["bundle"].clone()).unwrap();
        input["proposal"]["source_evidence_bundle_sha256"] =
            serde_json::json!(evidence_bundle_sha256(&bundle).unwrap());
        let report = evaluate_request(&input.to_string()).unwrap();
        assert_eq!(report.status, "fallback_required");
        assert!(report.evaluation.envelope.is_none());
    }
}
