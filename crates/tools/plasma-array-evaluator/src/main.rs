use plasma_array_evaluator::audit_manifest_json;
use std::{env, fs, process};

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/plasma/plasma-array-scaleout-v0.json".to_owned());

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            process::exit(2);
        }
    };

    let report = match audit_manifest_json(&source) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("audit error: {error}");
            process::exit(2);
        }
    };

    println!(
        "{}: {}/{} expectations matched ({} mismatched)",
        report.benchmark_id,
        report.matched_expectations,
        report.scenario_count,
        report.mismatched_expectations
    );
    for result in &report.results {
        println!(
            "{}: {} (expected {})",
            result.id, result.disposition, result.expected_disposition
        );
        for reason in &result.reasons {
            println!("  - {reason}");
        }
    }

    if report.mismatched_expectations != 0 {
        process::exit(1);
    }
}
