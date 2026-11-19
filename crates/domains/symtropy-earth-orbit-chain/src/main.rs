// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use serde_json::json;
use sha2::{Digest, Sha256};
use std::env;
use std::fs;
use std::process::ExitCode;

use symtropy_earth_orbit_chain::{
    CampaignDisposition, CampaignV1, evaluate_campaign,
};

fn is_valid_source_commit(value: &str) -> bool {
    value.len() == 40 && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}

fn sha256_hex(bytes: &[u8]) -> String {
    let digest = Sha256::digest(bytes);
    digest.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn run() -> Result<ExitCode, String> {
    let mut args = env::args().skip(1);
    let fixture_path = args.next().ok_or_else(|| {
        "usage: earth-orbit-chain-reference <campaign.json> --source-commit <40-hex-sha>".to_owned()
    })?;
    let flag = args.next().ok_or_else(|| "missing --source-commit".to_owned())?;
    if flag != "--source-commit" {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args.next().ok_or_else(|| "missing source commit value".to_owned())?;
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }
    if !is_valid_source_commit(&source_commit) {
        return Err("source commit must be exactly 40 hexadecimal characters".to_owned());
    }

    let campaign_bytes = fs::read(&fixture_path)
        .map_err(|error| format!("cannot read campaign fixture {fixture_path}: {error}"))?;
    let campaign: CampaignV1 = serde_json::from_slice(&campaign_bytes)
        .map_err(|error| format!("invalid campaign JSON: {error}"))?;
    let report = evaluate_campaign(&campaign);
    let result = match report.disposition {
        CampaignDisposition::Ready => "pass",
        CampaignDisposition::NeedsEvidence => "indeterminate",
        CampaignDisposition::Infeasible => "fail",
    };
    let receipt = json!({
        "receipt_schema": "luminous.earth-orbit-chain.receipt.v1",
        "validator_version": env!("CARGO_PKG_VERSION"),
        "source_commit": source_commit,
        "campaign_sha256": sha256_hex(&campaign_bytes),
        "result": result,
        "report": report
    });
    let rendered = serde_json::to_string_pretty(&receipt)
        .map_err(|error| format!("cannot serialize delivery receipt: {error}"))?;
    println!("{rendered}");

    if result == "pass" {
        Ok(ExitCode::SUCCESS)
    } else {
        Ok(ExitCode::FAILURE)
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("earth-orbit-chain-reference: {error}");
            ExitCode::FAILURE
        }
    }
}
