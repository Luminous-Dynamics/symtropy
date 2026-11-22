// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use mobility_rrvr_trace::{TlcImportContext, import_tlc_json_trace, trace_bundle_to_json_value};
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

fn hex(bytes: &[u8]) -> String {
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 12 {
        return Err(
            "usage: tlc_trace_import <trace.json> <model.tla> <model.cfg> <tla2tools.jar> <output.json> <bundle-id> <system-id> <implementation-revision> <property-id> <violation-class> <replay-seed>".to_owned()
        );
    }

    let raw_trace = fs::read(&args[1]).map_err(|error| format!("read TLC trace: {error}"))?;
    let model_source = fs::read(&args[2]).map_err(|error| format!("read TLA model: {error}"))?;
    let model_config = fs::read(&args[3]).map_err(|error| format!("read TLC config: {error}"))?;
    let tool_jar = fs::read(&args[4]).map_err(|error| format!("read pinned TLC JAR: {error}"))?;
    let model_digest = Sha256::digest(&model_source);
    let config_digest = Sha256::digest(&model_config);
    let tool_digest = Sha256::digest(&tool_jar);
    let replay_seed = args[11]
        .parse::<u64>()
        .map_err(|error| format!("invalid replay seed: {error}"))?;

    let model_revision_label = format!("sha256:{}", hex(&model_digest));
    let fault_profile_revision_label = format!("sha256:{}", hex(&config_digest));
    let oracle_revision_label = format!("TLC-2ef2af8+sha256:{}", hex(&tool_digest));

    let context = TlcImportContext {
        bundle_id: &args[6],
        system_id: &args[7],
        implementation_revision: &args[8],
        replay_seed,
        property_id: &args[9],
        violation_class: &args[10],
        actor_id: "TLC",
        model_source_bytes: &model_source,
        model_config_bytes: &model_config,
        tla_tool_jar_bytes: &tool_jar,
        model_revision_label: &model_revision_label,
        fault_profile_revision_label: &fault_profile_revision_label,
        oracle_revision_label: &oracle_revision_label,
    };

    let bundle = import_tlc_json_trace(&raw_trace, &context)
        .map_err(|error| format!("TLC JSON import rejected: {error:?}"))?;
    let output = serde_json::to_vec_pretty(&trace_bundle_to_json_value(&bundle))
        .map_err(|error| format!("serialize RRVR trace bundle: {error}"))?;
    fs::write(&args[5], output).map_err(|error| format!("write RRVR trace bundle: {error}"))?;

    println!(
        "Imported {} TLC transitions; witness={}; bundle={}",
        bundle.events.len(),
        bundle.header.target_failure.witness_event_id,
        bundle.header.bundle_id
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
