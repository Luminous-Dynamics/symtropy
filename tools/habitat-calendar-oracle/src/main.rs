// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use habitat_calendar_oracle::evaluate_fixture_bytes;
use std::env;
use std::fs;
use std::process;

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "docs/research/fixtures/HABITAT_CALENDAR_DISRUPTION_V0_1.json".to_owned());
    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read {path}: {error}");
            process::exit(2);
        }
    };

    match evaluate_fixture_bytes(&bytes) {
        Ok(report) => {
            let all_expected = report.all_expected_outcomes_match;
            match serde_json::to_string_pretty(&report) {
                Ok(json) => println!("{json}"),
                Err(error) => {
                    eprintln!("could not serialize oracle report: {error}");
                    process::exit(2);
                }
            }
            if !all_expected {
                eprintln!("one or more calendar reference outcomes did not match");
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}
