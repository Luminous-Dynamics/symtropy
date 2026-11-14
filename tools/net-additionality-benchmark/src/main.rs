use std::{env, fs, process};

fn main() {
    let path = env::args().nth(1).unwrap_or_else(|| {
        "tools/net-additionality-benchmark/fixtures/net-additionality-v0-1.json".to_owned()
    });
    let input = match fs::read(&path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("cannot read manifest {path}: {error}");
            process::exit(2);
        }
    };
    let source_revision = env::var("SOURCE_REVISION").unwrap_or_else(|_| "unprovided".to_owned());
    match net_additionality_benchmark::evaluate(&input, &source_revision) {
        Ok(report) => {
            let failed = report.overall_status != "PASS_SYNTHETIC_CONTRACT";
            match serde_json::to_string_pretty(&report) {
                Ok(output) => println!("{output}"),
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
            eprintln!("manifest rejected: {error}");
            process::exit(2);
        }
    }
}
