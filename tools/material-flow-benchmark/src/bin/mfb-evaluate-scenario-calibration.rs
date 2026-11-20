// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::scenario_calibration::{
    ScenarioCalibrationCorpus, evaluate_scenario_calibration,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <scenario-calibration-corpus.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one scenario-calibration corpus path");
        return ExitCode::from(2);
    }
    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read scenario-calibration corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: ScenarioCalibrationCorpus = match serde_json::from_str(&raw) {
        Ok(corpus) => corpus,
        Err(error) => {
            eprintln!("invalid scenario-calibration JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_scenario_calibration(&corpus) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("scenario-calibration corpus rejected:");
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
            eprintln!("cannot encode scenario-calibration report: {error}");
            ExitCode::from(1)
        }
    }
}
