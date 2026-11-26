// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, path::Path, process::ExitCode};

use mobility_rrvr_replay_provenance::validate_provenance_json;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!(
            "usage: rrvr_replay_provenance <replay-manifest.json> <provenance-manifest.json> <artifact-root>"
        );
        return ExitCode::from(2);
    }
    let replay = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("REPLAY_MANIFEST_READ_FAILED: {error}");
            return ExitCode::FAILURE;
        }
    };
    let provenance = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("PROVENANCE_MANIFEST_READ_FAILED: {error}");
            return ExitCode::FAILURE;
        }
    };
    match validate_provenance_json(&replay, &provenance, Path::new(&args[2])) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(output) => {
                println!("{output}");
                if report.invalid_n == 0 && report.candidate_n == report.valid_n {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::FAILURE
                }
            }
            Err(error) => {
                eprintln!("PROVENANCE_REPORT_SERIALIZATION_FAILED: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("PROVENANCE_PREFLIGHT_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}
