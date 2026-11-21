use maritime_custody_continuity::{evaluate, parse_events_tsv, parse_expectations_tsv};

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
    println!("No real cargo custody, handoff, or delivery is inferred.");
}
