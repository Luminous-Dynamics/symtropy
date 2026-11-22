// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use mobility_rrvr_runtime_trace::RuntimeProjectionRecording;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

fn digest_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    const DIGITS: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(64);
    for byte in digest {
        output.push(DIGITS[(byte >> 4) as usize] as char);
        output.push(DIGITS[(byte & 0x0f) as usize] as char);
    }
    output
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 4 {
        return Err(
            "usage: rrvr_runtime_project <recording.json> <reflow-trace.ndjson> <provenance-sidecar.json>"
                .to_owned(),
        );
    }

    let input = fs::read(&args[1]).map_err(|error| format!("read recording: {error}"))?;
    let recording: RuntimeProjectionRecording = serde_json::from_slice(&input)
        .map_err(|error| format!("strict runtime projection parse failed: {error}"))?;
    recording
        .validate()
        .map_err(|error| format!("runtime projection rejected: {error:?}"))?;

    let trace = recording
        .to_tla_trace_ndjson()
        .map_err(|error| format!("TLA trace emission failed: {error:?}"))?;
    let sidecar = recording
        .to_provenance_sidecar_json()
        .map_err(|error| format!("provenance sidecar emission failed: {error:?}"))?;
    let sidecar_bytes = serde_json::to_vec_pretty(&sidecar)
        .map_err(|error| format!("serialize sidecar: {error}"))?;

    fs::write(&args[2], trace.as_bytes())
        .map_err(|error| format!("write Reflow trace: {error}"))?;
    fs::write(&args[3], &sidecar_bytes)
        .map_err(|error| format!("write provenance sidecar: {error}"))?;

    let manifest = json!({
        "version": recording.version,
        "bundle_id": recording.header.bundle_id,
        "projection_validation": "passed",
        "tla_conformance": "not_run",
        "input_sha256": digest_hex(&input),
        "trace_sha256": digest_hex(trace.as_bytes()),
        "sidecar_sha256": digest_hex(&sidecar_bytes),
        "snapshot_count": recording.snapshots.len(),
        "claim_note": "Projection validation is not a TLC/Reflow trace-conformance result or a physical-safety claim."
    });
    println!("{}", serde_json::to_string_pretty(&manifest).expect("manifest serializes"));
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
