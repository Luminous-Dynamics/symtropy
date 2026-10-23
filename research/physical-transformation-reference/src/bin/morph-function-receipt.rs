// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic request-bound receipt for the finite morphology/function benchmark.
//!
//! This receipt computes outputs with the existing 2D axial-truss reference solver.
//! It does not claim physical validation, code compliance, or safety certification.

use physical_transformation_reference::planar_truss::{
    solve, PlanarTrussInput, PlanarTrussResult, TrussBar, TrussNode,
};
use std::env;
use std::process::ExitCode;

const REQUEST_SHA256: &str = "18c78aa05c911dcb0b7566ac670565d27774cb73e82b46424fcaf420097a499e";
const AREA_M2: f64 = 1.0e-4;
const YOUNGS_MODULUS_PA: f64 = 200.0e9;
const BASE_LOAD_N: f64 = 2_000.0;
const LOAD_DISPLACEMENT_LIMIT_M: f64 = 3.0e-4;
const GRID_DISPLACEMENT_LIMIT_M: f64 = 120.0e-6;
const DISPLACEMENT_TOLERANCE_M: f64 = 2.0e-12;
const RESIDUAL_LIMIT: f64 = 1.0e-12;
const VARIATION_FACTORS: [f64; 3] = [0.95, 1.0, 1.05];

#[derive(Clone, Copy)]
struct LoadCase {
    id: &'static str,
    c_fx_n: f64,
    c_fy_n: f64,
    d_fx_n: f64,
    d_fy_n: f64,
    expected_ad: [f64; 4],
    expected_bc: [f64; 4],
}

fn load_cases() -> [LoadCase; 5] {
    [
        LoadCase {
            id: "symmetric_down_2kN",
            c_fx_n: 0.0,
            c_fy_n: -2_000.0,
            d_fx_n: 0.0,
            d_fy_n: -2_000.0,
            expected_ad: [100e-6, -100e-6, 100e-6, -100e-6],
            expected_bc: [-100e-6, -100e-6, -100e-6, -100e-6],
        },
        LoadCase {
            id: "asymmetric_down",
            c_fx_n: 0.0,
            c_fy_n: -1_000.0,
            d_fx_n: 0.0,
            d_fy_n: -500.0,
            expected_ad: [25e-6, -50e-6, 25e-6, -25e-6],
            expected_bc: [-50e-6, -50e-6, -50e-6, -25e-6],
        },
        LoadCase {
            id: "lateral_right_at_C",
            c_fx_n: 1_000.0,
            c_fy_n: 0.0,
            d_fx_n: 0.0,
            d_fy_n: 0.0,
            expected_ad: [241.4213562373096e-6, 0.0, 191.42135623730958e-6, -50e-6],
            expected_bc: [191.4213562373096e-6, 50e-6, 191.42135623730964e-6, 0.0],
        },
        LoadCase {
            id: "lateral_left_at_C",
            c_fx_n: -1_000.0,
            c_fy_n: 0.0,
            d_fx_n: 0.0,
            d_fy_n: 0.0,
            expected_ad: [-241.4213562373096e-6, 0.0, -191.42135623730958e-6, 50e-6],
            expected_bc: [-191.4213562373096e-6, -50e-6, -191.42135623730964e-6, 0.0],
        },
        LoadCase {
            id: "symmetric_uplift_1kN",
            c_fx_n: 0.0,
            c_fy_n: 1_000.0,
            d_fx_n: 0.0,
            d_fy_n: 1_000.0,
            expected_ad: [-50e-6, 50e-6, -50e-6, 50e-6],
            expected_bc: [50e-6, 50e-6, 50e-6, 50e-6],
        },
    ]
}

fn node(
    id: &str,
    x_m: f64,
    y_m: f64,
    supported: bool,
    force_x_n: f64,
    force_y_n: f64,
) -> TrussNode {
    TrussNode {
        id: id.to_owned(),
        x_m,
        y_m,
        restrained_x: supported,
        restrained_y: supported,
        force_x_n,
        force_y_n,
    }
}

