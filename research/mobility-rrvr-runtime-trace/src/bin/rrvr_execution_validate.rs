// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Validate simulator/robot fault execution evidence against a compiled plan.
//!
//! A passing record means the supplied request, dispatch, execution and
//! observation evidence are internally linked. It does not authenticate
//! producers or establish physical truth or retry authority.

use mobility_rrvr_runtime_trace::campaign_plan::CompiledCampaignPlan;
use mobility_rrvr_runtime_trace::execution_evidence::ExecutionEvidenceBundle;
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

fn valid_sha256(value: &str) -> bool {
    value.len() == 64 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 && args.len() != 6 {
        return Err(concat!(
            "usage: rrvr_execution_validate <compiled-plan.json> <source-manifest.json> ",
            "<expected-plan-sha256> <execution-bundle.json> [report.json]"
        )
        .to_owned());
    }

    let plan_bytes =
        fs::read(&args[1]).map_err(|error| format!("read compiled campaign plan: {error}"))?;
    let source_bytes =
        fs::read(&args[2]).map_err(|error| format!("read source campaign manifest: {error}"))?;
    let expected_plan_sha256 = &args[3];
    let evidence_bytes =
        fs::read(&args[4]).map_err(|error| format!("read execution evidence bundle: {error}"))?;

    if !valid_sha256(expected_plan_sha256) {
        return Err("expected plan digest must be 64 hexadecimal characters".to_owned());
    }

    let plan: CompiledCampaignPlan = serde_json::from_slice(&plan_bytes)
        .map_err(|error| format!("strict compiled plan parse failed: {error}"))?;
    plan.verify_integrity()
        .map_err(|error| format!("compiled plan integrity check failed: {error:?}"))?;
    plan.verify_source_manifest_bytes(&source_bytes)
        .map_err(|error| format!("source manifest binding failed: {error:?}"))?;
    if plan
        .canonical_bytes()
        .map_err(|error| format!("{error:?}"))?
        != plan_bytes
    {
        return Err("compiled plan is not in the declared canonical encoding".to_owned());
    }
    if !plan.plan_sha256.eq_ignore_ascii_case(expected_plan_sha256) {
        return Err(
            "compiled plan digest does not match the externally pinned expected digest".to_owned(),
        );
    }

    let bundle: ExecutionEvidenceBundle = serde_json::from_slice(&evidence_bytes)
        .map_err(|error| format!("strict execution evidence parse failed: {error}"))?;
    let summary = bundle
        .validate_against_plan(&plan)
        .map_err(|error| format!("execution evidence rejected: {error:?}"))?;

    let report = json!({
        "validation": "passed",
        "campaign_id": bundle.campaign_id,
        "scenario_id": summary.scenario_id,
        "plan_sha256": plan.plan_sha256,
        "compiled_plan_artifact_sha256": digest_hex(&plan_bytes),
        "source_manifest_sha256": digest_hex(&source_bytes),
        "execution_bundle_sha256": digest_hex(&evidence_bytes),
        "run_id": bundle.run_id,
        "run_epoch": bundle.run_epoch,
        "world_epoch": bundle.world_epoch,
        "reset_status": summary.reset_status.as_str(),
        "planned_fault_count": summary.planned_fault_count,
        "dispatch_accepted_count": summary.dispatch_accepted_count,
        "dispatch_rejected_count": summary.dispatch_rejected_count,
        "dispatch_unknown_count": summary.dispatch_unknown_count,
        "executed_count": summary.executed_count,
        "not_executed_count": summary.not_executed_count,
        "execution_unknown_count": summary.execution_unknown_count,
        "delivery_confirmed_count": summary.delivery_confirmed_count,
        "delivery_refuted_count": summary.delivery_refuted_count,
        "delivery_unknown_count": summary.delivery_unknown_count,
        "delivery_not_observed_count": summary.delivery_not_observed_count,
        "delivery_observer_receipt_count": summary.delivery_observer_receipt_count,
        "required_delivery_observer_quorum": plan.platform.required_delivery_observer_quorum,
        "reconcile_required": summary.reconcile_required,
        "retry_permitted": summary.retry_permitted,
        "native_authority_created": false,
        "producer_authentication_performed": false,
        "physical_observer_independence_proven": false,
        "physical_safety_established": false,
        "claim_note": "PASS means the supplied records are linked to the exact compiled plan and internally consistent. Dispatch acceptance is not delivery; delivery confirmation requires execution evidence and a separate observation artifact. Producer authenticity, universal observer independence, physical safety and retry authority are not established."
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize execution evidence report: {error}"))?;

    if args.len() == 6 {
        fs::write(&args[5], &report_bytes)
            .map_err(|error| format!("write execution evidence report: {error}"))?;
    }

    let output = String::from_utf8(report_bytes)
        .map_err(|error| format!("execution evidence report is not UTF-8: {error}"))?;
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
