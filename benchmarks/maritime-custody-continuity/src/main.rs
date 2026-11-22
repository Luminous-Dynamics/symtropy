use maritime_custody_continuity::{
    evaluate, parse_events_tsv, parse_expectations_tsv,
    temporal_order::{
        ClockBoundProfile, ClockBoundProfileSpec, ClockSyncState, compare_time_evidence,
    },
};

fn main() {
    let expectations = parse_expectations_tsv(include_str!("../data/expectations.tsv"))
        .unwrap_or_else(|error| {
            eprintln!("expectation fixture rejected: {error}");
            std::process::exit(2);
        });
    let events = parse_events_tsv(include_str!("../data/events.tsv")).unwrap_or_else(|error| {
        eprintln!("event fixture rejected: {error}");
        std::process::exit(2);
    });
    let report = evaluate(&expectations, &events, 200).unwrap_or_else(|error| {
        eprintln!("custody evaluation rejected: {error:?}");
        std::process::exit(2);
    });

    println!("Maritime custody continuity benchmark v0.1");
    println!("claim_ceiling=SYNTHETIC_CUSTODY_STATE_MACHINE_ONLY");
    println!("as_of=200");
    println!("input_event_rows={}", report.input_event_rows);
    println!("visible_event_rows={}", report.visible_event_rows);
    println!("future_event_rows={}", report.future_event_rows);
    println!("superseded_event_rows={}", report.superseded_event_rows);
    println!("replayed_event_rows={}", report.replayed_event_rows);
    println!("voided_event_rows={}", report.voided_event_rows);
    println!("planned_event_rows={}", report.planned_event_rows);
    println!("actual_event_rows={}", report.actual_event_rows);
    println!("complete_chains={}", report.complete_chains());
    println!("incomplete_chains={}", report.incomplete_box_ids.len());
    println!("rejected_chains={}", report.rejected_box_ids.len());
    println!("unknown_box_events={}", report.unknown_box_events);
    for (failure, count) in report.failure_counts {
        println!("failure.{failure:?}={count}");
    }
    let synthetic_clock = ClockBoundProfile::new(ClockBoundProfileSpec {
        clock_id: "SYNTHETIC_CLOCK_01".to_string(),
        profile_id: "SYNTHETIC_CLOCK_PROFILE_V1".to_string(),
        time_basis_id: "UTC_NS_PROFILE_V1".to_string(),
        calibration_evidence_id: "SYNTHETIC_CALIBRATION_RECEIPT".to_string(),
        synchronization_method: "SYNTHETIC_REFERENCE_CLOCK".to_string(),
        sync_state: ClockSyncState::Locked,
        calibrated_at: 0,
        valid_until: 1_000,
        max_offset_error_ticks: Some(2),
        max_holdover_drift_ticks: Some(1),
        max_capture_path_ticks: Some(1),
        max_rounding_error_ticks: Some(1),
    })
    .expect("valid synthetic clock profile");
    let release_time = synthetic_clock
        .qualify_event("SYNTHETIC_RELEASE", 105)
        .expect("qualified synthetic release time");
    let accept_time = synthetic_clock
        .qualify_event("SYNTHETIC_ACCEPT", 116)
        .expect("qualified synthetic acceptance time");
    let overlapping_time = synthetic_clock
        .qualify_event("SYNTHETIC_OVERLAP", 113)
        .expect("qualified synthetic overlapping time");
    println!(
        "synthetic_disjoint_handoff_time_order={:?}",
        compare_time_evidence(&release_time, &accept_time)
    );
    println!(
        "synthetic_overlapping_handoff_time_order={:?}",
        compare_time_evidence(&release_time, &overlapping_time)
    );
    println!("Time-order examples are a standalone P1 primitive, not a real custody decision.");
    println!("No real cargo custody, handoff, or delivery is inferred.");
}
