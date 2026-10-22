// SPDX-License-Identifier: AGPL-3.0-or-later
//! Dimensionless applicability screen for Euler-Bernoulli versus shear-flexible
//! beam theory. The screen is advisory: it can request a Timoshenko comparison
//! or decline a modal recommendation, but it cannot validate a structure.
//!
//! Reference heuristics:
//! - Abaqus beam-element guidance describes cross-section dimensions below
//!   about 1/15 of typical axial distances / participating-mode wavelengths as
//!   a useful Euler-Bernoulli slenderness criterion.
//! - ANSYS BEAM188 guidance uses G*A*L^2/(E*I) and recommends values above 30
//!   for its Timoshenko element. Its cantilever example reports Timoshenko /
//!   Euler-Bernoulli deflection ratios of 1.12, 1.06, and 1.03 at indicator
//!   values 25, 50, and 100, respectively.
//! These source-specific heuristics are not universal error bounds.

use crate::Check;

/// Analysis purpose controls whether a modal wavelength is a required input.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeamAnalysisPurpose {
    StaticOrQuasiStatic,
    Modal,
}

/// Result of the dimensionless applicability screen. Candidate means only
/// that both declared screening heuristics fall in their slender range; it is
/// not a claim of numerical convergence or physical validation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BeamTheoryScreenStatus {
    EulerBernoulliCandidate,
    TimoshenkoComparisonRecommended,
    EulerBernoulliOutsideReferenceScreen,
    MissingDynamicWavelength,
}

/// All geometric/material quantities are explicit SI inputs. The maximum
/// cross-section dimension is a declared bounding dimension; the screen does
/// not guess whether a section annotation means radius or diameter.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BeamTheoryApplicabilityInput {
    pub purpose: BeamAnalysisPurpose,
    pub member_length_m: f64,
    pub shortest_relevant_wavelength_m: Option<f64>,
    pub max_cross_section_dimension_m: f64,
    pub area_m2: f64,
    pub second_moment_m4: f64,
    pub youngs_modulus_pa: f64,
    pub shear_modulus_pa: f64,
}

/// Explicit dimensionless indicators and recommendation. The static cantilever
/// proxy is reported only for static/quasi-static requests; it must not be read
/// as a modal frequency error estimate.
#[derive(Clone, Debug, PartialEq)]
pub struct BeamTheoryApplicabilityReport {
    pub status: BeamTheoryScreenStatus,
    pub effective_axial_scale_m: f64,
    pub cross_section_to_axial_scale_ratio: f64,
    /// G*A*L_eff^2/(E*I), following the cited ANSYS BEAM188 guidance.
    pub shear_slenderness_indicator: f64,
    pub cross_section_ratio_limit: f64,
    pub high_shear_slenderness_reference: f64,
    pub recommended_shear_slenderness_reference: f64,
    /// Uncorrected 3/S proxy for the classical cantilever comparison. It omits
    /// section shear-correction factors and is not a predicted error bound.
    pub uncorrected_static_cantilever_shear_deflection_ratio_proxy: Option<f64>,
    pub wavelength_required_but_missing: bool,
    pub evidence_basis: &'static str,
    pub claim_ceiling: &'static str,
}

const CROSS_SECTION_RATIO_LIMIT: f64 = 1.0 / 15.0;
const OUTSIDE_REFERENCE_CROSS_SECTION_RATIO: f64 = 1.0 / 8.0;
const SHEAR_SLENDERNESS_CANDIDATE_LIMIT: f64 = 100.0;
const SHEAR_SLENDERNESS_COMPARISON_LIMIT: f64 = 30.0;

