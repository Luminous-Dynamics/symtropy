use serde_json::json;
use shipyard_benchmark_auditor::{audit_bytes, sha256_hex};
use std::{env, fs, process::ExitCode};

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: shipyard-benchmark-auditor MANIFEST.json REPORT.json");
        return ExitCode::from(2);
    }
    let manifest = match fs::read(&args[1]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}", json!({"status":"invalid","diagnostics":[format!("manifest: {error}")]}));
            return ExitCode::from(2);
        }
    };
    let report = match fs::read(&args[2]) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("{}", json!({"status":"invalid","diagnostics":[format!("report: {error}")]}));
            return ExitCode::from(2);
        }
    };
    match audit_bytes(&manifest, &report) {
        Ok(summary) => {
            println!("{}", json!({
                "status":"valid",
                "manifest_sha256":sha256_hex(&manifest),
                "summary":{
                    "case_count":summary.case_count,
                    "verified_feasible_count":summary.verified_feasible_count,
                    "verified_infeasible_count":summary.verified_infeasible_count,
                    "unknown_count":summary.unknown_count,
                    "timeout_count":summary.timeout_count,
                    "not_run_count":summary.not_run_count,
                    "invalid_output_count":summary.invalid_output_count,
                    "false_feasible_count":summary.false_feasible_count,
                    "false_infeasible_count":summary.false_infeasible_count
                }
            }));
            ExitCode::SUCCESS
        }
        Err(diagnostics) => {
            eprintln!("{}", json!({"status":"invalid","diagnostics":diagnostics}));
            ExitCode::from(1)
        }
    }
}
