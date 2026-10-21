// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic receipt for the frozen symmetric planar-truss oracle.
//!
//! CI hashes the exact UTF-8 bytes in `request_canonical` and adds that SHA-256
//! to the published receipt. Keeping hashing outside this tiny, dependency-free
//! binary avoids claiming a custom in-tree cryptographic implementation.

use physical_transformation_reference::planar_truss::{
    PlanarTrussResult, canonical_request, solve, symmetric_two_bar_reference_fixture,
};
use std::env;
use std::process::ExitCode;

const FIXTURE_ID: &str = "PHYS-TRUSS-V-001";
const FIXED_REQUEST: &str = include_str!("../../fixtures/planar_truss_request_v0_1.txt");

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
    let mut index = 0;
    while index < args.len() {
        if args[index] == "--source-commit" {
            if source_commit.is_some() {
                return Err("--source-commit may be supplied only once".to_owned());
            }
            let value = args
                .get(index + 1)
                .ok_or_else(|| "--source-commit requires a value".to_owned())?;
            source_commit = Some(normalize_source_commit(value)?);
            index += 2;
        } else {
            return Err(format!("unknown argument: {}", args[index]));
        }
    }

    let source_commit = source_commit.ok_or_else(|| {
        "usage: planar-truss-reference --source-commit <40-char-git-sha>".to_owned()
    })?;
    let input = symmetric_two_bar_reference_fixture();
    let request_canonical = canonical_request(FIXTURE_ID, &input)?;
    if request_canonical != FIXED_REQUEST.trim_end_matches('\n') {
        return Err("canonical request differs from the frozen fixture bytes".to_owned());
    }

    let result = solve(&input)?;
    verify_reference_outcome(&result)?;

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
            "\"benchmark_id\":\"PHYS-TRUSS-001\",",
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
            "\"verification\":{{",
            "\"independent_known_answer\":\"pass\",",
            "\"equilibrium_gate\":\"pass\",",
            "\"physical_validation\":\"not_performed\"",
            "}},",
            "\"claim_ceiling\":{}",
            "}}"
        ),
        quote(FIXTURE_ID),
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
        quote(result.claim_ceiling)
    ))
}

fn normalize_source_commit(value: &str) -> Result<String, String> {
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn verify_reference_outcome(result: &PlanarTrussResult) -> Result<(), String> {
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
        assert!(verify_reference_outcome(&result).is_err());
    }
}
