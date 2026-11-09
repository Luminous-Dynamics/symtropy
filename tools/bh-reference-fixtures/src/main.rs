use std::{env, fs, process};

use bh_reference_fixtures::{
    KerrQnmSequenceFixture, RadialComparisonConfig, RadialComparisonStatus,
    ReferenceFixture, compare_radial_fixtures,
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
         bh-reference-fixtures compare-radial <left.json> <right.json> <abs-tol> <rel-tol>"
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
        [command, left_path, right_path, abs_tol, rel_tol]
            if command == "compare-radial" =>
        {
            let left = read_radial_fixture(left_path)?;
            let right = read_radial_fixture(right_path)?;
            let absolute_tolerance = abs_tol
                .parse::<f64>()
                .map_err(|error| format!("invalid absolute tolerance {abs_tol}: {error}"))?;
            let relative_tolerance = rel_tol
                .parse::<f64>()
                .map_err(|error| format!("invalid relative tolerance {rel_tol}: {error}"))?;
            let comparison = compare_radial_fixtures(
                &left,
                &right,
                RadialComparisonConfig {
                    absolute_tolerance,
                    relative_tolerance,
                },
            )
            .map_err(|error| error.to_string())?;
            let status = match comparison.status {
                RadialComparisonStatus::Unmatched => "unmatched",
                RadialComparisonStatus::Unresolved => "unresolved",
                RadialComparisonStatus::NumericallyAgreeing => "numerically_agreeing",
                RadialComparisonStatus::NumericallyDisagreeing => "numerically_disagreeing",
            };
            println!(
                "{}",
                json!({
                    "status": status,
                    "comparison": comparison,
                    "interpolation_performed": false
                })
            );
            match comparison.status {
                RadialComparisonStatus::NumericallyAgreeing => Ok(0),
                RadialComparisonStatus::Unmatched => Ok(4),
                RadialComparisonStatus::NumericallyDisagreeing => Ok(5),
                RadialComparisonStatus::Unresolved => Ok(6),
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
