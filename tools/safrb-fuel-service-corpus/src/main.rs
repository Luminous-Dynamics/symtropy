#![forbid(unsafe_code)]

use safrb_fuel_service_corpus::{generate_corpus, validate_corpus};
use std::{env, fs, path::Path};

fn run() -> Result<(), String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    let cases = generate_corpus();
    validate_corpus(&cases).map_err(|errors| errors.join("\n"))?;

    match args.as_slice() {
        [flag] if flag == "--check" => {
            println!(
                "SAFRB fuel-service corpus PASS: {} synthetic cases, 12 archetypes, {}-hour horizon",
                cases.len(),
                cases.first().map_or(0, |case| case.horizon_hours)
            );
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
            fs::write(&observation_path, observations)
                .map_err(|error| format!("cannot write planner observations: {error}"))?;
            fs::write(&oracle_path, oracle)
                .map_err(|error| format!("cannot write oracle truth: {error}"))?;

            println!(
                "wrote {} planner observations and {} oracle rows to {}",
                cases.len(),
                cases.len(),
                directory.display()
            );
            Ok(())
        }
        [] => {
            // Standard output is planner-safe by default. Oracle truth requires --emit.
            for case in &cases {
                println!("{}", case.to_planner_observation_json_line());
            }
            Ok(())
        }
        _ => Err("usage: safrb-fuel-service-corpus [--check | --emit OUTPUT_DIRECTORY]".to_string()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
