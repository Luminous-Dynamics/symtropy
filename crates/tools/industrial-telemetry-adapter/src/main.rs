#![forbid(unsafe_code)]

use industrial_telemetry_adapter::parse_and_summarize;
use std::env;
use std::fs;
use std::process::ExitCode;

const NIST_SOURCE_COMMIT: &str = "968279f14ebe96c03c8877eeb1901cf6b4c8fbab";
const NIST_SOURCE_BLOB: &str = "aa5e49ad87a787573259c4318b1516196e8b349a";

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!(
            "usage: {} <local-three-column-trace.txt>",
            std::path::Path::new(&program).display()
        );
        eprintln!("parser performs no network access and does not infer equipment availability");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one input path");
        return ExitCode::from(2);
    }

    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!("cannot read UTF-8 trace {}: {error}", std::path::Path::new(&path).display());
            return ExitCode::from(1);
        }
    };
    let summary = match parse_and_summarize(&content) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("trace rejected: {error}");
            return ExitCode::from(1);
        }
    };

    println!("industrial-telemetry-adapter v0.1");
    println!("input_path={}", std::path::Path::new(&path).display());
    println!("reference_nist_commit={NIST_SOURCE_COMMIT}");
    println!("reference_nist_git_blob_sha1={NIST_SOURCE_BLOB}");
    println!("note=reference identity is not independently checked by this parser; verify downloaded file with git hash-object");
    println!("record_count={}", summary.record_count);
    println!("distinct_field_count={}", summary.distinct_field_count);
    println!("first_event_utc={}", summary.first_event_utc);
    println!("last_event_utc={}", summary.last_event_utc);
    println!("timestamp_inversions={}", summary.timestamp_inversions);
    println!("adjacent_same_timestamp_pairs={}", summary.adjacent_same_timestamp_pairs);
    println!("unique_timestamp_count={}", summary.unique_timestamp_count);
    match summary.maximum_gap_ns {
        Some(gap_ns) => {
            println!("maximum_gap_ns={gap_ns}");
            println!("maximum_gap_minutes={:.9}", gap_ns as f64 / 60_000_000_000.0);
        }
        None => {
            println!("maximum_gap_ns=not_defined_fewer_than_two_unique_timestamps");
            println!("maximum_gap_minutes=not_defined_fewer_than_two_unique_timestamps");
        }
    }
    println!("field_counts:");
    for (field, count) in &summary.field_counts {
        println!("  {field}={count}");
    }
    println!("claim_ceiling=event chronology and sampling gaps only; no availability, safety, quality, energy, or cost inference");
    ExitCode::SUCCESS
}
