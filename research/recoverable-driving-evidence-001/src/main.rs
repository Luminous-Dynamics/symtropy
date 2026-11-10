//! JSON adapter for the finite-record recoverable-driving campaign auditor.
//!
//! Usage:
//!   recoverable-driving-evidence <campaign.json> <report.json>
//!
//! Exit codes: 0 = structurally valid, 1 = structurally rejected, 2 = input or
//! invocation error. A JSON report is written for both valid and rejected audits.

use std::env;
use std::error::Error;
use std::fs;
use std::path::Path;

use recoverable_driving_evidence::{validate_campaign, CampaignRecord, ReportDisposition};
use serde_json::{json, Value};

const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

fn audit_value(input: &str) -> Result<(Value, bool), String> {
    let campaign: CampaignRecord = serde_json::from_str(input)
        .map_err(|error| format!("campaign JSON does not match the typed audit schema: {error}"))?;
    let report = validate_campaign(&campaign);
    let valid = report.disposition == ReportDisposition::Valid;
    let rendered = json!({
        "schema_version": 1,
        "disposition": if valid { "valid" } else { "rejected" },
        "summary": {
            "expected_episodes": report.summary.expected_episodes,
            "observed_episodes": report.summary.observed_episodes,
            "completed_episodes": report.summary.completed_episodes,
            "failed_episodes": report.summary.failed_episodes,
            "incomplete_episodes": report.summary.incomplete_episodes,
            "censored_episodes": report.summary.censored_episodes,
            "expected_branches": report.summary.expected_branches,
            "observed_branches": report.summary.observed_branches,
            "actor_response_records": report.summary.actor_response_records,
            "random_event_records": report.summary.random_event_records
        },
        "finding_count": report.findings.len(),
        "findings": report.findings.iter().map(|finding| format!("{finding:?}")).collect::<Vec<_>>(),
        "claim_ceiling": "Finite typed-record consistency only. This report does not authenticate sources, prove exogeneity or physical causality, run a simulator, evaluate a policy, or establish vehicle safety."
    });
    Ok((rendered, valid))
}

fn write_report(path: &Path, report: &Value) -> Result<(), Box<dyn Error>> {
    if let Some(parent) = path.parent().filter(|parent| !parent.as_os_str().is_empty()) {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, serde_json::to_vec_pretty(report)?)?;
    Ok(())
}

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err("usage: recoverable-driving-evidence <campaign.json> <report.json>".into());
    }
    let input_path = Path::new(&args[0]);
    let output_path = Path::new(&args[1]);
    let metadata = fs::metadata(input_path)?;
    if !metadata.is_file() || metadata.len() > MAX_INPUT_BYTES {
        return Err("campaign input must be a regular file no larger than 64 MiB".into());
    }
    let input = fs::read_to_string(input_path)?;
    let (report, valid) = audit_value(&input).map_err(|error| format!("audit input rejected: {error}"))?;
    write_report(output_path, &report)?;
    println!(
        "campaign audit: {} ({} findings); report: {}",
        report["disposition"].as_str().unwrap_or("unknown"),
        report["finding_count"].as_u64().unwrap_or(0),
        output_path.display()
    );
    Ok(if valid { 0 } else { 1 })
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("recoverable-driving-evidence: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn actor(actor_id: &str, origin: &str, input_digest: &str, action_digest: &str) -> Value {
        json!({
            "actor_id": actor_id,
            "policy_revision_digest": format!("policy-{actor_id}-v1"),
            "policy_input_digest": input_digest,
            "action_digest": action_digest,
            "randomness_digest": format!("rng-{actor_id}-seed-1"),
            "origin": origin
        })
    }

    fn branch(id: &str, parent: Option<&str>, input_shift: bool, replay: bool) -> Value {
        let base_input = "lead-observation-at-tick-11";
        let actor_input = if input_shift {
            "lead-observation-after-alternate-action"
        } else {
            base_input
        };
        let origin = if replay { "replayed" } else { "recomputed" };
        let random = json!({
            "event_id": "ambient-weather",
            "shared_with_parent": parent.is_some(),
            "event_class": "exogenous",
            "invariant_under_intervention": true,
            "key_digest": "weather-key-11",
            "sampled_value_digest": "weather-value-11"
        });
        json!({
            "branch_id": id,
            "episode_id": "episode-fixture-1",
            "parent_branch_id": parent,
            "checkpoint_digest": "checkpoint-11",
            "recurrent_state_policy_id": "rnn-state-policy-v1",
            "observation_history_digest": "history-through-tick-11",
            "simulator_revision": "snapshot-backend-v1",
            "decision_tick": 11,
            "outcome_tick": if parent.is_some() { 12 } else { 11 },
            "intervention_digest": if parent.is_some() { "alternate-action-digest" } else { "base-action-digest" },
            "expected_actor_ids": ["lead-car", "pedestrian"],
            "actor_responses": [
                actor("lead-car", origin, actor_input, if parent.is_some() { "lead-response-after-alternate-action" } else { "lead-response-base" }),
                actor("pedestrian", "recomputed", "pedestrian-observation-at-tick-11", "pedestrian-response-recomputed")
            ],
            "expected_random_event_ids": ["ambient-weather"],
            "random_events": [random],
            "policy_input_fields": [
                {"field_id":"front-camera-frame", "source_role":"policy_visible", "available_tick":11},
                {"field_id":"ego-speed-sample", "source_role":"policy_visible", "available_tick":11}
            ],
            "query": null
        })
    }

    fn campaign(replay_stale_actor: bool) -> Value {
        json!({
            "expected_episode_ids": ["episode-fixture-1"],
            "expected_branch_ids": ["base", "alternate-action"],
            "episodes": [
                {"episode_id":"episode-fixture-1","disposition":"completed"}
            ],
            "branches": [
                branch("base", None, false, false),
                branch("alternate-action", Some("base"), true, replay_stale_actor)
            ],
            "evaluation_controls": {
                "frozen": true,
                "teacher_blending_enabled": false,
                "online_updates_enabled": false,
                "exploration_noise_enabled": false,
                "teacher_inputs_visible": false,
                "future_outcomes_visible": false
            }
        })
    }

    #[test]
    fn valid_recomputed_actor_branch_is_accepted() {
        let (report, valid) = audit_value(&campaign(false).to_string()).unwrap();
        assert!(valid);
        assert_eq!(report["disposition"], "valid");
        assert_eq!(report["summary"]["expected_branches"], 2);
        assert_eq!(report["summary"]["observed_episodes"], 1);
        assert_eq!(report["finding_count"], 0);
    }

    #[test]
    fn stale_actor_replay_negative_control_is_rejected() {
        let (report, valid) = audit_value(&campaign(true).to_string()).unwrap();
        assert!(!valid);
        assert_eq!(report["disposition"], "rejected");
        let findings = report["findings"].as_array().unwrap();
        assert!(findings.iter().any(|finding| {
            finding.as_str() == Some("ReplayedActorInputChanged")
        }));
    }

    #[test]
    fn malformed_or_unknown_campaign_fields_are_rejected() {
        let mut value = campaign(false);
        value["unexpected_privilege"] = json!("simulator-truth");
        assert!(audit_value(&value.to_string()).is_err());
    }
}
