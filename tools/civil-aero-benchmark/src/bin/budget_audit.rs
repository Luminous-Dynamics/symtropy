use std::{env, fs, process};

use civil_aero_benchmark::budget::{
    parse_ledger_csv, summarize_matched_budget, LEDGER_OUTPUT_HEADER,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("experiment cost ledger rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let path = args
        .next()
        .unwrap_or_else(|| "data/decision_value/matched_budget_ledger.csv".to_owned());
    if args.next().is_some() {
        return Err("usage: budget_audit [operation-ledger.csv]".to_owned());
    }

    let input =
        fs::read_to_string(&path).map_err(|error| format!("cannot read {path}: {error}"))?;
    let operations = parse_ledger_csv(&input)?;
    let summaries = summarize_matched_budget(&operations)?;

    println!("{LEDGER_OUTPUT_HEADER}");
    for summary in &summaries {
        println!("{}", summary_csv_row(summary));
    }
    Ok(())
}

fn summary_csv_row(summary: &civil_aero_benchmark::budget::PolicyBudgetSummary) -> String {
    civil_aero_benchmark::budget::summary_csv_row(summary)
}
