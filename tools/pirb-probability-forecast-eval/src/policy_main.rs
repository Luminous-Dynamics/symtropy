// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use pirb_probability_forecast_eval::policy_eval::evaluate_predeclared_policy;
use std::{env, fs, path::PathBuf, process};

fn option_value(args: &[String], name: &str) -> Result<Option<String>, String> {
    let Some(index) = args.iter().position(|arg| arg == name) else {
        return Ok(None);
    };
    args.get(index + 1)
        .filter(|value| !value.starts_with("--"))
        .cloned()
        .map(Some)
        .ok_or_else(|| format!("argument {name} requires a value"))
}

fn required_value(args: &[String], name: &str) -> Result<String, String> {
    option_value(args, name)?.ok_or_else(|| format!("required argument {name} is missing"))
}

fn read_path(path: &str, label: &str) -> Result<Vec<u8>, String> {
    fs::read(path).map_err(|error| format!("cannot read {label} {path}: {error}"))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let dataset_path = required_value(&args, "--dataset")?;
    let policy_path = required_value(&args, "--policy")?;
    let receipt_path = required_value(&args, "--receipt")?;
    let dataset = read_path(&dataset_path, "normalized dataset")?;
    let policy = read_path(&policy_path, "policy manifest")?;
    let source_commit = option_value(&args, "--source-commit")?
        .unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let rustc_version = option_value(&args, "--rustc-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());
    let cargo_version = option_value(&args, "--cargo-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let lock = fs::read(&lock_path)
        .map_err(|error| format!("cannot read generated dependency lock: {error}"))?;
    let receipt = evaluate_predeclared_policy(
        &dataset,
        &policy,
        &source_commit,
        &rustc_version,
        &cargo_version,
        &lock,
    )
    .map_err(|errors| errors.join("\n"))?;
    let mut output = serde_json::to_vec_pretty(&receipt)
        .map_err(|error| format!("cannot serialize policy receipt: {error}"))?;
    output.push(b'\n');
    fs::write(&receipt_path, output)
        .map_err(|error| format!("cannot write policy receipt {receipt_path}: {error}"))?;
    println!(
        "Evaluated {} predeclared thresholds; result={}",
        receipt.thresholds_evaluated.len(),
        receipt.result
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
