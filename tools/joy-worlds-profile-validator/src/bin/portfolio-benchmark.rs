use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::process::ExitCode;
use std::time::Instant;

use joy_worlds_profile_validator::comparison::compare_scenarios_json;

fn parse_count(args: &mut impl Iterator<Item = String>, flag: &str) -> Result<usize, String> {
    let Some(value) = args.next() else {
        return Err(format!("{flag} requires a value"));
    };
    let parsed = value.parse::<usize>().map_err(|_| format!("{flag} expects a positive integer"))?;
    if parsed == 0 {
        return Err(format!("{flag} must be greater than zero"));
    }
    Ok(parsed)
}

fn parse_args() -> Result<(usize, usize, usize, usize), String> {
    let mut args = env::args().skip(1);
    let mut candidates = 500;
    let mut scenarios = 20;
    let mut objectives = 8;
    let mut repetitions = 7;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--candidates" => candidates = parse_count(&mut args, "--candidates")?,
            "--scenarios" => scenarios = parse_count(&mut args, "--scenarios")?,
            "--objectives" => objectives = parse_count(&mut args, "--objectives")?,
            "--repetitions" => repetitions = parse_count(&mut args, "--repetitions")?,
            "--help" | "-h" => {
                return Err("USAGE: portfolio-benchmark [--candidates N] [--scenarios N] [--objectives N] [--repetitions N] (defaults: 500, 20, 8, 7)".to_owned());
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }
    if candidates < 2 || scenarios < 2 || objectives < 2 {
        return Err("candidates, scenarios and objectives must each be at least 2".to_owned());
    }
    Ok((candidates, scenarios, objectives, repetitions))
}

fn make_fixture(candidate_count: usize, scenario_count: usize, objective_count: usize) -> String {
    let objective_dimensions: Vec<Value> = (0..objective_count).map(|index| {
        json!({
            "id": format!("objective_{index:02}"),
            "unit": format!("unit_{index:02}"),
            "direction": if index % 2 == 0 { "maximize" } else { "minimize" },
            "dominance_tolerance": 0.0
        })
    }).collect();

    let required_hard_constraints = [
        "rights_and_access",
        "safety_case",
        "resource_envelope",
        "ecological_capacity"
    ];
    let scenarios: Vec<Value> = (0..scenario_count).map(|scenario_index| {
        let candidates: Vec<Value> = (0..candidate_count).map(|candidate_index| {
            let id = format!("C{candidate_index:05}");
            let constraint_checks: Vec<Value> = required_hard_constraints.iter().map(|constraint| {
                json!({
                    "id": constraint,
                    "status": "pass",
                    "evidence_ref": format!("synthetic://benchmark/{id}/{scenario_index}/{constraint}")
                })
            }).collect();

            let outcomes: serde_json::Map<String, Value> = (0..objective_count).map(|objective_index| {
                let objective_id = format!("objective_{objective_index:02}");
                let value = ((candidate_index * 37 + scenario_index * 13 + objective_index * 17) % 10_000) as f64 + 1.0;
                let lower = value * 0.95;
                let upper = value * 1.05;
                let unit = format!("unit_{objective_index:02}");
                (objective_id, json!({
                    "status": "known",
                    "value": value,
                    "lower_bound": lower,
                    "upper_bound": upper,
                    "unit": unit,
                    "evidence_ref": format!("synthetic://benchmark/{id}/{scenario_index}/objective_{objective_index:02}")
                }))
            }).collect();

            json!({
                "id": id,
                "name": format!("Synthetic candidate {candidate_index:05}"),
                "constraint_checks": constraint_checks,
                "outcomes": outcomes
            })
        }).collect();

        json!({
            "id": format!("scenario_{scenario_index:03}"),
            "assumptions_ref": format!("synthetic://benchmark/scenario_{scenario_index:03}"),
            "candidates": candidates
        })
    }).collect();

    json!({
        "schema_version": "0.1.0",
        "artifact_id": "luminous-dynamics.symtropy.generated-performance-fixture",
        "status": "synthetic_fixture",
        "non_claims": ["Generated performance fixture only; no real site inference."],
        "required_hard_constraints": required_hard_constraints,
        "objective_dimensions": objective_dimensions,
        "scenarios": scenarios
    }).to_string()
}

fn main() -> ExitCode {
    let (candidates, scenarios, objectives, repetitions) = match parse_args() {
        Ok(values) => values,
        Err(message) => {
            eprintln!("{message}");
            return if message.starts_with("USAGE:") { ExitCode::SUCCESS } else { ExitCode::FAILURE };
        }
    };

    let Some(candidate_scenario_pairs) = candidates.checked_mul(scenarios) else {
        eprintln!("candidate count multiplied by scenario count overflows usize");
        return ExitCode::FAILURE;
    };
    let raw = make_fixture(candidates, scenarios, objectives);
    let input_sha256 = format!("{:x}", Sha256::digest(raw.as_bytes()));
    let mut samples_ms = Vec::with_capacity(repetitions);
    let mut pareto_count = None;
    for _ in 0..repetitions {
        let start = Instant::now();
        let receipt = match compare_scenarios_json(&raw) {
            Ok(receipt) => receipt,
            Err(errors) => {
                eprintln!("Generated benchmark fixture was rejected: {errors:?}");
                return ExitCode::FAILURE;
            }
        };
        samples_ms.push(start.elapsed().as_secs_f64() * 1_000.0);
        pareto_count = receipt["pareto_front_ids"].as_array().map(Vec::len);
    }

    samples_ms.sort_by(f64::total_cmp);
    let median_ms = samples_ms[samples_ms.len() / 2];
    let p95_index = ((samples_ms.len() as f64 * 0.95).ceil() as usize)
        .saturating_sub(1)
        .min(samples_ms.len() - 1);
    let p95_ms = samples_ms[p95_index];

    println!("{}", json!({
        "benchmark": "joy-worlds-comparator/0.1.0",
        "scope": "synthetic fixture generation excluded; input parsing plus validation, aggregation and Pareto computation included",
        "candidate_count": candidates,
        "scenario_count": scenarios,
        "candidate_scenario_pairs": candidate_scenario_pairs,
        "input_bytes": raw.len(),
        "input_sha256": input_sha256,
        "objective_count": objectives,
        "repetitions": repetitions,
        "median_ms": median_ms,
        "p95_ms": p95_ms,
        "sample_ms_sorted": samples_ms,
        "pareto_front_count": pareto_count,
        "performance_target_ms_p95": 1000.0,
        "target_status": if p95_ms < 1000.0 { "met_for_this_run_only" } else { "not_met_for_this_run" },
        "non_claims": ["No site-model quality or real-world feasibility claim.", "Compare timings only on documented hardware, OS, Rust version and load."]
    }));
    ExitCode::SUCCESS
}
