// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use scan_bim_replay_contract::audit_manifest_bytes;
use std::env;
use std::error::Error;
use std::fs;
use std::path::PathBuf;

fn run() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os().skip(1);
    let flag = args.next().ok_or("expected --manifest <path>")?;
    if flag != "--manifest" {
        return Err("expected --manifest <path>".into());
    }
    let path = PathBuf::from(args.next().ok_or("missing manifest path")?);
    if args.next().is_some() {
        return Err("unexpected trailing arguments".into());
    }

    let bytes = fs::read(&path)?;
    let report = audit_manifest_bytes(&bytes).map_err(std::io::Error::other)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("scan-bim-replay-contract: {error}");
        std::process::exit(2);
    }
}
