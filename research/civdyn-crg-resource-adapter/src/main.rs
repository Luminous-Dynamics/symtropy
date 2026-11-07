// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use civdyn_crg_resource_adapter::{render_csv, run_standard_matrix};

fn main() {
    match run_standard_matrix() {
        Ok(episodes) => print!("{}", render_csv(&episodes)),
        Err(error) => {
            eprintln!("CRG/resource bridge failed closed: {error}");
            std::process::exit(1);
        }
    }
}
