// SPDX-License-Identifier: AGPL-3.0-or-later

use mucoll_detector_evidence_validator::{ValidationInputs, validate_package};
use std::env;
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err((code, message)) => {
            eprintln!("{message}");
            ExitCode::from(code)
        }
    }
}

fn run() -> Result<(), (u8, String)> {
    let mut args = env::args().skip(1);
    let mut report = None;
    let mut reco_input = None;
    let mut submission = None;
    let mut generation_input = None;
    let mut fragments_dir = None;
    let mut producer_dir = None;
    let mut aggregator_source = None;
    let mut sample_label = None;

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--report" => report = Some(next_path(&mut args, "--report")?),
            "--reco-input" => reco_input = Some(next_path(&mut args, "--reco-input")?),
            "--submission" => submission = Some(next_path(&mut args, "--submission")?),
            "--generation-input" => {
                generation_input = Some(next_path(&mut args, "--generation-input")?)
            }
            "--fragments-dir" => fragments_dir = Some(next_path(&mut args, "--fragments-dir")?),
            "--producer-dir" => producer_dir = Some(next_path(&mut args, "--producer-dir")?),
            "--aggregator-source" => {
                aggregator_source = Some(next_path(&mut args, "--aggregator-source")?)
            }
            "--sample-label" => {
                sample_label = Some(
                    args.next()
                        .ok_or_else(|| (64, "--sample-label requires a value".to_owned()))?,
                )
            }
            "--help" | "-h" => {
                println!(
                    "mucoll-detector-evidence-validator \\
                     --report metrics.json --reco-input reco.root \\
                     --submission submission.json --generation-input gen.root \\
                     --fragments-dir DIR --producer-dir DIR \\
                     --aggregator-source aggregate_metrics.py --sample-label LABEL\n\
                     Validates evidence package provenance only. Physics comparison remains not_evaluated."
                );
                return Ok(());
            }
            unknown => {
                return Err((64, format!("unknown argument {unknown:?}; use --help")));
            }
        }
    }

    let inputs = ValidationInputs {
        report: required(report, "--report")?,
        reco_input: required(reco_input, "--reco-input")?,
        submission: required(submission, "--submission")?,
        generation_input: required(generation_input, "--generation-input")?,
        fragments_dir: required(fragments_dir, "--fragments-dir")?,
        producer_dir: required(producer_dir, "--producer-dir")?,
        aggregator_source: required(aggregator_source, "--aggregator-source")?,
        sample_label: sample_label.ok_or_else(|| (64, "missing --sample-label".to_owned()))?,
    };

    match validate_package(&inputs) {
        Ok(receipt) => {
            println!(
                "{}",
                serde_json::to_string_pretty(&receipt)
                    .map_err(|error| (70, format!("cannot serialize receipt: {error}")))?
            );
            Ok(())
        }
        Err(errors) => {
            let output = serde_json::json!({
                "validator": "mucoll-detector-evidence-validator",
                "validator_version": env!("CARGO_PKG_VERSION"),
                "evidence_package_status": "rejected",
                "physics_reference_status": "not_evaluated",
                "qualification_decision": "reject",
                "errors": errors
            });
            Err((
                2,
                serde_json::to_string_pretty(&output)
                    .unwrap_or_else(|_| "evidence validation failed".to_owned()),
            ))
        }
    }
}

fn next_path(
    args: &mut impl Iterator<Item = String>,
    option: &str,
) -> Result<PathBuf, (u8, String)> {
    args.next()
        .map(PathBuf::from)
        .ok_or_else(|| (64, format!("{option} requires a path")))
}

fn required(value: Option<PathBuf>, option: &str) -> Result<PathBuf, (u8, String)> {
    value.ok_or_else(|| (64, format!("missing {option}")))
}
