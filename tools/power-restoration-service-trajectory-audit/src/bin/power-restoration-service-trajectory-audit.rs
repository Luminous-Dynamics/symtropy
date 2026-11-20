use power_restoration_service_trajectory_audit::{
    ServiceTrajectoryAuditReport, parse_and_audit,
};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn read(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn run() -> Result<ServiceTrajectoryAuditReport, String> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let values: Vec<_> = args.collect();
    if values.len() != 3 {
        return Err(format!(
            "Usage: {} <service-trajectory.json> <run-receipt.json> <benchmark-case.json>",
            program.to_string_lossy()
        ));
    }
    let paths: Vec<PathBuf> = values.into_iter().map(PathBuf::from).collect();
    let trajectory = read(&paths[0])?;
    let receipt = read(&paths[1])?;
    let case = read(&paths[2])?;
    parse_and_audit(&trajectory, &receipt, &case)
        .map_err(|errors| errors.join("\n"))
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                if matches!(
                    report.result_status.as_str(),
                    "modelled_service_and_score_reconciled"
                        | "measured_service_and_score_reconciled"
                ) {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(3)
                }
            }
            Err(error) => {
                eprintln!("service trajectory report serialization failed: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("service trajectory audit failed:\n{error}");
            ExitCode::from(1)
        }
    }
}
