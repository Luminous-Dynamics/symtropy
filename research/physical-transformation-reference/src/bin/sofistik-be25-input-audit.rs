// SPDX-License-Identifier: AGPL-3.0-or-later
//! Emits a deterministic source-bound receipt for the BE25 published-input
//! inconsistency audit. "audit-complete" means the audit ran; it is not a
//! benchmark pass. The result intentionally remains blocked pending the input
//! deck/source discrepancy resolution.

use physical_transformation_reference::sofistik_be25_audit::{
    audit_published_be25_inputs, canonical_request,
};
use std::env;
use std::process::ExitCode;

const FIXED_REQUEST: &str =
    include_str!("../../fixtures/sofistik_be25_input_audit_request_v0_1.txt");

fn main() -> ExitCode {
    match run() {
        Ok(receipt) => {
            println!("{receipt}");
            ExitCode::SUCCESS
        }
        Err(error) => {
            eprintln!("sofistik-be25-input-audit: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<String, String> {
    let source_commit = parse_source_commit()?;
    let canonical = canonical_request();
    if canonical != FIXED_REQUEST.trim_end_matches('\n') {
        return Err("canonical BE25 audit request differs from frozen fixture bytes".to_owned());
    }
    let audit = audit_published_be25_inputs()?;
    let cases_json = audit
        .cases
        .iter()
        .map(|case| {
            format!(
                concat!(
                    "{{",
                    "\"name\":{},",
                    "\"lambda\":{},",
                    "\"published_reference_hz\":{},",
                    "\"prediction_using_printed_mu_hz\":{},",
                    "\"prediction_using_gamma_area_over_g_hz\":{},",
                    "\"relative_error_printed_mu\":{},",
                    "\"relative_error_gamma_area_over_g\":{},",
                    "\"mass_per_length_implied_by_reference_kg_per_m\":{}",
                    "}}"
                ),
                quote(case.name),
                number(case.lambda),
                number(case.published_reference_hz),
                number(case.prediction_using_printed_mu_hz),
                number(case.prediction_using_gamma_area_over_g_hz),
                number(case.relative_error_printed_mu),
                number(case.relative_error_gamma_area_over_g),
                number(case.mass_per_length_implied_by_reference_kg_per_m),
            )
        })
        .collect::<Vec<_>>()
        .join(",");

    Ok(format!(
        concat!(
            "{{",
            "\"schema_version\":\"sofistik-be25-input-audit-receipt-v0.1\",",
            "\"benchmark_id\":\"SOFiSTiK-2026-BE25\",",
            "\"source_commit\":{},",
            "\"request_canonical\":{},",
            "\"evidence_class\":\"published-input-unit-audit\",",
            "\"result\":\"audit-complete\",",
            "\"audit_disposition\":{},",
            "\"inputs_si\":{{",
            "\"youngs_modulus_pa\":{},",
            "\"weight_density_n_per_m3\":{},",
            "\"section_area_m2\":{},",
            "\"second_moment_m4\":{},",
            "\"length_m\":{},",
            "\"gravity_m_per_s2\":{},",
            "\"mass_per_length_printed_kg_per_m\":{},",
            "\"mass_per_length_from_gamma_area_over_g_kg_per_m\":{},",
            "\"printed_to_derived_mass_ratio\":{},",
            "\"mean_mass_per_length_implied_by_reference_kg_per_m\":{},",
            "\"mass_from_reference_vs_gamma_relative_difference\":{}",
            "}},",
            "\"cases\":[{}],",
            "\"references\":{{",
            "\"benchmark_pdf\":\"https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be25.pdf\",",
            "\"benchmark_index\":\"https://docs.sofistik.com/2026/en/verification/index.html\",",
            "\"standard_vvuq\":\"https://www.asme.org/codes-standards/find-codes-standards/standard-for-verification-and-validation-in-computational-solid-mechanics/2019\"",
            "}},",
            "\"claim_ceiling\":{}",
            "}}"
        ),
        quote(&source_commit),
        quote(canonical),
        quote(audit.disposition),
        number(audit.youngs_modulus_pa),
        number(audit.weight_density_n_per_m3),
        number(audit.section_area_m2),
        number(audit.second_moment_m4),
        number(audit.length_m),
        number(audit.gravity_m_per_s2),
        number(audit.mass_per_length_printed_kg_per_m),
        number(audit.mass_per_length_from_gamma_area_over_g_kg_per_m),
        number(audit.printed_to_derived_mass_ratio),
        number(audit.mean_mass_per_length_implied_by_reference_kg_per_m),
        number(audit.mass_from_reference_vs_gamma_relative_difference),
        cases_json,
        quote(audit.claim_ceiling),
    ))
}

fn parse_source_commit() -> Result<String, String> {
    let args: Vec<String> = env::args().skip(1).collect();
    if args.len() != 2 || args[0] != "--source-commit" {
        return Err("usage: sofistik-be25-input-audit --source-commit <40-char-git-sha>".to_owned());
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
    use super::{parse_source_commit, quote};

    #[test]
    fn source_commit_is_strictly_validated_and_normalized() {
        assert!(parse_source_commit().is_err());
    }

    #[test]
    fn json_quote_escapes_control_characters() {
        assert_eq!(quote("quote\" slash\\ newline\n"), "\"quote\\\" slash\\\\ newline\\n\"");
    }
}
