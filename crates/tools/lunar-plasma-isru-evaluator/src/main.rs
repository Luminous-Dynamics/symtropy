use lunar_plasma_isru_evaluator::audit_manifest_json;
use std::{env, fs, process};

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/plasma/lunar-plasma-isru-process-v0.json".to_owned());

    let source = match fs::read_to_string(&path) {
        Ok(source) => source,
        Err(error) => {
            eprintln!("failed to read {path}: {error}");
            process::exit(2);
        }
    };

    let report = match audit_manifest_json(&source) {
        Ok(report) => report,
        Err(error) => {
            eprintln!("audit error: {error}");
            process::exit(2);
        }
    };

    println!(
        "{}: {}/{} expected dispositions matched; {} mismatched",
        report.benchmark_id,
        report.matched_expectations,
        report.scenario_count,
        report.mismatched_expectations
    );
    for result in &report.results {
        println!(
            "{}: {} (expected {})",
            result.id, result.disposition, result.expected_disposition
        );
        if let Some(residual) = result.feedstock_balance_residual_g {
            println!("  feedstock balance residual: {residual:.6} g");
        }
        if let Some(residual) = result.reagent_balance_residual_g {
            println!("  reagent inventory residual: {residual:.6} g");
        }
        if let Some(energy) = result.synthetic_grid_energy_wh_per_kg_o2 {
            println!("  synthetic grid intensity: {energy:.3} Wh/kg O2 (synthetic only)");
        }
        for reason in &result.reasons {
            println!("  - {reason}");
        }
    }

    if report.mismatched_expectations != 0 {
        process::exit(1);
    }
}
