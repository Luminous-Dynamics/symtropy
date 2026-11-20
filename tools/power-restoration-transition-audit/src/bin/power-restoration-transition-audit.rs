use power_restoration_transition_audit::{
    parse_and_audit, TransitionAuditReport,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn read(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn run() -> Result<TransitionAuditReport, String> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let values: Vec<_> = args.collect();
    if values.len() != 5 {
        return Err(format!(
            "Usage: {} <case.json> <proposal.json> <previous-state.json> <successor-state.json> <transition-envelope.json>",
            program.to_string_lossy()
        ));
    }
    let paths: Vec<PathBuf> = values.into_iter().map(PathBuf::from).collect();
    let case_json = read(&paths[0])?;
    let proposal_json = read(&paths[1])?;
    let previous_json = read(&paths[2])?;
    let successor_json = read(&paths[3])?;
    let envelope_json = read(&paths[4])?;

    parse_and_audit(
        &case_json,
        &proposal_json,
        &previous_json,
        &successor_json,
        &envelope_json,
    )
    .map_err(|errors| errors.join("\n"))
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                if report.result_status == "supported_transition_state_delta_verified" {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(3)
                }
            }
            Err(error) => {
                eprintln!("transition report serialization failed: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("transition audit failed: {error}");
            ExitCode::from(1)
        }
    }
}
