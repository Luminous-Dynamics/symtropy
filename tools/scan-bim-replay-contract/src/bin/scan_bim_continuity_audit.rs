// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Audit one representation-continuity receipt and emit a machine-readable report.

use scan_bim_replay_contract::continuity::audit_representation_continuity_bytes;
use std::env;
use std::fs;
use std::io::{self, Write};
use std::process::ExitCode;

fn run() -> Result<(), String> {
    let mut arguments = env::args_os().skip(1);
    let receipt_path = arguments
        .next()
        .ok_or_else(|| "expected <representation-continuity-receipt.json>".to_owned())?;
    if arguments.next().is_some() {
        return Err("expected exactly one receipt path".to_owned());
    }

    let bytes = fs::read(&receipt_path).map_err(|error| {
        format!(
            "cannot read {}: {error}",
            std::path::PathBuf::from(&receipt_path).display()
        )
    })?;
    let report = audit_representation_continuity_bytes(&bytes)?;
    let output = serde_json::to_vec(&report)
        .map_err(|error| format!("cannot serialize continuity report: {error}"))?;
    let stdout = io::stdout();
    let mut writer = stdout.lock();
    writer
        .write_all(&output)
        .and_then(|_| writer.write_all(b"\n"))
        .map_err(|error| format!("cannot write continuity report: {error}"))?;
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("scan-bim-continuity-audit: {error}");
            ExitCode::FAILURE
        }
    }
}
