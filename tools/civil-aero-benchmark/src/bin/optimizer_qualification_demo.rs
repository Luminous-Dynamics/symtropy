use std::{env, fs, process};

use civil_aero_benchmark::optimizer_qualification::{
    OptimizerQualificationConfig, candidates_to_csv, evaluate_optimizer_qualification,
    summary_to_csv,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("synthetic optimizer qualification rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let arguments = env::args().skip(1).collect::<Vec<_>>();
    if arguments.len() != 2 {
        return Err(
            "usage: optimizer_qualification_demo <summary.csv> <candidate-trace.csv>".to_owned(),
        );
    }
    let config = OptimizerQualificationConfig {
        decision_dimensions: 30,
        population_size: 40,
        maximum_evaluations: 1000,
        checkpoints: vec![100, 250, 500, 1000],
        seeds: vec![101, 202, 303, 404, 505],
    };
    let report = evaluate_optimizer_qualification(&config)?;
    fs::write(&arguments[0], summary_to_csv(&report))
        .map_err(|error| format!("cannot write {}: {error}", arguments[0]))?;
    fs::write(&arguments[1], candidates_to_csv(&report))
        .map_err(|error| format!("cannot write {}: {error}", arguments[1]))?;
    eprintln!(
        "synthetic_algorithm_qualification_only tasks=2 seeds=5 policies=2 checkpoints=4 summaries={} decision_dimensions={} population={} objective_evaluations={} candidate_trace_rows={} no_cfd_solves=true",
        report.summaries.len(),
        report.decision_dimensions,
        report.population_size,
        report.candidates.len(),
        report.candidates.len()
    );
    Ok(())
}
