// SPDX-License-Identifier: AGPL-3.0-or-later
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process;

use symtropy_construction_evidence_validator::evaluate_packet;

fn usage() -> &'static str {
    "Usage: symtropy-construction-evidence-validator --packet FILE [--output FILE]\n\nSchemas and claim-requirements profiles are embedded in the validator binary and cannot be replaced at runtime. Exit 0 means the packet was structurally evaluated. Individual claim outcomes are in the result JSON; inadmissible or indeterminate claims are valid evaluation results. Exit 2 means the packet or invocation is invalid."
}

fn run() -> Result<(), String> {
    let mut values = BTreeMap::new();
    let mut args = env::args().skip(1);
    while let Some(key) = args.next() {
        if key == "--help" || key == "-h" {
            println!("{}", usage());
            return Ok(());
        }
        if !key.starts_with("--") {
            return Err(format!("unexpected argument: {key}\n{}", usage()));
        }
        let value = args
            .next()
            .ok_or_else(|| format!("missing value for {key}\n{}", usage()))?;
        if values.insert(key.clone(), value).is_some() {
            return Err(format!("duplicate argument: {key}"));
        }
    }

    let packet_path = values
        .remove("--packet")
        .ok_or_else(|| format!("required argument --packet is missing\n{}", usage()))?;
    let output_path = values.remove("--output");
    if let Some((unknown, _)) = values.into_iter().next() {
        return Err(format!("unknown argument: {unknown}\n{}", usage()));
    }

    let packet = fs::read_to_string(&packet_path)
        .map_err(|error| format!("cannot read {packet_path}: {error}"))?;
    let result = evaluate_packet(&packet)?;
    let rendered = serde_json::to_string_pretty(&result)
        .map_err(|error| format!("cannot serialize result: {error}"))?;
    if let Some(path) = output_path {
        fs::write(&path, format!("{rendered}\n"))
            .map_err(|error| format!("cannot write {path}: {error}"))?;
    } else {
        println!("{rendered}");
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}
