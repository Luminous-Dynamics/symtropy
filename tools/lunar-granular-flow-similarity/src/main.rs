use std::{env, fs, io::{self, Read}, process::ExitCode};

use lunar_granular_flow_similarity::{ComparisonInput, compare};

fn read_input() -> Result<String, String> {
    let mut args = env::args().skip(1);
    if let Some(path) = args.next() {
        if args.next().is_some() {
            return Err("usage: lunar-granular-flow-similarity [input.json]".to_owned());
        }
        fs::read_to_string(&path).map_err(|error| format!("cannot read {path}: {error}"))
    } else {
        let mut input = String::new();
        io::stdin()
            .read_to_string(&mut input)
            .map_err(|error| format!("cannot read stdin: {error}"))?;
        Ok(input)
    }
}

fn main() -> ExitCode {
    let result = (|| {
        let raw = read_input()?;
        let input: ComparisonInput =
            serde_json::from_str(&raw).map_err(|error| format!("invalid input JSON: {error}"))?;
        let report = compare(&input)?;
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("cannot serialize report: {error}"))
    })();

    match result {
        Ok(report) => {
            println!("{report}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("similarity comparison error: {error}");
            ExitCode::FAILURE
        }
    }
}
