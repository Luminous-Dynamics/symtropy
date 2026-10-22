// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic static cantilever comparison of Timoshenko and Euler-Bernoulli
//! beam-element stiffness models at three declared shear-slenderness values.

use physical_transformation_reference::timoshenko_element::{
    euler_bernoulli_stiffness_for_comparison, timoshenko_frame_element_stiffness,
};
use std::env;
use std::process::ExitCode;

const LENGTH_M: f64 = 1.0;
const AREA_M2: f64 = 1.0e-4;
const YOUNGS_MODULUS_PA: f64 = 200.0e9;
const SHEAR_MODULUS_PA: f64 = 80.0e9;
const SHEAR_CORRECTION_FACTOR: f64 = 1.0;
const TIP_LOAD_N: f64 = 100.0;
const SHEAR_SLENDERNESS: [f64; 3] = [25.0, 50.0, 100.0];
const TARGET_RATIOS: [f64; 3] = [1.12, 1.06, 1.03];
const RELATIVE_TOLERANCE: f64 = 1.0e-12;
const FIXED_REQUEST: &str =
    include_str!("../../fixtures/timoshenko_cantilever_static_request_v0_1.txt");

#[derive(Clone, Debug)]
struct CaseResult {
    shear_slenderness: f64,
    second_moment_m4: f64,
    euler_bernoulli_tip_deflection_m: f64,
    timoshenko_tip_deflection_m: f64,
    measured_deflection_ratio: f64,
    closed_form_deflection_ratio: f64,
    target_deflection_ratio: f64,
    relative_error: f64,
}

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("timoshenko-cantilever-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let request = canonical_request();
    if request != FIXED_REQUEST.trim_end_matches('\n') {
        return Err("canonical Timoshenko cantilever request differs from frozen fixture".to_owned());
    }

    let mut cases = Vec::with_capacity(SHEAR_SLENDERNESS.len());
    for index in 0..SHEAR_SLENDERNESS.len() {
        let slenderness = SHEAR_SLENDERNESS[index];
        let second_moment_m4 = SHEAR_MODULUS_PA * AREA_M2 * LENGTH_M.powi(2)
            / (YOUNGS_MODULUS_PA * slenderness);
        let eb = euler_bernoulli_stiffness_for_comparison(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            second_moment_m4,
        )?;
        let timo = timoshenko_frame_element_stiffness(
            LENGTH_M,
            AREA_M2,
            YOUNGS_MODULUS_PA,
            SHEAR_MODULUS_PA,
            second_moment_m4,
            SHEAR_CORRECTION_FACTOR,
        )?;
        let eb_deflection = cantilever_tip_response(&eb)?.0;
        let timo_deflection = cantilever_tip_response(&timo.stiffness_local)?.0;
        let measured_ratio = timo_deflection / eb_deflection;
        let closed_form_ratio = 1.0 + 3.0
            / (SHEAR_CORRECTION_FACTOR * slenderness);
        let target_ratio = TARGET_RATIOS[index];
        let relative_error = (measured_ratio - target_ratio).abs() / target_ratio.abs();
        if !relative_error.is_finite() || relative_error >= RELATIVE_TOLERANCE {
            return Err(format!(
                "S={slenderness} measured ratio={measured_ratio:.17e} differs from target={target_ratio:.17e} by {relative_error:.17e}"
            ));
        }
        if (measured_ratio - closed_form_ratio).abs() >= RELATIVE_TOLERANCE {
            return Err(format!(
                "S={slenderness} matrix ratio differs from analytical Timoshenko ratio"
            ));
        }
        cases.push(CaseResult {
            shear_slenderness: slenderness,
            second_moment_m4,
            euler_bernoulli_tip_deflection_m: eb_deflection,
            timoshenko_tip_deflection_m: timo_deflection,
            measured_deflection_ratio: measured_ratio,
            closed_form_deflection_ratio: closed_form_ratio,
            target_deflection_ratio: target_ratio,
            relative_error,
        });
    }

    Ok(receipt_json(&source_commit, &request, &cases))
}

fn canonical_request() -> String {
    let shear_values = SHEAR_SLENDERNESS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    let inertias = SHEAR_SLENDERNESS
        .iter()
        .map(|slenderness| {
            (SHEAR_MODULUS_PA * AREA_M2 * LENGTH_M.powi(2)
                / (YOUNGS_MODULUS_PA * slenderness))
                .to_string()
        })
        .collect::<Vec<_>>()
        .join(",");
    let targets = TARGET_RATIOS
        .iter()
        .map(|value| value.to_string())
        .collect::<Vec<_>>()
        .join(",");
    format!(
        concat!(
            "schema=timoshenko-cantilever-static-request-v0.1",
            "|fixture=PHYS-TIMOSHENKO-CANTILEVER-001",
            "|length_m={}",
            "|area_m2={}",
            "|youngs_modulus_pa={}",
            "|shear_modulus_pa={}",
            "|shear_correction_factor={}",
            "|shear_slenderness_indicators={shear_values}",
            "|second_moment_m4={inertias}",
            "|load_n={}",
            "|target_timoshenko_to_euler_bernoulli_deflection_ratios={targets}",
            "|oracle=closed_form_cantilever_bending_plus_shear_deflection"
        ),
        LENGTH_M,
        AREA_M2,
        YOUNGS_MODULUS_PA,
        SHEAR_MODULUS_PA,
        SHEAR_CORRECTION_FACTOR,
        TIP_LOAD_N,
    )
}

