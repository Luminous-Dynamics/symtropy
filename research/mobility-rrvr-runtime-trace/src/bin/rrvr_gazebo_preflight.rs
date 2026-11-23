// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Validate a captured Gazebo capability profile against a compiled RRVR plan.
//!
//! This is a preflight validator only. It does not contact a live simulator,
//! reset a world, step time, inject a fault, or create native authority.

use mobility_rrvr_runtime_trace::campaign_plan::CompiledCampaignPlan;
use mobility_rrvr_runtime_trace::gazebo_adapter::{
    GazeboCapabilityProfile, SteppingMethod,
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

fn stepping_method_name(method: SteppingMethod) -> &'static str {
    match method {
        SteppingMethod::StepSimulationService => "step_simulation_service",
        SteppingMethod::SimulateStepsAction => "simulate_steps_action",
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 && args.len() != 6 {
        return Err(concat!(
            "usage: rrvr_gazebo_preflight <compiled-plan.json> <source-manifest.json> ",
            "<expected-plan-sha256> <gazebo-capabilities.json> [report.json]"
        )
        .to_owned());
    }

    let plan_bytes =
        fs::read(&args[1]).map_err(|error| format!("read compiled campaign plan: {error}"))?;
    let source_bytes =
        fs::read(&args[2]).map_err(|error| format!("read source campaign manifest: {error}"))?;
    let capability_bytes =
        fs::read(&args[4]).map_err(|error| format!("read Gazebo capability profile: {error}"))?;
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
    if plan.canonical_bytes().map_err(|error| format!("{error:?}"))? != plan_bytes {
        return Err("compiled plan is not in the declared canonical encoding".to_owned());
    }
    if !plan.plan_sha256.eq_ignore_ascii_case(expected_plan_sha256) {
        return Err("compiled plan digest does not match the externally pinned expected digest".to_owned());
    }

    let profile: GazeboCapabilityProfile = serde_json::from_slice(&capability_bytes)
        .map_err(|error| format!("strict Gazebo capability profile parse failed: {error}"))?;
    let stepping_method = profile
        .validate_for_plan(&plan)
        .map_err(|error| format!("Gazebo capability profile rejected: {error:?}"))?;

    let report = json!({
        "validation": "passed",
        "preflight_scope": "captured_capability_profile_only",
        "campaign_id": plan.campaign_id,
        "plan_sha256": plan.plan_sha256,
        "source_manifest_sha256": digest_hex(&source_bytes),
        "compiled_plan_artifact_sha256": digest_hex(&plan_bytes),
        "capability_profile_sha256": digest_hex(&capability_bytes),
        "simulator_id": profile.simulator_id,
        "simulator_revision": profile.simulator_revision,
        "ros_distribution": profile.ros_distribution,
        "simulation_interfaces_revision": profile.simulation_interfaces_revision,
        "world_name": profile.world_name,
        "scene_broadcaster_id": profile.scene_broadcaster_id,
        "scene_broadcaster_revision": profile.scene_broadcaster_revision,
        "scene_broadcaster_sha256": profile.scene_broadcaster_sha256,
        "scene_broadcaster_loaded": profile.scene_broadcaster_loaded,
        "capabilities": profile.capabilities,
        "selected_stepping_method": stepping_method_name(stepping_method),
        "live_simulator_discovery_performed": false,
        "simulator_mutation_performed": false,
        "fault_injection_executed": false,
        "execution_receipts_emitted": false,
        "native_authority_created": false,
        "physical_safety_established": false,
        "claim_note": "PASS means the captured capability profile matches the simulator, ROS, Simulation Interfaces, environment, reset, SceneBroadcaster and required interface pins in the compiled plan. It does not mean a live simulator was contacted or mutated, a fault was injected, or physical safety was established."
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize Gazebo preflight report: {error}"))?;

    if args.len() == 6 {
        fs::write(&args[5], &report_bytes)
            .map_err(|error| format!("write Gazebo preflight report: {error}"))?;
    }

    let output = String::from_utf8(report_bytes)
        .map_err(|error| format!("Gazebo preflight report is not UTF-8: {error}"))?;
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