/// Evaluate the named screening indicators and return an explicit advisory
/// status. For modal analysis, the effective length is min(member length,
/// shortest relevant wavelength); without that wavelength, the result remains
/// unqualified even when the member's overall geometry is slender.
pub fn assess_beam_theory_applicability(
    input: BeamTheoryApplicabilityInput,
) -> Check<BeamTheoryApplicabilityReport> {
    validate_positive("member_length_m", input.member_length_m)?;
    validate_positive(
        "max_cross_section_dimension_m",
        input.max_cross_section_dimension_m,
    )?;
    validate_positive("area_m2", input.area_m2)?;
    validate_positive("second_moment_m4", input.second_moment_m4)?;
    validate_positive("youngs_modulus_pa", input.youngs_modulus_pa)?;
    validate_positive("shear_modulus_pa", input.shear_modulus_pa)?;
    if let Some(wavelength) = input.shortest_relevant_wavelength_m {
        validate_positive("shortest_relevant_wavelength_m", wavelength)?;
    }

    let wavelength_required_but_missing =
        input.purpose == BeamAnalysisPurpose::Modal
            && input.shortest_relevant_wavelength_m.is_none();
    let effective_axial_scale_m = input
        .shortest_relevant_wavelength_m
        .map_or(input.member_length_m, |wavelength| {
            input.member_length_m.min(wavelength)
        });
    let cross_section_ratio =
        input.max_cross_section_dimension_m / effective_axial_scale_m;
    let shear_slenderness = input.shear_modulus_pa
        * input.area_m2
        * effective_axial_scale_m.powi(2)
        / (input.youngs_modulus_pa * input.second_moment_m4);
    if !cross_section_ratio.is_finite()
        || cross_section_ratio <= 0.0
        || !shear_slenderness.is_finite()
        || shear_slenderness <= 0.0
    {
        return Err("dimensionless applicability indicators must be finite and positive".to_owned());
    }

    let status = if wavelength_required_but_missing {
        BeamTheoryScreenStatus::MissingDynamicWavelength
    } else if cross_section_ratio > OUTSIDE_REFERENCE_CROSS_SECTION_RATIO
        || shear_slenderness <= SHEAR_SLENDERNESS_COMPARISON_LIMIT
    {
        BeamTheoryScreenStatus::EulerBernoulliOutsideReferenceScreen
    } else if cross_section_ratio <= CROSS_SECTION_RATIO_LIMIT
        && shear_slenderness > SHEAR_SLENDERNESS_CANDIDATE_LIMIT
    {
        BeamTheoryScreenStatus::EulerBernoulliCandidate
    } else {
        BeamTheoryScreenStatus::TimoshenkoComparisonRecommended
    };

    // This is deliberately uncorrected: the shear coefficient depends on the
    // section/beam formulation and must not be silently guessed here.
    let static_proxy = if input.purpose == BeamAnalysisPurpose::StaticOrQuasiStatic {
        Some(3.0 / shear_slenderness)
    } else {
        None
    };

    Ok(BeamTheoryApplicabilityReport {
        status,
        effective_axial_scale_m,
        cross_section_to_axial_scale_ratio: cross_section_ratio,
        shear_slenderness_indicator: shear_slenderness,
        cross_section_ratio_limit: CROSS_SECTION_RATIO_LIMIT,
        high_shear_slenderness_reference: SHEAR_SLENDERNESS_CANDIDATE_LIMIT,
        recommended_shear_slenderness_reference: SHEAR_SLENDERNESS_COMPARISON_LIMIT,
        uncorrected_static_cantilever_shear_deflection_ratio_proxy: static_proxy,
        wavelength_required_but_missing,
        evidence_basis: "Abaqus slenderness heuristic and ANSYS BEAM188 G*A*L^2/(E*I) heuristic; these are screening references, not universal error guarantees",
        claim_ceiling: "model-applicability screen only; no solver verification, experimental validation, structural safety or code-compliance conclusion",
    })
}

