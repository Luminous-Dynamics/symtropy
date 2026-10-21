// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic receipt for the synthetic fixed-free axial-bar modal fixture.

use physical_transformation_reference::modal_bar::{
    AxialBarMode, MassMatrixFormulation, analytical_fixed_free_axial_bar_frequencies,
    fixed_free_axial_bar_modes,
};
use std::env;
use std::process::ExitCode;

const FIXTURE_ID: &str = "PHYS-AXIAL-BAR-001";
const FIXED_REQUEST: &str = include_str!("../../fixtures/modal_bar_request_v0_1.txt");
const LENGTH_M: f64 = 1.0;
const AREA_M2: f64 = 0.01;
const YOUNGS_MODULUS_PA: f64 = 210_000_000_000.0;
const DENSITY_KG_M3: f64 = 7800.0;
const ELEMENT_COUNT: usize = 8;
const REQUESTED_MODES: usize = 2;
const MASS_FORMULATION: MassMatrixFormulation = MassMatrixFormulation::Consistent;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("modal-bar-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let canonical = canonical_request();
    if canonical != FIXED_REQUEST.trim_end_matches('\n') {
        return Err("canonical modal-bar request differs from frozen fixture bytes".to_owned());
    }

    let modes = fixed_free_axial_bar_modes(
        LENGTH_M,
        AREA_M2,
        YOUNGS_MODULUS_PA,
        DENSITY_KG_M3,
        ELEMENT_COUNT,
        REQUESTED_MODES,
        MASS_FORMULATION,
    )?;
    let analytical = analytical_fixed_free_axial_bar_frequencies(
        LENGTH_M,
        YOUNGS_MODULUS_PA,
        DENSITY_KG_M3,
        REQUESTED_MODES,
    )?;
    let mass = free_dof_mass_matrix(
        LENGTH_M,
        AREA_M2,
        DENSITY_KG_M3,
        ELEMENT_COUNT,
        MASS_FORMULATION,
    );
    verify_modes(&modes, &analytical, &mass)?;

    let modes_json = modes
        .iter()
        .enumerate()
        .map(|(index, mode)| {
            let shape_json = mode
                .shape_free_dofs
                .iter()
                .copied()
                .map(number)
                .collect::<Vec<_>>()
                .join(",");
            format!(
                concat!(
                    "{{\"mode_index\":{},",
                    "\"eigenvalue_rad_s_squared\":{},",
                    "\"frequency_hz\":{},",
                    "\"shape_free_dofs\":[{}],",
                    "\"normalized_eigen_residual\":{},",
                    "\"mass_norm_residual\":{}}}"
                ),
                index + 1,
                number(mode.eigenvalue_rad_s_squared),
                number(mode.frequency_hz),
                shape_json,
                number(mode.normalized_eigen_residual),
                number(mode.mass_norm_residual),
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    let analytical_json = analytical.iter().copied().map(number).collect::<Vec<_>>().join(",");
    let max_relative_error = modes
        .iter()
        .zip(&analytical)
        .map(|(mode, target)| (mode.frequency_hz - target).abs() / target)
        .fold(0.0_f64, f64::max);
    let orthogonality_residual = max_modal_orthogonality(&modes, &mass)?;

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"axial-bar-modal-receipt-v0.1\",",
            "\"benchmark_id\":{},",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"synthetic\",",
            "\"result\":\"pass\",",
            "\"model\":{{",
            "\"length_m\":{},",
            "\"area_m2\":{},",
            "\"youngs_modulus_pa\":{},",
            "\"density_kg_m3\":{},",
            "\"element_count\":{},",
            "\"requested_modes\":{},",
            "\"mass_formulation\":\"consistent\",",
            "\"boundary_condition\":\"fixed_at_x0_free_at_xL_longitudinal_only\"",
            "}},",
            "\"modes\":[{}],",
            "\"checks\":{{",
            "\"analytical_frequencies_hz\":[{}],",
            "\"max_relative_frequency_error\":{},",
            "\"max_normalized_eigen_residual\":{},",
            "\"max_mass_norm_residual\":{},",
            "\"max_off_diagonal_mass_orthogonality_residual\":{},",
            "\"frequency_error_gate\":\"pass_at_5_percent\",",
            "\"mode_residual_gate\":\"pass_below_1e-10\",",
            "\"mass_orthogonality_gate\":\"pass_below_1e-10\"",
            "}},",
            "\"claim_ceiling\":\"synthetic_1d_longitudinal_axial_bar_modal_verification_only_not_modal_truss_nbc05_physical_validation_or_structural_safety\"",
            "}}"
        ),
        quote(FIXTURE_ID),
        quote(&source_commit),
        quote(&canonical),
        number(LENGTH_M),
        number(AREA_M2),
        number(YOUNGS_MODULUS_PA),
        number(DENSITY_KG_M3),
        ELEMENT_COUNT,
        REQUESTED_MODES,
        modes_json,
        analytical_json,
        number(max_relative_error),
        number(modes.iter().map(|mode| mode.normalized_eigen_residual).fold(0.0_f64, f64::max)),
        number(modes.iter().map(|mode| mode.mass_norm_residual).fold(0.0_f64, f64::max)),
        number(orthogonality_residual),
    ))
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err("usage: modal-bar-reference --source-commit <40-char-git-sha>".to_owned());
    }
    normalize_source_commit(&args[1])
}

