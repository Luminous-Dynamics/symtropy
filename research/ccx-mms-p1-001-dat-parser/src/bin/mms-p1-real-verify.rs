//! Verify a genuine CalculiX DAT file for the frozen PHYS-CCX-MMS-P1-001 case.
//!
//! This program validates deterministic deck identity, parses full nodal U/RF
//! blocks, computes continuum error norms from solver-produced U, and audits
//! corrected support reactions against independently assembled nodal loads.
//! Process execution/build attestation belongs to the calling workflow receipt;
//! this report intentionally never sets qualification_pass=true.

use std::collections::BTreeMap;
use std::env;
use std::fs;
use std::process::ExitCode;

use ccx_mms_p1_dat_parser::{
    audit_global_force_balance, parse_dat, NodalField, Vec3,
};
use ccx_mms_p1_mesh::{generate_mesh, Mesh};
use ccx_mms_p1_oracle::{
    ManufacturedField, ManufacturedProblem, PlaneStrainMaterial, Vec2,
};

const DECK_QUADRATURE_ORDER: usize = 48;
const ERROR_QUADRATURE_ORDER: usize = 24;
const OUTPUT_TIME: f64 = 1.0;
const OUTPUT_TIME_TOLERANCE: f64 = 1.0e-5;
const FORCE_BALANCE_TOLERANCE: f64 = 1.0e-8;
const LOAD_RESULTANT_TOLERANCE: f64 = 1.0e-10;
const BOUNDARY_DISPLACEMENT_TOLERANCE: f64 = 1.0e-10;
const REPEAT_FIELD_TOLERANCE: f64 = 1.0e-12;

fn is_lowercase_sha40(value: &str) -> bool {
    value.len() == 40
        && value
            .bytes()
            .all(|byte| matches!(byte, b'0'..=b'9' | b'a'..=b'f'))
}

fn expected_node_ids(mesh: &ccx_mms_p1_mesh::Mesh) -> Vec<usize> {
    mesh.nodes.iter().map(|node| node.id).collect()
}

fn validate_deck_identity(actual: &str, expected: &str) -> Result<(), String> {
    if actual.as_bytes() == expected.as_bytes() {
        Ok(())
    } else {
        Err("input deck differs byte-for-byte from the deterministic frozen MMS generator".to_owned())
    }
}

fn parse_field(
    dat_text: &str,
    node_ids: &[usize],
) -> Result<ccx_mms_p1_dat_parser::ParsedDat, String> {
    parse_dat(
        dat_text,
        "NALL",
        node_ids,
        OUTPUT_TIME,
        OUTPUT_TIME_TOLERANCE,
    )
    .map_err(|error| format!("strict CalculiX DAT parsing failed: {error}"))
}

fn field_relative_difference(
    actual: &NodalField,
    reference: &NodalField,
    node_ids: &[usize],
) -> Result<f64, String> {
    if actual.output_time.to_bits() != reference.output_time.to_bits() {
        return Err("repeat DAT output times differ".to_owned());
    }
    let mut diff_norm = 0.0_f64;
    let mut ref_norm = 0.0_f64;
    for node_id in node_ids {
        let a = actual
            .get(*node_id)
            .ok_or_else(|| format!("repeat DAT is missing node {node_id}"))?;
        let b = reference
            .get(*node_id)
            .ok_or_else(|| format!("reference DAT is missing node {node_id}"))?;
        diff_norm = diff_norm.hypot(a.x - b.x);
        diff_norm = diff_norm.hypot(a.y - b.y);
        diff_norm = diff_norm.hypot(a.z - b.z);
        ref_norm = ref_norm.hypot(b.x);
        ref_norm = ref_norm.hypot(b.y);
        ref_norm = ref_norm.hypot(b.z);
    }
    if !diff_norm.is_finite() || !ref_norm.is_finite() || ref_norm <= 0.0 {
        return Err("invalid norm while comparing repeat displacement fields".to_owned());
    }
    Ok(diff_norm / ref_norm)
}

