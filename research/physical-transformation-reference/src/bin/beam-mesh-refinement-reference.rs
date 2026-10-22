// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic 1/2/4/8-element-per-member beam mesh-convergence receipt.
//! It deliberately stops at 128 total DOFs and makes no claim for the full
//! NAFEMS NBC05 200-elements-per-member beam spectrum.

use physical_transformation_reference::beam_mesh::{
    assemble_refined_released_joint_pin_truss, canonical_refinement_sweep_request,
};
use physical_transformation_reference::beam_modal::{
    BeamModeClass, solve_beam_pin_assembly_modes,
};
use physical_transformation_reference::beam_pin_assembly::{BeamPinMember, BeamPinNode};
use std::env;
use std::process::ExitCode;

const FROZEN_REQUEST: &str =
    include_str!("../../fixtures/beam_mesh_sweep_request_v0_1.txt");
const ELEMENT_COUNTS: [usize; 4] = [1, 2, 4, 8];

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-mesh-refinement-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let nodes = sample_nodes();
    let members = sample_members();
    let request = canonical_refinement_sweep_request(&nodes, &members, &ELEMENT_COUNTS)?;
    if request != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical beam-mesh sweep request differs from frozen fixture".to_owned());
    }

    let mut meshes_json = Vec::with_capacity(ELEMENT_COUNTS.len());
    let mut previous_fundamental: Option<f64> = None;
    for elements_per_member in ELEMENT_COUNTS {
        let assembly =
            assemble_refined_released_joint_pin_truss(&nodes, &members, elements_per_member)?;
        let solution = solve_beam_pin_assembly_modes(&assembly)?;
        if solution.modes.len() != assembly.free_dof_indices.len() {
            return Err(format!(
                "{}-element mesh returned an incomplete spectrum",
                elements_per_member
            ));
        }
        if solution
            .modes
            .iter()
            .any(|mode| mode.class != BeamModeClass::Elastic)
        {
            return Err(format!(
                "{}-element mesh has a near-zero/mechanism or negative/unstable mode; no pass receipt emitted",
                elements_per_member
            ));
        }
        let fundamental = solution.modes[0]
            .frequency_hz
            .ok_or_else(|| "fundamental mode has no positive frequency".to_owned())?;
        let shift_percent = previous_fundamental
            .map(|previous| number((fundamental / previous - 1.0) * 100.0))
            .unwrap_or_else(|| "null".to_owned());
        previous_fundamental = Some(fundamental);

        let modes_json = solution
            .modes
            .iter()
            .enumerate()
            .map(|(index, mode)| {
                let frequency = mode.frequency_hz.map(number).unwrap_or_else(|| "null".to_owned());
                let shape = mode
                    .shape_global_dofs
                    .iter()
                    .copied()
                    .map(number)
                    .collect::<Vec<_>>()
                    .join(",");
                format!(
                    concat!(
                        "{{",
                        "\"mode_number\":{},",
                        "\"eigenvalue_rad_s_squared\":{},",
                        "\"frequency_hz\":{},",
                        "\"class\":\"{}\",",
                        "\"normalized_eigen_residual\":{},",
                        "\"mass_norm_residual\":{},",
                        "\"shape_global_dofs\":[{}]",
                        "}}"
                    ),
                    index + 1,
                    number(mode.eigenvalue_rad_s_squared),
                    frequency,
                    class_name(mode.class),
                    number(mode.normalized_eigen_residual),
                    number(mode.mass_norm_residual),
                    shape
                )
            })
            .collect::<Vec<_>>()
            .join(",");

        meshes_json.push(format!(
            concat!(
                "{{",
                "\"elements_per_member\":{},",
                "\"total_dof_count\":{},",
                "\"free_dof_count\":{},",
                "\"mode_count\":{},",
                "\"mode_count_complete\":true,",
                "\"all_modes_elastic\":true,",
                "\"fundamental_frequency_hz\":{},",
                "\"relative_shift_percent_from_previous\":{},",
                "\"maximum_normalized_eigen_residual\":{},",
                "\"maximum_mass_norm_residual\":{},",
                "\"maximum_mass_orthogonality_residual\":{},",
                "\"modes\":[{}]",
                "}}"
            ),
            elements_per_member,
            assembly.stiffness_global.len(),
            assembly.free_dof_indices.len(),
            solution.modes.len(),
            number(fundamental),
            shift_percent,
            number(solution.maximum_normalized_eigen_residual),
            number(solution.maximum_mass_norm_residual),
            number(solution.maximum_off_diagonal_mass_orthogonality_residual),
            modes_json
        ));
    }

    let first_frequency = previous_first_fundamental(&meshes_json)?;
    let last_frequency = previous_last_fundamental(&meshes_json)?;
    if last_frequency > first_frequency * (1.0 + 1.0e-8) {
        return Err("fundamental frequency increased across nested refinement levels".to_owned());
    }

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-mesh-refinement-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-MESH-SWEEP-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"software-verification-mesh-sweep\",",
            "\"result\":\"pass\",",
            "\"mesh_count\":{},",
            "\"mesh_element_counts\":[1,2,4,8],",
            "\"fundamental_frequency_nonincreasing\":true,",
            "\"meshes\":[{}],",
            "\"claim_ceiling\":\"small dense Euler-Bernoulli pin-jointed beam mesh ladder only; capped at 128 total DOFs; not the NAFEMS NBC05 200-elements-per-member spectrum, physical validation or structural safety\"",
            "}}"
        ),
        quote(&source_commit),
        quote(&request),
        ELEMENT_COUNTS.len(),
        meshes_json.join(",")
    ))
}

fn previous_first_fundamental(meshes: &[String]) -> Result<f64, String> {
    meshes
        .first()
        .and_then(|mesh| numeric_json_field(mesh, "\"fundamental_frequency_hz\":"))
        .ok_or_else(|| "first mesh fundamental frequency is missing".to_owned())
}

fn previous_last_fundamental(meshes: &[String]) -> Result<f64, String> {
    meshes
        .last()
        .and_then(|mesh| numeric_json_field(mesh, "\"fundamental_frequency_hz\":"))
        .ok_or_else(|| "last mesh fundamental frequency is missing".to_owned())
}

fn numeric_json_field(text: &str, key: &str) -> Option<f64> {
    let rest = text.split_once(key)?.1;
    let token = rest.split([',', '}']).next()?;
    token.parse().ok()
}

fn class_name(class: BeamModeClass) -> &'static str {
    match class {
        BeamModeClass::Elastic => "elastic",
        BeamModeClass::NearZeroOrMechanism => "near-zero-or-mechanism",
        BeamModeClass::NegativeOrUnstable => "negative-or-unstable",
    }
}

fn parse_source_commit() -> Result<String, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-mesh-refinement-reference --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn sample_nodes() -> Vec<BeamPinNode> {
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

fn sample_members() -> Vec<BeamPinMember> {
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
