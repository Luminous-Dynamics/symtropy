// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use mobility_rrvr_command_semantics::fixture::evaluate_fixture_corpus;

fn main() -> ExitCode {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: rrvr_command_semantics_check <fixture.json>");
        return ExitCode::from(2);
    };

    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read {path}: {error}");
            return ExitCode::from(2);
        }
    };

    let report = match evaluate_fixture_corpus(&input) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("fixture corpus invalid: {error}");
            return ExitCode::from(2);
        }
    };

    for failure in &report.failures {
        eprintln!(
            "FAIL {} [{}]: expected {}, got {}",
            failure.case_id, failure.mutation, failure.expected, failure.actual
        );
    }

    println!(
        "RRVR_COMMAND_SEMANTICS total={} passed={} failed={}",
        report.total,
        report.passed,
        report.failures.len()
    );

    if report.is_success() {
        println!("command_semantics_fixture_validation=passed");
        ExitCode::SUCCESS
    } else {
        println!("command_semantics_fixture_validation=failed");
        ExitCode::FAILURE
    }
}
