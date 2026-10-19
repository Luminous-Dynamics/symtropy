// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;
use symtropy_garden_sim::field_eval_report::{
    FieldEvalReportAssessmentV1, FieldEvalReportDispositionV1, FieldEvalReportInputV1,
    assess_field_eval_report_v1, sha256_hex,
};

#[derive(serde::Serialize)]
struct ReportEnvelopeV1 {
    schema_version: u16,
    input_sha256: String,
    assessment: FieldEvalReportAssessmentV1,
}

fn main() -> ExitCode {
    let mut input_path: Option<PathBuf> = None;
    let mut report_path: Option<PathBuf> = None;
    let mut args = std::env::args().skip(1);

    while let Some(argument) = args.next() {
        match argument.as_str() {
            "--input" => {
                let Some(path) = args.next() else {
                    eprintln!("--input requires a JSON file path");
                    return ExitCode::from(2);
                };
                input_path = Some(PathBuf::from(path));
            }
            "--report-json" => {
                let Some(path) = args.next() else {
                    eprintln!("--report-json requires a file path");
                    return ExitCode::from(2);
                };
                report_path = Some(PathBuf::from(path));
            }
            "--help" | "-h" => {
                println!("verify-field-eval-report --input FILE [--report-json FILE]");
                return ExitCode::SUCCESS;
            }
            unknown => {
                eprintln!("unknown argument: {unknown}");
                return ExitCode::from(2);
            }
        }
    }

    let Some(input_path) = input_path else {
        eprintln!("missing required --input FILE");
        return ExitCode::from(2);
    };
    let bytes = match fs::read(&input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", input_path.display());
            return ExitCode::from(2);
        }
    };
    let input: FieldEvalReportInputV1 = match serde_json::from_slice(&bytes) {
        Ok(input) => input,
        Err(error) => {
            eprintln!("invalid field-evaluation report JSON: {error}");
            return ExitCode::from(2);
        }
    };
    let assessment = assess_field_eval_report_v1(&input);
    let envelope = ReportEnvelopeV1 {
        schema_version: 1,
        input_sha256: sha256_hex(&bytes),
        assessment,
    };
    let output = match serde_json::to_string_pretty(&envelope) {
        Ok(output) => output,
        Err(error) => {
            eprintln!("cannot serialize verification report: {error}");
            return ExitCode::from(2);
        }
    };
    if let Some(path) = report_path {
        if let Err(error) = fs::write(&path, format!("{output}\n")) {
            eprintln!("cannot write {}: {error}", path.display());
            return ExitCode::from(2);
        }
    }
    println!("{output}");
    match envelope.assessment.disposition {
        FieldEvalReportDispositionV1::Calculated | FieldEvalReportDispositionV1::NotAssessable => {
            ExitCode::SUCCESS
        }
        FieldEvalReportDispositionV1::Blocked => ExitCode::FAILURE,
    }
}
