use protective_systems_verifier::{evaluate_corpus, parse_corpus};
use std::{env, fs, process};

const EMBEDDED_FIXTURES: &str = include_str!("../../fixtures/benchmark-cases-v0.1.json");

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() > 1 {
        eprintln!("usage: protective-systems-verifier [fixture-corpus.json]");
        process::exit(2);
    }

    let input = match args.first() {
        Some(path) => match fs::read_to_string(path) {
            Ok(text) => text,
            Err(error) => {
                eprintln!("cannot read {}: {}", path, error);
                process::exit(2);
            }
        },
        None => EMBEDDED_FIXTURES.to_owned(),
    };

    let root = match parse_corpus(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid JSON: {error}");
            process::exit(2);
        }
    };

    let report = match evaluate_corpus(&root) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    };

    println!(
        "{}",
        serde_json::to_string_pretty(&report).expect("report serialization")
    );

    if !report.conformance_pass {
        process::exit(1);
    }
}
