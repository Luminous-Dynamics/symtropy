use geofusion_fmi_trace_verifier::{
    validate_trace, ClockBinding, CommitBarrier, FederateProfile, RunTrace, StepAttempt,
    StepOutcome,
};

fn profile(
    id: &str,
    model_digest: &str,
    binding_id: &str,
    mapping_digest: &str,
    ns_num: i64,
    ns_den: i64,
) -> FederateProfile {
    FederateProfile {
        federate_id: id.to_owned(),
        model_digest: model_digest.to_owned(),
        clock: ClockBinding {
            binding_id: binding_id.to_owned(),
            model_clock_id: format!("clock-{id}"),
            mapping_digest: mapping_digest.to_owned(),
            local_epoch_tick: 0,
            master_epoch_ns: 0,
            nanoseconds_per_tick_numerator: ns_num,
            nanoseconds_per_tick_denominator: ns_den,
        },
        initial_local_tick: 0,
        early_return_allowed: true,
        event_mode_enabled: false,
    }
}

#[allow(clippy::too_many_arguments)] // Mirrors the complete frozen step-attempt record.
fn attempt(
    attempt_id: &str,
    federate_id: &str,
    model_digest: &str,
    binding_id: &str,
    mapping_digest: &str,
    sequence: u64,
    retry_of: Option<&str>,
    local_start: i64,
    local_requested: i64,
    local_returned: i64,
    ns_per_tick: i64,
    outcome: StepOutcome,
) -> StepAttempt {
    StepAttempt {
        attempt_id: attempt_id.to_owned(),
        federate_id: federate_id.to_owned(),
        model_digest: model_digest.to_owned(),
        clock_binding_id: binding_id.to_owned(),
        clock_mapping_digest: mapping_digest.to_owned(),
        sequence,
        retry_of: retry_of.map(str::to_owned),
        local_start_tick: local_start,
        local_requested_end_tick: local_requested,
        local_returned_end_tick: local_returned,
        master_start_ns: local_start * ns_per_tick,
        master_requested_end_ns: local_requested * ns_per_tick,
        master_returned_end_ns: local_returned * ns_per_tick,
        state_before_digest: format!("state:{federate_id}:{local_start}"),
        state_after_digest: if outcome == StepOutcome::Rejected {
            format!("state:{federate_id}:{local_start}")
        } else {
            format!("state:{federate_id}:{local_returned}")
        },
        event_handling_needed: false,
        terminate_simulation: false,
        outcome,
    }
}

