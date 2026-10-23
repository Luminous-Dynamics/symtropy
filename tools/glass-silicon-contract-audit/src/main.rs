use glass_silicon_contract_audit::{audit_adaptive_decision_campaign, audit_corpus, audit_decision_value_pool, audit_mass_balance, hash_artifact_files};
use serde_json::Value;
use std::env;
use std::fs;
use std::path::Path;
use std::process::ExitCode;

fn read_json(path: &str) -> Result<Value, String> {
    let content = fs::read_to_string(path).map_err(|error| format!("cannot read {path}: {error}"))?;
    serde_json::from_str(&content).map_err(|error| format!("invalid JSON in {path}: {error}"))
}

fn usage() {
    eprintln!(
        "Usage:\n  glass-silicon-contract-audit audit-corpus <schema.json> <corpus.json>\n  glass-silicon-contract-audit check-mass <ledger.json>\n  glass-silicon-contract-audit hash-artifacts <corpus.json> <artifact-id> <directory>"
    );
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("audit-corpus") if args.len() == 4 => {
            let schema = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "benchmark_corpus",
                        "record_id": null,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let corpus = match read_json(&args[3]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "benchmark_corpus",
                        "record_id": null,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_corpus(&schema, &corpus);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("hash-artifacts") if args.len() == 5 => {
            let corpus = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "source_artifact_hash_receipt",
                        "artifact_id": args[3],
                        "files": [],
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no reuse rights or benchmark admission is established."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = hash_artifact_files(&corpus, &args[3], Path::new(&args[4]));
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-decision-value") if args.len() == 3 => {
            let pool = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "synthetic_decision_value_pool",
                        "benchmark_id": null,
                        "pre_measurement_worst_case_regret": null,
                        "measurements": [],
                        "pareto_frontier": [],
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical, model, or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_decision_value_pool(&pool);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("audit-adaptive-campaign") if args.len() == 3 => {
            let pool = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "synthetic_adaptive_decision_campaign",
                        "benchmark_id": null,
                        "pre_measurement_worst_case_regret": null,
                        "adaptive_oracle_worst_case_regret": null,
                        "policy_tree": null,
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical, model, or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_adaptive_decision_campaign(&pool);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        Some("check-mass") if args.len() == 3 => {
            let ledger = match read_json(&args[2]) {
                Ok(value) => value,
                Err(error) => {
                    println!("{}", serde_json::json!({
                        "status": "FAIL",
                        "audit_kind": "elemental_mass_balance",
                        "record_id": null,
                        "mass_unit": null,
                        "residuals": {},
                        "findings": [format!("INPUT_JSON:{error}")],
                        "claim_ceiling": "Input parsing only; no physical or manufacturing claim is validated."
                    }));
                    return ExitCode::from(2);
                }
            };
            let report = audit_mass_balance(&ledger);
            let code = report.exit_code() as u8;
            println!("{}", serde_json::to_string_pretty(&report).expect("report serializes"));
            ExitCode::from(code)
        }
        _ => {
            usage();
            ExitCode::from(64)
        }
    }
}
