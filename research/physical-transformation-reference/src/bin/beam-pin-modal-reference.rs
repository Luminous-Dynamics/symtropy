// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic evidence receipt for the single-element-per-member released-
//! joint beam assembly and modal solve. This is not the refined NBC05 model.

use physical_transformation_reference::beam_modal::{
    BeamModalMode, BeamModeClass, solve_beam_pin_assembly_modes,
};
use physical_transformation_reference::beam_pin_assembly::{
    BeamPinMember, BeamPinNode, assemble_released_joint_pin_truss,
    canonical_assembly_request,
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
            eprintln!("beam-pin-modal-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let nodes = fixture_nodes();
    let members = fixture_members();
    let request = canonical_assembly_request(&nodes, &members)?;
    if request != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical modal request differs from frozen fixture".to_owned());
    }

    let assembly = assemble_released_joint_pin_truss(&nodes, &members)?;
    let result = solve_beam_pin_assembly_modes(&assembly)?;
    if result.modes.len() != assembly.free_dof_indices.len() {
        return Err("modal result did not preserve the free-DOF mode count".to_owned());
    }
    if result
        .modes
        .iter()
        .any(|mode| mode.class == BeamModeClass::NegativeOrUnstable)
    {
        return Err("negative/unstable eigenvalue is present in the nominal fixture".to_owned());
    }
    if result.modes.iter().any(|mode| {
        mode.class != BeamModeClass::Elastic
            || mode.frequency_hz.is_none()
            || !mode.normalized_eigen_residual.is_finite()
            || !mode.mass_norm_residual.is_finite()
    }) {
        return Err("nominal fixture contains a non-elastic or non-finite mode".to_owned());
    }

    let modes_json = result
        .modes
        .iter()
        .enumerate()
        .map(|(index, mode)| mode_json(index + 1, mode))
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-pin-modal-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-PIN-MODAL-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-generalized-eigenproblem\",",
            "\"result\":\"pass\",",
            "\"model_class\":\"one-euler-bernoulli-element-per-member-with-ideal-pin-rotations\",",
            "\"global_dof_count\":{},",
            "\"free_dof_indices\":{},",
            "\"returned_mode_count\":{},",
            "\"zero_mode_relative_tolerance\":{},",
            "\"maximum_normalized_eigen_residual\":{},",
            "\"maximum_mass_norm_residual\":{},",
            "\"maximum_off_diagonal_mass_orthogonality_residual\":{},",
            "\"modes\":[{}],",
            "\"claim_ceiling\":\"synthetic one-element-per-member modal solve; not a mesh-converged NBC05 beam-spectrum reproduction, physical validation, structural safety or code compliance\"",
            "}}"
        ),
        json_quote(&source_commit),
        json_quote(&request),
        assembly.stiffness_global.len(),
        usize_array_json(&assembly.free_dof_indices),
        result.modes.len(),
        float_json(result.zero_mode_relative_tolerance),
        float_json(result.maximum_normalized_eigen_residual),
        float_json(result.maximum_mass_norm_residual),
        float_json(result.maximum_off_diagonal_mass_orthogonality_residual),
        modes_json,
    ))
}

fn fixture_nodes() -> Vec<BeamPinNode> {
    vec![
        BeamPinNode {
            id: "A".to_owned(),
            x_m: 0.0,
            y_m: 1.0,
            restrain_x: true,
            restrain_y: true,
        },
        BeamPinNode {
            id: "B".to_owned(),
            x_m: 0.0,
            y_m: 0.0,
            restrain_x: true,
            restrain_y: true,
        },
        BeamPinNode {
            id: "C".to_owned(),
            x_m: 1.0,
            y_m: 1.0,
            restrain_x: false,
            restrain_y: false,
        },
    ]
}

fn fixture_members() -> Vec<BeamPinMember> {
    vec![
        BeamPinMember {
            id: "AC".to_owned(),
            start_node_id: "A".to_owned(),
            end_node_id: "C".to_owned(),
            area_m2: 3.141592653589793e-4,
            youngs_modulus_pa: 210.0e9,
            density_kg_m3: 7800.0,
            second_moment_m4: 7.853981633974483e-9,
        },
        BeamPinMember {
            id: "BC".to_owned(),
            start_node_id: "B".to_owned(),
            end_node_id: "C".to_owned(),
            area_m2: 3.141592653589793e-4,
            youngs_modulus_pa: 210.0e9,
            density_kg_m3: 7800.0,
            second_moment_m4: 7.853981633974483e-9,
        },
    ]
}

fn mode_json(index: usize, mode: &BeamModalMode) -> String {
    let frequency = mode
        .frequency_hz
        .map(float_json)
        .unwrap_or_else(|| "null".to_owned());
    let class = match mode.class {
        BeamModeClass::Elastic => "elastic",
        BeamModeClass::NearZeroOrMechanism => "near-zero-or-mechanism",
        BeamModeClass::NegativeOrUnstable => "negative-or-unstable",
    };
    format!(
        concat!(
            "{{",
            "\"mode_index\":{},",
            "\"eigenvalue_rad_s_squared\":{},",
            "\"frequency_hz\":{},",
            "\"class\":{},",
            "\"normalized_eigen_residual\":{},",
            "\"mass_norm_residual\":{},",
            "\"shape_global_dofs\":[{}]",
            "}}"
        ),
        index,
        float_json(mode.eigenvalue_rad_s_squared),
        frequency,
        json_quote(class),
        float_json(mode.normalized_eigen_residual),
        float_json(mode.mass_norm_residual),
        mode.shape_global_dofs
            .iter()
            .map(|value| float_json(*value))
            .collect::<Vec<_>>()
            .join(","),
    )
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-pin-modal-reference --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn float_json(value: f64) -> String {
    format!("{value:.17e}")
}

fn json_quote(value: &str) -> String {
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

fn usize_array_json(values: &[usize]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(usize::to_string)
            .collect::<Vec<_>>()
            .join(",")
    )
}

#[cfg(test)]
mod tests {
    use super::parse_source_commit;

    #[test]
    fn source_commit_argument_is_required_and_strict() {
        assert!(parse_source_commit().is_err());
    }
}
