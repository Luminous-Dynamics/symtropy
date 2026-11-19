#[path = "../multishift.rs"]
pub mod multishift;

use std::env;
use std::fs;
use std::process;

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: multishift-replay <execution-case.json>");
        process::exit(2);
    };
    let input = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    });
    let case: multishift::ExecutionCase = serde_json::from_str(&input).unwrap_or_else(|error| {
        eprintln!("execution case JSON decode error: {error}");
        process::exit(2);
    });
    match multishift::validate(&case) {
        Ok(report) => {
            println!("{}", serde_json::to_string_pretty(&report).expect("report serialization"));
        }
        Err(errors) => {
            eprintln!("execution ledger rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            process::exit(1);
        }
    }
}