fn vec3_from_vec2(value: Vec2) -> Vec3 {
    Vec3 {
        x: value.x,
        y: value.y,
        z: 0.0,
    }
}

fn run() -> Result<(), String> {
    let mut args = env::args().skip(1);
    let resolution = args
        .next()
        .ok_or_else(|| "missing cells-per-side".to_owned())?
        .parse::<usize>()
        .map_err(|error| format!("invalid cells-per-side: {error}"))?;
    let deck_path = args.next().ok_or_else(|| "missing input-deck path".to_owned())?;
    let dat_path = args.next().ok_or_else(|| "missing DAT path".to_owned())?;
    let mut poisson_ratio = 0.3_f64;
    let mut field = "smooth-sine";
    let mut reference_dat_path: Option<String> = None;
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--poisson-ratio" => {
                let raw = args
                    .next()
                    .ok_or_else(|| "--poisson-ratio requires a value".to_owned())?;
                poisson_ratio = raw
                    .parse::<f64>()
                    .map_err(|error| format!("invalid Poisson ratio: {error}"))?;
            }
            "--field" => {
                field = match args
                    .next()
                    .ok_or_else(|| "--field requires a value".to_owned())?
                    .as_str()
                {
                    "smooth-sine" => "smooth-sine",
                    "divergence-free" => "divergence-free",
                    other => return Err(format!("unsupported manufactured field: {other}")),
                };
            }
            "--reference-dat" => {
                reference_dat_path = Some(
                    args.next()
                        .ok_or_else(|| "--reference-dat requires a path".to_owned())?,
                );
            }
            other => return Err(format!("unknown argument: {other}")),
        }
    }

    let source_sha = env::var("MMS_BENCH_SOURCE_SHA").unwrap_or_default();
    let checkout_sha = env::var("MMS_BENCH_CHECKOUT_SHA").unwrap_or_default();
    if !is_lowercase_sha40(&source_sha) || !is_lowercase_sha40(&checkout_sha) {
        return Err(
            "MMS_BENCH_SOURCE_SHA and MMS_BENCH_CHECKOUT_SHA must be lowercase 40-character Git SHAs"
                .to_owned(),
        );
    }

    let mesh = generate_mesh(resolution)
        .map_err(|error| format!("mesh construction failed: {error:?}"))?;
    let material = PlaneStrainMaterial::new(1.0, poisson_ratio)
        .map_err(|error| format!("material parameters are invalid: {error:?}"))?;
    let problem = match field {
        "smooth-sine" => ManufacturedProblem::new(material, 1.0, 0.7)
            .map_err(|error| format!("manufactured problem is invalid: {error:?}"))?,
        "divergence-free" => ManufacturedProblem::divergence_free(material),
        _ => unreachable!("field was validated while parsing"),
    };
    let field_name = match problem.field() {
        ManufacturedField::SmoothSine => "smooth-sine",
        ManufacturedField::DivergenceFree => "divergence-free",
    };
    let deck_text = fs::read_to_string(&deck_path)
        .map_err(|error| format!("cannot read input deck {deck_path}: {error}"))?;
    let expected_deck = mesh
        .to_calculix_input_deck(problem, DECK_QUADRATURE_ORDER)
        .map_err(|error| format!("deterministic reference deck generation failed: {error:?}"))?;
    validate_deck_identity(&deck_text, &expected_deck)?;

    let dat_text = fs::read_to_string(&dat_path)
        .map_err(|error| format!("cannot read DAT file {dat_path}: {error}"))?;
    let ids = expected_node_ids(&mesh);
    let parsed = parse_field(&dat_text, &ids)?;

    // Compute error norms from the actual solver-produced displacement field.
    let mut nodal_displacements = Vec::with_capacity(mesh.nodes.len());
    let mut max_boundary_displacement = 0.0_f64;
    for node in &mesh.nodes {
        let value = parsed
            .displacements
            .get(node.id)
            .ok_or_else(|| format!("solver displacement field is missing node {}", node.id))?;
        max_boundary_displacement = max_boundary_displacement
            .max(value.x.abs())
            .max(value.y.abs())
            .max(value.z.abs());
        if value.z != 0.0 {
            return Err(format!(
                "plane-strain displacement has nonzero out-of-plane component at node {}",
                node.id
            ));
        }
        nodal_displacements.push(Vec2 { x: value.x, y: value.y });
    }
    let errors = mesh
        .compute_error_norms(problem, &nodal_displacements, ERROR_QUADRATURE_ORDER)
        .map_err(|error| format!("continuum error integration failed: {error:?}"))?;

    // Independently rebuild every nodal load from the analytical body force.
    // Compare the assembled global resultant to the analytical integral and use
    // the complete vector in a separate corrected reaction balance audit.
    let load_assembly = mesh
        .assemble_body_force_with_diagnostics(problem, DECK_QUADRATURE_ORDER)
        .map_err(|error| format!("independent nodal load assembly failed: {error:?}"))?;
    let resultant = Mesh::resultant(&load_assembly.nodal_loads)
        .map_err(|error| format!("load resultant failed: {error:?}"))?;
    let exact_resultant = problem.integrated_body_force();
    let load_resultant_relative_x =
        (resultant.x - exact_resultant.x).abs() / exact_resultant.x.abs().max(1.0);
    let load_resultant_relative_y =
        resultant.y.abs() / load_assembly.integrated_absolute_force.y.max(1.0);

    let boundary_ids: Vec<usize> = mesh
        .nodes
        .iter()
        .filter(|node| node.boundary)
        .map(|node| node.id)
        .collect();
    let complete_loads: BTreeMap<usize, Vec3> = load_assembly
        .nodal_loads
        .iter()
        .enumerate()
        .map(|(index, load)| (index + 1, vec3_from_vec2(*load)))
        .collect();
    let omitted_boundary_loads: BTreeMap<usize, Vec3> = mesh
        .nodes
        .iter()
        .filter(|node| node.boundary)
        .map(|node| (node.id, vec3_from_vec2(load_assembly.nodal_loads[node.id - 1])))
        .collect();

    let force_audit = audit_global_force_balance(
        &parsed,
        &boundary_ids,
        &omitted_boundary_loads,
        &complete_loads,
        FORCE_BALANCE_TOLERANCE,
    )
    .map_err(|error| format!("global force-balance audit failed to execute: {error}"))?;

    let (repeat_relative_difference, repeat_pass) = if let Some(path) = reference_dat_path {
        let reference_text = fs::read_to_string(&path)
            .map_err(|error| format!("cannot read reference DAT file {path}: {error}"))?;
        let reference = parse_field(&reference_text, &ids)?;
        let difference = field_relative_difference(
            &parsed.displacements,
            &reference.displacements,
            &ids,
        )?;
        (Some(difference), Some(difference <= REPEAT_FIELD_TOLERANCE))
    } else {
        (None, None)
    };

    let load_resultant_pass = load_resultant_relative_x <= LOAD_RESULTANT_TOLERANCE
        && load_resultant_relative_y <= LOAD_RESULTANT_TOLERANCE;
    let boundary_displacement_pass =
        max_boundary_displacement <= BOUNDARY_DISPLACEMENT_TOLERANCE;
    let repeat_gate_pass = repeat_pass.unwrap_or(true);
    let numerical_diagnostics_pass =
        load_resultant_pass && force_audit.passed && boundary_displacement_pass && repeat_gate_pass;

    let repeat_difference_json = repeat_relative_difference
        .map(|value| format!("{value:.17e}"))
        .unwrap_or_else(|| "null".to_owned());
    let repeat_pass_json = repeat_pass
        .map(|value| value.to_string())
        .unwrap_or_else(|| "null".to_owned());
    println!(
        concat!(
            "{{\"schema\":\"ccx-mms-p1-real-run-verification-v1\",",
            "\"source_sha\":\"{}\",\"checkout_sha\":\"{}\",",
            "\"solver_output_parsed\":true,\"qualification_pass\":false,",
            "\"resolution\":{},\"node_count\":{},\"element_count\":{},",
            "\"poisson_ratio\":{:.17e},\"field\":\"{}\",",
            "\"output_time\":{:.17e},",
            "\"displacement_l2_relative\":{:.17e},",
            "\"energy_error_relative\":{:.17e},",
            "\"stress_l2_relative\":{:.17e},",
            "\"fem_volumetric_strain_l2_absolute\":{:.17e},",
            "\"volumetric_strain_error_l2_absolute\":{:.17e},",
            "\"exact_volumetric_strain_l2_norm\":{:.17e},",
            "\"load_resultant_relative_x\":{:.17e},",
            "\"load_resultant_relative_y\":{:.17e},",
            "\"load_resultant_pass\":{},",
            "\"force_balance_residual_linf\":{:.17e},",
            "\"force_balance_tolerance\":{:.17e},",
            "\"force_balance_pass\":{},",
            "\"max_boundary_displacement\":{:.17e},",
            "\"boundary_displacement_tolerance\":{:.17e},",
            "\"boundary_displacement_pass\":{},",
            "\"repeat_field_relative_difference\":{},",
            "\"repeat_field_pass\":{},",
            "\"numerical_diagnostics_pass\":{}",
            "}}"
        ),
        source_sha,
        checkout_sha,
        resolution,
        mesh.nodes.len(),
        mesh.triangles.len(),
        poisson_ratio,
        field_name,
        parsed.displacements.output_time,
        errors.displacement_l2_relative,
        errors.energy_error_relative,
        errors.stress_l2_relative,
        errors.fem_volumetric_strain_l2_absolute,
        errors.volumetric_strain_error_l2_absolute,
        errors.exact_volumetric_strain_l2_norm,
        load_resultant_relative_x,
        load_resultant_relative_y,
        load_resultant_pass,
        force_audit.normalized_residual_linf,
        FORCE_BALANCE_TOLERANCE,
        force_audit.passed,
        max_boundary_displacement,
        BOUNDARY_DISPLACEMENT_TOLERANCE,
        boundary_displacement_pass,
        repeat_difference_json,
        repeat_pass_json,
        numerical_diagnostics_pass,
    );

    // A numerically failed but parseable run still emits a receipt; workflow
    // policy gates the receipt after all cases execute so failure evidence is kept.
    Ok(())
}

