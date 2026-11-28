// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{
    env, fs,
    fs::{File, OpenOptions},
    io::Write,
    os::unix::fs::OpenOptionsExt,
    path::{Path, PathBuf},
    process::ExitCode,
};

use mobility_rrvr_aerb_evaluator::{
    blind_projection_audit_v0_1::{
        MetadataAttackRow, MetadataLeakageReport, ProjectionError, audit_metadata_leakage,
    },
    sha256_hex,
};
use serde::{Deserialize, Serialize};

const INPUT_SCHEMA: &str = "aerb-metadata-attack-input-v0.1";
const MAX_INPUT_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct MetadataAuditInput {
    schema_version: String,
    campaign_id: String,
    requested_folds: usize,
    rows: Vec<MetadataAttackRow>,
}

#[derive(Debug, Serialize)]
struct MetadataAuditOutput {
    schema_version: String,
    input_sha256: String,
    report: MetadataLeakageReport,
}

fn encode_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut output = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        output.push(HEX[(byte >> 4) as usize] as char);
        output.push(HEX[(byte & 0x0f) as usize] as char);
    }
    output
}

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    if path.exists() {
        return Err("OUTPUT_ALREADY_EXISTS".to_owned());
    }
    let parent = path
        .parent()
        .ok_or_else(|| "OUTPUT_PARENT_MISSING".to_owned())?
        .canonicalize()
        .map_err(|error| format!("OUTPUT_PARENT_UNAVAILABLE: {error}"))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| "OUTPUT_FILENAME_INVALID".to_owned())?;

    let mut random = [0_u8; 16];
    getrandom::fill(&mut random).map_err(|error| format!("OUTPUT_TEMP_ENTROPY_FAILED: {error}"))?;
    let temporary = parent.join(format!(".{file_name}.tmp-{}", encode_hex(&random)));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .mode(0o600)
        .open(&temporary)
        .map_err(|error| format!("OUTPUT_TEMP_CREATE_FAILED: {error}"))?;

    let write_result = (|| -> Result<(), String> {
        file.write_all(bytes)
            .map_err(|error| format!("OUTPUT_WRITE_FAILED: {error}"))?;
        file.sync_all()
            .map_err(|error| format!("OUTPUT_FSYNC_FAILED: {error}"))?;
        drop(file);
        // Hard-link creation is atomic and fails when the final path already
        // exists; unlike rename, it cannot silently replace an existing report.
        fs::hard_link(&temporary, path)
            .map_err(|error| format!("OUTPUT_ATOMIC_CREATE_FAILED: {error}"))?;
        fs::remove_file(&temporary)
            .map_err(|error| format!("OUTPUT_TEMP_REMOVE_FAILED: {error}"))?;
        File::open(&parent)
            .and_then(|directory| directory.sync_all())
            .map_err(|error| format!("OUTPUT_DIRECTORY_FSYNC_FAILED: {error}"))?;
        Ok(())
    })();

    if write_result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    write_result
}

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 {
        return Err(
            "usage: aerb_metadata_audit <evaluator-only-input.json> <evaluator-only-report.json>"
                .to_owned(),
        );
    }

    let input_path = PathBuf::from(&args[0]);
    let output_path = PathBuf::from(&args[1]);
    let input_metadata =
        fs::symlink_metadata(&input_path).map_err(|error| format!("INPUT_UNAVAILABLE: {error}"))?;
    if input_metadata.file_type().is_symlink() || !input_metadata.is_file() {
        return Err("INPUT_MUST_BE_A_REGULAR_NON_SYMLINK_FILE".to_owned());
    }
    if input_metadata.len() > MAX_INPUT_BYTES {
        return Err("INPUT_EXCEEDS_64_MIB_LIMIT".to_owned());
    }

    let input_parent = input_path
        .parent()
        .ok_or_else(|| "INPUT_PARENT_MISSING".to_owned())?
        .canonicalize()
        .map_err(|error| format!("INPUT_PARENT_UNAVAILABLE: {error}"))?;
    let output_parent = output_path
        .parent()
        .ok_or_else(|| "OUTPUT_PARENT_MISSING".to_owned())?
        .canonicalize()
        .map_err(|error| format!("OUTPUT_PARENT_UNAVAILABLE: {error}"))?;
    if input_parent != output_parent {
        return Err(
            "INPUT_AND_REPORT_MUST_REMAIN_IN_THE_SAME_EVALUATOR_CUSTODY_DIRECTORY".to_owned(),
        );
    }
    if fs::symlink_metadata(&output_path).is_ok() {
        return Err("OUTPUT_ALREADY_EXISTS_OR_IS_SYMLINK".to_owned());
    }

    let input_bytes =
        fs::read(&input_path).map_err(|error| format!("INPUT_READ_FAILED: {error}"))?;
    let input: MetadataAuditInput = serde_json::from_slice(&input_bytes)
        .map_err(|error| format!("INPUT_SCHEMA_INVALID: {error}"))?;
    if input.schema_version != INPUT_SCHEMA || input.campaign_id.trim().is_empty() {
        return Err("INPUT_SCHEMA_VERSION_OR_CAMPAIGN_ID_INVALID".to_owned());
    }
    if input
        .rows
        .iter()
        .any(|row| row.campaign_id != input.campaign_id)
    {
        return Err("INPUT_CAMPAIGN_ID_MISMATCH".to_owned());
    }

    let report = audit_metadata_leakage(&input.rows, input.requested_folds)
        .map_err(|error: ProjectionError| format!("METADATA_AUDIT_FAILED: {error}"))?;
    let output = MetadataAuditOutput {
        schema_version: "aerb-metadata-audit-output-v0.1".to_owned(),
        input_sha256: sha256_hex(&input_bytes),
        report,
    };
    let output_bytes = serde_json::to_vec_pretty(&output)
        .map_err(|error| format!("OUTPUT_SERIALIZATION_FAILED: {error}"))?;
    atomic_write(&output_path, &output_bytes)?;

    println!(
        "AERB_METADATA_AUDIT=COMPLETE campaign={} samples={} folds={} majority_accuracy={:.4} combined_accuracy={:.4} balanced_accuracy={:.4}",
        input.campaign_id,
        output.report.sample_n,
        output.report.fold_n,
        output.report.majority_baseline_accuracy,
        output.report.combined_metadata_only_accuracy,
        output.report.combined_metadata_only_balanced_accuracy
    );
    println!("Evaluator-only report written atomically; low accuracy is not proof of no leakage.");
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("AERB_METADATA_AUDIT_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}
