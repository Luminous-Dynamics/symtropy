// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic receipt reproducing the published NAFEMS NBC05
//! single-element-per-member axial-bar frequency table through an independent
//! two-DOF analytic oracle. This does not reproduce the structure's bending modes.

use physical_transformation_reference::modal_bar::MassMatrixFormulation;
use physical_transformation_reference::nafems_nbc05::{
    Nbc05AxialTrussResult, canonical_request, published_nbc05_parameters,
    solve_published_axial_truss,
};
use std::env;
use std::process::ExitCode;

const FIXED_REQUEST: &str =
    include_str!("../../fixtures/nafems_nbc05_axial_truss_request_v0_1.txt");
const REFERENCE_RELATIVE_TOLERANCE: f64 = 1.0e-4;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("nafems-nbc05-modal-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let parameters = published_nbc05_parameters();
    let canonical = canonical_request(parameters);
    if canonical != FIXED_REQUEST.trim_end_matches('\n') {
        return Err("canonical NAFEMS NBC05 request differs from frozen fixture bytes".to_owned());
    }

    let consistent = solve_published_axial_truss(parameters, MassMatrixFormulation::Consistent)?;
    let lumped = solve_published_axial_truss(parameters, MassMatrixFormulation::Lumped)?;
    verify_reference(&consistent, &lumped)?;

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"nafems-nbc05-axial-truss-receipt-v0.1\",",
            "\"references\":{\"challenge\":\"https://www.nafems.org/downloads/nbc5_challenge.pdf\",\"solution\":\"https://www.nafems.org/downloads/nbr5v3.pdf\"},",
            "\"benchmark_id\":\"PHYS-NAFEMS-NBC05-AXIAL-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"published-reference-reproduction\",",
            "\"result\":\"pass\",",
            "\"model_class\":\"2D_two_dof_pin_jointed_axial_truss_single_element_per_member\",",
            "\"inputs\":{{",
            "\"top_span_m\":{},",
            "\"vertical_offset_m\":{},",
            "\"youngs_modulus_pa\":{},",
            "\"poisson_ratio_recorded_not_used_by_axial_model\":{},",
            "\"density_kg_m3\":{},",
            "\"section_radius_assumption_m\":{},",
            "\"section_area_cancels_from_uniform_axial_frequency\":true,",
            "\"supports\":\"left_upper_and_left_lower_nodes_fixed_in_x_y\",",
            "\"free_joint\":\"top_right_node_has_x_y_translation\"",
            "}},",
            "\"cs1_consistent_mass\":{},",
            "\"cs2_lumped_mass\":{},",
            "\"acceptance\":{{",
            "\"published_table_relative_tolerance\":{},",
            "\"all_four_frequency_targets_within_tolerance\":true,",
            "\"consistent_lumped_ratio\":{},",
            "\"expected_consistent_lumped_ratio\":{}",
            "}},",
            "\"reference_interpretation\":\"matches_reported_NAFEMS_table_for_named_axial_bar_model_only\",",
            "\"claim_ceiling\":\"not_member_bending_or_rotary_inertia_not_full_structural_dynamic_validation_not_code_compliance_or_safety\"",
            "}}"
        ),
        quote(&source_commit),
        quote(&canonical),
        number(parameters.top_span_m),
        number(parameters.vertical_offset_m),
        number(parameters.youngs_modulus_pa),
        number(parameters.poisson_ratio),
        number(parameters.density_kg_m3),
        number(parameters.section_radius_m),
        result_json(&consistent),
        result_json(&lumped),
        number(REFERENCE_RELATIVE_TOLERANCE),
        number(lumped.frequencies_hz[0] / consistent.frequencies_hz[0]),
        number((2.0_f64 / 3.0).sqrt()),
    ))
}

fn verify_reference(
    consistent: &Nbc05AxialTrussResult,
    lumped: &Nbc05AxialTrussResult,
) -> Result<(), String> {
    for (formulation, result) in [("CS1 consistent", consistent), ("CS2 lumped", lumped)] {
        for (mode, error) in result.reference_relative_errors.iter().enumerate() {
            if !error.is_finite() || *error >= REFERENCE_RELATIVE_TOLERANCE {
                return Err(format!(
                    "{formulation} mode {} relative error {:.17e} exceeds {:.17e}",
                    mode + 1,
                    error,
                    REFERENCE_RELATIVE_TOLERANCE
                ));
            }
        }
    }

    let expected_ratio = (2.0_f64 / 3.0).sqrt();
    for mode in 0..2 {
        let actual_ratio = lumped.frequencies_hz[mode] / consistent.frequencies_hz[mode];
        if (actual_ratio - expected_ratio).abs() >= 1.0e-12 {
            return Err(format!(
                "mass-formulation ratio gate failed for mode {}: {:.17e}",
                mode + 1,
                actual_ratio
            ));
        }
    }
    Ok(())
}

fn result_json(result: &Nbc05AxialTrussResult) -> String {
    let stiffness = format!(
        "[[{},{}],[{},{}]]",
        number(result.reduced_stiffness_n_per_m[0][0]),
        number(result.reduced_stiffness_n_per_m[0][1]),
        number(result.reduced_stiffness_n_per_m[1][0]),
        number(result.reduced_stiffness_n_per_m[1][1]),
    );
    format!(
        concat!(
            "{{",
            "\"frequencies_hz\":[{},{}],",
            "\"reference_relative_errors\":[{},{}],",
            "\"reduced_stiffness_n_per_m\":{},",
            "\"effective_mass_kg\":{},",
            "\"section_area_m2\":{},",
            "\"member_lengths_m\":[{},{}]",
            "}}"
        ),
        number(result.frequencies_hz[0]),
        number(result.frequencies_hz[1]),
        number(result.reference_relative_errors[0]),
        number(result.reference_relative_errors[1]),
        stiffness,
        number(result.effective_mass_kg),
        number(result.section_area_m2),
        number(result.member_lengths_m[0]),
        number(result.member_lengths_m[1]),
    )
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: nafems-nbc05-modal-reference --source-commit <40-char-git-sha>".to_owned(),
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
    fn source_commit_is_strictly_validated_and_normalized() {
        assert!(parse_source_commit().is_err());
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