fn main() -> ExitCode {
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(message) => {
            eprintln!("{message}");
            ExitCode::FAILURE
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_generated_deck_is_accepted_and_tampered_deck_fails_closed() {
        let mesh = generate_mesh(4).unwrap();
        let expected = mesh
            .to_calculix_input_deck(ManufacturedProblem::reference(), DECK_QUADRATURE_ORDER)
            .unwrap();
        assert!(validate_deck_identity(&expected, &expected).is_ok());

        let tampered = expected.replacen("*CLOAD", "*DLOAD", 1);
        assert_ne!(tampered.as_bytes(), expected.as_bytes());
        assert!(validate_deck_identity(&tampered, &expected).is_err());
    }

    #[test]
    fn node_id_helpers_remain_deterministic() {
        let mesh = generate_mesh(2).unwrap();
        assert_eq!(expected_node_ids(&mesh), (1..=9).collect::<Vec<_>>());
    }

    #[test]
    fn reference_material_and_mms_fields_are_finite() {
        let problem = ManufacturedProblem::reference();
        let value = problem.displacement(0.25, 0.75);
        assert!(value.x.is_finite() && value.y.is_finite());
    }

    #[test]
    fn deterministic_deck_generation_is_byte_repeatable() {
        let mesh = generate_mesh(4).unwrap();
        let problem = ManufacturedProblem::reference();
        let a = mesh.to_calculix_input_deck(problem, DECK_QUADRATURE_ORDER).unwrap();
        let b = mesh.to_calculix_input_deck(problem, DECK_QUADRATURE_ORDER).unwrap();
        assert_eq!(a.as_bytes(), b.as_bytes());
    }
}
