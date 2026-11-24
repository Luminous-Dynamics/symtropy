// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::service_decision_impact::{
    ServiceDecisionImpactInput, evaluate_service_decision_impact,
};

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 1 {
        eprintln!("usage: service_decision_impact_screen <decision-impact.json>");
        return ExitCode::from(2);
    }

    let raw = match fs::read_to_string(&args[0]) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read decision-impact input: {error}");
            return ExitCode::from(2);
        }
    };
    let input: ServiceDecisionImpactInput = match serde_json::from_str(&raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid decision-impact JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_service_decision_impact(&input) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("decision-impact input rejected:");
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
            eprintln!("cannot encode decision-impact report: {error}");
            ExitCode::from(1)
        }
    }
}
