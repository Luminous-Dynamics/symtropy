// SPDX-License-Identifier: AGPL-3.0-or-later
//! Replays the 10-element SOFiSTiK BE25 line model under two explicitly
//! inconsistent mass-per-length candidates. The output is candidate evidence,
//! never a BE25 pass until the official input deck resolves the contradiction.

use physical_transformation_reference::beam_line::{
    BeamLineBoundaryCase, BeamLineMassModel, canonical_request, solve_beam_line_modes,
};
use std::env;
use std::process::ExitCode;

const FIXTURE: &str =
    include_str!("../../fixtures/sofistik_be25_line_candidate_request_v0_1.txt");
const MU_PRINTED_KG_PER_M: f64 = 25.0;
const MU_FROM_GAMMA_AREA_OVER_G_KG_PER_M: f64 = 0.25492905324448206;

fn main() -> ExitCode {
    match run() {
        Ok(value) => {
            println!("{value}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sofistik-be25-line-candidate: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let request = canonical_request();
    if request != FIXTURE.trim_end_matches('\n') {
        return Err("canonical BE25 candidate request differs from frozen fixture".to_owned());
    }

    let candidates = [
        ("gamma_area_over_g", MU_FROM_GAMMA_AREA_OVER_G_KG_PER_M),
        ("printed_mu", MU_PRINTED_KG_PER_M),
    ];
    let mut cases = Vec::new();
    let mut convergence = Vec::new();

    for boundary in BeamLineBoundaryCase::ALL {
        for (candidate_name, mass_per_length) in candidates {
            let solution = solve_beam_line_modes(
                1.0,
                200.0e6,
                1.0e-9,
                mass_per_length,
                10,
                boundary,
                BeamLineMassModel::ConsistentTranslational,
            )?;
            let frequencies = solution
                .modes
                .iter()
                .map(|mode| number(mode.frequency_hz))
                .collect::<Vec<_>>()
                .join(",");
            cases.push(format!(
                concat!(
                    "{{",
                    "\"boundary_case\":{},",
                    "\"mass_candidate\":{},",
                    "\"mass_per_length_kg_per_m\":{},",
                    "\"element_count\":10,",
                    "\"frequencies_hz\":[{}],",
                    "\"analytical_first_frequency_hz\":{},",
                    "\"published_reference_frequency_hz\":{},",
                    "\"relative_error_to_analytical\":{},",
                    "\"relative_error_to_published_reference\":{},",
                    "\"maximum_normalized_eigen_residual\":{}",
                    "}}"
                ),
                quote(boundary.name()),
                quote(candidate_name),
                number(mass_per_length),
                frequencies,
                number(solution.analytical_first_frequency_hz),
                number(solution.published_reference_frequency_hz),
                number(solution.first_frequency_relative_error),
                number(solution.first_frequency_relative_error_to_published_reference),
                number(solution.maximum_normalized_eigen_residual),
            ));
        }

        let ten = solve_beam_line_modes(
            1.0,
            200.0e6,
            1.0e-9,
            MU_FROM_GAMMA_AREA_OVER_G_KG_PER_M,
            10,
            boundary,
            BeamLineMassModel::ConsistentTranslational,
        )?;
        let thirty_two = solve_beam_line_modes(
            1.0,
            200.0e6,
            1.0e-9,
            MU_FROM_GAMMA_AREA_OVER_G_KG_PER_M,
            32,
            boundary,
            BeamLineMassModel::ConsistentTranslational,
        )?;
        convergence.push(format!(
            concat!(
                "{{",
                "\"boundary_case\":{},",
                "\"mesh_10_first_frequency_hz\":{},",
                "\"mesh_32_first_frequency_hz\":{},",
                "\"mesh_10_relative_error_to_analytical\":{},",
                "\"mesh_32_relative_error_to_analytical\":{},",
                "\"mesh_10_to_32_relative_frequency_change\":{}",
                "}}"
            ),
            quote(boundary.name()),
            number(ten.modes[0].frequency_hz),
            number(thirty_two.modes[0].frequency_hz),
            number(ten.first_frequency_relative_error),
            number(thirty_two.first_frequency_relative_error),
            number(
                (thirty_two.modes[0].frequency_hz - ten.modes[0].frequency_hz).abs()
                    / ten.modes[0].frequency_hz
            ),
        ));
    }

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"sofistik-be25-line-candidate-receipt-v0.1\",",
            "\"benchmark_id\":\"SOFiSTiK-2026-BE25\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"candidate-numerical-analysis\",",
            "\"result\":\"candidate-analysis-complete\",",
            "\"benchmark_disposition\":\"blocked-until-source-input-resolved\",",
            "\"source_pdf\":\"https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be25.pdf\",",
            "\"mass_model\":\"Euler-Bernoulli beam with consistent translational mass only\",",
            "\"candidate_case_count\":8,",
            "\"cases\":[{}],",
            "\"derived_mass_mesh_convergence\":[{}],",
            "\"claim_ceiling\":\"numerical candidate calculations under contradictory published inputs; not BE25 reproduction or physical validation\"",
            "}}"
        ),
        quote(&source_commit),
        quote(request),
        cases.join(","),
        convergence.join(","),
    ))
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: sofistik-be25-line-candidate --source-commit <40-char-git-sha>".to_owned(),
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
