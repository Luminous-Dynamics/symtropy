use std::{
    env, fs,
    io::{self, Read},
    process::ExitCode,
};

use symtropy_qualified_throughput::{BenchmarkInputV1, evaluate};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("symtropy-qualified-throughput: {message}");
            ExitCode::from(2)
        }
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args_os().skip(1);
    let input = if let Some(path) = args.next() {
        if args.next().is_some() {
            return Err("usage: symtropy-qualified-throughput [input.json]".to_owned());
        }
        fs::read_to_string(path).map_err(|error| format!("read input: {error}"))?
    } else {
        let mut contents = String::new();
        io::stdin()
            .read_to_string(&mut contents)
            .map_err(|error| format!("read stdin: {error}"))?;
        contents
    };

    let benchmark: BenchmarkInputV1 =
        serde_json::from_str(&input).map_err(|error| format!("parse input: {error}"))?;
    let report = evaluate(&benchmark).map_err(|error| error.to_string())?;
    let output = serde_json::to_string_pretty(&report)
        .map_err(|error| format!("serialize report: {error}"))?;
    println!("{output}");
    Ok(())
}
