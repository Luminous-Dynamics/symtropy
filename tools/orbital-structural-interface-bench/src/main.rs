use orbital_structural_interface_bench::run_corpus;
use std::env;
use std::fs;

fn main() {
    if let Err(error) = run() {
        eprintln!("orbital_structural_interface_bench_error: {error}");
        std::process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let input_path = args.next().ok_or(
        "usage: orbital-structural-interface-bench <cases.json> --source-commit <40-hex-sha>",
    )?;
    let flag = args.next().ok_or("missing --source-commit flag")?;
    if flag != "--source-commit" {
        return Err("expected --source-commit".to_owned());
    }
    let source_commit = args.next().ok_or("missing source commit")?;
    if args.next().is_some() {
        return Err("unexpected trailing arguments".to_owned());
    }
    let bytes =
        fs::read(&input_path).map_err(|error| format!("cannot read {input_path}: {error}"))?;
    let receipt = run_corpus(&bytes, &source_commit)?;
    let output = serde_json::to_string_pretty(&receipt).map_err(|error| error.to_string())?;
    println!("{output}");
    if receipt.mismatched_cases > 0 {
        return Err(format!(
            "{} of {} expected dispositions mismatched",
            receipt.mismatched_cases, receipt.case_count
        ));
    }
    Ok(())
}
