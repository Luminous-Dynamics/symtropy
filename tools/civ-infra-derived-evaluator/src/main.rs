// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use civ_infra_derived_evaluator::evaluate_json;
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let mut args = env::args_os().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: civ-infra-derived-evaluator <fixture.json>");
        return ExitCode::FAILURE;
    };
    if args.next().is_some() {
        eprintln!("expected exactly one fixture path");
        return ExitCode::FAILURE;
    }

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read fixture: {error}");
            return ExitCode::FAILURE;
        }
    };
    let report = match evaluate_json(&bytes) {
        Ok(report) => report,
        Err(errors) => {
            for error in errors {
                eprintln!("{error}");
            }
            return ExitCode::FAILURE;
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot serialize report: {error}");
            ExitCode::FAILURE
        }
    }
}
