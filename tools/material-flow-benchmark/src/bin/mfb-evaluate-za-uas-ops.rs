// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::za_uas_ops_gate::{
    UasEligibilityBatch, evaluate_za_uas_eligibility,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <za-uas-eligibility-batch.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one South African UAS eligibility input path");
        return ExitCode::from(2);
    }

    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read South African UAS eligibility batch: {error}");
            return ExitCode::from(2);
        }
    };
    let batch: UasEligibilityBatch = match serde_json::from_str(&raw) {
        Ok(batch) => batch,
        Err(error) => {
            eprintln!("invalid South African UAS eligibility JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_za_uas_eligibility(&batch) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("South African UAS eligibility input rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot encode South African UAS eligibility report: {error}");
            ExitCode::from(1)
        }
    }
}
