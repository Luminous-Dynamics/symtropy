use std::{env, fs, process};

use civil_aero_benchmark::parse_cases_csv;
use civil_aero_benchmark::robustness::{
    STRESS_OUTPUT_HEADER, parse_stress_scenarios_csv, run_stress_suite, stress_result_csv_row,
};

fn main() {
    if let Err(error) = run() {
        eprintln!("robust stress evaluation rejected: {error}");
        process::exit(2);
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let case_path = args
        .next()
        .unwrap_or_else(|| "data/example_cases.csv".to_owned());
    let scenario_path = args
        .next()
        .unwrap_or_else(|| "data/illustrative_stress_scenarios.csv".to_owned());
    if args.next().is_some() {
        return Err("usage: robust [candidate-cases.csv] [stress-scenarios.csv]".to_owned());
    }

    let cases_csv = fs::read_to_string(&case_path)
        .map_err(|error| format!("cannot read {case_path}: {error}"))?;
    let scenarios_csv = fs::read_to_string(&scenario_path)
        .map_err(|error| format!("cannot read {scenario_path}: {error}"))?;
    let cases = parse_cases_csv(&cases_csv)?;
    let scenarios = parse_stress_scenarios_csv(&scenarios_csv)?;

    println!("{STRESS_OUTPUT_HEADER}");
    for case in &cases {
        let report = run_stress_suite(case, &scenarios)?;
        for result in &report.results {
            println!("{}", stress_result_csv_row(&case.case_id, result));
        }
        eprintln!(
            "case={} model={} evidence={} scenario_basis={} scenarios={} passed={} lift_failures={} energy_failures={} worst_energy_scenario={} worst_energy_wh={:.8} worst_energy_margin_scenario={} worst_energy_margin_wh={:.8} lowest_lift_margin={} lowest_lift_margin_coefficient={:.8} robust_screen_pass={} probability=none",
            report.case_id,
            report.model_version,
            report.evidence_level,
            report.scenario_basis,
            report.scenario_count,
            report.passed_count,
            report.lift_failure_count,
            report.energy_failure_count,
            report.worst_energy_scenario,
            report
                .results
                .iter()
                .map(|row| row.route_energy_wh)
                .fold(f64::NEG_INFINITY, f64::max),
            report.worst_energy_margin_scenario,
            report.worst_energy_margin_wh,
            report.lowest_lift_margin_scenario,
            report.lowest_lift_margin_coefficient,
            report.robust_screen_pass,
        );
    }
    Ok(())
}
