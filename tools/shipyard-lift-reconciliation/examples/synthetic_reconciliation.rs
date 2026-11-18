use shipyard_lift_envelope::{EvidenceRef, EvidenceState};
use shipyard_lift_reconciliation::{
    canonical_coverage_bundle_json, canonical_event_ledger_json, reconcile, Channel,
    ExecutionBinding, ExecutionOutcome, HoistState, Observation, PreflightDisposition,
    ReconciliationPolicy, StreamCoverage, TelemetryEvent,
};
use std::env;
use std::error::Error;

fn evidence(id: &str, subject: &str, revision: &str) -> EvidenceRef {
    EvidenceRef {
        source_record_id: id.to_owned(),
        owner_id: format!("synthetic-owner::{id}"),
        revision: "r1".to_owned(),
        subject_id: subject.to_owned(),
        subject_revision: revision.to_owned(),
        sha256: "a".repeat(64),
        validity_clock_domain_id: "yard-clock".to_owned(),
        valid_from_tick: 0,
        valid_until_tick: 1_000,
        state: EvidenceState::Current,
    }
}

fn channel_source(channel: Channel) -> (&'static str, &'static str) {
    match channel {
        Channel::HookLoad => ("load-cell-01", "load-cell-r2"),
        Channel::WindGust => ("wind-sensor-01", "wind-sensor-r3"),
        Channel::CraneConfiguration => ("crane-controller-01", "controller-r5"),
        Channel::MotionEnvelope => ("motion-oracle-01", "oracle-r1"),
        Channel::OccupancyProjection => ("occupancy-oracle-01", "oracle-r1"),
        Channel::OverloadAlarm => ("overload-relay-01", "relay-r2"),
        Channel::HoistState => ("hoist-controller-01", "controller-r5"),
    }
}

fn event(
    channel: Channel,
    sequence: u64,
    tick: u64,
    observation: Observation,
) -> TelemetryEvent {
    let (source_id, source_revision) = channel_source(channel);
    TelemetryEvent {
        event_id: format!("synthetic-event::{channel:?}::{sequence}"),
        source_id: source_id.to_owned(),
        source_revision: source_revision.to_owned(),
        sequence,
        event_tick: tick,
        received_tick: tick + 2,
        clock_domain_id: "yard-clock".to_owned(),
        calibration_or_source_evidence: evidence(
            &format!("calibration::{source_id}"),
            source_id,
            source_revision,
        ),
        observation,
    }
}

