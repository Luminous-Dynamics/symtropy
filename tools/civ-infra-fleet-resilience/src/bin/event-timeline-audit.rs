use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::collections::BTreeSet;
use std::{env, fs, process};

const EMBEDDED_FIXTURE: &str = include_str!(
    "../../../../docs/research/civ-infra-resilience/fixtures/fleet-resilience-v0.1.json"
);

// This binary validates the event-timeline projection. The main manifest
// verifier remains responsible for strict validation of every root-level field.
#[derive(Clone, Debug, Deserialize)]
struct Fixture {
    schema_version: String,
    fixture_id: String,
    classification: String,
    claim_ceiling: String,
    clock: ClockSpec,
    disturbance_cases: Vec<DisturbanceCase>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct ClockSpec {
    unit: String,
    interval_semantics: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, PartialOrd, Ord)]
enum DisturbanceKind {
    #[serde(rename = "localization_quality_out_of_profile")]
    LocalizationQualityOutOfProfile,
    #[serde(rename = "tool_identity_changed")]
    ToolIdentityChanged,
    #[serde(rename = "communication_link_unavailable")]
    CommunicationLinkUnavailable,
    #[serde(rename = "excavator_drive_fault")]
    ExcavatorDriveFault,
    #[serde(rename = "water-pipe-delivery_delayed")]
    WaterPipeDeliveryDelayed,
    #[serde(rename = "material_class_mismatch")]
    MaterialClassMismatch,
    #[serde(rename = "as_built_inspection_failed")]
    AsBuiltInspectionFailed,
    #[serde(rename = "workface_geometry_revision_changed")]
    WorkfaceGeometryRevisionChanged,
    #[serde(rename = "two_tasks_require_same_capacity_one_tool")]
    CapacityOneToolConflict,
    #[serde(rename = "remaining_energy_below_task_plus_return_reserve")]
    EnergyReturnReserveInfeasible,
    #[serde(rename = "utility_envelope_unbounded_unknown")]
    UnboundedUnknownUtility,
    #[serde(rename = "required_operator_approval_declined")]
    OperatorApprovalDeclined,
    #[serde(rename = "robot_capability_revision_superseded")]
    CapabilityRevisionSuperseded,
}

impl DisturbanceKind {
    const ALL: [Self; 13] = [
        Self::LocalizationQualityOutOfProfile,
        Self::ToolIdentityChanged,
        Self::CommunicationLinkUnavailable,
        Self::ExcavatorDriveFault,
        Self::WaterPipeDeliveryDelayed,
        Self::MaterialClassMismatch,
        Self::AsBuiltInspectionFailed,
        Self::WorkfaceGeometryRevisionChanged,
        Self::CapacityOneToolConflict,
        Self::EnergyReturnReserveInfeasible,
        Self::UnboundedUnknownUtility,
        Self::OperatorApprovalDeclined,
        Self::CapabilityRevisionSuperseded,
    ];

