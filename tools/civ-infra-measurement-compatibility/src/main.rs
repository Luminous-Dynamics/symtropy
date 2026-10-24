use civ_infra_measurement_compatibility::evaluate_bytes;
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("civ-infra-measurement-compatibility: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        return Err("usage: civ-infra-measurement-compatibility INPUT.json [REPORT.json]".to_owned());
    }

    let input_path = &args[1];
    let bytes = fs::read(input_path).map_err(|err| format!("cannot read input {input_path:?}: {err}"))?;
    let report = evaluate_bytes(&bytes)?;
    let input_sha256 = report.input_sha256.clone();
    let report_bytes = serde_json::to_vec_pretty(&report).map_err(|err| format!("cannot serialize report: {err}"))?;

    if let Some(output_path) = args.get(2) {
        fs::write(output_path, &report_bytes).map_err(|err| format!("cannot write report {output_path:?}: {err}"))?;
        println!(
            "report_written={} input_sha256={} states={} outcomes={} edges={}",
            output_path, input_sha256, report.state_count, report.outcome_count, report.compatibility_edge_count
        );
    } else {
        println!(
            "{}",
            String::from_utf8(report_bytes).map_err(|err| format!("report is not valid UTF-8: {err}"))?
        );
    }
    Ok(())
}
