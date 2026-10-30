// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: Apache-2.0 OR MIT

use std::env;
use std::process::ExitCode;

use symtropy_experiment_selection::{
    PolicyKind, ProblemId, run_campaign, render_receipts, summarize,
};

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("experiment-selection-bench: {message}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<(), String> {
    let mut source_sha = String::from("UNBOUND");
    let mut seed_count = 20_usize;
    let mut budget = 32_usize;
    let mut initial_design = 8_usize;

    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let value = arguments
            .next()
            .ok_or_else(|| format!("missing value after {argument}"))?;
        match argument.as_str() {
            "--source-sha" => source_sha = value,
            "--seed-count" => {
                seed_count = value
                    .parse()
                    .map_err(|_| format!("invalid seed count: {value}"))?;
            }
            "--budget" => {
                budget = value
                    .parse()
                    .map_err(|_| format!("invalid evaluation budget: {value}"))?;
            }
            "--initial-design" => {
                initial_design = value
                    .parse()
                    .map_err(|_| format!("invalid initial design size: {value}"))?;
            }
            _ => return Err(format!("unknown argument: {argument}")),
        }
    }

    if seed_count == 0 {
        return Err("seed count must be positive".to_string());
    }
    if budget == 0 || initial_design > budget {
        return Err("require budget > 0 and initial design <= budget".to_string());
    }

    let seeds: Vec<u64> = (0..seed_count).map(|seed| seed as u64).collect();
    let results = run_campaign(&seeds, budget, initial_design);
    let summaries = summarize(&results);

    println!(
        "EXPERIMENT_CAMPAIGN,v1,source_sha={source_sha},problems={},policies={},seeds={},budget={},initial_design={}",
        ProblemId::ALL.len(),
        PolicyKind::ALL.len(),
        seed_count,
        budget,
        initial_design
    );
    print!("{}", render_receipts(&source_sha, &results));
    println!(
        "EXPERIMENT_SUMMARY,v1,problem,policy,trials,budget,mean_regret,median_regret,p90_regret,mean_infeasible_calls,mean_trial_runtime_ns"
    );
    for summary in summaries {
        println!(
            "EXPERIMENT_SUMMARY,v1,{},{},{},{},{:.12e},{:.12e},{:.12e},{:.6},{:.3}",
            summary.problem.as_str(),
            summary.policy.as_str(),
            summary.trials,
            summary.evaluations_per_trial,
            summary.mean_regret,
            summary.median_regret,
            summary.p90_regret,
            summary.mean_infeasible_calls,
            summary.mean_elapsed_nanos
        );
    }

    Ok(())
}
