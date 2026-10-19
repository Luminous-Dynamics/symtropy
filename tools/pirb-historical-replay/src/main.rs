// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use pirb_historical_replay::evaluate;
use std::{env, fs, path::PathBuf, process};

fn option_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|argument| argument == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("argument {name} requires a value"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input_path = option_value(&args, "--input")?
        .ok_or_else(|| "usage: pirb-historical-replay --input DATASET.json [--source-commit SHA] [--rustc-version VERSION] [--cargo-version VERSION]".to_owned())?;
    let input = fs::read(&input_path)
        .map_err(|error| format!("cannot read input {}: {error}", input_path))?;
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let rustc_version = option_value(&args, "--rustc-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());
    let cargo_version = option_value(&args, "--cargo-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());

    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("cannot read generated dependency lock: {error}"))?;

    let receipt = evaluate(
        &input,
        &source_commit,
        &rustc_version,
        &cargo_version,
        &lock_bytes,
    )
    .map_err(|findings| findings.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt)
            .map_err(|error| format!("cannot serialize replay receipt: {error}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
