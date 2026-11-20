use power_restoration_generalization_audit::{parse_and_audit, SplitAuditReport};
use std::env;
use std::fs;
use std::path::PathBuf;
use std::process::ExitCode;

fn run() -> Result<SplitAuditReport, String> {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let values: Vec<_> = args.collect();
    if values.len() != 1 {
        return Err(format!(
            "Usage: {} <split-audit-input.json>",
            program.to_string_lossy()
        ));
    }
    let path = PathBuf::from(&values[0]);
    let input = fs::read_to_string(&path)
        .map_err(|error| format!("cannot read {}: {error}", path.display()))?;
    parse_and_audit(&input).map_err(|errors| errors.join("\n"))
}

fn main() -> ExitCode {
    match run() {
        Ok(report) => match serde_json::to_string_pretty(&report) {
            Ok(json) => {
                println!("{json}");
                if report.leakage_finding_count == 0 {
                    ExitCode::SUCCESS
                } else {
                    ExitCode::from(3)
                }
            }
            Err(error) => {
                eprintln!("split report serialization failed: {error}");
                ExitCode::from(2)
            }
        },
        Err(error) => {
            eprintln!("split audit failed: {error}");
            ExitCode::from(1)
        }
    }
}
