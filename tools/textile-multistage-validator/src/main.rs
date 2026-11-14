use textile_multistage_validator::{fixtures::two_job_four_stage_case, validate};

fn main() {
    let (problem, schedule) = two_job_four_stage_case();
    let report = validate(&problem, &schedule);

    if report.valid {
        println!(
            "PASS {}: {} assignments, makespan {} ticks",
            report.problem_name,
            report.supplied_assignments,
            report.makespan_ticks.unwrap_or_default()
        );
        return;
    }

    eprintln!("FAIL {}:", report.problem_name);
    for item in report.findings {
        eprintln!("{} [{}]: {}", item.code, item.entity_id, item.message);
    }
    std::process::exit(2);
}
