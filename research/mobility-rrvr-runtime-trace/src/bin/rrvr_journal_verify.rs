// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Read-only verifier for an existing RRVR durable journal.

use mobility_rrvr_runtime_trace::durable_journal::recovery_projection_file;
use serde_json::json;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args().skip(1);
    let Some(journal_path) = args.next() else {
        eprintln!("usage: rrvr_journal_verify <journal.jsonl> [report.json]");
        return ExitCode::from(2);
    };
    let report_path = args.next();
    if args.next().is_some() {
        eprintln!("usage: rrvr_journal_verify <journal.jsonl> [report.json]");
        return ExitCode::from(2);
    }

    let projection = match recovery_projection_file(&journal_path) {
        Ok(summary) => summary,
        Err(error) => {
            eprintln!("journal validation failed: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = json!({
        "validation": "passed",
        "journal_path": journal_path,
        "recovery_projection": projection,
        "read_only": true,
        "transport_calls_performed": false,
        "native_authority_created": false,
        "physical_safety_established": false,
        "owner_authentication_performed": false
    });
    let bytes = match serde_json::to_vec_pretty(&report) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("failed to serialize validation report: {error}");
            return ExitCode::FAILURE;
        }
    };
    if let Some(path) = report_path
        && let Err(error) = fs::write(path, &bytes)
    {
        eprintln!("failed to write validation report: {error}");
        return ExitCode::FAILURE;
    }
    println!("{}", String::from_utf8_lossy(&bytes));
    ExitCode::SUCCESS
}
