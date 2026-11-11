// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bernoulli outcome-distribution sensitivity for causal effect transport.
//!
//! This is a finite, binary-outcome specialization of the bounded likelihood-
//! ratio sensitivity model studied in CLeaR 2026. It is not the paper's general
//! continuous-outcome estimator and does not include sampling uncertainty.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use crate::causal_transport::{
    AssumptionDisposition, AssumptionEvidence, CausalTransportAssumptions, CausalTransportInput,
    SourceEffectStratum, TargetPopulationStratum,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivityIdentificationStatus {
    Identified,
    SensitivityBounded,
    PartiallyIdentified,
    NotIdentified,
}

impl SensitivityIdentificationStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::Identified => "identified",
            Self::SensitivityBounded => "sensitivity_bounded",
            Self::PartiallyIdentified => "partially_identified",
            Self::NotIdentified => "not_identified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SensitivityIdentificationReason {
    ConditionalInvarianceAtLambdaOne,
    BoundedOutcomeDistributionShift,
    UnsupportedTargetModifierMass,
    RequiredAssumptionNotSupported,
}

impl SensitivityIdentificationReason {
    pub const fn name(self) -> &'static str {
        match self {
            Self::ConditionalInvarianceAtLambdaOne => "conditional_invariance_at_lambda_one",
            Self::BoundedOutcomeDistributionShift => "bounded_outcome_distribution_shift",
            Self::UnsupportedTargetModifierMass => "unsupported_target_modifier_mass",
            Self::RequiredAssumptionNotSupported => "required_assumption_not_supported",
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct BernoulliRiskShiftBounds {
    pub source_risk: f64,
    pub target_risk_lower: f64,
    pub target_risk_upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeShiftStratumResult {
    pub modifier_id: String,
    pub target_population_count: u64,
    pub target_population_share: f64,
    pub source_support: bool,
    pub control: Option<BernoulliRiskShiftBounds>,
    pub treatment: Option<BernoulliRiskShiftBounds>,
    pub risk_difference_lower: Option<f64>,
    pub risk_difference_upper: Option<f64>,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CausalTransportSensitivityResult {
    pub estimand: &'static str,
    pub likelihood_ratio_bound: f64,
    pub sensitivity_scenario_digest: String,
    pub identification_status: SensitivityIdentificationStatus,
    pub identification_reason: SensitivityIdentificationReason,
    pub source_ate: f64,
    pub target_ate_point: Option<f64>,
    pub target_ate_lower: Option<f64>,
    pub target_ate_upper: Option<f64>,
    pub sensitivity_interval_width: Option<f64>,
    pub source_population_total: u64,
    pub target_population_total: u64,
    pub supported_target_population_count: u64,
    pub unsupported_target_population_count: u64,
    pub supported_target_mass: f64,
    pub unsupported_target_mass: f64,
    pub assumptions: CausalTransportAssumptions,
    pub sensitivity_evidence: AssumptionEvidence,
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub protocol_digest: String,
    pub strata: Vec<OutcomeShiftStratumResult>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LambdaTippingPoint {
    pub lambda: f64,
    pub lower_bracket: f64,
    pub upper_bracket: f64,
    pub tolerance: f64,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeShiftSensitivityError {
    MissingDataDigest,
    MissingSensitivityScenarioDigest,
    MissingAssumptionEvidenceDigest,
    SensitivityEnvelopeNotSupported,
    EmptySourceStrata,
    EmptyTargetStrata,
    CountOverflow,
    ZeroSourcePopulation,
    ZeroTargetPopulation,
    InvalidLambda,
    InvalidTippingSearch,
    EmptyModifierId,
    DuplicateSourceModifier { modifier_id: String },
    DuplicateTargetModifier { modifier_id: String },
    InvalidSourceStratum { modifier_id: String },
    InvalidTargetStratum { modifier_id: String },
    InconsistentBernoulliBounds,
    UnsupportedTargetMassBelowFloatingPrecision,
    NonFiniteResult,
}

impl fmt::Display for OutcomeShiftSensitivityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for OutcomeShiftSensitivityError {}

fn is_supported(disposition: AssumptionDisposition) -> bool {
    matches!(
        disposition,
        AssumptionDisposition::SupportedByDesign | AssumptionDisposition::SupportedByEvidence
    )
}

fn required_assumptions_supported(assumptions: &CausalTransportAssumptions) -> bool {
    is_supported(assumptions.source_internal_validity.disposition)
        && is_supported(assumptions.no_interference.disposition)
        && is_supported(assumptions.treatment_version_invariance.disposition)
}

fn validate_evidence(
    input: &CausalTransportInput,
    sensitivity_evidence: &AssumptionEvidence,
) -> Result<(), OutcomeShiftSensitivityError> {
    if input.source_data_digest.trim().is_empty()
        || input.target_population_digest.trim().is_empty()
        || input.protocol_digest.trim().is_empty()
    {
        return Err(OutcomeShiftSensitivityError::MissingDataDigest);
    }
    for evidence in [
        &input.assumptions.source_internal_validity,
        &input.assumptions.selection_exchangeability,
        &input.assumptions.no_interference,
        &input.assumptions.treatment_version_invariance,
    ] {
        if evidence.evidence_digest.trim().is_empty() {
            return Err(OutcomeShiftSensitivityError::MissingAssumptionEvidenceDigest);
        }
    }
    if sensitivity_evidence.evidence_digest.trim().is_empty() {
        return Err(OutcomeShiftSensitivityError::MissingSensitivityScenarioDigest);
    }
    if !is_supported(sensitivity_evidence.disposition) {
        return Err(OutcomeShiftSensitivityError::SensitivityEnvelopeNotSupported);
    }
    Ok(())
}

fn validate_source_strata(
    strata: &[SourceEffectStratum],
) -> Result<(BTreeMap<&str, &SourceEffectStratum>, u64), OutcomeShiftSensitivityError> {
    if strata.is_empty() {
        return Err(OutcomeShiftSensitivityError::EmptySourceStrata);
    }
    let mut indexed = BTreeMap::new();
    let mut total = 0_u64;
    for stratum in strata {
        if stratum.modifier_id.trim().is_empty() {
            return Err(OutcomeShiftSensitivityError::EmptyModifierId);
        }
        if stratum.source_population_count == 0
            || stratum.control_sample_size == 0
            || stratum.treatment_sample_size == 0
            || stratum.control_events > stratum.control_sample_size
            || stratum.treatment_events > stratum.treatment_sample_size
        {
            return Err(OutcomeShiftSensitivityError::InvalidSourceStratum {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        if indexed.insert(stratum.modifier_id.as_str(), stratum).is_some() {
            return Err(OutcomeShiftSensitivityError::DuplicateSourceModifier {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        total = total
            .checked_add(stratum.source_population_count)
            .ok_or(OutcomeShiftSensitivityError::CountOverflow)?;
    }
    if total == 0 {
        return Err(OutcomeShiftSensitivityError::ZeroSourcePopulation);
    }
    Ok((indexed, total))
}

fn validate_target_strata(
    strata: &[TargetPopulationStratum],
) -> Result<(BTreeMap<&str, &TargetPopulationStratum>, u64), OutcomeShiftSensitivityError> {
    if strata.is_empty() {
        return Err(OutcomeShiftSensitivityError::EmptyTargetStrata);
    }
    let mut indexed = BTreeMap::new();
    let mut total = 0_u64;
    for stratum in strata {
        if stratum.modifier_id.trim().is_empty() {
            return Err(OutcomeShiftSensitivityError::EmptyModifierId);
        }
        if stratum.target_population_count == 0 {
            return Err(OutcomeShiftSensitivityError::InvalidTargetStratum {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        if indexed.insert(stratum.modifier_id.as_str(), stratum).is_some() {
            return Err(OutcomeShiftSensitivityError::DuplicateTargetModifier {
                modifier_id: stratum.modifier_id.clone(),
            });
        }
        total = total
            .checked_add(stratum.target_population_count)
            .ok_or(OutcomeShiftSensitivityError::CountOverflow)?;
    }
    if total == 0 {
        return Err(OutcomeShiftSensitivityError::ZeroTargetPopulation);
    }
    Ok((indexed, total))
}

/// Exact support-restricted risk interval for a Bernoulli outcome.
/// The two category likelihood ratios lie in [1/Lambda, Lambda] and normalize
/// to a target Bernoulli distribution. Source structural zeros/ones stay fixed.
pub fn bernoulli_outcome_shift_bounds(
    source_risk: f64,
    lambda: f64,
) -> Result<(f64, f64), OutcomeShiftSensitivityError> {
    if !lambda.is_finite() || lambda < 1.0 {
        return Err(OutcomeShiftSensitivityError::InvalidLambda);
    }
    if !source_risk.is_finite() || !(0.0..=1.0).contains(&source_risk) {
        return Err(OutcomeShiftSensitivityError::InconsistentBernoulliBounds);
    }
    if source_risk == 0.0 || source_risk == 1.0 {
        return Ok((source_risk, source_risk));
    }

    let lower = (source_risk / lambda)
        .max(1.0 - lambda * (1.0 - source_risk))
        .max(0.0);
    let upper = (lambda * source_risk)
        .min(1.0 - (1.0 - source_risk) / lambda)
        .min(1.0);
    if !lower.is_finite()
        || !upper.is_finite()
        || lower < 0.0
        || upper > 1.0
        || lower > upper + 16.0 * f64::EPSILON
    {
        return Err(OutcomeShiftSensitivityError::InconsistentBernoulliBounds);
    }
    Ok((lower.clamp(0.0, 1.0), upper.clamp(0.0, 1.0)))
}

fn source_ate(source_strata: &[SourceEffectStratum], source_total: u64) -> f64 {
    source_strata
        .iter()
        .map(|stratum| {
            let control = stratum.control_events as f64 / stratum.control_sample_size as f64;
            let treatment = stratum.treatment_events as f64 / stratum.treatment_sample_size as f64;
            (stratum.source_population_count as f64 / source_total as f64)
                * (treatment - control)
        })
        .sum()
}

/// Evaluate target ATE bounds under one declared Bernoulli likelihood-ratio
/// envelope. Conditional invariance is replaced by this sensitivity model;
/// source internal validity, no interference and treatment-version invariance
/// remain required. Unsupported target modifier mass keeps risk difference
/// bounds [-1, 1] and is never imputed through the source distribution.
pub fn assess_bernoulli_outcome_shift_transport(
    input: &CausalTransportInput,
    lambda: f64,
    sensitivity_evidence: &AssumptionEvidence,
) -> Result<CausalTransportSensitivityResult, OutcomeShiftSensitivityError> {
    if !lambda.is_finite() || lambda < 1.0 {
        return Err(OutcomeShiftSensitivityError::InvalidLambda);
    }
    validate_evidence(input, sensitivity_evidence)?;
    let (source_by_modifier, source_total) = validate_source_strata(&input.source_strata)?;
    let (_target_by_modifier, target_total) = validate_target_strata(&input.target_strata)?;

    let mut supported_target_population_count = 0_u64;
    let mut supported_lower = 0.0;
    let mut supported_upper = 0.0;
    let mut strata = Vec::with_capacity(input.target_strata.len());

    for target in &input.target_strata {
        let target_share = target.target_population_count as f64 / target_total as f64;
        if let Some(source) = source_by_modifier.get(target.modifier_id.as_str()) {
            supported_target_population_count = supported_target_population_count
                .checked_add(target.target_population_count)
                .ok_or(OutcomeShiftSensitivityError::CountOverflow)?;

            let control_source =
                source.control_events as f64 / source.control_sample_size as f64;
            let treatment_source =
                source.treatment_events as f64 / source.treatment_sample_size as f64;
            let (control_lower, control_upper) =
                bernoulli_outcome_shift_bounds(control_source, lambda)?;
            let (treatment_lower, treatment_upper) =
                bernoulli_outcome_shift_bounds(treatment_source, lambda)?;
            let risk_difference_lower = treatment_lower - control_upper;
            let risk_difference_upper = treatment_upper - control_lower;
            supported_lower += target_share * risk_difference_lower;
            supported_upper += target_share * risk_difference_upper;

            strata.push(OutcomeShiftStratumResult {
                modifier_id: target.modifier_id.clone(),
                target_population_count: target.target_population_count,
                target_population_share: target_share,
                source_support: true,
                control: Some(BernoulliRiskShiftBounds {
                    source_risk: control_source,
                    target_risk_lower: control_lower,
                    target_risk_upper: control_upper,
                }),
                treatment: Some(BernoulliRiskShiftBounds {
                    source_risk: treatment_source,
                    target_risk_lower: treatment_lower,
                    target_risk_upper: treatment_upper,
                }),
                risk_difference_lower: Some(risk_difference_lower),
                risk_difference_upper: Some(risk_difference_upper),
            });
        } else {
            strata.push(OutcomeShiftStratumResult {
                modifier_id: target.modifier_id.clone(),
                target_population_count: target.target_population_count,
                target_population_share: target_share,
                source_support: false,
                control: None,
                treatment: None,
                risk_difference_lower: None,
                risk_difference_upper: None,
            });
        }
    }

    let unsupported_target_population_count = target_total
        .checked_sub(supported_target_population_count)
        .ok_or(OutcomeShiftSensitivityError::CountOverflow)?;
    let supported_target_mass = supported_target_population_count as f64 / target_total as f64;
    let unsupported_target_mass = unsupported_target_population_count as f64 / target_total as f64;
    if unsupported_target_population_count > 0 && unsupported_target_mass <= f64::EPSILON {
        return Err(OutcomeShiftSensitivityError::UnsupportedTargetMassBelowFloatingPrecision);
    }

    let source_ate = source_ate(&input.source_strata, source_total);
    let assumptions_supported = required_assumptions_supported(&input.assumptions);
    let (identification_status, identification_reason, target_ate_point, target_ate_lower, target_ate_upper) =
        if !assumptions_supported {
            (
                SensitivityIdentificationStatus::NotIdentified,
                SensitivityIdentificationReason::RequiredAssumptionNotSupported,
                None,
                None,
                None,
            )
        } else {
            let lower = supported_lower - unsupported_target_mass;
            let upper = supported_upper + unsupported_target_mass;
            if !lower.is_finite() || !upper.is_finite() || lower < -1.0 || upper > 1.0 || lower > upper {
                return Err(OutcomeShiftSensitivityError::NonFiniteResult);
            }
            if unsupported_target_population_count > 0 {
                (
                    SensitivityIdentificationStatus::PartiallyIdentified,
                    SensitivityIdentificationReason::UnsupportedTargetModifierMass,
                    None,
                    Some(lower),
                    Some(upper),
                )
            } else if lambda == 1.0 {
                let point = lower + (upper - lower) / 2.0;
                (
                    SensitivityIdentificationStatus::Identified,
                    SensitivityIdentificationReason::ConditionalInvarianceAtLambdaOne,
                    Some(point),
                    Some(point),
                    Some(point),
                )
            } else {
                (
                    SensitivityIdentificationStatus::SensitivityBounded,
                    SensitivityIdentificationReason::BoundedOutcomeDistributionShift,
                    None,
                    Some(lower),
                    Some(upper),
                )
            }
        };

    let sensitivity_interval_width =
        target_ate_lower.zip(target_ate_upper).map(|(lower, upper)| upper - lower);
    if !source_ate.is_finite()
        || !supported_target_mass.is_finite()
        || !unsupported_target_mass.is_finite()
        || sensitivity_interval_width.is_some_and(|width| !width.is_finite() || width < 0.0)
        || target_ate_point.is_some_and(|value| !value.is_finite())
    {
        return Err(OutcomeShiftSensitivityError::NonFiniteResult);
    }

    Ok(CausalTransportSensitivityResult {
        estimand: "target_average_treatment_effect_risk_difference",
        likelihood_ratio_bound: lambda,
        sensitivity_scenario_digest: sensitivity_evidence.evidence_digest.clone(),
        identification_status,
        identification_reason,
        source_ate,
        target_ate_point,
        target_ate_lower,
        target_ate_upper,
        sensitivity_interval_width,
        source_population_total: source_total,
        target_population_total: target_total,
        supported_target_population_count,
        unsupported_target_population_count,
        supported_target_mass,
        unsupported_target_mass,
        assumptions: input.assumptions.clone(),
        sensitivity_evidence: sensitivity_evidence.clone(),
        source_data_digest: input.source_data_digest.clone(),
        target_population_digest: input.target_population_digest.clone(),
        protocol_digest: input.protocol_digest.clone(),
        strata,
    })
}

/// Find the first Lambda at which the target identified set includes zero.
/// Uses deterministic bisection over the actual bound function, not a grid.
pub fn find_null_tipping_point(
    input: &CausalTransportInput,
    sensitivity_evidence: &AssumptionEvidence,
    maximum_lambda: f64,
    tolerance: f64,
) -> Result<Option<LambdaTippingPoint>, OutcomeShiftSensitivityError> {
    if !maximum_lambda.is_finite()
        || maximum_lambda < 1.0
        || !tolerance.is_finite()
        || tolerance <= 0.0
        || tolerance >= maximum_lambda
    {
        return Err(OutcomeShiftSensitivityError::InvalidTippingSearch);
    }
    let at_one = assess_bernoulli_outcome_shift_transport(input, 1.0, sensitivity_evidence)?;
    let (Some(initial_lower), Some(initial_upper)) =
        (at_one.target_ate_lower, at_one.target_ate_upper)
    else {
        return Ok(None);
    };
    if at_one.identification_status == SensitivityIdentificationStatus::NotIdentified {
        return Ok(None);
    }
    if initial_lower <= 0.0 && initial_upper >= 0.0 {
        return Ok(Some(LambdaTippingPoint {
            lambda: 1.0,
            lower_bracket: 1.0,
            upper_bracket: 1.0,
            tolerance,
        }));
    }

    let crossing_lower = initial_lower > 0.0;
    let at_max =
        assess_bernoulli_outcome_shift_transport(input, maximum_lambda, sensitivity_evidence)?;
    let (Some(max_lower), Some(max_upper)) = (at_max.target_ate_lower, at_max.target_ate_upper)
    else {
        return Ok(None);
    };
    if !(max_lower <= 0.0 && max_upper >= 0.0) {
        return Ok(None);
    }

    let mut low = 1.0;
    let mut high = maximum_lambda;
    while high - low > tolerance {
        let middle = low + (high - low) / 2.0;
        let candidate =
            assess_bernoulli_outcome_shift_transport(input, middle, sensitivity_evidence)?;
        let (Some(lower), Some(upper)) = (candidate.target_ate_lower, candidate.target_ate_upper)
        else {
            return Ok(None);
        };
        let crossed = if crossing_lower {
            lower <= 0.0
        } else {
            upper >= 0.0
        };
        if crossed || (lower <= 0.0 && upper >= 0.0) {
            high = middle;
        } else {
            low = middle;
        }
    }
    Ok(Some(LambdaTippingPoint {
        lambda: high,
        lower_bracket: low,
        upper_bracket: high,
        tolerance,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    const SOURCE_DIGEST: &str = "sha256-source-rct";
    const TARGET_DIGEST: &str = "sha256-target-population";
    const PROTOCOL_DIGEST: &str = "sha256-transport-protocol";

    fn assumptions() -> CausalTransportAssumptions {
        let evidence = |disposition| AssumptionEvidence {
            disposition,
            evidence_digest: "sha256-assumption-evidence".to_string(),
        };
        CausalTransportAssumptions {
            source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
            selection_exchangeability: evidence(AssumptionDisposition::Unknown),
            no_interference: evidence(AssumptionDisposition::SupportedByDesign),
            treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
        }
    }

    fn sensitivity_evidence() -> AssumptionEvidence {
        AssumptionEvidence {
            disposition: AssumptionDisposition::SupportedByEvidence,
            evidence_digest: "sha256-predeclared-bernoulli-lambda-scenario".to_string(),
        }
    }

    fn source_strata() -> Vec<SourceEffectStratum> {
        vec![
            SourceEffectStratum {
                modifier_id: "low-infrastructure".to_string(),
                source_population_count: 80,
                control_sample_size: 40,
                control_events: 4,
                treatment_sample_size: 40,
                treatment_events: 12,
            },
            SourceEffectStratum {
                modifier_id: "high-infrastructure".to_string(),
                source_population_count: 20,
                control_sample_size: 10,
                control_events: 5,
                treatment_sample_size: 10,
                treatment_events: 8,
            },
        ]
    }

    fn input(target_strata: Vec<TargetPopulationStratum>) -> CausalTransportInput {
        CausalTransportInput {
            source_data_digest: SOURCE_DIGEST.to_string(),
            target_population_digest: TARGET_DIGEST.to_string(),
            protocol_digest: PROTOCOL_DIGEST.to_string(),
            assumptions: assumptions(),
            source_strata: source_strata(),
            target_strata,
        }
    }

    fn full_support_target() -> Vec<TargetPopulationStratum> {
        vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 80,
            },
        ]
    }

    #[test]
    fn bernoulli_risk_bounds_match_closed_form_and_preserve_structural_zeros() {
        assert_eq!(bernoulli_outcome_shift_bounds(0.0, 8.0).unwrap(), (0.0, 0.0));
        assert_eq!(bernoulli_outcome_shift_bounds(1.0, 8.0).unwrap(), (1.0, 1.0));
        assert_eq!(bernoulli_outcome_shift_bounds(0.3, 1.0).unwrap(), (0.3, 0.3));
        assert_eq!(bernoulli_outcome_shift_bounds(0.1, 2.0).unwrap(), (0.05, 0.2));
        assert_eq!(bernoulli_outcome_shift_bounds(0.3, 2.0).unwrap(), (0.15, 0.6));
        assert_eq!(bernoulli_outcome_shift_bounds(0.5, 2.0).unwrap(), (0.25, 0.75));
        assert_eq!(bernoulli_outcome_shift_bounds(0.8, 2.0).unwrap(), (0.6, 0.9));
        assert_eq!(
            bernoulli_outcome_shift_bounds(0.2, f64::INFINITY).unwrap_err(),
            OutcomeShiftSensitivityError::InvalidLambda
        );
        assert_eq!(
            bernoulli_outcome_shift_bounds(0.2, f64::NAN).unwrap_err(),
            OutcomeShiftSensitivityError::InvalidLambda
        );
        assert_eq!(
            bernoulli_outcome_shift_bounds(0.2, 0.9).unwrap_err(),
            OutcomeShiftSensitivityError::InvalidLambda
        );
    }

    #[test]
    fn lambda_one_reproduces_target_standardization() {
        let result = assess_bernoulli_outcome_shift_transport(
            &input(full_support_target()),
            1.0,
            &sensitivity_evidence(),
        )
        .unwrap();
        assert_eq!(result.identification_status, SensitivityIdentificationStatus::Identified);
        assert!((result.target_ate_point.unwrap() - 0.28).abs() < 1e-12);
        assert!((result.target_ate_lower.unwrap() - 0.28).abs() < 1e-12);
        assert!((result.target_ate_upper.unwrap() - 0.28).abs() < 1e-12);
        assert_eq!(result.sensitivity_interval_width, Some(0.0));
    }

    #[test]
    fn lambda_two_matches_known_target_effect_bounds() {
        let result = assess_bernoulli_outcome_shift_transport(
            &input(full_support_target()),
            2.0,
            &sensitivity_evidence(),
        )
        .unwrap();
        assert_eq!(
            result.identification_status,
            SensitivityIdentificationStatus::SensitivityBounded
        );
        assert_eq!(result.target_ate_point, None);
        assert!((result.target_ate_lower.unwrap() - (-0.13)).abs() < 1e-12);
        assert!((result.target_ate_upper.unwrap() - 0.63).abs() < 1e-12);
        assert!((result.sensitivity_interval_width.unwrap() - 0.76).abs() < 1e-12);
    }

    #[test]
    fn intervals_widen_monotonically_over_frozen_lambda_grid() {
        let candidate = input(full_support_target());
        let grid = [1.0, 1.1, 1.25, 1.5, 2.0, 4.0, 8.0];
        let mut previous: Option<(f64, f64)> = None;
        for lambda in grid {
            let result = assess_bernoulli_outcome_shift_transport(
                &candidate,
                lambda,
                &sensitivity_evidence(),
            )
            .unwrap();
            let current = (result.target_ate_lower.unwrap(), result.target_ate_upper.unwrap());
            if let Some((previous_lower, previous_upper)) = previous {
                assert!(current.0 <= previous_lower + 1e-12);
                assert!(current.1 >= previous_upper - 1e-12);
            }
            previous = Some(current);
        }
    }

    #[test]
    fn unsupported_target_mass_keeps_full_worst_case_interval() {
        let candidate = input(vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 60,
            },
            TargetPopulationStratum {
                modifier_id: "remote-isolated".to_string(),
                target_population_count: 20,
            },
        ]);
        let lambda_one = assess_bernoulli_outcome_shift_transport(
            &candidate,
            1.0,
            &sensitivity_evidence(),
        )
        .unwrap();
        assert!((lambda_one.target_ate_lower.unwrap() - 0.02).abs() < 1e-12);
        assert!((lambda_one.target_ate_upper.unwrap() - 0.42).abs() < 1e-12);

        let lambda_two = assess_bernoulli_outcome_shift_transport(
            &candidate,
            2.0,
            &sensitivity_evidence(),
        )
        .unwrap();
        assert_eq!(
            lambda_two.identification_status,
            SensitivityIdentificationStatus::PartiallyIdentified
        );
        assert!((lambda_two.target_ate_lower.unwrap() - (-0.30)).abs() < 1e-12);
        assert!((lambda_two.target_ate_upper.unwrap() - 0.70).abs() < 1e-12);
        assert_eq!(lambda_two.unsupported_target_population_count, 20);
        let unsupported = lambda_two
            .strata
            .iter()
            .find(|row| row.modifier_id == "remote-isolated")
            .unwrap();
        assert!(unsupported.control.is_none());
        assert!(unsupported.treatment.is_none());
        assert_eq!(unsupported.risk_difference_lower, None);
    }

    #[test]
    fn conditional_invariance_is_replaced_but_other_assumptions_still_gate() {
        let mut candidate = input(full_support_target());
        candidate.assumptions.selection_exchangeability.disposition =
            AssumptionDisposition::Violated;
        let result = assess_bernoulli_outcome_shift_transport(
            &candidate,
            2.0,
            &sensitivity_evidence(),
        )
        .unwrap();
        assert_eq!(
            result.identification_status,
            SensitivityIdentificationStatus::SensitivityBounded
        );

        candidate.assumptions.no_interference.disposition = AssumptionDisposition::Unknown;
        let blocked = assess_bernoulli_outcome_shift_transport(
            &candidate,
            2.0,
            &sensitivity_evidence(),
        )
        .unwrap();
        assert_eq!(
            blocked.identification_status,
            SensitivityIdentificationStatus::NotIdentified
        );
        assert_eq!(blocked.target_ate_lower, None);
        assert_eq!(blocked.target_ate_upper, None);
    }

    #[test]
    fn unsupported_sensitivity_envelope_and_missing_evidence_block_bounds() {
        let candidate = input(full_support_target());
        let mut unsupported = sensitivity_evidence();
        unsupported.disposition = AssumptionDisposition::Unknown;
        assert_eq!(
            assess_bernoulli_outcome_shift_transport(&candidate, 2.0, &unsupported)
                .unwrap_err(),
            OutcomeShiftSensitivityError::SensitivityEnvelopeNotSupported
        );
        let mut missing = sensitivity_evidence();
        missing.evidence_digest.clear();
        assert_eq!(
            assess_bernoulli_outcome_shift_transport(&candidate, 2.0, &missing).unwrap_err(),
            OutcomeShiftSensitivityError::MissingSensitivityScenarioDigest
        );
    }

    #[test]
    fn tipping_point_is_found_by_bracketed_search() {
        let tipping = find_null_tipping_point(
            &input(full_support_target()),
            &sensitivity_evidence(),
            4.0,
            1e-8,
        )
        .unwrap()
        .expect("null is reached within Lambda <= 4");
        let expected = (23.0_f64 / 9.0).sqrt();
        assert!((tipping.lambda - expected).abs() < 2e-8);
        assert!(tipping.lower_bracket <= expected);
        assert!(tipping.upper_bracket >= expected);
        assert!(tipping.upper_bracket - tipping.lower_bracket <= tipping.tolerance);
    }

    #[test]
    fn tipping_point_is_one_when_support_bounds_already_include_null() {
        let candidate = input(vec![
            TargetPopulationStratum {
                modifier_id: "low-infrastructure".to_string(),
                target_population_count: 20,
            },
            TargetPopulationStratum {
                modifier_id: "high-infrastructure".to_string(),
                target_population_count: 60,
            },
            TargetPopulationStratum {
                modifier_id: "remote-isolated".to_string(),
                target_population_count: 20,
            },
        ]);
        let tipping = find_null_tipping_point(
            &candidate,
            &sensitivity_evidence(),
            4.0,
            1e-8,
        )
        .unwrap()
        .expect("support bounds already contain zero");
        assert_eq!(tipping.lambda, 1.0);
    }

    #[test]
    fn invalid_tipping_search_fails_closed() {
        assert_eq!(
            find_null_tipping_point(
                &input(full_support_target()),
                &sensitivity_evidence(),
                0.9,
                1e-4,
            )
            .unwrap_err(),
            OutcomeShiftSensitivityError::InvalidTippingSearch
        );
        assert_eq!(
            find_null_tipping_point(
                &input(full_support_target()),
                &sensitivity_evidence(),
                2.0,
                0.0,
            )
            .unwrap_err(),
            OutcomeShiftSensitivityError::InvalidTippingSearch
        );
    }
}
