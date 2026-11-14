// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use habitat_revalidation_scale::{generate_workload, Receipt};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut args = env::args_os();
    let _program = args.next();
    let supplied: Vec<_> = args.collect();
    if supplied.len() != 5 {
        eprintln!(
            "usage: habitat-revalidation-scale <matrix.json> <profile-id> \
             <workload.json> <receipt.json> <source-commit-sha>"
        );
        return ExitCode::from(2);
    }
    let matrix_path = &supplied[0];
    let profile_id = supplied[1].to_string_lossy();
    let workload_path = &supplied[2];
    let receipt_path = &supplied[3];
    let source_commit = supplied[4].to_string_lossy();
    if matrix_path == workload_path
        || matrix_path == receipt_path
        || workload_path == receipt_path
    {
        eprintln!("error: matrix, workload, and receipt paths must be distinct");
        return ExitCode::from(2);
    }

    let matrix_bytes = match fs::read(matrix_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: could not read profile matrix: {error}");
            return ExitCode::from(2);
        }
    };
    let generated = match generate_workload(&matrix_bytes, &profile_id, &source_commit) {
        Ok(result) => result,
        Err(error) => {
            eprintln!("error: workload generation rejected: {error}");
            return ExitCode::from(3);
        }
    };
    let mut receipt_bytes = match serde_json::to_vec_pretty(&generated.receipt) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: could not serialize workload receipt: {error}");
            return ExitCode::from(4);
        }
    };
    receipt_bytes.push(b'\n');

    if let Err(error) = fs::write(workload_path, &generated.workload_bytes) {
        eprintln!("error: could not write generated workload: {error}");
        return ExitCode::from(5);
    }
    if let Err(error) = fs::write(receipt_path, &receipt_bytes) {
        eprintln!("error: could not write generation receipt: {error}");
        return ExitCode::from(5);
    }

    // Re-parse the emitted receipt through its strict schema before announcing success.
    let checked_receipt: Receipt = match serde_json::from_slice(&receipt_bytes) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("error: generated receipt failed its own schema: {error}");
            return ExitCode::from(6);
        }
    };
    println!(
        "workload={} artifacts={} tasks={} reachable_tasks={} sha256={}",
        checked_receipt.profile_id,
        checked_receipt.artifact_count,
        checked_receipt.task_count,
        checked_receipt.reachable_task_count,
        checked_receipt.exact_input_sha256
    );
    ExitCode::SUCCESS
}
