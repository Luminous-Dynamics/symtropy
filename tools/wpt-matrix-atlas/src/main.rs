// SPDX-License-Identifier: AGPL-3.0-or-later
use std::{env, fs, process::ExitCode};

use wpt_matrix_atlas::MatrixAtlas;

fn main() -> ExitCode {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if !(2..=3).contains(&arguments.len()) {
        eprintln!("usage: wpt-matrix-atlas <atlas.json> <position_m> [expected-disposition]");
        return ExitCode::from(2);
    }

    let position_m = match arguments[1].parse::<f64>() {
        Ok(value) => value,
        Err(error) => {
            eprintln!("invalid position: {error}");
            return ExitCode::from(2);
        }
    };
    let source = match fs::read_to_string(&arguments[0]) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("cannot read atlas file: {error}");
            return ExitCode::FAILURE;
        }
    };
    let atlas: MatrixAtlas = match serde_json::from_str(&source) {
        Ok(atlas) => atlas,
        Err(error) => {
            eprintln!("invalid atlas JSON: {error}");
            return ExitCode::FAILURE;
        }
    };

    match atlas.interpolate_at(position_m) {
        Ok(result) => match serde_json::to_string_pretty(&result) {
            Ok(json) => {
                println!("{json}");
                if let Some(expected) = arguments
                    .get(2)
                    .filter(|expected| result.disposition.as_str() != expected.as_str())
                {
                    eprintln!(
                        "disposition mismatch: expected {expected}, got {}",
                        result.disposition
                    );
                    return ExitCode::FAILURE;
                }
                if result.disposition == "UNRESOLVED" {
                    eprintln!("matrix-atlas result is UNRESOLVED; refusing successful exit");
                    return ExitCode::FAILURE;
                }
                ExitCode::SUCCESS
            }
            Err(error) => {
                eprintln!("cannot serialize interpolation result: {error}");
                ExitCode::FAILURE
            }
        },
        Err(error) => {
            eprintln!("{error}");
            ExitCode::FAILURE
        }
    }
}
