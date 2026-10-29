use std::{env, fs, process};

use civil_aero_benchmark::comparison::compare_candidates;
use civil_aero_benchmark::parse_cases_csv;

const COMPARISON_HEADER: &str = "case_id,parameter_basis,evidence_level,total_mass_kg,route_energy_wh,estimated_lift_to_drag,lift_screen_pass,energy_screen_pass,pareto_member,infeasible";

fn main() {
    let mut args = env::args();
    let program = args.next().unwrap_or_else(|| "pareto".to_owned());
    let input_path = args
        .next()
        .unwrap_or_else(|| "data/candidate_designs.csv".to_owned());
    if args.next().is_some() {
        eprintln!("usage: {program} [candidate-designs.csv]");
        process::exit(2);
    }

    let input = fs::read_to_string(&input_path).unwrap_or_else(|error| {
        eprintln!("cannot read {input_path}: {error}");
        process::exit(2);
    });
    let cases = parse_cases_csv(&input).unwrap_or_else(|error| {
        eprintln!("input rejected: {error}");
        process::exit(2);
    });
    let rows: Vec<_> = cases
        .iter()
        .map(|case| case.evaluate())
        .collect::<Result<_, _>>()
        .unwrap_or_else(|error| {
            eprintln!("evaluation rejected: {error}");
            process::exit(2);
        });
    let report = compare_candidates(&rows).unwrap_or_else(|error| {
        eprintln!("comparison rejected: {error}");
        process::exit(2);
    });

    println!("{COMPARISON_HEADER}");
    for row in &rows {
        let pareto_member = report.pareto_candidate_ids.iter().any(|id| id == &row.case.case_id);
        let infeasible = report.infeasible_candidate_ids.iter().any(|id| id == &row.case.case_id);
        println!(
            "{},{},screening_only,{:.8},{:.8},{:.8},{},{},{},{}",
            row.case.case_id,
            row.case.parameter_basis,
            row.total_mass_kg,
            row.route_energy_wh,
            row.estimated_lift_to_drag,
            row.lift_screen_pass,
            row.energy_screen_pass,
            pareto_member,
            infeasible,
        );
    }
    eprintln!(
        "model={} evidence={} eligible={} infeasible={} dominated={} pareto={}",
        report.model_version,
        report.evidence_level,
        report.eligible_candidate_ids.len(),
        report.infeasible_candidate_ids.len(),
        report.dominated_candidate_ids.len(),
        report.pareto_candidate_ids.join(";"),
    );
}
