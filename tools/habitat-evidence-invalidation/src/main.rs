// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use habitat_evidence_invalidation::verify_fixture_bytes;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args_os();
    let _program = args.next();
    let Some(path) = args.next() else {
        eprintln!("usage: habitat-evidence-invalidation <fixture.json>");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("error: expected exactly one fixture path");
        return ExitCode::from(2);
    }
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: could not read {:?}: {error}", path);
            return ExitCode::from(2);
        }
    };
    let report = match verify_fixture_bytes(&bytes) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors { eprintln!("evidence fixture rejected: {error}"); }
            return ExitCode::from(3);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => { println!("{json}"); ExitCode::SUCCESS }
        Err(error) => {
            eprintln!("error: could not serialize report: {error}");
            ExitCode::from(4)
        }
    }
}