fn validate_positive(name: &str, value: f64) -> Check<()> {
    if value.is_finite() && value > 0.0 {
        Ok(())
    } else {
        Err(format!("{name} must be finite and positive"))
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BeamAnalysisPurpose, BeamTheoryApplicabilityInput, BeamTheoryScreenStatus,
        assess_beam_theory_applicability,
    };

    fn slender_input() -> BeamTheoryApplicabilityInput {
        BeamTheoryApplicabilityInput {
            purpose: BeamAnalysisPurpose::StaticOrQuasiStatic,
            member_length_m: 1.0,
            shortest_relevant_wavelength_m: None,
            max_cross_section_dimension_m: 0.01,
            area_m2: 7.853981633974483e-5,
            second_moment_m4: 4.908738521234052e-10,
            youngs_modulus_pa: 210.0e9,
            shear_modulus_pa: 80.0e9,
        }
    }

    #[test]
    fn slender_static_member_is_only_an_euler_bernoulli_candidate() {
        let result = assess_beam_theory_applicability(slender_input()).expect("screen result");
        assert_eq!(result.status, BeamTheoryScreenStatus::EulerBernoulliCandidate);
        assert!(result.cross_section_to_axial_scale_ratio <= 1.0 / 15.0);
        assert!(result.shear_slenderness_indicator > 100.0);
        assert!(
            result
                .static_cantilever_shear_deflection_fraction_proxy
                .expect("static proxy")
                < 0.03
        );
        assert!(!result.wavelength_required_but_missing);
        assert!(result.claim_ceiling.contains("no solver verification"));
    }

    #[test]
    fn high_frequency_short_wavelength_can_reject_a_globally_slender_member() {
        let mut input = slender_input();
        input.purpose = BeamAnalysisPurpose::Modal;
        input.shortest_relevant_wavelength_m = Some(0.04);
        let result = assess_beam_theory_applicability(input).expect("modal screen");
        assert_eq!(
            result.status,
            BeamTheoryScreenStatus::EulerBernoulliOutsideReferenceScreen
        );
        assert!((result.effective_axial_scale_m - 0.04).abs() < 1.0e-14);
        assert!(result.cross_section_to_axial_scale_ratio > 1.0 / 8.0);
        assert!(result.uncorrected_static_cantilever_shear_deflection_ratio_proxy.is_none());
    }

    #[test]
    fn missing_modal_wavelength_stays_unqualified_instead_of_crashing() {
        let mut input = slender_input();
        input.purpose = BeamAnalysisPurpose::Modal;
        input.shortest_relevant_wavelength_m = None;
        let result = assess_beam_theory_applicability(input).expect("incomplete modal screen");
        assert_eq!(result.status, BeamTheoryScreenStatus::MissingDynamicWavelength);
        assert!(result.wavelength_required_but_missing);
    }

    #[test]
    fn intermediate_slenderness_requests_a_timoshenko_comparison() {
        let mut input = slender_input();
        input.member_length_m = 0.2;
        input.max_cross_section_dimension_m = 0.02;
        input.area_m2 = 0.0004;
        input.second_moment_m4 = 1.3333333333333333e-8;
        let result = assess_beam_theory_applicability(input).expect("intermediate screen");
        assert_eq!(
            result.status,
            BeamTheoryScreenStatus::TimoshenkoComparisonRecommended
        );
    }

    fn input_with_shear_slenderness(target: f64) -> BeamTheoryApplicabilityInput {
        let mut input = slender_input();
        input.second_moment_m4 = input.shear_modulus_pa * input.area_m2
            * input.member_length_m.powi(2)
            / (target * input.youngs_modulus_pa);
        input
    }

    #[test]
    fn shear_slenderness_thresholds_are_separate_and_tested() {
        let outside = assess_beam_theory_applicability(input_with_shear_slenderness(25.0))
            .expect("low shear slenderness");
        let comparison = assess_beam_theory_applicability(input_with_shear_slenderness(50.0))
            .expect("intermediate shear slenderness");
        let candidate = assess_beam_theory_applicability(input_with_shear_slenderness(150.0))
            .expect("high shear slenderness");

        assert_eq!(outside.status, BeamTheoryScreenStatus::EulerBernoulliOutsideReferenceScreen);
        assert_eq!(comparison.status, BeamTheoryScreenStatus::TimoshenkoComparisonRecommended);
        assert_eq!(candidate.status, BeamTheoryScreenStatus::EulerBernoulliCandidate);
        assert!((outside.shear_slenderness_indicator - 25.0).abs() < 1.0e-12);
        assert!((comparison.shear_slenderness_indicator - 50.0).abs() < 1.0e-12);
        assert!((candidate.shear_slenderness_indicator - 150.0).abs() < 1.0e-12);
    }

    #[test]
    fn uniform_geometric_rescaling_preserves_dimensionless_decision() {
        let input = slender_input();
        let baseline = assess_beam_theory_applicability(input).expect("baseline");
        let scale = 7.0;
        let mut rescaled = input;
        rescaled.member_length_m *= scale;
        rescaled.max_cross_section_dimension_m *= scale;
        rescaled.area_m2 *= scale * scale;
        rescaled.second_moment_m4 *= scale.powi(4);
        let changed = assess_beam_theory_applicability(rescaled).expect("rescaled");
        assert_eq!(baseline.status, changed.status);
        assert!(
            (baseline.cross_section_to_axial_scale_ratio
                - changed.cross_section_to_axial_scale_ratio)
                .abs()
                < 1.0e-14
        );
        assert!(
            (baseline.shear_slenderness_indicator - changed.shear_slenderness_indicator).abs()
                < 1.0e-12
        );
    }

    #[test]
    fn invalid_units_or_nonphysical_inputs_fail_closed() {
        let mut input = slender_input();
        input.second_moment_m4 = 0.0;
        assert!(assess_beam_theory_applicability(input).is_err());
        input = slender_input();
        input.shortest_relevant_wavelength_m = Some(f64::NAN);
        assert!(assess_beam_theory_applicability(input).is_err());
    }
}
