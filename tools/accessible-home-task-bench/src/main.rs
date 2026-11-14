// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use accessible_home_task_bench::evaluate_fixture_bytes;
use std::{env, fs, path::PathBuf, process};

fn main() {
    if let Err(error) = run() {
        eprintln!("accessible-home-task-bench: {error}");
        process::exit(2);
    }
}
fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut input = PathBuf::from("tools/accessible-home-task-bench/fixtures/accessibility-task-benchmark-v0.1.json");
    let mut output: Option<PathBuf> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" | "--fixture" => input = PathBuf::from(args.next().ok_or_else(|| format!("{arg} requires a path"))?),
            "--output" => output = Some(PathBuf::from(args.next().ok_or_else(|| "--output requires a path".to_owned())?)),
            "--help" | "-h" => {
                println!("Usage: accessible-home-task-bench [--input FIXTURE.json] [--output REPORT.json]\nValidates a synthetic fixture and checks declared assessments. Exit code 2 signals invalid input or an assessment mismatch.");
                return Ok(());
            }
            other => return Err(format!("unknown argument '{other}'; use --help")),
        }
    }
    let bytes = fs::read(&input).map_err(|error| format!("cannot read fixture '{}': {error}",input.display()))?;
    let report = evaluate_fixture_bytes(&bytes)?;
    let rendered = serde_json::to_string_pretty(&report).map_err(|error| format!("cannot serialize receipt: {error}"))? + "\n";
    if let Some(path) = output {
        fs::write(&path,rendered.as_bytes()).map_err(|error| format!("cannot write report '{}': {error}",path.display()))?;
        eprintln!("PASS synthetic fixture assertions: {} scenarios, {} tasks; sha256 {}",report.scenario_count,report.task_count,report.input_sha256);
    } else {
        print!("{rendered}");
    }
    Ok(())
}
