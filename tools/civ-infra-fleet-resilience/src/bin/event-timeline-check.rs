use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{env, fs, process};

#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: String,
    fixture_id: String,
    classification: String,
    claim_ceiling: String,
    disturbance_cases: Vec<FixtureDisturbance>,
}

#[derive(Clone, Debug, Deserialize)]
struct FixtureDisturbance {
    case_id: String,
    name: String,
    injection_tick: u64,
    observable_tick: u64,
    event: String,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidates: Vec<String>,
    recovery_evidence: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct TimelineReport {
    schema_version: String,
    fixture_id: String,
    input_sha256: String,
    classification: String,
    timeline_status: String,
    timeline_audit_executed: bool,
    policy_simulation_executed: bool,
    policy_runs_performed: usize,
    counts: Counts,
    event_cases: Vec<EventCaseReport>,
    evaluator_timeline: Vec<EvaluatorTick>,
    policy_timeline: Vec<PolicyTick>,
    claim_ceiling: String,
    explicit_limitations: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct Counts {
    disturbance_cases: usize,
    typed_disturbance_kinds: usize,
    delayed_observations: usize,
    total_observation_delay_ticks: u64,
    maximum_observation_delay_ticks: u64,
    timeline_tick_count: usize,
    referenced_resources: usize,
    referenced_tasks: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EventCaseReport {
    case_id: String,
    disturbance_kind: String,
    event_name: String,
    injection_tick: u64,
    observable_tick: u64,
    observation_delay_ticks: u64,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidated_evidence_or_state: Vec<String>,
    recovery_evidence: Vec<String>,
    timeline_status: String,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct EvaluatorTick {
    tick: u32,
    injected_event_ids: Vec<String>,
    events_injected_but_not_yet_observable: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PolicyTick {
    tick: u32,
    newly_observed_event_ids: Vec<String>,
    newly_observed_events: Vec<PolicyObservation>,
    visible_event_ids: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, Eq)]
#[serde(deny_unknown_fields)]
struct PolicyObservation {
    case_id: String,
    disturbance_kind: String,
    event_name: String,
    observed_at_tick: u32,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidates: Vec<String>,
    recovery_evidence: Vec<String>,
}

fn main() {
    let mut args = env::args_os().skip(1);
    let fixture_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: event-timeline-check <fixture.json> <report.json>");
            process::exit(2);
        }
    };
    let report_path = match args.next() {
        Some(path) => path,
        None => {
            eprintln!("usage: event-timeline-check <fixture.json> <report.json>");
            process::exit(2);
        }
    };
    if args.next().is_some() {
        eprintln!("usage: event-timeline-check <fixture.json> <report.json>");
        process::exit(2);
    }

    let fixture_bytes = match fs::read(&fixture_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read fixture {}: {error}", fixture_path.to_string_lossy());
            process::exit(2);
        }
    };
    let report_bytes = match fs::read(&report_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read report {}: {error}", report_path.to_string_lossy());
            process::exit(2);
        }
    };

    match verify_report_bytes(&fixture_bytes, &report_bytes) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("cannot serialize checker result: {error}");
                process::exit(2);
            }
        },
        Err(errors) => {
            for error in errors {
                eprintln!("independent event-timeline check failed: {error}");
            }
            process::exit(2);
        }
    }
}

#[derive(Clone, Debug, Serialize)]
struct CheckResult {
    status: String,
    fixture_id: String,
    input_sha256: String,
    event_count: usize,
    timeline_tick_count: usize,
    policy_simulation_executed: bool,
}

