use serde::Serialize;
use std::{env, process::ExitCode};

use symtropy_physics_env_layout::{AosoaLayout, AosLayout, SoaLayout};

#[derive(Serialize)]
struct LayoutReceipt {
    schema_version: u32,
    workload_id: &'static str,
    environment_count: usize,
    steps: usize,
    dt: f64,
    aos_digest: String,
    soa_digest: String,
    aosoa4_digest: String,
    aosoa8_digest: String,
    aosoa16_digest: String,
    outputs_bit_identical: bool,
    evidence_class: &'static str,
    claim_ceiling: &'static str,
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("symtropy-physics-env-layout: {error}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let count = parse_arg(args.next(), 10_000, "environment_count")?;
    let steps = parse_arg(args.next(), 100, "steps")?;
    if args.next().is_some() {
        return Err("usage: symtropy-physics-env-layout [environment_count] [steps]".to_owned());
    }

    let base = AosLayout::from_count(count);
    let mut aos = base.clone();
    let mut soa = SoaLayout::from_aos(&base);
    let mut aosoa4 = AosoaLayout::<4>::from_aos(&base).map_err(|e| e.to_string())?;
    let mut aosoa8 = AosoaLayout::<8>::from_aos(&base).map_err(|e| e.to_string())?;
    let mut aosoa16 = AosoaLayout::<16>::from_aos(&base).map_err(|e| e.to_string())?;

    const DT: f64 = 1.0 / 120.0;
    aos.step_n(steps, DT).map_err(|e| e.to_string())?;
    soa.step_n(steps, DT).map_err(|e| e.to_string())?;
    aosoa4.step_n(steps, DT).map_err(|e| e.to_string())?;
    aosoa8.step_n(steps, DT).map_err(|e| e.to_string())?;
    aosoa16.step_n(steps, DT).map_err(|e| e.to_string())?;

    // Compare canonical per-environment f64 bit patterns directly. Digests
    // are compact receipts, not the correctness oracle itself.
    let aos_snapshot = aos.canonical_snapshot(count).map_err(|e| e.to_string())?;
    let soa_snapshot = soa.canonical_snapshot(count).map_err(|e| e.to_string())?;
    let aosoa4_snapshot = aosoa4.canonical_snapshot(count).map_err(|e| e.to_string())?;
    let aosoa8_snapshot = aosoa8.canonical_snapshot(count).map_err(|e| e.to_string())?;
    let aosoa16_snapshot = aosoa16.canonical_snapshot(count).map_err(|e| e.to_string())?;
    let outputs_bit_identical = aos_snapshot == soa_snapshot
        && aos_snapshot == aosoa4_snapshot
        && aos_snapshot == aosoa8_snapshot
        && aos_snapshot == aosoa16_snapshot;
    if !outputs_bit_identical {
        return Err("layout outputs are not bit-identical".to_owned());
    }

    let aos_digest = aos.digest(count).map_err(|e| e.to_string())?;
    let soa_digest = soa.digest(count).map_err(|e| e.to_string())?;
    let aosoa4_digest = aosoa4.digest(count).map_err(|e| e.to_string())?;
    let aosoa8_digest = aosoa8.digest(count).map_err(|e| e.to_string())?;
    let aosoa16_digest = aosoa16.digest(count).map_err(|e| e.to_string())?;

    let receipt = LayoutReceipt {
        schema_version: 1,
        workload_id: "independent-constant-acceleration-layout-v0",
        environment_count: count,
        steps,
        dt: DT,
        aos_digest,
        soa_digest,
        aosoa4_digest,
        aosoa8_digest,
        aosoa16_digest,
        outputs_bit_identical,
        evidence_class: "synthetic_only",
        claim_ceiling: "same-kernel-layout-conformance-only;no-contact-physics-or-gpu-performance-claim",
    };
    println!("{}", serde_json::to_string_pretty(&receipt).map_err(|e| e.to_string())?);
    Ok(())
}

fn parse_arg(value: Option<String>, default: usize, name: &str) -> Result<usize, String> {
    match value {
        None => Ok(default),
        Some(value) => value.parse().map_err(|e| format!("invalid {name}: {e}")),
    }
}
