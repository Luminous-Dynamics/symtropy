// Copyright (C) 2026 Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Reproducible scaling benchmark for the exact synthetic portfolio evaluator.

use accessible_home_portfolio_planner::{evaluate_bytes, PortfolioReport, EVALUATOR_VERSION};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use std::env;
use std::hint::black_box;
use std::process::ExitCode;
use std::time::Instant;

#[derive(Clone, Debug)]
struct Options {
    interventions: usize,
    iterations: usize,
    warmup: usize,
}

fn validate_count(count: usize) -> Result<(), String> {
    if [5, 10, 15, 20].contains(&count) {
        Ok(())
    } else {
        Err("interventions must be one of 5, 10, 15, 20 for comparable grouped workloads".into())
    }
}

fn parse_args() -> Result<Options, String> {
    let mut options = Options { interventions: 10, iterations: 30, warmup: 3 };
    let mut args = env::args().skip(1);
    while let Some(arg) = args.next() {
        let slot = match arg.as_str() {
            "--interventions" => &mut options.interventions,
            "--iterations" => &mut options.iterations,
            "--warmup" => &mut options.warmup,
            "--help" | "-h" => {
                println!("Usage: accessible-home-portfolio-benchmark [--interventions 5|10|15|20] [--warmup N] [--iterations N]");
                println!("Defaults: 10 interventions, 3 warmups, 30 timed iterations.");
                return Err(String::new());
            }
            _ => return Err(format!("unknown argument {arg:?}; use --help")),
        };
        let raw = args.next().ok_or_else(|| format!("missing value after {arg}"))?;
        *slot = raw.parse().map_err(|_| format!("invalid integer {raw:?} after {arg}"))?;
    }
    validate_count(options.interventions)?;
    if options.iterations < 2 || options.iterations > 10_000 || options.warmup > 10_000 {
        return Err("iterations must be 2..=10000 and warmup must be 0..=10000".into());
    }
    Ok(options)
}

fn build_input(count: usize) -> Result<Vec<u8>, String> {
    validate_count(count)?;
    let groups = 5usize;
    let options_per_group = count / groups;
    let hard_constraints: Vec<Value> = (0..groups).map(|g| json!({
        "id": format!("bench-hard-{g}"),
        "statement": format!("Synthetic constraint group {g} must have a declared planned remediation."),
        "baseline_status": "unresolved",
        "note": "Synthetic benchmark constraint only."
    })).collect();

    let interventions: Vec<Value> = (0..count).map(|i| {
        let group = i % groups;
        let option = i / groups;
        let low = 100_000u64 + option as u64 * 25_000 + group as u64 * 1_000;
        let maintenance_low = 16_000u64 - option as u64 * 2_000 + group as u64 * 100;
        let lead_high = 10u32 + (3 - option as u32) * 5 + group as u32;
        json!({
            "id": format!("bench-option-{option}-{group}"),
            "title": format!("Synthetic alternative {option} for constraint group {group}"),
            "depends_on": [],
            "addresses_hard_constraint_ids": [format!("bench-hard-{group}")],
            "supports_preference_ids": [],
            "conflicts_with_preference_ids": [],
            "resolves_preference_conflicts": [],
            "cost": {
                "cost_status": "known_range",
                "basis": "synthetic_scenario",
                "one_time_low_minor_units": low,
                "one_time_high_minor_units": low + 10_000,
                "annual_maintenance_low_minor_units": maintenance_low,
                "annual_maintenance_high_minor_units": maintenance_low + 500,
                "source_ref": format!("synthetic://portfolio-benchmark/cost/bench-option-{option}-{group}"),
                "observed_at": "2026-10-17T00:00:00Z"
            },
            "lead_time_days": {"low": lead_high.saturating_sub(1), "high": lead_high},
            "resident_disruption_points": option as u8,
            "access_gate": "cleared",
            "delivery_state": "proposed",
            "note": "Generated synthetic performance workload, not a real intervention or quote."
        })
    }).collect();

    let input = json!({
        "schema_version": "0.1.0",
        "fixture_kind": "synthetic_intervention_portfolio",
        "portfolio_id": format!("synthetic-benchmark-{count}-interventions"),
        "claim_boundary": "Synthetic benchmark only, not a quote, not a site assessment, and not proof of real-home accessibility or safety.",
        "currency": "ZAR",
        "budget_cap_upper_minor_units": 10_000_000u64,
        "planning_horizon_months": 12,
        "hard_constraints": hard_constraints,
        "preferences": [],
        "interventions": interventions
    });
    serde_json::to_vec(&input).map_err(|error| format!("could not encode synthetic input: {error}"))
}

