use std::env;
use std::fs;
use std::path::Path;

use safrb_cargo_receipt_ledger::{report_json, synthetic_cases, validate_and_project, HORIZON_HOURS};

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let cases = synthetic_cases();
    let projections = cases.iter().map(validate_and_project).collect::<Result<Vec<_>, _>>()?;

    match args.as_slice() {
        [flag] if flag == "--check" => {
            if cases.len() != 9 || projections.iter().any(|projection| projection.hourly.len() != HORIZON_HOURS as usize) {
                return Err("cargo receipt ledger corpus shape mismatch".into());
            }
            println!("SAFRB cargo receipt ledger PASS: {} synthetic cases, {} hourly boundary observations per case", cases.len(), HORIZON_HOURS);
            Ok(())
        }
        [flag] if flag == "--benchmark" => {
            println!("{}", report_json(&cases, &projections));
            Ok(())
        }
        [flag, output_dir] if flag == "--emit" => {
            let directory = Path::new(output_dir);
            fs::create_dir_all(directory).map_err(|error| format!("cannot create output directory: {error}"))?;
            let planner = cases.iter().map(|case| case.planner_json()).collect::<Vec<_>>().join("\n") + "\n";
            let oracle = cases.iter().zip(&projections).map(|(case, projection)| case.oracle_json(projection)).collect::<Vec<_>>().join("\n") + "\n";
            fs::write(directory.join("planner-manifests.jsonl"), planner).map_err(|error| format!("cannot write planner manifests: {error}"))?;
            fs::write(directory.join("oracle-ledger-truth.jsonl"), oracle).map_err(|error| format!("cannot write oracle truth: {error}"))?;
            fs::write(directory.join("cargo-ledger-report.json"), report_json(&cases, &projections)).map_err(|error| format!("cannot write report: {error}"))?;
            println!("wrote {} planner manifests, {} oracle rows and report to {}", cases.len(), cases.len(), directory.display());
            Ok(())
        }
        [] => {
            for case in &cases { println!("{}", case.planner_json()); }
            Ok(())
        }
        _ => Err("usage: safrb-cargo-receipt-ledger [--check | --benchmark | --emit OUTPUT_DIRECTORY]".into()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
