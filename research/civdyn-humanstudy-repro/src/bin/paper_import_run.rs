// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Assemble a complete source-pinned HumanStudy-Bench paper-12 run receipt.
//!
//! Usage:
//! cargo run --locked --manifest-path research/civdyn-humanstudy-repro/Cargo.toml \
//!   --bin paper-import-run -- <receipt.json|-> <study_001_run_dir> ... <study_012_run_dir>

use civdyn_humanstudy_repro::{
    assemble_paper_run_receipt, import_paper_study_run, paper_receipt_to_json,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn usage() {
    eprintln!(
        "Usage: paper-import-run <receipt.json|-> <study_001_run_dir> ... <study_012_run_dir>\n\
         Each study directory must contain full_benchmark.json, detailed_stats.csv,\n\
         evaluation_results.json, and civdyn_paper_run_attestation.json."
    );
}

fn run() -> Result<(), String> {
    let args: Vec<_> = env::args_os().collect();
    if args.len() != 14 {
        usage();
        return Err(format!(
            "expected one output target and exactly 12 study run directories, received {} argument(s)",
            args.len().saturating_sub(1)
        ));
    }

    let output = args[1].to_string_lossy().into_owned();
    let mut imports = Vec::with_capacity(12);
    for path in &args[2..] {
        let directory = PathBuf::from(path);
        let imported = import_paper_study_run(&directory)
            .map_err(|error| format!("{}: {error}", directory.display()))?;
        eprintln!(
            "imported {}: participants={}, trials={}, tests={}, runs={}, PAS={:.6}, ECS={}",
            imported.receipt.study_id,
            imported.receipt.participant_count,
            imported.receipt.trial_count,
            imported.test_count,
            imported.run_count,
            imported.receipt.official_pas.unwrap_or_default(),
            imported
                .receipt
                .official_ecs
                .map(|value| format!("{value:.6}"))
                .unwrap_or_else(|| "unavailable".to_string())
        );
        imports.push(imported);
    }

    let receipt = assemble_paper_run_receipt(imports)
        .map_err(|error| format!("paper-12 assembly failed: {error}"))?;
    let json = paper_receipt_to_json(&receipt);
    let serialized = serde_json::to_vec_pretty(&json)
        .map_err(|error| format!("cannot serialize assembled receipt: {error}"))?;

    if output == "-" {
        println!(
            "{}",
            String::from_utf8(serialized)
                .map_err(|error| format!("serialized receipt is not UTF-8: {error}"))?
        );
    } else {
        fs::write(&output, serialized)
            .map_err(|error| format!("cannot write {}: {error}", output))?;
        eprintln!("wrote source-pinned 12-study receipt to {output}");
    }
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("paper-import-run: {error}");
            ExitCode::FAILURE
        }
    }
}
