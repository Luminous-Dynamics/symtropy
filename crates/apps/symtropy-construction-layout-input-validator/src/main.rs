// SPDX-License-Identifier: AGPL-3.0-or-later
use std::{env, fs, process};

use symtropy_construction_layout_input_validator::{evaluate_manifest, TrustedPolicy};

const DEMO_POLICY: &str =
    include_str!("../../../../research/construction-layout-input-v0-1/policy.synthetic.json");


fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let mut manifest_path = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--manifest" => {
                if manifest_path.is_some() {
                    return Err("--manifest may be supplied only once".to_owned());
                }
                manifest_path = Some(
                    args.next()
                        .ok_or_else(|| "--manifest requires a path".to_owned())?,
                );
            }
            "--help" | "-h" => {
                println!(
                    "Usage: symtropy-construction-layout-input-validator --manifest FILE\n\
                     Uses an embedded synthetic demonstration policy. Production code must call \
                     the library with a trusted project policy."
                );
                return Ok(());
            }
            _ => return Err(format!("unknown argument: {arg}")),
        }
    }
    let manifest_path = manifest_path.ok_or_else(|| "--manifest is required".to_owned())?;
    let raw = fs::read_to_string(&manifest_path)
        .map_err(|e| format!("cannot read {manifest_path}: {e}"))?;
    let policy: TrustedPolicy =
        serde_json::from_str(DEMO_POLICY).map_err(|e| format!("invalid embedded policy: {e}"))?;
    let result = evaluate_manifest(&raw, &policy)?;
    println!(
        "{}",
        serde_json::to_string_pretty(&result)
            .map_err(|e| format!("cannot serialize evaluation: {e}"))?
    );
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        process::exit(2);
    }
}
