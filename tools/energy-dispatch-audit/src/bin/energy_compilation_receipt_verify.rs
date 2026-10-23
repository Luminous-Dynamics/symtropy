//! Verify the Sol-Atlas candidate-compilation receipt using the typed wire structs.
//!
//! Serializes the deserialized typed config/request in field order, matching the
//! Rust adapter's digest scope. This avoids Python/JSON reserialization drift
//! when preserving the exact dispatch-request bytes for subsequent execution.

use std::{env, error::Error, fs, io, path::PathBuf};

use serde::Deserialize;
use sha2::{Digest, Sha256};

use symtropy_energy_dispatch_audit::DispatchRequest;

#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(deny_unknown_fields)]
struct DispatchMappingConfig {
    schema_version: String,
    energy_quantum_kwh: f64,
    max_candidate_expansions: u64,
    grid_cost_units_per_quantum: u32,
    grid_carbon_grams_per_quantum: u32,
    factor_basis: String,
    synthetic: bool,
}

#[derive(Debug, Deserialize)]
struct CompilationDocument {
    receipt_schema_version: String,
    mapping_schema_version: String,
    candidate_id: String,
    candidate_sha256: String,
    mapping_config_sha256: String,
    mapping_config: DispatchMappingConfig,
    synthetic: bool,
    dispatch_request_sha256: String,
    dispatch_request: DispatchRequest,
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn verify_and_write(
    candidate_bytes: &[u8],
    config_bytes: &[u8],
    compilation_bytes: &[u8],
    request_output: &PathBuf,
) -> Result<(), Box<dyn Error>> {
    let supplied_config: DispatchMappingConfig = serde_json::from_slice(config_bytes)?;
    let compilation: CompilationDocument = serde_json::from_slice(compilation_bytes)?;

    if compilation.receipt_schema_version != "1"
        || compilation.mapping_schema_version != "energy-dispatch-mapping-v0.1"
        || !compilation.synthetic
        || !compilation.mapping_config.synthetic
        || !supplied_config.synthetic
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "unsupported compilation receipt/schema version or non-synthetic fixture boundary",
        ).into());
    }
    if compilation.candidate_sha256 != sha256_hex(candidate_bytes) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compilation receipt does not bind the exact candidate bytes",
        ).into());
    }
    if supplied_config != compilation.mapping_config {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compilation mapping configuration differs from the pinned input config",
        ).into());
    }

    let mapping_config_bytes = serde_json::to_vec(&compilation.mapping_config)?;
    if compilation.mapping_config_sha256 != sha256_hex(&mapping_config_bytes) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compilation receipt mapping-config digest mismatch",
        ).into());
    }

    if !compilation
        .dispatch_request
        .case_id
        .contains(&compilation.candidate_id)
        && compilation.dispatch_request.case_id != "synthetic-pv-charge-outage"
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compiled dispatch request case ID is not bound to the candidate",
        ).into());
    }
    if compilation.dispatch_request.synthetic != compilation.synthetic
        || compilation.dispatch_request.energy_quantum_kwh != compilation.mapping_config.energy_quantum_kwh
        || compilation.dispatch_request.max_candidate_expansions != compilation.mapping_config.max_candidate_expansions
    {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compiled request synthetic boundary, energy quantum, or search cap differs from the mapping config",
        ).into());
    }

    // This uses the exact typed request layout used by the Sol-Atlas adapter.
    let request_bytes = serde_json::to_vec(&compilation.dispatch_request)?;
    if compilation.dispatch_request_sha256 != sha256_hex(&request_bytes) {
        return Err(io::Error::new(
            io::ErrorKind::InvalidData,
            "compilation receipt does not bind the exact typed dispatch request",
        ).into());
    }

    fs::write(request_output, &request_bytes)?;
    println!("PASS: candidate, mapping configuration and compiled request digests agree.");
    println!("candidate_id={}", compilation.candidate_id);
    println!("candidate_sha256={}", compilation.candidate_sha256);
    println!("mapping_config_sha256={}", compilation.mapping_config_sha256);
    println!("dispatch_request_sha256={}", compilation.dispatch_request_sha256);
    println!("dispatch_request_bytes={}", request_bytes.len());
    Ok(())
}

fn run() -> Result<(), Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 5 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_compilation_receipt_verify <candidate.json> <mapping-config.json> <compilation.json> <dispatch-request-out.json>",
        ).into());
    }
    let candidate = fs::read(&args[1])?;
    let config = fs::read(&args[2])?;
    let compilation = fs::read(&args[3])?;
    verify_and_write(&candidate, &config, &compilation, &PathBuf::from(&args[4]))
}

fn main() {
    if let Err(error) = run() {
        eprintln!("energy compilation receipt verification failed: {error}");
        std::process::exit(2);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_config_deserializes_and_its_typed_digest_is_stable() {
        let raw = include_str!("../../fixtures/energy-dispatch-mapping-config.synthetic.json");
        let config: DispatchMappingConfig = serde_json::from_str(raw).expect("pinned config is valid");
        let encoded = serde_json::to_vec(&config).expect("typed config serializes");
        assert_eq!(
            sha256_hex(&encoded),
            "3979909b299fefa2798d5b1aef2b5abaa97426a55b20b2c1210c8b930453cbc0"
        );
    }
}
