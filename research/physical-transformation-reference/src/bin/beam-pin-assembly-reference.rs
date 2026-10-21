// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic request-bound receipt for the released-joint beam
//! assembly fixture. This does not solve a generalized eigenproblem.

use physical_transformation_reference::beam_pin_assembly::{
    BeamPinDofKind, BeamPinMember, BeamPinNode, assemble_released_joint_pin_truss,
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
            eprintln!("beam-pin-assembly-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let nodes = fixture_nodes();
    let members = fixture_members();
    let canonical_request = canonical_assembly_request(&nodes, &members)?;
    if canonical_request != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical assembly request differs from frozen fixture bytes".to_owned());
    }
    let assembly = assemble_released_joint_pin_truss(&nodes, &members)?;
    let rotation_indices = assembly
        .dof_descriptors
        .iter()
        .filter_map(|descriptor| match &descriptor.kind {
            BeamPinDofKind::MemberEndRotation { .. } => Some(descriptor.index),
            _ => None,
        })
        .collect::<Vec<_>>();

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-pin-assembly-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-PIN-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-software-assembly\",",
            "\"result\":\"pass\",",
            "\"model_class\":\"planar-euler-bernoulli-members-with-ideal-pin-joints\",",
            "\"ordered_node_ids\":{},",
            "\"ordered_member_ids\":{},",
            "\"dof_descriptors\":{},",
            "\"dof_count\":{},",
            "\"fixed_dof_indices\":{},",
            "\"free_dof_indices\":{},",
            "\"member_end_rotation_indices\":{},",
            "\"stiffness_global\":{},",
            "\"mass_global\":{},",
            "\"translational_mass_global\":{},",
            "\"rotary_inertia_mass_global\":{},",
            "\"claim_ceiling\":\"assembly and DOF-map verification only; no eigen-solution, NBC05 beam-spectrum reproduction, physical validation or structural safety\"",
            "}}"
        ),
        json_quote(&source_commit),
        json_quote(&canonical_request),
        string_array_json(&assembly.ordered_node_ids),
        string_array_json(&assembly.ordered_member_ids),
        descriptors_json(&assembly.dof_descriptors),
        assembly.dof_descriptors.len(),
        usize_array_json(&assembly.fixed_dof_indices),
        usize_array_json(&assembly.free_dof_indices),
        usize_array_json(&rotation_indices),
        matrix_json(&assembly.stiffness_global),
        matrix_json(&assembly.mass_global),
        matrix_json(&assembly.translational_mass_global),
        matrix_json(&assembly.rotary_inertia_mass_global),
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

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-pin-assembly-reference --source-commit <40-char-git-sha>".to_owned(),
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

fn string_array_json(values: &[String]) -> String {
    format!(
        "[{}]",
        values
            .iter()
            .map(|value| json_quote(value))
            .collect::<Vec<_>>()
            .join(",")
    )
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

fn matrix_json(matrix: &[Vec<f64>]) -> String {
    format!(
        "[{}]",
        matrix
            .iter()
            .map(|row| {
                format!(
                    "[{}]",
                    row.iter()
                        .map(|value| format!("{value:.17e}"))
                        .collect::<Vec<_>>()
                        .join(",")
                )
            })
            .collect::<Vec<_>>()
            .join(",")
    )
}

fn descriptors_json(
    descriptors: &[physical_transformation_reference::beam_pin_assembly::BeamPinDofDescriptor],
) -> String {
    format!(
        "[{}]",
        descriptors
            .iter()
            .map(|descriptor| {
                let kind = match &descriptor.kind {
                    BeamPinDofKind::NodeTranslationX { node_id } => {
                        format!(
                            "\"kind\":\"node-translation-x\",\"node_id\":{}",
                            json_quote(node_id)
                        )
                    }
                    BeamPinDofKind::NodeTranslationY { node_id } => {
                        format!(
                            "\"kind\":\"node-translation-y\",\"node_id\":{}",
                            json_quote(node_id)
                        )
                    }
                    BeamPinDofKind::MemberEndRotation { member_id, at_start } => {
                        format!(
                            "\"kind\":\"member-end-rotation\",\"member_id\":{},\"at_start\":{}",
                            json_quote(member_id),
                            at_start
                        )
                    }
                    BeamPinDofKind::MemberInteriorRotation {
                        member_id,
                        station_index,
                    } => {
                        format!(
                            "\"kind\":\"member-interior-rotation\",\"member_id\":{},\"station_index\":{}",
                            json_quote(member_id),
                            station_index
                        )
                    }
                };
                format!("{{\"index\":{}, {kind}}}", descriptor.index)
            })
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