fn normalize_source_commit(value: &str) -> Result<String, String> {
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn canonical_request() -> String {
    format!(
        "schema=modal-bar-request-v0.1|fixture={FIXTURE_ID}|length_m=1|area_m2=0.01|youngs_modulus_pa=210000000000|density_kg_m3=7800|element_count={ELEMENT_COUNT}|requested_modes={REQUESTED_MODES}|mass_formulation=consistent"
    )
}

fn free_dof_mass_matrix(
    length_m: f64,
    area_m2: f64,
    density_kg_m3: f64,
    element_count: usize,
    formulation: MassMatrixFormulation,
) -> Vec<Vec<f64>> {
    let element_length = length_m / element_count as f64;
    let element_mass = density_kg_m3 * area_m2 * element_length;
    let mut mass = vec![vec![0.0; element_count]; element_count];
    for element in 0..element_count {
        let nodes = [element, element + 1];
        if formulation == MassMatrixFormulation::Consistent {
            for local_row in 0..2 {
                let global_row = nodes[local_row];
                if global_row == 0 {
                    continue;
                }
                for local_col in 0..2 {
                    let global_col = nodes[local_col];
                    if global_col == 0 {
                        continue;
                    }
                    let factor = if local_row == local_col { 2.0 } else { 1.0 };
                    mass[global_row - 1][global_col - 1] += element_mass * factor / 6.0;
                }
            }
        } else {
            for node in nodes {
                if node != 0 {
                    mass[node - 1][node - 1] += element_mass / 2.0;
                }
            }
        }
    }
    mass
}

fn verify_modes(
    modes: &[AxialBarMode],
    analytical: &[f64],
    mass: &[Vec<f64>],
) -> Result<(), String> {
    if modes.len() != REQUESTED_MODES || analytical.len() != REQUESTED_MODES {
        return Err("modal result count differs from the frozen request".to_owned());
    }
    for (index, (mode, target)) in modes.iter().zip(analytical).enumerate() {
        if mode.shape_free_dofs.len() != ELEMENT_COUNT {
            return Err(format!("mode {} has an unexpected shape length", index + 1));
        }
        if !mode.frequency_hz.is_finite()
            || mode.frequency_hz <= 0.0
            || (mode.frequency_hz - target).abs() / target > 0.05
        {
            return Err(format!("mode {} exceeds the analytical frequency-error gate", index + 1));
        }
        if !mode.normalized_eigen_residual.is_finite()
            || mode.normalized_eigen_residual >= 1.0e-10
        {
            return Err(format!("mode {} exceeds the eigen-residual gate", index + 1));
        }
        if !mode.mass_norm_residual.is_finite() || mode.mass_norm_residual >= 1.0e-10 {
            return Err(format!("mode {} exceeds the unit-modal-mass gate", index + 1));
        }
        if mode.shape_free_dofs.iter().any(|value| !value.is_finite()) {
            return Err(format!("mode {} contains a non-finite shape component", index + 1));
        }
    }
    if !modes.windows(2).all(|pair| pair[0].frequency_hz < pair[1].frequency_hz) {
        return Err("modal frequencies are not strictly ascending".to_owned());
    }
    let orthogonality = max_modal_orthogonality(modes, mass)?;
    if !orthogonality.is_finite() || orthogonality >= 1.0e-10 {
        return Err("modal mass orthogonality gate failed".to_owned());
    }
    Ok(())
}

fn max_modal_orthogonality(modes: &[AxialBarMode], mass: &[Vec<f64>]) -> Result<f64, String> {
    let mut maximum = 0.0_f64;
    for first in 0..modes.len() {
        for second in (first + 1)..modes.len() {
            let shape_a = &modes[first].shape_free_dofs;
            let shape_b = &modes[second].shape_free_dofs;
            if shape_a.len() != mass.len() || shape_b.len() != mass.len() {
                return Err("modal shape does not match the mass matrix".to_owned());
            }
            let mut inner = 0.0_f64;
            for row in 0..mass.len() {
                for col in 0..mass.len() {
                    inner += shape_a[row] * mass[row][col] * shape_b[col];
                }
            }
            if !inner.is_finite() {
                return Err("modal orthogonality calculation is non-finite".to_owned());
            }
            maximum = maximum.max(inner.abs());
        }
    }
    Ok(maximum)
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

#[cfg(test)]
mod tests {
    use super::{FIXTURE_ID, canonical_request, normalize_source_commit, quote};

    #[test]
    fn source_commit_must_be_well_formed_and_is_normalized() {
        assert!(normalize_source_commit("not-a-sha").is_err());
        assert_eq!(
            normalize_source_commit("ABCDEF0123456789ABCDEF0123456789ABCDEF01").expect("valid SHA"),
            "abcdef0123456789abcdef0123456789abcdef01"
        );
    }

    #[test]
    fn canonical_request_matches_the_named_fixture_schema() {
        assert!(canonical_request().starts_with("schema=modal-bar-request-v0.1|"));
        assert!(canonical_request().contains(FIXTURE_ID));
    }

    #[test]
    fn json_quoting_is_stable_for_delimiters_and_control_characters() {
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
    }
}
