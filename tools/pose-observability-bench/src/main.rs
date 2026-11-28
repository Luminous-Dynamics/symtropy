use pose_observability_bench::{evaluate, ObservabilityRequest};
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
            "--input" => {
                input = Some(args.next().ok_or_else(|| "missing --input value".to_string())?);
            }
            "--output" => {
                output = Some(args.next().ok_or_else(|| "missing --output value".to_string())?);
            }
            "--help" | "-h" => {
                println!("pose-observability-bench --input FILE [--output FILE]");
                return Ok(());
            }
            _ => return Err(format!("unknown argument {flag}")),
        }
    }
    let input = input.ok_or_else(|| "missing --input".to_string())?;
    let metadata = fs::metadata(&input).map_err(|error| format!("cannot stat {input}: {error}"))?;
    if metadata.len() > MAX_INPUT_BYTES {
        return Err("input exceeds the 32 MiB limit".to_string());
    }
    let bytes = fs::read(&input).map_err(|error| format!("cannot read {input}: {error}"))?;
    let request: ObservabilityRequest =
        serde_json::from_slice(&bytes).map_err(|error| format!("invalid request JSON: {error}"))?;
    let result = evaluate(&request)?;
    let serialized = serde_json::to_vec_pretty(&result)
        .map_err(|error| format!("cannot serialize observability result: {error}"))?;
    match output {
        Some(path) => {
            if Path::new(&path).exists() {
                return Err(format!("refusing to overwrite existing output {path}"));
            }
            fs::write(&path, serialized)
                .map_err(|error| format!("cannot write {path}: {error}"))?;
        }
        None => println!("{}", String::from_utf8(serialized).map_err(|error| error.to_string())?),
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("pose-observability-bench: {error}");
        process::exit(2);
    }
}
