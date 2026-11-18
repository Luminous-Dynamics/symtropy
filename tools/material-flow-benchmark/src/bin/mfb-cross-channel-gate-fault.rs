// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::{
    ScenarioManifest, cross_channel::run_gate_fault_observation_cross_check,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <scenario.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one scenario path");
        return ExitCode::from(2);
    }

    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read scenario: {error}");
            return ExitCode::from(2);
        }
    };
    let scenario: ScenarioManifest = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid scenario JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let result = match run_gate_fault_observation_cross_check(&scenario) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("cross-channel fault evaluation rejected: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&result) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("cannot encode cross-channel fault report: {error}");
            return ExitCode::from(1);
        }
    }

    if result.fault_oracle_passed {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "cross-channel oracle failed: disagreement={}, reconciliation={}, retry_blocked={}, custody_blocked={}, false_completion_blocked={}",
            result.physical_and_observation_disagreement_detected,
            result.reconciliation_required,
            result.blind_retry_blocked,
            result.custody_completion_blocked,
            result.false_completion_blocked
        );
        ExitCode::from(1)
    }
}
