// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic, per-case conditioning receipt for the planar-truss oracle.
//!
//! No external solver is invoked. The shared library routine produces the exact same
//! diagnostics used by the unit test, including expected fail-closed cases.

use physical_transformation_reference::truss_conditioning::{
    ConditioningCaseReport, conditioning_sweep_report,
};
use std::{env, process::ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("planar-truss-conditioning: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut source_commit: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--source-commit" {
            if source_commit.is_some() {
                return Err("--source-commit may be supplied only once".to_owned());
            }
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--source-commit requires a value".to_owned())?;
            source_commit = Some(normalize_source_commit(value)?);
            index += 2;
        } else {
            return Err(format!("unknown argument: {}", args[index]));
        }
    }
    let source_commit = source_commit.ok_or_else(|| {
        "usage: planar-truss-conditioning --source-commit <40-char-git-sha>".to_owned()
    })?;

    let report = conditioning_sweep_report()?;
    let cases_json = report
        .cases
        .iter()
        .map(serialize_case)
        .collect::<Vec<_>>()
        .join(",");
    let source_commit = quote(&source_commit);
    let benchmark_id = quote(&report.benchmark_id);
    let claim_ceiling = quote(&report.claim_ceiling);

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"planar-truss-conditioning-receipt-v0.1\",",
            "\"benchmark_id\":{},",
            "\"source_commit\":{},",
            "\"evidence_class\":\"software-test-run\",",
            "\"result\":\"pass\",",
            "\"machine_epsilon\":{},",
            "\"condition_reject_threshold\":{},",
            "\"accepted_case_count\":{},",
            "\"rejected_case_count\":{},",
            "\"cases\":[{}],",
            "\"claim_ceiling\":{}",
            "}}"
        ),
        benchmark_id,
        source_commit,
        number(report.machine_epsilon),
        number(report.condition_reject_threshold),
        report.accepted_case_count,
        report.rejected_case_count,
        cases_json,
        claim_ceiling
    ))
}

fn serialize_case(case: &ConditioningCaseReport) -> String {
    let observed_displacement = case
        .observed_displacement_m
        .map(|value| format!("[{},{}]", number(value[0]), number(value[1])))
        .unwrap_or_else(|| "null".to_owned());
    let solver_error = case
        .solver_error
        .as_deref()
        .map(quote)
        .unwrap_or_else(|| "null".to_owned());
    format!(
        concat!(
            "{{",
            "\"case_index\":{},",
            "\"apex_height_m\":{},",
            "\"reduced_stiffness_n_per_m\":{{",
            "\"kxx\":{},\"kxy\":{},\"kyy\":{}}},",
            "\"determinant_n2_per_m2\":{},",
            "\"condition_infinity\":{},",
            "\"reciprocal_condition_infinity\":{},",
            "\"condition_times_machine_epsilon\":{},",
            "\"target_displacement_m\":[{},{}],",
            "\"applied_load_n\":[{},{}],",
            "\"disposition\":{},",
            "\"observed_displacement_m\":{},",
            "\"forward_error_relative\":{},",
            "\"backward_error_relative\":{},",
            "\"condition_amplified_backward_error\":{},",
            "\"forward_error_bound\":{},",
            "\"forward_error_acceptance_limit\":{},",
            "\"normalized_force_residual\":{},",
            "\"normalized_moment_residual\":{},",
            "\"normalized_free_dof_residual\":{},",
            "\"normalized_energy_residual\":{},",
            "\"solver_error\":{}",
            "}}"
        ),
        case.case_index,
        number(case.apex_height_m),
        number(case.reduced_kxx_n_per_m),
        number(case.reduced_kxy_n_per_m),
        number(case.reduced_kyy_n_per_m),
        number(case.determinant_n2_per_m2),
        number(case.condition_infinity),
        number(case.reciprocal_condition_infinity),
        number(case.condition_times_machine_epsilon),
        number(case.target_displacement_m[0]),
        number(case.target_displacement_m[1]),
        number(case.applied_load_n[0]),
        number(case.applied_load_n[1]),
        quote(&case.disposition),
        observed_displacement,
        optional_number(case.forward_error_relative),
        optional_number(case.backward_error_relative),
        optional_number(case.condition_amplified_backward_error),
        optional_number(case.forward_error_bound),
        optional_number(case.forward_error_acceptance_limit),
        optional_number(case.normalized_force_residual),
        optional_number(case.normalized_moment_residual),
        optional_number(case.normalized_free_dof_residual),
        optional_number(case.normalized_energy_residual),
        solver_error
    )
}

fn normalize_source_commit(value: &str) -> Result<String, String> {
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_else(|| "null".to_owned())
}

fn quote(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            '\n' => escaped.push_str("\\n"),
            '\r' => escaped.push_str("\\r"),
            '\t' => escaped.push_str("\\t"),
            character if character.is_control() => {
                escaped.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => escaped.push(character),
        }
    }
    escaped.push('\"');
    escaped
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_commit_must_be_a_well_formed_git_sha() {
        assert!(normalize_source_commit("not-a-sha").is_err());
        assert_eq!(
            normalize_source_commit("ABCDEF0123456789ABCDEF0123456789ABCDEF01")
                .expect("valid SHA"),
            "abcdef0123456789abcdef0123456789abcdef01"
        );
    }

    #[test]
    fn json_quote_escapes_delimiters_and_control_characters() {
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
