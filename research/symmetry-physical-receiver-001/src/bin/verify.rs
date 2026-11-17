// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::error::Error;
use std::path::PathBuf;

use symmetry_physical_handoff_receiver::{ReceiverPackageV1, ReceiverProfileV1, verify_package};

fn main() {
    if let Err(error) = run() {
        eprintln!("symmetry handoff verification failed: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), Box<dyn Error>> {
    let path = std::env::args_os()
        .nth(1)
        .map(PathBuf::from)
        .ok_or("usage: verify-symmetry-handoff <receiver-package.json>")?;
    let bytes = std::fs::read(&path)?;
    let package: ReceiverPackageV1 = serde_json::from_slice(&bytes)?;
    let receipt = verify_package(&package, ReceiverProfileV1::default())?;
    println!("{}", serde_json::to_string_pretty(&receipt)?);
    Ok(())
}
