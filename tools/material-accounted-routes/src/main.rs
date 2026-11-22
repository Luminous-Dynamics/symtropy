use material_accounted_routes::{CASES, RECEIPTS, evaluate_corpus, render_report};
use std::process::ExitCode;

fn main() -> ExitCode {
    let report = match evaluate_corpus(CASES, RECEIPTS) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("material-accounted-routes: {error}");
            return ExitCode::FAILURE;
        }
    };

    let accepted = report
        .iter()
        .filter(|row| row.actual.as_str() == "ACCEPTED_SYNTHETIC")
        .count();
    let blocked = report
        .iter()
        .filter(|row| row.actual.as_str() == "BLOCKED")
        .count();
    let indeterminate = report
        .iter()
        .filter(|row| row.actual.as_str() == "INDETERMINATE")
        .count();
    let mismatches = report.iter().filter(|row| !row.expected_matches).count();

    print!("{}", render_report(&report));
    eprintln!(
        "cases={} accepted_synthetic={} blocked={} indeterminate={} expected_mismatches={}",
        report.len(),
        accepted,
        blocked,
        indeterminate,
        mismatches
    );

    if mismatches == 0 {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
