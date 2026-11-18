use std::{env, fs, process};

use haps_group_handover::{evaluate_case, parse_case_json};

fn run() -> Result<(), Box<dyn std::error::Error>> {
    let path = env::args()
        .nth(1)
        .ok_or("usage: haps-group-handover <handover-case.json>")?;
    if env::args().nth(2).is_some() {
        return Err("only one input path is accepted".into());
    }

    let bytes = fs::read(&path)?;
    let case = parse_case_json(&bytes)?;
    let report = evaluate_case(&case)?;
    println!("{}", serde_json::to_string_pretty(&report)?);
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("haps-group-handover: {error}");
        process::exit(2);
    }
}
