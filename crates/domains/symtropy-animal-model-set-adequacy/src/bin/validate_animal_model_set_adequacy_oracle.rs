// SPDX-License-Identifier: AGPL-3.0-or-later

//! Verify the frozen model-set adequacy oracle without running ecology.

use std::fs;
use std::process;
use symtropy_animal_model_set_adequacy::{validate_embedded_oracle, validate_oracle_json};

fn main() {
    let mut args = std::env::args_os().skip(1);
    let candidate_path = args.next();
    if args.next().is_some() {
        eprintln!("usage: validate_animal_model_set_adequacy_oracle [candidate-oracle.json]");
        process::exit(2);
    }

    let result = match candidate_path {
        Some(path) => match fs::read_to_string(&path) {
            Ok(raw) => validate_oracle_json(&raw),
            Err(error) => {
                eprintln!(
                    "could not read oracle '{}': {error}",
                    path.to_string_lossy()
                );
                process::exit(2);
            }
        },
        None => validate_embedded_oracle(),
    };

    match result {
        Ok(count) => println!(
            "validated {count} synthetic model-set adequacy contract cases; no ecological validation implied"
        ),
        Err(errors) => {
            for error in errors {
                eprintln!("model-set adequacy oracle validation failed: {error}");
            }
            process::exit(1);
        }
    }
}
