// SPDX-License-Identifier: AGPL-3.0-or-later
use std::{env, fs, process::ExitCode};

use wpt_segment_energy_ledger::RouteLedger;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 1 {
        eprintln!("usage: wpt-segment-energy-ledger <route-ledger.json>");
        return ExitCode::from(2);
    }
    let source = match fs::read_to_string(&arguments[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("cannot read route ledger: {error}");
            return ExitCode::FAILURE;
        }
    };
    let ledger: RouteLedger = match serde_json::from_str(&source) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid route ledger JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    match ledger.evaluate() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot serialize route report: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
