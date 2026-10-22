// SPDX-License-Identifier: AGPL-3.0-or-later
//! Provenance/unit-consistency audit for the SOFiSTiK 2026 BE25 beam modal
//! benchmark. This does not run BE25. It preserves the printed inputs as
//! competing interpretations and fails closed rather than guessing which
//! field or unit should be corrected.
//!
//! Primary source:
//! https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be25.pdf

use crate::Check;

/// The four end-condition families shown by SOFiSTiK BE25.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Be25BoundaryCase {
    pub name: &'static str,
    pub lambda: f64,
    pub published_reference_hz: f64,
}

/// Frequency predictions under each literal interpretation of the mass input.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Be25CaseAudit {
    pub name: &'static str,
    pub lambda: f64,
    pub published_reference_hz: f64,
    pub prediction_using_printed_mu_hz: f64,
    pub prediction_using_gamma_area_over_g_hz: f64,
    pub relative_error_printed_mu: f64,
    pub relative_error_gamma_area_over_g: f64,
    pub mass_per_length_implied_by_reference_kg_per_m: f64,
}

/// Unit and formula audit. These are source-comparison diagnostics, not solver
/// outputs or a qualified numerical benchmark result.
#[derive(Clone, Debug, PartialEq)]
pub struct Be25InputAudit {
    pub youngs_modulus_pa: f64,
    pub weight_density_n_per_m3: f64,
    pub section_area_m2: f64,
    pub second_moment_m4: f64,
    pub length_m: f64,
    pub gravity_m_per_s2: f64,
    pub mass_per_length_printed_kg_per_m: f64,
    pub mass_per_length_from_gamma_area_over_g_kg_per_m: f64,
    pub printed_to_derived_mass_ratio: f64,
    pub mean_mass_per_length_implied_by_reference_kg_per_m: f64,
    pub mass_from_reference_vs_gamma_relative_difference: f64,
    pub cases: Vec<Be25CaseAudit>,
    pub disposition: &'static str,
    pub claim_ceiling: &'static str,
}

const GRAVITY_M_PER_S2: f64 = 9.80665;
const PUBLISHED_E_MPA: f64 = 200.0;
const PUBLISHED_GAMMA_KN_PER_M3: f64 = 25.0;
const PUBLISHED_AREA_CM2: f64 = 1.0;
const PUBLISHED_SECOND_MOMENT_CM4: f64 = 0.1;
const PUBLISHED_LENGTH_M: f64 = 1.0;
const PUBLISHED_MASS_TONNE_PER_M: f64 = 0.025;
const REFERENCE_RELATIVE_TOLERANCE: f64 = 1.0e-2;

/// Canonical request: the source values are written exactly as reported in
/// BE25 Table 2/3, with conversions and disputed interpretations explicit.
pub const fn canonical_request() -> &'static str {
    concat!(
        "schema=sofistik-be25-input-audit-v0.1",
        "|benchmark=SOFiSTiK-2026-BE25",
        "|source=https://docs.sofistik.com/2026/en/verification/_static/verification/pdf/be25.pdf",
        "|E=200-MPa",
        "|gamma=25-kN-per-m3",
        "|A=1-cm2",
        "|I=0.1-cm4",
        "|L=1-m",
        "|mu_printed=0.025-tonne-per-m",
        "|g=9.80665-m-per-s2",
        "|lambdas=1.875,3.926,pi,4.73",
        "|published_reference_hz=0.457,2.003,1.283,2.907",
        "|disposition=blocked-until-source-input-resolved",
        "|claim_ceiling=unit-and-analytical-input-audit-not-BE25-reproduction"
    )
}

