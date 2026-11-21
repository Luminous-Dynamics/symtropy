use maritime_engineering_health_evaluator::evaluate_bytes;
use std::{env, fs, process};

fn main() {
    let Some(path) = env::args_os().nth(1) else {
        eprintln!("usage: maritime-engineering-health-evaluator <corpus.json>");
        process::exit(2);
    };

    let bytes = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read {}: {error}", path.to_string_lossy());
            process::exit(2);
        }
    };

    match evaluate_bytes(&bytes) {
        Ok(report) => {
            let failed = report.failed_cases != 0;
            match serde_json::to_string_pretty(&report) {
                Ok(json) => println!("{json}"),
                Err(error) => {
                    eprintln!("cannot serialize report: {error}");
                    process::exit(2);
                }
            }
            if failed {
                process::exit(1);
            }
        }
        Err(error) => {
            eprintln!("{error}");
            process::exit(2);
        }
    }
}
