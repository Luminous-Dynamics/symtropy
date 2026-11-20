use power_restoration_benchmark_case::{canonical_case_sha256, validate_json};
use std::env;
use std::fs;
use std::process;

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: power-restoration-case-validate <case.json>");
        process::exit(2);
    };
    let input = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    });
    match validate_json(&input) {
        Ok(case) => {
            let digest = canonical_case_sha256(&case).expect("canonical JSON serialization");
            let case_id = serde_json::to_string(&case.case_id).expect("case ID JSON string");
            let maturity = serde_json::to_string(&case.maturity).expect("case maturity JSON string");
            println!(
                "{{\"caseId\":{},\"caseMaturity\":{},\"caseSha256\":\"{}\",\"status\":\"case_contract_valid\"}}",
                case_id, maturity, digest
            );
        }
        Err(errors) => {
            eprintln!("benchmark case rejected:");
            for error in errors { eprintln!("- {error}"); }
            process::exit(1);
        }
    }
}
