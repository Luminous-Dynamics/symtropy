use std::{env, fs, process};

use bh_reference_fixtures::{
    CompatibilityStatus, KerrQnmSequenceFixture, ReferenceFixture, assess_compatibility,
};
use serde_json::json;

fn read_radial_fixture(path: &str) -> Result<ReferenceFixture, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read radial fixture {path}: {error}"))?;
    let fixture: ReferenceFixture = serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse radial fixture {path}: {error}"))?;
    fixture
        .validate()
        .map_err(|error| format!("invalid radial fixture {path}: {error}"))?;
    Ok(fixture)
}

fn read_qnm_sequence(path: &str) -> Result<KerrQnmSequenceFixture, String> {
    let source = fs::read_to_string(path)
        .map_err(|error| format!("cannot read QNM sequence {path}: {error}"))?;
    let sequence: KerrQnmSequenceFixture = serde_json::from_str(&source)
        .map_err(|error| format!("cannot parse QNM sequence {path}: {error}"))?;
    sequence
        .validate()
        .map_err(|error| format!("invalid QNM sequence {path}: {error}"))?;
    Ok(sequence)
}

fn print_usage() {
    eprintln!(
        "Usage:\n  bh-reference-fixtures validate-radial <fixture.json>\n  \
         bh-reference-fixtures validate-qnm <sequence.json>\n  \
         bh-reference-fixtures compare-radial <left.json> <right.json>"
    );
}

fn run(args: &[String]) -> Result<i32, String> {
    match args {
        [command, path] if command == "validate-radial" => {
            let fixture = read_radial_fixture(path)?;
            println!(
                "{}",
                json!({
                    "status": "valid",
                    "schema_version": fixture.schema_version,
                    "fixture_id": fixture.fixture_id,
                    "radial_samples": fixture.data.radial_samples.len()
                })
            );
            Ok(0)
        }
        [command, path] if command == "validate-qnm" => {
            let sequence = read_qnm_sequence(path)?;
            println!(
                "{}",
                json!({
                    "status": "valid",
                    "schema_version": sequence.schema_version,
                    "sequence_id": sequence.sequence_id,
                    "family_classification": sequence.family_classification,
                    "modes": sequence.modes.len()
                })
            );
            Ok(0)
        }
        [command, left_path, right_path] if command == "compare-radial" => {
            let left = read_radial_fixture(left_path)?;
            let right = read_radial_fixture(right_path)?;
            let compatibility = assess_compatibility(&left, &right);
            let status = match compatibility.status {
                CompatibilityStatus::Matched => "matched",
                CompatibilityStatus::Unmatched => "unmatched",
            };
            println!(
                "{}",
                json!({
                    "status": status,
                    "differing_fields": compatibility.differing_fields,
                    "numerical_comparison_performed": false
                })
            );
            match compatibility.status {
                CompatibilityStatus::Matched => Ok(0),
                CompatibilityStatus::Unmatched => Ok(4),
            }
        }
        _ => {
            print_usage();
            Err("invalid command or argument count".to_owned())
        }
    }
}

fn main() {
    let args: Vec<String> = env::args().skip(1).collect();
    let code = match run(&args) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            2
        }
    };
    process::exit(code);
}
