use plasma_diagnostic_fault_replay::audit_manifest_json;
use std::{env, fs, process};

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/plasma/diagnostic-fault-replay-v0.json".to_owned());

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
        "{}: {}/{} expected dispositions matched; {} mismatched",
        report.benchmark_id,
        report.matched_expectations,
        report.case_count,
        report.mismatched_expectations
    );
    for result in report.results {
        println!(
            "{}: {} (expected {})",
            result.case_id, result.actual_disposition, result.expected_disposition
        );
        if let Some(error) = result.error {
            println!("  refusal: {error}");
        }
        if let Some(digest) = result.receipt_sha256 {
            println!("  receipt: {digest}");
        }
    }

    if report.mismatched_expectations != 0 {
        process::exit(1);
    }
}