/// Build an audit of the literal published parameter interpretations.
///
/// The Bernoulli beam analytical expression is
/// f = lambda^2/(2*pi) * sqrt(E*I/(mu*L^4)).
pub fn audit_published_be25_inputs() -> Check<Be25InputAudit> {
    let youngs_modulus_pa = PUBLISHED_E_MPA * 1.0e6;
    let weight_density_n_per_m3 = PUBLISHED_GAMMA_KN_PER_M3 * 1.0e3;
    let section_area_m2 = PUBLISHED_AREA_CM2 * 1.0e-4;
    let second_moment_m4 = PUBLISHED_SECOND_MOMENT_CM4 * 1.0e-8;
    let length_m = PUBLISHED_LENGTH_M;
    let mass_per_length_printed_kg_per_m = PUBLISHED_MASS_TONNE_PER_M * 1.0e3;
    let mass_per_length_from_gamma_area_over_g_kg_per_m =
        weight_density_n_per_m3 * section_area_m2 / GRAVITY_M_PER_S2;

    for (name, value) in [
        ("youngs_modulus_pa", youngs_modulus_pa),
        ("weight_density_n_per_m3", weight_density_n_per_m3),
        ("section_area_m2", section_area_m2),
        ("second_moment_m4", second_moment_m4),
        ("length_m", length_m),
        ("mass_per_length_printed_kg_per_m", mass_per_length_printed_kg_per_m),
        (
            "mass_per_length_from_gamma_area_over_g_kg_per_m",
            mass_per_length_from_gamma_area_over_g_kg_per_m,
        ),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("BE25 source value {name} must be finite and positive"));
        }
    }

    let printed_to_derived_mass_ratio =
        mass_per_length_printed_kg_per_m / mass_per_length_from_gamma_area_over_g_kg_per_m;
    let boundary_cases = [
        Be25BoundaryCase {
            name: "simple-cantilever",
            lambda: 1.875,
            published_reference_hz: 0.457,
        },
        Be25BoundaryCase {
            name: "cantilever-with-simply-supported-end",
            lambda: 3.926,
            published_reference_hz: 2.003,
        },
        Be25BoundaryCase {
            name: "simply-supported",
            lambda: std::f64::consts::PI,
            published_reference_hz: 1.283,
        },
        Be25BoundaryCase {
            name: "both-ends-fixed",
            lambda: 4.73,
            published_reference_hz: 2.907,
        },
    ];

    let mut cases = Vec::with_capacity(boundary_cases.len());
    for case in boundary_cases {
        let prediction_using_printed_mu_hz = analytical_frequency_hz(
            case.lambda,
            youngs_modulus_pa,
            second_moment_m4,
            mass_per_length_printed_kg_per_m,
            length_m,
        )?;
        let prediction_using_gamma_area_over_g_hz = analytical_frequency_hz(
            case.lambda,
            youngs_modulus_pa,
            second_moment_m4,
            mass_per_length_from_gamma_area_over_g_kg_per_m,
            length_m,
        )?;
        let reference = case.published_reference_hz;
        let mass_per_length_implied_by_reference_kg_per_m = youngs_modulus_pa
            * second_moment_m4
            * case.lambda.powi(4)
            / ((2.0 * std::f64::consts::PI * reference).powi(2) * length_m.powi(4));
        cases.push(Be25CaseAudit {
            name: case.name,
            lambda: case.lambda,
            published_reference_hz: reference,
            prediction_using_printed_mu_hz,
            prediction_using_gamma_area_over_g_hz,
            relative_error_printed_mu: relative_error(prediction_using_printed_mu_hz, reference),
            relative_error_gamma_area_over_g: relative_error(
                prediction_using_gamma_area_over_g_hz,
                reference,
            ),
            mass_per_length_implied_by_reference_kg_per_m,
        });
    }

    let mean_mass_per_length_implied_by_reference_kg_per_m = cases
        .iter()
        .map(|case| case.mass_per_length_implied_by_reference_kg_per_m)
        .sum::<f64>()
        / cases.len() as f64;
    let mass_from_reference_vs_gamma_relative_difference =
        relative_error(
            mean_mass_per_length_implied_by_reference_kg_per_m,
            mass_per_length_from_gamma_area_over_g_kg_per_m,
        );

    let reference_candidates_within_tolerance = cases.iter().all(|case| {
        case.relative_error_printed_mu <= REFERENCE_RELATIVE_TOLERANCE
            || case.relative_error_gamma_area_over_g <= REFERENCE_RELATIVE_TOLERANCE
    });
    let input_fields_consistent = (printed_to_derived_mass_ratio - 1.0).abs() <= 1.0e-6;
    let disposition = if input_fields_consistent && reference_candidates_within_tolerance {
        "candidate-input-consistent-review-required"
    } else {
        "blocked-until-source-input-resolved"
    };

    Ok(Be25InputAudit {
        youngs_modulus_pa,
        weight_density_n_per_m3,
        section_area_m2,
        second_moment_m4,
        length_m,
        gravity_m_per_s2: GRAVITY_M_PER_S2,
        mass_per_length_printed_kg_per_m,
        mass_per_length_from_gamma_area_over_g_kg_per_m,
        printed_to_derived_mass_ratio,
        mean_mass_per_length_implied_by_reference_kg_per_m,
        mass_from_reference_vs_gamma_relative_difference,
        cases,
        disposition,
        claim_ceiling: "unit-and-analytical-input-audit-not-BE25-reproduction",
    })
}