fn cantilever_tip_response(stiffness: &[[f64; 6]; 6]) -> Result<(f64, f64), String> {
    // Clamp node 1; invert the free tip [v2, theta2] stiffness block.
    let k11 = stiffness[4][4];
    let k12 = stiffness[4][5];
    let k22 = stiffness[5][5];
    let determinant = k11 * k22 - k12 * k12;
    if !determinant.is_finite() || determinant <= 0.0 {
        return Err("cantilever free-tip stiffness block is not positive definite".to_owned());
    }
    let displacement = k22 * TIP_LOAD_N / determinant;
    let rotation = -k12 * TIP_LOAD_N / determinant;
    if !displacement.is_finite() || displacement <= 0.0 || !rotation.is_finite() {
        return Err("cantilever response is non-finite or non-positive".to_owned());
    }
    Ok((displacement, rotation))
}

fn receipt_json(source_commit: &str, request: &str, cases: &[CaseResult]) -> String {
    let cases_json = cases.iter().map(case_json).collect::<Vec<_>>().join(",");
    format!(
        concat!(
            "{{",
            "\"schema_version\":\"timoshenko-cantilever-static-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-TIMOSHENKO-CANTILEVER-001\",",
            "\"references\":{{\"ansys_beam188\":\"https://ansyshelp.ansys.com/public/Views/Secured/corp/v261/en/ans_elem/Hlp_E_BEAM188.html\",\"abaqus_beam_elements\":\"https://3dexperience.fiir.upb.ro/internal/docv6/english/SIMA3DXELMRefMap/simaelm-c-beamelem.htm\"}},",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"analytical_static_model_comparison\",",
            "\"result\":\"pass\",",
            "\"case_count\":{},",
            "\"relative_tolerance\":{},",
            "\"shear_correction_factor\":{},",
            "\"mass_model\":\"not_applicable_static_stiffness_only\",",
            "\"cases\":[{}],",
            "\"claim_ceiling\":\"local linear static cantilever element comparison only; no Timoshenko modal mass, full NAFEMS reproduction, physical validation, structural safety or code-compliance claim\"",
            "}}"
        ),
        json_quote(source_commit),
        json_quote(request),
        cases.len(),
        number(RELATIVE_TOLERANCE),
        number(SHEAR_CORRECTION_FACTOR),
        cases_json,
    )
}

fn case_json(case: &CaseResult) -> String {
    format!(
        concat!(
            "{{",
            "\"shear_slenderness\":{},",
            "\"second_moment_m4\":{},",
            "\"euler_bernoulli_tip_deflection_m\":{},",
            "\"timoshenko_tip_deflection_m\":{},",
            "\"measured_deflection_ratio\":{},",
            "\"closed_form_deflection_ratio\":{},",
            "\"target_deflection_ratio\":{},",
            "\"relative_error\":{}",
            "}}"
        ),
        number(case.shear_slenderness),
        number(case.second_moment_m4),
        number(case.euler_bernoulli_tip_deflection_m),
        number(case.timoshenko_tip_deflection_m),
        number(case.measured_deflection_ratio),
        number(case.closed_form_deflection_ratio),
        number(case.target_deflection_ratio),
        number(case.relative_error),
    )
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn json_quote(value: &str) -> String {
    let mut output = String::from("\"");
    for character in value.chars() {
        match character {
            '\"' => output.push_str("\\\""),
            '\\' => output.push_str("\\\\"),
            '\n' => output.push_str("\\n"),
            '\r' => output.push_str("\\r"),
            '\t' => output.push_str("\\t"),
            character if character.is_control() => {
                output.push_str(&format!("\\u{:04x}", character as u32));
            }
            character => output.push(character),
        }
    }
    output.push('\"');
    output
}

fn parse_source_commit() -> Result<String, String> {
    let args = env::args().skip(1).collect::<Vec<_>>();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: timoshenko-cantilever-reference --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

#[cfg(test)]
mod tests {
    use super::{canonical_request, parse_source_commit};

    #[test]
    fn canonical_request_matches_frozen_static_fixture() {
        assert_eq!(
            canonical_request(),
            include_str!("../../fixtures/timoshenko_cantilever_static_request_v0_1.txt")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn source_commit_argument_is_strictly_validated() {
        assert!(parse_source_commit().is_err());
    }
}
