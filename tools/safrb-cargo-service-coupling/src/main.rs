use std::env;
use std::fs;
use std::path::Path;

use safrb_cargo_service_coupling::{build_corpus, evaluate_coupling, report_json};

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let cases = build_corpus()?;
    let report = evaluate_coupling(&cases)?;
    match args.as_slice() {
        [flag] if flag == "--check" => {
            if report.case_count != 12 || report.hourly_interval_count != 864
                || report.required_predicate_results != 1_728
                || report.hourly_guaranteed_count == 0
                || report.hourly_failed_count == 0 {
                return Err("cargo-to-service coupling corpus did not exercise expected states".into());
            }
            println!(
                "SAFRB cargo/service coupling PASS: {} cases, {} hourly intervals, {} hourly guarantees, {} failures and {} indeterminate",
                report.case_count, report.hourly_interval_count, report.hourly_guaranteed_count,
                report.hourly_failed_count, report.hourly_indeterminate_count
            );
            Ok(())
        }
        [flag] if flag == "--benchmark" => {
            println!("{}", report_json(&cases)?);
            Ok(())
        }
        [flag, output_dir] if flag == "--emit" => {
            let directory = Path::new(output_dir);
            fs::create_dir_all(directory).map_err(|error| format!("cannot create output directory: {error}"))?;
            let planner = cases.iter().map(|case| case.planner_json()).collect::<Vec<_>>().join("\n") + "\n";
            let oracle = cases.iter().map(|case| case.oracle_json()).collect::<Vec<_>>().join("\n") + "\n";
            fs::write(directory.join("planner-observations.jsonl"), planner).map_err(|error| format!("cannot write planner output: {error}"))?;
            fs::write(directory.join("oracle-truth.jsonl"), oracle).map_err(|error| format!("cannot write oracle output: {error}"))?;
            fs::write(directory.join("coupling-report.json"), report_json(&cases)?).map_err(|error| format!("cannot write report: {error}"))?;
            println!("wrote {} planner observations and oracle truth rows to {}", cases.len(), directory.display());
            Ok(())
        }
        [] => {
            for case in &cases { println!("{}", case.planner_json()); }
            Ok(())
        }
        _ => Err("usage: safrb-cargo-service-coupling [--check | --benchmark | --emit OUTPUT_DIRECTORY]".into()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
