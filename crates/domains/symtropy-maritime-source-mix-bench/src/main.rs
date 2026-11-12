use std::{env, fs, process::ExitCode};
use serde_json::Value;
use symtropy_maritime_source_mix_bench::evaluate_case;

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 6 || args[2] != "--source-commit" || args[4] != "--case-sha256" {
        eprintln!("usage: source-mix-evaluator <case.json> --source-commit <40-hex> --case-sha256 <64-hex>");
        return ExitCode::from(2);
    }
    let raw = match fs::read_to_string(&args[1]) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("cannot read case: {e}");
            return ExitCode::FAILURE;
        }
    };
    let case: Value = match serde_json::from_str(&raw) {
        Ok(x) => x,
        Err(e) => {
            eprintln!("invalid JSON: {e}");
            return ExitCode::FAILURE;
        }
    };
    match evaluate_case(&case, &args[3], &args[5]) {
        Ok(report) => {
            match serde_json::to_string_pretty(&report) {
                Ok(s) => println!("{s}"),
                Err(e) => {
                    eprintln!("cannot serialize report: {e}");
                    return ExitCode::FAILURE;
                }
            }
            ExitCode::SUCCESS
        }
        Err(e) => {
            eprintln!("source-mix benchmark failed closed: {e}");
            ExitCode::FAILURE
        }
    }
}
