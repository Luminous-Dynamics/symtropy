// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use scan_bim_replay_contract::{
    audit_manifest_bytes, verify_manifest_artifact_files, ArtifactVerificationSummary,
    ReplayAuditReport,
};
use serde::Serialize;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize)]
struct CliOutput {
    metadata_audit: ReplayAuditReport,
    artifact_verification: ArtifactVerificationSummary,
}

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let mut manifest_path = None;
    let mut candidate_root = None;
    let mut evaluator_root = None;

    while let Some(flag) = args.next() {
        let value = PathBuf::from(
            args.next()
                .ok_or_else(|| format!("missing value for {}", flag.to_string_lossy()))?,
        );
        match flag.to_str() {
            Some("--manifest") => manifest_path = Some(value),
            Some("--candidate-root") => candidate_root = Some(value),
            Some("--evaluator-root") => evaluator_root = Some(value),
            _ => return Err(format!("unknown argument: {}", flag.to_string_lossy()).into()),
        }
    }

    let manifest_path = manifest_path.ok_or("expected --manifest <path>")?;
    let candidate_root = candidate_root.ok_or("expected --candidate-root <directory>")?;
    let evaluator_root = evaluator_root.ok_or("expected --evaluator-root <directory>")?;

    let bytes = fs::read(&manifest_path)?;
    let metadata_audit = audit_manifest_bytes(&bytes).map_err(std::io::Error::other)?;
    let artifact_verification =
        verify_manifest_artifact_files(&bytes, &candidate_root, &evaluator_root)
            .map_err(std::io::Error::other)?;

    let output = CliOutput {
        metadata_audit,
        artifact_verification,
    };
    println!("{}", serde_json::to_string_pretty(&output)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("scan-bim-replay-contract: {error}");
        std::process::exit(2);
    }
}
