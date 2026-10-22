// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic, source-bound receipt for the five named
//! beam-theory applicability-screen cases.

use physical_transformation_reference::beam_theory_applicability::{
    BeamTheoryApplicabilityInput, BeamTheoryApplicabilityReport,
    assess_beam_theory_applicability, canonical_beam_theory_screen_request,
    reference_beam_theory_screen_cases,
};
use std::env;
use std::process::ExitCode;

const FIXED_REQUEST: &str =
    include_str!("../../fixtures/beam_theory_applicability_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-theory-applicability-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let cases = reference_beam_theory_screen_cases();
    let request = canonical_beam_theory_screen_request(&cases)?;
    if request != FIXED_REQUEST.trim_end_matches('\n') {
        return Err("canonical beam-theory screen request differs from frozen fixture".to_owned());
    }
    let case_json = cases
        .iter()
        .map(|(name, input)| {
            let report = assess_beam_theory_applicability(*input)?;
            Ok(case_receipt(name, *input, &report))
        })
        .collect::<Result<Vec<_>, String>>()?
        .join(",");

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-theory-applicability-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-THEORY-SCREEN-001\",",
            "\"references\":{{\"abaqus_beam_elements\":\"https://3dexperience.fiir.upb.ro/internal/docv6/english/SIMA3DXELMRefMap/simaelm-c-beamelem.htm\",\"ansys_beam188\":\"https://ansyshelp.ansys.com/public/Views/Secured/corp/v261/en/ans_elem/Hlp_E_BEAM188.html\"}},",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"model-applicability-screen\",",
            "\"result\":\"pass\",",
            "\"case_count\":{},",
            "\"cases\":[{}],",
            "\"claim_ceiling\":\"dimensionless model-class screening only; no solver accuracy, physical validation, structural safety or code-compliance claim\"",
            "}}"
        ),
        json_quote(&source_commit),
        json_quote(&request),
        cases.len(),
        case_json,
    ))
}

fn case_receipt(
    name: &str,
    input: BeamTheoryApplicabilityInput,
    report: &BeamTheoryApplicabilityReport,
) -> String {
    let purpose = match input.purpose {
        physical_transformation_reference::beam_theory_applicability::BeamAnalysisPurpose::StaticOrQuasiStatic => "static-or-quasi-static",
        physical_transformation_reference::beam_theory_applicability::BeamAnalysisPurpose::Modal => "modal",
    };
    let wavelength = input
        .shortest_relevant_wavelength_m
        .map(number)
        .unwrap_or_else(|| "null".to_owned());
    let proxy = report
        .uncorrected_static_cantilever_shear_deflection_ratio_proxy
        .map(number)
        .unwrap_or_else(|| "null".to_owned());
    format!(
        concat!(
            "{{",
            "\"case_id\":{},",
            "\"purpose\":{},",
            "\"member_length_m\":{},",
            "\"shortest_relevant_wavelength_m\":{},",
            "\"max_cross_section_dimension_m\":{},",
            "\"area_m2\":{},",
            "\"second_moment_m4\":{},",
            "\"youngs_modulus_pa\":{},",
            "\"shear_modulus_pa\":{},",
            "\"effective_axial_scale_m\":{},",
            "\"cross_section_to_axial_scale_ratio\":{},",
            "\"shear_slenderness_indicator\":{},",
            "\"cross_section_ratio_limit\":{},",
            "\"high_shear_slenderness_reference\":{},",
            "\"recommended_shear_slenderness_reference\":{},",
            "\"uncorrected_static_cantilever_shear_deflection_ratio_proxy\":{},",
            "\"wavelength_required_but_missing\":{},",
            "\"status\":{},",
            "\"evidence_basis\":{}",
            "}}"
        ),
        json_quote(name),
        json_quote(purpose),
        number(input.member_length_m),
        wavelength,
        number(input.max_cross_section_dimension_m),
        number(input.area_m2),
        number(input.second_moment_m4),
        number(input.youngs_modulus_pa),
        number(input.shear_modulus_pa),
        number(report.effective_axial_scale_m),
        number(report.cross_section_to_axial_scale_ratio),
        number(report.shear_slenderness_indicator),
        number(report.cross_section_ratio_limit),
        number(report.high_shear_slenderness_reference),
        number(report.recommended_shear_slenderness_reference),
        proxy,
        report.wavelength_required_but_missing,
        json_quote(&format!("{:?}", report.status)),
        json_quote(report.evidence_basis),
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
            "usage: beam-theory-applicability-reference --source-commit <40-char-git-sha>"
                .to_owned(),
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
    use super::parse_source_commit;

    #[test]
    fn source_commit_argument_is_strictly_validated() {
        assert!(parse_source_commit().is_err());
    }
}
