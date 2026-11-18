// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};
use symtropy_material_flow_benchmark::ScenarioManifest;

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
        Ok(scenario) => scenario,
        Err(error) => {
            eprintln!("invalid scenario JSON: {error}");
            return ExitCode::from(1);
        }
    };
    match scenario.validate() {
        Ok(()) => {
            println!(
                "VALID {} (schema {}, evidence {:?}, seed {})",
                scenario.scenario_id,
                scenario.schema_version,
                scenario.evidence_level,
                scenario.seed
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors {
                eprintln!("INVALID: {error}");
            }
            ExitCode::from(1)
        }
    }
}
