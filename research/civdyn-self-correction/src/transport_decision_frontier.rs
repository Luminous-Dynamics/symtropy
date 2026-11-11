// Copyright (C) 2024-2026 Tristan Stoltz / Luminous Dynamics
// SPDX-License-Identifier: AGPL-3.0-or-later

//! Decision stability under a predeclared target-effect threshold.
//!
//! This module keeps the causal effect sensitivity interval and the policy
//! threshold separate. It does not infer preferences, monetize outcomes or
//! replace multi-objective policy feasibility analysis.

#![forbid(unsafe_code)]

use std::fmt;

use crate::causal_transport::AssumptionDisposition;
use crate::causal_transport_sensitivity::{
    CausalTransportSensitivityResult, SensitivityIdentificationStatus,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TargetDecision {
    RecommendTreatment,
    RecommendAgainstTreatment,
    Indeterminate,
    NotIdentified,
}

impl TargetDecision {
    pub const fn name(self) -> &'static str {
        match self {
            Self::RecommendTreatment => "recommend_treatment",
            Self::RecommendAgainstTreatment => "recommend_against_treatment",
            Self::Indeterminate => "indeterminate",
            Self::NotIdentified => "not_identified",
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DecisionFrontierStatus {
    TippingPointFound,
    NotRobustAtBaseline,
    NoTippingPointWithinHorizon,
    NotIdentified,
}

impl DecisionFrontierStatus {
    pub const fn name(self) -> &'static str {
        match self {
            Self::TippingPointFound => "tipping_point_found",
            Self::NotRobustAtBaseline => "not_robust_at_baseline",
            Self::NoTippingPointWithinHorizon => "no_tipping_point_within_horizon",
            Self::NotIdentified => "not_identified",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DecisionThresholdEvidence {
    /// Minimum worthwhile target risk difference, in the estimand's native unit.
    pub threshold: String,
    pub effect_unit: String,
    pub rationale: String,
    pub source_digest: String,
    pub disposition: AssumptionDisposition,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionClassification {
    pub decision: TargetDecision,
    pub likelihood_ratio_bound: f64,
    pub threshold: f64,
    pub effect_unit: String,
    pub lower_effect_bound: Option<f64>,
    pub upper_effect_bound: Option<f64>,
    pub interval_width: Option<f64>,
    pub threshold_source_digest: String,
    pub threshold_rationale: String,
    pub sensitivity_scenario_digest: String,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct DecisionTippingPoint {
    /// Upper end of the final bracket: first point where the recommendation
    /// no longer clears the threshold.
    pub lambda: f64,
    pub lower_bracket: f64,
    pub upper_bracket: f64,
    pub tolerance: f64,
}

#[derive(Debug, Clone, PartialEq)]
pub struct DecisionFrontierReport {
    pub status: DecisionFrontierStatus,
    pub baseline_decision: TargetDecision,
    pub decision_at_horizon: TargetDecision,
    pub likelihood_ratio_horizon: f64,
    pub threshold: f64,
    pub effect_unit: String,
    pub tipping_point: Option<DecisionTippingPoint>,
    pub threshold_source_digest: String,
    pub threshold_rationale: String,
    pub sensitivity_scenario_digest: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecisionFrontierError {
    InvalidThreshold,
    InvalidEffectUnit,
    MissingThresholdRationale,
    MissingThresholdSourceDigest,
    UnsupportedThresholdEvidence,
    InvalidSearchHorizon,
    SensitivityBoundsMissing,
    SensitivityScenarioDigestMissing,
}

impl fmt::Display for DecisionFrontierError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{self:?}")
    }
}

impl std::error::Error for DecisionFrontierError {}

fn validate_threshold(
    evidence: &DecisionThresholdEvidence,
) -> Result<f64, DecisionFrontierError> {
    if evidence.effect_unit.trim().is_empty() {
        return Err(DecisionFrontierError::InvalidEffectUnit);
    }
    if evidence.rationale.trim().is_empty() {
        return Err(DecisionFrontierError::MissingThresholdRationale);
    }
    if evidence.source_digest.trim().is_empty() {
        return Err(DecisionFrontierError::MissingThresholdSourceDigest);
    }
    if !matches!(
        evidence.disposition,
        AssumptionDisposition::SupportedByDesign | AssumptionDisposition::SupportedByEvidence
    ) {
        return Err(DecisionFrontierError::UnsupportedThresholdEvidence);
    }
    let threshold = evidence
        .threshold
        .parse::<f64>()
        .map_err(|_| DecisionFrontierError::InvalidThreshold)?;
    if !threshold.is_finite() || !(-1.0..=1.0).contains(&threshold) {
        return Err(DecisionFrontierError::InvalidThreshold);
    }
    Ok(threshold)
}

pub fn classify_target_effect_decision(
    sensitivity: &CausalTransportSensitivityResult,
    threshold_evidence: &DecisionThresholdEvidence,
) -> Result<DecisionClassification, DecisionFrontierError> {
    let threshold = validate_threshold(threshold_evidence)?;
    if sensitivity.sensitivity_scenario_digest.trim().is_empty() {
        return Err(DecisionFrontierError::SensitivityScenarioDigestMissing);
    }

    let (decision, lower, upper) =
        if sensitivity.identification_status == SensitivityIdentificationStatus::NotIdentified {
            (TargetDecision::NotIdentified, None, None)
        } else {
            let lower = sensitivity
                .target_ate_lower
                .ok_or(DecisionFrontierError::SensitivityBoundsMissing)?;
            let upper = sensitivity
                .target_ate_upper
                .ok_or(DecisionFrontierError::SensitivityBoundsMissing)?;
            if !lower.is_finite() || !upper.is_finite() || lower > upper {
                return Err(DecisionFrontierError::SensitivityBoundsMissing);
            }
            let decision = if lower >= threshold {
                TargetDecision::RecommendTreatment
            } else if upper < threshold {
                TargetDecision::RecommendAgainstTreatment
            } else {
                TargetDecision::Indeterminate
            };
            (decision, Some(lower), Some(upper))
        };

    Ok(DecisionClassification {
        decision,
        likelihood_ratio_bound: sensitivity.likelihood_ratio_bound,
        threshold,
        effect_unit: threshold_evidence.effect_unit.clone(),
        lower_effect_bound: lower,
        upper_effect_bound: upper,
        interval_width: lower.zip(upper).map(|(lo, hi)| hi - lo),
        threshold_source_digest: threshold_evidence.source_digest.clone(),
        threshold_rationale: threshold_evidence.rationale.clone(),
        sensitivity_scenario_digest: sensitivity.sensitivity_scenario_digest.clone(),
    })
}

/// Find the first Lambda where a baseline robust positive recommendation
/// ceases to satisfy the minimum-benefit threshold. A result that is already
/// Indeterminate or negative at Lambda=1 is reported as NotRobustAtBaseline.
pub fn find_recommendation_tipping_point<F>(
    mut evaluate: F,
    threshold_evidence: &DecisionThresholdEvidence,
    maximum_lambda: f64,
    tolerance: f64,
) -> Result<DecisionFrontierReport, DecisionFrontierError>
where
    F: FnMut(f64) -> Result<CausalTransportSensitivityResult, DecisionFrontierError>,
{
    let threshold = validate_threshold(threshold_evidence)?;
    if !maximum_lambda.is_finite()
        || maximum_lambda < 1.0
        || !tolerance.is_finite()
        || tolerance <= 0.0
        || tolerance >= maximum_lambda
    {
        return Err(DecisionFrontierError::InvalidSearchHorizon);
    }

    let at_one = evaluate(1.0)?;
    let baseline = classify_target_effect_decision(&at_one, threshold_evidence)?;
    if baseline.decision == TargetDecision::NotIdentified {
        return Ok(DecisionFrontierReport {
            status: DecisionFrontierStatus::NotIdentified,
            baseline_decision: baseline.decision,
            decision_at_horizon: TargetDecision::NotIdentified,
            likelihood_ratio_horizon: maximum_lambda,
            threshold,
            effect_unit: threshold_evidence.effect_unit.clone(),
            tipping_point: None,
            threshold_source_digest: threshold_evidence.source_digest.clone(),
            threshold_rationale: threshold_evidence.rationale.clone(),
            sensitivity_scenario_digest: at_one.sensitivity_scenario_digest,
        });
    }

    let at_horizon = evaluate(maximum_lambda)?;
    let horizon_classification =
        classify_target_effect_decision(&at_horizon, threshold_evidence)?;
    if baseline.decision != TargetDecision::RecommendTreatment {
        return Ok(DecisionFrontierReport {
            status: DecisionFrontierStatus::NotRobustAtBaseline,
            baseline_decision: baseline.decision,
            decision_at_horizon: horizon_classification.decision,
            likelihood_ratio_horizon: maximum_lambda,
            threshold,
            effect_unit: threshold_evidence.effect_unit.clone(),
            tipping_point: None,
            threshold_source_digest: threshold_evidence.source_digest.clone(),
            threshold_rationale: threshold_evidence.rationale.clone(),
            sensitivity_scenario_digest: at_one.sensitivity_scenario_digest,
        });
    }

    if horizon_classification.decision == TargetDecision::RecommendTreatment {
        return Ok(DecisionFrontierReport {
            status: DecisionFrontierStatus::NoTippingPointWithinHorizon,
            baseline_decision: baseline.decision,
            decision_at_horizon: horizon_classification.decision,
            likelihood_ratio_horizon: maximum_lambda,
            threshold,
            effect_unit: threshold_evidence.effect_unit.clone(),
            tipping_point: None,
            threshold_source_digest: threshold_evidence.source_digest.clone(),
            threshold_rationale: threshold_evidence.rationale.clone(),
            sensitivity_scenario_digest: at_one.sensitivity_scenario_digest,
        });
    }

    let mut low = 1.0;
    let mut high = maximum_lambda;
    while high - low > tolerance {
        let middle = low + (high - low) / 2.0;
        let candidate = evaluate(middle)?;
        let classification = classify_target_effect_decision(&candidate, threshold_evidence)?;
        if classification.decision == TargetDecision::RecommendTreatment {
            low = middle;
        } else {
            high = middle;
        }
    }

    Ok(DecisionFrontierReport {
        status: DecisionFrontierStatus::TippingPointFound,
        baseline_decision: baseline.decision,
        decision_at_horizon: horizon_classification.decision,
        likelihood_ratio_horizon: maximum_lambda,
        threshold,
        effect_unit: threshold_evidence.effect_unit.clone(),
        tipping_point: Some(DecisionTippingPoint {
            lambda: high,
            lower_bracket: low,
            upper_bracket: high,
            tolerance,
        }),
        threshold_source_digest: threshold_evidence.source_digest.clone(),
        threshold_rationale: threshold_evidence.rationale.clone(),
        sensitivity_scenario_digest: at_one.sensitivity_scenario_digest,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::causal_transport::{
        AssumptionEvidence, CausalTransportAssumptions, CausalTransportInput, SourceEffectStratum,
        TargetPopulationStratum,
    };
    use crate::causal_transport_sensitivity::assess_bernoulli_outcome_shift_transport;

    fn input() -> CausalTransportInput {
        let evidence = |disposition| AssumptionEvidence {
            disposition,
            evidence_digest: "sha256-assumption".to_string(),
        };
        CausalTransportInput {
            source_data_digest: "sha256-source".to_string(),
            target_population_digest: "sha256-target".to_string(),
            protocol_digest: "sha256-protocol".to_string(),
            assumptions: CausalTransportAssumptions {
                source_internal_validity: evidence(AssumptionDisposition::SupportedByDesign),
                selection_exchangeability: evidence(AssumptionDisposition::Unknown),
                no_interference: evidence(AssumptionDisposition::SupportedByDesign),
                treatment_version_invariance: evidence(AssumptionDisposition::SupportedByEvidence),
            },
            source_strata: vec![
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
            ],
            target_strata: vec![
                TargetPopulationStratum {
                    modifier_id: "low-infrastructure".to_string(),
                    target_population_count: 20,
                },
                TargetPopulationStratum {
                    modifier_id: "high-infrastructure".to_string(),
                    target_population_count: 80,
                },
            ],
        }
    }

    fn threshold(value: &str) -> DecisionThresholdEvidence {
        DecisionThresholdEvidence {
            threshold: value.to_string(),
            effect_unit: "risk_difference".to_string(),
            rationale: "Predeclared minimum worthwhile target-population risk reduction".to_string(),
            source_digest: "sha256-decision-threshold-v1".to_string(),
            disposition: AssumptionDisposition::SupportedByEvidence,
        }
    }

    fn sensitivity(lambda: f64) -> Result<CausalTransportSensitivityResult, DecisionFrontierError> {
        let evidence = AssumptionEvidence {
            disposition: AssumptionDisposition::SupportedByEvidence,
            evidence_digest: "sha256-sensitivity-envelope".to_string(),
        };
        assess_bernoulli_outcome_shift_transport(&input(), lambda, &evidence)
            .map_err(|_| DecisionFrontierError::SensitivityBoundsMissing)
    }

    #[test]
    fn threshold_classification_separates_positive_negative_and_indeterminate() {
        let baseline = sensitivity(1.0).unwrap();
        assert_eq!(
            classify_target_effect_decision(&baseline, &threshold("0.10"))
                .unwrap()
                .decision,
            TargetDecision::RecommendTreatment
        );
        assert_eq!(
            classify_target_effect_decision(&baseline, &threshold("0.30"))
                .unwrap()
                .decision,
            TargetDecision::RecommendAgainstTreatment
        );
        assert_eq!(
            classify_target_effect_decision(&baseline, &threshold("0.50"))
                .unwrap()
                .decision,
            TargetDecision::RecommendAgainstTreatment
        );
        let shifted = sensitivity(2.0).unwrap();
        assert_eq!(
            classify_target_effect_decision(&shifted, &threshold("0.10"))
                .unwrap()
                .decision,
            TargetDecision::Indeterminate
        );
    }

    #[test]
    fn higher_minimum_benefit_threshold_has_no_larger_lambda_robustness_radius() {
        let mut radii = Vec::new();
        for delta in ["0.00", "0.10", "0.20"] {
            let report = find_recommendation_tipping_point(
                sensitivity,
                &threshold(delta),
                4.0,
                1e-8,
            )
            .unwrap();
            let tipping = report.tipping_point.expect("baseline robust recommendation tips");
            assert_eq!(report.status, DecisionFrontierStatus::TippingPointFound);
            radii.push(tipping.lambda);
        }
        assert!(radii[0] >= radii[1]);
        assert!(radii[1] >= radii[2]);
        assert!((radii[0] - (23.0_f64 / 9.0).sqrt()).abs() < 2e-8);
        assert!((radii[1] - 1.3447868740566928).abs() < 2e-8);
        assert!((radii[2] - 1.1368384679697574).abs() < 2e-8);
    }

    #[test]
    fn baseline_nonrecommendation_and_no_crossing_within_horizon_are_explicit() {
        let not_robust = find_recommendation_tipping_point(
            sensitivity,
            &threshold("0.30"),
            4.0,
            1e-8,
        )
        .unwrap();
        assert_eq!(not_robust.status, DecisionFrontierStatus::NotRobustAtBaseline);
        assert_eq!(not_robust.baseline_decision, TargetDecision::RecommendAgainstTreatment);
        assert_eq!(not_robust.tipping_point, None);

        let no_crossing = find_recommendation_tipping_point(
            sensitivity,
            &threshold("-0.90"),
            8.0,
            1e-8,
        )
        .unwrap();
        assert_eq!(
            no_crossing.status,
            DecisionFrontierStatus::NoTippingPointWithinHorizon
        );
        assert_eq!(no_crossing.decision_at_horizon, TargetDecision::RecommendTreatment);
    }

    #[test]
    fn invalid_thresholds_and_missing_threshold_provenance_fail_closed() {
        assert_eq!(
            validate_threshold(&threshold("NaN")).unwrap_err(),
            DecisionFrontierError::InvalidThreshold
        );
        assert_eq!(
            validate_threshold(&threshold("1.1")).unwrap_err(),
            DecisionFrontierError::InvalidThreshold
        );
        let mut missing_source = threshold("0.1");
        missing_source.source_digest.clear();
        assert_eq!(
            validate_threshold(&missing_source).unwrap_err(),
            DecisionFrontierError::MissingThresholdSourceDigest
        );
        let mut unsupported = threshold("0.1");
        unsupported.disposition = AssumptionDisposition::Unknown;
        assert_eq!(
            validate_threshold(&unsupported).unwrap_err(),
            DecisionFrontierError::UnsupportedThresholdEvidence
        );
    }

    #[test]
    fn search_horizon_and_tolerance_are_validated() {
        assert_eq!(
            find_recommendation_tipping_point(
                sensitivity,
                &threshold("0.1"),
                0.9,
                1e-4
            )
            .unwrap_err(),
            DecisionFrontierError::InvalidSearchHorizon
        );
        assert_eq!(
            find_recommendation_tipping_point(
                sensitivity,
                &threshold("0.1"),
                4.0,
                0.0
            )
            .unwrap_err(),
            DecisionFrontierError::InvalidSearchHorizon
        );
    }
}
