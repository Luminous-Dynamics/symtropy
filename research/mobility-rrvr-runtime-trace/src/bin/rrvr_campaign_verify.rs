// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Verify a compiled RRVR plan against its exact source manifest and pinned digest.
//!
//! This checks canonical encoding and nested commitments. It does not authenticate
//! the party that supplied the plan or execute any declared fault.

use mobility_rrvr_runtime_trace::campaign_plan::CompiledCampaignPlan;
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
    if args.len() != 4 && args.len() != 5 {
        return Err(concat!(
            "usage: rrvr_campaign_verify <compiled-plan.json> <source-manifest.json> ",
            "<expected-plan-sha256> [report.json]"
        )
        .to_owned());
    }

    let plan_bytes =
        fs::read(&args[1]).map_err(|error| format!("read compiled campaign plan: {error}"))?;
    let source_bytes =
        fs::read(&args[2]).map_err(|error| format!("read source campaign manifest: {error}"))?;
    let expected_plan_sha256 = &args[3];
    if expected_plan_sha256.len() != 64
        || !expected_plan_sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err("expected plan digest must be 64 hexadecimal characters".to_owned());
    }

    let plan: CompiledCampaignPlan = serde_json::from_slice(&plan_bytes)
        .map_err(|error| format!("strict compiled plan parse failed: {error}"))?;
    plan.verify_integrity()
        .map_err(|error| format!("compiled plan integrity check failed: {error:?}"))?;
    plan.verify_source_manifest_bytes(&source_bytes)
        .map_err(|error| format!("source manifest binding failed: {error:?}"))?;

    let canonical_bytes = plan
        .canonical_bytes()
        .map_err(|error| format!("canonical plan serialization failed: {error:?}"))?;
    if canonical_bytes != plan_bytes {
        return Err("compiled plan bytes are not in the declared canonical encoding".to_owned());
    }
    if !plan.plan_sha256.eq_ignore_ascii_case(expected_plan_sha256) {
        return Err(concat!(
            "compiled plan digest does not match the externally pinned ",
            "expected digest"
        )
        .to_owned());
    }

    let report = json!({
        "validation": "passed",
        "campaign_id": plan.campaign_id,
        "source_manifest_sha256": digest_hex(&source_bytes),
        "plan_sha256": plan.plan_sha256,
        "compiled_plan_artifact_sha256": digest_hex(&plan_bytes),
        "canonical_encoding": true,
        "nested_pair_and_schedule_commitments": true,
        "source_manifest_binding": true,
        "external_expected_digest_match": true,
        "fault_injection_executed": false,
        "native_authority_created": false,
        "platform_inventory_authenticated": false,
        "physical_safety_established": false,
        "claim_note": concat!(
            "PASS means the plan is canonically encoded and its commitments match the supplied ",
            "source manifest and external digest pin. It does not authenticate the plan publisher, ",
            "execute an injector, or establish physical safety."
        )
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize plan verification report: {error}"))?;

    if args.len() == 5 {
        fs::write(&args[4], &report_bytes)
            .map_err(|error| format!("write plan verification report: {error}"))?;
    }

    let output = String::from_utf8(report_bytes)
        .map_err(|error| format!("verification report is not UTF-8: {error}"))?;
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