fn percentile(sorted: &[u128], p: f64) -> u128 {
    let index = ((sorted.len() as f64 * p).ceil() as usize)
        .saturating_sub(1)
        .min(sorted.len().saturating_sub(1));
    sorted[index]
}

fn receipt_digest(report: &PortfolioReport) -> Result<String, String> {
    let mut bytes = serde_json::to_vec_pretty(report).map_err(|error| error.to_string())?;
    bytes.push(b'\n');
    Ok(Sha256::digest(&bytes).iter().map(|byte| format!("{byte:02x}")).collect())
}

fn run() -> Result<(), String> {
    let options = match parse_args() {
        Ok(options) => options,
        Err(error) if error.is_empty() => return Ok(()),
        Err(error) => return Err(error),
    };
    let input = build_input(options.interventions)?;
    let baseline = evaluate_bytes(&input)
        .map_err(|errors| format!("generated benchmark input rejected:\n- {}", errors.join("\n- ")))?;
    let expected_subsets = 1u64 << options.interventions;
    if baseline.intervention_count != options.interventions || baseline.evaluated_subset_count != expected_subsets {
        return Err("benchmark report counts do not match generated input".into());
    }
    let baseline_receipt_sha256 = receipt_digest(&baseline)?;
    let mut durations = Vec::with_capacity(options.iterations);

    for _ in 0..options.warmup {
        let report = evaluate_bytes(black_box(&input))
            .map_err(|errors| format!("warmup replay failed: {}", errors.join("; ")))?;
        if report != baseline || receipt_digest(&report)? != baseline_receipt_sha256 {
            return Err("warmup result differs from baseline report".into());
        }
        black_box(report);
    }
    for _ in 0..options.iterations {
        let start = Instant::now();
        let report = evaluate_bytes(black_box(&input))
            .map_err(|errors| format!("timed replay failed: {}", errors.join("; ")))?;
        let elapsed = start.elapsed();
        if report != baseline || receipt_digest(&report)? != baseline_receipt_sha256 {
            return Err("timed replay was not deterministic".into());
        }
        black_box(report);
        durations.push(elapsed.as_nanos());
    }
    durations.sort_unstable();

    let output = json!({
        "benchmark_schema_version": "0.1.0",
        "synthetic_only": true,
        "evaluator_version": EVALUATOR_VERSION,
        "os": env::consts::OS,
        "architecture": env::consts::ARCH,
        "intervention_count": options.interventions,
        "evaluated_subset_count": baseline.evaluated_subset_count,
        "frontier_count": baseline.pareto_frontier.len(),
        "within_budget_candidate_count": baseline.within_budget_candidate_count,
        "input_bytes": input.len(),
        "input_sha256": baseline.input_sha256,
        "receipt_sha256": baseline_receipt_sha256,
        "warmup_iterations": options.warmup,
        "timed_iterations": options.iterations,
        "minimum_elapsed_ns": durations[0],
        "median_elapsed_ns": percentile(&durations, 0.50),
        "p95_elapsed_ns": percentile(&durations, 0.95),
        "maximum_elapsed_ns": *durations.last().unwrap_or(&0),
        "peak_rss_bytes": null,
        "peak_rss_note": "Collect externally with /usr/bin/time -v or an OS-equivalent; preserve compiler and machine metadata.",
        "measurement_boundary": "Elapsed time wraps evaluate_bytes only. Fixture generation and receipt digesting are outside the timed interval."
    });
    println!("{}", serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?);
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("{error}");
            ExitCode::from(2)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_input_has_exact_subset_count() {
        let input = build_input(10).expect("build synthetic benchmark input");
        let report = evaluate_bytes(&input).expect("generated 10-intervention input evaluates");
        assert_eq!(report.intervention_count, 10);
        assert_eq!(report.evaluated_subset_count, 1_024);
        assert!(!report.pareto_frontier.is_empty());
    }

    #[test]
    fn only_comparable_workload_sizes_are_accepted() {
        assert!(validate_count(5).is_ok());
        assert!(validate_count(10).is_ok());
        assert!(validate_count(15).is_ok());
        assert!(validate_count(20).is_ok());
        assert!(validate_count(9).is_err());
    }
}
