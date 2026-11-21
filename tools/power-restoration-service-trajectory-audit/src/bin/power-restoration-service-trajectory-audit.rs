use power_restoration_service_trajectory_audit::{
    CascadeEndpointMapping, ServiceTrajectoryAuditReport, aggregate_cascade_endpoint_availability,
    parse_and_audit,
};
use serde::Serialize;
use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

#[derive(Serialize)]
struct CliReport {
    audit: ServiceTrajectoryAuditReport,
    cascade_endpoint_availability: Option<BTreeMap<String, Vec<f64>>>,
}

fn read(path: &PathBuf) -> Result<String, String> {
    fs::read_to_string(path).map_err(|error| format!("cannot read {}: {error}", path.display()))
}

fn run() -> Result<CliReport, String> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let values: Vec<_> = args.collect();
    if !(values.len() == 3 || values.len() == 4) {
        return Err(format!(
            "Usage: {} <service-trajectory.json> <run-receipt.json> <benchmark-case.json> [cascade-endpoint-mapping.json]",
            program.to_string_lossy()
        ));
    }
    let paths: Vec<PathBuf> = values.into_iter().map(PathBuf::from).collect();
    let trajectory = read(&paths[0])?;
    let receipt = read(&paths[1])?;
    let case_json = read(&paths[2])?;
    let report =
        parse_and_audit(&trajectory, &receipt, &case_json).map_err(|errors| errors.join("\n"))?;

    let cascade_endpoint_availability = if paths.len() == 4 {
        let case: power_restoration_benchmark_case::BenchmarkCase =
            serde_json::from_str(&case_json)
                .map_err(|error| format!("benchmark case JSON decode error: {error}"))?;
        let mapping_json = read(&paths[3])?;
        let mapping: CascadeEndpointMapping = serde_json::from_str(&mapping_json)
            .map_err(|error| format!("cascade mapping JSON decode error: {error}"))?;
        Some(
            aggregate_cascade_endpoint_availability(&report, &case, &mapping)
                .map_err(|errors| errors.join("\n"))?,
        )
    } else {
        None
    };
    Ok(CliReport {
        audit: report,
        cascade_endpoint_availability,
    })
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                if matches!(
                    report.audit.result_status.as_str(),
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
