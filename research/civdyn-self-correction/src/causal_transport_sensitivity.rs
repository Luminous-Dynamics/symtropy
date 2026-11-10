// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Bernoulli specialization of sharp likelihood-ratio outcome-shift bounds
//! for transporting a risk difference from a randomized source trial.
//!
//! The conditional target/source likelihood ratio for each binary outcome is
//! bounded by [1/Lambda, Lambda] and normalized to a valid Bernoulli
//! distribution. This produces sharp bounds on each target conditional risk.
//! Target-only modifier strata remain separately partially identified in
//! [-1, 1]; the source outcome-shift envelope is not applied where source
//! support is absent.
//!
//! This finite-stratum specialization does not implement the full continuous
//! outcome, covariate-density-weighted estimator or its sampling inference.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fmt;

use crate::causal_transport::{
    AssumptionDisposition, CausalTransportAssumptions, SourceEffectStratum,
    TargetPopulationStratum,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OutcomeShiftSensitivityError {
    MissingDataDigest,
    MissingAssumptionEvidenceDigest,
    AssumptionsNotSupported,
    EmptySourceStrata,
    EmptyTargetStrata,
    EmptyModifierId,
    DuplicateSourceModifier { modifier_id: String },
    DuplicateTargetModifier { modifier_id: String },
    InvalidSourceStratum { modifier_id: String },
    InvalidTargetStratum { modifier_id: String },
    CountOverflow,
    InvalidLambdaGrid,
    InvalidTippingSearch,
    NonFiniteResult,
    NonMonotoneFrontier,
}

impl fmt::Display for OutcomeShiftSensitivityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for OutcomeShiftSensitivityError {}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BernoulliRiskBounds {
    pub lower: f64,
    pub upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct StratumOutcomeShiftBound {
    pub modifier_id: String,
    pub target_population_count: u64,
    pub target_population_share: f64,
    pub source_support: bool,
    pub control_source_risk: Option<f64>,
    pub treatment_source_risk: Option<f64>,
    pub control_target_risk_lower: Option<f64>,
    pub control_target_risk_upper: Option<f64>,
    pub treatment_target_risk_lower: Option<f64>,
    pub treatment_target_risk_upper: Option<f64>,
    pub risk_difference_lower: f64,
    pub risk_difference_upper: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct LambdaSensitivityPoint {
    pub lambda: f64,
    pub target_ate_lower: f64,
    pub target_ate_upper: f64,
    pub interval_width: f64,
    pub supported_target_population_count: u64,
    pub unsupported_target_population_count: u64,
    pub supported_target_mass: f64,
    pub unsupported_target_mass: f64,
    pub strata: Vec<StratumOutcomeShiftBound>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NullTippingPoint {
    /// Upper endpoint of the final bisection bracket; its interval includes 0.
    pub lambda: f64,
    /// The previous lower endpoint does not yet include 0.
    pub lower_bracket: f64,
    pub upper_bracket: f64,
    pub search_max: f64,
    pub tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct OutcomeShiftSensitivityReport {
    pub estimand: &'static str,
    pub source_ate: f64,
    pub source_population_total: u64,
    pub target_population_total: u64,
    pub source_data_digest: String,
    pub target_population_digest: String,
    pub protocol_digest: String,
    pub assumptions: CausalTransportAssumptions,
    pub frontier: Vec<LambdaSensitivityPoint>,
    pub null_tipping_point: Option<NullTippingPoint>,
}

fn validate_assumptions(
    assumptions: &CausalTransportAssumptions,
) -> Result<(), OutcomeShiftSensitivityError> {
    let evidence = [
        &assumptions.source_internal_validity,
        &assumptions.selection_exchangeability,
        &assumptions.no_interference,
        &assumptions.treatment_version_invariance,
    ];
    if evidence.iter().any(|item| item.evidence_digest.trim().is_empty()) {
        return Err(OutcomeShiftSensitivityError::MissingAssumptionEvidenceDigest);
    }
    if evidence.iter().any(|item| {
        !matches!(
            item.disposition,
            AssumptionDisposition::SupportedByDesign
                | AssumptionDisposition::SupportedByEvidence
        )
    }) {
        return Err(OutcomeShiftSensitivityError::AssumptionsNotSupported);
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
    Ok((indexed, total))
}

/// Exact feasible target Bernoulli risk under a pointwise likelihood-ratio
/// envelope and its separate normalization constraint.
pub fn bernoulli_target_risk_bounds(
    source_risk: f64,
    lambda: f64,
) -> Result<BernoulliRiskBounds, OutcomeShiftSensitivityError> {
    if !source_risk.is_finite()
        || !(0.0..=1.0).contains(&source_risk)
        || !lambda.is_finite()
        || lambda < 1.0
    {
        return Err(OutcomeShiftSensitivityError::InvalidLambdaGrid);
    }

    let lower = (source_risk / lambda)
        .max(1.0 - (1.0 - source_risk) * lambda)
        .max(0.0);
    let upper = (source_risk * lambda)
        .min(1.0 - (1.0 - source_risk) / lambda)
        .min(1.0);
    if !lower.is_finite() || !upper.is_finite() || lower > upper + 1e-12 {
        return Err(OutcomeShiftSensitivityError::NonFiniteResult);
    }

    Ok(BernoulliRiskBounds {
        lower: lower.min(upper),
        upper: upper.max(lower),
    })
}

fn source_ate(
    source_strata: &[SourceEffectStratum],
    source_total: u64,
) -> f64 {
    source_strata
        .iter()
        .map(|stratum| {
            let control = stratum.control_events as f64 / stratum.control_sample_size as f64;
            let treatment = stratum.treatment_events as f64 / stratum.treatment_sample_size as f64;
            let share = stratum.source_population_count as f64 / source_total as f64;
            share * (treatment - control)
        })
        .sum()
}

fn evaluate_lambda(
    lambda: f64,
    source_by_modifier: &BTreeMap<&str, &SourceEffectStratum>,
    target_strata: &[TargetPopulationStratum],
    target_total: u64,
) -> Result<LambdaSensitivityPoint, OutcomeShiftSensitivityError> {
    let mut lower = 0.0;
    let mut upper = 0.0;
    let mut supported_count = 0_u64;
    let mut strata = Vec::with_capacity(target_strata.len());

    for target in target_strata {
        let share = target.target_population_count as f64 / target_total as f64;
        if let Some(source) = source_by_modifier.get(target.modifier_id.as_str()) {
            supported_count = supported_count
                .checked_add(target.target_population_count)
                .ok_or(OutcomeShiftSensitivityError::CountOverflow)?;
            let control_source = source.control_events as f64 / source.control_sample_size as f64;
            let treatment_source =
                source.treatment_events as f64 / source.treatment_sample_size as f64;
            let control_target = bernoulli_target_risk_bounds(control_source, lambda)?;
            let treatment_target = bernoulli_target_risk_bounds(treatment_source, lambda)?;
            let stratum_lower = treatment_target.lower - control_target.upper;
            let stratum_upper = treatment_target.upper - control_target.lower;
            lower += share * stratum_lower;
            upper += share * stratum_upper;
            strata.push(StratumOutcomeShiftBound {
                modifier_id: target.modifier_id.clone(),
                target_population_count: target.target_population_count,
                target_population_share: share,
                source_support: true,
                control_source_risk: Some(control_source),
                treatment_source_risk: Some(treatment_source),
                control_target_risk_lower: Some(control_target.lower),
                control_target_risk_upper: Some(control_target.upper),
                treatment_target_risk_lower: Some(treatment_target.lower),
                treatment_target_risk_upper: Some(treatment_target.upper),
                risk_difference_lower: stratum_lower,
                risk_difference_upper: stratum_upper,
            });
        } else {
            lower -= share;
            upper += share;
            strata.push(StratumOutcomeShiftBound {
                modifier_id: target.modifier_id.clone(),
                target_population_count: target.target_population_count,
                target_population_share: share,
                source_support: false,
                control_source_risk: None,
                treatment_source_risk: None,
                control_target_risk_lower: None,
                control_target_risk_upper: None,
                treatment_target_risk_lower: None,
                treatment_target_risk_upper: None,
                risk_difference_lower: -1.0,
                risk_difference_upper: 1.0,
            });
        }
    }

    let unsupported_count = target_total
        .checked_sub(supported_count)
        .ok_or(OutcomeShiftSensitivityError::CountOverflow)?;
    let supported_mass = supported_count as f64 / target_total as f64;
    let unsupported_mass = unsupported_count as f64 / target_total as f64;
    let width = upper - lower;
    if [
        lower,
        upper,
        width,
        supported_mass,
        unsupported_mass,
    ]
    .iter()
    .any(|value| !value.is_finite())
        || lower < -1.0 - 1e-12
        || upper > 1.0 + 1e-12
        || lower > upper + 1e-12
    {
        return Err(OutcomeShiftSensitivityError::NonFiniteResult);
    }

    Ok(LambdaSensitivityPoint {
        lambda,
        target_ate_lower: lower,
        target_ate_upper: upper,
        interval_width: width,
        supported_target_population_count: supported_count,
        unsupported_target_population_count: unsupported_count,
        supported_target_mass: supported_mass,
        unsupported_target_mass: unsupported_mass,
        strata,
    })
}

fn interval_contains_null(point: &LambdaSensitivityPoint) -> bool {
    point.target_ate_lower <= 0.0 && point.target_ate_upper >= 0.0
}

fn find_null_tipping_point(
    source_by_modifier: &BTreeMap<&str, &SourceEffectStratum>,
    target_strata: &[TargetPopulationStratum],
    target_total: u64,
    search_max: f64,
    tolerance: f64,
) -> Result<Option<NullTippingPoint>, OutcomeShiftSensitivityError> {
    let at_one = evaluate_lambda(1.0, source_by_modifier, target_strata, target_total)?;
    if interval_contains_null(&at_one) {
        return Ok(Some(NullTippingPoint {
            lambda: 1.0,
            lower_bracket: 1.0,
            upper_bracket: 1.0,
            search_max,
            tolerance,
        }));
    }

    let at_max = evaluate_lambda(search_max, source_by_modifier, target_strata, target_total)?;
    if !interval_contains_null(&at_max) {
        return Ok(None);
    }

    let mut lower = 1.0;
    let mut upper = search_max;
    for _ in 0..128 {
        if upper - lower <= tolerance {
            break;
        }
        let midpoint = lower + (upper - lower) / 2.0;
        let point = evaluate_lambda(midpoint, source_by_modifier, target_strata, target_total)?;
        if interval_contains_null(&point) {
            upper = midpoint;
        } else {
            lower = midpoint;
        }
    }

    Ok(Some(NullTippingPoint {
        lambda: upper,
        lower_bracket: lower,
        upper_bracket: upper,
        search_max,
        tolerance,
    }))
}

/// Compute the exact Bernoulli likelihood-ratio sensitivity frontier on a
/// fixed target modifier distribution.
///
/// Lambda values must be strictly increasing and begin at 1.0. The separate
/// tipping search uses continuous bisection up to the supplied maximum.
pub fn assess_binary_outcome_shift_sensitivity(
    source_data_digest: &str,
    target_population_digest: &str,
    protocol_digest: &str,
    assumptions: &CausalTransportAssumptions,
    source_strata: &[SourceEffectStratum],
    target_strata: &[TargetPopulationStratum],
    lambda_values: &[f64],
    tipping_search_max: f64,
    tipping_tolerance: f64,
) -> Result<OutcomeShiftSensitivityReport, OutcomeShiftSensitivityError> {
    if source_data_digest.trim().is_empty()
        || target_population_digest.trim().is_empty()
        || protocol_digest.trim().is_empty()
    {
        return Err(OutcomeShiftSensitivityError::MissingDataDigest);
    }
    validate_assumptions(assumptions)?;

    if lambda_values.is_empty()
        || lambda_values[0] != 1.0
        || lambda_values.iter().any(|value| !value.is_finite() || *value < 1.0)
        || lambda_values.windows(2).any(|pair| pair[0] >= pair[1])
    {
        return Err(OutcomeShiftSensitivityError::InvalidLambdaGrid);
    }
    if !tipping_search_max.is_finite()
        || tipping_search_max < 1.0
        || tipping_search_max < *lambda_values.last().unwrap_or(&1.0)
        || !tipping_tolerance.is_finite()
        || tipping_tolerance <= 0.0
        || tipping_tolerance >= tipping_search_max.max(1.0)
    {
        return Err(OutcomeShiftSensitivityError::InvalidTippingSearch);
    }

    let (source_by_modifier, source_total) = validate_source_strata(source_strata)?;
    let (_target_by_modifier, target_total) = validate_target_strata(target_strata)?;
    let mut canonical_target_strata = target_strata.to_vec();
    canonical_target_strata.sort_by(|left, right| left.modifier_id.cmp(&right.modifier_id));

    let source_ate_value = source_ate(source_strata, source_total);
    if !source_ate_value.is_finite() {
        return Err(OutcomeShiftSensitivityError::NonFiniteResult);
    }

    let mut frontier = Vec::with_capacity(lambda_values.len());
    for &lambda in lambda_values {
        let point = evaluate_lambda(
            lambda,
            &source_by_modifier,
            &canonical_target_strata,
            target_total,
        )?;
        if let Some(previous) = frontier.last() {
            let previous: &LambdaSensitivityPoint = previous;
            if point.target_ate_lower > previous.target_ate_lower + 1e-12
                || point.target_ate_upper + 1e-12 < previous.target_ate_upper
            {
                return Err(OutcomeShiftSensitivityError::NonMonotoneFrontier);
            }
        }
        frontier.push(point);
    }

    let null_tipping_point = find_null_tipping_point(
        &source_by_modifier,
        &canonical_target_strata,
        target_total,
        tipping_search_max,
        tipping_tolerance,
    )?;

    Ok(OutcomeShiftSensitivityReport {
        estimand: "target_average_treatment_effect_risk_difference",
        source_ate: source_ate_value,
        source_population_total: source_total,
        target_population_total: target_total,
        source_data_digest: source_data_digest.to_string(),
        target_population_digest: target_population_digest.to_string(),
        protocol_digest: protocol_digest.to_string(),
        assumptions: assumptions.clone(),
        frontier,
        null_tipping_point,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn supported_assumptions() -> CausalTransportAssumptions {
        let evidence = |disposition| crate::causal_transport::AssumptionEvidence {
            disposition,
            evidence_digest: "sha256-sensitivity-assumption-review".to_string(),
        };
        CausalTransportAssumptions {
            source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
            selection_exchangeability: evidence(AssumptionDisposition::SupportedByEvidence),
            no_interference: evidence(AssumptionDisposition::SupportedByDesign),
            treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
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

    fn report(target: Vec<TargetPopulationStratum>) -> OutcomeShiftSensitivityReport {
        assess_binary_outcome_shift_sensitivity(
            "sha256-sensitivity-source-rct",
            "sha256-sensitivity-target-population",
            "sha256-sensitivity-protocol-v0.1",
            &supported_assumptions(),
            &source_strata(),
            &target,
            &[1.0, 1.25, 1.5, 2.0, 4.0],
            16.0,
            1e-8,
        )
        .expect("valid binary outcome-shift frontier")
    }

    #[test]
    fn bernoulli_likelihood_ratio_bounds_reduce_to_source_risk_at_lambda_one() {
        for source_risk in [0.0, 0.1, 0.3, 0.5, 0.8, 1.0] {
            let bounds = bernoulli_target_risk_bounds(source_risk, 1.0)
                .expect("valid Lambda=1 risk bounds");
            assert_eq!(bounds.lower, source_risk);
            assert_eq!(bounds.upper, source_risk);
        }
    }

    #[test]
    fn zero_and_one_source_risks_are_invariant_under_finite_likelihood_ratio_bounds() {
        for lambda in [1.0, 1.25, 2.0, 8.0, 16.0] {
            let zero = bernoulli_target_risk_bounds(0.0, lambda).expect("zero risk");
            let one = bernoulli_target_risk_bounds(1.0, lambda).expect("one risk");
            assert_eq!(zero.lower, 0.0);
            assert_eq!(zero.upper, 0.0);
            assert_eq!(one.lower, 1.0);
            assert_eq!(one.upper, 1.0);
        }
    }

    #[test]
    fn lambda_one_reproduces_standardized_target_ate_and_frontier_widens() {
        let report = report(full_support_target());
        let first = &report.frontier[0];
        assert_eq!(first.lambda, 1.0);
        assert!((first.target_ate_lower - 0.28).abs() < 1e-12);
        assert!((first.target_ate_upper - 0.28).abs() < 1e-12);
        assert_eq!(first.interval_width, 0.0);

        for pair in report.frontier.windows(2) {
            assert!(pair[1].target_ate_lower <= pair[0].target_ate_lower + 1e-12);
            assert!(pair[1].target_ate_upper + 1e-12 >= pair[0].target_ate_upper);
            assert!(pair[1].interval_width + 1e-12 >= pair[0].interval_width);
        }
    }

    #[test]
    fn lambda_one_with_target_support_failure_reproduces_worst_case_bounds() {
        let target = vec![
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
        ];
        let report = report(target);
        let first = &report.frontier[0];
        assert!((first.target_ate_lower - 0.02).abs() < 1e-12);
        assert!((first.target_ate_upper - 0.42).abs() < 1e-12);
        assert_eq!(first.supported_target_population_count, 80);
        assert_eq!(first.unsupported_target_population_count, 20);
    }

    #[test]
    fn unsupported_target_stratum_stays_at_worst_case_for_every_lambda() {
        let target = vec![
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
        ];
        let report = report(target);
        for point in &report.frontier {
            let unsupported = point
                .strata
                .iter()
                .find(|stratum| stratum.modifier_id == "remote-isolated")
                .expect("unsupported stratum");
            assert!(!unsupported.source_support);
            assert_eq!(unsupported.risk_difference_lower, -1.0);
            assert_eq!(unsupported.risk_difference_upper, 1.0);
            assert_eq!(unsupported.control_target_risk_lower, None);
            assert_eq!(unsupported.treatment_target_risk_upper, None);
        }
    }

    #[test]
    fn computes_continuous_null_tipping_point_and_auditable_bracket() {
        let report = report(full_support_target());
        let tipping = report.null_tipping_point.expect("a finite tipping point");
        assert!(tipping.lambda > 1.0);
        assert!(tipping.lambda <= tipping.search_max);
        assert!(tipping.upper_bracket - tipping.lower_bracket <= tipping.tolerance + 1e-12);
        let (source_by_modifier, _) = validate_source_strata(&source_strata()).unwrap();
        let lower_point = evaluate_lambda(
            tipping.lower_bracket,
            &source_by_modifier,
            &full_support_target(),
            100,
        )
        .expect("lower endpoint");
        let upper_point = evaluate_lambda(
            tipping.upper_bracket,
            &source_by_modifier,
            &full_support_target(),
            100,
        )
        .expect("upper endpoint");
        assert!(!interval_contains_null(&lower_point));
        assert!(interval_contains_null(&upper_point));
    }

    #[test]
    fn no_tipping_point_is_reported_when_the_effect_survives_the_search_horizon() {
        let source = vec![SourceEffectStratum {
            modifier_id: "only".to_string(),
            source_population_count: 1,
            control_sample_size: 100,
            control_events: 0,
            treatment_sample_size: 100,
            treatment_events: 100,
        }];
        let target = vec![TargetPopulationStratum {
            modifier_id: "only".to_string(),
            target_population_count: 1,
        }];
        let report = assess_binary_outcome_shift_sensitivity(
            "source",
            "target",
            "protocol",
            &supported_assumptions(),
            &source,
            &target,
            &[1.0, 2.0, 4.0],
            4.0,
            1e-8,
        )
        .expect("valid no-tipping fixture");
        assert_eq!(report.null_tipping_point, None);
    }

    #[test]
    fn unsupported_assumptions_and_invalid_sensitivity_inputs_fail_closed() {
        let mut assumptions = supported_assumptions();
        assumptions.no_interference.disposition = AssumptionDisposition::Unknown;
        assert_eq!(
            assess_binary_outcome_shift_sensitivity(
                "source",
                "target",
                "protocol",
                &assumptions,
                &source_strata(),
                &full_support_target(),
                &[1.0, 2.0],
                4.0,
                1e-8,
            )
            .unwrap_err(),
            OutcomeShiftSensitivityError::AssumptionsNotSupported
        );

        assert_eq!(
            assess_binary_outcome_shift_sensitivity(
                "source",
                "target",
                "protocol",
                &supported_assumptions(),
                &source_strata(),
                &full_support_target(),
                &[1.5, 2.0],
                4.0,
                1e-8,
            )
            .unwrap_err(),
            OutcomeShiftSensitivityError::InvalidLambdaGrid
        );
        assert_eq!(
            bernoulli_target_risk_bounds(0.5, 0.5).unwrap_err(),
            OutcomeShiftSensitivityError::InvalidLambdaGrid
        );
        assert_eq!(
            assess_binary_outcome_shift_sensitivity(
                "source",
                "target",
                "protocol",
                &supported_assumptions(),
                &source_strata(),
                &full_support_target(),
                &[1.0, 4.0],
                2.0,
                1e-8,
            )
            .unwrap_err(),
            OutcomeShiftSensitivityError::InvalidTippingSearch
        );
    }

    #[test]
    fn duplicate_modifiers_invalid_counts_and_overflow_are_rejected() {
        let mut duplicate_source = source_strata();
        duplicate_source.push(duplicate_source[0].clone());
        assert!(matches!(
            assess_binary_outcome_shift_sensitivity(
                "source",
                "target",
                "protocol",
                &supported_assumptions(),
                &duplicate_source,
                &full_support_target(),
                &[1.0, 2.0],
                4.0,
                1e-8,
            ),
            Err(OutcomeShiftSensitivityError::DuplicateSourceModifier { .. })
        ));

        let mut invalid = source_strata();
        invalid[0].control_events = invalid[0].control_sample_size + 1;
        assert!(matches!(
            assess_binary_outcome_shift_sensitivity(
                "source",
                "target",
                "protocol",
                &supported_assumptions(),
                &invalid,
                &full_support_target(),
                &[1.0, 2.0],
                4.0,
                1e-8,
            ),
            Err(OutcomeShiftSensitivityError::InvalidSourceStratum { .. })
        ));

        let mut overflow = full_support_target();
        overflow[0].target_population_count = u64::MAX - 1;
        overflow[1].target_population_count = 2;
        assert_eq!(
            assess_binary_outcome_shift_sensitivity(
                "source",
                "target",
                "protocol",
                &supported_assumptions(),
                &source_strata(),
                &overflow,
                &[1.0, 2.0],
                4.0,
                1e-8,
            )
            .unwrap_err(),
            OutcomeShiftSensitivityError::CountOverflow
        );
    }
}
