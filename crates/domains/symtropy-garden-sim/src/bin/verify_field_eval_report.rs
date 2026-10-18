// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

//! Validate a field-study cohort manifest and print a machine-readable report.

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use symtropy_garden_sim::field_eval_report::{
    FieldEvalPreflightStatusV1, FieldStudyManifestV1, preflight_field_study_v1,
};

fn main() -> ExitCode {
    let mut args = std::env::args().skip(1);
    let input = args.next().map(PathBuf::from).unwrap_or_else(|| {
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures/field_eval_report.v1.json")
    });
    if args.next().is_some() {
        eprintln!("usage: verify-field-eval-report [MANIFEST.json]");
        return ExitCode::from(2);
    }

    let bytes = match fs::read(&input) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", input.display());
            return ExitCode::from(2);
        }
    };
    let manifest: FieldStudyManifestV1 = match serde_json::from_slice(&bytes) {
        Ok(manifest) => manifest,
        Err(error) => {
            eprintln!("invalid field-study manifest {}: {error}", input.display());
            return ExitCode::from(2);
        }
    };
    let report = preflight_field_study_v1(&manifest);
    let output = match serde_json::to_string_pretty(&report) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("cannot serialize field-study report: {error}");
            return ExitCode::from(2);
        }
    };
    println!("{output}");

    match report.status {
        FieldEvalPreflightStatusV1::Calculated => ExitCode::SUCCESS,
        FieldEvalPreflightStatusV1::Blocked => ExitCode::FAILURE,
    }
}
