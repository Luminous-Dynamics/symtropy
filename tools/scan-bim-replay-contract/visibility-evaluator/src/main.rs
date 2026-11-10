// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use scan_bim_visibility_evaluator::visibility::VisibilityRequest;
use std::io::{self, Read, Write};
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut input = Vec::new();
    if let Err(error) = io::stdin().read_to_end(&mut input) {
        eprintln!("visibility evaluator: failed to read request: {error}");
        return ExitCode::FAILURE;
    }

    let request: VisibilityRequest = match serde_json::from_slice(&input) {
        Ok(request) => request,
        Err(error) => {
            eprintln!("visibility evaluator: invalid request: {error}");
            return ExitCode::FAILURE;
        }
    };

    let receipt = match request.evaluate() {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("visibility evaluator: request rejected: {error}");
            return ExitCode::FAILURE;
        }
    };

    let output = match serde_json::to_vec(&receipt) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("visibility evaluator: failed to serialize receipt: {error}");
            return ExitCode::FAILURE;
        }
    };

    let stdout = io::stdout();
    let mut writer = stdout.lock();
    if let Err(error) = writer.write_all(&output).and_then(|_| writer.write_all(b"\n")) {
        eprintln!("visibility evaluator: failed to write receipt: {error}");
        return ExitCode::FAILURE;
    }
    ExitCode::SUCCESS
}
