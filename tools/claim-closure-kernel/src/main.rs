use engineering_claim_closure::{
    ClaimClosureInput, ClosureDisposition, ClosureReport, evaluate, verify_report,
};
use std::env;
use std::fs;
use std::process::ExitCode;

fn run() -> Result<ExitCode, String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    match arguments.as_slice() {
        [command, input_path] if command == "evaluate" => {
            let input_bytes = fs::read(input_path)
                .map_err(|error| format!("could not read input {input_path}: {error}"))?;
            let input: ClaimClosureInput = serde_json::from_slice(&input_bytes)
                .map_err(|error| format!("input JSON is invalid: {error}"))?;
            let report = evaluate(&input)?;
            println!(
                "{}",
                serde_json::to_string_pretty(&report)
                    .map_err(|error| format!("report serialization failed: {error}"))?
            );
            Ok(match report.disposition {
                ClosureDisposition::ClosedForIndependentReview => ExitCode::SUCCESS,
                ClosureDisposition::Blocked => ExitCode::from(2),
            })
        }
        [command, input_path, report_path] if command == "verify" => {
            let input_bytes = fs::read(input_path)
                .map_err(|error| format!("could not read input {input_path}: {error}"))?;
            let report_bytes = fs::read(report_path)
                .map_err(|error| format!("could not read report {report_path}: {error}"))?;
            let input: ClaimClosureInput = serde_json::from_slice(&input_bytes)
                .map_err(|error| format!("input JSON is invalid: {error}"))?;
            let report: ClosureReport = serde_json::from_slice(&report_bytes)
                .map_err(|error| format!("report JSON is invalid: {error}"))?;
            if verify_report(&input, &report)? {
                println!("VERIFIED: report exactly matches independent recomputation");
                Ok(ExitCode::SUCCESS)
            } else {
                eprintln!("REJECTED: report does not match independent recomputation");
                Ok(ExitCode::from(3))
            }
        }
        _ => Err(
            "usage: claim-closure-kernel evaluate <input.json> | verify <input.json> <report.json>"
                .to_owned(),
        ),
    }
}

fn main() -> ExitCode {
    match run() {
        Ok(code) => code,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(1)
        }
    }
}
