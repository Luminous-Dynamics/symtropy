// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::env;
use std::fs;
use std::process::ExitCode;

use av_decision_sufficient_observation_v0::audit_fixtures;
use serde_json::Value;

fn read_json(path: &str) -> Result<Value, String> {
    let bytes = fs::read(path).map_err(|error| format!("cannot read {path:?}: {error}"))?;
    serde_json::from_slice(&bytes).map_err(|error| format!("invalid JSON in {path:?}: {error}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 3 {
        return Err(
            "usage: av-dso-fixture-audit <agent-inputs.json> <evaluator-oracle.json> <expected-scorecard.json>"
                .to_owned(),
        );
    }

    let agent = read_json(&args[0])?;
    let oracle = read_json(&args[1])?;
    let expected = read_json(&args[2])?;
    let report = audit_fixtures(&agent, &oracle, &expected)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize audit report: {error}"))?
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