fn bar(
    id: &str,
    node_i: &str,
    node_j: &str,
    area_m2: f64,
    youngs_modulus_pa: f64,
) -> TrussBar {
    TrussBar {
        id: id.to_owned(),
        node_i: node_i.to_owned(),
        node_j: node_j.to_owned(),
        area_m2,
        youngs_modulus_pa,
    }
}

fn truss(
    diagonal: Option<(&str, &str)>,
    area_scale: f64,
    modulus_scale: f64,
    load_scale: f64,
    loads: Option<LoadCase>,
) -> PlanarTrussInput {
    let load = loads.unwrap_or(LoadCase {
        id: "parameter-grid",
        c_fx_n: 0.0,
        c_fy_n: -BASE_LOAD_N * load_scale,
        d_fx_n: 0.0,
        d_fy_n: -BASE_LOAD_N * load_scale,
        expected_ad: [0.0; 4],
        expected_bc: [0.0; 4],
    });
    let nodes = vec![
        node("A", 0.0, 0.0, true, 0.0, 0.0),
        node("B", 1.0, 0.0, true, 0.0, 0.0),
        node("C", 0.0, 1.0, false, load.c_fx_n, load.c_fy_n),
        node("D", 1.0, 1.0, false, load.d_fx_n, load.d_fy_n),
    ];
    let area = AREA_M2 * area_scale;
    let modulus = YOUNGS_MODULUS_PA * modulus_scale;
    let mut bars = vec![
        bar("AB", "A", "B", area, modulus),
        bar("AC", "A", "C", area, modulus),
        bar("BD", "B", "D", area, modulus),
        bar("CD", "C", "D", area, modulus),
    ];
    if let Some((a, b)) = diagonal {
        bars.push(bar(&format!("{a}{b}"), a, b, area, modulus));
    }
    PlanarTrussInput { nodes, bars }
}

fn finite_below(value: f64, limit: f64) -> bool {
    value.is_finite() && value >= 0.0 && value < limit
}

fn close_to(actual: f64, expected: f64, label: &str) -> Result<(), String> {
    if !actual.is_finite() || (actual - expected).abs() > DISPLACEMENT_TOLERANCE_M {
        return Err(format!(
            "{label}: expected {expected:.17e}, got {actual:.17e}"
        ));
    }
    Ok(())
}

fn json_escape(value: &str) -> String {
    let mut output = String::with_capacity(value.len() + 2);
    for ch in value.chars() {
        match ch {
            '"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            ch if ch.is_control() => output.push('?'),
            ch => output.push(ch),
        }
    }
    output
}

fn get_displacements(result: &PlanarTrussResult) -> Result<[f64; 4], String> {
    let c = result
        .node_results
        .iter()
        .find(|node| node.node_id == "C")
        .ok_or_else(|| "missing node C result".to_owned())?;
    let d = result
        .node_results
        .iter()
        .find(|node| node.node_id == "D")
        .ok_or_else(|| "missing node D result".to_owned())?;
    Ok([
        c.displacement_x_m,
        c.displacement_y_m,
        d.displacement_x_m,
        d.displacement_y_m,
    ])
}