fn verify_report_bytes(
    fixture_bytes: &[u8],
    report_bytes: &[u8],
) -> Result<CheckResult, Vec<String>> {
    let fixture: Fixture = serde_json::from_slice(fixture_bytes).map_err(|error| {
        vec![format!("fixture could not be parsed: {error}")]
    })?;
    let report: TimelineReport = serde_json::from_slice(report_bytes).map_err(|error| {
        vec![format!("report does not match strict report schema: {error}")]
    })?;

    let expected = expected_report(&fixture, fixture_bytes);
    let mut errors = Vec::new();

    if report.schema_version != expected.schema_version {
        errors.push("report schema_version mismatch".to_string());
    }
    if report.fixture_id != expected.fixture_id {
        errors.push("report fixture_id mismatch".to_string());
    }
    if report.input_sha256 != expected.input_sha256 {
        errors.push("report input_sha256 does not bind the supplied fixture bytes".to_string());
    }
    if report.classification != "synthetic_only" || report.classification != expected.classification {
        errors.push("report classification must be synthetic_only and match the fixture".to_string());
    }
    if report.timeline_status != "canonical_event_queue_timing_audited" {
        errors.push("unexpected timeline_status".to_string());
    }
    if !report.timeline_audit_executed {
        errors.push("report must declare that the timeline audit executed".to_string());
    }
    if report.policy_simulation_executed || report.policy_runs_performed != 0 {
        errors.push("timeline-only report must not claim policy simulation or policy runs".to_string());
    }
    if report.claim_ceiling != expected.claim_ceiling {
        errors.push("report claim_ceiling differs from the fixture".to_string());
    }
    if report.counts != expected.counts {
        errors.push("report counts differ from independently recomputed fixture counts".to_string());
    }
    if report.event_cases != expected.event_cases {
        errors.push("event case records differ from independently normalized fixture cases".to_string());
    }
    if report.evaluator_timeline != expected.evaluator_timeline {
        errors.push("evaluator injection/hidden-event timeline differs from independent recomputation".to_string());
    }
    if report.policy_timeline != expected.policy_timeline {
        errors.push("policy-visible observation timeline differs from independent recomputation".to_string());
    }
    if report.explicit_limitations.is_empty() {
        errors.push("report must state explicit limitations".to_string());
    }

    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    Ok(CheckResult {
        status: "independent_event_timeline_report_verified".to_string(),
        fixture_id: report.fixture_id,
        input_sha256: report.input_sha256,
        event_count: report.counts.disturbance_cases,
        timeline_tick_count: report.counts.timeline_tick_count,
        policy_simulation_executed: false,
    })
}

