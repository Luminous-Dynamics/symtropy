// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use pirb_probability_forecast_eval::compare_outcome_vintages;
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

fn repeated_values(args: &[String], name: &str) -> Result<Vec<String>, String> {
    let mut values = Vec::new();
    let mut index = 0;
    while index < args.len() {
        if args[index] == name {
            let Some(value) = args.get(index + 1) else {
                return Err(format!("argument {name} requires a value"));
            };
            if value.starts_with("--") {
                return Err(format!(
                    "argument {name} requires a file path before the next flag"
                ));
            }
            values.push(value.clone());
            index += 2;
        } else {
            index += 1;
        }
    }
    if !(2..=20).contains(&values.len()) {
        return Err(
            "provide between 2 and 20 frozen datasets with repeated --input PATH arguments"
                .to_owned(),
        );
    }
    Ok(values)
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let input_paths = repeated_values(&args, "--input")?;
    let mut input_bytes = Vec::with_capacity(input_paths.len());
    for path in &input_paths {
        input_bytes
            .push(fs::read(path).map_err(|error| format!("cannot read input {path}: {error}"))?);
    }
    let input_refs: Vec<&[u8]> = input_bytes.iter().map(Vec::as_slice).collect();
    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let rustc_version = option_value(&args, "--rustc-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());
    let cargo_version = option_value(&args, "--cargo-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("cannot read generated dependency lock: {error}"))?;

    let receipt = compare_outcome_vintages(
        &input_refs,
        &source_commit,
        &rustc_version,
        &cargo_version,
        &lock_bytes,
    )
    .map_err(|errors| errors.join("\n"))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&receipt).map_err(|error| format!(
            "cannot serialize outcome-vintage sensitivity receipt: {error}"
        ))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
