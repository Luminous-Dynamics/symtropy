use protective_systems_verifier::robust::{parse_problem, run};
use std::{env, fs, process};

const EMBEDDED: &str =
    include_str!("../../../robust/fixtures/finite-ambiguity-v0.1.json");

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 1 {
        eprintln!("usage: protective-robust [problem.json]");
        process::exit(2);
    }
    let input = match args.first() {
        Some(path) => match fs::read_to_string(path) {
            Ok(value) => value,
            Err(error) => {
                eprintln!("cannot read {}: {}", path, error);
                process::exit(2);
            }
        },
        None => EMBEDDED.to_owned(),
    };
    let problem = match parse_problem(&input) {
        Ok(problem) => problem,
        Err(error) => {
            eprintln!("invalid robust-design JSON: {error}");
            process::exit(2);
        }
    };
    let report = match run(&problem) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("invalid robust-design problem: {error}");
            process::exit(2);
        }
    };
    println!("{}", serde_json::to_string_pretty(&report).expect("serialize report"));
    if !report.oracle_matches_fixture {
        process::exit(1);
    }
}
