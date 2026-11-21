// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::published_summary::{
    PublishedSummaryInput, verify_published_summary,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <published-summary.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one published-summary input path");
        return ExitCode::from(2);
    }

    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read published-summary input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: PublishedSummaryInput = match serde_json::from_str(&raw) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("invalid published-summary input JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match verify_published_summary(&input) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("published-summary input rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            if report.summary_arithmetic_consistent {
                ExitCode::SUCCESS
            } else {
                eprintln!("published summary arithmetic is internally inconsistent");
                ExitCode::from(1)
            }
        }
        Err(error) => {
            eprintln!("cannot encode published-summary report: {error}");
            ExitCode::from(1)
        }
    }
}
