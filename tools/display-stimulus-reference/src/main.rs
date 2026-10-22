// SPDX-License-Identifier: Apache-2.0 OR MIT

use display_stimulus_reference::generate_stimulus;
use std::env;
use std::fs;
use std::path::{Path, PathBuf};

const DEFAULT_MANIFEST: &str = "docs/visual-technology/display-stimulus-recipes-v0.1.json";
const DEFAULT_SCHEMA: &str = "docs/visual-technology/display-stimulus-recipes.schema.json";

fn main() {
    if let Err(error) = run() {
        eprintln!("DISPLAY_STIMULUS_ERROR: {error}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let mut manifest_path = PathBuf::from(DEFAULT_MANIFEST);
    let mut schema_path = PathBuf::from(DEFAULT_SCHEMA);
    let mut output_path: Option<PathBuf> = None;
    let mut report_path: Option<PathBuf> = None;
    let mut case_id: Option<String> = None;
    let mut args = env::args().skip(1);

    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => manifest_path = next_path(&mut args, "--manifest")?,
            "--schema" => schema_path = next_path(&mut args, "--schema")?,
            "--case" => {
                case_id = Some(next_value(&mut args, "--case")?);
            }
            "--out" => output_path = Some(next_path(&mut args, "--out")?),
            "--report" => report_path = Some(next_path(&mut args, "--report")?),
            "--help" | "-h" => {
                print_help();
                return Ok(());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let case_id = case_id.ok_or_else(|| "--case is required".to_owned())?;
    let output_path = output_path.ok_or_else(|| "--out is required".to_owned())?;
    let manifest_text = read_file(&manifest_path)?;
    let schema_text = read_file(&schema_path)?;

    let generated = generate_stimulus(&manifest_text, &schema_text, &case_id)
        .map_err(|error| error.to_string())?;
    fs::write(&output_path, &generated.bytes)
        .map_err(|error| format!("cannot write {}: {error}", output_path.display()))?;

    let report_bytes = serde_json::to_vec_pretty(&generated.report)
        .map_err(|error| format!("cannot serialize report: {error}"))?;
    match report_path {
        Some(path) => {
            fs::write(&path, &report_bytes)
                .map_err(|error| format!("cannot write {}: {error}", path.display()))?;
        }
        None => {
            println!(
                "{}",
                String::from_utf8(report_bytes)
                    .map_err(|error| format!("report contains invalid UTF-8: {error}"))?
            );
        }
    }

    Ok(())
}

fn next_value(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<String, String> {
    args.next()
        .filter(|value| !value.is_empty())
        .ok_or_else(|| format!("{flag} requires a value"))
}

fn next_path(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<PathBuf, String> {
    next_value(args, flag).map(PathBuf::from)
}

fn read_file(path: &Path) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn print_help() {
    println!(
        "Generate a deterministic, unqualified PPM stimulus artifact.\n\
         Usage: display-stimulus-reference --case ID --out PATH [--report PATH]\n\
         Optional: --manifest PATH --schema PATH\n\
         A generated raster is a synthetic source artifact, not proof of physical display output."
    );
}
