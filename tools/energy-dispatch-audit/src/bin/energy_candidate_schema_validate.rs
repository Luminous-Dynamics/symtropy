//! Validate a candidate JSON instance against the pinned shared JSON Schema.
//!
//! Keeps CI schema validation in the Rust toolchain and reports every violation
//! with its instance path. The schema file is supplied explicitly so the
//! caller controls and records the exact schema revision.

use std::{env, error::Error, fs, io};

fn run() -> Result<i32, Box<dyn Error>> {
    let args = env::args().collect::<Vec<_>>();
    if args.len() != 3 {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "usage: energy_candidate_schema_validate <schema.json> <candidate.json>",
        ).into());
    }

    let schema_bytes = fs::read(&args[1])?;
    let candidate_bytes = fs::read(&args[2])?;
    let schema: serde_json::Value = serde_json::from_slice(&schema_bytes)?;
    let candidate: serde_json::Value = serde_json::from_slice(&candidate_bytes)?;

    let validator = jsonschema::validator_for(&schema)
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, format!("invalid JSON Schema: {error}")))?;
    let errors = validator.iter_errors(&candidate).collect::<Vec<_>>();
    if errors.is_empty() {
        println!("PASS: candidate validates against the supplied JSON Schema.");
        return Ok(0);
    }

    for error in &errors {
        eprintln!("schema violation at {}: {error}", error.instance_path());
    }
    eprintln!("FAIL: {} JSON Schema violation(s).", errors.len());
    Ok(2)
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("candidate JSON Schema validation failed to run: {error}");
            std::process::exit(2);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_and_candidate_fixture_validate() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/energy-system-candidate-v0.1.schema.json"
        )).expect("pinned fixture schema is JSON");
        let candidate: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/energy-system-candidate.synthetic.json"
        )).expect("candidate fixture is JSON");
        let validator = jsonschema::validator_for(&schema).expect("schema is valid");
        assert!(validator.is_valid(&candidate), "{:?}", validator.iter_errors(&candidate).collect::<Vec<_>>());
    }

    #[test]
    fn additional_properties_negative_control_is_rejected() {
        let schema: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/energy-system-candidate-v0.1.schema.json"
        )).expect("pinned fixture schema is JSON");
        let mut candidate: serde_json::Value = serde_json::from_str(include_str!(
            "../../fixtures/energy-system-candidate.synthetic.json"
        )).expect("candidate fixture is JSON");
        candidate["unexpected_field"] = serde_json::json!(true);
        let validator = jsonschema::validator_for(&schema).expect("schema is valid");
        assert!(!validator.is_valid(&candidate));
    }
}
