// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

#![forbid(unsafe_code)]

use industrial_telemetry_adapter::{TraceSummary, parse_and_summarize};
use std::env;
use std::fs;
use std::process::ExitCode;

const NIST_SOURCE_COMMIT: &str = "968279f14ebe96c03c8877eeb1901cf6b4c8fbab";
const NIST_SOURCE_BLOB: &str = "aa5e49ad87a787573259c4318b1516196e8b349a";

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(mode) = args.next() else {
        print_usage();
        return ExitCode::from(2);
    };
    let Some(path) = args.next() else {
        print_usage();
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected a mode and exactly one input path");
        print_usage();
        return ExitCode::from(2);
    }

    let Some(mode) = mode.to_str() else {
        eprintln!("mode must be valid UTF-8");
        return ExitCode::from(2);
    };
    if mode != "summarize" && mode != "verify-nist-reference" {
        eprintln!("unknown mode: {mode}");
        print_usage();
        return ExitCode::from(2);
    }

    let content = match fs::read_to_string(&path) {
        Ok(content) => content,
        Err(error) => {
            eprintln!(
                "cannot read UTF-8 trace {}: {error}",
                std::path::Path::new(&path).display()
            );
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

    print_summary(&path, &summary);
    if mode == "summarize" {
        println!("reference_profile_comparison=not_requested");
        return ExitCode::SUCCESS;
    }

    match verify_nist_profile(content.len(), &summary) {
        Ok(()) => {
            println!("reference_profile_match=true");
            println!("warning=profile match is not a cryptographic source identity check");
            println!("verify_expected_git_blob_sha1_with=git hash-object <input-path>");
            ExitCode::SUCCESS
        }
        Err(reason) => {
            println!("reference_profile_match=false");
            eprintln!("NIST profile check failed: {reason}");
            ExitCode::from(1)
        }
    }
}

fn print_usage() {
    eprintln!("usage: industrial-telemetry-adapter <summarize|verify-nist-reference> <trace.txt>");
    eprintln!("the CLI reads a local file only and never infers equipment availability");
}

fn print_summary(path: &std::ffi::OsStr, summary: &TraceSummary) {
    println!("industrial-telemetry-adapter v0.1");
    println!("input_path={}", std::path::Path::new(path).display());
    println!("expected_nist_source_commit={NIST_SOURCE_COMMIT}");
    println!("expected_nist_git_blob_sha1={NIST_SOURCE_BLOB}");
    println!("input_blob_sha1_verified_by_cli=false");
    println!("record_count={}", summary.record_count);
    println!("distinct_field_count={}", summary.distinct_field_count);
    println!("first_event_utc={}", summary.first_event_utc);
    println!("last_event_utc={}", summary.last_event_utc);
    println!("timestamp_inversions={}", summary.timestamp_inversions);
    println!(
        "adjacent_same_timestamp_pairs={}",
        summary.adjacent_same_timestamp_pairs
    );
    println!("unique_timestamp_count={}", summary.unique_timestamp_count);
    match summary.maximum_gap_ns {
        Some(gap_ns) => {
            println!("maximum_gap_ns={gap_ns}");
            println!(
                "maximum_gap_minutes={:.9}",
                gap_ns as f64 / 60_000_000_000.0
            );
            println!(
                "maximum_gap_start_timestamp_unix_ns={}",
                summary
                    .maximum_gap_start_timestamp_unix_ns
                    .unwrap_or_default()
            );
            println!(
                "maximum_gap_end_timestamp_unix_ns={}",
                summary
                    .maximum_gap_end_timestamp_unix_ns
                    .unwrap_or_default()
            );
        }
        None => {
            println!("maximum_gap_ns=not_defined_fewer_than_two_unique_timestamps");
            println!("maximum_gap_minutes=not_defined_fewer_than_two_unique_timestamps");
            println!("maximum_gap_start_timestamp_unix_ns=not_defined");
            println!("maximum_gap_end_timestamp_unix_ns=not_defined");
        }
    }
    println!("field_counts:");
    for (field, count) in &summary.field_counts {
        println!("  {field}={count}");
    }
    println!(concat!(
        "claim_ceiling=event chronology and sampling gaps only; ",
        "no availability, safety, quality, energy, or cost inference"
    ));
}

fn verify_nist_profile(byte_len: usize, summary: &TraceSummary) -> Result<(), &'static str> {
    if byte_len != 600_974 {
        return Err("byte length differs from pinned source profile");
    }
    if summary.record_count != 11_719 || summary.distinct_field_count != 11 {
        return Err("record or field count differs from pinned source profile");
    }
    if summary.first_event_utc != "2017-02-02T16:37:08.9062500Z"
        || summary.last_event_utc != "2017-02-02T18:35:48.9687500Z"
    {
        return Err("first/last timestamp differs from pinned source profile");
    }
    if summary.timestamp_inversions != 0
        || summary.adjacent_same_timestamp_pairs != 458
        || summary.unique_timestamp_count != 11_261
    {
        return Err("timestamp order/tie/uniqueness metrics differ from pinned source profile");
    }
    if summary.maximum_gap_ns != Some(5_369_937_500_000_i128)
        || summary.maximum_gap_start_timestamp_unix_ns != Some(1_486_053_428_906_250_000_i128)
        || summary.maximum_gap_end_timestamp_unix_ns != Some(1_486_058_798_843_750_000_i128)
    {
        return Err("maximum observed-event gap differs from pinned source profile");
    }

    const EXPECTED_FIELDS: [(&str, u64); 11] = [
        ("Block_Number", 3_875),
        ("Current_Tool", 8),
        ("Feed_Override", 43),
        ("Feed_Rate", 5_657),
        ("Program_Runtime_Seconds", 1_781),
        ("Program_Status", 2),
        ("Part_Count", 1),
        ("Rapid_Override", 76),
        ("Spindle_Override", 82),
        ("Spindle_Speed", 189),
        ("Spindle_Time", 5),
    ];
    for (field, expected_count) in EXPECTED_FIELDS {
        if summary.field_counts.get(field) != Some(&expected_count) {
            return Err("one or more field counts differ from pinned source profile");
        }
    }
    Ok(())
}
