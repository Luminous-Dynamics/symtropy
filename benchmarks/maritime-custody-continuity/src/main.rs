use maritime_custody_continuity::{
    evaluate, parse_events_tsv, parse_expectations_tsv,
    temporal_order::{TimeBoundKind, TimeEvidence, TimeInterval, compare_time_evidence},
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
    let release_time = TimeEvidence::new(
        "SYNTHETIC_RELEASE",
        "UTC_NS_PROFILE_V1",
        TimeInterval::new(100, 110).expect("valid synthetic release interval"),
        TimeBoundKind::QualifiedEnclosure,
    )
    .expect("valid synthetic release time evidence");
    let accept_time = TimeEvidence::new(
        "SYNTHETIC_ACCEPT",
        "UTC_NS_PROFILE_V1",
        TimeInterval::new(111, 120).expect("valid synthetic acceptance interval"),
        TimeBoundKind::QualifiedEnclosure,
    )
    .expect("valid synthetic acceptance time evidence");
    let overlapping_time = TimeEvidence::new(
        "SYNTHETIC_OVERLAP",
        "UTC_NS_PROFILE_V1",
        TimeInterval::new(108, 115).expect("valid synthetic overlap interval"),
        TimeBoundKind::QualifiedEnclosure,
    )
    .expect("valid synthetic overlap time evidence");
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
