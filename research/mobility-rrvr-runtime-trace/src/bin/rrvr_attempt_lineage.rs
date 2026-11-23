// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Validate a supplied RRVR multi-attempt evidence lineage.
//!
//! A successful report means the supplied claims/receipts are structurally
//! consistent under this reference contract. It does not authenticate owners,
//! prove capture completeness, run the TLA+ model, or issue native authority.

use mobility_rrvr_runtime_trace::attempt_lineage::{
    AttemptLineageRecording, PhysicalOutcome,
};
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

fn outcome_name(outcome: PhysicalOutcome) -> &'static str {
    match outcome {
        PhysicalOutcome::NotObserved => "not_observed",
        PhysicalOutcome::Applied => "applied",
        PhysicalOutcome::NoEffect => "no_effect",
        PhysicalOutcome::Partial => "partial",
        PhysicalOutcome::Unknown => "unknown",
        PhysicalOutcome::Disputed => "disputed",
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 2 && args.len() != 3 {
        return Err(
            "usage: rrvr_attempt_lineage <recording.json> [summary.json]".to_owned(),
        );
    }

    let input = fs::read(&args[1]).map_err(|error| format!("read recording: {error}"))?;
    let recording: AttemptLineageRecording = serde_json::from_slice(&input)
        .map_err(|error| format!("strict attempt-lineage parse failed: {error}"))?;
    let summary = recording
        .validate()
        .map_err(|error| format!("attempt lineage rejected: {error:?}"))?;

    let report = json!({
        "version": recording.version,
        "logical_effect_id": recording.logical_effect_id,
        "fault_profile_revision": recording.fault_profile_revision,
        "recording_sha256": digest_hex(&input),
        "validation": "passed",
        "attempt_count": summary.attempt_count,
        "recovery_edge_count": summary.recovery_edge_count,
        "applied_attempt_count": summary.applied_attempt_count,
        "head_attempt_id": summary.head_attempt_id,
        "head_physical_outcome": outcome_name(summary.head_outcome),
        "owner_authentication": "not_performed",
        "evidence_completeness": "not_established",
        "tla_refinement": "not_checked",
        "physical_safety": "not_established",
        "native_action_authority": "not_created",
        "claim_note": "PASS means supplied evidence records satisfy the reference lineage contract; it is not proof that the owner assertions or physical observations are true."
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize validation report: {error}"))?;

    if args.len() == 3 {
        fs::write(&args[2], &report_bytes)
            .map_err(|error| format!("write validation report: {error}"))?;
    }

    let output = String::from_utf8(report_bytes)
        .map_err(|error| format!("validation report is not UTF-8: {error}"))?;
    println!("{output}");
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
