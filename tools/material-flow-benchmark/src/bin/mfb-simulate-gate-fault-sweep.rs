// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::{
    ScenarioManifest, simulation::simulate_closed_gate_obstruction_sweep,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(scenario_path) = args.next() else {
        eprintln!(
            "usage: {} <scenario.json> [run-count]",
            program.to_string_lossy()
        );
        return ExitCode::from(2);
    };
    let run_count = match args.next() {
        Some(value) => match value.to_string_lossy().parse::<usize>() {
            Ok(value) => value,
            Err(error) => {
                eprintln!("invalid run-count: {error}");
                return ExitCode::from(2);
            }
        },
        None => 30,
    };
    if args.next().is_some() {
        eprintln!("expected a scenario path and optional run-count");
        return ExitCode::from(2);
    }

    let scenario_path = scenario_path.to_string_lossy();
    let raw = match fs::read_to_string(scenario_path.as_ref()) {
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
    let result = match simulate_closed_gate_obstruction_sweep(&scenario, run_count) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("gate-fault sweep rejected: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&result) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("cannot encode gate-fault sweep report: {error}");
            return ExitCode::from(1);
        }
    }

    if result.all_fault_oracles_passed
        && result.executed_run_count == result.requested_run_count
        && result.failed_run_count == 0
    {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "gate-fault sweep failed: requested={}, executed={}, passed={}, failed={}",
            result.requested_run_count,
            result.executed_run_count,
            result.passed_run_count,
            result.failed_run_count
        );
        ExitCode::from(1)
    }
}
