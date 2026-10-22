// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic request-bound frequency ladder for the first
//! released-joint beam-mesh convergence experiment. Mode pairing is by
//! frequency rank only; this receipt does not claim MAC-based tracking.

use physical_transformation_reference::beam_mesh_convergence::{
    canonical_beam_mesh_convergence_request, run_beam_mesh_convergence,
};
use physical_transformation_reference::beam_pin_assembly::nafems_nbc05_released_joint_beam_fixture;
use std::env;
use std::process::ExitCode;

const FROZEN_REQUEST: &str =
    include_str!("../../fixtures/beam_mesh_convergence_request_v0_1.txt");
const ELEMENT_COUNTS: [usize; 4] = [1, 2, 4, 8];
const REQUESTED_MODES: usize = 6;
const MONOTONIC_TOLERANCE: f64 = 1.0e-8;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-mesh-convergence-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let (nodes, members) = nafems_nbc05_released_joint_beam_fixture();
    let canonical = canonical_beam_mesh_convergence_request(
        &nodes,
        &members,
        &ELEMENT_COUNTS,
        REQUESTED_MODES,
        MONOTONIC_TOLERANCE,
    )?;
    if canonical != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical beam mesh-convergence request differs from frozen fixture bytes".to_owned());
    }
    let report = run_beam_mesh_convergence(
        &nodes,
        &members,
        &ELEMENT_COUNTS,
        REQUESTED_MODES,
        MONOTONIC_TOLERANCE,
    )?;
    if report.points.len() != ELEMENT_COUNTS.len() {
        return Err("mesh sweep returned an unexpected number of points".to_owned());
    }

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-mesh-convergence-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-MESH-CONVERGENCE-001\",",
            "\"references\":{{\"challenge\":\"https://www.nafems.org/downloads/nbc5_challenge.pdf\",\"solution\":\"https://www.nafems.org/downloads/nbr5v3.pdf\"}},",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-software-mesh-convergence-run\",",
            "\"result\":\"pass\",",
            "\"elements_per_member\":{},",
            "\"requested_mode_count\":{},",
            "\"monotonic_relative_tolerance\":{},",
            "\"mode_pairing\":\"frequency-ordinal-only; MAC not claimed\",",
            "\"points\":{},",
            "\"claim_ceiling\":\"1/2/4/8 element-per-member frequency trend only; no MAC mode-shape tracking, 200-element/member reproduction, physical validation or structural safety\",",
            "\"physical_comparator_status\":\"approximate beam markers in the NAFEMS figure are not asserted as numerical golden targets\"",
            "}}"
        ),
        json_quote(&source_commit),
        json_quote(&canonical),
        usize_array_json(&report.elements_per_member),
        report.requested_mode_count,
        number(report.monotonic_relative_tolerance),
        points_json(&report.points),
    ))
}

fn points_json(points: &[physical_transformation_reference::beam_mesh_convergence::BeamMeshConvergencePoint]) -> String {
    format!(
        "[{}]",
        points
            .iter()
            .map(|point| {
                format!(
                    concat!(
                        "{{",
                        "\"elements_per_member\":{},",
                        "\"total_element_count\":{},",
                        "\"interior_node_count\":{},",
                        "\"free_dof_count\":{},",
                        "\"frequencies_hz\":{},",
                        "\"relative_frequency_delta_from_previous\":{},",
                        "\"maximum_mass_orthogonality_residual\":{},",
                        "\"maximum_normalized_eigen_residual\":{},",
                        "\"maximum_mass_norm_residual\":{}",
                        "}}"
                    ),
                    point.elements_per_member,
                    point.total_element_count,
                    point.interior_node_count,
                    point.free_dof_count,
                    float_array_json(&point.frequencies_hz),
                    optional_float_array_json(&point.relative_frequency_delta_from_previous),
                    number(point.maximum_mass_orthogonality_residual),
                    number(point.maximum_normalized_eigen_residual),
                    number(point.maximum_mass_norm_residual),
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn optional_float_array_json(values: &Option<Vec<f64>>) -> String {
    match values {
        Some(values) => float_array_json(values),
        None => "null".to_owned(),
    }
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-mesh-convergence-reference --source-commit <40-char-git-sha>".to_owned(),
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

fn number(value: f64) -> String {
    format!("{value:.17e}")
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
    use super::parse_source_commit;

    #[test]
    fn source_commit_is_mandatory_and_normalized() {
        assert!(parse_source_commit().is_err());
    }
}