    fn as_str(self) -> &'static str {
        match self {
            Self::LocalizationQualityOutOfProfile => "localization_quality_out_of_profile",
            Self::ToolIdentityChanged => "tool_identity_changed",
            Self::CommunicationLinkUnavailable => "communication_link_unavailable",
            Self::ExcavatorDriveFault => "excavator_drive_fault",
            Self::WaterPipeDeliveryDelayed => "water-pipe-delivery_delayed",
            Self::MaterialClassMismatch => "material_class_mismatch",
            Self::AsBuiltInspectionFailed => "as_built_inspection_failed",
            Self::WorkfaceGeometryRevisionChanged => "workface_geometry_revision_changed",
            Self::CapacityOneToolConflict => "two_tasks_require_same_capacity_one_tool",
            Self::EnergyReturnReserveInfeasible => {
                "remaining_energy_below_task_plus_return_reserve"
            }
            Self::UnboundedUnknownUtility => "utility_envelope_unbounded_unknown",
            Self::OperatorApprovalDeclined => "required_operator_approval_declined",
            Self::CapabilityRevisionSuperseded => "robot_capability_revision_superseded",
        }
    }
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct DisturbanceCase {
    case_id: String,
    name: String,
    injection_tick: u64,
    observable_tick: u64,
    event: DisturbanceKind,
    affected_resources: Vec<String>,
    affected_tasks: Vec<String>,
    invalidates: Vec<String>,
    #[serde(default)]
    recoverability: Option<String>,
    recovery_evidence: Vec<String>,
    required_invariants: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct Counts {
    disturbance_cases: usize,
    typed_disturbance_kinds: usize,
    delayed_observations: usize,
    total_observation_delay_ticks: u64,
    maximum_observation_delay_ticks: u64,
    timeline_tick_count: u64,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct EvaluatorTick {
    tick: u64,
    injected_event_ids: Vec<String>,
    events_injected_but_not_yet_observable: Vec<String>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct NewlyObservedEvent {
    case_id: String,
    observed_at_tick: u64,
    event: String,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
struct PolicyTick {
    tick: u64,
    visible_event_ids: Vec<String>,
    newly_observed_events: Vec<NewlyObservedEvent>,
}

#[derive(Clone, Debug, Serialize, PartialEq, Eq)]
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
    evaluator_timeline: Vec<EvaluatorTick>,
    policy_timeline: Vec<PolicyTick>,
    claim_ceiling: String,
    explicit_limitations: Vec<String>,
}

fn main() {
    let mut args = env::args_os().skip(1);
    let input_bytes = match args.next() {
        Some(path) => match fs::read(&path) {
            Ok(bytes) => bytes,
            Err(error) => {
                eprintln!("cannot read fixture {}: {error}", path.to_string_lossy());
                process::exit(2);
            }
        },
        None => EMBEDDED_FIXTURE.as_bytes().to_vec(),
    };

    if args.next().is_some() {
        eprintln!("usage: event-timeline-audit [fixture.json]");
        process::exit(2);
    }

    match verify_fixture_bytes(&input_bytes) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("cannot serialize event timeline report: {error}");
                process::exit(2);
            }
        },
        Err(errors) => {
            for error in errors {
                eprintln!("event timeline rejected: {error}");
            }
            process::exit(2);
        }
    }
}

fn verify_fixture_bytes(input_bytes: &[u8]) -> Result<TimelineReport, Vec<String>> {
    let fixture: Fixture = serde_json::from_slice(input_bytes).map_err(|error| {
        vec![format!(
            "fixture JSON does not match the strict fleet-resilience schema: {error}"
        )]
    })?;

    let mut errors = validate_fixture(&fixture);
    if !errors.is_empty() {
        errors.sort();
        errors.dedup();
        return Err(errors);
    }

    let mut cases = fixture.disturbance_cases.clone();
    cases.sort_by(|left, right| left.case_id.cmp(&right.case_id));

    let maximum_observation_delay_ticks = cases
        .iter()
        .map(|case| case.observable_tick - case.injection_tick)
        .max()
        .unwrap_or(0);
    let delayed_observations = cases
        .iter()
        .filter(|case| case.observable_tick > case.injection_tick)
        .count();
    let total_observation_delay_ticks = cases.iter().try_fold(0_u64, |total, case| {
        total.checked_add(case.observable_tick - case.injection_tick)
    });
    let Some(total_observation_delay_ticks) = total_observation_delay_ticks else {
        return Err(vec!["observation delay total overflowed".to_string()]);
    };

    let last_tick = cases
        .iter()
        .map(|case| case.observable_tick.max(case.injection_tick))
        .max()
        .unwrap_or(0);
    let timeline_tick_count = last_tick
        .checked_add(1)
        .ok_or_else(|| vec!["timeline tick count overflowed".to_string()])?;

    let evaluator_timeline = (0..timeline_tick_count)
        .map(|tick| {
            let injected_event_ids = cases
                .iter()
                .filter(|case| case.injection_tick == tick)
                .map(|case| case.case_id.clone())
                .collect();
            let events_injected_but_not_yet_observable = cases
                .iter()
                .filter(|case| {
                    case.injection_tick <= tick && case.observable_tick > tick
                })
                .map(|case| case.case_id.clone())
                .collect();
            EvaluatorTick {
                tick,
                injected_event_ids,
                events_injected_but_not_yet_observable,
            }
        })
        .collect();

    // This projection contains only events observable by this tick. Injection
    // timestamps and future event state are deliberately not policy-visible.
    let policy_timeline = (0..timeline_tick_count)
        .map(|tick| {
            let visible_event_ids = cases
                .iter()
                .filter(|case| case.observable_tick <= tick)
                .map(|case| case.case_id.clone())
                .collect();
            let newly_observed_events = cases
                .iter()
                .filter(|case| case.observable_tick == tick)
                .map(|case| NewlyObservedEvent {
                    case_id: case.case_id.clone(),
                    observed_at_tick: tick,
                    event: case.event.as_str().to_string(),
                })
                .collect();
            PolicyTick {
                tick,
                visible_event_ids,
                newly_observed_events,
            }
        })
        .collect();

    let typed_disturbance_kinds = cases
        .iter()
        .map(|case| case.event)
        .collect::<BTreeSet<_>>()
        .len();

    let digest = Sha256::digest(input_bytes);
    Ok(TimelineReport {
        schema_version: "luminous.civ_infra.event_timeline_audit.v0.1".to_string(),
        fixture_id: fixture.fixture_id.clone(),
        input_sha256: format!("{digest:x}"),
        classification: fixture.classification.clone(),
        timeline_status: "canonical_event_queue_timing_audited".to_string(),
        timeline_audit_executed: true,
        policy_simulation_executed: false,
        policy_runs_performed: 0,
        counts: Counts {
            disturbance_cases: cases.len(),
            typed_disturbance_kinds,
            delayed_observations,
            total_observation_delay_ticks,
            maximum_observation_delay_ticks,
            timeline_tick_count,
        },
        evaluator_timeline,
        policy_timeline,
        claim_ceiling: fixture.claim_ceiling.clone(),
        explicit_limitations: vec![
            concat!(
                "This audits declared injection and observation times; it does not ",
                "simulate a construction policy."
            )
            .to_string(),
            concat!(
                "Evaluator-private event knowledge is represented separately and must not ",
                "be passed to a policy before observable_tick."
            )
            .to_string(),
            concat!(
                "The disturbance declarations are synthetic fixtures, not measurements ",
                "of real infrastructure or robot failures."
            )
            .to_string(),
            concat!(
                "A visible event is not proof that the event has been independently ",
                "verified or that a recovery action is authorized."
            )
            .to_string(),
        ],
    })
}