fn main() -> Result<(), Box<dyn Error>> {
    let channels = vec![
        Channel::HookLoad,
        Channel::WindGust,
        Channel::CraneConfiguration,
        Channel::MotionEnvelope,
        Channel::OccupancyProjection,
        Channel::OverloadAlarm,
        Channel::HoistState,
    ];
    let mut policy = ReconciliationPolicy {
        policy_id: "synthetic-reconciliation-policy-01".to_owned(),
        policy_revision: "policy-r1".to_owned(),
        // Populated from the exact canonical policy bytes by the workflow.
        policy_sha256: "0".repeat(64),
        required_channels: channels.clone(),
        maximum_sample_gap_ticks: 60,
        maximum_ingest_lateness_ticks: 10,
    };

    let events = vec![
        event(Channel::HookLoad, 0, 110, Observation::HookLoad { upper_mn: 400_000_000 }),
        event(Channel::HookLoad, 1, 150, Observation::HookLoad { upper_mn: 410_000_000 }),
        event(Channel::HookLoad, 2, 190, Observation::HookLoad { upper_mn: 405_000_000 }),
        event(Channel::WindGust, 0, 110, Observation::WindGust { upper_mm_per_s: 8_000 }),
        event(Channel::WindGust, 1, 150, Observation::WindGust { upper_mm_per_s: 9_000 }),
        event(Channel::WindGust, 2, 190, Observation::WindGust { upper_mm_per_s: 8_500 }),
        event(Channel::CraneConfiguration, 0, 110, Observation::CraneConfiguration {
            crane_id: "gantry-01".to_owned(),
            configuration_revision: "gantry-r3".to_owned(),
        }),
        event(Channel::MotionEnvelope, 0, 150, Observation::MotionEnvelope {
            report_id: "motion-01".to_owned(),
            report_sha256: "3".repeat(64),
            within_envelope: true,
        }),
        event(Channel::OccupancyProjection, 0, 150, Observation::OccupancyProjection {
            projection_id: "occupancy-01".to_owned(),
            projection_sha256: "4".repeat(64),
            covers_lift: true,
        }),
        event(Channel::OverloadAlarm, 0, 110, Observation::OverloadAlarm { active: false }),
        event(Channel::HoistState, 0, 110, Observation::HoistState { state: HoistState::Hoisting }),
        event(Channel::HoistState, 1, 190, Observation::HoistState { state: HoistState::Lowering }),
    ];

    let mut coverage = Vec::new();
    for channel in channels {
        let (source_id, source_revision) = channel_source(channel);
        let next_sequence_exclusive = events
            .iter()
            .filter(|item| item.observation.channel() == channel)
            .count() as u64;
        coverage.push(StreamCoverage {
            channel,
            source_id: source_id.to_owned(),
            source_revision: source_revision.to_owned(),
            first_sequence: 0,
            next_sequence_exclusive,
            covered_start_tick: 100,
            covered_end_tick: 200,
            complete: true,
            evidence: evidence(
                &format!("coverage::{source_id}"),
                source_id,
                source_revision,
            ),
        });
    }

    if env::args().any(|arg| arg == "--emit-inputs") {
        // Output order is fixed: policy, coverage bundle, event ledger.
        println!("{}", policy.canonical_json());
        println!("{}", canonical_coverage_bundle_json(&coverage));
        println!("{}", canonical_event_ledger_json(&events));
        return Ok(());
    }

    policy.policy_sha256 = env::var("RECONCILIATION_POLICY_SHA256")?;
    let event_ledger_sha256 = env::var("EVENT_LEDGER_SHA256")?;
    let coverage_bundle_sha256 = env::var("COVERAGE_BUNDLE_SHA256")?;

    let binding = ExecutionBinding {
        session_id: "synthetic-session-001".to_owned(),
        plan_id: "synthetic-plan-001".to_owned(),
        plan_revision: "plan-r4".to_owned(),
        plan_input_sha256: "1".repeat(64),
        preflight_bound_input_sha256: "1".repeat(64),
        preflight_report_sha256: "2".repeat(64),
        event_ledger_sha256,
        coverage_bundle_sha256,
        preflight_disposition: PreflightDisposition::ReadyForHumanReview,
        authorisation_id: "synthetic-authorisation-001".to_owned(),
        authorisation_revision: "auth-r2".to_owned(),
        authorisation_evidence: evidence(
            "synthetic-authorisation-evidence",
            "synthetic-plan-001",
            "plan-r4",
        ),
        block_id: "synthetic-block-001".to_owned(),
        block_revision: "block-r7".to_owned(),
        crane_id: "gantry-01".to_owned(),
        crane_configuration_revision: "gantry-r3".to_owned(),
        site_epoch_id: "yard-epoch-2".to_owned(),
        frame_id: "yard-frame-mm".to_owned(),
        clock_domain_id: "yard-clock".to_owned(),
        start_tick: 100,
        end_tick: 200,
        maximum_hook_load_mn: 500_000_000,
        hook_load_limit_evidence: evidence(
            "synthetic-hook-limit-evidence",
            "gantry-01",
            "gantry-r3",
        ),
        maximum_gust_mm_per_s: 10_000,
        gust_limit_profile_id: "wind-limit-01".to_owned(),
        gust_limit_profile_revision: "wind-limit-r2".to_owned(),
        gust_limit_evidence: evidence(
            "synthetic-gust-limit-evidence",
            "wind-limit-01",
            "wind-limit-r2",
        ),
        motion_report_id: "motion-01".to_owned(),
        motion_report_sha256: "3".repeat(64),
        occupancy_projection_id: "occupancy-01".to_owned(),
        occupancy_projection_sha256: "4".repeat(64),
        outcome: ExecutionOutcome::Completed,
    };

    let report = reconcile(&binding, &policy, &coverage, &events);
    assert_eq!(
        report.disposition,
        shipyard_lift_reconciliation::Disposition::ConformingToDeclaredEnvelope,
        "synthetic reconciliation fixture must be conforming only to its declared evidence contract"
    );
    println!("{}", report.canonical_json());
    Ok(())
}
