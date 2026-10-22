// SPDX-License-Identifier: AGPL-3.0-or-later
//! Deterministic, request-bound modal sensitivity receipt for the four declared
//! NAFEMS-shaped beam-chain uncertainty parameters. This is local sensitivity,
//! not a probabilistic confidence interval or physical validation.

use physical_transformation_reference::beam_pin_assembly::{BeamMemberChain, BeamPinNode};
use physical_transformation_reference::modal_uncertainty::{
    BeamInputUncertainty, BeamModalUncertaintyReport, BeamUncertaintyParameter,
    SensitivityDisposition, canonical_modal_uncertainty_request,
    evaluate_beam_modal_uncertainty,
};
use std::env;
use std::process::ExitCode;

const FROZEN_REQUEST: &str =
    include_str!("../../fixtures/modal_uncertainty_request_v0_1.txt");
const SAMPLING_POINTS_PER_MEMBER: usize = 17;

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("beam-modal-uncertainty-reference: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let joints = fixture_joints();
    let members = fixture_members();
    let parameters = fixture_uncertainties();
    let request = canonical_modal_uncertainty_request(
        &joints,
        &members,
        &parameters,
        SAMPLING_POINTS_PER_MEMBER,
    )?;
    if request != FROZEN_REQUEST.trim_end_matches('\n') {
        return Err("canonical modal uncertainty request differs from frozen fixture bytes".to_owned());
    }

    let report = evaluate_beam_modal_uncertainty(
        &joints,
        &members,
        &parameters,
        SAMPLING_POINTS_PER_MEMBER,
    )?;
    verify_exact_scaling_laws(&report)?;

    let parameters_json = report
        .parameters
        .iter()
        .map(|spec| {
            format!(
                "{{\"parameter\":{},\"assumed_relative_uncertainty\":{},\"finite_difference_step_relative\":{}}}",
                quote(spec.parameter.as_str()),
                number(spec.assumed_relative_uncertainty),
                number(spec.finite_difference_step_relative)
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let results_json = report
        .sensitivity_results
        .iter()
        .map(sensitivity_json)
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"beam-modal-uncertainty-receipt-v0.1\",",
            "\"benchmark_id\":\"PHYS-BEAM-MODAL-UQ-001\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"deterministic-local-modal-sensitivity\",",
            "\"result\":\"pass\",",
            "\"baseline_mode_count\":{},",
            "\"sampling_points_per_member\":{},",
            "\"qualified_sensitivity_count\":{},",
            "\"abstained_sensitivity_count\":{},",
            "\"parameters\":[{}],",
            "\"sensitivity_results\":[{}],",
            "\"scaling_law_gates\":{{",
            "\"youngs_modulus_log_sensitivity_plus_half\":true,",
            "\"density_log_sensitivity_minus_half\":true,",
            "\"declared_modulus_uncertainty_contribution_matches\":true,",
            "\"declared_density_uncertainty_contribution_matches\":true",
            "}},",
            "\"claim_ceiling\":{}",
            "}}"
        ),
        quote(&source_commit),
        quote(&request),
        report.baseline_mode_count,
        report.sampling_points_per_member,
        report.qualified_sensitivity_count,
        report.abstained_sensitivity_count,
        parameters_json,
        results_json,
        quote(report.claim_ceiling)
    ))
}

fn verify_exact_scaling_laws(report: &BeamModalUncertaintyReport) -> Result<(), String> {
    for (parameter, expected_sensitivity, expected_contribution) in [
        (BeamUncertaintyParameter::YoungsModulus, 0.5, 0.025),
        (BeamUncertaintyParameter::Density, -0.5, 0.015),
    ] {
        let qualifying = report
            .sensitivity_results
            .iter()
            .filter(|result| {
                result.parameter == parameter && result.disposition == SensitivityDisposition::Qualified
            })
            .collect::<Vec<_>>();
        if qualifying.is_empty() {
            return Err(format!(
                "no isolated mode qualified for exact scaling law {}",
                parameter.as_str()
            ));
        }
        for result in qualifying {
            let sensitivity = result
                .logarithmic_frequency_sensitivity
                .ok_or_else(|| "qualified result lacks sensitivity".to_owned())?;
            let contribution = result
                .estimated_relative_frequency_uncertainty
                .ok_or_else(|| "qualified result lacks uncertainty contribution".to_owned())?;
            if (sensitivity - expected_sensitivity).abs() >= 1.0e-5
                || (contribution - expected_contribution).abs() >= 1.0e-6
            {
                return Err(format!(
                    "scaling law mismatch for {} mode {}: sensitivity={sensitivity:.17e}, contribution={contribution:.17e}",
                    parameter.as_str(),
                    result.baseline_mode_index
                ));
            }
        }
    }
    Ok(())
}

