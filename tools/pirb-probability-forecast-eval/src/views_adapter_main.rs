// Copyright (C) 2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use pirb_probability_forecast_eval::views_adapter::{
    import_views_archive, referenced_source_paths, ImportRuntime,
};
use std::{
    collections::BTreeMap,
    env, fs,
    path::{Path, PathBuf},
    process,
};

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

fn resolved_manifest_source(manifest_path: &Path, relative: &str) -> Result<PathBuf, String> {
    let parent = manifest_path
        .parent()
        .ok_or_else(|| "manifest path has no parent directory".to_owned())?;
    Ok(parent.join(relative))
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let manifest_path_text = required_value(&args, "--manifest")?;
    let outcomes_path = required_value(&args, "--outcomes")?;
    let block_map_path = required_value(&args, "--block-map")?;
    let output_path = required_value(&args, "--output")?;
    let receipt_path = required_value(&args, "--receipt")?;

    let manifest_path = PathBuf::from(&manifest_path_text);
    let manifest_bytes = read_path(&manifest_path_text, "import manifest")?;
    let outcomes_bytes = read_path(&outcomes_path, "outcome snapshot")?;
    let block_map_bytes = read_path(&block_map_path, "spatial block map")?;

    let mut source_files = BTreeMap::new();
    for relative in referenced_source_paths(&manifest_bytes).map_err(|errors| errors.join("\n"))? {
        let resolved = resolved_manifest_source(&manifest_path, &relative)?;
        let resolved_text = resolved.to_string_lossy().into_owned();
        let bytes = read_path(&resolved_text, "source file")?;
        source_files.insert(relative, bytes);
    }

    let source_commit =
        option_value(&args, "--source-commit")?.unwrap_or_else(|| "UNPINNED_LOCAL_RUN".to_owned());
    let rustc_version = option_value(&args, "--rustc-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());
    let cargo_version = option_value(&args, "--cargo-version")?
        .unwrap_or_else(|| "UNSPECIFIED_LOCAL_TOOLCHAIN".to_owned());
    let lock_path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("Cargo.lock");
    let lock_bytes = fs::read(&lock_path)
        .map_err(|error| format!("cannot read generated dependency lock: {error}"))?;

    let bundle = import_views_archive(
        &manifest_bytes,
        &outcomes_bytes,
        &block_map_bytes,
        &source_files,
        ImportRuntime {
            source_commit: &source_commit,
            rustc_version: &rustc_version,
            cargo_version: &cargo_version,
            lock_bytes: &lock_bytes,
        },
    )
    .map_err(|errors| errors.join("\n"))?;

    fs::write(&output_path, &bundle.normalized_dataset_bytes)
        .map_err(|error| format!("cannot write normalized dataset {output_path}: {error}"))?;
    let receipt_bytes = serde_json::to_vec_pretty(&bundle.receipt)
        .map_err(|error| format!("cannot serialize import receipt: {error}"))?;
    let mut receipt_bytes = receipt_bytes;
    receipt_bytes.push(b'\n');
    fs::write(&receipt_path, &receipt_bytes)
        .map_err(|error| format!("cannot write import receipt {receipt_path}: {error}"))?;

    println!(
        "Offline archive import completed; normalized dataset and receipt were written to the requested paths."
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(1);
    }
}
