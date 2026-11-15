// SPDX-License-Identifier: Apache-2.0 OR MIT
use std::{env, fs, process};

use symtropy_tropical_forest_qualifier::evaluate;

fn main() {
    let mut args = env::args_os().skip(1);
    let Some(input_path) = args.next() else {
        eprintln!(
            "usage: symtropy-tropical-forest-qualifier <fixture.json>\n\
evaluates one synthetic fixture and prints a deterministic JSON receipt"
        );
        process::exit(2);
    };

    if args.next().is_some() {
        eprintln!("error: exactly one input fixture path is accepted");
        process::exit(2);
    }

    let bytes = match fs::read(&input_path) {
        Ok(bytes) => bytes,
        Err(error) => {
            eprintln!(
                "error: cannot read {}: {error}",
                input_path.to_string_lossy()
            );
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

    let successful_result = matches!(
        receipt["result"].as_str(),
        Some("PASS")
            | Some("PASS_SYNTHETIC_RULES")
            | Some("MANIFEST_VALID")
            | Some("PASS_SYNTHETIC_GEOMETRY")
            | Some("PASS_SYNTHETIC_SUPPORT")
            | Some("PASS_SYNTHETIC_AGGREGATION")
            | Some("PASS_SYNTHETIC_CONSILIENCE")
            | Some("PASS_SYNTHETIC_MODEL_SET_DIAGNOSTICS")
            | Some("PASS_SYNTHETIC_FORECAST_SCORING")
            | Some("PASS_SYNTHETIC_BRIER")
            | Some("PASS_SYNTHETIC_BLOCK_CONFORMAL")
            | Some("PASS_SYNTHETIC_BLOCK_COVERAGE_INFERENCE")
            | Some("PASS_SYNTHETIC_SPATIAL_LOCALIZATION")
            | Some("PASS_SYNTHETIC_PROCESS_TRAJECTORY")
            | Some("PASS_SYNTHETIC_FLUX_IDENTIFIABILITY")
            | Some("PASS_SYNTHETIC_PRACTICAL_IDENTIFIABILITY")
            | Some("PASS_SYNTHETIC_FORESTPATHS_CROSSWALK")
            | Some("PASS_SYNTHETIC_AFEX_DESIGN")
            | Some("PASS_SYNTHETIC_ROOT_TURNOVER")
            | Some("PASS_SYNTHETIC_AFEX_ROOT_DISTRIBUTION_ORACLE")
    );
    if !successful_result {
        process::exit(1);
    }
}
