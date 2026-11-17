use factory_accepted_output_economics::{
    evaluate_corpus, render_report, CASES, COST_ITEMS, EXPECTED, OUTPUTS,
};
use std::process::ExitCode;

fn main() -> ExitCode {
    let results = match evaluate_corpus(CASES, COST_ITEMS, OUTPUTS, EXPECTED) {
        Ok(results) => results,
        Err(error) => {
            eprintln!("factory-accepted-output-economics: {error}");
            return ExitCode::FAILURE;
        }
    };

    let computed = results
        .iter()
        .filter(|row| row.disposition.as_str() == "COMPUTED")
        .count();
    let cost_incomplete = results
        .iter()
        .filter(|row| row.disposition.as_str() == "COST_INCOMPLETE")
        .count();
    let zero_released = results
        .iter()
        .filter(|row| row.disposition.as_str() == "ZERO_RELEASED")
        .count();
    let ineligible = results
        .iter()
        .filter(|row| row.disposition.as_str() == "ROUTE_INELIGIBLE")
        .count();
    let unknown = results
        .iter()
        .filter(|row| row.disposition.as_str() == "ROUTE_UNKNOWN")
        .count();

    print!("{}", render_report(&results));
    eprintln!(
        "cases={} computed={} cost_incomplete={} zero_released={} route_ineligible={} route_unknown={}",
        results.len(),
        computed,
        cost_incomplete,
        zero_released,
        ineligible,
        unknown
    );
    ExitCode::SUCCESS
}
