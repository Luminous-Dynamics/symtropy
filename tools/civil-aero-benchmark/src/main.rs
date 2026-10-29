use std::{env, fs, process};

use civil_aero_benchmark::{parse_cases_csv, OUTPUT_HEADER};

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "civil-aero-benchmark".to_owned());
    let input_path = args.next().unwrap_or_else(|| "data/example_cases.csv".to_owned());
    if args.next().is_some() {
        eprintln!("usage: {program} [input.csv] (CSV only; see the documented exact header)");
        process::exit(2);
    }

    let input = fs::read_to_string(&input_path).unwrap_or_else(|error| {
        eprintln!("cannot read {input_path}: {error}");
        process::exit(2);
    });
    let cases = parse_cases_csv(&input).unwrap_or_else(|error| {
        eprintln!("input rejected: {error}");
        process::exit(2);
    });

    println!("{OUTPUT_HEADER}");
    for case in cases {
        match case.evaluate() {
            Ok(row) => println!("{}", row.to_csv_row()),
            Err(error) => {
                eprintln!("evaluation rejected: {error}");
                process::exit(2);
            }
        }
    }
}
