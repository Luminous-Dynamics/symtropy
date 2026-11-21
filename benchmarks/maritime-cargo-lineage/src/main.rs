use maritime_cargo_lineage::{evaluate, parse_crosswalk_tsv, parse_events_tsv};

fn main() {
    let mappings = parse_crosswalk_tsv(include_str!("../data/crosswalk.tsv")).unwrap_or_else(|error| {
        eprintln!("crosswalk fixture rejected: {error}");
        std::process::exit(2);
    });
    let events = parse_events_tsv(include_str!("../data/events.tsv")).unwrap_or_else(|error| {
        eprintln!("event fixture rejected: {error}");
        std::process::exit(2);
    });
    let report = evaluate(&mappings, &events, 200).unwrap_or_else(|error| {
        eprintln!("lineage evaluation rejected: {error:?}");
        std::process::exit(2);
    });

    println!("Maritime cargo lineage benchmark v0.1");
    println!("claim_ceiling=SYNTHETIC_IDENTITY_AND_DELIVERY_COUNTING_ONLY");
    println!("as_of=200");
    println!("input_event_rows={}", report.input_event_rows);
    println!("visible_event_rows={}", report.visible_event_rows);
    println!("future_event_rows={}", report.future_event_rows);
    println!("replayed_event_rows={}", report.replayed_event_rows);
    println!("planned_event_rows={}", report.planned_event_rows);
    println!("actual_event_rows={}", report.actual_event_rows);
    println!("resolved_actual_event_rows={}", report.resolved_actual_event_rows);
    println!("unresolved_actual_event_rows={}", report.unresolved_actual_event_rows);
    println!("unqualified_delivery_claims={}", report.unqualified_delivery_claims);
    println!("receiver_receipt_delivery_events={}", report.receiver_receipt_delivery_events);
    println!("unique_receipt_backed_boxes={}", report.unique_receipt_backed_boxes());

    for (reason, count) in report.unresolved_reasons {
        println!("unresolved.{reason:?}={count}");
    }
    println!("No real DGT cargo identities, movements or deliveries are inferred.");
}
