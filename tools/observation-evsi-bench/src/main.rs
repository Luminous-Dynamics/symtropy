use observation_evsi_bench::{DecisionProblem, evaluate};
use std::env;
use std::fs;
use std::path::Path;
use std::process;

const MAX_INPUT_BYTES: u64 = 32 * 1024 * 1024;

fn run() -> Result<(), String> {
    let mut input = None;
    let mut output = None;
    let mut args = env::args().skip(1);
    while let Some(flag) = args.next() {
        match flag.as_str() {
            "--input" => input = Some(args.next().ok_or_else(|| "missing --input value".to_string())?),
            "--output" => output = Some(args.next().ok_or_else(|| "missing --output value".to_string())?),
            "--help" | "-h" => {
                println!("observation-evsi-bench --input FILE [--output FILE]");
                return Ok(());
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    let input = input.ok_or_else(|| "missing --input".to_string())?;
    let metadata = fs::metadata(&input).map_err(|error| format!("cannot stat {input}: {error}"))?;
    if metadata.len() > MAX_INPUT_BYTES {
        return Err("input exceeds 32 MiB limit".to_string());
    }
    let bytes = fs::read(&input).map_err(|error| format!("cannot read {input}: {error}"))?;
    let problem: DecisionProblem =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid problem JSON: {error}"))?;
    let result = evaluate(&problem)?;
    let serialized = serde_json::to_vec_pretty(&result)
        .map_err(|error| format!("cannot serialize EVSI result: {error}"))?;
    match output {
        Some(path) => {
            if Path::new(&path).exists() {
                return Err(format!("refusing to overwrite existing output {path}"));
            }
            fs::write(&path, serialized).map_err(|error| format!("cannot write {path}: {error}"))?;
        }
        None => println!("{}", String::from_utf8(serialized).map_err(|error| error.to_string())?),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("observation-evsi-bench: {error}");
        process::exit(2);
    }
}
