// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use sanitation_t0_evaluator::evaluate_case;
use std::{env, fs, process};

fn main() {
    let mut args = env::args().skip(1);
    let Some(path) = args.next() else {
        eprintln!("usage: sanitation-t0 <case.json>");
        process::exit(64);
    };
    if args.next().is_some() {
        eprintln!("usage: sanitation-t0 <case.json>");
        process::exit(64);
    }

    let input = match fs::read_to_string(&path) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            process::exit(66);
        }
    };
    match evaluate_case(&input) {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report.to_json()).expect("report serializes"));
            if !report.oracle_matches {
                eprintln!("case disposition or reason codes differ from the frozen oracle");
                process::exit(2);
            }
        }
        Err(error) => {
            eprintln!("evaluation error: {error}");
            process::exit(65);
        }
    }
}
