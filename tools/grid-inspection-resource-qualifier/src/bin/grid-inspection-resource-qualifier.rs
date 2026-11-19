use grid_inspection_resource_qualifier::{Case, qualify};
use std::env;
use std::fs;
use std::process;

fn main() {
    let Some(path) = env::args().nth(1) else {
        eprintln!("usage: grid-inspection-resource-qualifier <case.json>");
        process::exit(2);
    };
    let input = fs::read_to_string(&path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    });
    let case: Case = serde_json::from_str(&input).unwrap_or_else(|error| {
        eprintln!("case JSON decode error: {error}");
        process::exit(2);
    });
    match qualify(&case) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("report serialization")),
        Err(errors) => {
            eprintln!("resource-time certificate rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            process::exit(1);
        }
    }
}