fn analytical_frequency_hz(
    lambda: f64,
    youngs_modulus_pa: f64,
    second_moment_m4: f64,
    mass_per_length_kg_per_m: f64,
    length_m: f64,
) -> Check<f64> {
    for (name, value) in [
        ("lambda", lambda),
        ("youngs_modulus_pa", youngs_modulus_pa),
        ("second_moment_m4", second_moment_m4),
        ("mass_per_length_kg_per_m", mass_per_length_kg_per_m),
        ("length_m", length_m),
    ] {
        if !value.is_finite() || value <= 0.0 {
            return Err(format!("{name} must be finite and positive"));
        }
    }
    let frequency = lambda.powi(2) / (2.0 * std::f64::consts::PI)
        * (youngs_modulus_pa * second_moment_m4
            / (mass_per_length_kg_per_m * length_m.powi(4)))
            .sqrt();
    if frequency.is_finite() && frequency > 0.0 {
        Ok(frequency)
    } else {
        Err("BE25 analytical frequency is not finite and positive".to_owned())
    }
}

fn relative_error(actual: f64, reference: f64) -> f64 {
    (actual - reference).abs() / reference.abs()
}

#[cfg(test)]
mod tests {
    use super::{audit_published_be25_inputs, canonical_request};

    #[test]
    fn source_mass_per_length_interpretations_remain_explicitly_distinct() {
        let audit = audit_published_be25_inputs().expect("BE25 input audit");
        assert!((audit.mass_per_length_printed_kg_per_m - 25.0).abs() < 1.0e-14);
        assert!(
            (audit.mass_per_length_from_gamma_area_over_g_kg_per_m - 0.25492905324448206).abs()
                < 1.0e-12
        );
        assert!(
            (audit.printed_to_derived_mass_ratio - 98.0665).abs() < 1.0e-4,
            "printed/derived mass ratio={:.17e}",
            audit.printed_to_derived_mass_ratio
        );
        assert_eq!(audit.disposition, "blocked-until-source-input-resolved");
        assert!(audit.mass_from_reference_vs_gamma_relative_difference > 0.15);
    }

    #[test]
    fn published_reference_table_does_not_match_either_literal_mass_candidate() {
        let audit = audit_published_be25_inputs().expect("BE25 input audit");
        assert_eq!(audit.cases.len(), 4);
        for case in &audit.cases {
            assert!(case.relative_error_printed_mu > 0.8);
            assert!(case.relative_error_gamma_area_over_g > 0.08);
            assert!(
                (case.mass_per_length_implied_by_reference_kg_per_m - 0.3).abs() < 4.0e-4,
                "reference-implied mass for {} = {:.17e}",
                case.name,
                case.mass_per_length_implied_by_reference_kg_per_m
            );
        }
        assert!(
            (audit.cases[0].prediction_using_printed_mu_hz - 0.05004580383402026).abs() < 1.0e-12
        );
        assert!(
            (audit.cases[0].prediction_using_gamma_area_over_g_hz - 0.4955962448521477).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn canonical_request_is_the_frozen_source_statement() {
        assert_eq!(
            canonical_request(),
            include_str!("../fixtures/sofistik_be25_input_audit_request_v0_1.txt")
                .trim_end_matches('\n')
        );
    }

    #[test]
    fn invalid_positive_domain_guard_rejects_zero_mass() {
        assert!(super::analytical_frequency_hz(1.875, 2.0e8, 1.0e-9, 0.0, 1.0).is_err());
    }
}
