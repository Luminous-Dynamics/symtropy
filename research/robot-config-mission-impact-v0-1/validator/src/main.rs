// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

fn main() {
    match robot_mission_impact_package_check::validate_package(
        &robot_mission_impact_package_check::repository_root(),
    ) {
        Ok(()) => {
            println!("PASS: schemas, development fixtures and cross-file invariants are valid");
        }
        Err(errors) => {
            eprintln!("FAIL: mission-impact development package validation");
            for error in errors {
                eprintln!("{error}");
            }
            std::process::exit(1);
        }
    }
}
