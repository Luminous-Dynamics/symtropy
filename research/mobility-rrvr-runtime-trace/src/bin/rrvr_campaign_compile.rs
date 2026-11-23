// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Compile a validated RRVR manifest into an immutable execution plan.
//!
//! The plan is a deterministic instruction artifact. It does not execute the
//! injector, issue permits, create native authority, or claim fault delivery.

use mobility_rrvr_runtime_trace::campaign_plan::CompiledCampaignPlan;
use mobility_rrvr_runtime_trace::fault_campaign::FaultCampaignManifest;
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
    if args.len() != 3 && args.len() != 4 {
        return Err(
            "usage: rrvr_campaign_compile <manifest.json> <compiled-plan.json> [report.json]"
                .to_owned(),
        );
    }

    let input = fs::read(&args[1]).map_err(|error| format!("read campaign manifest: {error}"))?;
    let source_manifest_sha256 = digest_hex(&input);
    let manifest: FaultCampaignManifest = serde_json::from_slice(&input)
        .map_err(|error| format!("strict campaign manifest parse failed: {error}"))?;
    let plan = CompiledCampaignPlan::compile(&manifest, &source_manifest_sha256)
        .map_err(|error| format!("campaign plan compilation failed: {error:?}"))?;
    let plan_bytes = plan
        .canonical_bytes()
        .map_err(|error| format!("serialize canonical campaign plan: {error:?}"))?;
    let compiled_plan_artifact_sha256 = digest_hex(&plan_bytes);
    fs::write(&args[2], &plan_bytes)
        .map_err(|error| format!("write compiled campaign plan: {error}"))?;

    let fault_event_count: usize = plan
        .scenarios
        .iter()
        .map(|scenario| scenario.fault_events.len())
        .sum();
    let pair_commitments: Vec<_> = plan
        .pairs
        .iter()
        .map(|pair| {
            json!({
                "pair_id": pair.pair_id,
                "pair_commitment_sha256": pair.pair_commitment_sha256,
                "scenario_ids": pair.scenario_ids,
                "arms": pair.arms
            })
        })
        .collect();
    let schedule_commitments: Vec<_> = plan
        .scenarios
        .iter()
        .map(|scenario| {
            json!({
                "scenario_id": scenario.scenario_id,
                "pair_id": scenario.pair_id,
                "arm": scenario.arm,
                "schedule_sha256": scenario.schedule_sha256,
                "fault_event_count": scenario.fault_events.len()
            })
        })
        .collect();

    let report = json!({
        "version": plan.version,
        "canonicalization": plan.canonicalization,
        "campaign_id": plan.campaign_id,
        "source_manifest_sha256": source_manifest_sha256,
        "plan_sha256": plan.plan_sha256,
        "compiled_plan_artifact_sha256": compiled_plan_artifact_sha256,
        "platform_profile_sha256": plan.platform.profile_sha256,
        "environment_artifact_sha256": plan.platform.environment_artifact_sha256,
        "fault_injector_revision": plan.platform.fault_injector_revision,
        "fault_injector_sha256": plan.platform.fault_injector_sha256,
        "fault_injector_fault_domain_id": plan.platform.fault_injector_fault_domain_id,
        "native_adapter_revision": plan.platform.native_adapter_revision,
        "runtime_monitor_revision": plan.platform.runtime_monitor_revision,
        "scenario_count": plan.scenarios.len(),
        "pair_count": plan.pairs.len(),
        "fault_event_count": fault_event_count,
        "pairs": pair_commitments,
        "scenario_schedules": schedule_commitments,
        "fault_injection_executed": false,
        "native_authority_created": false,
        "platform_inventory_authenticated": false,
        "physical_safety_established": false,
        "claim_note": concat!(
            "PASS means a deterministic, digest-bound campaign instruction plan was compiled ",
            "from a validated manifest. It does not mean any fault was injected or delivered, ",
            "authority was issued, or a physical safety claim was established."
        )
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize compilation report: {error}"))?;

    if args.len() == 4 {
        fs::write(&args[3], &report_bytes)
            .map_err(|error| format!("write campaign compilation report: {error}"))?;
    }

    let output = String::from_utf8(report_bytes)
        .map_err(|error| format!("compilation report is not UTF-8: {error}"))?;
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
