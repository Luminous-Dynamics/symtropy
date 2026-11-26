// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use mobility_rrvr_fault_harness::{report_json, run_campaign};

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if !arguments.is_empty() && arguments.as_slice() != ["--known-campaign"] {
        eprintln!("usage: rrvr_fault_harness [--known-campaign]");
        return ExitCode::from(2);
    }
    let report = run_campaign();
    let output = report_json(&report);
    if arguments.as_slice() == ["--known-campaign"] {
        println!("{output}");
        return ExitCode::SUCCESS;
    }
    println!("{output}");
    ExitCode::SUCCESS
}
