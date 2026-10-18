#![forbid(unsafe_code)]

use jobshop_reference_benchmark::{parse_classic_pair_format, solve_exact, summarize};
use std::env;
use std::error::Error;
use std::fs;

fn run() -> Result<(), Box<dyn Error>> {
    let mut arguments = env::args();
    let _program = arguments.next();
    let mode = arguments.next().ok_or(
        "usage: jobshop-reference-benchmark <summarize|solve-exact> <instance-file>",
    )?;
    let path = arguments.next().ok_or(
        "usage: jobshop-reference-benchmark <summarize|solve-exact> <instance-file>",
    )?;
    if arguments.next().is_some() {
        return Err(
            "usage: jobshop-reference-benchmark <summarize|solve-exact> <instance-file>".into(),
        );
    }

    let contents = fs::read_to_string(&path)?;
    let instance = parse_classic_pair_format(&contents)?;
    match mode.as_str() {
        "summarize" => {
            let summary = summarize(&instance)?;
            println!(
                "benchmark=JOBSHOP-REF-001 format=classic-pairs jobs={} machines={} operations={} total_processing_time={} max_job_work={} max_machine_load={} lower_bound_makespan={}",
                summary.job_count,
                summary.machine_count,
                summary.operation_count,
                summary.total_processing_time,
                summary.maximum_job_work,
                summary.maximum_machine_load,
                summary.lower_bound_makespan
            );
        }
        "solve-exact" => {
            let result = solve_exact(&instance)?;
            println!(
                "benchmark=JOBSHOP-REF-001 solver=bounded-exhaustive optimality=proved makespan={} lower_bound={} expanded_nodes={} scheduled_operations={} claim_ceiling=tiny-instance-model",
                result.makespan,
                result.lower_bound_makespan,
                result.expanded_nodes,
                result.schedule.len()
            );
        }
        _ => {
            return Err("unknown mode; use summarize or solve-exact".into());
        }
    }
    Ok(())
}

fn main() {
    if let Err(error) = run() {
        eprintln!("error: {error}");
        std::process::exit(2);
    }
}