fn valid_trace() -> RunTrace {
    let geothermal = profile(
        "geothermal",
        "sha256:model-geothermal-v1",
        "clock-binding-geothermal-v1",
        "sha256:clock-map-geothermal-v1",
        1_000_000_000,
        1,
    );
    let power_cycle = profile(
        "power-cycle",
        "sha256:model-power-cycle-v1",
        "clock-binding-power-cycle-v1",
        "sha256:clock-map-power-cycle-v1",
        500_000_000,
        1,
    );

    let attempts = vec![
        attempt(
            "geo-step-0", "geothermal", "sha256:model-geothermal-v1",
            "clock-binding-geothermal-v1", "sha256:clock-map-geothermal-v1",
            0, None, 0, 1, 1, 1_000_000_000, StepOutcome::Accepted,
        ),
        attempt(
            "power-step-0", "power-cycle", "sha256:model-power-cycle-v1",
            "clock-binding-power-cycle-v1", "sha256:clock-map-power-cycle-v1",
            0, None, 0, 2, 2, 500_000_000, StepOutcome::Accepted,
        ),
        attempt(
            "geo-step-1", "geothermal", "sha256:model-geothermal-v1",
            "clock-binding-geothermal-v1", "sha256:clock-map-geothermal-v1",
            1, None, 1, 3, 2, 1_000_000_000, StepOutcome::EarlyReturn,
        ),
        attempt(
            "power-step-1", "power-cycle", "sha256:model-power-cycle-v1",
            "clock-binding-power-cycle-v1", "sha256:clock-map-power-cycle-v1",
            1, None, 2, 4, 4, 500_000_000, StepOutcome::Accepted,
        ),
        attempt(
            "geo-step-2-reject", "geothermal", "sha256:model-geothermal-v1",
            "clock-binding-geothermal-v1", "sha256:clock-map-geothermal-v1",
            2, None, 2, 3, 2, 1_000_000_000, StepOutcome::Rejected,
        ),
        attempt(
            "geo-step-2-retry", "geothermal", "sha256:model-geothermal-v1",
            "clock-binding-geothermal-v1", "sha256:clock-map-geothermal-v1",
            2, Some("geo-step-2-reject"), 2, 3, 3, 1_000_000_000,
            StepOutcome::Accepted,
        ),
        attempt(
            "power-step-2", "power-cycle", "sha256:model-power-cycle-v1",
            "clock-binding-power-cycle-v1", "sha256:clock-map-power-cycle-v1",
            2, None, 4, 6, 6, 500_000_000, StepOutcome::Accepted,
        ),
    ];

    RunTrace {
        run_id: "synthetic-geofusion-run-001".to_owned(),
        federates: vec![geothermal, power_cycle],
        attempts,
        barriers: vec![
            CommitBarrier {
                barrier_id: "barrier-1s".to_owned(),
                sequence: 0,
                master_time_ns: 1_000_000_000,
                participant_attempt_ids: vec![
                    "geo-step-0".to_owned(),
                    "power-step-0".to_owned(),
                ],
            },
            CommitBarrier {
                barrier_id: "barrier-2s-early-return".to_owned(),
                sequence: 1,
                master_time_ns: 2_000_000_000,
                participant_attempt_ids: vec![
                    "geo-step-1".to_owned(),
                    "power-step-1".to_owned(),
                ],
            },
            CommitBarrier {
                barrier_id: "barrier-3s-after-retry".to_owned(),
                sequence: 2,
                master_time_ns: 3_000_000_000,
                participant_attempt_ids: vec![
                    "geo-step-2-retry".to_owned(),
                    "power-step-2".to_owned(),
                ],
            },
        ],
    }
}

fn assert_code(trace: &RunTrace, expected: &str) {
    let error = validate_trace(trace).expect_err("trace should be rejected");
    assert_eq!(error.code(), expected, "{error:?}");
}

#[test]
fn accepts_mixed_clock_scales_early_return_and_rejected_retry() {
    let trace = valid_trace();
    assert_eq!(validate_trace(&trace), Ok(()));
}


#[test]
fn accepted_step_may_report_last_successful_time_before_requested_endpoint() {
    let mut trace = valid_trace();
    trace.attempts[0].local_requested_end_tick = 2;
    trace.attempts[0].master_requested_end_ns = 2_000_000_000;
    assert_eq!(validate_trace(&trace), Ok(()));
}

#[test]
fn clock_binding_maps_exactly_without_float_rounding() {
    let trace = valid_trace();
    let binding = &trace.federates[1].clock;
    assert_eq!(binding.to_master_ns(4).unwrap(), 2_000_000_000);
    assert_eq!(binding.to_master_ns(5).unwrap(), 2_500_000_000);
}

