// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic receipt for the frozen symmetric planar-truss oracle.
//!
//! CI hashes the exact UTF-8 bytes in `request_canonical` and adds that SHA-256
//! to the published receipt. Keeping hashing outside this tiny, dependency-free
//! binary avoids claiming a custom in-tree cryptographic implementation.

use physical_transformation_reference::planar_truss::{
    PlanarTrussResult, asymmetric_two_bar_reference_fixture, canonical_request, solve,
    symmetric_two_bar_reference_fixture,
};
use std::env;
use std::process::ExitCode;

const SYMMETRIC_FIXTURE_ID: &str = "PHYS-TRUSS-V-001";
const ASYMMETRIC_FIXTURE_ID: &str = "PHYS-TRUSS-ASYM-001";
const SYMMETRIC_REQUEST: &str = include_str!("../../fixtures/planar_truss_request_v0_1.txt");
const ASYMMETRIC_REQUEST: &str =
    include_str!("../../fixtures/planar_truss_asymmetric_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("planar-truss-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    let mut source_commit: Option<String> = None;
    let mut fixture_kind = "symmetric";
    let mut fixture_was_supplied = false;
    let mut index = 0;
    while index < args.len() {
        match args[index].as_str() {
            "--source-commit" => {
                if source_commit.is_some() {
                    return Err("--source-commit may be supplied only once".to_owned());
                }
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--source-commit requires a value".to_owned())?;
                source_commit = Some(normalize_source_commit(value)?);
                index += 2;
            }
            "--fixture" => {
                if fixture_was_supplied {
                    return Err("--fixture may be supplied only once".to_owned());
                }
                let value = args
                    .get(index + 1)
                    .ok_or_else(|| "--fixture requires symmetric or asymmetric".to_owned())?;
                if !matches!(value.as_str(), "symmetric" | "asymmetric") {
                    return Err("--fixture must be symmetric or asymmetric".to_owned());
                }
                fixture_kind = if value == "asymmetric" {
                    "asymmetric"
                } else {
                    "symmetric"
                };
                fixture_was_supplied = true;
                index += 2;
            }
            unknown => return Err(format!("unknown argument: {unknown}")),
        }
    }

    let source_commit = source_commit.ok_or_else(|| {
        "usage: planar-truss-reference --source-commit <40-char-git-sha> [--fixture symmetric|asymmetric]".to_owned()
    })?;
    let (fixture_id, benchmark_id, input, fixed_request) = if fixture_kind == "asymmetric" {
        (
            ASYMMETRIC_FIXTURE_ID,
            "PHYS-TRUSS-ASYM-001",
            asymmetric_two_bar_reference_fixture(),
            ASYMMETRIC_REQUEST,
        )
    } else {
        (
            SYMMETRIC_FIXTURE_ID,
            "PHYS-TRUSS-001",
            symmetric_two_bar_reference_fixture(),
            SYMMETRIC_REQUEST,
        )
    };
    let request_canonical = canonical_request(fixture_id, &input)?;
    if request_canonical != fixed_request.trim_end_matches('\n') {
        return Err("canonical request differs from the frozen fixture bytes".to_owned());
    }

    let result = solve(&input)?;
    verify_reference_outcome(&result, fixture_kind)?;

    let nodes_json = result
        .node_results
        .iter()
        .map(|node| {
            format!(
                concat!(
                    "{{\"node_id\":{},",
                    "\"displacement_x_m\":{},",
                    "\"displacement_y_m\":{},",
                    "\"reaction_x_n\":{},",
                    "\"reaction_y_n\":{}}}"
                ),
                quote(&node.node_id),
                number(node.displacement_x_m),
                number(node.displacement_y_m),
                optional_number(node.reaction_x_n),
                optional_number(node.reaction_y_n)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let bars_json = result
        .bar_results
        .iter()
        .map(|bar| {
            format!(
                concat!(
                    "{{\"bar_id\":{},",
                    "\"length_m\":{},",
                    "\"axial_extension_m\":{},",
                    "\"axial_strain\":{},",
                    "\"axial_force_n\":{},",
                    "\"axial_stress_pa\":{}}}"
                ),
                quote(&bar.bar_id),
                number(bar.length_m),
                number(bar.axial_extension_m),
                number(bar.axial_strain),
                number(bar.axial_force_n),
                number(bar.axial_stress_pa)
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let e = &result.equilibrium;
    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"planar-truss-receipt-v0.1\",",
            "\"benchmark_id\":{},",
            "\"input_fixture_id\":{},",
            "\"source_commit\":{},",
            "\"evidence_class\":\"synthetic\",",
            "\"result\":\"pass\",",
            "\"model_class\":\"static_small_displacement_linear_elastic_2d_pin_jointed_bars\",",
            "\"request_canonical\":{},",
            "\"node_results\":[{}],",
            "\"bar_results\":[{}],",
            "\"equilibrium\":{{",
            "\"applied_force_x_n\":{},",
            "\"applied_force_y_n\":{},",
            "\"support_reaction_x_n\":{},",
            "\"support_reaction_y_n\":{},",
            "\"net_force_residual_x_n\":{},",
            "\"net_force_residual_y_n\":{},",
            "\"net_moment_residual_nm\":{},",
            "\"normalized_force_residual\":{},",
            "\"normalized_moment_residual\":{},",
            "\"max_free_dof_residual_n\":{},",
            "\"normalized_free_dof_residual\":{}",
            "}},",
            "\"energy\":{{",
            "\"strain_energy_j\":{},",
            "\"external_work_j\":{},",
            "\"residual_j\":{},",
            "\"normalized_residual\":{}",
            "}},",
            "\"verification\":{{",
            "\"independent_known_answer\":\"pass\",",
            "\"equilibrium_gate\":\"pass\",",
            "\"energy_balance\":\"pass\",",
            "\"physical_validation\":\"not_performed\"",
            "}},",
            "\"claim_ceiling\":{}",
            "}}"
        ),
        quote(benchmark_id),
        quote(fixture_id),
        quote(&source_commit),
        quote(&request_canonical),
        nodes_json,
        bars_json,
        number(e.applied_force_x_n),
        number(e.applied_force_y_n),
        number(e.support_reaction_x_n),
        number(e.support_reaction_y_n),
        number(e.net_force_residual_x_n),
        number(e.net_force_residual_y_n),
        number(e.net_moment_residual_nm),
        number(e.normalized_force_residual),
        number(e.normalized_moment_residual),
        number(e.max_free_dof_residual_n),
        number(e.normalized_free_dof_residual),
        number(result.energy.strain_energy_j),
        number(result.energy.external_work_j),
        number(result.energy.residual_j),
        number(result.energy.normalized_residual),
        quote(result.claim_ceiling)
    ))
}

fn normalize_source_commit(value: &str) -> Result<String, String> {
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn verify_asymmetric_reference_outcome(result: &PlanarTrussResult) -> Result<(), String> {
    let node = |id: &str| {
        result
            .node_results
            .iter()
            .find(|entry| entry.node_id == id)
            .ok_or_else(|| format!("asymmetric known-answer result is missing node {id}"))
    };
    let bar = |id: &str| {
        result
            .bar_results
            .iter()
            .find(|entry| entry.bar_id == id)
            .ok_or_else(|| format!("asymmetric known-answer result is missing bar {id}"))
    };

    let apex = node("C")?;
    let support_a = node("A")?;
    let support_b = node("B")?;
    let ac = bar("AC")?;
    let bc = bar("BC")?;
    close(
        apex.displacement_x_m,
        1.875e-7,
        1.0e-15,
        "asymmetric apex x",
    )?;
    close(
        apex.displacement_y_m,
        -5.625e-7,
        1.0e-15,
        "asymmetric apex y",
    )?;
    close(
        support_a
            .reaction_x_n
            .ok_or_else(|| "A x reaction missing".to_owned())?,
        150.0,
        1.0e-6,
        "asymmetric A x reaction",
    )?;
    close(
        support_a
            .reaction_y_n
            .ok_or_else(|| "A y reaction missing".to_owned())?,
        450.0,
        1.0e-6,
        "asymmetric A y reaction",
    )?;
    close(
        support_b
            .reaction_x_n
            .ok_or_else(|| "B x reaction missing".to_owned())?,
        -750.0,
        1.0e-6,
        "asymmetric B x reaction",
    )?;
    close(
        support_b
            .reaction_y_n
            .ok_or_else(|| "B y reaction missing".to_owned())?,
        750.0,
        1.0e-6,
        "asymmetric B y reaction",
    )?;
    close(
        ac.axial_force_n,
        -1_500.0 / 10.0_f64.sqrt(),
        1.0e-6,
        "asymmetric AC force",
    )?;
    close(
        bc.axial_force_n,
        -1_500.0 / 2.0_f64.sqrt(),
        1.0e-6,
        "asymmetric BC force",
    )?;
    close(
        ac.axial_stress_pa,
        -30_000.0,
        1.0e-6,
        "asymmetric AC stress",
    )?;
    close(
        bc.axial_stress_pa,
        -25_000.0,
        1.0e-6,
        "asymmetric BC stress",
    )?;
    close(ac.axial_strain, -1.5e-7, 1.0e-15, "asymmetric AC strain")?;
    close(bc.axial_strain, -1.25e-7, 1.0e-15, "asymmetric BC strain")?;
    close(
        ac.axial_extension_m,
        -1.5e-7 * 10.0_f64.sqrt(),
        1.0e-15,
        "asymmetric AC extension",
    )?;
    close(
        bc.axial_extension_m,
        -1.25e-7 * 18.0_f64.sqrt(),
        1.0e-15,
        "asymmetric BC extension",
    )?;

    let expected_energy_j = 3.9375e-4;
    close(
        result.energy.strain_energy_j,
        expected_energy_j,
        1.0e-12,
        "asymmetric strain energy",
    )?;
    close(
        result.energy.external_work_j,
        expected_energy_j,
        1.0e-12,
        "asymmetric external work",
    )?;
    close(
        result.energy.residual_j,
        0.0,
        1.0e-12,
        "asymmetric energy residual",
    )?;

    for (label, value) in [
        ("force", result.equilibrium.normalized_force_residual),
        ("moment", result.equilibrium.normalized_moment_residual),
        ("free-DOF", result.equilibrium.normalized_free_dof_residual),
        ("energy", result.energy.normalized_residual),
    ] {
        if !value.is_finite() || value >= 1.0e-12 {
            return Err(format!(
                "asymmetric {label} residual exceeds receipt gate: {value:.17e}"
            ));
        }
    }
    Ok(())
}

fn verify_reference_outcome(result: &PlanarTrussResult, fixture_kind: &str) -> Result<(), String> {
    if fixture_kind == "asymmetric" {
        return verify_asymmetric_reference_outcome(result);
    }
    let apex = result
        .node_results
        .iter()
        .find(|node| node.node_id == "apex")
        .ok_or_else(|| "known-answer result is missing apex node".to_owned())?;
    let left = result
        .node_results
        .iter()
        .find(|node| node.node_id == "left")
        .ok_or_else(|| "known-answer result is missing left support".to_owned())?;
    let right = result
        .node_results
        .iter()
        .find(|node| node.node_id == "right")
        .ok_or_else(|| "known-answer result is missing right support".to_owned())?;
    let left_bar = result
        .bar_results
        .iter()
        .find(|bar| bar.bar_id == "left-diagonal")
        .ok_or_else(|| "known-answer result is missing left bar".to_owned())?;
    let right_bar = result
        .bar_results
        .iter()
        .find(|bar| bar.bar_id == "right-diagonal")
        .ok_or_else(|| "known-answer result is missing right bar".to_owned())?;

    let expected_apex_y = -1_000.0 * 2.0_f64.sqrt() / (0.01 * 200.0e9);
    let expected_bar_force = -1_000.0 / 2.0_f64.sqrt();
    let expected_energy_j = 0.5 * 1_000.0 * expected_apex_y.abs();
    close(apex.displacement_x_m, 0.0, 1.0e-14, "apex x displacement")?;
    close(
        apex.displacement_y_m,
        expected_apex_y,
        1.0e-15,
        "apex y displacement",
    )?;
    close(
        left_bar.axial_force_n,
        expected_bar_force,
        1.0e-9,
        "left bar force",
    )?;
    close(
        right_bar.axial_force_n,
        expected_bar_force,
        1.0e-9,
        "right bar force",
    )?;
    close(
        left_bar.axial_stress_pa,
        expected_bar_force / 0.01,
        1.0e-6,
        "left bar stress",
    )?;
    close(
        right_bar.axial_stress_pa,
        expected_bar_force / 0.01,
        1.0e-6,
        "right bar stress",
    )?;

    close(
        left.reaction_x_n
            .ok_or_else(|| "left x reaction is absent".to_owned())?,
        500.0,
        1.0e-8,
        "left support x reaction",
    )?;
    close(
        right
            .reaction_x_n
            .ok_or_else(|| "right x reaction is absent".to_owned())?,
        -500.0,
        1.0e-8,
        "right support x reaction",
    )?;
    close(
        left.reaction_y_n
            .ok_or_else(|| "left y reaction is absent".to_owned())?,
        500.0,
        1.0e-8,
        "left support y reaction",
    )?;
    close(
        right
            .reaction_y_n
            .ok_or_else(|| "right y reaction is absent".to_owned())?,
        500.0,
        1.0e-8,
        "right support y reaction",
    )?;

    close(
        result.energy.strain_energy_j,
        expected_energy_j,
        expected_energy_j.abs() * 1.0e-10,
        "strain energy",
    )?;
    close(
        result.energy.external_work_j,
        expected_energy_j,
        expected_energy_j.abs() * 1.0e-10,
        "external work",
    )?;
    close(
        result.energy.residual_j,
        0.0,
        expected_energy_j.abs() * 1.0e-10,
        "energy residual",
    )?;
    if !result.energy.normalized_residual.is_finite()
        || result.energy.normalized_residual >= 1.0e-12
    {
        return Err(format!(
            "normalized energy residual exceeds receipt gate: {:.17e}",
            result.energy.normalized_residual
        ));
    }

    for (label, value) in [
        (
            "normalized force residual",
            result.equilibrium.normalized_force_residual,
        ),
        (
            "normalized moment residual",
            result.equilibrium.normalized_moment_residual,
        ),
        (
            "normalized free-DOF residual",
            result.equilibrium.normalized_free_dof_residual,
        ),
    ] {
        if !value.is_finite() || value >= 1.0e-12 {
            return Err(format!("{label} exceeds receipt gate: {value:.17e}"));
        }
    }
    Ok(())
}

fn close(actual: f64, expected: f64, tolerance: f64, label: &str) -> Result<(), String> {
    if !actual.is_finite() || (actual - expected).abs() > tolerance {
        Err(format!(
            "{label} outside receipt tolerance: actual={actual:.17e}, expected={expected:.17e}, abs_tolerance={tolerance:.17e}"
        ))
    } else {
        Ok(())
    }
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_else(|| "null".to_owned())
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
    use super::*;

    #[test]
    fn source_commit_must_be_well_formed_and_is_normalized() {
        assert!(normalize_source_commit("not-a-sha").is_err());
        assert_eq!(
            normalize_source_commit("ABCDEF0123456789ABCDEF0123456789ABCDEF01").expect("valid SHA"),
            "abcdef0123456789abcdef0123456789abcdef01"
        );
    }

    #[test]
    fn json_quoting_is_stable_for_delimiters_and_control_characters() {
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }

    #[test]
    fn receipt_verification_rejects_bad_known_answer_values() {
        let input = symmetric_two_bar_reference_fixture();
        let mut result = solve(&input).expect("valid fixture");
        result.equilibrium.normalized_force_residual = f64::NAN;
        assert!(verify_reference_outcome(&result, "symmetric").is_err());
    }

    #[test]
    fn asymmetric_receipt_verification_pins_known_answer_and_energy() {
        let input = asymmetric_two_bar_reference_fixture();
        let result = solve(&input).expect("valid asymmetric fixture");
        verify_reference_outcome(&result, "asymmetric")
            .expect("asymmetric known-answer and energy gates should pass");

        let mut bad_energy = result;
        bad_energy.energy.strain_energy_j *= 2.0;
        assert!(verify_reference_outcome(&bad_energy, "asymmetric").is_err());
    }
}
