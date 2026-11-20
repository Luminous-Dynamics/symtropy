use power_restoration_run_receipt::parse_and_qualify;
use std::env;
use std::fs;
use std::process;

fn read(path: &str) -> String {
    fs::read_to_string(path).unwrap_or_else(|error| {
        eprintln!("cannot read {path}: {error}");
        process::exit(2);
    })
}

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 3 {
        eprintln!("usage: power-restoration-run-receipt-validate <receipt.json> <case.json>");
        process::exit(2);
    }
    match parse_and_qualify(&read(&args[1]), &read(&args[2])) {
        Ok(report) => println!("{}", serde_json::to_string_pretty(&report).expect("report serialization")),
        Err(errors) => {
            eprintln!("run receipt rejected:");
            for error in errors { eprintln!("- {error}"); }
            process::exit(1);
        }
    }
}
