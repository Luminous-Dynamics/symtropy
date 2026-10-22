// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic receipt for the 200-element banded-Lanczos BE25 candidate.
//! This is a numerical scaling result under the derived mass candidate, not a
//! BE25 reproduction while the official input deck remains uninspected.

use physical_transformation_reference::beam_line::{
    BeamLineBoundaryCase, BeamLineMassModel, solve_beam_line_modes,
};
use physical_transformation_reference::beam_line_sparse::{
    canonical_request, modal_assurance_criterion, solve_sparse_beam_line_modes,
};
use std::env;
use std::process::ExitCode;

const FIXTURE: &str =
    include_str!("../../fixtures/sofistik_be25_sparse_modal_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sofistik-be25-sparse-modal: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let request = canonical_request();
    if request != FIXTURE.trim_end_matches('\n') {
        return Err("canonical sparse modal request differs from frozen fixture".to_owned());
    }

    let solution = solve_sparse_beam_line_modes(
        1.0,
        200.0e6,
        1.0e-9,
        0.25492905324448206,
        200,
        BeamLineBoundaryCase::SimpleCantilever,
        6,
        120,
    )?;
    let mut dense_crosscheck_max_relative_frequency_error = 0.0_f64;
    let mut dense_crosscheck_min_mac = 1.0_f64;
    for boundary_case in BeamLineBoundaryCase::ALL {
        let dense = solve_beam_line_modes(
            1.0,
            200.0e6,
            1.0e-9,
            0.25492905324448206,
            10,
            boundary_case,
            BeamLineMassModel::ConsistentTranslational,
        )?;
        let sparse = solve_sparse_beam_line_modes(
            1.0,
            200.0e6,
            1.0e-9,
            0.25492905324448206,
            10,
            boundary_case,
            4,
            32,
        )?;
        for mode_index in 0..4 {
            let relative_error = (dense.modes[mode_index].frequency_hz
                - sparse.modes[mode_index].frequency_hz)
                .abs()
                / dense.modes[mode_index].frequency_hz;
            dense_crosscheck_max_relative_frequency_error =
                dense_crosscheck_max_relative_frequency_error.max(relative_error);
            let mac = modal_assurance_criterion(
                &dense.modes[mode_index].shape,
                &sparse.modes[mode_index].shape,
            )?;
            dense_crosscheck_min_mac = dense_crosscheck_min_mac.min(mac);
        }
    }

    let modes = solution
        .modes
        .iter()
        .enumerate()
        .map(|(index, mode)| {
            format!(
                concat!(
                    "{{",
                    "\"mode_index\":{},",
                    "\"eigenvalue_rad_s_squared\":{},",
                    "\"frequency_hz\":{},",
                    "\"normalized_eigen_residual\":{}",
                    "}}"
                ),
                index + 1,
                number(mode.eigenvalue_rad_s_squared),
                number(mode.frequency_hz),
                number(mode.normalized_eigen_residual),
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"sofistik-be25-sparse-modal-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BE25-SPARSE-MODAL-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"sparse-candidate-numerical-analysis\",",
            "\"result\":\"candidate-analysis-complete\",",
            "\"benchmark_disposition\":\"blocked-until-source-input-resolved\",",
            "\"model\":{{",
            "\"element_formulation\":\"Euler-Bernoulli continuous beam\",",
            "\"mass_model\":\"consistent translational mass only\",",
            "\"boundary_case\":\"simple-cantilever\",",
            "\"element_count\":{},",
            "\"total_dof_count\":{},",
            "\"free_dof_count\":{},",
            "\"requested_modes\":{},",
            "\"actual_lanczos_iterations\":{},",
            "\"mass_per_length_kg_per_m\":{},",
            "\"youngs_modulus_pa\":{},",
            "\"second_moment_m4\":{},",
            "\"length_m\":{}",
            "}},",
            "\"diagnostics\":{{",
            "\"analytical_first_frequency_hz\":{},",
            "\"first_frequency_hz\":{},",
            "\"first_frequency_relative_error\":{},",
            "\"maximum_normalized_eigen_residual\":{},",
            "\"maximum_modal_mass_norm_error\":{},",
            "\"maximum_off_diagonal_mass_orthogonality_error\":{},",
            "\"banded_matrix_storage_bytes\":{},",
            "\"lanczos_basis_storage_bytes\":{},",
            "\"dense_stiffness_mass_storage_estimate_bytes\":{},",
            "\"banded_matrix_storage_fraction_of_dense\":{},",
            "\"dense_crosscheck_max_relative_frequency_error\":{},",
            "\"dense_crosscheck_min_mac\":{},",
            "}},",
            "\"modes\":[{}],",
            "\"claim_ceiling\":\"200-element continuous beam candidate only; not BE25 reproduction or physical validation\"",
            "}}"
        ),
        quote(&source_commit),
        quote(request),
        solution.element_count,
        solution.total_dof_count,
        solution.free_dof_count,
        solution.requested_modes,
        solution.actual_lanczos_iterations,
        number(0.25492905324448206),
        number(200.0e6),
        number(1.0e-9),
        number(1.0),
        number(solution.analytical_first_frequency_hz),
        number(solution.modes[0].frequency_hz),
        number(solution.first_frequency_relative_error),
        number(solution.maximum_normalized_eigen_residual),
        number(solution.maximum_modal_mass_norm_error),
        number(solution.maximum_off_diagonal_mass_orthogonality_error),
        solution.matrix_storage_bytes,
        solution.lanczos_basis_storage_bytes,
        solution.dense_stiffness_mass_storage_bytes,
        number(
            solution.matrix_storage_bytes as f64
                / solution.dense_stiffness_mass_storage_bytes as f64,
        ),
        number(dense_crosscheck_max_relative_frequency_error),
        number(dense_crosscheck_min_mac),
        modes,
    ))
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: sofistik-be25-sparse-modal --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
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
    use super::{parse_source_commit, quote};

    #[test]
    fn source_commit_is_strictly_validated() {
        assert!(parse_source_commit().is_err());
    }

    #[test]
    fn json_quote_escapes_control_characters() {
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
