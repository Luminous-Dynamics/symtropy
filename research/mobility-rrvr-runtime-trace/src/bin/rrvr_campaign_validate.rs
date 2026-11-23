// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Validate a frozen, paired RRVR fault campaign manifest.
//!
//! A successful report means the experiment plan is pinned and internally
//! comparable. It does not execute fault injection or establish physical safety.

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
    if args.len() != 2 && args.len() != 3 {
        return Err("usage: rrvr_campaign_validate <manifest.json> [summary.json]".to_owned());
    }

    let input = fs::read(&args[1]).map_err(|error| format!("read campaign manifest: {error}"))?;
    let manifest: FaultCampaignManifest = serde_json::from_slice(&input)
        .map_err(|error| format!("strict campaign manifest parse failed: {error}"))?;
    let summary = manifest
        .validate()
        .map_err(|error| format!("campaign manifest rejected: {error:?}"))?;

    let covered_layers: Vec<&str> = summary
        .covered_layers
        .iter()
        .map(|layer| layer.as_str())
        .collect();
    let covered_cut_points: Vec<&str> = summary
        .covered_cut_points
        .iter()
        .map(|cut| cut.as_str())
        .collect();
    let required_layers: Vec<&str> = manifest
        .required_layers
        .iter()
        .map(|layer| layer.as_str())
        .collect();
    let required_cut_points: Vec<&str> = manifest
        .required_cut_points
        .iter()
        .map(|cut| cut.as_str())
        .collect();

    let report = json!({
        "version": manifest.version,
        "campaign_id": manifest.campaign_id,
        "manifest_sha256": digest_hex(&input),
        "fault_model_revision": manifest.fault_model_revision,
        "platform_id": manifest.platform.platform_id,
        "platform_profile_sha256": manifest.platform.profile_sha256,
        "environment_artifact_sha256": manifest.platform.environment_artifact_sha256,
        "simulator_revision": manifest.platform.simulator_revision,
        "gazebo_release": manifest.platform.gazebo_release,
        "world_stats_topic": manifest.platform.world_stats_topic,
        "scene_broadcaster_id": manifest.platform.scene_broadcaster_id,
        "scene_broadcaster_revision": manifest.platform.scene_broadcaster_revision,
        "scene_broadcaster_sha256": manifest.platform.scene_broadcaster_sha256,
        "ros_distribution": manifest.platform.ros_distribution,
        "simulation_interfaces_revision": manifest.platform.simulation_interfaces_revision,
        "fault_injector_id": manifest.platform.fault_injector_id,
        "fault_injector_revision": manifest.platform.fault_injector_revision,
        "fault_injector_sha256": manifest.platform.fault_injector_sha256,
        "fault_injector_fault_domain_id": manifest.platform.fault_injector_fault_domain_id,
        "native_adapter_id": manifest.platform.native_adapter_id,
        "native_adapter_revision": manifest.platform.native_adapter_revision,
        "runtime_monitor_revision": manifest.platform.runtime_monitor_revision,
        "reset_profile_sha256": manifest.reset.profile_sha256,
        "validation": "passed",
        "scenario_count": summary.scenario_count,
        "pair_count": summary.pair_count,
        "positive_control_count": summary.positive_control_count,
        "negative_control_count": summary.negative_control_count,
        "mutation_control_count": summary.mutation_control_count,
        "fault_injection_count": summary.fault_injection_count,
        "full_rrvr_positive_count": summary.full_rrvr_positive_count,
        "full_rrvr_negative_count": summary.full_rrvr_negative_count,
        "required_layers": required_layers,
        "covered_layers": covered_layers,
        "required_cut_points": required_cut_points,
        "covered_cut_points": covered_cut_points,
        "coverage_complete": true,
        "fault_injection_executed": false,
        "platform_inventory_authenticated": false,
        "owner_evidence_authenticated": false,
        "physical_safety_established": false,
        "tla_refinement_checked": false,
        "claim_note": "PASS means the manifest is pinned, complete against its declared layer/cut inventory, and internally paired. It does not mean the injector ran, the inventory is complete, or the robot is safe."
    });
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("serialize campaign report: {error}"))?;

    if args.len() == 3 {
        fs::write(&args[2], &report_bytes)
            .map_err(|error| format!("write campaign report: {error}"))?;
    }

    let output = String::from_utf8(report_bytes)
        .map_err(|error| format!("campaign report is not UTF-8: {error}"))?;
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
