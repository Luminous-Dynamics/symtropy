use physical_solver_contract::{sha256_hex, validate_receipt_json, validate_request_json};
use serde_json::{Value, json};
use std::{env, fs, process::ExitCode};

fn read_json(path: &str) -> Result<(Vec<u8>, Value), String> {
    let bytes = fs::read(path).map_err(|err| format!("cannot read {path}: {err}"))?;
    let value =
        serde_json::from_slice(&bytes).map_err(|err| format!("invalid JSON in {path}: {err}"))?;
    Ok((bytes, value))
}

fn emit_report(report: Value, valid: bool) -> ExitCode {
    match serde_json::to_string_pretty(&report) {
        Ok(encoded) => println!("{encoded}"),
        Err(err) => {
            eprintln!("cannot encode report: {err}");
            return ExitCode::from(70);
        }
    }
    if valid {
        ExitCode::SUCCESS
    } else {
        ExitCode::from(2)
    }
}

fn usage() -> ExitCode {
    eprintln!(
        "Usage:\n  physical-solver-contract validate-request REQUEST.json\n  physical-solver-contract validate-receipt RECEIPT.json REQUEST.json"
    );
    ExitCode::from(64)
}

fn main() -> ExitCode {
    let args: Vec<String> = env::args().collect();
    match args.get(1).map(String::as_str) {
        Some("validate-request") if args.len() == 3 => {
            let (bytes, request) = match read_json(&args[2]) {
                Ok(v) => v,
                Err(message) => {
                    eprintln!("{message}");
                    return ExitCode::from(2);
                }
            };
            let issues = validate_request_json(&request);
            let valid = issues.is_empty();
            let report = json!({
                "schema_version": "physical-solver-validation-report-v0.1",
                "valid": valid,
                "request_id": request.get("request_id").and_then(Value::as_str),
                "source_revision": request.get("source_revision").and_then(Value::as_str),
                "request_sha256": sha256_hex(&bytes),
                "issues": issues,
                "interpretation": "contract validity only; no solver was executed and no physical result was produced"
            });
            emit_report(report, valid)
        }
        Some("validate-receipt") if args.len() == 4 => {
            let (request_bytes, request) = match read_json(&args[3]) {
                Ok(v) => v,
                Err(message) => {
                    eprintln!("{message}");
                    return ExitCode::from(2);
                }
            };
            let (_, receipt) = match read_json(&args[2]) {
                Ok(v) => v,
                Err(message) => {
                    eprintln!("{message}");
                    return ExitCode::from(2);
                }
            };
            let issues = validate_receipt_json(&receipt, Some(&request), Some(&request_bytes));
            let valid = issues.is_empty();
            let report = json!({
                "schema_version": "physical-solver-validation-report-v0.1",
                "valid": valid,
                "status": receipt.get("status").and_then(Value::as_str),
                "request_id": receipt.get("request_id").and_then(Value::as_str),
                "request_sha256": receipt.get("request_sha256").and_then(Value::as_str),
                "issues": issues,
                "interpretation": "receipt consistency only; this does not attest execution or validate physical truth"
            });
            emit_report(report, valid)
        }
        _ => usage(),
    }
}
