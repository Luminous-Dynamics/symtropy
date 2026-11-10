// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use scan_bim_visibility_evaluator::visibility::VisibilityRequest;
use std::io::{self, Read, Write};
use std::process::ExitCode;

const MAX_REQUEST_BYTES: u64 = 64 * 1024 * 1024;

fn main() -> ExitCode {
    let stdin = io::stdin();
    let mut input = Vec::new();
    let mut limited_reader = stdin.lock().take(MAX_REQUEST_BYTES + 1);
    if let Err(error) = limited_reader.read_to_end(&mut input) {
        eprintln!("visibility evaluator: failed to read request: {error}");
        return ExitCode::FAILURE;
    }
    if input.len() as u64 > MAX_REQUEST_BYTES {
        eprintln!("visibility evaluator: request exceeds the 64 MiB input limit");
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
