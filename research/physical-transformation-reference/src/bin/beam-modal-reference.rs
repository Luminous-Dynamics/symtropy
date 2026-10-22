// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic source/request-bound receipt for the first
//! released-joint Euler-Bernoulli beam spectrum. This is a coarse two-member,
//! one-element-per-member model, not a converged NBC05 reproduction.

use physical_transformation_reference::beam_modal::{
    BeamModalOptions, solve_released_joint_beam_modes,
};
use physical_transformation_reference::beam_pin_assembly::{
    assemble_released_joint_pin_truss, canonical_assembly_request,
    nafems_nbc05_released_joint_beam_fixture,
};
use std::env;
use std::process::ExitCode;

const FROZEN_REQUEST: &str =
    include_str!("../../fixtures/beam_pin_assembly_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-modal-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
    let canonical = canonical_assembly_request(&nodes, &members)?;
    if canonical != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical beam-modal request differs from frozen fixture bytes".to_owned());
    }
    let assembly = assemble_released_joint_pin_truss(&nodes, &members)?;
    let spectrum = solve_released_joint_beam_modes(
        &assembly,
        BeamModalOptions::first_modes(assembly.free_dof_indices.len()),
    )?;
    if spectrum.returned_mode_count != spectrum.total_free_dof_count {
        return Err("this receipt requires the complete free-DOF spectrum".to_owned());
    }

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"released-joint-beam-modal-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-MODAL-001\",",
            "\"references\":{{\"challenge\":\"https://www.nafems.org/downloads/nbc5_challenge.pdf\",\"solution\":\"https://www.nafems.org/downloads/nbr5v3.pdf\"}},",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-software-modal-run\",",
            "\"result\":\"pass\",",
            "\"model_class\":\"two-member-one-element-per-member-released-joint-euler-bernoulli-frame\",",
            "\"free_dof_count\":{},",
            "\"fixed_dof_indices\":{},",
            "\"free_dof_indices\":{},",
            "\"mechanism_eigenvalue_relative_tolerance\":{},",
            "\"reference_eigenvalue_scale_rad_s_squared\":{},",
            "\"maximum_mass_orthogonality_residual\":{},",
            "\"maximum_normalized_eigen_residual\":{},",
            "\"maximum_mass_norm_residual\":{},",
            "\"modes\":{},",
            "\"claim_ceiling\":\"coarse one-element-per-member modal prototype only; no full NBC05 convergence, experimental validation, structural safety or code compliance\",",
            "\"physical_comparator_status\":\"source beam-mode markers are graphical approximate values and are not asserted as a golden numerical target\"",
            "}}"
        ),
        json_quote(&source_commit),
        json_quote(&canonical),
        spectrum.total_free_dof_count,
        usize_array_json(&assembly.fixed_dof_indices),
        usize_array_json(&assembly.free_dof_indices),
        number(spectrum.mechanism_eigenvalue_relative_tolerance),
        number(spectrum.reference_eigenvalue_scale_rad_s_squared),
        number(spectrum.maximum_mass_orthogonality_residual),
        number(spectrum.maximum_normalized_eigen_residual),
        number(spectrum.maximum_mass_norm_residual),
        modes_json(&spectrum.modes),
    ))
}

fn modes_json(modes: &[physical_transformation_reference::beam_modal::BeamModalMode]) -> String {
    format!(
        "[{}]",
        modes
            .iter()
            .map(|mode| {
                format!(
                    concat!(
                        "{{",
                        "\"ordinal\":{},",
                        "\"eigenvalue_rad_s_squared\":{},",
                        "\"frequency_hz\":{},",
                        "\"normalized_eigen_residual\":{},",
                        "\"mass_norm_residual\":{},",
                        "\"shape_free_dofs\":{},",
                        "\"shape_global_dofs\":{}",
                        "}}"
                    ),
                    mode.ordinal,
                    number(mode.eigenvalue_rad_s_squared),
                    number(mode.frequency_hz),
                    number(mode.normalized_eigen_residual),
                    number(mode.mass_norm_residual),
                    float_array_json(&mode.shape_free_dofs),
                    float_array_json(&mode.shape_global_dofs),
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-modal-reference --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn json_quote(value: &str) -> String {
    let mut escaped = String::from("\"");
    for character in value.chars() {
        match character {
            '"' => escaped.push_str("\\\""),
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
    escaped.push('"');
    escaped
}

fn usize_array_json(values: &[usize]) -> String {
    format!(
        "[{}]",
        values.iter().map(usize::to_string).collect::<Vec<_>>().join(",")
    )
}

fn float_array_json(values: &[f64]) -> String {
    format!(
        "[{}]",
        values.iter().map(|value| number(*value)).collect::<Vec<_>>().join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::{json_quote, parse_source_commit};

    #[test]
    fn source_commit_parser_and_json_escaping_are_strict() {
        assert!(parse_source_commit().is_err());
        assert_eq!(json_quote("quote\" slash\\ newline\n"), "\"quote\\\" slash\\\\ newline\\n\"");
    }
}
