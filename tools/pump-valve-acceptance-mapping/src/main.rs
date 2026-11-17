use pump_valve_acceptance_mapping::{evaluate_corpus, render_report, EXPECTED, PROFILES, SCENARIOS};
use std::process::ExitCode;

fn main() -> ExitCode {
    let results = match evaluate_corpus(PROFILES, SCENARIOS, EXPECTED) {
        Ok(results) => results,
        Err(error) => {
            eprintln!("pump-valve-acceptance-mapping: {error}");
            return ExitCode::FAILURE;
        }
    };

    let applicable = results
        .iter()
        .filter(|row| row.disposition.as_str() == "APPLICABLE")
        .count();
    let not_applicable = results
        .iter()
        .filter(|row| row.disposition.as_str() == "NOT_APPLICABLE")
        .count();
    let profile_incomplete = results
        .iter()
        .filter(|row| row.disposition.as_str() == "PROFILE_INCOMPLETE")
        .count();
    let evidence_incomplete = results
        .iter()
        .filter(|row| row.disposition.as_str() == "EVIDENCE_INCOMPLETE")
        .count();
    let stale = results
        .iter()
        .filter(|row| row.disposition.as_str() == "STALE_OR_SUPERSEDED")
        .count();
    let unsupported = results
        .iter()
        .filter(|row| row.disposition.as_str() == "UNSUPPORTED_CLAIM")
        .count();

    let mode = std::env::args().nth(1);
    match mode.as_deref() {
        None | Some("--report") => print!("{}", render_report(&results)),
        Some(other) => {
            eprintln!("unsupported argument: {other}");
            return ExitCode::FAILURE;
        }
    }
    eprintln!(
        "scenarios={} applicable={} not_applicable={} profile_incomplete={} evidence_incomplete={} stale_or_superseded={} unsupported_claim={}",
        results.len(),
        applicable,
        not_applicable,
        profile_incomplete,
        evidence_incomplete,
        stale,
        unsupported
    );
    ExitCode::SUCCESS
}
