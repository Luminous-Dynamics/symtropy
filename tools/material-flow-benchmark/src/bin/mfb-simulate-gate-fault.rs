// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::{
    ScenarioManifest, simulation::simulate_closed_gate_obstruction,
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
    let result = match simulate_closed_gate_obstruction(&scenario) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("closed-gate fault simulation rejected: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&result) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("cannot encode fault simulation report: {error}");
            return ExitCode::from(1);
        }
    }

    if result.fault_oracle_passed {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "closed-gate fault oracle failed: effect_observed={}, reconciliation={}, retry_blocked={}, false_completion_blocked={}",
            result.physical_fault_effect_observed,
            result.reconciliation_required,
            result.blind_retry_blocked,
            result.false_completion_blocked
        );
        ExitCode::from(1)
    }
}
