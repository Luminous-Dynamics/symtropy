// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic receipt emitter for the rotary-inertia mass sensitivity ladder.

use physical_transformation_reference::beam_mass_sensitivity::{
    BeamMassSensitivityMode, BeamMassSensitivityPoint, BeamMassSensitivityReport,
    canonical_beam_mass_sensitivity_request, run_beam_mass_formulation_sensitivity,
};
use physical_transformation_reference::beam_pin_assembly::nafems_nbc05_released_joint_beam_fixture;
use std::env;
use std::process::ExitCode;

const MESH_LEVELS: &[usize] = &[1, 2, 4, 8];
const REQUESTED_MODES: usize = 6;
const FIXED_REQUEST: &str =
    include_str!("../../fixtures/beam_mass_sensitivity_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-mass-formulation-sensitivity-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
    let request = canonical_beam_mass_sensitivity_request(
        &nodes,
        &members,
        MESH_LEVELS,
        REQUESTED_MODES,
    )?;
    if request != FIXED_REQUEST.trim_end_matches('\n') {
        return Err("canonical mass sensitivity request differs from the frozen fixture".to_owned());
    }
    let report =
        run_beam_mass_formulation_sensitivity(&nodes, &members, MESH_LEVELS, REQUESTED_MODES)?;
    Ok(receipt_json(&source_commit, &request, &report))
}

fn receipt_json(
    source_commit: &str,
    request: &str,
    report: &BeamMassSensitivityReport,
) -> String {
    let points = report.points.iter().map(point_json).collect::<Vec<_>>().join(",");
    let levels = report
        .elements_per_member
        .iter()
        .map(usize::to_string)
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-mass-formulation-sensitivity-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-MASS-SENS-001\",",
            "\"references\":{{\"nafems_challenge\":\"https://www.nafems.org/downloads/nbc5_challenge.pdf\",\"nafems_solution\":\"https://www.nafems.org/downloads/nbr5v3.pdf\",\"abaqus_beam_elements\":\"https://3dexperience.fiir.upb.ro/internal/docv6/english/SIMA3DXELMRefMap/simaelm-c-beamelem.htm\"}},",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"software-model-sensitivity-run\",",
            "\"result\":\"pass\",",
            "\"elements_per_member\":[{}],",
            "\"requested_mode_count\":{},",
            "\"rotation_characteristic_length_m\":{},",
            "\"minimum_combined_mac_for_confident_pairing\":{},",
            "\"consistent_mass_model\":{},",
            "\"sensitivity_counterfactual\":{},",
            "\"points\":[{}],",
            "\"physical_comparator_status\":\"not asserted as numerical golden targets\",",
            "\"claim_ceiling\":{}",
            "}}"
        ),
        json_quote(source_commit),
        json_quote(request),
        levels,
        report.requested_mode_count,
        number(report.rotation_characteristic_length_m),
        number(report.minimum_combined_mac_for_confident_pairing),
        json_quote(report.consistent_mass_model),
        json_quote(report.sensitivity_counterfactual),
        points,
        json_quote(report.claim_ceiling),
    )
}

fn point_json(point: &BeamMassSensitivityPoint) -> String {
    let modes = point.modes.iter().map(mode_json).collect::<Vec<_>>().join(",");
    format!(
        concat!(
            "{{",
            "\"elements_per_member\":{},",
            "\"total_element_count\":{},",
            "\"free_dof_count\":{},",
            "\"maximum_consistent_eigen_residual\":{},",
            "\"maximum_no_rotary_eigen_residual\":{},",
            "\"minimum_ordinal_relative_frequency_shift\":{},",
            "\"maximum_ordinal_relative_frequency_shift\":{},",
            "\"confident_shape_pair_count\":{},",
            "\"ambiguous_shape_pair_count\":{},",
            "\"modes\":[{}]",
            "}}"
        ),
        point.elements_per_member,
        point.total_element_count,
        point.free_dof_count,
        number(point.maximum_consistent_eigen_residual),
        number(point.maximum_no_rotary_eigen_residual),
        number(point.minimum_ordinal_relative_frequency_shift),
        number(point.maximum_ordinal_relative_frequency_shift),
        point.confident_shape_pair_count,
        point.ambiguous_shape_pair_count,
        modes,
    )
}

fn mode_json(mode: &BeamMassSensitivityMode) -> String {
    format!(
        concat!(
            "{{",
            "\"consistent_mode_ordinal\":{},",
            "\"no_rotary_shape_paired_mode_ordinal\":{},",
            "\"consistent_frequency_hz\":{},",
            "\"no_rotary_frequency_at_same_ordinal_hz\":{},",
            "\"no_rotary_frequency_shape_paired_hz\":{},",
            "\"relative_frequency_shift_by_ordinal\":{},",
            "\"relative_frequency_shift_by_shape\":{},",
            "\"mac\":{{",
            "\"translation_mac\":{},",
            "\"rotation_mac\":{},",
            "\"scaled_combined_mac\":{}",
            "}},",
            "\"shape_pairing_status\":{}",
            "}}"
        ),
        mode.consistent_mode_ordinal,
        mode.no_rotary_shape_paired_mode_ordinal,
        number(mode.consistent_frequency_hz),
        number(mode.no_rotary_frequency_at_same_ordinal_hz),
        number(mode.no_rotary_frequency_shape_paired_hz),
        number(mode.relative_frequency_shift_by_ordinal),
        number(mode.relative_frequency_shift_by_shape),
        optional_number(mode.mac.translation_mac),
        optional_number(mode.mac.rotation_mac),
        number(mode.mac.scaled_combined_mac),
        json_quote(mode.shape_pairing_status),
    )
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_else(|| "null".to_owned())
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn json_quote(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('\"');
    output
}

fn parse_source_commit() -> Result<String, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-mass-formulation-sensitivity-reference --source-commit <40-char-git-sha>"
                .to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{json_quote, parse_source_commit};

    #[test]
    fn source_commit_argument_is_strict_and_json_strings_are_escaped() {
        assert!(parse_source_commit().is_err());
        assert_eq!(
            json_quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
