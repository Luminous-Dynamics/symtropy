// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::{
    FaultCampaignManifest, ScenarioManifest, campaign::run_fault_campaign,
};

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
    let result = match run_fault_campaign(&scenario, &campaign) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("protocol campaign execution failed: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&result) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("cannot encode protocol campaign report: {error}");
            return ExitCode::from(1);
        }
    }

    if result.all_expected_outcomes_observed
        && result.pre_dispatch_faults_never_dispatched
        && result.post_dispatch_ambiguities_blocked_retry
        && result.no_episode_false_completed
    {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "campaign rejected: expected_outcomes={}, pre_dispatch_safe={}, post_dispatch_retry_blocked={}, no_false_completion={}",
            result.all_expected_outcomes_observed,
            result.pre_dispatch_faults_never_dispatched,
            result.post_dispatch_ambiguities_blocked_retry,
            result.no_episode_false_completed
        );
        ExitCode::from(1)
    }
}