fn expected_report(fixture: &Fixture, fixture_bytes: &[u8]) -> TimelineReport {
    let mut disturbances = fixture.disturbance_cases.clone();
    disturbances.sort_by(|left, right| {
        (left.injection_tick, &left.case_id).cmp(&(right.injection_tick, &right.case_id))
    });

    let event_cases: Vec<EventCaseReport> = disturbances
        .iter()
        .map(|case| {
            let delay = case.observable_tick - case.injection_tick;
            EventCaseReport {
                case_id: case.case_id.clone(),
                disturbance_kind: disturbance_kind(&case.case_id).to_string(),
                event_name: case.event.clone(),
                injection_tick: case.injection_tick,
                observable_tick: case.observable_tick,
                observation_delay_ticks: delay,
                affected_resources: sorted_unique(case.affected_resources.clone()),
                affected_tasks: sorted_unique(case.affected_tasks.clone()),
                invalidated_evidence_or_state: sorted_unique(case.invalidates.clone()),
                recovery_evidence: sorted_unique(case.recovery_evidence.clone()),
                timeline_status: if delay == 0 {
                    "visible_on_injection_tick".to_string()
                } else {
                    "hidden_until_observation_tick".to_string()
                },
            }
        })
        .collect();

    let max_tick = disturbances
        .iter()
        .map(|case| case.observable_tick)
        .max()
        .unwrap_or(0);
    let mut evaluator_timeline = Vec::with_capacity(max_tick as usize + 1);
    let mut policy_timeline = Vec::with_capacity(max_tick as usize + 1);

    for tick in 0..=max_tick {
        let injected_event_ids = sorted_unique(
            disturbances
                .iter()
                .filter(|case| case.injection_tick == u64::from(tick))
                .map(|case| case.case_id.clone())
                .collect(),
        );
        let events_injected_but_not_yet_observable = sorted_unique(
            disturbances
                .iter()
                .filter(|case| {
                    case.injection_tick <= u64::from(tick)
                        && case.observable_tick > u64::from(tick)
                })
                .map(|case| case.case_id.clone())
                .collect(),
        );
        let newly_observed: Vec<PolicyObservation> = disturbances
            .iter()
            .filter(|case| case.observable_tick == u64::from(tick))
            .map(|case| PolicyObservation {
                case_id: case.case_id.clone(),
                disturbance_kind: disturbance_kind(&case.case_id).to_string(),
                event_name: case.event.clone(),
                observed_at_tick: tick,
                affected_resources: sorted_unique(case.affected_resources.clone()),
                affected_tasks: sorted_unique(case.affected_tasks.clone()),
                invalidates: sorted_unique(case.invalidates.clone()),
                recovery_evidence: sorted_unique(case.recovery_evidence.clone()),
            })
            .collect();
        let newly_observed_event_ids = newly_observed
            .iter()
            .map(|observation| observation.case_id.clone())
            .collect::<Vec<_>>();
        let visible_event_ids = sorted_unique(
            disturbances
                .iter()
                .filter(|case| case.observable_tick <= u64::from(tick))
                .map(|case| case.case_id.clone())
                .collect(),
        );

        evaluator_timeline.push(EvaluatorTick {
            tick,
            injected_event_ids,
            events_injected_but_not_yet_observable,
        });
        policy_timeline.push(PolicyTick {
            tick,
            newly_observed_event_ids,
            newly_observed_events: newly_observed,
            visible_event_ids,
        });
    }

    let delays: Vec<u64> = disturbances
        .iter()
        .map(|case| case.observable_tick - case.injection_tick)
        .collect();
    let delayed_observations = delays.iter().filter(|delay| **delay > 0).count();
    let total_observation_delay_ticks = delays.iter().sum();
    let maximum_observation_delay_ticks = delays.iter().copied().max().unwrap_or(0);
    let input_sha256 = format!("{:x}", Sha256::digest(fixture_bytes));

    TimelineReport {
        schema_version: "luminous.civ_infra.event_timeline_audit.v0.1".to_string(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256,
        classification: fixture.classification.clone(),
        timeline_status: "canonical_event_queue_timing_audited".to_string(),
        timeline_audit_executed: true,
        policy_simulation_executed: false,
        policy_runs_performed: 0,
        counts: Counts {
            disturbance_cases: disturbances.len(),
            typed_disturbance_kinds: disturbances
                .iter()
                .map(|case| disturbance_kind(&case.case_id))
                .collect::<std::collections::BTreeSet<_>>()
                .len(),
            delayed_observations,
            total_observation_delay_ticks,
            maximum_observation_delay_ticks,
            timeline_tick_count: evaluator_timeline.len(),
            referenced_resources: 0,
            referenced_tasks: 0,
        },
        event_cases,
        evaluator_timeline,
        policy_timeline,
        claim_ceiling: fixture.claim_ceiling.clone(),
        explicit_limitations: vec![
            "independent checker verifies deterministic timeline output only".to_string(),
        ],
    }
}

fn disturbance_kind(case_id: &str) -> &'static str {
    match case_id {
        "R-001" => "localization_lost",
        "R-002" => "calibration_stale",
        "R-003" => "communication_outage",
        "R-004" => "robot_failure",
        "R-005" => "late_material",
        "R-006" => "material_mismatch",
        "R-007" => "inspection_rejected",
        "R-008" => "geometry_changed",
        "R-009" => "resource_conflict",
        "R-010" => "energy_reserve_infeasible",
        "R-011" => "unknown_utility",
        "R-012" => "approval_declined",
        "R-013" => "capability_profile_stale",
        _ => "unknown",
    }
}

fn sorted_unique(mut values: Vec<String>) -> Vec<String> {
    values.sort();
    values.dedup();
    values
}

