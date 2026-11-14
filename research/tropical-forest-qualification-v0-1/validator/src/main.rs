use std::{env, fs, process};

use symtropy_tropical_forest_qualifier::evaluate;

fn main() {
    let mut args = env::args_os().skip(1);
    let Some(input_path) = args.next() else {
        eprintln!("usage: symtropy-tropical-forest-qualifier <fixture.json>\n\
evaluates one synthetic fixture and prints a deterministic JSON receipt");
        process::exit(2);
    };

    if args.next().is_some() {
        eprintln!("error: exactly one input fixture path is accepted");
        process::exit(2);
    }

    let bytes = match fs::read(&input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!("error: cannot read {}: {error}", input_path.to_string_lossy());
            process::exit(2);
        }
    };

    let receipt = match evaluate(&bytes) {
        Ok(receipt) => receipt,
        Err(error) => {
            eprintln!("error: {error}");
            process::exit(2);
        }
    };

    match serde_json::to_string_pretty(&receipt) {
        Ok(serialized) => println!("{serialized}"),
        Err(error) => {
            eprintln!("error: could not serialize receipt: {error}");
            process::exit(2);
        }
    }

    if receipt["result"] != "PASS" {
        process::exit(1);
    }
}
