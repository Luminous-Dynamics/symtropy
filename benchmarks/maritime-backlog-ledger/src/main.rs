use maritime_backlog_ledger::evaluate_reference_corpus;

fn main() {
    let report = evaluate_reference_corpus().unwrap_or_else(|error| {
        eprintln!("ledger corpus could not be evaluated: {error}");
        std::process::exit(2);
    });

    println!("Maritime backlog ledger reference verifier v0.1");
    println!(
        "Cases: {} | accepted: {} | rejected: {} | expected dispositions matched: {}",
        report.total, report.accepted, report.rejected, report.matched
    );

    for case in &report.cases {
        println!(
            "{:<5} expected={:<6} actual={:<6} {}",
            case.case_id, case.expected, case.actual, case.detail
        );
    }

    if !report.all_expected_dispositions_match() {
        eprintln!("frozen fixture expectation mismatch");
        std::process::exit(1);
    }

    println!();
    println!("All frozen synthetic dispositions match.");
    println!("This is accounting/replay conformance, not a measured port-performance result.");
}
