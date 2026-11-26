// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

use std::{env, fs, process::ExitCode};

use mobility_rrvr_aerb_evaluator::evaluate_json;

fn main() -> ExitCode {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 3 {
        eprintln!(
            "usage: aerb_evaluate <candidate-episode.json> <candidate-submission.json> <evaluator-world-pack.json>"
        );
        return ExitCode::from(2);
    }
    let episode = match fs::read_to_string(&args[0]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("CANDIDATE_EPISODE_READ_FAILED: {error}");
            return ExitCode::FAILURE;
        }
    };
    let submission = match fs::read_to_string(&args[1]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("CANDIDATE_SUBMISSION_READ_FAILED: {error}");
            return ExitCode::FAILURE;
        }
    };
    let world_pack = match fs::read_to_string(&args[2]) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("EVALUATOR_WORLD_PACK_READ_FAILED: {error}");
            return ExitCode::FAILURE;
        }
    };
    match evaluate_json(&episode, &submission, &world_pack) {
        Ok(report) => {
            match serde_json::to_string_pretty(&report) {
                Ok(output) => println!("{output}"),
                Err(error) => {
                    eprintln!("EVALUATOR_REPORT_SERIALIZATION_FAILED: {error}");
                    return ExitCode::FAILURE;
                }
            }
            if report.candidate_contract_valid && report.selected_disposition_admissible {
                ExitCode::SUCCESS
            } else {
                ExitCode::FAILURE
            }
        }
        Err(error) => {
            eprintln!("AERB_EVALUATION_FAILED: {error}");
            ExitCode::FAILURE
        }
    }
}
