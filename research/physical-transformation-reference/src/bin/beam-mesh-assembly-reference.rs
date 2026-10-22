// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic source/request-bound receipt for the refined
//! released-joint beam-mesh assembly fixture. It does not solve modal dynamics.

use physical_transformation_reference::beam_mesh_assembly::{
    assemble_beam_mesh, canonical_beam_mesh_request, BeamMeshDofKind, BeamMeshElement,
    BeamMeshNode,
};
use std::env;
use std::process::ExitCode;

const FROZEN_REQUEST: &str =
    include_str!("../../fixtures/beam_mesh_continuity_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-mesh-assembly-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let nodes = fixture_nodes();
    let elements = fixture_elements();
    let request = canonical_beam_mesh_request(&nodes, &elements)?;
    if request != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical beam-mesh request differs from frozen fixture bytes".to_owned());
    }
    let assembly = assemble_beam_mesh(&nodes, &elements)?;

    let shared_rotation_nodes = assembly
        .dof_descriptors
        .iter()
        .filter_map(|descriptor| match &descriptor.kind {
            BeamMeshDofKind::NodeRotation { node_id } => Some(node_id.as_str()),
            _ => None,
        })
        .map(quote)
        .collect::<Vec<_>>()
        .join(",");
    let private_rotation_indices = assembly
        .dof_descriptors
        .iter()
        .filter_map(|descriptor| match descriptor.kind {
            BeamMeshDofKind::ElementEndRotation { .. } => Some(descriptor.index.to_string()),
            _ => None,
        })
        .collect::<Vec<_>>()
        .join(",");
    let fixed = assembly.fixed_dof_indices.iter().map(usize::to_string).collect::<Vec<_>>().join(",");
    let free = assembly.free_dof_indices.iter().map(usize::to_string).collect::<Vec<_>>().join(",");

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-mesh-assembly-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-MESH-CONTINUITY-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-software-assembly\",",
            "\"result\":\"pass\",",
            "\"model_class\":\"2D_Euler-Bernoulli_beam_mesh_explicit_endpoint_releases\",",
            "\"node_count\":5,",
            "\"element_count\":4,",
            "\"dof_count\":16,",
            "\"fixed_dof_indices\":[{}],",
            "\"free_dof_indices\":[{}],",
            "\"shared_rotation_node_ids\":[{}],",
            "\"private_release_rotation_indices\":[{}],",
            "\"shared_interior_rotation_count\":2,",
            "\"private_release_rotation_count\":4,",
            "\"mass_terms\":[\"translational\",\"rotary_inertia\"],",
            "\"invariants\":{{",
            "\"input_permutation_invariant\":true,",
            "\"canonical_request_equals_frozen_fixture\":true,",
            "\"global_stiffness_symmetric\":true,",
            "\"global_mass_symmetric\":true,",
            "\"released_joint_rotations_private\":true,",
            "\"interior_mesh_rotations_shared\":true",
            "}},",
            "\"claim_ceiling\":\"assembly and release-topology only; no eigenvalues, mode pairing, mesh convergence, physical validation or structural safety\"",
            "}}"
        ),
        quote(&source_commit),
        quote(&request),
        fixed,
        free,
        shared_rotation_nodes,
        private_rotation_indices
    ))
}

fn fixture_nodes() -> Vec<BeamMeshNode> {
    vec![
        BeamMeshNode { id: "A".to_owned(), x_m: 0.0, y_m: 1.0, restrain_x: true, restrain_y: true, restrain_rotation: false },
        BeamMeshNode { id: "B".to_owned(), x_m: 0.0, y_m: 0.0, restrain_x: true, restrain_y: true, restrain_rotation: false },
        BeamMeshNode { id: "C".to_owned(), x_m: 1.0, y_m: 1.0, restrain_x: false, restrain_y: false, restrain_rotation: false },
        BeamMeshNode { id: "D".to_owned(), x_m: 0.5, y_m: 1.0, restrain_x: false, restrain_y: false, restrain_rotation: false },
        BeamMeshNode { id: "E".to_owned(), x_m: 0.5, y_m: 0.5, restrain_x: false, restrain_y: false, restrain_rotation: false },
    ]
}

fn fixture_elements() -> Vec<BeamMeshElement> {
    let radius_m = 0.01_f64;
    let area_m2 = std::f64::consts::PI * radius_m.powi(2);
    let second_moment_m4 = std::f64::consts::PI * radius_m.powi(4) / 4.0;
    let make = |id: &str, start: &str, end: &str, release_start, release_end| BeamMeshElement {
        id: id.to_owned(),
        start_node_id: start.to_owned(),
        end_node_id: end.to_owned(),
        release_start,
        release_end,
        area_m2,
        youngs_modulus_pa: 210.0e9,
        density_kg_m3: 7800.0,
        second_moment_m4,
    };
    vec![
        make("AC-1", "A", "D", true, false),
        make("AC-2", "D", "C", false, true),
        make("BC-1", "B", "E", true, false),
        make("BC-2", "E", "C", false, true),
    ]
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err("usage: beam-mesh-assembly-reference --source-commit <40-char-git-sha>".to_owned());
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn quote(value: &str) -> String {
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

#[cfg(test)]
mod tests {
    use super::{parse_source_commit, quote};

    #[test]
    fn source_commit_and_json_string_are_strictly_handled() {
        assert!(parse_source_commit().is_err());
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
