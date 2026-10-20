// SPDX-License-Identifier: AGPL-3.0-or-later

//! Verify the frozen model-set adequacy oracle without running ecology.

use std::process;
use symtropy_animal_model_set_adequacy::validate_embedded_oracle;

fn main() {
    match validate_embedded_oracle() {
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
