use civ_infra_heldout_calibration::evaluate_bytes;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("civ-infra-heldout-calibration: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        return Err("usage: civ-infra-heldout-calibration INPUT.json [REPORT.json]".to_owned());
    }
    let input_path = &args[1];
    let bytes = fs::read(input_path)
        .map_err(|error| format!("cannot read input {input_path:?}: {error}"))?;
    let report = evaluate_bytes(&bytes)?;
    let input_sha256 = report.input_sha256.clone();
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|error| format!("cannot serialize report: {error}"))?;

    if let Some(output_path) = args.get(2) {
        fs::write(output_path, &report_bytes)
            .map_err(|error| format!("cannot write report {output_path:?}: {error}"))?;
        println!(
            "report_written={} input_sha256={} calibration_records={} holdout_records={} false_robust_feasible={}",
            output_path,
            input_sha256,
            report.calibration_record_count,
            report.holdout_record_count,
            report.false_robust_feasible_count
        );
    } else {
        println!(
            "{}",
            String::from_utf8(report_bytes)
                .map_err(|error| format!("report is not valid UTF-8: {error}"))?
        );
    }
    Ok(())
}