fn validate_fixture(fixture: &Fixture) -> Vec<String> {
    let mut errors = Vec::new();

    if fixture.schema_version != "civ-infra-fleet-resilience/0.1" {
        errors.push("unsupported schema_version".to_string());
    }
    if fixture.fixture_id.trim().is_empty() || fixture.claim_ceiling.trim().is_empty() {
        errors.push("fixture_id and claim_ceiling must be explicit".to_string());
    }
    if fixture.classification != "synthetic_only" {
        errors.push("classification must remain synthetic_only".to_string());
    }
    if fixture.clock.unit != "fixture_tick" || fixture.clock.interval_semantics != "half_open" {
        errors.push("unsupported clock unit or interval semantics".to_string());
    }
    if fixture.disturbance_cases.is_empty() {
        errors.push("disturbance_cases must not be empty".to_string());
    }

    let mut case_ids = BTreeSet::new();
    let mut event_kinds = BTreeSet::new();
    let mut maximum_tick = 0_u64;
    for case in &fixture.disturbance_cases {
        if case.case_id.trim().is_empty()
            || case.name.trim().is_empty()
            || case.event.trim().is_empty()
        {
            errors.push("disturbance case ID, name and event must be non-empty".to_string());
        }
        if !case_ids.insert(case.case_id.as_str()) {
            errors.push(format!("duplicate disturbance case ID: {}", case.case_id));
        }
        event_kinds.insert(case.event);
        if case.observable_tick < case.injection_tick {
            errors.push(format!(
                "{}: observation cannot precede event injection",
                case.case_id
            ));
        }
        if case.injection_tick > 100_000 || case.observable_tick > 100_000 {
            errors.push(format!("{}: timeline tick exceeds bounded audit horizon", case.case_id));
        }
        if case.affected_resources.iter().any(|item| item.trim().is_empty())
            || case.affected_tasks.iter().any(|item| item.trim().is_empty())
            || case.invalidates.iter().any(|item| item.trim().is_empty())
            || case.recovery_evidence.iter().any(|item| item.trim().is_empty())
            || case.required_invariants.iter().any(|item| item.trim().is_empty())
        {
            errors.push(format!(
                "{}: disturbance references must not contain empty IDs",
                case.case_id
            ));
        }
        maximum_tick = maximum_tick.max(case.observable_tick).max(case.injection_tick);
    }

    for kind in DisturbanceKind::ALL {
        if !event_kinds.contains(&kind) {
            errors.push(format!("required v0.1 disturbance kind is missing: {}", kind.as_str()));
        }
    }
    if event_kinds.len() != DisturbanceKind::ALL.len() {
        errors.push("v0.1 disturbance fixture must cover exactly the 13 typed event kinds".to_string());
    }
    if maximum_tick >= 100_000 {
        errors.push("timeline exceeds the bounded 100000-tick audit horizon".to_string());
    }
    errors
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture() -> Fixture {
        serde_json::from_str(EMBEDDED_FIXTURE).expect("embedded fixture must parse")
    }

    #[test]
    fn frozen_fixture_has_expected_event_queue_totals() {
        let bytes = EMBEDDED_FIXTURE.as_bytes();
        let report = verify_fixture_bytes(bytes).expect("frozen event timeline must validate");

        assert_eq!(report.counts.disturbance_cases, 13);
        assert_eq!(report.counts.typed_disturbance_kinds, 13);
        assert_eq!(report.counts.delayed_observations, 3);
        assert_eq!(report.counts.total_observation_delay_ticks, 4);
        assert_eq!(report.counts.maximum_observation_delay_ticks, 2);
        assert_eq!(report.counts.timeline_tick_count, 8);
        assert!(!report.policy_simulation_executed);
        assert_eq!(report.policy_runs_performed, 0);
    }

    #[test]
    fn delayed_event_is_evaluator_known_but_policy_hidden_until_observed() {
        let bytes = EMBEDDED_FIXTURE.as_bytes();
        let report = verify_fixture_bytes(bytes).expect("fixture must validate");
        let evaluator_tick_two = &report.evaluator_timeline[2];
        let policy_tick_three = &report.policy_timeline[3];
        let policy_tick_four = &report.policy_timeline[4];

        assert!(evaluator_tick_two
            .injected_event_ids
            .iter()
            .any(|item| item == "R-008"));
        assert!(evaluator_tick_two
            .events_injected_but_not_yet_observable
            .iter()
            .any(|item| item == "R-008"));
        assert!(!policy_tick_three
            .visible_event_ids
            .iter()
            .any(|item| item == "R-008"));
        assert!(policy_tick_four
            .visible_event_ids
            .iter()
            .any(|item| item == "R-008"));
        assert!(policy_tick_four
            .newly_observed_events
            .iter()
            .any(|item| {
                item.case_id == "R-008"
                    && item.observed_at_tick == 4
                    && item.event == "workface_geometry_revision_changed"
            }));
    }

    #[test]
    fn unknown_disturbance_kind_is_rejected() {
        let mut candidate: serde_json::Value =
            serde_json::from_str(EMBEDDED_FIXTURE).unwrap();
        candidate["disturbance_cases"][0]["event"] =
            serde_json::Value::String("unknown_event_kind".to_string());
        let bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(verify_fixture_bytes(&bytes).is_err());
    }

    #[test]
    fn observation_before_injection_is_rejected() {
        let mut candidate = fixture();
        candidate
            .disturbance_cases
            .iter_mut()
            .find(|case| case.case_id == "R-008")
            .unwrap()
            .observable_tick = 1;
        let bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(verify_fixture_bytes(&bytes).is_err());
    }

    #[test]
    fn missing_declared_disturbance_kind_is_rejected() {
        let mut candidate = fixture();
        candidate
            .disturbance_cases
            .retain(|case| case.case_id != "R-013");
        let bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(verify_fixture_bytes(&bytes).is_err());
    }

    #[test]
    fn duplicate_case_identity_is_rejected() {
        let mut candidate = fixture();
        candidate.disturbance_cases[1].case_id = candidate.disturbance_cases[0].case_id.clone();
        let bytes = serde_json::to_vec(&candidate).unwrap();
        assert!(verify_fixture_bytes(&bytes).is_err());
    }

    #[test]
    fn report_is_deterministic_for_identical_fixture_bytes() {
        let bytes = EMBEDDED_FIXTURE.as_bytes();
        let first = verify_fixture_bytes(bytes).unwrap();
        let second = verify_fixture_bytes(bytes).unwrap();
        assert_eq!(first, second);
    }
}
