// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use habitat_calendar_oracle::evaluate_fixture_bytes;
use habitat_calendar_oracle::external_audit::audit_embedded_external_corpus;
use habitat_calendar_oracle::provenance::evaluate_fixture_bytes
    as evaluate_provenance_fixture_bytes;
use std::env;
use std::fs;
use std::process;

fn write_json<T: serde::Serialize>(value: &T, label: &str) {
    match serde_json::to_string_pretty(value) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("could not serialize {label}: {error}");
            process::exit(2);
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.first().is_some_and(|arg| arg == "--external-audit") {
        match audit_embedded_external_corpus() {
            Ok(report) => write_json(&report, "external CA-RCPSP audit report"),
            Err(error) => {
                eprintln!("{error}");
                process::exit(2);
            }
        }
        return;
    }

    let provenance_mode = args.first().is_some_and(|arg| arg == "--provenance");
    let path = if provenance_mode {
        args.get(1)
            .cloned()
            .unwrap_or_else(|| {
                "docs/research/fixtures/HABITAT_CALENDAR_PROVENANCE_V0_1.json".to_owned()
            })
    } else {
        args.first().cloned().unwrap_or_else(|| {
            "docs/research/fixtures/HABITAT_CALENDAR_DISRUPTION_V0_1.json".to_owned()
        })
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("could not read {path}: {error}");
            process::exit(2);
        }
    };

    if provenance_mode {
        match evaluate_provenance_fixture_bytes(&bytes) {
            Ok(report) => {
                let all_expected = report.all_expected_statuses_match;
                write_json(&report, "calendar provenance report");
                if !all_expected {
                    eprintln!("one or more calendar provenance expectations did not match");
                    process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("{error}");
                process::exit(2);
            }
        }
    } else {
        match evaluate_fixture_bytes(&bytes) {
            Ok(report) => {
                let all_expected = report.all_expected_outcomes_match;
                write_json(&report, "calendar oracle report");
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
}
