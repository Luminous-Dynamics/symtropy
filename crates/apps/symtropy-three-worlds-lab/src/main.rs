// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

fn main() {
    match symtropy_three_worlds_lab::run_three_worlds() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => println!("{json}"),
            Err(error) => {
                eprintln!("Three Worlds report serialization failed: {error}");
                std::process::exit(1);
            }
        },
        Err(error) => {
            eprintln!("Three Worlds lab failed: {error}");
            std::process::exit(1);
        }
    }
}
