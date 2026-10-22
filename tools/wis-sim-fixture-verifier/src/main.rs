use std::{env, process};

use wis_sim_fixture_verifier::{verify_files, verify_run_report_files};

fn print_receipt(receipt: serde_json::Value) -> bool {
    let qualified = receipt.get("status").and_then(serde_json::Value::as_str)
        == Some("RUN_REPORT_ASSERTIONS_PASS")
        || receipt.get("status").and_then(serde_json::Value::as_str)
        == Some("STRUCTURAL_FIXTURE_MATCH_NOT_SIMULATION");
    match serde_json::to_string_pretty(&receipt) {
        Ok(output) => println!("{}", output),
        Err(error) => {
            eprintln!("failed to serialize receipt: {}", error);
            process::exit(2);
        }
    }
    qualified
}

fn main() {
    let args: Vec<String> = env::args().collect();
    let result = match args.get(1).map(String::as_str) {
        Some("verify") if args.len() == 5 => {
            verify_files(&args[2], &args[3], &args[4])
        }
        Some("verify-run") if args.len() == 6 => {
            verify_run_report_files(&args[2], &args[3], &args[4], &args[5])
        }
        _ => {
            eprintln!("usage:");
            eprintln!("  wis-sim-fixture-verifier verify <fixture.jsonl> <manifest.json> <scenario-schema.json>");
            eprintln!("  wis-sim-fixture-verifier verify-run <fixture.jsonl> <manifest.json> <scenario-schema.json> <run-report.json>");
            process::exit(2);
        }
    };

    match result {
        Ok(receipt) => {
            if !print_receipt(receipt) {
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("verification rejected: {}", error);
            process::exit(2);
        }
    }
}
