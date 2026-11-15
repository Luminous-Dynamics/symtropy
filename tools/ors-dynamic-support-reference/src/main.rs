use ors_dynamic_support_reference::{compare_to_golden, evaluate, parse_corpus};
use std::env;
use std::process::ExitCode;

const FROZEN_CASES: &str = include_str!("../cases-v0.1.csv");

fn valid_commit(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}

fn main() -> ExitCode {
    let source_commit = match env::var("SOURCE_COMMIT") {
        Ok(value) if valid_commit(&value) => Some(value),
        Ok(_) => {
            eprintln!("SOURCE_COMMIT must be a 40-character lowercase Git SHA");
            return ExitCode::FAILURE;
        }
        Err(_) => None,
    };

    let cases = match parse_corpus(FROZEN_CASES) {
        Ok(cases) => cases,
        Err(error) => {
            eprintln!("corpus rejected: {error}");
            return ExitCode::FAILURE;
        }
    };

    let mut passed = 0usize;
    let mut rows = Vec::with_capacity(cases.len());
    for case in &cases {
        match evaluate(&case.scenario) {
            Ok(report) => {
                let pass = compare_to_golden(case, &report);
                if pass {
                    passed += 1;
                }
                rows.push(format!(
                    concat!(
                        "{{\"case_id\":\"{}\",\"status\":\"{}\",",
                        "\"time_steps\":{},",
                        "\"final_separation_m\":{:.12e},",
                        "\"final_relative_velocity_mps\":{:.12e},",
                        "\"peak_abs_separation_m\":{:.12e},",
                        "\"peak_abs_relative_velocity_mps\":{:.12e},",
                        "\"peak_abs_port_force_n\":{:.12e},",
                        "\"final_dissipated_energy_j\":{:.12e},",
                        "\"final_external_work_j\":{:.12e},",
                        "\"max_abs_linear_momentum_residual_kg_mps\":{:.12e},",
                        "\"max_abs_energy_balance_residual_j\":{:.12e}",
                        "}}"
                    ),
                    case.scenario.case_id,
                    if pass { "PASS" } else { "FAIL" },
                    report.time_steps,
                    report.final_separation_m,
                    report.final_relative_velocity_mps,
                    report.peak_abs_separation_m,
                    report.peak_abs_relative_velocity_mps,
                    report.peak_abs_port_force_n,
                    report.final_dissipated_energy_j,
                    report.final_external_work_j,
                    report.max_abs_linear_momentum_residual_kg_mps,
                    report.max_abs_energy_balance_residual_j
                ));
            }
            Err(error) => {
                rows.push(format!(
                    "{{\"case_id\":\"{}\",\"status\":\"ERROR\",\"reason\":\"{}\"}}",
                    case.scenario.case_id, error
                ));
            }
        }
    }

    let commit_json = source_commit
        .map(|value| format!("\"{value}\""))
        .unwrap_or_else(|| "null".to_owned());
    println!(
        concat!(
            "{{",
            "\"schema\":\"ors-dynamic-support-receipt-v0.1\",",
            "\"model\":\"two-mass-synthetic-linear-force-port\",",
            "\"force_law_id\":\"synthetic-linear-spring-damper-v1\",",
            "\"claim_class\":\"SYNTHETIC_NOT_PHYSICAL\",",
            "\"physical_qualification\":false,",
            "\"source_commit\":{},",
            "\"results\":[{}],",
            "\"summary\":{{\"passed\":{},\"failed\":{},\"total\":{}}}",
            "}}"
        ),
        commit_json,
        rows.join(","),
        passed,
        cases.len() - passed,
        cases.len()
    );

    if passed == cases.len() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    }
}
