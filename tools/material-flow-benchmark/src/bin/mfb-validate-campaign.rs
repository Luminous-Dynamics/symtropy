// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};
use symtropy_material_flow_benchmark::{FaultCampaignManifest, ScenarioManifest};

fn read_json(path: &str) -> Result<String, ExitCode> {
    fs::read_to_string(path).map_err(|error| {
        eprintln!("cannot read {path}: {error}");
        ExitCode::from(2)
    })
}

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(scenario_path) = args.next() else {
        eprintln!(
            "usage: {} <scenario.json> <campaign.json>",
            program.to_string_lossy()
        );
        return ExitCode::from(2);
    };
    let Some(campaign_path) = args.next() else {
        eprintln!("expected both scenario and campaign paths");
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly two paths: scenario and campaign");
        return ExitCode::from(2);
    }

    let scenario_path = scenario_path.to_string_lossy();
    let campaign_path = campaign_path.to_string_lossy();
    let scenario_raw = match read_json(scenario_path.as_ref()) {
        Ok(raw) => raw,
        Err(code) => return code,
    };
    let campaign_raw = match read_json(campaign_path.as_ref()) {
        Ok(raw) => raw,
        Err(code) => return code,
    };

    let scenario: ScenarioManifest = match serde_json::from_str(&scenario_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid scenario JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let campaign: FaultCampaignManifest = match serde_json::from_str(&campaign_raw) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid campaign JSON: {error}");
            return ExitCode::from(1);
        }
    };

    if let Err(errors) = scenario.validate() {
        for error in errors {
            eprintln!("INVALID SCENARIO: {error}");
        }
        return ExitCode::from(1);
    }

    match campaign.validate_against(&scenario) {
        Ok(()) => {
            println!(
                "VALID CAMPAIGN {} (schema {}, episodes {}, base {})",
                campaign.campaign_id,
                campaign.schema_version,
                campaign.episodes.len(),
                campaign.base_scenario_id
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for error in errors {
                eprintln!("INVALID CAMPAIGN: {error}");
            }
            ExitCode::from(1)
        }
    }
}
