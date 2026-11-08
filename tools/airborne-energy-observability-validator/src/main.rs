// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline CLI for the synthetic receiver-pose observability verifier.

use airborne_energy_observability_validator::validate_json;
use std::{env, fs, process};

fn main() {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "airborne-energy-observability-validator".to_string());
    let Some(path) = args.next() else {
        eprintln!("usage: {program} <observability-case.json>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: {program} <observability-case.json>");
        process::exit(2);
    }
    let contents = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("cannot read observability case: {error}");
            process::exit(2);
        }
    };
    match validate_json(&contents) {
        Ok(report) => {
            println!(
                "PASS_SYNTHETIC_OBSERVABILITY case={} disposition={} poses={} in_domain={} out_of_domain={} uniquely_observable={} ambiguous={} alias_pairs={}",
                report.case_id,
                report.disposition,
                report.total_pose_samples,
                report.in_domain_pose_samples,
                report.out_of_domain_pose_samples,
                report.uniquely_observable_pose_samples,
                report.ambiguous_pose_samples,
                report.alias_pairs.len()
            );
        }
        Err(error) => {
            eprintln!("REJECTED_OBSERVABILITY {error}");
            process::exit(1);
        }
    }
}
