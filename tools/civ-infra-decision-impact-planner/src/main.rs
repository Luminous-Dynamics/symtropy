use civ_infra_decision_impact_planner::{plan, sha256_hex, PlannerInput};
use std::env;
use std::fs;
use std::process::ExitCode;

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("civ-infra-decision-impact-planner: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let args: Vec<String> = env::args().collect();
    if args.len() < 2 || args.len() > 3 {
        return Err(
            "usage: civ-infra-decision-impact-planner INPUT.json [REPORT.json]".to_owned(),
        );
    }

    let input_path = &args[1];
    let input_bytes = fs::read(input_path)
        .map_err(|err| format!("cannot read input {input_path:?}: {err}"))?;
    let input_sha256 = sha256_hex(&input_bytes);
    let input: PlannerInput = serde_json::from_slice(&input_bytes)
        .map_err(|err| format!("invalid planner input JSON: {err}"))?;
    let report = plan(&input, &input_sha256)?;
    let report_bytes = serde_json::to_vec_pretty(&report)
        .map_err(|err| format!("cannot serialize report: {err}"))?;

    if let Some(output_path) = args.get(2) {
        fs::write(output_path, &report_bytes)
            .map_err(|err| format!("cannot write report {output_path:?}: {err}"))?;
        println!(
            "report_written={} input_sha256={} states={} actions={}",
            output_path,
            input_sha256,
            report.state_count,
            report.actions_ranked.len()
        );
    } else {
        println!(
            "{}",
            String::from_utf8(report_bytes)
                .map_err(|err| format!("report is not valid UTF-8: {err}"))?
        );
    }
    Ok(())
}