fn append_solver_record(
    records: &mut Vec<String>,
    suite: &str,
    case_id: &str,
    topology: &str,
    input: &PlanarTrussInput,
    expected: [f64; 4],
    displacement_limit: f64,
) -> Result<(), String> {
    let result =
        solve(input).map_err(|error| format!("{suite}/{case_id}/{topology}: {error}"))?;
    let observed = get_displacements(&result)?;
    for (name, actual, expected_value) in [
        ("Cx", observed[0], expected[0]),
        ("Cy", observed[1], expected[1]),
        ("Dx", observed[2], expected[2]),
        ("Dy", observed[3], expected[3]),
    ] {
        close_to(
            actual,
            expected_value,
            &format!("{suite}/{case_id}/{topology}/{name}"),
        )?;
        if !actual.is_finite() || actual.abs() > displacement_limit {
            return Err(format!(
                "{suite}/{case_id}/{topology}/{name} violates displacement limit: {actual}"
            ));
        }
    }

    let eq = &result.equilibrium;
    for (name, value) in [
        ("force", eq.normalized_force_residual),
        ("moment", eq.normalized_moment_residual),
        ("free_dof", eq.normalized_free_dof_residual),
    ] {
        if !finite_below(value, RESIDUAL_LIMIT) {
            return Err(format!(
                "{suite}/{case_id}/{topology}/{name} residual outside limit: {value}"
            ));
        }
    }

    let mut member_results = Vec::new();
    for member in &result.bar_results {
        member_results.push(format!(
            concat!(
                "{{\"id\":\"{}\",\"axial_force_n\":{:.17e},",
                "\"axial_stress_pa\":{:.17e},\"axial_strain\":{:.17e}}}"
            ),
            json_escape(&member.bar_id),
            member.axial_force_n,
            member.axial_stress_pa,
            member.axial_strain
        ));
    }
    records.push(format!(
        concat!(
            "{{\"suite\":\"{}\",\"case_id\":\"{}\",\"topology\":\"{}\",",
            "\"disposition\":\"pass\",",
            "\"displacement_m\":{{\"Cx\":{:.17e},\"Cy\":{:.17e},",
            "\"Dx\":{:.17e},\"Dy\":{:.17e}}},",
            "\"member_results\":[{}],",
            "\"normalized_force_residual\":{:.17e},",
            "\"normalized_moment_residual\":{:.17e},",
            "\"normalized_free_dof_residual\":{:.17e},",
            "\"serviceability_limit_m\":{:.17e}}}"
        ),
        json_escape(suite),
        json_escape(case_id),
        json_escape(topology),
        observed[0],
        observed[1],
        observed[2],
        observed[3],
        member_results.join(","),
        eq.normalized_force_residual,
        eq.normalized_moment_residual,
        eq.normalized_free_dof_residual,
        displacement_limit
    ));
    Ok(())
}

fn append_mechanism_record(
    records: &mut Vec<String>,
    suite: &str,
    case_id: &str,
    input: &PlanarTrussInput,
) -> Result<(), String> {
    let error = match solve(input) {
        Ok(_) => return Err(format!("{suite}/{case_id}/none: mechanism unexpectedly solved")),
        Err(error) => error,
    };
    if !error.contains("singular or numerically unresolved truss stiffness") {
        return Err(format!(
            "{suite}/{case_id}/none: unexpected failure class: {error}"
        ));
    }
    records.push(format!(
        concat!(
            "{{\"suite\":\"{}\",\"case_id\":\"{}\",\"topology\":\"none\",",
            "\"disposition\":\"expected_mechanism_rejected\",",
            "\"member_set_iou_vs_braced\":0.8,\"solver_error\":\"{}\"}}"
        ),
        json_escape(suite),
        json_escape(case_id),
        json_escape(&error)
    ));
    Ok(())
}

fn parse_args() -> Result<(String, String), String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut commit: Option<String> = None;
    let mut tree: Option<String> = None;
    let mut index = 0;
    while index < args.len() {
        let value = args
            .get(index + 1)
            .ok_or_else(|| format!("{} requires a value", args[index]))?;
        match args[index].as_str() {
            "--source-commit" if commit.is_none() => commit = Some(value.clone()),
            "--source-tree-sha" if tree.is_none() => tree = Some(value.clone()),
            "--source-commit" => {
                return Err("--source-commit may be supplied only once".into());
            }
            "--source-tree-sha" => {
                return Err("--source-tree-sha may be supplied only once".into());
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
        index += 2;
    }
    let commit = commit.ok_or_else(|| "missing --source-commit".to_owned())?;
    let tree = tree.ok_or_else(|| "missing --source-tree-sha".to_owned())?;
    for (label, sha) in [("commit", &commit), ("tree", &tree)] {
        if sha.len() != 40 || !sha.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "source {label} must be a 40-character hexadecimal Git SHA"
            ));
        }
    }
    Ok((commit.to_ascii_lowercase(), tree.to_ascii_lowercase()))
}

