// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline command-line entry point for the aerial energy-session evidence verifier.
//! A successful synthetic report is not a hardware permit or physical safety claim.

use airborne_energy_session_validator::validate_json;
use std::{env, fs, process};

fn main() {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "airborne-energy-session-validator".to_string());
    let Some(path) = args.next() else {
        eprintln!("usage: {program} <session.json>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: {program} <session.json>");
        process::exit(2);
    }

    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("cannot read session document: {error}");
            process::exit(2);
        }
    };
    match validate_json(&contents) {
        Ok(report) => {
            println!(
                "PASS_SYNTHETIC_SESSION episode={} final_state={} events={} power_intervals={} source_wh={} useful_battery_wh={} declared_loss_wh={}",
                report.episode_id,
                report.final_state,
                report.event_count,
                report.power_active_intervals,
                report.source_energy_wh,
                report.useful_battery_energy_wh,
                report.total_declared_loss_wh
            );
        }
        Err(error) => {
            eprintln!("REJECTED_SESSION {error}");
            process::exit(1);
        }
    }
}
