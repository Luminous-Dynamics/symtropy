// Copyright (C) 2024-2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use symtropy_material_flow_benchmark::za_uas_eligibility::{
    ZaUasEligibilityCorpus, evaluate_za_uas_corpus,
};

fn main() -> ExitCode {
    let mut args = env::args_os();
    let program = args.next().unwrap_or_default();
    let Some(path) = args.next() else {
        eprintln!("usage: {} <za-uas-eligibility-corpus.json>", program.to_string_lossy());
        return ExitCode::from(2);
    };
    if args.next().is_some() {
        eprintln!("expected exactly one South African UAS eligibility corpus path");
        return ExitCode::from(2);
    }
    let path = path.to_string_lossy();
    let raw = match fs::read_to_string(path.as_ref()) {
        Ok(raw) => raw,
        Err(error) => {
            eprintln!("cannot read ZA UAS eligibility corpus: {error}");
            return ExitCode::from(2);
        }
    };
    let corpus: ZaUasEligibilityCorpus = match serde_json::from_str(&raw) {
        Ok(corpus) => corpus,
        Err(error) => {
            eprintln!("invalid ZA UAS eligibility JSON: {error}");
            return ExitCode::from(1);
        }
    };
    let report = match evaluate_za_uas_corpus(&corpus) {
        Ok(report) => report,
        Err(errors) => {
            eprintln!("ZA UAS eligibility corpus rejected:");
            for error in errors {
                eprintln!("- {error}");
            }
            return ExitCode::from(1);
        }
    };
    match serde_json::to_string_pretty(&report) {
        Ok(json) => {
            println!("{json}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("cannot encode ZA UAS eligibility report: {error}");
            ExitCode::from(1)
        }
    }
}
