// SPDX-License-Identifier: AGPL-3.0-or-later
use std::{env, fs, process::ExitCode};

use wpt_multifidelity_bench::BenchmarkSuite;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 1 {
        eprintln!("usage: wpt-multifidelity-bench <suite.json>");
        return ExitCode::from(2);
    }

    let source = match fs::read_to_string(&arguments[0]) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read benchmark suite: {error}");
            return ExitCode::FAILURE;
        }
    };
    let suite: BenchmarkSuite = match serde_json::from_str(&source) {
        Ok(suite) => suite,
        Err(error) => {
            eprintln!("invalid benchmark suite JSON: {error}");
            return ExitCode::FAILURE;
        }
    };
    match suite.evaluate() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot serialize benchmark report: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
