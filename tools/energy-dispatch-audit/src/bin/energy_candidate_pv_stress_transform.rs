//! Apply a named, receipt-bound PV-availability stress transform to a generated candidate.
//!
//! The input must be the exact Symthaea-exported PV+battery+grid candidate. The
//! transformed bytes and parent digest are preserved so this stress scenario
//! cannot be mistaken for an unmodified generator output.

use std::{env, error::Error, fs, io, path::PathBuf};

use serde::Serialize;
use serde_json::Value;
use sha2::{Digest, Sha256};

const BASE_CANDIDATE_ID: &str = "synthetic-pv-battery-grid";
const STRESS_CANDIDATE_ID: &str = "synthetic-pv-limited-battery-grid";
const PV_SURPLUS_INTERVAL: &str = "pv-surplus";
const EXPECTED_BASE_PV_AVAILABILITY_KWH: f64 = 2.0;
const STRESS_PV_AVAILABILITY_KWH: f64 = 1.0;

#[derive(Debug, Serialize)]
struct TransformReceipt {
    receipt_schema_version: &'static str,
    transform_id: &'static str,
    transform_version: &'static str,
    base_candidate_id: String,
    transformed_candidate_id: String,
    digest_algorithm: &'static str,
    digest_scope: &'static str,
    base_candidate_sha256: String,
    transformed_candidate_sha256: String,
    base_candidate_bytes: usize,
    transformed_candidate_bytes: usize,
    mutation_path: &'static str,
    pv_resource_interval_id: &'static str,
    pv_resource_before_kwh: f64,
    pv_resource_after_kwh: f64,
    synthetic: bool,
    claim_ceiling: &'static str,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn required_str<'a>(value: &'a Value, path: &str) -> Result<&'a str, Box<dyn Error>> {
    let mut current = value;
    for part in path.split('.') {
        current = current.get(part).ok_or_else(|| {
            io::Error::new(io::ErrorKind::InvalidData, format!("missing candidate field {path}"))
        })?;
    }
    current.as_str().ok_or_else(|| {
        io::Error::new(io::ErrorKind::InvalidData, format!("candidate field {path} must be a string"))
            .into()
    })
}

fn transform(raw: &[u8]) -> Result<(Vec<u8>, TransformReceipt), Box<dyn Error>> {
    let mut candidate: Value = serde_json::from_slice(raw)?;
    let base_id = required_str(&candidate, "candidate_id")?.to_owned();
    if base_id != BASE_CANDIDATE_ID {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("this transform accepts only candidate {BASE_CANDIDATE_ID}; got {base_id}"),
        )
        .into());
    }
    if candidate.get("synthetic").and_then(Value::as_bool) != Some(true) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "PV stress transform requires synthetic=true",
        )
        .into());
    }

    let scenarios = candidate.get_mut("scenarios")
        .and_then(Value::as_array_mut)
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "scenarios must be an array"))?;
    let mut before = None;
    let mut changed = 0_usize;
    for scenario in scenarios.iter_mut() {
        let intervals = scenario.get_mut("intervals")
            .and_then(Value::as_array_mut)
            .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "scenario intervals must be an array"))?;
        for interval in intervals.iter_mut() {
            if interval.get("interval_id").and_then(Value::as_str) != Some(PV_SURPLUS_INTERVAL) {
                continue;
            }
            let resources = interval.get_mut("resource_available_kwh")
                .and_then(Value::as_array_mut)
                .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "resource_available_kwh must be an array"))?;
            for resource in resources.iter_mut() {
                if resource.get("component_id").and_then(Value::as_str) == Some("pv") {
                    let old = resource.get("available_kwh").and_then(Value::as_f64)
                        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "PV availability must be numeric"))?;
                    if (old - EXPECTED_BASE_PV_AVAILABILITY_KWH).abs() > f64::EPSILON {
                        return Err(io::Error::new(
                            io::ErrorKind::InvalidData,
                            format!("expected base PV availability {EXPECTED_BASE_PV_AVAILABILITY_KWH} kWh, got {old}"),
                        ).into());
                    }
                    resource["available_kwh"] = serde_json::json!(STRESS_PV_AVAILABILITY_KWH);
                    before = Some(old);
                    changed += 1;
                }
            }
        }
    }
    if changed != 1 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            format!("expected exactly one PV resource in interval {PV_SURPLUS_INTERVAL}, changed {changed}"),
        ).into());
    }

    candidate["candidate_id"] = serde_json::json!(STRESS_CANDIDATE_ID);
    candidate["generator"]["name"] = serde_json::json!("symtropy-energy-objective-context-transform");
    candidate["generator"]["version"] = serde_json::json!("0.1.0");
    candidate["system_boundary"]["description"] = serde_json::json!(
        "Synthetic single-site essential electric service hub with an explicit first-interval PV availability derate stress scenario."
    );

    let scenario = candidate["scenarios"].as_array_mut()
        .and_then(|items| items.get_mut(0))
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidData, "candidate must contain the frozen scenario"))?;
    let old_description = scenario.get("description").and_then(Value::as_str).unwrap_or_default().to_owned();
    scenario["description"] = serde_json::json!(format!(
        "{old_description} Explicit stress transformation: PV available energy is reduced from 2 kWh to 1 kWh in the surplus interval; the component's 2 kWh cap remains an upper bound, not a forecast."
    ));

    let mut output = serde_json::to_vec_pretty(&candidate)?;
    output.push(b'\n');
    let receipt = TransformReceipt {
        receipt_schema_version: "1",
        transform_id: "pv-availability-50pct-one-interval",
        transform_version: "0.1.0",
        base_candidate_id: base_id,
        transformed_candidate_id: STRESS_CANDIDATE_ID.to_owned(),
        digest_algorithm: "SHA-256",
        digest_scope: "exact_base_candidate_bytes_and_exact_transformed_candidate_bytes",
        base_candidate_sha256: sha256_hex(raw),
        transformed_candidate_sha256: sha256_hex(&output),
        base_candidate_bytes: raw.len(),
        transformed_candidate_bytes: output.len(),
        mutation_path: "scenarios[0].intervals[interval_id=pv-surplus].resource_available_kwh[component_id=pv].available_kwh",
        pv_resource_interval_id: PV_SURPLUS_INTERVAL,
        pv_resource_before_kwh: before.expect("one change required"),
        pv_resource_after_kwh: STRESS_PV_AVAILABILITY_KWH,
        synthetic: true,
        claim_ceiling: "Synthetic PV availability stress case only; no field forecast, economic truth, dynamic physics or safety qualification is implied.",
    };
    Ok((output, receipt))
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 4 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_candidate_pv_stress_transform <base-candidate.json> <transformed-candidate-out.json> <transform-receipt-out.json>",
        )
        .into());
    }
    let base = fs::read(&args[1])?;
    let (transformed, receipt) = transform(&base)?;
    let receipt_bytes = {
        let mut bytes = serde_json::to_vec_pretty(&receipt)?;
        bytes.push(b'\n');
        bytes
    };
    fs::write(PathBuf::from(&args[2]), &transformed)?;
    fs::write(PathBuf::from(&args[3]), &receipt_bytes)?;
    println!("PASS: transformed generated candidate with explicit 50% PV resource derate.");
    println!("base_candidate_sha256={}", receipt.base_candidate_sha256);
    println!("transformed_candidate_sha256={}", receipt.transformed_candidate_sha256);
    println!("transform_receipt_sha256={}", sha256_hex(&receipt_bytes));
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("candidate PV stress transform failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const BASE: &str = include_str!("../../fixtures/energy-system-candidate.synthetic.json");

    #[test]
    fn transform_changes_exactly_the_declared_pv_sample_and_binds_parent_bytes() {
        let (bytes, receipt) = transform(BASE.as_bytes()).expect("frozen base candidate transforms");
        let transformed: Value = serde_json::from_slice(&bytes).expect("transformed output is JSON");
        assert_eq!(required_str(&transformed, "candidate_id").unwrap(), STRESS_CANDIDATE_ID);
        assert_eq!(receipt.base_candidate_sha256, sha256_hex(BASE.as_bytes()));
        assert_eq!(receipt.transformed_candidate_sha256, sha256_hex(&bytes));
        assert_eq!(receipt.pv_resource_before_kwh, 2.0);
        assert_eq!(receipt.pv_resource_after_kwh, 1.0);
        assert_eq!(
            transformed["scenarios"][0]["intervals"][0]["resource_available_kwh"][0]["available_kwh"],
            1.0
        );
    }

    #[test]
    fn refuses_wrong_candidate_or_nonmatching_parent_resource() {
        let mut candidate: Value = serde_json::from_str(BASE).unwrap();
        candidate["candidate_id"] = serde_json::json!("different-candidate");
        assert!(transform(&serde_json::to_vec(&candidate).unwrap()).is_err());

        let mut candidate: Value = serde_json::from_str(BASE).unwrap();
        candidate["scenarios"][0]["intervals"][0]["resource_available_kwh"][0]["available_kwh"] = serde_json::json!(1.5);
        assert!(transform(&serde_json::to_vec(&candidate).unwrap()).is_err());
    }
}
