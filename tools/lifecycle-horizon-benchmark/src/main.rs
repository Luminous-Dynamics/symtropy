use std::{env, fs, process};

fn main() {
    let arguments: Vec<String> = env::args().skip(1).collect();
    let policy_search = arguments.first().is_some_and(|argument| argument == "--policy-search");
    let manifest_argument = if policy_search {
        arguments.get(1)
    } else {
        arguments.first()
    };
    let path = manifest_argument.cloned().unwrap_or_else(|| {
        "tools/lifecycle-horizon-benchmark/fixtures/lifecycle-horizon-v0-1.json".to_owned()
    });
    let input = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read manifest {path}: {error}");
            process::exit(2);
        }
    };
    let source_revision = env::var("SOURCE_REVISION").unwrap_or_else(|_| "unprovided".to_owned());

    if policy_search {
        match lifecycle_horizon_benchmark::evaluate_policy_search(&input, &source_revision) {
            Ok(report) => {
                let failed = report.overall_status != "PASS_SYNTHETIC_CONTRACT";
                match serde_json::to_string_pretty(&report) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("cannot serialize policy-search report: {error}");
                        process::exit(2);
                    }
                }
                if failed {
                    process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("policy-search manifest rejected: {error}");
                process::exit(2);
            }
        }
    } else {
        match lifecycle_horizon_benchmark::evaluate(&input, &source_revision) {
            Ok(report) => {
                let failed = report.overall_status != "PASS_SYNTHETIC_CONTRACT";
                match serde_json::to_string_pretty(&report) {
                    Ok(output) => println!("{output}"),
                    Err(error) => {
                        eprintln!("cannot serialize horizon report: {error}");
                        process::exit(2);
                    }
                }
                if failed {
                    process::exit(1);
                }
            }
            Err(error) => {
                eprintln!("horizon manifest rejected: {error}");
                process::exit(2);
            }
        }
    }
}