#[cfg(test)]
mod tests {
    use super::*;

    const EMBEDDED_FIXTURE: &str = include_str!(
        "../../../../docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json"
    );

    fn report_bytes(fixture_bytes: &[u8]) -> Vec<u8> {
        let fixture: Fixture =
            serde_json::from_slice(fixture_bytes).expect("embedded fixture should parse");
        serde_json::to_vec(&expected_report(&fixture, fixture_bytes))
            .expect("reference report should serialize")
    }

    #[test]
    fn accepts_independently_recomputed_reference_report() {
        let report = report_bytes(EMBEDDED_FIXTURE.as_bytes());
        let checked = verify_report_bytes(EMBEDDED_FIXTURE.as_bytes(), &report)
            .expect("reference report should verify");
        assert_eq!(checked.event_count, 13);
        assert_eq!(checked.timeline_tick_count, 8);
        assert!(!checked.policy_simulation_executed);
    }

    #[test]
    fn rejects_early_visibility_of_geometry_revision() {
        let mut report: serde_json::Value =
            serde_json::from_slice(&report_bytes(EMBEDDED_FIXTURE.as_bytes()))
                .expect("reference report should parse");
        report["policy_timeline"][3]["visible_event_ids"]
            .as_array_mut()
            .expect("visible IDs should be an array")
            .push(serde_json::json!("R-008"));
        let bytes = serde_json::to_vec(&report).expect("mutated report should serialize");
        let errors = verify_report_bytes(EMBEDDED_FIXTURE.as_bytes(), &bytes)
            .expect_err("early visibility must fail");
        assert!(errors.iter().any(|error| error.contains("policy-visible observation timeline")));
    }

    #[test]
    fn rejects_injection_timestamp_inside_policy_observation() {
        let mut report: serde_json::Value =
            serde_json::from_slice(&report_bytes(EMBEDDED_FIXTURE.as_bytes()))
                .expect("reference report should parse");
        report["policy_timeline"][4]["newly_observed_events"][0]["injection_tick"] =
            serde_json::json!(2);
        let bytes = serde_json::to_vec(&report).expect("mutated report should serialize");
        assert!(verify_report_bytes(EMBEDDED_FIXTURE.as_bytes(), &bytes).is_err());
    }

    #[test]
    fn rejects_report_bound_to_different_fixture_bytes() {
        let report = report_bytes(EMBEDDED_FIXTURE.as_bytes());
        let mut altered = EMBEDDED_FIXTURE.as_bytes().to_vec();
        altered.push(b' ');
        let errors =
            verify_report_bytes(&altered, &report).expect_err("digest mismatch must fail");
        assert!(errors.iter().any(|error| error.contains("input_sha256")));
    }

    #[test]
    fn rejects_claim_that_timeline_audit_simulated_policies() {
        let mut report: serde_json::Value =
            serde_json::from_slice(&report_bytes(EMBEDDED_FIXTURE.as_bytes()))
                .expect("reference report should parse");
        report["policy_simulation_executed"] = serde_json::json!(true);
        report["policy_runs_performed"] = serde_json::json!(13);
        let bytes = serde_json::to_vec(&report).expect("mutated report should serialize");
        let errors =
            verify_report_bytes(EMBEDDED_FIXTURE.as_bytes(), &bytes).expect_err("claim must fail");
        assert!(errors.iter().any(|error| error.contains("must not claim policy simulation")));
    }

    #[test]
    fn rejects_unknown_report_fields() {
        let mut report: serde_json::Value =
            serde_json::from_slice(&report_bytes(EMBEDDED_FIXTURE.as_bytes()))
                .expect("reference report should parse");
        report["hidden_dispatch_authority"] = serde_json::json!(true);
        let bytes = serde_json::to_vec(&report).expect("mutated report should serialize");
        assert!(verify_report_bytes(EMBEDDED_FIXTURE.as_bytes(), &bytes).is_err());
    }
}