fn run() -> Result<(), String> {
    let (source_commit, source_tree_sha) = parse_args()?;
    let mut records = Vec::new();
    let mut braced_passes = 0_u32;
    let mut mechanism_rejections = 0_u32;
    let mut load_envelope_cases = 0_u32;
    let mut parameter_grid_cases = 0_u32;

    for loads in load_cases() {
        for (topology, diagonal, expected) in [
            ("A-D", Some(("A", "D")), loads.expected_ad),
            ("B-C", Some(("B", "C")), loads.expected_bc),
        ] {
            let input = truss(diagonal, 1.0, 1.0, 1.0, Some(loads));
            append_solver_record(
                &mut records,
                "load_envelope",
                loads.id,
                topology,
                &input,
                expected,
                LOAD_DISPLACEMENT_LIMIT_M,
            )?;
            braced_passes += 1;
        }
        append_mechanism_record(
            &mut records,
            "load_envelope",
            loads.id,
            &truss(None, 1.0, 1.0, 1.0, Some(loads)),
        )?;
        mechanism_rejections += 1;
        load_envelope_cases += 3;
    }

    for area_scale in VARIATION_FACTORS {
        for modulus_scale in VARIATION_FACTORS {
            for load_scale in VARIATION_FACTORS {
                let case_id = format!(
                    "area_{area_scale:.2}_modulus_{modulus_scale:.2}_load_{load_scale:.2}"
                );
                let displacement = BASE_LOAD_N * load_scale
                    / (YOUNGS_MODULUS_PA * modulus_scale * AREA_M2 * area_scale);
                for (topology, diagonal, sign) in [
                    ("A-D", Some(("A", "D")), 1.0_f64),
                    ("B-C", Some(("B", "C")), -1.0_f64),
                ] {
                    let expected = [
                        sign * displacement,
                        -displacement,
                        sign * displacement,
                        -displacement,
                    ];
                    let input = truss(diagonal, area_scale, modulus_scale, load_scale, None);
                    append_solver_record(
                        &mut records,
                        "parameter_grid",
                        &case_id,
                        topology,
                        &input,
                        expected,
                        GRID_DISPLACEMENT_LIMIT_M,
                    )?;
                    braced_passes += 1;
                }
                append_mechanism_record(
                    &mut records,
                    "parameter_grid",
                    &case_id,
                    &truss(None, area_scale, modulus_scale, load_scale, None),
                )?;
                mechanism_rejections += 1;
                parameter_grid_cases += 3;
            }
        }
    }

    if records.len() != 96
        || load_envelope_cases != 15
        || parameter_grid_cases != 81
        || braced_passes != 64
        || mechanism_rejections != 32
    {
        return Err(format!(
            "unexpected case census: total={}, braced_passes={braced_passes}, mechanism_rejections={mechanism_rejections}",
            records.len()
        ));
    }

    println!(
        concat!(
            "{{\"schema_version\":\"morph-function-receipt-v0.1\",",
            "\"benchmark_id\":\"MORPH-FUNCTION-001\",",
            "\"source_commit\":\"{}\",\"source_tree_sha\":\"{}\",",
            "\"request_sha256\":\"{}\",\"evidence_class\":\"synthetic\",",
            "\"model_class\":\"2d_small_displacement_linear_elastic_axial_truss\",",
            "\"case_count\":{},\"load_envelope_case_count\":{},",
            "\"parameter_grid_case_count\":{},\"braced_pass_count\":{},",
            "\"mechanism_rejection_count\":{},",
            "\"claim_ceiling\":\"synthetic_2d_axial_truss_only_no_physical_validation_no_code_compliance\",",
            "\"cases\":[{}]}}"
        ),
        source_commit,
        source_tree_sha,
        REQUEST_SHA256,
        records.len(),
        load_envelope_cases,
        parameter_grid_cases,
        braced_passes,
        mechanism_rejections,
        records.join(",")
    );
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("morph-function-receipt: {error}");
            ExitCode::FAILURE
        }
    }
}
