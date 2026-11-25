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
        [flag, output] if flag == "--emit" => {
            let path = Path::new(output);
            if let Some(parent) = path.parent() {
                fs::create_dir_all(parent)
                    .map_err(|error| format!("cannot create output directory: {error}"))?;
            }
            let mut content = String::new();
            for case in &cases {
                content.push_str(&case.to_json_line());
                content.push('\n');
            }
            fs::write(path, content)
                .map_err(|error| format!("cannot write corpus JSONL: {error}"))?;
            println!("wrote {} deterministic synthetic cases to {}", cases.len(), path.display());
            Ok(())
        }
        [] => {
            for case in &cases {
                println!("{}", case.to_json_line());
            }
            Ok(())
        }
        _ => Err("usage: safrb-fuel-service-corpus [--check | --emit PATH]".to_string()),
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("{error}");
        std::process::exit(1);
    }
}