#[test]
fn frozen_adversarial_corpus() {
    let fixture = include_str!("../fixtures/frozen-cases.tsv");
    let mut seen = std::collections::BTreeSet::new();

    for line in fixture.lines().skip(1).filter(|line| !line.trim().is_empty()) {
        let fields: Vec<_> = line.split('\t').collect();
        assert_eq!(fields.len(), 3, "malformed fixture row: {line}");
        let case_id = fields[0];
        let mutation = fields[1];
        let expected = fields[2];
        assert!(seen.insert(case_id), "duplicate fixture case: {case_id}");

        let mut trace = valid_trace();
        match mutation {
            "model_digest_mismatch" => {
                trace.attempts[2].model_digest = "sha256:stale-model".to_owned();
            }
            "clock_mapping_mismatch" => {
                trace.attempts[0].master_returned_end_ns += 1;
            }
            "rejected_step_advanced" => {
                trace.attempts[4].local_returned_end_tick = 3;
                trace.attempts[4].master_returned_end_ns = 3_000_000_000;
            }
            "retry_link_mismatch" => {
                trace.attempts[5].retry_of = None;
            }
            "retry_start_mismatch" => {
                trace.attempts[5].local_start_tick = 3;
                trace.attempts[5].master_start_ns = 3_000_000_000;
                trace.attempts[5].local_requested_end_tick = 4;
                trace.attempts[5].master_requested_end_ns = 4_000_000_000;
                trace.attempts[5].local_returned_end_tick = 4;
                trace.attempts[5].master_returned_end_ns = 4_000_000_000;
            }
            "retry_target_widened" => {
                trace.attempts[5].local_requested_end_tick = 4;
                trace.attempts[5].master_requested_end_ns = 4_000_000_000;
                trace.attempts[5].local_returned_end_tick = 4;
                trace.attempts[5].master_returned_end_ns = 4_000_000_000;
            }
            "unresolved_rejection" => {
                trace.attempts[5].outcome = StepOutcome::Rejected;
                trace.attempts[5].local_returned_end_tick = 2;
                trace.attempts[5].master_returned_end_ns = 2_000_000_000;
                trace.attempts[5].state_after_digest =
                    trace.attempts[5].state_before_digest.clone();
            }
            "missing_barrier_participant" => {
                trace.barriers[0].participant_attempt_ids.pop();
            }
            "duplicate_barrier_participant" => {
                trace.barriers[0].participant_attempt_ids[1] =
                    trace.barriers[0].participant_attempt_ids[0].clone();
            }
            "barrier_time_mismatch" => {
                trace.barriers[0].master_time_ns = 1_500_000_000;
            }
            "inexact_clock_mapping" => {
                trace.federates[1].clock.nanoseconds_per_tick_numerator = 1;
                trace.federates[1].clock.nanoseconds_per_tick_denominator = 2;
                trace.federates[1].initial_local_tick = 1;
            }
            "sequence_gap" => {
                trace.attempts[2].sequence = 2;
                trace.attempts[4].sequence = 3;
                trace.attempts[5].sequence = 3;
            }
            "rejected_state_changed" => {
                trace.attempts[4].state_after_digest = "state:mutated".to_owned();
            }
            "early_return_without_permission" => {
                trace.federates[0].early_return_allowed = false;
            }
            "zero_progress_early_return" => {
                trace.attempts[2].local_returned_end_tick = 1;
                trace.attempts[2].master_returned_end_ns = 1_000_000_000;
            }
            "event_without_event_mode" => {
                trace.attempts[2].event_handling_needed = true;
            }
            "event_mode_without_round_trip" => {
                trace.federates[0].event_mode_enabled = true;
                trace.attempts[2].event_handling_needed = true;
            }
            "fmu_requests_termination" => {
                trace.attempts[2].terminate_simulation = true;
            }
            "retry_recorded_after_later_step" => {
                let mut future = trace.attempts[5].clone();
                future.attempt_id = "geo-step-3".to_owned();
                future.sequence = 3;
                future.retry_of = None;
                future.local_start_tick = 3;
                future.local_requested_end_tick = 4;
                future.local_returned_end_tick = 4;
                future.master_start_ns = 3_000_000_000;
                future.master_requested_end_ns = 4_000_000_000;
                future.master_returned_end_ns = 4_000_000_000;
                trace.attempts.insert(5, future);
            }
            "early_return_at_requested_endpoint" => {
                trace.attempts[2].local_returned_end_tick = 3;
                trace.attempts[2].master_returned_end_ns = 3_000_000_000;
            }
            "clock_binding_identity_mismatch" => {
                trace.attempts[0].clock_binding_id = "stale-clock-binding".to_owned();
            }
            "invalid_negative_clock_scale" => {
                trace.federates[0].clock.nanoseconds_per_tick_numerator = -1;
            }
            "clock_mapping_exceeds_master_range" => {
                trace.federates[0].clock.local_epoch_tick = i64::MIN;
                trace.federates[0].initial_local_tick = i64::MAX;
                trace.federates[0].clock.nanoseconds_per_tick_numerator = i64::MAX;
            }
            "missing_state_snapshot_digest" => {
                trace.attempts[0].state_after_digest.clear();
            }
            "barrier_sequence_skipped" => {
                trace.barriers[1].sequence = 2;
            }
            other => panic!("unknown mutation in frozen corpus: {other}"),
        }

        assert_code(&trace, expected);
    }

    assert_eq!(seen.len(), 25);
}
