// SPDX-License-Identifier: AGPL-3.0-or-later

//! Validate the committed model-structure uncertainty oracle or a supplied JSON file.

use std::fs;
use std::process;

use symtropy_animal_model_uncertainty::{EMBEDDED_ORACLE, validate_oracle_json};

fn main() {
    let mut arguments = std::env::args_os().skip(1);
    let path = arguments.next();
    if arguments.next().is_some() {
        eprintln!("usage: validate_animal_model_uncertainty_oracle [oracle.json]");
        process::exit(2);
    }

    let input = match path {
        Some(path) => match fs::read_to_string(&path) {
            Ok(content) => content,
            Err(error) => {
                eprintln!(
                    "could not read oracle '{}': {error}",
                    path.to_string_lossy()
                );
                process::exit(2);
            }
        },
        None => EMBEDDED_ORACLE.to_owned(),
    };

    match validate_oracle_json(&input) {
        Ok(count) => println!(
            "validated {count} synthetic model-structure uncertainty cases; no empirical biology claim"
        ),
        Err(errors) => {
            for error in errors {
                eprintln!("validation error: {error}");
            }
            process::exit(1);
        }
    }
}
