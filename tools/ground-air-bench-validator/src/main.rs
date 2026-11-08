// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Offline semantic evaluator for GroundAirBench episode manifests.

use ground_air_bench_validator::evaluate_json;
use std::{env, fs, process};

fn main() {
    let mut args = env::args_os();
    let program = args
        .next()
        .and_then(|value| value.into_string().ok())
        .unwrap_or_else(|| "ground-air-bench-validator".to_owned());
    let Some(path) = args.next() else {
        eprintln!("usage: {program} <ground-air-episode.json>");
        process::exit(2);
    };
    if args.next().is_some() {
        eprintln!("usage: {program} <ground-air-episode.json>");
        process::exit(2);
    }

    let input = match fs::read_to_string(&path) {
        Ok(contents) => contents,
        Err(error) => {
            eprintln!("cannot read episode manifest: {error}");
            process::exit(2);
        }
    };

    match evaluate_json(&input) {
        Ok(report) => match serde_json::to_string_pretty(&report.as_json()) {
            Ok(output) => println!("{output}"),
            Err(error) => {
                eprintln!("cannot serialize evaluation report: {error}");
                process::exit(2);
            }
        },
        Err(error) => {
            eprintln!("REJECTED_GROUNDAIR_EPISODE {error}");
            process::exit(1);
        }
    }
}
