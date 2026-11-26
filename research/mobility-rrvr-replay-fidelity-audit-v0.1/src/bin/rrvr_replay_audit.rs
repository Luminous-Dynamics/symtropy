// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode, path::Path};

use mobility_rrvr_replay_fidelity_audit::audit_json;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        eprintln!("usage: rrvr_replay_audit <manifest.json> <artifact-root>");
        return ExitCode::from(2);
    }
    let manifest = match fs::read_to_string(&arguments[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("MANIFEST_READ_FAILED: {error}");
            return ExitCode::FAILURE;
        }
    };
    match audit_json(&manifest, Path::new(&arguments[1])) {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(output) => {
                println!("{output}");
                if report.invalid_n > 0 || report.unresolved_n > 0 {
                    ExitCode::FAILURE
                } else {
                    ExitCode::SUCCESS
                }
            }
            Err(error) => {
                eprintln!("REPORT_SERIALIZATION_FAILED: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
