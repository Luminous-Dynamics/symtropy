// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use ev_passenger_cost_frontier::mobility_service_risk_envelope::{
    MobilityServiceRiskInput, evaluate_mobility_service_risk_envelope,
};
use ev_passenger_cost_frontier::nonanticipative_service_oracle::NonanticipativeServiceOracleInput;
use ev_passenger_cost_frontier::sustainable_mobility_service_capacity::SustainableServiceCapacityInput;
use ev_passenger_cost_frontier::sustainable_mobility_service_quality::LinkedServiceQualityInput;

fn read_json<T: serde::de::DeserializeOwned>(
    path: &std::ffi::OsStr,
    label: &str,
) -> Result<T, String> {
    let raw = fs::read_to_string(path).map_err(|error| format!("cannot read {label}: {error}"))?;
    serde_json::from_str(&raw).map_err(|error| format!("invalid {label} JSON: {error}"))
}

fn main() -> ExitCode {
    let args: Vec<_> = env::args_os().skip(1).collect();
    if args.len() != 4 {
        eprintln!(
            "usage: mobility_service_risk_envelope_screen <scenario-tree.json> <linked-service.json> <capacity.json> <probability-manifest.json>"
        );
        return ExitCode::from(2);
    }

    let tree: NonanticipativeServiceOracleInput = match read_json(&args[0], "scenario tree") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let service: LinkedServiceQualityInput = match read_json(&args[1], "linked service input") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let capacity: SustainableServiceCapacityInput = match read_json(&args[2], "capacity input") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };
    let risk: MobilityServiceRiskInput = match read_json(&args[3], "service risk manifest") {
        Ok(value) => value,
        Err(error) => {
            eprintln!("{error}");
            return ExitCode::from(1);
        }
    };

    let report = match evaluate_mobility_service_risk_envelope(&tree, &service, &capacity, &risk) {
        Ok(value) => value,
        Err(errors) => {
            eprintln!("mobility service risk evaluation rejected:");
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
            eprintln!("cannot encode mobility service risk report: {error}");
            ExitCode::from(1)
        }
    }
}
