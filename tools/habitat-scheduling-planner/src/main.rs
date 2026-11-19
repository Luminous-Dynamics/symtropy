// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use habitat_scheduling_planner::plan_fixture_bytes;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args_os();
    let _program = args.next();
    let Some(fixture_path) = args.next() else {
        eprintln!("usage: habitat-scheduling-planner <micro-oracle-fixture.json>");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("error: expected exactly one fixture path");
        return ExitCode::from(2);
    }
    let bytes = match fs::read(&fixture_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: could not read schedule fixture: {error}");
            return ExitCode::from(2);
        }
    };
    let report = match plan_fixture_bytes(&bytes) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("error: scheduling input rejected: {error}");
            return ExitCode::from(3);
        }
    };
    let json = match serde_json::to_string_pretty(&report) {
        Ok(json) => json,
        Err(error) => {
            eprintln!("error: could not serialize schedule report: {error}");
            return ExitCode::from(4);
        }
    };
    println!("{json}");
    if report.all_returned_schedules_independently_validated {
        ExitCode::SUCCESS
    } else {
        eprintln!("error: at least one emitted candidate failed independent validation");
        ExitCode::from(5)
    }
}
