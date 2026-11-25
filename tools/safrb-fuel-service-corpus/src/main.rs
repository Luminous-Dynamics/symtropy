#![forbid(unsafe_code)]

use safrb_fuel_service_corpus::{
    evaluate_temporal_ablation, generate_corpus, validate_corpus,
};
use std::{env, fs, path::Path};

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let cases = generate_corpus();
    validate_corpus(&cases).map_err(|errors| errors.join("\n"))?;
    let report = evaluate_temporal_ablation(&cases, 4)
        .map_err(|errors| errors.join("\n"))?;

    match args.as_slice() {
        [flag] if flag == "--check" => {
            if report.cases_total != 120 || report.aggregate_false_guarantee_count == 0 {
                return Err(
                    "temporal ablation must contain 120 cases and at least one aggregate-only false guarantee"
                        .to_string(),
                );
            }
            println!(
                "SAFRB corpus PASS: {} cases, {} hourly intervals, {} aggregate-only false guarantees",
                cases.len(),
                cases.iter().map(|case| case.hourly_trajectory.len()).sum::<usize>(),
                report.aggregate_false_guarantee_count
            );
            Ok(())
        }
        [flag] if flag == "--benchmark" => {
            println!("{}", report.to_json());
            Ok(())
        }
        [flag, output_dir] if flag == "--emit" => {
            let directory = Path::new(output_dir);
            fs::create_dir_all(directory)
                .map_err(|error| format!("cannot create output directory: {error}"))?;

            let mut observations = String::new();
            let mut oracle = String::new();
            for case in &cases {
                observations.push_str(&case.to_planner_observation_json_line());
                observations.push('\n');
                oracle.push_str(&case.to_oracle_truth_json_line());
                oracle.push('\n');
            }

            let observation_path = directory.join("planner-observations.jsonl");
            let oracle_path = directory.join("oracle-truth.jsonl");
            let report_path = directory.join("temporal-ablation-report.json");
            fs::write(&observation_path, observations)
                .map_err(|error| format!("cannot write planner observations: {error}"))?;
            fs::write(&oracle_path, oracle)
                .map_err(|error| format!("cannot write oracle truth: {error}"))?;
            fs::write(&report_path, report.to_json())
                .map_err(|error| format!("cannot write temporal ablation report: {error}"))?;

            println!(
                "wrote {} planner observations, {} oracle rows and temporal ablation report to {}",
                cases.len(),
                cases.len(),
                directory.display()
            );
            Ok(())
        }
        [] => {
            // Standard output is planner-safe by default. Oracle truth and
            // evaluator reports are emitted only to the explicit output path.
            for case in &cases {
                println!("{}", case.to_planner_observation_json_line());
            }
            Ok(())
        }
        _ => Err(
            "usage: safrb-fuel-service-corpus [--check | --benchmark | --emit OUTPUT_DIRECTORY]"
                .to_string(),
        ),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
