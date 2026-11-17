// SPDX-License-Identifier: AGPL-3.0-or-later

use physics_agent_benchmark::evaluate_registry;
use serde_json::Value;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<(), (u8, String)> {
    let mut args = env::args().skip(1);
    let registry_path = args.next().map(PathBuf::from).ok_or_else(|| {
        (
            64,
            "usage: physics-agent-benchmark REGISTRY.json".to_owned(),
        )
    })?;

    if args.next().is_some() {
        return Err((64, "unexpected extra argument".to_owned()));
    }

    let bytes = fs::read(&registry_path).map_err(|error| {
        (
            66,
            format!("cannot read registry {}: {error}", registry_path.display()),
        )
    })?;
    let registry: Value = serde_json::from_slice(&bytes)
        .map_err(|error| (65, format!("registry JSON parse failed: {error}")))?;

    let report = evaluate_registry(&registry).map_err(|error| (2, error))?;
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| (70, format!("cannot serialize benchmark report: {error}")))?
    );

    if report["status"].as_str() == Some("passed") {
        Ok(())
    } else {
        Err((
            1,
            "one or more synthetic case classifications differ from the frozen oracle".to_owned(),
        ))
    }
}