fn sensitivity_json(result: &physical_transformation_reference::modal_uncertainty::BeamModeSensitivity) -> String {
    format!(
        concat!(
            "{{",
            "\"baseline_mode_index\":{},",
            "\"parameter\":{},",
            "\"assumed_relative_uncertainty\":{},",
            "\"baseline_frequency_hz\":{},",
            "\"plus_frequency_hz\":{},",
            "\"minus_frequency_hz\":{},",
            "\"plus_matched_mode_index\":{},",
            "\"minus_matched_mode_index\":{},",
            "\"plus_mac\":{},",
            "\"minus_mac\":{},",
            "\"logarithmic_frequency_sensitivity\":{},",
            "\"estimated_relative_frequency_uncertainty\":{},",
            "\"disposition\":{}",
            "}}"
        ),
        result.baseline_mode_index,
        quote(result.parameter.as_str()),
        number(result.assumed_relative_uncertainty),
        optional_number(result.baseline_frequency_hz),
        optional_number(result.plus_frequency_hz),
        optional_number(result.minus_frequency_hz),
        optional_index(result.plus_matched_mode_index),
        optional_index(result.minus_matched_mode_index),
        optional_number(result.plus_mac),
        optional_number(result.minus_mac),
        optional_number(result.logarithmic_frequency_sensitivity),
        optional_number(result.estimated_relative_frequency_uncertainty),
        quote(disposition_str(result.disposition))
    )
}

fn disposition_str(disposition: SensitivityDisposition) -> &'static str {
    match disposition {
        SensitivityDisposition::Qualified => "qualified",
        SensitivityDisposition::BaselineModeNotElastic => "baseline_mode_not_elastic",
        SensitivityDisposition::BaselineModeClustered => "baseline_mode_clustered",
        SensitivityDisposition::PerturbedModeClustered => "perturbed_mode_clustered",
        SensitivityDisposition::ModeAssociationAmbiguous => "mode_association_ambiguous",
        SensitivityDisposition::NoQualifyingModeMatch => "no_qualifying_mode_match",
    }
}

fn fixture_joints() -> Vec<BeamPinNode> {
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

fn fixture_members() -> Vec<BeamMemberChain> {
    let radius_m = 0.01_f64;
    let area_m2 = std::f64::consts::PI * radius_m.powi(2);
    let second_moment_m4 = std::f64::consts::PI * radius_m.powi(4) / 4.0;
    vec![
        BeamMemberChain {
            id: "AC".to_owned(),
            start_node_id: "A".to_owned(),
            end_node_id: "C".to_owned(),
            area_m2,
            youngs_modulus_pa: 210.0e9,
            density_kg_m3: 7800.0,
            second_moment_m4,
            elements_per_member: 1,
        },
        BeamMemberChain {
            id: "BC".to_owned(),
            start_node_id: "B".to_owned(),
            end_node_id: "C".to_owned(),
            area_m2,
            youngs_modulus_pa: 210.0e9,
            density_kg_m3: 7800.0,
            second_moment_m4,
            elements_per_member: 1,
        },
    ]
}

fn fixture_uncertainties() -> Vec<BeamInputUncertainty> {
    let finite_difference_step_relative = 0.001;
    vec![
        BeamInputUncertainty {
            parameter: BeamUncertaintyParameter::YoungsModulus,
            assumed_relative_uncertainty: 0.05,
            finite_difference_step_relative,
        },
        BeamInputUncertainty {
            parameter: BeamUncertaintyParameter::Density,
            assumed_relative_uncertainty: 0.03,
            finite_difference_step_relative,
        },
        BeamInputUncertainty {
            parameter: BeamUncertaintyParameter::SectionRadius,
            assumed_relative_uncertainty: 0.02,
            finite_difference_step_relative,
        },
        BeamInputUncertainty {
            parameter: BeamUncertaintyParameter::GeometryScale,
            assumed_relative_uncertainty: 0.01,
            finite_difference_step_relative,
        },
    ]
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err(
            "usage: beam-modal-uncertainty-reference --source-commit <40-char-git-sha>".to_owned(),
        );
    }
    let value = &args[1];
    if value.len() != 40 || !value.chars().all(|character| character.is_ascii_hexdigit()) {
        return Err("--source-commit must be a 40-character Git SHA-1".to_owned());
    }
    Ok(value.to_ascii_lowercase())
}

fn number(value: f64) -> String {
    format!("{value:.17e}")
}

fn optional_number(value: Option<f64>) -> String {
    value.map(number).unwrap_or_else(|| "null".to_owned())
}

fn optional_index(value: Option<usize>) -> String {
    value.map(|index| index.to_string()).unwrap_or_else(|| "null".to_owned())
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
    use super::{disposition_str, fixture_uncertainties, parse_source_commit, quote};
    use physical_transformation_reference::modal_uncertainty::SensitivityDisposition;

    #[test]
    fn source_commit_json_string_and_disposition_are_strict() {
        assert!(parse_source_commit().is_err());
        assert_eq!(disposition_str(SensitivityDisposition::Qualified), "qualified");
        assert_eq!(
            quote("quote\" slash\\ newline\n"),
            "\"quote\\\" slash\\\\ newline\\n\""
        );
        assert_eq!(fixture_uncertainties().len(), 4);
    }
}
