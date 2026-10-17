use std::{env, process};

use wis_sim_fixture_verifier::verify_files;

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.len() != 5 || args[1] != "verify" {
        eprintln!("usage: wis-sim-fixture-verifier verify <fixture.jsonl> <manifest.json> <schema.json>");
        process::exit(2);
    }

    match verify_files(&args[2], &args[3], &args[4]) {
        Ok(receipt) => match serde_json::to_string_pretty(&receipt) {
            Ok(output) => println!("{}", output),
            Err(error) => {
                eprintln!("failed to serialize receipt: {}", error);
                process::exit(2);
            }
        },
        Err(error) => {
            eprintln!("fixture rejected: {}", error);
            process::exit(2);
        }
    }
}
