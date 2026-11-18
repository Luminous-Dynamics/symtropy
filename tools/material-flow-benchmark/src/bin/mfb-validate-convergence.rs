// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::process::ExitCode;

use symtropy_material_flow_benchmark::convergence::run_free_fall_convergence;

fn main() -> ExitCode {
    let report = match run_free_fall_convergence() {
        Ok(report) => report,
        Err(error) => {
            eprintln!("convergence campaign failed: {error}");
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => println!("{json}"),
        Err(error) => {
            eprintln!("cannot encode convergence report: {error}");
            return ExitCode::from(1);
        }
    }
    if report.position_error_monotonic && report.all_cases_numerically_healthy {
        ExitCode::SUCCESS
    } else {
        eprintln!(
            "convergence rejected: monotonic_position_error={}, numerically_healthy={}, estimated_order={:?}",
            report.position_error_monotonic,
            report.all_cases_numerically_healthy,
            report.estimated_position_order
        );
        ExitCode::from(1)
    }
}
