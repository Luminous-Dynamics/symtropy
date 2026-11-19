use nh3_h2_service_evaluator::audit_manifest_json;
use std::{env, fs, process};

fn main() {
    let path = env::args()
        .nth(1)
        .unwrap_or_else(|| "benchmarks/ammonia/nh3-h2-service-v0.json".to_owned());

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
        "{}: {}/{} scenario dispositions; {}/{} comparisons matched",
        report.benchmark_id,
        report.scenario_expectations_matched,
        report.scenario_count,
        report.comparison_expectations_matched,
        report.comparison_count
    );

    for scenario in &report.scenarios {
        println!(
            "{}: {} (expected {})",
            scenario.id, scenario.disposition, scenario.expected_disposition
        );
        if let Some(value) = scenario.accepted_h2_specific_energy_kwh_per_kg {
            println!("  synthetic grid-to-accepted-H2 intensity: {value:.3} kWh/kg");
        }
        if let Some(value) = scenario.customer_delivery_settling_ms {
            println!("  synthetic customer delivery settling: {value:.3} ms");
        }
        for reason in &scenario.reasons {
            println!("  - {reason}");
        }
    }

    for comparison in &report.comparisons {
        println!(
            "comparison {}: {} (expected {})",
            comparison.id, comparison.disposition, comparison.expected_disposition
        );
        if let Some(delta) = comparison.candidate_vs_reference_energy_delta_fraction {
            println!(
                "  synthetic candidate/reference energy delta: {:.2}%",
                delta * 100.0
            );
        }
    }

    if report.scenario_expectations_matched != report.scenario_count
        || report.comparison_expectations_matched != report.comparison_count
    {
        process::exit(1);
    }
}
